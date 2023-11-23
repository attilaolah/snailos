use std::{
    collections::HashMap,
    ops::Deref,
    rc::Rc,
    sync::{Condvar, Mutex},
};

use js_sys::{Error, Function, JsString, Object, Promise, Reflect};
use wasm_bindgen::{closure::Closure, JsValue};
use wasm_bindgen_futures::JsFuture;

use crate::async_io::AsyncIo;
use crate::binfs::BinFs;
use crate::compilation_mode::unexpected;
use crate::js;

pub struct ProcessManager {
    cnt: u32,
    map: HashMap<u32, Process>,
    p_defer: Function,

    binfs: BinFs,
}

struct Process {
    state: Rc<(Mutex<State>, Condvar)>,

    istream: Rc<AsyncIo>,
    ostream: Rc<AsyncIo>,

    #[allow(dead_code)]
    callbacks: Callbacks,
}

struct Callbacks {
    set_module: Closure<dyn Fn(Object)>,
    init_module: Closure<dyn Fn(Object, Object)>,
    init_runtime: Closure<dyn Fn()>,
    read: Closure<dyn Fn(i32, u32, u32) -> Promise>,
    print: Closure<dyn Fn(JsString)>,
    exit: Closure<dyn Fn(i32)>,
}

pub enum State {
    Init,
    InitFailed,
    Waiting(JsFuture),
    Running,
    Exited(i32),
}

impl ProcessManager {
    pub fn new(p_defer: Function) -> Self {
        Self {
            cnt: 1,
            map: HashMap::new(),
            p_defer,
            binfs: BinFs::new("/bin"),
        }
    }

    /// Executes the given binary file.
    /// Once the process has started, returns its pid.
    pub async fn exec(&mut self, file_path: &str, args: &[&str]) -> Result<u32, Error> {
        let resolved_path = self
            .binfs
            .resolve(file_path)
            .ok_or(Error::new(&format!("failed to resolve: {}", file_path)))?;

        let module = js::load_module(&resolved_path.to_string_lossy().to_string()).await?;

        let p = Process::new(
            module,
            &resolved_path
                .file_stem()
                .unwrap() // already validated above
                .to_string_lossy()
                .to_string(),
            args,
            self.p_defer.clone(),
        )?;

        let pid = self.next_pid();
        self.map.entry(pid).or_insert(p);

        Ok(pid)
    }

    /// Waits until a process produces output.
    pub async fn wait_output(&self, pid: u32) -> Result<Option<Vec<JsValue>>, Error> {
        match self.map.get(&pid) {
            Some(p) => p.ostream.wait().await,
            None => Err(Error::new(&format!("no such process: {}", pid))),
        }
    }

    /// Waits until a process exits, returning its exit code.
    pub async fn wait_quit(&mut self, pid: u32) -> Result<i32, Error> {
        match self.map.get_mut(&pid) {
            Some(p) => {
                let exit_code = p.wait().await?;
                self.map.remove(&pid);
                Ok(exit_code)
            }
            None => Err(Error::new(&format!("no such process: {}", pid))),
        }
    }

    fn next_pid(&mut self) -> u32 {
        self.cnt += 1;
        self.cnt
    }
}

