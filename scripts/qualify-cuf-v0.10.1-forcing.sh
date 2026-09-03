#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Local qualification gate for CUF deterministic forcing evidence v0.10.1.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

OBSERVATION_SRC="crates/core/symtropy-sim-contracts/src/observation.rs"

for marker in \
    'pub struct ObservationEvidence' \
    'pub struct ForcingModelId' \
    'pub struct DeterministicForcingEvidence' \
    'symtropy.observation-evidence.digest.v1' \
    'symtropy.deterministic-forcing-evidence.digest.v1'; do
    if ! grep -Fq "$marker" "$OBSERVATION_SRC"; then
        printf 'ERROR: required v0.10.1 marker missing: %s\n' "$marker" >&2
        exit 1
    fi
done

printf 'CUF v0.10.1 forcing qualification head: %s\n' "$(git rev-parse HEAD)"
printf 'Rust:  %s\n' "$(rustc --version)"
printf 'Cargo: %s\n' "$(cargo --version)"

printf '\n== Core forcing evidence ==\n'
cargo fmt --all -- --check
cargo test --locked -p symtropy-sim-contracts
cargo clippy --locked -p symtropy-sim-contracts --all-targets -- -D warnings

printf '\n== Explicit forcing evidence tests ==\n'
cargo test --locked -p symtropy-sim-contracts forcing_digest_is_stable_and_input_sensitive
cargo test --locked -p symtropy-sim-contracts forcing_digest_is_output_sensitive
cargo test --locked -p symtropy-sim-contracts forcing_round_trip_preserves_identity
cargo test --locked -p symtropy-sim-contracts invalid_forcing_model_identity_is_rejected

printf '\n== CUF v0.10 regression stack ==\n'
bash scripts/qualify-cuf-v0.10-stack.sh

printf '\n== Diff hygiene ==\n'
git diff --check
git diff --cached --check

printf '\nPASS: CUF deterministic forcing evidence v0.10.1 qualification gates\n'
printf 'Qualified head: %s\n' "$(git rev-parse HEAD)"
