// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Echo memory: HDC-encoded episodic recall over the live physics state.
//!
//! Wires `symtropy-hdc-physics` (a real, previously-built implementation —
//! deterministic HDC encoding + associative episode memory, with its own
//! passing example — that had zero consumers anywhere in the workspace)
//! into the live game. It samples `PhysicsWorldRes` at a fixed real-time
//! rate, bundles fixed-length windows into `PhysicsEpisode`s via HDC
//! temporal binding, and queries past episodes for the nearest match each
//! time one completes. A high-similarity hit means "this stretch of play
//! strongly resembles an earlier one this run" — surfaced to the player as
//! a console line ("déjà vu").

use bevy::prelude::*;
use symtropy_hdc_physics::{
    EpisodeBuilder, EpisodeMemory, EpisodeMetadata, PhysicsEncoderConfig, PhysicsFrameEncoder,
};

use crate::resources::PhysicsWorldRes;

/// Real-time seconds between physics-frame samples — decouples episode
/// encoding cost from render framerate (16,384-D HDC bundling isn't free
/// enough to redo on every rendered frame at 100+ FPS).
const SAMPLE_INTERVAL_SECS: f32 = 1.0 / 20.0;
/// Samples per episode (~3s of play at the sample rate above).
const EPISODE_LEN: usize = 60;
/// Temporal-position permutation stride, matching the crate's own reference
/// example (`examples/physics_episode_retrieval.rs`) — keeps frame order
/// distinguishable within an episode's bundled vector.
const TEMPORAL_STRIDE: i64 = 101;
/// Below this similarity, a match isn't distinctive enough to surface.
const ECHO_SIMILARITY_THRESHOLD: f32 = 0.65;

#[derive(Resource)]
pub struct EchoMemoryRes {
    encoder: PhysicsFrameEncoder,
    memory: EpisodeMemory,
    builder: Option<EpisodeBuilder>,
    frames_in_episode: usize,
    sample_timer: f32,
    tick: u64,
    episode_index: u64,
}

impl Default for EchoMemoryRes {
    fn default() -> Self {
        let config = PhysicsEncoderConfig::default();
        let encoder =
            PhysicsFrameEncoder::new(config).expect("default PhysicsEncoderConfig is valid");
        let memory = EpisodeMemory::from_encoder(&encoder);
        Self {
            encoder,
            memory,
            builder: None,
            frames_in_episode: 0,
            sample_timer: 0.0,
            tick: 0,
            episode_index: 0,
        }
    }
}

/// Sample the current physics frame at a fixed real-time rate, bundle it
/// into the in-flight episode, and on episode completion query memory for
/// the nearest past episode before inserting the new one.
pub fn echo_memory_system(
    mut echo: ResMut<EchoMemoryRes>,
    physics: Res<PhysicsWorldRes>,
    time: Res<Time>,
) {
    let echo = &mut *echo;
    echo.sample_timer += time.delta_secs();
    if echo.sample_timer < SAMPLE_INTERVAL_SECS {
        return;
    }
    echo.sample_timer -= SAMPLE_INTERVAL_SECS;

    let tick = echo.tick;
    echo.tick += 1;

    let frame = match echo.encoder.encode_world(tick, &physics.world) {
        Ok(frame) => frame,
        Err(e) => {
            // Encoding is deterministic on well-formed state; a failure here
            // means a config mismatch, not a per-tick transient — log once
            // rather than spamming every sample.
            if tick == 0 {
                eprintln!("[echo-memory] frame encode failed, disabling: {e}");
            }
            return;
        }
    };

    if echo.builder.is_none() {
        let episode_id = format!("episode-{}", echo.episode_index);
        let metadata = EpisodeMetadata {
            label: "live-play".to_owned(),
            run_id: episode_id.clone(),
            ..EpisodeMetadata::default()
        };
        echo.builder = Some(
            echo.encoder
                .episode_builder(episode_id, metadata, TEMPORAL_STRIDE),
        );
    }

    let Some(builder) = echo.builder.as_mut() else {
        return;
    };

    if builder.push(&frame).is_err() {
        // Fingerprint drift shouldn't happen from one long-lived encoder
        // instance; drop the in-flight episode rather than propagate a panic.
        echo.builder = None;
        echo.frames_in_episode = 0;
        return;
    }
    echo.frames_in_episode += 1;

    if echo.frames_in_episode < EPISODE_LEN {
        return;
    }

    let Some(builder) = echo.builder.take() else {
        return;
    };
    echo.frames_in_episode = 0;
    echo.episode_index += 1;

    let Ok(episode) = builder.finish() else {
        return;
    };

    if !echo.memory.is_empty() {
        if let Ok(hits) = echo.memory.query_episode(&episode, 1, false) {
            if let Some(hit) = hits.first() {
                if hit.similarity >= ECHO_SIMILARITY_THRESHOLD {
                    println!(
                        "[echo-memory] déjà vu — this moment echoes {} (similarity {:.2})",
                        hit.episode_id, hit.similarity
                    );
                }
            }
        }
    }

    let _ = echo.memory.insert(episode);
}
