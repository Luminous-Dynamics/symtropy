#!/usr/bin/env python3
"""Independent geometry and temporal-label preservation challenger."""

from __future__ import annotations

import csv
import hashlib
import json
import math
import sys
from collections import defaultdict, deque
from pathlib import Path

SCHEMA = "luminous.spacecraft-translation-geometry-audit.receipt.v0.1"
MAX_MASK_PIXELS = 16384
MPX_PER_PIXEL = 1000
UNIT_FIELDS = {"TARGET": "target_id", "SESSION": "session_id", "SEQUENCE": "sequence_id"}
VISIBILITY = {"VISIBLE", "OCCLUDED", "OUT_OF_FRAME"}
CROP_STATES = {"FULL", "CROPPED", "TRUNCATED"}
PARTITION = "CALIBRATION"


def valid_text(value: str) -> bool:
    return bool(value.strip()) and value != "NONE"


def valid_sha(value: str) -> bool:
    return len(value) == 64 and all(c in "0123456789abcdefABCDEF" for c in value)


def scope_key(row: dict[str, str]) -> tuple[str, ...]:
    return (
        row["domain_id"], row["condition_id"], row["crop_stratum"], row["modality"],
        row["ood_group"], row["calibration_revision"], row["translator_sha256"],
        row["image_width_px"], row["image_height_px"], row["unit_kind"],
        row["frame_budget"], row["landmark_tolerance_mpx"],
        row["temporal_motion_tolerance_mpx"], row["default_mask_iou_floor_bps"],
    )


def frame_metadata_key(row: dict[str, str]) -> tuple[str, ...]:
    return (
        row["sample_id"], row["target_id"], row["session_id"], row["sequence_id"],
        row["unit_id"], row["unit_kind"], row["partition"], row["frame_index"],
        *scope_key(row), row["source_image_sha256"], row["translated_image_sha256"],
        row["source_annotation_sha256"], row["translated_annotation_sha256"],
    )


def decode_mask(value: str, pixel_count: int) -> list[bool]:
    if len(value) != (pixel_count + 3) // 4:
        raise ValueError("mask hex length does not match image dimensions")
    output: list[bool] = []
    for character in value:
        try:
            nibble = int(character, 16)
        except ValueError as error:
            raise ValueError("mask contains non-hex character") from error
        output.extend(bool(nibble & (1 << bit)) for bit in range(4) if len(output) < pixel_count)
    return output


def component_count(mask: list[bool], width: int, height: int) -> int:
    seen = [False] * len(mask)
    components = 0
    for index, active in enumerate(mask):
        if not active or seen[index]:
            continue
        components += 1
        seen[index] = True
        queue = deque([index])
        while queue:
            current = queue.popleft()
            x = current % width
            y = current // width
            neighbors = []
            if x > 0:
                neighbors.append(current - 1)
            if x + 1 < width:
                neighbors.append(current + 1)
            if y > 0:
                neighbors.append(current - width)
            if y + 1 < height:
                neighbors.append(current + width)
            for neighbor in neighbors:
                if mask[neighbor] and not seen[neighbor]:
                    seen[neighbor] = True
                    queue.append(neighbor)
    return components


def score_mask(row: dict[str, str], width: int, height: int) -> dict[str, object]:
    floor = int(row["mask_iou_floor_bps"])
    if floor > 10000:
        raise ValueError("mask IoU floor exceeds 10000 bps")
    count = width * height
    if count <= 0 or count > MAX_MASK_PIXELS:
        raise ValueError("mask dimensions exceed bounded v0.1 size")
    source = decode_mask(row["source_mask_hex"], count)
    translated = decode_mask(row["translated_mask_hex"], count)
    intersection = sum(a and b for a, b in zip(source, translated, strict=True))
    union = sum(a or b for a, b in zip(source, translated, strict=True))
    if union == 0:
        raise ValueError("empty source/translated mask union")
    iou = intersection * 10000 // union
    source_components = component_count(source, width, height)
    translated_components = component_count(translated, width, height)
    return {
        "region_id": row["record_id"],
        "intersection_pixels": intersection,
        "union_pixels": union,
        "iou_bps": iou,
        "source_component_count": source_components,
        "translated_component_count": translated_components,
        "passed": iou >= floor and source_components == translated_components,
    }


