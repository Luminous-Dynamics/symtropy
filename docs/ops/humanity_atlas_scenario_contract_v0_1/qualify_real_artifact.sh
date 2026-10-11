#!/usr/bin/env bash
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Capture once, adapt once, verify those exact bytes in the standalone consumer.
# Required environment:
#   LUMINOUS_MONOREPO_ROOT=/path/to/luminous-dynamics
#   SOL_ATLAS_ROOT=/path/to/sol-atlas-leptos
# Optional:
#   ATLAS_QUALIFICATION_DIR=/absolute/output/path YEARS=150 SEED=42

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
: "${LUMINOUS_MONOREPO_ROOT:?Set LUMINOUS_MONOREPO_ROOT to the canonical monorepo checkout}"
: "${SOL_ATLAS_ROOT:?Set SOL_ATLAS_ROOT to the standalone Sol-Atlas checkout}"

MONOREPO_ROOT="$(cd -- "$LUMINOUS_MONOREPO_ROOT" && pwd)"
SOL_ATLAS_ROOT="$(cd -- "$SOL_ATLAS_ROOT" && pwd)"
SYMTROPY_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
YEARS="${YEARS:-150}"
SEED="${SEED:-42}"
OUT_DIR="${ATLAS_QUALIFICATION_DIR:-$PWD/.humanity-atlas-qualification}"

case "$YEARS" in
  ''|*[!0-9]*) echo "YEARS must be a positive integer" >&2; exit 2 ;;
esac
case "$SEED" in
  ''|*[!0-9]*) echo "SEED must be a non-negative integer" >&2; exit 2 ;;
esac
if (( YEARS < 1 || YEARS > 100000 )); then
  echo "YEARS must be in 1..=100000" >&2
  exit 2
fi

mkdir -p -- "$OUT_DIR"
OUT_DIR="$(cd -- "$OUT_DIR" && pwd)"
case "$OUT_DIR/" in
  "$MONOREPO_ROOT/"*|"$SOL_ATLAS_ROOT/"*|"$SYMTROPY_ROOT/"*)
    echo "qualification output directory must be outside both source checkouts" >&2
    exit 2
    ;;
esac
for checkout in "$MONOREPO_ROOT" "$SOL_ATLAS_ROOT" "$SYMTROPY_ROOT"; do
  if [[ -n "$(git -C "$checkout" status --porcelain --untracked-files=all)" ]]; then
    echo "source checkout is dirty; refusing to bind a build to HEAD only: $checkout" >&2
    exit 2
  fi
done
SOURCE_ARTIFACT="$OUT_DIR/simulator-output.json"
RUN_ENVELOPE="$OUT_DIR/run-envelope.v0.1.json"
VALIDATION_REPORT="$OUT_DIR/sol-atlas-validation.json"

for path in "$MONOREPO_ROOT/mycelix-multiworld-sim/Cargo.toml" "$SOL_ATLAS_ROOT/sol-atlas-core/Cargo.toml"; do
  if [[ ! -f "$path" ]]; then
    echo "required manifest not found: $path" >&2
    exit 2
  fi
done

git -C "$MONOREPO_ROOT" rev-parse HEAD > "$OUT_DIR/monorepo-commit.txt"
git -C "$SOL_ATLAS_ROOT" rev-parse HEAD > "$OUT_DIR/sol-atlas-commit.txt"
git -C "$SYMTROPY_ROOT" rev-parse HEAD > "$OUT_DIR/symtropy-commit.txt"
rustc --version > "$OUT_DIR/rustc-version.txt"
cargo --version > "$OUT_DIR/cargo-version.txt"
python3 --version > "$OUT_DIR/python-version.txt"

echo "[1/4] Capturing one actual simulator run (stdout is the immutable source artifact)." >&2
if ! (
  cd -- "$MONOREPO_ROOT"
  cargo run --release \
    --manifest-path "$MONOREPO_ROOT/mycelix-multiworld-sim/Cargo.toml" \
    --bin luminous_sim -- --years "$YEARS" --seed "$SEED" \
    --format humanity-atlas-json
) > "$SOURCE_ARTIFACT" 2> "$OUT_DIR/simulator-run.log"; then
  cat "$OUT_DIR/simulator-run.log" >&2
  exit 1
