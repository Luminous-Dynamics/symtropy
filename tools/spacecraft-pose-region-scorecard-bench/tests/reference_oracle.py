#!/usr/bin/env python3
"""Independent Python oracle for the spacecraft pose region scorecard."""

from __future__ import annotations

import csv
import hashlib
import json
import math
import sys
from collections import defaultdict
from pathlib import Path

SCHEMA = "luminous.spacecraft-pose-region-scorecard.receipt.v0.1"
LEVELS = (50, 80, 90, 95, 99)
MAX_ANGLE_URAD = 3_141_593
NORM_TOLERANCE = 1.0e-6


def result_base(row: dict[str, str], disposition: str, reason: str | None) -> dict[str, object]:
    return {
        "case_id": row["case_id"],
        "sample_id": row["sample_id"],
        "target_id": row["target_id"],
        "split": row["split"],
        "crop_stratum": row["crop_stratum"],
        "visible_parts": row["visible_parts"],
        "modality": row["modality"],
        "ood_group": row["ood_group"],
        "calibration_revision": row["calibration_revision"],
        "frame_id": row["frame_id"],
        "disposition": disposition,
        "reason_codes": [] if reason is None else [reason],
        "translation_error_l2_um": None,
        "orientation_error_urad": None,
        "regions": [],
    }


def canonical_quaternion(row: dict[str, str], prefix: str) -> list[float] | None:
    values = [float(row[f"{prefix}_q{i}"]) for i in range(4)]
    if not all(math.isfinite(value) for value in values):
        return None
    if row["quaternion_order"] == "XYZW":
        q = values
    elif row["quaternion_order"] == "WXYZ":
        q = [values[1], values[2], values[3], values[0]]
    else:
        return None
    norm = math.sqrt(sum(value * value for value in q))
    if not math.isfinite(norm) or norm <= 1.0e-12 or abs(norm - 1.0) > NORM_TOLERANCE:
        return None
    return [value / norm for value in q]


def orientation_error_urad(predicted: list[float], truth: list[float]) -> int | None:
    dot = abs(sum(left * right for left, right in zip(predicted, truth, strict=True)))
    dot = min(1.0, max(0.0, dot))
    angle = 2.0 * math.acos(dot)
    scaled = math.ceil(angle * 1_000_000.0)
    if scaled < 0 or scaled > MAX_ANGLE_URAD:
        return None
    return scaled


def validate_identity(row: dict[str, str]) -> str | None:
    required = (
        "case_id",
        "sample_id",
        "target_id",
        "split",
        "crop_stratum",
        "visible_parts",
        "modality",
        "ood_group",
        "calibration_revision",
        "frame_id",
        "pose_semantics",
    )
    if any(not row[field].strip() for field in required):
        return "IDENTITY_OR_GROUP_METADATA_MISSING"
    if row["split"] not in ("train", "validation", "test"):
        return "UNKNOWN_SPLIT"
    if row["translation_unit"] != "micrometre":
        return "UNSUPPORTED_TRANSLATION_UNIT"
    if row["quaternion_order"] not in ("XYZW", "WXYZ"):
        return "UNSUPPORTED_QUATERNION_ORDER"
    return None


