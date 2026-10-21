#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation checks for member-chain continuity and deterministic mesh identity.
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
crate_dir="$repo_root/research/physical-transformation-reference"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/beam-chain-mesh-mutation-receipt.json"
source_commit="$(git rev-parse HEAD)"
assembly_sha256="$(sha256sum "$crate_dir/src/beam_pin_assembly.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"
test_suite="beam_pin_assembly::tests"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib "$test_suite" >"$control_log" 2>&1; then
  printf 'ERROR: unmutated beam-chain assembly tests failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: beam member-chain assembly tests\n'

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
    disconnect_interior_rotation)
      grep -Fq '                rotations[element_index + 1],' "$source"
      perl -0pi -e 's/                rotations\[element_index \+ 1\],/                rotations[element_index],/' "$source"
      grep -Fq '                rotations[element_index],' "$source"
      ;;
    ignore_canonical_mesh_order)
      grep -Fq '    levels.sort_unstable();' "$source"
      perl -0pi -e 's/    levels\.sort_unstable\(\);\n//' "$source"
      if grep -Fq '    levels.sort_unstable();' "$source"; then
        printf 'ERROR: canonical mesh order mutation did not apply\n' >&2
        return 1
      fi
      ;;
    bypass_mesh_count_limit)
      grep -Fq 'if !(1..=MAX_CHAIN_ELEMENTS_PER_MEMBER).contains(&member.elements_per_member) {' "$source"
      perl -0pi -e 's/if !\(1\.\.=MAX_CHAIN_ELEMENTS_PER_MEMBER\)\.contains\(&member\.elements_per_member\) \{/if false {/' "$source"
      grep -Fq 'if false {' "$source"
      ;;
    *)
      printf 'Unknown beam-chain mesh mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "$test_suite::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: beam-chain mesh mutation survived: %s\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  if ! grep -Fq "test $test_suite::$test_name ... FAILED" "$log"; then
    printf 'ERROR: mutant %s failed outside its named assertion.\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  printf 'KILLED mutation: %s by %s\n' "$label" "$test_name"
}

run_mutation \
  "interior-rotation-disconnected" \
  disconnect_interior_rotation \
  mesh_refinement_shares_interior_rotations_without_creating_artificial_pins
run_mutation \
  "noncanonical-mesh-order" \
  ignore_canonical_mesh_order \
  canonical_mesh_sweep_request_covers_levels_and_is_order_invariant
run_mutation \
  "mesh-count-limit-bypassed" \
  bypass_mesh_count_limit \
  member_chain_invalid_mesh_envelope_fails_closed

test "$assembly_sha256" = "$(sha256sum "$crate_dir/src/beam_pin_assembly.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg assembly_sha256 "$assembly_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "beam-chain-mesh-mutation-receipt-v0.1",
    benchmark_id: "PHYS-BEAM-CHAIN-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {beam_pin_assembly_rs: $assembly_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "beam_pin_assembly::tests", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-BEAM-CHAIN-MUT-ROTATION-CONTINUITY-001",
        expected_test: "mesh_refinement_shares_interior_rotations_without_creating_artificial_pins",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-CHAIN-MUT-CANONICAL-ORDER-001",
        expected_test: "canonical_mesh_sweep_request_covers_levels_and_is_order_invariant",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-CHAIN-MUT-COUNT-LIMIT-001",
        expected_test: "member_chain_invalid_mesh_envelope_fails_closed",
        disposition: "detected"
      }
    ],
    mutation_count: 3,
    detected_count: 3,
    production_source_unchanged: true,
    claim_ceiling: "member-chain topology and mesh identity test sensitivity only; no 200-element NBC05 reproduction or physical validation"
  }' > "$receipt_path"

printf 'PASS beam-chain mesh mutation receipt: %s\n' "$receipt_path"
