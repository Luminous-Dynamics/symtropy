#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FREEZER="$ROOT/scripts/freeze-pr-integration-subject.sh"
TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
repo="$TMP/repo"
git init -q "$repo"
git -C "$repo" config user.name "Frozen Subject Test"
git -C "$repo" config user.email "frozen-subject@example.invalid"
printf 'base\n' > "$repo/state.txt"; git -C "$repo" add state.txt; git -C "$repo" commit -q -m base
base="$(git -C "$repo" rev-parse HEAD)"
printf 'candidate\n' > "$repo/state.txt"; git -C "$repo" add state.txt; git -C "$repo" commit -q -m candidate
head="$(git -C "$repo" rev-parse HEAD)"
expected_tree="$(git -C "$repo" merge-tree --write-tree "$base" "$head" | sed -n '1p')"
out="$TMP/output"
(
  cd "$repo"
  GITHUB_OUTPUT="$out" bash "$FREEZER" "$base" "$head" >/dev/null
)
grep -qx "base_sha=$base" "$out"
grep -qx "head_sha=$head" "$out"
grep -qx "tree_sha=$expected_tree" "$out"
echo "ok: exact base/head independently freeze expected merge tree"
set +e
(cd "$repo" && bash "$FREEZER" "$base" "$base" >/dev/null 2>&1); rc=$?
set -e
[[ $rc -eq 2 ]] || { echo "expected wrong checked-out head exit 2, got $rc" >&2; exit 1; }
echo "ok: checked-out head mismatch denied"
set +e
(cd "$repo" && bash "$FREEZER" deadbeef "$head" >/dev/null 2>&1); rc=$?
set -e
[[ $rc -eq 64 ]] || { echo "expected malformed base exit 64, got $rc" >&2; exit 1; }
echo "ok: malformed base denied"

# Build two divergent commits that both edit the same line differently.
git -C "$repo" checkout -q -b conflict-base "$base"
printf 'left\n' > "$repo/state.txt"; git -C "$repo" add state.txt; git -C "$repo" commit -q -m left
left="$(git -C "$repo" rev-parse HEAD)"
git -C "$repo" checkout -q -b conflict-head "$base"
printf 'right\n' > "$repo/state.txt"; git -C "$repo" add state.txt; git -C "$repo" commit -q -m right
right="$(git -C "$repo" rev-parse HEAD)"
set +e
(cd "$repo" && bash "$FREEZER" "$left" "$right" >/dev/null 2>&1); rc=$?
set -e
[[ $rc -eq 2 ]] || { echo "expected conflicting integration exit 2, got $rc" >&2; exit 1; }
echo "ok: conflicting integration denied deterministically"
