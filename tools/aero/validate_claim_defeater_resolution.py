#!/usr/bin/env python3
"""CI-only independent oracle for AERO advisory claim defeater resolution v0.1."""
from __future__ import annotations

import copy
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker
import validate_claim_obligation_evidence_binding as evidence_binding_oracle

ROOT = Path(__file__).resolve().parents[2]
AVIATION = ROOT / "docs" / "research" / "aviation"
SCHEMA_PATH = AVIATION / "schemas" / "aero_claim_defeater_resolution_v0_1.schema.json"
UPSTREAM_EXAMPLE = AVIATION / "examples" / "aero_claim_obligation_evidence_binding_v0_1.json"
HEX64 = re.compile(r"^[a-f0-9]{64}$")
CANONICALIZATION = "rust-serde-json-struct-order-v1"


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def digest_json(value: Any) -> str:
    return digest_bytes(
        json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    )


def support_manifest_sha256(refs: list[str]) -> str:
    if len(refs) != len(set(refs)):
        raise ValueError("duplicate supporting evidence reference")
    return digest_json({
        "schema": "AeroDefeaterSupportingEvidenceManifestV0_1",
        "canonicalization_profile": CANONICALIZATION,
        "evidence_refs": sorted(refs),
    })


def expected_defeaters(spec: dict[str, Any]) -> list[dict[str, str]]:
    expected: list[dict[str, str]] = []
    for statement in spec["assumptions"]:
        expected.append({
            "obligation_id": f"assumption-defeater:{statement}",
            "requirement_source_ref": f"claim-spec:/assumptions/{statement}",
            "defeater_kind": "ASSUMPTION",
            "statement_sha256": digest_bytes(statement.encode("utf-8")),
        })
    for statement in spec["exclusions"]:
        expected.append({
            "obligation_id": f"exclusion-defeater:{statement}",
            "requirement_source_ref": f"claim-spec:/exclusions/{statement}",
            "defeater_kind": "EXCLUSION",
            "statement_sha256": digest_bytes(statement.encode("utf-8")),
        })
    return sorted(expected, key=lambda item: item["obligation_id"])


def make_receipts(upstream: dict[str, Any]) -> list[dict[str, Any]]:
    spec = upstream["claim_specification"]
    context = upstream["decision_context"]
    profile_sha = evidence_binding_oracle.requirement_profile_sha256()
    obligations = evidence_binding_oracle.derive_obligations(spec)
    semantic_sha = evidence_binding_oracle.semantic_sha256(spec)
    obligation_sha = evidence_binding_oracle.obligation_manifest_sha256(
        spec, semantic_sha, profile_sha, obligations
    )
    expected = expected_defeaters(spec)
    cutoff = context["as_known_at_unix_ms"]
    receipts: list[dict[str, Any]] = []
    for index, item in enumerate(expected):
        states = [
            "RESOLVED_WITH_EVIDENCE",
            "ACCEPTED_AS_LIMITATION",
            "NOT_APPLICABLE_WITH_JUSTIFICATION",
        ]
        state = states[index % len(states)]
        refs = [f"evidence:defeater-review-{index}"] if state == "RESOLVED_WITH_EVIDENCE" else []
        receipts.append({
            "verification_status": "VERIFIED",
            "reviewer_namespace": "reviewer:structures-board",
            "authority_holder_namespace": "authority:independent-structures-reviewer",
            "authority_role_ref": spec["acceptance_authority_role_ref"],
            "claim_id": spec["claim_id"],
            "configuration_ref": spec["configuration_ref"],
            "decision_context_sha256": upstream["decision_context_sha256"],
            "claim_specification_semantic_sha256": semantic_sha,
            "obligation_profile_sha256": profile_sha,
            "obligation_manifest_sha256": obligation_sha,
            "obligation_id": item["obligation_id"],
            "requirement_source_ref": item["requirement_source_ref"],
            "defeater_kind": item["defeater_kind"],
            "statement_sha256": item["statement_sha256"],
            "state": state,
            "supporting_evidence_refs": refs,
            "supporting_evidence_manifest_sha256": support_manifest_sha256(refs),
            "rationale_sha256": (
                f"{index + 1:064x}"
                if state in ("ACCEPTED_AS_LIMITATION", "NOT_APPLICABLE_WITH_JUSTIFICATION")
                else None
            ),
            "receiver_first_seen_at_unix_ms": cutoff - 100,
            "effective_from_unix_ms": cutoff - 200,
            "effective_until_unix_ms": cutoff + 200,
            "receipt_sha256": f"{index + 10:064x}",
        })
    return receipts


