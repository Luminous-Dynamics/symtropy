#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Generate an explicit Cargo.lock-only repair for CUF v0.11 Stage A or Stage B.
# This script never commits, stages, resets, cleans, or stashes. It mutates only
# after Cargo itself classifies the locked precheck as a lock-update requirement.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

CONTEXT="${1:-}"
OUT="${2:-}"
LOCK_DIR="$(git rev-parse --git-path cuf-v0.11-parent-candidate.lock)"
lock_acquired=0
mutation_started=0

usage() {
    printf 'usage: %s <parent|v4.8> [evidence-dir]\n' "$0" >&2
}

die() {
    printf 'ERROR: %s\n' "$*" >&2
    exit 1
}

finalize_manifest() {
    [[ -n "$OUT" && -d "$OUT" ]] || return 0
    (
        cd "$OUT"
        find . -maxdepth 1 -type f ! -name MANIFEST.sha256 -printf '%P\n' \
            | LC_ALL=C sort | xargs -r sha256sum > MANIFEST.sha256
    )
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
    if [[ "$status" -ne 0 && -n "$OUT" && -d "$OUT" ]]; then
        [[ -f "$OUT/STATUS.txt" ]] || printf 'FAILED\n' > "$OUT/STATUS.txt"
        finalize_manifest || true
    fi
    if [[ "$status" -ne 0 && "$mutation_started" -eq 1 ]]; then
        printf '\nCargo.lock repair failed after mutation began.\n' >&2
        printf 'No reset/clean/stash/commit was performed. Inspect with:\n' >&2
        printf '  git status --short\n  git diff -- Cargo.lock\n' >&2
        if [[ -n "$OUT" && -d "$OUT" ]]; then
            printf 'Evidence: %s\n' "$OUT" >&2
        fi
    fi
    release_lock
    return "$status"
}
trap on_exit EXIT
trap 'exit 130' INT TERM

case "$CONTEXT" in
    parent|v4.8) ;;
    *) usage; exit 2 ;;
esac

for cmd in git sha256sum realpath nix bash awk sort cmp wc date find xargs grep tee mkdir rmdir rm; do
    command -v "$cmd" >/dev/null 2>&1 || die "required command unavailable: $cmd"
done

[[ -f Cargo.toml ]] || die 'root Cargo.toml is missing'
[[ -f Cargo.lock ]] || die 'Cargo.lock is missing; this harness repairs an existing lockfile only'
git ls-files --error-unmatch Cargo.lock >/dev/null 2>&1 || die 'Cargo.lock is not tracked'
[[ -z "$(git status --porcelain)" ]] || {
    git status --short >&2
    die 'lock repair requires a completely clean committed starting tree'
}

if ! mkdir "$LOCK_DIR" 2>/dev/null; then
    printf 'Existing CUF v0.11 execution lock: %s\n' "$LOCK_DIR" >&2
    [[ -f "$LOCK_DIR/owner" ]] && cat "$LOCK_DIR/owner" >&2 || true
    die 'another CUF v0.11 execution/repair harness may be active; inspect before removing a stale lock manually'
fi
lock_acquired=1

base_head="$(git rev-parse HEAD^{commit})"
base_tree="$(git rev-parse HEAD^{tree})"
lock_before="$(sha256sum Cargo.lock | awk '{print $1}')"
{
    printf 'pid=%s\n' "$$"
    printf 'mode=lock-repair-%s\n' "$CONTEXT"
    printf 'started_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf 'head=%s\n' "$base_head"
} > "$LOCK_DIR/owner"

if [[ -z "$OUT" ]]; then
    OUT="/tmp/cuf-v0.11-${CONTEXT}-lock-repair-${base_head:0:12}-$(date -u +%Y%m%dT%H%M%SZ)"
fi
OUT="$(realpath -m "$OUT")"
case "$OUT/" in
    "$(realpath "$ROOT")/"*) die 'evidence output must be outside the repository worktree' ;;
esac
[[ ! -e "$OUT" ]] || die "evidence output already exists: $OUT"
mkdir -p "$OUT"

printf '%s\n' "$CONTEXT" > "$OUT/CONTEXT.txt"
printf 'OPERATOR_LABEL_ONLY\n' > "$OUT/CONTEXT_AUTHORITY.txt"
printf '%s\n' "$base_head" > "$OUT/BASE_HEAD.txt"
printf '%s\n' "$base_tree" > "$OUT/BASE_TREE.txt"
printf '%s  Cargo.lock\n' "$lock_before" > "$OUT/CARGO_LOCK_SHA256_BEFORE.txt"
git status --short > "$OUT/GIT_STATUS_BEFORE.txt"

