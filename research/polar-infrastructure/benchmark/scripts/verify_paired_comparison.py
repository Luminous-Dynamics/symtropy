#!/usr/bin/env python3
"""Independent standard-library verifier for paired-comparison reports.

This recomputes coverage and pairwise outcomes from the input case rows. It does
not import or execute the Rust scorer and does not validate physical semantics.
"""
from __future__ import annotations

import json
import math
import sys
from pathlib import Path
from typing import Any

TOP_KEYS = {
    "schema_version", "artifact_kind", "evidence_class", "status",
    "reference_controller_id", "candidate_controller_id", "controllers",
    "axes", "gate_ids", "cases",
}
CASE_KEYS = {
    "case_id", "controller_id", "trajectory_id", "comparison_group_id",
    "hazard_family", "initial_state_id", "replicate_id", "severity",
    "gate_results", "signed_gate_margins",
}
RESULT_KEYS = {
    "schema_version", "report_kind", "evidence_class", "status",
    "controller_count", "trajectory_count", "case_count", "coverage",
    "paired_gate_comparisons", "performance_claim", "result_scope",
}
STATUSES = {"pass", "breach", "invalid", "not_evaluated"}


def _need(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def _unique(values: list[Any]) -> bool:
    return len(values) == len(set(values))


def _close(left: Any, right: Any, path: str = "$") -> None:
    if isinstance(left, bool) or isinstance(right, bool):
        _need(left is right, f"{path}: boolean mismatch")
    elif isinstance(left, (int, float)) and isinstance(right, (int, float)):
        _need(math.isclose(float(left), float(right), rel_tol=1e-12, abs_tol=1e-12),
              f"{path}: numeric mismatch {left!r} != {right!r}")
    elif isinstance(left, dict) and isinstance(right, dict):
        _need(set(left) == set(right), f"{path}: key mismatch")
        for key in left:
            _close(left[key], right[key], f"{path}.{key}")
    elif isinstance(left, list) and isinstance(right, list):
        _need(len(left) == len(right), f"{path}: length mismatch")
        for index, (a, b) in enumerate(zip(left, right)):
            _close(a, b, f"{path}[{index}]")
    else:
        _need(left == right, f"{path}: mismatch {left!r} != {right!r}")


def validate_input(doc: dict[str, Any]) -> tuple[list[str], list[str], list[str]]:
    _need(set(doc) == TOP_KEYS, "input keys do not match the v0.3 contract")
    _need(doc["schema_version"] == "0.3.0"
          and doc["artifact_kind"] == "paired-resilience-comparison-input"
          and doc["evidence_class"] == "synthetic_fixture"
          and doc["status"] == "synthetic_only",
          "unsupported header/evidence class")
    controllers = doc["controllers"]
    axes = doc["axes"]
    gates = doc["gate_ids"]
    _need(isinstance(controllers, list) and len(controllers) >= 2
          and all(isinstance(x, str) and x.strip() for x in controllers)
          and _unique(controllers), "invalid controller declarations")
    _need(isinstance(axes, list) and axes, "at least one axis is required")
    axis_ids = []
    for axis in axes:
        _need(set(axis) == {"id", "unit", "higher_is_more_severe"}
              and isinstance(axis["id"], str) and axis["id"].strip()
              and isinstance(axis["unit"], str) and axis["unit"].strip()
              and isinstance(axis["higher_is_more_severe"], bool),
              "invalid severity-axis declaration")
        axis_ids.append(axis["id"])
    _need(_unique(axis_ids), "duplicate axis ID")
    _need(isinstance(gates, list) and gates and all(isinstance(g, str) and g.strip() for g in gates)
          and _unique(gates), "invalid gate declarations")
    reference = doc["reference_controller_id"]
    candidate = doc["candidate_controller_id"]
    _need(reference in controllers and candidate in controllers and reference != candidate,
          "reference and candidate must be distinct declared controllers")

    rows: dict[tuple[str, str], dict[str, Any]] = {}
    case_ids: set[str] = set()
    metadata: dict[str, tuple[Any, ...]] = {}
    trajectories: set[str] = set()
    expected_axes, expected_gates = set(axis_ids), set(gates)
    for case in doc["cases"]:
        _need(set(case) == CASE_KEYS, "case keys do not match contract")
        for field in ("case_id", "controller_id", "trajectory_id", "comparison_group_id",
                      "hazard_family", "initial_state_id", "replicate_id"):
            _need(isinstance(case[field], str) and case[field].strip(), f"empty {field}")
        _need(case["case_id"] not in case_ids, "duplicate case ID")
        case_ids.add(case["case_id"])
        _need(case["controller_id"] in controllers, "undeclared controller")
        severity = case["severity"]
        _need(set(severity) == expected_axes, "severity axis set mismatch")
        for value in severity.values():
            _need(isinstance(value, (int, float)) and not isinstance(value, bool)
                  and math.isfinite(value) and value >= 0, "invalid severity")
        _need(set(case["gate_results"]) == expected_gates
              and set(case["signed_gate_margins"]) == expected_gates,
              "gate result/margin key set mismatch")
        for gate in gates:
            result = case["gate_results"][gate]
            margin = case["signed_gate_margins"][gate]
            _need(result in STATUSES, "unknown gate result")
            if result == "pass":
                _need(isinstance(margin, (int, float)) and not isinstance(margin, bool)
                      and math.isfinite(margin) and margin >= 0, "pass needs nonnegative margin")
            elif result == "breach":
                _need(isinstance(margin, (int, float)) and not isinstance(margin, bool)
                      and math.isfinite(margin) and margin < 0, "breach needs negative margin")
            else:
                _need(margin is None, "invalid/not_evaluated margin must be null")
        tid = case["trajectory_id"]
        key = (case["controller_id"], tid)
        _need(key not in rows, "duplicate controller/trajectory result")
        rows[key] = case
        trajectories.add(tid)
        meta = (case["comparison_group_id"], case["hazard_family"], case["initial_state_id"],
                case["replicate_id"], json.dumps(severity, sort_keys=True))
        if tid in metadata:
            _need(metadata[tid] == meta, "paired rows differ in trajectory metadata")
        else:
            metadata[tid] = meta

    _need(bool(trajectories), "empty trajectory set")
    expected_pairs = {(controller, trajectory) for controller in controllers for trajectory in trajectories}
    _need(set(rows) == expected_pairs, "incomplete controller/trajectory matrix")
    return controllers, gates, sorted(trajectories)


def build_expected_report(doc: dict[str, Any]) -> dict[str, Any]:
    controllers, gates, trajectories = validate_input(doc)
    rows = {(c["controller_id"], c["trajectory_id"]): c for c in doc["cases"]}
    coverage = []
    for controller in controllers:
        for gate in gates:
            counts = {s: 0 for s in STATUSES}
            for case in doc["cases"]:
                if case["controller_id"] == controller:
                    counts[case["gate_results"][gate]] += 1
            coverage.append({
                "controller_id": controller, "gate_id": gate,
                "pass_count": counts["pass"], "breach_count": counts["breach"],
                "invalid_count": counts["invalid"], "not_evaluated_count": counts["not_evaluated"],
                "valid_evaluated_count": counts["pass"] + counts["breach"],
            })

    comparisons = []
    reference, candidate = doc["reference_controller_id"], doc["candidate_controller_id"]
    for gate in gates:
        counts = {
            "both_pass": 0, "both_breach": 0,
            "reference_breach_candidate_pass": 0,
            "reference_pass_candidate_breach": 0, "incomplete": 0,
        }
        deltas = []
        for trajectory in trajectories:
            r, c = rows[(reference, trajectory)], rows[(candidate, trajectory)]
            rs, cs = r["gate_results"][gate], c["gate_results"][gate]
            if rs == "pass" and cs == "pass":
                counts["both_pass"] += 1
            elif rs == "breach" and cs == "breach":
                counts["both_breach"] += 1
            elif rs == "breach" and cs == "pass":
                counts["reference_breach_candidate_pass"] += 1
            elif rs == "pass" and cs == "breach":
                counts["reference_pass_candidate_breach"] += 1
            else:
                counts["incomplete"] += 1
            rm, cm = r["signed_gate_margins"][gate], c["signed_gate_margins"][gate]
            if rm is not None and cm is not None:
                deltas.append(float(cm) - float(rm))
        comparisons.append({
            "gate_id": gate,
            "reference_controller_id": reference,
            "candidate_controller_id": candidate,
            "trajectory_pair_count": len(trajectories),
            "both_pass_count": counts["both_pass"],
            "both_breach_count": counts["both_breach"],
            "reference_breach_candidate_pass_count": counts["reference_breach_candidate_pass"],
            "reference_pass_candidate_breach_count": counts["reference_pass_candidate_breach"],
            "incomplete_pair_count": counts["incomplete"],
            "margin_delta_count": len(deltas),
            "mean_candidate_minus_reference_margin": sum(deltas) / len(deltas) if deltas else None,
            "minimum_candidate_minus_reference_margin": min(deltas) if deltas else None,
        })
    return {
        "schema_version": "0.3.0",
        "report_kind": "paired-resilience-comparison-report",
        "evidence_class": "synthetic_fixture",
        "status": "synthetic_only",
        "controller_count": len(controllers),
        "trajectory_count": len(trajectories),
        "case_count": len(doc["cases"]),
        "coverage": coverage,
        "paired_gate_comparisons": comparisons,
        "performance_claim": False,
        "result_scope": "finite synthetic contract fixture only; no physical model, field validation, standards conformity, or controller superiority established",
    }


def verify_documents(input_doc: dict[str, Any], report_doc: dict[str, Any]) -> None:
    expected = build_expected_report(input_doc)
    _need(set(report_doc) == RESULT_KEYS, "report keys do not match v0.3 report contract")
    _close(report_doc, expected)


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print("usage: verify_paired_comparison.py INPUT.json REPORT.json", file=sys.stderr)
        return 2
    try:
        input_doc = json.loads(Path(argv[1]).read_text())
        report_doc = json.loads(Path(argv[2]).read_text())
        verify_documents(input_doc, report_doc)
    except (OSError, json.JSONDecodeError, ValueError) as exc:
        print(f"verification failed: {exc}", file=sys.stderr)
        return 1
    print("independent paired-comparison report verification passed (synthetic contract only)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
