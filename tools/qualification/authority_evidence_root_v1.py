#!/usr/bin/env python3
"""Build and independently verify the recursive QUAL-001B evidence root."""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

from tools.qualification import authority_dispatch_evidence_v1 as dispatch_evidence
from tools.qualification import authority_dispatch_v1 as dispatch
from tools.qualification import authority_evidence_envelope_v1 as envelope
from tools.qualification import authority_evidence_manifest_v1 as manifest
from tools.qualification import qualification_contract_v1 as contract_v1

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
GRAPH_RELATIONSHIP_CONTRACTS = (
    {
        "edge": ("dispatch_payload", "binds", "qualification_contract"),
        "checks": (
            (("value", "dispatch_payload", "contract_commit_sha"),
             ("graph", "contract_identity", "commit_sha")),
            (("value", "dispatch_payload", "contract_commit_sha"),
             ("value", "qualification_contract", "subject_head_sha")),
        ),
    },
    {
        "edge": ("dispatch_payload", "selects", "verifier_release"),
        "checks": (
            (("value", "dispatch_evidence", "release_sha256"),
             ("member_sha256", "verifier_release")),
        ),
    },
    {
        "edge": ("qualification_contract_checkout_identity", "materializes", "qualification_contract"),
        "checks": (
            (("value", "qualification_contract_checkout_identity", "contract_blob_sha"),
             ("git_blob_sha", "qualification_contract")),
        ),
    },
    {
        "edge": ("execution_evidence", "uses", "qualification_contract"),
        "checks": (
            (("value", "execution_evidence", "contract_commit_sha"),
             ("graph", "contract_identity", "commit_sha")),
        ),
    },
    {
        "edge": ("execution_evidence", "targets", "subject_identity"),
        "checks": (
            (("value", "execution_evidence", "subject_head_sha"),
             ("graph", "subject_identity", "head_sha")),
            (("value", "execution_evidence", "subject_head_sha"),
             ("value", "qualification_contract", "subject_head_sha")),
        ),
    },
    {
        "edge": ("execution_evidence", "produces", "final_result"),
        "checks": (
            (("value", "execution_evidence", "final_result"),
             ("graph", "final_result", "value")),
        ),
    },
)
GRAPH_EDGES = tuple(spec["edge"] for spec in GRAPH_RELATIONSHIP_CONTRACTS)

RELATIONSHIP_OPERAND_KINDS = {"value", "member_sha256", "git_blob_sha", "graph"}

# The relationship catalog must consume the same field vocabularies that validate
# the retained evidence. This prevents the root verifier from growing a second,
# manually maintained notion of what an evidence member contains.
RELATIONSHIP_VALUE_FIELDS = {
    "dispatch_payload": frozenset(dispatch.FIELDS),
    "dispatch_evidence": frozenset(dispatch_evidence.FIELDS),
    "qualification_contract": frozenset(contract_v1.FIELDS),
    "qualification_contract_checkout_identity": frozenset(envelope.REQUIRED_CHECKOUT_FIELDS),
    "execution_evidence": frozenset(envelope.REQUIRED_EXECUTION_FIELDS),
}
GRAPH_OPERAND_FIELDS = {
    "contract_identity": frozenset({"commit_sha", "tree_sha", "blob_sha"}),
    "subject_identity": frozenset({"head_sha", "tree_sha"}),
    "final_result": frozenset({"value"}),
}

