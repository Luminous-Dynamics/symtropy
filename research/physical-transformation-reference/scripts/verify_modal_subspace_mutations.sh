#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation sensitivity for basis-invariant, mass-weighted modal subspace comparison.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/modal-subspace-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
source_sha256="$(sha256sum "$crate_dir/src/modal_subspace.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"

control_log="$tmp_root/control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib modal_subspace::tests >"$control_log" 2>&1; then
  printf 'ERROR: unmutated modal-subspace controls failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated modal-subspace tests\n'

run_mutation() {
  local label="$1"
  local mutation="$2"
  local test_name="$3"
  local mutant_dir="$tmp_root/$label"
  local source="$mutant_dir/src/modal_subspace.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
  cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"

  case "$mutation" in
    omit_gram_whitening)
      grep -Fq 'let normalized_cross = multiply(' "$source"
      grep -Fq '&multiply(&left_inverse_sqrt, &cross_gram),' "$source"
      perl -0pi -e 's/let normalized_cross = multiply\(\n        &multiply\(&left_inverse_sqrt, &cross_gram\),\n        &right_inverse_sqrt,\n    \);/let normalized_cross = cross_gram.clone();/' "$source"
      grep -Fq 'let normalized_cross = cross_gram.clone();' "$source"
      ;;
    ignore_mass_metric)
      grep -Fq 'let mass_right = matrix_vector_product(mass, right_mode)?;' "$source"
      perl -0pi -e 's/let mass_right = matrix_vector_product\(mass, right_mode\)\?;/let mass_right = right_mode.clone();/' "$source"
      grep -Fq 'let mass_right = right_mode.clone();' "$source"
      ;;
    absolute_frequency_gap)
      grep -Fq 'let relative_gap = (current - previous).abs() / current.abs().max(previous.abs());' "$source"
      perl -0pi -e 's/let relative_gap = \(current - previous\)\.abs\(\) \/ current\.abs\(\)\.max\(previous\.abs\(\)\);/let relative_gap = (current - previous).abs();/' "$source"
      grep -Fq 'let relative_gap = (current - previous).abs();' "$source"
      ;;
    *)
      printf 'Unknown modal-subspace mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "modal_subspace::tests::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: modal-subspace mutation survived: %s\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  if ! grep -Fq "test modal_subspace::tests::$test_name ... FAILED" "$log"; then
    printf 'ERROR: mutant %s failed for a reason other than the expected assertion.\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  printf 'KILLED mutation: %s by %s\n' "$label" "$test_name"
}

run_mutation "subspace-gram-whitening-omitted" omit_gram_whitening repeated_mode_cluster_is_invariant_to_basis_rotation_and_scaling
run_mutation "mass-metric-ignored" ignore_mass_metric comparison_uses_the_mass_inner_product_not_euclidean_mac
run_mutation "absolute-not-relative-frequency-gap" absolute_frequency_gap frequency_clusters_use_scale_invariant_predeclared_relative_gap

test "$source_sha256" = "$(sha256sum "$crate_dir/src/modal_subspace.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg source_sha256 "$source_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "modal-subspace-mutation-receipt-v0.1",
    benchmark_id: "PHYS-MODAL-SUBSPACE-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {modal_subspace_rs: $source_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "modal_subspace::tests", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-MODAL-SUBSPACE-MUT-WHITENING-001",
        expected_test: "repeated_mode_cluster_is_invariant_to_basis_rotation_and_scaling",
        disposition: "detected"
      },
      {
        id: "PHYS-MODAL-SUBSPACE-MUT-MASS-METRIC-001",
        expected_test: "comparison_uses_the_mass_inner_product_not_euclidean_mac",
        disposition: "detected"
      }
      ,
      {
        id: "PHYS-MODAL-SUBSPACE-MUT-FREQ-GAP-001",
        expected_test: "frequency_clusters_use_scale_invariant_predeclared_relative_gap",
        disposition: "detected"
      }
    ],
    mutation_count: 2,
    detected_count: 2,
    production_source_unchanged: true,
    claim_ceiling: "basis-invariant software comparison of equal-dimensional modal subspaces only; no full structural validation"
  }' > "$receipt_path"

printf 'PASS modal-subspace mutation receipt: %s\n' "$receipt_path"
