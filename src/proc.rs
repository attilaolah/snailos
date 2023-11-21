use js_sys::{Array, Error, Function, JsString, Object, Promise, Reflect};
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Condvar, Mutex};
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
    output: Rc<AsyncIo>,

    // TODO: Vec<Closure<…>>?

    // These closures need to stay alive as long as the process is running.
    // We will never reference them outside of the constructor, so silence the warnings.
    #[allow(dead_code)]
    set_module: Closure<dyn Fn(Object)>,
    #[allow(dead_code)]
    init_module: Closure<dyn Fn(Object, Object)>,
    #[allow(dead_code)]
    init_runtime: Closure<dyn Fn()>,
    #[allow(dead_code)]
    read: Closure<dyn Fn(i32, u32, u32) -> Promise>,

    #[allow(dead_code)]
    print: Closure<dyn Fn(JsString)>,
    #[allow(dead_code)]
    print_err: Closure<dyn Fn(JsString)>,
    #[allow(dead_code)]
    quit: Closure<dyn Fn(i32)>,
}

enum State {
    Initialising,
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
            Some(p) => p.output.wait().await,
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
        let args_js = Array::new_with_length(arguments.len() as u32);
        for (i, arg) in arguments.iter().enumerate() {
            args_js.set(i as u32, JsString::from(*arg).into());
        }

        let output = Rc::new(AsyncIo::new(p_defer));
        let stdout = output.clone();
        let print: Closure<dyn Fn(_)> = Closure::new(move |text: JsString| {
            if let Err(_) = stdout.send(text.into()) {
                js::error("proc: stdout: write failed")
            }
        });

        let stderr = output.clone();
        let print_err: Closure<dyn Fn(_)> = Closure::new(move |text: JsString| {
            if let Err(_) = stderr.send(text.into()) {
                js::error("proc: stderr: write failed")
            }
        });

        let state = Rc::new((Mutex::new(State::Initialising), Condvar::new()));
        let state_quit = Rc::clone(&state);
        let output_closer = output.clone();
        let quit: Closure<dyn Fn(_)> = Closure::new(move |code: i32| {
            let (lock, cvar) = &*state_quit;
            match lock.lock() {
                Ok(mut state) => {
                    if let State::Exited(_) = *state {
                        js::warn("proc: quit: called more than once");
                        return;
                    }
                    if let Err(_) = output_closer.close() {
                        js::error("proc: quit: failed to close output");
                    }
                    *state = State::Exited(code);
                    cvar.notify_all();
                }
                Err(_) => js::error("proc: quit: mutex poisoned"),
            }
        });

        let os_arg = Object::new();

        let mod_args = Object::new();
        Reflect::set(&mod_args, &"thisProgram".into(), &name.into())?;
        Reflect::set(&mod_args, &"arguments".into(), &args_js.into())?;
        Reflect::set(&mod_args, &"print".into(), &print.as_ref())?;
        Reflect::set(&mod_args, &"printErr".into(), &print_err.as_ref())?;
        Reflect::set(&mod_args, &"quit".into(), &quit.as_ref())?;
        Reflect::set(&mod_args, &"os".into(), &os_arg)?;

        let set_module: Closure<dyn Fn(_)> = Closure::new(move |_module: Object| {
            // TODO: Set the module here!
        });
        let init_module: Closure<dyn Fn(_, _)> = Closure::new(|env: Object, fs: Object| {
            Reflect::set(&env, &"USER".into(), &JsString::from("snail")).unwrap();

            // TODO: Write a JS binding for this!
            let rename: Function = Reflect::get(&fs, &"rename".into()).unwrap().into();
            rename
                .call2(
                    &fs,
                    &JsString::from("/home/web_user"),
                    &JsString::from("/home/snail"),
                )
                .unwrap();
        });
        let init_runtime: Closure<dyn Fn()> = Closure::new(|| {
            // Currently this is a no-op.
        });
        let read: Closure<dyn Fn(_, _, _) -> _> =
            Closure::new(|fd: i32, _buf: u32, _count: u32| -> Promise {
                // TODO: Hook up the i/o.
                // For now, we just return a promise that leaks, but closes stdin.
                Promise::new(&mut |res: Function, _: Function| {
                    if let Err(_) = res.call1(&JsValue::null(), &0.into()) {
                        js::log(&format!("proc: fd {}: read error", fd));
                    }
                })
            });

        Reflect::set(&os_arg, &"set_module".into(), &set_module.as_ref())?;
        Reflect::set(&os_arg, &"init_module".into(), &init_module.as_ref())?;
        Reflect::set(&os_arg, &"init_runtime".into(), &init_runtime.as_ref())?;
        Reflect::set(&os_arg, &"read".into(), &read.as_ref())?;

        let promise: Promise = module.call1(&JsValue::null(), &mod_args)?.into();
        let future = JsFuture::from(promise);

        // NOTE: At this point the module has started running the code.
        // If there is nothing blocking it, it might have already quit!

        let running_state = Rc::clone(&state);
        let (lock, _) = &*running_state;
        match lock.lock() {
            Ok(mut guard) => match *guard {
                State::Initialising => {
                    *guard = State::Waiting(future);
                }
                State::Exited(_) => (),
                _ => js::error("proc: new: unexpected state"),
            },
            Err(_) => js::error("proc: new: mutex poisoned"),
        }

        Ok(Self {
            state,
            output,

            set_module,
            init_module,
            init_runtime,
            read,

            print,
            print_err,
            quit,
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
                State::Initialising => return unexpected("state: initialising"),
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
