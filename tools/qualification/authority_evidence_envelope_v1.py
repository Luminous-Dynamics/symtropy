#!/usr/bin/env python3
"""Independently replay the complete QUAL-001B authority evidence envelope."""
from __future__ import annotations
import argparse, json, sys
from pathlib import Path
from tools.qualification import authority_dispatch_v1 as dispatch
from tools.qualification import authority_dispatch_evidence_v1 as dispatch_evidence
from tools.qualification import qualification_contract_v1 as contract
from tools.qualification import verifier_release_v1 as release

REQUIRED_CHECKOUT_FIELDS = {
    "schema_id", "schema_version", "contract_commit_sha", "contract_tree_sha", "contract_blob_sha",
}
\nREQUIRED_EXECUTION_FIELDS = {
    "schema_id","verifier_commit_sha","verifier_tree_sha","contract_commit_sha",
    "contract_tree_sha","contract_sha256","contract_id","subject_repository",
    "subject_head_sha","subject_tree_sha","manifest_sha256","manifest_profile_id",
    "suite_profile_sha256","suite_id","suite_revision","toolchain_id",
    "expanded_step_ids","executed_step_ids","steps","first_failing_step_id",
    "first_failing_step_result","final_result",
}
FINAL_RESULTS = {"TheoremExecutedPass","TheoremExecutedFail","InfrastructureFailure"}

def _load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))

def main(argv: list[str] | None = None) -> int:
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("--payload",type=Path,required=True)
    p.add_argument("--dispatch-evidence",type=Path,required=True)
    p.add_argument("--release",type=Path,required=True)
    p.add_argument("--contract",type=Path,required=True)
    p.add_argument("--execution-evidence",type=Path,required=True)
    p.add_argument("--contract-checkout-identity",type=Path,required=True)
    a=p.parse_args(argv)
    try:
        payload_raw=a.payload.read_bytes()
        payload,payload_digest=dispatch.load_bytes(payload_raw)
        de=_load_json(a.dispatch_evidence); dispatch_evidence.validate(de)
        release_value,release_digest=release.load_record_bytes(a.release.read_bytes())
        release.validate_record(release_value)
        contract_value,contract_digest=contract.load_contract_bytes(a.contract.read_bytes())
        contract.validate_contract(contract_value)
        checkout=_load_json(a.contract_checkout_identity)
        if not isinstance(checkout,dict) or set(checkout)!=REQUIRED_CHECKOUT_FIELDS:
            raise ValueError("contract checkout identity fields are not the closed v1 set")
        if checkout["schema_id"]!="luminous.qualification-contract-checkout-identity.v1" or checkout["schema_version"]!=1:
            raise ValueError("contract checkout identity schema mismatch")
        if not all(isinstance(checkout[k],str) and len(checkout[k])==40 and all(c in "0123456789abcdef" for c in checkout[k]) for k in ("contract_commit_sha","contract_tree_sha","contract_blob_sha")):
            raise ValueError("contract checkout identity is invalid")
        contract_raw=a.contract.read_bytes()
        computed_blob=__import__("hashlib").sha1(b"blob "+str(len(contract_raw)).encode()+b"\0"+contract_raw).hexdigest()
        if checkout["contract_blob_sha"]!=computed_blob:
            raise ValueError("retained contract bytes do not match recorded Git blob")
        execution=_load_json(a.execution_evidence)
        if not isinstance(execution,dict) or set(execution)!=REQUIRED_EXECUTION_FIELDS:
            raise ValueError("execution evidence fields are not the closed QUAL-001B v1 evidence set")
        if execution["schema_id"]!="luminous.qualification-execution-evidence.v1":
            raise ValueError("execution evidence schema_id mismatch")
        if execution["final_result"] not in FINAL_RESULTS:
            raise ValueError("execution evidence final_result is invalid")
        if de["dispatch_sha256"]!=payload_digest: raise ValueError("dispatch digest mismatch")
        if de["release_sha256"]!=release_digest: raise ValueError("release digest mismatch")
        if de["contract_sha256"]!=contract_digest: raise ValueError("contract digest mismatch")
        for key in ("contract_commit_sha","contract_path","contract_sha256"):
            if de[key]!=payload[key]: raise ValueError(f"dispatch/evidence mismatch: {key}")
        for key in ("verifier_commit_sha","verifier_tree_sha","suite_id","suite_revision","toolchain_id"):
            if release_value[key]!=execution[key]: raise ValueError(f"release/execution mismatch: {key}")
        if checkout["contract_commit_sha"]!=payload["contract_commit_sha"]:
            raise ValueError("retained contract checkout commit does not match dispatch")
        if checkout["contract_tree_sha"]!=execution["contract_tree_sha"]:
            raise ValueError("retained contract checkout tree does not match execution identity")
        if execution["contract_commit_sha"]!=payload["contract_commit_sha"]:
            raise ValueError("execution contract commit does not match dispatch")
        if execution["contract_sha256"]!=contract_digest:
            raise ValueError("execution contract digest does not match retained contract")
        for key in ("subject_repository","subject_head_sha","subject_tree_sha","manifest_sha256","suite_profile_sha256"):
            if execution[key]!=contract_value[key]: raise ValueError(f"contract/execution mismatch: {key}")
        print("QUAL001B_AUTHORITY_EVIDENCE_ENVELOPE_V1 PASS")
        print(f"dispatch_sha256={payload_digest}")
        print(f"release_sha256={release_digest}")
        print(f"contract_sha256={contract_digest}")
        print(f"execution_final_result={execution['final_result']}")
        return 0
    except (OSError,ValueError,json.JSONDecodeError) as exc:
        print(f"QUAL001B_AUTHORITY_EVIDENCE_ENVELOPE_V1 FAIL: {exc}",file=sys.stderr)
        return 2

if __name__=="__main__": raise SystemExit(main())
