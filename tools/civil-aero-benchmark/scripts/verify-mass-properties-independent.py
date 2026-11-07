#!/usr/bin/env python3
"""Independent standard-library mass-properties oracle for the Rust audit CLI.

This implementation deliberately parses the input fixture and recomputes the
mass, CG and inertia tensor without importing or calling production Rust code.
It is a finite-fixture oracle, not a certified weight-and-balance program.
"""

from __future__ import annotations

import csv
import math
import sys
from pathlib import Path

EXPECTED_HEADER = [
    "configuration_sha256",
    "status",
    "evidence_level",
    "total_mass_kg",
    "known_mass_subtotal_kg",
    "cg_x_m",
    "cg_y_m",
    "cg_z_m",
    "Ixx",
    "Iyy",
    "Izz",
    "Ixy",
    "Ixz",
    "Iyz",
    "unknown_mass_item_ids",
    "unknown_geometry_item_ids",
]
TOLERANCE = 1.0e-9


def optional_float(value: str, field: str, line: int) -> float | None:
    value = value.strip()
    if not value:
        return None
    parsed = float(value)
    if not math.isfinite(parsed):
        raise ValueError(f"line {line}: {field} must be finite")
    return parsed


def load_input(path: Path) -> tuple[str, list[dict[str, object]]]:
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        expected = [
            "configuration_sha256", "item_id", "category", "mass_kg",
            "geometry_kind", "x_m", "y_m", "z_m",
            "Ixx", "Iyy", "Izz", "Ixy", "Ixz", "Iyz",
        ]
        if reader.fieldnames != expected:
            raise ValueError("input CSV header does not match the exact fixture schema")
        rows: list[dict[str, object]] = []
        config_ids: set[str] = set()
        seen_ids: set[str] = set()
        for line, raw in enumerate(reader, start=2):
            config = (raw["configuration_sha256"] or "").strip()
            item_id = (raw["item_id"] or "").strip()
            if len(config) != 64 or any(ch not in "0123456789abcdefABCDEF" for ch in config):
                raise ValueError(f"line {line}: invalid configuration SHA-256")
            if not item_id or item_id in seen_ids:
                raise ValueError(f"line {line}: empty or duplicate item ID")
            seen_ids.add(item_id)
            config_ids.add(config)

            mass = optional_float(raw["mass_kg"] or "", "mass_kg", line)
            if mass is not None and mass <= 0.0:
                raise ValueError(f"line {line}: known mass must be > 0")
            geometry = (raw["geometry_kind"] or "").strip()
            xyz = [
                optional_float(raw[name] or "", name, line)
                for name in ("x_m", "y_m", "z_m")
            ]
            tensor = [
                optional_float(raw[name] or "", name, line)
                for name in ("Ixx", "Iyy", "Izz", "Ixy", "Ixz", "Iyz")
            ]
            if geometry == "point":
                if any(v is None for v in xyz) or any(v is not None for v in tensor):
                    raise ValueError(f"line {line}: invalid point-mass fields")
                local_inertia = [0.0] * 6
            elif geometry == "rigid_body":
                if any(v is None for v in xyz) or any(v is None for v in tensor):
                    raise ValueError(f"line {line}: invalid rigid-body fields")
                local_inertia = [float(v) for v in tensor]
            elif geometry == "unknown":
                if any(v is not None for v in xyz + tensor):
                    raise ValueError(f"line {line}: unknown geometry must omit coordinates")
                local_inertia = None
            else:
                raise ValueError(f"line {line}: unsupported geometry kind")

            rows.append({
                "id": item_id,
                "mass": mass,
                "geometry": geometry,
                "xyz": [float(v) for v in xyz] if all(v is not None for v in xyz) else None,
                "local_inertia": local_inertia,
            })
    if len(config_ids) != 1:
        raise ValueError("input must contain exactly one configuration identity")
    if not rows:
        raise ValueError("input fixture has no records")
    return next(iter(config_ids)), rows


