#!/usr/bin/env python3
"""CI-only independent oracle for AERO claim model obligation binding v0.1.

Rust is the production adapter. This script independently reconstructs the
model-cut and model-mapping commitments and cross-checks the upstream
obligation-to-source binding using its independent validator.
"""
from __future__ import annotations

import copy
import hashlib
import json
import sys
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker
import validate_claim_obligation_evidence_binding as evidence_binding_oracle

ROOT = Path(__file__).resolve().parents[2]
AVIATION = ROOT / "docs" / "research" / "aviation"
SCHEMA_PATH = AVIATION / "schemas" / "aero_claim_model_obligation_binding_v0_1.schema.json"
EXAMPLE_PATH = AVIATION / "examples" / "aero_claim_model_obligation_binding_v0_1.json"
CANONICALIZATION = "rust-serde-json-struct-order-v1"
MODALITY_ORDER = [
    "synthetic_known_truth",
    "analytic_reference",
    "software_in_loop",
    "hardware_in_loop",
    "calibrated_component_bench",
    "controlled_flight_test",
    "shadow_mode_field_replay",
]
UNCERTAINTY_ORDER = [
    "observation_noise",
    "measurement_uncertainty",
    "parameter_uncertainty",
    "structural_model_discrepancy",
    "environmental_uncertainty",
]


