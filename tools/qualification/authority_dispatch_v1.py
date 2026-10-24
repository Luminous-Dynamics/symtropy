#!/usr/bin/env python3
"""Closed validator for the QUAL-001B authority dispatch payload v1."""
from __future__ import annotations
import argparse, hashlib, json, re, sys
from pathlib import Path
from typing import Any, NoReturn
SCHEMA_ID="luminous.authority-dispatch.v1"; MAX_BYTES=8192
FIELDS={"schema_id","schema_version","contract_commit_sha","contract_path","contract_sha256"}
SHA1_RE=re.compile(r"^[0-9a-f]{40}$"); SHA256_RE=re.compile(r"^[0-9a-f]{64}$")
def _fail(message:str)->NoReturn: raise ValueError(message)
def _reject_constant(value:str)->NoReturn: _fail(f"non-finite JSON number is forbidden: {value}")
def _pairs(pairs:list[tuple[str,Any]])->dict[str,Any]:
    out={}
    for k,v in pairs:
        if k in out: _fail(f"duplicate JSON object key: {k}")
        out[k]=v
    return out
def _path(value:Any)->str:
    if not isinstance(value,str) or not value or len(value)>512: _fail("contract_path must be a non-empty string of at most 512 bytes")
    if "\x00" in value or value.startswith(("/", "\\", "~")) or "\\" in value:
        _fail("contract_path must be a repository-relative POSIX path")
    parts=value.split("/")
    if any(p in ("",".","..") for p in parts): _fail("contract_path contains forbidden path components")
    return value
def validate(value:dict[str,Any])->dict[str,Any]:
    if set(value)!=FIELDS: _fail(f"dispatch fields must be exactly {sorted(FIELDS)!r}")
    if value["schema_id"]!=SCHEMA_ID: _fail(f"schema_id must equal {SCHEMA_ID!r}")
    if value["schema_version"]!=1: _fail("schema_version must equal 1")
    if not isinstance(value["contract_commit_sha"],str) or not SHA1_RE.fullmatch(value["contract_commit_sha"]): _fail("contract_commit_sha must be 40 lowercase hexadecimal characters")
    _path(value["contract_path"])
    if not isinstance(value["contract_sha256"],str) or not SHA256_RE.fullmatch(value["contract_sha256"]): _fail("contract_sha256 must be 64 lowercase hexadecimal characters")
    return value
def load_bytes(raw:bytes)->tuple[dict[str,Any],str]:
    if len(raw)>MAX_BYTES: _fail(f"dispatch payload exceeds {MAX_BYTES} bytes")
    if raw.startswith(b"\xef\xbb\xbf"): _fail("UTF-8 BOM is forbidden")
    try: value=json.loads(raw.decode("utf-8"),object_pairs_hook=_pairs,parse_constant=_reject_constant)
    except ValueError: raise
    except (UnicodeDecodeError,json.JSONDecodeError) as exc: _fail(f"invalid JSON/UTF-8: {exc}")
    if not isinstance(value,dict): _fail("dispatch payload root must be an object")
    validate(value)
    return value,hashlib.sha256(raw).hexdigest()
def main(argv:list[str]|None=None)->int:
    p=argparse.ArgumentParser(description=__doc__); p.add_argument("--payload",type=Path,required=True); args=p.parse_args(argv)
    try: value,digest=load_bytes(args.payload)
    except ValueError as exc: print(f"AUTHORITY_DISPATCH_V1 FAIL: {exc}",file=sys.stderr); return 2
    print("AUTHORITY_DISPATCH_V1 PASS"); print(f"dispatch_sha256={digest}"); print(f"contract_commit_sha={value['contract_commit_sha']}"); print(f"contract_path={value['contract_path']}"); print(f"contract_sha256={value['contract_sha256']}"); return 0
if __name__=="__main__": raise SystemExit(main())
