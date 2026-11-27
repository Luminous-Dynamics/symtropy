#!/usr/bin/env python3
"""Independent standard-library oracle for typed inspection safety claims."""

import hashlib
import json
import sys
from pathlib import Path

CORPUS_SCHEMA = "luminous.orbital-inspection-safety-claim.corpus.v0.1"
RECEIPT_SCHEMA = "luminous.orbital-inspection-safety-claim.receipt.v0.1"
BENCHMARK_ID = "ORBITAL-INSPECTION-SAFETY-CLAIM-001"
CLAIM_CEILING = "synthetic_safety_claim_scope_and_component_composition_only"

SCOPE_TEXT_FIELDS = [
    "action_id",
    "action_revision",
    "vehicle_id",
    "vehicle_config_revision",
    "trajectory_revision",
    "horizon_id",
    "protected_object_id",
    "protected_object_revision",
    "keepout_profile_id",
    "keepout_geometry_revision",
    "reference_frame_id",
    "frame_revision",
    "transform_convention",
    "dynamics_revision",
    "initial_state_revision",
    "uncertainty_set_revision",
    "time_basis_id",
]
SCOPE_FIELDS = set(SCOPE_TEXT_FIELDS + [
    "action_start_tai_ns",
    "action_end_tai_ns",
    "required_clearance_um",
])
COMPONENTS = {
    "FINITE_HORIZON_KEEP_OUT",
    "SAFE_CONTINUATION",
    "STATISTICAL_RISK",
}


def valid_text(value):
    return isinstance(value, str) and bool(value.strip())


def valid_sha256(value):
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdefABCDEF" for character in value)
    )


def validate_scope(scope):
    if any(not valid_text(scope[name]) for name in SCOPE_TEXT_FIELDS):
        raise ValueError("ACTION_SCOPE_FIELD_EMPTY")
    if scope["action_start_tai_ns"] >= scope["action_end_tai_ns"]:
        raise ValueError("ACTION_INTERVAL_INVALID")
    if scope["required_clearance_um"] <= 0:
        raise ValueError("REQUIRED_CLEARANCE_MUST_BE_POSITIVE")


def validate_validity(interval):
    if (
        not valid_text(interval["time_basis_id"])
        or interval["start_tai_ns"] >= interval["end_tai_ns"]
    ):
        raise ValueError("CLAIM_VALIDITY_INTERVAL_INVALID")


def validate_claim_shape(claim):
    if not valid_text(claim["evidence_id"]) or not valid_sha256(
        claim["receipt_sha256"]
    ):
        raise ValueError("CLAIM_ID_OR_RECEIPT_INVALID")
    validate_scope(claim["scope"])
    validate_validity(claim["validity"])

    component = claim["component"]
    assurance = claim["assurance_class"]
    if component == "FINITE_HORIZON_KEEP_OUT":
        if (
            assurance != "ROBUST_TRAJECTORY_ENCLOSURE"
            or not claim["temporal_enclosure_complete"]
            or claim["minimum_clearance_lower_um"] is None
            or claim["required_clearance_um"] != claim["scope"]["required_clearance_um"]
            or claim["terminal_set_id"] is not None
            or claim["terminal_set_revision"] is not None
            or claim["terminal_epoch_tai_ns"] is not None
            or claim["terminal_invariant_proof_sha256"] is not None
            or claim["risk_measure"] is not None
        ):
            raise ValueError("FINITE_HORIZON_CLAIM_SHAPE_INVALID")
    elif component == "SAFE_CONTINUATION":
        if (
            assurance not in {"PASSIVE_INVARIANT_SET", "ABORT_REACHABLE_SET"}
            or not claim["temporal_enclosure_complete"]
            or not valid_text(claim["terminal_set_id"] or "")
            or not valid_text(claim["terminal_set_revision"] or "")
            or claim["terminal_epoch_tai_ns"] is None
            or not valid_sha256(claim["terminal_invariant_proof_sha256"] or "")
            or claim["minimum_clearance_lower_um"] is not None
            or claim["required_clearance_um"] is not None
            or claim["risk_measure"] is not None
        ):
            raise ValueError("SAFE_CONTINUATION_CLAIM_SHAPE_INVALID")
    elif component == "STATISTICAL_RISK":
        if (
            assurance != "PROBABILISTIC_RISK_BOUND"
            or not valid_text(claim["risk_measure"] or "")
            or claim["minimum_clearance_lower_um"] is not None
            or claim["required_clearance_um"] is not None
            or claim["terminal_set_id"] is not None
            or claim["terminal_set_revision"] is not None
            or claim["terminal_epoch_tai_ns"] is not None
            or claim["terminal_invariant_proof_sha256"] is not None
        ):
            raise ValueError("STATISTICAL_RISK_CLAIM_SHAPE_INVALID")
    else:
        raise ValueError("CLAIM_COMPONENT_INVALID")


