use std::cell::RefCell;

use js_sys::{
    eval, Array, Error, Function, JsString, Object, Promise, Reflect, Uint8Array, JSON::stringify,
};
use wasm_bindgen::{closure::Closure, prelude::wasm_bindgen, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::HtmlElement;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen]
    pub type Module;

    #[wasm_bindgen(method, getter, js_name=HEAPU8)]
    pub fn heap(this: &Module) -> Uint8Array;

    #[wasm_bindgen]
    pub type Deferred;

    #[wasm_bindgen(method)]
    pub fn resolve(this: &Deferred, value: &JsValue);

    #[wasm_bindgen(method)]
    pub fn reject(this: &Deferred, value: &JsValue);

    #[wasm_bindgen(method, getter)]
    pub fn promise(this: &Deferred) -> Promise;

    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    pub fn warn(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    pub fn error(s: &str);

    #[wasm_bindgen]
    pub type Terminal;

    #[wasm_bindgen(method, catch)]
    pub fn open(this: &Terminal, parent: &HtmlElement) -> Result<(), JsValue>;

    #[wasm_bindgen(method, catch, js_name = loadAddon)]
    pub fn load_fit_addon(this: &Terminal, addon: &FitAddon) -> Result<(), JsValue>;

    #[wasm_bindgen(method, catch)]
    pub fn write(this: &Terminal, data: &Uint8Array) -> Result<(), JsValue>;

    #[wasm_bindgen(method, js_name = write)]
    pub fn write_string(this: &Terminal, data: &str);

    #[wasm_bindgen(method, catch, js_name = onData)]
    pub fn on_data(
        this: &Terminal,
        callback: &Closure<dyn Fn(String)>,
    ) -> Result<Disposable, JsValue>;

    #[wasm_bindgen]
    pub type FitAddon;

    #[wasm_bindgen(method, catch)]
    pub fn fit(this: &FitAddon) -> Result<(), JsValue>;

    #[wasm_bindgen]
    pub type Disposable;

    #[wasm_bindgen(method, catch)]
    pub fn dispose(this: &Disposable) -> Result<(), JsValue>;
}

thread_local! {
pub static P_DEFER: RefCell<Option<Function>> = RefCell::new(None);
}

pub fn p_defer_init(p_defer: Function) {
    P_DEFER.with(|rc| *rc.borrow_mut() = Some(p_defer));
}

pub fn deferred() -> Result<Deferred, Error> {
    Ok(P_DEFER
        .with(|rc| rc.borrow().as_ref().map(|f| f.call0(&JsValue::null())))
        .expect("deferred: not ready")?
        .into())
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

pub fn str_array(items: &[&str]) -> Array {
    let array = Array::new_with_length(items.len() as u32);
    for (i, arg) in items.iter().enumerate() {
        array.set(i as u32, JsString::from(*arg).into());
    }
    array
}

/// Object builder.
pub struct Builder {
    obj: Object,
}

impl Builder {
    pub fn new() -> Self {
        Self { obj: Object::new() }
    }

    /// Sets the object key to the specified value.
    ///
    /// If the key contains dots, a nested structure will be created.
    pub fn set<T>(self, key: &str, value: T) -> Result<Self, Error>
    where
        T: Into<JsValue>,
    {
        let mut current = self.obj.clone();
        let parts: Vec<&str> = key.split('.').collect();

        for (i, part) in parts.iter().enumerate() {
            if parts.len() - 1 == i {
                continue;
            }
            if !Reflect::has(&current, &JsValue::from_str(part))? {
                let obj = Object::new();
                Reflect::set(&current, &JsValue::from_str(part), &obj)?;
                current = obj.clone();
            } else {
                current = Reflect::get(&current, &JsValue::from_str(part))
                    .unwrap()
                    .clone()
                    .into();
            }
        }

        Reflect::set(
            &current,
            &JsValue::from_str(parts.last().unwrap()),
            &value.into(),
        )?;

        Ok(self)
    }
}

impl Into<Object> for Builder {
    fn into(self) -> Object {
        self.obj
    }
}

impl Into<JsValue> for Builder {
    fn into(self) -> JsValue {
        self.obj.into()
    }
}
