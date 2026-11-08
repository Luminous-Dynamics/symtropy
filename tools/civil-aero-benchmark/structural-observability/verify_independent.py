#!/usr/bin/env python3
"""Independent Python oracle for synthetic structural observability design.

This implementation does not import the Rust crate. It independently forms
Fisher-information matrices, enumerates sensor subsets, evaluates sensor-dropout
scenarios and checks the Rust JSON output against the frozen known-answer case.

The fixture is dimensionless and synthetic. No physical aircraft sensor layout,
calibration, health-monitoring performance or certification claim is made.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import itertools
import json
import math
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
FIXTURE = HERE / "data" / "synthetic-wing-observability-v0.1.json"
RANK_TOLERANCE = 1.0e-10
SCORE_TOLERANCE = 1.0e-12
CLAIM_CEILING = "synthetic_linear_fisher_observability_only"


class AuditError(ValueError):
    """Raised when a fixture or observability result violates its contract."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AuditError(message)


def validate_fixture(fixture: dict[str, Any]) -> None:
    require(
        fixture.get("schema_version") == "aero-structural-observability-fixture/0.1.0",
        "unsupported fixture schema",
    )
    require(fixture.get("claim_ceiling") == CLAIM_CEILING, "claim ceiling changed")
    require(bool(fixture.get("problem_id")), "problem_id missing")

    assumptions = fixture.get("model_assumptions", {})
    require(assumptions.get("physical_aircraft_model") is False, "synthetic fixture was promoted to physical aircraft")
    require(len(assumptions.get("parameter_names", [])) == 2, "v0.1 requires two target parameters")
    require(assumptions.get("observation_model") == "linearized_static_sensitivity", "unsupported observation model")
    require(
        assumptions.get("noise_model") == "independent_zero_mean_gaussian_sensor_noise",
        "unsupported noise covariance assumption",
    )
    require(assumptions.get("sensitivity_units") == "dimensionless_normalized", "sensitivity units changed")
    require(
        assumptions.get("noise_standard_uncertainty_units") == "dimensionless_normalized",
        "noise uncertainty units changed",
    )
    require(assumptions.get("calibration_model") == "synthetic_reference_only", "fixture calibration model changed")

    constraints = fixture.get("constraints", {})
    require(constraints.get("d_opt_sensor_count", 0) > 0, "D-opt sensor count must be positive")
    require(constraints.get("e_opt_sensor_count", 0) > 0, "E-opt sensor count must be positive")
    require(
        0 <= constraints.get("sensor_dropout_budget", -1) < constraints.get("robust_e_opt_sensor_count", 0),
        "dropout budget is invalid",
    )
    budget = constraints.get("maximum_total_cost_units")
    require(isinstance(budget, (int, float)) and math.isfinite(budget) and budget > 0, "cost budget invalid")

    sensors = fixture.get("sensors")
    require(isinstance(sensors, list) and len(sensors) >= 3, "at least three sensors are required")
    ids = [sensor.get("sensor_id") for sensor in sensors]
    require(all(isinstance(value, str) and value for value in ids), "sensor IDs must be non-empty")
    require(len(ids) == len(set(ids)), "duplicate sensor IDs")
    require(ids == sorted(ids), "sensor candidate list must be sorted by ID")

    for sensor in sensors:
        sid = sensor["sensor_id"]
        require(bool(sensor.get("sensor_type")), f"{sid}: sensor_type missing")
        for coordinate in ("span_eta", "chord_x_over_c"):
            value = sensor.get(coordinate)
            require(
                isinstance(value, (int, float)) and math.isfinite(value) and 0.0 <= value <= 1.0,
                f"{sid}: {coordinate} outside normalized domain",
            )
        sensitivity = sensor.get("sensitivity")
        require(
            isinstance(sensitivity, list)
            and len(sensitivity) == 2
            and all(isinstance(value, (int, float)) and math.isfinite(value) for value in sensitivity),
            f"{sid}: invalid sensitivity vector",
        )
        uncertainty = sensor.get("noise_standard_uncertainty")
        require(
            isinstance(uncertainty, (int, float))
            and math.isfinite(uncertainty)
            and uncertainty > 0,
            f"{sid}: noise standard uncertainty must be finite and positive",
        )
        require(sensor.get("calibration_admitted") is True, f"{sid}: calibration is not admitted")
        require(bool(sensor.get("calibration_reference_id")), f"{sid}: calibration reference missing")
        for field in ("deployment_cost_units",):
            value = sensor.get(field)
            require(
                isinstance(value, (int, float)) and math.isfinite(value) and value > 0,
                f"{sid}: {field} must be finite and positive",
            )
        for field in ("mass_g", "power_mw"):
            value = sensor.get(field)
            require(
                isinstance(value, (int, float)) and math.isfinite(value) and value >= 0,
                f"{sid}: {field} must be finite and non-negative",
            )

    for field in ("d_opt_sensor_count", "e_opt_sensor_count", "robust_e_opt_sensor_count"):
        require(0 < constraints[field] <= len(sensors), f"{field} outside candidate count")


