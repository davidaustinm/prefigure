#!/usr/bin/env python3
"""
PreFigure: Rust vs Python full-build performance comparison.

This driver benchmarks the Rust and Python implementations of PreFigure by
running identical example diagrams through each `prefig build` CLI and timing
the end-to-end wall-clock cost. It is the Rust analogue of the C++
`profiling_comparison.py` that shipped with the C++ port: same example set,
same reporting, with the C++ backend swapped for the Rust CLI.

Because the Rust implementation is a standalone binary (not importable into
Python like the C++ pybind11 module was), both backends are measured the same
way -- as a subprocess `prefig build -i <example>.xml`. That makes the two
numbers directly comparable and reflects real command-line usage, including
each implementation's process-startup and import cost. Use `--startup` to see
how much of each time is fixed startup overhead versus diagram work.

Usage:
    python benchmarks/bench_build.py [--runs N] [--output plot.png]

Prerequisites:
    - Python backend installed:  pip install -e .   (or an active venv)
    - Rust CLI built in release:  cd packages/prefig-rust && cargo build --release -p prefig-cli
"""

import argparse
import json
import math
import os
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path

# ============================================================================
# Configuration
# ============================================================================

PACKAGES_DIR = Path(__file__).resolve().parent.parent   # packages/
PROJECT_ROOT = PACKAGES_DIR.parent                       # repo root (holds .venv)
EXAMPLES_DIR = PACKAGES_DIR / "tests" / "examples" / "hand_crafted"
RUST_BIN_DEFAULT = PACKAGES_DIR / "prefig-rust" / "target" / "release" / "prefig"

# Example XML files to benchmark, matching the C++ profiling_comparison.py set.
EXAMPLE_FILES = [
    "tangent.xml",
    "derivatives.xml",
    "de-system.xml",
    "diffeqs.xml",
    "implicit.xml",
    "projection.xml",
    "riemann.xml",
    "roots_of_unity.xml",
]

DEFAULT_RUNS = 5
DEFAULT_WARMUP = 1


# ============================================================================
# Backend discovery
# ============================================================================

def find_python_backend(explicit=None):
    """Return an argv prefix that invokes the Python `prefig` CLI, or None."""
    if explicit:
        return explicit.split()

    # Prefer a project virtualenv, then anything on PATH, then `-m prefig`.
    candidates = [
        PROJECT_ROOT / ".venv" / "bin" / "prefig",
        PROJECT_ROOT / ".venv" / "Scripts" / "prefig.exe",  # Windows
    ]
    for c in candidates:
        if c.exists():
            return [str(c)]

    on_path = shutil.which("prefig")
    if on_path:
        # Make sure it's the Python one, not the Rust binary we're comparing to.
        return [on_path]

    # Fall back to running the package as a module via the current interpreter.
    try:
        subprocess.run(
            [sys.executable, "-c", "import prefig"],
            check=True, capture_output=True,
        )
        return [sys.executable, "-m", "prefig.cli"]
    except (subprocess.CalledProcessError, FileNotFoundError):
        return None


def find_rust_backend(explicit=None):
    """Return an argv prefix that invokes the Rust `prefig` CLI, or None."""
    if explicit:
        p = Path(explicit)
        return [str(p)] if p.exists() else None
    if RUST_BIN_DEFAULT.exists():
        return [str(RUST_BIN_DEFAULT)]
    return None


def find_cpp_backend(explicit=None):
    """
    Return an argv prefix that invokes the C++ backend via its CLI shim, or None.

    The C++ port is a pybind11 module (no standalone binary), so it is driven
    through a tiny Python shim that speaks the same `build -i <file>` interface.
    `explicit` is a command string like "python /path/to/prefig_cpp.py".
    """
    if not explicit:
        return None
    parts = explicit.split()
    # Sanity-check the shim path (last token that looks like a file) exists.
    shim = next((Path(p) for p in parts if p.endswith(".py")), None)
    if shim is not None and not shim.exists():
        return None
    return parts


# ============================================================================
# Timing
# ============================================================================

