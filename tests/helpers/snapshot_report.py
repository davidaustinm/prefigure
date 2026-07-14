#!/usr/bin/env python3
"""Build the examples corpus and report how it differs from the snapshots.

CI companion to ``test_snapshots.py``: instead of failing an assertion, this
builds every example that has a committed snapshot (``tests/snapshots/examples``)
and writes a machine-readable ``report.json`` plus a ready-to-post PR comment
``comment.md`` listing the differences. When fewer than ``--max-visual``
(default 8) examples differ, both the snapshot and the freshly built SVG are
rendered to PNG with ``rsvg-convert`` so the comment can show them side by side.

Usage:
    poetry run python tests/helpers/snapshot_report.py --out-dir OUT
    poetry run python tests/helpers/snapshot_report.py --out-dir OUT \
        --comment-only --image-base-url https://raw.githubusercontent.com/o/r/SHA

The second form rewrites ``comment.md`` from the existing ``report.json`` and
renders, embedding hosted image URLs — used by the workflow after it pushes the
renders to an image branch (the commit SHA is only known after the push).

Outputs under --out-dir:
    report.json    {"total": N, "differing": [...], "new_sources": [...]}
    comment.md     the PR comment body (sticky-comment marker included)
    renders/       <id>-expected.png / <id>-actual.png / <id>-actual.svg
Exit code is always 0; consumers read report.json.
"""

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

TESTS_DIR = Path(__file__).resolve().parents[1]
REPO_ROOT = TESTS_DIR.parent
SNAPSHOTS = TESTS_DIR / "snapshots" / "examples"
EXAMPLES = TESTS_DIR / "examples"

sys.path.insert(0, str(REPO_ROOT))
sys.path.insert(0, str(TESTS_DIR))
from helpers.build_helper import build_diagram, pushd  # noqa: E402
from helpers.compare import DEFAULT_TOL, compare_svgs  # noqa: E402

COMMENT_MARKER = "<!-- prefigure-snapshot-report -->"
MAX_DIFF_LINES = 12   # per example, in the collapsed details block
IMG_WIDTH = 380       # two of these fit a PR comment side by side


def flat_name(snapshot_id: str) -> str:
    """'examples/hand_crafted/tangent' -> 'examples--hand_crafted--tangent'."""
    return snapshot_id.replace("/", "--")


def collect() -> dict:
    """Build every snapshotted example and compare; also list unsnapshotted sources."""
    differing = []
    total = 0
    for snapshot in sorted(SNAPSHOTS.rglob("*.svg")):
        rel = snapshot.relative_to(TESTS_DIR / "snapshots")   # examples/<cat>/<stem>.svg
        snapshot_id = str(rel.with_suffix(""))
        source = TESTS_DIR / rel.with_suffix(".xml")
        total += 1

        if not source.exists():
            differing.append({
                "id": snapshot_id,
                "status": "missing-source",
                "diffs": [f"source {rel.with_suffix('.xml')} no longer exists"],
            })
            continue
        try:
            with pushd(source.parent):
                result = build_diagram(source.name)
        except Exception as exc:  # noqa: BLE001 - report, don't crash the report
            differing.append({
                "id": snapshot_id,
                "status": "build-failed",
                "diffs": [f"build raised {exc!r}"],
            })
            continue
        if result is None:
            differing.append({
                "id": snapshot_id,
                "status": "build-failed",
                "diffs": ["build produced no diagram"],
            })
            continue

        svg = result[0]
        diffs = compare_svgs(svg, snapshot.read_text(), DEFAULT_TOL)
        if diffs:
            differing.append({
                "id": snapshot_id,
                "status": "differs",
                "diffs": diffs,
                "actual_svg": svg,
            })

    new_sources = [
        str(x.relative_to(TESTS_DIR).with_suffix(""))
        for x in sorted(EXAMPLES.rglob("*.xml"))
        if not (TESTS_DIR / "snapshots" / x.relative_to(TESTS_DIR)).with_suffix(".svg").exists()
    ]
    return {"total": total, "differing": differing, "new_sources": new_sources}


def render_pngs(report: dict, out_dir: Path, max_visual: int) -> bool:
    """Render expected/actual PNG pairs when fewer than max_visual examples differ."""
    diffs = [d for d in report["differing"] if d["status"] == "differs"]
    if not diffs or len(report["differing"]) >= max_visual:
        return False
    if shutil.which("rsvg-convert") is None:
        print("rsvg-convert not found; skipping side-by-side renders", file=sys.stderr)
        return False

    renders = out_dir / "renders"
    renders.mkdir(parents=True, exist_ok=True)
    ok = True
    for entry in diffs:
        name = flat_name(entry["id"])
        actual_svg = renders / f"{name}-actual.svg"
        actual_svg.write_text(entry["actual_svg"])
        expected_svg = TESTS_DIR / "snapshots" / f"{entry['id']}.svg"
        for label, svg_path in (("expected", expected_svg), ("actual", actual_svg)):
            png = renders / f"{name}-{label}.png"
            run = subprocess.run(
                ["rsvg-convert", "--background-color=white", "-o", str(png), str(svg_path)],
                capture_output=True,
            )
            if run.returncode != 0:
                print(f"rsvg-convert failed for {svg_path}: {run.stderr.decode()}", file=sys.stderr)
                ok = False
    return ok


