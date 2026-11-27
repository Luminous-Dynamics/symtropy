#!/usr/bin/env python3
"""Independent standard-library oracle for SE(3) pose trustworthiness."""

from __future__ import annotations

import csv
import hashlib
import json
import math
import sys
from collections import defaultdict
from pathlib import Path

SCHEMA = "luminous.se3-pose-trustworthiness.receipt.v0.1"
ORDER = "BODY_LOCAL_TRANSLATION_XYZ_ROTATION_VECTOR_XYZ"
UNIT = "m,m,m,rad,rad,rad"
MIN_VISIBLE = 0.05
MIN_CORRESPONDENCES = 6
MAX_CONDITION = 100_000_000.0
ADMISSION_BPS = 9500
SYMMETRY_TOL = 1.0e-10
CHI2 = [
    (5000, 5.348120627447118),
    (8000, 8.558059720250295),
    (9000, 10.644640675668422),
    (9500, 12.591587243743977),
    (9900, 16.811893829770927),
]
THRESHOLDS = [0, 250000, 500000, 750000, 900000, 950000, 990000, 1000000]


def text_ok(value: str) -> bool:
    return bool(value.strip())


def valid_sha(value: str) -> bool:
    return len(value) == 64 and all(ch in "0123456789abcdefABCDEF" for ch in value)


def f64(row: dict[str, str], name: str) -> float:
    return float(row[name])


def vec(row: dict[str, str], prefix: str, count: int) -> list[float]:
    return [f64(row, f"{prefix}{axis}") for axis in (("x", "y", "z", "w") if count == 4 else ("x", "y", "z"))]


def finite(values: list[float]) -> bool:
    return all(math.isfinite(value) for value in values)


def q_norm(q: list[float]) -> float:
    return math.sqrt(sum(value * value for value in q))


def normalized_quaternion(q: list[float]) -> list[float] | None:
    if not finite(q):
        return None
    norm = q_norm(q)
    if not math.isfinite(norm) or norm <= 1.0e-12 or abs(norm - 1.0) > 1.0e-6:
        return None
    return [value / norm for value in q]


def q_conjugate(q: list[float]) -> list[float]:
    return [-q[0], -q[1], -q[2], q[3]]


def q_multiply(a: list[float], b: list[float]) -> list[float]:
    ax, ay, az, aw = a
    bx, by, bz, bw = b
    return [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]


def rotate(q: list[float], v: list[float]) -> list[float]:
    u = q[:3]
    uv = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ]
    uuv = [
        u[1] * uv[2] - u[2] * uv[1],
        u[2] * uv[0] - u[0] * uv[2],
        u[0] * uv[1] - u[1] * uv[0],
    ]
    return [
        v[0] + 2.0 * (q[3] * uv[0] + uuv[0]),
        v[1] + 2.0 * (q[3] * uv[1] + uuv[1]),
        v[2] + 2.0 * (q[3] * uv[2] + uuv[2]),
    ]


def rotation_log(q: list[float]) -> list[float]:
    canonical = q[:]
    negate = canonical[3] < 0.0
    if canonical[3] == 0.0:
        first_nonzero = next((value for value in canonical[:3] if abs(value) > 1.0e-15), None)
        negate = first_nonzero is not None and first_nonzero < 0.0
    if negate:
        canonical = [-value for value in canonical]
    vnorm = math.sqrt(sum(value * value for value in canonical[:3]))
    if vnorm <= 1.0e-12:
        return [2.0 * canonical[i] for i in range(3)]
    angle = 2.0 * math.atan2(vnorm, max(-1.0, min(1.0, canonical[3])))
    return [canonical[i] * angle / vnorm for i in range(3)]


def covariance_from_row(row: dict[str, str]) -> list[list[float]]:
    flat = [float(row[f"cov_{i}{j}"]) for i in range(6) for j in range(6)]
    return [flat[6 * i:6 * i + 6] for i in range(6)]


