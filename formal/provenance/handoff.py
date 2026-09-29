#!/usr/bin/env python3
"""Bounded handoff from PKG admission to the existing qualification authority."""

from __future__ import annotations

import argparse
import json
from typing import Any

from admission import validate_admission
from validate import SCHEMA_VERSION, Validator

HANDOFF_VERSION = "luminous.formal-authority-handoff.v0"


def _node_map(graph: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {node["id"]: node for node in graph["nodes"]}


def _binding(
    envelope: dict[str, Any],
    nodes: dict[str, dict[str, Any]],
    field: str,
    expected_kind: str,
) -> tuple[str, str]:
    matches: list[dict[str, Any]] = []
    for record in envelope["bindings"]:
        for reference in record["references"]:
            if reference["field"] == field:
                matches.append(reference)
    if len(matches) != 1:
        raise ValueError(f"handoff requires exactly one {field} binding")
    reference = matches[0]
    node_id = reference["node_id"]
    if node_id not in nodes or nodes[node_id]["kind"] != expected_kind:
        raise ValueError(f"handoff {field} binding has wrong node kind")
    if reference["identity_digest"] != _identity_digest(nodes[node_id]):
        raise ValueError(f"handoff {field} identity digest mismatch")
    return node_id, reference["identity_digest"]


def _identity_digest(node: dict[str, Any]) -> str:
    metadata = node.get("metadata")
    value = (
        metadata.get("semantic_digest")
        if node["kind"] in {"Theorem", "Lemma", "Invariant"} and isinstance(metadata, dict)
        else node.get("content_digest")
    )
    if not isinstance(value, str) or len(value) != 64:
        raise ValueError(f"node {node['id']} lacks an exact identity/content digest")
    return value


def build_handoff(
    graph: dict[str, Any],
    impact: dict[str, Any],
    envelope: dict[str, Any],
) -> dict[str, Any]:
    if Validator(graph).validate()["status"] != "Valid":
        raise ValueError("input provenance graph is not semantically valid")

    validated = validate_admission(graph, impact, envelope)
    nodes = _node_map(graph)

    evidence_records = [
        record for record in validated["bindings"]
        if record["kind"] == "QualificationEvidence"
    ]
    if len(evidence_records) != 1:
        raise ValueError("handoff requires exactly one qualification evidence binding")

    evidence_id = evidence_records[0]["node_id"]
    evidence = nodes[evidence_id]
    metadata = evidence.get("metadata")
    if not isinstance(metadata, dict):
        raise ValueError("qualification evidence lacks metadata")

    execution_id, execution_digest = _binding(
        validated, nodes, "execution_id", "ProofExecution"
    )
    contract_id, contract_digest = _binding(
        validated, nodes, "contract_id", "QualificationContract"
    )
    verifier_release_id, verifier_release_digest = _binding(
        validated, nodes, "verifier_release_id", "VerifierRelease"
    )

    # This object is deliberately declarative. It contains no command, workflow,
    # ref-selection, or executable policy supplied by the provenance graph.
    handoff = {
        "schema_version": SCHEMA_VERSION,
        "handoff_version": HANDOFF_VERSION,
        "admission_digest": validated["admission_digest"],
        "expected_result": validated["expected_result"],
        "subject": dict(validated["subject"]),
        "authority_binding": {
            "contract_id": contract_id,
            "contract_digest": contract_digest,
            "verifier_release_id": verifier_release_id,
            "verifier_release_digest": verifier_release_digest,
            "execution_id": execution_id,
            "execution_digest": execution_digest,
        },
    }
    return handoff


def main() -> int:
    parser = argparse.ArgumentParser(description="Build a bounded qualification authority handoff")
    parser.add_argument("graph")
    parser.add_argument("impact")
    parser.add_argument("envelope")
    args = parser.parse_args()

    try:
        with open(args.graph, encoding="utf-8") as handle:
            graph = json.load(handle)
        with open(args.impact, encoding="utf-8") as handle:
            impact = json.load(handle)
        with open(args.envelope, encoding="utf-8") as handle:
            envelope = json.load(handle)
        result = build_handoff(graph, impact, envelope)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError, KeyError, TypeError) as exc:
        result = {
            "schema_version": SCHEMA_VERSION,
            "handoff_version": HANDOFF_VERSION,
            "handoff_status": "Invalid",
            "error": str(exc),
        }

    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 1 if result.get("handoff_status") == "Invalid" else 0


if __name__ == "__main__":
    raise SystemExit(main())
