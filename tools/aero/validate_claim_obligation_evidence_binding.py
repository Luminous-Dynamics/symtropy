#!/usr/bin/env python3
"""CI-only independent oracle for AERO obligation-to-source evidence binding v0.1.

Rust is the production adapter. This script independently recomputes the
struct-order digest chain and exercises fail-closed mapping/receipt mutations.
It does not authenticate signatures or establish physical truth.
"""
from __future__ import annotations

import copy
import hashlib
import json
import sys
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[2]
AVIATION = ROOT / "docs" / "research" / "aviation"
SCHEMA_PATH = AVIATION / "schemas" / "aero_claim_obligation_evidence_binding_v0_1.schema.json"
EXAMPLE_PATH = AVIATION / "examples" / "aero_claim_obligation_evidence_binding_v0_1.json"
CANONICALIZATION = "rust-serde-json-struct-order-v1"
PROFILE_ID = "aero-scalar-engineering-claim-v0.1"


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def digest_json(value: Any) -> str:
    return digest_bytes(
        json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    )


def semantic_sha256(specification: dict[str, Any]) -> str:
    return digest_json({
        "schema": "AeroClaimSpecificationSemanticCommitmentV1",
        "canonicalization_profile": CANONICALIZATION,
        "specification": specification,
    })


def required_evidence_manifest_sha256(evidence_refs: list[str]) -> str:
    return digest_json({
        "schema": "AeroRequiredEvidenceManifestV1",
        "canonicalization_profile": CANONICALIZATION,
        "evidence_refs": sorted(evidence_refs),
    })


def requirement_profile_sha256() -> str:
    return digest_json({
        "schema": "AeroClaimRequirementProfileCommitmentV1",
        "canonicalization_profile": CANONICALIZATION,
        "profile_id": PROFILE_ID,
        "required_rules": [
            ["claim_identity", "CLAIM_IDENTITY", True, True],
            ["semantic_validity", "SEMANTIC_VALIDITY", True, True],
            ["producer_authority", "PRODUCER_AUTHORITY", True, True],
            ["model_applicability", "MODEL_APPLICABILITY", True, True],
            ["temporal_knowledge", "TEMPORAL_KNOWLEDGE", True, True],
            ["dependency_observation", "DEPENDENCY_OBSERVATION", True, True],
        ],
        "strata_rule": ["MODEL_CREDIBILITY_PER_REQUIRED_STRATUM", True, True],
        "evidence_class_rule": ["EVIDENCE_INTEGRITY_PER_REQUIRED_CLASS", True, True],
        "uncertainty_rule": ["MODEL_CREDIBILITY_FOR_DECLARED_UNCERTAINTY", True, True],
        "defeater_rule": ["DEFEATER_RESOLUTION_FOR_ASSUMPTIONS_AND_EXCLUSIONS", False, False],
    })


