#!/usr/bin/env python3
"""Independent challenger for calibrated SE(3) pose-region construction."""

from __future__ import annotations

import csv
import hashlib
import json
import math
import sys
from pathlib import Path

SCHEMA = "luminous.spacecraft-se3-pose-region.receipt.v0.1"
LEVEL_BPS = 9500
MIN_SAMPLES = 100
Q32_SCALE = 4294967296.0
MAX_EXACT_Q32 = 9007199254740991
TANGENT = "BODY_LOCAL_TRANSLATION_XYZ_ROTATION_VECTOR_XYZ"
PERTURBATION = "RIGHT_COMPOSED_BODY_LOCAL_SE3_EXP_V1"
SCORE = "SQRT_MAHALANOBIS_BODY_LOCAL_SE3_V1"
PI_UPPER = math.nextafter(math.pi, math.inf)
MIN_I64 = -(2**63)
MAX_I64_EXCLUSIVE = 2**63
MAX_U64_EXCLUSIVE = 2**64
MICRO = 1_000_000.0


def digest(ch: str) -> str:
    return ch * 64


def valid_sha(value: str) -> bool:
    return len(value) == 64 and all(c in "0123456789abcdefABCDEF" for c in value)


def valid_text(value: str) -> bool:
    return bool(value.strip())


def next_up(value: float) -> float:
    return math.nextafter(value, math.inf)


def next_down(value: float) -> float:
    return math.nextafter(value, -math.inf)


def finite(values: list[float]) -> bool:
    return all(math.isfinite(value) for value in values)


def interval_add(left: tuple[float, float], right: tuple[float, float]) -> tuple[float, float] | None:
    lower = next_down(left[0] + right[0])
    upper = next_up(left[1] + right[1])
    return (lower, upper) if math.isfinite(lower) and math.isfinite(upper) and lower <= upper else None


def interval_sub(left: tuple[float, float], right: tuple[float, float]) -> tuple[float, float] | None:
    lower = next_down(left[0] - right[1])
    upper = next_up(left[1] - right[0])
    return (lower, upper) if math.isfinite(lower) and math.isfinite(upper) and lower <= upper else None


def interval_mul(left: tuple[float, float], right: tuple[float, float]) -> tuple[float, float] | None:
    if left == (0.0, 0.0) or right == (0.0, 0.0):
        return (0.0, 0.0)
    products = (
        left[0] * right[0],
        left[0] * right[1],
        left[1] * right[0],
        left[1] * right[1],
    )
    if not finite(list(products)):
        return None
    lower = next_down(min(products))
    upper = next_up(max(products))
    return (lower, upper) if math.isfinite(lower) and math.isfinite(upper) and lower <= upper else None


def interval_square(value: tuple[float, float]) -> tuple[float, float] | None:
    if value[0] <= 0.0 <= value[1]:
        maximum = max(abs(value[0]), abs(value[1]))
        upper = next_up(maximum * maximum)
        return (0.0, upper) if math.isfinite(upper) else None
    minimum = min(abs(value[0]), abs(value[1]))
    maximum = max(abs(value[0]), abs(value[1]))
    lower = max(0.0, next_down(minimum * minimum))
    upper = next_up(maximum * maximum)
    return (lower, upper) if math.isfinite(lower) and math.isfinite(upper) else None


def interval_div_positive(numerator: tuple[float, float], denominator: tuple[float, float]) -> tuple[float, float] | None:
    if denominator[0] <= 0.0:
        return None
    quotients = (
        numerator[0] / denominator[0],
        numerator[0] / denominator[1],
        numerator[1] / denominator[0],
        numerator[1] / denominator[1],
    )
    if not finite(list(quotients)):
        return None
    lower = next_down(min(quotients))
    upper = next_up(max(quotients))
    return (lower, upper) if math.isfinite(lower) and math.isfinite(upper) and lower <= upper else None


