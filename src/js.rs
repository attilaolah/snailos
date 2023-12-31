/// Contains JS bindings.
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen]
    pub type Terminal;

    #[wasm_bindgen(method)]
    pub fn open(this: &Terminal, parent: &JsValue);

    #[wasm_bindgen(method, js_name = loadAddon)]
    pub fn load_addon(this: &Terminal, addon: &JsValue);

    #[wasm_bindgen(method)]
    pub fn write(this: &Terminal, data: &JsValue, callback: JsValue);

    #[wasm_bindgen]
    pub type FitAddon;

    #[wasm_bindgen(method)]
    pub fn fit(this: &FitAddon);
}