def digest_json(value: Any) -> str:
    return hashlib.sha256(
        json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


def model_cut_sha256(model: dict[str, Any]) -> str:
    modalities = sorted(
        model["required_modalities"], key=MODALITY_ORDER.index
    )
    modality_evidence = sorted(
        model["modality_evidence"],
        key=lambda item: (
            MODALITY_ORDER.index(item["modality"]),
            item["evidence_ref"],
        ),
    )
    required_uncertainty = sorted(
        model["required_uncertainty_components"], key=UNCERTAINTY_ORDER.index
    )
    uncertainty_evidence = sorted(
        model["uncertainty_components"],
        key=lambda item: UNCERTAINTY_ORDER.index(item["component"]),
    )
    required_metric_strata = sorted(
        model["required_metric_strata"],
        key=lambda item: (item["metric_id"], item["stratum_ref"]),
    )
    metric_reports = sorted(
        model["metric_stratum_reports"],
        key=lambda item: (item["metric_id"], item["stratum_ref"]),
    )
    return digest_json({
        "schema": "AeroModelCredibilityCutV0_1",
        "canonicalization_profile": CANONICALIZATION,
        "evidence_ref": model["evidence_ref"],
        "claim_id": model["claim_id"],
        "model_ref": model["model_ref"],
        "model_artifact_sha256": model["model_artifact_sha256"],
        "solver_configuration_sha256": model["solver_configuration_sha256"],
        "configuration_ref": model["configuration_ref"],
        "operating_envelope_ref": model["operating_envelope_ref"],
        "applicability_profile_sha256": model["applicability_profile_sha256"],
        "risk_classification_sha256": model["risk_classification_sha256"],
        "validation_package_sha256": model["validation_package_sha256"],
        "validation_protocol_sha256": model["validation_protocol_sha256"],
        "calibration_manifest_sha256": model["calibration_manifest_sha256"],
        "development_manifest_sha256": model["development_manifest_sha256"],
        "confirmatory_manifest_sha256": model["confirmatory_manifest_sha256"],
        "independent_reference_manifest_sha256": model["independent_reference_manifest_sha256"],
        "package_status": model["package_status"],
        "split_integrity_status": model["split_integrity_status"],
        "independent_reference_status": model["independent_reference_status"],
        "domain_coverage_status": model["domain_coverage_status"],
        "required_modalities": modalities,
        "modality_evidence": modality_evidence,
        "required_uncertainty_components": required_uncertainty,
        "uncertainty_components": uncertainty_evidence,
        "required_metric_strata": required_metric_strata,
        "metric_stratum_reports": metric_reports,
    })


def mapping_digests(source: dict[str, Any], profile_sha: str, obligation_sha: str, model_sha: str) -> tuple[str, str, str]:
    upstream = source["evidence_binding_input"]
    spec = upstream["claim_specification"]
    context = upstream["decision_context"]
    applicability = source["applicability_mapping"]
    applicability_sha = digest_json({
        "schema": "AeroModelApplicabilityMappingManifestV0_1",
        "canonicalization_profile": CANONICALIZATION,
        "claim_id": spec["claim_id"],
        "decision_context_sha256": upstream["decision_context_sha256"],
        "claim_specification_semantic_sha256": context["claim_specification_sha256"],
        "obligation_profile_sha256": profile_sha,
        "obligation_manifest_sha256": obligation_sha,
        "model_credibility_cut_sha256": model_sha,
        "entry": applicability,
    })
    metrics = sorted(
        source["metric_strata_mappings"],
        key=lambda item: (item["metric_id"], item["model_stratum_ref"]),
    )
    metric_sha = digest_json({
        "schema": "AeroModelMetricStrataMappingManifestV0_1",
        "canonicalization_profile": CANONICALIZATION,
        "claim_id": spec["claim_id"],
        "decision_context_sha256": upstream["decision_context_sha256"],
        "claim_specification_semantic_sha256": context["claim_specification_sha256"],
        "obligation_profile_sha256": profile_sha,
        "obligation_manifest_sha256": obligation_sha,
        "model_credibility_cut_sha256": model_sha,
        "entries": metrics,
    })
    uncertainty = sorted(
        source["uncertainty_mappings"],
        key=lambda item: UNCERTAINTY_ORDER.index(item["component"]),
    )
    uncertainty_sha = digest_json({
        "schema": "AeroModelUncertaintyMappingManifestV0_1",
        "canonicalization_profile": CANONICALIZATION,
        "claim_id": spec["claim_id"],
        "decision_context_sha256": upstream["decision_context_sha256"],
        "claim_specification_semantic_sha256": context["claim_specification_sha256"],
        "obligation_profile_sha256": profile_sha,
        "obligation_manifest_sha256": obligation_sha,
        "model_credibility_cut_sha256": model_sha,
        "uncertainty_method_ref": spec["uncertainty"]["method_ref"],
        "uncertainty_treatment": spec["uncertainty"]["treatment"],
        "confidence_level_ppm": spec["uncertainty"]["confidence_level_ppm"],
        "entries": uncertainty,
    })
    return applicability_sha, metric_sha, uncertainty_sha


def expected_disposition(source: dict[str, Any]) -> tuple[list[str], str]:
    errors: list[str] = []
    upstream = source["evidence_binding_input"]
    upstream_errors, upstream_disposition = evidence_binding_oracle.validate(upstream)
    if upstream_errors or upstream_disposition != "BOUND":
        errors.append("upstream evidence-class binding is not BOUND")
        return errors, "REJECTED"

    spec = upstream["claim_specification"]
    context = upstream["decision_context"]
    model = source["model_credibility_cut"]
    receipt = source["mapping_receipt"]
    cutoff = context["as_known_at_unix_ms"]

    semantic_sha = evidence_binding_oracle.semantic_sha256(spec)
    profile_sha = evidence_binding_oracle.requirement_profile_sha256()
    obligations = evidence_binding_oracle.derive_obligations(spec)
    obligation_sha = evidence_binding_oracle.obligation_manifest_sha256(
        spec, semantic_sha, profile_sha, obligations
    )
    context_sha = evidence_binding_oracle.decision_context_sha256(context)
    model_sha = model_cut_sha256(model)

    if context_sha != upstream["decision_context_sha256"]:
        errors.append("decision context digest does not match the upstream evidence binding")
    if model["claim_id"] != spec["claim_id"]:
        errors.append("model cut claim ID mismatch")
    if model["configuration_ref"] != context["configuration_ref"]:
        errors.append("model cut configuration mismatch")
    if model["operating_envelope_ref"] != context["operating_envelope_ref"]:
        errors.append("model cut operating-envelope mismatch")
    if model["applicability_profile_sha256"] != context["model_applicability_profile_sha256"]:
        errors.append("model cut applicability-profile mismatch")
    if model["risk_classification_sha256"] != context["risk_classification_sha256"]:
        errors.append("model cut risk-classification mismatch")

    owner_cut = upstream["owner_evidence_cut"]
    owner_record = next(
        (item for item in owner_cut["evidence_records"] if item["evidence_ref"] == model["evidence_ref"]),
        None,
    )
    source_receipt = next(
        (item for item in upstream["source_receipts"] if item["evidence_ref"] == model["evidence_ref"]),
        None,
    )
    model_class_entry = next(
        (
            item for item in upstream["mapping_entries"]
            if item["evidence_class"] == "evidence-class:model-validation"
        ),
        None,
    )
    if model_class_entry is None or model["evidence_ref"] not in model_class_entry["evidence_refs"]:
        errors.append("model cut evidence ref is not mapped to the model-validation evidence class")
    if owner_record is None or owner_record["record_sha256"] != model_sha:
        errors.append("model cut digest differs from source-owned evidence record")
    if source_receipt is None or source_receipt["evidence_record_sha256"] != model_sha:
        errors.append("model cut digest differs from its per-record source receipt")

    derived = {item["obligation_id"]: item for item in obligations}
    applicability = source["applicability_mapping"]
    applicability_obligation = derived.get("model-applicability")
    if applicability_obligation is None:
        errors.append("model-applicability obligation not derived")
    elif (
        applicability["obligation_id"] != "model-applicability"
        or applicability["requirement_source_ref"] != applicability_obligation["requirement_source_ref"]
        or applicability["model_ref"] != model["model_ref"]
        or applicability["model_artifact_sha256"] != model["model_artifact_sha256"]
        or applicability["solver_configuration_sha256"] != model["solver_configuration_sha256"]
        or applicability["operating_envelope_ref"] != spec["operating_envelope_ref"]
        or applicability["applicability_profile_sha256"] != context["model_applicability_profile_sha256"]
        or applicability["risk_classification_sha256"] != context["risk_classification_sha256"]
    ):
        errors.append("model applicability mapping scope mismatch")

    required_model_pairs = {
        (item["metric_id"], item["stratum_ref"]) for item in model["required_metric_strata"]
    }
    mapped_pairs = [
        (item["metric_id"], item["model_stratum_ref"])
        for item in source["metric_strata_mappings"]
    ]
    if len(mapped_pairs) != len(set(mapped_pairs)) or set(mapped_pairs) != required_model_pairs:
        errors.append("model metric/stratum mapping set mismatch")
    claim_strata = {
        item["stratum_id"]: item
        for item in spec["population"]["required_strata"]
    }
    covered_claim_strata: set[str] = set()
    metric_dispositions: list[str] = []
    for entry in source["metric_strata_mappings"]:
        obligation = derived.get(entry["obligation_id"])
        claim_stratum = claim_strata.get(entry["claim_stratum_id"])
        if claim_stratum is None or obligation is None:
            errors.append("model stratum mapping references an unknown claim stratum/obligation")
            continue
        expected_scope = "|".join([
            spec["population"]["population_ref"],
            claim_stratum["dimension_ref"],
            claim_stratum["level_ref"],
        ])
        if (
            entry["obligation_id"] != f"model-credibility:{claim_stratum['stratum_id']}"
            or obligation["kind"] != "MODEL_CREDIBILITY"
            or entry["requirement_source_ref"] != obligation["requirement_source_ref"]
            or entry["population_manifest_sha256"] != spec["population"]["population_manifest_sha256"]
            or entry["metric_id"] not in {metric for metric, _ in required_model_pairs}
            or not expected_scope
        ):
            errors.append("model stratum mapping does not preserve the exact derived claim scope")
        else:
            covered_claim_strata.add(claim_stratum["stratum_id"])
        report = next(
            (
                item for item in model["metric_stratum_reports"]
                if item["metric_id"] == entry["metric_id"]
                and item["stratum_ref"] == entry["model_stratum_ref"]
            ),
            None,
        )
        if report is None:
            errors.append("model metric stratum report missing")
            continue
        if report["population_manifest_sha256"] != spec["population"]["population_manifest_sha256"]:
            errors.append("model metric population manifest mismatch")
        metric_dispositions.append(report["status"])

    if covered_claim_strata != set(claim_strata):
        errors.append("not every claim-required population stratum is bound")

    uncertainty_obligation = derived.get("model-uncertainty")
    required_uncertainty = set(model["required_uncertainty_components"])
    mapped_uncertainty = [item["component"] for item in source["uncertainty_mappings"]]
    if uncertainty_obligation is None:
        errors.append("model-uncertainty obligation not derived")
    if len(mapped_uncertainty) != len(set(mapped_uncertainty)) or set(mapped_uncertainty) != required_uncertainty:
        errors.append("model uncertainty component mapping set mismatch")
    uncertainty_dispositions: list[str] = []
    for entry in source["uncertainty_mappings"]:
        component = next(
            (item for item in model["uncertainty_components"] if item["component"] == entry["component"]),
            None,
        )
        if (
            entry["obligation_id"] != "model-uncertainty"
            or uncertainty_obligation is None
            or entry["requirement_source_ref"] != uncertainty_obligation["requirement_source_ref"]
            or entry["uncertainty_method_ref"] != spec["uncertainty"]["method_ref"]
            or component is None
        ):
            errors.append("model uncertainty mapping scope mismatch")
            continue
        if (
            entry["method_profile_sha256"] != component["method_profile_sha256"]
            or entry["report_sha256"] != component["report_sha256"]
        ):
            errors.append("model uncertainty report commitment mismatch")
        uncertainty_dispositions.append(component["status"])

    receipt_digests = mapping_digests(source, profile_sha, obligation_sha, model_sha)
    if (
        receipt["verification_status"] != "VERIFIED"
        or receipt["owner_namespace"] != owner_cut["owner_namespace"]
        or receipt["claim_id"] != spec["claim_id"]
        or receipt["configuration_ref"] != spec["configuration_ref"]
        or receipt["decision_context_sha256"] != context_sha
        or receipt["claim_specification_semantic_sha256"] != semantic_sha
        or receipt["obligation_profile_sha256"] != profile_sha
        or receipt["obligation_manifest_sha256"] != obligation_sha
        or receipt["model_evidence_ref"] != model["evidence_ref"]
        or receipt["model_evidence_record_sha256"] != model_sha
        or receipt["model_credibility_cut_sha256"] != model_sha
        or receipt["applicability_mapping_sha256"] != receipt_digests[0]
        or receipt["metric_strata_mapping_sha256"] != receipt_digests[1]
        or receipt["uncertainty_mapping_sha256"] != receipt_digests[2]
        or receipt["uncertainty_treatment"] != "DeterministicUpperBound"
        or receipt["uncertainty_method_ref"] != spec["uncertainty"]["method_ref"]
        or receipt["uncertainty_confidence_level_ppm"] != spec["uncertainty"]["confidence_level_ppm"]
        or len(receipt["receipt_sha256"]) != 64
    ):
        errors.append("model mapping receipt scope or commitment mismatch")
    late_receipt = receipt["receiver_first_seen_at_unix_ms"] > cutoff
    if not receipt["effective_from_unix_ms"] <= cutoff < receipt["effective_until_unix_ms"]:
        errors.append("model mapping receipt is not effective at cutoff")

    # Reproduce the non-compensatory model cut blockers.
    blockers: list[str] = []
    if model["package_status"] == "rejected":
        blockers.append("MODEL_CREDIBILITY_PACKAGE_REJECTED")
    elif model["package_status"] == "incomplete":
        blockers.append("MODEL_VALIDATION_PACKAGE_INCOMPLETE")
    if model["split_integrity_status"] == "leakage_detected":
        blockers.append("MODEL_CREDIBILITY_CONFIRMATORY_SPLIT_LEAKAGE_REJECTED")
    elif model["split_integrity_status"] == "unverified":
        blockers.append("MODEL_CREDIBILITY_CONFIRMATORY_SPLIT_UNVERIFIED")
    if model["independent_reference_status"] == "absent":
        blockers.append("MODEL_INDEPENDENT_REFERENCE_ABSENT")
    elif model["independent_reference_status"] == "unverified":
        blockers.append("MODEL_INDEPENDENT_REFERENCE_UNVERIFIED")
    if model["domain_coverage_status"] == "not_covered":
        blockers.append("MODEL_CLAIM_OUTSIDE_VALIDATED_DOMAIN")
    elif model["domain_coverage_status"] == "indeterminate":
        blockers.append("MODEL_DOMAIN_COVERAGE_INDETERMINATE")
    elif model["domain_coverage_status"] == "unverified":
        blockers.append("MODEL_DOMAIN_COVERAGE_UNVERIFIED")
    modalities = {item["modality"]: item for item in model["modality_evidence"]}
    for required in model["required_modalities"]:
        if required not in modalities or modalities[required]["status"] != "qualified_for_declared_purpose":
            blockers.append(f"MODEL_REQUIRED_MODALITY_NOT_QUALIFIED:{required}")
    uncertainty_by_component = {item["component"]: item for item in model["uncertainty_components"]}
    for required in required_uncertainty:
        item = uncertainty_by_component.get(required)
        if item is None or item["status"] == "unverified":
            blockers.append(f"MODEL_UNCERTAINTY_COMPONENT_UNVERIFIED:{required}")
        elif item["status"] == "uncharacterized":
            blockers.append(f"MODEL_UNCERTAINTY_COMPONENT_UNCHARACTERIZED:{required}")
    metric_report_map = {(item["metric_id"], item["stratum_ref"]): item for item in model["metric_stratum_reports"]}
    for metric, stratum in required_model_pairs:
        item = metric_report_map.get((metric, stratum))
        if item is None:
            blockers.append(f"MODEL_METRIC_STRATUM_REPORT_MISSING:{metric}:{stratum}")
        elif item["status"] == "outside_declared_profile":
            blockers.append(f"MODEL_METRIC_OUTSIDE_DECLARED_PROFILE:{metric}:{stratum}")
        elif item["status"] == "indeterminate":
            blockers.append(f"MODEL_METRIC_STRATUM_INDETERMINATE:{metric}:{stratum}")
        elif item["status"] == "unverified":
            blockers.append(f"MODEL_METRIC_STRATUM_UNVERIFIED:{metric}:{stratum}")

    if errors:
        disposition = "REJECTED"
    elif late_receipt:
        disposition = "UNVERIFIED"
    elif model["domain_coverage_status"] == "not_covered" or "outside_declared_profile" in metric_dispositions:
        disposition = "OUT_OF_SCOPE"
    elif any(x == "unverified" for x in uncertainty_dispositions + metric_dispositions) or model["split_integrity_status"] == "unverified" or model["domain_coverage_status"] == "unverified":
        disposition = "UNVERIFIED"
    elif blockers:
        disposition = "INCOMPLETE"
    else:
        disposition = "BOUND"
    return errors + blockers, disposition


def main() -> int:
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    instance = json.loads(EXAMPLE_PATH.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    schema_errors = sorted(
        validator.iter_errors(instance),
        key=lambda error: (list(map(str, error.absolute_path)), error.message),
    )
    if schema_errors:
        for error in schema_errors:
            print(f"SCHEMA FAIL /{'/'.join(map(str, error.absolute_path))}: {error.message}")
        return 1

    errors, disposition = expected_disposition(instance)
    if errors or disposition != "BOUND":
        for error in errors:
            print(f"CONTRACT FAIL: {error}")
        raise SystemExit(f"FAIL: synthetic model-obligation binding disposition is {disposition}")

    # 10 mutation / permutation controls.
    missing_metric = copy.deepcopy(instance)
    missing_metric["metric_strata_mappings"].pop()
    missing_errors, missing_result = expected_disposition(missing_metric)
    if missing_result != "REJECTED" or not missing_errors:
        raise SystemExit("FAIL: missing metric/stratum mapping was accepted")

    wrong_stratum = copy.deepcopy(instance)
    wrong_stratum["metric_strata_mappings"][0]["claim_stratum_id"] = "stratum:unrelated"
    wrong_errors, wrong_result = expected_disposition(wrong_stratum)
    if wrong_result != "REJECTED" or not wrong_errors:
        raise SystemExit("FAIL: mapping to an unrelated claim stratum was accepted")

    wrong_owner = copy.deepcopy(instance)
    wrong_owner["mapping_receipt"]["owner_namespace"] = "owner:unrelated-model-owner"
    wrong_errors, _ = expected_disposition(wrong_owner)
    if not any("receipt scope or commitment mismatch" in error for error in wrong_errors):
        raise SystemExit("FAIL: model mapping receipt from another owner was accepted")

    late_receipt = copy.deepcopy(instance)
    late_receipt["mapping_receipt"]["receiver_first_seen_at_unix_ms"] = (
        late_receipt["evidence_binding_input"]["decision_context"]["as_known_at_unix_ms"] + 1
    )
    late_errors, late_result = expected_disposition(late_receipt)
    if late_result != "UNVERIFIED":
        raise SystemExit("FAIL: late model mapping receipt backfilled historical knowledge")

    wrong_method = copy.deepcopy(instance)
    wrong_method["uncertainty_mappings"][0]["uncertainty_method_ref"] = "method:unrelated"
    wrong_method_errors, wrong_method_result = expected_disposition(wrong_method)
    if wrong_method_result != "REJECTED":
        raise SystemExit("FAIL: uncertainty method substitution was accepted")

    changed_report = copy.deepcopy(instance)
    changed_report["model_credibility_cut"]["metric_stratum_reports"][0]["report_sha256"] = "9" * 64
    changed_report_errors, changed_report_result = expected_disposition(changed_report)
    if changed_report_result != "REJECTED":
        raise SystemExit("FAIL: changed model report without refreshed owner binding was accepted")

    wrong_population = copy.deepcopy(instance)
    wrong_population["metric_strata_mappings"][0]["population_manifest_sha256"] = "9" * 64
    wrong_population_errors, wrong_population_result = expected_disposition(wrong_population)
    if wrong_population_result != "REJECTED":
        raise SystemExit("FAIL: mapping to another population manifest was accepted")

    failed_receipt = copy.deepcopy(instance)
    failed_receipt["mapping_receipt"]["verification_status"] = "FAILED"
    failed_errors, failed_result = expected_disposition(failed_receipt)
    if failed_result != "REJECTED":
        raise SystemExit("FAIL: failed model mapping receipt was accepted")

    permuted = copy.deepcopy(instance)
    permuted["metric_strata_mappings"].reverse()
    permuted["uncertainty_mappings"].reverse()
    permuted["model_credibility_cut"]["required_modalities"].reverse()
    permuted["model_credibility_cut"]["modality_evidence"].reverse()
    permuted["model_credibility_cut"]["required_uncertainty_components"].reverse()
    permuted["model_credibility_cut"]["uncertainty_components"].reverse()
    permuted["model_credibility_cut"]["required_metric_strata"].reverse()
    permuted["model_credibility_cut"]["metric_stratum_reports"].reverse()
    permuted_errors, permuted_result = expected_disposition(permuted)
    if permuted_errors or permuted_result != "BOUND":
        raise SystemExit("FAIL: order-only permutation changed model binding result")

    late_source = copy.deepcopy(instance)
    source = next(
        item for item in late_source["evidence_binding_input"]["source_receipts"]
        if item["evidence_ref"] == late_source["model_credibility_cut"]["evidence_ref"]
    )
    source["receiver_first_seen_at_unix_ms"] = (
        late_source["evidence_binding_input"]["decision_context"]["as_known_at_unix_ms"] + 1
    )
    late_source_errors, late_source_result = expected_disposition(late_source)
    if late_source_result != "REJECTED":
        raise SystemExit("FAIL: late source receipt was accepted by the model binding")

    print(
        "PASS: AERO model obligation binding v0.1 schema, source-bound model-cut digest, "
        "applicability/strata/uncertainty mappings, receipt chronology and 10 mutation/permutation controls."
    )
    print(
        "NOTE: synthetic approvals are placeholders. This oracle does not authenticate signatures, "
        "establish model credibility, validate physics or grant engineering acceptance/physical authority."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
