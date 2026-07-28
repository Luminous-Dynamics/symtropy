#!/usr/bin/env bash
# Sync to Standalone — Push symtropy crate to Luminous-Dynamics/symtropy repo
#
# GitHub Actions only reads workflows from a repository ROOT. Symtropy's
# `.github/workflows/ci.yml` lives inside the monorepo subdirectory
# `symtropy/`, so it has never actually run. This script syncs symtropy to
# the standalone public repo (github.com/Luminous-Dynamics/symtropy) so CI
# can see it, applying Cargo.toml fixups so the synced tree actually builds
# outside the monorepo (see "Cargo.toml fixups" section below for the full
# judgment-call writeup on each escaping dependency).
#
# Modeled on symthaea/scripts/sync-to-standalone.sh.
#
# Usage:
#   bash symtropy/scripts/sync-to-standalone.sh [--dry-run] [--skip-check] [--force] [--allow-check-failure]
#
# Options:
#   --dry-run              Show what would change without modifying the standalone repo
#   --skip-check           Skip the post-sync `cargo check` verification entirely
#   --force                Commit and push without interactive confirmation
#   --allow-check-failure  Push even if the post-sync cargo fmt/check verification
#                          fails (default: a failed check aborts before commit/push,
#                          regardless of --force, since a broken push to the public
#                          repo is worse than a delayed one)

set -euo pipefail

# --- Configuration -----------------------------------------------------------

MONOREPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SYMTROPY_DIR="${MONOREPO_ROOT}/symtropy"
STANDALONE_REPO="/tmp/symtropy-standalone-sync"
STANDALONE_REMOTE="git@github.com:Luminous-Dynamics/symtropy.git"

DRY_RUN=false
SKIP_CHECK=false
FORCE=false
ALLOW_CHECK_FAILURE=false
for arg in "$@"; do
    case "$arg" in
        --dry-run)              DRY_RUN=true ;;
        --skip-check)           SKIP_CHECK=true ;;
        --force)                FORCE=true ;;
        --allow-check-failure)  ALLOW_CHECK_FAILURE=true ;;
    esac
done

# --- Colors -------------------------------------------------------------------

GREEN="\033[32m"
YELLOW="\033[33m"
RED="\033[31m"
CYAN="\033[36m"
RESET="\033[0m"
BOLD="\033[1m"

info()  { printf "${CYAN}[info]${RESET}  %s\n" "$*"; }
warn()  { printf "${YELLOW}[warn]${RESET}  %s\n" "$*"; }
ok()    { printf "${GREEN}[ok]${RESET}    %s\n" "$*"; }
error() { printf "${RED}[error]${RESET} %s\n" "$*"; exit 1; }

# --- Versions of the 3 published symthaea-* crates this repo escapes to -----
#
# symtropy depends on several crates from the private symthaea monorepo via
# `../symthaea/...` paths. Three of them are published on crates.io and get
# rewritten to version deps below. Confirmed live 2026-07-03.
SYMTHAEA_CORE_VERSION="0.5.1"
SYMTHAEA_FEP_VERSION="0.1.0"
SYMTHAEA_CONSCIOUSNESS_EQUATION_VERSION="0.1.0"

# --- Validate monorepo --------------------------------------------------------

if [ ! -f "${SYMTROPY_DIR}/Cargo.toml" ]; then
    error "Cannot find ${SYMTROPY_DIR}/Cargo.toml — run from monorepo root"
fi

# --- Snapshot committed HEAD, not the live working tree ----------------------
#
# This monorepo routinely runs 12+ concurrent Claude/human sessions, any of
# which can have uncommitted edits sitting in symtropy/ at the moment this
# script runs (mid-refactor, scratch experiments, another session's WIP).
# Syncing straight from the live working directory means that uncommitted
# state -- good or bad, understood or not -- gets swept into a PUBLIC push
# with no way to tell the two apart short of manually inspecting `git status`
# first. Found in practice 2026-07-28: a real, necessary, already-once-
# committed fix was sitting uncommitted in the tree (silently reverted by an
# unrelated later commit) right when this script was about to run; it
# happened to be safe to include, but only after manual investigation.
# Snapshotting `git archive HEAD` instead makes "what gets published"
# strictly equal to "what's committed" -- no judgment call needed on a
# future run, regardless of what any other session leaves lying around.
SNAPSHOT_DIR="$(mktemp -d /tmp/symtropy-sync-snapshot.XXXXXX)"
trap 'rm -rf "${SNAPSHOT_DIR}"' EXIT
info "Snapshotting committed HEAD (not the live working tree) into ${SNAPSHOT_DIR}..."
git -C "${MONOREPO_ROOT}" archive HEAD -- symtropy | tar -x -C "${SNAPSHOT_DIR}"
SYMTROPY_DIR="${SNAPSHOT_DIR}/symtropy"

info "Monorepo root: ${MONOREPO_ROOT}"
info "Symtropy dir:  ${SYMTROPY_DIR}"
info "Standalone:    ${STANDALONE_REPO}"
if $DRY_RUN; then
    warn "DRY RUN — no commits or pushes will be made"
fi
echo

# --- Clone or update standalone repo ------------------------------------------

if [ -d "${STANDALONE_REPO}/.git" ]; then
    info "Updating existing standalone clone..."
    git -C "${STANDALONE_REPO}" fetch origin
    git -C "${STANDALONE_REPO}" checkout main
    git -C "${STANDALONE_REPO}" reset --hard origin/main
else
    info "Cloning standalone repo..."
    GIT_LFS_SKIP_SMUDGE=1 git clone "${STANDALONE_REMOTE}" "${STANDALONE_REPO}"
fi
echo

# --- Rsync directories -------------------------------------------------------
#
# Exclusions mirror symthaea's script: build artifacts, runtime output,
# caches, and anything that would bloat the public repo for no reason.

RSYNC_EXCLUDE=(
    --exclude='target/'
    --exclude='.DS_Store'
    --exclude='__pycache__/'
    --exclude='__pyphi_cache__/'
    --exclude='.pytest_cache/'
    --exclude='node_modules/'
    --exclude='result'
)
RSYNC_OPTS=(-a --delete "${RSYNC_EXCLUDE[@]}")
if $DRY_RUN; then
    RSYNC_OPTS+=(--dry-run -v)
else
    RSYNC_OPTS+=(-v)
fi

# Directories that contain source code and are needed for compilation/CI.
SOURCE_DIRS=(
    src
    crates
    tests
    examples
    benches
)

# Directories that contain supporting files needed by CI or the project.
# NOT synced (with explanation):
#   assets/       - 27MB of Bevy game assets (textures/audio); CI never loads
#                   them (cargo check/fmt only), so they'd only bloat the
#                   public repo. Add back manually if standalone users want
#                   to actually run the game.
#   archive/      - historical, not needed
#   chronicle/    - runtime event log output (manifest.json/events.jsonl) plus
#                   a stray .rs file; not referenced by any `mod` declaration
#                   in src/ (the real chronicle recorder is
#                   src/systems/chronicle_recorder.rs, which IS synced as
#                   part of src/)
#   .cargo/       - host-specific linker/cache tuning (mold + sccache
#                   rustflags). Neither is installed on GitHub Actions
#                   runners; shipping this would break the CI build with a
#                   "linker `mold` not found" error.
#   .agents/, __pycache__/, __pyphi_cache__/, .pytest_cache/, deps/,
#   symthaea-deps/, _archive-2026-06-17-workspace-normalization/, tools/ -
#                   empty, agent-instruction, or dev-cache directories, not
#                   needed for CI
SUPPORT_DIRS=(
    .github
    book
    docs
    papers
    plans
    scripts
    launcher
)

