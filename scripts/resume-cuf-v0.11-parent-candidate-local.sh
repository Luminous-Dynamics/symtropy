#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Resume CUF v0.11 qualification from an already-staged exact Universal Matter
# v4.8 replay. This never reapplies, resets, cleans, commits, or unstages state.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

PATCH="${1:-}"
OUT="${2:-}"
FROZEN_EVIDENCE_INTEGRITY_HEAD="55ef050600cb051f676c0798518e56093eaa87cf"
EXPECTED_PATCH_SHA256="23f6baf3545bace49252eee190f181fa8a88c650d2994b72b65bdaf83cc74637"
EXPECTED_STAGED_PATHS=275
PRIVATE_MULTIWORLD="../mycelix-multiworld-sim"
LOCK_DIR="$(git rev-parse --git-path cuf-v0.11-parent-candidate.lock)"
lock_acquired=0
patch_paths=""
staged_paths=""

usage() {
    printf 'usage: %s /path/to/SYMTROPY_UNIVERSAL_MATTER_V4.8_CUMULATIVE.patch [new-evidence-dir]\n' "$0" >&2
}

die() {
    printf 'ERROR: %s\n' "$*" >&2
    exit 1
}

show_preserved_state() {
    local status="${1:-unknown}"
    printf '\n=== CUF v0.11 resume %s ===\n' "$status" >&2
    printf 'HEAD: %s\n' "$(git rev-parse HEAD 2>/dev/null || printf unknown)" >&2
    printf 'Staged paths: %s\n' "$(git diff --cached --name-only 2>/dev/null | wc -l | tr -d ' ')" >&2
    printf 'No automatic reset/clean/commit/unstage was performed.\n' >&2
    if [[ -n "$OUT" && -e "$OUT" ]]; then
        printf 'Evidence: %s\n' "$(realpath -m "$OUT")" >&2
        if [[ -f "$OUT/STATUS.txt" ]]; then
            printf 'Evidence status: '
            cat "$OUT/STATUS.txt" >&2
        fi
    fi
}

cleanup() {
    local status=$?
    [[ -n "$patch_paths" ]] && rm -f "$patch_paths" 2>/dev/null || true
    [[ -n "$staged_paths" ]] && rm -f "$staged_paths" 2>/dev/null || true
    if [[ "$lock_acquired" -eq 1 ]]; then
        rm -f "$LOCK_DIR/owner" 2>/dev/null || true
        rmdir "$LOCK_DIR" 2>/dev/null || true
        lock_acquired=0
    fi
    if [[ "$status" -ne 0 ]]; then
        show_preserved_state 'FAILED/INTERRUPTED'
    fi
    return "$status"
}

trap cleanup EXIT
trap 'exit 130' INT TERM

if [[ -z "$PATCH" || ! -f "$PATCH" ]]; then
    usage
    exit 2
fi
PATCH="$(realpath "$PATCH")"

for cmd in git sha256sum realpath nix bash awk sort cmp wc date mktemp mkdir rmdir; do
    command -v "$cmd" >/dev/null 2>&1 || die "required command unavailable: $cmd"
done

if ! mkdir "$LOCK_DIR" 2>/dev/null; then
    printf 'Existing CUF v0.11 execution lock: %s\n' "$LOCK_DIR" >&2
    [[ -f "$LOCK_DIR/owner" ]] && cat "$LOCK_DIR/owner" >&2 || true
    die 'another fresh/resume qualification may be active; verify before removing a stale lock manually'
fi
lock_acquired=1
{
    printf 'pid=%s\n' "$$"
    printf 'mode=resume\n'
    printf 'started_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf 'head=%s\n' "$(git rev-parse HEAD^{commit})"
} > "$LOCK_DIR/owner"

current_head="$(git rev-parse HEAD^{commit})"
if ! git merge-base --is-ancestor "$FROZEN_EVIDENCE_INTEGRITY_HEAD" "$current_head"; then
    die "HEAD is not a descendant of frozen evidence-integrity head $FROZEN_EVIDENCE_INTEGRITY_HEAD"
fi

actual_patch_sha="$(sha256sum "$PATCH" | awk '{print $1}')"
[[ "$actual_patch_sha" == "$EXPECTED_PATCH_SHA256" ]] \
    || die "retained v4.8 patch SHA-256 mismatch: $actual_patch_sha"
[[ -d "$PRIVATE_MULTIWORLD" ]] || die "required private sibling is absent: $PRIVATE_MULTIWORLD"

