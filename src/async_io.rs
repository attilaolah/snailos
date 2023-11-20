use js_sys::{Array, Error, Function, JsString, Object, Promise, Reflect};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{
    mpsc::{channel, Receiver},
    Condvar, Mutex,
};
use wasm_bindgen::{closure::Closure, prelude::wasm_bindgen, JsValue};
use wasm_bindgen_futures::JsFuture;

/// Asynchronous, Promise-backed I/O with an infinite buffer.
///
/// Supports multiple producers and a single consumer. As soon as one producer closes the channel,
/// no more data can be sent. Once the buffer is drained, no more consumers can connect.
pub struct AsyncIo {
    // Internal I/O buffer.
    queue: VecDeque<Option<JsValue>>,
    // Promise "resolve" function for flushing the entire buffer.
    flush: Option<Function>,
    // Marker indicating whether the channel is closed.
    end: Option<JsValue>,
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
            end: None,
        }
    }

    /// Send some data.
    ///
    /// If there is a consumer connected, the data will be delivered immediately and this function
    /// returns true. Otherwise, the data is queued and this function returns false.
    pub fn send(&mut self, data: JsValue) -> Result<bool, Error> {
        // TODO: Use an EOF marker instead so that null values could be sent.
        if data.is_null() {
            self.close()
        } else {
            self.queue_and_forward(Some(data))
        }
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
    /// If there is data in the buffer, it will be returned immediately. If the last item returned
    /// is None, that means the channel is now closed and there is no more data to receive.
    pub async fn wait(&mut self) -> Result<Vec<Option<JsValue>>, Error> {
        if self.flush.is_some() {
            return Err(Error::new("channel busy (receiver already attached)"));
        }

        // Fast path if there is already data in the buffer.
        // There may already be a None in the queue, which signals that the channel is closed.
        if !self.queue.is_empty() {
            return Ok(self.queue.drain(..).collect());
        }

        if self.is_closed() {
            // No data, but the channel is closed.
            return Err(Error::new("channel closed"));
        }

        // Wait for data.
        let promise = SyncPromise::new();
        self.flush = Some(promise.resolve);
        let data: Array = JsFuture::from(promise.promise).await?.into();
        let vec: Vec<Option<JsValue>> = data
            .iter()
            .map(|val| Some(val))
            // Replace the end marker with None.
            .map(|opt| if opt == self.end { None } else { opt })
            .collect();

        Ok(vec)
    }

    fn queue_and_forward(&mut self, data: Option<JsValue>) -> Result<bool, Error> {
        if self.is_closed() {
            return Err(Error::new("channel closed"));
        }
        if data.is_none() {
            self.end = Some(Object::new().into());
        }

        self.queue.push_back(data);
        if let Some(flush) = &self.flush {
            let data_js = Array::new();
            while let Some(item) = self.queue.pop_front() {
                data_js.push(&item.unwrap_or(JsValue::null()));
            }

            flush.call1(&JsValue::undefined(), &data_js)?;
            self.flush = None;
            return Ok(true);
        }

        Ok(false)
    }

    fn is_closed(&self) -> bool {
        self.end.is_some()
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
        let mut guard = mutex.lock().unwrap();
        let mut guard = cvar.wait(guard).unwrap();
        SyncPromise {
            promise,
            resolve: guard.0.clone().unwrap(),
            reject: guard.1.clone().unwrap(),
        }
    }
}
