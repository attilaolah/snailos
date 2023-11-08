use js_sys::Error;
use wasm_bindgen::prelude::wasm_bindgen;
use web_sys::console;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), Error> {
    console::log_1(&"Hello, World!".into());

    Ok(())
}
