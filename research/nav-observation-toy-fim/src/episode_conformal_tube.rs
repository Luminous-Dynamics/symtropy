//! Episode-level conformal calibration for whole-trajectory position tubes.
//!
//! Each calibration unit contributes exactly one score: the maximum Euclidean
//! position error over that complete trajectory episode. This avoids treating
//! temporally correlated packets or adjacent positions as independent trials.
//!
//! Under exchangeability of the independent episode-level scores, the
//! split-conformal order statistic gives marginal coverage for a future
//! complete trajectory. It is not conditional coverage under arbitrary domain
//! shift, and it is not a rare-event integrity-risk certificate.
//!
//! Calibration truth, episode independence, cohort selection and source hashes
//! are supplied by the caller. This module checks structure and commitments;
//! it does not authenticate data or prove exchangeability.

use std::collections::BTreeSet;
use std::fmt;

const SHA256_BYTES: usize = 32;
const DIM: usize = 2;

/// One independent calibration episode summarized by its worst position error.
///
/// maximum_position_error_m must be the maximum Euclidean position error
/// over every evaluated position in the episode. independence_group_id
/// identifies the actual independent experimental unit; repeated trajectories
/// from one run/site/session must share a group ID and are rejected.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EpisodeMaximumPositionError2d {
    pub episode_id: u64,
    pub independence_group_id: u64,
    pub evaluated_position_count: usize,
    pub maximum_position_error_m: f64,
    pub source_manifest_sha256: [u8; SHA256_BYTES],
}

/// Frozen calibration policy and exact cohort/score-definition commitments.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EpisodeConformalConfig {
    /// Desired marginal coverage of a complete future trajectory episode.
    pub target_coverage: f64,
    /// Governance floor in addition to the finite-sample rank requirement.
    pub minimum_independent_episodes: usize,
    /// Caller-computed digest of the exact calibration cohort manifest.
    pub calibration_cohort_sha256: [u8; SHA256_BYTES],
    /// Caller-computed digest of the maximum-error score definition.
    pub score_definition_sha256: [u8; SHA256_BYTES],
}

/// Result of finite-sample episode-level split-conformal calibration.
///
/// Fields are private so safe callers cannot construct a report that bypasses
/// the calibration checks. The report retains calibration episode and
/// independence-group identities to prevent reusing a calibration episode as
/// the evaluated trajectory.
#[derive(Clone, Debug, PartialEq)]
pub struct EpisodeConformalCalibration2d {
    target_coverage: f64,
    calibration_radius_m: f64,
    calibration_episode_count: usize,
    total_evaluated_position_count: usize,
    selected_order_statistic_rank: usize,
    calibration_cohort_sha256: [u8; SHA256_BYTES],
    score_definition_sha256: [u8; SHA256_BYTES],
    calibration_episode_ids: Vec<u64>,
    calibration_independence_group_ids: Vec<u64>,
}

impl EpisodeConformalCalibration2d {
    pub fn target_coverage(&self) -> f64 {
        self.target_coverage
    }

    pub fn calibration_radius_m(&self) -> f64 {
        self.calibration_radius_m
    }

    pub fn calibration_episode_count(&self) -> usize {
        self.calibration_episode_count
    }

    pub fn total_evaluated_position_count(&self) -> usize {
        self.total_evaluated_position_count
    }

    /// One-based order-statistic rank selected by ceil((n + 1) * coverage).
    pub fn selected_order_statistic_rank(&self) -> usize {
        self.selected_order_statistic_rank
    }

    pub fn calibration_cohort_sha256(&self) -> [u8; SHA256_BYTES] {
        self.calibration_cohort_sha256
    }

    pub fn score_definition_sha256(&self) -> [u8; SHA256_BYTES] {
        self.score_definition_sha256
    }

    pub fn calibration_episode_ids(&self) -> &[u64] {
        &self.calibration_episode_ids
    }

    pub fn calibration_independence_group_ids(&self) -> &[u64] {
        &self.calibration_independence_group_ids
    }
}

