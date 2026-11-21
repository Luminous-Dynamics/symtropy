#!/usr/bin/env python3
"""Finite synthetic sequential AV acquisition-plan evaluator.

v0 scores serial, non-branching acquisition plans against an exact
non-adaptive action-set oracle. Research tooling only.
"""
from __future__ import annotations

import argparse
import itertools
import json
import math
import sys
from pathlib import Path
from typing import Any

BENCHMARK_ID = "av-sequential-acquisition-v0"
EPSILON = 1e-9
MAX_ACTIONS_FOR_EXACT_ORACLE = 16


class EvaluationError(ValueError):
    """Workload, hidden truth, or candidate plan is structurally invalid."""


def read_json(path: str | Path) -> dict[str, Any]:
    with Path(path).open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise EvaluationError(f"{path}: root must be an object")
    return value


def finite_nonnegative(value: Any) -> bool:
    return (
        isinstance(value, (int, float)) and not isinstance(value, bool)
        and math.isfinite(float(value)) and float(value) >= 0.0
    )


def normalise(values: dict[str, float]) -> dict[str, float]:
    total = sum(values.values())
    if not math.isfinite(total) or total <= 0.0:
        raise EvaluationError("posterior has zero or non-finite probability mass")
    return {key: value / total for key, value in values.items()}


def entropy_bits(distribution: dict[str, float]) -> float:
    return -sum(p * math.log2(p) for p in distribution.values() if p > 0.0)


def bayes_decision(
    posterior: dict[str, float], decision_losses: dict[str, dict[str, float]]
) -> tuple[str, float]:
    risks = {
        decision: sum(posterior[state] * losses[state] for state in posterior)
        for decision, losses in decision_losses.items()
    }
    selected = min(risks, key=lambda decision: (risks[decision], decision))
    return selected, risks[selected]


