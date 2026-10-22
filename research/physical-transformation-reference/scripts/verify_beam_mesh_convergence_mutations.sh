#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation sensitivity for frequency-ranked beam mesh convergence diagnostics.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/beam-mesh-convergence-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
convergence_sha256="$(sha256sum "$crate_dir/src/beam_mesh_convergence.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"
test_suite="beam_mesh_convergence::tests"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib "$test_suite" >"$control_log" 2>&1; then
  printf 'ERROR: unmutated beam mesh convergence tests failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated beam mesh convergence tests\n'

run_mutation() {
  local label="$1"
  local mutation="$2"
  local test_name="$3"
  local mutant_dir="$tmp_root/$label"
  local source="$mutant_dir/src/beam_mesh_convergence.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
  cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"

  case "$mutation" in
    wrong_relative_frequency_delta)
      grep -Fq 'let delta = (fine - coarse).abs() / coarse.abs().max(f64::MIN_POSITIVE);' "$source"
      perl -0pi -e 's/let delta = \(fine - coarse\)\.abs\(\) \/ coarse\.abs\(\)\.max\(f64::MIN_POSITIVE\);/let delta = (fine + coarse).abs() \/ coarse.abs().max(f64::MIN_POSITIVE);/' "$source"
      grep -Fq 'let delta = (fine + coarse).abs() / coarse.abs().max(f64::MIN_POSITIVE);' "$source"
      ;;
    inverted_monotonic_gate)
      grep -Fq 'if fine > coarse * (1.0 + monotonic_relative_tolerance) {' "$source"
      perl -0pi -e 's/if fine > coarse \* \(1\.0 \+ monotonic_relative_tolerance\) \{/if fine < coarse * (1.0 - monotonic_relative_tolerance) {/' "$source"
      grep -Fq 'if fine < coarse * (1.0 - monotonic_relative_tolerance) {' "$source"
      ;;
    omit_mode_count_from_request)
      grep -Fq '|requested_modes={requested_modes}' "$source"
      perl -0pi -e 's/\|requested_modes=\{requested_modes\}/|requested_modes=6/' "$source"
      # Keep the format expression compilable by removing the now-unused named argument.
      perl -0pi -e 's/\|requested_modes=6\|monotonic_relative_tolerance=\{monotonic_relative_tolerance\}/|requested_modes=6|monotonic_relative_tolerance={monotonic_relative_tolerance}/' "$source"
      grep -Fq '|requested_modes=6' "$source"
      ;;
    *)
      printf 'Unknown beam mesh-convergence mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "$test_suite::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: beam mesh-convergence mutation survived: %s\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  if ! grep -Fq "test $test_suite::$test_name ... FAILED" "$log"; then
    printf 'ERROR: mutant %s failed for a reason other than its named assertion.\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  printf 'KILLED mutation: %s by %s\n' "$label" "$test_name"
}

run_mutation \
  "incorrect-relative-delta" \
  wrong_relative_frequency_delta \
  four_level_mesh_ladder_reports_sorted_modes_residuals_and_refinement_deltas
run_mutation \
  "inverted-monotonic-gate" \
  inverted_monotonic_gate \
  four_level_mesh_ladder_reports_sorted_modes_residuals_and_refinement_deltas
run_mutation \
  "mode-count-omitted-from-request" \
  omit_mode_count_from_request \
  request_digest_changes_when_mesh_ladder_or_mode_count_changes

test "$convergence_sha256" = "$(sha256sum "$crate_dir/src/beam_mesh_convergence.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg convergence_sha256 "$convergence_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "beam-mesh-convergence-mutation-receipt-v0.1",
    benchmark_id: "PHYS-BEAM-MESH-CONVERGENCE-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {beam_mesh_convergence_rs: $convergence_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "beam_mesh_convergence::tests", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-BEAM-CONVERGENCE-DELTA-001",
        expected_test: "four_level_mesh_ladder_reports_sorted_modes_residuals_and_refinement_deltas",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-CONVERGENCE-MONOTONIC-001",
        expected_test: "four_level_mesh_ladder_reports_sorted_modes_residuals_and_refinement_deltas",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-CONVERGENCE-REQUEST-MODE-001",
        expected_test: "request_digest_changes_when_mesh_ladder_or_mode_count_changes",
        disposition: "detected"
      }
    ],
    mutation_count: 3,
    detected_count: 3,
    production_source_unchanged: true,
    claim_ceiling: "mesh-ladder diagnostic sensitivity only; frequency-ordinal pairing, no MAC or physical validation"
  }' > "$receipt_path"

printf 'PASS beam-mesh convergence mutation receipt: %s\n' "$receipt_path"
