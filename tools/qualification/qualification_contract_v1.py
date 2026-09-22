#!/usr/bin/env python3
"""Closed parser for Luminous authoritative qualification contracts v1."""
from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path
from typing import Any, NoReturn

SCHEMA_ID = "luminous.qualification-contract.v1"
MAX_BYTES = 65_536
MAX_PATH_BYTES = 256
CONTRACT_ID_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$")
SHA1_RE = re.compile(r"^[0-9a-f]{40}$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
REPO_RE = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")
PATH_COMPONENT_RE = re.compile(r"^[A-Za-z0-9._@+\-]+$")
FIELDS = {
    "schema_id", "contract_id", "contract_version", "subject_repository",
    "subject_head_sha", "subject_tree_sha", "manifest_path", "manifest_sha256",
    "suite_profile_path", "suite_profile_sha256",
}

class ContractValidationError(ValueError):
    pass

def _fail(message: str) -> NoReturn:
    raise ContractValidationError(message)

def _reject_constant(value: str) -> NoReturn:
    _fail(f"non-finite JSON number is forbidden: {value}")

def _object_no_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    out: dict[str, Any] = {}
    for key, value in pairs:
        if key in out:
            _fail(f"duplicate JSON object key: {key}")
        out[key] = value
    return out

def repo_path(value: Any, field: str) -> str:
    if not isinstance(value, str) or not value:
        _fail(f"{field} must be a non-empty string")
    if len(value.encode("utf-8")) > MAX_PATH_BYTES:
        _fail(f"{field} exceeds {MAX_PATH_BYTES} UTF-8 bytes")
    if value.startswith("/") or value.endswith("/") or "\\" in value or "//" in value or "\x00" in value:
        _fail(f"{field} must be a normalized repository-relative path")
    for component in value.split("/"):
        if component in {"", ".", ".."} or component.casefold() == ".git":
            _fail(f"{field} contains forbidden path component: {component!r}")
        if not PATH_COMPONENT_RE.fullmatch(component):
            _fail(f"{field} contains non-canonical component: {component!r}")
    return value

def load_contract_bytes(raw: bytes) -> tuple[dict[str, Any], str]:
    if len(raw) > MAX_BYTES:
        _fail(f"contract exceeds {MAX_BYTES} bytes")
    if raw.startswith(b"\xef\xbb\xbf"):
        _fail("UTF-8 BOM is forbidden")
    try:
        text = raw.decode("utf-8")
        value = json.loads(text, object_pairs_hook=_object_no_duplicates, parse_constant=_reject_constant)
    except ContractValidationError:
        raise
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        _fail(f"invalid contract JSON/UTF-8: {exc}")
    if not isinstance(value, dict):
        _fail("contract root must be an object")
    return value, hashlib.sha256(raw).hexdigest()

def load_contract(path: Path) -> tuple[dict[str, Any], str]:
    try:
        return load_contract_bytes(path.read_bytes())
    except OSError as exc:
        _fail(f"cannot read contract: {exc}")

def validate_contract(value: dict[str, Any]) -> dict[str, Any]:
    actual = set(value)
    if actual != FIELDS:
        missing = sorted(FIELDS - actual)
        extra = sorted(actual - FIELDS)
        if missing:
            _fail(f"contract missing fields: {', '.join(missing)}")
        _fail(f"contract has unknown fields: {', '.join(extra)}")
    if value["schema_id"] != SCHEMA_ID:
        _fail(f"schema_id must equal {SCHEMA_ID!r}")
    if not isinstance(value["contract_id"], str) or not CONTRACT_ID_RE.fullmatch(value["contract_id"]):
        _fail("contract_id has invalid grammar")
    version = value["contract_version"]
    if isinstance(version, bool) or not isinstance(version, int) or not 1 <= version <= 0xFFFFFFFF:
        _fail("contract_version must be an integer in [1, 4294967295]")
    repo = value["subject_repository"]
    if not isinstance(repo, str) or not REPO_RE.fullmatch(repo):
        _fail("subject_repository has invalid owner/name grammar")
    if repo != "Luminous-Dynamics/symtropy":
        _fail("v1 supports only Luminous-Dynamics/symtropy subjects")
    for field in ("subject_head_sha", "subject_tree_sha"):
        if not isinstance(value[field], str) or not SHA1_RE.fullmatch(value[field]):
            _fail(f"{field} must be 40 lowercase hexadecimal characters")
    for field in ("manifest_sha256", "suite_profile_sha256"):
        if not isinstance(value[field], str) or not SHA256_RE.fullmatch(value[field]):
            _fail(f"{field} must be 64 lowercase hexadecimal characters")
    manifest = repo_path(value["manifest_path"], "manifest_path")
    profile = repo_path(value["suite_profile_path"], "suite_profile_path")
    if manifest == profile:
        _fail("manifest_path and suite_profile_path must be distinct")
    return value

def resolve_regular_file(root: Path, rel: str, field: str) -> Path:
    root = root.resolve()
    candidate = root.joinpath(*rel.split("/"))
    try:
        resolved = candidate.resolve(strict=True)
        resolved.relative_to(root)
    except (OSError, ValueError) as exc:
        _fail(f"{field} cannot be resolved safely: {exc}")
    if candidate.is_symlink() or not resolved.is_file():
        _fail(f"{field} must name a regular non-symlink file")
    return resolved
