#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Verify that a promotion commit exactly commits the CUF v0.11 pre-Q2 parent
# tree proven by a PASS evidence capsule. Promotion creates the committed parent
# for Q2 repairs; it does not itself claim Q2 continuation/replay qualification.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

EVIDENCE="${1:-}"
COMMIT="${2:-HEAD}"
EXPECTED_PATCH_SHA256="23f6baf3545bace49252eee190f181fa8a88c650d2994b72b65bdaf83cc74637"
EXPECTED_STAGED_PATHS=275
FROZEN_PRE_Q2_BASE="a544594648c88a0f5e3ea4daf982ede205663af3"

if [[ -z "$EVIDENCE" || ! -d "$EVIDENCE" ]]; then
    printf 'usage: %s /path/to/cuf-v0.11-parent-evidence [commit]\n' "$0" >&2
    exit 2
fi
EVIDENCE="$(realpath "$EVIDENCE")"

required=(
    BASE_HEAD.txt
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
    LINEAGE.txt
    STATUS.txt
    MANIFEST.sha256
)
for name in "${required[@]}"; do
    if [[ ! -f "$EVIDENCE/$name" ]]; then
        printf 'ERROR: parent evidence capsule missing %s\n' "$name" >&2
        exit 1
    fi
done

(
    cd "$EVIDENCE"
    sha256sum -c MANIFEST.sha256 >/dev/null
)

if [[ "$(cat "$EVIDENCE/STATUS.txt")" != "PASS" ]]; then
    printf 'ERROR: parent evidence capsule is not PASS.\n' >&2
    exit 1
fi
if [[ "$(cat "$EVIDENCE/POSTCONDITIONS.txt")" != "PASS" ]]; then
    printf 'ERROR: parent evidence postconditions are not PASS.\n' >&2
    exit 1
fi
if [[ "$(cat "$EVIDENCE/UNIVERSAL_MATTER_V4_8_PATCH_SHA256.txt")" != "$EXPECTED_PATCH_SHA256" ]]; then
    printf 'ERROR: parent evidence binds the wrong retained v4.8 patch.\n' >&2
    exit 1
fi
if [[ "$(cat "$EVIDENCE/STAGED_PATH_COUNT.txt")" != "$EXPECTED_STAGED_PATHS" ]]; then
    printf 'ERROR: qualified staged path count claim is not %s.\n' "$EXPECTED_STAGED_PATHS" >&2
    exit 1
fi
for path_manifest in PATCH_PATHS.txt STAGED_PATHS.txt STAGED_PATHS_AFTER.txt; do
    actual_count="$(wc -l < "$EVIDENCE/$path_manifest" | tr -d ' ')"
    if [[ "$actual_count" -ne "$EXPECTED_STAGED_PATHS" ]]; then
        printf 'ERROR: %s contains %s paths; expected %s.\n' \
            "$path_manifest" "$actual_count" "$EXPECTED_STAGED_PATHS" >&2
        exit 1
    fi
done

for pair in \
    'PATCH_PATHS.txt STAGED_PATHS.txt' \
    'STAGED_PATHS.txt STAGED_PATHS_AFTER.txt' \
    'CARGO_LOCK_SHA256.txt CARGO_LOCK_SHA256_AFTER.txt' \
    'GIT_STATUS_BEFORE.txt GIT_STATUS_AFTER.txt' \
    'PARENT_COMPOSITION.txt PARENT_COMPOSITION_AFTER.txt' \
    'TOOLING_BLOBS_BEFORE.txt TOOLING_BLOBS_AFTER.txt'; do
    left="${pair%% *}"
    right="${pair#* }"
    if ! cmp -s "$EVIDENCE/$left" "$EVIDENCE/$right"; then
        printf 'ERROR: evidence mismatch between %s and %s.\n' "$left" "$right" >&2
        exit 1
    fi
done

qualified_parent="$(cat "$EVIDENCE/BASE_HEAD.txt")"
qualified_tree="$(cat "$EVIDENCE/STAGED_TREE.txt")"
qualified_tree_after="$(cat "$EVIDENCE/STAGED_TREE_AFTER.txt")"

if [[ "$qualified_tree" != "$qualified_tree_after" ]]; then
    printf 'ERROR: evidence recorded different staged trees before/after qualification.\n' >&2
    exit 1
fi
if ! git merge-base --is-ancestor "$FROZEN_PRE_Q2_BASE" "$qualified_parent"; then
    printf 'ERROR: evidence candidate head is not a descendant of frozen pre-Q2 base %s.\n' \
        "$FROZEN_PRE_Q2_BASE" >&2
    exit 1
fi

for expected in \
    "candidate_head=$qualified_parent" \
    "qualified_staged_tree=$qualified_tree" \
    "universal_matter_patch_sha256=$EXPECTED_PATCH_SHA256" \
    'parent_composition_proof=PARENT_COMPOSITION.txt' \
    'qualification_tooling=TOOLING_BLOBS_BEFORE.txt' \
    'v4.8_patch_path_set=PATCH_PATHS.txt' \
    'qualified_staged_path_set=STAGED_PATHS.txt' \
    'dependency_resolution=repository_Cargo.lock_locked' \
    'tier_a_portability=SUPPLEMENTARY_NOT_PROMOTION' \
    'qualification_level=Q0/Q1_plus_continuation_core_only' \
    'q2_status=NOT_CLAIMED'; do
    if ! grep -Fxq "$expected" "$EVIDENCE/LINEAGE.txt"; then
        printf 'ERROR: required lineage binding missing: %s\n' "$expected" >&2
        exit 1
    fi
