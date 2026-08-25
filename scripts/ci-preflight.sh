#!/usr/bin/env bash
set -euo pipefail

mode="${1:-fast}"

case "$mode" in
  fast|--fast) ;;
  full|--full) ;;
  *)
    echo "usage: bash scripts/ci-preflight.sh [fast|full]" >&2
    exit 2
    ;;
esac

echo "== toolchain =="
rustup show active-toolchain
rustc --version --verbose
cargo --version
rustfmt --version
cargo clippy --version

echo "== format =="
cargo fmt --all -- --check

echo "== workspace invariants =="
bash scripts/check-workspace.sh

echo "== license invariants =="
bash scripts/check-licenses.sh

echo "== physics clippy =="
cargo clippy \
  -p symtropy-math \
  -p symtropy-physics \
  -p symtropy-consciousness-physics \
  --all-targets \
  --all-features \
  -- -D warnings

echo "== physics tests =="
cargo test \
  -p symtropy-math \
  -p symtropy-physics \
  -p symtropy-consciousness-physics \
  --lib

if [[ "$mode" == "full" || "$mode" == "--full" ]]; then
  echo "== full workspace clippy =="
  cargo clippy --workspace --all-targets --all-features -- -D warnings

  echo "== full workspace check =="
  cargo check --workspace --all-targets
fi

echo "PASS: Symtropy CI preflight ($mode)"
