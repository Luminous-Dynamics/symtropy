#!/usr/bin/env python3
"""Independent, test-only MDTR reference evaluator (standard library only).

This deliberately does not import or invoke Rust implementation logic. It
re-parses the frozen TSV and computes its own policy-view and adjudication.
"""
from __future__ import annotations

import argparse
import csv
from pathlib import Path


def disposition(fields: list[str]) -> tuple[int, str, str, str, str]:
    (
        case_id, cutoff, subject, _source_id, event_id, event_subject,
        _event_time, published, retrieved, use_from, use_until, rights,
        physical, authority, participant, expected, accepted, in_transit,
        outstanding, unresolved, duplicated, expected_disposition,
        expected_reason,
    ) = fields

    cutoff_i = int(cutoff)
    if rights != "true":
        result = (0, "ABSTAIN", "-", "reject", "SOURCE_RIGHTS_DENIED")
    elif event_subject != subject:
        result = (0, "ABSTAIN", "-", "reject", "SUBJECT_BINDING_MISMATCH")
    else:
        visible = (
            int(published) <= cutoff_i
            and int(retrieved) <= cutoff_i
            and int(use_from) <= cutoff_i <= int(use_until)
        )
        if not visible:
            result = (0, "ABSTAIN", "-", "abstain", "NO_VISIBLE_EVIDENCE")
        else:
            rejected: list[str] = []
            uncertain: list[str] = []
            for value, reject_code, unknown_code in (
                (physical, "PHYSICAL_INFEASIBLE", "PHYSICAL_UNKNOWN"),
                (authority, "AUTHORITY_DENIED", "AUTHORITY_UNKNOWN"),
                (participant, "PARTICIPANT_FLOOR_VIOLATION", "PARTICIPANT_FLOOR_UNKNOWN"),
            ):
                if value == "FAIL":
                    rejected.append(reject_code)
                elif value == "UNKNOWN":
                    uncertain.append(unknown_code)
                elif value != "PASS":
                    raise ValueError(f"{case_id}: unsupported gate value {value}")

            expected_i, accepted_i, transit_i, outstanding_i, unresolved_i, duplicate_i = map(
                int, (expected, accepted, in_transit, outstanding, unresolved, duplicated)
            )
            if duplicate_i:
                rejected.append("CARGO_DUPLICATION")

            accounted = accepted_i + transit_i + outstanding_i + unresolved_i
            if accounted > expected_i:
                rejected.append("CARGO_OVERCOUNT")
            elif accounted < expected_i:
                uncertain.append("CARGO_UNRECONCILED")

            if rejected:
                result = (
                    1, "PROPOSE_REVIEW", event_id, "reject",
                    "|".join(rejected + uncertain),
                )
            elif uncertain:
                result = (
                    1, "PROPOSE_REVIEW", event_id, "abstain",
                    "|".join(uncertain),
                )
            else:
                result = (
                    1, "PROPOSE_REVIEW", event_id,
                    "eligible_for_decision_review", "ALL_GATES_PASS",
                )

    # The frozen fixture's hand-authored expected result is a third oracle.
    if result[3] != expected_disposition or result[4] != expected_reason:
        raise AssertionError(
            f"{case_id}: independent result {result[3]}/{result[4]} "
            f"does not match frozen expected {expected_disposition}/{expected_reason}"
        )
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument(
        "--fixture",
        type=Path,
        default=Path("research/maritime-decision-time-replay/fixtures/episodes.tsv"),
    )
    parser.add_argument("--temporal-receipt", type=Path)
    parser.add_argument(
        "--temporal-fixture",
        type=Path,
        default=Path("research/maritime-decision-time-replay/fixtures/temporal-cases.tsv"),
    )
    args = parser.parse_args()

    with args.fixture.open("r", encoding="utf-8", newline="") as handle:
        fixture_rows = [
            row for row in csv.reader(handle, delimiter="\t")
            if row and not row[0].startswith("#")
        ]

    expected_rows: dict[str, tuple[int, str, str, str, str]] = {}
    for fields in fixture_rows:
        if len(fields) != 23:
            raise AssertionError(f"fixture row must have 23 fields, got {len(fields)}")
        case_id = fields[0]
        if case_id in expected_rows:
            raise AssertionError(f"duplicate fixture case id: {case_id}")
        expected_rows[case_id] = disposition(fields)

    receipt_lines = args.receipt.read_text(encoding="utf-8").splitlines()
    if len(receipt_lines) < 4 or receipt_lines[0] != "MDTR_RECEIPT\t0.1.0":
        raise AssertionError("missing or invalid MDTR receipt header")
    if receipt_lines[1] != "case_id\tvisible_count\tpolicy_action\tevidence_ids\tdisposition\treason":
        raise AssertionError("unexpected MDTR receipt columns")

    actual_rows: dict[str, tuple[int, str, str, str, str]] = {}
    for line in receipt_lines[2:]:
        fields = line.split("\t")
        if fields[0] == "SUMMARY":
            continue
        if len(fields) != 6:
            raise AssertionError(f"receipt row must have 6 fields: {line}")
        case_id, visible_count, action, evidence_ids, actual_disposition, reason = fields
        if case_id in actual_rows:
            raise AssertionError(f"duplicate receipt case id: {case_id}")
        actual_rows[case_id] = (
            int(visible_count), action, evidence_ids, actual_disposition, reason
        )

    if actual_rows != expected_rows:
        missing = sorted(set(expected_rows) - set(actual_rows))
        extra = sorted(set(actual_rows) - set(expected_rows))
        mismatched = sorted(
            case_id for case_id in set(actual_rows) & set(expected_rows)
            if actual_rows[case_id] != expected_rows[case_id]
        )
        raise AssertionError(
            f"Rust/reference mismatch; missing={missing}, extra={extra}, mismatched={mismatched}"
        )

    print(f"INDEPENDENT_REFERENCE\tPASS\t{len(actual_rows)}/{len(expected_rows)}")
    if args.temporal_receipt is not None:
        verify_temporal_reference(args.temporal_receipt, args.temporal_fixture)
    return 0


