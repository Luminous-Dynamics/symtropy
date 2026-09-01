#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Capture checksum-manifested evidence for the exact staged Universal Matter
# v4.8 replay plus the finalized CUF continuation/forcing parent candidate.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

PATCH="${1:-}"
OUT="${2:-}"
EXPECTED_PATCH_SHA256="23f6baf3545bace49252eee190f181fa8a88c650d2994b72b65bdaf83cc74637"
EXPECTED_STAGED_PATHS=275

if [[ -z "$PATCH" || ! -f "$PATCH" ]]; then
    printf 'usage: %s /path/to/SYMTROPY_UNIVERSAL_MATTER_V4.8_CUMULATIVE.patch [output-dir]\n' "$0" >&2
    exit 2
fi

actual_patch_sha="$(sha256sum "$PATCH" | awk '{print $1}')"
if [[ "$actual_patch_sha" != "$EXPECTED_PATCH_SHA256" ]]; then
    printf 'ERROR: retained v4.8 patch SHA-256 mismatch.\n' >&2
    exit 1
fi

staged_count="$(git diff --cached --name-only | wc -l | tr -d ' ')"
if [[ "$staged_count" -ne "$EXPECTED_STAGED_PATHS" ]]; then
    printf 'ERROR: expected exactly %s staged v4.8 paths; found %s.\n' \
        "$EXPECTED_STAGED_PATHS" "$staged_count" >&2
    exit 1
fi

base_head="$(git rev-parse HEAD)"
staged_tree="$(git write-tree)"
if [[ -z "$OUT" ]]; then
    OUT="/tmp/symtropy-cuf-v0.11-parent-${staged_tree:0:12}"
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

if ! bash scripts/verify-cuf-v0.11-parent-candidate.sh > "$OUT/PARENT_COMPOSITION.txt" 2>&1; then
    cat "$OUT/PARENT_COMPOSITION.txt" >&2
    rm -rf "$OUT"
    exit 1
fi

git apply --numstat "$PATCH" | cut -f3- | LC_ALL=C sort -u > "$OUT/PATCH_PATHS.txt"
git diff --cached --name-only | LC_ALL=C sort -u > "$OUT/STAGED_PATHS.txt"

patch_count="$(wc -l < "$OUT/PATCH_PATHS.txt" | tr -d ' ')"
staged_unique_count="$(wc -l < "$OUT/STAGED_PATHS.txt" | tr -d ' ')"
if [[ "$patch_count" -ne "$EXPECTED_STAGED_PATHS" || "$staged_unique_count" -ne "$EXPECTED_STAGED_PATHS" ]]; then
    printf 'ERROR: patch/staged unique path count mismatch. patch=%s staged=%s expected=%s\n' \
        "$patch_count" "$staged_unique_count" "$EXPECTED_STAGED_PATHS" >&2
    rm -rf "$OUT"
    exit 1
fi
if ! cmp -s "$OUT/PATCH_PATHS.txt" "$OUT/STAGED_PATHS.txt"; then
    printf 'ERROR: staged path set differs from retained v4.8 artifact.\n' >&2
    diff -u "$OUT/PATCH_PATHS.txt" "$OUT/STAGED_PATHS.txt" >&2 || true
    rm -rf "$OUT"
    exit 1
fi

printf '%s\n' "$base_head" > "$OUT/BASE_HEAD.txt"
printf '%s\n' "$staged_tree" > "$OUT/STAGED_TREE.txt"
printf '%s\n' "$actual_patch_sha" > "$OUT/UNIVERSAL_MATTER_V4_8_PATCH_SHA256.txt"
printf '%s\n' "$EXPECTED_STAGED_PATHS" > "$OUT/STAGED_PATH_COUNT.txt"
printf '%s  Cargo.lock\n' "$(sha256sum Cargo.lock | awk '{print $1}')" > "$OUT/CARGO_LOCK_SHA256.txt"
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

cat > "$OUT/LINEAGE.txt" <<EOF
candidate_head=$base_head
qualified_staged_tree=$staged_tree
frozen_v4.8_cuf_ancestor=2222e6c2d23b95372aa9a93763018b61e3d5351f
frozen_continuation_semantic_head=c90b5ad1966263ff35b2eee73f8bf6655377344a
universal_matter_patch_sha256=$actual_patch_sha
parent_composition_proof=PARENT_COMPOSITION.txt
v4.8_patch_path_set=PATCH_PATHS.txt
qualified_staged_path_set=STAGED_PATHS.txt
qualification_level=Q0/Q1_plus_continuation_core_only
q2_status=NOT_CLAIMED
EOF

printf 'RUNNING\n' > "$OUT/STATUS.txt"
set +e
nix develop --command bash scripts/qualify-cuf-v0.11-parent-candidate.sh \
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
        | LC_ALL=C sort | xargs -r sha256sum > MANIFEST.sha256
)

printf '\nCUF v0.11 parent-candidate evidence capsule: %s\n' "$OUT"
printf 'Status: '
cat "$OUT/STATUS.txt"
printf 'Candidate HEAD: %s\n' "$base_head"
printf 'Staged tree:    %s\n' "$staged_tree"
printf 'Q2 is not claimed by this capsule.\n'

exit "$status"
