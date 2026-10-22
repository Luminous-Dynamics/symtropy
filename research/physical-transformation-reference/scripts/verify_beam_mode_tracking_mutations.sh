#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation sensitivity for MAC-based cross-mesh mode identity.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/beam-mode-tracking-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
tracking_sha256="$(sha256sum "$crate_dir/src/beam_mode_tracking.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"
test_suite="beam_mode_tracking::tests"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib "$test_suite" >"$control_log" 2>&1; then
  printf 'ERROR: unmutated beam-mode-tracking tests failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated beam-mode-tracking tests\n'

run_mutation() {
  local label="$1"
  local mutation="$2"
  local test_name="$3"
  local mutant_dir="$tmp_root/$label"
  local source="$mutant_dir/src/beam_mode_tracking.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
  cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"

  case "$mutation" in
    wrong_station_fraction)
      grep -Fq 'StationKey::new(*station, elements_per_member)' "$source"
      perl -0pi -e 's/StationKey::new\(\*station, elements_per_member\)/StationKey::new(*station, 1)/g' "$source"
      grep -Fq 'StationKey::new(*station, 1)' "$source"
      ;;
    minimum_mac_instead_of_maximum)
      grep -Fq 'let cost = 1.0 - weights[row0 - 1][column - 1];' "$source"
      perl -0pi -e 's/let cost = 1\.0 - weights\[row0 - 1\]\[column - 1\];/let cost = weights[row0 - 1][column - 1];/' "$source"
      grep -Fq 'let cost = weights[row0 - 1][column - 1];' "$source"
      ;;
    rotation_unscaled)
      grep -Fq 'combined_left.push(*previous_value * rotation_scale);' "$source"
      perl -0pi -e 's/combined_left\.push\(\*previous_value \* rotation_scale\);/combined_left.push(*previous_value);/; s/combined_right\.push\(\*current_value \* rotation_scale\);/combined_right.push(*current_value);/' "$source"
      grep -Fq 'combined_left.push(*previous_value);' "$source"
      grep -Fq 'combined_right.push(*current_value);' "$source"
      ;;
    *)
      printf 'Unknown beam mode-tracking mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "$test_suite::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: beam mode-tracking mutation survived: %s\n' "$label" >&2
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
  "wrong-rational-station-key" \
  wrong_station_fraction \
  mode_tracking_reports_mac_and_bijective_assignments_across_nested_meshes
run_mutation \
  "minimum-instead-of-maximum-mac-assignment" \
  minimum_mac_instead_of_maximum \
  hungarian_assignment_finds_global_not_greedy_optimum
run_mutation \
  "rotation-component-left-unscaled" \
  rotation_unscaled \
  reference_length_changes_combined_mac_when_translation_and_rotation_mix

test "$tracking_sha256" = "$(sha256sum "$crate_dir/src/beam_mode_tracking.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg tracking_sha256 "$tracking_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "beam-mode-tracking-mutation-receipt-v0.1",
    benchmark_id: "PHYS-BEAM-MODE-TRACKING-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {beam_mode_tracking_rs: $tracking_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "beam_mode_tracking::tests", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-BEAM-TRACKING-STATION-KEY-001",
        expected_test: "mode_tracking_reports_mac_and_bijective_assignments_across_nested_meshes",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-TRACKING-ASSIGNMENT-001",
        expected_test: "hungarian_assignment_finds_global_not_greedy_optimum",
        disposition: "detected"
      },
      {
        id: "PHYS-BEAM-TRACKING-ROTATION-SCALE-001",
        expected_test: "tracking_request_records_nested_levels_mode_count_and_rotation_scale",
        disposition: "detected"
      }
    ],
    mutation_count: 3,
    detected_count: 3,
    production_source_unchanged: true,
    claim_ceiling: "cross-mesh MAC sensitivity only; not full NBC05 reproduction or physical validation"
  }' > "$receipt_path"

printf 'PASS beam-mode-tracking mutation receipt: %s\n' "$receipt_path"
