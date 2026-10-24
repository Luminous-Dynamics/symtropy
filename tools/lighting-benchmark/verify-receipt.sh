#!/usr/bin/env bash
# Independently audit a complete-grid lighting receipt against its input artifacts.
# This is a contract verifier, not a Radiance parser or an optical-accuracy validator.
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
Usage:
  verify-receipt.sh RECEIPT CASE DESIGN SCENE PROPERTY_DATA ADAPTER_CONFIG EXECUTABLE ENVIRONMENT SETTINGS NORMALIZED_SAMPLES RAW_OUTPUT STDERR

NORMALIZED_SAMPLES must be a JSON array of ordered grid sample objects. The array must
match receipt.illuminance_samples exactly; its file digest is recorded in the receipt.
The verifier checks artifact digests, grid coordinates/order, derived summary metrics,
interval order and scoreability constraints. It does not prove that a solver parser
correctly transformed native solver output into NORMALIZED_SAMPLES.
USAGE
}

fail() {
  printf 'lighting receipt audit: FAIL: %s\n' "$*" >&2
  exit 1
}

[[ $# -eq 12 ]] || { usage; fail "expected 12 arguments, got $#"; }
command -v jq >/dev/null 2>&1 || fail "jq is required"
command -v sha256sum >/dev/null 2>&1 || fail "sha256sum is required"

RECEIPT=$1
CASE=$2
DESIGN=$3
SCENE=$4
PROPERTY_DATA=$5
ADAPTER_CONFIG=$6
EXECUTABLE=$7
ENVIRONMENT=$8
SETTINGS=$9
NORMALIZED_SAMPLES=${10}
RAW_OUTPUT=${11}
STDERR_FILE=${12}

for path in "$RECEIPT" "$CASE" "$DESIGN" "$SCENE" "$PROPERTY_DATA" \
            "$ADAPTER_CONFIG" "$EXECUTABLE" "$ENVIRONMENT" "$SETTINGS" \
            "$NORMALIZED_SAMPLES" "$RAW_OUTPUT" "$STDERR_FILE"; do
  [[ -f "$path" && -r "$path" ]] || fail "missing or unreadable artifact: $path"
done
[[ -x "$EXECUTABLE" ]] || fail "recorded executable artifact is not executable: $EXECUTABLE"

sha256() {
  sha256sum -- "$1" | awk '{print $1}'
}

expect_digest() {
  local field=$1
  local path=$2
  local expected actual
  expected=$(jq -er "$field | select(type == \"string\")" "$RECEIPT") ||
    fail "receipt field missing or not a string: $field"
  [[ "$expected" =~ ^[a-f0-9]{64}$ ]] || fail "receipt field is not a lowercase SHA-256 digest: $field"
  actual=$(sha256 "$path") || fail "could not hash artifact: $path"
  [[ "$actual" == "$expected" ]] ||
    fail "digest mismatch for $field (artifact: $path)"
}

expect_digest '.inputs.case_sha256' "$CASE"
expect_digest '.inputs.design_sha256' "$DESIGN"
expect_digest '.inputs.scene_sha256' "$SCENE"
expect_digest '.inputs.property_data_sha256' "$PROPERTY_DATA"
expect_digest '.inputs.adapter_config_sha256' "$ADAPTER_CONFIG"
expect_digest '.toolchain.executable_sha256' "$EXECUTABLE"
expect_digest '.toolchain.environment_sha256' "$ENVIRONMENT"
expect_digest '.toolchain.settings_sha256' "$SETTINGS"
expect_digest '.execution.normalized_samples_sha256' "$NORMALIZED_SAMPLES"
expect_digest '.execution.raw_output_sha256' "$RAW_OUTPUT"
expect_digest '.execution.stderr_sha256' "$STDERR_FILE"

jq -e --slurpfile receipt "$RECEIPT"   '. == $receipt[0].illuminance_samples' "$NORMALIZED_SAMPLES" >/dev/null ||
  fail "normalized sample artifact does not exactly match receipt.illuminance_samples"

# Recompute the grid from the frozen case, then independently recompute all summary
# metrics from raw per-point lux values. Coordinates are x-fastest, then y.
jq -e --slurpfile case "$CASE" '
  def near($a; $b):
    (($a - $b) | abs) <= (0.000001 * ([1, ($a | abs), ($b | abs)] | max));
  . as $r
  | $case[0] as $c
  | $c.task_surface as $t
  | $t.measurement_grid as $g
  | ($g.spacing_m) as $spacing
  | (($t.length_x_m / $spacing) | round) as $x_intervals
  | (($t.width_y_m / $spacing) | round) as $y_intervals
  | ($x_intervals + 1) as $nx
  | ($y_intervals + 1) as $ny
  | ($nx * $ny) as $expected_points
  | ($r.illuminance_samples | map(.illuminance_lx)) as $lux
  | ($lux | add / length) as $mean
  | ($lux | min) as $minimum
  | ($lux | max) as $maximum
  | (
      $r.schema_version == "0.2.0"
      and $r.evaluator.receipt_format_version == "0.2.0"
      and $r.case_id == $c.case_id
      and $spacing > 0
      and near(($t.length_x_m / $spacing); $x_intervals)
      and near(($t.width_y_m / $spacing); $y_intervals)
      and $g.expected_points == $expected_points
      and ($r.illuminance_samples | length) == $expected_points
      and ($r.illuminance_samples | length) > 0
      and all(range(0; $expected_points) as $i;
        $r.illuminance_samples[$i] as $s
        | $s.grid_index == $i
          and near($s.x_m; ($t.origin_xyz_m[0] + (($i % $nx) * $spacing)))
          and near($s.y_m; ($t.origin_xyz_m[1] + ((($i / $nx) | floor) * $spacing)))
          and near($s.z_m; $t.origin_xyz_m[2])
          and ($s.illuminance_lx >= 0)
          and (if ($s | has("interval")) then
            ($s.interval.lower_lx | type) == "number"
            and ($s.interval.upper_lx | type) == "number"
            and $s.interval.lower_lx >= 0
            and $s.interval.lower_lx <= $s.interval.upper_lx
          else true end)
      )
      and ($mean > 0)
      and near($r.illumination_summary.average_lx; $mean)
      and near($r.illumination_summary.minimum_lx; $minimum)
      and near($r.illumination_summary.maximum_lx; $maximum)
      and near($r.illumination_summary.minimum_to_average_uniformity; ($minimum / $mean))
      and $r.illumination_summary.derivation == "independent_evaluator_from_raw_grid"
      and (($r.gate_results | map(.name) | unique | length) == ($r.gate_results | length))
      and all($r.gate_results[];
        (.name | length) > 0 and (.evidence_id | length) > 0
        and (.status == "pass" or .status == "fail" or .status == "unknown"
             or .status == "unsupported" or .status == "not_evaluated"
             or .status == "timeout" or .status == "invalid_output"))
      and (if $r.run_status == "compliant" then
        $r.execution.exit_code == 0
        and ($r.execution.timed_out == false)
        and ($r.gate_results | length > 0)
        and all($r.gate_results[]; .status == "pass")
        and $r.interval_evidence.basis != "unknown"
        and $mean >= $c.service_targets.average_illuminance_min_lx
        and $minimum >= $c.service_targets.minimum_grid_illuminance_min_lx
        and ($minimum / $mean) >= $c.service_targets.minimum_to_average_uniformity_min
        and $r.power_observation.wall_power_upper_bound_w <= $c.energy_boundary.power_limit_w
        and $r.power_observation.wall_power_upper_bound_w <= $c.candidate_domain.maximum_wall_power_w
        and $r.power_observation.observed_hours > 0
        and $r.power_observation.compliant_task_hours_lower_bound > 0
        and $r.power_observation.compliant_task_hours_lower_bound <= $r.power_observation.observed_hours
      else true end)
      and (if $r.run_status == "timeout" then $r.execution.timed_out == true else true end)
      and (if $r.execution.timed_out == true then $r.run_status == "timeout" else true end)
      and (if $r.interval_evidence.basis == "unknown" then $r.run_status != "compliant" else true end)
      and (if $r.evidence_tier == "calibrated_simulation" then
        $r.interval_evidence.basis == "calibrated_prediction_interval"
        and ($r | has("measurement"))
      else true end)
      and (if ($r.evidence_tier == "physical_prototype"
               or $r.evidence_tier == "independently_tested_product"
               or $r.evidence_tier == "calibrated_simulation")
          then ($r | has("measurement")) else true end)
    )
' "$RECEIPT" >/dev/null || fail "receipt, grid, summaries or evidence gates are inconsistent"

printf 'lighting receipt audit: PASS (artifact digests, ordered grid, recomputed summary and eligibility gates)\n'
printf 'scope: complete-grid receipt consistency only; solver-output parsing and physical validity are not established\n'
