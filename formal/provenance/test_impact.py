import copy
import json
import pathlib
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from impact import analyze  # noqa: E402

with open(ROOT / "example-v0.json", encoding="utf-8") as f:
    EXAMPLE = json.load(f)


class ImpactTests(unittest.TestCase):
    def test_theorem_change_reaches_proof_and_checker(self):
        result = analyze(
            copy.deepcopy(EXAMPLE),
            ["theorem:physics-body-ref-resolution-v1"],
        )
        self.assertEqual(result["impact_status"], "RevalidationRequired")
        ids = {record["node_id"] for record in result["records"]}
        self.assertIn("proof:physics-body-ref-resolution-v1", ids)
        self.assertIn("checker:example-v0", ids)

    def test_impact_is_deterministic(self):
        first = analyze(
            copy.deepcopy(EXAMPLE),
            ["theorem:physics-body-ref-resolution-v1"],
        )
        reordered = copy.deepcopy(EXAMPLE)
        reordered["nodes"].reverse()
        reordered["edges"].reverse()
        second = analyze(
            reordered,
            ["theorem:physics-body-ref-resolution-v1"],
        )
        self.assertEqual(first, second)

    def test_unknown_change_fails_closed(self):
        with self.assertRaises(ValueError):
            analyze(copy.deepcopy(EXAMPLE), ["node:missing"])

    def test_checker_change_propagates_with_checker_reason(self):
        graph = copy.deepcopy(EXAMPLE)
        graph["nodes"].append(
            {
                "schema_version": "luminous.formal-provenance.v0",
                "kind": "ProofExecution",
                "id": "execution:example",
                "label": "Example execution",
                "metadata": {"checker_id": "checker:example-v0"},
            }
        )
        result = analyze(graph, ["checker:example-v0"])
        execution = next(
            r for r in result["records"] if r["node_id"] == "execution:example"
        )
        self.assertEqual(execution["reason_code"], "CHECKER_CHANGED")
        self.assertEqual(execution["via_edges"], ["checker:example-v0"])

    def test_source_dependency_uses_dedicated_reason(self):
        graph = copy.deepcopy(EXAMPLE)
        graph["nodes"].append(
            {
                "schema_version": "luminous.formal-provenance.v0",
                "kind": "ProofExecution",
                "id": "execution:source",
                "label": "Source-bound execution",
                "metadata": {"source_commit_id": "source:commit"},
            }
        )
        graph["nodes"].append(
            {
                "schema_version": "luminous.formal-provenance.v0",
                "kind": "SourceCommit",
                "id": "source:commit",
                "label": "Source commit",
                "provenance": {
                    "repository": "Luminous-Dynamics/symtropy",
                    "commit_sha": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
                    "tree_sha": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                },
            }
        )
        result = analyze(graph, ["source:commit"])
        execution = next(
            r for r in result["records"] if r["node_id"] == "execution:source"
        )
        self.assertEqual(execution["reason_code"], "SOURCE_DEPENDENCY_CHANGED")
        self.assertEqual(execution["via_edges"], ["source:commit"])

    def test_no_impact_for_isolated_source(self):
        graph = copy.deepcopy(EXAMPLE)
        graph["nodes"].append(
            {
                "schema_version": "luminous.formal-provenance.v0",
                "kind": "Definition",
                "id": "definition:isolated",
                "label": "Unreferenced definition",
            }
        )
        result = analyze(graph, ["definition:isolated"])
        self.assertEqual(result["impact_status"], "NoImpact")
        self.assertEqual(result["records"], [])


if __name__ == "__main__":
    unittest.main()
