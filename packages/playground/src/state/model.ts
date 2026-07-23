import {
    action,
    Action,
    Thunk,
    thunk,
    Computed,
    computed,
    ThunkOn,
    thunkOn,
} from "easy-peasy";
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
    annotations: string;
    status: "" | "loadingPyodide" | "compiling";
    compileMode: "svg" | "tactile";
    prefigVersion: string;
    setPrefigVersion: Action<PlaygroundModel, string>;
    errorState: string;
    setSource: Action<PlaygroundModel, string>;
    onSetSource: ThunkOn<PlaygroundModel>;
    setCompiledSource: Action<PlaygroundModel, string>;
    setAnnotations: Action<PlaygroundModel, string>;
    setStatus: Action<PlaygroundModel, "" | "loadingPyodide" | "compiling">;
    setErrorState: Action<PlaygroundModel, string>;
    setCompileMode: Action<PlaygroundModel, "svg" | "tactile">;
    onSetCompileMode: ThunkOn<PlaygroundModel>;
    loadPyodide: Thunk<PlaygroundModel>;
    compile: Thunk<PlaygroundModel>;
    needsCompile: Computed<PlaygroundModel, boolean>;
    /**
     * The state of the source code when it was last compiled.
     */
    lastCompileState: { source: string; mode: "svg" | "tactile" };
    saveCompileState: Action<PlaygroundModel>;
}

export const playgroundModel: PlaygroundModel = {
    source: `<diagram dimensions="(300,300)" margins="5">
  <definition>f(x)=2.5-x^2/2</definition>
  <definition>a = 1</definition>
  <coordinates bbox="(-4,-4,4,4)">
    <grid-axes xlabel="x" ylabel="y"/>
    <graph at="graph" function="f"/>
    <tangent-line at="tangent" function="f" point="a"/>
    <point at="point" p="(a,f(a))" alignment="ne">
      <m>(a,f(a))</m>
    </point>
  </coordinates>

  <annotations>
    <annotation ref="figure"
                text="The graph of a function and its tangent line at the point a equals 1">
      <annotation ref="graph-group" text="The graph and its tangent line">
        <annotation ref="graph" text="The graph of the function f" sonify="yes"/>
        <annotation ref="point" text="The point a comma f of a"/>
        <annotation ref="tangent" text="The tangent line to the graph of f at the point"/>
      </annotation>
    </annotation>
  </annotations>
</diagram>`,
    compiledSource: "",
    annotations: "",
    compileMode: "svg",
    errorState: "",
    status: "",
    prefigVersion: "",
    lastCompileState: { source: "", mode: "svg" },
    needsCompile: computed(
        (state) =>
            state.source !== state.lastCompileState.source ||
            state.compileMode !== state.lastCompileState.mode,
    ),
    saveCompileState: action((state, payload) => {
        state.lastCompileState.source = state.source;
        state.lastCompileState.mode = state.compileMode;
    }),
    setSource: action((state, payload) => {
        state.source = payload;
    }),
    setCompiledSource: action((state, payload) => {
        state.compiledSource = payload;
    }),
    setAnnotations: action((state, payload) => {
        state.annotations = payload;
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
            actions.setStatus("compiling");
            const compiled = await compiler.compile(mode, source);
            // console.log("Got compiled results", compiled);
            actions.setCompiledSource(compiled.svg);
            actions.setAnnotations(compiled.annotations || "");
            actions.saveCompileState();
        } catch (e) {
            console.error(e);
            actions.setErrorState(String(e));
        } finally {
            actions.setStatus("");
        }
    }),
    /**
     * Whenever the compile mode changes, we want to recompile as a side effect.
     */
    onSetCompileMode: thunkOn(
        (actions, storeActions) => actions.setCompileMode,
        (actions, target, { getState }) => {
            if (getState().needsCompile) {
                actions.compile();
            }
        },
    ),
    /**
     * Whenever the source changes, we want to debounce and then recompile as a side effect.
     */
    onSetSource: thunkOn(
        (actions, storeActions) => [actions.loadPyodide, actions.setSource],
        async (actions, target, { getState }) => {
            // Wait a maximum of 2 minutes if we are still loading pyodide
            let timeStart = Date.now();
            while (
                getState().status === "loadingPyodide" &&
                Date.now() - timeStart < 120000
            ) {
                await sleep(100);
            }

            // Debounce the compile
            await sleep(500);

            // Wait at most 1 second if we are compiling
            timeStart = Date.now();
            while (
                getState().status === "compiling" &&
                Date.now() - timeStart < 1000
            ) {
                await sleep(100);
            }

            // If we are still compiling or we no longer need to compile, give up
            if (getState().status === "compiling" || !getState().needsCompile) {
                return;
            }
            // Do the compile
            await actions.compile();
        },
    ),
};

/**
 * Returns a promise that sleeps for the requested time (in milliseconds).
 */
function sleep(ms: number) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}
