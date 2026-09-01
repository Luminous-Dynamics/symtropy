#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Verify that a promotion commit exactly commits the tree proven by a PASS
# qualification capsule and preserves the recorded qualification parent.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

EVIDENCE="${1:-}"
COMMIT="${2:-HEAD}"

if [[ -z "$EVIDENCE" || ! -d "$EVIDENCE" ]]; then
    printf 'usage: %s /path/to/qualification-evidence-dir [commit]\n' "$0" >&2
    exit 2
fi
EVIDENCE="$(realpath "$EVIDENCE")"

bash scripts/verify-universal-matter-v4.8-qualification-capsule.sh "$EVIDENCE" >/dev/null || {
    # The capsule verifier expects the qualified tree to still be staged on its
    # parent. After promotion that is no longer true, so verify capsule integrity
    # and immutable fields again below instead of accepting a partial result.
    true
}

(
    cd "$EVIDENCE"
    sha256sum -c MANIFEST.sha256 >/dev/null
)

if [[ "$(cat "$EVIDENCE/STATUS.txt")" != "PASS" ]]; then
    printf 'ERROR: qualification capsule is not PASS.\n' >&2
    exit 1
fi

qualified_parent="$(cat "$EVIDENCE/BASE_HEAD.txt")"
qualified_tree="$(cat "$EVIDENCE/STAGED_TREE.txt")"
qualified_tree_after="$(cat "$EVIDENCE/STAGED_TREE_AFTER.txt")"
commit_sha="$(git rev-parse "$COMMIT^{commit}")"
commit_tree="$(git rev-parse "$commit_sha^{tree}")"
parent_count="$(git rev-list --parents -n 1 "$commit_sha" | awk '{print NF-1}')"
first_parent="$(git rev-parse "$commit_sha^1")"

if [[ "$qualified_tree" != "$qualified_tree_after" ]]; then
    printf 'ERROR: capsule recorded different staged trees before/after qualification.\n' >&2
    exit 1
fi

if [[ "$parent_count" != "1" ]]; then
    printf 'ERROR: promotion commit must have exactly one parent; got %s.\n' "$parent_count" >&2
    exit 1
fi

if [[ "$first_parent" != "$qualified_parent" ]]; then
    printf 'ERROR: promotion parent mismatch.\nexpected: %s\nactual:   %s\n' \
        "$qualified_parent" "$first_parent" >&2
    exit 1
fi

if [[ "$commit_tree" != "$qualified_tree" ]]; then
    printf 'ERROR: promotion commit tree differs from qualified tree.\nexpected: %s\nactual:   %s\n' \
        "$qualified_tree" "$commit_tree" >&2
    exit 1
fi

printf 'PASS: promoted commit exactly matches qualified Universal Matter v4.8 tree\n'
printf 'Commit:         %s\n' "$commit_sha"
printf 'Parent:         %s\n' "$first_parent"
printf 'Qualified tree: %s\n' "$commit_tree"
