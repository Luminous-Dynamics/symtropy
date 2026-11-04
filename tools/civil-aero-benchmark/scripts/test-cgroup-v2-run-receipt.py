#!/usr/bin/env python3
"""Synthetic self-test for the cgroup v2 run receipt verifier; no privileged cgroup access."""
from __future__ import annotations

import csv
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

VERIFY = Path(__file__).with_name("verify-cgroup-v2-run-independent.py")
RESOURCE_VERIFY = Path(__file__).with_name("verify-cgroup-v2-resource-independent.py")
SNAPSHOT_SCHEMA = "linux-cgroup-v2-resource-evidence/0.1.0"
SNAPSHOT_EVIDENCE = (
    "raw_cgroup_v2_counter_snapshots_bound_to_process_tree_measure_cpu_and_peak_memory_not_gpu_or_energy"
)
RUN_SCHEMA = "symtropy-cgroup-v2-run/0.1.0"
RUN_EVIDENCE = (
    "delegated_cgroup_v2_process_tree_cpu_wall_peak_memory_measured_gpu_energy_unknown_physics_unverified"
)
RUN_FIELDS = (
    "schema", "run_id", "cgroup_id", "state", "exit_code", "signal",
    "timed_out", "elapsed_wall_time_ns", "cpu_usage_usec", "cpu_user_usec",
    "cpu_system_usec", "cpu_core_hours_numerator_usec",
    "cpu_core_hours_denominator_usec", "peak_memory_bytes", "oom_event_delta",
    "oom_kill_event_delta", "oom_group_kill_event_delta", "command_sha256",
    "environment_sha256", "resource_evidence_sha256", "artifact_manifest_sha256",
    "stdout_sha256", "stderr_sha256", "cleanup_state", "evidence_level",
    "receipt_sha256",
)
RESOURCE_HEADER = (
    "record_type,cgroup_id,memory_peak_reader_id,memory_peak_reader_flags,"
    "cgroup2_mount_options_sha256,monotonic_time_ns,cpu_usage_usec,cpu_user_usec,"
    "cpu_system_usec,memory_peak_bytes,oom_events,oom_kill_events,"
    "oom_group_kill_events,populated,frozen,cpu_stat_sha256,memory_peak_sha256,"
    "memory_events_sha256,cgroup_events_sha256,snapshot_sha256,schema,"
    "start_monotonic_time_ns,end_monotonic_time_ns,elapsed_wall_time_ns,"
    "cpu_usage_delta_usec,cpu_user_delta_usec,cpu_system_delta_usec,"
    "cpu_core_hours_numerator_usec,cpu_core_hours_denominator_usec,"
    "peak_memory_bytes,oom_event_delta,oom_kill_event_delta,"
    "oom_group_kill_event_delta,start_snapshot_sha256,end_snapshot_sha256,"
    "evidence_level,report_sha256"
).split(",")


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def add_u64(buf: bytearray, value: int) -> None:
    buf.extend(value.to_bytes(8, "little"))


def add_string(buf: bytearray, value: str) -> None:
    raw = value.encode()
    add_u64(buf, len(raw))
    buf.extend(raw)


def snapshot_record(side: str, root: Path, mount_line: bytes):
    cgroup_id = "fixture-cgroup:dev=0:ino=101"
    reader_id = "dev=0:ino=202"
    mount_digest = sha(mount_line)
    if side == "start":
        values = {
            "time": 1_000_000_000, "usage": 1_000_000, "user": 800_000,
            "system": 200_000, "peak": 0, "oom": 0, "kill": 0,
        }
    else:
        values = {
            "time": 11_000_000_000, "usage": 6_000_000, "user": 4_800_000,
            "system": 1_200_000, "peak": 48_000_000, "oom": 1, "kill": 1,
        }
    cpu = (
        f'usage_usec {values["usage"]}\n'
        f'user_usec {values["user"]}\n'
        f'system_usec {values["system"]}\n'
        "nr_periods 10\n"
    ).encode()
    peak = f'{values["peak"]}\n'.encode()
    events = (
        f'low 0\nhigh 0\nmax 0\noom {values["oom"]}\n'
        f'oom_kill {values["kill"]}\noom_group_kill 0\n'
    ).encode()
    cgroup_events = b"populated 0\nfrozen 0\n"
    raw = {
        "cpu_stat": cpu, "memory_peak": peak,
        "memory_events": events, "cgroup_events": cgroup_events,
    }
    hashes = {name: sha(data) for name, data in raw.items()}
    for name, data in raw.items():
        (root / f"{side}-{name.replace('_', '-')}.txt").write_bytes(data)

    canonical = bytearray()
    for value in (
        SNAPSHOT_SCHEMA, cgroup_id, hashes["cpu_stat"], hashes["memory_peak"],
        hashes["memory_events"], hashes["cgroup_events"], reader_id, mount_digest,
    ):
        add_string(canonical, value)
    for value in (
        values["time"], values["usage"], values["user"], values["system"],
        values["peak"], values["oom"], values["kill"],
    ):
        add_u64(canonical, value)
    canonical.extend(b"\x01")
    add_u64(canonical, 0)
    canonical.extend(b"\x00\x00")
    snapshot_hash = sha(bytes(canonical))
    row = {
        "record_type": side,
        "cgroup_id": cgroup_id,
        "memory_peak_reader_id": reader_id,
        "memory_peak_reader_flags": "0",
        "cgroup2_mount_options_sha256": mount_digest,
        "monotonic_time_ns": str(values["time"]),
        "cpu_usage_usec": str(values["usage"]),
        "cpu_user_usec": str(values["user"]),
        "cpu_system_usec": str(values["system"]),
        "memory_peak_bytes": str(values["peak"]),
        "oom_events": str(values["oom"]),
        "oom_kill_events": str(values["kill"]),
        "oom_group_kill_events": "0",
        "populated": "false",
        "frozen": "false",
        "cpu_stat_sha256": hashes["cpu_stat"],
        "memory_peak_sha256": hashes["memory_peak"],
        "memory_events_sha256": hashes["memory_events"],
        "cgroup_events_sha256": hashes["cgroup_events"],
        "snapshot_sha256": snapshot_hash,
    }
    return row, hashes, values


