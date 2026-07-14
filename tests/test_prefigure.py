"""End-to-end CLI smoke test.

Runs the real ``prefig`` command line — install the bundled examples, then build
one — and checks the SVG is written. Executes inside ``tmp_test_outputs/`` (via
the ``in_tmp_output`` fixture) so it no longer litters ``examples/`` and
``examples/output/`` at the repo root.
"""

import os
import subprocess


def _run(args):
    """Run `prefig ...`, falling back to `poetry run prefig ...`."""
    try:
        return subprocess.run(["prefig", *args])
    except FileNotFoundError:
        return subprocess.run(["poetry", "run", "prefig", *args])


def test_prefigure(in_tmp_output):
    _run(["-vv", "examples"])
    result = _run(["-vv", "build", "examples/de-system.xml"])
    assert result.returncode == 0
    assert os.path.exists("examples/output/de-system.svg")
