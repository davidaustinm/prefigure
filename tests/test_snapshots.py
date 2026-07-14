"""Snapshot (golden-file) regression tests for the Python renderer.

Builds every example diagram with the Python package (``pretext`` environment,
in memory) and compares the SVG to the committed snapshot under
``tests/snapshots/`` within a numeric tolerance. This locks the current Python
output; it is also the corpus the Rust port is checked against.

Regenerate the snapshots with ``tests/tools/generate_snapshots.py`` after an
intentional rendering change (see tests/README.md).
"""

from pathlib import Path

import pytest

from _harness.build_helper import build_diagram, pushd
from _harness.compare import DEFAULT_TOL, compare_svgs

TESTS_DIR = Path(__file__).resolve().parent
EXAMPLES_DIR = TESTS_DIR / "examples"
SNAPSHOTS_DIR = TESTS_DIR / "snapshots"
CATEGORIES = ("repo", "docs", "synth")


def _cases():
    cases = []
    for category in CATEGORIES:
        cat_dir = EXAMPLES_DIR / category
        if not cat_dir.is_dir():
            continue
        for xml in sorted(cat_dir.glob("*.xml")):
            snapshot = SNAPSHOTS_DIR / category / f"{xml.stem}.svg"
            cases.append(pytest.param(xml, snapshot, id=f"{category}/{xml.stem}"))
    return cases


def test_snapshot_corpus_present():
    # 8 repo + 29 docs + 3 synth = 40 committed example diagrams.
    assert len(_cases()) >= 40


@pytest.mark.parametrize("xml_path,snapshot_path", _cases())
def test_matches_snapshot(xml_path, snapshot_path):
    assert snapshot_path.exists(), f"missing snapshot {snapshot_path}"
    expected = snapshot_path.read_text()

    # <read>/<image> resolve data/ relative to the working directory.
    with pushd(xml_path.parent):
        result = build_diagram(xml_path.name)

    assert result is not None, f"{xml_path.name} produced no diagram"
    svg, _annotations = result
    diffs = compare_svgs(svg, expected, DEFAULT_TOL)
    assert not diffs, f"{xml_path.name} differs from snapshot:\n" + "\n".join(diffs)
