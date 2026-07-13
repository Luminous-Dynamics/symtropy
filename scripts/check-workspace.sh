#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

expected=(
  symtropy-bevy-core
  symtropy-bevy-scene
  symtropy-cognitive-bridge
  symtropy-core
  symtropy-core-stable
  symtropy-devconsole
  symtropy-fluid
  symtropy-holochain-relay
  symtropy-math
  symtropy-mesh
  symtropy-net-core
  symtropy-physics
  symtropy-render-bridge
  symtropy-soft
)

metadata=$(cargo metadata --locked --no-deps --format-version 1)
actual=$(
  python3 -c '
import json
import sys

metadata = json.load(sys.stdin)
members = set(metadata["workspace_members"])
names = sorted(
    package["name"]
    for package in metadata["packages"]
    if package["id"] in members
)
print("\n".join(names))
' <<<"$metadata"
)
expected_sorted=$(printf '%s\n' "${expected[@]}" | sort)

if [[ "$actual" != "$expected_sorted" ]]; then
  echo "Workspace membership drift detected."
  diff -u <(printf '%s\n' "$expected_sorted") <(printf '%s\n' "$actual") || true
  exit 1
fi

if rg -n 'path\s*=\s*"/srv/luminous-dynamics' \
  Cargo.toml launcher/Cargo.toml crates/*/Cargo.toml; then
  echo "Absolute machine-specific Cargo paths are forbidden."
  exit 1
fi

bevy_versions=$(
  awk '
    $0 == "name = \"bevy\"" { in_bevy = 1; next }
    in_bevy && /^version = / {
      gsub(/version = "|"/, "")
      print
      in_bevy = 0
    }
  ' Cargo.lock | sort -u
)
if [[ "$bevy_versions" != "0.18.1" ]]; then
  echo "Expected exactly Bevy 0.18.1 in Cargo.lock; found:"
  printf '%s\n' "$bevy_versions"
  exit 1
fi

echo "Workspace check passed (${#expected[@]} members, Bevy 0.18.1, portable paths)."
