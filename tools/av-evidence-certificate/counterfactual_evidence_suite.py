#!/usr/bin/env python3
"""Counterfactual robustness harness for claim-level AV evidence certificates.

The harness reuses the claim scorer in evaluate_evidence_certificate.py.
It does not generate candidate answers: every intervention bundle must contain
an independently produced candidate submission bound to the changed transcript.

Supported interventions:
- REMOVE_IRRELEVANT: remove observations outside every minimal sufficient set;
  the grounded answer must remain invariant.
- REMOVE_ALL_SUFFICIENT: remove at least one member from every alternative
  sufficient evidence set; the candidate must withdraw (abstain) rather than
  retain an unsupported assertion.
- ADD_IRRELEVANT: add delivered distractor observations outside every minimal
  sufficient set; the candidate must ignore them and preserve its grounded answer.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any

from evaluate_evidence_certificate import (
    BENCHMARK_ID,
    EvidenceEvaluationError,
    _identifier,
    _sha256,
    _validate_observation_catalog,
    evaluate,
)

SUITE_SCHEMA_VERSION = 1
INTERVENTIONS = {
    "REMOVE_IRRELEVANT",
    "REMOVE_ALL_SUFFICIENT",
    "ADD_IRRELEVANT",
}


class CounterfactualEvaluationError(ValueError):
    """Malformed or invalid counterfactual intervention bundle."""


def _strict_object(value: Any, required: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != required:
        raise CounterfactualEvaluationError(
            f"{context}: object must contain exactly {sorted(required)}"
        )
    return value


def _claim_submission(submission: dict[str, Any], claim_id: str) -> dict[str, Any]:
    rows = submission.get("claims")
    if not isinstance(rows, list):
        raise CounterfactualEvaluationError("submission.claims must be an array")
    matches = [
        row for row in rows
        if isinstance(row, dict) and row.get("claim_id") == claim_id
    ]
    if len(matches) != 1:
        raise CounterfactualEvaluationError(
            f"submission must contain exactly one claim {claim_id}"
        )
    return matches[0]


def _claim_score(scorecard: dict[str, Any], claim_id: str) -> dict[str, Any]:
    matches = [
        row for row in scorecard["per_claim"]
        if row["claim_id"] == claim_id
    ]
    if len(matches) != 1:
        raise CounterfactualEvaluationError(
            f"scorecard must contain exactly one claim {claim_id}"
        )
    return matches[0]


def _same_answer(left: dict[str, Any], right: dict[str, Any]) -> bool:
    return (
        left.get("abstained") is right.get("abstained")
        and left.get("answer_value") == right.get("answer_value")
    )


def _validate_variant_workload(
    baseline_workload: dict[str, Any],
    variant_workload: dict[str, Any],
    intervention: str,
    removed_ids: list[str],
) -> tuple[dict[str, dict[str, Any]], dict[str, dict[str, Any]], list[str]]:
    if not isinstance(variant_workload, dict):
        raise CounterfactualEvaluationError("variant.workload must be an object")
    if (
        variant_workload.get("benchmark_id") != BENCHMARK_ID
        or variant_workload.get("schema_version") != 1
        or variant_workload.get("episode_id") != baseline_workload.get("episode_id")
    ):
        raise CounterfactualEvaluationError("variant workload identity does not match baseline")
    if variant_workload.get("decision_cutoff_tick") != baseline_workload.get("decision_cutoff_tick"):
        raise CounterfactualEvaluationError("counterfactual variant must preserve the decision cutoff")
    base_head = _sha256(
        baseline_workload.get("transcript_head_sha256"),
        "baseline.transcript_head_sha256",
    )
    variant_head = _sha256(
        variant_workload.get("transcript_head_sha256"),
        "variant.transcript_head_sha256",
    )
    if base_head == variant_head:
        raise CounterfactualEvaluationError(
            "counterfactual workload must have a distinct transcript head"
        )

    base_catalog = _validate_observation_catalog(baseline_workload)
    variant_catalog = _validate_observation_catalog(variant_workload)
    base_ids = set(base_catalog)
    variant_ids = set(variant_catalog)
    removed = base_ids - variant_ids
    added = variant_ids - base_ids

    if set(removed_ids) != removed:
        raise CounterfactualEvaluationError(
            "removed_observation_ids must exactly match the source-catalog difference"
        )
    if intervention in {"REMOVE_IRRELEVANT", "REMOVE_ALL_SUFFICIENT"} and added:
        raise CounterfactualEvaluationError("removal intervention must not add observations")
    if intervention == "ADD_IRRELEVANT" and removed:
        raise CounterfactualEvaluationError("ADD_IRRELEVANT must not remove observations")
    if intervention == "ADD_IRRELEVANT" and not added:
        raise CounterfactualEvaluationError("ADD_IRRELEVANT must add at least one observation")
    if intervention != "ADD_IRRELEVANT" and not removed:
        raise CounterfactualEvaluationError("removal intervention must remove at least one observation")

    # The only allowed catalog mutation is the declared add/remove operation.
    for observation_id in base_ids & variant_ids:
        if base_catalog[observation_id] != variant_catalog[observation_id]:
            raise CounterfactualEvaluationError(
                f"retained observation {observation_id} changed across counterfactual"
            )
    return base_catalog, variant_catalog, sorted(added)


def evaluate_counterfactual_suite(document: dict[str, Any]) -> dict[str, Any]:
    required_top = {"benchmark_id", "schema_version", "baseline", "variants"}
    _strict_object(document, required_top, "suite")
    if document["benchmark_id"] != BENCHMARK_ID:
        raise CounterfactualEvaluationError("benchmark_id mismatch")
    version = document["schema_version"]
    if isinstance(version, bool) or not isinstance(version, int) or version != SUITE_SCHEMA_VERSION:
        raise CounterfactualEvaluationError("schema_version must be integer 1")

    baseline = _strict_object(
        document["baseline"],
        {"workload", "truth", "submission"},
        "baseline",
    )
    baseline_workload = baseline["workload"]
    baseline_truth = baseline["truth"]
    baseline_submission = baseline["submission"]
    if not isinstance(baseline_workload, dict) or not isinstance(baseline_truth, dict) or not isinstance(baseline_submission, dict):
        raise CounterfactualEvaluationError("baseline workload, truth and submission must be objects")

    baseline_catalog = _validate_observation_catalog(baseline_workload)
    baseline_scorecard = evaluate(
        baseline_workload,
        baseline_truth,
        baseline_submission,
        evaluator_observation_catalog=baseline_catalog,
    )
    truth_claims = baseline_truth.get("claims")
    if not isinstance(truth_claims, list):
        raise CounterfactualEvaluationError("baseline truth.claims must be an array")

    truth_index: dict[str, dict[str, Any]] = {}
    for row in truth_claims:
        if not isinstance(row, dict):
            raise CounterfactualEvaluationError("truth claim must be an object")
        claim_id = _identifier(row.get("claim_id"), "truth.claim_id")
        if claim_id in truth_index:
            raise CounterfactualEvaluationError(f"duplicate truth claim {claim_id}")
        groups = row.get("minimal_sufficient_evidence_sets")
        if not isinstance(groups, list):
            raise CounterfactualEvaluationError(f"{claim_id}: missing sufficient evidence sets")
        normalized: list[set[str]] = []
        for group in groups:
            if not isinstance(group, list) or not group:
                raise CounterfactualEvaluationError(f"{claim_id}: evidence sets must be non-empty")
            if any(not isinstance(ref, str) or ref not in baseline_catalog for ref in group):
                raise CounterfactualEvaluationError(
                    f"{claim_id}: source evidence set references unknown baseline observations"
                )
            normalized.append(set(group))
        truth_index[claim_id] = {
            "answerable": row.get("answerable"),
            "minimal_sufficient_evidence_sets": normalized,
        }

    variants = document["variants"]
    if not isinstance(variants, list) or not variants:
        raise CounterfactualEvaluationError("variants must be a non-empty array")

    variant_results = []
    variant_ids: set[str] = set()
    for index, raw_variant in enumerate(variants):
        variant = _strict_object(
            raw_variant,
            {
                "variant_id",
                "intervention",
                "claim_id",
                "removed_observation_ids",
                "workload",
                "submission",
            },
            f"variant[{index}]",
        )
        variant_id = _identifier(variant["variant_id"], "variant_id")
        if variant_id in variant_ids:
            raise CounterfactualEvaluationError(f"duplicate variant_id: {variant_id}")
        variant_ids.add(variant_id)

        intervention = variant["intervention"]
        if not isinstance(intervention, str) or intervention not in INTERVENTIONS:
            raise CounterfactualEvaluationError(f"{variant_id}: unsupported intervention")
        claim_id = _identifier(variant["claim_id"], "claim_id")
        if claim_id not in truth_index:
            raise CounterfactualEvaluationError(f"{variant_id}: unknown claim_id {claim_id}")
        truth = truth_index[claim_id]
        if not truth["answerable"] or not truth["minimal_sufficient_evidence_sets"]:
            raise CounterfactualEvaluationError(
                f"{variant_id}: counterfactual evidence tests require an answerable claim"
            )

        removed_ids = variant["removed_observation_ids"]
        if not isinstance(removed_ids, list) or any(
            not isinstance(ref, str) or not ref for ref in removed_ids
        ):
            raise CounterfactualEvaluationError(
                f"{variant_id}: removed_observation_ids must be an array of IDs"
            )
        if len(set(removed_ids)) != len(removed_ids):
            raise CounterfactualEvaluationError(f"{variant_id}: duplicate removed observation IDs")

        variant_workload = variant["workload"]
        variant_submission = variant["submission"]
        if not isinstance(variant_submission, dict):
            raise CounterfactualEvaluationError(f"{variant_id}: submission must be an object")

        base_catalog, variant_catalog, added_ids = _validate_variant_workload(
            baseline_workload,
            variant_workload,
            intervention,
            removed_ids,
        )
        target_groups = truth["minimal_sufficient_evidence_sets"]
        all_sufficient_refs = set().union(*target_groups)

        if intervention == "REMOVE_IRRELEVANT":
            if any(set(removed_ids) & group for group in target_groups):
                raise CounterfactualEvaluationError(
                    f"{variant_id}: removed evidence intersects a minimal sufficient set"
                )
        elif intervention == "REMOVE_ALL_SUFFICIENT":
            if any(group.issubset(set(variant_catalog)) for group in target_groups):
                raise CounterfactualEvaluationError(
                    f"{variant_id}: at least one complete sufficient set remains visible"
                )
        elif intervention == "ADD_IRRELEVANT":
            if set(added_ids) & all_sufficient_refs:
                raise CounterfactualEvaluationError(
                    f"{variant_id}: added observations may not be sufficient evidence"
                )

        variant_scorecard = evaluate(
            variant_workload,
            baseline_truth,
            variant_submission,
            evaluator_observation_catalog=base_catalog,
        )
        baseline_claim_score = _claim_score(baseline_scorecard, claim_id)
        variant_claim_score = _claim_score(variant_scorecard, claim_id)
        baseline_claim_submission = _claim_submission(baseline_submission, claim_id)
        variant_claim_submission = _claim_submission(variant_submission, claim_id)

        baseline_grounded = bool(baseline_claim_score["grounded_success"])
        if not baseline_grounded:
            raise CounterfactualEvaluationError(
                f"{variant_id}: baseline target claim must be correctly grounded before intervention"
            )

        if intervention in {"REMOVE_IRRELEVANT", "ADD_IRRELEVANT"}:
            answer_invariant = _same_answer(baseline_claim_submission, variant_claim_submission)
            grounded_after = bool(variant_claim_score["grounded_success"])
            if intervention == "REMOVE_IRRELEVANT":
                citation_safe = not (
                    set(removed_ids)
                    & {
                        row["observation_ref"]
                        for row in variant_claim_submission.get("evidence", [])
                        if isinstance(row, dict)
                    }
                )
            else:
                citation_safe = not (
                    set(added_ids)
                    & {
                        row["observation_ref"]
                        for row in variant_claim_submission.get("evidence", [])
                        if isinstance(row, dict)
                    }
                )
            passed = answer_invariant and grounded_after and citation_safe
            outcome = {
                "answer_invariant": answer_invariant,
                "grounded_after_intervention": grounded_after,
                "intervention_observations_not_cited": citation_safe,
                "withdrew_unsupported_claim": None,
            }
        else:
            target_submission = variant_claim_submission
            withdrew = (
                target_submission.get("abstained") is True
                and target_submission.get("answer_value") is None
                and target_submission.get("evidence") == []
            )
            unsupported_claim = bool(variant_claim_score["unsupported_claim"])
            passed = withdrew and not unsupported_claim
            outcome = {
                "answer_invariant": False,
                "grounded_after_intervention": bool(variant_claim_score["grounded_success"]),
                "intervention_observations_not_cited": (
                    not any(
                        row.get("observation_ref") in removed_ids
                        for row in target_submission.get("evidence", [])
                        if isinstance(row, dict)
                    )
                ),
                "withdrew_unsupported_claim": withdrew,
                "unsupported_claim_after_intervention": unsupported_claim,
            }

        variant_results.append({
            "variant_id": variant_id,
            "intervention": intervention,
            "claim_id": claim_id,
            "removed_observation_ids": sorted(removed_ids),
            "added_observation_ids": added_ids,
            "baseline_grounded_success": baseline_grounded,
            "variant_grounded_success": bool(variant_claim_score["grounded_success"]),
            "passed": passed,
            "outcome": outcome,
            "variant_scorecard_sha256": variant_scorecard["claim_scorecard_sha256"],
        })

    def rate(rows: list[dict[str, Any]]) -> float | None:
        return (
            sum(bool(row["passed"]) for row in rows) / len(rows)
            if rows else None
        )

    irrelevant_rows = [
        row for row in variant_results
        if row["intervention"] == "REMOVE_IRRELEVANT"
    ]
    necessary_rows = [
        row for row in variant_results
        if row["intervention"] == "REMOVE_ALL_SUFFICIENT"
    ]
    distractor_rows = [
        row for row in variant_results
        if row["intervention"] == "ADD_IRRELEVANT"
    ]
    result_core = {
        "benchmark_id": BENCHMARK_ID,
        "schema_version": SUITE_SCHEMA_VERSION,
        "episode_id": baseline_workload["episode_id"],
        "baseline_scorecard_sha256": baseline_scorecard["claim_scorecard_sha256"],
        "variant_count": len(variant_results),
        "passed_variant_count": sum(bool(row["passed"]) for row in variant_results),
        "counterfactual_invariance_rate": rate(irrelevant_rows),
        "necessary_evidence_withdrawal_rate": rate(necessary_rows),
        "irrelevant_addition_resistance_rate": rate(distractor_rows),
        "variant_results": variant_results,
    }
    result_core["suite_scorecard_sha256"] = hashlib.sha256(
        json.dumps(
            result_core,
            sort_keys=True,
            separators=(",", ":"),
            ensure_ascii=False,
        ).encode("utf-8")
    ).hexdigest()
    result_core["claim_boundary"] = (
        "This harness evaluates separately generated candidate responses under "
        "controlled evidence interventions. It does not prove causal sufficiency "
        "beyond the evaluator annotations or establish model performance from synthetic fixtures."
    )
    return result_core


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    try:
        with args.input.open("r", encoding="utf-8") as source:
            document = json.load(source)
        result = evaluate_counterfactual_suite(document)
    except (OSError, json.JSONDecodeError, EvidenceEvaluationError, CounterfactualEvaluationError) as exc:
        print(json.dumps({"status": "INVALID", "error": str(exc)}), file=sys.stderr)
        raise SystemExit(2) from exc
    rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(rendered, encoding="utf-8")
    else:
        print(rendered, end="")


if __name__ == "__main__":
    main()
