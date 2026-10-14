#!/usr/bin/env python3
"""Independent, standard-library oracle for synthetic CRASH-BOOT resource-flow fixtures.

This is test tooling only. It is not a production planner or a life-support model.
"""
from __future__ import annotations

import json
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "docs/settlement-bootstrap/fixtures/resource_flow_reference_v0.1.json"
checks = 0


def require(condition: bool, message: str) -> None:
    global checks
    checks += 1
    if not condition:
        raise AssertionError(message)


def q(value: object) -> Fraction:
    # Fraction(str(...)) keeps JSON decimal literals such as 1.5 exact.
    return Fraction(str(value))


def interval(value: dict[str, object], label: str) -> tuple[Fraction, Fraction]:
    low, high = q(value["min"]), q(value["max"])
    require(low <= high, f"{label}: lower bound exceeds upper bound")
    return low, high


def case_index(corpus: dict[str, object]) -> dict[str, dict[str, object]]:
    cases = corpus["cases"]
    assert isinstance(cases, list)
    indexed: dict[str, dict[str, object]] = {}
    for case in cases:
        assert isinstance(case, dict)
        case_id = str(case["id"])
        require(case_id not in indexed, f"duplicate case id: {case_id}")
        indexed[case_id] = case
    return indexed


def validate_dependency_graph(case: dict[str, object]) -> str:
    nodes = case["nodes"]
    assert isinstance(nodes, list)
    by_id: dict[str, dict[str, object]] = {}
    for node in nodes:
        assert isinstance(node, dict)
        node_id = str(node["id"])
        if node_id in by_id:
            return "REJECT_DUPLICATE_NODE"
        by_id[node_id] = node
    for node in nodes:
        for dependency in node["required_dependencies"]:
            if dependency not in by_id:
                return "REJECT_UNKNOWN_DEPENDENCY"
    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(node_id: str) -> bool:
        if node_id in visiting:
            return False
        if node_id in visited:
            return True
        visiting.add(node_id)
        for dependency in by_id[node_id]["required_dependencies"]:
            if not visit(str(dependency)):
                return False
        visiting.remove(node_id)
        visited.add(node_id)
        return True

    for node_id in by_id:
        if not visit(node_id):
            return "REJECT_CYCLE"
    return "VALID"


def unavailable_dependency_closure(case: dict[str, object]) -> set[str]:
    nodes = case["nodes"]
    assert isinstance(nodes, list)
    by_id = {str(node["id"]): node for node in nodes}
    unavailable = {
        node_id
        for node_id, node in by_id.items()
        if node["state"] != "VERIFIED_AVAILABLE"
    }
    changed = True
    while changed:
        changed = False
        for node_id, node in by_id.items():
            if node_id in unavailable:
                continue
            if any(str(dep) in unavailable for dep in node["required_dependencies"]):
                unavailable.add(node_id)
                changed = True
    return unavailable


def surviving_alternative_groups(
    case: dict[str, object], unavailable: set[str]
) -> list[list[str]]:
    groups = case.get("alternative_groups", [])
    assert isinstance(groups, list)
    result: list[list[str]] = []
    for group in groups:
        assert isinstance(group, list)
        if all(str(node_id) not in unavailable for node_id in group):
            result.append([str(node_id) for node_id in group])
    return result


def demand_bounds(case: dict[str, object]) -> tuple[Fraction, Fraction]:
    population = int(case["population"])
    require(population > 0, f"{case['id']}: population must be positive")
    components = case["demand_components"]
    assert isinstance(components, dict)
    rate_low, rate_high = interval(
        components["per_person_rate_u_per_person_day"], f"{case['id']}: per-person rate"
    )
    lows: list[Fraction] = [population * rate_low]
    highs: list[Fraction] = [population * rate_high]
    for key in (
        "fixed_system_load_u_per_day",
        "growth_load_u_per_day",
        "declared_losses_u_per_day",
    ):
        low, high = interval(components[key], f"{case['id']}: {key}")
        require(low >= 0, f"{case['id']}: {key} cannot be negative")
        lows.append(low)
        highs.append(high)
    return sum(lows, Fraction(0)), sum(highs, Fraction(0))


