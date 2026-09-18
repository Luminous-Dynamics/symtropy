// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Passive hidden-ground-truth evaluation protocols for visual cognition.
//!
//! Canonical simulation identity and intervention truth remain evaluator-private.
//! The system under evaluation receives only blind A/B sensor frames. Post-hoc
//! scoring recovers the target hypothesis from the system's own sensor-observation
//! lineage; no canonical-to-belief association is ever supplied to cognition.

#![deny(unsafe_code)]

/// Frame-local sensor observation identity.
///
/// This identifies one sensor record, not a canonical simulation entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SensorObservationId(pub u64);

/// A small stand-in for a future rendered/perceptual observation.
///
/// Only sensor-facing quantities are exposed. There is no canonical entity id,
/// hidden branch label, scenario phase, intervention marker, or oracle association.
#[derive(Debug, Clone, PartialEq)]
pub struct SensorBlob {
    pub observation_id: SensorObservationId,
    pub x_norm: f32,
    pub y_norm: f32,
    pub visible_fraction: f32,
    pub appearance: [f32; 4],
}

/// Everything the evaluated system may receive for one synthetic frame.
#[derive(Debug, Clone, PartialEq)]
pub struct SensorFrame {
    pub frame_index: u64,
    pub observations: Vec<SensorBlob>,
}

/// Belief-layer lifecycle vocabulary used by the passive scorer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeliefLifecycle {
    Visible,
    PartiallyOccluded,
    Unobserved,
    OccludedPredicted,
    Lost,
    Retired,
}

/// One exported belief sample from the evaluated system.
///
/// `hypothesis_id` belongs to the evaluated system. `source_observation` is optional
/// because prediction-only samples must not invent a current observation merely to
/// keep a hypothesis alive.
#[derive(Debug, Clone, PartialEq)]
pub struct BeliefSample {
    pub frame_index: u64,
    pub hypothesis_id: u64,
    pub lifecycle: BeliefLifecycle,
    pub last_observed_frame: Option<u64>,
    pub source_observation: Option<SensorObservationId>,
    pub persistence_confidence: f32,
}

impl BeliefSample {
    pub fn validate(&self) -> bool {
        self.persistence_confidence.is_finite()
            && (0.0..=1.0).contains(&self.persistence_confidence)
            && self
                .last_observed_frame
                .is_none_or(|last| last <= self.frame_index)
    }
}

/// Ordered belief transcript supplied only after a trial completes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BeliefTranscript {
    samples: Vec<BeliefSample>,
}

impl BeliefTranscript {
    pub fn new(mut samples: Vec<BeliefSample>) -> Result<Self, EvaluationError> {
        if samples.iter().any(|sample| !sample.validate()) {
            return Err(EvaluationError::InvalidBeliefSample);
        }
        samples.sort_by_key(|sample| sample.frame_index);
        Ok(Self { samples })
    }

    pub fn samples(&self) -> &[BeliefSample] {
        &self.samples
    }
}

/// Scenario-level object-permanence measurements.
///
/// These remain separate evidence dimensions rather than an aggregate score.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectPermanenceMetrics {
    pub target_hypothesis: Option<u64>,
    pub prediction_used_during_occlusion: bool,
    pub last_observation_not_refreshed_by_prediction: Option<bool>,
    pub unsupported_observation_refresh: bool,
    pub unknown_source_observation: bool,
    pub cross_associated_non_target_observation: bool,
    pub reacquired_same_hypothesis: Option<bool>,
    pub removed_target_resolved_by_deadline: Option<bool>,
    pub prediction_overran_lost_deadline: Option<bool>,
}

/// Assessment of the hidden paired intervention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedObjectPermanenceAssessment {
    pub visible_prefix_identical: bool,
    pub persist: ObjectPermanenceMetrics,
    pub remove: ObjectPermanenceMetrics,
}

/// A deterministic-sensor, branch-blinded pair of object-permanence trials.
///
/// `scenario_seed` controls only legal sensor/world content. `blinding_nonce` controls
/// which public A/B slot receives Persist versus Remove and must remain evaluator-private
/// until the evaluated system has committed both transcripts.
pub struct ObjectPermanencePair {
    trial_a: ObjectPermanenceTrial,
    trial_b: ObjectPermanenceTrial,
    branch_a: HiddenBranch,
}

