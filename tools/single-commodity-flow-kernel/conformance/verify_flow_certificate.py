#!/usr/bin/env python3
"""Independent standard-library verifier for FlowCertificateBundle v1.

Research/conformance code only. It does not call the Rust solver or Rust
verifier, has no third-party dependencies, and is not a production service.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import struct
import sys
from pathlib import Path
from typing import Any

SCHEMA = "luminous-flow-certificate-v1"
KERNEL = "single-commodity-directed-flow-v0.1.0"
MODEL = "single_commodity_directed_integer_capacity"
MAGIC = b"LUMINOUS-SCFK-GRAPH-V1\x00"
U64_MAX = (1 << 64) - 1


class CertificateError(ValueError):
    """Raised when a portable certificate fails strict independent checks."""


def _fail(message: str) -> None:
    raise CertificateError(message)


def _object(value: Any, required: set[str], path: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        _fail(path + " must be an object")
    actual = set(value)
    missing = required - actual
    unknown = actual - required
    if missing:
        _fail(path + " missing keys: " + ", ".join(sorted(missing)))
    if unknown:
        _fail(path + " unknown keys: " + ", ".join(sorted(unknown)))
    return value


def _string(value: Any, path: str) -> str:
    if not isinstance(value, str) or not value.strip():
        _fail(path + " must be a non-empty string")
    return value


def _uint64(value: Any, path: str) -> int:
    # bool is a subclass of int in Python, so require the exact type.
    if type(value) is not int or value < 0 or value > U64_MAX:
        _fail(path + " must be an exact uint64 integer token")
    return value


def _optional_string(value: Any, path: str) -> str | None:
    if value is None:
        return None
    return _string(value, path)


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            _fail("duplicate JSON object key: " + key)
        result[key] = value
    return result


def _reject_float(value: str) -> None:
    _fail("floating-point JSON numbers are forbidden in the certificate: " + value)


def _reject_constant(value: str) -> None:
    _fail("non-standard JSON numeric constant is forbidden: " + value)


def load_json_strict(text: str) -> Any:
    try:
        return json.loads(
            text,
            object_pairs_hook=_reject_duplicate_pairs,
            parse_float=_reject_float,
            parse_constant=_reject_constant,
        )
    except CertificateError:
        raise
    except (json.JSONDecodeError, UnicodeError) as exc:
        raise CertificateError("invalid JSON: " + str(exc)) from exc


def _put_string(buffer: bytearray, value: str) -> None:
    encoded = value.encode("utf-8")
    buffer.extend(struct.pack("<Q", len(encoded)))
    buffer.extend(encoded)


def canonical_graph_bytes(graph: dict[str, Any]) -> bytes:
    required = {
        "commodity_ref",
        "capacity_unit_ref",
        "quantity_boundary_ref",
        "nodes",
        "edges",
        "source",
        "sink",
    }
    _object(graph, required, "graph")

    commodity = _string(graph["commodity_ref"], "graph.commodity_ref")
    unit = _string(graph["capacity_unit_ref"], "graph.capacity_unit_ref")
    boundary = _string(graph["quantity_boundary_ref"], "graph.quantity_boundary_ref")
    source = _string(graph["source"], "graph.source")
    sink = _string(graph["sink"], "graph.sink")
    if source == sink:
        _fail("graph source and sink must differ")

    nodes = graph["nodes"]
    edges = graph["edges"]
    if not isinstance(nodes, list) or not 2 <= len(nodes) <= 4096:
        _fail("graph.nodes count must be within 2..=4096")
    if not isinstance(edges, list) or len(edges) > 65536:
        _fail("graph.edges count must be at most 65536")

    node_ids: list[str] = []
    for index, item in enumerate(nodes):
        _object(item, {"id"}, "graph.nodes[" + str(index) + "]")
        node_ids.append(_string(item["id"], "graph.nodes.id"))
    if len(set(node_ids)) != len(node_ids):
        _fail("graph contains duplicate node IDs")
    node_set = set(node_ids)
    if source not in node_set or sink not in node_set:
        _fail("graph source or sink is absent from graph.nodes")

    edge_ids: set[str] = set()
    checked_edges: list[dict[str, Any]] = []
    for index, item in enumerate(edges):
        path = "graph.edges[" + str(index) + "]"
        _object(item, {"id", "from", "to", "capacity", "metadata_ref"}, path)
        edge_id = _string(item["id"], path + ".id")
        tail = _string(item["from"], path + ".from")
        head = _string(item["to"], path + ".to")
        capacity = _uint64(item["capacity"], path + ".capacity")
        metadata = _optional_string(item["metadata_ref"], path + ".metadata_ref")
        if edge_id in edge_ids:
            _fail("graph contains duplicate edge ID " + edge_id)
        edge_ids.add(edge_id)
        if tail == head:
            _fail("self-loop edge is outside the supported profile: " + edge_id)
        if tail not in node_set or head not in node_set:
            _fail("edge has a dangling endpoint: " + edge_id)
        checked_edges.append({
            "id": edge_id,
            "from": tail,
            "to": head,
            "capacity": capacity,
            "metadata_ref": metadata,
        })

    buffer = bytearray(MAGIC)
    for value in (commodity, unit, boundary, source, sink):
        _put_string(buffer, value)

    sorted_nodes = sorted(node_ids, key=lambda value: value.encode("utf-8"))
    buffer.extend(struct.pack("<Q", len(sorted_nodes)))
    for node_id in sorted_nodes:
        _put_string(buffer, node_id)

    sorted_edges = sorted(checked_edges, key=lambda edge: edge["id"].encode("utf-8"))
    buffer.extend(struct.pack("<Q", len(sorted_edges)))
    for edge in sorted_edges:
        _put_string(buffer, edge["id"])
        _put_string(buffer, edge["from"])
        _put_string(buffer, edge["to"])
        buffer.extend(struct.pack("<Q", edge["capacity"]))
        if edge["metadata_ref"] is None:
            buffer.extend(b"\x00")
        else:
            buffer.extend(b"\x01")
            _put_string(buffer, edge["metadata_ref"])
    return bytes(buffer)


def graph_digest(graph: dict[str, Any]) -> str:
    return "sha256:" + hashlib.sha256(canonical_graph_bytes(graph)).hexdigest()


def verify_bundle(bundle: Any) -> dict[str, Any]:
    _object(bundle, {"schema_version", "graph", "result"}, "bundle")
    if bundle["schema_version"] != SCHEMA:
        _fail("unsupported bundle schema version")

    graph = bundle["graph"]
    result = bundle["result"]
    graph_bytes = canonical_graph_bytes(graph)
    _object(
        result,
        {
            "certificate_schema",
            "graph_sha256",
            "commodity_ref",
            "capacity_unit_ref",
            "quantity_boundary_ref",
            "kernel_version",
            "supported_model_class",
            "source",
            "sink",
            "max_flow",
            "edge_flows",
            "source_side_nodes",
            "min_cut",
        },
        "result",
    )
    if result["certificate_schema"] != SCHEMA:
        _fail("unsupported result certificate schema")
    if result["kernel_version"] != KERNEL:
        _fail("unsupported flow kernel version")
    if result["supported_model_class"] != MODEL:
        _fail("unsupported flow model class")

    expected_digest = "sha256:" + hashlib.sha256(graph_bytes).hexdigest()
    if result["graph_sha256"] != expected_digest:
        _fail("graph SHA-256 does not match the supplied graph")

    for field in ("commodity_ref", "capacity_unit_ref", "quantity_boundary_ref", "source", "sink"):
        if result[field] != graph[field]:
            _fail("result " + field + " does not match graph")

    max_flow = _uint64(result["max_flow"], "result.max_flow")
    edge_by_id = {edge["id"]: edge for edge in graph["edges"]}
    witness_rows = result["edge_flows"]
    if not isinstance(witness_rows, list) or len(witness_rows) != len(edge_by_id):
        _fail("result.edge_flows must contain every original edge exactly once")

    flow_by_id: dict[str, int] = {}
    balance = {node["id"]: 0 for node in graph["nodes"]}
    residual: dict[str, list[str]] = {node["id"]: [] for node in graph["nodes"]}
    for index, witness in enumerate(witness_rows):
        path = "result.edge_flows[" + str(index) + "]"
        _object(witness, {"id", "from", "to", "capacity", "flow", "metadata_ref"}, path)
        edge_id = _string(witness["id"], path + ".id")
        if edge_id in flow_by_id or edge_id not in edge_by_id:
            _fail("edge witness contains duplicate or unknown edge ID " + edge_id)
        edge = edge_by_id[edge_id]
        for field in ("from", "to", "capacity", "metadata_ref"):
            if witness[field] != edge[field]:
                _fail("edge witness identity/capacity mismatch for " + edge_id)
        flow = _uint64(witness["flow"], path + ".flow")
        capacity = edge["capacity"]
        if flow > capacity:
            _fail("edge witness exceeds capacity for " + edge_id)
        flow_by_id[edge_id] = flow
        balance[edge["from"]] += flow
        balance[edge["to"]] -= flow
        if flow < capacity:
            residual[edge["from"]].append(edge["to"])
        if flow > 0:
            residual[edge["to"]].append(edge["from"])

    if set(flow_by_id) != set(edge_by_id):
        _fail("edge witness omits one or more original edges")
    if [row["id"] for row in witness_rows] != sorted(edge_by_id, key=lambda value: value.encode("utf-8")):
        _fail("edge witness rows are not in canonical edge-ID order")

    for node_id, net_outflow in balance.items():
        expected = max_flow if node_id == graph["source"] else (
            -max_flow if node_id == graph["sink"] else 0
        )
        if net_outflow != expected:
            _fail("flow conservation failed at node " + node_id)

    reachable = {graph["source"]}
    queue = [graph["source"]]
    cursor = 0
    while cursor < len(queue):
        node_id = queue[cursor]
        cursor += 1
        for neighbor in residual[node_id]:
            if neighbor not in reachable:
                reachable.add(neighbor)
                queue.append(neighbor)
    expected_source_side = sorted(reachable, key=lambda value: value.encode("utf-8"))
    source_side = result["source_side_nodes"]
    if not isinstance(source_side, list) or source_side != expected_source_side:
        _fail("source-side node set is not the exact residual-reachable partition")

    cut_edges = []
    cut_capacity = 0
    for edge in sorted(graph["edges"], key=lambda item: item["id"].encode("utf-8")):
        if edge["from"] in reachable and edge["to"] not in reachable:
            cut_capacity += edge["capacity"]
            cut_edges.append({
                "id": edge["id"],
                "from": edge["from"],
                "to": edge["to"],
                "capacity": edge["capacity"],
                "metadata_ref": edge["metadata_ref"],
            })

    cut = result["min_cut"]
    _object(cut, {"capacity", "edges"}, "result.min_cut")
    cut_value = _uint64(cut["capacity"], "result.min_cut.capacity")
    if cut["edges"] != cut_edges:
        _fail("minimum-cut edge set does not match residual source partition")
    if cut_value != cut_capacity or cut_value != max_flow:
        _fail("minimum-cut capacity does not equal maximum flow")

    return {
        "valid": True,
        "graph_sha256": expected_digest,
        "maximum_flow": max_flow,
        "minimum_cut": cut_value,
        "nodes": len(graph["nodes"]),
        "edges": len(graph["edges"]),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("bundle", type=Path, help="FlowCertificateBundle JSON file")
    args = parser.parse_args()
    try:
        document = load_json_strict(args.bundle.read_text(encoding="utf-8"))
        report = verify_bundle(document)
    except (OSError, CertificateError) as exc:
        print("INVALID: " + str(exc), file=sys.stderr)
        return 1
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