/// A constant-radius disk around every predicted trajectory position.
///
/// The joint event is that the complete true trajectory lies inside all disks,
/// not that an arbitrary individual packet is covered independently.
#[derive(Clone, Debug, PartialEq)]
pub struct EpisodeConformalTube2d {
    pub episode_id: u64,
    pub independence_group_id: u64,
    pub centers_m: Vec<[f64; DIM]>,
    pub radius_m: f64,
    pub target_coverage: f64,
    pub calibration_episode_count: usize,
    pub selected_order_statistic_rank: usize,
    pub calibration_cohort_sha256: [u8; SHA256_BYTES],
    pub score_definition_sha256: [u8; SHA256_BYTES],
    pub solution_sha256: [u8; SHA256_BYTES],
}

impl EpisodeConformalTube2d {
    /// Return whether a finite truth position lies in the disk at time_index.
    /// Invalid indices or non-finite truth values return false.
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EpisodeConformalError {
    EmptyCalibrationCohort,
    InvalidConfiguration,
    MissingCalibrationCohortCommitment,
    MissingScoreDefinitionCommitment,
    InvalidEpisodeIdentity,
    DuplicateEpisodeIdentity(u64),
    DuplicateIndependenceGroup(u64),
    EmptyEpisode(u64),
    InvalidEpisodeMaximumError(u64),
    MissingEpisodeSourceCommitment(u64),
    InsufficientGovernanceEpisodeCount {
        minimum: usize,
        available: usize,
    },
    InsufficientFiniteSampleRank {
        needed_rank: usize,
        available: usize,
    },
    InvalidEvaluationIdentity,
    EvaluationEpisodeReused(u64),
    EvaluationIndependenceGroupReused(u64),
    EmptyEvaluationTrajectory,
    InvalidEvaluationPosition(usize),
    MissingSolutionCommitment,
    InvalidCalibrationReport,
    ArithmeticOverflow,
}

impl fmt::Display for EpisodeConformalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCalibrationCohort => write!(f, "calibration cohort must contain at least one episode"),
            Self::InvalidConfiguration => write!(f, "coverage must be finite and in (0, 1), and the minimum episode count must be positive"),
            Self::MissingCalibrationCohortCommitment => write!(f, "calibration cohort SHA-256 commitment is missing"),
            Self::MissingScoreDefinitionCommitment => write!(f, "score-definition SHA-256 commitment is missing"),
            Self::InvalidEpisodeIdentity => write!(f, "episode and independence-group identities must be nonzero"),
            Self::DuplicateEpisodeIdentity(id) => write!(f, "duplicate calibration episode identity: {id}"),
            Self::DuplicateIndependenceGroup(id) => write!(f, "calibration episodes are not independent: repeated group {id}"),
            Self::EmptyEpisode(id) => write!(f, "calibration episode {id} has no evaluated positions"),
            Self::InvalidEpisodeMaximumError(id) => write!(f, "calibration episode {id} has an invalid maximum position error"),
            Self::MissingEpisodeSourceCommitment(id) => write!(f, "calibration episode {id} is missing its source-manifest commitment"),
            Self::InsufficientGovernanceEpisodeCount { minimum, available } => write!(f, "calibration cohort has {available} independent episodes; policy requires at least {minimum}"),
            Self::InsufficientFiniteSampleRank { needed_rank, available } => write!(f, "target coverage requires order-statistic rank {needed_rank}, but only {available} calibration episodes are available; no finite conformal radius is emitted"),
            Self::InvalidEvaluationIdentity => write!(f, "evaluation episode and independence-group identities must be nonzero"),
            Self::EvaluationEpisodeReused(id) => write!(f, "evaluation episode {id} is present in the calibration cohort"),
            Self::EvaluationIndependenceGroupReused(id) => write!(f, "evaluation independence group {id} overlaps the calibration cohort"),
            Self::EmptyEvaluationTrajectory => write!(f, "evaluation trajectory must contain at least one predicted position"),
            Self::InvalidEvaluationPosition(index) => write!(f, "evaluation trajectory position {index} is non-finite"),
            Self::MissingSolutionCommitment => write!(f, "evaluation solution SHA-256 commitment is missing"),
            Self::InvalidCalibrationReport => write!(f, "calibration report is internally invalid"),
            Self::ArithmeticOverflow => write!(f, "calibration arithmetic overflowed"),
        }
    }
}

