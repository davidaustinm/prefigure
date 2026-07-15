# PreFigure test suite

Python tests for the reference implementation, organized so the **test assets
are language-neutral**: the example diagrams, snapshot corpus, and expression
corpus live in plain folders here that the Python tests read today and the Rust
port's tests will be re-pointed at later (from the *same* paths). The only
Python-specific pieces are the `test_*.py` entry points and `helpers/`.

## Layout

```
tests/
  test_snapshots.py                    # build each example, compare SVG + annotation
                                       # snapshots, check annotation ids resolve
  test_expressions.py                  # replay the expression corpus through user_namespace
  test_examples_without_snapshots.py   # the few unsnapshotted examples build w/o crashing
  test_pretext_svg11.py                # pretext mode's SVG 1.1 conversion (double arrows)
  test_prefigure.py                    # end-to-end `prefig` CLI smoke test
  helpers/                # all Python-side support code
    compare.py            # tolerance SVG structural comparator
    build_helper.py       # build a diagram in memory (+ tmp_test_outputs helpers)
    expr.py               # expression eval + structural value comparison
    generate_snapshots.py     # regenerate snapshots/ from examples/
    generate_expressions.py   # refresh expected values in expression_tests.json

  examples/               # ── neutral inputs ── all source diagrams (+ data/)
    hand_crafted/          # bundled with the package (8)
    extracted_from_docs/   # from the PreFigure Guide: its examples + code listings (~155)
    uses_external_data/    # load CSV/images via <read>/<image> (3)
                           # (data categories carry a pf_publication.xml with
                           #  <directories data="data"/>, like a CLI project)
  snapshots/              # ── reference snapshots ── mirror the input tree by name
    examples/<category>/          snapshots for everything that builds (+ annotation .xml)
    manifest.json                 what built vs skipped, per category
  expressions/
    expression_tests.json # ── reference snapshot ── expression corpus
  README.md
```

Everything is built exactly the way `prefig build` builds it: the **`pf_cli`**
environment, with any `pf_publication.xml` found from the working directory
upward applied (that's how the data categories point `<read>`/`<image>` at
`data/`). The one exception is `test_pretext_svg11.py`, which deliberately
builds a few double-arrow figures in the **pretext** environment: pretext
additionally emits an SVG 1.1-compliant `<stem>-11.svg` (backward arrowheads
can't use SVG 2's `orient="auto-start-reverse"` there), and the test verifies
that conversion.

`helpers/` is importable during a test run via `pythonpath = ["tests"]` in
`pyproject.toml`, so no `conftest.py` is needed. Ephemeral output (the CLI smoke
test's build products) goes to `../tmp_test_outputs/`, which is gitignored.

## Running

```bash
poetry install --all-extras          # needs pycairo; MathJax runs via node
poetry run pytest                    # everything (~45s)
poetry run pytest tests/test_snapshots.py -v
```

Snapshot and expression tests build in memory and write nothing. Building
diagrams requires MathJax (node) and libcairo, matching a normal `prefig build`.

## Updating a failing snapshot

A snapshot test fails when the built SVG no longer matches the committed snapshot.
First decide whether the change is a **regression** (fix the code) or
**intentional** (accept the new output). The failure message prints the exact
command to accept it. To update one snapshot, run its node id with
`UPDATE_SNAPSHOTS=1` — the test rewrites that snapshot in place instead of asserting:

```bash
UPDATE_SNAPSHOTS=1 poetry run pytest \
    "tests/test_snapshots.py::test_matches_snapshot[examples/hand_crafted/tangent]"
```

The id is `examples/<category>/<stem>` (shown in pytest output as
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
  `raw.githubusercontent.com`; `snapshot-cleanup.yml` deletes the branch when
  the PR closes.
- On PRs from forks the `pull_request` token is read-only, so a follow-up
  workflow (`snapshot-comment.yml`, triggered by `workflow_run` in the base
  repository with write permissions) downloads the report artifact and posts
  the comment instead. It never executes PR code — it only moves data. Like
  all `workflow_run` workflows it runs from the default branch, so it takes
  effect only after being merged. The report also always appears in the job
  summary and as the `snapshot-report` artifact.
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
  `snapshots/examples/<category>/<stem>.svg` is the expected output of
  `tests/examples/<category>/<stem>.xml`. The few sources that build to trivial
  output in isolation get no snapshot and are instead smoke-tested
  (build-without-crash) by `test_examples_without_snapshots.py`.
- Sources whose build produces annotations also have an annotation snapshot
  (`<stem>.xml` next to the SVG one). `test_snapshots.py` compares those and
  additionally checks that **every annotation id resolves to an SVG element
  id** — that lookup is what drives diagcess highlighting, so an unresolved id
  is a silently broken annotation. Examples that fail this today (stale refs,
  or the `<annotation id=...>` syntax that prefigure does not prefix) are
  recorded with reasons in [`KNOWN_UNRESOLVED.yml`](KNOWN_UNRESOLVED.yml) as
  strict xfails. When an issue is resolved, its test XPASSes and fails the
  suite until the corresponding line is deleted from that file — so resolving
  an issue means editing just that file.
- Categories: `hand_crafted` (bundled with the package), `extracted_from_docs`
  (everything vendored from the PreFigure Guide — its polished examples plus
  every code listing in its text), `uses_external_data` (load CSV/images via
  `<read>`/`<image>`; `<histogram>`/delta-forced ODEs). Same-named files across
  categories (e.g. `tangent.xml`) are intentionally different variants, not
  duplicates.
