#!/usr/bin/env python3
"""Independent standard-library oracle for synthetic aircraft loading-state fixtures.

This script recomputes total mass, centre of gravity and inertia from the JSON
fixture. It does not import production Rust code. It also checks selected
hostile evidence/effectivity/unit/accounting controls.

The source identifiers and numeric values are synthetic. Passing this finite
oracle is software/arithmetic evidence only, not aircraft validation.
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path
from typing import Any

TOLERANCE = 1.0e-9


def close(a: float, b: float) -> bool:
    return math.isfinite(a) and math.isfinite(b) and abs(a - b) <= TOLERANCE * max(1.0, abs(a), abs(b))


def convert_mass(value: float, unit: str) -> float:
    factors = {"kg": 1.0, "g": 1.0 / 1000.0, "lb": 0.45359237}
    if unit not in factors or not math.isfinite(value) or value <= 0.0:
        raise ValueError(f"invalid source mass {value!r} {unit!r}")
    return value * factors[unit]


def convert_length(value: float, unit: str) -> float:
    factors = {"m": 1.0, "cm": 0.01, "mm": 0.001, "ft": 0.3048, "in": 0.0254}
    if unit not in factors or not math.isfinite(value):
        raise ValueError(f"invalid source length {value!r} {unit!r}")
    return value * factors[unit]


def convert_inertia(value: float, unit: str) -> float:
    factors = {
        "kg_m2": 1.0,
        "g_cm2": 1.0e-7,
        "lb_ft2": 0.45359237 * 0.3048**2,
    }
    if unit not in factors or not math.isfinite(value) or value < 0.0:
        raise ValueError(f"invalid source inertia {value!r} {unit!r}")
    return value * factors[unit]


def validate_source_units(fixture: dict[str, Any]) -> None:
    for item in fixture["base_items"]:
        if item.get("coordinate_frame_id") != fixture["aircraft"]["reference_frame_id"]:
            raise ValueError(f"base item {item['id']}: source coordinate frame mismatch")
        source = item["unit_source_mass"]
        converted = convert_mass(source["value"], source["unit"])
        if not close(converted, item["mass_kg"]):
            raise ValueError(f"base item {item['id']}: source mass does not convert to kg")
    # Known conversion vectors, intentionally independent of Rust implementation.
    if not close(convert_mass(220.46226218487757, "lb"), 100.0):
        raise ValueError("pound-to-kilogram conversion vector failed")
    if not close(convert_length(12.0, "in"), 0.3048):
        raise ValueError("inch-to-metre conversion vector failed")
    if not close(convert_inertia(10_000_000.0, "g_cm2"), 1.0):
        raise ValueError("gram-centimetre-squared to kg-metre-squared conversion failed")


def reject_reason(fixture: dict[str, Any], state: dict[str, Any]) -> str | None:
    aircraft = fixture["aircraft"]
    if state.get("source_generation_sha256", aircraft["source_generation_sha256"]) != aircraft["source_generation_sha256"]:
        return "stale_source_generation"
    if state.get("reference_frame_id", aircraft["reference_frame_id"]) != aircraft["reference_frame_id"]:
        return "wrong_reference_frame"
    if state.get("serial_effectivity") is not None and aircraft["serial"] not in state["serial_effectivity"]:
        return "wrong_serial_effectivity"
    if "source_mass_override" in state:
        for item_id, source in state["source_mass_override"].items():
            base = next(item for item in fixture["base_items"] if item["id"] == item_id)
            if not close(convert_mass(source["value"], source["unit"]), base["mass_kg"]):
                return "source_unit_mismatch"
    aggregate = state.get("aggregate_record")
    if aggregate is not None:
        installed_ids = {row["id"] for row in state["items"] if row["installation"] == "installed"}
        if aggregate["id"] in installed_ids and any(child in installed_ids for child in aggregate["includes"]):
            return "aggregate_child_double_count"
    return None


def compute_state(fixture: dict[str, Any], state: dict[str, Any]) -> dict[str, Any]:
    aircraft = fixture["aircraft"]
    expected = state["expected"]
    if expected["status"] == "reject":
        reason = reject_reason(fixture, state)
        if reason != expected["reason"]:
            raise ValueError(f"{state['id']}: expected rejection {expected['reason']}, observed {reason}")
        return expected

    base = {item["id"]: item for item in fixture["base_items"]}
    state_rows = state["items"]
    state_ids = [row["id"] for row in state_rows]
    expected_ids = sorted(base)
    # The aggregate hostile record adds one explicit extra manifest row.
    if state.get("aggregate_record") is not None:
        expected_ids.append(state["aggregate_record"]["id"])
        expected_ids.sort()
    if state_ids != expected_ids:
        raise ValueError(f"{state['id']}: state manifest must cover every item once in sorted order")

    if state["configuration_sha256"] not in (
        aircraft["configuration_sha256"],
        aircraft["alternate_configuration_sha256"],
    ):
        raise ValueError(f"{state['id']}: unrecognized configuration identity")
    if state.get("reference_frame_id", aircraft["reference_frame_id"]) != aircraft["reference_frame_id"]:
        raise ValueError(f"{state['id']}: reference frame mismatch")

    pending = False
    unknown_mass = False
    unknown_geometry = False
    unresolved_installation = False
    items: list[tuple[float, tuple[float, float, float]]] = []

    for row in state_rows:
        item_id = row["id"]
        if item_id == state.get("aggregate_record", {}).get("id"):
            base_item = state["aggregate_record"]
        else:
            base_item = base[item_id]

        if state.get("item_evidence_overrides", {}).get(item_id, {}).get("admission") == "pending_review":
            pending = True

        installation = row["installation"]
        if installation == "removed":
            continue
        if installation == "unknown":
            unresolved_installation = True
            unknown_mass = True
            unknown_geometry = True
            continue
        if installation != "installed":
            raise ValueError(f"{state['id']}: unsupported installation state")

        mass = row.get("mass_kg", base_item["mass_kg"])
        pos = row.get("position_m", base_item["position_m"])
        if "source_mass_override" in state and item_id in state["source_mass_override"]:
            source = state["source_mass_override"][item_id]
            if not close(convert_mass(source["value"], source["unit"]), float(mass)):
                raise ValueError(f"{state['id']}: state source mass mismatch unexpectedly escaped reject gate")
        if mass is None:
            unknown_mass = True
            continue
        if pos is None:
            unknown_geometry = True
            # Keep known mass separately; no complete CG/inertia result is possible.
            items.append((float(mass), (math.nan, math.nan, math.nan)))
            continue
        if not math.isfinite(float(mass)) or float(mass) <= 0:
            raise ValueError(f"{state['id']}: invalid installed mass")
        if len(pos) != 3 or any(not math.isfinite(float(value)) for value in pos):
            raise ValueError(f"{state['id']}: invalid position")
        items.append((float(mass), tuple(float(value) for value in pos)))

    known_mass = sum(mass for mass, _ in items)
    total = None if unknown_mass else known_mass
    cg = None
    inertia = None
    if not unknown_mass and not unknown_geometry and not unresolved_installation:
        total = known_mass
        cg = tuple(sum(mass * pos[axis] for mass, pos in items) / total for axis in range(3))
        inertia_values = [0.0] * 6
        for mass, pos in items:
            dx, dy, dz = (pos[axis] - cg[axis] for axis in range(3))
            inertia_values[0] += mass * (dy * dy + dz * dz)
            inertia_values[1] += mass * (dx * dx + dz * dz)
            inertia_values[2] += mass * (dx * dx + dy * dy)
            inertia_values[3] -= mass * dx * dy
            inertia_values[4] -= mass * dx * dz
            inertia_values[5] -= mass * dy * dz
        inertia = tuple(inertia_values)

    status = (
        "complete_for_screening"
        if not pending and not unknown_mass and not unknown_geometry and not unresolved_installation
        else "indeterminate"
    )
    observed = {
        "status": status,
        "total_mass_kg": total,
        "cg_m": cg,
        "inertia_kg_m2": inertia,
    }
    for key in ("status", "total_mass_kg", "cg_m", "inertia_kg_m2"):
        target = expected[key]
        actual = observed[key]
        if target is None:
            if actual is not None:
                raise ValueError(f"{state['id']}: {key} expected null, observed {actual}")
        elif isinstance(target, list):
            if actual is None or len(actual) != len(target) or any(not close(float(a), float(b)) for a, b in zip(actual, target)):
                raise ValueError(f"{state['id']}: {key} expected {target}, observed {actual}")
        elif isinstance(target, (int, float)):
            if actual is None or not close(float(actual), float(target)):
                raise ValueError(f"{state['id']}: {key} expected {target}, observed {actual}")
        elif actual != target:
            raise ValueError(f"{state['id']}: {key} expected {target!r}, observed {actual!r}")
    return observed


def verify(path: Path) -> None:
    fixture = json.loads(path.read_text(encoding="utf-8"))
    if fixture.get("schema") != "luminous.aero.mass-state-fixtures":
        raise ValueError("unexpected mass-state fixture schema")
    validate_source_units(fixture)
    seen: set[str] = set()
    for state in fixture["states"]:
        if state["id"] in seen:
            raise ValueError(f"duplicate state id {state['id']}")
        seen.add(state["id"])
        compute_state(fixture, state)
    print(f"mass_state_independent_oracle=pass states={len(seen)} source_units=mass,length,inertia")


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: verify-mass-state-fixtures.py <mass_state_fixtures.json>", file=sys.stderr)
        return 2
    try:
        verify(Path(sys.argv[1]))
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"mass_state_oracle_error={error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
