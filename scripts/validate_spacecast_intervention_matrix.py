#!/usr/bin/env python3
"""CI-only source audit for the metadata-only SpaceCast intervention matrix.

This is not a production evaluator or spatial inference implementation.
It intentionally uses only the Python standard library.
"""
from __future__ import annotations

import csv
import sys
from collections import defaultdict
from pathlib import Path

EXPECTED_HEADER = [
    "case_id",
    "pair_id",
    "role",
    "level",
    "task_lane",
    "intervention",
    "relation_family",
    "reference_frame",
    "evidence_profile",
    "expected_behavior",
    "mutation_description",
]
EXPECTED_ROWS = 24
EXPECTED_PAIRS = 12
EXPECTED_ROLES = {"control", "mutated"}
VALID_LEVELS = {"L1", "L2", "L3"}
VALID_LANES = {"known_intervention", "observed_intervention"}
PAIR_SCOPE_FIELDS = (
    "level",
    "task_lane",
    "intervention",
    "relation_family",
    "reference_frame",
)
EVIDENCE_PROFILE_MUTATION_PAIRS = {"SPI001-009", "SPI001-011"}


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    path = Path("fixtures/spacecast-intervention-001/spacecast-intervention-matrix-v0.tsv")
    try:
        with path.open("r", encoding="utf-8", newline="") as handle:
            rows = list(csv.DictReader(handle, delimiter="\t"))
    except (OSError, UnicodeError, csv.Error) as exc:
        fail(f"cannot read TSV: {exc}")

    if not rows:
        fail("matrix has no data rows")
    if list(rows[0].keys()) != EXPECTED_HEADER:
        fail(f"header mismatch: {list(rows[0].keys())!r}")
    if len(rows) != EXPECTED_ROWS:
        fail(f"expected {EXPECTED_ROWS} data rows, found {len(rows)}")

    case_ids: set[str] = set()
    grouped: dict[str, list[dict[str, str]]] = defaultdict(list)
    levels: set[str] = set()
    lanes: set[str] = set()

    for line_number, row in enumerate(rows, start=2):
        if None in row or any(value is None or not value.strip() for value in row.values()):
            fail(f"blank or extra TSV field at line {line_number}")
        case_id = row["case_id"]
        pair_id = row["pair_id"]
        if case_id in case_ids:
            fail(f"duplicate case_id {case_id!r}")
        case_ids.add(case_id)
        if row["role"] not in EXPECTED_ROLES:
            fail(f"invalid role at line {line_number}: {row['role']!r}")
        if row["level"] not in VALID_LEVELS:
            fail(f"invalid capability level at line {line_number}: {row['level']!r}")
        if row["task_lane"] not in VALID_LANES:
            fail(f"invalid task lane at line {line_number}: {row['task_lane']!r}")
        if row["mutation_description"] == "None; known-transform control." and row["role"] != "control":
            fail("known-transform control label appears on a mutated row")
        grouped[pair_id].append(row)
        levels.add(row["level"])
        lanes.add(row["task_lane"])

    if len(grouped) != EXPECTED_PAIRS:
        fail(f"expected {EXPECTED_PAIRS} unique pair IDs, found {len(grouped)}")
    if levels != VALID_LEVELS:
        fail(f"expected all L1/L2/L3 levels, found {sorted(levels)}")
    if lanes != VALID_LANES:
        fail(f"expected both task lanes, found {sorted(lanes)}")

    for pair_id, pair_rows in sorted(grouped.items()):
        if len(pair_rows) != 2:
            fail(f"{pair_id}: expected exactly two rows, found {len(pair_rows)}")
        roles = {row["role"] for row in pair_rows}
        if roles != EXPECTED_ROLES:
            fail(f"{pair_id}: expected one control and one mutated row, found {sorted(roles)}")
        by_role = {row["role"]: row for row in pair_rows}
        control = by_role["control"]
        mutated = by_role["mutated"]
        for field in PAIR_SCOPE_FIELDS:
            if control[field] != mutated[field]:
                fail(f"{pair_id}: paired task-scope mismatch for {field}")
        evidence_profiles_differ = control["evidence_profile"] != mutated["evidence_profile"]
        if (pair_id in EVIDENCE_PROFILE_MUTATION_PAIRS) != evidence_profiles_differ:
            fail(f"{pair_id}: evidence-profile mutation does not match the preregistered pair design")

    print(
        "PASS: metadata matrix source integrity only — "
        f"{len(rows)} rows, {len(case_ids)} unique case IDs, "
        f"{len(grouped)} complete control/mutation pairs, "
        f"levels={','.join(sorted(levels))}, lanes={','.join(sorted(lanes))}. "
        "No renderer, replay, custody, candidate model or spatial evaluator was run."
    )


if __name__ == "__main__":
    main()
