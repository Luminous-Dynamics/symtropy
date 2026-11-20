#!/usr/bin/env python3
"""Matched A-only / V-only / A+V ablation tournament for AV benchmark runs."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from pathlib import Path
from typing import Any

from evaluate import EvaluationError, NECESSARY_MODALITIES, evaluate, read_json

EXPECTED_ARMS = {
    "video_only": {"video"},
    "audio_only": {"audio"},
    "audio_video": {"audio", "video"},
}
MATCHED_FIELDS = (
    "comparison_id", "policy_id", "model_revision", "runtime_revision",
    "hardware_id", "seed", "workload_ref", "protocol_id", "shared_config_id", "budget",
    "measurement_status",
)

MEASURED_FIELDS = (
    "audio_bytes", "video_frames", "input_tokens", "output_tokens",
    "latency_ms", "energy_joules",
)


def _percentile(values: list[float], quantile: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    index = max(0, math.ceil(quantile * len(ordered)) - 1)
    return ordered[index]


def _resource_usage(run: dict[str, Any], predictions: list[dict[str, Any]]) -> dict[str, Any]:
    status = run.get("measurement_status")
    if status == "synthetic_reference":
        if any(prediction.get(field) is not None for prediction in predictions for field in MEASURED_FIELDS):
            raise EvaluationError("synthetic_reference arm must not claim measured resource telemetry")
        return {
            "measurement_status": status, "audio_bytes_total": None, "video_frames_total": None,
            "input_tokens_total": None, "output_tokens_total": None, "energy_joules_total": None,
            "latency_p50_ms": None, "latency_p95_ms": None, "latency_p99_ms": None,
        }
    if status != "measured":
        raise EvaluationError("measurement_status must be synthetic_reference or measured")

    budget = run["budget"]
    collected: dict[str, list[float]] = {field: [] for field in MEASURED_FIELDS}
    caps = {
        "audio_bytes": budget["max_audio_bytes"],
        "video_frames": budget["max_video_frames"],
        "input_tokens": budget["max_input_tokens"],
        "output_tokens": budget["max_output_tokens"],
        "latency_ms": budget["max_response_ms"],
        "energy_joules": budget["max_energy_joules"],
    }
    for prediction in predictions:
        for field in MEASURED_FIELDS:
            value = prediction.get(field)
            if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(float(value)) or value < 0:
                raise EvaluationError(f"measured arm requires finite non-negative {field} for every episode")
            if value > caps[field]:
                raise EvaluationError(f"measured arm exceeded {field} per-episode budget")
            collected[field].append(float(value))
        used = set(prediction.get("used_modalities", []))
        if "audio" not in run["allowed_modalities"] and prediction["audio_bytes"] != 0:
            raise EvaluationError("non-audio arm reports nonzero audio bytes")
        if "video" not in run["allowed_modalities"] and prediction["video_frames"] != 0:
            raise EvaluationError("non-video arm reports nonzero video frames")

    return {
        "measurement_status": status,
        "audio_bytes_total": int(sum(collected["audio_bytes"])),
        "video_frames_total": int(sum(collected["video_frames"])),
        "input_tokens_total": int(sum(collected["input_tokens"])),
        "output_tokens_total": int(sum(collected["output_tokens"])),
        "energy_joules_total": sum(collected["energy_joules"]),
        "latency_p50_ms": _percentile(collected["latency_ms"], 0.50),
        "latency_p95_ms": _percentile(collected["latency_ms"], 0.95),
        "latency_p99_ms": _percentile(collected["latency_ms"], 0.99),
    }


def _index(rows: list[dict[str, Any]], key: str) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for row in rows:
        value = row.get(key)
        if not isinstance(value, str) or not value or value in result:
            raise EvaluationError(f"invalid or duplicate {key}: {value!r}")
        result[value] = row
    return result


def _exact_binomial_two_sided(wins: int, losses: int) -> float:
    """Exact two-sided McNemar/binomial p-value for discordant paired outcomes."""
    discordant = wins + losses
    if discordant == 0:
        return 1.0
    tail = min(wins, losses)
    probability = sum(math.comb(discordant, k) for k in range(tail + 1)) / (2 ** discordant)
    return min(1.0, 2.0 * probability)


def _paired_comparison(
    base: dict[str, dict[str, Any]],
    candidate: dict[str, dict[str, Any]],
    truth_by_id: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    def score_subset(ids: list[str]) -> dict[str, Any]:
        wins = losses = ties = 0
        for identifier in ids:
            b = bool(base[identifier]["answer_correct"])
            c = bool(candidate[identifier]["answer_correct"])
            if c and not b:
                wins += 1
            elif b and not c:
                losses += 1
            else:
                ties += 1
        count = len(ids)
        return {
            "episode_count": count,
            "candidate_answer_wins": wins,
            "baseline_answer_wins": losses,
            "ties": ties,
            "net_answer_accuracy_gain": (wins - losses) / count if count else None,
            "exact_mcnemar_two_sided_p_value": _exact_binomial_two_sided(wins, losses),
        }

    all_ids = sorted(set(base) & set(candidate))
    strata: dict[str, list[str]] = {}
    for identifier in all_ids:
        stratum = str(truth_by_id[identifier].get("necessity", "unknown"))
        strata.setdefault(stratum, []).append(identifier)
    return {
        "overall": score_subset(all_ids),
        "per_necessity": {
            key: score_subset(sorted(values)) for key, values in sorted(strata.items())
        },
    }


def compare_arms(
    workload: dict[str, Any],
    truth: dict[str, Any],
    arm_runs: list[dict[str, Any]],
    workload_ref: str | None = None,
) -> dict[str, Any]:
    if not isinstance(arm_runs, list) or len(arm_runs) != 3:
        raise EvaluationError("exactly three arm runs are required")
    by_arm: dict[str, dict[str, Any]] = {}
    for run in arm_runs:
        arm = run.get("input_arm")
        if arm not in EXPECTED_ARMS or arm in by_arm:
            raise EvaluationError(f"missing, unknown or duplicate input_arm: {arm!r}")
        by_arm[arm] = run
    if set(by_arm) != set(EXPECTED_ARMS):
        raise EvaluationError("the tournament requires video_only, audio_only and audio_video")

    reference = by_arm["audio_video"]
    for arm, run in by_arm.items():
        for field in MATCHED_FIELDS:
            if run.get(field) != reference.get(field):
                raise EvaluationError(f"unmatched run metadata for {arm}: {field}")
        if set(run.get("allowed_modalities", [])) != EXPECTED_ARMS[arm]:
            raise EvaluationError(f"{arm}: allowed_modalities does not match input_arm")
        submission = run.get("submission")
        if not isinstance(submission, dict):
            raise EvaluationError(f"{arm}: missing nested candidate submission")
        if submission.get("policy_id") != run.get("policy_id") or submission.get("run_id") != run.get("run_id"):
            raise EvaluationError(f"{arm}: nested submission identity does not match arm envelope")
        if submission.get("model_revision") != run.get("model_revision") or submission.get("runtime_revision") != run.get("runtime_revision"):
            raise EvaluationError(f"{arm}: nested model/runtime revision does not match arm envelope")
        if workload_ref is not None and run.get("workload_ref") != workload_ref:
            raise EvaluationError(f"{arm}: workload reference does not match exact input bytes")

    scored: dict[str, dict[str, Any]] = {}
    indexed_results: dict[str, dict[str, dict[str, Any]]] = {}
    resource_usage: dict[str, dict[str, Any]] = {}
    for arm, run in by_arm.items():
        report = evaluate(workload, truth, run["submission"])
        allowed = EXPECTED_ARMS[arm]
        for prediction in run["submission"]["predictions"]:
            used = set(prediction.get("used_modalities", []))
            if not used <= allowed:
                raise EvaluationError(f"{arm}: candidate claims to use unavailable modality")
            if prediction.get("used_transcript") is True:
                raise EvaluationError(f"{arm}: transcript path is not admitted in raw-modality tournament")
        resource_usage[arm] = _resource_usage(run, run["submission"]["predictions"])
        scored[arm] = report
        indexed_results[arm] = _index(report["per_episode"], "episode_id")

    workload_ids = {row["episode_id"] for row in workload["episodes"]}
    truth_by_id = _index(truth["episodes"], "episode_id")
    for arm, indexed in indexed_results.items():
        if set(indexed) != workload_ids:
            raise EvaluationError(f"{arm}: scored episode set differs from the frozen workload")

    av = indexed_results["audio_video"]
    comparisons = {
        "audio_video_vs_video_only": _paired_comparison(
            indexed_results["video_only"], av, truth_by_id
        ),
        "audio_video_vs_audio_only": _paired_comparison(
            indexed_results["audio_only"], av, truth_by_id
        ),
    }

    per_arm = {}
    episode_count = len(workload_ids)
    for arm, report in scored.items():
        totals = report["totals"]
        per_arm[arm] = {
            "episode_count": totals["episode_count"],
            "answer_accuracy": (
                totals["answered_correct_count"] / episode_count if episode_count else None
            ),
            "answered_correct_count": totals["answered_correct_count"],
            "coverage": totals["coverage"],
            "selective_accuracy": totals["selective_accuracy"],
            "safe_decision_accuracy": totals["decision_accuracy"],
            "evidence_compliance_rate": totals["evidence_compliance_rate"],
            "hard_violation_count": totals["hard_violation_count"],
            "deadline_miss_count": totals["deadline_miss_count"],
            "resource_usage": resource_usage[arm],
        }

    return {
        "schema_version": 1,
        "benchmark_id": "av-modality-necessity-v0",
        "comparison_id": reference["comparison_id"],
        "policy_id": reference["policy_id"],
        "matched_protocol": {
            field: reference[field]
            for field in MATCHED_FIELDS if field != "policy_id"
        },
        "interpretation": {
            "status": "synthetic_matched_ablation_reference",
            "limitations": [
                "This result is generated from an authored synthetic reference policy, not an ML model run.",
                "The workload includes synthetic metadata and placeholder media commitments, not raw PCM/RGB evidence.",
                "A-only, V-only and A+V are controlled input-availability arms; this does not prove a model's internal causal feature use.",
                "The exact McNemar p-values are descriptive on this tiny synthetic fixture and must not be treated as inferential evidence.",
                "No latency, energy, throughput or cost superiority is inferred from replay ticks or declared budgets."
            ]
        },
        "per_arm": per_arm,
        "paired_answer_comparisons": comparisons,
    }


def git_blob_sha1(path: str | Path) -> str:
    """Return the exact Git blob object ID for a workload file."""
    data = Path(path).read_bytes()
    header = f"blob {len(data)}\\0".encode("ascii")
    return hashlib.sha1(header + data).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workload", required=True)
    parser.add_argument("--truth", required=True, help="Evaluator-only truth; never expose to candidate processes")
    parser.add_argument("--video-only", required=True)
    parser.add_argument("--audio-only", required=True)
    parser.add_argument("--audio-video", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        runs = [
            read_json(args.video_only),
            read_json(args.audio_only),
            read_json(args.audio_video),
        ]
        exact_workload_ref = "git-blob-sha1:" + git_blob_sha1(args.workload)
        report = compare_arms(
            read_json(args.workload), read_json(args.truth), runs,
            workload_ref=exact_workload_ref,
        )
        output = Path(args.output)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    except (OSError, json.JSONDecodeError, EvaluationError) as error:
        print(f"ablation tournament failed: {error}", file=sys.stderr)
        return 2
    print(
        f"wrote {output}; comparison={report['comparison_id']} "
        f"arms={len(report['per_arm'])}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())