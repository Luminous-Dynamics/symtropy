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
if [[ -n "${ATLAS_QUALIFICATION_DIR:-}" ]]; then
  # Resolve a possibly not-yet-existing destination without creating it first.
  # This keeps a rejected in-checkout path from leaving an untracked directory.
  OUT_DIR="$(python3 - "$ATLAS_QUALIFICATION_DIR" <<'PY'
import os, sys
print(os.path.realpath(sys.argv[1]))
PY
)"
else
  TEMP_ROOT="${TMPDIR:-/tmp}"
  if [[ ! -d "$TEMP_ROOT" ]]; then
    echo "TMPDIR must name an existing directory: $TEMP_ROOT" >&2
    exit 2
  fi
  TEMP_ROOT="$(cd -- "$TEMP_ROOT" && pwd -P)"
  case "$TEMP_ROOT/" in
    "$MONOREPO_ROOT/"*|"$SOL_ATLAS_ROOT/"*|"$SYMTROPY_ROOT/"*)
      echo "TMPDIR must be outside all three source checkouts" >&2
      exit 2
      ;;
  esac
  OUT_DIR="$TEMP_ROOT/humanity-atlas-qualification.pending"
fi

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

case "$OUT_DIR/" in
  "$MONOREPO_ROOT/"*|"$SOL_ATLAS_ROOT/"*|"$SYMTROPY_ROOT/"*)
    echo "qualification output directory must be outside all three source checkouts" >&2
    exit 2
    ;;
esac
if [[ -z "${ATLAS_QUALIFICATION_DIR:-}" ]]; then
  # Only create the temporary directory after validating its parent location.
  OUT_DIR="$(mktemp -d "$TEMP_ROOT/humanity-atlas-qualification.XXXXXX")"
elif [[ -e "$OUT_DIR" ]]; then
  if [[ ! -d "$OUT_DIR" ]]; then
    echo "qualification output path exists but is not a directory: $OUT_DIR" >&2
    exit 2
  fi
  if [[ -n "$(find "$OUT_DIR" -mindepth 1 -print -quit)" ]]; then
    echo "qualification output directory must be empty: $OUT_DIR" >&2
    exit 2
  fi
else
  mkdir -p -- "$OUT_DIR"