def interval_sqrt_positive(value: tuple[float, float]) -> tuple[float, float] | None:
    if value[0] <= 0.0 or value[1] <= 0.0:
        return None
    lower = next_down(math.sqrt(value[0]))
    upper = next_up(math.sqrt(value[1]))
    return (lower, upper) if lower > 0.0 and math.isfinite(lower) and math.isfinite(upper) else None


def cholesky_spd(matrix: list[list[float]]) -> bool:
    if not finite([value for row in matrix for value in row]):
        return False
    for row in range(6):
        for column in range(6):
            if matrix[row][column] != matrix[column][row]:
                return False

    zero = (0.0, 0.0)
    lower = [[zero for _ in range(6)] for _ in range(6)]
    for row in range(6):
        for column in range(row + 1):
            residual = (matrix[row][column], matrix[row][column])
            for index in range(column):
                if row == column:
                    product = interval_square(lower[row][index])
                else:
                    product = interval_mul(lower[row][index], lower[column][index])
                if product is None:
                    return False
                residual = interval_sub(residual, product)
                if residual is None:
                    return False

            if row == column:
                pivot = interval_sqrt_positive(residual)
                if pivot is None:
                    return False
                lower[row][column] = pivot
            else:
                factor = interval_div_positive(residual, lower[column][column])
                if factor is None:
                    return False
                lower[row][column] = factor
    return True

