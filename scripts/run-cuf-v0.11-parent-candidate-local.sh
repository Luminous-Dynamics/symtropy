#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Fail-safe local operator harness for the CUF v0.11 pre-Q2 parent candidate.
#
# This script intentionally does NOT commit, reset, clean, stash, or otherwise
# hide the staged Universal Matter replay. On failure it leaves the staged tree
# and evidence in place for inspection.

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
mutation_started=0

usage() {
    printf 'usage: %s /path/to/SYMTROPY_UNIVERSAL_MATTER_V4.8_CUMULATIVE.patch [evidence-dir]\n' "$0" >&2
}

die() {
    printf 'ERROR: %s\n' "$*" >&2
    exit 1
}

show_preserved_state() {
    local status="${1:-unknown}"
    printf '\n=== CUF v0.11 local run %s ===\n' "$status" >&2
    printf 'HEAD: %s\n' "$(git rev-parse HEAD 2>/dev/null || printf unknown)" >&2
    printf 'Staged paths: %s\n' "$(git diff --cached --name-only 2>/dev/null | wc -l | tr -d ' ')" >&2
    printf 'No automatic reset/clean/commit was performed.\n' >&2
    if [[ -n "$OUT" && -e "$OUT" ]]; then
        printf 'Evidence: %s\n' "$(realpath -m "$OUT")" >&2
        if [[ -f "$OUT/STATUS.txt" ]]; then
            printf 'Evidence status: '
            cat "$OUT/STATUS.txt" >&2
        fi
    fi
    printf 'Inspect with: git status --short && git diff --cached --stat\n' >&2
}

release_lock() {
    if [[ "$lock_acquired" -eq 1 ]]; then
        rm -f "$LOCK_DIR/owner" 2>/dev/null || true
        rmdir "$LOCK_DIR" 2>/dev/null || true
        lock_acquired=0
    fi
}

on_exit() {
    local status=$?
    if [[ "$status" -ne 0 && "$mutation_started" -eq 1 ]]; then
        show_preserved_state 'FAILED/INTERRUPTED'
    fi
    release_lock
    return "$status"
}

trap on_exit EXIT
trap 'exit 130' INT TERM

if [[ -z "$PATCH" || ! -f "$PATCH" ]]; then
    usage
    exit 2
fi
PATCH="$(realpath "$PATCH")"

for cmd in git sha256sum realpath nix bash awk sort cmp wc date mkdir rmdir; do
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
    printf 'mode=fresh\n'
    printf 'started_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf 'head=%s\n' "$(git rev-parse HEAD^{commit})"
} > "$LOCK_DIR/owner"

current_head="$(git rev-parse HEAD^{commit})"
if ! git merge-base --is-ancestor "$FROZEN_EVIDENCE_INTEGRITY_HEAD" "$current_head"; then
    die "HEAD is not a descendant of frozen evidence-integrity head $FROZEN_EVIDENCE_INTEGRITY_HEAD"
fi

actual_patch_sha="$(sha256sum "$PATCH" | awk '{print $1}')"
if [[ "$actual_patch_sha" != "$EXPECTED_PATCH_SHA256" ]]; then
    die "retained v4.8 patch SHA-256 mismatch: $actual_patch_sha"
fi

if [[ ! -d "$PRIVATE_MULTIWORLD" ]]; then
    die "required private sibling is absent: $PRIVATE_MULTIWORLD"
fi

if [[ -n "$(git status --porcelain)" ]]; then
    printf 'Current repository state:\n' >&2
    git status --short >&2
    die 'local execution requires a completely clean starting worktree/index'
fi

if [[ -z "$OUT" ]]; then
    OUT="/tmp/symtropy-cuf-v0.11-parent-run-$(date -u +%Y%m%dT%H%M%SZ)"
fi
OUT="$(realpath -m "$OUT")"
case "$OUT/" in
    "$(realpath "$ROOT")/"*) die 'evidence output must be outside the repository worktree' ;;
esac
[[ ! -e "$OUT" ]] || die "evidence output already exists: $OUT"

