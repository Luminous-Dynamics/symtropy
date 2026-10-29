#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
GATE="$SCRIPT_DIR/evaluate_environment_coverage.sh"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

# Negative control: 4/31 rows are not enough to pass a preregistered 80% gate.
jq -n '{
  schema_version:"0.1.0",
  artifact_kind:"symtropy-environment-coverage-assessment",
  assessment_id:"synthetic_sparse_31_day_window",
  source_id:"synthetic_station_fixture",
  variable_id:"daily_fog",
  window_id:"2023-01-metadata-shaped-control",
  expected_sample_count:31,
  observed_sample_count:4,
  minimum_coverage_fraction:0.80
}' > "$TMP_DIR/sparse.json"
if bash "$GATE" "$TMP_DIR/sparse.json" "$TMP_DIR/sparse-report.json" >/dev/null 2>&1; then
  echo "ERROR: sparse coverage unexpectedly passed" >&2
  exit 1
else
  RC=$?
  test "$RC" -eq 4
fi
jq -e '
  .decision == "insufficient_coverage" and
  .score_eligible == false and
  .expected_sample_count == 31 and
  .observed_sample_count == 4 and
  .missing_sample_count == 27 and
  (.coverage_fraction > 0.12 and .coverage_fraction < 0.14) and
  .missing_values_are_zero == false and
  (.validation_scope | contains("not_source_origin_license_or_physical_validation"))
' "$TMP_DIR/sparse-report.json" >/dev/null

# Positive control: one fully observed 24-sample window clears 80% coverage,
# but this says nothing about other windows or the source's physical validity.
jq -n '{
  schema_version:"0.1.0",
  artifact_kind:"symtropy-environment-coverage-assessment",
  assessment_id:"synthetic_complete_24_sample_window",
  source_id:"synthetic_station_fixture",
  variable_id:"hourly_temperature",
  window_id:"2022-07-27-24-sample-control",
  expected_sample_count:24,
  observed_sample_count:24,
  minimum_coverage_fraction:0.80
}' > "$TMP_DIR/full.json"
bash "$GATE" "$TMP_DIR/full.json" "$TMP_DIR/full-report.json" >/dev/null
jq -e '
  .decision == "coverage_gate_passed" and
  .score_eligible == true and
  .coverage_fraction == 1 and
  .missing_sample_count == 0 and
  .missing_values_are_zero == false
' "$TMP_DIR/full-report.json" >/dev/null

# Fail closed if the threshold is not frozen before the locked evaluation.
jq '.minimum_coverage_fraction = null' "$TMP_DIR/full.json" > "$TMP_DIR/unset.json"
if bash "$GATE" "$TMP_DIR/unset.json" "$TMP_DIR/unset-report.json" >/dev/null 2>&1; then
  echo "ERROR: unset coverage threshold unexpectedly passed" >&2
  exit 1
else
  RC=$?
  test "$RC" -eq 3
fi
jq -e '.decision == "blocked_threshold_unset" and .score_eligible == false' "$TMP_DIR/unset-report.json" >/dev/null

# Invalid sample relationships cannot be scored.
jq '.observed_sample_count = 25' "$TMP_DIR/full.json" > "$TMP_DIR/overcount.json"
if bash "$GATE" "$TMP_DIR/overcount.json" "$TMP_DIR/overcount-report.json" >/dev/null 2>&1; then
  echo "ERROR: observed count larger than expected count unexpectedly passed" >&2
  exit 1
else
  RC=$?
  test "$RC" -eq 2
fi
jq -e '.decision == "invalid_input" and .score_eligible == false' "$TMP_DIR/overcount-report.json" >/dev/null

# Zero coverage threshold is rejected because it would allow no observations.
jq '.minimum_coverage_fraction = 0' "$TMP_DIR/full.json" > "$TMP_DIR/zero-threshold.json"
if bash "$GATE" "$TMP_DIR/zero-threshold.json" "$TMP_DIR/zero-threshold-report.json" >/dev/null 2>&1; then
  echo "ERROR: zero minimum coverage threshold unexpectedly passed" >&2
  exit 1
else
  RC=$?
  test "$RC" -eq 2
fi

echo "Environment coverage gate contract tests: PASS (synthetic cases only)"
