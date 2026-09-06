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
import warnings
import zipfile
from pathlib import Path

SCRIPT = Path(__file__).with_name("verify-evidence-execution-envelope.py")
SPEC = importlib.util.spec_from_file_location("evidence_envelope_verifier", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
VERIFIER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VERIFIER)


def canonical(value: object) -> bytes:
    return (
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
        + "\n"
    ).encode("utf-8")


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def fixture_files() -> dict[str, bytes]:
    identity = {
        "schema": VERIFIER.IDENTITY_SCHEMA,
        "identity": {
            "repository": "luminous-dynamics/symtropy",
            "execution_commit": "a" * 40,
            "execution_tree": "b" * 40,
            "workflow_path": ".github/workflows/evidence.yml",
            "workflow_blob": "c" * 40,
        },
        "binding_contract": {
            "required": [
                "environment.arch",
                "environment.kernel_release",
                "source.os_release",
                "tool.git",
                "tool.python",
            ]
        },
        "bindings": {
            "environment.arch": "x64",
            "environment.kernel_release": "6.1.0-test",
            "source.os_release": "d" * 64,
            "tool.git": "2.54.0",
            "tool.python": "3.12.14",
        },
    }
    identity_bytes = canonical(identity)
    observation = {
        "schema": VERIFIER.OBSERVATION_SCHEMA,
        "execution_identity_sha256": sha(identity_bytes),
        "github": {
            "repository": "Luminous-Dynamics/symtropy",
            "run_id": "123",
            "run_attempt": "1",
            "event_name": "pull_request",
            "workflow_ref": "Luminous-Dynamics/symtropy/.github/workflows/evidence.yml@refs/pull/1/merge",
            "workflow_sha": "e" * 40,
        },
        "runner": {
            "name": "GitHub Actions 1",
            "os": "Linux",
            "arch": "X64",
            "environment": "github-hosted",
            "image_os": "",
            "image_version": "",
        },
        "host": {
            "kernel_release": "6.1.0-test",
            "os_release_sha256": "d" * 64,
        },
        "tools": {
            "git": "git version 2.54.0",
            "python": "3.12.14",
        },
        "transport": {
            "upload_artifact_commit": "f" * 40,
        },
    }
    observation_bytes = canonical(observation)
    return {
        "EXECUTION_IDENTITY.json": identity_bytes,
        "EXECUTION_IDENTITY.sha256": (
            f"{sha(identity_bytes)}  EXECUTION_IDENTITY.json\n".encode("ascii")
        ),
        "OBSERVATION.json": observation_bytes,
        "OBSERVATION.sha256": (
            f"{sha(observation_bytes)}  OBSERVATION.json\n".encode("ascii")
        ),
    }


def write_dir(root: Path, files: dict[str, bytes] | None = None) -> Path:
    evidence = root / "evidence"
    evidence.mkdir()
    for name, data in (files or fixture_files()).items():
        (evidence / name).write_bytes(data)
    return evidence


def write_zip(
    root: Path,
    files: dict[str, bytes] | None = None,
    *,
    extra_members: list[tuple[str, bytes]] | None = None,
) -> Path:
    path = root / "evidence.zip"
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_STORED) as archive:
        for name, data in (files or fixture_files()).items():
            archive.writestr(name, data)
        for name, data in extra_members or []:
            archive.writestr(name, data)
    return path


def args(path: Path, **overrides: object) -> argparse.Namespace:
    values = {
        "evidence": path,
        "expected_archive_sha256": None,
        "expected_identity_sha256": None,
        "expected_observation_sha256": None,
        "expected_repository": None,
        "expected_commit": None,
        "expected_tree": None,
        "expected_workflow_blob": None,
        "expected_workflow_path": None,
        "expected_run_id": None,
        "expected_run_attempt": None,
        "expected_event": None,
        "expected_upload_artifact_commit": None,
    }
    values.update(overrides)
    return argparse.Namespace(**values)


