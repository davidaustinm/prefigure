// Runs the WebAssembly build across the whole shared example corpus
// (`packages/tests/examples/**/*.xml` -- the same sources the Python suite and
// the native Rust tests use), for BOTH build variants:
//   - `pkg`        -- the default build; math is rendered by the host via the
//                     `processMath` callback (MathJax in the playground).
//   - `pkg-native` -- the `--features ratex` build; math is rendered by the
//                     pure-Rust RaTeX engine inside the wasm module, so the host
//                     provides no math at all.
// A variant whose pkg dir isn't built is skipped (so `npm run test:only` after a
// single `build:mathjax` still runs). `npm test` builds both first.
//
// This is the wasm-boundary analogue of the native
// `examples_build_but_output_not_checked.rs` + `tactile.rs` tests: it guards the
// binding against crashes and preserves the tactile-layout invariants across
// every snapshot, complementing the native numeric-parity suite
// (`expected_svgs.rs`) rather than duplicating it.
//
// Byte/numeric parity against the committed Python SVG snapshots is NOT asserted
// here, and can't honestly be: those snapshots use Python's real label backends
// (node/MathJax `mj_sre` + pycairo + liblouis) in the `pretext` environment,
// whereas these builds render labels through a host object / RaTeX in the
// `pyodide` environment. The label subsystems genuinely differ -- the same
// reason the native stub-based tests (annotations/tactile) don't do SVG parity.
// RaTeX in particular is KaTeX-styled, so the two variants don't match each
// other either; only structural invariants are shared.

import { describe, it, expect } from "vitest";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";
import { dirname, join } from "node:path";
import type { BuildResult, HostApi } from "./wasm-types.js";

const HERE = dirname(fileURLToPath(import.meta.url));
const EXAMPLES_DIR = join(HERE, "..", "..", "tests", "examples");

/** The subset of the generated pkg API the corpus test drives. */
interface WasmModule {
    build_from_string(format: string, source: string): unknown;
    set_host_api(api: unknown): void;
}

/** The two build variants produced by package.json's build:mathjax/build:native. */
const VARIANTS = [
    { name: "mathjax (default, host-rendered math)", dir: "pkg" },
    { name: "native (ratex, in-module math)", dir: "pkg-native" },
];

// A deterministic stand-in for the playground's PrefigBrowserApi that mirrors
// the native tests' stub label services (packages/prefig-rust/prefig-core/
// tests/common/mod.rs), so this test needs no MathJax/cairo/liblouis and is
// fully portable: a fixed-size math placeholder, char-count text metrics, and a
// single Braille glyph per character. The native (ratex) build renders math
// in-module, so it ignores processMath/processBraille and uses only the text and
// braille callbacks.
const STUB_MATH_SVG =
    '<svg xmlns="http://www.w3.org/2000/svg" width="1.5ex" height="1.5ex" ' +
    'viewBox="0 -1 1.5 1.5" style="vertical-align: -0.25ex"><defs></defs><g></g></svg>';

const stubHostApi: HostApi = {
    measure_text: (text, fontString) => {
        // fontString is `[italic ][bold ]{size}px {family}`; recover the size to
        // match the native StubText metrics: [n*size*0.5, size*0.75, size*0.25].
        const size = Number.parseFloat(/(\d+(?:\.\d+)?)px/.exec(fontString)?.[1] ?? "12");
        const n = [...text].length;
        return [n * size * 0.5, size * 0.75, size * 0.25];
    },
    translate_text: (text, _typeform) => [...text].map(() => "⠿").join(""),
    processMath: (_tex) => STUB_MATH_SVG,
    processBraille: (tex) => [...tex].map(() => "⠿").join(""),
};

/** Every `*.xml` under `dir`, recursively, as absolute paths (sorted). */
function collectXml(dir: string): string[] {
    const out: string[] = [];
    for (const name of readdirSync(dir)) {
        const path = join(dir, name);
        if (statSync(path).isDirectory()) {
            out.push(...collectXml(path));
        } else if (name.endsWith(".xml")) {
            out.push(path);
        }
    }
    return out.sort();
}

/** [width, height] of the root `<svg>` element, from its opening tag. */
function rootDimensions(svg: string): [string | undefined, string | undefined] {
    const open = svg.slice(0, svg.indexOf(">"));
    const attr = (name: string) => new RegExp(`${name}="([^"]*)"`).exec(open)?.[1];
    return [attr("width"), attr("height")];
}

/** A human-readable message for a thrown value of unknown type. */
function errMessage(e: unknown): string {
    return e instanceof Error ? e.message : String(e);
}

