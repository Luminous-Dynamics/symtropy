#!/usr/bin/env python3
"""Independent standard-library oracle for correlated sensor observability.

This verifier does not import or call the Rust implementation. It independently
validates the fixture, covariance and measurement operators, computes the
Fisher-information metrics, enumerates sensor subsets and checks Rust JSON.
It is synthetic quasi-static evidence only, not aircraft SHM or certification.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import itertools
import json
import math
import re
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
FIXTURE = HERE / "data" / "instrument-correlated-noise-v0.1.json"
JOINT_FIXTURE = HERE / "data" / "instrument-correlated-failure-v0.1.json"
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
RANK_TOL = 1.0e-10
PIVOT_TOL = 1.0e-12
SCORE_TOL = 1.0e-12
CLAIM = "synthetic_correlated_fisher_observability_only"


class AuditError(ValueError):
    """Raised when fixture, covariance or result violates the contract."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AuditError(message)


def close(actual: float, expected: float, label: str, abs_tol: float = 1e-10, rel_tol: float = 1e-10) -> None:
    require(
        isinstance(actual, (int, float))
        and math.isfinite(float(actual))
        and math.isfinite(float(expected))
        and abs(float(actual) - float(expected))
        <= abs_tol + rel_tol * abs(float(expected)),
        f"{label}: expected {expected:.16g}, received {actual!r}",
    )


def is_digest(value: Any) -> bool:
    return isinstance(value, str) and SHA256_RE.fullmatch(value) is not None


def cholesky_factor(matrix: list[list[float]]) -> list[list[float]]:
    n = len(matrix)
    require(n > 0 and all(len(row) == n for row in matrix),
            "covariance must be non-empty square matrix")
    for i in range(n):
        for j in range(n):
            require(math.isfinite(matrix[i][j]), "covariance contains non-finite entry")
            close(matrix[i][j], matrix[j][i],
                  f"covariance symmetry[{i},{j}]", abs_tol=1e-12, rel_tol=0.0)

    lower = [[0.0 for _ in range(n)] for _ in range(n)]
    for i in range(n):
        for j in range(i + 1):
            value = matrix[i][j] - sum(lower[i][k] * lower[j][k] for k in range(j))
            if i == j:
                require(math.isfinite(value) and value > PIVOT_TOL,
                        "covariance is not strictly positive definite")
                lower[i][j] = math.sqrt(value)
            else:
                require(lower[j][j] > PIVOT_TOL, "invalid Cholesky pivot")
                lower[i][j] = value / lower[j][j]
    return lower


def cholesky_positive_definite(matrix: list[list[float]]) -> None:
    cholesky_factor(matrix)


def solve_cholesky(lower: list[list[float]], rhs: list[float]) -> list[float]:
    n = len(lower)
    require(len(rhs) == n, "covariance solve dimension mismatch")
    forward = [0.0 for _ in range(n)]
    for i in range(n):
        value = rhs[i] - sum(lower[i][j] * forward[j] for j in range(i))
        forward[i] = value / lower[i][i]
    solution = [0.0 for _ in range(n)]
    for i in range(n - 1, -1, -1):
        value = forward[i] - sum(lower[j][i] * solution[j] for j in range(i + 1, n))
        solution[i] = value / lower[i][i]
    require(all(math.isfinite(value) for value in solution),
            "covariance solve is non-finite")
    return solution


