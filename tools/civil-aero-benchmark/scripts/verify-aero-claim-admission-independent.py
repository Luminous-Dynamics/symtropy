#!/usr/bin/env python3
"""Independent verifier for claim-admission request, evidence manifest and decision receipts."""
from __future__ import annotations

import csv
import hashlib
import struct
import sys
from pathlib import Path


SCHEMA = "aero-claim-admission/0.2.0"
EVIDENCE_LEVEL = (
    "claim_specific_policy_evaluation_with_exact_scope_and_full_evidence_manifest_digest_"
    "binding_not_artifact_verification_not_regulatory_approval_or_flight_authorization"
)
MANIFEST_SCHEMA = "aero-claim-evidence-manifest/0.1.0"
KINDS = [
    "geometry_identity",
    "analysis_inputs_identity",
    "solver_executable_identity",
    "process_receipt",
    "convergence_screen",
    "comparison_protocol",
    "grid_convergence",
    "temporal_convergence",
    "time_step_sensitivity",
    "conservation_audit",
    "statistical_sampling_assessment",
    "model_uncertainty_assessment",
    "experimental_validation",
    "multivariate_validation",
    "validation_uncertainty_budget",
    "structural_verification",
    "structural_validation",
    "aeroelastic_coupling_convergence",
    "flutter_validation",
    "propulsion_validation",
    "mission_model_verification",
    "mission_uncertainty_budget",
    "operational_envelope_validation",
    "calibration_evidence",
    "independent_review",
    "certification_basis_evidence",
    "certification_authority_decision_record",
    "quality_system_evidence",
}
KIND_ORDER = {kind: index for index, kind in enumerate(KINDS)}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read_rows(path):
    with path.open(newline="", encoding="utf-8") as stream:
        return list(csv.DictReader(stream))


def add_string(buffer, value):
    raw = value.encode("utf-8")
    buffer.extend(struct.pack("<Q", len(raw)))
    buffer.extend(raw)


def add_kinds(buffer, kinds):
    buffer.extend(struct.pack("<Q", len(kinds)))
    for kind in kinds:
        add_string(buffer, kind)


def is_digest(value):
    return (
        len(value) == 64
        and all(character in "0123456789abcdefABCDEF" for character in value)
    )


def normalized_digest(value, label):
    require(is_digest(value), f"{label} is not a 64-character SHA-256 digest")
    require(value == value.lower(), f"{label} must use lowercase hexadecimal encoding")
    return value


def split_kinds(value):
    return [] if not value else value.split(";")


def required_for(claim_class, consequence):
    e = set()
    e.update({
        "geometry_identity",
        "analysis_inputs_identity",
        "solver_executable_identity",
        "process_receipt",
        "convergence_screen",
    })
    if claim_class == "exploratory_design_ranking":
        e.add("comparison_protocol")
    elif claim_class == "aerodynamic_prediction":
        e.update({
            "grid_convergence",
            "conservation_audit",
            "model_uncertainty_assessment",
            "experimental_validation",
            "validation_uncertainty_budget",
        })
    elif claim_class == "unsteady_aerodynamic_prediction":
        e.update({
            "grid_convergence",
            "temporal_convergence",
            "time_step_sensitivity",
            "conservation_audit",
            "statistical_sampling_assessment",
            "model_uncertainty_assessment",
            "experimental_validation",
            "multivariate_validation",
            "validation_uncertainty_budget",
        })
    elif claim_class == "structural_load_prediction":
        e.update({
            "grid_convergence",
            "conservation_audit",
            "structural_verification",
            "structural_validation",
            "model_uncertainty_assessment",
            "validation_uncertainty_budget",
        })
    elif claim_class == "aeroelastic_stability_prediction":
        e.update({
            "grid_convergence",
            "temporal_convergence",
            "time_step_sensitivity",
            "conservation_audit",
            "structural_verification",
            "structural_validation",
            "aeroelastic_coupling_convergence",
            "flutter_validation",
            "multivariate_validation",
            "model_uncertainty_assessment",
            "validation_uncertainty_budget",
        })
    elif claim_class == "mission_performance_prediction":
        e.update({
            "grid_convergence",
            "conservation_audit",
            "experimental_validation",
            "structural_verification",
            "structural_validation",
            "propulsion_validation",
            "mission_model_verification",
            "mission_uncertainty_budget",
            "model_uncertainty_assessment",
            "validation_uncertainty_budget",
        })
    elif claim_class == "flight_release":
        e.update({
            "grid_convergence",
            "temporal_convergence",
            "time_step_sensitivity",
            "conservation_audit",
            "statistical_sampling_assessment",
            "experimental_validation",
            "multivariate_validation",
            "validation_uncertainty_budget",
            "structural_verification",
            "structural_validation",
            "aeroelastic_coupling_convergence",
            "flutter_validation",
            "propulsion_validation",
            "mission_model_verification",
            "mission_uncertainty_budget",
            "operational_envelope_validation",
            "calibration_evidence",
            "independent_review",
            "certification_basis_evidence",
            "quality_system_evidence",
            "model_uncertainty_assessment",
        })
    else:
        raise ValueError(f"unsupported claim class {claim_class!r}")

    if consequence in {"high", "safety_critical"}:
        e.update({
            "independent_review",
            "calibration_evidence",
            "model_uncertainty_assessment",
        })
    if consequence == "safety_critical":
        e.add("multivariate_validation")
    if consequence not in {"exploratory", "engineering", "high", "safety_critical"}:
        raise ValueError(f"unsupported decision consequence {consequence!r}")
    return sorted(e, key=KIND_ORDER.__getitem__)


