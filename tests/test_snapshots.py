"""Snapshot (reference-render) regression tests for the Python renderer.

Builds every source that has a committed reference snapshot under
``tests/snapshots/`` and compares the SVG within a numeric tolerance. A snapshot
at ``snapshots/examples/<category>/<stem>.svg`` corresponds to the source
``tests/examples/<category>/<stem>.xml``. This locks the current Python output;
it is also the corpus the Rust port is checked against.

To accept an intentional rendering change, rewrite just the snapshots a
selection covers by setting ``UPDATE_SNAPSHOTS=1`` — one exact snapshot:

    UPDATE_SNAPSHOTS=1 poetry run pytest \\
        "tests/test_snapshots.py::test_matches_snapshot[examples/hand_crafted/tangent]"

or a group (``-k`` is a substring match), or all of them (no selector). Then
review ``git diff tests/snapshots`` before committing. To (re)generate every
snapshot — including new sources and annotation files — run
``tests/helpers/generate_snapshots.py`` instead.
"""

import os
from pathlib import Path

import pytest

from helpers.build_helper import build_diagram, pushd
from helpers.compare import DEFAULT_TOL, compare_svgs

TESTS_DIR = Path(__file__).resolve().parent
SNAPSHOTS_DIR = TESTS_DIR / "snapshots"

# When set, a comparison instead rewrites its snapshot in place (jest `-u` style).
UPDATE_SNAPSHOTS = os.environ.get("UPDATE_SNAPSHOTS", "") not in ("", "0", "false")


def _cases():
    cases = []
    for snapshot in sorted(SNAPSHOTS_DIR.rglob("*.svg")):
        rel = snapshot.relative_to(SNAPSHOTS_DIR)        # e.g. examples/hand_crafted/tangent.svg
        source = TESTS_DIR / rel.with_suffix(".xml")     # tests/examples/hand_crafted/tangent.xml
        cases.append(pytest.param(source, snapshot, id=str(rel.with_suffix(""))))
    return cases


def test_snapshot_corpus_present():
    # 8 hand_crafted + ~155 extracted_from_docs + 3 uses_external_data
    assert len(list((SNAPSHOTS_DIR / "examples").rglob("*.svg"))) >= 160


@pytest.mark.parametrize("source_path,snapshot_path", _cases())
def test_matches_snapshot(source_path, snapshot_path):
    assert source_path.exists(), f"snapshot {snapshot_path.name} has no source {source_path}"

    # <read>/<image> resolve data/ relative to the working directory.
    with pushd(source_path.parent):
        result = build_diagram(source_path.name)

    assert result is not None, f"{source_path.name} produced no diagram"
    svg, _annotations = result

    if UPDATE_SNAPSHOTS:
        snapshot_path.write_text(svg)
        return

    diffs = compare_svgs(svg, snapshot_path.read_text(), DEFAULT_TOL)
    snapshot_id = str(snapshot_path.relative_to(SNAPSHOTS_DIR).with_suffix(""))
    assert not diffs, (
        f"{source_path.name} differs from its snapshot:\n"
        + "\n".join(diffs)
        + "\n\nIf this change is intentional, update just this snapshot with:\n"
        + f'  UPDATE_SNAPSHOTS=1 poetry run pytest "tests/test_snapshots.py::test_matches_snapshot[{snapshot_id}]"\n'
        + f"  (snapshot: tests/snapshots/{snapshot_id}.svg)"
    )