def validate_fixture(data: dict[str, Any]) -> None:
    require(data.get("schema_version") == "aero-correlated-sensor-information/0.1.0", "unsupported schema")
    require(data.get("claim_ceiling") == CLAIM, "claim ceiling changed or weakened")
    require(isinstance(data.get("problem_id"), str) and data["problem_id"].strip(), "problem_id missing")

    assumptions = data["model_assumptions"]
    require(len(assumptions["parameter_names"]) == 2, "v0.1 requires two target parameters")
    require(assumptions["observation_model"] == "linear_scalar_gain_times_static_sensitivity", "observation-model scope mismatch")
    require(assumptions["noise_model"] == "correlated_zero_mean_gaussian_covariance", "noise-model scope mismatch")
    require(assumptions["sensitivity_units"] == "dimensionless_normalized", "sensitivity units mismatch")
    require(assumptions["covariance_units"] == "normalized_output_squared", "covariance units mismatch")
    require(assumptions["operating_point_id"] == "synthetic_quasistatic_reference", "operating point mismatch")
    require(assumptions["dynamic_transfer_function_status"] == "NOT_ASSESSED", "dynamic operator promoted in static oracle")
    require(assumptions["physical_aircraft_model"] is False, "synthetic fixture cannot claim a physical aircraft model")

    constraints = data["constraints"]
    sensor_count = len(data["sensors"])
    require(2 <= constraints["d_opt_sensor_count"] <= sensor_count,
            "D-optimal selection count outside candidate set")
    require(2 <= constraints["e_opt_sensor_count"] <= sensor_count,
            "E-optimal selection count outside candidate set")
    require(3 <= constraints["robust_e_opt_sensor_count"] <= sensor_count,
            "robust selection count outside candidate set")
    require(constraints["sensor_dropout_budget"] == 1
            and constraints["common_cause_group_dropout"] is True,
            "v0.1 only supports one sensor or common-cause group dropout")
    require(
        isinstance(constraints["minimum_robust_minimum_eigenvalue"], (int, float))
        and math.isfinite(constraints["minimum_robust_minimum_eigenvalue"])
        and constraints["minimum_robust_minimum_eigenvalue"] > 0.0,
        "minimum robust information threshold must be finite and positive",
    )

    sensors = data["sensors"]
    require(5 <= len(sensors) <= 16, "v0.1 requires 5..=16 sensor candidates")
    ids = [sensor["sensor_id"] for sensor in sensors]
    require(ids == sorted(ids) and len(ids) == len(set(ids)), "sensor IDs must be unique and ordered")
    for sensor in sensors:
        for key in ("sensor_id", "sensor_type", "common_cause_group"):
            require(isinstance(sensor.get(key), str) and sensor[key].strip(), f"{key} missing")
        require(
            isinstance(sensor["sensitivity"], list)
            and len(sensor["sensitivity"]) == 2
            and all(isinstance(v, (int, float)) and math.isfinite(v) for v in sensor["sensitivity"]),
            f"{sensor['sensor_id']}: invalid sensitivity",
        )
        require(
            isinstance(sensor["operator_gain"], (int, float))
            and math.isfinite(sensor["operator_gain"])
            and sensor["operator_gain"] > 0.0,
            f"{sensor['sensor_id']}: invalid operator gain",
        )
        require(sensor["operator_status"] == "ADMITTED", f"{sensor['sensor_id']}: operator not admitted")
        require(is_digest(sensor["operator_sha256"]), f"{sensor['sensor_id']}: invalid operator digest")
        require(sensor["calibration_status"] == "VALID", f"{sensor['sensor_id']}: calibration not valid")
        for key in ("deployment_cost_units", "mass_g"):
            require(math.isfinite(sensor[key]) and sensor[key] >= 0.0, f"{sensor['sensor_id']}: invalid {key}")
        require(math.isfinite(sensor["power_mw"]) and sensor["power_mw"] >= 0.0, f"{sensor['sensor_id']}: invalid power")

    covariance = data["noise_covariance"]
    require(covariance["channel_ids"] == ids, "covariance channel order differs from sensor registry")
    require(covariance["units"] == "normalized_output_squared", "covariance units mismatch")
    require(covariance["evidence_status"] == "synthetic_reference_only", "covariance source status not admitted")
    require(len(covariance["matrix"]) == len(ids), "covariance row count mismatch")
    cholesky_positive_definite(covariance["matrix"])


def principal(matrix: list[list[float]], indices: list[int]) -> list[list[float]]:
    return [[matrix[i][j] for j in indices] for i in indices]


