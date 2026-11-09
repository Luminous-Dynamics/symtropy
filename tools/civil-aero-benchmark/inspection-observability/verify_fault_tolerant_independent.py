#!/usr/bin/env python3
"""Independent standard-library verifier for the Tier-0 dropout oracle."""

from __future__ import annotations

import argparse
import copy
import hashlib
import itertools
import json
import math
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
FIXTURE = HERE / "fixtures" / "fault-tolerant-observability-v0.1.json"
TOL = 1.0e-12


class AuditError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AuditError(message)


def validate(data: dict[str, Any]) -> None:
    require(data.get("schema_version") == "aero-inspection-fault-tolerant-observability/0.1.0",
            "unsupported schema_version")
    require(isinstance(data.get("model_id"), str) and data["model_id"].strip(),
            "model_id must be non-empty")
    require(data.get("model_scope") == "synthetic_local_linear_observation",
            "unsupported model_scope")
    require(data.get("noise_model") == "independent_variance_per_sensor",
            "unsupported noise_model")
    parameters = data.get("parameters")
    sensors = data.get("sensors")
    groups = data.get("common_cause_groups")
    require(isinstance(parameters, list) and len(parameters) == 2,
            "v0.1 requires exactly two parameters")
    require(isinstance(sensors, list) and 3 <= len(sensors) <= 16,
            "candidate sensor count must be in 3..=16")
    require(isinstance(groups, list) and len(groups) <= 32,
            "common_cause_groups must contain at most 32 groups")
    select_count = data.get("select_count")
    require(isinstance(select_count, int) and 3 <= select_count <= len(sensors),
            "select_count must be in 3..=candidate sensor count")
    require(data.get("maximum_simultaneous_failure_domains") == 1,
            "v0.1 permits one sensor or one common-cause domain failure")

    parameter_ids: set[str] = set()
    for parameter in parameters:
        require(isinstance(parameter, dict), "parameter must be an object")
        identifier = parameter.get("id")
        scale = parameter.get("scale")
        require(isinstance(identifier, str) and identifier.strip(),
                "parameter ID must be non-empty")
        require(identifier not in parameter_ids, "duplicate parameter ID")
        parameter_ids.add(identifier)
        require(isinstance(scale, (int, float)) and math.isfinite(scale) and scale > 0,
                "parameter scale must be finite and positive")

    sensor_ids: set[str] = set()
    for sensor in sensors:
        require(isinstance(sensor, dict), "sensor must be an object")
        identifier = sensor.get("id")
        row = sensor.get("observation_row")
        variance = sensor.get("variance")
        require(isinstance(identifier, str) and identifier.strip(),
                "sensor ID must be non-empty")
        require(identifier not in sensor_ids, "duplicate sensor ID")
        sensor_ids.add(identifier)
        require(isinstance(row, list) and len(row) == 2
                and all(isinstance(value, (int, float)) and math.isfinite(value) for value in row),
                "sensor must have two finite sensitivities")
        require(isinstance(variance, (int, float)) and math.isfinite(variance) and variance > 0,
                "sensor variance must be finite and positive")

    group_ids: set[str] = set()
    for group in groups:
        require(isinstance(group, dict), "common-cause group must be an object")
        identifier = group.get("id")
        members = group.get("sensor_ids")
        require(isinstance(identifier, str) and identifier.strip(),
                "common-cause group ID must be non-empty")
        require(identifier not in group_ids, "duplicate common-cause group ID")
        group_ids.add(identifier)
        require(isinstance(members, list) and len(members) >= 2,
                "common-cause group must contain at least two sensors")
        require(len(members) == len(set(members)), "duplicate sensor in common-cause group")
        require(all(member in sensor_ids for member in members),
                "common-cause group references unknown sensor")


def build_scenarios(data: dict[str, Any]) -> list[dict[str, Any]]:
    sensors = sorted(data["sensors"], key=lambda item: item["id"])
    groups = sorted(data["common_cause_groups"], key=lambda item: item["id"])
    scenarios = [{"scenario_id": "none", "failed_sensor_ids": []}]
    for sensor in sensors:
        scenarios.append({
            "scenario_id": f"single_sensor:{sensor['id']}",
            "failed_sensor_ids": [sensor["id"]],
        })
    for group in groups:
        scenarios.append({
            "scenario_id": f"common_cause:{group['id']}",
            "failed_sensor_ids": sorted(group["sensor_ids"]),
        })
    return scenarios


