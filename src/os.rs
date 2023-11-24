use js_sys::{Error, Reflect};
use wasm_bindgen::JsValue;

use crate::{
    async_io::STDOUT, compilation_mode::COMPILATION_MODE, js, proc::ProcessManager, term::Terminal,
};

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
    term: Terminal,
    proc: ProcessManager,
}

impl OS {
    pub fn new(config: JsValue) -> Result<Self, Error> {
        js::p_defer_init(Reflect::get(&config, &"pDefer".into())?.into());

        Ok(Self {
            proc: ProcessManager::new(),
            term: Terminal::new(
                Reflect::get(&config, &"Terminal".into())?.into(),
                Reflect::get(&config, &"FitAddon".into())?.into(),
            )?,
        })
    }

    pub async fn boot(&mut self) -> Result<(), Error> {
        self.term.open()?;

        self.term
            .writeln(&format!("_@/\" OS {}-{}, booting…", VERSION, COMPILATION_MODE).as_bytes())?;
        self.term.writeln(b"")?;

        let pid = self.proc.exec("/bin/busybox", &["hush"]).await?;

        // TODO: Merge stdout and stderr!
        // For now, let's just display the output of stdout.
        while let Some(chunks) = self.proc.wait_data(pid, STDOUT).await? {
            for chunk in chunks.into_iter() {
                self.term.writeln(chunk.as_slice())?;
            }
        }

        let exit_code = self.proc.wait_quit(pid).await?;
        self.term.writeln(b"")?;
        self.term.writeln(&format!("EXIT {}", exit_code).as_bytes())
    }
}
