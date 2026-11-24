#!/usr/bin/env python3
"""Independent stdlib/Fraction oracle for observable incumbent policy adequacy.

This is a test/reference implementation, not an operational decision service.
It neither generates nor authenticates upstream evidence and grants no authority.
"""
from __future__ import annotations

import hashlib
import json
import sys
from fractions import Fraction
from pathlib import Path
from typing import Any

CORPUS_SCHEMA = "luminous.orbital-incumbent-policy-adequacy.corpus.v0.1"
CLAIM_CEILING = "synthetic-observable-policy-maintenance-algebra-only"
SOURCE_SCHEMA = "luminous.orbital-feasibility-discovery.receipt.v0.1"
SOURCE_CEILING = "finite-passive-query-policy-tree-algebra-only"
NO_QUERY = "__NO_QUERY__"
MAX_SAFE = (1 << 53) - 1


def canonical_json(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"))


def sha256_json(value: Any) -> str:
    return hashlib.sha256(canonical_json(value).encode("utf-8")).hexdigest()


def valid_id(value: Any) -> bool:
    return (
        isinstance(value, str)
        and bool(value.strip())
        and all(32 <= ord(char) <= 126 for char in value)
    )


def safe_int(value: Any) -> bool:
    return type(value) is int and 0 <= value <= MAX_SAFE


def exact_fraction(value: Any) -> Fraction | None:
    if not isinstance(value, dict) or set(value) != {"numerator", "denominator"}:
        return None
    n, d = value.get("numerator"), value.get("denominator")
    if type(n) is not int or type(d) is not int or n < 0 or d < 1 or n > 10**12 or d > 10**12:
        return None
    return Fraction(n, d)


def cost_valid(value: Any) -> bool:
    return (
        isinstance(value, dict)
        and set(value) == {"reference_trials", "exposure_micro_units", "elapsed_millis"}
        and all(safe_int(value[key]) for key in (
            "reference_trials", "exposure_micro_units", "elapsed_millis"
        ))
    )


def cost_add(left: dict[str, int], right: dict[str, int]) -> dict[str, int] | None:
    result = {key: left[key] + right[key] for key in left}
    if any(value > MAX_SAFE for value in result.values()):
        return None
    return result


def cost_within(actual: dict[str, int], budget: dict[str, int]) -> bool:
    return all(actual[key] <= budget[key] for key in actual)


def signed_difference(incumbent: Fraction, alternative: Fraction) -> dict[str, Any]:
    difference = incumbent - alternative
    if difference > 0:
        direction = "positive_improvement"
    elif difference < 0:
        direction = "negative_improvement"
    else:
        direction = "no_change"
    magnitude = abs(difference)
    return {
        "direction": direction,
        "magnitude": {
            "numerator": str(magnitude.numerator),
            "denominator": str(magnitude.denominator),
        },
    }


def joint_inventory_payload(scenarios: list[dict[str, Any]]) -> list[dict[str, str]]:
    return [
        {
            "scenario_id": item["scenario_id"],
            "world_id": item["world_id"],
            "loss_model_id": item["loss_model_id"],
        }
        for item in sorted(scenarios, key=lambda item: item["scenario_id"])
    ]


def policy_payload(corpus_case: dict[str, Any]) -> dict[str, Any]:
    """Match the Rust adapter's explicit struct/field serialization order."""
    policies = []
    for source in sorted(corpus_case["policies"], key=lambda item: item["policy_id"]):
        rows = []
        for row in sorted(source["per_scenario"], key=lambda item: item["scenario_id"]):
            rows.append({
                "scenario_id": row["scenario_id"],
                "world_id": row["world_id"],
                "loss_model_id": row["loss_model_id"],
                "observed_outcome_id": row["observed_outcome_id"],
                "selected_action_id": row["selected_action_id"],
                "loss": row["loss"],
                "wait_and_see_regret": row["wait_and_see_regret"],
                "outcome_class_regret": row["outcome_class_regret"],
            })
        policies.append({
            "policy_id": source["policy_id"],
            "query_id": source["query_id"],
            "branch_actions": dict(sorted(source["branch_actions"].items())),
            "observation_class_world_ids": {
                key: source["observation_class_world_ids"][key]
                for key in sorted(source["observation_class_world_ids"])
            },
            "worst_case_loss": source["worst_case_loss"],
            "worst_case_wait_and_see_regret": source["worst_case_wait_and_see_regret"],
            "worst_case_outcome_class_regret": source["worst_case_outcome_class_regret"],
            "worst_loss_scenario_ids": source["worst_loss_scenario_ids"],
            "worst_wait_and_see_scenario_ids": source["worst_wait_and_see_scenario_ids"],
            "worst_outcome_class_regret_scenario_ids": source["worst_outcome_class_regret_scenario_ids"],
            "worst_loss_reduction_vs_best_no_query": source["worst_loss_reduction_vs_best_no_query"],
            "query_cost": source["query_cost"],
            "branch_action_cost_upper_bound": source["branch_action_cost_upper_bound"],
            "total_cost_upper_bound": source["total_cost_upper_bound"],
            "per_scenario": rows,
        })
    return {
        "policy_pareto_frontier_ids": sorted(corpus_case["policy_pareto_frontier_ids"]),
        "policies": policies,
    }


def invalid(case_id: str, reason: str, loss_unit: str = "") -> dict[str, Any]:
    return {
        "case_id": case_id,
        "status": "INVALID_INPUT",
        "trigger_reason": reason,
        "loss_unit": loss_unit,
        "materiality_margin": {"numerator": 0, "denominator": 1},
        "incumbent_worst_case_loss": None,
        "best_eligible_reference_worst_case_loss": None,
        "worst_case_improvement": None,
        "eligible_reference_policy_count": 0,
        "resource_blocked_policy_count": 0,
        "authority_granted": False,
    }


def abstain(case_id: str, reason: str, loss_unit: str, margin: Fraction) -> dict[str, Any]:
    return {
        "case_id": case_id,
        "status": "ABSTAIN",
        "trigger_reason": reason,
        "loss_unit": loss_unit,
        "materiality_margin": {"numerator": margin.numerator, "denominator": margin.denominator},
        "incumbent_worst_case_loss": None,
        "best_eligible_reference_worst_case_loss": None,
        "worst_case_improvement": None,
        "eligible_reference_policy_count": 0,
        "resource_blocked_policy_count": 0,
        "authority_granted": False,
    }


def policy_loss_and_validate(
    case: dict[str, Any],
    policy: dict[str, Any],
    expected_scenarios: dict[str, tuple[str, str]],
) -> Fraction:
    policy_id = policy["policy_id"]
    if (
        not valid_id(policy_id)
        or (policy["query_id"] is not None and not valid_id(policy["query_id"]))
        or not isinstance(policy["branch_actions"], dict)
        or not isinstance(policy["observation_class_world_ids"], dict)
        or not policy["branch_actions"]
        or not policy["per_scenario"]
    ):
        raise ValueError(f"policy {policy_id}: malformed policy identity or branch domain")

    query_cost = policy["query_cost"]
    branch_cost = policy["branch_action_cost_upper_bound"]
    total_cost = policy["total_cost_upper_bound"]
    if not cost_valid(query_cost) or not cost_valid(branch_cost) or not cost_valid(total_cost):
        raise ValueError(f"policy {policy_id}: malformed native cost vector")
    computed_total = cost_add(query_cost, branch_cost)
    if computed_total is None or computed_total != total_cost:
        raise ValueError(f"policy {policy_id}: total cost differs from query plus worst-branch cost")

    if policy["query_id"] is None:
        if set(policy["branch_actions"]) != {NO_QUERY}:
            raise ValueError(f"policy {policy_id}: static policy must use the no-query branch")
    elif NO_QUERY in policy["branch_actions"]:
        raise ValueError(f"policy {policy_id}: query policy uses no-query sentinel")

    seen: set[str] = set()
    observed: set[str] = set()
    world_outcomes: dict[str, str] = {}
    worlds_by_outcome: dict[str, set[str]] = {}
    losses: dict[str, Fraction] = {}
    wait_regrets: dict[str, Fraction] = {}
    outcome_regrets: dict[str, Fraction] = {}

    for row in policy["per_scenario"]:
        sid = row["scenario_id"]
        if not all(valid_id(row.get(key)) for key in (
            "scenario_id", "world_id", "loss_model_id", "observed_outcome_id", "selected_action_id"
        )) or sid in seen:
            raise ValueError(f"policy {policy_id}: malformed or duplicate scenario row")
        expected = expected_scenarios.get(sid)
        if expected != (row["world_id"], row["loss_model_id"]):
            raise ValueError(f"policy {policy_id}: scenario identity is absent or mismatched")
        if policy["branch_actions"].get(row["observed_outcome_id"]) != row["selected_action_id"]:
            raise ValueError(f"policy {policy_id}: selected action differs from observable branch")
        previous = world_outcomes.setdefault(row["world_id"], row["observed_outcome_id"])
        if previous != row["observed_outcome_id"]:
            raise ValueError(f"policy {policy_id}: branches on hidden loss-model identity")
        if row["observed_outcome_id"] in expected_scenarios or row["observed_outcome_id"] in {
            world for world, _ in expected_scenarios.values()
        } or row["observed_outcome_id"] in {
            model for _, model in expected_scenarios.values()
        }:
            raise ValueError(f"policy {policy_id}: hidden identity used as observable branch")

        loss = exact_fraction(row["loss"])
        wait_regret = exact_fraction(row["wait_and_see_regret"])
        outcome_regret = exact_fraction(row["outcome_class_regret"])
        if loss is None or wait_regret is None or outcome_regret is None:
            raise ValueError(f"policy {policy_id}: malformed exact scenario metric")

        seen.add(sid)
        observed.add(row["observed_outcome_id"])
        worlds_by_outcome.setdefault(row["observed_outcome_id"], set()).add(row["world_id"])
        losses[sid] = loss
        wait_regrets[sid] = wait_regret
        outcome_regrets[sid] = outcome_regret

    if seen != set(expected_scenarios):
        raise ValueError(f"policy {policy_id}: does not cover exact joint scenario inventory")
    if set(policy["branch_actions"]) != observed:
        raise ValueError(f"policy {policy_id}: missing or extra observable branch")
    declared_classes = policy["observation_class_world_ids"]
    expected_classes = {key: sorted(value) for key, value in worlds_by_outcome.items()}
    if declared_classes != expected_classes:
        raise ValueError(f"policy {policy_id}: observation-class world IDs disagree with scenario outcomes")

    worst_loss = max(losses.values())
    worst_wait = max(wait_regrets.values())
    worst_outcome = max(outcome_regrets.values())
    if exact_fraction(policy["worst_case_loss"]) != worst_loss:
        raise ValueError(f"policy {policy_id}: declared worst-case loss disagrees with scenario rows")
    if exact_fraction(policy["worst_case_wait_and_see_regret"]) != worst_wait:
        raise ValueError(f"policy {policy_id}: declared regret metrics disagree with scenario rows")
    if exact_fraction(policy["worst_case_outcome_class_regret"]) != worst_outcome:
        raise ValueError(f"policy {policy_id}: declared regret metrics disagree with scenario rows")

    expected_worst_loss_ids = sorted(sid for sid, value in losses.items() if value == worst_loss)
    expected_wait_ids = sorted(sid for sid, value in wait_regrets.items() if value == worst_wait)
    expected_outcome_ids = sorted(sid for sid, value in outcome_regrets.items() if value == worst_outcome)
    if set(policy["worst_loss_scenario_ids"]) != set(expected_worst_loss_ids) or len(policy["worst_loss_scenario_ids"]) != len(expected_worst_loss_ids):
        raise ValueError(f"policy {policy_id}: worst-metric scenario ID sets disagree with per-scenario rows")
    if set(policy["worst_wait_and_see_scenario_ids"]) != set(expected_wait_ids) or len(policy["worst_wait_and_see_scenario_ids"]) != len(expected_wait_ids):
        raise ValueError(f"policy {policy_id}: worst-metric scenario ID sets disagree with per-scenario rows")
    if set(policy["worst_outcome_class_regret_scenario_ids"]) != set(expected_outcome_ids) or len(policy["worst_outcome_class_regret_scenario_ids"]) != len(expected_outcome_ids):
        raise ValueError(f"policy {policy_id}: worst-metric scenario ID sets disagree with per-scenario rows")
    diagnostic = policy["worst_loss_reduction_vs_best_no_query"]
    if diagnostic is not None:
        if (
            diagnostic.get("direction") not in {"positive_improvement", "negative_improvement", "no_change"}
            or exact_fraction(diagnostic.get("magnitude")) is None
        ):
            raise ValueError(f"policy {policy_id}: loss-reduction diagnostic is malformed")
    return worst_loss


def evaluate_case(case: dict[str, Any]) -> dict[str, Any]:
    case_id, loss_unit = case["case_id"], case["loss_unit"]
    margin = exact_fraction(case["materiality_margin"])
    if margin is None:
        return invalid(case_id, "case identity, scope, loss unit or cost bounds are malformed", loss_unit)

    source = case["source_receipt"]
    if (
        source.get("receipt_schema") != SOURCE_SCHEMA
        or source.get("claim_ceiling") != SOURCE_CEILING
        or not isinstance(source.get("source_commit"), str)
        or len(source["source_commit"]) != 40
        or not valid_id(source.get("source_case_id"))
        or source["source_case_id"] != case["source_policy_case_id"]
        or source.get("source_scope_id") != case["decision_scope_id"]
        or source.get("source_scope_sha256") != case["scope_sha256"]
        or source.get("source_decision_cutoff_id") != case["decision_cutoff_id"]
        or source.get("source_world_set_receipt_id") != case["world_set_receipt_id"]
        or source.get("source_joint_loss_receipt_id") != case["joint_loss_receipt_id"]
        or source.get("source_joint_scenario_manifest_sha256") != case["joint_scenario_manifest_sha256"]
        or source.get("result") != "pass"
    ):
        return invalid(case_id, "upstream feasibility-discovery receipt pins are malformed", loss_unit)

    if source.get("state") != "CURRENT":
        return abstain(case_id, "SOURCE_RECEIPT_NOT_CURRENT", loss_unit, margin)
    for key, state, good, reason in (
        ("world_set_state", case["world_set_state"], "CURRENT", "WORLD_SET_NOT_CURRENT"),
        ("loss_model_state", case["loss_model_state"], "CURRENT", "LOSS_MODEL_SET_NOT_CURRENT"),
        ("model_family_state", case["model_family_state"], "ADEQUATE", "MODEL_FAMILY_NOT_ADEQUATE"),
        ("domain_closure_state", case["domain_closure_state"], "CLOSED_WITHIN_DECLARED_SCOPE", "ACTION_DOMAIN_NOT_CLOSED"),
        ("dependency_graph_state", case["dependency_graph_state"], "GRAPH_COMPLETE_WITHIN_DECLARED_GRAMMAR", "DEPENDENCY_GRAPH_NOT_ADEQUATE"),
        ("evidence_state", case["evidence_state"], "CURRENT", "EVIDENCE_NOT_CURRENT"),
    ):
        if state != good:
            return abstain(case_id, reason, loss_unit, margin)

    scenarios = case["expected_joint_scenarios"]
    if not scenarios:
        return abstain(case_id, "NO_SURVIVING_JOINT_SCENARIOS", loss_unit, margin)
    scenario_ids = [row["scenario_id"] for row in scenarios]
    if len(set(scenario_ids)) != len(scenario_ids):
        return invalid(case_id, "planned joint scenario identities are malformed or duplicated", loss_unit)
    if sha256_json(sorted(scenario_ids)) != case["expected_scenario_ids_sha256"]:
        return invalid(case_id, "joint scenario ID inventory digest mismatch", loss_unit)
    canonical_scenarios = joint_inventory_payload(scenarios)
    if sha256_json(canonical_scenarios) != case["expected_joint_scenarios_sha256"]:
        return invalid(case_id, "joint scenario identity mapping digest mismatch", loss_unit)

    policy_ids = [policy["policy_id"] for policy in case["policies"]]
    if len(set(policy_ids)) != len(policy_ids):
        return invalid(case_id, "policy IDs are duplicated", loss_unit)
    expected_scenarios = {
        row["scenario_id"]: (row["world_id"], row["loss_model_id"]) for row in scenarios
    }

    if sha256_json(policy_payload(case)) != source["policy_payload_sha256"]:
        return invalid(case_id, "policy frontier payload digest differs from source receipt pin", loss_unit)

    policy_metrics: dict[str, tuple[dict[str, Any], Fraction]] = {}
    for policy in case["policies"]:
        try:
            loss = policy_loss_and_validate(case, policy, expected_scenarios)
        except (ValueError, KeyError, TypeError) as error:
            return invalid(case_id, str(error), loss_unit)
        policy_metrics[policy["policy_id"]] = (policy, loss)

    frontier = case["policy_pareto_frontier_ids"]
    if len(set(frontier)) != len(frontier) or any(pid not in policy_metrics for pid in frontier):
        return invalid(case_id, "policy frontier contains malformed or unknown IDs", loss_unit)

    if case["incumbent_feasibility_state"] == "INDETERMINATE":
        return abstain(case_id, "INCUMBENT_FEASIBILITY_INDETERMINATE", loss_unit, margin)
    if case["incumbent_feasibility_state"] == "INFEASIBLE":
        return {
            "case_id": case_id, "status": "POLICY_REVIEW_WARRANTED",
            "trigger_reason": "INCUMBENT_POLICY_INFEASIBLE", "loss_unit": loss_unit,
            "materiality_margin": {"numerator": margin.numerator, "denominator": margin.denominator},
            "incumbent_worst_case_loss": None, "best_eligible_reference_worst_case_loss": None,
            "worst_case_improvement": None, "eligible_reference_policy_count": 0,
            "resource_blocked_policy_count": 0, "authority_granted": False,
        }

    incumbent_item = policy_metrics.get(case["incumbent_policy_id"])
    if incumbent_item is None:
        return abstain(case_id, "INCUMBENT_POLICY_NOT_PRESENT_IN_SOURCE_FRONTIER", loss_unit, margin)
    incumbent, incumbent_loss = incumbent_item
    budget = case["resource_budget"]
    switch = case["policy_switch_cost"]
    if not cost_valid(budget) or not cost_valid(switch):
        return invalid(case_id, "case identity, scope, loss unit or cost bounds are malformed", loss_unit)
    incumbent_cost = incumbent["total_cost_upper_bound"]
    if not cost_within(incumbent_cost, budget):
        return {
            "case_id": case_id, "status": "POLICY_REVIEW_WARRANTED",
            "trigger_reason": "INCUMBENT_POLICY_EXCEEDS_RESOURCE_ENVELOPE", "loss_unit": loss_unit,
            "materiality_margin": {"numerator": margin.numerator, "denominator": margin.denominator},
            "incumbent_worst_case_loss": {"numerator": incumbent_loss.numerator, "denominator": incumbent_loss.denominator},
            "best_eligible_reference_worst_case_loss": None, "worst_case_improvement": None,
            "eligible_reference_policy_count": 0, "resource_blocked_policy_count": 0, "authority_granted": False,
        }

    eligible: list[tuple[str, Fraction]] = []
    blocked = 0
    for pid in frontier:
        if pid == case["incumbent_policy_id"]:
            continue
        policy, loss = policy_metrics[pid]
        total = cost_add(policy["total_cost_upper_bound"], switch)
        if total is None:
            return invalid(case_id, "candidate policy plus transition cost overflows", loss_unit)
        if not cost_within(total, budget):
            blocked += 1
            continue
        eligible.append((pid, loss))

    if not eligible:
        return {
            "case_id": case_id, "status": "RETAIN_INCUMBENT_POLICY",
            "trigger_reason": "ALTERNATIVES_EXCEED_RESOURCE_ENVELOPE" if blocked else "NO_ADMISSIBLE_ALTERNATIVE_POLICY",
            "loss_unit": loss_unit,
            "materiality_margin": {"numerator": margin.numerator, "denominator": margin.denominator},
            "incumbent_worst_case_loss": {"numerator": incumbent_loss.numerator, "denominator": incumbent_loss.denominator},
            "best_eligible_reference_worst_case_loss": None, "worst_case_improvement": None,
            "eligible_reference_policy_count": 0, "resource_blocked_policy_count": blocked, "authority_granted": False,
        }

    reference = min(loss for _, loss in eligible)
    improvement = signed_difference(incumbent_loss, reference)
    magnitude = Fraction(int(improvement["magnitude"]["numerator"]), int(improvement["magnitude"]["denominator"]))
    material = improvement["direction"] == "positive_improvement" and magnitude > margin
    return {
        "case_id": case_id,
        "status": "POLICY_REVIEW_WARRANTED" if material else "RETAIN_INCUMBENT_POLICY",
        "trigger_reason": "OBSERVABLE_POLICY_IMPROVEMENT_EXCEEDS_MATERIALITY_MARGIN" if material else "WITHIN_POLICY_RETENTION_ENVELOPE",
        "loss_unit": loss_unit,
        "materiality_margin": {"numerator": margin.numerator, "denominator": margin.denominator},
        "incumbent_worst_case_loss": {"numerator": incumbent_loss.numerator, "denominator": incumbent_loss.denominator},
        "best_eligible_reference_worst_case_loss": {"numerator": reference.numerator, "denominator": reference.denominator},
        "worst_case_improvement": improvement,
        "eligible_reference_policy_count": len(eligible),
        "resource_blocked_policy_count": blocked,
        "authority_granted": False,
    }


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: incumbent_policy_adequacy_oracle.py <corpus.json>", file=sys.stderr)
        return 2
    corpus = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    if corpus.get("schema") != CORPUS_SCHEMA or corpus.get("evidence_class") != "synthetic" or corpus.get("claim_ceiling") != CLAIM_CEILING:
        print("unsupported incumbent policy adequacy corpus", file=sys.stderr)
        return 2

    results = [evaluate_case(case) for case in corpus["cases"]]
    results.sort(key=lambda item: item["case_id"])
    counts = lambda state: sum(item["status"] == state for item in results)
    report = {
        "result": "pass" if all(not item["authority_granted"] for item in results) else "fail",
        "evidence_class": "synthetic",
        "claim_ceiling": CLAIM_CEILING,
        "summary": {
            "case_count": len(results),
            "retain_incumbent_policy_cases": counts("RETAIN_INCUMBENT_POLICY"),
            "policy_review_warranted_cases": counts("POLICY_REVIEW_WARRANTED"),
            "abstain_cases": counts("ABSTAIN"),
            "invalid_input_cases": counts("INVALID_INPUT"),
            "resource_blocked_cases": sum(item["resource_blocked_policy_count"] > 0 for item in results),
            "authority_grants": 0,
        },
        "cases": results,
        "authority_granted": False,
    }
    print(json.dumps(report, indent=2, sort_keys=True))
    print(
        f"python-policy-adequacy-oracle: {len(results)} case(s); Fraction exactness; "
        "stdlib only; no authority",
        file=sys.stderr,
    )
    return 0 if report["result"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
