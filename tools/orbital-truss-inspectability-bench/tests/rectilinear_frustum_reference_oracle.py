#!/usr/bin/env python3
"""Independent standard-library oracle for support-vs-rectilinear-frustum corpus v0.1."""

import hashlib
import json
import math
import sys
from pathlib import Path

CORPUS_SCHEMA = "luminous.orbital-truss-rectilinear-frustum.corpus.v0.1"
RECEIPT_SCHEMA = "luminous.orbital-truss-rectilinear-frustum.receipt.v0.1"
BENCHMARK_ID = "ORBITAL-TRUSS-RECTILINEAR-FRUSTUM-001"
CLAIM_CEILING = "synthetic_3d_support_direction_cap_vs_rectilinear_pinhole_frustum_only_occlusion_not_assessed"
CASE_COUNT = 9
UNIT_EPS = 1e-6
EPS = 1e-12
DISTANCE_SCALE = 1e9
ANGLE_SCALE = 1e9


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def norm(v):
    return math.sqrt(dot(v, v))


def subtract(a, b):
    return [x - y for x, y in zip(a, b)]


def scale(v, factor):
    return [x * factor for x in v]


def cross(a, b):
    return [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]


def fixed_nonnegative(value, factor):
    if not math.isfinite(value) or value < 0:
        return None
    rounded = math.floor(value * factor + 0.5)
    return rounded if rounded <= 2**64 - 1 else None


def fixed_signed(value, factor):
    if not math.isfinite(value):
        return None
    rounded = math.floor(abs(value * factor) + 0.5)
    if rounded > 2**63 - 1:
        return None
    return -rounded if value < 0 else rounded


def cap_max_dot(plane, direction, spread):
    magnitude = norm(plane)
    if magnitude <= EPS:
        return None
    theta = math.acos(max(-1.0, min(1.0, dot(plane, direction) / magnitude)))
    if theta <= spread:
        return magnitude
    return magnitude * math.cos(theta - spread)


def cap_min_dot(plane, direction, spread):
    return -cap_max_dot(scale(plane, -1.0), direction, spread)


def invalid(case, reason):
    return {
        "case_id": case["case_id"],
        "support_id": case["support"]["support_id"],
        "camera_id": case["camera"]["camera_id"],
        "status": "INVALID_INPUT",
        "reason_codes": [reason],
        "center_distance_nm": None,
        "support_direction_spread_nrad": None,
        "halfspace_inside_margin_nrad": [None] * 5,
        "halfspace_outside_margin_nrad": [None] * 5,
        "minimum_horizontal_inside_margin_nrad": None,
        "minimum_vertical_inside_margin_nrad": None,
        "forward_hemisphere_inside_margin_nrad": None,
        "support_bound_input_asserted": case["support"]["enclosing_ball_asserted"],
        "precondition_proofs_verified_independently": False,
        "occlusion_status": "NOT_ASSESSED",
        "within_rectilinear_frustum_certified": False,
        "may_count_toward_visibility": False,
    }


def validate(case):
    support = case["support"]
    camera = case["camera"]
    if not case["case_id"].strip() or not case["coordinate_frame_id"].strip():
        return "CASE_ID_OR_COORDINATE_FRAME_INVALID"
    if (not support["support_id"].strip()
            or not all(math.isfinite(x) for x in support["center_m"])
            or not math.isfinite(support["enclosing_ball_radius_m"])
            or support["enclosing_ball_radius_m"] <= 0):
        return "SUPPORT_ID_CENTER_OR_ENCLOSURE_INVALID"
    if (not camera["camera_id"].strip() or not camera["camera_model_id"].strip()
            or not all(math.isfinite(x) for x in camera["position_m"])):
        return "CAMERA_ID_OR_POSITION_INVALID"
    forward = camera["optical_axis_unit"]
    image_up = camera["image_up_unit"]
    if (not all(math.isfinite(x) for x in forward + image_up)
            or abs(norm(forward) - 1) > UNIT_EPS
            or abs(norm(image_up) - 1) > UNIT_EPS):
        return "CAMERA_BASIS_VECTORS_NOT_UNIT"
    if abs(dot(forward, image_up)) > UNIT_EPS:
        return "CAMERA_IMAGE_UP_NOT_ORTHOGONAL_TO_OPTICAL_AXIS"
    hfov = camera["horizontal_fov_rad"]
    vfov = camera["vertical_fov_rad"]
    if (not math.isfinite(hfov) or not math.isfinite(vfov)
            or hfov <= 0 or vfov <= 0 or hfov >= math.pi or vfov >= math.pi):
        return "RECTILINEAR_CAMERA_FOV_INVALID"
    return None