impl std::error::Error for EpisodeConformalError {}

/// Calibrate a radius for the maximum position error over an entire episode.
///
/// For n independent calibration episodes and target coverage p, select
/// the one-based order statistic k = ceil((n + 1) * p). If k > n, the
/// sample is too small to emit a finite radius and the function fails closed.
/// Packet counts affect descriptive metadata only; they never increase n.
pub fn calibrate_episode_conformal_radius_2d(
    episodes: &[EpisodeMaximumPositionError2d],
    config: EpisodeConformalConfig,
) -> Result<EpisodeConformalCalibration2d, EpisodeConformalError> {
    if !config.target_coverage.is_finite()
        || config.target_coverage <= 0.0
        || config.target_coverage >= 1.0
        || config.minimum_independent_episodes == 0
    {
        return Err(EpisodeConformalError::InvalidConfiguration);
    }
    if config.calibration_cohort_sha256.iter().all(|byte| *byte == 0) {
        return Err(EpisodeConformalError::MissingCalibrationCohortCommitment);
    }
    if config.score_definition_sha256.iter().all(|byte| *byte == 0) {
        return Err(EpisodeConformalError::MissingScoreDefinitionCommitment);
    }
    if episodes.is_empty() {
        return Err(EpisodeConformalError::EmptyCalibrationCohort);
    }
    if episodes.len() < config.minimum_independent_episodes {
        return Err(EpisodeConformalError::InsufficientGovernanceEpisodeCount {
            minimum: config.minimum_independent_episodes,
            available: episodes.len(),
        });
    }

    let mut episode_ids = BTreeSet::new();
    let mut independence_group_ids = BTreeSet::new();
    let mut total_evaluated_position_count = 0_usize;
    let mut scores = Vec::with_capacity(episodes.len());

    for episode in episodes {
        if episode.episode_id == 0 || episode.independence_group_id == 0 {
            return Err(EpisodeConformalError::InvalidEpisodeIdentity);
        }
        if !episode_ids.insert(episode.episode_id) {
            return Err(EpisodeConformalError::DuplicateEpisodeIdentity(
                episode.episode_id,
            ));
        }
        if !independence_group_ids.insert(episode.independence_group_id) {
            return Err(EpisodeConformalError::DuplicateIndependenceGroup(
                episode.independence_group_id,
            ));
        }
        if episode.evaluated_position_count == 0 {
            return Err(EpisodeConformalError::EmptyEpisode(episode.episode_id));
        }
        if !episode.maximum_position_error_m.is_finite()
            || episode.maximum_position_error_m < 0.0
        {
            return Err(EpisodeConformalError::InvalidEpisodeMaximumError(
                episode.episode_id,
            ));
        }
        if episode.source_manifest_sha256.iter().all(|byte| *byte == 0) {
            return Err(EpisodeConformalError::MissingEpisodeSourceCommitment(
                episode.episode_id,
            ));
        }
        total_evaluated_position_count = total_evaluated_position_count
            .checked_add(episode.evaluated_position_count)
            .ok_or(EpisodeConformalError::ArithmeticOverflow)?;
        scores.push((
            episode.maximum_position_error_m,
            episode.episode_id,
            episode.independence_group_id,
        ));
    }

    let n_plus_one = episodes
        .len()
        .checked_add(1)
        .ok_or(EpisodeConformalError::ArithmeticOverflow)?;
    let rank_as_float = (n_plus_one as f64 * config.target_coverage).ceil();
    if !rank_as_float.is_finite()
        || rank_as_float < 1.0
        || rank_as_float > usize::MAX as f64
    {
        return Err(EpisodeConformalError::ArithmeticOverflow);
    }
    let selected_order_statistic_rank = rank_as_float as usize;
    if selected_order_statistic_rank > episodes.len() {
        return Err(EpisodeConformalError::InsufficientFiniteSampleRank {
            needed_rank: selected_order_statistic_rank,
            available: episodes.len(),
        });
    }

    scores.sort_by(|left, right| {
        left.0
            .total_cmp(&right.0)
            .then_with(|| left.1.cmp(&right.1))
    });
    let calibration_radius_m = scores[selected_order_statistic_rank - 1].0;
    let mut calibration_episode_ids = episode_ids.into_iter().collect::<Vec<_>>();
    let mut calibration_independence_group_ids =
        independence_group_ids.into_iter().collect::<Vec<_>>();
    calibration_episode_ids.sort_unstable();
    calibration_independence_group_ids.sort_unstable();

    Ok(EpisodeConformalCalibration2d {
        target_coverage: config.target_coverage,
        calibration_radius_m,
        calibration_episode_count: episodes.len(),
        total_evaluated_position_count,
        selected_order_statistic_rank,
        calibration_cohort_sha256: config.calibration_cohort_sha256,
        score_definition_sha256: config.score_definition_sha256,
        calibration_episode_ids,
        calibration_independence_group_ids,
    })
}

