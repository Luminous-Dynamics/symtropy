#!/usr/bin/env python3
"""Independently verify QUAL-001B authority evidence against retained immutable bytes."""
from __future__ import annotations
import argparse, json, sys
from pathlib import Path
from tools.qualification import authority_dispatch_v1 as dispatch
from tools.qualification import authority_dispatch_evidence_v1 as evidence
from tools.qualification import qualification_contract_v1 as contract
from tools.qualification import verifier_release_v1 as release

def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--payload", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--release", type=Path, required=True)
    parser.add_argument("--contract", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        payload_raw = args.payload.read_bytes()
        payload_value, payload_digest = dispatch.load_bytes(payload_raw)

        evidence_value = json.loads(args.evidence.read_text(encoding="utf-8"))
        if not isinstance(evidence_value, dict):
            raise ValueError("evidence root must be an object")
        evidence.validate(evidence_value)

        release_raw = args.release.read_bytes()
        release_value, release_digest = release.load_record_bytes(release_raw)
        release.validate_record(release_value)

        contract_raw = args.contract.read_bytes()
        contract_value, contract_digest = contract.load_contract_bytes(contract_raw)
        contract.validate_contract(contract_value)

        if evidence_value["release_sha256"] != release_digest:
            raise ValueError("release_sha256 does not match retained release record bytes")
        if evidence_value["dispatch_sha256"] != payload_digest:
            raise ValueError("dispatch_sha256 does not match payload bytes")
        if evidence_value["contract_sha256"] != contract_digest:
            raise ValueError("contract_sha256 does not match retained contract bytes")
        for key in ("contract_commit_sha", "contract_path", "contract_sha256"):
            if evidence_value[key] != payload_value[key]:
                raise ValueError(f"{key} does not match dispatch payload")
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"AUTHORITY_DISPATCH_EVIDENCE_VERIFY_V1 FAIL: {exc}", file=sys.stderr)
        return 2
    print("AUTHORITY_DISPATCH_EVIDENCE_VERIFY_V1 PASS")
    print(f"dispatch_sha256={payload_digest}")
    print(f"release_sha256={release_digest}")
    print(f"contract_sha256={contract_digest}")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