fi
OUT_DIR="$(cd -- "$OUT_DIR" && pwd -P)"
QUALIFICATION_PHASE="preflight"
SIMULATOR_TARGET_DIR=""
CORE_TARGET_DIR=""
SIMULATOR_LOCK_PATH=""
CORE_LOCK_PATH=""
SIMULATOR_LOCK_PREEXISTED=1
CORE_LOCK_PREEXISTED=1
cleanup_generated_locks_and_targets() {
  local exit_status="$?"
  if [[ "$SIMULATOR_LOCK_PREEXISTED" -eq 0 && -n "$SIMULATOR_LOCK_PATH" && -f "$SIMULATOR_LOCK_PATH" ]]; then
    rm -f -- "$SIMULATOR_LOCK_PATH"
  fi
  if [[ "$CORE_LOCK_PREEXISTED" -eq 0 && -n "$CORE_LOCK_PATH" && -f "$CORE_LOCK_PATH" ]]; then
    rm -f -- "$CORE_LOCK_PATH"
  fi
  if [[ -n "$SIMULATOR_TARGET_DIR" && -d "$SIMULATOR_TARGET_DIR" ]]; then
    rm -rf -- "$SIMULATOR_TARGET_DIR"
  fi
  if [[ -n "$CORE_TARGET_DIR" && -d "$CORE_TARGET_DIR" ]]; then
    rm -rf -- "$CORE_TARGET_DIR"
  fi
  if [[ -d "$OUT_DIR" ]]; then
    python3 - "$OUT_DIR/qualification-execution-status.json" "$exit_status" "$QUALIFICATION_PHASE" "$OUT_DIR" "$MONOREPO_ROOT" "$SOL_ATLAS_ROOT" "$SYMTROPY_ROOT" <<'PY'
import hashlib, json, os, subprocess, sys
status_path, raw_exit_code, phase, out_dir, monorepo, sol_atlas, symtropy = sys.argv[1:]
exit_code = int(raw_exit_code)
def git_head(root):
    try:
        return subprocess.run(
            ["git", "-C", root, "rev-parse", "HEAD"],
            check=True, capture_output=True, text=True,
        ).stdout.strip()
    except Exception:
        return None
def sha(path):
    h = hashlib.sha256()
    with open(path, "rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()
def maybe_text(path):
    try:
        with open(path, encoding="utf-8") as stream:
            return stream.read().strip()
    except OSError:
        return None
names = [
    "simulator-output.json",
    "run-envelope.v0.1.json",
    "sol-atlas-validation.json",
    "simulator-Cargo.lock",
    "sol-atlas-core-Cargo.lock",
    "simulator-run.log",
    "producer-consumer-roundtrip.log",
    "adapter.log",
    "python-adapter-tests.log",
    "run-schema-validation.log",
    "source-model-schema-validation.log",
    "sol-atlas-core-tests.log",
    "sol-atlas-validation.log",
    "qualification-metadata.json",
]
available = {}
for name in names:
    path = os.path.join(out_dir, name)
    if os.path.isfile(path):
        available[name] = sha(path)
metadata = {
    "record_type": "humanity-atlas-execution-status",
    "exit_code": exit_code,
    "execution_state": "completed_successfully" if exit_code == 0 else "failed",
    "qualification_state": "pass" if exit_code == 0 else "not_passed",
    "last_phase": phase,
    "repository_heads": {
        "monorepo": maybe_text(os.path.join(out_dir, "monorepo-commit.txt")) or git_head(monorepo),
        "sol_atlas": maybe_text(os.path.join(out_dir, "sol-atlas-commit.txt")) or git_head(sol_atlas),
        "symtropy": maybe_text(os.path.join(out_dir, "symtropy-commit.txt")) or git_head(symtropy),
    },
    "available_artifact_sha256": available,
    "tool_versions": {
        "rustc": maybe_text(os.path.join(out_dir, "rustc-version.txt")),
        "cargo": maybe_text(os.path.join(out_dir, "cargo-version.txt")),
        "python": maybe_text(os.path.join(out_dir, "python-version.txt")),
    },
    "scientific_validation": "not_established_by_this_execution_status_record",
}
temp_path = status_path + ".tmp"
with open(temp_path, "w", encoding="utf-8") as stream:
    json.dump(metadata, stream, indent=2, sort_keys=True)
    stream.write("\n")
os.replace(temp_path, status_path)
PY
  fi
  exit "$exit_status"
}
trap cleanup_generated_locks_and_targets EXIT
echo "Evidence directory: $OUT_DIR" >&2
for checkout in "$MONOREPO_ROOT" "$SOL_ATLAS_ROOT" "$SYMTROPY_ROOT"; do
  if [[ -n "$(git -C "$checkout" status --porcelain --untracked-files=all)" ]]; then
    echo "source checkout is dirty; refusing to bind a build to HEAD only: $checkout" >&2
    exit 2
  fi
done
SOURCE_ARTIFACT="$OUT_DIR/simulator-output.json"
RUN_ENVELOPE="$OUT_DIR/run-envelope.v0.1.json"
VALIDATION_REPORT="$OUT_DIR/sol-atlas-validation.json"
RUN_SCHEMA_SOURCE="$SCRIPT_DIR/schemas/run-v0.1.schema.json"
TREE_SCHEMA_PRODUCER="$MONOREPO_ROOT/sol-atlas-core/schemas/mycelix-multiworld-sim-tech-tree-v1.schema.json"
TREE_SCHEMA_CONSUMER="$SOL_ATLAS_ROOT/sol-atlas-core/schemas/mycelix-multiworld-sim-tech-tree-v1.schema.json"
RUN_MANIFEST_SCHEMA_PRODUCER="$MONOREPO_ROOT/sol-atlas-core/schemas/humanity-atlas-run-manifest-v1.schema.json"
RUN_MANIFEST_SCHEMA_CONSUMER="$SOL_ATLAS_ROOT/sol-atlas-core/schemas/humanity-atlas-run-manifest-v1.schema.json"
TARGET_ROOT="${TMPDIR:-/tmp}"
if [[ ! -d "$TARGET_ROOT" ]]; then
  echo "TMPDIR must name an existing directory: $TARGET_ROOT" >&2
  exit 2
fi
TARGET_ROOT="$(cd -- "$TARGET_ROOT" && pwd -P)"
case "$TARGET_ROOT/" in
  "$MONOREPO_ROOT/"*|"$SOL_ATLAS_ROOT/"*|"$SYMTROPY_ROOT/"*)
    echo "TMPDIR must be outside all three source checkouts" >&2
    exit 2
    ;;