def diagonal_principal(matrix: list[list[float]], indices: list[int]) -> list[list[float]]:
    return [
        [matrix[i][i] if i == j else 0.0 for j in indices]
        for i in indices
    ]


def fisher(data: dict[str, Any], indices: list[int], diagonal_only: bool = False) -> list[list[float]]:
    require(len(indices) >= 2, "at least two surviving sensors required")
    sensors = data["sensors"]
    matrix = data["noise_covariance"]["matrix"]
    covariance = diagonal_principal(matrix, indices) if diagonal_only else principal(matrix, indices)
    lower = cholesky_factor(covariance)
    effective = [
        [sensors[i]["operator_gain"] * value for value in sensors[i]["sensitivity"]]
        for i in indices
    ]
    columns = [
        solve_cholesky(lower, [row[parameter] for row in effective])
        for parameter in range(2)
    ]
    return [
        [
            sum(effective[i][a] * columns[b][i] for i in range(len(indices)))
            for b in range(2)
        ]
        for a in range(2)
    ]


def eigenvalues_2x2(matrix: list[list[float]]) -> list[float]:
    a = matrix[0][0]
    b = 0.5 * (matrix[0][1] + matrix[1][0])
    c = matrix[1][1]
    trace = a + c
    discriminant = math.sqrt((a - c) ** 2 + 4.0 * b**2)
    return [max(0.0, (trace - discriminant) / 2.0), max(0.0, (trace + discriminant) / 2.0)]


def information_metrics(matrix: list[list[float]]) -> dict[str, Any]:
    eigenvalues = eigenvalues_2x2(matrix)
    determinant = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0]
    rank = sum(value > RANK_TOL for value in eigenvalues)
    determinant_value = determinant if rank == 2 and determinant > 0.0 else None
    return {
        "fisher_information": matrix,
        "eigenvalues_ascending": eigenvalues,
        "effective_rank": rank,
        "determinant": determinant_value,
        "log_determinant": math.log(determinant_value) if determinant_value is not None else None,
        "minimum_eigenvalue": eigenvalues[0],
    }


def ids_for(data: dict[str, Any], indices: list[int]) -> list[str]:
    return [data["sensors"][index]["sensor_id"] for index in indices]


def combinations(n: int, k: int) -> list[list[int]]:
    return [list(values) for values in itertools.combinations(range(n), k)]


def select_d_opt(data: dict[str, Any]) -> dict[str, Any]:
    candidates = []
    for indices in combinations(len(data["sensors"]), data["constraints"]["d_opt_sensor_count"]):
        info = information_metrics(fisher(data, indices))
        if info["determinant"] is not None:
            candidates.append((info["determinant"], ids_for(data, indices), info))
    require(candidates, "no full-rank D-opt candidate")
    candidates.sort(key=lambda row: (-row[0], row[1]))
    score, selected, info = candidates[0]
    return {
        "sensor_ids": selected,
        "objective": "MAX_DETERMINANT_FULL_CORRELATED_COVARIANCE",
        "score": score,
        "information": info,
    }


def select_e_opt(data: dict[str, Any], diagonal_only: bool) -> dict[str, Any]:
    candidates = []
    for indices in combinations(len(data["sensors"]), data["constraints"]["e_opt_sensor_count"]):
        info = information_metrics(fisher(data, indices, diagonal_only=diagonal_only))
        if info["effective_rank"] == 2:
            candidates.append((info["minimum_eigenvalue"], ids_for(data, indices), info))
    require(candidates, "no full-rank E-opt candidate")
    candidates.sort(key=lambda row: (-row[0], row[1]))
    score, selected, info = candidates[0]
    return {
        "sensor_ids": selected,
        "objective": (
            "MAX_MINIMUM_EIGENVALUE_DIAGONAL_NOISE_BASELINE"
            if diagonal_only
            else "MAX_MINIMUM_EIGENVALUE_FULL_CORRELATED_COVARIANCE"
        ),
        "score": score,
        "information": info,
    }


