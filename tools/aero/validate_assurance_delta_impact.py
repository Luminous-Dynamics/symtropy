#!/usr/bin/env python3
"""CI-only independent oracle for AERO assurance delta impact v0.1.

Rust is the production evaluator. This script independently reconstructs the
graph commitment, scope/chronology checks and dependency-closure dispositions.
It intentionally does not authenticate owner signatures or physical evidence.
"""
from __future__ import annotations

import copy
import hashlib
import json
import sys
from collections import deque
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[2]
AVIATION = ROOT / "docs" / "research" / "aviation"
SCHEMA_PATH = AVIATION / "schemas" / "aero_assurance_delta_impact_v0_1.schema.json"
EXAMPLE_PATH = AVIATION / "examples" / "aero_assurance_delta_impact_v0_1.json"


def digest_json(value: Any) -> str:
    payload = json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def graph_sha256(graph: dict[str, Any]) -> str:
    source_nodes = sorted(graph["nodes"], key=lambda item: item["node_id"])
    nodes = [
        {
            "node_id": item["node_id"],
            "owner_namespace": item["owner_namespace"],
            "kind": item["kind"],
            "content_sha256": item["content_sha256"],
            "dependency_manifest_complete": item["dependency_manifest_complete"],
        }
        for item in source_nodes
    ]
    edges = sorted(
        graph["edges"],
        key=lambda item: (item["dependent_node_id"], item["dependency_node_id"]),
    )
    return digest_json({
        "schema": graph["schema"],
        "canonicalization_profile": "rust-serde-json-struct-order-v1",
        "snapshot_id": graph["snapshot_id"],
        "claim_id": graph["claim_id"],
        "configuration_ref": graph["configuration_ref"],
        "coverage_status": graph["coverage_status"],
        "nodes": nodes,
        "edges": edges,
    })


def closure(start: str, adjacency: dict[str, list[str]]) -> set[str]:
    seen = {start}
    queue = deque([start])
    while queue:
        current = queue.popleft()
        for dependency in adjacency.get(current, []):
            if dependency not in seen:
                seen.add(dependency)
                queue.append(dependency)
    return seen


def path_to(start: str, target: str, adjacency: dict[str, list[str]]) -> list[str] | None:
    queue = deque([start])
    parents: dict[str, str | None] = {start: None}
    while queue:
        current = queue.popleft()
        if current == target:
            result: list[str] = []
            cursor: str | None = current
            while cursor is not None:
                result.append(cursor)
                cursor = parents[cursor]
            return list(reversed(result))
        for dependency in adjacency.get(current, []):
            if dependency not in parents:
                parents[dependency] = current
                queue.append(dependency)
    return None


