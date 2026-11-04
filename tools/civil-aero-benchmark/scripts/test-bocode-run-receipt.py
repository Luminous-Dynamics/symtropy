#!/usr/bin/env python3
"""Self-test the independent BOCoDe receipt verifier with synthetic NPZ archives."""
from __future__ import annotations

import csv
import hashlib
import json
import struct
import subprocess
import shutil
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


def npy(
    descr: str, shape: tuple[int, ...], payload: bytes, *, fortran_order: bool = False
) -> bytes:
    shape_text = "()" if not shape else "(" + ", ".join(str(x) for x in shape) + ("," if len(shape) == 1 else "") + ")"
    header = "{'descr': %r, 'fortran_order': %r, 'shape': %s, }" % (
        descr, fortran_order, shape_text
    )
    encoded = header.encode("latin1")
    padding = (16 - ((10 + len(encoded) + 1) % 16)) % 16
    encoded += b" " * padding + b"\n"
    return b"\x93NUMPY\x01\x00" + struct.pack("<H", len(encoded)) + encoded + payload


def make_npz(
    path: Path, wrong_x_shape: bool = False, fortran_x: bool = False,
    wrong_best_history: bool = False,
) -> None:
    x_shape = (2, 2 if wrong_x_shape else 3)
    arrays = {
        "algorithm.npy": npy("|S13", (), b"random_search"),
        "problem.npy": npy("|S7", (), b"Fixture"),
        "seed.npy": npy("<i8", (), struct.pack("<q", 7)),
        "best.npy": npy("<f8", (), struct.pack("<d", 1.25)),
        "n_init.npy": npy("<i8", (), struct.pack("<q", 1)),
        "iterations.npy": npy("<i8", (2,), struct.pack("<qq", 0, 1)),
        "per_iteration_value.npy": npy(
            "<f8", (2,), struct.pack("<dd", 0.5, 1.2 if wrong_best_history else 1.25)
        ),
        "wall_time.npy": npy("<f8", (2,), struct.pack("<dd", 0.0, 0.2)),
        "X.npy": npy(
            "<f8",
            x_shape,
            struct.pack("<" + "d" * (x_shape[0] * x_shape[1]), *([0.25] * (x_shape[0] * x_shape[1]))),
            fortran_order=fortran_x,
        ),
        "y.npy": npy("<f8", (2, 1), struct.pack("<dd", 0.5, 1.25)),
        "c.npy": npy("<f8", (2, 1), struct.pack("<dd", -0.1, -0.2)),
    }
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_STORED) as archive:
        for name, data in arrays.items():
            archive.writestr(name, data)


def artifact(root: Path, name: str, data: bytes) -> dict:
    (root / name).write_bytes(data)
    return {"path": name, "byte_length": len(data), "sha256": sha(data)}


