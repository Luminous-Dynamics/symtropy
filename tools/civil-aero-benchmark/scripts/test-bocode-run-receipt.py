#!/usr/bin/env python3
"""Self-test the independent BOCoDe receipt verifier with synthetic NPZ archives."""
from __future__ import annotations

import csv
import hashlib
import json
import struct
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

VERIFY = Path(__file__).with_name("verify-bocode-run-receipt.py")
SCHEMA = "bocode-single-trial-receipt/0.1.0"


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical(value) -> bytes:
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":"), allow_nan=False).encode()


def npy(descr: str, shape: tuple[int, ...], payload: bytes) -> bytes:
    shape_text = "()" if not shape else "(" + ", ".join(str(x) for x in shape) + ("," if len(shape) == 1 else "") + ")"
    header = "{'descr': %r, 'fortran_order': False, 'shape': %s, }" % (descr, shape_text)
    encoded = header.encode("latin1")
    padding = (16 - ((10 + len(encoded) + 1) % 16)) % 16
    encoded += b" " * padding + b"\n"
    return b"\x93NUMPY\x01\x00" + struct.pack("<H", len(encoded)) + encoded + payload


def make_npz(path: Path, wrong_x_shape: bool = False) -> None:
    x_shape = (2, 2 if wrong_x_shape else 3)
    arrays = {
        "algorithm.npy": npy("|S9", (), b"random___"),
        "problem.npy": npy("|S7", (), b"fixture"),
        "seed.npy": npy("<i8", (), struct.pack("<q", 7)),
        "best.npy": npy("<f8", (), struct.pack("<d", 1.25)),
        "n_init.npy": npy("<i8", (), struct.pack("<q", 1)),
        "iterations.npy": npy("<i8", (2,), struct.pack("<qq", 0, 1)),
        "per_iteration_value.npy": npy("<f8", (2,), struct.pack("<dd", 0.5, 1.25)),
        "wall_time.npy": npy("<f8", (2,), struct.pack("<dd", 0.0, 0.2)),
        "X.npy": npy("<f8", x_shape, struct.pack("<" + "d" * (x_shape[0] * x_shape[1]), *([0.25] * (x_shape[0] * x_shape[1])))),
        "y.npy": npy("<f8", (2, 1), struct.pack("<dd", 0.5, 1.25)),
        "c.npy": npy("<f8", (2, 1), struct.pack("<dd", -0.1, -0.2)),
    }
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_STORED) as archive:
        for name, data in arrays.items():
            archive.writestr(name, data)


def artifact(root: Path, name: str, data: bytes) -> dict:
    (root / name).write_bytes(data)
    return {"path": name, "byte_length": len(data), "sha256": sha(data)}


