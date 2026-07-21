"""Structural SVG comparison with numeric tolerance.

The Python twin of ``rust/prefig-core/tests/svg_compare.rs``: parse two SVG
strings and walk the element trees in lockstep, comparing tags/text/attributes.
Numeric values (coordinates, path data, transforms, dash arrays, style numbers)
are compared within a relative tolerance so float-formatting noise — especially
from MathJax glyph paths — does not cause spurious failures, while structural
differences (missing elements, changed commands, different attribute sets) do.

Extracted from ``correctness_comparison.py`` (the old Python/C++ comparator);
the C++/CLI driver was dropped. ``compare_svgs`` returns a list of human-readable
difference strings — empty means the two SVGs are equivalent within tolerance.
"""

import re

import lxml.etree as ET

# Default relative tolerance, matching the Rust suite (expected_svgs.rs uses 1e-2).
DEFAULT_TOL = 1e-2

# Attributes whose values are numeric and compared with tolerance.
NUMERIC_ATTRS = {
    "x", "y", "x1", "y1", "x2", "y2",
    "cx", "cy", "r", "rx", "ry",
    "width", "height",
    "stroke-width", "stroke-miterlimit",
    "opacity", "stroke-opacity", "fill-opacity",
    "font-size",
    "refX", "refY",
    "markerWidth", "markerHeight",
}

# Attributes skipped entirely (volatile: generated ids and reference handles).
SKIP_ATTRS = {"id", "clip-path", "href", "xlink:href"}

# Cap the number of reported differences so failures stay readable.
MAX_DIFFS = 20

FLOAT_RE = re.compile(r"^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?$")
NUM_RE = re.compile(r"[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?")
PATH_TOKEN_RE = re.compile(
    r"([MmLlHhVvCcSsQqTtAaZz]|[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?)"
)


def parse_svg(svg_string):
    if svg_string is None:
        return None
    try:
        data = svg_string.encode("utf-8") if isinstance(svg_string, str) else svg_string
        return ET.fromstring(data)
    except ET.XMLSyntaxError:
        return None


def is_numeric(s):
    return bool(FLOAT_RE.match(s.strip()))


def numbers_close(a_str, b_str, tol):
    try:
        a = float(a_str)
        b = float(b_str)
        if a == b:
            return True
        return abs(a - b) <= tol + tol * max(1.0, abs(a), abs(b))
    except (ValueError, TypeError):
        return a_str == b_str


def compare_path_data(d1, d2, tol):
    tokens1 = PATH_TOKEN_RE.findall(d1)
    tokens2 = PATH_TOKEN_RE.findall(d2)
    if len(tokens1) != len(tokens2):
        return False, f"path token count differs: {len(tokens1)} vs {len(tokens2)}"
    for i, (t1, t2) in enumerate(zip(tokens1, tokens2)):
        if t1[0].isalpha() and t2[0].isalpha():
            if t1 != t2:
                return False, f"path command differs at token {i}: '{t1}' vs '{t2}'"
        elif not numbers_close(t1, t2, tol):
            return False, f"path value differs at token {i}: {t1} vs {t2}"
    return True, ""


