#!/usr/bin/env python3
"""Model-independent all-action EVSI forecast and decision-regret evaluator."""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path
from typing import Any

BENCHMARK_ID = "av-action-space-evsi-v0"
STOP_ACTION_ID = "__stop__"
FORECAST_INTERVAL_ALPHA = 0.1  # Preregistered central 90% interval score.


class EvaluationError(ValueError):
    """Malformed or causally invalid benchmark input."""


def read_json(path: str | Path) -> dict[str, Any]:
    with Path(path).open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise EvaluationError(f"{path}: expected a JSON object")
    return value


def finite_number(value: Any) -> bool:
    return (
        not isinstance(value, bool)
        and isinstance(value, (int, float))
        and math.isfinite(float(value))
    )


def unique_rows(rows: Any, key: str, source: str) -> dict[str, dict[str, Any]]:
    if not isinstance(rows, list):
        raise EvaluationError(f"{source}: expected an array")
    indexed: dict[str, dict[str, Any]] = {}
    for row in rows:
        if not isinstance(row, dict):
            raise EvaluationError(f"{source}: rows must be objects")
        identifier = row.get(key)
        if not isinstance(identifier, str) or not identifier or identifier in indexed:
            raise EvaluationError(f"{source}: missing or duplicate {key}: {identifier!r}")
        indexed[identifier] = row
    return indexed


