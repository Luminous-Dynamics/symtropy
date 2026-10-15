#!/usr/bin/env python3
"""Independent, standard-library oracle for synthetic CRASH-BOOT resource-flow fixtures.

This is test tooling only. It is not a production planner or a life-support model.
"""
from __future__ import annotations

import hashlib
import json
import re
from datetime import datetime, timezone
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



def parse_timestamp(value: object) -> datetime | None:
    if not isinstance(value, str) or not value:
        return None
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None
    if parsed.tzinfo is None:
        return None
    return parsed.astimezone(timezone.utc)


def evaluate_service_evidence(
    record: dict[str, object], target: dict[str, object]
) -> str:
    lifecycle = record.get("lifecycle_state")
    if lifecycle == "REVOKED":
        return "REJECT_REVOKED"
    if lifecycle == "SUPERSEDED":
        return "REJECT_SUPERSEDED"
    if lifecycle == "REPORTED_REPAIRED_UNVERIFIED":
        return "REQUALIFICATION_REQUIRED"
    if lifecycle not in ("CURRENT", "POST_REPAIR_TEST_PASSED"):
        return "REJECT_UNKNOWN_LIFECYCLE"

    for field in ("artifact_digest", "configuration_digest"):
        field_digest = record.get(field)
        if not isinstance(field_digest, str) or not re.fullmatch(r"sha256:[0-9a-f]{64}", field_digest):
            return "REJECT_INVALID_DIGEST"

    payload_utf8 = record.get("artifact_payload_utf8")
    if payload_utf8 is not None:
        if not isinstance(payload_utf8, str):
            return "REJECT_ARTIFACT_PAYLOAD_INVALID"
        # The payload hashes to artifact_digest, never to configuration_digest.
        try:
            payload_bytes = payload_utf8.encode("utf-8")
        except UnicodeEncodeError:
            return "REJECT_ARTIFACT_PAYLOAD_INVALID"
        if "sha256:" + hashlib.sha256(payload_bytes).hexdigest() != record.get("artifact_digest"):
            return "REJECT_ARTIFACT_DIGEST_MISMATCH"
        try:
            payload_claims = json.loads(payload_utf8)
        except json.JSONDecodeError:
            return "REJECT_ARTIFACT_PAYLOAD_INVALID"
        if not isinstance(payload_claims, dict):
            return "REJECT_ARTIFACT_PAYLOAD_INVALID"
        try:
            canonical_payload = json.dumps(
                payload_claims,
                sort_keys=True,
                separators=(",", ":"),
                ensure_ascii=False,
                allow_nan=False,
            )
        except (TypeError, ValueError):
            return "REJECT_ARTIFACT_PAYLOAD_INVALID"
        if canonical_payload != payload_utf8:
            return "REJECT_ARTIFACT_PAYLOAD_NONCANONICAL"
        bound_fields = (
            "evidence_id",
            "issuer_id",
            "asset_id",
            "asset_revision",
            "configuration_digest",
            "scope_id",
            "observed_at",
            "valid_until",
            "lifecycle_state",
            "method_id",
            "method_qualification",
            "calibration_id",
            "calibration_status",
            "acceptance_criterion_id",
            "measured_value",
            "unit",
            "acceptance_min",
            "acceptance_max",
        )
        expected_claims = {field: record.get(field) for field in bound_fields}
        if not isinstance(payload_claims, dict) or payload_claims != expected_claims:
            return "REJECT_ARTIFACT_CLAIM_MISMATCH"

    required = (
        "evidence_id",
        "issuer_id",
        "asset_id",
        "asset_revision",
        "configuration_digest",
        "scope_id",
        "observed_at",
        "valid_until",
        "method_id",
        "method_qualification",
        "calibration_id",
        "calibration_status",
        "acceptance_criterion_id",
        "measured_value",
        "unit",
        "acceptance_min",
        "acceptance_max",
    )
    if any(record.get(key) is None or record.get(key) == "" for key in required):
        return "REJECT_INCOMPLETE_QUALIFICATION"

    for field in ("asset_id", "asset_revision", "configuration_digest", "scope_id"):
        if record[field] != target[field]:
            return "REJECT_SCOPE_MISMATCH"

    if lifecycle == "POST_REPAIR_TEST_PASSED":
        repair_required = (
            "repair_receipt_id",
            "repair_attempt_id",
            "repaired_asset_revision",
            "repair_configuration_digest",
            "post_repair_test_id",
            "post_repair_test_status",
            "post_repair_tested_asset_revision",
        )
        if any(record.get(key) is None or record.get(key) == "" for key in repair_required):
            return "REQUALIFICATION_REQUIRED"
        repair_digest = record["repair_configuration_digest"]
        if not isinstance(repair_digest, str) or not re.fullmatch(r"sha256:[0-9a-f]{64}", repair_digest):
            return "REJECT_INVALID_DIGEST"
        if (
            record["repaired_asset_revision"] != target["asset_revision"]
            or record["post_repair_tested_asset_revision"] != target["asset_revision"]
            or repair_digest != target["configuration_digest"]
        ):
            return "REJECT_SCOPE_MISMATCH"
        if record["post_repair_test_status"] != "PASS":
            return "REQUALIFICATION_REQUIRED"

    evaluated_at = parse_timestamp(target.get("evaluation_time"))
    observed_at = parse_timestamp(record.get("observed_at"))
    valid_until = parse_timestamp(record.get("valid_until"))
    if evaluated_at is None or observed_at is None or valid_until is None:
        return "REJECT_INVALID_TIME"
    if observed_at > evaluated_at:
        return "REJECT_FUTURE_OBSERVATION"
    if (
        (evaluated_at - observed_at).total_seconds() > float(target["max_age_seconds"])
        or valid_until <= evaluated_at
    ):
        return "NOT_AVAILABLE_STALE"

    if record["method_qualification"] != "QUALIFIED":
        return "REJECT_INCOMPLETE_QUALIFICATION"
    if record["calibration_status"] != "VALID":
        return "REJECT_CALIBRATION_EXPIRED"
    if record["unit"] != target["unit"]:
        return "REJECT_UNIT_MISMATCH"
    if record["acceptance_criterion_id"] != target["acceptance_criterion_id"]:
        return "REJECT_SCOPE_MISMATCH"

    try:
        lower = q(record["acceptance_min"])
        upper = q(record["acceptance_max"])
        measured = q(record["measured_value"])
    except (ValueError, ZeroDivisionError, TypeError):
        return "REJECT_INCOMPLETE_QUALIFICATION"
    if lower > upper:
        return "REJECT_INCOMPLETE_QUALIFICATION"
    if measured < lower or measured > upper:
        return "FAIL_ACCEPTANCE"
    return "VERIFIED_AVAILABLE_CANDIDATE"

