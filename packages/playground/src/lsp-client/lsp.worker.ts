/**
 * The PreFigure language server, hosted in a Web Worker.
 *
 * This is intentionally one line: importing `@prefigure/lsp` for its side
 * effects boots the server and binds it to this Worker's postMessage channel.
 * The VS Code web extension's language-server bundle (Phase 5) is the same one
 * line — the server is written once and hosted in whatever Worker imports it.
 */
import "@prefigure/lsp";
