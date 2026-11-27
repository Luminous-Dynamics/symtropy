#!/usr/bin/env python3
"""Independent exact-key oracle for SpaceSense-style multimodal frame bundles."""

from __future__ import annotations

import csv
import hashlib
import json
import sys
from collections import defaultdict
from datetime import date
from pathlib import Path

SCHEMA = "luminous.spacecraft-multimodal-frame-sync.receipt.v0.1"
SPACESENSE_REVISION = "f8c4ee9c7c0f9a4037f4977f3ddc4e0610a8fb8e"
SPACESENSE_POSE_CSV_RELEASE = "pose-ground-truth-regenerated-2026-05-19"
TIMESTAMP_FORMAT = "YYYYMMDDHHMMSSmmm"
MODALITIES = (
    ("rgb", "cam0", "rgb"),
    ("depth", "cam0", "depth"),
    ("segmentation", "cam0", "segmentation"),
    ("lidar", "lidar0", "lidar"),
    ("pose", "pose_ground_truth", "pose"),
)


def valid_text(value: str) -> bool:
    return bool(value.strip())


def valid_sha(value: str) -> bool:
    return len(value) == 64 and all(ch in "0123456789abcdefABCDEF" for ch in value)


def valid_timestamp(value: str) -> bool:
    if len(value) != 17 or not value.isascii() or not value.isdigit():
        return False
    try:
        year = int(value[0:4])
        month = int(value[4:6])
        day = int(value[6:8])
        hour = int(value[8:10])
        minute = int(value[10:12])
        second = int(value[12:14])
        millis = int(value[14:17])
        date(year, month, day)
    except ValueError:
        return False
    return hour <= 23 and minute <= 59 and second <= 59 and millis <= 999


def modality(row: dict[str, str], name: str) -> dict[str, object]:
    return {
        "timestamp": row[f"{name}_timestamp"],
        "frame_id": row[f"{name}_frame_id"],
        "sensor_id": row[f"{name}_sensor_id"],
        "path": row[f"{name}_path"],
        "sha256": row[f"{name}_sha256"],
        "present": row[f"{name}_present"] == "true",
    }


def result_base(
    row: dict[str, str],
    disposition: str,
    reason: str | None,
) -> dict[str, object]:
    sha_map = {name: row[f"{name}_sha256"] for name, _, _ in MODALITIES}
    path_map = {name: row[f"{name}_path"] for name, _, _ in MODALITIES}
    return {
        "case_id": row["case_id"],
        "dataset_id": row["dataset_id"],
        "dataset_revision": row["dataset_revision"],
        "license": row["license"],
        "spacecraft_id": row["spacecraft_id"],
        "trajectory_id": row["trajectory_id"],
        "sample_id": row["sample_id"],
        "frame_id": row["frame_id"],
        "split": row["split"],
        "capture_timestamp": row["capture_timestamp"],
        "pose_csv_revision": row["pose_csv_revision"],
        "corrected_pose_revision": row["corrected_pose_revision"],
        "camera_calibration_revision": row["camera_calibration_revision"],
        "camera_calibration_sha256": row["camera_calibration_sha256"],
        "lidar_to_camera_revision": row["lidar_to_camera_revision"],
        "lidar_to_camera_sha256": row["lidar_to_camera_sha256"],
        "lidar_coordinate_frame": row["lidar_coordinate_frame"],
        "pose_semantics": row["pose_semantics"],
        "disposition": disposition,
        "modality_sha256": sha_map,
        "modality_paths": path_map,
        "reason_codes": [] if reason is None else [reason],
        "bundle_admissible": disposition == "SYNCHRONIZED",
        "source_bytes_authenticated": False,
        "calibration_physically_validated": False,
        "authority_granted": False,
    }


