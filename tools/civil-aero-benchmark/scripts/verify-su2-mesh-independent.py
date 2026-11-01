#!/usr/bin/env python3
"""Independent stdlib oracle for the Rust SU2 native-mesh structural parser."""
from __future__ import annotations

import csv
import hashlib
import math
import sys
from pathlib import Path

SCHEMA = "su2-native-mesh-summary/0.1.0"
EVIDENCE = (
    "mesh_bytes_and_native_format_structure_verified_not_mesh_quality_or_solver_compatibility"
)
VOLUME_NODES = {
    2: {5: 3, 9: 4},
    3: {10: 4, 12: 8, 13: 6, 14: 5},
}
BOUNDARY_NODES = {
    2: {3: 2},
    3: {5: 3, 9: 4},
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def nonempty(lines: list[tuple[int, str]], cursor: int, context: str) -> tuple[int, str, int]:
    while cursor < len(lines):
        number, raw = lines[cursor]
        cursor += 1
        if raw.strip():
            return number, raw.strip(), cursor
    raise ValueError(f"unexpected end while reading {context}")


def count_record(number: int, line: str, key: str, allow_zero: bool = False) -> int:
    require(line.startswith(key), f"line {number}: expected {key}")
    value = int(line[len(key):].strip())
    require(value >= 0 and (allow_zero or value > 0), f"line {number}: invalid {key} count")
    return value


def element_record(number: int, line: str, dimension: int, boundary: bool, point_count: int | None):
    fields = [int(item) for item in line.split()]
    require(bool(fields), f"line {number}: blank element")
    kind = fields[0]
    type_map = BOUNDARY_NODES[dimension] if boundary else VOLUME_NODES[dimension]
    require(kind in type_map, f"line {number}: unsupported element type {kind}")
    node_count = type_map[kind]
    require(len(fields) in (node_count + 1, node_count + 2), f"line {number}: malformed element record")
    indices = fields[1:1 + node_count]
    require(all(index >= 0 for index in indices), f"line {number}: negative node index")
    if point_count is not None:
        require(all(index < point_count for index in indices), f"line {number}: node index out of range")
    return kind, indices


def main() -> int:
    if len(sys.argv) != 3:
        raise ValueError("usage: verify-su2-mesh-independent.py MESH.su2 RUST_SUMMARY.csv")
    mesh_path = Path(sys.argv[1])
    summary_path = Path(sys.argv[2])
    raw = mesh_path.read_bytes()
    require(raw, "mesh is empty")
    require(raw.isascii(), "mesh is not ASCII")
    digest = hashlib.sha256(raw).hexdigest()
    lines = [(index + 1, line.rstrip("\r")) for index, line in enumerate(raw.decode("ascii").splitlines())]
    cursor = 0
    number, line, cursor = nonempty(lines, cursor, "NDIME")
    require(line.startswith("NDIME="), "first record is not NDIME")
    dimension = int(line.split("=", 1)[1].strip())
    require(dimension in (2, 3), "dimension must be 2 or 3")

    point_count = None
    volume_count = None
    marker_count = None
    volume_types: dict[int, int] = {}
    marker_records: list[tuple[str, int]] = []
    all_node_refs: list[tuple[int, int]] = []
    minima = [math.inf, math.inf, math.inf]
    maxima = [-math.inf, -math.inf, -math.inf]
    boundary_count = 0

    while cursor < len(lines):
        number, line, cursor = nonempty(lines, cursor, "mesh section")
        if line.startswith("NPOIN="):
            require(point_count is None, "duplicate NPOIN section")
            count = count_record(number, line, "NPOIN=")
            for _ in range(count):
                point_line, point_record, cursor = nonempty(lines, cursor, "point record")
                fields = point_record.split()
                require(len(fields) in (dimension, dimension + 1), f"line {point_line}: malformed point")
                coordinates = [float(value) for value in fields[:dimension]]
                require(all(math.isfinite(value) for value in coordinates), f"line {point_line}: nonfinite coordinate")
                for axis, value in enumerate(coordinates):
                    minima[axis] = min(minima[axis], value)
                    maxima[axis] = max(maxima[axis], value)
                if len(fields) == dimension + 1:
                    require(int(fields[-1]) >= 0, f"line {point_line}: invalid legacy point index")
            point_count = count
        elif line.startswith("NELEM="):
            require(volume_count is None, "duplicate NELEM section")
            count = count_record(number, line, "NELEM=")
            for _ in range(count):
                element_line, element, cursor = nonempty(lines, cursor, "volume element")
                kind, indices = element_record(element_line, element, dimension, False, point_count)
                volume_types[kind] = volume_types.get(kind, 0) + 1
                all_node_refs.extend((element_line, index) for index in indices)
            volume_count = count
        elif line.startswith("NMARK="):
            require(marker_count is None, "duplicate NMARK section")
            count = count_record(number, line, "NMARK=", allow_zero=True)
            seen_tags: set[str] = set()
            for _ in range(count):
                tag_line, tag_record, cursor = nonempty(lines, cursor, "MARKER_TAG")
                require(tag_record.startswith("MARKER_TAG="), f"line {tag_line}: expected MARKER_TAG")
                tag = tag_record.split("=", 1)[1].strip()
                require(tag and tag not in seen_tags, f"line {tag_line}: empty or duplicate marker")
                seen_tags.add(tag)
                count_line, elem_count_line, cursor = nonempty(lines, cursor, "MARKER_ELEMS")
                elem_count = count_record(count_line, elem_count_line, "MARKER_ELEMS=")
                for _ in range(elem_count):
                    elem_line, element, cursor = nonempty(lines, cursor, "marker connectivity")
                    _, indices = element_record(elem_line, element, dimension, True, point_count)
                    all_node_refs.extend((elem_line, index) for index in indices)
                marker_records.append((tag, elem_count))
                boundary_count += elem_count
            marker_count = count
        else:
            raise ValueError(f"line {number}: unsupported mesh section {line}")

    require(point_count is not None and volume_count is not None and marker_count is not None,
            "mesh is missing one or more required sections")
    require(sum(volume_types.values()) == volume_count, "volume element count mismatch")
    require(len(marker_records) == marker_count, "marker count mismatch")
    for line_number, index in all_node_refs:
        require(index < point_count, f"line {line_number}: node index out of range")

    with summary_path.open(newline="", encoding="utf-8") as stream:
        result_rows = list(csv.DictReader(stream))
    require(len(result_rows) == 1, "expected one summary row")
    result = result_rows[0]
    require(result["schema"] == SCHEMA, "schema mismatch")
    require(result["evidence_level"] == EVIDENCE, "evidence ceiling mismatch")
    require(int(result["byte_length"]) == len(raw), "mesh byte length mismatch")
    require(result["mesh_sha256"] == digest, "mesh digest mismatch")
    require(int(result["dimension"]) == dimension, "dimension mismatch")
    require(int(result["point_count"]) == point_count, "point count mismatch")
    require(int(result["volume_element_count"]) == volume_count, "volume element count mismatch")
    require(int(result["marker_count"]) == marker_count, "marker count mismatch")
    require(int(result["boundary_element_count"]) == boundary_count, "boundary element count mismatch")
    require(result["volume_element_type_counts"] == ";".join(f"{kind}:{volume_types[kind]}" for kind in sorted(volume_types)),
            "volume element type counts mismatch")
    require(result["markers"] == ";".join(f"{tag}:{count}" for tag, count in sorted(marker_records)),
            "marker summaries mismatch")
    for axis, suffix in enumerate(("x", "y", "z")):
        expected_min = "" if axis >= dimension else format(minima[axis], ".17g")
        expected_max = "" if axis >= dimension else format(maxima[axis], ".17g")
        require(math.isclose(float(result[f"min_{suffix}"]), minima[axis], rel_tol=0.0, abs_tol=1e-15)
                if axis < dimension else result[f"min_{suffix}"] == "", f"min {suffix} mismatch")
        require(math.isclose(float(result[f"max_{suffix}"]), maxima[axis], rel_tol=0.0, abs_tol=1e-15)
                if axis < dimension else result[f"max_{suffix}"] == "", f"max {suffix} mismatch")
    print(
        "independent_su2_mesh_oracle=pass "
        f"dimension={dimension} points={point_count} volume_elements={volume_count} "
        f"markers={marker_count} boundary_elements={boundary_count} mesh_sha256={digest}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"independent SU2 mesh oracle failed: {error}", file=sys.stderr)
        raise SystemExit(1)
