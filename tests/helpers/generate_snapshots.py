#!/usr/bin/env python3
"""Regenerate the reference SVG snapshots from the current Python implementation.

Self-contained: builds the vendored sources under ``tests/examples/`` (no
external checkout needed) the way ``prefig build`` does — the ``pf_cli``
environment, with each category's ``pf_publication.xml`` applied — and writes
the results under ``tests/snapshots/examples/<category>/``.
Uses the same ``build_diagram`` routine the snapshot test uses, so building and
checking cannot drift apart. Diagrams that build to empty/trivial output (e.g. a
fragment or a missing data file) are skipped rather than checked in as an empty
snapshot, and recorded in the manifest.

Run from anywhere:
    poetry run python tests/helpers/generate_snapshots.py
"""

import json
import sys
from pathlib import Path

import lxml.etree as ET

TESTS_DIR = Path(__file__).resolve().parents[1]
REPO_ROOT = TESTS_DIR.parent
SNAPSHOTS = TESTS_DIR / "snapshots"

# Input corpus (a directory under tests/) -> its category subdirectories.
CORPORA = {
    "examples": ("hand_crafted", "extracted_from_docs", "uses_external_data"),
}

# SVGs with only <defs> (or less) mean the diagram did not really build; skip
# those rather than check in an empty snapshot.
MIN_MEANINGFUL_CHILDREN = 2

sys.path.insert(0, str(REPO_ROOT))
sys.path.insert(0, str(TESTS_DIR))
from helpers.build_helper import build_diagram, pushd  # noqa: E402


def meaningful(svg: str) -> bool:
    try:
        return len(ET.fromstring(svg.encode("utf-8"))) >= MIN_MEANINGFUL_CHILDREN
    except Exception:  # noqa: BLE001
        return False


def build_category(corpus: str, category: str):
    src = TESTS_DIR / corpus / category
    out = SNAPSHOTS / corpus / category
    out.mkdir(parents=True, exist_ok=True)
    built, skipped = [], []
    for xml_path in sorted(src.glob("*.xml")):
        name = xml_path.stem
        if xml_path.name == "pf_publication.xml":
            continue  # publication file, not a diagram source
        try:
            with pushd(xml_path.parent):
                result = build_diagram(xml_path.name)
        except Exception as exc:  # noqa: BLE001
            skipped.append((name, f"error: {exc}"))
            continue
        if result is None or not meaningful(result[0]):
            skipped.append((name, "empty or trivial output"))
            continue
        svg, annotations = result
        (out / f"{name}.svg").write_text(svg)
        if annotations:
            (out / f"{name}.xml").write_text(annotations)
        built.append(name)
    return built, skipped


def main():
    manifest = {}
    for corpus, categories in CORPORA.items():
        manifest[corpus] = {}
        for category in categories:
            if (TESTS_DIR / corpus / category).is_dir():
                built, skipped = build_category(corpus, category)
                manifest[corpus][category] = {"built": built, "skipped": skipped}
    SNAPSHOTS.mkdir(parents=True, exist_ok=True)
    (SNAPSHOTS / "manifest.json").write_text(json.dumps(manifest, indent=1))
    counts = {
        f"{c}/{cat}": (len(v["built"]), len(v["skipped"]))
        for c, cats in manifest.items()
        for cat, v in cats.items()
    }
    print(f"snapshot manifest (built, skipped): {counts}")


if __name__ == "__main__":
    main()