# One semantic-binding catalog is the policy source for the recursive root.
# It records the authoritative evidence source, graph projection, enforcing
# relationship(s), and the invariant each binding is intended to preserve.
# Derived compatibility maps below are projections, not independent policy.
SEMANTIC_BINDINGS = {
    "contract_commit_sha": {
        "source": ("qualification_contract_checkout_identity", "contract_commit_sha"),
        "graph": ("contract_identity", "commit_sha"),
        "coverage_class": "enforced",
        "relationship_operand_kind": "value",
        "enforced_by": (
            ("dispatch_payload", "binds", "qualification_contract"),
            ("execution_evidence", "uses", "qualification_contract"),
        ),
        "enforcement_checks": (
            (("dispatch_payload", "binds", "qualification_contract"),
             (("value", "dispatch_payload", "contract_commit_sha"),
              ("graph", "contract_identity", "commit_sha"))),
            (("execution_evidence", "uses", "qualification_contract"),
             (("value", "execution_evidence", "contract_commit_sha"),
              ("graph", "contract_identity", "commit_sha"))),
        ),
        "purpose": "bind qualification dispatch and execution to one frozen contract commit",
    },
    "verifier_release_sha256": {
        "source": ("dispatch_evidence", "release_sha256"),
        "graph": ("verifier_release", "sha256"),
        "coverage_class": "enforced",
        "relationship_operand_kind": "member_sha256",
        "enforced_by": (
            ("dispatch_payload", "selects", "verifier_release"),
        ),
        "enforcement_checks": (
            (("dispatch_payload", "selects", "verifier_release"),
             (("value", "dispatch_evidence", "release_sha256"),
              ("member_sha256", "verifier_release"))),
        ),
        "purpose": "bind dispatch selection to the exact retained verifier release member",
    },
    "contract_tree_sha": {
        "source": ("qualification_contract_checkout_identity", "contract_tree_sha"),
        "graph": ("contract_identity", "tree_sha"),
        "coverage_class": "retained",
        "relationship_operand_kind": None,
        "enforced_by": (),
        "enforcement_checks": (),
        "purpose": "retain the exact checkout tree identity of the frozen contract",
    },
    "contract_blob_sha": {
        "source": ("qualification_contract_checkout_identity", "contract_blob_sha"),
        "graph": ("contract_identity", "blob_sha"),
        "coverage_class": "enforced",
        "relationship_operand_kind": "git_blob_sha",
        "enforced_by": (
            ("qualification_contract_checkout_identity", "materializes", "qualification_contract"),
        ),
        "enforcement_checks": (
            (("qualification_contract_checkout_identity", "materializes", "qualification_contract"),
             (("value", "qualification_contract_checkout_identity", "contract_blob_sha"),
              ("git_blob_sha", "qualification_contract"))),
        ),
        "purpose": "bind the checkout identity to the exact retained contract bytes",
    },
    "subject_head_sha": {
        "source": ("execution_evidence", "subject_head_sha"),
        "graph": ("subject_identity", "head_sha"),
        "coverage_class": "enforced",
        "relationship_operand_kind": "value",
        "enforced_by": (
            ("execution_evidence", "targets", "subject_identity"),
        ),
        "enforcement_checks": (
            (("execution_evidence", "targets", "subject_identity"),
             (("value", "execution_evidence", "subject_head_sha"),
              ("graph", "subject_identity", "head_sha"))),
        ),
        "purpose": "bind execution evidence to one immutable subject revision",
    },
    "contract_subject_head_sha": {
        "source": ("qualification_contract", "subject_head_sha"),
        "graph": ("subject_identity", "head_sha"),
        "coverage_class": "enforced",
        "relationship_operand_kind": "value",
        "enforced_by": (
            ("dispatch_payload", "binds", "qualification_contract"),
        ),
        "enforcement_checks": (
            (("dispatch_payload", "binds", "qualification_contract"),
             (("value", "dispatch_payload", "contract_commit_sha"),
              ("value", "qualification_contract", "subject_head_sha"))),
        ),
        "purpose": "bind the qualification contract to the dispatched subject revision",
    },
    "subject_tree_sha": {
        "source": ("execution_evidence", "subject_tree_sha"),
        "graph": ("subject_identity", "tree_sha"),
        "relationship_operand_kind": None,
        "enforced_by": (),
        "enforcement_checks": (),
        "purpose": "retain the subject checkout tree identity",
    },
    "final_result": {
        "source": ("execution_evidence", "final_result"),
        "graph": ("final_result", "value"),
        "coverage_class": "enforced",
        "relationship_operand_kind": "value",
        "enforced_by": (
            ("execution_evidence", "produces", "final_result"),
        ),
        "enforcement_checks": (
            (("execution_evidence", "produces", "final_result"),
             (("value", "execution_evidence", "final_result"),
              ("graph", "final_result", "value"))),
        ),
        "purpose": "bind the observed qualification result to retained execution evidence",
    },
}
SEMANTIC_BINDING_SOURCES = {
    binding: spec["source"] for binding, spec in SEMANTIC_BINDINGS.items()
}
GRAPH_SEMANTIC_FIELDS = {
    spec["graph"]: spec["source"] for spec in SEMANTIC_BINDINGS.values()
}

# Closed inventory of evidence fields that carry semantic/security meaning across
# the recursive provenance boundary. Every inventory entry must have exactly one
# semantic binding; every semantic binding must therefore have an inventory owner.
SECURITY_SURFACE_FIELDS = frozenset({
    ("qualification_contract_checkout_identity", "contract_commit_sha"),
    ("dispatch_evidence", "release_sha256"),
    ("qualification_contract_checkout_identity", "contract_tree_sha"),
    ("qualification_contract_checkout_identity", "contract_blob_sha"),
    ("execution_evidence", "subject_head_sha"),
    ("qualification_contract", "subject_head_sha"),
    ("execution_evidence", "subject_tree_sha"),
    ("execution_evidence", "final_result"),
})

