#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Prove that selected planar-truss tests kill representative formula/numerical defects.
# Every mutant lives in a fresh temporary copy; the production checkout stays authoritative.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/planar-truss-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
truss_sha256="$(sha256sum "$crate_dir/src/planar_truss.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib planar_truss::tests >"$control_log" 2>&1; then
  printf 'ERROR: unmutated planar-truss unit-test control failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated planar-truss unit tests\n'

run_mutation() {
  local label="$1"
  local mutation="$2"
  local test_name="$3"
  local mutant_dir="$tmp_root/$label"
  local source="$mutant_dir/src/planar_truss.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
  cp -a "$crate_dir/src" "$mutant_dir/"

  case "$mutation" in
    truss_stiffness_omits_area)
      grep -Fq 'let axial_stiffness = bar.youngs_modulus_pa * bar.area_m2 / length;' "$source"
      perl -0pi -e 's/let axial_stiffness = bar\.youngs_modulus_pa \* bar\.area_m2 \/ length;/let axial_stiffness = bar.youngs_modulus_pa \/ length;/' "$source"
      grep -Fq 'let axial_stiffness = bar.youngs_modulus_pa / length;' "$source"
      ;;
    member_force_omits_area)
      grep -Fq 'let axial_force = bar.youngs_modulus_pa * bar.area_m2 * strain;' "$source"
      perl -0pi -e 's/let axial_force = bar\.youngs_modulus_pa \* bar\.area_m2 \* strain;/let axial_force = bar.youngs_modulus_pa * strain;/' "$source"
      grep -Fq 'let axial_force = bar.youngs_modulus_pa * strain;' "$source"
      ;;
    support_reaction_sign)
      grep -Fq 'let value = internal[2 * index + 1] - loads[2 * index + 1];' "$source"
      perl -0pi -e 's/let value = internal\[2 \* index \+ 1\] - loads\[2 \* index \+ 1\];/let value = loads[2 * index + 1] - internal[2 * index + 1];/' "$source"
      grep -Fq 'let value = loads[2 * index + 1] - internal[2 * index + 1];' "$source"
      ;;
    absolute_pivot_floor)
      grep -Fq 'f64::EPSILON * PIVOT_EPSILON_FACTOR * matrix_scale * n as f64' "$source"
      perl -0pi -e 's/f64::EPSILON \* PIVOT_EPSILON_FACTOR \* matrix_scale \* n as f64/f64::EPSILON * PIVOT_EPSILON_FACTOR * matrix_scale.max(1.0) * n as f64/' "$source"
      grep -Fq 'matrix_scale.max(1.0) * n as f64' "$source"
      ;;
    *)
      printf 'Unknown truss mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "planar_truss::tests::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: planar-truss mutation survived: %s\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  if ! grep -Fq "test planar_truss::tests::$test_name ... FAILED" "$log"; then
    printf 'ERROR: mutant %s failed for a reason other than the expected assertion.\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  printf 'KILLED mutation: %s by %s\n' "$label" "$test_name"
}

run_mutation \
  "truss-stiffness-area-factor" \
  truss_stiffness_omits_area \
  load_and_stiffness_scaling_obey_metamorphic_relations
run_mutation \
  "member-axial-force-area-factor" \
  member_force_omits_area \
  symmetric_two_bar_truss_matches_independent_closed_form_oracle
run_mutation \
  "vertical-support-reaction-sign" \
  support_reaction_sign \
  symmetric_two_bar_truss_matches_independent_closed_form_oracle
run_mutation \
  "unit-scale-dependent-singularity-floor" \
  absolute_pivot_floor \
  uniform_stiffness_and_load_rescaling_does_not_change_displacement

test "$truss_sha256" = "$(sha256sum "$crate_dir/src/planar_truss.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg truss_sha256 "$truss_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "planar-truss-mutation-receipt-v0.1",
    benchmark_id: "PHYS-TRUSS-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {planar_truss_rs: $truss_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "planar_truss::tests", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-TRUSS-MUT-STIFFNESS-AREA-001",
        expected_test: "load_and_stiffness_scaling_obey_metamorphic_relations",
        disposition: "detected"
      },
      {
        id: "PHYS-TRUSS-MUT-MEMBER-FORCE-AREA-001",
        expected_test: "symmetric_two_bar_truss_matches_independent_closed_form_oracle",
        disposition: "detected"
      },
      {
        id: "PHYS-TRUSS-MUT-REACTION-SIGN-001",
        expected_test: "symmetric_two_bar_truss_matches_independent_closed_form_oracle",
        disposition: "detected"
      },
      {
        id: "PHYS-TRUSS-MUT-PIVOT-SCALE-001",
        expected_test: "uniform_stiffness_and_load_rescaling_does_not_change_displacement",
        disposition: "detected"
      }
    ],
    summary: {
      detected: 4,
      undetected: 0,
      invalid_mutant: 0,
      equivalent_mutant: 0,
      control_passed: true
    }
  }' > "$receipt_path"

jq -e '
  .schema_version == "planar-truss-mutation-receipt-v0.1"
  and (.source_commit | type == "string" and length == 40)
  and .baseline_control.disposition == "pass"
  and .summary.detected == 4
  and .summary.undetected == 0
  and ([.mutations[].disposition] | all(. == "detected"))
' "$receipt_path" >/dev/null

printf 'Planar-truss mutation sensitivity: 4/4 named defects detected. Receipt: %s\n' "$receipt_path"
