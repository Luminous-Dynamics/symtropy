#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Verify a committed CUF v0.11 Cargo.lock repair against the evidence emitted by
# prepare-cuf-v0.11-cargo-lock-repair.sh. This proves repair scope/identity only;
# it does not claim Q1 or Q2 qualification.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

EVIDENCE="${1:-}"
COMMIT="${2:-HEAD}"

if [[ -z "$EVIDENCE" || ! -d "$EVIDENCE" ]]; then
    printf 'usage: %s /path/to/lock-repair-evidence [commit]\n' "$0" >&2
    exit 2
fi
EVIDENCE="$(realpath "$EVIDENCE")"

required=(
    CONTEXT.txt
    CONTEXT_AUTHORITY.txt
    BASE_HEAD.txt
    BASE_TREE.txt
    CARGO_LOCK_SHA256_BEFORE.txt
    CARGO_LOCK_SHA256_AFTER.txt
    CARGO_LOCK.diff
    CARGO_LOCK_DIFF_STAT.txt
    CARGO_MANIFEST_PATHS.txt
    CARGO_MANIFEST_BLOBS.txt
    CARGO_MANIFEST_BLOBS_AFTER.txt
    PRECHECK_COMMAND.txt
    PRECHECK_EXIT.txt
    PRECHECK.stderr.log
    FAILURE_CLASS.txt
    GENERATION_COMMAND.txt
    GENERATION_EXIT.txt
    POSTCHECK_COMMAND.txt
    POSTCHECK_EXIT.txt
    TOOLCHAIN.txt
    LINEAGE.txt
    STATUS.txt
    MANIFEST.sha256
)
for name in "${required[@]}"; do
    [[ -f "$EVIDENCE/$name" ]] || {
        printf 'ERROR: lock-repair evidence missing %s\n' "$name" >&2
        exit 1
    }
done

(
    cd "$EVIDENCE"
    sha256sum -c MANIFEST.sha256 >/dev/null
)

context="$(cat "$EVIDENCE/CONTEXT.txt")"
case "$context" in
    parent|v4.8) ;;
    *) printf 'ERROR: invalid repair context in evidence: %s\n' "$context" >&2; exit 1 ;;
esac
[[ "$(cat "$EVIDENCE/CONTEXT_AUTHORITY.txt")" == "OPERATOR_LABEL_ONLY" ]] || {
    printf 'ERROR: repair evidence overstates context authority.\n' >&2
    exit 1
}

[[ "$(cat "$EVIDENCE/STATUS.txt")" == "GENERATED_UNCOMMITTED" ]] || {
    printf 'ERROR: evidence status is not GENERATED_UNCOMMITTED.\n' >&2
    exit 1
}
[[ "$(cat "$EVIDENCE/FAILURE_CLASS.txt")" == "LOCK_UPDATE_REQUIRED" ]] || {
    printf 'ERROR: evidence did not classify the precheck as LOCK_UPDATE_REQUIRED.\n' >&2
    exit 1
}
precheck_exit="$(cat "$EVIDENCE/PRECHECK_EXIT.txt")"
[[ "$precheck_exit" =~ ^[0-9]+$ && "$precheck_exit" -ne 0 ]] || {
    printf 'ERROR: repair evidence must bind a failing locked precheck.\n' >&2
    exit 1
}
[[ "$(cat "$EVIDENCE/GENERATION_EXIT.txt")" == "0" ]] || {
    printf 'ERROR: Cargo lock generation did not exit successfully.\n' >&2
    exit 1
}
[[ "$(cat "$EVIDENCE/POSTCHECK_EXIT.txt")" == "0" ]] || {
    printf 'ERROR: repaired lock did not pass locked metadata postcheck.\n' >&2
    exit 1
}
cmp -s "$EVIDENCE/CARGO_MANIFEST_BLOBS.txt" "$EVIDENCE/CARGO_MANIFEST_BLOBS_AFTER.txt" || {
    printf 'ERROR: Cargo manifest inputs changed during repair generation.\n' >&2
    exit 1
}

base_head="$(cat "$EVIDENCE/BASE_HEAD.txt")"
base_tree="$(cat "$EVIDENCE/BASE_TREE.txt")"
actual_base_tree="$(git rev-parse "$base_head^{tree}")"
[[ "$actual_base_tree" == "$base_tree" ]] || {
    printf 'ERROR: recorded repair base tree does not match recorded base commit.\n' >&2
    exit 1
}

