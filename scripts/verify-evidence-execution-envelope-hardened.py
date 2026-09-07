#!/usr/bin/env python3
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Hardened hostile-input boundary for the qualified v1 envelope verifier."""

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent


def _load(name: str, filename: str):
    spec = importlib.util.spec_from_file_location(name, HERE / filename)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


BASE = _load("evidence_envelope_verifier_core", "verify-evidence-execution-envelope.py")
INGEST = _load("evidence_envelope_ingest", "evidence-envelope-ingest.py")
VerificationError = BASE.VerificationError

FLOATING_GUARD_NAMESPACES = {"environment", "tool"}
FLOATING_ALIASES = {
    "current", "default", "head", "latest", "main", "master",
    "nightly", "stable", "tip", "trunk",
}
LATEST_SUFFIXES = ("-latest", "/latest", ":latest", "@latest")


def _parse_strict_json(path: Path) -> dict[str, object]:
    def reject_constant(token: str) -> object:
        raise VerificationError(f"{path.name} contains non-standard JSON constant: {token}")

    try:
        value = json.loads(path.read_text(encoding="utf-8"), parse_constant=reject_constant)
    except UnicodeDecodeError as error:
        raise VerificationError(f"{path.name} is not UTF-8") from error
    except json.JSONDecodeError as error:
        raise VerificationError(f"{path.name} is not valid JSON") from error
    if not isinstance(value, dict):
        raise VerificationError(f"{path.name} top level must be an object")
    return value


def _validate_hardening(directory: Path) -> None:
    identity = _parse_strict_json(directory / "EXECUTION_IDENTITY.json")
    observation = _parse_strict_json(directory / "OBSERVATION.json")

    bindings = identity.get("bindings")
    if isinstance(bindings, dict):
        for key, raw_value in bindings.items():
            if not isinstance(key, str) or not isinstance(raw_value, str) or "." not in key:
                continue  # qualified core performs authoritative structure validation
            namespace = key.split(".", 1)[0]
            normalized = raw_value.lower()
            if namespace in FLOATING_GUARD_NAMESPACES and (
                normalized in FLOATING_ALIASES or normalized.endswith(LATEST_SUFFIXES)
            ):
                raise VerificationError(f"binding {key} uses an obvious moving alias")

    github = observation.get("github")
    if isinstance(github, dict):
        for key in ("run_id", "run_attempt"):
            value = github.get(key)
            if isinstance(value, str) and (not value.isascii() or not value.isdigit()):
                raise VerificationError(f"observation github.{key} must be ASCII decimal digits")


def verify_envelope(args: argparse.Namespace) -> dict[str, str]:
    expected_archive = args.expected_archive_sha256
    try:
        with INGEST.materialize_evidence(args.evidence) as (directory, archive_digest):
            if expected_archive is not None:
                if not archive_digest:
                    raise VerificationError(
                        "--expected-archive-sha256 requires a ZIP evidence path"
                    )
                if archive_digest != expected_archive:
                    raise VerificationError(
                        f"archive SHA-256 mismatch: got {archive_digest!r}, "
                        f"expected {expected_archive!r}"
                    )

            _validate_hardening(directory)
            delegated = argparse.Namespace(**vars(args))
            delegated.evidence = directory
            delegated.expected_archive_sha256 = None
            result = BASE.verify_envelope(delegated)
            result["archive_sha256"] = archive_digest
            return result
    except INGEST.IngestError as error:
        raise VerificationError(str(error)) from error


def main() -> int:
    args = BASE.parse_args()
    try:
        result = verify_envelope(args)
    except VerificationError as error:
        raise SystemExit(f"ERROR: {error}") from error
    print(f"PASS identity_sha256={result['identity_sha256']}")
    print(f"PASS observation_sha256={result['observation_sha256']}")
    if result["archive_sha256"]:
        print(f"PASS archive_sha256={result['archive_sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
