#!/usr/bin/env python3
"""Independent stdlib oracle for the matched-budget Pareto scorecard known-answer fixture."""
from __future__ import annotations

import csv
import math
import sys
from pathlib import Path

EXPECTED = {
    "uniform": {
        "cost": 6.75,
        "hypervolume": 9.5,
        "points": [(0.03, 600.0), (0.035, 500.0)],
    },
    "voi": {
        "cost": 5.0,
        "hypervolume": 10.04,
        "points": [(0.028, 650.0), (0.032, 520.0)],
    },
}


def require(ok: bool, message: str) -> None:
    if not ok:
        raise ValueError(message)


def close(actual: str, expected: float, field: str) -> None:
    value = float(actual)
    require(math.isclose(value, expected, rel_tol=1e-10, abs_tol=1e-10),
            f"{field}: {value} != {expected}")


def hypervolume(points: list[tuple[float, float]], reference: tuple[float, float]) -> float:
    # 2D minimization: sort on objective 1, retaining strictly improving objective 2.
    ordered = sorted(points)
    frontier = []
    best_second = math.inf
    for point in ordered:
        if point[1] < best_second:
            frontier.append(point)
            best_second = point[1]
    area = 0.0
    for index, point in enumerate(frontier):
        next_x = frontier[index + 1][0] if index + 1 < len(frontier) else reference[0]
        area += (next_x - point[0]) * (reference[1] - point[1])
    return area


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: verify-policy-benchmark-independent.py scorecard.csv", file=sys.stderr)
        return 2
    with Path(sys.argv[1]).open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 4, f"expected 4 task/replicate/policy rows, got {len(rows)}")

    seen = set()
    for row in rows:
        policy = row["policy_id"]
        replicate = row["replicate_id"]
        require(policy in EXPECTED, f"unexpected policy {policy}")
        require(replicate in {"seed-1", "seed-2"}, f"unexpected replicate {replicate}")
        require((replicate, policy) not in seen, "duplicate scorecard row")
        seen.add((replicate, policy))
        expected = EXPECTED[policy]
        require(row["task_id"] == "airfoil-multipoint-smoke", "task mismatch")
        require(row["objective_1_name"] == "drag_coefficient", "objective 1 mismatch")
        require(row["objective_2_name"] == "mission_energy_wh", "objective 2 mismatch")
        close(row["ideal_point_1"], 0.0, "ideal_point_1")
        close(row["ideal_point_2"], 0.0, "ideal_point_2")
        close(row["reference_point_1"], 0.05, "reference_point_1")
        close(row["reference_point_2"], 1000.0, "reference_point_2")
        close(row["budget_cap_units"], 8.0, "budget_cap_units")
        close(row["total_cost_units"], expected["cost"], "total_cost_units")
        close(row["remaining_budget_units"], 8.0 - expected["cost"], "remaining_budget_units")
        close(row["budget_utilization_fraction"], expected["cost"] / 8.0, "budget utilization")
        require(int(row["operation_count"]) == 5, "operation count mismatch")
        require(int(row["successful_operations"]) == 4, "successful operation count mismatch")
        require(int(row["failed_operations"]) == 1, "failed operation cost was omitted")
        require(int(row["high_fidelity_attempts"]) == 4, "HF attempt count mismatch")
        require(int(row["successful_high_fidelity_operations"]) == 3, "successful HF count mismatch")
        require(int(row["admitted_candidates"]) == 2, "admitted count mismatch")
        require(int(row["not_admitted_candidates"]) == 1, "non-admitted candidate count mismatch")
        require(int(row["feasible_admitted_candidates"]) == 2, "feasible admitted count mismatch")
        close(row["feasible_yield"], 0.5, "feasible_yield")
        close(row["admission_yield"], 0.5, "admission_yield")
        hv = hypervolume(expected["points"], (0.05, 1000.0))
        close(row["validated_hypervolume_2d"], hv, "validated_hypervolume_2d")
        normalized_hv = hv / (0.05 * 1000.0)
        close(row["normalized_validated_hypervolume_fraction"], normalized_hv,
              "normalized_validated_hypervolume_fraction")
        close(row["validated_hypervolume_per_cost"], hv / expected["cost"],
              "validated_hypervolume_per_cost")
        close(row["normalized_hypervolume_per_cost"], normalized_hv / expected["cost"],
              "normalized_hypervolume_per_cost")
        require(row["evidence_level"] ==
                "caller_bound_admission_receipt_scorecard_not_solver_validation",
                "evidence ceiling missing")

    require(seen == {(rep, pol) for rep in ("seed-1", "seed-2")
                     for pol in ("uniform", "voi")}, "matched policy/replicate matrix incomplete")
    require(EXPECTED["voi"]["hypervolume"] / EXPECTED["voi"]["cost"] >
            EXPECTED["uniform"]["hypervolume"] / EXPECTED["uniform"]["cost"],
            "known-answer VOI policy should have better HV per cost")
    print("independent_policy_benchmark_oracle=pass rows=4 tasks=1 replicates=2 policies=2 "
          "uniform_cost=6.75 voi_cost=5.0 failed_attempts_counted=true admitted_only_hv=true")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError) as error:
        print(f"independent_policy_benchmark_oracle=fail reason={error}", file=sys.stderr)
        raise SystemExit(1)
