#!/usr/bin/env python3
"""Independent Fraction oracle for acceleration/jerk-bounded projection tubes."""

from fractions import Fraction
import hashlib
import json
import sys
from pathlib import Path

CORPUS_SCHEMA = "luminous.orbital-time-varying-acquisition.corpus.v0.1"
RECEIPT_SCHEMA = "luminous.orbital-time-varying-acquisition.receipt.v0.1"
BENCHMARK_ID = "ORBITAL-TIME-VARYING-ACQUISITION-001"
CLAIM_CEILING = "synthetic_acceleration_jerk_bounded_projective_path_envelopes_only"
MAX_INPUT = 1_000_000_000_000
MAX_TIME_NS = 1_000_000_000
MAX_JERK = 1_000_000_000
MAX_U64 = (1 << 64) - 1
NS = 1_000_000
NS2 = 1_000_000_000_000_000
NS3 = 1_000_000_000_000_000_000_000_000


def ceil_fraction(value):
    return (value.numerator + value.denominator - 1) // value.denominator


def valid_text(value):
    return isinstance(value, str) and bool(value.strip())


def valid_sha256(value):
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdefABCDEF" for character in value)
    )


def validate_case(case):
    model = case["model"]
    reasons = set()
    if not valid_text(case["case_id"]):
        reasons.add("CASE_ID_EMPTY")

    numeric_inputs = [
        "reference_point_speed_um_s",
        "reference_point_acceleration_um_s2",
        "point_jerk_upper_um_s3",
        "max_point_radius_um",
        "min_depth_um",
        "max_abs_x_um",
        "max_abs_y_um",
        "focal_x_micro_px",
        "focal_y_micro_px",
        "exposure_budget_x_micro_px",
        "exposure_budget_y_micro_px",
        "row_time_budget_x_micro_px",
        "row_time_budget_y_micro_px",
        "combined_frame_budget_x_micro_px",
        "combined_frame_budget_y_micro_px",
    ]
    if any(model[name] > MAX_INPUT for name in numeric_inputs):
        reasons.add("INPUT_EXCEEDS_DECLARED_LIMIT")
    if model["row_exposure_duration_ns"] > MAX_TIME_NS:
        reasons.add("INPUT_EXCEEDS_DECLARED_LIMIT")
    if model["first_to_last_row_midpoint_span_ns"] > MAX_TIME_NS:
        reasons.add("INPUT_EXCEEDS_DECLARED_LIMIT")
    if model["point_jerk_upper_um_s3"] > MAX_JERK:
        reasons.add("INPUT_EXCEEDS_DECLARED_LIMIT")
    if model["min_depth_um"] == 0:
        reasons.add("NON_POSITIVE_MINIMUM_DEPTH")
    if model["focal_x_micro_px"] == 0 or model["focal_y_micro_px"] == 0:
        reasons.add("NON_POSITIVE_FOCAL_LENGTH")
    if (
        model["max_point_radius_um"] < model["max_abs_x_um"]
        or model["max_point_radius_um"] < model["max_abs_y_um"]
        or model["max_point_radius_um"] < model["min_depth_um"]
    ):
        reasons.add("RADIUS_ENVELOPE_INCONSISTENT")
    for name, reason in [
        ("motion_receipt_sha256", "MOTION_RECEIPT_DIGEST_INVALID"),
        ("support_receipt_sha256", "SUPPORT_RECEIPT_DIGEST_INVALID"),
        ("timing_receipt_sha256", "TIMING_RECEIPT_DIGEST_INVALID"),
        ("calibration_receipt_sha256", "CALIBRATION_RECEIPT_DIGEST_INVALID"),
    ]:
        if not valid_sha256(model[name]):
            reasons.add(reason)
    return sorted(reasons)


def displacement_terms(speed, acceleration, jerk):
    total = speed + acceleration + jerk
    if max(speed, acceleration, jerk, total) > MAX_U64:
        raise OverflowError("ARITHMETIC_OVERFLOW")
    return {
        "speed_nm": speed,
        "acceleration_nm": acceleration,
        "jerk_nm": jerk,
        "total_nm": total,
    }


def projected_axis(focal, displacement, extent, min_depth):
    numerator = focal * displacement * (min_depth + extent)
    denominator = min_depth * min_depth * 1_000
    value = ceil_fraction(Fraction(numerator, denominator))
    if value > MAX_U64:
        raise OverflowError("ARITHMETIC_OVERFLOW")
    return value


