#!/usr/bin/env python3
"""Deterministic evaluator for long-horizon audiovisual evidence-memory fixtures."""
from __future__ import annotations

import argparse
import json
import math
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

BENCHMARK_ID = "av-long-horizon-evidence-v0"
MEMORY_CONDITIONS = {
    "recent_window", "summary_only", "episodic_retrieval", "evidence_ledger"
}


class EvaluationError(ValueError):
    """The workload, truth or candidate submission is structurally invalid."""


def read_json(path: str | Path) -> dict[str, Any]:
    with Path(path).open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise EvaluationError(f"{path}: root must be an object")
    return value


def index_unique(rows: list[dict[str, Any]], key: str, label: str) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for row in rows:
        identifier = row.get(key)
        if not isinstance(identifier, str) or not identifier:
            raise EvaluationError(f"{label}: missing {key}")
        if identifier in result:
            raise EvaluationError(f"{label}: duplicate {key}: {identifier}")
        result[identifier] = row
    return result


def ratio(numerator: int, denominator: int) -> float | None:
    return numerator / denominator if denominator else None


def _episode_state(
    episode: dict[str, Any], as_of_tick: int
) -> tuple[dict[str, int], set[str], set[str]]:
    """Return all source refs plus supersession/revocation effective by as_of_tick."""
    events = episode.get("stream_events")
    if not isinstance(events, list):
        raise EvaluationError(f"{episode.get('episode_id')}: stream_events must be an array")
    event_ids: set[str] = set()
    ordered: list[dict[str, Any]] = []
    observations: dict[str, int] = {}
    observation_signatures: dict[str, tuple[Any, ...]] = {}
    previous_availability_tick = -1

    for event in events:
        if not isinstance(event, dict):
            raise EvaluationError("stream event must be an object")
        event_id = event.get("event_id")
        if not isinstance(event_id, str) or not event_id or event_id in event_ids:
            raise EvaluationError(f"invalid or duplicate event_id: {event_id!r}")
        event_ids.add(event_id)
        kind = event.get("kind")
        tick = event.get("availability_tick")
        if isinstance(tick, bool) or not isinstance(tick, int) or tick < 0:
            raise EvaluationError(f"{episode.get('episode_id')}: invalid event availability tick")
        if tick < previous_availability_tick:
            raise EvaluationError(f"{episode.get('episode_id')}: stream events are not ordered by availability")
        previous_availability_tick = tick
        ordered.append(event)
        if kind == "observation":
            ref = event.get("observation_ref")
            if not isinstance(ref, str) or not ref:
                raise EvaluationError(f"{episode.get('episode_id')}: observation missing ref")
            modalities = event.get("modalities")
            capture_tick = event.get("capture_tick")
            digest = event.get("artifact_sha256")
            clock_id = event.get("clock_domain_id")
            generation = event.get("clock_generation")
            dependency_group = event.get("source_dependency_group")
            if (
                not isinstance(modalities, list)
                or not modalities
                or any(not isinstance(modality, str) or modality not in {"audio", "video"} for modality in modalities)
                or len(modalities) != len(set(modalities))
                or isinstance(capture_tick, bool)
                or not isinstance(capture_tick, int)
                or capture_tick < 0
                or not isinstance(digest, str)
                or len(digest) != 64
                or any(char not in "0123456789abcdef" for char in digest)
                or not isinstance(clock_id, str)
                or not clock_id
                or isinstance(generation, bool)
                or not isinstance(generation, int)
                or generation < 0
                or (dependency_group is not None and (not isinstance(dependency_group, str) or not dependency_group))
            ):
                raise EvaluationError(f"{episode.get('episode_id')}: malformed immutable observation identity")
            signature = (
                tuple(modalities), capture_tick, digest, clock_id, generation, dependency_group
            )
            if ref in observation_signatures and observation_signatures[ref] != signature:
                raise EvaluationError(f"{episode.get('episode_id')}: duplicate observation reference changed immutable source identity")
            observation_signatures[ref] = signature
            # Duplicate delivery reuses the original immutable source identity; first availability wins.
            observations[ref] = min(observations.get(ref, tick), tick)
        elif not isinstance(kind, str) or kind not in {"correction", "revocation"}:
            raise EvaluationError(f"{episode.get('episode_id')}: unknown event kind {kind!r}")

    superseded: set[str] = set()
    revoked: set[str] = set()
    for event in sorted(ordered, key=lambda item: item["availability_tick"]):
        tick = event["availability_tick"]
        if tick > as_of_tick:
            continue
        if event["kind"] == "correction":
            old_ref = event.get("supersedes_observation_ref")
            new_ref = event.get("replacement_observation_ref")
            if (
                not isinstance(old_ref, str) or not old_ref
                or not isinstance(new_ref, str) or not new_ref
                or old_ref == new_ref
                or old_ref not in observations or new_ref not in observations
                or observations[old_ref] > tick or observations[new_ref] > tick
            ):
                raise EvaluationError(f"{episode.get('episode_id')}: correction ancestry is unresolved")
            superseded.add(old_ref)
        elif event["kind"] == "revocation":
            target = event.get("target_observation_ref")
            if (
                not isinstance(target, str) or not target
                or target not in observations or observations[target] > tick
            ):
                raise EvaluationError(f"{episode.get('episode_id')}: revocation target is unresolved")
            revoked.add(target)
    return observations, superseded, revoked


