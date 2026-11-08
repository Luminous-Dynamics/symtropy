#!/usr/bin/env python3
"""Independent standard-library oracle for the analytical 2-DOF complex FRF corpus.

No NumPy/SciPy and no import of the Rust executable. The implementation solves
the 2x2 complex dynamic-stiffness system independently and checks the frozen
known answers as well as Rust's emitted result.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
CORPUS = HERE / "data" / "frf-corpus-v0.1.json"


class OracleError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise OracleError(message)


def cadd(a: complex, b: complex) -> complex:
    return complex(a.real + b.real, a.imag + b.imag)


def csub(a: complex, b: complex) -> complex:
    return complex(a.real - b.real, a.imag - b.imag)


def cmul(a: complex, b: complex) -> complex:
    return complex(a.real * b.real - a.imag * b.imag,
                   a.real * b.imag + a.imag * b.real)


def cdiv(a: complex, b: complex, label: str) -> complex:
    denominator = b.real * b.real + b.imag * b.imag
    require(math.isfinite(denominator) and denominator > 1.0e-28,
            f"{label}: singular or nearly singular complex denominator")
    value = complex((a.real * b.real + a.imag * b.imag) / denominator,
                    (a.imag * b.real - a.real * b.imag) / denominator)
    require(math.isfinite(value.real) and math.isfinite(value.imag),
            f"{label}: non-finite complex quotient")
    return value


def from_json(value: dict[str, Any]) -> complex:
    return complex(float(value["re"]), float(value["im"]))


def to_json(value: complex) -> dict[str, float]:
    return {"re": value.real, "im": value.imag}


def validate_system(system: dict[str, Any], label: str) -> None:
    mass = system["mass_kg"]
    stiffness = system["stiffness_n_m"]
    damping = system["damping_ns_m"]
    forces = system["force_n"]

    require(len(mass) == 2 and all(math.isfinite(float(x)) and float(x) > 0 for x in mass),
            f"{label}: mass must be two finite positive values")
    for matrix_name, matrix in (("stiffness", stiffness), ("damping", damping)):
        require(len(matrix) == 2 and all(len(row) == 2 for row in matrix),
                f"{label}: {matrix_name} must be 2x2")
        require(all(math.isfinite(float(x)) for row in matrix for x in row),
                f"{label}: {matrix_name} contains non-finite entries")
        require(abs(float(matrix[0][1]) - float(matrix[1][0])) <= 1.0e-12,
                f"{label}: {matrix_name} matrix is not symmetric")

    kdet = float(stiffness[0][0]) * float(stiffness[1][1]) \
        - float(stiffness[0][1]) * float(stiffness[1][0])
    require(float(stiffness[0][0]) > 0 and kdet > 0,
            f"{label}: stiffness matrix is not positive definite")
    cdet = float(damping[0][0]) * float(damping[1][1]) \
        - float(damping[0][1]) * float(damping[1][0])
    require(float(damping[0][0]) >= 0 and float(damping[1][1]) >= 0 and cdet >= -1.0e-10,
            f"{label}: damping matrix is not positive semidefinite")
    require(len(forces) == 2, f"{label}: expected two force entries")
    for force in forces:
        value = from_json(force)
        require(math.isfinite(value.real) and math.isfinite(value.imag),
                f"{label}: non-finite force")


def solve_frf(system: dict[str, Any], frequency_hz: float) -> list[complex]:
    require(math.isfinite(frequency_hz) and frequency_hz > 0,
            "frequency must be positive Hz")
    omega = 2.0 * math.pi * frequency_hz
    mass = system["mass_kg"]
    stiffness = system["stiffness_n_m"]
    damping = system["damping_ns_m"]
    force = [from_json(value) for value in system["force_n"]]

    d00 = complex(float(stiffness[0][0]) - omega * omega * float(mass[0]),
                  omega * float(damping[0][0]))
    d01 = complex(float(stiffness[0][1]), omega * float(damping[0][1]))
    d10 = complex(float(stiffness[1][0]), omega * float(damping[1][0]))
    d11 = complex(float(stiffness[1][1]) - omega * omega * float(mass[1]),
                  omega * float(damping[1][1]))

    determinant = csub(cmul(d00, d11), cmul(d01, d10))
    numerator0 = csub(cmul(d11, force[0]), cmul(d01, force[1]))
    numerator1 = csub(cmul(d00, force[1]), cmul(d10, force[0]))
    return [
        cdiv(numerator0, determinant, "DOF 0 FRF"),
        cdiv(numerator1, determinant, "DOF 1 FRF"),
    ]


def sensor_value(sensor: dict[str, Any], response: list[complex]) -> complex | None:
    if not sensor["available"] or sensor["fault"] == "DROPOUT":
        return None
    if sensor["fault"] == "STUCK_AT":
        require(sensor.get("stuck_value") is not None, "STUCK_AT sensor requires stuck_value")
        value = from_json(sensor["stuck_value"])
        require(math.isfinite(value.real) and math.isfinite(value.imag),
                "STUCK_AT sensor value is non-finite")
        return value

    require(sensor["dof"] in (0, 1), "sensor DOF must be 0 or 1")
    gain = float(sensor["gain"])
    phase = math.radians(float(sensor["phase_deg"]))
    bias = from_json(sensor["bias"])
    return cmul(complex(gain * math.cos(phase), gain * math.sin(phase)),
                response[int(sensor["dof"])]) + bias


def observation_rank(sensors: list[dict[str, Any]]) -> int:
    dofs = {
        int(sensor["dof"])
        for sensor in sensors
        if sensor["available"] and sensor["fault"] not in ("DROPOUT", "STUCK_AT")
    }
    return len(dofs)


def angular_error(actual: complex, expected: complex) -> float:
    delta = math.atan2(actual.imag, actual.real) - math.atan2(expected.imag, expected.real)
    return abs((delta + math.pi) % (2.0 * math.pi) - math.pi)


def compare_complex(actual: complex, expected: complex, tolerance: dict[str, float], label: str) -> None:
    error = abs(actual - expected)
    threshold = float(tolerance["absolute"]) + float(tolerance["relative"]) * abs(expected)
    require(error <= threshold, f"{label}: complex error {error:e} > {threshold:e}")
    phase_error = angular_error(actual, expected)
    require(phase_error <= float(tolerance["phase_rad"]),
            f"{label}: phase error {phase_error:e} rad > {tolerance['phase_rad']:e}")


def compute_case(case: dict[str, Any], tolerance: dict[str, float],
                 corpus_sha256: str) -> dict[str, Any]:
    validate_system(case["nominal_model"], f"{case['case_id']} nominal")
    validate_system(case["physical_model"], f"{case['case_id']} physical")
    frequencies = case["frequencies_hz"]
    require(len(frequencies) == len(case["expected_reference"]),
            f"{case['case_id']}: reference length mismatch")
    ids = [sensor["sensor_id"] for sensor in case["sensors"]]
    require(len(ids) == len(set(ids)), f"{case['case_id']}: duplicate sensor IDs")
    for sensor in case["sensors"]:
        require(sensor["dof"] in (0, 1), f"{case['case_id']}: invalid sensor DOF")
        require(math.isfinite(float(sensor["gain"])) and float(sensor["gain"]) >= 0,
                f"{case['case_id']}: invalid sensor gain")
        require(math.isfinite(float(sensor["phase_deg"])), f"{case['case_id']}: invalid sensor phase")
        bias = from_json(sensor["bias"])
        require(math.isfinite(bias.real) and math.isfinite(bias.imag),
                f"{case['case_id']}: invalid sensor bias")
        require(sensor["fault"] in ("NONE", "GAIN_PHASE", "BIAS", "DROPOUT", "STUCK_AT"),
                f"{case['case_id']}: unsupported sensor fault")

    rank = observation_rank(case["sensors"])
    require(rank == int(case["expected_observation_rank"]),
            f"{case['case_id']}: observation rank mismatch")

    computed = []
    for index, frequency in enumerate(frequencies):
        expected = case["expected_reference"][index]
        require(abs(float(expected["frequency_hz"]) - float(frequency)) <= 1.0e-12,
                f"{case['case_id']}: frequency schedule mismatch")
        nominal = solve_frf(case["nominal_model"], float(frequency))
        physical = solve_frf(case["physical_model"], float(frequency))

        for dof in range(2):
            compare_complex(
                nominal[dof],
                from_json(expected["nominal_response_m_per_n"][dof]),
                tolerance,
                f"{case['case_id']} nominal DOF {dof} at {frequency} Hz",
            )
            compare_complex(
                physical[dof],
                from_json(expected["physical_response_m_per_n"][dof]),
                tolerance,
                f"{case['case_id']} physical DOF {dof} at {frequency} Hz",
            )

        observed = []
        for sensor_index, sensor in enumerate(case["sensors"]):
            expected_channel = expected["observed_channels"][sensor_index]
            require(expected_channel["sensor_id"] == sensor["sensor_id"],
                    f"{case['case_id']}: sensor order/ID mismatch")
            value = sensor_value(sensor, physical)
            expected_value = expected_channel["value"]
            require((value is None) == (expected_value is None),
                    f"{case['case_id']}: missing-channel semantics mismatch")
            if value is not None:
                compare_complex(
                    value,
                    from_json(expected_value),
                    tolerance,
                    f"{case['case_id']} {sensor['sensor_id']} at {frequency} Hz",
                )
            observed.append({
                "sensor_id": sensor["sensor_id"],
                "value": None if value is None else to_json(value),
            })

        computed.append({
            "frequency_hz": float(frequency),
            "omega_rad_s": 2.0 * math.pi * float(frequency),
            "nominal_response_m_per_n": [to_json(x) for x in nominal],
            "physical_response_m_per_n": [to_json(x) for x in physical],
            "observed_channels": observed,
            "nominal_vs_physical_complex_residual_m_per_n": [
                to_json(physical[0] - nominal[0]),
                to_json(physical[1] - nominal[1]),
            ],
        })

    scenario_identity = f"{corpus_sha256}:{case['case_id']}".encode()
    return {
        "case_id": case["case_id"],
        "state": case["state"],
        "scenario_sha256": hashlib.sha256(scenario_identity).hexdigest(),
        "expected_disposition": case["expected_disposition"],
        "observation_rank": rank,
        "frequencies": computed,
    }


def validate_corpus(data: dict[str, Any], raw_bytes: bytes | None = None) -> dict[str, Any]:
    require(data.get("schema_version") == "aero-virtual-sensing-frf-corpus/0.1.0",
            "unsupported corpus schema")
    require(data.get("evidence_class") == "synthetic_analytical_known_answer",
            "evidence class changed")
    require(data.get("claim_ceiling") == "analytical_2dof_frequency_response_and_sensor_mapping_only",
            "claim ceiling changed")
    require(data.get("physical_pod_claim") is False, "synthetic corpus cannot claim physical POD")
    require(data.get("aircraft_validation_claim") is False,
            "synthetic corpus cannot claim aircraft validation")
    require(data.get("frequency_convention") == "f_hz; omega_rad_s = 2*pi*f_hz",
            "frequency convention changed")
    require(data.get("response_convention") ==
            "x = inverse(K - omega^2*M + i*omega*C) * F; e^(+i*omega*t) phasor",
            "phasor convention changed")
    require(
        data.get("unit_system") == {
            "mass": "kg",
            "stiffness": "N/m",
            "damping": "N*s/m",
            "force": "N",
            "response": "m/N",
        },
        "unit system convention changed",
    )
    require(len(data.get("cases", [])) == 9, "frozen scenario count changed")
    if raw_bytes is None:
        raw_bytes = json.dumps(data, sort_keys=True, separators=(",", ":")).encode("utf-8")
    corpus_sha256 = hashlib.sha256(raw_bytes).hexdigest()
    computed_cases = [
        compute_case(case, data["tolerance"], corpus_sha256) for case in data["cases"]
    ]
    return {
        "schema_version": "aero-virtual-sensing-frf-result/0.1.0",
        "corpus_id": data["corpus_id"],
        "corpus_sha256": corpus_sha256,
        "case_count": len(computed_cases),
        "frequency_count_per_case": len(data["cases"][0]["frequencies_hz"]),
        "computed_cases": computed_cases,
        "physical_pod": "NOT_ASSESSED",
        "aircraft_validation": "NOT_ASSESSED",
        "structural_clearance": "NOT_AUTHORIZED",
        "claim_ceiling": "analytical_2dof_frequency_response_and_sensor_mapping_only",
    }


def expect_rejected(label: str, action) -> None:
    try:
        action()
    except (OracleError, KeyError, TypeError, ValueError, ZeroDivisionError):
        print(f"negative_control=pass case={label}")
        return
    raise OracleError(f"negative control unexpectedly passed: {label}")


def run_hostile_controls(data: dict[str, Any]) -> int:
    count = 0
    bad = copy.deepcopy(data)
    bad["cases"][0]["physical_model"]["mass_kg"][0] = 0.0
    expect_rejected("non_positive_mass", lambda: validate_corpus(bad))
    count += 1

    bad = copy.deepcopy(data)
    bad["cases"][0]["physical_model"]["stiffness_n_m"] = [[1.0, 2.0], [2.0, 1.0]]
    expect_rejected("non_positive_definite_stiffness", lambda: validate_corpus(bad))
    count += 1

    bad = copy.deepcopy(data)
    bad["cases"][0]["expected_reference"][0]["observed_channels"][1]["value"] = {"re": 0.0, "im": 0.0}
    expect_rejected("missing_channel_as_zero", lambda: validate_corpus(bad))
    count += 1

    bad = copy.deepcopy(data)
    bad["cases"][1]["sensors"][1]["phase_deg"] *= -1
    expect_rejected("phase_sign_mismatch", lambda: validate_corpus(bad))
    count += 1

    bad = copy.deepcopy(data)
    bad["cases"][0]["frequencies_hz"][0] = 2.0 * math.pi * bad["cases"][0]["frequencies_hz"][0]
    expect_rejected("frequency_unit_hz_vs_rad_s", lambda: validate_corpus(bad))
    count += 1

    bad = copy.deepcopy(data)
    bad["cases"][0]["sensors"][0]["dof"] = 2
    expect_rejected("sensor_dof_out_of_range", lambda: validate_corpus(bad))
    count += 1

    return count


def compare_complex_json(actual: dict[str, Any], expected: dict[str, Any],
                          tolerance: dict[str, float], label: str) -> None:
    a = complex(float(actual["re"]), float(actual["im"]))
    e = complex(float(expected["re"]), float(expected["im"]))
    error = abs(a - e)
    bound = float(tolerance["absolute"]) + float(tolerance["relative"]) * abs(e)
    require(error <= bound, f"{label}: complex error {error:e} > {bound:e}")
    phase_error = angular_error(a, e)
    require(
        phase_error <= float(tolerance["phase_rad"]),
        f"{label}: phase error {phase_error:e} rad > {tolerance['phase_rad']:e}",
    )


def close_scalar(actual: float, expected: float, tolerance: dict[str, float], label: str) -> None:
    error = abs(float(actual) - float(expected))
    bound = float(tolerance["absolute"]) + float(tolerance["relative"]) * abs(float(expected))
    require(error <= bound, f"{label}: scalar error {error:e} > {bound:e}")


def verify_rust_output(actual: dict[str, Any], expected: dict[str, Any],
                       tolerance: dict[str, float]) -> None:
    for key in ("schema_version", "corpus_id", "corpus_sha256", "case_count",
                "frequency_count_per_case", "physical_pod", "aircraft_validation",
                "structural_clearance", "claim_ceiling"):
        require(actual.get(key) == expected.get(key), f"Rust/Python result mismatch: {key}")

    actual_cases = actual.get("computed_cases")
    expected_cases = expected.get("computed_cases")
    require(isinstance(actual_cases, list) and isinstance(expected_cases, list),
            "Rust/Python computed_cases must be arrays")
    require(len(actual_cases) == len(expected_cases), "Rust/Python scenario count mismatch")

    for case_index, (a_case, e_case) in enumerate(zip(actual_cases, expected_cases)):
        label = f"scenario[{case_index}]"
        for key in ("case_id", "state", "scenario_sha256", "expected_disposition", "observation_rank"):
            require(a_case.get(key) == e_case.get(key), f"{label}: Rust/Python metadata mismatch: {key}")
        a_freqs = a_case.get("frequencies")
        e_freqs = e_case.get("frequencies")
        require(isinstance(a_freqs, list) and isinstance(e_freqs, list),
                f"{label}: frequency results must be arrays")
        require(len(a_freqs) == len(e_freqs), f"{label}: frequency count mismatch")

        for frequency_index, (a_freq, e_freq) in enumerate(zip(a_freqs, e_freqs)):
            f_label = f"{label}.frequency[{frequency_index}]"
            close_scalar(a_freq["frequency_hz"], e_freq["frequency_hz"], tolerance,
                         f"{f_label}.frequency_hz")
            close_scalar(a_freq["omega_rad_s"], e_freq["omega_rad_s"], tolerance,
                         f"{f_label}.omega_rad_s")
            for key in ("nominal_response_m_per_n", "physical_response_m_per_n",
                        "nominal_vs_physical_complex_residual_m_per_n"):
                a_values = a_freq[key]
                e_values = e_freq[key]
                require(len(a_values) == len(e_values), f"{f_label}: {key} length mismatch")
                for dof, (a_value, e_value) in enumerate(zip(a_values, e_values)):
                    compare_complex_json(a_value, e_value, tolerance,
                                         f"{f_label}.{key}.dof[{dof}]")

            a_channels = a_freq["observed_channels"]
            e_channels = e_freq["observed_channels"]
            require(len(a_channels) == len(e_channels), f"{f_label}: observed channel count mismatch")
            for channel_index, (a_channel, e_channel) in enumerate(zip(a_channels, e_channels)):
                c_label = f"{f_label}.channel[{channel_index}]"
                require(a_channel["sensor_id"] == e_channel["sensor_id"],
                        f"{c_label}: sensor ID mismatch")
                require((a_channel["value"] is None) == (e_channel["value"] is None),
                        f"{c_label}: missing-channel representation mismatch")
                if e_channel["value"] is not None:
                    compare_complex_json(a_channel["value"], e_channel["value"], tolerance,
                                         f"{c_label}.value")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-output", type=Path)
    args = parser.parse_args()
    try:
        raw = CORPUS.read_bytes()
        data = json.loads(raw.decode("utf-8"))
        expected = validate_corpus(data, raw)
        hostile_count = run_hostile_controls(data)
        rust_boundary = "not_run"
        rust_output_sha256 = None
        if args.rust_output is not None:
            rust_bytes = args.rust_output.read_bytes()
            actual = json.loads(rust_bytes.decode("utf-8"))
            verify_rust_output(actual, expected, data['tolerance'])
            rust_boundary = "pass"
            rust_output_sha256 = hashlib.sha256(rust_bytes).hexdigest()

        audit = {
            "schema_version": "aero-virtual-sensing-frf-audit/0.1.0",
            "corpus_sha256": hashlib.sha256(raw).hexdigest(),
            "case_count": expected["case_count"],
            "frequency_count_per_case": expected["frequency_count_per_case"],
            "hostile_control_count": hostile_count,
            "rust_python_boundary": rust_boundary,
            "rust_output_sha256": rust_output_sha256,
            "physical_pod": "NOT_ASSESSED",
            "aircraft_validation": "NOT_ASSESSED",
            "structural_clearance": "NOT_AUTHORIZED",
            "claim_ceiling": "analytical_2dof_frequency_response_and_sensor_mapping_only",
        }
        print(json.dumps(audit, sort_keys=True, separators=(",", ":")))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError, OracleError, KeyError, TypeError, ValueError) as error:
        print(f"frf_independent_oracle_error={error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
