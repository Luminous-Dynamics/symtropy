#!/usr/bin/env python3
"""Independent structural/statistical oracle for a correlated LHS input design."""
from __future__ import annotations

import csv
import math
import sys
from pathlib import Path


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: verify-lhs-input-design-independent.py output.csv", file=sys.stderr)
        return 2
    with Path(sys.argv[1]).open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        rows = list(reader)
        headers = reader.fieldnames or []
    require(len(headers) == 19, f"expected 19 columns, found {len(headers)}")
    require(len(rows) == 128 * 3, f"expected 384 long-form sample rows, got {len(rows)}")
    require(all(None not in row for row in rows), "CSV row has unbound extra columns")
    require(all(row["study_id"] == "synthetic-oat15a-input-design-v0.1" for row in rows), "study ID mismatch")
    require(all(row["seed"] == "20261031" for row in rows), "seed mismatch")
    require(all(row["sample_count"] == "128" for row in rows), "sample count mismatch")
    require(all("no_solver_evaluations_no_output_uq" in row["evidence_level"] for row in rows), "evidence ceiling missing")

    input_ids = sorted({row["input_id"] for row in rows})
    require(input_ids == ["Mach", "Re_scale", "T_static"], f"unexpected input IDs: {input_ids}")
    for input_id in input_ids:
        subset = [row for row in rows if row["input_id"] == input_id]
        require(len(subset) == 128, f"{input_id}: wrong row count")
        strata = sorted(int(float(row["stratified_probability"]) * 128) for row in subset)
        require(strata == list(range(128)), f"{input_id}: independent LHS strata are not one-per-bin")
        values = [float(row["sampled_value"]) for row in subset]
        require(len(set(values)) == 128, f"{input_id}: repeated physical sample value")
        for row in subset:
            lower = float(row["minimum_allowed"])
            upper = float(row["maximum_allowed"])
            value = float(row["sampled_value"])
            require(lower <= value <= upper, f"{input_id}: sample violates physical range")
            require(0.0 < float(row["stratified_probability"]) < 1.0, "LHS probability outside open interval")
            require(math.isfinite(float(row["correlated_normal_score"])), "non-finite correlated score")

    matrix_targets = {
        "Mach": [1.0, 0.3, -0.1],
        "Re_scale": [0.3, 1.0, 0.4],
        "T_static": [-0.1, 0.4, 1.0],
    }
    for row in rows:
        target = [float(value) for value in row["target_correlation_row"].split("|")]
        empirical_latent = [float(value) for value in row["empirical_latent_correlation_row"].split("|")]
        empirical_output = [float(value) for value in row["empirical_output_correlation_row"].split("|")]
        require(
            len(target) == 3 and len(empirical_latent) == 3 and len(empirical_output) == 3,
            "correlation row dimension mismatch",
        )
        require(
            all(math.isclose(a, b, abs_tol=1e-10) for a, b in zip(target, matrix_targets[row["input_id"]])),
            f"{row['input_id']}: target correlation row mismatch",
        )
        require(
            all(math.isfinite(value) and -1.0 <= value <= 1.0 for value in empirical_latent + empirical_output),
            "empirical correlation out of bounds",
        )
        require(
            all(abs(actual - expected) <= 0.15 for actual, expected in zip(empirical_output, target)),
            f"{row['input_id']}: physical output correlation too far from target for this fixture",
        )

    by_sample = {}
    for row in rows:
        by_sample.setdefault(int(row["sample_id"]), set()).add(row["input_id"])
    require(sorted(by_sample) == list(range(128)), "sample IDs are not contiguous")
    require(all(values == set(input_ids) for values in by_sample.values()), "sample is missing an input dimension")
    print("independent_correlated_lhs_oracle=pass samples=128 dimensions=3 exact_independent_strata=true bounds_respected=true empirical_R_reported=true solver_evaluations=0")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError) as error:
        print(f"independent_correlated_lhs_oracle=fail reason={error}", file=sys.stderr)
        raise SystemExit(1)
