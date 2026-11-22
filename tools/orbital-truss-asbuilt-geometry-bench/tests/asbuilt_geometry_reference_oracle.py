#!/usr/bin/env python3
"""Independent standard-library oracle for the frozen 2D geometry corpus."""
import hashlib
import json
import math
import sys

SCHEMA = "luminous.orbital-truss-asbuilt-geometry.corpus.v0.1"
CLAIM = "synthetic_2d_asbuilt_geometry_conformance_only"
EPS = 1.0e-9


def transform(registration, point):
    r = registration["rotation"]
    t = registration["translation_m"]
    return [
        r[0][0] * point[0] + r[0][1] * point[1] + t[0],
        r[1][0] * point[0] + r[1][1] * point[1] + t[1],
    ]


def valid_registration(reg):
    if (not reg["source_frame_id"].strip()
            or not reg["target_frame_id"].strip()
            or not reg["independent_datum_set_id"].strip()
            or reg["basis"] != "independent_external_datums"
            or reg["part_geometry_used_for_fit"]
            or reg["control_point_count"] < 2
            or not math.isfinite(reg["registration_uncertainty_m"])
            or reg["registration_uncertainty_m"] < 0):
        return False, "REGISTRATION_EVIDENCE_INVALID"
    a, b = reg["rotation"][0]
    c, d = reg["rotation"][1]
    det = a * d - b * c
    if (abs(a*a + b*b - 1.0) > EPS
            or abs(c*c + d*d - 1.0) > EPS
            or abs(a*c + b*d) > EPS
            or abs(det - 1.0) > EPS):
        return False, "REGISTRATION_TRANSFORM_NOT_PROPER_RIGID_2D"
    return True, None


def valid_instrument(inst, source_time):
    digest = inst["calibration_evidence_sha256"]
    valid_digest = len(digest) == 64 and all(ch in "0123456789abcdef" for ch in digest)
    ok = (
        bool(inst["instrument_id"].strip())
        and bool(inst["calibration_id"].strip())
        and valid_digest
        and inst["valid_from_unix_ms"] <= inst["valid_until_unix_ms"]
        and inst["valid_from_unix_ms"] <= inst["used_at_unix_ms"] <= inst["valid_until_unix_ms"]
        and inst["used_at_unix_ms"] == source_time
    )
    return ok


def unresolved(req):
    return {
        "feature_id": req["feature_id"],
        "critical": req["critical"],
        "disposition": "UNRESOLVED",
        "residual_m": None,
        "uncertainty_radius_m": None,
        "lower_error_bound_m": None,
        "upper_error_bound_m": None,
        "tolerance_m": req["tolerance_m"],
    }


