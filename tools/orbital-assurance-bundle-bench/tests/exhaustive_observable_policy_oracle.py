#!/usr/bin/env python3
"""Exhaustive exact oracle for tiny observable policy domains.

Enumerates every non-anticipative action map induced by a declared observation
partition. It is intentionally bounded to tiny finite domains and is independent
of the production adapter's frontier input. It is a test oracle, not a planner.
"""
from __future__ import annotations

import itertools
import json
import sys
from fractions import Fraction
from pathlib import Path
from typing import Any

SCHEMA = "luminous.orbital-observable-policy-exhaustive-domain.v0.1"
NO_QUERY = "__NO_QUERY__"
RESOURCE_KEYS = ("reference_trials", "exposure_micro_units", "elapsed_millis")
MAX_SAFE = (1 << 53) - 1


def frac(value: Any) -> Fraction:
    if not isinstance(value, dict) or set(value) != {"numerator", "denominator"}:
        raise ValueError("rational must have exactly numerator and denominator")
    n, d = value["numerator"], value["denominator"]
    if type(n) is not int or type(d) is not int or n < 0 or d < 1:
        raise ValueError("rational components must be non-negative/positive integers")
    return Fraction(n, d)


def validate_cost(value: Any) -> dict[str, int]:
    if not isinstance(value, dict) or set(value) != set(RESOURCE_KEYS):
        raise ValueError("resource vector has unexpected dimensions")
    result = {}
    for key in RESOURCE_KEYS:
        item = value[key]
        if type(item) is not int or not 0 <= item <= MAX_SAFE:
            raise ValueError(f"resource dimension {key} is not a bounded integer")
        result[key] = item
    return result


def add_cost(*values: dict[str, int]) -> dict[str, int]:
    result = {key: sum(value[key] for value in values) for key in RESOURCE_KEYS}
    if any(value > MAX_SAFE for value in result.values()):
        raise ValueError("resource vector addition overflow")
    return result


def max_cost(values: list[dict[str, int]]) -> dict[str, int]:
    if not values:
        return {key: 0 for key in RESOURCE_KEYS}
    return {key: max(value[key] for value in values) for key in RESOURCE_KEYS}


def within(actual: dict[str, int], budget: dict[str, int]) -> bool:
    return all(actual[key] <= budget[key] for key in RESOURCE_KEYS)


def dominates(left: dict[str, Any], right: dict[str, Any]) -> bool:
    dimensions_left = [left["worst_case_loss"]] + [left["resource_cost"][key] for key in RESOURCE_KEYS]
    dimensions_right = [right["worst_case_loss"]] + [right["resource_cost"][key] for key in RESOURCE_KEYS]
    no_worse = all(a <= b for a, b in zip(dimensions_left, dimensions_right))
    strictly_better = any(a < b for a, b in zip(dimensions_left, dimensions_right))
    return no_worse and strictly_better