def derive_obligations(spec: dict[str, Any]) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []

    def add(
        obligation_id: str,
        kind: str,
        mandatory: bool,
        hard_gate: bool,
        source_ref: str,
        scoped_subject_ref: str,
    ) -> None:
        result.append({
            "obligation_id": obligation_id,
            "kind": kind,
            "mandatory": mandatory,
            "hard_gate": hard_gate,
            "requirement_source_ref": source_ref,
            "scoped_subject_ref": scoped_subject_ref,
        })

    for obligation_id, kind, source in [
        ("claim-identity", "CLAIM_IDENTITY", "claim-spec:/claim_id+subject_ref+configuration_ref"),
        ("semantic-validity", "SEMANTIC_VALIDITY", "claim-spec:/measurand+predicate+population+intended_use_ref"),
        ("producer-authority", "PRODUCER_AUTHORITY", "claim-spec:/acceptance_authority_role_ref"),
        ("model-applicability", "MODEL_APPLICABILITY", "claim-spec:/operating_envelope_ref+intended_use_ref"),
        ("temporal-knowledge", "TEMPORAL_KNOWLEDGE", "claim-spec:/validity"),
        ("dependency-observation", "DEPENDENCY_OBSERVATION", "claim-spec:/subject_ref+configuration_ref+validity"),
    ]:
        add(obligation_id, kind, True, True, source, spec["subject_ref"])

    for evidence_class in spec["required_evidence_classes"]:
        add(
            f"evidence-integrity:{evidence_class}",
            "EVIDENCE_INTEGRITY",
            True,
            True,
            f"claim-spec:/required_evidence_classes/{evidence_class}",
            spec["subject_ref"],
        )

    for stratum in spec["population"]["required_strata"]:
        add(
            f"model-credibility:{stratum['stratum_id']}",
            "MODEL_CREDIBILITY",
            True,
            True,
            f"claim-spec:/population/required_strata/{stratum['stratum_id']}",
            "|".join([
                spec["population"]["population_ref"],
                stratum["dimension_ref"],
                stratum["level_ref"],
            ]),
        )

    add(
        "model-uncertainty",
        "MODEL_CREDIBILITY",
        True,
        True,
        "claim-spec:/uncertainty/treatment+method_ref:" + spec["uncertainty"]["method_ref"],
        spec["subject_ref"],
    )
    for assumption in spec["assumptions"]:
        add(
            f"assumption-defeater:{assumption}",
            "DEFEATER_RESOLUTION",
            False,
            False,
            f"claim-spec:/assumptions/{assumption}",
            spec["subject_ref"],
        )
    for exclusion in spec["exclusions"]:
        add(
            f"exclusion-defeater:{exclusion}",
            "DEFEATER_RESOLUTION",
            False,
            False,
            f"claim-spec:/exclusions/{exclusion}",
            spec["subject_ref"],
        )
    return sorted(result, key=lambda item: item["obligation_id"])


def obligation_manifest_sha256(
    spec: dict[str, Any],
    semantic_sha: str,
    profile_sha: str,
    obligations: list[dict[str, Any]],
) -> str:
    return digest_json({
        "schema": "AeroDerivedClaimObligationManifestV0_1",
        "canonicalization_profile": CANONICALIZATION,
        "claim_id": spec["claim_id"],
        "claim_specification_semantic_sha256": semantic_sha,
        "profile_sha256": profile_sha,
        "obligations": obligations,
    })


def source_receipt_manifest_sha256(receipts: list[dict[str, Any]]) -> str:
    ordered = sorted(
        receipts,
        key=lambda item: (item["evidence_ref"], item["receipt_sha256"]),
    )
    return digest_json({
        "schema": "AeroOwnerEvidenceSourceReceiptManifestV0_1",
        "canonicalization_profile": CANONICALIZATION,
        "receipts": ordered,
    })


def owner_evidence_cut_sha256(cut: dict[str, Any]) -> str:
    record_fields = [
        "evidence_ref", "record_sha256", "subject_ref", "configuration_ref",
        "lifecycle", "predecessor_ref", "freshness", "dependency_coverage",
        "unresolved_inputs", "requires_reverification", "eligible_under_declared_coverage",
    ]
    records = [
        {field: record[field] for field in record_fields}
        for record in sorted(cut["evidence_records"], key=lambda item: item["evidence_ref"])
    ]
    envelope = {
        "schema": "AeroOwnerEvidenceCutV0_1",
        "canonicalization_profile": CANONICALIZATION,
        "owner_namespace": cut["owner_namespace"],
        "component_packet_id": cut["component_packet_id"],
        "physical_unit_ref": cut["physical_unit_ref"],
        "configuration_ref": cut["configuration_ref"],
        "component_packet_sha256": cut["component_packet_sha256"],
        "component_packet_byte_length": cut["component_packet_byte_length"],
        "artifact_payloads_verified": cut["artifact_payloads_verified"],
        "evidence_records": records,
        "basalt_run_id": cut["basalt_run_id"],
        "basalt_report_version": cut["basalt_report_version"],
        "basalt_timeline_input_sha256": cut["basalt_timeline_input_sha256"],
        "basalt_final_state_sha256": cut["basalt_final_state_sha256"],
        "basalt_report_sha256": cut["basalt_report_sha256"],
        "basalt_confirmatory_eligible": cut["basalt_confirmatory_eligible"],
        "receiver_receipt_manifest_sha256": cut["receiver_receipt_manifest_sha256"],
        "receiver_receipt_watermark_unix_ms": cut["receiver_receipt_watermark_unix_ms"],
        "trust_policy_sha256": cut["trust_policy_sha256"],
        "observer_coverage_profile_id": cut["observer_coverage_profile_id"],
        "observer_coverage_receipt_sha256": cut["observer_coverage_receipt_sha256"],
        "observer_coverage_status": cut["observer_coverage_status"],
        "observer_coverage_watermark_unix_ms": cut["observer_coverage_watermark_unix_ms"],
    }
    return digest_json(envelope)


