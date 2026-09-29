#!/usr/bin/env python3
"""Deterministic fail-closed revalidation planning for PKG v0."""

from __future__ import annotations

import argparse
import json
from typing import Any

from validate import SCHEMA_VERSION, Validator

PLAN_VERSION = "luminous.formal-revalidation-plan.v0"

REQUIRED_KINDS = {
    "Theorem",
    "Lemma",
    "Invariant",
    "ProofArtifact",
    "ProofChecker",
    "ProofExecution",
    "SourceCommit",
    "SourceTree",
    "VerifierRelease",
    "QualificationContract",
    "QualificationEvidence",
}

REFERENCE_FIELDS = (
    "artifact_id",
    "checker_id",
    "source_commit_id",
    "source_tree_id",
    "execution_id",
    "contract_id",
    "verifier_release_id",
    "subject_commit_id",
    "subject_tree_id",
)


def _validate_graph(graph: dict[str, Any]) -> None:
    result = Validator(graph).validate()
    if result["status"] != "Valid":
        raise ValueError(
            "input provenance graph is not semantically valid: "
            + json.dumps(result, sort_keys=True, separators=(",", ":"))
        )


def _history_invalidations(graph: dict[str, Any]) -> tuple[set[str], set[str]]:
    superseded: set[str] = set()
    invalidated: set[str] = set()
    for edge in graph["edges"]:
        if edge["relation"] == "supersedes":
            superseded.add(edge["target"])
        elif edge["relation"] == "invalidated_by":
            invalidated.add(edge["target"])
    return superseded, invalidated


def _bindings(node: dict[str, Any], nodes: dict[str, dict[str, Any]]) -> tuple[list[str], list[str]]:
    metadata = node.get("metadata")
    if not isinstance(metadata, dict):
        return [], []

    missing: list[str] = []
    contradictory: list[str] = []
    for field in REFERENCE_FIELDS:
        if field not in metadata:
            continue
        ref = metadata[field]
        if not isinstance(ref, str) or ref not in nodes:
            missing.append(field)
            continue
        target = nodes[ref]
        expected = {
            "artifact_id": "ProofArtifact",
            "checker_id": "ProofChecker",
            "source_commit_id": "SourceCommit",
            "source_tree_id": "SourceTree",
            "execution_id": "ProofExecution",
            "contract_id": "QualificationContract",
            "verifier_release_id": "VerifierRelease",
            "subject_commit_id": "SourceCommit",
            "subject_tree_id": "SourceTree",
        }[field]
        if target["kind"] != expected:
            contradictory.append(
                f"{field}:expected:{expected}:actual:{target['kind']}"
            )

    return sorted(missing), sorted(contradictory)


def build_plan(graph: dict[str, Any], impact: dict[str, Any]) -> dict[str, Any]:
    _validate_graph(graph)
    if impact.get("schema_version") != SCHEMA_VERSION:
        raise ValueError("impact schema_version does not match provenance schema")
    if impact.get("impact_status") == "GraphInvalid":
        raise ValueError("cannot plan revalidation from an invalid impact result")
    if impact.get("impact_version") != "luminous.formal-impact.v0":
        raise ValueError("unsupported impact_version")

    nodes = {node["id"]: node for node in graph["nodes"]}
    superseded, invalidated = _history_invalidations(graph)
    records = impact.get("records")
    if not isinstance(records, list):
        raise ValueError("impact records must be an array")

    plans: list[dict[str, Any]] = []
    blockers: list[dict[str, Any]] = []

    for record in sorted(records, key=lambda item: item.get("node_id", "")):
        node_id = record.get("node_id")
        if not isinstance(node_id, str) or node_id not in nodes:
            raise ValueError("impact record references an unknown node")

        node = nodes[node_id]
        missing, contradictory = _bindings(node, nodes)
        historical = []
        if node_id in superseded:
            historical.append("SUPERSEDED")
        if node_id in invalidated:
            historical.append("INVALIDATED")

        if missing or contradictory or historical:
            blocker = {
                "node_id": node_id,
                "kind": node["kind"],
                "status": "Blocked",
                "missing_bindings": missing,
                "contradictory_bindings": contradictory,
                "historical_flags": sorted(historical),
            }
            blockers.append(blocker)
            continue

        action = (
            "RECHECK_PROOF"
            if node["kind"] in {"Theorem", "Lemma", "Invariant", "ProofArtifact", "ProofChecker"}
            else "RECHECK_EXECUTION"
            if node["kind"] == "ProofExecution"
            else "RECHECK_QUALIFICATION_EVIDENCE"
            if node["kind"] == "QualificationEvidence"
            else "RECHECK_DEPENDENCY"
        )

        plans.append(
            {
                "node_id": node_id,
                "kind": node["kind"],
                "action": action,
                "reason_code": record.get("reason_code", "DEPENDENCY_CHANGED"),
                "via_edges": list(record.get("via_edges", [])),
            }
        )

    status = "Blocked" if blockers else "Ready"
    if not records:
        status = "NoRevalidationRequired"

    return {
        "schema_version": SCHEMA_VERSION,
        "plan_version": PLAN_VERSION,
        "plan_status": status,
        "records": plans,
        "blockers": blockers,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Build a PKG revalidation plan")
    parser.add_argument("graph")
    parser.add_argument("impact")
    args = parser.parse_args()

    try:
        with open(args.graph, encoding="utf-8") as handle:
            graph = json.load(handle)
        with open(args.impact, encoding="utf-8") as handle:
            impact = json.load(handle)
        result = build_plan(graph, impact)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as exc:
        result = {
            "schema_version": SCHEMA_VERSION,
            "plan_version": PLAN_VERSION,
            "plan_status": "Invalid",
            "records": [],
            "blockers": [],
            "error": str(exc),
        }

    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 0 if result["plan_status"] == "Invalid" else 0


if __name__ == "__main__":
    raise SystemExit(main())
