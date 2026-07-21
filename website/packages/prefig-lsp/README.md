# @prefigure/lsp

The PreFigure language server. It runs unchanged in a browser Web Worker (the
playground) and, later, in a VS Code web extension — one JSON-RPC server over
`postMessage`.

See `../../../LSP_PLAN.md` for the full roadmap.

## Status: Phase 0 — XML well-formedness only

The server validates **syntax only**. Nothing here knows about the PreFigure
schema (that is Phase 3, on a separate channel). It flags:

- unclosed tags (anchored on the offending open tag),
- mismatched closing tags,
- duplicate attributes,
- stray `<` / `&`, and other lexical malformations.

Two parsers do the work (`src/features/validate-syntax.ts`):

- **`@lezer/xml`** — error-tolerant, position-tracking. Its tree structure
  tells us which element is unclosed so we can put the squiggle on the exact
  tag. It carries no messages, so we synthesize them.
- **`saxes`** — a strict fallback used only when lezer is clean, for the few
  lexical problems lezer tolerates (undefined entities, etc.), with good
  human-worded messages.

## Two diagnostic channels

1. **Native** — the XML-syntax diagnostics above, computed in-worker on every
   change. Never touches Python.
2. **`prefigure/setAdditionalDiagnostics`** — a custom notification the host
   uses to push externally-produced diagnostics (a real prefig compile) in.
   The server merges and re-publishes. See `src/globals.ts` and `src/server.ts`.

## Layout

```
src/index.ts                     browser Worker entry (import for side effects)
src/server.ts                    createServer(connection) — transport-agnostic
src/globals.ts                   additional-diagnostics store + merge/dedupe
src/features/validate-syntax.ts  Phase 0 well-formedness checks
src/source-object/positions.ts   offset <-> LSP position math
test/                            unit tests + in-process LSP-protocol tests
```

## Tests

```
npm test          # from this package (vitest)
```

`test/server.test.ts` stands up the real server on one end of an in-process
duplex stream pair and drives it as a JSON-RPC client — the same wire protocol
the playground and VS Code use, no editor or browser required.
