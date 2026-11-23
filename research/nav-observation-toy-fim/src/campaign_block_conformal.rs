//! Conservative campaign/block-level anisotropic conformal calibration.
//!
//! Each calibration campaign contributes exactly one score: the maximum
//! normalized position error over every member trajectory and every position
//! in that campaign block. This prevents file chunking or multiple trajectories
//! from one collection campaign from artificially increasing the conformal
//! calibration sample count.
//!
//! The finite-sample order statistic is the existing anisotropic episode
//! conformal family, applied to one flattened synthetic trajectory per block.
//! Flattening is only a max-score reduction; member boundaries are retained for
//! evaluation and complete-block containment.
//!
//! The guarantee is marginal over exchangeable campaign blocks, not arbitrary
//! dependent blocks. A block ID is not proof of independence: the caller must
//! supply a defensible source-backed grouping contract. This module does not
//! estimate beta-mixing or functional-dependence coefficients.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::anisotropic_episode_tube::{
    AnisotropicCalibrationEpisode2d, AnisotropicEmptyEpisodeSet2d,
    AnisotropicEpisodeAbstention, AnisotropicEpisodeConformalFamily2d,
    AnisotropicEpisodeFamilyConfig, AnisotropicEpisodeSetError,
    AnisotropicEpisodeSetOutcome2d, AnisotropicEpisodeTube2d,
};

const SHA256_BYTES: usize = 32;
const DIM: usize = 2;

/// One physical collection block, e.g. a source-justified site-day, hardware
/// boot cohort or contiguous field campaign. All member trajectories receive
/// one joint maximum score and count as one calibration unit.
#[derive(Clone, Debug, PartialEq)]
pub struct CampaignCalibrationBlock2d {
    pub block_id: u64,
    /// Unique identity of the independent block unit used for overlap checks.
    pub independence_group_id: u64,
    /// Must commit to the complete block manifest and member source lineage.
    pub source_block_manifest_sha256: [u8; SHA256_BYTES],
    pub member_episodes: Vec<AnisotropicCalibrationEpisode2d>,
}

/// One predicted trajectory within a future evaluation block.
#[derive(Clone, Debug, PartialEq)]
pub struct CampaignPredictedEpisode2d {
    pub episode_id: u64,
    pub predicted_positions_m: Vec<[f64; DIM]>,
    pub shape_matrices: Vec<[[f64; DIM]; DIM]>,
}

