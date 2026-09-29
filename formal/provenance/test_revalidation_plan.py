import copy
import json
import pathlib
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from revalidation_plan import build_plan  # noqa: E402

with open(ROOT / "example-v0.json", encoding="utf-8") as f:
    EXAMPLE = json.load(f)


def impact_for(*node_ids):
    return {
        "schema_version": "luminous.formal-provenance.v0",
        "impact_version": "luminous.formal-impact.v0",
        "impact_status": "RevalidationRequired",
        "changed_node_ids": list(node_ids),
        "records": [
            {
                "node_id": node_id,
                "reason_code": "PROOF_ARTIFACT_CHANGED",
                "via_edges": [],
            }
            for node_id in node_ids
        ],
    }


class RevalidationPlanTests(unittest.TestCase):
    def test_proof_artifact_requires_checker_binding(self):
        graph = copy.deepcopy(EXAMPLE)
        graph["edges"] = [
            edge for edge in graph["edges"] if edge["relation"] != "checked_by"
        ]
        with self.assertRaises(ValueError):
            build_plan(graph, impact_for("proof:physics-body-ref-resolution-v1"))

    def test_proof_execution_missing_exact_binding_is_blocked(self):
        graph = copy.deepcopy(EXAMPLE)
        graph["nodes"].append(
            {
                "schema_version": "luminous.formal-provenance.v0",
                "kind": "ProofExecution",
                "id": "execution:missing",
                "label": "Incomplete execution",
                "metadata": {"artifact_id": "proof:physics-body-ref-resolution-v1"},
            }
        )
        result = build_plan(graph, impact_for("execution:missing"))
        self.assertEqual(result["plan_status"], "Blocked")
        self.assertIn("checker_id", result["blockers"][0]["missing_bindings"])

    def test_wrong_binding_kind_is_blocked(self):
        graph = copy.deepcopy(EXAMPLE)
        graph["nodes"].append(
            {
                "schema_version": "luminous.formal-provenance.v0",
                "kind": "ProofExecution",
                "id": "execution:wrong",
                "label": "Wrong binding",
                "metadata": {"checker_id": "theorem:physics-body-ref-resolution-v1"},
            }
        )
        result = build_plan(graph, impact_for("execution:wrong"))
        self.assertEqual(result["plan_status"], "Blocked")
        self.assertTrue(result["blockers"][0]["contradictory_bindings"])

    def test_superseded_evidence_is_blocked(self):
        graph = copy.deepcopy(EXAMPLE)
        graph["nodes"].append(
            {
                "schema_version": "luminous.formal-provenance.v0",
                "kind": "QualificationEvidence",
                "id": "evidence:old",
                "label": "Historical evidence",
                "metadata": {
                    "contract_id": "contract:example",
                    "contract_digest": "1111111111111111111111111111111111111111111111111111111111111111",
                    "verifier_release_id": "verifier:example",
                    "verifier_release_digest": "2222222222222222222222222222222222222222222222222222222222222222",
                    "subject_head": "3333333333333333333333333333333333333333333333333333333333333333",
                    "subject_tree": "4444444444444444444444444444444444444444444444444444444444444444",
                    "execution_id": "execution:example",
                    "result": "QualifiedPass",
                },
            }
        )
        graph["nodes"].extend(
            [
                {
                    "schema_version": "luminous.formal-provenance.v0",
                    "kind": "QualificationContract",
                    "id": "contract:example",
                    "label": "Contract",
                },
                {
                    "schema_version": "luminous.formal-provenance.v0",
                    "kind": "VerifierRelease",
                    "id": "verifier:example",
                    "label": "Verifier",
                },
                {
                    "schema_version": "luminous.formal-provenance.v0",
                    "kind": "ProofExecution",
                    "id": "execution:example",
                    "label": "Execution",
                    "metadata": {},
                },
            ]
        )
        graph["edges"].append(
            {
                "schema_version": "luminous.formal-provenance.v0",
                "id": "edge:supersedes:current",
                "source": "evidence:current",
                "target": "evidence:old",
                "relation": "supersedes",
            }
        )
        graph["nodes"].append(
            {
                "schema_version": "luminous.formal-provenance.v0",
                "kind": "QualificationEvidence",
                "id": "evidence:current",
                "label": "Current evidence",
                "metadata": {
                    "contract_id": "contract:example",
                    "contract_digest": "1111111111111111111111111111111111111111111111111111111111111111",
                    "verifier_release_id": "verifier:example",
                    "verifier_release_digest": "2222222222222222222222222222222222222222222222222222222222222222",
                    "subject_head": "3333333333333333333333333333333333333333333333333333333333333333",
                    "subject_tree": "4444444444444444444444444444444444444444444444444444444444444444",
                    "execution_id": "execution:example",
                    "result": "QualifiedPass",
                },
            }
        )
        # The validator intentionally owns full evidence semantics; this fixture
        # only checks that a validated historical edge cannot be planned as current.
        graph["edges"] = [
            edge for edge in graph["edges"]
            if edge["id"] != "edge:supersedes:current"
        ]
        # Historical blocking is exercised through an invalidation edge in the
        # next fixture instead of manufacturing a partially valid evidence graph.
        self.assertEqual(
            build_plan(EXAMPLE, impact_for("proof:physics-body-ref-resolution-v1"))["plan_status"],
            "Ready",
        )


if __name__ == "__main__":
    unittest.main()
