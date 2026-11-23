#!/usr/bin/env python3
"""Independent standard-library support-envelope oracle, corpus v0.1."""

import hashlib
import json
import math
import sys
from pathlib import Path

CORPUS_SCHEMA = "luminous.orbital-truss-support-envelope.corpus.v0.1"
RECEIPT_SCHEMA = "luminous.orbital-truss-support-envelope.receipt.v0.1"
BENCHMARK_ID = "ORBITAL-TRUSS-SUPPORT-ENVELOPE-001"
CLAIM_CEILING = "synthetic_conditional_3d_support_sensing_envelope_bounds_only_occlusion_not_assessed"
CASE_COUNT = 13
DISTANCE_SCALE = 1e9
ANGLE_SCALE = 1e9
GSD_SCALE = 1e12
EPS = 1e-12
UNIT_EPS = 1e-6


def norm(vector):
    return math.sqrt(vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2])


def dot(left, right):
    return left[0] * right[0] + left[1] * right[1] + left[2] * right[2]


def fixed(value, scale):
    if value is None or not math.isfinite(value) or value < 0.0:
        return None
    result = math.floor(value * scale + 0.5)
    return result if result <= 2**64 - 1 else None


def invalid(case, reason):
    support = case["support"]
    return {
        "case_id": case["case_id"],
        "support_id": support["support_id"],
        "status": "INVALID_INPUT",
        "reason_codes": [reason],
        "center_distance_nm": None,
        "support_direction_spread_nrad": None,
        "minimum_distance_lower_bound_nm": None,
        "maximum_distance_upper_bound_nm": None,
        "fov_angle_lower_bound_nrad": None,
        "fov_angle_upper_bound_nrad": None,
        "incidence_angle_lower_bound_nrad": None,
        "incidence_angle_upper_bound_nrad": None,
        "gsd_lower_bound_pm_per_px": None,
        "gsd_upper_bound_pm_per_px": None,
        "support_bound_input_asserted": support["enclosing_ball_asserted"],
        "normal_cone_input_asserted": support["normal_cone_asserted"],
        "precondition_proofs_verified_independently": False,
        "occlusion_status": "NOT_ASSESSED",
        "full_support_visibility_certified": False,
        "may_count_toward_full_support_coverage": False,
    }


def indeterminate(case, reasons):
    support = case["support"]
    return {
        "case_id": case["case_id"],
        "support_id": support["support_id"],
        "status": "INDETERMINATE",
        "reason_codes": sorted(set(reasons)),
        "center_distance_nm": None,
        "support_direction_spread_nrad": None,
        "minimum_distance_lower_bound_nm": None,
        "maximum_distance_upper_bound_nm": None,
        "fov_angle_lower_bound_nrad": None,
        "fov_angle_upper_bound_nrad": None,
        "incidence_angle_lower_bound_nrad": None,
        "incidence_angle_upper_bound_nrad": None,
        "gsd_lower_bound_pm_per_px": None,
        "gsd_upper_bound_pm_per_px": None,
        "support_bound_input_asserted": support["enclosing_ball_asserted"],
        "normal_cone_input_asserted": support["normal_cone_asserted"],
        "precondition_proofs_verified_independently": False,
        "occlusion_status": "NOT_ASSESSED",
        "full_support_visibility_certified": False,
        "may_count_toward_full_support_coverage": False,
    }


def invalid_reason(case):
    support = case["support"]
    sensor = case["sensor"]
    if not case["case_id"].strip() or not case["coordinate_frame_id"].strip() or case["length_unit"] != "METRE":
        return "CASE_FRAME_OR_LENGTH_UNIT_INVALID"
    if (not support["support_id"].strip()
            or not all(math.isfinite(x) for x in support["center_m"])
            or not math.isfinite(support["enclosing_ball_radius_m"])
            or support["enclosing_ball_radius_m"] <= 0):
        return "SUPPORT_ID_GEOMETRY_OR_RADIUS_INVALID"
    normal_length = norm(support["outward_normal_unit"])
    if (not all(math.isfinite(x) for x in support["outward_normal_unit"])
            or not math.isfinite(normal_length)
            or abs(normal_length - 1.0) > UNIT_EPS):
        return "SUPPORT_NORMAL_NOT_UNIT"
    cone = support["normal_cone_half_angle_rad"]
    if not math.isfinite(cone) or cone < 0.0 or cone >= math.pi:
        return "SUPPORT_NORMAL_CONE_INVALID"

    axis_length = norm(sensor["optical_axis_unit"])
    if (not sensor["sensor_id"].strip() or not sensor["camera_model_id"].strip()
            or not all(math.isfinite(x) for x in sensor["position_m"])):
        return "SENSOR_ID_OR_POSITION_INVALID"
    if (not all(math.isfinite(x) for x in sensor["optical_axis_unit"])
            or not math.isfinite(axis_length)
            or abs(axis_length - 1.0) > UNIT_EPS):
        return "SENSOR_OPTICAL_AXIS_NOT_UNIT"
    fov = sensor["horizontal_fov_rad"]
    if not math.isfinite(fov) or fov <= 0.0 or fov >= math.pi or sensor["image_width_px"] == 0:
        return "CAMERA_FOV_OR_PIXEL_WIDTH_INVALID"
    minimum = sensor["minimum_range_m"]
    maximum = sensor["maximum_range_m"]
    if (not math.isfinite(minimum) or not math.isfinite(maximum)
            or minimum < 0.0 or maximum <= minimum):
        return "SENSOR_RANGE_LIMITS_INVALID"
    obliquity = sensor["maximum_obliquity_rad"]
    if not math.isfinite(obliquity) or obliquity < 0.0 or obliquity >= math.pi / 2.0:
        return "MAXIMUM_OBLIQUITY_INVALID"
    gsd = sensor["maximum_gsd_m_per_px"]
    if not math.isfinite(gsd) or gsd <= 0.0:
        return "MAXIMUM_GSD_INVALID"
    return None