def validate(source: dict[str, Any]) -> tuple[list[str], dict[str, Any]]:
    errors: list[str] = []
    graph = source["graph"]
    approval = source["graph_approval_receipt"]
    change = source["change_receipt"]
    cutoff = source["as_known_at_unix_ms"]

    if source["claim_id"] != graph["claim_id"] or source["configuration_ref"] != graph["configuration_ref"]:
        errors.append("top-level scope differs from the graph scope")
    if graph_sha256(graph) != source["graph_snapshot_sha256"]:
        errors.append("graph snapshot digest mismatch")

    nodes = graph["nodes"]
    ids = [node["node_id"] for node in nodes]
    node_map = {node["node_id"]: node for node in nodes}
    if len(ids) != len(set(ids)):
        errors.append("node IDs are duplicated")
    if any(not node.get("owner_namespace") for node in nodes):
        errors.append("a dependency node has no owner namespace")
    claim_roots = [node for node in nodes if node["kind"] == "CLAIM"]
    if len(claim_roots) != 1 or claim_roots[0]["node_id"] != source["claim_id"]:
        errors.append("graph must have exactly one matching claim root")

    edges = [(edge["dependent_node_id"], edge["dependency_node_id"]) for edge in graph["edges"]]
    if len(edges) != len(set(edges)):
        errors.append("dependency edges are duplicated")
    for dependent, dependency in edges:
        if dependent not in node_map or dependency not in node_map or dependent == dependency:
            errors.append("dependency edge has a missing endpoint or self-reference")

    if approval["verification_status"] != "VERIFIED":
        errors.append("graph approval receipt is not upstream-verified")
    if (
        approval["claim_id"] != source["claim_id"]
        or approval["configuration_ref"] != source["configuration_ref"]
        or approval["graph_snapshot_sha256"] != source["graph_snapshot_sha256"]
    ):
        errors.append("graph approval receipt scope mismatch")
    if approval["receiver_first_seen_at_unix_ms"] > cutoff:
        errors.append("graph approval was not known at the selected cutoff")
    if not approval["effective_from_unix_ms"] <= cutoff < approval["effective_until_unix_ms"]:
        errors.append("graph approval is not effective at the selected cutoff")

    if change["verification_status"] != "VERIFIED":
        errors.append("change receipt is not upstream-verified")
    if (
        change["claim_id"] != source["claim_id"]
        or change["configuration_ref"] != source["configuration_ref"]
    ):
        errors.append("change receipt scope mismatch")
    if change["receiver_first_seen_at_unix_ms"] > cutoff:
        errors.append("change receipt was not known at the selected cutoff")
    if not change["effective_from_unix_ms"] <= cutoff < change["effective_until_unix_ms"]:
        errors.append("change receipt is not effective at the selected cutoff")
    if change["previous_content_sha256"] == change["replacement_content_sha256"]:
        errors.append("change receipt does not change the content digest")
    changed_node = node_map.get(change["target_node_id"])
    if changed_node is None:
        errors.append("changed node is not in the dependency graph")
    elif changed_node["content_sha256"] != change["previous_content_sha256"]:
        errors.append("previous content digest does not match the graph node")
    elif changed_node["owner_namespace"] != change["owner_namespace"]:
        errors.append("change receipt owner does not own the changed node")

    if errors:
        return errors, {
            "disposition": "INVALID_INPUT",
            "affected": [],
            "retained": [],
            "incomplete": [],
        }

    adjacency: dict[str, list[str]] = {}
    for dependent, dependency in edges:
        adjacency.setdefault(dependent, []).append(dependency)
    for deps in adjacency.values():
        deps.sort()

    claim_closure = closure(source["claim_id"], adjacency)
    claim_root_complete = node_map[source["claim_id"]]["dependency_manifest_complete"]
    obligations = sorted(
        (node for node in nodes if node["kind"] == "OBLIGATION"),
        key=lambda item: item["node_id"],
    )
    affected: list[dict[str, Any]] = []
    retained: list[dict[str, Any]] = []
    incomplete: list[str] = []

    for obligation in obligations:
        obligation_id = obligation["node_id"]
        if obligation_id not in claim_closure:
            incomplete.append(obligation_id)
            continue
        dependency_path = path_to(obligation_id, change["target_node_id"], adjacency)
        if dependency_path is not None:
            affected.append({
                "obligation_id": obligation_id,
                "dependency_path": dependency_path,
            })
            continue

        node_closure = closure(obligation_id, adjacency)
        is_complete = (
            graph["coverage_status"] == "COMPLETE"
            and claim_root_complete
            and obligation_id in claim_closure
            and all(node_map[node_id]["dependency_manifest_complete"] for node_id in node_closure)
        )
        if is_complete:
            manifest = {
                "schema": "AeroRetainedDependencyClosureV0_1",
                "graph_snapshot_sha256": source["graph_snapshot_sha256"],
                "obligation_id": obligation_id,
                "nodes": [
                    {"node_id": node_id, "content_sha256": node_map[node_id]["content_sha256"]}
                    for node_id in sorted(node_closure)
                ],
                "edges": [
                    {"dependent_node_id": dependent, "dependency_node_id": dependency}
                    for dependent, dependency in sorted(edges)
                    if dependent in node_closure and dependency in node_closure
                ],
            }
            retained.append({
                "obligation_id": obligation_id,
                "dependency_node_ids": sorted(node_closure),
                "retained_dependency_closure_sha256": digest_json(manifest),
            })
        else:
            incomplete.append(obligation_id)

    if incomplete:
        disposition = "INCOMPLETE_IMPACT"
    elif affected:
        disposition = "IMPACT_IDENTIFIED"
    else:
        disposition = "NO_AFFECTED_OBLIGATIONS"
    return [], {
        "disposition": disposition,
        "affected": affected,
        "retained": retained,
        "incomplete": incomplete,
    }


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

    errors, result = validate(instance)
    if errors:
        for error in errors:
            print(f"CONTRACT FAIL: {error}")
        return 1
    if result["disposition"] != "IMPACT_IDENTIFIED":
        raise SystemExit("FAIL: synthetic complete graph did not identify the changed material-lot obligation")
    if [item["obligation_id"] for item in result["affected"]] != ["obligation:material-lot"]:
        raise SystemExit("FAIL: synthetic affected-obligation set differs from expected")
    if [item["obligation_id"] for item in result["retained"]] != ["obligation:solver-validation"]:
        raise SystemExit("FAIL: disjoint, fully closed obligation was not retained")

    late_change = copy.deepcopy(instance)
    late_change["change_receipt"]["receiver_first_seen_at_unix_ms"] = (
        late_change["as_known_at_unix_ms"] + 1
    )
    late_errors, _ = validate(late_change)
    if not any("not known at the selected cutoff" in error for error in late_errors):
        raise SystemExit("FAIL: late change receipt was accepted")

    previous_digest = copy.deepcopy(instance)
    previous_digest["change_receipt"]["previous_content_sha256"] = "f" * 64
    digest_errors, _ = validate(previous_digest)
    if not any("previous content digest" in error for error in digest_errors):
        raise SystemExit("FAIL: source digest substitution was accepted")

    duplicate_edge = copy.deepcopy(instance)
    duplicate_edge["graph"]["edges"].append(copy.deepcopy(duplicate_edge["graph"]["edges"][0]))
    duplicate_edge["graph_snapshot_sha256"] = graph_sha256(duplicate_edge["graph"])
    duplicate_edge["graph_approval_receipt"]["graph_snapshot_sha256"] = duplicate_edge["graph_snapshot_sha256"]
    duplicate_errors, _ = validate(duplicate_edge)
    if not any("duplicated" in error for error in duplicate_errors):
        raise SystemExit("FAIL: duplicate dependency edge was accepted")

    wrong_owner = copy.deepcopy(instance)
    wrong_owner["change_receipt"]["owner_namespace"] = "owner:unrelated-authority"
    owner_errors, _ = validate(wrong_owner)
    if not any("does not own the changed node" in error for error in owner_errors):
        raise SystemExit("FAIL: a different owner changed the source node")

    orphan_obligation = copy.deepcopy(instance)
    orphan_obligation["graph"]["edges"] = [
        edge for edge in orphan_obligation["graph"]["edges"]
        if not (
            edge["dependent_node_id"] == orphan_obligation["claim_id"]
            and edge["dependency_node_id"] == "obligation:material-lot"
        )
    ]
    orphan_obligation["graph_snapshot_sha256"] = graph_sha256(orphan_obligation["graph"])
    orphan_obligation["graph_approval_receipt"]["graph_snapshot_sha256"] = orphan_obligation["graph_snapshot_sha256"]
    orphan_errors, orphan_result = validate(orphan_obligation)
    if orphan_errors or orphan_result["disposition"] != "INCOMPLETE_IMPACT":
        raise SystemExit("FAIL: orphan obligation was not classified as incomplete impact")
    if orphan_result["affected"]:
        raise SystemExit("FAIL: obligation outside the claim closure was incorrectly marked affected")

    incomplete_root = copy.deepcopy(instance)
    root_node = next(
        node for node in incomplete_root["graph"]["nodes"]
        if node["kind"] == "CLAIM"
    )
    root_node["dependency_manifest_complete"] = False
    incomplete_root["graph_snapshot_sha256"] = graph_sha256(incomplete_root["graph"])
    incomplete_root["graph_approval_receipt"]["graph_snapshot_sha256"] = incomplete_root["graph_snapshot_sha256"]
    root_errors, root_result = validate(incomplete_root)
    if root_errors or root_result["disposition"] != "INCOMPLETE_IMPACT" or root_result["retained"]:
        raise SystemExit("FAIL: incomplete claim-root dependency manifest allowed retained status")

    incomplete_graph = copy.deepcopy(instance)
    incomplete_graph["graph"]["coverage_status"] = "INCOMPLETE"
    incomplete_graph["graph_snapshot_sha256"] = graph_sha256(incomplete_graph["graph"])
    incomplete_graph["graph_approval_receipt"]["graph_snapshot_sha256"] = incomplete_graph["graph_snapshot_sha256"]
    incomplete_errors, incomplete_result = validate(incomplete_graph)
    if incomplete_errors or incomplete_result["disposition"] != "INCOMPLETE_IMPACT":
        raise SystemExit("FAIL: incomplete graph did not produce INCOMPLETE_IMPACT")
    if incomplete_result["retained"]:
        raise SystemExit("FAIL: incomplete dependency graph retained an obligation")

    permuted = copy.deepcopy(instance)
    permuted["graph"]["nodes"].reverse()
    permuted["graph"]["edges"].reverse()
    permuted["graph_snapshot_sha256"] = graph_sha256(permuted["graph"])
    permuted["graph_approval_receipt"]["graph_snapshot_sha256"] = permuted["graph_snapshot_sha256"]
    permuted_errors, permuted_result = validate(permuted)
    if permuted_errors or permuted_result != result:
        raise SystemExit("FAIL: node/edge order permutation changed the impact result")

    expanded = copy.deepcopy(instance)
    expanded["graph"]["edges"].append({
        "dependent_node_id": "obligation:solver-validation",
        "dependency_node_id": "source:material-lot-record",
    })
    expanded["graph_snapshot_sha256"] = graph_sha256(expanded["graph"])
    expanded["graph_approval_receipt"]["graph_snapshot_sha256"] = expanded["graph_snapshot_sha256"]
    expanded_errors, expanded_result = validate(expanded)
    if expanded_errors:
        raise SystemExit("FAIL: expanded graph did not validate")
    before = {item["obligation_id"] for item in result["affected"]}
    after = {item["obligation_id"] for item in expanded_result["affected"]}
    if not before.issubset(after):
        raise SystemExit("FAIL: adding a dependency reduced the affected-obligation set")

    print(
        "PASS: AERO assurance delta v0.1 schema, temporal/source binding, graph commitment, "
        "impact closure, retained-closure digest, and 8 mutation/permutation controls."
    )
    print(
        "NOTE: synthetic receipts are placeholders; VERIFIED is an upstream disposition. "
        "This oracle does not authenticate signatures, graph completeness, engineering truth or physical qualification."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
