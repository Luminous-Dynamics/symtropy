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

printf 'CUF v0.11 parent-candidate qualification head: %s\n' "$(git rev-parse HEAD)"

printf '\n== Candidate composition ==\n'
bash scripts/verify-cuf-v0.11-parent-candidate.sh

staged_tree_before="$(git write-tree)"
cargo_lock_before="$(git hash-object Cargo.lock)"

printf '\n== Q0/Q1 Universal Matter v4.8 + CUF v0.10.1 ==\n'
bash scripts/qualify-universal-matter-v4.8-cuf-v0.10.1.sh

printf '\n== Finalized dependency-light world continuation core ==\n'
bash scripts/qualify-world-continuation-core-v0.1.sh

printf '\n== Candidate identity after qualification ==\n'
bash scripts/verify-cuf-v0.11-parent-candidate.sh

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
printf 'Candidate HEAD:       %s\n' "$(git rev-parse HEAD)"
printf 'Candidate staged tree:%s\n' "$staged_tree_after"
printf 'Cargo.lock unchanged: %s\n' "$cargo_lock_after"
printf 'This is NOT a Q2 continuation/replay PASS; #76/#79/#81 remain required.\n'