impl ObjectPermanencePair {
    pub fn blinded(scenario_seed: u64, blinding_nonce: u64) -> Self {
        let target = CanonicalEntityId(nonzero_mix(scenario_seed ^ 0xA11C_E001));
        let distractor = CanonicalEntityId(nonzero_mix(scenario_seed ^ 0xD157_AC70));
        let branch_a = if nonzero_mix(blinding_nonce ^ 0xB11D_A11B) & 1 == 0 {
            HiddenBranch::Persist
        } else {
            HiddenBranch::Remove
        };
        let branch_b = branch_a.opposite();
        Self {
            trial_a: ObjectPermanenceTrial::build(
                scenario_seed,
                target,
                distractor,
                branch_a,
            ),
            trial_b: ObjectPermanenceTrial::build(
                scenario_seed,
                target,
                distractor,
                branch_b,
            ),
            branch_a,
        }
    }

    /// Blind trial A. No branch, seed, nonce, or oracle accessor is exposed.
    pub fn trial_a(&self) -> &ObjectPermanenceTrial {
        &self.trial_a
    }

    /// Blind trial B. No branch, seed, nonce, or oracle accessor is exposed.
    pub fn trial_b(&self) -> &ObjectPermanenceTrial {
        &self.trial_b
    }

    pub fn visible_prefix_identical(&self) -> bool {
        let end = self.trial_a.oracle.last_identical_sensor_frame;
        self.trial_a
            .sensor_frames
            .iter()
            .filter(|frame| frame.frame_index <= end)
            .eq(self
                .trial_b
                .sensor_frames
                .iter()
                .filter(|frame| frame.frame_index <= end))
    }

    /// Assess blind A/B transcripts and restore Persist/Remove labels only in the
    /// evaluator result, after both transcripts already exist.
    pub fn assess(
        &self,
        trial_a_beliefs: &BeliefTranscript,
        trial_b_beliefs: &BeliefTranscript,
    ) -> PairedObjectPermanenceAssessment {
        let a = self.trial_a.assess(trial_a_beliefs);
        let b = self.trial_b.assess(trial_b_beliefs);
        let (persist, remove) = match self.branch_a {
            HiddenBranch::Persist => (a, b),
            HiddenBranch::Remove => (b, a),
        };
        PairedObjectPermanenceAssessment {
            visible_prefix_identical: self.visible_prefix_identical(),
            persist,
            remove,
        }
    }
}

/// One blind branch of the paired protocol.
///
/// The private oracle and branch truth have no public accessor or `Debug` surface.
/// The evaluated system should receive only `sensor_frames()`.
pub struct ObjectPermanenceTrial {
    branch: HiddenBranch,
    sensor_frames: Vec<SensorFrame>,
    oracle: HiddenOracle,
}

impl ObjectPermanenceTrial {
    fn build(
        scenario_seed: u64,
        target: CanonicalEntityId,
        distractor: CanonicalEntityId,
        branch: HiddenBranch,
    ) -> Self {
        let mut sensor_frames = Vec::new();
        let mut target_sensor_observations = Vec::new();
        let mut non_target_sensor_observations = Vec::new();

        for frame_index in 0..=11 {
            let mut observations = Vec::new();

            let distractor_observation = SensorObservationId(observation_id(
                scenario_seed,
                frame_index,
                1,
            ));
            observations.push(SensorBlob {
                observation_id: distractor_observation,
                x_norm: 0.18,
                y_norm: 0.64,
                visible_fraction: 1.0,
                appearance: [0.22, 0.71, 0.36, 0.58],
            });
            non_target_sensor_observations.push(distractor_observation);

            let target_visible = frame_index <= 4
                || (branch == HiddenBranch::Persist && frame_index >= 9);
            if target_visible {
                let target_observation = SensorObservationId(observation_id(
                    scenario_seed,
                    frame_index,
                    2,
                ));
                let visible_fraction = if frame_index == 4 { 0.35 } else { 1.0 };
                let x_norm = if frame_index >= 9 { 0.74 } else { 0.62 };
                observations.push(SensorBlob {
                    observation_id: target_observation,
                    x_norm,
                    y_norm: 0.52,
                    visible_fraction,
                    appearance: [0.81, 0.19, 0.27, 0.66],
                });
                target_sensor_observations.push((frame_index, target_observation));
            }

            sensor_frames.push(SensorFrame {
                frame_index,
                observations,
            });
        }

        Self {
            branch,
            sensor_frames,
            oracle: HiddenOracle {
                target,
                distractor,
                last_visible_prefix_frame: 4,
                first_fully_occluded_frame: 5,
                hidden_intervention_frame: 7,
                last_identical_sensor_frame: 8,
                reveal_frame: 9,
                lost_deadline_frame: 11,
                target_sensor_observations,
                non_target_sensor_observations,
            },
        }
    }

