use js_sys::{Array, Error, Function, JsString, Object, Promise, Reflect};
use wasm_bindgen::{closure::Closure, prelude::wasm_bindgen, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

use crate::term::Terminal;

mod term;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// TODO:
//
// - proc: simple process tracking
// - users: simple user/group management
// - mnt: simple mount point management
// - binfs: read-only fs mounted at /bin
// - other signals

struct SnailOs {
    term: Terminal,
    import: Function,
}

impl SnailOs {
    fn new(config: JsValue) -> Result<SnailOs, Error> {
        let term = Terminal::new(
            Reflect::get(&config, &"term".into())?,
            Reflect::get(&config, &"term_fit_addon".into())?,
        );
        let import = Reflect::get(&config, &"import".into())?.dyn_into()?;

        Ok(Self { term, import })
    }

    async fn boot(&self) -> Result<(), Error> {
        self.term.open()?;

        self.term.writeln("BOOT: Starting busybox shell…")?;

        // TODO: Structure the virtual filesystem like so:
        // /bin/busybox is the JS binary without any extension
        // /usr/wasm/cid.wasm is the WASM binary that it loads, where "cid" is the content ID.
        self.exec("busybox").await
    }

    async fn exec(&self, binary: &str) -> Result<(), Error> {
        let module_loader = self.load_module(&format!("/bin/{}.js", binary)).await?;

        let module_arg = Object::new();

        Reflect::set(&module_arg, &"thisProgram".into(), &"busybox".into())?;
        Reflect::set(&module_arg, &"arguments".into(), &Array::new())?;

        let print: Closure<dyn Fn(_)> = Closure::new(|text: JsString| {
            log(&format!(">1: {}", text.to_string()));
        });
        Reflect::set(&module_arg, &"print".into(), print.as_ref())?;
        // TODO: Attach to the process object!
        print.forget();

        let print_err: Closure<dyn Fn(_)> = Closure::new(|text: JsString| {
            log(&format!(">2: {}", text.to_string()));
        });
        Reflect::set(&module_arg, &"printErr".into(), print_err.as_ref())?;
        // TODO: Attach to the process object!
        print_err.forget();

        let quit: Closure<dyn Fn(_)> = Closure::new(|code: i32| {
            log(&format!("EXIT {}", code));
        });
        Reflect::set(&module_arg, &"quit".into(), quit.as_ref())?;
        // TODO: Attach to the process object!
        quit.forget();

        module_loader.call1(&JsValue::undefined(), &module_arg)?;

        // TODO: Execute the shell, like this:
        // const module = await import(url)
        // await module.default({
        //   thisProgram: "busybox",
        //   arguments: ["foo", "bar"],
        //   print: (str) => console.log(`OUT: ${str}`),
        //   printErr: (str) => console.log(`ERR: ${str}`),
        //   quit: (code) => console.log(`EXIT: ${code}`),
        //   // wasmBinary if pre-fetched in parallel with the module,
        //   // which would likely speed things up, also the only real way
        //   // to load the binary if we're using a CAS filesystem
        // })

        Ok(())
    }

    async fn load_module(&self, path: &str) -> Result<Function, Error> {
        let promise: Promise = self
            .import
            .call1(&JsValue::undefined(), &path.into())?
            .into();
        Ok(Reflect::get(&JsFuture::from(promise).await?, &"default".into())?.into())
    }
}

#[wasm_bindgen]
pub async fn boot(config: JsValue) -> Result<(), Error> {
    SnailOs::new(config)?.boot().await?;

    // TODO: SHUTDOWN, what now?
    Ok(())
}
