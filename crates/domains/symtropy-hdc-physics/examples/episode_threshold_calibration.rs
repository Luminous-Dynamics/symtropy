// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Held-out calibration of HDC episode similarity and abstention.
//!
//! Uses the real PhysicsFrameEncoder / EpisodeMemory over physical-scene
//! trajectories. Threshold selection sees calibration labels only; evaluation
//! queries are scored after the threshold is frozen. The default runtime cutoff
//! (0.65) is a comparator and is not changed by this example.
//!
//! cargo run --locked --package symtropy-hdc-physics --example episode_threshold_calibration
//! cargo test --locked --package symtropy-hdc-physics --example episode_threshold_calibration

use symtropy_hdc_physics::{
    EpisodeMemory, EpisodeMetadata, IdentityPolicy, PhysicsEncoderConfig, PhysicsEpisode,
    PhysicsFrameEncoder, ReferenceFramePolicy,
};
use symtropy_math::Point;
use symtropy_physics::PhysicsWorld;

const DT: f64 = 1.0 / 60.0;
const FRAMES: usize = 48;
const TEMPORAL_STRIDE: i64 = 101;
const CURRENT_THRESHOLD: f32 = 0.65;
const FALSE_MATCH_COST: usize = 4;
const FALSE_REJECT_COST: usize = 1;
const THRESHOLD_STEPS: u16 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scenario {
    DriftRight,
    DriftLeft,
    Fall,
    HeadOn,
    VerticalDrift,
    StaticPair,
}

impl Scenario {
    const BANK: [Self; 4] = [
        Self::DriftRight,
        Self::DriftLeft,
        Self::Fall,
        Self::HeadOn,
    ];

    const NOVEL: [Self; 2] = [Self::VerticalDrift, Self::StaticPair];