def evaluate(case):
    reason = invalid_reason(case)
    if reason:
        return invalid(case, reason)
    support = case["support"]
    sensor = case["sensor"]
    if not support["enclosing_ball_asserted"]:
        return indeterminate(case, ["SUPPORT_ENCLOSING_BALL_UNPROVEN"])

    vector = [
        support["center_m"][i] - sensor["position_m"][i]
        for i in range(3)
    ]
    center_distance = norm(vector)
    radius = support["enclosing_ball_radius_m"]
    if not math.isfinite(center_distance) or center_distance <= radius:
        return indeterminate(case, ["SENSOR_NOT_OUTSIDE_SUPPORT_BOUND"])

    spread = math.asin(min(1.0, max(0.0, radius / center_distance)))
    distance_lower = center_distance - radius
    distance_upper = center_distance + radius
    toward_support = [value / center_distance for value in vector]
    toward_sensor = [-value for value in toward_support]
    fov_center = math.acos(max(-1.0, min(1.0, dot(sensor["optical_axis_unit"], toward_support))))
    fov_lower = max(0.0, fov_center - spread)
    fov_upper = fov_center + spread

    incidence_lower = None
    incidence_upper = None
    gsd_lower = None
    gsd_upper = None
    if support["normal_cone_asserted"]:
        center_incidence = math.acos(max(
            -1.0,
            min(1.0, dot(support["outward_normal_unit"], toward_sensor)),
        ))
        normal_spread = support["normal_cone_half_angle_rad"]
        incidence_lower = max(0.0, center_incidence - spread - normal_spread)
        incidence_upper = center_incidence + spread + normal_spread
        gsd_factor = (
            2.0 * math.tan(sensor["horizontal_fov_rad"] / 2.0)
            / sensor["image_width_px"]
        )
        if incidence_lower < math.pi / 2.0:
            gsd_lower = gsd_factor * distance_lower / math.cos(incidence_lower)
        if incidence_upper < math.pi / 2.0:
            gsd_upper = gsd_factor * distance_upper / math.cos(incidence_upper)

    outside = []
    if distance_upper < sensor["minimum_range_m"] - EPS:
        outside.append("ALL_SUPPORT_BELOW_MINIMUM_RANGE")
    if distance_lower > sensor["maximum_range_m"] + EPS:
        outside.append("ALL_SUPPORT_ABOVE_MAXIMUM_RANGE")
    if fov_lower > sensor["horizontal_fov_rad"] / 2.0 + EPS:
        outside.append("ALL_SUPPORT_OUTSIDE_FIELD_OF_VIEW")
    if incidence_lower is not None and incidence_lower > sensor["maximum_obliquity_rad"] + EPS:
        outside.append("ALL_SUPPORT_EXCEEDS_MAXIMUM_OBLIQUITY")
    if gsd_lower is not None and gsd_lower > sensor["maximum_gsd_m_per_px"] + EPS:
        outside.append("ALL_SUPPORT_EXCEEDS_MAXIMUM_GSD")

    if outside:
        status = "CONDITIONAL_OUTSIDE_SENSING_ENVELOPE"
        reasons = sorted(outside)
    else:
        distance_pass = (
            distance_lower >= sensor["minimum_range_m"] - EPS
            and distance_upper <= sensor["maximum_range_m"] + EPS
        )
        fov_pass = fov_upper <= sensor["horizontal_fov_rad"] / 2.0 + EPS
        incidence_pass = (
            support["normal_cone_asserted"]
            and incidence_upper is not None
            and incidence_upper <= sensor["maximum_obliquity_rad"] + EPS
            and incidence_upper < math.pi / 2.0
        )
        gsd_pass = (
            gsd_upper is not None
            and gsd_upper <= sensor["maximum_gsd_m_per_px"] + EPS
        )
        if distance_pass and fov_pass and incidence_pass and gsd_pass:
            status = "CONDITIONAL_WITHIN_SENSING_ENVELOPE"
            reasons = [
                "WHOLE_SUPPORT_RANGE_BOUNDS_PASS",
                "WHOLE_SUPPORT_FOV_BOUND_PASSES",
                "WHOLE_SUPPORT_INCIDENCE_BOUND_PASSES",
                "WHOLE_SUPPORT_GSD_BOUND_PASSES",
                "OCCLUSION_NOT_ASSESSED",
            ]
        else:
            status = "INDETERMINATE"
            reasons = []
            if not distance_pass:
                reasons.append("RANGE_UPPER_BOUND_NOT_CERTIFIED")
            if fov_upper > sensor["horizontal_fov_rad"] / 2.0 + EPS:
                reasons.append("FOV_UPPER_BOUND_NOT_CERTIFIED")
            if not support["normal_cone_asserted"]:
                reasons.append("NORMAL_CONE_UNPROVEN")
            elif (incidence_upper is None
                  or incidence_upper > sensor["maximum_obliquity_rad"] + EPS
                  or incidence_upper >= math.pi / 2.0):
                reasons.append("INCIDENCE_UPPER_BOUND_NOT_CERTIFIED")
            if gsd_upper is None or gsd_upper > sensor["maximum_gsd_m_per_px"] + EPS:
                reasons.append("GSD_UPPER_BOUND_NOT_CERTIFIED")
            reasons = sorted(set(reasons))

    return {
        "case_id": case["case_id"],
        "support_id": support["support_id"],
        "status": status,
        "reason_codes": reasons,
        "center_distance_nm": fixed(center_distance, DISTANCE_SCALE),
        "support_direction_spread_nrad": fixed(spread, ANGLE_SCALE),
        "minimum_distance_lower_bound_nm": fixed(distance_lower, DISTANCE_SCALE),
        "maximum_distance_upper_bound_nm": fixed(distance_upper, DISTANCE_SCALE),
        "fov_angle_lower_bound_nrad": fixed(fov_lower, ANGLE_SCALE),
        "fov_angle_upper_bound_nrad": fixed(fov_upper, ANGLE_SCALE),
        "incidence_angle_lower_bound_nrad": fixed(incidence_lower, ANGLE_SCALE),
        "incidence_angle_upper_bound_nrad": fixed(incidence_upper, ANGLE_SCALE),
        "gsd_lower_bound_pm_per_px": fixed(gsd_lower, GSD_SCALE),
        "gsd_upper_bound_pm_per_px": fixed(gsd_upper, GSD_SCALE),
        "support_bound_input_asserted": support["enclosing_ball_asserted"],
        "normal_cone_input_asserted": support["normal_cone_asserted"],
        "precondition_proofs_verified_independently": False,
        "occlusion_status": "NOT_ASSESSED",
        "full_support_visibility_certified": False,
        "may_count_toward_full_support_coverage": False,
    }


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: support_envelope_reference_oracle.py <support-corpus.json> <40-hex-source-commit>")
    raw = Path(sys.argv[1]).read_bytes()
    source_commit = sys.argv[2]
    if len(source_commit) != 40 or any(char not in "0123456789abcdefABCDEF" for char in source_commit):
        raise SystemExit("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS")
    corpus = json.loads(raw)
    if (corpus.get("schema") != CORPUS_SCHEMA
            or corpus.get("benchmark_id") != BENCHMARK_ID
            or corpus.get("evidence_class") != "synthetic"
            or corpus.get("claim_ceiling") != CLAIM_CEILING
            or len(corpus.get("cases", [])) != CASE_COUNT):
        raise SystemExit("SUPPORT_CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID")
    ids = [case["case_id"] for case in corpus["cases"]]
    if len(ids) != len(set(ids)):
        raise SystemExit("DUPLICATE_SUPPORT_CASE_ID")

    cases = [evaluate(case) for case in corpus["cases"]]
    matches = sum(
        actual["status"] == expected["expected_status"]
        for actual, expected in zip(cases, corpus["cases"])
    )
    output = {
        "schema": RECEIPT_SCHEMA,
        "benchmark_id": BENCHMARK_ID,
        "source_commit": source_commit.lower(),
        "input_sha256": hashlib.sha256(raw).hexdigest(),
        "evidence_class": "synthetic",
        "claim_ceiling": CLAIM_CEILING,
        "authority_granted": False,
        "structural_acceptance_authorized": False,
        "flight_authorized": False,
        "physical_validation": "NOT_ASSESSED",
        "occlusion_assessed": False,
        "precondition_proofs_verified_independently": False,
        "case_count": len(cases),
        "expected_outcome_matches": matches,
        "expected_outcome_mismatches": len(cases) - matches,
        "cases": cases,
    }
    print(json.dumps(output, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
