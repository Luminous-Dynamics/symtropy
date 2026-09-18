// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Passive hidden-ground-truth evaluation protocols for visual cognition.
//!
//! The evaluator owns canonical simulation identity and branch truth. Sensor-facing
//! frames intentionally contain no canonical entity identifiers, and scoring is
//! post-hoc: belief hypotheses are associated to the hidden target only through
//! sensor-observation lineage that the evaluated system itself reports.

#![deny(unsafe_code)]

/// Public scenario branch. The branch itself is evaluation metadata, not a sensor input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HiddenBranch {
    /// The target remains in the world while fully occluded and later reappears.
    Persist,
    /// The target is removed while fully occluded and does not reappear.
    Remove,
}

/// Coarse public trial phase. This describes the scenario schedule, not canonical identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrialPhase {
    VisiblePrefix,
    PartialOcclusion,
    FullyOccluded,
    Reveal,
}

/// Frame-local sensor observation identity.
///
/// This is safe to expose: it identifies a sensor record, not a canonical simulation entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SensorObservationId(pub u64);

/// A deliberately small stand-in for a future rendered/perceptual observation.
///
/// It contains only sensor-facing quantities. There is no canonical entity id, world-object
/// handle, hidden branch flag, or evaluator association field.
#[derive(Debug, Clone, PartialEq)]
pub struct SensorBlob {
    pub observation_id: SensorObservationId,
    pub x_norm: f32,
    pub y_norm: f32,
    pub visible_fraction: f32,
    pub appearance: [f32; 4],
}

/// Everything the system under evaluation may receive for one synthetic frame.
#[derive(Debug, Clone, PartialEq)]
pub struct SensorFrame {
    pub frame_index: u64,
    pub phase: TrialPhase,
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
/// `hypothesis_id` is owned by the evaluated system. It is never a canonical simulation id.
/// `source_observation` is optional because prediction-only samples should not invent a current
/// observation merely to keep a hypothesis alive.
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

/// Ordered belief transcript supplied after a trial completes.
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
/// These fields are evidence, not an overall score. A caller can preserve the complete metric
/// vector instead of collapsing qualitatively different failure modes into one number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectPermanenceMetrics {
    pub target_hypothesis: Option<u64>,
    pub prediction_used_during_occlusion: bool,
    pub last_observation_not_refreshed_by_prediction: Option<bool>,
    pub cross_associated_non_target_observation: bool,
    pub reacquired_same_hypothesis: Option<bool>,
    pub removed_target_resolved_by_deadline: Option<bool>,
    pub prediction_overran_lost_deadline: Option<bool>,
}

/// Assessment of the paired intervention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedObjectPermanenceAssessment {
    pub visible_prefix_identical: bool,
    pub persist: ObjectPermanenceMetrics,
    pub remove: ObjectPermanenceMetrics,
}

/// Deterministic paired object-permanence protocol.
#[derive(Debug, Clone)]
pub struct ObjectPermanencePair {
    seed: u64,
    persist: ObjectPermanenceTrial,
    remove: ObjectPermanenceTrial,
}

impl ObjectPermanencePair {
    pub fn deterministic(seed: u64) -> Self {
        let target = CanonicalEntityId(nonzero_mix(seed ^ 0xA11C_E001));
        let distractor = CanonicalEntityId(nonzero_mix(seed ^ 0xD157_AC70));
        Self {
            seed,
            persist: ObjectPermanenceTrial::build(seed, target, distractor, HiddenBranch::Persist),
            remove: ObjectPermanenceTrial::build(seed, target, distractor, HiddenBranch::Remove),
        }
    }

    pub const fn seed(&self) -> u64 {
        self.seed
    }

    pub fn persist_trial(&self) -> &ObjectPermanenceTrial {
        &self.persist
    }

    pub fn remove_trial(&self) -> &ObjectPermanenceTrial {
        &self.remove
    }

    pub fn visible_prefix_identical(&self) -> bool {
        let end = self.persist.oracle.last_identical_sensor_frame;
        self.persist
            .sensor_frames
            .iter()
            .filter(|frame| frame.frame_index <= end)
            .eq(self
                .remove
                .sensor_frames
                .iter()
                .filter(|frame| frame.frame_index <= end))
    }

