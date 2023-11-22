use js_sys::{Array, Error, Function, JsString, Object, Promise, Reflect};
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Condvar, Mutex};
use wasm_bindgen::{closure::Closure, JsValue};
use wasm_bindgen_futures::JsFuture;

pub struct Closures {
    container: Vec<Box<dyn AnyClosure>>,
}

pub trait AnyClosure {}
impl AnyClosure for Closure<dyn Fn()> {}
impl AnyClosure for Closure<dyn Fn(i32)> {}
impl AnyClosure for Closure<dyn Fn(i32, u32, u32) -> Promise> {}
impl AnyClosure for Closure<dyn Fn(Object, Object)> {}
impl AnyClosure for Closure<dyn Fn(JsString)> {}
impl AnyClosure for Closure<dyn Fn(Object)> {}

impl Closures {
    pub fn new() -> Self {
        Self {
            container: Vec::new(),
        }
    }

    pub fn add(&mut self, closure: impl AnyClosure + 'static) {
        self.container.push(Box::new(closure));
    }
}
