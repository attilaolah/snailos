use js_sys::Error;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use web_sys::window;

const CRLF: &str = "\r\n";

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = Terminal)]
    type JsTerm;

    #[wasm_bindgen(method)]
    fn open(this: &JsTerm, parent: &JsValue);

    #[wasm_bindgen(method, js_name = loadAddon)]
    fn load_addon(this: &JsTerm, addon: &JsValue);

    #[wasm_bindgen(method)]
    fn write(this: &JsTerm, data: &JsValue, callback: JsValue);

    #[wasm_bindgen(js_name = FitAddon)]
    type JsFitAddon;

    #[wasm_bindgen(method)]
    fn fit(this: &JsFitAddon);
}

pub struct Terminal {
    term: JsTerm,
    term_fit_addon: JsFitAddon,
}

impl Terminal {
    pub fn new(term: JsValue, term_fit_addon: JsValue) -> Self {
        let term: JsTerm = term.into();
        let term_fit_addon: JsFitAddon = term_fit_addon.into();
        term.load_addon(&term_fit_addon);
        Self {
            term,
            term_fit_addon,
        }
    }

    pub fn open(&self) -> Result<(), Error> {
        self.term.open(
            &window()
                .ok_or(Error::new("window not found"))?
                .document()
                .ok_or(Error::new("document not found"))?
                .get_element_by_id("term")
                .ok_or(Error::new("term not found"))?
                .into(),
        );

        self.term_fit_addon.fit();
        // TODO: Re-run fit on window resize.

        Ok(())
    }

    pub fn write(&self, text: &str) -> Result<(), Error> {
        self.term.write(&text.into(), JsValue::undefined());

        Ok(())
    }

    pub fn writeln(&self, text: &str) -> Result<(), Error> {
        self.write(text)?;
        self.write(CRLF)
    }
}
