#!/usr/bin/env python3
"""Fail-closed structural and semantic checks for Kitchen Workcell Benchmark V0.1.

This validates the preregistration manifest only. It does not validate instruments,
test physical equipment, or produce any benchmark result.
"""
from __future__ import annotations
import json
import sys
from pathlib import Path
from typing import Any
from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = ROOT / "benchmarks/kitchen-workcell/v0.1/kitchen-workcell-benchmark.schema.json"
MANIFEST = ROOT / "benchmarks/kitchen-workcell/v0.1/manifest.json"

def load_json(path: Path) -> Any:
    try:
        with path.open("r", encoding="utf-8") as stream:
            return json.load(stream)
    except (OSError, json.JSONDecodeError) as exc:
        raise RuntimeError(f"Cannot load {path.relative_to(ROOT)}: {exc}") from exc

def unique_ids(items: list[dict[str, Any]], key: str, label: str) -> set[str]:
    values = [item.get(key) for item in items]
    if any(not isinstance(value, str) or not value.strip() for value in values):
        raise AssertionError(f"{label} contains an empty or non-string {key}")
    if len(values) != len(set(values)):
        raise AssertionError(f"{label} contains duplicate {key} values")
    return set(values)

def main() -> int:
    schema = load_json(SCHEMA)
    manifest = load_json(MANIFEST)
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    errors = sorted(
        validator.iter_errors(manifest),
        key=lambda error: (list(map(str, error.absolute_path)), error.message),
    )
    if errors:
        for error in errors:
            location = ".".join(str(part) for part in error.absolute_path) or "$"
            print(f"SCHEMA FAIL {location}: {error.message}")
        return 1

    task_ids = unique_ids(manifest["tasks"], "task_id", "tasks")
    metric_ids = unique_ids(manifest["metrics"], "metric_id", "metrics")
    comparator_ids = unique_ids(manifest["comparators"], "comparator_id", "comparators")
    hypothesis_ids = unique_ids(
        manifest["target_hypotheses"], "hypothesis_id", "target_hypotheses"
    )
    baseline_ids = unique_ids(
        manifest["planner_baselines"], "baseline_id", "planner_baselines"
    )
    instrument_ids = unique_ids(
        manifest.get("instruments", []), "instrument_id", "instruments"
    )

    for task in manifest["tasks"]:
        unknown = set(task["primary_metrics"]) - metric_ids
        if unknown:
            raise AssertionError(
                f"task {task['task_id']} references unknown metric IDs: {sorted(unknown)}"
            )
    for hypothesis in manifest["target_hypotheses"]:
        if hypothesis["metric_id"] not in metric_ids:
            raise AssertionError(
                f"hypothesis {hypothesis['hypothesis_id']} references unknown metric "
                f"{hypothesis['metric_id']}"
            )

    plan = manifest["analysis_plan"]
    unknown_endpoints = set(plan["primary_endpoints"]) - metric_ids
    if unknown_endpoints:
        raise AssertionError(
            f"analysis plan references unknown primary endpoints: {sorted(unknown_endpoints)}"
        )
    if plan["estimands"]["no_composite_score"] is not True:
        raise AssertionError("the analysis plan must forbid a composite kitchen score")
    if plan["reporting"]["forbid_metric_compensation"] is not True:
        raise AssertionError("hard quality/safety gates must not be compensable")
    if plan["sample_size"]["status"] != "not_calculated":
        raise AssertionError("sample size must remain uncalculated until a reviewed power plan exists")
    if (
        plan["sample_size"]["minimum_repetitions_per_condition"] is not None
        or plan["sample_size"]["consumer_panel_size_per_sample"] is not None
        or plan["sample_size"]["power_analysis_ref"] is not None
    ):
        raise AssertionError("do not fabricate repetitions or panel size before power analysis")

    # This script validates the current draft state. When the protocol is frozen
    # or physical trials begin, deliberately revise this guard and the run parser.
    if manifest["status"] != "draft_not_run":
        raise AssertionError("V0.1 validator currently qualifies only the draft_not_run state")
    if manifest["preregistration"]["frozen_before_trials"] is not False:
        raise AssertionError("draft manifest must not claim preregistration has been frozen")
    if manifest["preregistration"].get("frozen_at") is not None:
        raise AssertionError("draft manifest cannot contain a preregistration freeze timestamp")
    if manifest["preregistration"].get("reviewer_refs"):
        raise AssertionError("draft manifest cannot claim reviewer approval")
    if manifest["runs"]:
        raise AssertionError("draft manifest must not contain unverified run records")

    for comparator in manifest["comparators"]:
        if comparator["market_status"] != "candidate_not_acquired":
            raise AssertionError(
                f"draft comparator {comparator['comparator_id']} unexpectedly claims acquisition"
            )
        if comparator["exact_revision_status"] != "unverified":
            raise AssertionError(
                f"draft comparator {comparator['comparator_id']} unexpectedly claims exact revision"
            )
    for instrument in manifest.get("instruments", []):
        if instrument.get("status") != "not_selected" or instrument.get("calibration_ref") is not None:
            raise AssertionError(
                f"draft instrument {instrument['instrument_id']} unexpectedly claims selection/calibration"
            )
    if any(item["status"] != "unverified_hypothesis" for item in manifest["target_hypotheses"]):
        raise AssertionError("every target hypothesis must remain explicitly unverified in draft state")

    print(
        "PASS kitchen benchmark manifest: "
        f"{len(task_ids)} tasks, {len(metric_ids)} metrics, "
        f"{len(hypothesis_ids)} hypotheses, {len(comparator_ids)} comparators, "
        f"{len(baseline_ids)} baselines, {len(instrument_ids)} instrument roles"
    )
    print("PASS analysis plan: matched crossover, no composite, power calculation pending")
    print("PASS draft boundary: zero runs, no acquired comparator or calibrated instrument claims")
    return 0

if __name__ == "__main__":
    sys.exit(main())
