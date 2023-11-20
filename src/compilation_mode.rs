use js_sys::Error;
use std::fmt;

pub enum CompilationMode {
    #[cfg(feature = "dbg")]
    Debug,
    #[cfg(not(any(feature = "dbg", feature = "opt")))]
    FastBuild,
    #[cfg(feature = "opt")]
    Optimised,
}

#[cfg(feature = "dbg")]
pub const COMPILATION_MODE: CompilationMode = CompilationMode::Debug;

#[cfg(not(any(feature = "dbg", feature = "opt")))]
pub const COMPILATION_MODE: CompilationMode = CompilationMode::FastBuild;

#[cfg(feature = "opt")]
pub const COMPILATION_MODE: CompilationMode = CompilationMode::Optimised;

impl fmt::Display for CompilationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(feature = "dbg")]
        let res = write!(f, "debug");
        #[cfg(not(any(feature = "dbg", feature = "opt")))]
        let res = write!(f, "fastbuild");
        #[cfg(feature = "opt")]
        let res = write!(f, "opt");

        res
    }
}

#[cfg(not(feature = "opt"))]
pub fn unexpected<T>(text: &str) -> Result<T, Error> {
    Err(Error::new(&format!("unexpected: {}", text)))
}

#[cfg(feature = "opt")]
pub fn unexpected<T>(_text: &str) -> Result<T, Error> {
    unreachable!()
}
