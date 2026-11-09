#!/usr/bin/env python3
"""Independent standard-library oracle for the Tier-0 observability fixture."""

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
FIXTURE = HERE / "fixtures" / "known-answer-observability-v0.1.json"
RANK_REL_TOL = 1.0e-12
EIGEN_REL_TOL = 1.0e-12
CHOLESKY_REL_TOL = 1.0e-14


class AuditError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AuditError(message)


def validate(data: dict[str, Any]) -> None:
    require(data.get("schema_version") == "aero-inspection-observability/0.1.0",
            "unsupported schema_version")
    require(isinstance(data.get("model_id"), str) and data["model_id"].strip(),
            "model_id must be non-empty")
    require(data.get("model_scope") == "synthetic_local_linear_observation",
            "unsupported model scope")
    require(data.get("noise_model") == "independent_variance_per_sensor",
            "unsupported noise model")

    parameters = data.get("parameters")
    sensors = data.get("sensors")
    require(isinstance(parameters, list) and 0 < len(parameters) <= 8,
            "parameter count must be in 1..=8")
    require(isinstance(sensors, list) and 0 < len(sensors) <= 16,
            "candidate sensor count must be in 1..=16")
    select_count = data.get("select_count")
    require(isinstance(select_count, int) and 0 < select_count <= len(sensors),
            "select_count outside candidate set")

    parameter_ids: set[str] = set()
    for parameter in parameters:
        require(isinstance(parameter, dict), "parameter must be an object")
        identifier = parameter.get("id")
        scale = parameter.get("scale")
        require(isinstance(identifier, str) and identifier.strip(),
                "parameter ID must be non-empty")
        require(identifier not in parameter_ids, "duplicate parameter ID")
        parameter_ids.add(identifier)
        require(isinstance(scale, (int, float)) and math.isfinite(scale) and scale > 0.0,
                "parameter scale must be finite and positive")

    sensor_ids: set[str] = set()
    dimension = len(parameters)
    for sensor in sensors:
        require(isinstance(sensor, dict), "sensor must be an object")
        identifier = sensor.get("id")
        row = sensor.get("observation_row")
        variance = sensor.get("variance")
        require(isinstance(identifier, str) and identifier.strip(),
                "sensor ID must be non-empty")
        require(identifier not in sensor_ids, "duplicate sensor ID")
        sensor_ids.add(identifier)
        require(isinstance(row, list) and len(row) == dimension,
                "observation row dimension mismatch")
        require(all(isinstance(value, (int, float)) and math.isfinite(value) for value in row),
                "observation row must contain finite values")
        require(isinstance(variance, (int, float)) and math.isfinite(variance) and variance > 0.0,
                "variance must be finite and positive")


def matrix_rank(rows: list[list[float]]) -> int:
    if not rows or not rows[0]:
        return 0
    work = [row[:] for row in rows]
    row_count = len(work)
    column_count = len(work[0])
    maximum = max((abs(value) for row in work for value in row), default=0.0)
    tolerance = RANK_REL_TOL * max(1.0, maximum)
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


def jacobi_eigenvalues(matrix: list[list[float]]) -> list[float]:
    size = len(matrix)
    require(size > 0 and all(len(row) == size for row in matrix),
            "eigenvalue matrix must be square")
    work = [row[:] for row in matrix]
    maximum_diagonal = max(abs(work[index][index]) for index in range(size))
    tolerance = EIGEN_REL_TOL * max(1.0, maximum_diagonal)
    converged = size == 1

    for _ in range(max(1, 100 * size * size)):
        p = q = 0
        largest = 0.0
        for i in range(size):
            for j in range(i + 1, size):
                if abs(work[i][j]) > largest:
                    largest = abs(work[i][j])
                    p, q = i, j
        if largest <= tolerance:
            converged = True
            break

        apq = work[p][q]
        if apq == 0.0:
            continue
        tau = (work[q][q] - work[p][p]) / (2.0 * apq)
        if tau >= 0.0:
            t = 1.0 / (tau + math.sqrt(1.0 + tau * tau))
        else:
            t = -1.0 / (-tau + math.sqrt(1.0 + tau * tau))
        cosine = 1.0 / math.sqrt(1.0 + t * t)
        sine = t * cosine
        app = work[p][p]
        aqq = work[q][q]
        work[p][p] = app - t * apq
        work[q][q] = aqq + t * apq
        work[p][q] = work[q][p] = 0.0

        for k in range(size):
            if k in (p, q):
                continue
            akp = work[k][p]
            akq = work[k][q]
            work[k][p] = cosine * akp - sine * akq
            work[p][k] = work[k][p]
            work[k][q] = sine * akp + cosine * akq
            work[q][k] = work[k][q]

    require(converged, "Jacobi eigenvalue iteration did not converge")
    values = sorted(work[index][index] for index in range(size))
    require(all(math.isfinite(value) for value in values),
            "non-finite Fisher eigenvalue")
    scale = max(1.0, *(abs(value) for value in values))
    require(values[0] >= -1.0e-10 * scale,
            "Fisher information is not positive semidefinite")
    if values[0] < 0.0:
        values[0] = 0.0
    return values


