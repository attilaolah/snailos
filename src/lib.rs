use js_sys::{Error, Reflect};
use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};

use crate::proc::ProcessManager;
use crate::term::Terminal;

mod async_io;
mod binfs;
mod proc;
mod term;

// TODO:
//
// - users: simple user/group management
// - mnt: simple mount point management
// - binfs: read-only fs mounted at /bin
// - other signals
//
// TODO: Structure the virtual filesystem like so:
// /bin/busybox is the JS binary without any extension
// /usr/wasm/cid.wasm is the WASM binary that it loads, where "cid" is the content ID.

struct SnailOs {
    proc: ProcessManager,
    term: Terminal,
}

impl SnailOs {
    fn new(config: JsValue) -> Result<Self, Error> {
        let proc = ProcessManager::new(
            Reflect::get(&config, &"import".into())?.dyn_into()?,
            Reflect::get(&config, &"p_defer".into())?.dyn_into()?,
        );
        let term = Terminal::new(
            Reflect::get(&config, &"term".into())?,
            Reflect::get(&config, &"term_fit_addon".into())?,
        );

        Ok(Self { proc, term })
    }

    async fn boot(&mut self) -> Result<(), Error> {
        self.term.open()?;

        self.term.writeln("BOOT: Starting BusyBox shell…")?;
        let pid = self.proc.exec("/bin/busybox").await?;

        while let Some(output) = self.proc.wait_output(pid).await? {
            for chunk in output {
                self.term.write(&chunk.as_string().unwrap())?;
            }
        }

        self.term.writeln(&format!(
            "\r\n\nSHUTDOWN: BusyBox shell exited with code {}",
            self.proc.wait_quit(pid).await?,
        ))?;
        Ok(())
    }
}

#[wasm_bindgen]
pub async fn boot(config: JsValue) -> Result<(), Error> {
    SnailOs::new(config)?.boot().await
}