def evaluate_service_evidence_case(
    case: dict[str, object], target: dict[str, object]
) -> str:
    records = case["records"]
    assert isinstance(records, list) and records
    results = [evaluate_service_evidence(record, target) for record in records]
    if len(records) > 1:
        measurements = {str(record.get("measured_value")) for record in records}
        if len(measurements) > 1 or len(set(results)) > 1:
            return "UNKNOWN_CONFLICTED"
    return results[0]


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
    integrity = corpus["artifact_integrity_reference"]
    require(
        integrity["contract_version"] == "0.1.0",
        "unexpected artifact-integrity reference version",
    )
    payload = integrity["payload_utf8"].encode("utf-8")
    expected_digest = str(integrity["expected_sha256"])
    recomputed_digest = hashlib.sha256(payload).hexdigest()
    require(
        recomputed_digest == expected_digest,
        "SHA-256 reference payload did not match pinned digest",
    )
    altered_payload = integrity["altered_payload_utf8"].encode("utf-8")
    require(
        hashlib.sha256(altered_payload).hexdigest() != expected_digest,
        "one-field payload mutation must change the SHA-256 digest",
    )
    configuration_digest = hashlib.sha256(
        integrity["configuration_payload_utf8"].encode("utf-8")
    ).hexdigest()
    require(
        configuration_digest == integrity["expected_configuration_sha256"],
        "configuration bytes did not match pinned SHA-256 reference",
    )
    require(
        "distinct from evidence records" in integrity["purpose"].lower(),
        "integrity canary must not imply evidence-record artifact binding",
    )
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

    evidence_contract = corpus["service_evidence_reference"]
    require(
        evidence_contract["contract_version"] == "0.1.0"
        and evidence_contract["authority"] == "NON_AUTHORITATIVE_SYNTHETIC_REFERENCE",
        "service evidence contract must retain synthetic/non-authoritative scope",
    )
    target = evidence_contract["target"]
    evidence_cases = evidence_contract["cases"]
    assert isinstance(evidence_cases, list)
    evidence_by_id: dict[str, dict[str, object]] = {}
    for evidence_case in evidence_cases:
        assert isinstance(evidence_case, dict)
        evidence_case_id = str(evidence_case["id"])
        require(
            evidence_case_id not in evidence_by_id,
            f"duplicate service-evidence case id: {evidence_case_id}",
        )
        evidence_by_id[evidence_case_id] = evidence_case
    require(len(evidence_by_id) == 21, "service-evidence case count changed")

    for evidence_case in evidence_cases:
        expected_status = str(evidence_case["expected_status"])
        observed_status = evaluate_service_evidence_case(evidence_case, target)
        require(
            observed_status == expected_status,
            f"{evidence_case['id']}: expected {expected_status}, got {observed_status}",
        )

    repair_contract = evidence_contract["repair_receipt_semantics"]
    required_repair_fields = (
        "repair_receipt_id",
        "repair_attempt_id",
        "repaired_asset_revision",
        "repair_configuration_digest",
        "post_repair_test_id",
        "post_repair_test_status",
        "post_repair_tested_asset_revision",
    )
    require(
        tuple(repair_contract["required_for_post_repair_candidate"]) == required_repair_fields,
        "repair receipt field contract changed without revision",
    )
    require(
        "exact current asset revision" in repair_contract["rule"].lower(),
        "repair receipt must bind to current asset revision",
    )
    valid_repair = evidence_by_id["qualified_post_repair_test_current_revision"]
    require(
        evaluate_service_evidence_case(valid_repair, target) == "VERIFIED_AVAILABLE_CANDIDATE",
        "current-revision passing repair receipt should satisfy the synthetic candidate rule",
    )
    missing_repair = evidence_by_id["post_repair_status_without_receipt"]
    require(
        evaluate_service_evidence_case(missing_repair, target) == "REQUALIFICATION_REQUIRED",
        "a post-repair status without receipt must remain unqualified",
    )
    wrong_revision_repair = evidence_by_id["repair_receipt_bound_to_wrong_revision"]
    require(
        evaluate_service_evidence_case(wrong_revision_repair, target) == "REJECT_SCOPE_MISMATCH",
        "repair and test receipts for an older revision must not qualify the current asset",
    )

    artifact_binding = evidence_contract["artifact_binding_rule"]
    require(
        tuple(artifact_binding["claims_bound"])
        == (
            "evidence_id",
            "issuer_id",
            "asset_id",
            "asset_revision",
            "configuration_digest",
            "scope_id",
            "observed_at",
            "valid_until",
            "lifecycle_state",
            "method_id",
            "method_qualification",
            "calibration_id",
            "calibration_status",
            "acceptance_criterion_id",
            "measured_value",
            "unit",
            "acceptance_min",
            "acceptance_max",
        ),
        "content-bound claim field set changed without contract revision",
    )
    require(
        "does not authenticate issuer identity" in artifact_binding["scope"].lower(),
        "artifact binding must preserve authentication boundary",
    )
    bound_fresh = evidence_by_id["fresh_qualified_evidence"]
    require(
        evaluate_service_evidence_case(bound_fresh, target) == "VERIFIED_AVAILABLE_CANDIDATE",
        "content-bound known-answer evidence record failed",
    )
    noncanonical_record = dict(bound_fresh["records"][0])
    noncanonical_payload_utf8 = str(noncanonical_record["artifact_payload_utf8"]).replace("{", "{ ", 1)
    noncanonical_record["artifact_payload_utf8"] = noncanonical_payload_utf8
    noncanonical_record["artifact_digest"] = (
        "sha256:" + hashlib.sha256(noncanonical_payload_utf8.encode("utf-8")).hexdigest()
    )
    noncanonical_case = {"records": [noncanonical_record]}
    require(
        evaluate_service_evidence_case(noncanonical_case, target)
        == "REJECT_ARTIFACT_PAYLOAD_NONCANONICAL",
        "canonical claim bytes must be enforced even when the altered payload is correctly rehashed",
    )

    duplicate_key_record = dict(bound_fresh["records"][0])
    duplicate_key_payload = str(duplicate_key_record["artifact_payload_utf8"]).replace(
        '"asset_id":"water-pump-7",',
        '"asset_id":"water-pump-7","asset_id":"water-pump-7",',
        1,
    )
    duplicate_key_record["artifact_payload_utf8"] = duplicate_key_payload
    duplicate_key_record["artifact_digest"] = (
        "sha256:" + hashlib.sha256(duplicate_key_payload.encode("utf-8")).hexdigest()
    )
    require(
        evaluate_service_evidence_case({"records": [duplicate_key_record]}, target)
        == "REJECT_ARTIFACT_PAYLOAD_NONCANONICAL",
        "duplicate JSON property names must be rejected even when the payload is correctly re-hashed",
    )

    invalid_unicode_record = dict(bound_fresh["records"][0])
    invalid_unicode_record["artifact_payload_utf8"] = "\ud800"
    require(
        evaluate_service_evidence_case({"records": [invalid_unicode_record]}, target)
        == "REJECT_ARTIFACT_PAYLOAD_INVALID",
        "non-UTF-8-encodable payload strings must be rejected without crashing the oracle",
    )

    nonfinite_record = dict(bound_fresh["records"][0])
    nonfinite_payload = str(nonfinite_record["artifact_payload_utf8"]).replace(
        '"measured_value":1.0,',
        '"measured_value":NaN,',
        1,
    )
    nonfinite_record["artifact_payload_utf8"] = nonfinite_payload
    nonfinite_record["artifact_digest"] = (
        "sha256:" + hashlib.sha256(nonfinite_payload.encode("utf-8")).hexdigest()
    )
    require(
        evaluate_service_evidence_case({"records": [nonfinite_record]}, target)
        == "REJECT_ARTIFACT_PAYLOAD_INVALID",
        "non-finite JSON number tokens must be rejected even when their payload digest matches",
    )

    negative_zero_record = dict(bound_fresh["records"][0])
    negative_zero_payload = str(negative_zero_record["artifact_payload_utf8"]).replace(
        '"measured_value":1.0,',
        '"measured_value":-0,',
        1,
    )
    negative_zero_record["artifact_payload_utf8"] = negative_zero_payload
    negative_zero_record["artifact_digest"] = (
        "sha256:" + hashlib.sha256(negative_zero_payload.encode("utf-8")).hexdigest()
    )
    require(
        evaluate_service_evidence_case({"records": [negative_zero_record]}, target)
        == "REJECT_ARTIFACT_PAYLOAD_NONCANONICAL",
        "negative-zero spelling must not silently normalize under the local canonical byte rule",
    )

    mutated_claim = evidence_by_id["measurement_mutated_without_rehashed_artifact"]
    require(
        evaluate_service_evidence_case(mutated_claim, target) == "REJECT_ARTIFACT_CLAIM_MISMATCH",
        "mutated claim without changing artifact bytes must be rejected",
    )
    mutated_bytes = evidence_by_id["artifact_payload_mutated_without_digest_update"]
    require(
        evaluate_service_evidence_case(mutated_bytes, target) == "REJECT_ARTIFACT_DIGEST_MISMATCH",
        "mutated artifact bytes without digest update must be rejected",
    )

    conflict_case = evidence_by_id["conflicting_current_observations"]
    conflict_status = evaluate_service_evidence_case(conflict_case, target)
    reverse_case = dict(conflict_case)
    reverse_case["records"] = list(reversed(conflict_case["records"]))
    reverse_status = evaluate_service_evidence_case(reverse_case, target)
    require(
        conflict_status == reverse_status == "UNKNOWN_CONFLICTED",
        "conflicting-source conclusion must be order-invariant and not last-write-wins",
    )
    require(
        "digest strings are syntax fixtures" in evidence_contract["digest_semantics"].lower(),
        "digest-format test must not claim artifact-byte verification",
    )
    require(
        "not physical availability proof" in evidence_contract["availability_rule"].lower(),
        "service evidence candidate must not be represented as physical truth",
    )

    print(f"PASS: synthetic CRASH-BOOT reference corpus; {checks} contract/arithmetic assertions.")
    print("SCOPE: fixture and oracle only; no physical safety, model credibility, or survivability claim.")


if __name__ == "__main__":
    main()
