use js_sys::{Function, JsString, Object, Promise, Reflect};
use std::rc::Rc;
use std::sync::{Condvar, Mutex};
use wasm_bindgen::closure::Closure;

use crate::async_io::AsyncIo;
use crate::js;
use crate::proc::State;

pub struct ProcClosures {
    container: Vec<Box<dyn AnyClosure>>,
}

pub trait AnyClosure {}

impl ProcClosures {
    pub fn new() -> Self {
        Self {
            container: Vec::new(),
        }
    }

    pub fn add(&mut self, closure: impl AnyClosure + 'static) {
        self.container.push(Box::new(closure));
    }

    pub fn set_module() -> Closure<dyn Fn(Object)> {
        Closure::new(move |_module: Object| {
            // TODO: Set the module here!
        })
    }
    pub fn init_module() -> Closure<dyn Fn(Object, Object)> {
        Closure::new(|env: Object, fs: Object| {
            if let Err(_) = Reflect::set(&env, &"USER".into(), &JsString::from("snail")) {
                js::error("proc: module init: failed to set user");
            }

            // TODO: Write a JS binding for this!
            let rename: Function = Reflect::get(&fs, &"rename".into()).unwrap().into();
            if let Err(_) = rename.call2(
                &fs,
                &JsString::from("/home/web_user"),
                &JsString::from("/home/snail"),
            ) {
                js::error("proc: module init: failed to rename home dir");
            }
        })
    }

    pub fn init_runtime() -> Closure<dyn Fn()> {
        Closure::new(move || {})
    }

    pub fn read(io: &Rc<AsyncIo>) -> Closure<dyn Fn(i32, u32, u32) -> Promise> {
        let _channel = io.clone();

        // TODO: Refactor AsyncIo, add a lower-level interface.
        // Instead of blocking, it should return the promise directly to be used here.
        Closure::new(|_fd: i32, _buf: u32, _count: u32| -> Promise {
            // Create a deferred object (p-defer).
            // Send back the resolve function via a back-channel.
            // (It will be resolved when data comes in from the terminal.)
            // Return the promise.
            Promise::new(&mut |_res: Function, _: Function| {
                // Never resolve.
            })
        })
    }

    pub fn print(io: &Rc<AsyncIo>) -> Closure<dyn Fn(JsString)> {
        let channel = io.clone();

        Closure::new(move |text: JsString| {
            if let Err(_) = channel.send(text.into()) {
                js::error("proc: write failed")
            }
        })
    }

    // TODO: This doesn't really need a mutex/condvar combo.
    // Refactor the code to use either a promise or a ref cell.
    pub fn exit(state: &Rc<(Mutex<State>, Condvar)>, io: &Rc<AsyncIo>) -> Closure<dyn Fn(i32)> {
        let channel = io.clone();
        let state = Rc::clone(&state);

        Closure::new(move |code: i32| {
            let (lock, cvar) = &*state;
            match lock.lock() {
                Ok(mut state) => {
                    if let State::Exited(_) = *state {
                        // This happens when quit() is called more than once.
                        // This seems to happen due to Emscripten's Asyncify rewinding.
                        js::warn("proc: quit: called more than once");
                        return;
                    }
                    if let Err(_) = channel.close() {
                        js::error("proc: quit: failed to close output");
                    }
                    *state = State::Exited(code);
                    cvar.notify_all();
                }
                Err(_) => js::error("proc: quit: mutex poisoned"),
            }
        })
    }
}

impl AnyClosure for Closure<dyn Fn()> {}
impl AnyClosure for Closure<dyn Fn(JsString)> {}
impl AnyClosure for Closure<dyn Fn(Object)> {}
impl AnyClosure for Closure<dyn Fn(Object, Object)> {}
impl AnyClosure for Closure<dyn Fn(i32)> {}
impl AnyClosure for Closure<dyn Fn(i32, u32, u32) -> Promise> {}
