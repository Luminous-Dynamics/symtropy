#!/usr/bin/env python3
"""Independent stdlib replay of a BOCoDe per-trial receipt and retained artifact bytes."""
from __future__ import annotations

import csv
import hashlib
import os
from pathlib import Path
import re
import struct
import sys

SCHEMA = "bocode-trial-receipt/0.1.0"
HEADER = (
    "schema", "trial_id", "problem_id", "algorithm", "seed",
    "source_commit", "bocode_version", "python_version", "wrapper_script_sha256",
    "metadata_sha256", "dimension", "num_objectives", "num_constraints",
    "input_type", "requested_n_init", "requested_iters", "state",
    "objective_evaluation_count", "initial_design_count",
    "feasible_evaluation_count", "first_feasible_eval_index", "best_value",
    "archive_path", "archive_bytes", "archive_sha256",
    "summary_path", "summary_bytes", "summary_sha256",
    "stdout_path", "stdout_bytes", "stdout_sha256",
    "stderr_path", "stderr_bytes", "stderr_sha256",
    "metadata_path", "metadata_bytes", "metadata_file_sha256",
    "freeze_path", "freeze_bytes", "freeze_sha256",
    "environment_path", "environment_bytes", "environment_sha256",
    "command_path", "command_bytes", "command_sha256",
    "diagnostic_path", "diagnostic_bytes", "diagnostic_sha256",
    "elapsed_monotonic_ns", "child_user_cpu_ns", "child_system_cpu_ns",
    "process_exit_code", "skip_detected", "summary_row_present", "trace_valid",
    "diagnostic_code", "receipt_sha256",
)
ARTIFACTS = (
    ("archive", "archive_path", "archive_bytes", "archive_sha256"),
    ("summary", "summary_path", "summary_bytes", "summary_sha256"),
    ("stdout", "stdout_path", "stdout_bytes", "stdout_sha256"),
    ("stderr", "stderr_path", "stderr_bytes", "stderr_sha256"),
    ("metadata", "metadata_path", "metadata_bytes", "metadata_file_sha256"),
    ("freeze", "freeze_path", "freeze_bytes", "freeze_sha256"),
    ("environment", "environment_path", "environment_bytes", "environment_sha256"),
    ("command", "command_path", "command_bytes", "command_sha256"),
    ("diagnostic", "diagnostic_path", "diagnostic_bytes", "diagnostic_sha256"),
)
DIAGNOSTICS = {
    "none", "preflight_failed", "subprocess_error", "upstream_skip",
    "process_nonzero", "summary_row_mismatch", "missing_archive", "invalid_trace",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def append_string(buffer: bytearray, value: str) -> None:
    encoded = value.encode("utf-8")
    require(len(encoded) < 1 << 64, "canonical field is too large")
    buffer.extend(struct.pack("<Q", len(encoded)))
    buffer.extend(encoded)


def canonical_digest(row: dict[str, str]) -> str:
    data = bytearray()
    for name in HEADER[:-1]:
        append_string(data, name)
        append_string(data, row[name])
    return hashlib.sha256(data).hexdigest()


def bool_value(value: str, name: str) -> bool:
    require(value in ("true", "false"), f"{name} must be true or false")
    return value == "true"


def integer(value: str, name: str, optional: bool = False) -> int | None:
    if optional and value == "-":
        return None
    parsed = int(value)
    require(parsed >= 0, f"{name} must be non-negative")
    return parsed


def verify_artifacts(root: Path, row: dict[str, str]) -> int:
    root = root.resolve(strict=True)
    seen = set()
    verified = 0
    for role, path_key, size_key, digest_key in ARTIFACTS:
        relative = row[path_key]
        size = integer(row[size_key], size_key)
        digest = row[digest_key].lower()
        if relative == "-":
            require(size == 0 and digest == "-", f"absent {role} has non-empty size/digest")
            continue
        rel = Path(relative)
        require(not rel.is_absolute(), f"{role} path must be relative")
        require(all(part not in ("", ".", "..") for part in rel.parts),
                f"{role} path has traversal or empty components")
        require(relative not in seen, f"duplicate artifact path {relative}")
        seen.add(relative)
        candidate = (root / rel).resolve(strict=True)
        require(candidate.is_relative_to(root), f"{role} artifact resolves outside receipt root")
        require(candidate.is_file(), f"{role} artifact is not a regular file")
        data = candidate.read_bytes()
        require(len(data) == size, f"{role} artifact byte length mismatch")
        require(re.fullmatch(r"[0-9a-f]{64}", digest) is not None,
                f"{role} artifact digest is malformed")
        actual = hashlib.sha256(data).hexdigest()
        require(actual == digest, f"{role} artifact SHA-256 mismatch")
        verified += 1
    return verified


def verify(receipt_csv: Path, receipt_root: Path) -> dict[str, object]:
    with receipt_csv.open("r", encoding="utf-8", newline="") as stream:
        reader = csv.DictReader(stream)
        require(reader.fieldnames == list(HEADER), "receipt CSV header mismatch")
        rows = list(reader)
    require(len(rows) == 1, "receipt CSV must contain exactly one data row")
    row = rows[0]
    require(all(value is not None for value in row.values()), "receipt row contains missing fields")
    require(row["schema"] == SCHEMA, "unsupported receipt schema")
    require(row["trial_id"] == f'{row["problem_id"]}__{row["algorithm"]}__seed{row["seed"]}',
            "trial_id does not bind problem, algorithm and seed")
    require(re.fullmatch(r"[0-9a-fA-F]{40}", row["source_commit"]) is not None
            or (row["source_commit"] == "-" and row["state"] == "wrapper_error"),
            "source_commit must be a full SHA or a wrapper_error placeholder")
    for key in ("wrapper_script_sha256", "metadata_sha256"):
        require(re.fullmatch(r"[0-9a-fA-F]{64}", row[key]) is not None,
                f"{key} must be SHA-256")
    require(row["diagnostic_code"] in DIAGNOSTICS, "unknown diagnostic code")
    state = row["state"]
    require(state in ("succeeded", "failed", "invalid_trace", "wrapper_error"),
            "unknown terminal state")
    skip = bool_value(row["skip_detected"], "skip_detected")
    summary = bool_value(row["summary_row_present"], "summary_row_present")
    trace = bool_value(row["trace_valid"], "trace_valid")
    exit_code = integer(row["process_exit_code"], "process_exit_code", optional=True)
    evaluations = integer(row["objective_evaluation_count"], "objective_evaluation_count")
    initial = integer(row["initial_design_count"], "initial_design_count")
    feasible = integer(row["feasible_evaluation_count"], "feasible_evaluation_count")
    first_feasible = integer(row["first_feasible_eval_index"], "first_feasible_eval_index", optional=True)
    require(initial <= evaluations and feasible <= evaluations, "evaluation counters are inconsistent")
    require(
        (feasible == 0 and first_feasible is None)
        or (feasible > 0 and first_feasible is not None and first_feasible < evaluations),
        "first feasible index disagrees with feasible evaluation count",
    )
    require(integer(row["requested_iters"], "requested_iters") > 0,
            "requested_iters must be positive")
    if row["best_value"] != "-":
        require(float(row["best_value"]) == float(row["best_value"])
                and abs(float(row["best_value"])) != float("inf"),
                "best_value must be finite")
    if state == "succeeded":
        require(exit_code == 0 and not skip and summary and trace,
                "succeeded state violates process/summary/trace invariants")
        require(row["diagnostic_code"] == "none", "succeeded state must have diagnostic_code=none")
        require(row["best_value"] != "-" and evaluations > 0, "succeeded state lacks observed trace metrics")
        require(row["metadata_file_sha256"].lower() == row["metadata_sha256"].lower(),
                "succeeded state metadata digest differs from catalog")
    else:
        require(row["diagnostic_code"] != "none", "non-success requires a diagnostic code")
        if state == "failed":
            require(row["diagnostic_code"] in ("upstream_skip", "process_nonzero", "summary_row_mismatch"),
                    "failed state has incompatible diagnostic")
            if row["diagnostic_code"] == "upstream_skip":
                require(skip, "upstream_skip requires skip_detected=true")
            if row["diagnostic_code"] == "process_nonzero":
                require(exit_code not in (None, 0), "process_nonzero requires non-zero process exit")
        elif state == "invalid_trace":
            require(row["diagnostic_code"] in ("missing_archive", "invalid_trace") and not trace,
                    "invalid_trace state has incompatible diagnostic")
        elif state == "wrapper_error":
            require(row["diagnostic_code"] in ("preflight_failed", "subprocess_error"),
                    "wrapper_error has incompatible diagnostic")

    verified = verify_artifacts(receipt_root, row)
    artifacts_by_role = {role: row[path_key] for role, path_key, _, _ in ARTIFACTS}
    require(artifacts_by_role["stdout"] != "-" and artifacts_by_role["stderr"] != "-",
            "stdout/stderr must always be retained")
    require(artifacts_by_role["freeze"] != "-" and artifacts_by_role["environment"] != "-",
            "dependency freeze and safe environment projection must be retained")
    if state == "succeeded":
        require(artifacts_by_role["archive"] != "-" and artifacts_by_role["summary"] != "-",
                "succeeded state must retain NPZ and summary CSV")
        require(artifacts_by_role["metadata"] != "-" and artifacts_by_role["command"] != "-",
                "succeeded state must retain metadata and exact argv")
        require(artifacts_by_role["diagnostic"] == "-", "succeeded state must not carry a failure diagnostic")
    else:
        require(artifacts_by_role["diagnostic"] != "-", "non-success state must retain diagnostic text")

    # The companion JSON receipt is a canonical projection of the same row.
    json_path = receipt_root / "receipt.json"
    if json_path.exists():
        import json
        payload = json.loads(json_path.read_text(encoding="utf-8"))
        require(payload == row, "receipt.json does not match normalized receipt.csv")

    actual_receipt_sha = canonical_digest(row)
    require(actual_receipt_sha == row["receipt_sha256"].lower(), "canonical receipt digest mismatch")
    return {
        "trial_id": row["trial_id"],
        "state": state,
        "receipt_sha256": actual_receipt_sha,
        "verified_artifact_count": verified,
        "objective_evaluation_count": evaluations,
        "feasible_evaluation_count": feasible,
    }


def main() -> int:
    if len(sys.argv) != 3:
        raise ValueError("usage: verify-bocode-trial-receipt-independent.py <receipt.csv> <receipt-root>")
    result = verify(Path(sys.argv[1]), Path(sys.argv[2]))
    print(
        "independent_bocode_trial_receipt=pass "
        f"trial={result['trial_id']} state={result['state']} "
        f"artifacts={result['verified_artifact_count']} "
        f"evaluations={result['objective_evaluation_count']}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"independent BOCoDe trial receipt verification failed: {error}", file=sys.stderr)
        raise
