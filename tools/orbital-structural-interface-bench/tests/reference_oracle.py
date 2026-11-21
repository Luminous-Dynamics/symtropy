#!/usr/bin/env python3
"""Independent stdlib-only oracle for the Rust structural-interface benchmark."""
import hashlib
import json
import math
import sys

CORPUS_SCHEMA = "luminous.orbital-structural-interface.corpus.v0.1"
CLAIM_CEILING = "synthetic_structural_profile_compatibility_only"


def add_reason(reasons, code, severity):
    reasons[code] = max(reasons.get(code, 0), severity)


def set_path(root, path, value):
    segments = path.split(".")
    if not segments or any(not part for part in segments):
        raise ValueError("invalid mutation path")
    cursor = root
    for segment in segments[:-1]:
        cursor = cursor[segment]
        if not isinstance(cursor, dict):
            raise ValueError("mutation parent is not an object")
    if segments[-1] not in cursor:
        raise ValueError("mutation field does not exist: " + path)
    cursor[segments[-1]] = value


def mutate(provider, client, mutation):
    target_name = mutation["target"]
    if target_name not in ("provider", "client"):
        raise ValueError("unsupported mutation target")
    target = provider if target_name == "provider" else client
    operation = mutation["operation"]
    if operation == "set":
        set_path(target, mutation["path"], mutation["value"])
    elif operation == "remove_evidence":
        before = len(target["evidence"])
        target["evidence"] = [item for item in target["evidence"]
                              if item["evidence_id"] != mutation["evidence_id"]]
        if len(target["evidence"]) == before:
            raise ValueError("evidence record not found")
    elif operation == "set_evidence":
        records = [item for item in target["evidence"]
                   if item["evidence_id"] == mutation["evidence_id"]]
        if len(records) != 1:
            raise ValueError("evidence record missing or ambiguous")
        field = mutation["field"]
        if field not in records[0]:
            raise ValueError("evidence field missing")
        records[0][field] = mutation["value"]
    elif operation == "reverse_array":
        target[mutation["path"]].reverse()
    else:
        raise ValueError("unsupported mutation operation")


def valid_digest(value):
    return (isinstance(value, str) and len(value) == 64
            and all(c in "0123456789abcdef" for c in value))


def profile_evidence(profile, reference, required_kind, required_class,
                     prefix, reasons):
    if not isinstance(reference, str) or not reference.strip():
        add_reason(reasons, prefix + "_EVIDENCE_MISSING", 1)
        return None
    found = [item for item in profile["evidence"]
             if item["evidence_id"] == reference]
    if len(found) != 1:
        add_reason(reasons, "EVIDENCE_REFERENCE_MISSING", 1)
        return None
    evidence = found[0]
    expected_subject = (profile.get("material_lineage_ref")
                        if required_kind == "material_process"
                        else profile.get("article_subject_ref"))
    if ((expected_subject is not None and expected_subject != evidence.get("subject_ref"))
            or evidence.get("configuration_id") != profile.get("configuration_id")
            or evidence.get("icd_id") != profile.get("icd_id")
            or evidence.get("icd_revision") != profile.get("icd_revision")):
        add_reason(reasons, "EVIDENCE_SCOPE_MISMATCH", 3)
    if evidence["status"] == "stale":
        add_reason(reasons, "EVIDENCE_STALE", 3)
    if evidence["status"] == "disputed":
        add_reason(reasons, "EVIDENCE_DISPUTED", 1)
    if evidence["kind"] != required_kind or evidence["evidence_class"] != required_class:
        add_reason(reasons, "EVIDENCE_CLASS_INSUFFICIENT", 1)
    return evidence


def valid_vector(vector):
    keys = ("axial_n", "shear_y_n", "shear_z_n",
            "bending_y_nm", "bending_z_nm", "torsion_nm")
    return all(isinstance(vector.get(k), (int, float))
               and math.isfinite(vector[k]) and vector[k] >= 0 for k in keys)


def valid_numbers(profile):
    keys = ("max_pose_error_mm", "max_deflection_mm", "max_slip_mm",
            "minimum_retention_n", "temperature_min_k", "temperature_max_k")
    return (all(isinstance(profile.get(k), (int, float))
                and math.isfinite(profile[k]) and profile[k] >= 0 for k in keys)
            and profile["temperature_min_k"] <= profile["temperature_max_k"]
            and all(valid_vector(case["limits"]) for case in profile["load_cases"]))


