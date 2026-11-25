#!/usr/bin/env python3
"""Independent standard-library oracle for the fixed-target binomial screen."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
import struct
from pathlib import Path

CLAIM_CEILING = "fixed_target_binomial_screening_only"
OUTPUT_SCHEMA = "luminous.aero.nde-fixed-target-receipt.v0.1"


def log_choose(n: int, k: int) -> float:
    k = min(k, n - k)
    return sum(math.log((n + 1 - i) / i) for i in range(1, k + 1))


def log_binomial_probability(n: int, k: int, p: float) -> float:
    if p <= 0.0:
        return 0.0 if k == 0 else -math.inf
    if p >= 1.0:
        return 0.0 if k == n else -math.inf
    return log_choose(n, k) + k * math.log(p) + (n - k) * math.log1p(-p)


def log_sum_exp(values: list[float]) -> float:
    maximum = max(values)
    if not math.isfinite(maximum):
        return maximum
    return maximum + math.log(sum(math.exp(value - maximum) for value in values))


def upper_tail(n: int, x: int, p: float) -> float:
    return min(1.0, max(0.0, math.exp(log_sum_exp(
        [log_binomial_probability(n, k, p) for k in range(x, n + 1)]
    ))))


def lower_tail(n: int, x: int, p: float) -> float:
    return min(1.0, max(0.0, math.exp(log_sum_exp(
        [log_binomial_probability(n, k, p) for k in range(0, x + 1)]
    ))))


def detection_lower(n: int, hits: int, confidence: float) -> float:
    if hits == 0:
        return 0.0
    alpha = 1.0 - confidence
    if hits == n:
        return alpha ** (1.0 / n)
    low, high = 0.0, 1.0
    for _ in range(100):
        mid = (low + high) / 2.0
        if upper_tail(n, hits, mid) < alpha:
            low = mid
        else:
            high = mid
    return (low + high) / 2.0


def false_call_upper(n: int, calls: int, confidence: float) -> float:
    if calls == n:
        return 1.0
    alpha = 1.0 - confidence
    if calls == 0:
        return 1.0 - alpha ** (1.0 / n)
    low, high = 0.0, 1.0
    for _ in range(100):
        mid = (low + high) / 2.0
        if lower_tail(n, calls, mid) > alpha:
            low = mid
        else:
            high = mid
    return (low + high) / 2.0


def apply_mutation(study: dict, mutation: dict) -> None:
    operation = mutation["operation"]
    inspector_id = mutation.get("inspector_id")
    inspectors = study["inspectors"]
    if operation == "remove_inspector":
        before = len(inspectors)
        study["inspectors"] = [item for item in inspectors if item["inspector_id"] != inspector_id]
        if len(study["inspectors"]) == before:
            raise ValueError(f"missing inspector {inspector_id}")
        return

    if operation == "set_scope_sha256":
        study["scope_sha256"] = mutation["value"]
        return

    inspector = next((item for item in inspectors if item["inspector_id"] == inspector_id), None)
    if inspector is None:
        raise ValueError(f"missing inspector {inspector_id}")
    if operation == "set_calibration_valid":
        inspector["calibration_valid"] = mutation["value"]
        return
    if operation == "truncate_blank_trials":
        length = mutation["value"]
        if not isinstance(length, int) or length < 0 or length > len(inspector["blank_trials"]):
            raise ValueError("invalid blank-trial truncation")
        inspector["blank_trials"] = inspector["blank_trials"][:length]
        return

    index = mutation["trial_index"]
    if operation == "set_flaw_detected":
        inspector["flaw_trials"][index]["detected"] = mutation["value"]
    elif operation == "set_blank_false_call":
        inspector["blank_trials"][index]["false_call"] = mutation["value"]
    elif operation == "set_flaw_size_mm":
        inspector["flaw_trials"][index]["flaw_size_mm"] = mutation["value"]
    elif operation == "set_flaw_specimen_id":
        inspector["flaw_trials"][index]["specimen_id"] = mutation["value"]
    else:
        raise ValueError(f"unsupported mutation {operation}")


def evaluate(study: dict, scope: dict, scope_sha256: str, criteria: dict) -> tuple[str, list[dict]]:
    if not study["study_id"].strip() or study["evidence_class"] != "synthetic":
        return "NOT_EVALUABLE_EVIDENCE_CLASS", []
    if study["scope_ref"] != scope["scope_ref"]:
        return "NOT_EVALUABLE_SCOPE_MISMATCH", []
    if study["scope_sha256"] != scope_sha256:
        return "NOT_EVALUABLE_SCOPE_DIGEST_MISMATCH", []
    if len(study["inspectors"]) < criteria["minimum_inspectors"]:
        return "INSUFFICIENT_INSPECTOR_COUNT", []

    inspector_ids: set[str] = set()
    specimen_ids: set[str] = set()
    for inspector in study["inspectors"]:
        identity = (
            inspector["inspector_id"],
            inspector["equipment_serial"],
            inspector["calibration_id"],
        )
        if any(not part.strip() for part in identity) or inspector["inspector_id"] in inspector_ids:
            return "NOT_EVALUABLE_DUPLICATE_OR_MISSING_IDENTITY", []
        inspector_ids.add(inspector["inspector_id"])
        if not inspector["calibration_valid"]:
            return "NOT_EVALUABLE_INVALID_CALIBRATION", []
        for trial in inspector["flaw_trials"]:
            if not trial["specimen_id"].strip() or trial["specimen_id"] in specimen_ids:
                return "NOT_EVALUABLE_DUPLICATE_SPECIMEN", []
            specimen_ids.add(trial["specimen_id"])
            if trial["scope_ref"] != scope["scope_ref"] or not math.isfinite(trial["flaw_size_mm"]) or abs(
                trial["flaw_size_mm"] - scope["target_flaw_size_mm"]
            ) > 1.0e-9:
                return "NOT_EVALUABLE_SCOPE_MISMATCH", []
        for trial in inspector["blank_trials"]:
            if not trial["specimen_id"].strip() or trial["specimen_id"] in specimen_ids:
                return "NOT_EVALUABLE_DUPLICATE_SPECIMEN", []
            specimen_ids.add(trial["specimen_id"])
            if trial["scope_ref"] != scope["scope_ref"]:
                return "NOT_EVALUABLE_SCOPE_MISMATCH", []
        if (
            len(inspector["flaw_trials"]) < criteria["minimum_flaw_trials_per_inspector"]
            or len(inspector["blank_trials"]) < criteria["minimum_blank_trials_per_inspector"]
        ):
            return "SAMPLE_COUNT_INSUFFICIENT", []

    detail = []
    for inspector in study["inspectors"]:
        flaw = inspector["flaw_trials"]
        hits = sum(1 for trial in flaw if trial["detected"])
        blanks = inspector["blank_trials"]
        calls = sum(1 for trial in blanks if trial["false_call"])
        low = detection_lower(len(flaw), hits, criteria["detection_one_sided_confidence"])
        high = false_call_upper(len(blanks), calls, criteria["false_call_one_sided_confidence"])
        detection_pass = low + 1.0e-12 >= criteria["target_detection_probability"]
        false_call_pass = high <= criteria["maximum_false_call_probability"] + 1.0e-12
        detail.append({
            "inspector_id": inspector["inspector_id"],
            "flaw_trials": len(flaw),
            "detections": hits,
            "detection_lower_bound": low,
            "blank_trials": len(blanks),
            "false_calls": calls,
            "false_call_upper_bound": high,
            "detection_bound_pass": detection_pass,
            "false_call_bound_pass": false_call_pass,
            "inspector_screen_pass": detection_pass and false_call_pass,
        })
    disposition = "PASS_FIXED_TARGET_SCREEN" if all(row["inspector_screen_pass"] for row in detail) else "SCREENING_BOUNDS_NOT_MET"
    return disposition, detail


def verify(corpus_path: Path, rust_path: Path) -> dict:
    raw = corpus_path.read_bytes()
    corpus = json.loads(raw)
    rust = json.loads(rust_path.read_text(encoding="utf-8"))
    assert corpus["schema"] == "luminous.aero.nde-fixed-target-screen.v0.1"
    assert corpus["evidence_class"] == "synthetic"
    assert corpus["claim_ceiling"] == CLAIM_CEILING
    assert rust["schema"] == OUTPUT_SCHEMA
    assert rust["corpus_sha256"] == hashlib.sha256(raw).hexdigest()
    scope = corpus["scope"]
    scope_bytes = bytearray()
    for field in (
        "scope_ref",
        "method_id",
        "procedure_revision",
        "material_system",
        "defect_family",
        "defect_orientation",
        "environment_id",
    ):
        encoded = scope[field].encode("utf-8")
        scope_bytes.extend(struct.pack(">Q", len(encoded)))
        scope_bytes.extend(encoded)
    scope_bytes.extend(struct.pack(">d", float(scope["target_flaw_size_mm"])))
    scope_digest = hashlib.sha256(scope_bytes).hexdigest()
    assert rust["scope_sha256"] == scope_digest
    assert rust["claim_ceiling"] == CLAIM_CEILING
    assert rust["authority_granted"] is False
    assert rust["physical_pod"] == "NOT_ASSESSED"
    assert rust["a90_95"] == "NOT_ESTIMATED"
    assert rust["astm_e2862_compliance"] == "NOT_ASSESSED"
    assert rust["mil_hdbk_1823a_compliance"] == "NOT_ASSESSED"
    assert rust["inspection_acceptance"] == "NOT_AUTHORIZED"

    expected = {}
    for case in corpus["cases"]:
        study = copy.deepcopy(corpus["base_study"])
        for mutation in case["mutations"]:
            apply_mutation(study, mutation)
        disposition, detail = evaluate(study, corpus["scope"], scope_digest, corpus["criteria"])
        expected[case["case_id"]] = (disposition, detail, case["expected_disposition"])

    actual_results = {row["case_id"]: row for row in rust["results"]}
    assert set(actual_results) == set(expected), "case identity mismatch"
    detail_rows = {row["case_id"]: rows for row, rows in zip(rust["results"], rust["inspector_details"], strict=True)}
    for case_id, (disposition, details, expected_disposition) in expected.items():
        row = actual_results[case_id]
        assert disposition == expected_disposition, f"{case_id}: frozen expected {expected_disposition}, oracle computed {disposition}"
        assert row["disposition"] == disposition, f"{case_id}: Rust {row['disposition']} != Python {disposition}"
        assert row["expected_disposition"] == expected_disposition
        assert row["expected_matched"] is True
        rust_details = detail_rows[case_id]
        assert len(rust_details) == len(details), f"{case_id}: inspector-detail cardinality mismatch"
        for rust_item, py_item in zip(rust_details, details, strict=True):
            assert rust_item["inspector_id"] == py_item["inspector_id"]
            for field in ("detection_lower_bound", "false_call_upper_bound"):
                assert abs(rust_item[field] - py_item[field]) <= 1.0e-8, f"{case_id}/{field} numerical disagreement"
            for field in ("flaw_trials", "detections", "blank_trials", "false_calls", "detection_bound_pass", "false_call_bound_pass", "inspector_screen_pass"):
                assert rust_item[field] == py_item[field], f"{case_id}/{field} disagreement"

    assert rust["case_count"] == len(expected)
    assert rust["expected_matches"] == len(expected)
    assert rust["expected_mismatches"] == 0
    return {
        "schema": "luminous.aero.nde-fixed-target-python-audit.v0.1",
        "corpus_sha256": hashlib.sha256(raw).hexdigest(),
        "cases_checked": len(expected),
        "known_answer_matches": len(expected),
        "known_answer_mismatches": 0,
        "physical_pod": "NOT_ASSESSED",
        "a90_95": "NOT_ESTIMATED",
        "authority_granted": False,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--rust-output", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(verify(args.corpus, args.rust_output), sort_keys=True, indent=2))


if __name__ == "__main__":
    main()