def score_frame(frame: dict[str, object]) -> dict[str, object]:
    metadata = frame["metadata"]
    width = int(metadata["image_width_px"]) * MPX_PER_PIXEL
    height = int(metadata["image_height_px"]) * MPX_PER_PIXEL
    reasons: set[str] = set()
    displacements: list[int] = []

    for item in frame["landmarks"]:
        landmark_id = item["landmark_id"]
        if item["source_visibility"] != item["translated_visibility"]:
            reasons.add(f"VISIBILITY_CHANGED:{landmark_id}")
            continue
        if item["source_crop_state"] != item["translated_crop_state"]:
            reasons.add(f"CROP_STATE_CHANGED:{landmark_id}")
            continue
        if item["source_visibility"] == "VISIBLE":
            source_ok = (
                0 <= item["source_x_mpx"] < width
                and 0 <= item["source_y_mpx"] < height
            )
            translated_ok = (
                0 <= item["translated_x_mpx"] < width
                and 0 <= item["translated_y_mpx"] < height
            )
            if not source_ok or not translated_ok:
                reasons.add(f"VISIBLE_LANDMARK_OUT_OF_BOUNDS:{landmark_id}")
                continue
            distance = math.isqrt(
                (item["translated_x_mpx"] - item["source_x_mpx"]) ** 2
                + (item["translated_y_mpx"] - item["source_y_mpx"]) ** 2
            )
            if distance * distance < (
                (item["translated_x_mpx"] - item["source_x_mpx"]) ** 2
                + (item["translated_y_mpx"] - item["source_y_mpx"]) ** 2
            ):
                distance += 1
            if distance > int(metadata["landmark_tolerance_mpx"]):
                reasons.add(f"LANDMARK_DISPLACEMENT_EXCEEDED:{landmark_id}")
            displacements.append(distance)

    masks = [score_mask(mask, int(metadata["image_width_px"]), int(metadata["image_height_px"])) for mask in frame["masks"]]
    for mask in masks:
        if not mask["passed"]:
            reasons.add(f"MASK_PRESERVATION_FAILED:{mask['region_id']}")

    ordered = sorted(displacements)
    p95 = None if not ordered else ordered[(len(ordered) * 95 + 99) // 100 - 1]
    return {
        "sample_id": metadata["sample_id"],
        "frame_index": int(metadata["frame_index"]),
        "disposition": "PASS_GEOMETRY_PRESERVATION" if not reasons else "BLOCKED_GEOMETRY_PRESERVATION",
        "reason_codes": sorted(reasons),
        "compared_landmark_count": len(displacements),
        "max_landmark_displacement_mpx": max(displacements) if displacements else None,
        "p95_landmark_displacement_mpx": p95,
        "mask_scores": masks,
    }


def validate_manifest(rows: list[dict[str, str]]) -> dict[str, dict[str, object]]:
    if not rows:
        raise ValueError("manifest has no geometry rows")
    if rows[0]["unit_kind"] not in UNIT_FIELDS:
        raise ValueError("unsupported unit kind")
    manifest_unit_kind = rows[0]["unit_kind"]
    frames: dict[str, dict[str, object]] = {}
    for line, row in enumerate(rows, start=2):
        for field in (
            "sample_id", "target_id", "session_id", "sequence_id", "unit_id",
            "domain_id", "condition_id", "crop_stratum", "modality", "ood_group",
            "calibration_revision",
        ):
            if not valid_text(row[field]):
                raise ValueError(f"row {line}: {field} is empty or reserved")
        kind = row["unit_kind"]
        if kind != manifest_unit_kind:
            raise ValueError(f"row {line}: unit kind changes within manifest")
        if kind not in UNIT_FIELDS:
            raise ValueError(f"row {line}: unsupported unit kind")
        if row["partition"] != PARTITION:
            raise ValueError(f"row {line}: geometry audit accepts CALIBRATION only")
        if row["unit_id"] != row[UNIT_FIELDS[kind]]:
            raise ValueError(f"row {line}: unit_id mismatches declared identity")

        width = int(row["image_width_px"])
        height = int(row["image_height_px"])
        budget = int(row["frame_budget"])
        index = int(row["frame_index"])
        if width <= 0 or height <= 0 or width * height > MAX_MASK_PIXELS or budget <= 0 or index >= budget:
            raise ValueError(f"row {line}: invalid dimensions or frame index/budget")
        if any(not valid_sha(row[field]) for field in (
            "translator_sha256", "source_image_sha256", "translated_image_sha256",
            "source_annotation_sha256", "translated_annotation_sha256",
        )):
            raise ValueError(f"row {line}: invalid source/translation/annotation digest")
        if int(row["default_mask_iou_floor_bps"]) > 10000:
            raise ValueError(f"row {line}: default mask IoU floor exceeds 10000")

        sample = row["sample_id"]
        key = frame_metadata_key(row)
        if sample not in frames:
            frames[sample] = {
                "metadata_key": key,
                "metadata": {
                    "sample_id": sample,
                    "target_id": row["target_id"],
                    "session_id": row["session_id"],
                    "sequence_id": row["sequence_id"],
                    "unit_id": row["unit_id"],
                    "unit_kind": kind,
                    "partition": row["partition"],
                    "frame_index": index,
                    "domain_id": row["domain_id"],
                    "condition_id": row["condition_id"],
                    "crop_stratum": row["crop_stratum"],
                    "modality": row["modality"],
                    "ood_group": row["ood_group"],
                    "calibration_revision": row["calibration_revision"],
                    "translator_sha256": row["translator_sha256"],
                    "image_width_px": width,
                    "image_height_px": height,
                    "landmark_tolerance_mpx": int(row["landmark_tolerance_mpx"]),
                    "temporal_motion_tolerance_mpx": int(row["temporal_motion_tolerance_mpx"]),
                    "default_mask_iou_floor_bps": int(row["default_mask_iou_floor_bps"]),
                    "source_image_sha256": row["source_image_sha256"],
                    "translated_image_sha256": row["translated_image_sha256"],
                    "source_annotation_sha256": row["source_annotation_sha256"],
                    "translated_annotation_sha256": row["translated_annotation_sha256"],
                    "frame_budget": budget,
                },
                "landmarks": [],
                "masks": [],
                "record_ids": set(),
            }
        elif frames[sample]["metadata_key"] != key:
            raise ValueError(f"row {line}: frame metadata differs across records")
        frame = frames[sample]
        record_kind = row["record_kind"]
        record_id = row["record_id"]
        identity = (record_kind, record_id)
        if not valid_text(record_id) or identity in frame["record_ids"]:
            raise ValueError(f"row {line}: empty or duplicate record identity")
        frame["record_ids"].add(identity)

        if record_kind == "LANDMARK":
            if any(row[field] != "NONE" for field in (
                "source_mask_hex", "translated_mask_hex", "mask_iou_floor_bps"
            )):
                raise ValueError(f"row {line}: landmark row carries mask fields")
            if row["source_visibility"] not in VISIBILITY or row["translated_visibility"] not in VISIBILITY:
                raise ValueError(f"row {line}: invalid visibility")
            if row["source_crop_state"] not in CROP_STATES or row["translated_crop_state"] not in CROP_STATES:
                raise ValueError(f"row {line}: invalid crop state")
            frame["landmarks"].append({
                "landmark_id": record_id,
                "source_x_mpx": int(row["source_x_mpx"]),
                "source_y_mpx": int(row["source_y_mpx"]),
                "translated_x_mpx": int(row["translated_x_mpx"]),
                "translated_y_mpx": int(row["translated_y_mpx"]),
                "source_visibility": row["source_visibility"],
                "translated_visibility": row["translated_visibility"],
                "source_crop_state": row["source_crop_state"],
                "translated_crop_state": row["translated_crop_state"],
            })
        elif record_kind == "MASK":
            if any(row[field] != "NONE" for field in (
                "source_x_mpx", "source_y_mpx", "translated_x_mpx", "translated_y_mpx",
                "source_visibility", "translated_visibility", "source_crop_state", "translated_crop_state"
            )):
                raise ValueError(f"row {line}: mask row carries landmark fields")
            if int(row["mask_iou_floor_bps"]) < int(row["default_mask_iou_floor_bps"]):
                raise ValueError(f"row {line}: per-region IoU floor below scope floor")
            frame["masks"].append({
                "region_id": record_id,
                "source_mask_hex": row["source_mask_hex"],
                "translated_mask_hex": row["translated_mask_hex"],
                "iou_floor_bps": int(row["mask_iou_floor_bps"]),
            })
        else:
            raise ValueError(f"row {line}: unsupported record_kind")
    for frame in frames.values():
        if not frame["landmarks"] or not frame["masks"]:
            raise ValueError("every frame requires at least one landmark and mask")
    return frames


def temporal_motion(frames: list[dict[str, object]]) -> int | None:
    maximum: int | None = None
    frames = sorted(frames, key=lambda frame: frame["metadata"]["frame_index"])
    for previous, following in zip(frames, frames[1:]):
        left = {item["landmark_id"]: item for item in previous["landmarks"]}
        right = {item["landmark_id"]: item for item in following["landmarks"]}
        for landmark_id, a in left.items():
            b = right.get(landmark_id)
            if b is None:
                raise ValueError("temporal landmark missing")
            if any(a[field] != "VISIBLE" or b[field] != "VISIBLE" for field in (
                "source_visibility", "translated_visibility"
            )):
                continue
            source_dx = b["source_x_mpx"] - a["source_x_mpx"]
            source_dy = b["source_y_mpx"] - a["source_y_mpx"]
            translated_dx = b["translated_x_mpx"] - a["translated_x_mpx"]
            translated_dy = b["translated_y_mpx"] - a["translated_y_mpx"]
            dx = translated_dx - source_dx
            dy = translated_dy - source_dy
            residual = math.isqrt(dx * dx + dy * dy)
            if residual * residual < dx * dx + dy * dy:
                residual += 1
            maximum = residual if maximum is None else max(maximum, residual)
    return maximum


def evaluate(frames_by_sample: dict[str, dict[str, object]]) -> dict[str, object]:
    units: dict[str, list[dict[str, object]]] = defaultdict(list)
    unit_scopes: dict[str, tuple[object, ...]] = {}
    for frame in frames_by_sample.values():
        metadata = frame["metadata"]
        unit_id = metadata["unit_id"]
        scope = (
            metadata["unit_kind"], metadata["domain_id"], metadata["condition_id"],
            metadata["crop_stratum"], metadata["modality"], metadata["ood_group"],
            metadata["calibration_revision"], metadata["translator_sha256"],
            metadata["image_width_px"], metadata["image_height_px"], metadata["frame_budget"],
            metadata["landmark_tolerance_mpx"], metadata["temporal_motion_tolerance_mpx"],
            metadata["default_mask_iou_floor_bps"],
        )
        if unit_id in unit_scopes and unit_scopes[unit_id] != scope:
            raise ValueError("unit crosses geometry audit scopes")
        unit_scopes[unit_id] = scope
        units[unit_id].append(frame)

    results = []
    passed_units = blocked_units = frame_count = 0
    for unit_id in sorted(units):
        frames = sorted(units[unit_id], key=lambda frame: frame["metadata"]["frame_index"])
        first = frames[0]["metadata"]
        budget = first["frame_budget"]
        if len(frames) != budget:
            raise ValueError(f"unit {unit_id} incomplete frame budget")
        if [frame["metadata"]["frame_index"] for frame in frames] != list(range(budget)):
            raise ValueError(f"unit {unit_id} has missing or duplicate frame indices")
        expected_landmarks = {item["landmark_id"] for item in frames[0]["landmarks"]}
        expected_masks = {item["region_id"] for item in frames[0]["masks"]}
        for frame in frames:
            metadata = frame["metadata"]
            if (
                metadata["unit_id"] != first["unit_id"]
                or metadata["unit_kind"] != first["unit_kind"]
                or metadata["target_id"] != first["target_id"]
                or metadata["domain_id"] != first["domain_id"]
                or metadata["condition_id"] != first["condition_id"]
                or metadata["crop_stratum"] != first["crop_stratum"]
                or metadata["modality"] != first["modality"]
                or metadata["ood_group"] != first["ood_group"]
                or metadata["calibration_revision"] != first["calibration_revision"]
            ):
                raise ValueError(f"unit {unit_id} changes scope or target")
            if {item["landmark_id"] for item in frame["landmarks"]} != expected_landmarks:
                raise ValueError(f"unit {unit_id} changes landmark identity set")
            if {item["region_id"] for item in frame["masks"]} != expected_masks:
                raise ValueError(f"unit {unit_id} changes mask identity set")

        frame_scores = [score_frame(frame) for frame in frames]
        temporal = temporal_motion(frames)
        reasons: set[str] = set()
        if temporal is not None and temporal > first["temporal_motion_tolerance_mpx"]:
            reasons.add("TEMPORAL_MOTION_RESIDUAL_EXCEEDED")
        elif budget > 1 and temporal is None:
            reasons.add("NO_VISIBLE_LANDMARK_TRAJECTORY_FOR_TEMPORAL_AUDIT")
        if any(frame["disposition"] != "PASS_GEOMETRY_PRESERVATION" for frame in frame_scores):
            reasons.add("ONE_OR_MORE_FRAMES_FAILED_GEOMETRY_PRESERVATION")

        frame_passed = sum(frame["disposition"] == "PASS_GEOMETRY_PRESERVATION" for frame in frame_scores)
        frame_blocked = len(frame_scores) - frame_passed
        displacements = [
            frame["max_landmark_displacement_mpx"]
            for frame in frame_scores
            if frame["max_landmark_displacement_mpx"] is not None
        ]
        ordered = sorted(displacements)
        p95 = None if not ordered else ordered[(len(ordered) * 95 + 99) // 100 - 1]
        if reasons:
            blocked_units += 1
        else:
            passed_units += 1
        frame_count += len(frames)
        results.append({
            "unit_id": unit_id,
            "unit_kind": first["unit_kind"],
            "frame_count": len(frames),
            "passed_frame_count": frame_passed,
            "blocked_frame_count": frame_blocked,
            "max_landmark_displacement_mpx": max(displacements) if displacements else None,
            "p95_landmark_displacement_mpx": p95,
            "max_temporal_motion_residual_mpx": temporal,
            "disposition": "PASS_GEOMETRY_PRESERVATION" if not reasons else "BLOCKED_GEOMETRY_PRESERVATION",
            "reason_codes": sorted(reasons),
            "frames": frame_scores,
        })

    return {
        "schema": SCHEMA,
        "unit_count": len(results),
        "frame_count": frame_count,
        "passed_unit_count": passed_units,
        "blocked_unit_count": blocked_units,
        "results": results,
        "image_pixels_inspected_here": False,
        "annotation_source_bytes_authenticated": False,
        "independent_metrology_verified_here": False,
        "physical_truth_established": False,
        "calibration_exchangeability_proven": False,
        "flight_qualification": False,
        "maneuver_authority_granted": False,
    }


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit(
            "usage: reference_oracle.py <annotations.tsv> <source-commit> <corpus-sha256>"
        )
    path = Path(sys.argv[1])
    raw = path.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if digest != sys.argv[3]:
        raise SystemExit("manifest SHA-256 does not match input bytes")
    with path.open(newline="", encoding="utf-8") as stream:
        rows = list(csv.DictReader(stream, delimiter="\t"))
    frames = validate_manifest(rows)
    receipt = evaluate(frames)
    receipt["source_commit"] = sys.argv[2]
    receipt["corpus_sha256"] = digest
    receipt = {
        "schema": receipt.pop("schema"),
        "source_commit": receipt.pop("source_commit"),
        "corpus_sha256": receipt.pop("corpus_sha256"),
        **receipt,
    }
    print(json.dumps(receipt, sort_keys=True, separators=(",", ":"), allow_nan=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
