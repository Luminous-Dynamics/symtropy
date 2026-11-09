#!/usr/bin/env python3
"""Independent standard-library oracle for joint structural/sensor health states."""

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
FIXTURE = HERE / "data" / "hh-hf-dh-df-v0.1.json"
CLAIM_CEILING = "synthetic_joint_structure_sensor_state_and_abstention_only"


class AuditError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AuditError(message)


def is_sha256(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value)
    )


def validate(data: dict[str, Any]) -> None:
    require(data.get("schema_version") == "aero-virtual-sensing-health-state/0.1.0",
            "unsupported schema version")
    require(isinstance(data.get("model_id"), str) and data["model_id"].strip(),
            "model_id must be non-empty")
    require(data.get("model_scope") == "synthetic_static_response_template_hypothesis_test",
            "unsupported model scope")
    require(data.get("claim_ceiling") == CLAIM_CEILING,
            "claim ceiling changed or weakened")

    assumptions = data.get("assumptions")
    require(isinstance(assumptions, dict), "assumptions must be an object")
    require(assumptions.get("observation_model") == "static_normalized_sensor_response",
            "unsupported observation model")
    require(assumptions.get("fault_model") == "at_most_one_arbitrary_channel_residual",
            "unsupported sensor fault model")
    require(assumptions.get("structural_states") == ["HEALTHY", "DAMAGED"],
            "structural-state domain changed")
    require(assumptions.get("response_units") == "normalized_response",
            "unsupported response units")
    require(assumptions.get("operator_status") == "ADMITTED"
            and assumptions.get("calibration_status") == "VALID",
            "unsupported channel admission assumptions")

    channels = data.get("channels")
    require(isinstance(channels, list) and len(channels) == 3,
            "v0.1 requires exactly three channels")
    ids = [channel.get("sensor_id") for channel in channels]
    require(all(isinstance(identifier, str) and identifier.strip() for identifier in ids),
            "sensor IDs must be non-empty strings")
    require(ids == assumptions.get("channel_order"),
            "channel registry order differs from declared channel order")
    require(len(ids) == len(set(ids)), "sensor IDs must be unique")
    for channel in channels:
        require(isinstance(channel.get("quantity"), str) and channel["quantity"].strip(),
                "channel quantity must be non-empty")
        require(channel.get("unit") == "normalized_response",
                "channel unit is not supported")
        require(isinstance(channel.get("common_cause_group"), str)
                and channel["common_cause_group"].strip(),
                "common-cause group ID must be non-empty")
        require(is_sha256(channel.get("operator_sha256")),
                "invalid operator digest")
        require(channel.get("operator_status") in ("ADMITTED", "UNKNOWN"),
                "unsupported operator status")
        require(channel.get("calibration_status") in ("VALID", "EXPIRED", "UNKNOWN"),
                "unsupported calibration status")

    policy = data.get("policy")
    require(isinstance(policy, dict), "policy must be an object")
    require(policy.get("maximum_faulty_channels") == 1,
            "v0.1 supports one arbitrary faulty channel")
    require(isinstance(policy.get("minimum_consistent_channels"), int)
            and 2 <= policy["minimum_consistent_channels"] <= len(channels),
            "minimum consistent channel count outside supported range")
    tolerance = policy.get("absolute_residual_tolerance")
    require(isinstance(tolerance, (int, float)) and math.isfinite(tolerance) and tolerance > 0,
            "residual tolerance must be finite and positive")
    require(policy.get("reconstruction_requires_unique_joint_hypothesis") is True,
            "correction must require a unique joint hypothesis")
    require(policy.get("damage_clearance_from_reconstruction_prohibited") is True,
            "damage-preservation guard may not be disabled")
    require(policy.get("fault_confirmation_requires_independent_witness") is True,
            "independent fault witness must be required")
    require(policy.get("raw_measurements_immutable") is True,
            "raw measurements must remain immutable")
    require(policy.get("reconstruction_is_not_measurement") is True,
            "model reconstruction must not be represented as a measurement")

    cases = data.get("cases")
    require(isinstance(cases, list) and 0 < len(cases) <= 64,
            "case count must be in 1..=64")
    case_ids: set[str] = set()
    for case in cases:
        identifier = case.get("case_id")
        require(isinstance(identifier, str) and identifier.strip(),
                "case_id must be non-empty")
        require(identifier not in case_ids, "duplicate case_id")
        case_ids.add(identifier)
        for field in ("healthy_template", "damaged_template", "observed_values"):
            values = case.get(field)
            require(isinstance(values, list) and len(values) == len(channels),
                    f"{identifier}: {field} dimension mismatch")
        for field in ("healthy_template", "damaged_template"):
            require(all(isinstance(value, (int, float)) and math.isfinite(value)
                        for value in case[field]),
                    f"{identifier}: {field} contains non-finite values")
        require(all(value is None or (
            isinstance(value, (int, float)) and math.isfinite(value)
        ) for value in case["observed_values"]),
                f"{identifier}: observation contains non-finite value")
        witnesses = case.get("sensor_fault_witnesses")
        require(isinstance(witnesses, list), f"{identifier}: sensor_fault_witnesses must be a list")
        witness_ids: set[str] = set()
        for witness in witnesses:
            sensor_id = witness.get("sensor_id")
            require(sensor_id in ids and sensor_id not in witness_ids,
                    f"{identifier}: unknown or duplicate sensor-fault witness")
            witness_ids.add(sensor_id)
            require(isinstance(witness.get("evidence_kind"), str)
                    and witness["evidence_kind"].strip(),
                    f"{identifier}: witness evidence kind missing")
            if witness.get("status") == "FAULT_CONFIRMED":
                require(witness.get("evidence_kind") == "independent_builtin_self_test",
                        f"{identifier}: unsupported fault-witness mechanism")
                require(is_sha256(witness.get("evidence_sha256")),
                        f"{identifier}: confirmed fault witness requires SHA-256 evidence")
            elif witness.get("status") == "NOT_CONFIRMED":
                require(witness.get("evidence_sha256") is None,
                        f"{identifier}: unconfirmed witness cannot carry a confirmation digest")
            else:
                raise AuditError(f"{identifier}: unsupported sensor-fault witness status")
        expected = case.get("expected")
        require(isinstance(expected, dict), f"{identifier}: expected answer missing")
        require(isinstance(expected.get("disposition"), str),
                f"{identifier}: expected disposition missing")


