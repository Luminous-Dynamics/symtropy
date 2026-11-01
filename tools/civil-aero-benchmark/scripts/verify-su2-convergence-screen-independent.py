#!/usr/bin/env python3
"""Independent arithmetic and digest oracle for the declared SU2 history screen."""
from __future__ import annotations

import csv
import hashlib
import math
import struct
import sys
from pathlib import Path


def require(value: bool, message: str) -> None:
    if not value:
        raise ValueError(message)


def read_rows(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as stream:
        return list(csv.DictReader(stream))


def add_string(buffer: bytearray, value: str) -> None:
    raw = value.encode("utf-8")
    buffer.extend(struct.pack("<Q", len(raw)))
    buffer.extend(raw)


def add_optional(buffer: bytearray, value: str) -> None:
    if value == "":
        buffer.append(0)
    else:
        buffer.append(1)
        buffer.extend(struct.pack("<d", float(value)))


def main() -> int:
    if len(sys.argv) != 4:
        raise ValueError("usage: verify-su2-convergence-screen-independent.py RUN IMPORT_CSV SCREEN_CSV")
    run = Path(sys.argv[1])
    imported = read_rows(Path(sys.argv[2]))
    rows = read_rows(Path(sys.argv[3]))
    receipt = read_rows(run / "runner_receipt.csv")
    history = read_rows(run / "history.csv")

    require(len(receipt) == 1, "expected one runner receipt")
    require(len(imported) == 2, "expected two receipt-bound history points")
    require(len(rows) == 3, "expected one residual and two monitor criteria")
    r = receipt[0]
    first_import = imported[0]
    require(r["solver_id"] == "SU2_CFD", "wrong solver identity")
    require(first_import["runner_receipt_sha256"] == r["receipt_sha256"], "history import receipt mismatch")

    expected = [
        ("residual_final_absolute_and_reduction_ratio", "RMS_RES", "1", 2,
         0.01, 0.0001, 0.01, 0.0002, 0.02, None, None),
        ("trailing_window_absolute_range", "CL", "dimensionless", 2,
         None, None, None, None, None, 0.02, 0.025),
        ("trailing_window_absolute_range", "CD", "dimensionless", 2,
         None, None, None, None, None, 0.002, 0.0025),
    ]

    for row, spec in zip(rows, expected, strict=True):
        kind, column, unit, count, initial, final, ratio, max_final, max_ratio, window, max_window = spec
        require(row["criterion_kind"] == kind, f"{column}: criterion kind mismatch")
        require(row["column"] == column, f"{column}: column mismatch")
        require(row["unit"] == unit, f"{column}: unit mismatch")
        require(int(row["rows_used"]) == count, f"{column}: row count mismatch")
        require(row["criterion_status"] == "passed", f"{column}: criterion should pass")
        for field, target in [
            ("observed_initial_absolute", initial),
            ("observed_final_absolute", final),
            ("observed_final_to_initial_ratio", ratio),
            ("maximum_final_absolute", max_final),
            ("maximum_final_to_initial_ratio", max_ratio),
            ("observed_window_range", window),
            ("maximum_window_range", max_window),
        ]:
            value = row[field]
            if target is None:
                require(value == "", f"{column}: {field} should be empty")
            else:
                require(math.isclose(float(value), target, rel_tol=1e-12, abs_tol=1e-14),
                        f"{column}: {field} mismatch")
        require(row["overall_status"] == "declared_thresholds_met", "overall status mismatch")
        require(row["evidence_level"] ==
                "declared_residual_and_monitor_threshold_screen_only_not_complete_CFD_convergence_grid_independence_or_physical_validation",
                "convergence screen overstates evidence")

    require(history[0]["RMS_RES"] == "0.01" and history[-1]["RMS_RES"] == "0.0001",
            "residual values do not match the independent fixture")
    require(history[0]["CL"] == "0.40" and history[-1]["CL"] == "0.42",
            "CL monitor values do not match the independent fixture")
    require(history[0]["CD"] == "0.020" and history[-1]["CD"] == "0.018",
            "CD monitor values do not match the independent fixture")
    require(all(row["run_id"] == r["run_id"] for row in rows), "run ID not bound")
    require(all(row["runner_receipt_sha256"] == r["receipt_sha256"] for row in rows),
            "runner receipt digest not bound")
    require(all(row["history_sha256"] == first_import["history_sha256"] for row in rows),
            "history digest not bound")
    require(all(row["history_import_sha256"] == first_import["import_sha256"] for row in rows),
            "history import digest not bound")
    require(all(len(row["screen_sha256"]) == 64 for row in rows), "screen digest malformed")

    buffer = bytearray()
    for item in [
        "su2-declared-convergence-screen/0.1.0",
        r["run_id"],
        r["receipt_sha256"],
        first_import["history_sha256"],
        first_import["import_sha256"],
        "declared_thresholds_met",
        "declared_residual_and_monitor_threshold_screen_only_not_complete_CFD_convergence_grid_independence_or_physical_validation",
    ]:
        add_string(buffer, item)
    buffer.extend(struct.pack("<Q", len(rows)))
    optional_fields = [
        "observed_initial_absolute",
        "observed_final_absolute",
        "observed_final_to_initial_ratio",
        "maximum_final_absolute",
        "maximum_final_to_initial_ratio",
        "observed_window_range",
        "maximum_window_range",
    ]
    for row in rows:
        for field in ["criterion_kind", "column", "unit", "criterion_status"]:
            add_string(buffer, row[field])
        buffer.extend(struct.pack("<Q", int(row["rows_used"])))
        for field in optional_fields:
            add_optional(buffer, row[field])
    digest = hashlib.sha256(buffer).hexdigest()
    require(all(row["screen_sha256"] == digest for row in rows), "canonical screen digest mismatch")
    require(rows[0]["passed_count"] == "3" and rows[0]["failed_count"] == "0"
            and rows[0]["insufficient_count"] == "0", "criterion counts mismatch")
    print(f"independent_su2_convergence_screen_oracle=pass criteria=3 screen_sha256={digest} thresholds=caller_declared")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError, IndexError) as error:
        print(f"independent_su2_convergence_screen_oracle=fail reason={error}", file=sys.stderr)
        raise SystemExit(1)