    pub fn sensor_frames(&self) -> &[SensorFrame] {
        &self.sensor_frames
    }

    fn assess(&self, beliefs: &BeliefTranscript) -> ObjectPermanenceMetrics {
        let target_hypothesis = self.find_pre_occlusion_target_hypothesis(beliefs);
        let Some(target_hypothesis) = target_hypothesis else {
            return ObjectPermanenceMetrics {
                target_hypothesis: None,
                prediction_used_during_occlusion: false,
                last_observation_not_refreshed_by_prediction: None,
                unsupported_observation_refresh: false,
                unknown_source_observation: false,
                cross_associated_non_target_observation: false,
                reacquired_same_hypothesis: (self.branch == HiddenBranch::Persist).then_some(false),
                removed_target_resolved_by_deadline: (self.branch == HiddenBranch::Remove)
                    .then_some(false),
                prediction_overran_lost_deadline: (self.branch == HiddenBranch::Remove)
                    .then_some(false),
            };
        };

        let target_samples: Vec<&BeliefSample> = beliefs
            .samples()
            .iter()
            .filter(|sample| sample.hypothesis_id == target_hypothesis)
            .collect();
        let hidden_samples: Vec<&BeliefSample> = target_samples
            .iter()
            .copied()
            .filter(|sample| {
                sample.frame_index >= self.oracle.first_fully_occluded_frame
                    && sample.frame_index <= self.oracle.last_identical_sensor_frame
            })
            .collect();
        let predicted: Vec<&BeliefSample> = hidden_samples
            .iter()
            .copied()
            .filter(|sample| sample.lifecycle == BeliefLifecycle::OccludedPredicted)
            .collect();
        let prediction_used_during_occlusion = !predicted.is_empty();
        let last_observation_not_refreshed_by_prediction = prediction_used_during_occlusion
            .then(|| {
                predicted.iter().all(|sample| {
                    sample
                        .last_observed_frame
                        .is_some_and(|frame| frame <= self.oracle.last_visible_prefix_frame)
                        && sample.source_observation.is_none()
                })
            });

        let all_sensor_observations: Vec<SensorObservationId> = self
            .sensor_frames
            .iter()
            .flat_map(|frame| frame.observations.iter().map(|blob| blob.observation_id))
            .collect();
        let unknown_source_observation = target_samples.iter().any(|sample| {
            sample
                .source_observation
                .is_some_and(|observation| !all_sensor_observations.contains(&observation))
        });
        let cross_associated_non_target_observation = target_samples.iter().any(|sample| {
            sample.source_observation.is_some_and(|observation| {
                self.oracle
                    .non_target_sensor_observations
                    .contains(&observation)
            })
        });

        let supported_target_frames: Vec<u64> = target_samples
            .iter()
            .filter_map(|sample| {
                sample.source_observation.and_then(|observation| {
                    self.oracle
                        .target_sensor_observations
                        .iter()
                        .find(|(_, candidate)| *candidate == observation)
                        .map(|(frame, _)| *frame)
                })
            })
            .collect();
        let unsupported_observation_refresh = target_samples.iter().any(|sample| {
            sample
                .last_observed_frame
                .is_some_and(|frame| !supported_target_frames.contains(&frame))
        });

        let reacquired_same_hypothesis = (self.branch == HiddenBranch::Persist).then(|| {
            self.oracle
                .target_sensor_observations
                .iter()
                .filter(|(frame, _)| *frame >= self.oracle.reveal_frame)
                .any(|(_, observation)| {
                    target_samples.iter().any(|sample| {
                        sample.source_observation == Some(*observation)
                            && matches!(
                                sample.lifecycle,
                                BeliefLifecycle::Visible | BeliefLifecycle::PartiallyOccluded
                            )
                    })
                })
        });

        let removed_target_resolved_by_deadline = (self.branch == HiddenBranch::Remove).then(|| {
            target_samples.iter().any(|sample| {
                sample.frame_index >= self.oracle.lost_deadline_frame
                    && matches!(sample.lifecycle, BeliefLifecycle::Lost | BeliefLifecycle::Retired)
                    && sample
                        .last_observed_frame
                        .is_none_or(|frame| frame <= self.oracle.last_visible_prefix_frame)
            })
        });

        let prediction_overran_lost_deadline = (self.branch == HiddenBranch::Remove).then(|| {
            target_samples.iter().any(|sample| {
                sample.frame_index >= self.oracle.lost_deadline_frame
                    && sample.lifecycle == BeliefLifecycle::OccludedPredicted
            })
        });

        ObjectPermanenceMetrics {
            target_hypothesis: Some(target_hypothesis),
            prediction_used_during_occlusion,
            last_observation_not_refreshed_by_prediction,
            unsupported_observation_refresh,
            unknown_source_observation,
            cross_associated_non_target_observation,
            reacquired_same_hypothesis,
            removed_target_resolved_by_deadline,
            prediction_overran_lost_deadline,
        }
    }

