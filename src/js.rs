use js_sys::{eval, Array, Error, Function, JsString, Object, Promise, Reflect, JSON::stringify};
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
        self.set_item(key, value)?;
        Ok(self)
    }

    fn set_item<T>(&self, key: &str, value: T) -> Result<(), Error>
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

        Ok(())
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
