import { describe, expect, test } from "vitest";
import { validateSyntax } from "../src/features/validate-syntax";

describe("validateSyntax", () => {
    test("well-formed XML produces no diagnostics", () => {
        const source = `<diagram dimensions="(300,300)"><point at="p"/></diagram>`;
        expect(validateSyntax(source)).toEqual([]);
    });

    test("multi-line well-formed XML produces no diagnostics", () => {
        const source = [
            `<diagram dimensions="(300,300)" margins="5">`,
            `  <coordinates bbox="(-4,-4,4,4)">`,
            `    <point at="p"><m>(1,2)</m></point>`,
            `  </coordinates>`,
            `</diagram>`,
        ].join("\n");
        expect(validateSyntax(source)).toEqual([]);
    });

    // The bug from the field: a <label> whose inner <m> never gets closed.
    // It reached the compiler and surfaced only as a console traceback; here
    // it must become a diagnostic anchored on the offending <m> open tag.
    test("missing </m> is anchored on the <m> open tag", () => {
        const source = `<label alignment="ne"><m>(x2,y2)</label>`;
        const diagnostics = validateSyntax(source);
        expect(diagnostics).toHaveLength(1);
        expect(diagnostics[0].message).toBe("Missing closing tag for <m>.");
        // `<m>` occupies characters 22..25 on line 0.
        expect(diagnostics[0].range).toEqual({
            start: { line: 0, character: 22 },
            end: { line: 0, character: 25 },
        });
    });

    test("missing close tag reports the correct line in a multi-line doc", () => {
        const source = [
            `<diagram>`,
            `  <label><m>x</label>`,
            `</diagram>`,
        ].join("\n");
        const diagnostics = validateSyntax(source);
        expect(diagnostics).toHaveLength(1);
        expect(diagnostics[0].message).toBe("Missing closing tag for <m>.");
        // `<m>` is on line 1 (0-based), starting at character 9.
        expect(diagnostics[0].range.start).toEqual({ line: 1, character: 9 });
    });

    // From the field: deleting the `>` of a closing tag. lezer swallows the
    // tag up to the next `<`, leaving its only raw error node at the *start of
    // the following tag* — so the squiggle used to land on `</annotations>`
    // rather than the `</annotation` actually missing its `>`.
    test("close tag missing its > is anchored on that close tag", () => {
        const source = [
            `<annotations>`,
            `  <annotation ref="figure">`,
            `    <annotation ref="graph"/>`,
            `  </annotation`,
            `</annotations>`,
        ].join("\n");
        const diagnostics = validateSyntax(source);
        expect(diagnostics).toHaveLength(1);
        expect(diagnostics[0].message).toBe(
            'Missing ">" to close the </annotation> tag.',
        );
        // `</annotation` occupies characters 2..14 on line 3, and the squiggle
        // must not bleed onto the next line or onto `</annotations>`.
        expect(diagnostics[0].range).toEqual({
            start: { line: 3, character: 2 },
            end: { line: 3, character: 14 },
        });
    });

    test("open tag missing its > is anchored on that open tag", () => {
        const diagnostics = validateSyntax(`<diagram><point at="p"`);
        const missingGt = diagnostics.find((d) =>
            d.message.startsWith('Missing ">"'),
        );
        expect(missingGt?.message).toBe(
            'Missing ">" to close the <point> tag.',
        );
        // `<point ...` starts at character 9 on line 0.
        expect(missingGt?.range.start).toEqual({ line: 0, character: 9 });
    });

    // A tag missing its `>` should report exactly once — the precise "missing
    // >" diagnostic — not also the redundant "unclosed element" that lezer
    // emits as a side effect for the same tag.
    test("open tag missing its > reports once, not also 'missing closing tag'", () => {
        const source = [`<diagram>`, `  <point at="p"`, `</diagram>`].join(
            "\n",
        );
        const diagnostics = validateSyntax(source);
        expect(diagnostics).toHaveLength(1);
        expect(diagnostics[0].message).toBe(
            'Missing ">" to close the <point> tag.',
        );
        expect(diagnostics[0].range.start).toEqual({ line: 1, character: 2 });
    });

    test("mismatched closing tag is flagged", () => {
        const diagnostics = validateSyntax(`<a><b></a></b>`);
        const messages = diagnostics.map((d) => d.message);
        expect(messages).toContain("Mismatched closing tag </b>.");
    });

    test("duplicate attribute is flagged on the repeated name", () => {
        const source = `<point x="1" x="2"/>`;
        const diagnostics = validateSyntax(source);
        expect(diagnostics).toHaveLength(1);
        expect(diagnostics[0].message).toBe('Duplicate attribute "x".');
        // The second `x` starts at character 13.
        expect(diagnostics[0].range.start).toEqual({ line: 0, character: 13 });
    });

    test("stray ampersand is flagged", () => {
        const diagnostics = validateSyntax(`<diagram>a & b</diagram>`);
        expect(diagnostics.length).toBeGreaterThanOrEqual(1);
        expect(diagnostics[0].range.start.character).toBe(11);
    });

    test("unclosed root element is flagged on its open tag", () => {
        const diagnostics = validateSyntax(`<diagram><point/>`);
        expect(diagnostics).toHaveLength(1);
        expect(diagnostics[0].message).toBe("Unclosed tag <diagram>.");
        expect(diagnostics[0].range).toEqual({
            start: { line: 0, character: 0 },
            end: { line: 0, character: 9 },
        });
    });

    test("every diagnostic is an error from the prefigure source", () => {
        for (const d of validateSyntax(`<a><b></a></b>`)) {
            expect(d.severity).toBe(1 /* Error */);
            expect(d.source).toBe("prefigure");
        }
    });
});
