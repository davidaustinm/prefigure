import { prefigBrowserApi } from "./compat-api";

/**
 * The minimal surface both wasm builds expose (see
 * `packages/prefig-wasm/src/lib.rs`). `default` is the wasm-pack `--target web`
 * initializer that fetches and instantiates the `.wasm` file.
 */
interface PrefigWasmModule {
    default: (module_or_path?: unknown) => Promise<unknown>;
    set_host_api: (api: unknown) => void;
    version: () => string;
    build_from_string: (
        format: string,
        source: string,
    ) => { svg: string; annotations: string | null };
}

/**
 * Compiles a PreFigure document using the Rust port compiled to WebAssembly,
 * a drop-in alternative to `PreFigureCompiler` (which uses Python via Pyodide).
 *
 * There are two wasm flavors, built from the one `@prefigure/prefig-wasm`
 * crate (see `packages/prefig-wasm`):
 *
 *  - **mathjax** (`pkg-web`, the default build): math is rendered by the host's
 *    MathJax via the `processMath` callback on `prefigBrowserApi`.
 *  - **ratex** (`pkg-web-native`, the `ratex` cargo feature): math is rendered
 *    by the pure-Rust RaTeX engine embedded in the wasm module — no host math
 *    callback needed.
 *
 * Both still delegate braille and text measurement to `prefigBrowserApi`, so
 * `set_host_api` is called for either flavor. The concrete module is supplied
 * as a loader so a single class serves both builds; the loader's dynamic
 * import lets Vite emit each `.wasm` as its own asset and load it on demand.
 */
export class PreFigureWasmCompiler {
    private wasm: PrefigWasmModule | null = null;
    private initPromise: Promise<void> | null = null;

    constructor(private loadModule: () => Promise<PrefigWasmModule>) {}

    /** Safe to call multiple times; initialization happens at most once. */
    async init(): Promise<void> {
        if (this.wasm) {
            return;
        }
        if (this.initPromise) {
            return this.initPromise;
        }
        this.initPromise = (async () => {
            const mod = await this.loadModule();
            // `--target web` builds export a default init() that loads the .wasm
            await mod.default();
            // MathJax / speech-rule-engine finish loading asynchronously
            await prefigBrowserApi.initFinished;
            mod.set_host_api(prefigBrowserApi);
            this.wasm = mod;
        })();
        return this.initPromise;
    }

    /** The version of the Rust prefig package that is loaded. */
    version(): string {
        if (!this.wasm) {
            throw new Error("Compiler not initialized");
        }
        return this.wasm.version();
    }

    /** Compile PreFigure source, returning the SVG and any annotations. */
    async compile(
        mode: "svg" | "tactile",
        source: string,
    ): Promise<{ svg: string; annotations: string }> {
        if (!this.wasm) {
            throw new Error("Compiler not initialized");
        }
        const result = this.wasm.build_from_string(mode, source);
        return { svg: result.svg, annotations: result.annotations ?? "" };
    }
}

/** Loads the MathJax-backed wasm build (`pkg-web`). */
export function loadMathjaxWasm(): Promise<PrefigWasmModule> {
    return import("../../../prefig-wasm/pkg-web/prefig_wasm.js") as Promise<PrefigWasmModule>;
}

/** Loads the RaTeX-backed wasm build (`pkg-web-native`). */
export function loadRatexWasm(): Promise<PrefigWasmModule> {
    return import("../../../prefig-wasm/pkg-web-native/prefig_wasm.js") as Promise<PrefigWasmModule>;
}
