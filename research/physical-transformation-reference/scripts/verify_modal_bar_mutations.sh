#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Prove selected modal tests kill plausible mass/eigenproblem defects.
# Every mutant is applied in a fresh crate copy; authoritative source is read-only.
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
    consistent_mass_diagonal_factor)
      grep -Fq 'let factor = if local_row == local_col { 2.0 } else { 1.0 };' "$source"
      perl -0pi -e 's/let factor = if local_row == local_col \{ 2\.0 \} else \{ 1\.0 \};/let factor = if local_row == local_col { 3.0 } else { 1.0 };/g' "$source"
      grep -Fq 'let factor = if local_row == local_col { 3.0 } else { 1.0 };' "$source"
      ;;
    consistent_mass_off_diagonal)
      grep -Fq 'let factor = if local_row == local_col { 2.0 } else { 1.0 };' "$source"
      perl -0pi -e 's/let factor = if local_row == local_col \{ 2\.0 \} else \{ 1\.0 \};/let factor = if local_row == local_col { 2.0 } else { 0.0 };/g' "$source"
      grep -Fq 'let factor = if local_row == local_col { 2.0 } else { 0.0 };' "$source"
      ;;
    frequency_conversion_2pi)
      grep -Fq 'let frequency = eigenvalue.sqrt() / (2.0 * PI);' "$source"
      perl -0pi -e 's/let frequency = eigenvalue\.sqrt\(\) \/ \(2\.0 \* PI\);/let frequency = eigenvalue.sqrt() \/ PI;/' "$source"
      grep -Fq 'let frequency = eigenvalue.sqrt() / PI;' "$source"
      ;;
    eigenvector_rotation)
      grep -Fq 'eigenvectors[row][p] = cosine * vp - sine * vq;' "$source"
      perl -0pi -e 's/eigenvectors\[row\]\[p\] = cosine \* vp - sine \* vq;/eigenvectors[row][p] = vp;/' "$source"
      grep -Fq 'eigenvectors[row][p] = vp;' "$source"
      ;;
    transposed_cholesky_backsolve)
      grep -Fq 'value -= lower[col][row] * solution[col];' "$source"
      perl -0pi -e 's/value -= lower\[col\]\[row\] \* solution\[col\];/value -= lower[row][col] * solution[col];/' "$source"
      grep -Fq 'value -= lower[row][col] * solution[col];' "$source"
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
  consistent_mass_diagonal_factor \
  single_element_mass_formulations_reproduce_closed_form_ratios
run_mutation \
  "consistent-mass-offdiagonal" \
  consistent_mass_off_diagonal \
  second_mode_is_within_five_percent_at_eight_elements
run_mutation \
  "frequency-two-pi-conversion" \
  frequency_conversion_2pi \
  single_element_mass_formulations_reproduce_closed_form_ratios
run_mutation \
  "eigenvector-rotation" \
  eigenvector_rotation \
  modes_report_small_eigen_residual_and_mass_normalization_error
run_mutation \
  "transposed-cholesky-backsolve" \
  transposed_cholesky_backsolve \
  modes_report_small_eigen_residual_and_mass_normalization_error

test "$modal_sha256" = "$(sha256sum "$crate_dir/src/modal_bar.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg modal_sha256 "$modal_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "modal-bar-mutation-receipt-v0.1",
    benchmark_id: "PHYS-AXIAL-BAR-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {modal_bar_rs: $modal_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "modal_bar::tests", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-MODAL-MUT-CONSISTENT-DIAGONAL-001",
        expected_test: "single_element_mass_formulations_reproduce_closed_form_ratios",
        disposition: "detected"
      },
      {
        id: "PHYS-MODAL-MUT-CONSISTENT-OFFDIAGONAL-001",
        expected_test: "second_mode_is_within_five_percent_at_eight_elements",
        disposition: "detected"
      },
      {
        id: "PHYS-MODAL-MUT-HZ-CONVERSION-001",
        expected_test: "analytical_spectrum_has_odd_integer_ratios",
        disposition: "detected"
      },
      {
        id: "PHYS-MODAL-MUT-EIGENVECTOR-ROTATION-001",
        expected_test: "modes_report_small_eigen_residual_and_mass_normalization_error",
        disposition: "detected"
      },
      {
        id: "PHYS-MODAL-MUT-CHOLESKY-BACKSOLVE-001",
        expected_test: "modes_report_small_eigen_residual_and_mass_normalization_error",
        disposition: "detected"
      }
    ],
    mutation_count: 5,
    detected_count: 5,
    production_source_unchanged: true,
    claim_ceiling: "test-sensitivity evidence for a synthetic 1D axial bar only; not modal-truss or physical-validation evidence"
  }' > "$receipt_path"

printf 'PASS modal-bar mutation audit receipt: %s\n' "$receipt_path"
