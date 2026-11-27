#!/usr/bin/env python3
"""Independent standard-library oracle for spacecraft pose annotation geometry."""

from __future__ import annotations

import csv
import hashlib
import json
import math
import sys
from collections import defaultdict
from pathlib import Path

PIXEL_SCALE = 1_000_000.0
QUATERNION_TOLERANCE = 1.0e-6
SCHEMA = "luminous.spacecraft-pose-annotation-contract.receipt.v0.1"


def finite(values: list[float]) -> bool:
    return all(math.isfinite(value) for value in values)


def rotate(q: list[float], vector: list[float]) -> list[float]:
    x, y, z, w = q
    uxv = [
        y * vector[2] - z * vector[1],
        z * vector[0] - x * vector[2],
        x * vector[1] - y * vector[0],
    ]
    uxu = [
        y * uxv[2] - z * uxv[1],
        z * uxv[0] - x * uxv[2],
        x * uxv[1] - y * uxv[0],
    ]
    return [
        vector[0] + 2.0 * (w * uxv[0] + uxu[0]),
        vector[1] + 2.0 * (w * uxv[1] + uxu[1]),
        vector[2] + 2.0 * (w * uxv[2] + uxu[2]),
    ]


def scale_i64(value: float) -> int | None:
    scaled = math.floor(value * PIXEL_SCALE + 0.5) if value >= 0 else math.ceil(value * PIXEL_SCALE - 0.5)
    return scaled if -(1 << 63) <= scaled <= (1 << 63) - 1 else None


def make_result(
    row: dict[str, str],
    status: str,
    reason: str | None,
    norm_error: int | None,
    projected: list[float] | None,
    reprojection: int | None,
) -> dict[str, object]:
    return {
        "case_id": row["case_id"],
        "sample_id": row["sample_id"],
        "split": row["split"],
        "disposition": status,
        "keypoint_id": row["keypoint_id"],
        "quaternion_norm_error_scaled": norm_error,
        "projected_u_micro_px": None if projected is None else scale_i64(projected[0]),
        "projected_v_micro_px": None if projected is None else scale_i64(projected[1]),
        "reprojection_error_micro_px": reprojection,
        "reason_codes": [] if reason is None else [reason],
        "pose_contract_version": "spacecraft-pose-annotation-contract.v0.1",
        "source_annotation_release": row["annotation_release"],
        "source_bytes_authenticated": False,
        "pose_ground_truth_physically_validated": False,
    }


def evaluate(row: dict[str, str]) -> dict[str, object]:
    case_id = row["case_id"]
    if (
        not case_id.strip()
        or not row["sample_id"].strip()
        or not row["keypoint_id"].strip()
        or int(row["keypoint_count"]) == 0
    ):
        return make_result(
            row, "INVALID_INPUT", "IDENTITY_OR_KEYPOINT_CARDINALITY_INVALID", None, None, None
        )

    if row["annotation_release"] != "CORRECTION" or row["bbox3d_present"] != "true":
        return make_result(
            row, "ANNOTATION_INCOMPLETE", "CORRECTED_BBOX3D_ANNOTATION_REQUIRED", None, None, None
        )

    if row["split"] not in ("train", "val", "test"):
        return make_result(row, "INVALID_INPUT", "UNKNOWN_SPLIT", None, None, None)

    if (
        row["pose_semantics"] != "CAMERA_TO_WORLD"
        or row["camera_axis_convention"] != "BLENDER_NEG_Z_FORWARD_POS_Y_UP"
        or row["projection_axis_convention"] != "OPENCV_POS_Z_FORWARD_Y_DOWN"
        or row["length_unit"] != "metre"
        or row["quaternion_order"] not in ("XYZW", "WXYZ")
    ):
        return make_result(
            row, "OUT_OF_SCOPE", "UNSUPPORTED_POSE_OR_COORDINATE_CONVENTION", None, None, None
        )

    width = int(row["image_width"])
    height = int(row["image_height"])
    fx, fy, cx, cy = (float(row[field]) for field in ("fx", "fy", "cx", "cy"))
    tolerance = float(row["tolerance_px"])
    observed = [float(row["u"]), float(row["v"])]
    translation = [float(row[field]) for field in ("tx", "ty", "tz")]
    keypoint = [float(row[field]) for field in ("kpx", "kpy", "kpz")]
    source_q = [float(row[field]) for field in ("qx", "qy", "qz", "qw")]

    if (
        width == 0
        or height == 0
        or not finite([fx, fy, cx, cy, tolerance, *observed, *translation, *keypoint, *source_q])
        or fx <= 0
        or fy <= 0
        or tolerance < 0
    ):
        return make_result(
            row, "INVALID_INPUT", "CAMERA_OR_GEOMETRY_NUMERIC_INPUT_INVALID", None, None, None
        )

    if row["quaternion_order"] == "XYZW":
        q = source_q
    else:
        q = [source_q[1], source_q[2], source_q[3], source_q[0]]

    norm = math.sqrt(sum(value * value for value in q))
    if not math.isfinite(norm) or norm <= 1.0e-12:
        return make_result(row, "INVALID_INPUT", "QUATERNION_NORM_INVALID", None, None, None)

    norm_error = abs(norm - 1.0)
    norm_error_scaled = scale_i64(norm_error * 1.0e9)
    if norm_error > QUATERNION_TOLERANCE:
        return make_result(
            row,
            "INVALID_INPUT",
            "QUATERNION_NOT_UNIT_WITHIN_TOLERANCE",
            norm_error_scaled,
            None,
            None,
        )

    q = [value / norm for value in q]
    inverse_q = [-q[0], -q[1], -q[2], q[3]]
    relative = [keypoint[i] - translation[i] for i in range(3)]
    camera_blender = rotate(inverse_q, relative)
    camera_cv = [camera_blender[0], -camera_blender[1], -camera_blender[2]]
    depth = camera_cv[2]

    if not math.isfinite(depth) or depth <= 1.0e-9:
        return make_result(
            row,
            "INCONSISTENT",
            "KEYPOINT_NOT_IN_FRONT_OF_CAMERA_OR_PROJECTION_INVALID",
            norm_error_scaled,
            None,
            None,
        )

    projected = [
        fx * camera_cv[0] / depth + cx,
        fy * camera_cv[1] / depth + cy,
    ]
    if not finite(projected):
        return make_result(
            row,
            "INCONSISTENT",
            "KEYPOINT_NOT_IN_FRONT_OF_CAMERA_OR_PROJECTION_INVALID",
            norm_error_scaled,
            None,
            None,
        )

    error = math.hypot(projected[0] - observed[0], projected[1] - observed[1])
    error_scaled = scale_i64(error * PIXEL_SCALE)
    reprojection = None if error_scaled is None else error_scaled if error_scaled >= 0 else None
    if not math.isfinite(error) or error > tolerance:
        return make_result(
            row,
            "INCONSISTENT",
            "ANNOTATED_KEYPOINT_EXCEEDS_REPROJECTION_TOLERANCE",
            norm_error_scaled,
            projected,
            reprojection,
        )

    return make_result(row, "GEOMETRICALLY_CONSISTENT", None, norm_error_scaled, projected, reprojection)


