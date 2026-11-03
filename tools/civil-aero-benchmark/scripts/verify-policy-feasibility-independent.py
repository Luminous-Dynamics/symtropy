#!/usr/bin/env python3
"""Independent verifier for policy false-feasible bounds and candidate provenance."""
from __future__ import annotations

import csv
import hashlib
import math
import struct
import sys
from pathlib import Path

SCHEMA = "aero-policy-feasibility-audit/0.1.0"
EVIDENCE = (
    "exact_admitted_candidate_frame_with_separate_caller_declared_independent_"
    "adjudication_and_worst_case_unresolved_bounds_not_authenticated_verifier_"
    "independence_or_physical_certification"
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as stream:
        return list(csv.DictReader(stream))


def add_string(buffer: bytearray, value: str) -> None:
    raw = value.encode("utf-8")
    buffer.extend(struct.pack("<Q", len(raw)))
    buffer.extend(raw)


def add_u64(buffer: bytearray, value: int) -> None:
    buffer.extend(struct.pack("<Q", value))


def add_optional_rate(buffer: bytearray, bits: str) -> float | None:
    if not bits:
        buffer.append(0)
        return None
    require(len(bits) == 16 and all(ch in "0123456789abcdef" for ch in bits),
            "rate f64 bits must be 16 lowercase hex digits")
    raw = int(bits, 16)
    buffer.append(1)
    buffer.extend(struct.pack("<Q", raw))
    value = struct.unpack("<d", struct.pack("<Q", raw))[0]
    require(math.isfinite(value) and 0.0 <= value <= 1.0, "rate is not a finite fraction")
    return value


def check_sha(value: str, label: str) -> None:
    require(
        len(value) == 64
        and value == value.lower()
        and all(ch in "0123456789abcdef" for ch in value),
        f"{label} is not a lowercase SHA-256 digest",
    )


def check_same(rows: list[dict[str, str]], key: str) -> str:
    values = {row[key] for row in rows}
    require(len(values) == 1, f"report rows disagree on {key}")
    return next(iter(values))


def main() -> int:
    if len(sys.argv) != 2:
        raise ValueError("usage: verify-policy-feasibility-independent.py REPORT.csv")
    rows = read_csv(Path(sys.argv[1]))
    require(rows, "report CSV must contain at least a header and one summary row")
    first = rows[0]
    require(first["schema"] == SCHEMA, "schema mismatch")
    require(first["evidence_level"] == EVIDENCE, "evidence ceiling mismatch")

    summary_fields = [
        "schema", "frame_id", "policy_id", "selection_design",
        "selection_design_sha256", "selection_plan_sha256", "selection_receipt_sha256",
        "sampling_frame_sha256", "candidate_frame_sha256", "adjudication_manifest_sha256",
        "admitted_candidate_count", "adjudicated_candidate_count", "confirmed_feasible_count",
        "falsified_infeasible_count", "inconclusive_count", "unassessed_count",
        "resolved_count", "false_feasible_rate_lower_bound",
        "false_feasible_rate_upper_bound", "false_feasible_rate_among_resolved",
        "false_feasible_rate_lower_bound_f64_bits",
        "false_feasible_rate_upper_bound_f64_bits",
        "false_feasible_rate_among_resolved_f64_bits", "status", "report_sha256",
        "evidence_level",
    ]
    summary = {field: check_same(rows, field) for field in summary_fields}
    candidate_fields = [
        "task_id", "replicate_id", "operation_id", "candidate_id", "output_sha256",
        "scope_sha256", "admission_receipt_sha256", "admission_verifier_id",
        "admission_verifier_sha256", "adjudication_status", "method", "adjudicator_id",
        "adjudicator_sha256", "adjudication_receipt_sha256", "reason",
    ]
    candidates = [row for row in rows if row["candidate_id"]]
    require(
        len(candidates) == len(rows) or (not candidates and len(rows) == 1),
        "CSV must contain one row per candidate or one empty-frame summary row",
    )

    admitted = int(summary["admitted_candidate_count"])
    adjudicated = int(summary["adjudicated_candidate_count"])
    confirmed = int(summary["confirmed_feasible_count"])
    falsified = int(summary["falsified_infeasible_count"])
    inconclusive = int(summary["inconclusive_count"])
    unassessed = int(summary["unassessed_count"])
    resolved = int(summary["resolved_count"])
    require(admitted == len(candidates), "admitted-candidate count differs from CSV frame")
    require(adjudicated == confirmed + falsified + inconclusive,
            "adjudicated count does not equal resolved plus inconclusive outcomes")
    require(unassessed == sum(row["adjudication_status"] == "unassessed" for row in candidates),
            "unassessed count differs from candidate rows")
    require(confirmed == sum(row["adjudication_status"] == "confirmed_feasible" for row in candidates),
            "confirmed-feasible count differs from candidate rows")
    require(falsified == sum(row["adjudication_status"] == "falsified_infeasible" for row in candidates),
            "falsified-infeasible count differs from candidate rows")
    require(inconclusive == sum(row["adjudication_status"] == "inconclusive" for row in candidates),
            "inconclusive count differs from candidate rows")
    require(resolved == confirmed + falsified, "resolved count mismatch")

    design = summary["selection_design"]
    plan_sha = summary["selection_plan_sha256"]
    selection_receipt_sha = summary["selection_receipt_sha256"]
    if design == "full_census":
        require(not plan_sha and not selection_receipt_sha, "full census must not carry sample selection references")
    elif design in {"pre_registered_probability_sample", "targeted_challenge_set"}:
        check_sha(plan_sha, "selection plan")
        check_sha(selection_receipt_sha, "selection receipt")
    else:
        raise ValueError(f"unknown selection design {design!r}")

    seen_candidates: set[str] = set()
    seen_operations: set[tuple[str, str, str, str]] = set()
    candidate_map: dict[str, dict[str, str]] = {}
    for row in candidates:
        candidate_id = row["candidate_id"]
        require(candidate_id and candidate_id not in seen_candidates, "duplicate or empty candidate ID")
        seen_candidates.add(candidate_id)
        key = (row["task_id"], row["replicate_id"], summary["policy_id"], row["operation_id"])
        require(all(key), "candidate operation identity contains an empty field")
        require(key not in seen_operations, "multiple candidates bind to the same operation")
        seen_operations.add(key)
        for field in (
            "output_sha256", "scope_sha256", "admission_receipt_sha256",
            "admission_verifier_sha256",
        ):
            check_sha(row[field], field)
        require(row["admission_verifier_id"], "admission verifier identity is empty")
        status = row["adjudication_status"]
        require(status in {
            "confirmed_feasible", "falsified_infeasible", "inconclusive", "unassessed"
        }, f"unknown candidate adjudication status {status!r}")
        if status == "unassessed":
            require(
                not row["method"] and not row["adjudicator_id"]
                and not row["adjudicator_sha256"] and not row["adjudication_receipt_sha256"],
                "unassessed candidate contains a claimed adjudication reference",
            )
            require(row["reason"], "unassessed candidate requires an explicit reason")
        else:
            check_sha(row["adjudicator_sha256"], "adjudicator")
            check_sha(row["adjudication_receipt_sha256"], "adjudication receipt")
            require(row["method"] and row["adjudicator_id"], "adjudication method/identity is empty")
            require(row["adjudicator_id"] != row["admission_verifier_id"],
                    "adjudicator identity reuses admission verifier identity")
            require(row["adjudicator_sha256"] != row["admission_verifier_sha256"],
                    "adjudicator digest reuses admission verifier digest")
            require(row["adjudication_receipt_sha256"] != row["admission_receipt_sha256"],
                    "adjudication receipt reuses admission receipt")
            if status == "inconclusive":
                require(row["reason"], "inconclusive candidate requires a reason")
            else:
                require(not row["reason"], "resolved adjudication must not contain an unresolved reason")
        candidate_map[candidate_id] = row

    frame = bytearray()
    add_string(frame, "aero-admitted-candidate-frame/0.1.0")
    add_string(frame, summary["policy_id"])
    add_string(frame, summary["frame_id"])
    ordered = sorted(candidate_map.items())
    add_u64(frame, len(ordered))
    for candidate_id, row in ordered:
        for value in (
            candidate_id, row["task_id"], row["replicate_id"], summary["policy_id"],
            row["operation_id"], row["output_sha256"], row["admission_receipt_sha256"],
            row["admission_verifier_id"], row["admission_verifier_sha256"], row["scope_sha256"],
        ):
            add_string(frame, value)
    frame_digest = hashlib.sha256(frame).hexdigest()
    require(frame_digest == summary["candidate_frame_sha256"] == summary["sampling_frame_sha256"],
            "candidate frame or selection sampling-frame digest mismatch")

    selection = bytearray()
    add_string(selection, "aero-adjudication-selection-design/0.1.0")
    add_string(selection, design)
    add_string(selection, summary["sampling_frame_sha256"])
    if design != "full_census":
        add_string(selection, plan_sha)
        add_string(selection, selection_receipt_sha)
    selection_digest = hashlib.sha256(selection).hexdigest()
    require(selection_digest == summary["selection_design_sha256"],
            "selection-design canonical digest mismatch")

    adjudicated_rows = [
        row for row in candidates if row["adjudication_status"] != "unassessed"
    ]
    adjudication = bytearray()
    add_string(adjudication, "aero-independent-feasibility-adjudications/0.1.0")
    add_u64(adjudication, len(adjudicated_rows))
    for row in sorted(adjudicated_rows, key=lambda item: item["candidate_id"]):
        for value in (
            row["candidate_id"], row["output_sha256"], row["scope_sha256"],
            row["adjudicator_id"], row["adjudicator_sha256"],
            row["adjudication_receipt_sha256"], row["method"], row["adjudication_status"],
            row["reason"],
        ):
            add_string(adjudication, value)
    adjudication_digest = hashlib.sha256(adjudication).hexdigest()
    require(adjudication_digest == summary["adjudication_manifest_sha256"],
            "adjudication manifest canonical digest mismatch")

    if design == "full_census":
        require(unassessed == 0, "full census contains unassessed candidates")
    expected_status = (
        "no_admitted_candidates" if admitted == 0
        else "complete_census_all_adjudications_resolved"
        if unassessed + inconclusive == 0 and design == "full_census"
        else "sampled_or_targeted_frame_all_selected_adjudications_resolved"
        if unassessed + inconclusive == 0
        else "unresolved_candidates_included_in_worst_case_bounds"
    )
    require(summary["status"] == expected_status, "report status differs from candidate outcomes")

    expected_lower = None if admitted == 0 else falsified / admitted
    expected_upper = None if admitted == 0 else (falsified + inconclusive + unassessed) / admitted
    expected_resolved = None if resolved == 0 else falsified / resolved
    bit_values = [
        add_optional_rate(frame_bits, summary[f"false_feasible_rate_{name}_f64_bits"])
        for name, frame_bits in (
            ("lower_bound", bytearray()),
            ("upper_bound", bytearray()),
            ("among_resolved", bytearray()),
        )
    ]
    rate_names = (
        "false_feasible_rate_lower_bound",
        "false_feasible_rate_upper_bound",
        "false_feasible_rate_among_resolved",
    )
    for index, expected in enumerate((expected_lower, expected_upper, expected_resolved)):
        observed = bit_values[index]
        require(
            (observed is None and expected is None)
            or (observed is not None and expected is not None and observed == expected),
            f"{rate_names[index]} exact value mismatch",
        )
        displayed = summary[rate_names[index]]
        require(
            (not displayed and expected is None)
            or (displayed and expected is not None and float(displayed) == round(expected, 12)),
            f"{rate_names[index]} display value mismatch",
        )

    canonical = bytearray()
    for value in (
        SCHEMA, summary["frame_id"], summary["policy_id"], design, selection_digest,
        plan_sha, selection_receipt_sha, summary["sampling_frame_sha256"],
        frame_digest, adjudication_digest, summary["status"], EVIDENCE,
    ):
        add_string(canonical, value)
    for count in (
        admitted, adjudicated, confirmed, falsified, inconclusive, unassessed, resolved,
    ):
        add_u64(canonical, count)
    add_u64(canonical, len(ordered))
    for candidate_id, row in ordered:
        for value in (
            row["task_id"], row["replicate_id"], row["operation_id"], candidate_id,
            row["output_sha256"], row["scope_sha256"], row["admission_receipt_sha256"],
            row["admission_verifier_id"], row["admission_verifier_sha256"],
            row["adjudication_status"], row["method"], row["adjudicator_id"],
            row["adjudicator_sha256"], row["adjudication_receipt_sha256"], row["reason"],
        ):
            add_string(canonical, value)
    for bits_field in (
        "false_feasible_rate_lower_bound_f64_bits",
        "false_feasible_rate_upper_bound_f64_bits",
        "false_feasible_rate_among_resolved_f64_bits",
    ):
        add_optional_rate(canonical, summary[bits_field])
    report_digest = hashlib.sha256(canonical).hexdigest()
    require(report_digest == summary["report_sha256"], "report canonical digest mismatch")

    print(
        "independent_policy_feasibility_oracle=pass "
        f"policy={summary['policy_id']} admitted={admitted} confirmed={confirmed} "
        f"falsified={falsified} inconclusive={inconclusive} unassessed={unassessed} "
        f"lower={expected_lower} upper={expected_upper} report_sha256={report_digest}"
    )
    return 0


def add_optional_rate(buffer: bytearray, bits: str) -> float | None:
    if not bits:
        buffer.append(0)
        return None
    require(len(bits) == 16 and all(ch in "0123456789abcdef" for ch in bits),
            "rate f64 bits must be 16 lowercase hex digits")
    raw = int(bits, 16)
    buffer.append(1)
    buffer.extend(struct.pack("<Q", raw))
    value = struct.unpack("<d", struct.pack("<Q", raw))[0]
    require(math.isfinite(value) and 0.0 <= value <= 1.0, "rate is not a finite fraction")
    return value


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"independent policy feasibility oracle failed: {error}", file=sys.stderr)
        raise SystemExit(1)
