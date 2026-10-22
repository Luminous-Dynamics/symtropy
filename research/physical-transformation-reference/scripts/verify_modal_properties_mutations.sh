#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation sensitivity for effective modal mass and directional participation.
# Mutants are applied only to isolated temporary copies; production source is fixed.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/modal-properties-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
source_sha256="$(sha256sum "$crate_dir/src/modal_properties.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"

control_log="$tmp_root/control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib modal_properties::tests >"$control_log" 2>&1; then
  printf 'ERROR: unmutated modal-properties controls failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated modal-properties tests\n'

run_mutation() {
  local label="$1"
  local mutation="$2"
  local test_name="$3"
  local mutant_dir="$tmp_root/$label"
  local source="$mutant_dir/src/modal_properties.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
  cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"

  case "$mutation" in
    remove_generalized_mass_denominator)
      grep -Fq 'let effective_mass_x = projection_x * projection_x / generalized_mass;' "$source"
      perl -0pi -e 's/let effective_mass_x = projection_x \* projection_x \/ generalized_mass;/let effective_mass_x = projection_x * projection_x;/' "$source"
      grep -Fq 'let effective_mass_x = projection_x * projection_x;' "$source"
      ;;
    reuse_x_influence_for_y)
      grep -Fq 'let mass_influence_y = matrix_vector_product(&assembly.mass_global, &influence_y)?;' "$source"
      perl -0pi -e 's/let mass_influence_y = matrix_vector_product\(&assembly\.mass_global, &influence_y\)\?;/let mass_influence_y = matrix_vector_product(\&assembly.mass_global, \&influence_x)?;/' "$source"
      grep -Fq 'let mass_influence_y = matrix_vector_product(&assembly.mass_global, &influence_x)?;' "$source"
      ;;
    skip_complete_spectrum_gate)
      grep -Fq 'if modal_result.modes.len() != assembly.free_dof_indices.len() {' "$source"
      perl -0pi -e 's/if modal_result\.modes\.len\(\) != assembly\.free_dof_indices\.len\(\) \{/if false {/' "$source"
      grep -Fq 'if false {' "$source"
      ;;
    *)
      printf 'Unknown modal-properties mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "modal_properties::tests::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: modal-properties mutation survived: %s\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  if ! grep -Fq "test modal_properties::tests::$test_name ... FAILED" "$log"; then
    printf 'ERROR: mutant %s failed for a reason other than the expected assertion.\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  printf 'KILLED mutation: %s by %s\n' "$label" "$test_name"
}

run_mutation \
  "effective-mass-generalized-mass-denominator-removed" \
  remove_generalized_mass_denominator \
  participation_is_invariant_to_arbitrary_mode_vector_scaling
run_mutation \
  "y-participation-reuses-x-influence" \
  reuse_x_influence_for_y \
  orthogonal_directional_modes_report_complete_effective_mass
run_mutation \
  "incomplete-spectrum-gate-removed" \
  skip_complete_spectrum_gate \
  malformed_or_inconsistent_modal_inputs_fail_closed

test "$source_sha256" = "$(sha256sum "$crate_dir/src/modal_properties.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg source_sha256 "$source_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "modal-properties-mutation-receipt-v0.1",
    benchmark_id: "PHYS-MODAL-PROPERTIES-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {modal_properties_rs: $source_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "modal_properties::tests", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-MODAL-PROPERTIES-MUT-GENERALIZED-MASS-001",
        expected_test: "participation_is_invariant_to_arbitrary_mode_vector_scaling",
        disposition: "detected"
      },
      {
        id: "PHYS-MODAL-PROPERTIES-MUT-DIRECTION-001",
        expected_test: "orthogonal_directional_modes_report_complete_effective_mass",
        disposition: "detected"
      },
      {
        id: "PHYS-MODAL-PROPERTIES-MUT-SPECTRUM-COVERAGE-001",
        expected_test: "malformed_or_inconsistent_modal_inputs_fail_closed",
        disposition: "detected"
      }
    ],
    mutation_count: 2,
    detected_count: 2,
    production_source_unchanged: true,
    claim_ceiling: "synthetic effective modal mass and X/Y participation formulas only; no full physical validation, structural safety or code compliance"
  }' > "$receipt_path"

printf 'PASS modal-properties mutation receipt: %s\n' "$receipt_path"
