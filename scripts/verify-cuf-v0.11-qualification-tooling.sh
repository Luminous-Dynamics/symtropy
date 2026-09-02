#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Prove that every critical script used to stage, qualify, capture, and promote
# the CUF v0.11 pre-Q2 parent candidate is byte-identical to the committed HEAD.
# Output is a stable <git-blob><TAB><path> manifest suitable for evidence capture.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

CANDIDATE_HEAD="$(git rev-parse HEAD^{commit})"

TOOLING_PATHS=(
    scripts/preflight-universal-matter-v4.8.sh
    scripts/apply-universal-matter-v4.8.sh
    scripts/verify-cuf-v0.11-parent-candidate.sh
    scripts/verify-cuf-v0.11-qualification-tooling.sh
    scripts/qualify-cuf-v0.11-parent-candidate.sh
    scripts/capture-cuf-v0.11-parent-candidate-evidence.sh
    scripts/qualify-universal-matter-v4.8-cuf-v0.10.1.sh
    scripts/qualify-world-continuation-core-v0.1.sh
    scripts/qualify-cuf-v0.10.1-forcing.sh
    scripts/qualify-cuf-v0.10-stack.sh
    scripts/check-workspace.sh
    scripts/check-licenses.sh
    scripts/verify-cuf-v0.11-parent-promotion.sh
)

if [[ -f scripts/verify-terrain-handoff.sh ]]; then
    TOOLING_PATHS+=(scripts/verify-terrain-handoff.sh)
fi

for path in "${TOOLING_PATHS[@]}"; do
    if ! git cat-file -e "$CANDIDATE_HEAD:$path" 2>/dev/null; then
        printf 'ERROR: critical qualification tool is not committed at candidate HEAD: %s\n' \
            "$path" >&2
        exit 1
    fi
    if [[ ! -f "$path" ]]; then
        printf 'ERROR: critical qualification tool is missing from working tree: %s\n' \
            "$path" >&2
        exit 1
    fi
    if ! git diff --quiet "$CANDIDATE_HEAD" -- "$path"; then
        printf 'ERROR: critical qualification tool differs from candidate HEAD: %s\n' \
            "$path" >&2
        exit 1
    fi

    committed_blob="$(git rev-parse "$CANDIDATE_HEAD:$path")"
    working_blob="$(git hash-object "$path")"
    if [[ "$working_blob" != "$committed_blob" ]]; then
        printf 'ERROR: qualification tool blob mismatch for %s\ncommitted: %s\nworking:   %s\n' \
            "$path" "$committed_blob" "$working_blob" >&2
        exit 1
    fi
    printf '%s\t%s\n' "$committed_blob" "$path"
done