def abstain(case: dict[str, Any], disposition: str,
           explanation_count: int = 0,
           sensor_fault_confirmation: str = "NOT_ASSESSED") -> dict[str, Any]:
    return {
        "case_id": case["case_id"],
        "disposition": disposition,
        "structural_state": None,
        "faulty_sensor_ids": [],
        "explanation_count": explanation_count,
        "consistent_channel_count": None,
        "maximum_consistent_residual": None,
        "observed_values": list(case["observed_values"]),
        "model_imputed_values": None,
        "reconstructed_sensor_ids": [],
        "reconstruction_status": "NOT_PERFORMED",
        "sensor_fault_confirmation": sensor_fault_confirmation,
        "post_reconstruction_structural_state": None,
        "damage_preserved": None,
        "claim_ceiling": CLAIM_CEILING,
    }


def explanation_for_template(data: dict[str, Any], case: dict[str, Any],
                             structural_state: str, template: list[float],
                             observed_indices: list[int]) -> dict[str, Any] | None:
    faulty = []
    consistent_count = 0
    max_consistent_residual = 0.0
    tolerance = data["policy"]["absolute_residual_tolerance"]
    for index in observed_indices:
        residual = abs(case["observed_values"][index] - template[index])
        if residual > tolerance:
            faulty.append(index)
        else:
            consistent_count += 1
            max_consistent_residual = max(max_consistent_residual, residual)

    if len(faulty) > data["policy"]["maximum_faulty_channels"]:
        return None
    if consistent_count < data["policy"]["minimum_consistent_channels"]:
        return None
    return {
        "structural_state": structural_state,
        "template": template,
        "faulty_indices": faulty,
        "consistent_count": consistent_count,
        "maximum_consistent_residual": max_consistent_residual,
    }