printf '=== CUF v0.11 parent-candidate local execution ===\n'
printf 'HEAD:       %s\n' "$current_head"
printf 'Patch:      %s\n' "$PATCH"
printf 'Patch SHA:  %s\n' "$actual_patch_sha"
printf 'Evidence:   %s\n' "$OUT"
printf 'Repository: %s\n' "$ROOT"
printf 'Run lock:   %s\n' "$LOCK_DIR"
printf '\nFilesystem snapshot:\n'
df -h "$ROOT" || true
if command -v free >/dev/null 2>&1; then
    printf '\nMemory snapshot:\n'
    free -h || true
fi

printf '\n== Candidate/tooling identity ==\n'
bash scripts/verify-cuf-v0.11-parent-candidate.sh
bash scripts/verify-cuf-v0.11-qualification-tooling.sh >/dev/null
printf 'PASS: candidate and qualification tooling match committed HEAD\n'

printf '\n== Nix development environment readiness ==\n'
nix develop --command bash -lc 'set -euo pipefail; rustc --version; cargo --version'
if [[ -n "$(git status --porcelain)" ]]; then
    git status --short >&2
    die 'Nix readiness check changed repository state'
fi

printf '\n== Pre-v4.8 parent Cargo.lock coherence ==\n'
# Detect a pre-existing parent lock defect before staging the 275-file retained
# artifact. A failure here is a parent-lineage prerequisite failure, not evidence
# that authored Universal Matter v4.8 itself failed Q1.
if ! nix develop --command bash -lc \
    'set -euo pipefail; cargo metadata --locked --no-deps --format-version 1 >/dev/null'; then
    die 'pre-v4.8 parent is not representable by committed Cargo.lock; do not stage v4.8. Preserve this prerequisite failure and follow the explicit Q1 repair lane (#119)'
fi
if [[ -n "$(git status --porcelain)" ]]; then
    git status --short >&2
    die 'parent Cargo.lock coherence check changed repository state'
fi
printf 'PASS: pre-v4.8 parent resolves under committed Cargo.lock\n'

printf '\n== Q0 preflight ==\n'
bash scripts/preflight-universal-matter-v4.8.sh "$PATCH"
if [[ -n "$(git status --porcelain)" ]]; then
    git status --short >&2
    die 'preflight was expected to be non-mutating but changed repository state'
fi

printf '\n== Stage exact retained v4.8 replay ==\n'
mutation_started=1
bash scripts/apply-universal-matter-v4.8.sh "$PATCH"

staged_count="$(git diff --cached --name-only | wc -l | tr -d ' ')"
if [[ "$staged_count" -ne "$EXPECTED_STAGED_PATHS" ]]; then
    die "expected $EXPECTED_STAGED_PATHS staged paths after replay; found $staged_count"
fi
if ! git diff --quiet; then
    die 'unexpected unstaged tracked changes after replay'
fi
untracked="$(git ls-files --others --exclude-standard)"
if [[ -n "$untracked" ]]; then
    printf 'Unexpected untracked paths:\n%s\n' "$untracked" >&2
    die 'unexpected untracked files after replay'
fi

printf '\n== Capture Q0/Q1 + continuation-core evidence ==\n'
# The capture script remains in the host shell. It independently queries the
# Nix environment for TOOLCHAIN.txt and enters nix develop for qualification,
# avoiding a redundant nested nix-develop wrapper.
set +e
bash scripts/capture-cuf-v0.11-parent-candidate-evidence.sh "$PATCH" "$OUT"
status=$?
set -e

if [[ "$status" -ne 0 ]]; then
    exit "$status"
fi

if [[ ! -f "$OUT/STATUS.txt" || "$(cat "$OUT/STATUS.txt")" != "PASS" ]]; then
    die 'capture command returned success but evidence STATUS.txt is not PASS'
fi

printf '\n=== PASS: pre-Q2 parent candidate qualified ===\n'
printf 'Evidence: %s\n' "$OUT"
printf 'Staged tree remains uncommitted for explicit review/promotion.\n'
printf '\nNext, inspect the exact replay:\n'
printf '  git status --short\n'
printf '  git diff --cached --stat\n'
printf '\nIf and only if the evidence is accepted, create the one-parent promotion commit:\n'
printf "  git commit -m 'feat(terrain): replay Universal Matter v4.8 authored tree'\n"
printf '  bash scripts/verify-cuf-v0.11-parent-promotion.sh %q HEAD\n' "$OUT"
printf '\nDo not begin #76/#79/#81 until the promotion verifier passes. Q2 is still NOT_CLAIMED.\n'
