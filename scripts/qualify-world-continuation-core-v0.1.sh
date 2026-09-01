#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

if [[ -n "$(git status --porcelain --untracked-files=all)" ]]; then
  echo "ERROR: qualification requires a clean worktree" >&2
  git status --short >&2
  exit 1
fi

BEFORE_TREE="$(git rev-parse HEAD^{tree})"
TMP="$(mktemp -d /tmp/symtropy-continuation-core-v0.1-XXXXXX)"
trap 'rm -rf "$TMP"' EXIT

cp -R crates/core/symtropy-sim-contracts/. "$TMP/"
rm -f "$TMP/Cargo.lock"

if grep -Eq 'path[[:space:]]*=' "$TMP/Cargo.toml"; then
  echo "ERROR: symtropy-sim-contracts gained a path dependency; Tier A boundary must be reviewed" >&2
  exit 1
fi

(
  cd "$TMP"
  cargo fmt --all -- --check
  cargo test --all-targets
  cargo clippy --all-targets -- -D warnings
)

AFTER_TREE="$(git rev-parse HEAD^{tree})"
if [[ "$BEFORE_TREE" != "$AFTER_TREE" ]]; then
  echo "ERROR: Git tree identity changed during continuation-core qualification" >&2
  exit 1
fi

if [[ -n "$(git status --porcelain --untracked-files=all)" ]]; then
  echo "ERROR: qualification dirtied the source worktree" >&2
  git status --short >&2
  exit 1
fi

echo "PASS: World Continuation Core v0.1 dependency-light qualification"