def enumerate_policies(domain: dict[str, Any]) -> list[dict[str, Any]]:
    scenarios = domain["scenarios"]
    worlds = sorted({row["world_id"] for row in scenarios})
    if not scenarios or not worlds:
        raise ValueError("scenario/world domain must be nonempty")
    scenario_ids = [row["scenario_id"] for row in scenarios]
    if len(set(scenario_ids)) != len(scenario_ids):
        raise ValueError("scenario IDs must be unique")
    if any(row["world_id"] not in worlds for row in scenarios):
        raise ValueError("scenario references unknown world")

    action_costs = {name: validate_cost(cost) for name, cost in domain["action_costs"].items()}
    scenario_actions: dict[str, set[str]] = {}
    scenario_losses: dict[tuple[str, str], Fraction] = {}
    for row in scenarios:
        feasible = set(row["action_losses"].keys())
        if not feasible:
            raise ValueError(f"scenario {row['scenario_id']} has no feasible action")
        for action_id, loss in row["action_losses"].items():
            if action_id not in action_costs:
                raise ValueError(f"unknown action cost for {action_id}")
            scenario_losses[(row["scenario_id"], action_id)] = frac(loss)
        scenario_actions[row["scenario_id"]] = feasible

    policies: list[dict[str, Any]] = []

    def emit(query_id: str | None, query_cost: dict[str, int], branch_actions: dict[str, str],
             class_scenarios: dict[str, list[dict[str, Any]]]) -> None:
        losses: dict[str, Fraction] = {}
        per_scenario = []
        selected_costs = []
        for outcome in sorted(class_scenarios):
            action_id = branch_actions[outcome]
            selected_costs.append(action_costs[action_id])
            for row in class_scenarios[outcome]:
                loss = scenario_losses.get((row["scenario_id"], action_id))
                if loss is None:
                    raise ValueError(
                        f"action {action_id} is not feasible for scenario {row['scenario_id']}"
                    )
                losses[row["scenario_id"]] = loss
                per_scenario.append({
                    "scenario_id": row["scenario_id"],
                    "world_id": row["world_id"],
                    "loss_model_id": row["loss_model_id"],
                    "observed_outcome_id": outcome,
                    "selected_action_id": action_id,
                    "loss": loss,
                })
        worst_loss = max(losses.values())
        branch_cost = max_cost(selected_costs)
        resource_cost = add_cost(query_cost, branch_cost)
        if query_id is None:
            policy_id = f"static::{branch_actions[NO_QUERY]}"
        else:
            mapping = "::".join(
                f"{outcome}={branch_actions[outcome]}" for outcome in sorted(branch_actions)
            )
            policy_id = f"query::{query_id}::{mapping}"
        policies.append({
            "policy_id": policy_id,
            "query_id": query_id,
            "branch_actions": dict(branch_actions),
            "per_scenario": sorted(per_scenario, key=lambda row: row["scenario_id"]),
            "worst_case_loss": worst_loss,
            "query_cost": dict(query_cost),
            "branch_action_cost_upper_bound": branch_cost,
            "resource_cost": resource_cost,
        })

    # Static policies are one common action across every joint scenario.
    static_class = {NO_QUERY: scenarios}
    common_static = set.intersection(*(scenario_actions[row["scenario_id"]] for row in scenarios))
    for action_id in sorted(common_static):
        emit(None, {key: 0 for key in RESOURCE_KEYS}, {NO_QUERY: action_id}, static_class)

    cutoff = domain["decision_cutoff_sequence"]
    if type(cutoff) is not int or cutoff < 0:
        raise ValueError("decision cutoff sequence is invalid")
    expected_worlds = set(worlds)
    for query in domain["queries"]:
        if query.get("evidence_state") != "CURRENT" or query.get("safety_state") != "QUALIFIED":
            raise ValueError(f"query {query['query_id']} is not source-current and safety-qualified")
        if type(query.get("observation_sequence")) is not int or query["observation_sequence"] > cutoff:
            raise ValueError(f"query {query['query_id']} contains a post-cutoff observation")
        outcome_by_world = query["outcome_by_world"]
        if set(outcome_by_world) != expected_worlds:
            raise ValueError(f"query {query['query_id']} does not map every physical world exactly")
        if any(not isinstance(outcome, str) or not outcome.strip() for outcome in outcome_by_world.values()):
            raise ValueError(f"query {query['query_id']} contains malformed outcome IDs")
        groups: dict[str, list[dict[str, Any]]] = {}
        for row in scenarios:
            outcome = outcome_by_world[row["world_id"]]
            groups.setdefault(outcome, []).append(row)

        outcomes = sorted(groups)
        feasible_by_outcome = []
        for outcome in outcomes:
            rows = groups[outcome]
            feasible = set.intersection(*(scenario_actions[row["scenario_id"]] for row in rows))
            if not feasible:
                feasible_by_outcome = []
                break
            feasible_by_outcome.append(sorted(feasible))
        if not feasible_by_outcome:
            continue

        query_cost = validate_cost(query["query_cost"])
        for actions in itertools.product(*feasible_by_outcome):
            branch_actions = dict(zip(outcomes, actions))
            emit(query["query_id"], query_cost, branch_actions, groups)

    policies.sort(key=lambda policy: policy["policy_id"])
    return policies