def evaluate_case(data: dict[str, Any], case: dict[str, Any]) -> dict[str, Any]:
    channels = data["channels"]
    observed_indices = [
        index for index, value in enumerate(case["observed_values"])
        if value is not None
    ]
    if len(observed_indices) < data["policy"]["minimum_consistent_channels"]:
        return abstain(case, "ABSTAIN_INSUFFICIENT_OBSERVABILITY")

    # Do not reconstruct unqualified observations into apparently admissible values.
    if any(
        channels[index]["operator_status"] != "ADMITTED"
        or channels[index]["calibration_status"] != "VALID"
        for index in observed_indices
    ):
        return abstain(case, "ABSTAIN_UNQUALIFIED_MEASUREMENT_CHAIN")

    explanations = []
    healthy = explanation_for_template(
        data, case, "HEALTHY", case["healthy_template"], observed_indices
    )
    damaged = explanation_for_template(
        data, case, "DAMAGED", case["damaged_template"], observed_indices
    )
    if healthy is not None:
        explanations.append(healthy)
    if damaged is not None:
        explanations.append(damaged)
    if not explanations:
        return abstain(case, "ABSTAIN_OUT_OF_MODEL")
    if len(explanations) > 1:
        return abstain(case, "ABSTAIN_AMBIGUOUS_HYPOTHESES", len(explanations))

    explanation = explanations[0]
    faulty_ids = [channels[index]["sensor_id"] for index in explanation["faulty_indices"]]
    if faulty_ids:
        witnesses = {
            item["sensor_id"]: item
            for item in case["sensor_fault_witnesses"]
        }
        confirmed = all(
            sensor_id in witnesses
            and witnesses[sensor_id]["status"] == "FAULT_CONFIRMED"
            and witnesses[sensor_id]["evidence_kind"] == "independent_builtin_self_test"
            and is_sha256(witnesses[sensor_id]["evidence_sha256"])
            for sensor_id in faulty_ids
        )
        if not confirmed:
            return abstain(
                case,
                "ABSTAIN_SENSOR_FAULT_UNCONFIRMED",
                1,
                "NOT_CONFIRMED",
            )

    state = explanation["structural_state"]
    if state == "HEALTHY" and not faulty_ids:
        disposition = "HEALTHY_WITHIN_DECLARED_SYNTHETIC_SUPPORT"
    elif state == "HEALTHY":
        disposition = "SENSOR_FAULT_WITNESS_CONFIRMED"
    elif not faulty_ids:
        disposition = "STRUCTURAL_DAMAGE_SUSPECTED"
    else:
        disposition = "DAMAGE_AND_SENSOR_FAULT_WITNESS_CONFIRMED"

    # Preserve raw values. A model-imputed value is separate and not a measurement.
    imputed = None
    reconstruction_status = "NOT_APPLICABLE"
    if faulty_ids:
        imputed = [None] * len(case["observed_values"])
        for index in explanation["faulty_indices"]:
            imputed[index] = explanation["template"][index]
        reconstruction_status = "CONDITIONAL_MODEL_ESTIMATE_NOT_MEASUREMENT"

    return {
        "case_id": case["case_id"],
        "disposition": disposition,
        "structural_state": state,
        "faulty_sensor_ids": faulty_ids,
        "explanation_count": 1,
        "consistent_channel_count": explanation["consistent_count"],
        "maximum_consistent_residual": explanation["maximum_consistent_residual"],
        "observed_values": list(case["observed_values"]),
        "model_imputed_values": imputed,
        "reconstructed_sensor_ids": faulty_ids,
        "reconstruction_status": reconstruction_status,
        "sensor_fault_confirmation": (
            "INDEPENDENT_WITNESS_CONFIRMED" if faulty_ids
            else "NO_SENSOR_FAULT_HYPOTHESIS"
        ),
        "post_reconstruction_structural_state": state,
        "damage_preserved": state == "DAMAGED",
        "claim_ceiling": CLAIM_CEILING,
    }


