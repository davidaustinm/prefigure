import { action, Action, Thunk, thunk } from "easy-peasy";
import { PreFigureCompiler } from "./compiler";

const compiler = new PreFigureCompiler();
(window as any).comp = compiler;

export interface PlaygroundModel {
    source: string;
    compiledSource: string;
    status: "" | "loadingPyodide";
    prefigVersion: string;
    setPrefigVersion: Action<PlaygroundModel, string>;
    errorState: string;
    setSource: Action<PlaygroundModel, string>;
    setCompiledSource: Action<PlaygroundModel, string>;
    setStatus: Action<PlaygroundModel, "" | "loadingPyodide">;
    setErrorState: Action<PlaygroundModel, string>;
    loadPyodide: Thunk<PlaygroundModel>;
}

export const playgroundModel: PlaygroundModel = {
    source: `<diagram dimensions="(300,300)" margins="5">
  <definition> f(x) = exp(x/3)*cos(x) </definition>
  <definition> a = 1 </definition>
  <coordinates bbox="(-4,-4,4,4)">
    <grid-axes xlabel="x" ylabel="y"/>    
    <graph function="f"/>
    <tangent-line function="f" point="a"/>
    <point p="(a,f(a))">
      <m>(a,f(a))</m>
    </point>
  </coordinates>
</diagram>`,
    compiledSource: "",
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
    loadPyodide: thunk(async (actions, _, helpers) => {
        actions.setStatus("loadingPyodide");
        // Initialize Pyodide
        await compiler.init({
            indexURL: "./assets/pyodide",
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
};
