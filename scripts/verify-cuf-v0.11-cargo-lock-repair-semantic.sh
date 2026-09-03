#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Verify the exact Cargo.lock-only repair proof from the recorded repair base,
# then recompute and compare the deterministic semantic package delta from the
# committed parent/child locks.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

EVIDENCE="${1:-}"
COMMIT="${2:-HEAD}"
ANALYZER_PATH="scripts/analyze-cuf-v0.11-cargo-lock-delta.py"
TEST_PATH="scripts/test-cuf-v0.11-cargo-lock-delta.py"
BASE_VERIFIER_PATH="scripts/verify-cuf-v0.11-cargo-lock-repair.sh"

if [[ -z "$EVIDENCE" || ! -d "$EVIDENCE" ]]; then
    printf 'usage: %s /path/to/lock-repair-evidence [commit]\n' "$0" >&2
    exit 2
fi
EVIDENCE="$(realpath "$EVIDENCE")"
for cmd in git python3 mktemp cmp realpath sha256sum awk grep diff bash cat rm cp mkdir; do
    command -v "$cmd" >/dev/null 2>&1 || {
        printf 'ERROR: required command unavailable: %s\n' "$cmd" >&2
        exit 1
    }
done

# Verify the enriched capsule before trusting BASE_HEAD to select executable tooling.
[[ -f "$EVIDENCE/MANIFEST.sha256" ]] || {
    printf 'ERROR: semantic lock evidence is missing MANIFEST.sha256\n' >&2
    exit 1
}
(
    cd "$EVIDENCE"
    sha256sum -c MANIFEST.sha256 >/dev/null
)

for name in \
    BASE_HEAD.txt \
    BASE_REPAIR_MANIFEST.sha256 \
    BASE_REPAIR_MANIFEST_SHA256.txt \
    BASE_REPAIR_LINEAGE.txt \
    CARGO_LOCK_SHA256_AFTER.txt \
    CARGO_LOCK_SEMANTIC_DELTA.json \
    CARGO_LOCK_SEMANTIC_DELTA.txt \
    SEMANTIC_LOCK_SNAPSHOT_SHA256.txt \
    SEMANTIC_ANALYZER_BLOB.txt \
    SEMANTIC_ANALYZER_TEST_BLOB.txt \
    SEMANTIC_ANALYZER_TEST_EXIT.txt \
    SEMANTIC_ANALYZER_TEST.stdout.log \
    SEMANTIC_ANALYZER_TEST.stderr.log \
    SEMANTIC_DELTA_POLICY.txt; do
    [[ -f "$EVIDENCE/$name" ]] || {
        printf 'ERROR: semantic lock evidence missing %s\n' "$name" >&2
        exit 1
    }
done

base_head="$(cat "$EVIDENCE/BASE_HEAD.txt")"
git cat-file -e "$base_head^{commit}" 2>/dev/null || {
    printf 'ERROR: recorded repair base is not a reachable commit: %s\n' "$base_head" >&2
    exit 1
}

recorded_base_manifest_sha256="$(cat "$EVIDENCE/BASE_REPAIR_MANIFEST_SHA256.txt")"
[[ "$recorded_base_manifest_sha256" =~ ^[0-9a-f]{64}$ ]] || {
    printf 'ERROR: malformed base repair manifest SHA-256 evidence.\n' >&2
    exit 1
}
actual_base_manifest_sha256="$(sha256sum "$EVIDENCE/BASE_REPAIR_MANIFEST.sha256" | awk '{print $1}')"
[[ "$actual_base_manifest_sha256" == "$recorded_base_manifest_sha256" ]] || {
    printf 'ERROR: preserved base repair manifest hash does not match evidence.\n' >&2
    exit 1
}

recorded_lock_after="$(awk '{print $1}' "$EVIDENCE/CARGO_LOCK_SHA256_AFTER.txt")"
recorded_snapshot_hash="$(awk '{print $1}' "$EVIDENCE/SEMANTIC_LOCK_SNAPSHOT_SHA256.txt")"
for value in "$recorded_lock_after" "$recorded_snapshot_hash"; do
    [[ "$value" =~ ^[0-9a-f]{64}$ ]] || {
        printf 'ERROR: malformed semantic Cargo.lock SHA-256 evidence.\n' >&2
        exit 1
    }
