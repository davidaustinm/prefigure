import { PyodideInterface, loadPyodide } from "pyodide";

type Options = Parameters<typeof loadPyodide>[0];
const DECODER = new TextDecoder();

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

                // Load all the dependencies
                await this.pyodide.loadPackage([
                    "micropip",
                    "packaging",
                    "lxml",
                    "numpy",
                    "scipy",
                    "shapely",
                    "click",
                    "./assets/pyodide/prefig-0.2.11-py3-none-any.whl",
                ]);

                //await this.pyodide.unpackArchive(rawZip, "zip");
                //await this.pyodide.unpackArchive(rawMicropip, "zip");
                //await this.pyodide.unpackArchive(rawPackaging, "zip");
                //await this.pyodide.unpackArchive(rawLxml, "zip");
                //this.pyodide.FS.mkdir("./tmp_compile");
                //this.pyodide.FS.mkdir("./tmp_compile/generated-assets");
                //this.pyodide.FS.mkdir("./tmp_compile/assets");
                //this.pyodide.FS.mkdir("./tmp_compile/out");
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

    ///**
    // * Set `main.ptx` to the given string. If no string is provided, a default "hello world" template is used.
    // */
    //setMainPtx(contents: string = mainPtxDefault) {
    //    this._checkInit();
    //    this.pyodide.FS.writeFile(MAIN_PTX_PATH, contents);
    //}

    ///**
    // * Get the contents of `main.ptx`.
    // */
    //getMainPtx() {
    //    this._checkInit();
    //    return this.pyodide.FS.readFile(MAIN_PTX_PATH);
    //}

    ///**
    // * Set `publication.ptx` to the given string. If no string is provided, a default template is used.
    // */
    //setPublicationPtx(contents: string = publicationPtxDefault) {
    //    this._checkInit();
    //    this.pyodide.FS.writeFile(PUBLICATION_PTX_PATH, contents);
    //}

    ///**
    // * Get the contents of `publication.ptx`.
    // */
    //getPublicationPtx() {
    //    this._checkInit();
    //    return this.pyodide.FS.readFile(PUBLICATION_PTX_PATH);
    //}

    ///**
    // * Compile the PreTeXt file.
    // */
    //async compile() {
    //    this._checkInit();
    //    // Check that `main.ptx` and `publication.ptx` exist.
    //    if (!this.pyodide.FS.findObject("/home/pyodide/tmp_compile/main.ptx")) {
    //        this.setMainPtx();
    //    }
    //    if (
    //        !this.pyodide.FS.findObject(
    //            "/home/pyodide/tmp_compile/publication.ptx",
    //        )
    //    ) {
    //        this.setPublicationPtx();
    //    }
    //    await this.pyodide.runPythonAsync(`
    //        import pretext_wasm
    //        pretext_wasm.compile("${MAIN_PTX_PATH}", "${PUBLICATION_PTX_PATH}")
    //    `);
    //}

    ///**
    // * Get the compiled HTML.
    // */
    //getHtml() {
    //    this._checkInit();
    //    // We need to find the "root" file. There is always an `index.html` with a
    //    //    `<meta http-equiv="refresh" content="0; URL='hello-world.html'">`
    //    // style redirect element. Find the redirect URL and return the corresponding HTML.
    //    const indexHtml = DECODER.decode(
    //        this.pyodide.FS.readFile(`${OUT_DIR}/index.html`),
    //    );
    //    const redirectMatch = indexHtml.match(/URL='([^']+)'/);
    //    if (redirectMatch) {
    //        const redirectUrl = redirectMatch[1];
    //        return DECODER.decode(
    //            this.pyodide.FS.readFile(`${OUT_DIR}/${redirectUrl}`),
    //        );
    //    }
    //    // Try actual DOM parsing

    //    try {
    //        const parser = new DOMParser();
    //        const doc = parser.parseFromString(indexHtml, "text/html");
    //        const redirectElm = doc.querySelector("meta[http-equiv=refresh]");
    //        if (!redirectElm) {
    //            throw new Error("No redirect element found");
    //        }
    //        const redirectUrl =
    //            redirectElm.getAttribute("URL") ||
    //            redirectElm.getAttribute("url");
    //        if (redirectUrl) {
    //            return DECODER.decode(
    //                this.pyodide.FS.readFile(`${OUT_DIR}/${redirectUrl}`),
    //            );
    //        }
    //    } catch {}

    //    // Last guess is to return the root-1-1.html file
    //    return DECODER.decode(
    //        this.pyodide.FS.readFile(
    //            "/home/pyodide/tmp_compile/out/root-1-1.html",
    //        ),
    //    );
    //}

    ///**
    // * Returns the HTML but references to local style sheets/javascript are replaced by blob URLs
    // * containing the contents of the requested files.
    // */
    //getHtmlWithLocalReferences() {
    //    this._checkInit();
    //    const rawHtml = this.getHtml();
    //    // If we are run in a WebWorker, we might not have access to the DOMParser. We still want to return something
    //    // sensible.
    //    try {
    //        const parser = new DOMParser();
    //        const doc = parser.parseFromString(rawHtml, "text/html");
    //        // Loop through all the CSS tags and replace the href with a blob URL if applicable
    //        for (const link of doc.querySelectorAll("link[rel=stylesheet]")) {
    //            const href = link.getAttribute("href");
    //            const path = `${OUT_DIR}/${href}`;
    //            if (!this.pyodide.FS.findObject(path)) {
    //                continue;
    //            }
    //            const cssFile = DECODER.decode(this.pyodide.FS.readFile(path));
    //            const blob = new Blob([cssFile], { type: "text/css" });
    //            const blobUrl = URL.createObjectURL(blob);
    //            link.setAttribute("href", blobUrl);
    //        }

    //        // Loop through all the script tags and replace the src with a blob URL if applicable
    //        for (const script of doc.querySelectorAll("script")) {
    //            const src = script.getAttribute("src");
    //            const path = `${OUT_DIR}/${src}`;
    //            if (!this.pyodide.FS.findObject(path)) {
    //                continue;
    //            }
    //            const jsFile = DECODER.decode(this.pyodide.FS.readFile(path));
    //            const blob = new Blob([jsFile], { type: "text/javascript" });
    //            const blobUrl = URL.createObjectURL(blob);
    //            script.setAttribute("src", blobUrl);
    //        }

    //        return doc.documentElement.outerHTML;
    //    } catch (e) {
    //        console.error(e);
    //        return rawHtml;
    //    }
    //}
}
