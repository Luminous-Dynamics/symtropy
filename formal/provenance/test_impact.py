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
        graph = copy.deepcopy(EXAMPLE)
        result = analyze(graph, ["theorem:physics-body-ref-resolution-v1"])
        self.assertEqual(result["impact_status"], "RevalidationRequired")
        ids = {record["node_id"] for record in result["records"]}
        self.assertIn("proof:physics-body-ref-resolution-v1", ids)
        self.assertIn("checker:example-v0", ids)

    def test_impact_is_deterministic(self):
        graph = copy.deepcopy(EXAMPLE)
        first = analyze(graph, ["theorem:physics-body-ref-resolution-v1"])
        reordered = copy.deepcopy(EXAMPLE)
        reordered["nodes"].reverse()
        reordered["edges"].reverse()
        second = analyze(reordered, ["theorem:physics-body-ref-resolution-v1"])
        self.assertEqual(first, second)

    def test_unknown_change_fails_closed(self):
        with self.assertRaises(ValueError):
            analyze(copy.deepcopy(EXAMPLE), ["node:missing"])

    def test_checker_change_propagates_to_proof_execution_metadata(self):
        graph = copy.deepcopy(EXAMPLE)
        graph["nodes"].append({
            "schema_version": "luminous.formal-provenance.v0",
            "kind": "ProofExecution",
            "id": "execution:example",
            "label": "Example execution",
            "metadata": {"checker_id": "checker:example-v0"}
        })
        result = analyze(graph, ["checker:example-v0"])
        ids = {record["node_id"] for record in result["records"]}
        self.assertIn("execution:example", ids)
        execution = next(r for r in result["records"] if r["node_id"] == "execution:example")
        self.assertEqual(execution["reason_code"], "CHECKER_CHANGED")

    def test_no_impact_for_isolated_source(self):
        graph = copy.deepcopy(EXAMPLE)
        graph["nodes"].append({
            "schema_version": "luminous.formal-provenance.v0",
            "kind": "Definition",
            "id": "definition:isolated",
            "label": "Unreferenced definition"
        })
        result = analyze(graph, ["definition:isolated"])
        self.assertEqual(result["impact_status"], "NoImpact")
        self.assertEqual(result["records"], [])


if __name__ == "__main__":
    unittest.main()
