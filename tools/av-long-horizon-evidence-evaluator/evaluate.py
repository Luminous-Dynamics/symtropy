#!/usr/bin/env python3
"""Deterministic evaluator for long-horizon audiovisual evidence-memory submissions.

Research/benchmark tooling only. This is not a production memory engine,
retrieval service, privacy enforcement mechanism, or safety function.
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
BENCHMARK_ID = "av-long-horizon-evidence-v0"
MEMORY_CONDITIONS = {
    "recent_window",
    "summary_only",
    "episodic_retrieval",
    "evidence_ledger",
}


class EvaluationError(ValueError):
    """Raised when the evaluation inputs are structurally invalid."""


def read_json(path: str | Path) -> dict[str, Any]:
    with Path(path).open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise EvaluationError(f"{path}: root must be an object")
    return value


def ratio(numerator: int, denominator: int) -> float | None:
    return numerator / denominator if denominator else None


def index_unique(rows: list[dict[str, Any]], key: str) -> dict[str, dict[str, Any]]:
    indexed: dict[str, dict[str, Any]] = {}
    for row in rows:
        identifier = row.get(key)
        if not isinstance(identifier, str) or not identifier:
            raise EvaluationError(f"missing or invalid {key}")
        if identifier in indexed:
            raise EvaluationError(f"duplicate {key}: {identifier}")
        indexed[identifier] = row
    return indexed


def _event_index(episode: dict[str, Any]) -> tuple[
    dict[str, dict[str, Any]],
    dict[str, list[str]],
    dict[str, int],
    dict[str, int],
    dict[str, int],
]:
    """Return unique observations, observation->event IDs, first delivery ticks,
    revocation ticks, and superseded refs at their effective ticks.
    """
    observations: dict[str, dict[str, Any]] = {}
    delivery_event_ids: dict[str, list[str]] = defaultdict(list)
    first_available_tick: dict[str, int] = {}
    revoked_at: dict[str, int] = {}
    superseded_at: dict[str, int] = {}
    seen_event_ids: set[str] = set()

    events = episode.get("stream_events")
    if not isinstance(events, list):
        raise EvaluationError(f"{episode.get('episode_id')}: stream_events must be an array")

    previous_tick = -1
    for event in events:
        event_id = event.get("event_id")
        kind = event.get("kind")
        availability_tick = event.get("availability_tick")
        if not isinstance(event_id, str) or not event_id:
            raise EvaluationError("stream event has missing event_id")
        if event_id in seen_event_ids:
            raise EvaluationError(f"duplicate event_id: {event_id}")
        seen_event_ids.add(event_id)
        if not isinstance(availability_tick, int) or isinstance(availability_tick, bool) or availability_tick < previous_tick:
            raise EvaluationError(f"{episode.get('episode_id')}: event availability is not monotonic")
        previous_tick = availability_tick

        if kind == "observation":
            observation_ref = event.get("observation_ref")
            if not isinstance(observation_ref, str) or not observation_ref:
                raise EvaluationError(f"{event_id}: observation_ref is missing")
            identity = {
                "artifact_sha256": event.get("artifact_sha256"),
                "capture_tick": event.get("capture_tick"),
                "modalities": tuple(sorted(event.get("modalities") or [])),
                "source_dependency_group": event.get("source_dependency_group"),
                "clock_domain_id": event.get("clock_domain_id"),
                "clock_generation": event.get("clock_generation"),
            }
            if observation_ref in observations:
                if observations[observation_ref] != identity:
                    raise EvaluationError(
                        f"{episode.get('episode_id')}: duplicate delivery changed source identity for {observation_ref}"
                    )
            else:
                observations[observation_ref] = identity
                first_available_tick[observation_ref] = availability_tick
            delivery_event_ids[observation_ref].append(event_id)
        elif kind == "correction":
            superseded_ref = event.get("supersedes_observation_ref")
            replacement_ref = event.get("replacement_observation_ref")
            if superseded_ref not in observations or replacement_ref not in observations:
                raise EvaluationError(f"{event_id}: correction references an unknown observation")
            superseded_at[superseded_ref] = min(
                superseded_at.get(superseded_ref, availability_tick), availability_tick
            )
        elif kind == "revocation":
            target_ref = event.get("target_observation_ref")
            if target_ref not in observations:
                raise EvaluationError(f"{event_id}: revocation references an unknown observation")
            revoked_at[target_ref] = min(
                revoked_at.get(target_ref, availability_tick), availability_tick
            )
        else:
            raise EvaluationError(f"{event_id}: unsupported stream event kind {kind!r}")

    # The event stream is availability-ordered. These maps preserve the exact
    # tick at which a correction or revocation becomes effective.
    return observations, delivery_event_ids, first_available_tick, revoked_at, superseded_at


def _derived_memory_revocation_state(
    episode: dict[str, Any],
    observations: dict[str, dict[str, Any]],
    revoked_at: dict[str, int],
    query_tick: int,
) -> tuple[dict[str, dict[str, Any]], dict[str, int], set[str]]:
    """Compute transitive invalidation of memory items from revoked observations.

    An item's invalidation time is the earliest effective revocation among its
    direct source observations and its transitive parent memory items.
    """
    raw_items = episode.get("derived_memory_items", [])
    if not isinstance(raw_items, list):
        raise EvaluationError(f"{episode.get('episode_id')}: derived_memory_items must be an array")

    items: dict[str, dict[str, Any]] = {}
    for item in raw_items:
        if not isinstance(item, dict):
            raise EvaluationError("derived memory item must be an object")
        item_ref = item.get("item_ref")
        created_tick = item.get("created_tick")
        if not isinstance(item_ref, str) or not item_ref:
            raise EvaluationError("derived memory item has missing item_ref")
        if item_ref in items:
            raise EvaluationError(f"duplicate derived memory item: {item_ref}")
        if isinstance(created_tick, bool) or not isinstance(created_tick, int) or created_tick < 0:
            raise EvaluationError(f"{item_ref}: invalid created_tick")
        if created_tick > query_tick:
            raise EvaluationError(f"{item_ref}: workload exposes a future-created memory item")
        source_refs = item.get("derived_from_observation_refs", [])
        parent_refs = item.get("derived_from_item_refs", [])
        if (
            not isinstance(source_refs, list)
            or any(not isinstance(ref, str) or not ref for ref in source_refs)
            or len(source_refs) != len(set(source_refs))
        ):
            raise EvaluationError(f"{item_ref}: invalid derived_from_observation_refs")
        if (
            not isinstance(parent_refs, list)
            or any(not isinstance(ref, str) or not ref for ref in parent_refs)
            or len(parent_refs) != len(set(parent_refs))
        ):
            raise EvaluationError(f"{item_ref}: invalid derived_from_item_refs")
        items[item_ref] = item

    for item_ref, item in items.items():
        for source_ref in item.get("derived_from_observation_refs", []):
            if source_ref not in observations:
                raise EvaluationError(f"{item_ref}: unknown source observation {source_ref}")
        for parent_ref in item.get("derived_from_item_refs", []):
            if parent_ref not in items:
                raise EvaluationError(f"{item_ref}: unknown parent memory item {parent_ref}")
            if items[parent_ref]["created_tick"] > item["created_tick"]:
                raise EvaluationError(f"{item_ref}: derived memory parent is created later than child")

    memo: dict[str, int | None] = {}
    visiting: set[str] = set()
    created_after_revocation: set[str] = set()

    def invalidated_at(item_ref: str) -> int | None:
        if item_ref in memo:
            return memo[item_ref]
        if item_ref in visiting:
            raise EvaluationError(f"derived memory dependency cycle at {item_ref}")
        visiting.add(item_ref)
        item = items[item_ref]
        causes = [
            revoked_at[source_ref]
            for source_ref in item.get("derived_from_observation_refs", [])
            if source_ref in revoked_at and revoked_at[source_ref] <= query_tick
        ]
        for parent_ref in item.get("derived_from_item_refs", []):
            parent_tick = invalidated_at(parent_ref)
            if parent_tick is not None:
                causes.append(parent_tick)
        result = min(causes) if causes else None
        if result is not None and item["created_tick"] > result:
            created_after_revocation.add(item_ref)
        visiting.remove(item_ref)
        memo[item_ref] = result
        return result

    affected = {
        item_ref: tick
        for item_ref in items
        if (tick := invalidated_at(item_ref)) is not None
    }
    return items, affected, created_after_revocation


def _metric(rows: list[dict[str, Any]]) -> dict[str, Any]:
    answered = [row for row in rows if not row["abstained"]]
    answer_correct = sum(bool(row["answer_correct"]) for row in rows)
    decision_correct = sum(bool(row["decision_correct"]) for row in rows)
    citations_complete = sum(bool(row["citations_complete"]) for row in rows)
    citation_precision_values = [row["citation_precision"] for row in rows]
    revocation_affected = sum(row["revocation_affected_item_count"] for row in rows)
    revocation_purged = sum(row["revocation_purged_item_count"] for row in rows)
    revocation_episodes = [row for row in rows if row["revocation_affected_item_count"] > 0]
    revocation_compliant_episodes = sum(bool(row["revocation_compliant"]) for row in revocation_episodes)
    return {
        "episode_count": len(rows),
        "answer_correct_count": answer_correct,
        "answer_accuracy": ratio(answer_correct, len(rows)),
        "answered_count": len(answered),
        "coverage": ratio(len(answered), len(rows)),
        "selective_accuracy": ratio(
            sum(bool(row["answer_correct"]) for row in answered), len(answered)
        ),
        "decision_correct_count": decision_correct,
        "safe_decision_accuracy": ratio(decision_correct, len(rows)),
        "citations_complete_count": citations_complete,
        "citation_completeness_rate": ratio(citations_complete, len(rows)),
        "mean_citation_precision": ratio(
            sum(citation_precision_values), len(citation_precision_values)
        ),
        "revocation_affected_item_count": revocation_affected,
        "revocation_purged_item_count": revocation_purged,
        "revocation_compliance_rate": ratio(revocation_purged, revocation_affected),
        "revocation_affected_episode_count": len(revocation_episodes),
        "revocation_compliant_episode_count": revocation_compliant_episodes,
        "revocation_episode_compliance_rate": ratio(
            revocation_compliant_episodes, len(revocation_episodes)
        ),
        "transitive_revocation_violation_count": sum(
            len(row["revocation_violations"]) for row in rows
        ),
        "hard_violation_count": sum(len(row["violations"]) for row in rows),
        "deadline_miss_count": sum(bool(row["deadline_miss"]) for row in rows),
    }


def evaluate(
    workload: dict[str, Any],
    truth: dict[str, Any],
    submission: dict[str, Any],
) -> dict[str, Any]:
    """Score one memory-condition submission against hidden evaluator truth."""
    for label, document in (("workload", workload), ("truth", truth), ("submission", submission)):
        if document.get("schema_version") != SCHEMA_VERSION:
            raise EvaluationError(f"{label}: unsupported schema_version")
        if document.get("benchmark_id") != BENCHMARK_ID:
            raise EvaluationError(f"{label}: benchmark_id mismatch")

    if submission.get("memory_condition") not in MEMORY_CONDITIONS:
        raise EvaluationError("submission has unsupported memory_condition")

    workload_rows = workload.get("episodes")
    truth_rows = truth.get("episodes")
    prediction_rows = submission.get("predictions")
    if not all(isinstance(rows, list) for rows in (workload_rows, truth_rows, prediction_rows)):
        raise EvaluationError("episodes and predictions must be arrays")
    work = index_unique(workload_rows, "episode_id")
    expected = index_unique(truth_rows, "episode_id")
    predictions = index_unique(prediction_rows, "episode_id")
    episode_ids = set(work)
    if set(expected) != episode_ids:
        raise EvaluationError("workload/truth episode IDs do not match exactly")
    if set(predictions) != episode_ids:
        missing = sorted(episode_ids - set(predictions))
        extra = sorted(set(predictions) - episode_ids)
        raise EvaluationError(f"submission episode IDs differ; missing={missing}, extra={extra}")

    per_episode: list[dict[str, Any]] = []
    for episode_id in sorted(episode_ids):
        episode = work[episode_id]
        label = expected[episode_id]
        prediction = predictions[episode_id]
        query = episode.get("query") or {}
        query_tick = query.get("query_tick")
        deadline = query.get("response_deadline_tick")
        if not isinstance(query_tick, int) or isinstance(query_tick, bool):
            raise EvaluationError(f"{episode_id}: invalid query_tick")
        observations, delivery_event_ids, first_available, revoked_at, superseded_at = _event_index(episode)

        violations: list[str] = []
        decision = prediction.get("decision")
        abstained = decision == "abstain"
        answer = prediction.get("answer")
        confidence = prediction.get("confidence")
        if decision not in {"answer", "abstain"}:
            violations.append("invalid_decision")
        if abstained and answer is not None:
            violations.append("abstention_must_not_include_answer")
        if not abstained and (not isinstance(answer, str) or not answer):
            violations.append("answer_decision_requires_nonempty_answer")
        if isinstance(confidence, bool) or not isinstance(confidence, (int, float)) or not math.isfinite(float(confidence)) or not 0.0 <= float(confidence) <= 1.0:
            violations.append("confidence_outside_finite_unit_interval")
            confidence = 0.0
        else:
            confidence = float(confidence)
        if abstained and confidence != 0.0:
            violations.append("abstention_confidence_must_be_zero")

        as_of_tick = prediction.get("evidence_as_of_tick")
        response_tick = prediction.get("response_tick")
        if not isinstance(as_of_tick, int) or isinstance(as_of_tick, bool):
            violations.append("invalid_evidence_as_of_tick")
        elif as_of_tick != query_tick:
            violations.append("evidence_cutoff_must_equal_query_tick")
        if not isinstance(response_tick, int) or isinstance(response_tick, bool):
            violations.append("invalid_response_tick")
        else:
            if response_tick < query_tick:
                violations.append("response_precedes_query")
            if isinstance(as_of_tick, int) and response_tick < as_of_tick:
                violations.append("response_precedes_evidence_cutoff")
        deadline_miss = isinstance(deadline, int) and isinstance(response_tick, int) and response_tick > deadline

        cited = prediction.get("cited_observation_refs")
        current = prediction.get("current_observation_refs")
        if not isinstance(cited, list) or any(not isinstance(x, str) for x in cited):
            cited = []
            violations.append("invalid_cited_observation_refs")
        if not isinstance(current, list) or any(not isinstance(x, str) for x in current):
            current = []
            violations.append("invalid_current_observation_refs")
        if len(set(cited)) != len(cited):
            violations.append("duplicate_observation_citation")
        if len(set(current)) != len(current):
            violations.append("duplicate_current_observation_reference")
        if any(ref not in cited for ref in current):
            violations.append("current_observation_not_cited")

        as_of_valid = isinstance(as_of_tick, int) and not isinstance(as_of_tick, bool)
        for ref in cited:
            if ref not in observations:
                violations.append("cited_observation_unknown")
                continue
            if not as_of_valid or first_available[ref] > as_of_tick:
                violations.append("future_observation_cited")
            if ref in revoked_at and as_of_valid and revoked_at[ref] <= as_of_tick:
                violations.append("revoked_observation_cited")
        for ref in current:
            correction_tick = superseded_at.get(ref)
            if isinstance(correction_tick, int) and as_of_valid and correction_tick <= as_of_tick:
                violations.append("superseded_observation_claimed_current")

        required_refs = label.get("required_observation_refs") or []
        required_set = set(required_refs)
        cited_set = set(cited)
        citations_complete = required_set <= cited_set
        citation_precision = (
            len(required_set & cited_set) / len(cited_set) if cited_set else
            (1.0 if not required_set else 0.0)
        )
        if not citations_complete and label.get("expected_decision") == "answer":
            violations.append("required_source_evidence_missing")

        revoked_refs = set(label.get("revoked_refs") or [])
        if revoked_refs & cited_set:
            violations.append("evaluator_revoked_source_cited")
        superseded_truth_refs = set(label.get("superseded_refs") or [])
        if superseded_truth_refs & set(current):
            violations.append("evaluator_superseded_source_claimed_current")

        if label.get("independence_claim_allowed") is False and prediction.get("claimed_independent_corroboration") is True:
            violations.append("common_cause_views_claimed_as_independent")
        if (
            label.get("cross_clock_comparison") == "incomparable_without_qualified_transform"
            and prediction.get("claimed_cross_clock_order") is True
        ):
            violations.append("incomparable_clocks_claimed_as_ordered")

        claimed_count = prediction.get("claimed_distinct_event_count")
        if claimed_count is not None:
            if isinstance(claimed_count, bool) or not isinstance(claimed_count, int) or claimed_count < 0:
                violations.append("invalid_claimed_distinct_event_count")
            else:
                distinct_count = len(set(delivery_event_ids.keys()))
                if episode_id == "lh-002-repeated-event-dedup" and claimed_count != distinct_count:
                    violations.append("duplicate_transport_inflated_distinct_event_count")

        expected_decision = label.get("expected_decision")
        expected_answer = label.get("expected_answer")
        answer_correct = (not abstained) and answer == expected_answer and expected_decision == "answer"
        if expected_decision == "abstain":
            decision_correct = abstained and answer is None
            if cited:
                decision_correct = False
                violations.append("absent_event_abstention_must_not_invent_source")
        else:
            decision_correct = answer_correct

        per_episode.append({
            "episode_id": episode_id,
            "memory_condition": submission["memory_condition"],
            "answer_correct": bool(answer_correct),
            "decision_correct": bool(decision_correct),
            "abstained": abstained,
            "confidence": confidence,
            "citations_complete": bool(citations_complete),
            "citation_precision": citation_precision,
            "violations": sorted(set(violations)),
            "deadline_miss": bool(deadline_miss),
        })

    totals = _metric(per_episode)
    return {
        "schema_version": 1,
        "benchmark_id": BENCHMARK_ID,
        "policy_id": submission.get("policy_id"),
        "run_id": submission.get("run_id"),
        "memory_condition": submission["memory_condition"],
        "interpretation": {
            "status": "offline_candidate_submission_score",
            "limitations": [
                "The workload contains synthetic metadata and placeholder content digests, not raw PCM or RGB evidence.",
                "The reference policy and authored labels validate evaluator semantics only; they are not model results.",
                "Correct source citations do not prove internal memory representation, attention or causal feature use.",
                "The public evaluator-only truth fixture is not a blind test set or security boundary.",
                "Memory size, retrieval latency and model costs are not inferred unless measured telemetry is supplied by a future run contract."
            ],
        },
        "totals": totals,
        "per_episode": per_episode,
        "hard_violations": [
            {"episode_id": row["episode_id"], "violations": row["violations"]}
            for row in per_episode if row["violations"]
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workload", required=True)
    parser.add_argument("--truth", required=True, help="Evaluator-only truth; never mount into the candidate process")
    parser.add_argument("--submission", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        report = evaluate(read_json(args.workload), read_json(args.truth), read_json(args.submission))
        output = Path(args.output)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    except (OSError, json.JSONDecodeError, EvaluationError) as error:
        print(f"evaluation failed: {error}", file=sys.stderr)
        return 2
    print(
        f"wrote {output}; episodes={report['totals']['episode_count']} "
        f"answer_correct={report['totals']['answer_correct_count']} "
        f"decision_correct={report['totals']['decision_correct_count']} "
        f"hard_violations={report['totals']['hard_violation_count']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())