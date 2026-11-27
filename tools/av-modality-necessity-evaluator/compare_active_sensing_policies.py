#!/usr/bin/env python3
"""Compare synthetic greedy-EIG and decision-value active-sensing policies."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from active_sensing_evaluate import EvaluationError, evaluate, read_json


def _summary(report: dict[str, Any]) -> dict[str, Any]:
    totals = report["totals"]
    return {
        "answer_accuracy": totals["answer_accuracy"],
        "safe_decision_accuracy": totals["safe_decision_accuracy"],
        "coverage": totals["coverage"],
        "decision_regret_total": totals["decision_regret_total"],
        "acquisition_count": totals["acquisition_count"],
        "wasted_acquisition_count": totals["wasted_acquisition_count"],
        "late_acquisition_count": totals["late_acquisition_count"],
        "budget_violation_count": totals["budget_violation_count"],
        "evidence_violation_count": totals["evidence_violation_count"],
        "mean_realized_evsi": totals["mean_realized_evsi"],
        "catalogue_audio_bytes_total": totals["catalogue_audio_bytes_total"],
        "catalogue_video_frames_total": totals["catalogue_video_frames_total"],
        "catalogue_energy_cost_total": totals["catalogue_energy_cost_total"],
        "estimated_evsi_count": totals["estimated_evsi_count"],
        "mean_estimated_evsi_mae": totals["mean_estimated_evsi_mae"],
        "mean_estimated_evsi_bias": totals["mean_estimated_evsi_bias"],
    }


def _paired(base: list[dict[str, Any]], candidate: list[dict[str, Any]], metric: str) -> dict[str, int]:
    left = {row["episode_id"]: bool(row[metric]) for row in base}
    right = {row["episode_id"]: bool(row[metric]) for row in candidate}
    if set(left) != set(right):
        raise EvaluationError("policy scorecards do not align on episode IDs")
    wins = losses = ties = 0
    for episode_id in sorted(left):
        if right[episode_id] and not left[episode_id]:
            wins += 1
        elif left[episode_id] and not right[episode_id]:
            losses += 1
        else:
            ties += 1
    return {
        "candidate_wins": wins,
        "baseline_wins": losses,
        "ties": ties,
        "episode_count": len(left),
    }


def compare(workload: dict[str, Any], truth: dict[str, Any],
            decision_value_submission: dict[str, Any],
            max_eig_submission: dict[str, Any]) -> dict[str, Any]:
    for name, submission in (
        ("decision_value", decision_value_submission),
        ("max_eig", max_eig_submission),
    ):
        if submission.get("policy_family") != ("synthetic_reference" if name == "decision_value" else "max_eig"):
            raise EvaluationError(f"{name}: policy_family does not match comparator arm")
    decision_report = evaluate(workload, truth, decision_value_submission)
    max_eig_report = evaluate(workload, truth, max_eig_submission)
    if decision_report["run_id"] == max_eig_report["run_id"]:
        raise EvaluationError("policy run IDs must be distinct")
    if decision_report["per_episode"] and max_eig_report["per_episode"]:
        if [x["episode_id"] for x in decision_report["per_episode"]] != [
            x["episode_id"] for x in max_eig_report["per_episode"]
        ]:
            raise EvaluationError("policy reports must have identical episode ordering")

    dv = _summary(decision_report)
    eig = _summary(max_eig_report)
    return {
        "schema_version": 1,
        "benchmark_id": "av-active-sensing-decision-value-v0",
        "comparison_id": "synthetic-eig-versus-evsi-v1",
        "interpretation": {
            "status": "synthetic_policy_comparison",
            "limitations": [
                "Both policies are hand-authored synthetic fixtures, not learned model runs.",
                "All action outcomes and regret values are evaluator-authored synthetic labels.",
                "Action-catalogue bytes, frames and energy costs are configured scenario estimates, not measured hardware resources.",
                "Eight public-repository episodes are not a blind or statistically powered benchmark set.",
                "This comparison demonstrates only that the evaluator distinguishes raw-EIG acquisition from decision-value acquisition on its authored counterexamples."
            ]
        },
        "per_policy": {
            "decision_value": dv,
            "max_eig": eig,
        },
        "paired": {
            "answer_correctness": _paired(
                max_eig_report["per_episode"], decision_report["per_episode"], "answer_correct"
            ),
            "safe_decision_correctness": _paired(
                max_eig_report["per_episode"], decision_report["per_episode"], "decision_correct"
            ),
        },
        "deltas_decision_value_minus_max_eig": {
            "answer_accuracy": dv["answer_accuracy"] - eig["answer_accuracy"],
            "safe_decision_accuracy": dv["safe_decision_accuracy"] - eig["safe_decision_accuracy"],
            "decision_regret_reduction": eig["decision_regret_total"] - dv["decision_regret_total"],
            "acquisitions_reduced": eig["acquisition_count"] - dv["acquisition_count"],
            "wasted_acquisitions_reduced": eig["wasted_acquisition_count"] - dv["wasted_acquisition_count"],
            "late_acquisitions_reduced": eig["late_acquisition_count"] - dv["late_acquisition_count"],
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workload", required=True)
    parser.add_argument("--truth", required=True)
    parser.add_argument("--decision-value", required=True)
    parser.add_argument("--max-eig", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        result = compare(
            read_json(args.workload), read_json(args.truth),
            read_json(args.decision_value), read_json(args.max_eig),
        )
        output = Path(args.output)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    except (OSError, json.JSONDecodeError, EvaluationError) as error:
        print(f"active sensing policy comparison failed: {error}", file=sys.stderr)
        return 2
    print(f"wrote {output}; regret reduction={result['deltas_decision_value_minus_max_eig']['decision_regret_reduction']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())