def fraction_json(value: Fraction) -> dict[str, int]:
    return {"numerator": value.numerator, "denominator": value.denominator}


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: exhaustive_observable_policy_oracle.py <domain.json>", file=sys.stderr)
        return 2
    domain = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    if domain.get("schema") != SCHEMA or domain.get("evidence_class") != "synthetic":
        print("unsupported exhaustive policy domain", file=sys.stderr)
        return 2
    policies = enumerate_policies(domain)
    frontier = []
    for candidate in policies:
        dominated = any(
            other["policy_id"] != candidate["policy_id"] and dominates(other, candidate)
            for other in policies
        )
        if not dominated:
            frontier.append(candidate)
    frontier.sort(key=lambda policy: policy["policy_id"])
    frontier_ids = {policy["policy_id"] for policy in frontier}

    incumbent_id = domain["incumbent_policy_id"]
    by_id = {policy["policy_id"]: policy for policy in policies}
    incumbent = by_id.get(incumbent_id)
    if incumbent is None:
        raise ValueError("declared incumbent policy was not exhaustively enumerated")
    budget = validate_cost(domain["resource_budget"])
    transition = validate_cost(domain["policy_switch_cost"])
    eligible = []
    blocked = 0
    for policy in frontier:
        if policy["policy_id"] == incumbent_id:
            continue
        total = add_cost(policy["resource_cost"], transition)
        if within(total, budget):
            eligible.append((policy, total))
        else:
            blocked += 1
    best = min(eligible, key=lambda item: (item[0]["worst_case_loss"], item[0]["policy_id"])) if eligible else None
    margin = frac(domain["materiality_margin"])
    if best is None:
        review = False
        improvement = Fraction(0, 1)
        trigger = "NO_ELIGIBLE_ALTERNATIVE"
    else:
        improvement = incumbent["worst_case_loss"] - best[0]["worst_case_loss"]
        review = improvement > margin
        trigger = "OBSERVABLE_POLICY_IMPROVEMENT_EXCEEDS_MATERIALITY_MARGIN" if review else "WITHIN_POLICY_RETENTION_ENVELOPE"

    report = {
        "schema": "luminous.orbital-observable-policy-exhaustive-report.v0.1",
        "evidence_class": "synthetic",
        "claim_ceiling": "tiny-finite-domain-exhaustive-observable-policy-oracle-only",
        "policy_count": len(policies),
        "frontier_policy_ids": sorted(frontier_ids),
        "dominated_policy_count": len(policies) - len(frontier),
        "incumbent_policy_id": incumbent_id,
        "incumbent_worst_case_loss": fraction_json(incumbent["worst_case_loss"]),
        "eligible_reference_policy_count": len(eligible),
        "resource_blocked_frontier_policy_count": blocked,
        "best_eligible_reference_policy_id": best[0]["policy_id"] if best else None,
        "best_eligible_reference_worst_case_loss": fraction_json(best[0]["worst_case_loss"]) if best else None,
        "worst_case_improvement": fraction_json(improvement),
        "materiality_margin": fraction_json(margin),
        "review_warranted": review,
        "trigger": trigger,
        "authority_granted": False,
        "policies": [
            {
                "policy_id": policy["policy_id"],
                "query_id": policy["query_id"],
                "branch_actions": policy["branch_actions"],
                "worst_case_loss": fraction_json(policy["worst_case_loss"]),
                "resource_cost": policy["resource_cost"],
                "on_pareto_frontier": policy["policy_id"] in frontier_ids,
            }
            for policy in policies
        ],
    }
    print(json.dumps(report, indent=2, sort_keys=True))
    print(
        f"exhaustive-observable-policy-oracle: {len(policies)} policies; "
        f"{len(frontier)} Pareto policies; review={str(review).lower()}; no authority",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
