# PreFigure Language Server — Plan

Status: **Phases 0 and 1 implemented** (2026-07-17); Phases 2–6 proposed.

Implemented so far:

- `website/packages/prefig-lsp` — the server (`@prefigure/lsp`). Phase 0
  XML-well-formedness diagnostics via `@lezer/xml` (structure/ranges) + `saxes`
  (backstop messages), the two-channel design (`prefigure/setAdditionalDiagnostics`),
  and both unit and in-process LSP-protocol tests (13 tests, all green).
- `website/packages/playground/src/lsp-client` — the thin Monaco client:
  `lsp.worker.ts` (`import "@prefigure/lsp"`), `client.ts` (JSON-RPC over the
  Worker, editor-agnostic), `compile-error.ts` (the compile→additional-diagnostics
  bridge). `editor.tsx` paints diagnostics as Monaco markers in XML mode and
  feeds compile errors into the second channel. Production build bundles the
  worker cleanly.

Decisions locked so far:

- **Keep Monaco** in the playground; write a thin hand-rolled LSP client
  against Monaco's provider APIs rather than adopting `monaco-languageclient`.
  Revisit only if Monaco integration fights back.
- The LSP core is **self-contained JS/TS** — no Pyodide dependency. Anything
  Python (the authoritative RelaxNG validation, real compiles) reaches the
  editor through a side channel (see "Two diagnostic channels").
