#!/usr/bin/env python3
"""Deterministic offline evaluator; research tooling, not production AV runtime."""
from __future__ import annotations
import argparse
import json
import math
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

SCHEMA_VERSION = 1
BENCHMARK_ID = "av-modality-necessity-v0"
VALID_MODALITIES = {"audio", "video"}
NECESSARY_MODALITIES = {
    "audio_necessary": {"audio"},
    "non_speech_audio_required": {"audio"},
    "video_necessary": {"video"},
    "jointly_necessary": {"audio", "video"},
    "dependent_redundant_views": {"audio", "video"},
    "conflict_abstain_or_separate_hypotheses": {"audio", "video"},
    "no_cross_modal_association": {"audio", "video"},
    "nuisance_invariant_pair": {"audio", "video"},
}


class EvaluationError(ValueError):
    """Evaluation inputs are structurally invalid."""


def read_json(path: str | Path) -> dict[str, Any]:
    with Path(path).open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise EvaluationError(f"{path}: root must be an object")
    return value


def ratio(numerator: int, denominator: int) -> float | None:
    return numerator / denominator if denominator else None


def metric(rows: list[dict[str, Any]]) -> dict[str, Any]:
    answered = [row for row in rows if not row["abstained"]]
    correct = sum(bool(row["decision_correct"]) for row in rows)
    answered_correct = sum(bool(row["answer_correct"]) for row in answered)
    brier = [(row["confidence"] - (1.0 if row["answer_correct"] else 0.0)) ** 2 for row in answered]
    compliant = sum(bool(row["evidence_compliant"]) for row in rows)
    return {
        "episode_count": len(rows), "decision_correct_count": correct,
        "answered_count": len(answered), "answered_correct_count": answered_correct,
        "decision_accuracy": ratio(correct, len(rows)),
        "coverage": ratio(len(answered), len(rows)),
        "selective_accuracy": ratio(answered_correct, len(answered)),
        "answered_brier_score": ratio(sum(brier), len(brier)),
        "evidence_compliant_count": compliant,
        "evidence_compliance_rate": ratio(compliant, len(rows)),
        "hard_violation_count": sum(len(row["violations"]) for row in rows),
        "deadline_miss_count": sum(bool(row["deadline_miss"]) for row in rows),
    }


def index_unique(rows: list[dict[str, Any]], key: str) -> dict[str, dict[str, Any]]:
    indexed: dict[str, dict[str, Any]] = {}
    for row in rows:
        identifier = row.get(key)
        if not isinstance(identifier, str) or not identifier:
            raise EvaluationError(f"missing {key}")
        if identifier in indexed:
            raise EvaluationError(f"duplicate {key}: {identifier}")
        indexed[identifier] = row
    return indexed


