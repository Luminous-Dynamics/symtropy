#!/usr/bin/env python3
"""Independent standard-library oracle for synthetic 2D external datum registration."""
import hashlib
import json
import math
import sys

SCHEMA = "luminous.orbital-truss-independent-datum.corpus.v0.1"
CLAIM = "synthetic_2d_external_datum_registration_only"


def dist(a, b):
    return math.hypot(a[0] - b[0], a[1] - b[1])


def sorted_points(points):
    return sorted(points, key=lambda point: point["point_id"])


def span(points):
    return max((dist(a["source_position_m"], b["source_position_m"])
                for i, a in enumerate(points) for b in points[i + 1:]), default=0.0)


def max_triangle_area(points):
    maximum = 0.0
    for i, a in enumerate(points):
        for j in range(i + 1, len(points)):
            b = points[j]["source_position_m"]
            for k in range(j + 1, len(points)):
                c = points[k]["source_position_m"]
                p = a["source_position_m"]
                twice = ((b[0] - p[0]) * (c[1] - p[1])
                         - (b[1] - p[1]) * (c[0] - p[0]))
                maximum = max(maximum, abs(twice) * 0.5)
    return maximum


def fit(points):
    count = len(points)
    source_mean = [sum(p["source_position_m"][axis] / count for p in points)
                   for axis in range(2)]
    target_mean = [sum(p["target_position_m"][axis] / count for p in points)
                   for axis in range(2)]
    dot = 0.0
    cross = 0.0
    for point in points:
        sx = point["source_position_m"][0] - source_mean[0]
        sy = point["source_position_m"][1] - source_mean[1]
        tx = point["target_position_m"][0] - target_mean[0]
        ty = point["target_position_m"][1] - target_mean[1]
        dot += sx * tx + sy * ty
        cross += sx * ty - sy * tx
    if not math.isfinite(dot) or not math.isfinite(cross) or math.hypot(dot, cross) <= 1e-12:
        return None
    angle = math.atan2(cross, dot)
    cosine, sine = math.cos(angle), math.sin(angle)
    rotated = [cosine * source_mean[0] - sine * source_mean[1],
               sine * source_mean[0] + cosine * source_mean[1]]
    translation = [target_mean[0] - rotated[0], target_mean[1] - rotated[1]]
    return angle, cosine, sine, translation


def evaluate(case):
    reasons = []
    if case["datum_origin"] != "independent_external_reference_network":
        reasons.append("DATUM_ORIGIN_NOT_INDEPENDENT_EXTERNAL_NETWORK")
    if case["article_geometry_used"]:
        reasons.append("ARTICLE_GEOMETRY_USED_FOR_REGISTRATION")
    if (not case["source_frame_id"].strip() or not case["target_frame_id"].strip()
            or not case["datum_set_id"].strip()
            or case["source_frame_id"] == case["target_frame_id"]):
        reasons.append("FRAME_OR_DATUM_IDENTITY_INVALID")
    if len(case["controls"]) < 3:
        reasons.append("INSUFFICIENT_CONTROL_POINTS")
    if not case["check_points"]:
        reasons.append("NO_INDEPENDENT_CHECK_POINTS")
    ids = [point["point_id"] for point in case["controls"] + case["check_points"]]
    if len(ids) != len(set(ids)) or any(not point_id.strip() for point_id in ids):
        reasons.append("POINT_IDENTITY_OR_COORDINATE_EVIDENCE_INVALID")

    controls = sorted_points(case["controls"])
    checks = sorted_points(case["check_points"])

    def unresolved():
        return {
            "case_id": case["case_id"], "disposition": "INDETERMINATE",
            "reason_codes": sorted(set(reasons)), "rotation_radians": None,
            "translation_m": None, "control_span_m": None,
            "maximum_triangle_area_m2": None, "control_residuals": [],
            "check_residuals": [], "authority_granted": False,
        }

    if len(controls) < 3 or reasons:
        return unresolved()

    control_span = span(controls)
    triangle_area = max_triangle_area(controls)
    if control_span < case["minimum_span_m"]:
        reasons.append("CONTROL_NETWORK_SPAN_BELOW_MINIMUM")
    if triangle_area < case["minimum_triangle_area_m2"]:
        reasons.append("CONTROL_NETWORK_NEAR_DEGENERATE")

    pose = fit(controls)
    if pose is None:
        reasons.append("RIGID_POSE_FIT_UNOBSERVABLE")
        return unresolved()
    angle, cosine, sine, translation = pose

    def compute(points, base):
        values = []
        for point in points:
            source = point["source_position_m"]
            transformed = [
                cosine * source[0] - sine * source[1] + translation[0],
                sine * source[0] + cosine * source[1] + translation[1],
            ]
            residual = dist(transformed, point["target_position_m"])
            allowed = (base + point["source_error_bound_m"]
                       + point["target_error_bound_m"]
                       + case["registration_error_bound_m"])
            values.append({
                "point_id": point["point_id"], "residual_m": residual,
                "allowed_residual_m": allowed, "within_bound": residual <= allowed,
            })
        return values

    control_results = compute(controls, case["maximum_control_residual_m"])
    check_results = compute(checks, case["maximum_check_residual_m"])
    control_bad = any(not item["within_bound"] for item in control_results)
    check_bad = any(not item["within_bound"] for item in check_results)
    if control_bad:
        reasons.append("CONTROL_RESIDUAL_EXCEEDS_DECLARED_HARD_BOUND")
    if check_bad:
        reasons.append("WITHHELD_CHECK_RESIDUAL_EXCEEDS_DECLARED_HARD_BOUND")
    disposition = ("REJECTED" if control_bad or check_bad else
                   "INDETERMINATE" if reasons else "QUALIFIED")
    return {
        "case_id": case["case_id"], "disposition": disposition,
        "reason_codes": sorted(set(reasons)), "rotation_radians": angle,
        "translation_m": translation, "control_span_m": control_span,
        "maximum_triangle_area_m2": triangle_area,
        "control_residuals": control_results, "check_residuals": check_results,
        "authority_granted": False,
    }


def main():
    raw = open(sys.argv[1], "rb").read()
    corpus = json.loads(raw)
    assert corpus["schema"] == SCHEMA
    assert corpus["claim_ceiling"] == CLAIM
    cases = sorted((evaluate(case) for case in corpus["cases"]),
                   key=lambda item: item["case_id"])
    expected = {case["case_id"]: case["expected_disposition"]
                for case in corpus["cases"]}
    matches = sum(item["disposition"] == expected[item["case_id"]] for item in cases)
    receipt = {
        "schema": "luminous.orbital-truss-independent-datum.receipt.v0.1",
        "benchmark_id": corpus["benchmark_id"],
        "input_sha256": hashlib.sha256(raw).hexdigest(),
        "evidence_class": "synthetic",
        "claim_ceiling": CLAIM,
        "authority_granted": False,
        "physical_validation": "NOT_ASSESSED",
        "case_count": len(cases),
        "declared_matches": matches,
        "declared_mismatches": len(cases) - matches,
        "cases": cases,
    }
    print(json.dumps(receipt, sort_keys=True, indent=2))


if __name__ == "__main__":
    main()
