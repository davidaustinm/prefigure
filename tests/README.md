# PreFigure test suite

Python tests for the reference implementation, organized so the **test assets
are language-neutral**: the snapshot corpus, example diagrams, guide figures, and
expression corpus live in plain folders here that the Python tests read today and
the Rust port's tests will be re-pointed at later (from the *same* paths). The
only Python-specific pieces are the `test_*.py` entry points and `helpers/`.

## Layout

```
tests/
  test_snapshots.py       # build each example, compare to a committed SVG snapshot
  test_expressions.py     # replay the expression corpus through user_namespace
  test_guide_figures.py   # build every Guide figure without crashing (marked slow)
  test_prefigure.py       # end-to-end `prefig` CLI smoke test
  helpers/                # all Python-side support code
    compare.py            # tolerance SVG structural comparator
    build_helper.py       # build a diagram in memory (+ tmp_test_outputs helpers)
    expr.py               # expression eval + structural value comparison
    generate_snapshots.py     # regenerate snapshots/ from examples/
    generate_expressions.py   # refresh expected values in expression_tests.json

  examples/               # ── neutral inputs ──  source diagrams (+ data/)
    hand-crafted/  extracted-from-docs/  uses-external-data/
  guide_figures/          # ── neutral inputs ── Guide diagrams (code/ + images/)
  snapshots/              # ── reference snapshots ── mirror the input tree by name
    examples/<category>/          snapshots for examples/  (+ annotation .xml)
    guide_figures/code/           snapshots for Guide figures that build (~126)
    manifest.json                 what built vs skipped, per corpus/category
  expressions/
    expression_tests.json # ── reference snapshot ── expression corpus
  README.md
```

`helpers/` is importable during a test run via `pythonpath = ["tests"]` in
`pyproject.toml`, so no `conftest.py` is needed. Ephemeral output (the CLI smoke
test's build products) goes to `../tmp_test_outputs/`, which is gitignored.

## Running

```bash
poetry install --all-extras          # needs pycairo; MathJax runs via node
poetry run pytest                    # everything
poetry run pytest -m "not slow"      # skip the ~138 guide-figure builds
poetry run pytest tests/test_snapshots.py -v
```

Snapshot and expression tests build in memory and write nothing. The snapshot and
guide-figure tests require MathJax (node) and libcairo, matching a normal
`prefig build`.

## Updating a failing snapshot

A snapshot test fails when the built SVG no longer matches the committed snapshot.
First decide whether the change is a **regression** (fix the code) or
**intentional** (accept the new output). The failure message prints the exact
command to accept it. To update one snapshot, run its node id with
`UPDATE_SNAPSHOTS=1` — the test rewrites that snapshot in place instead of asserting:

```bash
UPDATE_SNAPSHOTS=1 poetry run pytest \
    "tests/test_snapshots.py::test_matches_snapshot[examples/hand-crafted/tangent]"
```

The id is `<corpus>/<category>/<stem>` (shown in pytest output as
`test_matches_snapshot[...]`). Update a group with `-k <substring>`, or every
snapshot by running the whole file with `UPDATE_SNAPSHOTS=1`. Always
`git diff tests/snapshots` and eyeball the change before committing.

## Regenerating all snapshots

To rebuild the entire corpus — picking up **new** source files and refreshing the
annotation `.xml` snapshots and `manifest.json`, which `UPDATE_SNAPSHOTS` does not —
run the generators:

```bash
poetry run python tests/helpers/generate_snapshots.py     # rewrites tests/snapshots/
poetry run python tests/helpers/generate_expressions.py   # rewrites tests/expressions/…
```

The usual reason a snapshot drifts *without* a code change is a different
node-MathJax version producing different glyph geometry; the comparator's numeric
tolerance absorbs float noise but not structural changes.

## CI: snapshot report on pull requests

The `snapshot report` workflow (`.github/workflows/snapshot-report.yml`) builds
the examples corpus on every PR that touches `prefig/`, `tests/`, or the Python
project files, using `tests/helpers/snapshot_report.py`. If any example renders
differently from its reference snapshot, it posts (and thereafter updates) a
single sticky PR comment listing the differences per example — and when fewer
than 8 examples differ, the comment embeds side-by-side PNG renders of the
snapshot vs what the PR produces. The check also fails so the divergence is
visible in the PR status; accept intentional changes with `UPDATE_SNAPSHOTS=1`
(above) and push, and the comment flips to ✅.

Mechanics worth knowing:

- GitHub comments can only embed hosted images, so the renders are force-pushed
  to a scratch branch `snapshot-diff/pr-<N>` and referenced by commit SHA via
  `raw.githubusercontent.com`; the branch is deleted when the PR closes.
- On PRs from forks the token is read-only, so the comment/branch steps are
  skipped — the same report still appears in the job summary and as the
  `snapshot-report` workflow artifact (with the PNGs).
- The runner needs the same toolchain as a local run: cairo, `rsvg-convert`,
  and MathJax (`prefig init`); the workflow installs them.

Run the report locally:

```bash
poetry run python tests/helpers/snapshot_report.py --out-dir tmp_test_outputs/report
cat tmp_test_outputs/report/comment.md
```

## Notes

- The comparator ignores volatile attributes (`id`, `clip-path`, `href`) and
  compares numbers within a relative tolerance (default `1e-2`), mirroring the
  Rust `svg_compare` module.
- `snapshots/` mirrors the input tree: a snapshot at
  `snapshots/<corpus>/<category>/<stem>.svg` is the expected output of
  `tests/<corpus>/<category>/<stem>.xml`. Guide-figure comparisons are marked
  `slow` (there are ~126); the curated `examples` snapshots always run. The few
  Guide figures that build to trivial output in isolation get no snapshot and are
  instead smoke-tested (build-without-crash) by `test_guide_figures.py`.
- `examples/` categories: `hand-crafted` (bundled with the package),
  `extracted-from-docs` (diagrams from the PreFigure Guide),
  `uses-external-data` (load CSV/images via `<read>`/`<image>`;
  `<histogram>`/delta-forced ODEs). `guide_figures/` categories: `code` (Guide
  source snippets), `images` (Guide asset diagrams).