def run_build(argv_prefix, workdir, xml_name):
    """
    Run `<prefix> build -i <xml_name>` in workdir. Returns (elapsed_ms, ok).
    `ok` is False if the process exits non-zero or produces no SVG output.
    """
    cmd = list(argv_prefix) + ["build", "-i", xml_name]
    start = time.perf_counter()
    proc = subprocess.run(cmd, cwd=workdir, capture_output=True, text=True)
    elapsed = (time.perf_counter() - start) * 1000.0  # ms
    ok = proc.returncode == 0 and bool(list((workdir / "output").glob("*.svg")))
    return elapsed, ok


def time_backend(argv_prefix, xml_path, runs, warmup):
    """
    Time `prefig build` for one example over `runs` timed iterations
    (after `warmup` discarded iterations). Each iteration runs in a fresh
    temp copy so output directories never collide or cache.

    Returns (times_ms, ok). `times_ms` is empty and ok False on failure.
    """
    times = []
    for i in range(warmup + runs):
        with tempfile.TemporaryDirectory() as tmp:
            workdir = Path(tmp)
            shutil.copy(xml_path, workdir / xml_path.name)
            elapsed, ok = run_build(argv_prefix, workdir, xml_path.name)
            if not ok:
                return [], False
            if i >= warmup:
                times.append(elapsed)
    return times, True


def measure_startup(argv_prefix, runs):
    """Time `prefig --help` as a proxy for fixed process/import startup cost."""
    times = []
    for _ in range(runs):
        start = time.perf_counter()
        subprocess.run(list(argv_prefix) + ["--help"], capture_output=True)
        times.append((time.perf_counter() - start) * 1000.0)
    return statistics.mean(times) if times else None


# ============================================================================
# Reporting
# ============================================================================

def stat(times):
    """Return (mean, std) in ms, or (None, 0) if no samples."""
    if not times:
        return None, 0.0
    mean = statistics.mean(times)
    std = statistics.stdev(times) if len(times) > 1 else 0.0
    return mean, std


def fmt_ms(v):
    return "  n/a  " if v is None else f"{v:8.1f}"


# (key, column label) for every backend, in display order. `key` selects the
# `<key>_mean` / `<key>_std` fields in each result dict.
BACKENDS = [("python", "Python"), ("rust", "Rust"), ("cpp", "C++"), ("wasm", "WASM")]


def print_table(results, active, startup=None):
    """
    Print a summary table for the `active` backend keys (a subset of BACKENDS).
    Speedups are shown relative to Python when Python is one of the backends.
    """
    labels = dict(BACKENDS)
    name_w = max([len(r["name"]) for r in results] + [len("TOTAL")])
    baseline = "python" if "python" in active else None
    speed_keys = [k for k in active if k != baseline] if baseline else []

    header = f"  {'example'.ljust(name_w)}"
    for k in active:
        header += f"   {(labels[k] + ' (ms)'):>14}"
    for k in speed_keys:
        header += f"   {(labels[k] + ' x'):>8}"
    print(header)
    print("  " + "-" * (len(header) - 2))

    totals = {k: 0.0 for k in active}
    complete = {k: True for k in active}
    for r in results:
        row = f"  {r['name'].ljust(name_w)}"
        for k in active:
            m, s = r.get(f"{k}_mean"), r.get(f"{k}_std", 0.0)
            if m is not None:
                row += f"   {fmt_ms(m)}±{s:5.1f}"
                totals[k] += m
            else:
                row += f"   {'FAILED':>14}"
                complete[k] = False
        for k in speed_keys:
            bm, km = r.get(f"{baseline}_mean"), r.get(f"{k}_mean")
            row += f"   {(f'{bm / km:.1f}x' if bm and km else '-'):>8}"
        print(row)

    print("  " + "-" * (len(header) - 2))
    total_row = f"  {'TOTAL'.ljust(name_w)}"
    for k in active:
        total_row += f"   {totals[k]:8.1f}      " if complete[k] else f"   {'(partial)':>14}"
    for k in speed_keys:
        ok = complete.get(baseline) and complete[k] and totals[k]
        total_row += f"   {(f'{totals[baseline] / totals[k]:.1f}x' if ok else '-'):>8}"
    print(total_row)

    if startup:
        print()
        print("  Startup overhead (`prefig --help`, subtract from each column above):")
        line = "    " + "    ".join(
            f"{labels[k]}: {fmt_ms(startup.get(k))} ms" for k in active if k in startup
        )
        print(line)


