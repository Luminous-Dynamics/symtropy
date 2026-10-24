#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
AUDITOR="$ROOT/tools/lighting-benchmark/verify-receipt.sh"
CASE="$ROOT/docs/research/fixtures/task-light-room-v0.1.json"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

fail() { printf 'receipt-audit smoke: FAIL: %s\n' "$*" >&2; exit 1; }
pass() { printf 'receipt-audit smoke: PASS: %s\n' "$*"; }
digest() { sha256sum -- "$1" | awk '{print $1}'; }

for tool in jq sha256sum bash; do
  command -v "$tool" >/dev/null 2>&1 || fail "required test command missing: $tool"
done
[[ -f "$AUDITOR" && -f "$CASE" ]] || fail "auditor or case fixture missing"

printf '{"design":"synthetic test artifact"}\n' > "$TMP/design.json"
printf 'void synthetic scene\n' > "$TMP/scene.rad"
printf '{"reflectance_source":"synthetic-only"}\n' > "$TMP/properties.json"
printf '{"adapter":"contract-test","settings":"settings.json"}\n' > "$TMP/adapter-config.json"
printf 'environment=contract-test\n' > "$TMP/environment.txt"
printf '{"ambient_bounces":1,"sampling":"fixed"}\n' > "$TMP/settings.json"
printf 'mock native solver stdout\n' > "$TMP/raw-output.txt"
: > "$TMP/stderr.txt"
EXECUTABLE=$(command -v true)
[[ -x "$EXECUTABLE" ]] || fail "test executable unavailable"

# x advances fastest, then y; values are all 500 lx so the expected summary is exact.
jq -c '
  .task_surface as $t
  | $t.measurement_grid.spacing_m as $spacing
  | (($t.length_x_m / $spacing) | round) as $xi
  | (($t.width_y_m / $spacing) | round) as $yi
  | ($xi + 1) as $nx
  | ($yi + 1) as $ny
  | [range(0; $nx * $ny) as $i
      | {
          grid_index: $i,
          x_m: ($t.origin_xyz_m[0] + (($i % $nx) * $spacing)),
          y_m: ($t.origin_xyz_m[1] + ((($i / $nx) | floor) * $spacing)),
          z_m: $t.origin_xyz_m[2],
          illuminance_lx: 500
        }]
' "$CASE" > "$TMP/normalized-samples.json"

jq -n   --arg case_id "$(jq -r .case_id "$CASE")"   --arg case_sha "$(digest "$CASE")"   --arg design_sha "$(digest "$TMP/design.json")"   --arg scene_sha "$(digest "$TMP/scene.rad")"   --arg property_sha "$(digest "$TMP/properties.json")"   --arg config_sha "$(digest "$TMP/adapter-config.json")"   --arg executable_sha "$(digest "$EXECUTABLE")"   --arg environment_sha "$(digest "$TMP/environment.txt")"   --arg settings_sha "$(digest "$TMP/settings.json")"   --arg normalized_sha "$(digest "$TMP/normalized-samples.json")"   --arg raw_sha "$(digest "$TMP/raw-output.txt")"   --arg stderr_sha "$(digest "$TMP/stderr.txt")"   --slurpfile samples "$TMP/normalized-samples.json" '
  {
    schema_version: "0.2.0",
    case_id: $case_id,
    run_id: "receipt-audit-smoke-001",
    evidence_tier: "synthetic_contract",
    run_status: "compliant",
    inputs: {
      case_sha256: $case_sha,
      design_sha256: $design_sha,
      scene_sha256: $scene_sha,
      property_data_sha256: $property_sha,
      adapter_config_sha256: $config_sha
    },
    toolchain: {
      engine_name: "mock-radiance-contract-test",
      engine_version: "not-a-real-optical-run",
      engine_source_revision: "synthetic-test",
      executable_sha256: $executable_sha,
      environment_sha256: $environment_sha,
      command_argv: ["true"],
      settings_sha256: $settings_sha
    },
    execution: {
      exit_code: 0,
      timed_out: false,
      duration_ms: 1,
      normalized_samples_sha256: $normalized_sha,
      raw_output_sha256: $raw_sha,
      stderr_sha256: $stderr_sha
    },
    illuminance_samples: $samples[0],
    illumination_summary: {
      average_lx: 500,
      minimum_lx: 500,
      maximum_lx: 500,
      minimum_to_average_uniformity: 1,
      derivation: "independent_evaluator_from_raw_grid"
    },
    power_observation: {
      measurement_boundary: "synthetic AC-input boundary",
      wall_power_upper_bound_w: 8,
      energy_upper_bound_wh: 8,
      observed_hours: 1,
      compliant_task_hours_lower_bound: 1
    },
    interval_evidence: {
      basis: "declared_engineering_bound",
      source_id: "synthetic-fixture-only"
    },
    gate_results: [
      {name: "synthetic-contract-gate", status: "pass", evidence_id: "test-only"}
    ],
    evaluator: {
      name: "lighting receipt audit smoke",
      source_revision: "test-fixture",
      receipt_format_version: "0.2.0"
    }
  }' > "$TMP/receipt.json"

run_audit() {
  bash "$AUDITOR" "$TMP/receipt.json" "$CASE" "$TMP/design.json" "$TMP/scene.rad" \
    "$TMP/properties.json" "$TMP/adapter-config.json" "$EXECUTABLE" \
    "$TMP/environment.txt" "$TMP/settings.json" "$TMP/normalized-samples.json" \
    "$TMP/raw-output.txt" "$TMP/stderr.txt"
}

run_audit >/dev/null || fail "valid complete-grid receipt was rejected"
pass "valid complete-grid receipt accepted"

jq '.illumination_summary.average_lx = 501' "$TMP/receipt.json" > "$TMP/mutated.json"
mv "$TMP/mutated.json" "$TMP/receipt.json"
if run_audit >/dev/null 2>&1; then fail "summary tampering was accepted"; fi
pass "summary tampering rejected"

jq '.illumination_summary.average_lx = 500
    | .run_status = "compliant"
    | .gate_results[0].status = "unknown"' "$TMP/receipt.json" > "$TMP/mutated.json"
mv "$TMP/mutated.json" "$TMP/receipt.json"
if run_audit >/dev/null 2>&1; then fail "compliant receipt with an unknown gate was accepted"; fi
pass "unknown required gate rejected"

jq '.gate_results[0].status = "pass"
    | .execution.timed_out = true' "$TMP/receipt.json" > "$TMP/mutated.json"
mv "$TMP/mutated.json" "$TMP/receipt.json"
if run_audit >/dev/null 2>&1; then fail "timed-out compliant receipt was accepted"; fi
pass "contradictory timeout status rejected"

jq '.execution.timed_out = false
    | .illuminance_samples[0].x_m += 0.25' "$TMP/receipt.json" > "$TMP/mutated.json"
mv "$TMP/mutated.json" "$TMP/receipt.json"
if run_audit >/dev/null 2>&1; then fail "sample/normalized-artifact mismatch was accepted"; fi
pass "sample artifact mismatch rejected"

printf 'receipt-audit smoke: all negative controls behaved as expected\n'
