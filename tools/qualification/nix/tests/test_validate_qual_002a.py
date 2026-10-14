#!/usr/bin/env python3
"""Hostile reference corpus for QUAL-002 capsule policy v1."""

from __future__ import annotations

import copy
import importlib.util
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VALIDATOR_PATH = ROOT / "validate_qual_002a.py"
PROFILE_PATH = ROOT / "qual-002a-profile-v1.json"
SCHEMA_PATH = ROOT / "QUAL_002_CAPSULE_POLICY_V1.schema.json"

spec = importlib.util.spec_from_file_location("qual002a", VALIDATOR_PATH)
assert spec and spec.loader
qual002a = importlib.util.module_from_spec(spec)
spec.loader.exec_module(qual002a)

EXPECTED_PROFILE_SHA256 = "1988dbcaff6acf7a7037dc7958062a172cb477682f9bbb318fd10f3383b05d34"
EXPECTED_PROFILE_BYTES = 2590


class Qual002APolicyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.valid = json.loads(PROFILE_PATH.read_text(encoding="utf-8"))

    def assert_rejected(self, value: object) -> None:
        with self.assertRaises(qual002a.PolicyError):
            if not isinstance(value, dict):
                qual002a.validate_policy(value)  # type: ignore[arg-type]
            else:
                qual002a.validate_policy(value)

    def test_valid_profile_and_frozen_bytes(self) -> None:
        digest, byte_len = qual002a.validate_file(PROFILE_PATH)
        self.assertEqual(digest, EXPECTED_PROFILE_SHA256)
        self.assertEqual(byte_len, EXPECTED_PROFILE_BYTES)

    def test_schema_declares_closed_root_and_frozen_ids(self) -> None:
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        self.assertFalse(schema["additionalProperties"])
        self.assertEqual(set(schema["required"]), qual002a.EXPECTED_ROOT_KEYS)
        self.assertEqual(
            schema["properties"]["schema_id"]["const"], qual002a.SCHEMA_ID
        )
        self.assertEqual(
            schema["properties"]["profile_id"]["const"], qual002a.PROFILE_ID
        )

    def test_unknown_root_field_rejected(self) -> None:
        candidate = copy.deepcopy(self.valid)
        candidate["command"] = "cargo test"
        self.assert_rejected(candidate)

    def test_unknown_nested_policy_field_rejected(self) -> None:
        candidate = copy.deepcopy(self.valid)
        candidate["nix_policy"]["trust_me"] = True
        self.assert_rejected(candidate)

    def test_unsafe_nix_switches_rejected(self) -> None:
        unsafe = {
            "sandbox": False,
            "sandbox_fallback": True,
            "pure_eval": False,
            "restrict_eval": False,
            "allow_import_from_derivation": True,
            "accept_flake_config": True,
            "allow_unsafe_native_code_during_evaluation": True,
            "require_sigs": False,
            "substitute_theorem_result": True,
            "remote_builders_allowed": True,
            "remote_store_allowed": True,
            "plugin_files_allowed": True,
            "pre_build_hook_allowed": True,
            "post_build_hook_allowed": True,
            "diff_hook_allowed": True,
            "fixed_output_theorem_allowed": True,
            "impure_theorem_allowed": True,
            "no_chroot_theorem_allowed": True,
            "network_theorem_allowed": True,
        }
        for field, bad_value in unsafe.items():
            with self.subTest(field=field):
                candidate = copy.deepcopy(self.valid)
                candidate["nix_policy"][field] = bad_value
                self.assert_rejected(candidate)

    def test_preparation_cannot_execute_subject_code(self) -> None:
        candidate = copy.deepcopy(self.valid)
        candidate["preparation_policy"]["subject_code_execution_allowed"] = True
        self.assert_rejected(candidate)

    def test_remote_fetches_must_be_content_pinned(self) -> None:
        candidate = copy.deepcopy(self.valid)
        candidate["preparation_policy"]["all_remote_inputs_content_pinned"] = False
        self.assert_rejected(candidate)

    def test_environment_base_is_frozen(self) -> None:
        candidate = copy.deepcopy(self.valid)
        candidate["environment_policy"]["base_set"]["TZ"] = "Africa/Johannesburg"
        self.assert_rejected(candidate)

    def test_nix_user_config_is_neutralized(self) -> None:
        candidate = copy.deepcopy(self.valid)
        candidate["environment_policy"]["base_set"]["NIX_USER_CONF_FILES"] = ""
        self.assert_rejected(candidate)

    def test_source_nar_and_effective_nix_config_are_required_evidence(self) -> None:
        for field in (
            "source_nar_hash",
            "effective_nix_config",
            "derivation_json_digest",
        ):
            with self.subTest(field=field):
                candidate = copy.deepcopy(self.valid)
                candidate["evidence_required"].remove(field)
                self.assert_rejected(candidate)

    def test_ambient_clear_order_and_membership_are_frozen(self) -> None:
        candidate = copy.deepcopy(self.valid)
        candidate["environment_policy"]["ambient_clear"] = list(
            reversed(candidate["environment_policy"]["ambient_clear"])
        )
        self.assert_rejected(candidate)

    def test_suite_environment_cannot_add_arbitrary_key(self) -> None:
        candidate = copy.deepcopy(self.valid)
        candidate["environment_policy"]["suite_owned_keys"].append(
            "AWS_SECRET_ACCESS_KEY"
        )
        self.assert_rejected(candidate)

    def test_source_identity_fields_cannot_be_weakened(self) -> None:
        candidate = copy.deepcopy(self.valid)
        candidate["source_identity_required"].remove("subject_tree_sha")
        self.assert_rejected(candidate)

    def test_evidence_fields_cannot_be_weakened(self) -> None:
        candidate = copy.deepcopy(self.valid)
        candidate["evidence_required"].remove("builders_config")
        self.assert_rejected(candidate)

    def test_other_system_rejected(self) -> None:
        candidate = copy.deepcopy(self.valid)
        candidate["supported_systems"] = ["aarch64-darwin"]
        self.assert_rejected(candidate)

    def test_integer_cannot_masquerade_as_boolean(self) -> None:
        candidate = copy.deepcopy(self.valid)
        candidate["nix_policy"]["sandbox"] = 1
        self.assert_rejected(candidate)

    def test_null_is_rejected(self) -> None:
        candidate = copy.deepcopy(self.valid)
        candidate["profile_id"] = None
        self.assert_rejected(candidate)

    def test_duplicate_json_key_rejected(self) -> None:
        raw = (
            '{"schema_id":"luminous.qual-002-capsule-policy.v1",'
            '"schema_id":"luminous.qual-002-capsule-policy.v1"}'
        )
        path = self._temp(raw.encode("utf-8"))
        with self.assertRaises(qual002a.PolicyError):
            qual002a.load_json_strict(path)

    def test_utf8_bom_rejected(self) -> None:
        path = self._temp(b"\xef\xbb\xbf" + PROFILE_PATH.read_bytes())
        with self.assertRaises(qual002a.PolicyError):
            qual002a.load_json_strict(path)

    def test_nonfinite_json_number_rejected(self) -> None:
        path = self._temp(b'{"x":NaN}')
        with self.assertRaises(qual002a.PolicyError):
            qual002a.load_json_strict(path)

    def test_root_array_rejected(self) -> None:
        path = self._temp(b"[]")
        with self.assertRaises(qual002a.PolicyError):
            qual002a.load_json_strict(path)

    @staticmethod
    def _temp(data: bytes) -> Path:
        import tempfile

        handle = tempfile.NamedTemporaryFile(delete=False)
        handle.write(data)
        handle.flush()
        handle.close()
        return Path(handle.name)


if __name__ == "__main__":
    unittest.main()
