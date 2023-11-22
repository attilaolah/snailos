use js_sys::{eval, Error, Function, Object, Promise, Reflect, JSON::stringify};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use wasm_bindgen_futures::JsFuture;

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

    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    pub fn warn(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    pub fn error(s: &str);
}

pub async fn load_module(path: &str) -> Result<Function, Error> {
    // Currently wasm-bindgen doesn't seem to support dynamic imports.
    // As a fallback, we eval(…) the import statement. Not very elegant, but it works.
    let promise: Promise = eval(&format!(
        "import({})",
        &stringify(&path.into())?.as_string().unwrap()
    ))?
    .into();

    Ok(Reflect::get(&JsFuture::from(promise).await?, &"default".into())?.into())
}

pub struct Builder {
    obj: Object,
}

impl Builder {
    pub fn new() -> Self {
        Self { obj: Object::new() }
    }

    pub fn set(self, name: &str, value: &JsValue) -> Result<Self, Error> {
        Reflect::set(&self.obj, &JsValue::from_str(name), value)?;
        Ok(self)
    }
}

impl Into<Object> for Builder {
    fn into(self) -> Object {
        self.obj
    }
}
