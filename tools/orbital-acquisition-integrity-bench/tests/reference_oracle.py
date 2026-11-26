#!/usr/bin/env python3
"""Independent standard-library oracle for orbital acquisition-integrity v0.1."""

import hashlib
import json
import sys
from pathlib import Path

CORPUS_SCHEMA = "luminous.orbital-acquisition-integrity.corpus.v0.1"
RECEIPT_SCHEMA = "luminous.orbital-acquisition-integrity.receipt.v0.1"
BENCHMARK_ID = "ORBITAL-ACQUISITION-INTEGRITY-001"
CLAIM_CEILING = "synthetic_constant_relative_image_plane_motion_exposure_and_readout_bounds_only"

PRIORITY = {
    "CALIBRATION_NOT_VALID": 0,
    "PROJECTOR_PATTERN_MISMATCH": 1,
    "MOTION_MODEL_OUT_OF_DOMAIN": 2,
    "TIMING_UNCERTAINTY_EXCEEDED": 3,
    "MOTION_BLUR_BUDGET_EXCEEDED": 4,
    "ROLLING_SHUTTER_BUDGET_EXCEEDED": 5,
    "COMBINED_ACQUISITION_BUDGET_EXCEEDED": 6,
    "INDETERMINATE": 7,
    "ACQUISITION_VALID_WITHIN_BUDGET": 8,
}

I64_MIN = -(2**63)
I64_MAX = 2**63 - 1
U64_MAX = 2**64 - 1
U128_MAX = 2**128 - 1


def valid_text(value):
    return isinstance(value, str) and bool(value.strip())


def valid_sha256(value):
    return (isinstance(value, str) and len(value) == 64
            and all(char in "0123456789abcdefABCDEF" for char in value))


def checked_integer(value, minimum, maximum, code):
    if not isinstance(value, int) or isinstance(value, bool) or value < minimum or value > maximum:
        raise ValueError(code)
    return value


def choose_status(statuses):
    return min(statuses, key=lambda item: PRIORITY[item]) if statuses else "ACQUISITION_VALID_WITHIN_BUDGET"


def ceil_div(numerator, denominator):
    if denominator == 0:
        raise ValueError("ZERO_DENOMINATOR")
    return numerator // denominator + (1 if numerator % denominator else 0)


def pixel_bound_micro(l1_velocity, duration_ns, gsd):
    numerator = l1_velocity * duration_ns
    denominator = gsd * 1000
    if numerator > U128_MAX:
        raise ValueError("MOTION_NUMERATOR_OVERFLOW")
    if denominator > U128_MAX:
        raise ValueError("MOTION_DENOMINATOR_OVERFLOW")
    value = ceil_div(numerator, denominator)
    if value > U64_MAX:
        raise ValueError("MOTION_BOUND_EXCEEDS_U64")
    return value


def validate_case(case):
    required_texts = ("case_id", "camera_id", "motion_model_id")
    if any(not valid_text(case[name]) for name in required_texts):
        raise ValueError("ACQUISITION_INPUT_IDENTITY_OR_LIMIT_INVALID")
    if (not valid_sha256(case["calibration_receipt_sha256"])
            or not valid_sha256(case["clock_map_receipt_sha256"])
            or case["gsd_um_per_pixel"] == 0
            or case["exposure_duration_ns"] == 0
            or case["image_height_px"] == 0
            or case["motion_budget_micro_px"] == 0):
        raise ValueError("ACQUISITION_INPUT_IDENTITY_OR_LIMIT_INVALID")

    for key in ("frame_timestamp_ns", "relative_velocity_x_um_per_s", "relative_velocity_y_um_per_s",
                "stable_start_ns", "stable_end_ns"):
        if key in case:
            checked_integer(case[key], I64_MIN, I64_MAX, "SIGNED_FIELD_OUT_OF_RANGE")
    for key in ("timestamp_uncertainty_ns", "trigger_jitter_ns", "gsd_um_per_pixel",
                "exposure_duration_ns", "image_height_px", "readout_duration_ns",
                "readout_uncertainty_ns", "motion_budget_micro_px"):
        checked_integer(case[key], 0 if key not in ("gsd_um_per_pixel", "exposure_duration_ns",
                                                     "image_height_px", "motion_budget_micro_px") else 1,
                        U64_MAX, "UNSIGNED_FIELD_OUT_OF_RANGE")

    if case["image_height_px"] > 2**32 - 1:
        raise ValueError("IMAGE_HEIGHT_OUT_OF_RANGE")
    if case["exposure_duration_ns"] == U64_MAX:
        raise ValueError("EXPOSURE_DURATION_OVERFLOW")

    if case["shutter_mode"] == "GLOBAL_SHUTTER":
        if case["readout_duration_ns"] != 0 or case["readout_uncertainty_ns"] != 0:
            raise ValueError("GLOBAL_SHUTTER_HAS_ROLLING_READOUT")
    elif case["shutter_mode"] == "ROLLING_SHUTTER":
        if case["image_height_px"] < 2 or case["readout_duration_ns"] == 0:
            raise ValueError("ROLLING_SHUTTER_READOUT_INVALID")
    else:
        raise ValueError("SHUTTER_MODE_INVALID")

    if not case["requires_projector"] and case["projector_window"] is not None:
        raise ValueError("UNEXPECTED_PROJECTOR_WINDOW")
    window = case["projector_window"]
    if window is not None:
        checked_integer(window["stable_start_ns"], I64_MIN, I64_MAX, "SIGNED_FIELD_OUT_OF_RANGE")
        checked_integer(window["stable_end_ns"], I64_MIN, I64_MAX, "SIGNED_FIELD_OUT_OF_RANGE")
        if (not valid_text(window["pattern_id"])
                or not valid_sha256(window["pattern_receipt_sha256"])
                or window["stable_end_ns"] <= window["stable_start_ns"]):
            raise ValueError("PROJECTOR_WINDOW_INVALID")


