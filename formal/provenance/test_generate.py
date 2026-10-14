import copy
import json
import pathlib
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from generate import GRAPH_VERSION, materialize  # noqa: E402


with open(ROOT / "example-v0.json", encoding="utf-8") as f:
    EXAMPLE = json.load(f)


class GeneratorTests(unittest.TestCase):
    def test_materialization_is_reproducible(self):
        first = materialize(copy.deepcopy(EXAMPLE))
        second = materialize(copy.deepcopy(EXAMPLE))
        self.assertEqual(first, second)
        self.assertEqual(len(first["graph_digest"]), 64)

    def test_node_and_edge_input_order_does_not_change_digest(self):
        graph = copy.deepcopy(EXAMPLE)
        reordered = copy.deepcopy(EXAMPLE)
        reordered["nodes"] = list(reversed(reordered["nodes"]))
        reordered["edges"] = list(reversed(reordered["edges"]))
        self.assertEqual(
            materialize(graph)["graph_digest"],
            materialize(reordered)["graph_digest"],
        )

    def test_json_object_insertion_order_does_not_change_digest(self):
        graph = copy.deepcopy(EXAMPLE)
        reordered = {
            "edges": graph["edges"],
            "nodes": graph["nodes"],
            "schema_version": graph["schema_version"],
        }
        self.assertEqual(
            materialize(graph)["graph_digest"],
            materialize(reordered)["graph_digest"],
        )

    def test_semantic_content_change_changes_digest(self):
        graph = copy.deepcopy(EXAMPLE)
        graph["nodes"][0]["label"] = "Changed theorem label"
        self.assertNotEqual(
            materialize(EXAMPLE)["graph_digest"],
            materialize(graph)["graph_digest"],
        )

    def test_nodes_and_edges_are_canonically_sorted(self):
        graph = copy.deepcopy(EXAMPLE)
        output = materialize(graph)
        self.assertEqual(
            output["nodes"],
            sorted(output["nodes"], key=lambda node: (node["kind"], node["id"])),
        )
        self.assertEqual(
            output["edges"],
            sorted(
                output["edges"],
                key=lambda edge: (
                    edge["relation"], edge["source"], edge["target"], edge["id"]
                ),
            ),
        )

    def test_digest_is_hash_of_payload_without_digest(self):
        import hashlib

        payload = {key: value for key, value in materialize(EXAMPLE).items() if key != "graph_digest"}
        canonical = json.dumps(
            payload, ensure_ascii=False, sort_keys=True, separators=(",", ":")
        ).encode("utf-8")
        expected = hashlib.sha256(canonical).hexdigest()
        self.assertEqual(materialize(EXAMPLE)["graph_digest"], expected)

    def test_invalid_graph_fails_closed(self):
        graph = copy.deepcopy(EXAMPLE)
        graph["edges"][0]["target"] = "theorem:missing"
        with self.assertRaises(ValueError):
            materialize(graph)

    def test_graph_version_is_explicit(self):
        self.assertEqual(materialize(EXAMPLE)["graph_version"], GRAPH_VERSION)


if __name__ == "__main__":
    unittest.main()
