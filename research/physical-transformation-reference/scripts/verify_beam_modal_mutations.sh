#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation-sensitivity gates for the generalized released-joint beam solver.
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
crate_dir="$repo_root/research/physical-transformation-reference"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/beam-modal-mutation-receipt.json"
source_commit="$(git rev-parse HEAD)"
modal_sha256="$(sha256sum "$crate_dir/src/beam_modal.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"
control_suite="beam_modal::tests"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib "$control_suite" >"$control_log" 2>&1; then
  printf 'ERROR: unmutated beam-modal test suite failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: beam-modal eigenvalues, mode counts, residuals and mechanisms\n'

run_mutation() {
  local label="$1"
  local mutation="$2"
  local test_name="$3"
  local mutant_dir="$tmp_root/$label"
  local source="$mutant_dir/src/beam_modal.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
  cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"

  case "$mutation" in
    zero_mode_classified_elastic)
      grep -Fq '} else if eigenvalue <= zero_tolerance {' "$source"
      perl -0pi -e 's/\} else if eigenvalue <= zero_tolerance \{/\} else if eigenvalue < -zero_tolerance \{/' "$source"
      grep -Fq '} else if eigenvalue < -zero_tolerance {' "$source"
      ;;
    reverse_eigenmode_order)
      grep -Fq 'let eigenpairs = generalized_symmetric_eigenpairs(&stiffness_reduced, &mass_reduced)?;' "$source"
      perl -0pi -e 's/let eigenpairs = generalized_symmetric_eigenpairs\(&stiffness_reduced, &mass_reduced\)\?;/let eigenpairs = generalized_symmetric_eigenpairs(\&stiffness_reduced, \&mass_reduced)?.into_iter().rev().collect::<Vec<_>>();/' "$source"
      grep -Fq 'into_iter().rev().collect::<Vec<_>>()' "$source"
      ;;
    drop_one_mode)
      grep -Fq 'let eigenpairs = generalized_symmetric_eigenpairs(&stiffness_reduced, &mass_reduced)?;' "$source"
      perl -0pi -e 's/let eigenpairs = generalized_symmetric_eigenpairs\(&stiffness_reduced, &mass_reduced\)\?;/let eigenpairs = generalized_symmetric_eigenpairs(\&stiffness_reduced, \&mass_reduced)?.into_iter().take(assembly.free_dof_indices.len() - 1).collect::<Vec<_>>();/' "$source"
      grep -Fq 'take(assembly.free_dof_indices.len() - 1)' "$source"
      ;;
    euclidean_instead_of_generalized_mass_norm)
      grep -Fq 'let mass_norm = quadratic_form(&mass_reduced, &shape_free)?;' "$source"
      perl -0pi -e 's/let mass_norm = quadratic_form\(&mass_reduced, &shape_free\)\?;/let mass_norm = shape_free.iter().map(|value| value * value).sum::<f64>();/' "$source"
      grep -Fq 'let mass_norm = shape_free.iter().map(|value| value * value).sum::<f64>();' "$source"
      ;;
    negative_mode_misclassified_as_mechanism)
      grep -Fq 'if eigenvalue < -zero_tolerance {' "$source"
      perl -0pi -e 's/if eigenvalue < -zero_tolerance \{/if eigenvalue < -zero_tolerance \&\& false {/' "$source"
      grep -Fq 'if eigenvalue < -zero_tolerance && false {' "$source"
      ;;
    hermite_rotation_interpolation_sign)
      grep -Fq 'let transverse = h1 * transverse_i' "$source"
      grep -Fq '+ h2 * element_length * rotation_i' "$source"
      perl -0pi -e 's/\+ h2 \* element_length \* rotation_i/- h2 * element_length * rotation_i/' "$source"
      grep -Fq '- h2 * element_length * rotation_i' "$source"
      ;;
    *)
      printf 'Unknown beam modal mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "$control_suite::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: beam-modal mutation survived: %s\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  if ! grep -Fq "test $control_suite::$test_name ... FAILED" "$log"; then
    printf 'ERROR: mutant %s failed outside its named test assertion.\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  printf 'KILLED mutation: %s by %s\n' "$label" "$test_name"
}

run_mutation \
  "zero-mode-misclassification" \
  zero_mode_classified_elastic \
  zero_eigenvalues_are_reported_not_silently_dropped
run_mutation \
  "reverse-mode-order" \
  reverse_eigenmode_order \
  modes_preserve_all_free_dofs_and_have_small_residuals
run_mutation \
  "drop-mode" \
  drop_one_mode \
  modes_preserve_all_free_dofs_and_have_small_residuals
run_mutation \
  "wrong-modal-norm" \
  euclidean_instead_of_generalized_mass_norm \
  modes_preserve_all_free_dofs_and_have_small_residuals
run_mutation \
  "negative-mode-misclassified" \
  negative_mode_misclassified_as_mechanism \
  negative_eigenvalues_are_reported_as_unstable_not_silently_dropped
run_mutation \
  "hermite-rotation-interpolation-sign" \
  hermite_rotation_interpolation_sign \
  fixed_grid_mode_sampling_is_invariant_for_rigid_rotation_across_meshes

test "$modal_sha256" = "$(sha256sum "$crate_dir/src/beam_modal.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg modal_sha256 "$modal_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "beam-modal-mutation-receipt-v0.1",
    benchmark_id: "PHYS-BEAM-MODAL-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {beam_modal_rs: $modal_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "beam_modal::tests", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-BEAM-MOD-NEARZERO-CLASS-001",
        expected_test: "zero_eigenvalues_are_reported_not_silently_dropped",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-MOD-SORT-001",
        expected_test: "modes_preserve_all_free_dofs_and_have_small_residuals",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-MOD-MODE-COUNT-001",
        expected_test: "modes_preserve_all_free_dofs_and_have_small_residuals",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-MOD-GENERALIZED-MASS-001",
        expected_test: "modes_preserve_all_free_dofs_and_have_small_residuals",
        disposition: "detected"
      }
      ,
      {
        id: "PHYS-BEAM-MOD-NEGATIVE-CLASS-001",
        expected_test: "negative_eigenvalues_are_reported_as_unstable_not_silently_dropped",
        disposition: "detected"
      }
      ,
      {
        id: "PHYS-BEAM-MOD-HERMITE-SAMPLING-001",
        expected_test: "fixed_grid_mode_sampling_is_invariant_for_rigid_rotation_across_meshes",
        disposition: "detected"
      }
    ],
    mutation_count: 4,
    detected_count: 4,
    production_source_unchanged: true,
    claim_ceiling: "software test sensitivity for synthetic one-element-per-member generalized modal solve; no refined NAFEMS spectrum or physical validation"
  }' > "$receipt_path"

printf 'PASS beam-modal mutation receipt: %s\n' "$receipt_path"
