#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Verify that a captured Universal Matter v4.8 qualification capsule proves the
# exact currently staged tree. This script is non-mutating.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

EVIDENCE="${1:-}"
EXPECTED_PATCH_SHA256="23f6baf3545bace49252eee190f181fa8a88c650d2994b72b65bdaf83cc74637"
EXPECTED_STAGED_PATHS="275"

if [[ -z "$EVIDENCE" || ! -d "$EVIDENCE" ]]; then
    printf 'usage: %s /path/to/qualification-evidence-dir\n' "$0" >&2
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
    QUALIFICATION.log
    QUALIFICATION.err.log
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
    sha256sum -c MANIFEST.sha256
)

if [[ "$(cat "$EVIDENCE/STATUS.txt")" != "PASS" ]]; then
    printf 'ERROR: qualification capsule is not PASS.\n' >&2
    cat "$EVIDENCE/STATUS.txt" >&2
    exit 1
fi

if [[ "$(cat "$EVIDENCE/UNIVERSAL_MATTER_V4_8_PATCH_SHA256.txt")" != "$EXPECTED_PATCH_SHA256" ]]; then
    printf 'ERROR: qualification capsule binds the wrong v4.8 patch.\n' >&2
    exit 1
fi

if [[ "$(cat "$EVIDENCE/STAGED_PATH_COUNT.txt")" != "$EXPECTED_STAGED_PATHS" ]]; then
    printf 'ERROR: qualification capsule staged path count is not %s.\n' "$EXPECTED_STAGED_PATHS" >&2
    exit 1
fi

base_head="$(cat "$EVIDENCE/BASE_HEAD.txt")"
qualified_tree="$(cat "$EVIDENCE/STAGED_TREE.txt")"
qualified_tree_after="$(cat "$EVIDENCE/STAGED_TREE_AFTER.txt")"
current_head="$(git rev-parse HEAD)"
current_tree="$(git write-tree)"

if [[ "$base_head" != "$current_head" ]]; then
    printf 'ERROR: current HEAD differs from the qualification parent.\nexpected: %s\nactual:   %s\n' \
        "$base_head" "$current_head" >&2
    exit 1
fi

if [[ "$qualified_tree" != "$qualified_tree_after" ]]; then
    printf 'ERROR: staged tree changed while qualification was running.\n' >&2
    exit 1
fi

if [[ "$qualified_tree" != "$current_tree" ]]; then
    printf 'ERROR: current staged tree differs from the qualified tree.\nexpected: %s\nactual:   %s\n' \
        "$qualified_tree" "$current_tree" >&2
    exit 1
fi

if ! cmp -s "$EVIDENCE/CARGO_LOCK_SHA256.txt" "$EVIDENCE/CARGO_LOCK_SHA256_AFTER.txt"; then
    printf 'ERROR: Cargo.lock identity changed during qualification.\n' >&2
    exit 1
fi

current_lock="$(sha256sum Cargo.lock | awk '{print $1}')"
capsule_lock="$(awk '{print $1}' "$EVIDENCE/CARGO_LOCK_SHA256_AFTER.txt")"
if [[ "$current_lock" != "$capsule_lock" ]]; then
    printf 'ERROR: current Cargo.lock differs from the qualified capsule.\n' >&2
    exit 1
fi

if ! cmp -s "$EVIDENCE/GIT_STATUS_BEFORE.txt" "$EVIDENCE/GIT_STATUS_AFTER.txt"; then
    printf 'ERROR: repository status changed during qualification.\n' >&2
    exit 1
fi

current_status="$(mktemp)"
trap 'rm -f "$current_status"' EXIT
git status --short > "$current_status"
if ! cmp -s "$EVIDENCE/GIT_STATUS_AFTER.txt" "$current_status"; then
    printf 'ERROR: repository status no longer matches the qualified capsule.\n' >&2
    exit 1
fi

printf 'PASS: qualification capsule proves current staged Universal Matter v4.8 tree\n'
printf 'Parent HEAD:    %s\n' "$base_head"
printf 'Qualified tree: %s\n' "$qualified_tree"
printf 'Staged paths:   %s\n' "$EXPECTED_STAGED_PATHS"
printf '\nThe code tree is eligible for a promotion commit. The commit must have\n'
printf 'parent %s and tree %s.\n' "$base_head" "$qualified_tree"