def cholesky(matrix: list[list[float]]) -> list[list[float]] | None:
    flat = [value for row in matrix for value in row]
    if not finite(flat):
        return None
    scale = max(abs(value) for value in flat)
    if not math.isfinite(scale) or scale <= 0.0:
        return None
    for i in range(6):
        for j in range(6):
            if abs(matrix[i][j] - matrix[j][i]) > SYMMETRY_TOL * scale:
                return None

    lower = [[0.0] * 6 for _ in range(6)]
    for i in range(6):
        for j in range(i + 1):
            total = matrix[i][j]
            for k in range(j):
                total -= lower[i][k] * lower[j][k]
            if i == j:
                if not math.isfinite(total) or total <= sys.float_info.epsilon * scale * 16.0:
                    return None
                lower[i][j] = math.sqrt(total)
            else:
                pivot = lower[j][j]
                if pivot <= 0.0 or not math.isfinite(pivot):
                    return None
                lower[i][j] = total / pivot
    return lower


def mahalanobis_squared(lower: list[list[float]], error: list[float]) -> float | None:
    solved = [0.0] * 6
    for i in range(6):
        value = error[i]
        for j in range(i):
            value -= lower[i][j] * solved[j]
        value /= lower[i][i]
        if not math.isfinite(value):
            return None
        solved[i] = value
    result = sum(value * value for value in solved)
    return result if math.isfinite(result) else None


def base_result(row: dict[str, str], disposition: str, reason: str | None) -> dict[str, object]:
    decision = row["decision"]
    measured = row["compute_state"] == "MEASURED"
    return {
        "case_id": row["case_id"],
        "sample_id": row["sample_id"],
        "target_id": row["target_id"],
        "split": row["split"],
        "dataset_id": row["dataset_id"],
        "dataset_revision": row["dataset_revision"],
        "dataset_sha256": row["dataset_sha256"],
        "model_revision": row["model_revision"],
        "model_sha256": row["model_sha256"],
        "calibration_revision": row["calibration_revision"],
        "calibration_sha256": row["calibration_sha256"],
        "sensor_mode": row["sensor_mode"],
        "support_mode": row["support_mode"],
        "frame_convention": row["frame_convention"],
        "tangent_convention": row["tangent_convention"],
        "decision": decision,
        "decision_reason": row["abstain_reason"],
        "disposition": disposition,
        "translation_error_m": None,
        "rotation_error_rad": None,
        "mahalanobis_squared": None,
        "gaussian_negative_log_likelihood": None,
        "covariance_log_determinant": None,
        "prediction_region_log_volume_95": None,
        "coverage_50": None,
        "coverage_80": None,
        "coverage_90": None,
        "coverage_95": None,
        "coverage_99": None,
        "confidence_score_ppm": int(row["confidence_score_ppm"]),
        "accepted_by_model": decision == "ACCEPT",
        "observability_policy_violation": False,
        "false_confident_95": False,
        "benchmark_admissible": False,
        "visible_fraction": f'{float(row["visible_fraction"]):.12f}',
        "correspondence_count": int(row["correspondence_count"]),
        "geometry_condition": f'{float(row["geometry_condition"]):.12f}',
        "ood_flag": row["ood_flag"] == "true",
        "symmetry_ambiguous": row["symmetry_ambiguous"] == "true",
        "latency_us": int(row["latency_us"]) if measured else None,
        "energy_uj": int(row["energy_uj"]) if measured else None,
        "reason_codes": [] if reason is None else [reason],
        "source_bytes_authenticated": False,
        "physical_calibration_validated": False,
        "maneuver_authority_granted": False,
    }


