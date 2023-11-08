use js_sys::Error;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use web_sys::window;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = Terminal)]
    type JsTerm;

    #[wasm_bindgen(method)]
    fn open(this: &JsTerm, parent: &JsValue);

    #[wasm_bindgen(method)]
    fn write(this: &JsTerm, data: &JsValue, callback: JsValue);
}

pub struct Terminal {
    js: JsValue,
}

impl Terminal {
    pub fn new(js: JsValue) -> Self {
        Self { js }
    }

    pub fn open(&self) -> Result<(), Error> {
        // TODO: Avoid cloning every time!
        // This should be moved in the constructor, but that causes lifetime issues…
        let js: JsTerm = self.js.clone().into();
        js.open(
            &window()
                .ok_or(Error::new("window not found"))?
                .document()
                .ok_or(Error::new("document not found"))?
                .get_element_by_id("term")
                .ok_or(Error::new("term not found"))?
                .into(),
        );

        Ok(())
    }

    pub fn write(&self, text: &str) -> Result<(), Error> {
        // TODO: Avoid cloning every time!
        // This should be moved in the constructor, but that causes lifetime issues…
        let js: JsTerm = self.js.clone().into();
        js.write(&text.into(), JsValue::undefined());

        Ok(())
    }
}
