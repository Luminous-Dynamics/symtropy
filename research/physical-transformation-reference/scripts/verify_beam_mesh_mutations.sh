#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation sensitivity for continuous beam-member mesh assembly.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/beam-mesh-assembly-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
mesh_sha256="$(sha256sum "$crate_dir/src/beam_mesh.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"
test_suite="beam_mesh::tests"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib "$test_suite" >"$control_log" 2>&1; then
  printf 'ERROR: unmutated beam-mesh assembly tests failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated beam-mesh assembly tests\n'

run_mutation() {
  local label="$1"
  local mutation="$2"
  local test_name="$3"
  local mutant_dir="$tmp_root/$label"
  local source="$mutant_dir/src/beam_mesh.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
  cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"

  case "$mutation" in
    internal_rotations_not_shared)
      grep -Fq 'let rotation_start = rotation_base + segment_index;' "$source"
      perl -0pi -e 's/let rotation_start = rotation_base \+ segment_index;/let rotation_start = rotation_base;/' "$source"
      grep -Fq 'let rotation_start = rotation_base;' "$source"
      ;;
    segment_geometry_fraction)
      grep -Fq 'let fraction = station as f64 / elements_per_member as f64;' "$source"
      perl -0pi -e 's/let fraction = station as f64 \/ elements_per_member as f64;/let fraction = station as f64 \/ (elements_per_member as f64 + 1.0);/' "$source"
      grep -Fq 'let fraction = station as f64 / (elements_per_member as f64 + 1.0);' "$source"
      ;;
    element_count_receipt)
      grep -Fq 'total_element_count: members.len() * elements_per_member,' "$source"
      perl -0pi -e 's/total_element_count: members\.len\(\) \* elements_per_member,/total_element_count: members.len(),/' "$source"
      grep -Fq 'total_element_count: members.len(),' "$source"
      ;;
    *)
      printf 'Unknown beam mesh mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "$test_suite::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: beam-mesh mutation survived: %s\n' "$label" >&2
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
  "internal-rotation-station-collapse" \
  internal_rotations_not_shared \
  internal_rotations_are_shared_within_members_but_not_at_truss_joints
run_mutation \
  "wrong-segment-coordinate-fraction" \
  segment_geometry_fraction \
  assembly_is_deterministic_under_input_permutation_and_request_tracks_mesh_density
run_mutation \
  "incorrect-total-element-count" \
  element_count_receipt \
  internal_rotations_are_shared_within_members_but_not_at_truss_joints

test "$mesh_sha256" = "$(sha256sum "$crate_dir/src/beam_mesh.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg mesh_sha256 "$mesh_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "beam-mesh-assembly-mutation-receipt-v0.1",
    benchmark_id: "PHYS-BEAM-MESH-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {beam_mesh_rs: $mesh_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "beam_mesh::tests", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-BEAM-MESH-ROTATION-SHARE-001",
        expected_test: "internal_rotations_are_shared_within_members_but_not_at_truss_joints",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-MESH-COORDINATE-001",
        expected_test: "assembly_is_deterministic_under_input_permutation_and_request_tracks_mesh_density",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-MESH-ELEMENT-COUNT-001",
        expected_test: "internal_rotations_are_shared_within_members_but_not_at_truss_joints",
        disposition: "detected"
      }
    ],
    mutation_count: 3,
    detected_count: 3,
    production_source_unchanged: true,
    claim_ceiling: "small-mesh DOF assembly sensitivity only; no mesh-converged NBC05 spectrum or physical validation"
  }' > "$receipt_path"

printf 'PASS beam-mesh assembly mutation receipt: %s\n' "$receipt_path"
