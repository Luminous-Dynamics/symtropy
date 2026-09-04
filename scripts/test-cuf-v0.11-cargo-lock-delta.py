#!/usr/bin/env python3
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later

from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path


ANALYZER = Path(__file__).with_name("analyze-cuf-v0.11-cargo-lock-delta.py")
spec = importlib.util.spec_from_file_location("cuf_lock_delta", ANALYZER)
if spec is None or spec.loader is None:
    raise RuntimeError("unable to load Cargo.lock semantic delta analyzer")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


BEFORE = '''\
version = 4

[[package]]
name = "app"
version = "0.1.0"
dependencies = [
 "dep 1.0.0 (registry+https://example.invalid/index)",
 "same 1.0.0 (registry+https://example.invalid/index)",
]

[[package]]
name = "dep"
version = "1.0.0"
source = "registry+https://example.invalid/index"
checksum = "aaa"

[[package]]
name = "same"
version = "1.0.0"
source = "registry+https://example.invalid/index"
checksum = "old-checksum"
'''

AFTER = '''\
version = 4

[[package]]
name = "app"
version = "0.1.0"
dependencies = [
 "dep 2.0.0 (registry+https://example.invalid/index)",
 "new-local",
 "same 1.0.0 (registry+https://example.invalid/index)",
]

[[package]]
name = "dep"
version = "2.0.0"
source = "registry+https://example.invalid/index"
checksum = "bbb"
resolver_note = "preserve-me"

[[package]]
name = "new-local"
version = "0.1.0"

[[package]]
name = "same"
version = "1.0.0"
source = "registry+https://example.invalid/index"
checksum = "new-checksum"
'''

STRUCTURAL_BEFORE = '''\
version = 4

[metadata]
opaque = "before"

[[package]]
name = "stable"
version = "1.0.0"
opaque_field = "before"
'''

STRUCTURAL_AFTER = '''\
version = 4

[metadata]
opaque = "after"
new_value = 7

[[package]]
name = "stable"
version = "1.0.0"
opaque_field = "after"
opaque_list = ["a", "b"]
'''


