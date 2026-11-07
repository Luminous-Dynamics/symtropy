#!/usr/bin/env python3
"""Independent stdlib oracle for carrier-aware aircraft phase-energy fixtures.

This is intentionally separate from production Rust. It recomputes fuel,
prime-mover, generator, storage, electrical-bus, motor and propulsor balances
from raw JSON values. Inputs are synthetic, and passing does not validate
physical component maps or aircraft performance.
"""

from __future__ import annotations

import copy
import json
import math
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "data" / "mission" / "phase_energy_planes_fixtures_v0_2.json"
SCHEMA = ROOT / "schemas" / "phase-energy-balance-v0.2.schema.json"
TOLERANCE_J = 1.0
REL_TOLERANCE = 1.0e-10


class AuditError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AuditError(message)


def finite_nonnegative(value: Any, name: str) -> float:
    require(isinstance(value, (int, float)), f"{name}: expected number")
    number = float(value)
    require(math.isfinite(number) and number >= 0.0, f"{name}: must be finite and >= 0")
    return number


def positive(value: Any, name: str) -> float:
    number = finite_nonnegative(value, name)
    require(number > 0.0, f"{name}: must be > 0")
    return number


def digest(value: Any, name: str) -> str:
    require(
        isinstance(value, str)
        and len(value) == 64
        and all(ch in "0123456789abcdefABCDEF" for ch in value),
        f"{name}: expected 64-character hexadecimal digest",
    )
    return value.lower()


def close(actual: float, expected: float, name: str, tolerance: float = TOLERANCE_J) -> bool:
    require(
        math.isfinite(actual) and math.isfinite(expected),
        f"{name}: non-finite residual or energy",
    )
    return abs(actual - expected) <= max(tolerance, REL_TOLERANCE * max(1.0, abs(actual), abs(expected)))


def unique_ids(rows: list[dict[str, Any]], key: str, label: str) -> None:
    values = [row[key] for row in rows]
    require(all(isinstance(value, str) and value for value in values), f"{label}: empty identity")
    require(len(values) == len(set(values)), f"{label}: duplicate identity")