def evaluate_claim(claim, action):
    def result(disposition, reasons=None):
        return {
            "evidence_id": claim["evidence_id"],
            "component": claim["component"],
            "assurance_class": claim["assurance_class"],
            "disposition": disposition,
            "reason_codes": sorted(reasons or []),
            "receipt_sha256": claim["receipt_sha256"],
        }

    try:
        validate_claim_shape(claim)
    except ValueError as error:
        return result("INVALID_INPUT", [str(error)])

    if claim["scope"] != action:
        return result("OUT_OF_SCOPE", ["ACTION_SCOPE_MISMATCH"])
    if claim["validity"]["time_basis_id"] != action["time_basis_id"]:
        return result("OUT_OF_SCOPE", ["TIME_BASIS_MISMATCH"])

    component = claim["component"]
    validity = claim["validity"]
    if component == "FINITE_HORIZON_KEEP_OUT":
        if (
            validity["start_tai_ns"] > action["action_start_tai_ns"]
            or validity["end_tai_ns"] < action["action_end_tai_ns"]
        ):
            return result(
                "OUT_OF_SCOPE", ["KEEP_OUT_CLAIM_DOES_NOT_COVER_ACTION_HORIZON"]
            )
        if claim["minimum_clearance_lower_um"] < claim["required_clearance_um"]:
            return result(
                "SAFETY_CLAIM_FAILED", ["ROBUST_CLEARANCE_BELOW_REQUIRED_MARGIN"]
            )
    elif component == "SAFE_CONTINUATION":
        if (
            claim["terminal_epoch_tai_ns"] != action["action_end_tai_ns"]
            or validity["start_tai_ns"] > action["action_end_tai_ns"]
            or validity["end_tai_ns"] <= action["action_end_tai_ns"]
        ):
            return result(
                "OUT_OF_SCOPE", ["TERMINAL_CLAIM_NOT_BOUND_TO_ACTION_END"]
            )
    elif component == "STATISTICAL_RISK":
        if (
            validity["start_tai_ns"] > action["action_start_tai_ns"]
            or validity["end_tai_ns"] < action["action_end_tai_ns"]
        ):
            return result(
                "OUT_OF_SCOPE", ["RISK_CLAIM_DOES_NOT_COVER_ACTION_HORIZON"]
            )
        state = claim["state"]
        if state == "PASS":
            return result(
                "RISK_ONLY",
                ["STATISTICAL_RISK_IS_NOT_A_DETERMINISTIC_SHIELD"],
            )
        if state == "FAIL":
            return result("SAFETY_CLAIM_FAILED", ["STATISTICAL_RISK_GATE_FAIL"])
        return result(
            "INDETERMINATE", ["STATISTICAL_RISK_SOURCE_UNRESOLVED"]
        )

    state = claim["state"]
    if state == "PASS":
        return result("SAFETY_CLAIMS_COMPLETE")
    if state == "FAIL":
        return result("SAFETY_CLAIM_FAILED", ["SAFETY_COMPONENT_FAIL"])
    if state == "UNKNOWN":
        return result("INDETERMINATE", ["SAFETY_COMPONENT_UNKNOWN"])
    if state == "STALE":
        return result("INDETERMINATE", ["SAFETY_COMPONENT_STALE"])
    return result("INDETERMINATE", ["SAFETY_COMPONENT_REVOKED"])


