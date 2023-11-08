use js_sys::{Error, Reflect};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

use crate::term::Terminal;

mod term;

#[wasm_bindgen]
pub struct SnailOs {
    term: Terminal,
}

#[wasm_bindgen]
impl SnailOs {
    #[wasm_bindgen(constructor)]
    pub fn new(config: JsValue) -> Result<SnailOs, Error> {
        let term = Terminal::new(Reflect::get(&config, &"term".into())?);
        Ok(Self { term })
    }

    pub fn run(&self) -> Result<(), Error> {
        self.term.open()?;
        self.term.write("Hello, Snails!")?;

        Ok(())
    }
}