done
[[ "$recorded_snapshot_hash" == "$recorded_lock_after" ]] || {
    printf 'ERROR: semantic lock snapshot identity differs from #120 post-repair lock identity.\n' >&2
    exit 1
}

# Reconstruct the exact successful #120 capsule as it existed before semantic
# enrichment. The semantic wrapper changes LINEAGE.txt and MANIFEST.sha256, so
# preserve/replay those originals rather than asking the historical base verifier
# to interpret the later derivative capsule.
tmp_dir="$(mktemp -d /tmp/cuf-v011-lock-semantic-verify.XXXXXX)"
trap 'rm -rf "$tmp_dir"' EXIT
base_capsule="$tmp_dir/base-capsule"
mkdir "$base_capsule"

while IFS= read -r manifest_line; do
    [[ "$manifest_line" =~ ^([0-9a-f]{64})[[:space:]][[:space:]]([^/]+)$ ]] || {
        printf 'ERROR: malformed or non-flat path in preserved base manifest: %s\n' "$manifest_line" >&2
        exit 1
    }
    path="${BASH_REMATCH[2]}"
    source_path="$EVIDENCE/$path"
    if [[ "$path" == "LINEAGE.txt" ]]; then
        source_path="$EVIDENCE/BASE_REPAIR_LINEAGE.txt"
    fi
    [[ -f "$source_path" ]] || {
        printf 'ERROR: preserved base capsule source file missing: %s\n' "$path" >&2
        exit 1
    }
    cp "$source_path" "$base_capsule/$path"
done < "$EVIDENCE/BASE_REPAIR_MANIFEST.sha256"
cp "$EVIDENCE/BASE_REPAIR_MANIFEST.sha256" "$base_capsule/MANIFEST.sha256"
(
    cd "$base_capsule"
    sha256sum -c MANIFEST.sha256 >/dev/null
) || {
    printf 'ERROR: reconstructed pre-semantic #120 capsule fails checksum verification.\n' >&2
    exit 1
}

# Execute #120's exact verifier from the recorded repair base. The semantic
# child must not gain authority by substituting a later working-tree verifier.
for path in "$BASE_VERIFIER_PATH" "$ANALYZER_PATH" "$TEST_PATH"; do
    git cat-file -e "$base_head:$path" 2>/dev/null || {
        printf 'ERROR: repair base is missing required verification tool: %s\n' "$path" >&2
        exit 1
    }
done

analyzer_name="${ANALYZER_PATH##*/}"
test_name="${TEST_PATH##*/}"
git show "$base_head:$BASE_VERIFIER_PATH" > "$tmp_dir/base-verifier.sh"
git show "$base_head:$ANALYZER_PATH" > "$tmp_dir/$analyzer_name"
git show "$base_head:$TEST_PATH" > "$tmp_dir/$test_name"
bash "$tmp_dir/base-verifier.sh" "$base_capsule" "$COMMIT"

[[ "$(cat "$EVIDENCE/SEMANTIC_DELTA_POLICY.txt")" == "DESCRIPTIVE_REVIEW_REQUIRED" ]] || {
    printf 'ERROR: unexpected semantic delta policy.\n' >&2
    exit 1
}
[[ "$(cat "$EVIDENCE/SEMANTIC_ANALYZER_TEST_EXIT.txt")" == "0" ]] || {
    printf 'ERROR: semantic analyzer regression test did not pass during evidence production.\n' >&2
    exit 1
}

commit_sha="$(git rev-parse "$COMMIT^{commit}")"
recorded_blob="$(awk -F '\t' -v path="$ANALYZER_PATH" '$2 == path {print $1}' \
    "$EVIDENCE/SEMANTIC_ANALYZER_BLOB.txt")"
recorded_test_blob="$(awk -F '\t' -v path="$TEST_PATH" '$2 == path {print $1}' \
    "$EVIDENCE/SEMANTIC_ANALYZER_TEST_BLOB.txt")"
for value in "$recorded_blob" "$recorded_test_blob"; do
    [[ "$value" =~ ^[0-9a-f]{40}$ ]] || {
        printf 'ERROR: malformed semantic tool blob evidence.\n' >&2
        exit 1
    }
done
committed_blob="$(git rev-parse "$base_head:$ANALYZER_PATH")"
committed_test_blob="$(git rev-parse "$base_head:$TEST_PATH")"
[[ "$recorded_blob" == "$committed_blob" ]] || {
    printf 'ERROR: semantic analyzer evidence does not match repair base.\n' >&2
    exit 1
}
[[ "$recorded_test_blob" == "$committed_test_blob" ]] || {
    printf 'ERROR: semantic analyzer test evidence does not match repair base.\n' >&2
    exit 1
}

