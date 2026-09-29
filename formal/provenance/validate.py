#!/usr/bin/env python3
"""Fail-closed semantic validator for Symtropy formal provenance graph v0.

This validator intentionally stays independent of the runtime crates. JSON shape
validation is complemented by cross-record identity, evidence, and DAG checks.
It does not infer authority from execution success.
"""

from __future__ import annotations

import argparse
import json
import re
from collections import defaultdict
from pathlib import PurePosixPath
from dataclasses import dataclass
from typing import Any

SCHEMA_VERSION = "luminous.formal-provenance.v0"
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
GIT_OID_RE = re.compile(r"^(?:[0-9a-f]{40}|[0-9a-f]{64})$")
NODE_KINDS = {
    "Invariant", "Definition", "Theorem", "Lemma", "ProofArtifact",
    "ProofChecker", "ProofExecution", "Counterexample", "SourceCommit",
    "SourceTree", "QualificationContract", "QualificationEvidence",
    "VerifierRelease",
}
RELATIONS = {
    "defines", "refines", "depends_on", "proves", "checked_by",
    "derived_from", "falsified_by", "revalidated_by", "qualifies",
    "supersedes", "invalidated_by",
}
EXECUTION_STATUSES = {
    "Queued", "Running", "Passed", "Failed", "Cancelled",
    "InfrastructureFailure", "Skipped", "Superseded", "Stale",
}


@dataclass(frozen=True)
class Diagnostic:
    code: str
    message: str
    path: str

    def as_dict(self) -> dict[str, str]:
        return {"code": self.code, "path": self.path, "message": self.message}


