use std::{cell::RefCell, collections::HashMap, ops::Deref, rc::Rc};

use js_sys::{Error, Function, JsString, Object, Promise, Reflect};
use wasm_bindgen::{closure::Closure, throw_val, JsValue};
use wasm_bindgen_futures::JsFuture;

use crate::async_io::AsyncIo;
use crate::binfs::BinFs;

use crate::js;

// TODO: Include from libc?
pub const STDOUT_FILENO: u32 = 1;
pub const STDERR_FILENO: u32 = 2;

pub struct ProcessManager {
    cnt: u32,
    map: HashMap<u32, Process>,
    binfs: BinFs,
}

struct Process {
    id: u32,
    state: Rc<RefCell<State>>,
    io: Rc<AsyncIo>,

    #[allow(dead_code)]
    callbacks: Callbacks,
}

struct Callbacks {
    set_module: Closure<dyn Fn(Object)>,
    init_module: Closure<dyn Fn(Object, Object)>,
    init_runtime: Closure<dyn Fn()>,
    read: Closure<dyn Fn(i32, u32, u32) -> Promise>,
    print: Closure<dyn Fn(JsString)>,
    print_err: Closure<dyn Fn(JsString)>,
    exit: Closure<dyn FnMut(i32)>,
}

pub enum State {
    Running(js::Deferred),
    Exited(i32),
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            cnt: 1,
            map: HashMap::new(),
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

        let pid = self.next_pid();
        let p = Process::new(
            pid,
            module,
            &resolved_path
                .file_stem()
                .unwrap() // already validated above
                .to_string_lossy()
                .to_string(),
            args,
        )?;

        self.map.entry(pid).or_insert(p);

        Ok(pid)
    }

    /// Waits until a process produces output on a file descriptor.
    pub async fn wait_data(&self, pid: u32, fd: u32) -> Result<Option<Vec<Vec<u8>>>, Error> {
        self.map
            .get(&pid)
            .ok_or(Error::new(&format!("no such process: {}", pid)))?
            .io
            .read_all(fd)
            .await
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
    fn new(id: u32, module: Function, name: &str, arguments: &[&str]) -> Result<Self, Error> {
        let state = Rc::new(RefCell::new(State::Running(js::deferred()?)));
        let io = Rc::new({
            let mut io = AsyncIo::new();
            io.open(STDOUT_FILENO)?;
            io.open(STDERR_FILENO)?;
            io
        });
        let callbacks = Callbacks::new(&state, &io);

        let module_args = js::Builder::new()
            .set("thisProgram", name)?
            .set("arguments", js::str_array(arguments))?
            .set("print", callbacks.print.as_ref())?
            .set("printErr", callbacks.print.as_ref())?
            .set("exit", callbacks.exit.as_ref())?
            .set("os.set_module", callbacks.set_module.as_ref())?
            .set("os.init_module", callbacks.init_module.as_ref())?
            .set("os.init_runtime", callbacks.init_runtime.as_ref())?
            .set("os.read", callbacks.read.as_ref())?;

        let _promise: Promise = module.call1(&JsValue::null(), &module_args.into())?.into();
        // TODO: js::spawn(promise)

        Ok(Self {
            id,
            state,
            io,
            callbacks,
        })
    }

    /// Waits until the program exits and returns its exit code.
    async fn wait(&self) -> Result<i32, Error> {
        if let State::Running(def) = self.state.borrow().deref() {
            JsFuture::from(def.promise()).await?;
        }

        match self.state.borrow().deref() {
            State::Exited(code) => Ok(*code),
            _ => Err(Error::new(&format!("proc: pid {}: zombie", self.id))),
        }
    }
}

impl Callbacks {
    fn new(state: &Rc<RefCell<State>>, io: &Rc<AsyncIo>) -> Self {
        Self {
            set_module: Self::set_module(),
            init_module: Self::init_module(),
            init_runtime: Self::init_runtime(),
            read: Self::read(io.clone()),
            print: Self::print(io.clone(), STDOUT_FILENO),
            print_err: Self::print(io.clone(), STDERR_FILENO),
            exit: Self::exit(state.clone(), io.clone()),
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

    pub fn read(_io: Rc<AsyncIo>) -> Closure<dyn Fn(i32, u32, u32) -> Promise> {
        Closure::new(move |fd: i32, _buf: u32, _count: u32| -> Promise {
            match u32::try_from(fd) {
                Err(_) => Promise::new(&mut |res: Function, _: Function| {
                    js::error(&format!("proc: fd {}: read failed", fd));
                    if let Err(err) = res.call1(&JsValue::null(), &(-1).into()) {
                        throw_val(err);
                    }
                }),
                // TODO: Return io.promise_read().then(……)?
                Ok(fd) => Promise::new(&mut |_res: Function, _: Function| {
                    js::warn(&format!("proc: fd {}: read: not implemented", fd));
                    // Never resolve.
                }),
            }
        })
    }

    pub fn print(io: Rc<AsyncIo>, fd: u32) -> Closure<dyn Fn(JsString)> {
        Closure::new(move |text: JsString| {
            if let Err(_) = io.write_string(fd, text) {
                js::error(&format!("proc: fd {}: write failed", fd));
            }
        })
    }

    pub fn exit(state: Rc<RefCell<State>>, io: Rc<AsyncIo>) -> Closure<dyn FnMut(i32)> {
        Closure::new(move |code: i32| match state.borrow().deref() {
            State::Running(def) => {
                RefCell::swap(state.as_ref(), &State::Exited(code).into());
                if let Err(_) = io.close_all() {
                    js::error("proc: failed to close file descriptors")
                }
                def.resolve(&JsValue::null());
            }
            State::Exited(_) => js::error("proc: exit called more than once"),
        })
    }
}
