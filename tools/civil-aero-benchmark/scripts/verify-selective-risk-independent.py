#!/usr/bin/env python3
"""Independent stdlib oracle for blind risk-coverage and false-feasible curves."""
from __future__ import annotations

import csv
import math
import sys
from collections import defaultdict
from pathlib import Path


def require(ok: bool, message: str) -> None:
    if not ok:
        raise ValueError(message)


def expected(domain: str, model: str) -> list[dict]:
    shift = 1.0 if domain == "in_domain" else 2.0
    samples = []
    for index in range(20):
        observed_feasible = index < 15
        observed_value = 0.0 if observed_feasible else 1.0
        error = 0.9 if index == 19 else (index + 1) * 0.01 * shift
        point = 0.1 if index == 19 else (
            error if observed_feasible else observed_value + error
        )
        predicted_feasible = observed_feasible or index == 19
        uncertainty = error if model == "error_ranked_uncertainty" else 1.0 - error
        samples.append({
            "sample_id": f"{domain}-{index}",
            "uncertainty": uncertainty,
            "error": abs(point - observed_value),
            "false_feasible": predicted_feasible and not observed_feasible,
            "predicted_feasible": predicted_feasible,
            "missed_feasible": not predicted_feasible and observed_feasible,
        })
    samples.sort(key=lambda row: (row["uncertainty"], row["sample_id"]))
    return samples


def curve(samples: list[dict]) -> list[dict]:
    count = len(samples)
    rows = []
    accepted = 0
    error_sum = squared_sum = 0.0
    false_feasible = predicted_feasible = missed_feasible = 0
    previous_coverage = previous_mae = previous_false_rate = 0.0
    area_mae = area_false = 0.0
    cursor = 0
    while cursor < count:
        threshold = samples[cursor]["uncertainty"]
        end = cursor
        while end < count and samples[end]["uncertainty"] == threshold:
            sample = samples[end]
            accepted += 1
            error_sum += sample["error"]
            squared_sum += sample["error"]**2
            false_feasible += int(sample["false_feasible"])
            predicted_feasible += int(sample["predicted_feasible"])
            missed_feasible += int(sample["missed_feasible"])
            end += 1
        coverage = accepted / count
        mae = error_sum / accepted
        rmse = math.sqrt(squared_sum / accepted)
        false_rate = false_feasible / accepted
        per_predicted = false_feasible / predicted_feasible if predicted_feasible else None
        area_mae += (coverage - previous_coverage) * (previous_mae + mae) / 2.0
        area_false += (coverage - previous_coverage) * (previous_false_rate + false_rate) / 2.0
        rows.append({
            "accepted_count": accepted,
            "sample_count": count,
            "uncertainty_threshold": threshold,
            "coverage_fraction": coverage,
            "selective_mae": mae,
            "selective_rmse": rmse,
            "false_feasible_count": false_feasible,
            "false_feasible_rate": false_rate,
            "predicted_feasible_count": predicted_feasible,
            "false_feasible_rate_per_predicted_feasible": per_predicted,
            "missed_feasible_count": missed_feasible,
            "partial_area_under_mae_risk_coverage": area_mae,
            "partial_area_under_false_feasible_risk_coverage": area_false,
        })
        previous_coverage, previous_mae, previous_false_rate = coverage, mae, false_rate
        cursor = end
    return rows


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: verify-selective-risk-independent.py curve.csv", file=sys.stderr)
        return 2
    with Path(sys.argv[1]).open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 80, f"expected 80 risk/coverage rows, got {len(rows)}")
    grouped = defaultdict(list)
    for row in rows:
        key = (row["model_id"], row["domain_status"])
        grouped[key].append(row)
    require(set(grouped) == {
        ("error_ranked_uncertainty", "in_domain"),
        ("error_ranked_uncertainty", "out_of_domain"),
        ("misranked_uncertainty", "in_domain"),
        ("misranked_uncertainty", "out_of_domain"),
    }, "unexpected model/domain groups")
    areas = {}
    for (model, domain), actual_rows in grouped.items():
        expected_rows = curve(expected(domain, model))
        require(len(actual_rows) == len(expected_rows), f"curve length mismatch for {model}/{domain}")
        for actual, reference in zip(actual_rows, expected_rows):
            require(actual["task_id"] == "synthetic-selective-task", "task id mismatch")
            require(actual["constraint_id"] == "energy_margin_nonnegative", "constraint id mismatch")
            require(actual["evidence_level"] == "empirical_selective_risk_coverage_and_false_feasible_audit_not_future_guarantee",
                    "evidence ceiling missing")
            for field, target in reference.items():
                if target is None:
                    require(actual[field] == "", f"{model}/{domain} {field} should be blank")
                elif isinstance(target, int):
                    require(int(actual[field]) == target,
                            f"{model}/{domain} {field}: {actual[field]} != {target}")
                else:
                    require(math.isclose(float(actual[field]), target,
                                         rel_tol=1e-10, abs_tol=1e-12),
                            f"{model}/{domain} {field}: {actual[field]} != {target}")
        areas[(model, domain)] = (
            float(actual_rows[-1]["partial_area_under_mae_risk_coverage"]),
            float(actual_rows[-1]["partial_area_under_false_feasible_risk_coverage"]),
        )
    for domain in ("in_domain", "out_of_domain"):
        ranked = areas[("error_ranked_uncertainty", domain)]
        misranked = areas[("misranked_uncertainty", domain)]
        require(ranked[0] < misranked[0], f"uncertainty ranking did not improve MAE risk in {domain}")
        require(ranked[1] < misranked[1], f"uncertainty ranking did not reduce false-feasible risk in {domain}")
    print("independent_selective_risk_oracle=pass groups=4 rows=80 risk_area_and_false_feasible_area=verified")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError) as error:
        print(f"independent_selective_risk_oracle=fail reason={error}", file=sys.stderr)
        raise SystemExit(1)
