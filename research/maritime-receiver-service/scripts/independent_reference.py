#!/usr/bin/env python3
"""Independent standard-library reference for receiver-side service outcomes."""
from __future__ import annotations

import argparse
from collections import defaultdict
from pathlib import Path


def parse_fixture(path: Path):
    cases: dict[str, dict] = {}
    for line_number, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        fields = line.split("\t")
        kind = fields[0]
        if kind == "CASE" and len(fields) == 3:
            case_id = fields[1]
            if case_id in cases:
                raise AssertionError(f"duplicate case header at line {line_number}")
            cases[case_id] = {"horizon": int(fields[2]), "units": [], "floors": {}}
        elif kind == "UNIT" and len(fields) == 11:
            case_id = fields[1]
            if case_id not in cases:
                raise AssertionError(f"unit precedes case header at line {line_number}")
            cases[case_id]["units"].append(fields[2:])
        elif kind == "FLOOR" and len(fields) == 5:
            case_id = fields[1]
            if case_id not in cases:
                raise AssertionError(f"floor precedes case header at line {line_number}")
            key = (fields[2], fields[3])
            if key in cases[case_id]["floors"]:
                raise AssertionError(f"duplicate floor {key}")
            basis_points = int(fields[4])
            if not 0 <= basis_points <= 10_000:
                raise AssertionError(f"invalid basis points on line {line_number}")
            cases[case_id]["floors"][key] = basis_points
        else:
            raise AssertionError(f"unknown/malformed record at line {line_number}: {fields}")
    return cases


def add(acc: dict[str, int], key: str, quantity: int) -> None:
    value = acc[key] + quantity
    if value > (1 << 64) - 1:
        raise AssertionError(f"u64 overflow in {key}")
    acc[key] = value


def build_report(case_id: str, case: dict):
    horizon = case["horizon"]
    groups: dict[tuple[str, str], dict[str, int]] = defaultdict(
        lambda: {
            "scheduled": 0,
            "adjudicated": 0,
            "on_time": 0,
            "late": 0,
            "not_accepted": 0,
            "lost": 0,
            "unresolved": 0,
            "reconciliation_failure": 0,
        }
    )
    seen_units: set[str] = set()

    for row in case["units"]:
        unit_id, cohort, quantity_s, unit_name, deadline_s, outcome, time_s, verified_s, reason = row
        del reason
        quantity = int(quantity_s)
        deadline = int(deadline_s)
        if not unit_id or not cohort or not unit_name or quantity <= 0:
            raise AssertionError(f"{case_id}: invalid unit record")
        if unit_id in seen_units:
            raise AssertionError(f"{case_id}: duplicate unit identity {unit_id}")
        seen_units.add(unit_id)
        acc = groups[(cohort, unit_name)]
        add(acc, "scheduled", quantity)

        verified = verified_s == "true"
        time_value = None if time_s == "-" else int(time_s)

        if outcome == "ACCEPTED":
            if verified and time_value is not None:
                add(acc, "adjudicated", quantity)
                if time_value > horizon:
                    add(acc, "not_accepted", quantity)
                elif time_value <= deadline:
                    add(acc, "on_time", quantity)
                else:
                    add(acc, "late", quantity)
            else:
                add(acc, "unresolved", quantity)
        elif outcome == "NOT_ACCEPTED":
            if verified:
                add(acc, "adjudicated", quantity)
                add(acc, "not_accepted", quantity)
            else:
                add(acc, "unresolved", quantity)
        elif outcome == "TERMINAL_EXIT":
            if time_value is None or time_value > horizon:
                raise AssertionError(f"{case_id}: terminal exit after horizon")
            add(acc, "unresolved", quantity)
        elif outcome == "CENSORED":
            if time_value is None or time_value > horizon:
                raise AssertionError(f"{case_id}: censor timestamp after horizon")
            add(acc, "unresolved", quantity)
        elif outcome == "DISPUTED":
            add(acc, "unresolved", quantity)
        elif outcome == "LOST":
            if verified and time_value is not None:
                if time_value > horizon:
                    raise AssertionError(f"{case_id}: loss observed after horizon")
                add(acc, "adjudicated", quantity)
                add(acc, "lost", quantity)
            else:
                add(acc, "unresolved", quantity)
        elif outcome == "RECON_FAILURE":
            add(acc, "reconciliation_failure", quantity)
        else:
            raise AssertionError(f"{case_id}: unsupported outcome {outcome}")

    lines = []
    for (cohort, unit_name), acc in sorted(groups.items()):
        floor = case["floors"].get((cohort, unit_name))
        if floor is None:
            status = "NOT_CONFIGURED"
            floor_text = "-"
        elif acc["reconciliation_failure"] > 0:
            status = "INVALID_EVIDENCE"
            floor_text = str(floor)
        else:
            threshold = acc["scheduled"] * floor
            lower = acc["on_time"] * 10_000
            upper = (acc["on_time"] + acc["unresolved"]) * 10_000
            if lower >= threshold:
                status = "PASS"
            elif upper < threshold:
                status = "FAIL"
            else:
                status = "UNKNOWN"
            floor_text = str(floor)

        lines.append(
            "COHORT\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}".format(
                case_id,
                cohort,
                unit_name,
                acc["scheduled"],
                acc["adjudicated"],
                acc["on_time"],
                acc["late"],
                acc["not_accepted"],
                acc["lost"],
                acc["unresolved"],
                acc["reconciliation_failure"],
                acc["on_time"],
                acc["adjudicated"],
                acc["on_time"],
                acc["scheduled"],
                acc["on_time"] + acc["unresolved"],
                acc["scheduled"],
                floor_text,
                status,
            )
        )
    return len(seen_units), lines


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument(
        "--fixture",
        type=Path,
        default=Path("research/maritime-receiver-service/fixtures/outcomes.tsv"),
    )
    args = parser.parse_args()
    cases = parse_fixture(args.fixture)

    expected = [
        "RECEIVER_SERVICE_RECEIPT\t0.1",
        "CASE\tcase_id\tunit_count\tcohort_count",
    ]
    for case_id in sorted(cases):
        unit_count, cohorts = build_report(case_id, cases[case_id])
        expected.append(f"CASE\t{case_id}\t{unit_count}\t{len(cohorts)}")
        expected.extend(cohorts)
    expected.append(f"SUMMARY\tPASS\t{len(cases)}/{len(cases)}")

    actual = args.receipt.read_text(encoding="utf-8").splitlines()
    if actual != expected:
        raise AssertionError(f"receiver-service Rust/reference mismatch\nexpected={expected}\nactual={actual}")

    print(f"RECEIVER_SERVICE_REFERENCE\tPASS\t{len(cases)}/{len(cases)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
