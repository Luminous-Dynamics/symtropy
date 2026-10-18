#!/usr/bin/env bash
set -euo pipefail
manifest="$1"
upstream_root="$2"
command -v jq >/dev/null 2>&1 || { echo "MISSING_TOOL: jq" >&2; exit 2; }
command -v git >/dev/null 2>&1 || { echo "MISSING_TOOL: git" >&2; exit 2; }
[[ -f "$manifest" ]] || { echo "MISSING_MANIFEST: $manifest" >&2; exit 2; }
[[ -d "$upstream_root/.git" ]] || { echo "MISSING_EXTERNAL_GIT_CHECKOUT: $upstream_root" >&2; exit 2; }
expected_commit="$(jq -er '.upstream.commit_sha' "$manifest")"
actual_commit="$(git -C "$upstream_root" rev-parse HEAD)"
[[ "$actual_commit" == "$expected_commit" ]] || { echo "UPSTREAM_COMMIT_MISMATCH: expected=$expected_commit actual=$actual_commit" >&2; exit 1; }
expected_cases="$(jq -er '.case_count' "$manifest")"
expected_files="$(jq -er '.file_count' "$manifest")"
actual_cases="$(jq -er '.cases | length' "$manifest")"
actual_files=$((actual_cases * 2))
[[ "$actual_cases" -eq "$expected_cases" ]] || { echo "CASE_COUNT_MISMATCH: expected=$expected_cases actual=$actual_cases" >&2; exit 1; }
[[ "$actual_files" -eq "$expected_files" ]] || { echo "FILE_COUNT_MISMATCH: expected=$expected_files actual=$actual_files" >&2; exit 1; }
jq -e '(.cases | map(.case_id) | unique | length) == (.cases | length)' "$manifest" >/dev/null || { echo "DUPLICATE_CASE_IDS" >&2; exit 1; }
jq -e '([.cases[].ids.path, .cases[].ifc.path] | length) == ([.cases[].ids.path, .cases[].ifc.path] | unique | length)' "$manifest" >/dev/null || { echo "DUPLICATE_SOURCE_PATHS" >&2; exit 1; }
jq -e 'all(.cases[]; .expected_outcome == "UNVERIFIED" and .execution_status == "NOT_RUN_EXTERNAL_CORPUS")' "$manifest" >/dev/null || { echo "UNSUPPORTED_INFERRED_OUTCOME_PRESENT" >&2; exit 1; }
verify_object() {
  local relative_path="$1" expected_oid="$2" expected_size="$3" abs_path="$upstream_root/$relative_path"
  [[ -f "$abs_path" ]] || { echo "SOURCE_FILE_MISSING: $relative_path" >&2; return 1; }
  local actual_oid actual_size
  actual_oid="$(git -C "$upstream_root" hash-object -- "$relative_path")"
  actual_size="$(wc -c < "$abs_path" | tr -d '[:space:]')"
  [[ "$actual_oid" == "$expected_oid" ]] || { echo "GIT_BLOB_OID_MISMATCH: $relative_path expected=$expected_oid actual=$actual_oid" >&2; return 1; }
  [[ "$actual_size" == "$expected_size" ]] || { echo "BYTE_SIZE_MISMATCH: $relative_path expected=$expected_size actual=$actual_size" >&2; return 1; }
}
verify_object "$(jq -er '.upstream.license.path' "$manifest")" "$(jq -er '.upstream.license.git_blob_oid' "$manifest")" "$(jq -er '.upstream.license.size_bytes' "$manifest")"
while IFS= read -r row; do
  for kind in ids ifc; do
    path="$(jq -er --arg kind "$kind" '.[$kind].path' <<<"$row")"
    oid="$(jq -er --arg kind "$kind" '.[$kind].git_blob_oid' <<<"$row")"
    size="$(jq -er --arg kind "$kind" '.[$kind].size_bytes' <<<"$row")"
    verify_object "$path" "$oid" "$size"
  done
done < <(jq -c '.cases[]' "$manifest")
printf 'CORPUS_INVENTORY_INTEGRITY_OK: commit=%s cases=%s files=%s\n' "$expected_commit" "$expected_cases" "$expected_files"
printf 'CORPUS_OUTCOME_STATUS: UNVERIFIED / NOT_RUN_EXTERNAL_CORPUS (no outcomes inferred)\n'