def make_case(row: dict[str, str]) -> dict[str, object]:
    tvar = float(row["translation_variance"])
    rvar = float(row["rotation_variance"])
    cross = float(row["cross_covariance"])
    covariance = [[0.0] * 6 for _ in range(6)]
    for axis in range(3):
        covariance[axis][axis] = tvar
        covariance[axis + 3][axis + 3] = rvar
    covariance[0][3] = cross
    covariance[3][0] = cross

    profile = {
        "profile_id": "profile-1",
        "profile_revision": "r1",
        "profile_sha256": digest("a"),
        "calibration_scores_sha256": digest("b"),
        "profile_state": "CURRENT",
        "dataset_sha256": digest("c"),
        "model_sha256": digest("d"),
        "model_config_sha256": digest("e"),
        "camera_calibration_sha256": digest("f"),
        "target_population_sha256": digest("1"),
        "calibration_split_sha256": digest("2"),
        "heldout_split_sha256": digest("3"),
        "calibration_method": "SPLIT_CONFORMAL_SE3-v1",
        "score_definition": SCORE,
        "tangent_convention": TANGENT,
        "perturbation_convention": PERTURBATION,
        "nominal_level_bps": LEVEL_BPS,
        "sample_count": int(row["sample_count"]),
        "order_statistic_rank": int(row["order_statistic_rank"]),
        "quantile_radius_q32_32": int(row["quantile_q32_32"]),
        "heldout_coverage_lower_ci_ppm": int(row["heldout_coverage_lower_ci_ppm"]),
        "required_coverage_lower_ci_ppm": int(row["required_coverage_lower_ci_ppm"]),
        "source_bytes_authenticated": True,
    }
    runtime = {
        "case_id": row["case_id"],
        "sample_id": row["sample_id"],
        "frame_id": "frame-" + row["case_id"],
        "receipt_sha256": digest("4"),
        "disposition": "CANDIDATE_STATISTICAL_REGION",
        "candidate_only": True,
        "deterministic_safety_bound": False,
        "maneuver_authority_granted": False,
        "upstream_source_bytes_authenticated": True,
        "profile_id": "profile-1",
        "profile_sha256": digest("a"),
        "dataset_sha256": digest("c"),
        "model_sha256": digest("d"),
        "model_config_sha256": digest("e"),
        "camera_calibration_sha256": digest("f"),
        "target_population_sha256": digest("1"),
        "frame_convention": "WORLD_RIGHT_HANDED_METRES",
        "quaternion_order": "XYZW",
        "tangent_convention": TANGENT,
        "translation_unit": "metre",
        "rotation_unit": "radian",
        "nominal_level_bps": LEVEL_BPS,
        "predicted_translation_m": [
            float(row["tx"]), float(row["ty"]), float(row["tz"])
        ],
        "predicted_quaternion_xyzw": [0.0, 0.0, 0.0, 1.0],
        "covariance": covariance,
        "profile": profile,
    }

    mutation = row["mutation"]
    if mutation == "none":
        pass
    elif mutation == "weak_coverage":
        profile["heldout_coverage_lower_ci_ppm"] = 899999
    elif mutation == "stale_profile":
        profile["profile_state"] = "STALE"
    elif mutation == "insufficient_samples":
        profile["sample_count"] = 99
        profile["order_statistic_rank"] = 95
    elif mutation == "wrong_rank":
        profile["order_statistic_rank"] = 950
    elif mutation == "missing_auth":
        runtime["upstream_source_bytes_authenticated"] = False
    elif mutation == "profile_auth_missing":
        profile["source_bytes_authenticated"] = False
    elif mutation == "not_candidate":
        runtime["disposition"] = "ABSTAIN_UNOBSERVABLE"
        runtime["candidate_only"] = False
    elif mutation == "authority_escalation":
        runtime["deterministic_safety_bound"] = True
    elif mutation == "profile_digest_mismatch":
        runtime["profile_sha256"] = digest("9")
    elif mutation == "dataset_scope_mismatch":
        profile["dataset_sha256"] = digest("9")
    elif mutation == "wrong_score":
        profile["score_definition"] = "GAUSSIAN_CHI_SQUARE"
    elif mutation == "wrong_tangent":
        profile["tangent_convention"] = "WORLD_TANGENT"
    elif mutation == "wrong_perturbation":
        profile["perturbation_convention"] = "LEFT_COMPOSED"
    elif mutation == "wrong_level":
        runtime["nominal_level_bps"] = 9000
        profile["nominal_level_bps"] = 9000
    elif mutation == "bad_quantile":
        profile["quantile_radius_q32_32"] = 2**64 - 1
    elif mutation == "invalid_quaternion":
        runtime["predicted_quaternion_xyzw"] = [0.0, 0.0, 0.0, 0.0]
    elif mutation == "asymmetric_covariance":
        covariance[0][3] = 1.0e-6
    elif mutation == "non_spd_covariance":
        covariance[0][0] = -1.0
    elif mutation == "singular_covariance":
        covariance[0][0] = 0.0
    elif mutation == "nonfinite_covariance":
        covariance[0][0] = math.nan
    elif mutation == "overflow_translation":
        for axis in range(3):
            covariance[axis][axis] = 1.0e308
    elif mutation == "negative_quaternion":
        runtime["predicted_quaternion_xyzw"] = [0.0, 0.0, 0.0, -1.0]
    elif mutation == "wide_orientation":
        for axis in range(3, 6):
            covariance[axis][axis] = 9.0
    elif mutation == "bad_split_identity":
        profile["heldout_split_sha256"] = profile["calibration_split_sha256"]
    elif mutation == "bad_digest":
        runtime["receipt_sha256"] = "bad"
    else:
        raise ValueError(f"unknown mutation: {mutation}")
    return {"expected": row["expected"], "runtime": runtime}


def empty_result(runtime: dict[str, object], disposition: str, reason: str) -> dict[str, object]:
    profile = runtime["profile"]
    return {
        "case_id": runtime["case_id"],
        "sample_id": runtime["sample_id"],
        "frame_id": runtime["frame_id"],
        "runtime_receipt_sha256": runtime["receipt_sha256"],
        "profile_id": profile["profile_id"],
        "profile_sha256": profile["profile_sha256"],
        "calibration_scores_sha256": profile["calibration_scores_sha256"],
        "nominal_level_bps": profile["nominal_level_bps"],
        "calibration_sample_count": profile["sample_count"],
        "order_statistic_rank": profile["order_statistic_rank"],
        "quantile_radius_q32_32": profile["quantile_radius_q32_32"],
        "disposition": disposition,
        "reason_code": reason,
        "region": None,
        "region_is_statistical": False,
        "outer_approximation_of_joint_tangent_region": False,
        "source_authentication_verified_here": False,
        "source_bytes_authenticated": False,
        "physical_calibration_validated": False,
        "deterministic_safety_bound": False,
        "maneuver_authority_granted": False,
    }


