#!/usr/bin/env python3
"""Independent no-ground-truth oracle for runtime SE(3) pose admission."""

from __future__ import annotations

import csv
import hashlib
import json
import math
import sys
from collections import defaultdict
from pathlib import Path

SCHEMA = "luminous.spacecraft-runtime-pose-admission.receipt.v0.1"
TANGENT = "BODY_LOCAL_TRANSLATION_XYZ_ROTATION_VECTOR_XYZ"
COVARIANCE_UNITS = "m,m,m,rad,rad,rad"
ADMISSION_BPS = 9500
MIN_PROFILE_SAMPLES = 100
MIN_VISIBLE = 0.05
MIN_CORRESPONDENCES = 6
MAX_GEOMETRY_CONDITION = 100_000_000.0
SYMMETRY_TOLERANCE = 1.0e-10


def valid_text(value: str) -> bool:
    return bool(value.strip())


def valid_sha(value: str) -> bool:
    return len(value) == 64 and all(ch in "0123456789abcdefABCDEF" for ch in value)


def num(row: dict[str, str], field: str) -> float:
    return float(row[field])


def boolean(row: dict[str, str], field: str) -> bool:
    return row[field] == "true"


def vector(row: dict[str, str], prefix: str, count: int) -> list[float]:
    suffixes = ("x", "y", "z", "w") if count == 4 else ("x", "y", "z")
    return [num(row, prefix + suffix) for suffix in suffixes]


def finite(values: list[float]) -> bool:
    return all(math.isfinite(value) for value in values)


def quaternion_is_unit(q: list[float]) -> bool:
    if not finite(q):
        return False
    norm2 = sum(value * value for value in q)
    if not math.isfinite(norm2) or norm2 <= 1.0e-24:
        return False
    return abs(math.sqrt(norm2) - 1.0) <= 1.0e-6


def covariance_is_spd(flat: list[float]) -> bool:
    if len(flat) != 36 or not finite(flat):
        return False
    matrix = [flat[6 * i:6 * i + 6] for i in range(6)]
    scale = max(abs(value) for value in flat)
    if not math.isfinite(scale) or scale <= 0.0:
        return False
    for row in range(6):
        for column in range(6):
            if abs(matrix[row][column] - matrix[column][row]) > SYMMETRY_TOLERANCE * scale:
                return False

    lower = [[0.0] * 6 for _ in range(6)]
    for row in range(6):
        for column in range(row + 1):
            total = matrix[row][column]
            for left, right in zip(lower[row][:column], lower[column][:column]):
                total -= left * right
            if row == column:
                if not math.isfinite(total) or total <= sys.float_info.epsilon * scale * 16.0:
                    return False
                lower[row][column] = math.sqrt(total)
            else:
                pivot = lower[column][column]
                if pivot <= 0.0 or not math.isfinite(pivot):
                    return False
                lower[row][column] = total / pivot
    return True


def profile_shape_valid(row: dict[str, str]) -> bool:
    required = (
        "profile_id", "profile_revision", "profile_dataset_id",
        "profile_dataset_revision", "profile_model_revision", "profile_sensor_mode",
        "profile_frame_convention", "profile_camera_calibration_revision",
        "profile_target_population_id", "profile_method",
    )
    return (
        all(valid_text(row[field]) for field in required)
        and valid_sha(row["profile_sha256"])
        and valid_sha(row["profile_model_sha256"])
        and valid_sha(row["profile_camera_calibration_sha256"])
        and valid_sha(row["profile_target_population_sha256"])
        and valid_sha(row["calibration_split_sha256"])
        and valid_sha(row["heldout_split_sha256"])
        and int(row["profile_coverage_lower_ci_ppm"]) <= 1_000_000
        and int(row["required_coverage_lower_ci_ppm"]) <= 1_000_000
        and int(row["profile_nominal_level_bps"]) <= 10_000
    )


def prediction_shape_valid(row: dict[str, str]) -> bool:
    required = (
        "case_id", "sample_id", "frame_id", "target_id", "trajectory_id",
        "target_population_id", "dataset_id", "dataset_revision", "model_revision",
        "camera_calibration_revision", "frame_bundle_revision", "bundle_sample_id",
        "bundle_frame_id", "bundle_target_id", "bundle_dataset_revision",
        "bundle_camera_calibration_revision", "support_mode",
    )
    numbers = [
        *vector(row, "pred_t", 3),
        num(row, "visible_fraction"),
        num(row, "geometry_condition"),
    ]
    return (
        all(valid_text(row[field]) for field in required)
        and all(valid_sha(row[field]) for field in (
            "dataset_sha256", "target_population_sha256", "model_sha256",
            "model_config_sha256", "camera_calibration_sha256", "frame_bundle_sha256",
        ))
        and finite(numbers)
        and 0.0 <= num(row, "visible_fraction") <= 1.0
        and num(row, "geometry_condition") >= 0.0
        and int(row["nominal_level_bps"]) <= 10_000
    )