esac
SIMULATOR_TARGET_DIR="$(mktemp -d "$TARGET_ROOT/humanity-atlas-simulator-target.XXXXXX")"
CORE_TARGET_DIR="$(mktemp -d "$TARGET_ROOT/humanity-atlas-consumer-target.XXXXXX")"
SIMULATOR_LOCK_PATH="$MONOREPO_ROOT/mycelix-multiworld-sim/Cargo.lock"
CORE_LOCK_PATH="$SOL_ATLAS_ROOT/sol-atlas-core/Cargo.lock"
if [[ -f "$SIMULATOR_LOCK_PATH" ]]; then
  SIMULATOR_LOCK_PREEXISTED=1
else
  SIMULATOR_LOCK_PREEXISTED=0
fi
if [[ -f "$CORE_LOCK_PATH" ]]; then
  CORE_LOCK_PREEXISTED=1
else
  CORE_LOCK_PREEXISTED=0
fi
for path in \
  "$MONOREPO_ROOT/mycelix-multiworld-sim/Cargo.toml" \
  "$SOL_ATLAS_ROOT/sol-atlas-core/Cargo.toml" \
  "$RUN_SCHEMA_SOURCE" "$TREE_SCHEMA_PRODUCER" "$TREE_SCHEMA_CONSUMER" \
  "$RUN_MANIFEST_SCHEMA_PRODUCER" "$RUN_MANIFEST_SCHEMA_CONSUMER"; do
  if [[ ! -f "$path" ]]; then
    echo "required manifest/schema not found: $path" >&2
    exit 2
  fi
done
if ! cmp -s -- "$TREE_SCHEMA_PRODUCER" "$TREE_SCHEMA_CONSUMER"; then
  echo "producer and standalone consumer technology-tree schemas differ byte-for-byte" >&2
  exit 1
fi
if ! cmp -s -- "$RUN_MANIFEST_SCHEMA_PRODUCER" "$RUN_MANIFEST_SCHEMA_CONSUMER"; then
  echo "producer and standalone consumer run-manifest schemas differ byte-for-byte" >&2
  exit 1
fi

# Snapshot every schema/script that participates in the qualification. Hashes
# below describe these copies, while the checkout HEADs bind their source trees.
cp -- "$SCRIPT_DIR/adapt_multiworld_output.py" "$OUT_DIR/adapt_multiworld_output.py"
cp -- "$SCRIPT_DIR/test_adapt_multiworld_output.py" "$OUT_DIR/test_adapt_multiworld_output.py"
cp -- "$RUN_SCHEMA_SOURCE" "$OUT_DIR/run-v0.1.schema.json"
cp -- "$TREE_SCHEMA_PRODUCER" "$OUT_DIR/producer-tech-tree-v1.schema.json"
cp -- "$TREE_SCHEMA_CONSUMER" "$OUT_DIR/consumer-tech-tree-v1.schema.json"
cp -- "$RUN_MANIFEST_SCHEMA_PRODUCER" "$OUT_DIR/producer-run-manifest-v1.schema.json"
cp -- "$RUN_MANIFEST_SCHEMA_CONSUMER" "$OUT_DIR/consumer-run-manifest-v1.schema.json"
if ! cmp -s -- "$RUN_SCHEMA_SOURCE" "$OUT_DIR/run-v0.1.schema.json"; then
  echo "run schema snapshot does not match its source" >&2
  exit 1
fi

git -C "$MONOREPO_ROOT" rev-parse HEAD > "$OUT_DIR/monorepo-commit.txt"
git -C "$SOL_ATLAS_ROOT" rev-parse HEAD > "$OUT_DIR/sol-atlas-commit.txt"
git -C "$SYMTROPY_ROOT" rev-parse HEAD > "$OUT_DIR/symtropy-commit.txt"
PRODUCER_REVISION="$(cat "$OUT_DIR/monorepo-commit.txt")"
rustc --version > "$OUT_DIR/rustc-version.txt"
cargo --version > "$OUT_DIR/cargo-version.txt"
python3 --version > "$OUT_DIR/python-version.txt"