/// A complete future evaluation block. All member episodes are calibrated
/// simultaneously as one block-level event.
#[derive(Clone, Debug, PartialEq)]
pub struct CampaignBlockPrediction2d {
    pub block_id: u64,
    pub independence_group_id: u64,
    pub member_episodes: Vec<CampaignPredictedEpisode2d>,
    pub requested_miscoverage: f64,
    pub predictor_sha256: [u8; SHA256_BYTES],
    pub shape_definition_sha256: [u8; SHA256_BYTES],
    pub decision_sequence_id: u64,
    pub decision_receipt_sha256: [u8; SHA256_BYTES],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CampaignMemberSpan {
    pub episode_id: u64,
    pub start_index: usize,
    pub end_index_exclusive: usize,
}

impl CampaignMemberSpan {
    pub fn position_count(&self) -> usize {
        self.end_index_exclusive - self.start_index
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CampaignBlockCalibrationError {
    EmptyCalibrationBlocks,
    InvalidBlockIdentity,
    DuplicateBlockId(u64),
    DuplicateBlockIndependenceGroup(u64),
    MissingBlockManifestCommitment(u64),
    EmptyBlock(u64),
    InvalidMemberEpisodeIdentity(u64),
    DuplicateMemberEpisodeId(u64),
    MissingMemberSourceCommitment(u64),
    EmptyMemberTrajectory(u64),
    MemberTrajectoryLengthMismatch(u64),
    DependencyGroupCrossesBlocks { group_id: u64, first_block: u64, second_block: u64 },
    InvalidEvaluationBlockIdentity,
    EvaluationBlockReused(u64),
    EvaluationGroupReused(u64),
    EvaluationMemberCountMismatch { expected: usize, observed: usize },
    EvaluationEpisodeReused(u64),
    DuplicateEvaluationEpisodeId(u64),
    EmptyEvaluationMember(u64),
    EvaluationTrajectoryLengthMismatch(u64),
    InvalidEvaluationPosition { episode_id: u64, time_index: usize },
    ArithmeticOverflow,
    BaseFamily(AnisotropicEpisodeSetError),
}

impl fmt::Display for CampaignBlockCalibrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCalibrationBlocks => write!(f, "at least one calibration campaign block is required"),
            Self::InvalidBlockIdentity => write!(f, "block and independence-group IDs must be nonzero"),
            Self::DuplicateBlockId(id) => write!(f, "duplicate campaign block ID: {id}"),
            Self::DuplicateBlockIndependenceGroup(id) => write!(f, "duplicate campaign independence-group ID: {id}"),
            Self::MissingBlockManifestCommitment(id) => write!(f, "campaign block {id} is missing its source-manifest commitment"),
            Self::EmptyBlock(id) => write!(f, "campaign block {id} has no member episodes"),
            Self::InvalidMemberEpisodeIdentity(id) => write!(f, "member episode {id} has a zero identity or dependency group"),
            Self::DuplicateMemberEpisodeId(id) => write!(f, "member episode ID is duplicated: {id}"),
            Self::MissingMemberSourceCommitment(id) => write!(f, "member episode {id} is missing its source commitment"),
            Self::EmptyMemberTrajectory(id) => write!(f, "member episode {id} has no positions"),
            Self::MemberTrajectoryLengthMismatch(id) => write!(f, "member episode {id} has mismatched prediction, truth and shape lengths"),
            Self::DependencyGroupCrossesBlocks { group_id, first_block, second_block } => write!(
                f, "dependency group {group_id} crosses campaign blocks {first_block} and {second_block}"
            ),
            Self::InvalidEvaluationBlockIdentity => write!(f, "evaluation block and independence-group IDs must be nonzero"),
            Self::EvaluationBlockReused(id) => write!(f, "evaluation block ID was already consumed: {id}"),
            Self::EvaluationGroupReused(id) => write!(f, "evaluation dependency group overlaps calibration or prior evaluation: {id}"),
            Self::EvaluationMemberCountMismatch { expected, observed } => write!(f, "evaluation block has {observed} members but caller declared {expected}"),
            Self::EvaluationEpisodeReused(id) => write!(f, "evaluation member episode overlaps calibration or prior evaluation: {id}"),
            Self::DuplicateEvaluationEpisodeId(id) => write!(f, "evaluation member episode ID is duplicated: {id}"),
            Self::EmptyEvaluationMember(id) => write!(f, "evaluation member episode {id} is empty"),
            Self::EvaluationTrajectoryLengthMismatch(id) => write!(f, "evaluation member episode {id} has mismatched prediction and shape lengths"),
            Self::InvalidEvaluationPosition { episode_id, time_index } => write!(f, "evaluation member episode {episode_id} has invalid position at index {time_index}"),
            Self::ArithmeticOverflow => write!(f, "campaign block flattening overflowed"),
            Self::BaseFamily(error) => write!(f, "base anisotropic conformal family rejected input: {error}"),
        }
    }
}

impl std::error::Error for CampaignBlockCalibrationError {}

/// One block-level conformal family. Calibration sample count equals the
/// number of source-justified campaign blocks, never the number of member files
/// or trajectories.
#[derive(Clone, Debug, PartialEq)]
pub struct CampaignBlockAnisotropicConformalFamily2d {
    base_family: AnisotropicEpisodeConformalFamily2d,
    calibration_block_ids: BTreeSet<u64>,
    calibration_block_group_ids: BTreeSet<u64>,
    calibration_member_episode_ids: BTreeSet<u64>,
    calibration_source_dependency_groups: BTreeMap<u64, u64>,
    seen_evaluation_block_ids: BTreeSet<u64>,
    seen_evaluation_group_ids: BTreeSet<u64>,
    seen_evaluation_member_episode_ids: BTreeSet<u64>,
    calibration_member_counts: BTreeMap<u64, usize>,
}