def evaluate(case):
    reason = validate(case)
    if reason:
        return invalid(case, reason)
    support = case["support"]
    camera = case["camera"]
    if not support["enclosing_ball_asserted"]:
        result = invalid(case, "SUPPORT_ENCLOSING_BALL_UNPROVEN")
        result["status"] = "INDETERMINATE"
        return result

    vector = subtract(support["center_m"], camera["position_m"])
    distance = norm(vector)
    radius = support["enclosing_ball_radius_m"]
    if not math.isfinite(distance) or distance <= radius:
        result = invalid(case, "CAMERA_NOT_OUTSIDE_SUPPORT_ENCLOSURE")
        result["status"] = "INDETERMINATE"
        return result
    direction = scale(vector, 1 / distance)
    spread = math.asin(max(0.0, min(1.0, radius / distance)))
    forward = camera["optical_axis_unit"]
    image_up = camera["image_up_unit"]
    right = cross(forward, image_up)
    right = scale(right, 1 / norm(right))
    up = cross(right, forward)
    up = scale(up, 1 / norm(up))
    if dot(up, image_up) < 1 - UNIT_EPS:
        return invalid(case, "CAMERA_BASIS_HANDEDNESS_INCONSISTENT")

    tan_h = math.tan(camera["horizontal_fov_rad"] / 2)
    tan_v = math.tan(camera["vertical_fov_rad"] / 2)
    planes = [
        scale(forward, -1),
        [right[i] - tan_h * forward[i] for i in range(3)],
        [-right[i] - tan_h * forward[i] for i in range(3)],
        [up[i] - tan_v * forward[i] for i in range(3)],
        [-up[i] - tan_v * forward[i] for i in range(3)],
    ]
    inside = []
    outside = []
    all_inside = True
    some_plane_wholly_outside = False
    for plane in planes:
        magnitude = norm(plane)
        theta = math.acos(max(-1.0, min(1.0, dot(plane, direction) / magnitude)))
        inside.append(theta - spread - math.pi / 2)
        outside.append(math.pi / 2 - theta - spread)
        all_inside = all_inside and cap_max_dot(plane, direction, spread) <= EPS
        some_plane_wholly_outside = (
            some_plane_wholly_outside or cap_min_dot(plane, direction, spread) > EPS
        )

    if all_inside:
        status = "WITHIN"
        reasons = ["WHOLE_SUPPORT_RECTILINEAR_FRUSTUM_BOUNDS_PASS", "OCCLUSION_NOT_ASSESSED"]
    elif some_plane_wholly_outside:
        status = "OUTSIDE"
        reasons = ["WHOLE_SUPPORT_OUTSIDE_RECTILINEAR_FRUSTUM"]
    else:
        status = "INDETERMINATE"
        reasons = ["RECTILINEAR_FRUSTUM_BOUNDARY_OR_SUPPORT_STRADDLE"]

    return {
        "case_id": case["case_id"],
        "support_id": support["support_id"],
        "camera_id": camera["camera_id"],
        "status": status,
        "reason_codes": sorted(set(reasons)),
        "center_distance_nm": fixed_nonnegative(distance, DISTANCE_SCALE),
        "support_direction_spread_nrad": fixed_nonnegative(spread, ANGLE_SCALE),
        "halfspace_inside_margin_nrad": [fixed_signed(value, ANGLE_SCALE) for value in inside],
        "halfspace_outside_margin_nrad": [fixed_signed(value, ANGLE_SCALE) for value in outside],
        "minimum_horizontal_inside_margin_nrad": fixed_signed(min(inside[1:3]), ANGLE_SCALE),
        "minimum_vertical_inside_margin_nrad": fixed_signed(min(inside[3:5]), ANGLE_SCALE),
        "forward_hemisphere_inside_margin_nrad": fixed_signed(inside[0], ANGLE_SCALE),
        "support_bound_input_asserted": True,
        "precondition_proofs_verified_independently": False,
        "occlusion_status": "NOT_ASSESSED",
        "within_rectilinear_frustum_certified": status == "WITHIN",
        "may_count_toward_visibility": False,
    }


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: rectilinear_frustum_reference_oracle.py <corpus.json> <40-hex-source-commit>")
    raw = Path(sys.argv[1]).read_bytes()
    source_commit = sys.argv[2]
    if len(source_commit) != 40 or any(c not in "0123456789abcdefABCDEF" for c in source_commit):
        raise SystemExit("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS")
    corpus = json.loads(raw)
    if (corpus.get("schema") != CORPUS_SCHEMA
            or corpus.get("benchmark_id") != BENCHMARK_ID
            or corpus.get("evidence_class") != "synthetic"
            or corpus.get("claim_ceiling") != CLAIM_CEILING
            or len(corpus.get("cases", [])) != CASE_COUNT):
        raise SystemExit("FRUSTUM_CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID")
    identifiers = [case["case_id"] for case in corpus["cases"]]
    if len(identifiers) != len(set(identifiers)):
        raise SystemExit("DUPLICATE_FRUSTUM_CASE_ID")
    cases = [evaluate(case) for case in corpus["cases"]]
    matches = sum(result["status"] == case["expected_status"]
                  for result, case in zip(cases, corpus["cases"]))
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
