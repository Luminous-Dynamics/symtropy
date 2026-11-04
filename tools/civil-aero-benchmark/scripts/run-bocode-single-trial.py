#!/usr/bin/env python3
"""Run exactly one BOCoDe trial and retain an artifact-bound terminal receipt.

This wrapper belongs in a separately pinned BOCoDe environment. It never makes
CI/process exit status stand in for per-trial success: upstream SKIP output is a
failed trial even where run_experiments.py exits successfully.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import importlib.metadata
import importlib.resources
import importlib.util
import json
import os
import re
import resource
import subprocess
import sys
import time
import uuid
from pathlib import Path
from typing import Any

SCHEMA = "bocode-single-trial-receipt/0.1.0"
SKIP_RE = re.compile(r"^\s*SKIP\s+", re.MULTILINE)
ENV_ALLOWLIST = (
    "OMP_NUM_THREADS",
    "MKL_NUM_THREADS",
    "OPENBLAS_NUM_THREADS",
    "NUMEXPR_NUM_THREADS",
    "PYTHONHASHSEED",
    "CUDA_VISIBLE_DEVICES",
    "CUBLAS_WORKSPACE_CONFIG",
    "TORCH_DETERMINISTIC",
)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, ensure_ascii=False, separators=(",", ":"), allow_nan=False
    ).encode("utf-8")


def write_artifact(root: Path, name: str, data: bytes) -> dict[str, Any]:
    path = root / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    return {
        "path": name,
        "byte_length": len(data),
        "sha256": sha256(data),
    }


def git_identity(root: Path, expected_commit: str) -> tuple[str, bool]:
    commit = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=root,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    ).stdout.strip().lower()
    status = subprocess.run(
        ["git", "status", "--porcelain", "--untracked-files=normal"],
        cwd=root,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    ).stdout
    if commit != expected_commit.lower():
        raise ValueError(f"BOCoDe source commit mismatch: expected {expected_commit}, got {commit}")
    if status.strip():
        raise ValueError("BOCoDe working tree must be clean; uncommitted/untracked files are present")
    return commit, True


def validate_npz(
    path: Path,
    *,
    task_id: str,
    algorithm: str,
    seed: int,
    dimension: int,
    num_objectives: int,
    num_constraints: int,
) -> dict[str, Any]:
    import numpy as np

    with np.load(path, allow_pickle=False) as archive:
        required = {
            "algorithm", "problem", "seed", "best", "n_init", "iterations",
            "per_iteration_value", "wall_time", "X", "y", "c",
        }
        missing = sorted(required - set(archive.files))
        if missing:
            raise ValueError(f"NPZ trace missing required arrays: {', '.join(missing)}")

        def scalar(name: str):
            value = np.asarray(archive[name])
            if value.size != 1:
                raise ValueError(f"NPZ {name} must be scalar")
            return value.reshape(()).item()

        actual_algorithm = str(scalar("algorithm"))
        actual_problem = str(scalar("problem"))
        actual_seed = int(scalar("seed"))
        best = float(scalar("best"))
        n_init = int(scalar("n_init"))
        X = np.asarray(archive["X"], dtype=float)
        y = np.asarray(archive["y"], dtype=float)
        c = np.asarray(archive["c"], dtype=float)
        iterations = np.asarray(archive["iterations"])
        best_trace = np.asarray(archive["per_iteration_value"], dtype=float)
        wall_trace = np.asarray(archive["wall_time"], dtype=float)

        if actual_algorithm != algorithm or actual_problem != task_id or actual_seed != seed:
            raise ValueError(
                "NPZ identity mismatch: "
                f"({actual_algorithm}, {actual_problem}, {actual_seed}) != "
                f"({algorithm}, {task_id}, {seed})"
            )
        if not np.isfinite(best):
            raise ValueError("NPZ best value must be finite")
        if X.ndim != 2 or X.shape[0] == 0 or X.shape[1] != dimension:
            raise ValueError(f"NPZ X must have shape (n>0, {dimension}), got {X.shape}")
        if y.ndim == 1:
            y = y.reshape(-1, 1)
        if y.ndim != 2 or y.shape[0] != X.shape[0] or y.shape[1] != num_objectives:
            raise ValueError(
                f"NPZ y must have shape ({X.shape[0]}, {num_objectives}), got {y.shape}"
            )
        if c.ndim != 2 or c.shape[0] != X.shape[0] or c.shape[1] != num_constraints:
            raise ValueError(
                f"NPZ c must have shape ({X.shape[0]}, {num_constraints}), got {c.shape}"
            )
        if not np.isfinite(X).all() or not np.isfinite(y).all() or not np.isfinite(c).all():
            raise ValueError("NPZ X/y/c must contain only finite values")
        if np.any(X < -1.0e-8) or np.any(X > 1.0 + 1.0e-8):
            raise ValueError("NPZ X contains design points outside the normalized unit cube")
        if n_init < 0 or n_init > X.shape[0]:
            raise ValueError("NPZ n_init is outside the observed evaluation history")
        if iterations.ndim != 1 or best_trace.ndim != 1 or wall_trace.ndim != 1:
            raise ValueError("NPZ iteration, best and wall traces must be one-dimensional")
        if not (len(iterations) == len(best_trace) == len(wall_trace)) or len(iterations) == 0:
            raise ValueError("NPZ iteration, best and wall traces have inconsistent lengths")
        if not np.isfinite(best_trace).all() or not np.isfinite(wall_trace).all():
            raise ValueError("NPZ best and wall traces must be finite")
        if np.any(np.diff(wall_trace) < -1.0e-12):
            raise ValueError("NPZ wall-time trace is not monotonic")
        if abs(float(wall_trace[0])) > 1.0e-9:
            raise ValueError("NPZ wall-time trace must start at zero")

        feasible = (
            np.all(c <= 0.0, axis=1)
            if num_constraints > 0
            else np.ones(X.shape[0], dtype=bool)
        )
        return {
            "objective_evaluation_count": int(X.shape[0]),
            "initial_design_count": n_init,
            "objective_observation_count": int(y.shape[0]),
            "objective_count": int(y.shape[1]),
            "constraint_observation_count": int(c.shape[0]),
            "constraint_count": int(c.shape[1]),
            "feasible_evaluation_count": int(feasible.sum()),
            "infeasible_evaluation_count": int((~feasible).sum()),
            "feasible_evaluation_fraction": float(feasible.mean()),
            "best_encoded_maximization_value": best,
            "trace_point_count": int(len(iterations)),
            "trace_final_wall_seconds": float(wall_trace[-1]),
            "trace_final_best": float(best_trace[-1]),
        }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bocode-root", type=Path, required=True)
    parser.add_argument("--result-root", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--package-version", required=True)
    parser.add_argument("--task-id", required=True)
    parser.add_argument("--metadata-sha256", required=True)
    parser.add_argument("--algorithm", required=True)
    parser.add_argument("--seed", type=int, required=True)
    parser.add_argument("--n-init", type=int, required=True)
    parser.add_argument("--iters", type=int, required=True)
    parser.add_argument("--environment", action="append", default=[], metavar="KEY=VALUE")
    args = parser.parse_args()

    root = args.bocode_root.resolve()
    result_root = args.result_root.resolve()
    result_root.mkdir(parents=True, exist_ok=True)
    state = "wrapper_error"
    reason = "wrapper did not complete"
    stdout = b""
    stderr = b""
    artifacts: dict[str, dict[str, Any]] = {}
    metrics: dict[str, Any] = {}
    exit_code = None
    wall_seconds = None
    cpu_user_seconds = None
    cpu_system_seconds = None
    task_metadata = None
    package_version = None
    source_commit = None
    freeze_sha256 = None

    request = {
        "schema": SCHEMA,
        "attempt_id": str(uuid.uuid4()),
        "source_commit": args.source_commit.lower(),
        "package_version_expected": args.package_version,
        "task_id": args.task_id,
        "metadata_sha256_expected": args.metadata_sha256.lower(),
        "algorithm": args.algorithm,
        "seed": args.seed,
        "n_init_requested": args.n_init,
        "iterations_requested": args.iters,
        "environment_overrides": sorted(args.environment),
    }
    request_sha256 = sha256(canonical_json(request))
    run_id = sha256(canonical_json({"request": request_sha256, "attempt_id": request["attempt_id"]}))
    run_root = result_root / run_id
    run_root.mkdir(parents=True, exist_ok=False)

    try:
        if not re.fullmatch(r"[0-9a-fA-F]{40}", args.source_commit):
            raise ValueError("--source-commit must be a full 40-character SHA")
        if not re.fullmatch(r"[0-9a-fA-F]{64}", args.metadata_sha256):
            raise ValueError("--metadata-sha256 must be SHA-256")
        if args.seed < 0 or args.n_init < 0 or args.iters <= 0:
            raise ValueError("seed/n_init/iters contain invalid values")
        if not (root / "examples" / "run_experiments.py").is_file():
            raise ValueError("bocode-root must contain examples/run_experiments.py")

        source_commit, _clean = git_identity(root, args.source_commit)
        sys.path.insert(0, str(root))
        bocode_spec = importlib.util.find_spec("bocode")
        if bocode_spec is None or bocode_spec.origin is None:
            raise ValueError("cannot resolve BOCoDe package from the pinned source tree")
        package_origin = Path(bocode_spec.origin).resolve()
        if not package_origin.is_relative_to(root):
            raise ValueError("imported BOCoDe package does not resolve inside the pinned source tree")
        package_version = importlib.metadata.version("bocode")
        if package_version != args.package_version:
            raise ValueError(
                f"BOCoDe package version mismatch: expected {args.package_version}, got {package_version}"
            )
        resources = importlib.resources.files("bocode.opt_problems_metadata")
        raw_metadata = resources.joinpath(f"{args.task_id}.json").read_bytes()
        raw_metadata_sha = sha256(raw_metadata)
        if raw_metadata_sha != args.metadata_sha256.lower():
            raise ValueError("task raw metadata SHA-256 differs from the admitted catalog")
        task_metadata = json.loads(raw_metadata.decode("utf-8"))
        if task_metadata.get("name") != args.task_id:
            raise ValueError("task metadata name differs from requested task ID")
        dimension = int(task_metadata["dim"])
        num_objectives = int(task_metadata["num_objectives"])
        num_constraints = int(task_metadata.get("num_constraints", 0))
        if dimension <= 0 or num_objectives <= 0 or num_constraints < 0:
            raise ValueError("task metadata dimensions/objectives/constraints are invalid")

        freeze = subprocess.run(
            [sys.executable, "-m", "pip", "freeze", "--all"],
            cwd=root,
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        freeze_sha256 = sha256(freeze.stdout)
        artifacts["environment_freeze"] = write_artifact(run_root, "pip-freeze.txt", freeze.stdout)
        artifacts["environment_freeze_stderr"] = write_artifact(
            run_root, "pip-freeze.stderr", freeze.stderr
        )

        env = os.environ.copy()
        for item in args.environment:
            if "=" not in item:
                raise ValueError("--environment entries must be KEY=VALUE")
            key, value = item.split("=", 1)
            if key not in ENV_ALLOWLIST:
                raise ValueError(
                    f"environment key {key!r} is not in the reviewed allow-list"
                )
            env[key] = value
        environment_projection = {
            key: env[key] for key in ENV_ALLOWLIST if key in env
        }
        for item in args.environment:
            key, value = item.split("=", 1)
            environment_projection[key] = value

        outdir = run_root / "upstream-output"
        outdir.mkdir(parents=True, exist_ok=True)
        command = [
            sys.executable,
            "examples/run_experiments.py",
            "--problems", args.task_id,
            "--algorithms", args.algorithm,
            "--seeds", str(args.seed),
            "--n-init", str(args.n_init),
            "--iters", str(args.iters),
            "--outdir", str(outdir),
        ]

        before = resource.getrusage(resource.RUSAGE_CHILDREN)
        started = time.perf_counter()
        process = subprocess.run(
            command,
            cwd=root,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        wall_seconds = time.perf_counter() - started
        after = resource.getrusage(resource.RUSAGE_CHILDREN)
        cpu_user_seconds = max(0.0, after.ru_utime - before.ru_utime)
        cpu_system_seconds = max(0.0, after.ru_stime - before.ru_stime)
        exit_code = process.returncode
        stdout, stderr = process.stdout, process.stderr
        artifacts["stdout"] = write_artifact(run_root, "stdout.bin", stdout)
        artifacts["stderr"] = write_artifact(run_root, "stderr.bin", stderr)

        summary_path = outdir / "summary_task0.csv"
        npz_path = outdir / f"{args.algorithm}_{args.task_id}_seed{args.seed}.npz"
        if summary_path.is_file():
            artifacts["upstream_summary"] = write_artifact(
                run_root, "summary_task0.csv", summary_path.read_bytes()
            )
        if npz_path.is_file():
            artifacts["upstream_npz"] = write_artifact(
                run_root, "upstream_trace.npz", npz_path.read_bytes()
            )

        if SKIP_RE.search(stdout.decode("utf-8", errors="replace")):
            state = "failed"
            reason = "upstream runner emitted SKIP for the requested trial"
        elif exit_code != 0:
            state = "failed"
            reason = f"upstream process exited with status {exit_code}"
        elif not summary_path.is_file():
            state = "failed"
            reason = "upstream summary CSV is missing"
        elif not npz_path.is_file():
            state = "failed"
            reason = "upstream NPZ trace is missing"
        else:
            try:
                with summary_path.open("r", encoding="utf-8", newline="") as stream:
                    summary_rows = list(csv.DictReader(stream))
                if len(summary_rows) != 1:
                    raise ValueError(f"expected one summary row, got {len(summary_rows)}")
                summary = summary_rows[0]
                if (
                    summary.get("algorithm") != args.algorithm
                    or summary.get("problem") != args.task_id
                    or int(summary.get("seed", "-1")) != args.seed
                ):
                    raise ValueError("upstream summary identity mismatch")
                metrics = validate_npz(
                    npz_path,
                    task_id=args.task_id,
                    algorithm=args.algorithm,
                    seed=args.seed,
                    dimension=dimension,
                    num_objectives=num_objectives,
                    num_constraints=num_constraints,
                )
                summary_best = float(summary["best"])
                if abs(summary_best - metrics["best_encoded_maximization_value"]) > (
                    1.0e-10 * max(abs(summary_best), abs(metrics["best_encoded_maximization_value"]), 1.0)
                ):
                    raise ValueError("summary best value differs from raw NPZ best")
                state = "succeeded"
                reason = "one trial completed and raw objective/constraint history replayed structurally"
            except Exception as error:
                state = "invalid_trace"
                reason = f"{type(error).__name__}: {error}"

        environment_projection = {
            key: env[key] for key in ENV_ALLOWLIST if key in env
        }
        for item in args.environment:
            key, value = item.split("=", 1)
            environment_projection[key] = value

        receipt = {
            **request,
            "run_id": run_id,
            "state": state,
            "reason": reason,
            "source_commit_verified": source_commit,
            "package_version_observed": package_version,
            "python_version": sys.version,
            "task_metadata": {
                "dimension": dimension,
                "num_objectives": num_objectives,
                "num_constraints": num_constraints,
                "input_type": task_metadata.get("input_type"),
                "application": task_metadata.get("application"),
            },
            "environment_projection": environment_projection,
            "environment_freeze_sha256": freeze_sha256,
            "command": command,
            "process_exit_code": exit_code,
            "wall_seconds": wall_seconds,
            "cpu_user_seconds_measured": cpu_user_seconds,
            "cpu_system_seconds_measured": cpu_system_seconds,
            "peak_memory_bytes": None,
            "gpu_seconds": None,
            "energy_joules": None,
            "resource_unknown_reasons": [
                "peak RSS is not measured per-trial by this wrapper",
                "GPU time is not measured by this wrapper",
                "energy is not measured by this wrapper",
            ],
            "metrics": metrics,
            "artifacts": artifacts,
        }
    except Exception as error:
        state = "wrapper_error"
        reason = f"{type(error).__name__}: {error}"
        receipt = {
            **request,
            "run_id": run_id,
            "state": state,
            "reason": reason,
            "source_commit_verified": source_commit,
            "package_version_observed": package_version,
            "python_version": sys.version,
            "environment_freeze_sha256": freeze_sha256,
            "process_exit_code": exit_code,
            "wall_seconds": wall_seconds,
            "cpu_user_seconds_measured": cpu_user_seconds,
            "cpu_system_seconds_measured": cpu_system_seconds,
            "peak_memory_bytes": None,
            "gpu_seconds": None,
            "energy_joules": None,
            "metrics": metrics,
            "artifacts": artifacts,
        }

    receipt["request_sha256"] = request_sha256
    receipt["receipt_sha256"] = sha256(canonical_json(receipt))
    receipt_path = run_root / "receipt.json"
    receipt_path.write_bytes(canonical_json(receipt) + b"\n")

    import io

    row = {
        "schema": SCHEMA,
        "attempt_id": request["attempt_id"],
        "run_id": run_id,
        "request_sha256": request_sha256,
        "state": state,
        "reason": reason,
        "source_commit": receipt.get("source_commit_verified") or args.source_commit.lower(),
        "package_version": receipt.get("package_version_observed") or args.package_version,
        "task_id": args.task_id,
        "metadata_sha256": args.metadata_sha256.lower(),
        "algorithm": args.algorithm,
        "seed": str(args.seed),
        "n_init_requested": str(args.n_init),
        "iterations_requested": str(args.iters),
        "objective_evaluation_count": str(metrics.get("objective_evaluation_count", "")),
        "initial_design_count": str(metrics.get("initial_design_count", "")),
        "feasible_evaluation_count": str(metrics.get("feasible_evaluation_count", "")),
        "infeasible_evaluation_count": str(metrics.get("infeasible_evaluation_count", "")),
        "feasible_evaluation_fraction": str(metrics.get("feasible_evaluation_fraction", "")),
        "process_exit_code": "" if exit_code is None else str(exit_code),
        "wall_seconds": "" if wall_seconds is None else str(wall_seconds),
        "cpu_user_seconds": "" if cpu_user_seconds is None else str(cpu_user_seconds),
        "cpu_system_seconds": "" if cpu_system_seconds is None else str(cpu_system_seconds),
        "peak_memory_state": "unknown",
        "gpu_state": "unknown",
        "energy_state": "unknown",
        "stdout_sha256": artifacts.get("stdout", {}).get("sha256", ""),
        "stderr_sha256": artifacts.get("stderr", {}).get("sha256", ""),
        "npz_sha256": artifacts.get("upstream_npz", {}).get("sha256", ""),
        "summary_sha256": artifacts.get("upstream_summary", {}).get("sha256", ""),
        "environment_freeze_sha256": freeze_sha256 or "",
        "receipt_sha256": receipt["receipt_sha256"],
    }
    buffer = io.StringIO(newline="")
    writer = csv.DictWriter(buffer, fieldnames=list(row.keys()), lineterminator="\n")
    writer.writeheader()
    writer.writerow(row)
    (run_root / "run-row.csv").write_text(buffer.getvalue(), encoding="utf-8", newline="")

    print(json.dumps({
        "run_id": run_id,
        "state": state,
        "reason": reason,
        "receipt": str(receipt_path),
        "receipt_sha256": receipt["receipt_sha256"],
        "objective_evaluation_count": metrics.get("objective_evaluation_count"),
    }, sort_keys=True))
    # Exit 0 means the wrapper successfully wrote a terminal receipt; trial state remains
    # explicit in the receipt. Exit 2 means wrapper_error, so shell pipelines see the problem.
    return 2 if state == "wrapper_error" else 0


if __name__ == "__main__":
    sys.exit(main())
