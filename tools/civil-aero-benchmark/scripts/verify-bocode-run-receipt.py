#!/usr/bin/env python3
"""Independently verify one BOCoDe per-trial receipt and all retained artifacts."""
from __future__ import annotations

import argparse
import ast
import csv
import hashlib
import json
import math
import re
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
    require(
        isinstance(shape, tuple) and all(isinstance(x, int) and x >= 0 for x in shape),
        "NumPy shape is malformed",
    )
    require(isinstance(descr, str) and descr and descr[1:2] != "O",
            "NumPy object arrays are prohibited")
    return shape, descr, end


def parse_npy_values(data: bytes) -> tuple[tuple[int, ...], list[Any]]:
    shape, descr, offset = parse_npy_header(data)
    payload = data[offset:]
    count = math.prod(shape) if shape else 1
    match = re.fullmatch(r"([<>=|])([fiuSbU])(\d+)", descr)
    require(match is not None, f"unsupported NPZ array dtype {descr!r}")
    endian, kind, width_text = match.groups()
    width = int(width_text)
    if kind == "S":
        item_size = width
        require(width > 0, "string array item width must be positive")
        require(len(payload) == count * item_size, "NumPy string payload size mismatch")
        values = [
            payload[index * item_size:(index + 1) * item_size].split(b"\x00", 1)[0].decode("utf-8")
            for index in range(count)
        ]
        return shape, values
    if kind == "U":
        item_size = width * 4
        require(width > 0, "Unicode array item width must be positive")
        require(len(payload) == count * item_size, "NumPy Unicode payload size mismatch")
        codec = "utf-32-be" if endian == ">" else "utf-32-le"
        values = [
            payload[index * item_size:(index + 1) * item_size].decode(codec).rstrip("\x00")
            for index in range(count)
        ]
        return shape, values
    if kind == "b":
        item_size = 1
        require(width == 1, "invalid NumPy boolean width")
    else:
        item_size = width
        require(width in (1, 2, 4, 8), f"unsupported numeric dtype width {width}")
    require(len(payload) == count * item_size, "NumPy numeric payload size mismatch")
    code = {
        "f": {4: "f", 8: "d"},
        "i": {1: "b", 2: "h", 4: "i", 8: "q"},
        "u": {1: "B", 2: "H", 4: "I", 8: "Q"},
    }.get(kind, {}).get(width)
    if kind == "b":
        values = [bool(value) for value in payload]
    else:
        require(code is not None, f"unsupported numeric dtype {descr!r}")
        prefix = ">" if endian == ">" else "<"
        values = [item[0] for item in struct.iter_unpack(prefix + code, payload)]
    return shape, values


def npy_scalar(arrays: dict[str, tuple[tuple[int, ...], list[Any]]], name: str) -> Any:
    require(name in arrays, f"NPZ is missing {name}")
    shape, values = arrays[name]
    require(shape in ((), (1,)) and len(values) == 1, f"NPZ {name} must be scalar")
    return values[0]


def finite_values(values: list[Any], name: str) -> list[float]:
    try:
        converted = [float(value) for value in values]
    except (TypeError, ValueError) as error:
        raise ValueError(f"NPZ {name} must be numeric") from error
    require(all(math.isfinite(value) for value in converted),
            f"NPZ {name} contains non-finite values")
    return converted


