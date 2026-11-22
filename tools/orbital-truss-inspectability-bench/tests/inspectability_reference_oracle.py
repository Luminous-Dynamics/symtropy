#!/usr/bin/env python3
"""Independent stdlib oracle for the synthetic 2D inspectability corpus."""
import hashlib
import json
import math
import sys
from pathlib import Path

CORPUS_SCHEMA = "luminous.orbital-truss-inspectability.corpus.v0.1"
RECEIPT_SCHEMA = "luminous.orbital-truss-inspectability.oracle.v0.1"
BENCHMARK_ID = "ORBITAL-TRUSS-INSPECTABILITY-001"
CLAIM_CEILING = "synthetic_2d_geometric_observability_only"
EPS = 1.0e-9
UNIT_EPS = 1.0e-6


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1])


def norm(v):
    return math.hypot(v[0], v[1])


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1]


def cross(a, b, c):
    return (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])


def on_segment(a, b, p):
    return (
        abs(cross(a, b, p)) <= EPS
        and min(a[0], b[0]) - EPS <= p[0] <= max(a[0], b[0]) + EPS
        and min(a[1], b[1]) - EPS <= p[1] <= max(a[1], b[1]) + EPS
    )


def intersects(a, b, c, d):
    o1, o2, o3, o4 = cross(a, b, c), cross(a, b, d), cross(c, d, a), cross(c, d, b)
    if abs(o1) <= EPS and on_segment(a, b, c):
        return True
    if abs(o2) <= EPS and on_segment(a, b, d):
        return True
    if abs(o3) <= EPS and on_segment(c, d, a):
        return True
    if abs(o4) <= EPS and on_segment(c, d, b):
        return True
    return (
        ((o1 > EPS and o2 < -EPS) or (o1 < -EPS and o2 > EPS))
        and ((o3 > EPS and o4 < -EPS) or (o3 < -EPS and o4 > EPS))
    )


def validate_case(case):
    if not case.get("case_id") or not case.get("features") or not case.get("views"):
        raise ValueError("CASE_ID_OR_REQUIRED_COLLECTION_INVALID")
    feature_ids = set()
    for feature in case["features"]:
        normal_length = norm(feature["outward_normal"])
        if (
            not feature["feature_id"]
            or feature["feature_id"] in feature_ids
            or feature["minimum_distinct_failure_domains"] < 1
            or not math.isfinite(normal_length)
            or abs(normal_length - 1.0) > UNIT_EPS
            or not all(math.isfinite(x) for x in feature["position_m"] + feature["outward_normal"])
        ):
            raise ValueError("FEATURE_CONTRACT_INVALID")
        feature_ids.add(feature["feature_id"])

    view_ids = set()
    for view in case["views"]:
        if (
            not view["view_id"]
            or view["view_id"] in view_ids
            or not view["sensor_id"]
            or not view["failure_domain_id"]
            or not all(math.isfinite(x) for x in view["origin_m"])
            or not math.isfinite(view["heading_rad"])
            or not math.isfinite(view["half_fov_deg"])
            or not 0.0 <= view["half_fov_deg"] <= 180.0
            or not math.isfinite(view["min_standoff_m"])
            or view["min_standoff_m"] < 0.0
            or not math.isfinite(view["max_standoff_m"])
            or view["max_standoff_m"] <= view["min_standoff_m"]
            or not math.isfinite(view["max_incidence_deg"])
            or not 0.0 <= view["max_incidence_deg"] <= 180.0
            or not view["pose_perturbations"]
        ):
            raise ValueError("VIEW_CONTRACT_INVALID")
        view_ids.add(view["view_id"])
        for perturbation in view["pose_perturbations"]:
            if not all(math.isfinite(perturbation[k]) for k in ("dx_m", "dy_m", "heading_delta_rad")):
                raise ValueError("POSE_PERTURBATION_INVALID")

    occluder_ids = set()
    for occluder in case["occluders"]:
        if (
            not occluder["occluder_id"]
            or occluder["occluder_id"] in occluder_ids
            or not all(math.isfinite(x) for x in occluder["start_m"] + occluder["end_m"])
            or norm(sub(occluder["start_m"], occluder["end_m"])) <= EPS
        ):
            raise ValueError("OCCLUDER_CONTRACT_INVALID")
        occluder_ids.add(occluder["occluder_id"])


def view_reasons(feature, view, origin, heading, occluders):
    sensor_to_target = sub(feature["position_m"], origin)
    distance = norm(sensor_to_target)
    if distance <= EPS:
        return ["ZERO_STANDOFF"]

    reasons = []
    if distance < view["min_standoff_m"] - EPS:
        reasons.append("STANDOFF_TOO_SHORT")
    if distance > view["max_standoff_m"] + EPS:
        reasons.append("STANDOFF_TOO_LONG")

    direction = (sensor_to_target[0] / distance, sensor_to_target[1] / distance)
    forward = (math.cos(heading), math.sin(heading))
    angle = math.degrees(math.acos(max(-1.0, min(1.0, dot(direction, forward)))))
    if angle > view["half_fov_deg"] + EPS:
        reasons.append("FIELD_OF_VIEW_EXCEEDED")

    target_to_sensor = (-direction[0], -direction[1])
    incidence = math.degrees(math.acos(max(-1.0, min(1.0, dot(feature["outward_normal"], target_to_sensor)))))
    if incidence > view["max_incidence_deg"] + EPS:
        reasons.append("INCIDENCE_ANGLE_EXCEEDED")

    for occluder in occluders:
        if intersects(origin, feature["position_m"], occluder["start_m"], occluder["end_m"]):
            reasons.append("LINE_OF_SIGHT_OCCLUDED")
            break
    return reasons


