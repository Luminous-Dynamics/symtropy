#!/usr/bin/env bash
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"

schema="tools/lunar-granular-flow-benchmark/granular-flow-run.schema.json"
manifest_schema="tools/lunar-granular-flow-benchmark/dataset-manifest.schema.json"
doe="tools/lunar-granular-flow-benchmark/design-of-experiments.json"
advanced_doe="tools/lunar-granular-flow-benchmark/advanced-doe-v0.json"
examples="tools/lunar-granular-flow-benchmark/examples"
manifest="$examples/synthetic-dataset-manifest.json"
cal="$examples/synthetic-calibration-run.json"
val="$examples/synthetic-validation-run.json"
blind="$examples/synthetic-blind-test-run.json"

fail() {
  printf 'lunar granular-flow fixture check: FAIL: %s\n' "$*" >&2
  exit 1
}

command -v jq >/dev/null 2>&1 || fail "jq is required"

jq -e '.["$schema"] == "https://json-schema.org/draft/2020-12/schema"
  and .title == "Lunar Granular Flow and Material Transfer Run"' "$schema" >/dev/null \
  || fail "run schema JSON or title invalid"
jq -e '.["$schema"] == "https://json-schema.org/draft/2020-12/schema"
  and .title == "Lunar Granular Flow Dataset Manifest"' "$manifest_schema" >/dev/null \
  || fail "manifest schema JSON or title invalid"

jq -e '
  .evidence_class == "design_of_experiments_only"
  and .source_reproduction.unique_cells == 30
  and (.source_reproduction.cells | length) == 30
  and ([.source_reproduction.cells[].test_id] | length == (unique | length))
  and all(.source_reproduction.cells[]; .execution_status == "planned_not_run")
  and .extension_screen.unique_cells == 90
  and (.extension_screen.cells | length) == 90
  and ([.extension_screen.cells[].test_id] | length == (unique | length))
  and all(.extension_screen.cells[]; .execution_status == "planned_not_run")
  and .batch_transfer_screen.unique_cells == 36
  and (.batch_transfer_screen.cells | length) == 36
  and ([.batch_transfer_screen.cells[].test_id] | length == (unique | length))
  and all(.batch_transfer_screen.cells[]; .execution_status == "planned_not_run")
  and ([.batch_transfer_screen.cells[].batch_role] | unique | sort)
      == ["blind_test_batch", "calibration_batch", "validation_batch"]
' "$doe" >/dev/null || fail "DOE cell counts, unique IDs or planned-only claim ceiling invalid"

jq -e '
  .evidence_class == "planned_experimental_design_only"
  and .vibration_assisted_feeder_source_reconstruction.source_design_reconstruction.planned_records == 480
  and (.vibration_assisted_feeder_source_reconstruction.source_design_reconstruction.records | length) == 480
  and ([.vibration_assisted_feeder_source_reconstruction.source_design_reconstruction.records[].source_record_id] | length == (unique | length))
  and all(.vibration_assisted_feeder_source_reconstruction.source_design_reconstruction.records[]; .execution_status == "source_reconstruction_pending")
  and .vibration_assisted_feeder_source_reconstruction.proposed_gamma_threshold_screen.unique_cells == 800
  and (.vibration_assisted_feeder_source_reconstruction.proposed_gamma_threshold_screen.cells | length) == 800
  and all(.vibration_assisted_feeder_source_reconstruction.proposed_gamma_threshold_screen.cells[]; .execution_status == "planned_not_run")
  and .lhs1_ground_hopper_regime_extension.unique_cells == 63
  and (.lhs1_ground_hopper_regime_extension.cells | length) == 63
  and ([.lhs1_ground_hopper_regime_extension.cells[].test_id] | length == (unique | length))
  and all(.lhs1_ground_hopper_regime_extension.cells[]; .execution_status == "planned_not_run")
  and ([.lhs1_ground_hopper_regime_extension.cells[].target_gamma] | unique | length) == 7
  and ([.lhs1_ground_hopper_regime_extension.cells[].frequency_band] | unique | sort)
      == ["high", "low", "middle"]
  and ([.lhs1_ground_hopper_regime_extension.cells[].charge_mass_role] | unique | sort)
      == ["0.25_rated_mass", "0.50_rated_mass", "1.00_rated_mass"]
  and .vibration_assisted_feeder_source_reconstruction.proposed_segregation_shear_cell.unique_cells == 90
  and (.vibration_assisted_feeder_source_reconstruction.proposed_segregation_shear_cell.cells | length) == 90
  and all(.vibration_assisted_feeder_source_reconstruction.proposed_segregation_shear_cell.cells[]; .execution_status == "planned_not_run")
  and ([.vibration_assisted_feeder_source_reconstruction.proposed_gamma_threshold_screen.cells[].target_gamma] | unique | min) == 0
  and ([.vibration_assisted_feeder_source_reconstruction.proposed_gamma_threshold_screen.cells[].target_gamma] | unique | max) == 23.8
