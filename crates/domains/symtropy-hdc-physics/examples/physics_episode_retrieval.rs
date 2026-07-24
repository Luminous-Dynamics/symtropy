// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Reproducible HDC physics-episode retrieval baseline.
//!
//! Emits CSV to stdout. It deliberately uses the exact linear-scan memory so
//! retrieval accuracy measures the encoder rather than an approximate index.

use symtropy_hdc_physics::{
    EpisodeMemory, EpisodeMetadata, IdentityPolicy, PhysicsEncoderConfig, PhysicsEpisode,
    PhysicsFrameEncoder, ReferenceFramePolicy,
};
use symtropy_math::Point;
use symtropy_physics::PhysicsWorld;

const DT: f64 = 1.0 / 60.0;
const FRAMES: usize = 48;
const TEMPORAL_STRIDE: i64 = 101;

#[derive(Clone, Copy)]
enum Scenario {
    DriftRight,
    DriftLeft,
    Fall,
    HeadOn,
}

impl Scenario {
    fn label(self) -> &'static str {
        match self {
            Self::DriftRight => "drift-right",
            Self::DriftLeft => "drift-left",
            Self::Fall => "fall",
            Self::HeadOn => "head-on",
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = PhysicsEncoderConfig::default();
    config.hdc.dimension = 16_384;
    config.hdc.scalar_levels = 257;
    config.hdc.seed = 0x4844_4350_4859_5331;
    config.identity_policy = IdentityPolicy::None;
    config.reference_frame = ReferenceFramePolicy::CenterOfDynamicMass;
    let encoder = PhysicsFrameEncoder::new(config)?;

    let scenarios = [
        Scenario::DriftRight,
        Scenario::DriftLeft,
        Scenario::Fall,
        Scenario::HeadOn,
    ];

    let mut memory = EpisodeMemory::from_encoder(&encoder);
    for (index, scenario) in scenarios.iter().copied().enumerate() {
        memory.insert(capture_episode(
            &encoder,
            scenario,
            1.0,
            format!("train-{index}"),
            "train",
        )?)?;
    }

    println!(
        "query_id,true_label,predicted_label,similarity,correct,novelty,encoder_fingerprint,first_digest"
    );
    let mut correct = 0_usize;
    let mut total = 0_usize;
    for (scenario_index, scenario) in scenarios.iter().copied().enumerate() {
        for perturbation_index in 0..5 {
            let scale = 0.82 + perturbation_index as f64 * 0.09;
            let query_id = format!("query-{scenario_index}-{perturbation_index}");
            let query = capture_episode(&encoder, scenario, scale, query_id.clone(), "query")?;
            let hit = memory
                .query_episode(&query, 1, false)?
                .into_iter()
                .next()
                .expect("training memory is non-empty");
            let is_correct = hit.label == scenario.label();
            correct += usize::from(is_correct);
            total += 1;
            let novelty = memory.novelty(&query.vector)?;
            println!(
                "{query_id},{},{},{:.6},{is_correct},{novelty:.6},{:016x},{}",
                scenario.label(),
                hit.label,
                hit.similarity,
                query.encoder_fingerprint,
                query.exact_digests[0],
            );
        }
    }
    eprintln!(
        "summary,total={total},correct={correct},accuracy={:.6},encoder={:016x}",
        correct as f64 / total as f64,
        encoder.config().fingerprint(),
    );
    Ok(())
}

fn capture_episode(
    encoder: &PhysicsFrameEncoder,
    scenario: Scenario,
    scale: f64,
    id: String,
    split: &str,
) -> Result<PhysicsEpisode, Box<dyn std::error::Error>> {
    let mut world = make_world(scenario, scale);
    let mut metadata = EpisodeMetadata {
        label: scenario.label().to_owned(),
        run_id: id.clone(),
        seed: 0,
        ..EpisodeMetadata::default()
    };
    metadata.tags.insert("split".to_owned(), split.to_owned());
    metadata
        .tags
        .insert("scale".to_owned(), format!("{scale:.6}"));
    let mut builder = encoder.episode_builder(id, metadata, TEMPORAL_STRIDE);
    for tick in 0..FRAMES {
        let frame = encoder.encode_world(tick as u64, &world)?;
        builder.push(&frame)?;
        world.step(DT);
    }
    Ok(builder.finish()?)
}

fn make_world(scenario: Scenario, scale: f64) -> PhysicsWorld<3> {
    match scenario {
        Scenario::DriftRight => {
            let mut world = PhysicsWorld::default();
            let body = world.add_sphere(Point::origin(), 0.4, 1.0);
            world.body_mut(body).unwrap().linear_velocity[0] = 2.0 * scale;
            world
        }
        Scenario::DriftLeft => {
            let mut world = PhysicsWorld::default();
            let body = world.add_sphere(Point::origin(), 0.4, 1.0);
            world.body_mut(body).unwrap().linear_velocity[0] = -2.0 * scale;
            world
        }
        Scenario::Fall => {
            let mut world = PhysicsWorld::new(nalgebra::SVector::from([0.0, -9.81 * scale, 0.0]));
            world.add_sphere(Point::new([0.0, 8.0, 0.0]), 0.4, 1.0);
            world
        }
        Scenario::HeadOn => {
            let mut world = PhysicsWorld::default();
            let left = world.add_sphere(Point::new([-1.2, 0.0, 0.0]), 0.4, 1.0);
            let right = world.add_sphere(Point::new([1.2, 0.0, 0.0]), 0.4, 1.0);
            world.body_mut(left).unwrap().linear_velocity[0] = 2.5 * scale;
            world.body_mut(right).unwrap().linear_velocity[0] = -2.5 * scale;
            world
        }
    }
}