def information_metrics(sensors: list[dict[str, Any]]) -> dict[str, Any]:
    f11 = 0.0
    f12 = 0.0
    f22 = 0.0
    for sensor in sensors:
        h1, h2 = sensor["sensitivity"]
        variance = sensor["noise_standard_uncertainty"] ** 2
        weight = 1.0 / variance
        f11 += h1 * h1 * weight
        f12 += h1 * h2 * weight
        f22 += h2 * h2 * weight

    determinant = f11 * f22 - f12 * f12
    discriminant = math.sqrt(max(0.0, (f11 - f22) ** 2 + 4.0 * f12**2))
    trace = f11 + f22
    minimum = max(0.0, (trace - discriminant) / 2.0)
    maximum = max(0.0, (trace + discriminant) / 2.0)
    rank = int(minimum > RANK_TOLERANCE) + int(maximum > RANK_TOLERANCE)

    return {
        "fisher_information": [[f11, f12], [f12, f22]],
        "determinant": determinant,
        "log_determinant": math.log(determinant) if determinant > 0 else None,
        "eigenvalues_ascending": [minimum, maximum],
        "effective_rank": rank,
    }


def totals(sensors: list[dict[str, Any]]) -> tuple[float, float, float]:
    return (
        sum(sensor["deployment_cost_units"] for sensor in sensors),
        sum(sensor["mass_g"] for sensor in sensors),
        sum(sensor["power_mw"] for sensor in sensors),
    )


def survivor_worst_case(
    selected: list[dict[str, Any]], dropout_budget: int
) -> tuple[float, dict[str, Any]]:
    require(0 <= dropout_budget < len(selected), "dropout budget invalid")
    scenarios = []
    for survivors in itertools.combinations(selected, len(selected) - dropout_budget):
        survivor_list = list(survivors)
        survivors_ids = {sensor["sensor_id"] for sensor in survivor_list}
        dropped_ids = sorted(
            sensor["sensor_id"]
            for sensor in selected
            if sensor["sensor_id"] not in survivors_ids
        )
        metrics = information_metrics(survivor_list)
        scenarios.append(
            {
                "dropped_sensor_id": dropped_ids[0] if dropped_ids else "NONE",
                "surviving_sensor_ids": [sensor["sensor_id"] for sensor in survivor_list],
                "minimum_eigenvalue": metrics["eigenvalues_ascending"][0],
                "effective_rank": metrics["effective_rank"],
            }
        )
    worst = min(
        scenarios,
        key=lambda item: (item["minimum_eigenvalue"], item["dropped_sensor_id"]),
    )
    return worst["minimum_eigenvalue"], worst


def selection_record(
    selected: list[dict[str, Any]],
    objective: str,
    objective_value: float,
    worst_case: dict[str, Any] | None = None,
) -> dict[str, Any]:
    cost, mass, power = totals(selected)
    return {
        "objective": objective,
        "selected_sensor_ids": [sensor["sensor_id"] for sensor in selected],
        "objective_value": objective_value,
        "information": information_metrics(selected),
        "total_cost_units": cost,
        "total_mass_g": mass,
        "total_power_mw": power,
        "worst_case_dropout": worst_case,
    }


