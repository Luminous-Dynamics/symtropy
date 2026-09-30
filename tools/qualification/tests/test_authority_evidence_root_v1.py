from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from tools.qualification import authority_evidence_manifest_v1 as manifest
from tools.qualification import authority_evidence_root_v1 as root
from tools.qualification.tests.test_authority_evidence_envelope_v1 import write as write_envelope_fixture


NAMES = (
    "authority-dispatch-payload.json",
    "authority-dispatch-evidence.json",
    "verifier-release-v1.json",
    "qualification-contract-v1.json",
    "qualification-contract-checkout-identity-v1.json",
    "qualification-execution-evidence-v1.json",
)


def materialize_fixture(directory: Path) -> Path:
    with tempfile.TemporaryDirectory() as source:
        source_path = Path(source)
        write_envelope_fixture(source_path)
        mapping = {
            "payload.json": NAMES[0],
            "dispatch-evidence.json": NAMES[1],
            "release.json": NAMES[2],
            "contract.json": NAMES[3],
            "contract-checkout.json": NAMES[4],
            "execution.json": NAMES[5],
        }
        for source_name, target_name in mapping.items():
            (directory / target_name).write_bytes((source_path / source_name).read_bytes())
    manifest_path = directory / "authority-evidence-manifest-v1.json"
    value = manifest.build(directory)
    manifest_path.write_bytes(manifest.canonical(value) + b"\n")
    return manifest_path


