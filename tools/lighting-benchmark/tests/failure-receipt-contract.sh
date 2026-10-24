#!/usr/bin/env bash
# Contract checks for deliberately non-rankable failed/incomplete lighting runs.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
SCHEMA="$ROOT/docs/research/lighting-failure-receipt.schema.json"
CASE="$ROOT/docs/research/fixtures/task-light-room-v0.1.json"
command -v jq >/dev/null 2>&1 || { echo "failure receipt contract: jq is required" >&2; exit 1; }
command -v python3 >/dev/null 2>&1 || { echo "failure receipt contract: python3 is required" >&2; exit 1; }

fail() { printf 'failure receipt contract: FAIL: %s\n' "$*" >&2; exit 1; }
pass() { printf 'failure receipt contract: PASS: %s\n' "$*"; }

[[ -f "$SCHEMA" && -f "$CASE" ]] || fail "schema or frozen case fixture missing"
python3 -m json.tool "$SCHEMA" >/dev/null || fail "failure-receipt schema is not valid JSON"
python3 -m json.tool "$CASE" >/dev/null || fail "lighting case fixture is not valid JSON"

# Check the contract itself cannot express a rankable result. This is a narrow
# schema-invariant test, not a general JSON Schema validator.
jq -e '
  .properties.run_status.enum as $statuses
  | .properties.scoring.properties.eligible.const == false
    and .properties.scoring.properties.service_energy_metric_wh_per_compliant_task_hour.type == "null"
    and ($statuses | index("compliant") == null)
    and (.required | index("scoring") != null)
    and (.required | index("summary") != null)
    and (.required | index("power_observation") != null)
' "$SCHEMA" >/dev/null || fail "schema permits or omits rankability protections"

count=0
for receipt in "$ROOT"/docs/research/fixtures/lighting-failure-*.v0.1.json; do
  [[ -f "$receipt" ]] || fail "no failure-receipt examples found"
  python3 -m json.tool "$receipt" >/dev/null || fail "invalid JSON: $receipt"

  jq -e --slurpfile case "$CASE" '
    . as $r
    | ($r.inputs as $in
       | ([$in.case, $in.design, $in.scene, $in.property_data, $in.adapter_config,
           $r.toolchain.executable, $r.toolchain.environment, $r.toolchain.settings,
           $r.artifacts.stdout, $r.artifacts.stderr, $r.artifacts.normalized_samples]) as $refs
       | all($refs[];
           if .availability == "available" then
             (.sha256 | type == "string" and test("^[a-f0-9]{64}$"))
             and .reason == null
           elif .availability == "unavailable" then
             .sha256 == null and (.reason | type == "string" and length > 0)
           else false end))
    and $r.schema_version == "0.1.0"
    and $r.receipt_kind == "failure"
    and $r.case_id == $case[0].case_id
    and ($r.run_status != "compliant")
    and ($r.observation.expected_sample_count >= 1)
    and ($r.observation.observed_sample_count >= 0)
    and ($r.observation.observed_sample_count <= $r.observation.expected_sample_count)
    and ($r.observation.samples | length) == $r.observation.observed_sample_count
    and (
      if $r.observation.completeness == "none" then
        $r.observation.observed_sample_count == 0 and ($r.observation.samples | length) == 0
      elif $r.observation.completeness == "partial" then
        $r.observation.observed_sample_count > 0
        and $r.observation.observed_sample_count < $r.observation.expected_sample_count
      elif $r.observation.completeness == "complete" then
        $r.observation.observed_sample_count == $r.observation.expected_sample_count
      else false end)
    and $r.summary == null
    and $r.power_observation == null
    and $r.scoring.eligible == false
    and $r.scoring.service_energy_metric_wh_per_compliant_task_hour == null
    and ($r.gate_results | length > 0)
    and (($r.gate_results | map(.name) | unique | length) == ($r.gate_results | length))
    and all($r.gate_results[]; (.name | length) > 0 and (.evidence_id | length) > 0)
    and (
      if $r.failure_code == "missing_executable" then
        $r.run_status == "unsupported"
        and $r.process.invocation_attempted == false
        and $r.process.executable_availability == "missing"
        and $r.process.exit_code == null
        and $r.process.timed_out == false
        and $r.observation.completeness == "none"
        and $r.toolchain.executable.availability == "unavailable"
      elif $r.failure_code == "timeout" then
        $r.run_status == "timeout"
        and $r.process.invocation_attempted == true
        and $r.process.timed_out == true
      elif $r.failure_code == "truncated_output" then
        $r.run_status == "invalid_output"
        and $r.observation.completeness == "partial"
      elif $r.failure_code == "malformed_output" then
        $r.run_status == "invalid_output"
        and $r.observation.completeness != "complete"
      elif $r.failure_code == "artifact_digest_mismatch" then
        $r.run_status == "invalid_output"
        and $r.failure_class == "evidence_failure"
        and $r.scoring.non_rankable_reason == "integrity_failure"
        and ($r.integrity_findings | length > 0)
        and any($r.integrity_findings[];
          (.claimed_sha256 | type == "string" and test("^[a-f0-9]{64}$"))
          and (.observed_sha256 | type == "string" and test("^[a-f0-9]{64}$"))
          and .claimed_sha256 != .observed_sha256)
      elif $r.failure_code == "unknown_required_gate" then
        $r.run_status == "unknown"
        and $r.failure_class == "evidence_failure"
        and $r.scoring.non_rankable_reason == "unknown_gate"
        and any($r.gate_results[]; .status == "unknown")
      else true end)
    and all($r.observation.samples[];
      . as $sample
      | ($sample.grid_index >= 0)
        and ($sample.illuminance_lx >= 0)
        and ($sample.grid_index < $r.observation.expected_sample_count))
  ' "$receipt" >/dev/null || fail "receipt invariant failed: $receipt"

  # For the frozen task plane, any retained partial samples must still match the
  # grid's coordinate/index convention; partial does not mean reorderable.
  jq -e --slurpfile case "$CASE" '
    . as $r
    | $case[0].task_surface as $t
    | $t.measurement_grid.spacing_m as $spacing
    | ((($t.length_x_m / $spacing) | round) + 1) as $nx
    | all($r.observation.samples[];
        . as $s
        | $s.x_m == ($t.origin_xyz_m[0] + (($s.grid_index % $nx) * $spacing))
          and $s.y_m == ($t.origin_xyz_m[1] + (((($s.grid_index / $nx) | floor)) * $spacing))
          and $s.z_m == $t.origin_xyz_m[2])
    and ([$r.observation.samples[].grid_index] == ([$r.observation.samples[].grid_index] | unique))
    and ([$r.observation.samples[].grid_index] == ([$r.observation.samples[].grid_index] | sort))
  ' "$receipt" >/dev/null || fail "sample coordinate/order contract failed: $receipt"

  printf 'failure receipt contract: checked %s\n' "$(basename "$receipt")"
  count=$((count + 1))
done

[[ "$count" -ge 5 ]] || fail "expected at least 5 failure-receipt fixtures, found $count"
pass "$count non-rankable failure-receipt examples satisfy structural invariants and negative-control rules"
printf 'scope: JSON syntax and selected contract invariants only; a full JSON Schema implementation and real solver provenance are not established by this script\n'
