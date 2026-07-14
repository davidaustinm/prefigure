#!/usr/bin/env python3
"""Refresh the expected values in the expression corpus.

The input corpus (session names, ordered steps, and per-step tolerances) lives
in ``tests/expressions/expression_tests.json``; this script re-runs every
``eval`` step through the current ``prefig.core.user_namespace`` and rewrites its
``expect`` field, leaving inputs untouched. Run it after an intentional change to
the evaluator, or after hand-adding a new step (give a new ``eval`` step an empty
``expect`` and this fills it in).

Run from anywhere:
    poetry run python tests/tools/generate_expressions.py
"""

import importlib
import json
import sys
from pathlib import Path

TESTS_DIR = Path(__file__).resolve().parents[1]
REPO_ROOT = TESTS_DIR.parent
CORPUS = TESTS_DIR / "expressions" / "expression_tests.json"

sys.path.insert(0, str(REPO_ROOT))
sys.path.insert(0, str(TESTS_DIR))
from _harness.expr import to_jsonable  # noqa: E402


def main():
    data = json.loads(CORPUS.read_text())
    from prefig.core import user_namespace as un

    total = 0
    for session in data["sessions"]:
        importlib.reload(un)
        for step in session["steps"]:
            op, expr = step["op"], step["input"]
            total += 1
            if op == "define":
                un.define(expr)
            elif op == "eval":
                step["expect"] = to_jsonable(un.valid_eval(expr))
            elif op == "error":
                try:
                    un.valid_eval(expr)
                    raise AssertionError(f"expected error for {expr!r} but it evaluated")
                except AssertionError:
                    raise
                except Exception:  # noqa: BLE001 - the expected outcome
                    pass
                step.pop("expect", None)

    CORPUS.write_text(json.dumps(data, indent=1))
    print(f"wrote {CORPUS} ({len(data['sessions'])} sessions, {total} steps)")


if __name__ == "__main__":
    main()
