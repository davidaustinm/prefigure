"""End-to-end CLI smoke test.

Runs the real ``prefig`` command line — install the bundled examples, then build
one — and checks the SVG is written. Executes inside ``tmp_test_outputs/`` (via
``temp_workdir``) so it no longer litters ``examples/`` and ``examples/output/``
at the repo root.
"""

import os
import subprocess

from helpers.build_helper import temp_workdir


def _run(args):
    """Run `prefig ...`, falling back to `poetry run prefig ...`."""
    try:
        return subprocess.run(["prefig", *args])
    except FileNotFoundError:
        return subprocess.run(["poetry", "run", "prefig", *args])


def test_prefigure():
    with temp_workdir("test_prefigure"):
        _run(["-vv", "examples"])
        result = _run(["-vv", "build", "examples/de-system.xml"])
        assert result.returncode == 0
        assert os.path.exists("examples/output/de-system.svg")