def evaluate(row: dict[str, str]) -> dict[str, object]:
    reason = validate_identity(row)
    if reason is not None:
        return result_base(row, "INVALID_INPUT", reason)

    status = row["status"]
    if status == "ABSTAINED":
        return result_base(row, "ABSTAINED", "PREDICTOR_ABSTAINED")
    if status == "MISSING":
        return result_base(row, "MISSING", "PREDICTION_MISSING")
    if status != "ACCEPTED":
        return result_base(row, "INVALID_INPUT", "UNKNOWN_PREDICTION_STATUS")

    predicted_translation = [int(row[f"pred_t{axis}_um"]) for axis in "xyz"]
    truth_translation = [int(row[f"truth_t{axis}_um"]) for axis in "xyz"]
    predicted_q = canonical_quaternion(row, "pred")
    truth_q = canonical_quaternion(row, "truth")
    if predicted_q is None:
        return result_base(row, "INVALID_INPUT", "PREDICTED_QUATERNION_INVALID")
    if truth_q is None:
        return result_base(row, "INVALID_INPUT", "TRUTH_QUATERNION_INVALID")

    angular_error = orientation_error_urad(predicted_q, truth_q)
    if angular_error is None:
        return result_base(row, "INVALID_INPUT", "ORIENTATION_ERROR_UNREPRESENTABLE")

    squared_translation_error = sum(
        (predicted_translation[axis] - truth_translation[axis]) ** 2
        for axis in range(3)
    )
    translation_error = math.isqrt(squared_translation_error)
    if translation_error * translation_error < squared_translation_error:
        translation_error += 1
    if translation_error > (1 << 64) - 1:
        return result_base(row, "INVALID_INPUT", "TRANSLATION_ERROR_UNREPRESENTABLE")

    region_scores = []
    previous: dict[str, object] | None = None
    for index, nominal in enumerate(LEVELS):
        prefix = f"l{nominal}_"
        lower = [int(row[f"{prefix}lower_{axis}"]) for axis in "xyz"]
        upper = [int(row[f"{prefix}upper_{axis}"]) for axis in "xyz"]
        radius = int(row[f"{prefix}orientation_radius_urad"])

        if radius > MAX_ANGLE_URAD:
            return result_base(row, "INVALID_INPUT", "ORIENTATION_REGION_RADIUS_OUT_OF_RANGE")

        widths = []
        axis_coverage = []
        for axis in range(3):
            if lower[axis] > upper[axis]:
                return result_base(row, "INVALID_INPUT", "TRANSLATION_REGION_BOUNDS_REVERSED")
            if not (lower[axis] <= predicted_translation[axis] <= upper[axis]):
                return result_base(row, "INVALID_INPUT", "PREDICTION_CENTER_OUTSIDE_DECLARED_REGION")
            widths.append(upper[axis] - lower[axis])
            axis_coverage.append(lower[axis] <= truth_translation[axis] <= upper[axis])

        if previous is not None:
            for axis in range(3):
                if lower[axis] > previous["lower"][axis] or upper[axis] < previous["upper"][axis]:
                    return result_base(row, "INVALID_INPUT", "POSE_REGIONS_NOT_NESTED")
            if radius < previous["radius"]:
                return result_base(row, "INVALID_INPUT", "POSE_REGIONS_NOT_NESTED")

        volume = widths[0] * widths[1] * widths[2]
        if volume > (1 << 128) - 1:
            return result_base(row, "INVALID_INPUT", "TRANSLATION_REGION_VOLUME_OVERFLOW")

        translation_joint = all(axis_coverage)
        orientation_covered = angular_error <= radius
        region_scores.append(
            {
                "nominal_percent": nominal,
                "translation_axis_covered": axis_coverage,
                "translation_joint_covered": translation_joint,
                "orientation_covered": orientation_covered,
                "joint_covered": translation_joint and orientation_covered,
                "orientation_error_urad": angular_error,
                "width_um": widths,
                "box_volume_um3": volume,
                "orientation_radius_urad": radius,
            }
        )
        previous = {"lower": lower, "upper": upper, "radius": radius}

    result = result_base(row, "SCORED", None)
    result["translation_error_l2_um"] = translation_error
    result["orientation_error_urad"] = angular_error
    result["regions"] = region_scores
    return result


def empty_aggregate(level: int) -> dict[str, object]:
    return {
        "nominal_percent": level,
        "accepted_count": 0,
        "translation_axis_covered": [0, 0, 0],
        "translation_joint_covered": 0,
        "orientation_covered": 0,
        "joint_covered": 0,
        "false_confident": 0,
        "sum_width_um": [0, 0, 0],
        "sum_box_volume_um3": 0,
        "sum_orientation_radius_urad": 0,
    }


def aggregate(rows: list[dict[str, object]]) -> list[dict[str, object]]:
    levels = [empty_aggregate(level) for level in LEVELS]
    for row in rows:
        if row["disposition"] != "SCORED":
            continue
        for index, region in enumerate(row["regions"]):
            entry = levels[index]
            entry["accepted_count"] += 1
            for axis in range(3):
                if region["translation_axis_covered"][axis]:
                    entry["translation_axis_covered"][axis] += 1
                entry["sum_width_um"][axis] += region["width_um"][axis]
            if region["translation_joint_covered"]:
                entry["translation_joint_covered"] += 1
            if region["orientation_covered"]:
                entry["orientation_covered"] += 1
            if region["joint_covered"]:
                entry["joint_covered"] += 1
            else:
                entry["false_confident"] += 1
            entry["sum_box_volume_um3"] += region["box_volume_um3"]
            entry["sum_orientation_radius_urad"] += region["orientation_radius_urad"]
    return levels


