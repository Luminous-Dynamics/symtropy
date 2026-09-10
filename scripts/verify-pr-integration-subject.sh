#!/usr/bin/env bash
# Fail closed unless the checked-out PR merge commit is exactly the integration
# subject declared by the pull_request event.
set -euo pipefail

usage() {
  echo "usage: $0 <expected-base-sha> <expected-head-sha>" >&2
}

if [[ $# -ne 2 ]]; then
  usage
  exit 64
fi

expected_base="$1"
expected_head="$2"
sha_re='^[0-9a-f]{40}$'

if [[ ! "$expected_base" =~ $sha_re ]] || [[ ! "$expected_head" =~ $sha_re ]]; then
  echo "INTEGRATION_SUBJECT_INPUT_INVALID: expected full lowercase 40-hex SHAs" >&2
  exit 64
fi

actual_commit="$(git rev-parse 'HEAD^{commit}')"
actual_tree="$(git rev-parse 'HEAD^{tree}')"

# Bash 3-compatible: macOS runners do not guarantee mapfile/readarray.
parent_words="$(git cat-file -p "$actual_commit" | sed -n 's/^parent //p' | tr '\n' ' ')"
parents=()
if [[ -n "$parent_words" ]]; then
  read -r -a parents <<< "$parent_words"
fi

if [[ ${#parents[@]} -ne 2 ]]; then
  echo "INTEGRATION_SUBJECT_MISMATCH: checked-out commit $actual_commit has ${#parents[@]} parent(s), expected exactly 2" >&2
  exit 2
fi

actual_base="${parents[0]}"
actual_head="${parents[1]}"

if [[ "$actual_base" != "$expected_base" ]] || [[ "$actual_head" != "$expected_head" ]]; then
  cat >&2 <<EOF
INTEGRATION_SUBJECT_MISMATCH
expected_base=$expected_base
expected_head=$expected_head
actual_merge=$actual_commit
actual_base=$actual_base
actual_head=$actual_head
actual_tree=$actual_tree
EOF
  exit 2
fi

cat <<EOF
INTEGRATION_SUBJECT_VERIFIED
base_sha=$actual_base
head_sha=$actual_head
merge_sha=$actual_commit
tree_sha=$actual_tree
EOF

if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  {
    echo "base_sha=$actual_base"
    echo "head_sha=$actual_head"
    echo "merge_sha=$actual_commit"
    echo "tree_sha=$actual_tree"
  } >> "$GITHUB_OUTPUT"
fi

if [[ -n "${GITHUB_ENV:-}" ]]; then
  {
    echo "LUMINOUS_INTEGRATION_BASE_SHA=$actual_base"
    echo "LUMINOUS_INTEGRATION_HEAD_SHA=$actual_head"
    echo "LUMINOUS_INTEGRATION_MERGE_SHA=$actual_commit"
    echo "LUMINOUS_INTEGRATION_TREE_SHA=$actual_tree"
  } >> "$GITHUB_ENV"
fi
