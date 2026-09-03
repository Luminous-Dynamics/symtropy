#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Run the proven CUF v0.11 lock-repair generator, then enrich a successful
# GENERATED_UNCOMMITTED capsule with a deterministic semantic package delta.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

CONTEXT="${1:-}"
OUT="${2:-}"
ANALYZER_PATH="scripts/analyze-cuf-v0.11-cargo-lock-delta.py"
TEST_PATH="scripts/test-cuf-v0.11-cargo-lock-delta.py"

if [[ -z "$CONTEXT" || -z "$OUT" ]]; then
    printf 'usage: %s <parent|v4.8> /path/to/evidence-dir\n' "$0" >&2
    exit 2
fi
case "$CONTEXT" in
    parent|v4.8) ;;
    *) printf 'ERROR: context must be parent or v4.8\n' >&2; exit 2 ;;
esac
for cmd in git python3 mktemp sha256sum find sort xargs realpath awk cmp bash cat cp rm; do
    command -v "$cmd" >/dev/null 2>&1 || {
        printf 'ERROR: required command unavailable: %s\n' "$cmd" >&2
        exit 1
    }
done

finalize_manifest() {
    (
        cd "$OUT"
        find . -maxdepth 1 -type f ! -name MANIFEST.sha256 -printf '%P\n' \
            | LC_ALL=C sort | xargs -r sha256sum > MANIFEST.sha256
    )
}

# The base generator owns mutation, failure classification, repository locking,
# and the initial evidence capsule.
set +e
bash scripts/prepare-cuf-v0.11-cargo-lock-repair.sh "$CONTEXT" "$OUT"
prepare_status=$?
set -e
if [[ "$prepare_status" -ne 0 ]]; then
    exit "$prepare_status"
fi

OUT="$(realpath "$OUT")"
[[ -f "$OUT/STATUS.txt" && "$(cat "$OUT/STATUS.txt")" == "GENERATED_UNCOMMITTED" ]] || {
    printf 'ERROR: base repair generator did not produce GENERATED_UNCOMMITTED evidence.\n' >&2
    exit 1
}
for required in BASE_HEAD.txt CARGO_LOCK_SHA256_AFTER.txt LINEAGE.txt; do
    [[ -f "$OUT/$required" ]] || {
        printf 'ERROR: repair evidence is missing %s\n' "$required" >&2
        exit 1
    }
done
base_head="$(cat "$OUT/BASE_HEAD.txt")"
expected_lock_after="$(awk '{print $1}' "$OUT/CARGO_LOCK_SHA256_AFTER.txt")"
[[ "$expected_lock_after" =~ ^[0-9a-f]{64}$ ]] || {
    printf 'ERROR: malformed post-repair Cargo.lock SHA-256 evidence.\n' >&2
    exit 1
}

# Use analyzer + regression test committed in the exact repair base, not an
# arbitrary later working tree copy.
for path in "$ANALYZER_PATH" "$TEST_PATH"; do
    git cat-file -e "$base_head:$path" 2>/dev/null || {
        printf 'ERROR: repair base does not contain semantic tool: %s\n' "$path" >&2
        exit 1
    }
done
analyzer_blob="$(git rev-parse "$base_head:$ANALYZER_PATH")"
test_blob="$(git rev-parse "$base_head:$TEST_PATH")"
analyzer_name="${ANALYZER_PATH##*/}"
test_name="${TEST_PATH##*/}"
tmp_dir="$(mktemp -d /tmp/cuf-v011-lock-semantic.XXXXXX)"
trap 'rm -rf "$tmp_dir"' EXIT

git show "$base_head:$ANALYZER_PATH" > "$tmp_dir/$analyzer_name"
git show "$base_head:$TEST_PATH" > "$tmp_dir/$test_name"
git show "$base_head:Cargo.lock" > "$tmp_dir/Cargo.lock.before"
cp Cargo.lock "$tmp_dir/Cargo.lock.after"

# #120 releases its execution lock before returning. Semantic enrichment is
# observational, so independently prove the generated lock and repository state
# are still exactly what #120 emitted before and after analyzer execution.
lock_hash_now="$(sha256sum Cargo.lock | awk '{print $1}')"
[[ "$lock_hash_now" == "$expected_lock_after" ]] || {
    printf 'SEMANTIC_PRECHECK_LOCK_DRIFT\n' > "$OUT/STATUS.txt"
    finalize_manifest
    printf 'ERROR: Cargo.lock drifted after base repair generation.\n' >&2
    exit 1
}
[[ "$(git diff --name-only)" == "Cargo.lock" ]] || {
    printf 'SEMANTIC_PRECHECK_REPOSITORY_DRIFT\n' > "$OUT/STATUS.txt"
    finalize_manifest
    printf 'ERROR: semantic enrichment requires Cargo.lock to be the only tracked working-tree delta.\n' >&2
    exit 1
}
[[ -z "$(git diff --cached --name-only)" && -z "$(git ls-files --others --exclude-standard)" ]] || {
    printf 'SEMANTIC_PRECHECK_REPOSITORY_DRIFT\n' > "$OUT/STATUS.txt"
    finalize_manifest
    printf 'ERROR: semantic enrichment rejects staged or untracked repository drift.\n' >&2
    exit 1
}

