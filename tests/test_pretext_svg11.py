"""Pretext-mode SVG 1.1 compliance for double-arrow figures.

The snapshot corpus builds in the CLI mode (``pf_cli``); pretext mode
additionally writes an SVG 1.1-compliant conversion (``<stem>-11.svg``).
The conversion matters exactly when a line/curve has arrowheads at *both*
ends: the backward arrowhead relies on ``orient="auto-start-reverse"``, an
SVG 2 feature, which SVG 1.1 renderers ignore (the arrowhead points the
wrong way). The conversion instead materializes a rotated ``…-start``
marker and rewrites ``marker-start`` references; it also moves ``<use
href>`` to the ``xlink:href`` form SVG 1.1 requires.

This module builds a few representative double-arrow examples through the
real pretext file-writing pipeline and checks the ``-11.svg`` output:

* no ``auto-start-reverse`` anywhere (the SVG 2 feature is gone),
* every ``marker-start/mid/end`` reference resolves to a defined marker,
* every ``<use>`` carries ``xlink:href`` rather than SVG 2's ``href``,
* and (sanity) the regular output *does* use ``auto-start-reverse``, so
  these examples genuinely exercise the conversion.
"""

import re
from pathlib import Path

import lxml.etree as ET
import pytest

from helpers.build_helper import build_diagram_files, temp_workdir

TESTS_DIR = Path(__file__).resolve().parent

# Double-arrow (arrows="2") examples covering <line> and <path>/curve cases.
DOUBLE_ARROW_EXAMPLES = [
    "extracted_from_docs/arrow_properties",   # double-arrow lines
    "extracted_from_docs/paths",              # double-arrow path (curves)
    "extracted_from_docs/arrow_angle_def",    # line with custom arrow angles
]

XLINK_HREF = "{http://www.w3.org/1999/xlink}href"
MARKER_REF = re.compile(r"url\(#([^)]+)\)")


@pytest.fixture(scope="module", params=DOUBLE_ARROW_EXAMPLES, ids=lambda p: p)
def pretext_outputs(request):
    """Build one example in pretext mode; yield (name, svg2_tree, svg11_tree)."""
    rel = request.param
    source = TESTS_DIR / "examples" / Path(rel).with_suffix(".xml")
    name = source.stem
    with temp_workdir(f"pretext_svg11/{name}"):
        out_dir = build_diagram_files(source, environment="pretext")
        assert out_dir is not None, f"{name} did not build"
        svg2_path = out_dir / f"{name}.svg"
        svg11_path = out_dir / f"{name}-11.svg"
        assert svg2_path.exists(), f"pretext build wrote no {svg2_path.name}"
        assert svg11_path.exists(), (
            f"pretext build wrote no {svg11_path.name}; "
            "the SVG 1.1 conversion did not run"
        )
        svg2 = ET.parse(str(svg2_path)).getroot()
        svg11 = ET.parse(str(svg11_path)).getroot()
    return name, svg2, svg11


def _local(tag):
    return tag.rsplit("}", 1)[-1] if isinstance(tag, str) else tag


def test_regular_output_uses_svg2_arrowheads(pretext_outputs):
    """Sanity: these examples rely on auto-start-reverse in the SVG 2 output."""
    name, svg2, _svg11 = pretext_outputs
    orients = {m.get("orient") for m in svg2.iter() if _local(m.tag) == "marker"}
    assert "auto-start-reverse" in orients, (
        f"{name} has no auto-start-reverse marker in its regular output; "
        "it does not exercise the SVG 1.1 conversion (pick another example)"
    )
    assert any(el.get("marker-start") for el in svg2.iter()), (
        f"{name} has no marker-start (backward arrowhead) in its regular output"
    )


def test_svg11_has_no_svg2_marker_orientation(pretext_outputs):
    name, _svg2, svg11 = pretext_outputs
    offenders = [
        m.get("id")
        for m in svg11.iter()
        if _local(m.tag) == "marker" and m.get("orient") == "auto-start-reverse"
    ]
    assert not offenders, (
        f"{name}-11.svg still uses SVG 2's auto-start-reverse on markers: {offenders}"
    )


def test_svg11_marker_references_resolve(pretext_outputs):
    """marker-start/mid/end all point at markers that exist in the -11 output."""
    name, _svg2, svg11 = pretext_outputs
    marker_ids = {m.get("id") for m in svg11.iter() if _local(m.tag) == "marker"}
    unresolved = []
    starts_checked = 0
    for el in svg11.iter():
        for attr in ("marker-start", "marker-mid", "marker-end"):
            value = el.get(attr)
            if not value:
                continue
            match = MARKER_REF.fullmatch(value.strip())
            if match is None or match.group(1) not in marker_ids:
                unresolved.append((attr, value))
            if attr == "marker-start":
                starts_checked += 1
                # the backward arrowhead must use the materialized start marker
                assert "arrow-head-start" in value, (
                    f"{name}-11.svg marker-start still references the end "
                    f"marker: {value}"
                )
    assert starts_checked > 0, f"{name}-11.svg lost its marker-start arrowheads"
    assert not unresolved, f"{name}-11.svg has dangling marker references: {unresolved}"


def test_svg11_uses_xlink_href(pretext_outputs):
    """SVG 1.1 <use> requires xlink:href; SVG 2's plain href must be gone."""
    name, _svg2, svg11 = pretext_outputs
    uses = [el for el in svg11.iter() if _local(el.tag) == "use"]
    plain = [el for el in uses if el.get("href")]
    assert not plain, f"{name}-11.svg has <use href=...>; SVG 1.1 needs xlink:href"
    # every <use> must still reference something, now via xlink
    for el in uses:
        assert el.get(XLINK_HREF), f"{name}-11.svg has a <use> with no xlink:href"