def resource_report(start: dict, end: dict) -> dict:
    elapsed = int(end["monotonic_time_ns"]) - int(start["monotonic_time_ns"])
    usage = int(end["cpu_usage_usec"]) - int(start["cpu_usage_usec"])
    user = int(end["cpu_user_usec"]) - int(start["cpu_user_usec"])
    system = int(end["cpu_system_usec"]) - int(start["cpu_system_usec"])
    oom = int(end["oom_events"]) - int(start["oom_events"])
    oom_kill = int(end["oom_kill_events"]) - int(start["oom_kill_events"])
    group = int(end["oom_group_kill_events"]) - int(start["oom_group_kill_events"])
    canonical = bytearray()
    for value in (
        SNAPSHOT_SCHEMA, start["cgroup_id"], start["snapshot_sha256"],
        end["snapshot_sha256"], SNAPSHOT_EVIDENCE,
    ):
        add_string(canonical, value)
    for value in (
        int(start["monotonic_time_ns"]), int(end["monotonic_time_ns"]), elapsed,
        usage, user, system, int(end["memory_peak_bytes"]), oom, oom_kill,
    ):
        add_u64(canonical, value)
    add_u64(canonical, usage)
    add_u64(canonical, 3_600_000_000)
    canonical.extend(b"\x01")
    add_u64(canonical, group)
    return {
        "record_type": "report",
        "cgroup_id": start["cgroup_id"],
        "schema": SNAPSHOT_SCHEMA,
        "start_monotonic_time_ns": start["monotonic_time_ns"],
        "end_monotonic_time_ns": end["monotonic_time_ns"],
        "elapsed_wall_time_ns": str(elapsed),
        "cpu_usage_delta_usec": str(usage),
        "cpu_user_delta_usec": str(user),
        "cpu_system_delta_usec": str(system),
        "cpu_core_hours_numerator_usec": str(usage),
        "cpu_core_hours_denominator_usec": "3600000000",
        "peak_memory_bytes": end["memory_peak_bytes"],
        "oom_event_delta": str(oom),
        "oom_kill_event_delta": str(oom_kill),
        "oom_group_kill_event_delta": str(group),
        "start_snapshot_sha256": start["snapshot_sha256"],
        "end_snapshot_sha256": end["snapshot_sha256"],
        "evidence_level": SNAPSHOT_EVIDENCE,
        "report_sha256": sha(bytes(canonical)),
    }