def compare_attr_value(attr_name, val1, val2, tol):
    if val1 == val2:
        return True, ""

    if attr_name == "d":
        return compare_path_data(val1, val2, tol)

    if attr_name == "transform":
        nums1 = NUM_RE.findall(val1)
        nums2 = NUM_RE.findall(val2)
        if len(nums1) != len(nums2):
            return False, f"transform number count: {len(nums1)} vs {len(nums2)}"
        for n1, n2 in zip(nums1, nums2):
            if not numbers_close(n1, n2, tol):
                return False, f"transform value: {n1} vs {n2}"
        text1 = NUM_RE.sub("", val1)
        text2 = NUM_RE.sub("", val2)
        if text1 != text2:
            return False, f"transform structure: '{text1}' vs '{text2}'"
        return True, ""

    if attr_name == "viewBox":
        parts1, parts2 = val1.split(), val2.split()
        if len(parts1) != len(parts2):
            return False, f"viewBox parts: {len(parts1)} vs {len(parts2)}"
        for p1, p2 in zip(parts1, parts2):
            if not numbers_close(p1, p2, tol):
                return False, f"viewBox value: {p1} vs {p2}"
        return True, ""

    if attr_name == "stroke-dasharray":
        parts1 = re.split(r"[,\s]+", val1.strip())
        parts2 = re.split(r"[,\s]+", val2.strip())
        if len(parts1) != len(parts2):
            return False, f"dasharray length: {len(parts1)} vs {len(parts2)}"
        for p1, p2 in zip(parts1, parts2):
            if not numbers_close(p1, p2, tol):
                return False, f"dasharray value: {p1} vs {p2}"
        return True, ""

    if attr_name in NUMERIC_ATTRS and is_numeric(val1) and is_numeric(val2):
        if numbers_close(val1, val2, tol):
            return True, ""
        return False, f"{val1} vs {val2}"

    if attr_name == "style":
        props1 = dict(p.strip().split(":", 1) for p in val1.split(";") if ":" in p)
        props2 = dict(p.strip().split(":", 1) for p in val2.split(";") if ":" in p)
        if set(props1) != set(props2):
            return False, f"style properties differ: {set(props1) ^ set(props2)}"
        for k in props1:
            pv1, pv2 = props1[k].strip(), props2[k].strip()
            if pv1 != pv2:
                if is_numeric(pv1) and is_numeric(pv2):
                    if not numbers_close(pv1, pv2, tol):
                        return False, f"style {k}: {pv1} vs {pv2}"
                else:
                    return False, f"style {k}: '{pv1}' vs '{pv2}'"
        return True, ""

    return False, f"'{val1}' vs '{val2}'"


def strip_ns(tag):
    if isinstance(tag, str) and "}" in tag:
        return tag.split("}", 1)[1]
    return tag


def compare_elements(elem1, elem2, tol, path=""):
    """Recursively compare two elements; return a list of (path, detail)."""
    diffs = []
    tag1, tag2 = strip_ns(elem1.tag), strip_ns(elem2.tag)
    current = f"{path}/{tag1}"

    if tag1 != tag2:
        diffs.append((current, f"tag mismatch: <{tag1}> vs <{tag2}>"))
        return diffs

    text1 = (elem1.text or "").strip()
    text2 = (elem2.text or "").strip()
    if text1 != text2:
        if is_numeric(text1) and is_numeric(text2):
            if not numbers_close(text1, text2, tol):
                diffs.append((current, f"text: {text1} vs {text2}"))
        else:
            diffs.append((current, f"text: '{text1[:60]}' vs '{text2[:60]}'"))

    attrs1 = {k: v for k, v in elem1.attrib.items() if k not in SKIP_ATTRS}
    attrs2 = {k: v for k, v in elem2.attrib.items() if k not in SKIP_ATTRS}
    only_in_1 = set(attrs1) - set(attrs2)
    only_in_2 = set(attrs2) - set(attrs1)
    if only_in_1:
        diffs.append((current, f"attrs only in actual: {only_in_1}"))
    if only_in_2:
        diffs.append((current, f"attrs only in expected: {only_in_2}"))
    for attr in sorted(set(attrs1) & set(attrs2)):
        match, detail = compare_attr_value(attr, attrs1[attr], attrs2[attr], tol)
        if not match:
            diffs.append((f"{current}/@{attr}", detail))

    children1, children2 = list(elem1), list(elem2)
    if len(children1) != len(children2):
        diffs.append((current, f"child count: {len(children1)} vs {len(children2)}"))
    for i in range(min(len(children1), len(children2))):
        diffs.extend(compare_elements(children1[i], children2[i], tol, current))
    return diffs


def compare_svgs(actual, expected, tol=DEFAULT_TOL):
    """Compare two SVG strings; return a list of difference strings (empty = equivalent)."""
    if actual is not None and expected is not None and actual.strip() == expected.strip():
        return []

    actual_tree = parse_svg(actual)
    expected_tree = parse_svg(expected)
    if actual_tree is None and expected_tree is None:
        return ["both produced no valid SVG"]
    if actual_tree is None:
        return ["actual produced no valid SVG"]
    if expected_tree is None:
        return ["expected snapshot is not valid SVG"]

    diffs = compare_elements(actual_tree, expected_tree, tol)
    out = [f"{p}: {d}" for p, d in diffs]
    if len(out) > MAX_DIFFS:
        extra = len(out) - MAX_DIFFS
        out = out[:MAX_DIFFS] + [f"... and {extra} more"]
    return out
