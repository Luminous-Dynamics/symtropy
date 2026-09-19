// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reference manipulation checks for SYM-EVAL-001E Q0/Q1.
//!
//! These tests qualify the leak detector against the frozen synthetic
//! SYM-EVAL-001A scaffold. A passing test means the detector recognized the
//! expected clean controls/canaries. It does NOT mean the benchmark is free of
//! leaks or that Symthaea passed an object-permanence task.

use symtropy_vision_eval::{
    BeliefLifecycle, BeliefSample, BeliefTranscript, ObjectPermanencePair, SensorObservationId,
};

const LAST_IDENTICAL_PREFIX_FRAME: usize = 8;
const FIRST_LEGAL_DIVERGENCE_FRAME: usize = 9;
const CHANNEL_INVENTORY: &str =
    include_str!("../../../../docs/research/SYM_EVAL_001E_CHANNEL_INVENTORY_V1.json");

fn first_divergence<T: PartialEq>(left: &[T], right: &[T]) -> Option<usize> {
    let shared = left.len().min(right.len());
    left[..shared]
        .iter()
        .zip(&right[..shared])
        .position(|(lhs, rhs)| lhs != rhs)
        .or_else(|| (left.len() != right.len()).then_some(shared))
}

fn belief_sample(
    frame_index: u64,
    hypothesis_id: u64,
    lifecycle: BeliefLifecycle,
    last_observed_frame: u64,
    source_observation: SensorObservationId,
) -> BeliefSample {
    BeliefSample {
        frame_index,
        hypothesis_id,
        lifecycle,
        last_observed_frame: Some(last_observed_frame),
        source_observation: Some(source_observation),
        persistence_confidence: 0.9,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CanaryFrame {
    path_tag: &'static str,
    timestamp_us: u64,
    observation_id: u64,
    background_pixel: [u8; 4],
}

fn clean_canary_fixture() -> Vec<CanaryFrame> {
    vec![
        CanaryFrame {
            path_tag: "trial/frame-000",
            timestamp_us: 10_000,
            observation_id: 100,
            background_pixel: [12, 24, 48, 255],
        },
        CanaryFrame {
            path_tag: "trial/frame-001",
            timestamp_us: 20_000,
            observation_id: 101,
            background_pixel: [12, 24, 48, 255],
        },
    ]
}

#[test]
fn q0_current_scaffold_exposes_future_divergence_through_public_slice() {
    let pair = ObjectPermanencePair::blinded(7, 7_001);
    let trial_a = pair.trial_a().sensor_frames();
    let trial_b = pair.trial_b().sensor_frames();

    assert!(trial_a.len() > FIRST_LEGAL_DIVERGENCE_FRAME);
    assert_eq!(trial_a.len(), trial_b.len());
    assert_eq!(
        first_divergence(
            &trial_a[..=LAST_IDENTICAL_PREFIX_FRAME],
            &trial_b[..=LAST_IDENTICAL_PREFIX_FRAME],
        ),
        None
    );
    assert_eq!(
        first_divergence(trial_a, trial_b),
        Some(FIRST_LEGAL_DIVERGENCE_FRAME)
    );

    // A producer holding the full public slice can inspect frame 9 while it is
    // supposedly still processing frame 0. In the current synthetic fixture,
    // the Persist branch has an extra target observation at the reveal frame.
    assert_ne!(
        trial_a[FIRST_LEGAL_DIVERGENCE_FRAME].observations.len(),
        trial_b[FIRST_LEGAL_DIVERGENCE_FRAME].observations.len()
    );
}

#[test]
fn q0_frame_count_alone_does_not_classify_current_pair() {
    let pair = ObjectPermanencePair::blinded(17, 17_001);
    let trial_a = pair.trial_a().sensor_frames();
    let trial_b = pair.trial_b().sensor_frames();

    // This is a clean negative control for the simple frame-count heuristic.
    // The horizon is still implicitly exposed by the slice API, but it is
    // branch-independent in the frozen synthetic profile.
    assert_eq!(trial_a.len(), 12);
    assert_eq!(trial_a.len(), trial_b.len());
}

#[test]
fn q1_current_scorer_accepts_future_source_observation_as_reacquisition() {
    let pair = ObjectPermanencePair::blinded(19, 19_001);
    let trial_a = pair.trial_a().sensor_frames();
    let trial_b = pair.trial_b().sensor_frames();

    let trial_a_is_persist = trial_a[FIRST_LEGAL_DIVERGENCE_FRAME].observations.len()
        > trial_b[FIRST_LEGAL_DIVERGENCE_FRAME].observations.len();
    let (persist_frames, remove_frames) = if trial_a_is_persist {
        (trial_a, trial_b)
    } else {
        (trial_b, trial_a)
    };

    let persist_prefix_observation = persist_frames[4]
        .observations
        .iter()
        .find(|blob| blob.visible_fraction < 1.0)
        .expect("frame 4 must contain the partially visible target")
        .observation_id;
    let persist_future_observation = persist_frames[FIRST_LEGAL_DIVERGENCE_FRAME]
        .observations
        .iter()
        .find(|blob| blob.x_norm > 0.5)
        .expect("frame 9 must contain the reappearing target")
        .observation_id;
    let remove_prefix_observation = remove_frames[4]
        .observations
        .iter()
        .find(|blob| blob.visible_fraction < 1.0)
        .expect("frame 4 must contain the partially visible target")
        .observation_id;

    // This transcript is intentionally impossible in a causal stream: the
    // frame-6 sample cites an observation that does not exist until frame 9.
    // `last_observed_frame` remains 4, so the current freshness guard does not
    // reject it as an unsupported refresh.
    let persist = BeliefTranscript::new(vec![
        belief_sample(
            4,
            42,
            BeliefLifecycle::PartiallyOccluded,
            4,
            persist_prefix_observation,
        ),
        belief_sample(
            6,
            42,
            BeliefLifecycle::Visible,
            4,
            persist_future_observation,
        ),
    ])
    .expect("current v1 validation permits the temporal-lineage canary");
    let remove = BeliefTranscript::new(vec![belief_sample(
        4,
        42,
        BeliefLifecycle::PartiallyOccluded,
        4,
        remove_prefix_observation,
    )])
    .expect("control transcript must remain valid");

    let (trial_a_beliefs, trial_b_beliefs) = if trial_a_is_persist {
        (persist, remove)
    } else {
        (remove, persist)
    };
    let assessment = pair.assess(&trial_a_beliefs, &trial_b_beliefs);

    // Detector PASS: the frozen scorer currently interprets the future lineage
    // as successful reacquisition and does not classify it as unknown,
    // distractor-associated, or unsupported freshness. SYM-EVAL-001A3 must
    // make this canary fail closed.
    assert_eq!(assessment.persist.reacquired_same_hypothesis, Some(true));
    assert!(!assessment.persist.unknown_source_observation);
    assert!(!assessment.persist.cross_associated_non_target_observation);
    assert!(!assessment.persist.unsupported_observation_refresh);
}

#[test]
fn q1_clean_metadata_fixture_has_no_branch_signal() {
    let clean_a = clean_canary_fixture();
    let clean_b = clean_canary_fixture();

    assert_eq!(first_divergence(&clean_a, &clean_b), None);
}

#[test]
fn q1_probe_detects_path_label_canary() {
    let clean = clean_canary_fixture();
    let mut contaminated = clean_canary_fixture();
    contaminated[0].path_tag = "trial/persist/frame-000";

    assert_eq!(first_divergence(&clean, &contaminated), Some(0));
}

#[test]
fn q1_probe_detects_timestamp_canary() {
    let clean = clean_canary_fixture();
    let mut contaminated = clean_canary_fixture();
    contaminated[0].timestamp_us += 1;

    assert_eq!(first_divergence(&clean, &contaminated), Some(0));
}

#[test]
fn q1_probe_detects_observation_id_canary() {
    let clean = clean_canary_fixture();
    let mut contaminated = clean_canary_fixture();
    contaminated[0].observation_id += 1;

    assert_eq!(first_divergence(&clean, &contaminated), Some(0));
}

#[test]
fn q1_probe_detects_background_pixel_canary() {
    let clean = clean_canary_fixture();
    let mut contaminated = clean_canary_fixture();
    contaminated[0].background_pixel[0] ^= 1;

    assert_eq!(first_divergence(&clean, &contaminated), Some(0));
}

#[test]
fn q0_inventory_binds_known_leak_and_nonclaims() {
    assert!(CHANNEL_INVENTORY.contains(
        r#""evaluated_subject_sha": "14dd4b15bef736a5d73106036c5b92d17e83790a""#
    ));
    assert!(CHANNEL_INVENTORY.contains(r#""channel_id": "trial.full_future_sequence""#));
    assert!(CHANNEL_INVENTORY.contains(r#""current_status": "LeakDetected""#));
    assert!(CHANNEL_INVENTORY.contains(r#""future_source_observation_lineage""#));
    assert!(CHANNEL_INVENTORY.contains(
        r#""benchmark_pass": "Not established by this profile.""#
    ));
    assert!(CHANNEL_INVENTORY.contains(
        r#""model_performance": "Not evaluated by this profile.""#
    ));
    assert!(!CHANNEL_INVENTORY.contains(r#""online_benchmark_validity": "LeakChecksPass""#));
}