class EvidenceRootTests(unittest.TestCase):
    def build_root(self, directory: Path, manifest_path: Path) -> Path:
        output = directory / "authority-evidence-root-v1.json"
        self.assertEqual(root.main([
            "build",
            "--evidence-dir", str(directory),
            "--manifest", str(manifest_path),
            "--output", str(output),
        ]), 0)
        return output

    def verify_root(self, directory: Path, manifest_path: Path, root_path: Path) -> int:
        return root.main([
            "verify",
            "--root", str(root_path),
            "--evidence-dir", str(directory),
            "--manifest", str(manifest_path),
        ])

    def test_recursive_root_round_trip(self):
        with tempfile.TemporaryDirectory() as d:
            directory = Path(d)
            manifest_path = materialize_fixture(directory)
            root_path = self.build_root(directory, manifest_path)
            self.assertEqual(self.verify_root(directory, manifest_path, root_path), 0)

    def test_root_rejects_manifest_byte_substitution(self):
        with tempfile.TemporaryDirectory() as d:
            directory = Path(d)
            manifest_path = materialize_fixture(directory)
            root_path = self.build_root(directory, manifest_path)
            manifest_path.write_bytes(manifest_path.read_bytes() + b" ")
            self.assertEqual(self.verify_root(directory, manifest_path, root_path), 2)

    def test_build_rejects_semantically_inconsistent_member_set(self):
        with tempfile.TemporaryDirectory() as d:
            directory = Path(d)
            manifest_path = materialize_fixture(directory)
            execution = json.loads((directory / NAMES[5]).read_text())
            execution["subject_head_sha"] = "9" * 40
            (directory / NAMES[5]).write_text(json.dumps(execution))
            manifest_path.write_bytes(manifest.canonical(manifest.build(directory)) + b"\n")
            output = directory / "authority-evidence-root-v1.json"
            self.assertEqual(root.main([
                "build",
                "--evidence-dir", str(directory),
                "--manifest", str(manifest_path),
                "--output", str(output),
            ]), 2)
            self.assertFalse(output.exists())


    def test_relationship_contracts_are_canonical_and_semantically_bound(self):
        edges = tuple(spec["edge"] for spec in root.GRAPH_RELATIONSHIP_CONTRACTS)
        self.assertEqual(edges, root.GRAPH_EDGES)
        self.assertEqual(len(edges), len(set(edges)))
        self.assertEqual(len(edges), 6)
        for spec in root.GRAPH_RELATIONSHIP_CONTRACTS:
            self.assertGreaterEqual(len(spec["checks"]), 1)
            for operand_pair in spec["checks"]:
                self.assertEqual(len(operand_pair), 2)
                for operand in operand_pair:
                    self.assertGreaterEqual(len(operand), 2)


    def test_relationship_contract_catalog_rejects_unknown_operand_field(self):
        original = root.GRAPH_RELATIONSHIP_CONTRACTS
        try:
            malformed = dict(original[0])
            malformed["checks"] = (
                (("value", "dispatch_payload", "not_a_real_field"),
                 ("graph", "contract_identity", "commit_sha")),
            )
            root.GRAPH_RELATIONSHIP_CONTRACTS = (malformed, *original[1:])
            with self.assertRaises(root.EvidenceRootValidationError):
                root._validate_relationship_contract_catalog()
        finally:
            root.GRAPH_RELATIONSHIP_CONTRACTS = original

    def test_relationship_contract_catalog_rejects_missing_checks(self):
        original = root.GRAPH_RELATIONSHIP_CONTRACTS
        try:
            malformed = dict(original[0])
            malformed["checks"] = ()
            root.GRAPH_RELATIONSHIP_CONTRACTS = (malformed, *original[1:])
            with self.assertRaises(root.EvidenceRootValidationError):
                root._validate_relationship_contract_catalog()
        finally:
            root.GRAPH_RELATIONSHIP_CONTRACTS = original

    def test_relationship_contract_catalog_rejects_unknown_operand_kind(self):
        original = root.GRAPH_RELATIONSHIP_CONTRACTS
        try:
            malformed = dict(original[0])
            malformed["checks"] = (
                (("mystery", "dispatch_payload"),
                 ("graph", "contract_identity", "commit_sha")),
            )
            root.GRAPH_RELATIONSHIP_CONTRACTS = (malformed, *original[1:])
            with self.assertRaises(root.EvidenceRootValidationError):
                root._validate_relationship_contract_catalog()
        finally:
            root.GRAPH_RELATIONSHIP_CONTRACTS = original

    def test_relationship_catalog_uses_authoritative_validator_vocabularies(self):
        from tools.qualification import authority_dispatch_evidence_v1 as dispatch_evidence
        from tools.qualification import authority_dispatch_v1 as dispatch
        from tools.qualification import authority_evidence_envelope_v1 as envelope
        from tools.qualification import qualification_contract_v1 as contract_v1

        self.assertEqual(root.RELATIONSHIP_VALUE_FIELDS["dispatch_payload"], frozenset(dispatch.FIELDS))
        self.assertEqual(root.RELATIONSHIP_VALUE_FIELDS["dispatch_evidence"], frozenset(dispatch_evidence.FIELDS))
        self.assertEqual(root.RELATIONSHIP_VALUE_FIELDS["qualification_contract"], frozenset(contract_v1.FIELDS))
        self.assertEqual(root.RELATIONSHIP_VALUE_FIELDS["execution_evidence"], frozenset(envelope.REQUIRED_EXECUTION_FIELDS))

    def test_semantic_binding_coverage_is_derived_not_duplicated(self):
        value_sources = {
            source
            for source in root.SEMANTIC_BINDING_SOURCES.values()
        }
        declared = {
            (operand[1], operand[2])
            for spec in root.GRAPH_RELATIONSHIP_CONTRACTS
            for pair in spec["checks"]
            for operand in pair
            if operand[0] == "value"
        }
        self.assertTrue(value_sources <= declared)

    def test_semantic_binding_catalog_rejects_phantom_source_field(self):
        original = root.SEMANTIC_BINDING_SOURCES
        try:
            root.SEMANTIC_BINDING_SOURCES = {
                **original,
                "phantom": ("execution_evidence", "not_a_real_field"),
            }
            with self.assertRaises(root.EvidenceRootValidationError):
                root._validate_relationship_contract_catalog()
        finally:
            root.SEMANTIC_BINDING_SOURCES = original

    def test_graph_semantic_binding_catalog_rejects_phantom_source_field(self):
        original = root.GRAPH_SEMANTIC_FIELDS
        try:
            root.GRAPH_SEMANTIC_FIELDS = {
                **original,
                ("subject_identity", "head_sha"): ("execution_evidence", "not_a_real_field"),
            }
            with self.assertRaises(root.EvidenceRootValidationError):
                root._validate_relationship_contract_catalog()
        finally:
            root.GRAPH_SEMANTIC_FIELDS = original

    def test_relationship_contract_catalog_covers_only_declared_graph_semantics(self):
        root._validate_relationship_contract_catalog()
        value_fields = {
            (operand[1], operand[2])
            for spec in root.GRAPH_RELATIONSHIP_CONTRACTS
            for pair in spec["checks"]
            for operand in pair
            if operand[0] == "value"
        }
        self.assertEqual(
            value_fields,
            {
                ("dispatch_payload", "contract_commit_sha"),
                ("dispatch_evidence", "release_sha256"),
                ("qualification_contract", "subject_head_sha"),
                ("qualification_contract_checkout_identity", "contract_blob_sha"),
                ("execution_evidence", "contract_commit_sha"),
                ("execution_evidence", "subject_head_sha"),
                ("execution_evidence", "final_result"),
            },
        )

    def test_root_exposes_typed_provenance_graph(self):
        with tempfile.TemporaryDirectory() as d:
            directory = Path(d)
            manifest_path = materialize_fixture(directory)
            root_path = self.build_root(directory, manifest_path)
            value = json.loads(root_path.read_text())
            graph = value["provenance_graph"]
            self.assertEqual(
                [node["id"] for node in graph["nodes"]],
                list(root.GRAPH_NODE_IDS),
            )
            self.assertEqual(
                [(edge["from"], edge["type"], edge["to"]) for edge in graph["edges"]],
                list(root.GRAPH_EDGES),
            )
            self.assertEqual(graph["nodes"][-1]["value"], value["semantic_bindings"]["final_result"])

    def test_root_rejects_provenance_graph_relationship_substitution(self):
        with tempfile.TemporaryDirectory() as d:
            directory = Path(d)
            manifest_path = materialize_fixture(directory)
            root_path = self.build_root(directory, manifest_path)
            value = json.loads(root_path.read_text())
            value["provenance_graph"]["edges"][0]["type"] = "produces"
            root_path.write_text(json.dumps(value))
            self.assertEqual(self.verify_root(directory, manifest_path, root_path), 2)


    def test_root_rejects_graph_identity_substitution(self):
        with tempfile.TemporaryDirectory() as d:
            directory = Path(d)
            manifest_path = materialize_fixture(directory)
            root_path = self.build_root(directory, manifest_path)
            value = json.loads(root_path.read_text())
            value["provenance_graph"]["nodes"][6]["commit_sha"] = "a" * 40
            root_path.write_text(json.dumps(value))
            self.assertEqual(self.verify_root(directory, manifest_path, root_path), 2)

    def test_graph_semantics_are_derived_from_retained_evidence(self):
        with tempfile.TemporaryDirectory() as d:
            directory = Path(d)
            manifest_path = materialize_fixture(directory)
            root_path = self.build_root(directory, manifest_path)
            value = json.loads(root_path.read_text())
            execution = json.loads((directory / NAMES[5]).read_text())
            self.assertEqual(
                value["provenance_graph"]["nodes"][7]["head_sha"],
                execution["subject_head_sha"],
            )
            self.assertEqual(
                value["provenance_graph"]["nodes"][8]["value"],
                execution["final_result"],
            )

    def test_root_rejects_member_substitution(self):
        with tempfile.TemporaryDirectory() as d:
            directory = Path(d)
            manifest_path = materialize_fixture(directory)
            root_path = self.build_root(directory, manifest_path)
            execution = json.loads((directory / NAMES[5]).read_text())
            execution["subject_head_sha"] = "9" * 40
            (directory / NAMES[5]).write_text(json.dumps(execution))
            self.assertEqual(self.verify_root(directory, manifest_path, root_path), 2)

    def test_root_rejects_checkout_tree_substitution(self):
        with tempfile.TemporaryDirectory() as d:
            directory = Path(d)
            manifest_path = materialize_fixture(directory)
            root_path = self.build_root(directory, manifest_path)
            checkout = json.loads((directory / NAMES[4]).read_text())
            checkout["contract_tree_sha"] = "b" * 40
            (directory / NAMES[4]).write_text(json.dumps(checkout))
            self.assertEqual(self.verify_root(directory, manifest_path, root_path), 2)

    def test_root_rejects_semantic_root_substitution(self):
        with tempfile.TemporaryDirectory() as d:
            directory = Path(d)
            manifest_path = materialize_fixture(directory)
            root_path = self.build_root(directory, manifest_path)
            value = json.loads(root_path.read_text())
            value["semantic_bindings"]["contract_blob_sha"] = "0" * 40
            root_path.write_text(json.dumps(value))
            self.assertEqual(self.verify_root(directory, manifest_path, root_path), 2)

    def test_root_rejects_extra_member(self):
        with tempfile.TemporaryDirectory() as d:
            directory = Path(d)
            manifest_path = materialize_fixture(directory)
            root_path = self.build_root(directory, manifest_path)
            (directory / "unexpected.json").write_text("{}")
            self.assertEqual(self.verify_root(directory, manifest_path, root_path), 2)


if __name__ == "__main__":
    unittest.main()
