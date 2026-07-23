"""Build a PreFigure diagram to an in-memory SVG string.

The Python twin of the routine that generates the reference snapshots: parse a
source file, strip namespaces exactly like ``engine.py``/``parse.py`` do, and
call ``mk_diagram(..., return_string=True)`` so nothing is written to disk. Used
by both ``test_snapshots.py`` (to check against committed snapshots) and
``tools/generate_snapshots.py`` (to write them) so building and checking agree.
"""

import contextlib
import importlib
import os
from pathlib import Path

import lxml.etree as ET

_NS = {"pf": "https://prefigure.org"}

# Repo-root scratch dir for tests/tools that must write to disk (gitignored).
# This file is packages/tests/helpers/build_helper.py, so the repo root is
# four levels up (helpers -> tests -> packages -> repo root).
TMP_OUTPUTS_DIR = Path(__file__).resolve().parents[3] / "tmp_test_outputs"


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


def find_publication():
    """Locate and load a ``pf_publication.xml``, mirroring the CLI exactly.

    ``prefig build`` (``engine.build``) walks up from the working directory
    looking for ``pf_publication.xml``; ``core.parse.parse`` then extracts its
    ``<prefigure>`` element and strips namespaces from the children. Returns
    that element, or None when no publication file is found.
    """
    pub_path = None
    cwd = Path.cwd()
    for directory in [cwd, *cwd.parents]:
        candidate = directory / "pf_publication.xml"
        if candidate.exists():
            pub_path = candidate
            break
    if pub_path is None:
        return None
    publication = ET.parse(str(pub_path))
    pubs = publication.xpath("//pf:prefigure", namespaces=_NS) + publication.xpath(
        "//prefigure"
    )
    if not pubs:
        return None
    publication = pubs[0]
    for child in publication:
        if child.tag is not ET.Comment:
            child.tag = ET.QName(child).localname
    return publication


def load_source(xml_path):
    """Parse *xml_path* and return its first namespace-stripped <diagram>, or None."""
    from prefig.core import parse

    tree = ET.parse(str(Path(xml_path)))
    diagrams = tree.xpath("//pf:diagram", namespaces=_NS) + tree.xpath("//diagram")
    if not diagrams:
        return None
    diagram = diagrams[0]
    for elem in diagram.getiterator():
        if not isinstance(elem, (ET._Comment, ET._ProcessingInstruction)):
            elem.tag = ET.QName(elem).localname
    parse.check_duplicate_handles(diagram, set())
    return diagram


def build_diagram(xml_path, environment="pf_cli", format="svg"):
    """Build the first <diagram> in *xml_path*; return (svg, annotations) or None.

    Builds the way ``prefig build`` does: in the ``pf_cli`` environment, with
    any ``pf_publication.xml`` found from the working directory upward applied.
    Data files (``<read>``/``<image>``) resolve relative to the working
    directory — through the publication's ``<directories data="...">`` when one
    is present — so callers should ``pushd`` into the source's category
    directory first.
    """
    from prefig.core import parse, user_namespace

    xml_path = Path(xml_path)
    importlib.reload(user_namespace)
    diagram = load_source(xml_path)
    if diagram is None:
        return None
    return parse.mk_diagram(
        diagram,
        format,
        find_publication(),
        xml_path.stem,   # filename -> id prefix
        False,           # suppress caption
        None,            # diagram number
        environment,
        return_string=True,
    )


def build_diagram_files(xml_path, environment="pretext"):
    """Build *xml_path* through the file-writing pipeline; return the output dir.

    Unlike :func:`build_diagram`, this runs ``end_figure`` — the path pretext
    uses — which writes ``output/<stem>.svg`` under the working directory plus
    the environment's extra artifacts (for pretext: the SVG 1.1 conversion
    ``<stem>-11.svg`` and, when annotated, ``<stem>-annotations.xml`` and the
    diagcess SVG). Call inside :func:`temp_workdir` to keep the tree clean.
    """
    from prefig.core import parse, user_namespace

    xml_path = Path(xml_path)
    importlib.reload(user_namespace)
    diagram = load_source(xml_path)
    if diagram is None:
        return None
    parse.mk_diagram(
        diagram,
        "svg",
        find_publication(),
        xml_path.stem,
        False,
        None,
        environment,
        return_string=False,
    )
    return Path.cwd() / "output"