def calculate_bounds(model):
    speed = model["reference_point_speed_um_s"]
    acceleration = model["reference_point_acceleration_um_s2"]
    jerk = model["point_jerk_upper_um_s3"]
    exposure = model["row_exposure_duration_ns"]
    row_span = model["first_to_last_row_midpoint_span_ns"]

    row_speed = ceil_fraction(Fraction(speed * row_span, NS))
    row_acceleration = ceil_fraction(
        Fraction(acceleration * row_span * row_span, 2 * NS2)
    )
    row_jerk = ceil_fraction(
        Fraction(jerk * row_span**3, 6 * NS3)
    )
    row = displacement_terms(row_speed, row_acceleration, row_jerk)

    exposure_speed = ceil_fraction(Fraction(speed * exposure, NS))
    if Fraction(2 * row_span, 1) >= Fraction(exposure, 1):
        integral_abs_time = Fraction(row_span * exposure, 1)
    else:
        integral_abs_time = Fraction(row_span * row_span, 1) + Fraction(
            exposure * exposure, 4
        )
    exposure_acceleration = ceil_fraction(
        Fraction(acceleration, 1) * integral_abs_time / NS2
    )
    integral_time_squared = (
        Fraction(exposure * row_span * row_span, 1)
        + Fraction(exposure**3, 12)
    )
    exposure_jerk = ceil_fraction(
        Fraction(jerk, 2) * integral_time_squared / NS3
    )
    latest_exposure = displacement_terms(
        exposure_speed, exposure_acceleration, exposure_jerk
    )

    # Integrate the complete frame interval [-E/2, T+E/2] directly.
    # This is intentionally not derived by adding separately rounded row and
    # latest-exposure maxima: that shortcut can under-bound when T < E/2.
    frame_duration = row_span + exposure
    frame_speed = ceil_fraction(Fraction(speed * frame_duration, NS))
    integral_abs_time_frame = (
        Fraction(exposure * exposure, 8)
        + Fraction((row_span + Fraction(exposure, 2)) ** 2, 2)
    )
    frame_acceleration = ceil_fraction(
        Fraction(acceleration, 1) * integral_abs_time_frame / NS2
    )
    integral_time_squared_frame = (
        (row_span + Fraction(exposure, 2)) ** 3
        + Fraction(exposure, 2) ** 3
    ) / 3
    frame_jerk = ceil_fraction(
        Fraction(jerk, 2) * integral_time_squared_frame / NS3
    )
    combined_frame = displacement_terms(frame_speed, frame_acceleration, frame_jerk)

    # Comparator only: apply the largest permitted instantaneous speed
    # across the entire frame. The time-varying integral remains authoritative.
    twice_max_time = 2 * row_span + exposure
    constant_speed = ceil_fraction(Fraction(speed * frame_duration, NS))
    constant_acceleration = ceil_fraction(
        Fraction(acceleration * twice_max_time * frame_duration, 2 * NS2)
    )
    constant_jerk = ceil_fraction(
        Fraction(jerk * twice_max_time**2 * frame_duration, 8 * NS3)
    )
    constant_max_speed_frame = displacement_terms(
        constant_speed, constant_acceleration, constant_jerk
    )
    constant_speed_excess = (
        constant_max_speed_frame["total_nm"] - combined_frame["total_nm"]
    )
    if constant_speed_excess < 0:
        raise OverflowError("CONSTANT_SPEED_COMPARATOR_UNDER_BOUND")

    exposure_x = projected_axis(
        model["focal_x_micro_px"],
        latest_exposure["total_nm"],
        model["max_abs_x_um"],
        model["min_depth_um"],
    )
    exposure_y = projected_axis(
        model["focal_y_micro_px"],
        latest_exposure["total_nm"],
        model["max_abs_y_um"],
        model["min_depth_um"],
    )
    row_x = projected_axis(
        model["focal_x_micro_px"],
        row["total_nm"],
        model["max_abs_x_um"],
        model["min_depth_um"],
    )
    row_y = projected_axis(
        model["focal_y_micro_px"],
        row["total_nm"],
        model["max_abs_y_um"],
        model["min_depth_um"],
    )
    combined_x = projected_axis(
        model["focal_x_micro_px"],
        combined_frame["total_nm"],
        model["max_abs_x_um"],
        model["min_depth_um"],
    )
    combined_y = projected_axis(
        model["focal_y_micro_px"],
        combined_frame["total_nm"],
        model["max_abs_y_um"],
        model["min_depth_um"],
    )

    constant_speed_frame_x = projected_axis(
        model["focal_x_micro_px"],
        constant_max_speed_frame["total_nm"],
        model["max_abs_x_um"],
        model["min_depth_um"],
    )
    constant_speed_frame_y = projected_axis(
        model["focal_y_micro_px"],
        constant_max_speed_frame["total_nm"],
        model["max_abs_y_um"],
        model["min_depth_um"],
    )

    return {
        "row_time": row,
        "latest_row_exposure": latest_exposure,
        "combined_frame": combined_frame,
        "constant_max_speed_frame": constant_max_speed_frame,
        "constant_speed_excess_nm_upper": constant_speed_excess,
        "constant_speed_frame_x_micro_px_upper": constant_speed_frame_x,
        "constant_speed_frame_y_micro_px_upper": constant_speed_frame_y,
        "exposure_x_micro_px_upper": exposure_x,
        "exposure_y_micro_px_upper": exposure_y,
        "row_time_x_micro_px_upper": row_x,
        "row_time_y_micro_px_upper": row_y,
        "combined_frame_x_micro_px_upper": combined_x,
        "combined_frame_y_micro_px_upper": combined_y,
    }