def build_bundle(root: Path) -> Path:
    root.mkdir(parents=True)
    mount = (
        b"36 25 0:31 / /sys/fs/cgroup rw,nosuid,nodev,noexec,relatime "
        b"- cgroup2 cgroup rw,nsdelegate\n"
    )
    (root / "cgroup2-mountinfo.txt").write_bytes(mount)

    start, _, _ = snapshot_record("start", root, mount)
    end, _, _ = snapshot_record("end", root, mount)
    report = resource_report(start, end)
    resource_rows = [start, end, report]
    with (root / "resource-evidence.csv").open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=RESOURCE_HEADER, lineterminator="\n")
        writer.writeheader()
        writer.writerows(resource_rows)
    resource_bytes = (root / "resource-evidence.csv").read_bytes()

    program, cwd = "/usr/bin/true", "/tmp"
    argv = ["--fixture"]
    command_canonical = bytearray()
    add_string(command_canonical, program)
    add_string(command_canonical, cwd)
    add_u64(command_canonical, len(argv))
    for arg in argv:
        add_string(command_canonical, arg)
    command_sha = sha(bytes(command_canonical))
    command_text = (
        "schema=symtropy-command-identity/0.1.0\n"
        f"program_sha256={sha(program.encode())}\n"
        f"cwd_sha256={sha(cwd.encode())}\n"
        f"arg_0_bytes={len(argv[0])} arg_0_sha256={sha(argv[0].encode())}\n"
        f"command_identity_sha256={command_sha}\n"
    ).encode()
    (root / "command-manifest.txt").write_bytes(command_text)

    environment = {"OMP_NUM_THREADS": "2", "PATH": "/usr/bin"}
    environment_canonical = bytearray()
    add_u64(environment_canonical, len(environment))
    for key, value in sorted(environment.items()):
        add_string(environment_canonical, key)
        add_string(environment_canonical, value)
    environment_sha = sha(bytes(environment_canonical))
    environment_lines = ["schema=symtropy-environment-identity/0.1.0"]
    for key, value in sorted(environment.items()):
        environment_lines.append(
            f"key_utf8_hex={key.encode().hex()} value_bytes={len(value)} value_sha256={sha(value.encode())}"
        )
    environment_lines.append(f"environment_identity_sha256={environment_sha}")
    (root / "environment-manifest.txt").write_text("\n".join(environment_lines) + "\n")

    (root / "stdout.bin").write_bytes(b"fixture target completed\n")
    (root / "stderr.bin").write_bytes(b"")

    inventory = [
        ("command_manifest", "command-manifest.txt"),
        ("environment_manifest", "environment-manifest.txt"),
        ("resource_evidence", "resource-evidence.csv"),
        ("cgroup2_mountinfo", "cgroup2-mountinfo.txt"),
        ("stdout", "stdout.bin"),
        ("stderr", "stderr.bin"),
    ]
    for side in ("start", "end"):
        for name in ("cpu-stat", "memory-peak", "memory-events", "cgroup-events"):
            inventory.append((f"{side}_{name.replace('-', '_')}", f"{side}-{name}.txt"))

    artifact_rows = []
    for role, path in inventory:
        data = (root / path).read_bytes()
        artifact_rows.append({
            "role": role, "path": path, "byte_length": str(len(data)), "sha256": sha(data)
        })
    with (root / "artifacts.csv").open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(
            stream, fieldnames=("role", "path", "byte_length", "sha256"),
            quoting=csv.QUOTE_ALL, lineterminator="\n",
        )
        writer.writeheader()
        writer.writerows(artifact_rows)

    fields = {
        "schema": RUN_SCHEMA,
        "run_id": "fixture-run-001",
        "cgroup_id": start["cgroup_id"],
        "state": "succeeded",
        "exit_code": "0",
        "signal": "",
        "timed_out": "false",
        "elapsed_wall_time_ns": report["elapsed_wall_time_ns"],
        "cpu_usage_usec": report["cpu_usage_delta_usec"],
        "cpu_user_usec": report["cpu_user_delta_usec"],
        "cpu_system_usec": report["cpu_system_delta_usec"],
        "cpu_core_hours_numerator_usec": report["cpu_core_hours_numerator_usec"],
        "cpu_core_hours_denominator_usec": report["cpu_core_hours_denominator_usec"],
        "peak_memory_bytes": report["peak_memory_bytes"],
        "oom_event_delta": report["oom_event_delta"],
        "oom_kill_event_delta": report["oom_kill_event_delta"],
        "oom_group_kill_event_delta": report["oom_group_kill_event_delta"],
        "command_sha256": command_sha,
        "environment_sha256": environment_sha,
        "resource_evidence_sha256": sha(resource_bytes),
        "artifact_manifest_sha256": sha((root / "artifacts.csv").read_bytes()),
        "stdout_sha256": sha((root / "stdout.bin").read_bytes()),
        "stderr_sha256": sha((root / "stderr.bin").read_bytes()),
        "cleanup_state": "removed",
        "evidence_level": RUN_EVIDENCE,
    }
    canonical_receipt = bytearray()
    for name in FIELDS[:-1]:
        add_string(canonical_receipt, name)
        add_string(canonical_receipt, fields[name])
    fields["receipt_sha256"] = sha(bytes(canonical_receipt))
    with (root / "run-receipt.csv").open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=FIELDS, quoting=csv.QUOTE_ALL, lineterminator="\n")
        writer.writeheader()
        writer.writerow(fields)
    return root / "run-receipt.csv"


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="symtropy-cgroup-run-receipt-") as temporary:
        root = Path(temporary) / "success"
        receipt = build_bundle(root)
        verified = subprocess.run(
            [sys.executable, str(VERIFY), str(receipt)],
            capture_output=True, text=True, check=False,
        )
        require(verified.returncode == 0,
                "valid synthetic receipt rejected: " + (verified.stderr or verified.stdout))

        stdout = root / "stdout.bin"
        stdout.write_bytes(b"tampered\n")
        rejected = subprocess.run(
            [sys.executable, str(VERIFY), str(receipt)],
            capture_output=True, text=True, check=False,
        )
        require(rejected.returncode != 0, "tampered stdout artifact was accepted")
        print("cgroup_v2_run_receipt_selftest=pass valid_receipt=accepted tampered_artifact=rejected")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"cgroup v2 run receipt self-test failed: {error}", file=sys.stderr)
        raise