def parse_adjudications(value: str) -> list[tuple[int, int, str]]:
    if value == "-":
        return []
    parsed: list[tuple[int, int, str]] = []
    for item in value.split(";"):
        revision_s, recorded_s, state = item.split("@")
        if state not in {"ACCEPTED", "REJECTED", "DISPUTED", "NOT_EVALUABLE"}:
            raise ValueError(f"unsupported adjudication state: {state}")
        parsed.append((int(revision_s), int(recorded_s), state))
    return parsed


def temporal_group_reference(rows: list[list[str]]) -> tuple[str, str, str]:
    first = rows[0]
    physical_i, decision_i, audit_i = map(int, (first[1], first[2], first[3]))
    expected_subject = first[4]
    for row in rows:
        if (row[1], row[2], row[3], row[4], row[19], row[20], row[21]) != (
            first[1], first[2], first[3], first[4], first[19], first[20], first[21]
        ):
            raise AssertionError(f"{first[0]}: inconsistent group cutoffs or expectations")

    admissible: list[list[str]] = []
    for row in rows:
        valid = int(row[11]) <= physical_i <= int(row[12])
        published = int(row[13]) <= decision_i
        retrieved = int(row[14]) <= decision_i
        rights = (
            row[15] == "true"
            and int(row[16]) <= decision_i <= int(row[17])
        )
        if valid and published and retrieved and rights:
            if row[7] != expected_subject:
                return "-", "-", "SUBJECT_BINDING_MISMATCH"
            admissible.append(row)

    seen: dict[tuple[str, str, int], str] = {}
    selected: dict[tuple[str, str], list[str]] = {}
    for row in admissible:
        source_id, event_id, revision = row[5], row[6], int(row[8])
        identity = (source_id, event_id, revision)
        digest = row[9]
        if identity in seen:
            if seen[identity] != digest:
                return "-", "-", "CONFLICTING_SOURCE_REVISION"
            continue
        seen[identity] = digest

        source_event = (source_id, event_id)
        current = selected.get(source_event)
        if current is None or revision > int(current[8]):
            selected[source_event] = row

    selected_rows = [selected[key] for key in sorted(selected)]
    selected_text = ",".join(
        f"{row[5]}:{row[6]}:{row[8]}" for row in selected_rows
    ) or "-"

    audit_values: list[str] = []
    for row in selected_rows:
        adjudications = [
            item for item in parse_adjudications(row[18]) if item[1] <= audit_i
        ]
        latest: tuple[int, int, str] | None = None
        for candidate in adjudications:
            if latest is None or candidate[0] > latest[0]:
                latest = candidate
            elif candidate[0] == latest[0] and candidate != latest:
                return "-", "-", "CONFLICTING_ADJUDICATION_REVISION"
        key = f"{row[5]}:{row[6]}:{row[8]}"
        if latest is None:
            audit_values.append(f"{key}=NOT_YET")
        else:
            audit_values.append(f"{key}={latest[2]}@{latest[0]}")
    audit_text = ",".join(audit_values) or "-"
    return selected_text, audit_text, "-"


