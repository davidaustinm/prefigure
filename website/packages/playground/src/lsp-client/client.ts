import {
    BrowserMessageReader,
    BrowserMessageWriter,
    createMessageConnection,
    MessageConnection,
} from "vscode-jsonrpc/browser";
import type { Diagnostic } from "vscode-languageserver-protocol";
import PrefigLspWorker from "./lsp.worker?worker";

/**
 * A thin, editor-agnostic client for the PreFigure language server.
 *
 * It owns the Worker + JSON-RPC connection and speaks LSP by plain method name
 * (no `vscode-languageserver-protocol` request objects — those carry
 * `ParameterStructures` singletons that break across the several copies of
 * `vscode-jsonrpc` in the tree). Diagnostics are handed back through the
 * `onDiagnostics` callback; translating them into editor markers is the
 * caller's job, which keeps this reusable if the editor ever changes.
 *
 * This is the Monaco counterpart of DoenetML's ~300-line CodeMirror LSP
 * adapter: same protocol, different editor.
 */

export const LSP_DOCUMENT_URI = "inmemory://model/prefigure.xml";

const SET_ADDITIONAL_DIAGNOSTICS = "prefigure/setAdditionalDiagnostics";
const METHOD = {
    initialize: "initialize",
    initialized: "initialized",
    didOpen: "textDocument/didOpen",
    didChange: "textDocument/didChange",
    publishDiagnostics: "textDocument/publishDiagnostics",
} as const;

interface PublishDiagnosticsParams {
    uri: string;
    diagnostics: Diagnostic[];
}

export type { Diagnostic };

export class PrefigureLspClient {
    private readonly worker: Worker;
    private readonly connection: MessageConnection;
    private readonly ready: Promise<void>;
    private failed = false;
    private version = 0;
    private opened = false;

    constructor(private readonly onDiagnostics: (diagnostics: Diagnostic[]) => void) {
        this.worker = new PrefigLspWorker();
        this.connection = createMessageConnection(
            new BrowserMessageReader(this.worker),
            new BrowserMessageWriter(this.worker),
        );
        this.connection.onNotification(
            METHOD.publishDiagnostics,
            (params: PublishDiagnosticsParams) => {
                if (params.uri === LSP_DOCUMENT_URI) {
                    this.onDiagnostics(params.diagnostics);
                }
            },
        );
        this.connection.listen();
        // Callers `void` these methods, so a rejected handshake would surface
        // as an unhandled rejection. Absorb it here and latch `failed` so
        // later calls become quiet no-ops rather than piling up more.
        this.ready = this.initialize().catch((error) => {
            this.failed = true;
            console.error("PreFigure language server failed to start:", error);
        });
    }

    private async initialize(): Promise<void> {
        await this.connection.sendRequest(METHOD.initialize, {
            processId: null,
            rootUri: null,
            capabilities: {},
        });
        await this.connection.sendNotification(METHOD.initialized, {});
    }

    /**
     * Push the current document text to the server. The server declares
     * incremental sync, but a change event with no range is a full replace,
     * which is all the playground needs (one small model, retyped freely).
     */
    async syncDocument(text: string): Promise<void> {
        await this.ready;
        if (this.failed) {
            return;
        }
        try {
            if (!this.opened) {
                this.opened = true;
                this.version = 1;
                await this.connection.sendNotification(METHOD.didOpen, {
                    textDocument: {
                        uri: LSP_DOCUMENT_URI,
                        languageId: "xml",
                        version: this.version,
                        text,
                    },
                });
            } else {
                this.version += 1;
                await this.connection.sendNotification(METHOD.didChange, {
                    textDocument: {
                        uri: LSP_DOCUMENT_URI,
                        version: this.version,
                    },
                    contentChanges: [{ text }],
                });
            }
        } catch (error) {
            console.error("PreFigure language server document sync failed:", error);
        }
    }

    /**
     * Feed the server diagnostics produced *outside* it — here, the real
     * prefig compile that runs in the Pyodide worker. The server merges these
     * with its own XML-syntax diagnostics and re-publishes. Pass `[]` to clear.
     */
    async setAdditionalDiagnostics(diagnostics: Diagnostic[]): Promise<void> {
        await this.ready;
        if (this.failed) {
            return;
        }
        try {
            await this.connection.sendNotification(SET_ADDITIONAL_DIAGNOSTICS, {
                uri: LSP_DOCUMENT_URI,
                diagnostics,
            });
        } catch (error) {
            console.error(
                "PreFigure language server additional-diagnostics push failed:",
                error,
            );
        }
    }

    dispose(): void {
        try {
            this.connection.dispose();
        } catch {
            // already torn down
        }
        this.worker.terminate();
    }
}