impl Process {
    fn new(
        module: Function,
        name: &str,
        arguments: &[&str],
        // TODO: Find a better place for this.
        p_defer: Function,
    ) -> Result<Self, Error> {
        let state = Rc::new((Mutex::new(State::Init), Condvar::new()));
        let istream = Rc::new(AsyncIo::new(p_defer.clone()));
        let ostream = Rc::new(AsyncIo::new(p_defer.clone()));
        let callbacks = Callbacks::new(&state, &istream, &ostream);

        let module_args = js::Builder::new()
            .set("thisProgram", name)?
            .set("arguments", js::str_array(arguments))?
            .set("os.set_module", callbacks.set_module.as_ref())?
            .set("os.init_module", callbacks.init_module.as_ref())?
            .set("os.init_runtime", callbacks.init_runtime.as_ref())?
            .set("os.read", callbacks.read.as_ref())?
            .set("print", callbacks.print.as_ref())?
            .set("printErr", callbacks.print.as_ref())?
            .set("exit", callbacks.exit.as_ref())?;

        let new_state = match module.call1(&JsValue::null(), &module_args.into()) {
            Ok(result) => {
                let promise: Promise = result.into();
                State::Waiting(JsFuture::from(promise))
            }
            Err(err) => {
                js::error(&format!("proc: exec failed: {:?}", err));
                State::InitFailed
            }
        };

        // NOTE: At this point the module has started running the code.
        // If there is nothing blocking it, it might have already quit!

        let running_state = Rc::clone(&state);
        let (lock, _) = &*running_state;
        match lock.lock() {
            Ok(mut guard) => match *guard {
                State::Init => {
                    *guard = new_state;
                }
                State::Exited(_) | State::InitFailed => (),
                _ => js::error("proc: new: unexpected state"),
            },
            Err(_) => js::error("proc: new: mutex poisoned"),
        }

        Ok(Self {
            state,
            istream,
            ostream,
            callbacks,
        })
    }

    /// Waits until the program exits and returns its exit code.
    async fn wait(&mut self) -> Result<i32, Error> {
        let (lock, cvar) = &*self.state;

        // First one to lock should get the future and wait on it.
        // Any subsequent calls should get the exit code and return it.
        match lock.lock() {
            Ok(mut guard) => match &mut *guard {
                // We're in the first call, extract the future from the mutex.
                State::Waiting(_) => match std::mem::replace(&mut *guard, State::Running) {
                    State::Waiting(future) => future,
                    // This should never happen, since we already matched on the type above.
                    _ => return unexpected("unexpected state: !waiting while waiting"),
                },
                // Some other caller is already executing the mutex. Wait for the exit signal.
                State::Running => match cvar.wait(guard) {
                    Ok(mut guard) => match &mut *guard {
                        // The conditional var should only be triggered by the exit callback.
                        State::Exited(exit_code) => return Ok(*exit_code),
                        _ => return unexpected("state: !exited after cvar notify"),
                    },
                    Err(_) => return unexpected("mutex poisoned after cvar notify"),
                },
                State::Exited(exit_code) => return Ok(*exit_code),
                State::Init => return unexpected("state: init"),
                State::InitFailed => return unexpected("state: init failed"),
            },
            Err(_) => return unexpected("mutex poisoned"),
        }
        .await?;

        // If we're here, that means we were in the first call.
        // Any subsequent attempt to acquire the lock shoulod yield the exit code.
        // If it doesn't, that means quit() was not called, so we return -1 instead.
        match lock.lock().unwrap().deref() {
            State::Running => Ok(-1), // zombie process
            State::Exited(exit_code) => Ok(*exit_code),
            _ => return unexpected("state: !exited && !running after running"),
        }
    }
}

impl Callbacks {
    fn new(
        state: &Rc<(Mutex<State>, Condvar)>,
        istream: &Rc<AsyncIo>,
        ostream: &Rc<AsyncIo>,
    ) -> Self {
        Self {
            set_module: Self::set_module(),
            init_module: Self::init_module(),
            init_runtime: Self::init_runtime(),
            read: Self::read(istream),
            print: Self::print(ostream),
            exit: Self::exit(state, ostream),
        }
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

    pub fn read(istream: &Rc<AsyncIo>) -> Closure<dyn Fn(i32, u32, u32) -> Promise> {
        let _channel = istream.clone();

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

    pub fn print(ostream: &Rc<AsyncIo>) -> Closure<dyn Fn(JsString)> {
        let channel = ostream.clone();

        Closure::new(move |text: JsString| {
            if let Err(_) = channel.send(text.into()) {
                js::error("proc: write failed")
            }
        })
    }

    // TODO: This doesn't really need a mutex/condvar combo.
    // Refactor the code to use either a promise or a ref cell.
    pub fn exit(
        state: &Rc<(Mutex<State>, Condvar)>,
        ostream: &Rc<AsyncIo>,
    ) -> Closure<dyn Fn(i32)> {
        let channel = ostream.clone();
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
