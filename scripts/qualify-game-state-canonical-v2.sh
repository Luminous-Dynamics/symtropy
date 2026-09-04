#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

printf '=== GAME-STATE CANONICAL V2 QUALIFICATION ===\n'
printf 'repo:   %s\n' "$ROOT"
printf 'head:   %s\n' "$(git rev-parse HEAD)"
printf 'branch: %s\n' "$(git branch --show-current)"

if ! git diff --quiet -- Cargo.lock; then
  echo 'FAIL: Cargo.lock is already modified before qualification' >&2
  exit 1
fi

printf '\n=== FORMAT ===\n'
cargo fmt -p symtropy-game-state -- --check

printf '\n=== TEST ===\n'
cargo test -p symtropy-game-state --all-targets

printf '\n=== CLIPPY ===\n'
cargo clippy -p symtropy-game-state --all-targets -- -D warnings

printf '\n=== CHECK ===\n'
cargo check -p symtropy-game-state --all-targets

if ! git diff --quiet -- Cargo.lock; then
  echo 'FAIL: qualification mutated Cargo.lock' >&2
  git diff -- Cargo.lock >&2
  exit 1
fi

printf '\nPASS: symtropy-game-state canonical v2 qualification gate\n'
