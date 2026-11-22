#!/usr/bin/env python3
"""Append-only evidence-gain ledger over multi-turn audiovisual trajectories.

This adapter reuses the #6859 claim scorer and consumes the #6753 all-action
forecast / action-value receipt. It does not recompute potential outcomes,
EVSI, counterfactual truth, or candidate decisions.

A turn's action forecast is pre-outcome and bound to that turn's transcript
head. The next turn carries the exact observations delivered after that action.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from pathlib import Path
from typing import Any

_EVIDENCE_DIR = Path(__file__).resolve().parents[1] / "av-evidence-certificate"
if str(_EVIDENCE_DIR) not in sys.path:
    sys.path.insert(0, str(_EVIDENCE_DIR))

from evaluate_evidence_certificate import (  # noqa: E402
    BENCHMARK_ID as EVIDENCE_BENCHMARK_ID,
    EvidenceEvaluationError,
    _validate_observation_catalog,
    evaluate as evaluate_evidence,
)

BENCHMARK_ID = "av-evidence-gain-trajectory-v0"
EVSI_BENCHMARK_ID = "av-action-space-evsi-v0"
STOP_ACTION_ID = "__stop__"
RESOURCE_FIELDS = (
    "new_frames",
    "video_bytes",
    "audio_samples",
    "audio_bytes",
    "input_tokens",
    "output_tokens",
    "model_calls",
    "tool_calls",
    "wall_time_ms",
    "peak_ram_mb",
    "peak_vram_mb",
)
ENERGY_PROVENANCE = {"MEASURED", "ESTIMATED", "SYNTHETIC", "NOT_MEASURED"}


class EvidenceGainError(ValueError):
    """Malformed, causally invalid or unjoinable evidence-gain trajectory."""


def _finite(value: Any) -> bool:
    return (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and math.isfinite(float(value))
    )


def _integer(value: Any, field: str, *, minimum: int = 0) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < minimum:
        raise EvidenceGainError(f"{field} must be an integer >= {minimum}")
    return value


def _nonempty(value: Any, field: str) -> str:
    if not isinstance(value, str) or not value:
        raise EvidenceGainError(f"{field} must be a non-empty string")
    return value


def _sha256(value: Any, field: str) -> str:
    if not isinstance(value, str) or len(value) != 64:
        raise EvidenceGainError(f"{field} must be a 64-character SHA-256 digest")
    try:
        int(value, 16)
    except ValueError as exc:
        raise EvidenceGainError(f"{field} must be hexadecimal") from exc
    return value.lower()


def _canonical(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")


def _strict_object(value: Any, required: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != required:
        raise EvidenceGainError(
            f"{context}: expected exactly these fields: {sorted(required)}"
        )
    return value


def _rows_by_id(rows: Any, key: str, context: str) -> dict[str, dict[str, Any]]:
    if not isinstance(rows, list):
        raise EvidenceGainError(f"{context} must be an array")
    indexed: dict[str, dict[str, Any]] = {}
    for row in rows:
        if not isinstance(row, dict):
            raise EvidenceGainError(f"{context} entries must be objects")
        identifier = _nonempty(row.get(key), f"{context}.{key}")
        if identifier in indexed:
            raise EvidenceGainError(f"{context}: duplicate {key} {identifier}")
        indexed[identifier] = row
    return indexed


def _validate_resource_delta(value: Any, context: str) -> dict[str, Any]:
    required = set(RESOURCE_FIELDS) | {
        "energy_millijoules",
        "energy_provenance",
        "energy_method",
        "energy_evidence_digest_sha256",
    }
    row = _strict_object(value, required, context)
    result: dict[str, Any] = {}
    for field in RESOURCE_FIELDS:
        metric = row[field]
        if metric is not None and (not _finite(metric) or float(metric) < 0.0):
            raise EvidenceGainError(f"{context}.{field} must be finite, non-negative or null")
        result[field] = float(metric) if metric is not None else None

    energy = row["energy_millijoules"]
    provenance = row["energy_provenance"]
    method = row["energy_method"]
    digest = row["energy_evidence_digest_sha256"]
    if not isinstance(provenance, str) or provenance not in ENERGY_PROVENANCE:
        raise EvidenceGainError(f"{context}: invalid energy_provenance")
    if energy is not None and (not _finite(energy) or float(energy) < 0.0):
        raise EvidenceGainError(f"{context}: energy_millijoules must be non-negative or null")
    if provenance == "MEASURED":
        if energy is None:
            raise EvidenceGainError(f"{context}: measured energy requires a value")
        if not isinstance(method, str) or not method:
            raise EvidenceGainError(f"{context}: measured energy requires a method")
        result["energy_evidence_digest_sha256"] = _sha256(
            digest, f"{context}.energy_evidence_digest_sha256"
        )
    else:
        if provenance == "NOT_MEASURED" and (energy is not None or method is not None):
            raise EvidenceGainError(f"{context}: NOT_MEASURED energy must have null value and method")
        if provenance in {"ESTIMATED", "SYNTHETIC"} and energy is not None:
            if not isinstance(method, str) or not method:
                raise EvidenceGainError(f"{context}: estimated/synthetic energy requires a method label")
        if digest is not None:
            raise EvidenceGainError(f"{context}: only measured energy can carry an evidence digest")
        result["energy_evidence_digest_sha256"] = None

    result.update({
        "energy_millijoules": float(energy) if energy is not None else None,
        "energy_provenance": provenance,
        "energy_method": method,
    })
    return result


def _validate_action_forecast(
    turn: dict[str, Any],
    workload: dict[str, Any],
    policy_id: str,
    turn_index: int,
) -> dict[str, Any]:
    forecast = _strict_object(
        turn["action_forecast"],
        {
            "episode_id",
            "turn_id",
            "transcript_head_sha256",
            "forecast_tick",
            "deadline_tick",
            "cost_weight",
            "budget_cost_units",
            "action_catalogue",
            "selected_action_id",
            "forecasts",
        },
        f"turn[{turn_index}].action_forecast",
    )
    turn_id = turn["turn_id"]
    head = _sha256(workload.get("transcript_head_sha256"), f"{turn_id}.workload.transcript_head_sha256")
    if forecast["episode_id"] != workload.get("episode_id"):
        raise EvidenceGainError(f"{turn_id}: action forecast episode_id mismatch")
    if forecast["turn_id"] != turn_id:
        raise EvidenceGainError(f"{turn_id}: action forecast turn_id mismatch")
    if _sha256(forecast["transcript_head_sha256"], f"{turn_id}.forecast.transcript_head_sha256") != head:
        raise EvidenceGainError(f"{turn_id}: forecast is not bound to the current transcript head")

    current_tick = _integer(workload.get("decision_cutoff_tick"), f"{turn_id}.decision_cutoff_tick")
    forecast_tick = _integer(forecast["forecast_tick"], f"{turn_id}.forecast_tick")
    if forecast_tick != current_tick:
        raise EvidenceGainError(f"{turn_id}: forecast must be committed at the current decision tick")
    deadline_tick = _integer(forecast["deadline_tick"], f"{turn_id}.deadline_tick")
    if deadline_tick < current_tick:
        raise EvidenceGainError(f"{turn_id}: deadline precedes current tick")
    cost_weight = forecast["cost_weight"]
    if not _finite(cost_weight) or float(cost_weight) < 0.0:
        raise EvidenceGainError(f"{turn_id}: cost_weight must be finite and non-negative")
    budget = forecast["budget_cost_units"]
    if not _finite(budget) or float(budget) < 0.0:
        raise EvidenceGainError(f"{turn_id}: budget_cost_units must be finite and non-negative")

    catalogue = _rows_by_id(
        forecast["action_catalogue"], "action_id", f"{turn_id}.action_catalogue"
    )
    if STOP_ACTION_ID in catalogue:
        raise EvidenceGainError(f"{turn_id}: reserved stop action appears in catalogue")
    for action_id, action in catalogue.items():
        if set(action) != {
            "action_id",
            "modality",
            "description",
            "cost_units",
            "arrival_tick",
            "source_dependency_group",
            "information_gain_proxy",
        }:
            raise EvidenceGainError(f"{turn_id}/{action_id}: action catalogue must follow the #6753 workload contract")
        if not isinstance(action["modality"], str) or action["modality"] not in {"audio", "video", "other"}:
            raise EvidenceGainError(f"{turn_id}/{action_id}: invalid modality")
        _nonempty(action["description"], f"{turn_id}/{action_id}.description")
        _nonempty(action["source_dependency_group"], f"{turn_id}/{action_id}.source_dependency_group")
        information_gain = action["information_gain_proxy"]
        if not _finite(information_gain) or float(information_gain) < 0.0:
            raise EvidenceGainError(f"{turn_id}/{action_id}: information_gain_proxy must be finite and non-negative")
        cost = action["cost_units"]
        if not _finite(cost) or float(cost) < 0.0:
            raise EvidenceGainError(f"{turn_id}/{action_id}: invalid cost_units")
        arrival = _integer(action["arrival_tick"], f"{turn_id}/{action_id}.arrival_tick")
        if arrival < current_tick:
            raise EvidenceGainError(f"{turn_id}/{action_id}: arrival tick precedes current tick")

    forecasts = _rows_by_id(
        forecast["forecasts"], "action_id", f"{turn_id}.forecasts"
    )
    expected = set(catalogue) | {STOP_ACTION_ID}
    if set(forecasts) != expected:
        raise EvidenceGainError(
            f"{turn_id}: all-action forecasts plus stop are required; "
            f"missing={sorted(expected-set(forecasts))}, extra={sorted(set(forecasts)-expected)}"
        )
    for action_id, row in forecasts.items():
        if set(row) != {
            "action_id", "estimated_gross_evsi", "lower_gross_evsi", "upper_gross_evsi"
        }:
            raise EvidenceGainError(f"{turn_id}/{action_id}: malformed forecast row")
        values = [
            row["estimated_gross_evsi"],
            row["lower_gross_evsi"],
            row["upper_gross_evsi"],
        ]
        if any(not _finite(value) for value in values):
            raise EvidenceGainError(f"{turn_id}/{action_id}: non-finite EVSI forecast")
        estimate, lower, upper = (float(value) for value in values)
        if lower > estimate or estimate > upper:
            raise EvidenceGainError(f"{turn_id}/{action_id}: invalid EVSI interval")
        if action_id == STOP_ACTION_ID and any(value != 0.0 for value in values):
            raise EvidenceGainError(f"{turn_id}: stop/no-acquisition forecast must be exactly zero")

    selected = _nonempty(forecast["selected_action_id"], f"{turn_id}.selected_action_id")
    if selected not in expected:
        raise EvidenceGainError(f"{turn_id}: selected action was not forecast")
    if selected == STOP_ACTION_ID:
        feasible = True
    else:
        action = catalogue[selected]
        feasible = (
            float(action["cost_units"]) <= float(budget)
            and int(action["arrival_tick"]) <= deadline_tick
        )
    if not feasible:
        raise EvidenceGainError(f"{turn_id}: selected action violates budget or deadline")

    receipt = _strict_object(
        turn["action_value_receipt"],
        {
            "benchmark_id",
            "episode_id",
            "turn_id",
            "policy_id",
            "run_id",
            "transcript_head_sha256",
            "forecast_digest_sha256",
            "scorecard_sha256",
            "selected_action_id",
            "selected_action_feasible",
            "missed_best_action",
            "decision_regret",
            "best_feasible_net_evsi",
        },
        f"turn[{turn_index}].action_value_receipt",
    )
    if receipt["benchmark_id"] != EVSI_BENCHMARK_ID:
        raise EvidenceGainError(f"{turn_id}: linked action-value receipt benchmark mismatch")
    if (
        receipt["episode_id"] != workload.get("episode_id")
        or receipt["turn_id"] != turn_id
        or receipt["policy_id"] != policy_id
    ):
        raise EvidenceGainError(f"{turn_id}: linked action-value receipt identity mismatch")
    _nonempty(receipt["run_id"], f"{turn_id}.action_value_receipt.run_id")
    if _sha256(receipt["transcript_head_sha256"], f"{turn_id}.receipt.transcript_head_sha256") != head:
        raise EvidenceGainError(f"{turn_id}: action-value receipt has stale transcript head")
    forecast_digest = hashlib.sha256(_canonical(forecast)).hexdigest()
    if _sha256(receipt["forecast_digest_sha256"], f"{turn_id}.receipt.forecast_digest_sha256") != forecast_digest:
        raise EvidenceGainError(f"{turn_id}: receipt is not bound to the exact action forecast")
    _sha256(receipt["scorecard_sha256"], f"{turn_id}.receipt.scorecard_sha256")
    if receipt["selected_action_id"] != selected or receipt["selected_action_feasible"] is not True:
        raise EvidenceGainError(f"{turn_id}: EVSI receipt disagrees with selected action feasibility")
    if not isinstance(receipt["missed_best_action"], bool):
        raise EvidenceGainError(f"{turn_id}: missed_best_action must be boolean")
    if not _finite(receipt["decision_regret"]) or float(receipt["decision_regret"]) < 0.0:
        raise EvidenceGainError(f"{turn_id}: decision_regret must be finite and non-negative")
    if not _finite(receipt["best_feasible_net_evsi"]):
        raise EvidenceGainError(f"{turn_id}: best_feasible_net_evsi must be finite")

    return {
        "selected_action_id": selected,
        "action_catalogue_ids": sorted(catalogue),
        "action_forecast_count": len(forecasts),
        "action_space_coverage": 1.0,
        "forecast_digest_sha256": forecast_digest,
        "action_scorecard_sha256": receipt["scorecard_sha256"],
        "decision_regret": float(receipt["decision_regret"]),
        "missed_best_action": receipt["missed_best_action"],
        "best_feasible_net_evsi": float(receipt["best_feasible_net_evsi"]),
        "selected_action_feasible": True,
    }


def _claim_metrics(scorecard: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {
        row["claim_id"]: row
        for row in scorecard["per_claim"]
    }


def _delta(after: Any, before: Any) -> float | None:
    if not _finite(after) or not _finite(before):
        return None
    return float(after) - float(before)


def _transition(
    previous_scorecard: dict[str, Any],
    current_scorecard: dict[str, Any],
    current_submission: dict[str, Any],
    new_observation_ids: list[str],
) -> dict[str, Any]:
    before = _claim_metrics(previous_scorecard)
    after = _claim_metrics(current_scorecard)
    if set(before) != set(after):
        raise EvidenceGainError("claim ID set changed across turns")
    newly_grounded = []
    lost_grounded = []
    deltas = []
    for claim_id in sorted(before):
        old = before[claim_id]
        new = after[claim_id]
        if not old["grounded_success"] and new["grounded_success"]:
            newly_grounded.append(claim_id)
        if old["grounded_success"] and not new["grounded_success"]:
            lost_grounded.append(claim_id)
        deltas.append({
            "claim_id": claim_id,
            "grounded_success_before": old["grounded_success"],
            "grounded_success_after": new["grounded_success"],
            "answer_correct_before": old["answer_correct"],
            "answer_correct_after": new["answer_correct"],
            "answer_correct_changed": old["answer_correct"] != new["answer_correct"],
            "abstained_before": old["abstained"],
            "abstained_after": new["abstained"],
            "abstention_changed": old["abstained"] != new["abstained"],
            "citation_precision_delta": _delta(new["citation_precision"], old["citation_precision"]),
            "citation_recall_delta": _delta(new["citation_recall"], old["citation_recall"]),
            "citation_f1_delta": _delta(new["citation_f1"], old["citation_f1"]),
            "temporal_iou_delta": _delta(new["temporal_iou"], old["temporal_iou"]),
            "spatial_iou_delta": _delta(new["spatial_iou"], old["spatial_iou"]),
        })

    cited = set()
    for claim in current_submission.get("claims", []):
        for entry in claim.get("evidence", []):
            cited.add(entry["observation_ref"])
    new_set = set(new_observation_ids)
    new_cited = sorted(cited & new_set)
    new_uncited = sorted(new_set - cited)
    return {
        "newly_grounded_claim_ids": newly_grounded,
        "lost_grounded_claim_ids": lost_grounded,
        "newly_grounded_claim_count": len(newly_grounded),
        "lost_grounded_claim_count": len(lost_grounded),
        "new_observations_cited": new_cited,
        "new_observations_uncited": new_uncited,
        "new_observation_count": len(new_set),
        "new_observations_cited_count": len(new_cited),
        "new_observations_uncited_count": len(new_uncited),
        "claim_deltas": deltas,
    }


def evaluate_trajectory(document: dict[str, Any]) -> dict[str, Any]:
    required = {
        "benchmark_id",
        "schema_version",
        "trajectory_id",
        "episode_id",
        "source_cluster_id",
        "policy_id",
        "model_digest_sha256",
        "runtime_digest_sha256",
        "configuration_digest_sha256",
        "hardware_digest_sha256",
        "evaluator_source_catalog",
        "truth",
        "turns",
    }
    _strict_object(document, required, "trajectory")
    if document["benchmark_id"] != BENCHMARK_ID:
        raise EvidenceGainError("benchmark_id mismatch")
    version = document["schema_version"]
    if isinstance(version, bool) or not isinstance(version, int) or version != 1:
        raise EvidenceGainError("schema_version must be integer 1")

    trajectory_id = _nonempty(document["trajectory_id"], "trajectory_id")
    episode_id = _nonempty(document["episode_id"], "episode_id")
    source_cluster_id = _nonempty(document["source_cluster_id"], "source_cluster_id")
    policy_id = _nonempty(document["policy_id"], "policy_id")
    identity_digests = {
        key: _sha256(document[key], key)
        for key in (
            "model_digest_sha256",
            "runtime_digest_sha256",
            "configuration_digest_sha256",
            "hardware_digest_sha256",
        )
    }
    truth = document["truth"]
    if not isinstance(truth, dict) or truth.get("benchmark_id") != EVIDENCE_BENCHMARK_ID:
        raise EvidenceGainError("truth must use the #6859 evidence-certificate schema")
    if truth.get("episode_id") != episode_id:
        raise EvidenceGainError("truth episode_id mismatch")

    turns = document["turns"]
    if not isinstance(turns, list) or not turns:
        raise EvidenceGainError("turns must be a non-empty array")

    # Evaluator-only complete source catalog. It may contain observations
    # that were never delivered (for example, failed acquisitions) and future
    # observations. Candidate submissions are still validated against each
    # turn's candidate-visible catalog only.
    source_rows = document["evaluator_source_catalog"]
    max_turn_cutoff = max(
        _integer(
            row.get("workload", {}).get("decision_cutoff_tick")
            if isinstance(row, dict) and isinstance(row.get("workload"), dict)
            else None,
            f"turn[{index}].workload.decision_cutoff_tick",
        )
        for index, row in enumerate(turns)
    )
    if not isinstance(source_rows, list):
        raise EvidenceGainError("evaluator_source_catalog must be an array")
    source_ticks = [max_turn_cutoff]
    for index, row in enumerate(source_rows):
        if not isinstance(row, dict):
            raise EvidenceGainError(f"evaluator_source_catalog[{index}] must be an object")
        source_ticks.append(
            _integer(row.get("available_tick"), f"evaluator_source_catalog[{index}].available_tick")
        )
        source_ticks.append(
            _integer(row.get("capture_complete_tick"), f"evaluator_source_catalog[{index}].capture_complete_tick")
        )
    source_catalog_cutoff = max(source_ticks)
    source_wrapper = {
        "benchmark_id": EVIDENCE_BENCHMARK_ID,
        "schema_version": 1,
        "episode_id": episode_id,
        "transcript_head_sha256": hashlib.sha256(
            f"{trajectory_id}:evaluator-source-catalog".encode("utf-8")
        ).hexdigest(),
        "decision_cutoff_tick": source_catalog_cutoff,
        "observation_catalog": source_rows,
    }
    full_truth_catalog = _validate_observation_catalog(source_wrapper)

    turn_results = []
    seen_turn_ids: set[str] = set()
    prior_catalog: dict[str, dict[str, Any]] | None = None
    prior_scorecard: dict[str, Any] | None = None
    prior_action: str | None = None
    prior_head: str | None = None
    prior_cutoff = -1
    seen_feedback_ids: set[str] = set()
    feedback_source_counts: dict[str, int] = {}
    feedback_kind_counts: dict[str, int] = {}
    total_feedback_events = 0
    resource_summaries = {
        field: {"observed_count": 0, "sum": 0.0}
        for field in RESOURCE_FIELDS
    }
    measured_energy_sum = 0.0
    measured_energy_count = 0
    total_regret = 0.0
    missed_best_count = 0
    total_newly_grounded = 0
    total_lost_grounded = 0
    total_uncited = 0

    for index, raw_turn in enumerate(turns):
        turn = _strict_object(
            raw_turn,
            {
                "turn_id",
                "turn_index",
                "parent_transcript_head_sha256",
                "preceding_action_id",
                "action_result_status",
                "action_result_digest_sha256",
                "new_observation_ids",
                "external_feedback_events",
                "workload",
                "submission",
                "action_forecast",
                "action_value_receipt",
                "resource_delta",
            },
            f"turn[{index}]",
        )
        turn_id = _nonempty(turn["turn_id"], f"turn[{index}].turn_id")
        if turn_id in seen_turn_ids:
            raise EvidenceGainError(f"duplicate turn_id {turn_id}")
        seen_turn_ids.add(turn_id)
        if _integer(turn["turn_index"], f"{turn_id}.turn_index") != index:
            raise EvidenceGainError(f"{turn_id}: turn_index must be contiguous and zero-based")

        preceding_action = _nonempty(
            turn["preceding_action_id"], f"{turn_id}.preceding_action_id"
        )
        status = turn["action_result_status"]
        parent_head = turn["parent_transcript_head_sha256"]
        action_result_digest = turn["action_result_digest_sha256"]
        feedback_rows = turn["external_feedback_events"]
        if not isinstance(feedback_rows, list):
            raise EvidenceGainError(f"{turn_id}: external_feedback_events must be an array")
        if index == 0:
            if preceding_action != "__initial__" or status != "INITIAL":
                raise EvidenceGainError("first turn must use __initial__ / INITIAL")
            if parent_head is not None or action_result_digest is not None or feedback_rows:
                raise EvidenceGainError("initial turn must not claim a parent head, action result or feedback event")
        else:
            if preceding_action != prior_action:
                raise EvidenceGainError(f"{turn_id}: preceding_action_id does not match prior selection")
            if status not in {"DELIVERED", "FAILED", "TIMEOUT", "MALFORMED"}:
                raise EvidenceGainError(f"{turn_id}: invalid action_result_status")
            if prior_action == STOP_ACTION_ID:
                raise EvidenceGainError("a turn follows a selected stop action")
            if _sha256(parent_head, f"{turn_id}.parent_transcript_head_sha256") != prior_head:
                raise EvidenceGainError(f"{turn_id}: parent transcript head does not match prior turn")
            action_result_digest = _sha256(
                action_result_digest, f"{turn_id}.action_result_digest_sha256"
            )

        feedback_ids: list[str] = []
        for feedback_index, feedback in enumerate(feedback_rows):
            context = f"{turn_id}.external_feedback_events[{feedback_index}]"
            feedback = _strict_object(
                feedback,
                {
                    "feedback_id",
                    "source_role",
                    "feedback_kind",
                    "delivered_tick",
                    "content_sha256",
                    "parent_transcript_head_sha256",
                },
                context,
            )
            feedback_id = _nonempty(feedback["feedback_id"], f"{context}.feedback_id")
            if feedback_id in seen_feedback_ids:
                raise EvidenceGainError(f"{turn_id}: duplicate external feedback ID {feedback_id}")
            seen_feedback_ids.add(feedback_id)
            source_role = feedback["source_role"]
            if not isinstance(source_role, str) or source_role not in {"user_simulator", "human_user", "operator"}:
                raise EvidenceGainError(f"{context}: unsupported feedback source_role")
            feedback_kind = feedback["feedback_kind"]
            if not isinstance(feedback_kind, str) or feedback_kind not in {"hint", "clarification", "correction", "challenge", "request"}:
                raise EvidenceGainError(f"{context}: unsupported feedback_kind")
            delivered_tick = _integer(feedback["delivered_tick"], f"{context}.delivered_tick")
            if delivered_tick <= prior_cutoff or delivered_tick > _integer(
                turn.get("workload", {}).get("decision_cutoff_tick")
                if isinstance(turn.get("workload"), dict) else None,
                f"{turn_id}.workload.decision_cutoff_tick",
            ):
                raise EvidenceGainError(f"{context}: feedback must arrive after prior cutoff and by current cutoff")
            if index == 0:
                raise EvidenceGainError("initial turn cannot contain an external feedback event")
            if _sha256(
                feedback["parent_transcript_head_sha256"],
                f"{context}.parent_transcript_head_sha256",
            ) != prior_head:
                raise EvidenceGainError(f"{context}: feedback must bind to the parent transcript head")
            _sha256(feedback["content_sha256"], f"{context}.content_sha256")
            feedback_ids.append(feedback_id)
            feedback_source_counts[source_role] = feedback_source_counts.get(source_role, 0) + 1
            feedback_kind_counts[feedback_kind] = feedback_kind_counts.get(feedback_kind, 0) + 1
            total_feedback_events += 1

        workload = turn["workload"]
        submission = turn["submission"]
        if not isinstance(workload, dict) or not isinstance(submission, dict):
            raise EvidenceGainError(f"{turn_id}: workload/submission must be objects")
        if (
            workload.get("benchmark_id") != EVIDENCE_BENCHMARK_ID
            or workload.get("schema_version") != 1
            or workload.get("episode_id") != episode_id
        ):
            raise EvidenceGainError(f"{turn_id}: evidence workload identity mismatch")
        if (
            submission.get("benchmark_id") != EVIDENCE_BENCHMARK_ID
            or submission.get("schema_version") != 1
            or submission.get("episode_id") != episode_id
        ):
            raise EvidenceGainError(f"{turn_id}: evidence submission identity mismatch")

        cutoff = _integer(
            workload.get("decision_cutoff_tick"), f"{turn_id}.decision_cutoff_tick"
        )
        if cutoff <= prior_cutoff:
            raise EvidenceGainError(f"{turn_id}: decision cutoffs must strictly increase")
        catalog = _validate_observation_catalog(workload)
        for observation_id, row in catalog.items():
            if full_truth_catalog.get(observation_id) != row:
                raise EvidenceGainError(
                    f"{turn_id}: delivered observation {observation_id} differs from immutable evaluator source catalog"
                )
        if prior_catalog is None:
            if turn["new_observation_ids"] != []:
                raise EvidenceGainError("initial turn must not report newly acquired observations")
            new_ids: list[str] = []
        else:
            previous_ids = set(prior_catalog)
            current_ids = set(catalog)
            removed = previous_ids - current_ids
            if removed:
                raise EvidenceGainError(f"{turn_id}: observation catalog cannot shrink")
            for observation_id in previous_ids:
                if catalog[observation_id] != prior_catalog[observation_id]:
                    raise EvidenceGainError(
                        f"{turn_id}: previously delivered observation {observation_id} was mutated"
                    )
            new_ids = sorted(current_ids - previous_ids)
            supplied_new_ids = turn["new_observation_ids"]
            if (
                not isinstance(supplied_new_ids, list)
                or any(not isinstance(value, str) for value in supplied_new_ids)
                or supplied_new_ids != new_ids
            ):
                raise EvidenceGainError(f"{turn_id}: new_observation_ids must exactly match catalog growth")
            if index > 0:
                if status == "DELIVERED" and not new_ids and not feedback_ids:
                    raise EvidenceGainError(f"{turn_id}: DELIVERED result requires a new observation or external feedback event")
                if status != "DELIVERED" and (new_ids or feedback_ids):
                    raise EvidenceGainError(f"{turn_id}: failed/timeout result cannot add observations or feedback")
                for observation_id in new_ids:
                    if catalog[observation_id]["available_tick"] <= prior_cutoff:
                        raise EvidenceGainError(
                            f"{turn_id}: new observation {observation_id} predates the preceding decision cutoff"
                        )

        if submission.get("transcript_head_sha256") != workload.get("transcript_head_sha256"):
            raise EvidenceGainError(f"{turn_id}: evidence submission is bound to a different transcript head")

        # Truth is evaluated against the immutable evaluator source catalog;
        # the candidate remains limited to this turn's delivered catalog.
        action_result = _validate_action_forecast(
            turn, workload, policy_id, index
        )
        resource = _validate_resource_delta(turn["resource_delta"], f"{turn_id}.resource_delta")
        scorecard = evaluate_evidence(
            workload,
            truth,
            submission,
            evaluator_observation_catalog=full_truth_catalog,
        )
        transition = None
        if prior_scorecard is not None:
            transition = _transition(
                prior_scorecard,
                scorecard,
                submission,
                new_ids,
            )
            total_newly_grounded += transition["newly_grounded_claim_count"]
            total_lost_grounded += transition["lost_grounded_claim_count"]
            total_uncited += transition["new_observations_uncited_count"]

        total_regret += action_result["decision_regret"]
        missed_best_count += int(action_result["missed_best_action"])
        for field in RESOURCE_FIELDS:
            value = resource[field]
            if value is not None:
                resource_summaries[field]["observed_count"] += 1
                resource_summaries[field]["sum"] += value
        if resource["energy_provenance"] == "MEASURED":
            measured_energy_sum += resource["energy_millijoules"]
            measured_energy_count += 1

        turn_results.append({
            "turn_id": turn_id,
            "turn_index": index,
            "parent_transcript_head_sha256": parent_head,
            "action_result_digest_sha256": action_result_digest,
            "external_feedback_event_ids": feedback_ids,
            "external_feedback_count": len(feedback_ids),
            "decision_cutoff_tick": cutoff,
            "preceding_action_id": preceding_action,
            "action_result_status": status,
            "new_observation_ids": new_ids,
            "selected_action_id": action_result["selected_action_id"],
            "action_forecast_count": action_result["action_forecast_count"],
            "action_space_coverage": action_result["action_space_coverage"],
            "forecast_digest_sha256": action_result["forecast_digest_sha256"],
            "action_scorecard_sha256": action_result["action_scorecard_sha256"],
            "decision_regret": action_result["decision_regret"],
            "missed_best_action": action_result["missed_best_action"],
            "best_feasible_net_evsi": action_result["best_feasible_net_evsi"],
            "resource_delta": resource,
            "claim_scorecard_sha256": scorecard["claim_scorecard_sha256"],
            "answer_accuracy": scorecard["answer_accuracy"],
            "grounded_answer_accuracy": scorecard["grounded_answer_accuracy"],
            "unsupported_claim_rate": scorecard["unsupported_claim_rate"],
            "transition_from_previous": transition,
        })

        prior_catalog = catalog
        prior_scorecard = scorecard
        prior_action = action_result["selected_action_id"]
        prior_head = _sha256(workload.get("transcript_head_sha256"), f"{turn_id}.workload.transcript_head_sha256")
        prior_cutoff = cutoff

    if prior_action != STOP_ACTION_ID:
        raise EvidenceGainError("completed trajectory must terminate with selected stop/no-acquisition")

    summarized_resources = {}
    for field, values in resource_summaries.items():
        summarized_resources[field] = {
            "observed_count": values["observed_count"],
            "observed_sum": round(values["sum"], 12),
        }
    return {
        "benchmark_id": BENCHMARK_ID,
        "schema_version": 1,
        "trajectory_id": trajectory_id,
        "episode_id": episode_id,
        "source_cluster_id": source_cluster_id,
        "policy_id": policy_id,
        **identity_digests,
        "turn_count": len(turn_results),
        "acquisition_count": max(0, len(turn_results) - 1),
        "external_feedback_event_count": total_feedback_events,
        "external_feedback_source_counts": dict(sorted(feedback_source_counts.items())),
        "external_feedback_kind_counts": dict(sorted(feedback_kind_counts.items())),
        "newly_grounded_claim_count": total_newly_grounded,
        "lost_grounded_claim_count": total_lost_grounded,
        "uncited_new_observation_count": total_uncited,
        "decision_regret_sum": round(total_regret, 12),
        "missed_best_action_count": missed_best_count,
        "resource_observed_summaries": summarized_resources,
        "measured_energy_millijoules_observed_sum": (
            round(measured_energy_sum, 12) if measured_energy_count else None
        ),
        "measured_energy_turn_count": measured_energy_count,
        "turns": turn_results,
        "ledger_sha256": hashlib.sha256(_canonical(turn_results)).hexdigest(),
        "claim_boundary": (
            "The ledger scores observable claim-certificate changes and linked "
            "all-action value receipts. Citations do not expose private reasoning "
            "or prove internal causal feature use; uncited observations are not "
            "automatically irrelevant."
        ),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    try:
        with args.input.open("r", encoding="utf-8") as source:
            document = json.load(source)
        result = evaluate_trajectory(document)
    except (OSError, json.JSONDecodeError, EvidenceEvaluationError, EvidenceGainError) as exc:
        print(json.dumps({"status": "INVALID", "error": str(exc)}), file=sys.stderr)
        raise SystemExit(2) from exc
    rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(rendered, encoding="utf-8")
    else:
        print(rendered, end="")


if __name__ == "__main__":
    main()