def evaluate(balance: dict[str, Any], fixture: dict[str, Any]) -> dict[str, Any]:
    require(balance["schema_version"] == "phase-energy-balance/0.2.0", "unsupported balance schema")
    require(balance["mission_id"] == fixture["expected_mission_id"], "mission identity mismatch")
    require(
        digest(balance["configuration_sha256"], "configuration_sha256")
        == digest(fixture["expected_configuration_sha256"], "expected configuration"),
        "configuration identity mismatch",
    )
    require(
        digest(balance["start_state_sha256"], "start_state_sha256")
        == digest(fixture["expected_start_state_sha256"], "expected start state"),
        "start-state identity mismatch",
    )
    require(
        digest(balance["end_state_sha256"], "end_state_sha256")
        == digest(fixture["expected_end_state_sha256"], "expected end state"),
        "end-state identity mismatch",
    )

    duration = positive(balance["duration_s"], "duration_s")
    tolerance = finite_nonnegative(balance["tolerance_j"], "tolerance_j")
    errors: list[str] = []

    fuels = balance["fuel_states"]
    engines = balance["prime_movers"]
    generators = balance["generators"]
    stores = balance["stores"]
    motors = balance["motor_drives"]
    propulsors = balance["propulsors"]
    for rows, key, label in (
        (fuels, "fuel_source_id", "fuel source"),
        (engines, "prime_mover_id", "prime mover"),
        (generators, "generator_id", "generator"),
        (stores, "store_id", "energy store"),
        (motors, "motor_id", "motor"),
        (propulsors, "propulsor_id", "propulsor"),
    ):
        unique_ids(rows, key, label)

    fuel_by_id = {row["fuel_source_id"]: row for row in fuels}
    engine_by_id = {row["prime_mover_id"]: row for row in engines}
    generator_inputs_by_engine: dict[str, float] = {}
    generator_bus_total = 0.0

    for fuel in fuels:
        digest(fuel["evidence_sha256"], "fuel evidence")
        start = finite_nonnegative(fuel["chemical_energy_start_j"], "fuel start")
        consumed = finite_nonnegative(fuel["chemical_energy_consumed_j"], "fuel consumed")
        end = finite_nonnegative(fuel["chemical_energy_end_j"], "fuel end")
        reserve = finite_nonnegative(fuel["required_reserve_j"], "fuel reserve")
        residual = start - consumed - end
        margin = end - reserve
        if not close(residual, 0.0, f"fuel {fuel['fuel_source_id']} residual", tolerance):
            errors.append(f"fuel_conservation:{fuel['fuel_source_id']}")
        if margin < -tolerance:
            errors.append(f"fuel_reserve:{fuel['fuel_source_id']}")

    for engine in engines:
        digest(engine["model_evidence_sha256"], "prime mover model evidence")
        fuel_id = engine["fuel_source_id"]
        require(fuel_id in fuel_by_id, f"prime mover {engine['prime_mover_id']}: missing fuel source")
        fuel_input = finite_nonnegative(engine["fuel_energy_input_j"], "engine fuel input")
        mechanical = finite_nonnegative(engine["mechanical_output_j"], "engine mechanical output")
        heat_loss = finite_nonnegative(engine["thermal_and_exhaust_loss_j"], "engine thermal loss")
        average_mechanical_limit = positive(engine["average_mechanical_output_power_limit_w"], "engine average mechanical output limit")
        peak_mechanical = finite_nonnegative(engine["peak_mechanical_output_power_w"], "engine peak mechanical output")
        peak_mechanical_limit = positive(engine["peak_mechanical_output_power_limit_w"], "engine peak mechanical output limit")
        average_mechanical = mechanical / duration
        direct_shaft = finite_nonnegative(engine["direct_propulsor_shaft_j"], "engine direct shaft")
        accessory = finite_nonnegative(engine["accessory_mechanical_load_j"], "engine accessory load")
        mechanical_loss = finite_nonnegative(engine["mechanical_distribution_loss_j"], "engine mechanical loss")
        fuel_residual = fuel_input - mechanical - heat_loss
        fuel_state_residual = fuel_input - float(fuel_by_id[fuel_id]["chemical_energy_consumed_j"])
        shaft_used_by_generators = sum(
            finite_nonnegative(gen["mechanical_input_j"], "generator mechanical input")
            for gen in generators
            if gen["prime_mover_id"] == engine["prime_mover_id"]
        )
        shaft_residual = mechanical - shaft_used_by_generators - direct_shaft - accessory - mechanical_loss
        for residual, name in (
            (fuel_residual, f"prime_mover_fuel:{engine['prime_mover_id']}"),
            (fuel_state_residual, f"fuel_state_link:{engine['prime_mover_id']}"),
            (shaft_residual, f"prime_mover_shaft:{engine['prime_mover_id']}"),
        ):
            if not close(residual, 0.0, name, tolerance):
                errors.append(name)

    for generator in generators:
        digest(generator["model_evidence_sha256"], "generator model evidence")
        require(
            generator["prime_mover_id"] in engine_by_id,
            f"generator {generator['generator_id']}: missing prime mover",
        )
        mechanical = finite_nonnegative(generator["mechanical_input_j"], "generator mechanical input")
        electrical = finite_nonnegative(generator["electrical_output_to_bus_j"], "generator electrical output")
        loss = finite_nonnegative(generator["conversion_loss_j"], "generator conversion loss")
        residual = mechanical - electrical - loss
        if not close(residual, 0.0, f"generator:{generator['generator_id']}", tolerance):
            errors.append(f"generator:{generator['generator_id']}")
        generator_inputs_by_engine[generator["prime_mover_id"]] = (
            generator_inputs_by_engine.get(generator["prime_mover_id"], 0.0) + mechanical
        )
        generator_bus_total += electrical

    store_discharge_total = 0.0
    store_charge_total = 0.0
    store_results = []
    for store in stores:
        digest(store["installed_system_evidence_sha256"], "installed-system evidence")
        digest(store["operating_envelope_evidence_sha256"], "store operating-envelope evidence")
        start = finite_nonnegative(store["usable_energy_start_j"], "store start energy")
        end = finite_nonnegative(store["usable_energy_end_j"], "store end energy")
        capacity = positive(store["usable_capacity_j"], "store capacity")
        reserve = finite_nonnegative(store["reserve_floor_j"], "store reserve floor")
        discharge = finite_nonnegative(store["discharge_to_bus_j"], "store discharge to bus")
        charge = finite_nonnegative(store["charge_from_bus_j"], "store charge from bus")
        eta_d = positive(store["discharge_efficiency"], "discharge efficiency")
        eta_c = positive(store["charge_efficiency"], "charge efficiency")
        require(eta_d <= 1.0 and eta_c <= 1.0, "store efficiencies must be <= 1")
        soh = finite_nonnegative(store["state_of_health_fraction"], "store SOH")
        min_soh = finite_nonnegative(store["minimum_state_of_health_fraction"], "minimum SOH")
        require(soh <= 1.0 and min_soh <= 1.0, "SOH fractions must be <= 1")
        temperature = float(store["temperature_c"])
        tmin = float(store["minimum_temperature_c"])
        tmax = float(store["maximum_temperature_c"])
        require(all(math.isfinite(x) for x in (temperature, tmin, tmax)), "non-finite store temperature")
        require(tmin <= tmax, "store temperature envelope is inverted")

        predicted_end = start - discharge / eta_d + charge * eta_c
        residual = predicted_end - end
        margin = end - reserve
        avg_discharge = discharge / duration
        avg_charge = charge / duration
        avg_discharge_limit = positive(store["average_discharge_power_limit_w"], "average discharge limit")
        peak_discharge = finite_nonnegative(store["peak_discharge_power_w"], "peak discharge")
        peak_discharge_limit = positive(store["peak_discharge_power_limit_w"], "peak discharge limit")
        avg_charge_limit = positive(store["average_charge_power_limit_w"], "average charge limit")
        peak_charge = finite_nonnegative(store["peak_charge_power_w"], "peak charge")
        peak_charge_limit = positive(store["peak_charge_power_limit_w"], "peak charge limit")

        if not close(residual, 0.0, f"store:{store['store_id']}", tolerance):
            errors.append(f"store_transition:{store['store_id']}")
        if margin < -tolerance:
            errors.append(f"store_reserve:{store['store_id']}")
        if start > capacity + tolerance or end > capacity + tolerance:
            errors.append(f"store_capacity:{store['store_id']}")
        if soh < min_soh or temperature < tmin or temperature > tmax:
            errors.append(f"store_operating_domain:{store['store_id']}")
        if avg_discharge > avg_discharge_limit or peak_discharge > peak_discharge_limit:
            errors.append(f"store_discharge_power:{store['store_id']}")
        if avg_charge > avg_charge_limit or peak_charge > peak_charge_limit:
            errors.append(f"store_charge_power:{store['store_id']}")

        store_discharge_total += discharge
        store_charge_total += charge
        store_results.append({
            "store_id":store["store_id"],
            "predicted_end_j":predicted_end,
            "actual_end_j":end,
            "residual_j":residual,
            "reserve_margin_j":margin,
            "average_discharge_power_w":avg_discharge,
            "average_charge_power_w":avg_charge
        })

    bus = balance["bus_loads"]
    aux = finite_nonnegative(bus["auxiliary_load_j"], "auxiliary load")
    thermal = finite_nonnegative(bus["thermal_electrical_load_j"], "thermal electrical load")
    distribution = finite_nonnegative(bus["distribution_loss_j"], "distribution loss")
    motor_bus_total = 0.0
    motor_shaft_total = 0.0
    for motor in motors:
        digest(motor["model_evidence_sha256"], "motor model evidence")
        bus_input = finite_nonnegative(motor["bus_input_j"], "motor bus input")
        inverter_loss = finite_nonnegative(motor["inverter_loss_j"], "inverter loss")
        motor_loss = finite_nonnegative(motor["motor_loss_j"], "motor loss")
        shaft = finite_nonnegative(motor["shaft_output_j"], "motor shaft output")
        residual = bus_input - inverter_loss - motor_loss - shaft
        if not close(residual, 0.0, f"motor:{motor['motor_id']}", tolerance):
            errors.append(f"motor:{motor['motor_id']}")
        motor_bus_total += bus_input
        motor_shaft_total += shaft

    direct_shaft_total = sum(float(engine["direct_propulsor_shaft_j"]) for engine in engines)
    expected_propulsor_shaft_total = direct_shaft_total + motor_shaft_total
    actual_propulsor_shaft_total = 0.0
    for propulsor in propulsors:
        digest(propulsor["model_evidence_sha256"], "propulsor model evidence")
        shaft_input = finite_nonnegative(propulsor["shaft_input_j"], "propulsor shaft input")
        useful = finite_nonnegative(propulsor["useful_propulsive_energy_j"], "useful propulsive energy")
        loss = finite_nonnegative(propulsor["propulsor_loss_j"], "propulsor loss")
        residual = shaft_input - useful - loss
        if not close(residual, 0.0, f"propulsor:{propulsor['propulsor_id']}", tolerance):
            errors.append(f"propulsor:{propulsor['propulsor_id']}")
        actual_propulsor_shaft_total += shaft_input

    if not close(actual_propulsor_shaft_total, expected_propulsor_shaft_total, "shaft combiner", tolerance):
        errors.append("shaft_combiner")

    total_supply = generator_bus_total + store_discharge_total
    total_demand = motor_bus_total + aux + thermal + store_charge_total + distribution
    bus_residual = total_supply - total_demand
    if not close(bus_residual, 0.0, "electrical bus", tolerance):
        errors.append("electrical_bus")

    tolerance_w = finite_nonnegative(balance["tolerance_w"], "tolerance_w")
    peak = finite_nonnegative(balance["peak_bus_supply_power_w"], "phase peak bus supply power")
    peak_limit = positive(balance["peak_bus_supply_power_limit_w"], "peak bus supply power limit")
    continuous_limit = positive(balance["continuous_bus_supply_power_limit_w"], "continuous bus supply power limit")
    continuous = total_supply / duration
    if peak > peak_limit + tolerance_w or peak + tolerance_w < continuous:
        errors.append("peak_bus_supply_power")
    if continuous > continuous_limit + tolerance_w:
        errors.append("continuous_bus_supply_power")

    require(bool(engines) or bool(stores), "phase must declare a generator/prime mover or energy store")
    status = "infeasible" if errors else "conserved_within_declared_model"
    return {
        "status":status,
        "errors":errors,
        "bus_residual_j":bus_residual,
        "continuous_source_power_w":continuous,
        "store_results":store_results
    }


