/*
 * Webpack entry point.
 *
 * This file lists the other files in the order in which they need to be
 * bundled, as well as kickstarts the wasm-bindgen "start" entry point. When
 * "wasm-bindgen" is updated, this file might need updating to make sure the
 * relevant entry point is still called.
 *
 * See the documentation here:
 * https://rustwasm.github.io/wasm-bindgen/reference/attributes/on-rust-exports/start.html
 */

// Bazel --compilation_mode (-c) flag.
const COMPILATION_MODE = "${COMPILATION_MODE}";

// OS runtime.
import * as wasm from "./wasm_bg.wasm";
import { __wbg_set_wasm } from "./wasm_bg.js";
__wbg_set_wasm(wasm);
import * as os from "./wasm_bg.js";

// OS runtime dependencies.
// These are injected to the "boot" function below.
import { Terminal } from "xterm";
import { FitAddon } from "xterm-addon-fit";
import pDefer from "p-defer"

const deps = { Terminal, FitAddon, pDefer };

// Dynamic import.
// NOTE: We use eval() to prevent webpack from intercepting the import.
// TODO: Strip out the `eval` from the final "opt" build.
//args["import"] = import: (mod) => eval(`import(${JSON.stringify(mod)})`),


// Don't block the page load.
// This will still execute on the main thread, but in the next event loop.
setTimeout(os.boot.bind(os, deps), 0);
