"""Expression-evaluation regression tests for ``prefig.core.user_namespace``.

Replays the committed corpus (``tests/expressions/expression_tests.json``)
through a fresh user namespace per session and checks each result against the
stored expectation (numbers within an optional per-step tolerance), plus that
the ``error`` cases are rejected. Regenerate the corpus with
``tests/tools/generate_expressions.py`` when the evaluator changes.
"""

import json
from pathlib import Path

import pytest

from _harness.expr import run_session

EXPRESSIONS_JSON = Path(__file__).resolve().parent / "expressions" / "expression_tests.json"


def _sessions():
    data = json.loads(EXPRESSIONS_JSON.read_text())
    return [pytest.param(s, id=s["name"]) for s in data["sessions"]]


def test_expression_corpus_present():
    data = json.loads(EXPRESSIONS_JSON.read_text())
    steps = sum(len(s["steps"]) for s in data["sessions"])
    assert len(data["sessions"]) >= 12 and steps >= 140


@pytest.mark.parametrize("session", _sessions())
def test_expression_session(session):
    failures = run_session(session)
    assert not failures, (
        f"session {session['name']!r} had failures:\n" + "\n".join(failures)
    )
