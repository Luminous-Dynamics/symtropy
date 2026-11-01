#!/usr/bin/env python3
"""Independent arithmetic and canonical-digest oracle for the SU2 steady-history screen."""
from __future__ import annotations

import csv
import hashlib
import math
import struct
import sys
from pathlib import Path


EVIDENCE = (
    "declared_residual_encoding_terminal_stability_and_monitor_screen_only_"
    "not_complete_CFD_convergence_grid_independence_or_physical_validation"
)
SCHEMA = "su2-declared-convergence-screen/0.2.0"


def require(condition: bool, message: str) -> None:
    if not condition:
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


def close(actual: str, expected: float, label: str) -> None:
    require(actual != "", f"{label}: expected numeric output, got blank")
    require(math.isclose(float(actual), expected, rel_tol=1e-12, abs_tol=1e-15),
            f"{label}: {actual} != {expected:.17g}")


def mean(values: list[float]) -> float:
    divisor = float(len(values))
    return sum(value / divisor for value in values)


def slope_per_1000(iterations: list[int], values: list[float]) -> float:
    x0 = iterations[0]
    xs = [float(value - x0) for value in iterations]
    x_mean = mean(xs)
    y_mean = mean(values)
    variance = sum((value - x_mean) ** 2 for value in xs)
    covariance = sum((x - x_mean) * (y - y_mean) for x, y in zip(xs, values, strict=True))
    return abs(covariance / variance * 1000.0)