    fn label(self) -> &'static str {
        match self {
            Self::DriftRight => "drift-right",
            Self::DriftLeft => "drift-left",
            Self::Fall => "fall",
            Self::HeadOn => "head-on",
            Self::VerticalDrift => "novel-vertical-drift",
            Self::StaticPair => "novel-static-pair",
        }
    }

    fn in_bank(self) -> bool {
        Self::BANK.contains(&self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Split {
    Calibration,
    Evaluation,
}

impl Split {
    fn label(self) -> &'static str {
        match self {
            Self::Calibration => "calibration",
            Self::Evaluation => "evaluation",
        }
    }
}

#[derive(Clone, Debug)]
struct QueryCase {
    id: String,
    split: Split,
    scenario: Scenario,
    scale: f64,
    episode: PhysicsEpisode,
}

#[derive(Clone, Debug)]
struct ScoredQuery {
    id: String,
    split: Split,
    true_label: &'static str,
    label_is_stored: bool,
    nearest_episode: String,
    predicted_label: String,
    similarity: f32,
    useful_if_accepted: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ThresholdMetrics {
    threshold: f32,
    queries: usize,
    accepted: usize,
    useful: usize,
    harmful: usize,
    false_rejects: usize,
    cost: usize,
}

impl ThresholdMetrics {
    fn from_queries(queries: &[ScoredQuery], threshold: f32) -> Self {
        let mut result = Self {
            threshold,
            queries: queries.len(),
            accepted: 0,
            useful: 0,
            harmful: 0,
            false_rejects: 0,
            cost: 0,
        };
        for query in queries {
            let accepted = query.similarity >= threshold;
            if accepted {
                result.accepted += 1;
                if query.useful_if_accepted {
                    result.useful += 1;
                } else {
                    result.harmful += 1;
                }
            } else if query.label_is_stored {
                result.false_rejects += 1;
            }
        }
        result.cost = result.harmful * FALSE_MATCH_COST
            + result.false_rejects * FALSE_REJECT_COST;
        result
    }

    fn coverage(self) -> f64 {
        ratio(self.accepted, self.queries)
    }

    fn risk_among_accepted(self) -> f64 {
        ratio(self.harmful, self.accepted)
    }
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn build_encoder() -> Result<PhysicsFrameEncoder, Box<dyn std::error::Error>> {
    let mut config = PhysicsEncoderConfig::default();
    config.hdc.dimension = 16_384;
    config.hdc.scalar_levels = 257;
    config.hdc.seed = 0x4844_4350_4859_5331;
    config.identity_policy = IdentityPolicy::None;
    config.reference_frame = ReferenceFramePolicy::CenterOfDynamicMass;
    // Bound quantization to this controlled scene's scales so the calibration
    // measures HDC similarity rather than values collapsing into one bin.
    config.position_extent = 20.0;
    config.velocity_extent = 10.0;
    config.angular_velocity_extent = 10.0;
    config.energy_extent = 1.0e6;
    Ok(PhysicsFrameEncoder::new(config)?)
}

fn capture_episode(
    encoder: &PhysicsFrameEncoder,
    scenario: Scenario,
    scale: f64,
    split: Split,
    id: String,
) -> Result<PhysicsEpisode, Box<dyn std::error::Error>> {
    let mut world = make_world(scenario, scale);
    let mut metadata = EpisodeMetadata {
        label: scenario.label().to_owned(),
        run_id: format!("episode-threshold-calibration-{}", split.label()),
        seed: 0,
        ..EpisodeMetadata::default()
    };
    metadata.tags.insert("split".to_owned(), split.label().to_owned());
    metadata.tags.insert("scenario".to_owned(), scenario.label().to_owned());
    metadata.tags.insert("scale".to_owned(), format!("{scale:.6}"));

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
        Scenario::VerticalDrift => {
            let mut world = PhysicsWorld::default();
            let body = world.add_sphere(Point::origin(), 0.4, 1.0);
            world.body_mut(body).unwrap().linear_velocity[1] = 2.0 * scale;
            world
        }
        Scenario::StaticPair => {
            let mut world = PhysicsWorld::default();
            world.add_sphere(Point::new([-2.0 * scale, 0.0, 0.0]), 0.4, 1.0);
            world.add_sphere(Point::new([2.0 * scale, 0.0, 0.0]), 0.4, 1.0);
            world
        }
    }
}

fn query_cases(
    encoder: &PhysicsFrameEncoder,
    split: Split,
) -> Result<Vec<QueryCase>, Box<dyn std::error::Error>> {
    let seen_scales: &[f64] = match split {
        Split::Calibration => &[0.72, 0.88, 1.12, 1.28],
        Split::Evaluation => &[0.65, 0.95, 1.05, 1.35],
    };
    let novel_scales: &[f64] = match split {
        Split::Calibration => &[0.80, 1.20],
        Split::Evaluation => &[0.90, 1.10],
    };
    let mut cases = Vec::new();
    for scenario in Scenario::BANK {
        for scale in seen_scales {
            let id = format!("{}-{}-{:.2}", split.label(), scenario.label(), scale);
            let episode = capture_episode(encoder, scenario, *scale, split, id.clone())?;
            cases.push(QueryCase {
                id,
                split,
                scenario,
                scale: *scale,
                episode,
            });
        }
    }
    for scenario in Scenario::NOVEL {
        for scale in novel_scales {
            let id = format!("{}-{}-{:.2}", split.label(), scenario.label(), scale);
            let episode = capture_episode(encoder, scenario, *scale, split, id.clone())?;
            cases.push(QueryCase {
                id,
                split,
                scenario,
                scale: *scale,
                episode,
            });
        }
    }
    Ok(cases)
}

fn score_queries(
    memory: &EpisodeMemory,
    cases: &[QueryCase],
) -> Result<Vec<ScoredQuery>, Box<dyn std::error::Error>> {
    let mut scored = Vec::with_capacity(cases.len());
    for case in cases {
        let hit = memory
            .query_episode(&case.episode, 1, false)?
            .into_iter()
            .next()
            .expect("calibration memory contains four prototypes");
        scored.push(ScoredQuery {
            id: case.id.clone(),
            split: case.split,
            true_label: case.scenario.label(),
            label_is_stored: case.scenario.in_bank(),
            nearest_episode: hit.episode_id,
            predicted_label: hit.label,
            similarity: hit.similarity,
            useful_if_accepted: case.scenario.in_bank()
                && hit.label == case.scenario.label(),
        });
    }
    Ok(scored)
}

fn choose_threshold(calibration: &[ScoredQuery]) -> ThresholdMetrics {
    let mut best: Option<ThresholdMetrics> = None;
    for step in 0..=THRESHOLD_STEPS {
        let threshold = f32::from(step) / f32::from(THRESHOLD_STEPS);
        let metrics = ThresholdMetrics::from_queries(calibration, threshold);
        let replace = best.is_none_or(|current| {
            metrics.cost < current.cost
                || (metrics.cost == current.cost && metrics.threshold > current.threshold)
        });
        if replace {
            best = Some(metrics);
        }
    }
    best.expect("threshold grid contains at least one candidate")
}

fn fnv64(text: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn push(line: &mut String, fields: &[String]) {
    line.push_str(&fields.join("\t"));
    line.push('\n');
}

fn flag(value: bool) -> String {
    if value { "1".to_owned() } else { "0".to_owned() }
}

fn report() -> Result<String, Box<dyn std::error::Error>> {
    let encoder = build_encoder()?;
    let fingerprint = encoder.config().fingerprint();
    let mut memory = EpisodeMemory::from_encoder(&encoder);
    for scenario in Scenario::BANK {
        let id = format!("bank-{}", scenario.label());
        memory.insert(capture_episode(&encoder, scenario, 1.0, Split::Calibration, id)?)?;
    }
    let calibration = score_queries(&memory, &query_cases(&encoder, Split::Calibration)?)?;
    let evaluation = score_queries(&memory, &query_cases(&encoder, Split::Evaluation)?)?;
    let selected = choose_threshold(&calibration);
    let selected_threshold = selected.threshold;

    let mut out = String::new();
    push(&mut out, &["META".into(), "format".into(), "HDC-THRESHOLD-CAL-001-v0.1".into()]);
    push(&mut out, &["META".into(), "scope".into(), "real physics-encoder episode trajectories; threshold calibration only, not robot performance".into()]);
    push(&mut out, &["META".into(), "encoder_fingerprint".into(), format!("{fingerprint:016x}")]);
    push(&mut out, &["META".into(), "dimension".into(), encoder.config().hdc.dimension.to_string()]);
    push(&mut out, &["META".into(), "bank_size".into(), memory.len().to_string()]);
    push(&mut out, &["META".into(), "current_threshold".into(), format!("{CURRENT_THRESHOLD:.2}")]);
    push(&mut out, &["META".into(), "false_match_cost".into(), FALSE_MATCH_COST.to_string()]);
    push(&mut out, &["META".into(), "false_reject_cost".into(), FALSE_REJECT_COST.to_string()]);
    push(&mut out, &["META".into(), "selected_threshold".into(), format!("{selected_threshold:.2}")]);
    push(&mut out, &["META".into(), "threshold_selection".into(), "calibration labels only; minimize false_match_cost*harmful_accepts + false_reject_cost*known_class_rejects; ties prefer higher threshold".into()]);
    push(&mut out, &["META".into(), "calibration_split_fnv64".into(), format!("{:016x}", fnv64(&calibration.iter().map(|q| format!("{}:{}:{:.6}", q.id, q.true_label, q.similarity)).collect::<Vec<_>>().join("|")))]);
    push(&mut out, &["META".into(), "evaluation_split_fnv64".into(), format!("{:016x}", fnv64(&evaluation.iter().map(|q| format!("{}:{}:{:.6}", q.id, q.true_label, q.similarity)).collect::<Vec<_>>().join("|")))]);
    push(&mut out, &["META".into(), "hash_note".into(), "FNV64 split fingerprints are deterministic non-cryptographic identifiers; artifact SHA-256 is emitted by CI".into()]);

    push(&mut out, &[
        "THRESHOLD_HEADER".into(), "threshold".into(), "queries".into(), "accepted".into(),
        "useful".into(), "harmful".into(), "false_rejects".into(), "coverage".into(),
        "risk_accepted".into(), "weighted_cost".into(),
    ]);
    for step in 0..=THRESHOLD_STEPS {
        let threshold = f32::from(step) / f32::from(THRESHOLD_STEPS);
        let m = ThresholdMetrics::from_queries(&calibration, threshold);
        push(&mut out, &[
            "THRESHOLD".into(), format!("{:.2}", m.threshold), m.queries.to_string(),
            m.accepted.to_string(), m.useful.to_string(), m.harmful.to_string(),
            m.false_rejects.to_string(), format!("{:.3}", m.coverage()),
            format!("{:.3}", m.risk_among_accepted()), m.cost.to_string(),
        ]);
    }

    push(&mut out, &[
        "QUERY_HEADER".into(), "split".into(), "query_id".into(), "true_label".into(),
        "label_is_stored".into(), "nearest_episode".into(), "predicted_label".into(),
        "similarity".into(), "accepted_at_065".into(), "useful_at_065".into(),
        "accepted_at_selected".into(), "useful_at_selected".into(), "harmful_at_selected".into(),
    ]);
    for query in calibration.iter().chain(evaluation.iter()) {
        let accepted_065 = query.similarity >= CURRENT_THRESHOLD;
        let accepted_selected = query.similarity >= selected_threshold;
        push(&mut out, &[
            "QUERY".into(), query.split.label().into(), query.id.clone(),
            query.true_label.into(), flag(query.label_is_stored), query.nearest_episode.clone(),
            query.predicted_label.clone(), format!("{:.6}", query.similarity), flag(accepted_065),
            flag(accepted_065 && query.useful_if_accepted), flag(accepted_selected),
            flag(accepted_selected && query.useful_if_accepted),
            flag(accepted_selected && !query.useful_if_accepted),
        ]);
    }

    push(&mut out, &[
        "PAIR_HEADER".into(), "split".into(), "query_id".into(), "query_label".into(),
        "query_label_is_stored".into(), "candidate_episode".into(), "candidate_label".into(),
        "similarity".into(), "positive_pair".into(),
    ]);
    for query in calibration.iter().chain(evaluation.iter()) {
        let case = calibration.iter().chain(evaluation.iter())
            .find(|case| case.id == query.id).expect("query maps to a query case");
        for candidate in memory.episodes() {
            let similarity = case.episode.vector.similarity(&candidate.vector)?;
            let positive_pair = query.label_is_stored && query.true_label == candidate.metadata.label;
            push(&mut out, &[
                "PAIR".into(), query.split.label().into(), query.id.clone(),
                query.true_label.into(), flag(query.label_is_stored), candidate.id.clone(),
                candidate.metadata.label.clone(), format!("{similarity:.6}"), flag(positive_pair),
            ]);
        }
    }

    push(&mut out, &[
        "SUMMARY_HEADER".into(), "split".into(), "threshold_source".into(), "threshold".into(),
        "queries".into(), "accepted".into(), "useful".into(), "harmful".into(),
        "false_rejects".into(), "coverage".into(), "risk_accepted".into(), "weighted_cost".into(),
    ]);
    for (split_name, queries) in [("calibration", calibration.as_slice()), ("evaluation", evaluation.as_slice())] {
        for (source, threshold) in [("fixed_0.65", CURRENT_THRESHOLD), ("calibrated", selected_threshold)] {
            let m = ThresholdMetrics::from_queries(queries, threshold);
            push(&mut out, &[
                "SUMMARY".into(), split_name.into(), source.into(), format!("{:.2}", threshold),
                m.queries.to_string(), m.accepted.to_string(), m.useful.to_string(),
                m.harmful.to_string(), m.false_rejects.to_string(), format!("{:.3}", m.coverage()),
                format!("{:.3}", m.risk_among_accepted()), m.cost.to_string(),
            ]);
        }
    }
    Ok(out)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print!("{}", report()?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_choice_uses_calibration_data_only_and_prefers_higher_ties() {
        let calibration = vec![
            ScoredQuery { id:"a".into(), split:Split::Calibration, true_label:"known", label_is_stored:true, nearest_episode:"a".into(), predicted_label:"known".into(), similarity:0.80, useful_if_accepted:true },
            ScoredQuery { id:"b".into(), split:Split::Calibration, true_label:"novel", label_is_stored:false, nearest_episode:"a".into(), predicted_label:"known".into(), similarity:0.70, useful_if_accepted:false },
            ScoredQuery { id:"c".into(), split:Split::Calibration, true_label:"known", label_is_stored:true, nearest_episode:"x".into(), predicted_label:"wrong".into(), similarity:0.60, useful_if_accepted:false },
        ];
        let selected = choose_threshold(&calibration);
        assert!(selected.threshold >= 0.70);
        assert!(selected.cost <= ThresholdMetrics::from_queries(&calibration, CURRENT_THRESHOLD).cost);
    }

    #[test]
    fn novel_query_accepted_above_threshold_is_counted_as_harmful_not_useful() {
        let query = ScoredQuery {
            id:"novel".into(), split:Split::Evaluation, true_label:"novel", label_is_stored:false,
            nearest_episode:"bank-a".into(), predicted_label:"bank-a".into(), similarity:0.90,
            useful_if_accepted:false
        };
        let metrics = ThresholdMetrics::from_queries(&[query],0.65);
        assert_eq!(metrics.accepted,1);
        assert_eq!(metrics.harmful,1);
        assert_eq!(metrics.useful,0);
    }

    #[test]
    fn every_nonbank_scenario_is_a_negative_against_every_bank_episode() {
        assert!(Scenario::NOVEL.iter().all(|scenario| !scenario.in_bank()));
        assert_eq!(Scenario::BANK.len(),4);
        assert_eq!(Scenario::NOVEL.len(),2);
    }

    #[test]
    fn report_replays_with_exact_threshold_query_and_pair_counts() {
        let first=report().unwrap();
        assert_eq!(first,report().unwrap());
        assert_eq!(first.lines().filter(|line|line.starts_with("THRESHOLD\t")).count(),21);
        assert_eq!(first.lines().filter(|line|line.starts_with("QUERY\t")).count(),40);
        assert_eq!(first.lines().filter(|line|line.starts_with("PAIR\t")).count(),160);
        assert_eq!(first.lines().filter(|line|line.starts_with("SUMMARY\t")).count(),4);
        for line in first.lines() {
            if line.starts_with("THRESHOLD_HEADER\t") || line.starts_with("THRESHOLD\t") {
                assert_eq!(line.split('\t').count(),10,"threshold schema: {line}");
            }
            if line.starts_with("QUERY_HEADER\t") || line.starts_with("QUERY\t") {
                assert_eq!(line.split('\t').count(),13,"query schema: {line}");
            }
            if line.starts_with("PAIR_HEADER\t") || line.starts_with("PAIR\t") {
                assert_eq!(line.split('\t').count(),9,"pair schema: {line}");
            }
            if line.starts_with("SUMMARY_HEADER\t") || line.starts_with("SUMMARY\t") {
                assert_eq!(line.split('\t').count(),12,"summary schema: {line}");
            }
        }
    }
}
