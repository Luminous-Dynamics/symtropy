#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation-sensitivity audit for the released-joint generalized modal layer.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/beam-modal-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
modal_sha256="$(sha256sum "$crate_dir/src/beam_modal.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"
test_suite="beam_modal::tests"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib "$test_suite" >"$control_log" 2>&1; then
  printf 'ERROR: unmutated beam-modal tests failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated beam-modal tests\n'

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
    wrong_hz_conversion)
      grep -Fq 'let frequency_hz = eigenvalue.sqrt() / (2.0 * PI);' "$source"
      perl -0pi -e 's/let frequency_hz = eigenvalue\.sqrt\(\) \/ \(2\.0 \* PI\);/let frequency_hz = eigenvalue.sqrt() \/ PI;/' "$source"
      grep -Fq 'let frequency_hz = eigenvalue.sqrt() / PI;' "$source"
      ;;
    suppress_mechanism_detection)
      grep -Fq '*eigenvalue <= mechanism_floor' "$source"
      perl -0pi -e 's/\*eigenvalue <= mechanism_floor/*eigenvalue < 0.0/' "$source"
      grep -Fq '*eigenvalue < 0.0' "$source"
      ;;
    wrong_mass_norm_residual)
      grep -Fq 'let mass_norm_residual = (mass_norm - 1.0).abs();' "$source"
      perl -0pi -e 's/let mass_norm_residual = \(mass_norm - 1\.0\)\.abs\(\);/let mass_norm_residual = mass_norm.abs();/' "$source"
      grep -Fq 'let mass_norm_residual = mass_norm.abs();' "$source"
      ;;
    *)
      printf 'Unknown beam-modal mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "$test_suite::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: beam-modal mutation survived: %s\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  if ! grep -Fq "test $test_suite::$test_name ... FAILED" "$log"; then
    printf 'ERROR: mutant %s failed for a reason other than the named assertion.\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  printf 'KILLED mutation: %s by %s\n' "$label" "$test_name"
}

run_mutation \
  "wrong-frequency-conversion" \
  wrong_hz_conversion \
  two_free_beam_rotations_match_independent_closed_form_generalized_eigenvalues
run_mutation \
  "mechanism-detection-disabled" \
  suppress_mechanism_detection \
  zero_modes_are_reported_as_unqualified_not_silently_filtered
run_mutation \
  "wrong-mass-normalization-diagnostic" \
  wrong_mass_norm_residual \
  six_modes_are_sorted_positive_and_have_small_eigenpair_residuals

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
        id: "PHYS-BEAM-MOD-HZ-CONVERSION-001",
        expected_test: "two_free_beam_rotations_match_independent_closed_form_generalized_eigenvalues",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-MOD-MECHANISM-001",
        expected_test: "zero_modes_are_reported_as_unqualified_not_silently_filtered",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-MOD-MASS-NORM-001",
        expected_test: "six_modes_are_sorted_positive_and_have_small_eigenpair_residuals",
        disposition: "detected"
      }
    ],
    mutation_count: 3,
    detected_count: 3,
    production_source_unchanged: true,
    claim_ceiling: "small coarse released-joint beam modal layer sensitivity only; no converged NAFEMS reproduction or physical validation"
  }' > "$receipt_path"

printf 'PASS beam-modal mutation receipt: %s\n' "$receipt_path"
