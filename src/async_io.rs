use std::{
    cell::RefCell,
    collections::{
        hash_map::Entry::{Occupied, Vacant},
        HashMap, VecDeque,
    },
    rc::Rc,
};

use js_sys::{Error, JsString, Promise};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

use crate::js;

pub const STDIN: u32 = 0;
pub const STDOUT: u32 = 1;
pub const STDERR: u32 = 2;
pub const OPEN_FDS: [u32; 3] = [STDIN, STDOUT, STDERR];

/// Asynchronous, Promise-backed I/O with an infinite buffer.
///
/// Supports one channel per file descriptor, with multiple producers and a single consumer. As
/// soon as one producer closes the channel, no more data can be sent. Once the buffer is drained,
/// the channel is removed and can be re-opened.
pub struct AsyncIo {
    // Internal I/O buffers, keyed by file descriptors.
    buffers: RefCell<HashMap<u32, Rc<AsyncBuffer>>>,
}

struct AsyncBuffer {
    // Internal I/O buffer.
    buffer: RefCell<VecDeque<Vec<u8>>>,
    // Deferred object, waiting for data to become available.
    deferred: RefCell<Option<js::Deferred>>,
    // Whether the file descriptor is open.
    closed: RefCell<bool>,
}

impl AsyncIo {
    pub fn new() -> Result<Self, Error> {
        let io = Self {
            buffers: RefCell::new(HashMap::new()),
        };
        for fd in OPEN_FDS {
            io.open(fd)?;
        }
        Ok(io)
    }

    /// Opens a file descriptor.
    ///
    /// Returns an error if the file descriptor is already open.
    pub fn open(&self, fd: u32) -> Result<(), Error> {
        match self.buffers.borrow_mut().entry(fd) {
            Occupied(_) => Err(Error::new(&format!("open: fd {}: already open", fd))),
            Vacant(v) => {
                v.insert(AsyncBuffer::new().into());
                Ok(())
            }
        }
    }

    /// Read from the file descriptor.
    ///
    /// If there is data in the buffer, it will be returned immediately. Otherwise, if the producer
    /// side is still connected, this function will block until either data becomes available or
    /// the producer side disconnects. If the producer is disconnected and the queue is drained,
    /// None is returned.
    pub async fn read(&self, _fd: u32, _count: usize) -> Result<Option<Vec<u8>>, Error> {
        todo!();
    }

    /// Read all data from the file descriptor.
    ///
    /// This can be more efficient as the data is returned in chunks and no copy needs to be done.
    pub async fn read_all(&self, fd: u32) -> Result<Option<Vec<Vec<u8>>>, Error> {
        {
            self.buffers
                .borrow() // returned before await
                .get(&fd)
                .ok_or(Error::new(&format!("read_all: fd {}: not open", fd)))?
                .clone()
        }
        .read_all()
        .await
    }

    /// Read from the file descriptor.
    ///
    /// Similar to read(), but returns a promise that can be chained. The promise will resolve with
    /// a Uint8Array when data is available, or with null indicating that the channel is closed.
    pub fn read_promise(&self, _fd: u32, _count: usize) -> Result<Promise, Error> {
        todo!();
    }

    /// Writes data to the file descriptor.
    ///
    /// Returns the number of bytes written, or an error.
    ///
    /// If there is a consumer connected, some data might be delivered immediately, otherwise, the
    /// data is buffered.
    pub fn write(&self, fd: u32, data: Vec<u8>) -> Result<u32, Error> {
        self.buffers
            .borrow()
            .get(&fd)
            .ok_or(Error::new(&format!("write: fd {}: not open", fd)))?
            .write(data)
    }

    /// Encodes the JS string and writes it to the file descriptor.
    pub fn write_string(&self, fd: u32, data: JsString) -> Result<u32, Error> {
        self.write(
            fd,
            data.as_string()
                .ok_or(Error::new(&format!(
                    "write_string: fd {}: invalid argument",
                    fd
                )))?
                .as_bytes()
                .to_vec(),
        )
    }

    /// Closes the file descriptor, indicating that no more data can be sent.
    pub fn close(&self, fd: u32) -> Result<(), Error> {
        match self.buffers.borrow_mut().remove(&fd) {
            Some(buf) => Ok(buf.close()),
            None => Err(Error::new(&format!("close: fd {}: not open", fd))),
        }
    }

    /// Closes all file descriptors.
    pub fn close_all(&self) -> Result<(), Error> {
        let mut buffers = self.buffers.borrow_mut();
        for buf in buffers.values() {
            buf.close();
        }
        buffers.clear();

        Ok(())
    }
}

impl AsyncBuffer {
    fn new() -> Self {
        Self {
            buffer: RefCell::new(VecDeque::new()),
            deferred: RefCell::new(None),
            closed: RefCell::new(false),
        }
    }

    /// Read all data from the buffer.
    ///
    /// If there is data in the buffer, it will be returned immediately. Otherwise, if the producer
    /// side is still connected, this function will block until either data becomes available or
    /// the producer side disconnects. If the producer is disconnected and the buffer is drained,
    /// None is returned.
    ///
    /// This function may be more efficient than read() as the data is returned in chunks and no
    /// copy needs to be done.
    async fn read_all(&self) -> Result<Option<Vec<Vec<u8>>>, Error> {
        if self.deferred.borrow().is_some() {
            return Err(Error::new("channel busy (receiver already attached)"));
        }

        if !self.buffer.borrow().is_empty() {
            // Fast path: there is already data in the buffer.
            return Ok(Some(self.buffer.borrow_mut().drain(..).collect()));
        }

        if *self.closed.borrow() {
            // The buffer is drained and the channel is closed.
            // Indicate that there is no more data by returning None.
            return Ok(None);
        }

        self.deferred.replace(Some(js::deferred()?));
        self.wait_read().await?; // Wait for data.

        Ok(if !self.buffer.borrow().is_empty() {
            Some(self.buffer.borrow_mut().drain(..).collect())
        } else {
            // The promise resolved but the buffer is empty: channel must be closed.
            None
        })
    }

    /// Writes data to the buffer.
    ///
    /// Returns the number of bytes written, or an error.
    ///
    /// If there is a consumer connected, some data might be delivered immediately, otherwise, the
    /// data is buffered.
    fn write(&self, data: Vec<u8>) -> Result<u32, Error> {
        if *self.closed.borrow() {
            return Err(Error::new("stream closed"));
        }

        let result = data.len() as u32;
        self.buffer.borrow_mut().push_back(data);
        self.signal_write();

        Ok(result)
    }

    /// Blocks until someone calls signal_write().
    async fn wait_read(&self) -> Result<(), Error> {
        if let Some(promise) = {
            // Temporary borrow(), we need to release it before the await.
            // Otherwise someone calling signal_write() would fail to replace() it.
            let opt = self.deferred.borrow().as_ref().map(|d| d.promise());
            opt
        } {
            // Now that the borrow is returned, we can block safely.
            JsFuture::from(promise).await?;
        }

        Ok(())
    }

    /// Unblocks anyone waiting in wait_read().
    fn signal_write(&self) {
        if let Some(def) = self.deferred.replace(None) {
            def.resolve(&JsValue::null());
        }
    }

    fn close(&self) {
        self.closed.replace(true);
        self.signal_write();
    }
}
