"""Build a PreFigure diagram to an in-memory SVG string.

The Python twin of the routine that generates the golden snapshots: parse a
source file, strip namespaces exactly like ``engine.py``/``parse.py`` do, and
call ``mk_diagram(..., return_string=True)`` so nothing is written to disk. Used
by both ``test_snapshots.py`` (to check against committed goldens) and
``tools/generate_snapshots.py`` (to write them) so building and checking agree.
"""

import contextlib
import importlib
import os
from pathlib import Path

import lxml.etree as ET

_NS = {"pf": "https://prefigure.org"}

# Repo-root scratch dir for tests/tools that must write to disk (gitignored).
TMP_OUTPUTS_DIR = Path(__file__).resolve().parents[2] / "tmp_test_outputs"


@contextlib.contextmanager
def pushd(directory):
    """Temporarily chdir into *directory* (data files resolve relative to cwd)."""
    prev = Path.cwd()
    os.chdir(directory)
    try:
        yield
    finally:
        os.chdir(prev)


@contextlib.contextmanager
def temp_workdir(name):
    """Run inside a fresh subdirectory of tmp_test_outputs/ (gitignored scratch)."""
    workdir = TMP_OUTPUTS_DIR / name
    workdir.mkdir(parents=True, exist_ok=True)
    with pushd(workdir):
        yield workdir


def build_diagram(xml_path, environment="pretext"):
    """Build the first <diagram> in *xml_path*; return (svg, annotations) or None.

    The "pretext" environment resolves ``<read>``/``<image>`` files relative to
    the working directory, so callers should ``pushd`` into the source's
    category directory (which holds ``data/``) first.
    """
    from prefig.core import parse, user_namespace

    xml_path = Path(xml_path)
    importlib.reload(user_namespace)
    tree = ET.parse(str(xml_path))
    diagrams = tree.xpath("//pf:diagram", namespaces=_NS) + tree.xpath("//diagram")
    if not diagrams:
        return None
    diagram = diagrams[0]
    for elem in diagram.getiterator():
        if not isinstance(elem, (ET._Comment, ET._ProcessingInstruction)):
            elem.tag = ET.QName(elem).localname
    parse.check_duplicate_handles(diagram, set())
    return parse.mk_diagram(
        diagram,
        "svg",
        None,            # publication
        xml_path.stem,   # filename -> id prefix
        False,           # suppress caption
        None,            # diagram number
        environment,
        return_string=True,
    )
