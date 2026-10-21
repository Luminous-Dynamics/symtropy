#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Verify that selected metamorphic tests reject deliberately broken mechanics
# formulas. All mutations occur in disposable copies; production source is never edited.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/mutation-sensitivity-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
mechanics_sha256="$(sha256sum "$crate_dir/src/mechanics.rs" | awk '{print $1}')"
metamorphic_sha256="$(sha256sum "$crate_dir/tests/metamorphic.rs" | awk '{print $1}')"
units_sha256="$(sha256sum "$crate_dir/src/units.rs" | awk '{print $1}')"
unit_tests_sha256="$(sha256sum "$crate_dir/tests/units.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"

# Control: the unmutated metamorphic suite must pass before interpreting mutants.
control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --test metamorphic >"$control_log" 2>&1; then
  printf 'ERROR: unmutated metamorphic control failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated metamorphic suite\n'

units_control_log="$tmp_root/unmutated-units-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --test units >"$units_control_log" 2>&1; then
  printf 'ERROR: unmutated typed unit boundary control failed.\n' >&2
  cat "$units_control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated typed unit boundary suite\n'

run_mutation() {
  local label="$1"
  local mutant="$2"
  local test_name="$3"
  local test_target="${4:-metamorphic}"
  local mutant_dir="$tmp_root/$label"
  local mechanics="$mutant_dir/src/mechanics.rs"
  local units="$mutant_dir/src/units.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp -a "$crate_dir/." "$mutant_dir/"

  case "$mutant" in
    axial_extension_length_squared)
      grep -Fq 'let numerator = force_n * length_m;' "$mechanics"
      perl -0pi -e 's/let numerator = force_n \* length_m;/let numerator = force_n * length_m * length_m;/' "$mechanics"
      grep -Fq 'let numerator = force_n * length_m * length_m;' "$mechanics"
      ;;
    axial_stress_area_squared)
      grep -Fq 'let axial_stress_pa = force_n / area_m2;' "$mechanics"
      perl -0pi -e 's/let axial_stress_pa = force_n \/ area_m2;/let axial_stress_pa = force_n \/ (area_m2 * area_m2);/' "$mechanics"
      grep -Fq 'let axial_stress_pa = force_n / (area_m2 * area_m2);' "$mechanics"
      ;;
    cantilever_deflection_length_squared)
      grep -Fq 'let deflection_numerator = force_n * length_cubed;' "$mechanics"
      perl -0pi -e 's/let deflection_numerator = force_n \* length_cubed;/let deflection_numerator = force_n * length_squared;/' "$mechanics"
      grep -Fq 'let deflection_numerator = force_n * length_squared;' "$mechanics"
      ;;
    cantilever_omit_youngs_modulus)
      grep -Fq 'let flexural_denominator = 3.0 * youngs_modulus_pa * second_moment_area_m4;' "$mechanics"
      perl -0pi -e 's/let flexural_denominator = 3.0 \* youngs_modulus_pa \* second_moment_area_m4;/let flexural_denominator = 3.0 * second_moment_area_m4;/' "$mechanics"
      grep -Fq 'let flexural_denominator = 3.0 * second_moment_area_m4;' "$mechanics"
      ;;
    supported_beam_deflection_length_cubed)
      grep -Fq 'let length_fourth = length_squared * length_squared;' "$mechanics"
      perl -0pi -e 's/let length_fourth = length_squared \* length_squared;/let length_fourth = length_squared * length_m;/' "$mechanics"
      grep -Fq 'let length_fourth = length_squared * length_m;' "$mechanics"
      ;;
    supported_beam_stress_depends_on_modulus)
      grep -Fq 'maximum_bending_moment * outer_fiber_distance_m / second_moment_area_m4;' "$mechanics"
      perl -0pi -e 's/maximum_bending_moment \* outer_fiber_distance_m \/ second_moment_area_m4;/maximum_bending_moment * outer_fiber_distance_m \/ (second_moment_area_m4 * youngs_modulus_pa);/' "$mechanics"
      grep -Fq 'maximum_bending_moment * outer_fiber_distance_m / (second_moment_area_m4 * youngs_modulus_pa);' "$mechanics"
      ;;
    unit_length_mm_factor_1000)
      grep -Fq 'const MILLIMETERS_TO_METERS: f64 = 1.0e-3;' "$units"
      perl -0pi -e 's/const MILLIMETERS_TO_METERS: f64 = 1\.0e-3;/const MILLIMETERS_TO_METERS: f64 = 1.0;/' "$units"
      grep -Fq 'const MILLIMETERS_TO_METERS: f64 = 1.0;' "$units"
      ;;
    unit_force_kn_factor_1000)
      grep -Fq 'const KILONEWTONS_TO_NEWTONS: f64 = 1.0e3;' "$units"
      perl -0pi -e 's/const KILONEWTONS_TO_NEWTONS: f64 = 1\.0e3;/const KILONEWTONS_TO_NEWTONS: f64 = 1.0;/' "$units"
      grep -Fq 'const KILONEWTONS_TO_NEWTONS: f64 = 1.0;' "$units"
      ;;
    unit_area_mm2_exponent)
      grep -Fq 'const SQUARE_MILLIMETERS_TO_SQUARE_METERS: f64 = 1.0e-6;' "$units"
      perl -0pi -e 's/const SQUARE_MILLIMETERS_TO_SQUARE_METERS: f64 = 1\.0e-6;/const SQUARE_MILLIMETERS_TO_SQUARE_METERS: f64 = 1.0e-3;/' "$units"
      grep -Fq 'const SQUARE_MILLIMETERS_TO_SQUARE_METERS: f64 = 1.0e-3;' "$units"
      ;;
    unit_modulus_gpa_factor)
      grep -Fq 'const GIGAPASCALS_TO_PASCALS: f64 = 1.0e9;' "$units"
      perl -0pi -e 's/const GIGAPASCALS_TO_PASCALS: f64 = 1\.0e9;/const GIGAPASCALS_TO_PASCALS: f64 = 1.0;/' "$units"
      grep -Fq 'const GIGAPASCALS_TO_PASCALS: f64 = 1.0;' "$units"
      ;;
    unit_second_moment_mm4_exponent)
      grep -Fq 'const MILLIMETERS4_TO_METERS4: f64 = 1.0e-12;' "$units"
      perl -0pi -e 's/const MILLIMETERS4_TO_METERS4: f64 = 1\.0e-12;/const MILLIMETERS4_TO_METERS4: f64 = 1.0e-6;/' "$units"
      grep -Fq 'const MILLIMETERS4_TO_METERS4: f64 = 1.0e-6;' "$units"
      ;;
    unit_distributed_load_kn_per_m_factor)
      grep -Fq 'const KILONEWTONS_PER_METER_TO_NEWTONS_PER_METER: f64 = 1.0e3;' "$units"
      perl -0pi -e 's/const KILONEWTONS_PER_METER_TO_NEWTONS_PER_METER: f64 = 1\.0e3;/const KILONEWTONS_PER_METER_TO_NEWTONS_PER_METER: f64 = 1.0;/' "$units"
      grep -Fq 'const KILONEWTONS_PER_METER_TO_NEWTONS_PER_METER: f64 = 1.0;' "$units"
      ;;
    *)
      printf 'Unknown mutation: %s\n' "$mutant" >&2
      return 2
      ;;
  esac

  if cargo test --locked \
      --manifest-path "$mutant_dir/Cargo.toml" \
      --test "$test_target" "$test_name" -- --exact >"$log" 2>&1; then
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
  "axial-stress-area-exponent" \
  axial_stress_area_squared \
  axial_bar_obeys_force_homogeneity_and_geometric_scaling