def expected_properties(rows: list[dict[str, object]]) -> dict[str, object]:
    unknown_mass = sorted(str(row["id"]) for row in rows if row["mass"] is None)
    unknown_geometry = sorted(
        str(row["id"]) for row in rows if row["geometry"] == "unknown"
    )
    known_mass = [float(row["mass"]) for row in rows if row["mass"] is not None]
    subtotal = sum(known_mass)
    total = None if unknown_mass else subtotal
    complete = not unknown_mass and not unknown_geometry

    expected: dict[str, object] = {
        "status": "complete" if complete else "indeterminate",
        "evidence_level": "screening_only",
        "total_mass_kg": total,
        "known_mass_subtotal_kg": subtotal,
        "unknown_mass_item_ids": ";".join(unknown_mass),
        "unknown_geometry_item_ids": ";".join(unknown_geometry),
    }
    if not complete:
        for key in ("cg_x_m", "cg_y_m", "cg_z_m", "Ixx", "Iyy", "Izz", "Ixy", "Ixz", "Iyz"):
            expected[key] = None
        return expected

    total_mass = float(total)
    weighted = [0.0, 0.0, 0.0]
    for row in rows:
        mass = float(row["mass"])
        xyz = row["xyz"]
        assert isinstance(xyz, list)
        for axis in range(3):
            weighted[axis] += mass * xyz[axis]
    cg = [value / total_mass for value in weighted]

    inertia = [0.0] * 6
    for row in rows:
        mass = float(row["mass"])
        xyz = row["xyz"]
        local = row["local_inertia"]
        assert isinstance(xyz, list) and isinstance(local, list)
        dx, dy, dz = (xyz[0] - cg[0], xyz[1] - cg[1], xyz[2] - cg[2])
        inertia[0] += local[0] + mass * (dy * dy + dz * dz)
        inertia[1] += local[1] + mass * (dx * dx + dz * dz)
        inertia[2] += local[2] + mass * (dx * dx + dy * dy)
        inertia[3] += local[3] - mass * dx * dy
        inertia[4] += local[4] - mass * dx * dz
        inertia[5] += local[5] - mass * dy * dz

    expected.update({
        "cg_x_m": cg[0], "cg_y_m": cg[1], "cg_z_m": cg[2],
        "Ixx": inertia[0], "Iyy": inertia[1], "Izz": inertia[2],
        "Ixy": inertia[3], "Ixz": inertia[4], "Iyz": inertia[5],
    })
    return expected


def verify(input_path: Path, report_path: Path) -> None:
    config, rows = load_input(input_path)
    expected = expected_properties(rows)
    with report_path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        if reader.fieldnames != EXPECTED_HEADER:
            raise ValueError("Rust report header does not match the expected schema")
        output_rows = list(reader)
    if len(output_rows) != 1:
        raise ValueError(f"expected exactly one Rust output row, found {len(output_rows)}")
    actual = output_rows[0]
    if actual["configuration_sha256"] != config:
        raise ValueError("configuration identity does not match input")
    for field in EXPECTED_HEADER[1:]:
        target = expected[field]
        observed = actual[field]
        if target is None:
            if observed != "":
                raise ValueError(f"{field}: expected unresolved blank, got {observed!r}")
        elif isinstance(target, float):
            value = float(observed)
            if not math.isfinite(value) or abs(value - target) > TOLERANCE * max(1.0, abs(target)):
                raise ValueError(f"{field}: expected {target:.15g}, got {value:.15g}")
        elif observed != str(target):
            raise ValueError(f"{field}: expected {target!r}, got {observed!r}")
    print(
        f"mass_properties_oracle=pass items={len(rows)} "
        f"status={expected['status']} independent=python-stdlib"
    )


def main() -> int:
    if len(sys.argv) != 3:
        print(
            "usage: verify-mass-properties-independent.py <input.csv> <rust-report.csv>",
            file=sys.stderr,
        )
        return 2
    try:
        verify(Path(sys.argv[1]), Path(sys.argv[2]))
    except (OSError, ValueError, AssertionError) as error:
        print(f"mass_properties_oracle_error={error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