def cholesky_logdet(matrix: list[list[float]]) -> float | None:
    size = len(matrix)
    maximum_diagonal = max(abs(matrix[index][index]) for index in range(size))
    tolerance = CHOLESKY_REL_TOL * max(1.0, maximum_diagonal)
    lower = [[0.0] * size for _ in range(size)]
    logdet = 0.0

    for i in range(size):
        for j in range(i + 1):
            value = matrix[i][j] - sum(lower[i][k] * lower[j][k] for k in range(j))
            if i == j:
                if not math.isfinite(value) or value <= tolerance:
                    return None
                lower[i][j] = math.sqrt(value)
                logdet += 2.0 * math.log(lower[i][j])
            else:
                if lower[j][j] <= 0.0:
                    return None
                lower[i][j] = value / lower[j][j]
    return logdet if math.isfinite(logdet) else None


def fisher_matrix(rows: list[list[float]], variances: list[float],
                  dimension: int) -> list[list[float]]:
    fisher = [[0.0] * dimension for _ in range(dimension)]
    for row, variance in zip(rows, variances):
        weight = 1.0 / variance
        for i in range(dimension):
            for j in range(dimension):
                fisher[i][j] += row[i] * row[j] * weight
    require(all(math.isfinite(value) for row in fisher for value in row),
            "Fisher information overflowed")
    return fisher


def select_best(subsets: list[dict[str, Any]], use_logdet: bool) -> dict[str, Any] | None:
    best = None
    for subset in subsets:
        if use_logdet:
            if subset["nullity"] != 0 or subset["log_determinant"] is None:
                continue
            objective = subset["log_determinant"]
        else:
            objective = subset["minimum_eigenvalue"]
        candidate = {
            "sensor_ids": subset["sensor_ids"],
            "rank": subset["rank"],
            "nullity": subset["nullity"],
            "objective_value": objective,
        }
        if best is None:
            best = candidate
            continue
        delta = candidate["objective_value"] - best["objective_value"]
        if delta > 1.0e-12 or (
            abs(delta) <= 1.0e-12 and candidate["sensor_ids"] < best["sensor_ids"]
        ):
            best = candidate
    return best