# Bind every tracked Cargo manifest to the exact repair base. This is redundant
# with the later Cargo.lock-only diff check, but makes the dependency input set
# independently inspectable in the evidence capsule.
git ls-files '*Cargo.toml' | LC_ALL=C sort > "$OUT/CARGO_MANIFEST_PATHS.txt"
: > "$OUT/CARGO_MANIFEST_BLOBS.txt"
while IFS= read -r path; do
    [[ -n "$path" ]] || continue
    blob="$(git rev-parse "$base_head:$path")"
    printf '%s\t%s\n' "$blob" "$path" >> "$OUT/CARGO_MANIFEST_BLOBS.txt"
done < "$OUT/CARGO_MANIFEST_PATHS.txt"

printf 'RUNNING\n' > "$OUT/STATUS.txt"
{
    printf 'nix: %s\n' "$(nix --version)"
    printf 'system: %s\n' "$(uname -a)"
    nix develop --command bash -lc '
        set -euo pipefail
        export LC_ALL=C
        printf "rustc: %s\n" "$(rustc --version)"
        printf "cargo: %s\n" "$(cargo --version)"
    '
} > "$OUT/TOOLCHAIN.txt"
if [[ -n "$(git status --porcelain)" ]]; then
    git status --short > "$OUT/GIT_STATUS_READINESS_SIDE_EFFECT.txt"
    printf 'READINESS_SIDE_EFFECT\n' > "$OUT/STATUS.txt"
    die 'Nix/toolchain readiness changed repository state'
fi

printf 'LC_ALL=C cargo metadata --locked --no-deps --format-version 1\n' > "$OUT/PRECHECK_COMMAND.txt"
set +e
nix develop --command bash -lc 'set -euo pipefail; export LC_ALL=C; cargo metadata --locked --no-deps --format-version 1 >/dev/null' \
    > "$OUT/PRECHECK.stdout.log" 2> "$OUT/PRECHECK.stderr.log"
precheck_status=$?
set -e
printf '%s\n' "$precheck_status" > "$OUT/PRECHECK_EXIT.txt"
if [[ -n "$(git status --porcelain)" ]]; then
    git status --short > "$OUT/GIT_STATUS_PRECHECK_SIDE_EFFECT.txt"
    printf 'PRECHECK_SIDE_EFFECT\n' > "$OUT/STATUS.txt"
    die 'locked metadata precheck changed repository state'
fi

if [[ "$precheck_status" -eq 0 ]]; then
    printf 'NOT_NEEDED\n' > "$OUT/FAILURE_CLASS.txt"
    printf 'NOT_NEEDED\n' > "$OUT/STATUS.txt"
    finalize_manifest
    printf 'No Cargo.lock repair is required: locked metadata already passes.\n'
    exit 3
fi

# Fail closed unless Cargo itself says the lockfile must be updated but --locked
# prevents it. LC_ALL=C stabilizes the diagnostic language used for this guard.
# Do not turn unrelated metadata/config/source failures into speculative lock
# mutation.
if grep -qi 'lock file' "$OUT/PRECHECK.stderr.log" \
    && grep -qi 'needs to be updated' "$OUT/PRECHECK.stderr.log" \
    && grep -q -- '--locked' "$OUT/PRECHECK.stderr.log"; then
    printf 'LOCK_UPDATE_REQUIRED\n' > "$OUT/FAILURE_CLASS.txt"
else
    printf 'UNCLASSIFIED_PRECHECK_FAILURE\n' > "$OUT/FAILURE_CLASS.txt"
    printf 'UNCLASSIFIED_PRECHECK_FAILURE\n' > "$OUT/STATUS.txt"
    finalize_manifest
    cat "$OUT/PRECHECK.stderr.log" >&2
    die 'locked metadata failed for a reason not proven to be a Cargo.lock update requirement; no mutation performed'
fi

printf '\n== Cargo-generated lock reconciliation (%s context) ==\n' "$CONTEXT"
printf 'LC_ALL=C cargo metadata --no-deps --format-version 1\n' > "$OUT/GENERATION_COMMAND.txt"
mutation_started=1
set +e
nix develop --command bash -lc 'set -euo pipefail; export LC_ALL=C; cargo metadata --no-deps --format-version 1 >/dev/null' \
    > "$OUT/GENERATION.stdout.log" 2> "$OUT/GENERATION.stderr.log"
generation_status=$?
set -e
printf '%s\n' "$generation_status" > "$OUT/GENERATION_EXIT.txt"
if [[ "$generation_status" -ne 0 ]]; then
    printf 'GENERATION_FAILED\n' > "$OUT/STATUS.txt"
    exit "$generation_status"
fi

# The repair scope is intentionally one file. Any source, manifest, config,
# staged, or untracked side effect invalidates this repair attempt.
changed="$(git diff --name-only)"
if [[ "$changed" != "Cargo.lock" ]]; then
    printf 'Unexpected tracked changes after Cargo lock generation:\n%s\n' "$changed" >&2
    printf 'SCOPE_VIOLATION\n' > "$OUT/STATUS.txt"
    die 'Cargo lock generation changed files outside Cargo.lock'
