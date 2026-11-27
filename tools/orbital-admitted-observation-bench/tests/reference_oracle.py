#!/usr/bin/env python3
"""Independent standard-library oracle for source-bound acquisition-attempt handoff."""

import hashlib
import json
import sys
from pathlib import Path

CORPUS_SCHEMA = "luminous.orbital-admitted-observation.corpus.v0.1"
RECEIPT_SCHEMA = "luminous.orbital-admitted-observation.receipt.v0.1"
BENCHMARK_ID = "ORBITAL-ADMITTED-OBSERVATION-001"
CLAIM_CEILING = "synthetic_source_bound_attempt_outcome_composition_only"

OWNER_ORDER = [
    "CAPTURE",
    "TIMING",
    "MOTION",
    "CALIBRATION",
    "GEOMETRY",
    "FEATURE",
    "METROLOGY",
    "PHOTOMETRY",
    "DIVERSITY",
]
OWNER_ENUM = set(OWNER_ORDER)

MEASUREMENT_OUTCOMES = {
    "MEASUREMENT_AVAILABLE",
    "NO_INDICATION",
    "INDICATION_REPORTED",
}
FAILURE_OWNER = {
    "OCCLUDED": ("GEOMETRY", "FAIL"),
    "CAPTURE_DROPPED": ("CAPTURE", "FAIL"),
    "TRIGGER_OR_FRAME_UNASSOCIATED": ("TIMING", "FAIL"),
    "SENSOR_SATURATED": ("PHOTOMETRY", "FAIL"),
    "PHOTOMETRY_OUT_OF_PROFILE": ("PHOTOMETRY", "FAIL"),
    "MOTION_BUDGET_EXCEEDED": ("MOTION", "FAIL"),
    "CALIBRATION_INVALID": ("CALIBRATION", "FAIL"),
    "FEATURE_NOT_IDENTIFIABLE": ("FEATURE", "FAIL"),
    "METROLOGY_INDETERMINATE": ("METROLOGY", "UNKNOWN"),
}


def valid_text(value):
    return isinstance(value, str) and bool(value.strip())


def valid_sha256(value):
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdefABCDEF" for character in value)
    )


def owner_sort_key(owner):
    return OWNER_ORDER.index(owner)


def invalid_result(case, reason):
    return {
        "case_id": case["case_id"],
        "attempt_id": case["attempt_id"],
        "disposition": "INVALID_INPUT",
        "sequential_update_permitted": False,
        "public_outcome_kind": case["outcome"]["kind"],
        "missing_channel_ids": list(case["outcome"]["missing_channel_ids"]),
        "owner_results": [],
        "reason_codes": [reason],
        "cutoff_sha256": case["cutoff"]["history_sha256"],
        "scope_receipt_sha256": case["scope_receipt_sha256"],
        "outcome_record_sha256": case["outcome"]["outcome_record_sha256"],
    }


def build_owner_results(case):
    return [
        {
            "owner": binding["owner"],
            "required": binding["required"],
            "state": binding["state"],
            "source_state": binding["source_state"],
            "receipt_sha256": binding["receipt_sha256"],
        }
        for binding in sorted(
            case["owner_bindings"], key=lambda item: owner_sort_key(item["owner"])
        )
    ]


def disposition_result(case, disposition, reason_codes, owners, sequential=False):
    return {
        "case_id": case["case_id"],
        "attempt_id": case["attempt_id"],
        "disposition": disposition,
        "sequential_update_permitted": sequential,
        "public_outcome_kind": case["outcome"]["kind"],
        "missing_channel_ids": sorted(case["outcome"]["missing_channel_ids"]),
        "owner_results": owners,
        "reason_codes": sorted(reason_codes),
        "cutoff_sha256": case["cutoff"]["history_sha256"],
        "scope_receipt_sha256": case["scope_receipt_sha256"],
        "outcome_record_sha256": case["outcome"]["outcome_record_sha256"],
    }