def decision_context_sha256(context: dict[str, Any]) -> str:
    return digest_json({
        "schema": "AeroDecisionContextV0_1",
        "canonicalization_profile": CANONICALIZATION,
        "claim_id": context["claim_id"],
        "subject_ref": context["subject_ref"],
        "configuration_ref": context["configuration_ref"],
        "operating_envelope_ref": context["operating_envelope_ref"],
        "acquisition_time_earliest_unix_ms": context["acquisition_time_earliest_unix_ms"],
        "acquisition_time_latest_unix_ms": context["acquisition_time_latest_unix_ms"],
        "as_known_at_unix_ms": context["as_known_at_unix_ms"],
        "source_scope_manifest_sha256": context["source_scope_manifest_sha256"],
        "evidence_knowledge_cut_sha256": context["evidence_knowledge_cut_sha256"],
        "trust_policy_sha256": context["trust_policy_sha256"],
        "claim_specification_sha256": context["claim_specification_sha256"],
        "required_evidence_manifest_sha256": context["required_evidence_manifest_sha256"],
        "risk_classification_sha256": context["risk_classification_sha256"],
        "model_applicability_profile_sha256": context["model_applicability_profile_sha256"],
    })


def mapping_manifest_sha256(source: dict[str, Any], profile_sha: str, obligation_sha: str) -> str:
    entries = []
    for entry in sorted(source["mapping_entries"], key=lambda item: item["obligation_id"]):
        entries.append({
            "obligation_id": entry["obligation_id"],
            "requirement_source_ref": entry["requirement_source_ref"],
            "evidence_class": entry["evidence_class"],
            "evidence_refs": sorted(entry["evidence_refs"]),
        })
    return digest_json({
        "schema": "AeroClaimObligationEvidenceMappingManifestV0_1",
        "canonicalization_profile": CANONICALIZATION,
        "claim_id": source["claim_specification"]["claim_id"],
        "decision_context_sha256": source["decision_context_sha256"],
        "claim_specification_semantic_sha256": source["decision_context"]["claim_specification_sha256"],
        "obligation_profile_sha256": profile_sha,
        "obligation_manifest_sha256": obligation_sha,
        "required_evidence_manifest_sha256": source["decision_context"]["required_evidence_manifest_sha256"],
        "owner_evidence_cut_sha256": source["decision_context"]["evidence_knowledge_cut_sha256"],
        "trust_policy_sha256": source["decision_context"]["trust_policy_sha256"],
        "entries": entries,
    })