def matrix_rank(rows: list[list[float]]) -> int:
    if not rows:
        return 0
    work = [row[:] for row in rows]
    row_count = len(work)
    column_count = 2
    maximum = max((abs(value) for row in work for value in row), default=0.0)
    tolerance = TOL * max(1.0, maximum)
    pivot_row = 0
    for column in range(column_count):
        if pivot_row >= row_count:
            break
        best_row = max(range(pivot_row, row_count),
                       key=lambda index: abs(work[index][column]))
        best_value = abs(work[best_row][column])
        if best_value <= tolerance:
            continue
        work[pivot_row], work[best_row] = work[best_row], work[pivot_row]
        pivot = work[pivot_row][column]
        for row in range(pivot_row + 1, row_count):
            factor = work[row][column] / pivot
            for col in range(column, column_count):
                work[row][col] -= factor * work[pivot_row][col]
        pivot_row += 1
    return pivot_row


def scenario_metrics(chosen: list[dict[str, Any]], scenario: dict[str, Any],
                     scales: list[float]) -> dict[str, Any]:
    failed = set(scenario["failed_sensor_ids"])
    retained = [sensor for sensor in chosen if sensor["id"] not in failed]
    retained.sort(key=lambda item: item["id"])
    rows = [[sensor["observation_row"][index] * scales[index] for index in range(2)]
            for sensor in retained]
    require(all(math.isfinite(value) for row in rows for value in row),
            "scaled observation row overflowed")
    rank = matrix_rank(rows)

    f00 = f01 = f11 = 0.0
    for sensor, row in zip(retained, rows):
        weight = 1.0 / sensor["variance"]
        f00 += row[0] * row[0] * weight
        f01 += row[0] * row[1] * weight
        f11 += row[1] * row[1] * weight
    require(all(math.isfinite(value) for value in (f00, f01, f11)),
            "Fisher information overflowed")
    discriminant = math.sqrt(max(0.0, (f00 - f11) ** 2 + 4.0 * f01 ** 2))
    minimum_eigenvalue = max(0.0, ((f00 + f11) - discriminant) / 2.0)
    determinant = f00 * f11 - f01 * f01
    logdet = math.log(determinant) if rank == 2 and determinant > 0 else None

    return {
        "scenario_id": scenario["scenario_id"],
        "failed_sensor_ids": sorted(failed),
        "retained_sensor_ids": [sensor["id"] for sensor in retained],
        "rank": rank,
        "nullity": 2 - rank,
        "minimum_eigenvalue": minimum_eigenvalue,
        "log_determinant": logdet,
    }


def better(candidate: dict[str, Any], previous: dict[str, Any]) -> bool:
    delta = candidate["objective_value"] - previous["objective_value"]
    return delta > TOL or (
        abs(delta) <= TOL and candidate["sensor_ids"] < previous["sensor_ids"]
    )


def evaluate(data: dict[str, Any], raw: bytes) -> dict[str, Any]:
    validate(data)
    sensors = sorted(data["sensors"], key=lambda item: item["id"])
    scales = [parameter["scale"] for parameter in data["parameters"]]
    scenarios = build_scenarios(data)
    subsets = []

    for chosen in itertools.combinations(sensors, data["select_count"]):
        sensor_ids = [sensor["id"] for sensor in chosen]
        metrics = [scenario_metrics(list(chosen), scenario, scales) for scenario in scenarios]
        nominal = next(metric for metric in metrics if metric["scenario_id"] == "none")
        robustly_feasible = all(metric["rank"] == 2 for metric in metrics)
        limiting = metrics[0]
        for metric in metrics[1:]:
            delta = metric["minimum_eigenvalue"] - limiting["minimum_eigenvalue"]
            if delta < -TOL or (abs(delta) <= TOL and metric["scenario_id"] < limiting["scenario_id"]):
                limiting = metric

        subsets.append({
            "sensor_ids": sensor_ids,
            "nominal_rank": nominal["rank"],
            "nominal_minimum_eigenvalue": nominal["minimum_eigenvalue"],
            "nominal_log_determinant": nominal["log_determinant"],
            "robustly_feasible": robustly_feasible,
            "worst_case_minimum_eigenvalue": (
                limiting["minimum_eigenvalue"] if robustly_feasible else None
            ),
            "limiting_scenario_id": limiting["scenario_id"],
            "scenario_metrics": metrics,
        })

    nominal_best = None
    robust_best = None
    for subset in subsets:
        if subset["nominal_rank"] == 2:
            candidate = {
                "sensor_ids": subset["sensor_ids"],
                "objective_value": subset["nominal_minimum_eigenvalue"],
            }
            if nominal_best is None or better(candidate, nominal_best):
                nominal_best = candidate
        if subset["robustly_feasible"]:
            candidate = {
                "sensor_ids": subset["sensor_ids"],
                "objective_value": subset["worst_case_minimum_eigenvalue"],
            }
            if robust_best is None or better(candidate, robust_best):
                robust_best = candidate

    return {
        "schema_version": data["schema_version"],
        "model_id": data["model_id"],
        "source_sha256": hashlib.sha256(raw).hexdigest(),
        "candidate_sensor_count": len(sensors),
        "select_count": data["select_count"],
        "failure_scenario_count": len(scenarios),
        "subset_count": len(subsets),
        "robustly_feasible_subset_count": sum(1 for subset in subsets
                                               if subset["robustly_feasible"]),
        "nominal_e_optimal_selection": nominal_best,
        "robust_e_optimal_selection": robust_best,
        "subsets": subsets,
        "claim_ceiling": "synthetic_single_sensor_dropout_and_single_common_cause_group_only",
    }


