use js_sys::Error;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use web_sys::window;

use crate::js;

const CRLF: &str = "\r\n";

pub struct Terminal {
    term: js::Terminal,
    term_fit_addon: js::FitAddon,
}

impl Terminal {
    pub fn new(term: JsValue, term_fit_addon: JsValue) -> Self {
        let term: js::Terminal = term.into();
        let term_fit_addon: js::FitAddon = term_fit_addon.into();
        term.load_addon(&term_fit_addon);
        Self {
            term,
            term_fit_addon,
        }
    }

    pub fn open(&self) -> Result<(), Error> {
        self.term.open(
            &window()
                .ok_or(Error::new("not found: [window]"))?
                .document()
                .ok_or(Error::new("not found: [document]"))?
                .get_element_by_id("term")
                .ok_or(Error::new("not found: <#term>"))?
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