def validate(source: dict[str, Any]) -> tuple[list[str], str]:
    errors: list[str] = []
    spec = source["claim_specification"]
    context = source["decision_context"]
    cut = source["owner_evidence_cut"]
    receipt = source["mapping_receipt"]
    cutoff = context["as_known_at_unix_ms"]

    raw = source["raw_claim_specification_payload"]
    try:
        parsed_raw = json.loads(raw)
    except json.JSONDecodeError:
        return ["raw claim payload is not valid JSON"], "REJECTED"
    if digest_bytes(raw.encode("utf-8")) != source["expected_raw_claim_specification_sha256"]:
        errors.append("raw claim payload digest mismatch")
    if parsed_raw != spec:
        errors.append("raw payload and typed claim specification differ")

    semantic_sha = semantic_sha256(spec)
    if context["claim_specification_sha256"] != semantic_sha:
        errors.append("typed claim semantic digest mismatch")

    classes = spec["required_evidence_classes"]
    required_refs = [item["evidence_ref"] for item in source["required_evidence_admissions"]]
    required_sha = required_evidence_manifest_sha256(required_refs)
    if context["required_evidence_manifest_sha256"] != required_sha:
        errors.append("required-evidence manifest digest mismatch")
    if len(required_refs) != len(set(required_refs)):
        errors.append("required evidence admission references are duplicated")

    if (
        context["claim_id"] != spec["claim_id"]
        or context["subject_ref"] != spec["subject_ref"]
        or context["configuration_ref"] != spec["configuration_ref"]
        or context["operating_envelope_ref"] != spec["operating_envelope_ref"]
    ):
        errors.append("claim/subject/configuration/envelope scope mismatch")
    if context["acquisition_time_earliest_unix_ms"] > context["acquisition_time_latest_unix_ms"]:
        errors.append("acquisition interval is reversed")
    if context["acquisition_time_latest_unix_ms"] > cutoff:
        errors.append("acquisition interval extends after as-known cutoff")
    if not (
        spec["validity"]["effective_from_unix_ms"]
        <= context["acquisition_time_earliest_unix_ms"]
        <= context["acquisition_time_latest_unix_ms"]
        < spec["validity"]["effective_until_unix_ms"]
    ):
        errors.append("acquisition interval is outside claim validity interval")

    context_sha = decision_context_sha256(context)
    if context_sha != source["decision_context_sha256"]:
        errors.append("decision-context digest mismatch")

    profile_sha = requirement_profile_sha256()
    obligations = derive_obligations(spec)
    obligation_sha = obligation_manifest_sha256(spec, semantic_sha, profile_sha, obligations)
    declared_profile = source["derived_profile_commitments"]
    if declared_profile["profile_id"] != PROFILE_ID or declared_profile["profile_sha256"] != profile_sha:
        errors.append("registered requirement-profile commitment mismatch")
    if declared_profile["obligation_manifest_sha256"] != obligation_sha:
        errors.append("derived obligation manifest digest mismatch")

    evidence_obligations = {
        item["obligation_id"]: item
        for item in obligations if item["kind"] == "EVIDENCE_INTEGRITY"
    }
    expected_ids = set(evidence_obligations)
    mapped_ids = [item["obligation_id"] for item in source["mapping_entries"]]
    if len(mapped_ids) != len(set(mapped_ids)):
        errors.append("evidence-class obligation mappings are duplicated")
    if set(mapped_ids) != expected_ids:
        errors.append("mapping does not cover the exact derived evidence-class obligation set")

    mapped_refs: list[str] = []
    for entry in source["mapping_entries"]:
        obligation = evidence_obligations.get(entry["obligation_id"])
        if obligation is None:
            errors.append(f"mapping refers to an obligation not derived by profile: {entry['obligation_id']}")
            continue
        expected_class = entry["obligation_id"].removeprefix("evidence-integrity:")
        if (
            entry["evidence_class"] != expected_class
            or entry["requirement_source_ref"] != obligation["requirement_source_ref"]
            or entry["requirement_source_ref"] != f"claim-spec:/required_evidence_classes/{expected_class}"
        ):
            errors.append(f"evidence class or requirement source mismatch: {entry['obligation_id']}")
        if not entry["evidence_refs"]:
            errors.append(f"mapping has no evidence references: {entry['obligation_id']}")
        mapped_refs.extend(entry["evidence_refs"])
    if len(mapped_refs) != len(set(mapped_refs)):
        errors.append("one source record is mapped more than once or duplicated")
    if set(mapped_refs) != set(required_refs):
        errors.append("mapping references differ from the complete required evidence set")

    source_refs = [item["evidence_ref"] for item in source["source_receipts"]]
    if len(source_refs) != len(set(source_refs)):
        errors.append("source receipts are duplicated")
    if set(source_refs) != set(mapped_refs):
        errors.append("source receipt set differs from mapped evidence references")

    source_manifest_sha = source_receipt_manifest_sha256(source["source_receipts"])
    if cut["receiver_receipt_manifest_sha256"] != source_manifest_sha:
        errors.append("owner-cut receiver receipt manifest does not bind the exact source receipt set")
    if cut["receiver_receipt_watermark_unix_ms"] > cutoff:
        errors.append("owner evidence cut watermark is after the selected historical cutoff")
    owner_cut_sha = owner_evidence_cut_sha256(cut)
    if context["evidence_knowledge_cut_sha256"] != owner_cut_sha:
        errors.append("owner evidence cut digest mismatch")
    if cut["owner_namespace"] == "" or cut["trust_policy_sha256"] != context["trust_policy_sha256"]:
        errors.append("owner namespace or trust-policy binding mismatch")
    if (
        cut["physical_unit_ref"] != context["subject_ref"]
        or cut["configuration_ref"] != context["configuration_ref"]
    ):
        errors.append("owner evidence cut subject/configuration mismatch")

    owner_records = {item["evidence_ref"]: item for item in cut["evidence_records"]}
    if len(owner_records) != len(cut["evidence_records"]):
        errors.append("owner evidence cut contains duplicate evidence IDs")

    admissions = {item["evidence_ref"]: item for item in source["required_evidence_admissions"]}
    if set(admissions) != set(required_refs):
        errors.append("required evidence admission inventory mismatch")

    incomplete = False
    unverified = False
    for source_receipt in source["source_receipts"]:
        ref = source_receipt["evidence_ref"]
        record = owner_records.get(ref)
        admission = admissions.get(ref)
        if record is None:
            errors.append(f"source receipt has no owner-cut record: {ref}")
            continue
        if admission is None:
            errors.append(f"source receipt has no evidence admission: {ref}")
            continue
        if (
            source_receipt["owner_namespace"] != cut["owner_namespace"]
            or source_receipt["claim_id"] != spec["claim_id"]
            or source_receipt["subject_ref"] != spec["subject_ref"]
            or source_receipt["configuration_ref"] != spec["configuration_ref"]
            or source_receipt["trust_policy_sha256"] != cut["trust_policy_sha256"]
        ):
            errors.append(f"source receipt scope mismatch: {ref}")
        if (
            source_receipt["acquisition_time_earliest_unix_ms"] != context["acquisition_time_earliest_unix_ms"]
            or source_receipt["acquisition_time_latest_unix_ms"] != context["acquisition_time_latest_unix_ms"]
        ):
            errors.append(f"source receipt acquisition interval mismatch: {ref}")
        if source_receipt["evidence_record_sha256"] != record["record_sha256"]:
            errors.append(f"source receipt record digest mismatch: {ref}")
        if source_receipt["receiver_first_seen_at_unix_ms"] > cutoff:
            unverified = True
            errors.append(f"source receipt was not known at cutoff: {ref}")
        if source_receipt["receiver_first_seen_at_unix_ms"] > cut["receiver_receipt_watermark_unix_ms"]:
            errors.append(f"source receipt arrived after owner-cut watermark: {ref}")
        if not (
            source_receipt["evidence_effective_from_unix_ms"]
            <= context["acquisition_time_earliest_unix_ms"]
            <= context["acquisition_time_latest_unix_ms"]
            < source_receipt["evidence_effective_until_unix_ms"]
        ):
            incomplete = True
            errors.append(f"source evidence does not cover the acquisition interval: {ref}")
        if not (
            source_receipt["authority_effective_from_unix_ms"]
            <= cutoff
            < source_receipt["authority_effective_until_unix_ms"]
        ):
            errors.append(f"producer authority not effective at cutoff: {ref}")
        if source_receipt["verification_status"] == "FAILED" or source_receipt["integrity_state"] == "FAILED":
            errors.append(f"source receipt or evidence integrity failed: {ref}")
        elif source_receipt["verification_status"] == "UNVERIFIED" or source_receipt["integrity_state"] == "UNVERIFIED":
            unverified = True
            errors.append(f"source receipt or integrity is unverified: {ref}")
        if source_receipt["producer_authority_state"] == "DENIED":
            errors.append(f"producer authority denied: {ref}")
        elif source_receipt["producer_authority_state"] == "UNKNOWN":
            unverified = True
            errors.append(f"producer authority unknown: {ref}")
        if admission["decision_context_sha256"] != context_sha:
            errors.append(f"admission context mismatch: {ref}")
        if admission["state"] != "ADMITTED":
            incomplete = True
        if record["lifecycle"] != "Active":
            if record["lifecycle"] == "Disputed":
                unverified = True
            elif record["lifecycle"] in ("Revoked", "Withdrawn"):
                errors.append(f"owner evidence record revoked/withdrawn: {ref}")
            else:
                incomplete = True
        if (
            record["freshness"] != "fresh"
            or record["dependency_coverage"] != "complete"
            or record["unresolved_inputs"]
            or record["requires_reverification"]
            or not record["eligible_under_declared_coverage"]
        ):
            incomplete = True

    mapping_sha = mapping_manifest_sha256(source, profile_sha, obligation_sha)
    if receipt["mapping_manifest_sha256"] != mapping_sha:
        errors.append("mapping approval receipt manifest digest mismatch")
    if (
        receipt["verification_status"] != "VERIFIED"
        or receipt["owner_namespace"] != cut["owner_namespace"]
        or receipt["claim_id"] != spec["claim_id"]
        or receipt["configuration_ref"] != spec["configuration_ref"]
        or receipt["decision_context_sha256"] != context_sha
        or receipt["claim_specification_semantic_sha256"] != semantic_sha
        or receipt["obligation_profile_sha256"] != profile_sha
        or receipt["obligation_manifest_sha256"] != obligation_sha
        or receipt["required_evidence_manifest_sha256"] != required_sha
        or receipt["owner_evidence_cut_sha256"] != owner_cut_sha
        or receipt["trust_policy_sha256"] != cut["trust_policy_sha256"]
    ):
        errors.append("mapping approval scope or upstream verification mismatch")
    if receipt["receiver_first_seen_at_unix_ms"] > cutoff:
        unverified = True
        errors.append("mapping approval was not known at cutoff")
    if not receipt["effective_from_unix_ms"] <= cutoff < receipt["effective_until_unix_ms"]:
        incomplete = True
        errors.append("mapping approval not effective at cutoff")

    if any(error for error in errors if "digest mismatch" in error or "scope mismatch" in error or "denied" in error or "failed" in error or "duplicate" in error or "revoked" in error or "after owner-cut watermark" in error):
        disposition = "REJECTED"
    elif unverified:
        disposition = "UNVERIFIED"
    elif incomplete or errors:
        disposition = "INCOMPLETE"
    else:
        disposition = "BOUND"
    return errors, disposition