def _validate_relationship_contract_catalog() -> None:
    if tuple(spec.get("edge") for spec in GRAPH_RELATIONSHIP_CONTRACTS) != GRAPH_EDGES:
        _fail("relationship contract edge projection mismatch")
    if len(GRAPH_RELATIONSHIP_CONTRACTS) != len(GRAPH_EDGES) or len(GRAPH_EDGES) != len(set(GRAPH_EDGES)):
        _fail("relationship contract edges must be unique")
    for spec in GRAPH_RELATIONSHIP_CONTRACTS:
        if set(spec) != {"edge", "checks"}:
            _fail("relationship contract fields are not the closed v1 set")
        edge = spec["edge"]
        if not isinstance(edge, tuple) or len(edge) != 3:
            _fail("relationship contract edge must be a 3-tuple")
        if edge[0] not in GRAPH_NODE_IDS or edge[2] not in GRAPH_NODE_IDS:
            _fail("relationship contract edge references an unknown graph node")
        if not isinstance(edge[1], str) or not edge[1]:
            _fail("relationship contract edge type must be non-empty")
        checks = spec["checks"]
        if not isinstance(checks, tuple) or not checks:
            _fail("relationship contract edge must have at least one check")
        for pair in checks:
            if not isinstance(pair, tuple) or len(pair) != 2:
                _fail("relationship contract check must be a pair")
            for operand in pair:
                if not isinstance(operand, tuple) or len(operand) not in (2, 3):
                    _fail("relationship contract operand must have two or three elements")
                kind, target, *field = operand
                if kind not in RELATIONSHIP_OPERAND_KINDS:
                    _fail(f"unknown relationship operand kind: {kind}")
                if kind == "value":
                    if target not in RELATIONSHIP_VALUE_FIELDS or len(field) != 1 or field[0] not in RELATIONSHIP_VALUE_FIELDS[target]:
                        _fail(f"relationship value operand references an undeclared field: {target}.{field[0] if field else '<missing>'}")
                elif kind == "graph":
                    if target not in GRAPH_OPERAND_FIELDS or len(field) != 1 or field[0] not in GRAPH_OPERAND_FIELDS[target]:
                        _fail(f"relationship graph operand references an undeclared field: {target}.{field[0] if field else '<missing>'}")
                elif field:
                    _fail(f"relationship {kind} operand cannot name a field")

    declared_edges = set(GRAPH_EDGES)
    for binding, spec in SEMANTIC_BINDINGS.items():
        if set(spec) != {"source", "graph", "coverage_class", "relationship_operand_kind", "enforced_by", "enforcement_checks", "purpose"}:
            _fail(f"semantic binding {binding} has a non-canonical field set")
        if not isinstance(binding, str) or not binding:
            _fail("semantic binding id must be non-empty")
        source = spec["source"]
        graph_field = spec["graph"]
        enforced_by = spec["enforced_by"]
        enforcement_checks = spec["enforcement_checks"]
        coverage_class = spec["coverage_class"]
        operand_kind = spec["relationship_operand_kind"]
        purpose = spec["purpose"]
        if coverage_class not in {"enforced", "retained"}:
            _fail(f"semantic binding {binding} has an invalid coverage class")
        if operand_kind not in {None, "value", "member_sha256", "git_blob_sha", "graph"}:
            _fail(f"semantic binding {binding} has an invalid relationship operand kind")
        if not isinstance(source, tuple) or len(source) != 2:
            _fail(f"semantic binding {binding} source must be a (role, field) tuple")
        if not isinstance(graph_field, tuple) or len(graph_field) != 2:
            _fail(f"semantic binding {binding} graph projection must be a (node, field) tuple")
        if not isinstance(enforced_by, tuple):
            _fail(f"semantic binding {binding} enforced_by must be a tuple")
        if coverage_class == "enforced" and not enforced_by:
            _fail(f"semantic binding {binding} is enforced but has no enforcing relationship")
        if coverage_class == "retained" and enforced_by:
            _fail(f"semantic binding {binding} is retained but declares enforcing relationships")
        if operand_kind is not None and not enforced_by:
            _fail(f"semantic binding {binding} declares an operand kind without enforcement")
        if not isinstance(enforcement_checks, tuple):
            _fail(f"semantic binding {binding} enforcement_checks must be a tuple")
        if len(enforcement_checks) != len(enforced_by):
            _fail(f"semantic binding {binding} enforcement anchors must cover every enforcing relationship")
        if not isinstance(purpose, str) or not purpose:
            _fail(f"semantic binding {binding} purpose must be non-empty")
        for edge in enforced_by:
            if edge not in declared_edges:
                _fail(f"semantic binding {binding} references a missing relationship: {edge}")
        for check_edge, check_pair in enforcement_checks:
            if check_edge not in enforced_by:
                _fail(f"semantic binding {binding} check references a non-enforcing relationship: {check_edge}")
            matching = next(spec for spec in GRAPH_RELATIONSHIP_CONTRACTS if spec["edge"] == check_edge)
            if check_pair not in matching["checks"]:
                _fail(
                    f"semantic binding {binding} relationship {check_edge} "
                    f"does not contain its declared enforcement check"
                )
            if operand_kind is not None and not any(operand[0] == operand_kind for operand in check_pair):
                _fail(
                    f"semantic binding {binding} enforcement check does not contain "
                    f"the declared {operand_kind} operand kind"
                )
            source_role, source_field = source
            source_operand = ("value", source_role, source_field)
            graph_operand = ("graph", graph_field[0], graph_field[1])
            if source_operand not in check_pair and graph_operand not in check_pair:
                _fail(
                    f"semantic binding {binding} enforcement check does not directly "
                    f"reference its source or graph projection"
                )
    required_binding_sources = {
        spec["source"] for spec in SEMANTIC_BINDINGS.values()
    }
    if required_binding_sources != SECURITY_SURFACE_FIELDS:
        _fail("semantic bindings do not exactly cover the declared security surface")
    if set(SEMANTIC_BINDING_SOURCES.values()) != required_binding_sources:
        _fail("semantic binding source projection is not canonical")
    required_graph_bindings = {
        spec["graph"]: spec["source"] for spec in SEMANTIC_BINDINGS.values()
    }
    if GRAPH_SEMANTIC_FIELDS != required_graph_bindings:
        _fail("semantic graph binding projection is not canonical")

    classified_checks = {
        check_pair
        for spec in SEMANTIC_BINDINGS.values()
        for _, check_pair in spec["enforcement_checks"]
    }
    all_checks = {
        check_pair
        for spec in GRAPH_RELATIONSHIP_CONTRACTS
        for check_pair in spec["checks"]
    }
    if len(classified_checks) != sum(len(spec["enforcement_checks"]) for spec in SEMANTIC_BINDINGS.values()):
        _fail("relationship checks are classified more than once")
    if classified_checks != all_checks:
        _fail("relationship checks are not exactly covered by semantic bindings")

    declared_graph_fields = {
        (operand[1], operand[2])
        for spec in GRAPH_RELATIONSHIP_CONTRACTS
        for pair in spec["checks"]
        for operand in pair
        if operand[0] == "graph"
    }
    required_graph_fields = set(GRAPH_SEMANTIC_FIELDS)
    if declared_graph_fields != required_graph_fields:
        _fail("relationship contract graph-field coverage is not canonical")

    for binding, source in SEMANTIC_BINDING_SOURCES.items():
        target, field = source
        if target not in RELATIONSHIP_VALUE_FIELDS or field not in RELATIONSHIP_VALUE_FIELDS[target]:
            _fail(f"semantic binding {binding} references an undeclared evidence field: {target}.{field}")

    for graph_field, source in GRAPH_SEMANTIC_FIELDS.items():
        target, field = graph_field
        source_target, source_field = source
        if target not in GRAPH_OPERAND_FIELDS or field not in GRAPH_OPERAND_FIELDS[target]:
            _fail(f"semantic graph binding references an undeclared graph field: {target}.{field}")
        if source_target not in RELATIONSHIP_VALUE_FIELDS or source_field not in RELATIONSHIP_VALUE_FIELDS[source_target]:
            _fail(f"semantic graph binding references an undeclared evidence field: {source_target}.{source_field}")



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