done

required_tool_paths=(
    scripts/preflight-universal-matter-v4.8.sh
    scripts/apply-universal-matter-v4.8.sh
    scripts/run-cuf-v0.11-parent-candidate-local.sh
    scripts/resume-cuf-v0.11-parent-candidate-local.sh
    scripts/verify-cuf-v0.11-parent-candidate.sh
    scripts/verify-cuf-v0.11-qualification-tooling.sh
    scripts/qualify-cuf-v0.11-parent-candidate.sh
    scripts/capture-cuf-v0.11-parent-candidate-evidence.sh
    scripts/qualify-universal-matter-v4.8-cuf-v0.10.1.sh
    scripts/qualify-cuf-v0.10.1-forcing.sh
    scripts/qualify-cuf-v0.10-stack.sh
    scripts/check-workspace.sh
    scripts/check-licenses.sh
    scripts/verify-cuf-v0.11-parent-promotion.sh
)

tool_lines="$(wc -l < "$EVIDENCE/TOOLING_BLOBS_BEFORE.txt" | tr -d ' ')"
tool_unique="$(cut -f2 "$EVIDENCE/TOOLING_BLOBS_BEFORE.txt" | LC_ALL=C sort -u | wc -l | tr -d ' ')"
if [[ "$tool_lines" -ne "$tool_unique" ]]; then
    printf 'ERROR: tooling manifest contains duplicate paths.\n' >&2
    exit 1
fi

for required_path in "${required_tool_paths[@]}"; do
    matches="$(awk -F '\t' -v path="$required_path" '$2 == path {count++} END {print count+0}' \
        "$EVIDENCE/TOOLING_BLOBS_BEFORE.txt")"
    if [[ "$matches" -ne 1 ]]; then
        printf 'ERROR: tooling manifest must contain exactly one entry for %s; found %s.\n' \
            "$required_path" "$matches" >&2
        exit 1
    fi
done

while IFS=$'\t' read -r recorded_blob path extra; do
    if [[ -z "$recorded_blob" || -z "$path" || -n "${extra:-}" ]]; then
        printf 'ERROR: malformed tooling manifest entry: %s %s %s\n' \
            "$recorded_blob" "$path" "${extra:-}" >&2
        exit 1
    fi
    if ! git cat-file -e "$qualified_parent:$path" 2>/dev/null; then
        printf 'ERROR: recorded qualification tool is absent from evidence parent: %s\n' \
            "$path" >&2
        exit 1
    fi
    committed_blob="$(git rev-parse "$qualified_parent:$path")"
    if [[ "$committed_blob" != "$recorded_blob" ]]; then
        printf 'ERROR: recorded tooling blob does not match evidence parent for %s.\nexpected: %s\nrecorded: %s\n' \
            "$path" "$committed_blob" "$recorded_blob" >&2
        exit 1
    fi
done < "$EVIDENCE/TOOLING_BLOBS_BEFORE.txt"

commit_sha="$(git rev-parse "$COMMIT^{commit}")"
commit_tree="$(git rev-parse "$commit_sha^{tree}")"
parent_line="$(git rev-list --parents -n 1 "$commit_sha")"
parent_count="$(awk '{print NF-1}' <<<"$parent_line")"
if [[ "$parent_count" != "1" ]]; then
    printf 'ERROR: promotion commit must have exactly one parent; got %s.\n' "$parent_count" >&2
    exit 1
fi
first_parent="$(git rev-parse "$commit_sha^1")"
if [[ "$first_parent" != "$qualified_parent" ]]; then
    printf 'ERROR: promotion parent mismatch.\nexpected: %s\nactual:   %s\n' \
        "$qualified_parent" "$first_parent" >&2
    exit 1
fi
if [[ "$commit_tree" != "$qualified_tree" ]]; then
    printf 'ERROR: promotion tree differs from qualified staged tree.\nexpected: %s\nactual:   %s\n' \
        "$qualified_tree" "$commit_tree" >&2
    exit 1
fi

capsule_lock="$(awk '{print $1}' "$EVIDENCE/CARGO_LOCK_SHA256_AFTER.txt")"
commit_lock="$(git show "$commit_sha:Cargo.lock" | sha256sum | awk '{print $1}')"
if [[ "$commit_lock" != "$capsule_lock" ]]; then
    printf 'ERROR: promoted Cargo.lock differs from qualified lockfile.\n' >&2
    exit 1
fi

printf 'PASS: promoted CUF v0.11 pre-Q2 parent exactly matches qualified evidence\n'
printf 'Promotion commit: %s\n' "$commit_sha"
printf 'Evidence parent:  %s\n' "$qualified_parent"
printf 'Qualified tree:   %s\n' "$qualified_tree"
printf 'Tooling blobs:    independently matched to evidence parent\n'
printf 'Q2 status:        NOT_CLAIMED\n'