def _revoked_derived_item_ticks(
    episode: dict[str, Any], as_of_tick: int
) -> dict[str, int]:
    """Return derived items invalidated by source revocation, after validating their DAG."""
    source_revocations: dict[str, int] = {}
    for event in episode.get("stream_events", []):
        if event.get("kind") == "revocation" and event.get("availability_tick", as_of_tick + 1) <= as_of_tick:
            target = event.get("target_observation_ref")
            if isinstance(target, str) and target:
                source_revocations[target] = min(
                    source_revocations.get(target, event["availability_tick"]),
                    event["availability_tick"],
                )

    observations, _, _ = _episode_state(episode, as_of_tick)
    items = episode.get("derived_memory_items", [])
    if not isinstance(items, list):
        raise EvaluationError(f"{episode.get('episode_id')}: derived_memory_items must be an array")
    by_ref: dict[str, dict[str, Any]] = {}
    for item in items:
        if not isinstance(item, dict):
            raise EvaluationError("derived memory item must be an object")
        ref = item.get("item_ref")
        created_tick = item.get("created_tick")
        if not isinstance(ref, str) or not ref or ref in by_ref:
            raise EvaluationError(f"invalid or duplicate derived memory item ref: {ref!r}")
        if isinstance(created_tick, bool) or not isinstance(created_tick, int) or created_tick < 0:
            raise EvaluationError(f"{ref}: invalid derived-memory creation tick")
        source_refs = item.get("derived_from_observation_refs", [])
        parent_refs = item.get("derived_from_item_refs", [])
        if (
            not isinstance(source_refs, list)
            or any(not isinstance(source, str) or not source for source in source_refs)
            or len(source_refs) != len(set(source_refs))
            or not isinstance(parent_refs, list)
            or any(not isinstance(parent, str) or not parent for parent in parent_refs)
            or len(parent_refs) != len(set(parent_refs))
        ):
            raise EvaluationError(f"{ref}: malformed or duplicate derived-memory dependencies")
        by_ref[ref] = item

    # Validate all dependencies and establish a topological order. A cyclic
    # derived-memory graph cannot be given deterministic revocation lineage.
    indegree = {ref: 0 for ref in by_ref}
    children: dict[str, list[str]] = defaultdict(list)
    for ref, item in by_ref.items():
        created_tick = item["created_tick"]
        for source in item["derived_from_observation_refs"]:
            if source not in observations:
                raise EvaluationError(f"{ref}: unknown source observation dependency {source!r}")
            if observations[source] > created_tick:
                raise EvaluationError(f"{ref}: derived item predates its source observation")
        for parent in item["derived_from_item_refs"]:
            if parent not in by_ref:
                raise EvaluationError(f"{ref}: unknown derived-memory parent {parent!r}")
            if by_ref[parent]["created_tick"] > created_tick:
                raise EvaluationError(f"{ref}: derived item predates its parent")
            indegree[ref] += 1
            children[parent].append(ref)

    ready = sorted(ref for ref, degree in indegree.items() if degree == 0)
    topological: list[str] = []
    while ready:
        parent = ready.pop(0)
        topological.append(parent)
        for child in sorted(children[parent]):
            indegree[child] -= 1
            if indegree[child] == 0:
                ready.append(child)
        ready.sort()
    if len(topological) != len(by_ref):
        raise EvaluationError(f"{episode.get('episode_id')}: cyclic derived-memory dependency graph")

    invalidated: dict[str, int] = {}
    for ref in topological:
        item = by_ref[ref]
        ticks = [
            source_revocations[source]
            for source in item["derived_from_observation_refs"]
            if source in source_revocations
        ]
        ticks.extend(
            invalidated[parent]
            for parent in item["derived_from_item_refs"]
            if parent in invalidated
        )
        if ticks:
            invalidated[ref] = min(ticks)
    return invalidated

