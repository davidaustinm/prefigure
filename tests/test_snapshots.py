"""Snapshot (reference-render) regression tests for the Python renderer.

Builds every source that has a committed reference snapshot under
``tests/snapshots/`` and compares the SVG within a numeric tolerance. A snapshot
at ``snapshots/examples/<category>/<stem>.svg`` corresponds to the source
``tests/examples/<category>/<stem>.xml``. Sources whose build produces
annotations also have a ``<stem>.xml`` annotation snapshot next to the SVG one;
those are compared too, and every annotation id is checked to resolve to an
element id in the built SVG (that resolution is what drives diagcess
highlighting). Everything is built exactly the way ``prefig build`` does: the
``pf_cli`` environment, with any ``pf_publication.xml`` in the category applied.

This locks the current Python output; it is also the corpus the Rust port is
checked against.

To accept an intentional rendering change, rewrite just the snapshots a
selection covers by setting ``UPDATE_SNAPSHOTS=1`` — one exact snapshot:

    UPDATE_SNAPSHOTS=1 poetry run pytest \\
        "tests/test_snapshots.py::test_matches_snapshot[examples/hand_crafted/tangent]"

or a group (``-k`` is a substring match), or all of them (no selector). Then
review ``git diff tests/snapshots`` before committing. To (re)generate every
snapshot — including new sources — run ``tests/helpers/generate_snapshots.py``
instead.
"""

import functools
import os
from pathlib import Path

import lxml.etree as ET
import pytest

from helpers.build_helper import build_diagram, pushd
from helpers.compare import DEFAULT_TOL, compare_svgs

TESTS_DIR = Path(__file__).resolve().parent
SNAPSHOTS_DIR = TESTS_DIR / "snapshots"

# When set, a comparison instead rewrites its snapshot in place (jest `-u` style).
UPDATE_SNAPSHOTS = os.environ.get("UPDATE_SNAPSHOTS", "") not in ("", "0", "false")


@functools.lru_cache(maxsize=None)
def _build(source_path_str):
    """Build once per source; the SVG and annotation tests share the result."""
    source = Path(source_path_str)
    with pushd(source.parent):
        return build_diagram(source.name)


def _cases():
    cases = []
    for snapshot in sorted(SNAPSHOTS_DIR.rglob("*.svg")):
        rel = snapshot.relative_to(SNAPSHOTS_DIR)        # e.g. examples/hand_crafted/tangent.svg
        source = TESTS_DIR / rel.with_suffix(".xml")     # tests/examples/hand_crafted/tangent.xml
        cases.append(pytest.param(source, snapshot, id=str(rel.with_suffix(""))))
    return cases


def _annotation_cases():
    """One case per committed annotation snapshot (sources that annotate)."""
    cases = []
    for ann_snapshot in sorted(SNAPSHOTS_DIR.rglob("*.xml")):
        if ann_snapshot.name == "manifest.json":
            continue
        rel = ann_snapshot.relative_to(SNAPSHOTS_DIR)
        source = TESTS_DIR / rel.with_suffix(".xml")
        case_id = str(rel.with_suffix(""))
        cases.append(pytest.param(source, ann_snapshot, id=case_id))
    return cases


def test_snapshot_corpus_present():
    # 8 hand_crafted + ~155 extracted_from_docs + 3 uses_external_data
    assert len(list((SNAPSHOTS_DIR / "examples").rglob("*.svg"))) >= 150
    # a healthy chunk of the corpus carries annotations
    annotated = [x for x in SNAPSHOTS_DIR.rglob("*.xml") if x.name != "manifest.json"]
    assert len(annotated) >= 15



@pytest.mark.parametrize("source_path,snapshot_path", _cases())
def test_matches_snapshot(source_path, snapshot_path):
    assert source_path.exists(), f"snapshot {snapshot_path.name} has no source {source_path}"

    result = _build(str(source_path))
    assert result is not None, f"{source_path.name} produced no diagram"
    svg, annotations = result

    if UPDATE_SNAPSHOTS:
        snapshot_path.write_text(svg)
        return

    # coverage guard: a build that produces annotations must have them snapshotted
    if annotations:
        assert snapshot_path.with_suffix(".xml").exists(), (
            f"{source_path.name} produces annotations but has no annotation snapshot; "
            "run tests/helpers/generate_snapshots.py"
        )

    diffs = compare_svgs(svg, snapshot_path.read_text(), DEFAULT_TOL)
    snapshot_id = str(snapshot_path.relative_to(SNAPSHOTS_DIR).with_suffix(""))
    assert not diffs, (
        f"{source_path.name} differs from its snapshot:\n"
        + "\n".join(diffs)
        + "\n\nIf this change is intentional, update just this snapshot with:\n"
        + f'  UPDATE_SNAPSHOTS=1 poetry run pytest "tests/test_snapshots.py::test_matches_snapshot[{snapshot_id}]"\n'
        + f"  (snapshot: tests/snapshots/{snapshot_id}.svg)"
    )


@pytest.mark.parametrize("source_path,ann_snapshot_path", _annotation_cases())
def test_annotations_match_snapshot(source_path, ann_snapshot_path):
    """The built annotations XML matches its committed snapshot."""
    result = _build(str(source_path))
    assert result is not None, f"{source_path.name} produced no diagram"
    _svg, annotations = result
    assert annotations, f"{source_path.name} no longer produces annotations"

    if UPDATE_SNAPSHOTS:
        ann_snapshot_path.write_text(annotations)
        return

    diffs = compare_svgs(annotations, ann_snapshot_path.read_text(), DEFAULT_TOL)
    snapshot_id = str(ann_snapshot_path.relative_to(SNAPSHOTS_DIR).with_suffix(""))
    assert not diffs, (
        f"{source_path.name} annotations differ from their snapshot:\n"
        + "\n".join(diffs)
        + "\n\nIf this change is intentional, update with:\n"
        + f'  UPDATE_SNAPSHOTS=1 poetry run pytest "tests/test_snapshots.py::test_annotations_match_snapshot[{snapshot_id}]"'
    )


@pytest.mark.parametrize("source_path,ann_snapshot_path", _annotation_cases())
def test_annotation_ids_resolve(source_path, ann_snapshot_path):
    """Every leaf annotation id points at an element id present in the built SVG.

    diagcess drives highlighting by looking up the annotation's id among the
    SVG element ids, so an unresolved id is a silently broken annotation.
    Grouping annotations (those with a <children> element) need not resolve to
    an SVG element — only their leaf descendants need to.
    """
    result = _build(str(source_path))
    assert result is not None, f"{source_path.name} produced no diagram"
    svg, annotations = result
    assert annotations, f"{source_path.name} no longer produces annotations"

    svg_ids = {
        el.get("id")
        for el in ET.fromstring(svg.encode("utf-8")).iter()
        if el.get("id")
    }
    leaf_ann_ids = [
        a.get("id")
        for a in ET.fromstring(annotations.encode("utf-8")).iter("annotation")
        if a.get("id") and a.find("children") is None
    ]
    assert leaf_ann_ids, f"{source_path.name} annotations contain no leaf ids"

    unresolved = [i for i in leaf_ann_ids if i not in svg_ids]
    assert not unresolved, (
        f"{source_path.name}: {len(unresolved)}/{len(leaf_ann_ids)} annotation ids "
        f"do not match any SVG element id: {unresolved}"
    )