def evidence_manifest_digest(evidence_rows):
    seen = set()
    ordered = []
    for row in evidence_rows:
        kind = row["kind"]
        require(kind in KIND_ORDER, f"unknown evidence kind {kind!r}")
        require(kind not in seen, f"duplicate evidence kind {kind!r}")
        seen.add(kind)
        ordered.append(row)
    ordered.sort(key=lambda row: KIND_ORDER[row["kind"]])

    canonical = bytearray()
    add_string(canonical, MANIFEST_SCHEMA)
    canonical.extend(struct.pack("<Q", len(ordered)))
    for row in ordered:
        add_string(canonical, row["kind"])
        add_string(canonical, normalized_digest(row["scope_sha256"], "evidence scope"))
        status = row["assertion_status"]
        if status == "verified":
            canonical.append(1)
            add_string(canonical, normalized_digest(row["artifact_sha256"], "artifact"))
            add_string(canonical, row["verifier_id"])
            add_string(canonical, normalized_digest(row["verifier_sha256"], "verifier"))
            add_string(
                canonical,
                normalized_digest(row["verifier_receipt_sha256"], "verifier receipt"),
            )
            require(row["verifier_id"].strip(), "verified assertion has empty verifier ID")
            require(not row["reason"], "verified assertion must not have an unknown/failed reason")
        elif status == "unknown":
            canonical.append(2)
            require(not row["artifact_sha256"] and not row["verifier_id"]
                    and not row["verifier_sha256"] and not row["verifier_receipt_sha256"],
                    "unknown assertion must not carry verifier/artifact fields")
            require(row["reason"].strip(), "unknown assertion requires a reason")
            add_string(canonical, row["reason"])
        elif status == "failed":
            canonical.append(3)
            require(not row["artifact_sha256"] and not row["verifier_id"]
                    and not row["verifier_sha256"] and not row["verifier_receipt_sha256"],
                    "failed assertion must not carry verifier/artifact fields")
            require(row["reason"].strip(), "failed assertion requires a reason")
            add_string(canonical, row["reason"])
        else:
            raise ValueError(f"unsupported assertion status {status!r}")
    return hashlib.sha256(canonical).hexdigest(), ordered


