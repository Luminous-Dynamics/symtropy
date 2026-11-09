#!/usr/bin/env python3
"""Independent standard-library verifier for joint covariance/fault observability."""

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
FIXTURE = HERE / "fixtures" / "correlated-observability-v0.1.json"
RANK_TOL = 1.0e-12
SYMMETRY_TOL = 1.0e-12
CHOLESKY_TOL = 1.0e-14
TIE_TOL = 1.0e-12


class AuditError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AuditError(message)


def cholesky(matrix: list[list[float]]) -> list[list[float]]:
    n = len(matrix)
    require(n > 0 and all(len(row) == n for row in matrix),
            "covariance must be non-empty and square")
    maximum = max(abs(matrix[i][i]) for i in range(n))
    tolerance = CHOLESKY_TOL * max(1.0, maximum)
    lower = [[0.0] * n for _ in range(n)]
    for i in range(n):
        for j in range(i + 1):
            value = matrix[i][j] - sum(lower[i][k] * lower[j][k] for k in range(j))
            if i == j:
                require(math.isfinite(value) and value > tolerance,
                        "covariance is not positive definite at declared tolerance")
                lower[i][j] = math.sqrt(value)
            else:
                require(lower[j][j] > 0.0, "non-positive Cholesky pivot")
                lower[i][j] = value / lower[j][j]
    return lower


def solve_cholesky(lower: list[list[float]], rhs: list[float]) -> list[float]:
    n = len(lower)
    require(len(rhs) == n, "covariance solve dimension mismatch")
    forward = [0.0] * n
    for i in range(n):
        forward[i] = (
            rhs[i] - sum(lower[i][j] * forward[j] for j in range(i))
        ) / lower[i][i]
    solution = [0.0] * n
    for i in reversed(range(n)):
        solution[i] = (
            forward[i] - sum(lower[j][i] * solution[j] for j in range(i + 1, n))
        ) / lower[i][i]
    require(all(math.isfinite(value) for value in solution),
            "covariance solve produced non-finite values")
    return solution