def main() -> int:
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    instance = json.loads(EXAMPLE_PATH.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    schema_validator = Draft202012Validator(schema, format_checker=FormatChecker())
    schema_errors = sorted(
        schema_validator.iter_errors(instance),
        key=lambda error: (list(map(str, error.absolute_path)), error.message),
    )
    if schema_errors:
        for error in schema_errors:
            print(f"SCHEMA FAIL /{'/'.join(map(str, error.absolute_path))}: {error.message}")
        return 1

    errors, disposition = validate(instance)
    if errors or disposition != "BOUND":
        for error in errors:
            print(f"CONTRACT FAIL: {error}")
        raise SystemExit(f"FAIL: synthetic evidence-class mapping disposition is {disposition}")

    # 10 independent mutation / permutation controls.
    missing_class = copy.deepcopy(instance)
    missing_class["mapping_entries"].pop()
    missing_errors, missing_disposition = validate(missing_class)
    if not missing_errors or missing_disposition == "BOUND":
        raise SystemExit("FAIL: missing derived evidence-class obligation mapping was accepted")

    wrong_class = copy.deepcopy(instance)
    wrong_class["mapping_entries"][0]["requirement_source_ref"] = "claim-spec:/required_evidence_classes/unrelated"
    wrong_errors, _ = validate(wrong_class)
    if not any("evidence class or requirement source mismatch" in error for error in wrong_errors):
        raise SystemExit("FAIL: mapping from another requirement source was accepted")

    late_mapping = copy.deepcopy(instance)
    late_mapping["mapping_receipt"]["receiver_first_seen_at_unix_ms"] = (
        late_mapping["decision_context"]["as_known_at_unix_ms"] + 1
    )
    late_errors, late_disposition = validate(late_mapping)
    if late_disposition != "UNVERIFIED" or not any("mapping approval was not known" in error for error in late_errors):
        raise SystemExit("FAIL: late mapping receipt backfilled historical knowledge")

    stale_manifest = copy.deepcopy(instance)
    stale_manifest["mapping_entries"][0]["evidence_refs"][0] = "evidence:substituted-record"
    stale_errors, _ = validate(stale_manifest)
    if not any("mapping approval receipt manifest digest mismatch" in error for error in stale_errors):
        raise SystemExit("FAIL: changed evidence mapping retained old approval commitment")

    wrong_owner = copy.deepcopy(instance)
    wrong_owner["source_receipts"][0]["owner_namespace"] = "owner:unrelated-source"
    owner_errors, _ = validate(wrong_owner)
    if not any("source receipt scope mismatch" in error for error in owner_errors):
        raise SystemExit("FAIL: receipt from another owner was accepted")

    changed_record = copy.deepcopy(instance)
    changed_record["source_receipts"][0]["evidence_record_sha256"] = "9" * 64
    record_errors, _ = validate(changed_record)
    if not any("source receipt record digest mismatch" in error for error in record_errors):
        raise SystemExit("FAIL: source record digest substitution was accepted")

    late_source = copy.deepcopy(instance)
    late_source["source_receipts"][0]["receiver_first_seen_at_unix_ms"] = (
        late_source["decision_context"]["as_known_at_unix_ms"] + 1
    )
    late_source_errors, late_source_disposition = validate(late_source)
    if late_source_disposition != "REJECTED" or not any("not known at cutoff" in error for error in late_source_errors):
        raise SystemExit("FAIL: late source receipt appended to an older owner cut was accepted")

    duplicate_receipt = copy.deepcopy(instance)
    duplicate_receipt["source_receipts"].append(copy.deepcopy(duplicate_receipt["source_receipts"][0]))
    duplicate_errors, _ = validate(duplicate_receipt)
    if not any("source receipts are duplicated" in error for error in duplicate_errors):
        raise SystemExit("FAIL: duplicate owner source receipt was accepted")

    not_received = copy.deepcopy(instance)
    not_received["required_evidence_admissions"][0]["state"] = "NOT_RECEIVED"
    _, not_received_disposition = validate(not_received)
    if not_received_disposition != "INCOMPLETE":
        raise SystemExit("FAIL: missing evidence admission was not kept incomplete")

    permuted = copy.deepcopy(instance)
    permuted["mapping_entries"].reverse()
    permuted["source_receipts"].reverse()
    permuted["owner_evidence_cut"]["evidence_records"].reverse()
    permuted["required_evidence_admissions"].reverse()
    permuted_errors, permuted_disposition = validate(permuted)
    if permuted_errors or permuted_disposition != "BOUND":
        raise SystemExit("FAIL: semantically equivalent input permutation changed the result")

    print(
        "PASS: AERO obligation-to-evidence binding v0.1 schema, raw/semantic/context/owner-cut/"
        "source-receipt/profile/mapping digest chain, exact evidence-class coverage and "
        "10 mutation/permutation controls."
    )
    print(
        "NOTE: synthetic receipts are placeholders and VERIFIED is an upstream disposition. "
        "This oracle does not authenticate signatures, source truth, model/observer obligations, "
        "engineering acceptance or physical qualification."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
