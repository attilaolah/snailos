use std::{cell::RefCell, collections::HashMap, ops::Deref, rc::Rc};

use js_sys::{Error, Function, JsString, Object, Promise, Reflect};
use wasm_bindgen::{closure::Closure, JsValue};
use wasm_bindgen_futures::JsFuture;

use crate::{
    async_io::{AsyncIo, STDERR, STDIN, STDOUT},
    binfs::BinFs,
    js,
};

pub type Pid = u32;

pub struct ProcessManager {
    map: RefCell<HashMap<Pid, Rc<Process>>>,
    next_pid: RefCell<Pid>,
    binfs: BinFs,
}

struct Process {
    id: Pid,
    state: Rc<RefCell<State>>,
    module: Rc<RefCell<Option<js::Module>>>,
    io: Rc<AsyncIo>,
    promise: Promise,

    #[allow(dead_code)]
    callbacks: Callbacks,
}

struct Callbacks {
    print: Closure<dyn Fn(String)>,
    print_err: Closure<dyn Fn(String)>,
    exit: Closure<dyn FnMut(i32)>,

    // OS init:
    set_module: Closure<dyn Fn(js::Module)>,
    init_module: Closure<dyn Fn(Object, Object)>,
    init_runtime: Closure<dyn Fn()>,

    // Mocked syscalls & library functions:
    read: Closure<dyn Fn(i32, u32, u32) -> Promise>, // -> ssize_t = usize
    vfork: Closure<dyn Fn() -> Promise>,             // -> pid_t = u32
    wait4: Closure<dyn Fn(u32, u32, i32, u32) -> Promise>, // -> pid_t = u32
    waitpid: Closure<dyn Fn(u32, u32, i32) -> Promise>, // -> pid_t = u32
}

pub enum State {
    Running(js::Deferred),
    Exited(i32),
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            map: RefCell::new(HashMap::new()),
            next_pid: RefCell::new(1),
            binfs: BinFs::new("/bin"),
        }
    }

    /// Executes the given binary file.
    /// Once the process has started, returns its pid.
    pub async fn exec(&self, file_path: &str, args: &[&str]) -> Result<Pid, Error> {
        let resolved_path = self
            .binfs
            .resolve(file_path)
            .ok_or(Error::new(&format!("failed to resolve: {}", file_path)))?;

        let ctor = js::load_module(&resolved_path.to_string_lossy().to_string()).await?;

        let pid: Pid = *self.next_pid.borrow();
        self.next_pid.replace(pid + 1);

        let p = Process::new(
            pid,
            ctor,
            &resolved_path
                .file_stem()
                .unwrap() // already validated above
                .to_string_lossy()
                .to_string(),
            args,
        )?;

        self.map.borrow_mut().insert(pid, Rc::new(p));

        Ok(pid)
    }

    /// Writes data to the standard input of a process.
    pub fn stdin_write(&self, pid: Pid, data: Vec<u8>) -> Result<usize, Error> {
        self.map
            .borrow()
            .get(&pid)
            .ok_or(Error::new(&format!("no such process: {}", pid)))?
            .io
            .write(STDIN, data)
    }

    /// Closes the the standard input of a process.
    pub fn stdin_close(&self, pid: Pid) -> Result<(), Error> {
        self.map
            .borrow()
            .get(&pid)
            .ok_or(Error::new(&format!("no such process: {}", pid)))?
            .io
            .close(STDIN)
    }

    /// Waits until a process produces output on a file descriptor.
    pub async fn wait_data(&self, pid: Pid, fd: u32) -> Result<Option<Vec<Vec<u8>>>, Error> {
        {
            self.map
                .borrow()
                .get(&pid)
                .ok_or(Error::new(&format!("no such process: {}", pid)))?
                .clone()
        }
        .io
        .consume_all(fd)
        .await
    }

    /// Waits until a process exits, returning its exit code.
    pub async fn wait_quit(&self, pid: Pid) -> Result<i32, Error> {
        let proc = self
            .map
            .borrow()
            .get(&pid)
            .ok_or(Error::new(&format!("no such process: {}", pid)))?
            .clone();
        let exit_code = proc.wait().await?;
        self.map.borrow_mut().remove(&pid);
        Ok(exit_code)
    }
}

