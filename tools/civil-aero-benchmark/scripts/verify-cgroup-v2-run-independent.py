#!/usr/bin/env python3
"""Independent verifier for Symtropy's delegated cgroup v2 process-tree run receipt."""
from __future__ import annotations

import csv
import hashlib
import os
from pathlib import Path
import re
import subprocess
import sys
import struct

SCHEMA = "symtropy-cgroup-v2-run/0.1.0"
EVIDENCE = (
    "delegated_cgroup_v2_process_tree_cpu_wall_peak_memory_measured_gpu_energy_unknown_physics_unverified"
)
FIELDS = (
    "schema", "run_id", "cgroup_id", "state", "exit_code", "signal",
    "timed_out", "elapsed_wall_time_ns", "cpu_usage_usec", "cpu_user_usec",
    "cpu_system_usec", "cpu_core_hours_numerator_usec",
    "cpu_core_hours_denominator_usec", "peak_memory_bytes", "oom_event_delta",
    "oom_kill_event_delta", "oom_group_kill_event_delta", "command_sha256",
    "environment_sha256", "resource_evidence_sha256", "artifact_manifest_sha256",
    "stdout_sha256", "stderr_sha256", "cleanup_state", "evidence_level",
    "receipt_sha256",
)
ARTIFACT_HEADER = ("role", "path", "byte_length", "sha256")
REQUIRED_ROLES = {
    "command_manifest", "environment_manifest", "resource_evidence",
    "start_cpu_stat", "start_memory_peak", "start_memory_events", "start_cgroup_events",
    "end_cpu_stat", "end_memory_peak", "end_memory_events", "end_cgroup_events",
    "cgroup2_mountinfo", "stdout", "stderr",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def append_string(buffer: bytearray, value: str) -> None:
    encoded = value.encode("utf-8")
    require(len(encoded) < (1 << 64), "canonical field exceeds u64")
    buffer.extend(struct.pack("<Q", len(encoded)))
    buffer.extend(encoded)


def receipt_digest(row: dict[str, str]) -> str:
    payload = bytearray()
    for name in FIELDS[:-1]:
        append_string(payload, name)
        append_string(payload, row[name])
    return hashlib.sha256(payload).hexdigest()


def valid_sha(value: str) -> bool:
    return re.fullmatch(r"[0-9a-f]{64}", value) is not None


def uint(value: str, name: str) -> int:
    require(value.isdigit(), f"{name} must be a non-negative integer")
    return int(value)


def verify_artifact_inventory(root: Path) -> dict[str, dict[str, str]]:
    manifest_path = root / "artifacts.csv"
    require(manifest_path.is_file(), "artifacts.csv is missing")
    raw = manifest_path.read_bytes()
    with manifest_path.open("r", encoding="utf-8", newline="") as stream:
        reader = csv.DictReader(stream)
        require(reader.fieldnames == list(ARTIFACT_HEADER), "artifact manifest header mismatch")
        rows = list(reader)
    by_role: dict[str, dict[str, str]] = {}
    seen_paths = set()
    for item in rows:
        role, relative = item["role"], item["path"]
        require(role not in by_role, f"duplicate artifact role {role}")
        require(relative not in seen_paths, f"duplicate artifact path {relative}")
        seen_paths.add(relative)
        candidate_relative = Path(relative)
        require(not candidate_relative.is_absolute(), f"{role} path must be relative")
        require(
            all(component not in ("", ".", "..") for component in candidate_relative.parts),
            f"{role} path contains traversal or non-normal components",
        )
        candidate = root / candidate_relative
        info = os.lstat(candidate)
        require(not os.path.islink(candidate), f"{role} must not be a symlink")
        require(os.path.isfile(candidate), f"{role} is not a regular file")
        data = candidate.read_bytes()
        byte_length = uint(item["byte_length"], f"{role}.byte_length")
        digest = item["sha256"]
        require(len(data) == byte_length, f"{role} artifact byte length mismatch")
        require(valid_sha(digest), f"{role} artifact SHA-256 is malformed")
        require(hashlib.sha256(data).hexdigest() == digest, f"{role} artifact SHA-256 mismatch")
        by_role[role] = item
    require(REQUIRED_ROLES.issubset(set(by_role)), "artifact inventory is missing mandatory files")
    require(not ("artifact_manifest" in by_role), "artifacts.csv must not hash itself recursively")
    return by_role


def read_resource_csv(root: Path, resource_item: dict[str, str]) -> tuple[dict[str, str], dict[str, str]]:
    resource_path = root / resource_item["path"]
    with resource_path.open("r", encoding="utf-8", newline="") as stream:
        reader = csv.DictReader(stream)
        rows = list(reader)
    require(len(rows) == 3, "resource evidence CSV must have start, end and report records")
    records = {row["record_type"]: row for row in rows}
    require(set(records) == {"start", "end", "report"}, "resource evidence record types are not exact")
    return records["start"], records["end"], records["report"]


def verify_manifest_file(root: Path, role: str, digest_name: str, artifact: dict[str, str], marker: str) -> None:
    data = (root / artifact["path"]).read_text(encoding="utf-8")
    match = re.search(rf"(?m)^{re.escape(marker)}=([0-9a-f]{{64}})$", data)
    require(match is not None, f"{role} does not retain its canonical identity digest")
    require(match.group(1) == digest_name, f"{role} identity digest differs from receipt")


def verify(receipt_path: Path) -> dict[str, object]:
    root = receipt_path.resolve(strict=True).parent
    with receipt_path.open("r", encoding="utf-8", newline="") as stream:
        reader = csv.DictReader(stream)
        require(reader.fieldnames == list(FIELDS), "run receipt header mismatch")
        rows = list(reader)
    require(len(rows) == 1, "run receipt must contain exactly one row")
    row = rows[0]
    require(all(value is not None for value in row.values()), "run receipt contains missing fields")
    require(row["schema"] == SCHEMA, "unsupported cgroup run receipt schema")
    require(re.fullmatch(r"[A-Za-z0-9_-]{1,64}", row["run_id"]) is not None, "run_id malformed")
    require(row["state"] in {"succeeded", "failed", "timed_out", "cleanup_failed"},
            "unsupported cgroup run state")
    require(row["evidence_level"] == EVIDENCE, "evidence ceiling mismatch")
    require(row["timed_out"] in {"true", "false"}, "timed_out must be boolean")
    timed_out = row["timed_out"] == "true"
    require(row["cleanup_state"] in {"removed", "retained"}, "cleanup_state malformed")
    require(row["cgroup_id"].strip() != "", "cgroup identity is missing")

    for name in (
        "command_sha256", "environment_sha256", "resource_evidence_sha256",
        "artifact_manifest_sha256", "stdout_sha256", "stderr_sha256",
    ):
        require(valid_sha(row[name]), f"{name} is malformed")
    require(valid_sha(row["receipt_sha256"]), "receipt_sha256 malformed")
    require(receipt_digest(row) == row["receipt_sha256"], "canonical receipt digest mismatch")

    artifact_manifest = root / "artifacts.csv"
    artifact_manifest_bytes = artifact_manifest.read_bytes()
    require(
        hashlib.sha256(artifact_manifest_bytes).hexdigest() == row["artifact_manifest_sha256"],
        "artifact manifest SHA-256 differs from receipt",
    )
    artifacts = verify_artifact_inventory(root)
    require(artifacts["stdout"]["sha256"] == row["stdout_sha256"], "stdout digest differs from receipt")
    require(artifacts["stderr"]["sha256"] == row["stderr_sha256"], "stderr digest differs from receipt")
    require(artifacts["resource_evidence"]["sha256"] == row["resource_evidence_sha256"],
            "resource evidence digest differs from receipt")

    verify_manifest_file(
        root, "command_manifest", row["command_sha256"],
        artifacts["command_manifest"], "command_identity_sha256",
    )
    verify_manifest_file(
        root, "environment_manifest", row["environment_sha256"],
        artifacts["environment_manifest"], "environment_identity_sha256",
    )

    start, end, report = read_resource_csv(root, artifacts["resource_evidence"])
    require(start["cgroup_id"] == row["cgroup_id"] and end["cgroup_id"] == row["cgroup_id"],
            "resource evidence cgroup identity differs from receipt")
    require(start["memory_peak_reader_id"] == end["memory_peak_reader_id"],
            "memory.peak file descriptor identity changed")
    require(start["memory_peak_reader_flags"] == end["memory_peak_reader_flags"],
            "memory.peak file descriptor flags changed")
    require(start["cgroup2_mount_options_sha256"] == end["cgroup2_mount_options_sha256"],
            "cgroup2 mount identity changed")

    for side in ("start", "end"):
        for counter in ("cpu_stat", "memory_peak", "memory_events", "cgroup_events"):
            role = f"{side}_{counter}"
            item = artifacts[role]
            content = (root / item["path"]).read_bytes()
            csv_digest = start[f"{counter}_sha256"] if side == "start" else end[f"{counter}_sha256"]
            require(hashlib.sha256(content).hexdigest() == csv_digest,
                    f"{role} raw bytes differ from resource evidence snapshot")
    mount_bytes = (root / artifacts["cgroup2_mountinfo"]["path"]).read_bytes()
    require(hashlib.sha256(mount_bytes).hexdigest() == start["cgroup2_mount_options_sha256"],
            "mountinfo bytes differ from snapshot mount identity")

    resource_verifier = Path(__file__).with_name("verify-cgroup-v2-resource-independent.py")
    verified = subprocess.run(
        [sys.executable, str(resource_verifier), str(root / artifacts["resource_evidence"]["path"])],
        capture_output=True, text=True, check=False,
    )
    require(verified.returncode == 0,
            "independent resource oracle rejected bundle: " + (verified.stderr or verified.stdout).strip())

    for receipt_field, report_field in (
        ("elapsed_wall_time_ns", "elapsed_wall_time_ns"),
        ("cpu_usage_usec", "cpu_usage_delta_usec"),
        ("cpu_user_usec", "cpu_user_delta_usec"),
        ("cpu_system_usec", "cpu_system_delta_usec"),
        ("cpu_core_hours_numerator_usec", "cpu_core_hours_numerator_usec"),
        ("cpu_core_hours_denominator_usec", "cpu_core_hours_denominator_usec"),
        ("peak_memory_bytes", "peak_memory_bytes"),
        ("oom_event_delta", "oom_event_delta"),
        ("oom_kill_event_delta", "oom_kill_event_delta"),
    ):
        require(row[receipt_field] == report[report_field],
                f"receipt {receipt_field} differs from resource report")
    require(row["oom_group_kill_event_delta"] == report["oom_group_kill_event_delta"],
            "receipt oom_group_kill delta differs from resource report")

    exit_code = None if row["exit_code"] == "" else int(row["exit_code"])
    signal_number = None if row["signal"] == "" else int(row["signal"])
    if row["state"] == "succeeded":
        require(exit_code == 0 and signal_number is None and not timed_out,
                "succeeded state requires zero exit, no signal and no timeout")
        require(row["cleanup_state"] == "removed", "succeeded state requires removed cgroup")
    elif row["state"] == "failed":
        require(not timed_out and (exit_code not in (None, 0) or signal_number is not None),
                "failed state requires nonzero exit or signal")
    elif row["state"] == "timed_out":
        require(timed_out, "timed_out state requires timed_out=true")
    elif row["state"] == "cleanup_failed":
        require(exit_code == 0 and signal_number is None and not timed_out,
                "cleanup_failed state requires a successful process and no timeout")
        require(row["cleanup_state"] == "retained", "cleanup_failed requires retained cgroup")

    return {
        "run_id": row["run_id"],
        "state": row["state"],
        "receipt_sha256": row["receipt_sha256"],
        "artifact_count": len(artifacts),
        "cpu_usage_usec": uint(row["cpu_usage_usec"], "cpu_usage_usec"),
        "peak_memory_bytes": uint(row["peak_memory_bytes"], "peak_memory_bytes"),
        "gpu_energy_state": "unknown",
    }


def main() -> None:
    require(len(sys.argv) == 2, "usage: verify-cgroup-v2-run-independent.py RUN_RECEIPT.csv")
    result = verify(Path(sys.argv[1]))
    print("independent_cgroup_v2_run_receipt=pass " + " ".join(f"{key}={value}" for key, value in result.items()))


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"independent cgroup v2 run receipt verifier failed: {error}", file=sys.stderr)
        raise
