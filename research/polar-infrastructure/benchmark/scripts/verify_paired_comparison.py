#!/usr/bin/env python3
"""Independent standard-library verifier for paired-comparison reports.

This recomputes coverage and pairwise outcomes from the input case rows. It does
not import or execute the Rust scorer and does not validate physical semantics.
"""
from __future__ import annotations

import hashlib
import json
import math
import re
import sys
from pathlib import Path
from typing import Any

TOP_KEYS = {
    "schema_version", "artifact_kind", "evidence_class", "status",
    "reference_controller_id", "candidate_controller_id", "controllers",
    "axes", "gate_ids", "gate_margin_contracts", "information_budget_payload",
    "information_budget_sha256", "trajectory_artifacts", "controller_artifacts", "cases",
}
CASE_KEYS = {
    "case_id", "controller_id", "trajectory_id", "comparison_group_id",
    "hazard_family", "initial_state_id", "replicate_id", "severity",
    "gate_results", "signed_gate_margins", "trajectory_sha256",
    "initial_state_sha256", "controller_artifact_sha256", "information_budget_sha256",
}
RESULT_KEYS = {
    "schema_version", "report_kind", "evidence_class", "status",
    "controller_count", "trajectory_count", "case_count", "gate_margin_contracts",
    "information_budget", "content_bindings", "coverage", "paired_gate_comparisons", "performance_claim", "result_scope",
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
    _need(doc["schema_version"] == "0.4.0"
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
    digest_re = re.compile(r"^[a-f0-9]{64}$")

    trajectory_artifacts: dict[str, dict[str, Any]] = {}
    initial_state_digests: dict[str, str] = {}
    for artifact in doc["trajectory_artifacts"]:
        _need(set(artifact) == {"trajectory_id", "initial_state_id", "trajectory_payload",
                                "trajectory_sha256", "initial_state_payload", "initial_state_sha256"},
              "trajectory artifact keys do not match the v0.4 contract")
        for field in ("trajectory_id", "initial_state_id", "trajectory_payload", "initial_state_payload"):
            _need(isinstance(artifact[field], str) and artifact[field], f"empty trajectory artifact field: {field}")
        for field in ("trajectory_sha256", "initial_state_sha256"):
            _need(isinstance(artifact[field], str) and digest_re.fullmatch(artifact[field]),
                  f"invalid artifact digest: {field}")
        trajectory_bytes = artifact["trajectory_payload"].encode("utf-8")
        initial_state_bytes = artifact["initial_state_payload"].encode("utf-8")
        _need(hashlib.sha256(trajectory_bytes).hexdigest() == artifact["trajectory_sha256"],
              f"trajectory payload SHA-256 mismatch: {artifact['trajectory_id']}")
        _need(hashlib.sha256(initial_state_bytes).hexdigest() == artifact["initial_state_sha256"],
              f"initial-state payload SHA-256 mismatch: {artifact['trajectory_id']}")
        _need(artifact["trajectory_id"] not in trajectory_artifacts, "duplicate trajectory artifact ID")
        previous_state_digest = initial_state_digests.get(artifact["initial_state_id"])
        _need(previous_state_digest is None or previous_state_digest == artifact["initial_state_sha256"],
              "one initial_state_id references multiple payload digests")
        initial_state_digests[artifact["initial_state_id"]] = artifact["initial_state_sha256"]
        trajectory_artifacts[artifact["trajectory_id"]] = artifact

    controller_artifacts: dict[str, dict[str, Any]] = {}
    for artifact in doc["controller_artifacts"]:
        _need(set(artifact) == {"controller_id", "controller_payload", "controller_sha256"},
              "controller artifact keys do not match the v0.4 contract")
        _need(isinstance(artifact["controller_id"], str) and artifact["controller_id"]
              and isinstance(artifact["controller_payload"], str) and artifact["controller_payload"],
              "controller artifacts require non-empty IDs and payloads")
        digest = artifact["controller_sha256"]
        _need(isinstance(digest, str) and digest_re.fullmatch(digest), "invalid controller artifact digest")
        _need(hashlib.sha256(artifact["controller_payload"].encode("utf-8")).hexdigest() == digest,
              f"controller payload SHA-256 mismatch: {artifact['controller_id']}")
        _need(artifact["controller_id"] not in controller_artifacts, "duplicate controller artifact ID")
        controller_artifacts[artifact["controller_id"]] = artifact
    _need(set(controller_artifacts) == set(controllers), "controller artifacts do not match declared controllers")

    contracts = doc["gate_margin_contracts"]
    _need(set(contracts) == expected_gates, "gate margin contracts do not match declared gates")
    for gate, contract in contracts.items():
        _need(set(contract) == {"unit", "signed_margin_definition", "hard_gate"}
              and isinstance(contract["unit"], str) and contract["unit"].strip()
              and isinstance(contract["signed_margin_definition"], str) and contract["signed_margin_definition"].strip()
              and contract["hard_gate"] is True, "every scored gate needs an explicit hard-gate margin contract")
    for case in doc["cases"]:
        _need(set(case) == CASE_KEYS, "case keys do not match contract")
        for field in ("case_id", "controller_id", "trajectory_id", "comparison_group_id",
                      "hazard_family", "initial_state_id", "replicate_id"):
            _need(isinstance(case[field], str) and case[field].strip(), f"empty {field}")
        _need(case["case_id"] not in case_ids, "duplicate case ID")
        case_ids.add(case["case_id"])
        _need(case["controller_id"] in controllers, "undeclared controller")
        for field in ("trajectory_sha256", "initial_state_sha256", "controller_artifact_sha256"):
            _need(isinstance(case[field], str) and digest_re.fullmatch(case[field]),
                  f"invalid case digest: {field}")
        ta = trajectory_artifacts.get(case["trajectory_id"])
        ca = controller_artifacts.get(case["controller_id"])
        _need(ta is not None, "case references an undeclared trajectory artifact")
        _need(ca is not None, "case references an undeclared controller artifact")
        _need(case["trajectory_sha256"] == ta["trajectory_sha256"]
              and case["initial_state_id"] == ta["initial_state_id"]
              and case["initial_state_sha256"] == ta["initial_state_sha256"]
              and case["controller_artifact_sha256"] == ca["controller_sha256"],
              "case digest/ID binding does not match artifact registry")
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
                case["replicate_id"], json.dumps(severity, sort_keys=True),
                case["trajectory_sha256"], case["initial_state_sha256"])
        if tid in metadata:
            _need(metadata[tid] == meta, "paired rows differ in trajectory metadata")
        else:
            metadata[tid] = meta

    _need(bool(trajectories), "empty trajectory set")
    _need(set(trajectory_artifacts) == trajectories, "trajectory artifacts do not match observed trajectory IDs")
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
    trajectory_artifacts = {a["trajectory_id"]: a for a in doc["trajectory_artifacts"]}
    controller_artifacts = {a["controller_id"]: a for a in doc["controller_artifacts"]}
    trajectory_digest_map = {
        tid: {
            "trajectory_sha256": artifact["trajectory_sha256"],
            "initial_state_id": artifact["initial_state_id"],
            "initial_state_sha256": artifact["initial_state_sha256"],
        }
        for tid, artifact in sorted(trajectory_artifacts.items())
    }
    controller_digest_map = {
        cid: artifact["controller_sha256"] for cid, artifact in sorted(controller_artifacts.items())
    }
    return {
        "schema_version": "0.4.0",
        "report_kind": "paired-resilience-comparison-report",
        "evidence_class": "synthetic_fixture",
        "status": "synthetic_only",
        "controller_count": len(controllers),
        "trajectory_count": len(trajectories),
        "case_count": len(doc["cases"]),
        "gate_margin_contracts": doc["gate_margin_contracts"],
        "content_bindings": {
            "trajectory_digests": trajectory_digest_map,
            "controller_digests": controller_digest_map,
            "payload_sha256_verification": "delegated_to_independent_verifier",
        },
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
    print("independent paired-comparison report and artifact SHA-256 verification passed (synthetic contract only)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
