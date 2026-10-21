#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Prove that modal verification detects representative mass/stiffness defects.
# Every mutant lives in an isolated temporary crate; production source is immutable.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/modal-bar-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
modal_sha256="$(sha256sum "$crate_dir/src/modal_bar.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib modal_bar::tests >"$control_log" 2>&1; then
  printf 'ERROR: unmutated modal-bar unit-test control failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated modal-bar unit tests\n'

run_mutation() {
  local label="$1"
  local mutation="$2"
  local test_name="$3"
  local mutant_dir="$tmp_root/$label"
  local source="$mutant_dir/src/modal_bar.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
  cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"

  case "$mutation" in
    consistent_mass_diagonal_wrong)
      grep -Fq 'mass[right][right] += element_mass / 3.0;' "$source"
      perl -0pi -e 's/mass\[right\]\[right\] \+= element_mass \/ 3\.0;/mass[right][right] += element_mass \/ 2.0;/' "$source"
      grep -Fq 'mass[right][right] += element_mass / 2.0;' "$source"
      ;;
    lumped_mass_diagonal_wrong)
      grep -Fq 'mass[right][right] += element_mass / 2.0;' "$source"
      perl -0pi -e 's/mass\[right\]\[right\] \+= element_mass \/ 2\.0;/mass[right][right] += element_mass \/ 3.0;/' "$source"
      grep -Fq 'mass[right][right] += element_mass / 3.0;' "$source"
      ;;
    axial_stiffness_wrong_length_exponent)
      grep -Fq 'let element_stiffness = bar.youngs_modulus_pa * bar.area_m2 / element_length;' "$source"
      perl -0pi -e 's/let element_stiffness = bar\.youngs_modulus_pa \* bar\.area_m2 \/ element_length;/let element_stiffness = bar.youngs_modulus_pa * bar.area_m2 \/ (element_length * element_length);/' "$source"
      grep -Fq 'bar.area_m2 / (element_length * element_length)' "$source"
      ;;
    consistent_mass_offdiagonal_omitted)
      grep -Fq 'mass[left][right] += element_mass / 6.0;' "$source"
      grep -Fq 'mass[right][left] += element_mass / 6.0;' "$source"
      perl -0pi -e 's/mass\[left\]\[right\] \+= element_mass \/ 6\.0;/mass[left][right] += 0.0;/' "$source"
      perl -0pi -e 's/mass\[right\]\[left\] \+= element_mass \/ 6\.0;/mass[right][left] += 0.0;/' "$source"
      grep -Fq 'mass[left][right] += 0.0;' "$source"
      grep -Fq 'mass[right][left] += 0.0;' "$source"
      ;;
    analytical_wave_speed_omits_density)
      grep -Fq 'let wave_speed = (bar.youngs_modulus_pa / bar.density_kg_m3).sqrt();' "$source"
      perl -0pi -e 's/let wave_speed = \(bar\.youngs_modulus_pa \/ bar\.density_kg_m3\)\.sqrt\(\);/let wave_speed = bar.youngs_modulus_pa.sqrt();/' "$source"
      grep -Fq 'let wave_speed = bar.youngs_modulus_pa.sqrt();' "$source"
      ;;
    *)
      printf 'Unknown modal-bar mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "modal_bar::tests::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: modal-bar mutation survived: %s\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  if ! grep -Fq "test modal_bar::tests::$test_name ... FAILED" "$log"; then
    printf 'ERROR: mutant %s failed for a reason other than the expected assertion.\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  printf 'KILLED mutation: %s by %s\n' "$label" "$test_name"
}

run_mutation \
  "consistent-mass-diagonal" \
  consistent_mass_diagonal_wrong \
  one_element_brackets_exact_fixed_free_frequency_for_both_mass_models
run_mutation \
  "lumped-mass-diagonal" \
  lumped_mass_diagonal_wrong \
  one_element_brackets_exact_fixed_free_frequency_for_both_mass_models
run_mutation \
  "modal-stiffness-length-exponent" \
  axial_stiffness_wrong_length_exponent \
  frequency_is_invariant_to_area_and_joint_scaling_of_modulus_and_density
run_mutation \
  "consistent-mass-offdiagonal" \
  consistent_mass_offdiagonal_omitted \
  second_mode_converges_toward_independent_continuum_value
run_mutation \
  "continuum-wave-speed-density" \
  analytical_wave_speed_omits_density \
  first_mode_converges_from_opposite_sides_under_mesh_refinement

test "$modal_sha256" = "$(sha256sum "$crate_dir/src/modal_bar.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg modal_sha256 "$modal_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "modal-bar-mutation-receipt-v0.1",
    benchmark_id: "PHYS-MODAL-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {modal_bar_rs: $modal_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "modal_bar::tests", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-MODAL-MUT-CONSISTENT-MASS-DIAG-001",
        expected_test: "one_element_brackets_exact_fixed_free_frequency_for_both_mass_models",
        disposition: "detected"
      },
      {
        id: "PHYS-MODAL-MUT-LUMPED-MASS-DIAG-001",
        expected_test: "one_element_brackets_exact_fixed_free_frequency_for_both_mass_models",
        disposition: "detected"
      },
      {
        id: "PHYS-MODAL-MUT-STIFFNESS-LENGTH-001",
        expected_test: "frequency_is_invariant_to_area_and_joint_scaling_of_modulus_and_density",
        disposition: "detected"
      },
      {
        id: "PHYS-MODAL-MUT-MASS-OFFDIAGONAL-001",
        expected_test: "second_mode_converges_toward_independent_continuum_value",
        disposition: "detected"
      },
      {
        id: "PHYS-MODAL-MUT-WAVE-SPEED-DENSITY-001",
        expected_test: "first_mode_converges_from_opposite_sides_under_mesh_refinement",
        disposition: "detected"
      }
    ],
    summary: {
      detected: 5,
      undetected: 0,
      invalid_mutant: 0,
      equivalent_mutant: 0,
      control_passed: true
    }
  }' > "$receipt_path"

jq -e '
  .schema_version == "modal-bar-mutation-receipt-v0.1"
  and (.source_commit | type == "string" and length == 40)
  and .baseline_control.disposition == "pass"
  and .summary.detected == 5
  and .summary.undetected == 0
  and ([.mutations[].disposition] | all(. == "detected"))
' "$receipt_path" >/dev/null

printf 'Modal-bar mutation sensitivity: 5/5 named defects detected. Receipt: %s\n' "$receipt_path"