def write_comment(report: dict, out_dir: Path, image_base_url: str | None, max_visual: int):
    differing = report["differing"]
    total = report["total"]
    lines = [COMMENT_MARKER, "## 📸 Snapshot report", ""]

    if not differing:
        lines += [f"✅ All {total} example snapshots match this PR's rendering."]
    else:
        lines += [
            f"⚠️ **{len(differing)} of {total}** example snapshots differ from what this PR renders.",
            "",
            "| Example | Status | Differences |",
            "|---|---|---|",
        ]
        for d in differing:
            lines.append(f"| `{d['id']}` | {d['status']} | {len(d['diffs'])} |")
        lines.append("")

        for d in differing:
            shown = d["diffs"][:MAX_DIFF_LINES]
            more = len(d["diffs"]) - len(shown)
            lines += [f"<details><summary><code>{d['id']}</code> — difference list</summary>", "", "```"]
            lines += shown
            if more > 0:
                lines.append(f"... and {more} more")
            lines += ["```", "", "</details>", ""]

        renders_present = (out_dir / "renders").is_dir() and any((out_dir / "renders").glob("*.png"))
        visual = [d for d in differing if d["status"] == "differs"]
        if len(differing) < max_visual and visual:
            lines += ["### Side-by-side", ""]
            if image_base_url and renders_present:
                base = image_base_url.rstrip("/")
                for d in visual:
                    name = flat_name(d["id"])
                    lines += [
                        f"#### `{d['id']}`",
                        "",
                        "| Snapshot (expected) | This PR (actual) |",
                        "|---|---|",
                        f'| <img src="{base}/{name}-expected.png" width="{IMG_WIDTH}"/> '
                        f'| <img src="{base}/{name}-actual.png" width="{IMG_WIDTH}"/> |',
                        "",
                    ]
            else:
                lines += ["_Rendered side-by-side PNGs are attached as the_ "
                          "`snapshot-report` _workflow artifact._", ""]
        elif len(differing) >= max_visual:
            lines += [f"_{len(differing)} snapshots differ (≥ {max_visual}); "
                      "side-by-side views omitted — see the workflow artifact for details._", ""]

        lines += [
            "<sub>To accept an intentional change, run "
            "<code>UPDATE_SNAPSHOTS=1 poetry run pytest \"tests/test_snapshots.py::"
            "test_matches_snapshot[&lt;id&gt;]\"</code> and commit the result; "
            "see <code>tests/README.md</code>.</sub>",
        ]

    if report["new_sources"]:
        lines += ["", f"ℹ️ {len(report['new_sources'])} example source(s) have no snapshot yet "
                  f"(`{'`, `'.join(report['new_sources'][:5])}`"
                  + (", …" if len(report["new_sources"]) > 5 else "")
                  + ") — run `tests/helpers/generate_snapshots.py`."]

    (out_dir / "comment.md").write_text("\n".join(lines) + "\n")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out-dir", required=True, type=Path)
    ap.add_argument("--image-base-url", default=None,
                    help="Base URL where renders/ will be hosted (embedded in comment.md)")
    ap.add_argument("--max-visual", type=int, default=8,
                    help="Include side-by-side images only when fewer than this many differ")
    ap.add_argument("--comment-only", action="store_true",
                    help="Rewrite comment.md from an existing report.json (no rebuilding)")
    args = ap.parse_args()

    out_dir = args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)
    report_path = out_dir / "report.json"

    if args.comment_only:
        report = json.loads(report_path.read_text())
    else:
        report = collect()
        render_pngs(report, out_dir, args.max_visual)
        # actual_svg is bulky; keep report.json lean (SVGs live in renders/)
        slim = {**report, "differing": [
            {k: v for k, v in d.items() if k != "actual_svg"} for d in report["differing"]
        ]}
        report_path.write_text(json.dumps(slim, indent=1))
        report = slim

    write_comment(report, out_dir, args.image_base_url, args.max_visual)
    print(f"{len(report['differing'])} of {report['total']} snapshots differ; "
          f"report in {out_dir}")


if __name__ == "__main__":
    main()
