#!/usr/bin/env python3
"""Independent stdlib oracle for DPW-8 V1a protocol-conformance output."""
from __future__ import annotations

import csv
import sys
from pathlib import Path

EXPECTED = {
    "mach": ("conforms", "0.73 1"),
    "chord_reynolds_number": ("conforms", "3000000 1"),
    "static_temperature_kelvin": ("conforms", "271 K"),
    "angle_of_attack_deg": ("conforms", "1.36;1.50;2.50;3.00;3.10"),
    "sidewall_boundary_condition": ("deviates", "symmetric"),
    "grid_level_count": ("meets_minimum_below_desirable", "4"),
    "reproduction_artifact_identity": ("unknown", "unknown"),
}
OVERALL = "deviations_detected_and_incomplete"


def main() -> int:
    if len(sys.argv) != 2:
        print(
            "usage: verify-dpw8-protocol-conformance-independent.py report.csv",
            file=sys.stderr,
        )
        return 2

    with Path(sys.argv[1]).open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))

    if len(rows) != len(EXPECTED):
        raise ValueError(f"expected {len(EXPECTED)} findings, found {len(rows)}")
    by_key = {row["key"]: row for row in rows}
    if set(by_key) != set(EXPECTED):
        raise ValueError(f"unexpected finding keys: {sorted(by_key)}")

    for key, (status, observed) in EXPECTED.items():
        row = by_key[key]
        if row["status"] != status:
            raise ValueError(f"{key}: expected status {status}, found {row['status']}")
        if row["observed"] != observed:
            raise ValueError(f"{key}: unexpected observed value {row['observed']!r}")
        if row["overall_status"] != OVERALL:
            raise ValueError(f"{key}: aggregate status mismatch")
        if row["protocol_id"] != "AIAA-DPW8-V1A-ONERA-OAT15A":
            raise ValueError(f"{key}: protocol identity mismatch")
        if row["task_id"] != "DPW8-V1a-ONERA-OAT15A":
            raise ValueError(f"{key}: task identity mismatch")
        if row["evidence_level"] != (
            "caller_supplied_metadata_not_source_authenticated_not_physical_validation"
        ):
            raise ValueError(f"{key}: evidence ceiling changed")
        if key != "reproduction_artifact_identity" and not row["source_reference"].startswith(
            "https://github.com/Drag-Prediction-Workshop/DPW8-Buffet/blob/"
        ):
            raise ValueError(f"{key}: source reference missing")
        if key == "sidewall_boundary_condition":
            if row["expected"] != "periodic":
                raise ValueError("sidewall requirement no longer reflects the V1a protocol")
            if "no physical equivalence" not in row["note"]:
                raise ValueError("sidewall difference must not be silently treated as equivalent")
        if key == "grid_level_count" and row["expected"] != "minimum=4; desirable=6":
            raise ValueError("grid minimum/desirable distinction was lost")
        if key == "reproduction_artifact_identity" and row["source_reference"]:
            raise ValueError("unknown artifact identity must not acquire a fabricated source")

    print(
        "independent_dpw8_protocol_conformance_oracle=pass "
        "findings=7 deviations=1 minimum_below_desirable=1 unknown=1"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, KeyError, ValueError) as error:
        print(
            f"independent_dpw8_protocol_conformance_oracle=fail reason={error}",
            file=sys.stderr,
        )
        raise SystemExit(1)
