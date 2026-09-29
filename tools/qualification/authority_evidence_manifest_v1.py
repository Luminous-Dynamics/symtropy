#!/usr/bin/env python3
"""Canonical QUAL-001B authority evidence envelope manifest v1."""
from __future__ import annotations
import argparse, hashlib, json, sys
from pathlib import Path

SCHEMA_ID="luminous.authority-evidence-manifest.v1"
SCHEMA_VERSION=1
MAX_BYTES=65536
MEMBERS=(
 ("authority-dispatch-payload.json","dispatch_payload"),
 ("authority-dispatch-evidence.json","dispatch_evidence"),
 ("verifier-release-v1.json","verifier_release"),
 ("qualification-contract-v1.json","qualification_contract"),
 ("qualification-execution-evidence-v1.json","execution_evidence"),
)
class ManifestValidationError(ValueError): pass
def _fail(message): raise ManifestValidationError(message)
def _reject_constant(value): _fail(f"non-finite JSON number is forbidden: {value}")
def _object_no_duplicates(pairs):
 out={}
 for k,v in pairs:
  if k in out: _fail(f"duplicate JSON object key: {k}")
  out[k]=v
 return out
def canonical(value): return json.dumps(value,sort_keys=True,separators=(",",":"),ensure_ascii=False).encode()
def validate(value):
 if not isinstance(value,dict) or set(value)!={"schema_id","schema_version","members","root_sha256"}: _fail("manifest fields are not the closed v1 set")
 if value["schema_id"]!=SCHEMA_ID or value["schema_version"]!=SCHEMA_VERSION: _fail("manifest schema identity mismatch")
 members=value["members"]
 if not isinstance(members,list) or len(members)!=len(MEMBERS): _fail("manifest member count mismatch")
 for item,(expected_name,expected_role) in zip(members,MEMBERS):
  if not isinstance(item,dict) or set(item)!={"name","role","byte_len","sha256"}: _fail("manifest member fields are not closed")
  if item["name"]!=expected_name or item["role"]!=expected_role: _fail("manifest member identity/order mismatch")
  if not isinstance(item["byte_len"],int) or isinstance(item["byte_len"],bool) or item["byte_len"]<1: _fail("manifest byte_len is invalid")
  digest=item["sha256"]
  if not isinstance(digest,str) or len(digest)!=64 or any(c not in "0123456789abcdef" for c in digest): _fail("manifest sha256 is invalid")
 body={"schema_id":value["schema_id"],"schema_version":value["schema_version"],"members":members}
 if value["root_sha256"]!=hashlib.sha256(canonical(body)).hexdigest(): _fail("manifest root_sha256 mismatch")
 return value
def load(raw):
 if len(raw)>MAX_BYTES or raw.startswith(b"\xef\xbb\xbf"): _fail("invalid manifest bytes")
 try: value=json.loads(raw.decode(),object_pairs_hook=_object_no_duplicates,parse_constant=_reject_constant)
 except (UnicodeDecodeError,json.JSONDecodeError) as exc: _fail(f"invalid UTF-8/JSON: {exc}")
 validate(value); return value,hashlib.sha256(raw).hexdigest()
def build(directory):
 members=[]
 for name,role in MEMBERS:
  p=directory/name
  try: raw=p.read_bytes()
  except OSError as exc: _fail(f"missing evidence member {name}: {exc}")
  if not raw: _fail(f"empty evidence member {name}")
  members.append({"name":name,"role":role,"byte_len":len(raw),"sha256":hashlib.sha256(raw).hexdigest()})
 body={"schema_id":SCHEMA_ID,"schema_version":SCHEMA_VERSION,"members":members}
 return {**body,"root_sha256":hashlib.sha256(canonical(body)).hexdigest()}
def verify(value,directory):
 validate(value)
 expected={m["name"]:(m["byte_len"],m["sha256"]) for m in value["members"]}
 for name,(length,digest) in expected.items():
  try: raw=(directory/name).read_bytes()
  except OSError as exc: _fail(f"missing evidence member {name}: {exc}")
  if len(raw)!=length: _fail(f"evidence length mismatch: {name}")
  if hashlib.sha256(raw).hexdigest()!=digest: _fail(f"evidence digest mismatch: {name}")
 actual=sorted(p.name for p in directory.iterdir() if p.is_file() and p.name!="authority-evidence-manifest-v1.json")
 if actual!=sorted(expected): _fail("evidence directory contains files outside the closed member set")
def main(argv=None):
 p=argparse.ArgumentParser(); sub=p.add_subparsers(dest="command",required=True)
 b=sub.add_parser("build"); b.add_argument("--evidence-dir",type=Path,required=True); b.add_argument("--output",type=Path,required=True)
 v=sub.add_parser("verify"); v.add_argument("--manifest",type=Path,required=True); v.add_argument("--evidence-dir",type=Path,required=True)
 a=p.parse_args(argv)
 try:
  if a.command=="build":
   value=build(a.evidence_dir); a.output.write_bytes(canonical(value)+b"\n"); print("root_sha256="+value["root_sha256"]); return 0
  value,digest=load(a.manifest.read_bytes()); verify(value,a.evidence_dir); print("QUAL001B_AUTHORITY_EVIDENCE_MANIFEST_V1 PASS"); print("root_sha256="+value["root_sha256"]); print("manifest_sha256="+digest); return 0
 except (OSError,ManifestValidationError) as exc: print(f"QUAL001B_AUTHORITY_EVIDENCE_MANIFEST_V1 FAIL: {exc}",file=sys.stderr); return 2
if __name__=="__main__": raise SystemExit(main())
