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

import * as wasm from "../src/src_bg.wasm";
import { __wbg_set_wasm } from "../src/src_bg.js";
__wbg_set_wasm(wasm);
export * from "../src/src_bg.js";

// Start the binary.
wasm.__wbindgen_start();