def compare(actual: Any, expected: Any, path: str = "root") -> None:
    if isinstance(expected, float):
        require(isinstance(actual, (int, float)) and math.isclose(
            float(actual), expected, rel_tol=1.0e-10, abs_tol=1.0e-12
        ), f"{path}: {actual!r} != {expected!r}")
    elif isinstance(expected, dict):
        require(isinstance(actual, dict) and set(actual) == set(expected),
                f"{path}: object keys differ")
        for key in expected:
            compare(actual[key], expected[key], f"{path}.{key}")
    elif isinstance(expected, list):
        require(isinstance(actual, list) and len(actual) == len(expected),
                f"{path}: list length differs")
        for index, (left, right) in enumerate(zip(actual, expected)):
            compare(left, right, f"{path}[{index}]")
    else:
        require(actual == expected, f"{path}: {actual!r} != {expected!r}")


def hostile_controls(data: dict[str, Any]) -> None:
    cases: list[tuple[str, dict[str, Any]]] = []

    candidate = copy.deepcopy(data)
    candidate["sensors"][1]["id"] = candidate["sensors"][0]["id"]
    cases.append(("duplicate_sensor_id", candidate))

    candidate = copy.deepcopy(data)
    candidate["common_cause_groups"][0]["sensor_ids"].append("UNKNOWN")
    cases.append(("unknown_group_member", candidate))

    candidate = copy.deepcopy(data)
    candidate["common_cause_groups"][0]["sensor_ids"].append(
        candidate["common_cause_groups"][0]["sensor_ids"][0]
    )
    cases.append(("duplicate_group_member", candidate))

    candidate = copy.deepcopy(data)
    candidate["sensors"][0]["variance"] = 0.0
    cases.append(("zero_variance", candidate))

    for label, candidate in cases:
        try:
            validate(candidate)
        except (AuditError, TypeError):
            print(f"negative_control=pass case={label}")
        else:
            raise AuditError(f"negative control unexpectedly passed: {label}")

    candidate = copy.deepcopy(data)
    for sensor in candidate["sensors"]:
        sensor["observation_row"] = [1.0, 0.0]
    deficient = evaluate(candidate, b"rank-deficient-control")
    require(deficient["robustly_feasible_subset_count"] == 0,
            "rank-deficient corpus was promoted")
    require(deficient["robust_e_optimal_selection"] is None,
            "rank-deficient corpus returned a robust selection")
    print("negative_control=pass case=no_robust_full_rank_subset")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-output", type=Path, required=True)
    args = parser.parse_args()
    try:
        raw = FIXTURE.read_bytes()
        data = json.loads(raw)
        expected = evaluate(data, raw)
        require(expected["subset_count"] == 15, "known-answer subset count changed")
        require(expected["failure_scenario_count"] == 10, "known-answer scenario count changed")
        require(expected["robustly_feasible_subset_count"] == 12,
                "known-answer robust-feasibility count changed")
        require(expected["nominal_e_optimal_selection"]["sensor_ids"]
                == ["S_A", "S_B", "S_C", "S_D"],
                "nominal E-optimal known answer changed")
        require(expected["robust_e_optimal_selection"]["sensor_ids"]
                == ["S_A", "S_C", "S_E", "S_F"],
                "robust E-optimal known answer changed")
        nominal = next(subset for subset in expected["subsets"]
                       if subset["sensor_ids"] == ["S_A", "S_B", "S_C", "S_D"])
        require(not nominal["robustly_feasible"],
                "nominal optimum unexpectedly passed common-cause cases")
        actual = json.loads(args.rust_output.read_text(encoding="utf-8"))
        compare(actual, expected)
        hostile_controls(data)
        print("aero_fault_tolerant_observability=pass "
              "subsets=15 scenarios=10 robust_feasible=12 "
              "nominal=ABCD robust=ACEF rust_python=pass")
        return 0
    except (OSError, json.JSONDecodeError, AuditError, KeyError, TypeError, ValueError) as error:
        print(f"fault_tolerant_observability_error={error}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
