#!/usr/bin/env bash
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"

schema="tools/lunar-granular-flow-benchmark/granular-flow-run.schema.json"
manifest_schema="tools/lunar-granular-flow-benchmark/dataset-manifest.schema.json"
doe="tools/lunar-granular-flow-benchmark/design-of-experiments.json"
formula_registry="tools/lunar-granular-flow-benchmark/dimensionless-coordinate-formulas.json"
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
  .formula_set_id == "lunar-granular-dimensionless-groups-v1"
  and .evidence_class == "dimensionless_definition_registry"
  and ([.coordinates[].id] | sort) == [
    "electrostatic_number",
    "granular_bond_number",
    "inertial_number",
    "particle_to_drum_ratio",
    "vibration_amplitude_to_particle_size",
    "vibration_frequency_gravity_scale",
    "vibration_gamma",
    "wall_adhesion_to_particle_weight",
    "wall_roughness_to_particle_ratio"
  ]
  and (.coordinates | length) == 9
  and ([.coordinates[] | select(.id == "electrostatic_number").domain][0]) == "signed"
  and ([.coordinates[] | select(.id == "electrostatic_number").sign_semantics][0]
       | contains("preserve sign"))
  and all(.coordinates[]; (.inputs | length) >= 2 and .missing_rule != null)
' "$formula_registry" >/dev/null || fail "dimensionless formula registry identity or coverage invalid"

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
  and all(.vibration_assisted_feeder_source_reconstruction.source_design_reconstruction.records[]; .execution_status == "awaiting_author_data_request" and .record_type == "empty_source_record_template")
  and .vibration_assisted_feeder_source_reconstruction.proposed_gamma_threshold_screen.unique_cells == 800
  and (.vibration_assisted_feeder_source_reconstruction.proposed_gamma_threshold_screen.cells | length) == 800
  and all(.vibration_assisted_feeder_source_reconstruction.proposed_gamma_threshold_screen.cells[]; .execution_status == "planned_not_run")
  and .cross_gravity_material_transfer_screen.unique_cells == 80
  and (.cross_gravity_material_transfer_screen.cells | length) == 80
  and ([.cross_gravity_material_transfer_screen.cells[].test_id] | length == (unique | length))
  and all(.cross_gravity_material_transfer_screen.cells[]; .execution_status == "planned_not_run")
  and ([.cross_gravity_material_transfer_screen.cells[].material] | unique | sort) == ["ICN-LHT-1", "LHS-1"]
  and ([.cross_gravity_material_transfer_screen.cells[].gravity_m_s2] | unique | sort) == [1.62, 9.81]
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
  and .polydisperse_agglutinate_breakage_screen.unique_cells == 36
  and (.polydisperse_agglutinate_breakage_screen.cells | length) == 36
  and ([.polydisperse_agglutinate_breakage_screen.cells[].test_id] | length == (unique | length))
  and all(.polydisperse_agglutinate_breakage_screen.cells[]; .execution_status == "planned_not_run")
  and ([.polydisperse_agglutinate_breakage_screen.cells[].initial_agglutinate_volume_fraction] | unique | sort) == [0.1,0.3,0.6]
  and ([.polydisperse_agglutinate_breakage_screen.cells[].electrostatic_boundary] | unique | sort)
      == ["grounded_contact_system","isolated_contact_system"]
  and .polydisperse_agglutinate_breakage_screen.unique_cells == 36
  and (.polydisperse_agglutinate_breakage_screen.cells | length) == 36
  and ([.polydisperse_agglutinate_breakage_screen.cells[].test_id] | length == (unique | length))
  and all(.polydisperse_agglutinate_breakage_screen.cells[]; .execution_status == "planned_not_run")
  and ([.polydisperse_agglutinate_breakage_screen.cells[].initial_agglutinate_volume_fraction] | unique | sort) == [0.1,0.3,0.6]
  and ([.polydisperse_agglutinate_breakage_screen.cells[].electrostatic_boundary] | unique | sort)
      == ["grounded_contact_system","isolated_contact_system"]
  and .cross_gravity_material_transfer_screen.unique_cells == 80
  and (.cross_gravity_material_transfer_screen.cells | length) == 80
  and ([.cross_gravity_material_transfer_screen.cells[].test_id] | length == (unique | length))
  and all(.cross_gravity_material_transfer_screen.cells[]; .execution_status == "planned_not_run")
  and ([.cross_gravity_material_transfer_screen.cells[].material] | unique | sort) == ["ICN-LHT-1", "LHS-1"]
  and ([.cross_gravity_material_transfer_screen.cells[].gravity_m_s2] | unique | sort) == [1.62, 9.81]
  and .lhs1_ground_hopper_regime_extension.unique_cells == 63
  and (.lhs1_ground_hopper_regime_extension.cells | length) == 63
  and ([.lhs1_ground_hopper_regime_extension.cells[].test_id] | length == (unique | length))
  and all(.lhs1_ground_hopper_regime_extension.cells[]; .execution_status == "planned_not_run")
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

  jq -e '
    . as $r
    | $r.derived_coordinates as $d
    | ($d.formula_set_id == "lunar-granular-dimensionless-groups-v1")
    and ($d.uncertainty_propagation_method == "not_applicable_synthetic")
    and ((($d.vibration_gamma.value
           - ($r.operation.vibration_peak_acceleration_m_s2.value
              / $r.environment.gravity_m_s2.value)) | abs) <= 0.001)
    and ((($d.vibration_frequency_gravity_scale.value
           - ($r.operation.vibration_frequency_hz.value
              * sqrt(($r.material.d50_um.value / 1000000)
                     / $r.environment.gravity_m_s2.value))) | abs) <= 0.001)
    and ((($d.vibration_amplitude_to_particle_size.value
           - ($r.operation.vibration_amplitude_m.value
              / ($r.material.d50_um.value / 1000000))) | abs) <= 0.01)
    and ((($d.particle_to_drum_ratio.value
           - (($r.material.d50_um.value / 1000000)
              / $r.apparatus.drum_inner_radius_m.value)) | abs) <= 0.00001)
    and ((($d.wall_roughness_to_particle_ratio.value
           - ($r.apparatus.wall_roughness_ra_um.value
              / $r.material.d50_um.value)) | abs) <= 0.001)
    and ($d.granular_bond_number.status == "unknown")
    and ($d.wall_adhesion_to_particle_weight.status == "unknown")
    and ($d.inertial_number.status == "unknown")
    and ($d.electrostatic_number.status == "unknown")
  ' "$run" >/dev/null || fail "derived dimensionless coordinates do not recompute or unsupported coordinates are not unknown: $run"
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

