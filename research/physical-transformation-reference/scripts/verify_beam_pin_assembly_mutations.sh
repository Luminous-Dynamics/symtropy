#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation sensitivity for deterministic released-joint beam assembly.
# Each source mutant lives only in a temporary copy; production files are never edited.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/beam-pin-assembly-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
assembly_sha256="$(sha256sum "$crate_dir/src/beam_pin_assembly.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"
test_suite="beam_pin_assembly::tests"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib "$test_suite" >"$control_log" 2>&1; then
  printf 'ERROR: unmutated released-joint assembly tests failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated released-joint assembly tests\n'

run_mutation() {
  local label="$1"
  local mutation="$2"
  local test_name="$3"
  local mutant_dir="$tmp_root/$label"
  local source="$mutant_dir/src/beam_pin_assembly.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
  cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"

  case "$mutation" in
    share_end_rotation_at_common_joint)
      grep -Fq '            private_rotation_base + 1,' "$source"
      perl -0pi -e 's/            private_rotation_base \+ 1,\n        \];/            translation_dof_count + 1,\n        ];/' "$source"
      grep -Fq '            translation_dof_count + 1,' "$source"
      ;;
    input_order_controls_global_numbering)
      grep -Fq '    ordered_nodes.sort_by(|left, right| left.id.cmp(&right.id));' "$source"
      perl -0pi -e 's/    ordered_nodes\.sort_by\(\|left, right\| left\.id\.cmp\(&right\.id\)\);\n//' "$source"
      if grep -Fq '    ordered_nodes.sort_by(|left, right| left.id.cmp(&right.id));' "$source"; then
        printf 'ERROR: could not apply input-order mutation\n' >&2
        return 1
      fi
      ;;
    *)
      printf 'Unknown beam-assembly mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "$test_suite::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: released-joint assembly mutation survived: %s\n' "$label" >&2
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
  "shared-rotation-at-joint" \
  share_end_rotation_at_common_joint \
  every_member_end_rotation_has_stiffness_and_mass_support
run_mutation \
  "noncanonical-input-order" \
  input_order_controls_global_numbering \
  assembly_is_invariant_to_input_storage_order

test "$assembly_sha256" = "$(sha256sum "$crate_dir/src/beam_pin_assembly.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg assembly_sha256 "$assembly_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "beam-pin-assembly-mutation-receipt-v0.1",
    benchmark_id: "PHYS-BEAM-PIN-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {beam_pin_assembly_rs: $assembly_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "beam_pin_assembly::tests", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-BEAM-PIN-MUT-SHARED-ROTATION-001",
        expected_test: "every_member_end_rotation_has_stiffness_and_mass_support",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-PIN-MUT-INPUT-ORDER-001",
        expected_test: "assembly_is_invariant_to_input_storage_order",
        disposition: "detected"
      }
    ],
    mutation_count: 2,
    detected_count: 2,
    production_source_unchanged: true,
    claim_ceiling: "pin-joint DOF-map and deterministic assembly sensitivity only; no global modal result or physical validation"
  }' > "$receipt_path"

printf 'PASS beam-pin-assembly mutation receipt: %s\n' "$receipt_path"
