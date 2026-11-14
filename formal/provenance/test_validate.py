import copy
import json
import pathlib
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from validate import Validator  # noqa: E402


with open(ROOT / "example-v0.json", encoding="utf-8") as f:
    EXAMPLE = json.load(f)


class ValidatorTests(unittest.TestCase):
    def test_example_is_valid(self):
        result = Validator(copy.deepcopy(EXAMPLE)).validate()
        self.assertEqual(result["status"], "Valid")

    def test_missing_edge_endpoint_is_rejected(self):
        graph = copy.deepcopy(EXAMPLE)
        graph["edges"][0]["target"] = "theorem:missing"
        result = Validator(graph).validate()
        self.assertIn("E_EDGE_ENDPOINT_MISSING", {e["code"] for e in result["errors"]})

    def test_proof_cycle_is_rejected(self):
        graph = copy.deepcopy(EXAMPLE)
        graph["nodes"].append({
            "schema_version": "luminous.formal-provenance.v0",
            "kind": "ProofArtifact",
            "id": "proof:child",
            "label": "Child proof",
        })
        graph["edges"].extend([
            {
                "schema_version": "luminous.formal-provenance.v0",
                "id": "edge:depends:a",
                "source": "proof:physics-body-ref-resolution-v1",
                "target": "proof:child",
                "relation": "depends_on",
            },
            {
                "schema_version": "luminous.formal-provenance.v0",
                "id": "edge:depends:b",
                "source": "proof:child",
                "target": "proof:physics-body-ref-resolution-v1",
                "relation": "depends_on",
            },
        ])
        result = Validator(graph).validate()
        self.assertIn("E_PROOF_CYCLE", {e["code"] for e in result["errors"]})


    def test_semantic_identity_cannot_be_reused_under_changed_content(self):
        graph = copy.deepcopy(EXAMPLE)
        theorem = graph["nodes"][0]
        theorem["semantic_digest"] = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
        result = Validator(graph).validate()
        self.assertIn("E_SEMANTIC_ID_MISMATCH", {e["code"] for e in result["errors"]})

    def test_proof_must_have_exact_checker_binding(self):
        graph = copy.deepcopy(EXAMPLE)
        graph["edges"] = [e for e in graph["edges"] if e["relation"] != "checked_by"]
        result = Validator(graph).validate()
        self.assertIn("E_CHECKER_BINDING", {e["code"] for e in result["errors"]})

    def test_proof_target_digest_mismatch_is_rejected(self):
        graph = copy.deepcopy(EXAMPLE)
        graph["nodes"][1]["metadata"]["target_semantic_digest"] = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
        result = Validator(graph).validate()
        self.assertIn("E_SEMANTIC_ID_MISMATCH", {e["code"] for e in result["errors"]})

    def test_qualification_cannot_use_skipped_execution(self):
        graph = copy.deepcopy(EXAMPLE)
        graph["nodes"].extend([
            {
                "schema_version": "luminous.formal-provenance.v0",
                "kind": "QualificationContract",
                "id": "contract:v1",
                "label": "Contract v1",
            },
            {
                "schema_version": "luminous.formal-provenance.v0",
                "kind": "VerifierRelease",
                "id": "verifier:v1",
                "label": "Verifier v1",
            },
            {
                "schema_version": "luminous.formal-provenance.v0",
                "kind": "ProofExecution",
                "id": "execution:skipped",
                "label": "Skipped execution",
                "metadata": {"status": "Skipped"},
            },
            {
                "schema_version": "luminous.formal-provenance.v0",
                "kind": "QualificationEvidence",
                "id": "evidence:bad",
                "label": "Invalid qualification evidence",
                "metadata": {
                    "contract_id": "contract:v1",
                    "verifier_release_id": "verifier:v1",
                    "subject_head": "d56d6a6c1381dc131d493f0713b657f01c98cb80",
                    "subject_tree": "d56d6a6c1381dc131d493f0713b657f01c98cb80",
                    "execution_id": "execution:skipped",
                    "result": "Pass",
                },
            },
        ])
        graph["edges"].append({
            "schema_version": "luminous.formal-provenance.v0",
            "id": "edge:qualifies:bad",
            "source": "proof:physics-body-ref-resolution-v1",
            "target": "contract:v1",
            "relation": "qualifies",
            "evidence": ["evidence:bad"],
        })
        result = Validator(graph).validate()
        codes = {e["code"] for e in result["errors"]}
        self.assertIn("E_AUTHORITY_UNPROVEN", codes)


    def test_stale_subject_binding_is_rejected(self):
        graph = copy.deepcopy(EXAMPLE)
        graph["nodes"].extend([
            {"schema_version": "luminous.formal-provenance.v0", "kind": "QualificationContract", "id": "contract:v1", "label": "Contract"},
            {"schema_version": "luminous.formal-provenance.v0", "kind": "VerifierRelease", "id": "verifier:v1", "label": "Verifier"},
            {"schema_version": "luminous.formal-provenance.v0", "kind": "ProofExecution", "id": "execution:pass", "label": "Pass",
             "metadata": {"status": "Passed", "subject_head": "d56d6a6c1381dc131d493f0713b657f01c98cb80",
                          "subject_tree": "d56d6a6c1381dc131d493f0713b657f01c98cb80", "result": "Pass"}},
            {"schema_version": "luminous.formal-provenance.v0", "kind": "QualificationEvidence", "id": "evidence:stale", "label": "Stale",
             "metadata": {"contract_id": "contract:v1", "verifier_release_id": "verifier:v1",
                          "subject_head": "896838a2ee666a69f612e1a2b797f1a46373b87f",
                          "subject_tree": "d56d6a6c1381dc131d493f0713b657f01c98cb80",
                          "execution_id": "execution:pass", "result": "Pass"}},
        ])
        graph["edges"].append({
            "schema_version": "luminous.formal-provenance.v0",
            "id": "edge:qualifies:stale",
            "source": "proof:physics-body-ref-resolution-v1",
            "target": "contract:v1",
            "relation": "qualifies",
            "evidence": ["evidence:stale"],
        })
        result = Validator(graph).validate()
        self.assertIn("E_STALE_EVIDENCE", {e["code"] for e in result["errors"]})

    def test_authority_is_not_manufactured(self):
        graph = copy.deepcopy(EXAMPLE)
        graph["nodes"].extend([
            {"schema_version": "luminous.formal-provenance.v0", "kind": "QualificationContract", "id": "contract:v1", "label": "Contract"},
            {"schema_version": "luminous.formal-provenance.v0", "kind": "VerifierRelease", "id": "verifier:v1", "label": "Verifier"},
            {"schema_version": "luminous.formal-provenance.v0", "kind": "ProofExecution", "id": "execution:pass", "label": "Pass", "metadata": {"status": "Passed"}},
            {"schema_version": "luminous.formal-provenance.v0", "kind": "QualificationEvidence", "id": "evidence:pass", "label": "Pass", "metadata": {
                "contract_id": "contract:v1", "verifier_release_id": "verifier:v1",
                "subject_head": "d56d6a6c1381dc131d493f0713b657f01c98cb80",
                "subject_tree": "d56d6a6c1381dc131d493f0713b657f01c98cb80",
                "execution_id": "execution:pass", "result": "QualifiedPass",
            }},
        ])
        graph["edges"].append({
            "schema_version": "luminous.formal-provenance.v0",
            "id": "edge:qualifies:pass",
            "source": "proof:physics-body-ref-resolution-v1",
            "target": "contract:v1",
            "relation": "qualifies",
            "evidence": ["evidence:pass"],
        })
        result = Validator(graph).validate()
        self.assertIn("E_AUTHORITY_NOT_LOCAL", {e["code"] for e in result["errors"]})


if __name__ == "__main__":
    unittest.main()