jq -e '
  .material.adhesion_characterization_status == "synthetic"
  and (.material.adhesion_by_size_class | length) == 1
  and (.derived_coordinates.adhesion_bond_by_size_class | length) == 1
  and (.material.adhesion_by_size_class[0].adhesion_force_p05_n.value
       <= .material.adhesion_by_size_class[0].adhesion_force_p50_n.value)
  and (.material.adhesion_by_size_class[0].adhesion_force_p50_n.value
       <= .material.adhesion_by_size_class[0].adhesion_force_p95_n.value)
  and (.material.adhesion_by_size_class[0].adhesion_force_p95_n.value
       <= .material.adhesion_by_size_class[0].adhesion_force_p99_n.value)
  and (.derived_coordinates.adhesion_bond_by_size_class[0].bo_p05.value
       <= .derived_coordinates.adhesion_bond_by_size_class[0].bo_p50.value)
  and (.derived_coordinates.adhesion_bond_by_size_class[0].bo_p50.value
       <= .derived_coordinates.adhesion_bond_by_size_class[0].bo_p95.value)
  and (.derived_coordinates.adhesion_bond_by_size_class[0].bo_p95.value
       <= .derived_coordinates.adhesion_bond_by_size_class[0].bo_p99.value)
' "$cal" >/dev/null || fail "synthetic size-resolved adhesion quantile fixture invalid"

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

# Negative control 3: do not manufacture a dimensionless value when its
# required primitive measurements are explicitly unknown.
jq '
  .derived_coordinates.granular_bond_number = {
    "value": 0.0,
    "standard_uncertainty": 0.0,
    "unit": "dimensionless",
    "status": "synthetic",
    "source_artifact_id": null,
    "calibration_record_id": null
  }
