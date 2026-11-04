#!/usr/bin/env python3
"""Run one BOCoDe optimizer trial and retain a strict, replayable evidence receipt.

This is an external Python adapter because BOCoDe itself is Python/PyTorch. Symtropy's
production evaluator and receipt admission remain Rust-first; no BOCoDe dependency is added
to the Rust crate.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import importlib.metadata
import json
import math
import os
from pathlib import Path
import re
import resource
import shutil
import subprocess
import sys
import time
import traceback
from typing import Any

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
    "peak_rss_bytes", "gpu_compute_ns", "energy_microjoules",
    "process_exit_code", "skip_detected", "summary_row_present", "trace_valid",
    "diagnostic_code", "receipt_sha256",
)
ENV_ALLOWLIST = (
    "OMP_NUM_THREADS", "MKL_NUM_THREADS", "OPENBLAS_NUM_THREADS",
    "NUMEXPR_NUM_THREADS", "CUDA_VISIBLE_DEVICES", "PYTHONHASHSEED",
    "CUBLAS_WORKSPACE_CONFIG",
)
DIAGNOSTIC_CODES = {
    "none", "preflight_failed", "subprocess_error", "upstream_skip",
    "process_nonzero", "missing_summary", "summary_row_mismatch",
    "missing_archive", "invalid_trace",
}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def append_u64(buffer: bytearray, value: int) -> None:
    if value < 0 or value >= 1 << 64:
        raise ValueError("canonical receipt length is outside u64")
    buffer.extend(value.to_bytes(8, "little"))


def append_string(buffer: bytearray, value: str) -> None:
    encoded = value.encode("utf-8")
    append_u64(buffer, len(encoded))
    buffer.extend(encoded)


def receipt_digest(fields: dict[str, str]) -> str:
    buffer = bytearray()
    for name in HEADER[:-1]:
        append_string(buffer, name)
        append_string(buffer, fields[name])
    return sha256(bytes(buffer))


def safe_token(value: str, name: str) -> str:
    if not value or any(char in value for char in "\r\n\t\0"):
        raise ValueError(f"{name} must be non-empty and single-line")
    return value


def file_record(root: Path, path: Path | None) -> tuple[str, str, str]:
    if path is None or not path.is_file():
        return "-", "0", "-"
    relative = path.relative_to(root).as_posix()
    data = path.read_bytes()
    return relative, str(len(data)), sha256(data)


def write_receipt(root: Path, fields: dict[str, str]) -> None:
    for name in HEADER[:-1]:
        if name not in fields:
            raise ValueError(f"receipt is missing field {name}")
    fields["receipt_sha256"] = receipt_digest(fields)
    ordered = {name: fields[name] for name in HEADER}
    receipt_json = json.dumps(
        ordered, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False
    ).encode("utf-8") + b"\n"
    (root / "receipt.json").write_bytes(receipt_json)
    with (root / "receipt.csv").open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(
            stream, fieldnames=HEADER, quoting=csv.QUOTE_ALL, lineterminator="\n"
        )
        writer.writeheader()
        writer.writerow(ordered)


def load_npz(path: Path, expected: dict[str, Any]) -> dict[str, Any]:
    import numpy as np

    with np.load(path, allow_pickle=False) as archive:
        required = {
            "algorithm", "problem", "seed", "best", "n_init",
            "iterations", "per_iteration_value", "wall_time", "X", "y", "c",
        }
        missing = required - set(archive.files)
        if missing:
            raise ValueError("NPZ is missing required arrays: " + ",".join(sorted(missing)))

        def scalar(name: str) -> Any:
            value = np.asarray(archive[name])
            if value.size != 1:
                raise ValueError(f"NPZ {name} must contain exactly one scalar")
            scalar_value = value.reshape(-1)[0]
            if isinstance(scalar_value, bytes):
                return scalar_value.decode("utf-8")
            if hasattr(scalar_value, "item"):
                return scalar_value.item()
            return scalar_value

        algorithm = str(scalar("algorithm"))
        problem = str(scalar("problem"))
        seed = int(scalar("seed"))
        if algorithm != expected["algorithm"] or problem != expected["problem_id"] or seed != expected["seed"]:
            raise ValueError("NPZ algorithm/problem/seed identity mismatch")

        X = np.asarray(archive["X"])
        y = np.asarray(archive["y"])
        c = np.asarray(archive["c"])
        if X.ndim != 2 or X.shape[1] != expected["dimension"] or X.shape[0] == 0:
            raise ValueError("NPZ X must be a non-empty (evaluations, dimension) matrix")
        evaluations = int(X.shape[0])
        if y.ndim == 1:
            y_outputs = 1
            y_rows = int(y.shape[0])
        elif y.ndim == 2:
            y_outputs = int(y.shape[1])
            y_rows = int(y.shape[0])
        else:
            raise ValueError("NPZ y must be a one- or two-dimensional numeric array")
        if y_rows != evaluations or y_outputs != expected["num_objectives"]:
            raise ValueError("NPZ objective history shape contradicts task metadata")
        if c.ndim != 2 or c.shape != (evaluations, expected["num_constraints"]):
            raise ValueError("NPZ constraint history shape contradicts task metadata")
        for name, array in (("X", X), ("y", y), ("c", c)):
            if not np.issubdtype(array.dtype, np.number) or not np.isfinite(array).all():
                raise ValueError(f"NPZ {name} must be finite numeric data")

        n_init_value = scalar("n_init")
        if isinstance(n_init_value, bool) or int(n_init_value) != n_init_value:
            raise ValueError("NPZ n_init must be an integer")
        n_init = int(n_init_value)
        if n_init < 0 or n_init > evaluations:
            raise ValueError("NPZ n_init is outside the observed evaluation count")

        best = float(scalar("best"))
        if not math.isfinite(best):
            raise ValueError("NPZ best value must be finite")
        iterations = np.asarray(archive["iterations"])
        values = np.asarray(archive["per_iteration_value"])
        wall_time = np.asarray(archive["wall_time"])
        if iterations.ndim != 1 or values.ndim != 1 or wall_time.ndim != 1:
            raise ValueError("NPZ per-iteration traces must be one-dimensional")
        if len(iterations) == 0 or len(iterations) != len(values) or len(values) != len(wall_time):
            raise ValueError("NPZ iteration, value and wall-time traces must be non-empty and aligned")
        if not np.issubdtype(iterations.dtype, np.number) or not np.isfinite(iterations).all():
            raise ValueError("NPZ iteration indices must be finite numeric values")
        if not np.issubdtype(values.dtype, np.number) or not np.isfinite(values).all():
            raise ValueError("NPZ best-so-far trace must be finite numeric values")
        if not np.issubdtype(wall_time.dtype, np.number) or not np.isfinite(wall_time).all():
            raise ValueError("NPZ wall-time trace must be finite numeric values")
        if (np.diff(iterations) < 0).any() or (np.diff(wall_time) < 0).any():
            raise ValueError("NPZ iteration and wall-time traces must be monotonic")

        if expected["num_constraints"] == 0:
            feasible = np.ones(evaluations, dtype=bool)
        else:
            feasible = np.all(c <= 0, axis=1)
        feasible_count = int(feasible.sum())
        first_feasible = int(np.flatnonzero(feasible)[0]) if feasible_count else None
        return {
            "objective_evaluation_count": evaluations,
            "initial_design_count": n_init,
            "feasible_evaluation_count": feasible_count,
            "first_feasible_eval_index": first_feasible,
            "best_value": format(best, ".17g"),
        }


def resource_snapshot() -> tuple[int | None, int | None]:
    try:
        usage = resource.getrusage(resource.RUSAGE_CHILDREN)
        return (
            int(round(usage.ru_utime * 1_000_000_000)),
            int(round(usage.ru_stime * 1_000_000_000)),
        )
    except Exception:
        return None, None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bocode-root", type=Path, required=True)
    parser.add_argument("--outdir", type=Path, required=True)
    parser.add_argument("--problem", required=True)
    parser.add_argument("--algorithm", required=True)
    parser.add_argument("--seed", type=int, required=True)
    parser.add_argument("--n-init", type=int, required=True)
    parser.add_argument("--iters", type=int, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--package-version", required=True)
    parser.add_argument("--metadata-sha256", required=True)
    args = parser.parse_args()

    problem = safe_token(args.problem, "problem")
    algorithm = safe_token(args.algorithm, "algorithm")
    if args.seed < 0 or args.n_init < 0 or args.iters <= 0:
        parser.error("seed/n-init must be non-negative and iters must be positive")
    if len(args.source_commit) != 40 or not re.fullmatch(r"[0-9a-fA-F]{40}", args.source_commit):
        parser.error("--source-commit must be a full 40-character hexadecimal commit")
    if not re.fullmatch(r"[0-9a-fA-F]{64}", args.metadata_sha256):
        parser.error("--metadata-sha256 must be a 64-character SHA-256 digest")
    if not re.fullmatch(r"[0-9a-fA-F]{64}", args.metadata_sha256):
        parser.error("malformed metadata digest")
    trial_id = f"{problem}__{algorithm}__seed{args.seed}"
    root = args.outdir.resolve()
    repo_root = args.bocode_root.resolve()
    try:
        root.relative_to(repo_root)
    except ValueError:
        pass
    else:
        parser.error("outdir must be outside the BOCoDe source tree to preserve clean-tree evidence")
    if root.exists() and any(root.iterdir()):
        parser.error("outdir must be new or empty; stale files are not accepted")
    root.mkdir(parents=True, exist_ok=True)
    upstream_out = root / "upstream"
    upstream_out.mkdir()
    (root / "stdout.bin").write_bytes(b"")
    (root / "stderr.bin").write_bytes(b"")
    (root / "environment-freeze.txt").write_bytes(b"")
    (root / "environment.json").write_text(
        json.dumps({key: os.environ[key] for key in ENV_ALLOWLIST if key in os.environ},
                   sort_keys=True, separators=(",", ":")) + "\\n",
        encoding="utf-8",
    )

    source_commit = "-"
    package_version = "-"
    metadata_bytes = b""
    metadata_path: Path | None = None
    environment_path = root / "environment.json"
    command_path: Path | None = None
    archive_path: Path | None = None
    summary_path: Path | None = None
    diagnostic_path: Path | None = None
    stdout_path = root / "stdout.bin"
    stderr_path = root / "stderr.bin"
    freeze_path = root / "environment-freeze.txt"
    state = "wrapper_error"
    diagnostic_code = "preflight_failed"
    diagnostic_message = ""
    process_exit_code: int | None = None
    skip_detected = False
    summary_row_present = False
    trace_valid = False
    metrics = {
        "objective_evaluation_count": "0",
        "initial_design_count": "0",
        "feasible_evaluation_count": "0",
        "first_feasible_eval_index": "-",
        "best_value": "-",
    }
    dimension = num_objectives = num_constraints = 0
    input_type = "-"
    metadata_file_sha256 = "-"
    metadata_byte_count = 0
    freeze_byte_count = 0
    freeze_sha256 = "-"
    elapsed_ns: int | None = None
    child_user_ns: int | None = None
    child_system_ns: int | None = None

    try:
        if not repo_root.is_dir():
            raise ValueError("BOCoDe root is not a directory")
        runner = repo_root / "examples" / "run_experiments.py"
        if not runner.is_file():
            raise ValueError("BOCoDe examples/run_experiments.py is missing")
        git = subprocess.run(
            ["git", "-C", str(repo_root), "rev-parse", "HEAD"],
            check=True, capture_output=True, text=True,
        )
        source_commit = git.stdout.strip().lower()
        if source_commit != args.source_commit.lower():
            raise ValueError("BOCoDe source HEAD does not match the declared commit")
        status = subprocess.run(
            ["git", "-C", str(repo_root), "status", "--porcelain", "--untracked-files=all"],
            check=True, capture_output=True,
        )
        if status.stdout.strip():
            raise ValueError("BOCoDe source tree is not clean")

        package_version = importlib.metadata.version("bocode")
        if package_version != args.package_version:
            raise ValueError("installed BOCoDe package version does not match the declared version")
        source_metadata_path = repo_root / "bocode" / "opt_problems_metadata" / f"{problem}.json"
        metadata_bytes = source_metadata_path.read_bytes()
        metadata_path = root / "metadata.json"
        metadata_path.write_bytes(metadata_bytes)
        metadata_file_sha256 = sha256(metadata_bytes)
        metadata_byte_count = len(metadata_bytes)
        if metadata_file_sha256 != args.metadata_sha256.lower():
            raise ValueError("task metadata bytes do not match the catalog's declared SHA-256")
        metadata = json.loads(metadata_bytes.decode("utf-8"))
        if metadata.get("name") != problem:
            raise ValueError("task metadata resource name does not match requested problem")
        dimension = int(metadata["dim"])
        num_objectives = int(metadata["num_objectives"])
        num_constraints = int(metadata.get("num_constraints", 0))
        input_type = str(metadata["input_type"])
        if dimension <= 0 or num_objectives <= 0 or num_constraints < 0:
            raise ValueError("task metadata has invalid dimension/objective/constraint counts")
        if input_type not in ("continuous", "discrete", "mixed"):
            raise ValueError("task metadata has an unknown input_type")

        freeze = subprocess.run(
            [sys.executable, "-m", "pip", "freeze", "--all"],
            cwd=repo_root, capture_output=True, check=False,
        )
        freeze_bytes = freeze.stdout
        freeze_path.write_bytes(freeze_bytes)
        freeze_byte_count = len(freeze_bytes)
        freeze_sha256 = sha256(freeze_bytes)
        if freeze.returncode != 0:
            raise ValueError("pip freeze --all failed")

        # Import once before timing the optimizer process to confirm that this interpreter
        # resolves the expected editable/source checkout, but preserve the child process as
        # the actual execution boundary.
        import bocode
        module_path = Path(bocode.__file__).resolve()
        if not module_path.is_relative_to(repo_root):
            raise ValueError("imported bocode module does not resolve inside the pinned source checkout")

        command = [
            sys.executable, str(runner),
            "--problems", problem,
            "--algorithms", algorithm,
            "--seeds", str(args.seed),
            "--n-init", str(args.n_init),
            "--iters", str(args.iters),
            "--outdir", str(upstream_out),
            "--task-id", "0",
            "--num-tasks", "1",
        ]
        command_path = root / "command.json"
        command_path.write_text(
            json.dumps({"argv": command, "environment_allowlist": ENV_ALLOWLIST},
                       ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\\n",
            encoding="utf-8",
        )
        child_user_before, child_system_before = resource_snapshot()
        started = time.monotonic_ns()
        completed = subprocess.run(command, cwd=repo_root, env=os.environ.copy(), capture_output=True)
        elapsed_ns = time.monotonic_ns() - started
        child_user_after, child_system_after = resource_snapshot()
        if child_user_before is not None and child_user_after is not None:
            child_user_ns = max(0, child_user_after - child_user_before)
        if child_system_before is not None and child_system_after is not None:
            child_system_ns = max(0, child_system_after - child_system_before)
        process_exit_code = completed.returncode
        stdout_path.write_bytes(completed.stdout)
        stderr_path.write_bytes(completed.stderr)
        stdout_text = completed.stdout.decode("utf-8", errors="replace")
        skip_detected = bool(re.search(r"(?m)^\s*SKIP\s+", stdout_text))
        expected_tag = f"{algorithm} / {problem} / seed{args.seed}"
        if skip_detected and expected_tag not in stdout_text:
            raise ValueError("upstream reported SKIP for a non-matching trial identity")

        candidate_summary = upstream_out / "summary_task0.csv"
        if candidate_summary.is_file():
            summary_path = root / "summary_task0.csv"
            shutil.copyfile(candidate_summary, summary_path)
            with candidate_summary.open("r", encoding="utf-8", newline="") as stream:
                summary_rows = list(csv.DictReader(stream))
            matching = [
                row for row in summary_rows
                if row.get("algorithm") == algorithm
                and row.get("problem") == problem
                and row.get("seed") == str(args.seed)
            ]
            summary_row_present = len(matching) == 1
            if matching:
                best_summary = float(matching[0]["best"])
                if not math.isfinite(best_summary):
                    summary_row_present = False

        candidate_archive = upstream_out / f"{algorithm}_{problem}_seed{args.seed}.npz"
        if candidate_archive.is_file():
            archive_path = root / "result.npz"
            shutil.copyfile(candidate_archive, archive_path)

        if skip_detected:
            state, diagnostic_code = "failed", "upstream_skip"
            diagnostic_message = "upstream runner emitted SKIP for the scheduled trial"
        elif process_exit_code != 0:
            state, diagnostic_code = "failed", "process_nonzero"
            diagnostic_message = f"upstream runner exited with status {process_exit_code}"
        elif not summary_row_present:
            state, diagnostic_code = "failed", "summary_row_mismatch"
            diagnostic_message = "summary CSV had no unique row for the scheduled trial"
        elif archive_path is None:
            state, diagnostic_code = "invalid_trace", "missing_archive"
            diagnostic_message = "upstream summary exists but the expected NPZ archive is missing"
        else:
            try:
                metrics = load_npz(archive_path, {
                    "algorithm": algorithm,
                    "problem_id": problem,
                    "seed": args.seed,
                    "dimension": dimension,
                    "num_objectives": num_objectives,
                    "num_constraints": num_constraints,
                })
                trace_valid = True
                if not math.isclose(
                    float(metrics["best_value"]), best_summary,
                    rel_tol=1e-12, abs_tol=1e-12,
                ):
                    raise ValueError("summary best value does not match NPZ best value")
                state, diagnostic_code = "succeeded", "none"
            except Exception as error:
                state, diagnostic_code = "invalid_trace", "invalid_trace"
                diagnostic_message = f"{type(error).__name__}: {error}"

    except Exception as error:
        state = "wrapper_error"
        diagnostic_code = "preflight_failed" if process_exit_code is None else "subprocess_error"
        diagnostic_message = f"{type(error).__name__}: {error}"
        if not stderr_path.read_bytes():
            stderr_path.write_text(diagnostic_message + "\n", encoding="utf-8")

    if diagnostic_code not in DIAGNOSTIC_CODES:
        state, diagnostic_code = "wrapper_error", "preflight_failed"
        diagnostic_message = "unrecognized wrapper diagnostic state"
    if diagnostic_message:
        diagnostic_path = root / "diagnostic.txt"
        diagnostic_path.write_text(diagnostic_message + "\n", encoding="utf-8")

    artifacts = {
        "archive": file_record(root, archive_path),
        "summary": file_record(root, summary_path),
        "stdout": file_record(root, stdout_path),
        "stderr": file_record(root, stderr_path),
        "metadata": file_record(root, metadata_path if metadata_path and metadata_path.is_file() else None),
        "freeze": file_record(root, freeze_path),
        "environment": file_record(root, environment_path),
        "command": file_record(root, command_path),
        "diagnostic": file_record(root, diagnostic_path),
    }
    fields = {
        "schema": SCHEMA,
        "trial_id": trial_id,
        "problem_id": problem,
        "algorithm": algorithm,
        "seed": str(args.seed),
        "source_commit": source_commit,
        "bocode_version": package_version,
        "python_version": sys.version.split()[0],
        "wrapper_script_sha256": sha256(Path(__file__).read_bytes()),
        "metadata_sha256": args.metadata_sha256.lower(),
        "dimension": str(dimension),
        "num_objectives": str(num_objectives),
        "num_constraints": str(num_constraints),
        "input_type": input_type,
        "requested_n_init": str(args.n_init),
        "requested_iters": str(args.iters),
        "state": state,
        **metrics,
        "archive_path": artifacts["archive"][0],
        "archive_bytes": artifacts["archive"][1],
        "archive_sha256": artifacts["archive"][2],
        "summary_path": artifacts["summary"][0],
        "summary_bytes": artifacts["summary"][1],
        "summary_sha256": artifacts["summary"][2],
        "stdout_path": artifacts["stdout"][0],
        "stdout_bytes": artifacts["stdout"][1],
        "stdout_sha256": artifacts["stdout"][2],
        "stderr_path": artifacts["stderr"][0],
        "stderr_bytes": artifacts["stderr"][1],
        "stderr_sha256": artifacts["stderr"][2],
        "metadata_path": artifacts["metadata"][0],
        "metadata_bytes": artifacts["metadata"][1],
        "metadata_file_sha256": artifacts["metadata"][2],
        "freeze_path": artifacts["freeze"][0],
        "freeze_bytes": artifacts["freeze"][1],
        "freeze_sha256": artifacts["freeze"][2],
        "environment_path": artifacts["environment"][0],
        "environment_bytes": artifacts["environment"][1],
        "environment_sha256": artifacts["environment"][2],
        "command_path": artifacts["command"][0],
        "command_bytes": artifacts["command"][1],
        "command_sha256": artifacts["command"][2],
        "diagnostic_path": artifacts["diagnostic"][0],
        "diagnostic_bytes": artifacts["diagnostic"][1],
        "diagnostic_sha256": artifacts["diagnostic"][2],
        "elapsed_monotonic_ns": str(elapsed_ns) if elapsed_ns is not None else "-",
        "child_user_cpu_ns": str(child_user_ns) if child_user_ns is not None else "-",
        "child_system_cpu_ns": str(child_system_ns) if child_system_ns is not None else "-",
        "peak_rss_bytes": "-",
        "gpu_compute_ns": "-",
        "energy_microjoules": "-",
        "process_exit_code": str(process_exit_code) if process_exit_code is not None else "-",
        "skip_detected": str(skip_detected).lower(),
        "summary_row_present": str(summary_row_present).lower(),
        "trace_valid": str(trace_valid).lower(),
        "diagnostic_code": diagnostic_code,
    }
    write_receipt(root, fields)
    print(json.dumps({
        "trial_id": trial_id,
        "state": state,
        "diagnostic_code": diagnostic_code,
        "receipt": str(root / "receipt.json"),
        "receipt_sha256": fields["receipt_sha256"],
    }, sort_keys=True))
    return 0 if state == "succeeded" else 2


if __name__ == "__main__":
    raise SystemExit(main())