class Validator:
    def __init__(self, graph: dict[str, Any]) -> None:
        self.graph = graph
        self.errors: list[Diagnostic] = []
        self.nodes: dict[str, dict[str, Any]] = {}
        self.edges: dict[str, dict[str, Any]] = {}

    def error(self, code: str, path: str, message: str) -> None:
        self.errors.append(Diagnostic(code, message, path))

    def validate(self) -> dict[str, Any]:
        self._shape()
        if self.errors:
            return self.result()

        self._index()
        self._relationships()
        self._semantic_bindings()
        self._proof_dag()
        self._qualification()
        return self.result()

    def result(self) -> dict[str, Any]:
        status = "Valid" if not self.errors else "GraphInvalid"
        if any(e.code.startswith("E_SCHEMA") for e in self.errors):
            status = "SchemaInvalid"
        elif any(e.code.startswith("E_EVIDENCE") for e in self.errors):
            status = "EvidenceInvalid"
        elif any(e.code.startswith("E_AUTHORITY") for e in self.errors):
            status = "AuthorityNotEstablished"
        return {
            "schema_version": SCHEMA_VERSION,
            "status": status,
            "errors": [e.as_dict() for e in self.errors],
            "authority": "NotEstablished" if any(
                e.code.startswith("E_AUTHORITY") for e in self.errors
            ) else "Unchanged",
        }

    def _shape(self) -> None:
        if not isinstance(self.graph, dict):
            self.error("E_SCHEMA_ROOT", "$", "graph must be an object")
            return
        if self.graph.get("schema_version") != SCHEMA_VERSION:
            self.error("E_SCHEMA_VERSION", "$.schema_version", "unsupported schema version")
        for key in ("nodes", "edges"):
            if not isinstance(self.graph.get(key), list):
                self.error("E_SCHEMA_ROOT", f"$.{key}", "must be an array")
        if self.errors:
            return
        for i, node in enumerate(self.graph["nodes"]):
            if not isinstance(node, dict):
                self.error("E_SCHEMA_NODE", f"$.nodes[{i}]", "node must be an object")
                continue
            if node.get("schema_version") != SCHEMA_VERSION:
                self.error("E_SCHEMA_VERSION", f"$.nodes[{i}].schema_version", "invalid schema version")
            if node.get("kind") not in NODE_KINDS:
                self.error("E_SCHEMA_NODE", f"$.nodes[{i}].kind", "unknown node kind")
            if not isinstance(node.get("id"), str) or not node["id"]:
                self.error("E_SCHEMA_NODE", f"$.nodes[{i}].id", "node id must be non-empty")
            if not isinstance(node.get("label"), str) or not node["label"]:
                self.error("E_SCHEMA_NODE", f"$.nodes[{i}].label", "node label must be non-empty")
        for i, edge in enumerate(self.graph["edges"]):
            if not isinstance(edge, dict):
                self.error("E_SCHEMA_EDGE", f"$.edges[{i}]", "edge must be an object")
                continue
            if edge.get("schema_version") != SCHEMA_VERSION:
                self.error("E_SCHEMA_VERSION", f"$.edges[{i}].schema_version", "invalid schema version")
            if edge.get("relation") not in RELATIONS:
                self.error("E_SCHEMA_EDGE", f"$.edges[{i}].relation", "unknown relation")
            for field in ("id", "source", "target"):
                if not isinstance(edge.get(field), str) or not edge[field]:
                    self.error("E_SCHEMA_EDGE", f"$.edges[{i}].{field}", "must be non-empty")

    def _index(self) -> None:
        for i, node in enumerate(self.graph["nodes"]):
            nid = node.get("id")
            if nid in self.nodes:
                self.error("E_NODE_DUPLICATE", f"$.nodes[{i}].id", f"duplicate node id {nid!r}")
            else:
                self.nodes[nid] = node
        for i, edge in enumerate(self.graph["edges"]):
            eid = edge.get("id")
            if eid in self.edges:
                self.error("E_EDGE_DUPLICATE", f"$.edges[{i}].id", f"duplicate edge id {eid!r}")
            else:
                self.edges[eid] = edge
            if edge.get("source") not in self.nodes:
                self.error("E_EDGE_ENDPOINT_MISSING", f"$.edges[{i}].source", "source node does not exist")
            if edge.get("target") not in self.nodes:
                self.error("E_EDGE_ENDPOINT_MISSING", f"$.edges[{i}].target", "target node does not exist")
        for nid, node in self.nodes.items():
            if node.get("kind") in {"Theorem", "Lemma", "Invariant", "Definition"}:
                digest = node.get("content_digest")
                if digest is not None and not SHA256_RE.fullmatch(digest):
                    self.error("E_SCHEMA_DIGEST", f"node:{nid}.content_digest", "invalid SHA-256 digest")
            self._check_provenance(node, f"node:{nid}.provenance")
        for eid, edge in self.edges.items():
            self._check_provenance(edge, f"edge:{eid}.provenance")

    def _check_provenance(self, record: dict[str, Any], path: str) -> None:
        p = record.get("provenance")
        if p is None:
            return
        if not isinstance(p, dict):
            self.error("E_PROVENANCE_MALFORMED", path, "provenance must be an object")
            return
        for field in ("commit_sha", "tree_sha"):
            value = p.get(field)
            if not isinstance(value, str) or not GIT_OID_RE.fullmatch(value):
                self.error("E_PROVENANCE_MALFORMED", f"{path}.{field}", "must be a lowercase 40- or 64-hex Git object ID")
        repo = p.get("repository")
        if not isinstance(repo, str) or not repo.strip() or any(c.isspace() for c in repo):
            self.error("E_PROVENANCE_MALFORMED", f"{path}.repository", "repository identity is invalid")
        repo_path = p.get("path")
        if repo_path is not None:
            if not isinstance(repo_path, str) or not repo_path or repo_path.startswith("/") or "\x00" in repo_path:
                self.error("E_PROVENANCE_MALFORMED", f"{path}.path", "repository path is not normalized")
            else:
                parts = PurePosixPath(repo_path).parts
                if any(part in {".", ".."} for part in parts) or "//" in repo_path:
                    self.error("E_PROVENANCE_MALFORMED", f"{path}.path", "repository path is not normalized")
        digest = p.get("content_digest")
        if digest is not None and (not isinstance(digest, str) or not SHA256_RE.fullmatch(digest)):
            self.error("E_PROVENANCE_MALFORMED", f"{path}.content_digest", "must be lowercase SHA-256")

    def _relationships(self) -> None:
        for eid, edge in self.edges.items():
            source = self.nodes.get(edge["source"])
            target = self.nodes.get(edge["target"])
            if not source or not target:
                continue
            rel = edge["relation"]
            if rel == "proves" and source["kind"] != "ProofArtifact":
                self.error("E_SEMANTIC_ID_MISMATCH", f"edge:{eid}", "proves source must be a ProofArtifact")
            if rel == "proves" and target["kind"] not in {"Theorem", "Lemma", "Invariant"}:
                self.error("E_SEMANTIC_ID_MISMATCH", f"edge:{eid}", "proves target must be a theorem, lemma, or invariant")
            if rel == "checked_by" and source["kind"] not in {"ProofArtifact", "ProofExecution"}:
                self.error("E_SEMANTIC_ID_MISMATCH", f"edge:{eid}", "checked_by source must be a proof artifact or execution")
            if rel == "checked_by" and target["kind"] != "ProofChecker":
                self.error("E_SEMANTIC_ID_MISMATCH", f"edge:{eid}", "checked_by target must be a ProofChecker")
            if rel == "qualifies" and target["kind"] != "QualificationContract":
                self.error("E_AUTHORITY_QUALIFIES_TARGET", f"edge:{eid}", "qualifies target must be a QualificationContract")

            evidence = edge.get("evidence", [])
            if evidence is not None:
                if not isinstance(evidence, list) or any(not isinstance(x, str) for x in evidence):
                    self.error("E_EVIDENCE_REFERENCE", f"edge:{eid}.evidence", "evidence must be node-id strings")
                else:
                    for ref in evidence:
                        if ref not in self.nodes:
                            self.error("E_EVIDENCE_REFERENCE", f"edge:{eid}.evidence", f"unknown evidence node {ref!r}")

            if rel == "qualifies":
                if not evidence:
                    self.error("E_AUTHORITY_UNPROVEN", f"edge:{eid}", "qualifies edge requires independently checkable evidence")
                for ref in evidence or []:
                    if self.nodes.get(ref, {}).get("kind") != "QualificationEvidence":
                        self.error("E_AUTHORITY_UNPROVEN", f"edge:{eid}.evidence", "qualifies evidence must reference QualificationEvidence")

    def _semantic_bindings(self) -> None:
        semantic_kinds = {"Theorem", "Lemma", "Invariant"}
        for nid, node in self.nodes.items():
            if node["kind"] in semantic_kinds:
                metadata = node.get("metadata")
                digest = metadata.get("semantic_digest") if isinstance(metadata, dict) else None
                if not isinstance(digest, str) or not SHA256_RE.fullmatch(digest):
                    self.error("E_SEMANTIC_ID_MISSING", f"node:{nid}.metadata.semantic_digest", "semantic identity must be a lowercase SHA-256 digest")
            if node["kind"] == "ProofArtifact":
                digest = node.get("content_digest")
                if not isinstance(digest, str) or not SHA256_RE.fullmatch(digest):
                    self.error("E_ARTIFACT_ID_MISSING", f"node:{nid}.content_digest", "ProofArtifact requires an exact content digest")
                metadata = node.get("metadata")
                target_digest = metadata.get("semantic_target_digest") if isinstance(metadata, dict) else None
                if not isinstance(target_digest, str) or not SHA256_RE.fullmatch(target_digest):
                    self.error("E_SEMANTIC_TARGET_MISSING", f"node:{nid}.metadata.semantic_target_digest", "ProofArtifact requires an exact semantic target digest")
                if not any(e["relation"] == "checked_by" and e["source"] == nid for e in self.edges.values()):
                    self.error("E_CHECKER_BINDING_MISSING", f"node:{nid}", "ProofArtifact requires an explicit checked_by ProofChecker edge")

        for eid, edge in self.edges.items():
            if edge["relation"] == "proves":
                source = self.nodes.get(edge["source"])
                target = self.nodes.get(edge["target"])
                if not source or not target or target["kind"] not in semantic_kinds:
                    continue
                target_digest = target.get("metadata", {}).get("semantic_digest") if isinstance(target.get("metadata"), dict) else None
                proof_digest = source.get("metadata", {}).get("semantic_target_digest") if isinstance(source.get("metadata"), dict) else None
                if proof_digest != target_digest:
                    self.error("E_SEMANTIC_ID_MISMATCH", f"edge:{eid}", "proof artifact target digest does not match semantic target identity")

        for eid, edge in self.edges.items():
            if edge["relation"] in {"supersedes", "invalidated_by"}:
                source = self.nodes.get(edge["source"])
                target = self.nodes.get(edge["target"])
                if source and target and source["kind"] != target["kind"]:
                    self.error("E_HISTORY_KIND_MISMATCH", f"edge:{eid}", "historical relation must preserve node kind")
                if source and target and source["id"] == target["id"]:
                    self.error("E_HISTORY_SELF_REFERENCE", f"edge:{eid}", "historical relation cannot target itself")

    def _proof_dag(self) -> None:
        adjacency: dict[str, list[str]] = defaultdict(list)
        proof_nodes = {nid for nid, n in self.nodes.items() if n["kind"] == "ProofArtifact"}
        for eid, edge in self.edges.items():
            if edge["relation"] != "depends_on":
                continue
            if edge["source"] in proof_nodes and edge["target"] in proof_nodes:
                if edge["source"] == edge["target"]:
                    self.error("E_PROOF_CYCLE", f"edge:{eid}", "proof artifact cannot depend on itself")
                adjacency[edge["source"]].append(edge["target"])

        visiting: set[str] = set()
        visited: set[str] = set()

        def visit(node: str) -> None:
            if node in visiting:
                self.error("E_PROOF_CYCLE", f"node:{node}", "proof dependency graph contains a cycle")
                return
            if node in visited:
                return
            visiting.add(node)
            for child in adjacency[node]:
                visit(child)
            visiting.remove(node)
            visited.add(node)

        for node in proof_nodes:
            visit(node)

    def _qualification(self) -> None:
        for nid, node in self.nodes.items():
            if node["kind"] != "QualificationEvidence":
                continue
            metadata = node.get("metadata")
            if not isinstance(metadata, dict):
                self.error("E_EVIDENCE_BINDING", f"node:{nid}.metadata", "qualification evidence requires binding metadata")
                continue
            required = ("contract_id", "contract_digest", "verifier_release_id", "verifier_release_digest", "subject_head", "subject_tree", "execution_id", "result")
            for key in required:
                if key not in metadata:
                    self.error("E_EVIDENCE_BINDING", f"node:{nid}.metadata.{key}", "missing qualification binding")
            for key in ("subject_head", "subject_tree"):
                if key in metadata and (not isinstance(metadata[key], str) or not GIT_OID_RE.fullmatch(metadata[key])):
                    self.error("E_EVIDENCE_BINDING", f"node:{nid}.metadata.{key}", "must be a lowercase 40-hex Git object ID")

            contract = self.nodes.get(metadata.get("contract_id"))
            verifier = self.nodes.get(metadata.get("verifier_release_id"))
            if not contract or contract.get("kind") != "QualificationContract":
                self.error("E_EVIDENCE_BINDING", f"node:{nid}.metadata.contract_id", "contract_id must reference QualificationContract")
            elif metadata.get("contract_digest") != contract.get("content_digest"):
                self.error("E_EVIDENCE_BINDING", f"node:{nid}.metadata.contract_digest", "contract digest does not match referenced contract")
            if not verifier or verifier.get("kind") != "VerifierRelease":
                self.error("E_EVIDENCE_BINDING", f"node:{nid}.metadata.verifier_release_id", "verifier_release_id must reference VerifierRelease")
            elif metadata.get("verifier_release_digest") != verifier.get("content_digest"):
                self.error("E_EVIDENCE_BINDING", f"node:{nid}.metadata.verifier_release_digest", "verifier release digest does not match referenced release")

            result = metadata.get("result")
            if result == "QualifiedPass":
                self.error("E_AUTHORITY_NOT_LOCAL", f"node:{nid}", "QualifiedPass must be imported from an independently established qualification system")

            execution_id = metadata.get("execution_id")
            execution = self.nodes.get(execution_id)
            if execution and execution["kind"] != "ProofExecution":
                self.error("E_EVIDENCE_BINDING", f"node:{nid}.metadata.execution_id", "execution_id must reference ProofExecution")
                execution = None
            if execution:
                execution_metadata = execution.get("metadata")
                if not isinstance(execution_metadata, dict):
                    execution_metadata = {}
                status = execution_metadata.get("status")
                if status not in {"Passed", "Failed"}:
                    self.error("E_AUTHORITY_UNPROVEN", f"node:{nid}", "qualification evidence cannot bind a non-terminal successful/failed execution")
                if status == "Passed" and result not in {"QualifiedPass", "Pass", "Passed"}:
                    self.error("E_EVIDENCE_BINDING", f"node:{nid}", "passed execution requires an explicit qualification result")
                if status == "Failed" and result not in {"QualifiedFail", "Fail", "Failed"}:
                    self.error("E_EVIDENCE_BINDING", f"node:{nid}", "failed execution requires an explicit qualification result")
                for field, expected_kind in (
                    ("artifact_id", "ProofArtifact"),
                    ("checker_id", "ProofChecker"),
                    ("source_commit_id", "SourceCommit"),
                    ("source_tree_id", "SourceTree"),
                ):
                    ref = execution_metadata.get(field)
                    if ref is None:
                        self.error("E_EVIDENCE_BINDING", f"node:{execution_id}.metadata.{field}", f"{field} is required for exact execution binding")
                        continue
                    target = self.nodes.get(ref)
                    if not target or target["kind"] != expected_kind:
                        self.error("E_EVIDENCE_BINDING", f"node:{execution_id}.metadata.{field}", f"{field} must reference {expected_kind}")
                    elif expected_kind == "SourceCommit" and execution_metadata.get("source_commit_sha") != target.get("metadata", {}).get("commit_sha"):
                        self.error("E_EVIDENCE_BINDING", f"node:{execution_id}.metadata.source_commit_sha", "source commit ID does not match referenced SourceCommit metadata.commit_sha")
                    elif expected_kind == "SourceTree" and execution_metadata.get("source_tree_sha") != target.get("metadata", {}).get("tree_sha"):
                        self.error("E_EVIDENCE_BINDING", f"node:{execution_id}.metadata.source_tree_sha", "source tree ID does not match referenced SourceTree metadata.tree_sha")
                for field in ("source_commit_sha", "source_tree_sha"):
                    if not isinstance(execution_metadata.get(field), str) or not GIT_OID_RE.fullmatch(execution_metadata.get(field, "")):
                        self.error("E_EVIDENCE_BINDING", f"node:{execution_id}.metadata.{field}", "exact source binding requires a lowercase 40-hex Git object ID")
                if execution_metadata.get("subject_head") != metadata.get("subject_head"):
                    self.error("E_STALE_EVIDENCE", f"node:{nid}.metadata.subject_head", "qualification subject head differs from execution binding")
                if execution_metadata.get("subject_tree") != metadata.get("subject_tree"):
                    self.error("E_STALE_EVIDENCE", f"node:{nid}.metadata.subject_tree", "qualification subject tree differs from execution binding")
                if execution_metadata.get("result") is not None and execution_metadata["result"] != result:
                    self.error("E_EVIDENCE_BINDING", f"node:{nid}.metadata.result", "qualification result differs from execution result binding")

        for eid, edge in self.edges.items():
            if edge["relation"] != "qualifies":
                continue
            for ref in edge.get("evidence", []):
                evidence = self.nodes.get(ref)
                if not evidence:
                    continue
                if evidence["kind"] != "QualificationEvidence":
                    continue
                result = evidence.get("metadata", {}).get("result")
                if result not in {"Pass", "Passed", "QualifiedPass"}:
                    self.error("E_AUTHORITY_UNPROVEN", f"edge:{eid}", "qualifies edge lacks a passing qualification result")
                if result == "QualifiedPass":
                    self.error("E_AUTHORITY_NOT_LOCAL", f"edge:{eid}", "graph cannot manufacture a QualifiedPass authority state")
                for historical_edge in self.edges.values():
                    if historical_edge["relation"] == "supersedes" and historical_edge["target"] == ref:
                        self.error("E_EVIDENCE_HISTORICAL", f"edge:{eid}.evidence", "superseded evidence cannot establish current qualification")
                    if historical_edge["relation"] == "invalidated_by" and historical_edge["source"] == ref:
                        self.error("E_EVIDENCE_HISTORICAL", f"edge:{eid}.evidence", "invalidated evidence cannot establish current qualification")


def validate_file(path: str) -> dict[str, Any]:
    with open(path, encoding="utf-8") as handle:
        graph = json.load(handle)
    return Validator(graph).validate()


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate Symtropy formal provenance graph v0")
    parser.add_argument("graph", help="path to a formal-provenance-v0 JSON graph")
    args = parser.parse_args()
    try:
        result = validate_file(args.graph)
    except (OSError, json.JSONDecodeError, UnicodeError) as exc:
        result = {
            "schema_version": SCHEMA_VERSION,
            "status": "SchemaInvalid",
            "errors": [{"code": "E_SCHEMA_INPUT", "path": "$", "message": str(exc)}],
            "authority": "NotEstablished",
        }
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 0 if result["status"] == "Valid" else 1


if __name__ == "__main__":
    raise SystemExit(main())
