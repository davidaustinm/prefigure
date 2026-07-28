# PreFigure benchmarks

Performance comparison between different PreFigure builds (e.g., Python vs. Rust).

## What it measures

[`bench_build.py`](bench_build.py) times an end-to-end `prefig build` of each
example diagram in [`../tests/examples/hand_crafted/`](../tests/examples/hand_crafted/) and reports the mean/standard
deviation and the speedup of each backend relative to Python. Pass `--wasm` to
also benchmark the WebAssembly build (via [`bench_wasm.mjs`](bench_wasm.mjs),
described below).

The C++ profiler could time the C++ core *in-process* because it was a
pybind11 extension module. The Rust implementation is a standalone binary, so
both backends are instead timed the same way — as a subprocess
`prefig build -i <example>.xml`. That keeps the two numbers directly
comparable and reflects real command-line usage, **including each
implementation's process-startup and import cost**.

Pass `--startup` to see how much of each time is fixed startup overhead: the
Python interpreter plus `numpy`/`scipy`/`shapely`/`lxml` imports account for a
large, constant slice of every Python build, whereas the Rust binary starts in
a couple of milliseconds. Subtract those baselines to compare the diagram work
itself.

## The WASM build (`bench_wasm.mjs`)

[`bench_wasm.mjs`](bench_wasm.mjs) benchmarks the **same Rust core compiled to
`wasm32`**, driven from JavaScript through `build_from_string` exactly as the
browser playground drives it. Run it directly with Node, or fold it into the
main table with `bench_build.py --wasm`.

The comparison is not perfectly apples-to-apples, and the harness is explicit
about why:

- **Math rendering is kept warm.** The Python and Rust CLIs render math labels
  by spawning a fresh `node` + MathJax subprocess on every build. The WASM
  benchmark instead loads the bundled `mathjax-full` (the same MathJax the CLIs
  use, from `prefig/core/mj_sre`) **once, in-process**, mirroring how a browser
  keeps MathJax resident. So the WASM number reflects "the engine with math
  kept warm" and does not pay per-build node startup.
- **Text metrics are approximated.** Plain Node has no canvas/cairo, so glyph
  widths are estimated. That shifts label *placement* but not the amount of
  engine work being timed. Output *correctness* is checked by a separate script
  (see below), not here.

Because of the warm-MathJax difference, read the WASM column as a lower bound on
the engine's raw throughput, not as a drop-in CLI timing.

Build the WASM package first:

```sh
cd ../prefig-wasm && npm run build     # wasm-pack build --target nodejs
```

Then either:

```sh
# Standalone
node bench_wasm.mjs --runs 5

# Merged into the Rust/Python table and chart
python bench_build.py --wasm --startup --output all.png
```

## Prerequisites

- **Python backend** installed (a project virtualenv at `../../.venv` is picked up
  automatically, otherwise `prefig` on `PATH`, otherwise `python -m prefig.cli`):

  ```sh
  pip install -e ../..
  ```

- **Rust CLI** built in release mode:

  ```sh
  cd ../prefig-rust && cargo build --release -p prefig-cli
  ```

- **WASM package** (only for `--wasm`) built for Node, plus `node` on `PATH`:

  ```sh
  cd ../prefig-wasm && npm run build
  ```

- **matplotlib** (only for `--output`, the PNG chart): `pip install matplotlib`

## Usage

```sh
# Full run, 5 timed builds per example, with startup breakdown
python bench_build.py --startup

# Quicker iteration on a subset
python bench_build.py --runs 3 --examples tangent.xml implicit.xml

# All three backends, with a chart and raw numbers
python bench_build.py --wasm --startup --output all.png --json results.json
```

Useful flags:

| Flag | Meaning |
| --- | --- |
| `--runs N` | Timed builds per example (default 5) |
| `--warmup N` | Discarded warmup builds per example (default 1) |
| `--examples a.xml b.xml` | Only benchmark these examples |
| `--wasm` | Also benchmark the WASM build via Node |
| `--node PATH` | Node executable for the WASM benchmark (default `node`) |
| `--startup` | Also report fixed startup cost (`prefig --help`) |
| `--python-cmd "..."` | Override the Python `prefig` command |
| `--rust-bin PATH` | Override the Rust binary path |
| `--output FILE.png` | Write a bar-chart PNG (needs matplotlib) |
| `--json FILE` | Write raw results as JSON |

The script exits non-zero if any active backend fails a build, so it can double
as a smoke test in CI.

## Example output

```
  example            Python (ms)        Rust (ms)        WASM (ms)     Rust x   WASM x
  -----------------------------------------------------------------------------------
  tangent             792.7±  4.0      363.9±  4.7       28.4±  1.1      2.2x     27.9x
  implicit           1038.5± 10.1      427.4±  8.5       61.2±  2.0      2.4x     17.0x
  ...
  -----------------------------------------------------------------------------------
  TOTAL              6675.5           2967.5            330.0            2.2x     20.2x

  Startup overhead (`prefig --help`, subtract from each column above):
    Python: 422.5 ms    Rust: 2.6 ms
```

(WASM keeps MathJax warm in-process while the CLIs spawn it per build; see the
WASM section above for how to read that column. Numbers are illustrative.)

## Correctness

Performance is only meaningful if the backends produce equivalent output. The
tolerance-based SVG comparison for that lives separately — the C++ port's
[`correctness_comparison.py`](../../tmp-prefig-with-cpp/correctness_comparison.py)
is the reference implementation being adapted for the Rust CLI (see
`RUST_PORT_OUTLINE.md` §12).
