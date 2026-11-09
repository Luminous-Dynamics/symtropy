#!/usr/bin/env python3
"""Validate layout benchmark profile, scorecard and frozen adversarial corpus."""
from __future__ import annotations
import json
from pathlib import Path
from jsonschema import Draft202012Validator

root = Path("research/construction-layout-robot-benchmark-v0-1")
profile = json.loads((root / "profile/layout-printing-common-reference-v0.1.json").read_text())
schema = json.loads((root / "schema/layout-benchmark-adversarial-v0.1.schema.json").read_text())
cases = json.loads((root / "fixtures/layout-benchmark-adversarial-v0.1.json").read_text())

Draft202012Validator.check_schema(schema)
Draft202012Validator(schema).validate(cases)

ids = [case["case_id"] for case in cases["cases"]]
assert len(ids) == len(set(ids)), "duplicate frozen case ID"
assert len(ids) == 15

required_metrics = {
    "p50_plan_residual_mm", "p95_plan_residual_mm", "p99_plan_residual_mm",
    "maximum_plan_residual_mm", "feature_completeness_percent",
    "accepted_printed_metres_per_hour", "crew_hours_per_accepted_1000m",
    "setup_time_s", "rework_time_s", "task_completion_rate"
}
all_metrics = set().union(*profile["metrics"].values())
missing = required_metrics - all_metrics
assert not missing, f"profile missing scorecard metrics: {sorted(missing)}"

assert profile["comparison_policy"]["evidence_labels"] == [
    "vendor_reported", "independently_measured", "synthetic_fixture",
    "not_publicly_disclosed", "not_evaluated"
]
assert all(claim["evidence_label"] == "vendor_reported" for claim in profile["product_reference_claims"])
assert profile["prospective_targets"]["status"] == "aspirational_challenge_targets_not_achieved_results"

expected_cases = {
    "stale-design-revision", "site-epoch-invalidated", "coordinate-frame-mismatch",
    "transform-bundle-changed", "uncertainty-too-large-for-decision-margin",
    "preparation-time-omitted", "blocked-features-removed-from-total-denominator",
    "vendor-tolerance-substitution", "executor-finished-without-independent-receipt",
    "manual-baseline-different-geometry", "missing-instrument-calibration-evidence",
    "unit-conversion-loss", "accessible-and-total-coverage-separated",
    "lifecycle-cost-excludes-downtime-and-rework"
}
actual_cases = {case["name"] for case in cases["cases"]}
assert expected_cases <= actual_cases

targets = profile["prospective_targets"]
assert targets["minimum_total_workflow_crew_hour_reduction_vs_directly_measured_matched_manual_baseline_percent"] == 70
assert targets["minimum_elapsed_layout_duration_reduction_vs_directly_measured_matched_manual_baseline_percent"] == 20
assert targets["minimum_rework_crew_hour_reduction_vs_directly_measured_matched_manual_baseline_percent"] == 90

print(
    f"validated {len(profile['layout_strata'])} layout strata, "
    f"{len(all_metrics)} metrics, {len(ids)} frozen adversarial cases"
)