def verify_temporal_reference(receipt_path: Path, fixture_path: Path) -> None:
    with fixture_path.open("r", encoding="utf-8", newline="") as handle:
        fixture_rows = [
            row for row in csv.reader(handle, delimiter="\t")
            if row and not row[0].startswith("#")
        ]

    groups: dict[str, list[list[str]]] = {}
    for fields in fixture_rows:
        if len(fields) != 22:
            raise AssertionError(f"temporal fixture row must have 22 fields, got {len(fields)}")
        groups.setdefault(fields[0], []).append(fields)

    expected_rows: dict[str, tuple[str, str, str]] = {}
    for case_id, rows in groups.items():
        expected = temporal_group_reference(rows)
        hand_authored = (rows[0][19], rows[0][20], rows[0][21])
        if expected != hand_authored:
            raise AssertionError(
                f"{case_id}: independent temporal reference {expected} "
                f"does not match frozen expected {hand_authored}"
            )
        expected_rows[case_id] = expected

    receipt_lines = receipt_path.read_text(encoding="utf-8").splitlines()
    if len(receipt_lines) < 4 or receipt_lines[0] != "MDTR_TEMPORAL_RECEIPT\t0.1.0":
        raise AssertionError("missing or invalid MDTR temporal receipt header")
    if receipt_lines[1] != "case_id\tselected_revisions\taudit_projection\terror":
        raise AssertionError("unexpected MDTR temporal receipt columns")

    actual_rows: dict[str, tuple[str, str, str]] = {}
    for line in receipt_lines[2:]:
        fields = line.split("\t")
        if fields[0] == "SUMMARY":
            continue
        if len(fields) != 4:
            raise AssertionError(f"temporal receipt row must have 4 fields: {line}")
        case_id, selected, audit, error = fields
        if case_id in actual_rows:
            raise AssertionError(f"duplicate temporal receipt case id: {case_id}")
        actual_rows[case_id] = (selected, audit, error)

    if actual_rows != expected_rows:
        missing = sorted(set(expected_rows) - set(actual_rows))
        extra = sorted(set(actual_rows) - set(expected_rows))
        mismatched = sorted(
            case_id for case_id in set(actual_rows) & set(expected_rows)
            if actual_rows[case_id] != expected_rows[case_id]
        )
        raise AssertionError(
            f"temporal Rust/reference mismatch; missing={missing}, extra={extra}, "
            f"mismatched={mismatched}"
        )
    print(f"TEMPORAL_REFERENCE\tPASS\t{len(actual_rows)}/{len(expected_rows)}")


if __name__ == "__main__":
    raise SystemExit(main())
