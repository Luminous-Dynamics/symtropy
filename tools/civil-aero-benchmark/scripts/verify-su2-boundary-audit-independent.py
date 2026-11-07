#!/usr/bin/env python3
"""Independent receipt/config/mesh and canonical digest oracle for SU2 boundary mapping."""
from __future__ import annotations

import csv
import hashlib
import struct
import sys
import traceback
from pathlib import Path

SCHEMA = "su2-boundary-marker-audit/0.1.0"
EVIDENCE = (
    "receipt_bound_exact_mesh_marker_to_declared_config_boundary_option_mapping_"
    "not_boundary_physics_or_flow_validation"
)
STATUS = "every_mesh_marker_mapped_once_and_config_marker_references_resolve"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def add_string(buffer: bytearray, value: str) -> None:
    raw = value.encode("utf-8")
    buffer.extend(struct.pack("<Q", len(raw)))
    buffer.extend(raw)


def add_u64(buffer: bytearray, value: int) -> None:
    buffer.extend(struct.pack("<Q", value))


def read_one_csv(path: Path) -> dict[str, str]:
    with path.open(newline="", encoding="utf-8") as stream:
        rows = list(csv.DictReader(stream))
    require(len(rows) >= 1, f"{path} has no report rows")
    first = rows[0]
    for row in rows[1:]:
        for key, value in first.items():
            if key not in {"assigned_marker", "config_option", "record_index", "marker_position"}:
                require(row[key] == value, f"report global field {key} is inconsistent")
    return first


def parse_config(path: Path) -> dict[str, str]:
    text = path.read_text(encoding="ascii")
    result: dict[str, str] = {}
    pending: tuple[int, str] | None = None
    for index, raw in enumerate(text.splitlines(), start=1):
        line = raw.split("%", 1)[0].strip()
        if not line:
            continue
        if pending is not None:
            start, previous = pending
            continued = line.endswith("\\")
            fragment = line[:-1].strip() if continued else line
            combined = previous + (" " + fragment if fragment else "")
            pending = (start, combined) if continued else None
            if not continued and "=" in combined:
                key, value = combined.split("=", 1)
                key = key.strip()
                require(key not in result, f"duplicate config key {key} at line {start}")
                result[key] = value.strip()
            continue
        if line.endswith("\\"):
            pending = (index, line[:-1].strip())
            continue
        if "=" in line:
            key, value = line.split("=", 1)
            key = key.strip()
            require(key not in result, f"duplicate config key {key} at line {index}")
            result[key] = value.strip()
    if pending is not None:
        raise ValueError(f"unterminated config continuation at line {pending[0]}")
    return result


def parse_parenthesized(value: str, label: str) -> list[str]:
    value = value.strip()
    require(value.startswith("(") and value.endswith(")"), f"{label} lacks parenthesized list")
    fields = [field.strip().strip('"').strip("'") for field in value[1:-1].split(",")]
    require(fields and all(fields), f"{label} has empty fields")
    return fields


def mesh_markers(path: Path) -> tuple[list[str], int, int, int]:
    text = path.read_text(encoding="ascii")
    lines = [line.strip() for line in text.splitlines() if line.strip()]
    dimension = point_count = element_count = None
    tags: list[str] = []
    for line in lines:
        if line.startswith("NDIME="):
            dimension = int(line.split("=", 1)[1])
        elif line.startswith("NPOIN="):
            point_count = int(line.split("=", 1)[1])
        elif line.startswith("NELEM="):
            element_count = int(line.split("=", 1)[1])
        elif line.startswith("MARKER_TAG="):
            tags.append(line.split("=", 1)[1].strip())
    require(dimension in (2, 3), "mesh dimension missing/unsupported")
    require(point_count is not None and element_count is not None, "mesh count records missing")
    require(len(tags) == len(set(tags)), "duplicate mesh marker tags")
    return sorted(tags), dimension, point_count, element_count


def parse_specs(encoded: str) -> list[tuple[str, int, list[int]]]:
    specs = []
    for item in encoded.split(";"):
        name, width, positions = item.split(":", 2)
        specs.append((name, int(width), [int(value) for value in positions.split("|")]))
    return specs


