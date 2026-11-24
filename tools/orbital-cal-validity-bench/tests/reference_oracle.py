#!/usr/bin/env python3
"""Independent standard-library oracle for orbital calibration-validity v0.1."""

import hashlib
import json
import sys
from pathlib import Path

CORPUS_SCHEMA = "luminous.orbital-cal-validity.corpus.v0.1"
RECEIPT_SCHEMA = "luminous.orbital-cal-validity.receipt.v0.1"
BENCHMARK_ID = "ORBITAL-CAL-VALIDITY-001"
CLAIM_CEILING = "synthetic_calibration_scope_and_acquisition_state_validity_only"

PRIORITY = {
    "INVALID_INPUT": 0,
    "IDENTITY_MISMATCH": 1,
    "REVOKED": 2,
    "DRIFT_SUSPECTED": 3,
    "INVALIDATED": 4,
    "OUT_OF_SCOPE": 5,
    "EXPIRED": 6,
    "INDETERMINATE": 7,
    "VALID_WITHIN_SCOPE": 8,
}


def require_text(value):
    return isinstance(value, str) and bool(value.strip())


def valid_sha256(value):
    return (isinstance(value, str) and len(value) == 64
            and all(char in "0123456789abcdefABCDEF" for char in value))


def interval(center, uncertainty):
    if not isinstance(center, int) or isinstance(center, bool):
        raise ValueError("TIME_INTERVAL_INVALID")
    if not isinstance(uncertainty, int) or isinstance(uncertainty, bool) or uncertainty < 0:
        raise ValueError("TIME_INTERVAL_INVALID")
    return [center - uncertainty, center + uncertainty]


def choose_status(statuses):
    return min(statuses, key=lambda item: PRIORITY[item]) if statuses else "VALID_WITHIN_SCOPE"


def validate_case(case):
    if not require_text(case["case_id"]) or not case["required_scope_ids"] or not case["scopes"]:
        raise ValueError("CASE_ID_OR_REQUIRED_SCOPE_INVALID")
    acq = interval(case["acquisition_time_ns"], case["acquisition_uncertainty_ns"])

    component_ids = set()
    for component in case["components"]:
        cid = component["component_id"]
        if not require_text(cid) or cid in component_ids:
            raise ValueError("COMPONENT_ID_DUPLICATE_OR_EMPTY")
        component_ids.add(cid)
        if any(not require_text(key) or not require_text(value)
               for key, value in component["configuration"].items()):
            raise ValueError("COMPONENT_CONFIGURATION_INVALID")

    scope_ids = set()
    for scope in case["scopes"]:
        sid = scope["scope_id"]
        if (not require_text(sid) or not require_text(scope["calibration_id"])
                or not require_text(scope["calibration_revision"])
                or not valid_sha256(scope["certificate_sha256"])
                or not require_text(scope["component_id"])
                or scope["calibration_epoch_ns"] > scope["valid_from_ns"]
                or scope["valid_until_ns"] <= scope["valid_from_ns"]
                or sid in scope_ids):
            raise ValueError("CALIBRATION_SCOPE_INVALID")
        scope_ids.add(sid)
        if any(not require_text(key) or not require_text(value)
               for key, value in scope["required_configuration"].items()):
            raise ValueError("REQUIRED_CONFIGURATION_INVALID")
        if scope["check_required"]:
            if scope["check_state"] == "NOT_REQUIRED":
                raise ValueError("REQUIRED_CHECK_MARKED_NOT_REQUIRED")
            if scope["max_check_age_ns"] is None:
                raise ValueError("REQUIRED_CHECK_MAX_AGE_MISSING")
            if not valid_sha256(scope["check_receipt_sha256"]):
                raise ValueError("REQUIRED_CHECK_RECEIPT_DIGEST_INVALID")

    required = set()
    for sid in case["required_scope_ids"]:
        if not require_text(sid) or sid in required or sid not in scope_ids:
            raise ValueError("REQUIRED_SCOPE_REFERENCE_INVALID")
        required.add(sid)

    event_ids = set()
    for event in case["events"]:
        interval(event["effective_time_ns"], event["time_uncertainty_ns"])
        if (not require_text(event["event_id"]) or not valid_sha256(event["event_receipt_sha256"])
                or event["event_id"] in event_ids or not event["affected_scope_ids"]):
            raise ValueError("CHANGE_EVENT_INVALID")
        event_ids.add(event["event_id"])
        affected = set()
        for sid in event["affected_scope_ids"]:
            if sid in affected or sid not in scope_ids:
                raise ValueError("CHANGE_EVENT_SCOPE_REFERENCE_INVALID")
            affected.add(sid)
    return acq