def evaluate(
    workload: dict[str, Any],
    truth: dict[str, Any],
    submission: dict[str, Any],
    *,
    permitted_modalities: set[str] | None = None,
) -> dict[str, Any]:
    """Score a candidate submission against a public workload and evaluator-only truth.

    permitted_modalities is the experiment's actual input mask, not the
    modalities the candidate chooses to cite or claim it used.
    """
    if permitted_modalities is None:
        permitted_modalities = set(VALID_MODALITIES)
    if not permitted_modalities <= VALID_MODALITIES:
        raise EvaluationError("permitted_modalities contains an unknown modality")
    for label, document in (("workload", workload), ("truth", truth), ("submission", submission)):
        if document.get("schema_version") != SCHEMA_VERSION:
            raise EvaluationError(f"{label}: unsupported schema_version")
        if document.get("benchmark_id") != BENCHMARK_ID:
            raise EvaluationError(f"{label}: benchmark_id mismatch")
    work_rows, truth_rows, pred_rows = workload.get("episodes"), truth.get("episodes"), submission.get("predictions")
    if not all(isinstance(value, list) for value in (work_rows, truth_rows, pred_rows)):
        raise EvaluationError("episodes/predictions must be arrays")
    work = index_unique(work_rows, "episode_id")
    expected = index_unique(truth_rows, "episode_id")
    predictions = index_unique(pred_rows, "episode_id")
    identifiers = set(work)
    if set(expected) != identifiers:
        raise EvaluationError("workload/truth episode IDs do not match exactly")
    if set(predictions) != identifiers:
        missing, extra = sorted(identifiers - set(predictions)), sorted(set(predictions) - identifiers)
        raise EvaluationError(f"submission episode IDs differ; missing={missing}, extra={extra}")

    results: list[dict[str, Any]] = []
    pairs: dict[str, list[dict[str, Any]]] = defaultdict(list)
    strata: dict[str, list[dict[str, Any]]] = defaultdict(list)

    for episode_id in sorted(identifiers):
        episode, label, pred = work[episode_id], expected[episode_id], predictions[episode_id]
        task, availability = episode.get("task") or {}, episode.get("availability") or {}
        video, audio = episode.get("video"), episode.get("audio")
        violations: list[str] = []

        used_list = pred.get("used_modalities", [])
        if not isinstance(used_list, list) or any(not isinstance(x, str) for x in used_list):
            used_list = []
            violations.append("invalid_used_modalities")
        used = set(used_list)
        if len(used) != len(used_list):
            violations.append("duplicate_used_modalities")
        if not used <= VALID_MODALITIES:
            violations.append("unknown_modality_declared")
            used &= VALID_MODALITIES

        refs = pred.get("cited_observation_refs", [])
        if not isinstance(refs, list) or any(not isinstance(x, str) for x in refs):
            refs = []
            violations.append("invalid_observation_references")
        if len(set(refs)) != len(refs):
            violations.append("duplicate_observation_references")

        # Citations are episode-scoped and must have arrived by the evidence cutoff.
        admitted: dict[str, tuple[str, Any]] = {}
        if "video" in permitted_modalities and isinstance(video, dict) and isinstance(video.get("observation_ref"), str):
            admitted[video["observation_ref"]] = ("video", availability.get("video_tick"))
        if "audio" in permitted_modalities and isinstance(audio, dict) and isinstance(audio.get("observation_ref"), str):
            admitted[audio["observation_ref"]] = ("audio", availability.get("audio_tick"))
        if any(ref not in admitted for ref in refs):
            violations.append("cited_observation_not_admitted_for_episode")

        cited_modalities: set[str] = set()
        cited_groups: list[str] = []
        for ref in refs:
            if ref not in admitted:
                continue
            modality, available_tick = admitted[ref]
            cited_modalities.add(modality)
            cutoff_tick = pred.get("evidence_as_of_tick")
            if not isinstance(available_tick, int):
                violations.append(f"{modality}_availability_unknown")
            elif not isinstance(cutoff_tick, int) or cutoff_tick < available_tick:
                violations.append(f"{modality}_not_available_at_evidence_cutoff")
            observation = video if modality == "video" else audio
            group = observation.get("source_dependency_group") if isinstance(observation, dict) else None
            if isinstance(group, str) and group:
                cited_groups.append(group)
        if cited_modalities != used:
            violations.append("declared_modality_use_does_not_match_cited_observations")

        cutoff, response_tick = pred.get("evidence_as_of_tick"), pred.get("response_tick")
        query_cutoff, deadline = task.get("query_cutoff_tick"), task.get("deadline_tick")
        if not isinstance(cutoff, int) or not isinstance(response_tick, int):
            violations.append("missing_or_invalid_evidence_or_response_tick")
        else:
            if isinstance(query_cutoff, int) and cutoff > query_cutoff:
                violations.append("evidence_cutoff_exceeds_query_cutoff")
            if response_tick < cutoff:
                violations.append("response_precedes_declared_evidence_cutoff")
        deadline_miss = isinstance(deadline, int) and isinstance(response_tick, int) and response_tick > deadline

        if pred.get("used_transcript", False) and (episode.get("audio_profile") or {}).get("transcript_available") is False:
            violations.append("transcript_used_when_unavailable")

        necessity = label.get("necessity", "unknown")
        required = NECESSARY_MODALITIES.get(necessity, set())
        abstained, answer = pred.get("abstained") is True, pred.get("answer")
        confidence = pred.get("confidence")
        if isinstance(confidence, bool) or not isinstance(confidence, (int, float)) or not math.isfinite(float(confidence)) or not 0 <= float(confidence) <= 1:
            violations.append("confidence_outside_finite_unit_interval")
            confidence = 0.0
        else:
            confidence = float(confidence)
        if abstained and answer is not None:
            violations.append("abstention_must_not_include_answer")
        if not abstained and (not isinstance(answer, str) or not answer):
            violations.append("answered_prediction_requires_nonempty_answer")
        if abstained and confidence != 0.0:
            violations.append("abstention_confidence_must_be_zero")
        if not abstained and not required <= used:
            violations.append("necessary_modality_not_declared_as_used")

        if necessity == "abstain_audio_missing":
            if not abstained:
                violations.append("missing_necessary_audio_requires_abstention")
            if "audio" in used:
                violations.append("missing_audio_episode_cannot_claim_audio_use")
        if necessity in {"conflict_abstain_or_separate_hypotheses", "no_cross_modal_association"}:
            if not abstained and not {"audio", "video"} <= used:
                violations.append("cross_modal_task_answered_without_both_modalities")
        if necessity == "dependent_redundant_views":
            if pred.get("claimed_independent_corroboration") is True:
                violations.append("common_cause_views_claimed_as_independent_corroboration")
            shared = label.get("shared_dependency_group")
            if shared and len(cited_groups) >= 2 and set(cited_groups) == {shared} and pred.get("claimed_independent_corroboration") is True:
                violations.append("shared_dependency_group_double_counted")
        if necessity in {"conflict_abstain_or_separate_hypotheses", "no_cross_modal_association"} and pred.get("claimed_cross_modal_association") is True:
            violations.append("cross_modal_association_claimed_under_conflict_or_misalignment")

        cutoff_for_eligibility = pred.get("evidence_as_of_tick")
        eligible_modalities: set[str] = set()
        if isinstance(cutoff_for_eligibility, int) and not isinstance(cutoff_for_eligibility, bool):
            for modality, observation in (("audio", audio), ("video", video)):
                available_tick = availability.get(f"{modality}_tick")
                if (
                    modality in permitted_modalities
                    and isinstance(observation, dict)
                    and isinstance(available_tick, int)
                    and not isinstance(available_tick, bool)
                    and available_tick <= cutoff_for_eligibility
                ):
                    eligible_modalities.add(modality)

        if necessity == "abstain_audio_missing":
            decision_correct, answer_correct = abstained, False
        else:
            answer_correct = (not abstained) and answer == label.get("truth_answer")
            missing_required_modality = bool(required) and not required <= eligible_modalities
            safe_abstention = abstained and (
                missing_required_modality
                or necessity in {
                    "conflict_abstain_or_separate_hypotheses",
                    "no_cross_modal_association",
                }
            )
            decision_correct = answer_correct or safe_abstention
        row = {
            "episode_id": episode_id, "necessity": necessity,
            "answer_correct": answer_correct, "decision_correct": decision_correct,
            "abstained": abstained, "confidence": confidence,
            "evidence_compliant": not violations, "violations": sorted(set(violations)),
            "deadline_miss": bool(deadline_miss),
        }
        results.append(row)
        strata[necessity].append(row)
        pair_id = episode.get("pair_id")
        if isinstance(pair_id, str) and pair_id:
            pairs[pair_id].append({"result": row, "truth_answer": label.get("truth_answer"), "answer": answer})

    paired = []
    for pair_id in sorted(pairs):
        rows = pairs[pair_id]
        truth_answers = {row["truth_answer"] for row in rows}
        predicted_answers = {row["answer"] for row in rows if isinstance(row["answer"], str)}
        required_discrimination = len(truth_answers) > 1
        observed_discrimination = (
            len(predicted_answers) > 1 and all(isinstance(row["answer"], str) for row in rows)
        ) if required_discrimination else True
        paired.append({
            "pair_id": pair_id, "episode_ids": [row["result"]["episode_id"] for row in rows],
            "episode_count": len(rows),
            "all_decisions_correct": all(row["result"]["decision_correct"] for row in rows),
            "counterfactual_discrimination_required": required_discrimination,
            "counterfactual_discrimination_observed": observed_discrimination,
        })

    return {
        "schema_version": SCHEMA_VERSION, "benchmark_id": BENCHMARK_ID,
        "policy_id": submission.get("policy_id"), "run_id": submission.get("run_id"),
        "interpretation": {
            "status": "offline_candidate_submission_score",
            "limitations": [
                "The supplied workload contains synthetic metadata and placeholder commitments, not raw PCM or RGB evidence.",
                "Modality use is candidate-declared and checked against cited observation references; it is not proof of internal model attention or execution.",
                "Passing this finite fixture does not establish real-world AV accuracy, causal feature use, sensor validity, privacy compliance, or SOTA superiority.",
                "Latency, energy, token, frame and byte budgets are not inferred from simulation ticks; they require separate measured run telemetry."
            ]
        },
        "totals": metric(results),
        "per_necessity": {name: metric(rows) for name, rows in sorted(strata.items())},
        "paired_groups": paired,
        "hard_violations": [
            {"episode_id": row["episode_id"], "violations": row["violations"]}
            for row in results if row["violations"]
        ],
        "per_episode": results
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workload", required=True)
    parser.add_argument("--truth", required=True, help="Evaluator-only truth; never mount this into candidate process")
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
    print(f"wrote {output}; episodes={report['totals']['episode_count']} hard_violations={report['totals']['hard_violation_count']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())