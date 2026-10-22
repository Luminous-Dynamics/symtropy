#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Execute a same-context object-count sweep and summarize only qualified reports.

set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if [[ -n "$(git status --porcelain)" ]]; then
  echo "working tree is not clean; commit the exact benchmark source before starting a qualified sweep" >&2
  exit 2
fi

profile="${SYMTROPY_RENDER_BENCH_PROFILE:-balanced-1080p}"
case "$profile" in
  quality-1080p|balanced-1080p|low-power-720p) ;;
  *)
    echo "unsupported profile '$profile'; choose quality-1080p, balanced-1080p, or low-power-720p" >&2
    exit 2
    ;;
esac

shadows="${SYMTROPY_RENDER_BENCH_SHADOWS:-}"
if [[ -z "$shadows" ]]; then
  case "$profile" in
    low-power-720p) shadows=off ;;
    *) shadows=on ;;
  esac
fi
case "$shadows" in
  on|true|1) shadows=on ;;
  off|false|0) shadows=off ;;
  *)
    echo "SYMTROPY_RENDER_BENCH_SHADOWS must be on/off, true/false, or 1/0" >&2
    exit 2
    ;;
esac

for key in SYMTROPY_DEMO_CAPTURE_GPU_ADAPTER SYMTROPY_DEMO_CAPTURE_GPU_DRIVER SYMTROPY_DEMO_CAPTURE_RENDER_BACKEND; do
  if [[ -z "${!key:-}" ]]; then
    echo "$key must be set explicitly (caller-declared; not hardware-attested)" >&2
    exit 2
  fi
done

base_dir="${SYMTROPY_RENDER_SWEEP_DIR:-target/render-sweeps/$profile-shadows-$shadows}"
mkdir -p "$base_dir"
summary_path="$base_dir/summary.json"
rm -f "$summary_path"
counts=(64 144 256 512 1024 2048 4096 8192 10000)
reports=()

for objects in "${counts[@]}"; do
  export SYMTROPY_RENDER_BENCH_PROFILE="$profile"
  export SYMTROPY_RENDER_BENCH_OBJECTS="$objects"
  export SYMTROPY_RENDER_BENCH_SHADOWS="$shadows"
  export SYMTROPY_DEMO_CAPTURE_DIR="$base_dir/n$objects"
  echo
  echo "=== SCENE-LIGHT-01 scaling point: $objects objects ==="
  bash scripts/run-render-benchmark.sh
  report="$SYMTROPY_DEMO_CAPTURE_DIR/frame-time.json"
  if [[ ! -s "$report" ]]; then
    echo "required report missing: $report" >&2
    exit 1
  fi
  reports+=("$report")
done

cargo run --release -p symtropy-render-benchmark --bin symtropy-render-sweep-report -- \
  "$summary_path" "${reports[@]}"

echo
echo "Scaling summary: $summary_path"
echo "Each point retains its own PNG captures, frame-time report, and metadata sidecars under $base_dir."
