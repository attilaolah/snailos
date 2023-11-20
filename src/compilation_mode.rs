enum CompilationMode{
    Debug,
    FastBuild,
    Optimised,
}

#[cfg(feature = "dbg")]
const COMPILATION_MODE: CompilationMode = CompilationMode::Debug;

#[cfg(not(any(feature = "dbg", feature = "opt")))]
const COMPILATION_MODE: CompilationMode = CompilationMode::FastBuild;

#[cfg(feature = "opt")]
const COMPILATION_MODE: CompilationMode = CompilationMode::Opt;
