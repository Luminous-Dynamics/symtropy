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

    def test_semantic_binding_projections_are_derived_not_duplicated(self):
        self.assertEqual(
            root.SEMANTIC_BINDING_SOURCES,
            {binding: spec["source"] for binding, spec in root.SEMANTIC_BINDINGS.items()},
        )
        self.assertEqual(
            root.GRAPH_SEMANTIC_FIELDS,
            {spec["graph"]: spec["source"] for spec in root.SEMANTIC_BINDINGS.values()},
        )


    def test_semantic_binding_catalog_rejects_unclassified_relationship_check(self):
        original = root.SEMANTIC_BINDINGS
        try:
            malformed = {
                **original,
                "final_result": {
                    **original["final_result"],
                    "enforcement_checks": (),
                },
            }
            root.SEMANTIC_BINDINGS = malformed
            with self.assertRaises(root.EvidenceRootValidationError):
                root._validate_relationship_contract_catalog()
        finally:
            root.SEMANTIC_BINDINGS = original

    def test_semantic_binding_catalog_rejects_duplicate_check_classification(self):
        original = root.SEMANTIC_BINDINGS
        try:
            malformed = {
                **original,
                "contract_commit_sha": {
                    **original["contract_commit_sha"],
                    "enforcement_checks": (
                        *original["contract_commit_sha"]["enforcement_checks"],
                        original["contract_commit_sha"]["enforcement_checks"][0],
                    ),
                },
            }
            root.SEMANTIC_BINDINGS = malformed
            with self.assertRaises(root.EvidenceRootValidationError):
                root._validate_relationship_contract_catalog()
        finally:
            root.SEMANTIC_BINDINGS = original


    def test_semantic_binding_catalog_rejects_check_without_binding_reference(self):
        original = root.SEMANTIC_BINDINGS
        try:
            malformed = {
                **original,
                "final_result": {
                    **original["final_result"],
                    "enforcement_checks": (
                        (
                            ("execution_evidence", "produces", "final_result"),
                            (
                                ("value", "execution_evidence", "subject_head_sha"),
                                ("graph", "final_result", "value"),
                            ),
                        ),
                    ),
                },
            }
            root.SEMANTIC_BINDINGS = malformed
            with self.assertRaises(root.EvidenceRootValidationError):
                root._validate_relationship_contract_catalog()
        finally:
            root.SEMANTIC_BINDINGS = original

    def test_semantic_binding_catalog_rejects_missing_enforcing_relationship(self):
        original = root.SEMANTIC_BINDINGS
        try:
            malformed = {
                **original,
                "contract_commit_sha": {
                    **original["contract_commit_sha"],
                    "enforced_by": (),
                },
            }
            root.SEMANTIC_BINDINGS = malformed
            with self.assertRaises(root.EvidenceRootValidationError):
                root._validate_relationship_contract_catalog()
        finally:
            root.SEMANTIC_BINDINGS = original

    def test_semantic_binding_catalog_rejects_wrong_enforcement_anchor(self):
        original = root.SEMANTIC_BINDINGS
        try:
            malformed = {
                **original,
                "final_result": {
                    **original["final_result"],
                    "enforcement_checks": (
                        (
                            ("execution_evidence", "produces", "final_result"),
                            (
                                ("graph", "subject_identity", "head_sha"),
                                ("value", "execution_evidence", "final_result"),
                            ),
                        ),
                    ),
                },
            }
            root.SEMANTIC_BINDINGS = malformed
            with self.assertRaises(root.EvidenceRootValidationError):
                root._validate_relationship_contract_catalog()
        finally:
            root.SEMANTIC_BINDINGS = original

    def test_semantic_binding_catalog_rejects_missing_enforcement_anchor(self):
        original = root.SEMANTIC_BINDINGS
        try:
            malformed = {
                **original,
                "contract_blob_sha": {
                    **original["contract_blob_sha"],
                    "enforcement_checks": (),
                },
            }
            root.SEMANTIC_BINDINGS = malformed
            with self.assertRaises(root.EvidenceRootValidationError):
                root._validate_relationship_contract_catalog()
        finally:
            root.SEMANTIC_BINDINGS = original

    def test_semantic_binding_catalog_rejects_phantom_graph_projection(self):
        original = root.SEMANTIC_BINDINGS
        try:
            malformed = {
                **original,
                "subject_head_sha": {
                    **original["subject_head_sha"],
                    "graph": ("subject_identity", "not_a_real_field"),
                },
            }
            root.SEMANTIC_BINDINGS = malformed
            with self.assertRaises(root.EvidenceRootValidationError):
                root._validate_relationship_contract_catalog()
        finally:
            root.SEMANTIC_BINDINGS = original

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



    def test_security_surface_is_exactly_covered_by_semantic_bindings(self):
        root._validate_relationship_contract_catalog()
        self.assertEqual(
            {
                spec["source"]
                for spec in root.SEMANTIC_BINDINGS.values()
            },
            root.SECURITY_SURFACE_FIELDS,
        )

    def test_semantic_binding_catalog_rejects_security_surface_coverage_gap(self):
        original = root.SEMANTIC_BINDINGS
        try:
            malformed = {
                binding: spec
                for binding, spec in original.items()
                if binding != "final_result"
            }
            root.SEMANTIC_BINDINGS = malformed
            with self.assertRaises(root.EvidenceRootValidationError):
                root._validate_relationship_contract_catalog()
        finally:
            root.SEMANTIC_BINDINGS = original

    def test_semantic_binding_catalog_rejects_phantom_security_surface_field(self):
        original = root.SECURITY_SURFACE_FIELDS
        try:
            root.SECURITY_SURFACE_FIELDS = original | {
                ("execution_evidence", "not_a_real_field"),
            }
            with self.assertRaises(root.EvidenceRootValidationError):
                root._validate_relationship_contract_catalog()
        finally:
            root.SECURITY_SURFACE_FIELDS = original

    def test_semantic_binding_policy_is_closed_and_explicit(self):
        root._validate_relationship_contract_catalog()
        policy = {
            binding: spec["coverage_class"]
            for binding, spec in root.SEMANTIC_BINDINGS.items()
        }
        self.assertEqual(
            policy,
            {
                "contract_commit_sha": "enforced",
                "verifier_release_sha256": "enforced",
                "contract_tree_sha": "retained",
                "contract_blob_sha": "enforced",
                "subject_head_sha": "enforced",
                "contract_subject_head_sha": "enforced",
                "subject_tree_sha": "retained",
                "final_result": "enforced",
            },
        )

    def test_semantic_binding_catalog_rejects_retained_binding_with_enforcement(self):
        original = root.SEMANTIC_BINDINGS
        try:
            malformed = {
                **original,
                "contract_tree_sha": {
                    **original["contract_tree_sha"],
                    "coverage_class": "retained",
                    "enforced_by": original["contract_commit_sha"]["enforced_by"],
                    "enforcement_checks": original["contract_commit_sha"]["enforcement_checks"],
                },
            }
            root.SEMANTIC_BINDINGS = malformed
            with self.assertRaises(root.EvidenceRootValidationError):
                root._validate_relationship_contract_catalog()
        finally:
            root.SEMANTIC_BINDINGS = original

    def test_root_rejects_semantic_binding_policy_substitution(self):
        with tempfile.TemporaryDirectory() as d:
            directory = Path(d)
            manifest_path = materialize_fixture(directory)
            root_path = self.build_root(directory, manifest_path)
            value = json.loads(root_path.read_text())
            value["semantic_binding_policy"]["contract_tree_sha"] = "enforced"
            root_path.write_text(json.dumps(value))
            self.assertEqual(self.verify_root(directory, manifest_path, root_path), 2)

    def test_root_exposes_exact_semantic_security_mapping(self):
        with tempfile.TemporaryDirectory() as d:
            directory = Path(d)
            manifest_path = materialize_fixture(directory)
            root_path = self.build_root(directory, manifest_path)
            value = json.loads(root_path.read_text())
            surface = value["semantic_security_surface"]
            self.assertEqual(set(surface), set(root.SEMANTIC_BINDINGS))
            for binding, spec in root.SEMANTIC_BINDINGS.items():
                self.assertEqual(
                    surface[binding]["source"],
                    {"role": spec["source"][0], "field": spec["source"][1]},
                )
                self.assertEqual(
                    surface[binding]["graph"],
                    {"node": spec["graph"][0], "field": spec["graph"][1]},
                )
                self.assertEqual(surface[binding]["coverage_class"], spec["coverage_class"])
                self.assertEqual(surface[binding]["purpose"], spec["purpose"])
                self.assertEqual(
                    surface[binding]["enforcement"]["enforced_by"],
                    [list(edge) for edge in spec["enforced_by"]],
                )
                self.assertEqual(
                    surface[binding]["enforcement"]["checks"],
                    [
                        {
                            "edge": list(edge),
                            "operands": [list(operand) for operand in check_pair],
                        }
                        for edge, check_pair in spec["enforcement_checks"]
                    ],
                )

    def test_root_rejects_semantic_security_mapping_substitution(self):
        with tempfile.TemporaryDirectory() as d:
            directory = Path(d)
            manifest_path = materialize_fixture(directory)
            root_path = self.build_root(directory, manifest_path)
            value = json.loads(root_path.read_text())
            value["semantic_security_surface"]["contract_commit_sha"]["graph"]["field"] = "tree_sha"
            root_path.write_text(json.dumps(value))
            self.assertEqual(self.verify_root(directory, manifest_path, root_path), 2)


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