impl CampaignBlockAnisotropicConformalFamily2d {
    pub fn fit(
        blocks: &[CampaignCalibrationBlock2d],
        config: AnisotropicEpisodeFamilyConfig,
    ) -> Result<Self, CampaignBlockCalibrationError> {
        if blocks.is_empty() {
            return Err(CampaignBlockCalibrationError::EmptyCalibrationBlocks);
        }

        let mut block_ids = BTreeSet::new();
        let mut block_group_ids = BTreeSet::new();
        let mut member_episode_ids = BTreeSet::new();
        let mut source_dependency_groups = BTreeMap::new();
        let mut member_counts = BTreeMap::new();
        let mut flattened_blocks = Vec::with_capacity(blocks.len());

        for block in blocks {
            if block.block_id == 0 || block.independence_group_id == 0 {
                return Err(CampaignBlockCalibrationError::InvalidBlockIdentity);
            }
            if !block_ids.insert(block.block_id) {
                return Err(CampaignBlockCalibrationError::DuplicateBlockId(block.block_id));
            }
            if !block_group_ids.insert(block.independence_group_id) {
                return Err(CampaignBlockCalibrationError::DuplicateBlockIndependenceGroup(
                    block.independence_group_id,
                ));
            }
            if block.source_block_manifest_sha256.iter().all(|byte| *byte == 0) {
                return Err(CampaignBlockCalibrationError::MissingBlockManifestCommitment(
                    block.block_id,
                ));
            }
            if block.member_episodes.is_empty() {
                return Err(CampaignBlockCalibrationError::EmptyBlock(block.block_id));
            }

            let mut predicted = Vec::new();
            let mut truth = Vec::new();
            let mut shapes = Vec::new();
            let mut block_member_ids = BTreeSet::new();

            for episode in &block.member_episodes {
                if episode.episode_id == 0 || episode.independence_group_id == 0 {
                    return Err(CampaignBlockCalibrationError::InvalidMemberEpisodeIdentity(
                        episode.episode_id,
                    ));
                }
                if !member_episode_ids.insert(episode.episode_id)
                    || !block_member_ids.insert(episode.episode_id)
                {
                    return Err(CampaignBlockCalibrationError::DuplicateMemberEpisodeId(
                        episode.episode_id,
                    ));
                }
                if episode.source_manifest_sha256.iter().all(|byte| *byte == 0) {
                    return Err(CampaignBlockCalibrationError::MissingMemberSourceCommitment(
                        episode.episode_id,
                    ));
                }
                if episode.predicted_positions_m.is_empty() {
                    return Err(CampaignBlockCalibrationError::EmptyMemberTrajectory(
                        episode.episode_id,
                    ));
                }
                if episode.predicted_positions_m.len() != episode.truth_positions_m.len()
                    || episode.predicted_positions_m.len() != episode.shape_matrices.len()
                {
                    return Err(CampaignBlockCalibrationError::MemberTrajectoryLengthMismatch(
                        episode.episode_id,
                    ));
                }

                if let Some(existing_block) = source_dependency_groups.get(&episode.independence_group_id) {
                    if *existing_block != block.block_id {
                        return Err(CampaignBlockCalibrationError::DependencyGroupCrossesBlocks {
                            group_id: episode.independence_group_id,
                            first_block: *existing_block,
                            second_block: block.block_id,
                        });
                    }
                } else {
                    source_dependency_groups.insert(episode.independence_group_id, block.block_id);
                }

                predicted.extend_from_slice(&episode.predicted_positions_m);
                truth.extend_from_slice(&episode.truth_positions_m);
                shapes.extend_from_slice(&episode.shape_matrices);
            }

            member_counts.insert(block.block_id, block.member_episodes.len());
            flattened_blocks.push(AnisotropicCalibrationEpisode2d {
                // One synthetic calibration unit per physical campaign block.
                episode_id: block.block_id,
                independence_group_id: block.independence_group_id,
                predicted_positions_m: predicted,
                truth_positions_m: truth,
                shape_matrices: shapes,
                source_manifest_sha256: block.source_block_manifest_sha256,
            });
        }

        let base_family = AnisotropicEpisodeConformalFamily2d::fit(&flattened_blocks, config)
            .map_err(CampaignBlockCalibrationError::BaseFamily)?;

        Ok(Self {
            base_family,
            calibration_block_ids: block_ids,
            calibration_block_group_ids: block_group_ids,
            calibration_member_episode_ids: member_episode_ids,
            calibration_source_dependency_groups: source_dependency_groups,
            seen_evaluation_block_ids: BTreeSet::new(),
            seen_evaluation_group_ids: BTreeSet::new(),
            seen_evaluation_member_episode_ids: BTreeSet::new(),
            calibration_member_counts: member_counts,
        })
    }

