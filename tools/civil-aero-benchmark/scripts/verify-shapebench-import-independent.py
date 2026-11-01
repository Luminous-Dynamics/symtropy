#!/usr/bin/env python3
"""Independent oracle for the synthetic ShapeBench CSV import contract."""
from __future__ import annotations

import csv
import hashlib
import struct
import sys
from pathlib import Path

SCHEMA = "shapebench-results-import/0.1.0"
EVIDENCE = "benchmark_output_import_only_not_solver_validation_or_physical_evidence"
TASK = "superwing-sw001-synthetic-fixture"
ENVIRONMENT = "SuperWing"
METHOD = "CMA-ES"
SOURCE_COMMIT = "0123456789abcdef0123456789abcdef01234567"
CONFIG_SHA256 = "1" * 64
RUN_MANIFEST_SHA256 = "2" * 64
EXPECTED_DIRECTION = "maximize"

def fail(message: str) -> None:
    raise SystemExit(f"shape_bench_independent_oracle=reject reason={message}")

def append_string(target: bytearray, value: str) -> None:
    encoded = value.encode("utf-8")
    target.extend(struct.pack("<Q", len(encoded)))
    target.extend(encoded)

def read_csv(path: Path) -> tuple[list[str], list[dict[str, str]]]:
    with path.open("r", newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        if reader.fieldnames is None:
            fail(f"{path} has no header")
        return reader.fieldnames, list(reader)

def main() -> None:
    if len(sys.argv) != 3:
        fail("usage: verify-shapebench-import-independent.py <source-results.csv> <imported.csv>")
    source_path, imported_path = map(Path, sys.argv[1:])
    source_header, source_rows = read_csv(source_path)
    expected_source_header = ["iteration", "design", "reward", "best_reward"]
    if source_header != expected_source_header or len(source_rows) != 3:
        fail("source fixture schema or row count mismatch")

    output_header, output_rows = read_csv(imported_path)
    expected_output_header = [
        "task_id", "environment_id", "method_id", "source_commit",
        "task_config_sha256", "run_manifest_sha256", "reward_direction",
        "iteration", "design_payload_sha256", "results_json_sha256",
        "cost_ledger_sha256", "reward", "best_reward", "import_sha256",
        "evidence_level",
    ]
    if output_header != expected_output_header or len(output_rows) != 3:
        fail("imported schema or row count mismatch")

    canonical = bytearray()
    for value in [
        SCHEMA, TASK, ENVIRONMENT, METHOD, SOURCE_COMMIT, CONFIG_SHA256,
        RUN_MANIFEST_SHA256, "", "", EXPECTED_DIRECTION,
    ]:
        append_string(canonical, value)
    canonical.extend(struct.pack("<Q", len(source_rows)))

    best: float | None = None
    for index, (source, imported) in enumerate(zip(source_rows, output_rows, strict=True)):
        try:
            iteration = int(source["iteration"])
            reward = float(source["reward"])
            best_reward = float(source["best_reward"])
            imported_iteration = int(imported["iteration"])
            imported_reward = float(imported["reward"])
            imported_best = float(imported["best_reward"])
        except (KeyError, ValueError) as error:
            fail(f"numeric field invalid at row {index}: {error}")
        if imported_iteration != iteration or imported_reward != reward or imported_best != best_reward:
            fail(f"imported numeric values differ from source at row {index}")
        if not all(map(lambda value: value == value and abs(value) != float("inf"),
                       [reward, best_reward, imported_reward, imported_best])):
            fail(f"non-finite reward at row {index}")
        best = reward if best is None else max(best, reward)
        if best_reward != best:
            fail(f"best_reward inconsistency at row {index}")
        design = source["design"]
        expected_design_hash = hashlib.sha256(design.encode("utf-8")).hexdigest()
        if imported["design_payload_sha256"] != expected_design_hash:
            fail(f"design payload digest mismatch at row {index}")
        if imported["task_id"] != TASK or imported["environment_id"] != ENVIRONMENT:
            fail(f"task/environment identity mismatch at row {index}")
        if imported["method_id"] != METHOD or imported["source_commit"] != SOURCE_COMMIT:
            fail(f"method/source identity mismatch at row {index}")
        if imported["task_config_sha256"] != CONFIG_SHA256:
            fail(f"task configuration digest mismatch at row {index}")
        if imported["run_manifest_sha256"] != RUN_MANIFEST_SHA256:
            fail(f"run manifest digest mismatch at row {index}")
        if imported["reward_direction"] != EXPECTED_DIRECTION:
            fail(f"reward direction mismatch at row {index}")
        if imported["results_json_sha256"] or imported["cost_ledger_sha256"]:
            fail("fixture must not invent companion JSON or cost-ledger hashes")
        if imported["evidence_level"] != EVIDENCE:
            fail("import evidence ceiling mismatch")

        canonical.extend(struct.pack("<Q", iteration))
        append_string(canonical, design)
        canonical.extend(struct.pack("<d", reward))
        canonical.extend(struct.pack("<d", best_reward))

    expected_digest = hashlib.sha256(canonical).hexdigest()
    observed_digests = {row["import_sha256"] for row in output_rows}
    if observed_digests != {expected_digest}:
        fail(f"canonical import digest mismatch expected={expected_digest} observed={observed_digests}")
    print(
        "shape_bench_independent_oracle=pass "
        f"rows={len(output_rows)} family=SuperWing method=CMA-ES "
        "cost_ledger_admitted=false surrogate_or_solver_validation_not_implied=true "
        f"import_sha256={expected_digest}"
    )

if __name__ == "__main__":
    main()
