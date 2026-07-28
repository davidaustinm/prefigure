// Shared types for the wasm test suites. The generated pkg types
// (`pkg/prefig_wasm.d.ts`) declare `build_from_string` as returning `any` and
// `set_host_api` as taking `any`; these give the tests concrete shapes to work
// against.

/** The host object PreFigure's wasm build calls for labels (a stand-in for the
 * playground's `PrefigBrowserApi`; see packages/playground/src/worker/compat-api.ts). */
export interface HostApi {
    /** [width, ascent, descent] of typeset `text` in the CSS `fontString`. */
    measure_text(text: string, fontString: string): [number, number, number];
    /** `text` translated to a Braille string. */
    translate_text(text: string, typeform: number[]): string;
    /** TeX `expression` rendered to an SVG (MathJax `<mjx-container>`/`<svg>`). */
    processMath(expression: string): string;
    /** TeX `expression` translated to Braille (via MathML). */
    processBraille(expression: string): string;
}

/** The `{ svg, annotations }` object `build_from_string` returns. */
export interface BuildResult {
    svg: string;
    annotations: string | null;
}
