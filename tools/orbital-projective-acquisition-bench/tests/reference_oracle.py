#!/usr/bin/env python3
"""Independent Fraction-based oracle for projective acquisition bounds."""

from fractions import Fraction
import hashlib
import json
import sys
from pathlib import Path

CORPUS_SCHEMA = "luminous.orbital-projective-acquisition.corpus.v0.1"
RECEIPT_SCHEMA = "luminous.orbital-projective-acquisition.receipt.v0.1"
BENCHMARK_ID = "ORBITAL-PROJECTIVE-ACQUISITION-001"
CLAIM_CEILING = "synthetic_outward_rounded_projective_motion_bounds_only"
MAX_INPUT = 1_000_000_000_000
MAX_U64 = (1 << 64) - 1


def ceil_fraction(value):
    numerator = value.numerator
    denominator = value.denominator
    return (numerator + denominator - 1) // denominator


def valid_text(value):
    return isinstance(value, str) and bool(value.strip())


def valid_sha256(value):
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdefABCDEF" for character in value)
    )


def validate_case(case):
    reasons = set()
    model = case["model"]
    if not valid_text(case["case_id"]):
        reasons.add("CASE_ID_EMPTY")

    numeric_names = [
        "relative_translation_speed_um_s",
        "relative_angular_speed_urad_s",
        "max_point_radius_um",
        "min_depth_um",
        "max_abs_x_um",
        "max_abs_y_um",
        "focal_x_micro_px",
        "focal_y_micro_px",
        "row_exposure_duration_ns",
        "first_to_last_row_midpoint_span_ns",
        "exposure_budget_x_micro_px",
        "exposure_budget_y_micro_px",
        "row_time_budget_x_micro_px",
        "row_time_budget_y_micro_px",
        "combined_frame_budget_x_micro_px",
        "combined_frame_budget_y_micro_px",
    ]
    if any(model[name] > MAX_INPUT for name in numeric_names):
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
    for name, key in [
        ("MOTION_RECEIPT", "motion_receipt_sha256"),
        ("SUPPORT_RECEIPT", "support_receipt_sha256"),
        ("TIMING_RECEIPT", "timing_receipt_sha256"),
    ]:
        if not valid_sha256(model[key]):
            reasons.add(name + "_DIGEST_INVALID")
    return sorted(reasons)


def result(case, status, bounds=None, reasons=None):
    model = case["model"]
    return {
        "case_id": case["case_id"],
        "status": status,
        "bounds": bounds,
        "reason_codes": sorted(reasons or []),
        "motion_receipt_sha256": model["motion_receipt_sha256"],
        "support_receipt_sha256": model["support_receipt_sha256"],
        "timing_receipt_sha256": model["timing_receipt_sha256"],
    }


def projected_axis_upper(focal_micro_px, displacement_um, extent_um, min_depth_um):
    z = min_depth_um
    numerator = focal_micro_px * displacement_um * (z + extent_um)
    denominator = z * z
    return ceil_fraction(Fraction(numerator, denominator))


def evaluate_case(case):
    model = case["model"]
    invalid = validate_case(case)
    if invalid:
        return result(case, "INVALID_INPUT", reasons=invalid)

    gates = [
        ("RELATIVE_MOTION", model["relative_motion_state"]),
        ("SUPPORT_ENVELOPE", model["support_envelope_state"]),
        ("TIMING", model["timing_state"]),
    ]
    failed = {
        "GATE_FAIL:" + name for name, state in gates if state == "FAIL"
    }
    unknown = {
        "GATE_UNKNOWN:" + name for name, state in gates if state == "UNKNOWN"
    }
    if failed:
        return result(case, "BLOCKED", reasons=failed)
    if unknown:
        return result(case, "INDETERMINATE", reasons=unknown)

    rotational_speed = ceil_fraction(
        Fraction(
            model["relative_angular_speed_urad_s"]
            * model["max_point_radius_um"],
            1_000_000,
        )
    )
    total_speed = model["relative_translation_speed_um_s"] + rotational_speed
    exposure_delta = ceil_fraction(
        Fraction(total_speed * model["row_exposure_duration_ns"], 1_000_000_000)
    )
    row_delta = ceil_fraction(
        Fraction(
            total_speed * model["first_to_last_row_midpoint_span_ns"],
            1_000_000_000,
        )
    )

    exposure_x = projected_axis_upper(
        model["focal_x_micro_px"],
        exposure_delta,
        model["max_abs_x_um"],
        model["min_depth_um"],
    )
    exposure_y = projected_axis_upper(
        model["focal_y_micro_px"],
        exposure_delta,
        model["max_abs_y_um"],
        model["min_depth_um"],
    )
    row_x = projected_axis_upper(
        model["focal_x_micro_px"],
        row_delta,
        model["max_abs_x_um"],
        model["min_depth_um"],
    )
    row_y = projected_axis_upper(
        model["focal_y_micro_px"],
        row_delta,
        model["max_abs_y_um"],
        model["min_depth_um"],
    )
    values = [
        rotational_speed,
        total_speed,
        exposure_delta,
        row_delta,
        exposure_x,
        exposure_y,
        row_x,
        row_y,
        exposure_x + row_x,
        exposure_y + row_y,
    ]
    if any(value > MAX_U64 for value in values):
        return result(case, "INVALID_INPUT", reasons=["ARITHMETIC_OVERFLOW"])

    bounds = {
        "rotational_speed_um_s_upper": rotational_speed,
        "total_relative_point_speed_um_s_upper": total_speed,
        "exposure_point_displacement_um_upper": exposure_delta,
        "row_time_point_displacement_um_upper": row_delta,
        "exposure_x_micro_px_upper": exposure_x,
        "exposure_y_micro_px_upper": exposure_y,
        "row_time_x_micro_px_upper": row_x,
        "row_time_y_micro_px_upper": row_y,
        "combined_frame_x_micro_px_upper": exposure_x + row_x,
        "combined_frame_y_micro_px_upper": exposure_y + row_y,
    }

    reasons = set()
    checks = [
        (exposure_x, model["exposure_budget_x_micro_px"], "EXPOSURE_X_BUDGET_EXCEEDED"),
        (exposure_y, model["exposure_budget_y_micro_px"], "EXPOSURE_Y_BUDGET_EXCEEDED"),
        (row_x, model["row_time_budget_x_micro_px"], "ROW_TIME_X_BUDGET_EXCEEDED"),
        (row_y, model["row_time_budget_y_micro_px"], "ROW_TIME_Y_BUDGET_EXCEEDED"),
        (exposure_x + row_x, model["combined_frame_budget_x_micro_px"], "COMBINED_FRAME_X_BUDGET_EXCEEDED"),
        (exposure_y + row_y, model["combined_frame_budget_y_micro_px"], "COMBINED_FRAME_Y_BUDGET_EXCEEDED"),
    ]
    for measured, budget, reason in checks:
        if measured > budget:
            reasons.add(reason)

    status = "BUDGET_EXCEEDED" if reasons else "WITHIN_DECLARED_BUDGET"
    return result(case, status, bounds, reasons)


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
        outcome = evaluate_case(case)
        if outcome["status"] == case["expected_status"]:
            expected_matches += 1
        results.append(outcome)

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
