#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Qualification gate for an applied/staged Universal Matter v4.8 tree together
# with the Causal Universe Fabric v0.1-v0.10 stack.

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

printf 'Universal Matter v4.8 + CUF qualification head: %s\n' "$(git rev-parse HEAD)"
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
    crates/domains/symtropy-terrain/src/planet_genesis_proof.rs \
    crates/domains/symtropy-terrain/src/solver_adapters.rs; do
    if [[ ! -f "$marker" ]]; then
        printf 'ERROR: Universal Matter v4.8 marker missing: %s\n' "$marker" >&2
        exit 1
    fi
done

if [[ ! -d "$PRIVATE_MULTIWORLD" ]]; then
    cat >&2 <<'EOF'
ERROR: ../mycelix-multiworld-sim is absent.
The combined CUF qualification requires the full/private Luminous Dynamics
monorepo layout. No partial "green" result will be reported.
EOF
    exit 1
fi

# The apply helper deliberately stages the retained patch as one reviewable
# change-set. Qualification must not silently widen that staged lineage.
require_staged_shape

cargo_lock_before="$(git hash-object Cargo.lock)"

printf '\n== Formatting ==\n'
cargo fmt --all -- --check

printf '\n== Universal Matter v4.8 Terrain authority ==\n'
cargo test -p symtropy-terrain
cargo clippy -p symtropy-terrain --all-targets -- -D warnings

if [[ -x scripts/verify-terrain-handoff.sh ]]; then
    printf '\n== Universal Matter handoff verification ==\n'
    bash scripts/verify-terrain-handoff.sh
fi

printf '\n== CUF v0.1-v0.10 regression ==\n'
bash scripts/qualify-cuf-v0.10-stack.sh

printf '\n== Qualification side-effect guards ==\n'
require_staged_shape
cargo_lock_after="$(git hash-object Cargo.lock)"
if [[ "$cargo_lock_after" != "$cargo_lock_before" ]]; then
    cat >&2 <<EOF
ERROR: Cargo.lock changed during v4.8 qualification.
Before: $cargo_lock_before
After:  $cargo_lock_after
Resolve and review the lockfile change as an explicit separate qualification
commit, then rerun this gate from a cleanly understood state.
EOF
    exit 1
fi

# No untracked/unstaged file may be silently created by qualification. The only
# expected source changes are the exactly 275 paths already staged by the replay.
if [[ -n "$(git status --porcelain --untracked-files=all | grep -v '^M  ' || true)" ]]; then
    printf 'ERROR: qualification produced unexpected unstaged/untracked changes.\n' >&2
    git status --short >&2
    exit 1
fi

printf '\n== Repository diff hygiene ==\n'
git diff --check
git diff --cached --check

printf '\nPASS: Universal Matter v4.8 + CUF v0.10 qualification gates\n'
printf 'Qualified working-tree head: %s\n' "$(git rev-parse HEAD)"
printf 'Staged v4.8 paths: %s\n' "$EXPECTED_STAGED_V48_PATHS"
printf 'Cargo.lock unchanged: %s\n' "$cargo_lock_after"
printf 'Record the staged tree / resulting commit SHA before treating this lineage as qualified.\n'