def validate_episode(episode: dict[str, Any]) -> tuple[list[str], dict[str, dict[str, Any]]]:
    episode_id = episode.get("episode_id", "<unknown>")
    prior = episode.get("prior_by_state")
    losses = episode.get("decision_loss_by_state")
    actions = episode.get("actions")
    if not isinstance(prior, dict) or len(prior) < 2:
        raise EvaluationError(f"{episode_id}: prior_by_state must contain at least two states")
    if any(not isinstance(state, str) or not state for state in prior):
        raise EvaluationError(f"{episode_id}: malformed state identifier")
    if any(not finite_nonnegative(value) for value in prior.values()):
        raise EvaluationError(f"{episode_id}: prior contains invalid probability")
    if not math.isclose(sum(float(x) for x in prior.values()), 1.0, rel_tol=0.0, abs_tol=EPSILON):
        raise EvaluationError(f"{episode_id}: prior probabilities must sum to one")
    if not isinstance(losses, dict) or len(losses) < 2:
        raise EvaluationError(f"{episode_id}: at least two decisions are required")
    states = set(prior)
    for decision, by_state in losses.items():
        if not isinstance(decision, str) or not decision or not isinstance(by_state, dict):
            raise EvaluationError(f"{episode_id}: malformed decision-loss table")
        if set(by_state) != states or any(not finite_nonnegative(v) for v in by_state.values()):
            raise EvaluationError(f"{episode_id}: decision loss must be finite for every state")
    if not isinstance(actions, list) or len(actions) > MAX_ACTIONS_FOR_EXACT_ORACLE:
        raise EvaluationError(f"{episode_id}: actions must be a list with at most {MAX_ACTIONS_FOR_EXACT_ORACLE} items")
    deadline = episode.get("deadline_tick")
    if isinstance(deadline, bool) or not isinstance(deadline, int) or deadline < 1:
        raise EvaluationError(f"{episode_id}: deadline_tick must be a positive integer")
    granted = episode.get("granted_permissions")
    if not isinstance(granted, list) or any(not isinstance(x, str) or not x for x in granted):
        raise EvaluationError(f"{episode_id}: malformed granted_permissions")
    budget = episode.get("budget")
    required_budget = (
        "max_acquisition_cost_units", "max_audio_bytes", "max_video_frames",
        "max_input_tokens", "max_energy_joules",
    )
    if not isinstance(budget, dict) or any(not finite_nonnegative(budget.get(key)) for key in required_budget):
        raise EvaluationError(f"{episode_id}: malformed resource budget")
    for key in ("max_audio_bytes", "max_video_frames", "max_input_tokens"):
        if not isinstance(budget[key], int):
            raise EvaluationError(f"{episode_id}: {key} must be an integer")

    indexed: dict[str, dict[str, Any]] = {}
    for action in actions:
        if not isinstance(action, dict):
            raise EvaluationError(f"{episode_id}: action must be an object")
        action_id = action.get("action_id")
        if not isinstance(action_id, str) or not action_id or action_id in indexed:
            raise EvaluationError(f"{episode_id}: invalid or duplicate action_id {action_id!r}")
        indexed[action_id] = action
        for field in ("acquisition_cost_units", "duration_ticks", "audio_bytes", "video_frames", "input_tokens", "energy_joules"):
            if not finite_nonnegative(action.get(field)):
                raise EvaluationError(f"{episode_id}/{action_id}: invalid {field}")
        if not isinstance(action["duration_ticks"], int) or action["duration_ticks"] < 1:
            raise EvaluationError(f"{episode_id}/{action_id}: duration_ticks must be a positive integer")
        for field in ("audio_bytes", "video_frames", "input_tokens"):
            if not isinstance(action[field], int):
                raise EvaluationError(f"{episode_id}/{action_id}: {field} must be an integer")
        perms = action.get("permissions_required")
        if not isinstance(perms, list) or any(not isinstance(x, str) or not x for x in perms):
            raise EvaluationError(f"{episode_id}/{action_id}: malformed permissions_required")
        outcomes = action.get("outcomes")
        if not isinstance(outcomes, list) or len(outcomes) < 2:
            raise EvaluationError(f"{episode_id}/{action_id}: at least two outcomes are required")
        outcome_ids: set[str] = set()
        sums = {state: 0.0 for state in states}
        for outcome in outcomes:
            if not isinstance(outcome, dict):
                raise EvaluationError(f"{episode_id}/{action_id}: malformed outcome")
            outcome_id = outcome.get("outcome_id")
            likelihoods = outcome.get("likelihood_by_state")
            if not isinstance(outcome_id, str) or not outcome_id or outcome_id in outcome_ids:
                raise EvaluationError(f"{episode_id}/{action_id}: invalid or duplicate outcome_id")
            outcome_ids.add(outcome_id)
            if not isinstance(likelihoods, dict) or set(likelihoods) != states:
                raise EvaluationError(f"{episode_id}/{action_id}/{outcome_id}: likelihood states differ from prior")
            for state, probability in likelihoods.items():
                if not finite_nonnegative(probability) or probability > 1.0:
                    raise EvaluationError(f"{episode_id}/{action_id}/{outcome_id}: invalid likelihood")
                sums[state] += float(probability)
        if any(not math.isclose(value, 1.0, rel_tol=0.0, abs_tol=EPSILON) for value in sums.values()):
            raise EvaluationError(f"{episode_id}/{action_id}: outcome likelihoods must sum to one per state")
    return sorted(states), indexed


def action_outcome_index(action: dict[str, Any]) -> dict[str, dict[str, float]]:
    return {
        row["outcome_id"]: {state: float(p) for state, p in row["likelihood_by_state"].items()}
        for row in action["outcomes"]
    }


