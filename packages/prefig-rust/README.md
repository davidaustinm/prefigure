# PreFigure in Rust

A Rust version of [PreFigure](https://prefigure.org), ported from the Python
implementation in [`../prefig/`](../prefig/). The Python version remains the
reference; this port follows it module by module so the two stay in sync
(see [PORTING.md](PORTING.md) for what is done and what isn't).

The main goal is a small, fast WebAssembly build so that websites (the
PreFigure playground, [DoenetML](https://github.com/Doenet/DoenetML)) can build
diagrams in the browser without downloading the much larger Python stack.

## What's here

| Directory | Contents |
|---|---|
| `prefig-core/` | The library: expression evaluator and the diagram drawing pipeline |
| `prefig-cli/` | The `prefig` command-line program |

The WebAssembly bindings live in the sibling package
[`packages/prefig-wasm`](../prefig-wasm/), which depends on `prefig-core` here
via a path dependency.

The drawing pipeline builds all 37 bundled example diagrams to SVG that matches
the Python version within tolerance (`prefig-core/tests/expected_svgs.rs`). A
few elements are not ported yet — boolean `<shape>` operations, automatic
`<network>` layout, `<read>`, `<histogram>`/`<scatter>`, and tactile output;
see [PORTING.md](PORTING.md).

## Requirements

- Rust (edition 2021 or later) — <https://rustup.rs>
- For the WebAssembly build: [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) and Node
- For regenerating test data: the Python version installed at the repo root (`poetry install`)

## Build and test

```sh
cd packages/prefig-rust
cargo build            # build everything
cargo test             # run all tests
```

Try the command line:

```sh
cargo run -p prefig-cli -- eval "(1,2) + (3,4)"
```

## WebAssembly

```sh
cd packages/prefig-wasm
npm install
npm test               # compiles to WebAssembly, then runs the Node tests
```

The compiled package lands in `packages/prefig-wasm/pkg/`.

## Test data

The tests compare this port against output from the Python version:

- `packages/tests/expressions/expression_tests.json` — expressions with the results
  Python produces. Regenerate with
  `poetry run python packages/tests/helpers/generate_expressions.py`.
- `packages/tests/snapshots/` — SVGs that Python builds from the diagrams in
  `packages/tests/examples/` (the shared corpus the Python suite also uses).
  Regenerate with `poetry run python packages/tests/helpers/generate_snapshots.py`.

Both are checked in, so running the tests does not require Python. Regenerate
them whenever the Python version changes behavior.

## Design documents

- [`../../RUST_PORT_OUTLINE.md`](../../RUST_PORT_OUTLINE.md) — the full plan for the port
- [PORTING.md](PORTING.md) — per-module status and rules for staying in sync
- [`../prefig-wasm/PLAYGROUND_PLAN.md`](../prefig-wasm/PLAYGROUND_PLAN.md) — plan for using the WebAssembly build on the website
