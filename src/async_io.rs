use js_sys::{Array, Error, Function, Promise};
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::{Condvar, Mutex};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

/// Asynchronous, Promise-backed I/O with an infinite buffer.
///
/// Supports multiple producers and a single consumer. As soon as one producer closes the channel,
/// no more data can be sent. Once the buffer is drained, no more consumers can connect.
pub struct AsyncIo {
    // Internal I/O buffer.
    queue: VecDeque<JsValue>,
    // Promise "resolve" function for flushing the entire buffer.
    flush: Option<Function>,
    // Marker indicating whether the channel is closed.
    closed: bool,
}

struct SyncPromise {
    promise: Promise,
    resolve: Function,
    // TODO: Remove unnecessary reject function.
    reject: Function,
}

impl AsyncIo {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            flush: None,
            closed: false,
        }
    }

    /// Send some data.
    ///
    /// If there is a consumer connected, the data will be delivered immediately and this function
    /// returns true. Otherwise, the data is queued and this function returns false.
    pub fn send(&mut self, data: JsValue) -> Result<bool, Error> {
        self.queue_and_forward(Some(data))
    }

    /// Closes the channel, indicating that no more data can be sent.
    ///
    /// Returns true if there was a consumer connected. In that case, the consumer will be
    /// disconnected immediately. Otherwise returns false and just closes the send buffer.
    pub fn close(&mut self) -> Result<bool, Error> {
        self.queue_and_forward(None)
    }

    /// Wait for data to become available.
    ///
    /// If there is data in the buffer, it will be returned immediately. Otherwise, if the producer
    /// side is still connected, this function will block until either data becomes available or
    /// the producer side disconnects. If the producer is disconnected and the queue is drained,
    /// None is returned.
    pub async fn wait(&mut self) -> Result<Option<Vec<JsValue>>, Error> {
        if self.flush.is_some() {
            return Err(Error::new("channel busy (receiver already attached)"));
        }

        // Fast path if there is already data in the buffer.
        // There may already be a None in the queue, which signals that the channel is closed.
        if !self.queue.is_empty() {
            return Ok(Some(self.queue.drain(..).collect()));
        }

        if self.closed {
            // The queue is drained and the channel is closed.
            // Indicate that there is no more data by returning None.
            return Ok(None);
        }

        // Wait for data.
        let promise = SyncPromise::new();
        self.flush = Some(promise.resolve);
        let data: Array = JsFuture::from(promise.promise).await?.into();

        // Wait is done, more data may have arrived.
        let vec: Vec<JsValue> = data.iter().collect();
        Ok(if vec.is_empty() && self.closed {
            // The channel was closed and there is no more data to return.
            None
        } else {
            // There was still data in the queue. Note that the channel may have been closed: if
            // so, the consumer will have to make another call to find out.
            Some(vec)
        })
    }

    fn queue_and_forward(&mut self, data: Option<JsValue>) -> Result<bool, Error> {
        if self.closed {
            return Err(Error::new("channel closed"));
        }
        match data {
            Some(item) => self.queue.push_back(item),
            None => {
                self.closed = true;
            }
        }

        if let Some(flush) = &self.flush {
            let data_js = Array::new();
            while let Some(item) = self.queue.pop_front() {
                data_js.push(&item);
            }

            flush.call1(&JsValue::undefined(), &data_js)?;
            self.flush = None;
            return Ok(true);
        }

        Ok(false)
    }
}

impl SyncPromise {
    fn new() -> Self {
        let mut_cv = Rc::new((
            Mutex::new((
                None as Option<Function>, // resolve
                None as Option<Function>, // reject
            )),
            Condvar::new(),
        ));
        let clone = Rc::clone(&mut_cv);
        let promise = Promise::new(&mut move |res: Function, rej: Function| {
            let (mutex, cvar) = &*clone;
            let mut guard = mutex.lock().unwrap();
            guard.0 = Some(res);
            guard.1 = Some(rej);
            cvar.notify_all();
        });

        let (mutex, cvar) = &*mut_cv;
        let guard = mutex.lock().unwrap();
        let guard = cvar.wait(guard).unwrap();
        SyncPromise {
            promise,
            resolve: guard.0.clone().unwrap(),
            reject: guard.1.clone().unwrap(),
        }
    }
}
