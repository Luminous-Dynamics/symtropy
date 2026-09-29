#!/usr/bin/env python3
"""Deterministic canonical materializer for Symtropy formal provenance graphs v0.

The materializer validates the input with the repository-local semantic validator,
then emits a canonical PKG representation and a SHA-256 digest over that
representation (excluding the digest field itself).
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import PurePosixPath
from typing import Any

from validate import Validator, SCHEMA_VERSION

GRAPH_VERSION = "luminous.formal-pkg.v0"


def _canonical_json(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")


def _normalized_path(path: str) -> str:
    if (
        not isinstance(path, str)
        or not path
        or path.startswith("/")
        or "\x00" in path
        or "//" in path
    ):
        raise ValueError("repository path is not normalized")
    parts = PurePosixPath(path).parts
    if any(part in {".", ".."} for part in parts):
        raise ValueError("repository path is not normalized")
    return path


def _canonical_record(record: dict[str, Any]) -> dict[str, Any]:
    result = json.loads(json.dumps(record, ensure_ascii=False))
    provenance = result.get("provenance")
    if isinstance(provenance, dict) and "path" in provenance:
        provenance["path"] = _normalized_path(provenance["path"])
    for key in ("content_digest",):
        if key in result and result[key] is not None:
            digest = result[key]
            if not isinstance(digest, str) or len(digest) != 64 or digest.lower() != digest:
                raise ValueError(f"{key} must be lowercase SHA-256")
    if isinstance(provenance, dict) and provenance.get("content_digest") is not None:
        digest = provenance["content_digest"]
        if not isinstance(digest, str) or len(digest) != 64 or digest.lower() != digest:
            raise ValueError("provenance.content_digest must be lowercase SHA-256")
    return result


def materialize(graph: dict[str, Any]) -> dict[str, Any]:
    validation = Validator(graph).validate()
    if validation["status"] != "Valid":
        raise ValueError(
            "input provenance graph is not semantically valid: "
            + json.dumps(validation, sort_keys=True, separators=(",", ":"))
        )

    nodes = [_canonical_record(node) for node in graph["nodes"]]
    edges = [_canonical_record(edge) for edge in graph["edges"]]

    nodes.sort(key=lambda node: (node["kind"], node["id"]))
    edges.sort(key=lambda edge: (
        edge["relation"], edge["source"], edge["target"], edge["id"]
    ))

    payload = {
        "schema_version": SCHEMA_VERSION,
        "graph_version": GRAPH_VERSION,
        "node_count": len(nodes),
        "edge_count": len(edges),
        "nodes": nodes,
        "edges": edges,
    }
    digest = hashlib.sha256(_canonical_json(payload)).hexdigest()

    return {**payload, "graph_digest": digest}


def write_canonical(graph: dict[str, Any], path: str) -> None:
    materialized = materialize(graph)
    with open(path, "wb") as handle:
        handle.write(_canonical_json(materialized))
        handle.write(b"\n")


def main() -> int:
    parser = argparse.ArgumentParser(description="Materialize canonical Symtropy PKG v0")
    parser.add_argument("graph", help="input formal-provenance-v0 JSON graph")
    parser.add_argument("--output", required=True, help="canonical PKG output path")
    args = parser.parse_args()

    try:
        with open(args.graph, encoding="utf-8") as handle:
            graph = json.load(handle)
        write_canonical(graph, args.output)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as exc:
        print(json.dumps({
            "schema_version": SCHEMA_VERSION,
            "status": "MaterializationInvalid",
            "error": str(exc),
        }, sort_keys=True, separators=(",", ":")))
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
