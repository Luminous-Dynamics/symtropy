#!/usr/bin/env python3
"""Independent standard-library oracle for exact acquisition-scope coherence."""

import hashlib
import json
import sys
from pathlib import Path

CORPUS_SCHEMA = "luminous.orbital-acquisition-scope-coherence.corpus.v0.1"
RECEIPT_SCHEMA = "luminous.orbital-acquisition-scope-coherence.receipt.v0.1"
BENCHMARK_ID = "ORBITAL-ACQUISITION-SCOPE-COHERENCE-001"
CLAIM_CEILING = "synthetic_exact_identity_and_admitted_interval_composition_only"

CANONICAL_SCOPE_FIELDS = {
    "acquisition_id",
    "camera_id",
    "sensor_mode_id",
    "intrinsics_revision",
    "robot_id",
    "mount_revision",
    "asset_id",
    "asset_revision",
    "camera_frame_id",
    "asset_frame_id",
    "transform_convention",
    "feature_id",
    "support_id",
    "motion_model_revision",
    "calibration_scope_id",
    "clock_map_id",
    "time_basis_id",
    "exposure_profile_id",
    "projector_profile_id",
    "pattern_sequence_id",
}

REQUIRED_FIELDS = {
    "MOTION": {
        "acquisition_id",
        "camera_id",
        "sensor_mode_id",
        "robot_id",
        "mount_revision",
        "asset_id",
        "asset_revision",
        "camera_frame_id",
        "asset_frame_id",
        "transform_convention",
        "feature_id",
        "support_id",
        "motion_model_revision",
        "clock_map_id",
        "time_basis_id",
    },
    "SUPPORT": {
        "asset_id",
        "asset_revision",
        "camera_frame_id",
        "asset_frame_id",
        "transform_convention",
        "feature_id",
        "support_id",
    },
    "CALIBRATION": {
        "camera_id",
        "sensor_mode_id",
        "intrinsics_revision",
        "robot_id",
        "mount_revision",
        "asset_id",
        "asset_revision",
        "camera_frame_id",
        "asset_frame_id",
        "calibration_scope_id",
        "clock_map_id",
        "time_basis_id",
    },
    "TIMING": {
        "acquisition_id",
        "camera_id",
        "sensor_mode_id",
        "clock_map_id",
        "time_basis_id",
        "exposure_profile_id",
        "projector_profile_id",
        "pattern_sequence_id",
    },
}
REQUIRED_KINDS = {"MOTION", "SUPPORT", "CALIBRATION", "TIMING"}


def valid_text(value):
    return isinstance(value, str) and bool(value.strip())


def valid_sha256(value):
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdefABCDEF" for character in value)
    )


def claims_to_map(claims):
    values = {}
    for claim in claims:
        field = claim["field"]
        value = claim["value"]
        if not valid_text(field) or not valid_text(value):
            raise ValueError("SCOPE_CLAIM_EMPTY")
        if field not in CANONICAL_SCOPE_FIELDS:
            raise ValueError("SCOPE_FIELD_UNSUPPORTED:" + field)
        if field in values:
            raise ValueError("SCOPE_FIELD_DUPLICATE:" + field)
        values[field] = value
    return values


def validate_interval(interval):
    if (
        not valid_text(interval["time_basis_id"])
        or interval["start_tai_ns"] >= interval["end_tai_ns"]
    ):
        raise ValueError("TIME_INTERVAL_INVALID")


