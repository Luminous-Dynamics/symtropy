#!/usr/bin/env python3
"""Closed parser for the QUAL-001B verifier-release identity record v1."""
from __future__ import annotations
import argparse, hashlib, json, re, sys
from pathlib import Path
from typing import Any, NoReturn
SCHEMA_ID="luminous.verifier-release.v1"; MAX_BYTES=16384
ID_RE=re.compile(r"^[A-Za-z0-9][A-Za-z0-9._/-]{0,127}$"); SHA1_RE=re.compile(r"^[0-9a-f]{40}$"); SHA256_RE=re.compile(r"^[0-9a-f]{64}$"); REPO_RE=re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")
FIELDS={"schema_id","release_id","release_version","verifier_repository","verifier_commit_sha","verifier_tree_sha","verifier_profile_id","verifier_profile_sha256","contract_schema_id","contract_schema_version","suite_id","suite_revision","toolchain_id"}
class VerifierReleaseValidationError(ValueError): pass
def _fail(message:str)->NoReturn: raise VerifierReleaseValidationError(message)
def _reject_constant(value:str)->NoReturn: _fail(f"non-finite JSON number is forbidden: {value}")
def _object_no_duplicates(pairs:list[tuple[str,Any]])->dict[str,Any]:
    out={}
    for key,value in pairs:
        if key in out: _fail(f"duplicate JSON object key: {key}")
        out[key]=value
    return out
def load_record_bytes(raw:bytes)->tuple[dict[str,Any],str]:
    if len(raw)>MAX_BYTES: _fail(f"release record exceeds {MAX_BYTES} bytes")
    if raw.startswith(b"\xef\xbb\xbf"): _fail("UTF-8 BOM is forbidden")
    try: value=json.loads(raw.decode("utf-8"),object_pairs_hook=_object_no_duplicates,parse_constant=_reject_constant)
    except VerifierReleaseValidationError: raise
    except (UnicodeDecodeError,json.JSONDecodeError) as exc: _fail(f"invalid release JSON/UTF-8: {exc}")
    if not isinstance(value,dict): _fail("release record root must be an object")
    return value,hashlib.sha256(raw).hexdigest()
def load_record(path:Path)->tuple[dict[str,Any],str]:
    try: return load_record_bytes(path.read_bytes())
    except OSError as exc: _fail(f"cannot read release record: {exc}")
def validate_record(value:dict[str,Any])->dict[str,Any]:
    actual=set(value)
    if actual!=FIELDS:
        missing,extra=sorted(FIELDS-actual),sorted(actual-FIELDS)
        if missing: _fail(f"release record missing fields: {', '.join(missing)}")
        _fail(f"release record has unknown fields: {', '.join(extra)}")
    if value["schema_id"]!=SCHEMA_ID: _fail(f"schema_id must equal {SCHEMA_ID!r}")
    for field in ("release_id","verifier_profile_id","contract_schema_id","suite_id","suite_revision","toolchain_id"):
        if not isinstance(value[field],str) or not ID_RE.fullmatch(value[field]): _fail(f"{field} has invalid grammar")
    version=value["release_version"]
    if isinstance(version,bool) or not isinstance(version,int) or not 1<=version<=0xFFFFFFFF: _fail("release_version must be an integer in [1, 4294967295]")
    repo=value["verifier_repository"]
    if not isinstance(repo,str) or not REPO_RE.fullmatch(repo): _fail("verifier_repository has invalid owner/name grammar")
    if repo!="Luminous-Dynamics/symtropy": _fail("v1 supports only Luminous-Dynamics/symtropy verifier releases")
    for field in ("verifier_commit_sha","verifier_tree_sha"):
        if not isinstance(value[field],str) or not SHA1_RE.fullmatch(value[field]): _fail(f"{field} must be 40 lowercase hexadecimal characters")
    if not isinstance(value["verifier_profile_sha256"],str) or not SHA256_RE.fullmatch(value["verifier_profile_sha256"]): _fail("verifier_profile_sha256 must be 64 lowercase hexadecimal characters")
    if value["contract_schema_id"]!="luminous.qualification-contract.v1": _fail("contract_schema_id must equal luminous.qualification-contract.v1")
    if value["contract_schema_version"]!=1: _fail("contract_schema_version must equal 1")
    if value["suite_id"]!="rust-workspace-package-v1": _fail("suite_id must equal rust-workspace-package-v1")
    if value["suite_revision"]!="rust-workspace-package-v1/1": _fail("suite_revision must equal rust-workspace-package-v1/1")
    if value["toolchain_id"]!="rust-1.96.0": _fail("toolchain_id must equal rust-1.96.0")
    return value
def validate_checkout_identity(*,head_sha:str,tree_sha:str,record:dict[str,Any])->None:
    validate_record(record)
    if head_sha!=record["verifier_commit_sha"]: _fail("checked-out verifier HEAD does not match approved release commit")
    if tree_sha!=record["verifier_tree_sha"]: _fail("checked-out verifier tree does not match approved release tree")
def main(argv:list[str]|None=None)->int:
    parser=argparse.ArgumentParser(description=__doc__); parser.add_argument("--record",type=Path,required=True); parser.add_argument("--expected-sha256"); args=parser.parse_args(argv)
    try:
        value,digest=load_record(args.record); validate_record(value)
        if args.expected_sha256 is not None and digest!=args.expected_sha256: _fail("release record SHA-256 does not match expected digest")
    except VerifierReleaseValidationError as exc: print(f"VERIFIER_RELEASE_V1 FAIL: {exc}",file=sys.stderr); return 2
    print("VERIFIER_RELEASE_V1 PASS"); print(f"release_id={value['release_id']}"); print(f"release_sha256={digest}"); print(f"verifier_repository={value['verifier_repository']}"); print(f"verifier_commit_sha={value['verifier_commit_sha']}"); print(f"verifier_tree_sha={value['verifier_tree_sha']}"); print(f"verifier_profile_id={value['verifier_profile_id']}"); print(f"contract_schema_id={value['contract_schema_id']}"); print(f"contract_schema_version={value['contract_schema_version']}"); print(f"suite_id={value['suite_id']}"); print(f"suite_revision={value['suite_revision']}"); print(f"toolchain_id={value['toolchain_id']}"); return 0
if __name__=="__main__": raise SystemExit(main())
