#!/usr/bin/env python3
"""Independent policy oracle for synthetic AER-01 inspection/SHM contracts.

This script evaluates only typed state/identity/coverage/abstention rules.
It does not estimate POD, sensor reliability probability, damage severity,
structural safety, maintenance status, or aircraft acceptability.
"""

from __future__ import annotations

import argparse

import copy
import hashlib
import json
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
CORPUS = HERE / "inspection-observability-corpus-v0.1.json"

EXPECTED_DISPOSITIONS = {
    "SYN-IO-001": "NO_ANOMALY_IN_SYNTHETIC_FIXTURE",
    "SYN-IO-002": "UNRESOLVED_INCOMPLETE_CRITICAL_COVERAGE",
    "SYN-IO-003": "REJECT_IDENTITY_MISMATCH",
    "SYN-IO-004": "REJECT_IDENTITY_MISMATCH",
    "SYN-IO-005": "REJECT_INVALID_INSPECTION_STATE",
    "SYN-IO-006": "UNRESOLVED_METHOD_OUT_OF_DOMAIN",
    "SYN-IO-007": "REQUIRE_INDEPENDENT_NDE",
    "SYN-IO-008": "SENSOR_FAULT_SUSPECTED_NO_STRUCTURAL_CLEARANCE",
    "SYN-IO-009": "REQUIRE_INDEPENDENT_NDE",
    "SYN-IO-010": "REASSESS_ENVIRONMENT",
    "SYN-IO-011": "REQUIRE_INDEPENDENT_NDE",
    "SYN-IO-012": "UNRESOLVED_SENSING_STATE",
}

class ContractError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ContractError(message)


def decide(case: dict[str, Any]) -> str:
    inspection = case["inspection"]
    sensing = case["sensing"]

    # Identity mismatch invalidates all downstream geometry/coverage evidence.
    if not inspection["geometry_generation_match"] or not inspection["coordinate_frame_match"]:
        return "REJECT_IDENTITY_MISMATCH"

    # Procedure and calibration are prerequisites, not performance metrics.
    if not inspection["procedure_current"] or not inspection["calibration_current"]:
        return "REJECT_INVALID_INSPECTION_STATE"

    if not inspection["method_applicable"]:
        return "UNRESOLVED_METHOD_OUT_OF_DOMAIN"

    required = set(inspection["required_regions"])
    observed = set(inspection["observed_regions"])
    if (
        not inspection["critical_regions_observed"]
        or not required.issubset(observed)
    ):
        return "UNRESOLVED_INCOMPLETE_CRITICAL_COVERAGE"

    if not sensing["state_known"]:
        return "UNRESOLVED_SENSING_STATE"

    # Conservative precedence: a damage hypothesis always requires independent
    # physical inspection, even when sensor faults also exist or correction
    # makes the reconstructed signal look nominal.
    if sensing["structural_damage_candidate"]:
        return "REQUIRE_INDEPENDENT_NDE"

    if sensing["sensor_fault_candidate"]:
        return "SENSOR_FAULT_SUSPECTED_NO_STRUCTURAL_CLEARANCE"

    if sensing["environment_shift"]:
        return "REASSESS_ENVIRONMENT"

    return "NO_ANOMALY_IN_SYNTHETIC_FIXTURE"