fi

sha256sum "$SOURCE_ARTIFACT" > "$SOURCE_ARTIFACT.sha256"

echo "[2/4] Adapting that exact artifact into Humanity Atlas run v0.1." >&2
if ! python3 "$SCRIPT_DIR/adapt_multiworld_output.py" "$SOURCE_ARTIFACT" --output "$RUN_ENVELOPE" 2> "$OUT_DIR/adapter.log"; then
  cat "$OUT_DIR/adapter.log" >&2
  exit 1
fi
sha256sum "$RUN_ENVELOPE" > "$RUN_ENVELOPE.sha256"

echo "[3/4] Running standalone Sol-Atlas core tests." >&2
(
  cd -- "$SOL_ATLAS_ROOT"
  cargo test --manifest-path "$SOL_ATLAS_ROOT/sol-atlas-core/Cargo.toml" --lib
) > "$OUT_DIR/sol-atlas-core-tests.log" 2>&1 || {
  cat "$OUT_DIR/sol-atlas-core-tests.log" >&2
  exit 1
}

echo "[4/4] Verifying the envelope against the same source bytes in the independent importer." >&2
if ! (
  cd -- "$SOL_ATLAS_ROOT"
  cargo run --manifest-path "$SOL_ATLAS_ROOT/sol-atlas-core/Cargo.toml" \
    --example validate_humanity_atlas_artifact -- \
    "$RUN_ENVELOPE" "$SOURCE_ARTIFACT" source-simulator-output
) > "$VALIDATION_REPORT" 2> "$OUT_DIR/sol-atlas-validation.log"; then
  cat "$OUT_DIR/sol-atlas-validation.log" >&2
  exit 1
fi

sha256sum "$VALIDATION_REPORT" > "$VALIDATION_REPORT.sha256"

python3 - "$OUT_DIR/qualification-metadata.json" "$SOURCE_ARTIFACT" "$RUN_ENVELOPE" "$VALIDATION_REPORT" "$YEARS" "$SEED" <<'PY'
import hashlib, json, os, subprocess, sys
metadata_path, source_path, envelope_path, report_path, years, seed = sys.argv[1:]
def sha(path):
    h = hashlib.sha256()
    with open(path, "rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()
def text(path):
    with open(path, encoding="utf-8") as stream:
        return stream.read().strip()
with open(report_path, encoding="utf-8") as stream:
    report = json.load(stream)
metadata = {
    "record_type": "humanity-atlas-structural-qualification",
    "status": "pass" if report.get("qualification") == "structural_contract_pass" else "fail",
    "qualification_scope": "contract and artifact integrity only; not scientific validation",
    "years": int(years),
    "seed": int(seed),
    "monorepo_commit": text(os.path.join(os.path.dirname(metadata_path), "monorepo-commit.txt")),
    "sol_atlas_commit": text(os.path.join(os.path.dirname(metadata_path), "sol-atlas-commit.txt")),
    "symtropy_commit": text(os.path.join(os.path.dirname(metadata_path), "symtropy-commit.txt")),
    "source_artifact_sha256": sha(source_path),
    "run_envelope_sha256": sha(envelope_path),
    "validation_report_sha256": sha(report_path),
    "validation_report": report,
}
with open(metadata_path, "w", encoding="utf-8") as stream:
    json.dump(metadata, stream, indent=2, sort_keys=True)
    stream.write("\n")
if metadata["status"] != "pass":
    raise SystemExit("consumer report did not declare structural_contract_pass")
PY

sha256sum "$OUT_DIR/qualification-metadata.json" > "$OUT_DIR/qualification-metadata.json.sha256"
echo "PASS: structural artifact qualification recorded at $OUT_DIR/qualification-metadata.json" >&2
echo "Note: this is not empirical calibration, scientific validation, or publisher authentication." >&2