def build_fixture(root: Path, wrong_x_shape: bool = False) -> Path:
    artifacts = {
        "stdout": artifact(root, "stdout.bin", b"OK fixture\n"),
        "stderr": artifact(root, "stderr.bin", b""),
        "environment_freeze": artifact(root, "pip-freeze.txt", b"bocode==1.0.2.dev2\n"),
        "environment_freeze_stderr": artifact(root, "pip-freeze.stderr", b""),
        "upstream_summary": artifact(root, "summary.csv", b"algorithm,problem,seed,best,gap,npz\nrandom_search,Fixture,7,1.25,,trace.npz\n"),
    }
    npz_path = root / "trace.npz"
    make_npz(npz_path, wrong_x_shape)
    artifacts["upstream_npz"] = artifact(root, "trace.npz", npz_path.read_bytes())

    request = {
        "schema": SCHEMA,
        "attempt_id": "fixture-attempt-0001",
        "source_commit": "1" * 40,
        "package_version_expected": "1.0.2.dev2",
        "task_id": "Fixture",
        "metadata_sha256_expected": "2" * 64,
        "algorithm": "random_search",
        "seed": 7,
        "n_init_requested": 1,
        "iterations_requested": 1,
        "environment_overrides": [],
    }
    request_sha = sha(canonical(request))
    run_id = sha(canonical({"request": request_sha, "attempt_id": request["attempt_id"]}))
    receipt = {
        **request,
        "run_id": run_id,
        "request_sha256": request_sha,
        "state": "succeeded",
        "reason": "synthetic fixture",
        "source_commit_verified": "1" * 40,
        "package_version_observed": "1.0.2.dev2",
        "python_version": "3.12.fixture",
        "task_metadata": {
            "dimension": 3,
            "num_objectives": 1,
            "num_constraints": 1,
            "input_type": "continuous",
            "application": "engineering",
        },
        "environment_projection": {},
        "environment_freeze_sha256": sha(b"bocode==1.0.2.dev2\n"),
        "command": ["python", "examples/run_experiments.py"],
        "process_exit_code": 0,
        "wall_seconds": 1.0,
        "cpu_user_seconds_measured": 0.5,
        "cpu_system_seconds_measured": 0.1,
        "peak_memory_bytes": None,
        "gpu_seconds": None,
        "energy_joules": None,
        "metrics": {"objective_evaluation_count": 2},
        "artifacts": artifacts,
    }
    receipt["receipt_sha256"] = sha(canonical(receipt))
    receipt_path = root / "receipt.json"
    receipt_path.write_bytes(canonical(receipt) + b"\n")
    row = {
        "schema": SCHEMA,
        "attempt_id": request["attempt_id"],
        "run_id": run_id,
        "request_sha256": request_sha,
        "state": "succeeded",
        "reason": "synthetic fixture",
        "source_commit": "1" * 40,
        "package_version": "1.0.2.dev2",
        "task_id": "Fixture",
        "metadata_sha256": "2" * 64,
        "algorithm": "random_search",
        "seed": "7",
        "n_init_requested": "1",
        "iterations_requested": "1",
        "objective_evaluation_count": "2",
        "initial_design_count": "1",
        "feasible_evaluation_count": "2",
        "infeasible_evaluation_count": "0",
        "feasible_evaluation_fraction": "1.0",
        "process_exit_code": "0",
        "wall_seconds": "1.0",
        "cpu_user_seconds": "0.5",
        "cpu_system_seconds": "0.1",
        "peak_memory_state": "unknown",
        "gpu_state": "unknown",
        "energy_state": "unknown",
        "stdout_sha256": artifacts["stdout"]["sha256"],
        "stderr_sha256": artifacts["stderr"]["sha256"],
        "npz_sha256": artifacts["upstream_npz"]["sha256"],
        "summary_sha256": artifacts["upstream_summary"]["sha256"],
        "environment_freeze_sha256": receipt["environment_freeze_sha256"],
        "receipt_sha256": receipt["receipt_sha256"],
    }
    with (root / "run-row.csv").open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(row.keys()), lineterminator="\n")
        writer.writeheader()
        writer.writerow(row)
    return receipt_path


def run_verifier(path: Path) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(VERIFY), str(path)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="bocode-receipt-selftest-") as temp:
        root = Path(temp) / "valid"
        root.mkdir()
        receipt = build_fixture(root)
        valid = run_verifier(receipt)
        assert valid.returncode == 0, valid.stderr

        # Artifact tampering must fail the retained length/hash binding.
        (root / "stdout.bin").write_bytes(b"tampered")
        tampered = run_verifier(receipt)
        assert tampered.returncode != 0, "artifact tampering was not rejected"

        wrong = Path(temp) / "wrong-shape"
        wrong.mkdir()
        wrong_receipt = build_fixture(wrong, wrong_x_shape=True)
        shape_result = run_verifier(wrong_receipt)
        assert shape_result.returncode != 0, "NPZ dimension mismatch was not rejected"

        print(
            "bocode_receipt_verifier_selftest=pass "
            "valid_receipt=1 artifact_tamper_rejected=1 npz_shape_tamper_rejected=1"
        )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"BOCoDe receipt verifier self-test failed: {type(error).__name__}: {error}", file=sys.stderr)
        raise
