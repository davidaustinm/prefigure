import { expose } from "comlink";
import { PreFigureCompiler } from "./compiler";
import {
    PreFigureWasmCompiler,
    loadMathjaxWasm,
    loadRatexWasm,
} from "./compiler-wasm";

// The Python-via-Pyodide compiler (current default).
const compiler = new PreFigureCompiler();

// The Rust-via-WebAssembly compilers (drop-in alternatives). Each is selected
// by the main thread when the user picks an engine in the UI:
//   - wasmMathjaxCompiler: renders math with the host's MathJax.
//   - wasmRatexCompiler:   renders math with the embedded pure-Rust RaTeX engine.
const wasmMathjaxCompiler = new PreFigureWasmCompiler(loadMathjaxWasm);
const wasmRatexCompiler = new PreFigureWasmCompiler(loadRatexWasm);

const add = (a: number, b: number) => a + b;

export const api = {
    compiler,
    wasmMathjaxCompiler,
    wasmRatexCompiler,
    add,
};

expose(api);
