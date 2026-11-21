#!/usr/bin/env python3
"""Matched comparison of recent-window, summary, episodic and evidence-ledger memory."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from pathlib import Path
from typing import Any

from long_horizon_evaluate import EvaluationError, evaluate, read_json

CONDITIONS = {"recent_window", "summary_only", "episodic_retrieval", "evidence_ledger"}
MATCHED_FIELDS = (
    "comparison_id", "policy_id", "model_revision", "runtime_revision", "hardware_id",
    "seed", "workload_ref", "protocol_id", "memory_budget_profile_id", "measurement_status", "budget",
)
RESOURCE_FIELDS = ("memory_bytes_used", "input_tokens", "retrieval_latency_ms", "energy_joules")


def _percentile(values: list[float], q: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    return ordered[max(0, math.ceil(q * len(ordered)) - 1)]


def _resource_usage(run: dict[str, Any], submission: dict[str, Any], workload: dict[str, Any]) -> dict[str, Any]:
    status = run.get("measurement_status")
    predictions = {p["episode_id"]: p for p in submission["predictions"]}
    episodes = {e["episode_id"]: e for e in workload["episodes"]}
    if set(predictions) != set(episodes):
        raise EvaluationError("resource accounting requires one prediction per workload episode")
    if status == "synthetic_reference":
        if any(prediction.get(field) is not None for prediction in predictions.values() for field in RESOURCE_FIELDS):
            raise EvaluationError("synthetic reference cannot claim measured memory or runtime costs")
        return {
            "measurement_status": status, "memory_bytes_total": None, "input_tokens_total": None,
            "retrieval_latency_p50_ms": None, "retrieval_latency_p95_ms": None, "energy_joules_total": None,
        }
    if status != "measured":
        raise EvaluationError("measurement_status must be synthetic_reference or measured")

    budget = run["budget"]
    collected = {field: [] for field in RESOURCE_FIELDS}
    for episode_id, prediction in predictions.items():
        episode_budget = episodes[episode_id]["memory_budget_bytes"]
        caps = {
            "memory_bytes_used": min(episode_budget, budget["max_memory_bytes_per_episode"]),
            "input_tokens": budget["max_input_tokens_per_episode"],
            "retrieval_latency_ms": budget["max_retrieval_latency_ms"],
            "energy_joules": budget["max_energy_joules_per_episode"],
        }
        for field in RESOURCE_FIELDS:
            value = prediction.get(field)
            if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(float(value)) or value < 0:
                raise EvaluationError(f"measured run requires finite non-negative {field} per episode")
            if value > caps[field]:
                raise EvaluationError(f"{episode_id}: exceeded per-episode {field} budget")
            collected[field].append(float(value))
    return {
        "measurement_status": status,
        "memory_bytes_total": int(sum(collected["memory_bytes_used"])),
        "input_tokens_total": int(sum(collected["input_tokens"])),
        "retrieval_latency_p50_ms": _percentile(collected["retrieval_latency_ms"], 0.50),
        "retrieval_latency_p95_ms": _percentile(collected["retrieval_latency_ms"], 0.95),
        "energy_joules_total": sum(collected["energy_joules"]),
    }


def _paired(ledger: dict[str, dict[str, Any]], baseline: dict[str, dict[str, Any]]) -> dict[str, Any]:
    wins = losses = ties = 0
    provenance_wins = provenance_losses = 0
    for episode_id in sorted(ledger):
        left = bool(ledger[episode_id]["answer_correct"])
        right = bool(baseline[episode_id]["answer_correct"])
        if left and not right:
            wins += 1
        elif right and not left:
            losses += 1
        else:
            ties += 1
        lp = bool(ledger[episode_id]["provenance_complete"])
        rp = bool(baseline[episode_id]["provenance_complete"])
        if lp and not rp:
            provenance_wins += 1
        elif rp and not lp:
            provenance_losses += 1
    count = len(ledger)
    return {
        "episode_count": count, "ledger_answer_wins": wins,
        "baseline_answer_wins": losses, "ties": ties,
        "net_answer_accuracy_gain": (wins - losses) / count if count else None,
        "ledger_provenance_wins": provenance_wins,
        "baseline_provenance_wins": provenance_losses,
    }


def compare_memory_conditions(
    workload: dict[str, Any],
    truth: dict[str, Any],
    runs: list[dict[str, Any]],
    workload_ref: str | None = None,
) -> dict[str, Any]:
    if not isinstance(runs, list) or len(runs) != 4:
        raise EvaluationError("exactly four memory-condition runs are required")
    by_condition = {}
    for run in runs:
        condition = run.get("memory_condition")
        if condition not in CONDITIONS or condition in by_condition:
            raise EvaluationError(f"missing, unknown or duplicate memory_condition: {condition!r}")
        by_condition[condition] = run
    if set(by_condition) != CONDITIONS:
        raise EvaluationError("tournament requires all four declared memory conditions")

    reference = by_condition["evidence_ledger"]
    for condition, run in by_condition.items():
        for field in MATCHED_FIELDS:
            if run.get(field) != reference.get(field):
                raise EvaluationError(f"unmatched run metadata for {condition}: {field}")
        if workload_ref is not None and run.get("workload_ref") != workload_ref:
            raise EvaluationError(f"{condition}: workload reference does not match exact input bytes")
        submission = run.get("submission")
        if not isinstance(submission, dict):
            raise EvaluationError(f"{condition}: missing nested submission")
        for field in ("policy_id", "run_id", "memory_condition", "model_revision", "runtime_revision"):
            if submission.get(field) != run.get(field):
                raise EvaluationError(f"{condition}: nested submission {field} does not match run envelope")

    reports = {}
    indexes = {}
    resources = {}
    for condition, run in by_condition.items():
        report = evaluate(workload, truth, run["submission"])
        reports[condition] = report
        indexes[condition] = {row["episode_id"]: row for row in report["per_episode"]}
        resources[condition] = _resource_usage(run, run["submission"], workload)

    episode_ids = {row["episode_id"] for row in workload["episodes"]}
    for condition, rows in indexes.items():
        if set(rows) != episode_ids:
            raise EvaluationError(f"{condition}: episode coverage differs from workload")

    per_condition = {}
    for condition, report in reports.items():
        total = report["totals"]
        per_condition[condition] = {
            "answer_accuracy": total["answer_accuracy"],
            "decision_accuracy": total["decision_accuracy"],
            "coverage": total["coverage"],
            "selective_accuracy": total["selective_accuracy"],
            "citation_precision": total["citation_precision"],
            "citation_recall": total["citation_recall"],
            "provenance_complete_rate": total["provenance_complete_rate"],
            "evidence_qualified_answer_accuracy": total["evidence_qualified_answer_accuracy"],
            "evidence_compliance_rate": total["evidence_compliance_rate"],
            "false_memory_count": total["false_memory_count"],
            "hard_violation_count": total["hard_violation_count"],
            "resource_usage": resources[condition],
        }

    ledger = indexes["evidence_ledger"]
    paired = {
        "vs_recent_window": _paired(ledger, indexes["recent_window"]),
        "vs_summary_only": _paired(ledger, indexes["summary_only"]),
        "vs_episodic_retrieval": _paired(ledger, indexes["episodic_retrieval"]),
    }
    return {
        "schema_version": 1,
        "benchmark_id": "av-long-horizon-evidence-v0",
        "comparison_id": reference["comparison_id"],
        "matched_protocol": {
            field: reference[field] for field in MATCHED_FIELDS if field != "comparison_id"
        },
        "interpretation": {
            "status": "synthetic_matched_memory_reference",
            "limitations": [
                "The four reference arms are hand-authored synthetic policies, not executed memory systems.",
                "The benchmark contains twelve metadata-only episodes and placeholder content digests, not raw PCM/RGB evidence.",
                "Paired differences are fixture-mechanics checks and are not statistical evidence of memory-system superiority.",
                "Synthetic reference arms are forbidden from claiming memory, token, latency or energy measurements.",
                "A public evaluator-truth file is not a blind holdout or a security boundary."
            ]
        },
        "per_condition": per_condition,
        "evidence_ledger_paired_comparisons": paired,
    }


def git_blob_sha1(path: str | Path) -> str:
    data = Path(path).read_bytes()
    return hashlib.sha1(f"blob {len(data)}\0".encode("ascii") + data).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workload", required=True)
    parser.add_argument("--truth", required=True)
    parser.add_argument("--recent-window", required=True)
    parser.add_argument("--summary-only", required=True)
    parser.add_argument("--episodic-retrieval", required=True)
    parser.add_argument("--evidence-ledger", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        runs = [
            read_json(args.recent_window), read_json(args.summary_only),
            read_json(args.episodic_retrieval), read_json(args.evidence_ledger),
        ]
        report = compare_memory_conditions(
            read_json(args.workload), read_json(args.truth), runs,
            workload_ref="git-blob-sha1:" + git_blob_sha1(args.workload),
        )
        output = Path(args.output)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    except (OSError, json.JSONDecodeError, EvaluationError) as error:
        print(f"memory tournament failed: {error}", file=sys.stderr)
        return 2
    print(f"wrote {output}; conditions={len(report['per_condition'])}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())