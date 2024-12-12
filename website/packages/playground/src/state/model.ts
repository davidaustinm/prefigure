import { action, Action, Thunk, thunk } from "easy-peasy";
import { PreFigureCompiler } from "../worker/compiler";
import * as Comlink from "comlink";
import Worker from "../worker?worker";
import type { api } from "../worker";

// Make a local version of the compiler
let compiler = new PreFigureCompiler();
(window as any).comp = compiler;

const worker = Comlink.wrap<typeof api>(new Worker());
//// Create a worker-based instance of the compiler
(window as any).compWorker = worker.compiler;
// The types seem to be wrong. We are not awaiting a promise here. Instead we have a proxy object
// directly exposed to the main thread.
compiler = worker.compiler as any as Awaited<typeof worker.compiler>;

export interface PlaygroundModel {
    source: string;
    compiledSource: string;
    status: "" | "loadingPyodide";
    compileMode: "svg" | "tactile";
    prefigVersion: string;
    setPrefigVersion: Action<PlaygroundModel, string>;
    errorState: string;
    setSource: Action<PlaygroundModel, string>;
    setCompiledSource: Action<PlaygroundModel, string>;
    setStatus: Action<PlaygroundModel, "" | "loadingPyodide">;
    setErrorState: Action<PlaygroundModel, string>;
    setCompileMode: Action<PlaygroundModel, "svg" | "tactile">;
    loadPyodide: Thunk<PlaygroundModel>;
    compile: Thunk<PlaygroundModel>;
}

export const playgroundModel: PlaygroundModel = {
    source: `<diagram dimensions="(300,180)" margins="5">
  <coordinates bbox="(-5,0,5,6)">
    <grid/>
    <circle center="(-2,3.5)" radius="2" fill="blue" thickness="5"/>
    <ellipse center="(2,3)" axes="(1,2)" stroke="red"
	     rotate="pi/6" degrees="no"/>
    <label anchor="(3,1)">foo</label>
  </coordinates>
</diagram>`,
    compiledSource: "",
    compileMode: "svg",
    errorState: "",
    status: "",
    prefigVersion: "",
    setSource: action((state, payload) => {
        state.source = payload;
    }),
    setCompiledSource: action((state, payload) => {
        state.compiledSource = payload;
    }),
    setErrorState: action((state, payload) => {
        state.errorState = payload;
    }),
    setStatus: action((state, payload) => {
        state.status = payload;
    }),
    setPrefigVersion: action((state, payload) => {
        state.prefigVersion = payload;
    }),
    setCompileMode: action((state, payload) => {
        state.compileMode = payload;
    }),
    loadPyodide: thunk(async (actions) => {
        actions.setStatus("loadingPyodide");
        // Initialize Pyodide
        const indexURL = new URL(
            "./assets/pyodide",
            window.location.href,
        ).toString();
        await compiler.init({
            indexURL,
        });
        // Import `prefig` once so that it is cached
        await compiler.pyodide?.runPythonAsync("import prefig");
        // Get the version of `prefig` that is loaded
        const version = await compiler.pyodide?.runPythonAsync(
            "from importlib.metadata import version; version('prefig')",
        );
        actions.setPrefigVersion(version);
        actions.setStatus("");
    }),
    compile: thunk(async (actions, _, { getState }) => {
        const source = getState().source;
        const mode = getState().compileMode;
        try {
            actions.setErrorState("");
            const compiled = await compiler.compile(mode, source);
            // console.log("Got compiled results", compiled);
            actions.setCompiledSource(compiled.svg);
        } catch (e) {
            console.error(e);
            actions.setErrorState(String(e));
        }
    }),
};
