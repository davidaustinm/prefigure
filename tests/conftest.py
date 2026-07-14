"""Shared pytest fixtures and paths for the PreFigure test suite.

Makes the ``tests/`` directory importable (so ``from _harness import ...``
works) and exposes the neutral asset locations plus a scratch directory for the
few tests that must write to disk.
"""

import os
import sys
from pathlib import Path

import pytest

TESTS_DIR = Path(__file__).resolve().parent
REPO_ROOT = TESTS_DIR.parent

# Neutral, language-agnostic asset directories (the Rust tests will later be
# re-pointed at these same paths).
EXAMPLES_DIR = TESTS_DIR / "examples"
SNAPSHOTS_DIR = TESTS_DIR / "snapshots"
GUIDE_FIGURES_DIR = TESTS_DIR / "guide_figures"
EXPRESSIONS_JSON = TESTS_DIR / "expressions" / "expression_tests.json"

# All ephemeral output goes here; it is gitignored.
TMP_OUTPUTS_DIR = REPO_ROOT / "tmp_test_outputs"

# Let tests do `from _harness import compare, build_helper, expr`.
if str(TESTS_DIR) not in sys.path:
    sys.path.insert(0, str(TESTS_DIR))


@pytest.fixture(scope="session")
def tmp_outputs_dir():
    """Session-wide scratch dir for tests that must write files."""
    TMP_OUTPUTS_DIR.mkdir(parents=True, exist_ok=True)
    return TMP_OUTPUTS_DIR


@pytest.fixture
def in_tmp_output(tmp_outputs_dir, request):
    """Run a test inside a fresh subdirectory of tmp_test_outputs/."""
    workdir = tmp_outputs_dir / request.node.name
    workdir.mkdir(parents=True, exist_ok=True)
    prev = Path.cwd()
    os.chdir(workdir)
    try:
        yield workdir
    finally:
        os.chdir(prev)
