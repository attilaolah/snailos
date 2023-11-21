use js_sys::{Array, Error, Function, Reflect};
use wasm_bindgen::JsValue;
use web_sys::window;

use crate::js;

const CRLF: &str = "\r\n";

pub struct Terminal {
    term: js::Terminal,
    term_fit_addon: js::FitAddon,
}

impl Terminal {
    pub fn new(terminal: Function, fit_addon: Function) -> Result<Self, Error> {
        let term: js::Terminal = Reflect::construct(&terminal, &Array::new())?.into();
        let term_fit_addon: js::FitAddon = Reflect::construct(&fit_addon, &Array::new())?.into();
        term.load_addon(&term_fit_addon);
        Ok(Self {
            term,
            term_fit_addon,
        })
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
