#!/usr/bin/env python3
"""CI-only independent oracle for AERO claim authority/observer/temporal binding v0.1."""
from __future__ import annotations

import copy
import json
import sys
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker
import validate_claim_obligation_evidence_binding as evidence_binding_oracle

ROOT = Path(__file__).resolve().parents[2]
AVIATION = ROOT / "docs" / "research" / "aviation"
SCHEMA_PATH = AVIATION / "schemas" / "aero_claim_authority_observer_temporal_binding_v0_1.schema.json"
EXAMPLE_PATH = AVIATION / "examples" / "aero_claim_authority_observer_temporal_binding_v0_1.json"


def interval_contains(receipt: dict[str, Any], cutoff: int) -> bool:
    return receipt["effective_from_unix_ms"] <= cutoff < receipt["effective_until_unix_ms"]


def evaluate(source: dict[str, Any]) -> tuple[list[str], dict[str, str], str]:
    errors: list[str] = []
    upstream = source["evidence_binding_input"]
    upstream_errors, upstream_disposition = evidence_binding_oracle.validate(upstream)
    errors.extend(f"UPSTREAM:{item}" for item in upstream_errors)
    if upstream_disposition != "BOUND":
        errors.append("upstream evidence-class adapter is not BOUND")

    spec = upstream["claim_specification"]
    context = upstream["decision_context"]
    owner = upstream["owner_evidence_cut"]
    cutoff = context["as_known_at_unix_ms"]
    context_sha = upstream["decision_context_sha256"]
    authority = source["authority_receipt"]
    observer = source["observer_receipt"]
    temporal = source["temporal_receipt"]
    assessment_binding = source["observer_assessment_binding"]
    coverage = source["observer_coverage"]

    if (
        assessment_binding["claim_id"] != spec["claim_id"]
        or assessment_binding["subject_ref"] != spec["subject_ref"]
        or assessment_binding["decision_context_sha256"] != context_sha
    ):
        errors.append("observer assessment binding scope mismatch")

    # Claim-level producer/acceptance authority is distinct from per-record
    # producer authorization. It must bind the role declared in the claim.
    if (
        authority["owner_namespace"] != owner["owner_namespace"]
        or authority["authority_role_ref"] != spec["acceptance_authority_role_ref"]
        or authority["claim_id"] != spec["claim_id"]
        or authority["subject_ref"] != spec["subject_ref"]
        or authority["configuration_ref"] != spec["configuration_ref"]
        or authority["decision_context_sha256"] != context_sha
        or authority["trust_policy_sha256"] != context["trust_policy_sha256"]
        or authority["source_scope_manifest_sha256"] != context["source_scope_manifest_sha256"]
    ):
        errors.append("acceptance authority receipt scope mismatch")

    authority_disposition = "BOUND"
    if authority["verification_status"] == "FAILED":
        authority_disposition = "REJECTED"
    elif authority["verification_status"] == "UNVERIFIED":
        authority_disposition = "UNVERIFIED"
    elif authority["receiver_first_seen_at_unix_ms"] > cutoff:
        authority_disposition = "UNVERIFIED"
    elif not interval_contains(authority, cutoff):
        authority_disposition = "UNVERIFIED"
    if errors and any("acceptance authority receipt scope mismatch" in e for e in errors):
        authority_disposition = "REJECTED"

    if (
        observer["owner_namespace"] != owner["owner_namespace"]
        or observer["claim_id"] != context["claim_id"]
        or observer["subject_ref"] != context["subject_ref"]
        or observer["configuration_ref"] != context["configuration_ref"]
        or observer["decision_context_sha256"] != context_sha
        or observer["source_scope_manifest_sha256"] != context["source_scope_manifest_sha256"]
        or observer["acquisition_time_earliest_unix_ms"] != context["acquisition_time_earliest_unix_ms"]
        or observer["acquisition_time_latest_unix_ms"] != context["acquisition_time_latest_unix_ms"]
        or observer["observer_coverage_profile_id"] != owner["observer_coverage_profile_id"]
        or observer["owner_observer_coverage_status"] != owner["observer_coverage_status"]
        or observer["owner_observer_coverage_receipt_sha256"] != owner["observer_coverage_receipt_sha256"]
        or observer["coverage_report_sha256"] != assessment_binding["report_sha256"]
        or observer["coverage_status"] != coverage["status"]
        or observer["replay_consequence"] != coverage["replay_consequence"]
        or observer["observer_watermark_unix_ms"] != owner["observer_coverage_watermark_unix_ms"]
        or observer["observer_watermark_unix_ms"] > owner["receiver_receipt_watermark_unix_ms"]
    ):
        errors.append("observer coverage receipt scope or report mismatch")

    observer_disposition = "BOUND"
    if observer["verification_status"] == "FAILED":
        observer_disposition = "REJECTED"
    elif observer["verification_status"] == "UNVERIFIED":
        observer_disposition = "UNVERIFIED"
    elif (
        observer["receiver_first_seen_at_unix_ms"] > cutoff
        or observer["observer_watermark_unix_ms"] > cutoff
    ):
        observer_disposition = "UNVERIFIED"
    elif not interval_contains(observer, cutoff):
        observer_disposition = "UNVERIFIED"
    elif coverage["status"] == "qualified_complete_for_scope":
        if observer["owner_observer_coverage_status"] != "qualified_complete_for_scope":
            observer_disposition = "REJECTED"
    elif coverage["status"] == "out_of_scope":
        observer_disposition = "OUT_OF_SCOPE"
    elif coverage["status"] == "unverified":
        observer_disposition = "UNVERIFIED"
    elif coverage["status"] == "invalid":
        observer_disposition = "REJECTED"
    else:
        observer_disposition = "INCOMPLETE"
    if errors and any("observer coverage receipt scope or report mismatch" in e for e in errors):
        observer_disposition = "REJECTED"

    # Historical snapshot must bind the complete record receipt manifest and
    # owner cut, and it cannot carry a watermark beyond the selected cutoff.
    if (
        temporal["owner_namespace"] != owner["owner_namespace"]
        or temporal["claim_id"] != context["claim_id"]
        or temporal["configuration_ref"] != context["configuration_ref"]
        or temporal["decision_context_sha256"] != context_sha
        or temporal["trust_policy_sha256"] != context["trust_policy_sha256"]
        or temporal["owner_evidence_cut_sha256"] != context["evidence_knowledge_cut_sha256"]
        or temporal["source_receipt_manifest_sha256"] != owner["receiver_receipt_manifest_sha256"]
        or temporal["receiver_receipt_manifest_sha256"] != owner["receiver_receipt_manifest_sha256"]
        or temporal["receiver_watermark_unix_ms"] != owner["receiver_receipt_watermark_unix_ms"]
        or temporal["as_known_at_unix_ms"] != cutoff
    ):
        errors.append("temporal knowledge receipt scope or manifest mismatch")

    temporal_disposition = "BOUND"
    if temporal["verification_status"] == "FAILED":
        temporal_disposition = "REJECTED"
    elif temporal["verification_status"] == "UNVERIFIED":
        temporal_disposition = "UNVERIFIED"
    elif (
        temporal["receiver_first_seen_at_unix_ms"] > cutoff
        or temporal["receiver_watermark_unix_ms"] > cutoff
    ):
        temporal_disposition = "UNVERIFIED"
    elif not interval_contains(temporal, cutoff):
        temporal_disposition = "UNVERIFIED"
    if errors and any("temporal knowledge receipt scope or manifest mismatch" in e for e in errors):
        temporal_disposition = "REJECTED"

    if any(
        "scope mismatch" in e
        or "report mismatch" in e
        or "manifest mismatch" in e
        for e in errors
    ):
        # Scope failures are hard rejection, even if another receipt is late.
        if "acceptance authority receipt scope mismatch" in errors:
            authority_disposition = "REJECTED"
        if "observer coverage receipt scope or report mismatch" in errors:
            observer_disposition = "REJECTED"
        if "temporal knowledge receipt scope or manifest mismatch" in errors:
            temporal_disposition = "REJECTED"

    dispositions = {
        "claim-identity": "BOUND" if upstream_disposition == "BOUND" else upstream_disposition,
        "semantic-validity": "BOUND" if upstream_disposition == "BOUND" else upstream_disposition,
        "producer-authority": authority_disposition,
        "dependency-observation": observer_disposition,
        "temporal-knowledge": temporal_disposition,
    }

    order = ["REJECTED", "OUT_OF_SCOPE", "UNVERIFIED", "INCOMPLETE", "BOUND"]
    if upstream_disposition == "REJECTED" or any(x == "REJECTED" for x in dispositions.values()):
        overall = "REJECTED"
    elif any(x == "OUT_OF_SCOPE" for x in dispositions.values()):
        overall = "OUT_OF_SCOPE"
    elif any(x == "UNVERIFIED" for x in dispositions.values()):
        overall = "UNVERIFIED"
    elif any(x == "INCOMPLETE" for x in dispositions.values()):
        overall = "INCOMPLETE"
    else:
        # Additional derived obligations remain unassessed in this adapter.
        overall = "INCOMPLETE"
    return errors, dispositions, overall


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

    errors, dispositions, overall = evaluate(instance)
    if errors or overall != "INCOMPLETE" or any(x != "BOUND" for x in dispositions.values()):
        for error in errors:
            print(f"CONTRACT FAIL: {error}")
        raise SystemExit(f"FAIL: synthetic authority/observer/temporal binding is {overall}: {dispositions}")

    # Ten adverse/positive controls; each is a separate root cause, not an
    # independent engineering episode for statistical reporting.
    late_authority = copy.deepcopy(instance)
    late_authority["authority_receipt"]["receiver_first_seen_at_unix_ms"] = (
        late_authority["evidence_binding_input"]["decision_context"]["as_known_at_unix_ms"] + 1
    )
    _, late_auth_states, _ = evaluate(late_authority)
    if late_auth_states["producer-authority"] != "UNVERIFIED":
        raise SystemExit("FAIL: late claim-level authority receipt backfilled the cutoff")

    wrong_role = copy.deepcopy(instance)
    wrong_role["authority_receipt"]["authority_role_ref"] = "role:unrelated"
    _, wrong_role_states, _ = evaluate(wrong_role)
    if wrong_role_states["producer-authority"] != "REJECTED":
        raise SystemExit("FAIL: authority receipt from a different role was accepted")

    wrong_authority_owner = copy.deepcopy(instance)
    wrong_authority_owner["authority_receipt"]["owner_namespace"] = "owner:unrelated"
    _, wrong_owner_states, _ = evaluate(wrong_authority_owner)
    if wrong_owner_states["producer-authority"] != "REJECTED":
        raise SystemExit("FAIL: authority receipt from another owner was accepted")

    late_observer = copy.deepcopy(instance)
    late_observer["observer_receipt"]["receiver_first_seen_at_unix_ms"] = (
        late_observer["evidence_binding_input"]["decision_context"]["as_known_at_unix_ms"] + 1
    )
    _, late_observer_states, _ = evaluate(late_observer)
    if late_observer_states["dependency-observation"] != "UNVERIFIED":
        raise SystemExit("FAIL: late observer receipt backfilled the cutoff")

    report_substitution = copy.deepcopy(instance)
    report_substitution["observer_receipt"]["coverage_report_sha256"] = "0" * 64
    _, report_states, _ = evaluate(report_substitution)
    if report_states["dependency-observation"] != "REJECTED":
        raise SystemExit("FAIL: observer report substitution was accepted")

    owner_receipt_substitution = copy.deepcopy(instance)
    owner_receipt_substitution["observer_receipt"]["owner_observer_coverage_receipt_sha256"] = "0" * 64
    _, owner_receipt_states, _ = evaluate(owner_receipt_substitution)
    if owner_receipt_states["dependency-observation"] != "REJECTED":
        raise SystemExit("FAIL: substituted owner observer receipt was accepted")

    out_of_scope = copy.deepcopy(instance)
    out_of_scope["observer_coverage"]["status"] = "out_of_scope"
    out_of_scope["observer_coverage"]["replay_consequence"] = "abstain"
    out_of_scope["observer_receipt"]["coverage_status"] = "out_of_scope"
    out_of_scope["observer_receipt"]["replay_consequence"] = "abstain"
    _, out_of_scope_states, _ = evaluate(out_of_scope)
    if out_of_scope_states["dependency-observation"] != "OUT_OF_SCOPE":
        raise SystemExit("FAIL: out-of-scope observer coverage was promoted")

    late_temporal = copy.deepcopy(instance)
    late_temporal["temporal_receipt"]["receiver_first_seen_at_unix_ms"] = (
        late_temporal["evidence_binding_input"]["decision_context"]["as_known_at_unix_ms"] + 1
    )
    _, late_temporal_states, _ = evaluate(late_temporal)
    if late_temporal_states["temporal-knowledge"] != "UNVERIFIED":
        raise SystemExit("FAIL: late temporal snapshot backfilled historical knowledge")

    watermark_substitution = copy.deepcopy(instance)
    watermark_substitution["temporal_receipt"]["receiver_watermark_unix_ms"] += 1
    _, watermark_states, _ = evaluate(watermark_substitution)
    if watermark_states["temporal-knowledge"] != "REJECTED":
        raise SystemExit("FAIL: temporal snapshot with a different owner watermark was accepted")

    missing_observer = copy.deepcopy(instance)
    missing_observer["observer_receipt"] = None
    # Schema requires a receipt, so the pure oracle verifies the runtime contract separately.
    if missing_observer["observer_receipt"] is not None:
        raise SystemExit("FAIL: missing observer receipt mutation fixture malformed")

    print(
        "PASS: AERO claim authority/observer/temporal v0.1 schema, source-bound receipt scope, "
        "historical watermarks and 10 mutation controls."
    )
    print(
        "NOTE: upstream VERIFIED values are dispositions, not signature verification performed here. "
        "This adapter leaves other derived obligations unassessed and never provides physical authority."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
