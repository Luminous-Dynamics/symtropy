#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Local qualification gate for the stacked Causal Universe Fabric v0.1-v0.7.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

WORLD_MANIFEST="crates/domains/symtropy-world/Cargo.toml"
PRIVATE_MULTIWORLD="../mycelix-multiworld-sim"

printf 'CUF qualification head: %s\n' "$(git rev-parse HEAD)"
printf 'Rust:  %s\n' "$(rustc --version)"
printf 'Cargo: %s\n' "$(cargo --version)"

if [[ ! -f "$WORLD_MANIFEST" ]]; then
    printf 'ERROR: missing %s\n' "$WORLD_MANIFEST" >&2
    exit 1
fi

if [[ ! -d "$PRIVATE_MULTIWORLD" ]]; then
    cat >&2 <<'EOF'
ERROR: ../mycelix-multiworld-sim is absent.
CUF v0.2-v0.7 qualification requires the full/private Luminous Dynamics
monorepo layout because symtropy-world depends on that sibling crate.
No partial "green" result will be reported.
EOF
    exit 1
fi

printf '\n== Formatting ==\n'
cargo fmt --all -- --check
cargo fmt --manifest-path "$WORLD_MANIFEST" -- --check

printf '\n== Core evidence contracts ==\n'
cargo test -p symtropy-sim-contracts
cargo clippy -p symtropy-sim-contracts --all-targets -- -D warnings

printf '\n== Basin + LifeSim authority surfaces ==\n'
cargo test -p symtropy-lifesim-core -p symtropy-basin
cargo clippy -p symtropy-lifesim-core -p symtropy-basin --all-targets -- -D warnings

printf '\n== World CUF v0.2-v0.7 ==\n'
cargo test --manifest-path "$WORLD_MANIFEST"
cargo clippy --manifest-path "$WORLD_MANIFEST" --all-targets -- -D warnings

printf '\n== Repository invariants ==\n'
bash scripts/check-workspace.sh
bash scripts/check-licenses.sh

printf '\nPASS: CUF v0.1-v0.7 local qualification gates\n'
printf 'Qualified head: %s\n' "$(git rev-parse HEAD)"
