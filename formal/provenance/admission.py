#!/usr/bin/env python3
"""Canonical, fail-closed qualification admission envelope for PKG v0."""
from __future__ import annotations
import argparse, hashlib, json
from typing import Any
from impact import IMPACT_VERSION
from revalidation_plan import PLAN_VERSION, _authenticated_impact, build_plan
from validate import SCHEMA_VERSION, Validator

ADMISSION_VERSION = "luminous.formal-admission.v0"
REFERENCE_FIELDS = ("artifact_id","checker_id","source_commit_id","source_tree_id","execution_id","contract_id","verifier_release_id","subject_commit_id","subject_tree_id")

def _canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")

def digest(value: Any) -> str:
    return hashlib.sha256(_canonical_bytes(value)).hexdigest()

def _identity_digest(node: dict[str, Any]) -> str:
    metadata = node.get("metadata")
    value = metadata.get("semantic_digest") if node["kind"] in {"Theorem","Lemma","Invariant"} and isinstance(metadata, dict) else node.get("content_digest")
    if not isinstance(value, str) or len(value) != 64 or any(c not in "0123456789abcdef" for c in value):
        raise ValueError(f"node {node['id']} lacks an exact identity/content digest")
    return value

def _checker_ids(node_id: str, graph: dict[str, Any]) -> list[str]:
    return sorted({e["target"] for e in graph["edges"] if e["relation"] == "checked_by" and e["source"] == node_id})

def _binding_record(node: dict[str, Any], nodes: dict[str, dict[str, Any]], graph: dict[str, Any]) -> dict[str, Any]:
    refs = []
    metadata = node.get("metadata")
    if isinstance(metadata, dict):
        for field in REFERENCE_FIELDS:
            ref_id = metadata.get(field)
            if ref_id is None:
                continue
            if not isinstance(ref_id, str) or ref_id not in nodes:
                raise ValueError(f"{node['id']} has unresolved binding {field}")
            ref = nodes[ref_id]
            refs.append({"field": field, "node_id": ref_id, "kind": ref["kind"], "identity_digest": _identity_digest(ref)})
    for checker_id in _checker_ids(node["id"], graph):
        if checker_id not in nodes or nodes[checker_id]["kind"] != "ProofChecker":
            raise ValueError(f"{node['id']} has invalid checker binding")
        refs.append({"field": "checked_by", "node_id": checker_id, "kind": "ProofChecker", "identity_digest": _identity_digest(nodes[checker_id])})
    refs.sort(key=lambda x: (x["field"], x["node_id"]))
    return {"node_id": node["id"], "kind": node["kind"], "identity_digest": _identity_digest(node), "references": refs}


def _validate_evidence_binding(
    evidence: dict[str, Any],
    nodes: dict[str, dict[str, Any]],
    subject_head: str,
    subject_tree: str,
    expected_result: str,
) -> None:
    metadata = evidence.get("metadata")
    if not isinstance(metadata, dict):
        raise ValueError("qualification evidence lacks metadata")
    expected = {
        "execution_id": "ProofExecution",
        "contract_id": "QualificationContract",
        "verifier_release_id": "VerifierRelease",
    }
    for field, kind in expected.items():
        ref_id = metadata.get(field)
        if not isinstance(ref_id, str) or ref_id not in nodes:
            raise ValueError(f"qualification evidence lacks valid {field}")
        if nodes[ref_id]["kind"] != kind:
            raise ValueError(f"qualification evidence {field} has wrong node kind")
    contract = nodes[metadata["contract_id"]]
    verifier = nodes[metadata["verifier_release_id"]]
    if metadata.get("contract_digest") != _identity_digest(contract):
        raise ValueError("qualification evidence contract_digest mismatch")
    if metadata.get("verifier_release_digest") != _identity_digest(verifier):
        raise ValueError("qualification evidence verifier_release_digest mismatch")
    if metadata.get("subject_head") != subject_head or metadata.get("subject_tree") != subject_tree:
        raise ValueError("qualification evidence subject binding mismatch")
    result = metadata.get("result")
    expected_evidence_result = "Pass" if expected_result == "QualifiedPass" else "Fail"
    if result != expected_evidence_result:
        raise ValueError("qualification evidence result contradicts expected qualification result")

