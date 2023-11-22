use js_sys::{Error, Function, Promise};
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Condvar, Mutex};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

use crate::async_io::AsyncIo;
use crate::binfs::BinFs;
use crate::compilation_mode::unexpected;
use crate::js;
use crate::proc_closures::ProcClosures;

pub struct ProcessManager {
    cnt: u32,
    map: HashMap<u32, Process>,
    p_defer: Function,

    binfs: BinFs,
}

struct Process {
    state: Rc<(Mutex<State>, Condvar)>,
    output: Rc<AsyncIo>,

    #[allow(dead_code)]
    closures: ProcClosures,
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
        let mut closures = ProcClosures::new();

        let output = Rc::new(AsyncIo::new(p_defer));
        let state = Rc::new((Mutex::new(State::Init), Condvar::new()));

        let args_builder = js::Builder::new()
            .set("thisProgram", name)?
            .set("arguments", js::str_array(arguments))?;
        closures.add(args_builder.set_ref("os.set_module", ProcClosures::set_module())?);
        closures.add(args_builder.set_ref("os.init_module", ProcClosures::init_module())?);
        closures.add(args_builder.set_ref("os.init_runtime", ProcClosures::init_runtime())?);
        closures.add(args_builder.set_ref("os.read", ProcClosures::read())?);
        closures.add(args_builder.set_ref("print", ProcClosures::print(&output))?);
        closures.add(args_builder.set_ref("printErr", ProcClosures::print(&output))?);
        closures.add(args_builder.set_ref("exit", ProcClosures::exit(&state, &output))?);

        let new_state = match module.call1(&JsValue::null(), &args_builder.into()) {
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
            output,
            closures,
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