QUALIFICATION_PHASE="adapter_regressions"
echo "[1/9] Running adapter regressions with JSON Schema validation required." >&2
if ! (
  cd -- "$SCRIPT_DIR"
  PYTHONDONTWRITEBYTECODE=1 HUMANITY_ATLAS_REQUIRE_JSONSCHEMA=1 \
    python3 -m unittest -v test_adapt_multiworld_output.py
) > "$OUT_DIR/python-adapter-tests.log" 2>&1; then
  cat "$OUT_DIR/python-adapter-tests.log" >&2
  exit 1
fi

QUALIFICATION_PHASE="dependency_lock_resolution"
echo "[2/9] Resolving and snapshotting simulator + standalone-consumer dependency locks." >&2
if [[ ! -f "$SIMULATOR_LOCK_PATH" ]]; then
  if ! cargo generate-lockfile \
    --manifest-path "$MONOREPO_ROOT/mycelix-multiworld-sim/Cargo.toml" \
    > "$OUT_DIR/simulator-lock-resolution.log" 2>&1; then
    cat "$OUT_DIR/simulator-lock-resolution.log" >&2
    exit 1
  fi
fi
if [[ ! -f "$CORE_LOCK_PATH" ]]; then
  if ! cargo generate-lockfile \
    --manifest-path "$SOL_ATLAS_ROOT/sol-atlas-core/Cargo.toml" \
    > "$OUT_DIR/sol-atlas-lock-resolution.log" 2>&1; then
    cat "$OUT_DIR/sol-atlas-lock-resolution.log" >&2
    exit 1
  fi
fi
cp -- "$SIMULATOR_LOCK_PATH" "$OUT_DIR/simulator-Cargo.lock"
cp -- "$CORE_LOCK_PATH" "$OUT_DIR/sol-atlas-core-Cargo.lock"
printf '%s\n' "$([[ "$SIMULATOR_LOCK_PREEXISTED" -eq 1 ]] && printf 'present_at_start' || printf 'generated_during_qualification')" \
  > "$OUT_DIR/simulator-lockfile-origin.txt"
printf '%s\n' "$([[ "$CORE_LOCK_PREEXISTED" -eq 1 ]] && printf 'present_at_start' || printf 'generated_during_qualification')" \
  > "$OUT_DIR/sol-atlas-core-lockfile-origin.txt"
SIMULATOR_LOCK_SHA256="$(sha256sum "$SIMULATOR_LOCK_PATH" | awk '{print $1}')"
CORE_LOCK_SHA256="$(sha256sum "$CORE_LOCK_PATH" | awk '{print $1}')"
sha256sum "$OUT_DIR/simulator-Cargo.lock" > "$OUT_DIR/simulator-Cargo.lock.sha256"
sha256sum "$OUT_DIR/sol-atlas-core-Cargo.lock" > "$OUT_DIR/sol-atlas-core-Cargo.lock.sha256"

QUALIFICATION_PHASE="simulator_capture_and_revision_binding"
QUALIFICATION_PHASE="producer_consumer_roundtrip_test"
echo "[3/9] Running the simulator-backed producer → core round-trip integration test." >&2
if ! (
  cd -- "$MONOREPO_ROOT"
  CARGO_TARGET_DIR="$SIMULATOR_TARGET_DIR" \
  GITHUB_SHA="$PRODUCER_REVISION" \
  GIT_COMMIT="$PRODUCER_REVISION" \
  VERGEN_GIT_SHA="$PRODUCER_REVISION" \
    cargo test --locked \
      --manifest-path "$MONOREPO_ROOT/mycelix-multiworld-sim/Cargo.toml" \
      --test humanity_atlas_roundtrip
) > "$OUT_DIR/producer-consumer-roundtrip.log" 2>&1; then
  cat "$OUT_DIR/producer-consumer-roundtrip.log" >&2
  exit 1
fi

QUALIFICATION_PHASE="simulator_capture_and_revision_binding"
echo "[4/9] Capturing one actual simulator run (stdout is the immutable source artifact)." >&2
if ! (
  cd -- "$MONOREPO_ROOT"
  # option_env! reads these at compile time. The workflow's ambient GITHUB_SHA
  # belongs to Symtropy, so override every supported producer revision variable.
  CARGO_TARGET_DIR="$SIMULATOR_TARGET_DIR" \
  GITHUB_SHA="$PRODUCER_REVISION" \
  GIT_COMMIT="$PRODUCER_REVISION" \
  VERGEN_GIT_SHA="$PRODUCER_REVISION" \
    cargo run --locked --release \
      --manifest-path "$MONOREPO_ROOT/mycelix-multiworld-sim/Cargo.toml" \
      --bin luminous_sim -- --years "$YEARS" --seed "$SEED" \
      --format humanity-atlas-json
) > "$SOURCE_ARTIFACT" 2> "$OUT_DIR/simulator-run.log"; then
  cat "$OUT_DIR/simulator-run.log" >&2
  exit 1