def validate_fixture(fixture: dict[str, Any]) -> None:
    require(fixture["schema_version"] == "phase-energy-planes-fixtures/0.2.0", "unsupported fixture schema")
    require(fixture["expected_mission_id"] == "SYNTH-PHASE-ENERGY-PLANES-001", "wrong fixture mission")
    digest(fixture["expected_configuration_sha256"], "fixture config")
    digest(fixture["expected_start_state_sha256"], "fixture start")
    digest(fixture["expected_end_state_sha256"], "fixture end")
    seen = set()
    for case in fixture["cases"]:
        require(case["fixture_id"] not in seen, "duplicate fixture ID")
        seen.add(case["fixture_id"])
        report = evaluate(case["balance"], fixture)
        require(report["status"] == case["expected_status"], f"{case['fixture_id']}: expected {case['expected_status']}, got {report['status']} ({report['errors']})")
    expected = {
        "battery-only-discharge",
        "generator-only",
        "series-hybrid-battery-buffer",
        "parallel-hybrid-direct-shaft-assist",
        "generator-charging-battery",
        "generator-battery-supercapacitor",
    }
    require(seen == expected, "v0.2 fixture topology coverage changed")


def expect_bad(label: str, mutator) -> None:
    fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
    case = fixture["cases"][0]["balance"]
    mutator(case)
    report = evaluate(case, fixture)
    require(report["status"] == "infeasible", f"{label}: hostile case unexpectedly conserved")
    print(f"negative_control=pass case={label}")


def main() -> int:
    try:
        schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
        require(schema.get("$schema") == "https://json-schema.org/draft/2020-12/schema", "schema dialect mismatch")
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        validate_fixture(fixture)
        expect_bad("storage_internal_energy_residual", lambda b: b["stores"][0].update(usable_energy_end_j=90_000_000))
        expect_bad("bus_generator_double_count", lambda b: b["generators"].append(copy.deepcopy(b["generators"][0])) if b["generators"] else b["stores"][0].update(discharge_to_bus_j=110_000_000))
        expect_bad("reserve_floor_violation", lambda b: b["stores"][0].update(reserve_floor_j=120_000_000))
        expect_bad("store_temperature_outside_envelope", lambda b: b["stores"][0].update(temperature_c=95.0))
        expect_bad("peak_store_power_violation", lambda b: b["stores"][0].update(peak_discharge_power_w=99_000))
        print(f"phase_energy_planes_oracle=pass cases={len(fixture['cases'])} independent=python-stdlib")
    except (OSError, AuditError, KeyError, TypeError, ValueError) as error:
        print(f"phase_energy_planes_oracle_error={error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
