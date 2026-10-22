#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation sensitivity for the Timoshenko static beam stiffness reference.
# Mutants exist only in temporary crate copies; production source remains intact.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/timoshenko-element-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
source_sha256="$(sha256sum "$crate_dir/src/timoshenko_element.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib timoshenko_element::tests >"$control_log" 2>&1; then
  printf 'ERROR: unmutated Timoshenko tests failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: Timoshenko element tests\n'

run_mutation() {
  local label="$1"
  local mutation="$2"
  local test_name="$3"
  local mutant_dir="$tmp_root/$label"
  local source="$mutant_dir/src/timoshenko_element.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
  cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"

  case "$mutation" in
    zero_shear_flexibility)
      grep -Fq 'let shear_flexibility_phi = 12.0 * youngs_modulus_pa * second_moment_m4' "$source"
      perl -0pi -e 's/let shear_flexibility_phi = 12\.0 \* youngs_modulus_pa \* second_moment_m4\n        \/ \(shear_correction_factor \* shear_modulus_pa \* area_m2 \* l2\);/let shear_flexibility_phi = 0.0;/' "$source"
      grep -Fq 'let shear_flexibility_phi = 0.0;' "$source"
      ;;
    omit_shear_correction_factor)
      grep -Fq '(shear_correction_factor * shear_modulus_pa * area_m2 * l2)' "$source"
      perl -0pi -e 's/\(shear_correction_factor \* shear_modulus_pa \* area_m2 \* l2\)/(shear_modulus_pa * area_m2 * l2)/' "$source"
      grep -Fq '(shear_modulus_pa * area_m2 * l2)' "$source"
      ;;
    omit_area_from_shear_stiffness)
      grep -Fq '(shear_correction_factor * shear_modulus_pa * area_m2 * l2)' "$source"
      perl -0pi -e 's/\(shear_correction_factor \* shear_modulus_pa \* area_m2 \* l2\)/(shear_correction_factor * shear_modulus_pa * l2)/' "$source"
      grep -Fq '(shear_correction_factor * shear_modulus_pa * l2)' "$source"
      ;;
    *)
      printf 'Unknown Timoshenko mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "timoshenko_element::tests::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: Timoshenko element mutation survived: %s\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  if ! grep -Fq "test timoshenko_element::tests::$test_name ... FAILED" "$log"; then
    printf 'ERROR: mutation %s failed for a reason other than the expected assertion.\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  printf 'KILLED mutation: %s by %s\n' "$label" "$test_name"
}

run_mutation "shear-flexibility-zeroed" zero_shear_flexibility cantilever_tip_compliance_matches_timoshenko_analytical_solution
run_mutation "shear-correction-factor-ignored" omit_shear_correction_factor shear_correction_factor_changes_shear_flexibility_and_deflection
run_mutation "shear-area-omitted" omit_area_from_shear_stiffness cantilever_tip_compliance_matches_timoshenko_analytical_solution

test "$source_sha256" = "$(sha256sum "$crate_dir/src/timoshenko_element.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg source_sha256 "$source_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "timoshenko-element-mutation-receipt-v0.1",
    benchmark_id: "PHYS-TIMOSHENKO-ELEMENT-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {timoshenko_element_rs: $source_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "timoshenko_element::tests", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-TIMO-MUT-SHEAR-ZERO-001",
        expected_test: "cantilever_tip_compliance_matches_timoshenko_analytical_solution",
        disposition: "detected"
      },
      {
        id: "PHYS-TIMO-MUT-KAPPA-001",
        expected_test: "shear_correction_factor_changes_shear_flexibility_and_deflection",
        disposition: "detected"
      },
      {
        id: "PHYS-TIMO-MUT-AREA-001",
        expected_test: "cantilever_tip_compliance_matches_timoshenko_analytical_solution",
        disposition: "detected"
      }
    ],
    mutation_count: 3,
    detected_count: 3,
    production_source_unchanged: true,
    claim_ceiling: "static local Timoshenko stiffness test sensitivity only; no modal, physical validation or structural safety claim"
  }' > "$receipt_path"

printf 'PASS Timoshenko element mutation receipt: %s\n' "$receipt_path"
