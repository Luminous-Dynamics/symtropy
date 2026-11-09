#!/usr/bin/env python3
"""Independent standard-library oracle for environment/damage confounding."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
FIXTURE = HERE / "data" / "environment-damage-confounding-v0.1.json"
CLAIM = "synthetic_environment_damage_hypothesis_and_abstention_only"
EXPECTED_PROBE_QUANTITIES = [
    "wing_mean_temperature_delta",
    "wing_span_temperature_gradient",
    "wing_span_temperature_gradient",
]


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


def close(left: float, right: float, tolerance: float) -> bool:
    return abs(left - right) <= tolerance


REQUIRED_FAULT_DOMAIN_TYPES = (
    "power",
    "daq",
    "clock",
    "calibration_chain",
    "wiring_harness",
)


def validate_fault_domains(domains: list[dict[str, Any]]) -> set[tuple[str, str]]:
    require(len(domains) == len(REQUIRED_FAULT_DOMAIN_TYPES),
            "fault-domain manifest must contain five typed failure domains")
    types: set[str] = set()
    keys: set[tuple[str, str]] = set()
    for domain in domains:
        kind = domain.get("domain_type")
        identifier = domain.get("domain_id")
        require(kind in REQUIRED_FAULT_DOMAIN_TYPES
                and isinstance(identifier, str) and identifier.strip()
                and kind not in types,
                "fault-domain manifest has an unknown type, empty ID or duplicate type")
        types.add(kind)
        keys.add((kind, identifier))
    require(types == set(REQUIRED_FAULT_DOMAIN_TYPES),
            "fault-domain manifest is incomplete")
    return keys


def structural_fault_domain_keys(model: dict[str, Any]) -> set[tuple[str, str]]:
    records = model.get("structural_channel_fault_domains", [])
    require(len(records) == len(model["structural_channel_ids"]),
            "structural topology manifests must cover every channel")
    sensor_ids: set[str] = set()
    keys: set[tuple[str, str]] = set()
    for record in records:
        sensor_id = record.get("sensor_id")
        require(sensor_id in model["structural_channel_ids"] and sensor_id not in sensor_ids,
                "structural topology manifest has an unknown/duplicate sensor ID")
        sensor_ids.add(sensor_id)
        require(is_sha256(record.get("topology_evidence_sha256")),
                "structural topology evidence digest invalid")
        keys.update(validate_fault_domains(record["fault_domains"]))
    return keys


def validate(data: dict[str, Any]) -> None:
    require(
        data.get("schema_version") == "aero-causal-environment-damage-confounding/0.1.0",
        "unsupported environment/damage schema",
    )
    require(data.get("claim_ceiling") == CLAIM, "claim ceiling changed or weakened")
    require(isinstance(data.get("problem_id"), str) and data["problem_id"].strip(),
            "problem ID must be non-empty")
    model = data["model"]
    require(len(model["structural_channel_ids"]) == 3,
            "v0.1 requires three structural channels")
    require(len(model["structural_channel_common_cause_groups"]) == 3,
            "structural common-cause group dimension mismatch")
    require(model["structural_sensor_response_units"] == "normalized_response",
            "unsupported structural response units")
    require(len(set(model["structural_channel_ids"])) == 3,
            "duplicate structural sensor ID")
    require(len(set(model["structural_channel_common_cause_groups"])) == 3,
            "structural channels must have distinct common-cause groups")
    structural_fault_domain_keys(model)
    require(all(math.isfinite(x) for x in model["healthy_signature"]),
            "healthy signature contains non-finite value")
    require(all(math.isfinite(x) for x in model["damage_signature"]),
            "damage signature contains non-finite value")
    environment = model["environmental_model"]
    require(all(math.isfinite(x) for x in environment["mean_temperature_effect_per_kelvin"]),
            "mean-temperature response contains non-finite coefficient")
    require(all(math.isfinite(x) for x in environment["span_gradient_effect_per_kelvin"]),
            "gradient response contains non-finite coefficient")
    profiles = environment["profiles"]
    require(len(profiles) == 4, "v0.1 requires four environmental profiles")
    profile_ids = [profile["profile_id"] for profile in profiles]
    require(all(isinstance(value, str) and value.strip() for value in profile_ids),
            "environment profile IDs must be non-empty")
    require(len(profile_ids) == len(set(profile_ids)), "duplicate environmental profile ID")
    for profile in profiles:
        require(math.isfinite(profile["mean_temperature_delta_k"])
                and math.isfinite(profile["span_gradient_delta_k"]),
                "environment profile contains non-finite value")

    policy = data["policy"]
    for key in (
        "absolute_observation_tolerance",
        "environmental_profile_mean_tolerance_k",
        "environmental_profile_gradient_tolerance_k",
        "redundant_gradient_agreement_tolerance_k",
        "maximum_probe_standard_uncertainty_k",
    ):
        value = policy[key]
        require(isinstance(value, (int, float)) and math.isfinite(value) and value > 0.0,
                f"{key} must be finite and positive")
    require(policy["evidence_digest_algorithm"] == "sha256",
            "unsupported evidence digest algorithm")
    require(policy["witness_must_be_independent_of_structural_channels"] is True,
            "environmental-witness independence guard must remain enabled")
    require(policy["raw_structural_observations_immutable"] is True,
            "raw-observation immutability guard must remain enabled")
    require(policy["abstain_on_multiple_joint_hypotheses"] is True,
            "joint-hypothesis ambiguity must abstain")
    require(policy["required_environmental_probe_quantities"] == EXPECTED_PROBE_QUANTITIES,
            "required environmental probe set changed")

    cases = data["cases"]
    require(isinstance(cases, list) and 0 < len(cases) <= 128,
            "case count must be in 1..=128")
    identifiers: set[str] = set()
    for case in cases:
        identifier = case.get("case_id")
        require(isinstance(identifier, str) and identifier.strip(),
                "case_id must be non-empty")
        require(identifier not in identifiers, "duplicate case_id")
        identifiers.add(identifier)
        require(isinstance(case.get("acquisition_window_id"), str)
                and case["acquisition_window_id"].strip(),
                f"{identifier}: acquisition window missing")
        require(len(case["observed_values"]) == 3
                and all(math.isfinite(value) for value in case["observed_values"]),
                f"{identifier}: observed vector invalid")
        require(isinstance(case["environmental_witnesses"], list),
                f"{identifier}: environmental_witnesses must be a list")
        expected = case["expected"]
        require(isinstance(expected.get("disposition"), str),
                f"{identifier}: expected disposition missing")
        require(0 <= expected.get("candidate_count", -1) <= len(profiles) * 2,
                f"{identifier}: expected candidate count outside model")
        state = expected.get("structural_state")
        require(state is None or state in ("HEALTHY", "DAMAGED"),
                f"{identifier}: unsupported expected structural state")
        environment_state = expected.get("environmental_state")
        require(environment_state is None or environment_state in profile_ids,
                f"{identifier}: unsupported expected environment state")


def admit_witness(data: dict[str, Any], case: dict[str, Any],
                  witness: dict[str, Any]) -> dict[str, Any]:
    model = data["model"]
    policy = data["policy"]
    require(isinstance(witness.get("witness_id"), str) and witness["witness_id"].strip(),
            "ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS")
    probes = witness["probes"]
    require(len(probes) == 3, "ABSTAIN_INSUFFICIENT_ENVIRONMENTAL_OBSERVABILITY")
    structural_domains = structural_fault_domain_keys(model)
    seen_ids: set[str] = set()
    seen_groups: set[str] = set()
    seen_failure_domains: set[tuple[str, str]] = set()
    mean_probe = None
    gradient_probes = []
    for probe in probes:
        sensor_id = probe.get("sensor_id")
        group = probe.get("common_cause_group")
        require(isinstance(sensor_id, str) and sensor_id.strip()
                and sensor_id not in seen_ids
                and isinstance(group, str) and group.strip()
                and group not in seen_groups,
                "ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS")
        seen_ids.add(sensor_id)
        seen_groups.add(group)
        if (
            sensor_id in model["structural_channel_ids"]
            or group in model["structural_channel_common_cause_groups"]
        ):
            raise AuditError("ABSTAIN_ENVIRONMENTAL_WITNESS_NOT_INDEPENDENT")
        require(probe["independent_of_structural_channels"] is True,
                "ABSTAIN_ENVIRONMENTAL_WITNESS_NOT_INDEPENDENT")
        require(is_sha256(probe.get("topology_evidence_sha256")),
                "ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS")
        try:
            domain_keys = validate_fault_domains(probe["fault_domains"])
        except AuditError:
            raise AuditError("ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS")
        if (
            not domain_keys.isdisjoint(structural_domains)
            or not domain_keys.isdisjoint(seen_failure_domains)
        ):
            raise AuditError("ABSTAIN_ENVIRONMENTAL_WITNESS_NOT_INDEPENDENT")
        seen_failure_domains.update(domain_keys)
        require(probe["operator_status"] == "ADMITTED"
                and probe["calibration_status"] == "VALID"
                and is_sha256(probe["evidence_sha256"]),
                "ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS")
        reading = probe["reading_k"]
        uncertainty = probe["standard_uncertainty_k"]
        require(isinstance(reading, (int, float)) and math.isfinite(reading)
                and isinstance(uncertainty, (int, float))
                and math.isfinite(uncertainty)
                and 0.0 <= uncertainty <= policy["maximum_probe_standard_uncertainty_k"],
                "ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS")
        if probe["acquisition_window_id"] != case["acquisition_window_id"]:
            raise AuditError("ABSTAIN_ENVIRONMENT_WINDOW_MISMATCH")
        require(probe["spatial_support"] == "WING-THERMAL-CONTEXT-01",
                "ABSTAIN_ENVIRONMENT_WINDOW_MISMATCH")
        if probe["quantity"] == "wing_mean_temperature_delta":
            require(mean_probe is None, "ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS")
            mean_probe = probe
        elif probe["quantity"] == "wing_span_temperature_gradient":
            gradient_probes.append(probe)
        else:
            raise AuditError("ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS")

    require(mean_probe is not None and len(gradient_probes) == 2,
            "ABSTAIN_INSUFFICIENT_ENVIRONMENTAL_OBSERVABILITY")
    gradient_a = gradient_probes[0]["reading_k"]
    gradient_b = gradient_probes[1]["reading_k"]
    require(close(
        gradient_a,
        gradient_b,
        policy["redundant_gradient_agreement_tolerance_k"],
    ), "ABSTAIN_CONFLICTING_ENVIRONMENTAL_WITNESSES")
    gradient = 0.5 * (gradient_a + gradient_b)

    matches = [
        profile for profile in model["environmental_model"]["profiles"]
        if close(
            mean_probe["reading_k"],
            profile["mean_temperature_delta_k"],
            policy["environmental_profile_mean_tolerance_k"],
        )
        and close(
            gradient,
            profile["span_gradient_delta_k"],
            policy["environmental_profile_gradient_tolerance_k"],
        )
    ]
    require(len(matches) == 1, "ABSTAIN_ENVIRONMENT_OUT_OF_MODEL")
    return matches[0]


def environmental_effect(data: dict[str, Any], profile: dict[str, Any]) -> list[float]:
    model = data["model"]["environmental_model"]
    return [
        profile["mean_temperature_delta_k"] * model["mean_temperature_effect_per_kelvin"][i]
        + profile["span_gradient_delta_k"] * model["span_gradient_effect_per_kelvin"][i]
        for i in range(3)
    ]


def joint_candidates(data: dict[str, Any], observed: list[float],
                     profiles: list[dict[str, Any]]) -> list[dict[str, Any]]:
    model = data["model"]
    tolerance = data["policy"]["absolute_observation_tolerance"]
    candidates = []
    for state, signature in (
        ("HEALTHY", model["healthy_signature"]),
        ("DAMAGED", model["damage_signature"]),
    ):
        for profile in profiles:
            effect = environmental_effect(data, profile)
            predicted = [signature[i] + effect[i] for i in range(3)]
            residuals = [abs(observed[i] - predicted[i]) for i in range(3)]
            if all(value <= tolerance for value in residuals):
                candidates.append({
                    "structural_state": state,
                    "environmental_state": profile["profile_id"],
                    "predicted_values": predicted,
                    "maximum_absolute_residual": max(residuals),
                })
    return candidates


def abstain(case: dict[str, Any], disposition: str, witness_status: str) -> dict[str, Any]:
    return {
        "case_id": case["case_id"],
        "disposition": disposition,
        "structural_state": None,
        "environmental_state": None,
        "witness_status": witness_status,
        "observed_values": list(case["observed_values"]),
        "candidate_hypotheses": [],
        "claim_ceiling": CLAIM,
    }


def evaluate_case(data: dict[str, Any], case: dict[str, Any]) -> dict[str, Any]:
    profiles = data["model"]["environmental_model"]["profiles"]
    witnesses = case["environmental_witnesses"]
    admitted = []
    if not witnesses:
        admitted = profiles
    else:
        witness_ids: set[str] = set()
        probe_ids: set[str] = set()
        common_cause_groups: set[str] = set()
        failure_domain_keys: set[tuple[str, str]] = set()
        for witness in witnesses:
            if witness.get("witness_id") in witness_ids:
                return abstain(
                    case,
                    "ABSTAIN_ENVIRONMENTAL_WITNESS_NOT_INDEPENDENT",
                    "REJECTED",
                )
            witness_ids.add(witness.get("witness_id"))
            for probe in witness.get("probes", []):
                sensor_id = probe.get("sensor_id")
                group = probe.get("common_cause_group")
                try:
                    domains = validate_fault_domains(probe["fault_domains"])
                except AuditError:
                    return abstain(
                        case,
                        "ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS",
                        "REJECTED",
                    )
                if (
                    sensor_id in probe_ids
                    or group in common_cause_groups
                    or not failure_domain_keys.isdisjoint(domains)
                ):
                    return abstain(
                        case,
                        "ABSTAIN_ENVIRONMENTAL_WITNESS_NOT_INDEPENDENT",
                        "REJECTED",
                    )
                probe_ids.add(sensor_id)
                common_cause_groups.add(group)
                failure_domain_keys.update(domains)
            try:
                profile = admit_witness(data, case, witness)
            except AuditError as error:
                return abstain(case, str(error), "OUT_OF_MODEL" if str(error)
                               == "ABSTAIN_ENVIRONMENT_OUT_OF_MODEL" else "REJECTED")
            if any(existing["profile_id"] != profile["profile_id"] for existing in admitted):
                return abstain(
                    case,
                    "ABSTAIN_CONFLICTING_ENVIRONMENTAL_WITNESSES",
                    "CONFLICTING",
                )
            if not any(existing["profile_id"] == profile["profile_id"] for existing in admitted):
                admitted.append(profile)

    candidates = joint_candidates(data, case["observed_values"], admitted)
    witness_status = "NOT_AVAILABLE" if not witnesses else "ADMITTED"
    if not candidates:
        return abstain(
            case,
            "ABSTAIN_STRUCTURAL_ENVIRONMENT_MODEL_MISMATCH",
            witness_status,
        )
    if len(candidates) > 1:
        return {
            "case_id": case["case_id"],
            "disposition": "ABSTAIN_ENVIRONMENT_DAMAGE_AMBIGUOUS",
            "structural_state": None,
            "environmental_state": None,
            "witness_status": witness_status,
            "observed_values": list(case["observed_values"]),
            "candidate_hypotheses": candidates,
            "claim_ceiling": CLAIM,
        }

    selected = candidates[0]
    return {
        "case_id": case["case_id"],
        "disposition": (
            "STRUCTURAL_DAMAGE_SUSPECTED"
            if selected["structural_state"] == "DAMAGED"
            else "HEALTHY_WITHIN_DECLARED_SUPPORT"
        ),
        "structural_state": selected["structural_state"],
        "environmental_state": selected["environmental_state"],
        "witness_status": witness_status,
        "observed_values": list(case["observed_values"]),
        "candidate_hypotheses": [selected],
        "claim_ceiling": CLAIM,
    }


def evaluate(data: dict[str, Any], raw: bytes) -> dict[str, Any]:
    validate(data)
    cases = [evaluate_case(data, case) for case in data["cases"]]
    counts: dict[str, int] = {}
    for case in cases:
        counts[case["disposition"]] = counts.get(case["disposition"], 0) + 1
    return {
        "schema_version": "aero-causal-environment-damage-confounding-result/0.1.0",
        "problem_id": data["problem_id"],
        "source_sha256": hashlib.sha256(raw).hexdigest(),
        "source_bytes": len(raw),
        "case_count": len(cases),
        "disposition_counts": [list(item) for item in sorted(counts.items())],
        "cases": cases,
        "claim_ceiling": CLAIM,
    }


def compare(actual: Any, expected: Any, path: str = "root") -> None:
    if isinstance(expected, float):
        require(
            isinstance(actual, (int, float))
            and math.isclose(float(actual), expected, rel_tol=1e-10, abs_tol=1e-12),
            f"{path}: {actual!r} != {expected!r}",
        )
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
    ambiguous = next(case for case in data["cases"]
                     if case["case_id"] == "AMBIGUOUS_DAMAGE_VS_THERMAL_GRADIENT_NO_WITNESS")
    result = evaluate_case(data, ambiguous)
    require(result["disposition"] == "ABSTAIN_ENVIRONMENT_DAMAGE_AMBIGUOUS"
            and len(result["candidate_hypotheses"]) == 2,
            "unwitnessed damage/environment collision was promoted")

    damaged = next(case for case in data["cases"]
                   if case["case_id"] == "DH_NOMINAL_ENVIRONMENT")
    wrong_group = copy.deepcopy(damaged)
    wrong_group["environmental_witnesses"][0]["probes"][0]["common_cause_group"] = "STRAIN-BUS-A"
    result = evaluate_case(data, wrong_group)
    require(result["disposition"] == "ABSTAIN_ENVIRONMENTAL_WITNESS_NOT_INDEPENDENT",
            "environmental witness sharing a structural common cause was accepted")

    wrong_daq = copy.deepcopy(damaged)
    structural = next(
        item for item in data["model"]["structural_channel_fault_domains"]
        if item["sensor_id"] == "S1"
    )
    wrong_daq["environmental_witnesses"][0]["probes"][0]["fault_domains"] = copy.deepcopy(
        structural["fault_domains"]
    )
    result = evaluate_case(data, wrong_daq)
    require(result["disposition"] == "ABSTAIN_ENVIRONMENTAL_WITNESS_NOT_INDEPENDENT",
            "environmental probe sharing structural DAQ/power/clock topology was accepted")

    cross_witness = next(
        case for case in data["cases"]
        if case["case_id"] == "DUPLICATE_COMMON_CAUSE_ENVIRONMENTAL_WITNESSES"
    )
    result = evaluate_case(data, cross_witness)
    require(result["disposition"] == "ABSTAIN_ENVIRONMENTAL_WITNESS_NOT_INDEPENDENT",
            "separate witnesses sharing one typed failure domain were accepted")

    expired = copy.deepcopy(damaged)
    expired["environmental_witnesses"][0]["probes"][0]["calibration_status"] = "EXPIRED"
    result = evaluate_case(data, expired)
    require(result["disposition"] == "ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS",
            "expired environmental calibration was accepted")

    bad_digest = copy.deepcopy(damaged)
    bad_digest["environmental_witnesses"][0]["probes"][0]["evidence_sha256"] = "bad"
    result = evaluate_case(data, bad_digest)
    require(result["disposition"] == "ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS",
            "invalid environmental evidence digest was accepted")

    bad_topology_digest = copy.deepcopy(damaged)
    bad_topology_digest["environmental_witnesses"][0]["probes"][0]["topology_evidence_sha256"] = "bad"
    result = evaluate_case(data, bad_topology_digest)
    require(result["disposition"] == "ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS",
            "invalid topology evidence digest was accepted")

    incomplete_domains = copy.deepcopy(damaged)
    incomplete_domains["environmental_witnesses"][0]["probes"][0]["fault_domains"].pop()
    result = evaluate_case(data, incomplete_domains)
    require(result["disposition"] == "ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS",
            "incomplete fault-domain manifest was accepted")

    shared = next(case for case in data["cases"]
                  if case["case_id"] == "DUPLICATE_COMMON_CAUSE_ENVIRONMENTAL_WITNESSES")
    result = evaluate_case(data, shared)
    require(result["disposition"] == "ABSTAIN_ENVIRONMENTAL_WITNESS_NOT_INDEPENDENT",
            "separate witnesses with a shared common-cause group were accepted")

    print("negative_control=pass case=damage_environment_ambiguity")
    print("negative_control=pass case=shared_structural_daq_failure_domain")
    print("negative_control=pass case=independent_witness_shared_clock_failure_domain")
    print("negative_control=pass case=expired_calibration")
    print("negative_control=pass case=invalid_measurement_and_topology_digests")
    print("negative_control=pass case=incomplete_fault_domain_manifest")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-output", type=Path, required=True)
    args = parser.parse_args()
    try:
        raw = FIXTURE.read_bytes()
        data = json.loads(raw)
        expected = evaluate(data, raw)
        require(expected["case_count"] == 17, "frozen case count changed")
        by_id = {case["case_id"]: case for case in expected["cases"]}
        require(by_id["APPARENT_DAMAGE_EXPLAINED_BY_THERMAL_GRADIENT"]["structural_state"]
                == "HEALTHY", "thermal-gradient counterexample changed")
        require(by_id["AMBIGUOUS_DAMAGE_VS_THERMAL_GRADIENT_NO_WITNESS"]["disposition"]
                == "ABSTAIN_ENVIRONMENT_DAMAGE_AMBIGUOUS",
                "unwitnessed thermal/damage ambiguity changed")
        require(by_id["DH_NOMINAL_ENVIRONMENT"]["structural_state"] == "DAMAGED",
                "nominal-environment damage known answer changed")
        actual = json.loads(args.rust_output.read_text(encoding="utf-8"))
        compare(actual, expected)
        hostile_controls(data)
        print(
            "aero_environment_damage_confounding=pass cases=17 "
            "joint_states=structure_x_environment "
            "damage_gradient_collision=abstain rust_python=pass"
        )
        return 0
    except (OSError, json.JSONDecodeError, AuditError, KeyError, TypeError, ValueError) as error:
        print(f"environment_damage_confounding_error={error}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