def best_selection(
    fixture: dict[str, Any],
    sensor_count: int,
    objective: str,
) -> dict[str, Any]:
    constraints = fixture["constraints"]
    sensors = fixture["sensors"]
    best_score = -math.inf
    best_selection_result: dict[str, Any] | None = None

    for combo in itertools.combinations(sensors, sensor_count):
        selected = list(combo)
        cost, _, _ = totals(selected)
        if cost > constraints["maximum_total_cost_units"] + SCORE_TOLERANCE:
            continue

        worst_case = None
        if objective == "D_OPTIMAL_LOG_DETERMINANT":
            metrics = information_metrics(selected)
            score = metrics["log_determinant"] if metrics["log_determinant"] is not None else -math.inf
        elif objective == "E_OPTIMAL_MINIMUM_EIGENVALUE":
            metrics = information_metrics(selected)
            score = metrics["eigenvalues_ascending"][0]
        elif objective == "DROPOUT_ROBUST_E_OPTIMAL_MINIMUM_EIGENVALUE":
            score, worst_case = survivor_worst_case(selected, constraints["sensor_dropout_budget"])
        else:
            raise AuditError(f"unsupported objective {objective}")

        candidate_ids = [sensor["sensor_id"] for sensor in selected]
        previous_ids = (
            best_selection_result["selected_sensor_ids"]
            if best_selection_result is not None
            else None
        )
        better = score > best_score + SCORE_TOLERANCE
        tied_and_lexically_earlier = (
            abs(score - best_score) <= SCORE_TOLERANCE
            and (previous_ids is None or candidate_ids < previous_ids)
        )
        if better or tied_and_lexically_earlier:
            best_score = score
            best_selection_result = selection_record(selected, objective, score, worst_case)

    require(best_selection_result is not None, f"no feasible {objective} selection")
    require(math.isfinite(best_score), f"no full-information {objective} selection; no regularization permitted")
    return best_selection_result


def independent_report(fixture: dict[str, Any], fixture_bytes: bytes) -> dict[str, Any]:
    validate_fixture(fixture)
    constraints = fixture["constraints"]
    report = {
        "schema_version": "aero-structural-observability-audit/0.1.0",
        "problem_id": fixture["problem_id"],
        "fixture_sha256": hashlib.sha256(fixture_bytes).hexdigest(),
        "model_assumptions": fixture["model_assumptions"],
        "d_optimal": best_selection(
            fixture, constraints["d_opt_sensor_count"], "D_OPTIMAL_LOG_DETERMINANT"
        ),
        "e_optimal": best_selection(
            fixture, constraints["e_opt_sensor_count"], "E_OPTIMAL_MINIMUM_EIGENVALUE"
        ),
        "dropout_robust_e_optimal": best_selection(
            fixture,
            constraints["robust_e_opt_sensor_count"],
            "DROPOUT_ROBUST_E_OPTIMAL_MINIMUM_EIGENVALUE",
        ),
        "physical_aircraft_model": False,
        "aircraft_feasibility": "NOT_ASSESSED",
        "claim_ceiling": CLAIM_CEILING,
    }

    expected = fixture["expected"]
    require(report["d_optimal"]["selected_sensor_ids"] == expected["d_opt_sensor_ids"],
            "D-optimal fixture answer changed")
    require(close(report["d_optimal"]["information"]["determinant"], expected["d_opt_determinant"]),
            "D-optimal determinant differs from frozen answer")
    require(report["e_optimal"]["selected_sensor_ids"] == expected["e_opt_sensor_ids"],
            "E-optimal fixture answer changed")
    require(close(report["e_optimal"]["information"]["eigenvalues_ascending"][0], expected["e_opt_minimum_eigenvalue"]),
            "E-optimal minimum eigenvalue differs from frozen answer")
    require(report["dropout_robust_e_optimal"]["selected_sensor_ids"] == expected["robust_e_opt_sensor_ids"],
            "dropout-robust E-optimal fixture answer changed")
    require(close(report["dropout_robust_e_optimal"]["objective_value"], expected["robust_e_opt_worst_case_minimum_eigenvalue"]),
            "robust E-optimal worst-case score differs from frozen answer")
    return report


def close(actual: float, expected: float, tolerance: float = 1.0e-10) -> bool:
    return (
        isinstance(actual, (int, float))
        and math.isfinite(actual)
        and math.isfinite(expected)
        and abs(actual - expected) <= tolerance * max(1.0, abs(expected))
    )


