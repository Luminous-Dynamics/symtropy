#!/usr/bin/env python3
"""Independent standard-library oracle for the AER-01 structural bridge.

This file deliberately does not import the Rust implementation. It recomputes
beam, thin-wall wing-box, spanwise load-transfer and scalar aeroelastic cases
from the frozen JSON input fixture, then checks the Rust CLI output when given.

The corpus is synthetic and verifies arithmetic/contracts only. It is not an
aircraft structural model, flutter assessment, fatigue assessment or safety case.
"""

from __future__ import annotations

import argparse
import copy
import json
import math
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
FIXTURE = HERE / "data" / "known-answer-cases-v0.1.0.json"
TOL = 1.0e-10


class AuditError(ValueError):
    """Raised when an input or evidence result violates the frozen contract."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AuditError(message)


def close(actual: float, expected: float, field: str, tolerance: float = TOL) -> None:
    require(
        isinstance(actual, (int, float))
        and math.isfinite(float(actual))
        and math.isfinite(float(expected))
        and abs(float(actual) - float(expected))
        <= 1.0e-12 + tolerance * abs(float(expected)),
        f"{field}: expected {expected:.16g}, got {actual!r}",
    )


def finite_positive(value: float, field: str) -> None:
    require(
        isinstance(value, (int, float)) and math.isfinite(float(value)) and value > 0.0,
        f"{field} must be finite and positive",
    )


def vector_add(left: list[float], right: list[float]) -> list[float]:
    return [left[i] + right[i] for i in range(3)]


def vector_scale(value: list[float], factor: float) -> list[float]:
    return [component * factor for component in value]


def cross(left: list[float], right: list[float]) -> list[float]:
    return [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]


def max_component_delta(left: list[float], right: list[float]) -> float:
    return max(abs(left[i] - right[i]) for i in range(3))


def beam_oracle(case: dict[str, Any]) -> dict[str, float]:
    e = case["youngs_modulus_pa"]
    inertia = case["second_moment_m4"]
    length = case["span_m"]
    force = case["tip_force_n"]
    distributed = case["uniform_load_n_per_m"]
    fibre = case["extreme_fibre_distance_m"]
    for name, value in (
        ("youngs_modulus_pa", e),
        ("second_moment_m4", inertia),
        ("span_m", length),
        ("extreme_fibre_distance_m", fibre),
    ):
        finite_positive(value, name)
    require(math.isfinite(force), "tip_force_n must be finite")
    require(math.isfinite(distributed), "uniform_load_n_per_m must be finite")

    ei = e * inertia
    root_moment = force * length + distributed * length**2 / 2.0
    tip_deflection = force * length**3 / (3.0 * ei) + distributed * length**4 / (8.0 * ei)
    tip_slope = force * length**2 / (2.0 * ei) + distributed * length**3 / (6.0 * ei)
    root_stress = root_moment * fibre / inertia
    return {
        "root_moment_nm": root_moment,
        "tip_deflection_m": tip_deflection,
        "tip_slope_rad": tip_slope,
        "root_bending_stress_pa": root_stress,
    }


def wing_box_oracle(case: dict[str, Any]) -> dict[str, float]:
    cap_area = case["cap_area_m2"]
    separation = case["cap_separation_m"]
    width = case["median_width_m"]
    height = case["median_height_m"]
    thickness = case["wall_thickness_m"]
    youngs = case["youngs_modulus_pa"]
    shear = case["shear_modulus_pa"]
    bending_moment = case["root_bending_moment_nm"]
    torque = case["applied_torque_nm"]
    span = case["span_m"]

    for name, value in (
        ("cap_area_m2", cap_area),
        ("cap_separation_m", separation),
        ("median_width_m", width),
        ("median_height_m", height),
        ("wall_thickness_m", thickness),
        ("youngs_modulus_pa", youngs),
        ("shear_modulus_pa", shear),
        ("span_m", span),
    ):
        finite_positive(value, name)
    require(math.isfinite(bending_moment), "root bending moment must be finite")
    require(math.isfinite(torque), "applied torque must be finite")

    # Bending: two concentrated caps only; torsion: one closed thin-wall cell.
    second_moment = cap_area * separation**2 / 2.0
    area = width * height
    perimeter_over_t = 2.0 * (width + height) / thickness
    torsion_constant = 4.0 * area**2 / perimeter_over_t
    shear_flow = torque / (2.0 * area)
    wall_stress = shear_flow / thickness
    bending_stress = bending_moment * separation / (2.0 * second_moment)
    twist = torque * span / (shear * torsion_constant)
    return {
        "bending_second_moment_m4": second_moment,
        "bending_stress_pa": bending_stress,
        "torsion_constant_m4": torsion_constant,
        "torsional_shear_flow_n_per_m": shear_flow,
        "wall_shear_stress_pa": wall_stress,
        "tip_twist_rad": twist,
    }


def load_resultant(loads: list[dict[str, Any]]) -> tuple[list[float], list[float]]:
    force_sum = [0.0, 0.0, 0.0]
    moment_sum = [0.0, 0.0, 0.0]
    for load in loads:
        x = float(load["x_m"])
        force = list(map(float, load["force_n"]))
        couple = list(map(float, load["couple_nm"]))
        require(math.isfinite(x), "source load station must be finite")
        require(all(math.isfinite(x) for x in force + couple), "source load must be finite")
        force_sum = vector_add(force_sum, force)
        moment_sum = vector_add(moment_sum, cross([x, 0.0, 0.0], force))
        moment_sum = vector_add(moment_sum, couple)
    return force_sum, moment_sum


def independent_spanwise_map(
    stations: list[float], loads: list[dict[str, Any]]
) -> list[dict[str, Any]]:
    """Map each point load by linear shape functions on its bracketing segment."""
    require(len(stations) >= 2, "at least two target stations are required")
    require(all(math.isfinite(x) for x in stations), "target stations must be finite")
    require(
        all(stations[index + 1] > stations[index] for index in range(len(stations) - 1)),
        "target stations must be strictly increasing",
    )
    mapped = [
        {"x_m": x, "force_n": [0.0, 0.0, 0.0], "couple_nm": [0.0, 0.0, 0.0]}
        for x in stations
    ]
    for load in loads:
        x = float(load["x_m"])
        require(stations[0] <= x <= stations[-1], "source load lies outside target span")
        force = list(map(float, load["force_n"]))
        couple = list(map(float, load["couple_nm"]))
        require(all(math.isfinite(v) for v in force + couple), "source load must be finite")

        exact = next(
            (index for index, station in enumerate(stations) if abs(station - x) <= 1.0e-12),
            None,
        )
        if exact is not None:
            weights = [(exact, 1.0)]
        else:
            left = next(
                index
                for index in range(len(stations) - 1)
                if stations[index] < x < stations[index + 1]
            )
            fraction = (x - stations[left]) / (stations[left + 1] - stations[left])
            weights = [(left, 1.0 - fraction), (left + 1, fraction)]

        for index, weight in weights:
            mapped[index]["force_n"] = vector_add(
                mapped[index]["force_n"], vector_scale(force, weight)
            )
            mapped[index]["couple_nm"] = vector_add(
                mapped[index]["couple_nm"], vector_scale(couple, weight)
            )
    return mapped


def station_resultant(stations: list[dict[str, Any]]) -> tuple[list[float], list[float]]:
    force_sum = [0.0, 0.0, 0.0]
    moment_sum = [0.0, 0.0, 0.0]
    for station in stations:
        force = station["force_n"]
        couple = station["couple_nm"]
        force_sum = vector_add(force_sum, force)
        moment_sum = vector_add(moment_sum, cross([station["x_m"], 0.0, 0.0], force))
        moment_sum = vector_add(moment_sum, couple)
    return force_sum, moment_sum


def fixed_point_oracle(case: dict[str, Any]) -> dict[str, Any]:
    compliance = float(case["compliance_m_per_n"])
    baseline = float(case["baseline_lift_n"])
    feedback = float(case["lift_feedback_n_per_m"])
    tolerance = float(case["tolerance_m"])
    limit = int(case["max_iterations"])
    finite_positive(compliance, "compliance_m_per_n")
    finite_positive(tolerance, "tolerance_m")
    require(math.isfinite(baseline) and math.isfinite(feedback), "loads must be finite")
    require(limit > 0, "max_iterations must be positive")

    gain = compliance * feedback
    displacement = 0.0
    residual = math.inf
    iterations = 0
    for iterations in range(1, limit + 1):
        displacement = compliance * (baseline + feedback * displacement)
        residual = abs(displacement - compliance * (baseline + feedback * displacement))
        if residual <= tolerance:
            return {
                "status": "CONVERGED",
                "gain": gain,
                "iterations": iterations,
                "tip_deflection_m": displacement,
                "residual_m": residual,
            }
    return {
        "status": "NON_CONVERGED",
        "gain": gain,
        "iterations": iterations,
        "tip_deflection_m": None,
        "last_iterate_m": displacement,
        "residual_m": residual,
    }


def verify_fixture(fixture: dict[str, Any]) -> None:
    require(
        fixture.get("schema_version") == "aero-structural-analytic-oracle-fixture/0.1.0",
        "unsupported fixture schema",
    )
    require(
        fixture.get("claim_ceiling") == "bounded_analytical_software_verification_only",
        "fixture claim ceiling changed",
    )
    require(fixture.get("aircraft_feasibility") == "NOT_ASSESSED", "synthetic fixture cannot claim aircraft feasibility")

    expected = fixture["expected"]
    inputs = fixture["inputs"]
    beam = beam_oracle(inputs["beam"])
    for field, value in beam.items():
        close(value, float(expected["beam"][field]), f"beam.{field}")

    wing = wing_box_oracle(inputs["wing_box"])
    for field, value in wing.items():
        close(value, float(expected["wing_box"][field]), f"wing_box.{field}")

    transfer = inputs["load_transfer"]
    source_force, source_moment = load_resultant(transfer["source_loads"])
    mapped_nodes = independent_spanwise_map(
        transfer["target_stations_x_m"], transfer["source_loads"]
    )
    mapped_force, mapped_moment = station_resultant(mapped_nodes)
    force_residual = max_component_delta(source_force, mapped_force)
    moment_residual = max_component_delta(source_moment, mapped_moment)
    for axis in range(3):
        close(source_force[axis], float(expected["load_transfer"]["resultant_force_n"][axis]), f"load_transfer.force[{axis}]")
        close(source_moment[axis], float(expected["load_transfer"]["resultant_moment_nm"][axis]), f"load_transfer.moment[{axis}]")
        close(mapped_force[axis], source_force[axis], f"load_transfer.mapped_force[{axis}]")
        close(mapped_moment[axis], source_moment[axis], f"load_transfer.mapped_moment[{axis}]")
    close(force_residual, 0.0, "load_transfer.force_residual")
    close(moment_residual, 0.0, "load_transfer.moment_residual")

    coupled = fixed_point_oracle(inputs["static_aeroelastic"])
    require(coupled["status"] == "CONVERGED", "bounded static aeroelastic fixture must converge")
    closed_form = (
        inputs["static_aeroelastic"]["compliance_m_per_n"]
        * inputs["static_aeroelastic"]["baseline_lift_n"]
        / (1.0 - coupled["gain"])
    )
    close(coupled["tip_deflection_m"], closed_form, "static_aeroelastic.closed_form")
    close(
        coupled["gain"],
        float(expected["static_aeroelastic"]["gain"]),
        "static_aeroelastic.gain",
    )

    divergent = fixed_point_oracle(inputs["divergent_control"])
    require(divergent["status"] == "NON_CONVERGED", "divergent control must not converge")
    close(
        divergent["gain"],
        float(expected["divergent_control"]["gain"]),
        "divergent_control.gain",
    )
    require(
        divergent["iterations"] == expected["divergent_control"]["iteration_limit"],
        "divergent control must exhaust its declared iteration limit",
    )


def verify_rust_output(output: dict[str, Any], fixture: dict[str, Any]) -> None:
    verify_fixture(fixture)
    require(output.get("schema_version") == "aero-structural-analytic-oracle/0.1.0", "Rust output schema mismatch")
    require(output.get("claim_ceiling") == fixture["claim_ceiling"], "Rust output claim ceiling mismatch")
    require(output.get("aircraft_feasibility") == "NOT_ASSESSED", "Rust output promotes an unassessed aircraft")
    require(output.get("analysis_identity") == fixture["analysis_identity"], "Rust output state identity is stale or mismatched")
    require(output.get("identity_binding") == "MATCHED", "Rust output did not bind exact state identity")
    require(
        output.get("unassessed_aircraft_claims") == fixture["unassessed_aircraft_claims"],
        "Rust output omitted or changed unassessed claim categories",
    )

    expected = fixture["expected"]
    for section in ("beam", "wing_box"):
        actual = output[section]
        for field, value in expected[section].items():
            close(actual[field], float(value), f"rust.{section}.{field}")

    transfer_expected = expected["load_transfer"]
    transfer_actual = output["load_transfer"]
    for field in ("source_force_n", "mapped_force_n"):
        for axis in range(3):
            close(transfer_actual[field][axis], transfer_expected["resultant_force_n"][axis], f"rust.load_transfer.{field}[{axis}]")
    for field in ("source_moment_nm", "mapped_moment_nm"):
        for axis in range(3):
            close(transfer_actual[field][axis], transfer_expected["resultant_moment_nm"][axis], f"rust.load_transfer.{field}[{axis}]")
    close(transfer_actual["force_residual_n"], 0.0, "rust.load_transfer.force_residual")
    close(transfer_actual["moment_residual_nm"], 0.0, "rust.load_transfer.moment_residual")

    coupled_expected = expected["static_aeroelastic"]
    coupled_actual = output["static_aeroelastic"]
    require(coupled_actual["status"] == coupled_expected["status"], "Rust static coupling status mismatch")
    close(coupled_actual["gain"], coupled_expected["gain"], "rust.static_aeroelastic.gain")
    close(coupled_actual["tip_deflection_m"], coupled_expected["closed_form_tip_deflection_m"], "rust.static_aeroelastic.deflection")
    require(1 <= coupled_actual["iterations"] <= coupled_expected["iteration_limit"], "Rust coupling iteration count out of bounds")
    require(coupled_actual["residual_m"] <= fixture["inputs"]["static_aeroelastic"]["tolerance_m"], "Rust coupling residual exceeds tolerance")

    divergent_expected = expected["divergent_control"]
    divergent_actual = output["divergent_control"]
    require(divergent_actual["status"] == divergent_expected["status"], "Rust divergent control was incorrectly promoted")
    require(divergent_actual["tip_deflection_m"] is None, "non-converged control must not publish a converged deflection")
    require(divergent_actual["iterations"] == divergent_expected["iteration_limit"], "non-converged control did not report the iteration limit")
    require(divergent_actual["residual_m"] > fixture["inputs"]["divergent_control"]["tolerance_m"], "non-converged control residual is not above tolerance")


def expect_rejected(label: str, action) -> None:
    try:
        action()
    except (AuditError, KeyError, TypeError, ValueError, StopIteration):
        print(f"negative_control=pass case={label}")
        return
    raise AuditError(f"negative control unexpectedly passed: {label}")


def hostile_controls(fixture: dict[str, Any], reference_output: dict[str, Any] | None) -> None:
    expect_rejected(
        "zero_youngs_modulus",
        lambda: beam_oracle({**fixture["inputs"]["beam"], "youngs_modulus_pa": 0.0}),
    )
    expect_rejected(
        "duplicate_target_station",
        lambda: independent_spanwise_map(
            [0.0, 2.0, 2.0],
            fixture["inputs"]["load_transfer"]["source_loads"],
        ),
    )
    expect_rejected(
        "out_of_span_load",
        lambda: independent_spanwise_map(
            [0.0, 2.0, 4.0],
            [{**fixture["inputs"]["load_transfer"]["source_loads"][0], "x_m": 4.1}],
        ),
    )
    divergent = fixed_point_oracle(fixture["inputs"]["divergent_control"])
    require(divergent["status"] == "NON_CONVERGED", "divergent hostile control was promoted")

    if reference_output is None:
        return

    false_feasible = copy.deepcopy(reference_output)
    false_feasible["aircraft_feasibility"] = "FEASIBLE"
    expect_rejected(
        "false_feasible_aircraft_claim",
        lambda: verify_rust_output(false_feasible, fixture),
    )

    stale_state = copy.deepcopy(reference_output)
    stale_state["analysis_identity"]["geometry_sha256"] = "f" * 64
    expect_rejected(
        "stale_geometry_digest",
        lambda: verify_rust_output(stale_state, fixture),
    )

    lost_moment = copy.deepcopy(reference_output)
    lost_moment["load_transfer"]["mapped_moment_nm"][2] += 1.0
    expect_rejected(
        "nonconservative_moment_transfer",
        lambda: verify_rust_output(lost_moment, fixture),
    )

    false_convergence = copy.deepcopy(reference_output)
    false_convergence["divergent_control"]["status"] = "CONVERGED"
    expect_rejected(
        "divergent_coupling_reported_as_converged",
        lambda: verify_rust_output(false_convergence, fixture),
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--rust-output",
        type=Path,
        help="JSON output captured from the Rust oracle executable",
    )
    args = parser.parse_args()

    try:
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        verify_fixture(fixture)
        rust_output = None
        if args.rust_output is not None:
            rust_output = json.loads(args.rust_output.read_text(encoding="utf-8"))
            verify_rust_output(rust_output, fixture)
        hostile_controls(fixture, rust_output)

        mode = "rust_python_boundary=pass" if rust_output is not None else "rust_python_boundary=not_run"
        print(
            "structural_aeroelastic_independent_oracle=pass "
            f"beam=pass wing_box=pass load_transfer=conservative "
            f"static_coupling=pass divergent_control=abstained {mode}"
        )
    except (OSError, json.JSONDecodeError, AuditError, KeyError, TypeError, ValueError) as error:
        print(f"structural_aeroelastic_oracle_error={error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