def evaluate(row: dict[str, str]) -> dict[str, object]:
    required = (
        "case_id",
        "dataset_id",
        "dataset_revision",
        "license",
        "intended_use",
        "spacecraft_id",
        "trajectory_id",
        "sample_id",
        "frame_id",
        "capture_timestamp",
        "corrected_pose_revision",
        "pose_csv_revision",
        "camera_calibration_revision",
        "lidar_to_camera_revision",
    )
    if any(not valid_text(row[field]) for field in required):
        return result_base(row, "INVALID_INPUT", "BUNDLE_IDENTITY_MISSING")

    if row["split"] not in ("train", "validation", "test"):
        return result_base(row, "INVALID_INPUT", "UNKNOWN_SPLIT")

    if (
        row["dataset_id"] != "SpaceSense-Bench"
        or row["dataset_revision"] != SPACESENSE_REVISION
        or row["license"] != "CC-BY-NC-4.0"
        or row["intended_use"] != "NONCOMMERCIAL"
        or row["corrected_pose_revision"] != SPACESENSE_POSE_CSV_RELEASE
        or row["camera_frame"] != "FRD_X_FORWARD_Y_RIGHT_Z_DOWN"
        or row["world_frame"] != "AIRSIM_NED"
        or row["lidar_coordinate_frame"] != "SENSOR_LOCAL_FRAME"
        or row["pose_semantics"] != "TARGET_IN_CAMERA_AND_CAMERA_POSE_IN_WORLD"
        or row["quaternion_convention"] != "HAMILTON_WXYZ"
        or row["pose_length_unit"] != "metre"
        or row["depth_unit"] != "millimetre"
        or row["lidar_unit"] != "metre"
        or row["timestamp_format"] != TIMESTAMP_FORMAT
    ):
        return result_base(
            row,
            "OUT_OF_SCOPE",
            "DATASET_REVISION_LICENSE_OR_CONVENTION_UNSUPPORTED",
        )

    if not valid_sha(row["camera_calibration_sha256"]) or not valid_sha(
        row["lidar_to_camera_sha256"]
    ):
        return result_base(
            row,
            "INVALID_INPUT",
            "CALIBRATION_OR_EXTRINSICS_DIGEST_MALFORMED",
        )

    if row["pose_csv_revision"] != row["corrected_pose_revision"]:
        return result_base(
            row,
            "OUT_OF_SCOPE",
            "POSE_CSV_NOT_FROM_CORRECTED_FRAME_ALIGNED_REVISION",
        )

    if row["sample_id"] != "/".join(
        (row["spacecraft_id"], row["trajectory_id"], row["capture_timestamp"])
    ):
        return result_base(
            row,
            "OUT_OF_SCOPE",
            "SAMPLE_ID_DOES_NOT_BIND_SPACECRAFT_TRAJECTORY_AND_TIMESTAMP",
        )

    if not valid_timestamp(row["capture_timestamp"]):
        return result_base(row, "INVALID_INPUT", "CAPTURE_TIMESTAMP_INVALID")

    for name, expected_sensor, expected_path_component in MODALITIES:
        record = modality(row, name)
        if not record["present"]:
            reasons = {
                "rgb": "RGB_MODALITY_MISSING",
                "depth": "DEPTH_MODALITY_MISSING",
                "segmentation": "SEGMENTATION_MODALITY_MISSING",
                "lidar": "LIDAR_MODALITY_MISSING",
                "pose": "POSE_MODALITY_MISSING",
            }
            return result_base(row, "INCOMPLETE", reasons[name])

        if not valid_text(str(record["path"])) or not valid_sha(str(record["sha256"])):
            return result_base(row, "INVALID_INPUT", "MODALITY_PATH_OR_DIGEST_INVALID")

        path = str(record["path"])
        path_components = path.split("/")
        if (
            path.startswith("/")
            or "\\" in path
            or any(component in ("", ".", "..") for component in path_components)
        ):
            return result_base(
                row,
                "INVALID_INPUT",
                "MODALITY_PATH_IS_NOT_A_SAFE_RELATIVE_PATH",
            )
        if any(
            expected not in path_components
            for expected in (
                row["spacecraft_id"],
                row["trajectory_id"],
                expected_path_component,
                row["capture_timestamp"],
            )
        ):
            return result_base(
                row,
                "OUT_OF_SCOPE",
                "MODALITY_PATH_DOES_NOT_BIND_EXPECTED_SENSOR_STREAM_AND_FRAME",
            )

        if (
            not valid_text(str(record["sensor_id"]))
            or record["sensor_id"] != expected_sensor
            or record["frame_id"] != row["frame_id"]
        ):
            return result_base(row, "OUT_OF_SCOPE", "MODALITY_FRAME_OR_SENSOR_ID_MISMATCH")

        if not valid_timestamp(str(record["timestamp"])):
            return result_base(row, "INVALID_INPUT", "MODALITY_TIMESTAMP_INVALID")

        if record["timestamp"] != row["capture_timestamp"]:
            return result_base(
                row, "UNSYNCHRONIZED", "MODALITY_TIMESTAMP_DIFFERS_FROM_CAPTURE_EVENT"
            )

    return result_base(row, "SYNCHRONIZED", None)


