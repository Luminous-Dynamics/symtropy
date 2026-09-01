#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Fail-closed preflight for the authored Universal Matter v4.8 cumulative patch.
# This script does not mutate the repository.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

PATCH="${1:-}"
if [[ -z "$PATCH" ]]; then
    printf 'usage: %s /path/to/SYMTROPY_UNIVERSAL_MATTER_V4.8_CUMULATIVE.patch\n' "$0" >&2
    exit 2
fi
if [[ ! -f "$PATCH" ]]; then
    printf 'ERROR: patch not found: %s\n' "$PATCH" >&2
    exit 1
fi

EXPECTED_PATCH_SHA256="23f6baf3545bace49252eee190f181fa8a88c650d2994b72b65bdaf83cc74637"
EXPECTED_DIFFS=275
EXPECTED_NEW_FILES=269
EXPECTED_MODIFIED_FILES=6

printf 'Universal Matter v4.8 preflight head: %s\n' "$(git rev-parse HEAD)"

if [[ -n "$(git status --porcelain)" ]]; then
    printf 'ERROR: working tree must be clean before v4.8 preflight.\n' >&2
    exit 1
fi

actual_patch_sha="$(sha256sum "$PATCH" | awk '{print $1}')"
if [[ "$actual_patch_sha" != "$EXPECTED_PATCH_SHA256" ]]; then
    printf 'ERROR: patch SHA-256 mismatch.\nexpected: %s\nactual:   %s\n' \
        "$EXPECTED_PATCH_SHA256" "$actual_patch_sha" >&2
    exit 1
fi

actual_diffs="$(grep -c '^diff --git a/' "$PATCH")"
actual_new="$(grep -c '^new file mode ' "$PATCH")"
actual_deleted="$(grep -c '^deleted file mode ' "$PATCH" || true)"
actual_modified="$((actual_diffs - actual_new - actual_deleted))"

if [[ "$actual_diffs" -ne "$EXPECTED_DIFFS" \
   || "$actual_new" -ne "$EXPECTED_NEW_FILES" \
   || "$actual_modified" -ne "$EXPECTED_MODIFIED_FILES" \
   || "$actual_deleted" -ne 0 ]]; then
    printf 'ERROR: patch shape mismatch: diffs=%s new=%s modified=%s deleted=%s\n' \
        "$actual_diffs" "$actual_new" "$actual_modified" "$actual_deleted" >&2
    exit 1
fi

check_blob_prefix() {
    local path="$1"
    local expected="$2"
    if [[ ! -f "$path" ]]; then
        printf 'ERROR: expected preimage is missing: %s\n' "$path" >&2
        exit 1
    fi
    local actual
    actual="$(git hash-object "$path")"
    if [[ "${actual:0:${#expected}}" != "$expected" ]]; then
        printf 'ERROR: preimage mismatch for %s\nexpected prefix: %s\nactual:          %s\n' \
            "$path" "$expected" "$actual" >&2
        exit 1
    fi
}

check_blob_prefix crates/domains/symtropy-terrain/Cargo.toml 6b3dd8c
check_blob_prefix crates/domains/symtropy-terrain/src/lib.rs bbf60dd
check_blob_prefix docs/canon/CONSTRUCTION_REPAIR_AND_STRUCTURAL_TRANSFORMATION_CONTRACT_V0_1.md 1bdea3d
check_blob_prefix docs/canon/FIRSTLIGHT_CORE_ACTION_DANGER_COMBAT_RESCUE_AND_DESTRUCTION_CONTRACT_V0_1.md 9496c13
check_blob_prefix docs/ops/FIRSTLIGHT_VERTICAL_SLICE_IMPLEMENTATION_TICKET_BACKLOG_V0_1.md c3237a3
check_blob_prefix docs/tech/STRUCTURAL_INTEGRITY_CONSTRUCTION_AND_DESTRUCTION_RUNTIME_V0_1.md 5d0ab77

# Every path declared by the cumulative patch as a new file must still be absent.
while IFS= read -r new_path; do
    if [[ -e "$new_path" ]]; then
        printf 'ERROR: v4.8 new-file target already exists: %s\n' "$new_path" >&2
        exit 1
    fi
done < <(
    awk '
        /^diff --git a\// { path=$3; sub(/^a\//, "", path); next }
        /^new file mode / { print path }
    ' "$PATCH"
)

printf '\n== git apply structural check ==\n'
git apply --check "$PATCH"

printf '\nPASS: Universal Matter v4.8 patch is structurally applicable\n'
printf 'Patch SHA-256: %s\n' "$actual_patch_sha"
printf 'Patch shape: %s diffs; %s new; %s modified; 0 deleted\n' \
    "$actual_diffs" "$actual_new" "$actual_modified"
