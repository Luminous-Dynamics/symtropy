#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation-sensitivity gate for local 3D Timoshenko beam matrices.
# Each source mutant lives in a temporary crate copy; production files are not edited.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/timoshenko-3d-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
source_sha256="$(sha256sum "$crate_dir/src/timoshenko_3d.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib timoshenko_3d::tests >"$control_log" 2>&1; then
  printf 'ERROR: unmutated 3D Timoshenko test control failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated 3D Timoshenko element tests\n'

run_mutation() {
  local label="$1"
  local mutation="$2"
  local test_name="$3"
  local mutant_dir="$tmp_root/$label"
  local source="$mutant_dir/src/timoshenko_3d.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
  cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"

  case "$mutation" in
    axial_stiffness_drops_area)
      grep -Fq 'let axial_stiffness = youngs_modulus * area / length;' "$source"
      perl -0pi -e 's/let axial_stiffness = youngs_modulus \\* area \\/ length;/let axial_stiffness = youngs_modulus \\/ length;/' "$source"
      grep -Fq 'let axial_stiffness = youngs_modulus / length;' "$source"
      ;;
    torsional_stiffness_drops_J)
      grep -Fq 'let torsional_stiffness = shear_modulus * parameters.torsional_constant_m4 / length;' "$source"
      perl -0pi -e 's/let torsional_stiffness = shear_modulus \* parameters\.torsional_constant_m4 \/ length;/let torsional_stiffness = shear_modulus \/ length;/' "$source"
      grep -Fq 'let torsional_stiffness = shear_modulus / length;' "$source"
      ;;
    wrong_Timoshenko_shear_factor)
      grep -Fq 'let phi_z = 12.0 * youngs_modulus * parameters.second_moment_z_m4' "$source"
      perl -0pi -e 's/let phi_z = 12\.0 \* youngs_modulus \* parameters\.second_moment_z_m4/let phi_z = 6.0 * youngs_modulus * parameters.second_moment_z_m4/' "$source"
      grep -Fq 'let phi_z = 6.0 * youngs_modulus * parameters.second_moment_z_m4' "$source"
      ;;
    swapped_shear_area_axis)
      grep -Fq 'shear_modulus * parameters.shear_area_y_m2 * length_squared);' "$source"
      perl -0pi -e 's/shear_modulus \* parameters\.shear_area_y_m2 \* length_squared\);/shear_modulus * parameters.shear_area_z_m2 * length_squared);/' "$source"
      grep -Fq 'shear_modulus * parameters.shear_area_z_m2 * length_squared);' "$source"
      ;;
    cross_section_bending_axes_swapped)
      grep -Fq 'let factor_z = youngs_modulus * parameters.second_moment_z_m4' "$source"
      perl -0pi -e 's/let factor_z = youngs_modulus \* parameters\.second_moment_z_m4/let factor_z = youngs_modulus * parameters.second_moment_y_m4/' "$source"
      grep -Fq 'let factor_z = youngs_modulus * parameters.second_moment_y_m4' "$source"
      ;;
    polar_mass_uses_torsional_J)
      grep -Fq 'let polar_second_moment = parameters.second_moment_y_m4' "$source"
      perl -0pi -e 's/let polar_second_moment = parameters\.second_moment_y_m4\s*\+ parameters\.second_moment_z_m4;/let polar_second_moment = parameters.torsional_constant_m4;/' "$source"
      grep -Fq 'let polar_second_moment = parameters.torsional_constant_m4;' "$source"
      ;;
    mass_half_factor)
      grep -Fq 'let half_mass = density * area * length / 2.0;' "$source"
      perl -0pi -e 's/let half_mass = density \* area \* length \/ 2\.0;/let half_mass = density * area * length \/ 3.0;/' "$source"
      grep -Fq 'let half_mass = density * area * length / 3.0;' "$source"
      ;;
    *)
      printf 'Unknown 3D Timoshenko mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "timoshenko_3d::tests::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: 3D Timoshenko mutation survived: %s\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  if ! grep -Fq "test timoshenko_3d::tests::$test_name ... FAILED" "$log"; then
    printf 'ERROR: mutant %s failed for a reason other than the named assertion.\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  printf 'KILLED mutation: %s by %s\n' "$label" "$test_name"
}

run_mutation "axial-area-factor" axial_stiffness_drops_area axial_torsional_and_both_bending_blocks_match_closed_forms
run_mutation "torsional-J-factor" torsional_stiffness_drops_J axial_torsional_and_both_bending_blocks_match_closed_forms
run_mutation "Timoshenko-shear-factor" wrong_Timoshenko_shear_factor axial_torsional_and_both_bending_blocks_match_closed_forms
run_mutation "shear-area-axis-swap" swapped_shear_area_axis axial_torsional_and_both_bending_blocks_match_closed_forms
run_mutation "bending-axis-I-swap" cross_section_bending_axes_swapped axial_torsional_and_both_bending_blocks_match_closed_forms
run_mutation "polar-mass-uses-J" polar_mass_uses_torsional_J lumped_translational_and_rotary_mass_are_explicit_and_scale_correctly
run_mutation "translational-mass-half-factor" mass_half_factor lumped_translational_and_rotary_mass_are_explicit_and_scale_correctly

test "$source_sha256" = "$(sha256sum "$crate_dir/src/timoshenko_3d.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg source_sha256 "$source_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "timoshenko-3d-mutation-receipt-v0.1",
    benchmark_id: "PHYS-TIMOSHENKO-3D-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {timoshenko_3d_rs: $source_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "timoshenko_3d::tests", disposition: "pass"},
    mutations: [
      {id: "PHYS-T3D-MUT-AXIAL-AREA-001", expected_test: "axial_torsional_and_both_bending_blocks_match_closed_forms", disposition: "detected"},
      {id: "PHYS-T3D-MUT-TORSION-J-001", expected_test: "axial_torsional_and_both_bending_blocks_match_closed_forms", disposition: "detected"},
      {id: "PHYS-T3D-MUT-SHEAR-PHI-001", expected_test: "axial_torsional_and_both_bending_blocks_match_closed_forms", disposition: "detected"},
      {id: "PHYS-T3D-MUT-SHEAR-AREA-AXIS-001", expected_test: "axial_torsional_and_both_bending_blocks_match_closed_forms", disposition: "detected"},
      {id: "PHYS-T3D-MUT-BENDING-AXIS-001", expected_test: "axial_torsional_and_both_bending_blocks_match_closed_forms", disposition: "detected"},
      {id: "PHYS-T3D-MUT-POLAR-MASS-001", expected_test: "lumped_translational_and_rotary_mass_are_explicit_and_scale_correctly", disposition: "detected"},
      {id: "PHYS-T3D-MUT-MASS-HALF-001", expected_test: "lumped_translational_and_rotary_mass_are_explicit_and_scale_correctly", disposition: "detected"}
    ],
    mutation_count: 7,
    detected_count: 7,
    production_source_unchanged: true,
    claim_ceiling: "local 3D Timoshenko stiffness and declared lumped mass sensitivity only; no global FV5 solve or physical validation"
  }' > "$receipt_path"

printf 'PASS 3D Timoshenko mutation receipt: %s\n' "$receipt_path"