- Model the whole thing on DoenetML's editor-tooling stack
  (`packages/lsp`, `packages/lsp-tools`, `packages/codemirror/src/extensions/lsp`,
  `packages/vscode-extension` in <https://github.com/Doenet/DoenetML>), which
  runs one LSP-protocol server identically in a browser Worker and in a VS Code
  web extension.

## Why an LSP (and not just editor callbacks)

A real `vscode-languageserver/browser` server running in a Web Worker — talking
JSON-RPC over `postMessage` via `BrowserMessageReader`/`BrowserMessageWriter` —
is reusable byte-for-byte as the server of a VS Code **web extension**
(`vscode-languageclient/browser` + `new Worker(bundle)`). DoenetML's extension
proves the shape: its language-server entry point is literally
`import "@doenet/lsp";`. Building features as one-off Monaco callbacks would
mean writing them twice.

## Architecture

```
website/packages/prefig-lsp        ── the server (new)
  src/index.ts                     worker entry: createConnection(BrowserMessageReader/Writer)
  src/globals.ts                   per-document state map, settings
  src/features/validate-syntax.ts  Phase 0: XML well-formedness diagnostics
  src/features/validate-schema.ts  Phase 3: table-based schema checks
  src/features/completions.ts      Phase 4
  src/features/hover.ts            Phase 4
  src/source-object/               offset<->position, element-at-offset, tag/attr
                                   ranges (mirror of Doenet's doenet-source-object),
                                   built on @lezer/xml (error-tolerant, positions)
  src/generated/schema.json        Phase 2 output: flattened schema table
  test/                            LSP-protocol tests over an in-process message
                                   connection (Doenet's init-message-connection.ts
                                   pattern)

website/packages/playground        ── the client (existing app)
  src/lsp-client/                  thin Monaco adapter (~300 lines):
                                   vscode-jsonrpc/browser to the LSP worker;
                                   setModelMarkers + registerCompletionItemProvider
                                   + registerHoverProvider on the Monaco side
  (existing Pyodide worker)        unchanged; additionally pushes compile errors
                                   into the LSP via prefigure/setAdditionalDiagnostics

scripts/ (or tests/helpers/)       ── build-time schema-table generator (Phase 2)
  generate_lsp_schema.py           lxml walk of prefig/resources/schema/pf_schema.rng
                                   -> prefig-lsp/src/generated/schema.json (run in CI)

packages/vscode-extension          ── Phase 5 (mirror Doenet's layout)
  src/language-server/index.ts     `import "@prefigure/lsp";` (one line)
  src/extension/index.ts           vscode-languageclient/browser + Worker
  extension/                       manifest, TextMate grammar, language config
```

### Two diagnostic channels (the key architectural rule)

1. **LSP-native diagnostics** — fast, error-tolerant, precise ranges, computed
   in the worker on every (debounced) change. Sources: XML well-formedness
   (Phase 0), schema table checks (Phase 3). These must never require Python.
2. **`prefigure/setAdditionalDiagnostics`** — a custom JSON-RPC method
   (mirroring Doenet's `doenet/setAdditionalDiagnostics`) that lets the *host*
   push externally produced diagnostics into the server, which merges and
   re-publishes them. The playground feeds it from the Pyodide compile it
   already runs (real prefig errors: expression failures, runtime errors, and
   — optionally — authoritative `lxml.RelaxNG` output, line-granular). VS Code
   can later feed it from a `prefig-wasm` sub-worker. A dedupe pass (Doenet's
   `dedupe-lsp-diagnostics.ts`) collapses records that arrive via both
   channels.

Rationale: editor-grade and authoritative validation have opposite
requirements (fast/tolerant/precise-range vs. slow/whole-document/line-only).
Keeping them in separate channels avoids both a laggy editor and imprecise
squiggles, and keeps the LSP bundle small enough that the VS Code extension
stays a one-liner. Verified constraint that shaped this: `lxml`'s RelaxNG
validator reports **line numbers only (column always 0)** and emits cascading
duplicate messages on this schema's heavy `interleave` use — fine for a
side-channel, unacceptable as the primary squiggle source.

## Phases

### Phase 0 — LSP skeleton + XML well-formedness only (no schema) ✅ done

The server validates **syntax only**: mismatched/unclosed tags, malformed
attributes, stray `<`/`&`, duplicate attributes. Nothing is checked against
the PreFigure schema.

Built as described below. Notes from implementation: `@lezer/xml`'s error
nodes give precise *ranges* (the `MissingCloseTag`'s parent `Element` → its
`OpenTag` is exactly the `<m>` to anchor on) but no *messages* — those are
synthesized from node type; duplicate attributes are detected off the same
tree (lezer tolerates them). `saxes` runs only when lezer is clean, as a
message-quality backstop, so no error is ever reported twice.

- Scaffold `prefig-lsp` with `vscode-languageserver/browser` (copy Doenet's
  `index.ts` shape: capabilities handshake, `TextDocuments`, per-document
  state map).
- Parse with `@lezer/xml` (error-tolerant, position-tracking, works outside
  CodeMirror). Diagnostics from error nodes and `MismatchedCloseTag` /
  missing-close detection, each anchored to the offending tag's range.
  If lezer's error nodes prove too vague for good messages, layer a strict
  streaming pass (e.g. `saxes`) for message quality — lezer for ranges,
  saxes for words.
- Tests: in-process message-connection tests (open doc → edit → assert
  published diagnostics), including the exact failure from the field: a
  `<label ...><m>(x2,y2)</label>` missing its `</m>` must produce a marker on
  that tag.

This phase alone has user-visible value: that missing-`</m>` bug reached the
compiler and surfaced only as a console traceback; with Phase 0 it's a red
squiggle at the right spot before compile is ever attempted.

Gate: none (no schema involvement, minimal risk).

### Phase 1 — Playground integration (Monaco, thin client) ✅ done

- `src/lsp-client/`: connect Monaco to the LSP worker with
  `vscode-jsonrpc/browser` directly (Doenet's CodeMirror client is a thin
  ~300-line adapter over the same protocol — same idea, different editor).
  Map `textDocument/publishDiagnostics` → `monaco.editor.setModelMarkers`.
- Document lifecycle: one LSP document tracking the single Monaco model;
  incremental sync (`TextDocumentSyncKind.Incremental`).
- Wire the compile-error bridge: when the existing Pyodide compile fails,
  translate the error (it already carries a line number for XML syntax errors;
  compile errors may be document-level) into
  `prefigure/setAdditionalDiagnostics`. The error banner from the recent
  error-display work stays; markers and banner complement each other.
- Explicitly out: `monaco-languageclient` (requires the
  `@codingame/monaco-vscode-api` service layer, which conflicts with
  `@monaco-editor/react`'s loader). If the thin client ever feels like a
  reimplementation treadmill, that's the signal to reevaluate — not before.

### Phase 2 — Schema-table generator (spike + gate)

A build-time Python script (lxml, living beside the existing test helpers,
run in CI) walks `prefig/resources/schema/pf_schema.rng` — the authoritative,
upstream-maintained schema — and emits a flattened JSON table: per element,
its allowed attributes, required attributes, allowed children, and
allowed-at-root.

- **Over-approximation rule:** wherever RelaxNG structure is too clever to
  flatten (choice-dependent attributes, contexts reached via multiple
  `define`s), the table *allows*. The checker may under-report; it must never
  false-positive.
- **Acceptance gate:** run the Phase 3 checks against the ~170 known-valid
  example corpus in `tests/` — zero diagnostics required. The corpus is a
  ready-made ground truth.
- Deliberately *not* a general RelaxNG engine (`salve` was considered and
  dropped: dormant project, and full grammar validation isn't what we want —
  see below).

### Phase 3 — Schema validation (shallow but precise)

Modeled on Doenet's `get-schema-violations.ts`, which checks exactly four
things — unknown element, element-not-allowed-at-root, disallowed
parent→child, unknown/invalid attribute — and nothing else (no ordering, no
occurrence counts). That shallowness is deliberate: every check anchors to a
precise tag/attribute range, and a table lookup can't reproduce libxml2's
cascading-`interleave` noise. Full-grammar verdicts remain the compile-side
channel's job.

### Phase 4 — Completion and structural hover

- Completion from the same table: child elements valid under the enclosing
  element (trigger `<`), attributes of the current element (trigger space
  inside a tag), snippet-style insertion with required attributes prefilled.
- Hover: structural facts only ("allowed children: …", "required: …").
  **Known gap (verified):** `pf_schema.rng` contains zero `a:documentation`
  annotations, so there is no prose to show. Authoring per-element docs —
  ideally contributed upstream *into the schema* as `a:documentation`, so all
  consumers benefit — is a separable work item; the Guide corpus vendored in
  `tests/` is a candidate source to harvest from.

### Phase 5 — VS Code web extension

Mirror Doenet's `packages/vscode-extension` layout (three build configs:
extension, language-server, preview-window):

- `src/language-server/index.ts` is one line: `import "@prefigure/lsp";`
- `src/extension/index.ts`: `vscode-languageclient/browser`, `new Worker(...)`
  pointed at the bundled server. Works in desktop VS Code and vscode.dev.
- TextMate grammar for highlighting (start from a stock XML grammar).
- **Design item Doenet didn't have:** PreFigure sources are plain `.xml`.
  Use the language contribution's `firstLine` regex (match
  `^<\??(xml.*)?.*<?(diagram|prefigure)\b`-style pattern against
  `<diagram`/`<prefigure` on line 1) for detection, and/or bless a dedicated
  extension (`.prefig`). Needs a call from the maintainers.

### Phase 6 — `prefig-wasm` sub-worker (gated on the Rust branch maturing)

Doenet's LSP spawns a Rust/WASM core sub-worker (`rust-core.ts`: boot-probe
RPC, 30 s timeout, graceful `"unavailable"` degradation, per-document
lifecycle). PreFigure's analogue exists on the `Rust` branch: `prefig-wasm`
with `build_from_string`, and — with the `ratex` feature — fully host-free
math (no MathJax callback, no Pyodide, a few MB of wasm).