def evaluate_case(case):
    validate_case(case)
    statuses = []
    reasons = set()

    def add(status, reason):
        statuses.append(status)
        reasons.add(reason)

    if case["calibration_state"] == "INVALID":
        add("CALIBRATION_NOT_VALID", "CALIBRATION_CHAIN_NOT_VALID")
    elif case["calibration_state"] == "UNKNOWN":
        add("INDETERMINATE", "CALIBRATION_CHAIN_STATE_UNKNOWN")

    if case["clock_map_state"] == "INVALID":
        add("TIMING_UNCERTAINTY_EXCEEDED", "CLOCK_MAP_NOT_VALID")
    elif case["clock_map_state"] == "UNKNOWN":
        add("INDETERMINATE", "CLOCK_MAP_STATE_UNKNOWN")

    if case["motion_model_state"] == "OUT_OF_DOMAIN":
        add("MOTION_MODEL_OUT_OF_DOMAIN", "MOTION_MODEL_OUT_OF_DOMAIN")
    elif case["motion_model_state"] == "UNKNOWN":
        add("INDETERMINATE", "MOTION_MODEL_STATE_UNKNOWN")

    if case["timestamp_reference"] != "CENTER_ROW_EXPOSURE_MIDPOINT":
        add("INDETERMINATE", "TIMESTAMP_REFERENCE_UNSUPPORTED")

    l1_velocity = abs(case["relative_velocity_x_um_per_s"]) + abs(case["relative_velocity_y_um_per_s"])
    if l1_velocity > U128_MAX:
        raise ValueError("RELATIVE_VELOCITY_OVERFLOW")

    exposure_smear = pixel_bound_micro(l1_velocity, case["exposure_duration_ns"], case["gsd_um_per_pixel"])

    if case["shutter_mode"] == "GLOBAL_SHUTTER":
        maximum_row_offset = 0
    else:
        full_readout = case["readout_duration_ns"] + case["readout_uncertainty_ns"]
        if full_readout > U64_MAX:
            raise ValueError("READOUT_DURATION_OVERFLOW")
        maximum_row_offset = (full_readout + 1) // 2

    rolling_bound = pixel_bound_micro(l1_velocity, maximum_row_offset, case["gsd_um_per_pixel"])

    association_time = case["timestamp_uncertainty_ns"] + case["trigger_jitter_ns"]
    if association_time > U64_MAX:
        raise ValueError("TIMING_ASSOCIATION_OVERFLOW")
    timing_bound = pixel_bound_micro(l1_velocity, association_time, case["gsd_um_per_pixel"])

    total_bound = exposure_smear + rolling_bound + timing_bound
    if total_bound > U64_MAX:
        raise ValueError("TOTAL_MOTION_BOUND_OVERFLOW")

    budget = case["motion_budget_micro_px"]
    if exposure_smear > budget:
        add("MOTION_BLUR_BUDGET_EXCEEDED", "EXPOSURE_SMEAR_BUDGET_EXCEEDED")
    if rolling_bound > budget:
        add("ROLLING_SHUTTER_BUDGET_EXCEEDED", "ROLLING_SHUTTER_BUDGET_EXCEEDED")
    if timing_bound > budget:
        add("TIMING_UNCERTAINTY_EXCEEDED", "TIMING_ASSOCIATION_BUDGET_EXCEEDED")
    if total_bound > budget:
        add("COMBINED_ACQUISITION_BUDGET_EXCEEDED", "COMBINED_ACQUISITION_BUDGET_EXCEEDED")

    half_exposure = (case["exposure_duration_ns"] + 1) // 2
    projector_interval = None
    projector_assessed = False
    if case["requires_projector"]:
        if case["timestamp_reference"] != "CENTER_ROW_EXPOSURE_MIDPOINT":
            add("INDETERMINATE", "PROJECTOR_TIME_REFERENCE_UNRESOLVED")
        elif case["clock_map_state"] != "VALID":
            add("INDETERMINATE", "PROJECTOR_CLOCK_MAP_NOT_QUALIFIED")
        else:
            span = (half_exposure + maximum_row_offset
                    + case["timestamp_uncertainty_ns"] + case["trigger_jitter_ns"])
            if span > U128_MAX:
                raise ValueError("PROJECTOR_SPAN_OVERFLOW")
            center = case["frame_timestamp_ns"]
            projector_interval = [center - span, center + span]
            if case["projector_window"] is None:
                add("INDETERMINATE", "PROJECTOR_PATTERN_EVIDENCE_MISSING")
            else:
                projector_assessed = True
                stable_start = case["projector_window"]["stable_start_ns"]
                stable_end = case["projector_window"]["stable_end_ns"]
                if projector_interval[1] < stable_start or projector_interval[0] >= stable_end:
                    add("PROJECTOR_PATTERN_MISMATCH", "PROJECTOR_PATTERN_WINDOW_DISJOINT")
                elif projector_interval[0] < stable_start or projector_interval[1] >= stable_end:
                    add("INDETERMINATE", "PROJECTOR_PATTERN_WINDOW_BOUNDARY_OVERLAP")

    status = choose_status(statuses)
    return {
        "case_id": case["case_id"],
        "camera_id": case["camera_id"],
        "motion_model_id": case["motion_model_id"],
        "motion_model_state": case["motion_model_state"],
        "calibration_state": case["calibration_state"],
        "calibration_receipt_sha256": case["calibration_receipt_sha256"],
        "clock_map_state": case["clock_map_state"],
        "clock_map_receipt_sha256": case["clock_map_receipt_sha256"],
        "timestamp_reference": case["timestamp_reference"],
        "frame_timestamp_ns": case["frame_timestamp_ns"],
        "timestamp_uncertainty_ns": case["timestamp_uncertainty_ns"],
        "trigger_jitter_ns": case["trigger_jitter_ns"],
        "relative_velocity_x_um_per_s": case["relative_velocity_x_um_per_s"],
        "relative_velocity_y_um_per_s": case["relative_velocity_y_um_per_s"],
        "gsd_um_per_pixel": case["gsd_um_per_pixel"],
        "exposure_duration_ns": case["exposure_duration_ns"],
        "shutter_mode": case["shutter_mode"],
        "image_height_px": case["image_height_px"],
        "readout_duration_ns": case["readout_duration_ns"],
        "readout_uncertainty_ns": case["readout_uncertainty_ns"],
        "readout_direction": case["readout_direction"],
        "requires_projector": case["requires_projector"],
        "projector_window": case["projector_window"],
        "exposure_smear_bound_micro_px": exposure_smear,
        "rolling_shutter_bound_micro_px": rolling_bound,
        "timing_association_bound_micro_px": timing_bound,
        "total_motion_bound_micro_px": total_bound,
        "motion_budget_micro_px": budget,
        "maximum_row_offset_ns": maximum_row_offset,
        "projector_frame_interval_ns": projector_interval,
        "projector_pattern_assessed": projector_assessed,
        "status": status,
        "reason_codes": sorted(reasons),
        "may_enter_measurement_pipeline": status == "ACQUISITION_VALID_WITHIN_BUDGET",
        "visibility_assessed": False,
        "photometric_usability_assessed": False,
        "feature_measurability_assessed": False,
    }


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: reference_oracle.py <fixture.json> <40-hex-source-commit>")
    raw = Path(sys.argv[1]).read_bytes()
    source_commit = sys.argv[2]
    if (len(source_commit) != 40
            or any(char not in "0123456789abcdefABCDEF" for char in source_commit)):
        raise SystemExit("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS")
    corpus = json.loads(raw)
    if (corpus.get("schema") != CORPUS_SCHEMA
            or corpus.get("benchmark_id") != BENCHMARK_ID
            or corpus.get("evidence_class") != "synthetic"
            or corpus.get("claim_ceiling") != CLAIM_CEILING
            or not 10 <= len(corpus.get("cases", [])) <= 256):
        raise SystemExit("CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID")

    case_ids = [case["case_id"] for case in corpus["cases"]]
    if len(case_ids) != len(set(case_ids)):
        raise SystemExit("CASE_ID_DUPLICATE")
    results = [evaluate_case(case) for case in corpus["cases"]]
    matches = sum(result["status"] == case["expected_status"]
                  for result, case in zip(results, corpus["cases"]))
    receipt = {
        "schema": RECEIPT_SCHEMA,
        "benchmark_id": BENCHMARK_ID,
        "source_commit": source_commit.lower(),
        "input_sha256": hashlib.sha256(raw).hexdigest(),
        "evidence_class": "synthetic",
        "claim_ceiling": CLAIM_CEILING,
        "case_count": len(results),
        "expected_outcome_matches": matches,
        "expected_outcome_mismatches": len(results) - matches,
        "authority_granted": False,
        "physical_validation": "NOT_ASSESSED",
        "cases": results,
    }
    print(json.dumps(receipt, indent=2) + "\n")


if __name__ == "__main__":
    main()
