#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Qualification wrapper for the exact descendant intended to become the parent
# of native CUF v0.11 adapters. Universal Matter v4.8 must already be applied
# and staged through the guarded replay helpers before this script is invoked.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

TOOLING_BEFORE="$(mktemp /tmp/cuf-v011-tooling-before.XXXXXX)"
TOOLING_AFTER="$(mktemp /tmp/cuf-v011-tooling-after.XXXXXX)"
trap 'rm -f "$TOOLING_BEFORE" "$TOOLING_AFTER"' EXIT

printf 'CUF v0.11 parent-candidate qualification head: %s\n' "$(git rev-parse HEAD)"

printf '\n== Candidate composition ==\n'
bash scripts/verify-cuf-v0.11-parent-candidate.sh

printf '\n== Qualification tooling identity ==\n'
bash scripts/verify-cuf-v0.11-qualification-tooling.sh | tee "$TOOLING_BEFORE"

staged_tree_before="$(git write-tree)"
cargo_lock_before="$(git hash-object Cargo.lock)"
status_before="$(git status --short)"

printf '\n== Q0/Q1 Universal Matter v4.8 + CUF v0.10.1 ==\n'
bash scripts/qualify-universal-matter-v4.8-cuf-v0.10.1.sh

printf '\n== Finalized dependency-light world continuation core ==\n'
bash scripts/qualify-world-continuation-core-v0.1.sh

printf '\n== Candidate identity after qualification ==\n'
bash scripts/verify-cuf-v0.11-parent-candidate.sh
bash scripts/verify-cuf-v0.11-qualification-tooling.sh | tee "$TOOLING_AFTER"

if ! cmp -s "$TOOLING_BEFORE" "$TOOLING_AFTER"; then
    printf 'ERROR: qualification tooling identity changed during qualification.\n' >&2
    diff -u "$TOOLING_BEFORE" "$TOOLING_AFTER" >&2 || true
    exit 1
fi

staged_tree_after="$(git write-tree)"
if [[ "$staged_tree_after" != "$staged_tree_before" ]]; then
    printf 'ERROR: staged candidate tree changed during parent qualification.\nBefore: %s\nAfter:  %s\n' \
        "$staged_tree_before" "$staged_tree_after" >&2
    exit 1
fi

cargo_lock_after="$(git hash-object Cargo.lock)"
if [[ "$cargo_lock_after" != "$cargo_lock_before" ]]; then
    printf 'ERROR: Cargo.lock changed during parent qualification.\nBefore: %s\nAfter:  %s\n' \
        "$cargo_lock_before" "$cargo_lock_after" >&2
    exit 1
fi

status_after="$(git status --short)"
if [[ "$status_after" != "$status_before" ]]; then
    printf 'ERROR: repository status changed during parent qualification.\n' >&2
    diff -u <(printf '%s\n' "$status_before") <(printf '%s\n' "$status_after") >&2 || true
    exit 1
fi

if ! git diff --quiet; then
    printf 'ERROR: parent qualification produced unstaged tracked changes.\n' >&2
    git diff --stat >&2
    exit 1
fi

untracked="$(git ls-files --others --exclude-standard)"
if [[ -n "$untracked" ]]; then
    printf 'ERROR: parent qualification produced untracked files:\n%s\n' "$untracked" >&2
    exit 1
fi

git diff --check
git diff --cached --check

printf '\nPASS: CUF v0.11 parent-candidate Q0/Q1 + continuation-core gates\n'
printf 'Candidate HEAD:        %s\n' "$(git rev-parse HEAD)"
printf 'Candidate staged tree: %s\n' "$staged_tree_after"
printf 'Cargo.lock unchanged:  %s\n' "$cargo_lock_after"
printf 'Tooling identity:       exact committed HEAD blobs\n'
printf 'This is NOT a Q2 continuation/replay PASS; #76/#79/#81 remain required.\n'