def _score_metrics(rows: list[dict[str, Any]]) -> dict[str, Any]:
    answered = [row for row in rows if not row["abstained"]]
    count = len(rows)
    answer_correct = sum(row["answer_correct"] for row in rows)
    decision_correct = sum(row["decision_correct"] for row in rows)
    tp = sum(row["citation_tp"] for row in rows)
    fp = sum(row["citation_fp"] for row in rows)
    fn = sum(row["citation_fn"] for row in rows)
    provenance = sum(row["provenance_complete"] for row in rows)
    evidence_qualified = sum(
        row["answer_correct"] and row["provenance_complete"] and row["evidence_compliant"]
        for row in rows
    )
    compliant = sum(row["evidence_compliant"] for row in rows)
    false_memory = sum(row["false_memory"] for row in rows)
    violations = sum(len(row["violations"]) for row in rows)
    deadline_misses = sum(row["deadline_miss"] for row in rows)
    return {
        "episode_count": count,
        "answer_correct_count": answer_correct,
        "decision_correct_count": decision_correct,
        "answer_accuracy": ratio(answer_correct, count),
        "decision_accuracy": ratio(decision_correct, count),
        "coverage": ratio(len(answered), count),
        "selective_accuracy": ratio(sum(row["answer_correct"] for row in answered), len(answered)),
        "citation_precision": ratio(tp, tp + fp),
        "citation_recall": ratio(tp, tp + fn),
        "provenance_complete_count": provenance,
        "provenance_complete_rate": ratio(provenance, count),
        "evidence_qualified_answer_correct_count": evidence_qualified,
        "evidence_qualified_answer_accuracy": ratio(evidence_qualified, count),
        "evidence_compliant_count": compliant,
        "evidence_compliance_rate": ratio(compliant, count),
        "false_memory_count": false_memory,
        "hard_violation_count": violations,
        "deadline_miss_count": deadline_misses,
    }


