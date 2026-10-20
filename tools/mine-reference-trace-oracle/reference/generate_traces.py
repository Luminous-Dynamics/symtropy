#!/usr/bin/env python3
"""Test-only trace generator using the pinned upstream implementation.

This script is deliberately isolated to the parity workflow. It is not shipped
as a runtime dependency or used by the Rust benchmark at runtime.
"""
import argparse
import hashlib
import json
import math
import pathlib
import sys

EXPECTED_REPO = "creator-xi/Conformal-Optimistic-Prediction"
EXPECTED_COMMIT = "a41c0dcadc10979d8521ab9b5b8f3d460807d883"


def json_float(value):
    value = float(value)
    if math.isinf(value):
        return "Infinity" if value > 0 else "-Infinity"
    if math.isnan(value):
        return "NaN"
    return value


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", required=True)
    parser.add_argument("--inputs", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    import numpy as np

    repo = pathlib.Path(args.repo).resolve()
    inputs_path = pathlib.Path(args.inputs)
    inputs_bytes = inputs_path.read_bytes()
    inputs = json.loads(inputs_bytes)
    if inputs.get("reference_repository") != EXPECTED_REPO:
        raise SystemExit("input repository does not match pinned reference")
    if inputs.get("reference_commit") != EXPECTED_COMMIT:
        raise SystemExit("input commit does not match pinned reference")

    sys.path.insert(0, str(repo))
    from core import methods

    outputs = []
    for case in inputs["cases"]:
        scores = np.asarray(case["scores"], dtype=float)
        method = case["method"]
        alpha = float(case["alpha"])
        lr = float(case["learning_rate"])
        ahead = int(case["ahead"])

        if method == "aci":
            result = methods.aci(
                scores, alpha, lr, int(case["window_length"]),
                int(case["burnin"]), ahead
            )
        elif method == "aci_clipped":
            result = methods.aci_clipped(
                scores, alpha, lr, int(case["window_length"]),
                int(case["burnin"]), ahead
            )
        elif method == "cop":
            result = methods.COP(
                scores, alpha, lr, int(case["burnin"]), ahead,
                proportional_lr=True
            )
        elif method == "ogd":
            result = methods.OGD(scores, alpha, lr, ahead)
        elif method == "sf_ogd":
            result = methods.SF_OGD(scores, alpha, lr, ahead)
        elif method == "decay_ogd":
            result = methods.decay_OGD(scores, alpha, lr, ahead)
        else:
            raise SystemExit(f"unsupported method: {method}")

        quantiles = [json_float(value) for value in result["q"]]
        coverage = [
            bool(float(q) >= float(score))
            for q, score in zip(result["q"], scores)
        ]
        trace = {
            "name": case["name"],
            "method": method,
            "quantiles": quantiles,
            "coverage": coverage,
        }
        if "alpha" in result:
            trace["adaptive_alpha"] = [float(value) for value in result["alpha"]]
        outputs.append(trace)

    output = {
        "schema_version": 1,
        "reference_repository": EXPECTED_REPO,
        "reference_commit": EXPECTED_COMMIT,
        "implementation": "pinned-upstream-python-reference",
        "input_sha256": hashlib.sha256(inputs_bytes).hexdigest(),
        "cases": outputs,
    }
    pathlib.Path(args.output).write_text(
        json.dumps(output, indent=2, allow_nan=False) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