    pub fn calibration_block_count(&self) -> usize {
        self.calibration_block_ids.len()
    }

    pub fn calibration_member_episode_count(&self) -> usize {
        self.calibration_member_episode_ids.len()
    }

    /// Number of member trajectories retained for a calibration block.
    pub fn calibration_member_count(&self, block_id: u64) -> Option<usize> {
        self.calibration_member_counts.get(&block_id).copied()
    }

    /// The base family sees one maximum score per block.
    pub fn calibration_unit_count(&self) -> usize {
        self.base_family.calibration_episode_count()
    }

    /// Build one joint tube for an entire unseen campaign block.
    ///
    /// Evaluation IDs are consumed only after all input checks and a valid
    /// finite tube/empty-set outcome have been constructed. Abstention does not
    /// consume the block identity, allowing a safe retry with the same inputs.
    pub fn build_block_tube(
        &mut self,
        prediction: &CampaignBlockPrediction2d,
    ) -> Result<CampaignBlockSetOutcome2d, CampaignBlockCalibrationError> {
        if prediction.block_id == 0 || prediction.independence_group_id == 0 {
            return Err(CampaignBlockCalibrationError::InvalidEvaluationBlockIdentity);
        }
        if self.calibration_block_ids.contains(&prediction.block_id)
            || self.seen_evaluation_block_ids.contains(&prediction.block_id)
        {
            return Err(CampaignBlockCalibrationError::EvaluationBlockReused(
                prediction.block_id,
            ));
        }
        if self.calibration_block_group_ids.contains(&prediction.independence_group_id)
            || self.calibration_source_dependency_groups.contains_key(
                &prediction.independence_group_id,
            )
            || self.seen_evaluation_group_ids.contains(&prediction.independence_group_id)
        {
            return Err(CampaignBlockCalibrationError::EvaluationGroupReused(
                prediction.independence_group_id,
            ));
        }
        if prediction.member_episodes.is_empty() {
            return Err(CampaignBlockCalibrationError::EmptyBlock(prediction.block_id));
        }

        let mut member_ids = BTreeSet::new();
        let mut predicted = Vec::new();
        let mut shapes = Vec::new();
        let mut spans = Vec::with_capacity(prediction.member_episodes.len());

        for member in &prediction.member_episodes {
            if member.episode_id == 0 {
                return Err(CampaignBlockCalibrationError::InvalidMemberEpisodeIdentity(
                    member.episode_id,
                ));
            }
            if self.calibration_member_episode_ids.contains(&member.episode_id)
                || self.seen_evaluation_member_episode_ids.contains(&member.episode_id)
            {
                return Err(CampaignBlockCalibrationError::EvaluationEpisodeReused(
                    member.episode_id,
                ));
            }
            if !member_ids.insert(member.episode_id) {
                return Err(CampaignBlockCalibrationError::DuplicateEvaluationEpisodeId(
                    member.episode_id,
                ));
            }
            if member.predicted_positions_m.is_empty() {
                return Err(CampaignBlockCalibrationError::EmptyEvaluationMember(
                    member.episode_id,
                ));
            }
            if member.predicted_positions_m.len() != member.shape_matrices.len() {
                return Err(CampaignBlockCalibrationError::EvaluationTrajectoryLengthMismatch(
                    member.episode_id,
                ));
            }
            if let Some(time_index) = member.predicted_positions_m.iter().position(|position| {
                !position.iter().all(|value| value.is_finite())
            }) {
                return Err(CampaignBlockCalibrationError::InvalidEvaluationPosition {
                    episode_id: member.episode_id,
                    time_index,
                });
            }

            let start = predicted.len();
            predicted.extend_from_slice(&member.predicted_positions_m);
            shapes.extend_from_slice(&member.shape_matrices);
            let end = predicted.len();
            if end < start {
                return Err(CampaignBlockCalibrationError::ArithmeticOverflow);
            }
            spans.push(CampaignMemberSpan {
                episode_id: member.episode_id,
                start_index: start,
                end_index_exclusive: end,
            });
        }

        let inner_outcome = self.base_family
            .build_episode_tube(
                prediction.block_id,
                prediction.independence_group_id,
                &predicted,
                &shapes,
                prediction.requested_miscoverage,
                prediction.predictor_sha256,
                prediction.shape_definition_sha256,
                prediction.decision_sequence_id,
                prediction.decision_receipt_sha256,
            )
            .map_err(CampaignBlockCalibrationError::BaseFamily)?;

        let outcome = match inner_outcome {
            AnisotropicEpisodeSetOutcome2d::Ellipses(tube) => {
                CampaignBlockSetOutcome2d::Tube(CampaignBlockTube2d {
                    inner_tube: tube,
                    member_spans: spans,
                })
            }
            AnisotropicEpisodeSetOutcome2d::EmptyBounded(empty) => {
                CampaignBlockSetOutcome2d::EmptyBounded(CampaignBlockEmptySet2d {
                    inner_empty: empty,
                    member_spans: spans,
                })
            }
            AnisotropicEpisodeSetOutcome2d::Abstain(reason) => {
                return Ok(CampaignBlockSetOutcome2d::Abstain(reason));
            }
        };

        self.seen_evaluation_block_ids.insert(prediction.block_id);
        self.seen_evaluation_group_ids.insert(prediction.independence_group_id);
        self.seen_evaluation_member_episode_ids.extend(member_ids);
        Ok(outcome)
    }

