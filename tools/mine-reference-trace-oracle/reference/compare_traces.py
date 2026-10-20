#!/usr/bin/env python3
"""Compare Rust port output with independently generated upstream traces."""
import argparse
import json
import math
import pathlib
import sys

ATOL = 1e-10


def compare_scalar(path, expected, actual):
    if isinstance(expected, str) or isinstance(actual, str):
        if expected != actual:
            raise AssertionError(f"{path}: expected {expected!r}, got {actual!r}")
        return
    if isinstance(expected, bool) or isinstance(actual, bool):
        if expected is not actual:
            raise AssertionError(f"{path}: expected {expected!r}, got {actual!r}")
        return
    if expected is None or actual is None:
        if expected is not actual:
            raise AssertionError(f"{path}: expected {expected!r}, got {actual!r}")
        return
    if not math.isfinite(float(expected)) or not math.isfinite(float(actual)):
        if float(expected) != float(actual):
            raise AssertionError(f"{path}: expected {expected!r}, got {actual!r}")
        return
    if abs(float(expected) - float(actual)) > ATOL:
        raise AssertionError(
            f"{path}: expected {expected:.17g}, got {actual:.17g}; abs error "
            f"{abs(float(expected)-float(actual)):.3g} > {ATOL}"
        )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--expected", required=True)
    parser.add_argument("--actual", required=True)
    args = parser.parse_args()
    expected = json.loads(pathlib.Path(args.expected).read_text(encoding="utf-8"))
    actual = json.loads(pathlib.Path(args.actual).read_text(encoding="utf-8"))

    for document in (expected, actual):
        if document.get("schema_version") != 1:
            raise SystemExit("unsupported trace output schema")
        if document.get("reference_repository") != "creator-xi/Conformal-Optimistic-Prediction":
            raise SystemExit("unexpected reference repository in trace output")
        if document.get("reference_commit") != "a41c0dcadc10979d8521ab9b5b8f3d460807d883":
            raise SystemExit("unexpected reference commit in trace output")

    exp_cases = expected.get("cases", [])
    act_cases = actual.get("cases", [])
    if len(exp_cases) != len(act_cases):
        raise SystemExit(f"case count mismatch: reference={len(exp_cases)}, Rust={len(act_cases)}")

    comparisons = 0
    for index, (exp, act) in enumerate(zip(exp_cases, act_cases)):
        label = f"case[{index}]/{exp.get('name', '<unnamed>')}"
        for key in ("name", "method"):
            if exp.get(key) != act.get(key):
                raise SystemExit(f"{label}/{key}: expected {exp.get(key)!r}, got {act.get(key)!r}")
        for key in ("quantiles", "coverage"):
            exp_values, act_values = exp.get(key, []), act.get(key, [])
            if len(exp_values) != len(act_values):
                raise SystemExit(f"{label}/{key}: length mismatch {len(exp_values)} != {len(act_values)}")
            for step, (left, right) in enumerate(zip(exp_values, act_values)):
                compare_scalar(f"{label}/{key}[{step}]", left, right)
                comparisons += 1
        exp_alpha = exp.get("adaptive_alpha")
        act_alpha = act.get("adaptive_alpha")
        if (exp_alpha is None) != (act_alpha is None):
            raise SystemExit(f"{label}/adaptive_alpha: presence mismatch")
        if exp_alpha is not None:
            if len(exp_alpha) != len(act_alpha):
                raise SystemExit(f"{label}/adaptive_alpha: length mismatch")
            for step, (left, right) in enumerate(zip(exp_alpha, act_alpha)):
                compare_scalar(f"{label}/adaptive_alpha[{step}]", left, right)
                comparisons += 1

    print(json.dumps({
        "result": "passed",
        "reference_commit": expected["reference_commit"],
        "case_count": len(exp_cases),
        "scalar_comparisons": comparisons,
        "absolute_tolerance": ATOL,
        "semantics": "upstream Python-generated trace vs independent Rust port",
    }, indent=2))


if __name__ == "__main__":
    main()
