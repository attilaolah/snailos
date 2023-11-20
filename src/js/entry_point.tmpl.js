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

// Dependencies, injected.
import { Terminal } from "xterm";
import { FitAddon } from "xterm-addon-fit";
import pDefer from "p-defer"

const boot = os.boot.bind(os, {
  // TODO: Inject classes, don't create objects here.
  term: new Terminal(),
  term_fit_addon: new FitAddon(),
  // NOTE: We use eval() to prevent webpack from intercepting the import.
  // TODO: Strip it out in the final build.
  import: (mod) => eval(`import(${JSON.stringify(mod)})`),
  p_defer: pDefer,
  COMPILATION_MODE,
});

setTimeout(boot, 0);