def evaluate_case(case):
    model = case["model"]
    digests = {
        "motion_receipt_sha256": model["motion_receipt_sha256"],
        "support_receipt_sha256": model["support_receipt_sha256"],
        "timing_receipt_sha256": model["timing_receipt_sha256"],
        "calibration_receipt_sha256": model["calibration_receipt_sha256"],
    }

    def result(status, bounds=None, reasons=None):
        return {
            "case_id": case["case_id"],
            "status": status,
            "bounds": bounds,
            "reason_codes": sorted(reasons or []),
            **digests,
        }

    invalid = validate_case(case)
    if invalid:
        return result("INVALID_INPUT", reasons=invalid)

    gates = [
        ("KINEMATIC_ENVELOPE", model["kinematic_envelope_state"]),
        ("SUPPORT_ENVELOPE", model["support_envelope_state"]),
        ("TIMING", model["timing_state"]),
        ("CALIBRATION", model["calibration_state"]),
    ]
    failed = {
        "GATE_FAIL:" + name for name, state in gates if state == "FAIL"
    }
    unknown = {
        "GATE_UNKNOWN:" + name for name, state in gates if state == "UNKNOWN"
    }
    if failed:
        return result("BLOCKED", reasons=failed)
    if unknown:
        return result("INDETERMINATE", reasons=unknown)

    try:
        bounds = calculate_bounds(model)
    except OverflowError:
        return result("INVALID_INPUT", reasons=["ARITHMETIC_OVERFLOW"])

    reasons = set()
    checks = [
        (bounds["exposure_x_micro_px_upper"], model["exposure_budget_x_micro_px"], "EXPOSURE_X_BUDGET_EXCEEDED"),
        (bounds["exposure_y_micro_px_upper"], model["exposure_budget_y_micro_px"], "EXPOSURE_Y_BUDGET_EXCEEDED"),
        (bounds["row_time_x_micro_px_upper"], model["row_time_budget_x_micro_px"], "ROW_TIME_X_BUDGET_EXCEEDED"),
        (bounds["row_time_y_micro_px_upper"], model["row_time_budget_y_micro_px"], "ROW_TIME_Y_BUDGET_EXCEEDED"),
        (bounds["combined_frame_x_micro_px_upper"], model["combined_frame_budget_x_micro_px"], "COMBINED_FRAME_X_BUDGET_EXCEEDED"),
        (bounds["combined_frame_y_micro_px_upper"], model["combined_frame_budget_y_micro_px"], "COMBINED_FRAME_Y_BUDGET_EXCEEDED"),
    ]
    for value, budget, reason in checks:
        if value > budget:
            reasons.add(reason)

    status = "BUDGET_EXCEEDED" if reasons else "WITHIN_DECLARED_BUDGET"
    return result(status, bounds, reasons)


def audit_corpus(raw_bytes, source_commit):
    if (
        len(source_commit) != 40
        or any(character not in "0123456789abcdefABCDEF" for character in source_commit)
    ):
        raise ValueError("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS")
    corpus = json.loads(raw_bytes)
    if (
        corpus["schema"] != CORPUS_SCHEMA
        or corpus["benchmark_id"] != BENCHMARK_ID
        or corpus["evidence_class"] != "synthetic"
        or corpus["claim_ceiling"] != CLAIM_CEILING
        or not corpus["cases"]
        or len(corpus["cases"]) > 128
    ):
        raise ValueError("CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID")

    case_ids = set()
    results = []
    expected_matches = 0
    for case in corpus["cases"]:
        if case["case_id"] in case_ids:
            raise ValueError("CASE_ID_DUPLICATE")
        case_ids.add(case["case_id"])
        result = evaluate_case(case)
        if result["status"] == case["expected_status"]:
            expected_matches += 1
        results.append(result)

    return {
        "schema": RECEIPT_SCHEMA,
        "benchmark_id": BENCHMARK_ID,
        "source_commit": source_commit.lower(),
        "input_sha256": hashlib.sha256(raw_bytes).hexdigest(),
        "evidence_class": corpus["evidence_class"],
        "claim_ceiling": corpus["claim_ceiling"],
        "case_count": len(results),
        "expected_outcome_matches": expected_matches,
        "expected_outcome_mismatches": len(results) - expected_matches,
        "authority_granted": False,
        "physical_validation": "NOT_ASSESSED",
        "cases": results,
    }


def main():
    if len(sys.argv) != 3:
        raise SystemExit(
            "usage: reference_oracle.py <corpus.json> <40-hex-source-commit>"
        )
    raw = Path(sys.argv[1]).read_bytes()
    receipt = audit_corpus(raw, sys.argv[2])
    print(json.dumps(receipt, indent=2))
    if receipt["expected_outcome_mismatches"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
