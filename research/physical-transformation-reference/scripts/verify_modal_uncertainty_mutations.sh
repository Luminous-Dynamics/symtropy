#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation sensitivity for deterministic modal uncertainty propagation.
# Each mutant exists in a temporary copy; production source is not edited.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/modal-uncertainty-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
source_sha256="$(sha256sum "$crate_dir/src/modal_uncertainty.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"
test_name="elastic_mode_sensitivity_matches_exact_material_scaling_laws"

control_log="$tmp_root/control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib "modal_uncertainty::tests" >"$control_log" 2>&1; then
  printf 'ERROR: unmutated modal uncertainty tests failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated modal uncertainty tests\n'

run_mutation() {
  local label="$1"
  local mutation="$2"
  local mutant_dir="$tmp_root/$label"
  local source="$mutant_dir/src/modal_uncertainty.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
  cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"

  case "$mutation" in
    invert_plus_minus_frequency_ratio)
      grep -Fq '(plus_match.frequency_hz / minus_match.frequency_hz).ln() / denominator;' "$source"
      perl -0pi -e 's{\(plus_match\.frequency_hz / minus_match\.frequency_hz\)\.ln\(\) / denominator;}{(minus_match.frequency_hz / plus_match.frequency_hz).ln() / denominator;}' "$source"
      grep -Fq '(minus_match.frequency_hz / plus_match.frequency_hz).ln() / denominator;' "$source"
      ;;
    propagate_signed_frequency_sensitivity)
      grep -Fq 'logarithmic_sensitivity.abs() * spec.assumed_relative_uncertainty,' "$source"
      perl -0pi -e 's/logarithmic_sensitivity\.abs\(\) \* spec\.assumed_relative_uncertainty,/logarithmic_sensitivity * spec.assumed_relative_uncertainty,/' "$source"
      grep -Fq 'logarithmic_sensitivity * spec.assumed_relative_uncertainty,' "$source"
      ;;
    *)
      printf 'Unknown modal uncertainty mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "modal_uncertainty::tests::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: modal uncertainty mutation survived: %s\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  if ! grep -Fq "test modal_uncertainty::tests::$test_name ... FAILED" "$log"; then
    printf 'ERROR: mutant %s failed outside its expected sensitivity assertion.\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  printf 'KILLED mutation: %s by %s\n' "$label" "$test_name"
}

run_mutation "invert-central-log-sensitivity" invert_plus_minus_frequency_ratio
run_mutation "signed-uncertainty-contribution" propagate_signed_frequency_sensitivity

test "$source_sha256" = "$(sha256sum "$crate_dir/src/modal_uncertainty.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg source_sha256 "$source_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "modal-uncertainty-mutation-receipt-v0.1",
    benchmark_id: "PHYS-BEAM-MODAL-UQ-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {modal_uncertainty_rs: $source_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "modal_uncertainty::tests", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-BEAM-MODAL-UQ-MUT-SENSITIVITY-SIGN-001",
        expected_test: "elastic_mode_sensitivity_matches_exact_material_scaling_laws",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-MODAL-UQ-MUT-CONTRIBUTION-ABS-001",
        expected_test: "elastic_mode_sensitivity_matches_exact_material_scaling_laws",
        disposition: "detected"
      }
    ],
    mutation_count: 2,
    detected_count: 2,
    production_source_unchanged: true,
    claim_ceiling: "finite-difference modal sensitivity test sensitivity only; not probabilistic uncertainty quantification or physical validation"
  }' > "$receipt_path"

printf 'PASS modal uncertainty mutation receipt: %s\n' "$receipt_path"
