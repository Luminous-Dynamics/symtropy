#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Prove that selected analytical/metamorphic tests detect deliberate formula defects.
# All mutations run in disposable copies; production source is never edited.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/material-damage-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
fatigue_sha256="$(sha256sum "$crate_dir/src/fatigue.rs" | awk '{print $1}')"
paris_sha256="$(sha256sum "$crate_dir/src/paris.rs" | awk '{print $1}')"
oracle_sha256="$(sha256sum "$crate_dir/tests/oracle.rs" | awk '{print $1}')"
metamorphic_sha256="$(sha256sum "$crate_dir/tests/metamorphic.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" >"$control_log" 2>&1; then
  printf 'ERROR: unmutated material-damage test suite failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated material-damage suite\n'

run_mutation() {
  local label="$1"
  local mutant="$2"
  local test_target="$3"
  local test_name="$4"
  local mutant_dir="$tmp_root/$label"
  local fatigue="$mutant_dir/src/fatigue.rs"
  local paris="$mutant_dir/src/paris.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp -a "$crate_dir/." "$mutant_dir/"

  case "$mutant" in
    sn_interpolation_direction)
      grep -Fq 'let cycles = (ln_cycles_a + fraction * (ln_cycles_b - ln_cycles_a)).exp();' "$fatigue"
      perl -0pi -e 's/let cycles = \(ln_cycles_a \+ fraction \* \(ln_cycles_b - ln_cycles_a\)\)\.exp\(\);/let cycles = (ln_cycles_a - fraction * (ln_cycles_b - ln_cycles_a)).exp();/' "$fatigue"
      grep -Fq 'let cycles = (ln_cycles_a - fraction * (ln_cycles_b - ln_cycles_a)).exp();' "$fatigue"
      ;;
    miner_uses_product)
      grep -Fq 'let damage_increment = block.cycles as f64 / cycles_to_failure;' "$fatigue"
      perl -0pi -e 's/let damage_increment = block\.cycles as f64 \/ cycles_to_failure;/let damage_increment = block.cycles as f64 * cycles_to_failure;/' "$fatigue"
      grep -Fq 'let damage_increment = block.cycles as f64 * cycles_to_failure;' "$fatigue"
      ;;
    paris_omits_exponent)
      grep -Fq 'let delta_k_power = delta_k_mpa_sqrt_m.powf(self.exponent);' "$paris"
      perl -0pi -e 's/let delta_k_power = delta_k_mpa_sqrt_m\.powf\(self\.exponent\);/let delta_k_power = delta_k_mpa_sqrt_m;/' "$paris"
      grep -Fq 'let delta_k_power = delta_k_mpa_sqrt_m;' "$paris"
      ;;
    paris_cycle_division)
      grep -Fq 'let crack_extension_m = growth_rate_m_per_cycle * cycles as f64;' "$paris"
      perl -0pi -e 's/let crack_extension_m = growth_rate_m_per_cycle \* cycles as f64;/let crack_extension_m = growth_rate_m_per_cycle \/ cycles as f64;/' "$paris"
      grep -Fq 'let crack_extension_m = growth_rate_m_per_cycle / cycles as f64;' "$paris"
      ;;
    *)
      printf 'Unknown mutation: %s\n' "$mutant" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --test "$test_target" "$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: mutation survived: %s (expected %s to fail)\n' "$label" "$test_name" >&2
    cat "$log" >&2
    return 1
  fi

  if ! grep -Fq "test $test_name ... FAILED" "$log"; then
    printf 'ERROR: mutation did not reach the expected test assertion: %s\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi

  printf 'KILLED mutation: %s; detected by %s\n' "$label" "$test_name"
}

run_mutation \
  "sn-interpolation-direction" \
  sn_interpolation_direction \
  oracle \
  sn_curve_returns_frozen_endpoints_and_log_log_midpoint
run_mutation \
  "miner-product-instead-of-quotient" \
  miner_uses_product \
  oracle \
  miner_rule_matches_independent_block_damage_known_answer
run_mutation \
  "paris-omits-exponent" \
  paris_omits_exponent \
  oracle \
  paris_law_constant_delta_k_matches_closed_form_increment
run_mutation \
  "paris-divides-by-cycle-count" \
  paris_cycle_division \
  metamorphic \
  paris_growth_is_linear_in_cycles_and_coefficient_at_fixed_delta_k

test "$fatigue_sha256" = "$(sha256sum "$crate_dir/src/fatigue.rs" | awk '{print $1}')"
test "$paris_sha256" = "$(sha256sum "$crate_dir/src/paris.rs" | awk '{print $1}')"
test "$oracle_sha256" = "$(sha256sum "$crate_dir/tests/oracle.rs" | awk '{print $1}')"
test "$metamorphic_sha256" = "$(sha256sum "$crate_dir/tests/metamorphic.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg fatigue_sha256 "$fatigue_sha256" \
  --arg paris_sha256 "$paris_sha256" \
  --arg oracle_sha256 "$oracle_sha256" \
  --arg metamorphic_sha256 "$metamorphic_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "material-damage-mutation-receipt-v0.1",
    benchmark_id: "PHYS-DAMAGE-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {
      fatigue_rs: $fatigue_sha256,
      paris_rs: $paris_sha256,
      oracle_rs: $oracle_sha256,
      metamorphic_rs: $metamorphic_sha256
    },
    toolchain: { rustc: $rustc_version, cargo: $cargo_version },
    baseline_control: { suite: "all", disposition: "pass" },
    mutations: [
      { id: "PHYS-DAMAGE-MUT-SN-001", expected_test: "sn_curve_returns_frozen_endpoints_and_log_log_midpoint", disposition: "detected" },
      { id: "PHYS-DAMAGE-MUT-MINER-001", expected_test: "miner_rule_matches_independent_block_damage_known_answer", disposition: "detected" },
      { id: "PHYS-DAMAGE-MUT-PARIS-EXPONENT-001", expected_test: "paris_law_constant_delta_k_matches_closed_form_increment", disposition: "detected" },
      { id: "PHYS-DAMAGE-MUT-PARIS-CYCLES-001", expected_test: "paris_growth_is_linear_in_cycles_and_coefficient_at_fixed_delta_k", disposition: "detected" }
    ],
    summary: {
      detected: 4,
      undetected: 0,
      invalid_mutant: 0,
      equivalent_mutant: 0,
      control_passed: true
    }
  }' > "$receipt_path"

jq -e '
  .schema_version == "material-damage-mutation-receipt-v0.1"
  and (.source_commit | type == "string" and length == 40)
  and .baseline_control.disposition == "pass"
  and .summary.detected == 4
  and .summary.undetected == 0
  and ([.mutations[].disposition] | all(. == "detected"))
' "$receipt_path" >/dev/null

printf 'Mutation sensitivity: 4/4 selected formula defects detected. Receipt: %s\n' "$receipt_path"