printf '%s\n' "$(python3 --version 2>&1)" > "$OUT/SEMANTIC_PYTHON_VERSION.txt"
printf '%s\t%s\n' "$analyzer_blob" "$ANALYZER_PATH" > "$OUT/SEMANTIC_ANALYZER_BLOB.txt"
printf '%s\t%s\n' "$test_blob" "$TEST_PATH" > "$OUT/SEMANTIC_ANALYZER_TEST_BLOB.txt"

set +e
python3 "$tmp_dir/$test_name" \
    > "$OUT/SEMANTIC_ANALYZER_TEST.stdout.log" \
    2> "$OUT/SEMANTIC_ANALYZER_TEST.stderr.log"
test_status=$?
set -e
printf '%s\n' "$test_status" > "$OUT/SEMANTIC_ANALYZER_TEST_EXIT.txt"
if [[ "$test_status" -ne 0 ]]; then
    printf 'SEMANTIC_ANALYZER_TEST_FAILED\n' > "$OUT/STATUS.txt"
    finalize_manifest
    cat "$OUT/SEMANTIC_ANALYZER_TEST.stderr.log" >&2
    exit "$test_status"
fi

python3 "$tmp_dir/$analyzer_name" \
    "$tmp_dir/Cargo.lock.before" "$tmp_dir/Cargo.lock.after" --format json \
    > "$OUT/CARGO_LOCK_SEMANTIC_DELTA.json"
python3 "$tmp_dir/$analyzer_name" \
    "$tmp_dir/Cargo.lock.before" "$tmp_dir/Cargo.lock.after" --format text \
    > "$OUT/CARGO_LOCK_SEMANTIC_DELTA.txt"

lock_hash_after_semantic="$(sha256sum Cargo.lock | awk '{print $1}')"
[[ "$lock_hash_after_semantic" == "$expected_lock_after" ]] || {
    printf 'SEMANTIC_POSTCHECK_LOCK_DRIFT\n' > "$OUT/STATUS.txt"
    finalize_manifest
    printf 'ERROR: Cargo.lock changed during semantic enrichment.\n' >&2
    exit 1
}
[[ "$(git diff --name-only)" == "Cargo.lock" ]] || {
    printf 'SEMANTIC_POSTCHECK_REPOSITORY_DRIFT\n' > "$OUT/STATUS.txt"
    finalize_manifest
    printf 'ERROR: repository drifted during semantic enrichment.\n' >&2
    exit 1
}
[[ -z "$(git diff --cached --name-only)" && -z "$(git ls-files --others --exclude-standard)" ]] || {
    printf 'SEMANTIC_POSTCHECK_REPOSITORY_DRIFT\n' > "$OUT/STATUS.txt"
    finalize_manifest
    printf 'ERROR: semantic enrichment produced staged or untracked repository drift.\n' >&2
    exit 1
}

printf 'DESCRIPTIVE_REVIEW_REQUIRED\n' > "$OUT/SEMANTIC_DELTA_POLICY.txt"

cat >> "$OUT/LINEAGE.txt" <<EOF
semantic_delta_schema=symtropy.cuf.cargo-lock-semantic-delta.v1
semantic_analyzer_blob=$analyzer_blob
semantic_analyzer_test_blob=$test_blob
semantic_analyzer_test_status=PASS
semantic_delta_policy=DESCRIPTIVE_REVIEW_REQUIRED
EOF

# The semantic wrapper becomes the final capsule producer, so re-manifest every
# file including semantic evidence and updated lineage.
finalize_manifest

printf '\nPASS: semantic Cargo.lock delta captured\n'
printf 'Evidence:            %s\n' "$OUT"
printf 'Analyzer blob:       %s\n' "$analyzer_blob"
printf 'Analyzer test blob:  %s\n' "$test_blob"
printf 'Analyzer tests:      PASS\n'
printf '\nReview semantic delta before committing:\n'
printf '  cat %q\n' "$OUT/CARGO_LOCK_SEMANTIC_DELTA.txt"
printf '  less %q\n' "$OUT/CARGO_LOCK_SEMANTIC_DELTA.json"
printf '\nThis report is descriptive. Q1 and Q2 remain NOT_CLAIMED.\n'
