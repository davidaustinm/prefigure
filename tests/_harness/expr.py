"""Expression-evaluation harness.

The Python twin of ``rust/prefig-core/tests/expression_tests.rs``: replay each
session's steps through a fresh ``prefig.core.user_namespace`` and compare the
result to the committed snapshot in ``tests/expressions/expression_tests.json``.
``to_jsonable`` matches the generator's serialization exactly, so re-evaluated
values line up with the stored expectations; numbers are compared with an
optional per-step tolerance.
"""

import importlib
import math

import numpy as np


def to_jsonable(v, depth=0):
    """Serialize an evaluated value structurally (matches the corpus generator)."""
    if depth > 12:
        raise ValueError("value too deep")
    if isinstance(v, (bool, np.bool_)):
        return {"t": "bool", "v": bool(v)}
    if isinstance(v, (int, np.integer)):
        return {"t": "num", "v": float(v)}
    if isinstance(v, (float, np.floating)):
        if np.isinf(v):
            return {"t": "num", "v": "inf" if v > 0 else "-inf"}
        return {"t": "num", "v": float(v)}
    if isinstance(v, str):
        return {"t": "str", "v": v}
    if isinstance(v, dict):
        return {"t": "dict", "v": {str(k): to_jsonable(x, depth + 1) for k, x in v.items()}}
    if isinstance(v, np.ndarray):
        return {"t": "array", "v": [to_jsonable(x, depth + 1) for x in v]}
    if isinstance(v, (list, tuple)):
        return {"t": "array", "v": [to_jsonable(x, depth + 1) for x in v]}
    if callable(v):
        return {"t": "function"}
    raise ValueError(f"unhandled type {type(v)}")


def _to_float(v):
    if v == "inf":
        return math.inf
    if v == "-inf":
        return -math.inf
    return float(v)


def _compare_num(actual, expected, tol):
    a, e = _to_float(actual), _to_float(expected)
    if math.isinf(a) or math.isinf(e):
        return None if a == e else f"{a} != {e}"
    if math.isnan(a) or math.isnan(e):
        return None if (math.isnan(a) and math.isnan(e)) else f"{a} != {e}"
    if a == e or abs(a - e) <= tol + tol * max(1.0, abs(a), abs(e)):
        return None
    return f"{a} != {e} (tol={tol})"


def compare_jsonable(actual, expected, tol=0.0):
    """Return None if equivalent, else a difference string."""
    if actual["t"] != expected["t"]:
        return f"type {actual['t']} != {expected['t']}"
    t = expected["t"]
    if t == "num":
        return _compare_num(actual["v"], expected["v"], tol)
    if t in ("bool", "str"):
        return None if actual["v"] == expected["v"] else f"{actual['v']!r} != {expected['v']!r}"
    if t == "function":
        return None
    if t == "array":
        av, ev = actual["v"], expected["v"]
        if len(av) != len(ev):
            return f"array length {len(av)} != {len(ev)}"
        for i, (a, e) in enumerate(zip(av, ev)):
            diff = compare_jsonable(a, e, tol)
            if diff:
                return f"[{i}] {diff}"
        return None
    if t == "dict":
        av, ev = actual["v"], expected["v"]
        if set(av) != set(ev):
            return f"dict keys {set(av)} != {set(ev)}"
        for k in ev:
            diff = compare_jsonable(av[k], ev[k], tol)
            if diff:
                return f"[{k!r}] {diff}"
        return None
    return f"unknown type {t}"


def run_session(session):
    """Replay one session; return a list of failure strings (empty = all passed)."""
    from prefig.core import user_namespace as un

    importlib.reload(un)
    failures = []
    for step in session["steps"]:
        op, expr = step["op"], step["input"]
        tol = step.get("tol") or 0.0
        if op == "define":
            un.define(expr)
        elif op == "eval":
            try:
                result = un.valid_eval(expr)
            except Exception as exc:  # noqa: BLE001 - report and continue
                failures.append(f"{expr!r}: raised {exc!r}")
                continue
            diff = compare_jsonable(to_jsonable(result), step["expect"], tol)
            if diff:
                failures.append(f"{expr!r}: {diff}")
        elif op == "error":
            try:
                un.valid_eval(expr)
                failures.append(f"{expr!r}: expected an error but it evaluated")
            except Exception:  # noqa: BLE001 - the expected outcome
                pass
    return failures