def conformal_rank(count: int, level: int) -> int | None:
    if level <= 0 or level > 10000:
        return None
    return ((count + 1) * level + 9999) // 10000


def gershgorin_upper(matrix: list[list[float]], offset: int) -> float | None:
    maximum = 0.0
    for row in range(3):
        total = 0.0
        for column in range(3):
            total = next_up(total + abs(matrix[offset + row][offset + column]))
            if not math.isfinite(total):
                return None
        maximum = max(maximum, total)
    return next_up(maximum)


def sqrt_upper(value: float) -> float | None:
    if not math.isfinite(value) or value < 0:
        return None
    result = next_up(math.sqrt(next_up(value)))
    return result if math.isfinite(result) else None


def upper_mul(left: float, right: float) -> float | None:
    result = next_up(left * right)
    return result if math.isfinite(result) else None


def lower_um(center: float, radius: float) -> int | None:
    value = next_down(center - radius)
    if not math.isfinite(value):
        return None
    scaled = next_down(value * MICRO)
    if not math.isfinite(scaled):
        return None
    integer = math.floor(scaled)
    return integer if MIN_I64 <= integer < MAX_I64_EXCLUSIVE else None


def upper_um(center: float, radius: float) -> int | None:
    value = next_up(center + radius)
    if not math.isfinite(value):
        return None
    scaled = next_up(value * MICRO)
    if not math.isfinite(scaled):
        return None
    integer = math.ceil(scaled)
    return integer if MIN_I64 <= integer < MAX_I64_EXCLUSIVE else None


def orientation_cap_urad(radius: float) -> int | None:
    if not math.isfinite(radius) or radius < 0:
        return None
    scaled = next_up(min(radius, PI_UPPER) * MICRO)
    if not math.isfinite(scaled) or scaled < 0 or scaled >= MAX_U64_EXCLUSIVE:
        return None
    return math.ceil(scaled)


