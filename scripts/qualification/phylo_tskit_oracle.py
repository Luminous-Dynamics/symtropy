#!/usr/bin/env python3
# Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Qualification-only differential oracle for modeled-locus ancestry simplification."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

EXPECTED_TSKIT_VERSION = "1.0.3"


class OracleError(RuntimeError):
    pass


def _require_tskit():
    try:
        import tskit  # type: ignore
    except ImportError as exc:
        raise OracleError(
            "tskit is required for this qualification oracle; "
            "install scripts/qualification/phylo-tskit-requirements.txt"
        ) from exc
    if tskit.__version__ != EXPECTED_TSKIT_VERSION:
        raise OracleError(
            f"qualification requires tskit=={EXPECTED_TSKIT_VERSION}; "
            f"found {tskit.__version__}"
        )
    return tskit


def _validate_case(case: dict) -> None:
    required = {"name", "loci", "nodes", "edges", "focal", "protected", "expected"}
    missing = required - set(case)
    if missing:
        raise OracleError(f"{case.get('name', '<unnamed>')}: missing {sorted(missing)}")
    loci = case["loci"]
    if not loci or len(set(loci)) != len(loci):
        raise OracleError(f"{case['name']}: loci must be non-empty and unique")
    node_ids = [node["id"] for node in case["nodes"]]
    if not node_ids or len(set(node_ids)) != len(node_ids):
        raise OracleError(f"{case['name']}: node ids must be non-empty and unique")
    known = set(node_ids)
    for group in ("focal", "protected"):
        values = case[group]
        if len(set(values)) != len(values):
            raise OracleError(f"{case['name']}: duplicate {group} ids")
        unknown = set(values) - known
        if unknown:
            raise OracleError(f"{case['name']}: unknown {group} ids {sorted(unknown)}")
    if set(case["focal"]) & set(case["protected"]):
        raise OracleError(f"{case['name']}: focal/protected sets must be disjoint")
    if not case["focal"]:
        raise OracleError(f"{case['name']}: at least one focal copy is required")
    generations = {node["id"]: int(node["generation"]) for node in case["nodes"]}
    seen_child_locus = set()
    for edge in case["edges"]:
        if edge["source"] not in known or edge["child"] not in known:
            raise OracleError(f"{case['name']}: edge references unknown node")
        if edge["locus"] not in loci:
            raise OracleError(f"{case['name']}: edge references unknown locus")
        if generations[edge["source"]] >= generations[edge["child"]]:
            raise OracleError(f"{case['name']}: ancestry must move to later generations")
        key = (edge["child"], edge["locus"])
        if key in seen_child_locus:
            raise OracleError(f"{case['name']}: duplicate incoming ancestry {key}")
        seen_child_locus.add(key)


def _build_tree_sequence(case: dict, tskit):
    max_generation = max(int(node["generation"]) for node in case["nodes"])
    tables = tskit.TableCollection(sequence_length=float(len(case["loci"])))
    old_row_by_copy = {}
    copy_by_old_row = {}
    for node in case["nodes"]:
        copy_id = node["id"]
        row = tables.nodes.add_row(time=float(max_generation - int(node["generation"])))
        old_row_by_copy[copy_id] = row
        copy_by_old_row[row] = copy_id

    interval_by_locus = {
        locus: (float(index), float(index + 1))
        for index, locus in enumerate(case["loci"])
    }
    for edge in case["edges"]:
        left, right = interval_by_locus[edge["locus"]]
        tables.edges.add_row(
            left=left,
            right=right,
            parent=old_row_by_copy[edge["source"]],
            child=old_row_by_copy[edge["child"]],
        )
    tables.sort()
    return tables.tree_sequence(), old_row_by_copy, copy_by_old_row, interval_by_locus


def _simplify(case: dict, tskit):
    ts, old_row_by_copy, copy_by_old_row, interval_by_locus = _build_tree_sequence(
        case, tskit
    )
    samples = [old_row_by_copy[copy_id] for copy_id in case["focal"] + case["protected"]]
    simplified, node_map = ts.simplify(
        samples=samples,
        map_nodes=True,
        keep_input_roots=True,
        keep_unary=False,
        record_provenance=False,
    )
    copy_by_new_row = {}
    for old_row, copy_id in copy_by_old_row.items():
        new_row = int(node_map[old_row])
        if new_row != tskit.NULL:
            copy_by_new_row[new_row] = copy_id
    return simplified, node_map, old_row_by_copy, copy_by_new_row, interval_by_locus


def _edge_topology(ts, copy_by_new_row: dict, interval_by_locus: dict):
    topology = set()
    for locus, (left, right) in interval_by_locus.items():
        tree = ts.at((left + right) / 2.0)
        for child_row, child_copy in copy_by_new_row.items():
            parent_row = tree.parent(child_row)
            if parent_row != -1:
                parent_copy = copy_by_new_row.get(parent_row)
                if parent_copy is None:
                    raise OracleError(
                        f"tskit retained parent row {parent_row} without persistent-id mapping"
                    )
                topology.add((locus, parent_copy, child_copy))
    return topology


def _run_case(case: dict, tskit) -> dict:
    _validate_case(case)
    ts, node_map, old_row_by_copy, copy_by_new_row, intervals = _simplify(case, tskit)
    expected = case["expected"]
    retained = set(copy_by_new_row.values())
    expected_retained = set(expected["retained"])
    if retained != expected_retained:
        raise OracleError(
            f"{case['name']}: retained mismatch: "
            f"oracle={sorted(retained)} expected={sorted(expected_retained)}"
        )

    for copy_id in expected.get("removed", []):
        if int(node_map[old_row_by_copy[copy_id]]) != tskit.NULL:
            raise OracleError(f"{case['name']}: expected removed copy {copy_id} survived")

    actual_edges = _edge_topology(ts, copy_by_new_row, intervals)
    expected_edges = {
        (edge["locus"], edge["source"], edge["child"])
        for edge in expected["edges"]
    }
    if actual_edges != expected_edges:
        missing = sorted(expected_edges - actual_edges)
        extra = sorted(actual_edges - expected_edges)
        raise OracleError(
            f"{case['name']}: topology mismatch; missing={missing} extra={extra}"
        )

    return {
        "name": case["name"],
        "retained": sorted(retained),
        "edges": [
            {"locus": locus, "source": source, "child": child}
            for locus, source, child in sorted(actual_edges)
        ],
        "tskit_nodes": ts.num_nodes,
        "tskit_edges": ts.num_edges,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "fixture",
        nargs="?",
        type=Path,
        default=Path("scripts/qualification/phylo-tskit-v1.json"),
    )
    parser.add_argument("--json-output", type=Path)
    args = parser.parse_args()

    tskit = _require_tskit()
    payload = json.loads(args.fixture.read_text(encoding="utf-8"))
    if payload.get("fixture_version") != 1:
        raise OracleError("unsupported fixture_version")
    results = [_run_case(case, tskit) for case in payload["cases"]]
    report = {
        "oracle": "tskit",
        "tskit_version": tskit.__version__,
        "fixture_version": payload["fixture_version"],
        "case_count": len(results),
        "cases": results,
    }
    encoded = json.dumps(report, indent=2, sort_keys=True)
    if args.json_output:
        args.json_output.write_text(encoded + "\n", encoding="utf-8")
    print(encoded)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except OracleError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