/** Load a built variant's wasm module, or null if that pkg dir isn't built. */
async function loadVariant(dir: string): Promise<WasmModule | null> {
    const entry = join(HERE, "..", dir, "prefig_wasm.js");
    if (!existsSync(entry)) return null;
    return (await import(pathToFileURL(entry).href)) as unknown as WasmModule;
}

const figures = collectXml(EXAMPLES_DIR);

for (const variant of VARIANTS) {
    const wasm = await loadVariant(variant.dir);
    const suite = wasm ? describe : describe.skip;

    suite(`wasm corpus [${variant.name}]`, () => {
        if (wasm) wasm.set_host_api(stubHostApi);

        it("finds the shared example corpus", () => {
            expect(figures.length).toBeGreaterThanOrEqual(160);
        });

        // One accumulating pass, then assertions. Building each source twice
        // (svg + tactile) through the wasm module; a thrown error is a graceful
        // build failure (e.g. a fragment meant to be embedded in a PreTeXt
        // wrapper, or a <read>/<image> source whose data file the wasm module
        // can't reach without a host filesystem) -- allowed, but never a crash we
        // can't describe.
        const built: string[] = [];
        const gracefulErrors: string[] = [];
        const wrongPage: string[] = [];
        const identical: string[] = [];

        for (const path of figures) {
            if (!wasm) break;
            const name = path.slice(EXAMPLES_DIR.length + 1);
            const source = readFileSync(path, "utf8");

            let svg: string | undefined;
            let tactile: string | undefined;
            try {
                svg = (wasm.build_from_string("svg", source) as BuildResult).svg;
            } catch (e) {
                gracefulErrors.push(`${name} [svg]: ${errMessage(e)}`);
            }
            try {
                tactile = (wasm.build_from_string("tactile", source) as BuildResult).svg;
            } catch (e) {
                gracefulErrors.push(`${name} [tactile]: ${errMessage(e)}`);
            }

            if (svg === undefined || tactile === undefined) continue;
            built.push(name);

            // Invariant 1: every SVG build produces a well-formed <svg> root.
            if (!svg.startsWith("<svg")) wrongPage.push(`${name}: svg root not <svg>`);

            // Invariant 2: tactile is laid out on the fixed 828x792 emboss page,
            // regardless of the source's own dimensions.
            const [w, h] = rootDimensions(tactile);
            if (w !== "828" || h !== "792") {
                wrongPage.push(`${name}: tactile page ${w}x${h}, expected 828x792`);
            }

            // Invariant 3: tactile output differs from the SVG build.
            if (tactile === svg) identical.push(name);
        }

        it("builds most of the corpus as both svg and tactile", () => {
            // eslint-disable-next-line no-console
            console.log(
                `wasm [${variant.dir}]: ${figures.length} files => ${built.length} ` +
                    `built both ways, ${gracefulErrors.length} graceful build errors`,
            );
            expect(built.length).toBeGreaterThanOrEqual(100);
        });

        it("lays every tactile build on the 828x792 emboss page with a well-formed svg root", () => {
            expect(wrongPage).toEqual([]);
        });

        it("produces tactile output distinct from the svg build", () => {
            expect(identical).toEqual([]);
        });
    });
}

// Cross-variant: the two backends must genuinely differ on math. A diagram whose
// only content is a math label renders as a fixed placeholder in the MathJax
// build (the host `processMath` stub) but as real KaTeX glyph geometry in the
// native RaTeX build, so the native output carries strictly more content.
const MATH_DIAGRAM = `
<diagram dimensions="(200,200)" margins="5">
  <coordinates bbox="(-3,-3,3,3)">
    <label p="(0,0)" alignment="east"><m>\\frac{-b \\pm \\sqrt{b^2-4ac}}{2a}</m></label>
  </coordinates>
</diagram>`;

const mathjax = await loadVariant("pkg");
const native = await loadVariant("pkg-native");
const bothBuilt = mathjax && native;

(bothBuilt ? describe : describe.skip)("mathjax vs native math rendering", () => {
    it("renders the same math label differently (RaTeX glyphs vs host placeholder)", () => {
        mathjax!.set_host_api(stubHostApi);
        native!.set_host_api(stubHostApi);
        const withMathjax = (mathjax!.build_from_string("svg", MATH_DIAGRAM) as BuildResult).svg;
        const withNative = (native!.build_from_string("svg", MATH_DIAGRAM) as BuildResult).svg;

        expect(withMathjax).not.toEqual(withNative);
        // RaTeX emits embedded glyph outlines, so the native build carries more
        // geometry than the host-stub placeholder.
        expect(withNative.length).toBeGreaterThan(withMathjax.length);
        expect(withNative).toContain("<path");
    });
});