fi

# Prove the source run declares the producer checkout's exact immutable HEAD;
# never accept the caller workflow's SHA or an absent/malformed revision.
if ! python3 - "$SOURCE_ARTIFACT" "$PRODUCER_REVISION" <<'PY'
import json, sys
artifact_path, expected_revision = sys.argv[1:]
with open(artifact_path, encoding="utf-8") as stream:
    payload = json.load(stream)
manifest = payload.get("run_manifest") if isinstance(payload, dict) else None
actual_revision = manifest.get("source_revision") if isinstance(manifest, dict) else None
if actual_revision != expected_revision:
    raise SystemExit(
        "producer run_manifest.source_revision does not match the monorepo HEAD "
        f"(expected {expected_revision!r}, got {actual_revision!r})"
    )
print(f"producer source revision verified: {actual_revision}")
PY
then
  exit 1
fi

sha256sum "$SOURCE_ARTIFACT" > "$SOURCE_ARTIFACT.sha256"

QUALIFICATION_PHASE="adapter_execution"
echo "[5/9] Adapting that exact artifact into Humanity Atlas run v0.1." >&2
if ! python3 "$SCRIPT_DIR/adapt_multiworld_output.py" "$SOURCE_ARTIFACT" --output "$RUN_ENVELOPE" 2> "$OUT_DIR/adapter.log"; then
  cat "$OUT_DIR/adapter.log" >&2
  exit 1
fi
sha256sum "$RUN_ENVELOPE" > "$RUN_ENVELOPE.sha256"

QUALIFICATION_PHASE="run_schema_validation"
echo "[6/9] Validating the actual generated envelope against the published run schema." >&2
if ! python3 - "$OUT_DIR/run-v0.1.schema.json" "$RUN_ENVELOPE" <<'PY' > "$OUT_DIR/run-schema-validation.log" 2>&1
import json, sys
import jsonschema
schema_path, envelope_path = sys.argv[1:]
with open(schema_path, encoding="utf-8") as stream:
    schema = json.load(stream)
with open(envelope_path, encoding="utf-8") as stream:
    envelope = json.load(stream)
jsonschema.Draft202012Validator.check_schema(schema)
jsonschema.Draft202012Validator(schema).validate(envelope)
print("actual run envelope validates against run-v0.1.schema.json")
PY
then
  cat "$OUT_DIR/run-schema-validation.log" >&2
  exit 1
fi

QUALIFICATION_PHASE="source_model_schema_validation"
echo "[7/9] Validating the captured producer tree and run manifest against both versioned schemas." >&2
if ! python3 - "$OUT_DIR/producer-tech-tree-v1.schema.json" "$OUT_DIR/producer-run-manifest-v1.schema.json" "$SOURCE_ARTIFACT" <<'PY' > "$OUT_DIR/source-model-schema-validation.log" 2>&1
import json, sys
import jsonschema
tree_schema_path, manifest_schema_path, artifact_path = sys.argv[1:]
def load(path):
    with open(path, encoding="utf-8") as stream:
        return json.load(stream)
tree_schema = load(tree_schema_path)
manifest_schema = load(manifest_schema_path)
artifact = load(artifact_path)
if not isinstance(artifact, dict):
    raise SystemExit("source artifact must be a JSON object")
if "technology_tree" not in artifact or "run_manifest" not in artifact:
    raise SystemExit("source artifact must contain technology_tree and run_manifest")
for label, schema in (("technology tree", tree_schema), ("run manifest", manifest_schema)):
    jsonschema.Draft202012Validator.check_schema(schema)
    print(f"{label} schema meta-validation passed")
jsonschema.Draft202012Validator(tree_schema).validate(artifact["technology_tree"])
print("captured technology_tree validates against the producer versioned schema")
jsonschema.Draft202012Validator(manifest_schema).validate(artifact["run_manifest"])
print("captured run_manifest validates against the producer versioned schema")
PY
then
  cat "$OUT_DIR/source-model-schema-validation.log" >&2
  exit 1
