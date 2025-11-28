// Shorthand used by wrapped functions in libwrap, under //src/wrap.
const OS = Module.os;

/*
 * Pass a reference to this module back to the process manager.
 *
 * This allows the process manager to gain direct access to this module's heap,
 * as well as any internals. Direct heap access can speed up interop since
 * values can be written from one WebAssembly module directly to the other,
 * without the need to create JavaScript objects inbetween.
 */
OS.set_module(Module);

// Signal the process manager when runtime initialisation has completed.
Module.onRuntimeInitialized = () => {
  OS.init_runtime();
}