def evaluate_binding(binding, acquisition_interval, acquisition_scope):
    reasons = set()
    missing = set()
    mismatched = set()

    if not valid_text(binding["evidence_id"]) or not valid_sha256(
        binding["receipt_sha256"]
    ):
        return {
            "evidence_id": binding["evidence_id"],
            "kind": binding["kind"],
            "disposition": "INVALID_INPUT",
            "missing_fields": [],
            "mismatched_fields": [],
            "reason_codes": ["BINDING_ID_OR_RECEIPT_INVALID"],
            "receipt_sha256": binding["receipt_sha256"],
        }

    try:
        validate_interval(binding["validity_interval"])
    except ValueError as error:
        return {
            "evidence_id": binding["evidence_id"],
            "kind": binding["kind"],
            "disposition": "INVALID_INPUT",
            "missing_fields": [],
            "mismatched_fields": [],
            "reason_codes": [str(error)],
            "receipt_sha256": binding["receipt_sha256"],
        }

    try:
        evidence_scope = claims_to_map(binding["scope_claims"])
    except ValueError as error:
        return {
            "evidence_id": binding["evidence_id"],
            "kind": binding["kind"],
            "disposition": "INVALID_INPUT",
            "missing_fields": [],
            "mismatched_fields": [],
            "reason_codes": [str(error)],
            "receipt_sha256": binding["receipt_sha256"],
        }

    for field in sorted(REQUIRED_FIELDS[binding["kind"]]):
        if field not in acquisition_scope:
            missing.add(field)
            reasons.add("ACQUISITION_SCOPE_FIELD_MISSING:" + field)
            continue
        if field in evidence_scope:
            if evidence_scope[field] != acquisition_scope[field]:
                mismatched.add(field)
                reasons.add("SCOPE_MISMATCH:" + field)
        else:
            missing.add(field)
            reasons.add("EVIDENCE_SCOPE_FIELD_MISSING:" + field)

    evidence_time_basis = evidence_scope.get("time_basis_id")
    if (
        evidence_time_basis is not None
        and evidence_time_basis != acquisition_interval["time_basis_id"]
    ) or (
        binding["validity_interval"]["time_basis_id"]
        != acquisition_interval["time_basis_id"]
    ):
        mismatched.add("time_basis_id")
        reasons.add("TIME_BASIS_MISMATCH")

    validity = binding["validity_interval"]
    if (
        validity["start_tai_ns"] > acquisition_interval["start_tai_ns"]
        or acquisition_interval["end_tai_ns"] >= validity["end_tai_ns"]
    ):
        mismatched.add("validity_interval")
        reasons.add("EVIDENCE_VALIDITY_DOES_NOT_COVER_ACQUISITION")

    source_state = binding["source_state"]
    if source_state != "CURRENT":
        reasons.add("SOURCE_NOT_CURRENT:" + source_state)

    if missing and not mismatched and source_state == "CURRENT":
        disposition = "INDETERMINATE"
    elif mismatched:
        disposition = "OUT_OF_SCOPE"
    elif missing or source_state != "CURRENT":
        disposition = "INDETERMINATE"
    else:
        disposition = "SCOPE_COHERENT"

    return {
        "evidence_id": binding["evidence_id"],
        "kind": binding["kind"],
        "disposition": disposition,
        "missing_fields": sorted(missing),
        "mismatched_fields": sorted(mismatched),
        "reason_codes": sorted(reasons),
        "receipt_sha256": binding["receipt_sha256"],
    }


def evaluate_case(case):
    def invalid(reason):
        return {
            "case_id": case["case_id"],
            "disposition": "INVALID_INPUT",
            "checked_binding_count": 0,
            "matched_binding_count": 0,
            "missing_evidence_kinds": [],
            "binding_results": [],
            "reason_codes": [reason],
        }

    if not valid_text(case["case_id"]):
        return invalid("CASE_ID_EMPTY")
    try:
        validate_interval(case["acquisition_interval"])
    except ValueError as error:
        return invalid(str(error))
    try:
        acquisition_scope = claims_to_map(case["acquisition_scope"])
    except ValueError as error:
        return invalid(str(error))

    acquisition_missing = sorted(CANONICAL_SCOPE_FIELDS - set(acquisition_scope))
    evidence_ids = set()
    evidence_kinds = set()
    for binding in case["evidence"]:
        if binding["evidence_id"] in evidence_ids:
            return invalid("EVIDENCE_ID_DUPLICATE")
        evidence_ids.add(binding["evidence_id"])
        if binding["kind"] in evidence_kinds:
            return invalid("EVIDENCE_KIND_DUPLICATE:" + binding["kind"])
        evidence_kinds.add(binding["kind"])

    missing_kinds = sorted(REQUIRED_KINDS - evidence_kinds)
    binding_results = [
        evaluate_binding(binding, case["acquisition_interval"], acquisition_scope)
        for binding in case["evidence"]
    ]
    binding_results.sort(key=lambda item: item["evidence_id"])

    reasons = set()
    reasons.update(
        "ACQUISITION_SCOPE_FIELD_MISSING:" + field
        for field in acquisition_missing
    )
    reasons.update("REQUIRED_EVIDENCE_MISSING:" + kind for kind in missing_kinds)
    for binding in binding_results:
        reasons.update(binding["reason_codes"])

    has_invalid = any(item["disposition"] == "INVALID_INPUT" for item in binding_results)
    has_out_of_scope = any(item["disposition"] == "OUT_OF_SCOPE" for item in binding_results)
    has_indeterminate = any(item["disposition"] == "INDETERMINATE" for item in binding_results)

    if has_invalid:
        disposition = "INVALID_INPUT"
    elif has_out_of_scope:
        disposition = "OUT_OF_SCOPE"
    elif has_indeterminate or acquisition_missing or missing_kinds:
        disposition = "INDETERMINATE"
    else:
        disposition = "SCOPE_COHERENT"

    matched_count = sum(
        item["disposition"] == "SCOPE_COHERENT" for item in binding_results
    )
    return {
        "case_id": case["case_id"],
        "disposition": disposition,
        "checked_binding_count": len(binding_results),
        "matched_binding_count": matched_count,
        "missing_evidence_kinds": missing_kinds,
        "binding_results": binding_results,
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
        result = evaluate_case(case)
        if result["disposition"] == case["expected_disposition"]:
            matches += 1
        results.append(result)
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