def failure_scenarios(data: dict[str, Any], selected_indices: list[int]):
    sensors = data["sensors"]
    scenarios = []
    for index in selected_indices:
        surviving = [value for value in selected_indices if value != index]
        scenarios.append(
            (f"SENSOR_DROP:{sensors[index]['sensor_id']}", surviving, [sensors[index]["sensor_id"]])
        )
    groups = sorted({sensors[index]["common_cause_group"] for index in selected_indices})
    for group in groups:
        lost = [
            sensors[index]["sensor_id"]
            for index in selected_indices
            if sensors[index]["common_cause_group"] == group
        ]
        surviving = [
            index for index in selected_indices
            if sensors[index]["common_cause_group"] != group
        ]
        scenarios.append((f"GROUP_DROP:{group}", surviving, lost))
    return sorted(scenarios, key=lambda value: value[0])


def robust_score(data: dict[str, Any], selected_indices: list[int]):
    records = []
    worst_e = math.inf
    worst_d: float | None = math.inf
    for scenario_id, surviving, lost in failure_scenarios(data, selected_indices):
        if len(surviving) < 2:
            rank, minimum, log_determinant = 0, 0.0, None
        else:
            info = information_metrics(fisher(data, surviving))
            rank = info["effective_rank"]
            minimum = info["minimum_eigenvalue"] if rank == 2 else 0.0
            log_determinant = info["log_determinant"] if rank == 2 else None
        worst_e = min(worst_e, minimum)
        if log_determinant is None:
            worst_d = None
        elif worst_d is not None:
            worst_d = min(worst_d, log_determinant)
        records.append({
            "scenario_id": scenario_id,
            "lost_sensor_ids": lost,
            "surviving_sensor_ids": ids_for(data, surviving),
            "effective_rank": rank,
            "minimum_eigenvalue": minimum,
            "log_determinant": log_determinant,
        })
    return worst_e, worst_d, records


def select_robust_e_opt(data: dict[str, Any]) -> dict[str, Any]:
    candidates = []
    for indices in combinations(
        len(data["sensors"]), data["constraints"]["robust_e_opt_sensor_count"]
    ):
        worst_e, _worst_d, records = robust_score(data, indices)
        candidates.append((worst_e, ids_for(data, indices), records, indices))
    candidates.sort(key=lambda row: (-row[0], row[1]))
    score, selected, records, indices = candidates[0]
    require(score > RANK_TOL, "no robust subset preserves full target rank")
    sensors = data["sensors"]
    return {
        "sensor_ids": selected,
        "objective": "MAX_WORST_CASE_MINIMUM_EIGENVALUE_SINGLE_AND_GROUP_DROPOUT",
        "worst_case_minimum_eigenvalue": score,
        "scenarios": records,
        "total_deployment_cost_units": sum(sensors[i]["deployment_cost_units"] for i in indices),
        "total_mass_g": sum(sensors[i]["mass_g"] for i in indices),
        "total_power_mw": sum(sensors[i]["power_mw"] for i in indices),
    }


def select_robust_d_opt(data: dict[str, Any]) -> dict[str, Any]:
    candidates = []
    for indices in combinations(
        len(data["sensors"]), data["constraints"]["robust_e_opt_sensor_count"]
    ):
        _worst_e, worst_d, records = robust_score(data, indices)
        if worst_d is not None:
            candidates.append((worst_d, ids_for(data, indices), records, indices))
    require(candidates, "no robust full-rank D-optimal candidate")
    candidates.sort(key=lambda row: (-row[0], row[1]))
    score, selected, records, indices = candidates[0]
    sensors = data["sensors"]
    return {
        "sensor_ids": selected,
        "objective": "MAX_WORST_CASE_LOG_DETERMINANT_SINGLE_AND_GROUP_DROPOUT",
        "worst_case_log_determinant": score,
        "scenarios": records,
        "total_deployment_cost_units": sum(sensors[i]["deployment_cost_units"] for i in indices),
        "total_mass_g": sum(sensors[i]["mass_g"] for i in indices),
        "total_power_mw": sum(sensors[i]["power_mw"] for i in indices),
    }