def plot_results(results, active, output_file):
    """Grid of per-example bar charts for the `active` backends. Needs matplotlib."""
    try:
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
        import numpy as np
    except ImportError:
        print("matplotlib/numpy not available; skipping plot "
              "(install them or drop --output).", file=sys.stderr)
        return

    labels_map = dict(BACKENDS)
    colors = {"python": "#4C72B0", "rust": "#DEA584", "cpp": "#DD8452", "wasm": "#8172B3"}

    plotted = [r for r in results if any(r.get(f"{k}_mean") for k in active)]
    n = len(plotted)
    if n == 0:
        print("No results to plot.", file=sys.stderr)
        return

    ncols = min(3, n)
    nrows = math.ceil(n / ncols)
    fig, axes = plt.subplots(nrows, ncols, figsize=(5.0 * ncols, 4.0 * nrows))
    axes = np.atleast_2d(axes)
    baseline = "python" if "python" in active else None

    for idx, r in enumerate(plotted):
        ax = axes[idx // ncols][idx % ncols]
        bl, bm, bs, bc = [], [], [], []
        for k in active:
            m = r.get(f"{k}_mean")
            if m is not None:
                bl.append(labels_map[k]); bm.append(m)
                bs.append(r.get(f"{k}_std", 0.0)); bc.append(colors[k])

        x = np.arange(len(bl))
        bars = ax.bar(x, bm, 0.6, yerr=bs, color=bc,
                      capsize=5, edgecolor="black", linewidth=0.5)
        ax.set_xticks(x)
        ax.set_xticklabels(bl, fontweight="bold")
        ax.set_ylabel("Time (ms)")
        ax.set_title(r["name"], fontsize=10)
        for bar, mean in zip(bars, bm):
            ax.text(bar.get_x() + bar.get_width() / 2, bar.get_height(),
                    f"{mean:.0f} ms", ha="center", va="bottom", fontsize=8)
        # Speedup badge: fastest backend vs Python baseline.
        if baseline and r.get(f"{baseline}_mean"):
            others = [(k, r.get(f"{k}_mean")) for k in active
                      if k != baseline and r.get(f"{k}_mean")]
            if others:
                best_k, best_m = min(others, key=lambda kv: kv[1])
                speedup = r[f"{baseline}_mean"] / best_m
                ax.text(0.95, 0.95, f"{speedup:.1f}x faster\n({labels_map[best_k]})",
                        transform=ax.transAxes, ha="right", va="top",
                        fontsize=12, fontweight="bold", color="green",
                        bbox=dict(boxstyle="round,pad=0.3", facecolor="white",
                                  edgecolor="gray", alpha=0.8))
        ax.grid(axis="y", alpha=0.3)
        ax.set_axisbelow(True)

    for idx in range(n, nrows * ncols):
        axes[idx // ncols][idx % ncols].set_visible(False)

    title_names = " vs ".join(labels_map[k] for k in active)
    fig.suptitle(f"PreFigure: {title_names} build performance",
                 fontsize=14, fontweight="bold", y=1.02)
    fig.tight_layout()
    fig.savefig(output_file, dpi=150, bbox_inches="tight")
    print(f"\nPlot saved to {output_file}")


WASM_BENCH = Path(__file__).resolve().parent / "bench_wasm.mjs"


def run_wasm_benchmark(node_cmd, examples, runs, warmup):
    """
    Run the Node WASM benchmark (bench_wasm.mjs) and return {name: (mean, std)}.
    Returns None if it could not run at all.
    """
    if not WASM_BENCH.exists():
        print(f"  [!!] WASM benchmark script not found: {WASM_BENCH}")
        return None
    with tempfile.NamedTemporaryFile("r", suffix=".json", delete=False) as tf:
        out_json = Path(tf.name)
    cmd = [node_cmd, str(WASM_BENCH), "--runs", str(runs), "--warmup", str(warmup),
           "--json", str(out_json)]
    if examples:
        cmd += ["--examples"] + [f"{e}.xml" for e in examples]
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True)
    except FileNotFoundError:
        print(f"  [!!] `{node_cmd}` not found; cannot run the WASM benchmark.")
        return None
    # Surface the WASM script's own progress/table.
    for line in proc.stdout.splitlines():
        print("  │ " + line)
    if not out_json.exists() or out_json.stat().st_size == 0:
        print("  [!!] WASM benchmark produced no results.")
        if proc.stderr.strip():
            print("       " + proc.stderr.strip().splitlines()[-1])
        return None
    data = json.loads(out_json.read_text())
    out_json.unlink(missing_ok=True)
    return {r["name"]: (r.get("wasm_mean"), r.get("wasm_std", 0.0)) for r in data["results"]}


# ============================================================================
# Main
# ============================================================================

def main():
    parser = argparse.ArgumentParser(
        description="PreFigure Rust vs Python build performance comparison")
    parser.add_argument("--runs", type=int, default=DEFAULT_RUNS,
                        help=f"Timed runs per example (default: {DEFAULT_RUNS})")
    parser.add_argument("--warmup", type=int, default=DEFAULT_WARMUP,
                        help=f"Discarded warmup runs per example (default: {DEFAULT_WARMUP})")
    parser.add_argument("--examples", nargs="*", default=None,
                        help="Specific example files to run (default: all)")
    parser.add_argument("--python-cmd", default=None,
                        help="Override the Python prefig command (e.g. 'python -m prefig.cli')")
    parser.add_argument("--rust-bin", default=None,
                        help="Override the path to the Rust prefig binary")
    parser.add_argument("--cpp-cmd", default=None,
                        help="C++ backend command via the pybind11 CLI shim, e.g. "
                             "'python tmp/prefig_cpp.py'")
    parser.add_argument("--wasm", action="store_true",
                        help="Also benchmark the WASM build via Node (bench_wasm.mjs)")
    parser.add_argument("--node", default="node",
                        help="Node executable to run the WASM benchmark (default: node)")
    parser.add_argument("--startup", action="store_true",
                        help="Also measure fixed startup cost via `prefig --help`")
    parser.add_argument("--output", default=None,
                        help="Write a bar-chart PNG (requires matplotlib)")
    parser.add_argument("--json", default=None,
                        help="Write raw results as JSON to this path")
    args = parser.parse_args()

    print("=" * 64)
    print("  PreFigure: build performance comparison")
    print("=" * 64)
    print(f"  Runs per example: {args.runs} (+{args.warmup} warmup)")

    python_cmd = find_python_backend(args.python_cmd)
    rust_cmd = find_rust_backend(args.rust_bin)
    cpp_cmd = find_cpp_backend(args.cpp_cmd)

    if python_cmd:
        print(f"  [OK] Python backend: {' '.join(python_cmd)}")
    else:
        print("  [!!] Python backend not found. Install with: pip install -e .")
    if rust_cmd:
        print(f"  [OK] Rust backend:   {' '.join(rust_cmd)}")
    else:
        print("  [!!] Rust backend not found. Build with:")
        print("       cd packages/prefig-rust && cargo build --release -p prefig-cli")
    if cpp_cmd:
        print(f"  [OK] C++ backend:    {' '.join(cpp_cmd)}")
    elif args.cpp_cmd:
        print("  [!!] C++ shim not found for --cpp-cmd; skipping C++.")

    if args.wasm:
        print(f"  [OK] WASM backend:   {args.node} {WASM_BENCH.name}")

    if not python_cmd and not rust_cmd and not cpp_cmd and not args.wasm:
        sys.exit("No backends available.")

    example_names = args.examples or EXAMPLE_FILES
    examples = [EXAMPLES_DIR / f for f in example_names if (EXAMPLES_DIR / f).exists()]
    missing = [f for f in example_names if not (EXAMPLES_DIR / f).exists()]
    for m in missing:
        print(f"  [--] skipping missing example: {m}")
    if not examples:
        sys.exit(f"No example files found in {EXAMPLES_DIR}")
    print(f"  Examples: {len(examples)}")
    print()

    results = []
    for i, xml_path in enumerate(examples):
        name = xml_path.stem
        print(f"  [{i + 1}/{len(examples)}] {name} ...", end=" ", flush=True)

        py_times, py_ok = ([], False)
        if python_cmd:
            py_times, py_ok = time_backend(python_cmd, xml_path, args.runs, args.warmup)
        rust_times, rust_ok = ([], False)
        if rust_cmd:
            rust_times, rust_ok = time_backend(rust_cmd, xml_path, args.runs, args.warmup)
        cpp_times, cpp_ok = ([], False)
        if cpp_cmd:
            cpp_times, cpp_ok = time_backend(cpp_cmd, xml_path, args.runs, args.warmup)

        pm, ps = stat(py_times)
        rm, rs = stat(rust_times)
        cm, cs = stat(cpp_times)
        results.append({
            "name": name,
            "python_mean": pm, "python_std": ps,
            "rust_mean": rm, "rust_std": rs,
            "cpp_mean": cm, "cpp_std": cs,
        })

        parts = []
        if python_cmd:
            parts.append(f"Py={pm:.0f}ms" if pm is not None else "Py=FAILED")
        if rust_cmd:
            parts.append(f"Rust={rm:.0f}ms" if rm is not None else "Rust=FAILED")
        if cpp_cmd:
            parts.append(f"C++={cm:.0f}ms" if cm is not None else "C++=FAILED")
        if pm and rm:
            parts.append(f"(Rust {pm / rm:.1f}x)")
        if pm and cm:
            parts.append(f"(C++ {pm / cm:.1f}x)")
        print(" | ".join(parts))

    # WASM is timed separately, in Node, then merged into the same rows.
    wasm_ran = False
    if args.wasm:
        print()
        print("  Running WASM benchmark (Node) ...")
        example_stems = [x.stem for x in examples]
        wasm_times = run_wasm_benchmark(args.node, example_stems, args.runs, args.warmup)
        if wasm_times is not None:
            wasm_ran = True
            for r in results:
                wm, ws = wasm_times.get(r["name"], (None, 0.0))
                r["wasm_mean"], r["wasm_std"] = wm, ws

    active = [k for k, cmd in (("python", python_cmd), ("rust", rust_cmd),
                               ("cpp", cpp_cmd), ("wasm", wasm_ran)) if cmd]

    startup = {}
    if args.startup:
        if python_cmd:
            startup["python"] = measure_startup(python_cmd, args.runs)
        if rust_cmd:
            startup["rust"] = measure_startup(rust_cmd, args.runs)
        if cpp_cmd:
            startup["cpp"] = measure_startup(cpp_cmd, args.runs)

    print()
    print("=" * 64)
    print_table(results, active, startup)
    print("=" * 64)
    if wasm_ran:
        print("  Note: WASM keeps MathJax warm in-process; the Python/Rust CLIs")
        print("  spawn a fresh node+MathJax per build. See benchmarks/README.md.")

    if args.json:
        Path(args.json).write_text(json.dumps({
            "runs": args.runs, "warmup": args.warmup,
            "startup_ms": startup, "results": results,
        }, indent=2))
        print(f"\nJSON written to {args.json}")

    if args.output:
        plot_results(results, active, args.output)

    # Non-zero exit if any active backend failed a build, so CI catches regressions.
    failed = [r["name"] for r in results
              if any(r.get(f"{k}_mean") is None for k in active)]
    if failed:
        print(f"\nBuilds failed for: {', '.join(failed)}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
