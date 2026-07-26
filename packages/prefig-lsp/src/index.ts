/**
 * Browser Worker entry point for the PreFigure language server.
 *
 * Importing this module for its side effects boots a full LSP server that
 * speaks JSON-RPC over the Worker's postMessage channel. This is the shape the
 * VS Code web extension reuses verbatim (its language-server bundle is one
 * line: `import "@prefigure/lsp";`) and the shape the playground spawns as a
 * Worker.
 */
import {
    BrowserMessageReader,
    BrowserMessageWriter,
    createConnection,
} from "vscode-languageserver/browser";
import { createServer } from "./server";

const messageReader = new BrowserMessageReader(self as never);
const messageWriter = new BrowserMessageWriter(self as never);
const connection = createConnection(messageReader, messageWriter);
createServer(connection);
connection.listen();