def evaluate_manifest(rows: list[dict[str, str]]) -> list[dict[str, object]]:
    splits: dict[str, set[str]] = defaultdict(set)
    keypoints: dict[str, set[str]] = defaultdict(set)
    duplicate_keypoints: set[str] = set()
    duplicate_splits: set[str] = set()

    for row in rows:
        splits[row["sample_id"]].add(row["split"])
        if row["keypoint_id"] in keypoints[row["sample_id"]]:
            duplicate_keypoints.add(row["sample_id"])
        keypoints[row["sample_id"]].add(row["keypoint_id"])

    for sample_id, sample_splits in splits.items():
        if len(sample_splits) > 1:
            duplicate_splits.add(sample_id)

    results = []
    for row in rows:
        if row["sample_id"] in duplicate_splits:
            result = make_result(
                row,
                "DUPLICATE_SPLIT_SAMPLE",
                "SAMPLE_ID_PRESENT_IN_MULTIPLE_SPLITS",
                None,
                None,
                None,
            )
        elif row["sample_id"] in duplicate_keypoints:
            result = make_result(
                row,
                "DUPLICATE_KEYPOINT",
                "KEYPOINT_ID_DUPLICATED_WITHIN_SAMPLE",
                None,
                None,
                None,
            )
        else:
            result = evaluate(row)
        results.append(result)
    return results


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit("usage: reference_oracle.py <corpus.csv> <source_commit> <corpus_sha256>")

    path = Path(sys.argv[1])
    source_commit = sys.argv[2]
    expected_digest = sys.argv[3]
    raw = path.read_bytes()
    actual_digest = hashlib.sha256(raw).hexdigest()
    assert actual_digest == expected_digest, "corpus digest mismatch"

    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))

    assert len(rows) == 41, f"expected 41 fixtures, found {len(rows)}"
    assert len({row["case_id"] for row in rows}) == len(rows), "duplicate case ID"
    results = evaluate_manifest(rows)
    for row, result in zip(rows, results, strict=True):
        assert result["disposition"] == row["expected"], (
            f'{row["case_id"]}: expected {row["expected"]}, got {result["disposition"]}'
        )

    receipt = {
        "schema": SCHEMA,
        "source_commit": source_commit,
        "corpus_sha256": actual_digest,
        "case_count": len(results),
        "expected_disposition_matches": len(results),
        "expected_disposition_mismatches": 0,
        "geometrically_consistent_count": sum(
            result["disposition"] == "GEOMETRICALLY_CONSISTENT" for result in results
        ),
        "claim_ready": False,
        "source_bytes_authenticated": False,
        "pose_ground_truth_physically_validated": False,
        "authority_granted": False,
        "results": results,
    }
    print(json.dumps(receipt, separators=(",", ":")))
    print(f"Validated {len(results)} spacecraft pose annotation fixtures.", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