fi

QUALIFICATION_PHASE="consumer_unit_tests"
echo "[8/9] Running standalone Sol-Atlas core tests with the snapshotted lock." >&2
(
  cd -- "$SOL_ATLAS_ROOT"
  CARGO_TARGET_DIR="$CORE_TARGET_DIR" \
    cargo test --locked --manifest-path "$SOL_ATLAS_ROOT/sol-atlas-core/Cargo.toml" --lib
) > "$OUT_DIR/sol-atlas-core-tests.log" 2>&1 || {
  cat "$OUT_DIR/sol-atlas-core-tests.log" >&2
  exit 1
}

QUALIFICATION_PHASE="independent_consumer_import"
echo "[9/9] Verifying the envelope against the same source bytes in the independent importer." >&2
if ! (
  cd -- "$SOL_ATLAS_ROOT"
  CARGO_TARGET_DIR="$CORE_TARGET_DIR" \
    cargo run --locked --manifest-path "$SOL_ATLAS_ROOT/sol-atlas-core/Cargo.toml" \
      --example verify_humanity_atlas_artifact -- \
    "$RUN_ENVELOPE" "$SOURCE_ARTIFACT" source-simulator-output
) > "$VALIDATION_REPORT" 2> "$OUT_DIR/sol-atlas-validation.log"; then
  cat "$OUT_DIR/sol-atlas-validation.log" >&2
  exit 1
fi

sha256sum "$VALIDATION_REPORT" > "$VALIDATION_REPORT.sha256"

QUALIFICATION_PHASE="lockfile_integrity_and_checkout_cleanliness"
if [[ "$(sha256sum "$SIMULATOR_LOCK_PATH" | awk '{print $1}')" != "$SIMULATOR_LOCK_SHA256" ]]; then
  echo "simulator Cargo.lock changed during qualification despite --locked" >&2
  exit 1
fi
if [[ "$(sha256sum "$CORE_LOCK_PATH" | awk '{print $1}')" != "$CORE_LOCK_SHA256" ]]; then
  echo "standalone Sol-Atlas Cargo.lock changed during qualification despite --locked" >&2
  exit 1
fi

# Newly resolved lockfiles are evidence inputs, not source edits. Keep their
# exact copies in the evidence packet and restore clean checkouts before PASS.
if [[ "$SIMULATOR_LOCK_PREEXISTED" -eq 0 ]]; then
  rm -f -- "$SIMULATOR_LOCK_PATH"
fi
if [[ "$CORE_LOCK_PREEXISTED" -eq 0 ]]; then
  rm -f -- "$CORE_LOCK_PATH"
fi
SIMULATOR_LOCK_PREEXISTED=1
CORE_LOCK_PREEXISTED=1
for checkout in "$MONOREPO_ROOT" "$SOL_ATLAS_ROOT" "$SYMTROPY_ROOT"; do
  if [[ -n "$(git -C "$checkout" status --porcelain --untracked-files=all)" ]]; then
    echo "source checkout became dirty during qualification: $checkout" >&2
    exit 1
  fi
done

QUALIFICATION_PHASE="qualification_metadata_finalization"
python3 - "$OUT_DIR/qualification-metadata.json" "$SOURCE_ARTIFACT" "$RUN_ENVELOPE" "$VALIDATION_REPORT" "$YEARS" "$SEED" "$OUT_DIR/simulator-Cargo.lock" "$OUT_DIR/sol-atlas-core-Cargo.lock" "$OUT_DIR/simulator-lockfile-origin.txt" "$OUT_DIR/sol-atlas-core-lockfile-origin.txt" <<'PY'
import hashlib, json, os, subprocess, sys
metadata_path, source_path, envelope_path, report_path, years, seed, simulator_lock_path, core_lock_path, simulator_lock_origin_path, core_lock_origin_path = sys.argv[1:]
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
expected_producer_revision = text(os.path.join(os.path.dirname(metadata_path), "monorepo-commit.txt"))
reported_source_revision = report.get("source_revision")
if reported_source_revision != expected_producer_revision:
    raise SystemExit("consumer report source revision does not match the captured monorepo HEAD")
