#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import pathlib
import unittest

MODULE_PATH = (
    pathlib.Path(__file__).resolve().parents[1] / "validate_manifest_v1.py"
)
SPEC = importlib.util.spec_from_file_location(
    "validate_manifest_v1", MODULE_PATH
)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)


def valid_manifest() -> dict:
    return {
        "schema_id": "luminous.qualification-manifest.v1",
        "profile_id": "qual-001a-manifest-validator-v1",
        "profile_version": 1,
        "exact_parent": "d56d6a6c1381dc131d493f0713b657f01c98cb80",
        "required_commit_count": 1,
        "owned_paths": [
            {
                "kind": "file",
                "path": ".github/workflows/qual-001a-manifest-validator-v1.yml",
            },
            {
                "kind": "file",
                "path": ".qualification/qual-001a-manifest-validator-v1.json",
            },
            {"kind": "prefix", "path": "tools/qualification"},
        ],
        "profile_files": [
            {
                "path": "tools/qualification/QUALIFICATION_MANIFEST_V1.schema.json",
                "sha256": "11" * 32,
                "byte_len": 3000,
            }
        ],
        "parent_equal_paths": [
            {"kind": "file", "path": "Cargo.lock"},
            {"kind": "file", "path": "Cargo.toml"},
        ],
        "qualification_suite_id": "schema-only-v1",
        "toolchain_id": "python-3.12.10",
        "allow_transient_paths": [],
        "artifact_name": "qual-001a-manifest-validator-v1",
    }


class QualificationManifestV1Tests(unittest.TestCase):
    def test_valid_manifest_passes(self) -> None:
        self.assertEqual(
            validator.validate_manifest(valid_manifest())["profile_version"],
            1,
        )

    def test_schema_registry_matches_validator(self) -> None:
        schema_path = MODULE_PATH.parent / "QUALIFICATION_MANIFEST_V1.schema.json"
        schema = json.loads(schema_path.read_text(encoding="utf-8"))

        self.assertFalse(schema["additionalProperties"])
        self.assertEqual(set(schema["required"]), validator.ROOT_FIELDS)

        suite_ids = set(
            schema["properties"]["qualification_suite_id"]["enum"]
        )
        self.assertEqual(suite_ids, set(validator.SUITE_TOOLCHAINS))

        toolchain_ids = set(schema["properties"]["toolchain_id"]["enum"])
        expected_toolchains = set().union(*validator.SUITE_TOOLCHAINS.values())
        self.assertEqual(toolchain_ids, expected_toolchains)

        self.assertFalse(schema["$defs"]["pathRef"]["additionalProperties"])
        self.assertFalse(schema["$defs"]["profileFile"]["additionalProperties"])

    def test_unknown_root_field_fails_closed(self) -> None:
        value = valid_manifest()
        value["shell"] = "rm -rf /"
        with self.assertRaises(validator.ManifestValidationError):
            validator.validate_manifest(value)

    def test_duplicate_json_key_is_rejected(self) -> None:
        raw = b'{"schema_id":"a","schema_id":"b"}'
        with self.assertRaises(validator.ManifestValidationError):
            validator.load_manifest_bytes(raw)

    def test_nonfinite_json_number_is_rejected(self) -> None:
        raw = json.dumps(valid_manifest()).replace(
            '"profile_version": 1',
            '"profile_version": NaN',
        ).encode()
        with self.assertRaises(validator.ManifestValidationError):
            validator.load_manifest_bytes(raw)

    def test_parent_must_be_lowercase_full_sha1(self) -> None:
        value = valid_manifest()
        value["exact_parent"] = "A" * 40
        with self.assertRaises(validator.ManifestValidationError):
            validator.validate_manifest(value)

    def test_path_traversal_is_rejected(self) -> None:
        value = valid_manifest()
        value["owned_paths"][0]["path"] = "../workflow.yml"
        value["owned_paths"].sort(key=lambda item: (item["path"], item["kind"]))
        with self.assertRaises(validator.ManifestValidationError):
            validator.validate_manifest(value)

    def test_backslash_path_is_rejected(self) -> None:
        value = valid_manifest()
        value["owned_paths"][0]["path"] = ".github\\workflow.yml"
        value["owned_paths"].sort(key=lambda item: (item["path"], item["kind"]))
        with self.assertRaises(validator.ManifestValidationError):
            validator.validate_manifest(value)

    def test_owned_paths_must_be_sorted(self) -> None:
        value = valid_manifest()
        value["owned_paths"] = list(reversed(value["owned_paths"]))
        with self.assertRaises(validator.ManifestValidationError):
            validator.validate_manifest(value)

    def test_overlapping_owned_prefixes_are_rejected(self) -> None:
        value = valid_manifest()
        value["owned_paths"].append(
            {"kind": "file", "path": "tools/qualification/extra.py"}
        )
        value["owned_paths"].sort(key=lambda item: (item["path"], item["kind"]))
        with self.assertRaises(validator.ManifestValidationError):
            validator.validate_manifest(value)

    def test_parent_equal_must_not_overlap_owned(self) -> None:
        value = valid_manifest()
        value["parent_equal_paths"] = [
            {"kind": "prefix", "path": "tools"},
        ]
        with self.assertRaises(validator.ManifestValidationError):
            validator.validate_manifest(value)

    def test_profile_digest_must_be_lowercase_sha256(self) -> None:
        value = valid_manifest()
        value["profile_files"][0]["sha256"] = "GG" * 32
        with self.assertRaises(validator.ManifestValidationError):
            validator.validate_manifest(value)

    def test_suite_toolchain_pair_must_be_reviewed(self) -> None:
        value = valid_manifest()
        value["toolchain_id"] = "rust-1.96.0"
        with self.assertRaises(validator.ManifestValidationError):
            validator.validate_manifest(value)

    def test_transient_path_must_not_cover_profile_file(self) -> None:
        value = valid_manifest()
        value["allow_transient_paths"] = [
            {"kind": "prefix", "path": "tools"},
        ]
        with self.assertRaises(validator.ManifestValidationError):
            validator.validate_manifest(value)

    def test_transient_path_must_not_overlap_owned_source(self) -> None:
        value = valid_manifest()
        value["allow_transient_paths"] = [
            {"kind": "prefix", "path": ".github"},
        ]
        with self.assertRaises(validator.ManifestValidationError):
            validator.validate_manifest(value)

    def test_boolean_is_not_accepted_as_integer(self) -> None:
        value = valid_manifest()
        value["required_commit_count"] = True
        with self.assertRaises(validator.ManifestValidationError):
            validator.validate_manifest(value)

    def test_bom_is_rejected(self) -> None:
        raw = b"\xef\xbb\xbf" + json.dumps(valid_manifest()).encode()
        with self.assertRaises(validator.ManifestValidationError):
            validator.load_manifest_bytes(raw)


if __name__ == "__main__":
    unittest.main()