def evaluate(row: dict[str, str]) -> dict[str, object]:
    required = (
        "case_id", "sample_id", "target_id", "dataset_id", "dataset_revision",
        "model_revision", "calibration_revision",
    )
    if any(not text_ok(row[name]) for name in required):
        return base_result(row, "INVALID_INPUT", "IDENTITY_OR_REASON_INVALID")

    if (
        not valid_sha(row["dataset_sha256"])
        or not valid_sha(row["model_sha256"])
        or not valid_sha(row["calibration_sha256"])
        or row["split"] not in ("train", "validation", "test")
        or int(row["confidence_score_ppm"]) > 1_000_000
        or int(row["admission_confidence_bps"]) != ADMISSION_BPS
        or (row["decision"] == "ABSTAIN" and not text_ok(row["abstain_reason"]))
        or (row["decision"] == "ACCEPT" and row["abstain_reason"] != "")
    ):
        return base_result(row, "INVALID_INPUT", "IDENTITY_CONFIDENCE_OR_DECISION_INVALID")

    if (
        row["dataset_id"] != "SYNTHETIC-SE3-TRUST-v0.1"
        or row["sensor_mode"] != "RGB"
        or row["support_mode"] not in ("KEYPOINT_PNP", "DENSE_PNP", "EVENT_KEYPOINT_PNP", "DIRECT_REGRESSION")
        or row["frame_convention"] != "WORLD_RIGHT_HANDED_METRES"
        or row["quaternion_order"] != "XYZW"
        or row["tangent_convention"] != ORDER
        or row["translation_unit"] != "metre"
        or row["rotation_unit"] != "radian"
    ):
        return base_result(row, "OUT_OF_SCOPE", "FRAME_UNIT_OR_SENSOR_CONVENTION_UNSUPPORTED")

    pred_t = vec(row, "pred", 3)
    gt_t = vec(row, "gt", 3)
    visible = float(row["visible_fraction"])
    condition = float(row["geometry_condition"])
    if (
        not finite([*pred_t, *gt_t, visible, condition])
        or not (0.0 <= visible <= 1.0)
        or (row["support_mode"] != "DIRECT_REGRESSION" and condition <= 0.0)
        or (row["support_mode"] == "DIRECT_REGRESSION" and condition < 0.0)
    ):
        return base_result(row, "OUT_OF_SCOPE", "FRAME_UNIT_OR_SENSOR_CONVENTION_UNSUPPORTED")

    latency = int(row["latency_us"])
    energy = int(row["energy_uj"])
    compute_state = row["compute_state"]
    if compute_state == "MEASURED" and (latency == 0 or energy == 0):
        return base_result(row, "INVALID_INPUT", "MEASURED_COMPUTE_REQUIRES_POSITIVE_LATENCY_AND_ENERGY")
    if compute_state == "NOT_MEASURED" and (latency != 0 or energy != 0):
        return base_result(row, "INVALID_INPUT", "UNMEASURED_COMPUTE_MUST_NOT_CARRY_NUMERIC_CLAIMS")

    pred_q = normalized_quaternion(vec(row, "pred_q", 4))
    if pred_q is None:
        return base_result(row, "INVALID_INPUT", "PREDICTED_QUATERNION_INVALID")
    gt_q = normalized_quaternion(vec(row, "gt_q", 4))
    if gt_q is None:
        return base_result(row, "INVALID_INPUT", "GROUND_TRUTH_QUATERNION_INVALID")

    cov = covariance_from_row(row)
    lower = cholesky(cov)
    if lower is None:
        return base_result(row, "INVALID_COVARIANCE", "COVARIANCE_NOT_FINITE_SYMMETRIC_POSITIVE_DEFINITE")

    delta_world = [gt_t[i] - pred_t[i] for i in range(3)]
    body_translation = rotate(q_conjugate(pred_q), delta_world)
    relative_rotation = q_multiply(q_conjugate(pred_q), gt_q)
    rotation_vector = rotation_log(relative_rotation)
    error = [*body_translation, *rotation_vector]
    if not finite(error):
        return base_result(row, "INVALID_INPUT", "SE3_TANGENT_RESIDUAL_NONFINITE")

    mahal = mahalanobis_squared(lower, error)
    if mahal is None:
        return base_result(row, "INVALID_COVARIANCE", "MAHALANOBIS_SOLVE_FAILED")

    translation_error = math.sqrt(sum(value * value for value in delta_world))
    rotation_error = math.sqrt(sum(value * value for value in rotation_vector))
    coverages = [mahal <= threshold for _, threshold in CHI2]
    covariance_logdet = 2.0 * sum(math.log(lower[index][index]) for index in range(6))
    gaussian_nll = 0.5 * (mahal + covariance_logdet + 6.0 * math.log(2.0 * math.pi))
    region_log_volume_95 = (
        math.log(math.pi ** 3 / 6.0)
        + 0.5 * covariance_logdet
        + 3.0 * math.log(CHI2[3][1])
    )
    ood = row["ood_flag"] == "true"
    symmetry = row["symmetry_ambiguous"] == "true"
    corr = int(row["correspondence_count"])
    requires_correspondence_geometry = row["support_mode"] in (
        "KEYPOINT_PNP", "DENSE_PNP", "EVENT_KEYPOINT_PNP"
    )
    observability = (
        ood or symmetry or visible < MIN_VISIBLE
        or (
            requires_correspondence_geometry
            and (corr < MIN_CORRESPONDENCES or condition > MAX_CONDITION)
        )
    )
    accepted = row["decision"] == "ACCEPT"
    false_confident = accepted and (not coverages[3] or observability)
    result = base_result(
        row,
        "ABSTAINED" if not accepted else "EVALUATED",
        None,
    )
    result.update(
        {
            "translation_error_m": f"{translation_error:.12f}",
            "rotation_error_rad": f"{rotation_error:.12f}",
            "mahalanobis_squared": f"{mahal:.12f}",
            "gaussian_negative_log_likelihood": f"{gaussian_nll:.12f}",
            "covariance_log_determinant": f"{covariance_logdet:.12f}",
            "prediction_region_log_volume_95": f"{region_log_volume_95:.12f}",
            "coverage_50": coverages[0],
            "coverage_80": coverages[1],
            "coverage_90": coverages[2],
            "coverage_95": coverages[3],
            "coverage_99": coverages[4],
            "observability_policy_violation": observability,
            "false_confident_95": false_confident,
            "benchmark_admissible": accepted and not false_confident,
        }
    )
    if false_confident:
        result["reason_codes"] = [
            "ACCEPT_OUTSIDE_DECLARED_95_PERCENT_REGION"
            if not coverages[3]
            else "ACCEPT_WITH_OOD_OR_UNOBSERVABLE_GEOMETRY"
        ]
    return result