def evaluate(data: dict[str, Any], raw: bytes) -> dict[str, Any]:
    validate(data)
    results = [evaluate_case(data, case) for case in data["cases"]]
    counts: dict[str, int] = {}
    for result in results:
        counts[result["disposition"]] = counts.get(result["disposition"], 0) + 1
    return {
        "schema_version": "aero-virtual-sensing-health-state-result/0.1.0",
        "model_id": data["model_id"],
        "source_sha256": hashlib.sha256(raw).hexdigest(),
        "source_bytes": len(raw),
        "channel_count": len(data["channels"]),
        "case_count": len(results),
        "disposition_counts": [list(item) for item in sorted(counts.items())],
        "cases": results,
        "claim_ceiling": CLAIM_CEILING,
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
    # The ambiguity counterexample must remain ambiguous, not be "resolved" by
    # applying a sensor correction before structural-state inference.
    ambiguous = next(case for case in data["cases"]
                     if case["case_id"] == "AMBIGUOUS_DAMAGE_VS_SINGLE_SENSOR_FAULT")
    result = evaluate_case(data, ambiguous)
    require(result["disposition"] == "ABSTAIN_AMBIGUOUS_HYPOTHESES",
            "ambiguous damage-vs-sensor case was promoted")
    require(result["model_imputed_values"] is None,
            "ambiguous case received a virtual reconstruction")

    unqualified = copy.deepcopy(data)
    unqualified["channels"][1]["operator_status"] = "UNKNOWN"
    result = evaluate_case(unqualified, unqualified["cases"][1])
    require(result["disposition"] == "ABSTAIN_UNQUALIFIED_MEASUREMENT_CHAIN",
            "unqualified operator was accepted")
    require(result["model_imputed_values"] is None,
            "unqualified operator was reconstructed")

    expired = copy.deepcopy(data)
    expired["channels"][1]["calibration_status"] = "EXPIRED"
    result = evaluate_case(expired, expired["cases"][1])
    require(result["disposition"] == "ABSTAIN_UNQUALIFIED_MEASUREMENT_CHAIN",
            "expired calibration was accepted")
    require(result["model_imputed_values"] is None,
            "expired channel was reconstructed")

    deficient = copy.deepcopy(data)
    deficient["cases"][0]["observed_values"] = [0.0, None, None]
    result = evaluate_case(deficient, deficient["cases"][0])
    require(result["disposition"] == "ABSTAIN_INSUFFICIENT_OBSERVABILITY",
            "insufficient observations were promoted")

    out_of_model = next(case for case in data["cases"]
                        if case["case_id"] == "OUT_OF_MODEL_ENVIRONMENTAL_OR_UNKNOWN_SHIFT")
    result = evaluate_case(data, out_of_model)
    require(result["disposition"] == "ABSTAIN_OUT_OF_MODEL",
            "unknown operating/environmental shift was mislabeled as damage")

    df = next(case for case in data["cases"]
              if case["case_id"] == "DF_DAMAGED_STRUCTURE_FAULTY_SENSOR")
    result = evaluate_case(data, df)
    require(result["post_reconstruction_structural_state"] == "DAMAGED"
            and result["damage_preserved"] is True,
            "sensor reconstruction erased a damage hypothesis")
    require(result["observed_values"][1] == 1.8
            and result["model_imputed_values"][1] == 1.0,
            "raw observation was overwritten or imputation mislabeled")

    unwitnessed = next(case for case in data["cases"]
                       if case["case_id"] == "UNWITNESSED_LOCALIZED_ANOMALY")
    result = evaluate_case(data, unwitnessed)
    require(result["disposition"] == "ABSTAIN_SENSOR_FAULT_UNCONFIRMED"
            and result["model_imputed_values"] is None,
            "single-channel residual was treated as proof of sensor failure")

    wrong_witness = copy.deepcopy(data)
    hf = next(case for case in wrong_witness["cases"]
              if case["case_id"] == "HF_HEALTHY_STRUCTURE_FAULTY_SENSOR")
    hf["sensor_fault_witnesses"][0]["sensor_id"] = "S1"
    result = evaluate_case(wrong_witness, hf)
    require(result["disposition"] == "ABSTAIN_SENSOR_FAULT_UNCONFIRMED"
            and result["model_imputed_values"] is None,
            "a witness for the wrong channel confirmed a sensor fault")
    print("negative_control=pass case=ambiguous_damage_sensor")
    print("negative_control=pass case=unqualified_operator_and_calibration")
    print("negative_control=pass case=insufficient_observability")
    print("negative_control=pass case=out_of_model_shift")
    print("negative_control=pass case=damage_preserved_after_correction")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-output", type=Path, required=True)
    args = parser.parse_args()
    try:
        raw = FIXTURE.read_bytes()
        data = json.loads(raw)
        expected = evaluate(data, raw)
        require(expected["case_count"] == 8, "frozen case count changed")
        by_id = {case["case_id"]: case for case in expected["cases"]}
        require(by_id["HH_HEALTHY_STRUCTURE_HEALTHY_SENSORS"]["disposition"]
                == "HEALTHY_WITHIN_DECLARED_SYNTHETIC_SUPPORT", "HH known answer changed")
        require(by_id["HF_HEALTHY_STRUCTURE_FAULTY_SENSOR"]["faulty_sensor_ids"] == ["S2"],
                "HF channel known answer changed")
        require(by_id["HF_HEALTHY_STRUCTURE_FAULTY_SENSOR"]["disposition"]
                == "SENSOR_FAULT_WITNESS_CONFIRMED", "HF witness disposition changed")
        require(by_id["HF_HEALTHY_STRUCTURE_FAULTY_SENSOR"]["observed_values"][1] == 0.8
                and by_id["HF_HEALTHY_STRUCTURE_FAULTY_SENSOR"]["model_imputed_values"][1] == 0.0,
                "HF raw observation/imputation separation changed")
        require(by_id["DH_DAMAGED_STRUCTURE_HEALTHY_SENSORS"]["structural_state"] == "DAMAGED",
                "DH known answer changed")
        require(by_id["DF_DAMAGED_STRUCTURE_FAULTY_SENSOR"]["structural_state"] == "DAMAGED"
                and by_id["DF_DAMAGED_STRUCTURE_FAULTY_SENSOR"]["faulty_sensor_ids"] == ["S2"]
                and by_id["DF_DAMAGED_STRUCTURE_FAULTY_SENSOR"]["disposition"]
                == "DAMAGE_AND_SENSOR_FAULT_WITNESS_CONFIRMED",
                "DF joint known answer changed")
        require(by_id["UNWITNESSED_LOCALIZED_ANOMALY"]["disposition"]
                == "ABSTAIN_SENSOR_FAULT_UNCONFIRMED",
                "unwitnessed localized anomaly was promoted")
        require(by_id["AMBIGUOUS_DAMAGE_VS_SINGLE_SENSOR_FAULT"]["disposition"]
                == "ABSTAIN_AMBIGUOUS_HYPOTHESES", "ambiguity known answer changed")
        actual = json.loads(args.rust_output.read_text(encoding="utf-8"))
        compare(actual, expected)
        hostile_controls(data)
        print("aero_virtual_sensing_health_state=pass cases=8 "
              "HH=healthy HF=sensor_fault DH=damage DF=damage_plus_sensor_fault "
              "ambiguous=abstain out_of_model=abstain rust_python=pass")
        return 0
    except (OSError, json.JSONDecodeError, AuditError, KeyError, TypeError, ValueError) as error:
        print(f"virtual_sensing_health_state_error={error}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
