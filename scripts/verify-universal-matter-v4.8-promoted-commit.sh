#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Verify that a promotion commit exactly commits the tree proven by a PASS
# qualification capsule and preserves the recorded qualification parent.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

EVIDENCE="${1:-}"
COMMIT="${2:-HEAD}"
EXPECTED_PATCH_SHA256="23f6baf3545bace49252eee190f181fa8a88c650d2994b72b65bdaf83cc74637"
EXPECTED_STAGED_PATHS="275"

if [[ -z "$EVIDENCE" || ! -d "$EVIDENCE" ]]; then
    printf 'usage: %s /path/to/qualification-evidence-dir [commit]\n' "$0" >&2
    exit 2
fi
EVIDENCE="$(realpath "$EVIDENCE")"

required=(
    BASE_HEAD.txt
    STAGED_TREE.txt
    STAGED_TREE_AFTER.txt
    UNIVERSAL_MATTER_V4_8_PATCH_SHA256.txt
    CARGO_LOCK_SHA256.txt
    CARGO_LOCK_SHA256_AFTER.txt
    STAGED_PATH_COUNT.txt
    GIT_STATUS_BEFORE.txt
    GIT_STATUS_AFTER.txt
    STATUS.txt
    MANIFEST.sha256
)
for name in "${required[@]}"; do
    if [[ ! -f "$EVIDENCE/$name" ]]; then
        printf 'ERROR: qualification capsule missing %s\n' "$name" >&2
        exit 1
    fi
done

(
    cd "$EVIDENCE"
    sha256sum -c MANIFEST.sha256 >/dev/null
)

if [[ "$(cat "$EVIDENCE/STATUS.txt")" != "PASS" ]]; then
    printf 'ERROR: qualification capsule is not PASS.\n' >&2
    exit 1
fi

if [[ "$(cat "$EVIDENCE/UNIVERSAL_MATTER_V4_8_PATCH_SHA256.txt")" != "$EXPECTED_PATCH_SHA256" ]]; then
    printf 'ERROR: qualification capsule binds the wrong retained patch.\n' >&2
    exit 1
fi

if [[ "$(cat "$EVIDENCE/STAGED_PATH_COUNT.txt")" != "$EXPECTED_STAGED_PATHS" ]]; then
    printf 'ERROR: qualified staged path count is not %s.\n' "$EXPECTED_STAGED_PATHS" >&2
    exit 1
fi

if ! cmp -s "$EVIDENCE/CARGO_LOCK_SHA256.txt" "$EVIDENCE/CARGO_LOCK_SHA256_AFTER.txt"; then
    printf 'ERROR: Cargo.lock changed during qualification.\n' >&2
    exit 1
fi

if ! cmp -s "$EVIDENCE/GIT_STATUS_BEFORE.txt" "$EVIDENCE/GIT_STATUS_AFTER.txt"; then
    printf 'ERROR: repository status changed during qualification.\n' >&2
    exit 1
fi

qualified_parent="$(cat "$EVIDENCE/BASE_HEAD.txt")"
qualified_tree="$(cat "$EVIDENCE/STAGED_TREE.txt")"
qualified_tree_after="$(cat "$EVIDENCE/STAGED_TREE_AFTER.txt")"
commit_sha="$(git rev-parse "$COMMIT^{commit}")"
commit_tree="$(git rev-parse "$commit_sha^{tree}")"
parent_count="$(git rev-list --parents -n 1 "$commit_sha" | awk '{print NF-1}')"
first_parent="$(git rev-parse "$commit_sha^1")"

if [[ "$qualified_tree" != "$qualified_tree_after" ]]; then
    printf 'ERROR: capsule recorded different staged trees before/after qualification.\n' >&2
    exit 1
fi

if [[ "$parent_count" != "1" ]]; then
    printf 'ERROR: promotion commit must have exactly one parent; got %s.\n' "$parent_count" >&2
    exit 1
fi

if [[ "$first_parent" != "$qualified_parent" ]]; then
    printf 'ERROR: promotion parent mismatch.\nexpected: %s\nactual:   %s\n' \
        "$qualified_parent" "$first_parent" >&2
    exit 1
fi

if [[ "$commit_tree" != "$qualified_tree" ]]; then
    printf 'ERROR: promotion commit tree differs from qualified tree.\nexpected: %s\nactual:   %s\n' \
        "$qualified_tree" "$commit_tree" >&2
    exit 1
fi

capsule_lock="$(awk '{print $1}' "$EVIDENCE/CARGO_LOCK_SHA256_AFTER.txt")"
commit_lock="$(git show "$commit_sha:Cargo.lock" | sha256sum | awk '{print $1}')"
if [[ "$commit_lock" != "$capsule_lock" ]]; then
    printf 'ERROR: promoted commit Cargo.lock differs from qualified lockfile.\n' >&2
    exit 1
fi

printf 'PASS: promoted commit exactly matches qualified Universal Matter v4.8 tree\n'
printf 'Commit:         %s\n' "$commit_sha"
printf 'Parent:         %s\n' "$first_parent"
printf 'Qualified tree: %s\n' "$commit_tree"
printf 'Patch SHA-256:  %s\n' "$EXPECTED_PATCH_SHA256"
