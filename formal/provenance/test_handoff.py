#!/usr/bin/env python3
"""Adversarial tests for the bounded qualification authority handoff."""

from __future__ import annotations

import copy
import hashlib
import json
import sys
import unittest

from admission import build_admission, digest
from handoff import HANDOFF_VERSION, build_handoff

sys.path.insert(0, __import__("os").path.dirname(__file__))


def _node(kind: str, node_id: str, content: str, metadata: dict | None = None) -> dict:
    node = {"id": node_id, "kind": kind, "content_digest": hashlib.sha256(content.encode()).hexdigest()}
    if metadata is not None:
        node["metadata"] = metadata
    return node


def make_fixture() -> tuple[dict, dict, dict]:
    theorem_digest = hashlib.sha256(b"theorem").hexdigest()
    checker_digest = hashlib.sha256(b"checker").hexdigest()
    contract_digest = hashlib.sha256(b"contract").hexdigest()
    verifier_digest = hashlib.sha256(b"verifier").hexdigest()
    execution_digest = hashlib.sha256(b"execution").hexdigest()
    source_commit_sha = "a" * 40
    source_tree_sha = "b" * 40
    subject_head = "c" * 40
    subject_tree = "d" * 40

    nodes = [
        _node("Theorem", "theorem:t", "theorem", {"semantic_digest": theorem_digest}),
        _node("ProofArtifact", "proof:t", "proof", {"semantic_target_digest": theorem_digest}),
        _node("ProofChecker", "checker:t", "checker"),
        _node("SourceCommit", "source:commit", "source-commit", {"commit_sha": source_commit_sha}),
        _node("SourceTree", "source:tree", "source-tree", {"tree_sha": source_tree_sha}),
        _node(
            "ProofExecution", "execution:t", "execution",
            {
                "artifact_id": "proof:t",
                "checker_id": "checker:t",
                "source_commit_id": "source:commit",
                "source_tree_id": "source:tree",
                "source_commit_sha": source_commit_sha,
                "source_tree_sha": source_tree_sha,
                "subject_head": subject_head,
                "subject_tree": subject_tree,
            },
        ),
        _node("QualificationContract", "contract:t", "contract"),
        _node("VerifierRelease", "verifier:t", "verifier"),
        _node(
            "QualificationEvidence", "evidence:t", "evidence",
            {
                "execution_id": "execution:t",
                "contract_id": "contract:t",
                "verifier_release_id": "verifier:t",
                "contract_digest": contract_digest,
                "verifier_release_digest": verifier_digest,
                "subject_head": subject_head,
                "subject_tree": subject_tree,
                "result": "Pass",
            },
        ),
    ]
    # Make the fixture's identity digests match its evidence bindings.
    for node_id, expected in {
        "contract:t": contract_digest,
        "verifier:t": verifier_digest,
        "execution:t": execution_digest,
    }.items():
        nodes[[n["id"] for n in nodes].index(node_id)]["content_digest"] = expected

    edges = [
        {"id": "edge:proves", "relation": "proves", "source": "proof:t", "target": "theorem:t"},
        {"id": "edge:checked", "relation": "checked_by", "source": "proof:t", "target": "checker:t"},
        {"id": "edge:qualifies", "relation": "qualifies", "source": "execution:t", "target": "evidence:t"},
    ]
    graph = {"schema_version": "luminous.formal-provenance.v0", "nodes": nodes, "edges": edges}

    # The validator requires source provenance on SourceCommit/SourceTree and
    # exact qualification metadata; fill the repository/source identity fields.
    for node in graph["nodes"]:
        if node["kind"] == "SourceCommit":
            node["provenance"] = {
                "repository": "Luminous-Dynamics/symtropy",
                "commit_sha": source_commit_sha,
                "tree_sha": source_tree_sha,
            }
        if node["kind"] == "SourceTree":
            node["provenance"] = {
                "repository": "Luminous-Dynamics/symtropy",
                "commit_sha": source_commit_sha,
                "tree_sha": source_tree_sha,
            }

    impact = {
        "schema_version": graph["schema_version"],
        "impact_version": "luminous.formal-impact.v0",
        "impact_status": "RevalidationRequired",
        "changed_node_ids": ["theorem:t"],
        "records": [
            {
                "node_id": "theorem:t",
                "kind": "Theorem",
                "reason_code": "SEMANTIC_DEPENDENCY_CHANGED",
                "via_edges": [],
            }
        ],
    }
    # Use the real analyzer shape by importing the implementation rather than
    # treating this fixture as an authority oracle.
    from impact import analyze
    impact = analyze(graph, ["theorem:t"])
    envelope = build_admission(
        graph,
        impact,
        subject_repository="Luminous-Dynamics/symtropy",
        expected_result="QualifiedPass",
    )
    return graph, impact, envelope


class HandoffTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.graph, cls.impact, cls.envelope = make_fixture()

    def test_round_trip_is_declarative(self):
        result = build_handoff(self.graph, self.impact, self.envelope)
        self.assertEqual(result["handoff_version"], HANDOFF_VERSION)
        self.assertEqual(result["admission_digest"], self.envelope["admission_digest"])
        self.assertEqual(result["subject"], self.envelope["subject"])
        self.assertEqual(
            set(result["authority_binding"]),
            {
                "contract_id", "contract_digest",
                "verifier_release_id", "verifier_release_digest",
                "execution_id", "execution_digest",
            },
        )
        self.assertNotIn("command", json.dumps(result))
        self.assertNotIn("cargo_args", json.dumps(result))
        self.assertNotIn("workflow", json.dumps(result))

    def test_tampered_admission_rejected(self):
        tampered = copy.deepcopy(self.envelope)
        tampered["expected_result"] = "QualifiedFail"
        tampered["admission_digest"] = digest({k: v for k, v in tampered.items() if k != "admission_digest"})
        with self.assertRaises(ValueError):
            build_handoff(self.graph, self.impact, tampered)

    def test_injected_executable_field_rejected(self):
        tampered = copy.deepcopy(self.envelope)
        tampered["command"] = "cargo test"
        tampered["admission_digest"] = digest({k: v for k, v in tampered.items() if k != "admission_digest"})
        with self.assertRaises(ValueError):
            build_handoff(self.graph, self.impact, tampered)

    def test_graph_replay_rejected(self):
        altered = copy.deepcopy(self.graph)
        altered["nodes"][0]["metadata"]["semantic_digest"] = hashlib.sha256(b"other theorem").hexdigest()
        with self.assertRaises(ValueError):
            build_handoff(altered, self.impact, self.envelope)

    def test_frontier_replay_rejected(self):
        altered_impact = copy.deepcopy(self.impact)
        altered_impact["changed_node_ids"] = ["proof:t"]
        from impact import analyze
        altered_impact = analyze(self.graph, ["proof:t"])
        with self.assertRaises(ValueError):
            build_handoff(self.graph, altered_impact, self.envelope)


if __name__ == "__main__":
    unittest.main()
