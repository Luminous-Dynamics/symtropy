#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Reusable local/CI preflight. Fast mode runs deterministic repository gates
# plus targeted physics validation; full mode adds workspace-wide Clippy/check.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

mode="${1:-fast}"
case "$mode" in
  fast|--fast) mode="fast" ;;
  full|--full) mode="full" ;;
  *)
    echo "usage: bash scripts/ci-preflight.sh [fast|full]" >&2
    exit 2
    ;;
esac

printf '== toolchain provenance ==\n'
printf 'repo: %s\n' "$ROOT"
printf 'head: %s\n' "$(git rev-parse HEAD 2>/dev/null || printf unknown)"
if command -v rustup >/dev/null 2>&1; then
  rustup show active-toolchain
else
  echo 'rustup: not present (toolchain supplied externally)'
fi
rustc --version --verbose
cargo --version
rustfmt --version
cargo clippy --version
python3 --version

printf '\n== format ==\n'
cargo fmt --all -- --check

printf '\n== workspace invariants ==\n'
bash scripts/check-workspace.sh

printf '\n== license invariants ==\n'
bash scripts/check-licenses.sh

printf '\n== physics clippy ==\n'
cargo clippy --locked \
  -p symtropy-math \
  -p symtropy-physics \
  -p symtropy-consciousness-physics \
  --all-targets \
  --all-features \
  -- -D warnings

printf '\n== physics tests ==\n'
cargo test --locked \
  -p symtropy-math \
  -p symtropy-physics \
  -p symtropy-consciousness-physics \
  --lib

if [[ "$mode" == "full" ]]; then
  printf '\n== full workspace clippy ==\n'
  cargo clippy --locked --workspace --all-targets --all-features -- -D warnings

  printf '\n== full workspace check ==\n'
  cargo check --locked --workspace --all-targets
fi

printf '\nPASS: Symtropy CI preflight (%s)\n' "$mode"
