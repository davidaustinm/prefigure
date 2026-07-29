# @prefigure/prefig-wasm

PreFigure (the Rust core) compiled to WebAssembly for use in the browser and
Node. Exposes `build_from_string`, an `Evaluator`, `set_host_api`, and
`version` — the same surface the playground drives.

## Two build variants

The one crate produces two npm packages that differ **only in how math labels
are rendered**. The JavaScript API is identical, so a consumer picks a variant
by which package it installs.

| Variant | Output dir | npm name | Math rendering | Host must provide |
|---|---|---|---|---|
| **MathJax** (default) | `pkg/` | `prefig-wasm` | host `processMath` callback (MathJax in the playground) | `measure_text`, `translate_text`, `processMath`, `processBraille` |
| **native** (RaTeX) | `pkg-native/` | `prefig-wasm-native` | pure-Rust RaTeX (`--features ratex`), **in the wasm module** | `measure_text`, `translate_text` only |

The native variant renders math itself, so the host never needs MathJax or the
speech-rule-engine — the big dependency drops out. That's the reason to pick it.

### What the native variant still needs from the host

It is **not** fully host-free:

- **Plain (non-math) text measurement** still comes from the host `measure_text`
  (canvas `measureText` in the browser) — wasm has no cairo.
- **Braille for tactile math** still comes from the host `translate_text` —
  RaTeX emits SVG glyphs, not Nemeth braille.

So `processMath`/`processBraille` become unused, but a small host is still
required for text metrics and braille.

### Styling caveat

RaTeX is **KaTeX-styled**, not MathJax-styled, so the native variant's math
labels look slightly different from the MathJax variant (and from the reference
Python output, which uses MathJax). The two variants therefore do not produce
byte-identical SVGs; only structural output is comparable.

## Building

```sh
npm run build          # MathJax variant -> pkg/     (alias of build:mathjax)
npm run build:native   # native variant  -> pkg-native/
npm run build:all      # both
```

`build:native` passes `--features ratex` to the crate and relabels the generated
`pkg-native/package.json` as `prefig-wasm-native` (see
`scripts/label-native-pkg.mjs`).

## Testing

```sh
npm test        # builds both variants, then runs vitest over both
npm run test:only   # runs vitest against whatever pkg dirs are already built
npm run typecheck   # tsc --noEmit
```

`tests/snapshots.test.ts` runs the whole shared example corpus
(`../tests/examples`) through each built variant and checks the tactile-layout
invariants; a variant whose pkg dir is absent is skipped. A cross-variant test
confirms the two math backends genuinely differ.
