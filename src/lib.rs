use js_sys::Error;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

use crate::os::OS;

mod async_io;
mod binfs;
mod closures;
mod compilation_mode;
mod js;
mod os;
mod proc;
mod term;

#[wasm_bindgen]
pub async fn boot(config: JsValue) -> Result<(), Error> {
    OS::new(config)?.boot().await
}
