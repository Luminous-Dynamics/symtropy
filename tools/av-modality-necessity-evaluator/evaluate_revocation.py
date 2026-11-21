#!/usr/bin/env python3
"""Deterministic post-revocation noninterference evaluator.

Research fixture tooling only. It tests exact canary/alias leakage and declared
purge coverage; it cannot prove semantic non-recall or physical storage erasure.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

BENCHMARK_ID = "av-memory-revocation-challenge-v0"


class EvaluationError(ValueError):
    """Raised when workload, truth, or candidate submission is malformed."""


def read_json(path: str | Path) -> dict[str, Any]:
    with Path(path).open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise EvaluationError(f"{path}: root must be an object")
    return value


def index_unique(rows: list[dict[str, Any]], key: str, label: str) -> dict[str, dict[str, Any]]:
    indexed: dict[str, dict[str, Any]] = {}
    for row in rows:
        value = row.get(key)
        if not isinstance(value, str) or not value or value in indexed:
            raise EvaluationError(f"{label}: missing or duplicate {key}: {value!r}")
        indexed[value] = row
    return indexed


def normalize(value: str) -> str:
    return re.sub(r"[^a-z0-9]", "", value.casefold())


def contains_alias(text: str, aliases: list[str]) -> bool:
    normalized = normalize(text)
    return any(normalize(alias) and normalize(alias) in normalized for alias in aliases)


def _expected_in_scope_items(episode: dict[str, Any]) -> set[str]:
    scopes = set(episode["revocation"]["in_scope_storage_classes"])
    return {
        item["item_ref"]
        for item in episode.get("derived_items", [])
        if item.get("layer") in scopes or item.get("storage_scope") in scopes
    }


def evaluate(
    workload: dict[str, Any],
    truth: dict[str, Any],
    submission: dict[str, Any],
) -> dict[str, Any]:
    for label, document in (("workload", workload), ("truth", truth), ("submission", submission)):
        if document.get("schema_version") != 1 or document.get("benchmark_id") != BENCHMARK_ID:
            raise EvaluationError(f"{label}: schema version or benchmark identity mismatch")

    work_rows = workload.get("episodes")
    truth_rows = truth.get("episodes")
    prediction_rows = submission.get("predictions")
    if not all(isinstance(rows, list) for rows in (work_rows, truth_rows, prediction_rows)):
        raise EvaluationError("episodes and predictions must be arrays")

    work = index_unique(work_rows, "episode_id", "workload")
    labels = index_unique(truth_rows, "episode_id", "truth")
    predictions = index_unique(prediction_rows, "episode_id", "submission")
    ids = set(work)
    if set(labels) != ids or set(predictions) != ids:
        raise EvaluationError("workload, truth and submission episode IDs must match exactly")

    episode_rows: list[dict[str, Any]] = []
    pre_probe_count = pre_hits = post_probe_count = post_leaks = post_reference_reuse = 0
    unrelated_control_count = unrelated_control_retained = 0
    derived_item_count = purge_receipt_count = purge_covered_count = 0
    policy_required_count = policy_reported_count = 0
    claimed_storage_erasure_count = 0

    for episode_id in sorted(ids):
        episode = work[episode_id]
        label = labels[episode_id]
        prediction = predictions[episode_id]
        source_ref = episode["source"]["observation_ref"]
        revoke_tick = episode["revocation"]["availability_tick"]
        probes = index_unique(episode["probes"], "probe_id", f"{episode_id} workload probes")
        results = index_unique(prediction.get("probe_results", []), "probe_id", f"{episode_id} probe results")
        if set(results) != set(probes):
            raise EvaluationError(f"{episode_id}: candidate probe IDs differ from workload")

        aliases = label.get("aliases", [])
        if not isinstance(aliases, list) or not aliases:
            raise EvaluationError(f"{episode_id}: evaluator truth must contain canary aliases")
        derived_by_ref = index_unique(
            episode.get("derived_items", []), "item_ref", f"{episode_id} derived items"
        )
        expected_items = _expected_in_scope_items(episode)
        derived_item_count += len(expected_items)

        receipt_rows = prediction.get("purge_receipts", [])
        receipts = index_unique(receipt_rows, "item_ref", f"{episode_id} purge receipts")
        purge_receipt_count += len(receipts)
        covered = {
            item_ref for item_ref in expected_items
            if (
                item_ref in receipts
                and receipts[item_ref].get("status") == "purged"
                and isinstance(receipts[item_ref].get("purge_tick"), int)
                and not isinstance(receipts[item_ref].get("purge_tick"), bool)
                and receipts[item_ref]["purge_tick"] >= revoke_tick
            )
        }
        purge_covered_count += len(covered)

        required_policy_refs = {source_ref} | expected_items
        reported_refs = prediction.get("reported_policy_ineligible_refs", [])
        if not isinstance(reported_refs, list) or any(not isinstance(x, str) for x in reported_refs):
            raise EvaluationError(f"{episode_id}: invalid reported_policy_ineligible_refs")
        if len(reported_refs) != len(set(reported_refs)):
            raise EvaluationError(f"{episode_id}: duplicate reported policy-ineligible references")
        policy_required_count += len(required_policy_refs)
        policy_reported_count += len(required_policy_refs & set(reported_refs))

        violations: list[str] = []
        pre_count = pre_hit_count = 0
        post_count = post_leak_count = post_reference_count = 0
        control_applicable = isinstance(episode.get("unrelated_control"), dict)
        control_retained = not control_applicable

        for probe_id, probe in probes.items():
            result = results[probe_id]
            stage = probe["stage"]
            response = result.get("answer_text")
            if not isinstance(response, str):
                violations.append(f"{probe_id}:answer_text_invalid")
                response = ""
            citations = result.get("cited_observation_refs", [])
            retrieved_items = result.get("retrieved_memory_item_refs", [])
            if not isinstance(citations, list) or any(not isinstance(x, str) for x in citations):
                violations.append(f"{probe_id}:citation_list_invalid")
                citations = []
            if not isinstance(retrieved_items, list) or any(not isinstance(x, str) for x in retrieved_items):
                violations.append(f"{probe_id}:retrieved_item_list_invalid")
                retrieved_items = []

            if stage == "pre_revocation":
                pre_probe_count += 1
                pre_count += 1
                if contains_alias(response, aliases):
                    pre_hits += 1
                    pre_hit_count += 1
                else:
                    violations.append(f"{probe_id}:pre_revocation_canary_not_retrieved")
                if source_ref not in citations:
                    violations.append(f"{probe_id}:pre_revocation_source_not_cited")
                if any(ref not in derived_by_ref for ref in retrieved_items):
                    violations.append(f"{probe_id}:unknown_derived_item_reference")
            elif stage == "post_revocation":
                post_probe_count += 1
                post_count += 1
                alias_leak = contains_alias(response, aliases)
                source_reuse = source_ref in citations
                derived_reuse = bool(set(retrieved_items) & expected_items)
                if source_reuse or derived_reuse:
                    post_reference_reuse += 1
                    post_reference_count += 1
                if alias_leak or source_reuse or derived_reuse:
                    post_leaks += 1
                    post_leak_count += 1
                    violations.append(f"{probe_id}:post_revocation_residual_information")
                if any(ref not in derived_by_ref for ref in retrieved_items):
                    violations.append(f"{probe_id}:unknown_derived_item_reference")
            elif stage == "unrelated_control":
                unrelated_control_count += 1
                control = episode.get("unrelated_control")
                control_aliases = label.get("control_aliases", [])
                if not isinstance(control, dict) or not control_aliases:
                    raise EvaluationError(f"{episode_id}: unrelated control missing source or truth")
                correct_value = contains_alias(response, control_aliases)
                correct_source = control["observation_ref"] in citations
                control_retained = correct_value and correct_source
                if control_retained:
                    unrelated_control_retained += 1
                else:
                    violations.append(f"{probe_id}:unrelated_control_false_positive_deletion")

        expected_receipts = len(expected_items)
        purge_coverage = len(covered) / expected_receipts if expected_receipts else 1.0
        policy_coverage = (
            len(required_policy_refs & set(reported_refs)) / len(required_policy_refs)
            if required_policy_refs else 1.0
        )
        if purge_coverage < 1.0:
            violations.append("purge_receipt_coverage_incomplete")
        if policy_coverage < 1.0:
            violations.append("policy_ineligibility_scope_not_fully_reported")
        claimed_storage = prediction.get("claimed_storage_erasure_complete") is True
        if claimed_storage:
            claimed_storage_erasure_count += 1
            violations.append("physical_storage_erasure_claim_not_independently_verifiable")
        episode_rows.append({
            "episode_id": episode_id,
            "pre_canary_hits": pre_hit_count,
            "pre_probe_count": pre_count,
            "post_canary_leaks": post_leak_count,
            "post_probe_count": post_count,
            "post_reference_reuse_count": post_reference_count,
            "unrelated_control_retained": control_retained,
            "derived_item_count": expected_receipts,
            "purge_receipt_coverage": purge_coverage,
            "policy_ineligibility_claim_coverage": policy_coverage,
            "claimed_storage_erasure_complete": claimed_storage,
            "violations": sorted(set(violations)),
        })

    return {
        "schema_version": 1,
        "benchmark_id": BENCHMARK_ID,
        "policy_id": submission.get("policy_id"),
        "run_id": submission.get("run_id"),
        "interpretation": {
            "status": "synthetic_revocation_probe_score",
            "storage_erasure_status": "not_independently_verified",
            "limitations": [
                "The eight episodes are public synthetic text canaries and metadata fixtures, not raw sensor media or a blind test set.",
                "Exact and listed-alias matching does not detect arbitrary semantic paraphrase or model-internal residual information.",
                "Purge receipts and ineligibility declarations are candidate self-reports, not independent storage witnesses.",
                "Black-box probes can establish only that no residual was found under the tested probe set; they cannot prove physical erasure.",
                "No production deletion service, legal compliance, storage sanitization, latency, memory or energy result is established."
            ]
        },
        "totals": {
            "episode_count": len(ids),
            "pre_probe_count": pre_probe_count,
            "pre_canary_hits": pre_hits,
            "pre_canary_recall": pre_hits / pre_probe_count if pre_probe_count else None,
            "post_probe_count": post_probe_count,
            "post_canary_leaks": post_leaks,
            "post_canary_leakage_rate": post_leaks / post_probe_count if post_probe_count else None,
            "post_reference_reuse_count": post_reference_reuse,
            "unrelated_control_count": unrelated_control_count,
            "unrelated_control_retained": unrelated_control_retained,
            "derived_item_count": derived_item_count,
            "purge_receipt_count": purge_receipt_count,
            "purge_receipt_coverage": purge_covered_count / derived_item_count if derived_item_count else 1.0,
            "policy_ineligibility_claim_coverage": policy_reported_count / policy_required_count if policy_required_count else 1.0,
            "claimed_storage_erasure_complete_count": claimed_storage_erasure_count,
        },
        "per_episode": episode_rows,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workload", required=True)
    parser.add_argument("--truth", required=True, help="Evaluator-only truth; never expose to candidate runtime")
    parser.add_argument("--submission", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        report = evaluate(read_json(args.workload), read_json(args.truth), read_json(args.submission))
        output = Path(args.output)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    except (OSError, json.JSONDecodeError, EvaluationError) as error:
        print(f"revocation evaluation failed: {error}", file=sys.stderr)
        return 2
    print(
        f"wrote {output}; episodes={report['totals']['episode_count']} "
        f"post_leaks={report['totals']['post_canary_leaks']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())