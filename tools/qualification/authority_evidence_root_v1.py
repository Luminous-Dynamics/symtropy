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
GRAPH_NODE_IDS = (
    "dispatch_payload", "dispatch_evidence", "verifier_release",
    "qualification_contract", "qualification_contract_checkout_identity",
    "execution_evidence", "contract_identity", "subject_identity", "final_result",
)
GRAPH_EDGES = (
    ("dispatch_payload", "binds", "qualification_contract"),
    ("dispatch_payload", "selects", "verifier_release"),
    ("qualification_contract_checkout_identity", "materializes", "qualification_contract"),
    ("execution_evidence", "uses", "qualification_contract"),
    ("execution_evidence", "targets", "subject_identity"),
    ("execution_evidence", "produces", "final_result"),
)


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


def _build_graph(values: dict, members: dict) -> dict:
    checkout = values["qualification_contract_checkout_identity"]
    contract = values["qualification_contract"]
    dispatch = values["dispatch_payload"]
    dispatch_evidence = values["dispatch_evidence"]
    execution = values["execution_evidence"]
    nodes = [
        *[{"id": role, "kind": "evidence", "sha256": members[role]} for _, role in MEMBER_ROLES],
        {"id": "contract_identity", "kind": "identity",
         "commit_sha": checkout["contract_commit_sha"],
         "tree_sha": checkout["contract_tree_sha"],
         "blob_sha": checkout["contract_blob_sha"]},
        {"id": "subject_identity", "kind": "identity",
         "head_sha": execution["subject_head_sha"],
         "tree_sha": execution["subject_tree_sha"]},
        {"id": "final_result", "kind": "observation", "value": execution["final_result"]},
    ]
    edges = [{"from": source, "type": edge_type, "to": target}
             for source, edge_type, target in GRAPH_EDGES]
    if dispatch["contract_commit_sha"] != checkout["contract_commit_sha"]:
        _fail("graph dispatch binding does not match contract identity")
    if dispatch_evidence["release_sha256"] != members["verifier_release"]:
        _fail("graph release selection does not match retained release evidence")
    if execution["contract_commit_sha"] != checkout["contract_commit_sha"]:
        _fail("graph execution/contract identity binding mismatch")
    if execution["subject_head_sha"] != contract["subject_head_sha"]:
        _fail("graph execution/subject binding mismatch")
    return {"nodes": nodes, "edges": edges}


def _validate_graph(graph: object) -> None:
    if not isinstance(graph, dict) or set(graph) != {"nodes", "edges"}:
        _fail("provenance graph fields are not the closed v1 set")
    nodes, edges = graph["nodes"], graph["edges"]
    if not isinstance(nodes, list) or len(nodes) != len(GRAPH_NODE_IDS):
        _fail("provenance graph node count mismatch")
    if [n.get("id") if isinstance(n, dict) else None for n in nodes] != list(GRAPH_NODE_IDS):
        _fail("provenance graph node identity/order mismatch")
    expected_edges = [{"from": s, "type": t, "to": d} for s, t, d in GRAPH_EDGES]
    if edges != expected_edges:
        _fail("provenance graph relationship set mismatch")
    if not isinstance(edges, list) or len(edges) != len(GRAPH_EDGES):
        _fail("provenance graph edge count mismatch")
    for node in nodes[:len(MEMBER_ROLES)]:
        if set(node) != {"id", "kind", "sha256"} or node["kind"] != "evidence":
            _fail("provenance graph evidence node is invalid")
        if not isinstance(node["sha256"], str) or len(node["sha256"]) != 64 or any(c not in "0123456789abcdef" for c in node["sha256"]):
            _fail("provenance graph evidence digest is invalid")
    contract_node = nodes[len(MEMBER_ROLES)]
    if set(contract_node) != {"id", "kind", "commit_sha", "tree_sha", "blob_sha"} or contract_node["kind"] != "identity":
        _fail("provenance graph contract identity node is invalid")
    subject_node = nodes[len(MEMBER_ROLES) + 1]
    if set(subject_node) != {"id", "kind", "head_sha", "tree_sha"} or subject_node["kind"] != "identity":
        _fail("provenance graph subject identity node is invalid")
    result_node = nodes[len(MEMBER_ROLES) + 2]
    if set(result_node) != {"id", "kind", "value"} or result_node["kind"] != "observation" or result_node["value"] not in FINAL_RESULTS:
        _fail("provenance graph result observation is invalid")


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

    graph = _build_graph(values, members)
    _validate_graph(graph)

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
        "provenance_graph": graph,
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
        "member_sha256", "semantic_bindings", "provenance_graph", "root_sha256"
    }
    if set(root_value) != expected_fields:
        _fail("evidence root fields are not the closed v1 set")
    if root_value["schema_id"] != SCHEMA_ID or root_value["schema_version"] != SCHEMA_VERSION:
        _fail("evidence root schema identity mismatch")
    _validate_graph(root_value["provenance_graph"])
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