sync_dir() {
    local dir="$1"
    if [ ! -d "${SYMTROPY_DIR}/${dir}" ]; then
        warn "Skipping ${dir}/ (not found in monorepo)"
        return
    fi
    info "Syncing ${dir}/"
    rsync "${RSYNC_OPTS[@]}" \
        "${SYMTROPY_DIR}/${dir}/" \
        "${STANDALONE_REPO}/${dir}/"
}

info "=== Syncing source directories ==="
for dir in "${SOURCE_DIRS[@]}"; do
    sync_dir "$dir"
done

echo
info "=== Syncing support directories ==="
for dir in "${SUPPORT_DIRS[@]}"; do
    sync_dir "$dir"
done

echo

# --- Copy individual files ----------------------------------------------------

INDIVIDUAL_FILES=(
    "Cargo.lock"
    "LICENSE-APACHE"
    "LICENSE-MIT"
    "README.md"
    ".gitignore"
    "justfile"
    "flake.nix"
    "flake.lock"
)
# rust-toolchain.toml, CONTRIBUTING.md, clippy.toml, deny.toml, rustfmt.toml,
# SECURITY.md, SAFETY.md, CHANGELOG.md do not exist in symtropy/ (checked
# 2026-07-03) — nothing to copy for those.

for f in "${INDIVIDUAL_FILES[@]}"; do
    if [ -f "${SYMTROPY_DIR}/${f}" ]; then
        info "Copying ${f}"
        if ! $DRY_RUN; then
            cp "${SYMTROPY_DIR}/${f}" "${STANDALONE_REPO}/${f}"
        fi
    else
        warn "Missing ${f} in monorepo symtropy/ — skipping"
    fi
done

echo

# --- Ensure stub crates exist in the standalone repo -------------------------
#
# symtropy depends on 3 pieces of the private symthaea monorepo that are
# NOT published to crates.io and are too large to vendor whole:
#   - symthaea (full crate, ~1.68M LOC)
#   - symthaea-muse (music synthesis)
#   - symthaea-biometrics (input telemetry -> stress model)
# ...plus one platform crate that IS small enough to stub in full:
#   - symthaea-orbital (needed only by symtropy-symthaea-adapter's
#     integration test; the demo crate that needs its full simulator/encoder
#     surface is excluded from the workspace instead, see below)
#
# Per each consumer's ACTUAL API usage (verified by reading every call site),
# these stubs are real, minimal, type-compatible stand-ins — not fakes.
#
# CONTENT-ADDRESSED (changed 2026-07-28). This function used to write only when
# the target was MISSING, on the theory that hand edits made directly in the
# standalone repo should never be clobbered. That theory cost us three
# separate CI outages, because the heredocs below are the only real source of
# truth for these files and a fix applied here silently did nothing to an
# already-populated standalone repo:
#
#   1. symthaea-biometrics/src/lib.rs — missing InputTelemetryEncoder::{reset,
#      velocity_surprise}, both called unconditionally by plugin.rs/player.rs/
#      postprocess.rs.
#   2. symthaea-orbital/src/lib.rs — apply_moral_gate() had a rustfmt
#      violation under the CI toolchain.
#   3. symthaea-muse/src/lib.rs — two hand-written `impl Default` blocks
#      tripping clippy::derivable_impls under `-D warnings`, which took public
#      CI down from 2026-06-21 to 2026-07-28 across seven consecutive syncs.
#
# Each was previously worked around with a one-off `rm -f` immediately before
# the regeneration block. Those three lines are now deleted: writing whenever
# content differs makes the heredoc authoritative by construction, so the
# class of bug cannot recur. The "don't clobber hand edits" property is
# preserved where it actually matters — a hand edit that MATCHES the heredoc
# is a no-op, and one that does not is a divergence we want overwritten,
# because CI builds the heredoc's intent, not the drift.

ensure_stub_file() {
    local path="$1"
    local content
    content="$(cat)"
    if [ -f "$path" ] && [ "$(cat "$path")" = "$content" ]; then
        return
    fi
    local verb="Created"
    [ -f "$path" ] && verb="Updated (content drifted from generator)"
    mkdir -p "$(dirname "$path")"
    printf '%s\n' "$content" > "$path"
    ok "${verb} stub file: ${path#${STANDALONE_REPO}/}"
}

if ! $DRY_RUN; then
    info "=== Ensuring stub crates match their generators (content-addressed) ==="

    # --- stubs/symthaea ---
    # Real API surface needed: exactly the two `use` lines in this repo's
    # crates/bridges/symthaea-bevy-brain/src/lib.rs:
    #   pub use symthaea::cognitive_loop::{CognitiveLoopBuilder, CognitiveLoopService, CycleMetadata, CycleResult};
    #   pub use symthaea::symthaea_core::hdc::unified_hv::ContinuousHV;
    # Nothing else in this whole workspace calls into the full `symthaea`
    # crate (verified via `grep -rn '\bsymthaea::'` across src/ and crates/;
    # the root package's own unconditional `symthaea` dependency is unused
    # dead weight from Cargo's perspective). `symthaea_core` is re-exported
    # here as `pub use symthaea_core;`, exactly mirroring what the real
    # symthaea crate does (its src/lib.rs has `pub use symthaea_core;`).
    ensure_stub_file "${STANDALONE_REPO}/stubs/symthaea/Cargo.toml" <<EOF
[package]
name = "symthaea"
version = "0.1.0"
edition = "2024"
license = "AGPL-3.0-or-later"
description = "Standalone-repo stub: minimal type-compatible stand-in for the full (unpublished, ~1.68M LOC) symthaea crate. Exposes only the cognitive_loop surface actually consumed by symtropy's symthaea-bevy-brain crate. Maintained IN this standalone repo, not synced from the monorepo -- see scripts/sync-to-standalone.sh."
publish = false

[dependencies]
symthaea-core = "${SYMTHAEA_CORE_VERSION}"

