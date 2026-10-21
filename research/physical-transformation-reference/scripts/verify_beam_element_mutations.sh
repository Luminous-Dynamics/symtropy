#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation-sensitivity gate for the local Euler-Bernoulli frame element.
# All changes happen in isolated temporary copies; production source stays fixed.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/beam-element-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
beam_sha256="$(sha256sum "$crate_dir/src/beam_element.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib beam_element::tests >"$control_log" 2>&1; then
  printf 'ERROR: unmutated beam-element test control failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated beam-element tests\n'

run_mutation() {
  local label="$1"
  local mutation="$2"
  local test_name="$3"
  local mutant_dir="$tmp_root/$label"
  local source="$mutant_dir/src/beam_element.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
  cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"

  case "$mutation" in
    bending_omits_second_moment)
      grep -Fq 'let bending_factor = youngs_modulus_pa * second_moment_m4 / l3;' "$source"
      perl -0pi -e 's/let bending_factor = youngs_modulus_pa \* second_moment_m4 \/ l3;/let bending_factor = youngs_modulus_pa \/ l3;/' "$source"
      grep -Fq 'let bending_factor = youngs_modulus_pa / l3;' "$source"
      ;;
    bending_coupling_sign)
      grep -Fq '[12.0, 6.0 * length_m, -12.0, 6.0 * length_m],' "$source"
      perl -0pi -e 's/\[12\.0, 6\.0 \* length_m, -12\.0, 6\.0 \* length_m\],/[12.0, -6.0 * length_m, -12.0, 6.0 * length_m],/' "$source"
      grep -Fq '[12.0, -6.0 * length_m, -12.0, 6.0 * length_m],' "$source"
      ;;
    axial_stiffness_omits_area)
      grep -Fq 'let axial_stiffness = youngs_modulus_pa * area_m2 / length_m;' "$source"
      perl -0pi -e 's/let axial_stiffness = youngs_modulus_pa \* area_m2 \/ length_m;/let axial_stiffness = youngs_modulus_pa \/ length_m;/' "$source"
      grep -Fq 'let axial_stiffness = youngs_modulus_pa / length_m;' "$source"
      ;;
    rotary_inertia_omitted)
      grep -Fq 'let rotary_mass_factor = density_kg_m3 * second_moment_m4 / (30.0 * length_m);' "$source"
      perl -0pi -e 's/let rotary_mass_factor = density_kg_m3 \* second_moment_m4 \/ \(30\.0 \* length_m\);/let rotary_mass_factor = 0.0;/' "$source"
      grep -Fq 'let rotary_mass_factor = 0.0;' "$source"
      ;;
    rotary_inertia_wrong_length_exponent)
      grep -Fq 'let rotary_mass_factor = density_kg_m3 * second_moment_m4 / (30.0 * length_m);' "$source"
      perl -0pi -e 's/let rotary_mass_factor = density_kg_m3 \* second_moment_m4 \/ \(30\.0 \* length_m\);/let rotary_mass_factor = density_kg_m3 * second_moment_m4 \/ (30.0 * length_m * length_m);/' "$source"
      grep -Fq 'let rotary_mass_factor = density_kg_m3 * second_moment_m4 / (30.0 * length_m * length_m);' "$source"
      ;;
    local_to_global_transverse_sign)
      grep -Fq '[-sine, cosine, 0.0, 0.0, 0.0, 0.0],' "$source"
      perl -0pi -e 's/\[-sine, cosine, 0\.0, 0\.0, 0\.0, 0\.0\],/[sine, -cosine, 0.0, 0.0, 0.0, 0.0],/' "$source"
      grep -Fq '[sine, -cosine, 0.0, 0.0, 0.0, 0.0],' "$source"
      ;;
    transverse_mass_factor)
      grep -Fq 'let transverse_mass_factor = density_kg_m3 * area_m2 * length_m / 420.0;' "$source"
      perl -0pi -e 's/let transverse_mass_factor = density_kg_m3 \* area_m2 \* length_m \/ 420\.0;/let transverse_mass_factor = density_kg_m3 * area_m2 * length_m \/ 400.0;/' "$source"
      grep -Fq 'let transverse_mass_factor = density_kg_m3 * area_m2 * length_m / 400.0;' "$source"
      ;;
    rotary_mass_offdiagonal_sign)
      grep -Fq '[36.0, 3.0 * length_m, -36.0, 3.0 * length_m],' "$source"
      perl -0pi -e 's/\[36\.0, 3\.0 \* length_m, -36\.0, 3\.0 \* length_m\],/[36.0, -3.0 * length_m, -36.0, 3.0 * length_m],/' "$source"
      grep -Fq '[36.0, -3.0 * length_m, -36.0, 3.0 * length_m],' "$source"
      ;;
    *)
      printf 'Unknown beam-element mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "beam_element::tests::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: beam-element mutation survived: %s\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  if ! grep -Fq "test beam_element::tests::$test_name ... FAILED" "$log"; then
    printf 'ERROR: mutant %s failed for a reason other than the expected assertion.\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  printf 'KILLED mutation: %s by %s\n' "$label" "$test_name"
}

