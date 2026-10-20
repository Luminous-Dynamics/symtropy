#!/usr/bin/env python3
"""Compare the Rust port with live and versioned upstream reference traces."""
import argparse
import json
import math
import pathlib

ATOL = 1e-10
REFERENCE_REPO = "creator-xi/Conformal-Optimistic-Prediction"
REFERENCE_COMMIT = "a41c0dcadc10979d8521ab9b5b8f3d460807d883"


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


def validate_document(document, label):
    if document.get("schema_version") != 1:
        raise SystemExit(f"{label}: unsupported trace output schema")
    if document.get("reference_repository") != REFERENCE_REPO:
        raise SystemExit(f"{label}: unexpected reference repository")
    if document.get("reference_commit") != REFERENCE_COMMIT:
        raise SystemExit(f"{label}: unexpected reference commit")


def compare_case_sets(expected_doc, actual_doc, label):
    exp_cases = expected_doc.get("cases", [])
    act_cases = actual_doc.get("cases", [])
    if len(exp_cases) != len(act_cases):
        raise SystemExit(
            f"{label}: case count mismatch: expected={len(exp_cases)}, actual={len(act_cases)}"
        )

    comparisons = 0
    for index, (exp, act) in enumerate(zip(exp_cases, act_cases)):
        case_name = exp.get("name", "<unnamed>")
        prefix = f"{label}/case[{index}]/{case_name}"
        for key in ("name", "method"):
            if exp.get(key) != act.get(key):
                raise SystemExit(
                    f"{prefix}/{key}: expected {exp.get(key)!r}, got {act.get(key)!r}"
                )
        for key in ("quantiles", "coverage"):
            exp_values, act_values = exp.get(key, []), act.get(key, [])
            if len(exp_values) != len(act_values):
                raise SystemExit(
                    f"{prefix}/{key}: length mismatch {len(exp_values)} != {len(act_values)}"
                )
            for step, (left, right) in enumerate(zip(exp_values, act_values)):
                compare_scalar(f"{prefix}/{key}[{step}]", left, right)
                comparisons += 1
        exp_alpha = exp.get("adaptive_alpha")
        act_alpha = act.get("adaptive_alpha")
        if (exp_alpha is None) != (act_alpha is None):
            raise SystemExit(f"{prefix}/adaptive_alpha: presence mismatch")
        if exp_alpha is not None:
            if len(exp_alpha) != len(act_alpha):
                raise SystemExit(f"{prefix}/adaptive_alpha: length mismatch")
            for step, (left, right) in enumerate(zip(exp_alpha, act_alpha)):
                compare_scalar(f"{prefix}/adaptive_alpha[{step}]", left, right)
                comparisons += 1
    return comparisons


def read_json(path):
    return json.loads(pathlib.Path(path).read_text(encoding="utf-8"))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--expected", required=True, help="live trace generated from pinned upstream")
    parser.add_argument("--actual", required=True, help="Rust trace output")
    parser.add_argument("--golden", required=True, help="committed versioned expected trace JSON")
    args = parser.parse_args()

    expected = read_json(args.expected)
    actual = read_json(args.actual)
    golden = read_json(args.golden)
    for label, document in (("live upstream", expected), ("Rust", actual), ("golden", golden)):
        validate_document(document, label)

    if not expected.get("input_sha256"):
        raise SystemExit("live upstream trace is missing its source input digest")
    if golden.get("input_sha256") != expected.get("input_sha256"):
        raise SystemExit(
            "golden input digest mismatch; frozen input fixture or expected output changed "
            "without an intentional version bump"
        )

    # First check that the published-output fixture still matches a fresh run of
    # the pinned upstream source, then check the Rust port against that same run.
    golden_comparisons = compare_case_sets(golden, expected, "golden-vs-live-upstream")
    rust_comparisons = compare_case_sets(expected, actual, "live-upstream-vs-rust")
    print(json.dumps({
        "result": "passed",
        "reference_commit": expected["reference_commit"],
        "case_count": len(expected.get("cases", [])),
        "golden_vs_live_scalar_comparisons": golden_comparisons,
        "live_vs_rust_scalar_comparisons": rust_comparisons,
        "absolute_tolerance": ATOL,
        "input_sha256": expected["input_sha256"],
        "semantics": "committed golden vs pinned Python oracle vs independent Rust port",
    }, indent=2))


if __name__ == "__main__":
    main()
