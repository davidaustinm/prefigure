import { PyodideInterface, loadPyodide } from "pyodide";
import { prefigBrowserApi } from "./compat-api";

type Options = Parameters<typeof loadPyodide>[0];

/**
 * A class for compiling a PreFigure document file using a WASM implementation of python.
 */
export class PreFigureCompiler {
    pyodide: PyodideInterface | null = null;
    _pyodide: ReturnType<typeof loadPyodide> | null = null;
    pyodideInitPromise: Promise<void> | null = null;

    /**
     * @param pyodidePromise Optionally pass in an instance of `loadPyodide` to use a custom configuration.
     */
    constructor(pyodidePromise?: ReturnType<typeof loadPyodide>) {
        if (pyodidePromise) {
            this._pyodide = pyodidePromise;
        }
    }

    /**
     * Initialize the compiler. This is safe to call multiple times.
     */
    async init(options: Options = {}) {
        // Wait for any other initialization to finish
        await this.pyodideInitPromise;
        // Don't accidentally initialize a second time!
        if (this.pyodide) {
            return;
        }

        this.pyodideInitPromise = new Promise(async (resolve, reject) => {
            try {
                // Prefer `._pyodide` over creating a new pyodide instance
                // since `._pyodide` was provided by the user.
                this.pyodide =
                    (await this._pyodide) || (await loadPyodide(options));

                // There may be some MathJax etc. setup that needs to be done
                await prefigBrowserApi.initFinished;

                // Set up our global compatibility API so it can be imported from Python with `import prefigBrowserApi`
                this.pyodide.registerJsModule(
                    "prefigBrowserApi",
                    prefigBrowserApi,
                );

                // We want to make sure to load the prefig package from the same location that we are loading all
                // the other packages from. This is accessing the internal `._api` from pyodide and might break in the
                // future.
                const PREFIG_PATH =
                    ((this.pyodide as any)._api.config.indexURL as string) +
                    "prefig-0.4.4-py3-none-any.whl";

                // Load all the dependencies
                await this.pyodide.loadPackage([
                    "micropip",
                    "packaging",
                    "lxml",
                    "numpy",
                    "scipy",
                    "shapely",
                    "click",
                    "networkx",
                    PREFIG_PATH,
                ]);
            } catch (e) {
                reject(e);
            }

            resolve();
        });

        await this.pyodideInitPromise;
    }

    _checkInit(): asserts this is { pyodide: PyodideInterface } {
        if (!this.pyodide) {
            throw new Error("Compiler not initialized");
        }
    }

    /**
     * Compile the given PreFigure source and return the SVG string
     */
    async compile(
        mode: "svg" | "tactile",
        source: string,
    ): Promise<{ svg: string; annotations: unknown }> {
        this._checkInit();

        const result = await this.pyodide.runPython(`
import prefig
prefig.engine.build_from_string("${mode}", ${JSON.stringify(source)})
        `);
        const [svg, annotations] = result;
        return { svg, annotations };
    }
}
