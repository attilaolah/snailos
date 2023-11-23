use js_sys::{Array, Error, Function, Reflect, Uint8Array};
use wasm_bindgen::JsCast;
use web_sys::window;

use crate::js;

pub struct Terminal {
    term: js::Terminal,
    term_fit_addon: js::FitAddon,
}

impl Terminal {
    pub fn new(terminal: Function, fit_addon: Function) -> Result<Self, Error> {
        let term: js::Terminal = Reflect::construct(&terminal, &Array::new())?.into();
        let term_fit_addon: js::FitAddon = Reflect::construct(&fit_addon, &Array::new())?.into();
        term.load_fit_addon(&term_fit_addon)?;
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
                .dyn_into()
                .map_err(|_| Error::new("not an html element: <#term>"))?,
        )?;

        self.term_fit_addon.fit()?;
        // TODO: Re-run fit on window resize.

        Ok(())
    }

    pub fn write(&self, data: &[u8]) -> Result<(), Error> {
        if data.len() > 0 {
            // TODO: Is it possible to construct an ArrayBuffer view into the slice?
            let array = Uint8Array::new_with_length(data.len() as u32);
            array.copy_from(data);
            Ok(self.term.write(&array)?)
        } else {
            Ok(())
        }
    }

    pub fn writeln(&self, data: &[u8]) -> Result<(), Error> {
        self.write(data)?;
        self.write(b"\r\n")
    }
}
