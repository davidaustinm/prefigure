"""Smoke test: every Guide figure must build without crashing.

Mirrors ``rust/prefig-core/tests/guide_figures.rs``. The ~138 diagrams vendored
from the PreFigure Guide are built with the real Python pipeline; a graceful
"no diagram" result is allowed (some are fragments), but an uncaught exception
fails. This exercises far more of the renderer than the curated snapshot set.

Building uses MathJax (node), so the parametrized build is marked ``slow`` and
can be skipped with ``-m "not slow"``.
"""

from pathlib import Path

import pytest

from _harness.build_helper import build_diagram, pushd

GUIDE_DIR = Path(__file__).resolve().parent / "guide_figures"


def _figures():
    return sorted(GUIDE_DIR.rglob("*.xml"))


def test_enough_guide_figures():
    assert len(_figures()) >= 130


@pytest.mark.slow
@pytest.mark.parametrize("xml_path", _figures(), ids=lambda p: f"{p.parent.name}/{p.name}")
def test_guide_figure_builds(xml_path):
    with pushd(xml_path.parent):
        try:
            build_diagram(xml_path.name)  # None is acceptable; a raise is not
        except Exception as exc:  # noqa: BLE001
            pytest.fail(f"{xml_path.name} raised {exc!r}")
