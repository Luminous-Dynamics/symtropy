//! Frozen-predictor conformal set family for adaptive episode-level miscoverage.
//!
//! The family maps a requested whole-episode miscoverage level eta to the
//! finite-sample split-conformal order statistic of maximum trajectory errors.
//! It can be queried at multiple levels by the anytime running or rolling
//! policies, but the predictor, preprocessing and score definition remain
//! frozen to the calibration cohort.
//!
//! This module constructs a constant-radius 2D tube around every predicted
//! position in an episode. It does not authenticate digests, prove episode
//! exchangeability, or guarantee the adaptive policy's theorem by itself.

use std::collections::BTreeSet;
use std::fmt;

use crate::episode_conformal_tube::EpisodeMaximumPositionError2d;

const SHA256_BYTES: usize = 32;
const DIM: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EpisodeConformalFamilyConfig {
    /// Digest of the exact frozen calibration episode manifest.
    pub calibration_cohort_sha256: [u8; SHA256_BYTES],
    /// Digest of predictor weights and preprocessing.
    pub predictor_sha256: [u8; SHA256_BYTES],
    /// Digest of the maximum whole-episode position-error score definition.
    pub score_definition_sha256: [u8; SHA256_BYTES],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectedEpisodeRadius {
    pub requested_miscoverage: f64,
    pub requested_coverage: f64,
    pub calibration_episode_count: usize,
    /// One-based split-conformal order-statistic rank.
    pub selected_order_statistic_rank: usize,
    pub radius_m: f64,
    pub calibration_cohort_sha256: [u8; SHA256_BYTES],
    pub predictor_sha256: [u8; SHA256_BYTES],
    pub score_definition_sha256: [u8; SHA256_BYTES],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DynamicEpisodeSetAbstention {
    UnsupportedMiscoverageLevel,
    InsufficientCalibrationRank {
        needed_rank: usize,
        available_episodes: usize,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum DynamicEpisodeSetOutcome {
    /// Finite-radius split-conformal tube.
    Bounded(DynamicEpisodeTube2d),
    /// Empty, bounded set for eta >= 1. Its whole-episode miss indicator is one.
    EmptyBounded(DynamicEpisodeEmptySet2d),
    /// Unbounded/unsupported request; caller must record an abstention (Z=0).
    Abstain(DynamicEpisodeSetAbstention),
}

/// A degenerate empty prediction set for a requested miscoverage level >= 1.
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicEpisodeEmptySet2d {
    episode_id: u64,
    independence_group_id: u64,
    position_count: usize,
    requested_miscoverage: f64,
    calibration_cohort_sha256: [u8; SHA256_BYTES],
    predictor_sha256: [u8; SHA256_BYTES],
    score_definition_sha256: [u8; SHA256_BYTES],
    decision_sequence_id: u64,
    decision_receipt_sha256: [u8; SHA256_BYTES],
}

impl DynamicEpisodeEmptySet2d {
    pub fn episode_id(&self) -> u64 {
        self.episode_id
    }

    pub fn independence_group_id(&self) -> u64 {
        self.independence_group_id
    }

    pub fn position_count(&self) -> usize {
        self.position_count
    }

    pub fn requested_miscoverage(&self) -> f64 {
        self.requested_miscoverage
    }

    pub fn decision_sequence_id(&self) -> u64 {
        self.decision_sequence_id
    }

    pub fn decision_receipt_sha256(&self) -> [u8; SHA256_BYTES] {
        self.decision_receipt_sha256
    }

    /// An empty set contains no finite position.
    pub fn contains_position(&self, _time_index: usize, _truth_position_m: [f64; DIM]) -> bool {
        false
    }

    /// Validate full truth completeness, then report the miss (false).
    pub fn contains_complete_trajectory(
        &self,
        truth_positions_m: &[[f64; DIM]],
    ) -> Result<bool, DynamicEpisodeSetError> {
        if truth_positions_m.len() != self.position_count {
            return Err(DynamicEpisodeSetError::TruthPositionCountMismatch {
                expected: self.position_count,
                observed: truth_positions_m.len(),
            });
        }
        if let Some(index) = truth_positions_m
            .iter()
            .position(|position| !position.iter().all(|value| value.is_finite()))
        {
            return Err(DynamicEpisodeSetError::InvalidTruthPosition(index));
        }
        Ok(false)
    }
}

/// A constant-radius disk around every predicted position in one episode.
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicEpisodeTube2d {
    episode_id: u64,
    independence_group_id: u64,
    centers_m: Vec<[f64; DIM]>,
    radius_m: f64,
    requested_miscoverage: f64,
    requested_coverage: f64,
    calibration_episode_count: usize,
    selected_order_statistic_rank: usize,
    calibration_cohort_sha256: [u8; SHA256_BYTES],
    predictor_sha256: [u8; SHA256_BYTES],
    score_definition_sha256: [u8; SHA256_BYTES],
    decision_sequence_id: u64,
    decision_receipt_sha256: [u8; SHA256_BYTES],
}

impl DynamicEpisodeTube2d {
    pub fn episode_id(&self) -> u64 {
        self.episode_id
    }

    pub fn independence_group_id(&self) -> u64 {
        self.independence_group_id
    }

    pub fn centers_m(&self) -> &[[f64; DIM]] {
        &self.centers_m
    }

    pub fn radius_m(&self) -> f64 {
        self.radius_m
    }

    pub fn requested_miscoverage(&self) -> f64 {
        self.requested_miscoverage
    }

    pub fn requested_coverage(&self) -> f64 {
        self.requested_coverage
    }

    pub fn calibration_episode_count(&self) -> usize {
        self.calibration_episode_count
    }

    pub fn selected_order_statistic_rank(&self) -> usize {
        self.selected_order_statistic_rank
    }

    pub fn calibration_cohort_sha256(&self) -> [u8; SHA256_BYTES] {
        self.calibration_cohort_sha256
    }

    pub fn predictor_sha256(&self) -> [u8; SHA256_BYTES] {
        self.predictor_sha256
    }

    pub fn score_definition_sha256(&self) -> [u8; SHA256_BYTES] {
        self.score_definition_sha256
    }

    pub fn decision_sequence_id(&self) -> u64 {
        self.decision_sequence_id
    }

    pub fn decision_receipt_sha256(&self) -> [u8; SHA256_BYTES] {
        self.decision_receipt_sha256
    }

    pub fn contains_position(&self, time_index: usize, truth_position_m: [f64; DIM]) -> bool {
        let Some(center) = self.centers_m.get(time_index) else {
            return false;
        };
        if !truth_position_m.iter().all(|value| value.is_finite()) {
            return false;
        }
        let dx = truth_position_m[0] - center[0];
        let dy = truth_position_m[1] - center[1];
        dx.hypot(dy) <= self.radius_m
    }

    /// Score the complete episode. A missing, truncated or non-finite truth
    /// trajectory is an evaluator error, never a successful partial hit.
    pub fn contains_complete_trajectory(
        &self,
        truth_positions_m: &[[f64; DIM]],
    ) -> Result<bool, DynamicEpisodeSetError> {
        if truth_positions_m.len() != self.centers_m.len() {
            return Err(DynamicEpisodeSetError::TruthPositionCountMismatch {
                expected: self.centers_m.len(),
                observed: truth_positions_m.len(),
            });
        }
        if let Some(index) = truth_positions_m
            .iter()
            .position(|position| !position.iter().all(|value| value.is_finite()))
        {
            return Err(DynamicEpisodeSetError::InvalidTruthPosition(index));
        }
        Ok(truth_positions_m
            .iter()
            .enumerate()
            .all(|(index, position)| self.contains_position(index, *position)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DynamicEpisodeSetError {
    EmptyCalibrationCohort,
    MissingCalibrationCohortCommitment,
    MissingPredictorCommitment,
    MissingScoreDefinitionCommitment,
    InvalidCalibrationEpisodeIdentity,
    DuplicateCalibrationEpisode(u64),
    DuplicateCalibrationIndependenceGroup(u64),
    EmptyCalibrationEpisode(u64),
    InvalidCalibrationScore(u64),
    MissingCalibrationSourceCommitment(u64),
    EvaluationIdentityInvalid,
    EvaluationEpisodeReused(u64),
    EvaluationIndependenceGroupReused(u64),
    PredictorCommitmentMismatch,
    ScoreDefinitionMismatch,
    EmptyPredictedTrajectory,
    InvalidPredictedPosition(usize),
    TruthPositionCountMismatch {
        expected: usize,
        observed: usize,
    },
    InvalidTruthPosition(usize),
    MissingDecisionSequence,
    MissingDecisionReceipt,
    ArithmeticOverflow,
}

impl fmt::Display for DynamicEpisodeSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCalibrationCohort => write!(f, "calibration cohort must contain at least one episode"),
            Self::MissingCalibrationCohortCommitment => write!(f, "calibration cohort commitment is missing"),
            Self::MissingPredictorCommitment => write!(f, "frozen predictor commitment is missing"),
            Self::MissingScoreDefinitionCommitment => write!(f, "score-definition commitment is missing"),
            Self::InvalidCalibrationEpisodeIdentity => write!(f, "calibration episode and independence-group IDs must be nonzero"),
            Self::DuplicateCalibrationEpisode(id) => write!(f, "duplicate calibration episode identity: {id}"),
            Self::DuplicateCalibrationIndependenceGroup(id) => write!(f, "duplicate calibration independence group: {id}"),
            Self::EmptyCalibrationEpisode(id) => write!(f, "calibration episode {id} has no scored positions"),
            Self::InvalidCalibrationScore(id) => write!(f, "calibration episode {id} has an invalid maximum-error score"),
            Self::MissingCalibrationSourceCommitment(id) => write!(f, "calibration episode {id} is missing its source-manifest commitment"),
            Self::EvaluationIdentityInvalid => write!(f, "evaluation episode and independence-group IDs must be nonzero"),
            Self::EvaluationEpisodeReused(id) => write!(f, "evaluation episode {id} overlaps calibration"),
            Self::EvaluationIndependenceGroupReused(id) => write!(f, "evaluation independence group {id} overlaps calibration"),
            Self::PredictorCommitmentMismatch => write!(f, "evaluation predictor differs from the frozen calibration predictor"),
            Self::ScoreDefinitionMismatch => write!(f, "evaluation score definition differs from the frozen calibration score"),
            Self::EmptyPredictedTrajectory => write!(f, "evaluation prediction trajectory must not be empty"),
            Self::InvalidPredictedPosition(index) => write!(f, "evaluation predicted position {index} is non-finite"),
            Self::TruthPositionCountMismatch { expected, observed } => write!(f, "truth trajectory has {observed} positions but prediction has {expected}"),
            Self::InvalidTruthPosition(index) => write!(f, "truth position {index} is non-finite"),
            Self::MissingDecisionSequence => write!(f, "decision sequence ID must be nonzero"),
            Self::MissingDecisionReceipt => write!(f, "decision receipt commitment is missing"),
            Self::ArithmeticOverflow => write!(f, "conformal family arithmetic overflowed"),
        }
    }
}

impl std::error::Error for DynamicEpisodeSetError {}

#[derive(Clone, Debug, PartialEq)]
pub struct DynamicEpisodeConformalFamily2d {
    config: EpisodeConformalFamilyConfig,
    ordered_scores: Vec<(f64, u64)>,
    calibration_episode_ids: Vec<u64>,
    calibration_independence_group_ids: Vec<u64>,
    total_evaluated_position_count: usize,
}

impl DynamicEpisodeConformalFamily2d {
    /// Fit the score family once on an independent calibration cohort.
    ///
    /// The predictor and score definition are committed at fit time. Evaluation
    /// calls must present those exact digests; a model/preprocessing/score
    /// change requires a new calibration family.
    pub fn fit(
        episodes: &[EpisodeMaximumPositionError2d],
        config: EpisodeConformalFamilyConfig,
    ) -> Result<Self, DynamicEpisodeSetError> {
        if episodes.is_empty() {
            return Err(DynamicEpisodeSetError::EmptyCalibrationCohort);
        }
        if config
            .calibration_cohort_sha256
            .iter()
            .all(|byte| *byte == 0)
        {
            return Err(DynamicEpisodeSetError::MissingCalibrationCohortCommitment);
        }
        if config.predictor_sha256.iter().all(|byte| *byte == 0) {
            return Err(DynamicEpisodeSetError::MissingPredictorCommitment);
        }
        if config
            .score_definition_sha256
            .iter()
            .all(|byte| *byte == 0)
        {
            return Err(DynamicEpisodeSetError::MissingScoreDefinitionCommitment);
        }

        let mut episode_ids = BTreeSet::new();
        let mut group_ids = BTreeSet::new();
        let mut ordered_scores = Vec::with_capacity(episodes.len());
        let mut total_evaluated_position_count = 0_usize;

        for episode in episodes {
            if episode.episode_id == 0 || episode.independence_group_id == 0 {
                return Err(DynamicEpisodeSetError::InvalidCalibrationEpisodeIdentity);
            }
            if !episode_ids.insert(episode.episode_id) {
                return Err(DynamicEpisodeSetError::DuplicateCalibrationEpisode(
                    episode.episode_id,
                ));
            }
            if !group_ids.insert(episode.independence_group_id) {
                return Err(
                    DynamicEpisodeSetError::DuplicateCalibrationIndependenceGroup(
                        episode.independence_group_id,
                    ),
                );
            }
            if episode.evaluated_position_count == 0 {
                return Err(DynamicEpisodeSetError::EmptyCalibrationEpisode(
                    episode.episode_id,
                ));
            }
            if !episode.maximum_position_error_m.is_finite()
                || episode.maximum_position_error_m < 0.0
            {
                return Err(DynamicEpisodeSetError::InvalidCalibrationScore(
                    episode.episode_id,
                ));
            }
            if episode
                .source_manifest_sha256
                .iter()
                .all(|byte| *byte == 0)
            {
                return Err(
                    DynamicEpisodeSetError::MissingCalibrationSourceCommitment(
                        episode.episode_id,
                    ),
                );
            }
            total_evaluated_position_count = total_evaluated_position_count
                .checked_add(episode.evaluated_position_count)
                .ok_or(DynamicEpisodeSetError::ArithmeticOverflow)?;
            ordered_scores.push((episode.maximum_position_error_m, episode.episode_id));
        }

        ordered_scores.sort_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
        });

        Ok(Self {
            config,
            ordered_scores,
            calibration_episode_ids: episode_ids.into_iter().collect(),
            calibration_independence_group_ids: group_ids.into_iter().collect(),
            total_evaluated_position_count,
        })
    }

    pub fn calibration_episode_count(&self) -> usize {
        self.ordered_scores.len()
    }

    pub fn total_evaluated_position_count(&self) -> usize {
        self.total_evaluated_position_count
    }

    pub fn calibration_cohort_sha256(&self) -> [u8; SHA256_BYTES] {
        self.config.calibration_cohort_sha256
    }

    pub fn predictor_sha256(&self) -> [u8; SHA256_BYTES] {
        self.config.predictor_sha256
    }

    pub fn score_definition_sha256(&self) -> [u8; SHA256_BYTES] {
        self.config.score_definition_sha256
    }

    /// Select the exact split-conformal radius for a requested miscoverage eta.
    ///
    /// Coverage is 1-eta and the one-based order-statistic rank is
    /// ceil((n+1)*(1-eta)). A rank greater than n yields no finite radius.
    pub fn select_radius(
        &self,
        requested_miscoverage: f64,
    ) -> Result<Result<SelectedEpisodeRadius, DynamicEpisodeSetAbstention>, DynamicEpisodeSetError>
    {
        if !requested_miscoverage.is_finite() || requested_miscoverage <= 0.0 {
            return Ok(Err(DynamicEpisodeSetAbstention::UnsupportedMiscoverageLevel));
        }
        if requested_miscoverage >= 1.0 {
            return Ok(Err(DynamicEpisodeSetAbstention::UnsupportedMiscoverageLevel));
        }
        let n = self.ordered_scores.len();
        if n == 0 {
            return Err(DynamicEpisodeSetError::EmptyCalibrationCohort);
        }
        let coverage = 1.0 - requested_miscoverage;
        let rank_float = ((n as f64) + 1.0) * coverage;
        if !rank_float.is_finite()
            || rank_float < 1.0
            || rank_float > usize::MAX as f64
        {
            return Err(DynamicEpisodeSetError::ArithmeticOverflow);
        }
        let rank = rank_float.ceil() as usize;
        if rank > n {
            return Ok(Err(DynamicEpisodeSetAbstention::InsufficientCalibrationRank {
                needed_rank: rank,
                available_episodes: n,
            }));
        }
        let radius_m = self.ordered_scores[rank - 1].0;
        Ok(Ok(SelectedEpisodeRadius {
            requested_miscoverage,
            requested_coverage: coverage,
            calibration_episode_count: n,
            selected_order_statistic_rank: rank,
            radius_m,
            calibration_cohort_sha256: self.config.calibration_cohort_sha256,
            predictor_sha256: self.config.predictor_sha256,
            score_definition_sha256: self.config.score_definition_sha256,
        }))
    }

    /// Build a constant-radius trajectory tube or explicitly abstain when the
    /// requested eta is unsupported or the calibration cohort is too small.
    pub fn build_episode_tube(
        &self,
        episode_id: u64,
        independence_group_id: u64,
        predicted_positions_m: &[[f64; DIM]],
        requested_miscoverage: f64,
        evaluation_predictor_sha256: [u8; SHA256_BYTES],
        evaluation_score_definition_sha256: [u8; SHA256_BYTES],
        decision_sequence_id: u64,
        decision_receipt_sha256: [u8; SHA256_BYTES],
    ) -> Result<DynamicEpisodeSetOutcome, DynamicEpisodeSetError> {
        if episode_id == 0 || independence_group_id == 0 {
            return Err(DynamicEpisodeSetError::EvaluationIdentityInvalid);
        }
        if self.calibration_episode_ids.contains(&episode_id) {
            return Err(DynamicEpisodeSetError::EvaluationEpisodeReused(episode_id));
        }
        if self
            .calibration_independence_group_ids
            .contains(&independence_group_id)
        {
            return Err(DynamicEpisodeSetError::EvaluationIndependenceGroupReused(
                independence_group_id,
            ));
        }
        if evaluation_predictor_sha256.iter().all(|byte| *byte == 0) {
            return Err(DynamicEpisodeSetError::MissingPredictorCommitment);
        }
        if evaluation_predictor_sha256 != self.config.predictor_sha256 {
            return Err(DynamicEpisodeSetError::PredictorCommitmentMismatch);
        }
        if evaluation_score_definition_sha256.iter().all(|byte| *byte == 0) {
            return Err(DynamicEpisodeSetError::MissingScoreDefinitionCommitment);
        }
        if evaluation_score_definition_sha256 != self.config.score_definition_sha256 {
            return Err(DynamicEpisodeSetError::ScoreDefinitionMismatch);
        }
        if predicted_positions_m.is_empty() {
            return Err(DynamicEpisodeSetError::EmptyPredictedTrajectory);
        }
        if let Some(index) = predicted_positions_m
            .iter()
            .position(|position| !position.iter().all(|value| value.is_finite()))
        {
            return Err(DynamicEpisodeSetError::InvalidPredictedPosition(index));
        }
        if decision_sequence_id == 0 {
            return Err(DynamicEpisodeSetError::MissingDecisionSequence);
        }
        if decision_receipt_sha256.iter().all(|byte| *byte == 0) {
            return Err(DynamicEpisodeSetError::MissingDecisionReceipt);
        }

        if !requested_miscoverage.is_finite() || requested_miscoverage <= 0.0 {
            return Ok(DynamicEpisodeSetOutcome::Abstain(
                DynamicEpisodeSetAbstention::UnsupportedMiscoverageLevel,
            ));
        }

        if requested_miscoverage >= 1.0 {
            return Ok(DynamicEpisodeSetOutcome::EmptyBounded(
                DynamicEpisodeEmptySet2d {
                    episode_id,
                    independence_group_id,
                    position_count: predicted_positions_m.len(),
                    requested_miscoverage,
                    calibration_cohort_sha256: self.config.calibration_cohort_sha256,
                    predictor_sha256: self.config.predictor_sha256,
                    score_definition_sha256: self.config.score_definition_sha256,
                    decision_sequence_id,
                    decision_receipt_sha256,
                },
            ));
        }

        let selected = match self.select_radius(requested_miscoverage)? {
            Ok(selected) => selected,
            Err(abstention) => return Ok(DynamicEpisodeSetOutcome::Abstain(abstention)),
        };

        Ok(DynamicEpisodeSetOutcome::Bounded(DynamicEpisodeTube2d {
            episode_id,
            independence_group_id,
            centers_m: predicted_positions_m.to_vec(),
            radius_m: selected.radius_m,
            requested_miscoverage: selected.requested_miscoverage,
            requested_coverage: selected.requested_coverage,
            calibration_episode_count: selected.calibration_episode_count,
            selected_order_statistic_rank: selected.selected_order_statistic_rank,
            calibration_cohort_sha256: selected.calibration_cohort_sha256,
            predictor_sha256: selected.predictor_sha256,
            score_definition_sha256: selected.score_definition_sha256,
            decision_sequence_id,
            decision_receipt_sha256,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> EpisodeConformalFamilyConfig {
        EpisodeConformalFamilyConfig {
            calibration_cohort_sha256: [0xA1; SHA256_BYTES],
            predictor_sha256: [0xB2; SHA256_BYTES],
            score_definition_sha256: [0xC3; SHA256_BYTES],
        }
    }

    fn episode(id: u64, error_m: f64) -> EpisodeMaximumPositionError2d {
        EpisodeMaximumPositionError2d {
            episode_id: id,
            independence_group_id: id + 10_000,
            evaluated_position_count: 100,
            maximum_position_error_m: error_m,
            source_manifest_sha256: [((id % 255) + 1) as u8; SHA256_BYTES],
        }
    }

    fn episodes(count: usize) -> Vec<EpisodeMaximumPositionError2d> {
        (1..=count as u64)
            .map(|id| episode(id, id as f64 / 100.0))
            .collect()
    }

    fn family(count: usize) -> DynamicEpisodeConformalFamily2d {
        DynamicEpisodeConformalFamily2d::fit(&episodes(count), config()).unwrap()
    }

    #[test]
    fn dynamic_level_selects_exact_split_conformal_order_statistics() {
        let family = family(99);
        let low_miscoverage = family.select_radius(0.01).unwrap().unwrap();
        assert_eq!(low_miscoverage.requested_coverage, 0.99);
        assert_eq!(low_miscoverage.selected_order_statistic_rank, 99);
        assert!((low_miscoverage.radius_m - 0.99).abs() < 1e-12);

        let higher_miscoverage = family.select_radius(0.20).unwrap().unwrap();
        assert_eq!(higher_miscoverage.selected_order_statistic_rank, 80);
        assert!((higher_miscoverage.radius_m - 0.80).abs() < 1e-12);
        assert!(higher_miscoverage.radius_m <= low_miscoverage.radius_m);
    }

    #[test]
    fn finite_sample_rank_boundary_abstains_below_required_calibration_size() {
        let insufficient = family(98);
        assert_eq!(
            insufficient.select_radius(0.01).unwrap(),
            Err(DynamicEpisodeSetAbstention::InsufficientCalibrationRank {
                needed_rank: 99,
                available_episodes: 98
            })
        );
        let sufficient = family(99);
        assert!(sufficient.select_radius(0.01).unwrap().is_ok());
    }

    #[test]
    fn unsupported_or_extreme_levels_do_not_get_clipped() {
        let family = family(20);
        for eta in [0.0, 1.0, 1.1, f64::NAN] {
            assert_eq!(
                family.select_radius(eta).unwrap(),
                Err(DynamicEpisodeSetAbstention::UnsupportedMiscoverageLevel)
            );
        }
    }

    #[test]
    fn builder_constructs_full_episode_tube_and_retains_decision_receipt() {
        let family = family(99);
        let outcome = family
            .build_episode_tube(
                500,
                20_500,
                &[[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]],
                0.01,
                [0xB2; SHA256_BYTES],
                [0xC3; SHA256_BYTES],
                501,
                [0xD4; SHA256_BYTES],
            )
            .unwrap();
        let DynamicEpisodeSetOutcome::Bounded(tube) = outcome else {
            panic!("expected a bounded tube");
        };
        assert_eq!(tube.radius_m(), 0.99);
        assert_eq!(tube.selected_order_statistic_rank(), 99);
        assert_eq!(tube.decision_sequence_id(), 501);
        assert_eq!(tube.decision_receipt_sha256(), [0xD4; SHA256_BYTES]);
        assert!(tube.contains_position(0, [0.5, 0.0]));
        assert!(!tube.contains_position(2, [3.0, 3.0]));
    }

    #[test]
    fn whole_episode_scoring_requires_every_truth_position() {
        let family = family(99);
        let outcome = family
            .build_episode_tube(
                500,
                20_500,
                &[[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]],
                0.01,
                [0xB2; SHA256_BYTES],
                [0xC3; SHA256_BYTES],
                501,
                [0xD4; SHA256_BYTES],
            )
            .unwrap();
        let DynamicEpisodeSetOutcome::Bounded(tube) = outcome else {
            panic!("expected a bounded tube");
        };
        assert!(tube
            .contains_complete_trajectory(&[[0.1, 0.0], [1.0, 1.1], [2.0, 2.0]])
            .unwrap());
        assert!(!tube
            .contains_complete_trajectory(&[[0.1, 0.0], [1.0, 1.1], [4.0, 4.0]])
            .unwrap());
        assert_eq!(
            tube.contains_complete_trajectory(&[[0.1, 0.0], [1.0, 1.1]]),
            Err(DynamicEpisodeSetError::TruthPositionCountMismatch {
                expected: 3,
                observed: 2
            })
        );
        assert_eq!(
            tube.contains_complete_trajectory(&[[0.1, 0.0], [f64::NAN, 1.1], [2.0, 2.0]]),
            Err(DynamicEpisodeSetError::InvalidTruthPosition(1))
        );
    }

    #[test]
    fn dynamic_radius_is_monotone_in_requested_miscoverage() {
        let family = family(199);
        let mut previous = f64::INFINITY;
        for eta in [0.005, 0.01, 0.02, 0.05, 0.10, 0.20, 0.40] {
            let selected = family.select_radius(eta).unwrap().unwrap();
            assert!(selected.radius_m <= previous);
            previous = selected.radius_m;
        }
    }

    #[test]
    fn eta_at_or_above_one_emits_empty_bounded_set_not_abstention() {
        let family = family(19);
        for eta in [1.0, 1.25, 10.0] {
            let outcome = family
                .build_episode_tube(
                    500,
                    20_500,
                    &[[0.0, 0.0], [1.0, 1.0]],
                    eta,
                    [0xB2; SHA256_BYTES],
                    [0xC3; SHA256_BYTES],
                    501,
                    [0xD4; SHA256_BYTES],
                )
                .unwrap();
            let DynamicEpisodeSetOutcome::EmptyBounded(empty) = outcome else {
                panic!("eta >= 1 must be an explicit empty bounded set");
            };
            assert_eq!(empty.requested_miscoverage(), eta);
            assert_eq!(empty.position_count(), 2);
            assert!(!empty.contains_position(0, [0.0, 0.0]));
            assert!(!empty.contains_complete_trajectory(&[[0.0, 0.0], [1.0, 1.0]]).unwrap());
            assert_eq!(
                empty.contains_complete_trajectory(&[[0.0, 0.0]]),
                Err(DynamicEpisodeSetError::TruthPositionCountMismatch {
                    expected: 2,
                    observed: 1
                })
            );
        }
    }

    #[test]
    fn eta_at_or_above_one_does_not_consume_calibration_rank() {
        let family = family(1);
        let outcome = family
            .build_episode_tube(
                500,
                20_500,
                &[[0.0, 0.0]],
                1.0,
                [0xB2; SHA256_BYTES],
                [0xC3; SHA256_BYTES],
                501,
                [0xD4; SHA256_BYTES],
            )
            .unwrap();
        assert!(matches!(outcome, DynamicEpisodeSetOutcome::EmptyBounded(_)));
    }

    #[test]
    fn missing_calibration_commitments_fail_closed() {
        let data = episodes(10);
        let mut cfg = config();
        cfg.calibration_cohort_sha256 = [0; SHA256_BYTES];
        assert_eq!(
            DynamicEpisodeConformalFamily2d::fit(&data, cfg),
            Err(DynamicEpisodeSetError::MissingCalibrationCohortCommitment)
        );
        cfg = config();
        cfg.predictor_sha256 = [0; SHA256_BYTES];
        assert_eq!(
            DynamicEpisodeConformalFamily2d::fit(&data, cfg),
            Err(DynamicEpisodeSetError::MissingPredictorCommitment)
        );
        cfg = config();
        cfg.score_definition_sha256 = [0; SHA256_BYTES];
        assert_eq!(
            DynamicEpisodeConformalFamily2d::fit(&data, cfg),
            Err(DynamicEpisodeSetError::MissingScoreDefinitionCommitment)
        );
    }

    #[test]
    fn insufficient_support_produces_explicit_abstention() {
        let family = family(98);
        assert_eq!(
            family
                .build_episode_tube(
                    500,
                    20_500,
                    &[[0.0, 0.0]],
                    0.01,
                    [0xB2; SHA256_BYTES],
                    [0xC3; SHA256_BYTES],
                    501,
                    [0xD4; SHA256_BYTES],
                )
                .unwrap(),
            DynamicEpisodeSetOutcome::Abstain(
                DynamicEpisodeSetAbstention::InsufficientCalibrationRank {
                    needed_rank: 99,
                    available_episodes: 98
                }
            )
        );
    }

    #[test]
    fn calibration_evaluation_groups_and_predictor_contract_are_separate() {
        let family = family(19);
        assert_eq!(
            family.build_episode_tube(
                1,
                20_001,
                &[[0.0, 0.0]],
                0.05,
                [0xB2; SHA256_BYTES],
                [0xC3; SHA256_BYTES],
                501,
                [0xD4; SHA256_BYTES],
            ),
            Err(DynamicEpisodeSetError::EvaluationEpisodeReused(1))
        );
        assert_eq!(
            family.build_episode_tube(
                500,
                10_001,
                &[[0.0, 0.0]],
                0.05,
                [0xB2; SHA256_BYTES],
                [0xC3; SHA256_BYTES],
                501,
                [0xD4; SHA256_BYTES],
            ),
            Err(DynamicEpisodeSetError::EvaluationIndependenceGroupReused(10_001))
        );
        assert_eq!(
            family.build_episode_tube(
                500,
                20_500,
                &[[0.0, 0.0]],
                0.05,
                [0xE5; SHA256_BYTES],
                [0xC3; SHA256_BYTES],
                501,
                [0xD4; SHA256_BYTES],
            ),
            Err(DynamicEpisodeSetError::PredictorCommitmentMismatch)
        );
        assert_eq!(
            family.build_episode_tube(
                500,
                20_500,
                &[[0.0, 0.0]],
                0.05,
                [0xB2; SHA256_BYTES],
                [0xE5; SHA256_BYTES],
                501,
                [0xD4; SHA256_BYTES],
            ),
            Err(DynamicEpisodeSetError::ScoreDefinitionMismatch)
        );
    }

    #[test]
    fn invalid_source_scores_and_decision_receipts_fail_closed() {
        let mut calibration = episodes(2);
        calibration[0].maximum_position_error_m = f64::NAN;
        assert_eq!(
            DynamicEpisodeConformalFamily2d::fit(&calibration, config()),
            Err(DynamicEpisodeSetError::InvalidCalibrationScore(1))
        );
        let family = family(19);
        assert_eq!(
            family.build_episode_tube(
                500,
                20_500,
                &[[0.0, 0.0]],
                0.05,
                [0xB2; SHA256_BYTES],
                [0xC3; SHA256_BYTES],
                0,
                [0xD4; SHA256_BYTES],
            ),
            Err(DynamicEpisodeSetError::MissingDecisionSequence)
        );
        assert_eq!(
            family.build_episode_tube(
                500,
                20_500,
                &[[0.0, 0.0]],
                0.05,
                [0xB2; SHA256_BYTES],
                [0xC3; SHA256_BYTES],
                501,
                [0; SHA256_BYTES],
            ),
            Err(DynamicEpisodeSetError::MissingDecisionReceipt)
        );
    }
}