def main() -> int:
    if len(sys.argv) != 3:
        raise ValueError("usage: verify-su2-boundary-audit-independent.py RUN_DIR REPORT.csv")
    run = Path(sys.argv[1])
    with (run / "runner_receipt.csv").open(newline="", encoding="utf-8") as stream:
        receipts = list(csv.DictReader(stream))
    require(len(receipts) == 1, "expected one runner receipt")
    receipt = receipts[0]
    report_path = Path(sys.argv[2])
    with report_path.open(newline="", encoding="utf-8") as stream:
        rows = list(csv.DictReader(stream))
    require(rows, "boundary audit report contains no assignments")
    row = rows[0]
    for other in rows[1:]:
        for key in row:
            if key not in {"assigned_marker", "config_option", "record_index", "marker_position"}:
                require(other[key] == row[key], f"inconsistent report field {key}")

    require(row["schema"] == SCHEMA, "schema mismatch")
    require(row["evidence_level"] == EVIDENCE, "evidence ceiling mismatch")
    require(row["status"] == STATUS, "boundary audit did not pass")
    require(row["solver_id"] == "SU2_CFD_DRY_RUN" == receipt["solver_id"], "solver ID mismatch")
    require(row["run_id"] == receipt["run_id"], "run ID mismatch")
    require(row["runner_receipt_sha256"] == receipt["receipt_sha256"], "receipt digest mismatch")
    require(row["executable_sha256"] == receipt["executable_sha256"], "executable digest mismatch")

    config = run / row["configuration_path"]
    mesh = run / row["mesh_path"]
    config_bytes, mesh_bytes = config.read_bytes(), mesh.read_bytes()
    config_sha, mesh_sha = hashlib.sha256(config_bytes).hexdigest(), hashlib.sha256(mesh_bytes).hexdigest()
    require(len(config_bytes) == int(row["configuration_byte_length"]), "config byte length mismatch")
    require(config_sha == row["configuration_sha256"], "config SHA-256 mismatch")
    require(len(mesh_bytes) == int(row["mesh_byte_length"]), "mesh byte length mismatch")
    require(mesh_sha == row["mesh_sha256"], "mesh SHA-256 mismatch")

    invocation: dict[str, str] = {}
    for line in (run / "invocation_manifest.txt").read_text(encoding="utf-8").splitlines():
        key, value = line.split("=", 1)
        require(key not in invocation, f"duplicate invocation key {key}")
        invocation[key] = value
    require(invocation.get("argument_count") == "2", "dry-run must have exactly 2 argv elements")
    args = [bytes.fromhex(invocation[f"argument_{i}_utf8_hex"]).decode("utf-8") for i in range(2)]
    require(args == ["-d", row["configuration_path"]], f"wrong dry-run argv {args}")

    config_values = parse_config(config)
    markers, dimension, point_count, element_count = mesh_markers(mesh)
    require(row["mesh_markers"].split(";") == markers, "reported mesh markers differ from raw mesh")
    require(int(row["mesh_dimension"]) == dimension, "mesh dimension mismatch")
    require(int(row["mesh_point_count"]) == point_count, "mesh point count mismatch")
    require(int(row["mesh_volume_element_count"]) == element_count, "mesh element count mismatch")

    specs = parse_specs(row["boundary_option_specs"])
    ignored = row["non_boundary_marker_options"].split(";") if row["non_boundary_marker_options"] else []
    spec_names = {name for name, _, _ in specs}
    ignored_names = set(ignored)
    require(not (spec_names & ignored_names), "option classified as both boundary and non-boundary")
    for key in config_values:
        if key.startswith("MARKER_") or key == "GEO_MARKER":
            require(key in spec_names or key in ignored_names, f"unclassified config marker option {key}")
    for name in ignored:
        if name in config_values:
            for marker in parse_parenthesized(config_values[name], name):
                require(marker in markers, f"non-boundary option {name} references missing mesh marker {marker}")

    assigned: dict[str, tuple[str, int, int]] = {}
    for name, width, positions in specs:
        require(name in config_values, f"config lacks declared option {name}")
        values = parse_parenthesized(config_values[name], name)
        require(len(values) % width == 0, f"{name} values do not align to record width")
        for record_index in range(len(values) // width):
            record = values[record_index * width:(record_index + 1) * width]
            for position in positions:
                marker = record[position]
                require(marker in markers, f"{name} references marker absent from mesh: {marker}")
                require(marker not in assigned, f"mesh marker assigned more than once: {marker}")
                assigned[marker] = (name, record_index, position)
    require(set(assigned) == set(markers), "not every mesh marker has exactly one assignment")

    reported = {
        row_["assigned_marker"]: (
            row_["config_option"],
            int(row_["record_index"]),
            int(row_["marker_position"]),
        )
        for row_ in rows
    }
    require(reported == assigned, "report assignments differ from independent parser")

    buffer = bytearray()
    for value in [
        SCHEMA, row["run_id"], row["solver_id"], row["runner_receipt_sha256"],
        row["executable_sha256"], row["configuration_path"], config_sha,
        row["mesh_path"], mesh_sha, STATUS, EVIDENCE,
    ]:
        add_string(buffer, value)
    for value in [len(config_bytes), len(mesh_bytes), dimension, point_count, element_count]:
        add_u64(buffer, value)
    add_u64(buffer, len(markers))
    for marker in markers:
        add_string(buffer, marker)
    add_u64(buffer, len(specs))
    for name, width, positions in specs:
        add_string(buffer, name)
        add_u64(buffer, width)
        add_u64(buffer, len(positions))
        for position in positions:
            add_u64(buffer, position)
    add_u64(buffer, len(ignored))
    for name in ignored:
        add_string(buffer, name)
    ordered = sorted((marker, *assigned[marker]) for marker in assigned)
    add_u64(buffer, len(ordered))
    for marker, option, record_index, position in ordered:
        add_string(buffer, marker)
        add_string(buffer, option)
        add_u64(buffer, record_index)
        add_u64(buffer, position)
    digest = hashlib.sha256(buffer).hexdigest()
    require(row["audit_sha256"] == digest, "canonical boundary audit digest mismatch")
    print(
        "independent_su2_boundary_audit_oracle=pass "
        f"markers={len(markers)} exact_assignments={len(assigned)} "
        "orphan_references=0 unclassified_marker_options=0"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"independent SU2 boundary audit oracle failed: {error}", file=sys.stderr)
        traceback.print_exc(file=sys.stderr)
        raise SystemExit(1)