def validate_corpus(data: dict[str, Any]) -> None:
    require(
        data.get("schema_version") == "aero-inspection-observability-corpus/0.1.0",
        "unsupported schema version",
    )
    require(
        data.get("claim_ceiling") == "synthetic_contract_conformance_only",
        "claim ceiling changed",
    )
    require(data.get("physical_probability_of_detection_claim") is False,
            "synthetic corpus cannot claim physical POD")
    require(data.get("aircraft_acceptance_claim") is False,
            "synthetic corpus cannot claim aircraft acceptance")

    contract = data.get("invariant_contract")
    require(isinstance(contract, dict), "invariant contract missing")
    require(contract.get("synthetic_POD_claim") == "FORBIDDEN", "synthetic POD ban weakened")
    require(contract.get("synthetic_sensor_reliability_probability") == "FORBIDDEN",
            "synthetic sensor probability ban weakened")
    require(contract.get("maintenance_release") == "NOT_AUTHORIZED",
            "maintenance-release boundary weakened")
    require(contract.get("required_region_coverage") == "ALL_CRITICAL_REGIONS",
            "critical coverage rule weakened")
    require(contract.get("correction_must_not_clear_damage_hypothesis") is True,
            "damage correction protection missing")
    require(contract.get("unknown_sensing_state_must_abstain") is True,
            "unknown sensing abstention missing")

    cases = data.get("cases")
    require(isinstance(cases, list) and cases, "cases missing")
    ids = [case.get("case_id") for case in cases]
    require(len(ids) == len(set(ids)), "duplicate case ID")
    require(set(ids) == set(EXPECTED_DISPOSITIONS), "case set changed without protocol revision")

    for case in cases:
        case_id = case["case_id"]
        require(case.get("evidence_class") == "synthetic", f"{case_id}: not marked synthetic")
        require(case.get("expected_disposition") == EXPECTED_DISPOSITIONS[case_id],
                f"{case_id}: expected disposition changed")
        article = case.get("article")
        inspection = case.get("inspection")
        sensing = case.get("sensing")
        require(isinstance(article, dict), f"{case_id}: article identity missing")
        require(isinstance(inspection, dict), f"{case_id}: inspection state missing")
        require(isinstance(sensing, dict), f"{case_id}: sensing state missing")
        for key in ("configuration_id", "geometry_generation", "material_family"):
            require(isinstance(article.get(key), str) and article[key],
                    f"{case_id}: missing article identity {key}")
        for key in (
            "procedure_current",
            "calibration_current",
            "coordinate_frame_match",
            "geometry_generation_match",
            "method_applicable",
            "critical_regions_observed",
        ):
            require(isinstance(inspection.get(key), bool), f"{case_id}: {key} must be boolean")
        coverage = inspection.get("surface_coverage_fraction")
        require(
            isinstance(coverage, (int, float)) and not isinstance(coverage, bool)
            and 0.0 <= coverage <= 1.0,
            f"{case_id}: invalid synthetic coverage fraction",
        )
        required = inspection.get("required_regions")
        observed = inspection.get("observed_regions")
        require(
            isinstance(required, list) and required
            and all(isinstance(item, str) and item for item in required),
            f"{case_id}: required region set invalid",
        )
        require(
            isinstance(observed, list)
            and all(isinstance(item, str) and item for item in observed),
            f"{case_id}: observed region set invalid",
        )
        for key in (
            "state_known",
            "structural_damage_candidate",
            "sensor_fault_candidate",
            "environment_shift",
            "correction_applied",
        ):
            require(isinstance(sensing.get(key), bool), f"{case_id}: {key} must be boolean")

        disposition = decide(case)
        require(
            disposition == case["expected_disposition"],
            f"{case_id}: oracle {disposition} != frozen expected {case['expected_disposition']}",
        )

        # A synthetic corpus may never emit a POD value, physical reliability
        # estimate, inspection acceptance, maintenance disposition or safety pass.
        for forbidden in ("pod", "pod90_95", "sensor_reliability_probability",
                          "inspection_accepted", "maintenance_release", "aircraft_safe"):
            require(forbidden not in case, f"{case_id}: forbidden physical claim field {forbidden}")