- Compile-on-idle diagnostics in VS Code (where there is no Pyodide): spawn
  the sub-worker, build on debounce, surface failures via the
  additional-diagnostics path. Caveats, verified: the Rust engine returns
  `Err(String)` — **no source positions**, so these are document-level
  diagnostics; and the Rust port is not the reference implementation.
- Preview panel (Doenet's `preview-panel/` transfers structurally): live SVG
  render via `prefig-wasm` — an "edit PreFigure in VS Code with live preview"
  story with no Python anywhere.
- Copy the sub-worker *seam* (Doenet's `getRustCore` pattern) into the server
  design from day one, even though nothing uses it until this phase.

## Testing strategy

- **Server**: LSP-protocol tests over an in-process duplex connection
  (Doenet's `test/utils/init-message-connection.ts` pattern) — no editor, no
  browser; assert published diagnostics / completion payloads for scripted
  edits.
- **Schema table**: generator unit tests + the corpus zero-false-positive
  gate (Phase 2).
- **Client**: a handful of integration tests in the playground (vitest) for
  marker translation; keep thin.

## Risks / open questions

| Item | Status |
| --- | --- |
| Monaco thin client turns into a treadmill | Accepted risk (decision: keep Monaco). Escape hatch documented in Phase 1. |
| Lezer error nodes give poor *messages* | Mitigation in Phase 0: strict `saxes` pass for message text. |
| Schema constructs that don't flatten | Over-approximation rule + corpus gate (Phase 2). |
| No docs in schema for hover | Verified gap; separable authoring task, ideally upstreamed as `a:documentation`. |
| `.xml` file association in VS Code | `firstLine` regex and/or new `.prefig` extension — maintainer call (Phase 5). |
| Rust engine errors carry no positions | Document-level diagnostics only in Phase 6; fine for compile-status, not for squiggles. |
| Licensing of lifted Doenet code | Doenet is AGPL-3.0, PreFigure GPL-3.0-or-later; GPLv3 §13 covers combination, and the Doenet editor-tooling code was authored by this project's maintainer anyway. |

## Prior investigation this plan rests on (all verified in-repo)

- `prefig/resources/schema/pf_schema.rng` (2234 lines) + `.rnc` (862) exist and
  are already used by `prefig validate` (`engine.validate_source`).
- `lxml.RelaxNG` diagnostics: line-only positions, `interleave` cascades —
  measured against deliberately broken diagrams.
- The playground's compiler already runs in a Worker via comlink and loads
  `lxml` + the prefig wheel (the additional-diagnostics bridge costs nothing
  new at runtime).
- Playground `package.json` still carries CodeMirror 6 deps and `editor.tsx`'s
  docstring says CodeMirror — leftovers from an old migration; harmless, but
  clean up when touching the editor.