def decision_status(request, required, evidence_by_kind):
    failed = set()
    verified = set()
    unknown = set()
    missing = set()

    for kind, row in evidence_by_kind.items():
        if row["scope_sha256"].lower() != request["scope_sha256"].lower():
            failed.add(kind)

    for kind in required:
        if kind in failed:
            continue
        row = evidence_by_kind.get(kind)
        if row is None:
            missing.add(kind)
        elif row["assertion_status"] == "verified":
            verified.add(kind)
        elif row["assertion_status"] == "unknown":
            unknown.add(kind)
        elif row["assertion_status"] == "failed":
            failed.add(kind)
        else:
            raise ValueError(f"unsupported assertion status for {kind}")

    ordered = lambda values: sorted(values, key=KIND_ORDER.__getitem__)
    if failed:
        status = "rejected_failed_or_mismatched_evidence"
    elif missing or unknown:
        status = "abstain_insufficient_or_unknown_evidence"
    elif request["claim_class"] == "flight_release" or request["consequence"] == "safety_critical":
        status = "requires_human_authority_decision"
    else:
        status = "admitted_within_declared_scope"
    return status, ordered(verified), ordered(unknown), ordered(missing), ordered(failed)


def main():
    if len(sys.argv) != 4:
        raise ValueError(
            "usage: verify-aero-claim-admission-independent.py REQUEST.csv EVIDENCE.csv DECISION.csv"
        )
    request_rows = read_rows(Path(sys.argv[1]))
    evidence_rows = read_rows(Path(sys.argv[2]))
    decision_rows = read_rows(Path(sys.argv[3]))
    require(len(request_rows) == 1, "expected exactly one claim request")
    require(len(decision_rows) == 1, "expected exactly one claim decision")
    request = request_rows[0]
    decision = decision_rows[0]

    require(decision["schema"] == SCHEMA, "claim decision schema mismatch")
    require(decision["evidence_level"] == EVIDENCE_LEVEL, "claim decision evidence ceiling mismatch")
    require(request["claim_id"] == decision["claim_id"], "claim ID mismatch")
    require(request["claim_class"] == decision["claim_class"], "claim class mismatch")
    require(request["consequence"] == decision["consequence"], "decision consequence mismatch")
    scope = normalized_digest(request["scope_sha256"], "claim scope")
    require(decision["scope_sha256"] == scope, "claim scope digest mismatch")

    required = required_for(request["claim_class"], request["consequence"])
    policy = bytearray()
    add_string(policy, SCHEMA)
    add_string(policy, request["claim_class"])
    add_string(policy, request["consequence"])
    add_kinds(policy, required)
    policy_digest = hashlib.sha256(policy).hexdigest()
    require(decision["policy_sha256"] == policy_digest, "claim policy digest mismatch")
    require(split_kinds(decision["required_evidence"]) == required,
            "claim required-evidence set mismatch")

    manifest_digest, ordered_rows = evidence_manifest_digest(evidence_rows)
    require(decision["evidence_manifest_sha256"] == manifest_digest,
            "claim evidence-manifest digest mismatch")
    evidence_by_kind = {row["kind"]: row for row in ordered_rows}
    status, verified, unknown, missing, failed = decision_status(
        request, required, evidence_by_kind
    )
    require(decision["status"] == status, "claim decision status mismatch")
    require(split_kinds(decision["verified_evidence"]) == verified,
            "verified-evidence set mismatch")
    require(split_kinds(decision["unknown_evidence"]) == unknown,
            "unknown-evidence set mismatch")
    require(split_kinds(decision["missing_evidence"]) == missing,
            "missing-evidence set mismatch")
    require(split_kinds(decision["failed_evidence"]) == failed,
            "failed-evidence set mismatch")

    canonical = bytearray()
    for value in (
        SCHEMA,
        request["claim_id"],
        request["claim_class"],
        request["consequence"],
        scope,
        policy_digest,
        manifest_digest,
    ):
        add_string(canonical, value)
    add_kinds(canonical, required)
    add_kinds(canonical, verified)
    add_kinds(canonical, unknown)
    add_kinds(canonical, missing)
    add_kinds(canonical, failed)
    add_string(canonical, status)
    add_string(canonical, EVIDENCE_LEVEL)
    digest = hashlib.sha256(canonical).hexdigest()
    require(decision["decision_sha256"] == digest, "claim decision canonical digest mismatch")

    print(
        "independent_aero_claim_admission_oracle=pass "
        f"claim={request['claim_class']} consequence={request['consequence']} "
        f"required={len(required)} supplied={len(ordered_rows)} status={status} "
        f"evidence_manifest_sha256={manifest_digest} decision_sha256={digest}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"independent aerospace claim-admission oracle failed: {error}", file=sys.stderr)
        raise SystemExit(1)