def _git_blob_sha(raw: bytes) -> str:
    return hashlib.sha1(
        b"blob " + str(len(raw)).encode("ascii") + b"\0" + raw
    ).hexdigest()


def _resolve_relationship_operand(operand, values, raw_members, members, graph):
    kind, target, *field = operand
    if kind == "value":
        if len(field) != 1:
            _fail("relationship value operand must name exactly one field")
        return values[target][field[0]]
    if kind == "member_sha256":
        if field:
            _fail("relationship member digest operand cannot name a field")
        return members[target]
    if kind == "git_blob_sha":
        if field:
            _fail("relationship Git blob operand cannot name a field")
        return _git_blob_sha(raw_members[target])
    if kind == "graph":
        if len(field) != 1:
            _fail("relationship graph operand must name exactly one field")
        node = next((n for n in graph["nodes"] if n["id"] == target), None)
        if node is None:
            _fail(f"relationship graph node is missing: {target}")
        return node[field[0]]
    _fail(f"unknown relationship operand kind: {kind}")


def _validate_relationship_contracts(values, raw_members, members, graph):
    _validate_relationship_contract_catalog()
    for spec in GRAPH_RELATIONSHIP_CONTRACTS:
        for left, right in spec["checks"]:
            if _resolve_relationship_operand(left, values, raw_members, members, graph) != _resolve_relationship_operand(right, values, raw_members, members, graph):
                source, edge_type, target = spec["edge"]
                _fail(f"graph relationship contract failed for {source} --{edge_type}--> {target}")


