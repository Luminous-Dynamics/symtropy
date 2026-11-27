#!/usr/bin/env python3
"""Independent test-only reference oracle for claim-closure v0.1.

This deliberately recomputes finding codes in Python instead of calling the Rust
library. It is a benchmark comparator, not a production evaluator or authority
adapter. It checks the frozen positive fixture and every authored mutation.
"""
from __future__ import annotations

import copy
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INPUT_VERSION = "engineering-claim-closure-v0.1"
MAX_DEPTH = 256
SHA256 = re.compile(r"^[0-9a-fA-F]{64}$")


def valid_identifier(value: object) -> bool:
    return (
        isinstance(value, str)
        and bool(value)
        and value.strip() == value
        and not any(ord(character) < 32 or ord(character) == 127 for character in value)
    )


def valid_sha256(value: object) -> bool:
    return isinstance(value, str) and SHA256.fullmatch(value) is not None


def apply_pointer(document: object, pointer: str) -> tuple[object, str]:
    if not pointer.startswith("/"):
        raise ValueError(f"invalid JSON pointer: {pointer!r}")
    tokens = [token.replace("~1", "/").replace("~0", "~") for token in pointer[1:].split("/")]
    parent = document
    for token in tokens[:-1]:
        parent = parent[int(token)] if isinstance(parent, list) else parent[token]
    return parent, tokens[-1]


def apply_mutation(document: dict, operation: str, pointer: str, value: object) -> None:
    parent, token = apply_pointer(document, pointer)
    if operation == "set":
        if isinstance(parent, list):
            parent[int(token)] = value
        else:
            parent[token] = value
    elif operation == "append":
        target = parent[int(token)] if isinstance(parent, list) else parent[token]
        if not isinstance(target, list):
            raise TypeError(f"append target is not an array: {pointer}")
        target.append(value)
    else:
        raise ValueError(f"unsupported mutation operation: {operation}")