def verify_npz(path: Path, receipt: dict[str, Any]) -> dict[str, Any]:
    metadata = receipt.get("task_metadata") or {}
    dimension = int(metadata["dimension"])
    objectives = int(metadata["num_objectives"])
    constraints = int(metadata["num_constraints"])
    arrays: dict[str, tuple[tuple[int, ...], list[Any]]] = {}
    with zipfile.ZipFile(path, "r") as archive:
        infos = archive.infolist()
        names = [info.filename for info in infos]
        require(len(names) == len(set(names)), "NPZ contains duplicate ZIP member names")
        total_uncompressed = 0
        for info in infos:
            member = PurePosixPath(info.filename)
            require(
                not info.is_dir() and len(member.parts) == 1 and member.name.endswith(".npy"),
                f"NPZ contains unsafe/non-array member {info.filename!r}",
            )
            total_uncompressed += info.file_size
            require(total_uncompressed <= 512 * 1024 * 1024,
                    "NPZ uncompressed content exceeds the 512 MiB verifier cap")
        require(NPZ_REQUIRED.issubset(set(names)), "NPZ trace omits one or more required arrays")
        require(archive.testzip() is None, "NPZ ZIP CRC verification failed")
        for name in NPZ_REQUIRED:
            arrays[name[:-4]] = parse_npy_values(archive.read(name))

    algorithm = npy_scalar(arrays, "algorithm")
    problem = npy_scalar(arrays, "problem")
    seed = int(npy_scalar(arrays, "seed"))
    require(
        algorithm == receipt["algorithm"]
        and problem == receipt["task_id"]
        and seed == int(receipt["seed"]),
        "NPZ algorithm/problem/seed identity differs from receipt",
    )
    best = float(npy_scalar(arrays, "best"))
    n_init = int(npy_scalar(arrays, "n_init"))
    require(math.isfinite(best), "NPZ best must be finite")

    x_shape, x_values = arrays["X"]
    y_shape, y_values_raw = arrays["y"]
    c_shape, c_values_raw = arrays["c"]
    iterations_shape, iterations_raw = arrays["iterations"]
    history_shape, history_raw = arrays["per_iteration_value"]
    wall_shape, wall_raw = arrays["wall_time"]

    require(len(x_shape) == 2 and x_shape[0] > 0 and x_shape[1] == dimension,
            f"NPZ X shape must be (n>0,{dimension}), got {x_shape}")
    n = x_shape[0]
    require(len(y_shape) in (1, 2) and y_shape[0] == n,
            f"NPZ y must have {n} rows, got {y_shape}")
    y_width = 1 if len(y_shape) == 1 else y_shape[1]
    require(y_width == objectives, "NPZ y objective count differs from admitted metadata")
    require(len(c_shape) == 2 and c_shape[0] == n,
            f"NPZ c must have {n} rows, got {c_shape}")
    if constraints == 0:
        require(c_shape[1] == 0, "unconstrained task must have an empty c matrix")
        constraint_history_state = "not_applicable"
    elif c_shape[1] == 0:
        constraint_history_state = "unavailable"
    elif c_shape[1] == constraints:
        constraint_history_state = "available"
    else:
        raise ValueError(
            f"NPZ c must have zero columns (unavailable) or {constraints} raw constraint columns, got {c_shape}"
        )
    require(len(iterations_shape) == len(history_shape) == len(wall_shape) == 1,
            "NPZ iterations, best-history and wall-time arrays must be vectors")
    require(iterations_shape[0] > 0 and iterations_shape == history_shape == wall_shape,
            "NPZ iteration, best and wall traces have inconsistent lengths")

    x_values = finite_values(x_values, "X")
    y_values = finite_values(y_values_raw, "y")
    c_values = finite_values(c_values_raw, "c")
    iterations = finite_values(iterations_raw, "iterations")
    history = finite_values(history_raw, "per_iteration_value")
    wall = finite_values(wall_raw, "wall_time")
    require(min(x_values) >= -1.0e-8 and max(x_values) <= 1.0 + 1.0e-8,
            "NPZ X observations lie outside the normalized unit cube")
    require(0 <= n_init <= n, "NPZ n_init is outside the observed evaluation history")
    require(iterations == [float(index) for index in range(len(iterations))],
            "NPZ iteration indices must be exactly 0..n")
    require(abs(wall[0]) <= 1.0e-9 and all(
        wall[index] <= wall[index + 1] + 1.0e-12 for index in range(len(wall) - 1)
    ), "NPZ wall-time trace must start at zero and be monotonic")
    require(all(
        history[index] <= history[index + 1] + 1.0e-10
        for index in range(len(history) - 1)
    ), "NPZ best-so-far trace regresses under maximization")
    require(abs(history[-1] - best) <= 1.0e-10 * max(1.0, abs(best)),
            "NPZ final best-so-far value differs from its best scalar")

    if constraint_history_state == "available":
        feasible_count = 0
        for index in range(n):
            row = c_values[index * constraints:(index + 1) * constraints]
            feasible_count += int(all(value <= 0.0 for value in row))
        infeasible_count = n - feasible_count
        feasible_fraction = feasible_count / n
        constraint_observation_count = n
    else:
        feasible_count = None
        infeasible_count = None
        feasible_fraction = None
        constraint_observation_count = 0

    return {
        "npz_evaluation_count_from_shape": n,
        "npz_trace_point_count_from_shape": len(iterations),
        "npz_n_init": n_init,
        "npz_best": best,
        "constraint_history_state": constraint_history_state,
        "constraint_observation_count": constraint_observation_count,
        "feasible_evaluation_count": feasible_count,
        "infeasible_evaluation_count": infeasible_count,
        "feasible_evaluation_fraction": feasible_fraction,
    }


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
    stdout_bytes = None
    if "stdout" in artifacts:
        stdout_bytes = safe_artifact_path(root, artifacts["stdout"]["path"]).read_bytes()
    has_skip = stdout_bytes is not None and re.search(
        rb"^\s*SKIP\s+", stdout_bytes, flags=re.MULTILINE
    ) is not None
    if state == "succeeded":
        require(receipt.get("process_exit_code") == 0, "succeeded trial must have exit code 0")
        require(not has_skip, "succeeded trial stdout contains upstream SKIP")
        require("upstream_npz" in artifacts and "upstream_summary" in artifacts,
                "succeeded trial must retain raw NPZ and upstream summary")
        npz_path = safe_artifact_path(root, artifacts["upstream_npz"]["path"])
        npz_result = verify_npz(npz_path, receipt)
        require(metrics.get("objective_evaluation_count") == npz_result["npz_evaluation_count_from_shape"],
                "receipt objective count differs from raw NPZ X shape")
    elif state == "invalid_trace":
        require("upstream_npz" in artifacts and "upstream_summary" in artifacts,
                "invalid_trace must retain both NPZ and summary artifacts")
    elif state == "failed":
        require(bool(receipt.get("reason")), "failed trial requires a failure reason")
        require(
            receipt.get("process_exit_code") not in (None, 0)
            or has_skip
            or "upstream_npz" not in artifacts
            or "upstream_summary" not in artifacts,
            "failed state has zero exit, no SKIP and complete raw outputs",
        )
    elif state == "wrapper_error":
        require(bool(receipt.get("reason")), "wrapper_error requires a failure reason")

    row_path = root / "run-row.csv"
    require(row_path.is_file(), "normalized per-trial row is missing")
    with row_path.open("r", encoding="utf-8", newline="") as stream:
        rows = list(csv.DictReader(stream))
    require(len(rows) == 1, "run-row.csv must contain exactly one row")
    row = rows[0]
    metrics_expected = {
        "objective_evaluation_count": metrics.get("objective_evaluation_count", ""),
        "constraint_history_state": metrics.get("constraint_history_state", ""),
        "initial_design_count": metrics.get("initial_design_count", ""),
        "feasible_evaluation_count": metrics.get("feasible_evaluation_count", ""),
        "infeasible_evaluation_count": metrics.get("infeasible_evaluation_count", ""),
        "feasible_evaluation_fraction": metrics.get("feasible_evaluation_fraction", ""),
    }
    text_expected = {
        "schema": receipt["schema"],
        "attempt_id": receipt["attempt_id"],
        "run_id": run_id,
        "request_sha256": request_sha,
        "state": state,
        "reason": receipt.get("reason", ""),
        "source_commit": receipt.get("source_commit_verified") or receipt["source_commit"],
        "package_version": receipt.get("package_version_observed") or receipt["package_version_expected"],
        "task_id": receipt["task_id"],
        "metadata_sha256": receipt["metadata_sha256_expected"],
        "algorithm": receipt["algorithm"],
        "seed": str(receipt["seed"]),
        "n_init_requested": str(receipt["n_init_requested"]),
        "iterations_requested": str(receipt["iterations_requested"]),
        "process_exit_code": "" if receipt.get("process_exit_code") is None else str(receipt["process_exit_code"]),
        "wall_seconds": "" if receipt.get("wall_seconds") is None else str(receipt["wall_seconds"]),
        "cpu_user_seconds": "" if receipt.get("cpu_user_seconds_measured") is None else str(receipt["cpu_user_seconds_measured"]),
        "cpu_system_seconds": "" if receipt.get("cpu_system_seconds_measured") is None else str(receipt["cpu_system_seconds_measured"]),
        "peak_memory_state": "unknown" if receipt.get("peak_memory_bytes") is None else "measured",
        "gpu_state": "unknown" if receipt.get("gpu_seconds") is None else "measured",
        "energy_state": "unknown" if receipt.get("energy_joules") is None else "measured",
        "stdout_sha256": artifacts.get("stdout", {}).get("sha256", ""),
        "stderr_sha256": artifacts.get("stderr", {}).get("sha256", ""),
        "npz_sha256": artifacts.get("upstream_npz", {}).get("sha256", ""),
        "summary_sha256": artifacts.get("upstream_summary", {}).get("sha256", ""),
        "environment_freeze_sha256": receipt.get("environment_freeze_sha256") or "",
        "receipt_sha256": claimed_receipt_sha,
    }
    for name, expected in text_expected.items():
        require(row.get(name) == str(expected), f"normalized run row {name} differs from receipt")
    for name, expected in metrics_expected.items():
        actual = row.get(name, "")
        require(actual == str(expected), f"normalized run row {name} differs from receipt metrics")
    if state == "succeeded":
        require(
            int(row["objective_evaluation_count"]) == npz_result["npz_evaluation_count_from_shape"],
            "normalized row evaluation count differs from raw NPZ",
        )
        require(
            row["constraint_history_state"] == npz_result["constraint_history_state"],
            "normalized row constraint-history state differs from raw NPZ shape",
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
