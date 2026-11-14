#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation sensitivity for mesh-refined beam topology and request identity.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/beam-mesh-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
mesh_sha256="$(sha256sum "$crate_dir/src/beam_mesh.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib beam_mesh::tests >"$control_log" 2>&1; then
  printf 'ERROR: unmutated beam-mesh test control failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated beam-mesh tests\n'

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
    collapse_member_rotation_ranges)
      grep -Fq 'let rotation_base = translation_dofs + member_index * rotations_per_member;' "$source"
      perl -0pi -e 's/let rotation_base = translation_dofs \+ member_index \* rotations_per_member;/let rotation_base = translation_dofs;/' "$source"
      grep -Fq 'let rotation_base = translation_dofs;' "$source"
      ;;
    break_internal_rotation_continuity)
      grep -Fq '            rotation_base + segment_index + 1,' "$source"
      perl -0pi -e 's/            rotation_base \+ segment_index \+ 1,\n        \];/            rotation_base,\n        ];/' "$source"
      grep -Fq '            rotation_base,' "$source"
      ;;
    collapse_interior_coordinates)
      grep -Fq 'let fraction = point_index as f64 / elements_per_member as f64;' "$source"
      perl -0pi -e 's/let fraction = point_index as f64 \/ elements_per_member as f64;/let fraction = 0.0;/' "$source"
      grep -Fq 'let fraction = 0.0;' "$source"
      ;;
    hardcode_element_count_in_request)
      grep -Fq 'elements_per_member={elements_per_member}' "$source"
      perl -0pi -e 's/elements_per_member=\\{elements_per_member\\}/elements_per_member=1/' "$source"
      grep -Fq 'elements_per_member=1' "$source"
      ;;
    reverse_element_translation_map)
      grep -Fq '            2 * start_index,' "$source"
      perl -0pi -e 's/            2 \\* start_index,\\n            2 \\* start_index \\+ 1,\\n            rotation_base \\+ segment_index,/            2 * end_index,\\n            2 * end_index + 1,\\n            rotation_base + segment_index,/' "$source"
      grep -Fq '            2 * end_index,' "$source"
      ;;
    *)
      printf 'Unknown beam-mesh mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "beam_mesh::tests::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: beam-mesh mutation survived: %s\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  if ! grep -Fq "test beam_mesh::tests::$test_name ... FAILED" "$log"; then
    printf 'ERROR: mutant %s failed for a reason other than the named assertion.\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  printf 'KILLED mutation: %s by %s\n' "$label" "$test_name"
}

run_mutation \
  "member-rotation-ranges-collapsed" \
  collapse_member_rotation_ranges \
  internal_rotations_are_shared_per_member_but_private_at_joints
run_mutation \
  "internal-rotation-continuity-broken" \
  break_internal_rotation_continuity \
  internal_rotations_are_shared_per_member_but_private_at_joints
run_mutation \
  "interior-coordinates-collapsed" \
  collapse_interior_coordinates \
  refined_assembly_is_invariant_to_input_storage_order
run_mutation \
  "refinement-count-omitted-from-request" \
  hardcode_element_count_in_request \
  mesh_request_is_frozen_and_includes_element_count

run_mutation \
  "element-translation-map-reversed" \
  reverse_element_translation_map \
  condensing_refined_beam_rotations_recovers_independent_axial_mesh_stiffness

test "$mesh_sha256" = "$(sha256sum "$crate_dir/src/beam_mesh.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg mesh_sha256 "$mesh_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "beam-mesh-mutation-receipt-v0.1",
    benchmark_id: "PHYS-BEAM-MESH-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {beam_mesh_rs: $mesh_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "beam_mesh::tests", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-BEAM-MESH-MUT-ROTATION-RANGE-001",
        expected_test: "internal_rotations_are_shared_per_member_but_private_at_joints",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-MESH-MUT-INTERIOR-CONTINUITY-001",
        expected_test: "internal_rotations_are_shared_per_member_but_private_at_joints",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-MESH-MUT-COORDINATES-001",
        expected_test: "refined_assembly_is_invariant_to_input_storage_order",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-MESH-MUT-REQUEST-COUNT-001",
        expected_test: "mesh_request_is_frozen_and_includes_element_count",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-MESH-MUT-TRANSLATION-MAP-001",
        expected_test: "condensing_refined_beam_rotations_recovers_independent_axial_mesh_stiffness",
        disposition: "detected"
      }
    ],
    mutation_count: 5,
    detected_count: 5,
    production_source_unchanged: true,
    claim_ceiling: "small dense mesh-topology sensitivity only; no full NAFEMS beam reproduction or physical validation"
  }' > "$receipt_path"

printf 'PASS beam-mesh mutation receipt: %s\n' "$receipt_path"