    fn find_pre_occlusion_target_hypothesis(&self, beliefs: &BeliefTranscript) -> Option<u64> {
        self.oracle
            .target_sensor_observations
            .iter()
            .filter(|(frame, _)| *frame <= self.oracle.last_visible_prefix_frame)
            .rev()
            .find_map(|(_, observation)| {
                beliefs
                    .samples()
                    .iter()
                    .rev()
                    .find(|sample| sample.source_observation == Some(*observation))
                    .map(|sample| sample.hypothesis_id)
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HiddenBranch {
    Persist,
    Remove,
}

impl HiddenBranch {
    const fn opposite(self) -> Self {
        match self {
            Self::Persist => Self::Remove,
            Self::Remove => Self::Persist,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CanonicalEntityId(u64);

/// Private evaluator oracle. No public accessor returns this structure or its canonical ids.
#[derive(Debug)]
struct HiddenOracle {
    #[allow(dead_code)]
    target: CanonicalEntityId,
    #[allow(dead_code)]
    distractor: CanonicalEntityId,
    last_visible_prefix_frame: u64,
    first_fully_occluded_frame: u64,
    #[allow(dead_code)]
    hidden_intervention_frame: u64,
    last_identical_sensor_frame: u64,
    reveal_frame: u64,
    lost_deadline_frame: u64,
    target_sensor_observations: Vec<(u64, SensorObservationId)>,
    non_target_sensor_observations: Vec<SensorObservationId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationError {
    InvalidBeliefSample,
}

impl std::fmt::Display for EvaluationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBeliefSample => f.write_str("belief transcript contains an invalid sample"),
        }
    }
}

impl std::error::Error for EvaluationError {}

fn observation_id(scenario_seed: u64, frame: u64, slot: u64) -> u64 {
    nonzero_mix(scenario_seed ^ frame.rotate_left(17) ^ slot.rotate_left(41))
}

fn nonzero_mix(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^= value >> 31;
    value.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trial_by_branch(
        pair: &ObjectPermanencePair,
        branch: HiddenBranch,
    ) -> &ObjectPermanenceTrial {
        if pair.trial_a.branch == branch {
            &pair.trial_a
        } else {
            &pair.trial_b
        }
    }

    fn observation_for(trial: &ObjectPermanenceTrial, frame: u64) -> SensorObservationId {
        trial
            .oracle
            .target_sensor_observations
            .iter()
            .find(|(candidate, _)| *candidate == frame)
            .map(|(_, observation)| *observation)
            .expect("test frame must contain target sensor evidence")
    }

    fn sample(
        frame_index: u64,
        hypothesis_id: u64,
        lifecycle: BeliefLifecycle,
        last_observed_frame: u64,
        source_observation: Option<SensorObservationId>,
    ) -> BeliefSample {
        BeliefSample {
            frame_index,
            hypothesis_id,
            lifecycle,
            last_observed_frame: Some(last_observed_frame),
            source_observation,
            persistence_confidence: 0.9,
        }
    }

    fn beliefs_for_branch(
        pair: &ObjectPermanencePair,
        persist: BeliefTranscript,
        remove: BeliefTranscript,
    ) -> (BeliefTranscript, BeliefTranscript) {
        match pair.branch_a {
            HiddenBranch::Persist => (persist, remove),
            HiddenBranch::Remove => (remove, persist),
        }
    }

    #[test]
    fn paired_scenarios_have_identical_sensor_prefix_through_hidden_intervention() {
        let pair = ObjectPermanencePair::blinded(7, 7001);
        assert!(pair.visible_prefix_identical());
        assert_eq!(pair.trial_a.oracle.last_identical_sensor_frame, 8);
        assert_eq!(pair.trial_b.oracle.last_identical_sensor_frame, 8);
    }

    #[test]
    fn branch_order_is_controlled_only_by_private_blinding_nonce() {
        let scenario_seed = 17;
        let mut saw_persist_a = false;
        let mut saw_remove_a = false;
        let baseline_prefix = ObjectPermanencePair::blinded(scenario_seed, 0)
            .trial_a()
            .sensor_frames()[..=8]
            .to_vec();
        for nonce in 0..64 {
            let pair = ObjectPermanencePair::blinded(scenario_seed, nonce);
            assert_eq!(
                &pair.trial_a().sensor_frames()[..=8],
                baseline_prefix.as_slice()
            );
            match pair.branch_a {
                HiddenBranch::Persist => saw_persist_a = true,
                HiddenBranch::Remove => saw_remove_a = true,
            }
        }
        assert!(saw_persist_a && saw_remove_a);
    }

    #[test]
    fn identical_seed_and_nonce_reproduce_blind_sensor_transcripts() {
        let a = ObjectPermanencePair::blinded(91, 1234);
        let b = ObjectPermanencePair::blinded(91, 1234);
        assert_eq!(a.trial_a().sensor_frames(), b.trial_a().sensor_frames());
        assert_eq!(a.trial_b().sensor_frames(), b.trial_b().sensor_frames());
    }

    #[test]
    fn prediction_does_not_count_as_observation_refresh() {
        let pair = ObjectPermanencePair::blinded(11, 1101);
        let trial = trial_by_branch(&pair, HiddenBranch::Persist);
        let beliefs = BeliefTranscript::new(vec![
            sample(
                4,
                42,
                BeliefLifecycle::PartiallyOccluded,
                4,
                Some(observation_for(trial, 4)),
            ),
            sample(6, 42, BeliefLifecycle::OccludedPredicted, 6, None),
        ])
        .unwrap();
        let metrics = trial.assess(&beliefs);
        assert_eq!(metrics.target_hypothesis, Some(42));
        assert!(metrics.prediction_used_during_occlusion);
        assert_eq!(metrics.last_observation_not_refreshed_by_prediction, Some(false));
        assert!(metrics.unsupported_observation_refresh);
    }

    #[test]
    fn paired_protocol_preserves_identity_and_revises_hidden_removal_separately() {
        let pair = ObjectPermanencePair::blinded(19, 1901);
        let persist_trial = trial_by_branch(&pair, HiddenBranch::Persist);
        let remove_trial = trial_by_branch(&pair, HiddenBranch::Remove);
        let persist = BeliefTranscript::new(vec![
            sample(
                4,
                100,
                BeliefLifecycle::PartiallyOccluded,
                4,
                Some(observation_for(persist_trial, 4)),
            ),
            sample(6, 100, BeliefLifecycle::OccludedPredicted, 4, None),
            sample(8, 100, BeliefLifecycle::OccludedPredicted, 4, None),
            sample(
                9,
                100,
                BeliefLifecycle::Visible,
                9,
                Some(observation_for(persist_trial, 9)),
            ),
        ])
        .unwrap();
        let remove = BeliefTranscript::new(vec![
            sample(
                4,
                100,
                BeliefLifecycle::PartiallyOccluded,
                4,
                Some(observation_for(remove_trial, 4)),
            ),
            sample(6, 100, BeliefLifecycle::OccludedPredicted, 4, None),
            sample(8, 100, BeliefLifecycle::OccludedPredicted, 4, None),
            sample(11, 100, BeliefLifecycle::Lost, 4, None),
        ])
        .unwrap();
        let (a, b) = beliefs_for_branch(&pair, persist, remove);
        let assessment = pair.assess(&a, &b);
        assert!(assessment.visible_prefix_identical);
        assert_eq!(assessment.persist.reacquired_same_hypothesis, Some(true));
        assert_eq!(
            assessment.persist.last_observation_not_refreshed_by_prediction,
            Some(true)
        );
        assert!(!assessment.persist.unsupported_observation_refresh);
        assert_eq!(assessment.remove.removed_target_resolved_by_deadline, Some(true));
        assert_eq!(assessment.remove.prediction_overran_lost_deadline, Some(false));
        assert!(!assessment.remove.unsupported_observation_refresh);
    }

    #[test]
    fn always_persist_strategy_is_exposed_by_remove_branch() {
        let pair = ObjectPermanencePair::blinded(23, 2301);
        let remove = trial_by_branch(&pair, HiddenBranch::Remove);
        let beliefs = BeliefTranscript::new(vec![
            sample(
                4,
                5,
                BeliefLifecycle::PartiallyOccluded,
                4,
                Some(observation_for(remove, 4)),
            ),
            sample(6, 5, BeliefLifecycle::OccludedPredicted, 4, None),
            sample(11, 5, BeliefLifecycle::OccludedPredicted, 4, None),
        ])
        .unwrap();
        let metrics = remove.assess(&beliefs);
        assert_eq!(metrics.removed_target_resolved_by_deadline, Some(false));
        assert_eq!(metrics.prediction_overran_lost_deadline, Some(true));
    }

    #[test]
    fn always_drop_strategy_is_exposed_by_persist_branch() {
        let pair = ObjectPermanencePair::blinded(29, 2901);
        let persist = trial_by_branch(&pair, HiddenBranch::Persist);
        let beliefs = BeliefTranscript::new(vec![
            sample(
                4,
                8,
                BeliefLifecycle::PartiallyOccluded,
                4,
                Some(observation_for(persist, 4)),
            ),
            sample(6, 8, BeliefLifecycle::Lost, 4, None),
            sample(
                9,
                9,
                BeliefLifecycle::Visible,
                9,
                Some(observation_for(persist, 9)),
            ),
        ])
        .unwrap();
        let metrics = persist.assess(&beliefs);
        assert_eq!(metrics.reacquired_same_hypothesis, Some(false));
    }

    #[test]
    fn distractor_observation_cannot_silently_refresh_target_identity() {
        let pair = ObjectPermanencePair::blinded(31, 3101);
        let persist = trial_by_branch(&pair, HiddenBranch::Persist);
        let distractor = persist.oracle.non_target_sensor_observations[6];
        let beliefs = BeliefTranscript::new(vec![
            sample(
                4,
                12,
                BeliefLifecycle::PartiallyOccluded,
                4,
                Some(observation_for(persist, 4)),
            ),
            sample(6, 12, BeliefLifecycle::Visible, 6, Some(distractor)),
        ])
        .unwrap();
        let metrics = persist.assess(&beliefs);
        assert!(metrics.cross_associated_non_target_observation);
        assert!(metrics.unsupported_observation_refresh);
    }

    #[test]
    fn invented_sensor_observation_is_reported() {
        let pair = ObjectPermanencePair::blinded(37, 3701);
        let persist = trial_by_branch(&pair, HiddenBranch::Persist);
        let beliefs = BeliefTranscript::new(vec![
            sample(
                4,
                17,
                BeliefLifecycle::PartiallyOccluded,
                4,
                Some(observation_for(persist, 4)),
            ),
            sample(
                6,
                17,
                BeliefLifecycle::Visible,
                6,
                Some(SensorObservationId(u64::MAX)),
            ),
        ])
        .unwrap();
        let metrics = persist.assess(&beliefs);
        assert!(metrics.unknown_source_observation);
        assert!(metrics.unsupported_observation_refresh);
    }

    #[test]
    fn malformed_future_last_observation_is_rejected() {
        let error = BeliefTranscript::new(vec![BeliefSample {
            frame_index: 3,
            hypothesis_id: 1,
            lifecycle: BeliefLifecycle::Visible,
            last_observed_frame: Some(4),
            source_observation: None,
            persistence_confidence: 0.9,
        }])
        .unwrap_err();
        assert_eq!(error, EvaluationError::InvalidBeliefSample);
    }
}
