#!/usr/bin/env python3
"""Closed validator for QUAL-001B authority dispatch evidence v1."""
from __future__ import annotations
import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any, NoReturn

SCHEMA_ID = "luminous.authority-dispatch-evidence.v1"
FIELDS = {
    "schema_id", "schema_version", "dispatch_schema_id",
    "dispatch_sha256", "release_sha256",
    "contract_commit_sha", "contract_path", "contract_sha256",
}
SHA1_RE = re.compile(r"^[0-9a-f]{40}$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")

def _fail(message: str) -> NoReturn:
    raise ValueError(message)

def _path(value: Any) -> str:
    if not isinstance(value, str) or not value or len(value) > 512:
        _fail("contract_path must be a non-empty string of at most 512 bytes")
    if "\x00" in value or value.startswith(("/", "\\", "~")) or "\\" in value:
        _fail("contract_path must be a repository-relative POSIX path")
    if any(part in ("", ".", "..") for part in value.split("/")):
        _fail("contract_path contains forbidden path components")
    return value

def validate(value: dict[str, Any]) -> dict[str, Any]:
    if set(value) != FIELDS:
        _fail(f"evidence fields must be exactly {sorted(FIELDS)!r}")
    if value["schema_id"] != SCHEMA_ID:
        _fail(f"schema_id must equal {SCHEMA_ID!r}")
    if value["schema_version"] != 1:
        _fail("schema_version must equal 1")
    if value["dispatch_schema_id"] != "luminous.authority-dispatch.v1":
        _fail("dispatch_schema_id must identify authority-dispatch.v1")
    if not isinstance(value["dispatch_sha256"], str) or not SHA256_RE.fullmatch(value["dispatch_sha256"]):
        _fail("dispatch_sha256 must be 64 lowercase hexadecimal characters")
    if not isinstance(value["release_sha256"], str) or not SHA256_RE.fullmatch(value["release_sha256"]):
        _fail("release_sha256 must be 64 lowercase hexadecimal characters")
    if not isinstance(value["contract_commit_sha"], str) or not SHA1_RE.fullmatch(value["contract_commit_sha"]):
        _fail("contract_commit_sha must be 40 lowercase hexadecimal characters")
    _path(value["contract_path"])
    if not isinstance(value["contract_sha256"], str) or not SHA256_RE.fullmatch(value["contract_sha256"]):
        _fail("contract_sha256 must be 64 lowercase hexadecimal characters")
    return value

def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        value = json.loads(args.evidence.read_text(encoding="utf-8"))
        if not isinstance(value, dict):
            _fail("evidence root must be an object")
        validate(value)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"AUTHORITY_DISPATCH_EVIDENCE_V1 FAIL: {exc}", file=sys.stderr)
        return 2
    print("AUTHORITY_DISPATCH_EVIDENCE_V1 PASS")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