run_mutation "bending-second-moment-omitted" bending_omits_second_moment bending_stiffness_matches_closed_form_entrywise
run_mutation "bending-coupling-sign" bending_coupling_sign bending_stiffness_matches_closed_form_entrywise
run_mutation "axial-stiffness-area-omitted" axial_stiffness_omits_area axial_stiffness_and_consistent_mass_match_closed_forms
run_mutation "rotary-inertia-omitted" rotary_inertia_omitted rotary_inertia_is_separate_and_scales_with_density_and_second_moment
run_mutation "rotary-inertia-length-exponent" rotary_inertia_wrong_length_exponent rotary_inertia_is_separate_and_scales_with_density_and_second_moment
run_mutation "local-to-global-transverse-sign" local_to_global_transverse_sign rigid_body_translation_and_rotation_have_zero_internal_force
run_mutation "transverse-mass-factor" transverse_mass_factor transverse_translational_mass_matches_closed_form_entrywise
run_mutation "rotary-mass-offdiagonal-sign" rotary_mass_offdiagonal_sign rotary_inertia_mass_matches_closed_form_entrywise

test "$beam_sha256" = "$(sha256sum "$crate_dir/src/beam_element.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg beam_sha256 "$beam_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "beam-element-mutation-receipt-v0.1",
    benchmark_id: "PHYS-BEAM-ELEMENT-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {beam_element_rs: $beam_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "beam_element::tests", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-BEAM-MUT-SECOND-MOMENT-001",
        expected_test: "bending_stiffness_matches_closed_form_entrywise",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-MUT-BENDING-SIGN-001",
        expected_test: "bending_stiffness_matches_closed_form_entrywise",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-MUT-AXIAL-AREA-001",
        expected_test: "axial_stiffness_and_consistent_mass_match_closed_forms",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-MUT-ROTARY-OMITTED-001",
        expected_test: "rotary_inertia_is_separate_and_scales_with_density_and_second_moment",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-MUT-ROTARY-EXPONENT-001",
        expected_test: "rotary_inertia_is_separate_and_scales_with_density_and_second_moment",
        disposition: "detected"
      }
      ,
      {
        id: "PHYS-BEAM-MUT-GLOBAL-TRANSVERSE-SIGN-001",
        expected_test: "rigid_body_translation_and_rotation_have_zero_internal_force",
        disposition: "detected"
      }
      ,
      {
        id: "PHYS-BEAM-MUT-TRANSVERSE-MASS-FACTOR-001",
        expected_test: "transverse_translational_mass_matches_closed_form_entrywise",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-MUT-ROTARY-MASS-SIGN-001",
        expected_test: "rotary_inertia_mass_matches_closed_form_entrywise",
        disposition: "detected"
      }
    ],
    mutation_count: 5,
    detected_count: 5,
    production_source_unchanged: true,
    claim_ceiling: "local planar Euler-Bernoulli element matrix sensitivity only; no global modal or physical validation"
  }' > "$receipt_path"

printf 'PASS beam-element mutation receipt: %s\n' "$receipt_path"