def evaluate_view(feature, view, occluders):
    nominal_reasons = view_reasons(feature, view, view["origin_m"], view["heading_rad"], occluders)
    failed = []
    for index, perturbation in enumerate(view["pose_perturbations"]):
        origin = (
            view["origin_m"][0] + perturbation["dx_m"],
            view["origin_m"][1] + perturbation["dy_m"],
        )
        heading = view["heading_rad"] + perturbation["heading_delta_rad"]
        if view_reasons(feature, view, origin, heading, occluders):
            failed.append(index)
    return {
        "view_id": view["view_id"],
        "failure_domain_id": view["failure_domain_id"],
        "nominal_visible": not nominal_reasons,
        "nominal_reason_codes": nominal_reasons,
        "robust_visible": not nominal_reasons and not failed,
        "perturbations_evaluated": len(view["pose_perturbations"]),
        "failed_perturbation_indices": failed,
    }


def evaluate_case(case):
    validate_case(case)
    feature_results = []
    case_reasons = []
    nominal_gap = False
    robust_gap = False
    all_views = []

    for feature in case["features"]:
        view_results = [evaluate_view(feature, view, case["occluders"]) for view in case["views"]]
        all_views.extend(view_results)
        nominal_ids = [r["view_id"] for r in view_results if r["nominal_visible"]]
        robust_ids = [r["view_id"] for r in view_results if r["robust_visible"]]
        nominal_domains = {r["failure_domain_id"] for r in view_results if r["nominal_visible"]}
        robust_domains = {r["failure_domain_id"] for r in view_results if r["robust_visible"]}
        required = feature["minimum_distinct_failure_domains"]

        if not nominal_ids:
            nominal_gap = True
            status = "NOT_NOMINALLY_VISIBLE"
            case_reasons.append("FEATURE_NOT_NOMINALLY_VISIBLE:" + feature["feature_id"])
        elif len(nominal_domains) < required:
            nominal_gap = True
            status = "NOMINAL_DIVERSITY_INSUFFICIENT"
            case_reasons.append("NOMINAL_FAILURE_DOMAIN_DIVERSITY_INSUFFICIENT:" + feature["feature_id"])
        elif len(robust_domains) < required:
            robust_gap = True
            status = "ROBUST_DIVERSITY_INSUFFICIENT"
            case_reasons.append("ROBUST_FAILURE_DOMAIN_DIVERSITY_NOT_ESTABLISHED:" + feature["feature_id"])
        else:
            status = "ROBUSTLY_COVERED"

        feature_results.append({
            "feature_id": feature["feature_id"],
            "status": status,
            "nominal_view_ids": nominal_ids,
            "robust_view_ids": robust_ids,
            "nominal_failure_domain_count": len(nominal_domains),
            "robust_failure_domain_count": len(robust_domains),
            "required_failure_domain_count": required,
        })

    disposition = "INFEASIBLE" if nominal_gap else ("INDETERMINATE" if robust_gap else "FEASIBLE")
    return {
        "case_id": case["case_id"],
        "disposition": disposition,
        "reason_codes": case_reasons,
        "features": feature_results,
        "views": all_views,
    }


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: inspectability_reference_oracle.py <corpus.json>")
    raw = Path(sys.argv[1]).read_bytes()
    corpus = json.loads(raw)
    if (
        corpus.get("schema") != CORPUS_SCHEMA
        or corpus.get("benchmark_id") != BENCHMARK_ID
        or corpus.get("evidence_class") != "synthetic"
        or corpus.get("claim_ceiling") != CLAIM_CEILING
        or not corpus.get("cases")
    ):
        raise SystemExit("CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID")
    ids = [case["case_id"] for case in corpus["cases"]]
    if len(ids) != len(set(ids)):
        raise SystemExit("DUPLICATE_CASE_ID")
    results = [evaluate_case(case) for case in corpus["cases"]]
    matches = sum(r["disposition"] == c["expected_disposition"] for r, c in zip(results, corpus["cases"]))
    print(json.dumps({
        "schema": RECEIPT_SCHEMA,
        "benchmark_id": BENCHMARK_ID,
        "input_sha256": hashlib.sha256(raw).hexdigest(),
        "evidence_class": "synthetic",
        "claim_ceiling": CLAIM_CEILING,
        "authority_granted": False,
        "structural_acceptance_authorized": False,
        "flight_authorized": False,
        "physical_validation": "NOT_ASSESSED",
        "case_count": len(results),
        "expected_outcome_matches": matches,
        "expected_outcome_mismatches": len(results) - matches,
        "cases": results,
    }, sort_keys=True, indent=2))


if __name__ == "__main__":
    main()
