#!/usr/bin/env python3
"""Independent Python oracle for AER-01 reduced-order dynamic coupling audits.

This module does not import Rust implementation code and does not solve CFD/FEM.
It checks a scalar added-mass fixed point and the exact time-window / energy /
interface transaction contract represented by the frozen JSON fixture.
"""

from __future__ import annotations

import argparse
import copy
import json
import math
import re
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
FIXTURE = HERE / "data" / "known-answer-cases-v0.1.0.json"
DIGEST = re.compile(r"^[0-9a-f]{64}$")


class AuditError(ValueError):
    """Raised for invalid or unsupported coupling evidence."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AuditError(message)


def finite(value: Any, label: str) -> float:
    require(isinstance(value, (int, float)) and math.isfinite(float(value)), f"{label} must be finite")
    return float(value)


def positive(value: Any, label: str) -> float:
    result = finite(value, label)
    require(result > 0.0, f"{label} must be positive")
    return result


def digest(value: Any, label: str) -> str:
    require(isinstance(value, str) and DIGEST.fullmatch(value) is not None, f"{label} must be lowercase SHA-256")
    return value


def bound(abs_tol: float, rel_tol: float, scale: float) -> float:
    return abs_tol + rel_tol * max(1.0, abs(scale))


def added_mass_oracle(case: dict[str, Any]) -> dict[str, Any]:
    structural_mass = positive(case["structural_mass_kg"], "structural_mass_kg")
    added_mass = finite(case["fluid_added_mass_kg"], "fluid_added_mass_kg")
    require(added_mass >= 0.0, "fluid_added_mass_kg cannot be negative")
    force = finite(case["applied_force_n"], "applied_force_n")
    acceleration = finite(case["initial_acceleration_m_per_s2"], "initial_acceleration")
    relaxation = positive(case["relaxation"], "relaxation")
    require(relaxation <= 1.0, "relaxation must be in (0, 1]")
    atol = positive(case["absolute_residual_tolerance_n"], "absolute residual tolerance")
    rtol = finite(case["relative_residual_tolerance"], "relative residual tolerance")
    require(rtol >= 0.0, "relative residual tolerance cannot be negative")
    max_iterations = case["max_iterations"]
    require(isinstance(max_iterations, int) and max_iterations > 0, "max_iterations must be positive")

    total_mass = structural_mass + added_mass
    exact = force / total_mass
    amplification = 1.0 - relaxation * (1.0 + added_mass / structural_mass)
    tolerance = bound(atol, rtol, force)

    for iteration in range(1, max_iterations + 1):
        unrelaxed = (force - added_mass * acceleration) / structural_mass
        acceleration = acceleration + relaxation * (unrelaxed - acceleration)
        residual = abs(total_mass * acceleration - force)
        if math.isfinite(acceleration) and residual <= tolerance:
            return {
                "status": "CONVERGED",
                "exact_acceleration_m_per_s2": exact,
                "acceleration_m_per_s2": acceleration,
                "last_iterate_m_per_s2": acceleration,
                "amplification_factor": amplification,
                "residual_n": residual,
                "iterations": iteration,
            }
        if not math.isfinite(acceleration):
            return {
                "status": "NON_CONVERGED",
                "exact_acceleration_m_per_s2": exact,
                "acceleration_m_per_s2": None,
                "last_iterate_m_per_s2": None,
                "amplification_factor": amplification,
                "residual_n": None,
                "iterations": iteration,
            }

    residual = abs(total_mass * acceleration - force)
    return {
        "status": "NON_CONVERGED",
        "exact_acceleration_m_per_s2": exact,
        "acceleration_m_per_s2": None,
        "last_iterate_m_per_s2": acceleration,
        "amplification_factor": amplification,
        "residual_n": residual,
        "iterations": max_iterations,
    }


def validate_grid(grid: dict[str, Any], start: int, end: int, label: str) -> int:
    require(grid["start_tick"] == start and grid["end_tick"] == end, f"{label} grid does not cover the exact window")
    step = grid["step_ticks"]
    require(isinstance(step, int) and step > 0, f"{label} step_ticks must be positive integer")
    duration = end - start
    require(duration > 0 and duration % step == 0, f"{label} grid has a substep gap or incomplete final step")
    return duration // step


def validate_window_identity(window: dict[str, Any]) -> None:
    for key in (
        "timebase_sha256",
        "configuration_sha256",
        "geometry_sha256",
        "loading_state_sha256",
        "mission_phase_sha256",
        "environment_sha256",
        "fluid_model_sha256",
        "structure_model_sha256",
        "interface_map_sha256",
        "temporal_operator_sha256",
        "input_state_sha256",
        "proposed_output_state_sha256",
    ):
        digest(window[key], key)

    require(isinstance(window["window_id"], str) and window["window_id"].strip(), "window_id must be non-empty")
    start, end = window["start_tick"], window["end_tick"]
    require(isinstance(start, int) and isinstance(end, int) and end > start, "invalid synchronization interval")
    require(window["temporal_interpolation"] in {"linear", "endpoint-hold"}, "unsupported temporal interpolation")
    require(window["extrapolation_allowed"] is False, "temporal extrapolation not admitted")

    exchange = window["exchange"]
    for key in ("exchange_id", "timebase_sha256", "input_state_sha256", "interface_map_sha256"):
        digest(exchange[key], "exchange." + key)
    require(exchange["window_id"] == window["window_id"], "exchange window identity mismatch")
    require(exchange["timebase_sha256"] == window["timebase_sha256"], "exchange timebase identity mismatch")
    require(exchange["input_state_sha256"] == window["input_state_sha256"], "exchange input-state identity mismatch")
    require(exchange["interface_map_sha256"] == window["interface_map_sha256"], "exchange map identity mismatch")
    require(exchange["start_tick"] == start and exchange["end_tick"] == end, "exchange interval mismatch")


def audit_window(window: dict[str, Any]) -> dict[str, Any]:
    validate_window_identity(window)
    start, end = window["start_tick"], window["end_tick"]
    fluid_steps = validate_grid(window["fluid_grid"], start, end, "fluid")
    structure_steps = validate_grid(window["structure_grid"], start, end, "structure")

    iterations = window["coupling_iterations"]
    maximum = window["max_coupling_iterations"]
    require(isinstance(iterations, int) and isinstance(maximum, int), "coupling iteration counts must be integers")
    require(maximum > 0 and 0 < iterations <= maximum, "coupling iteration budget invalid")

    coupling_tolerance = positive(window["coupling_tolerance"], "coupling_tolerance")
    inner_tolerance = positive(window["inner_tolerance"], "inner_tolerance")
    tightening = positive(window["minimum_inner_tightening_factor"], "minimum_inner_tightening_factor")
    coupling_residual = finite(window["coupling_residual"], "coupling_residual")
    fluid_inner = finite(window["fluid_inner_residual"], "fluid_inner_residual")
    structure_inner = finite(window["structure_inner_residual"], "structure_inner_residual")
    require(min(coupling_residual, fluid_inner, structure_inner) >= 0.0, "residual magnitudes cannot be negative")

    energy = window["energy"]
    for key, value in energy.items():
        finite(value, "energy." + key)
    require(min(
        energy["initial_fluid_energy_j"],
        energy["initial_structure_energy_j"],
        energy["final_fluid_energy_j"],
        energy["final_structure_energy_j"],
        energy["declared_dissipation_j"],
    ) >= 0.0, "stored energy and dissipation cannot be negative")

    exchange = window["exchange"]
    fluid_impulse = [finite(x, "fluid impulse") for x in exchange["fluid_impulse_ns"]]
    structure_impulse = [finite(x, "structure impulse") for x in exchange["structure_impulse_ns"]]
    require(len(fluid_impulse) == 3 and len(structure_impulse) == 3, "impulse must be a 3-vector")
    impulse_residual = max(abs(fluid_impulse[i] + structure_impulse[i]) for i in range(3))
    fluid_work = finite(exchange["fluid_interface_work_j"], "fluid_interface_work_j")
    structure_work = finite(exchange["structure_interface_work_j"], "structure_interface_work_j")
    work_residual = abs(fluid_work + structure_work)

    initial_total = energy["initial_fluid_energy_j"] + energy["initial_structure_energy_j"]
    final_total = energy["final_fluid_energy_j"] + energy["final_structure_energy_j"]
    observed_change = final_total - initial_total
    expected_change = energy["external_work_j"] - energy["declared_dissipation_j"]
    energy_residual = abs(observed_change - expected_change)

    tolerance = window["tolerance"]
    for key in ("impulse_abs_ns", "work_abs_j", "energy_abs_j"):
        positive(tolerance[key], "tolerance." + key)
    relative = finite(tolerance["relative"], "tolerance.relative")
    require(relative >= 0.0, "relative tolerance cannot be negative")

    errors: list[str] = []
    if not window["coupling_converged"]:
        errors.append("coupling_not_converged")
    if coupling_residual > bound(coupling_tolerance, relative, 1.0):
        errors.append("coupling_residual_exceeds_tolerance")
    if tightening * inner_tolerance > coupling_tolerance:
        errors.append("inner_tolerance_not_stricter_than_coupling_policy")
    if fluid_inner > bound(inner_tolerance, relative, 1.0):
        errors.append("fluid_inner_residual_exceeds_tolerance")
    if structure_inner > bound(inner_tolerance, relative, 1.0):
        errors.append("structure_inner_residual_exceeds_tolerance")
    if impulse_residual > bound(tolerance["impulse_abs_ns"], relative, max(map(abs, fluid_impulse + structure_impulse))):
        errors.append("interface_impulse_not_action_reaction_balanced")
    if work_residual > bound(tolerance["work_abs_j"], relative, max(abs(fluid_work), abs(structure_work))):
        errors.append("interface_work_not_balanced")
    if energy_residual > bound(tolerance["energy_abs_j"], relative, max(abs(observed_change), abs(expected_change))):
        errors.append("whole_window_energy_balance_failed")

    status = "COMMITTED" if not errors else "REJECTED"
    committed_state = window["proposed_output_state_sha256"] if status == "COMMITTED" else window["input_state_sha256"]
    return {
        "status": status,
        "commit_allowed": status == "COMMITTED",
        "window_id": window["window_id"],
        "exchange_id": exchange["exchange_id"],
        "fluid_substeps": fluid_steps,
        "structure_substeps": structure_steps,
        "impulse_residual_ns": impulse_residual,
        "interface_work_residual_j": work_residual,
        "energy_residual_j": energy_residual,
        "committed_state_sha256": committed_state,
        "errors": errors,
        "configuration_sha256": window["configuration_sha256"],
        "timebase_sha256": window["timebase_sha256"],
        "input_state_sha256": window["input_state_sha256"],
        "start_tick": start,
        "end_tick": end,
    }


def audit_sequence(results: list[dict[str, Any]]) -> None:
    require(bool(results), "committed sequence cannot be empty")
    window_ids: set[str] = set()
    exchange_ids: set[str] = set()
    for index, current in enumerate(results):
        require(current["window_id"] not in window_ids, "duplicate window ID")
        require(current["exchange_id"] not in exchange_ids, "duplicate exchange settlement")
        window_ids.add(current["window_id"])
        exchange_ids.add(current["exchange_id"])
        require(current["commit_allowed"], "rejected window cannot advance committed sequence")
        if index:
            previous = results[index - 1]
            require(current["start_tick"] == previous["end_tick"], "committed windows contain gap/overlap")
            require(current["input_state_sha256"] == previous["committed_state_sha256"], "committed state chain broken")
            require(current["configuration_sha256"] == previous["configuration_sha256"], "configuration changed in frozen sequence")
            require(current["timebase_sha256"] == previous["timebase_sha256"], "timebase changed in frozen sequence")


def close(actual: float, expected: float, label: str, abs_tol: float = 1.0e-12, rel_tol: float = 1.0e-10) -> None:
    require(
        math.isfinite(float(actual))
        and abs(float(actual) - float(expected)) <= abs_tol + rel_tol * abs(float(expected)),
        f"{label}: expected {expected:.16g}, received {actual!r}",
    )


def expect_rejected(label: str, operation) -> None:
    try:
        operation()
    except (AuditError, KeyError, TypeError, ValueError):
        print(f"negative_control=pass case={label}")
        return
    raise AuditError(f"negative control unexpectedly passed: {label}")


def hostile_controls(fixture: dict[str, Any], good: dict[str, Any]) -> None:
    bad = copy.deepcopy(fixture["good_window"])
    bad["fluid_grid"]["step_ticks"] = 30
    expect_rejected("incomplete_fluid_subcycle", lambda: audit_window(bad))

    bad = copy.deepcopy(fixture["good_window"])
    bad["exchange"]["timebase_sha256"] = "3333" * 16
    expect_rejected("stale_timebase_exchange", lambda: audit_window(bad))

    bad = copy.deepcopy(fixture["good_window"])
    bad["extrapolation_allowed"] = True
    expect_rejected("unqualified_temporal_extrapolation", lambda: audit_window(bad))

    bad = copy.deepcopy(fixture["good_window"])
    bad["exchange"]["fluid_interface_work_j"] = 7.0
    result = audit_window(bad)
    require(not result["commit_allowed"], "work imbalance committed")
    require(result["committed_state_sha256"] == bad["input_state_sha256"], "work failure did not roll back")
    require("interface_work_not_balanced" in result["errors"], "work failure missing diagnostic")
    print("negative_control=pass case=interface_work_imbalance_rollback")

    bad = copy.deepcopy(fixture["good_window"])
    bad["energy"]["final_fluid_energy_j"] += 1.0
    result = audit_window(bad)
    require(not result["commit_allowed"], "unexplained energy creation committed")
    require(result["committed_state_sha256"] == bad["input_state_sha256"], "energy failure did not roll back")
    require("whole_window_energy_balance_failed" in result["errors"], "energy failure missing diagnostic")
    print("negative_control=pass case=unexplained_energy_creation_rollback")

    bad = copy.deepcopy(fixture["good_window"])
    bad["exchange"]["structure_impulse_ns"][1] = 9.0
    result = audit_window(bad)
    require(not result["commit_allowed"], "action/reaction mismatch committed")
    require("interface_impulse_not_action_reaction_balanced" in result["errors"], "impulse failure missing diagnostic")
    print("negative_control=pass case=action_reaction_mismatch")

    bad = copy.deepcopy(fixture["good_window"])
    bad["inner_tolerance"] = 1.0e-6
    result = audit_window(bad)
    require(not result["commit_allowed"], "loose inner solver tolerance committed")
    require("inner_tolerance_not_stricter_than_coupling_policy" in result["errors"], "inner tolerance policy failure missing")
    print("negative_control=pass case=inner_outer_tolerance_policy")

    bad = copy.deepcopy(fixture["good_window"])
    bad["coupling_converged"] = False
    result = audit_window(bad)
    require(not result["commit_allowed"], "non-converged window committed")
    require(result["committed_state_sha256"] == bad["input_state_sha256"], "non-converged window did not roll back")
    print("negative_control=pass case=nonconverged_window_rollback")

    # A duplicate exchange ID across contiguous windows is a duplicate settlement.
    second = copy.deepcopy(good)
    second["window_id"] = "aero-window-0002"
    second["exchange_id"] = good["exchange_id"]
    second["start_tick"] = good["end_tick"]
    second["end_tick"] = good["end_tick"] + 100
    second["input_state_sha256"] = good["committed_state_sha256"]
    second["committed_state_sha256"] = "6666666666666666666666666666666666666666666666666666666666666666"
    expect_rejected("duplicate_exchange_settlement", lambda: audit_sequence([good, second]))

    second = copy.deepcopy(good)
    second["window_id"] = "aero-window-0002"
    second["exchange_id"] = "8888888888888888888888888888888888888888888888888888888888888888"
    second["start_tick"] = good["end_tick"] + 1
    second["end_tick"] = second["start_tick"] + 100
    second["input_state_sha256"] = good["committed_state_sha256"]
    expect_rejected("time_window_gap", lambda: audit_sequence([good, second]))


def verify_rust_output(output: dict[str, Any], fixture: dict[str, Any], expected_window: dict[str, Any]) -> None:
    require(output.get("schema_version") == fixture["schema_version"], "Rust schema version mismatch")
    require(output.get("claim_ceiling") == fixture["claim_ceiling"], "Rust claim ceiling mismatch")
    require(output.get("aircraft_feasibility") == "NOT_ASSESSED", "aircraft feasibility promoted")

    for name, expected in fixture["added_mass_cases"].items():
        actual = output["added_mass"][name]
        oracle = added_mass_oracle(expected)
        require(actual["status"] == expected["expected_status"] == oracle["status"], f"{name}: status mismatch")
        close(actual["exact_acceleration_m_per_s2"], oracle["exact_acceleration_m_per_s2"], f"{name}.exact")
        close(actual["amplification_factor"], expected["expected_amplification_factor"], f"{name}.amplification")
        close(actual["residual_n"], oracle["residual_n"], f"{name}.residual", abs_tol=1.0e-9)
        require(actual["iterations"] == oracle["iterations"], f"{name}: iteration count mismatch")
        if expected["expected_acceleration_m_per_s2"] is None:
            require(actual["acceleration_m_per_s2"] is None, f"{name}: non-converged acceleration was published")
        else:
            close(actual["acceleration_m_per_s2"], expected["expected_acceleration_m_per_s2"], f"{name}.acceleration")

    actual_window = output["window"]
    for key in (
        "status",
        "commit_allowed",
        "window_id",
        "exchange_id",
        "fluid_substeps",
        "structure_substeps",
        "committed_state_sha256",
    ):
        require(actual_window.get(key) == expected_window[key], f"Rust window mismatch for {key}")
    for key in ("impulse_residual_ns", "interface_work_residual_j", "energy_residual_j"):
        close(actual_window[key], expected_window[key], f"Rust window {key}")
    require(actual_window["errors"] == [], "good Rust window unexpectedly contains errors")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-output", type=Path)
    args = parser.parse_args()
    try:
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        for name, case in fixture["added_mass_cases"].items():
            result = added_mass_oracle(case)
            require(result["status"] == case["expected_status"], f"{name}: frozen status mismatch")
            close(result["exact_acceleration_m_per_s2"], case["expected_exact_acceleration_m_per_s2"], name + ".exact")
            close(result["amplification_factor"], case["expected_amplification_factor"], name + ".amplification")
            if case["expected_acceleration_m_per_s2"] is None:
                require(result["acceleration_m_per_s2"] is None, f"{name}: non-converged result promoted")
            else:
                close(result["acceleration_m_per_s2"], case["expected_acceleration_m_per_s2"], name + ".acceleration")

        good = audit_window(fixture["good_window"])
        expected_window = fixture["good_window"]["expected"]
        for key in ("status", "commit_allowed", "fluid_substeps", "structure_substeps", "committed_state_sha256"):
            require(good[key] == expected_window[key], f"fixture window mismatch for {key}")
        for key in ("impulse_residual_ns", "interface_work_residual_j", "energy_residual_j"):
            close(good[key], expected_window[key], "fixture window " + key)

        hostile_controls(fixture, good)
        if args.rust_output is not None:
            output = json.loads(args.rust_output.read_text(encoding="utf-8"))
            verify_rust_output(output, fixture, expected_window)

        mode = "rust_python_boundary=pass" if args.rust_output is not None else "rust_python_boundary=not_run"
        print(
            "dynamic_coupling_independent_oracle=pass "
            "added_mass_low=CONVERGED added_mass_high_unrelaxed=NON_CONVERGED "
            "added_mass_high_relaxed=CONVERGED window=COMMITTED "
            f"fluid_substeps={good['fluid_substeps']} structure_substeps={good['structure_substeps']} {mode}"
        )
    except (OSError, json.JSONDecodeError, AuditError, KeyError, TypeError, ValueError) as error:
        print(f"dynamic_coupling_oracle_error={error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
