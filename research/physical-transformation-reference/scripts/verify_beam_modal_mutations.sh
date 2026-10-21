#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Mutation-sensitivity gate for explicit rigid/mechanism-mode classification.
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
crate_dir="$repo_root/research/physical-transformation-reference"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
receipt_path="$repo_root/beam-modal-mutation-receipt.json"
source_commit="$(git rev-parse HEAD)"
modal_sha256="$(sha256sum "$crate_dir/src/beam_modal.rs" | awk '{print $1}')"
rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"
test_name="zero_eigenvalues_are_reported_not_silently_dropped"

control_log="$tmp_root/unmutated-control.log"
if ! cargo test --locked --manifest-path "$crate_dir/Cargo.toml" --lib "beam_modal::tests::$test_name" -- --exact >"$control_log" 2>&1; then
  printf 'ERROR: unmutated beam-modal mechanism control failed.\n' >&2
  cat "$control_log" >&2
  exit 1
fi
printf 'PASS control: near-zero eigenvalues are explicitly classified\n'

mutant_dir="$tmp_root/dropped-zero-modes"
mkdir -p "$mutant_dir"
cp "$crate_dir/Cargo.toml" "$crate_dir/Cargo.lock" "$mutant_dir/"
cp -a "$crate_dir/src" "$crate_dir/fixtures" "$mutant_dir/"
source="$mutant_dir/src/beam_modal.rs"
log="$tmp_root/dropped-zero-modes.log"

grep -Fq '        } else if eigenvalue <= zero_tolerance {' "$source"
perl -0pi -e 's/\} else if eigenvalue <= zero_tolerance \{/\} else if eigenvalue < -zero_tolerance \{/' "$source"
grep -Fq '        } else if eigenvalue < -zero_tolerance {' "$source"

if cargo test --locked --manifest-path "$mutant_dir/Cargo.toml" --lib "beam_modal::tests::$test_name" -- --exact >"$log" 2>&1; then
  printf 'ERROR: zero-mode misclassification mutant survived.\n' >&2
  cat "$log" >&2
  exit 1
fi
if ! grep -Fq "test beam_modal::tests::$test_name ... FAILED" "$log"; then
  printf 'ERROR: zero-mode mutant failed outside the expected assertion.\n' >&2
  cat "$log" >&2
  exit 1
fi
printf 'KILLED mutation: zero eigenvalue falsely classified as elastic\n'

test "$modal_sha256" = "$(sha256sum "$crate_dir/src/beam_modal.rs" | awk '{print $1}')"

jq -n \
  --arg source_commit "$source_commit" \
  --arg modal_sha256 "$modal_sha256" \
  --arg rustc_version "$rustc_version" \
  --arg cargo_version "$cargo_version" \
  '{
    schema_version: "beam-modal-mutation-receipt-v0.1",
    benchmark_id: "PHYS-BEAM-MODAL-MUTATION-001",
    evidence_class: "software-test-sensitivity",
    source_commit: $source_commit,
    source_sha256: {beam_modal_rs: $modal_sha256},
    toolchain: {rustc: $rustc_version, cargo: $cargo_version},
    baseline_control: {test: "zero_eigenvalues_are_reported_not_silently_dropped", disposition: "pass"},
    mutations: [
      {
        id: "PHYS-BEAM-MOD-NEARZERO-DROP-001",
        expected_test: "zero_eigenvalues_are_reported_not_silently_dropped",
        disposition: "detected"
      }
    ],
    mutation_count: 1,
    detected_count: 1,
    production_source_unchanged: true,
    claim_ceiling: "modal eigenvalue classification test sensitivity for synthetic released-joint beam matrices only; no benchmark or physical-validation claim"
  }' > "$receipt_path"

printf 'PASS beam-modal mutation receipt: %s\n' "$receipt_path"
