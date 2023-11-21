use js_sys::{Error, Reflect};
use wasm_bindgen::JsValue;

use crate::compilation_mode::COMPILATION_MODE;
use crate::proc::ProcessManager;
use crate::term::Terminal;

const VERSION: &str = env!("CARGO_PKG_VERSION");

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

pub struct OS {
    proc: ProcessManager,
    term: Terminal,
}

impl OS {
    pub fn new(config: JsValue) -> Result<Self, Error> {
        let proc = ProcessManager::new(
            //Reflect::get(&config, &"import".into())?.into(),
            Reflect::get(&config, &"pDefer".into())?.into(),
        );
        let term = Terminal::new(
            Reflect::get(&config, &"Terminal".into())?.into(),
            Reflect::get(&config, &"FitAddon".into())?.into(),
        )?;

        Ok(Self { proc, term })
    }

    pub async fn boot(&mut self) -> Result<(), Error> {
        self.term.open()?;

        self.term.writeln(&format!(
            "_@/\" OS {}-{}, booting…",
            VERSION, COMPILATION_MODE,
        ))?;
        self.term.writeln("")?;

        let pid = self.proc.exec("/bin/busybox", &["hush"]).await?;
        while let Some(output) = self.proc.wait_output(pid).await? {
            for chunk in output {
                self.term.write(&chunk.as_string().unwrap())?;
            }
        }

        self.term
            .writeln(&format!("\r\nEXIT {}", self.proc.wait_quit(pid).await?))?;
        Ok(())
    }
}