def validate(source: dict[str, Any]) -> tuple[list[str], str, list[str]]:
    errors: list[str] = []
    spec = source["claim_specification"]
    context = source["decision_context"]
    cutoff = context["as_known_at_unix_ms"]
    upstream_errors, upstream_state = evidence_binding_oracle.validate(
        source["upstream_evidence_binding"]
    )
    if upstream_errors or upstream_state != "BOUND":
        errors.append("upstream evidence-class binding is not BOUND")

    if (
        context["claim_id"] != spec["claim_id"]
        or context["subject_ref"] != spec["subject_ref"]
        or context["configuration_ref"] != spec["configuration_ref"]
        or context["operating_envelope_ref"] != spec["operating_envelope_ref"]
        or context["as_known_at_unix_ms"] != source["upstream_evidence_binding"]["decision_context"]["as_known_at_unix_ms"]
    ):
        errors.append("claim/decision-context scope mismatch")

    context_sha = evidence_binding_oracle.decision_context_sha256(context)
    if context_sha != source["decision_context_sha256"]:
        errors.append("decision-context digest mismatch")
    profile_sha = evidence_binding_oracle.requirement_profile_sha256()
    semantic_sha = evidence_binding_oracle.semantic_sha256(spec)
    obligations = evidence_binding_oracle.derive_obligations(spec)
    obligation_sha = evidence_binding_oracle.obligation_manifest_sha256(
        spec, semantic_sha, profile_sha, obligations
    )
    expected = expected_defeaters(spec)
    expected_by_id = {item["obligation_id"]: item for item in expected}

    receipt_ids = [item["obligation_id"] for item in source["receipts"]]
    if len(receipt_ids) != len(set(receipt_ids)):
        errors.append("duplicate defeater resolution receipt")
    if set(receipt_ids) - set(expected_by_id):
        errors.append("receipt for undeclared defeater obligation")

    open_ids = sorted(set(expected_by_id) - set(receipt_ids))
    has_unverified = False
    has_rejected = bool(errors)
    for receipt in source["receipts"]:
        expected_item = expected_by_id.get(receipt["obligation_id"])
        if expected_item is None:
            continue
        if (
            receipt["claim_id"] != spec["claim_id"]
            or receipt["configuration_ref"] != spec["configuration_ref"]
            or receipt["decision_context_sha256"] != context_sha
            or receipt["claim_specification_semantic_sha256"] != semantic_sha
            or receipt["obligation_profile_sha256"] != profile_sha
            or receipt["obligation_manifest_sha256"] != obligation_sha
            or receipt["requirement_source_ref"] != expected_item["requirement_source_ref"]
            or receipt["defeater_kind"] != expected_item["defeater_kind"]
            or receipt["statement_sha256"] != expected_item["statement_sha256"]
            or receipt["authority_role_ref"] != spec["acceptance_authority_role_ref"]
            or not HEX64.fullmatch(receipt["receipt_sha256"])
            or not HEX64.fullmatch(receipt["supporting_evidence_manifest_sha256"])
        ):
            errors.append(f"receipt scope/commitment mismatch: {receipt['obligation_id']}")
            has_rejected = True
        try:
            support_sha = support_manifest_sha256(receipt["supporting_evidence_refs"])
        except ValueError:
            support_sha = ""
        if support_sha != receipt["supporting_evidence_manifest_sha256"]:
            errors.append(f"supporting evidence manifest mismatch: {receipt['obligation_id']}")
            has_rejected = True
        if any(not ref or ref.strip() != ref for ref in receipt["supporting_evidence_refs"]):
            errors.append(f"invalid supporting evidence reference: {receipt['obligation_id']}")
            has_rejected = True
        rationale = receipt["rationale_sha256"]
        if rationale is not None and not HEX64.fullmatch(rationale):
            errors.append(f"invalid rationale digest: {receipt['obligation_id']}")
            has_rejected = True

        state = receipt["state"]
        if state == "RESOLVED_WITH_EVIDENCE" and not receipt["supporting_evidence_refs"]:
            errors.append(f"resolved state has no evidence: {receipt['obligation_id']}")
            has_rejected = True
        if state in ("ACCEPTED_AS_LIMITATION", "NOT_APPLICABLE_WITH_JUSTIFICATION") and rationale is None:
            errors.append(f"justification missing: {receipt['obligation_id']}")
            has_rejected = True
        if receipt["verification_status"] == "FAILED":
            errors.append(f"receipt verification failed: {receipt['obligation_id']}")
            has_rejected = True
        elif receipt["verification_status"] == "UNVERIFIED":
            has_unverified = True
        if receipt["receiver_first_seen_at_unix_ms"] > cutoff or not (
            receipt["effective_from_unix_ms"] <= cutoff < receipt["effective_until_unix_ms"]
        ):
            errors.append(f"receipt not known/effective at cutoff: {receipt['obligation_id']}")
            has_unverified = True
        if state == "OPEN":
            open_ids.append(receipt["obligation_id"])

    open_ids = sorted(set(open_ids))
    if has_rejected:
        disposition = "REJECTED"
    elif has_unverified:
        disposition = "UNVERIFIED"
    elif open_ids:
        disposition = "OPEN_ITEMS_REMAIN"
    else:
        disposition = "ALL_REVIEWED"
    return errors, disposition, open_ids


