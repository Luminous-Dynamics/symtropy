#!/usr/bin/env python3
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("build-evidence-execution-identity.py")
SPEC = importlib.util.spec_from_file_location("evidence_execution_identity", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
IDENTITY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(IDENTITY)

BASE_BINDINGS = {
    "action.checkout": "d" * 40,
    "environment.arch": "x64",
    "environment.isolation": "vm",
    "environment.runner_label": "ubuntu-24.04",
    "input.context": "parent",
    "policy.review": "9" * 64,
    "source.cargo_lock": "7" * 64,
    "tool.rust": "1.96.0",
    "producer.stage_a": "e" * 40,
    "verifier.stage_a": "f" * 40,
}
BASE_REQUIRED = list(BASE_BINDINGS)


def binding_args(values: dict[str, str]) -> list[str]:
    return [f"{key}={value}" for key, value in values.items()]


def make_args(
    *,
    repository: str = "Luminous-Dynamics/Symtropy",
    commit: str = "a" * 40,
    tree: str = "b" * 40,
    workflow_path: str = ".github/workflows/evidence.yml",
    workflow_blob: str = "c" * 40,
    bindings: list[str] | None = None,
    required_bindings: list[str] | None = None,
    assert_checkout: bool = False,
    repository_root: Path = Path("."),
) -> argparse.Namespace:
    return argparse.Namespace(
        repository=repository,
        execution_commit=commit,
        execution_tree=tree,
        workflow_path=workflow_path,
        workflow_blob=workflow_blob,
        binding=bindings or [],
        require_binding=required_bindings or [],
        assert_checkout=assert_checkout,
        repository_root=repository_root,
    )


def build(**kwargs: object) -> tuple[bytes, str]:
    return IDENTITY.build_identity(make_args(**kwargs))


def git(root: Path, *arguments: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(root), *arguments],
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        raise AssertionError(result.stderr or result.stdout)
    return result.stdout.strip()


def create_git_fixture(root: Path) -> tuple[str, str, str]:
    root.mkdir(parents=True)
    subprocess.run(["git", "init", "--quiet", str(root)], check=True)
    git(root, "config", "user.name", "Execution Identity Test")
    git(root, "config", "user.email", "identity-test@example.invalid")

    workflow = root / ".github/workflows/evidence.yml"
    workflow.parent.mkdir(parents=True)
    workflow.write_text("name: fixture\non: workflow_dispatch\n", encoding="utf-8")
    (root / "README.md").write_text("fixture\n", encoding="utf-8")
    (root / ".gitignore").write_text("ignored.tmp\n", encoding="utf-8")
    git(root, "add", ".")
    git(root, "commit", "--quiet", "-m", "fixture")

    head = git(root, "rev-parse", "HEAD")
    tree = git(root, "rev-parse", "HEAD^{tree}")
    blob = git(root, "rev-parse", "HEAD:.github/workflows/evidence.yml")
    return head, tree, blob


class ExecutionIdentityTests(unittest.TestCase):
    def test_binding_and_requirement_order_do_not_change_identity(self) -> None:
        first = build(
            bindings=binding_args(BASE_BINDINGS),
            required_bindings=BASE_REQUIRED,
        )
        second = build(
            bindings=binding_args(dict(reversed(list(BASE_BINDINGS.items())))),
            required_bindings=list(reversed(BASE_REQUIRED)),
        )
        self.assertEqual(first, second)

    def test_repository_identity_is_case_normalized(self) -> None:
        upper = build(repository="Luminous-Dynamics/Symtropy")
        lower = build(repository="luminous-dynamics/symtropy")
        self.assertEqual(upper, lower)
        payload = json.loads(upper[0])
        self.assertEqual(payload["identity"]["repository"], "luminous-dynamics/symtropy")

    def test_authority_bearing_changes_change_identity(self) -> None:
        baseline = build(
            bindings=binding_args(BASE_BINDINGS),
            required_bindings=BASE_REQUIRED,
        )[1]
        variants: list[dict[str, object]] = [
            {
                "commit": "1" * 40,
                "bindings": binding_args(BASE_BINDINGS),
                "required_bindings": BASE_REQUIRED,
            },
            {
                "tree": "2" * 40,
                "bindings": binding_args(BASE_BINDINGS),
                "required_bindings": BASE_REQUIRED,
            },
            {
                "workflow_blob": "3" * 40,
                "bindings": binding_args(BASE_BINDINGS),
                "required_bindings": BASE_REQUIRED,
            },
        ]
        mutations = {
            "environment.arch": "arm64",
            "environment.isolation": "unprivileged-container",
            "environment.runner_label": "ubuntu-slim",
            "input.context": "v4.8",
            "tool.rust": "1.96.1",
            "policy.review": "a" * 64,
            "source.cargo_lock": "8" * 64,
            "action.checkout": "4" * 40,
            "producer.stage_a": "5" * 40,
            "verifier.stage_a": "6" * 40,
        }
        for key, value in mutations.items():
            changed = dict(BASE_BINDINGS)
            changed[key] = value
            variants.append(
                {
                    "bindings": binding_args(changed),
                    "required_bindings": BASE_REQUIRED,
                }
            )

        for variant in variants:
            with self.subTest(variant=variant):
                self.assertNotEqual(build(**variant)[1], baseline)

    def test_environment_contract_is_semantic_not_observation_metadata(self) -> None:
        vm = build(
            bindings=[
                "environment.runner_label=ubuntu-24.04",
                "environment.isolation=vm",
            ],
            required_bindings=[
                "environment.runner_label",
                "environment.isolation",
            ],
        )
        slim = build(
            bindings=[
                "environment.runner_label=ubuntu-slim",
                "environment.isolation=unprivileged-container",
            ],
            required_bindings=[
                "environment.runner_label",
                "environment.isolation",
            ],
        )
        self.assertNotEqual(vm[1], slim[1])
        payload = json.loads(vm[0])
        self.assertEqual(payload["bindings"]["environment.runner_label"], "ubuntu-24.04")

    def test_required_binding_contract_is_identity_bound(self) -> None:
        bindings = binding_args(BASE_BINDINGS)
        none_required = build(bindings=bindings)
        one_required = build(
            bindings=bindings,
            required_bindings=["input.context"],
        )
        all_required = build(
            bindings=bindings,
            required_bindings=BASE_REQUIRED,
        )
        self.assertNotEqual(none_required[1], one_required[1])
        self.assertNotEqual(one_required[1], all_required[1])
        payload = json.loads(all_required[0])
        self.assertEqual(
            payload["binding_contract"]["required"],
            sorted(BASE_REQUIRED),
        )

    def test_missing_required_binding_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "required binding is missing"):
            build(
                bindings=["input.context=parent"],
                required_bindings=["input.context", "tool.rust"],
            )

    def test_duplicate_required_binding_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "duplicate required binding key"):
            build(
                bindings=["tool.rust=1.96.0"],
                required_bindings=["tool.rust", "tool.rust"],
            )

    def test_invalid_required_binding_namespace_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "authority-bearing namespace"):
            build(
                bindings=["tool.rust=1.96.0"],
                required_bindings=["observation.run_id"],
            )

    def test_observation_metadata_namespace_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "authority-bearing namespace"):
            build(bindings=["observation.run_id=123"])

    def test_floating_or_descriptive_authority_refs_are_rejected(self) -> None:
        for binding in (
            "action.checkout=v4",
            "policy.review=current",
            "source.cargo_lock=current",
            "producer.stage_a=HEAD",
            "verifier.stage_a=main",
        ):
            with self.subTest(binding=binding):
                with self.assertRaisesRegex(ValueError, "object id"):
                    build(bindings=[binding])

    def test_free_form_semantic_values_are_review_safe(self) -> None:
        for binding in (
            "tool.rust= 1.96.0",
            "tool.rust=1.96.0 ",
            "tool.rust=1.96.0\t",
            "environment.runner_label=ubuntu-24.04\u200b",
        ):
            with self.subTest(binding=binding):
                with self.assertRaisesRegex(
                    ValueError, "printable ASCII|leading or trailing whitespace"
                ):
                    build(bindings=[binding])

    def test_obvious_moving_semantic_aliases_are_rejected(self) -> None:
        for binding in (
            "tool.rust=stable",
            "tool.channel=nightly",
            "tool.ref=HEAD",
            "environment.runner_label=ubuntu-latest",
            "environment.image=current",
        ):
            with self.subTest(binding=binding):
                with self.assertRaisesRegex(ValueError, "moving alias"):
                    build(bindings=[binding])

    def test_pinned_semantic_labels_remain_available(self) -> None:
        encoded, _ = build(
            bindings=[
                "tool.channel=nightly-2026-09-06",
                "environment.runner_label=ubuntu-24.04",
                "input.context=current",
                "input.description=retained replay",
            ]
        )
        payload = json.loads(encoded)
        self.assertEqual(payload["bindings"]["tool.channel"], "nightly-2026-09-06")
        self.assertEqual(payload["bindings"]["input.context"], "current")
        self.assertEqual(payload["bindings"]["input.description"], "retained replay")

    def test_live_checkout_assertion_binds_commit_tree_workflow_and_workspace(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "repo"
            head, tree, blob = create_git_fixture(root)
            encoded, _ = build(
                commit=head,
                tree=tree,
                workflow_blob=blob,
                assert_checkout=True,
                repository_root=root,
            )
            payload = json.loads(encoded)
            self.assertEqual(payload["identity"]["execution_commit"], head)
            self.assertEqual(payload["identity"]["execution_tree"], tree)
            self.assertEqual(payload["identity"]["workflow_blob"], blob)

            with self.assertRaisesRegex(ValueError, "execution_commit does not match"):
                build(
                    commit="1" * 40,
                    tree=tree,
                    workflow_blob=blob,
                    assert_checkout=True,
                    repository_root=root,
                )
            with self.assertRaisesRegex(ValueError, "execution_tree does not match"):
                build(
                    commit=head,
                    tree="2" * 40,
                    workflow_blob=blob,
                    assert_checkout=True,
                    repository_root=root,
                )
            with self.assertRaisesRegex(ValueError, "workflow_blob does not match"):
                build(
                    commit=head,
                    tree=tree,
                    workflow_blob="3" * 40,
                    assert_checkout=True,
                    repository_root=root,
                )

            untracked = root / "UNTRACKED.txt"
            untracked.write_text("dirty\n", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "tracked HEAD material"):
                build(
                    commit=head,
                    tree=tree,
                    workflow_blob=blob,
                    assert_checkout=True,
                    repository_root=root,
                )
            untracked.unlink()

            ignored = root / "ignored.tmp"
            ignored.write_text("ignored but executable material\n", encoding="utf-8")
            self.assertEqual(git(root, "status", "--porcelain=v1", "--untracked-files=all"), "")
            with self.assertRaisesRegex(ValueError, "ignored/untracked files are forbidden"):
                build(
                    commit=head,
                    tree=tree,
                    workflow_blob=blob,
                    assert_checkout=True,
                    repository_root=root,
                )

    def test_assert_checkout_cli_requires_output_outside_checkout(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "repo"
            head, tree, blob = create_git_fixture(root)
            command = [
                sys.executable,
                str(SCRIPT),
                "--repository",
                "Luminous-Dynamics/Symtropy",
                "--execution-commit",
                head,
                "--execution-tree",
                tree,
                "--workflow-path",
                ".github/workflows/evidence.yml",
                "--workflow-blob",
                blob,
                "--assert-checkout",
                "--repository-root",
                str(root),
                "--output-dir",
                str(root / "evidence"),
            ]
            result = subprocess.run(command, text=True, capture_output=True, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("output_dir must be outside", result.stderr)
            self.assertFalse((root / "evidence").exists())

    def test_assert_checkout_cli_writes_outside_and_preserves_clean_tree(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "repo"
            output_dir = Path(temporary) / "evidence"
            head, tree, blob = create_git_fixture(root)
            command = [
                sys.executable,
                str(SCRIPT),
                "--repository",
                "Luminous-Dynamics/Symtropy",
                "--execution-commit",
                head,
                "--execution-tree",
                tree,
                "--workflow-path",
                ".github/workflows/evidence.yml",
                "--workflow-blob",
                blob,
                "--binding",
                "environment.runner_label=ubuntu-slim",
                "--require-binding",
                "environment.runner_label",
                "--assert-checkout",
                "--repository-root",
                str(root),
                "--output-dir",
                str(output_dir),
            ]
            result = subprocess.run(command, text=True, capture_output=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(git(root, "status", "--porcelain=v1", "--untracked-files=all"), "")
            self.assertEqual(git(root, "clean", "-ndx"), "")
            encoded = (output_dir / "EXECUTION_IDENTITY.json").read_bytes()
            digest = hashlib.sha256(encoded).hexdigest()
            self.assertEqual(result.stdout.strip(), digest)

    def test_duplicate_binding_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "duplicate binding key"):
            build(bindings=["tool.rust=1.96.0", "tool.rust=1.96.0"])

    def test_invalid_object_identity_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "object id"):
            build(commit="not-a-sha")

    def test_non_relative_workflow_path_is_rejected(self) -> None:
        for value in (
            "/tmp/evidence.yml",
            "../evidence.yml",
            ".github/../evidence.yml",
            ".github//evidence.yml",
            "./.github/evidence.yml",
        ):
            with self.subTest(value=value):
                with self.assertRaisesRegex(ValueError, "workflow_path"):
                    build(workflow_path=value)

    def test_sha256_length_git_object_ids_are_supported(self) -> None:
        encoded, digest = build(
            commit="a" * 64,
            tree="b" * 64,
            workflow_blob="c" * 64,
        )
        self.assertEqual(len(digest), 64)
        self.assertEqual(hashlib.sha256(encoded).hexdigest(), digest)

    def test_cli_writes_self_consistent_identity_and_digest(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output_dir = Path(temporary) / "out"
            command = [
                sys.executable,
                str(SCRIPT),
                "--repository",
                "Luminous-Dynamics/Symtropy",
                "--execution-commit",
                "a" * 40,
                "--execution-tree",
                "b" * 40,
                "--workflow-path",
                ".github/workflows/evidence.yml",
                "--workflow-blob",
                "c" * 40,
                "--binding",
                "environment.runner_label=ubuntu-24.04",
                "--binding",
                "input.context=parent",
                "--binding",
                "tool.rust=1.96.0",
                "--require-binding",
                "environment.runner_label",
                "--require-binding",
                "input.context",
                "--require-binding",
                "tool.rust",
                "--output-dir",
                str(output_dir),
            ]
            result = subprocess.run(command, text=True, capture_output=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)

            encoded = (output_dir / "EXECUTION_IDENTITY.json").read_bytes()
            digest = hashlib.sha256(encoded).hexdigest()
            self.assertEqual(result.stdout.strip(), digest)
            self.assertEqual(
                (output_dir / "EXECUTION_IDENTITY.sha256").read_text(encoding="ascii"),
                f"{digest}  EXECUTION_IDENTITY.json\n",
            )
            payload = json.loads(encoded)
            self.assertEqual(
                payload["binding_contract"]["required"],
                ["environment.runner_label", "input.context", "tool.rust"],
            )


if __name__ == "__main__":
    unittest.main()
