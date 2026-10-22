#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation tests for SOFiSTiK BE25 source-input unit conversions.
# Every mutant is confined to a temporary crate copy.
set -euo pipefail

crate_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(git -C "$crate_dir" rev-parse --show-toplevel)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="${GITHUB_WORKSPACE:-$repo_root}/sofistik-be25-audit-mutation-receipt.json"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
audit_sha256="$(sha256sum "$crate_dir/src/sofistik_be25_audit.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib sofistik_be25_audit::tests >"$control_log" 2>&1; then
  printf 'ERROR: unmutated BE25 audit tests failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: unmutated SOFiSTiK BE25 unit audit\n'

run_mutation() {
  local label="$1"
  local mutation="$2"
  local test_name="$3"
  local mutant_dir="$tmp_root/$label"
  local source="$mutant_dir/src/sofistik_be25_audit.rs"
  local log="$tmp_root/$label.log"

  mkdir -p "$mutant_dir"
  cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
  cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"

  case "$mutation" in
    square_centimetre_area_conversion)
      grep -Fq 'let section_area_m2 = PUBLISHED_AREA_CM2 * 1.0e-4;' "$source"
      perl -0pi -e 's/let section_area_m2 = PUBLISHED_AREA_CM2 \* 1\.0e-4;/let section_area_m2 = PUBLISHED_AREA_CM2 * 1.0e-2;/' "$source"
      grep -Fq 'let section_area_m2 = PUBLISHED_AREA_CM2 * 1.0e-2;' "$source"
      ;;
    tonne_per_metre_conversion)
      grep -Fq 'let mass_per_length_printed_kg_per_m = PUBLISHED_MASS_TONNE_PER_M * 1.0e3;' "$source"
      perl -0pi -e 's/let mass_per_length_printed_kg_per_m = PUBLISHED_MASS_TONNE_PER_M \* 1\.0e3;/let mass_per_length_printed_kg_per_m = PUBLISHED_MASS_TONNE_PER_M * 1.0e6;/' "$source"
      grep -Fq 'let mass_per_length_printed_kg_per_m = PUBLISHED_MASS_TONNE_PER_M * 1.0e6;' "$source"
      ;;
    megapascal_conversion)
      grep -Fq 'let youngs_modulus_pa = PUBLISHED_E_MPA * 1.0e6;' "$source"
      perl -0pi -e 's/let youngs_modulus_pa = PUBLISHED_E_MPA \* 1\.0e6;/let youngs_modulus_pa = PUBLISHED_E_MPA * 1.0e9;/' "$source"
      grep -Fq 'let youngs_modulus_pa = PUBLISHED_E_MPA * 1.0e9;' "$source"
      ;;
    *)
      printf 'Unknown BE25 audit mutation: %s\n' "$mutation" >&2
      return 2
      ;;
  esac

  if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "sofistik_be25_audit::tests::$test_name" -- --exact >"$log" 2>&1; then
    printf 'ERROR: BE25 audit mutation survived: %s\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  if ! grep -Fq "test sofistik_be25_audit::tests::$test_name ... FAILED" "$log"; then
    printf 'ERROR: mutant %s failed for a reason other than the intended assertion.\n' "$label" >&2
    cat "$log" >&2
    return 1
  fi
  printf 'KILLED mutation: %s by %s\n' "$label" "$test_name"
}

run_mutation \
  "area-cm2-conversion" \
  square_centimetre_area_conversion \
  source_mass_per_length_interpretations_remain_explicitly_distinct
run_mutation \
  "tonne-per-m-conversion" \
  tonne_per_metre_conversion \
  source_mass_per_length_interpretations_remain_explicitly_distinct
run_mutation \
  "MPa-conversion" \
  megapascal_conversion \
  published_reference_table_does_not_match_either_literal_mass_candidate

test "$audit_sha256" = "$(sha256sum "$crate_dir/src/sofistik_be25_audit.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg audit_sha256 "$audit_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "sofistik-be25-audit-mutation-receipt-v0.1",
    benchmark_id: "SOFiSTiK-2026-BE25-INPUT-AUDIT",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {sofistik_be25_audit_rs: $audit_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {suite: "sofistik_be25_audit::tests", disposition: "pass"},
    mutations: [
      {
        id: "BE25-MUT-AREA-CM2-001",
        expected_test: "source_mass_per_length_interpretations_remain_explicitly_distinct",
        disposition: "detected"
      },
      {
        id: "BE25-MUT-TONNE-MASS-001",
        expected_test: "source_mass_per_length_interpretations_remain_explicitly_distinct",
        disposition: "detected"
      },
      {
        id: "BE25-MUT-MPA-001",
        expected_test: "published_reference_table_does_not_match_either_literal_mass_candidate",
        disposition: "detected"
      }
    ],
    mutation_count: 3,
    detected_count: 3,
    production_source_unchanged: true,
    claim_ceiling: "conversion-audit mutation sensitivity only; the BE25 benchmark remains blocked and not reproduced"
  }' > "$receipt_path"

printf 'PASS BE25 audit mutation receipt: %s\n' "$receipt_path"