def plan_statistics(episode: dict[str, Any], action_ids: list[str]) -> dict[str, Any]:
    states, actions = validate_episode(episode)
    if len(action_ids) != len(set(action_ids)):
        raise EvaluationError(f"{episode['episode_id']}: an acquisition plan cannot repeat an action")
    if any(action_id not in actions for action_id in action_ids):
        raise EvaluationError(f"{episode['episode_id']}: acquisition plan references an unknown action")
    prior = {s: float(p) for s, p in episode["prior_by_state"].items()}
    losses = episode["decision_loss_by_state"]
    initial_decision, prior_risk = bayes_decision(prior, losses)
    selected = [actions[action_id] for action_id in action_ids]
    outcome_lists = [
        [(outcome_id, likelihoods) for outcome_id, likelihoods in action_outcome_index(action).items()]
        for action in selected
    ]
    combinations = itertools.product(*outcome_lists) if outcome_lists else [tuple()]
    expected_risk = 0.0
    expected_entropy = 0.0
    probability_mass = 0.0
    for chosen in combinations:
        weights = dict(prior)
        for _, likelihoods in chosen:
            for state in states:
                weights[state] *= likelihoods[state]
        joint_probability = sum(weights.values())
        if joint_probability <= 0.0:
            continue
        posterior = normalise(weights)
        _, risk = bayes_decision(posterior, losses)
        expected_risk += joint_probability * risk
        expected_entropy += joint_probability * entropy_bits(posterior)
        probability_mass += joint_probability
    if not math.isclose(probability_mass, 1.0, rel_tol=0.0, abs_tol=1e-8):
        raise EvaluationError(f"{episode['episode_id']}: acquisition outcome model is not normalised")
    cost = sum(float(x["acquisition_cost_units"]) for x in selected)
    return {
        "action_ids": list(action_ids),
        "prior_best_decision": initial_decision,
        "prior_expected_decision_loss": prior_risk,
        "expected_decision_loss_after_observations": expected_risk,
        "expected_information_gain_bits": max(0.0, entropy_bits(prior) - expected_entropy),
        "gross_evsi_units": prior_risk - expected_risk,
        "acquisition_cost_units": cost,
        "net_evsi_units": prior_risk - expected_risk - cost,
        "expected_objective_loss": expected_risk + cost,
        "duration_ticks": sum(int(x["duration_ticks"]) for x in selected),
        "audio_bytes": sum(int(x["audio_bytes"]) for x in selected),
        "video_frames": sum(int(x["video_frames"]) for x in selected),
        "input_tokens": sum(int(x["input_tokens"]) for x in selected),
        "energy_joules": sum(float(x["energy_joules"]) for x in selected),
    }


def plan_is_feasible(episode: dict[str, Any], action_ids: list[str]) -> bool:
    actions = {row["action_id"]: row for row in episode["actions"]}
    granted = set(episode.get("granted_permissions", []))
    selected = [actions[action_id] for action_id in action_ids]
    budget = episode["budget"]
    totals = {
        "acquisition_cost_units": sum(float(x["acquisition_cost_units"]) for x in selected),
        "audio_bytes": sum(int(x["audio_bytes"]) for x in selected),
        "video_frames": sum(int(x["video_frames"]) for x in selected),
        "input_tokens": sum(int(x["input_tokens"]) for x in selected),
        "energy_joules": sum(float(x["energy_joules"]) for x in selected),
        "duration_ticks": sum(int(x["duration_ticks"]) for x in selected),
    }
    return (
        all(set(x["permissions_required"]) <= granted for x in selected)
        and totals["acquisition_cost_units"] <= budget["max_acquisition_cost_units"] + EPSILON
        and totals["audio_bytes"] <= budget["max_audio_bytes"]
        and totals["video_frames"] <= budget["max_video_frames"]
        and totals["input_tokens"] <= budget["max_input_tokens"]
        and totals["energy_joules"] <= budget["max_energy_joules"] + EPSILON
        and totals["duration_ticks"] <= episode["deadline_tick"]
    )


