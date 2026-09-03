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
 "new-workspace",
 "same 1.0.0 (registry+https://example.invalid/index)",
]

[[package]]
name = "dep"
version = "2.0.0"
source = "registry+https://example.invalid/index"
checksum = "bbb"

[[package]]
name = "new-workspace"
version = "0.1.0"

[[package]]
name = "same"
version = "1.0.0"
source = "registry+https://example.invalid/index"
checksum = "new-checksum"
'''


class CargoLockSemanticDeltaTests(unittest.TestCase):
    def write_lock(self, root: Path, name: str, content: str) -> Path:
        path = root / name
        path.write_text(content, encoding="utf-8")
        return path

    def test_multiversion_workspace_dependency_and_checksum_delta(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            before = module.load_lock(self.write_lock(root, "before.lock", BEFORE))
            after = module.load_lock(self.write_lock(root, "after.lock", AFTER))
            result = module.analyze(before, after)

        self.assertEqual(result["schema"], "symtropy.cuf.cargo-lock-semantic-delta.v1")
        self.assertEqual(result["package_count_before"], 3)
        self.assertEqual(result["package_count_after"], 4)
        self.assertEqual(result["summary"]["added_packages"], 2)
        self.assertEqual(result["summary"]["removed_packages"], 1)
        self.assertEqual(result["summary"]["checksum_changes"], 1)
        self.assertEqual(result["summary"]["dependency_changes"], 1)

        added = {(item["name"], item["version"], item["source"]) for item in result["added_packages"]}
        self.assertIn(("dep", "2.0.0", "registry+https://example.invalid/index"), added)
        self.assertIn(("new-workspace", "0.1.0", "<workspace>"), added)

        removed = {(item["name"], item["version"], item["source"]) for item in result["removed_packages"]}
        self.assertEqual(
            removed,
            {("dep", "1.0.0", "registry+https://example.invalid/index")},
        )

        identity_changes = {item["name"]: item for item in result["name_identity_sets_changed"]}
        self.assertIn("dep", identity_changes)
        self.assertIn("new-workspace", identity_changes)

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
                "new-workspace",
            ],
        )
        self.assertEqual(
            dependency_change["removed"],
            ["dep 1.0.0 (registry+https://example.invalid/index)"],
        )

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