def evaluate_case(case):
    def invalid(reason):
        return {
            "case_id": case["case_id"],
            "disposition": "INVALID_INPUT",
            "finite_horizon_claim_present": False,
            "safe_continuation_claim_present": False,
            "risk_only_claim_present": False,
            "claim_results": [],
            "reason_codes": [reason],
        }

    if not valid_text(case["case_id"]):
        return invalid("CASE_ID_EMPTY")
    try:
        validate_scope(case["action_scope"])
    except ValueError as error:
        return invalid(str(error))

    evidence_ids = set()
    components = set()
    for claim in case["claims"]:
        if claim["evidence_id"] in evidence_ids:
            return invalid("EVIDENCE_ID_DUPLICATE")
        evidence_ids.add(claim["evidence_id"])
        if claim["component"] in components:
            return invalid("CLAIM_COMPONENT_DUPLICATE:" + claim["component"])
        components.add(claim["component"])

    claim_results = [
        evaluate_claim(claim, case["action_scope"]) for claim in case["claims"]
    ]
    claim_results.sort(key=lambda item: item["evidence_id"])

    finite_present = "FINITE_HORIZON_KEEP_OUT" in components
    continuation_present = "SAFE_CONTINUATION" in components
    risk_present = "STATISTICAL_RISK" in components

    reasons = set()
    for claim in claim_results:
        reasons.update(claim["reason_codes"])
    has_invalid = any(x["disposition"] == "INVALID_INPUT" for x in claim_results)
    has_out_of_scope = any(x["disposition"] == "OUT_OF_SCOPE" for x in claim_results)
    has_failed = any(x["disposition"] == "SAFETY_CLAIM_FAILED" for x in claim_results)
    has_indeterminate = any(x["disposition"] == "INDETERMINATE" for x in claim_results)

    if has_invalid:
        disposition = "INVALID_INPUT"
    elif has_out_of_scope:
        disposition = "OUT_OF_SCOPE"
    elif has_failed:
        disposition = "SAFETY_CLAIM_FAILED"
    elif has_indeterminate:
        disposition = "INDETERMINATE"
    elif finite_present and continuation_present:
        disposition = "SAFETY_CLAIMS_COMPLETE"
    elif risk_present and not finite_present and not continuation_present:
        disposition = "RISK_ONLY"
    else:
        disposition = "SAFETY_CLAIM_INCOMPLETE"

    if not finite_present:
        reasons.add("FINITE_HORIZON_KEEP_OUT_CLAIM_MISSING")
    if not continuation_present:
        reasons.add("SAFE_CONTINUATION_CLAIM_MISSING")

    return {
        "case_id": case["case_id"],
        "disposition": disposition,
        "finite_horizon_claim_present": finite_present,
        "safe_continuation_claim_present": continuation_present,
        "risk_only_claim_present": risk_present,
        "claim_results": claim_results,
        "reason_codes": sorted(reasons),
    }


def audit_corpus(raw_bytes, source_commit):
    if (
        len(source_commit) != 40
        or any(character not in "0123456789abcdefABCDEF" for character in source_commit)
    ):
        raise ValueError("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS")
    corpus = json.loads(raw_bytes)
    if (
        corpus["schema"] != CORPUS_SCHEMA
        or corpus["benchmark_id"] != BENCHMARK_ID
        or corpus["evidence_class"] != "synthetic"
        or corpus["claim_ceiling"] != CLAIM_CEILING
        or not corpus["cases"]
        or len(corpus["cases"]) > 128
    ):
        raise ValueError("CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID")

    case_ids = set()
    results = []
    matches = 0
    for case in corpus["cases"]:
        if case["case_id"] in case_ids:
            raise ValueError("CASE_ID_DUPLICATE")
        case_ids.add(case["case_id"])
        outcome = evaluate_case(case)
        if outcome["disposition"] == case["expected_disposition"]:
            matches += 1
        results.append(outcome)
    results.sort(key=lambda item: item["case_id"])

    return {
        "schema": RECEIPT_SCHEMA,
        "benchmark_id": BENCHMARK_ID,
        "source_commit": source_commit.lower(),
        "input_sha256": hashlib.sha256(raw_bytes).hexdigest(),
        "evidence_class": corpus["evidence_class"],
        "claim_ceiling": corpus["claim_ceiling"],
        "case_count": len(results),
        "expected_disposition_matches": matches,
        "expected_disposition_mismatches": len(results) - matches,
        "source_bytes_authenticated": False,
        "authority_granted": False,
        "physical_validation": "NOT_ASSESSED",
        "cases": results,
    }


def main():
    if len(sys.argv) != 3:
        raise SystemExit(
            "usage: reference_oracle.py <corpus.json> <40-hex-source-commit>"
        )
    raw = Path(sys.argv[1]).read_bytes()
    receipt = audit_corpus(raw, sys.argv[2])
    print(json.dumps(receipt, indent=2))
    if receipt["expected_disposition_mismatches"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
