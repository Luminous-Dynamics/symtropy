#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation sensitivity for principal-angle subspace MAC.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/beam-subspace-mac-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
subspace_sha256="$(sha256sum "$crate_dir/src/beam_subspace_mac.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"
test_suite="beam_subspace_mac::tests"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib "$test_suite" >"$control_log" 2>&1; then
  printf 'ERROR: unmutated subspace-MAC tests failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated subspace-MAC tests\n'

run_mutation() {
  local label="$1"
  local mutation="$2"
  local test_name="$3"
  local mutant_dir="$tmp_root/$label"
  local source="$mutant_dir/src/beam_subspace_mac.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
  cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"

  case "$mutation" in
    omit_basis_normalization)
      grep -Fq '*value /= residual_norm;' "$source"
      perl -0pi -e 's/\*value \/= residual_norm;/\*value \/= 1.0;/' "$source"
      grep -Fq '*value /= 1.0;' "$source"
      ;;
    reverse_principal_cosine_order)
      grep -Fq 'principal_cosines.reverse();' "$source"
      perl -0pi -e 's/\s*principal_cosines\.reverse\(\);\n/\n/' "$source"
      if grep -Fq 'principal_cosines.reverse();' "$source"; then
        printf 'ERROR: principal-cosine ordering mutation not applied\n' >&2
        return 1
      fi
      ;;
    *)
      printf 'Unknown subspace-MAC mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "$test_suite::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: subspace-MAC mutation survived: %s\n' "$label" >&2
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
  "basis-normalization-removed" \
  omit_basis_normalization \
  subspace_score_is_invariant_to_basis_rotation_and_individual_mode_scaling
run_mutation \
  "principal-cosines-returned-ascending" \
  reverse_principal_cosine_order \
  partial_overlap_matches_closed_form_principal_angles

test "$subspace_sha256" = "$(sha256sum "$crate_dir/src/beam_subspace_mac.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg subspace_sha256 "$subspace_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "beam-subspace-mac-mutation-receipt-v0.1",
    benchmark_id: "PHYS-BEAM-SUBSPACE-MAC-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {beam_subspace_mac_rs: $subspace_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "beam_subspace_mac::tests", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-SUBSPACE-MAC-NORMALIZATION-001",
        expected_test: "subspace_score_is_invariant_to_orthogonal_basis_rotation",
        disposition: "detected"
      },
      {
        id: "PHYS-SUBSPACE-MAC-ORDERING-001",
        expected_test: "partial_overlap_matches_closed_form_principal_angles",
        disposition: "detected"
      }
    ],
    mutation_count: 2,
    detected_count: 2,
    production_source_unchanged: true,
    claim_ceiling: "standalone principal-angle numerical utility sensitivity only; no physical validation"
  }' > "$receipt_path"

printf 'PASS beam-subspace-MAC mutation receipt: %s\n' "$receipt_path"
