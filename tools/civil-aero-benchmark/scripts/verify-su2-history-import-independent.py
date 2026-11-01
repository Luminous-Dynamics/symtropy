#!/usr/bin/env python3
"""Independent stdlib oracle for the receipt-bound SU2 history import fixture."""
from __future__ import annotations

import csv
import hashlib
import math
import re
import sys
from pathlib import Path


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def read_one_csv(path: Path) -> dict[str, str]:
    with path.open(newline="", encoding="utf-8") as stream:
        rows = list(csv.DictReader(stream))
    require(len(rows) == 1, f"{path.name}: expected exactly one receipt row")
    return rows[0]


def main() -> int:
    if len(sys.argv) != 3:
        raise ValueError("usage: verify-su2-history-import-independent.py RUN_DIRECTORY IMPORT_CSV")
    run_directory = Path(sys.argv[1])
    import_path = Path(sys.argv[2])
    history_path = run_directory / "history.csv"
    history_bytes = history_path.read_bytes()
    history_sha256 = hashlib.sha256(history_bytes).hexdigest()

    receipt = read_one_csv(run_directory / "runner_receipt.csv")
    require(receipt["solver_id"] == "SU2_CFD", "runner receipt solver id mismatch")
    require(
        receipt["status"] == "process_exited_zero_expected_outputs_present",
        "runner receipt process status is not successful",
    )
    require(receipt["receipt_sha256"], "runner receipt digest missing")
    require(re.fullmatch(r"[0-9a-f]{64}", receipt["executable_sha256"]) is not None,
            "executable digest missing or malformed")

    with (run_directory / "expected_outputs.csv").open(newline="", encoding="utf-8") as stream:
        output_rows = list(csv.DictReader(stream))
    output = [row for row in output_rows if row["relative_path"] == "history.csv"]
    require(len(output) == 1, "history output must have exactly one output-manifest record")
    require(output[0]["status"] == "present", "history output is not marked present")
    require(int(output[0]["byte_length"]) == len(history_bytes), "history byte length differs from output manifest")
    require(output[0]["sha256"] == history_sha256, "history digest differs from output manifest")

    with history_path.open(newline="", encoding="utf-8") as stream:
        history_reader = csv.DictReader(stream)
        require(history_reader.fieldnames == ["Iteration", "CL", "CD", "RMS_RES"], "unexpected fixture history header")
        history = list(history_reader)
    require(len(history) == 2, f"expected two history rows, got {len(history)}")

    with import_path.open(newline="", encoding="utf-8") as stream:
        imported_reader = csv.DictReader(stream)
        imported = list(imported_reader)
        header = imported_reader.fieldnames
    require(header is not None, "import output has no header")
    require(len(imported) == 2, f"expected two imported rows, got {len(imported)}")

    previous = -1
    for original, reported in zip(history, imported, strict=True):
        iteration = int(original["Iteration"])
        require(iteration > previous, "source iteration is not strictly increasing")
        previous = iteration
        require(int(reported["iteration"]) == iteration, "imported iteration differs from history")
        require(math.isclose(float(reported["CL [dimensionless]"]), float(original["CL"]), rel_tol=0.0, abs_tol=0.0),
                "imported CL differs from source history")
        require(math.isclose(float(reported["CD [dimensionless]"]), float(original["CD"]), rel_tol=0.0, abs_tol=0.0),
                "imported CD differs from source history")
        require(reported["run_id"] == receipt["run_id"], "import run ID differs from receipt")
        require(reported["executable_sha256"] == receipt["executable_sha256"],
                "import executable digest differs from receipt")
        require(reported["runner_receipt_sha256"] == receipt["receipt_sha256"],
                "import runner receipt digest differs from receipt")
        require(reported["history_byte_length"] == str(len(history_bytes)),
                "imported history byte length mismatch")
        require(reported["history_sha256"] == history_sha256, "imported history digest mismatch")
        require(re.fullmatch(r"[0-9a-f]{64}", reported["import_sha256"]) is not None,
                "import digest missing or malformed")
        require(
            reported["evidence_level"]
            == "receipt_bound_history_import_only_not_solver_convergence_or_physical_validation",
            "import evidence ceiling mismatch",
        )

    print(
        "independent_su2_history_import_oracle=pass "
        f"rows={len(imported)} history_sha256={history_sha256} "
        "receipt_binding=pass output_manifest_binding=pass"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"independent SU2 history import oracle failed: {error}", file=sys.stderr)
        raise SystemExit(1)
