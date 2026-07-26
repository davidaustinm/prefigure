import { Diagnostic } from "vscode-languageserver";

/**
 * Per-document store for the *second* diagnostic channel: diagnostics produced
 * outside the server (a real prefig compile in the playground's Pyodide
 * worker, or a `prefig-wasm` sub-worker in VS Code later) and pushed in via
 * the `prefigure/setAdditionalDiagnostics` method. The server merges these
 * with its own XML-syntax diagnostics and re-publishes.
 *
 * Keyed by document URI so a stale compile result for one document can never
 * bleed into another.
 *
 * A store is created per `createServer` (see `server.ts`) rather than living
 * as module state, so two servers sharing a JS realm — most concretely the
 * in-process test harness — never share this map.
 */
export interface AdditionalDiagnosticsStore {
    set(uri: string, diagnostics: Diagnostic[]): void;
    get(uri: string): Diagnostic[];
    clear(uri: string): void;
}

export function createAdditionalDiagnosticsStore(): AdditionalDiagnosticsStore {
    const additionalDiagnostics = new Map<string, Diagnostic[]>();
    return {
        set(uri, diagnostics) {
            additionalDiagnostics.set(uri, diagnostics);
        },
        get(uri) {
            return additionalDiagnostics.get(uri) ?? [];
        },
        clear(uri) {
            additionalDiagnostics.delete(uri);
        },
    };
}

/**
 * Combine the two channels, collapsing records that arrive via both (e.g. a
 * malformed-XML error reported both by our own syntax pass and by a compile
 * that choked on the same bad XML). Dedupe key is the full range plus message.
 */
export function mergeDiagnostics(native: Diagnostic[], additional: Diagnostic[]): Diagnostic[] {
    const merged: Diagnostic[] = [];
    const seen = new Set<string>();
    for (const diagnostic of [...native, ...additional]) {
        const { start, end } = diagnostic.range;
        const key = `${start.line}:${start.character}-${end.line}:${end.character}:${diagnostic.severity ?? ""}:${diagnostic.message}`;
        if (seen.has(key)) {
            continue;
        }
        seen.add(key);
        merged.push(diagnostic);
    }
    return merged;
}