def minimum_cost_robust_selection(data: dict[str, Any]) -> dict[str, Any] | None:
    required = data["constraints"]["minimum_robust_minimum_eigenvalue"]
    minimum_count = max(3, data["constraints"]["sensor_dropout_budget"] + 2)
    sensors = data["sensors"]
    candidates = []
    for count in range(minimum_count, len(sensors) + 1):
        for indices in combinations(len(sensors), count):
            worst_e, _worst_d, records = robust_score(data, indices)
            if worst_e + SCORE_TOL < required:
                continue
            cost = sum(sensors[i]["deployment_cost_units"] for i in indices)
            mass = sum(sensors[i]["mass_g"] for i in indices)
            power = sum(sensors[i]["power_mw"] for i in indices)
            candidates.append((cost, mass, power, ids_for(data, indices), worst_e, records))
    if not candidates:
        return None
    candidates.sort(key=lambda row: (
        round(row[0], 10), round(row[1], 10), round(row[2], 10), row[3]
    ))
    cost, mass, power, selected, achieved, records = candidates[0]
    return {
        "sensor_ids": selected,
        "objective": "MINIMIZE_DEPLOYMENT_COST_SUBJECT_TO_ROBUST_EIGENVALUE_FLOOR",
        "required_minimum_eigenvalue": required,
        "achieved_worst_case_minimum_eigenvalue": achieved,
        "total_deployment_cost_units": cost,
        "total_mass_g": mass,
        "total_power_mw": power,
        "scenarios": records,
    }


def compute(data: dict[str, Any], source_bytes: bytes) -> dict[str, Any]:
    validate_fixture(data)
    diagonal = select_e_opt(data, diagonal_only=True)
    d_opt = select_d_opt(data)
    e_opt = select_e_opt(data, diagonal_only=False)
    robust_d = select_robust_d_opt(data)
    robust = select_robust_e_opt(data)
    minimum_cost_robust = minimum_cost_robust_selection(data)
    covariance = data["noise_covariance"]["matrix"]
    correlated_pairs = sum(
        abs(covariance[i][j]) > 1.0e-12
        for i in range(len(covariance))
        for j in range(i + 1, len(covariance))
    )
    return {
        "schema_version": "aero-correlated-sensor-information-result/0.1.0",
        "problem_id": data["problem_id"],
        "source_sha256": hashlib.sha256(source_bytes).hexdigest(),
        "source_bytes": len(source_bytes),
        "sensor_candidate_count": len(data["sensors"]),
        "covariance_channel_count": len(data["noise_covariance"]["channel_ids"]),
        "covariance_positive_definite": True,
        "covariance_correlated_pairs": correlated_pairs,
        "diagonal_e_opt": diagonal,
        "correlated_d_opt": d_opt,
        "correlated_e_opt": e_opt,
        "robust_correlated_d_opt": robust_d,
        "robust_correlated_e_opt": robust,
        "cost_constrained_robust": minimum_cost_robust,
        "diagonal_to_correlated_e_opt_changed": diagonal["sensor_ids"] != e_opt["sensor_ids"],
        "model_claim_ceiling": CLAIM,
        "aircraft_feasibility": "NOT_ASSESSED",
        "dynamic_observation_status": "NOT_ASSESSED",
        "physical_calibration_status": "NOT_ASSESSED",
    }


