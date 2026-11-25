#!/usr/bin/env python3
"""Independent standard-library oracle for non-compensatory two-key admission."""

import hashlib
import json
import sys
from pathlib import Path

CORPUS_SCHEMA = "luminous.orbital-inspection-two-key-admission.corpus.v0.1"
RECEIPT_SCHEMA = "luminous.orbital-inspection-two-key-admission.receipt.v0.1"
BENCHMARK_ID = "ORBITAL-INSPECTION-TWO-KEY-ADMISSION-001"
CLAIM_CEILING = "synthetic_non_compensatory_action_admission_composition_only"
MAX_COST = 1_000_000_000_000


def valid_text(value):
    return isinstance(value, str) and bool(value.strip())


def valid_optional_text(value):
    return value is None or valid_text(value)


def valid_sha256(value):
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdefABCDEF" for character in value)
    )


def make_result(case, disposition, reason):
    candidate = case["candidate"]
    safety = candidate["physical_safety"]
    portfolio = candidate["post_action_portfolio"]
    measurement = candidate["measurement"]
    return {
        "case_id": case["case_id"],
        "disposition": disposition,
        "reason_codes": [reason] if reason else [],
        "delta_v_milli_m_s": candidate["delta_v_milli_m_s"],
        "duration_ms": candidate["duration_ms"],
        "compute_us": candidate["compute_us"],
        "decision_value_milli": candidate["decision_value_milli"],
        "physical_safety_receipt_sha256": safety["receipt_sha256"] if safety else None,
        "portfolio_receipt_sha256": portfolio["receipt_sha256"] if portfolio else None,
        "measurement_receipt_sha256": measurement["measurement_receipt_sha256"] if measurement else None,
        "scope_receipt_sha256": measurement["scope_receipt_sha256"] if measurement else None,
    }


