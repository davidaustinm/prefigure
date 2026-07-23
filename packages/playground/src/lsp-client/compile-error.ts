import { DiagnosticSeverity } from "vscode-languageserver-protocol";
import type { Diagnostic } from "vscode-languageserver-protocol";

/**
 * Pick out the one line worth showing from a raw error string.
 *
 * Compiling runs Python inside Pyodide, so a failure often arrives as a full
 * Python traceback (itself wrapped by Pyodide's own call stack). The last line
 * of that traceback is the actual `ExceptionType: message`; the rest is
 * interpreter plumbing. Anything that isn't a traceback (a plain JS Error) is
 * already short, so it is returned as-is.
 */
export function summarizeError(raw: string): string {
    const trimmed = raw.trim();
    if (!trimmed.includes("Traceback (most recent call last):")) {
        return trimmed;
    }
    const lines = trimmed
        .split("\n")
        .map((line) => line.trim())
        .filter((line) => line.length > 0);
    return lines[lines.length - 1] || trimmed;
}

/**
 * Translate a failed compile into LSP diagnostics for the *additional*
 * diagnostics channel (`prefigure/setAdditionalDiagnostics`).
 *
 * Two deliberate choices:
 *
 *   - Well-formedness failures are dropped. The native syntax pass already
 *     squiggles those precisely (on the exact offending tag); re-reporting the
 *     compiler's line-level version would just double up.
 *
 *   - Everything else (expression failures, runtime errors) generally carries
 *     no usable source position, so it lands as a document-level diagnostic.
 *     If the message happens to embed an `lxml`-style `line N` / `column M`,
 *     we honour it. Either way the error banner still shows the full text.
 */
export function compileErrorToDiagnostics(error: string, text: string): Diagnostic[] {
    const trimmed = error.trim();
    if (!trimmed) {
        return [];
    }
    // Well-formedness failures belong to the native syntax channel, which
    // squiggles them precisely and re-checks on every keystroke. The compile
    // channel only refreshes on compile, so a well-formedness error left here
    // both double-reports and goes stale the moment the user fixes the tag.
    // Drop them however they are phrased: our engine's wrapper ("... not
    // well-formed XML: ..."), or — when an older published wheel skips that
    // wrapper — lxml's raw `XMLSyntaxError` (e.g. "expected '>'").
    if (/not well-formed XML|XMLSyntaxError/i.test(trimmed)) {
        return [];
    }

    // Match position off the summarized exception line, not the raw traceback:
    // interpreter frames ("File ..., line 128, in build") would otherwise win
    // the `line N` match and anchor the squiggle to some library's line number.
    const summary = summarizeError(error);
    const documentLines = text.split("\n");
    const lineMatch = summary.match(/line (\d+)/i);
    const columnMatch = summary.match(/column (\d+)/i);
    const line = lineMatch
        ? clamp(Number.parseInt(lineMatch[1], 10) - 1, 0, Math.max(0, documentLines.length - 1))
        : 0;
    const lineText = documentLines[line] ?? "";
    const indent = lineText.length - lineText.trimStart().length;
    const startCharacter = columnMatch
        ? Math.max(0, Number.parseInt(columnMatch[1], 10) - 1)
        : indent;
    const endCharacter = Math.max(startCharacter + 1, lineText.length);

    return [
        {
            range: {
                start: { line, character: startCharacter },
                end: { line, character: endCharacter },
            },
            severity: DiagnosticSeverity.Error,
            source: "prefig-compile",
            message: summary,
        },
    ];
}

function clamp(value: number, low: number, high: number): number {
    return Math.min(Math.max(value, low), high);
}
