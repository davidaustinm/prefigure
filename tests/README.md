# PreFigure test suite

Python tests for the reference implementation, organized so the **test assets
are language-neutral**: the snapshot corpus, example diagrams, guide figures, and
expression corpus live in plain folders here that the Python tests read today and
the Rust port's tests will be re-pointed at later (from the *same* paths). The
only Python-specific pieces are the `test_*.py` entry points and `_harness/`.

## Layout

```
tests/
  test_snapshots.py       # build each example, compare to a committed SVG snapshot
  test_expressions.py     # replay the expression corpus through user_namespace
  test_guide_figures.py   # build every Guide figure without crashing (marked slow)
  test_prefigure.py       # end-to-end `prefig` CLI smoke test
  conftest.py             # fixtures + makes `_harness` importable
  _harness/
    compare.py            # tolerance SVG structural comparator
    build_helper.py       # build a diagram to an in-memory SVG (no disk writes)
    expr.py               # expression eval + structural value comparison

  examples/               # ── neutral inputs ──  source diagrams (+ data/)
    repo/  docs/  synth/
  snapshots/              # ── neutral goldens ── expected SVGs (+ annotation .xml)
    repo/  docs/  synth/  manifest.json
  guide_figures/          # ── neutral inputs ── Guide diagrams (code/ + images/)
  expressions/
    expression_tests.json # ── neutral golden ── expression corpus

  tools/
    generate_snapshots.py     # rebuild snapshots/ from examples/ (Python)
    generate_expressions.py   # refresh expected values in expression_tests.json
    synthetic_examples/       # sources for the `synth` category
```

Ephemeral output (the CLI smoke test's build products) goes to
`../tmp_test_outputs/`, which is gitignored.

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

## Regenerating the goldens

The snapshots are the current Python output, so the suite is a regression lock.
After an **intentional** rendering or evaluator change, re-baseline:

```bash
poetry run python tests/tools/generate_snapshots.py     # rewrites tests/snapshots/
poetry run python tests/tools/generate_expressions.py   # rewrites tests/expressions/…
```

Then review the diff before committing. The usual reason a snapshot drifts
without a code change is a different node-MathJax version producing different
glyph geometry; the comparator's numeric tolerance absorbs float noise but not
structural changes.

## Notes

- The comparator ignores volatile attributes (`id`, `clip-path`, `href`) and
  compares numbers within a relative tolerance (default `1e-2`), mirroring the
  Rust `svg_compare` module.
- Categories: `repo` (bundled examples), `docs` (PreFigure Guide diagrams),
  `synth` (synthetic diagrams exercising `<read>`/`<histogram>`/delta-forced ODEs).
