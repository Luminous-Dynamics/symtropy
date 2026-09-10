#!/usr/bin/env bash
set -euo pipefail

usage() { echo "usage: $0 <expected-base-sha> <expected-head-sha>" >&2; }
[[ $# -eq 2 ]] || { usage; exit 64; }
base="$1"; head="$2"
sha_re='^[0-9a-f]{40}$'
[[ "$base" =~ $sha_re && "$head" =~ $sha_re ]] || {
  echo "INTEGRATION_SUBJECT_INPUT_INVALID: expected full lowercase 40-hex SHAs" >&2
  exit 64
}
actual_head="$(git rev-parse 'HEAD^{commit}')"
[[ "$actual_head" == "$head" ]] || {
  echo "INTEGRATION_SUBJECT_HEAD_MISMATCH: expected=$head actual=$actual_head" >&2
  exit 2
}
git cat-file -e "$base^{commit}" 2>/dev/null || {
  echo "INTEGRATION_SUBJECT_BASE_MISSING: $base" >&2
  exit 2
}
tree="$(git merge-tree --write-tree "$base" "$head" | sed -n '1p')"
[[ "$tree" =~ $sha_re ]] || {
  echo "INTEGRATION_SUBJECT_MERGE_INVALID: merge-tree did not return exact tree SHA" >&2
  exit 2
}
cat <<EOF
FROZEN_INTEGRATION_SUBJECT
base_sha=$base
head_sha=$head
tree_sha=$tree
EOF
if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  {
    echo "base_sha=$base"
    echo "head_sha=$head"
    echo "tree_sha=$tree"
  } >> "$GITHUB_OUTPUT"
fi