' "$negative_root/examples/calibration.json" > "$negative_root/examples/calibration-fabricated-derived.json"
if bash scripts/validate-lunar-granular-flow-dataset.sh \
  --allow-synthetic-placeholders \
  "$negative_root/examples/manifest.json" \
  "$negative_root/examples/calibration-fabricated-derived.json" \
  "$negative_root/examples/validation.json" \
  "$negative_root/examples/blind-test.json"; then
  fail "negative control accepted a derived number whose primitive inputs are unknown"
fi

# Negative control 4: the central value of a reported dimensionless group
# must reconcile with the raw primitive measurements in the same run.
jq '.derived_coordinates.vibration_gamma.value = 0.4' \
  "$negative_root/examples/calibration.json" > "$negative_root/examples/calibration-tampered-derived.json"
if bash scripts/validate-lunar-granular-flow-dataset.sh \
  --allow-synthetic-placeholders \
  "$negative_root/examples/manifest.json" \
  "$negative_root/examples/calibration-tampered-derived.json" \
  "$negative_root/examples/validation.json" \
  "$negative_root/examples/blind-test.json"; then
  fail "negative control accepted a dimensionless coordinate inconsistent with its primitives"
fi

# Negative control 5: a confinement/tensile-strength ratio cannot be
# populated while either primitive measurement is unknown.
jq '
  .derived_coordinates.confinement_to_tensile_strength_ratio = {
    "value": 1.0,
    "standard_uncertainty": 0.01,
    "unit": "dimensionless",
    "status": "synthetic",
    "source_artifact_id": null,
    "calibration_record_id": null,
    "derivation_method_id": "synthetic-invalid",
    "calculation_version": "0.0.0"
  }
' "$negative_root/examples/calibration.json" > "$negative_root/examples/calibration-fabricated-sigma-star.json"
if bash scripts/validate-lunar-granular-flow-dataset.sh \
  --allow-synthetic-placeholders \
  "$negative_root/examples/manifest.json" \
  "$negative_root/examples/calibration-fabricated-sigma-star.json" \
  "$negative_root/examples/validation.json" \
  "$negative_root/examples/blind-test.json"; then
  fail "negative control accepted Sigma-star when its primitive measurements are unknown"
fi

# Negative control 6: breakage/segregation timescale ratio is unknown until all
# four physical primitives have been measured.
jq '
  .derived_coordinates.breakage_segregation_timescale_ratio = {
    "value": 1.0,
    "standard_uncertainty": 0.01,
    "unit": "dimensionless",
    "status": "synthetic",
    "source_artifact_id": null,
    "calibration_record_id": null,
    "derivation_method_id": "synthetic-invalid",
    "calculation_version": "0.0.0"
  }
' "$negative_root/examples/calibration.json" > "$negative_root/examples/calibration-fabricated-breakage-ratio.json"
if bash scripts/validate-lunar-granular-flow-dataset.sh \
  --allow-synthetic-placeholders \
  "$negative_root/examples/manifest.json" \
  "$negative_root/examples/calibration-fabricated-breakage-ratio.json" \
  "$negative_root/examples/validation.json" \
  "$negative_root/examples/blind-test.json"; then
  fail "negative control accepted breakage/segregation ratio with unknown primitives"
fi

# Negative control 7: a size-resolved adhesion bond quantile must reconcile
# with that size class's adhesion-force quantile, representative mass and g.
jq '
  .derived_coordinates.adhesion_bond_by_size_class[0].bo_p95.value = 999.0
' "$negative_root/examples/calibration.json" > "$negative_root/examples/calibration-tampered-size-bond.json"
if bash scripts/validate-lunar-granular-flow-dataset.sh \
  --allow-synthetic-placeholders \
  "$negative_root/examples/manifest.json" \
  "$negative_root/examples/calibration-tampered-size-bond.json" \
  "$negative_root/examples/validation.json" \
  "$negative_root/examples/blind-test.json"; then
  fail "negative control accepted size-resolved Bo quantile inconsistent with primitives"
fi

printf 'lunar granular-flow fixture check: PASS (synthetic fixtures and planned DOE only)\n'
