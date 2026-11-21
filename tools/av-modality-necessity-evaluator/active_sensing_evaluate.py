#!/usr/bin/env python3
"""Deterministic finite evaluator for closed-loop AV active sensing decisions."""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path
from typing import Any

BENCHMARK_ID = "av-active-sensing-decision-value-v0"


class EvaluationError(ValueError):
    """Malformed workload, truth or candidate input."""


def read_json(path: str | Path) -> dict[str, Any]:
    with Path(path).open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise EvaluationError(f"{path}: expected an object")
    return value


def unique_index(rows: list[dict[str, Any]], key: str, name: str) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for row in rows:
        value = row.get(key)
        if not isinstance(value, str) or not value or value in result:
            raise EvaluationError(f"{name}: missing or duplicate {key}: {value!r}")
        result[value] = row
    return result


def finite_number(value: Any) -> bool:
    return (
        not isinstance(value, bool)
        and isinstance(value, (int, float))
        and math.isfinite(float(value))
    )


def evaluate(
    workload: dict[str, Any],
    truth: dict[str, Any],
    submission: dict[str, Any],
) -> dict[str, Any]:
    for name, document in (("workload", workload), ("truth", truth), ("submission", submission)):
        if document.get("schema_version") != 1 or document.get("benchmark_id") != BENCHMARK_ID:
            raise EvaluationError(f"{name}: schema version or benchmark identity mismatch")
    work_rows = workload.get("episodes")
    truth_rows = truth.get("episodes")
    prediction_rows = submission.get("predictions")
    if not all(isinstance(rows, list) for rows in (work_rows, truth_rows, prediction_rows)):
        raise EvaluationError("episodes and predictions must be arrays")
    work = unique_index(work_rows, "episode_id", "workload")
    labels = unique_index(truth_rows, "episode_id", "truth")
    predictions = unique_index(prediction_rows, "episode_id", "submission")
    if set(work) != set(labels) or set(work) != set(predictions):
        raise EvaluationError("workload, truth and predictions must contain identical episode IDs")

    results: list[dict[str, Any]] = []
    for episode_id in sorted(work):
        episode, label, prediction = work[episode_id], labels[episode_id], predictions[episode_id]
        task = episode.get("task") or {}
        budget = episode.get("budget") or {}
        query_tick, deadline = task.get("query_tick"), task.get("deadline_tick")
        response_tick = prediction.get("response_tick")
        evidence_tick = prediction.get("evidence_as_of_tick")
        if any(isinstance(x, bool) or not isinstance(x, int) or x < 0 for x in (query_tick, deadline)):
            raise EvaluationError(f"{episode_id}: invalid task ticks")
        if isinstance(response_tick, bool) or not isinstance(response_tick, int) or response_tick < query_tick:
            raise EvaluationError(f"{episode_id}: invalid response tick")
        if isinstance(evidence_tick, bool) or not isinstance(evidence_tick, int) or evidence_tick < query_tick:
            raise EvaluationError(f"{episode_id}: invalid evidence cutoff")
        if evidence_tick > response_tick:
            raise EvaluationError(f"{episode_id}: evidence cutoff follows response")
        deadline_miss = response_tick > deadline

        catalogue = unique_index(episode.get("action_catalogue", []), "action_id", f"{episode_id} action catalogue")
        outcomes = label.get("action_outcomes")
        if not isinstance(outcomes, dict):
            raise EvaluationError(f"{episode_id}: truth action_outcomes must be an object")
        for action_id, outcome in outcomes.items():
            if action_id not in catalogue or not isinstance(outcome, dict):
                raise EvaluationError(f"{episode_id}: truth contains unknown action outcome {action_id!r}")
            latency = catalogue[action_id].get("latency_ticks")
            arrival = outcome.get("arrival_tick")
            if (
                isinstance(latency, bool) or not isinstance(latency, int) or latency < 0
                or isinstance(arrival, bool) or not isinstance(arrival, int) or arrival < 0
            ):
                raise EvaluationError(f"{episode_id}: invalid action latency/outcome tick")
            status = outcome.get("status")
            if not isinstance(status, str) or status not in {"delivered", "late", "partial", "unavailable", "failed"}:
                raise EvaluationError(f"{episode_id}: unsupported action outcome status")
            if status in {"delivered", "late"}:
                if not isinstance(outcome.get("observation_ref"), str) or not outcome["observation_ref"]:
                    raise EvaluationError(f"{episode_id}: delivered/late outcome requires an observation reference")
            elif outcome.get("observation_ref") is not None:
                raise EvaluationError(f"{episode_id}: partial/unavailable/failed outcome cannot contain an observation reference")

        requests = prediction.get("action_requests")
        if not isinstance(requests, list):
            requests = []
        violations: list[str] = []
        requested_ids: list[str] = []
        request_ids: set[str] = set()
        uses: dict[str, int] = {}
        total_audio = total_video = 0
        total_energy = 0.0
        requested_outcomes: list[tuple[dict[str, Any], dict[str, Any], int, bool]] = []
        for request in requests:
            if not isinstance(request, dict):
                violations.append("invalid_action_request")
                continue
            request_id = request.get("request_id")
            action_id = request.get("action_id")
            request_tick = request.get("request_tick")
            if not isinstance(request_id, str) or not request_id or request_id in request_ids:
                violations.append("invalid_or_duplicate_request_id")
                continue
            request_ids.add(request_id)
            if not isinstance(action_id, str) or action_id not in catalogue:
                violations.append("unknown_action_requested")
                continue
            if isinstance(request_tick, bool) or not isinstance(request_tick, int) or request_tick < query_tick:
                violations.append("action_request_precedes_query_or_has_invalid_tick")
                continue
            if request_tick > response_tick:
                violations.append("action_requested_after_response")
            action = catalogue[action_id]
            requested_ids.append(action_id)
            uses[action_id] = uses.get(action_id, 0) + 1
            within_use_limit = uses[action_id] <= action.get("max_uses", 0)
            if not within_use_limit:
                violations.append("action_max_uses_exceeded")
            resource = action.get("resource_cost") or {}
            audio_cost, video_cost, energy_cost = (
                resource.get("audio_bytes"), resource.get("video_frames"), resource.get("energy_joules")
            )
            if (
                isinstance(audio_cost, bool) or not isinstance(audio_cost, int) or audio_cost < 0
                or isinstance(video_cost, bool) or not isinstance(video_cost, int) or video_cost < 0
                or not finite_number(energy_cost) or energy_cost < 0
            ):
                violations.append("invalid_action_resource_cost")
                continue
            total_audio += audio_cost
            total_video += video_cost
            total_energy += float(energy_cost)
            latency = action.get("latency_ticks")
            expected_arrival = request_tick + latency
            outcome = outcomes.get(action_id)
            if outcome is None:
                violations.append("requested_action_has_no_evaluator_outcome")
                continue
            if outcome["arrival_tick"] != expected_arrival:
                raise EvaluationError(f"{episode_id}: outcome arrival does not match action latency")
            requested_outcomes.append((action, outcome, request_tick, within_use_limit))

            for estimate_field in ("estimated_eig_nats", "estimated_evsi", "estimated_cost_joules"):
                estimate = request.get(estimate_field)
                if not finite_number(estimate):
                    violations.append(f"invalid_{estimate_field}")
            if finite_number(request.get("estimated_eig_nats")) and request["estimated_eig_nats"] < 0:
                violations.append("negative_estimated_eig")
            if finite_number(request.get("estimated_cost_joules")) and request["estimated_cost_joules"] < 0:
                violations.append("negative_estimated_cost")

        budget_violations: list[str] = []
        if len(requests) > budget.get("max_actions", -1):
            budget_violations.append("max_actions_exceeded")
        if total_audio > budget.get("max_audio_bytes", -1):
            budget_violations.append("audio_byte_budget_exceeded")
        if total_video > budget.get("max_video_frames", -1):
            budget_violations.append("video_frame_budget_exceeded")
        if total_energy > budget.get("max_energy_joules", -1):
            budget_violations.append("energy_budget_exceeded")

        initial = episode.get("initial_observations", [])
        admitted: dict[str, tuple[int, str | None]] = {}
        for observation in initial:
            ref = observation["observation_ref"]
            tick = observation["availability_tick"]
            if ref in admitted:
                raise EvaluationError(f"{episode_id}: duplicate initial observation reference")
            admitted[ref] = (tick, observation.get("source_dependency_group"))
        for action, outcome, _, within_use_limit in requested_outcomes:
            if outcome.get("observation_ref") is not None:
                ref = outcome["observation_ref"]
                if ref in admitted:
                    violations.append("duplicate_action_observation_reference")
                else:
                    # A late result has an identity and an arrival time, but is not
                    # eligible before its arrival or after the task deadline.
                    admitted[ref] = (outcome["arrival_tick"], outcome.get("source_dependency_group"))

        citations = prediction.get("cited_observation_refs")
        if not isinstance(citations, list) or any(not isinstance(ref, str) or not ref for ref in citations):
            citations = []
            violations.append("invalid_citation_list")
        if len(citations) != len(set(citations)):
            violations.append("duplicate_observation_citation")
        admissible_citations: set[str] = set()
        for ref in citations:
            if ref not in admitted:
                violations.append("fabricated_or_unrequested_observation")
            elif admitted[ref][0] > evidence_tick:
                violations.append("future_or_post_cutoff_observation_cited")
            elif admitted[ref][0] > deadline:
                violations.append("post_deadline_observation_cited")
            else:
                admissible_citations.add(ref)

        required_refs = label.get("required_observation_refs", [])
        if (
            not isinstance(required_refs, list)
            or any(not isinstance(ref, str) or not ref for ref in required_refs)
            or len(required_refs) != len(set(required_refs))
        ):
            raise EvaluationError(f"{episode_id}: malformed required_observation_refs in evaluator truth")
        if not set(required_refs) <= admissible_citations:
            violations.append("required_action_evidence_missing_or_inadmissible")

        if prediction.get("claimed_independent_corroboration") is True:
            groups = [admitted[ref][1] for ref in citations if ref in admitted]
            nonempty_groups = [group for group in groups if isinstance(group, str) and group]
            if len(nonempty_groups) >= 2 and len(set(nonempty_groups)) < len(nonempty_groups):
                violations.append("common_cause_evidence_claimed_independent")

        late_count = sum(
            1 for _, outcome, _, _ in requested_outcomes
            if outcome.get("status") == "late" or outcome.get("arrival_tick", 0) > deadline
        )
        realized_evsi = 0.0
        wasted_count = 0
        for _, outcome, _, within_use_limit in requested_outcomes:
            delivered_on_time = (
                outcome.get("status") == "delivered"
                and outcome.get("arrival_tick", deadline + 1) <= deadline
            )
            evsi = outcome.get("evsi", 0.0)
            if not finite_number(evsi):
                raise EvaluationError(f"{episode_id}: truth EVSI must be finite")
            if within_use_limit and delivered_on_time and evsi > 0 and outcome.get("independent_source", True):
                realized_evsi += float(evsi)
            else:
                wasted_count += 1

        optimal_actions = label.get("optimal_action_ids")
        if not isinstance(optimal_actions, list) or any(not isinstance(x, str) for x in optimal_actions):
            raise EvaluationError(f"{episode_id}: optimal_action_ids must be a string array")
        expected_actions = set(optimal_actions)
        if len(expected_actions) != len(optimal_actions) or not expected_actions <= set(catalogue):
            raise EvaluationError(f"{episode_id}: invalid optimal action set")
        action_policy_correct = requested_ids == optimal_actions

        regret = 0.0
        if not action_policy_correct:
            if not requested_ids:
                regret = float(label.get("decision_regret_if_no_request", 0.0))
            elif late_count:
                regret = float(label.get("decision_regret_if_wait_past_deadline", label.get("decision_regret_if_request", 0.0)))
            elif any(action_id not in expected_actions for action_id in requested_ids):
                regret = float(label.get("decision_regret_if_wrong_action", label.get("decision_regret_if_request", 0.0)))
            else:
                regret = float(label.get("decision_regret_if_request", 0.0))
        if not finite_number(regret) or regret < 0:
            raise EvaluationError(f"{episode_id}: truth decision regret must be finite and non-negative")

        decision = prediction.get("decision")
        answer = prediction.get("answer")
        expected_answer = label.get("expected_answer")
        answer_correct = decision == "answer" and isinstance(answer, str) and answer == expected_answer
        decision_correct = (
            answer_correct
            and action_policy_correct
            and response_tick <= deadline
            and not violations
            and not budget_violations
        )
        evidence_violations = violations
        action_count = len(requests)
        results.append({
            "episode_id": episode_id,
            "answer_correct": bool(answer_correct),
            "decision_correct": bool(decision_correct),
            "action_policy_correct": bool(action_policy_correct),
            "answered": decision == "answer",
            "response_tick": response_tick,
            "decision_regret": regret,
            "acquisition_count": action_count,
            "wasted_acquisition_count": wasted_count,
            "late_acquisition_count": late_count,
            "budget_violations": sorted(set(budget_violations)),
            "evidence_violations": sorted(set(evidence_violations)),
            "realized_evsi": realized_evsi,
            "budget_use": {
                "audio_bytes": total_audio,
                "video_frames": total_video,
                "energy_joules": total_energy
            }
        })

    count = len(results)
    answer_correct_count = sum(row["answer_correct"] for row in results)
    decision_correct_count = sum(row["decision_correct"] for row in results)
    budget_violation_count = sum(len(row["budget_violations"]) for row in results)
    evidence_violation_count = sum(len(row["evidence_violations"]) for row in results)
    total_acquisitions = sum(row["acquisition_count"] for row in results)
    total_wasted = sum(row["wasted_acquisition_count"] for row in results)
    total_late = sum(row["late_acquisition_count"] for row in results)
    regret_total = sum(row["decision_regret"] for row in results)
    total_audio_bytes = sum(row["budget_use"]["audio_bytes"] for row in results)
    total_video_frames = sum(row["budget_use"]["video_frames"] for row in results)
    total_energy_cost = sum(row["budget_use"]["energy_joules"] for row in results)
    realized_mean = sum(row["realized_evsi"] for row in results) / count if count else None
    return {
        "schema_version": 1,
        "benchmark_id": BENCHMARK_ID,
        "policy_id": submission.get("policy_id"),
        "run_id": submission.get("run_id"),
        "interpretation": {
            "status": "synthetic_reference_score",
            "limitations": [
                "The score is computed on eight authored synthetic metadata-only episodes, not real sensor streams or model runs.",
                "Action EVSI and arrival outcomes are evaluator-only scenario labels; candidate estimates are not trusted as realized value.",
                "Logical ticks are not wall-clock latency, and catalogue resource costs are not measured physical energy.",
                "The public truth fixture is not blind or secure; blind evaluation requires a restricted evaluator process and hidden scenario storage.",
                "Passing this suite does not validate camera movement, microphone control, physical feasibility, safety or SOTA superiority."
            ]
        },
        "totals": {
            "episode_count": count,
            "answer_accuracy": answer_correct_count / count if count else None,
            "safe_decision_accuracy": decision_correct_count / count if count else None,
            "coverage": sum(row["answered"] for row in results) / count if count else None,
            "decision_regret_total": regret_total,
            "acquisition_count": total_acquisitions,
            "wasted_acquisition_count": total_wasted,
            "late_acquisition_count": total_late,
            "budget_violation_count": budget_violation_count,
            "evidence_violation_count": evidence_violation_count,
            "mean_realized_evsi": realized_mean,
            "catalogue_audio_bytes_total": total_audio_bytes,
            "catalogue_video_frames_total": total_video_frames,
            "catalogue_energy_cost_total": total_energy_cost
        },
        "per_episode": results
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workload", required=True)
    parser.add_argument("--truth", required=True)
    parser.add_argument("--submission", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        report = evaluate(read_json(args.workload), read_json(args.truth), read_json(args.submission))
        output = Path(args.output)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    except (OSError, json.JSONDecodeError, EvaluationError) as error:
        print(f"active sensing evaluation failed: {error}", file=sys.stderr)
        return 2
    print(f"wrote {output}; episodes={report['totals']['episode_count']} answer_accuracy={report['totals']['answer_accuracy']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())