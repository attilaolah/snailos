use js_sys::Error;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

use crate::os::OS;

mod async_io;
mod binfs;
mod compilation_mode;
mod js;
mod os;
mod proc;
mod proc_closures;
mod term;

#[wasm_bindgen]
pub async fn boot(config: JsValue) -> Result<(), Error> {
    OS::new(config)?.boot().await
}
