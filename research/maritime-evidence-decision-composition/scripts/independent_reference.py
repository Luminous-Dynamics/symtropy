#!/usr/bin/env python3
"""Independent, test-only reference reducer for the frozen composition corpus.

This is not production code and has no third-party dependencies. It deliberately
uses its own JSON/TSV parsers and a literal known-answer table rather than calling
the Rust reducer or sharing its case-injection implementation.
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
JSON_FIXTURE = ROOT / "fixtures" / "composition-cases.json"
TSV_FIXTURE = ROOT / "fixtures" / "composition-cases.tsv"

# Independently adjudicated expected outcomes for the frozen synthetic mutations.
# Keep this table intentionally separate from the Rust fault injector.
REFERENCE = {
    "NONE": ("eligible_for_decision_review", "ALL_REQUIRED_BINDINGS_MATCH"),
    "CAPTURE_NOT_DURABLE": ("abstain", "CAPTURE_NOT_DURABLE"),
    "EVENT_ID_PAYLOAD_CONFLICT": ("reject", "EVENT_ID_PAYLOAD_CONFLICT"),
    "SUBJECT_BINDING_MISMATCH": ("reject", "SUBJECT_BINDING_MISMATCH"),
    "ARTIFACT_NOT_VISIBLE_AS_OF": ("abstain", "ARTIFACT_NOT_VISIBLE_AS_OF"),
    "EVENT_NOT_KNOWN_AT_DECISION_TIME": ("abstain", "EVENT_NOT_KNOWN_AT_DECISION_TIME"),
    "TIME_BINDING_NOT_EVALUABLE": ("abstain", "TIME_BINDING_NOT_EVALUABLE"),
    "UNSUPPORTED_CLAIM_PROMOTION": ("reject", "UNSUPPORTED_CLAIM_PROMOTION"),
    "UNSUPPORTED_OUTCOME_PROMOTION": ("reject", "UNSUPPORTED_OUTCOME_PROMOTION"),
    "RECEIVER_PROOF_REQUIRED": ("reject", "RECEIVER_PROOF_REQUIRED"),
    "CUSTODY_CLOSURE_INCOMPLETE": ("abstain", "CUSTODY_CLOSURE_INCOMPLETE"),
    "ORACLE_INDEPENDENCE_NOT_ESTABLISHED": ("abstain", "ORACLE_INDEPENDENCE_NOT_ESTABLISHED"),
    "ORACLE_IMPLEMENTATION_DRIFT": ("reject", "ORACLE_IMPLEMENTATION_DRIFT"),
    "RECEIPT_EPISODE_BINDING_MISMATCH": ("reject", "RECEIPT_EPISODE_BINDING_MISMATCH"),
    "AUTHORIZATION_REVOKED": ("reject", "AUTHORIZATION_REVOKED"),
    "AUTHORIZATION_SCOPE_INVALID": ("reject", "AUTHORIZATION_SCOPE_INVALID"),
    "UNKNOWN_PHYSICAL_STATE_BLOCKS": ("abstain", "UNKNOWN_PHYSICAL_STATE_BLOCKS"),
    "NON_COMPENSABLE_PARTICIPANT_FLOOR": ("reject", "NON_COMPENSABLE_PARTICIPANT_FLOOR"),
    "DEPENDENCE_UNIT_MISDECLARED": ("reject", "DEPENDENCE_UNIT_MISDECLARED"),
    "CALIBRATION_EVALUATION_LEAKAGE": ("reject", "CALIBRATION_EVALUATION_LEAKAGE"),
    "FROZEN_POLICY_GRID_MISMATCH": ("reject", "FROZEN_POLICY_GRID_MISMATCH"),
    "MISSING_OUTCOME_PROMOTED": ("reject", "MISSING_OUTCOME_PROMOTED"),
    "CARGO_CONSERVATION_VIOLATION": ("reject", "CARGO_CONSERVATION_VIOLATION"),
    "RECEIVER_HANDOFF_BINDING_MISMATCH": ("reject", "RECEIVER_HANDOFF_BINDING_MISMATCH"),
    "STATISTICAL_GUARANTEE_TYPE_MISMATCH": ("reject", "STATISTICAL_GUARANTEE_TYPE_MISMATCH"),
    "OBSERVATION_STALE_FOR_DECISION": ("abstain", "OBSERVATION_STALE_FOR_DECISION"),
    "OBSERVATION_VALUE_NOT_EVALUABLE": ("abstain", "OBSERVATION_VALUE_NOT_EVALUABLE"),
    "AS_OF_HISTORY_MUTATED": ("reject", "AS_OF_HISTORY_MUTATED"),
    "SOURCE_RIGHTS_VIOLATION": ("reject", "SOURCE_RIGHTS_VIOLATION"),
    "NO_FEASIBLE_AUTHORIZED_ACTION": ("abstain", "NO_FEASIBLE_AUTHORIZED_ACTION"),
    "AGGREGATE_CLAIM_NOT_UNIT_RECEIPT": ("abstain", "AGGREGATE_CLAIM_NOT_UNIT_RECEIPT"),
}


def read_tsv(path: Path) -> list[tuple[str, str, str, str]]:
    rows = []
    for number, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith("case_id|"):
            continue
        parts = line.split("|")
        if len(parts) != 4:
            raise ValueError(f"{path}:{number}: expected four fields")
        rows.append(tuple(parts))
    return rows


def main() -> int:
    cases = json.loads(JSON_FIXTURE.read_text(encoding="utf-8"))["cases"]
    tsv_rows = read_tsv(TSV_FIXTURE)

    json_ids = [case["id"] for case in cases]
    tsv_ids = [row[0] for row in tsv_rows]
    if len(json_ids) != len(set(json_ids)) or json_ids != tsv_ids:
        raise AssertionError("JSON/TSV IDs are duplicated or differ in order")

    expected_rows = []
    for case in cases:
        fault = next((row[1] for row in tsv_rows if row[0] == case["id"]), None)
        if fault is None or fault not in REFERENCE:
            raise AssertionError(f"missing or unknown reference fault for {case['id']}")
        disposition, reason = REFERENCE[fault]
        if case["expected"] != disposition or case["reason"] != reason:
            raise AssertionError(f"JSON known answer disagrees with independent reference: {case['id']}")
        expected_rows.append(f"{case['id']}|{disposition}|{reason}")

    for row in tsv_rows:
        case_id, fault, disposition, reason = row
        if REFERENCE.get(fault) != (disposition, reason):
            raise AssertionError(f"TSV known answer disagrees with independent reference: {case_id}")

    binary = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "target" / "debug" / "composition-audit"
    run = subprocess.run([str(binary)], cwd=ROOT, check=False, capture_output=True, text=True)
    if run.returncode != 0:
        sys.stderr.write(run.stderr)
        raise SystemExit(f"Rust composition audit failed with exit code {run.returncode}")

    actual_rows = [line for line in run.stdout.splitlines() if line and not line.startswith("case_id|")]
    if actual_rows != expected_rows:
        for index, (expected, actual) in enumerate(zip(expected_rows, actual_rows)):
            if expected != actual:
                raise AssertionError(f"Rust/reference mismatch at row {index}: {expected!r} != {actual!r}")
        raise AssertionError(f"Rust receipt row count {len(actual_rows)} != reference count {len(expected_rows)}")

    print(f"independent-reference: cases={len(cases)} matched={len(actual_rows)}")
    print("JSON/TSV parity: exact")
    print("Rust/reference receipt: exact")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