def validate_case_shape(case):
    if (
        not valid_text(case["case_id"])
        or not valid_text(case["campaign_id"])
        or not valid_text(case["attempt_id"])
        or case["sequence_no"] == 0
        or case["campaign_attempt_count"] == 0
        or case["sequence_no"] > case["campaign_attempt_count"]
    ):
        return "ATTEMPT_IDENTITY_INVALID"

    cutoff = case["cutoff"]
    if (
        not valid_text(cutoff["cutoff_id"])
        or not valid_sha256(cutoff["history_sha256"])
        or cutoff["sequence_watermark"] != case["sequence_no"]
        or cutoff["history_max_sequence"] >= case["sequence_no"]
    ):
        return "CUTOFF_OR_NO_LOOKAHEAD_INVALID"
    if not valid_sha256(case["scope_receipt_sha256"]):
        return "SCOPE_RECEIPT_DIGEST_INVALID"

    action = case["action"]
    if (
        not valid_text(action["action_id"])
        or not valid_text(action["policy_revision"])
        or not valid_sha256(action["command_receipt_sha256"])
    ):
        return "ACTION_IDENTITY_INVALID"

    kernel = case["kernel"]
    if (
        not valid_text(kernel["profile_id"])
        or not valid_text(kernel["profile_revision"])
        or not valid_sha256(kernel["receipt_sha256"])
        or not kernel["allowed_outcomes"]
        or not kernel["required_owners"]
    ):
        return "KERNEL_PROFILE_INVALID"

    allowed = set()
    for kind in kernel["allowed_outcomes"]:
        if not valid_text(kind) or kind in allowed:
            return "KERNEL_OUTCOME_DUPLICATE_OR_EMPTY"
        allowed.add(kind)

    required_owners = set()
    for owner in kernel["required_owners"]:
        if owner not in OWNER_ENUM or owner in required_owners:
            return "KERNEL_REQUIRED_OWNER_DUPLICATE"
        required_owners.add(owner)

    outcome = case["outcome"]
    if not valid_text(outcome["kind"]) or not valid_sha256(
        outcome["outcome_record_sha256"]
    ):
        return "OUTCOME_RECORD_INVALID"
    if outcome["payload_sha256"] is not None and not valid_sha256(
        outcome["payload_sha256"]
    ):
        return "OUTCOME_PAYLOAD_DIGEST_INVALID"

    channel_ids = set()
    for channel in outcome["channel_ids"]:
        if not valid_text(channel) or channel in channel_ids:
            return "OBSERVED_CHANNEL_DUPLICATE_OR_EMPTY"
        channel_ids.add(channel)

    missing_ids = set()
    for channel in outcome["missing_channel_ids"]:
        if not valid_text(channel) or channel in missing_ids:
            return "MISSING_CHANNEL_DUPLICATE_OR_EMPTY"
        if channel in channel_ids:
            return "CHANNEL_BOTH_OBSERVED_AND_MISSING"
        missing_ids.add(channel)
    if not channel_ids and not missing_ids:
        return "OUTCOME_HAS_NO_CHANNELS"

    if action["realization"] == "VERIFIED":
        if action["realized_action_id"] != action["action_id"]:
            return "REALIZED_ACTION_ID_MISMATCH"
    elif action["realized_action_id"] is not None:
        return "UNVERIFIED_ACTION_HAS_REALIZED_ID"

    owner_bindings = {}
    for binding in case["owner_bindings"]:
        owner = binding["owner"]
        if owner not in OWNER_ENUM or owner in owner_bindings:
            return "OWNER_BINDING_DUPLICATE"
        if not valid_sha256(binding["receipt_sha256"]):
            return "OWNER_RECEIPT_DIGEST_INVALID"
        profile_requires_owner = owner in required_owners
        if binding["required"] != profile_requires_owner:
            return "OWNER_REQUIRED_FLAG_DOES_NOT_MATCH_KERNEL_PROFILE"
        if profile_requires_owner and binding["state"] == "NOT_REQUIRED":
            return "REQUIRED_OWNER_MARKED_NOT_REQUIRED"
        if not profile_requires_owner and binding["state"] != "NOT_REQUIRED":
            return "OPTIONAL_OWNER_HAS_GATE_STATE"
        owner_bindings[owner] = binding

    for owner in required_owners:
        if owner not in owner_bindings:
            return "OWNER_BINDING_MISSING:" + owner

    return None


