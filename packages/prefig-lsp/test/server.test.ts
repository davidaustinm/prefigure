import { describe, expect, test } from "vitest";
import { PassThrough } from "node:stream";
import {
    createMessageConnection,
    StreamMessageReader,
    StreamMessageWriter,
} from "vscode-jsonrpc/node";
import { createConnection } from "vscode-languageserver/node";
import type { PublishDiagnosticsParams } from "vscode-languageserver-protocol";
import { createServer, SET_ADDITIONAL_DIAGNOSTICS } from "../src/server";

// Talk to the server by plain LSP method name rather than the typed
// `vscode-languageserver-protocol` request objects. Those objects carry a
// `ParameterStructures` singleton, and there are several copies of
// `vscode-jsonrpc` in the tree (the protocol package pins its own); a raw
// client built from a different copy fails the identity check on those
// singletons ("Unknown parameter structure byName"). Method strings have no
// such coupling — and this is exactly how the thin hand-rolled playground
// client talks too.
const METHOD = {
    initialize: "initialize",
    initialized: "initialized",
    didOpen: "textDocument/didOpen",
    didChange: "textDocument/didChange",
    publishDiagnostics: "textDocument/publishDiagnostics",
} as const;

const URI = "file:///test.xml";

/**
 * Stand up the real server on one end of an in-process duplex pair and a
 * plain JSON-RPC client on the other — no editor, no browser. This is the
 * DoenetML `init-message-connection` pattern: exercise the actual LSP wire
 * protocol and assert on published diagnostics.
 */
function connect() {
    const clientToServer = new PassThrough();
    const serverToClient = new PassThrough();

    const server = createConnection(
        new StreamMessageReader(clientToServer),
        new StreamMessageWriter(serverToClient),
    );
    createServer(server);
    server.listen();

    const client = createMessageConnection(
        new StreamMessageReader(serverToClient),
        new StreamMessageWriter(clientToServer),
    );

    // Single-consumer queue of published diagnostics, drained in arrival order.
    const queue: PublishDiagnosticsParams[] = [];
    let waiting: ((p: PublishDiagnosticsParams) => void) | null = null;
    client.onNotification(METHOD.publishDiagnostics, (params: PublishDiagnosticsParams) => {
        if (waiting) {
            const resolve = waiting;
            waiting = null;
            resolve(params);
        } else {
            queue.push(params);
        }
    });
    client.listen();

    function nextDiagnostics(): Promise<PublishDiagnosticsParams> {
        const queued = queue.shift();
        if (queued) {
            return Promise.resolve(queued);
        }
        return new Promise((resolve) => {
            waiting = resolve;
        });
    }

    async function initialize() {
        await client.sendRequest(METHOD.initialize, {
            processId: null,
            rootUri: null,
            capabilities: {},
        });
        await client.sendNotification(METHOD.initialized, {});
    }

    function open(text: string) {
        return client.sendNotification(METHOD.didOpen, {
            textDocument: { uri: URI, languageId: "xml", version: 1, text },
        });
    }

    function change(text: string, version: number) {
        return client.sendNotification(METHOD.didChange, {
            textDocument: { uri: URI, version },
            contentChanges: [{ text }],
        });
    }

    function dispose() {
        client.dispose();
        server.dispose();
    }

    return { client, initialize, open, change, nextDiagnostics, dispose };
}

describe("prefig language server", () => {
    test("publishes a diagnostic for the missing-</m> document", async () => {
        const c = connect();
        await c.initialize();
        await c.open(`<label alignment="ne"><m>(x2,y2)</label>`);
        const published = await c.nextDiagnostics();
        expect(published.uri).toBe(URI);
        expect(published.diagnostics).toHaveLength(1);
        expect(published.diagnostics[0].message).toBe("Missing closing tag for <m>.");
        c.dispose();
    });

    test("publishes no diagnostics for well-formed XML", async () => {
        const c = connect();
        await c.initialize();
        await c.open(`<diagram><point at="p"/></diagram>`);
        const published = await c.nextDiagnostics();
        expect(published.diagnostics).toEqual([]);
        c.dispose();
    });

    test("re-validates on incremental change", async () => {
        const c = connect();
        await c.initialize();
        await c.open(`<diagram><point at="p"/></diagram>`);
        expect((await c.nextDiagnostics()).diagnostics).toEqual([]);

        await c.change(`<diagram><point at="p"></diagram>`, 2);
        const afterEdit = await c.nextDiagnostics();
        expect(afterEdit.diagnostics.length).toBeGreaterThanOrEqual(1);
        expect(afterEdit.diagnostics[0].message).toBe("Missing closing tag for <point>.");
        c.dispose();
    });

    test("merges host-pushed diagnostics via prefigure/setAdditionalDiagnostics", async () => {
        const c = connect();
        await c.initialize();
        await c.open(`<diagram><point at="p"/></diagram>`);
        expect((await c.nextDiagnostics()).diagnostics).toEqual([]);

        const compileError = {
            range: {
                start: { line: 0, character: 0 },
                end: { line: 0, character: 8 },
            },
            severity: 1,
            source: "prefig-compile",
            message: "name 'f' is not defined",
        };
        await c.client.sendNotification(SET_ADDITIONAL_DIAGNOSTICS, {
            uri: URI,
            diagnostics: [compileError],
        });

        const published = await c.nextDiagnostics();
        expect(published.diagnostics).toContainEqual(compileError);
        c.dispose();
    });
});
