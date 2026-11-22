#!/usr/bin/env python3
"""Claim-level AV evidence certificate evaluator (synthetic-first benchmark v0).

Evaluates whether each answer/abstention is supported by the exact audiovisual
observations delivered before its decision cutoff. Temporal intervals are
scored within an observation's declared clock domain; this evaluator does not
perform clock transforms or infer unsupported 3D localization.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from pathlib import Path
from typing import Any

BENCHMARK_ID = "av-evidence-certificate-v0"
SCHEMA_VERSION = 1


class EvidenceEvaluationError(ValueError):
    """Malformed or causally invalid benchmark input."""


def _is_int(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _finite_number(value: Any) -> bool:
    return (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and math.isfinite(float(value))
    )


def _sha256(value: Any, field: str) -> str:
    if not isinstance(value, str) or len(value) != 64:
        raise EvidenceEvaluationError(f"{field} must be a 64-character SHA-256 digest")
    try:
        int(value, 16)
    except ValueError as exc:
        raise EvidenceEvaluationError(f"{field} must be hexadecimal") from exc
    return value.lower()


def _identifier(value: Any, field: str) -> str:
    if not isinstance(value, str) or not value:
        raise EvidenceEvaluationError(f"{field} must be a non-empty string")
    return value


def _bbox(value: Any, field: str) -> tuple[float, float, float, float]:
    if not isinstance(value, list) or len(value) != 4:
        raise EvidenceEvaluationError(f"{field} must be [x1,y1,x2,y2]")
    if any(not _finite_number(v) for v in value):
        raise EvidenceEvaluationError(f"{field} must contain finite numbers")
    x1, y1, x2, y2 = (float(v) for v in value)
    if not (0.0 <= x1 < x2 <= 1.0 and 0.0 <= y1 < y2 <= 1.0):
        raise EvidenceEvaluationError(f"{field} must be a non-empty normalized box in [0,1]")
    return x1, y1, x2, y2


def _validate_header(document: dict[str, Any], expected_episode_id: str | None = None) -> None:
    if not isinstance(document, dict):
        raise EvidenceEvaluationError("document root must be an object")
    if document.get("benchmark_id") != BENCHMARK_ID:
        raise EvidenceEvaluationError("benchmark_id mismatch")
    version = document.get("schema_version")
    if not _is_int(version) or version != SCHEMA_VERSION:
        raise EvidenceEvaluationError("schema_version must be integer 1")
    episode_id = _identifier(document.get("episode_id"), "episode_id")
    if expected_episode_id is not None and episode_id != expected_episode_id:
        raise EvidenceEvaluationError("episode_id mismatch across workload, truth and submission")


def _validate_observation_catalog(workload: dict[str, Any]) -> dict[str, dict[str, Any]]:
    cutoff = workload.get("decision_cutoff_tick")
    if not _is_int(cutoff) or cutoff < 0:
        raise EvidenceEvaluationError("decision_cutoff_tick must be a non-negative integer")
    rows = workload.get("observation_catalog")
    if not isinstance(rows, list):
        raise EvidenceEvaluationError("observation_catalog must be an array")
    catalog: dict[str, dict[str, Any]] = {}
    for index, row in enumerate(rows):
        if not isinstance(row, dict):
            raise EvidenceEvaluationError(f"observation {index} must be an object")
        required = {
            "observation_id",
            "modality",
            "available_tick",
            "capture_complete_tick",
            "capture_start_ms",
            "capture_end_ms",
            "clock_domain_id",
            "clock_generation",
            "artifact_sha256",
        }
        if set(row) != required:
            raise EvidenceEvaluationError(f"observation {index} has missing or unknown fields")
        observation_id = _identifier(row["observation_id"], "observation_id")
        if observation_id in catalog:
            raise EvidenceEvaluationError(f"duplicate observation_id: {observation_id}")
        modality = row["modality"]
        if not isinstance(modality, str) or modality not in {"audio", "video"}:
            raise EvidenceEvaluationError(f"{observation_id}: modality must be audio or video")
        for field in ("available_tick", "capture_complete_tick", "clock_generation"):
            if not _is_int(row[field]) or row[field] < 0:
                raise EvidenceEvaluationError(f"{observation_id}: {field} must be a non-negative integer")
        if row["available_tick"] > cutoff or row["capture_complete_tick"] > cutoff:
            raise EvidenceEvaluationError(f"{observation_id}: observation was not complete and delivered by cutoff")
        if not _finite_number(row["capture_start_ms"]) or not _finite_number(row["capture_end_ms"]):
            raise EvidenceEvaluationError(f"{observation_id}: capture interval must be finite")
        start = float(row["capture_start_ms"])
        end = float(row["capture_end_ms"])
        if start < 0.0 or end <= start:
            raise EvidenceEvaluationError(f"{observation_id}: invalid capture interval")
        clock_domain = _identifier(row["clock_domain_id"], "clock_domain_id")
        digest = _sha256(row["artifact_sha256"], "artifact_sha256")
        catalog[observation_id] = {
            "observation_id": observation_id,
            "modality": modality,
            "available_tick": row["available_tick"],
            "capture_complete_tick": row["capture_complete_tick"],
            "capture_start_ms": start,
            "capture_end_ms": end,
            "clock_domain_id": clock_domain,
            "clock_generation": row["clock_generation"],
            "artifact_sha256": digest,
        }
    return catalog


def _validate_evidence_entries(
    evidence: Any,
    catalog: dict[str, dict[str, Any]],
    cutoff: int,
    *,
    source: str,
) -> list[dict[str, Any]]:
    if not isinstance(evidence, list):
        raise EvidenceEvaluationError(f"{source}: evidence must be an array")
    validated: list[dict[str, Any]] = []
    seen: set[tuple[Any, ...]] = set()
    for index, entry in enumerate(evidence):
        if not isinstance(entry, dict):
            raise EvidenceEvaluationError(f"{source}: evidence entry {index} must be an object")
        allowed = {"observation_ref", "start_ms", "end_ms", "frame_id", "bbox_xyxy"}
        if set(entry) - allowed or not {"observation_ref", "start_ms", "end_ms"}.issubset(entry):
            raise EvidenceEvaluationError(f"{source}: evidence entry {index} has missing or unknown fields")
        ref = _identifier(entry["observation_ref"], "observation_ref")
        if ref not in catalog:
            raise EvidenceEvaluationError(f"{source}: reference to unknown/unobserved observation {ref}")
        observation = catalog[ref]
        if observation["available_tick"] > cutoff or observation["capture_complete_tick"] > cutoff:
            raise EvidenceEvaluationError(f"{source}: future or undelivered evidence reference {ref}")
        if not _finite_number(entry["start_ms"]) or not _finite_number(entry["end_ms"]):
            raise EvidenceEvaluationError(f"{source}: evidence interval must be finite")
        start = float(entry["start_ms"])
        end = float(entry["end_ms"])
        if (
            start < observation["capture_start_ms"]
            or end > observation["capture_end_ms"]
            or end <= start
        ):
            raise EvidenceEvaluationError(f"{source}: evidence interval falls outside observation {ref}")
        frame_id = entry.get("frame_id")
        box = entry.get("bbox_xyxy")
        if box is not None:
            if observation["modality"] != "video":
                raise EvidenceEvaluationError(f"{source}: spatial boxes require video observations")
            frame_id = _identifier(frame_id, "frame_id")
            box = _bbox(box, "bbox_xyxy")
        elif frame_id is not None:
            frame_id = _identifier(frame_id, "frame_id")
        key = (ref, start, end, frame_id, box)
        if key in seen:
            raise EvidenceEvaluationError(f"{source}: duplicate evidence entry")
        seen.add(key)
        validated.append({
            "observation_ref": ref,
            "start_ms": start,
            "end_ms": end,
            "frame_id": frame_id,
            "bbox_xyxy": box,
        })
    return validated


def _interval_union(intervals: list[tuple[float, float]]) -> list[tuple[float, float]]:
    if not intervals:
        return []
    ordered = sorted(intervals)
    merged = [ordered[0]]
    for start, end in ordered[1:]:
        last_start, last_end = merged[-1]
        if start <= last_end:
            merged[-1] = (last_start, max(last_end, end))
        else:
            merged.append((start, end))
    return merged


def _union_length(intervals: list[tuple[float, float]]) -> float:
    return sum(end - start for start, end in _interval_union(intervals))


def _intersection_length(
    left: list[tuple[float, float]],
    right: list[tuple[float, float]],
) -> float:
    a = _interval_union(left)
    b = _interval_union(right)
    i = j = 0
    total = 0.0
    while i < len(a) and j < len(b):
        start = max(a[i][0], b[j][0])
        end = min(a[i][1], b[j][1])
        if end > start:
            total += end - start
        if a[i][1] <= b[j][1]:
            i += 1
        else:
            j += 1
    return total


def _temporal_iou(
    predicted: list[dict[str, Any]],
    truth: list[dict[str, Any]],
) -> float | None:
    if not truth:
        return None
    refs = {
        entry["observation_ref"] for entry in predicted
    } | {
        entry["observation_ref"] for entry in truth
    }
    intersection = 0.0
    union = 0.0
    for ref in refs:
        pred_intervals = [
            (row["start_ms"], row["end_ms"])
            for row in predicted if row["observation_ref"] == ref
        ]
        truth_intervals = [
            (row["start_ms"], row["end_ms"])
            for row in truth if row["observation_ref"] == ref
        ]
        intersection += _intersection_length(pred_intervals, truth_intervals)
        union += (
            _union_length(pred_intervals)
            + _union_length(truth_intervals)
            - _intersection_length(pred_intervals, truth_intervals)
        )
    return 1.0 if union == 0.0 else intersection / union


def _bbox_iou(
    left: tuple[float, float, float, float],
    right: tuple[float, float, float, float],
) -> float:
    x1 = max(left[0], right[0])
    y1 = max(left[1], right[1])
    x2 = min(left[2], right[2])
    y2 = min(left[3], right[3])
    intersection = max(0.0, x2 - x1) * max(0.0, y2 - y1)
    area_left = (left[2] - left[0]) * (left[3] - left[1])
    area_right = (right[2] - right[0]) * (right[3] - right[1])
    union = area_left + area_right - intersection
    return 0.0 if union <= 0.0 else intersection / union


def _spatial_iou(
    predicted: list[dict[str, Any]],
    truth: list[dict[str, Any]],
) -> float | None:
    truth_boxes = [row for row in truth if row.get("bbox_xyxy") is not None]
    predicted_boxes = [row for row in predicted if row.get("bbox_xyxy") is not None]
    if not truth_boxes:
        return None
    scores = []
    used_predicted: set[int] = set()
    for target in truth_boxes:
        candidates = [
            (index, row)
            for index, row in enumerate(predicted_boxes)
            if index not in used_predicted
            and row["observation_ref"] == target["observation_ref"]
            and row["frame_id"] == target["frame_id"]
        ]
        if not candidates:
            scores.append(0.0)
            continue
        best_index, best_row = max(
            candidates,
            key=lambda pair: _bbox_iou(pair[1]["bbox_xyxy"], target["bbox_xyxy"]),
        )
        used_predicted.add(best_index)
        scores.append(_bbox_iou(best_row["bbox_xyxy"], target["bbox_xyxy"]))
    denominator = max(len(truth_boxes), len(predicted_boxes), 1)
    return sum(scores) / denominator


def _truth_claims(
    truth: dict[str, Any],
    catalog: dict[str, dict[str, Any]],
) -> dict[str, dict[str, Any]]:
    claims = truth.get("claims")
    if not isinstance(claims, list):
        raise EvidenceEvaluationError("truth.claims must be an array")
    indexed: dict[str, dict[str, Any]] = {}
    for index, claim in enumerate(claims):
        if not isinstance(claim, dict):
            raise EvidenceEvaluationError(f"truth claim {index} must be an object")
        claim_id = _identifier(claim.get("claim_id"), "claim_id")
        if claim_id in indexed:
            raise EvidenceEvaluationError(f"duplicate truth claim_id: {claim_id}")
        answerable = claim.get("answerable")
        if not isinstance(answerable, bool):
            raise EvidenceEvaluationError(f"{claim_id}: answerable must be boolean")
        acceptable = claim.get("acceptable_answer_values")
        if not isinstance(acceptable, list):
            raise EvidenceEvaluationError(f"{claim_id}: acceptable_answer_values must be an array")
        if answerable and not acceptable:
            raise EvidenceEvaluationError(f"{claim_id}: answerable claim requires an acceptable answer")
        if not answerable and acceptable:
            raise EvidenceEvaluationError(f"{claim_id}: unanswerable claim must not define answer values")
        groups = claim.get("minimal_sufficient_evidence_sets")
        if not isinstance(groups, list):
            raise EvidenceEvaluationError(f"{claim_id}: minimal_sufficient_evidence_sets must be an array")
        if answerable and not groups:
            raise EvidenceEvaluationError(f"{claim_id}: answerable claim requires at least one sufficient evidence set")
        normalized_groups = []
        for group in groups:
            if not isinstance(group, list) or not group:
                raise EvidenceEvaluationError(f"{claim_id}: evidence groups must be non-empty arrays")
            if any(not isinstance(ref, str) or ref not in catalog for ref in group):
                raise EvidenceEvaluationError(f"{claim_id}: evidence group references an unknown observation")
            if len(set(group)) != len(group):
                raise EvidenceEvaluationError(f"{claim_id}: evidence group contains duplicate observations")
            normalized_groups.append(set(group))
        spans = claim.get("supporting_spans")
        boxes = claim.get("supporting_boxes")
        if not isinstance(spans, list) or not isinstance(boxes, list):
            raise EvidenceEvaluationError(f"{claim_id}: supporting_spans and supporting_boxes must be arrays")
        validated_spans = _validate_evidence_entries(
            spans, catalog, max((row["available_tick"] for row in catalog.values()), default=0),
            source=f"truth.{claim_id}.supporting_spans",
        )
        validated_boxes = _validate_evidence_entries(
            boxes, catalog, max((row["available_tick"] for row in catalog.values()), default=0),
            source=f"truth.{claim_id}.supporting_boxes",
        )
        if any(row["bbox_xyxy"] is None for row in validated_boxes):
            raise EvidenceEvaluationError(f"{claim_id}: supporting_boxes entries require bbox_xyxy")
        temporal_threshold = claim.get("temporal_iou_threshold", 0.3)
        spatial_threshold = claim.get("spatial_iou_threshold", 0.3)
        for field, value in (
            ("temporal_iou_threshold", temporal_threshold),
            ("spatial_iou_threshold", spatial_threshold),
        ):
            if not _finite_number(value) or not 0.0 <= value <= 1.0:
                raise EvidenceEvaluationError(f"{claim_id}: {field} must be in [0,1]")
        indexed[claim_id] = {
            "claim_id": claim_id,
            "answerable": answerable,
            "acceptable_answer_values": acceptable,
            "minimal_sufficient_evidence_sets": normalized_groups,
            "supporting_spans": validated_spans,
            "supporting_boxes": validated_boxes,
            "temporal_iou_threshold": float(temporal_threshold),
            "spatial_iou_threshold": float(spatial_threshold),
        }
    return indexed


def _score_claim(
    candidate: dict[str, Any],
    truth: dict[str, Any],
    catalog: dict[str, dict[str, Any]],
    cutoff: int,
) -> dict[str, Any]:
    claim_id = truth["claim_id"]
    abstained = candidate.get("abstained")
    if not isinstance(abstained, bool):
        raise EvidenceEvaluationError(f"{claim_id}: abstained must be boolean")
    answer_value = candidate.get("answer_value")
    evidence = _validate_evidence_entries(
        candidate.get("evidence"), catalog, cutoff, source=f"submission.{claim_id}"
    )
    if abstained and (answer_value is not None or evidence):
        raise EvidenceEvaluationError(f"{claim_id}: abstention must not include an answer or evidence")
    if not abstained and answer_value is None:
        raise EvidenceEvaluationError(f"{claim_id}: non-abstained claim requires answer_value")

    predicted_refs = {row["observation_ref"] for row in evidence}
    sufficient_sets = truth["minimal_sufficient_evidence_sets"]
    if sufficient_sets:
        best_group = max(
            sufficient_sets,
            key=lambda group: (
                len(predicted_refs & group) / len(group),
                -len(group),
            ),
        )
        citation_recall = len(predicted_refs & best_group) / len(best_group)
        positive_refs = set().union(*sufficient_sets)
    else:
        best_group = set()
        citation_recall = 0.0
        positive_refs = set()
    citation_precision = (
        len(predicted_refs & positive_refs) / len(predicted_refs)
        if predicted_refs else 0.0
    )
    citation_f1 = (
        2.0 * citation_precision * citation_recall / (citation_precision + citation_recall)
        if citation_precision + citation_recall else 0.0
    )
    evidence_sufficient = any(group.issubset(predicted_refs) for group in sufficient_sets)
    chosen_group = next(
        (group for group in sorted(sufficient_sets, key=len) if group.issubset(predicted_refs)),
        best_group,
    )
    unnecessary_refs = sorted(predicted_refs - chosen_group)

    answer_correct = (
        not abstained and answer_value in truth["acceptable_answer_values"]
        if truth["answerable"]
        else abstained
    )
    temporal_iou = _temporal_iou(evidence, truth["supporting_spans"])
    spatial_iou = _spatial_iou(evidence, truth["supporting_boxes"])
    temporal_pass = temporal_iou is None or temporal_iou >= truth["temporal_iou_threshold"]
    spatial_pass = spatial_iou is None or spatial_iou >= truth["spatial_iou_threshold"]
    grounded_success = (
        answer_correct
        and (
            (not truth["answerable"])
            or (evidence_sufficient and temporal_pass and spatial_pass)
        )
    )
    unsupported_claim = not abstained and (
        not answer_correct or not evidence_sufficient
    )
    return {
        "claim_id": claim_id,
        "answerable": truth["answerable"],
        "abstained": abstained,
        "answer_correct": answer_correct,
        "citation_precision": citation_precision,
        "citation_recall": citation_recall,
        "citation_f1": citation_f1,
        "evidence_sufficient": evidence_sufficient,
        "temporal_iou": temporal_iou,
        "temporal_iou_pass": temporal_pass,
        "spatial_iou": spatial_iou,
        "spatial_iou_pass": spatial_pass,
        "unnecessary_citations": unnecessary_refs,
        "unsupported_claim": unsupported_claim,
        "grounded_success": grounded_success,
    }


def evaluate(
    workload: dict[str, Any],
    truth_document: dict[str, Any],
    submission: dict[str, Any],
    *,
    evaluator_observation_catalog: dict[str, dict[str, Any]] | None = None,
) -> dict[str, Any]:
    _validate_header(workload)
    episode_id = workload["episode_id"]
    _validate_header(truth_document, episode_id)
    _validate_header(submission, episode_id)

    workload_allowed = {
        "benchmark_id", "schema_version", "episode_id",
        "transcript_head_sha256", "decision_cutoff_tick", "observation_catalog",
    }
    if set(workload) != workload_allowed:
        raise EvidenceEvaluationError("workload has missing or unknown fields")
    submission_allowed = {
        "benchmark_id", "schema_version", "episode_id",
        "transcript_head_sha256", "claims",
    }
    if set(submission) != submission_allowed:
        raise EvidenceEvaluationError("submission has missing or unknown fields")

    transcript_head = _sha256(workload.get("transcript_head_sha256"), "workload.transcript_head_sha256")
    if _sha256(submission.get("transcript_head_sha256"), "submission.transcript_head_sha256") != transcript_head:
        raise EvidenceEvaluationError("submission transcript head does not match candidate-visible workload")

    catalog = _validate_observation_catalog(workload)
    cutoff = workload["decision_cutoff_tick"]
    # Counterfactual variants may remove candidate-visible observations while
    # evaluator truth must continue to reference the immutable original source
    # catalog. The optional catalog is supplied only by the counterfactual
    # harness after it validates that the variant changes only its declared
    # observation set.
    truth_catalog = evaluator_observation_catalog or catalog
    truth_claim_index = _truth_claims(truth_document, truth_catalog)

    candidate_claims = submission.get("claims")
    if not isinstance(candidate_claims, list):
        raise EvidenceEvaluationError("submission.claims must be an array")
    candidate_index: dict[str, dict[str, Any]] = {}
    for claim in candidate_claims:
        if not isinstance(claim, dict):
            raise EvidenceEvaluationError("candidate claim must be an object")
        claim_id = _identifier(claim.get("claim_id"), "claim_id")
        if set(claim) != {"claim_id", "answer_value", "abstained", "evidence"}:
            raise EvidenceEvaluationError(f"{claim_id}: candidate claim has missing or unknown fields")
        if claim_id in candidate_index:
            raise EvidenceEvaluationError(f"duplicate candidate claim_id: {claim_id}")
        candidate_index[claim_id] = claim
    if set(candidate_index) != set(truth_claim_index):
        raise EvidenceEvaluationError("candidate claims must exactly cover the evaluator claim set")

    per_claim = [
        _score_claim(candidate_index[claim_id], claim_truth, catalog, cutoff)
        for claim_id, claim_truth in sorted(truth_claim_index.items())
    ]
    total = len(per_claim)
    if total == 0:
        raise EvidenceEvaluationError("at least one claim is required")

    def mean(values: list[float]) -> float | None:
        return sum(values) / len(values) if values else None

    temporal_values = [row["temporal_iou"] for row in per_claim if row["temporal_iou"] is not None]
    spatial_values = [row["spatial_iou"] for row in per_claim if row["spatial_iou"] is not None]
    answer_correct = sum(bool(row["answer_correct"]) for row in per_claim)
    grounded_success = sum(bool(row["grounded_success"]) for row in per_claim)
    unsupported = sum(bool(row["unsupported_claim"]) for row in per_claim)
    return {
        "benchmark_id": BENCHMARK_ID,
        "schema_version": SCHEMA_VERSION,
        "episode_id": episode_id,
        "transcript_head_sha256": transcript_head,
        "decision_cutoff_tick": cutoff,
        "observation_count": len(catalog),
        "claim_count": total,
        "answer_accuracy": answer_correct / total,
        "grounded_answer_accuracy": grounded_success / total,
        "unsupported_claim_rate": unsupported / total,
        "mean_citation_precision": mean([row["citation_precision"] for row in per_claim]),
        "mean_citation_recall": mean([row["citation_recall"] for row in per_claim]),
        "mean_citation_f1": mean([row["citation_f1"] for row in per_claim]),
        "mean_temporal_iou": mean(temporal_values),
        "mean_spatial_iou": mean(spatial_values),
        "correct_abstention_count": sum(
            1 for row in per_claim if row["answerable"] is False and row["abstained"] and row["answer_correct"]
        ),
        "per_claim": per_claim,
        "claim_scorecard_sha256": hashlib.sha256(
            json.dumps(per_claim, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
        ).hexdigest(),
        "claim_boundary": (
            "This evaluator scores declared claim references and annotations. "
            "It does not prove causality, sensor calibration, annotation correctness or blind custody."
        ),
    }


def read_json(path: str | Path) -> dict[str, Any]:
    with Path(path).open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise EvidenceEvaluationError(f"{path}: expected JSON object")
    return value


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workload", required=True, type=Path)
    parser.add_argument("--truth", required=True, type=Path)
    parser.add_argument("--submission", required=True, type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    try:
        result = evaluate(
            read_json(args.workload),
            read_json(args.truth),
            read_json(args.submission),
        )
    except (OSError, json.JSONDecodeError, EvidenceEvaluationError) as exc:
        print(json.dumps({"status": "INVALID", "error": str(exc)}), file=sys.stderr)
        raise SystemExit(2) from exc
    rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(rendered, encoding="utf-8")
    else:
        print(rendered, end="")


if __name__ == "__main__":
    main()