def main() -> int:
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    upstream = json.loads(UPSTREAM_EXAMPLE.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    instance = {
        "schema": "AeroClaimDefeaterResolutionV0_1",
        "requirement_profile": "AERO_SCALAR_ENGINEERING_CLAIM_V0_1",
        "claim_specification": upstream["claim_specification"],
        "decision_context": upstream["decision_context"],
        "decision_context_sha256": upstream["decision_context_sha256"],
        "upstream_evidence_binding": upstream,
        "receipts": make_receipts(upstream),
    }
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    schema_errors = sorted(
        validator.iter_errors({
            key: value for key, value in instance.items()
            if key != "upstream_evidence_binding" and key != "decision_context_sha256"
        }),
        key=lambda error: (list(map(str, error.absolute_path)), error.message),
    )
    if schema_errors:
        for error in schema_errors:
            print(f"SCHEMA FAIL /{'/'.join(map(str, error.absolute_path))}: {error.message}")
        return 1

    errors, disposition, open_ids = validate(instance)
    if errors or disposition != "ALL_REVIEWED" or open_ids:
        for error in errors:
            print(f"CONTRACT FAIL: {error}")
        raise SystemExit(f"FAIL: baseline defeater state is {disposition}; open={open_ids}")

    # 9 adversarial / permutation controls.
    open_item = copy.deepcopy(instance)
    open_item["receipts"][0]["state"] = "OPEN"
    open_errors, open_disposition, open_ids = validate(open_item)
    if open_errors or open_disposition != "OPEN_ITEMS_REMAIN" or not open_ids:
        raise SystemExit("FAIL: an OPEN defeater was silently resolved")

    missing = copy.deepcopy(instance)
    missing["receipts"].pop()
    _, missing_disposition, missing_ids = validate(missing)
    if missing_disposition != "OPEN_ITEMS_REMAIN" or not missing_ids:
        raise SystemExit("FAIL: missing defeater receipt was treated as resolved")

    extra = copy.deepcopy(instance)
    extra["receipts"].append(copy.deepcopy(extra["receipts"][0]))
    extra["receipts"][-1]["obligation_id"] = "assumption-defeater:assumption:undeclared"
    _, extra_disposition, _ = validate(extra)
    if extra_disposition != "REJECTED":
        raise SystemExit("FAIL: receipt for an undeclared defeater was accepted")

    wrong_role = copy.deepcopy(instance)
    wrong_role["receipts"][0]["authority_role_ref"] = "role:unrelated"
    _, wrong_role_disposition, _ = validate(wrong_role)
    if wrong_role_disposition != "REJECTED":
        raise SystemExit("FAIL: defeater reviewer substituted the acceptance role")

    no_support = copy.deepcopy(instance)
    item = next(x for x in no_support["receipts"] if x["state"] == "RESOLVED_WITH_EVIDENCE")
    item["supporting_evidence_refs"] = []
    item["supporting_evidence_manifest_sha256"] = support_manifest_sha256([])
    _, no_support_disposition, _ = validate(no_support)
    if no_support_disposition != "REJECTED":
        raise SystemExit("FAIL: RESOLVED_WITH_EVIDENCE without supporting refs was accepted")

    no_rationale = copy.deepcopy(instance)
    item = next(x for x in no_rationale["receipts"] if x["state"] == "ACCEPTED_AS_LIMITATION")
    item["rationale_sha256"] = None
    _, rationale_disposition, _ = validate(no_rationale)
    if rationale_disposition != "REJECTED":
        raise SystemExit("FAIL: accepted limitation without rationale was accepted")

    late = copy.deepcopy(instance)
    late["receipts"][0]["receiver_first_seen_at_unix_ms"] = (
        late["decision_context"]["as_known_at_unix_ms"] + 1
    )
    _, late_disposition, _ = validate(late)
    if late_disposition != "UNVERIFIED":
        raise SystemExit("FAIL: late defeater receipt backfilled historical review")

    wrong_statement = copy.deepcopy(instance)
    wrong_statement["receipts"][0]["statement_sha256"] = "0" * 64
    _, statement_disposition, _ = validate(wrong_statement)
    if statement_disposition != "REJECTED":
        raise SystemExit("FAIL: receipt for a substituted assumption/exclusion was accepted")

    permuted = copy.deepcopy(instance)
    permuted["receipts"].reverse()
    permuted_errors, permuted_disposition, permuted_open = validate(permuted)
    if permuted_errors or permuted_disposition != "ALL_REVIEWED" or permuted_open:
        raise SystemExit("FAIL: receipt order permutation changed defeater resolution")

    print(
        "PASS: AERO advisory defeater resolution v0.1 derived universe, receipt scope, "
        "evidence/rationale requirements, historical chronology and 9 mutation/permutation controls."
    )
    print(
        "NOTE: receipt VERIFIED is an upstream disposition. This adapter does not authenticate "
        "signatures, establish physical truth, change defeater severity or grant engineering authority."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
