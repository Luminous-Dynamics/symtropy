#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Launch SCENE-LIGHT-01 with exact, receipt-oriented metadata.
# Required caller inputs: GPU adapter, driver, and render backend labels.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

hash_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{ print $1 }'
  else
    shasum -a 256 "$1" | awk '{ print $1 }'
  fi
}

profile="${SYMTROPY_RENDER_BENCH_PROFILE:-balanced-1080p}"
case "$profile" in
  quality-1080p) width=1920; height=1080; default_objects=256; default_shadows=on ;;
  balanced-1080p) width=1920; height=1080; default_objects=144; default_shadows=on ;;
  low-power-720p) width=1280; height=720; default_objects=64; default_shadows=off ;;
  *)
    echo "unsupported profile '$profile'; choose quality-1080p, balanced-1080p, or low-power-720p" >&2
    exit 2
    ;;
esac

objects="${SYMTROPY_RENDER_BENCH_OBJECTS:-$default_objects}"
shadows="${SYMTROPY_RENDER_BENCH_SHADOWS:-$default_shadows}"
if [[ ! "$objects" =~ ^[0-9]+$ ]] || (( objects < 1 || objects > 10000 )); then
  echo "SYMTROPY_RENDER_BENCH_OBJECTS must be an integer in 1..=10000" >&2
  exit 2
fi
case "$shadows" in
  on|true|1) shadows_json=true; shadows=on ;;
  off|false|0) shadows_json=false; shadows=off ;;
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

if [[ -n "$(git status --porcelain)" ]]; then
  echo "working tree is not clean; commit the exact benchmark source before recording a qualified capture" >&2
  exit 2
fi

capture_dir="${SYMTROPY_DEMO_CAPTURE_DIR:-target/render-captures/$profile-n$objects-shadows-$shadows}"
mkdir -p "$capture_dir"
metrics_output="$capture_dir/frame-time.json"
rm -f "$metrics_output"

fixture_path="benchmarks/render/SCENE-LIGHT-01.json"
fixture_sha256="$(hash_file "$fixture_path")"
settings_path="$capture_dir/render-settings.json"
printf '{"camera_id":"auto-grid-camera-v1","height_px":%s,"object_count":%s,"profile":"%s","shadows_enabled":%s,"width_px":%s}\n' \
  "$height" "$objects" "$profile" "$shadows_json" "$width" > "$settings_path"
settings_sha256="$(hash_file "$settings_path")"
bevy_version="$(
  awk '
    $0 == "name = \"bevy\"" { found = 1; next }
    found && /^version = / {
      gsub(/version = \"|\"/, "")
      print
      exit
    }
  ' Cargo.lock
)"
if [[ -z "$bevy_version" ]]; then
  echo "could not resolve Bevy version from Cargo.lock" >&2
  exit 2
fi

export SYMTROPY_RENDER_BENCH_PROFILE="$profile"
export SYMTROPY_RENDER_BENCH_OBJECTS="$objects"
export SYMTROPY_RENDER_BENCH_SHADOWS="$shadows"
export SYMTROPY_RENDER_BENCH_METRICS_OUTPUT="$metrics_output"
export SYMTROPY_DEMO_CAPTURE_DIR="$capture_dir"
export SYMTROPY_DEMO_CAPTURE_TIMES="${SYMTROPY_DEMO_CAPTURE_TIMES:-1.5,4.0,7.0}"
export SYMTROPY_DEMO_CAPTURE_EXIT_AT="${SYMTROPY_DEMO_CAPTURE_EXIT_AT:-8.5}"
export SYMTROPY_DEMO_CAPTURE_COMMIT="$(git rev-parse HEAD)"
export SYMTROPY_DEMO_CAPTURE_FIXTURE_ID="SCENE-LIGHT-01"
export SYMTROPY_DEMO_CAPTURE_FIXTURE_SHA256="$fixture_sha256"
export SYMTROPY_DEMO_CAPTURE_SIMULATION_SEED="static-no-rng"
export SYMTROPY_DEMO_CAPTURE_SIMULATION_TICK="static-frame-0"
export SYMTROPY_DEMO_CAPTURE_QUALITY_PROFILE="$profile-n$objects-shadows-$shadows"
export SYMTROPY_DEMO_CAPTURE_CAMERA_ID="auto-grid-camera-v1"
export SYMTROPY_DEMO_CAPTURE_RENDERER_VERSION="bevy-$bevy_version"
export SYMTROPY_DEMO_CAPTURE_RENDER_SETTINGS_SHA256="$settings_sha256"
export SYMTROPY_DEMO_CAPTURE_WIDTH_PX="$width"
export SYMTROPY_DEMO_CAPTURE_HEIGHT_PX="$height"
export SYMTROPY_DEMO_CAPTURE_OS="$(uname -s)"
export SYMTROPY_DEMO_CAPTURE_RUSTC_VERSION="$(rustc --version)"

echo "Fixture: SCENE-LIGHT-01"
echo "Profile: $profile (objects=$objects, shadows=$shadows, resolution=${width}x${height})"
echo "Bevy: $bevy_version"
echo "GPU adapter: $SYMTROPY_DEMO_CAPTURE_GPU_ADAPTER"
echo "Render backend: $SYMTROPY_DEMO_CAPTURE_RENDER_BACKEND"
echo "Output: $capture_dir"

cargo run --release -p symtropy-render-benchmark

if [[ ! -s "$metrics_output" ]]; then
  echo "frame-time report missing or empty: $metrics_output" >&2
  exit 1
fi
if grep -q '"measurement_status": "insufficient_samples"' "$metrics_output"; then
  echo "frame-time report has fewer than the minimum 30 samples; benchmark is not qualified" >&2
  exit 1
fi
echo "Frame-time report: $metrics_output"