run_mutation \
  "cantilever-deflection-span-exponent" \
  cantilever_deflection_length_squared \
  cantilever_obeys_force_span_and_stiffness_scaling
run_mutation \
  "cantilever-omit-elastic-modulus" \
  cantilever_omit_youngs_modulus \
  cantilever_obeys_force_span_and_stiffness_scaling
run_mutation \
  "simply-supported-deflection-span-exponent" \
  supported_beam_deflection_length_cubed \
  simply_supported_beam_obeys_load_span_and_stiffness_scaling
run_mutation \
  "simply-supported-stress-modulus-coupling" \
  supported_beam_stress_depends_on_modulus \
  simply_supported_beam_obeys_load_span_and_stiffness_scaling
run_mutation \
  "millimeters-conversion-factor" \
  unit_length_mm_factor_1000 \
  named_source_unit_conversions_match_canonical_si_values \
  units
run_mutation \
  "kilonewtons-conversion-factor" \
  unit_force_kn_factor_1000 \
  named_source_unit_conversions_match_canonical_si_values \
  units
run_mutation \
  "square-millimeters-conversion-exponent" \
  unit_area_mm2_exponent \
  named_source_unit_conversions_match_canonical_si_values \
  units
run_mutation \
  "gigapascals-conversion-factor" \
  unit_modulus_gpa_factor \
  named_source_unit_conversions_match_canonical_si_values \
  units
run_mutation \
  "millimeters-fourth-conversion-exponent" \
  unit_second_moment_mm4_exponent \
  named_source_unit_conversions_match_canonical_si_values \
  units
