#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Prove that this integration lane preserves the frozen Universal Matter v4.8
# preflight parent while carrying byte-identical CUF v0.10.1 forcing artifacts.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

V48_PREFLIGHT_SOURCE="28f66090d29e85d002d5057d7f950956e9564c07"
FORCING_SOURCE="7ea01fd3cbc5a6d703a4869ed71704cb0c474287"

if ! git cat-file -e "$V48_PREFLIGHT_SOURCE^{commit}" 2>/dev/null; then
    printf 'ERROR: frozen v4.8 preflight source commit is unavailable: %s\n' "$V48_PREFLIGHT_SOURCE" >&2
    exit 1
fi
if ! git merge-base --is-ancestor "$V48_PREFLIGHT_SOURCE" HEAD; then
    printf 'ERROR: current integration head no longer descends from frozen v4.8 preflight source.\n' >&2
    exit 1
fi

# The forcing branch is intentionally composed by byte identity rather than by
# claiming a merge commit. These are the Git blob identities from its frozen head.
declare -A EXPECTED_BLOBS=(
    [crates/core/symtropy-sim-contracts/src/observation.rs]="7129730d7581aaea4f3766780245078c208bd925"
    [crates/core/symtropy-sim-contracts/src/lib.rs]="5cf182ceb0a763e4b96e8c137f5dc4fe2a8598be"
    [docs/canon/DETERMINISTIC_FORCING_EVIDENCE_CONTRACT_V0_10_1.md]="5ff4ad6abd358e0cca897d9f168835eec36f83d8"
    [docs/status/CUF_DETERMINISTIC_FORCING_EVIDENCE_V0_10_1_STATUS_2026_09_01.md]="eb73185d46571b3c47db2e44493c71a6751d8272"
    [scripts/qualify-cuf-v0.10.1-forcing.sh]="0613c43d61d04eb68bc07083abe48eb1fa046a3b"
)

for path in "${!EXPECTED_BLOBS[@]}"; do
    if [[ ! -f "$path" ]]; then
        printf 'ERROR: forcing composition file missing: %s\n' "$path" >&2
        exit 1
    fi
    actual="$(git hash-object "$path")"
    if [[ "$actual" != "${EXPECTED_BLOBS[$path]}" ]]; then
        printf 'ERROR: forcing composition blob mismatch for %s\nexpected: %s\nactual:   %s\n' \
            "$path" "${EXPECTED_BLOBS[$path]}" "$actual" >&2
        exit 1
    fi
done

# This list is the complete intentional delta from the frozen v4.8 preflight
# source before the retained 275-file patch is staged.
expected_delta="$(mktemp)"
actual_delta="$(mktemp)"
trap 'rm -f "$expected_delta" "$actual_delta"' EXIT
cat > "$expected_delta" <<'EOF'
.github/workflows/cuf-core-contracts.yml
crates/core/symtropy-sim-contracts/src/lib.rs
crates/core/symtropy-sim-contracts/src/observation.rs
docs/canon/DETERMINISTIC_FORCING_EVIDENCE_CONTRACT_V0_10_1.md
docs/status/CUF_DETERMINISTIC_FORCING_EVIDENCE_V0_10_1_STATUS_2026_09_01.md
docs/status/CUF_V0_10_1_UNIVERSAL_MATTER_V4_8_INTEGRATION_PREFLIGHT_STATUS_2026_09_01.md
scripts/capture-universal-matter-v4.8-cuf-v0.10.1-evidence.sh
scripts/qualify-cuf-v0.10.1-forcing.sh
scripts/qualify-universal-matter-v4.8-cuf-v0.10.1.sh
scripts/verify-cuf-v0.10.1-v4.8-composition.sh
EOF

git diff --name-only "$V48_PREFLIGHT_SOURCE" HEAD | LC_ALL=C sort > "$actual_delta"
LC_ALL=C sort -o "$expected_delta" "$expected_delta"
if ! cmp -s "$expected_delta" "$actual_delta"; then
    printf 'ERROR: integration delta from frozen v4.8 preflight source is not the canonical composition.\n' >&2
    printf '%s\n' '--- expected' >&2
    cat "$expected_delta" >&2
    printf '%s\n' '--- actual' >&2
    cat "$actual_delta" >&2
    exit 1
fi

for inherited in \
    scripts/preflight-universal-matter-v4.8.sh \
    scripts/apply-universal-matter-v4.8.sh \
    scripts/qualify-universal-matter-v4.8-cuf.sh \
    scripts/capture-universal-matter-v4.8-qualification-evidence.sh \
    scripts/verify-universal-matter-v4.8-qualification-capsule.sh \
    scripts/verify-universal-matter-v4.8-promoted-commit.sh \
    docs/canon/UNIVERSAL_MATTER_V4_8_CUF_INTEGRATION_GATE.md \
    docs/canon/QUALIFIED_TREE_PROMOTION_CONTRACT_V0_1.md; do
    if [[ ! -f "$inherited" ]]; then
        printf 'ERROR: inherited v4.8 preflight/promotion artifact missing: %s\n' "$inherited" >&2
        exit 1
    fi
done

printf 'PASS: CUF v0.10.1 + Universal Matter v4.8 preflight composition is canonical\n'
printf 'v4.8 preflight source: %s\n' "$V48_PREFLIGHT_SOURCE"
printf 'forcing source:        %s\n' "$FORCING_SOURCE"
printf 'forcing blobs:         byte-identical\n'
printf 'integration delta:     canonical 10 paths\n'
