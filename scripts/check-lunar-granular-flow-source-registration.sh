#!/usr/bin/env bash
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"
registrar="scripts/register-lunar-granular-flow-source.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

printf 'synthetic bytes; not source data\n' > "$tmp/metrics_frame_level.csv"
ln -s "$tmp/metrics_frame_level.csv" "$tmp/symlink.csv"

if bash "$registrar" WYLEGALA-UHL-2026-ROTATING-DRUM frame-level-metrics "$tmp/metrics_frame_level.csv"; then
  printf 'source registration test: FAIL: accepted bytes with wrong publisher MD5\n' >&2
  exit 1
fi

if bash "$registrar" WYLEGALA-UHL-2026-ROTATING-DRUM frame-level-metrics "$tmp/symlink.csv"; then
  printf 'source registration test: FAIL: accepted symlinked source path\n' >&2
  exit 1
fi

if bash "$registrar" UNKNOWN-SOURCE UNKNOWN-FILE "$tmp/metrics_frame_level.csv"; then
  printf 'source registration test: FAIL: accepted unknown source identity\n' >&2
  exit 1
fi

printf 'source registration test: PASS (tampered, symlink and unknown-source negative controls)\n'