pre_lock="$(awk '{print $1}' "$EVIDENCE/CARGO_LOCK_SHA256_BEFORE.txt")"
post_lock="$(awk '{print $1}' "$EVIDENCE/CARGO_LOCK_SHA256_AFTER.txt")"
[[ "$pre_lock" =~ ^[0-9a-f]{64}$ && "$post_lock" =~ ^[0-9a-f]{64}$ ]] || {
    printf 'ERROR: malformed Cargo.lock SHA-256 evidence.\n' >&2
    exit 1
}
[[ "$pre_lock" != "$post_lock" ]] || {
    printf 'ERROR: lock repair evidence records identical before/after hashes.\n' >&2
    exit 1
}

base_lock="$(git show "$base_head:Cargo.lock" | sha256sum | awk '{print $1}')"
[[ "$base_lock" == "$pre_lock" ]] || {
    printf 'ERROR: repair evidence pre-lock hash does not match repair base commit.\n' >&2
    exit 1
}

for expected in \
    "repair_context=$context" \
    'repair_context_authority=OPERATOR_LABEL_ONLY' \
    'repair_base_identity_authority=COMMIT_AND_TREE' \
    "repair_base_head=$base_head" \
    "repair_base_tree=$base_tree" \
    'failure_class=LOCK_UPDATE_REQUIRED' \
    'repair_generator=cargo_metadata_unlocked_LC_ALL_C' \
    'repair_scope=Cargo.lock_only' \
    "pre_lock_sha256=$pre_lock" \
    "post_lock_sha256=$post_lock" \
    'qualification_status=NOT_CLAIMED' \
    'q2_status=NOT_CLAIMED'; do
    grep -Fxq "$expected" "$EVIDENCE/LINEAGE.txt" || {
        printf 'ERROR: missing repair lineage binding: %s\n' "$expected" >&2
        exit 1
    }
done

commit_sha="$(git rev-parse "$COMMIT^{commit}")"
parent_line="$(git rev-list --parents -n 1 "$commit_sha")"
parent_count="$(awk '{print NF-1}' <<<"$parent_line")"
[[ "$parent_count" == "1" ]] || {
    printf 'ERROR: lock repair commit must have exactly one parent; got %s.\n' "$parent_count" >&2
    exit 1
}
repair_parent="$(git rev-parse "$commit_sha^1")"
[[ "$repair_parent" == "$base_head" ]] || {
    printf 'ERROR: lock repair commit parent mismatch.\nexpected: %s\nactual:   %s\n' \
        "$base_head" "$repair_parent" >&2
    exit 1
}

mapfile -t changed_paths < <(git diff-tree --no-commit-id --name-only -r "$commit_sha")
if [[ "${#changed_paths[@]}" -ne 1 || "${changed_paths[0]}" != "Cargo.lock" ]]; then
    printf 'ERROR: lock repair commit must change exactly Cargo.lock. Changed paths:\n' >&2
    if [[ "${#changed_paths[@]}" -eq 0 ]]; then
        printf '  <none>\n' >&2
    else
        printf '  %s\n' "${changed_paths[@]}" >&2
    fi
    exit 1
fi

commit_lock="$(git show "$commit_sha:Cargo.lock" | sha256sum | awk '{print $1}')"
[[ "$commit_lock" == "$post_lock" ]] || {
    printf 'ERROR: committed Cargo.lock does not match generated repair evidence.\nexpected: %s\nactual:   %s\n' \
        "$post_lock" "$commit_lock" >&2
    exit 1
}

# The exact textual repair delta is also evidence-bound. Reconstruct it from the
# committed parent/child pair and compare byte-for-byte with the generated diff.
tmp_diff="$(mktemp /tmp/cuf-v011-lock-repair-diff.XXXXXX)"
trap 'rm -f "$tmp_diff"' EXIT
git diff "$base_head" "$commit_sha" -- Cargo.lock > "$tmp_diff"
cmp -s "$EVIDENCE/CARGO_LOCK.diff" "$tmp_diff" || {
    printf 'ERROR: committed Cargo.lock diff differs from Cargo-generated evidence.\n' >&2
    diff -u "$EVIDENCE/CARGO_LOCK.diff" "$tmp_diff" >&2 || true
    exit 1
}

git diff --check "$base_head" "$commit_sha" -- Cargo.lock

printf 'PASS: CUF v0.11 Cargo.lock repair commit exactly matches generated evidence\n'
printf 'Context label:     %s (operator label only)\n' "$context"
printf 'Repair commit:     %s\n' "$commit_sha"
printf 'Repair base:       %s\n' "$base_head"
printf 'Repair base tree:  %s\n' "$base_tree"
printf 'Cargo.lock before: %s\n' "$pre_lock"
printf 'Cargo.lock after:  %s\n' "$post_lock"
printf 'Repair scope:      Cargo.lock only\n'
printf 'Qualification:     NOT_CLAIMED\n'
printf 'Q2 status:         NOT_CLAIMED\n'