class EvidenceEnvelopeVerifierTests(unittest.TestCase):
    def test_valid_directory(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            result = VERIFIER.verify_envelope(args(write_dir(Path(temporary))))
            self.assertEqual(result["archive_sha256"], "")

    def test_valid_zip_with_all_expectations(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = write_zip(Path(temporary))
            files = fixture_files()
            result = VERIFIER.verify_envelope(
                args(
                    path,
                    expected_archive_sha256=sha(path.read_bytes()),
                    expected_identity_sha256=sha(files["EXECUTION_IDENTITY.json"]),
                    expected_observation_sha256=sha(files["OBSERVATION.json"]),
                    expected_repository="luminous-dynamics/symtropy",
                    expected_commit="a" * 40,
                    expected_tree="b" * 40,
                    expected_workflow_blob="c" * 40,
                    expected_workflow_path=".github/workflows/evidence.yml",
                    expected_run_id="123",
                    expected_run_attempt="1",
                    expected_event="pull_request",
                    expected_upload_artifact_commit="f" * 40,
                )
            )
            self.assertEqual(result["archive_sha256"], sha(path.read_bytes()))

    def test_corrupted_identity_fails_checksum(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            files = fixture_files()
            files["EXECUTION_IDENTITY.json"] += b" "
            with self.assertRaisesRegex(VERIFIER.VerificationError, "identity checksum"):
                VERIFIER.verify_envelope(args(write_dir(Path(temporary), files)))

    def test_observation_identity_cross_binding_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            files = fixture_files()
            observation = json.loads(files["OBSERVATION.json"])
            observation["execution_identity_sha256"] = "0" * 64
            files["OBSERVATION.json"] = canonical(observation)
            files["OBSERVATION.sha256"] = (
                f"{sha(files['OBSERVATION.json'])}  OBSERVATION.json\n".encode("ascii")
            )
            with self.assertRaisesRegex(
                VERIFIER.VerificationError, "observation identity digest"
            ):
                VERIFIER.verify_envelope(args(write_dir(Path(temporary), files)))

    def test_noncanonical_json_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            files = fixture_files()
            identity = json.loads(files["EXECUTION_IDENTITY.json"])
            files["EXECUTION_IDENTITY.json"] = (
                json.dumps(identity, indent=2, sort_keys=True) + "\n"
            ).encode("utf-8")
            files["EXECUTION_IDENTITY.sha256"] = (
                f"{sha(files['EXECUTION_IDENTITY.json'])}  EXECUTION_IDENTITY.json\n".encode(
                    "ascii"
                )
            )
            with self.assertRaisesRegex(VERIFIER.VerificationError, "not canonical JSON"):
                VERIFIER.verify_envelope(args(write_dir(Path(temporary), files)))

    def test_missing_required_binding_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            files = fixture_files()
            identity = json.loads(files["EXECUTION_IDENTITY.json"])
            del identity["bindings"]["tool.git"]
            files["EXECUTION_IDENTITY.json"] = canonical(identity)
            files["EXECUTION_IDENTITY.sha256"] = (
                f"{sha(files['EXECUTION_IDENTITY.json'])}  EXECUTION_IDENTITY.json\n".encode(
                    "ascii"
                )
            )
            observation = json.loads(files["OBSERVATION.json"])
            observation["execution_identity_sha256"] = sha(files["EXECUTION_IDENTITY.json"])
            files["OBSERVATION.json"] = canonical(observation)
            files["OBSERVATION.sha256"] = (
                f"{sha(files['OBSERVATION.json'])}  OBSERVATION.json\n".encode("ascii")
            )
            with self.assertRaisesRegex(VERIFIER.VerificationError, "required bindings missing"):
                VERIFIER.verify_envelope(args(write_dir(Path(temporary), files)))

    def test_cross_bound_git_version_mismatch_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            files = fixture_files()
            observation = json.loads(files["OBSERVATION.json"])
            observation["tools"]["git"] = "git version 2.55.0"
            files["OBSERVATION.json"] = canonical(observation)
            files["OBSERVATION.sha256"] = (
                f"{sha(files['OBSERVATION.json'])}  OBSERVATION.json\n".encode("ascii")
            )
            with self.assertRaisesRegex(VERIFIER.VerificationError, "binding cross-check tool.git"):
                VERIFIER.verify_envelope(args(write_dir(Path(temporary), files)))

    def test_extra_directory_member_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            evidence = write_dir(Path(temporary))
            (evidence / "extra.txt").write_text("no", encoding="utf-8")
            with self.assertRaisesRegex(VERIFIER.VerificationError, "exactly"):
                VERIFIER.verify_envelope(args(evidence))

    def test_nested_zip_member_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = write_zip(
                Path(temporary), extra_members=[("nested/extra.txt", b"no")]
            )
            with self.assertRaisesRegex(VERIFIER.VerificationError, "exactly"):
                VERIFIER.verify_envelope(args(path))

    def test_duplicate_zip_member_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = root / "evidence.zip"
            files = fixture_files()
            with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_STORED) as archive:
                for name, data in files.items():
                    archive.writestr(name, data)
                with warnings.catch_warnings():
                    warnings.simplefilter("ignore", UserWarning)
                    archive.writestr("OBSERVATION.json", files["OBSERVATION.json"])
            with self.assertRaisesRegex(VERIFIER.VerificationError, "duplicate member"):
                VERIFIER.verify_envelope(args(path))

    def test_expected_commit_mismatch_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaisesRegex(VERIFIER.VerificationError, "expected commit"):
                VERIFIER.verify_envelope(
                    args(write_dir(Path(temporary)), expected_commit="9" * 40)
                )

    def test_cli_verifies_zip_with_expected_provenance(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = write_zip(Path(temporary))
            files = fixture_files()
            command = [
                sys.executable,
                str(SCRIPT),
                "--evidence",
                str(path),
                "--expected-archive-sha256",
                sha(path.read_bytes()),
                "--expected-identity-sha256",
                sha(files["EXECUTION_IDENTITY.json"]),
                "--expected-observation-sha256",
                sha(files["OBSERVATION.json"]),
                "--expected-repository",
                "Luminous-Dynamics/symtropy",
                "--expected-commit",
                "a" * 40,
                "--expected-tree",
                "b" * 40,
                "--expected-workflow-blob",
                "c" * 40,
                "--expected-workflow-path",
                ".github/workflows/evidence.yml",
                "--expected-run-id",
                "123",
                "--expected-run-attempt",
                "1",
                "--expected-event",
                "pull_request",
                "--expected-upload-artifact-commit",
                "f" * 40,
            ]
            result = subprocess.run(command, text=True, capture_output=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("PASS identity_sha256=", result.stdout)
            self.assertIn("PASS observation_sha256=", result.stdout)
            self.assertIn("PASS archive_sha256=", result.stdout)

    def test_archive_digest_requires_zip(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaisesRegex(VERIFIER.VerificationError, "requires a ZIP"):
                VERIFIER.verify_envelope(
                    args(
                        write_dir(Path(temporary)),
                        expected_archive_sha256="0" * 64,
                    )
                )


if __name__ == "__main__":
    unittest.main()
