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
import { SnailOs, __wbg_set_wasm } from "./wasm_bg.js";
__wbg_set_wasm(wasm);
export * from "./wasm_bg.js";

// Dependencies, injected.
import { Terminal } from "xterm/lib/xterm.js";

const os = new SnailOs({
  term: new Terminal(),
});

if (COMPILATION_MODE !== "opt") {
  window.os = os;
}

os.run();
