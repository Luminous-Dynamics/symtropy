#!/usr/bin/env python3
"""Deterministic dependency-aware proof impact analysis for PKG v0.

Impact analysis is advisory only. It identifies records that require
revalidation; it never establishes or withdraws qualification authority.
"""

from __future__ import annotations

import argparse
import json
from collections import defaultdict, deque
from typing import Any

from validate import SCHEMA_VERSION, Validator

IMPACT_VERSION = "luminous.formal-impact.v0"

NODE_REASONS = {
    "Theorem": "SEMANTIC_DEPENDENCY_CHANGED",
    "Lemma": "SEMANTIC_DEPENDENCY_CHANGED",
    "Invariant": "SEMANTIC_DEPENDENCY_CHANGED",
    "ProofArtifact": "PROOF_ARTIFACT_CHANGED",
    "ProofChecker": "CHECKER_CHANGED",
    "ProofExecution": "EXECUTION_CHANGED",
    "SourceCommit": "SOURCE_DEPENDENCY_CHANGED",
    "SourceTree": "SOURCE_DEPENDENCY_CHANGED",
    "VerifierRelease": "VERIFIER_RELEASE_CHANGED",
    "QualificationContract": "QUALIFICATION_CONTRACT_CHANGED",
    "QualificationEvidence": "EVIDENCE_STALE",
}

EDGE_REASONS = {
    "proves": "SEMANTIC_DEPENDENCY_CHANGED",
    "depends_on": "SHARED_SUBPROOF_CHANGED",
    "checked_by": "CHECKER_CHANGED",
    "qualifies": "QUALIFICATION_CONTRACT_CHANGED",
    "revalidated_by": "EVIDENCE_STALE",
    "supersedes": "EVIDENCE_STALE",
    "invalidated_by": "EVIDENCE_STALE",
}

# These references are dependency edges even when the graph does not encode
# them as first-class edge records. subject_* fields are optional v0 metadata
# bindings for exact qualification subjects.
METADATA_REASONS = {
    "artifact_id": "PROOF_ARTIFACT_CHANGED",
    "checker_id": "CHECKER_CHANGED",
    "source_commit_id": "SOURCE_DEPENDENCY_CHANGED",
    "source_tree_id": "SOURCE_DEPENDENCY_CHANGED",
    "execution_id": "EVIDENCE_STALE",
    "contract_id": "QUALIFICATION_CONTRACT_CHANGED",
    "verifier_release_id": "VERIFIER_RELEASE_CHANGED",
    "subject_commit_id": "SOURCE_DEPENDENCY_CHANGED",
    "subject_tree_id": "SOURCE_DEPENDENCY_CHANGED",
}


def _validate(graph: dict[str, Any]) -> None:
    result = Validator(graph).validate()
    if result["status"] != "Valid":
        raise ValueError(
            "input provenance graph is not semantically valid: "
            + json.dumps(result, sort_keys=True, separators=(",", ":"))
        )


def _build_reverse_dependencies(
    graph: dict[str, Any], nodes: dict[str, dict[str, Any]]
) -> dict[str, list[tuple[str, str, str, str]]]:
    reverse: dict[str, list[tuple[str, str, str, str]]] = defaultdict(list)

    for edge in graph["edges"]:
        reverse[edge["target"]].append(
            (
                edge["source"],
                edge["relation"],
                edge["id"],
                EDGE_REASONS.get(edge["relation"], "DEPENDENCY_CHANGED"),
            )
        )

    for node in graph["nodes"]:
        metadata = node.get("metadata")
        if not isinstance(metadata, dict):
            continue
        for field, reason in METADATA_REASONS.items():
            ref = metadata.get(field)
            if isinstance(ref, str) and ref in nodes:
                reverse[ref].append(
                    (node["id"], f"metadata:{field}", field, reason)
                )

    for dependencies in reverse.values():
        dependencies.sort(key=lambda item: (item[0], item[1], item[2]))

    return reverse


def analyze(graph: dict[str, Any], changed_ids: list[str]) -> dict[str, Any]:
    _validate(graph)
    nodes = {node["id"]: node for node in graph["nodes"]}
    changed = sorted(set(changed_ids))
    unknown = sorted(set(changed) - set(nodes))
    if unknown:
        raise ValueError("unknown changed node ids: " + ",".join(unknown))

    reverse = _build_reverse_dependencies(graph, nodes)

    # BFS gives a deterministic shortest dependency path because every
    # adjacency list is sorted. The first path wins when several paths reach
    # the same node; this keeps output stable without inventing authority.
    queue = deque()
    for node_id in changed:
        queue.append(
            (
                node_id,
                NODE_REASONS.get(
                    nodes[node_id]["kind"], "DEPENDENCY_CHANGED"
                ),
                [],
            )
        )

    visited: set[str] = set()
    records: dict[str, dict[str, Any]] = {}

    while queue:
        current, reason, path = queue.popleft()
        if current in visited:
            continue
        visited.add(current)

        if current not in changed:
            records[current] = {
                "node_id": current,
                "reason_code": reason,
                "via_edges": path,
            }

        for nxt, relation, edge_id, edge_reason in reverse.get(current, []):
            if nxt not in visited:
                queue.append(
                    (nxt, edge_reason, path + [edge_id])
                )

    affected = [records[node_id] for node_id in sorted(records)]
    return {
        "schema_version": SCHEMA_VERSION,
        "impact_version": IMPACT_VERSION,
        "impact_status": "RevalidationRequired" if affected else "NoImpact",
        "changed_node_ids": changed,
        "records": affected,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Analyze PKG proof impact")
    parser.add_argument("graph")
    parser.add_argument("--changed", nargs="+", required=True)
    args = parser.parse_args()

    try:
        with open(args.graph, encoding="utf-8") as handle:
            graph = json.load(handle)
        result = analyze(graph, args.changed)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as exc:
        result = {
            "schema_version": SCHEMA_VERSION,
            "impact_version": IMPACT_VERSION,
            "impact_status": "GraphInvalid",
            "changed_node_ids": sorted(set(args.changed)),
            "records": [],
            "error": str(exc),
        }

    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 0 if result["impact_status"] != "GraphInvalid" else 1


if __name__ == "__main__":
    raise SystemExit(main())