def evaluate_case(case):
    candidate = case["candidate"]
    scope = candidate["scope"]
    safety = candidate["physical_safety"]
    portfolio = candidate["post_action_portfolio"]
    measurement = candidate["measurement"]

    def result(disposition, reason=""):
        return make_result(case, disposition, reason)

    required_scope = [
        "action_id",
        "vehicle_id",
        "trajectory_revision",
        "horizon_id",
    ]
    if (
        not valid_text(case["case_id"])
        or any(not valid_text(scope[name]) for name in required_scope)
        or any(
            not valid_optional_text(scope[name])
            for name in ["acquisition_id", "camera_id", "asset_id", "feature_id"]
        )
        or any(
            candidate[name] > MAX_COST
            for name in [
                "delta_v_milli_m_s",
                "duration_ms",
                "compute_us",
                "decision_value_milli",
            ]
        )
    ):
        return result("INVALID_INPUT", "CASE_SCOPE_OR_COST_INVALID")

    if candidate["class"] == "RECOVERY" and (
        portfolio is not None
        or measurement is not None
        or any(
            scope[name] is not None
            for name in ["acquisition_id", "camera_id", "asset_id", "feature_id"]
        )
    ):
        return result("INVALID_INPUT", "RECOVERY_ACTION_MUST_NOT_CLAIM_INSPECTION")

    if safety is None:
        return result("INDETERMINATE", "PHYSICAL_SAFETY_EVIDENCE_MISSING")
    if (
        not valid_text(safety["action_id"])
        or not valid_text(safety["vehicle_id"])
        or not valid_text(safety["trajectory_revision"])
        or not valid_text(safety["horizon_id"])
        or not valid_sha256(safety["receipt_sha256"])
    ):
        return result("INVALID_INPUT", "PHYSICAL_SAFETY_BINDING_INVALID")
    if any(
        safety[name] != scope[name]
        for name in ["action_id", "vehicle_id", "trajectory_revision", "horizon_id"]
    ):
        return result("OUT_OF_SCOPE", "PHYSICAL_SAFETY_SCOPE_MISMATCH")

    if portfolio is not None:
        if (
            not valid_text(portfolio["action_id"])
            or not valid_text(portfolio["horizon_id"])
            or not valid_text(portfolio["event_set_revision"])
            or not valid_sha256(portfolio["receipt_sha256"])
        ):
            return result("INVALID_INPUT", "PORTFOLIO_BINDING_INVALID")
        if (
            portfolio["action_id"] != scope["action_id"]
            or portfolio["horizon_id"] != scope["horizon_id"]
        ):
            return result("OUT_OF_SCOPE", "POST_ACTION_PORTFOLIO_SCOPE_MISMATCH")

    if measurement is not None:
        if (
            not all(
                valid_text(measurement[name])
                for name in [
                    "action_id",
                    "acquisition_id",
                    "camera_id",
                    "asset_id",
                    "feature_id",
                ]
            )
            or not valid_sha256(measurement["measurement_receipt_sha256"])
            or not valid_sha256(measurement["scope_receipt_sha256"])
        ):
            return result("INVALID_INPUT", "MEASUREMENT_BINDING_INVALID")
        for name in ["acquisition_id", "camera_id", "asset_id", "feature_id"]:
            if scope[name] is None:
                return result("INDETERMINATE", name.upper() + "_SCOPE_MISSING")
        if any(
            measurement[left] != scope[right]
            for left, right in [
                ("action_id", "action_id"),
                ("acquisition_id", "acquisition_id"),
                ("camera_id", "camera_id"),
                ("asset_id", "asset_id"),
                ("feature_id", "feature_id"),
            ]
        ):
            return result("OUT_OF_SCOPE", "MEASUREMENT_SCOPE_MISMATCH")

    if safety["state"] == "FAIL":
        return result("REJECTED_SAFETY", "PHYSICAL_SAFETY_FAIL")
    if safety["state"] == "UNKNOWN":
        return result("INDETERMINATE", "PHYSICAL_SAFETY_UNKNOWN")
    if safety["state"] == "STALE":
        return result("INDETERMINATE", "PHYSICAL_SAFETY_STALE")

    if candidate["class"] == "RECOVERY":
        if portfolio is not None or measurement is not None:
            return result("INVALID_INPUT", "RECOVERY_ACTION_MUST_NOT_CLAIM_INSPECTION")
        if any(
            scope[name] is not None
            for name in ["acquisition_id", "camera_id", "asset_id", "feature_id"]
        ):
            return result("INVALID_INPUT", "RECOVERY_SCOPE_MUST_NOT_CLAIM_MEASUREMENT")
        return result("ELIGIBLE_RECOVERY")

    if portfolio is None:
        return result("INDETERMINATE", "POST_ACTION_PORTFOLIO_MISSING")
    if not portfolio["event_set_complete"]:
        return result("INDETERMINATE", "POST_ACTION_EVENT_SET_INCOMPLETE")
    if portfolio["state"] == "FAIL":
        return result("REJECTED_PORTFOLIO", "POST_ACTION_PORTFOLIO_FAIL")
    if portfolio["state"] == "UNKNOWN":
        return result("INDETERMINATE", "POST_ACTION_PORTFOLIO_UNKNOWN")
    if portfolio["state"] == "STALE":
        return result("INDETERMINATE", "POST_ACTION_PORTFOLIO_STALE")

    if measurement is None:
        return result("INDETERMINATE", "MEASUREMENT_EVIDENCE_MISSING")
    if measurement["acquisition_state"] == "FAIL":
        return result("REJECTED_EVIDENCE", "MEASUREMENT_ACQUISITION_FAIL")
    if measurement["acquisition_state"] == "UNKNOWN":
        return result("INDETERMINATE", "MEASUREMENT_ACQUISITION_UNKNOWN")
    if measurement["acquisition_state"] == "STALE":
        return result("INDETERMINATE", "MEASUREMENT_ACQUISITION_STALE")
    if measurement["scope_coherence_state"] == "FAIL":
        return result("REJECTED_EVIDENCE", "MEASUREMENT_SCOPE_COHERENCE_FAIL")
    if measurement["scope_coherence_state"] == "UNKNOWN":
        return result("INDETERMINATE", "MEASUREMENT_SCOPE_COHERENCE_UNKNOWN")
    if measurement["scope_coherence_state"] == "STALE":
        return result("INDETERMINATE", "MEASUREMENT_SCOPE_COHERENCE_STALE")
    return result("ELIGIBLE_INSPECTION")


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
    ids = set()
    results = []
    matches = 0
    for case in corpus["cases"]:
        if case["case_id"] in ids:
            raise ValueError("CASE_ID_DUPLICATE")
        ids.add(case["case_id"])
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
        raise SystemExit("usage: reference_oracle.py <corpus.json> <40-hex-source-commit>")
    raw = Path(sys.argv[1]).read_bytes()
    receipt = audit_corpus(raw, sys.argv[2])
    print(json.dumps(receipt, indent=2))
    if receipt["expected_disposition_mismatches"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
