# Porting status: Python → Rust

The Rust port shadows the reference Python implementation (see
`RUST_PORT_OUTLINE.md` at the repo root). This file is the sync contract:
one row per Python module, the Rust module that mirrors it, and the Python
commit it was last verified against. **When you change a `packages/prefig/`
module, update the matching Rust module and this table.**

Last full sync: Python core @ `d0ac23a` (version 0.7.0). Since then the repo was
reorganized (#64: `prefig/` → `packages/prefig/`, `tests/` → `packages/tests/`)
with no core-logic change, and the `<circuit>` elements (#67) were added and are
**not yet ported** (see the table).

## Status

Every `packages/prefig/core/*.py` handler is ported **except the `<circuit>`
elements added in #67** (`core/circuit.py`, `core/circuit_geometry/`), which are
not yet ported — see the table. The test corpus is the shared, language-neutral
one under `packages/tests/` (`packages/tests/examples`, `packages/tests/snapshots`,
`packages/tests/expressions`) — the same assets the Python suite uses. All 167 snapshotted examples build with the Rust pipeline and 152 match
the Python SVG output within tolerance (`tests/expected_svgs.rs`,
`MUST_PASS_ALL = true`); the other 15 are in its `KNOWN_NON_PARITY` list with
per-case reasons. Snapshots are generated in the `pretext` environment so
`<read>`/`<image>` resolve their data files
(`poetry run python packages/tests/helpers/generate_snapshots.py`).

Two subsystems are implemented but cannot be coordinate-parity-tested against
the reference, by nature: boolean `<shape>` ops (geometrically correct via the
`geo` crate, but not vertex-identical to shapely) and automatic `<network>`
layouts (valid deterministic layouts, but not identical to networkx's PRNG-based
coordinates). A long ODE integration (judson-system) drifts past tolerance late
in the trajectory from fp accumulation. Two snapshots capture reference-Python
bugs the Rust port does not reproduce (an outlined `<point>` dropped from the
output; `<clip shape=...>` truncating everything after the first shape).
Tactile output is a faithful port; it needs a braille translator (the
`braille-liblouis` feature natively, or the browser host in WASM), neither
available in CI, so it has no snapshot — but Python's own no-liblouis tactile
path actually crashes on outlined labels where the Rust port renders correctly.

| Python module | Rust module | Status | Notes |
|---|---|---|---|
| `core/user_namespace.py` | `evaluator/` (mod, parse, interp, builtins) | ✅ | Instance-based `ExpressionContext`; dedicated PEG parser (outline §16); ODE `delta`/break state on the context. |
| `core/math_utilities.py` | `evaluator/builtins.rs`, `core/math_utilities.rs` | ✅ | includes `intersect`/`solve`/`proj_2d`/`line_intersection`/`filter`/`delta` via the `env_bbox`/`env_ctm3d`/delta handles on the context. |
| `core/calculus.py` | `core/calculus.rs` | ✅ | |
| `core/utilities.py` | `core/utilities.rs` | ✅ | |
| `core/CTM.py` | `core/ctm.rs`, `core/ctm_handlers.rs` | ✅ | |
| `core/diagram.py` | `core/diagram.rs`, `core/svg11.rs` | ✅ | `svg11` output format ports `svg11_conversion` (href→xlink:href, backward-arrow `arrow-head-start` markers, `auto-start-reverse`→`auto`); parity in `tests/svg11.rs`. Serializes `xlink:href` with one root `xmlns:xlink` rather than lxml's per-element `ns0:` prefixes (same SVG 1.1). |
| `core/parse.py` | `core/parse.rs` | ✅ | |
| `engine.py` | `engine.rs` | ✅ | `build_from_string`, `build_source`, `build` (file). |
| `core/tags.py` | `core/tags.rs` | ✅ | |
| `core/label.py`, `label_tools.py` | `core/label.rs`, `core/label_tools.rs`, `core/ratex_math.rs` | ✅ | svg + braille label placement. Math backends, priority order: `ratex` (pure-Rust RaTeX LaTeX→SVG, in-module, KaTeX-styled — the host-free wasm path) → `mathjax-js` (embedded JS engine, experimental) → node/MathJax (`LocalMathLabels`) → WASM host `processMath`. Text via cairo (`text-cairo`) or the WASM host; braille via liblouis (`braille-liblouis`) or the WASM host. |
| `core/annotations.py` | `core/annotations.rs` | ✅ | includes `diagram_to_speech` (pyodide default annotations), verified byte-for-byte against Python. |
| `core/coordinates.py` | `core/coordinates.rs` | ✅ | |
| `core/grid_axes.py`, `axes.py` | `core/grid_axes.rs`, `core/axes.rs` | ✅ | |
| `core/graph.py` | `core/graph.rs` | ✅ | |
| `core/line.py`, `point.py`, `circle.py` | same names | ✅ | `circle.angle` (angle-marker) ported. |
| `core/arrow.py` | `core/arrow.rs` | ✅ | |
| `core/definition.py`, `group.py`, `repeat.py` | same names | ✅ | |
| `core/tangent_line.py` | `core/tangent_line.rs` | ✅ | |
| `core/rectangle.py`, `polygon.py` | same names | ✅ | polygon covers spline/triangle; cubic spline in `core/spline.rs`. |
| `core/parametric_curve.py`, `path.py` | same names | ✅ | path decorations (coil/zigzag/wave/ragged/capacitor) ported; ragged RNG reimplements numpy MT19937. |
| `core/area.py`, `riemann_sum.py`, `implicit.py` | same names | ✅ | |
| `core/slope_field.py`, `vector.py` | same names | ✅ | |
| `core/image.py`, `clip.py`, `shape.py` | same names | ✅ | shape boolean ops via `geo` (feature `shapes`); not vertex-identical to shapely. |
| `core/diffeqs.py` | `core/diffeqs.rs` | ✅ | in-repo RK45 (scipy `solve_ivp` constants + dense output) + `delta`-forcing breaks. |
| `core/legend.py` | `core/legend.rs` | ✅ | svg + tactile legend. |
| `core/network.py` | `core/network.rs`, `network_layout.rs` | ✅ | 7 auto-layouts (spring/spectral/bfs/circular/random/bipartite/planar→spring); not networkx-identical. |
| `core/read.py` | `core/read.rs` | ✅ | hand-rolled CSV reader (delimiter/quotechar). |
| `core/statistics.py` | `core/statistics.rs` | ✅ | `<scatter>`/`<histogram>`; `filter` builtin for CSV columns. |
| `core/circuit.py`, `core/circuit_geometry/` | — | ❌ | **Not yet ported** (added in #67). `<circuit>` handler + `circuit_geometry/{connections,shapes}.py`; registered in Python `tags.py` as `'circuit'`. |
| `cli.py`, `engine.py` | `prefig-cli`, `engine.rs` | ✅ | `build`/`pdf`/`png`/`new`/`init`/`examples`/`validate`/`eval` all wired. `pdf`/`png` shell out to `rsvg-convert`; `validate` shells out to `xmllint`/`jing` (no pure-Rust RelaxNG); `view` (browser launcher) is intentionally omitted. |

## Test suites (TDD ground truth)

| Suite | Source of truth | Regenerate with |
|---|---|---|
| `tests/parser.rs` | outline §16.5 corner-case table | hand-maintained |
| `tests/expression_tests.rs` + `<repo>/packages/tests/expressions/expression_tests.json` | Python `user_namespace` (147 steps, 12 sessions) | `poetry run python packages/tests/helpers/generate_expressions.py` |
| `tests/expected_svgs.rs` + `<repo>/packages/tests/snapshots/examples/**/*.svg` | Python-built SVGs (pretext env): 167 across hand_crafted / extracted_from_docs / uses_external_data | `poetry run python packages/tests/helpers/generate_snapshots.py` |
| `tests/examples_build_but_output_not_checked.rs` | every `<repo>/packages/tests/examples/**/*.xml` builds (svg+tactile) without crashing; output not checked here | shares the corpus above |
| `tests/annotations.rs` | built annotation XML matches the committed `snapshots/examples/**/*.xml` | mirrors Python `test_annotations_match_snapshot` |
| `tests/tactile.rs` | tactile builds are distinct from svg and laid out on the fixed 828×792 emboss page | property-based (no Python tactile reference in CI); braille content needs liblouis so isn't asserted |
| `packages/prefig-wasm/tests/snapshots.test.ts` | both wasm build variants (`pkg` MathJax-via-host + `pkg-native` RaTeX) compile the whole example corpus (svg + tactile) without crashing and keep the tactile invariants; a cross-variant check confirms the two math backends differ | the wasm-boundary analogue of `examples_build_but_output_not_checked.rs` + `tactile.rs`; a stub host API mirrors the native stub labels; a variant whose pkg isn't built is skipped. Numeric parity vs the Python SVG snapshots is NOT asserted at the wasm layer — the wasm label backends (host MathJax/canvas/SRE or in-module RaTeX, `pyodide` env) differ from Python's (`mj_sre`/pycairo/liblouis, `pretext` env), the same reason the native stub tests skip SVG parity |

The parity tests build in the `pretext` environment and need MathJax (node) and
libcairo on the host; data files (`<repo>/packages/tests/examples/*/data/`) are checked
in for `<read>`/`<image>`. See `<repo>/packages/tests/README.md` for the corpus layout
and regeneration workflow.

## Behavioral findings pinned by the test data (don't "fix" these)

- `m[1, 0]` is numpy **fancy row-indexing** (rows 1 and 0), not element
  `[1][0]` — Python's TransformList wraps the index tuple in `np.array`.
- `[(1,2),(3,4)] + (10,20)` broadcasts over rows (trailing-dimension
  alignment), not element-by-element zip.
- `round()` is banker's rounding **on the true decimal value**:
  `round(2.675, 2) == 2.67`. Implemented via format-then-parse.
- `valid_eval("  #abc")` returns the string **with leading whitespace**.
- `rgb(...)` components are evaluated then truncated toward zero (`int()`).
- Python `%` and `//` follow the divisor's sign: `-7 // 2 == -4`, `-7 % 3 == 2`.
- `math.sin(1/0)`-style errors: division is IEEE (`inf`/`nan`) but `sqrt`,
  `log`, `asin`, `acos` **raise** on domain errors — graphing relies on this to
  find domain edges and asymptotes.
- `np.linspace(a, b, N)` has **N** points (N-1 intervals). Our `linspace(a,b,m)`
  helper returns **m+1**, so call it with `N-1` when mirroring `np.linspace(...,N)`
  (diffeqs `t_eval`); the graph/axes code already accounts for this.
- The environment string controls the `data/` prefix for `<read>`/`<image>` and
  the comma format in axis labels. The parity harness uses `"pretext"` (reads
  from `data/`); `"pyodide"` uses a bare `,` grouping separator and a hashed id
  prefix. `ET.tostring` (and our writer) escape non-ASCII as `&#N;` entities.