def validate(data: dict[str, Any]) -> None:
    require(data.get("schema_version") == "aero-inspection-correlated-observability/0.1.0",
            "unsupported schema_version")
    require(isinstance(data.get("model_id"), str) and data["model_id"].strip(),
            "model_id must be non-empty")
    require(isinstance(data.get("covariance_id"), str) and data["covariance_id"].strip(),
            "covariance_id must be non-empty")
    require(data.get("model_scope") == "synthetic_local_linear_observation",
            "unsupported model_scope")
    require(data.get("covariance_provenance") == "synthetic_fixture_known_answer",
            "unsupported covariance provenance")
    require(data.get("maximum_simultaneous_failure_domains") == 1,
            "v0.1 permits one sensor or one declared common-cause domain outage")

    parameters = data.get("parameters")
    sensors = data.get("sensors")
    covariance = data.get("noise_covariance")
    groups = data.get("common_cause_groups")
    require(isinstance(parameters, list) and len(parameters) == 2,
            "v0.1 requires exactly two parameters")
    require(isinstance(sensors, list) and 2 <= len(sensors) <= 16,
            "sensor count must be in 2..=16")
    require(isinstance(groups, list) and len(groups) <= 32,
            "common_cause_groups must contain at most 32 groups")
    select_count = data.get("select_count")
    require(isinstance(select_count, int) and 2 <= select_count <= len(sensors),
            "select_count must be in 2..=candidate sensor count")

    parameter_ids: set[str] = set()
    for parameter in parameters:
        identifier = parameter.get("id")
        scale = parameter.get("scale")
        require(isinstance(identifier, str) and identifier.strip(),
                "parameter ID must be non-empty")
        require(identifier not in parameter_ids, "duplicate parameter ID")
        parameter_ids.add(identifier)
        require(isinstance(scale, (int, float)) and math.isfinite(scale) and scale > 0,
                "parameter scale must be finite and positive")

    sensor_ids: list[str] = []
    for sensor in sensors:
        identifier = sensor.get("id")
        row = sensor.get("observation_row")
        require(isinstance(identifier, str) and identifier.strip(),
                "sensor ID must be non-empty")
        require(isinstance(row, list) and len(row) == 2
                and all(isinstance(value, (int, float)) and math.isfinite(value) for value in row),
                "sensor requires two finite sensitivities")
        sensor_ids.append(identifier)
    require(len(sensor_ids) == len(set(sensor_ids)), "duplicate sensor IDs")
    require(sensor_ids == sorted(sensor_ids),
            "sensor records must be in canonical ascending channel-ID order")

    require(isinstance(covariance, dict), "noise_covariance must be an object")
    require(isinstance(covariance.get("unit"), str) and covariance["unit"].strip(),
            "covariance unit must be non-empty")
    require(covariance.get("channel_ids") == sensor_ids,
            "covariance channel_ids must exactly match sensor identity and matrix order")
    matrix = covariance.get("matrix")
    n = len(sensor_ids)
    require(isinstance(matrix, list) and len(matrix) == n
            and all(isinstance(row, list) and len(row) == n for row in matrix),
            "covariance dimensions must exactly match declared channel IDs")
    require(all(isinstance(value, (int, float)) and math.isfinite(value)
                for row in matrix for value in row),
            "covariance contains non-finite values")
    maximum = max(abs(value) for row in matrix for value in row)
    tolerance = SYMMETRY_TOL * max(1.0, maximum)
    for i in range(n):
        for j in range(n):
            require(abs(matrix[i][j] - matrix[j][i]) <= tolerance,
                    "covariance is not symmetric within declared tolerance")
    cholesky(matrix)

    group_ids: set[str] = set()
    for group in groups:
        identifier = group.get("id")
        members = group.get("sensor_ids")
        require(isinstance(identifier, str) and identifier.strip(),
                "common-cause group ID must be non-empty")
        require(identifier not in group_ids, "duplicate common-cause group ID")
        group_ids.add(identifier)
        require(isinstance(members, list) and len(members) >= 2,
                "common-cause group must contain at least two sensors")
        require(len(members) == len(set(members)),
                "duplicate sensor in common-cause group")
        require(all(member in sensor_ids for member in members),
                "common-cause group references unknown sensor")


