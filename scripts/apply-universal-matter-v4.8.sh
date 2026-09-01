#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Explicit, guarded replay of the authored Universal Matter v4.8 cumulative patch.
# The result is staged but intentionally NOT committed.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

PATCH="${1:-}"
if [[ -z "$PATCH" ]]; then
    printf 'usage: %s /path/to/SYMTROPY_UNIVERSAL_MATTER_V4.8_CUMULATIVE.patch\n' "$0" >&2
    exit 2
fi

bash scripts/preflight-universal-matter-v4.8.sh "$PATCH"

printf '\n== Applying Universal Matter v4.8 to the index ==\n'
git apply --index "$PATCH"

check_postimage_prefix() {
    local path="$1"
    local expected="$2"
    local actual
    actual="$(git hash-object "$path")"
    if [[ "${actual:0:${#expected}}" != "$expected" ]]; then
        printf 'ERROR: postimage mismatch for %s\nexpected prefix: %s\nactual:          %s\n' \
            "$path" "$expected" "$actual" >&2
        exit 1
    fi
}

check_postimage_prefix crates/domains/symtropy-terrain/Cargo.toml e93c0cc
check_postimage_prefix crates/domains/symtropy-terrain/src/lib.rs e138e1d
check_postimage_prefix docs/canon/CONSTRUCTION_REPAIR_AND_STRUCTURAL_TRANSFORMATION_CONTRACT_V0_1.md 8ee0b22
check_postimage_prefix docs/canon/FIRSTLIGHT_CORE_ACTION_DANGER_COMBAT_RESCUE_AND_DESTRUCTION_CONTRACT_V0_1.md 0b47b48
check_postimage_prefix docs/ops/FIRSTLIGHT_VERTICAL_SLICE_IMPLEMENTATION_TICKET_BACKLOG_V0_1.md 8b33e7d
check_postimage_prefix docs/tech/STRUCTURAL_INTEGRITY_CONSTRUCTION_AND_DESTRUCTION_RUNTIME_V0_1.md b3f3f40

staged_count="$(git diff --cached --name-only | wc -l | tr -d ' ')"
if [[ "$staged_count" -ne 275 ]]; then
    printf 'ERROR: expected exactly 275 staged v4.8 paths; found %s\n' "$staged_count" >&2
    exit 1
fi

printf '\nPASS: Universal Matter v4.8 replayed and staged, but NOT committed.\n'
printf 'Review with: git diff --cached --stat\n'
printf 'Qualify with: nix develop --command bash scripts/qualify-universal-matter-v4.8-cuf.sh\n'
printf 'Commit only after qualification succeeds.\n'