fi
[[ -z "$(git diff --cached --name-only)" ]] || {
    printf 'STAGED_SIDE_EFFECT\n' > "$OUT/STATUS.txt"
    die 'lock generation unexpectedly staged changes'
}
untracked="$(git ls-files --others --exclude-standard)"
[[ -z "$untracked" ]] || {
    printf 'Unexpected untracked files:\n%s\n' "$untracked" >&2
    printf 'UNTRACKED_SIDE_EFFECT\n' > "$OUT/STATUS.txt"
    die 'lock generation produced untracked repository files'
}

git diff --check -- Cargo.lock
lock_after="$(sha256sum Cargo.lock | awk '{print $1}')"
[[ "$lock_after" != "$lock_before" ]] || {
    printf 'NO_LOCK_DELTA\n' > "$OUT/STATUS.txt"
    die 'Cargo reported a required lock update but generated an identical Cargo.lock'
}
printf '%s  Cargo.lock\n' "$lock_after" > "$OUT/CARGO_LOCK_SHA256_AFTER.txt"
git diff -- Cargo.lock > "$OUT/CARGO_LOCK.diff"
git diff --stat -- Cargo.lock > "$OUT/CARGO_LOCK_DIFF_STAT.txt"
git status --short > "$OUT/GIT_STATUS_AFTER_GENERATION.txt"

printf 'LC_ALL=C cargo metadata --locked --no-deps --format-version 1\n' > "$OUT/POSTCHECK_COMMAND.txt"
set +e
nix develop --command bash -lc 'set -euo pipefail; export LC_ALL=C; cargo metadata --locked --no-deps --format-version 1 >/dev/null' \
    > "$OUT/POSTCHECK.stdout.log" 2> "$OUT/POSTCHECK.stderr.log"
postcheck_status=$?
set -e
printf '%s\n' "$postcheck_status" > "$OUT/POSTCHECK_EXIT.txt"
if [[ "$postcheck_status" -ne 0 ]]; then
    printf 'POSTCHECK_FAILED\n' > "$OUT/STATUS.txt"
    cat "$OUT/POSTCHECK.stderr.log" >&2
    exit "$postcheck_status"
fi

# Re-prove the entire tracked manifest input set is still exactly the committed
# base. Only Cargo.lock may remain modified.
: > "$OUT/CARGO_MANIFEST_BLOBS_AFTER.txt"
while IFS= read -r path; do
    [[ -n "$path" ]] || continue
    blob="$(git hash-object "$path")"
    printf '%s\t%s\n' "$blob" "$path" >> "$OUT/CARGO_MANIFEST_BLOBS_AFTER.txt"
done < "$OUT/CARGO_MANIFEST_PATHS.txt"
if ! cmp -s "$OUT/CARGO_MANIFEST_BLOBS.txt" "$OUT/CARGO_MANIFEST_BLOBS_AFTER.txt"; then
    printf 'MANIFEST_INPUT_CHANGED\n' > "$OUT/STATUS.txt"
    diff -u "$OUT/CARGO_MANIFEST_BLOBS.txt" "$OUT/CARGO_MANIFEST_BLOBS_AFTER.txt" >&2 || true
    die 'Cargo manifest inputs changed during lock repair'
fi

cat > "$OUT/LINEAGE.txt" <<EOF
repair_context=$CONTEXT
repair_context_authority=OPERATOR_LABEL_ONLY
repair_base_identity_authority=COMMIT_AND_TREE
repair_base_head=$base_head
repair_base_tree=$base_tree
failure_class=LOCK_UPDATE_REQUIRED
repair_generator=cargo_metadata_unlocked_LC_ALL_C
repair_scope=Cargo.lock_only
pre_lock_sha256=$lock_before
post_lock_sha256=$lock_after
qualification_status=NOT_CLAIMED
q2_status=NOT_CLAIMED
EOF

printf 'GENERATED_UNCOMMITTED\n' > "$OUT/STATUS.txt"
finalize_manifest

printf '\nPASS: Cargo generated a scoped CUF v0.11 lock repair\n'
printf 'Context label:     %s (operator label only)\n' "$CONTEXT"
printf 'Base HEAD:         %s\n' "$base_head"
printf 'Base tree:         %s\n' "$base_tree"
printf 'Cargo.lock before: %s\n' "$lock_before"
printf 'Cargo.lock after:  %s\n' "$lock_after"
printf 'Evidence:          %s\n' "$OUT"
printf 'Status:            GENERATED_UNCOMMITTED\n'
printf '\nReview before committing:\n'
printf '  git diff -- Cargo.lock\n'
printf '  git status --short\n'
printf '\nIf accepted, stage ONLY Cargo.lock and create one repair commit, then verify it with:\n'
printf '  bash scripts/verify-cuf-v0.11-cargo-lock-repair.sh %q HEAD\n' "$OUT"
printf '\nThe exact base commit/tree is authoritative; the context label is not lineage proof.\n'
printf 'Q1 and Q2 remain NOT_CLAIMED.\n'
