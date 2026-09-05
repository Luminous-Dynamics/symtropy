#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

INITIAL_HEAD="$(git rev-parse HEAD)"
INITIAL_LOCK_BLOB="$(git hash-object Cargo.lock)"

printf '=== GAME-STATE CANONICAL V2 QUALIFICATION ===\n'
printf 'repo:   %s\n' "$ROOT"
printf 'head:   %s\n' "$INITIAL_HEAD"
printf 'branch: %s\n' "$(git branch --show-current)"
printf 'lock:   %s\n' "$INITIAL_LOCK_BLOB"

if [[ -n "$(git status --porcelain --untracked-files=all)" ]]; then
  echo 'FAIL: worktree must be clean before qualification' >&2
  git status --short >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo 'FAIL: python3 is required for the independent canonical-vector oracle' >&2
  exit 1
fi

printf '\n=== INDEPENDENT CANONICAL VECTOR ORACLE ===\n'
python3 scripts/verify-canonical-event-v2-vectors.py

printf '\n=== FORMAT ===\n'
cargo fmt -p symtropy-game-state -- --check

printf '\n=== TEST ===\n'
cargo test --locked -p symtropy-game-state --all-targets

printf '\n=== CLIPPY ===\n'
cargo clippy --locked -p symtropy-game-state --all-targets -- -D warnings

printf '\n=== CHECK ===\n'
cargo check --locked -p symtropy-game-state --all-targets

FINAL_HEAD="$(git rev-parse HEAD)"
FINAL_LOCK_BLOB="$(git hash-object Cargo.lock)"

if [[ "$FINAL_HEAD" != "$INITIAL_HEAD" ]]; then
  echo "FAIL: HEAD changed during qualification: $INITIAL_HEAD -> $FINAL_HEAD" >&2
  exit 1
fi

if [[ "$FINAL_LOCK_BLOB" != "$INITIAL_LOCK_BLOB" ]]; then
  echo "FAIL: Cargo.lock identity changed: $INITIAL_LOCK_BLOB -> $FINAL_LOCK_BLOB" >&2
  exit 1
fi

if [[ -n "$(git status --porcelain --untracked-files=all)" ]]; then
  echo 'FAIL: qualification dirtied the worktree' >&2
  git status --short >&2
  exit 1
fi

printf '\nPASS: symtropy-game-state canonical v2 qualification gate\n'
