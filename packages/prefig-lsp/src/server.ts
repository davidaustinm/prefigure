import {
    Connection,
    Diagnostic,
    InitializeParams,
    InitializeResult,
    TextDocuments,
    TextDocumentSyncKind,
} from "vscode-languageserver";
import { TextDocument } from "vscode-languageserver-textdocument";
import { validateSyntax } from "./features/validate-syntax";
import { createAdditionalDiagnosticsStore, mergeDiagnostics } from "./globals";

/**
 * Custom notification the host uses to push externally-produced diagnostics
 * (a real prefig compile) into the server. Mirrors DoenetML's
 * `doenet/setAdditionalDiagnostics`.
 */
export const SET_ADDITIONAL_DIAGNOSTICS = "prefigure/setAdditionalDiagnostics";

export interface SetAdditionalDiagnosticsParams {
    uri: string;
    diagnostics: Diagnostic[];
}

/**
 * Wire up the PreFigure language server on a transport-agnostic `Connection`.
 *
 * The browser Worker entry (`index.ts`) and the in-process test harness both
 * construct a `Connection` with their own reader/writer and hand it here, so
 * the exact same server logic runs in the playground Worker, in a VS Code web
 * extension, and under vitest.
 */
export function createServer(connection: Connection): void {
    const documents = new TextDocuments(TextDocument);
    const additionalDiagnostics = createAdditionalDiagnosticsStore();

    connection.onInitialize((_params: InitializeParams): InitializeResult => {
        return {
            capabilities: {
                textDocumentSync: TextDocumentSyncKind.Incremental,
            },
        };
    });

    function publishFor(uri: string): void {
        const document = documents.get(uri);
        const native = document ? validateSyntax(document.getText()) : [];
        const additional = additionalDiagnostics.get(uri);
        connection.sendDiagnostics({
            uri,
            diagnostics: mergeDiagnostics(native, additional),
        });
    }

    // Fires on open and on every (incremental) change.
    documents.onDidChangeContent((change) => publishFor(change.document.uri));

    documents.onDidClose((event) => {
        additionalDiagnostics.clear(event.document.uri);
        connection.sendDiagnostics({ uri: event.document.uri, diagnostics: [] });
    });

    connection.onNotification(
        SET_ADDITIONAL_DIAGNOSTICS,
        (params: SetAdditionalDiagnosticsParams) => {
            additionalDiagnostics.set(params.uri, params.diagnostics ?? []);
            publishFor(params.uri);
        },
    );

    documents.listen(connection);
}
