#!/usr/bin/env python3
"""Regenerate the golden SVG snapshots from the current Python implementation.

Self-contained: builds the vendored example sources under ``tests/examples/`` (no
external checkout needed) in the ``pretext`` environment and writes the results
to ``tests/snapshots/``. Uses the same ``build_diagram`` routine the snapshot
test uses, so building and checking cannot drift apart.

Run from anywhere:
    poetry run python tests/tools/generate_snapshots.py

Outputs, under tests/:
    snapshots/{repo,docs,synth}/*.svg   the Python-built SVGs
    snapshots/{repo,docs}/*.xml         accessibility annotations, when produced
    snapshots/manifest.json             what built and what was skipped
"""

import json
import sys
from pathlib import Path

import lxml.etree as ET

TESTS_DIR = Path(__file__).resolve().parents[1]
REPO_ROOT = TESTS_DIR.parent
EXAMPLES = TESTS_DIR / "examples"
SNAPSHOTS = TESTS_DIR / "snapshots"
CATEGORIES = ("repo", "docs", "synth")

# SVGs with only <defs> (or less) mean the diagram did not really build (e.g. a
# missing data file); skip those rather than check in an empty golden.
MIN_MEANINGFUL_CHILDREN = 2

sys.path.insert(0, str(REPO_ROOT))
sys.path.insert(0, str(TESTS_DIR))
from _harness.build_helper import build_diagram, pushd  # noqa: E402


def meaningful(svg: str) -> bool:
    try:
        return len(ET.fromstring(svg.encode("utf-8"))) >= MIN_MEANINGFUL_CHILDREN
    except Exception:  # noqa: BLE001
        return False


def build_category(category: str):
    src = EXAMPLES / category
    out = SNAPSHOTS / category
    out.mkdir(parents=True, exist_ok=True)
    built, skipped = [], []
    for xml_path in sorted(src.glob("*.xml")):
        name = xml_path.stem
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
    for category in CATEGORIES:
        if (EXAMPLES / category).is_dir():
            built, skipped = build_category(category)
            manifest[category] = {"built": built, "skipped": skipped}
    SNAPSHOTS.mkdir(parents=True, exist_ok=True)
    (SNAPSHOTS / "manifest.json").write_text(json.dumps(manifest, indent=1))
    counts = {k: (len(v["built"]), len(v["skipped"])) for k, v in manifest.items()}
    print(f"snapshot manifest (built, skipped): {counts}")


if __name__ == "__main__":
    main()
