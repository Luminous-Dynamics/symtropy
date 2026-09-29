#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import pathlib
import sys
import unittest

QUALIFICATION_DIR = pathlib.Path(__file__).resolve().parents[1]
if str(QUALIFICATION_DIR) not in sys.path:
    sys.path.insert(0, str(QUALIFICATION_DIR))

MODULE_PATH = QUALIFICATION_DIR / "validate_manifest_v1_a2.py"
SPEC = importlib.util.spec_from_file_location("validate_manifest_v1_a2", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)


def valid_manifest() -> dict:
    return {
        "schema_id": "luminous.qualification-manifest.v1",
        "profile_id": "qual-001a2-manifest-hardening-v1",
        "profile_version": 1,
        "exact_parent": "d43af527c159da8764bacd04eecb81ff5aef9d8f",
        "required_commit_count": 1,
        "owned_paths": [
            {"kind": "prefix", "path": "tools/qualification"},
        ],
        "profile_files": [
            {
                "path": "tools/qualification/QUALIFICATION_MANIFEST_V1.schema.json",
                "sha256": "11" * 32,
                "byte_len": 4704,
            }
        ],
        "parent_equal_paths": [
            {"kind": "file", "path": "Cargo.lock"},
            {"kind": "file", "path": "Cargo.toml"},
        ],
        "qualification_suite_id": "schema-only-v1",
        "toolchain_id": "python-3.12.10",
        "allow_transient_paths": [],
        "artifact_name": "qual-001a2-manifest-hardening-v1",
    }


def sort_specs(value: dict, field: str) -> None:
    value[field].sort(key=lambda item: (item["path"], item["kind"]))


class QualificationManifestV1A2Tests(unittest.TestCase):
    def test_profile_under_owned_prefix_passes(self) -> None:
        self.assertEqual(
            validator.validate_manifest(valid_manifest())["profile_id"],
            "qual-001a2-manifest-hardening-v1",
        )

    def test_profile_exact_parent_equal_passes(self) -> None:
        value = valid_manifest()
        value["profile_files"][0]["path"] = "Cargo.lock"
        validator.validate_manifest(value)

    def test_profile_under_parent_equal_prefix_passes(self) -> None:
        value = valid_manifest()
        value["profile_files"][0]["path"] = "profiles/schema.json"
        value["parent_equal_paths"] = [
            {"kind": "prefix", "path": "profiles"},
        ]
        validator.validate_manifest(value)

    def test_profile_outside_owned_and_parent_equal_fails(self) -> None:
        value = valid_manifest()
        value["profile_files"][0]["path"] = "docs/floating-profile.json"
        with self.assertRaises(validator.base.ManifestValidationError):
            validator.validate_manifest(value)

    def test_git_control_component_rejected_from_owned_paths(self) -> None:
        value = valid_manifest()
        value["owned_paths"] = [
            {"kind": "file", "path": ".git/config"},
            {"kind": "prefix", "path": "tools/qualification"},
        ]
        sort_specs(value, "owned_paths")
        with self.assertRaises(validator.base.ManifestValidationError):
            validator.validate_manifest(value)

    def test_nested_git_control_component_rejected_from_profile_files(self) -> None:
        value = valid_manifest()
        value["profile_files"][0]["path"] = "foo/.git/index"
        with self.assertRaises(validator.base.ManifestValidationError):
            validator.validate_manifest(value)

    def test_casefolded_git_control_component_rejected_from_parent_equal(self) -> None:
        value = valid_manifest()
        value["parent_equal_paths"].append(
            {"kind": "prefix", "path": ".GIT/objects"}
        )
        sort_specs(value, "parent_equal_paths")
        with self.assertRaises(validator.base.ManifestValidationError):
            validator.validate_manifest(value)

    def test_git_control_component_rejected_from_transient_paths(self) -> None:
        value = valid_manifest()
        value["allow_transient_paths"] = [
            {"kind": "prefix", "path": "scratch/.GiT/cache"},
        ]
        with self.assertRaises(validator.base.ManifestValidationError):
            validator.validate_manifest(value)

    def test_ordinary_dot_paths_remain_valid(self) -> None:
        value = valid_manifest()
        value["owned_paths"] = [
            {"kind": "prefix", "path": ".github/workflows"},
            {"kind": "prefix", "path": ".qualification"},
            {"kind": "prefix", "path": "tools/qualification"},
        ]
        value["profile_files"][0]["path"] = ".qualification/profile.json"
        validator.validate_manifest(value)


if __name__ == "__main__":
    unittest.main()
