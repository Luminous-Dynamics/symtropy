#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation-sensitivity gate for pin-jointed beam DOF ownership.
# Mutants are isolated in temporary crate copies; production files stay fixed.
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

test_name="pin_joints_share_translations_but_not_member_rotations"
control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib "beam_pin_assembly::tests::$test_name" -- --exact >"$control_log" 2>&1; then
  printf 'ERROR: unmutated released-joint assembly control failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: pin-joint rotation-ownership test\n'

mutant_dir="$tmp_root/shared-member-rotations"
mkdir -p "$mutant_dir"
cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"
source="$mutant_dir/src/beam_pin_assembly.rs"
log="$tmp_root/shared-member-rotations.log"

grep -Fq 'let private_rotation_base = translation_dof_count + 2 * member_index;' "$source"
perl -0pi -e 's/let private_rotation_base = translation_dof_count \+ 2 \* member_index;/let private_rotation_base = translation_dof_count + end_node_index;/' "$source"
grep -Fq 'let private_rotation_base = translation_dof_count + end_node_index;' "$source"

if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "beam_pin_assembly::tests::$test_name" -- --exact >"$log" 2>&1; then
  printf 'ERROR: shared-rotation mutant survived.\n' >&2
  cat "$log" >&2
  exit 1
fi
if ! grep -Fq "test beam_pin_assembly::tests::$test_name ... FAILED" "$log"; then
  printf 'ERROR: released-joint mutant failed outside the expected assertion.\n' >&2
  cat "$log" >&2
  exit 1
fi
printf 'KILLED mutation: shared end rotations at the common apex\n'

test "$assembly_sha256" = "$(sha256sum "$crate_dir/src/beam_pin_assembly.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg assembly_sha256 "$assembly_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "beam-pin-assembly-mutation-receipt-v0.1",
    benchmark_id: "PHYS-BEAM-PIN-DOF-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {beam_pin_assembly_rs: $assembly_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {test: "pin_joints_share_translations_but_not_member_rotations", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-BEAM-PIN-MUT-SHARED-ROTATION-001",
        expected_test: "pin_joints_share_translations_but_not_member_rotations",
        disposition: "detected"
      }
    ],
    mutation_count: 1,
    detected_count: 1,
    production_source_unchanged: true,
    claim_ceiling: "released-joint DOF ownership test sensitivity only; no full beam modal or physical validation"
  }' > "$receipt_path"

printf 'PASS beam pin-assembly mutation receipt: %s\n' "$receipt_path"
