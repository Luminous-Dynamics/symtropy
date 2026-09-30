#!/usr/bin/env python3
"""Build and independently verify the recursive QUAL-001B evidence root."""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

from tools.qualification import authority_evidence_envelope_v1 as envelope
from tools.qualification import authority_evidence_manifest_v1 as manifest

SCHEMA_ID = "luminous.authority-evidence-root.v1"
SCHEMA_VERSION = 1
MEMBER_ROLES = (
    ("authority-dispatch-payload.json", "dispatch_payload"),
    ("authority-dispatch-evidence.json", "dispatch_evidence"),
    ("verifier-release-v1.json", "verifier_release"),
    ("qualification-contract-v1.json", "qualification_contract"),
    ("qualification-contract-checkout-identity-v1.json", "qualification_contract_checkout_identity"),
    ("qualification-execution-evidence-v1.json", "execution_evidence"),
)
FINAL_RESULTS = {"TheoremExecutedPass", "TheoremExecutedFail", "InfrastructureFailure"}


class EvidenceRootValidationError(ValueError):
    pass


def _fail(message: str) -> None:
    raise EvidenceRootValidationError(message)


def canonical(value: object) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")


def _object_no_duplicates(pairs):
    out = {}
    for key, value in pairs:
        if key in out:
            _fail(f"duplicate JSON object key: {key}")
        out[key] = value
    return out


def _reject_constant(value):
    _fail(f"non-finite JSON number is forbidden: {value}")


def load_json(raw: bytes):
    try:
        return json.loads(
            raw.decode("utf-8"),
            object_pairs_hook=_object_no_duplicates,
            parse_constant=_reject_constant,
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        _fail(f"invalid UTF-8/JSON: {exc}")


def sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def load_member(directory: Path, name: str):
    try:
        raw = (directory / name).read_bytes()
    except OSError as exc:
        _fail(f"cannot read {name}: {exc}")
    return raw, load_json(raw)


def _build_body(directory: Path, manifest_value: dict, manifest_sha256: str) -> dict:
    members = {}
    values = {}
    for name, role in MEMBER_ROLES:
        raw, value = load_member(directory, name)
        members[role] = sha256(raw)
        values[role] = value

    checkout = values["qualification_contract_checkout_identity"]
    contract = values["qualification_contract"]
    execution = values["execution_evidence"]

    if checkout["contract_commit_sha"] != contract.get("subject_head_sha") and checkout["contract_commit_sha"] != values["dispatch_payload"]["contract_commit_sha"]:
        _fail("checkout identity is not bound to the dispatched contract commit")

    blob = hashlib.sha1(
        b"blob " + str(len((directory / "qualification-contract-v1.json").read_bytes())).encode("ascii")
        + b"\0" + (directory / "qualification-contract-v1.json").read_bytes()
    ).hexdigest()
    if checkout["contract_blob_sha"] != blob:
        _fail("checkout identity blob does not match retained contract bytes")

    if execution["final_result"] not in FINAL_RESULTS:
        _fail("execution final_result is invalid")

    return {
        "schema_id": SCHEMA_ID,
        "schema_version": SCHEMA_VERSION,
        "manifest_sha256": manifest_sha256,
        "manifest_root_sha256": manifest_value["root_sha256"],
        "member_sha256": members,
        "semantic_bindings": {
            "contract_commit_sha": checkout["contract_commit_sha"],
            "contract_tree_sha": checkout["contract_tree_sha"],
            "contract_blob_sha": checkout["contract_blob_sha"],
            "subject_head_sha": execution["subject_head_sha"],
            "subject_tree_sha": execution["subject_tree_sha"],
            "final_result": execution["final_result"],
        },
    }


def _replay_envelope(directory: Path) -> None:
    envelope_args = [
        "--payload", str(directory / "authority-dispatch-payload.json"),
        "--dispatch-evidence", str(directory / "authority-dispatch-evidence.json"),
        "--release", str(directory / "verifier-release-v1.json"),
        "--contract", str(directory / "qualification-contract-v1.json"),
        "--execution-evidence", str(directory / "qualification-execution-evidence-v1.json"),
        "--contract-checkout-identity", str(directory / "qualification-contract-checkout-identity-v1.json"),
    ]
    if envelope.main(envelope_args) != 0:
        _fail("complete authority envelope replay failed")


def build(directory: Path, manifest_path: Path) -> dict:
    try:
        manifest_raw = manifest_path.read_bytes()
    except OSError as exc:
        _fail(f"cannot read manifest: {exc}")
    manifest_value, manifest_sha256 = manifest.load(manifest_raw)
    manifest.verify(manifest_value, directory)
    _replay_envelope(directory)
    return _build_body(directory, manifest_value, manifest_sha256)

def seal(body: dict) -> dict:
    return {**body, "root_sha256": sha256(canonical(body))}


def verify(root_value: dict, directory: Path, manifest_path: Path) -> None:
    expected_fields = {
        "schema_id", "schema_version", "manifest_sha256", "manifest_root_sha256",
        "member_sha256", "semantic_bindings", "root_sha256"
    }
    if set(root_value) != expected_fields:
        _fail("evidence root fields are not the closed v1 set")
    if root_value["schema_id"] != SCHEMA_ID or root_value["schema_version"] != SCHEMA_VERSION:
        _fail("evidence root schema identity mismatch")
    if root_value["root_sha256"] != sha256(
        canonical({k: root_value[k] for k in root_value if k != "root_sha256"})
    ):
        _fail("evidence root_sha256 mismatch")

    body = build(directory, manifest_path)
    if body != {k: root_value[k] for k in body}:
        _fail("evidence root body does not match retained evidence")

def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    build_parser = sub.add_parser("build")
    build_parser.add_argument("--evidence-dir", type=Path, required=True)
    build_parser.add_argument("--manifest", type=Path, required=True)
    build_parser.add_argument("--output", type=Path, required=True)

    verify_parser = sub.add_parser("verify")
    verify_parser.add_argument("--root", type=Path, required=True)
    verify_parser.add_argument("--evidence-dir", type=Path, required=True)
    verify_parser.add_argument("--manifest", type=Path, required=True)

    args = parser.parse_args(argv)
    try:
        if args.command == "build":
            body = build(args.evidence_dir, args.manifest)
            value = seal(body)
            args.output.write_bytes(canonical(value) + b"\n")
            print("root_sha256=" + value["root_sha256"])
            return 0

        root = load_json(args.root.read_bytes())
        verify(root, args.evidence_dir, args.manifest)
        print("QUAL001B_AUTHORITY_EVIDENCE_ROOT_V1 PASS")
        print("root_sha256=" + root["root_sha256"])
        return 0
    except (OSError, EvidenceRootValidationError, ValueError, json.JSONDecodeError) as exc:
        print(f"QUAL001B_AUTHORITY_EVIDENCE_ROOT_V1 FAIL: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
