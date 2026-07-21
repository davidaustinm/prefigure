"""SVG 1.1 downgrade regression tests (pf_cli, snapshot-based).

Examples that use SVG 2 features (auto-start-reverse for backward arrowheads,
or <use href> for symbol reuse) are detected automatically by scanning the
committed SVG 2 snapshots. For each, the SVG 1.1 output is produced via
pf_cli and compared against a committed -11.svg snapshot placed next to the
SVG 2 one. Regressions in the conversion are caught the same way as any other
rendering change.

To create -11.svg snapshots for newly detected examples, or to regenerate
them after an intentional conversion change:
    UPDATE_SNAPSHOTS=1 poetry run pytest tests/test_pretext_svg11.py
"""

import os
from pathlib import Path

import lxml.etree as ET
import pytest

from helpers.build_helper import build_diagram, pushd
from helpers.compare import DEFAULT_TOL, compare_svgs

TESTS_DIR = Path(__file__).resolve().parent
SNAPSHOTS_DIR = TESTS_DIR / "snapshots"
UPDATE_SNAPSHOTS = os.environ.get("UPDATE_SNAPSHOTS", "") not in ("", "0", "false")

XLINK_HREF = "{http://www.w3.org/1999/xlink}href"


def _svg2_feature_examples():
    """Committed SVG 2 snapshots that contain features requiring SVG 1.1 downgrade.

    Two SVG 2 features trigger downgrade:
    - marker-start: a backward arrowhead using auto-start-reverse
    - <use href=: symbol reuse via SVG 2 href (must become xlink:href)
    """
    cases = []
    for snapshot in sorted((SNAPSHOTS_DIR / "examples").rglob("*.svg")):
        if snapshot.stem.endswith("-11"):
            continue
        content = snapshot.read_text()
        has_backward_arrow = 'marker-start' in content
        has_use_href = ('<use' in content or '<image' in content) and ' href=' in content
        if not (has_backward_arrow or has_use_href):
            continue
        rel = str(snapshot.relative_to(SNAPSHOTS_DIR / "examples").with_suffix(""))
        cases.append(pytest.param(rel, id=rel))
    return cases


@pytest.fixture(scope="module", params=_svg2_feature_examples())
def svg_outputs(request):
    """Build one example via pf_cli; return (rel, svg2_str, svg11_str)."""
    rel = request.param
    source = TESTS_DIR / "examples" / Path(rel).with_suffix(".xml")
    with pushd(source.parent):
        svg2, _ = build_diagram(source.name)
        svg11, _ = build_diagram(source.name, format="svg11")
    return rel, svg2, svg11


def _local(tag):
    return tag.rsplit("}", 1)[-1] if isinstance(tag, str) else tag


def test_regular_output_uses_svg2_features(svg_outputs):
    """Sanity: the built SVG 2 output still contains the SVG 2 feature detected in its snapshot."""
    rel, svg2, _ = svg_outputs
    root = ET.fromstring(svg2.encode())
    has_auto_start_reverse = any(
        m.get("orient") == "auto-start-reverse"
        for m in root.iter()
        if _local(m.tag) == "marker"
    )
    has_use_href = any(
        el.get("href") is not None
        for el in root.iter()
        if _local(el.tag) in ("use", "image")
    )
    assert has_auto_start_reverse or has_use_href, (
        f"{rel} no longer contains auto-start-reverse or <use href> in its built output; "
        "remove it from the SVG 1.1 test corpus or pick a different example"
    )


def test_svg11_matches_snapshot(svg_outputs):
    """The SVG 1.1 output matches its committed snapshot."""
    rel, _, svg11 = svg_outputs
    snapshot = SNAPSHOTS_DIR / f"examples/{rel}-11.svg"
    if UPDATE_SNAPSHOTS:
        snapshot.parent.mkdir(parents=True, exist_ok=True)
        snapshot.write_text(svg11)
        return
    assert snapshot.exists(), (
        f"no SVG 1.1 snapshot at {snapshot.relative_to(TESTS_DIR)}; "
        "run UPDATE_SNAPSHOTS=1 to create it"
    )
    diffs = compare_svgs(svg11, snapshot.read_text(), DEFAULT_TOL)
    snapshot_id = str(snapshot.relative_to(SNAPSHOTS_DIR).with_suffix(""))
    assert not diffs, (
        f"{rel} SVG 1.1 output differs from its snapshot:\n"
        + "\n".join(diffs)
        + "\n\nIf intentional, update with:\n"
        + f'  UPDATE_SNAPSHOTS=1 poetry run pytest "tests/test_pretext_svg11.py::test_svg11_matches_snapshot[{rel}]"'
    )
