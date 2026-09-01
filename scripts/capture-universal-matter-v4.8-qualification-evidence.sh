#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Run the combined Universal Matter v4.8 + CUF gate and capture an immutable-ish
# filesystem evidence capsule outside the repository worktree.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

PATCH="${1:-}"
OUT="${2:-}"

if [[ -z "$PATCH" || ! -f "$PATCH" ]]; then
    printf 'usage: %s /path/to/SYMTROPY_UNIVERSAL_MATTER_V4.8_CUMULATIVE.patch [output-dir]\n' "$0" >&2
    exit 2
fi

EXPECTED_PATCH_SHA256="23f6baf3545bace49252eee190f181fa8a88c650d2994b72b65bdaf83cc74637"
actual_patch_sha="$(sha256sum "$PATCH" | awk '{print $1}')"
if [[ "$actual_patch_sha" != "$EXPECTED_PATCH_SHA256" ]]; then
    printf 'ERROR: retained patch SHA-256 mismatch.\n' >&2
    exit 1
fi

staged_tree="$(git write-tree)"
base_head="$(git rev-parse HEAD)"

if [[ -z "$OUT" ]]; then
    OUT="/tmp/symtropy-universal-matter-v4.8-cuf-${staged_tree:0:12}"
fi

case "$(realpath -m "$OUT")/" in
    "$(realpath "$ROOT")/"*)
        printf 'ERROR: evidence output must be outside the repository worktree.\n' >&2
        exit 1
        ;;
esac

if [[ -e "$OUT" ]]; then
    printf 'ERROR: evidence output already exists: %s\n' "$OUT" >&2
    exit 1
fi
mkdir -p "$OUT"

printf '%s\n' "$base_head" > "$OUT/BASE_HEAD.txt"
printf '%s\n' "$staged_tree" > "$OUT/STAGED_TREE.txt"
printf '%s\n' "$actual_patch_sha" > "$OUT/UNIVERSAL_MATTER_V4_8_PATCH_SHA256.txt"
printf '%s  Cargo.lock\n' "$(sha256sum Cargo.lock | awk '{print $1}')" > "$OUT/CARGO_LOCK_SHA256.txt"
printf '%s\n' "$(git diff --cached --name-only | wc -l | tr -d ' ')" > "$OUT/STAGED_PATH_COUNT.txt"

git status --short > "$OUT/GIT_STATUS_BEFORE.txt"
git diff --cached --stat > "$OUT/STAGED_DIFF_STAT.txt"

{
    printf 'rustc: %s\n' "$(rustc --version)"
    printf 'cargo: %s\n' "$(cargo --version)"
    if command -v nix >/dev/null 2>&1; then
        printf 'nix: %s\n' "$(nix --version)"
    else
        printf 'nix: unavailable\n'
    fi
    printf 'system: %s\n' "$(uname -a)"
} > "$OUT/TOOLCHAIN.txt"

printf 'RUNNING\n' > "$OUT/STATUS.txt"

set +e
nix develop --command bash scripts/qualify-universal-matter-v4.8-cuf.sh \
    > >(tee "$OUT/QUALIFICATION.log") \
    2> >(tee "$OUT/QUALIFICATION.err.log" >&2)
status=$?
set -e

if [[ "$status" -eq 0 ]]; then
    printf 'PASS\n' > "$OUT/STATUS.txt"
else
    printf 'FAIL exit=%s\n' "$status" > "$OUT/STATUS.txt"
fi

git status --short > "$OUT/GIT_STATUS_AFTER.txt"
printf '%s\n' "$(git write-tree)" > "$OUT/STAGED_TREE_AFTER.txt"
printf '%s  Cargo.lock\n' "$(sha256sum Cargo.lock | awk '{print $1}')" > "$OUT/CARGO_LOCK_SHA256_AFTER.txt"

(
    cd "$OUT"
    find . -maxdepth 1 -type f ! -name MANIFEST.sha256 -printf '%P\n' \
        | LC_ALL=C sort \
        | xargs -r sha256sum > MANIFEST.sha256
)

printf '\nQualification evidence capsule: %s\n' "$OUT"
printf 'Status: '
cat "$OUT/STATUS.txt"
printf 'Staged tree: %s\n' "$staged_tree"

exit "$status"
