use js_sys::Error;

enum CompilationMode {
    Debug,
    FastBuild,
    Optimised,
}

#[cfg(feature = "dbg")]
const COMPILATION_MODE: CompilationMode = CompilationMode::Debug;

#[cfg(not(any(feature = "dbg", feature = "opt")))]
const COMPILATION_MODE: CompilationMode = CompilationMode::FastBuild;

#[cfg(feature = "opt")]
const COMPILATION_MODE: CompilationMode = CompilationMode::Optimised;

#[cfg(not(feature = "opt"))]
pub fn unexpected<T>(text: &str) -> Result<T, Error> {
    Err(Error::new(&format!("unexpected: {}", text)))
}

#[cfg(feature = "opt")]
pub fn unexpected<T>(_text: &str) -> Result<T, Error> {
    unreachable!()
}