def compare_recursive(actual: Any, expected: Any, path: str = "$") -> None:
    if isinstance(expected, dict):
        require(isinstance(actual, dict) and set(actual) == set(expected), f"{path}: object keys differ")
        for key in expected:
            compare_recursive(actual[key], expected[key], f"{path}.{key}")
    elif isinstance(expected, list):
        require(isinstance(actual, list) and len(actual) == len(expected), f"{path}: array length differs")
        for index, (left, right) in enumerate(zip(actual, expected)):
            compare_recursive(left, right, f"{path}[{index}]")
    elif isinstance(expected, float):
        require(close(actual, expected), f"{path}: numeric mismatch {actual!r} != {expected!r}")
    else:
        require(actual == expected, f"{path}: value mismatch {actual!r} != {expected!r}")


def expect_rejected(label: str, action) -> None:
    try:
        action()
    except (AuditError, KeyError, TypeError, ValueError):
        print(f"negative_control=pass case={label}")
        return
    raise AuditError(f"negative control unexpectedly passed: {label}")


def hostile_controls(fixture: dict[str, Any]) -> None:
    bad = copy.deepcopy(fixture)
    bad["sensors"][1]["sensor_id"] = bad["sensors"][0]["sensor_id"]
    expect_rejected("duplicate_sensor_id", lambda: validate_fixture(bad))

    bad = copy.deepcopy(fixture)
    bad["sensors"][0]["noise_standard_uncertainty"] = 0.0
    expect_rejected("zero_sensor_uncertainty", lambda: validate_fixture(bad))

    bad = copy.deepcopy(fixture)
    bad["sensors"][0]["calibration_admitted"] = False
    expect_rejected("unadmitted_sensor_calibration", lambda: validate_fixture(bad))

    bad = copy.deepcopy(fixture)
    bad["sensors"][0]["span_eta"] = 1.01
    expect_rejected("sensor_outside_normalized_span", lambda: validate_fixture(bad))

    bad = copy.deepcopy(fixture)
    bad["model_assumptions"]["physical_aircraft_model"] = True
    expect_rejected("synthetic_fixture_promoted_to_physical_aircraft", lambda: validate_fixture(bad))

    bad = copy.deepcopy(fixture)
    bad["constraints"]["maximum_total_cost_units"] = 1.0
    expect_rejected("budget_infeasible_sensor_count", lambda: independent_report(bad, b"mutated"))

    collinear = [
        {"sensor_id": "A", "sensitivity": [1.0, 0.0], "noise_standard_uncertainty": 1.0},
        {"sensor_id": "B", "sensitivity": [2.0, 0.0], "noise_standard_uncertainty": 2.0},
    ]
    metrics = information_metrics(collinear)
    require(metrics["effective_rank"] == 1, "collinear sensors must remain rank deficient")
    require(metrics["log_determinant"] is None, "rank-deficient design must not receive a finite logdet")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-output", type=Path)
    args = parser.parse_args()

    try:
        fixture_bytes = FIXTURE.read_bytes()
        fixture = json.loads(fixture_bytes.decode("utf-8"))
        expected = independent_report(fixture, fixture_bytes)
        hostile_controls(fixture)

        if args.rust_output is not None:
            actual = json.loads(args.rust_output.read_text(encoding="utf-8"))
            compare_recursive(actual, expected)
            boundary = "rust_python_boundary=pass"
        else:
            boundary = "rust_python_boundary=not_run"

        print(
            "structural_observability_oracle=pass "
            f"candidates={len(fixture['sensors'])} "
            f"d_opt={','.join(expected['d_optimal']['selected_sensor_ids'])} "
            f"e_opt={','.join(expected['e_optimal']['selected_sensor_ids'])} "
            f"robust_e_opt={','.join(expected['dropout_robust_e_optimal']['selected_sensor_ids'])} "
            f"claim_ceiling={CLAIM_CEILING} "
            f"{boundary}"
        )
    except (OSError, json.JSONDecodeError, AuditError, KeyError, TypeError, ValueError) as error:
        print(f"structural_observability_error={error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
