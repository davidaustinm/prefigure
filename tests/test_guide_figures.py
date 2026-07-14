"""Smoke coverage for Guide figures that have no snapshot.

Most Guide figures are golden-tested by ``test_snapshots.py``. A handful build
to empty/trivial output in isolation (they need external data or are wrapper
diagrams), so they get no golden — this module still builds them to prove they
do not crash, so every Guide figure stays exercised. Mirrors the intent of
``rust/prefig-core/tests/guide_figures.rs``.
"""

from pathlib import Path

import pytest

from helpers.build_helper import build_diagram, pushd

TESTS_DIR = Path(__file__).resolve().parent
GUIDE_DIR = TESTS_DIR / "guide_figures"
GUIDE_SNAPSHOTS = TESTS_DIR / "snapshots" / "guide_figures"


def _all_figures():
    return sorted(GUIDE_DIR.rglob("*.xml"))


def _figures_without_golden():
    return [
        xml
        for xml in _all_figures()
        if not (GUIDE_SNAPSHOTS / xml.parent.name / f"{xml.stem}.svg").exists()
    ]


def test_enough_guide_figures():
    assert len(_all_figures()) >= 130


@pytest.mark.slow
@pytest.mark.parametrize(
    "xml_path", _figures_without_golden(), ids=lambda p: f"{p.parent.name}/{p.name}"
)
def test_ungolden_guide_figure_builds(xml_path):
    # No golden (builds to trivial output in isolation); just ensure no crash.
    with pushd(xml_path.parent):
        try:
            build_diagram(xml_path.name)
        except Exception as exc:  # noqa: BLE001
            pytest.fail(f"{xml_path.name} raised {exc!r}")