def evaluate_manifest(rows: list[dict[str, str]]) -> dict[str, object]:
    target_splits: dict[str, set[str]] = defaultdict(set)
    seen_samples: set[str] = set()
    duplicate_samples: set[str] = set()

    for row in rows:
        target_splits[row["target_id"]].add(row["split"])
        if row["sample_id"] in seen_samples:
            duplicate_samples.add(row["sample_id"])
        seen_samples.add(row["sample_id"])

    leaked_targets = {
        target_id for target_id, splits in target_splits.items() if len(splits) > 1
    }

    results = []
    for row in rows:
        if row["target_id"] in leaked_targets:
            score = result_base(row, "SPLIT_LEAKAGE", "TARGET_ID_PRESENT_IN_MULTIPLE_SPLITS")
        elif row["sample_id"] in duplicate_samples:
            score = result_base(row, "DUPLICATE_SAMPLE", "SAMPLE_ID_DUPLICATED")
        else:
            score = evaluate(row)
        results.append(score)
    results.sort(key=lambda item: item["sample_id"])

    grouped: dict[tuple[str, str, str, str], list[dict[str, object]]] = defaultdict(list)
    for score in results:
        key = (
            score["target_id"],
            score["crop_stratum"],
            score["modality"],
            score["ood_group"],
        )
        grouped[key].append(score)

    groups = []
    for key, group_rows in sorted(grouped.items()):
        groups.append(
            {
                "target_id": key[0],
                "crop_stratum": key[1],
                "modality": key[2],
                "ood_group": key[3],
                "sample_count": len(group_rows),
                "accepted_count": sum(row["disposition"] == "SCORED" for row in group_rows),
                "abstained_count": sum(row["disposition"] == "ABSTAINED" for row in group_rows),
                "missing_count": sum(row["disposition"] == "MISSING" for row in group_rows),
                "invalid_count": sum(row["disposition"] not in ("SCORED", "ABSTAINED", "MISSING") for row in group_rows),
                "sum_translation_error_l2_um": sum(row["translation_error_l2_um"] or 0 for row in group_rows),
                "max_translation_error_l2_um": max((row["translation_error_l2_um"] or 0 for row in group_rows), default=0),
                "sum_orientation_error_urad": sum(row["orientation_error_urad"] or 0 for row in group_rows),
                "max_orientation_error_urad": max((row["orientation_error_urad"] or 0 for row in group_rows), default=0),
                "levels": aggregate(group_rows),
            }
        )

    return {
        "case_count": len(results),
        "scored_count": sum(row["disposition"] == "SCORED" for row in results),
        "abstained_count": sum(row["disposition"] == "ABSTAINED" for row in results),
        "missing_count": sum(row["disposition"] == "MISSING" for row in results),
        "invalid_count": sum(row["disposition"] == "INVALID_INPUT" for row in results),
        "duplicate_sample_count": sum(row["disposition"] == "DUPLICATE_SAMPLE" for row in results),
        "split_leakage_count": sum(row["disposition"] == "SPLIT_LEAKAGE" for row in results),
        "levels": aggregate(results),
        "groups": groups,
        "results": results,
    }


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit("usage: reference_oracle.py <corpus.csv> <source_commit> <corpus_sha256>")

    corpus = Path(sys.argv[1])
    source_commit = sys.argv[2]
    expected_digest = sys.argv[3]
    raw = corpus.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    assert digest == expected_digest, "corpus digest mismatch"

    with corpus.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))

    assert len(rows) == 42, f"expected 42 cases, found {len(rows)}"
    assert len({row["case_id"] for row in rows}) == len(rows), "duplicate case ID"
    card = evaluate_manifest(rows)
    expected = {row["case_id"]: row["expected"] for row in rows}
    for result in card["results"]:
        assert result["disposition"] == expected[result["case_id"]], (
            f'{result["case_id"]}: expected {expected[result["case_id"]]}, '
            f'received {result["disposition"]}'
        )

    receipt = {
        "schema": SCHEMA,
        "source_commit": source_commit,
        "corpus_sha256": digest,
        "claim_ready": False,
        "source_bytes_authenticated": False,
        "physical_validation": "NOT_ASSESSED",
        "authority_granted": False,
        "scorecard": card,
    }
    print(json.dumps(receipt, separators=(",", ":")))
    print(f"Scored {len(rows)} pose-region cases across five nominal levels.", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