def evaluate(
    workload: dict[str, Any],
    truth: dict[str, Any],
    submission: dict[str, Any],
) -> dict[str, Any]:
    for name, document in (("workload", workload), ("truth", truth), ("submission", submission)):
        if document.get("schema_version") != 1 or document.get("benchmark_id") != BENCHMARK_ID:
            raise EvaluationError(f"{name}: benchmark identity/schema mismatch")

    work = unique_rows(workload.get("episodes"), "episode_id", "workload episodes")
    labels = unique_rows(truth.get("episodes"), "episode_id", "truth episodes")
    predictions = unique_rows(submission.get("predictions"), "episode_id", "submission predictions")
    if set(work) != set(labels) or set(work) != set(predictions):
        raise EvaluationError("workload, truth and submission episode IDs must match exactly")

    per_episode: list[dict[str, Any]] = []
    gross_errors: list[float] = []
    net_errors: list[float] = []
    interval_hits = 0
    interval_count = 0
    interval_widths: list[float] = []
    interval_scores: list[float] = []
    rank_correct = 0
    rank_pairs = 0
    missed_best = 0
    total_regret = 0.0
    selected = 0

    for episode_id in sorted(work):
        episode = work[episode_id]
        label = labels[episode_id]
        prediction = predictions[episode_id]

        current_tick = episode.get("current_tick")
        deadline_tick = episode.get("deadline_tick")
        cost_weight = episode.get("cost_weight")
        budget = episode.get("budget_cost_units")
        if any(isinstance(v, bool) or not isinstance(v, int) or v < 0 for v in (current_tick, deadline_tick)):
            raise EvaluationError(f"{episode_id}: invalid current/deadline tick")
        if deadline_tick < current_tick:
            raise EvaluationError(f"{episode_id}: deadline precedes current time")
        if not finite_number(cost_weight) or cost_weight < 0:
            raise EvaluationError(f"{episode_id}: invalid cost weight")
        if not finite_number(budget) or budget < 0:
            raise EvaluationError(f"{episode_id}: invalid budget")

        if prediction.get("transcript_head_sha256") != episode.get("transcript_head_sha256"):
            raise EvaluationError(f"{episode_id}: stale or mismatched transcript head")
        forecast_tick = prediction.get("forecast_tick")
        if isinstance(forecast_tick, bool) or not isinstance(forecast_tick, int) or forecast_tick != current_tick:
            raise EvaluationError(f"{episode_id}: forecast must be committed at the current transcript tick")
        if forecast_tick > deadline_tick:
            raise EvaluationError(f"{episode_id}: forecast is after deadline")

        catalogue = unique_rows(episode.get("action_catalogue"), "action_id", f"{episode_id} action catalogue")
        if STOP_ACTION_ID in catalogue:
            raise EvaluationError(f"{episode_id}: reserved stop ID appears in action catalogue")
        potential = label.get("potential_outcomes")
        if not isinstance(potential, dict) or set(potential) != set(catalogue):
            raise EvaluationError(f"{episode_id}: evaluator potential outcomes must cover every catalogue action")

        forecast_rows = unique_rows(prediction.get("forecasts"), "action_id", f"{episode_id} forecasts")
        expected_ids = set(catalogue) | {STOP_ACTION_ID}
        if set(forecast_rows) != expected_ids:
            missing = sorted(expected_ids - set(forecast_rows))
            extra = sorted(set(forecast_rows) - expected_ids)
            raise EvaluationError(f"{episode_id}: forecasts must cover every action plus stop; missing={missing}, extra={extra}")

        estimated_net: dict[str, float] = {STOP_ACTION_ID: 0.0}
        true_net: dict[str, float] = {STOP_ACTION_ID: 0.0}
        feasible: dict[str, bool] = {STOP_ACTION_ID: True}
        for action_id, action in catalogue.items():
            cost = action.get("cost_units")
            arrival = action.get("arrival_tick")
            if not finite_number(cost) or cost < 0:
                raise EvaluationError(f"{episode_id}/{action_id}: invalid cost")
            if isinstance(arrival, bool) or not isinstance(arrival, int) or arrival < current_tick:
                raise EvaluationError(f"{episode_id}/{action_id}: invalid arrival tick")
            is_feasible = arrival <= deadline_tick and cost <= budget
            feasible[action_id] = is_feasible

            outcome = potential[action_id]
            if not isinstance(outcome, dict) or not finite_number(outcome.get("expected_gross_evsi")):
                raise EvaluationError(f"{episode_id}/{action_id}: invalid potential outcome")
            actual_gross = float(outcome["expected_gross_evsi"])
            if actual_gross < 0:
                raise EvaluationError(f"{episode_id}/{action_id}: gross EVSI cannot be negative")
            true_lower = outcome.get("lower_gross_evsi")
            true_upper = outcome.get("upper_gross_evsi")
            if (
                not finite_number(true_lower)
                or not finite_number(true_upper)
                or float(true_lower) > actual_gross
                or actual_gross > float(true_upper)
                or float(true_lower) > float(true_upper)
            ):
                raise EvaluationError(f"{episode_id}/{action_id}: invalid evaluator EVSI interval")
            predicted = forecast_rows[action_id]
            estimate = predicted.get("estimated_gross_evsi")
            lower = predicted.get("lower_gross_evsi")
            upper = predicted.get("upper_gross_evsi")
            if not all(finite_number(v) for v in (estimate, lower, upper)):
                raise EvaluationError(f"{episode_id}/{action_id}: non-finite forecast")
            if float(lower) > float(estimate) or float(estimate) > float(upper):
                raise EvaluationError(f"{episode_id}/{action_id}: forecast interval does not contain its point estimate")

            estimated_action_net = float(estimate) - float(cost_weight) * float(cost)
            actual_action_net = actual_gross - float(cost_weight) * float(cost)
            estimated_net[action_id] = estimated_action_net
            true_net[action_id] = actual_action_net
            gross_errors.append(float(estimate) - actual_gross)
            net_errors.append(estimated_action_net - actual_action_net)

            # Score the declared central 90% interval against the evaluator-known
            # expected gross-EVSI label. Width penalizes vacuous, over-wide intervals;
            # misses receive the standard interval-score penalty.
            interval_width = float(upper) - float(lower)
            interval_score = interval_width
            if actual_gross < float(lower):
                interval_score += (2.0 / FORECAST_INTERVAL_ALPHA) * (float(lower) - actual_gross)
            elif actual_gross > float(upper):
                interval_score += (2.0 / FORECAST_INTERVAL_ALPHA) * (actual_gross - float(upper))
            interval_widths.append(interval_width)
            interval_scores.append(interval_score)
            if float(lower) <= actual_gross <= float(upper):
                interval_hits += 1
            interval_count += 1

        stop_row = forecast_rows[STOP_ACTION_ID]
        if any(not finite_number(stop_row.get(k)) for k in ("estimated_gross_evsi", "lower_gross_evsi", "upper_gross_evsi")):
            raise EvaluationError(f"{episode_id}: invalid stop forecast")
        if any(float(stop_row[k]) != 0.0 for k in ("estimated_gross_evsi", "lower_gross_evsi", "upper_gross_evsi")):
            raise EvaluationError(f"{episode_id}: stop/no-acquisition forecast must be exactly zero")
        gross_errors.append(0.0)
        net_errors.append(0.0)
        interval_widths.append(0.0)
        interval_scores.append(0.0)
        interval_hits += 1
        interval_count += 1

        # Pairwise ranking is scored across all alternatives, including unselected actions.
        action_ids = sorted(expected_ids)
        for i, left in enumerate(action_ids):
            for right in action_ids[i + 1:]:
                truth_delta = true_net[left] - true_net[right]
                estimate_delta = estimated_net[left] - estimated_net[right]
                if abs(truth_delta) <= 1e-12:
                    continue
                rank_pairs += 1
                if truth_delta * estimate_delta > 0:
                    rank_correct += 1
                elif abs(estimate_delta) <= 1e-12:
                    rank_correct += 0.5

        eligible_ids = [action_id for action_id in expected_ids if feasible[action_id]]
        best_value = max(true_net[action_id] for action_id in eligible_ids)
        best_ids = sorted(action_id for action_id in eligible_ids if abs(true_net[action_id] - best_value) <= 1e-12)
        selected_id = prediction.get("selected_action_id")
        if not isinstance(selected_id, str) or selected_id not in expected_ids:
            raise EvaluationError(f"{episode_id}: selected action is not in the catalogue or stop set")
        if not feasible[selected_id]:
            raise EvaluationError(f"{episode_id}: selected action is deadline- or budget-infeasible")
        regret = max(0.0, best_value - true_net[selected_id])
        is_best = selected_id in best_ids
        missed_best += 0 if is_best else 1
        total_regret += regret
        selected += 1
        per_episode.append({
            "episode_id": episode_id,
            "selected_action_id": selected_id,
            "oracle_best_action_ids": best_ids,
            "selected_action_feasible": True,
            "missed_best_action": not is_best,
            "decision_regret": round(regret, 12),
            "best_feasible_net_evsi": round(best_value, 12),
        })

    mean_abs = lambda values: sum(abs(v) for v in values) / len(values) if values else 0.0
    mean = lambda values: sum(values) / len(values) if values else 0.0
    return {
        "schema_version": 1,
        "benchmark_id": BENCHMARK_ID,
        "policy_id": submission.get("policy_id"),
        "run_id": submission.get("run_id"),
        "totals": {
            "episode_count": len(per_episode),
            "forecast_count": len(gross_errors),
            "action_space_coverage": 1.0,
            "gross_evsi_mae": round(mean_abs(gross_errors), 12),
            "gross_evsi_signed_bias": round(mean(gross_errors), 12),
            "net_evsi_mae": round(mean_abs(net_errors), 12),
            "net_evsi_signed_bias": round(mean(net_errors), 12),
            "forecast_interval_alpha": FORECAST_INTERVAL_ALPHA,
            "forecast_interval_coverage": round(interval_hits / interval_count, 12) if interval_count else 0.0,
            "forecast_interval_mean_width": round(mean(interval_widths), 12),
            "forecast_interval_winkler_score": round(mean(interval_scores), 12),
            "pairwise_rank_accuracy": round(rank_correct / rank_pairs, 12) if rank_pairs else 1.0,
            "pairwise_rank_pairs": rank_pairs,
            "missed_best_action_count": missed_best,
            "decision_regret_total": round(total_regret, 12),
            "decision_regret_mean": round(total_regret / selected, 12) if selected else 0.0,
            "oracle_action_selection_accuracy": round((selected - missed_best) / selected, 12) if selected else 0.0,
        },
        "episodes": per_episode,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workload", required=True)
    parser.add_argument("--truth", required=True)
    parser.add_argument("--submission", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        score = evaluate(read_json(args.workload), read_json(args.truth), read_json(args.submission))
        Path(args.output).write_text(json.dumps(score, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    except (OSError, json.JSONDecodeError, EvaluationError) as exc:
        print(f"evaluation failed: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