[features]
default = []
vision-manifold = []
muse = []
reasoning_engine = []
humanoid = []
vehicle = []
EOF

    ensure_stub_file "${STANDALONE_REPO}/stubs/symthaea/src/lib.rs" <<'EOF'
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Minimal stand-in for the real `symthaea` crate (private, unpublished,
//! ~1.68M LOC). Only reimplements the tiny surface actually used in this
//! repo (see `crates/bridges/symthaea-bevy-brain/src/lib.rs`): a trivial
//! `CognitiveLoopService` that returns zeroed telemetry every cycle. Real
//! HDC/CfC/Phi cognition lives in the private monorepo -- this stub exists
//! purely so the public repo's Cargo graph resolves and `cargo check`
//! succeeds.

pub use symthaea_core;

pub mod cognitive_loop {
    use symthaea_core::hdc::unified_hv::ContinuousHV;

    #[derive(Default, Clone, Debug)]
    pub struct EmbodiedTelemetry {
        pub body_phi_modulation: f64,
    }

    #[derive(Default, Clone, Debug)]
    pub struct CycleMetadata {
        pub mce_softmin: f64,
        pub mce_weighted_sum: f64,
        pub perception_attention_sensitivity: f32,
        pub mce_social: f64,
        pub mce_narrative: f64,
        pub memory_recall_quality: f32,
        pub embodied: EmbodiedTelemetry,
    }

    #[derive(Clone, Debug, Default)]
    pub struct CycleResult {
        pub output: Vec<f32>,
        pub metadata: CycleMetadata,
    }

    /// Trivial cognitive loop: produces zeroed output and telemetry every
    /// cycle. See module docs for why.
    pub struct CognitiveLoopService {
        cfc_neurons: usize,
        priors_mean: Vec<f64>,
        priors_precision: Vec<f64>,
    }

    impl CognitiveLoopService {
        pub fn inject_priors(&mut self, mean: Vec<f64>, precision: Vec<f64>) {
            self.priors_mean = mean;
            self.priors_precision = precision;
        }

        pub fn cycle(&mut self, _input: &str) -> CycleResult {
            CycleResult {
                output: vec![0.0; self.cfc_neurons],
                metadata: CycleMetadata::default(),
            }
        }

        pub fn cycle_with_hv(&mut self, _hv: &ContinuousHV) -> CycleResult {
            CycleResult {
                output: vec![0.0; self.cfc_neurons],
                metadata: CycleMetadata::default(),
            }
        }

        pub fn consciousness_level(&self) -> f32 {
            0.5
        }
    }

    #[derive(Default)]
    pub struct CognitiveLoopBuilder {
        cfc_neurons: usize,
        genesis_phrase: String,
    }

    impl CognitiveLoopBuilder {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with_cfc_neurons(mut self, n: usize) -> Self {
            self.cfc_neurons = n;
            self
        }

        pub fn with_genesis_phrase(mut self, phrase: &str) -> Self {
            self.genesis_phrase = phrase.to_string();
            self
        }

        pub fn build(self) -> Result<CognitiveLoopService, String> {
            let _ = &self.genesis_phrase;
            Ok(CognitiveLoopService {
                cfc_neurons: self.cfc_neurons,
                priors_mean: Vec::new(),
                priors_precision: Vec::new(),
            })
        }
    }
}
EOF

    # --- stubs/symthaea-muse ---
    # Real API surface needed (verified via grep across src/ and crates/):
    # `MuseConfig`, `MusicalState`, `ReverbConfig` -- plain data/config
    # structs with no algorithmic logic, so field-for-field replication is
    # both mechanical and necessary for type compatibility (callers rely on
    # `..Default::default()` composition and on `MuseConfig::horror()` /
    # `::elite_sterile()` preset constructors).
    ensure_stub_file "${STANDALONE_REPO}/stubs/symthaea-muse/Cargo.toml" <<'EOF'
[package]
name = "symthaea-muse"
version = "0.1.0"
edition = "2024"
license = "AGPL-3.0-or-later"
description = "Standalone-repo stub: minimal type-compatible stand-in for symthaea-muse's public config/state types actually used by symtropy (MuseConfig/MusicalState/ReverbConfig). Maintained IN this standalone repo -- see scripts/sync-to-standalone.sh."
publish = false

[dependencies]
serde = { version = "1.0", features = ["derive"] }

[features]
default = []
muse-live = []
EOF

    ensure_stub_file "${STANDALONE_REPO}/stubs/symthaea-muse/src/lib.rs" <<'EOF'
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Minimal stand-in for the real `symthaea-muse` crate (private). Only
//! reimplements the plain config/state data types symtropy actually
//! constructs and reads (`MuseConfig`, `MusicalState`, `ReverbConfig`); the
//! real crate's synthesis/melody/notation machinery is not needed here and
//! is not reimplemented.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MelodyMode {
    #[default]
    Classic,
    Neural,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OutputFormat {
    Mono16,
    MonoF32,
    #[default]
    StereoF32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ReverbConfig {
    pub room_size: f32,
    pub damping: f32,
    pub width: f32,
}

impl Default for ReverbConfig {
    fn default() -> Self {
        Self {
            room_size: 0.5,
            damping: 0.5,
            width: 0.8,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuseConfig {
    pub sample_rate: u32,
    pub duration_secs: f32,
    pub base_tempo_bpm: f32,
    pub max_notes: usize,
    pub melody_mode: MelodyMode,
    pub output_format: OutputFormat,
    pub num_partials: usize,
    pub enable_antialiasing: bool,
    pub reverb: ReverbConfig,
    pub cfc_layer_sizes: Vec<usize>,
    pub max_fm_depth: f32,
    pub enable_sub_bass: bool,
    pub unison_detune: f32,
    pub noise_mix: f32,
}

impl Default for MuseConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            duration_secs: 8.0,
            base_tempo_bpm: 80.0,
            max_notes: 32,
            melody_mode: MelodyMode::default(),
            output_format: OutputFormat::default(),
            num_partials: 8,
            enable_antialiasing: true,
            reverb: ReverbConfig::default(),
            cfc_layer_sizes: vec![16, 16, 8],
            max_fm_depth: 3.0,
            enable_sub_bass: false,
            unison_detune: 0.0,
            noise_mix: 0.0,
        }
    }
}

impl MuseConfig {
    /// Horror/high-tension preset (deep FM, sub-bass, detuned unison, noise).
    pub fn horror() -> Self {
        Self {
            max_fm_depth: 8.0,
            enable_sub_bass: true,
            unison_detune: 0.005,
            noise_mix: 0.1,
            num_partials: 12,
            reverb: ReverbConfig {
                room_size: 0.85,
                damping: 0.3,
                width: 1.0,
            },
            ..Default::default()
        }
    }

    /// Lunar Elite sterile preset (pure sine tones, tight reverb).
    pub fn elite_sterile() -> Self {
        Self {
            max_fm_depth: 0.5,
            enable_sub_bass: false,
            unison_detune: 0.0,
            noise_mix: 0.0,
            num_partials: 2,
            reverb: ReverbConfig {
                room_size: 0.2,
                damping: 0.8,
                width: 0.3,
            },
            ..Default::default()
        }
    }
}

/// Cognitive state for music generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicalState {
    pub harmony_activations: [f32; 8],
    pub dopamine: f32,
    pub serotonin: f32,
    pub noradrenaline: f32,
    pub arousal: f32,
    pub valence: f32,
    pub consciousness_level: f32,
    pub prediction_error: f32,
}

impl Default for MusicalState {
    fn default() -> Self {
        Self {
            harmony_activations: [0.3; 8],
            dopamine: 0.5,
            serotonin: 0.5,
            noradrenaline: 0.3,
            arousal: 0.4,
            valence: 0.0,
            consciousness_level: 0.5,
            prediction_error: 0.1,
        }
    }
}
EOF

    # --- stubs/symthaea-biometrics ---
    # Real API surface needed (verified via grep across src/): `MouseSample`,
    # `KeystrokeSample`, `InputTelemetryEncoder` (new/push_mouse/
    # push_keystroke/compute_stress_vector), `PlayerStressModel` (new/tick/
    # is_burnout/reset). The real crate derives its stress vector from an
    # Echo State Network surprise signal (symthaea_core::hdc::reservoir);
    # this stub uses a simple velocity/rate EMA instead -- deliberately NOT
    # copying that private algorithm, just matching the public shape.
    ensure_stub_file "${STANDALONE_REPO}/stubs/symthaea-biometrics/Cargo.toml" <<'EOF'
[package]
name = "symthaea-biometrics"
version = "0.1.0"
edition = "2024"
license = "AGPL-3.0-or-later"
description = "Standalone-repo stub: minimal type-compatible stand-in for symthaea-biometrics (mouse/keyboard -> stress vector). Uses simple velocity/rate heuristics instead of the real crate's private Echo-State-Network surprise detector. Maintained IN this standalone repo -- see scripts/sync-to-standalone.sh."
publish = false

[dependencies]

[features]
default = []
muse-bridge = []
EOF

    ensure_stub_file "${STANDALONE_REPO}/stubs/symthaea-biometrics/src/lib.rs" <<'EOF'
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Minimal stand-in for the real `symthaea-biometrics` crate (private).
//! Reimplements only the public shape (types + method signatures) that
//! symtropy calls; uses simple velocity/keystroke-rate EMA heuristics
//! rather than the real crate's Echo-State-Network surprise detector.

pub mod input_telemetry {
    use std::collections::VecDeque;

    const MAX_MOUSE_HISTORY: usize = 8;

    #[derive(Debug, Clone, Copy)]
    pub struct MouseSample {
        pub x: f32,
        pub y: f32,
        pub timestamp_ms: u64,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct KeystrokeSample {
        pub key_down: bool,
        pub timestamp_ms: u64,
    }

    #[derive(Debug, Clone, Copy, Default)]
    pub struct StressVector {
        pub arousal: f32,
        pub valence: f32,
        pub dominance: f32,
    }

    pub struct InputTelemetryEncoder {
        mouse_history: VecDeque<MouseSample>,
        velocity_ema: f32,
        keystroke_rate_ema: f32,
    }

    impl InputTelemetryEncoder {
        pub fn new() -> Self {
            Self::with_seed(42)
        }

        pub fn with_seed(_seed: u64) -> Self {
            Self {
                mouse_history: VecDeque::with_capacity(MAX_MOUSE_HISTORY),
                velocity_ema: 0.0,
                keystroke_rate_ema: 0.0,
            }
        }

        pub fn push_mouse(&mut self, sample: MouseSample) {
            if let Some(prev) = self.mouse_history.back() {
                let dt_secs =
                    (sample.timestamp_ms.saturating_sub(prev.timestamp_ms)).max(1) as f32 / 1000.0;
                let dx = sample.x - prev.x;
                let dy = sample.y - prev.y;
                let v = (dx * dx + dy * dy).sqrt() / dt_secs;
                self.velocity_ema = self.velocity_ema * 0.85 + v.min(10.0) * 0.15;
            }
            if self.mouse_history.len() >= MAX_MOUSE_HISTORY {
                self.mouse_history.pop_front();
            }
            self.mouse_history.push_back(sample);
        }

        pub fn push_keystroke(&mut self, sample: KeystrokeSample) {
            if sample.key_down {
                self.keystroke_rate_ema = self.keystroke_rate_ema * 0.9 + 0.1;
            }
        }

        /// Simplified stand-in for the real crate's ESN-based prediction
        /// error: this stub has no ESN, so it just reports the current
        /// smoothed velocity as a same-range [0, 1] proxy signal.
        pub fn velocity_surprise(&self) -> f32 {
            self.velocity_ema.clamp(0.0, 1.0)
        }

        /// Clears history and smoothed state (matches the real crate's
        /// reset() semantics for this stub's smaller state set).
        pub fn reset(&mut self) {
            self.mouse_history.clear();
            self.velocity_ema = 0.0;
            self.keystroke_rate_ema = 0.0;
        }

        pub fn compute_stress_vector(&self) -> StressVector {
            let arousal = (self.velocity_ema * 0.5 + self.keystroke_rate_ema * 0.5).clamp(0.0, 1.0);
            StressVector {
                arousal,
                valence: 0.0,
                dominance: 0.0,
            }
        }
    }

    impl Default for InputTelemetryEncoder {
        fn default() -> Self {
            Self::new()
        }
    }
}

pub mod stress_model {
    use crate::input_telemetry::StressVector;

    const BURNOUT_THRESHOLD: f32 = 0.8;

    #[derive(Debug, Clone)]
    pub struct PlayerStressModel {
        pub allostatic_load: f32,
        pub cortisol_proxy: f32,
        pub da_baseline: f32,
        pub sht_baseline: f32,
        pub calm_duration_secs: f32,
    }

    impl PlayerStressModel {
        pub fn new() -> Self {
            Self {
                allostatic_load: 0.0,
                cortisol_proxy: 0.0,
                da_baseline: 0.5,
                sht_baseline: 0.5,
                calm_duration_secs: 0.0,
            }
        }

        pub fn tick(&mut self, stress: &StressVector, dt_secs: f32) {
            if stress.arousal > 0.6 {
                self.cortisol_proxy = (self.cortisol_proxy + 0.02 * dt_secs).min(1.0);
                self.allostatic_load = (self.allostatic_load + 0.01 * dt_secs).min(1.0);
                self.calm_duration_secs = 0.0;
            } else {
                self.cortisol_proxy = (self.cortisol_proxy - 0.005 * dt_secs).max(0.0);
                self.allostatic_load = (self.allostatic_load - 0.002 * dt_secs).max(0.0);
                self.calm_duration_secs += dt_secs;
            }
        }

        pub fn is_burnout(&self) -> bool {
            self.allostatic_load > BURNOUT_THRESHOLD
        }

        pub fn reset(&mut self) {
            *self = Self::new();
        }
    }

    impl Default for PlayerStressModel {
        fn default() -> Self {
            Self::new()
        }
    }
}
EOF

    # --- stubs/symthaea-orbital ---
    # Only symtropy-symthaea-adapter's dev-dependency integration test
    # (tests/orbital_integration.rs) needs this crate, and only its
    # `EmbodimentBridge` surface (new/step/reset/safety_level/
    # set_safety_override/clear_safety_override/platform/num_actuators/
    # total_steps/telemetry/apply_moral_gate). The Phi-tier safety gating
    # logic is real (uses the actual published `MotorSafetyLevel::from_phi`
    # / `.motor_gain()`), so the adapter's safety-tier assertions pass; the
    # zero-g dual-body physics simulator is NOT reimplemented (out of
    # scope for the tiny surface this stub serves). symtropy-orbital-demo
    # needs the real crate's `types`/`simulator`/`encoder` modules directly
    # and is excluded from the workspace instead of stubbed further -- see
    # the workspace-members exclusion list below.
    ensure_stub_file "${STANDALONE_REPO}/stubs/symthaea-orbital/Cargo.toml" <<EOF
[package]
name = "symthaea-orbital"
version = "0.1.0"
edition = "2024"
license = "AGPL-3.0-or-later"
description = "Standalone-repo stub: minimal EmbodimentBridge for the orbital platform, sufficient only for symtropy-symthaea-adapter's integration test. Not a physics simulator. Maintained IN this standalone repo -- see scripts/sync-to-standalone.sh."
publish = false

[dependencies]
symthaea-core = "${SYMTHAEA_CORE_VERSION}"
EOF

    ensure_stub_file "${STANDALONE_REPO}/stubs/symthaea-orbital/src/lib.rs" <<'EOF'
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Minimal stand-in for the real `symthaea-orbital` crate (private). Only
//! implements the real, published `EmbodimentBridge` trait using the real
//! `MotorSafetyLevel` Phi-tier logic; does not reimplement the private
//! crate's zero-g dual-body physics simulator/controller/encoder.

pub mod embodiment {
    use symthaea_core::embodiment::{
        EmbodimentBridge, EmbodimentPlatform, EmbodimentResult, EmbodimentTelemetry,
        GROUNDING_SENSORIMOTOR, MoralGateInput, MotorSafetyLevel, grounding_from_prediction_error,
        grounding_label,
    };
    use symthaea_core::genesis::GenesisSeed;
    use symthaea_core::hdc::ContinuousHV;

    const NUM_ACTUATORS: usize = 7;
    const HV_DIM: usize = 16_384;

    pub struct OrbitalEmbodiment {
        safety: MotorSafetyLevel,
        safety_ov: Option<MotorSafetyLevel>,
        moral_safety: Option<MotorSafetyLevel>,
        steps: usize,
        effort: f32,
        pe: f32,
    }

    impl OrbitalEmbodiment {
        pub fn new(_genesis: &GenesisSeed) -> Self {
            Self {
                safety: MotorSafetyLevel::Green,
                safety_ov: None,
                moral_safety: None,
                steps: 0,
                effort: 0.0,
                pe: 0.0,
            }
        }
    }

    impl EmbodimentBridge for OrbitalEmbodiment {
        fn step(&mut self, _hv: &ContinuousHV, _dt: f32, phi: f64) -> EmbodimentResult {
            let mut safety = MotorSafetyLevel::from_phi(phi);
            if let Some(ov) = self.safety_ov {
                safety = safety.max(ov);
            }
            if let Some(m) = self.moral_safety {
                safety = safety.max(m);
            }
            self.safety = safety;
            self.effort = safety.motor_gain();
            self.pe = 0.0;
            self.steps += 1;
            EmbodimentResult {
                num_actuators: NUM_ACTUATORS,
                control_effort: self.effort,
                success: true,
                prediction_error: self.pe,
                safety_level: self.safety,
                epistemic_grounding: GROUNDING_SENSORIMOTOR,
                observation_confidence: grounding_from_prediction_error(self.pe),
            }
        }

        fn encode_perception(&mut self) -> ContinuousHV {
            ContinuousHV::zero(HV_DIM)
        }

        fn reset(&mut self) {
            self.safety = MotorSafetyLevel::Green;
            self.safety_ov = None;
            self.moral_safety = None;
            self.steps = 0;
            self.effort = 0.0;
            self.pe = 0.0;
        }

        fn safety_level(&self) -> MotorSafetyLevel {
            self.safety
        }

        fn set_safety_override(&mut self, level: MotorSafetyLevel) {
            self.safety_ov = Some(level);
        }

        fn clear_safety_override(&mut self) {
            self.safety_ov = None;
        }

        fn platform(&self) -> EmbodimentPlatform {
            EmbodimentPlatform::Orbital
        }

        fn num_actuators(&self) -> usize {
            NUM_ACTUATORS
        }

        fn total_steps(&self) -> usize {
            self.steps
        }

        fn telemetry(&self) -> EmbodimentTelemetry {
            EmbodimentTelemetry {
                total_steps: self.steps as u64,
                control_effort: self.effort,
                prediction_error: self.pe,
                safety_level: format!("{:?}", self.safety),
                platform: "orbital".to_string(),
                num_actuators: NUM_ACTUATORS,
                epistemic_grounding: grounding_label(GROUNDING_SENSORIMOTOR).to_string(),
                observation_confidence: grounding_from_prediction_error(self.pe),
                platform_specific: Vec::new(),
            }
        }

        fn apply_moral_gate(&mut self, gate: MoralGateInput) {
            self.moral_safety =
                if gate.ahimsa_violated || gate.verdict == MoralGateInput::VERDICT_BLOCKED {
                    Some(MotorSafetyLevel::Red)
                } else if gate.consent_violation {
                    Some(MotorSafetyLevel::Orange)
                } else if gate.verdict == MoralGateInput::VERDICT_CAUTION {
                    Some(MotorSafetyLevel::Yellow)
                } else {
                    None
                };
        }
    }
}
EOF
    echo
fi

# --- Cargo.toml fixups ---------------------------------------------------

info "Copying Cargo.toml with standalone fixups..."

if ! $DRY_RUN; then
    TOML="${STANDALONE_REPO}/Cargo.toml"
    cp "${SYMTROPY_DIR}/Cargo.toml" "$TOML"

    # --- 0. Force naga's "termcolor" feature for the standalone-only ----
    #        old wgpu/naga stack pulled in via published symthaea-core
    #
    # The monorepo's own symthaea-core is a local path dependency and never
    # hits this — only the standalone repo's crates.io symthaea-core@0.5.1
    # pins wgpu = "^27.0" (-> naga 27.0.3), which coexists here alongside
    # the Bevy 0.19 stack's own naga 29.0.4 (two incompatible major
    # versions, both legitimately needed — not a version conflict to
    # resolve away). naga 27.0.3 has a real upstream bug: its
    # emit_to_string_with_path() unconditionally requires the WriteColor
    # -implementing DiagnosticBuffer variant, which only exists behind
    # naga's own "termcolor" feature. wgpu 27.0.1 does declare
    # `features = ["termcolor"]` on its naga dependency, but empirically
    # that request isn't reaching the final build's feature resolution in
    # this workspace (root cause of that specific gap not fully pinned
    # down — plausibly a resolver-v2 feature-unification interaction
    # between the two coexisting naga major versions). Forcing an
    # explicit top-level dependency with the feature is the standard,
    # low-risk fix for "a transitive optional feature isn't propagating
    # as expected" regardless of the exact resolver mechanism, and is
    # scoped to the standalone Cargo.toml only (never written back to the
    # monorepo, so it can't introduce a second naga version there).
    sed -i '/^\[dependencies\]/a naga = { version = "27", default-features = false, features = ["termcolor"] }' "$TOML"

    # --- 1. Fix internal absolute paths ---------------------------------
    #
    # Many sub-crate Cargo.tomls reference OTHER symtropy sub-crates via
    # hardcoded absolute paths like "/srv/luminous-dynamics/symtropy/crates/
    # ...", which only resolve on this one machine. Rewrite every such path,
    # in every synced Cargo.toml, to the correct relative path within the
    # standalone tree (which mirrors the monorepo's crates/ layout exactly).
    info "Fixing internal absolute paths (/srv/luminous-dynamics/symtropy/...)..."
    FIXED_ABS_PATHS=0
    while IFS= read -r -d '' toml_file; do
        toml_dir=$(dirname "$toml_file")
        while IFS= read -r abs_path; do
            target="${STANDALONE_REPO}${abs_path#/srv/luminous-dynamics/symtropy}"
            rel=$(realpath --relative-to="$toml_dir" "$target" 2>/dev/null) || continue
            esc_abs=$(printf '%s' "$abs_path" | sed 's/[.[\*^$/]/\\&/g')
            esc_rel=$(printf '%s' "$rel" | sed 's/[\/&]/\\&/g')
            sed -i "s|${esc_abs}|${esc_rel}|g" "$toml_file"
            FIXED_ABS_PATHS=$((FIXED_ABS_PATHS + 1))
        done < <(grep -oE '/srv/luminous-dynamics/symtropy/[^"]*' "$toml_file" | sort -u)
    done < <(find "${STANDALONE_REPO}" -name Cargo.toml -not -path '*/target/*' -print0)
    ok "Fixed ${FIXED_ABS_PATHS} absolute path reference(s)"

    # --- 2. Rewrite the 4 escaping stub deps to point at stubs/ ----------
    #
    # For each of symthaea / symthaea-muse / symthaea-biometrics /
    # symthaea-orbital, replace the `path = "..."` value (wherever it's
    # declared, at whatever depth) with the correct relative path to this
    # repo's own stubs/<name> crate. Preserves any other keys on the same
    # dependency table (features, default-features, version, optional).
    rewrite_stub_dep() {
        local dep_name="$1"
        local stub_target="${STANDALONE_REPO}/stubs/${dep_name}"
        if [ ! -d "$stub_target" ]; then
            warn "Stub target missing: stubs/${dep_name} (skipping rewrite)"
            return
        fi
        while IFS= read -r -d '' toml_file; do
            if grep -qE "^${dep_name}[[:space:]]*=.*path[[:space:]]*=" "$toml_file"; then
                toml_dir=$(dirname "$toml_file")
                rel=$(realpath --relative-to="$toml_dir" "$stub_target")
                esc_rel=$(printf '%s' "$rel" | sed 's/[\/&]/\\&/g')
                sed -i -E "s|^(${dep_name}[[:space:]]*=[[:space:]]*\{[^}]*path[[:space:]]*=[[:space:]]*)\"[^\"]*\"|\1\"${esc_rel}\"|" "$toml_file"
                ok "Rewrote ${dep_name} -> stubs/${dep_name} in ${toml_file#${STANDALONE_REPO}/}"
            fi
        done < <(find "${STANDALONE_REPO}" -name Cargo.toml -not -path '*/target/*' -not -path '*/stubs/*' -print0)
    }
    info "Rewriting escaping deps to stubs/..."
    rewrite_stub_dep "symthaea"
    rewrite_stub_dep "symthaea-muse"
    rewrite_stub_dep "symthaea-biometrics"
    rewrite_stub_dep "symthaea-orbital"

    # --- 3. Rewrite the 3 published symthaea-* deps to version pins -----
    #
    # symthaea-core, symthaea-fep, symthaea-consciousness-equation are all
    # published on crates.io. Replace the whole dependency table (dropping
    # any path= and pre-existing version=) with a plain version string,
    # wherever each appears in the tree.
    #
    # IMPORTANT: preserve `optional = true` if the original table had it.
    # All 3 of these deps are declared optional in the monorepo root
    # Cargo.toml and referenced via `dep:<name>` in the `symthaea-stack`/
    # `fep-ai`/`consciousness-runtime` features (added by the Jul 2026
    # feature-gating refactor) -- collapsing to a bare version string
    # silently drops that flag, which breaks manifest parsing entirely
    # (`dep:X` requires X to be an optional dependency; Cargo errors with
    # "but `X` is not an optional dependency" during metadata resolution,
    # i.e. `cargo fmt`/`cargo check` fail before compiling anything). Found
    # by actually running this script's own post-sync check, not guessed —
    # see SYMTROPY_IMPROVEMENT_PLAN_2026-07-21.md.
    info "Rewriting published symthaea-* deps to version pins..."
    rewrite_published_dep() {
        local dep_name="$1"
        local version="$2"
        while IFS= read -r -d '' toml_file; do
            # Case 1: table includes `optional = true` in either order relative
            # to `path` -- keep it, collapsing only to `{ version = ..., optional = true }`.
            sed -i -E \
                "s#^${dep_name}[[:space:]]*=[[:space:]]*\{([^}]*path[[:space:]]*=[[:space:]]*\"[^\"]*/symthaea/crates/(core|domains)/${dep_name}\"[^}]*optional[[:space:]]*=[[:space:]]*true[^}]*)\}#${dep_name} = { version = \"${version}\", optional = true }#" \
                "$toml_file"
            sed -i -E \
                "s#^${dep_name}[[:space:]]*=[[:space:]]*\{([^}]*optional[[:space:]]*=[[:space:]]*true[^}]*path[[:space:]]*=[[:space:]]*\"[^\"]*/symthaea/crates/(core|domains)/${dep_name}\"[^}]*)\}#${dep_name} = { version = \"${version}\", optional = true }#" \
                "$toml_file"
            # Case 2: non-optional path dependency -- collapse to a bare version
            # pin. Runs after case 1, so a line already rewritten above no
            # longer matches this (it no longer contains `path = ".../symthaea/...")`.
            sed -i -E \
                "s#^${dep_name}[[:space:]]*=[[:space:]]*\{[^}]*path[[:space:]]*=[[:space:]]*\"[^\"]*/symthaea/crates/(core|domains)/${dep_name}\"[^}]*\}#${dep_name} = \"${version}\"#" \
                "$toml_file"
        done < <(find "${STANDALONE_REPO}" -name Cargo.toml -not -path '*/target/*' -not -path '*/stubs/*' -print0)
        ok "Rewrote ${dep_name} -> \"${version}\" wherever a monorepo path was used (preserving optional=true if present)"
    }
    rewrite_published_dep "symthaea-core" "$SYMTHAEA_CORE_VERSION"
    rewrite_published_dep "symthaea-fep" "$SYMTHAEA_FEP_VERSION"
    rewrite_published_dep "symthaea-consciousness-equation" "$SYMTHAEA_CONSCIOUSNESS_EQUATION_VERSION"

    # --- 4. Strip sol-atlas-bevy / sol-atlas-core / atlas feature --------
    #
    # Both are optional (only the `atlas` feature enables them), neither is
    # published anywhere, and vendoring two more full crates (a Bevy scene +
    # a core lib) is out of scope. Comment them out; the `atlas` feature
    # simply won't build standalone (it isn't in default features, so this
    # doesn't affect the normal build).
    info "Stripping sol-atlas-bevy / sol-atlas-core / atlas feature (optional, not needed for CI)..."
    sed -i '/^sol-atlas-bevy\s*=.*path.*\.\.\//s/^/# [standalone-stripped, not published, atlas feature only] /' "$TOML"
    sed -i '/^sol-atlas-core\s*=.*path.*\.\.\//s/^/# [standalone-stripped, not published, atlas feature only] /' "$TOML"
    sed -i '/^atlas\s*=\s*\[/s/^/# [standalone-stripped -- referenced deps above are stripped] /' "$TOML"

    # --- 4b. Strip the `mycelix` feature ---------------------------------
    #
    # A pre-existing, documented gap (see CLAUDE.md's "mycelix" feature
    # note, and the comment right above the `[workspace] members` list
    # a few lines above the one we're patching): plugin.rs's `#[cfg(feature
    # = "mycelix")]` block and components.rs's real-vs-fallback type switch
    # reference symtropy_sim_bridge / mycelix_bridge_common /
    # mycelix_core_types directly, but none of the three is ever declared
    # as a Cargo.toml dependency of symtropy-launcher -- there's nothing to
    # rewrite-to-a-stub the way sol-atlas-bevy/sol-atlas-core are just
    # above, because the gap is a missing `[dependencies]` entry, not an
    # escaping path. All three need the private mycelix-workspace tree
    # (same reasoning as symtropy-sim-bridge/symtropy-world being excluded
    # from workspace members below) -- not published, not stubbed. Since
    # `mycelix` isn't in default features, this doesn't affect the normal
    # build; it only matters for `--all-features` (which `cargo check
    # --workspace --all-targets --all-features` below exercises).
    info "Stripping mycelix feature (pre-existing gap -- needs private mycelix-workspace, not published)..."
    sed -i '/^mycelix\s*=\s*\[/s/^/# [standalone-stripped -- symtropy_sim_bridge\/mycelix_bridge_common\/mycelix_core_types are unpublished, needs private mycelix-workspace] /' "$TOML"

    # --- 4c. Tell rustc the stripped `atlas`/`mycelix` feature names are ---
    #         still intentional, even though no longer declared -----------
    #
    # Steps 4 and 4b comment out the `atlas =` / `mycelix =` feature
    # declarations above, but src/{components,experience,plugin,resources,
    # systems/mod}.rs still reference `#[cfg(feature = "atlas")]` /
    # `#[cfg(feature = "mycelix")]` -- and rustc's `unexpected_cfgs` lint
    # flags any `cfg(feature = "X")` where "X" isn't a *declared* feature
    # name, regardless of whether that feature is ever enabled in this
    # build. Found 2026-07-28: this was silently failing the standalone
    # CI's Test Suite job (a real, previously-invisible failure -- masked
    # until an unrelated clippy fix in stubs/symthaea-muse let the build
    # get this far). check-cfg is exactly the documented escape hatch for
    # "this cfg value is known/intentional, just not always active."
    info "Declaring stripped atlas/mycelix as known-but-inactive cfg values (avoids unexpected_cfgs)..."
    cat >> "$TOML" <<'EOF'

[lints.rust]
unexpected_cfgs = { level = "allow", check-cfg = ['cfg(feature, values("atlas", "mycelix"))'] }
EOF

    # --- 5. Exclude workspace members that need unpublished/unstubbable --
    #        deps too large to responsibly vendor or stub
    #
    # Judgment calls (see script header + task writeup for full reasoning):
    #   - The 11 platform demo crates each require either an unpublished
    #     symthaea-<platform> physics crate directly, or (for
    #     symtropy-orbital-demo) the real symthaea-orbital's types/simulator/
    #     encoder modules -- a much deeper surface than the tiny
    #     EmbodimentBridge-only stub above serves.
    #   - symtropy-scavenger-demo pulls in symtropy-robotics-bridge, which
    #     itself unconditionally requires SIX unpublished platform crates
    #     (symthaea-auv/manipulator/humanoid/helicopter/vehicle/multirotor).
    #   - symtropy-world and symtropy-sim-bridge require
    #     mycelix-multiworld-sim and/or the entire mycelix-workspace/ tree --
    #     a separate, large, private monorepo project. Not published, not
    #     stubbed (would mean faking a governance/ZKP/consensus stack).
    # None of these are depended on by anything else that's kept (verified
    # via `grep` across every Cargo.toml in the tree), so removing them from
    # `members` is a clean cut with no knock-on breakage.
    EXCLUDED_MEMBERS_STATIC=(
        "crates/apps/symtropy-auv-demo"
        "crates/apps/symtropy-exoskeleton-demo"
        "crates/apps/symtropy-flight-demo"
        "crates/apps/symtropy-gravcraft-demo"
        "crates/apps/symtropy-helicopter-demo"
        "crates/apps/symtropy-humanoid-demo"
        "crates/apps/symtropy-manipulator-demo"
        "crates/apps/symtropy-orbital-demo"
        "crates/apps/symtropy-quadruped-demo"
        "crates/apps/symtropy-scavenger-demo"
        "crates/apps/symtropy-surgical-demo"
        "crates/apps/symtropy-vehicle-demo"
        "crates/domains/symtropy-world"
        "crates/bridges/symtropy-sim-bridge"
    )
    info "Excluding ${#EXCLUDED_MEMBERS_STATIC[@]} workspace member(s) that need unpublished/unstubbable deps..."
    for member in "${EXCLUDED_MEMBERS_STATIC[@]}"; do
        if grep -q "\"${member}\"" "$TOML"; then
            sed -i "\|\"${member}\"|d" "$TOML"
            warn "Excluded workspace member: ${member} (directory kept on disk, just not a member)"
        fi
    done

    python3 - "$TOML" <<'PYEOF'
import sys
path = sys.argv[1]
comment = """
# --- Members intentionally excluded from the STANDALONE workspace ---
# (the private monorepo workspace still builds these; they're removed here
# only because they need dependencies that are unpublished/private and too
# large to responsibly stub or vendor -- see scripts/sync-to-standalone.sh
# for the full per-crate reasoning)
#   symtropy-auv-demo, symtropy-exoskeleton-demo, symtropy-flight-demo,
#   symtropy-gravcraft-demo, symtropy-helicopter-demo, symtropy-humanoid-demo,
#   symtropy-manipulator-demo, symtropy-orbital-demo, symtropy-quadruped-demo,
#   symtropy-scavenger-demo, symtropy-surgical-demo, symtropy-vehicle-demo,
#   symtropy-world, symtropy-sim-bridge
"""
text = open(path).read()
text = text.replace("[workspace]", "[workspace]" + comment, 1)
open(path, "w").write(text)
PYEOF
    ok "Documented exclusions inline in Cargo.toml"

    # --- 6. Safety net: remove any workspace member whose directory is ---
    #        missing after all the above (mirrors symthaea template)
    REMOVED_MEMBERS=0
    while IFS= read -r line; do
        crate_path=$(echo "$line" | sed -n 's/.*"\(crates\/[^"]*\)".*/\1/p')
        if [ -n "$crate_path" ] && [ ! -d "${STANDALONE_REPO}/${crate_path}" ]; then
            sed -i "\|\"${crate_path}\"|d" "$TOML"
            REMOVED_MEMBERS=$((REMOVED_MEMBERS + 1))
            warn "Removed missing workspace member: ${crate_path}"
        fi
    done < <(grep '"crates/' "$TOML")
    if [ $REMOVED_MEMBERS -gt 0 ]; then
        ok "Removed $REMOVED_MEMBERS additional missing workspace member(s)"
    fi

    # --- Verify the rewrites actually happened --------------------------
    REWRITE_OK=true
    for dep in symthaea symthaea-muse symthaea-biometrics; do
        if ! grep -q "${dep}.*stubs/${dep}" "$TOML"; then
            warn "Failed to rewrite ${dep} path to stubs/"
            REWRITE_OK=false
        fi
    done
    if grep -q 'symthaea-core.*path.*\.\./symthaea' "$TOML"; then
        warn "symthaea-core still has a monorepo path"
        REWRITE_OK=false
    fi
    ESCAPED_ABS=$(grep -n '/srv/luminous-dynamics' "$TOML" | grep -v '^#' || true)
    if [ -n "$ESCAPED_ABS" ]; then
        warn "Root Cargo.toml still has absolute monorepo paths:"
        echo "$ESCAPED_ABS"
        REWRITE_OK=false
    fi
    if $REWRITE_OK; then
        ok "Cargo.toml fixups verified"
    else
        error "Cargo.toml rewrite verification failed — check the sed patterns above"
    fi
else
    info "(dry-run) Would copy and patch Cargo.toml"
fi

echo

# --- Scan sub-crate Cargo.tomls for remaining escaping paths ------------------
#
# Only meaningful after a real (non-dry-run) sync — in --dry-run mode
# nothing was actually written to STANDALONE_REPO (rsync itself ran with
# --dry-run, and the Cargo.toml fixup section is skipped), so this would
# just be scanning whatever stale content already existed in the temp clone.

if ! $DRY_RUN; then
    info "Scanning all Cargo.tomls for remaining external/escaping path deps..."
    SUBCRATE_ISSUES=false
    while IFS= read -r toml_file; do
        rel_path="${toml_file#${STANDALONE_REPO}/}"
        if [ "$rel_path" = "Cargo.toml" ] || [[ "$rel_path" == stubs/* ]]; then
            continue
        fi
        BAD_PATHS=$(grep -nE 'path\s*=\s*"[^"]*(\.\./){3,}|/srv/luminous-dynamics|mycelix-workspace|mycelix-multiworld-sim' "$toml_file" 2>/dev/null | grep -v '^#' || true)
        if [ -n "$BAD_PATHS" ]; then
            warn "External path dep in ${rel_path}:"
            echo "  $BAD_PATHS"
            SUBCRATE_ISSUES=true
        fi
    done < <(find "${STANDALONE_REPO}" -name "Cargo.toml" -not -path "*/target/*" 2>/dev/null)

    if $SUBCRATE_ISSUES; then
        warn "Some sub-crates have path deps escaping the workspace tree (see above)"
    else
        ok "All sub-crate Cargo.tomls OK"
    fi
    echo
else
    info "(dry-run) Skipping escaping-path scan (nothing was actually synced yet)"
    echo
fi

# --- Post-sync cargo check ---------------------------------------------------

if ! $DRY_RUN && ! $SKIP_CHECK; then
    SYNC_CHECK_FAILED=false

    # Plain `cargo` fails on this NixOS host (rustup shim errors outside a
    # nix develop shell) -- wrap every verification call in nix develop so
    # the check actually runs instead of failing closed on an environment
    # error.
    #
    # IMPORTANT: every call site pipes cargo's output through `head`/`tail`
    # (to bound how much gets printed) -- without `set -o pipefail` inside
    # this inner `bash -c`, the pipeline's exit status is `head`/`tail`'s,
    # not cargo's, so a genuinely failing `cargo fmt`/`cargo check` gets
    # silently reported as "[ok] ... passed" (head/tail almost always exit
    # 0). This is the same "piped command masks the real exit code" bug
    # class this project has already hit before elsewhere -- found here by
    # actually running the check and seeing a real manifest error printed
    # right above a bogus "[ok]" line, not guessed. Since `SYNC_CHECK_FAILED`
    # gates whether this script proceeds to the commit/push confirmation
    # prompts, this bug meant a genuinely broken standalone tree could sail
    # through believing it passed. See SYMTROPY_IMPROVEMENT_PLAN_2026-07-21.md.
    run_standalone_cargo() {
        nix develop "${MONOREPO_ROOT}" --command bash -c \
            "set -o pipefail; cd '${STANDALONE_REPO}' && $1"
    }

    info "Running cargo fmt --all --check in standalone repo..."
    if run_standalone_cargo "cargo fmt --all -- --check 2>&1 | head -40"; then
        ok "cargo fmt passed"
    else
        warn "cargo fmt --check failed — unformatted code detected"
        SYNC_CHECK_FAILED=true
    fi

    # Mirrors CI's "Verify Workspace Build" step exactly.
    info "Running cargo check --workspace --all-targets (mirrors CI)..."
    if run_standalone_cargo "cargo check --workspace --all-targets 2>&1 | tail -80"; then
        ok "cargo check --workspace --all-targets passed"
    else
        warn "cargo check --workspace --all-targets failed — the sync may have issues"
        SYNC_CHECK_FAILED=true
    fi

    # Approximates the build surface CI's `--all-features` clippy job would
    # touch (without running clippy itself, which would be slow across a
    # Bevy workspace this size and isn't required for this sync's purposes).
    info "Running cargo check --workspace --all-targets --all-features..."
    if run_standalone_cargo "cargo check --workspace --all-targets --all-features 2>&1 | tail -80"; then
        ok "cargo check --all-features passed"
    else
        warn "cargo check --all-features failed"
        SYNC_CHECK_FAILED=true
    fi

    if $SYNC_CHECK_FAILED; then
        warn "Pre-push checks failed:"
        echo "  cd ${STANDALONE_REPO} && nix develop ${MONOREPO_ROOT} --command cargo fmt --all -- --check"
        echo "  cd ${STANDALONE_REPO} && nix develop ${MONOREPO_ROOT} --command cargo check --workspace --all-targets"
        echo "  cd ${STANDALONE_REPO} && nix develop ${MONOREPO_ROOT} --command cargo check --workspace --all-targets --all-features"
        if $ALLOW_CHECK_FAILURE; then
            warn "--allow-check-failure set — proceeding to commit/push anyway"
        else
            git -C "${STANDALONE_REPO}" reset HEAD -- . >/dev/null 2>&1
            error "Aborting before commit/push. Fix the above or re-run with --allow-check-failure to push anyway."
        fi
    fi
    echo
elif $SKIP_CHECK; then
    info "Skipping cargo check (--skip-check)"
    echo
fi

# --- Format with the toolchain CI uses (best-effort) -------------------------
# ci.yml uses dtolnay/rust-toolchain@stable (a floating tag), so there's no
# single pinned version to format against here; rely on whatever `cargo fmt`
# resolves in the nix develop shell (already run above as part of the check).

# --- Show diff summary --------------------------------------------------------

info "Changes in standalone repo:"
echo
git -C "${STANDALONE_REPO}" add -A -- ':!*.bin'
git -C "${STANDALONE_REPO}" diff --cached --stat
echo

CHANGES=$(git -C "${STANDALONE_REPO}" diff --cached --name-only | wc -l)
if [ "$CHANGES" -eq 0 ]; then
    info "No changes to sync — standalone is up to date."
    exit 0
fi

printf "${BOLD}%s files changed${RESET}\n" "$CHANGES"
echo

# --- Commit -------------------------------------------------------------------

if $DRY_RUN; then
    warn "DRY RUN — skipping commit and push"
    git -C "${STANDALONE_REPO}" reset HEAD -- . >/dev/null 2>&1
    exit 0
fi

MONOREPO_SHA=$(git -C "${MONOREPO_ROOT}" rev-parse --short HEAD)
COMMIT_MSG="sync: update from monorepo @ ${MONOREPO_SHA} ($(date +%Y-%m-%d))"

if $FORCE; then
    info "Committing: ${COMMIT_MSG}"
    git -C "${STANDALONE_REPO}" commit -m "${COMMIT_MSG}"
else
    printf "${YELLOW}Commit with message:${RESET} %s\n" "$COMMIT_MSG"
    printf "Proceed? [y/N] "
    # `read` returns non-zero on EOF (e.g. stdin redirected from /dev/null,
    # or a wrapper script that doesn't attach a real terminal); under
    # `set -e` that non-zero status kills the whole script right here,
    # BEFORE the abort branch below ever runs -- so the standalone repo is
    # left staged-but-uncommitted with no "Aborted" message and no cleanup.
    # Found in practice 2026-07-28 running this non-interactively. Guard it
    # so EOF is treated the same as an explicit "no".
    read -r confirm || confirm=""
    if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
        warn "Aborted — changes staged but not committed"
        git -C "${STANDALONE_REPO}" reset HEAD -- . >/dev/null 2>&1
        exit 1
    fi
    git -C "${STANDALONE_REPO}" commit -m "${COMMIT_MSG}"
fi

printf "${GREEN}Committed.${RESET}\n"
echo

# --- Push ---------------------------------------------------------------------

if $FORCE; then
    info "Pushing to origin/main..."
    git -C "${STANDALONE_REPO}" push origin main
    ok "Pushed to origin/main"
else
    printf "${YELLOW}Push to origin/main?${RESET} [y/N] "
    # See the matching comment on the commit confirmation above -- same EOF
    # hazard, same guard.
    read -r push_confirm || push_confirm=""
    if [[ "$push_confirm" =~ ^[Yy]$ ]]; then
        git -C "${STANDALONE_REPO}" push origin main
        ok "Pushed to origin/main"
    else
        info "Not pushed. You can push manually:"
        echo "  git -C ${STANDALONE_REPO} push origin main"
    fi
fi