def verify_frozen_expected(data: dict[str, Any], result: dict[str, Any]) -> None:
    expected = data["expected"]
    require(result["diagonal_e_opt"]["sensor_ids"] == expected["diagonal_e_opt_sensor_ids"], "diagonal E-opt IDs mismatch")
    close(result["diagonal_e_opt"]["score"], expected["diagonal_e_opt_minimum_eigenvalue"], "diagonal E-opt value")
    require(result["correlated_d_opt"]["sensor_ids"] == expected["correlated_d_opt_sensor_ids"], "correlated D-opt IDs mismatch")
    close(result["correlated_d_opt"]["score"], expected["correlated_d_opt_determinant"], "correlated D-opt determinant", 1e-9, 1e-10)
    require(result["correlated_e_opt"]["sensor_ids"] == expected["correlated_e_opt_sensor_ids"], "correlated E-opt IDs mismatch")
    close(result["correlated_e_opt"]["score"], expected["correlated_e_opt_minimum_eigenvalue"], "correlated E-opt value")
    require(result["robust_correlated_d_opt"]["sensor_ids"] == expected["robust_d_opt_sensor_ids"], "robust D-opt IDs mismatch")
    close(
        result["robust_correlated_d_opt"]["worst_case_log_determinant"],
        expected["robust_d_opt_worst_case_log_determinant"],
        "robust D-opt worst-case log determinant",
        1e-9,
        1e-10,
    )
    require(result["robust_correlated_e_opt"]["sensor_ids"] == expected["robust_e_opt_sensor_ids"], "robust E-opt IDs mismatch")
    close(
        result["robust_correlated_e_opt"]["worst_case_minimum_eigenvalue"],
        expected["robust_e_opt_worst_case_minimum_eigenvalue"],
        "robust E-opt worst-case value",
        1e-9,
        1e-10,
    )
    minimum_cost = result["cost_constrained_robust"]
    require(minimum_cost is not None, "minimum-cost robust design must exist in frozen fixture")
    require(
        minimum_cost["sensor_ids"] == expected["minimum_cost_robust_sensor_ids"],
        "minimum-cost robust sensor IDs mismatch",
    )
    close(
        minimum_cost["achieved_worst_case_minimum_eigenvalue"],
        expected["minimum_cost_robust_worst_case_minimum_eigenvalue"],
        "minimum-cost robust achieved information",
    )
    close(
        minimum_cost["total_deployment_cost_units"],
        expected["minimum_cost_robust_total_deployment_cost_units"],
        "minimum-cost robust deployment cost",
    )
    close(
        minimum_cost["total_mass_g"],
        expected["minimum_cost_robust_total_mass_g"],
        "minimum-cost robust mass",
    )
    close(
        minimum_cost["total_power_mw"],
        expected["minimum_cost_robust_total_power_mw"],
        "minimum-cost robust power",
    )


