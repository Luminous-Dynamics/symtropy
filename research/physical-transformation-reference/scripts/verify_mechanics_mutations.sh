#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Verify that selected metamorphic tests reject deliberately broken mechanics
# formulas. All mutations occur in disposable copies; production source is never edited.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT

run_mutation() {
  local label="$1"
  local mutant="$2"
  local test_name="$3"
  local mutant_dir="$tmp_root/$label"
  local mechanics="$mutant_dir/src/mechanics.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp -a "$crate_dir/." "$mutant_dir/"

  case "$mutant" in
    axial_extension_length_squared)
      grep -Fq 'let numerator = force_n * length_m;' "$mechanics"
      perl -0pi -e 's/let numerator = force_n \* length_m;/let numerator = force_n * length_m * length_m;/' "$mechanics"
      grep -Fq 'let numerator = force_n * length_m * length_m;' "$mechanics"
      ;;
    cantilever_deflection_length_squared)
      grep -Fq 'let deflection_numerator = force_n * length_cubed;' "$mechanics"
      perl -0pi -e 's/let deflection_numerator = force_n \* length_cubed;/let deflection_numerator = force_n * length_squared;/' "$mechanics"
      grep -Fq 'let deflection_numerator = force_n * length_squared;' "$mechanics"
      ;;
    supported_beam_deflection_length_cubed)
      grep -Fq 'let length_fourth = length_squared * length_squared;' "$mechanics"
      perl -0pi -e 's/let length_fourth = length_squared \* length_squared;/let length_fourth = length_squared * length_m;/' "$mechanics"
      grep -Fq 'let length_fourth = length_squared * length_m;' "$mechanics"
      ;;
    *)
      printf 'Unknown mutation: %s\n' "$mutant" >&2
      return 2
      ;;
  esac

  if cargo test --locked \
      --manifest-path "$mutant_dir/Cargo.toml" \
      --test metamorphic "$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: mutation survived: %s (expected %s to fail)\n' "$label" "$test_name" >&2
    cat "$log" >&2
    return 1
  fi

  if ! grep -Fq "test $test_name ... FAILED" "$log"; then
    printf 'ERROR: mutation run failed for a reason other than the expected test assertion: %s\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi

  printf 'KILLED mutation: %s; detected by %s\n' "$label" "$test_name"
}

run_mutation \
  "axial-extension-length-exponent" \
  axial_extension_length_squared \
  axial_bar_obeys_force_homogeneity_and_geometric_scaling

run_mutation \
  "cantilever-deflection-span-exponent" \
  cantilever_deflection_length_squared \
  cantilever_obeys_force_span_and_stiffness_scaling

run_mutation \
  "simply-supported-deflection-span-exponent" \
  supported_beam_deflection_length_cubed \
  simply_supported_beam_obeys_load_span_and_stiffness_scaling

printf 'Mutation sensitivity: 3/3 selected formula defects detected.\n'