def evaluate(document: dict) -> set[str]:
    findings: set[str] = set()

    def add(code: str) -> None:
        findings.add(code)

    claim = document["claim"]
    if document.get("schema_version") != INPUT_VERSION:
        add("INPUT_SCHEMA_VERSION_UNSUPPORTED")

    for field in ("claim_id", "subject_ref", "configuration_ref", "scope_ref"):
        if not valid_identifier(claim.get(field)):
            add(f"CLAIM_{field.upper()}_INVALID")

    acquisition_start = claim["acquisition_start_unix_ms"]
    acquisition_end = claim["acquisition_end_unix_ms"]
    knowledge_cut = claim["knowledge_cut_unix_ms"]
    evaluated_at = claim["evaluated_at_unix_ms"]
    if acquisition_start > acquisition_end:
        add("CLAIM_ACQUISITION_INTERVAL_REVERSED")
    if knowledge_cut > evaluated_at:
        add("KNOWLEDGE_CUT_AFTER_EVALUATION_TIME")
    if not claim["requirements"]:
        add("CLAIM_HAS_NO_REQUIRED_EVIDENCE")

    by_ref: dict[str, list[dict]] = {}
    for record in document["evidence_records"]:
        by_ref.setdefault(record.get("evidence_ref", ""), []).append(record)

    states: dict[str, int] = {}

    def validate_attestation(attestation: dict, identity_key: str, digest_key: str, label: str, evidence_ref: str) -> None:
        status = attestation.get("status")
        if status == "verified":
            if not valid_identifier(attestation.get(identity_key)):
                add(f"{label}_IDENTITY_MISSING")
            if not valid_sha256(attestation.get(digest_key)):
                add(f"{label}_DIGEST_INVALID")
            as_of = attestation.get("as_of_unix_ms")
            if as_of is None or as_of > knowledge_cut:
                add(f"{label}_NOT_KNOWN_AT_CLAIM_CUT")
        elif status in ("unverified", "disputed"):
            add(f"{label}_UNVERIFIED")
        elif status == "revoked":
            add(f"{label}_REVOKED")
        else:
            add(f"{label}_UNVERIFIED")

    def validate_record(record: dict, evidence_ref: str) -> None:
        for field in (
            "evidence_ref",
            "role",
            "subject_ref",
            "configuration_ref",
            "scope_ref",
        ):
            if not valid_identifier(record.get(field)):
                add(f"EVIDENCE_{field.upper()}_INVALID")
        if not valid_identifier(record["uncertainty_budget"].get("budget_ref")):
            add("EVIDENCE_UNCERTAINTY_BUDGET_REF_INVALID")
        if not valid_sha256(record.get("record_sha256")):
            add("EVIDENCE_RECORD_DIGEST_INVALID")
        if record["recorded_at_unix_ms"] > knowledge_cut:
            add("EVIDENCE_NOT_KNOWN_AT_CLAIM_CUT")

        start = record["effective_from_unix_ms"]
        end = record.get("effective_until_exclusive_unix_ms")
        if end is not None and end <= start:
            add("EVIDENCE_EFFECTIVE_INTERVAL_INVALID")
        else:
            overlaps = start <= acquisition_end and (end is None or end > acquisition_start)
            covers = start <= acquisition_start and (end is None or end > acquisition_end)
            if not overlaps:
                add("EVIDENCE_NOT_EFFECTIVE_DURING_CLAIM_INTERVAL")
            elif not covers:
                add("EVIDENCE_EFFECTIVE_INTERVAL_DOES_NOT_COVER_CLAIM")

        validate_attestation(
            record["producer_authority"], "identity_ref", "record_sha256",
            "PRODUCER_AUTHORITY", evidence_ref
        )
        validate_attestation(
            record["trust_snapshot"], "policy_ref", "policy_sha256",
            "TRUST_SNAPSHOT", evidence_ref
        )
        budget = record["uncertainty_budget"]
        if not valid_sha256(budget.get("digest_sha256")):
            add("UNCERTAINTY_BUDGET_DIGEST_INVALID")
        if not budget.get("resolved", False):
            add("UNCERTAINTY_BUDGET_UNRESOLVED")

    def visit(evidence_ref: str, depth: int) -> None:
        if depth > MAX_DEPTH:
            add("DEPENDENCY_GRAPH_DEPTH_LIMIT_EXCEEDED")
            return
        state = states.get(evidence_ref)
        if state == 1:
            add("DEPENDENCY_CYCLE_DETECTED")
            return
        if state == 2:
            return

        records = by_ref.get(evidence_ref)
        if records is None:
            add("DEPENDENCY_EVIDENCE_MISSING")
            return
        if len(records) != 1:
            add("DUPLICATE_EVIDENCE_REFERENCE")
            return

        record = records[0]
        states[evidence_ref] = 1
        validate_record(record, evidence_ref)
        dependency_refs: set[str] = set()
        dependencies = sorted(
            record["dependencies"],
            key=lambda item: (
                item.get("evidence_ref", ""),
                item.get("expected_record_sha256", ""),
            ),
        )
        for dependency in dependencies:
            child_ref = dependency.get("evidence_ref", "")
            if child_ref in dependency_refs:
                add("DUPLICATE_DEPENDENCY_EDGE")
                continue
            dependency_refs.add(child_ref)
            if not valid_identifier(child_ref) or not valid_sha256(dependency.get("expected_record_sha256")):
                add("DEPENDENCY_BINDING_INVALID")
                continue
            parents = by_ref.get(child_ref)
            if parents is None:
                add("DEPENDENCY_EVIDENCE_MISSING")
            elif len(parents) != 1:
                add("DUPLICATE_EVIDENCE_REFERENCE")
            else:
                if parents[0].get("record_sha256") != dependency["expected_record_sha256"]:
                    add("DEPENDENCY_RECORD_DIGEST_MISMATCH")
                visit(child_ref, depth + 1)
        states[evidence_ref] = 2

    seen_requirements: set[str] = set()
    for requirement in claim["requirements"]:
        evidence_ref = requirement.get("evidence_ref", "")
        if evidence_ref in seen_requirements:
            add("DUPLICATE_CLAIM_REQUIREMENT")
            continue
        seen_requirements.add(evidence_ref)

        if (
            not valid_identifier(evidence_ref)
            or not valid_identifier(requirement.get("role"))
            or not valid_identifier(requirement.get("uncertainty_budget_ref"))
            or not valid_sha256(requirement.get("expected_record_sha256"))
            or not valid_sha256(requirement.get("uncertainty_budget_sha256"))
        ):
            add("CLAIM_REQUIREMENT_BINDING_INVALID")
            continue

        records = by_ref.get(evidence_ref)
        if records is None:
            add("REQUIRED_EVIDENCE_MISSING")
            continue
        if len(records) != 1:
            add("DUPLICATE_EVIDENCE_REFERENCE")
            continue
        record = records[0]
        if record.get("record_sha256") != requirement["expected_record_sha256"]:
            add("REQUIRED_EVIDENCE_DIGEST_MISMATCH")
        if record.get("role") != requirement["role"]:
            add("REQUIRED_EVIDENCE_ROLE_MISMATCH")
        if (
            record.get("subject_ref") != claim["subject_ref"]
            or record.get("configuration_ref") != claim["configuration_ref"]
            or record.get("scope_ref") != claim["scope_ref"]
        ):
            add("REQUIRED_EVIDENCE_CLAIM_CONTEXT_MISMATCH")
        budget = record["uncertainty_budget"]
        if (
            budget.get("budget_ref") != requirement["uncertainty_budget_ref"]
            or budget.get("digest_sha256") != requirement["uncertainty_budget_sha256"]
        ):
            add("REQUIRED_EVIDENCE_UNCERTAINTY_BINDING_MISMATCH")
        visit(evidence_ref, 0)

    return findings


