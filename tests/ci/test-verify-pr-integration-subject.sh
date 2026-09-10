#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERIFIER="$ROOT/scripts/verify-pr-integration-subject.sh"
TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
repo="$TMP/repo"
git init -q "$repo"
git -C "$repo" config user.name "Integration Subject Test"
git -C "$repo" config user.email "integration-subject@example.invalid"
printf 'base\n' > "$repo/state.txt"; git -C "$repo" add state.txt; git -C "$repo" commit -q -m base
base="$(git -C "$repo" rev-parse HEAD)"
printf 'candidate\n' > "$repo/state.txt"; git -C "$repo" add state.txt; git -C "$repo" commit -q -m candidate
head="$(git -C "$repo" rev-parse HEAD)"; tree="$(git -C "$repo" rev-parse 'HEAD^{tree}')"
merge="$(printf 'synthetic integration\n' | git -C "$repo" commit-tree "$tree" -p "$base" -p "$head")"
git -C "$repo" checkout -q --detach "$merge"
(cd "$repo" && bash "$VERIFIER" "$base" "$head" "$tree" >/dev/null)
echo "ok: exact two-parent/tree integration subject accepted"
set +e
(cd "$repo" && bash "$VERIFIER" "$head" "$base" "$tree" >/dev/null 2>&1); rc=$?
set -e
[[ $rc -eq 2 ]] || { echo "expected wrong parent tuple exit 2, got $rc" >&2; exit 1; }
echo "ok: wrong ordered tuple denied"
set +e
(cd "$repo" && bash "$VERIFIER" "$base" "$head" "$(printf '0%.0s' {1..40})" >/dev/null 2>&1); rc=$?
set -e
[[ $rc -eq 2 ]] || { echo "expected wrong tree exit 2, got $rc" >&2; exit 1; }
echo "ok: wrong frozen tree denied"
git -C "$repo" checkout -q --detach "$head"
set +e
(cd "$repo" && bash "$VERIFIER" "$base" "$head" "$tree" >/dev/null 2>&1); rc=$?
set -e
[[ $rc -eq 2 ]] || { echo "expected single-parent checkout exit 2, got $rc" >&2; exit 1; }
echo "ok: non-merge checkout denied"
set +e
(cd "$repo" && bash "$VERIFIER" deadbeef "$head" "$tree" >/dev/null 2>&1); rc=$?
set -e
[[ $rc -eq 64 ]] || { echo "expected malformed SHA input exit 64, got $rc" >&2; exit 1; }
echo "ok: abbreviated SHA denied"
