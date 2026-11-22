#!/usr/bin/env python3
"""Deterministic Pareto and hard-budget evaluator for matched AV policy runs."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from pathlib import Path
from typing import Any

BENCHMARK_ID = "av-pareto-resource-frontier-v0"
ADMISSIBLE_MEASURED = {"MEASURED", "DERIVED_FROM_MEASUREMENT"}


class EvaluationError(ValueError):
    """Malformed, mismatched or non-reproducible comparison bundle."""


def read_json(path: str | Path) -> dict[str, Any]:
    with Path(path).open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise EvaluationError(f"{path}: expected a JSON object")
    return value


def finite_number(value: Any) -> bool:
    return (
        not isinstance(value, bool)
        and isinstance(value, (int, float))
        and math.isfinite(float(value))
    )


def is_sha256(value: Any) -> bool:
    if not isinstance(value, str) or len(value) != 64:
        return False
    try:
        int(value, 16)
    except ValueError:
        return False
    return True


def unique_rows(rows: Any, key: str, label: str) -> dict[str, dict[str, Any]]:
    if not isinstance(rows, list):
        raise EvaluationError(f"{label}: expected an array")
    result: dict[str, dict[str, Any]] = {}
    for row in rows:
        if not isinstance(row, dict):
            raise EvaluationError(f"{label}: rows must be objects")
        identifier = row.get(key)
        if not isinstance(identifier, str) or not identifier or identifier in result:
            raise EvaluationError(f"{label}: missing or duplicate {key}: {identifier!r}")
        result[identifier] = row
    return result


def canonical_digest(value: Any) -> str:
    payload = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
    return hashlib.sha256(payload).hexdigest()


def satisfies(value: float, op: str, limit: float) -> bool:
    if op == "max":
        return value <= limit
    if op == "min":
        return value >= limit
    raise EvaluationError(f"unsupported hard-constraint operator: {op!r}")


def dominates(
    left: dict[str, float],
    right: dict[str, float],
    specs: dict[str, dict[str, Any]],
) -> bool:
    """True only when left is no worse on every axis and better on at least one."""
    strictly_better = False
    for metric_id, spec in specs.items():
        a = left[metric_id]
        b = right[metric_id]
        tolerance = float(spec.get("tolerance", 0.0))
        if spec["direction"] == "maximize":
            if a < b - tolerance:
                return False
            if a > b + tolerance:
                strictly_better = True
        else:
            if a > b + tolerance:
                return False
            if a < b - tolerance:
                strictly_better = True
    return strictly_better


def _validate_metric_specs(rows: Any) -> dict[str, dict[str, Any]]:
    specs = unique_rows(rows, "metric_id", "metric manifest")
    if not specs:
        raise EvaluationError("metric manifest cannot be empty")
    for metric_id, spec in specs.items():
        if spec.get("direction") not in {"maximize", "minimize"}:
            raise EvaluationError(f"{metric_id}: direction must be maximize or minimize")
        if not isinstance(spec.get("unit"), str) or not spec["unit"]:
            raise EvaluationError(f"{metric_id}: unit is required")
        if not isinstance(spec.get("kind"), str) or spec["kind"] not in {"quality", "resource", "risk"}:
            raise EvaluationError(f"{metric_id}: invalid metric kind")
        tol = spec.get("tolerance", 0.0)
        if not finite_number(tol) or tol < 0:
            raise EvaluationError(f"{metric_id}: tolerance must be finite and non-negative")
    return specs


def _read_run_metrics(
    run: dict[str, Any],
    specs: dict[str, dict[str, Any]],
    measurement_class: str,
) -> tuple[dict[str, float], list[str]]:
    rows = unique_rows(run.get("metrics"), "metric_id", f"{run.get('run_id')} metrics")
    unknown = sorted(set(rows) - set(specs))
    if unknown:
        raise EvaluationError(f"{run.get('run_id')}: unregistered metrics: {unknown}")

    values: dict[str, float] = {}
    reasons: list[str] = []
    for metric_id, spec in specs.items():
        row = rows.get(metric_id)
        if row is None:
            reasons.append(f"missing_metric:{metric_id}")
            continue
        if row.get("unit") != spec["unit"]:
            reasons.append(f"unit_mismatch:{metric_id}")
            continue

        status = row.get("status")
        if measurement_class == "synthetic":
            if status != "SYNTHETIC":
                reasons.append(f"measurement_class_mismatch:{metric_id}")
                continue
        elif measurement_class == "measured":
            if status not in ADMISSIBLE_MEASURED:
                reasons.append(f"not_measured:{metric_id}")
                continue
            if not is_sha256(row.get("evidence_sha256")):
                reasons.append(f"missing_measurement_evidence:{metric_id}")
                continue
            if not isinstance(row.get("method"), str) or not row["method"].strip():
                reasons.append(f"missing_measurement_method:{metric_id}")
                continue
        else:
            raise EvaluationError("measurement_class must be synthetic or measured")

        value = row.get("value")
        if not finite_number(value):
            reasons.append(f"invalid_metric_value:{metric_id}")
            continue
        values[metric_id] = float(value)

    return values, reasons


def _apply_constraints(
    values: dict[str, float],
    constraints: list[dict[str, Any]],
) -> list[str]:
    reasons: list[str] = []
    for constraint in constraints:
        metric_id = constraint.get("metric_id")
        op = constraint.get("op")
        limit = constraint.get("value")
        if metric_id not in values:
            reasons.append(f"constraint_metric_unavailable:{metric_id}")
            continue
        if op not in {"max", "min"} or not finite_number(limit):
            raise EvaluationError(f"invalid hard constraint for {metric_id!r}")
        if not satisfies(values[metric_id], op, float(limit)):
            reasons.append(f"hard_constraint:{metric_id}:{op}")
    return reasons


def _frontier_for(
    eligible_ids: list[str],
    values: dict[str, dict[str, float]],
    specs: dict[str, dict[str, Any]],
) -> tuple[list[str], dict[str, list[str]]]:
    dominated_by: dict[str, list[str]] = {}
    for candidate_id in eligible_ids:
        dominated_by[candidate_id] = sorted(
            other_id
            for other_id in eligible_ids
            if other_id != candidate_id and dominates(values[other_id], values[candidate_id], specs)
        )
    frontier = sorted(candidate_id for candidate_id in eligible_ids if not dominated_by[candidate_id])
    return frontier, dominated_by


def evaluate(bundle: dict[str, Any]) -> dict[str, Any]:
    if bundle.get("schema_version") != 1 or bundle.get("benchmark_id") != BENCHMARK_ID:
        raise EvaluationError("bundle schema version or benchmark identity mismatch")
    comparison = bundle.get("comparison")
    if not isinstance(comparison, dict):
        raise EvaluationError("comparison manifest is required")
    comparison_id = comparison.get("comparison_id")
    measurement_class = comparison.get("measurement_class")
    if not isinstance(comparison_id, str) or not comparison_id:
        raise EvaluationError("comparison_id is required")
    if measurement_class not in {"synthetic", "measured"}:
        raise EvaluationError("measurement_class must be synthetic or measured")
    match_identity = comparison.get("match_identity")
    if not isinstance(match_identity, dict) or not match_identity:
        raise EvaluationError("match_identity must be a non-empty object")
    if any(not is_sha256(value) for value in match_identity.values()):
        raise EvaluationError("every match_identity value must be a SHA-256 digest")

    specs = _validate_metric_specs(comparison.get("metric_manifest"))
    constraints = comparison.get("hard_constraints", [])
    if not isinstance(constraints, list):
        raise EvaluationError("hard_constraints must be an array")
    for constraint in constraints:
        if not isinstance(constraint, dict) or constraint.get("metric_id") not in specs:
            raise EvaluationError("hard constraint references an unknown metric")
        if constraint.get("op") not in {"min", "max"} or not finite_number(constraint.get("value")):
            raise EvaluationError("hard constraint has an invalid operator or value")

    runs = unique_rows(bundle.get("runs"), "run_id", "runs")
    if not runs:
        raise EvaluationError("at least one run is required")

    values_by_run: dict[str, dict[str, float]] = {}
    result_by_run: dict[str, dict[str, Any]] = {}
    policy_ids: set[str] = set()

    for run_id, run in runs.items():
        policy_id = run.get("policy_id")
        if not isinstance(policy_id, str) or not policy_id or policy_id in policy_ids:
            raise EvaluationError(f"{run_id}: policy_id is missing or duplicated")
        policy_ids.add(policy_id)
        if not is_sha256(run.get("policy_source_sha256")):
            raise EvaluationError(f"{run_id}: policy source digest is required")
        if run.get("match_identity") != match_identity:
            raise EvaluationError(f"{run_id}: matched protocol/cohort/runtime/hardware identity mismatch")

        values, reasons = _read_run_metrics(run, specs, measurement_class)
        if not reasons:
            reasons.extend(_apply_constraints(values, constraints))
        eligible = not reasons
        result_by_run[run_id] = {
            "run_id": run_id,
            "policy_id": policy_id,
            "eligible": eligible,
            "reason_codes": sorted(set(reasons)),
            "pareto_frontier": False,
            "dominated_by": [],
        }
        if eligible:
            values_by_run[run_id] = values

    eligible_ids = sorted(values_by_run)
    frontier, dominated_by = _frontier_for(eligible_ids, values_by_run, specs)
    frontier_set = set(frontier)
    for run_id in eligible_ids:
        result_by_run[run_id]["pareto_frontier"] = run_id in frontier_set
        result_by_run[run_id]["dominated_by"] = dominated_by[run_id]

    # Optional budget slices create additional constrained frontiers, never weighted scores.
    slices = comparison.get("budget_slices", [])
    if not isinstance(slices, list):
        raise EvaluationError("budget_slices must be an array")
    slice_results: list[dict[str, Any]] = []
    seen_slice_ids: set[str] = set()
    for budget_slice in slices:
        if not isinstance(budget_slice, dict):
            raise EvaluationError("budget slice must be an object")
        slice_id = budget_slice.get("slice_id")
        if not isinstance(slice_id, str) or not slice_id or slice_id in seen_slice_ids:
            raise EvaluationError("budget slice ID is missing or duplicated")
        seen_slice_ids.add(slice_id)
        slice_constraints = budget_slice.get("constraints", [])
        if not isinstance(slice_constraints, list):
            raise EvaluationError(f"{slice_id}: constraints must be an array")
        for constraint in slice_constraints:
            if not isinstance(constraint, dict) or constraint.get("metric_id") not in specs:
                raise EvaluationError(f"{slice_id}: constraint references unknown metric")
            if constraint.get("op") not in {"min", "max"} or not finite_number(constraint.get("value")):
                raise EvaluationError(f"{slice_id}: invalid constraint")
        slice_eligible: list[str] = []
        for run_id in eligible_ids:
            if not _apply_constraints(values_by_run[run_id], slice_constraints):
                slice_eligible.append(run_id)
        slice_frontier, _ = _frontier_for(slice_eligible, values_by_run, specs)
        slice_results.append({
            "slice_id": slice_id,
            "eligible_run_ids": sorted(slice_eligible),
            "frontier_run_ids": slice_frontier,
        })

    return {
        "schema_version": 1,
        "benchmark_id": BENCHMARK_ID,
        "comparison_id": comparison_id,
        "measurement_class": measurement_class,
        "matched_identity_sha256": canonical_digest(match_identity),
        "metric_manifest_sha256": canonical_digest(comparison["metric_manifest"]),
        "hard_constraints_sha256": canonical_digest(constraints),
        "totals": {
            "run_count": len(runs),
            "eligible_run_count": len(eligible_ids),
            "ineligible_run_count": len(runs) - len(eligible_ids),
            "pareto_frontier_count": len(frontier),
            "single_weighted_score": None,
        },
        "pareto_frontier_run_ids": frontier,
        "budget_slices": slice_results,
        "runs": [result_by_run[run_id] for run_id in sorted(result_by_run)],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        scorecard = evaluate(read_json(args.bundle))
        Path(args.output).write_text(
            json.dumps(scorecard, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    except (OSError, json.JSONDecodeError, EvaluationError) as exc:
        print(f"frontier evaluation failed: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