metadata = {
    "record_type": "humanity-atlas-structural-qualification",
    "status": "pass" if report.get("qualification") == "structural_contract_pass" else "fail",
    "qualification_scope": "contract and artifact integrity only; not scientific validation",
    "years": int(years),
    "seed": int(seed),
    "monorepo_commit": expected_producer_revision,
    "producer_revision_reported_by_consumer": reported_source_revision,
    "source_revision_matches_monorepo_commit": reported_source_revision == expected_producer_revision,
    "sol_atlas_commit": text(os.path.join(os.path.dirname(metadata_path), "sol-atlas-commit.txt")),
    "symtropy_commit": text(os.path.join(os.path.dirname(metadata_path), "symtropy-commit.txt")),
    "source_artifact_sha256": sha(source_path),
    "run_envelope_sha256": sha(envelope_path),
    "validation_report_sha256": sha(report_path),
    "simulator_cargo_lock_sha256": sha(simulator_lock_path),
    "simulator_lockfile_origin": text(simulator_lock_origin_path),
    "sol_atlas_core_cargo_lock_sha256": sha(core_lock_path),
    "sol_atlas_core_lockfile_origin": text(core_lock_origin_path),
    "adapter_source_sha256": sha(os.path.join(os.path.dirname(metadata_path), "adapt_multiworld_output.py")),
    "adapter_tests_source_sha256": sha(os.path.join(os.path.dirname(metadata_path), "test_adapt_multiworld_output.py")),
    "run_schema_sha256": sha(os.path.join(os.path.dirname(metadata_path), "run-v0.1.schema.json")),
    "producer_tree_schema_sha256": sha(os.path.join(os.path.dirname(metadata_path), "producer-tech-tree-v1.schema.json")),
    "consumer_tree_schema_sha256": sha(os.path.join(os.path.dirname(metadata_path), "consumer-tech-tree-v1.schema.json")),
    "tree_schemas_byte_identical": True,
    "producer_run_manifest_schema_sha256": sha(os.path.join(os.path.dirname(metadata_path), "producer-run-manifest-v1.schema.json")),
    "consumer_run_manifest_schema_sha256": sha(os.path.join(os.path.dirname(metadata_path), "consumer-run-manifest-v1.schema.json")),
    "run_manifest_schemas_byte_identical": True,
    "simulator_log_sha256": sha(os.path.join(os.path.dirname(metadata_path), "simulator-run.log")),
    "adapter_log_sha256": sha(os.path.join(os.path.dirname(metadata_path), "adapter.log")),
    "python_adapter_tests_log_sha256": sha(os.path.join(os.path.dirname(metadata_path), "python-adapter-tests.log")),
    "run_schema_validation_log_sha256": sha(os.path.join(os.path.dirname(metadata_path), "run-schema-validation.log")),
    "source_model_schema_validation_log_sha256": sha(os.path.join(os.path.dirname(metadata_path), "source-model-schema-validation.log")),
    "producer_consumer_roundtrip_log_sha256": sha(os.path.join(os.path.dirname(metadata_path), "producer-consumer-roundtrip.log")),
    "producer_consumer_roundtrip_test_passed": True,
    "sol_atlas_core_tests_log_sha256": sha(os.path.join(os.path.dirname(metadata_path), "sol-atlas-core-tests.log")),
    "sol_atlas_validation_log_sha256": sha(os.path.join(os.path.dirname(metadata_path), "sol-atlas-validation.log")),
    "rustc_version": text(os.path.join(os.path.dirname(metadata_path), "rustc-version.txt")),
    "cargo_version": text(os.path.join(os.path.dirname(metadata_path), "cargo-version.txt")),
    "python_version": text(os.path.join(os.path.dirname(metadata_path), "python-version.txt")),
    "checkouts_clean_at_start": True,
    "checkouts_clean_after_qualification": True,
    "isolated_cargo_target_dirs": True,
    "validation_report": report,
}
with open(metadata_path, "w", encoding="utf-8") as stream:
    json.dump(metadata, stream, indent=2, sort_keys=True)
    stream.write("\n")
if metadata["status"] != "pass":
    raise SystemExit("consumer report did not declare structural_contract_pass")
PY

sha256sum "$OUT_DIR/qualification-metadata.json" > "$OUT_DIR/qualification-metadata.json.sha256"
QUALIFICATION_PHASE="completed"
echo "PASS: structural artifact qualification recorded at $OUT_DIR/qualification-metadata.json" >&2
echo "Note: this is not empirical calibration, scientific validation, or publisher authentication." >&2
