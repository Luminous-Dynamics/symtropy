#!/usr/bin/env python3
"""Run exactly one BOCoDe optimizer trial and emit an evidence-qualified receipt.

This script is designed to be staged and invoked by Symtropy's generic Rust
external-solver runner. It is not a BOCoDe optimizer and does not claim that
the objective implementation or physical model is scientifically valid.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import importlib.metadata
import importlib.resources
import io
import json
import math
import os
from pathlib import Path
import re
import resource
import shlex
import struct
import subprocess
import sys
import time
import traceback
import zipfile
from typing import Any

TRIAL_SCHEMA = "civil-bocode-trial-receipt/0.1.0"
TRACE_SCHEMA = "civil-bocode-trace-audit/0.1.0"
EVIDENCE_LEVEL = (
    "pinned_external_optimizer_process_and_trace_integrity_only_objective_correctness_"
    "convergence_and_scientific_validity_unverified"
)
RECEIPT_HEADER = (
    "schema", "problem_id", "algorithm", "seed", "source_commit", "bocode_version",
    "metadata_sha256", "git_clean", "return_code", "skip_detected", "trial_state",
    "diagnostic_code", "diagnostic_message", "stdout_sha256", "stderr_sha256",
    "pip_freeze_sha256", "elapsed_milliseconds", "raw_trace_sha256",
    "trace_audit_sha256", "receipt_sha256", "evidence_level",
)
TRACE_HEADER = (
    "schema", "problem_id", "algorithm", "seed", "dimension", "num_objectives",
    "num_constraints", "input_type", "raw_trace_path", "raw_trace_byte_length",
    "raw_trace_sha256", "summary_state", "summary_byte_length", "summary_sha256",
    "objective_evaluation_count", "x_rows", "y_rows", "c_rows", "n_init_recorded",
    "feasibility_evidence_status", "feasible_observations", "infeasible_observations",
    "best", "history_rows", "identity_status", "trace_validation_status", "evidence_level",
)
SUMMARY_HEADER = ("algorithm", "problem", "seed", "best", "gap", "npz")
SAFE_NAME = re.compile(r"^[A-Za-z0-9_.-]+$")
HEX = set("0123456789abcdef")


class TrialError(Exception):
    def __init__(self, code: str, message: str, state: str = "failed") -> None:
        super().__init__(message)
        self.code = code
        self.state = state


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False
    ).encode("utf-8")


def csv_bytes(header: tuple[str, ...], values: dict[str, Any]) -> bytes:
    output = io.StringIO(newline="")
    writer = csv.writer(output, lineterminator="\n")
    writer.writerow(header)
    writer.writerow([values.get(name, "") for name in header])
    return output.getvalue().encode("utf-8")


def valid_sha(value: str) -> bool:
    return len(value) == 64 and all(ch.lower() in HEX for ch in value)


def safe_name(value: str, field: str) -> None:
    if not SAFE_NAME.fullmatch(value) or value in (".", ".."):
        raise TrialError("invalid_argument", f"{field} must be a simple filename-safe identifier", "wrapper_error")


def read_npy(blob: bytes) -> tuple[str, tuple[int, ...], list[Any]]:
    """Read a safe, non-object NPY array using only the Python standard library."""
    if len(blob) < 10 or blob[:6] != b"\x93NUMPY":
        raise TrialError("invalid_npy", "NPZ member is not a valid NPY array", "invalid_trace")
    major, minor = blob[6], blob[7]
    if major == 1:
        header_size = struct.unpack("<H", blob[8:10])[0]
        start = 10
        encoding = "latin1"
    elif major in (2, 3):
        if len(blob) < 12:
            raise TrialError("invalid_npy", "NPY v2/v3 header is truncated", "invalid_trace")
        header_size = struct.unpack("<I", blob[8:12])[0]
        start = 12
        encoding = "utf-8" if major == 3 else "latin1"
    else:
        raise TrialError("unsupported_npy_version", f"unsupported NPY version {major}.{minor}", "invalid_trace")
    stop = start + header_size
    if stop > len(blob):
        raise TrialError("invalid_npy", "NPY header extends beyond member bytes", "invalid_trace")
    try:
        header = json.loads("{}") if False else __import__("ast").literal_eval(blob[start:stop].decode(encoding).strip())
    except Exception as error:
        raise TrialError("invalid_npy_header", f"cannot parse NPY header: {error}", "invalid_trace") from error
    if not isinstance(header, dict) or set(("descr", "fortran_order", "shape")) - set(header):
        raise TrialError("invalid_npy_header", "NPY header is missing required fields", "invalid_trace")
    descr = header["descr"]
    shape = header["shape"]
    if not isinstance(descr, str) or not isinstance(shape, tuple):
        raise TrialError("invalid_npy_header", "NPY dtype or shape is malformed", "invalid_trace")
    if header["fortran_order"]:
        raise TrialError("unsupported_npy_layout", "Fortran-order arrays are not accepted", "invalid_trace")
    if any(not isinstance(n, int) or n < 0 for n in shape):
        raise TrialError("invalid_npy_shape", "NPY shape contains an invalid dimension", "invalid_trace")
    count = math.prod(shape) if shape else 1
    data = blob[stop:]
    match = re.fullmatch(r"([<>=|])([fiu bU])(\d+)", descr.replace(" ", ""))
    if not match:
        raise TrialError("unsupported_npy_dtype", f"unsupported or unsafe NPY dtype {descr!r}", "invalid_trace")
    endian, kind, width_text = match.groups()
    width = int(width_text)
    if kind == "b":
        item_size = 1
        if width != 1:
            raise TrialError("unsupported_npy_dtype", f"invalid boolean dtype {descr}", "invalid_trace")
    elif kind == "U":
        item_size = 4 * width
    else:
        item_size = width
        if kind == "f" and width not in (4, 8):
            raise TrialError("unsupported_npy_dtype", f"unsupported float dtype {descr}", "invalid_trace")
        if kind in ("i", "u") and width not in (1, 2, 4, 8):
            raise TrialError("unsupported_npy_dtype", f"unsupported integer dtype {descr}", "invalid_trace")
    if len(data) != count * item_size:
        raise TrialError("npy_length_mismatch", "NPY payload length does not match dtype and shape", "invalid_trace")
    if kind == "U":
        order = "little" if endian in ("<", "=") and sys.byteorder == "little" or endian == "<" else "big"
        codec = "utf-32-le" if order == "little" else "utf-32-be"
        values = [
            data[index * item_size:(index + 1) * item_size].decode(codec).rstrip("\x00")
            for index in range(count)
        ]
    elif kind == "b":
        values = [bool(value) for value in data]
    else:
        prefix = "<" if endian in ("<", "=") and (endian != "=" or sys.byteorder == "little") else ">"
        code = {"f": {4: "f", 8: "d"}, "i": {1: "b", 2: "h", 4: "i", 8: "q"},
                "u": {1: "B", 2: "H", 4: "I", 8: "Q"}}[kind][width]
        values = [item[0] for item in struct.iter_unpack(prefix + code, data)]
    return kind, shape, values


def read_npz(path: Path, maximum_bytes: int) -> dict[str, tuple[str, tuple[int, ...], list[Any]]]:
    result: dict[str, tuple[str, tuple[int, ...], list[Any]]] = {}
    try:
        with zipfile.ZipFile(path, "r") as archive:
            total = sum(info.file_size for info in archive.infolist())
            if total > maximum_bytes:
                raise TrialError("trace_too_large", "NPZ uncompressed bytes exceed configured limit", "invalid_trace")
            for info in archive.infolist():
                if info.is_dir() or not info.filename.endswith(".npy") or "/" in info.filename or "\\" in info.filename:
                    raise TrialError("invalid_npz_member", "NPZ contains a non-array or unsafe member path", "invalid_trace")
                key = info.filename[:-4]
                if not key or key in result:
                    raise TrialError("duplicate_npz_key", f"NPZ contains a duplicate/empty key {key!r}", "invalid_trace")
                if info.file_size > maximum_bytes:
                    raise TrialError("trace_member_too_large", f"NPZ member {key} exceeds configured limit", "invalid_trace")
                result[key] = read_npy(archive.read(info))
    except TrialError:
        raise
    except (OSError, zipfile.BadZipFile, RuntimeError, ValueError) as error:
        raise TrialError("invalid_npz", f"cannot read NPZ archive: {error}", "invalid_trace") from error
    return result


def scalar(arrays: dict[str, tuple[str, tuple[int, ...], list[Any]]], key: str) -> Any:
    if key not in arrays:
        raise TrialError("missing_trace_key", f"NPZ is missing required key {key}", "invalid_trace")
    _, shape, values = arrays[key]
    if shape not in ((), (1,)) or len(values) != 1:
        raise TrialError("invalid_trace_scalar", f"NPZ key {key} must be scalar", "invalid_trace")
    return values[0]


def numeric_array(arrays: dict[str, tuple[str, tuple[int, ...], list[Any]]], key: str) -> tuple[tuple[int, ...], list[float]]:
    if key not in arrays:
        raise TrialError("missing_trace_key", f"NPZ is missing required key {key}", "invalid_trace")
    kind, shape, values = arrays[key]
    if kind not in ("f", "i", "u"):
        raise TrialError("invalid_trace_numeric_type", f"NPZ key {key} must be numeric", "invalid_trace")
    converted = [float(value) for value in values]
    if any(not math.isfinite(value) for value in converted):
        raise TrialError("non_finite_trace", f"NPZ key {key} contains non-finite values", "invalid_trace")
    return shape, converted


def verify_trace(path: Path, expected: dict[str, Any], max_bytes: int) -> dict[str, Any]:
    arrays = read_npz(path, max_bytes)
    required = ("algorithm", "problem", "seed", "best", "n_init", "iterations",
                "per_iteration_value", "wall_time", "X", "y", "c")
    for key in required:
        if key not in arrays:
            raise TrialError("missing_trace_key", f"NPZ is missing required key {key}", "invalid_trace")
    algorithm = scalar(arrays, "algorithm")
    problem = scalar(arrays, "problem")
    seed = int(scalar(arrays, "seed"))
    if algorithm != expected["algorithm"] or problem != expected["problem"] or seed != expected["seed"]:
        raise TrialError("trace_identity_mismatch", "NPZ algorithm/problem/seed do not match requested trial", "invalid_trace")

    x_shape, x_values = numeric_array(arrays, "X")
    y_shape, y_values = numeric_array(arrays, "y")
    c_shape, c_values = numeric_array(arrays, "c")
    iterations_shape, iterations = numeric_array(arrays, "iterations")
    history_shape, history = numeric_array(arrays, "per_iteration_value")
    wall_shape, wall = numeric_array(arrays, "wall_time")
    n_init = int(scalar(arrays, "n_init"))
    best = float(scalar(arrays, "best"))
    if not math.isfinite(best):
        raise TrialError("non_finite_best", "NPZ best value is non-finite", "invalid_trace")
    if len(x_shape) != 2 or x_shape[0] <= 0 or x_shape[1] != expected["dimension"]:
        raise TrialError("x_shape_mismatch", f"X shape {x_shape} differs from expected (n,{expected['dimension']})", "invalid_trace")
    evaluations, dimension = x_shape
    if len(y_shape) not in (1, 2) or y_shape[0] != evaluations:
        raise TrialError("y_shape_mismatch", "y must have one row per observed X evaluation", "invalid_trace")
    objectives = 1 if len(y_shape) == 1 else y_shape[1]
    if objectives != expected["num_objectives"] and not (objectives == 1 and expected["num_objectives"] == 1):
        raise TrialError("objective_count_mismatch", "y output width differs from metadata objective count", "invalid_trace")
    if len(c_shape) != 2 or c_shape[0] != evaluations:
        raise TrialError("constraint_shape_mismatch", "c must have one row per observed X evaluation", "invalid_trace")
    if c_shape[1] not in (0, expected["num_constraints"]):
        raise TrialError("constraint_count_mismatch", "c width differs from metadata constraint count", "invalid_trace")
    if not 0 <= n_init <= evaluations:
        raise TrialError("invalid_n_init", "recorded n_init lies outside observed evaluation count", "invalid_trace")
    if len(iterations_shape) != 1 or len(history_shape) != 1 or len(wall_shape) != 1:
        raise TrialError("invalid_history_shape", "iterations/history/wall_time must be vectors", "invalid_trace")
    if not (len(iterations) == len(history) == len(wall)) or not iterations:
        raise TrialError("history_length_mismatch", "iterations, best history and wall time lengths differ", "invalid_trace")
    if iterations != [float(index) for index in range(len(iterations))]:
        raise TrialError("iteration_index_mismatch", "iteration indices are not exactly 0..n", "invalid_trace")
    if any(wall[index] > wall[index + 1] for index in range(len(wall) - 1)):
        raise TrialError("wall_time_regression", "recorded wall time regresses", "invalid_trace")
    if abs(history[-1] - best) > 1e-10 * max(1.0, abs(best)):
        raise TrialError("best_history_mismatch", "best differs from the final best-so-far trace value", "invalid_trace")

    constraint_width = c_shape[1]
    if expected["num_constraints"] == 0:
        feasibility_status = "complete_unconstrained"
        feasible = evaluations
        infeasible = 0
    elif constraint_width == expected["num_constraints"]:
        feasibility_status = "observed_constraint_matrix"
        feasible = 0
        infeasible = 0
        for row in range(evaluations):
            start = row * constraint_width
            stop = start + constraint_width
            is_feasible = all(value <= 0 for value in c_values[start:stop])
            feasible += int(is_feasible)
            infeasible += int(not is_feasible)
    else:
        # Some upstream algorithms (notably random_search) save a penalty objective
        # but omit raw constraint samples. Preserve the optimization trace while
        # refusing to infer feasibility from its penalty value.
        feasibility_status = "constraint_samples_unavailable"
        feasible = ""
        infeasible = ""

    x_min = min(x_values)
    x_max = max(x_values)
    if x_min < -1e-9 or x_max > 1.0 + 1e-9:
        raise TrialError("x_out_of_bounds", "normalized X observations lie outside the unit cube", "invalid_trace")

    return {
        "objective_evaluation_count": evaluations,
        "x_rows": evaluations,
        "y_rows": evaluations,
        "c_rows": evaluations,
        "n_init_recorded": n_init,
        "feasibility_evidence_status": feasibility_status,
        "feasible_observations": feasible,
        "infeasible_observations": infeasible,
        "best": format(best, ".17g"),
        "history_rows": len(history),
        "identity_status": "exact_problem_algorithm_seed",
        "trace_validation_status": "structural_checks_passed",
    }


def read_summary(path: Path, problem: str, algorithm: str, seed: int) -> dict[str, str]:
    if not path.is_file():
        raise TrialError("summary_missing", "upstream summary CSV is missing", "failed")
    try:
        with path.open("r", encoding="utf-8", newline="") as stream:
            reader = csv.DictReader(stream)
            if tuple(reader.fieldnames or ()) != SUMMARY_HEADER:
                raise TrialError("summary_header_mismatch", "upstream summary header differs from expected schema", "invalid_trace")
            rows = list(reader)
    except UnicodeError as error:
        raise TrialError("summary_encoding_error", f"summary CSV is not UTF-8: {error}", "invalid_trace") from error
    matches = [row for row in rows if row["algorithm"] == algorithm and row["problem"] == problem and row["seed"] == str(seed)]
    if len(matches) != 1:
        raise TrialError("summary_identity_missing_or_duplicate", f"expected one summary row for {algorithm}/{problem}/{seed}, got {len(matches)}", "failed")
    return matches[0]


def run_git(root: Path, args: list[str]) -> str:
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True, check=False)
    if result.returncode != 0:
        raise TrialError("git_preflight_failed", result.stderr.decode("utf-8", "replace"), "wrapper_error")
    return result.stdout.decode("utf-8", "strict").strip()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bocode-root", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--package-version", required=True)
    parser.add_argument("--problem", required=True)
    parser.add_argument("--algorithm", required=True)
    parser.add_argument("--seed", type=int, required=True)
    parser.add_argument("--n-init", type=int, required=True)
    parser.add_argument("--iters", type=int, required=True)
    parser.add_argument("--metadata-json", type=Path, required=True)
    parser.add_argument("--expected-metadata-sha256", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--timeout-seconds", type=float, default=3600.0)
    parser.add_argument("--max-trace-bytes", type=int, default=536870912)
    args = parser.parse_args()

    output = args.output_dir
    output.mkdir(parents=True, exist_ok=True)
    (output / "pip-freeze.txt").write_bytes(b"")
    (output / "upstream.stdout").write_bytes(b"")
    (output / "upstream.stderr").write_bytes(b"")
    trace_fields: dict[str, Any] = {name: "" for name in TRACE_HEADER}
    trace_fields.update({
        "schema": TRACE_SCHEMA, "problem_id": args.problem, "algorithm": args.algorithm,
        "seed": args.seed, "raw_trace_path": "raw_trace.npz", "summary_state": "missing",
        "feasibility_evidence_status": "unavailable", "identity_status": "not_checked",
        "trace_validation_status": "not_checked", "evidence_level": EVIDENCE_LEVEL,
    })
    trial_fields: dict[str, Any] = {name: "" for name in RECEIPT_HEADER}
    trial_fields.update({
        "schema": TRIAL_SCHEMA, "problem_id": args.problem, "algorithm": args.algorithm,
        "seed": args.seed, "source_commit": args.source_commit,
        "metadata_sha256": args.expected_metadata_sha256, "git_clean": "false",
        "skip_detected": "false", "trial_state": "wrapper_error",
        "diagnostic_code": "preflight_not_completed", "diagnostic_message": "",
        "evidence_level": EVIDENCE_LEVEL,
    })
    stdout_bytes = b""
    stderr_bytes = b""
    freeze_bytes = b""
    start = time.monotonic_ns()
    child_usage_before = resource.getrusage(resource.RUSAGE_CHILDREN)
    raw_trace_sha = ""
    summary_bytes = b""
    try:
        safe_name(args.problem, "problem")
        safe_name(args.algorithm, "algorithm")
        if len(args.source_commit) != 40 or any(ch.lower() not in HEX for ch in args.source_commit):
            raise TrialError("invalid_source_commit", "source commit must be full 40-character hexadecimal", "wrapper_error")
        if not valid_sha(args.expected_metadata_sha256):
            raise TrialError("invalid_metadata_digest", "expected metadata SHA-256 is malformed", "wrapper_error")
        if args.seed < 0 or args.n_init < 0 or args.iters <= 0 or args.timeout_seconds <= 0 or args.max_trace_bytes <= 0:
            raise TrialError("invalid_budget", "seed/n_init/iters/timeout/trace limits are invalid", "wrapper_error")

        root = args.bocode_root.resolve(strict=True)
        head = run_git(root, ["rev-parse", "HEAD"])
        status = run_git(root, ["status", "--porcelain", "--untracked-files=all"])
        clean = not bool(status)
        trial_fields["git_clean"] = str(clean).lower()
        if head.lower() != args.source_commit.lower():
            raise TrialError("source_commit_mismatch", f"expected {args.source_commit}, got {head}", "wrapper_error")
        if not clean:
            raise TrialError("dirty_source_tree", "BOCoDe source tree is not clean", "wrapper_error")

        try:
            import bocode
            version = importlib.metadata.version("bocode")
        except Exception as error:
            raise TrialError("bocode_import_failed", f"cannot import BOCoDe or read package version: {error}", "wrapper_error") from error
        trial_fields["bocode_version"] = version
        if version != args.package_version:
            raise TrialError("package_version_mismatch", f"expected {args.package_version}, got {version}", "wrapper_error")
        module_path = Path(bocode.__file__).resolve()
        if not module_path.is_relative_to(root):
            raise TrialError("package_source_mismatch", f"imported BOCoDe module is outside pinned source tree: {module_path}", "wrapper_error")

        try:
            installed_metadata = importlib.resources.files("bocode.opt_problems_metadata").joinpath(f"{args.problem}.json").read_bytes()
        except Exception as error:
            raise TrialError("metadata_resource_missing", f"cannot read installed task metadata resource: {error}", "wrapper_error") from error
        staged_metadata = args.metadata_json.read_bytes()
        if installed_metadata != staged_metadata:
            raise TrialError("metadata_source_mismatch", "staged task metadata differs from installed BOCoDe resource", "wrapper_error")
        metadata_sha = sha256(installed_metadata)
        if metadata_sha != args.expected_metadata_sha256.lower():
            raise TrialError("metadata_digest_mismatch", "installed task metadata SHA-256 differs from expected digest", "wrapper_error")
        metadata = json.loads(installed_metadata.decode("utf-8"))
        if metadata.get("name") != args.problem:
            raise TrialError("metadata_identity_mismatch", "raw metadata resource name differs from requested problem", "wrapper_error")
        dim = metadata.get("dim")
        objectives = metadata.get("num_objectives")
        constraints = metadata.get("num_constraints", 0)
        input_type = metadata.get("input_type")
        if not isinstance(dim, int) or dim <= 0 or not isinstance(objectives, int) or objectives <= 0 or not isinstance(constraints, int) or constraints < 0:
            raise TrialError("metadata_counts_invalid", "dimension/objective/constraint metadata are invalid", "wrapper_error")
        if input_type not in ("continuous", "discrete", "mixed"):
            raise TrialError("metadata_input_type_invalid", "BOCoDe input_type is unsupported", "wrapper_error")

        freeze = subprocess.run([sys.executable, "-m", "pip", "freeze", "--all"], capture_output=True, check=False)
        freeze_bytes = freeze.stdout + (b"\n# pip freeze stderr follows\n" + freeze.stderr if freeze.stderr else b"")
        (output / "pip-freeze.txt").write_bytes(freeze_bytes)
        if freeze.returncode != 0:
            raise TrialError("pip_freeze_failed", f"pip freeze --all exited {freeze.returncode}", "wrapper_error")

        results_dir = output / "upstream_results"
        results_dir.mkdir(parents=True, exist_ok=False)
        script = root / "examples" / "run_experiments.py"
        if not script.is_file():
            raise TrialError("upstream_runner_missing", "examples/run_experiments.py is missing", "wrapper_error")
        command = [
            sys.executable, str(script),
            "--problems", args.problem,
            "--algorithms", args.algorithm,
            "--seeds", str(args.seed),
            "--n-init", str(args.n_init),
            "--iters", str(args.iters),
            "--outdir", str(results_dir),
        ]
        trial_fields["diagnostic_message"] = "argv=" + shlex.join(command)
        begin = time.monotonic_ns()
        try:
            process = subprocess.run(
                command, cwd=root, capture_output=True, check=False, timeout=args.timeout_seconds
            )
            stdout_bytes = process.stdout
            stderr_bytes = process.stderr
            return_code = process.returncode
        except subprocess.TimeoutExpired as error:
            stdout_bytes = error.stdout or b""
            stderr_bytes = error.stderr or b""
            return_code = 124
            raise TrialError("upstream_timeout", "upstream BOCoDe trial exceeded timeout", "failed")
        elapsed_ms = (time.monotonic_ns() - begin) // 1_000_000
        trial_fields["return_code"] = return_code
        trial_fields["elapsed_milliseconds"] = elapsed_ms
        (output / "upstream.stdout").write_bytes(stdout_bytes)
        (output / "upstream.stderr").write_bytes(stderr_bytes)
        text_output = stdout_bytes.decode("utf-8", "replace")
        skip_detected = re.search(
            rf"\bSKIP\s+{re.escape(args.algorithm)}\s*/\s*{re.escape(args.problem)}\s*/\s*seed{args.seed}\b",
            text_output,
        ) is not None
        trial_fields["skip_detected"] = str(skip_detected).lower()
        if skip_detected:
            raise TrialError("upstream_skip", "upstream runner emitted SKIP for the requested trial despite its process exit status", "failed")
        if return_code != 0:
            raise TrialError("upstream_nonzero_exit", f"upstream runner exited with code {return_code}", "failed")

        summary_path = results_dir / "summary_task0.csv"
        summary = read_summary(summary_path, args.problem, args.algorithm, args.seed)
        summary_bytes = summary_path.read_bytes()
        (output / "upstream_summary.csv").write_bytes(summary_bytes)
        trace_path = results_dir / f"{args.algorithm}_{args.problem}_seed{args.seed}.npz"
        if not trace_path.is_file():
            raise TrialError("trace_missing", "upstream summary exists but expected NPZ trace is missing", "failed")
        (output / "raw_trace.npz").write_bytes(trace_path.read_bytes())
        raw_trace_sha = sha256((output / "raw_trace.npz").read_bytes())

        trace = verify_trace(output / "raw_trace.npz", {
            "algorithm": args.algorithm, "problem": args.problem, "seed": args.seed,
            "dimension": dim, "num_objectives": objectives, "num_constraints": constraints,
        }, args.max_trace_bytes)
        if summary["npz"] != trace_path.name:
            raise TrialError("summary_trace_name_mismatch", "summary NPZ name differs from exact trace identity", "invalid_trace")
        try:
            summary_best = float(summary["best"])
        except ValueError as error:
            raise TrialError("summary_best_invalid", "summary best value is not numeric", "invalid_trace") from error
        if abs(summary_best - float(trace["best"])) > 1e-10 * max(1.0, abs(summary_best)):
            raise TrialError("summary_trace_best_mismatch", "summary best differs from NPZ best", "invalid_trace")

        trace_fields.update({
            "dimension": dim, "num_objectives": objectives, "num_constraints": constraints,
            "input_type": input_type, "raw_trace_byte_length": (output / "raw_trace.npz").stat().st_size,
            "raw_trace_sha256": raw_trace_sha, "summary_state": "present",
            "summary_byte_length": len(summary_bytes), "summary_sha256": sha256(summary_bytes),
            **trace,
        })
        trial_fields.update({
            "trial_state": "succeeded", "diagnostic_code": "none",
            "diagnostic_message": "trace structurally validated; feasibility may be unavailable when the upstream method does not save raw constraints",
            "raw_trace_sha256": raw_trace_sha,
        })
    except TrialError as error:
        trial_fields["trial_state"] = error.state
        trial_fields["diagnostic_code"] = error.code
        trial_fields["diagnostic_message"] = str(error)
    except Exception as error:
        trial_fields["trial_state"] = "wrapper_error"
        trial_fields["diagnostic_code"] = "unexpected_wrapper_exception"
        trial_fields["diagnostic_message"] = f"{type(error).__name__}: {error}\n{traceback.format_exc()}"
    finally:
        if not (output / "pip-freeze.txt").exists() or (output / "pip-freeze.txt").stat().st_size == 0:
            if not freeze_bytes:
                freeze_bytes = b"# pip freeze not reached\n"
                (output / "pip-freeze.txt").write_bytes(freeze_bytes)
        if not (output / "upstream.stdout").exists():
            (output / "upstream.stdout").write_bytes(stdout_bytes)
        if not (output / "upstream.stderr").exists():
            (output / "upstream.stderr").write_bytes(stderr_bytes)
        if not trial_fields["return_code"]:
            trial_fields["return_code"] = trial_fields["return_code"] or ""
        trial_fields["stdout_sha256"] = sha256(stdout_bytes)
        trial_fields["stderr_sha256"] = sha256(stderr_bytes)
        trial_fields["pip_freeze_sha256"] = sha256((output / "pip-freeze.txt").read_bytes())
        trial_fields["raw_trace_sha256"] = raw_trace_sha
        elapsed_ms = (time.monotonic_ns() - start) // 1_000_000
        trial_fields["elapsed_milliseconds"] = trial_fields["elapsed_milliseconds"] or elapsed_ms
        trial_fields["trace_audit_sha256"] = ""
        if not trace_fields["trace_validation_status"]:
            trace_fields["trace_validation_status"] = "not_validated"
        trace_bytes = csv_bytes(TRACE_HEADER, trace_fields)
        (output / "trace_audit.csv").write_bytes(trace_bytes)
        trial_fields["trace_audit_sha256"] = sha256(trace_bytes)
        digest_values = {key: str(trial_fields.get(key, "")) for key in RECEIPT_HEADER if key != "receipt_sha256"}
        trial_fields["receipt_sha256"] = sha256(canonical_json(digest_values))
        (output / "trial_receipt.csv").write_bytes(csv_bytes(RECEIPT_HEADER, trial_fields))

    print(f"bocode_trial_state={trial_fields['trial_state']} problem={args.problem} algorithm={args.algorithm} seed={args.seed}")
    print(f"bocode_trial_receipt_sha256={trial_fields['receipt_sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