def exact_oracle(episode: dict[str, Any]) -> dict[str, Any]:
    """Enumerate all feasible non-adaptive action sets; exact for finite v0."""
    _, actions = validate_episode(episode)
    ids = sorted(actions)
    best = None
    for count in range(len(ids) + 1):
        for subset in itertools.combinations(ids, count):
            selected = list(subset)
            if not plan_is_feasible(episode, selected):
                continue
            stats = plan_statistics(episode, selected)
            key = (stats["expected_objective_loss"], len(selected), tuple(selected))
            if best is None or key < best["_key"]:
                best = {**stats, "objective": stats["expected_objective_loss"], "_key": key}
    if best is None:
        raise EvaluationError(f"{episode['episode_id']}: no feasible plan including stop")
    best.pop("_key")
    return best


def posterior_after_observations(episode: dict[str, Any], selected: list[dict[str, Any]]) -> dict[str, float]:
    posterior = {s: float(p) for s, p in episode["prior_by_state"].items()}
    index = {x["action_id"]: x for x in episode["actions"]}
    for item in selected:
        likelihoods = action_outcome_index(index[item["action_id"]])[item["outcome_id"]]
        for state in posterior:
            posterior[state] *= likelihoods[state]
    return normalise(posterior)


def check_plan(episode: dict[str, Any], hidden: dict[str, Any], prediction: dict[str, Any]) -> dict[str, Any]:
    states, actions = validate_episode(episode)
    episode_id = episode["episode_id"]
    hidden_state = hidden.get("hidden_state")
    if not isinstance(hidden_state, str) or hidden_state not in states:
        raise EvaluationError(f"{episode_id}: hidden state is not in workload")
    hidden_outcomes = hidden.get("realized_outcomes")
    if not isinstance(hidden_outcomes, dict):
        raise EvaluationError(f"{episode_id}: realized_outcomes must be an object")
    if set(hidden_outcomes) != set(actions):
        raise EvaluationError(f"{episode_id}: hidden realizations must cover every available action exactly once")
    for action_id, action in actions.items():
        outcome_id = hidden_outcomes.get(action_id)
        outcome_index = action_outcome_index(action)
        if not isinstance(outcome_id, str) or outcome_id not in outcome_index:
            raise EvaluationError(f"{episode_id}/{action_id}: hidden realization is not a declared outcome")
        if outcome_index[outcome_id][hidden_state] <= 0.0:
            raise EvaluationError(f"{episode_id}/{action_id}: hidden realization has zero likelihood under hidden state")
    granted = set(episode.get("granted_permissions", []))
    budget = episode["budget"]
    trace = prediction.get("actions")
    if not isinstance(trace, list):
        raise EvaluationError(f"{episode_id}: actions must be an array")

    ids: list[str] = []
    observed: list[dict[str, Any]] = []
    resources = {
        "acquisition_cost_units": 0.0, "audio_bytes": 0, "video_frames": 0,
        "input_tokens": 0, "energy_joules": 0.0,
    }
    tick = 0
    for step in trace:
        if not isinstance(step, dict):
            raise EvaluationError(f"{episode_id}: acquisition step must be an object")
        action_id = step.get("action_id")
        if not isinstance(action_id, str) or action_id not in actions:
            raise EvaluationError(f"{episode_id}: unknown action")
        if action_id in ids:
            raise EvaluationError(f"{episode_id}: repeated acquisition action")
        action = actions[action_id]
        if not set(action["permissions_required"]) <= granted:
            raise EvaluationError(f"{episode_id}: candidate requested a denied permission action")
        request_tick, arrival_tick = step.get("request_tick"), step.get("arrival_tick")
        if isinstance(request_tick, bool) or not isinstance(request_tick, int) or request_tick != tick:
            raise EvaluationError(f"{episode_id}: acquisitions must be serial")
        expected_arrival = request_tick + action["duration_ticks"]
        if isinstance(arrival_tick, bool) or not isinstance(arrival_tick, int) or arrival_tick != expected_arrival:
            raise EvaluationError(f"{episode_id}: arrival tick disagrees with action duration")
        if arrival_tick > episode["deadline_tick"]:
            raise EvaluationError(f"{episode_id}: observation arrives after deadline")
        outcome_id = step.get("outcome_id")
        outcome_index = action_outcome_index(action)
        if not isinstance(outcome_id, str) or outcome_id not in outcome_index:
            raise EvaluationError(f"{episode_id}: unknown observation outcome")
        if hidden_outcomes.get(action_id) != outcome_id:
            raise EvaluationError(f"{episode_id}: observed outcome differs from evaluator realization")
        if outcome_index[outcome_id][hidden_state] <= 0.0:
            raise EvaluationError(f"{episode_id}: realized outcome has zero likelihood under hidden state")
        ids.append(action_id)
        observed.append({"action_id": action_id, "outcome_id": outcome_id})
        tick = arrival_tick
        resources["acquisition_cost_units"] += float(action["acquisition_cost_units"])
        resources["audio_bytes"] += int(action["audio_bytes"])
        resources["video_frames"] += int(action["video_frames"])
        resources["input_tokens"] += int(action["input_tokens"])
        resources["energy_joules"] += float(action["energy_joules"])

    checks = {
        "acquisition_cost_units": resources["acquisition_cost_units"] <= budget["max_acquisition_cost_units"] + EPSILON,
        "audio_bytes": resources["audio_bytes"] <= budget["max_audio_bytes"],
        "video_frames": resources["video_frames"] <= budget["max_video_frames"],
        "input_tokens": resources["input_tokens"] <= budget["max_input_tokens"],
        "energy_joules": resources["energy_joules"] <= budget["max_energy_joules"] + EPSILON,
    }
    if not all(checks.values()):
        raise EvaluationError(f"{episode_id}: candidate exceeded resource budget")
    response_tick = prediction.get("response_tick")
    if isinstance(response_tick, bool) or not isinstance(response_tick, int) or response_tick < tick:
        raise EvaluationError(f"{episode_id}: response precedes final admitted observation")
    if response_tick > episode["deadline_tick"]:
        raise EvaluationError(f"{episode_id}: response misses deadline")

    posterior = posterior_after_observations(episode, observed)
    best_decision, posterior_risk = bayes_decision(posterior, episode["decision_loss_by_state"])
    decision = prediction.get("decision")
    if not isinstance(decision, str) or decision not in episode["decision_loss_by_state"]:
        raise EvaluationError(f"{episode_id}: decision outside action space")
    if decision != best_decision:
        raise EvaluationError(f"{episode_id}: decision is not Bayes-optimal for admitted evidence")
    actual_loss = float(episode["decision_loss_by_state"][decision][hidden_state])
    plan = plan_statistics(episode, ids)
    return {
        "selected_action_ids": ids,
        "decision": decision,
        "posterior": posterior,
        "posterior_best_decision": best_decision,
        "posterior_expected_decision_loss": posterior_risk,
        "realized_decision_loss": actual_loss,
        "realized_total_loss": actual_loss + resources["acquisition_cost_units"],
        "response_tick": response_tick,
        "resource_usage": resources,
        "plan_statistics": plan,
    }