run_mutation \
  "kilonewtons-per-meter-conversion-factor" \
  unit_distributed_load_kn_per_m_factor \
  named_source_unit_conversions_match_canonical_si_values \
  units

# Prove the actual checkout's source and test files were not changed by any mutant run.
test "$mechanics_sha256" = "$(sha256sum "$crate_dir/src/mechanics.rs" | awk '{print $1}')"
test "$metamorphic_sha256" = "$(sha256sum "$crate_dir/tests/metamorphic.rs" | awk '{print $1}')"
test "$units_sha256" = "$(sha256sum "$crate_dir/src/units.rs" | awk '{print $1}')"
test "$unit_tests_sha256" = "$(sha256sum "$crate_dir/tests/units.rs" | awk '{print $1}')"

# Stable field order and no timestamps make the receipt reproducible at a fixed commit/toolchain.
jq -n \
  --arg source_commit "$source_commit" \
  --arg mechanics_sha256 "$mechanics_sha256" \
  --arg metamorphic_sha256 "$metamorphic_sha256" \
  --arg units_sha256 "$units_sha256" \
  --arg unit_tests_sha256 "$unit_tests_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "physical-transform-mutation-receipt-v0.1",
    benchmark_id: "PHYS-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {
      mechanics_rs: $mechanics_sha256,
      metamorphic_rs: $metamorphic_sha256,
      units_rs: $units_sha256,
      unit_tests_rs: $unit_tests_sha256
    },
    toolchain: {
      rustc: $rustc_version,
      cargo: $cargo_version
    },
    baseline_control: {
      suites: ["tests/metamorphic.rs", "tests/units.rs"],
      disposition: "pass"
    },
    mutations: [
      {
        id: "PHYS-MUT-AXIAL-LENGTH-001",
        expected_test: "axial_bar_obeys_force_homogeneity_and_geometric_scaling",
        disposition: "detected"
      },
      {
        id: "PHYS-MUT-AXIAL-AREA-001",
        expected_test: "axial_bar_obeys_force_homogeneity_and_geometric_scaling",
        disposition: "detected"
      },
      {
        id: "PHYS-MUT-CANTILEVER-SPAN-001",
        expected_test: "cantilever_obeys_force_span_and_stiffness_scaling",
        disposition: "detected"
      },
      {
        id: "PHYS-MUT-CANTILEVER-MODULUS-001",
        expected_test: "cantilever_obeys_force_span_and_stiffness_scaling",
        disposition: "detected"
      },
      {
        id: "PHYS-MUT-SUPPORTED-BEAM-SPAN-001",
        expected_test: "simply_supported_beam_obeys_load_span_and_stiffness_scaling",
        disposition: "detected"
      },
      {
        id: "PHYS-MUT-SUPPORTED-BEAM-STRESS-MODULUS-001",
        expected_test: "simply_supported_beam_obeys_load_span_and_stiffness_scaling",
        disposition: "detected"
      },
      {
        id: "PHYS-MUT-UNITS-LENGTH-MM-001",
        expected_test: "named_source_unit_conversions_match_canonical_si_values",
        disposition: "detected"
      },
      {
        id: "PHYS-MUT-UNITS-FORCE-KN-001",
        expected_test: "named_source_unit_conversions_match_canonical_si_values",
        disposition: "detected"
      },
      {
        id: "PHYS-MUT-UNITS-AREA-MM2-001",
        expected_test: "named_source_unit_conversions_match_canonical_si_values",
        disposition: "detected"
      },
      {
        id: "PHYS-MUT-UNITS-MODULUS-GPA-001",
        expected_test: "named_source_unit_conversions_match_canonical_si_values",
        disposition: "detected"
      },
      {
        id: "PHYS-MUT-UNITS-SECOND-MOMENT-MM4-001",
        expected_test: "named_source_unit_conversions_match_canonical_si_values",
        disposition: "detected"
      },
      {
        id: "PHYS-MUT-UNITS-DISTRIBUTED-LOAD-001",
        expected_test: "named_source_unit_conversions_match_canonical_si_values",
        disposition: "detected"
      }
    ],
    summary:    ],
    summary: {
      detected: 12,
      undetected: 0,
      invalid_mutant: 0,
      equivalent_mutant: 0,
      control_passed: true
    }
  }' > "$receipt_path"

jq -e '
  .schema_version == "physical-transform-mutation-receipt-v0.1"
  and (.source_commit | type == "string" and length == 40)
  and .baseline_control.disposition == "pass"
  and .summary.detected == 12
  and .summary.undetected == 0
  and ([.mutations[].disposition] | all(. == "detected"))
' "$receipt_path" >/dev/null

printf 'Mutation sensitivity: 12/12 selected formula and unit-conversion defects detected. Receipt: %s\n' "$receipt_path"
