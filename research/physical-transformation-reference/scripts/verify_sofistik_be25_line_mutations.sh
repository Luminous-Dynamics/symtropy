#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation sensitivity for the 1D BE25 candidate beam-line kernel.
# Mutants are isolated in temporary crate copies; production files are unchanged.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/sofistik-be25-line-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
beam_line_sha256="$(sha256sum "$crate_dir/src/beam_line.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib beam_line::tests >"$control_log" 2>&1; then
  printf 'ERROR: unmutated BE25 line candidate tests failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated BE25 continuous line tests\n'

run_mutation() {
  local label="$1"
  local mutation="$2"
  local test_name="$3"
  local mutant_dir="$tmp_root/$label"
  local source="$mutant_dir/src/beam_line.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
  cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"

  case "$mutation" in
    consistent_mass_factor)
      grep -Fq 'let mass_factor = mass_per_length_kg_per_m * h / 420.0;' "$source"
      perl -0pi -e 's/let mass_factor = mass_per_length_kg_per_m \* h \/ 420\.0;/let mass_factor = mass_per_length_kg_per_m * h \/ 400.0;/' "$source"
      grep -Fq 'let mass_factor = mass_per_length_kg_per_m * h / 400.0;' "$source"
      ;;
    stiffness_length_exponent)
      grep -Fq 'let stiffness_factor = youngs_modulus_pa * second_moment_m4 / l3;' "$source"
      perl -0pi -e 's/let stiffness_factor = youngs_modulus_pa \* second_moment_m4 \/ l3;/let stiffness_factor = youngs_modulus_pa * second_moment_m4 \/ l2;/' "$source"
      grep -Fq 'let stiffness_factor = youngs_modulus_pa * second_moment_m4 / l2;' "$source"
      ;;
    clamped_pinned_boundary_dof)
      grep -Fq 'BeamLineBoundaryCase::CantileverWithSimplySupportedEnd => vec![0, 1, 2 * last_node],' "$source"
      perl -0pi -e 's/BeamLineBoundaryCase::CantileverWithSimplySupportedEnd => vec!\[0, 1, 2 \* last_node\],/BeamLineBoundaryCase::CantileverWithSimplySupportedEnd => vec![0, 1, 2 * last_node + 1],/' "$source"
      grep -Fq 'BeamLineBoundaryCase::CantileverWithSimplySupportedEnd => vec![0, 1, 2 * last_node + 1],' "$source"
      ;;
    *)
      printf 'Unknown BE25 line mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "beam_line::tests::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: BE25 line mutation survived: %s\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  if ! grep -Fq "test beam_line::tests::$test_name ... FAILED" "$log"; then
    printf 'ERROR: mutant %s failed for a reason other than the expected assertion.\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  printf 'KILLED mutation: %s by %s\n' "$label" "$test_name"
}

run_mutation \
  "consistent-mass-factor" \
  consistent_mass_factor \
  ten_element_candidate_runs_cover_all_four_published_boundary_cases
run_mutation \
  "stiffness-length-exponent" \
  stiffness_length_exponent \
  ten_element_candidate_runs_cover_all_four_published_boundary_cases
run_mutation \
  "clamped-pinned-boundary-dof" \
  clamped_pinned_boundary_dof \
  ten_element_candidate_runs_cover_all_four_published_boundary_cases

test "$beam_line_sha256" = "$(sha256sum "$crate_dir/src/beam_line.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg beam_line_sha256 "$beam_line_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "sofistik-be25-line-mutation-receipt-v0.1",
    benchmark_id: "SOFiSTiK-2026-BE25-LINE-CANDIDATE",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {beam_line_rs: $beam_line_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "beam_line::tests", disposition: "pass"},
    mutations: [
      {
        id: "BE25-LINE-MUT-MASS-FACTOR-001",
        expected_test: "ten_element_candidate_runs_cover_all_four_published_boundary_cases",
        disposition: "detected"
      },
      {
        id: "BE25-LINE-MUT-STIFFNESS-EXPONENT-001",
        expected_test: "ten_element_candidate_runs_cover_all_four_published_boundary_cases",
        disposition: "detected"
      },
      {
        id: "BE25-LINE-MUT-CLAMPED-PINNED-DOF-001",
        expected_test: "ten_element_candidate_runs_cover_all_four_published_boundary_cases",
        disposition: "detected"
      }
    ],
    mutation_count: 3,
    detected_count: 3,
    production_source_unchanged: true,
    claim_ceiling: "candidate BE25 line model mutation sensitivity; benchmark remains blocked"
  }' > "$receipt_path"

printf 'PASS BE25 line candidate mutation receipt: %s\n' "$receipt_path"