def build_scenarios(data: dict[str, Any]) -> list[dict[str, Any]]:
    sensors = sorted((sensor["id"] for sensor in data["sensors"]))
    groups = sorted(data["common_cause_groups"], key=lambda item: item["id"])
    scenarios = [{"scenario_id": "none", "failed_sensor_ids": []}]
    for sensor_id in sensors:
        scenarios.append({
            "scenario_id": f"single_sensor:{sensor_id}",
            "failed_sensor_ids": [sensor_id],
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
    maximum = max((abs(value) for row in work for value in row), default=0.0)
    tolerance = RANK_TOL * max(1.0, maximum)
    pivot_row = 0
    for column in range(2):
        if pivot_row >= len(work):
            break
        best = max(range(pivot_row, len(work)),
                   key=lambda index: abs(work[index][column]))
        if abs(work[best][column]) <= tolerance:
            continue
        work[pivot_row], work[best] = work[best], work[pivot_row]
        pivot = work[pivot_row][column]
        for row in range(pivot_row + 1, len(work)):
            factor = work[row][column] / pivot
            for col in range(column, 2):
                work[row][col] -= factor * work[pivot_row][col]
        pivot_row += 1
    return pivot_row


def information(rows: list[list[float]], covariance: list[list[float]]) -> list[list[float]]:
    if not rows:
        return [[0.0, 0.0], [0.0, 0.0]]
    require(len(rows) == len(covariance), "observation/covariance dimension mismatch")
    lower = cholesky(covariance)
    columns = []
    for parameter in range(2):
        rhs = [row[parameter] for row in rows]
        columns.append(solve_cholesky(lower, rhs))
    fisher = [[0.0, 0.0], [0.0, 0.0]]
    for i in range(2):
        for j in range(2):
            fisher[i][j] = sum(rows[k][i] * columns[j][k] for k in range(len(rows)))
    require(all(math.isfinite(value) for row in fisher for value in row),
            "Fisher information contains non-finite values")
    return fisher


def eig_2x2(matrix: list[list[float]]) -> list[float]:
    a, b, d = matrix[0][0], 0.5 * (matrix[0][1] + matrix[1][0]), matrix[1][1]
    discriminant = math.sqrt((a - d) ** 2 + 4.0 * b ** 2)
    low = 0.5 * (a + d - discriminant)
    high = 0.5 * (a + d + discriminant)
    require(math.isfinite(low) and math.isfinite(high), "non-finite Fisher eigenvalue")
    require(low >= -1.0e-10 * max(1.0, abs(high)),
            "Fisher information is not positive semidefinite")
    return [max(0.0, low), high]


def logdet(matrix: list[list[float]], rank: int) -> float | None:
    if rank != 2:
        return None
    determinant = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0]
    return math.log(determinant) if math.isfinite(determinant) and determinant > 0 else None


def evaluate_scenario(data: dict[str, Any], chosen: list[int],
                      scenario: dict[str, Any], scales: list[float]) -> dict[str, Any]:
    sensors = data["sensors"]
    failed = set(scenario["failed_sensor_ids"])
    retained_indices = [index for index in chosen if sensors[index]["id"] not in failed]
    retained_ids = [sensors[index]["id"] for index in retained_indices]
    rows = [[sensors[index]["observation_row"][p] * scales[p] for p in range(2)]
            for index in retained_indices]
    require(all(math.isfinite(value) for row in rows for value in row),
            "scaled observation row overflowed")
    full_matrix = data["noise_covariance"]["matrix"]
    covariance = [[full_matrix[i][j] for j in retained_indices]
                  for i in retained_indices]
    diagonal_covariance = [
        [covariance[i][i] if i == j else 0.0 for j in range(len(retained_indices))]
        for i in range(len(retained_indices))
    ]
    rank = matrix_rank(rows)
    fisher = information(rows, covariance)
    diagonal_fisher = information(rows, diagonal_covariance)
    eigenvalues = eig_2x2(fisher)
    diagonal_eigenvalues = eig_2x2(diagonal_fisher)
    return {
        "scenario_id": scenario["scenario_id"],
        "failed_sensor_ids": sorted(failed),
        "retained_sensor_ids": retained_ids,
        "rank": rank,
        "fisher_information": fisher,
        "minimum_eigenvalue": eigenvalues[0],
        "log_determinant": logdet(fisher, rank),
        "diagonal_assumption_minimum_eigenvalue": diagonal_eigenvalues[0],
        "diagonal_assumption_log_determinant": logdet(diagonal_fisher, rank),
    }


def better(candidate: dict[str, Any], previous: dict[str, Any]) -> bool:
    delta = candidate["objective_value"] - previous["objective_value"]
    return delta > TIE_TOL or (
        abs(delta) <= TIE_TOL and candidate["sensor_ids"] < previous["sensor_ids"]
    )


def select_best(subsets: list[dict[str, Any]], robust: bool,
                objective_is_d: bool, diagonal: bool) -> dict[str, Any] | None:
    best = None
    for subset in subsets:
        if robust and not subset["robustly_feasible"]:
            continue
        if robust and diagonal:
            value = (
                subset["worst_case_diagonal_assumption_log_determinant"]
                if objective_is_d
                else subset["worst_case_diagonal_assumption_minimum_eigenvalue"]
            )
        elif robust:
            value = (subset["worst_case_log_determinant"] if objective_is_d
                     else subset["worst_case_minimum_eigenvalue"])
        elif diagonal:
            value = (subset["diagonal_assumption_log_determinant"] if objective_is_d
                     else subset["diagonal_assumption_minimum_eigenvalue"])
            if objective_is_d and value is None:
                continue
            if not objective_is_d and subset["nominal_rank"] != 2:
                continue
        else:
            value = (subset["nominal_log_determinant"] if objective_is_d
                     else subset["nominal_minimum_eigenvalue"])
            if objective_is_d and value is None:
                continue
            if not objective_is_d and subset["nominal_rank"] != 2:
                continue
        if value is None:
            continue
        candidate = {"sensor_ids": subset["sensor_ids"], "objective_value": value}
        if best is None or better(candidate, best):
            best = candidate
    return best


def evaluate(data: dict[str, Any], raw: bytes) -> dict[str, Any]:
    validate(data)
    scales = [parameter["scale"] for parameter in data["parameters"]]
    sensors = data["sensors"]
    scenarios = build_scenarios(data)
    subsets = []

    for chosen in itertools.combinations(range(len(sensors)), data["select_count"]):
        ids = [sensors[index]["id"] for index in chosen]
        metrics = [evaluate_scenario(data, list(chosen), scenario, scales)
                   for scenario in scenarios]
        nominal = next(metric for metric in metrics if metric["scenario_id"] == "none")
        robustly_feasible = all(metric["rank"] == 2 for metric in metrics)
        limiting = metrics[0]
        for metric in metrics[1:]:
            delta = metric["minimum_eigenvalue"] - limiting["minimum_eigenvalue"]
            if delta < -TIE_TOL or (
                abs(delta) <= TIE_TOL and metric["scenario_id"] < limiting["scenario_id"]
            ):
                limiting = metric
        worst_case_logdet = (
            min(metric["log_determinant"] for metric in metrics)
            if robustly_feasible else None
        )
        worst_case_diagonal_logdet = (
            min(metric["diagonal_assumption_log_determinant"] for metric in metrics)
            if robustly_feasible else None
        )
        worst_case_diagonal_min_eigenvalue = (
            min(metric["diagonal_assumption_minimum_eigenvalue"] for metric in metrics)
            if robustly_feasible else None
        )
        subsets.append({
            "sensor_ids": ids,
            "nominal_rank": nominal["rank"],
            "nominal_minimum_eigenvalue": nominal["minimum_eigenvalue"],
            "nominal_log_determinant": nominal["log_determinant"],
            "diagonal_assumption_minimum_eigenvalue":
                nominal["diagonal_assumption_minimum_eigenvalue"],
            "diagonal_assumption_log_determinant":
                nominal["diagonal_assumption_log_determinant"],
            "robustly_feasible": robustly_feasible,
            "worst_case_minimum_eigenvalue":
                limiting["minimum_eigenvalue"] if robustly_feasible else None,
            "worst_case_log_determinant": worst_case_logdet,
            "worst_case_diagonal_assumption_minimum_eigenvalue":
                worst_case_diagonal_min_eigenvalue,
            "worst_case_diagonal_assumption_log_determinant":
                worst_case_diagonal_logdet,
            "limiting_scenario_id": limiting["scenario_id"],
            "scenario_metrics": metrics,
        })

    return {
        "schema_version": data["schema_version"],
        "model_id": data["model_id"],
        "source_sha256": hashlib.sha256(raw).hexdigest(),
        "covariance_id": data["covariance_id"],
        "covariance_provenance": data["covariance_provenance"],
        "covariance_unit": data["noise_covariance"]["unit"],
        "channel_ids": data["noise_covariance"]["channel_ids"],
        "candidate_sensor_count": len(sensors),
        "select_count": data["select_count"],
        "failure_scenario_count": len(scenarios),
        "subset_count": len(subsets),
        "robustly_feasible_subset_count": sum(
            subset["robustly_feasible"] for subset in subsets
        ),
        "nominal_full_covariance_d_optimal_selection":
            select_best(subsets, False, True, False),
        "nominal_full_covariance_e_optimal_selection":
            select_best(subsets, False, False, False),
        "nominal_diagonal_assumption_d_optimal_selection":
            select_best(subsets, False, True, True),
        "nominal_diagonal_assumption_e_optimal_selection":
            select_best(subsets, False, False, True),
        "robust_full_covariance_d_optimal_selection":
            select_best(subsets, True, True, False),
        "robust_full_covariance_e_optimal_selection":
            select_best(subsets, True, False, False),
        "robust_diagonal_assumption_d_optimal_selection":
            select_best(subsets, True, True, True),
        "robust_diagonal_assumption_e_optimal_selection":
            select_best(subsets, True, False, True),
        "subsets": subsets,
        "claim_ceiling": "synthetic_full_covariance_single_failure_domain_observability_only",
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
    candidate["noise_covariance"]["channel_ids"][0:2] = ["S_B", "S_A"]
    cases.append(("covariance_channel_order_mismatch", candidate))
    candidate = copy.deepcopy(data)
    candidate["noise_covariance"]["matrix"][0][1] += 0.01
    cases.append(("non_symmetric_covariance", candidate))
    candidate = copy.deepcopy(data)
    candidate["noise_covariance"]["matrix"][0][1] = 0.2
    candidate["noise_covariance"]["matrix"][1][0] = 0.2
    cases.append(("non_positive_definite_covariance", candidate))
    candidate = copy.deepcopy(data)
    candidate["noise_covariance"]["matrix"].pop()
    cases.append(("covariance_dimension_mismatch", candidate))
    candidate = copy.deepcopy(data)
    candidate["common_cause_groups"][0]["sensor_ids"].append("UNKNOWN")
    cases.append(("unknown_failure_group_member", candidate))
    candidate = copy.deepcopy(data)
    candidate["common_cause_groups"][0]["sensor_ids"].append(
        candidate["common_cause_groups"][0]["sensor_ids"][0]
    )
    cases.append(("duplicate_failure_group_member", candidate))
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
    require(deficient["robust_full_covariance_d_optimal_selection"] is None,
            "rank-deficient robust D-optimal selection was emitted")
    require(deficient["robust_full_covariance_e_optimal_selection"] is None,
            "rank-deficient robust E-optimal selection was emitted")
    print("negative_control=pass case=no_robust_full_rank_subset")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-output", type=Path, required=True)
    args = parser.parse_args()
    try:
        raw = FIXTURE.read_bytes()
        data = json.loads(raw)
        expected = evaluate(data, raw)
        require(expected["subset_count"] == 15, "subset count changed")
        require(expected["failure_scenario_count"] == 10, "failure scenario count changed")
        require(expected["robustly_feasible_subset_count"] == 12,
                "robustly feasible subset count changed")

        checks = [
            ("nominal_full_covariance_d_optimal_selection", ["S_A", "S_C", "S_E", "S_F"]),
            ("nominal_full_covariance_e_optimal_selection", ["S_A", "S_B", "S_E", "S_F"]),
            ("nominal_diagonal_assumption_d_optimal_selection", ["S_A", "S_B", "S_C", "S_D"]),
            ("nominal_diagonal_assumption_e_optimal_selection", ["S_A", "S_B", "S_C", "S_D"]),
            ("robust_full_covariance_d_optimal_selection", ["S_A", "S_C", "S_E", "S_F"]),
            ("robust_full_covariance_e_optimal_selection", ["S_A", "S_C", "S_E", "S_F"]),
            ("robust_diagonal_assumption_d_optimal_selection", ["S_A", "S_C", "S_E", "S_F"]),
            ("robust_diagonal_assumption_e_optimal_selection", ["S_A", "S_C", "S_E", "S_F"]),
        ]
        for key, ids in checks:
            require(expected[key]["sensor_ids"] == ids, f"{key} changed")

        require(math.isclose(
            expected["robust_full_covariance_d_optimal_selection"]["objective_value"],
            3.2252963926711233, rel_tol=1.0e-9
        ), "robust full-covariance D objective changed")
        require(math.isclose(
            expected["robust_full_covariance_e_optimal_selection"]["objective_value"],
            2.906088177643904, rel_tol=1.0e-9
        ), "robust full-covariance E objective changed")

        actual = json.loads(args.rust_output.read_text(encoding="utf-8"))
        compare(actual, expected)
        hostile_controls(data)
        print("aero_correlated_fault_observability=pass subsets=15 scenarios=10 "
              "robust_feasible=12 nominal_full=ACEF/ABEF robust_full=ACEF/ACEF "
              "nominal_diagonal=ABCD/ABCD robust_diagonal=ACEF/ACEF rust_python=pass")
        return 0
    except (OSError, json.JSONDecodeError, AuditError, KeyError, TypeError, ValueError) as error:
        print(f"correlated_observability_error={error}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
