#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Proves the mass-sensitivity test detects a no-op rotary-inertia counterfactual.
# All mutation edits occur in a temporary crate copy; production files stay fixed.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/beam-mass-sensitivity-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
module_sha256="$(sha256sum "$crate_dir/src/beam_mass_sensitivity.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"
test_name="rotary_inertia_sensitivity_is_explicit_and_rank_frequency_is_monotone"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib beam_mass_sensitivity::tests >"$control_log" 2>&1; then
  printf 'ERROR: unmutated beam mass-sensitivity tests failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: beam mass-sensitivity test suite\n'

mutant_dir="$tmp_root/no-op-rotary-counterfactual"
mkdir -p "$mutant_dir"
cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"
source="$mutant_dir/src/beam_mass_sensitivity.rs"
log="$tmp_root/no-op-rotary-counterfactual.log"

grep -Fq 'without_rotary.mass_global = without_rotary.translational_mass_global.clone();' "$source"
perl -0pi -e 's/without_rotary\.mass_global = without_rotary\.translational_mass_global\.clone\(\);/without_rotary.mass_global = without_rotary.mass_global.clone();/' "$source"
grep -Fq 'without_rotary.mass_global = without_rotary.mass_global.clone();' "$source"

if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "beam_mass_sensitivity::tests::$test_name" -- --exact >"$log" 2>&1; then
  printf 'ERROR: no-op rotary-inertia sensitivity mutant survived.\n' >&2
  cat "$log" >&2
  exit 1
fi
if ! grep -Fq "test beam_mass_sensitivity::tests::$test_name ... FAILED" "$log"; then
  printf 'ERROR: mutant failed for a reason other than the expected sensitivity assertion.\n' >&2
  cat "$log" >&2
  exit 1
fi
printf 'KILLED mutation: no-op-rotary-counterfactual by %s\n' "$test_name"

test "$module_sha256" = "$(sha256sum "$crate_dir/src/beam_mass_sensitivity.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg module_sha256 "$module_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "beam-mass-sensitivity-mutation-receipt-v0.1",
    benchmark_id: "PHYS-BEAM-MASS-SENS-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {beam_mass_sensitivity_rs: $module_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "beam_mass_sensitivity::tests", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-BEAM-MASS-SENS-MUT-NO-OP-001",
        defect: "sensitivity counterfactual reuses full mass instead of removing rotary inertia",
        expected_test: "rotary_inertia_sensitivity_is_explicit_and_rank_frequency_is_monotone",
        disposition: "detected"
      }
    ],
    mutation_count: 1,
    detected_count: 1,
    production_source_unchanged: true,
    claim_ceiling: "tests distinguish the rotary-inertia sensitivity counterfactual from the full consistent-mass model; no physical validation"
  }' > "$receipt_path"

printf 'PASS beam mass-sensitivity mutation receipt: %s\n' "$receipt_path"
