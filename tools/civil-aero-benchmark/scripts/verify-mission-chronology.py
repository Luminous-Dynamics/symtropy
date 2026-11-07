#!/usr/bin/env python3
"""Independent standard-library oracle for the AER-01 mission chronology fixture.

This script deliberately does not import production Rust modules. It recomputes
mass properties, fuel transitions, phase continuity, energy balances and reserve
arithmetic from the raw JSON fixture. All supplied values are synthetic.
"""

from __future__ import annotations

import copy
import hashlib
import json
import math
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "data" / "mission" / "mission_chronology_fixture_v0_1.json"
SCHEMAS = (
    ROOT / "schemas" / "mission-timeline.schema.json",
    ROOT / "schemas" / "aircraft-phase-state.schema.json",
    ROOT / "schemas" / "phase-energy-balance.schema.json",
)
TOL = 1.0e-8


class AuditError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AuditError(message)


def close(actual: float, expected: float, field: str) -> None:
    require(
        math.isfinite(actual)
        and math.isfinite(expected)
        and abs(actual - expected) <= TOL * max(1.0, abs(expected)),
        f"{field}: expected {expected:.15g}, got {actual:.15g}",
    )


def canonical_state_sha256(state: dict[str, Any]) -> str:
    """Canonical SHA-256: sorted-key compact UTF-8 JSON, excluding only state_sha256."""
    payload = {key: value for key, value in state.items() if key != "state_sha256"}
    encoded = json.dumps(
        payload,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def rebuild_state_hashes(fixture: dict[str, Any]) -> None:
    """Rebuild predecessor and state identities after an intentional hostile mutation."""
    states = fixture["phase_states"]
    digest_by_id: dict[str, str] = {}
    for index, state in enumerate(states):
        state["previous_state_sha256"] = None if index == 0 else states[index - 1]["state_sha256"]
        state["state_sha256"] = canonical_state_sha256(state)
        digest_by_id[state["state_id"]] = state["state_sha256"]
    phase_state_ids = {
        "taxi": ("dispatch", "takeoff"),
        "takeoff": ("takeoff", "takeoff"),
        "climb": ("takeoff", "takeoff"),
        "cruise-1": ("takeoff", "cruise-end"),
        "descent": ("cruise-end", "landing"),
        "landing": ("landing", "landing"),
    }
    for phase in fixture["mission_timeline"]["phases"]:
        start_id, end_id = phase_state_ids[phase["phase_id"]]
        phase["start_state_sha256"] = digest_by_id[start_id]
        phase["end_state_sha256"] = digest_by_id[end_id]


def mass_properties(items: list[dict[str, Any]]) -> tuple[float, list[float], list[float]]:
    require(bool(items), "mass state must contain at least one item")
    ids: list[str] = []
    total = 0.0
    weighted = [0.0, 0.0, 0.0]
    for item in items:
        item_id = item["item_id"]
        require(item_id not in ids, f"duplicate mass item ID: {item_id}")
        ids.append(item_id)
        mass = item["mass_kg"]
        position = item["position_m"]
        require(
            isinstance(mass, (int, float)) and math.isfinite(mass) and mass > 0.0,
            f"{item_id}: mass must be known, finite and positive",
        )
        require(
            isinstance(position, list)
            and len(position) == 3
            and all(isinstance(v, (int, float)) and math.isfinite(v) for v in position),
            f"{item_id}: position must contain three finite coordinates",
        )
        total += float(mass)
        for axis in range(3):
            weighted[axis] += float(mass) * float(position[axis])
    require(ids == sorted(ids), "mass items must be sorted by item_id")
    cg = [value / total for value in weighted]
    inertia = [0.0] * 6
    for item in items:
        mass = float(item["mass_kg"])
        dx, dy, dz = [
            float(item["position_m"][axis]) - cg[axis] for axis in range(3)
        ]
        inertia[0] += mass * (dy * dy + dz * dz)
        inertia[1] += mass * (dx * dx + dz * dz)
        inertia[2] += mass * (dx * dx + dy * dy)
        inertia[3] -= mass * dx * dy
        inertia[4] -= mass * dx * dz
        inertia[5] -= mass * dy * dz
    return total, cg, inertia


def validate_state(
    state: dict[str, Any],
    config: str,
    source_generation: str,
    mission_id: str,
) -> None:
    require(state["mission_id"] == mission_id, "state mission identity mismatch")
    require(state["configuration_sha256"] == config, "state configuration identity mismatch")
    require(
        state["source_generation_sha256"] == source_generation,
        "state source-generation identity mismatch",
    )
    require(
        canonical_state_sha256(state) == state["state_sha256"],
        f"{state['state_id']}: canonical state SHA-256 content mismatch",
    )
    total, cg, inertia = mass_properties(state["mass_items"])
    expected = state["expected_mass_properties"]
    close(total, float(expected["total_mass_kg"]), f"{state['state_id']}.total_mass_kg")
    for axis, value in enumerate(cg):
        close(value, float(expected["center_of_gravity_m"][axis]), f"{state['state_id']}.cg[{axis}]")
    for axis, value in enumerate(inertia):
        close(
            value,
            float(expected["inertia_tensor_about_cg_kg_m2"][axis]),
            f"{state['state_id']}.inertia[{axis}]",
        )


def fuel_mass(state: dict[str, Any]) -> float:
    return sum(
        float(item["mass_kg"])
        for item in state["mass_items"]
        if item["category"] == "fuel_and_fluids"
    )


def validate_state_chain(fixture: dict[str, Any]) -> None:
    timeline = fixture["mission_timeline"]
    config = timeline["configuration_sha256"]
    source_generation = timeline["source_generation_sha256"]
    states = fixture["phase_states"]
    require(bool(states), "fixture must include phase states")
    by_hash: dict[str, dict[str, Any]] = {}
    for index, state in enumerate(states):
        require(state["state_sha256"] not in by_hash, "duplicate state SHA-256")
        if index == 0:
            require(state["previous_state_sha256"] is None, "first state must have null predecessor")
        else:
            previous = states[index - 1]
            require(
                state["previous_state_sha256"] == previous["state_sha256"],
                f"{state['state_id']}: stale or disconnected previous-state digest",
            )
        validate_state(state, config, source_generation, timeline["mission_id"])
        if index > 0:
            previous = states[index - 1]
            previous_items = {item["item_id"]: item for item in previous["mass_items"]}
            current_items = {item["item_id"]: item for item in state["mass_items"]}
            require(previous_items.keys() == current_items.keys(), "item set changed without an explicit installation transition")
            for item_id, old in previous_items.items():
                new = current_items[item_id]
                if old["category"] == "fuel_and_fluids":
                    continue
                require(old == new, f"{item_id}: non-fuel item changed without explicit transition evidence")
            burned = fuel_mass(previous) - fuel_mass(state)
            require(burned >= -TOL, "fuel mass increased without a declared refuelling phase")
            close(
                float(state["fuel_consumption_since_previous_kg"]),
                burned,
                f"{state['state_id']}.fuel_consumption_since_previous_kg",
            )
        by_hash[state["state_sha256"]] = state

    phases = timeline["phases"]
    require(phases, "timeline must contain phases")
    require([p["sequence"] for p in phases] == sorted(p["sequence"] for p in phases), "phase sequence is not ordered")
    energy_by_phase = {x["phase_id"]: x for x in fixture["phase_energy_balances"]}
    require(len(energy_by_phase) == len(fixture["phase_energy_balances"]), "duplicate energy balance phase")
    for index, phase in enumerate(phases):
        require(phase["start_state_sha256"] in by_hash, f"{phase['phase_id']}: start state missing")
        require(phase["end_state_sha256"] in by_hash, f"{phase['phase_id']}: end state missing")
        if index > 0:
            require(
                phase["start_state_sha256"] == phases[index - 1]["end_state_sha256"],
                f"{phase['phase_id']}: phase state chain is discontinuous",
            )
        if phase["assessment_status"] == "assessed_screening":
            require(phase.get("energy_balance_ref") in energy_by_phase, f"{phase['phase_id']}: assessed phase has no energy balance")
        else:
            require(phase.get("energy_balance_ref") is None, f"{phase['phase_id']}: unassessed phase must not reference an energy pass")
    assessed = all(phase["assessment_status"] == "assessed_screening" for phase in phases)
    if not assessed:
        require(
            fixture["expected_mission_status"] == "INDETERMINATE",
            "mission with one or more NOT_ASSESSED phases cannot be declared feasible",
        )


def validate_energy_balance(balance: dict[str, Any]) -> None:
    bus = balance["bus"]
    bus_residual = (
        float(bus["source_energy_j"])
        - float(bus["motor_bus_input_j"])
        - float(bus["auxiliary_bus_load_j"])
        - float(bus["thermal_bus_load_j"])
        - float(bus["other_conversion_loss_j"])
    )
    close(bus_residual, float(bus["residual_j"]), f"{balance['phase_id']}.bus.residual_j")
    close(bus_residual, 0.0, f"{balance['phase_id']}.bus.conservation")

    motor = balance["motor"]
    motor_residual = (
        float(motor["bus_input_j"])
        - float(motor["shaft_output_j"])
        - float(motor["conversion_loss_j"])
    )
    close(motor_residual, float(motor["residual_j"]), f"{balance['phase_id']}.motor.residual_j")
    close(motor_residual, 0.0, f"{balance['phase_id']}.motor.conservation")
    close(float(motor["bus_input_j"]), float(bus["motor_bus_input_j"]), f"{balance['phase_id']}.motor/bus coupling")

    shaft = balance["shaft"]
    shaft_residual = (
        float(shaft["shaft_output_j"])
        - float(shaft["propulsive_useful_energy_j"])
        - float(shaft["propulsor_loss_j"])
    )
    close(shaft_residual, float(shaft["residual_j"]), f"{balance['phase_id']}.shaft.residual_j")
    close(shaft_residual, 0.0, f"{balance['phase_id']}.shaft.conservation")
    close(float(shaft["shaft_output_j"]), float(motor["shaft_output_j"]), f"{balance['phase_id']}.shaft/motor coupling")

    store = balance["energy_store"]
    store_residual = (
        float(store["usable_energy_start_j"])
        - float(store["bus_source_energy_j"])
        - float(store["usable_energy_end_j"])
    )
    close(store_residual, 0.0, f"{balance['phase_id']}.energy_store.conservation")
    close(
        float(store["bus_source_energy_j"]),
        float(bus["source_energy_j"]),
        f"{balance['phase_id']}.energy_store/bus coupling",
    )
    margin = float(store["usable_energy_end_j"]) - float(store["reserve_required_j"])
    close(margin, float(store["reserve_margin_j"]), f"{balance['phase_id']}.reserve_margin")
    require(margin >= 0.0, f"{balance['phase_id']}: reserve requirement not met")
    close(
        float(balance["continuous_source_power_w"]),
        float(bus["source_energy_j"]) / float(balance["duration_s"]),
        f"{balance['phase_id']}.continuous_source_power_w",
    )
    require(
        float(balance["peak_source_power_w"]) <= float(balance["peak_source_power_limit_w"]),
        f"{balance['phase_id']}: peak source power exceeds declared peak limit",
    )
    require(
        float(balance["continuous_source_power_w"])
        <= float(balance["continuous_source_power_limit_w"]),
        f"{balance['phase_id']}: continuous source power exceeds declared continuous limit",
    )


def validate(fixture: dict[str, Any]) -> None:
    require(fixture["schema_version"] == "mission-chronology-fixture/0.1.0", "unsupported fixture schema")
    validate_state_chain(fixture)
    require(fixture["expected_mission_status"] in {"FEASIBLE_WITHIN_DECLARED_MODEL", "INFEASIBLE", "INDETERMINATE", "NON_CONVERGED"}, "unknown mission status")
    for balance in fixture["phase_energy_balances"]:
        validate_energy_balance(balance)
    checks = fixture["expected_checks"]
    states = {state["state_id"]: state for state in fixture["phase_states"]}
    close(states["takeoff"]["expected_mass_properties"]["total_mass_kg"], float(checks["takeoff_total_mass_kg"]), "expected takeoff mass")
    close(states["landing"]["expected_mass_properties"]["total_mass_kg"], float(checks["landing_total_mass_kg"]), "expected landing mass")
    require(float(checks["takeoff_to_cruise_fuel_burn_kg"]) == 25.0, "fuel-burn fixture contract changed")
    require(float(checks["cruise_to_landing_fuel_burn_kg"]) == 5.0, "landing fuel-burn fixture contract changed")


def expect_rejected(label: str, mutate) -> None:
    fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
    mutate(fixture)
    try:
        validate(fixture)
    except (AuditError, KeyError, TypeError, ValueError):
        print(f"negative_control=pass case={label}")
        return
    raise AuditError(f"negative control unexpectedly passed: {label}")


def validate_schema_json() -> None:
    for path in SCHEMAS:
        schema = json.loads(path.read_text(encoding="utf-8"))
        require(schema.get("$schema") == "https://json-schema.org/draft/2020-12/schema", f"{path.name}: wrong JSON Schema dialect")
        require(schema.get("type") == "object", f"{path.name}: root schema must be an object")
        require(bool(schema.get("properties")), f"{path.name}: root properties missing")


def main() -> int:
    try:
        validate_schema_json()
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        validate(fixture)
        expect_rejected(
            "stale_predecessor_digest",
            lambda d: d["phase_states"][2].update(previous_state_sha256="f" * 64),
        )
        expect_rejected(
            "state_content_digest_mismatch",
            lambda d: d["phase_states"][0]["mass_items"][0].update(mass_kg=501.0),
        )
        expect_rejected(
            "bus_energy_imbalance",
            lambda d: d["phase_energy_balances"][0]["bus"].update(auxiliary_bus_load_j=6_000_000),
        )
        expect_rejected(
            "over_limit_peak_power",
            lambda d: d["phase_energy_balances"][0].update(peak_source_power_w=120_000),
        )
        expect_rejected(
            "over_limit_continuous_power",
            lambda d: d["phase_energy_balances"][0].update(continuous_source_power_limit_w=20_000),
        )
        def mutate_fuel_gain(d: dict[str, Any]) -> None:
            cruise_state = d["phase_states"][2]
            cruise_state["mass_items"][1]["mass_kg"] = 105.0
            total, cg, inertia = mass_properties(cruise_state["mass_items"])
            cruise_state["expected_mass_properties"] = {
                "total_mass_kg": total,
                "center_of_gravity_m": cg,
                "inertia_tensor_about_cg_kg_m2": inertia,
            }
            cruise_state["fuel_consumption_since_previous_kg"] = -5.0
            rebuild_state_hashes(d)

        expect_rejected("fuel_mass_gain", mutate_fuel_gain)
        expect_rejected(
            "reserve_shortfall",
            lambda d: d["phase_energy_balances"][0]["energy_store"].update(
                usable_energy_start_j=140_000_000,
                usable_energy_end_j=40_000_000,
                reserve_required_j=50_000_000,
                reserve_margin_j=-10_000_000,
            ),
        )
        expect_rejected(
            "false_feasible_unassessed_phases",
            lambda d: d.update(expected_mission_status="FEASIBLE_WITHIN_DECLARED_MODEL"),
        )
        expect_rejected(
            "duplicate_mass_item",
            lambda d: d["phase_states"][0]["mass_items"].append(copy.deepcopy(d["phase_states"][0]["mass_items"][0])),
        )
        require(fixture["expected_mission_status"] == "INDETERMINATE", "the fixture must not claim whole-mission feasibility")
        print(
            f"mission_chronology_oracle=pass states={len(fixture['phase_states'])} "
            f"phases={len(fixture['mission_timeline']['phases'])} "
            f"balances={len(fixture['phase_energy_balances'])} independent=python-stdlib"
        )
    except (OSError, AuditError, KeyError, TypeError, ValueError) as error:
        print(f"mission_chronology_oracle_error={error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
