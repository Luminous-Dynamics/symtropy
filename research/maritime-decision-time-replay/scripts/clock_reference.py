#!/usr/bin/env python3
"""Independent standard-library interval-order reference for MDTR clock bounds."""
from __future__ import annotations

import argparse
import csv
from pathlib import Path


def relation_leq(left: tuple[int, int], right: tuple[int, int]) -> str:
    if left[1] <= right[0]:
        return "PASS"
    if left[0] > right[1]:
        return "FAIL"
    return "UNCERTAIN"


def evaluate(fields: list[str]) -> tuple[str, str]:
    numeric_indices = list(range(1, 13)) + list(range(14, 18))
    values = {index: int(fields[index]) for index in numeric_indices}
    pairs = [(1, 2), (3, 4), (5, 6), (7, 8), (9, 10), (11, 12), (14, 15), (16, 17)]
    for lower_index, upper_index in pairs:
        if values[lower_index] > values[upper_index]:
            return "INVALID_INTERVAL", "INVALID_INTERVAL"

    physical = (values[1], values[2])
    decision = (values[3], values[4])
    valid_from = (values[5], values[6])
    valid_until = (values[7], values[8])
    published = (values[9], values[10])
    retrieved = (values[11], values[12])
    rights_from = (values[14], values[15])
    rights_until = (values[16], values[17])

    reasons: list[str] = []
    uncertain = False
    inadmissible = False

    checks = [
        (valid_from, physical, "PHYSICAL_BEFORE_VALID_FROM", "PHYSICAL_VALID_FROM_ORDER_UNCERTAIN"),
        (physical, valid_until, "PHYSICAL_AFTER_VALID_UNTIL", "PHYSICAL_VALID_UNTIL_ORDER_UNCERTAIN"),
        (published, decision, "PUBLISHED_AFTER_DECISION", "PUBLICATION_ORDER_UNCERTAIN"),
        (retrieved, decision, "RETRIEVED_AFTER_DECISION", "RETRIEVAL_ORDER_UNCERTAIN"),
    ]
    for left, right, reject_reason, uncertain_reason in checks:
        relation = relation_leq(left, right)
        if relation == "FAIL":
            reasons.append(reject_reason)
            inadmissible = True
        elif relation == "UNCERTAIN":
            reasons.append(uncertain_reason)
            uncertain = True

    if fields[13] == "false":
        reasons.append("SOURCE_RIGHTS_DENIED")
        inadmissible = True
    elif fields[13] != "true":
        raise AssertionError(f"invalid rights flag: {fields[13]}")

    for left, right, reject_reason, uncertain_reason in [
        (rights_from, decision, "RIGHTS_NOT_YET_EFFECTIVE", "RIGHTS_START_ORDER_UNCERTAIN"),
        (decision, rights_until, "RIGHTS_EXPIRED", "RIGHTS_END_ORDER_UNCERTAIN"),
    ]:
        relation = relation_leq(left, right)
        if relation == "FAIL":
            reasons.append(reject_reason)
            inadmissible = True
        elif relation == "UNCERTAIN":
            reasons.append(uncertain_reason)
            uncertain = True

    if inadmissible:
        admission = "DEFINITELY_INADMISSIBLE"
    elif uncertain:
        admission = "TEMPORALLY_UNCERTAIN"
    else:
        admission = "DEFINITELY_ADMISSIBLE"
    if not reasons:
        reasons.append("ALL_CLOCK_BOUNDS_ESTABLISHED")
    return admission, "|".join(reasons)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument(
        "--fixture",
        type=Path,
        default=Path("research/maritime-decision-time-replay/fixtures/clock-cases.tsv"),
    )
    args = parser.parse_args()

    with args.fixture.open("r", encoding="utf-8", newline="") as handle:
        fixture_rows = [
            row for row in csv.reader(handle, delimiter="\t")
            if row and not row[0].startswith("#")
        ]

    expected = ["MDTR_CLOCK_RECEIPT\t0.1", "case_id\tadmission\treasons"]
    seen: set[str] = set()
    for fields in fixture_rows:
        if len(fields) != 20:
            raise AssertionError(f"clock fixture row must have 20 fields, got {len(fields)}")
        case_id = fields[0]
        if case_id in seen:
            raise AssertionError(f"duplicate clock case ID {case_id}")
        seen.add(case_id)
        admission, reasons = evaluate(fields)
        hand_authored = (fields[18], fields[19])
        if (admission, reasons) != hand_authored:
            raise AssertionError(
                f"{case_id}: independent result {(admission, reasons)} "
                f"differs from frozen expected {hand_authored}"
            )
        expected.append(f"{case_id}\t{admission}\t{reasons}")

    expected.append(f"SUMMARY\tPASS\t{len(seen)}/{len(seen)}")
    actual = args.receipt.read_text(encoding="utf-8").splitlines()
    if actual != expected:
        raise AssertionError(f"clock Rust/reference mismatch\nexpected={expected}\nactual={actual}")

    print(f"MDTR_CLOCK_REFERENCE\tPASS\t{len(seen)}/{len(seen)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
