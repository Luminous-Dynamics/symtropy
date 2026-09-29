#!/usr/bin/env python3
"""Independently verify QUAL-001B authority-dispatch evidence against payload bytes."""
from __future__ import annotations
import argparse, hashlib, json, sys
from pathlib import Path
from tools.qualification import authority_dispatch_v1 as dispatch
from tools.qualification import authority_dispatch_evidence_v1 as evidence

def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--payload", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        payload_raw = args.payload.read_bytes()
        payload_value, payload_digest = dispatch.load_bytes(payload_raw)
        evidence_value = json.loads(args.evidence.read_text(encoding="utf-8"))
        if not isinstance(evidence_value, dict):
            raise ValueError("evidence root must be an object")
        evidence.validate(evidence_value)
        if evidence_value["dispatch_sha256"] != payload_digest:
            raise ValueError("dispatch_sha256 does not match payload bytes")
        for key in ("contract_commit_sha", "contract_path", "contract_sha256"):
            if evidence_value[key] != payload_value[key]:
                raise ValueError(f"{key} does not match dispatch payload")
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"AUTHORITY_DISPATCH_EVIDENCE_VERIFY_V1 FAIL: {exc}", file=sys.stderr)
        return 2
    print("AUTHORITY_DISPATCH_EVIDENCE_VERIFY_V1 PASS")
    print(f"dispatch_sha256={payload_digest}")
    print(f"release_sha256={evidence_value["release_sha256"]}")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())