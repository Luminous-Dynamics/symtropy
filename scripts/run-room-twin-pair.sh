#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Render the ROOM-TWIN-01 A/B pair, then compare decoded RGBA8 candidate-prefix
# pixels and capture receipts. This is a development audit, not blind custody.

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

for key in SYMTROPY_DEMO_CAPTURE_GPU_ADAPTER SYMTROPY_DEMO_CAPTURE_GPU_DRIVER SYMTROPY_DEMO_CAPTURE_RENDER_BACKEND; do
  if [[ -z "${!key:-}" ]]; then
    echo "$key must be set explicitly (caller-declared; not hardware-attested)" >&2
    exit 2
  fi
done

if [[ -n "$(git status --porcelain)" ]]; then
  echo "working tree is not clean; commit exact source before recording captures" >&2
  exit 2
fi

fixture="benchmarks/render/ROOM-TWIN-01.json"
fixture_sha256="$(hash_file "$fixture")"
profile="low-power-720p"
width=1280
height=720
capture_times="1.5,4.0,7.0"
output_root="${SYMTROPY_ROOM_TWIN_OUTPUT_DIR:-target/room-twin}"
for variant_dir in "$output_root/world-a" "$output_root/world-b"; do
  if [[ -e "$variant_dir" ]]; then
    echo "refusing to reuse existing output directory: $variant_dir" >&2
    echo "move or remove stale artifacts explicitly before a new run" >&2
    exit 2
  fi
done
mkdir -p "$output_root/world-a" "$output_root/world-b"

settings="$output_root/render-settings.json"
cat > "$settings" <<EOF
{"camera_id":"room-twin-camera-v1","capture_times_secs":[1.5,4.0,7.0],"height_px":720,"profile":"low-power-720p","renderer":"bevy-0.19.0","shadows_enabled":false,"width_px":1280,"waypoints":[[0.0,0.0,1.65,-7.0],[4.5,0.0,1.65,-7.0],[5.7,0.0,1.65,2.0],[6.4,5.2,1.65,2.0],[6.8,5.2,1.65,2.0]]}
EOF
settings_sha256="$(hash_file "$settings")"

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
if [[ "$bevy_version" != "0.19.0" ]]; then
  echo "ROOM-TWIN-01 expects Bevy 0.19.0, found ${bevy_version:-unknown}" >&2
  exit 2
fi

export SYMTROPY_ROOM_TWIN_PROFILE="$profile"
export SYMTROPY_DEMO_CAPTURE_TIMES="$capture_times"
export SYMTROPY_DEMO_CAPTURE_EXIT_AT="8.5"
export SYMTROPY_DEMO_CAPTURE_COMMIT="$(git rev-parse HEAD)"
export SYMTROPY_DEMO_CAPTURE_FIXTURE_ID="ROOM-TWIN-01"
export SYMTROPY_DEMO_CAPTURE_FIXTURE_SHA256="$fixture_sha256"
export SYMTROPY_DEMO_CAPTURE_SIMULATION_SEED="room-twin-no-rng-v1"
export SYMTROPY_DEMO_CAPTURE_SIMULATION_TICK="camera-rail-v1"
export SYMTROPY_DEMO_CAPTURE_QUALITY_PROFILE="$profile"
export SYMTROPY_DEMO_CAPTURE_CAMERA_ID="room-twin-camera-v1"
export SYMTROPY_DEMO_CAPTURE_RENDERER_VERSION="bevy-$bevy_version"
export SYMTROPY_DEMO_CAPTURE_RENDER_SETTINGS_SHA256="$settings_sha256"
export SYMTROPY_DEMO_CAPTURE_GPU_ADAPTER
export SYMTROPY_DEMO_CAPTURE_GPU_DRIVER
export SYMTROPY_DEMO_CAPTURE_RENDER_BACKEND
export SYMTROPY_DEMO_CAPTURE_WIDTH_PX="$width"
export SYMTROPY_DEMO_CAPTURE_HEIGHT_PX="$height"
export SYMTROPY_DEMO_CAPTURE_OS="$(uname -s)"
export SYMTROPY_DEMO_CAPTURE_RUSTC_VERSION="$(rustc --version)"

for variant in A B; do
  case "$variant" in
    A) out="$output_root/world-a" ;;
    B) out="$output_root/world-b" ;;
  esac
  echo "Rendering ROOM-TWIN-01 variant $variant into $out"
  export SYMTROPY_DEMO_CAPTURE_DIR="$out"
  export SYMTROPY_ROOM_TWIN_VARIANT="$variant"
  cargo run --release -p symtropy-render-benchmark --bin room-twin
  receipt="$out/room-twin.capture-receipt.json"
  if [[ ! -s "$receipt" ]]; then
    echo "capture receipt missing: $receipt" >&2
    exit 1
  fi
done

unset SYMTROPY_ROOM_TWIN_VARIANT
unset SYMTROPY_DEMO_CAPTURE_DIR
cargo run --release -p symtropy-render-benchmark --bin room-twin-pair-audit -- \
  "$output_root/world-a" "$output_root/world-b"