def evaluate_case(case):
    invalid = validate_case_shape(case)
    if invalid:
        return invalid_result(case, invalid)

    owners = build_owner_results(case)
    owner_bindings = {binding["owner"]: binding for binding in case["owner_bindings"]}
    outcome = case["outcome"]
    kernel = case["kernel"]

    if kernel["source_state"] != "CURRENT":
        return disposition_result(
            case,
            "ABSTAIN_UNMODELLED_OUTCOME",
            ["KERNEL_PROFILE_SOURCE_NOT_CURRENT"],
            owners,
        )

    if case["scope_disposition"] == "INVALID_INPUT":
        return invalid_result(case, "SCOPE_RECEIPT_INVALID")
    if case["scope_disposition"] == "OUT_OF_SCOPE":
        return disposition_result(
            case,
            "INADMISSIBLE_MEASUREMENT",
            ["ACQUISITION_SCOPE_OUT_OF_SCOPE"],
            owners,
        )
    if case["scope_disposition"] == "INDETERMINATE":
        return disposition_result(
            case,
            "ABSTAIN_UNMODELLED_OUTCOME",
            ["ACQUISITION_SCOPE_INDETERMINATE"],
            owners,
        )

    for owner in OWNER_ORDER:
        binding = owner_bindings.get(owner)
        if binding and binding["source_state"] != "CURRENT":
            return disposition_result(
                case,
                "ABSTAIN_UNMODELLED_OUTCOME",
                ["OWNER_SOURCE_NOT_CURRENT:" + owner],
                owners,
            )

    if outcome["kind"] not in kernel["allowed_outcomes"]:
        return disposition_result(
            case,
            "ABSTAIN_UNMODELLED_OUTCOME",
            ["OUTCOME_NOT_IN_FROZEN_KERNEL_SUPPORT"],
            owners,
        )

    if outcome["kind"] in MEASUREMENT_OUTCOMES:
        if (
            outcome["payload_sha256"] is None
            or not outcome["channel_ids"]
            or outcome["missing_channel_ids"]
        ):
            return invalid_result(case, "MEASUREMENT_OUTCOME_PAYLOAD_OR_CHANNELS_INVALID")
        if (
            case["action"]["realization"] != "VERIFIED"
            and kernel["action_realization_required"]
        ):
            return disposition_result(
                case,
                "INADMISSIBLE_MEASUREMENT",
                ["ACTION_REALIZATION_NOT_VERIFIED"],
                owners,
            )

        for owner in OWNER_ORDER:
            binding = owner_bindings[owner]
            if binding["required"] and binding["state"] == "FAIL":
                return disposition_result(
                    case,
                    "INADMISSIBLE_MEASUREMENT",
                    ["REQUIRED_OWNER_GATE_FAILED:" + owner],
                    owners,
                )
        for owner in OWNER_ORDER:
            binding = owner_bindings[owner]
            if binding["required"] and binding["state"] == "UNKNOWN":
                return disposition_result(
                    case,
                    "ABSTAIN_UNMODELLED_OUTCOME",
                    ["REQUIRED_OWNER_GATE_UNKNOWN:" + owner],
                    owners,
                )
        return disposition_result(case, "MEASUREMENT_ADMISSIBLE", [], owners, True)

    if outcome["kind"] == "SCOPE_MISMATCH":
        return disposition_result(
            case, "INADMISSIBLE_MEASUREMENT", ["OUTCOME_SCOPE_MISMATCH"], owners
        )

    if not kernel["missingness_semantics_declared"]:
        return disposition_result(
            case, "ABSTAIN_UNMODELLED_OUTCOME", ["KERNEL_MISSINGNESS_SEMANTICS_NOT_DECLARED"], owners
        )

    if outcome["kind"] == "ACTION_NOT_REALIZED":
        if case["action"]["realization"] != "NOT_REALIZED":
            return invalid_result(case, "ACTION_NOT_REALIZED_OUTCOME_CONTRADICTS_READBACK")
    elif case["action"]["realization"] != "VERIFIED" and kernel[
        "action_realization_required"
    ]:
        return disposition_result(
            case, "ABSTAIN_UNMODELLED_OUTCOME", ["ACTION_REALIZATION_UNRESOLVED"], owners
        )

    if outcome["kind"] in FAILURE_OWNER:
        owner, expected_state = FAILURE_OWNER[outcome["kind"]]
        binding = owner_bindings.get(owner)
        if binding is None:
            return invalid_result(case, "FAILURE_OWNER_BINDING_MISSING")
        if binding["state"] == "UNKNOWN" or binding["source_state"] != "CURRENT":
            return disposition_result(
                case, "ABSTAIN_UNMODELLED_OUTCOME", ["FAILURE_OWNER_STATE_UNRESOLVED"], owners
            )
        if binding["state"] != expected_state:
            return invalid_result(case, "OUTCOME_KIND_CONTRADICTS_OWNER_GATE")
    elif outcome["kind"] not in {"ACTION_NOT_REALIZED", "OTHER_DECLARED_FAILURE"}:
        return disposition_result(
            case, "ABSTAIN_UNMODELLED_OUTCOME", ["OUTCOME_KIND_UNRECOGNIZED"], owners
        )
    elif outcome["kind"] == "OTHER_DECLARED_FAILURE":
        if not any(
            binding["required"] and binding["state"] == "FAIL"
            for binding in owner_bindings.values()
        ):
            return invalid_result(case, "OTHER_FAILURE_HAS_NO_FAILED_OWNER_GATE")

    return disposition_result(
        case, "MODELLED_OBSERVATION_PROCESS_OUTCOME", [], owners, True
    )


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
    attempt_ids = set()
    campaign_counts = {}
    campaign_sequences = {}
    results = []
    matches = 0
    for case in corpus["cases"]:
        if case["case_id"] in case_ids:
            raise ValueError("CASE_ID_DUPLICATE")
        case_ids.add(case["case_id"])
        attempt_key = (case["campaign_id"], case["attempt_id"])
        if attempt_key in attempt_ids:
            raise ValueError("ATTEMPT_ID_DUPLICATE_WITHIN_CAMPAIGN")
        attempt_ids.add(attempt_key)

        campaign = case["campaign_id"]
        expected_count = case["campaign_attempt_count"]
        if campaign in campaign_counts and campaign_counts[campaign] != expected_count:
            raise ValueError("CAMPAIGN_ATTEMPT_COUNT_CONFLICT")
        campaign_counts[campaign] = expected_count

        sequences = campaign_sequences.setdefault(campaign, set())
        if case["sequence_no"] in sequences:
            raise ValueError("CAMPAIGN_SEQUENCE_DUPLICATE")
        sequences.add(case["sequence_no"])

        result = evaluate_case(case)
        if result["disposition"] == case["expected_disposition"]:
            matches += 1
        results.append(result)

    for campaign, expected_count in campaign_counts.items():
        sequences = campaign_sequences.get(campaign, set())
        if len(sequences) != expected_count or sequences != set(range(1, expected_count + 1)):
            raise ValueError("CAMPAIGN_ATTEMPT_SEQUENCE_GAP")

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
        "authority_granted": False,
        "statistical_update_performed": False,
        "likelihood_or_eprocess_generated": False,
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
