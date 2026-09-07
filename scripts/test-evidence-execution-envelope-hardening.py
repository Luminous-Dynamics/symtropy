#!/usr/bin/env python3
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later

from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent


def _load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


HARDENED = _load(
    "evidence_envelope_hardened",
    HERE / "verify-evidence-execution-envelope-hardened.py",
)
BASE_TESTS = _load(
    "evidence_envelope_base_tests",
    HERE / "test-evidence-execution-envelope.py",
)


class EvidenceEnvelopeHardeningTests(unittest.TestCase):
    def test_valid_fixture_passes_hardened_boundary(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = BASE_TESTS.write_zip(Path(temporary))
            result = HARDENED.verify_envelope(BASE_TESTS.args(path))
            self.assertEqual(result["archive_sha256"], BASE_TESTS.sha(path.read_bytes()))

    def test_moving_tool_alias_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            files = BASE_TESTS.fixture_files()
            identity = json.loads(files["EXECUTION_IDENTITY.json"])
            identity["bindings"]["tool.git"] = "latest"
            files["EXECUTION_IDENTITY.json"] = BASE_TESTS.canonical(identity)
            files["EXECUTION_IDENTITY.sha256"] = (
                f"{BASE_TESTS.sha(files['EXECUTION_IDENTITY.json'])}  EXECUTION_IDENTITY.json\n".encode("ascii")
            )
            observation = json.loads(files["OBSERVATION.json"])
            observation["execution_identity_sha256"] = BASE_TESTS.sha(
                files["EXECUTION_IDENTITY.json"]
            )
            observation["tools"]["git"] = "git version latest"
            files["OBSERVATION.json"] = BASE_TESTS.canonical(observation)
            files["OBSERVATION.sha256"] = (
                f"{BASE_TESTS.sha(files['OBSERVATION.json'])}  OBSERVATION.json\n".encode("ascii")
            )
            with self.assertRaisesRegex(HARDENED.VerificationError, "moving alias"):
                HARDENED.verify_envelope(
                    BASE_TESTS.args(BASE_TESTS.write_zip(Path(temporary), files))
                )

    def test_oversized_zip_member_fails_before_full_decode(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            files = BASE_TESTS.fixture_files()
            files["OBSERVATION.json"] = b"x" * (HARDENED.INGEST.MAX_MEMBER_BYTES + 1)
            path = BASE_TESTS.write_zip(Path(temporary), files)
            with self.assertRaisesRegex(HARDENED.VerificationError, "member exceeds"):
                HARDENED.verify_envelope(BASE_TESTS.args(path))

    def test_nonstandard_json_constant_fails_cleanly(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            files = BASE_TESTS.fixture_files()
            identity = files["EXECUTION_IDENTITY.json"].decode("utf-8").replace(
                '"workflow_path":".github/workflows/evidence.yml"',
                '"workflow_path":NaN',
            ).encode("utf-8")
            files["EXECUTION_IDENTITY.json"] = identity
            files["EXECUTION_IDENTITY.sha256"] = (
                f"{BASE_TESTS.sha(identity)}  EXECUTION_IDENTITY.json\n".encode("ascii")
            )
            with self.assertRaisesRegex(HARDENED.VerificationError, "non-standard JSON constant"):
                HARDENED.verify_envelope(
                    BASE_TESTS.args(BASE_TESTS.write_zip(Path(temporary), files))
                )

    def test_nondigit_run_id_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            files = BASE_TESTS.fixture_files()
            observation = json.loads(files["OBSERVATION.json"])
            observation["github"]["run_id"] = "not-a-run"
            files["OBSERVATION.json"] = BASE_TESTS.canonical(observation)
            files["OBSERVATION.sha256"] = (
                f"{BASE_TESTS.sha(files['OBSERVATION.json'])}  OBSERVATION.json\n".encode("ascii")
            )
            with self.assertRaisesRegex(HARDENED.VerificationError, "run_id must be ASCII decimal"):
                HARDENED.verify_envelope(
                    BASE_TESTS.args(BASE_TESTS.write_zip(Path(temporary), files))
                )


if __name__ == "__main__":
    unittest.main()
