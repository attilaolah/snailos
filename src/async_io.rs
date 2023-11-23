use std::{
    cell::RefCell,
    collections::{
        hash_map::Entry::{Occupied, Vacant},
        HashMap, VecDeque,
    },
};

use js_sys::{Error, JsString, Promise};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::TextEncoder;

use crate::js;

/// Asynchronous, Promise-backed I/O with an infinite buffer.
///
/// Supports one channel per file descriptor, with multiple producers and a single consumer. As
/// soon as one producer closes the channel, no more data can be sent. Once the buffer is drained,
/// the channel is removed and can be re-opened.
pub struct AsyncIo {
    // Internal I/O buffer. Keyed by file descriptors.
    queue: HashMap<u32, AsyncBuffer>,
    // TextEncoder used for encoding strings.
    encoder: Option<TextEncoder>,
}

struct AsyncBuffer {
    // Internal I/O buffer.
    buffer: RefCell<VecDeque<Vec<u8>>>,
    // Deferred object, waiting for data to become available.
    deferred: RefCell<Option<js::Deferred>>,
    // Whether the file descriptor is open.
    closed: bool,
}

impl AsyncIo {
    pub fn new() -> Self {
        Self {
            queue: HashMap::new(),
            encoder: None,
        }
    }

    /// Opens a file descriptor.
    ///
    /// Returns an error if the file descriptor is already open.
    pub fn open(&mut self, fd: u32) -> Result<(), Error> {
        match self.queue.entry(fd) {
            Occupied(_) => Err(Error::new(&format!("open: fd {}: already open", fd))),
            Vacant(v) => {
                v.insert(AsyncBuffer::new());
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
        self.queue
            .get(&fd)
            .ok_or(Error::new(&format!("read_all: fd {}: not open", fd)))?
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
        self.queue
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
    pub fn close(&self, _fd: u32) -> Result<(), Error> {
        todo!();
    }

    /// Closes all file descriptors.
    pub fn close_all(&self) -> Result<(), Error> {
        todo!();
    }
}

impl AsyncBuffer {
    fn new() -> Self {
        Self {
            buffer: RefCell::new(VecDeque::new()),
            deferred: RefCell::new(None),
            closed: false,
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

        if self.closed {
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
        if self.closed {
            return Err(Error::new("channel closed"));
        }

        let result = data.len() as u32;
        self.buffer.borrow_mut().push_back(data);
        self.signal_write();

        Ok(result)
    }

    /// Blocks until someone calls signal_write().
    async fn wait_read(&self) -> Result<(), Error> {
        // Temporary borrow(), we need to release it immediately.
        // Otherwise someone calling signal_write() would fail to replace().
        let opt = self.deferred.borrow().as_ref().map(|d| d.promise());

        if let Some(promise) = opt {
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
}
