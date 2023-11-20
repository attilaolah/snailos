use js_sys::{Array, Error, Function, Object, Promise, Reflect};
use std::cell::RefCell;
use std::collections::VecDeque;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use wasm_bindgen_futures::JsFuture;

// TODO: Write a Rust wrapper (or binding) for Deferred (entry_point.js).

/// Asynchronous, Promise-backed I/O with an infinite buffer.
///
/// Supports multiple producers and a single consumer. As soon as one producer closes the channel,
/// no more data can be sent. Once the buffer is drained, no more consumers can connect.
pub struct AsyncIo {
    // Internal I/O buffer.
    queue: RefCell<VecDeque<JsValue>>,
    // Deferred object, waiting for data to become available.
    deferred: RefCell<Object>,
    // Marker indicating whether the channel is closed.
    closed: RefCell<bool>,

    p_defer: Function,
}

impl AsyncIo {
    pub fn new(p_defer: Function) -> Self {
        Self {
            queue: RefCell::new(VecDeque::new()),
            deferred: RefCell::new(JsValue::null().into()),
            closed: RefCell::new(false),
            p_defer,
        }
    }

    /// Send some data.
    ///
    /// If there is a consumer connected, the data will be delivered immediately and this function
    /// returns true. Otherwise, the data is queued and this function returns false.
    pub fn send(&self, data: JsValue) -> Result<bool, Error> {
        self.queue_and_forward(Some(data))
    }

    /// Closes the channel, indicating that no more data can be sent.
    ///
    /// Returns true if there was a consumer connected. In that case, the consumer will be
    /// disconnected immediately. Otherwise returns false and just closes the send buffer.
    pub fn close(&self) -> Result<bool, Error> {
        self.queue_and_forward(None)
    }

    /// Wait for data to become available.
    ///
    /// If there is data in the buffer, it will be returned immediately. Otherwise, if the producer
    /// side is still connected, this function will block until either data becomes available or
    /// the producer side disconnects. If the producer is disconnected and the queue is drained,
    /// None is returned.
    pub async fn wait(&self) -> Result<Option<Vec<JsValue>>, Error> {
        if !self.deferred.borrow().is_null() {
            return Err(Error::new("channel busy (receiver already attached)"));
        }

        // Fast path if there is already data in the buffer.
        // There may already be a None in the queue, which signals that the channel is closed.
        if !self.queue.borrow().is_empty() {
            return Ok(Some(self.queue.borrow_mut().drain(..).collect()));
        }

        if *self.closed.borrow() {
            // The queue is drained and the channel is closed.
            // Indicate that there is no more data by returning None.
            return Ok(None);
        }

        // Wait for data.
        let deferred: Object = self.p_defer.call0(&JsValue::null())?.into();
        let promise: Promise = Reflect::get(&deferred, &"promise".into())?.into();
        self.deferred.replace(deferred);

        // Blocks until the promise resolves.
        let data: Array = JsFuture::from(promise).await?.into();

        // Wait is done, more data may have arrived.
        let vec: Vec<JsValue> = data.iter().collect();
        Ok(if vec.is_empty() && *self.closed.borrow() {
            // The channel was closed and there is no more data to return.
            None
        } else {
            // There was still data in the queue. Note that the channel may have been closed: if
            // so, the consumer will have to make another call to find out.
            Some(vec)
        })
    }

    fn queue_and_forward(&self, data: Option<JsValue>) -> Result<bool, Error> {
        if *self.closed.borrow() {
            return Err(Error::new("channel closed"));
        }
        match data {
            Some(item) => self.queue.borrow_mut().push_back(item),
            None => {
                self.closed.replace(true);
            }
        }

        if !self.deferred.borrow().is_null() {
            let data_js = Array::new();
            while let Some(item) = self.queue.borrow_mut().pop_front() {
                data_js.push(&item);
            }

            let deferred = self.deferred.replace(JsValue::null().into());
            let resolve: Function = Reflect::get(&deferred, &"resolve".into()).unwrap().into();
            resolve.call1(&JsValue::null(), &data_js)?;
            return Ok(true);
        }

        Ok(false)
    }
}
