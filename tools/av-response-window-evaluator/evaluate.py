#!/usr/bin/env python3
"""Deterministic scorer for proactive response decisions and temporal windows.

Research/benchmark tooling only; this is not a full-duplex voice controller,
output scheduler, speech renderer, or production interaction service.
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

SCHEMA_VERSION = 1
BENCHMARK_ID = "av-response-window-v0"


class EvaluationError(ValueError):
    """Raised when response-window evaluation inputs are structurally invalid."""


def read_json(path: str | Path) -> dict[str, Any]:
    with Path(path).open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise EvaluationError(f"{path}: root must be an object")
    return value


def ratio(numerator: int, denominator: int) -> float | None:
    return numerator / denominator if denominator else None


def percentile(values: list[int], q: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    index = max(0, math.ceil(q * len(ordered)) - 1)
    return float(ordered[index])


def index_unique(rows: list[dict[str, Any]], key: str) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for row in rows:
        value = row.get(key)
        if not isinstance(value, str) or not value:
            raise EvaluationError(f"missing or invalid {key}")
        if value in result:
            raise EvaluationError(f"duplicate {key}: {value}")
        result[value] = row
    return result


def evaluate(
    workload: dict[str, Any],
    truth: dict[str, Any],
    submission: dict[str, Any],
) -> dict[str, Any]:
    """Score one response-submission against workload prompts and evaluator truth."""
    for label, document in (("workload", workload), ("truth", truth), ("submission", submission)):
        if document.get("schema_version") != SCHEMA_VERSION:
            raise EvaluationError(f"{label}: unsupported schema_version")
        if document.get("benchmark_id") != BENCHMARK_ID:
            raise EvaluationError(f"{label}: benchmark_id mismatch")

    workload_rows = workload.get("episodes")
    truth_rows = truth.get("episodes")
    response_rows = submission.get("responses")
    if not all(isinstance(rows, list) for rows in (workload_rows, truth_rows, response_rows)):
        raise EvaluationError("episodes and responses must be arrays")
    work = index_unique(workload_rows, "episode_id")
    expected = index_unique(truth_rows, "episode_id")
    if set(work) != set(expected):
        raise EvaluationError("workload/truth episode IDs do not match exactly")

    response_ids: set[str] = set()
    responses_by_episode: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for response in response_rows:
        response_id = response.get("response_id")
        episode_id = response.get("episode_id")
        if not isinstance(response_id, str) or not response_id or response_id in response_ids:
            raise EvaluationError(f"invalid or duplicate response_id: {response_id!r}")
        response_ids.add(response_id)
        if episode_id not in work:
            raise EvaluationError(f"response references unknown episode: {episode_id!r}")
        responses_by_episode[episode_id].append(response)

    per_episode: list[dict[str, Any]] = []
    total = {
        "opportunity_count": 0,
        "response_required_count": 0,
        "expected_silence_count": 0,
        "interaction_decision_correct_count": 0,
        "false_intervention_count": 0,
        "response_count": 0,
        "on_time_count": 0,
        "early_count": 0,
        "late_count": 0,
        "missed_count": 0,
        "content_correct_count": 0,
        "on_time_content_correct_count": 0,
        "deadline_miss_count": 0,
        "duplicate_response_count": 0,
    }
    total["response_count"] = len(response_rows)
    on_time_offsets: list[int] = []
    quality_denominator = 0
    on_time_quality_denominator = 0

    for episode_id in sorted(work):
        episode = work[episode_id]
        truth_episode = expected[episode_id]
        query_tick = episode.get("query_tick")
        deadline = episode.get("response_deadline_tick")
        if not isinstance(query_tick, int) or isinstance(query_tick, bool):
            raise EvaluationError(f"{episode_id}: invalid query_tick")
        if not isinstance(deadline, int) or isinstance(deadline, bool) or deadline < query_tick:
            raise EvaluationError(f"{episode_id}: invalid response deadline")

        opportunity_rows = truth_episode.get("opportunities")
        if not isinstance(opportunity_rows, list) or not opportunity_rows:
            raise EvaluationError(f"{episode_id}: opportunities must be a nonempty array")
        opportunities = index_unique(opportunity_rows, "opportunity_id")
        opportunities_by_trigger: dict[str, dict[str, Any]] = {}
        for opportunity in opportunity_rows:
            trigger_ref = opportunity.get("trigger_ref")
            if not isinstance(trigger_ref, str) or not trigger_ref or trigger_ref in opportunities_by_trigger:
                raise EvaluationError(f"{episode_id}: invalid or duplicate opportunity trigger_ref")
            opportunities_by_trigger[trigger_ref] = opportunity

        stream_events = episode.get("stream_events")
        if not isinstance(stream_events, list):
            raise EvaluationError(f"{episode_id}: stream_events must be an array")
        seen_event_ids: set[str] = set()
        visible_trigger_refs: set[str] = set()
        trigger_ticks: dict[str, int] = {}
        cancellation_ticks: dict[str, int] = {}
        previous_event_tick = -1
        for event in stream_events:
            event_id = event.get("event_id")
            tick = event.get("tick")
            if not isinstance(event_id, str) or not event_id or event_id in seen_event_ids:
                raise EvaluationError(f"{episode_id}: invalid or duplicate event_id")
            if not isinstance(tick, int) or isinstance(tick, bool) or tick < previous_event_tick:
                raise EvaluationError(f"{episode_id}: stream events must have ordered non-negative ticks")
            previous_event_tick = tick
            seen_event_ids.add(event_id)
            trigger_ref = event.get("trigger_ref")
            if isinstance(trigger_ref, str) and trigger_ref:
                visible_trigger_refs.add(trigger_ref)
                if event.get("kind") in {"trigger", "duplicate_trigger"}:
                    trigger_ticks.setdefault(trigger_ref, tick)
                if event.get("kind") == "cancel":
                    cancellation_ticks[trigger_ref] = min(
                        cancellation_ticks.get(trigger_ref, tick), tick
                    )
        for trigger_ref in opportunities_by_trigger:
            if trigger_ref not in visible_trigger_refs:
                raise EvaluationError(f"{episode_id}: truth trigger_ref is not in candidate-visible workload")

        for opportunity in opportunity_rows:
            if opportunity.get("expected_participation") == "respond":
                trigger_tick = trigger_ticks.get(opportunity["trigger_ref"])
                window_start = opportunity.get("window_start_tick")
                window_end = opportunity.get("window_end_tick")
                if trigger_tick is None:
                    raise EvaluationError(f"{episode_id}: response opportunity has no first-delivery trigger")
                if (
                    not isinstance(window_start, int) or isinstance(window_start, bool)
                    or not isinstance(window_end, int) or isinstance(window_end, bool)
                    or window_start < trigger_tick
                    or window_end <= window_start
                    or window_start < query_tick
                    or window_end > deadline
                ):
                    raise EvaluationError(f"{episode_id}/{opportunity['opportunity_id']}: response window is not causally admissible")

        by_opportunity: dict[str, list[dict[str, Any]]] = defaultdict(list)
        episode_violations: list[str] = []
        validated_intervals: list[tuple[int, int, str]] = []
        for response in responses_by_episode[episode_id]:
            trigger_ref = response.get("assigned_trigger_ref")
            if trigger_ref not in opportunities_by_trigger:
                episode_violations.append("response_assigned_to_unknown_trigger")
                continue
            start_tick = response.get("response_start_tick")
            end_tick = response.get("response_end_tick")
            if (
                not isinstance(start_tick, int) or isinstance(start_tick, bool) or start_tick < 0
                or not isinstance(end_tick, int) or isinstance(end_tick, bool) or end_tick < start_tick
            ):
                episode_violations.append("invalid_response_interval")
                continue
            if start_tick < query_tick:
                episode_violations.append("response_precedes_query")
            if end_tick > deadline:
                episode_violations.append("response_overhang_past_episode_deadline")
            validated_intervals.append((start_tick, end_tick, response["response_id"]))
            cancel_tick = cancellation_ticks.get(trigger_ref)
            if cancel_tick is not None:
                if start_tick >= cancel_tick:
                    episode_violations.append("response_starts_after_cancellation")
                if end_tick > cancel_tick:
                    episode_violations.append("response_continues_after_cancellation")
            opportunity = opportunities_by_trigger[trigger_ref]
            opportunity_id = opportunity["opportunity_id"]
            by_opportunity[opportunity_id].append(response)

        validated_intervals.sort()
        for previous, current in zip(validated_intervals, validated_intervals[1:]):
            if current[0] < previous[1]:
                episode_violations.append("assistant_response_intervals_overlap")

        episode_total = {
            "opportunity_count": len(opportunities),
            "respond_required_count": 0,
            "silent_expected_count": 0,
            "decision_correct_count": 0,
            "responses_assigned": len(responses_by_episode[episode_id]),
            "duplicate_response_count": 0,
            "on_time_count": 0,
            "early_count": 0,
            "late_count": 0,
            "missed_count": 0,
            "false_intervention_count": 0,
            "content_correct_count": 0,
            "response_quality_accuracy": None,
            "deadline_miss_count": 0,
        }
        episode_quality_denominator = 0
        episode_deadline_misses = 0

        for opportunity_id in sorted(opportunities):
            opportunity = opportunities[opportunity_id]
            participation = opportunity.get("expected_participation")
            assigned = sorted(
                by_opportunity.get(opportunity_id, []),
                key=lambda row: (row.get("response_start_tick", 0), row.get("response_id", "")),
            )
            if len(assigned) > 1:
                episode_violations.append("multiple_responses_assigned_to_one_opportunity")
                total["duplicate_response_count"] += len(assigned) - 1
                episode_total["duplicate_response_count"] += len(assigned) - 1

            if participation == "silent":
                total["expected_silence_count"] += 1
                episode_total["silent_expected_count"] += 1
                if assigned:
                    total["false_intervention_count"] += 1
                    episode_total["false_intervention_count"] += 1
                else:
                    total["interaction_decision_correct_count"] += 1
                    episode_total["decision_correct_count"] += 1
                continue

            if participation != "respond":
                raise EvaluationError(f"{episode_id}/{opportunity_id}: invalid participation label")
            start = opportunity.get("window_start_tick")
            end = opportunity.get("window_end_tick")
            if (
                not isinstance(start, int) or isinstance(start, bool)
                or not isinstance(end, int) or isinstance(end, bool)
                or start >= end
            ):
                raise EvaluationError(f"{episode_id}/{opportunity_id}: invalid half-open response window")

            total["response_required_count"] += 1
            episode_total["respond_required_count"] += 1
            if not assigned:
                total["missed_count"] += 1
                episode_total["missed_count"] += 1
                continue

            response = assigned[0]
            tick = response["response_start_tick"]
            total["interaction_decision_correct_count"] += 1
            episode_total["decision_correct_count"] += 1

            if tick < start:
                total["early_count"] += 1
                episode_total["early_count"] += 1
            elif tick < end:
                total["on_time_count"] += 1
                episode_total["on_time_count"] += 1
                on_time_offsets.append(tick - start)
                on_time_quality_denominator += 1
            else:
                total["late_count"] += 1
                episode_total["late_count"] += 1

            if response["response_end_tick"] > deadline:
                total["deadline_miss_count"] += 1
                episode_deadline_misses += 1

            content_correct = response.get("content") == opportunity.get("expected_content")
            episode_quality_denominator += 1
            quality_denominator += 1
            if content_correct:
                total["content_correct_count"] += 1
                episode_total["content_correct_count"] += 1
            if tick >= start and tick < end:
                if content_correct:
                    total["on_time_content_correct_count"] += 1

        total["opportunity_count"] += episode_total["opportunity_count"]
        episode_total["response_quality_accuracy"] = ratio(
            episode_total["content_correct_count"], episode_quality_denominator
        )
        episode_total["deadline_miss_count"] = episode_deadline_misses
        per_episode.append({
            "episode_id": episode_id,
            **episode_total,
            "violations": sorted(set(episode_violations)),
        })

    # Count every duplicate assigned response as a hard failure; unknown response
    # assignments and pre-query responses are also hard failures.
    hard_violations = [
        {"episode_id": row["episode_id"], "violations": row["violations"]}
        for row in per_episode if row["violations"]
    ]
    totals = {
        "episode_count": len(work),
        **total,
        "interaction_decision_accuracy": ratio(
            total["interaction_decision_correct_count"], total["opportunity_count"]
        ),
        "expected_silence_specificity": ratio(
            total["expected_silence_count"] - total["false_intervention_count"],
            total["expected_silence_count"],
        ),
        "window_response_rate": ratio(
            total["on_time_count"], total["response_required_count"]
        ),
        "content_accuracy": ratio(
            total["content_correct_count"], quality_denominator
        ),
        "on_time_content_accuracy": ratio(
            total["on_time_content_correct_count"], on_time_quality_denominator
        ),
        "response_offset_p50_ticks": percentile(on_time_offsets, 0.50),
        "response_offset_p95_ticks": percentile(on_time_offsets, 0.95),
        "hard_violation_count": sum(len(row["violations"]) for row in per_episode),
    }

    return {
        "schema_version": 1,
        "benchmark_id": BENCHMARK_ID,
        "policy_id": submission.get("policy_id"),
        "run_id": submission.get("run_id"),
        "interpretation": {
            "status": "synthetic_response_window_score",
            "limitations": [
                "The workload and valid response windows are synthetic replay ticks, not measured milliseconds.",
                "The reference submission validates evaluator semantics only; it is not a model result or a DIVE reproduction.",
                "Content correctness is compared against author-authored exact labels; no external human-rated quality cohort is included.",
                "A response is assigned to exactly one opportunity. The evaluator does not infer semantic cross-event attribution.",
                "No acoustic cancellation, hardware playback, network or physical loopback guarantee is measured."
            ],
        },
        "totals": totals,
        "per_episode": per_episode,
        "hard_violations": hard_violations,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workload", required=True)
    parser.add_argument("--truth", required=True, help="Evaluator-only response windows; never expose to candidate runtime")
    parser.add_argument("--submission", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        report = evaluate(read_json(args.workload), read_json(args.truth), read_json(args.submission))
        output = Path(args.output)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    except (OSError, json.JSONDecodeError, EvaluationError) as error:
        print(f"response-window evaluation failed: {error}", file=sys.stderr)
        return 2
    print(
        f"wrote {output}; opportunities={report['totals']['opportunity_count']} "
        f"WRR={report['totals']['window_response_rate']} "
        f"false_interventions={report['totals']['false_intervention_count']} "
        f"hard_violations={report['totals']['hard_violation_count']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())