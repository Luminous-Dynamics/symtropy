#!/usr/bin/env python3
"""Independently verify one BOCoDe per-trial receipt and all retained artifacts."""
from __future__ import annotations

import argparse
import ast
import csv
import hashlib
import json
import struct
import zipfile
from pathlib import Path, PurePosixPath
from typing import Any

SCHEMA = "bocode-single-trial-receipt/0.1.0"
STATES = {"succeeded", "failed", "invalid_trace", "wrapper_error"}
NPZ_REQUIRED = {
    "algorithm.npy", "problem.npy", "seed.npy", "best.npy", "n_init.npy",
    "iterations.npy", "per_iteration_value.npy", "wall_time.npy", "X.npy", "y.npy", "c.npy",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha256(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, ensure_ascii=False, separators=(",", ":"), allow_nan=False
    ).encode("utf-8")


def append_u64(buffer: bytearray, value: int) -> None:
    require(0 <= value < (1 << 64), "canonical length outside u64")
    buffer.extend(struct.pack("<Q", value))


def parse_npy_header(data: bytes) -> tuple[tuple[int, ...], str, int]:
    require(len(data) >= 10 and data[:6] == b"\x93NUMPY", "NPZ entry is not a NumPy array")
    major, minor = data[6], data[7]
    if major == 1:
        header_length = struct.unpack("<H", data[8:10])[0]
        start = 10
    elif major in (2, 3):
        require(len(data) >= 12, "truncated NumPy v2/v3 header")
        header_length = struct.unpack("<I", data[8:12])[0]
        start = 12
    else:
        raise ValueError(f"unsupported NumPy array format version {major}.{minor}")
    end = start + header_length
    require(end <= len(data), "truncated NumPy array header")
    encoding = "utf-8" if major == 3 else "latin1"
    header = ast.literal_eval(data[start:end].decode(encoding).strip())
    require(isinstance(header, dict), "NumPy header must be a dictionary")
    shape = header.get("shape")
    descr = header.get("descr")
    require(isinstance(shape, tuple) and all(isinstance(x, int) and x >= 0 for x in shape),
            "NumPy shape is malformed")
    require(isinstance(descr, str) and descr and descr[1:2] != "O",
            "NumPy object arrays are prohibited")
    return shape, descr, end


def verify_npz(path: Path, receipt: dict[str, Any]) -> dict[str, Any]:
    metadata = receipt.get("task_metadata") or {}
    dimension = int(metadata["dimension"])
    objectives = int(metadata["num_objectives"])
    constraints = int(metadata["num_constraints"])
    with zipfile.ZipFile(path, "r") as archive:
        names = set(archive.namelist())
        require(NPZ_REQUIRED.issubset(names), "NPZ trace omits one or more required arrays")
        require(archive.testzip() is None, "NPZ ZIP CRC verification failed")
        shapes: dict[str, tuple[int, ...]] = {}
        for name in NPZ_REQUIRED:
            shape, _descr, _offset = parse_npy_header(archive.read(name))
            shapes[name[:-4]] = shape

    x_shape = shapes["X"]
    y_shape = shapes["y"]
    c_shape = shapes["c"]
    require(len(x_shape) == 2 and x_shape[0] > 0 and x_shape[1] == dimension,
            f"NPZ X shape must be (n>0,{dimension}), got {x_shape}")
    n = x_shape[0]
    if len(y_shape) == 1:
        require(y_shape[0] == n, f"NPZ y vector must contain {n} rows, got {y_shape}")
        y_outputs = 1
    else:
        require(len(y_shape) == 2 and y_shape[0] == n,
                f"NPZ y shape must have {n} rows, got {y_shape}")
        y_outputs = y_shape[1]
    require(y_outputs == objectives, "NPZ y objective count differs from admitted metadata")
    require(len(c_shape) == 2 and c_shape == (n, constraints),
            f"NPZ c shape must be {(n, constraints)}, got {c_shape}")
    for name in ("iterations", "per_iteration_value", "wall_time"):
        require(len(shapes[name]) == 1, f"NPZ {name} must be one-dimensional")
    trace_lengths = [shapes[name][0] for name in ("iterations", "per_iteration_value", "wall_time")]
    require(len(set(trace_lengths)) == 1 and trace_lengths[0] > 0,
            "NPZ iteration, best and wall traces have inconsistent lengths")
    return {"npz_evaluation_count_from_shape": n, "npz_trace_point_count_from_shape": trace_lengths[0]}


def safe_artifact_path(root: Path, relative: str) -> Path:
    p = PurePosixPath(relative)
    require(not p.is_absolute() and ".." not in p.parts, "artifact path must be relative and traversal-free")
    target = (root / Path(*p.parts)).resolve()
    require(target.is_relative_to(root.resolve()), "artifact path escapes the receipt directory")
    return target


def verify_receipt(path: Path) -> dict[str, Any]:
    root = path.resolve().parent
    receipt = json.loads(path.read_text(encoding="utf-8"))
    require(receipt.get("schema") == SCHEMA, "receipt schema mismatch")
    require(receipt.get("state") in STATES, "unknown trial terminal state")
    require(bool(receipt.get("attempt_id")) and bool(receipt.get("run_id")), "attempt/run identity required")

    claimed_receipt_sha = receipt.get("receipt_sha256")
    require(isinstance(claimed_receipt_sha, str) and len(claimed_receipt_sha) == 64,
            "receipt SHA-256 missing or malformed")
    unsigned = dict(receipt)
    unsigned.pop("receipt_sha256", None)
    require(sha256(canonical_json(unsigned)) == claimed_receipt_sha, "receipt digest mismatch")

    request = {
        "schema": receipt["schema"],
        "attempt_id": receipt["attempt_id"],
        "source_commit": receipt["source_commit"],
        "package_version_expected": receipt["package_version_expected"],
        "task_id": receipt["task_id"],
        "metadata_sha256_expected": receipt["metadata_sha256_expected"],
        "algorithm": receipt["algorithm"],
        "seed": receipt["seed"],
        "n_init_requested": receipt["n_init_requested"],
        "iterations_requested": receipt["iterations_requested"],
        "environment_overrides": receipt["environment_overrides"],
    }
    request_sha = sha256(canonical_json(request))
    require(request_sha == receipt.get("request_sha256"), "request digest mismatch")
    run_id = sha256(canonical_json({
        "request": request_sha,
        "attempt_id": receipt["attempt_id"],
    }))
    require(run_id == receipt["run_id"], "run ID does not bind request and attempt identity")

    commit = receipt.get("source_commit_verified")
    require(commit is None or (isinstance(commit, str) and len(commit) == 40),
            "verified source commit is malformed")
    metadata_sha = receipt.get("metadata_sha256_expected")
    require(isinstance(metadata_sha, str) and len(metadata_sha) == 64,
            "task metadata digest is malformed")

    artifacts = receipt.get("artifacts")
    require(isinstance(artifacts, dict), "receipt artifacts must be an object")
    for name, item in artifacts.items():
        require(isinstance(item, dict), f"artifact {name} descriptor malformed")
        target = safe_artifact_path(root, str(item.get("path", "")))
        require(target.is_file(), f"artifact {name} is missing")
        data = target.read_bytes()
        require(len(data) == item.get("byte_length"), f"artifact {name} byte length mismatch")
        require(sha256(data) == item.get("sha256"), f"artifact {name} SHA-256 mismatch")

    state = receipt["state"]
    metrics = receipt.get("metrics") or {}
    npz_result = None
    if state == "succeeded":
        require(receipt.get("process_exit_code") == 0, "succeeded trial must have exit code 0")
        require("upstream_npz" in artifacts and "upstream_summary" in artifacts,
                "succeeded trial must retain raw NPZ and upstream summary")
        npz_path = safe_artifact_path(root, artifacts["upstream_npz"]["path"])
        npz_result = verify_npz(npz_path, receipt)
        require(metrics.get("objective_evaluation_count") == npz_result["npz_evaluation_count_from_shape"],
                "receipt objective count differs from raw NPZ X shape")
    elif state == "invalid_trace":
        require("upstream_npz" in artifacts, "invalid_trace must retain the NPZ that failed validation")
    elif state == "failed":
        require(bool(receipt.get("reason")), "failed trial requires a failure reason")
    elif state == "wrapper_error":
        require(bool(receipt.get("reason")), "wrapper_error requires a failure reason")

    row_path = root / "run-row.csv"
    require(row_path.is_file(), "normalized per-trial row is missing")
    with row_path.open("r", encoding="utf-8", newline="") as stream:
        rows = list(csv.DictReader(stream))
    require(len(rows) == 1, "run-row.csv must contain exactly one row")
    row = rows[0]
    require(row.get("run_id") == run_id, "normalized run row identity mismatch")
    require(row.get("state") == state, "normalized run row state mismatch")
    require(row.get("receipt_sha256") == claimed_receipt_sha,
            "normalized run row receipt digest mismatch")
    require(row.get("request_sha256") == request_sha,
            "normalized run row request digest mismatch")
    if state == "succeeded":
        require(
            int(row["objective_evaluation_count"]) == npz_result["npz_evaluation_count_from_shape"],
            "normalized row evaluation count differs from raw NPZ",
        )

    return {
        "run_id": run_id,
        "state": state,
        "receipt_sha256": claimed_receipt_sha,
        "artifact_count": len(artifacts),
        "npz_shape_replay": npz_result,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("receipt", type=Path)
    args = parser.parse_args()
    result = verify_receipt(args.receipt)
    print(json.dumps({"independent_bocode_receipt_verification": "pass", **result}, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        import sys
        print(f"independent BOCoDe receipt verification failed: {error}", file=sys.stderr)
        raise
