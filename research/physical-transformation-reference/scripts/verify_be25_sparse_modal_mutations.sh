#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation audit for the banded continuous-beam Lanczos path.
# Production code remains untouched; mutants exist only in temporary copies.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/sofistik-be25-sparse-modal-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
solver_sha256="$(sha256sum "$crate_dir/src/beam_line_sparse.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib beam_line_sparse::tests >"$control_log" 2>&1; then
  printf 'ERROR: unmutated sparse beam modal test control failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated sparse beam modal tests\n'

run_mutation() {
  local label="$1"
  local mutation="$2"
  local test_name="$3"
  local mutant_dir="$tmp_root/$label"
  local source="$mutant_dir/src/beam_line_sparse.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
  cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"

  case "$mutation" in
    consistent_mass_factor)
      grep -Fq 'let mass_factor = mass_per_length_kg_per_m * h / 420.0;' "$source"
      perl -0pi -e 's/let mass_factor = mass_per_length_kg_per_m \* h \/ 420\.0;/let mass_factor = mass_per_length_kg_per_m * h \/ 400.0;/' "$source"
      grep -Fq 'let mass_factor = mass_per_length_kg_per_m * h / 400.0;' "$source"
      ;;
    stiffness_length_exponent)
      grep -Fq 'let stiffness_factor = youngs_modulus_pa * second_moment_m4 / h3;' "$source"
      perl -0pi -e 's/let stiffness_factor = youngs_modulus_pa \* second_moment_m4 \/ h3;/let stiffness_factor = youngs_modulus_pa * second_moment_m4 \/ h2;/' "$source"
      grep -Fq 'let stiffness_factor = youngs_modulus_pa * second_moment_m4 / h2;' "$source"
      ;;
    omit_symmetric_matvec_contribution)
      grep -Fq 'result[col] += value * vector[row];' "$source"
      perl -0pi -e 's/result\[col\] \+= value \* vector\[row\];/result[col] += 0.0 * vector[row];/' "$source"
      grep -Fq 'result[col] += 0.0 * vector[row];' "$source"
      ;;
    rotational_mode_shape_sign)
      grep -Fq 'let physical_scale = if global_index % 2 == 0 { 1.0 } else { 1.0 / element_length_m };' "$source"
      perl -0pi -e 's#let physical_scale = if global_index % 2 == 0 \{ 1\.0 \} else \{ 1\.0 / element_length_m \};#let physical_scale = if global_index % 2 == 0 { 1.0 } else { -1.0 / element_length_m };#' "$source"
      grep -Fq 'let physical_scale = if global_index % 2 == 0 { 1.0 } else { -1.0 / element_length_m };' "$source"
      ;;
    *)
      printf 'Unknown sparse beam modal mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "beam_line_sparse::tests::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: sparse beam modal mutation survived: %s\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  if ! grep -Fq "test beam_line_sparse::tests::$test_name ... FAILED" "$log"; then
    printf 'ERROR: mutant %s failed for a reason other than the intended assertion.\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  printf 'KILLED mutation: %s by %s\n' "$label" "$test_name"
}

run_mutation \
  "consistent-mass-factor" \
  consistent_mass_factor \
  banded_lanczos_matches_dense_solver_for_all_boundary_cases
run_mutation \
  "stiffness-length-exponent" \
  stiffness_length_exponent \
  banded_lanczos_matches_dense_solver_for_all_boundary_cases
run_mutation \
  "symmetric-matvec-missing-half" \
  omit_symmetric_matvec_contribution \
  banded_lanczos_matches_dense_solver_for_all_boundary_cases
run_mutation \
  "rotational-mode-shape-sign" \
  rotational_mode_shape_sign \
  banded_lanczos_matches_dense_solver_for_all_boundary_cases

test "$solver_sha256" = "$(sha256sum "$crate_dir/src/beam_line_sparse.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg solver_sha256 "$solver_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "sofistik-be25-sparse-modal-mutation-receipt-v0.1",
    benchmark_id: "PHYS-BE25-SPARSE-MODAL-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {beam_line_sparse_rs: $solver_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "beam_line_sparse::tests", disposition: "pass"},
    mutations: [
      {
        id: "BE25-SPARSE-MUT-MASS-FACTOR-001",
        expected_test: "banded_lanczos_matches_dense_solver_for_all_boundary_cases",
        disposition: "detected"
      },
      {
        id: "BE25-SPARSE-MUT-STIFFNESS-EXPONENT-001",
        expected_test: "banded_lanczos_matches_dense_solver_for_all_boundary_cases",
        disposition: "detected"
      },
      {
        id: "BE25-SPARSE-MUT-MATVEC-SYMMETRY-001",
        expected_test: "banded_lanczos_matches_dense_solver_for_all_boundary_cases",
        disposition: "detected"
      },
      {
        id: "BE25-SPARSE-MUT-ROTATION-SHAPE-SIGN-001",
        expected_test: "banded_lanczos_matches_dense_solver_for_all_boundary_cases",
        disposition: "detected"
      }
    ],
    mutation_count: 4,
    detected_count: 4,
    production_source_unchanged: true,
    claim_ceiling: "sparse candidate solver software sensitivity only; no BE25 reproduction"
  }' > "$receipt_path"

printf 'PASS sparse beam modal mutation receipt: %s\n' "$receipt_path"