def evaluate_manifest(rows: list[dict[str, str]]) -> list[dict[str, object]]:
    seen_samples: set[str] = set()
    duplicate_samples: set[str] = set()
    target_splits: dict[str, set[str]] = defaultdict(set)
    for row in rows:
        if row["sample_id"] in seen_samples:
            duplicate_samples.add(row["sample_id"])
        seen_samples.add(row["sample_id"])
        target_splits[row["target_id"]].add(row["split"])
    leaked_targets = {
        target for target, splits in target_splits.items() if len(splits) > 1
    }
    results = []
    for row in rows:
        if row["target_id"] in leaked_targets:
            result = base_result(row, "SPLIT_LEAKAGE", "TARGET_ID_PRESENT_IN_MULTIPLE_SPLITS")
        elif row["sample_id"] in duplicate_samples:
            result = base_result(row, "DUPLICATE_SAMPLE", "SAMPLE_ID_DUPLICATED")
        else:
            result = evaluate(row)
        results.append(result)
    return results


def percentile(values: list[float], p: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    index = math.ceil((len(ordered) - 1) * p)
    return ordered[index]


def fmt_optional(values: list[float] | None, p: float | None = None) -> str | None:
    if values is None or not values:
        return None
    assert p is not None
    value = percentile(values, p)
    return None if value is None else f"{value:.12f}"


def summarize(results: list[dict[str, object]]) -> dict[str, object]:
    valid = [
        result for result in results
        if result["disposition"] in ("EVALUATED", "ABSTAINED")
    ]
    accepted = [result for result in valid if result["accepted_by_model"]]
    false_count = sum(bool(result["false_confident_95"]) for result in accepted)

    def coverage(key: str) -> str | None:
        if not accepted:
            return None
        value = sum(result[key] is True for result in accepted) / len(accepted)
        return f"{value:.12f}"

    translations = [
        float(result["translation_error_m"]) for result in accepted
        if result["translation_error_m"] is not None
    ]
    rotations = [
        float(result["rotation_error_rad"]) for result in accepted
        if result["rotation_error_rad"] is not None
    ]
    gaussian_nll = [
        float(result["gaussian_negative_log_likelihood"]) for result in accepted
        if result["gaussian_negative_log_likelihood"] is not None
    ]
    covariance_logd = [
        float(result["covariance_log_determinant"]) for result in accepted
        if result["covariance_log_determinant"] is not None
    ]
    region_log_volume = [
        float(result["prediction_region_log_volume_95"]) for result in accepted
        if result["prediction_region_log_volume_95"] is not None
    ]
    risk = []
    for threshold in THRESHOLDS:
        selected = [
            result for result in valid
            if result["accepted_by_model"]
            and int(result["confidence_score_ppm"]) >= threshold
        ]
        false = sum(bool(result["false_confident_95"]) for result in selected)
        risk.append(
            {
                "confidence_threshold_ppm": threshold,
                "selected_count": len(selected),
                "coverage_fraction": 0.0 if not valid else len(selected) / len(valid),
                "false_confident_count": false,
                "selective_risk": None if not selected else f"{false / len(selected):.12f}",
            }
        )
    return {
        "valid_count": len(valid),
        "accepted_count": len(accepted),
        "accepted_pose_coverage_fraction": None if not valid else f"{len(accepted) / len(valid):.12f}",
        "abstained_count": len(valid) - len(accepted),
        "invalid_count": len(results) - len(valid),
        "accepted_false_confident_95_count": false_count,
        "accepted_coverage_50": coverage("coverage_50"),
        "accepted_coverage_80": coverage("coverage_80"),
        "accepted_coverage_90": coverage("coverage_90"),
        "accepted_coverage_95": coverage("coverage_95"),
        "accepted_coverage_99": coverage("coverage_99"),
        "accepted_translation_p95_m": fmt_optional(translations, 0.95),
        "accepted_rotation_p95_rad": fmt_optional(rotations, 0.95),
        "accepted_gaussian_nll_mean": None if not gaussian_nll else f"{sum(gaussian_nll) / len(gaussian_nll):.12f}",
        "accepted_covariance_logdet_median": fmt_optional(covariance_logd, 0.5),
        "accepted_95_region_log_volume_median": fmt_optional(region_log_volume, 0.5),
        "risk_coverage": risk,
    }


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit("usage: reference_oracle.py <corpus.csv> <source_commit> <corpus_sha256>")

    corpus = Path(sys.argv[1])
    source_commit = sys.argv[2]
    expected_digest = sys.argv[3]
    raw = corpus.read_bytes()
    actual_digest = hashlib.sha256(raw).hexdigest()
    assert actual_digest == expected_digest, "corpus digest mismatch"

    with corpus.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))

    assert len(rows) == 45, f"expected 45 cases, found {len(rows)}"
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
        "covariance_order": ORDER,
        "covariance_units": UNIT,
        "admission_level_bps": ADMISSION_BPS,
        "case_count": len(results),
        "expected_disposition_matches": len(results),
        "expected_disposition_mismatches": 0,
        "claim_ready": False,
        "source_bytes_authenticated": False,
        "physical_calibration_validated": False,
        "maneuver_authority_granted": False,
        "summary": summarize(results),
        "results": results,
    }
    print(json.dumps(receipt, separators=(",", ":")))
    print(f"Validated {len(results)} SE(3) trustworthiness cases independently.", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
