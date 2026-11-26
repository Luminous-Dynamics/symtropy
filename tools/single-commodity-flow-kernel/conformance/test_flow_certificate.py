"""Adversarial conformance tests for the independent portable-certificate verifier."""
from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from verify_flow_certificate import (
    CertificateError,
    graph_digest,
    load_json_strict,
    verify_bundle,
)

FIXTURE = Path(__file__).with_name("fixtures") / "flow-certificate-v1.synthetic.json"


def two_path_bundle():
    graph = {
        "commodity_ref": "electricity",
        "capacity_unit_ref": "mWh",
        "quantity_boundary_ref": "test-boundary",
        "nodes": [{"id": "s"}, {"id": "a"}, {"id": "b"}, {"id": "t"}],
        "edges": [
            {"id": "e1", "from": "s", "to": "a", "capacity": 1, "metadata_ref": None},
            {"id": "e2", "from": "s", "to": "b", "capacity": 1, "metadata_ref": None},
            {"id": "e3", "from": "a", "to": "t", "capacity": 1, "metadata_ref": None},
            {"id": "e4", "from": "b", "to": "t", "capacity": 1, "metadata_ref": None},
        ],
        "source": "s",
        "sink": "t",
    }
    result = {
        "certificate_schema": "luminous-flow-certificate-v1",
        "graph_sha256": graph_digest(graph),
        "commodity_ref": "electricity",
        "capacity_unit_ref": "mWh",
        "quantity_boundary_ref": "test-boundary",
        "kernel_version": "single-commodity-directed-flow-v0.1.0",
        "supported_model_class": "single_commodity_directed_integer_capacity",
        "source": "s",
        "sink": "t",
        "max_flow": 2,
        "edge_flows": [
            {"id": "e1", "from": "s", "to": "a", "capacity": 1, "flow": 1, "metadata_ref": None},
            {"id": "e2", "from": "s", "to": "b", "capacity": 1, "flow": 1, "metadata_ref": None},
            {"id": "e3", "from": "a", "to": "t", "capacity": 1, "flow": 1, "metadata_ref": None},
            {"id": "e4", "from": "b", "to": "t", "capacity": 1, "flow": 1, "metadata_ref": None},
        ],
        "source_side_nodes": ["s"],
        "min_cut": {
            "capacity": 2,
            "edges": [
                {"id": "e1", "from": "s", "to": "a", "capacity": 1, "metadata_ref": None},
                {"id": "e2", "from": "s", "to": "b", "capacity": 1, "metadata_ref": None},
            ],
        },
    }
    return {"schema_version": "luminous-flow-certificate-v1", "graph": graph, "result": result}


class PortableCertificateTests(unittest.TestCase):
    def setUp(self):
        self.bundle = json.loads(FIXTURE.read_text(encoding="utf-8"))

    def test_golden_bundle_verifies(self):
        report = verify_bundle(self.bundle)
        self.assertTrue(report["valid"])
        self.assertEqual(report["maximum_flow"], 4)
        self.assertEqual(report["minimum_cut"], 4)
        self.assertEqual(report["graph_sha256"], self.bundle["result"]["graph_sha256"])

    def test_graph_substitution_is_rejected(self):
        changed = copy.deepcopy(self.bundle)
        changed["graph"]["edges"][0]["capacity"] += 1
        with self.assertRaises(CertificateError):
            verify_bundle(changed)

    def test_wrong_unit_is_rejected(self):
        changed = copy.deepcopy(self.bundle)
        changed["graph"]["capacity_unit_ref"] = "kWh"
        with self.assertRaises(CertificateError):
            verify_bundle(changed)

    def test_missing_or_duplicated_edge_witness_is_rejected(self):
        missing = copy.deepcopy(self.bundle)
        missing["result"]["edge_flows"].pop()
        with self.assertRaises(CertificateError):
            verify_bundle(missing)
        duplicated = copy.deepcopy(self.bundle)
        duplicated["result"]["edge_flows"][-1] = copy.deepcopy(duplicated["result"]["edge_flows"][0])
        with self.assertRaises(CertificateError):
            verify_bundle(duplicated)

    def test_nonconserving_flow_is_rejected(self):
        changed = copy.deepcopy(self.bundle)
        changed["result"]["edge_flows"][0]["flow"] = 1
        with self.assertRaises(CertificateError):
            verify_bundle(changed)

    def test_equal_capacity_alternate_cut_partition_is_rejected(self):
        bundle = two_path_bundle()
        verify_bundle(bundle)
        changed = copy.deepcopy(bundle)
        # {s,a} has cut capacity 2 as well, but it is not residual-reachable
        # from source for the submitted saturated two-path flow.
        changed["result"]["source_side_nodes"] = ["a", "s"]
        changed["result"]["min_cut"]["edges"] = [
            {"id": "e2", "from": "s", "to": "b", "capacity": 1, "metadata_ref": None},
            {"id": "e3", "from": "a", "to": "t", "capacity": 1, "metadata_ref": None},
        ]
        with self.assertRaises(CertificateError):
            verify_bundle(changed)

    def test_uint64_boundaries_are_exact_python_integers(self):
        for value in (0, (1 << 53) - 1, (1 << 53) + 1, (1 << 63), (1 << 64) - 1):
            self.assertIsInstance(value, int)
        changed = copy.deepcopy(self.bundle)
        changed["graph"]["edges"][0]["capacity"] = 1 << 64
        with self.assertRaises(CertificateError):
            verify_bundle(changed)
        changed = copy.deepcopy(self.bundle)
        changed["graph"]["edges"][0]["capacity"] = 1.0
        with self.assertRaises(CertificateError):
            verify_bundle(changed)
        changed = copy.deepcopy(self.bundle)
        changed["graph"]["edges"][0]["capacity"] = True
        with self.assertRaises(CertificateError):
            verify_bundle(changed)

    def test_unknown_fields_and_duplicate_json_keys_are_rejected(self):
        changed = copy.deepcopy(self.bundle)
        changed["unrecognized"] = 1
        with self.assertRaises(CertificateError):
            verify_bundle(changed)
        with self.assertRaises(CertificateError):
            load_json_strict('{"schema_version":"x","schema_version":"y"}')

    def test_floating_point_tokens_and_nan_are_rejected_at_parse(self):
        with self.assertRaises(CertificateError):
            load_json_strict('{"capacity":1.0}')
        with self.assertRaises(CertificateError):
            load_json_strict('{"capacity":NaN}')


if __name__ == "__main__":
    unittest.main()
