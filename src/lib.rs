use js_sys::{Error, Reflect};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

use crate::term::Terminal;

mod term;

pub struct SnailOs {
    term: Terminal,
}

impl SnailOs {
    pub fn new(config: JsValue) -> Result<SnailOs, Error> {
        let term = Terminal::new(
            Reflect::get(&config, &"term".into())?,
            Reflect::get(&config, &"term_fit_addon".into())?,
        );
        Ok(Self { term })
    }

    pub fn run(&self) -> Result<(), Error> {
        self.term.open()?;
        self.term.write("Hello, Snails!")?;

        Ok(())
    }
}

#[wasm_bindgen]
pub fn main(config: JsValue) -> Result<(), Error> {
    SnailOs::new(config)?.run()?;

    Ok(())
}
