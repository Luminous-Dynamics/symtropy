#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation sensitivity for the independent NBC05 two-DOF axial-truss oracle.
# Mutants live only in temporary crate copies; production source is never edited.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/nafems-nbc05-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
oracle_sha256="$(sha256sum "$crate_dir/src/nafems_nbc05.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"
test_name="reported_nbc05_cs1_cs2_frequency_table_matches_analytic_two_dof_oracle"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib nafems_nbc05::tests >"$control_log" 2>&1; then
  printf 'ERROR: unmutated NBC05 oracle unit-test control failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated NAFEMS NBC05 oracle tests\n'

run_mutation() {
  local label="$1"
  local mutation="$2"
  local mutant_dir="$tmp_root/$label"
  local source="$mutant_dir/src/nafems_nbc05.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
  cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"

  case "$mutation" in
    drop_off_diagonal_coupling)
      grep -Fq 'let kxy = diagonal_factor * cosine * sine;' "$source"
      perl -0pi -e 's/let kxy = diagonal_factor \* cosine \* sine;/let kxy = 0.0;/' "$source"
      grep -Fq 'let kxy = 0.0;' "$source"
      ;;
    lumped_mass_uses_consistent_factor)
      grep -Fq 'MassMatrixFormulation::Lumped => 1.0 / 2.0,' "$source"
      perl -0pi -e 's/MassMatrixFormulation::Lumped => 1\.0 \/ 2\.0,/MassMatrixFormulation::Lumped => 1.0 \/ 3.0,/' "$source"
      grep -Fq 'MassMatrixFormulation::Lumped => 1.0 / 3.0,' "$source"
      ;;
    swap_mass_formulation_labels)
      grep -Fq 'MassMatrixFormulation::Consistent => 1.0 / 3.0,' "$source"
      grep -Fq 'MassMatrixFormulation::Lumped => 1.0 / 2.0,' "$source"
      perl -0pi -e 's/MassMatrixFormulation::Consistent => 1\.0 \/ 3\.0,/MassMatrixFormulation::Consistent => 1.0 \/ 2.0,/; s/MassMatrixFormulation::Lumped => 1\.0 \/ 2\.0,/MassMatrixFormulation::Lumped => 1.0 \/ 3.0,/' "$source"
      grep -Fq 'MassMatrixFormulation::Consistent => 1.0 / 2.0,' "$source"
      grep -Fq 'MassMatrixFormulation::Lumped => 1.0 / 3.0,' "$source"
      ;;
    *)
      printf 'Unknown NBC05 mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "nafems_nbc05::tests::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: NBC05 oracle mutation survived: %s\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  if ! grep -Fq "test nafems_nbc05::tests::$test_name ... FAILED" "$log"; then
    printf 'ERROR: mutant %s failed for a reason other than the expected assertion.\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  printf 'KILLED mutation: %s by %s\n' "$label" "$test_name"
}

run_mutation \
  "missing-off-diagonal-coupling" \
  drop_off_diagonal_coupling
run_mutation \
  "lumped-mass-factor" \
  lumped_mass_uses_consistent_factor
run_mutation \
  "consistent-lumped-label-swap" \
  swap_mass_formulation_labels

test "$oracle_sha256" = "$(sha256sum "$crate_dir/src/nafems_nbc05.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg oracle_sha256 "$oracle_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "nafems-nbc05-mutation-receipt-v0.1",
    benchmark_id: "PHYS-NAFEMS-NBC05-AXIAL-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {nafems_nbc05_rs: $oracle_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "nafems_nbc05::tests", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-NBC05-MUT-OFFDIAGONAL-001",
        expected_test: "reported_nbc05_cs1_cs2_frequency_table_matches_analytic_two_dof_oracle",
        disposition: "detected"
      },
      {
        id: "PHYS-NBC05-MUT-LUMPED-MASS-001",
        expected_test: "reported_nbc05_cs1_cs2_frequency_table_matches_analytic_two_dof_oracle",
        disposition: "detected"
      },
      {
        id: "PHYS-NBC05-MUT-MASS-LABEL-001",
        expected_test: "reported_nbc05_cs1_cs2_frequency_table_matches_analytic_two_dof_oracle",
        disposition: "detected"
      }
    ],
    mutation_count: 3,
    detected_count: 3,
    production_source_unchanged: true,
    claim_ceiling: "test-sensitivity evidence for the reported synthetic axial-bar truss model; no physical validation or bending-mode qualification"
  }' > "$receipt_path"

printf 'PASS NAFEMS NBC05 mutation receipt: %s\n' "$receipt_path"