' "$advanced_doe" >/dev/null || fail "advanced feeder/segregation DOE counts or claim ceiling invalid"

jq -e '
  .schema_version == "0.1.0"
  and .benchmark_id == "lunar-granular-flow-validation-v0"
  and .source_class == "synthetic_fixture"
  and .split_policy.blind_test_access.hidden_from_model == true
  and .split_policy.blind_test_access.revealed_after_freeze == true
  and (.split_policy.blind_test_access.freeze_required | unique == [
    "configuration_artifact_id",
    "executable_sha256",
    "parameters_artifact_id",
    "prediction_artifacts",
    "random_seed",
    "source_revision"
  ])
' "$manifest" >/dev/null || fail "manifest source class, blind access or freeze identity invalid"

for run in "$cal" "$val" "$blind"; do
  jq -e '
    .source_class == "synthetic_fixture"
    and (.notes | contains("NON-PHYSICAL SYNTHETIC FIXTURE"))
  ' "$run" >/dev/null || fail "a run fixture is not explicitly synthetic: $run"
done

jq -e '
  .observations as $o
  | $o.input_mass_kg.value
    == ($o.delivered_mass_kg.value + $o.residual_mass_kg.value + $o.dust_and_fines_mass_kg.value)
    and (( $o.transfer_efficiency.value
           - ($o.delivered_mass_kg.value / $o.input_mass_kg.value) ) | abs) < 0.0001
    and (( $o.beneficiation_yield.value
           - ($o.accepted_target_size_mass_kg.value / $o.delivered_mass_kg.value) ) | abs) < 0.0001
' "$cal" >/dev/null || fail "synthetic fixture mass balance or derived efficiencies are inconsistent"

bash scripts/validate-lunar-granular-flow-dataset.sh \
  --allow-synthetic-placeholders \
  "$manifest" "$cal" "$val" "$blind" \
  || fail "reusable granular-flow validator rejected the positive fixture corpus"

# Negative control 1: move the validation run onto a blind-test wall surface.
# It must be rejected by the manifest's group-to-partition map.
negative_root="$(mktemp -d)"
trap 'rm -rf "$negative_root"' EXIT
mkdir -p "$negative_root/examples"
cp "$manifest" "$negative_root/examples/manifest.json"
cp "$cal" "$negative_root/examples/calibration.json"
cp "$val" "$negative_root/examples/validation.json"
cp "$blind" "$negative_root/examples/blind-test.json"
cp "$examples/synthetic-flow-trace.csv" "$examples/synthetic-segmentation-summary.json" \
   "$examples/synthetic-model-config.json" "$negative_root/examples/"

jq '
  .group_keys.wall_surface_id = "SYN-WALL-PTFE-C"
  | .apparatus.wall_surface_id = "SYN-WALL-PTFE-C"
' "$negative_root/examples/validation.json" > "$negative_root/examples/validation-bad.json"
if bash scripts/validate-lunar-granular-flow-dataset.sh \
  --allow-synthetic-placeholders \
  "$negative_root/examples/manifest.json" \
  "$negative_root/examples/calibration.json" \
  "$negative_root/examples/validation-bad.json" \
  "$negative_root/examples/blind-test.json"; then
  fail "negative control accepted cross-partition wall-surface contamination"
fi

# Negative control 2: mutate raw observation bytes after the digest was frozen.
printf 'tampered\n' >> "$negative_root/examples/synthetic-flow-trace.csv"
if bash scripts/validate-lunar-granular-flow-dataset.sh \
  --allow-synthetic-placeholders \
  "$negative_root/examples/manifest.json" \
  "$negative_root/examples/calibration.json" \
  "$negative_root/examples/validation.json" \
  "$negative_root/examples/blind-test.json"; then
  fail "negative control accepted post-freeze raw artifact tampering"
fi

printf 'lunar granular-flow fixture check: PASS (synthetic fixtures and planned DOE only)\n'
