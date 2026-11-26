#!/usr/bin/env python3
"""Independent standard-library oracle for SAFRB synthetic fuel-service fixtures.

This is a test-only reference implementation. It is not imported by the Rust
crate or used in production. It independently recomputes inventory, hourly
conversion/generation, service verdicts, temporal-ablation counts and the
planner/oracle information boundary.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any


HORIZON_HOURS = 72
RESTORATION_CONFIRMATION_HOURS = 4
U64_MASK = (1 << 64) - 1
ORACLE_ONLY_KEYS = {
    "openingGrams",
    "acceptedInflowGrams",
    "withdrawalsGrams",
    "declaredLossesGrams",
    "closingGrams",
    "usableGrams",
    "accessibleGrams",
    "deliverableGrams",
    "inputGrams",
    "outputGrams",
    "declaredProcessLossGrams",
    "fuelConsumedGrams",
    "grossElectricityWh",
    "networkDeliverableWh",
    "deliveredLowerWh",
    "deliveredUpperWh",
    "disposition",
    "hourlyTrajectory",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows = []
    with path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, 1):
            if not line.strip():
                continue
            try:
                value = json.loads(line)
            except json.JSONDecodeError as exc:
                raise AssertionError(f"{path}:{line_number}: invalid JSON: {exc}") from exc
            require(isinstance(value, dict), f"{path}:{line_number}: JSON row is not an object")
            rows.append(value)
    return rows


def mix_seed(seed: int, archetype_index: int) -> int:
    value = (seed ^ (archetype_index << 32)) & U64_MASK
    value = (value + 0x9E3779B97F4A7C15) & U64_MASK
    value = ((value ^ (value >> 30)) * 0xBF58476D1CE4E5B9) & U64_MASK
    value = ((value ^ (value >> 27)) * 0x94D049BB133111EB) & U64_MASK
    return (value ^ (value >> 31)) & U64_MASK


def expected_opening_measurement(opening: int, uncertainty_bps: int) -> tuple[int, int]:
    return (
        opening * (10_000 - uncertainty_bps) // 10_000,
        opening * (10_000 + uncertainty_bps) // 10_000,
    )


def disposition(telemetry: str, lower: int, upper: int, floor: int) -> str:
    if telemetry == "stale_or_conflicting":
        return "indeterminate"
    if lower >= floor:
        return "guaranteed"
    if upper < floor:
        return "confirmed_shortfall"
    return "indeterminate"


def validate_pair(observation: dict[str, Any], truth: dict[str, Any]) -> dict[str, bool]:
    require(observation["recordKind"] == "planner_observation", "planner row has wrong recordKind")
    require(truth["recordKind"] == "oracle_truth", "oracle row has wrong recordKind")
    require(observation["caseId"] == truth["caseId"], "planner/oracle case ID mismatch")
    require(observation["schemaVersion"] == truth["schemaVersion"] == "0.1.0", "unsupported schema version")
    require(observation["evidenceMaturity"] == truth["evidenceMaturity"] == "synthetic_fixture", "fixture promoted beyond synthetic evidence")
    require(observation["horizonHours"] == truth["horizonHours"] == HORIZON_HOURS, "unexpected horizon")

    # Check the blind boundary recursively by key name.
    def walk(value: Any, where: str) -> None:
        if isinstance(value, dict):
            for key, child in value.items():
                require(key not in ORACLE_ONLY_KEYS, f"oracle-only field {where}.{key} leaked to planner input")
                walk(child, f"{where}.{key}")
        elif isinstance(value, list):
            for index, child in enumerate(value):
                walk(child, f"{where}[{index}]")

    walk(observation, "planner")

    case_id = truth["caseId"]
    inventory = truth["inventory"]
    transport = truth["transport"]
    conversion = truth["conversion"]
    generator = truth["generator"]
    service = truth["service"]
    rows = truth["hourlyTrajectory"]
    obs_inventory = observation["inventoryObservation"]
    obs_transport = observation["transportCapability"]
    obs_conversion = observation["conversionCapability"]
    obs_generator = observation["generatorCapability"]
    obs_service = observation["serviceRequirement"]
    obs_rows = observation["hourlyObservations"]

    require(len(rows) == len(obs_rows) == HORIZON_HOURS, f"{case_id}: hourly horizon length mismatch")
    require([row["hourIndex"] for row in rows] == list(range(HORIZON_HOURS)), f"{case_id}: oracle hour index/order")
    require([row["hourIndex"] for row in obs_rows] == list(range(HORIZON_HOURS)), f"{case_id}: observation hour index/order")

    # Inventory: exact mass balance, opening-stock-only observation and authority.
    opening = inventory["openingGrams"]
    inflow = inventory["acceptedInflowGrams"]
    withdrawals = inventory["withdrawalsGrams"]
    losses = inventory["declaredLossesGrams"]
    closing = inventory["closingGrams"]
    require(opening + inflow == withdrawals + losses + closing, f"{case_id}: inventory mass balance")
    require(withdrawals == transport["deliverableGrams"], f"{case_id}: route withdrawals mismatch")
    arrival = transport["cargoArrivalHour"]
    require((arrival < HORIZON_HOURS) == (inflow > 0), f"{case_id}: arrival/inflow horizon classification")
    require(closing >= 0, f"{case_id}: negative closing inventory")

    gauge_bps = inventory["measurementUncertaintyBasisPoints"]
    expected_lower, expected_upper = expected_opening_measurement(opening, gauge_bps)
    require(inventory["observedLowerGrams"] == expected_lower, f"{case_id}: inventory lower observation")
    require(inventory["observedUpperGrams"] == expected_upper, f"{case_id}: inventory upper observation")
    require(obs_inventory["observedLowerGrams"] == expected_lower, f"{case_id}: planner observed lower differs from opening measurement")
    require(obs_inventory["observedUpperGrams"] == expected_upper, f"{case_id}: planner observed upper differs from opening measurement")
    require(obs_inventory["product"] == inventory["product"], f"{case_id}: planner product mismatch")
    require(obs_inventory["measurementUncertaintyBasisPoints"] == gauge_bps, f"{case_id}: planner measurement uncertainty mismatch")

    usable = expected_lower - inventory["quarantinedGrams"] - inventory["operatingHeelGrams"]
    require(usable == inventory["usableGrams"], f"{case_id}: usable stock does not reconcile")
    require(inventory["encumberedGrams"] <= usable, f"{case_id}: encumbrance exceeds usable stock")
    archetype_seed = truth["seed"]
    authority = archetype_seed % 10 != 7
    require(inventory["authorityAvailable"] == authority, f"{case_id}: authority seed rule")
    expected_encumbered = usable // 10 if archetype_seed % 4 == 0 else 0
    require(inventory["encumberedGrams"] == expected_encumbered, f"{case_id}: encumbrance seed rule")
    accessible = usable - expected_encumbered if authority else 0
    require(inventory["accessibleGrams"] == accessible, f"{case_id}: accessible stock")
    require(obs_inventory["authorityAvailable"] == authority, f"{case_id}: planner authority observation")

    # Transport: capacity, delay and opening-stock conservative delivery boundary.
    primary = transport["primaryCapacityGramsPerHour"]
    alternate = transport["alternateCapacityGramsPerHour"]
    capacity = primary + alternate
    require(capacity == transport["effectiveCapacityGramsPerHour"], f"{case_id}: path capacity sum")
    delay = transport["transitDelayHours"]
    require(0 <= delay <= HORIZON_HOURS, f"{case_id}: route delay outside horizon")
    require(transport["deliverableGrams"] <= accessible, f"{case_id}: route delivery exceeds opening accessible stock")
    require(transport["deliverableGrams"] <= capacity * (HORIZON_HOURS - delay), f"{case_id}: route delivery exceeds time-bounded throughput")
    require(obs_transport["cargoArrivalHour"] == arrival, f"{case_id}: ETA observation")
    require(obs_transport["effectiveCapacityGramsPerHour"] == capacity, f"{case_id}: planner transport capacity")

    # Product conversion: hourly input/output/loss and aggregate totals.
    require(conversion["inputProduct"] == inventory["product"], f"{case_id}: feedstock product mismatch")
    require(conversion["outputProduct"] == "diesel", f"{case_id}: output product mismatch")
    require(conversion["inputGrams"] <= transport["deliverableGrams"], f"{case_id}: process input exceeds route delivery")
    require(conversion["inputGrams"] <= conversion["processCapacityGrams"], f"{case_id}: process input exceeds total capacity")
    require(obs_conversion["processCapacityGrams"] == conversion["processCapacityGrams"], f"{case_id}: planner process capacity mismatch")
    require(obs_conversion["yieldBasisPoints"] == conversion["yieldBasisPoints"], f"{case_id}: planner yield mismatch")

    route_total = 0
    input_total = 0
    output_total = 0
    loss_total = 0
    fuel_total = 0
    generation_total = 0
    network_total = 0
    demand_total = 0
    floor_total = 0
    lower_total = 0
    upper_total = 0
    power_cap = generator["ratedPowerCapacityKw"] * 1_000 * generator["derateBasisPoints"] // 10_000
    expected_uncertainty = service["uncertaintyBasisPoints"]
    expected_telemetry = "stale_or_conflicting" if archetype_seed % 10 == 9 else "fresh_synthetic"
    require(service["telemetryState"] == expected_telemetry, f"{case_id}: telemetry seed rule")

    for index, row in enumerate(rows):
        obs_row = obs_rows[index]
        expected_time = truth["scenarioStartUnixS"] + index * 3_600
        require(row["intervalStartUnixS"] == expected_time, f"{case_id}: oracle time grid at hour {index}")
        require(obs_row["intervalStartUnixS"] == expected_time, f"{case_id}: planner time grid at hour {index}")
        require(obs_row["demandWh"] == row["demandWh"], f"{case_id}: planner hourly demand mismatch")
        require(obs_row["criticalFloorWh"] == row["criticalFloorWh"], f"{case_id}: planner hourly floor mismatch")
        require(obs_row["telemetryState"] == row["telemetryState"], f"{case_id}: planner telemetry mismatch")

        if index < delay:
            require(row["routeDeliveryGrams"] == 0, f"{case_id}: route delivers before transit delay at hour {index}")
            require(row["conversionInputGrams"] == 0, f"{case_id}: process input before route at hour {index}")
            require(row["fuelConsumedGrams"] == 0, f"{case_id}: generator fuel before route at hour {index}")
            require(row["grossGenerationWh"] == 0, f"{case_id}: generation before route at hour {index}")
            require(row["networkDeliverableWh"] == 0, f"{case_id}: network service before route at hour {index}")

        require(row["routeDeliveryGrams"] <= capacity, f"{case_id}: hourly route capacity at hour {index}")
        require(row["conversionInputGrams"] <= row["routeDeliveryGrams"], f"{case_id}: hourly feedstock exceeds delivery at hour {index}")
        expected_output = row["conversionInputGrams"] * conversion["yieldBasisPoints"] // 10_000
        require(row["conversionOutputGrams"] == expected_output, f"{case_id}: hourly conversion yield at hour {index}")
        require(row["processLossGrams"] + row["conversionOutputGrams"] == row["conversionInputGrams"], f"{case_id}: hourly process mass balance at hour {index}")
        require(row["fuelConsumedGrams"] <= row["conversionOutputGrams"], f"{case_id}: hourly fuel exceeds product output at hour {index}")
        if not generator["fuelGradeCompatible"]:
            require(row["fuelConsumedGrams"] == 0, f"{case_id}: incompatible fuel consumed at hour {index}")

        fuel_energy_wh = (
            row["fuelConsumedGrams"]
            * generator["syntheticLhvMilliWhPerGram"]
            * generator["efficiencyBasisPoints"]
            // 10_000
            // 1_000
        )
        expected_generation = min(fuel_energy_wh, power_cap)
        require(row["grossGenerationWh"] == expected_generation, f"{case_id}: hourly generation energy/power cap at hour {index}")
        expected_network = row["grossGenerationWh"] * generator["networkDeliveryBasisPoints"] // 10_000
        require(row["networkDeliverableWh"] == expected_network, f"{case_id}: hourly network delivery at hour {index}")

        demand = row["demandWh"]
        floor = row["criticalFloorWh"]
        lower = min(demand, row["networkDeliverableWh"] * (10_000 - expected_uncertainty) // 10_000)
        upper = min(demand, row["networkDeliverableWh"] * (10_000 + expected_uncertainty) // 10_000)
        require(floor == demand * 8_000 // 10_000, f"{case_id}: hourly critical floor at hour {index}")
        require(row["deliveredLowerWh"] == lower, f"{case_id}: hourly lower bound at hour {index}")
        require(row["deliveredUpperWh"] == upper, f"{case_id}: hourly upper bound at hour {index}")
        require(row["telemetryState"] == expected_telemetry, f"{case_id}: hourly telemetry at hour {index}")
        expected_row_disposition = disposition(expected_telemetry, lower, upper, floor)
        require(row["disposition"] == expected_row_disposition, f"{case_id}: hourly service verdict at hour {index}")

        route_total += row["routeDeliveryGrams"]
        input_total += row["conversionInputGrams"]
        output_total += row["conversionOutputGrams"]
        loss_total += row["processLossGrams"]
        fuel_total += row["fuelConsumedGrams"]
        generation_total += row["grossGenerationWh"]
        network_total += row["networkDeliverableWh"]
        demand_total += demand
        floor_total += floor
        lower_total += lower
        upper_total += upper

    require(route_total == transport["deliverableGrams"], f"{case_id}: hourly route total")
    require(input_total == conversion["inputGrams"], f"{case_id}: hourly conversion input total")
    require(output_total == conversion["outputGrams"], f"{case_id}: hourly conversion output total")
    require(loss_total == conversion["declaredProcessLossGrams"], f"{case_id}: hourly process loss total")
    require(fuel_total == generator["fuelConsumedGrams"], f"{case_id}: hourly fuel total")
    require(generation_total == generator["grossElectricityWh"], f"{case_id}: hourly generation total")
    require(network_total == generator["networkDeliverableWh"], f"{case_id}: hourly network total")
    require(demand_total == service["demandWh"], f"{case_id}: hourly demand total")
    require(floor_total == service["criticalFloorWh"], f"{case_id}: hourly floor total")
    require(lower_total == service["deliveredLowerWh"], f"{case_id}: hourly lower total")
    require(upper_total == service["deliveredUpperWh"], f"{case_id}: hourly upper total")

    has_shortfall = any(row["disposition"] == "confirmed_shortfall" for row in rows)
    has_indeterminate = any(row["disposition"] == "indeterminate" for row in rows)
    expected_service_disposition = (
        "confirmed_shortfall" if has_shortfall
        else "indeterminate" if has_indeterminate
        else "guaranteed"
    )
    require(service["disposition"] == expected_service_disposition, f"{case_id}: horizon verdict masks hourly state")
    require(service["demandWh"] == demand_total, f"{case_id}: service demand")
    require(service["criticalFloorWh"] == floor_total, f"{case_id}: service floor")
    require(service["deliveredLowerWh"] == lower_total, f"{case_id}: service lower total")
    require(service["deliveredUpperWh"] == upper_total, f"{case_id}: service upper total")
    require(service["uncertaintyBasisPoints"] == 500, f"{case_id}: service uncertainty")
    require(obs_service["demandWh"] == service["demandWh"], f"{case_id}: planner service demand")
    require(obs_service["criticalFloorWh"] == service["criticalFloorWh"], f"{case_id}: planner service floor")

    # The aggregate score is compared to the independently recomputed hourly verdict.
    aggregate_pass = lower_total >= floor_total
    temporal_disruption = has_shortfall or has_indeterminate
    recovery_confirmed = False
    if temporal_disruption:
        first_disruption = next(
            index for index, row in enumerate(rows)
            if row["disposition"] != "guaranteed"
        )
        streak = 0
        for row in rows[first_disruption:]:
            if row["disposition"] == "guaranteed":
                streak += 1
                if streak >= RESTORATION_CONFIRMATION_HOURS:
                    recovery_confirmed = True
                    break
            else:
                streak = 0
    return {
        "aggregate_pass": aggregate_pass,
        "temporal_guaranteed": service["disposition"] == "guaranteed",
        "temporal_shortfall": service["disposition"] == "confirmed_shortfall",
        "temporal_indeterminate": service["disposition"] == "indeterminate",
        "aggregate_false_guarantee": aggregate_pass and service["disposition"] != "guaranteed",
        "false_with_hourly_shortfall": aggregate_pass and has_shortfall,
        "false_with_indeterminate": aggregate_pass and has_indeterminate,
        "had_disruption": temporal_disruption,
        "recovery_confirmed": recovery_confirmed,
    }


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: independent_oracle.py CORPUS_DIRECTORY", file=sys.stderr)
        return 2
    root = Path(sys.argv[1])
    observations = read_jsonl(root / "planner-observations.jsonl")
    truths = read_jsonl(root / "oracle-truth.jsonl")
    report = json.loads((root / "temporal-ablation-report.json").read_text(encoding="utf-8"))

    require(len(observations) == len(truths) == 120, "expected exactly 120 observation/oracle rows")
    require(len({row["caseId"] for row in observations}) == 120, "planner case IDs are not unique")
    require(len({row["caseId"] for row in truths}) == 120, "oracle case IDs are not unique")
    require([row["caseId"] for row in observations] == [row["caseId"] for row in truths], "planner/oracle case order mismatch")

    results = [
        validate_pair(observation, truth)
        for observation, truth in zip(observations, truths)
    ]

    expected = {
        "casesTotal": len(results),
        "aggregateEnergyGuaranteeCount": sum(row["aggregate_pass"] for row in results),
        "temporalGuaranteeCount": sum(row["temporal_guaranteed"] for row in results),
        "temporalConfirmedShortfallCount": sum(row["temporal_shortfall"] for row in results),
        "temporalIndeterminateCount": sum(row["temporal_indeterminate"] for row in results),
        "aggregateFalseGuaranteeCount": sum(row["aggregate_false_guarantee"] for row in results),
        "falseGuaranteeWithHourlyShortfallCount": sum(row["false_with_hourly_shortfall"] for row in results if row["aggregate_false_guarantee"]),
        "falseGuaranteeWithIndeterminateEvidenceCount": sum(row["false_with_indeterminate"] for row in results if row["aggregate_false_guarantee"]),
        "sustainedRecoveryConfirmedCount": sum(row["recovery_confirmed"] for row in results if row["had_disruption"]),
        "rightCensoredRecoveryCount": sum(not row["recovery_confirmed"] for row in results if row["had_disruption"]),
        "restorationConfirmationHours": RESTORATION_CONFIRMATION_HOURS,
    }
    require(report == {
        "reportVersion": "0.1.0",
        "evidenceMaturity": "synthetic_fixture",
        **expected,
    }, "Rust temporal ablation report differs from the independent Python oracle")

    require(expected["aggregateFalseGuaranteeCount"] > 0, "benchmark contains no aggregate-only false guarantee")
    require(expected["falseGuaranteeWithHourlyShortfallCount"] > 0, "benchmark contains no hourly shortfall counterexample")
    require(expected["falseGuaranteeWithIndeterminateEvidenceCount"] > 0, "benchmark contains no indeterminate-evidence counterexample")
    require(expected["sustainedRecoveryConfirmedCount"] > 0, "benchmark contains no sustained restoration confirmation")
    require(expected["rightCensoredRecoveryCount"] > 0, "benchmark contains no right-censored recovery")

    print(
        "independent fuel-service oracle PASS: "
        f"{len(results)} cases, {len(results) * HORIZON_HOURS} hourly intervals, "
        f"{expected['aggregateFalseGuaranteeCount']} aggregate-only false guarantees, "
        f"{expected['sustainedRecoveryConfirmedCount']} sustained recoveries, "
        f"{expected['rightCensoredRecoveryCount']} right-censored episodes"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as error:
        print(f"independent oracle FAIL: {error}", file=sys.stderr)
        raise SystemExit(1)
