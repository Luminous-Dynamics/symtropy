#!/usr/bin/env python3
"""Dependency-free semantic validator for the SAM analytic crossing fixture.

This is test tooling, not a vehicle runtime, sensor model, simulator, or safety
authority. It validates a finite, synthetic contract and recomputes its option
sets. It does not attempt to implement a general JSON Schema engine.
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path
from typing import Any


EXPECTED_SCHEMA_VERSION = "sam.road-option-analytic-fixture.v0.1"
EXPECTED_STATUS = "synthetic_analytic_conformance_only"
EXPECTED_ARMS = {
    "A_myopic",
    "B_option_preserving",
    "C_option_preserving_with_sensing",
}
EXPECTED_QUERY_STATUSES = {"timely", "late", "dropped", "stale", "unsupported"}
EXPECTED_NEGATIVE_CONTROLS = {
    "late-query",
    "dropped-query",
    "stale-query",
    "hidden-label-mutation",
    "irrelevant-query",
    "same-payload-different-class",
    "future-command-mutation",
    "missing-successor-cell",
    "duplicate-execution-receipt",
}


def _finite_number(value: Any) -> bool:
    return (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and math.isfinite(float(value))
    )


def validate_fixture(data: Any) -> list[str]:
    """Return semantic errors; an empty list means the declared contract is coherent."""
    errors: list[str] = []

    def require(condition: bool, message: str) -> None:
        if not condition:
            errors.append(message)

    def mapping(value: Any, label: str) -> dict[str, Any]:
        if not isinstance(value, dict):
            errors.append(f"{label} must be an object")
            return {}
        return value

    def sequence(value: Any, label: str) -> list[Any]:
        if not isinstance(value, list):
            errors.append(f"{label} must be an array")
            return []
        return value

    root = mapping(data, "fixture")
    require(root.get("schema_version") == EXPECTED_SCHEMA_VERSION, "unsupported schema_version")
    require(root.get("status") == EXPECTED_STATUS, "fixture status must remain analytic-only")
    require(bool(root.get("fixture_id")), "fixture_id must be non-empty")
    require(bool(root.get("claim_ceiling")), "claim_ceiling must be non-empty")

    context = mapping(root.get("decision_context"), "decision_context")
    first_tick = context.get("initial_decision_tick")
    next_tick = context.get("next_decision_tick")
    deadline = context.get("next_action_deadline_tick")
    tick_rate = context.get("tick_rate_hz")
    for label, value in (
        ("tick_rate_hz", tick_rate),
        ("initial_decision_tick", first_tick),
        ("next_decision_tick", next_tick),
        ("next_action_deadline_tick", deadline),
    ):
        require(isinstance(value, int) and not isinstance(value, bool), f"{label} must be an integer")
    if isinstance(tick_rate, int) and not isinstance(tick_rate, bool):
        require(tick_rate > 0, "tick_rate_hz must be positive")
    if isinstance(first_tick, int) and isinstance(next_tick, int):
        require(first_tick < next_tick, "next_decision_tick must follow initial_decision_tick")
    if isinstance(next_tick, int) and isinstance(deadline, int):
        require(next_tick <= deadline, "next decision must not occur after its action deadline")
    require(bool(context.get("initial_observation_class")), "initial observation class is required")
    require(bool(context.get("initial_observation_receipt_id")), "initial observation receipt is required")

    domains = mapping(root.get("action_domains"), "action_domains")
    first_actions = sequence(domains.get("first_actions"), "first_actions")
    continuation_actions = sequence(domains.get("continuation_actions"), "continuation_actions")
    first_ids: list[str] = []
    first_scores: dict[str, float] = {}
    for index, raw in enumerate(first_actions):
        action = mapping(raw, f"first_actions[{index}]")
        action_id = action.get("action_id")
        require(isinstance(action_id, str) and bool(action_id.strip()), f"first_actions[{index}] has empty action_id")
        if isinstance(action_id, str):
            first_ids.append(action_id)
        require(action.get("immediately_feasible") is True, f"first action {action_id!r} must be immediately feasible")
        score = action.get("myopic_immediate_score")
        require(_finite_number(score), f"first action {action_id!r} has non-finite myopic score")
        if isinstance(action_id, str) and _finite_number(score):
            first_scores[action_id] = float(score)
    continuation_ids = [
        item for item in continuation_actions if isinstance(item, str) and item.strip()
    ]
    require(len(first_ids) > 0, "first-action domain must not be empty")
    require(len(set(first_ids)) == len(first_ids), "first-action IDs must be unique")
    require(len(continuation_ids) > 0, "continuation-action domain must not be empty")
    require(len(continuation_ids) == len(continuation_actions), "continuation action IDs must be non-empty strings")
    require(len(set(continuation_ids)) == len(continuation_ids), "continuation-action IDs must be unique")

    hypotheses = sequence(root.get("hidden_hypotheses"), "hidden_hypotheses")
    hypothesis_ids: list[str] = []
    for index, raw in enumerate(hypotheses):
        hypothesis = mapping(raw, f"hidden_hypotheses[{index}]")
        hypothesis_id = hypothesis.get("hypothesis_id")
        require(isinstance(hypothesis_id, str) and bool(hypothesis_id.strip()), f"hidden_hypotheses[{index}] has empty ID")
        if isinstance(hypothesis_id, str):
            hypothesis_ids.append(hypothesis_id)
        require(
            hypothesis.get("initial_observation_class") == context.get("initial_observation_class"),
            f"hypothesis {hypothesis_id!r} does not share the declared initial observation",
        )
        require(bool(hypothesis.get("evaluator_only_description")), f"hypothesis {hypothesis_id!r} lacks evaluator-only description")
    require(len(hypothesis_ids) >= 2, "at least two hidden hypotheses are required")
    require(len(set(hypothesis_ids)) == len(hypothesis_ids), "hidden hypothesis IDs must be unique")
    hypothesis_set = set(hypothesis_ids)
    first_set = set(first_ids)
    continuation_set = set(continuation_ids)

    cases = sequence(root.get("continuation_cases"), "continuation_cases")
    case_ids: set[str] = set()
    grouped: dict[tuple[str, str], list[dict[str, Any]]] = {}
    receipt_to_class: dict[tuple[str, str], str] = {}
    action_hypothesis_coverage: set[tuple[str, str]] = set()

    for index, raw in enumerate(cases):
        case = mapping(raw, f"continuation_cases[{index}]")
        case_id = case.get("case_id")
        parent = case.get("first_action_id")
        class_id = case.get("observation_class_id")
        receipt = case.get("observation_receipt_id")
        hypothesis_id = case.get("hypothesis_id")
        delivery_tick = case.get("delivery_tick")

        require(isinstance(case_id, str) and bool(case_id.strip()), f"continuation_cases[{index}] has empty case_id")
        if isinstance(case_id, str):
            require(case_id not in case_ids, f"duplicate case_id {case_id!r}")
            case_ids.add(case_id)
        require(parent in first_set, f"case {case_id!r} references undeclared first action")
        require(hypothesis_id in hypothesis_set, f"case {case_id!r} references undeclared hypothesis")
        require(isinstance(class_id, str) and bool(class_id.strip()), f"case {case_id!r} has empty observation class")
        require(isinstance(receipt, str) and bool(receipt.strip()), f"case {case_id!r} has empty observation receipt")
        require(isinstance(delivery_tick, int) and not isinstance(delivery_tick, bool), f"case {case_id!r} has invalid delivery tick")

        outcomes = sequence(case.get("outcomes"), f"case {case_id!r}.outcomes")
        outcome_ids: list[str] = []
        for outcome_index, outcome_raw in enumerate(outcomes):
            outcome = mapping(outcome_raw, f"case {case_id!r}.outcomes[{outcome_index}]")
            action_id = outcome.get("action_id")
            disposition = outcome.get("disposition")
            witness = outcome.get("witness_id")
            utility = outcome.get("terminal_utility_units")
            require(action_id in continuation_set, f"case {case_id!r} has undeclared continuation action {action_id!r}")
            if isinstance(action_id, str):
                outcome_ids.append(action_id)
            require(disposition in {"feasible", "infeasible"}, f"case {case_id!r} has invalid disposition for {action_id!r}")
            require(isinstance(witness, str) and bool(witness.strip()), f"case {case_id!r} lacks witness for {action_id!r}")
            require(_finite_number(utility), f"case {case_id!r} has non-finite utility for {action_id!r}")
        require(len(outcome_ids) == len(continuation_ids), f"case {case_id!r} must have exactly one outcome per continuation action")
        require(len(set(outcome_ids)) == len(outcome_ids), f"case {case_id!r} contains duplicate action outcomes")
        require(set(outcome_ids) == continuation_set, f"case {case_id!r} action outcomes do not cover the frozen action domain")

        if parent in first_set and hypothesis_id in hypothesis_set and isinstance(class_id, str):
            grouped.setdefault((parent, class_id), []).append(case)
            action_hypothesis_coverage.add((parent, hypothesis_id))
        if parent in first_set and isinstance(class_id, str) and isinstance(receipt, str):
            group_key = (parent, class_id)
            for prior in grouped.get(group_key, []):
                require(
                    prior.get("observation_receipt_id") == receipt,
                    f"parent {parent!r} observation class {class_id!r} has conflicting receipts",
                )
            receipt_key = (parent, receipt)
            previous_class = receipt_to_class.get(receipt_key)
            require(
                previous_class is None or previous_class == class_id,
                f"one observation receipt for parent {parent!r} is assigned to multiple classes",
            )
            receipt_to_class[receipt_key] = class_id

    for parent in first_ids:
        for hypothesis_id in hypothesis_ids:
            require(
                (parent, hypothesis_id) in action_hypothesis_coverage,
                f"missing base successor coverage for parent {parent!r}, hypothesis {hypothesis_id!r}",
            )

    # Recompute each class's lower/upper sets and clairvoyant union from the raw cells.
    calculated: dict[tuple[str, str], dict[str, Any]] = {}
    for key, cells in grouped.items():
        robust = [
            action_id
            for action_id in continuation_ids
            if all(
                next((o.get("disposition") for o in c.get("outcomes", []) if o.get("action_id") == action_id), None)
                == "feasible"
                for c in cells
            )
        ]
        possible = [
            action_id
            for action_id in continuation_ids
            if all(
                next((o.get("disposition") for o in c.get("outcomes", []) if o.get("action_id") == action_id), None)
                != "infeasible"
                for c in cells
            )
        ]
        union = [
            action_id
            for action_id in continuation_ids
            if any(
                next((o.get("disposition") for o in c.get("outcomes", []) if o.get("action_id") == action_id), None)
                == "feasible"
                for c in cells
            )
        ]
        calculated[key] = {
            "robust": robust,
            "possible": possible,
            "union": union,
            "gap": len(union) - len(robust),
            "cell_count": len(cells),
        }

    findings = sequence(root.get("expected_findings"), "expected_findings")
    finding_keys: set[tuple[str, str]] = set()
    for index, raw in enumerate(findings):
        finding = mapping(raw, f"expected_findings[{index}]")
        key = (finding.get("parent_action_id"), finding.get("observation_class_id"))
        require(key in calculated, f"expected finding {key!r} has no matching successor class")
        require(key not in finding_keys, f"duplicate expected finding for {key!r}")
        finding_keys.add(key)
        actual = calculated.get(key)
        if actual is None:
            continue
        require(finding.get("expected_robust_option_set") == actual["robust"], f"robust option-set mismatch for {key!r}")
        require(finding.get("expected_possible_option_set") == actual["possible"], f"possible option-set mismatch for {key!r}")
        require(finding.get("expected_clairvoyant_union") == actual["union"], f"clairvoyant union mismatch for {key!r}")
        require(finding.get("expected_clairvoyance_gap") == actual["gap"], f"clairvoyance-gap mismatch for {key!r}")
    require(finding_keys == set(calculated), "expected_findings must cover every computed observation class exactly once")

    measurement = mapping(root.get("measurement"), "measurement")
    require(bool(measurement.get("query_id")), "measurement query_id is required")
    request_tick = measurement.get("request_tick")
    latency = measurement.get("expected_latency_ticks")
    timely_tick = measurement.get("timely_delivery_tick")
    late_tick = measurement.get("late_delivery_tick")
    cost = measurement.get("resource_cost_units")
    for label, value in (("request_tick", request_tick), ("expected_latency_ticks", latency), ("timely_delivery_tick", timely_tick), ("late_delivery_tick", late_tick)):
        require(isinstance(value, int) and not isinstance(value, bool), f"measurement {label} must be an integer")
    require(_finite_number(cost) and float(cost) > 0, "measurement cost must be finite and positive")
    if all(isinstance(x, int) and not isinstance(x, bool) for x in (request_tick, latency, timely_tick)):
        require(request_tick + latency == timely_tick, "nominal query latency does not match request/delivery ticks")
    if isinstance(timely_tick, int) and isinstance(deadline, int):
        require(timely_tick <= deadline, "timely measurement arrives after the action deadline")
    if isinstance(late_tick, int) and isinstance(deadline, int):
        require(late_tick > deadline, "late measurement must arrive after the action deadline")
    require(measurement.get("request_after_first_action") == "early_yield", "measurement must be requested after the option-preserving first action")

    query_outcomes = sequence(measurement.get("outcomes"), "measurement.outcomes")
    status_counts: dict[str, int] = {}
    per_hypothesis: dict[str, list[str]] = {}
    for index, raw in enumerate(query_outcomes):
        outcome = mapping(raw, f"measurement.outcomes[{index}]")
        hypothesis_id = outcome.get("hypothesis_id")
        status = outcome.get("delivery_status")
        observation_class = outcome.get("delivered_observation_class")
        receipt = outcome.get("delivered_observation_receipt_id")
        delivery_tick = outcome.get("delivery_tick")
        require(hypothesis_id in hypothesis_set, f"measurement outcome {index} references undeclared hypothesis")
        require(status in EXPECTED_QUERY_STATUSES, f"measurement outcome {index} has unsupported delivery status")
        if isinstance(status, str):
            status_counts[status] = status_counts.get(status, 0) + 1
            per_hypothesis.setdefault(hypothesis_id, []).append(status)
        if status == "timely":
            require(isinstance(observation_class, str) and bool(observation_class), f"timely measurement {index} lacks observation class")
            require(isinstance(receipt, str) and bool(receipt), f"timely measurement {index} lacks receipt")
            require(delivery_tick == timely_tick, f"timely measurement {index} has wrong delivery tick")
            matches = [
                c for c in cases
                if c.get("first_action_id") == "early_yield"
                and c.get("hypothesis_id") == hypothesis_id
                and c.get("observation_class_id") == observation_class
                and c.get("observation_receipt_id") == receipt
            ]
            require(len(matches) == 1, f"timely measurement {index} must map to exactly one matching successor cell")
        else:
            require(observation_class is None and receipt is None, f"non-timely measurement {index} must not create an admitted observation class")
            if status == "late":
                require(delivery_tick == late_tick, f"late measurement {index} has wrong delivery tick")
            elif status == "stale":
                require(delivery_tick is not None, f"stale measurement {index} must retain its delivery tick")
            else:
                require(delivery_tick is None, f"{status} measurement {index} cannot claim a delivery tick")
    require(status_counts == {status: len(hypothesis_ids) for status in EXPECTED_QUERY_STATUSES}, "each hypothesis must have exactly one outcome for every query-delivery status")
    for hypothesis_id in hypothesis_ids:
        require(
            set(per_hypothesis.get(hypothesis_id, [])) == EXPECTED_QUERY_STATUSES,
            f"hypothesis {hypothesis_id!r} lacks one or more query-delivery controls",
        )

    arms = sequence(root.get("arms"), "arms")
    arm_ids: set[str] = set()
    arm_by_id: dict[str, dict[str, Any]] = {}
    for index, raw in enumerate(arms):
        arm = mapping(raw, f"arms[{index}]")
        arm_id = arm.get("arm_id")
        require(arm_id in EXPECTED_ARMS, f"unknown controller arm {arm_id!r}")
        if isinstance(arm_id, str):
            require(arm_id not in arm_ids, f"duplicate controller arm {arm_id!r}")
            arm_ids.add(arm_id)
            arm_by_id[arm_id] = arm
        require(arm.get("expected_first_action") in first_set, f"arm {arm_id!r} selects undeclared first action")
        require(bool(arm.get("policy")), f"arm {arm_id!r} lacks policy definition")
        require(isinstance(arm.get("query_enabled"), bool), f"arm {arm_id!r} query_enabled must be boolean")
        require(isinstance(arm.get("measurement_cost_charged"), bool), f"arm {arm_id!r} must declare measurement-cost accounting")
    require(arm_ids == EXPECTED_ARMS, "fixture must define all three controller arms exactly once")
    if "A_myopic" in arm_by_id:
        myopic = arm_by_id["A_myopic"]
        best = max(first_scores, key=first_scores.get) if first_scores else None
        require(myopic.get("expected_first_action") == best, "myopic arm must select the highest immediate-score action")
        require(myopic.get("query_enabled") is False, "myopic arm must not request the extra measurement")
        require(myopic.get("measurement_cost_charged") is False, "myopic arm must not be charged for an unrequested measurement")
    if "B_option_preserving" in arm_by_id:
        option_arm = arm_by_id["B_option_preserving"]
        require(option_arm.get("query_enabled") is False, "option-preserving arm must not enable sensing")
        require(option_arm.get("measurement_cost_charged") is False, "option-preserving arm must not be charged for sensing")
    if "C_option_preserving_with_sensing" in arm_by_id:
        sensing_arm = arm_by_id["C_option_preserving_with_sensing"]
        require(sensing_arm.get("query_enabled") is True, "sensing arm must enable the query")
        require(sensing_arm.get("measurement_cost_charged") is True, "sensing arm must charge query cost")

    controls = sequence(root.get("negative_controls"), "negative_controls")
    control_ids = {
        c.get("control_id")
        for index, raw in enumerate(controls)
        for c in [mapping(raw, f"negative_controls[{index}]")]
        if isinstance(c.get("control_id"), str)
    }
    require(
        EXPECTED_NEGATIVE_CONTROLS.issubset(control_ids),
        f"missing required negative controls: {sorted(EXPECTED_NEGATIVE_CONTROLS - control_ids)}",
    )
    for index, raw in enumerate(controls):
        control = mapping(raw, f"negative_controls[{index}]")
        require(bool(control.get("intervention")), f"negative control {index} lacks intervention")
        require(bool(control.get("expected_invariant")), f"negative control {index} lacks expected invariant")

    return errors


def _load_json(path: Path) -> Any:
    def reject_constant(value: str) -> None:
        raise ValueError(f"non-standard JSON numeric constant: {value}")
    with path.open("r", encoding="utf-8") as stream:
        return json.load(stream, parse_constant=reject_constant)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--fixture",
        type=Path,
        default=Path("docs/research/road_option_experiment_v0_1/analytic_three_arm_fixture.json"),
    )
    parser.add_argument(
        "--schema",
        type=Path,
        default=Path("docs/research/road_option_experiment_v0_1/analytic_three_arm_fixture.schema.json"),
    )
    args = parser.parse_args(argv)

    try:
        schema = _load_json(args.schema)
        fixture = _load_json(args.fixture)
    except (OSError, json.JSONDecodeError, ValueError) as exc:
        print(f"FAIL: unable to load fixture/schema: {exc}", file=sys.stderr)
        return 2

    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        print("FAIL: schema must declare JSON Schema Draft 2020-12", file=sys.stderr)
        return 2

    errors = validate_fixture(fixture)
    if errors:
        for error in errors:
            print(f"FAIL: {error}", file=sys.stderr)
        return 1

    classes = len({
        (case["first_action_id"], case["observation_class_id"])
        for case in fixture["continuation_cases"]
    })
    print(
        "PASS: analytic fixture semantic contract "
        f"({len(fixture['continuation_cases'])} successor cells, "
        f"{classes} observation classes, "
        f"{len(fixture['measurement']['outcomes'])} query outcomes, "
        f"{len(fixture['arms'])} controller arms)"
    )
    print("Claim ceiling: synthetic analytic conformance only.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
