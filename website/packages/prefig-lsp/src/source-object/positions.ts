import type { Range } from "vscode-languageserver";

/**
 * Offset <-> LSP position math over a single document string.
 *
 * This is the seed of the richer `source-object` module the plan calls for
 * (element-at-offset, tag/attr ranges for completion in Phase 4). For Phase 0
 * we only need to turn character offsets — the units `@lezer/xml` and `saxes`
 * both report in — into LSP {line, character} positions.
 *
 * `character` is a UTF-16 code-unit count, matching how JavaScript string
 * indices (and therefore lezer/saxes offsets) already work, so no re-encoding
 * is required.
 */
export class LineMap {
    private readonly lineStarts: number[];

    constructor(private readonly text: string) {
        this.lineStarts = [0];
        for (let i = 0; i < text.length; i++) {
            if (text.charCodeAt(i) === 10 /* \n */) {
                this.lineStarts.push(i + 1);
            }
        }
    }

    positionAt(offset: number): { line: number; character: number } {
        const clamped = Math.max(0, Math.min(offset, this.text.length));
        let low = 0;
        let high = this.lineStarts.length - 1;
        while (low < high) {
            const mid = (low + high + 1) >> 1;
            if (this.lineStarts[mid] <= clamped) {
                low = mid;
            } else {
                high = mid - 1;
            }
        }
        return { line: low, character: clamped - this.lineStarts[low] };
    }

    rangeFromOffsets(from: number, to: number): Range {
        return { start: this.positionAt(from), end: this.positionAt(to) };
    }
}
