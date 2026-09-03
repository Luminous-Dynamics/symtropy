#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Verify that a commit made after a pristine v4.8 Q1 failure preserves exactly
# the staged authored Universal Matter tree proven by the FAIL evidence capsule.
# This is a historical-preservation verifier, not a promotion/Q1 verifier.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

EVIDENCE="${1:-}"
COMMIT="${2:-HEAD}"
EXPECTED_PATCH_SHA256="23f6baf3545bace49252eee190f181fa8a88c650d2994b72b65bdaf83cc74637"
EXPECTED_STAGED_PATHS=275

if [[ -z "$EVIDENCE" || ! -d "$EVIDENCE" ]]; then
    printf 'usage: %s /path/to/failed-v4.8-evidence [commit]\n' "$0" >&2
    exit 2
fi
EVIDENCE="$(realpath "$EVIDENCE")"

required=(
    BASE_HEAD.txt
    BASE_HEAD_AFTER.txt
    STAGED_TREE.txt
    STAGED_TREE_AFTER.txt
    UNIVERSAL_MATTER_V4_8_PATCH_SHA256.txt
    STAGED_PATH_COUNT.txt
    PATCH_PATHS.txt
    STAGED_PATHS.txt
    STAGED_PATHS_AFTER.txt
    CARGO_LOCK_SHA256.txt
    CARGO_LOCK_SHA256_AFTER.txt
    GIT_STATUS_BEFORE.txt
    GIT_STATUS_AFTER.txt
    PARENT_COMPOSITION.txt
    PARENT_COMPOSITION_AFTER.txt
    TOOLING_BLOBS_BEFORE.txt
    TOOLING_BLOBS_AFTER.txt
    POSTCONDITIONS.txt
    STATUS.txt
    LINEAGE.txt
    MANIFEST.sha256
)
for name in "${required[@]}"; do
    [[ -f "$EVIDENCE/$name" ]] || {
        printf 'ERROR: failed-authored evidence capsule missing %s\n' "$name" >&2
        exit 1
    }
done

(
    cd "$EVIDENCE"
    sha256sum -c MANIFEST.sha256 >/dev/null
)

status="$(cat "$EVIDENCE/STATUS.txt")"
case "$status" in
    FAIL*) ;;
    *)
        printf 'ERROR: authored-failure preservation requires a FAIL evidence capsule; got: %s\n' "$status" >&2
        exit 1
        ;;
esac

# A semantic/build Q1 failure is preservable only when the evidence wrapper's
# independent integrity postconditions all passed. If postconditions failed,
# the staged tree/tooling/status changed and is not a trustworthy authored root.
[[ "$(cat "$EVIDENCE/POSTCONDITIONS.txt")" == "PASS" ]] || {
    printf 'ERROR: failed evidence has failed postconditions; do not preserve/promote that staged tree.\n' >&2
    exit 1
}

[[ "$(cat "$EVIDENCE/UNIVERSAL_MATTER_V4_8_PATCH_SHA256.txt")" == "$EXPECTED_PATCH_SHA256" ]] || {
    printf 'ERROR: evidence binds the wrong retained v4.8 artifact.\n' >&2
    exit 1
}
[[ "$(cat "$EVIDENCE/STAGED_PATH_COUNT.txt")" == "$EXPECTED_STAGED_PATHS" ]] || {
    printf 'ERROR: evidence staged path count claim is not %s.\n' "$EXPECTED_STAGED_PATHS" >&2
    exit 1
}

for manifest in PATCH_PATHS.txt STAGED_PATHS.txt STAGED_PATHS_AFTER.txt; do
    count="$(wc -l < "$EVIDENCE/$manifest" | tr -d ' ')"
    [[ "$count" -eq "$EXPECTED_STAGED_PATHS" ]] || {
        printf 'ERROR: %s contains %s paths; expected %s.\n' "$manifest" "$count" "$EXPECTED_STAGED_PATHS" >&2
        exit 1
    }
done
cmp -s "$EVIDENCE/PATCH_PATHS.txt" "$EVIDENCE/STAGED_PATHS.txt" || {
    printf 'ERROR: staged paths did not equal retained patch paths before qualification.\n' >&2
    exit 1
}
cmp -s "$EVIDENCE/STAGED_PATHS.txt" "$EVIDENCE/STAGED_PATHS_AFTER.txt" || {
    printf 'ERROR: staged path set changed during failed qualification.\n' >&2
    exit 1
}