def evaluate_manifest(rows: list[dict[str, str]]) -> list[dict[str, object]]:
    spacecraft_splits: dict[str, set[str]] = defaultdict(set)
    seen_samples: set[str] = set()
    duplicate_samples: set[str] = set()

    for row in rows:
        spacecraft_splits[row["spacecraft_id"]].add(row["split"])
        if row["sample_id"] in seen_samples:
            duplicate_samples.add(row["sample_id"])
        seen_samples.add(row["sample_id"])

    split_leaks = {
        spacecraft_id for spacecraft_id, splits in spacecraft_splits.items() if len(splits) > 1
    }

    results = []
    for row in rows:
        if row["spacecraft_id"] in split_leaks:
            result = result_base(
                row, "SPLIT_LEAKAGE", "SPACECRAFT_ID_PRESENT_IN_MULTIPLE_SPLITS"
            )
        elif row["sample_id"] in duplicate_samples:
            result = result_base(row, "DUPLICATE_SAMPLE", "SAMPLE_ID_DUPLICATED")
        else:
            result = evaluate(row)
        results.append(result)

    return sorted(results, key=lambda item: item["sample_id"])


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit(
            "usage: reference_oracle.py <corpus.csv> <source_commit> <corpus_sha256>"
        )

    corpus = Path(sys.argv[1])
    source_commit = sys.argv[2]
    expected_digest = sys.argv[3]
    raw = corpus.read_bytes()
    actual_digest = hashlib.sha256(raw).hexdigest()
    assert actual_digest == expected_digest, "corpus digest mismatch"

    with corpus.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))

    assert len(rows) == 49, f"expected 44 cases, found {len(rows)}"
    assert len({row["case_id"] for row in rows}) == len(rows), "duplicate case ID"

    results = evaluate_manifest(rows)
    expected = {row["case_id"]: row["expected"] for row in rows}
    for result in results:
        assert result["disposition"] == expected[result["case_id"]], (
            f'{result["case_id"]}: expected {expected[result["case_id"]]}, '
            f'received {result["disposition"]}'
        )

    receipt = {
        "schema": SCHEMA,
        "source_commit": source_commit,
        "corpus_sha256": actual_digest,
        "dataset_revision": SPACESENSE_REVISION,
        "pose_correction_date": "2026-05-19",
        "case_count": len(results),
        "expected_disposition_matches": len(results),
        "expected_disposition_mismatches": 0,
        "synchronized_count": sum(item["disposition"] == "SYNCHRONIZED" for item in results),
        "claim_ready": False,
        "source_bytes_authenticated": False,
        "calibration_physically_validated": False,
        "authority_granted": False,
        "results": results,
    }
    print(json.dumps(receipt, separators=(",", ":")))
    print(f"Validated {len(results)} multimodal frame bundles independently.", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
