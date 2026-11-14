#!/usr/bin/env python3
"""Closed suite profile for rust-workspace-package-v1."""
from __future__ import annotations
import hashlib, json, re
from pathlib import Path
from typing import Any, NoReturn

SCHEMA_ID = "luminous.rust-workspace-package-profile.v1"
MAX_BYTES = 32_768
IDENT_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_-]{0,127}$")
PATH_COMPONENT_RE = re.compile(r"^[A-Za-z0-9._@+\-]+$")
FIELDS = {"schema_id", "package", "integration_test", "edition", "test_path"}
class SuiteProfileValidationError(ValueError): pass
def _fail(message: str) -> NoReturn: raise SuiteProfileValidationError(message)
def _object_no_duplicates(pairs):
    out = {}
    for k,v in pairs:
        if k in out: _fail(f"duplicate JSON object key: {k}")
        out[k]=v
    return out
def _repo_path(value: Any) -> str:
    if not isinstance(value,str) or not value or len(value.encode("utf-8"))>256: _fail("test_path must be a non-empty repository-relative path <=256 bytes")
    if value.startswith("/") or value.endswith("/") or "\\" in value or "//" in value or "\x00" in value: _fail("test_path must be normalized")
    for c in value.split("/"):
        if c in {"",".",".."} or c.casefold()==".git": _fail("test_path contains forbidden component")
        if not PATH_COMPONENT_RE.fullmatch(c): _fail("test_path contains non-canonical component")
    return value
def load_profile_bytes(raw: bytes):
    if len(raw)>MAX_BYTES: _fail(f"suite profile exceeds {MAX_BYTES} bytes")
    if raw.startswith(b"\xef\xbb\xbf"): _fail("UTF-8 BOM is forbidden")
    try: value=json.loads(raw.decode("utf-8"), object_pairs_hook=_object_no_duplicates)
    except SuiteProfileValidationError: raise
    except (UnicodeDecodeError,json.JSONDecodeError) as exc: _fail(f"invalid suite profile JSON/UTF-8: {exc}")
    if not isinstance(value,dict): _fail("suite profile root must be an object")
    return value, hashlib.sha256(raw).hexdigest()
def load_profile(path: Path):
    try: return load_profile_bytes(path.read_bytes())
    except OSError as exc: _fail(f"cannot read suite profile: {exc}")
def validate_profile(value):
    actual=set(value)
    if actual!=FIELDS:
        missing=sorted(FIELDS-actual); extra=sorted(actual-FIELDS)
        if missing: _fail(f"suite profile missing fields: {', '.join(missing)}")
        _fail(f"suite profile has unknown fields: {', '.join(extra)}")
    if value["schema_id"]!=SCHEMA_ID: _fail(f"schema_id must equal {SCHEMA_ID!r}")
    for field in ("package","integration_test"):
        if not isinstance(value[field],str) or not IDENT_RE.fullmatch(value[field]): _fail(f"{field} has invalid grammar")
    if value["edition"] not in {"2018","2021","2024"}: _fail("edition must be one of 2018, 2021, 2024")
    test_path=_repo_path(value["test_path"]); expected=value["integration_test"]+".rs"; parts=test_path.split("/")
    if len(parts)<2 or parts[-2]!="tests" or parts[-1]!=expected: _fail("test_path must end in tests/<integration_test>.rs")
    return value
