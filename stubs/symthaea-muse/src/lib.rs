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