def base_result(row: dict[str, str], disposition: str, reason: str | None) -> dict[str, object]:
    frame_authenticated = boolean(row, "frame_source_bytes_authenticated")
    profile_authenticated = boolean(row, "profile_source_bytes_authenticated")
    return {
        "case_id": row["case_id"],
        "sample_id": row["sample_id"],
        "frame_id": row["frame_id"],
        "target_id": row["target_id"],
        "trajectory_id": row["trajectory_id"],
        "dataset_id": row["dataset_id"],
        "dataset_revision": row["dataset_revision"],
        "dataset_sha256": row["dataset_sha256"],
        "model_revision": row["model_revision"],
        "model_sha256": row["model_sha256"],
        "model_config_sha256": row["model_config_sha256"],
        "camera_calibration_revision": row["camera_calibration_revision"],
        "camera_calibration_sha256": row["camera_calibration_sha256"],
        "frame_bundle_revision": row["frame_bundle_revision"],
        "frame_bundle_sha256": row["frame_bundle_sha256"],
        "frame_source_bytes_authenticated": frame_authenticated,
        "profile_id": row["profile_id"],
        "profile_revision": row["profile_revision"],
        "profile_sha256": row["profile_sha256"],
        "profile_source_bytes_authenticated": profile_authenticated,
        "upstream_source_bytes_authenticated": frame_authenticated and profile_authenticated,
        "source_authentication_verified_here": False,
        "sensor_mode": row["sensor_mode"],
        "frame_convention": row["frame_convention"],
        "tangent_convention": row["tangent_convention"],
        "covariance_units": COVARIANCE_UNITS,
        "nominal_level_bps": int(row["nominal_level_bps"]),
        "profile_sample_count": int(row["profile_sample_count"]),
        "profile_coverage_lower_ci_ppm": int(row["profile_coverage_lower_ci_ppm"]),
        "predicted_translation_m": [f"{value:.12f}" for value in vector(row, "pred_t", 3)],
        "predicted_quaternion_xyzw": [f"{value:.12f}" for value in vector(row, "pred_q", 4)],
        "covariance_row_major": [
            f"{float(row[f'cov_{i}{j}']):.12f}" for i in range(6) for j in range(6)
        ],
        "disposition": disposition,
        "reason_codes": [] if reason is None else [reason],
        "candidate_only": disposition == "CANDIDATE_STATISTICAL_REGION",
        "statistical_region_only": disposition == "CANDIDATE_STATISTICAL_REGION",
        "deterministic_safety_bound": False,
        "physical_calibration_validated": False,
        "source_bytes_authenticated": False,
        "maneuver_authority_granted": False,
    }


