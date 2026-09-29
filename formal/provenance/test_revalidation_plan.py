import copy
import pathlib
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from revalidation_plan import build_plan  # noqa: E402

SCHEMA = "luminous.formal-provenance.v0"
A = "a" * 64
B = "b" * 64
C = "c" * 64
D = "d" * 64
E = "e" * 64
F = "f" * 64


def node(kind, node_id, label, **extra):
    value = {
        "schema_version": SCHEMA,
        "kind": kind,
        "id": node_id,
        "label": label,
    }
    value.update(extra)
    return value


def valid_graph():
    return {
        "schema_version": SCHEMA,
        "nodes": [
            node("Theorem", "theorem:t", "Theorem", metadata={"semantic_digest": A}),
            node("ProofArtifact", "proof:p", "Proof", content_digest=B, metadata={"semantic_target_digest": A}),
            node("ProofChecker", "checker:c", "Checker", content_digest=C),
            node("SourceCommit", "source:commit", "Source commit", content_digest=D,
                 provenance={"repository": "Luminous-Dynamics/symtropy", "commit_sha": D, "tree_sha": E}),
            node("SourceTree", "source:tree", "Source tree", content_digest=E,
                 provenance={"repository": "Luminous-Dynamics/symtropy", "commit_sha": D, "tree_sha": E}),
            node("ProofExecution", "execution:x", "Execution", metadata={
                "artifact_id": "proof:p", "checker_id": "checker:c",
                "source_commit_id": "source:commit", "source_tree_id": "source:tree",
                "source_commit_sha": D, "source_tree_sha": E,
                "subject_head": F, "subject_tree": F, "status": "Passed", "result": "Pass",
            }),
            node("QualificationContract", "contract:q", "Contract", content_digest=A),
            node("VerifierRelease", "verifier:v", "Verifier", content_digest=B),
            node("QualificationEvidence", "evidence:q", "Evidence", metadata={
                "contract_id": "contract:q", "contract_digest": A,
                "verifier_release_id": "verifier:v", "verifier_release_digest": B,
                "subject_head": F, "subject_tree": F,
                "execution_id": "execution:x", "result": "Pass",
            }),
        ],
        "edges": [
            {"schema_version": SCHEMA, "id": "edge:proves", "source": "proof:p", "target": "theorem:t", "relation": "proves"},
            {"schema_version": SCHEMA, "id": "edge:checked", "source": "proof:p", "target": "checker:c", "relation": "checked_by"},
        ],
    }


def impact_for(*node_ids):
    return {
        "schema_version": SCHEMA,
        "impact_version": "luminous.formal-impact.v0",
        "impact_status": "RevalidationRequired",
        "changed_node_ids": list(node_ids),
        "records": [{"node_id": node_id, "reason_code": "PROOF_ARTIFACT_CHANGED", "via_edges": []} for node_id in node_ids],
    }


class RevalidationPlanTests(unittest.TestCase):
    def test_ready_plan_for_exactly_bound_execution(self):
        result = build_plan(valid_graph(), impact_for("execution:x"))
        self.assertEqual(result["plan_status"], "Ready")
        self.assertEqual(result["records"][0]["action"], "RECHECK_EXECUTION")

    def test_missing_execution_binding_is_blocked(self):
        graph = valid_graph()
        graph["nodes"][5]["metadata"].pop("checker_id")
        result = build_plan(graph, impact_for("execution:x"))
        self.assertEqual(result["plan_status"], "Blocked")
        self.assertIn("checker_id", result["blockers"][0]["missing_bindings"])

    def test_wrong_binding_kind_is_blocked(self):
        graph = valid_graph()
        graph["nodes"][5]["metadata"]["checker_id"] = "theorem:t"
        result = build_plan(graph, impact_for("execution:x"))
        self.assertEqual(result["plan_status"], "Blocked")
        self.assertTrue(result["blockers"][0]["contradictory_bindings"])

    def test_superseded_evidence_is_blocked(self):
        graph = valid_graph()
        graph["nodes"].append(copy.deepcopy(graph["nodes"][8]))
        graph["nodes"][-1]["id"] = "evidence:old"
        graph["nodes"][-1]["label"] = "Old evidence"
        graph["edges"].append({
            "schema_version": SCHEMA, "id": "edge:supersedes",
            "source": "evidence:q", "target": "evidence:old", "relation": "supersedes",
        })
        result = build_plan(graph, impact_for("evidence:old"))
        self.assertEqual(result["plan_status"], "Blocked")
        self.assertEqual(result["blockers"][0]["historical_flags"], ["SUPERSEDED"])


    def test_tampered_impact_record_is_rejected(self):
        impact = impact_for("execution:x")
        impact["records"][0]["reason_code"] = "CHECKER_CHANGED"
        with self.assertRaises(ValueError):
            build_plan(valid_graph(), impact)

    def test_omitted_impact_record_is_rejected(self):
        graph = valid_graph()
        graph["nodes"].append(
            node("ProofExecution", "execution:y", "Second execution", metadata={
                "artifact_id": "proof:p", "checker_id": "checker:c",
                "source_commit_id": "source:commit", "source_tree_id": "source:tree",
                "source_commit_sha": D, "source_tree_sha": E,
                "subject_head": F, "subject_tree": F,
                "status": "Passed", "result": "Pass",
            })
        )
        impact = impact_for("proof:p")
        # Canonical impact reaches both executions through the artifact binding.
        impact["records"] = [
            record for record in impact["records"] if record["node_id"] != "execution:y"
        ]
        with self.assertRaises(ValueError):
            build_plan(graph, impact)

    def test_injected_unaffected_record_is_rejected(self):
        graph = valid_graph()
        impact = impact_for("execution:x")
        impact["records"].append({
            "node_id": "theorem:t",
            "reason_code": "SEMANTIC_DEPENDENCY_CHANGED",
            "via_edges": [],
        })
        with self.assertRaises(ValueError):
            build_plan(graph, impact)

    def test_no_impact_cannot_contain_records(self):
        impact = {
            "schema_version": SCHEMA,
            "impact_version": "luminous.formal-impact.v0",
            "impact_status": "NoImpact",
            "changed_node_ids": ["theorem:t"],
            "records": [{
                "node_id": "execution:x",
                "reason_code": "EXECUTION_CHANGED",
                "via_edges": [],
            }],
        }
        with self.assertRaises(ValueError):
            build_plan(valid_graph(), impact)

    def test_duplicate_changed_ids_are_rejected(self):
        impact = impact_for("execution:x")
        impact["changed_node_ids"] = ["execution:x", "execution:x"]
        with self.assertRaises(ValueError):
            build_plan(valid_graph(), impact)

    def test_unknown_changed_id_is_rejected(self):
        impact = impact_for("execution:x")
        impact["changed_node_ids"] = ["execution:missing"]
        with self.assertRaises(ValueError):
            build_plan(valid_graph(), impact)

    def test_no_revalidation_required(self):
        result = build_plan(valid_graph(), {
            "schema_version": SCHEMA,
            "impact_version": "luminous.formal-impact.v0",
            "impact_status": "NoImpact",
            "changed_node_ids": ["theorem:t"],
            "records": [],
        })
        self.assertEqual(result["plan_status"], "NoRevalidationRequired")


if __name__ == "__main__":
    unittest.main()
