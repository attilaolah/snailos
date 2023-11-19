use js_sys::{Array, Error, Function, JsString, Object, Promise, Reflect};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    mpsc,
};
use wasm_bindgen::{closure::Closure, prelude::wasm_bindgen, JsValue};
use wasm_bindgen_futures::JsFuture;

use crate::binfs::BinFs;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

pub struct ProcessManager {
    cnt: AtomicU32,
    map: HashMap<u32, Process>,
    import: Function,

    binfs: BinFs,
}

enum State {
    Loading,
    Running,
    Exited(i32),
}

struct Process {
    name: String,
    arguments: Vec<String>,
    state: State,
}

impl ProcessManager {
    pub fn new(import: Function) -> Self {
        let cnt = AtomicU32::new(1);
        let map = HashMap::new();
        let binfs = BinFs::new("/bin");
        Self {
            cnt,
            map,
            import,
            binfs,
        }
    }

    /// Executes the given binary file and returns the exit code.
    pub async fn exec(&mut self, file_path: &str) -> Result<i32, Error> {
        let resolved_path = self
            .binfs
            .resolve(file_path)
            .ok_or(Error::new(&format!("failed to resolve: {}", file_path)))?;

        let module_loader = self
            .load_module(&resolved_path.to_string_lossy().to_string())
            .await?;

        let process = self.map.entry(self.next_pid()).or_insert_with(|| {
            Process::new(
                &resolved_path
                    .file_stem()
                    .unwrap() // already validated above
                    .to_string_lossy()
                    .to_string(),
                &[],
            )
        });

        Ok(process.run(module_loader).await?)
    }

    async fn load_module(&self, path: &str) -> Result<Function, Error> {
        let promise: Promise = self
            .import
            .call1(&JsValue::undefined(), &path.into())?
            .into();
        Ok(Reflect::get(&JsFuture::from(promise).await?, &"default".into())?.into())
    }

    fn next_pid(&self) -> u32 {
        self.cnt.fetch_add(1, Ordering::SeqCst)
    }
}

impl Process {
    fn new(name: &str, arguments: &[&str]) -> Self {
        let name = name.to_string();
        let arguments: Vec<String> = arguments.iter().map(|&s| s.to_string()).collect();
        let state = State::Loading;

        Self {
            name,
            arguments,
            state,
        }
    }

    /// Executes the process and returns its exit code.
    async fn run(&mut self, module_loader: Function) -> Result<i32, Error> {
        let args = Object::new();
        let args_array = Array::new();
        for s in &self.arguments {
            let js_string = JsString::from(s.as_str());
            args_array.push(&js_string);
        }

        Reflect::set(&args, &"thisProgram".into(), &self.name.clone().into())?;
        Reflect::set(&args, &"arguments".into(), &args_array.into())?;

        let (stdout, fdr) = mpsc::channel();
        let stderr = stdout.clone();

        let print: Closure<dyn Fn(_)> = Closure::new(move |text: JsString| {
            log(&format!(">1: {}", text.to_string()));
            stdout.send(text).unwrap()
        });
        let print_err: Closure<dyn Fn(_)> = Closure::new(move |text: JsString| {
            log(&format!(">2: {}", text.to_string()));
            stderr.send(text).unwrap()
        });

        Reflect::set(&args, &"print".into(), &print.as_ref())?;
        Reflect::set(&args, &"printErr".into(), &print_err.as_ref())?;

        let (quits, quitr) = mpsc::channel();
        let quit: Closure<dyn Fn(_)> = Closure::new(move |code: i32| quits.send(code).unwrap());
        Reflect::set(&args, &"quit".into(), &quit.as_ref())?;

        self.state = State::Running;
        let promise: Promise = module_loader.call1(&JsValue::undefined(), &args)?.into();
        JsFuture::from(promise).await?;

        let exit_code = quitr.recv().unwrap_or(-1);
        self.state = State::Exited(exit_code);

        Ok(exit_code)
    }
}