def main() -> int:
    if len(sys.argv) != 4:
        raise ValueError("usage: verify-su2-convergence-screen-independent.py RUN IMPORT_CSV SCREEN_CSV")
    run = Path(sys.argv[1])
    imported = read_rows(Path(sys.argv[2]))
    rows = read_rows(Path(sys.argv[3]))
    receipt_rows = read_rows(run / "runner_receipt.csv")
    history_rows = read_rows(run / "history.csv")
    require(len(receipt_rows) == 1, "expected one runner receipt")
    require(len(history_rows) == 8, "expected eight source history rows")
    require(len(imported) == 8, "expected eight imported history rows")
    require(len(rows) == 4, "expected two residual and two monitor checks")

    receipt = receipt_rows[0]
    require(receipt["solver_id"] == "SU2_CFD", "wrong solver ID")
    require(receipt["status"] == "process_exited_zero_expected_outputs_present", "solver process did not exit successfully")
    require(all(row["run_id"] == receipt["run_id"] for row in imported), "history import run ID mismatch")
    require(all(row["runner_receipt_sha256"] == receipt["receipt_sha256"] for row in imported),
            "history import receipt digest mismatch")

    source_header = ["Iteration", "CL", "CD", "RMS_DENSITY", "RMS_MOMENTUM-X"]
    require(list(history_rows[0].keys()) == source_header, "source history header mismatch")
    expected_import_columns = [
        "CL [dimensionless]", "CD [dimensionless]",
        "RMS_DENSITY [log10(residual)]", "RMS_MOMENTUM-X [log10(residual)]",
    ]
    for source, imported_row in zip(history_rows, imported, strict=True):
        require(int(imported_row["iteration"]) == int(source["Iteration"]), "iteration binding mismatch")
        for name in expected_import_columns:
            source_name = name.split(" [", 1)[0]
            close(imported_row[name], float(source[source_name]), f"import {source_name}")
    first_import = imported[0]
    history_sha256 = hashlib.sha256((run / "history.csv").read_bytes()).hexdigest()
    require(all(row["history_sha256"] == history_sha256 for row in imported), "history digest mismatch")

    expected_spec = [
        ("residual_log10_final_reduction_and_terminal_span", "RMS_DENSITY", "log10(residual)"),
        ("residual_log10_final_reduction_and_terminal_span", "RMS_MOMENTUM-X", "log10(residual)"),
        ("monitor_terminal_range_mean_drift_and_slope", "CL", "dimensionless"),
        ("monitor_terminal_range_mean_drift_and_slope", "CD", "dimensionless"),
    ]
    terminal = history_rows[-6:]
    residual_values = {
        name: [float(row[name]) for row in history_rows]
        for name in ("RMS_DENSITY", "RMS_MOMENTUM-X")
    }
    monitor_values = {
        name: [float(row[name]) for row in terminal]
        for name in ("CL", "CD")
    }
    expected_metrics: list[dict[str, float]] = []
    for name in ("RMS_DENSITY", "RMS_MOMENTUM-X"):
        values = residual_values[name]
        trailing = values[-6:]
        expected_metrics.append({
            "observed_initial_log10_residual": values[0],
            "observed_final_log10_residual": values[-1],
            "observed_log10_reduction": values[0] - values[-1],
            "observed_terminal_log10_span": max(trailing) - min(trailing),
            "maximum_final_log10_residual": -8.0,
            "minimum_log10_reduction": 6.0,
            "maximum_terminal_log10_span": 2.0,
        })
    iterations = [int(row["Iteration"]) for row in terminal]
    for name in ("CL", "CD"):
        values = monitor_values[name]
        early = mean(values[:3])
        late = mean(values[3:])
        expected_metrics.append({
            "observed_window_range": max(values) - min(values),
            "maximum_window_range": 0.0004,
            "observed_mean_drift": abs(late - early),
            "maximum_mean_drift": 0.0003,
            "observed_slope_per_1000_iterations": slope_per_1000(iterations, values),
            "maximum_slope_per_1000_iterations": 0.02,
        })

    numeric_fields = [
        "observed_initial_absolute",
        "observed_final_absolute",
        "observed_final_to_initial_ratio",
        "maximum_final_absolute",
        "maximum_final_to_initial_ratio",
        "observed_window_range",
        "maximum_window_range",
        "observed_initial_log10_residual",
        "observed_final_log10_residual",
        "observed_log10_reduction",
        "observed_terminal_log10_span",
        "maximum_final_log10_residual",
        "minimum_log10_reduction",
        "maximum_terminal_log10_span",
        "observed_mean_drift",
        "maximum_mean_drift",
        "observed_slope_per_1000_iterations",
        "maximum_slope_per_1000_iterations",
    ]
    for index, (row, (kind, column, unit), metrics) in enumerate(
        zip(rows, expected_spec, expected_metrics, strict=True)
    ):
        require(row["criterion_kind"] == kind, f"{column}: criterion kind mismatch")
        require(row["column"] == column, f"{column}: criterion column mismatch")
        require(row["unit"] == unit, f"{column}: unit mismatch")
        require(int(row["rows_used"]) == (8 if index < 2 else 6), f"{column}: rows_used mismatch")
        require(row["criterion_status"] == "passed", f"{column}: expected passing criterion")
        for name in numeric_fields:
            value = row[name]
            if name in metrics:
                close(value, metrics[name], f"{column} {name}")
            else:
                require(value == "", f"{column} {name} should be blank")
        require(row["overall_status"] == "declared_thresholds_met", "overall status mismatch")
        require(row["evidence_level"] == EVIDENCE, "evidence ceiling mismatch")
        require(row["run_id"] == receipt["run_id"], "screen run ID mismatch")
        require(row["solver_id"] == receipt["solver_id"], "screen solver ID mismatch")
        require(row["runner_receipt_sha256"] == receipt["receipt_sha256"], "screen receipt digest mismatch")
        require(row["history_sha256"] == history_sha256, "screen history digest mismatch")
        require(row["history_import_sha256"] == first_import["import_sha256"], "screen import digest mismatch")

    buffer = bytearray()
    for item in [
        SCHEMA,
        receipt["run_id"],
        receipt["solver_id"],
        receipt["receipt_sha256"],
        history_sha256,
        first_import["import_sha256"],
        "declared_thresholds_met",
        EVIDENCE,
    ]:
        add_string(buffer, item)
    buffer.extend(struct.pack("<Q", len(rows)))
    for row in rows:
        for field in ("criterion_kind", "column", "unit", "criterion_status"):
            add_string(buffer, row[field])
        buffer.extend(struct.pack("<Q", int(row["rows_used"])))
        for field in numeric_fields:
            add_optional(buffer, row[field])
    digest = hashlib.sha256(buffer).hexdigest()
    require(all(row["screen_sha256"] == digest for row in rows), "canonical screen digest mismatch")
    require(rows[0]["passed_count"] == "4" and rows[0]["failed_count"] == "0"
            and rows[0]["insufficient_count"] == "0", "criterion counts mismatch")
    print(
        "independent_su2_convergence_screen_oracle=pass "
        f"criteria={len(rows)} screen_sha256={digest} "
        "residual_encoding=log10 terminal_stability=pass monitor_drift_and_slope=pass"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"independent SU2 convergence screen oracle failed: {error}", file=sys.stderr)
        raise SystemExit(1)