def evaluate(data: dict[str, Any], raw: bytes) -> dict[str, Any]:
    validate(data)
    scales = [parameter["scale"] for parameter in data["parameters"]]
    sensors = sorted(data["sensors"], key=lambda item: item["id"])
    dimension = len(data["parameters"])
    subsets: list[dict[str, Any]] = []

    for chosen in itertools.combinations(sensors, data["select_count"]):
        sensor_ids = [sensor["id"] for sensor in chosen]
        rows = [
            [sensitivity * scale
             for sensitivity, scale in zip(sensor["observation_row"], scales)]
            for sensor in chosen
        ]
        require(all(math.isfinite(value) for row in rows for value in row),
                "normalized observation row overflowed")
        variances = [sensor["variance"] for sensor in chosen]
        rank = matrix_rank(rows)
        fisher = fisher_matrix(rows, variances, dimension)
        eigenvalues = jacobi_eigenvalues(fisher)
        logdet = cholesky_logdet(fisher) if rank == dimension else None
        subsets.append({
            "sensor_ids": sensor_ids,
            "rank": rank,
            "nullity": dimension - rank,
            "fisher_information": fisher,
            "eigenvalues_ascending": eigenvalues,
            "minimum_eigenvalue": eigenvalues[0],
            "log_determinant": logdet,
        })

    return {
        "schema_version": data["schema_version"],
        "model_id": data["model_id"],
        "model_scope": data["model_scope"],
        "noise_model": data["noise_model"],
        "source_sha256": hashlib.sha256(raw).hexdigest(),
        "parameter_ids": [parameter["id"] for parameter in data["parameters"]],
        "parameter_scales": scales,
        "candidate_sensor_count": len(sensors),
        "select_count": data["select_count"],
        "subset_count": len(subsets),
        "full_rank_subset_count": sum(item["nullity"] == 0 for item in subsets),
        "d_optimal_selection": select_best(subsets, True),
        "e_optimal_selection": select_best(subsets, False),
        "subsets": subsets,
        "rank_relative_tolerance": RANK_REL_TOL,
        "eigen_relative_tolerance": EIGEN_REL_TOL,
        "claim_ceiling": "synthetic_local_observability_only",
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
    candidate["sensors"][0]["variance"] = 0.0
    cases.append(("zero_variance", candidate))

    candidate = copy.deepcopy(data)
    candidate["sensors"][0]["observation_row"].pop()
    cases.append(("observation_dimension_mismatch", candidate))

    candidate = copy.deepcopy(data)
    candidate["parameters"][1]["id"] = candidate["parameters"][0]["id"]
    cases.append(("duplicate_parameter_id", candidate))

    candidate = copy.deepcopy(data)
    candidate["select_count"] = 99
    cases.append(("selection_exceeds_candidates", candidate))

    candidate = copy.deepcopy(data)
    candidate["sensors"][0]["observation_row"][0] = float("nan")
    cases.append(("nonfinite_observation", candidate))

    for label, candidate in cases:
        try:
            validate(candidate)
        except AuditError:
            print(f"negative_control=pass case={label}")
        else:
            raise AuditError(f"negative control unexpectedly passed: {label}")

    candidate = copy.deepcopy(data)
    for sensor in candidate["sensors"]:
        sensor["observation_row"] = [1.0, 0.0]
    result = evaluate(candidate, b"rank-deficient-control")
    require(result["full_rank_subset_count"] == 0,
            "rank-deficient control was promoted")
    require(result["d_optimal_selection"] is None,
            "D-optimum incorrectly exists for rank-deficient control")
    require(result["e_optimal_selection"]["rank"] == 1
            and result["e_optimal_selection"]["nullity"] == 1,
            "E-optimum hid unobservable parameter")
    print("negative_control=pass case=rank_deficiency_remains_visible")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-output", type=Path, required=True)
    args = parser.parse_args()
    try:
        raw = FIXTURE.read_bytes()
        data = json.loads(raw)
        expected = evaluate(data, raw)
        require(expected["subset_count"] == 6, "known-answer subset count changed")
        require(expected["full_rank_subset_count"] == 6, "known-answer rank count changed")
        require(expected["d_optimal_selection"]["sensor_ids"] == ["S_A", "S_C"],
                "D-optimal known answer changed")
        require(expected["e_optimal_selection"]["sensor_ids"] == ["S_A", "S_C"],
                "E-optimal known answer changed")
        ac = next(item for item in expected["subsets"]
                  if item["sensor_ids"] == ["S_A", "S_C"])
        require(ac["fisher_information"] == [[2.0, 1.0], [1.0, 1.0]],
                "known-answer Fisher matrix changed")
        actual = json.loads(args.rust_output.read_text(encoding="utf-8"))
        compare(actual, expected)
        hostile_controls(data)
        print("aero_inspection_observability_oracle=pass "
              f"subsets={expected['subset_count']} "
              f"full_rank={expected['full_rank_subset_count']} "
              "d_opt=AC e_opt=AC rust_python=pass")
        return 0
    except (OSError, json.JSONDecodeError, AuditError, KeyError, TypeError, ValueError) as error:
        print(f"observability_oracle_error={error}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
