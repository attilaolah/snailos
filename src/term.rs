use std::{cell::RefCell, rc::Rc};

use js_sys::{Array, Error, Function, Reflect, Uint8Array};
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::window;

use crate::{
    js,
    proc::{Pid, ProcessManager},
};

pub struct Terminal {
    term: Rc<js::Terminal>,
    term_fit_addon: js::FitAddon,
    owner: Rc<RefCell<Option<Pid>>>,

    #[allow(dead_code)]
    callbacks: Callbacks,
    disposables: Disposables,
}

struct Callbacks {
    on_data: Closure<dyn Fn(String)>,
}

struct Disposables {
    on_data: js::Disposable,
}

impl Terminal {
    pub fn new(
        terminal: Function,
        fit_addon: Function,
        proc: &Rc<ProcessManager>,
    ) -> Result<Self, Error> {
        let term: Rc<js::Terminal> =
            Rc::new(Reflect::construct(&terminal, &Array::of1(&js::Builder::new().into()))?.into());

        let owner = Rc::new(RefCell::new(None)); // detached
        let callbacks = Callbacks::new(proc, &owner, &term);
        let disposables = Disposables {
            on_data: term.on_data(&callbacks.on_data)?,
        };

        // Addons:
        let term_fit_addon: js::FitAddon = Reflect::construct(&fit_addon, &Array::new())?.into();
        term.load_fit_addon(&term_fit_addon)?;

        Ok(Self {
            term,
            term_fit_addon,
            owner,
            callbacks,
            disposables,
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
            // NOTE: We need to make a copy here, because write() will return before consuming the
            // data. To avoid the copy, we could construct a Uint8Array view directly into our
            // heap, but then we'd need to keep the memory alive until the write callback fires.
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

    pub fn attach_to(&self, pid: Pid) {
        self.owner.borrow_mut().replace(pid);
    }
}

impl Callbacks {
    fn new(
        proc: &Rc<ProcessManager>,
        pid: &Rc<RefCell<Option<Pid>>>,
        term: &Rc<js::Terminal>,
    ) -> Self {
        Self {
            on_data: Self::on_data(proc.clone(), pid.clone(), term.clone()),
        }
    }

    fn on_data(
        proc: Rc<ProcessManager>,
        pid: Rc<RefCell<Option<Pid>>>,
        term: Rc<js::Terminal>,
    ) -> Closure<dyn Fn(String)> {
        Closure::new(move |input: String| {
            // Replace "\r" => "\n".
            let text = if input == "\r" {
                "\n".to_string()
            } else {
                input
            };
            // Echo back everything.
            term.write_string(&text);

            match *pid.borrow() {
                None => js::warn("term: on_data: detached"),
                Some(pid) => {
                    if let Err(err) = proc.stdin_write(pid, text.as_bytes().to_vec()) {
                        js::log(&format!(
                            "term: on_data: write error: {}",
                            err.as_string().unwrap_or("n/a".to_string())
                        ));
                    }
                }
            }
        })
    }
}