def compare_rust(actual: dict[str, Any], expected: dict[str, Any]) -> None:
    for key in (
        "schema_version", "problem_id", "source_sha256", "source_bytes",
        "sensor_candidate_count", "covariance_channel_count",
        "covariance_positive_definite", "covariance_correlated_pairs",
        "diagonal_to_correlated_e_opt_changed", "model_claim_ceiling",
        "aircraft_feasibility", "dynamic_observation_status",
        "physical_calibration_status",
    ):
        require(actual.get(key) == expected[key], f"Rust/Python mismatch for {key}")
    for section in ("diagonal_e_opt", "correlated_d_opt", "correlated_e_opt", "robust_correlated_d_opt", "robust_correlated_e_opt"):
        require(actual.get(section) is not None, f"Rust result missing {section}")
        a = actual[section]
        e = expected[section]
        for key in e:
            if key in ("score", "worst_case_minimum_eigenvalue", "worst_case_log_determinant", "total_deployment_cost_units", "total_mass_g", "total_power_mw"):
                close(a[key], e[key], f"{section}.{key}")
            elif key == "scenarios":
                require(len(a[key]) == len(e[key]), "robust scenario count mismatch")
                for actual_scenario, expected_scenario in zip(a[key], e[key]):
                    for scenario_key in ("scenario_id", "lost_sensor_ids", "surviving_sensor_ids", "effective_rank"):
                        require(actual_scenario[scenario_key] == expected_scenario[scenario_key], f"scenario mismatch: {scenario_key}")
                    close(actual_scenario["minimum_eigenvalue"], expected_scenario["minimum_eigenvalue"], "scenario.minimum_eigenvalue")
                    if expected_scenario["log_determinant"] is None:
                        require(actual_scenario["log_determinant"] is None, "scenario log determinant availability mismatch")
                    else:
                        close(actual_scenario["log_determinant"], expected_scenario["log_determinant"], "scenario.log_determinant")
            elif key == "information":
                continue
            else:
                require(a[key] == e[key], f"{section}.{key} mismatch")
        if "information" in e:
            ai = a["information"]
            ei = e["information"]
            require(ai["effective_rank"] == ei["effective_rank"], f"{section} effective rank mismatch")
            require(
                (ai["determinant"] is None) == (ei["determinant"] is None),
                f"{section} determinant availability mismatch",
            )
            if ei["determinant"] is not None:
                close(ai["determinant"], ei["determinant"], f"{section}.determinant")
                close(ai["log_determinant"], ei["log_determinant"], f"{section}.log_determinant")
            require(
                (ai["log_determinant"] is None) == (ei["log_determinant"] is None),
                f"{section} log-determinant availability mismatch",
            )
            for i in range(2):
                for j in range(2):
                    close(ai["fisher_information"][i][j], ei["fisher_information"][i][j], f"{section}.F[{i},{j}]")
            for i in range(2):
                close(ai["eigenvalues_ascending"][i], ei["eigenvalues_ascending"][i], f"{section}.eigen[{i}]")


    expected_minimum_cost = expected["cost_constrained_robust"]
    actual_minimum_cost = actual.get("cost_constrained_robust")
    require(
        (actual_minimum_cost is None) == (expected_minimum_cost is None),
        "minimum-cost robust result availability mismatch",
    )
    if expected_minimum_cost is not None:
        for key in ("sensor_ids", "objective"):
            require(actual_minimum_cost[key] == expected_minimum_cost[key],
                    f"minimum-cost robust {key} mismatch")
        for key in ("required_minimum_eigenvalue",
                    "achieved_worst_case_minimum_eigenvalue",
                    "total_deployment_cost_units", "total_mass_g", "total_power_mw"):
            close(actual_minimum_cost[key], expected_minimum_cost[key],
                  f"minimum-cost robust {key}")
        require(
            len(actual_minimum_cost["scenarios"]) == len(expected_minimum_cost["scenarios"]),
            "minimum-cost robust scenario count mismatch",
        )
        for a_scenario, e_scenario in zip(
            actual_minimum_cost["scenarios"], expected_minimum_cost["scenarios"]
        ):
            for field in ("scenario_id", "lost_sensor_ids", "surviving_sensor_ids", "effective_rank"):
                require(a_scenario[field] == e_scenario[field],
                        f"minimum-cost robust scenario {field} mismatch")
            close(a_scenario["minimum_eigenvalue"], e_scenario["minimum_eigenvalue"],
                  "minimum-cost robust scenario eigenvalue")
            if e_scenario["log_determinant"] is None:
                require(a_scenario["log_determinant"] is None,
                        "minimum-cost robust logdet availability mismatch")
            else:
                close(a_scenario["log_determinant"], e_scenario["log_determinant"],
                      "minimum-cost robust scenario logdet")


def expect_rejected(label: str, action) -> None:
    try:
        action()
    except (AuditError, KeyError, TypeError, ValueError, IndexError):
        print(f"negative_control=pass case={label}")
        return
    raise AuditError(f"negative control unexpectedly passed: {label}")


