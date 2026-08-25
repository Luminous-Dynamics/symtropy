#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Workspace invariant checks.
#
# Rewritten 2026-07-28. The previous version asserted a hardcoded list of 14
# expected member crates and a hardcoded `Bevy 0.18.1`. Both had rotted: the
# workspace has 60 members, and Bevy has been on 0.19.0 since the dual-bevy
# cleanup — so this script failed on every run and, being wired into no CI,
# failed silently forever.
#
# The fix is to assert PROPERTIES that stay true as the workspace grows,
# rather than an enumeration that must be hand-maintained:
#
#   1. the workspace resolves at all
#   2. exactly one Bevy version in the lockfile, matching the root manifest
#      (this is the invariant the old `!= "0.18.1"` check was a proxy for —
#      the dual-bevy/dual-wgpu duplication problem)
#   3. no absolute machine-specific path dependencies
#   4. no `// placeholder` files (patch-integration debris — see
#      archive/placeholder-artifacts-2026-07-28/README.md)

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

fail=0
note() { echo "FAIL: $*"; fail=1; }

# --- 1. Workspace resolves -------------------------------------------------
if ! metadata=$(cargo metadata --no-deps --format-version 1 2>&1); then
  note "cargo metadata could not resolve the workspace:"
  printf '%s\n' "$metadata" | head -20
  echo "Hint: a member crate with no build target produces this. Compare"
  echo "      symthaea/scripts/check-workspace-targets.sh, same class of break."
  exit 1
fi
member_count=$(python3 -c '
import json, sys
print(len(set(json.load(sys.stdin)["workspace_members"])))
' <<<"$metadata")

# --- 2. Exactly one Bevy version, matching the root manifest ---------------
declared=$(awk -F'"' '/^bevy = /{print $2; exit}' Cargo.toml)
locked=$(
  awk '
    $0 == "name = \"bevy\"" { in_bevy = 1; next }
    in_bevy && /^version = / { gsub(/version = "|"/, ""); print; in_bevy = 0 }
  ' Cargo.lock | sort -u
)
locked_count=$(printf '%s' "$locked" | grep -c . || true)
if [[ "$locked_count" -ne 1 ]]; then
  note "expected exactly one Bevy version in Cargo.lock; found ${locked_count}:"
  printf '  %s\n' $locked
  echo "  (dual-Bevy duplication inflates build time and splits the type graph)"
elif [[ "$locked" != "$declared"* ]]; then
  note "Cargo.lock has Bevy ${locked} but Cargo.toml declares \"${declared}\""
fi

# --- 3. No absolute machine-specific path deps -----------------------------
# Use only the Python standard library here so this guard is identical on
# Linux and macOS hosted runners; the previous `rg` dependency made the fast
# preflight needlessly runner-image-specific.
absolute_paths=$(
  python3 - <<'PY'
import os
import pathlib
import re

pattern = re.compile(r'path\s*=\s*"/srv/luminous-dynamics')
skip_dirs = {".git", "target", "node_modules"}

for root, dirs, files in os.walk("."):
    dirs[:] = [directory for directory in dirs if directory not in skip_dirs]
    if "Cargo.toml" not in files:
        continue
    manifest = pathlib.Path(root) / "Cargo.toml"
    for line_number, line in enumerate(
        manifest.read_text(errors="replace").splitlines(), start=1
    ):
        if pattern.search(line):
            print(f"{manifest}:{line_number}:{line.strip()}")
PY
)
if [[ -n "$absolute_paths" ]]; then
  printf '%s\n' "$absolute_paths"
  note "absolute machine-specific Cargo paths are forbidden (see above)"
fi

# --- 4. No `// placeholder` files ------------------------------------------
# Patch-integration debris: a file whose entire content is the literal text
# `// placeholder`. 57 of these were found and cleared on 2026-07-28; this
# guard exists so the class cannot silently return. Size-bounded to stay fast.
# The `if`/`fi` (rather than `[ … ] && printf`) and the trailing `true` are
# load-bearing under `set -euo pipefail`: a final loop iteration whose test is
# false makes the whole pipeline exit 1, which kills the assignment — and the
# script — silently, with no output at all.
placeholders=$(
  find . \( -name target -o -name .git -o -name node_modules \) -prune -o \
       -type f -size -20c -print 2>/dev/null |
  while IFS= read -r f; do
    if [ "$(cat "$f" 2>/dev/null)" = "// placeholder" ]; then
      printf '%s\n' "$f"
    fi
  done
  true
)
if [[ -n "$placeholders" ]]; then
  note "placeholder files found (patch-integration debris):"
  printf '  %s\n' $placeholders
  echo "  Recover real content from docs/ops/Symtropy_Document_Patch_Sets_v*/ if a"
  echo "  matching 'create mode' entry exists there; otherwise delete. See"
  echo "  archive/placeholder-artifacts-2026-07-28/README.md."
fi

if [[ "$fail" -ne 0 ]]; then
  exit 1
fi

echo "Workspace check passed (${member_count} members, Bevy ${locked}, portable paths, no placeholders)."