staged_count="$(git diff --cached --name-only | wc -l | tr -d ' ')"
if [[ "$staged_count" -ne "$EXPECTED_STAGED_PATHS" ]]; then
    die "resume requires exactly $EXPECTED_STAGED_PATHS staged paths; found $staged_count"
fi
if ! git diff --quiet; then
    git diff --stat >&2
    die 'resume refuses unstaged tracked changes'
fi
untracked="$(git ls-files --others --exclude-standard)"
if [[ -n "$untracked" ]]; then
    printf 'Untracked paths:\n%s\n' "$untracked" >&2
    die 'resume refuses untracked files'
fi

patch_paths="$(mktemp)"
staged_paths="$(mktemp)"
git apply --numstat "$PATCH" | cut -f3- | LC_ALL=C sort -u > "$patch_paths"
git diff --cached --name-only | LC_ALL=C sort -u > "$staged_paths"
patch_count="$(wc -l < "$patch_paths" | tr -d ' ')"
staged_unique_count="$(wc -l < "$staged_paths" | tr -d ' ')"
if [[ "$patch_count" -ne "$EXPECTED_STAGED_PATHS" \
   || "$staged_unique_count" -ne "$EXPECTED_STAGED_PATHS" ]]; then
    die "resume path-count mismatch: patch=$patch_count staged=$staged_unique_count expected=$EXPECTED_STAGED_PATHS"
fi
if ! cmp -s "$patch_paths" "$staged_paths"; then
    printf 'Staged path set differs from retained v4.8 artifact:\n' >&2
    diff -u "$patch_paths" "$staged_paths" >&2 || true
    die 'resume refuses a non-canonical staged path set'
fi

if [[ -z "$OUT" ]]; then
    OUT="/tmp/symtropy-cuf-v0.11-parent-resume-$(date -u +%Y%m%dT%H%M%SZ)"
fi
OUT="$(realpath -m "$OUT")"
case "$OUT/" in
    "$(realpath "$ROOT")/"*) die 'evidence output must be outside the repository worktree' ;;
esac
[[ ! -e "$OUT" ]] || die "evidence output already exists: $OUT"

staged_tree_before_readiness="$(git write-tree)"
printf '=== CUF v0.11 resume from exact staged replay ===\n'
printf 'HEAD:        %s\n' "$current_head"
printf 'Patch SHA:   %s\n' "$actual_patch_sha"
printf 'Staged tree: %s\n' "$staged_tree_before_readiness"
printf 'Evidence:    %s\n' "$OUT"
printf 'Run lock:    %s\n' "$LOCK_DIR"

printf '\n== Candidate/tooling identity ==\n'
bash scripts/verify-cuf-v0.11-parent-candidate.sh
bash scripts/verify-cuf-v0.11-qualification-tooling.sh >/dev/null
printf 'PASS: staged replay and qualification tooling are canonical\n'

printf '\n== Nix development environment readiness ==\n'
nix develop --command bash -lc 'set -euo pipefail; rustc --version; cargo --version'
staged_tree_after_readiness="$(git write-tree)"
if [[ "$staged_tree_after_readiness" != "$staged_tree_before_readiness" ]]; then
    die 'Nix readiness check changed the staged candidate tree'
fi
if ! git diff --quiet || [[ -n "$(git ls-files --others --exclude-standard)" ]]; then
    die 'Nix readiness check changed tracked/untracked working-tree state'
fi

printf '\n== Resume evidence capture ==\n'
set +e
bash scripts/capture-cuf-v0.11-parent-candidate-evidence.sh "$PATCH" "$OUT"
status=$?
set -e

if [[ "$status" -ne 0 ]]; then
    exit "$status"
fi
if [[ ! -f "$OUT/STATUS.txt" || "$(cat "$OUT/STATUS.txt")" != "PASS" ]]; then
    die 'capture returned success but evidence STATUS.txt is not PASS'
fi

printf '\n=== PASS: resumed parent-candidate qualification ===\n'
printf 'Evidence: %s\n' "$OUT"
printf 'Staged tree is still uncommitted. Review it before promotion.\n'
printf "Promotion command after explicit review:\n  git commit -m 'feat(terrain): replay Universal Matter v4.8 authored tree'\n"
printf '  bash scripts/verify-cuf-v0.11-parent-promotion.sh %q HEAD\n' "$OUT"
printf 'Q2 remains NOT_CLAIMED.\n'