for expected in \
    'semantic_base_manifest_status=PASS' \
    "semantic_base_manifest_sha256=$recorded_base_manifest_sha256" \
    "semantic_lock_snapshot_sha256=$recorded_snapshot_hash" \
    'semantic_delta_schema=symtropy.cuf.cargo-lock-semantic-delta.v1' \
    "semantic_analyzer_blob=$recorded_blob" \
    "semantic_analyzer_test_blob=$recorded_test_blob" \
    'semantic_analyzer_test_status=PASS' \
    'semantic_delta_policy=DESCRIPTIVE_REVIEW_REQUIRED'; do
    grep -Fxq "$expected" "$EVIDENCE/LINEAGE.txt" || {
        printf 'ERROR: missing semantic lineage binding: %s\n' "$expected" >&2
        exit 1
    }
done

# Re-run the exact regression suite from the repair base. Timing/log formatting is
# intentionally not compared byte-for-byte; only deterministic PASS is required.
set +e
python3 "$tmp_dir/$test_name" > "$tmp_dir/test.stdout.log" 2> "$tmp_dir/test.stderr.log"
rerun_test_status=$?
set -e
if [[ "$rerun_test_status" -ne 0 ]]; then
    cat "$tmp_dir/test.stderr.log" >&2
    printf 'ERROR: semantic analyzer regression suite fails during verification.\n' >&2
    exit "$rerun_test_status"
fi

git show "$base_head:Cargo.lock" > "$tmp_dir/Cargo.lock.before"
git show "$commit_sha:Cargo.lock" > "$tmp_dir/Cargo.lock.after"
committed_lock_hash="$(sha256sum "$tmp_dir/Cargo.lock.after" | awk '{print $1}')"
[[ "$committed_lock_hash" == "$recorded_snapshot_hash" ]] || {
    printf 'ERROR: committed Cargo.lock identity differs from analyzed semantic snapshot.\n' >&2
    exit 1
}

python3 "$tmp_dir/$analyzer_name" \
    "$tmp_dir/Cargo.lock.before" "$tmp_dir/Cargo.lock.after" --format json \
    > "$tmp_dir/semantic.json"
python3 "$tmp_dir/$analyzer_name" \
    "$tmp_dir/Cargo.lock.before" "$tmp_dir/Cargo.lock.after" --format text \
    > "$tmp_dir/semantic.txt"

cmp -s "$EVIDENCE/CARGO_LOCK_SEMANTIC_DELTA.json" "$tmp_dir/semantic.json" || {
    printf 'ERROR: committed semantic Cargo.lock JSON delta differs from evidence.\n' >&2
    diff -u "$EVIDENCE/CARGO_LOCK_SEMANTIC_DELTA.json" "$tmp_dir/semantic.json" >&2 || true
    exit 1
}
cmp -s "$EVIDENCE/CARGO_LOCK_SEMANTIC_DELTA.txt" "$tmp_dir/semantic.txt" || {
    printf 'ERROR: committed semantic Cargo.lock text delta differs from evidence.\n' >&2
    diff -u "$EVIDENCE/CARGO_LOCK_SEMANTIC_DELTA.txt" "$tmp_dir/semantic.txt" >&2 || true
    exit 1
}

base_verifier_blob="$(git rev-parse "$base_head:$BASE_VERIFIER_PATH")"
printf 'PASS: semantic Cargo.lock delta exactly matches committed repair\n'
printf 'Repair commit:       %s\n' "$commit_sha"
printf 'Repair base:         %s\n' "$base_head"
printf 'Base capsule:        PASS %s\n' "$recorded_base_manifest_sha256"
printf 'Lock snapshot:       %s\n' "$recorded_snapshot_hash"
printf 'Base verifier blob:  %s\n' "$base_verifier_blob"
printf 'Analyzer blob:       %s\n' "$recorded_blob"
printf 'Analyzer test blob:  %s\n' "$recorded_test_blob"
printf 'Analyzer tests:      PASS\n'
printf 'Policy:              DESCRIPTIVE_REVIEW_REQUIRED\n'
printf 'Qualification:       NOT_CLAIMED\n'
printf 'Q2 status:           NOT_CLAIMED\n'