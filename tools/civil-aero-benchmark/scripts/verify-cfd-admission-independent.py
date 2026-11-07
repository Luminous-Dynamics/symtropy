#!/usr/bin/env python3
"""Independent stdlib oracle for the synthetic CFD admission/resource receipt."""
from __future__ import annotations

import csv
import math
import sys
from pathlib import Path

H = [4.0, 2.0, 1.0, 0.5, 0.25, 0.125]
VALUES = [2.0 + 0.8 * h**2 for h in H]
CPU = [0.25 * n for n in range(1, 7)]
GPU = [0.5 if index % 2 == 0 else None for index in range(6)]
WALL = [120.0 * n for n in range(1, 7)]
MEMORY = [1024 * n for n in range(1, 7)]
ENERGY = [1000.0 * n if n <= 4 else None for n in range(1, 7)]
SHA = lambda character: character * 64


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def number(row: dict[str, str], key: str, expected: float, tolerance: float = 1e-10) -> None:
    actual = float(row[key])
    require(
        math.isclose(actual, expected, rel_tol=tolerance, abs_tol=1e-12),
        f"{key}: {actual} != {expected}",
    )


def optional_number(row: dict[str, str], key: str, expected: float | None) -> None:
    actual = row[key]
    if expected is None:
        require(actual == "", f"{key} must remain blank when unmeasured; got {actual!r}")
    else:
        number(row, key, expected)


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: verify-cfd-admission-independent.py receipt.csv", file=sys.stderr)
        return 2
    with Path(sys.argv[1]).open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        rows = list(reader)
        headers = set(reader.fieldnames or [])

    required = {
        "status", "grid_family_kind", "solver_build_sha256", "physics_config_sha256", "geometry_sha256",
        "study_total_cpu_core_hours", "study_total_gpu_hours_measured",
        "gpu_measured_run_count", "study_total_wall_clock_seconds",
        "study_peak_memory_bytes", "study_total_energy_joules_measured",
        "energy_measured_run_count",
    }
    for prefix in ("coarse", "medium", "fine"):
        required.update({
            f"{prefix}_run_id", f"{prefix}_grid_id", f"{prefix}_grid_level",
            f"{prefix}_spacing", f"{prefix}_mesh_sha256", f"{prefix}_mesh_node_count",
            f"{prefix}_solver_exit_code", f"{prefix}_iterations_completed",
            f"{prefix}_cpu_core_hours", f"{prefix}_gpu_hours",
            f"{prefix}_wall_clock_seconds", f"{prefix}_peak_memory_bytes",
            f"{prefix}_energy_consumed_joules", f"{prefix}_quantity_value",
            f"{prefix}_residual_final", f"{prefix}_residual_reduction_ratio",
            f"{prefix}_quantity_history_drift_absolute",
            f"{prefix}_mass_imbalance_fraction", f"{prefix}_energy_imbalance_fraction",
        })
    require(required <= headers, f"missing receipt columns: {sorted(required - headers)}")
    require(len(rows) == 4, f"expected four sliding three-grid windows, found {len(rows)}")

    expected_status = (
        "numerically_admitted_under_declared_criteria_"
        "caller_hashes_unverified_not_experimentally_validated"
    )
    common_hashes = {
        "solver_build_sha256": SHA("a"),
        "physics_config_sha256": SHA("b"),
        "geometry_sha256": SHA("c"),
    }

    for row_index, row in enumerate(rows):
        require(row["status"] == expected_status, "evidence status/ceiling mismatch")
        require(row["grid_family_kind"] == "structured", "grid-family type mismatch")
        number(row, "safety_factor", 1.25)
        for field, expected in common_hashes.items():
            require(row[field] == expected, f"common {field} changed")

        for offset, prefix in enumerate(("coarse", "medium", "fine")):
            grid_index = row_index + offset
            n = grid_index + 1
            number(row, f"{prefix}_spacing", H[grid_index])
            require(row[f"{prefix}_run_id"] == f"run-{n}", f"{prefix} run ID mismatch")
            require(row[f"{prefix}_grid_id"] == f"grid-{n}", f"{prefix} grid ID mismatch")
            require(int(row[f"{prefix}_grid_level"]) == n, f"{prefix} grid level mismatch")
            require(row[f"{prefix}_mesh_sha256"] == SHA(str(n)), f"{prefix} mesh digest mismatch")
            require(int(row[f"{prefix}_mesh_node_count"]) == 10_000 * n,
                    f"{prefix} mesh node count mismatch")
            require(int(row[f"{prefix}_solver_exit_code"]) == 0, f"{prefix} process did not succeed")
            require(int(row[f"{prefix}_iterations_completed"]) == 5000 + grid_index,
                    f"{prefix} iteration count mismatch")
            number(row, f"{prefix}_cpu_core_hours", CPU[grid_index])
            optional_number(row, f"{prefix}_gpu_hours", GPU[grid_index])
            number(row, f"{prefix}_wall_clock_seconds", WALL[grid_index])
            require(int(row[f"{prefix}_peak_memory_bytes"]) == MEMORY[grid_index],
                    f"{prefix} peak memory mismatch")
            optional_number(row, f"{prefix}_energy_consumed_joules", ENERGY[grid_index])
            number(row, f"{prefix}_quantity_value", VALUES[grid_index])
            number(row, f"{prefix}_residual_final", 1.0e-10)
            number(row, f"{prefix}_residual_reduction_ratio", 1.0e-9)
            number(row, f"{prefix}_quantity_history_drift_absolute", 0.0)
            number(row, f"{prefix}_mass_imbalance_fraction", 1.0e-8)
            number(row, f"{prefix}_energy_imbalance_fraction", 2.0e-8)

        h0, h1, h2 = H[row_index:row_index + 3]
        p0, p1, p2 = VALUES[row_index:row_index + 3]
        r_cm, r_mf = h0 / h1, h1 / h2
        order = 2.0
        gci_medium = 1.25 * abs(p0 - p1) / (r_cm**order - 1.0)
        gci_fine = 1.25 * abs(p1 - p2) / (r_mf**order - 1.0)
        expected_gci = {
            "refinement_ratio_coarse_medium": r_cm,
            "refinement_ratio_medium_fine": r_mf,
            "observed_order": order,
            "extrapolated_value": 2.0,
            "medium_gci_absolute": gci_medium,
            "fine_gci_absolute": gci_fine,
            "asymptotic_ratio": 1.0,
        }
        for field, value in expected_gci.items():
            number(row, field, value)
        number(row, "medium_gci_relative_percent", 100.0 * gci_medium / abs(p1))
        number(row, "fine_gci_relative_percent", 100.0 * gci_fine / abs(p2))

        number(row, "study_total_cpu_core_hours", 5.25)
        optional_number(row, "study_total_gpu_hours_measured", 1.5)
        require(int(row["gpu_measured_run_count"]) == 3, "GPU measured-run count mismatch")
        number(row, "study_total_wall_clock_seconds", 2520.0)
        require(int(row["study_peak_memory_bytes"]) == 6144, "study peak memory mismatch")
        optional_number(row, "study_total_energy_joules_measured", 10_000.0)
        require(int(row["energy_measured_run_count"]) == 4, "energy measured-run count mismatch")

    print(
        "independent_cfd_admission_oracle=pass windows=4 grids=6 "
        "cpu_core_hours=5.25 gpu_hours=1.5 measured_gpu_runs=3 "
        "wall_seconds=2520 peak_memory_bytes=6144 energy_joules=10000"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError) as error:
        print(f"independent_cfd_admission_oracle=fail reason={error}", file=sys.stderr)
        raise SystemExit(1)