for pair in \
    'BASE_HEAD.txt BASE_HEAD_AFTER.txt' \
    'STAGED_TREE.txt STAGED_TREE_AFTER.txt' \
    'CARGO_LOCK_SHA256.txt CARGO_LOCK_SHA256_AFTER.txt' \
    'GIT_STATUS_BEFORE.txt GIT_STATUS_AFTER.txt' \
    'PARENT_COMPOSITION.txt PARENT_COMPOSITION_AFTER.txt' \
    'TOOLING_BLOBS_BEFORE.txt TOOLING_BLOBS_AFTER.txt'; do
    left="${pair%% *}"
    right="${pair#* }"
    cmp -s "$EVIDENCE/$left" "$EVIDENCE/$right" || {
        printf 'ERROR: failed evidence integrity mismatch between %s and %s.\n' "$left" "$right" >&2
        exit 1
    }
done

base_head="$(cat "$EVIDENCE/BASE_HEAD.txt")"
qualified_tree="$(cat "$EVIDENCE/STAGED_TREE.txt")"

for expected in \
    "candidate_head=$base_head" \
    "qualified_staged_tree=$qualified_tree" \
    "universal_matter_patch_sha256=$EXPECTED_PATCH_SHA256" \
    'dependency_resolution=repository_Cargo.lock_locked' \
    'tier_a_portability=SUPPLEMENTARY_NOT_PROMOTION' \
    'qualification_level=Q0/Q1_plus_continuation_core_only' \
    'q2_status=NOT_CLAIMED'; do
    grep -Fxq "$expected" "$EVIDENCE/LINEAGE.txt" || {
        printf 'ERROR: required failed-evidence lineage binding missing: %s\n' "$expected" >&2
        exit 1
    }
done

commit_sha="$(git rev-parse "$COMMIT^{commit}")"
parent_line="$(git rev-list --parents -n 1 "$commit_sha")"
parent_count="$(awk '{print NF-1}' <<<"$parent_line")"
[[ "$parent_count" == "1" ]] || {
    printf 'ERROR: authored-failure preservation commit must have exactly one parent; got %s.\n' "$parent_count" >&2
    exit 1
}
actual_parent="$(git rev-parse "$commit_sha^1")"
[[ "$actual_parent" == "$base_head" ]] || {
    printf 'ERROR: preservation commit parent mismatch.\nexpected: %s\nactual:   %s\n' "$base_head" "$actual_parent" >&2
    exit 1
}

commit_tree="$(git rev-parse "$commit_sha^{tree}")"
[[ "$commit_tree" == "$qualified_tree" ]] || {
    printf 'ERROR: preservation commit tree differs from failed-but-intact authored staged tree.\nexpected: %s\nactual:   %s\n' \
        "$qualified_tree" "$commit_tree" >&2
    exit 1
}

# Prove the commit's changed path set is exactly the retained v4.8 path set.
tmp_paths="$(mktemp /tmp/cuf-v011-authored-failure-paths.XXXXXX)"
trap 'rm -f "$tmp_paths"' EXIT
git diff --name-only "$base_head" "$commit_sha" -- | LC_ALL=C sort -u > "$tmp_paths"
cmp -s "$EVIDENCE/PATCH_PATHS.txt" "$tmp_paths" || {
    printf 'ERROR: preservation commit changed paths differ from retained v4.8 artifact.\n' >&2
    diff -u "$EVIDENCE/PATCH_PATHS.txt" "$tmp_paths" >&2 || true
    exit 1
}

# v4.8 authored replay does not include Cargo.lock. The preservation commit must
# retain the exact lock identity that was present during the failed Q1 attempt.
capsule_lock="$(awk '{print $1}' "$EVIDENCE/CARGO_LOCK_SHA256_AFTER.txt")"
commit_lock="$(git show "$commit_sha:Cargo.lock" | sha256sum | awk '{print $1}')"
[[ "$commit_lock" == "$capsule_lock" ]] || {
    printf 'ERROR: preservation commit changed Cargo.lock relative to failed authored evidence.\n' >&2
    exit 1
}

git diff --check "$base_head" "$commit_sha"

printf 'PASS: failed Universal Matter v4.8 authored tree is preserved exactly as an unqualified commit\n'
printf 'Preservation commit: %s\n' "$commit_sha"
printf 'Evidence parent:     %s\n' "$base_head"
printf 'Authored tree:       %s\n' "$qualified_tree"
printf 'Retained paths:      %s\n' "$EXPECTED_STAGED_PATHS"
printf 'Q1 status:           FAIL preserved / NOT_QUALIFIED\n'
printf 'Q2 status:           NOT_CLAIMED\n'
printf 'This commit is a historical repair parent, not a qualified promotion.\n'