def main() -> int:
    fixture = json.loads((ROOT / "fixtures/positive-v0.1.json").read_text())
    mutation_base = json.loads((ROOT / "fixtures/mutation-base-v0.1.json").read_text())
    corpus = json.loads((ROOT / "fixtures/mutation-cases-v0.1.json").read_text())

    positive_findings = evaluate(fixture)
    if positive_findings:
        print("positive fixture unexpectedly blocked:", sorted(positive_findings), file=sys.stderr)
        return 1

    mutation_base_findings = evaluate(mutation_base)
    if mutation_base_findings:
        print(
            "mutation base unexpectedly blocked:",
            sorted(mutation_base_findings),
            file=sys.stderr,
        )
        return 1
    if len(mutation_base["evidence_records"]) != 3:
        print("mutation base does not contain the frozen three-record dependency chain", file=sys.stderr)
        return 1

    case_ids: set[str] = set()
    for mutation in corpus["cases"]:
        case_id = mutation["case_id"]
        if case_id in case_ids:
            print(f"duplicate mutation ID: {case_id}", file=sys.stderr)
            return 1
        case_ids.add(case_id)
        mutated = copy.deepcopy(mutation_base)
        apply_mutation(mutated, mutation["operation"], mutation["target"], mutation["value"])
        findings = evaluate(mutated)
        expected = mutation["expected_finding_code"]
        if expected not in findings:
            print(
                f"{case_id}: expected {expected}; observed {sorted(findings)}",
                file=sys.stderr,
            )
            return 1
        if not findings:
            print(f"{case_id}: mutation produced no finding", file=sys.stderr)
            return 1

    reordered = copy.deepcopy(mutation_base)
    reordered["evidence_records"].reverse()
    if evaluate(reordered) != mutation_base_findings:
        print("mutation-base finding set changed under evidence-record permutation", file=sys.stderr)
        return 1

    print(
        f"PASS: independent Python reference oracle; positive fixture, shared mutation base and "
        f"{len(case_ids)} adversarial mutations; evidence-order invariance"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
