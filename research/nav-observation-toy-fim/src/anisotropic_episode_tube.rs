//! Frozen-shape anisotropic conformal tubes for whole trajectory episodes.
//!
//! Each position has a frozen symmetric positive-definite shape matrix A_t.
//! The calibration score is the maximum normalized position error over the
//! entire episode:
//!
//!   S_i = max_t sqrt((xhat_it - x_it)^T A_it^{-1} (xhat_it - x_it)).
//!
//! A split-conformal order statistic q_eta scales every ellipse:
//!
//!   C_t(eta) = {x: (x - xhat_t)^T A_t^{-1} (x - xhat_t) <= q_eta^2}.
//!
//! The shape matrices are geometric descriptors, not automatically calibrated
//! covariances. The predictor, shape generator and score definition must be
//! frozen and digest-bound at calibration. Whole-episode marginal calibration
//! still requires exchangeable independent episode scores.

use std::collections::BTreeSet;
use std::fmt;

const SHA256_BYTES: usize = 32;
const DIM: usize = 2;
const PI: f64 = std::f64::consts::PI;

#[derive(Clone, Debug, PartialEq)]
pub struct AnisotropicCalibrationEpisode2d {
    pub episode_id: u64,
    pub independence_group_id: u64,
    pub predicted_positions_m: Vec<[f64; DIM]>,
    pub truth_positions_m: Vec<[f64; DIM]>,
    pub shape_matrices: Vec<[[f64; DIM]; DIM]>,
    pub source_manifest_sha256: [u8; SHA256_BYTES],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnisotropicEpisodeFamilyConfig {
    pub calibration_cohort_sha256: [u8; SHA256_BYTES],
    pub predictor_sha256: [u8; SHA256_BYTES],
    /// Commits to the shape generator, matrices' units, normalization and score.
    pub shape_definition_sha256: [u8; SHA256_BYTES],
    /// Relative tolerance for symmetry of each supplied shape matrix.
    pub symmetry_tolerance: f64,
    /// Reject matrices whose smaller eigenvalue / larger eigenvalue is below this.
    pub minimum_relative_eigenvalue: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnisotropicEpisodeRadius2d {
    pub requested_miscoverage: f64,
    pub requested_coverage: f64,
    pub calibration_episode_count: usize,
    pub selected_order_statistic_rank: usize,
    /// Dimensionless scale applied to every frozen shape matrix.
    pub normalized_radius: f64,
    pub calibration_cohort_sha256: [u8; SHA256_BYTES],
    pub predictor_sha256: [u8; SHA256_BYTES],
    pub shape_definition_sha256: [u8; SHA256_BYTES],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnisotropicEpisodeAbstention {
    UnsupportedMiscoverageLevel,
    InsufficientCalibrationRank {
        needed_rank: usize,
        available_episodes: usize,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum AnisotropicEpisodeSetOutcome2d {
    Ellipses(AnisotropicEpisodeTube2d),
    EmptyBounded(AnisotropicEmptyEpisodeSet2d),
    Abstain(AnisotropicEpisodeAbstention),
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnisotropicEmptyEpisodeSet2d {
    episode_id: u64,
    independence_group_id: u64,
    position_count: usize,
    requested_miscoverage: f64,
    decision_sequence_id: u64,
    decision_receipt_sha256: [u8; SHA256_BYTES],
}

impl AnisotropicEmptyEpisodeSet2d {
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

    pub fn contains_complete_trajectory(
        &self,
        truth_positions_m: &[[f64; DIM]],
    ) -> Result<bool, AnisotropicEpisodeSetError> {
        validate_truth_positions(truth_positions_m, self.position_count)?;
        Ok(false)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnisotropicEpisodeTube2d {
    episode_id: u64,
    independence_group_id: u64,
    centers_m: Vec<[f64; DIM]>,
    shape_matrices: Vec<[[f64; DIM]; DIM]>,
    normalized_radius: f64,
    requested_miscoverage: f64,
    requested_coverage: f64,
    calibration_episode_count: usize,
    selected_order_statistic_rank: usize,
    calibration_cohort_sha256: [u8; SHA256_BYTES],
    predictor_sha256: [u8; SHA256_BYTES],
    shape_definition_sha256: [u8; SHA256_BYTES],
    decision_sequence_id: u64,
    decision_receipt_sha256: [u8; SHA256_BYTES],
}

impl AnisotropicEpisodeTube2d {
    pub fn episode_id(&self) -> u64 {
        self.episode_id
    }

    pub fn independence_group_id(&self) -> u64 {
        self.independence_group_id
    }

    pub fn centers_m(&self) -> &[[f64; DIM]] {
        &self.centers_m
    }

    pub fn shape_matrices(&self) -> &[[[f64; DIM]; DIM]] {
        &self.shape_matrices
    }

    pub fn normalized_radius(&self) -> f64 {
        self.normalized_radius
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

    pub fn shape_definition_sha256(&self) -> [u8; SHA256_BYTES] {
        self.shape_definition_sha256
    }

    pub fn decision_sequence_id(&self) -> u64 {
        self.decision_sequence_id
    }

    pub fn decision_receipt_sha256(&self) -> [u8; SHA256_BYTES] {
        self.decision_receipt_sha256
    }

    pub fn contains_position(
        &self,
        time_index: usize,
        truth_position_m: [f64; DIM],
    ) -> Result<bool, AnisotropicEpisodeSetError> {
        let center = self
            .centers_m
            .get(time_index)
            .ok_or(AnisotropicEpisodeSetError::TruthPositionIndexOutOfRange(time_index))?;
        let shape = self
            .shape_matrices
            .get(time_index)
            .ok_or(AnisotropicEpisodeSetError::TruthPositionIndexOutOfRange(time_index))?;
        if !truth_position_m.iter().all(|value| value.is_finite()) {
            return Err(AnisotropicEpisodeSetError::InvalidTruthPosition(time_index));
        }
        let error = [
            truth_position_m[0] - center[0],
            truth_position_m[1] - center[1],
        ];
        if !error.iter().all(|value| value.is_finite()) {
            return Err(AnisotropicEpisodeSetError::ArithmeticOverflow);
        }
        let normalized_error = normalized_mahalanobis_radius(
            error,
            *shape,
            // The matrices were already validated at construction time.
            1e-12,
            1e-12,
        )?;
        Ok(normalized_error <= self.normalized_radius)
    }

    /// Evaluate the joint event: every truth position lies within its ellipse.
    /// Truncated/missing/non-finite truth is an error, never a partial hit.
    pub fn contains_complete_trajectory(
        &self,
        truth_positions_m: &[[f64; DIM]],
    ) -> Result<bool, AnisotropicEpisodeSetError> {
        validate_truth_positions(truth_positions_m, self.centers_m.len())?;
        for (index, truth) in truth_positions_m.iter().enumerate() {
            if !self.contains_position(index, *truth)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Sum the areas of the per-position ellipses.
    pub fn total_area_m2(&self) -> Result<f64, AnisotropicEpisodeSetError> {
        let radius_squared = self.normalized_radius * self.normalized_radius;
        if !radius_squared.is_finite() {
            return Err(AnisotropicEpisodeSetError::ArithmeticOverflow);
        }
        let mut total = 0.0_f64;
        for shape in &self.shape_matrices {
            let determinant_scale = shape[0][0].abs()
                .max(shape[0][1].abs())
                .max(shape[1][0].abs())
                .max(shape[1][1].abs());
            if !determinant_scale.is_finite() || determinant_scale <= 0.0 {
                return Err(AnisotropicEpisodeSetError::InvalidShapeMatrix);
            }
            let a = shape[0][0] / determinant_scale;
            let b = (shape[0][1] / determinant_scale) / 2.0
                + (shape[1][0] / determinant_scale) / 2.0;
            let d = shape[1][1] / determinant_scale;
            let determinant_scaled = a * d - b * b;
            if !determinant_scaled.is_finite() || determinant_scaled <= 0.0 {
                return Err(AnisotropicEpisodeSetError::InvalidShapeMatrix);
            }
            // sqrt(det(A)) = scale * sqrt(det(A/scale)).
            let area = PI
                * radius_squared
                * determinant_scale
                * determinant_scaled.sqrt();
            if !area.is_finite() || area < 0.0 {
                return Err(AnisotropicEpisodeSetError::ArithmeticOverflow);
            }
            total += area;
            if !total.is_finite() {
                return Err(AnisotropicEpisodeSetError::ArithmeticOverflow);
            }
        }
        Ok(total)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnisotropicEpisodeSetError {
    EmptyCalibrationCohort,
    MissingCalibrationCohortCommitment,
    MissingPredictorCommitment,
    MissingShapeDefinitionCommitment,
    InvalidShapeTolerance,
    InvalidRelativeEigenvalueFloor,
    InvalidCalibrationEpisodeIdentity,
    DuplicateCalibrationEpisode(u64),
    DuplicateCalibrationIndependenceGroup(u64),
    EmptyCalibrationTrajectory(u64),
    CalibrationTrajectoryLengthMismatch(u64),
    InvalidCalibrationPosition { episode_id: u64, time_index: usize },
    MissingCalibrationSourceCommitment(u64),
    InvalidShapeMatrix,
    AsymmetricShapeMatrix,
    IllConditionedShapeMatrix,
    EvaluationIdentityInvalid,
    EvaluationEpisodeReused(u64),
    EvaluationIndependenceGroupReused(u64),
    PredictorCommitmentMismatch,
    ShapeDefinitionMismatch,
    EmptyPredictedTrajectory,
    InvalidPredictedPosition(usize),
    MissingDecisionSequence,
    MissingDecisionReceipt,
    TruthPositionCountMismatch { expected: usize, observed: usize },
    InvalidTruthPosition(usize),
    TruthPositionIndexOutOfRange(usize),
    ArithmeticOverflow,
}

impl fmt::Display for AnisotropicEpisodeSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCalibrationCohort => write!(f, "calibration cohort must contain at least one independent episode"),
            Self::MissingCalibrationCohortCommitment => write!(f, "calibration cohort commitment is missing"),
            Self::MissingPredictorCommitment => write!(f, "frozen predictor commitment is missing"),
            Self::MissingShapeDefinitionCommitment => write!(f, "shape-definition commitment is missing"),
            Self::InvalidShapeTolerance => write!(f, "shape symmetry tolerance must be finite and nonnegative"),
            Self::InvalidRelativeEigenvalueFloor => write!(f, "minimum relative eigenvalue must be finite and in (0, 1)"),
            Self::InvalidCalibrationEpisodeIdentity => write!(f, "calibration episode/group IDs must be nonzero"),
            Self::DuplicateCalibrationEpisode(id) => write!(f, "duplicate calibration episode ID: {id}"),
            Self::DuplicateCalibrationIndependenceGroup(id) => write!(f, "duplicate calibration independence group: {id}"),
            Self::EmptyCalibrationTrajectory(id) => write!(f, "calibration episode {id} is empty"),
            Self::CalibrationTrajectoryLengthMismatch(id) => write!(f, "calibration episode {id} has mismatched prediction, truth and shape lengths"),
            Self::InvalidCalibrationPosition { episode_id, time_index } => write!(f, "calibration episode {episode_id} has invalid position at index {time_index}"),
            Self::MissingCalibrationSourceCommitment(id) => write!(f, "calibration episode {id} is missing its source commitment"),
            Self::InvalidShapeMatrix => write!(f, "shape matrix is non-finite or not positive definite"),
            Self::AsymmetricShapeMatrix => write!(f, "shape matrix is not symmetric within tolerance"),
            Self::IllConditionedShapeMatrix => write!(f, "shape matrix is below the configured relative eigenvalue floor"),
            Self::EvaluationIdentityInvalid => write!(f, "evaluation episode/group IDs must be nonzero"),
            Self::EvaluationEpisodeReused(id) => write!(f, "evaluation episode {id} overlaps calibration"),
            Self::EvaluationIndependenceGroupReused(id) => write!(f, "evaluation independence group {id} overlaps calibration"),
            Self::PredictorCommitmentMismatch => write!(f, "evaluation predictor differs from the calibrated predictor"),
            Self::ShapeDefinitionMismatch => write!(f, "evaluation shape definition differs from the calibrated shape definition"),
            Self::EmptyPredictedTrajectory => write!(f, "evaluation prediction trajectory is empty"),
            Self::InvalidPredictedPosition(index) => write!(f, "evaluation predicted position {index} is non-finite"),
            Self::MissingDecisionSequence => write!(f, "decision sequence ID must be nonzero"),
            Self::MissingDecisionReceipt => write!(f, "decision receipt commitment is missing"),
            Self::TruthPositionCountMismatch { expected, observed } => write!(f, "truth trajectory has {observed} positions but prediction has {expected}"),
            Self::InvalidTruthPosition(index) => write!(f, "truth position {index} is non-finite"),
            Self::TruthPositionIndexOutOfRange(index) => write!(f, "truth position index {index} is out of range"),
            Self::ArithmeticOverflow => write!(f, "anisotropic set arithmetic overflowed"),
        }
    }
}

impl std::error::Error for AnisotropicEpisodeSetError {}

#[derive(Clone, Debug, PartialEq)]
pub struct AnisotropicEpisodeConformalFamily2d {
    config: AnisotropicEpisodeFamilyConfig,
    ordered_scores: Vec<(f64, u64)>,
    calibration_episode_ids: Vec<u64>,
    calibration_independence_group_ids: Vec<u64>,
}

impl AnisotropicEpisodeConformalFamily2d {
    pub fn fit(
        episodes: &[AnisotropicCalibrationEpisode2d],
        config: AnisotropicEpisodeFamilyConfig,
    ) -> Result<Self, AnisotropicEpisodeSetError> {
        if episodes.is_empty() {
            return Err(AnisotropicEpisodeSetError::EmptyCalibrationCohort);
        }
        if config.calibration_cohort_sha256.iter().all(|byte| *byte == 0) {
            return Err(AnisotropicEpisodeSetError::MissingCalibrationCohortCommitment);
        }
        if config.predictor_sha256.iter().all(|byte| *byte == 0) {
            return Err(AnisotropicEpisodeSetError::MissingPredictorCommitment);
        }
        if config.shape_definition_sha256.iter().all(|byte| *byte == 0) {
            return Err(AnisotropicEpisodeSetError::MissingShapeDefinitionCommitment);
        }
        if !config.symmetry_tolerance.is_finite() || config.symmetry_tolerance < 0.0 {
            return Err(AnisotropicEpisodeSetError::InvalidShapeTolerance);
        }
        if !config.minimum_relative_eigenvalue.is_finite()
            || config.minimum_relative_eigenvalue <= 0.0
            || config.minimum_relative_eigenvalue >= 1.0
        {
            return Err(AnisotropicEpisodeSetError::InvalidRelativeEigenvalueFloor);
        }

        let mut episode_ids = BTreeSet::new();
        let mut group_ids = BTreeSet::new();
        let mut ordered_scores = Vec::with_capacity(episodes.len());

        for episode in episodes {
            if episode.episode_id == 0 || episode.independence_group_id == 0 {
                return Err(AnisotropicEpisodeSetError::InvalidCalibrationEpisodeIdentity);
            }
            if !episode_ids.insert(episode.episode_id) {
                return Err(AnisotropicEpisodeSetError::DuplicateCalibrationEpisode(
                    episode.episode_id,
                ));
            }
            if !group_ids.insert(episode.independence_group_id) {
                return Err(
                    AnisotropicEpisodeSetError::DuplicateCalibrationIndependenceGroup(
                        episode.independence_group_id,
                    ),
                );
            }
            if episode.predicted_positions_m.is_empty() {
                return Err(AnisotropicEpisodeSetError::EmptyCalibrationTrajectory(
                    episode.episode_id,
                ));
            }
            if episode.truth_positions_m.len() != episode.predicted_positions_m.len()
                || episode.shape_matrices.len() != episode.predicted_positions_m.len()
            {
                return Err(AnisotropicEpisodeSetError::CalibrationTrajectoryLengthMismatch(
                    episode.episode_id,
                ));
            }
            if episode.source_manifest_sha256.iter().all(|byte| *byte == 0) {
                return Err(AnisotropicEpisodeSetError::MissingCalibrationSourceCommitment(
                    episode.episode_id,
                ));
            }

            let mut maximum_score = 0.0_f64;
            for time_index in 0..episode.predicted_positions_m.len() {
                let predicted = episode.predicted_positions_m[time_index];
                let truth = episode.truth_positions_m[time_index];
                if !predicted.iter().all(|value| value.is_finite())
                    || !truth.iter().all(|value| value.is_finite())
                {
                    return Err(AnisotropicEpisodeSetError::InvalidCalibrationPosition {
                        episode_id: episode.episode_id,
                        time_index,
                    });
                }
                let error = [truth[0] - predicted[0], truth[1] - predicted[1]];
                if !error.iter().all(|value| value.is_finite()) {
                    return Err(AnisotropicEpisodeSetError::ArithmeticOverflow);
                }
                validate_shape_matrix(
                    episode.shape_matrices[time_index],
                    config.symmetry_tolerance,
                    config.minimum_relative_eigenvalue,
                )?;
                let score = normalized_mahalanobis_radius(
                    error,
                    episode.shape_matrices[time_index],
                    config.symmetry_tolerance,
                    config.minimum_relative_eigenvalue,
                )?;
                maximum_score = maximum_score.max(score);
            }
            ordered_scores.push((maximum_score, episode.episode_id));
        }

        ordered_scores.sort_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1)
        });

        Ok(Self {
            config,
            ordered_scores,
            calibration_episode_ids: episode_ids.into_iter().collect(),
            calibration_independence_group_ids: group_ids.into_iter().collect(),
        })
    }

    pub fn calibration_episode_count(&self) -> usize {
        self.ordered_scores.len()
    }

    pub fn select_radius(
        &self,
        requested_miscoverage: f64,
    ) -> Result<Result<AnisotropicEpisodeRadius2d, AnisotropicEpisodeAbstention>, AnisotropicEpisodeSetError>
    {
        if !requested_miscoverage.is_finite() || requested_miscoverage <= 0.0 || requested_miscoverage >= 1.0 {
            return Ok(Err(AnisotropicEpisodeAbstention::UnsupportedMiscoverageLevel));
        }
        let n = self.ordered_scores.len();
        let rank_float = (n as f64 + 1.0) * (1.0 - requested_miscoverage);
        if !rank_float.is_finite() || rank_float < 1.0 || rank_float > usize::MAX as f64 {
            return Err(AnisotropicEpisodeSetError::ArithmeticOverflow);
        }
        let rank = rank_float.ceil() as usize;
        if rank > n {
            return Ok(Err(AnisotropicEpisodeAbstention::InsufficientCalibrationRank {
                needed_rank: rank,
                available_episodes: n,
            }));
        }
        Ok(Ok(AnisotropicEpisodeRadius2d {
            requested_miscoverage,
            requested_coverage: 1.0 - requested_miscoverage,
            calibration_episode_count: n,
            selected_order_statistic_rank: rank,
            normalized_radius: self.ordered_scores[rank - 1].0,
            calibration_cohort_sha256: self.config.calibration_cohort_sha256,
            predictor_sha256: self.config.predictor_sha256,
            shape_definition_sha256: self.config.shape_definition_sha256,
        }))
    }

    pub fn build_episode_tube(
        &self,
        episode_id: u64,
        independence_group_id: u64,
        predicted_positions_m: &[[f64; DIM]],
        shape_matrices: &[[[f64; DIM]; DIM]],
        requested_miscoverage: f64,
        predictor_sha256: [u8; SHA256_BYTES],
        shape_definition_sha256: [u8; SHA256_BYTES],
        decision_sequence_id: u64,
        decision_receipt_sha256: [u8; SHA256_BYTES],
    ) -> Result<AnisotropicEpisodeSetOutcome2d, AnisotropicEpisodeSetError> {
        if episode_id == 0 || independence_group_id == 0 {
            return Err(AnisotropicEpisodeSetError::EvaluationIdentityInvalid);
        }
        if self.calibration_episode_ids.contains(&episode_id) {
            return Err(AnisotropicEpisodeSetError::EvaluationEpisodeReused(episode_id));
        }
        if self.calibration_independence_group_ids.contains(&independence_group_id) {
            return Err(AnisotropicEpisodeSetError::EvaluationIndependenceGroupReused(
                independence_group_id,
            ));
        }
        if predictor_sha256.iter().all(|byte| *byte == 0) {
            return Err(AnisotropicEpisodeSetError::MissingPredictorCommitment);
        }
        if predictor_sha256 != self.config.predictor_sha256 {
            return Err(AnisotropicEpisodeSetError::PredictorCommitmentMismatch);
        }
        if shape_definition_sha256.iter().all(|byte| *byte == 0) {
            return Err(AnisotropicEpisodeSetError::MissingShapeDefinitionCommitment);
        }
        if shape_definition_sha256 != self.config.shape_definition_sha256 {
            return Err(AnisotropicEpisodeSetError::ShapeDefinitionMismatch);
        }
        if predicted_positions_m.is_empty() {
            return Err(AnisotropicEpisodeSetError::EmptyPredictedTrajectory);
        }
        if predicted_positions_m.len() != shape_matrices.len() {
            return Err(AnisotropicEpisodeSetError::CalibrationTrajectoryLengthMismatch(episode_id));
        }
        if let Some(index) = predicted_positions_m.iter().position(|position| {
            !position.iter().all(|value| value.is_finite())
        }) {
            return Err(AnisotropicEpisodeSetError::InvalidPredictedPosition(index));
        }
        for shape in shape_matrices {
            validate_shape_matrix(
                *shape,
                self.config.symmetry_tolerance,
                self.config.minimum_relative_eigenvalue,
            )?;
        }
        if decision_sequence_id == 0 {
            return Err(AnisotropicEpisodeSetError::MissingDecisionSequence);
        }
        if decision_receipt_sha256.iter().all(|byte| *byte == 0) {
            return Err(AnisotropicEpisodeSetError::MissingDecisionReceipt);
        }
        if !requested_miscoverage.is_finite() || requested_miscoverage <= 0.0 {
            return Ok(AnisotropicEpisodeSetOutcome2d::Abstain(
                AnisotropicEpisodeAbstention::UnsupportedMiscoverageLevel,
            ));
        }
        if requested_miscoverage >= 1.0 {
            return Ok(AnisotropicEpisodeSetOutcome2d::EmptyBounded(
                AnisotropicEmptyEpisodeSet2d {
                    episode_id,
                    independence_group_id,
                    position_count: predicted_positions_m.len(),
                    requested_miscoverage,
                    decision_sequence_id,
                    decision_receipt_sha256,
                },
            ));
        }

        let radius = match self.select_radius(requested_miscoverage)? {
            Ok(radius) => radius,
            Err(abstention) => return Ok(AnisotropicEpisodeSetOutcome2d::Abstain(abstention)),
        };
        Ok(AnisotropicEpisodeSetOutcome2d::Ellipses(AnisotropicEpisodeTube2d {
            episode_id,
            independence_group_id,
            centers_m: predicted_positions_m.to_vec(),
            shape_matrices: shape_matrices.to_vec(),
            normalized_radius: radius.normalized_radius,
            requested_miscoverage,
            requested_coverage: radius.requested_coverage,
            calibration_episode_count: radius.calibration_episode_count,
            selected_order_statistic_rank: radius.selected_order_statistic_rank,
            calibration_cohort_sha256: radius.calibration_cohort_sha256,
            predictor_sha256: radius.predictor_sha256,
            shape_definition_sha256: radius.shape_definition_sha256,
            decision_sequence_id,
            decision_receipt_sha256,
        }))
    }
}

fn validate_truth_positions(
    truth_positions_m: &[[f64; DIM]],
    expected_length: usize,
) -> Result<(), AnisotropicEpisodeSetError> {
    if truth_positions_m.len() != expected_length {
        return Err(AnisotropicEpisodeSetError::TruthPositionCountMismatch {
            expected: expected_length,
            observed: truth_positions_m.len(),
        });
    }
    if let Some(index) = truth_positions_m
        .iter()
        .position(|position| !position.iter().all(|value| value.is_finite()))
    {
        return Err(AnisotropicEpisodeSetError::InvalidTruthPosition(index));
    }
    Ok(())
}

/// Validate a scale-free symmetric 2x2 SPD descriptor.
fn validate_shape_matrix(
    shape: [[f64; DIM]; DIM],
    symmetry_tolerance: f64,
    minimum_relative_eigenvalue: f64,
) -> Result<(), AnisotropicEpisodeSetError> {
    let values = shape.iter().flatten().copied().collect::<Vec<_>>();
    if !values.iter().all(|value| value.is_finite()) {
        return Err(AnisotropicEpisodeSetError::InvalidShapeMatrix);
    }
    let scale = values.iter().map(|value| value.abs()).fold(0.0_f64, f64::max);
    if scale <= 0.0 {
        return Err(AnisotropicEpisodeSetError::InvalidShapeMatrix);
    }
    let upper = shape[0][1];
    let lower = shape[1][0];
    if (upper - lower).abs() > symmetry_tolerance * scale {
        return Err(AnisotropicEpisodeSetError::AsymmetricShapeMatrix);
    }
    let a = shape[0][0] / scale;
    let b = upper / scale / 2.0 + lower / scale / 2.0;
    let d = shape[1][1] / scale;
    if a <= 0.0 || d <= 0.0 {
        return Err(AnisotropicEpisodeSetError::InvalidShapeMatrix);
    }
    let half_trace = a / 2.0 + d / 2.0;
    let half_spread = 0.5 * (a - d).hypot(2.0 * b);
    let lambda_max = half_trace + half_spread;
    let lambda_min = half_trace - half_spread;
    if !lambda_max.is_finite()
        || !lambda_min.is_finite()
        || lambda_max <= 0.0
        || lambda_min <= 0.0
    {
        return Err(AnisotropicEpisodeSetError::InvalidShapeMatrix);
    }
    if lambda_min / lambda_max < minimum_relative_eigenvalue {
        return Err(AnisotropicEpisodeSetError::IllConditionedShapeMatrix);
    }
    Ok(())
}

/// Compute sqrt(e^T A^-1 e) using a scaled Cholesky solve.
fn normalized_mahalanobis_radius(
    error_m: [f64; DIM],
    shape: [[f64; DIM]; DIM],
    symmetry_tolerance: f64,
    minimum_relative_eigenvalue: f64,
) -> Result<f64, AnisotropicEpisodeSetError> {
    validate_shape_matrix(shape, symmetry_tolerance, minimum_relative_eigenvalue)?;
    if !error_m.iter().all(|value| value.is_finite()) {
        return Err(AnisotropicEpisodeSetError::ArithmeticOverflow);
    }
    let scale = shape[0][0].abs()
        .max(shape[0][1].abs())
        .max(shape[1][0].abs())
        .max(shape[1][1].abs());
    let a = shape[0][0] / scale;
    let b = shape[0][1] / scale / 2.0 + shape[1][0] / scale / 2.0;
    let d = shape[1][1] / scale;
    let l11 = a.sqrt();
    let l21 = b / l11;
    let remainder = d - l21 * l21;
    if !remainder.is_finite() || remainder <= 0.0 {
        return Err(AnisotropicEpisodeSetError::InvalidShapeMatrix);
    }
    let l22 = remainder.sqrt();
    let root_scale = scale.sqrt();
    let y0 = (error_m[0] / root_scale) / l11;
    let y1 = ((error_m[1] / root_scale) - l21 * y0) / l22;
    let radius = y0.hypot(y1);
    if !radius.is_finite() {
        return Err(AnisotropicEpisodeSetError::ArithmeticOverflow);
    }
    Ok(radius)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> AnisotropicEpisodeFamilyConfig {
        AnisotropicEpisodeFamilyConfig {
            calibration_cohort_sha256: [0xA1; SHA256_BYTES],
            predictor_sha256: [0xB2; SHA256_BYTES],
            shape_definition_sha256: [0xC3; SHA256_BYTES],
            symmetry_tolerance: 1e-12,
            minimum_relative_eigenvalue: 1e-8,
        }
    }

    fn episode(
        id: u64,
        predicted: [f64; 2],
        truth: [f64; 2],
        shape: [[f64; 2]; 2],
    ) -> AnisotropicCalibrationEpisode2d {
        AnisotropicCalibrationEpisode2d {
            episode_id: id,
            independence_group_id: id + 10_000,
            predicted_positions_m: vec![predicted],
            truth_positions_m: vec![truth],
            shape_matrices: vec![shape],
            source_manifest_sha256: [((id % 255) + 1) as u8; SHA256_BYTES],
        }
    }

    fn calibration() -> Vec<AnisotropicCalibrationEpisode2d> {
        let shape = [[4.0, 0.0], [0.0, 1.0]];
        vec![
            episode(1, [0.0, 0.0], [1.0, 0.0], shape), // normalized score 0.5
            episode(2, [0.0, 0.0], [0.0, 1.0], shape), // score 1
            episode(3, [0.0, 0.0], [2.0, 0.0], shape), // score 1
            episode(4, [0.0, 0.0], [0.0, 2.0], shape), // score 2
        ]
    }

    fn family() -> AnisotropicEpisodeConformalFamily2d {
        AnisotropicEpisodeConformalFamily2d::fit(&calibration(), config()).unwrap()
    }

    #[test]
    fn calibration_scores_use_the_full_episode_mahalanobis_maximum() {
        let episodes = vec![
            AnisotropicCalibrationEpisode2d {
                episode_id: 1,
                independence_group_id: 1,
                predicted_positions_m: vec![[0.0, 0.0], [0.0, 0.0]],
                truth_positions_m: vec![[1.0, 0.0], [0.0, 2.0]],
                shape_matrices: vec![
                    [[4.0, 0.0], [0.0, 1.0]],
                    [[4.0, 0.0], [0.0, 1.0]],
                ],
                source_manifest_sha256: [0xA1; SHA256_BYTES],
            },
            episode(2, [0.0, 0.0], [0.0, 1.0], [[4.0, 0.0], [0.0, 1.0]]),
        ];
        let family = AnisotropicEpisodeConformalFamily2d::fit(&episodes, config()).unwrap();
        let selected = family.select_radius(0.5).unwrap().unwrap();
        assert_eq!(selected.selected_order_statistic_rank, 2);
        assert!((selected.normalized_radius - 2.0).abs() < 1e-12);
    }

    #[test]
    fn anisotropic_shapes_have_directional_support_and_correct_area() {
        let family = family();
        let outcome = family
            .build_episode_tube(
                500,
                20_500,
                &[[0.0, 0.0]],
                &[[[4.0, 0.0], [0.0, 1.0]]],
                0.2, // rank 4 of 4, q=2
                [0xB2; SHA256_BYTES],
                [0xC3; SHA256_BYTES],
                501,
                [0xD4; SHA256_BYTES],
            )
            .unwrap();
        let AnisotropicEpisodeSetOutcome2d::Ellipses(tube) = outcome else {
            panic!("expected calibrated ellipses");
        };
        assert!((tube.normalized_radius() - 2.0).abs() < 1e-12);
        assert!(tube.contains_position(0, [4.0, 0.0]).unwrap());
        assert!(tube.contains_position(0, [0.0, 2.0]).unwrap());
        assert!(!tube.contains_position(0, [0.0, 2.01]).unwrap());
        // Area = pi * q^2 * sqrt(det(A)) = pi * 4 * 2.
        assert!((tube.total_area_m2().unwrap() - (8.0 * PI)).abs() < 1e-10);
    }

    #[test]
    fn increasing_miscoverage_cannot_increase_the_selected_normalized_radius() {
        let family = family();
        let mut previous = f64::INFINITY;
        for eta in [0.05, 0.10, 0.20, 0.30, 0.40, 0.60] {
            let selected = family.select_radius(eta).unwrap().unwrap();
            assert!(selected.normalized_radius <= previous);
            previous = selected.normalized_radius;
        }
    }

    #[test]
    fn calibration_and_evaluation_groups_predictor_and_shape_definition_are_bound() {
        let family = family();
        assert_eq!(
            family.build_episode_tube(
                1,
                20_001,
                &[[0.0, 0.0]],
                &[[[4.0, 0.0], [0.0, 1.0]]],
                0.2,
                [0xB2; SHA256_BYTES],
                [0xC3; SHA256_BYTES],
                501,
                [0xD4; SHA256_BYTES],
            ),
            Err(AnisotropicEpisodeSetError::EvaluationEpisodeReused(1))
        );
        assert_eq!(
            family.build_episode_tube(
                500,
                10_001,
                &[[0.0, 0.0]],
                &[[[4.0, 0.0], [0.0, 1.0]]],
                0.2,
                [0xB2; SHA256_BYTES],
                [0xC3; SHA256_BYTES],
                501,
                [0xD4; SHA256_BYTES],
            ),
            Err(AnisotropicEpisodeSetError::EvaluationIndependenceGroupReused(10_001))
        );
        assert_eq!(
            family.build_episode_tube(
                500,
                20_500,
                &[[0.0, 0.0]],
                &[[[4.0, 0.0], [0.0, 1.0]]],
                0.2,
                [0xE5; SHA256_BYTES],
                [0xC3; SHA256_BYTES],
                501,
                [0xD4; SHA256_BYTES],
            ),
            Err(AnisotropicEpisodeSetError::PredictorCommitmentMismatch)
        );
        assert_eq!(
            family.build_episode_tube(
                500,
                20_500,
                &[[0.0, 0.0]],
                &[[[4.0, 0.0], [0.0, 1.0]]],
                0.2,
                [0xB2; SHA256_BYTES],
                [0xE5; SHA256_BYTES],
                501,
                [0xD4; SHA256_BYTES],
            ),
            Err(AnisotropicEpisodeSetError::ShapeDefinitionMismatch)
        );
    }

    #[test]
    fn invalid_singular_indefinite_asymmetric_and_ill_conditioned_shapes_reject() {
        let family = family();
        for shape in [
            [[1.0, 2.0], [2.0, 1.0]],      // indefinite
            [[1.0, 1.0], [1.0, 1.0]],      // singular
            [[1.0, 0.1], [0.0, 1.0]],      // asymmetric
            [[1.0, 0.0], [0.0, 1e-12]],    // below eigenvalue floor
            [[f64::NAN, 0.0], [0.0, 1.0]], // non-finite
        ] {
            assert!(family.build_episode_tube(
                500,
                20_500,
                &[[0.0, 0.0]],
                &[shape],
                0.2,
                [0xB2; SHA256_BYTES],
                [0xC3; SHA256_BYTES],
                501,
                [0xD4; SHA256_BYTES],
            ).is_err());
        }
    }

    #[test]
    fn calibration_rejects_truncated_truth_and_duplicate_dependency_groups() {
        let mut episodes = calibration();
        episodes[0].truth_positions_m.clear();
        assert_eq!(
            AnisotropicEpisodeConformalFamily2d::fit(&episodes, config()),
            Err(AnisotropicEpisodeSetError::CalibrationTrajectoryLengthMismatch(1))
        );
        episodes = calibration();
        episodes[1].independence_group_id = episodes[0].independence_group_id;
        assert_eq!(
            AnisotropicEpisodeConformalFamily2d::fit(&episodes, config()),
            Err(AnisotropicEpisodeSetError::DuplicateCalibrationIndependenceGroup(
                episodes[0].independence_group_id
            ))
        );
    }

    #[test]
    fn whole_episode_scoring_rejects_partial_and_nonfinite_truth() {
        let family = family();
        let outcome = family
            .build_episode_tube(
                500,
                20_500,
                &[[0.0, 0.0], [1.0, 1.0]],
                &[
                    [[4.0, 0.0], [0.0, 1.0]],
                    [[4.0, 0.0], [0.0, 1.0]],
                ],
                0.2,
                [0xB2; SHA256_BYTES],
                [0xC3; SHA256_BYTES],
                501,
                [0xD4; SHA256_BYTES],
            )
            .unwrap();
        let AnisotropicEpisodeSetOutcome2d::Ellipses(tube) = outcome else {
            panic!("expected ellipses");
        };
        assert_eq!(
            tube.contains_complete_trajectory(&[[0.0, 0.0]]),
            Err(AnisotropicEpisodeSetError::TruthPositionCountMismatch {
                expected: 2,
                observed: 1
            })
        );
        assert_eq!(
            tube.contains_complete_trajectory(&[[0.0, 0.0], [f64::NAN, 1.0]]),
            Err(AnisotropicEpisodeSetError::InvalidTruthPosition(1))
        );
    }

    #[test]
    fn requested_level_handles_abstention_and_empty_bounded_semantics() {
        let family = family();
        assert!(matches!(
            family.build_episode_tube(
                500,
                20_500,
                &[[0.0, 0.0]],
                &[[[4.0, 0.0], [0.0, 1.0]]],
                0.0,
                [0xB2; SHA256_BYTES],
                [0xC3; SHA256_BYTES],
                501,
                [0xD4; SHA256_BYTES],
            )
            .unwrap(),
            AnisotropicEpisodeSetOutcome2d::Abstain(_)
        ));
        assert!(matches!(
            family.build_episode_tube(
                500,
                20_500,
                &[[0.0, 0.0]],
                &[[[4.0, 0.0], [0.0, 1.0]]],
                1.2,
                [0xB2; SHA256_BYTES],
                [0xC3; SHA256_BYTES],
                501,
                [0xD4; SHA256_BYTES],
            )
            .unwrap(),
            AnisotropicEpisodeSetOutcome2d::EmptyBounded(_)
        ));
    }
}