impl Process {
    fn new(id: Pid, ctor: Function, name: &str, arguments: &[&str]) -> Result<Self, Error> {
        let state = Rc::new(RefCell::new(State::Running(js::deferred()?)));
        let module = Rc::new(RefCell::new(None));
        let io = Rc::new(AsyncIo::new()?);

        let callbacks = Callbacks::new(&state, &module, &io);
        let promise: Promise = ctor
            .call1(
                &JsValue::null(),
                &js::Builder::new()
                    .set("thisProgram", name)?
                    .set("arguments", js::str_array(arguments))?
                    .set("print", callbacks.print.as_ref())?
                    // TODO: Connect to print_err. For now we do 2>&1.
                    .set("printErr", callbacks.print.as_ref())?
                    .set("exit", callbacks.exit.as_ref())?
                    // OS init:
                    .set("os.set_module", callbacks.set_module.as_ref())?
                    .set("os.init_module", callbacks.init_module.as_ref())?
                    .set("os.init_runtime", callbacks.init_runtime.as_ref())?
                    // Mocked syscalls & functions:
                    .set("os.read", callbacks.read.as_ref())?
                    .set("os.vfork", callbacks.vfork.as_ref())?
                    .set("os.wait4", callbacks.wait4.as_ref())?
                    .set("os.waitpid", callbacks.waitpid.as_ref())?
                    .into(),
            )?
            .into();

        Ok(Self {
            id,
            state,
            module,
            io,
            promise,
            callbacks,
        })
    }

    /// Waits until the program exits and returns its exit code.
    async fn wait(&self) -> Result<i32, Error> {
        JsFuture::from(self.promise.clone()).await?;
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
    fn new(
        state: &Rc<RefCell<State>>,
        module: &Rc<RefCell<Option<js::Module>>>,
        io: &Rc<AsyncIo>,
    ) -> Self {
        Self {
            print: Self::print(io.clone(), STDOUT),
            print_err: Self::print(io.clone(), STDERR),
            exit: Self::exit(state.clone(), io.clone()),

            set_module: Self::set_module(module.clone()),
            init_module: Self::init_module(),
            init_runtime: Self::init_runtime(),

            read: Self::read(module.clone(), io.clone()),
            vfork: Self::vfork(),
            wait4: Self::wait4(),
            waitpid: Self::waitpid(),
        }
    }

    pub fn print(io: Rc<AsyncIo>, fd: u32) -> Closure<dyn Fn(String)> {
        Closure::new(move |text: String| {
            if let Err(_) = io.write(fd, text.as_bytes().to_vec()) {
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

    pub fn set_module(module: Rc<RefCell<Option<js::Module>>>) -> Closure<dyn Fn(js::Module)> {
        Closure::new(move |js_module: js::Module| {
            module.replace(Some(js_module));
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

    pub fn read(
        module: Rc<RefCell<Option<js::Module>>>,
        io: Rc<AsyncIo>,
    ) -> Closure<dyn Fn(i32, u32, u32) -> Promise> {
        Closure::new(move |fd: i32, buf: u32, count: u32| -> Promise {
            #[cfg(feature = "dbg")]
            js::log(&format!("proc: read({}, {}, {})?", fd, buf, count));

            // TODO: Remove this layer of indentation with if let/else.
            match u32::try_from(fd) {
                Err(_) => Promise::reject(&format!("proc: fd {}: bad file descriptor", fd).into()),
                Ok(fd) => match io.read_promise(fd, &module, buf, count) {
                    Err(_) => Promise::reject(&format!("proc: fd {}: read failed", fd).into()),
                    Ok(promise) => promise,
                },
            }
        })
    }

    pub fn vfork() -> Closure<dyn Fn() -> Promise> {
        Closure::new(move || {
            #[cfg(feature = "dbg")]
            js::log("proc: vfork()?");

            Promise::resolve(&1.into())
        })
    }

    pub fn wait4() -> Closure<dyn Fn(u32, u32, i32, u32) -> Promise> {
        Closure::new(move |pid, status, options, rusage| {
            #[cfg(feature = "dbg")]
            js::log(&format!("proc: wait4({}, {}, {}, {})?", pid, status, options, rusage));

            Promise::resolve(&(-1).into())
        })
    }

    pub fn waitpid() -> Closure<dyn Fn(u32, u32, i32) -> Promise> {
        Closure::new(move |pid, status, options| {
            #[cfg(feature = "dbg")]
            js::log(&format!("proc: waitpid({}, {}, {})?", pid, status, options));

            Promise::resolve(&1.into())
        })
    }
}