def evaluate(
    workload: dict[str, Any], truth: dict[str, Any], submission: dict[str, Any]
) -> dict[str, Any]:
    for label, document in (("workload", workload), ("truth", truth), ("submission", submission)):
        if document.get("schema_version") != 1 or document.get("benchmark_id") != BENCHMARK_ID:
            raise EvaluationError(f"{label}: schema version or benchmark identity mismatch")

    workload_rows = workload.get("episodes")
    truth_rows = truth.get("episodes")
    predictions = submission.get("predictions")
    if not all(isinstance(value, list) for value in (workload_rows, truth_rows, predictions)):
        raise EvaluationError("episodes and predictions must be arrays")

    work = index_unique(workload_rows, "episode_id", "workload")
    labels = index_unique(truth_rows, "episode_id", "truth")
    candidate = index_unique(predictions, "episode_id", "submission")
    if set(work) != set(labels) or set(work) != set(candidate):
        raise EvaluationError("workload, truth and submission episode IDs must match exactly")

    condition = submission.get("memory_condition")
    if not isinstance(condition, str) or condition not in MEMORY_CONDITIONS:
        raise EvaluationError(f"unsupported memory_condition: {condition!r}")

    results: list[dict[str, Any]] = []
    by_horizon: dict[str, list[dict[str, Any]]] = defaultdict(list)

    for episode_id in sorted(work):
        episode = work[episode_id]
        label = labels[episode_id]
        prediction = candidate[episode_id]
        query = episode.get("query") or {}
        query_tick = query.get("query_tick")
        deadline = query.get("response_deadline_tick")
        as_of_tick = prediction.get("evidence_as_of_tick")
        response_tick = prediction.get("response_tick")
        declared_decision = prediction.get("decision")
        abstained = declared_decision == "abstain"
        answer = prediction.get("answer")
        confidence = prediction.get("confidence")
        violations: list[str] = []
        if not isinstance(declared_decision, str) or declared_decision not in {"answer", "abstain"}:
            violations.append("invalid_decision")

        if isinstance(query_tick, bool) or not isinstance(query_tick, int):
            raise EvaluationError(f"{episode_id}: invalid query_tick")
        if isinstance(deadline, bool) or not isinstance(deadline, int):
            raise EvaluationError(f"{episode_id}: invalid response deadline")
        if isinstance(as_of_tick, bool) or not isinstance(as_of_tick, int):
            violations.append("invalid_evidence_as_of_tick")
            as_of_tick = -1
        elif as_of_tick != query_tick:
            violations.append("evidence_cutoff_must_equal_query_tick")
        if isinstance(response_tick, bool) or not isinstance(response_tick, int):
            violations.append("invalid_response_tick")
            response_tick = query_tick
        elif response_tick < query_tick:
            violations.append("response_precedes_query")
        deadline_miss = response_tick > deadline

        if isinstance(confidence, bool) or not isinstance(confidence, (int, float)) or not math.isfinite(float(confidence)) or not 0.0 <= float(confidence) <= 1.0:
            violations.append("confidence_outside_finite_unit_interval")
            confidence = 0.0
        else:
            confidence = float(confidence)
        if abstained and answer is not None:
            violations.append("abstention_must_not_include_answer")
        if not abstained and (not isinstance(answer, str) or not answer):
            violations.append("answer_requires_nonempty_text")
        if abstained and confidence != 0.0:
            violations.append("abstention_confidence_must_be_zero")

        citations = prediction.get("cited_observation_refs")
        if not isinstance(citations, list) or any(not isinstance(ref, str) or not ref for ref in citations):
            citations = []
            violations.append("invalid_citation_list")
        if len(citations) != len(set(citations)):
            violations.append("duplicate_observation_reference")

        current_refs = prediction.get("current_observation_refs")
        if not isinstance(current_refs, list) or any(not isinstance(ref, str) or not ref for ref in current_refs):
            current_refs = []
            violations.append("invalid_current_observation_reference_list")
        if len(current_refs) != len(set(current_refs)):
            violations.append("duplicate_current_observation_reference")
        if not set(current_refs) <= set(citations):
            violations.append("current_observation_not_in_citation_set")

        observations, superseded, revoked = _episode_state(episode, max(as_of_tick, 0))
        eligible = {ref for ref, available_tick in observations.items() if available_tick <= as_of_tick} - superseded - revoked
        valid_historical_citations: set[str] = set()
        invalid_cited_refs: set[str] = set()
        revoked_citation = False
        for ref in citations:
            if ref not in observations:
                invalid_cited_refs.add(ref)
                violations.append("citation_not_in_episode_evidence")
            elif observations[ref] > as_of_tick:
                invalid_cited_refs.add(ref)
                violations.append("citation_not_available_at_evidence_cutoff")
            elif ref in revoked:
                invalid_cited_refs.add(ref)
                revoked_citation = True
            else:
                # Superseded evidence remains citable as history; it is not
                # eligible to support the current state.
                valid_historical_citations.add(ref)

        valid_current_refs: set[str] = set()
        for ref in current_refs:
            if ref not in observations:
                violations.append("current_reference_not_in_episode_evidence")
            elif observations[ref] > as_of_tick:
                violations.append("current_reference_not_available_at_evidence_cutoff")
            elif ref in superseded:
                violations.append("current_reference_uses_superseded_observation")
            elif ref in revoked:
                revoked_citation = True
                violations.append("current_reference_uses_revoked_observation")
            else:
                valid_current_refs.add(ref)

        raw_required = label.get("required_observation_refs") or []
        if (
            not isinstance(raw_required, list)
            or any(not isinstance(ref, str) or not ref for ref in raw_required)
            or len(raw_required) != len(set(raw_required))
        ):
            raise EvaluationError(f"{episode_id}: truth contains invalid required observation refs")
        required = set(raw_required)
        ordered_required = label.get("required_ordered_observation_refs")
        sequence_correct = True
        if ordered_required is not None:
            if (
                not isinstance(ordered_required, list)
                or any(not isinstance(ref, str) or not ref for ref in ordered_required)
                or len(ordered_required) != len(set(ordered_required))
                or set(ordered_required) != required
            ):
                raise EvaluationError(f"{episode_id}: ordered truth refs must be a unique permutation of required refs")
            sequence_correct = abstained or current_refs == ordered_required
            if not sequence_correct:
                violations.append("current_observation_order_mismatch")
        if not required <= set(observations):
            raise EvaluationError(f"{episode_id}: truth requires an observation not present in stream")
        if not required <= eligible:
            raise EvaluationError(f"{episode_id}: truth requires superseded or revoked evidence")

        derived_items = {
            item["item_ref"]: item for item in episode.get("derived_memory_items", [])
        }
        known_memory_refs = set(derived_items)
        retrieved_refs = prediction.get("retrieved_memory_item_refs", [])
        retained_refs = prediction.get("retained_memory_item_refs", [])
        purge_receipts = prediction.get("purge_receipts", [])
        for field_name, refs in (
            ("retrieved_memory_item_refs", retrieved_refs),
            ("retained_memory_item_refs", retained_refs),
        ):
            if not isinstance(refs, list) or any(not isinstance(ref, str) or not ref for ref in refs):
                violations.append(f"invalid_{field_name}")
                refs = []
                if field_name == "retrieved_memory_item_refs":
                    retrieved_refs = []
                else:
                    retained_refs = []
            elif len(refs) != len(set(refs)):
                violations.append(f"duplicate_{field_name}")
            for ref in refs:
                if ref not in known_memory_refs:
                    violations.append("unknown_derived_memory_item_reference")
                elif derived_items[ref].get("created_tick", as_of_tick + 1) > as_of_tick:
                    violations.append("derived_memory_item_not_available_at_query")

        invalidated_items = _revoked_derived_item_ticks(episode, max(as_of_tick, 0))
        referenced_invalidated = (
            set(retrieved_refs if isinstance(retrieved_refs, list) else [])
            | set(retained_refs if isinstance(retained_refs, list) else [])
        ) & set(invalidated_items)
        purge_by_ref: dict[str, dict[str, Any]] = {}
        if not isinstance(purge_receipts, list):
            purge_receipts = []
            violations.append("invalid_purge_receipt_list")
        for receipt in purge_receipts:
            if not isinstance(receipt, dict):
                violations.append("invalid_purge_receipt")
                continue
            item_ref = receipt.get("item_ref")
            receipt_id = receipt.get("receipt_id")
            if not isinstance(item_ref, str) or not item_ref or item_ref in purge_by_ref:
                violations.append("invalid_or_duplicate_purge_receipt_item")
                continue
            if not isinstance(receipt_id, str) or not receipt_id:
                violations.append("purge_receipt_missing_identity")
            if receipt.get("attestation_level") != "candidate_self_reported":
                violations.append("purge_receipt_attestation_level_invalid")
            receipt_status = receipt.get("status")
            if not isinstance(receipt_status, str) or receipt_status not in {"purged", "retained"}:
                violations.append("purge_receipt_status_invalid")
            purge_by_ref[item_ref] = receipt

        purge_incomplete = False
        for item_ref, revoked_tick in invalidated_items.items():
            receipt = purge_by_ref.get(item_ref)
            if (
                receipt is None
                or receipt.get("status") != "purged"
                or not isinstance(receipt.get("purge_tick"), int)
                or isinstance(receipt.get("purge_tick"), bool)
                or receipt.get("purge_tick") < revoked_tick
            ):
                purge_incomplete = True
        # One fail-closed violation per episode captures any post-revocation
        # source citation, derived-memory residue or incomplete purge receipt.
        if revoked_citation or referenced_invalidated or purge_incomplete:
            violations.append("revoked_evidence_or_derived_memory_reuse")

        if label.get("independence_claim_allowed") is False and prediction.get("claimed_independent_corroboration") is True:
            violations.append("common_cause_views_claimed_independent")
        if not abstained and not required <= valid_current_refs and not any(
            code in violations for code in (
                "current_reference_uses_superseded_observation",
                "current_reference_not_in_episode_evidence",
                "current_reference_not_available_at_evidence_cutoff",
                "current_reference_uses_revoked_observation",
            )
        ):
            violations.append("required_current_observation_missing")
        if (
            label.get("cross_clock_comparison") == "incomparable_without_qualified_transform"
            and prediction.get("claimed_cross_clock_order") is True
        ):
            violations.append("cross_clock_order_claim_without_qualified_transform")

        expected_decision = label.get("expected_decision")
        expected_answer = label.get("expected_answer")
        if not isinstance(expected_decision, str) or expected_decision not in {"answer", "abstain"}:
            raise EvaluationError(f"{episode_id}: unsupported expected_decision")
        count_correct = (
            label.get("expected_distinct_event_count") is None
            or prediction.get("claimed_distinct_event_count") == label.get("expected_distinct_event_count")
        )
        if expected_decision == "abstain":
            answer_correct = False
            decision_correct = abstained and answer is None
            false_memory = not abstained
        else:
            answer_correct = not abstained and answer == expected_answer and count_correct and sequence_correct
            decision_correct = answer_correct
            false_memory = not abstained and not answer_correct

        tp = len(valid_current_refs & required)
        fp = (
            len(set(current_refs) - (valid_current_refs & required))
            + len(invalid_cited_refs - set(current_refs))
        )
        fn = len(required - valid_current_refs)
        citation_precision = ratio(tp, tp + fp)
        citation_recall = ratio(tp, tp + fn)
        provenance_complete = valid_current_refs == required and not violations
        row = {
            "episode_id": episode_id,
            "horizon_seconds": episode.get("horizon_seconds", 0),
            "answer_correct": bool(answer_correct),
            "decision_correct": bool(decision_correct),
            "abstained": bool(abstained),
            "confidence": confidence,
            "citation_precision": citation_precision,
            "citation_recall": citation_recall,
            "provenance_complete": bool(provenance_complete),
            "evidence_qualified_answer_correct": bool(answer_correct and provenance_complete and not violations),
            "evidence_compliant": not violations,
            "violations": sorted(set(violations)),
            "false_memory": bool(false_memory),
            "deadline_miss": bool(deadline_miss),
            "citation_tp": tp,
            "citation_fp": fp,
            "citation_fn": fn,
        }
        results.append(row)
        by_horizon[str(row["horizon_seconds"])].append(row)

    totals = _score_metrics(results)
    per_horizon = {key: _score_metrics(rows) for key, rows in sorted(by_horizon.items(), key=lambda item: int(item[0]))}
    public_episode_rows = [
        {key: value for key, value in row.items() if not key.startswith("citation_") or key in {"citation_precision", "citation_recall"}}
        for row in results
    ]
    return {
        "schema_version": 1,
        "benchmark_id": BENCHMARK_ID,
        "policy_id": submission.get("policy_id"),
        "run_id": submission.get("run_id"),
        "memory_condition": condition,
        "interpretation": {
            "status": "synthetic_reference_score",
            "limitations": [
                "This score is computed on nine authored synthetic metadata-only episodes, not model runs or external media.",
                "Artifact digests are placeholders; no PCM/RGB bytes or physical sensor qualification is present.",
                "The public evaluator-truth file is not a blind holdout or security boundary.",
                "Memory bytes, tokens, latency and energy are not inferred from logical replay ticks and are not claimed by this reference submission.",
                "A complete provenance score is a source-citation contract check, not proof that a model internally used the cited evidence.",
                "Purge receipts are candidate-self-reported claims; they do not independently prove deletion from live storage, indexes, caches, backups or external providers."
            ]
        },
        "totals": totals,
        "per_horizon_seconds": per_horizon,
        "hard_violations": [
            {"episode_id": row["episode_id"], "violations": row["violations"]}
            for row in results if row["violations"]
        ],
        "per_episode": public_episode_rows,
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
        print(f"long-horizon evaluation failed: {error}", file=sys.stderr)
        return 2
    print(
        f"wrote {output}; episodes={report['totals']['episode_count']} "
        f"answers={report['totals']['answer_correct_count']} "
        f"violations={report['totals']['hard_violation_count']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