def build_admission(graph: dict[str, Any], impact: dict[str, Any], *, subject_repository: str, expected_result: str) -> dict[str, Any]:
    if not isinstance(subject_repository, str) or not subject_repository or any(c.isspace() for c in subject_repository):
        raise ValueError("subject_repository must be a non-empty repository identifier")
    if expected_result not in {"QualifiedPass","QualifiedFail"}:
        raise ValueError("expected_result must be QualifiedPass or QualifiedFail")
    if Validator(graph).validate()["status"] != "Valid":
        raise ValueError("input provenance graph is not semantically valid")
    authenticated = _authenticated_impact(graph, impact)
    if not authenticated["changed_node_ids"]:
        raise ValueError("explicit admission requires at least one changed root")
    plan = build_plan(graph, authenticated)
    if plan["plan_status"] != "Ready":
        raise ValueError("qualification admission requires a Ready revalidation plan")
    nodes = {n["id"]: n for n in graph["nodes"]}
    bindings = sorted((_binding_record(nodes[r["node_id"]], nodes, graph) for r in plan["records"]), key=lambda x: x["node_id"])
    subject_values = []
    for binding in bindings:
        metadata = nodes[binding["node_id"]].get("metadata")
        if isinstance(metadata, dict):
            for field in ("subject_head","subject_tree"):
                if field in metadata:
                    value = metadata[field]
                    if not isinstance(value, str) or len(value) != 64 or any(c not in "0123456789abcdef" for c in value):
                        raise ValueError(f"{binding['node_id']} has invalid {field}")
                    subject_values.append((field, value))
    heads = sorted({v for f,v in subject_values if f == "subject_head"})
    trees = sorted({v for f,v in subject_values if f == "subject_tree"})
    if len(heads) != 1 or len(trees) != 1:
        raise ValueError("admission requires one consistent subject head and tree")
    evidence_ids = [b["node_id"] for b in bindings if b["kind"] == "QualificationEvidence"]
    if len(evidence_ids) != 1:
        raise ValueError("admission requires exactly one current qualification evidence binding")
    _validate_evidence_binding(nodes[evidence_ids[0]], nodes, heads[0], trees[0], expected_result)
    envelope = {
        "schema_version": SCHEMA_VERSION,
        "admission_version": ADMISSION_VERSION,
        "impact": {"impact_version": IMPACT_VERSION, "changed_node_ids": authenticated["changed_node_ids"], "impact_digest": digest(authenticated)},
        "plan": {"plan_version": PLAN_VERSION, "plan_status": plan["plan_status"], "plan_digest": digest(plan)},
        "subject": {"repository": subject_repository, "head_sha": heads[0], "tree_sha": trees[0]},
        "expected_result": expected_result,
        "bindings": bindings,
    }
    envelope["admission_digest"] = digest(envelope)
    return envelope

def validate_admission(graph: dict[str, Any], impact: dict[str, Any], envelope: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(envelope, dict) or envelope.get("schema_version") != SCHEMA_VERSION or envelope.get("admission_version") != ADMISSION_VERSION:
        raise ValueError("unsupported admission envelope")
    expected = envelope.get("admission_digest")
    if not isinstance(expected, str):
        raise ValueError("missing admission_digest")
    unsigned = dict(envelope)
    unsigned.pop("admission_digest", None)
    if digest(unsigned) != expected:
        raise ValueError("admission_digest does not match canonical envelope bytes")
    subject = envelope.get("subject")
    if not isinstance(subject, dict):
        raise ValueError("missing subject binding")
    rebuilt = build_admission(graph, impact, subject_repository=subject.get("repository"), expected_result=envelope.get("expected_result"))
    if rebuilt != envelope:
        raise ValueError("admission envelope does not match recomputed bindings")
    return rebuilt

def main() -> int:
    parser = argparse.ArgumentParser(description="Build a qualification admission envelope")
    parser.add_argument("graph")
    parser.add_argument("impact")
    parser.add_argument("--subject-repository", required=True)
    parser.add_argument("--expected-result", choices=("QualifiedPass","QualifiedFail"), required=True)
    args = parser.parse_args()
    try:
        with open(args.graph, encoding="utf-8") as f: graph = json.load(f)
        with open(args.impact, encoding="utf-8") as f: impact = json.load(f)
        result = build_admission(graph, impact, subject_repository=args.subject_repository, expected_result=args.expected_result)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError, KeyError, TypeError) as exc:
        result = {"schema_version": SCHEMA_VERSION, "admission_version": ADMISSION_VERSION, "admission_status": "Invalid", "error": str(exc)}
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 1 if result.get("admission_status") == "Invalid" else 0

if __name__ == "__main__":
    raise SystemExit(main())
