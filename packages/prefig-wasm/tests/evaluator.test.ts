// Node tests for the WebAssembly build.
// Run with `npm test` (builds the wasm first) or `npm run test:only`.

import { describe, it, expect } from "vitest";
import {
    version,
    Evaluator,
    build_from_string,
    set_host_api,
} from "../pkg/prefig_wasm.js";
import type { BuildResult, HostApi } from "./wasm-types.js";

// A minimal stand-in for the playground's PrefigBrowserApi. Real MathJax/SRE
// aren't available in this test, so math labels get fixed-size placeholders.
const mockHostApi: HostApi = {
    measure_text: (text, _font) => [text.length * 8, 10, 3],
    translate_text: (text, _typeform) => text,
    processMath: (_tex) =>
        `<svg xmlns="http://www.w3.org/2000/svg" width="1ex" height="1ex" style="vertical-align: 0ex"><defs/></svg>`,
    processBraille: (_tex) => "⠠",
};

describe("version", () => {
    it("reports the crate version", () => {
        expect(version()).toMatch(/^\d+\.\d+\.\d+$/);
    });
});

describe("Evaluator", () => {
    it("evaluates arithmetic", () => {
        const ev = new Evaluator();
        expect(ev.evaluate("3 + 4 * 2")).toBe(11);
        expect(ev.evaluate("2^5")).toBe(32); // ^ means exponent
        expect(ev.evaluate("-7 // 2")).toBe(-4); // Python-style floor division
    });

    it("evaluates vectors as arrays", () => {
        const ev = new Evaluator();
        expect(ev.evaluate("(1, 2) + (3, 4)")).toEqual([4, 6]);
        expect(ev.evaluate("midpoint((0,0), (2,4))")).toEqual([1, 2]);
    });

    it("remembers definitions", () => {
        const ev = new Evaluator();
        ev.define("a = 5");
        ev.define("f(x) = x^2 + a");
        expect(ev.evaluate("f(3)")).toBe(14);
    });

    it("keeps definitions separate between instances", () => {
        const first = new Evaluator();
        first.define("a = 5");
        const second = new Evaluator();
        expect(() => second.evaluate("a")).toThrow(/Unrecognized name/);
    });

    it("returns dictionaries as objects and strings as strings", () => {
        const ev = new Evaluator();
        expect(ev.evaluate("{'color': 'red', 'width': 2}")).toEqual({
            color: "red",
            width: 2,
        });
        expect(ev.evaluate("#ff0000")).toBe("#ff0000");
        expect(ev.evaluate("rgb(255, 0, 0)")).toBe("rgb(255,0,0)");
    });

    it("rejects expressions the Python version rejects", () => {
        const ev = new Evaluator();
        expect(() => ev.evaluate("x < 3")).toThrow();
        expect(() => ev.evaluate("__import__('os')")).toThrow();
    });
});

describe("build_from_string", () => {
    it("builds a simple diagram to SVG", () => {
        set_host_api(mockHostApi);
        const source = `
            <diagram dimensions="(200,200)" margins="5">
              <coordinates bbox="[-4,-4,4,4]">
                <grid-axes/>
                <circle center="(0,0)" radius="2" stroke="blue"/>
              </coordinates>
            </diagram>`;
        const { svg, annotations } = build_from_string("svg", source) as BuildResult;
        expect(svg).toMatch(/^<svg/);
        expect(svg).toContain("width=");
        expect(svg).toContain("path"); // the circle and grid render as paths
        // The wasm build runs in the "pyodide" environment, where a diagram with
        // no explicit <annotations> still gets default speech annotations
        // (Python's Diagram.annotate_source -> annotations.diagram_to_speech).
        expect(annotations).toMatch(/^<diagram><annotations>/);
        expect(annotations).toContain("A circle element"); // per-element speech
    });

    it("reports an error for source with no diagram", () => {
        set_host_api(mockHostApi);
        expect(() => build_from_string("svg", "<nope/>")).toThrow(/diagram/);
    });
});