    pub fn assess(
        &self,
        persist_beliefs: &BeliefTranscript,
        remove_beliefs: &BeliefTranscript,
    ) -> PairedObjectPermanenceAssessment {
        PairedObjectPermanenceAssessment {
            visible_prefix_identical: self.visible_prefix_identical(),
            persist: self.persist.assess(persist_beliefs),
            remove: self.remove.assess(remove_beliefs),
        }
    }
}

/// One branch of the paired trial.
///
/// The hidden oracle is private. Consumers receive only `sensor_frames()` and later submit a
/// belief transcript to `assess()`.
#[derive(Debug, Clone)]
pub struct ObjectPermanenceTrial {
    branch: HiddenBranch,
    sensor_frames: Vec<SensorFrame>,
    oracle: HiddenOracle,
}

impl ObjectPermanenceTrial {
    fn build(
        seed: u64,
        target: CanonicalEntityId,
        distractor: CanonicalEntityId,
        branch: HiddenBranch,
    ) -> Self {
        let mut sensor_frames = Vec::new();
        let mut target_sensor_observations = Vec::new();
        let mut non_target_sensor_observations = Vec::new();

        for frame_index in 0..=11 {
            let phase = match frame_index {
                0..=3 => TrialPhase::VisiblePrefix,
                4 => TrialPhase::PartialOcclusion,
                5..=8 => TrialPhase::FullyOccluded,
                _ => TrialPhase::Reveal,
            };
            let mut observations = Vec::new();

            let distractor_observation = SensorObservationId(observation_id(seed, frame_index, 1));
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
                let target_observation = SensorObservationId(observation_id(seed, frame_index, 2));
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
                phase,
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

    pub const fn branch(&self) -> HiddenBranch {
        self.branch
    }

    pub fn sensor_frames(&self) -> &[SensorFrame] {
        &self.sensor_frames
    }

    pub fn assess(&self, beliefs: &BeliefTranscript) -> ObjectPermanenceMetrics {
        let target_hypothesis = self.find_pre_occlusion_target_hypothesis(beliefs);
        let Some(target_hypothesis) = target_hypothesis else {
            return ObjectPermanenceMetrics {
                target_hypothesis: None,
                prediction_used_during_occlusion: false,
                last_observation_not_refreshed_by_prediction: None,
                cross_associated_non_target_observation: false,
                reacquired_same_hypothesis: (self.branch == HiddenBranch::Persist).then_some(false),
                removed_target_resolved_by_deadline: (self.branch == HiddenBranch::Remove)
                    .then_some(false),
                prediction_overran_lost_deadline: (self.branch == HiddenBranch::Remove)
                    .then_some(false),
            };
        };

        let hidden_samples: Vec<&BeliefSample> = beliefs
            .samples()
            .iter()
            .filter(|sample| {
                sample.hypothesis_id == target_hypothesis
                    && sample.frame_index >= self.oracle.first_fully_occluded_frame
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

        let cross_associated_non_target_observation = beliefs.samples().iter().any(|sample| {
            sample.hypothesis_id == target_hypothesis
                && sample.source_observation.is_some_and(|observation| {
                    self.oracle
                        .non_target_sensor_observations
                        .contains(&observation)
                })
        });

        let reacquired_same_hypothesis = (self.branch == HiddenBranch::Persist).then(|| {
            let reveal_target_observations = self
                .oracle
                .target_sensor_observations
                .iter()
                .filter(|(frame, _)| *frame >= self.oracle.reveal_frame)
                .map(|(_, observation)| *observation);
            reveal_target_observations.into_iter().any(|observation| {
                beliefs.samples().iter().any(|sample| {
                    sample.hypothesis_id == target_hypothesis
                        && sample.source_observation == Some(observation)
                        && matches!(
                            sample.lifecycle,
                            BeliefLifecycle::Visible | BeliefLifecycle::PartiallyOccluded
                        )
                })
            })
        });

        let removed_target_resolved_by_deadline = (self.branch == HiddenBranch::Remove).then(|| {
            beliefs.samples().iter().any(|sample| {
                sample.hypothesis_id == target_hypothesis
                    && sample.frame_index >= self.oracle.lost_deadline_frame
                    && matches!(sample.lifecycle, BeliefLifecycle::Lost | BeliefLifecycle::Retired)
                    && sample
                        .last_observed_frame
                        .is_none_or(|frame| frame <= self.oracle.last_visible_prefix_frame)
            })
        });

        let prediction_overran_lost_deadline = (self.branch == HiddenBranch::Remove).then(|| {
            beliefs.samples().iter().any(|sample| {
                sample.hypothesis_id == target_hypothesis
                    && sample.frame_index >= self.oracle.lost_deadline_frame
                    && sample.lifecycle == BeliefLifecycle::OccludedPredicted
            })
        });

        ObjectPermanenceMetrics {
            target_hypothesis: Some(target_hypothesis),
            prediction_used_during_occlusion,
            last_observation_not_refreshed_by_prediction,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CanonicalEntityId(u64);

/// Private evaluator oracle. No public accessor returns this structure or either canonical id.
#[derive(Debug, Clone)]
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

fn observation_id(seed: u64, frame: u64, slot: u64) -> u64 {
    nonzero_mix(seed ^ frame.rotate_left(17) ^ slot.rotate_left(41))
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

    #[test]
    fn paired_scenarios_have_identical_sensor_prefix_through_hidden_intervention() {
        let pair = ObjectPermanencePair::deterministic(7);
        assert!(pair.visible_prefix_identical());
        assert_eq!(
            pair.persist.oracle.hidden_intervention_frame,
            pair.remove.oracle.hidden_intervention_frame
        );
        assert_eq!(pair.persist.oracle.last_identical_sensor_frame, 8);
    }

    #[test]
    fn deterministic_seed_reproduces_sensor_transcript() {
        let a = ObjectPermanencePair::deterministic(91);
        let b = ObjectPermanencePair::deterministic(91);
        assert_eq!(a.persist.sensor_frames(), b.persist.sensor_frames());
        assert_eq!(a.remove.sensor_frames(), b.remove.sensor_frames());
    }

    #[test]
    fn prediction_does_not_count_as_observation_refresh() {
        let pair = ObjectPermanencePair::deterministic(11);
        let trial = pair.persist_trial();
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
    }

    #[test]
    fn paired_protocol_rewards_persistence_and_hidden_removal_revision_separately() {
        let pair = ObjectPermanencePair::deterministic(19);
        let persist = pair.persist_trial();
        let remove = pair.remove_trial();
        let persist_beliefs = BeliefTranscript::new(vec![
            sample(
                4,
                100,
                BeliefLifecycle::PartiallyOccluded,
                4,
                Some(observation_for(persist, 4)),
            ),
            sample(6, 100, BeliefLifecycle::OccludedPredicted, 4, None),
            sample(8, 100, BeliefLifecycle::OccludedPredicted, 4, None),
            sample(
                9,
                100,
                BeliefLifecycle::Visible,
                9,
                Some(observation_for(persist, 9)),
            ),
        ])
        .unwrap();
        let remove_beliefs = BeliefTranscript::new(vec![
            sample(
                4,
                100,
                BeliefLifecycle::PartiallyOccluded,
                4,
                Some(observation_for(remove, 4)),
            ),
            sample(6, 100, BeliefLifecycle::OccludedPredicted, 4, None),
            sample(8, 100, BeliefLifecycle::OccludedPredicted, 4, None),
            sample(11, 100, BeliefLifecycle::Lost, 4, None),
        ])
        .unwrap();

        let assessment = pair.assess(&persist_beliefs, &remove_beliefs);
        assert!(assessment.visible_prefix_identical);
        assert_eq!(assessment.persist.reacquired_same_hypothesis, Some(true));
        assert_eq!(
            assessment.persist.last_observation_not_refreshed_by_prediction,
            Some(true)
        );
        assert_eq!(assessment.remove.removed_target_resolved_by_deadline, Some(true));
        assert_eq!(assessment.remove.prediction_overran_lost_deadline, Some(false));
    }

    #[test]
    fn always_persist_strategy_is_exposed_by_remove_branch() {
        let pair = ObjectPermanencePair::deterministic(23);
        let remove = pair.remove_trial();
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
        let pair = ObjectPermanencePair::deterministic(29);
        let persist = pair.persist_trial();
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
        let pair = ObjectPermanencePair::deterministic(31);
        let persist = pair.persist_trial();
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
