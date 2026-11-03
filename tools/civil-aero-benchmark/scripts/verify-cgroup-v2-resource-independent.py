#!/usr/bin/env python3
"""Independent stdlib oracle for cgroup-v2 process-tree resource snapshots."""
from __future__ import annotations

import csv
import hashlib
import struct
import sys
from pathlib import Path

SCHEMA = "linux-cgroup-v2-resource-evidence/0.1.0"
EVIDENCE = (
    "raw_cgroup_v2_counter_snapshots_bound_to_process_tree_measure_cpu_and_peak_memory_not_gpu_or_energy"
)
HEADER = (
    "record_type,cgroup_id,monotonic_time_ns,cpu_usage_usec,cpu_user_usec,cpu_system_usec,"
    "memory_peak_bytes,oom_events,oom_kill_events,oom_group_kill_events,populated,frozen,"
    "cpu_stat_sha256,memory_peak_sha256,memory_events_sha256,cgroup_events_sha256,"
    "snapshot_sha256,schema,start_monotonic_time_ns,end_monotonic_time_ns,elapsed_wall_time_ns,"
    "cpu_usage_delta_usec,cpu_user_delta_usec,cpu_system_delta_usec,cpu_core_hours_numerator_usec,"
    "cpu_core_hours_denominator_usec,peak_memory_bytes,oom_event_delta,oom_kill_event_delta,oom_group_kill_event_delta,"
    "start_snapshot_sha256,end_snapshot_sha256,evidence_level,report_sha256"
).split(",")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def read_rows(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as stream:
        reader = csv.DictReader(stream)
        require(reader.fieldnames == HEADER, "cgroup resource CSV header mismatch")
        return list(reader)


def add_u64(buffer: bytearray, value: int) -> None:
    require(0 <= value <= (1 << 64) - 1, "u64 field out of range")
    buffer.extend(struct.pack("<Q", value))


def add_string(buffer: bytearray, value: str) -> None:
    raw = value.encode("utf-8")
    add_u64(buffer, len(raw))
    buffer.extend(raw)


def add_u128(buffer: bytearray, value: int) -> None:
    require(0 <= value <= (1 << 128) - 1, "u128 field out of range")
    buffer.extend(value.to_bytes(16, "little"))


def valid_sha(value: str) -> bool:
    return (
        len(value) == 64
        and value == value.lower()
        and all(character in "0123456789abcdef" for character in value)
    )


def parse_uint(row: dict[str, str], key: str) -> int:
    raw = row[key]
    require(raw.isdigit(), f"{key} is not an unsigned integer")
    return int(raw)


def parse_bool(row: dict[str, str], key: str) -> bool:
    require(row[key] in {"true", "false"}, f"{key} is not a boolean")
    return row[key] == "true"


def one(rows: list[dict[str, str]], kind: str) -> dict[str, str]:
    matches = [row for row in rows if row["record_type"] == kind]
    require(len(matches) == 1, f"expected exactly one {kind} row")
    return matches[0]


def snapshot_digest(row: dict[str, str]) -> str:
    canonical = bytearray()
    for value in (
        SCHEMA,
        row["cgroup_id"],
        row["cpu_stat_sha256"],
        row["memory_peak_sha256"],
        row["memory_events_sha256"],
        row["cgroup_events_sha256"],
    ):
        add_string(canonical, value)
    for key in (
        "monotonic_time_ns",
        "cpu_usage_usec",
        "cpu_user_usec",
        "cpu_system_usec",
        "memory_peak_bytes",
        "oom_events",
        "oom_kill_events",
    ):
        add_u64(canonical, parse_uint(row, key))
    if row["oom_group_kill_events"]:
        canonical.append(1)
        add_u64(canonical, int(row["oom_group_kill_events"]))
    else:
        canonical.append(0)
    canonical.append(1 if parse_bool(row, "populated") else 0)
    canonical.append(1 if parse_bool(row, "frozen") else 0)
    return hashlib.sha256(canonical).hexdigest()


def main() -> None:
    require(len(sys.argv) == 2, "usage: verify-cgroup-v2-resource-independent.py RESOURCE.csv")
    rows = read_rows(Path(sys.argv[1]))
    require(all(row["record_type"] in {"start", "end", "report"} for row in rows),
            "cgroup resource CSV contains an unknown record type")
    start, end, report = (one(rows, kind) for kind in ("start", "end", "report"))

    for snapshot in (start, end):
        require(bool(snapshot["cgroup_id"].strip()), "cgroup identity is empty")
        for field in (
            "cpu_stat_sha256",
            "memory_peak_sha256",
            "memory_events_sha256",
            "cgroup_events_sha256",
        ):
            require(valid_sha(snapshot[field]), f"{field} digest malformed")
        require(not parse_bool(snapshot, "populated"), "cgroup is populated at snapshot boundary")
        require(not parse_bool(snapshot, "frozen"), "cgroup is frozen at snapshot boundary")
        require(snapshot_digest(snapshot) == snapshot["snapshot_sha256"],
                "cgroup snapshot digest mismatch")

    require(start["cgroup_id"] == end["cgroup_id"], "cgroup identity mismatch")
    start_ns = parse_uint(start, "monotonic_time_ns")
    end_ns = parse_uint(end, "monotonic_time_ns")
    require(end_ns > start_ns, "monotonic interval must be positive")
    require(
        bool(start["oom_group_kill_events"]) == bool(end["oom_group_kill_events"]),
        "oom_group_kill capability changed",
    )

    def delta(field: str) -> int:
        before, after = parse_uint(start, field), parse_uint(end, field)
        require(after >= before, f"{field} counter regressed/reset")
        return after - before

    usage = delta("cpu_usage_usec")
    user = delta("cpu_user_usec")
    system = delta("cpu_system_usec")
    oom = delta("oom_events")
    oom_kill = delta("oom_kill_events")
    peak_start = parse_uint(start, "memory_peak_bytes")
    peak_end = parse_uint(end, "memory_peak_bytes")
    require(peak_end >= peak_start, "memory.peak regressed/reset")
    group_delta = None
    if start["oom_group_kill_events"]:
        group_start, group_end = int(start["oom_group_kill_events"]), int(end["oom_group_kill_events"])
        require(group_end >= group_start, "oom_group_kill counter regressed/reset")
        group_delta = group_end - group_start

    elapsed = end_ns - start_ns
    core_hours_numerator_usec = usage
    core_hours_denominator_usec = 3_600_000_000
    start_digest, end_digest = start["snapshot_sha256"], end["snapshot_sha256"]

    require(report["schema"] == SCHEMA, "resource report schema mismatch")
    require(report["cgroup_id"] == start["cgroup_id"], "report cgroup identity mismatch")
    require(report["start_monotonic_time_ns"] == str(start_ns), "report start time mismatch")
    require(report["end_monotonic_time_ns"] == str(end_ns), "report end time mismatch")
    expected = {
        "elapsed_wall_time_ns": elapsed,
        "cpu_usage_delta_usec": usage,
        "cpu_user_delta_usec": user,
        "cpu_system_delta_usec": system,
        "cpu_core_hours_numerator_usec": core_hours_numerator_usec,
        "cpu_core_hours_denominator_usec": core_hours_denominator_usec,
        "peak_memory_bytes": peak_end,
        "oom_event_delta": oom,
        "oom_kill_event_delta": oom_kill,
    }
    for field, value in expected.items():
        require(parse_uint(report, field) == value, f"report {field} mismatch")
    require(
        (int(report["oom_group_kill_event_delta"]) if report["oom_group_kill_event_delta"] else None)
        == group_delta,
        "report oom_group_kill delta mismatch",
    )
    require(report["start_snapshot_sha256"] == start_digest, "start snapshot digest binding mismatch")
    require(report["end_snapshot_sha256"] == end_digest, "end snapshot digest binding mismatch")
    require(report["evidence_level"] == EVIDENCE, "resource evidence ceiling mismatch")

    canonical = bytearray()
    for value in (SCHEMA, start["cgroup_id"], start_digest, end_digest, EVIDENCE):
        add_string(canonical, value)
    for value in (
        start_ns,
        end_ns,
        elapsed,
        usage,
        user,
        system,
        peak_end,
        oom,
        oom_kill,
    ):
        add_u64(canonical, value)
    add_u64(canonical, core_hours_numerator_usec)
    add_u64(canonical, core_hours_denominator_usec)
    if group_delta is None:
        canonical.append(0)
    else:
        canonical.append(1)
        add_u64(canonical, group_delta)
    require(
        hashlib.sha256(canonical).hexdigest() == report["report_sha256"],
        "resource report canonical digest mismatch",
    )
    print("independent cgroup-v2 resource oracle: PASS")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"independent cgroup-v2 resource oracle failed: {error}", file=sys.stderr)
        raise