/// Apply one episode-level conformal radius to every predicted position.
///
/// Calibration and evaluation episode IDs and independence-group IDs must be
/// disjoint. This prevents a caller from evaluating coverage on an episode
/// that contributed to its own calibration radius.
pub fn build_episode_conformal_tube_2d(
    episode_id: u64,
    independence_group_id: u64,
    predicted_positions_m: &[[f64; DIM]],
    solution_sha256: [u8; SHA256_BYTES],
    calibration: &EpisodeConformalCalibration2d,
) -> Result<EpisodeConformalTube2d, EpisodeConformalError> {
    if episode_id == 0 || independence_group_id == 0 {
        return Err(EpisodeConformalError::InvalidEvaluationIdentity);
    }
    if calibration.calibration_episode_ids.contains(&episode_id) {
        return Err(EpisodeConformalError::EvaluationEpisodeReused(episode_id));
    }
    if calibration
        .calibration_independence_group_ids
        .contains(&independence_group_id)
    {
        return Err(EpisodeConformalError::EvaluationIndependenceGroupReused(
            independence_group_id,
        ));
    }
    if predicted_positions_m.is_empty() {
        return Err(EpisodeConformalError::EmptyEvaluationTrajectory);
    }
    if predicted_positions_m
        .iter()
        .any(|position| !position.iter().all(|value| value.is_finite()))
    {
        let index = predicted_positions_m
            .iter()
            .position(|position| !position.iter().all(|value| value.is_finite()))
            .unwrap_or(0);
        return Err(EpisodeConformalError::InvalidEvaluationPosition(index));
    }
    if solution_sha256.iter().all(|byte| *byte == 0) {
        return Err(EpisodeConformalError::MissingSolutionCommitment);
    }
    if !calibration.target_coverage.is_finite()
        || calibration.target_coverage <= 0.0
        || calibration.target_coverage >= 1.0
        || !calibration.calibration_radius_m.is_finite()
        || calibration.calibration_radius_m < 0.0
        || calibration.calibration_episode_count == 0
        || calibration.selected_order_statistic_rank == 0
        || calibration.selected_order_statistic_rank > calibration.calibration_episode_count
        || calibration.calibration_episode_ids.len() != calibration.calibration_episode_count
        || calibration.calibration_independence_group_ids.len()
            != calibration.calibration_episode_count
        || calibration
            .calibration_cohort_sha256
            .iter()
            .all(|byte| *byte == 0)
        || calibration
            .score_definition_sha256
            .iter()
            .all(|byte| *byte == 0)
    {
        return Err(EpisodeConformalError::InvalidCalibrationReport);
    }

    Ok(EpisodeConformalTube2d {
        episode_id,
        independence_group_id,
        centers_m: predicted_positions_m.to_vec(),
        radius_m: calibration.calibration_radius_m,
        target_coverage: calibration.target_coverage,
        calibration_episode_count: calibration.calibration_episode_count,
        selected_order_statistic_rank: calibration.selected_order_statistic_rank,
        calibration_cohort_sha256: calibration.calibration_cohort_sha256,
        score_definition_sha256: calibration.score_definition_sha256,
        solution_sha256,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(target_coverage: f64, minimum_independent_episodes: usize) -> EpisodeConformalConfig {
        EpisodeConformalConfig {
            target_coverage,
            minimum_independent_episodes,
            calibration_cohort_sha256: [0xA1; SHA256_BYTES],
            score_definition_sha256: [0xB2; SHA256_BYTES],
        }
    }

    fn episode(id: u64, error_m: f64, position_count: usize) -> EpisodeMaximumPositionError2d {
        EpisodeMaximumPositionError2d {
            episode_id: id,
            independence_group_id: id + 10_000,
            evaluated_position_count: position_count,
            maximum_position_error_m: error_m,
            source_manifest_sha256: [((id % 255) + 1) as u8; SHA256_BYTES],
        }
    }

    fn episodes(count: usize) -> Vec<EpisodeMaximumPositionError2d> {
        (1..=count as u64)
            .map(|id| episode(id, id as f64 / 100.0, id as usize * 100))
            .collect()
    }

    #[test]
    fn calibrates_complete_episode_maxima_with_finite_sample_order_statistic() {
        let data = episodes(19);
        let report = calibrate_episode_conformal_radius_2d(&data, config(0.95, 19)).unwrap();
        assert_eq!(report.calibration_episode_count(), 19);
        assert_eq!(report.selected_order_statistic_rank(), 19);
        assert!((report.calibration_radius_m() - 0.19).abs() < 1e-12);
        assert_eq!(report.total_evaluated_position_count(), 19_000);
    }

    #[test]
    fn packet_counts_do_not_inflate_the_independent_episode_sample_size() {
        let data = episodes(18)
            .into_iter()
            .chain(std::iter::once(episode(19, 0.19, 10_000_000)))
            .collect::<Vec<_>>();
        assert!(matches!(
            calibrate_episode_conformal_radius_2d(&data, config(0.99, 19)),
            Err(EpisodeConformalError::InsufficientFiniteSampleRank {
                needed_rank: 20,
                available: 19
            })
        ));
    }

    #[test]
    fn ninety_nine_percent_target_needs_ninety_nine_independent_episodes() {
        let insufficient = episodes(98);
        assert_eq!(
            calibrate_episode_conformal_radius_2d(&insufficient, config(0.99, 98)),
            Err(EpisodeConformalError::InsufficientFiniteSampleRank {
                needed_rank: 99,
                available: 98
            })
        );
        let sufficient = episodes(99);
        let report =
            calibrate_episode_conformal_radius_2d(&sufficient, config(0.99, 99)).unwrap();
        assert_eq!(report.selected_order_statistic_rank(), 99);
    }

    #[test]
    fn ninety_nine_point_nine_percent_rank_boundary_is_exact() {
        let insufficient = episodes(998);
        assert_eq!(
            calibrate_episode_conformal_radius_2d(&insufficient, config(0.999, 998)),
            Err(EpisodeConformalError::InsufficientFiniteSampleRank {
                needed_rank: 999,
                available: 998
            })
        );
        let sufficient = episodes(999);
        let report =
            calibrate_episode_conformal_radius_2d(&sufficient, config(0.999, 999)).unwrap();
        assert_eq!(report.selected_order_statistic_rank(), 999);
    }

    #[test]
    fn calibration_is_permutation_invariant() {
        let data = episodes(19);
        let first = calibrate_episode_conformal_radius_2d(&data, config(0.95, 19)).unwrap();
        let mut reversed = data.clone();
        reversed.reverse();
        let second =
            calibrate_episode_conformal_radius_2d(&reversed, config(0.95, 19)).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn increasing_one_episode_maximum_cannot_shrink_the_calibrated_radius() {
        let mut data = episodes(19);
        let original =
            calibrate_episode_conformal_radius_2d(&data, config(0.95, 19)).unwrap();
        data[0].maximum_position_error_m = 1.0;
        let widened =
            calibrate_episode_conformal_radius_2d(&data, config(0.95, 19)).unwrap();
        assert!(widened.calibration_radius_m() >= original.calibration_radius_m());
        assert!((widened.calibration_radius_m() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_duplicate_episode_and_nonindependent_group_identities() {
        let mut data = episodes(3);
        data[1].episode_id = data[0].episode_id;
        assert_eq!(
            calibrate_episode_conformal_radius_2d(&data, config(0.5, 2)),
            Err(EpisodeConformalError::DuplicateEpisodeIdentity(1))
        );
        data = episodes(3);
        data[1].independence_group_id = data[0].independence_group_id;
        assert_eq!(
            calibrate_episode_conformal_radius_2d(&data, config(0.5, 2)),
            Err(EpisodeConformalError::DuplicateIndependenceGroup(10_001))
        );
    }

    #[test]
    fn whole_trajectory_tube_applies_one_joint_radius_to_every_position() {
        let calibration =
            calibrate_episode_conformal_radius_2d(&episodes(19), config(0.95, 19)).unwrap();
        let tube = build_episode_conformal_tube_2d(
            500,
            20_500,
            &[[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]],
            [0xC3; SHA256_BYTES],
            &calibration,
        )
        .unwrap();
        assert_eq!(tube.radius_m, 0.19);
        assert!(tube.contains_position(0, [0.1, 0.0]));
        assert!(tube.contains_position(1, [1.0, 1.1]));
        assert!(tube.contains_position(2, [2.0, 2.19]));
        assert!(!tube.contains_position(2, [2.0, 2.191]));
        assert!(!tube.contains_position(3, [2.0, 2.0]));
    }

    #[test]
    fn calibration_episode_or_independence_group_cannot_be_reused_for_evaluation() {
        let calibration =
            calibrate_episode_conformal_radius_2d(&episodes(19), config(0.95, 19)).unwrap();
        assert_eq!(
            build_episode_conformal_tube_2d(
                1,
                20_001,
                &[[0.0, 0.0]],
                [0xC3; SHA256_BYTES],
                &calibration
            ),
            Err(EpisodeConformalError::EvaluationEpisodeReused(1))
        );
        assert_eq!(
            build_episode_conformal_tube_2d(
                500,
                10_001,
                &[[0.0, 0.0]],
                [0xC3; SHA256_BYTES],
                &calibration
            ),
            Err(EpisodeConformalError::EvaluationIndependenceGroupReused(10_001))
        );
    }

    #[test]
    fn invalid_scores_commits_and_prediction_positions_fail_closed() {
        let mut data = episodes(19);
        data[0].maximum_position_error_m = f64::NAN;
        assert_eq!(
            calibrate_episode_conformal_radius_2d(&data, config(0.95, 19)),
            Err(EpisodeConformalError::InvalidEpisodeMaximumError(1))
        );
        data = episodes(19);
        data[0].source_manifest_sha256 = [0; SHA256_BYTES];
        assert_eq!(
            calibrate_episode_conformal_radius_2d(&data, config(0.95, 19)),
            Err(EpisodeConformalError::MissingEpisodeSourceCommitment(1))
        );
        let calibration =
            calibrate_episode_conformal_radius_2d(&episodes(19), config(0.95, 19)).unwrap();
        assert_eq!(
            build_episode_conformal_tube_2d(
                500,
                20_500,
                &[[f64::INFINITY, 0.0]],
                [0xC3; SHA256_BYTES],
                &calibration
            ),
            Err(EpisodeConformalError::InvalidEvaluationPosition(0))
        );
    }
}