class CargoLockSemanticDeltaTests(unittest.TestCase):
    def write_lock(self, root: Path, name: str, content: str) -> Path:
        path = root / name
        path.write_text(content, encoding="utf-8")
        return path

    def test_multiversion_local_dependency_and_checksum_delta(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            before = module.load_lock(self.write_lock(root, "before.lock", BEFORE))
            after = module.load_lock(self.write_lock(root, "after.lock", AFTER))
            result = module.analyze(before, after)

        self.assertEqual(result["schema"], "symtropy.cuf.cargo-lock-semantic-delta.v2")
        self.assertEqual(result["package_count_before"], 3)
        self.assertEqual(result["package_count_after"], 4)
        self.assertEqual(result["summary"]["lockfile_version_changed"], 0)
        self.assertEqual(result["summary"]["added_packages"], 2)
        self.assertEqual(result["summary"]["removed_packages"], 1)
        self.assertEqual(result["summary"]["checksum_changes"], 1)
        self.assertEqual(result["summary"]["dependency_changes"], 1)
        self.assertEqual(result["summary"]["unmodeled_top_level_changes"], 0)
        self.assertEqual(result["summary"]["unmodeled_package_field_changes"], 0)

        added = {
            (item["name"], item["version"], item["source"]): item
            for item in result["added_packages"]
        }
        self.assertIn(("dep", "2.0.0", "registry+https://example.invalid/index"), added)
        self.assertIn(("new-local", "0.1.0", "<source-omitted>"), added)
        self.assertEqual(
            added[("dep", "2.0.0", "registry+https://example.invalid/index")]["extra_fields"],
            {"resolver_note": "preserve-me"},
        )

        removed = {
            (item["name"], item["version"], item["source"])
            for item in result["removed_packages"]
        }
        self.assertEqual(
            removed,
            {("dep", "1.0.0", "registry+https://example.invalid/index")},
        )

        identity_changes = {
            item["name"]: item for item in result["name_identity_sets_changed"]
        }
        self.assertIn("dep", identity_changes)
        self.assertIn("new-local", identity_changes)

        checksum_change = result["checksum_changes"][0]
        self.assertEqual(checksum_change["name"], "same")
        self.assertEqual(checksum_change["before"], "old-checksum")
        self.assertEqual(checksum_change["after"], "new-checksum")

        dependency_change = result["dependency_changes"][0]
        self.assertEqual(dependency_change["name"], "app")
        self.assertEqual(
            dependency_change["added"],
            [
                "dep 2.0.0 (registry+https://example.invalid/index)",
                "new-local",
            ],
        )
        self.assertEqual(
            dependency_change["removed"],
            ["dep 1.0.0 (registry+https://example.invalid/index)"],
        )

        text = module.emit_text(result)
        self.assertIn(
            'ADD\tdep\t2.0.0\tregistry+https://example.invalid/index'
            '\tchecksum="bbb"\tdependencies=[]'
            '\textra_fields={"resolver_note":"preserve-me"}',
            text,
        )
        self.assertIn(
            'ADD\tnew-local\t0.1.0\t<source-omitted>'
            '\tchecksum=null\tdependencies=[]\textra_fields={}',
            text,
        )
        self.assertIn(
            'REMOVE\tdep\t1.0.0\tregistry+https://example.invalid/index'
            '\tchecksum="aaa"\tdependencies=[]\textra_fields={}',
            text,
        )
        self.assertIn(
            'CHECKSUM\tsame\t1.0.0\tregistry+https://example.invalid/index'
            '\tbefore="old-checksum"\tafter="new-checksum"',
            text,
        )

    def test_source_omission_is_not_claimed_as_workspace_membership(self) -> None:
        package = {"name": "local", "version": "0.1.0"}
        self.assertEqual(module.normalized_source(package), "<source-omitted>")

    def test_unmodeled_structure_is_never_silent(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            before = module.load_lock(
                self.write_lock(root, "structural-before.lock", STRUCTURAL_BEFORE)
            )
            after = module.load_lock(
                self.write_lock(root, "structural-after.lock", STRUCTURAL_AFTER)
            )
            result = module.analyze(before, after)

        self.assertEqual(result["summary"]["unmodeled_top_level_changes"], 1)
        self.assertEqual(result["summary"]["unmodeled_package_field_changes"], 1)

        top_change = result["unmodeled_top_level_changes"][0]
        self.assertEqual(top_change["field"], "metadata")
        self.assertEqual(
            top_change["before"],
            {"present": True, "value": {"opaque": "before"}},
        )
        self.assertEqual(
            top_change["after"],
            {"present": True, "value": {"new_value": 7, "opaque": "after"}},
        )

        package_change = result["unmodeled_package_field_changes"][0]
        self.assertEqual(package_change["name"], "stable")
        self.assertEqual(package_change["before"], {"opaque_field": "before"})
        self.assertEqual(
            package_change["after"],
            {"opaque_field": "after", "opaque_list": ["a", "b"]},
        )

        text = module.emit_text(result)
        self.assertIn("TOP_LEVEL\tmetadata\t", text)
        self.assertIn("PACKAGE_FIELDS\tstable\t1.0.0\t<source-omitted>\t", text)

    def test_top_level_presence_is_unambiguous(self) -> None:
        before_text = '''\
version = 4
[[package]]
name = "stable"
version = "1.0.0"
'''
        after_text = '''\
version = 4
[opaque]
__missing__ = true
[[package]]
name = "stable"
version = "1.0.0"
'''
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            before = module.load_lock(self.write_lock(root, "presence-before.lock", before_text))
            after = module.load_lock(self.write_lock(root, "presence-after.lock", after_text))
            result = module.analyze(before, after)

        change = result["unmodeled_top_level_changes"][0]
        self.assertEqual(change["before"], {"present": False})
        self.assertEqual(
            change["after"],
            {"present": True, "value": {"__missing__": True}},
        )

    def test_lockfile_version_change_is_counted(self) -> None:
        before_text = 'version = 3\n'
        after_text = 'version = 4\n'
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            before = module.load_lock(self.write_lock(root, "v3.lock", before_text))
            after = module.load_lock(self.write_lock(root, "v4.lock", after_text))
            result = module.analyze(before, after)

        self.assertEqual(result["summary"]["lockfile_version_changed"], 1)
        self.assertIn("lockfile_version_changed=1", module.emit_text(result))

    def test_invalid_modeled_lock_versions_are_rejected(self) -> None:
        cases = {
            "string": 'version = "4"\n',
            "boolean": 'version = true\n',
            "nan": 'version = nan\n',
            "zero": 'version = 0\n',
            "negative": 'version = -1\n',
        }
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for name, content in cases.items():
                with self.subTest(name=name):
                    path = self.write_lock(root, f"invalid-{name}.lock", content)
                    with self.assertRaisesRegex(ValueError, "version must be a positive integer"):
                        module.load_lock(path)

    def test_unmodeled_special_toml_values_are_json_safe(self) -> None:
        before_text = 'version = 4\n'
        after_text = '''\
version = 4
[opaque]
when = 1979-05-27T07:32:00Z
not_a_number = nan
positive_infinity = inf
negative_infinity = -inf
'''
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            before = module.load_lock(self.write_lock(root, "special-before.lock", before_text))
            after = module.load_lock(self.write_lock(root, "special-after.lock", after_text))
            result = module.analyze(before, after)

        change = result["unmodeled_top_level_changes"][0]
        value = change["after"]["value"]
        self.assertEqual(
            value["when"],
            {"__toml_type__": "datetime", "value": "1979-05-27T07:32:00+00:00"},
        )
        self.assertEqual(value["not_a_number"], {"__toml_type__": "float", "value": "nan"})
        self.assertEqual(value["positive_infinity"], {"__toml_type__": "float", "value": "inf"})
        self.assertEqual(value["negative_infinity"], {"__toml_type__": "float", "value": "-inf"})
        module.compact_json(result)

    def test_analysis_is_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            before = module.load_lock(self.write_lock(root, "before.lock", BEFORE))
            after = module.load_lock(self.write_lock(root, "after.lock", AFTER))
            first = module.analyze(before, after)
            second = module.analyze(before, after)
        self.assertEqual(first, second)
        self.assertEqual(module.emit_text(first), module.emit_text(second))

    def test_duplicate_exact_package_identity_is_rejected(self) -> None:
        duplicate = '''\
version = 4

[[package]]
name = "dup"
version = "1.0.0"

[[package]]
name = "dup"
version = "1.0.0"
'''
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            data = module.load_lock(self.write_lock(root, "duplicate.lock", duplicate))
            with self.assertRaisesRegex(ValueError, "duplicate package identity"):
                module.package_map(data)


if __name__ == "__main__":
    unittest.main()