def evaluate(provider, client):
    reasons = {}
    if not valid_numbers(provider) or not valid_numbers(client):
        add_reason(reasons, "NUMERIC_PROFILE_MALFORMED", 3)

    seen = set()
    for evidence in provider["evidence"]:
        evidence_id = evidence["evidence_id"]
        if not evidence_id or evidence_id in seen:
            add_reason(reasons, "EVIDENCE_ID_DUPLICATE_OR_EMPTY", 3)
        seen.add(evidence_id)
        if not valid_digest(evidence["sha256"]):
            add_reason(reasons, "EVIDENCE_DIGEST_INVALID", 3)
        status = evidence["status"]
        if status == "stale":
            add_reason(reasons, "EVIDENCE_STALE", 3)
        elif status == "disputed":
            add_reason(reasons, "EVIDENCE_DISPUTED", 1)
        elif status != "current":
            add_reason(reasons, "EVIDENCE_STATUS_UNKNOWN", 1)

    identity_fields = (
        "profile_id", "profile_revision", "icd_id", "icd_revision",
        "configuration_id", "article_subject_ref", "frame_id", "units_profile_id",
        "geometry_profile_id", "mating_feature_id", "retention_feature_id",
        "tooling_profile_id", "material_spec_id", "material_condition_id",
        "material_lot_id", "process_route_id", "inspection_method_id")
    if any(not isinstance(profile.get(field), str) or not profile[field].strip()
           for profile in (provider, client) for field in identity_fields):
        add_reason(reasons, "IDENTITY_FIELD_EMPTY", 3)

    for profile in (provider, client):
        load_case_ids = set()
        for load_case in profile["load_cases"]:
            case_id = load_case.get("case_id", "")
            if not case_id.strip() or case_id in load_case_ids:
                add_reason(reasons, "LOAD_CASE_ID_DUPLICATE_OR_EMPTY", 3)
            load_case_ids.add(case_id)
            if not load_case.get("interaction_profile_ref", "").strip():
                add_reason(reasons, "LOAD_INTERACTION_PROFILE_ID_EMPTY", 1)
    if not client["load_cases"]:
        add_reason(reasons, "LOAD_REQUIREMENT_SET_EMPTY", 1)

    if provider["icd_id"] != client["icd_id"] or provider["icd_revision"] != client["icd_revision"]:
        add_reason(reasons, "ICD_IDENTITY_MISMATCH", 3)
    if provider["configuration_id"] != client["configuration_id"]:
        add_reason(reasons, "CONFIGURATION_IDENTITY_MISMATCH", 3)
    if provider["frame_id"] != client["frame_id"]:
        add_reason(reasons, "FRAME_MISMATCH", 1)
    if provider["units_profile_id"] != client["units_profile_id"]:
        add_reason(reasons, "UNITS_PROFILE_MISMATCH", 1)
    for field, reason in (
        ("geometry_profile_id", "GEOMETRY_PROFILE_MISMATCH"),
        ("mating_feature_id", "MATING_FEATURE_MISMATCH"),
        ("retention_feature_id", "RETENTION_FEATURE_MISMATCH"),
        ("tooling_profile_id", "TOOLING_PROFILE_MISMATCH")):
        if provider[field] != client[field]:
            add_reason(reasons, reason, 2)

    if provider["max_pose_error_mm"] > client["max_pose_error_mm"]:
        add_reason(reasons, "POSE_TOLERANCE_EXCEEDED", 2)
    if not provider["independent_lock_verification"] or not client["independent_lock_verification"]:
        add_reason(reasons, "LOCK_VERIFICATION_UNAVAILABLE", 2)
    if (not provider.get("verification_method_id")
            or not client.get("verification_method_id")):
        add_reason(reasons, "VERIFICATION_METHOD_MISSING", 1)
    verification = profile_evidence(
        provider, provider.get("verification_evidence_ref"), "verification",
        "independent_engagement_verification", "VERIFICATION", reasons)

    for required in client["load_cases"]:
        offered = next((item for item in provider["load_cases"]
                        if item["case_id"] == required["case_id"]), None)
        if offered is None:
            add_reason(reasons, "LOAD_CASE_MISSING", 1)
            continue
        if offered["frame_id"] != required["frame_id"]:
            add_reason(reasons, "LOAD_FRAME_MISMATCH", 1)
        if offered["interaction_profile_ref"] != required["interaction_profile_ref"]:
            add_reason(reasons, "LOAD_INTERACTION_PROFILE_MISMATCH", 1)
        dimensions = ("axial_n", "shear_y_n", "shear_z_n",
                      "bending_y_nm", "bending_z_nm", "torsion_nm")
        if any(offered["limits"][dimension] < required["limits"][dimension]
               for dimension in dimensions):
            add_reason(reasons, "LOAD_CAPACITY_INSUFFICIENT", 2)
        profile_evidence(
            provider, offered.get("evidence_ref"), "structural_capability",
            "scoped_test_or_validated_analysis", "STRUCTURAL", reasons)

    if provider["max_deflection_mm"] > client["max_deflection_mm"]:
        add_reason(reasons, "DEFLECTION_LIMIT_EXCEEDED", 2)
    if provider["max_slip_mm"] > client["max_slip_mm"]:
        add_reason(reasons, "SLIP_LIMIT_EXCEEDED", 2)
    if provider["minimum_retention_n"] < client["minimum_retention_n"]:
        add_reason(reasons, "RETENTION_CAPACITY_INSUFFICIENT", 2)
    if (provider["temperature_min_k"] > client["temperature_min_k"]
            or provider["temperature_max_k"] < client["temperature_max_k"]):
        add_reason(reasons, "TEMPERATURE_RANGE_INSUFFICIENT", 2)
    if provider["maximum_service_cycles"] < client["maximum_service_cycles"]:
        add_reason(reasons, "SERVICE_LIFE_INSUFFICIENT", 2)

    for field, reason in (
        ("material_spec_id", "MATERIAL_SPEC_MISMATCH"),
        ("material_condition_id", "MATERIAL_CONDITION_MISMATCH"),
        ("material_lot_id", "MATERIAL_LOT_MISMATCH"),
        ("process_route_id", "PROCESS_ROUTE_MISMATCH")):
        if provider[field] != client[field]:
            add_reason(reasons, reason, 2)
    if not provider.get("material_lineage_ref"):
        add_reason(reasons, "MATERIAL_LINEAGE_MISSING", 1)
    material = profile_evidence(
        provider, provider.get("material_evidence_ref"), "material_process",
        "process_conditioned_material_characterization", "MATERIAL", reasons)

    if provider["inspection_method_id"] != client["inspection_method_id"]:
        add_reason(reasons, "INSPECTION_METHOD_UNMAPPED", 1)
    if client["critical_surface_observable"] and not provider["critical_surface_observable"]:
        add_reason(reasons, "CRITICAL_SURFACE_NOT_OBSERVABLE", 2)
    inspection = profile_evidence(
        provider, provider.get("inspection_evidence_ref"), "inspection",
        "method_specific_inspection_report", "INSPECTION", reasons)

    if client["requires_replacement"] and not provider["replacement_isolated_and_reverified"]:
        add_reason(reasons, "REPLACEMENT_REVERIFICATION_MISSING", 1)
    if client["requires_fatigue"]:
        profile_evidence(
            provider, provider.get("fatigue_evidence_ref"), "fatigue",
            "fatigue_life_evidence", "FATIGUE", reasons)
    if client["requires_assembled_path"]:
        profile_evidence(
            provider, provider.get("assembled_path_evidence_ref"), "assembled_path",
            "as_built_load_path_acceptance", "ASSEMBLED_PATH", reasons)
    if provider["uncertainty_basis"] != "hard_bound" or client["uncertainty_basis"] != "hard_bound":
        add_reason(reasons, "UNCERTAINTY_BASIS_UNSUPPORTED", 1)

    if client["requires_independent_evidence"]:
        structural = None
        if provider["load_cases"]:
            structural = profile_evidence(
                provider, provider["load_cases"][0].get("evidence_ref"),
                "structural_capability", "scoped_test_or_validated_analysis",
                "STRUCTURAL", reasons)
        if structural is not None and verification is not None:
            if structural["dependency_group"] == verification["dependency_group"]:
                add_reason(reasons, "INDEPENDENCE_NOT_ESTABLISHED", 1)
    if (material is not None and inspection is not None
            and material["dependency_group"] == inspection["dependency_group"]
            and client["requires_independent_evidence"]):
        add_reason(reasons, "MATERIAL_INSPECTION_COMMON_CAUSE", 1)

    if any(value == 3 for value in reasons.values()):
        disposition = "malformed_or_stale"
    elif any(value == 2 for value in reasons.values()):
        disposition = "incompatible"
    elif reasons:
        disposition = "indeterminate"
    else:
        disposition = "compatible_synthetic_profile"
    return disposition, sorted(reasons)


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: reference_oracle.py <cases.json> <40-hex-source-commit>")
    raw = open(sys.argv[1], "rb").read()
    source_commit = sys.argv[2]
    if len(source_commit) != 40 or any(c not in "0123456789abcdef" for c in source_commit):
        raise SystemExit("invalid source commit")
    corpus = json.loads(raw)
    if corpus.get("schema") != CORPUS_SCHEMA or corpus.get("claim_ceiling") != CLAIM_CEILING:
        raise SystemExit("unsupported schema or claim ceiling")
    if corpus.get("evidence_class") != "synthetic":
        raise SystemExit("only synthetic fixtures are admitted")
    results = []
    for spec in corpus["cases"]:
        provider = json.loads(json.dumps(corpus["base_provider"]))
        client = json.loads(json.dumps(corpus["base_client"]))
        for mutation in spec.get("mutations", []):
            mutate(provider, client, mutation)
        actual, reasons = evaluate(provider, client)
        expected = spec["expected"]
        matched = actual == expected["disposition"] and reasons == expected["reasons"]
        results.append({"case_id": spec["case_id"], "disposition": actual,
                        "reasons": reasons, "expected_matched": matched})
    results.sort(key=lambda item: item["case_id"])
    matched = sum(1 for item in results if item["expected_matched"])
    report = {
        "schema": "luminous.orbital-structural-interface.reference-oracle.v0.1",
        "input_sha256": hashlib.sha256(raw).hexdigest(),
        "source_commit": source_commit,
        "case_count": len(results),
        "matched_cases": matched,
        "mismatched_cases": len(results) - matched,
        "results": results,
    }
    print(json.dumps(report, sort_keys=True, indent=2))
    if len(results) != 44 or matched != 44:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
