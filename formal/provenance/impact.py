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

from validate import Validator, SCHEMA_VERSION

IMPACT_VERSION = "luminous.formal-impact.v0"

REASONS = {
    "Theorem": "SEMANTIC_DEPENDENCY_CHANGED",
    "Lemma": "SEMANTIC_DEPENDENCY_CHANGED",
    "Invariant": "SEMANTIC_DEPENDENCY_CHANGED",
    "ProofArtifact": "PROOF_ARTIFACT_CHANGED",
    "ProofChecker": "CHECKER_CHANGED",
    "VerifierRelease": "VERIFIER_RELEASE_CHANGED",
    "QualificationContract": "QUALIFICATION_CONTRACT_CHANGED",
    "QualificationEvidence": "EVIDENCE_STALE",
    "ProofExecution": "PROOF_ARTIFACT_CHANGED",
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

METADATA_FIELDS = {
    "artifact_id": "PROOF_ARTIFACT_CHANGED",
    "checker_id": "CHECKER_CHANGED",
    "source_commit_id": "PROOF_ARTIFACT_CHANGED",
    "source_tree_id": "PROOF_ARTIFACT_CHANGED",
    "execution_id": "EVIDENCE_STALE",
    "contract_id": "QUALIFICATION_CONTRACT_CHANGED",
    "verifier_release_id": "VERIFIER_RELEASE_CHANGED",
}


def _validate(graph: dict[str, Any]) -> None:
    result = Validator(graph).validate()
    if result["status"] != "Valid":
        raise ValueError(
            "input provenance graph is not semantically valid: "
            + json.dumps(result, sort_keys=True, separators=(",", ":"))
        )


def analyze(graph: dict[str, Any], changed_ids: list[str]) -> dict[str, Any]:
    _validate(graph)
    nodes = {node["id"]: node for node in graph["nodes"]}
    unknown = sorted(set(changed_ids) - set(nodes))
    if unknown:
        raise ValueError("unknown changed node ids: " + ",".join(unknown))

    reverse: dict[str, list[tuple[str, str, str]]] = defaultdict(list)
    for edge in graph["edges"]:
        reverse[edge["target"]].append(
            (edge["source"], edge["relation"], edge["id"])
        )

    for node in graph["nodes"]:
        metadata = node.get("metadata")
        if not isinstance(metadata, dict):
            continue
        for field, reason in METADATA_FIELDS.items():
            ref = metadata.get(field)
            if isinstance(ref, str) and ref in nodes:
                reverse[ref].append((node["id"], f"metadata:{field}", field))

    queue = deque(sorted(set(changed_ids)))
    seen: dict[str, dict[str, Any]] = {}

    while queue:
        current = queue.popleft()
        if current in seen:
            continue
        node = nodes[current]
        if current in changed_ids:
            reason = REASONS.get(node["kind"], "PROOF_ARTIFACT_CHANGED")
            via = []
        else:
            reason = "GRAPH_INVALID"
            via = []
        seen[current] = {
            "node_id": current,
            "reason_code": reason,
            "via_edges": via,
        }
        for nxt, relation, edge_id in sorted(
            reverse[current], key=lambda item: (item[0], item[1], item[2])
        ):
            if nxt not in seen:
                inherited = EDGE_REASONS.get(
                    relation,
                    next(
                        (value for key, value in METADATA_FIELDS.items() if relation == f"metadata:{key}"),
                        "PROOF_ARTIFACT_CHANGED",
                    ),
                )
                queue.append(nxt)
                # Record the traversal reason once the node is emitted.
                if nxt not in seen:
                    pass

    # Recompute records deterministically with shortest provenance path.
    records: dict[str, dict[str, Any]] = {}
    queue = deque((node_id, REASONS.get(nodes[node_id]["kind"], "PROOF_ARTIFACT_CHANGED"), []) for node_id in sorted(set(changed_ids)))
    visited: set[str] = set()
    while queue:
        current, reason, path = queue.popleft()
        if current in visited:
            continue
        visited.add(current)
        records[current] = {
            "node_id": current,
            "reason_code": reason,
            "via_edges": path,
        }
        for nxt, relation, edge_id in sorted(
            reverse[current], key=lambda item: (item[0], item[1], item[2])
        ):
            if nxt not in visited:
                next_reason = EDGE_REASONS.get(
                    relation,
                    next(
                        (value for key, value in METADATA_FIELDS.items() if relation == f"metadata:{key}"),
                        reason,
                    ),
                )
                queue.append((nxt, next_reason, path + [edge_id]))

    affected = [records[node_id] for node_id in sorted(records) if node_id not in set(changed_ids)]
    return {
        "schema_version": SCHEMA_VERSION,
        "impact_version": IMPACT_VERSION,
        "impact_status": "RevalidationRequired" if affected else "NoImpact",
        "changed_node_ids": sorted(set(changed_ids)),
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
