#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation sensitivity for release-aware mesh assembly.
# Mutants run in isolated copies; production source is never modified.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/beam-mesh-assembly-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
source_sha256="$(sha256sum "$crate_dir/src/beam_mesh_assembly.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"
test_name="refinement_keeps_interior_rotation_continuous_and_joint_releases_private"

control_log="$tmp_root/control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib beam_mesh_assembly::tests >"$control_log" 2>&1; then
  printf 'ERROR: unmutated beam-mesh tests failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated beam-mesh assembly tests\n'

mutant_dir="$tmp_root/shared-release-rotations"
mkdir -p "$mutant_dir"
cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"
mutant_source="$mutant_dir/src/beam_mesh_assembly.rs"
mutation_log="$tmp_root/shared-release-rotations.log"

grep -Fq 'if !element.release_start {' "$mutant_source"
grep -Fq 'if !element.release_end {' "$mutant_source"
grep -Fq 'let start_rotation = if element.release_start {' "$mutant_source"
grep -Fq 'let end_rotation = if element.release_end {' "$mutant_source"

perl -0pi -e '
  s/if !element\.release_start \{/if true {/g;
  s/if !element\.release_end \{/if true {/g;
  s/let start_rotation = if element\.release_start \{/let start_rotation = if false {/;
  s/let end_rotation = if element\.release_end \{/let end_rotation = if false {/;
' "$mutant_source"

grep -Fq 'if true {' "$mutant_source"
grep -Fq 'let start_rotation = if false {' "$mutant_source"
grep -Fq 'let end_rotation = if false {' "$mutant_source"

if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "beam_mesh_assembly::tests::$test_name" -- --exact >"$mutation_log" 2>&1; then
  printf 'ERROR: tying released member-end rotations survived the topology test.\n' >&2
  cat "$mutation_log" >&2
  exit 1
fi
if ! grep -Fq "test beam_mesh_assembly::tests::$test_name ... FAILED" "$mutation_log"; then
  printf 'ERROR: mutant failed outside the expected rotation-topology test.\n' >&2
  cat "$mutation_log" >&2
  exit 1
fi
printf 'KILLED mutation: pin-release-to-shared-rotation by %s\n' "$test_name"

test "$source_sha256" = "$(sha256sum "$crate_dir/src/beam_mesh_assembly.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg source_sha256 "$source_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "beam-mesh-assembly-mutation-receipt-v0.1",
    benchmark_id: "PHYS-BEAM-MESH-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {beam_mesh_assembly_rs: $source_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "beam_mesh_assembly::tests", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-BEAM-MESH-MUT-PIN-ROTATION-001",
        expected_test: "refinement_keeps_interior_rotation_continuous_and_joint_releases_private",
        disposition: "detected"
      }
    ],
    mutation_count: 1,
    detected_count: 1,
    production_source_unchanged: true,
    claim_ceiling: "released-joint and mesh-continuity assembly test sensitivity only; no modal solve or physical validation"
  }' > "$receipt_path"

printf 'PASS beam-mesh mutation receipt: %s\n' "$receipt_path"