def evaluate(runtime: dict[str, object]) -> dict[str, object]:
    profile = runtime["profile"]
    if (
        any(not valid_text(str(runtime[field])) for field in (
            "case_id", "sample_id", "frame_id", "disposition", "profile_id",
            "frame_convention", "quaternion_order", "tangent_convention",
            "translation_unit", "rotation_unit",
        ))
        or any(not valid_sha(str(runtime[field])) for field in (
            "receipt_sha256", "profile_sha256", "dataset_sha256", "model_sha256",
            "model_config_sha256", "camera_calibration_sha256", "target_population_sha256",
        ))
        or not finite(runtime["predicted_translation_m"])
        or not 0 < runtime["nominal_level_bps"] <= 10000
    ):
        return empty_result(runtime, "INVALID_INPUT", "RUNTIME_OR_PROFILE_SHAPE_INVALID")

    if (
        any(not valid_text(str(profile[field])) for field in (
            "profile_id", "profile_revision", "profile_state", "calibration_method",
            "score_definition", "tangent_convention", "perturbation_convention",
        ))
        or any(not valid_sha(str(profile[field])) for field in (
            "profile_sha256", "calibration_scores_sha256", "dataset_sha256",
            "model_sha256", "model_config_sha256", "camera_calibration_sha256",
            "target_population_sha256", "calibration_split_sha256", "heldout_split_sha256",
        ))
        or profile["calibration_split_sha256"] == profile["heldout_split_sha256"]
        or not 0 < profile["nominal_level_bps"] <= 10000
        or profile["heldout_coverage_lower_ci_ppm"] > 1_000_000
        or profile["required_coverage_lower_ci_ppm"] > 1_000_000
        or profile["quantile_radius_q32_32"] > MAX_EXACT_Q32
    ):
        return empty_result(runtime, "INVALID_INPUT", "RUNTIME_OR_PROFILE_SHAPE_INVALID")

    if runtime["disposition"] != "CANDIDATE_STATISTICAL_REGION" or not runtime["candidate_only"]:
        return empty_result(runtime, "ABSTAIN_RUNTIME_POSE_NOT_CANDIDATE", "RUNTIME_POSE_RECEIPT_NOT_CANDIDATE_ONLY")
    if runtime["deterministic_safety_bound"] or runtime["maneuver_authority_granted"]:
        return empty_result(runtime, "INVALID_INPUT", "RUNTIME_RECEIPT_ATTEMPTS_AUTHORITY_ESCALATION")

    if (
        runtime["frame_convention"] != "WORLD_RIGHT_HANDED_METRES"
        or runtime["quaternion_order"] != "XYZW"
        or runtime["tangent_convention"] != TANGENT
        or runtime["translation_unit"] != "metre"
        or runtime["rotation_unit"] != "radian"
        or profile["tangent_convention"] != TANGENT
        or profile["perturbation_convention"] != PERTURBATION
        or profile["score_definition"] != SCORE
        or profile["calibration_method"] != "SPLIT_CONFORMAL_SE3-v1"
        or profile["nominal_level_bps"] != LEVEL_BPS
        or runtime["nominal_level_bps"] != LEVEL_BPS
    ):
        return empty_result(runtime, "OUT_OF_SCOPE", "UNSUPPORTED_REGION_OR_TANGENT_CONVENTION")

    if (
        runtime["profile_id"] != profile["profile_id"]
        or runtime["profile_sha256"] != profile["profile_sha256"]
        or runtime["dataset_sha256"] != profile["dataset_sha256"]
        or runtime["model_sha256"] != profile["model_sha256"]
        or runtime["model_config_sha256"] != profile["model_config_sha256"]
        or runtime["camera_calibration_sha256"] != profile["camera_calibration_sha256"]
        or runtime["target_population_sha256"] != profile["target_population_sha256"]
    ):
        return empty_result(runtime, "OUT_OF_SCOPE", "RUNTIME_PROFILE_SCOPE_OR_DIGEST_MISMATCH")

    if not runtime["upstream_source_bytes_authenticated"] or not profile["source_bytes_authenticated"]:
        return empty_result(runtime, "ABSTAIN_UNCALIBRATED", "UPSTREAM_SOURCE_AUTHENTICATION_CLAIM_MISSING")
    if (
        profile["profile_state"] != "CURRENT"
        or profile["heldout_coverage_lower_ci_ppm"] < profile["required_coverage_lower_ci_ppm"]
    ):
        return empty_result(runtime, "ABSTAIN_UNCALIBRATED", "PROFILE_STALE_OR_HELDOUT_COVERAGE_TOO_WEAK")
    if profile["sample_count"] < MIN_SAMPLES:
        return empty_result(runtime, "ABSTAIN_UNCALIBRATED", "INSUFFICIENT_CALIBRATION_SAMPLE_COUNT")

    expected_rank = conformal_rank(profile["sample_count"], profile["nominal_level_bps"])
    if expected_rank is None:
        return empty_result(runtime, "INVALID_INPUT", "CONFORMAL_RANK_UNREPRESENTABLE")
    if expected_rank > profile["sample_count"]:
        return empty_result(runtime, "ABSTAIN_UNCALIBRATED", "NO_FINITE_SPLIT_CONFORMAL_ORDER_STATISTIC")
    if profile["order_statistic_rank"] != expected_rank:
        return empty_result(runtime, "INVALID_INPUT", "CONFORMAL_ORDER_STATISTIC_RANK_MISMATCH")
    if profile["quantile_radius_q32_32"] > MAX_EXACT_Q32:
        return empty_result(runtime, "INVALID_INPUT", "Q32_32_QUANTILE_EXCEEDS_EXACT_CONVERSION_RANGE")

    quaternion = runtime["predicted_quaternion_xyzw"]
    if (
        not finite(quaternion)
        or sum(value * value for value in quaternion) <= 1.0e-24
        or abs(math.sqrt(sum(value * value for value in quaternion)) - 1.0) > 1.0e-6
    ):
        return empty_result(runtime, "INVALID_INPUT", "PREDICTED_QUATERNION_NOT_UNIT_XYZW")

    matrix = runtime["covariance"]
    if not cholesky_spd(matrix):
        return empty_result(runtime, "INVALID_COVARIANCE", "COVARIANCE_NOT_FINITE_SYMMETRIC_POSITIVE_DEFINITE")

    q = float(profile["quantile_radius_q32_32"]) / Q32_SCALE
    t_lambda = gershgorin_upper(matrix, 0)
    r_lambda = gershgorin_upper(matrix, 3)
    if t_lambda is None or r_lambda is None:
        return empty_result(runtime, "OVERFLOW", "COVARIANCE_ROW_SUM_OVERFLOW")
    t_root = sqrt_upper(t_lambda)
    r_root = sqrt_upper(r_lambda)
    if t_root is None or r_root is None:
        return empty_result(runtime, "OVERFLOW", "COVARIANCE_SQUARE_ROOT_OVERFLOW")
    t_radius = upper_mul(q, t_root)
    r_radius = upper_mul(q, r_root)
    if t_radius is None or r_radius is None:
        return empty_result(runtime, "OVERFLOW", "CONFORMAL_REGION_RADIUS_OVERFLOW")

    bounds = []
    for center in runtime["predicted_translation_m"]:
        lower = lower_um(center, t_radius)
        upper = upper_um(center, t_radius)
        if lower is None or upper is None:
            return empty_result(runtime, "OVERFLOW", "TRANSLATION_BOUND_CONVERSION_OVERFLOW")
        if lower > upper:
            return empty_result(runtime, "INVALID_INPUT", "TRANSLATION_INTERVAL_ORDER_INVALID")
        bounds.append([lower, upper])
    cap = orientation_cap_urad(r_radius)
    if cap is None:
        return empty_result(runtime, "OVERFLOW", "ORIENTATION_CAP_CONVERSION_OVERFLOW")

    result = empty_result(runtime, "CANDIDATE_STATISTICAL_POSE_REGION", "CALIBRATED_TANGENT_REGION_OUTER_APPROXIMATION_CONSTRUCTED")
    result["region"] = {"translation_bounds_um": bounds, "orientation_cap_urad": cap}
    result["region_is_statistical"] = True
    result["outer_approximation_of_joint_tangent_region"] = True
    return result


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit("usage: reference_oracle.py <corpus.csv> <source-commit> <corpus-sha256>")
    path = Path(sys.argv[1])
    raw = path.read_bytes()
    actual_digest = hashlib.sha256(raw).hexdigest()
    if actual_digest != sys.argv[3]:
        raise SystemExit("corpus SHA-256 argument does not match input bytes")

    with path.open(newline="", encoding="utf-8") as stream:
        reader = csv.DictReader(stream)
        rows = [make_case(row) for row in reader]

    counts: dict[str, int] = {}
    for case in rows:
        sample = case["runtime"]["sample_id"]
        counts[sample] = counts.get(sample, 0) + 1

    results = []
    matches = 0
    for case in rows:
        runtime = case["runtime"]
        if counts[runtime["sample_id"]] > 1:
            result = empty_result(runtime, "DUPLICATE_SAMPLE", "SAMPLE_ID_DUPLICATED_IN_MANIFEST")
        else:
            result = evaluate(runtime)
        result["expected_disposition"] = case["expected"]
        matches += result["disposition"] == case["expected"]
        results.append(result)

    receipt = {
        "schema": SCHEMA,
        "source_commit": sys.argv[2],
        "corpus_sha256": actual_digest,
        "case_count": len(results),
        "expected_disposition_matches": matches,
        "expected_disposition_mismatches": len(results) - matches,
        "claim_ready": False,
        "source_bytes_authenticated": False,
        "physical_calibration_validated": False,
        "deterministic_safety_bound": False,
        "maneuver_authority_granted": False,
        "results": results,
    }
    print(json.dumps(receipt, sort_keys=True, separators=(",", ":"), allow_nan=False))
    return 0 if matches == len(results) and results else 1


if __name__ == "__main__":
    raise SystemExit(main())