def run_hostile_controls(data: dict[str, Any]) -> int:
    cases = {case["case_id"]: case for case in data["cases"]}
    count = 0

    good = copy.deepcopy(cases["SYN-IO-001"])
    good["inspection"]["geometry_generation_match"] = False
    require(decide(good) == "REJECT_IDENTITY_MISMATCH", "stale geometry accepted")
    count += 1

    good = copy.deepcopy(cases["SYN-IO-001"])
    good["inspection"]["coordinate_frame_match"] = False
    require(decide(good) == "REJECT_IDENTITY_MISMATCH", "wrong coordinate frame accepted")
    count += 1

    good = copy.deepcopy(cases["SYN-IO-001"])
    good["inspection"]["calibration_current"] = False
    require(decide(good) == "REJECT_INVALID_INSPECTION_STATE", "expired calibration accepted")
    count += 1

    good = copy.deepcopy(cases["SYN-IO-001"])
    good["inspection"]["observed_regions"].remove("R-ROOT")
    good["inspection"]["surface_coverage_fraction"] = 0.99
    require(decide(good) == "UNRESOLVED_INCOMPLETE_CRITICAL_COVERAGE",
            "high aggregate coverage hid missing critical region")
    count += 1

    good = copy.deepcopy(cases["SYN-IO-001"])
    good["inspection"]["method_applicable"] = False
    require(decide(good) == "UNRESOLVED_METHOD_OUT_OF_DOMAIN", "out-of-domain method accepted")
    count += 1

    good = copy.deepcopy(cases["SYN-IO-001"])
    good["sensing"]["state_known"] = False
    require(decide(good) == "UNRESOLVED_SENSING_STATE", "unknown sensing state accepted")
    count += 1

    good = copy.deepcopy(cases["SYN-IO-008"])
    good["sensing"]["structural_damage_candidate"] = True
    good["sensing"]["correction_applied"] = True
    good["sensing"]["corrected_signal_looks_healthy"] = True
    require(decide(good) == "REQUIRE_INDEPENDENT_NDE",
            "signal correction erased plausible structural-damage hypothesis")
    count += 1

    good = copy.deepcopy(cases["SYN-IO-001"])
    good["sensing"]["sensor_fault_candidate"] = True
    good["sensing"]["correction_applied"] = True
    require(decide(good) == "SENSOR_FAULT_SUSPECTED_NO_STRUCTURAL_CLEARANCE",
            "sensor correction promoted a structural clearance")
    count += 1

    return count


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-output", type=Path)
    args = parser.parse_args()
    try:
        raw = CORPUS.read_bytes()
        data = json.loads(raw.decode("utf-8"))
        validate_corpus(data)
        hostile_count = run_hostile_controls(data)
        expected_dispositions = {
            case["case_id"]: decide(case) for case in data["cases"]
        }
        rust_boundary = "not_run"
        rust_output_sha256 = None

        if args.rust_output is not None:
            rust_bytes = args.rust_output.read_bytes()
            rust_result = json.loads(rust_bytes.decode("utf-8"))
            require(
                rust_result.get("schema_version") == "aero-inspection-observability-result/0.1.0",
                "Rust result schema version mismatch",
            )
            require(
                rust_result.get("corpus_sha256") == hashlib.sha256(raw).hexdigest(),
                "Rust result corpus digest mismatch",
            )
            require(
                rust_result.get("case_count") == len(data["cases"]),
                "Rust result case count mismatch",
            )
            require(
                rust_result.get("synthetic_dispositions") == expected_dispositions,
                "Rust/Python disposition map mismatch",
            )
            require(
                rust_result.get("physical_pod") == "NOT_ASSESSED",
                "Rust result promoted synthetic POD",
            )
            require(
                rust_result.get("sensor_reliability_probability") == "NOT_ASSESSED",
                "Rust result promoted synthetic sensor reliability",
            )
            require(
                rust_result.get("inspection_acceptance") == "NOT_A_REAL_INSPECTION",
                "Rust result promoted inspection acceptance",
            )
            require(
                rust_result.get("maintenance_release") == "NOT_AUTHORIZED",
                "Rust result promoted maintenance release",
            )
            require(
                rust_result.get("aircraft_status") == "NOT_ASSESSED",
                "Rust result promoted aircraft status",
            )
            require(
                rust_result.get("claim_ceiling") == "synthetic_contract_conformance_only",
                "Rust result claim ceiling changed",
            )
            rust_boundary = "pass"
            rust_output_sha256 = hashlib.sha256(rust_bytes).hexdigest()

        result = {
            "schema_version": "aero-inspection-observability-result/0.1.0",
            "corpus_sha256": hashlib.sha256(raw).hexdigest(),
            "case_count": len(data["cases"]),
            "hostile_control_count": hostile_count,
            "synthetic_dispositions": expected_dispositions,
            "rust_python_boundary": rust_boundary,
            "rust_output_sha256": rust_output_sha256,
            "physical_pod": "NOT_ASSESSED",
            "sensor_reliability_probability": "NOT_ASSESSED",
            "inspection_acceptance": "NOT_A_REAL_INSPECTION",
            "maintenance_release": "NOT_AUTHORIZED",
            "aircraft_status": "NOT_ASSESSED",
            "claim_ceiling": "synthetic_contract_conformance_only",
        }
        print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    except (
        OSError,
        UnicodeDecodeError,
        json.JSONDecodeError,
        ContractError,
        KeyError,
        TypeError,
        ValueError,
    ) as error:
        print(f"inspection_observability_error={error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