def evaluate_scope(case, scope, acquisition):
    statuses = []
    reasons = set()

    def add(status, reason):
        statuses.append(status)
        reasons.add(reason)

    if scope["certificate_state"] == "REVOKED":
        add("REVOKED", "CERTIFICATE_REVOKED")

    start = scope["valid_from_ns"]
    end = scope["valid_until_ns"]
    if acquisition[1] < start:
        add("EXPIRED", "CALIBRATION_NOT_YET_VALID")
    elif acquisition[0] >= end:
        add("EXPIRED", "CALIBRATION_EXPIRED")
    elif acquisition[0] < start or acquisition[1] >= end:
        add("INDETERMINATE", "CALIBRATION_TIME_BOUNDARY_OVERLAP")

    component = next((item for item in case["components"]
                      if item["component_id"] == scope["component_id"]), None)
    if component is None:
        add("INDETERMINATE", "COMPONENT_STATE_MISSING")
    else:
        for key, required_value in scope["required_configuration"].items():
            if key not in component["configuration"]:
                add("INDETERMINATE", "CONFIGURATION_FIELD_MISSING")
            elif component["configuration"][key] != required_value:
                add("IDENTITY_MISMATCH", "CONFIGURATION_MISMATCH")

    if scope["check_required"]:
        state = scope["check_state"]
        if state == "HOLD":
            add("DRIFT_SUSPECTED", "REFERENCE_CHECK_HOLD")
        elif state in ("UNKNOWN", "NOT_REQUIRED"):
            add("INDETERMINATE", "REFERENCE_CHECK_STATE_UNKNOWN")
        elif state == "PASS":
            check_time = scope["last_check_time_ns"]
            max_age = scope["max_check_age_ns"]
            if check_time is None or max_age is None:
                add("INDETERMINATE", "REFERENCE_CHECK_EVIDENCE_MISSING")
            else:
                if check_time > acquisition[1]:
                    add("INDETERMINATE", "REFERENCE_CHECK_AFTER_ACQUISITION")
                elif acquisition[0] - check_time > max_age:
                    add("EXPIRED", "REFERENCE_CHECK_STALE")
                elif acquisition[1] - check_time > max_age:
                    add("INDETERMINATE", "REFERENCE_CHECK_AGE_BOUNDARY_OVERLAP")

    for event in case["events"]:
        if scope["scope_id"] not in event["affected_scope_ids"]:
            continue
        if event["impact"] == "NO_CALIBRATION_IMPACT":
            continue
        event_time = interval(event["effective_time_ns"], event["time_uncertainty_ns"])
        if event_time[1] <= scope["calibration_epoch_ns"]:
            continue
        if event_time[0] <= scope["calibration_epoch_ns"]:
            add("INDETERMINATE", "EVENT_TIME_OVERLAPS_CALIBRATION_EPOCH")
        elif event_time[1] <= acquisition[0]:
            if event["impact"] == "INVALIDATES":
                add("INVALIDATED", "CALIBRATION_INVALIDATED_BY_EVENT")
            elif event["impact"] == "UNKNOWN_IMPACT":
                add("INDETERMINATE", "EVENT_IMPACT_UNKNOWN")
        elif event_time[0] <= acquisition[1]:
            add("INDETERMINATE", "EVENT_TIME_OVERLAPS_ACQUISITION")

    return {
        "scope_id": scope["scope_id"],
        "calibration_id": scope["calibration_id"],
        "calibration_revision": scope["calibration_revision"],
        "certificate_sha256": scope["certificate_sha256"],
        "check_receipt_sha256": scope["check_receipt_sha256"],
        "calibration_epoch_ns": scope["calibration_epoch_ns"],
        "status": choose_status(statuses),
        "reason_codes": sorted(reasons),
    }


def evaluate_case(case):
    acquisition = validate_case(case)
    scopes_by_id = {scope["scope_id"]: scope for scope in case["scopes"]}
    scope_results = []
    statuses = []
    reasons = set()

    for sid in case["required_scope_ids"]:
        result = evaluate_scope(case, scopes_by_id[sid], acquisition)
        scope_results.append(result)
        statuses.append(result["status"])
        reasons.update(result["reason_codes"])

    if not case["event_inventory_complete"]:
        statuses.append("INDETERMINATE")
        reasons.add("EVENT_INVENTORY_INCOMPLETE")

    status = choose_status(statuses)
    return {
        "case_id": case["case_id"],
        "acquisition_interval_ns": acquisition,
        "status": status,
        "reason_codes": sorted(reasons),
        "event_inventory_complete": case["event_inventory_complete"],
        "scope_results": scope_results,
        "may_enter_geometry_precondition": status == "VALID_WITHIN_SCOPE",
    }


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: reference_oracle.py <fixture.json> <40-hex-source-commit>")
    raw = Path(sys.argv[1]).read_bytes()
    source_commit = sys.argv[2]
    if (len(source_commit) != 40
            or any(char not in "0123456789abcdefABCDEF" for char in source_commit)):
        raise SystemExit("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS")
    corpus = json.loads(raw)
    if (corpus.get("schema") != CORPUS_SCHEMA
            or corpus.get("benchmark_id") != BENCHMARK_ID
            or corpus.get("evidence_class") != "synthetic"
            or corpus.get("claim_ceiling") != CLAIM_CEILING
            or not 10 <= len(corpus.get("cases", [])) <= 256):
        raise SystemExit("CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID")

    case_ids = [case["case_id"] for case in corpus["cases"]]
    if len(case_ids) != len(set(case_ids)):
        raise SystemExit("CASE_ID_DUPLICATE")
    results = [evaluate_case(case) for case in corpus["cases"]]
    matches = sum(result["status"] == case["expected_status"]
                  for result, case in zip(results, corpus["cases"]))
    receipt = {
        "schema": RECEIPT_SCHEMA,
        "benchmark_id": BENCHMARK_ID,
        "source_commit": source_commit.lower(),
        "input_sha256": hashlib.sha256(raw).hexdigest(),
        "evidence_class": "synthetic",
        "claim_ceiling": CLAIM_CEILING,
        "case_count": len(results),
        "expected_outcome_matches": matches,
        "expected_outcome_mismatches": len(results) - matches,
        "authority_granted": False,
        "visibility_assessed": False,
        "measurement_performance_assessed": False,
        "physical_validation": "NOT_ASSESSED",
        "cases": results,
    }
    print(json.dumps(receipt, indent=2) + "\n")


if __name__ == "__main__":
    main()