    pub fn base_family(&self) -> &AnisotropicEpisodeConformalFamily2d {
        &self.base_family
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CampaignBlockSetOutcome2d {
    Tube(CampaignBlockTube2d),
    EmptyBounded(CampaignBlockEmptySet2d),
    Abstain(AnisotropicEpisodeAbstention),
}

/// One set covering every position of every member trajectory in a future
/// campaign block. The member spans preserve original trajectory boundaries.
#[derive(Clone, Debug, PartialEq)]
pub struct CampaignBlockTube2d {
    inner_tube: AnisotropicEpisodeTube2d,
    member_spans: Vec<CampaignMemberSpan>,
}

impl CampaignBlockTube2d {
    pub fn inner_tube(&self) -> &AnisotropicEpisodeTube2d {
        &self.inner_tube
    }

    pub fn member_spans(&self) -> &[CampaignMemberSpan] {
        &self.member_spans
    }

    pub fn contains_complete_block(
        &self,
        truth_members: &[CampaignTruthEpisode2d],
    ) -> Result<bool, CampaignBlockCalibrationError> {
        let positions = flatten_truth_members(&self.member_spans, truth_members)?;
        self.inner_tube
            .contains_complete_trajectory(&positions)
            .map_err(CampaignBlockCalibrationError::BaseFamily)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CampaignBlockEmptySet2d {
    inner_empty: AnisotropicEmptyEpisodeSet2d,
    member_spans: Vec<CampaignMemberSpan>,
}

impl CampaignBlockEmptySet2d {
    pub fn member_spans(&self) -> &[CampaignMemberSpan] {
        &self.member_spans
    }

    pub fn contains_complete_block(
        &self,
        truth_members: &[CampaignTruthEpisode2d],
    ) -> Result<bool, CampaignBlockCalibrationError> {
        let positions = flatten_truth_members(&self.member_spans, truth_members)?;
        self.inner_empty
            .contains_complete_trajectory(&positions)
            .map_err(CampaignBlockCalibrationError::BaseFamily)
    }
}

/// Evaluator-only truth for one member episode. This type is deliberately not
/// accepted by the calibration or prediction APIs.
#[derive(Clone, Debug, PartialEq)]
pub struct CampaignTruthEpisode2d {
    pub episode_id: u64,
    pub truth_positions_m: Vec<[f64; DIM]>,
}

fn flatten_truth_members(
    spans: &[CampaignMemberSpan],
    truth_members: &[CampaignTruthEpisode2d],
) -> Result<Vec<[f64; DIM]>, CampaignBlockCalibrationError> {
    if spans.len() != truth_members.len() {
        return Err(CampaignBlockCalibrationError::EvaluationMemberCountMismatch {
            expected: spans.len(),
            observed: truth_members.len(),
        });
    }
    let total = spans.iter().try_fold(0_usize, |sum, span| {
        sum.checked_add(span.position_count())
            .ok_or(CampaignBlockCalibrationError::ArithmeticOverflow)
    })?;
    let mut flattened = Vec::with_capacity(total);

    for (span, truth) in spans.iter().zip(truth_members) {
        if span.episode_id != truth.episode_id {
            return Err(CampaignBlockCalibrationError::EvaluationEpisodeReused(
                truth.episode_id,
            ));
        }
        if truth.truth_positions_m.len() != span.position_count() {
            return Err(CampaignBlockCalibrationError::EvaluationTrajectoryLengthMismatch(
                truth.episode_id,
            ));
        }
        if let Some(time_index) = truth.truth_positions_m.iter().position(|position| {
            !position.iter().all(|value| value.is_finite())
        }) {
            return Err(CampaignBlockCalibrationError::InvalidEvaluationPosition {
                episode_id: truth.episode_id,
                time_index,
            });
        }
        flattened.extend_from_slice(&truth.truth_positions_m);
    }
    Ok(flattened)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> AnisotropicEpisodeFamilyConfig {
        AnisotropicEpisodeFamilyConfig {
            calibration_cohort_sha256: [0xA1; SHA256_BYTES],
            predictor_sha256: [0xB2; SHA256_BYTES],
            shape_definition_sha256: [0xC3; SHA256_BYTES],
            symmetry_tolerance: 1.0e-12,
            minimum_relative_eigenvalue: 1.0e-8,
        }
    }

    fn member(
        id: u64,
        group: u64,
        predicted: Vec<[f64; DIM]>,
        truth: Vec<[f64; DIM]>,
        shape: [[f64; DIM]; DIM],
    ) -> AnisotropicCalibrationEpisode2d {
        AnisotropicCalibrationEpisode2d {
            episode_id: id,
            independence_group_id: group,
            predicted_positions_m: predicted,
            truth_positions_m: truth,
            shape_matrices: vec![shape;  truth.len()],
            source_manifest_sha256: [((id % 255) + 1) as u8; SHA256_BYTES],
        }
    }

    fn block(
        id: u64,
        group: u64,
        episodes: Vec<AnisotropicCalibrationEpisode2d>,
    ) -> CampaignCalibrationBlock2d {
        CampaignCalibrationBlock2d {
            block_id: id,
            independence_group_id: group,
            source_block_manifest_sha256: [((id % 255) + 10) as u8; SHA256_BYTES],
            member_episodes: episodes,
        }
    }

    fn calibration_blocks() -> Vec<CampaignCalibrationBlock2d> {
        let identity = [[1.0, 0.0], [0.0, 1.0]];
        vec![
            block(101, 1_001, vec![
                member(11, 11_001, vec![[0.0, 0.0]], vec![[1.0, 0.0]], identity),
                member(12, 11_001, vec![[0.0, 0.0]], vec![[0.0, 2.0]], identity),
            ]),
            block(102, 1_002, vec![
                member(21, 11_002, vec![[0.0, 0.0]], vec![[0.0, 1.0]], identity),
            ]),
            block(103, 1_003, vec![
                member(31, 11_003, vec![[0.0, 0.0]], vec![[1.5, 0.0]], identity),
            ]),
        ]
    }

    fn prediction() -> CampaignBlockPrediction2d {
        CampaignBlockPrediction2d {
            block_id: 900,
            independence_group_id: 9_000,
            member_episodes: vec![
                CampaignPredictedEpisode2d {
                    episode_id: 901,
                    predicted_positions_m: vec![[0.0, 0.0]],
                    shape_matrices: vec![[[1.0, 0.0], [0.0, 1.0]]],
                },
                CampaignPredictedEpisode2d {
                    episode_id: 902,
                    predicted_positions_m: vec![[1.0, 1.0], [2.0, 2.0]],
                    shape_matrices: vec![
                        [[1.0, 0.0], [0.0, 1.0]],
                        [[1.0, 0.0], [0.0, 1.0]],
                    ],
                },
            ],
            requested_miscoverage: 0.5,
            predictor_sha256: [0xB2; SHA256_BYTES],
            shape_definition_sha256: [0xC3; SHA256_BYTES],
            decision_sequence_id: 901,
            decision_receipt_sha256: [0xD4; SHA256_BYTES],
        }
    }

    #[test]
    fn calibration_unit_count_is_campaign_count_not_member_episode_count() {
        let family = CampaignBlockAnisotropicConformalFamily2d::fit(
            &calibration_blocks(), config()
        ).unwrap();
        assert_eq!(family.calibration_block_count(), 3);
        assert_eq!(family.calibration_unit_count(), 3);
        assert_eq!(family.calibration_member_episode_count(), 4);
        assert_eq!(family.calibration_member_count(101), Some(2));
        assert_eq!(family.calibration_member_count(102), Some(1));
    }

    #[test]
    fn campaign_score_is_the_maximum_across_every_member_episode_and_position() {
        let family = CampaignBlockAnisotropicConformalFamily2d::fit(
            &calibration_blocks(), config()
        ).unwrap();
        let selected = family.base_family().select_radius(0.5).unwrap().unwrap();
        // Block 101 has max score 2; block 102 has 1; block 103 has 1.5.
        assert!((selected.normalized_radius - 2.0).abs() < 1.0e-12);
        assert_eq!(selected.calibration_episode_count, 3);
    }

    #[test]
    fn splitting_a_campaign_into_more_files_does_not_change_effective_n_or_score() {
        let identity = [[1.0, 0.0], [0.0, 1.0]];
        let unsplit = vec![
            block(101, 1_001, vec![
                member(11, 11_001, vec![[0.0, 0.0], [0.0, 0.0]],
                    vec![[1.0, 0.0], [0.0, 2.0]], identity),
            ]),
            block(102, 1_002, vec![
                member(21, 11_002, vec![[0.0, 0.0]], vec![[0.0, 1.0]], identity),
            ]),
        ];
        let split = vec![
            block(101, 1_001, vec![
                member(11, 11_001, vec![[0.0, 0.0]], vec![[1.0, 0.0]], identity),
                member(12, 11_001, vec![[0.0, 0.0]], vec![[0.0, 2.0]], identity),
            ]),
            block(102, 1_002, vec![
                member(21, 11_002, vec![[0.0, 0.0]], vec![[0.0, 1.0]], identity),
            ]),
        ];
        let first = CampaignBlockAnisotropicConformalFamily2d::fit(&unsplit, config()).unwrap();
        let second = CampaignBlockAnisotropicConformalFamily2d::fit(&split, config()).unwrap();

        assert_eq!(first.calibration_unit_count(), 2);
        assert_eq!(second.calibration_unit_count(), 2);
        let first_radius = first.base_family().select_radius(0.5).unwrap().unwrap().normalized_radius;
        let second_radius = second.base_family().select_radius(0.5).unwrap().unwrap().normalized_radius;
        assert!((first_radius - second_radius).abs() < 1.0e-12);
    }

    #[test]
    fn dependency_groups_cannot_cross_campaign_blocks() {
        let identity = [[1.0, 0.0], [0.0, 1.0]];
        let blocks = vec![
            block(101, 1_001, vec![
                member(11, 77, vec![[0.0, 0.0]], vec![[1.0, 0.0]], identity),
            ]),
            block(102, 1_002, vec![
                member(21, 77, vec![[0.0, 0.0]], vec![[0.0, 1.0]], identity),
            ]),
        ];
        assert_eq!(
            CampaignBlockAnisotropicConformalFamily2d::fit(&blocks, config()),
            Err(CampaignBlockCalibrationError::DependencyGroupCrossesBlocks {
                group_id: 77, first_block: 101, second_block: 102,
            })
        );
    }

    #[test]
    fn duplicate_member_episode_ids_across_blocks_reject() {
        let identity = [[1.0, 0.0], [0.0, 1.0]];
        let blocks = vec![
            block(101, 1_001, vec![
                member(11, 11_001, vec![[0.0, 0.0]], vec![[1.0, 0.0]], identity),
            ]),
            block(102, 1_002, vec![
                member(11, 11_002, vec![[0.0, 0.0]], vec![[0.0, 1.0]], identity),
            ]),
        ];
        assert_eq!(
            CampaignBlockAnisotropicConformalFamily2d::fit(&blocks, config()),
            Err(CampaignBlockCalibrationError::DuplicateMemberEpisodeId(11))
        );
    }

    #[test]
    fn evaluation_builds_one_joint_tube_and_preserves_member_spans() {
        let mut family = CampaignBlockAnisotropicConformalFamily2d::fit(
            &calibration_blocks(), config()
        ).unwrap();
        let prediction = prediction();
        let CampaignBlockSetOutcome2d::Tube(tube) =
            family.build_block_tube(&prediction).unwrap()
        else {
            panic!("expected a finite block tube");
        };
        assert_eq!(tube.member_spans().len(), 2);
        assert_eq!(tube.member_spans()[0].episode_id, 901);
        assert_eq!(tube.member_spans()[0].position_count(), 1);
        assert_eq!(tube.member_spans()[1].episode_id, 902);
        assert_eq!(tube.member_spans()[1].start_index, 1);
        assert_eq!(tube.member_spans()[1].end_index_exclusive, 3);

        let truth = vec![
            CampaignTruthEpisode2d {
                episode_id: 901,
                truth_positions_m: vec![[0.0, 0.0]],
            },
            CampaignTruthEpisode2d {
                episode_id: 902,
                truth_positions_m: vec![[1.0, 1.0], [2.0, 2.0]],
            },
        ];
        assert!(tube.contains_complete_block(&truth).unwrap());
    }

    #[test]
    fn evaluation_block_and_member_identity_are_consume_once() {
        let mut family = CampaignBlockAnisotropicConformalFamily2d::fit(
            &calibration_blocks(), config()
        ).unwrap();
        let prediction = prediction();
        let _ = family.build_block_tube(&prediction).unwrap();
        assert_eq!(
            family.build_block_tube(&prediction),
            Err(CampaignBlockCalibrationError::EvaluationBlockReused(900))
        );
    }

    #[test]
    fn incomplete_member_truth_never_counts_as_block_hit() {
        let mut family = CampaignBlockAnisotropicConformalFamily2d::fit(
            &calibration_blocks(), config()
        ).unwrap();
        let CampaignBlockSetOutcome2d::Tube(tube) =
            family.build_block_tube(&prediction()).unwrap()
        else {
            panic!("expected a finite block tube");
        };
        let truncated = vec![CampaignTruthEpisode2d {
            episode_id: 901,
            truth_positions_m: vec![],
        }];
        assert_eq!(
            tube.contains_complete_block(&truncated),
            Err(CampaignBlockCalibrationError::EvaluationMemberCountMismatch {
                expected: 2, observed: 1,
            })
        );
    }

    #[test]
    fn insufficient_campaign_blocks_abstain_instead_of_using_member_count() {
        let single = vec![calibration_blocks().remove(0)];
        let mut family = CampaignBlockAnisotropicConformalFamily2d::fit(&single, config()).unwrap();
        let mut prediction = prediction();
        prediction.requested_miscoverage = 0.1;
        assert_eq!(
            family.build_block_tube(&prediction).unwrap(),
            CampaignBlockSetOutcome2d::Abstain(
                AnisotropicEpisodeAbstention::InsufficientCalibrationRank {
                    needed_rank: 2,
                    available_episodes: 1,
                }
            )
        );
    }
}