def build_fixture(
    root: Path, wrong_x_shape: bool = False, fortran_x: bool = False,
    wrong_best_history: bool = False,
) -> Path:
    metadata_bytes = (
        b'{"name":"Fixture","dim":3,"num_objectives":1,"num_constraints":1,'
        b'"input_type":"continuous","application":"Engineering"}\n'
    )
    environment_projection = {"PATH": "/usr/bin", "PYTHONNOUSERSITE": "1"}
    command = ["python", "examples/run_experiments.py", "--problems", "Fixture"]
    artifacts = {
        "task_metadata_json": artifact(root, "task-metadata.json", metadata_bytes),
        "stdout": artifact(root, "stdout.bin", b"OK fixture\n"),
        "stderr": artifact(root, "stderr.bin", b""),
        "environment_freeze": artifact(root, "pip-freeze.txt", b"bocode==1.0.2.dev2\n"),
        "environment_freeze_stderr": artifact(root, "pip-freeze.stderr", b""),
        "upstream_summary": artifact(root, "summary.csv", b"algorithm,problem,seed,best,gap,npz\nrandom_search,Fixture,7,1.25,,random_search_Fixture_seed7.npz\n"),
    }
    npz_path = root / "random_search_Fixture_seed7.npz"
    make_npz(npz_path, wrong_x_shape, fortran_x, wrong_best_history)
    artifacts["upstream_npz"] = artifact(root, "random_search_Fixture_seed7.npz", npz_path.read_bytes())

    request = {
        "schema": SCHEMA,
        "attempt_id": "fixture-attempt-0001",
        "source_commit": "1" * 40,
        "package_version_expected": "1.0.2.dev2",
        "task_id": "Fixture",
        "metadata_sha256_expected": sha(metadata_bytes),
        "algorithm": "random_search",
        "seed": 7,
        "n_init_requested": 1,
        "iterations_requested": 1,
        "timeout_seconds": 60.0,
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
            "application": "Engineering",
        },
        "environment_projection": environment_projection,
        "environment_projection_sha256": sha(canonical(environment_projection)),
        "environment_freeze_sha256": sha(b"bocode==1.0.2.dev2\n"),
        "command": command,
        "command_sha256": sha(canonical(command)),
        "timeout_seconds": 60.0,
        "timed_out": False,
        "process_exit_code": 0,
        "wall_seconds": 1.0,
        "cpu_user_seconds_measured": 0.5,
        "cpu_system_seconds_measured": 0.1,
        "peak_memory_bytes": None,
        "gpu_seconds": None,
        "energy_joules": None,
        "metrics": {
            "objective_evaluation_count": 2,
            "initial_design_count": 1,
            "best_encoded_maximization_value": 1.25,
            "constraint_history_state": "available",
            "feasible_evaluation_count": 2,
            "infeasible_evaluation_count": 0,
            "feasible_evaluation_fraction": 1.0,
        },
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
        "metadata_sha256": sha(metadata_bytes),
        "algorithm": "random_search",
        "seed": "7",
        "n_init_requested": "1",
        "iterations_requested": "1",
        "timed_out": "false",
        "objective_evaluation_count": "2",
        "constraint_history_state": "available",
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
        (root / "stdout.bin").write_bytes(b"OK fixture\n")

        wrong = Path(temp) / "wrong-shape"
        wrong.mkdir()
        wrong_receipt = build_fixture(wrong, wrong_x_shape=True)
        shape_result = run_verifier(wrong_receipt)
        assert shape_result.returncode != 0, "NPZ dimension mismatch was not rejected"

        fortran_root = Path(temp) / "fortran-layout"
        fortran_root.mkdir()
        fortran_receipt = build_fixture(fortran_root, fortran_x=True)
        fortran_result = run_verifier(fortran_receipt)
        assert fortran_result.returncode != 0, "Fortran-order NPZ array was not rejected"

        wrong_history_root = Path(temp) / "wrong-best-history"
        wrong_history_root.mkdir()
        wrong_history_receipt = build_fixture(
            wrong_history_root, wrong_best_history=True
        )
        wrong_history_result = run_verifier(wrong_history_receipt)
        assert wrong_history_result.returncode != 0, "NPZ best-history mismatch was not rejected"

        # A campaign must preserve a scheduled trial that has no receipt.
        campaign_root = Path(temp) / "campaign"
        campaign_root.mkdir()
        shutil.copytree(root, campaign_root / "trial-valid")
        expected = Path(temp) / "expected.csv"
        expected.write_text(
            "task_id,algorithm,seed\n"
            "Fixture,random_search,7\n"
            "Missing,random_search,8\n",
            encoding="utf-8",
        )
        ledger = Path(temp) / "campaign-ledger.csv"
        summary = Path(temp) / "campaign-summary.json"
        aggregate = Path(__file__).with_name("aggregate-bocode-campaign.py")
        campaign_result = subprocess.run(
            [
                sys.executable, str(aggregate),
                "--expected", str(expected),
                "--receipts", str(campaign_root),
                "--output", str(ledger),
                "--summary", str(summary),
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            check=False,
        )
        assert campaign_result.returncode == 2, campaign_result.stderr
        campaign_summary = json.loads(summary.read_text(encoding="utf-8"))
        assert campaign_summary["missing_trial_count"] == 1
        assert campaign_summary["campaign_complete"] is False
        with ledger.open("r", encoding="utf-8", newline="") as stream:
            ledger_rows = list(csv.DictReader(stream))
        assert any(row["trial_state"] == "missing" for row in ledger_rows)

        print(
            "bocode_receipt_verifier_selftest=pass "
            "valid_receipt=1 artifact_tamper_rejected=1 npz_shape_tamper_rejected=1 "
            "fortran_order_rejected=1 best_history_tamper_rejected=1 "
            "missing_campaign_trial_retained=1"
        )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"BOCoDe receipt verifier self-test failed: {type(error).__name__}: {error}", file=sys.stderr)
        raise