def evaluate(workload: dict[str, Any], truth: dict[str, Any], submission: dict[str, Any]) -> dict[str, Any]:
    for name, document in (("workload", workload), ("truth", truth), ("submission", submission)):
        if document.get("schema_version") != 1 or document.get("benchmark_id") != BENCHMARK_ID:
            raise EvaluationError(f"{name}: schema or benchmark identity mismatch")
    work_rows, truth_rows, predictions = workload.get("episodes"), truth.get("episodes"), submission.get("predictions")
    if not all(isinstance(x, list) for x in (work_rows, truth_rows, predictions)):
        raise EvaluationError("episodes/predictions must be arrays")
    def index(rows, label):
        result = {}
        for row in rows:
            if not isinstance(row, dict):
                raise EvaluationError(f"{label}: every episode row must be an object")
            key = row.get("episode_id")
            if not isinstance(key, str) or not key or key in result:
                raise EvaluationError(f"{label}: invalid or duplicate episode ID")
            result[key] = row
        return result
    work, labels, candidates = index(work_rows, "workload"), index(truth_rows, "truth"), index(predictions, "submission")
    if set(work) != set(labels) or set(work) != set(candidates):
        raise EvaluationError("workload, truth and submission IDs must match exactly")

    rows = []
    for episode_id in sorted(work):
        episode = work[episode_id]
        checked = check_plan(episode, labels[episode_id], candidates[episode_id])
        oracle = exact_oracle(episode)
        candidate_objective = checked["plan_statistics"]["expected_objective_loss"]
        regret = candidate_objective - oracle["objective"]
        if regret < -1e-7:
            raise EvaluationError(f"{episode_id}: candidate appears to beat the exact oracle")
        rows.append({
            "episode_id": episode_id,
            "scenario_family": episode["scenario_family"],
            "selected_action_ids": checked["selected_action_ids"],
            "decision": checked["decision"],
            "realized_decision_loss": checked["realized_decision_loss"],
            "realized_total_loss": checked["realized_total_loss"],
            "expected_information_gain_bits": checked["plan_statistics"]["expected_information_gain_bits"],
            "gross_evsi_units": checked["plan_statistics"]["gross_evsi_units"],
            "net_evsi_units": checked["plan_statistics"]["net_evsi_units"],
            "candidate_expected_objective_loss": candidate_objective,
            "oracle_expected_objective_loss": oracle["objective"],
            "oracle_action_ids": oracle["action_ids"],
            "regret_vs_exact_nonadaptive_oracle": max(0.0, regret),
            "oracle_action_set_matched": set(checked["selected_action_ids"]) == set(oracle["action_ids"]),
            "response_tick": checked["response_tick"],
            "resource_usage": checked["resource_usage"],
        })
    count = len(rows)
    mean = lambda key: sum(float(row[key]) for row in rows) / count if count else None
    return {
        "schema_version": 1,
        "benchmark_id": BENCHMARK_ID,
        "policy_id": submission.get("policy_id"),
        "run_id": submission.get("run_id"),
        "interpretation": {
            "status": "synthetic_finite_plan_audit",
            "limitations": [
                "v0 evaluates fixed serial, non-branching acquisition plans, not policies that choose the next sensor based on intermediate outcomes.",
                "The exact oracle enumerates all feasible non-adaptive action subsets in the finite action catalogue.",
                "Observation outcomes are conditionally independent given hidden state in this v0 contract.",
                "Logical ticks, acquisition-cost units and energy fields are synthetic, not measured wall-clock or hardware telemetry.",
                "This is a public synthetic evaluator set, not a blind holdout or model-performance result."
            ]
        },
        "totals": {
            "episode_count": count,
            "oracle_action_set_matches": sum(row["oracle_action_set_matched"] for row in rows),
            "mean_expected_information_gain_bits": mean("expected_information_gain_bits"),
            "mean_gross_evsi_units": mean("gross_evsi_units"),
            "mean_net_evsi_units": mean("net_evsi_units"),
            "mean_candidate_expected_objective_loss": mean("candidate_expected_objective_loss"),
            "mean_oracle_expected_objective_loss": mean("oracle_expected_objective_loss"),
            "mean_regret_vs_exact_nonadaptive_oracle": mean("regret_vs_exact_nonadaptive_oracle"),
            "hard_violation_count": 0
        },
        "per_episode": rows
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workload", required=True)
    parser.add_argument("--truth", required=True, help="Evaluator-only truth; never expose to candidate policy")
    parser.add_argument("--submission", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        report = evaluate(read_json(args.workload), read_json(args.truth), read_json(args.submission))
        output = Path(args.output)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    except (OSError, json.JSONDecodeError, EvaluationError) as error:
        print(f"sequential acquisition evaluation failed: {error}", file=sys.stderr)
        return 2
    print(f"wrote {output}; episodes={report['totals']['episode_count']}; regret={report['totals']['mean_regret_vs_exact_nonadaptive_oracle']:.6f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