def main() -> None:
    corpus = json.loads(FIXTURE.read_text(encoding="utf-8"))
    require(corpus["contract_version"] == "0.1.0", "unexpected contract version")
    require(
        corpus["authority"] == "NON_AUTHORITATIVE_SYNTHETIC_REFERENCE",
        "fixture must remain explicitly non-authoritative",
    )
    require(
        corpus["population_scale_probes"] == [1, 12, 100, 1000, 10000, 30000],
        "population scale probes changed without an explicit contract revision",
    )
    scale = corpus["population_scaling_reference"]
    scale_rate = q(scale["per_person_rate_u_per_person_day"])
    scale_rows = scale["expected_person_component_u_per_day"]
    require(
        [int(row["population"]) for row in scale_rows] == corpus["population_scale_probes"],
        "scale arithmetic rows must cover each declared population probe in order",
    )
    for row in scale_rows:
        require(
            int(row["population"]) * scale_rate == q(row["demand"]),
            f"per-person scale arithmetic mismatch at population {row['population']}",
        )
    require(
        corpus["units"]["abstract_resource_unit"] == "u",
        "fixture must use abstract units, not imply a real-world consumption rate",
    )
    cases = case_index(corpus)
    require(len(cases) == 11, "unexpected reference-corpus cardinality")

    exact = cases["exact_demand_12"]
    exact_low, exact_high = demand_bounds(exact)
    exact_expected = exact["expected"]["demand_u_per_day"]
    require(exact_low == q(exact_expected["min"]) == 36, "exact demand lower oracle mismatch")
    require(exact_high == q(exact_expected["max"]) == 36, "exact demand upper oracle mismatch")

    bounded = cases["bounded_runway_12"]
    demand_low, demand_high = demand_bounds(bounded)
    expected = bounded["expected"]
    require(demand_low == q(expected["demand_u_per_day"]["min"]) == 24, "bounded demand lower mismatch")
    require(demand_high == q(expected["demand_u_per_day"]["max"]) == 43, "bounded demand upper mismatch")
    inventory_low, inventory_high = interval(bounded["inventory_u"], "inventory")
    inflow_low, inflow_high = interval(
        bounded["qualified_inflow_u_per_day"], "qualified inflow"
    )
    net_low = max(Fraction(0), demand_low - inflow_high)
    net_high = demand_high - inflow_low
    require(net_low == q(expected["net_depletion_u_per_day"]["min"]) == 16, "net depletion lower mismatch")
    require(net_high == q(expected["net_depletion_u_per_day"]["max"]) == 43, "net depletion upper mismatch")
    require(net_low > 0 and net_high > 0, "runway oracle expects positive depletion throughout bounds")
    runway_low = inventory_low / net_high
    runway_high = inventory_high / net_low
    require(
        runway_low == Fraction(expected["runway_days_lower_fraction"]),
        "conservative runway lower fraction mismatch",
    )
    require(
        runway_high == Fraction(expected["runway_days_upper_fraction"]),
        "conservative runway upper fraction mismatch",
    )
    require(
        "no probability distribution" in expected["uncertainty_interpretation"],
        "interval arithmetic must not be described as a probability estimate",
    )

    capacity_case = cases["upper_bound_exceeds_service_capacity"]
    capacity = q(capacity_case["available_capacity_u_per_day"])
    demand = bounded["expected"]["demand_u_per_day"]
    minimum_covered = capacity >= q(demand["min"])
    maximum_covered = capacity >= q(demand["max"])
    require(
        minimum_covered is capacity_case["expected"]["minimum_demand_is_covered"],
        "minimum-demand capacity oracle mismatch",
    )
    require(
        maximum_covered is capacity_case["expected"]["maximum_demand_is_covered"],
        "maximum-demand capacity oracle mismatch",
    )
    require(
        (not maximum_covered)
        and capacity_case["expected"]["status"] == "NOT_CERTIFIED_WORST_CASE",
        "point estimate must not hide possible worst-case shortfall",
    )

    distribution = cases["aggregate_capacity_hides_disconnected_group"]
    groups = distribution["groups"]
    per_person = q(distribution["per_person_demand_u_per_day"])
    group_population = sum(int(group["population"]) for group in groups)
    group_demand = sum(
        (int(group["population"]) * per_person for group in groups), Fraction(0)
    )
    require(group_population == distribution["population"] == 1000, "group population conservation mismatch")
    require(group_demand == q(distribution["expected"]["total_demand_u_per_day"]) == 2000, "group demand sum mismatch")
    nominal_covers = q(distribution["nominal_total_capacity_u_per_day"]) >= group_demand
    require(nominal_covers is distribution["expected"]["nominal_capacity_covers_total"], "nominal capacity oracle mismatch")
    uncovered = [
        group["id"]
        for group in groups
        if q(group["delivered_capacity_u_per_day"]) < int(group["population"]) * per_person
    ]
    require(uncovered == distribution["expected"]["uncovered_groups"] == ["habitat-b"], "uncovered subgroup oracle mismatch")
    require(
        (not uncovered == [])
        and distribution["expected"]["all_groups_covered"] is False
        and distribution["expected"]["status"] == "FAIL_LOCAL_COVERAGE",
        "aggregate capacity must not override local coverage failure",
    )

    large = cases["thirty_thousand_multi_module_aggregate_masks_local_deficit"]
    large_groups = large["groups"]
    large_population = sum(int(group["population"]) for group in large_groups)
    large_rate = q(large["per_person_demand_u_per_day"])
    large_demand = sum(
        (int(group["population"]) * large_rate for group in large_groups), Fraction(0)
    )
    large_rated = sum(
        (q(group["rated_capacity_u_per_day"]) for group in large_groups), Fraction(0)
    )
    large_delivered = sum(
        (q(group["delivered_capacity_u_per_day"]) for group in large_groups), Fraction(0)
    )
    require(large_population == large["population"] == 30000, "large-scale population conservation mismatch")
    require(
        large_demand == q(large["expected"]["total_demand_u_per_day"]) == 60000,
        "30,000-person demand sum mismatch",
    )
    require(
        large_rated == q(large["nominal_total_rated_capacity_u_per_day"])
        == q(large["expected"]["total_rated_capacity_u_per_day"]) == 62000,
        "large-scale rated-capacity sum mismatch",
    )
    require(
        large_delivered == q(large["expected"]["total_delivered_capacity_u_per_day"])
        == large_demand == 60000,
        "large-scale aggregate delivered capacity must equal total demand in this fixture",
    )
    require(
        (large_delivered >= large_demand)
        is large["expected"]["aggregate_delivered_covers_total"],
        "large-scale aggregate delivery oracle mismatch",
    )
    large_uncovered = [
        group["id"]
        for group in large_groups
        if q(group["delivered_capacity_u_per_day"])
        < int(group["population"]) * large_rate
    ]
    large_surplus = [
        group["id"]
        for group in large_groups
        if q(group["delivered_capacity_u_per_day"])
        > int(group["population"]) * large_rate
    ]
    require(
        large_uncovered == large["expected"]["uncovered_groups"] == ["habitat-central"],
        "30,000-person local shortfall mismatch",
    )
    require(
        large_surplus == large["expected"]["surplus_groups"] == ["habitat-south"],
        "30,000-person local surplus mismatch",
    )
    transfer = large["inter_group_transfer_paths"]
    require(
        any(
            edge["from"] == "habitat-south"
            and edge["to"] == "habitat-central"
            and edge["available"] is False
            and q(edge["capacity_u_per_day"]) == 0
            for edge in transfer
        ),
        "fixture must explicitly disable transfer from surplus to underserved habitat",
    )
    require(
        large["expected"]["all_groups_covered"] is False
        and large["expected"]["status"] == "FAIL_LOCAL_COVERAGE",
        "aggregate delivery equality must not conceal a local deficit",
    )

    environment = cases["unknown_atmosphere_blocks_habitability_claim"]
    require(
        environment["environment"]["atmosphere_status"] == "UNKNOWN"
        and environment["environment"]["pressure_status"] == "UNKNOWN"
        and environment["expected"]["habitability_status"] == "UNKNOWN"
        and environment["expected"]["earth_defaults_applied"] is False,
        "unknown environment must not inherit Earth defaults",
    )

    resource = cases["quarantined_resource_is_not_usable_stock"]
    require(
        q(resource["resource"]["gross_quantity_u"]) == 1000
        and resource["resource"]["quarantine_status"] == "QUARANTINED",
        "gross inventory provenance must be retained",
    )
    require(
        q(resource["resource"]["usable_quantity_for_allocation_u"]) == 0
        and resource["expected"]["quarantined_quantity_counted_as_usable"] is False,
        "unqualified resource cannot enter allocatable inventory",
    )

    unit_case = cases["mass_volume_dimension_mismatch"]
    require(
        unit_case["demand"]["dimension"] != unit_case["supply"]["dimension"]
        and unit_case["conversion_evidence"] is None
        and unit_case["expected"]["status"] == "REJECT_INCOMPATIBLE_DIMENSIONS"
        and unit_case["expected"]["silent_conversion"] is False,
        "incompatible dimensions must be rejected without conversion evidence",
    )

    dependency = cases["shared_power_bus_is_common_mode_failure"]
    failed = set(dependency["failed_dependencies"])
    unavailable = [
        component["id"]
        for component in dependency["components"]
        if failed.intersection(component["depends_on"])
    ]
    require(
        unavailable == dependency["expected"]["unavailable_components"]
        == ["pump-1", "pump-2", "pump-3"],
        "common dependency closure mismatch",
    )
    require(
        dependency["expected"]["independent_redundancy_claim"] is False
        and dependency["expected"]["status"] == "COMMON_MODE_OUTAGE",
        "shared-bus components must not be counted as independent redundancy",
    )

    transfer = cases["lost_transfer_ack_is_ambiguous"]
    require(
        transfer["transfer_acknowledgement"] == "LOST"
        and transfer["destination_receipt"] == "UNCONFIRMED"
        and transfer["expected"]["inventory_state"] == "AMBIGUOUS"
        and transfer["expected"]["transfer_counted_at_both_ends"] is False
        and transfer["expected"]["status"] == "RECONCILIATION_REQUIRED",
        "ambiguous transfer must not double-count inventory",
    )

    evidence = cases["stale_conflicting_sensor_observations"]
    observed = {item["value"] for item in evidence["observations"]}
    require(
        len(observed) > 1
        and evidence["independent_adjudication_evidence"] is None
        and evidence["expected"]["state"] == "UNKNOWN_CONFLICTED"
        and evidence["expected"]["last_write_wins"] is False
        and evidence["expected"]["status"] == "BLOCKED_CONFLICTING_EVIDENCE",
        "contradictory observations must remain unresolved without adjudication",
    )

    graph = corpus["dependency_graph_reference"]
    require(
        graph["contract_version"] == "0.1.0",
        "unexpected dependency-graph contract version",
    )
    require(
        "all listed dependencies are required" in graph["edge_semantics"]["required_dependencies"].lower()
        and "alternative group" in graph["edge_semantics"]["alternative_groups"].lower()
        and "reject" in graph["edge_semantics"]["cycle_policy"].lower(),
        "dependency-graph logic semantics must be explicit",
    )
    graph_cases = graph["cases"]
    assert isinstance(graph_cases, list)
    graph_by_id: dict[str, dict[str, object]] = {}
    for graph_case in graph_cases:
        assert isinstance(graph_case, dict)
        graph_case_id = str(graph_case["id"])
        require(
            graph_case_id not in graph_by_id,
            f"duplicate dependency graph case id: {graph_case_id}",
        )
        graph_by_id[graph_case_id] = graph_case
    require(len(graph_by_id) == 6, "dependency graph reference case count changed")

    for graph_case_id in (
        "transitive_shared_bus_outage",
        "two_nominal_pumps_share_one_failed_source",
        "independent_manual_fallback_survives",
        "reported_repair_does_not_restore_service",
        "dependency_cycle_rejected",
        "unknown_dependency_rejected",
    ):
        require(graph_case_id in graph_by_id, f"missing dependency case: {graph_case_id}")

    cascade = graph_by_id["transitive_shared_bus_outage"]
    cascade_graph_status = validate_dependency_graph(cascade)
    cascade_unavailable = unavailable_dependency_closure(cascade)
    cascade_direct_failed = sorted(
        str(node["id"])
        for node in cascade["nodes"]
        if node["state"] in ("FAILED", "REPORTED_REPAIRED_UNVERIFIED")
    )
    cascade_dependents = sorted(cascade_unavailable - set(cascade_direct_failed))
    require(cascade_graph_status == "VALID", "valid cascade graph rejected")
    require(
        cascade_direct_failed == cascade["expected"]["direct_failed"]
        == ["main-power-bus"],
        "cascade direct-fault source mismatch",
    )
    require(
        cascade_dependents == cascade["expected"]["dependent_unavailable"]
        == ["clinic-hygiene", "water-loop", "water-pump"],
        "transitive outage closure mismatch",
    )
    require(
        "clinic-hygiene" in cascade_unavailable
        and cascade["expected"]["service_supported"] is False
        and cascade["expected"]["status"] == "COMMON_MODE_CASCADE",
        "critical downstream service must reflect shared-bus outage",
    )

    shared = graph_by_id["two_nominal_pumps_share_one_failed_source"]
    shared_status = validate_dependency_graph(shared)
    shared_unavailable = unavailable_dependency_closure(shared)
    shared_alternatives = surviving_alternative_groups(shared, shared_unavailable)
    require(shared_status == "VALID", "valid shared-source graph rejected")
    require(
        shared_alternatives == shared["expected"]["surviving_alternative_groups"] == [],
        "nominal redundant pumps incorrectly survive common-source loss",
    )
    require(
        shared["expected"]["service_supported"] is False
        and shared["expected"]["independent_redundancy"] is False
        and shared["expected"]["status"] == "NO_QUALIFIED_ALTERNATIVE",
        "shared power source must invalidate independent-redundancy claim",
    )

    fallback = graph_by_id["independent_manual_fallback_survives"]
    fallback_status = validate_dependency_graph(fallback)
    fallback_unavailable = unavailable_dependency_closure(fallback)
    fallback_alternatives = surviving_alternative_groups(fallback, fallback_unavailable)
    require(fallback_status == "VALID", "valid fallback graph rejected")
    require(
        fallback_alternatives == fallback["expected"]["surviving_alternative_groups"]
        == [["manual-pump"]],
        "qualified independent manual fallback did not survive power loss",
    )
    require(
        fallback["expected"]["service_supported"] is True
        and fallback["expected"]["status"] == "VERIFIED_FALLBACK_AVAILABLE",
        "verified fallback state mismatch",
    )

    repair = graph_by_id["reported_repair_does_not_restore_service"]
    repair_status = validate_dependency_graph(repair)
    repair_unavailable = unavailable_dependency_closure(repair)
    repair_direct = sorted(
        str(node["id"])
        for node in repair["nodes"]
        if node["state"] in ("FAILED", "REPORTED_REPAIRED_UNVERIFIED")
    )
    repair_dependents = sorted(repair_unavailable - set(repair_direct))
    require(repair_status == "VALID", "valid repair-evidence graph rejected")
    require(
        repair_direct == repair["expected"]["direct_failed"] == ["controller"]
        and repair_dependents == repair["expected"]["dependent_unavailable"]
        == ["pump", "water-service"],
        "reported but unverified repair incorrectly restored service",
    )
    require(
        repair["expected"]["service_supported"] is False
        and repair["expected"]["status"] == "REQUALIFICATION_REQUIRED",
        "repair must remain unqualified until verification",
    )

    cycle = graph_by_id["dependency_cycle_rejected"]
    require(
        validate_dependency_graph(cycle) == cycle["expected"]["status"] == "REJECT_CYCLE",
        "dependency cycle must be rejected",
    )
    unknown_dep = graph_by_id["unknown_dependency_rejected"]
    require(
        validate_dependency_graph(unknown_dep)
        == unknown_dep["expected"]["status"] == "REJECT_UNKNOWN_DEPENDENCY",
        "undeclared dependency must be rejected",
    )
    require(
        unknown_dep["expected"]["dependency"] == "missing-controller",
        "unknown dependency diagnostic must name the unresolved edge",
    )

    print(f"PASS: synthetic CRASH-BOOT reference corpus; {checks} contract/arithmetic assertions.")
    print("SCOPE: fixture and oracle only; no physical safety, model credibility, or survivability claim.")


if __name__ == "__main__":
    main()