def evaluate(case):
    feature_ids = [item["feature_id"] for item in case["required_features"]]
    if len(feature_ids) != len(set(feature_ids)) or not feature_ids:
        raise ValueError("invalid required feature identity")
    measurement_ids = [item["feature_id"] for item in case["measurements"]]
    if any(item not in feature_ids for item in measurement_ids):
        raise ValueError("measurement references undeclared feature")

    reasons = []
    forced = None
    if case["expected_subject_id"] != case["evidence_subject_id"]:
        reasons.append("PHYSICAL_UNIT_IDENTITY_MISMATCH")
        forced = "REJECT"
    if case["design_revision"] != case["evidence_design_revision"]:
        reasons.append("DESIGN_REVISION_MISMATCH")
        if forced != "REJECT":
            forced = "INDETERMINATE_HUMAN_REVIEW"
    reg_ok, reg_error = valid_registration(case["registration"])
    if not reg_ok:
        reasons.append(reg_error)
        if forced != "REJECT":
            forced = "INDETERMINATE_HUMAN_REVIEW"
    if not valid_instrument(case["instrument"], case["source_time_unix_ms"]):
        reasons.append("CALIBRATION_EVIDENCE_INVALID_OR_OUT_OF_WINDOW")
        if forced != "REJECT":
            forced = "INDETERMINATE_HUMAN_REVIEW"
    if len(measurement_ids) != len(set(measurement_ids)):
        reasons.append("DUPLICATE_MEASUREMENT_IDENTITY")
        if forced != "REJECT":
            forced = "INDETERMINATE_HUMAN_REVIEW"

    if forced is not None:
        return {
            "case_id": case["case_id"],
            "disposition": forced,
            "reason_codes": reasons,
            "features": [unresolved(req) for req in case["required_features"]],
            "authority_granted": False,
        }

    measurements = {item["feature_id"]: item for item in case["measurements"]}
    results = []
    has_reject = False
    has_unresolved = False
    for req in case["required_features"]:
        item = measurements.get(req["feature_id"])
        if item is None:
            has_unresolved = True
            reasons.append("REQUIRED_FEATURE_NOT_MEASURED:" + req["feature_id"])
            results.append(unresolved(req))
            continue
        if not item["observed"]:
            has_unresolved = True
            reasons.append("REQUIRED_FEATURE_UNOBSERVED:" + req["feature_id"])
            results.append(unresolved(req))
            continue
        position = transform(case["registration"], item["observed_position_source_m"])
        nominal = req["nominal_position_m"]
        residual = math.hypot(position[0] - nominal[0], position[1] - nominal[1])
        uncertainty = item["measurement_uncertainty_m"] + case["registration"]["registration_uncertainty_m"]
        lower = max(0.0, residual - uncertainty)
        upper = residual + uncertainty
        if upper <= req["tolerance_m"]:
            feature_disposition = "WITHIN_TOLERANCE"
        elif lower > req["tolerance_m"]:
            has_reject = True
            reasons.append("FEATURE_OUTSIDE_TOLERANCE:" + req["feature_id"])
            feature_disposition = "OUTSIDE_TOLERANCE"
        else:
            has_unresolved = True
            reasons.append("FEATURE_GUARD_BAND_OVERLAP:" + req["feature_id"])
            feature_disposition = "UNRESOLVED"
        results.append({
            "feature_id": req["feature_id"],
            "critical": req["critical"],
            "disposition": feature_disposition,
            "residual_m": residual,
            "uncertainty_radius_m": uncertainty,
            "lower_error_bound_m": lower,
            "upper_error_bound_m": upper,
            "tolerance_m": req["tolerance_m"],
        })

    disposition = "REJECT" if has_reject else (
        "INDETERMINATE_HUMAN_REVIEW" if has_unresolved else "ACCEPT"
    )
    reasons.sort()
    return {
        "case_id": case["case_id"],
        "disposition": disposition,
        "reason_codes": reasons,
        "features": results,
        "authority_granted": False,
    }


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: asbuilt_geometry_reference_oracle.py <corpus.json>")
    raw = open(sys.argv[1], "rb").read()
    corpus = json.loads(raw)
    if (corpus.get("schema") != SCHEMA
            or corpus.get("benchmark_id") != "ORBITAL-TRUSS-ASBUILT-GEOMETRY-001"
            or corpus.get("evidence_class") != "synthetic"
            or corpus.get("claim_ceiling") != CLAIM):
        raise SystemExit("unsupported corpus contract")
    if len(corpus["cases"]) != 13:
        raise SystemExit("unexpected frozen corpus cardinality")
    outputs = [evaluate(case) for case in corpus["cases"]]
    outputs.sort(key=lambda item: item["case_id"])
    expected = {case["case_id"]: case["expected_disposition"] for case in corpus["cases"]}
    matches = sum(item["disposition"] == expected[item["case_id"]] for item in outputs)
    report = {
        "input_sha256": hashlib.sha256(raw).hexdigest(),
        "case_count": len(outputs),
        "oracle_matches": matches,
        "oracle_mismatches": len(outputs) - matches,
        "cases": outputs,
    }
    print(json.dumps(report, sort_keys=True, indent=2))
    if matches != len(outputs):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
