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
import * as Comlink from "comlink";
import Worker from "../worker?worker";
import type { api } from "../worker";
import { getSourceFromQueryParam } from "../utils/source-query-param";

/**
 * Which implementation compiles the diagram:
 *   - "pyodide":      the Python package running in Pyodide.
 *   - "wasm-mathjax": the Rust port compiled to WebAssembly, math via host MathJax.
 *   - "wasm-ratex":   the Rust port compiled to WebAssembly, math via embedded RaTeX.
 */
export type Engine = "pyodide" | "wasm-mathjax" | "wasm-ratex";

const worker = Comlink.wrap<typeof api>(new Worker());
(window as any).compWorker = worker.compiler;

// The worker exposes three drop-in compilers. The Comlink types describe
// promises, but each is a proxy object we call methods on directly (each call
// returns a promise).
const pyodideCompiler = worker.compiler as any as Awaited<
    typeof worker.compiler
>;
const wasmMathjaxCompiler = worker.wasmMathjaxCompiler as any as Awaited<
    typeof worker.wasmMathjaxCompiler
>;
const wasmRatexCompiler = worker.wasmRatexCompiler as any as Awaited<
    typeof worker.wasmRatexCompiler
>;

/** The wasm compiler backing a given engine. */
function wasmCompilerFor(engine: Engine) {
    return engine === "wasm-ratex" ? wasmRatexCompiler : wasmMathjaxCompiler;
}

/** The engine requested via the `?engine=` query parameter, if any. */
function initialEngine(): Engine {
    try {
        const requested = new URLSearchParams(window.location.search).get(
            "engine",
        );
        switch (requested) {
            case "wasm-ratex":
                return "wasm-ratex";
            case "wasm-mathjax":
            case "wasm": // legacy alias for the MathJax-backed wasm build
                return "wasm-mathjax";
            default:
                return "pyodide";
        }
    } catch {
        return "pyodide";
    }
}

/**
 * Persist the selected engine in the URL so it survives a page refresh. The
 * default ("pyodide") drops the parameter to keep shared links clean.
 */
function writeEngineToUrl(engine: Engine): void {
    try {
        const url = new URL(window.location.href);
        if (engine === "pyodide") {
            url.searchParams.delete("engine");
        } else {
            url.searchParams.set("engine", engine);
        }
        window.history.replaceState(null, "", url.toString());
    } catch {
        // Non-browser or restricted environment; nothing to persist.
    }
}

export interface PlaygroundModel {
    source: string;
    compiledSource: string;
    annotations: string;
    status: "" | "loadingPyodide" | "loadingWasm" | "compiling";
    compileMode: "svg" | "tactile";
    engine: Engine;
    setEngine: Action<PlaygroundModel, Engine>;
    onSetEngine: ThunkOn<PlaygroundModel>;
    prefigVersion: string;
    setPrefigVersion: Action<PlaygroundModel, string>;
    errorState: string;
    setSource: Action<PlaygroundModel, string>;
    onSetSource: ThunkOn<PlaygroundModel>;
    setCompiledSource: Action<PlaygroundModel, string>;
    setAnnotations: Action<PlaygroundModel, string>;
    setStatus: Action<
        PlaygroundModel,
        "" | "loadingPyodide" | "loadingWasm" | "compiling"
    >;
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
    source:
        getSourceFromQueryParam() ??
        `<diagram dimensions="(300,300)" margins="5">
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
    engine: initialEngine(),
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
    setEngine: action((state, payload) => {
        state.engine = payload;
    }),
    loadPyodide: thunk(async (actions, _, { getState }) => {
        // Despite the name, this initializes whichever engine is selected. It
        // is a trigger for `onSetSource`, so the name is kept for stability.
        const engine = getState().engine;
        if (engine !== "pyodide") {
            actions.setStatus("loadingWasm");
            const compiler = wasmCompilerFor(engine);
            await compiler.init();
            actions.setPrefigVersion(await compiler.version());
            actions.setStatus("");
            return;
        }
        actions.setStatus("loadingPyodide");
        // Initialize Pyodide
        const indexURL = new URL(
            "./assets/pyodide",
            window.location.href,
        ).toString();
        await pyodideCompiler.init({
            indexURL,
        });
        // Import `prefig` once so that it is cached
        await pyodideCompiler.pyodide?.runPythonAsync("import prefig");
        // Get the version of `prefig` that is loaded
        const version = await pyodideCompiler.pyodide?.runPythonAsync(
            "from importlib.metadata import version; version('prefig')",
        );
        actions.setPrefigVersion(version);
        actions.setStatus("");
    }),
    compile: thunk(async (actions, _, { getState }) => {
        const source = getState().source;
        const mode = getState().compileMode;
        const engine = getState().engine;
        const activeCompiler =
            engine === "pyodide" ? pyodideCompiler : wasmCompilerFor(engine);
        try {
            actions.setErrorState("");
            actions.setStatus("compiling");
            const compiled = await activeCompiler.compile(mode, source);
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
     * Whenever the engine changes, initialize the newly-selected engine (if it
     * has not been loaded yet) and recompile so the view reflects it.
     */
    onSetEngine: thunkOn(
        (actions, storeActions) => actions.setEngine,
        async (actions, target) => {
            writeEngineToUrl(target.payload);
            await actions.loadPyodide();
            await actions.compile();
        },
    ),
    /**
     * Whenever the source changes, we want to debounce and then recompile as a side effect.
     */
    onSetSource: thunkOn(
        (actions, storeActions) => [actions.loadPyodide, actions.setSource],
        async (actions, target, { getState }) => {
            // Wait a maximum of 2 minutes if we are still loading an engine
            let timeStart = Date.now();
            while (
                (getState().status === "loadingPyodide" ||
                    getState().status === "loadingWasm") &&
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
