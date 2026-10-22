#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation-sensitivity audit for the beam-theory applicability screen.
# All source mutations occur in isolated temporary crate copies.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/beam-theory-applicability-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
source_sha256="$(sha256sum "$crate_dir/src/beam_theory_applicability.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib beam_theory_applicability::tests >"$control_log" 2>&1; then
  printf 'ERROR: unmutated beam-theory applicability test control failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: beam-theory applicability screen tests\n'

run_mutation() {
  local label="$1"
  local mutation="$2"
  local test_name="$3"
  local mutant_dir="$tmp_root/$label"
  local source="$mutant_dir/src/beam_theory_applicability.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
  cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"

  case "$mutation" in
    ignore_modal_wavelength)
      grep -Fq '.map_or(input.member_length_m, |wavelength| {' "$source"
      perl -0pi -e 's/\.map_or\(input\.member_length_m, \|wavelength\| \{\n            input\.member_length_m\.min\(wavelength\)\n        \}\);/.map_or(input.member_length_m, |_wavelength| input.member_length_m);/' "$source"
      grep -Fq '.map_or(input.member_length_m, |_wavelength| input.member_length_m);' "$source"
      ;;
    missing_wavelength_fails_open)
      grep -Fq 'input.purpose == BeamAnalysisPurpose::Modal' "$source"
      perl -0pi -e 's/let wavelength_required_but_missing =\n        input\.purpose == BeamAnalysisPurpose::Modal\n            && input\.shortest_relevant_wavelength_m\.is_none\(\);/let wavelength_required_but_missing = false;/' "$source"
      grep -Fq 'let wavelength_required_but_missing = false;' "$source"
      ;;
    wrong_wavelength_length_exponent)
      grep -Fq 'effective_axial_scale_m.powi(2)' "$source"
      perl -0pi -e 's/effective_axial_scale_m\.powi\(2\)/effective_axial_scale_m.powi(1)/' "$source"
      grep -Fq 'effective_axial_scale_m.powi(1)' "$source"
      ;;
    shear_comparison_threshold)
      grep -Fq 'const SHEAR_SLENDERNESS_COMPARISON_LIMIT: f64 = 30.0;' "$source"
      perl -0pi -e 's/const SHEAR_SLENDERNESS_COMPARISON_LIMIT: f64 = 30\.0;/const SHEAR_SLENDERNESS_COMPARISON_LIMIT: f64 = 300.0;/' "$source"
      grep -Fq 'const SHEAR_SLENDERNESS_COMPARISON_LIMIT: f64 = 300.0;' "$source"
      ;;
    cross_section_outside_threshold)
      grep -Fq 'const OUTSIDE_REFERENCE_CROSS_SECTION_RATIO: f64 = 1.0 / 8.0;' "$source"
      perl -0pi -e 's/const OUTSIDE_REFERENCE_CROSS_SECTION_RATIO: f64 = 1\.0 \/ 8\.0;/const OUTSIDE_REFERENCE_CROSS_SECTION_RATIO: f64 = 1.0 \/ 2.0;/' "$source"
      grep -Fq 'const OUTSIDE_REFERENCE_CROSS_SECTION_RATIO: f64 = 1.0 / 2.0;' "$source"
      ;;
    *)
      printf 'Unknown beam-theory applicability mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "beam_theory_applicability::tests::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: beam-theory applicability mutation survived: %s\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  if ! grep -Fq "test beam_theory_applicability::tests::$test_name ... FAILED" "$log"; then
    printf 'ERROR: mutant %s failed for a reason other than the expected test assertion.\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  printf 'KILLED mutation: %s by %s\n' "$label" "$test_name"
}

run_mutation "modal-wavelength-ignored" ignore_modal_wavelength high_frequency_short_wavelength_can_reject_a_globally_slender_member
run_mutation "missing-modal-wavelength-fails-open" missing_wavelength_fails_open missing_modal_wavelength_stays_unqualified_instead_of_crashing
run_mutation "wrong-effective-length-exponent" wrong_wavelength_length_exponent uniform_geometric_rescaling_preserves_dimensionless_decision
run_mutation "shear-threshold-loosened" shear_comparison_threshold shear_slenderness_thresholds_are_separate_and_tested
run_mutation "cross-section-threshold-loosened" cross_section_outside_threshold high_frequency_short_wavelength_can_reject_a_globally_slender_member

test "$source_sha256" = "$(sha256sum "$crate_dir/src/beam_theory_applicability.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg source_sha256 "$source_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "beam-theory-applicability-mutation-receipt-v0.1",
    benchmark_id: "PHYS-BEAM-THEORY-SCREEN-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {beam_theory_applicability_rs: $source_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "beam_theory_applicability::tests", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-BEAM-SCREEN-MUT-WAVELENGTH-001",
        expected_test: "high_frequency_short_wavelength_can_reject_a_globally_slender_member",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-SCREEN-MUT-MISSING-WAVELENGTH-001",
        expected_test: "missing_modal_wavelength_stays_unqualified_instead_of_crashing",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-SCREEN-MUT-LENGTH-EXPONENT-001",
        expected_test: "uniform_geometric_rescaling_preserves_dimensionless_decision",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-SCREEN-MUT-SHEAR-THRESHOLD-001",
        expected_test: "shear_slenderness_thresholds_are_separate_and_tested",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-SCREEN-MUT-CROSS-SECTION-THRESHOLD-001",
        expected_test: "high_frequency_short_wavelength_can_reject_a_globally_slender_member",
        disposition: "detected"
      }
    ],
    mutation_count: 5,
    detected_count: 5,
    production_source_unchanged: true,
    claim_ceiling: "screen-case mutation sensitivity only; no solver accuracy or physical validation"
  }' > "$receipt_path"

printf 'PASS beam-theory applicability mutation receipt: %s\n' "$receipt_path"