def _build_graph(values: dict, raw_members: dict, members: dict) -> dict:
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
    graph = {"nodes": nodes, "edges": edges}
    _validate_relationship_contracts(values, raw_members, members, graph)
    return graph


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


def _semantic_security_surface() -> dict:
    # This is a sealed root projection, not a second policy source. The canonical
    # policy remains SEMANTIC_BINDINGS; the projection makes the closed security
    # surface independently inspectable from the emitted root.
    return {
        binding: {
            "source": {
                "role": source[0],
                "field": source[1],
            },
            "coverage_class": SEMANTIC_BINDINGS[binding]["coverage_class"],
        }
        for binding, source in SEMANTIC_BINDING_SOURCES.items()
    }


def _build_body(directory: Path, manifest_value: dict, manifest_sha256: str) -> dict:
    members = {}
    raw_members = {}
    values = {}
    for name, role in MEMBER_ROLES:
        raw, value = load_member(directory, name)
        members[role] = sha256(raw)
        raw_members[role] = raw
        values[role] = value

    checkout = values["qualification_contract_checkout_identity"]
    contract = values["qualification_contract"]
    execution = values["execution_evidence"]

    if checkout["contract_commit_sha"] != contract.get("subject_head_sha") and checkout["contract_commit_sha"] != values["dispatch_payload"]["contract_commit_sha"]:
        _fail("checkout identity is not bound to the dispatched contract commit")

    contract_raw = raw_members["qualification_contract"]
    blob = _git_blob_sha(contract_raw)
    if checkout["contract_blob_sha"] != blob:
        _fail("checkout identity blob does not match retained contract bytes")

    if execution["final_result"] not in FINAL_RESULTS:
        _fail("execution final_result is invalid")

    graph = _build_graph(values, raw_members, members)
    _validate_graph(graph)

    return {
        "schema_id": SCHEMA_ID,
        "schema_version": SCHEMA_VERSION,
        "manifest_sha256": manifest_sha256,
        "manifest_root_sha256": manifest_value["root_sha256"],
        "member_sha256": members,
        "semantic_bindings": {
            binding: values[source_role][source_field]
            for binding, (source_role, source_field) in SEMANTIC_BINDING_SOURCES.items()
        },
        "provenance_graph": graph,
        "semantic_binding_policy": {
            binding: SEMANTIC_BINDINGS[binding]["coverage_class"]
            for binding in SEMANTIC_BINDINGS
        },
        "semantic_security_surface": _semantic_security_surface(),
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
        "member_sha256", "semantic_bindings", "provenance_graph",
        "semantic_binding_policy", "root_sha256"
    }
    if set(root_value) != expected_fields:
        _fail("evidence root fields are not the closed v1 set")
    if root_value["schema_id"] != SCHEMA_ID or root_value["schema_version"] != SCHEMA_VERSION:
        _fail("evidence root schema identity mismatch")
    _validate_graph(root_value["provenance_graph"])
    if root_value["semantic_binding_policy"] != {
        binding: spec["coverage_class"] for binding, spec in SEMANTIC_BINDINGS.items()
    }:
        _fail("evidence root semantic binding policy does not match the retained policy")
    expected_security_surface = _semantic_security_surface()
    if root_value["semantic_security_surface"] != expected_security_surface:
        _fail("evidence root semantic security surface does not match the retained policy")
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
