use js_sys::{JsString, Object, Promise};
use wasm_bindgen::closure::Closure;

pub struct Closures {
    container: Vec<Box<dyn AnyClosure>>,
}

pub trait AnyClosure {}
impl AnyClosure for Closure<dyn Fn()> {}
impl AnyClosure for Closure<dyn Fn(JsString)> {}
impl AnyClosure for Closure<dyn Fn(Object)> {}
impl AnyClosure for Closure<dyn Fn(Object, Object)> {}
impl AnyClosure for Closure<dyn Fn(i32)> {}
impl AnyClosure for Closure<dyn Fn(i32, u32, u32) -> Promise> {}

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
