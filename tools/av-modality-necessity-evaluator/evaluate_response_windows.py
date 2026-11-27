#!/usr/bin/env python3
"""Deterministic evaluator for streaming response-window, silence and turn-control fixtures.

Research benchmark tooling only. Synthetic ticks are logical time, not measured
wall-clock latency or proof of physical audio output cessation.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

BENCHMARK_ID = "av-response-window-silence-v0"
SPEECH_ACTIONS = {"respond", "clarify"}
OUTPUT_ACTIONS = {"respond", "clarify", "deescalate"}


class EvaluationError(ValueError):
    """Raised for invalid workload, evaluator truth or candidate submission."""


def read_json(path: str | Path) -> dict[str, Any]:
    with Path(path).open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise EvaluationError(f"{path}: root must be an object")
    return value


def index_unique(rows: list[dict[str, Any]], key: str, label: str) -> dict[str, dict[str, Any]]:
    indexed: dict[str, dict[str, Any]] = {}
    for row in rows:
        if not isinstance(row, dict):
            raise EvaluationError(f"{label}: every row must be an object")
        value = row.get(key)
        if not isinstance(value, str) or not value or value in indexed:
            raise EvaluationError(f"{label}: missing or duplicate {key}: {value!r}")
        indexed[value] = row
    return indexed


def in_window(tick: Any, start: Any, end: Any) -> bool:
    return (
        isinstance(tick, int) and not isinstance(tick, bool)
        and isinstance(start, int) and isinstance(end, int)
        and start <= tick < end
    )


def ratio(numerator: int, denominator: int) -> float | None:
    return numerator / denominator if denominator else None


def evaluate(workload: dict[str, Any], truth: dict[str, Any], submission: dict[str, Any]) -> dict[str, Any]:
    for name, document in (("workload", workload), ("truth", truth), ("submission", submission)):
        if document.get("schema_version") != 1 or document.get("benchmark_id") != BENCHMARK_ID:
            raise EvaluationError(f"{name}: schema_version or benchmark_id mismatch")

    work_rows, truth_rows, pred_rows = (
        workload.get("episodes"), truth.get("episodes"), submission.get("predictions")
    )
    if not all(isinstance(rows, list) for rows in (work_rows, truth_rows, pred_rows)):
        raise EvaluationError("episodes and predictions must be arrays")
    work = index_unique(work_rows, "episode_id", "workload")
    labels = index_unique(truth_rows, "episode_id", "truth")
    predictions = index_unique(pred_rows, "episode_id", "submission")
    ids = set(work)
    if set(labels) != ids or set(predictions) != ids:
        raise EvaluationError("workload, truth and submission episode IDs must match exactly")

    totals = {
        "episode_count": len(ids),
        "decision_count": 0,
        "correct_decision_count": 0,
        "required_response_count": 0,
        "response_window_hits": 0,
        "early_responses": 0,
        "late_responses": 0,
        "missed_responses": 0,
        "silence_window_count": 0,
        "silence_false_positives": 0,
        "unsolicited_responses": 0,
        "duplicate_responses": 0,
        "required_yields": 0,
        "yield_ack_within_bound_count": 0,
        "yield_ack_failures": 0,
        "required_stops": 0,
        "stop_ack_within_bound_count": 0,
        "stop_ack_failures": 0,
        "stop_latch_violations": 0,
        "backchannel_event_count": 0,
        "backchannel_false_yields": 0,
        "required_cancellations": 0,
        "cancellation_actions": 0,
        "stale_responses": 0,
        "required_deescalations": 0,
        "deescalation_actions": 0,
        "required_abstentions": 0,
        "abstention_actions": 0,
        "background_load_episodes": 0,
        "background_window_misses": 0,
    }
    per_episode: list[dict[str, Any]] = []
    silence_windows_with_false_positive = 0

    for episode_id in sorted(ids):
        episode, label, candidate = work[episode_id], labels[episode_id], predictions[episode_id]
        event_by_id = index_unique(episode.get("stream_events", []), "event_id", f"{episode_id} stream events")
        actions = candidate.get("actions")
        if not isinstance(actions, list):
            raise EvaluationError(f"{episode_id}: actions must be an array")
        action_by_id = index_unique(actions, "action_id", f"{episode_id} actions")
        if len(action_by_id) != len(actions):
            raise EvaluationError(f"{episode_id}: duplicate actions")

        violations: list[str] = []
        invalid_action_ids: set[str] = set()
        action_rows = list(action_by_id.values())
        for action in action_rows:
            trigger = action.get("trigger_event_id")
            if trigger not in event_by_id:
                invalid_action_ids.add(action["action_id"])
                violations.append(f"{action.get('action_id')}:unknown_trigger_event")
                continue
            trigger_tick = event_by_id[trigger].get("tick")
            decision_tick = action.get("decision_tick")
            if not isinstance(decision_tick, int) or isinstance(decision_tick, bool):
                invalid_action_ids.add(action["action_id"])
                violations.append(f"{action.get('action_id')}:invalid_decision_tick")
            elif isinstance(trigger_tick, int) and decision_tick < trigger_tick:
                invalid_action_ids.add(action["action_id"])
                violations.append(f"{action.get('action_id')}:decision_precedes_trigger")
            output_tick = action.get("output_start_tick")
            if output_tick is not None:
                if not isinstance(output_tick, int) or isinstance(output_tick, bool):
                    invalid_action_ids.add(action["action_id"])
                    violations.append(f"{action.get('action_id')}:invalid_output_start_tick")
                elif isinstance(trigger_tick, int) and output_tick < trigger_tick:
                    invalid_action_ids.add(action["action_id"])
                    violations.append(f"{action.get('action_id')}:output_precedes_trigger")
            if action.get("output_end_tick") is not None and isinstance(output_tick, int):
                if action["output_end_tick"] < output_tick:
                    violations.append(f"{action.get('action_id')}:output_end_precedes_start")
            if action.get("output_stop_ack_tick") is not None and isinstance(decision_tick, int):
                if action["output_stop_ack_tick"] < decision_tick:
                    violations.append(f"{action.get('action_id')}:stop_ack_precedes_request")

        decisions = label.get("decisions", [])
        if not isinstance(decisions, list):
            raise EvaluationError(f"{episode_id}: decisions must be an array")
        decision_rows = index_unique(decisions, "decision_id", f"{episode_id} truth decisions")
        expected_speech_triggers = {
            row["trigger_event_id"] for row in decisions
            if row.get("expected_action") in SPEECH_ACTIONS
        }
        expected_by_trigger: dict[str, dict[str, Any]] = {}
        for decision in decisions:
            trigger = decision.get("trigger_event_id")
            if trigger not in event_by_id:
                raise EvaluationError(f"{episode_id}: truth decision references unknown event {trigger!r}")
            if trigger in expected_by_trigger:
                raise EvaluationError(f"{episode_id}: multiple expected decisions target {trigger}")
            expected_by_trigger[trigger] = decision

        response_windows = 0
        window_hits = 0
        early = late = missed = 0
        correct_decisions = 0
        yield_failures = stop_failures = 0
        cancellation_actions = 0
        deescalation_actions = 0
        abstention_actions = 0
        duplicate_responses = 0
        backchannel_false_yields = 0
        stop_latch_violations = 0
        stale_responses = 0
        unsolicited_responses = 0

        actions_by_trigger: dict[str, list[dict[str, Any]]] = {}
        for action in action_rows:
            actions_by_trigger.setdefault(action["trigger_event_id"], []).append(action)

        for decision in decisions:
            trigger = decision["trigger_event_id"]
            expected_action = decision["expected_action"]
            candidates = actions_by_trigger.get(trigger, [])
            expected_match = next(
                (a for a in candidates if a.get("action_type") == expected_action),
                None,
            )
            decision_ok = False

            if expected_action == "silence":
                floor_disrupting = any(
                    a.get("action_type") in (SPEECH_ACTIONS | {"yield", "stop"})
                    for a in candidates
                )
                decision_ok = not floor_disrupting
            elif expected_action == "respond":
                # WRR is defined over required response events, not clarification
                # opportunities. Clarification quality remains part of decision accuracy.
                response_windows += 1
                candidate_action = expected_match
                output_tick = candidate_action.get("output_start_tick") if candidate_action else None
                if candidate_action is None or candidate_action["action_id"] in invalid_action_ids or output_tick is None:
                    missed += 1
                elif output_tick < decision.get("window_start_tick", 0):
                    early += 1
                elif output_tick >= decision.get("window_end_tick", 0):
                    late += 1
                else:
                    window_hits += 1
                    decision_ok = True
                    event_ref = event_by_id[trigger].get("observation_ref")
                    if event_ref and event_ref not in candidate_action.get("evidence_refs", []):
                        violations.append(f"{candidate_action['action_id']}:required_trigger_evidence_not_cited")
                        decision_ok = False
            elif expected_action == "clarify":
                # A clarify action is scored as a correct interaction decision,
                # but is intentionally excluded from DIVE-style response-window WRR.
                candidate_action = expected_match
                if candidate_action is not None and candidate_action["action_id"] not in invalid_action_ids and in_window(
                    candidate_action.get("output_start_tick"),
                    decision.get("window_start_tick"),
                    decision.get("window_end_tick"),
                ):
                    decision_ok = True
                    event_ref = event_by_id[trigger].get("observation_ref")
                    if event_ref and event_ref not in candidate_action.get("evidence_refs", []):
                        violations.append(f"{candidate_action['action_id']}:required_trigger_evidence_not_cited")
                        decision_ok = False
            elif expected_action in {"yield", "stop"}:
                candidate_action = expected_match
                if candidate_action is not None and candidate_action["action_id"] not in invalid_action_ids and in_window(
                    candidate_action.get("decision_tick"),
                    decision.get("window_start_tick"),
                    decision.get("window_end_tick"),
                ):
                    max_latency = decision.get("max_ack_latency_ticks")
                    ack_tick = candidate_action.get("output_stop_ack_tick")
                    latency_ok = (
                        isinstance(max_latency, int) and isinstance(ack_tick, int)
                        and ack_tick - candidate_action["decision_tick"] <= max_latency
                    )
                    if latency_ok:
                        decision_ok = True
                    elif expected_action == "yield":
                        yield_failures += 1
                    else:
                        stop_failures += 1
                else:
                    if expected_action == "yield":
                        yield_failures += 1
                    else:
                        stop_failures += 1
            elif expected_action in {"cancel", "deescalate", "abstain"}:
                candidate_action = expected_match
                if candidate_action is not None and candidate_action["action_id"] not in invalid_action_ids and in_window(
                    candidate_action.get("decision_tick"),
                    decision.get("window_start_tick"),
                    decision.get("window_end_tick"),
                ):
                    if expected_action == "cancel":
                        decision_ok = candidate_action.get("target_action_id") == decision.get("target_action_id")
                    else:
                        decision_ok = True

            if decision_ok:
                correct_decisions += 1

        forbidden_yield_triggers = {
            item.get("trigger_event_id")
            for item in label.get("forbidden_actions", [])
            if item.get("action_type") == "yield"
        }
        for action in action_rows:
            action_type = action.get("action_type")
            trigger = action.get("trigger_event_id")
            if action_type in SPEECH_ACTIONS and trigger not in expected_speech_triggers:
                unsolicited_responses += 1
            if action_type in SPEECH_ACTIONS and trigger in label.get("duplicate_event_ids", []):
                duplicate_responses += 1
            if action_type == "yield" and trigger in forbidden_yield_triggers:
                backchannel_false_yields += 1
            if action_type == "cancel":
                cancellation_actions += 1
            if action_type == "deescalate":
                deescalation_actions += 1
            if action_type == "abstain":
                abstention_actions += 1
        for decision in decisions:
            if decision.get("expected_action") in SPEECH_ACTIONS:
                matching_outputs = [
                    action for action in actions_by_trigger.get(decision["trigger_event_id"], [])
                    if action.get("action_type") == decision["expected_action"]
                ]
                duplicate_responses += max(0, len(matching_outputs) - 1)

        silence_windows = label.get("silence_windows", [])
        silence_false_positives = 0
        silence_windows_hit: set[int] = set()
        for action in action_rows:
            if action.get("action_type") not in OUTPUT_ACTIONS:
                continue
            output_tick = action.get("output_start_tick")
            if not isinstance(output_tick, int):
                continue
            for window_index, window in enumerate(silence_windows):
                if in_window(output_tick, window.get("start_tick"), window.get("end_tick")):
                    silence_false_positives += 1
                    silence_windows_hit.add(window_index)
                    break
        silence_windows_with_false_positive += len(silence_windows_hit)

        latching = label.get("latching_stop")
        if isinstance(latching, dict):
            for action in action_rows:
                if (
                    action.get("action_type") in OUTPUT_ACTIONS
                    and action.get("turn_epoch") == latching.get("turn_epoch")
                    and isinstance(action.get("output_start_tick"), int)
                    and action["output_start_tick"] >= latching.get("after_tick", 0)
                ):
                    stop_latch_violations += 1
                    violations.append(f"{action['action_id']}:output_after_latching_stop")

        superseded_ids = set(label.get("superseded_trigger_event_ids", []))
        superseded_at = label.get("superseded_at_tick")
        for action in action_rows:
            if (
                action.get("action_type") in OUTPUT_ACTIONS
                and action.get("trigger_event_id") in superseded_ids
                and isinstance(action.get("output_start_tick"), int)
                and isinstance(superseded_at, int)
                and action["output_start_tick"] >= superseded_at
            ):
                stale_responses += 1
                violations.append(f"{action['action_id']}:stale_superseded_response")

        required_yields = sum(d["expected_action"] == "yield" for d in decisions)
        required_stops = sum(d["expected_action"] == "stop" for d in decisions)
        required_cancellations = sum(d["expected_action"] == "cancel" for d in decisions)
        required_deescalations = sum(d["expected_action"] == "deescalate" for d in decisions)
        required_abstentions = sum(d["expected_action"] == "abstain" for d in decisions)
        yield_ack_within = required_yields - yield_failures
        stop_ack_within = required_stops - stop_failures
        backchannel_count = sum(
            1 for event in episode["stream_events"] if event["event_type"] == "backchannel"
        )
        if episode.get("background_load_profile") != "idle":
            totals["background_load_episodes"] += 1
            if any(
                decision.get("expected_action") == "respond"
                and not any(
                    action.get("action_type") == "respond"
                    and action.get("trigger_event_id") == decision.get("trigger_event_id")
                    and in_window(action.get("output_start_tick"), decision.get("window_start_tick"), decision.get("window_end_tick"))
                    for action in action_rows
                )
                for decision in decisions
            ):
                totals["background_window_misses"] += 1

        local = {
            "episode_id": episode_id,
            "decision_count": len(decisions),
            "correct_decision_count": correct_decisions,
            "response_window_count": response_windows,
            "response_window_hits": window_hits,
            "early_responses": early,
            "late_responses": late,
            "missed_responses": missed,
            "silence_window_count": len(silence_windows),
            "silence_false_positives": silence_false_positives,
            "unsolicited_responses": unsolicited_responses,
            "duplicate_responses": duplicate_responses,
            "yield_ack_failures": yield_failures,
            "stop_ack_failures": stop_failures,
            "stop_latch_violations": stop_latch_violations,
            "stale_responses": stale_responses,
            "backchannel_false_yields": backchannel_false_yields,
            "violations": sorted(set(violations)),
        }
        per_episode.append(local)

        totals["decision_count"] += len(decisions)
        totals["correct_decision_count"] += correct_decisions
        totals["required_response_count"] += response_windows
        totals["response_window_hits"] += window_hits
        totals["early_responses"] += early
        totals["late_responses"] += late
        totals["missed_responses"] += missed
        totals["silence_window_count"] += len(silence_windows)
        totals["silence_false_positives"] += silence_false_positives
        totals["unsolicited_responses"] += unsolicited_responses
        totals["duplicate_responses"] += duplicate_responses
        totals["required_yields"] += required_yields
        totals["yield_ack_within_bound_count"] += yield_ack_within
        totals["yield_ack_failures"] += yield_failures
        totals["required_stops"] += required_stops
        totals["stop_ack_within_bound_count"] += stop_ack_within
        totals["stop_ack_failures"] += stop_failures
        totals["stop_latch_violations"] += stop_latch_violations
        totals["backchannel_event_count"] += backchannel_count
        totals["backchannel_false_yields"] += backchannel_false_yields
        totals["required_cancellations"] += required_cancellations
        totals["cancellation_actions"] += cancellation_actions
        totals["stale_responses"] += stale_responses
        totals["required_deescalations"] += required_deescalations
        totals["deescalation_actions"] += deescalation_actions
        totals["required_abstentions"] += required_abstentions
        totals["abstention_actions"] += abstention_actions

    for key, numerator, denominator in (
        ("decision_accuracy", totals["correct_decision_count"], totals["decision_count"]),
        ("response_window_rate", totals["response_window_hits"], totals["required_response_count"]),
        ("silence_precision", totals["silence_window_count"] - silence_windows_with_false_positive, totals["silence_window_count"]),
    ):
        totals[key] = ratio(numerator, denominator)

    return {
        "schema_version": 1,
        "benchmark_id": BENCHMARK_ID,
        "policy_id": submission.get("policy_id"),
        "run_id": submission.get("run_id"),
        "interpretation": {
            "status": "synthetic_response_window_score",
            "limitations": [
                "The workload contains twelve synthetic metadata-only streams, not real audiovisual recordings or human-annotated live interactions.",
                "Ticks are logical simulation values, not wall-clock latency measurements.",
                "A yield/stop acknowledgement in this submission is candidate-reported; physical output-stop timing requires independent renderer or hardware acknowledgement.",
                "The scorecard is not an official SPOT-Bench, DIVE or ALIVE result and cannot establish SOTA superiority."
            ]
        },
        "totals": totals,
        "per_episode": per_episode,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workload", required=True)
    parser.add_argument("--truth", required=True, help="Evaluator-only expected decisions and response windows")
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
        f"wrote {output}; episodes={report['totals']['episode_count']} "
        f"window_rate={report['totals']['response_window_rate']} "
        f"silence_precision={report['totals']['silence_precision']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())