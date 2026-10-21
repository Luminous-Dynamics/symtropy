#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation sensitivity for frequency/MAC-gated isolated-mode assignment.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/modal-assignment-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
source_sha256="$(sha256sum "$crate_dir/src/modal_assignment.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"

control_log="$tmp_root/control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib modal_assignment::tests >"$control_log" 2>&1; then
  printf 'ERROR: unmutated modal-assignment controls failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated modal-assignment tests\n'

run_mutation() {
  local label="$1"
  local mutation="$2"
  local test_name="$3"
  local mutant_dir="$tmp_root/$label"
  local source="$mutant_dir/src/modal_assignment.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
  cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"

  case "$mutation" in
    minimize_mac_instead_of_maximize)
      grep -Fq 'costs[reference_index][candidate_index] = 1.0 - mac;' "$source"
      perl -0pi -e 's/costs\[reference_index\]\[candidate_index\] = 1\.0 - mac;/costs[reference_index][candidate_index] = mac;/' "$source"
      grep -Fq 'costs[reference_index][candidate_index] = mac;' "$source"
      ;;
    ignore_frequency_gate)
      grep -Fq 'if relative_gap <= maximum_relative_frequency_gap && mac >= minimum_mac {' "$source"
      perl -0pi -e 's/if relative_gap <= maximum_relative_frequency_gap && mac >= minimum_mac \{/if mac >= minimum_mac {/' "$source"
      grep -Fq 'if mac >= minimum_mac {' "$source"
      ;;
    *)
      printf 'Unknown modal-assignment mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "modal_assignment::tests::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: modal-assignment mutation survived: %s\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  if ! grep -Fq "test modal_assignment::tests::$test_name ... FAILED" "$log"; then
    printf 'ERROR: mutant %s failed for a reason other than the expected assertion.\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  printf 'KILLED mutation: %s by %s\n' "$label" "$test_name"
}

run_mutation \
  "assignment-minimizes-mac" \
  minimize_mac_instead_of_maximize \
  assignment_maximizes_total_mac_globally_not_greedily
run_mutation \
  "frequency-gate-ignored" \
  ignore_frequency_gate \
  assignment_abstains_when_no_complete_matching_meets_both_gates

test "$source_sha256" = "$(sha256sum "$crate_dir/src/modal_assignment.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg source_sha256 "$source_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "modal-assignment-mutation-receipt-v0.1",
    benchmark_id: "PHYS-MODAL-ASSIGNMENT-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {modal_assignment_rs: $source_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "modal_assignment::tests", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-MODAL-ASSIGNMENT-MUT-MAC-OBJECTIVE-001",
        expected_test: "assignment_maximizes_total_mac_globally_not_greedily",
        disposition: "detected"
      },
      {
        id: "PHYS-MODAL-ASSIGNMENT-MUT-FREQUENCY-GATE-001",
        expected_test: "assignment_abstains_when_no_complete_matching_meets_both_gates",
        disposition: "detected"
      }
    ],
    mutation_count: 2,
    detected_count: 2,
    production_source_unchanged: true,
    claim_ceiling: "software mode identity matching for pre-classified isolated modes only; no clustered-mode assignment, complete validation, physical validation or safety claim"
  }' > "$receipt_path"

printf 'PASS modal-assignment mutation receipt: %s\n' "$receipt_path"
