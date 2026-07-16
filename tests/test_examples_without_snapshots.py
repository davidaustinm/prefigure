"""Smoke coverage for example sources that have no reference snapshot.

Most of ``tests/examples/`` is snapshot-tested by ``test_snapshots.py``. A
few sources build to empty/trivial output in isolation, so they get no
snapshot — this module still builds them to prove they do not crash, so every
committed example stays exercised.
"""

from pathlib import Path

import pytest

from helpers.build_helper import build_diagram, pushd

TESTS_DIR = Path(__file__).resolve().parent
EXAMPLES_DIR = TESTS_DIR / "examples"
SNAPSHOTS_DIR = TESTS_DIR / "snapshots"


def _all_examples():
    return sorted(
        xml for xml in EXAMPLES_DIR.rglob("*.xml")
        if xml.name != "pf_publication.xml"   # publication files, not diagrams
    )


def _examples_without_snapshot():
    return [
        xml
        for xml in _all_examples()
        if not (SNAPSHOTS_DIR / xml.relative_to(TESTS_DIR)).with_suffix(".svg").exists()
    ]


def test_enough_examples():
    assert len(_all_examples()) >= 150


@pytest.mark.parametrize(
    "xml_path",
    _examples_without_snapshot(),
    ids=lambda p: f"{p.parent.name}/{p.name}",
)
def test_example_without_snapshot_builds(xml_path):
    # No snapshot (builds to trivial output in isolation); just ensure no crash.
    with pushd(xml_path.parent):
        try:
            build_diagram(xml_path.name)
        except Exception as exc:  # noqa: BLE001
            pytest.fail(f"{xml_path.name} raised {exc!r}")