def evaluate(row: dict[str, str]) -> dict[str, object]:
    if not prediction_shape_valid(row) or not profile_shape_valid(row):
        return base_result(row, "INVALID_INPUT", "PREDICTION_OR_PROFILE_SHAPE_INVALID")

    if (
        row["sensor_mode"] != row["profile_sensor_mode"]
        or row["frame_convention"] != "WORLD_RIGHT_HANDED_METRES"
        or row["frame_convention"] != row["profile_frame_convention"]
        or row["quaternion_order"] != "XYZW"
        or row["tangent_convention"] != TANGENT
        or row["translation_unit"] != "metre"
        or row["rotation_unit"] != "radian"
    ):
        return base_result(row, "OUT_OF_SCOPE", "FRAME_UNIT_OR_SENSOR_CONVENTION_MISMATCH")

    if (
        row["bundle_sample_id"] != row["sample_id"]
        or row["bundle_frame_id"] != row["frame_id"]
        or row["bundle_target_id"] != row["target_id"]
        or row["bundle_dataset_revision"] != row["dataset_revision"]
        or row["bundle_camera_calibration_revision"] != row["camera_calibration_revision"]
    ):
        return base_result(row, "OUT_OF_SCOPE", "FRAME_BUNDLE_SCOPE_MISMATCH")

    if row["frame_bundle_state"] != "SYNCHRONIZED":
        return base_result(row, "ABSTAIN_UNCALIBRATED", "FRAME_BUNDLE_NOT_SYNCHRONIZED")
    if row["frame_source_bytes_authenticated"] != "true":
        return base_result(row, "ABSTAIN_UNCALIBRATED", "FRAME_SOURCE_BYTES_NOT_AUTHENTICATED")

    if (
        row["profile_dataset_id"] != row["dataset_id"]
        or row["profile_dataset_revision"] != row["dataset_revision"]
        or row["profile_model_revision"] != row["model_revision"]
        or row["profile_model_sha256"] != row["model_sha256"]
        or row["profile_sensor_mode"] != row["sensor_mode"]
        or row["profile_frame_convention"] != row["frame_convention"]
        or row["profile_camera_calibration_revision"] != row["camera_calibration_revision"]
        or row["profile_camera_calibration_sha256"] != row["camera_calibration_sha256"]
        or row["profile_target_population_id"] != row["target_population_id"]
        or row["profile_target_population_sha256"] != row["target_population_sha256"]
    ):
        return base_result(row, "OUT_OF_SCOPE", "CALIBRATION_PROFILE_SCOPE_MISMATCH")

    if (
        row["profile_state"] != "CURRENT"
        or row["profile_source_bytes_authenticated"] != "true"
        or row["profile_method"] != "SPLIT_CONFORMAL_SE3-v1"
        or int(row["profile_nominal_level_bps"]) != int(row["nominal_level_bps"])
        or int(row["profile_nominal_level_bps"]) != ADMISSION_BPS
        or int(row["profile_sample_count"]) < MIN_PROFILE_SAMPLES
        or int(row["profile_coverage_lower_ci_ppm"]) < int(row["required_coverage_lower_ci_ppm"])
    ):
        return base_result(row, "ABSTAIN_UNCALIBRATED", "CALIBRATION_PROFILE_NOT_CURRENT_OR_SUFFICIENT")

    if not quaternion_is_unit(vector(row, "pred_q", 4)):
        return base_result(row, "INVALID_INPUT", "PREDICTED_QUATERNION_INVALID")

    covariance = [
        float(row[f"cov_{i}{j}"]) for i in range(6) for j in range(6)
    ]
    if not covariance_is_spd(covariance):
        return base_result(
            row, "INVALID_COVARIANCE", "COVARIANCE_NOT_FINITE_SYMMETRIC_POSITIVE_DEFINITE"
        )

    support_mode = row["support_mode"]
    if support_mode not in (
        "KEYPOINT_PNP", "DENSE_PNP", "EVENT_KEYPOINT_PNP", "DIRECT_REGRESSION"
    ):
        return base_result(row, "OUT_OF_SCOPE", "POSE_SUPPORT_MODE_UNSUPPORTED")

    direct = support_mode == "DIRECT_REGRESSION"
    if (
        row["ood_flag"] == "true"
        or row["symmetry_ambiguous"] == "true"
        or float(row["visible_fraction"]) < MIN_VISIBLE
        or (
            not direct
            and (
                int(row["correspondence_count"]) < MIN_CORRESPONDENCES
                or float(row["geometry_condition"]) <= 0.0
                or float(row["geometry_condition"]) > MAX_GEOMETRY_CONDITION
            )
        )
    ):
        return base_result(row, "ABSTAIN_UNOBSERVABLE", "POSE_OBSERVABILITY_OR_DOMAIN_GATE_FAILED")

    return base_result(row, "CANDIDATE_STATISTICAL_REGION", None)


def evaluate_manifest(rows: list[dict[str, str]]) -> list[dict[str, object]]:
    seen: set[str] = set()
    duplicate: set[str] = set()
    for row in rows:
        if row["sample_id"] in seen:
            duplicate.add(row["sample_id"])
        seen.add(row["sample_id"])
    results = []
    for row in rows:
        if row["sample_id"] in duplicate:
            results.append(base_result(row, "DUPLICATE_SAMPLE", "SAMPLE_ID_DUPLICATED"))
        else:
            results.append(evaluate(row))
    return results


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
    assert len(rows) == 50, f"expected 50 cases, found {len(rows)}"
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
        "corpus_sha256": digest,
        "covariance_order": TANGENT,
        "covariance_units": COVARIANCE_UNITS,
        "case_count": len(results),
        "expected_disposition_matches": len(results),
        "expected_disposition_mismatches": 0,
        "candidate_statistical_region_count": sum(
            result["disposition"] == "CANDIDATE_STATISTICAL_REGION" for result in results
        ),
        "claim_ready": False,
        "source_bytes_authenticated": False,
        "physical_calibration_validated": False,
        "deterministic_safety_bound": False,
        "maneuver_authority_granted": False,
        "contains_ground_truth_fields": False,
        "contains_offline_evaluation_labels": False,
        "results": results,
    }
    print(json.dumps(receipt, separators=(",", ":")))
    print(f"Validated {len(results)} no-ground-truth runtime pose profiles.", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
