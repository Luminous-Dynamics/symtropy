#!/usr/bin/env python3
"""Independent stdlib oracle for blind interval coverage, sharpness and OOD stress."""
from __future__ import annotations

import csv
import math
import sys
from pathlib import Path

LEVELS = [0.80, 0.90, 0.95]
HALF_WIDTHS = [0.80, 0.90, 0.95]
Z = 1.959963984540054


def require(ok: bool, message: str) -> None:
    if not ok:
        raise ValueError(message)


def wilson(successes: int, trials: int) -> tuple[float, float]:
    p = successes / trials
    z2 = Z * Z
    den = 1.0 + z2 / trials
    center = (p + z2 / (2.0 * trials)) / den
    half = Z * math.sqrt(p * (1.0 - p) / trials + z2 / (4.0 * trials**2)) / den
    return max(0.0, center - half), min(1.0, center + half)


def expected_row(model: str, domain: str, level_index: int) -> dict[str, float | int]:
    nominal = LEVELS[level_index]
    scale = 1.0 if domain == "in_domain" else 2.0
    width_scale = 2.0 if model == "robust_model" and domain == "out_of_domain" else 1.0
    width = HALF_WIDTHS[level_index] * width_scale
    magnitudes = [(i + 1) / 60.0 * scale for i in range(60)]
    observations = [m if i % 2 == 0 else -m for i, m in enumerate(magnitudes)]
    lower, upper = -width, width
    covered = sum(lower <= value <= upper for value in observations)
    lower_misses = sum(value < lower for value in observations)
    upper_misses = sum(value > upper for value in observations)
    alpha = 1.0 - nominal
    scores = [
        (upper - lower)
        + (2.0 / alpha) * max(lower - value, 0.0)
        + (2.0 / alpha) * max(value - upper, 0.0)
        for value in observations
    ]
    errors = [-value for value in observations]
    lower_wilson, upper_wilson = wilson(covered, 60)
    return {
        "sample_count": 60,
        "covered_count": covered,
        "lower_miss_count": lower_misses,
        "upper_miss_count": upper_misses,
        "empirical_coverage": covered / 60.0,
        "wilson_95_lower": lower_wilson,
        "wilson_95_upper": upper_wilson,
        "coverage_gap": covered / 60.0 - nominal,
        "mean_interval_width": upper - lower,
        "mean_interval_score": sum(scores) / 60.0,
        "point_mae": sum(abs(value) for value in errors) / 60.0,
        "point_rmse": math.sqrt(sum(value * value for value in errors) / 60.0),
        "point_bias": sum(errors) / 60.0,
    }


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: verify-interval-calibration-independent.py report.csv", file=sys.stderr)
        return 2
    with Path(sys.argv[1]).open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 12, f"expected 12 stratified summaries, got {len(rows)}")
    require(all(row["task_id"] == "synthetic-calibration-task" for row in rows),
            "unexpected task identifier")
    require(all(row["evidence_level"] == "empirical_interval_calibration_and_sharpness_audit_not_future_coverage_guarantee"
                for row in rows), "evidence ceiling missing")
    seen = set()
    for row in rows:
        key = (row["model_id"], row["domain_status"], float(row["nominal_coverage"]))
        require(key not in seen, f"duplicate summary key {key}")
        seen.add(key)
        model, domain, nominal = key
        expected = expected_row(model, domain, LEVELS.index(nominal))
        for field, target in expected.items():
            actual = int(row[field]) if field in {
                "sample_count", "covered_count", "lower_miss_count", "upper_miss_count"
            } else float(row[field])
            require(math.isclose(actual, float(target), rel_tol=1e-10, abs_tol=1e-12),
                    f"{key} {field}: {actual} != {target}")
    nominal_ood_90 = next(
        float(row["empirical_coverage"])
        for row in rows
        if row["model_id"] == "nominal_model"
        and row["domain_status"] == "out_of_domain"
        and float(row["nominal_coverage"]) == 0.90
    )
    robust_ood_90 = next(
        float(row["empirical_coverage"])
        for row in rows
        if row["model_id"] == "robust_model"
        and row["domain_status"] == "out_of_domain"
        and float(row["nominal_coverage"]) == 0.90
    )
    require(nominal_ood_90 == 0.45 and robust_ood_90 == 0.90,
            "out-of-domain challenge did not distinguish undercoverage from robust widening")
    print("independent_interval_oracle=pass summaries=12 levels=3 ood_undercoverage=detected robust_width_tradeoff=verified")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError, StopIteration) as error:
        print(f"independent_interval_oracle=fail reason={error}", file=sys.stderr)
        raise SystemExit(1)
