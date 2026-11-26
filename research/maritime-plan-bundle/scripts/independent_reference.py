#!/usr/bin/env python3
"""Independent standard-library reference for maritime plan-bundle coverage."""
from __future__ import annotations

import argparse
import csv
from collections import OrderedDict
from pathlib import Path


def requirement_disposition(row: list[str], cutoff: int) -> str:
    mode = row[21]
    if row[11] == "-":
        return "MISSING_REQUIREMENT"
    if mode == "MISSING":
        return "COMMITMENT_MISSING"
    if mode == "UNAVAILABLE":
        return "COMMITMENT_UNAVAILABLE"
    if mode != "PRESENT":
        return "COMMITMENT_INVALID"

    # Required commitment terms (source row) vs the projection receipt (provider).
    required = (
        row[12], row[13], row[14], row[15], row[16], row[17],
        row[18], row[19], row[20],
    )
    projected = (
        row[23], row[24], row[25], row[26], row[27], row[28],
        row[29], row[30], row[32],
    )
    if row[22] != "maritime-commitment-projection/0.1":
        return "COMMITMENT_INVALID"
    if int(row[34]) != cutoff:
        return "COMMITMENT_INVALID"
    if required != projected:
        return "TERMS_MISMATCH"

    state = row[33]
    if state == "ACCEPTED":
        if row[35] == "-" or int(row[35]) > cutoff:
            return "COMMITMENT_INVALID"
        if int(row[31]) <= cutoff:
            return "COMMITMENT_EXPIRED"
        return "SATISFIED"
    if state == "EXPIRED":
        return "COMMITMENT_EXPIRED"
    if state in {"OFFERED", "REJECTED", "REVOKED"}:
        return "COMMITMENT_NOT_ACCEPTED"
    return "COMMITMENT_INVALID"


def build_reference(rows: list[list[str]]) -> list[str]:
    groups: OrderedDict[str, list[list[str]]] = OrderedDict()
    for row in rows:
        if len(row) != 38:
            raise AssertionError(f"expected 38 TSV fields, got {len(row)}")
        groups.setdefault(row[0], []).append(row)

    receipt = [
        "PLAN_BUNDLE_RECEIPT\t0.1",
        "case_id\tbundle_disposition\tresult_count",
    ]
    for case_id in sorted(groups):
        case_rows = groups[case_id]
        first = case_rows[0]
        cutoff = int(first[1])
        for row in case_rows:
            if row[1:6] != first[1:6] or row[37] != first[37]:
                raise AssertionError(f"{case_id}: inconsistent group fields")

        physical = first[2]
        if physical == "INFEASIBLE":
            bundle = "PHYSICAL_PLAN_INFEASIBLE"
            result_rows: list[tuple[str, str, str]] = []
        elif physical == "UNKNOWN":
            bundle = "PHYSICAL_PLAN_UNKNOWN"
            result_rows = []
        elif physical != "FEASIBLE":
            raise AssertionError(f"{case_id}: invalid physical status {physical}")
        else:
            result_rows = []
            for row in case_rows:
                if row[6] == "-":
                    continue
                status = requirement_disposition(row, cutoff)
                if row[36] != status:
                    raise AssertionError(
                        f"{case_id}/{row[6]}: hand-authored expected {row[36]}, computed {status}"
                    )
                result_rows.append((row[6], row[12] if row[12] != "-" else "-", status))

            expected_bundle = "COMPLETE" if all(item[2] == "SATISFIED" for item in result_rows) else "INCOMPLETE"
            if expected_bundle != first[37]:
                raise AssertionError(
                    f"{case_id}: hand-authored expected bundle {first[37]}, computed {expected_bundle}"
                )
            bundle = expected_bundle

        if physical != "FEASIBLE" and first[37] != bundle:
            raise AssertionError(
                f"{case_id}: hand-authored expected bundle {first[37]}, computed {bundle}"
            )

        receipt.append(f"BUNDLE\t{case_id}\t{bundle}\t{len(result_rows)}")
        for reservation_id, commitment_id, status in result_rows:
            receipt.append(
                f"RESULT\t{case_id}\t{reservation_id}\t{commitment_id}\t{status}"
            )

    receipt.append(f"SUMMARY\tPASS\t{len(groups)}/{len(groups)}")
    return receipt


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument(
        "--fixture",
        type=Path,
        default=Path("research/maritime-plan-bundle/fixtures/bundles.tsv"),
    )
    args = parser.parse_args()

    with args.fixture.open("r", encoding="utf-8", newline="") as handle:
        rows = [
            row for row in csv.reader(handle, delimiter="\t")
            if row and not row[0].startswith("#")
        ]
    expected = build_reference(rows)
    actual = args.receipt.read_text(encoding="utf-8").splitlines()
    if actual != expected:
        raise AssertionError(
            f"Rust/reference receipt mismatch\nexpected={expected}\nactual={actual}"
        )

    case_count = len({row[0] for row in rows})
    print(f"PLAN_BUNDLE_REFERENCE\tPASS\t{case_count}/{case_count}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
