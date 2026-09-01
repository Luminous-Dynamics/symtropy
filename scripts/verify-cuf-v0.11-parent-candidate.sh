#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Verify that the current descendant still contains the exact frozen v4.8/CUF
# integration ancestor plus the finalized dependency-light continuation core.
# This script is intentionally non-mutating.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

FROZEN_V48_CUF_ANCESTOR="2222e6c2d23b95372aa9a93763018b61e3d5351f"
FROZEN_CONTINUATION_HEAD="c90b5ad1966263ff35b2eee73f8bf6655377344a"

check_ancestor() {
    local ancestor="$1"
    local label="$2"
    if ! git merge-base --is-ancestor "$ancestor" HEAD; then
        printf 'ERROR: current HEAD does not descend from frozen %s: %s\n' \
            "$label" "$ancestor" >&2
        exit 1
    fi
}

check_blob() {
    local path="$1"
    local expected="$2"
    if [[ ! -f "$path" ]]; then
        printf 'ERROR: required candidate file missing: %s\n' "$path" >&2
        exit 1
    fi
    local actual
    actual="$(git hash-object "$path")"
    if [[ "$actual" != "$expected" ]]; then
        printf 'ERROR: frozen candidate blob mismatch for %s\nexpected: %s\nactual:   %s\n' \
            "$path" "$expected" "$actual" >&2
        exit 1
    fi
}

check_ancestor "$FROZEN_V48_CUF_ANCESTOR" "v4.8 + CUF v0.10.1 ancestor"
check_ancestor "$FROZEN_CONTINUATION_HEAD" "world-continuation semantic head"

# These are working-tree hashes by design. They remain checkable after the
# Universal Matter patch is staged and catch accidental edits before evidence
# is attributed to this parent candidate.
check_blob crates/core/symtropy-sim-contracts/src/continuation.rs \
    b8121d7fbeffa07a11e7097ea5307f9edb4cd9c2
check_blob crates/core/symtropy-sim-contracts/src/lib.rs \
    01609d3ecbf703edf172525332c231428bf94770
check_blob crates/core/symtropy-sim-contracts/src/lineage.rs \
    76eee3bd1a8d5702a0be30356e842f4f16289357
check_blob crates/core/symtropy-sim-contracts/src/observation.rs \
    392c73200b5dc0ab04178157276cc43d34a8eb13
check_blob crates/core/symtropy-sim-contracts/tests/continuation_golden.rs \
    5c4fc814b612a0df2c801272a22013292a2e477c

printf 'PASS: finalized CUF v0.11 parent-candidate composition\n'
printf 'Current HEAD:                   %s\n' "$(git rev-parse HEAD)"
printf 'Frozen v4.8/CUF ancestor:       %s\n' "$FROZEN_V48_CUF_ANCESTOR"
printf 'Frozen continuation semantics:  %s\n' "$FROZEN_CONTINUATION_HEAD"
printf 'Continuation/forcing blobs:     exact\n'
