use std::{
    cell::RefCell,
    collections::{
        hash_map::Entry::{Occupied, Vacant},
        HashMap, VecDeque,
    },
    rc::Rc,
};

use js_sys::{Error, Promise, Uint8Array};
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
    // Target C-style buffer (pointer + length).
    target: RefCell<Option<HeapView>>,
    // Deferred object, waiting for data to become available.
    deferred: RefCell<Option<js::Deferred>>,
    // Whether the file descriptor is open.
    closed: RefCell<bool>,
}

struct HeapView {
    module: Rc<RefCell<Option<js::Module>>>,
    offset: u32,
    length: u32,
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
            Occupied(_) => Err(Error::new(&format!("io: fd {}: open: already open", fd))),
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
    /// no data is returned (but the promise resolves immediately).
    pub fn read_promise(
        &self,
        fd: u32,
        module: &Rc<RefCell<Option<js::Module>>>,
        offset: u32,
        length: u32,
    ) -> Result<Promise, Error> {
        self.buffers
            .borrow()
            .get(&fd)
            .ok_or(Error::new(&format!("io: fd {}: consume_all: not open", fd)))?
            .read_promise(module, offset, length)
    }

    /// Consume all data from the file descriptor.
    ///
    /// This can be more efficient as the data is returned in chunks and no copy needs to be done.
    pub async fn consume_all(&self, fd: u32) -> Result<Option<Vec<Vec<u8>>>, Error> {
        {
            let buf = self
                .buffers
                .borrow()
                .get(&fd)
                .ok_or(Error::new(&format!("io: fd {}: consume_all: not open", fd)))?
                .clone();
            // Releasing the temporary .borrow() here.
            buf
        }
        .consume_all()
        .await
    }

    /// Writes data to the file descriptor.
    ///
    /// Returns the number of bytes written, or an error.
    ///
    /// If there is a consumer connected, some data might be delivered immediately, otherwise, the
    /// data is buffered.
    pub fn write(&self, fd: u32, data: Vec<u8>) -> Result<usize, Error> {
        self.buffers
            .borrow()
            .get(&fd)
            .ok_or(Error::new(&format!("io: fd {}: write: not open", fd)))?
            .write(data)
    }

    /// Closes the file descriptor, indicating that no more data can be sent.
    pub fn close(&self, fd: u32) -> Result<(), Error> {
        match self.buffers.borrow_mut().remove(&fd) {
            Some(buf) => Ok(buf.close()),
            None => Err(Error::new(&format!("io: fd {}: close: not open", fd))),
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
            target: RefCell::new(None),
            deferred: RefCell::new(None),
            closed: RefCell::new(false),
        }
    }

    fn read_promise(
        &self,
        module: &Rc<RefCell<Option<js::Module>>>,
        offset: u32,
        length: u32,
    ) -> Result<Promise, Error> {
        if self.deferred.borrow().is_some() {
            return Err(Error::new("io: read_promise: channel busy"));
        }

        // Indicate where to copy the data.
        self.target.replace(Some(HeapView {
            module: module.clone(),
            offset,
            length,
        }));

        if !self.buffer.borrow().is_empty() {
            // Synchronous path: there is already data in the buffer.
            let def = js::deferred()?;
            let promise = def.promise();
            self.deferred.replace(Some(def));
            self.signal_write();
            return Ok(promise);
        }

        if *self.closed.borrow() {
            // The buffer is drained and the channel is closed.
            // Indicate that there is no more data by returning zero bytes.
            return Ok(Promise::resolve(&0.into()));
        }

        let def = js::deferred()?;
        let promise = def.promise();
        self.deferred.replace(Some(def));

        // Asynchronous path: the promise will receive the signal when a write() happens.
        Ok(promise)
    }

    /// Consume all data from the buffer.
    ///
    /// If there is data in the buffer, it will be returned immediately. Otherwise, if the producer
    /// side is still connected, this function will block until either data becomes available or
    /// the producer side disconnects. If the producer is disconnected and the buffer is drained,
    /// None is returned.
    ///
    /// This function may be more efficient than read() as the data is returned in chunks and no
    /// copy needs to be done.
    async fn consume_all(&self) -> Result<Option<Vec<Vec<u8>>>, Error> {
        if self.deferred.borrow().is_some() {
            return Err(Error::new("io: consume_all: channel busy"));
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

        // Wait for data. We have to make sure nothing is currently borrowed.
        self.wait_read().await?;

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
    fn write(&self, data: Vec<u8>) -> Result<usize, Error> {
        if *self.closed.borrow() {
            return Err(Error::new("io: write: stream closed"));
        }

        let count = data.len();
        // TODO: This borrow fails, someone is holding on to it?
        self.buffer.borrow_mut().push_back(data);
        self.signal_write();

        Ok(count)
    }

    /// Blocks until someone calls signal_write().
    async fn wait_read(&self) -> Result<(), Error> {
        if let Some(promise) = {
            let opt = self.deferred.borrow().as_ref().map(|d| d.promise());
            // Releasing the temporary .borrow() here.
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
            def.resolve(&self.copy_to_target().into());
        }
    }

    /// Copies bytes to the target buffer if there is one.
    ///
    /// This will also "consume" any bytes copied since there is no pending consume_all() if there
    /// is a target buffer (because there can be only one consumer).
    fn copy_to_target(&self) -> usize {
        let mut copied = 0;
        if let Some(target) = self.target.borrow().as_ref() {
            let mut target_buf = target.buffer();
            let mut buffer = self.buffer.borrow_mut();
            while let Some(mut chunk) = buffer.pop_front() {
                let length = target.length as usize;
                if chunk.len() > length {
                    // There is not enough space in the buffer to copy all the data.
                    // Consume as much as we can, and put the rest back into the buffer.
                    let (copy, remaining) = chunk.split_at_mut(length);
                    target_buf.copy_from(copy);
                    buffer.push_front(remaining.to_vec());
                    copied = length;
                    break;
                }

                // There is enough space in the buffer to copy the entire chunk.
                target_buf.subarray(0, chunk.len() as u32).copy_from(&chunk);
                copied += chunk.len();

                if copied == length {
                    break; // no more space left
                }

                // Subslice the array for the next iteration.
                target_buf = target_buf.subarray(chunk.len() as u32, target_buf.length());
            }
        }

        copied
    }

    fn close(&self) {
        self.closed.replace(true);
        self.signal_write();
    }
}

// TODO: Move to js::Module!
impl HeapView {
    fn buffer(&self) -> Uint8Array {
        // TODO: Avoid the .unwrap()!
        self.module
            .borrow()
            .as_ref()
            .unwrap()
            .heap()
            .subarray(self.offset, self.offset + self.length)
    }
}
