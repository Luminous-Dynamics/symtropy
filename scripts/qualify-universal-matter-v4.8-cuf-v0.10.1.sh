#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Combined local qualification gate for an applied/staged Universal Matter
# v4.8 tree together with CUF v0.10.1 deterministic forcing evidence.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

PRIVATE_MULTIWORLD="../mycelix-multiworld-sim"
EXPECTED_STAGED_V48_PATHS=275

check_postimage_prefix() {
    local path="$1"
    local expected="$2"
    if [[ ! -f "$path" ]]; then
        printf 'ERROR: required v4.8 file missing: %s\n' "$path" >&2
        exit 1
    fi
    local actual
    actual="$(git hash-object "$path")"
    if [[ "${actual:0:${#expected}}" != "$expected" ]]; then
        printf 'ERROR: v4.8 postimage mismatch for %s\nexpected prefix: %s\nactual:          %s\n' \
            "$path" "$expected" "$actual" >&2
        exit 1
    fi
}

require_staged_shape() {
    local staged_count
    staged_count="$(git diff --cached --name-only | wc -l | tr -d ' ')"
    if [[ "$staged_count" -ne "$EXPECTED_STAGED_V48_PATHS" ]]; then
        printf 'ERROR: expected exactly %s staged v4.8 paths; found %s\n' \
            "$EXPECTED_STAGED_V48_PATHS" "$staged_count" >&2
        exit 1
    fi
}

require_forcing_contract() {
    local observation="crates/core/symtropy-sim-contracts/src/observation.rs"
    local lib="crates/core/symtropy-sim-contracts/src/lib.rs"
    for marker in \
        'pub struct ObservationEvidence' \
        'pub struct ForcingModelId' \
        'pub struct DeterministicForcingEvidence' \
        'symtropy.observation-evidence.digest.v1' \
        'symtropy.deterministic-forcing-evidence.digest.v1'; do
        if ! grep -Fq "$marker" "$observation"; then
            printf 'ERROR: combined CUF forcing marker missing: %s\n' "$marker" >&2
            exit 1
        fi
    done
    if ! grep -Fq 'DeterministicForcingEvidence, ForcingModelId, ObservationEvidence' "$lib"; then
        printf 'ERROR: forcing evidence is not root-exported from symtropy-sim-contracts.\n' >&2
        exit 1
    fi
}

printf 'Universal Matter v4.8 + CUF v0.10.1 qualification head: %s\n' "$(git rev-parse HEAD)"
printf 'Rust:  %s\n' "$(rustc --version)"
printf 'Cargo: %s\n' "$(cargo --version)"

check_postimage_prefix crates/domains/symtropy-terrain/Cargo.toml e93c0cc
check_postimage_prefix crates/domains/symtropy-terrain/src/lib.rs e138e1d
check_postimage_prefix docs/canon/CONSTRUCTION_REPAIR_AND_STRUCTURAL_TRANSFORMATION_CONTRACT_V0_1.md 8ee0b22
check_postimage_prefix docs/canon/FIRSTLIGHT_CORE_ACTION_DANGER_COMBAT_RESCUE_AND_DESTRUCTION_CONTRACT_V0_1.md 0b47b48
check_postimage_prefix docs/ops/FIRSTLIGHT_VERTICAL_SLICE_IMPLEMENTATION_TICKET_BACKLOG_V0_1.md 8b33e7d
check_postimage_prefix docs/tech/STRUCTURAL_INTEGRITY_CONSTRUCTION_AND_DESTRUCTION_RUNTIME_V0_1.md b3f3f40

for marker in \
    crates/domains/symtropy-terrain/src/authority.rs \
    crates/domains/symtropy-terrain/src/dynamic_hydrology.rs \
    crates/domains/symtropy-terrain/src/surface_water.rs \
    crates/domains/symtropy-terrain/src/watershed.rs \
    crates/domains/symtropy-terrain/src/ecosystem_authority.rs \
    crates/domains/symtropy-terrain/src/weather_field.rs \
    crates/domains/symtropy-terrain/src/planet_genesis_proof.rs \
    crates/domains/symtropy-terrain/src/solver_adapters.rs; do
    if [[ ! -f "$marker" ]]; then
        printf 'ERROR: Universal Matter v4.8 marker missing: %s\n' "$marker" >&2
        exit 1
    fi
done

require_forcing_contract

if [[ ! -d "$PRIVATE_MULTIWORLD" ]]; then
    cat >&2 <<'EOF'
ERROR: ../mycelix-multiworld-sim is absent.
The combined v4.8 + CUF v0.10.1 qualification requires the full/private
Luminous Dynamics monorepo layout. No partial "green" result will be reported.
EOF
    exit 1
fi

require_staged_shape
cargo_lock_before="$(git hash-object Cargo.lock)"
staged_tree_before="$(git write-tree)"

printf '\n== Universal Matter v4.8 Terrain authority ==\n'
cargo fmt --all -- --check
cargo test -p symtropy-terrain
cargo clippy -p symtropy-terrain --all-targets -- -D warnings

if [[ -x scripts/verify-terrain-handoff.sh ]]; then
    printf '\n== Universal Matter handoff verification ==\n'
    bash scripts/verify-terrain-handoff.sh
fi

printf '\n== CUF v0.10.1 forcing + v0.10 regression ==\n'
bash scripts/qualify-cuf-v0.10.1-forcing.sh

printf '\n== Combined qualification side-effect guards ==\n'
require_staged_shape
require_forcing_contract

cargo_lock_after="$(git hash-object Cargo.lock)"
if [[ "$cargo_lock_after" != "$cargo_lock_before" ]]; then
    cat >&2 <<EOF
ERROR: Cargo.lock changed during combined qualification.
Before: $cargo_lock_before
After:  $cargo_lock_after
Treat any required lockfile update as an explicit qualification-repair commit.
EOF
    exit 1
fi

staged_tree_after="$(git write-tree)"
if [[ "$staged_tree_after" != "$staged_tree_before" ]]; then
    printf 'ERROR: staged v4.8 tree changed during qualification.\nBefore: %s\nAfter:  %s\n' \
        "$staged_tree_before" "$staged_tree_after" >&2
    exit 1
fi

if ! git diff --quiet; then
    printf 'ERROR: qualification produced unstaged tracked changes.\n' >&2
    git diff --stat >&2
    exit 1
fi
untracked="$(git ls-files --others --exclude-standard)"
if [[ -n "$untracked" ]]; then
    printf 'ERROR: qualification produced untracked files:\n%s\n' "$untracked" >&2
    exit 1
fi

printf '\n== Repository diff hygiene ==\n'
git diff --check
git diff --cached --check

printf '\nPASS: Universal Matter v4.8 + CUF v0.10.1 qualification gates\n'
printf 'Qualification parent: %s\n' "$(git rev-parse HEAD)"
printf 'Qualified staged tree: %s\n' "$staged_tree_after"
printf 'Staged v4.8 paths: %s\n' "$EXPECTED_STAGED_V48_PATHS"
printf 'Cargo.lock unchanged: %s\n' "$cargo_lock_after"
printf 'Do not promote unless the evidence capsule proves this exact tree.\n'