def hostile_controls(data: dict[str, Any], source_bytes: bytes) -> None:
    impossible = copy.deepcopy(data)
    impossible["constraints"]["minimum_robust_minimum_eigenvalue"] = 1.0e9
    require(
        minimum_cost_robust_selection(impossible) is None,
        "unreachable information threshold must return no selected design",
    )

    bad = copy.deepcopy(data)
    bad["noise_covariance"]["matrix"][0][1] = 0.1
    expect_rejected("asymmetric_covariance", lambda: compute(bad, source_bytes))

    bad = copy.deepcopy(data)
    bad["noise_covariance"]["matrix"][0][1] = 1.0
    bad["noise_covariance"]["matrix"][1][0] = 1.0
    expect_rejected("non_positive_definite_covariance", lambda: compute(bad, source_bytes))

    bad = copy.deepcopy(data)
    bad["sensors"][0]["operator_status"] = "UNKNOWN"
    expect_rejected("unknown_operator_not_identity", lambda: compute(bad, source_bytes))

    bad = copy.deepcopy(data)
    bad["sensors"][0]["operator_sha256"] = "not-a-sha256"
    expect_rejected("invalid_operator_digest", lambda: compute(bad, source_bytes))

    bad = copy.deepcopy(data)
    bad["sensors"][0]["calibration_status"] = "EXPIRED"
    expect_rejected("expired_calibration", lambda: compute(bad, source_bytes))

    bad = copy.deepcopy(data)
    bad["model_assumptions"]["dynamic_transfer_function_status"] = "ADMITTED_COMPLEX"
    expect_rejected("dynamic_operator_outside_static_oracle", lambda: compute(bad, source_bytes))

    bad = copy.deepcopy(data)
    bad["noise_covariance"]["matrix"][0][1] = 0.0
    bad["noise_covariance"]["matrix"][1][0] = 0.0
    expect_rejected(
        "correlation_removed_but_frozen_expected_claim_retained",
        lambda: verify_frozen_expected(bad, compute(bad, source_bytes)),
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-output", type=Path)
    args = parser.parse_args()
    try:
        source = FIXTURE.read_bytes()
        data = json.loads(source.decode("utf-8"))
        expected = compute(data, source)
        verify_frozen_expected(data, expected)
        hostile_controls(data, source)

        joint_source = JOINT_FIXTURE.read_bytes()
        joint_data = json.loads(joint_source.decode("utf-8"))
        joint_expected = compute(joint_data, joint_source)
        verify_frozen_expected(joint_data, joint_expected)
        require(joint_expected["sensor_candidate_count"] == 6,
                "joint regression must contain six sensor candidates")
        require(joint_expected["robust_correlated_e_opt"]["sensor_ids"]
                == ["S_A", "S_C", "S_E", "S_F"],
                "joint robust selection known answer changed")
        require(joint_expected["robust_correlated_d_opt"]["sensor_ids"]
                == ["S_A", "S_C", "S_E", "S_F"],
                "joint robust D-optimal selection known answer changed")
        nominal_e_indices = [0, 1, 4, 5]
        nominal_worst_e, nominal_worst_d, _ = robust_score(joint_data, nominal_e_indices)
        require(nominal_worst_e == 0.0 and nominal_worst_d is None,
                "nominal full-covariance E-optimum must fail PWR_Z dropout")

        if args.rust_output is not None:
            rust = json.loads(args.rust_output.read_text(encoding="utf-8"))
            compare_rust(rust["baseline"], expected)
            compare_rust(rust["six_channel_failure"], joint_expected)
            mode = "rust_python_boundary=pass"
        else:
            mode = "rust_python_boundary=not_run"
        print(
            "correlated_sensor_information_oracle=pass "
            f"baseline_sensors={expected['sensor_candidate_count']} "
            f"joint_sensors={joint_expected['sensor_candidate_count']} "
            f"joint_robust_D={','.join(joint_expected['robust_correlated_d_opt']['sensor_ids'])} "
            f"joint_robust_E={','.join(joint_expected['robust_correlated_e_opt']['sensor_ids'])} "
            f"{mode}"
        )
    except (OSError, json.JSONDecodeError, AuditError, KeyError, TypeError, ValueError) as error:
        print(f"correlated_sensor_oracle_error={error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
