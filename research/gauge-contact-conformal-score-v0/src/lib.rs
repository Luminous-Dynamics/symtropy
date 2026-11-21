//! Complete-trial pose nonconformity scoring for GAUGE contact-aware prediction sets.
//!
//! This crate does not parse GAUGE files, construct SE(3) errors, or implement
//! conformal calibration. It consumes already-qualified per-body/per-frame
//! translation and rotation discrepancies, converts them to one deterministic
//! whole-trial maximum score, and builds the existing #4147 CalibrationBlock.
//!
//! Translation and rotation diagnostics remain separate. Only their frozen,
//! dimensionless maximum is passed to the caller-owned conformal calibration.

use loco_episode_conformal::{
    ApplicationDecision, ApplicationIdentity, ApplicationMismatch, CalibrationBlock,
    CalibrationDataset, CalibrationReceipt, ConformalThreshold, EpisodeOutcome, EpisodeRecord,
    calibrate, check_application,
};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{Display, Formatter};

/// Fixed number of integer score units per unit of normalized pose error.
pub const SCORE_UNITS_PER_NORMALIZED_ERROR: f64 = 1_000_000_000.0;

/// Maximum representable score is reserved as the explicit incomplete-horizon sentinel.
pub const INCOMPLETE_HORIZON_SCORE: u64 = u64::MAX;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoseScoreError {
    EmptyIdentity,
    InvalidNormalizationScale,
    InvalidScoreDefinitionDigest,
    EmptyExpectedBodyManifest,
    DuplicateExpectedBody,
    EmptyBodyId,
    ZeroExpectedFrameCount,
    UnexpectedBody,
    DuplicateBodyFrame,
    FrameIndexOutOfRange,
    MissingExpectedSamples,
    InvalidPoseError,
    ScoreOverflow,
    EmptyTrialSet,
    DuplicateTrialId,
    InvalidIncompleteOutcome,
    InvalidIncompleteSentinel,
    InvalidBlockIdentity,
    InvalidTrialSourceIdentity,
    MixedBlockIdentity,
    EmptyCalibrationBlockSet,
    DuplicateExchangeabilityUnit,
    CalibrationBlockCountMismatch,
    CalibrationEpisodeCountMismatch,
    CalibrationBlockReceiptMismatch,
    InvalidSourceManifestDigest,
}

impl Display for PoseScoreError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::EmptyIdentity => "trial or exchangeability identity must not be empty",
            Self::InvalidNormalizationScale => {
                "translation and rotation normalization scales must be finite and positive"
            }
            Self::InvalidScoreDefinitionDigest => {
                "score-definition digest must be a 64-character hexadecimal SHA-256 value"
            }
            Self::EmptyExpectedBodyManifest => "expected body manifest must not be empty",
            Self::DuplicateExpectedBody => "expected body identifiers must be unique",
            Self::EmptyBodyId => "body identifier must not be empty",
            Self::ZeroExpectedFrameCount => "each expected body must declare at least one frame",
            Self::UnexpectedBody => {
                "pose error contains a body outside the frozen expected-body manifest"
            }
            Self::DuplicateBodyFrame => "pose error contains a duplicate body/frame pair",
            Self::FrameIndexOutOfRange => {
                "pose error frame index is outside the frozen expected range"
            }
            Self::MissingExpectedSamples => {
                "pose error is missing one or more expected body/frame samples"
            }
            Self::InvalidPoseError => {
                "translation and rotation errors must be finite and non-negative"
            }
            Self::ScoreOverflow => "normalized pose score cannot be represented as u64",
            Self::EmptyTrialSet => "calibration block must contain at least one trial score",
            Self::DuplicateTrialId => {
                "trial identifiers must be unique within an exchangeability block"
            }
            Self::InvalidIncompleteOutcome => {
                "an incomplete horizon must have Failed or Aborted outcome"
            }
            Self::InvalidIncompleteSentinel => {
                "the incomplete-horizon sentinel and disposition disagree"
            }
            Self::InvalidBlockIdentity => {
                "calibration block identity fields must be valid SHA-256 digests"
            }
            Self::InvalidTrialSourceIdentity => {
                "trial source, forecast, revision and model identities must be valid digests"
            }
            Self::MixedBlockIdentity => {
                "trial score identities do not match the declared calibration block identity"
            }
            Self::EmptyCalibrationBlockSet => {
                "at least one calibrated exchangeability block is required"
            }
            Self::DuplicateExchangeabilityUnit => {
                "calibration block exchangeability-unit identifiers must be unique"
            }
            Self::CalibrationBlockCountMismatch => {
                "conformal receipt independent-unit count does not match supplied block receipts"
            }
            Self::CalibrationEpisodeCountMismatch => {
                "conformal receipt episode count does not match supplied block receipts"
            }
            Self::CalibrationBlockReceiptMismatch => {
                "calibration block contents do not match their source-bound score receipts"
            }
            Self::InvalidSourceManifestDigest => {
                "source or prediction manifest must be a 64-character hexadecimal SHA-256 value"
            }
        };
        formatter.write_str(message)
    }
}

impl Error for PoseScoreError {}

/// One expected body stream and the exact number of exported frames included
/// in the preregistered prediction target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectedBodyFrames {
    pub body_id: String,
    pub frame_count: usize,
}

/// Already-qualified pose discrepancy from the upstream frame-aware evaluator.
///
/// Frame index is a zero-based exported sample index, not an absolute sensor
/// timestamp or an inferred high-rate acquisition frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoseErrorSample<'a> {
    pub body_id: &'a str,
    pub frame_index: usize,
    pub translation_error_m: f64,
    pub rotation_error_rad: f64,
}

/// Frozen normalization and score-definition identity.
///
/// The digest must bind the exact translation/rotation scales, units, body-set
/// definition, frame policy, target horizon and quantization version. It is
/// supplied by the preregistered experiment manifest; this crate validates
/// its shape but does not authenticate or generate the source manifest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoseScoreNormalization<'a> {
    pub translation_scale_m: f64,
    pub rotation_scale_rad: f64,
    pub score_definition_sha256: &'a str,
}

impl PoseScoreNormalization<'_> {
    fn validate(&self) -> Result<(), PoseScoreError> {
        if !self.translation_scale_m.is_finite()
            || self.translation_scale_m <= 0.0
            || !self.rotation_scale_rad.is_finite()
            || self.rotation_scale_rad <= 0.0
        {
            return Err(PoseScoreError::InvalidNormalizationScale);
        }
        if self.score_definition_sha256.len() != 64
            || !self
                .score_definition_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(PoseScoreError::InvalidScoreDefinitionDigest);
        }
        Ok(())
    }
}

/// Immutable source and predictor identity for one physical trial score.
///
/// The raw and prediction digests remain in the parallel calibration-block
/// receipt because the conformal core stores only episode IDs and scores.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrialScoreIdentity {
    pub dataset_revision: String,
    pub raw_trial_sha256: String,
    pub prediction_sha256: String,
    pub model_digest: String,
    pub target_digest: String,
    pub domain_digest: String,
}

impl TrialScoreIdentity {
    fn validate(&self) -> Result<(), PoseScoreError> {
        if self.dataset_revision.len() != 40
            || !self
                .dataset_revision
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || !is_sha256(&self.raw_trial_sha256)
            || !is_sha256(&self.prediction_sha256)
            || !is_sha256(&self.model_digest)
            || !is_sha256(&self.target_digest)
            || !is_sha256(&self.domain_digest)
        {
            return Err(PoseScoreError::InvalidTrialSourceIdentity);
        }
        Ok(())
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// One scored trial. Exactly one maximum score is retained per physical trial;
/// all body/frame diagnostics and source identities are bound before emitting.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrialPoseScore {
    pub trial_id: String,
    pub identity: TrialScoreIdentity,
    pub score_definition_digest: String,
    pub outcome: EpisodeOutcome,
    pub maximum_nonconformity_score: u64,
    pub expected_body_count: usize,
    pub expected_sample_count: usize,
    pub observed_sample_count: usize,
    pub incomplete_horizon: bool,
}

impl TrialPoseScore {
    /// Construct the conservative maximum-score sentinel for a failed or
    /// aborted episode whose complete target horizon/reference is unavailable.
    /// The record remains in the exchangeability block; it is never filtered.
    pub fn incomplete(
        trial_id: impl Into<String>,
        outcome: EpisodeOutcome,
        identity: TrialScoreIdentity,
        score_definition_digest: impl Into<String>,
    ) -> Result<Self, PoseScoreError> {
        let trial_id = trial_id.into();
        if trial_id.trim().is_empty() {
            return Err(PoseScoreError::EmptyIdentity);
        }
        if outcome == EpisodeOutcome::Completed {
            return Err(PoseScoreError::InvalidIncompleteOutcome);
        }
        identity.validate()?;
        let score_definition_digest = score_definition_digest.into();
        if !is_sha256(&score_definition_digest) {
            return Err(PoseScoreError::InvalidScoreDefinitionDigest);
        }
        Ok(Self {
            trial_id,
            identity,
            score_definition_digest,
            outcome,
            maximum_nonconformity_score: INCOMPLETE_HORIZON_SCORE,
            expected_body_count: 0,
            expected_sample_count: 0,
            observed_sample_count: 0,
            incomplete_horizon: true,
        })
    }
}

/// Compute one score for a complete trial.
///
/// For every expected body/frame:
///
/// r = max(translation_error_m / translation_scale_m,
///         rotation_error_rad / rotation_scale_rad)
///
/// score = ceil(1e9 * max(r))
///
/// Upward quantization is conservative: rounding cannot make the measured
/// maximum smaller. Missing or duplicated samples fail closed.
pub fn score_complete_trial(
    trial_id: &str,
    outcome: EpisodeOutcome,
    expected_bodies: &[ExpectedBodyFrames],
    samples: &[PoseErrorSample<'_>],
    normalization: PoseScoreNormalization<'_>,
    identity: TrialScoreIdentity,
) -> Result<TrialPoseScore, PoseScoreError> {
    if trial_id.trim().is_empty() {
        return Err(PoseScoreError::EmptyIdentity);
    }
    normalization.validate()?;
    identity.validate()?;
    if expected_bodies.is_empty() {
        return Err(PoseScoreError::EmptyExpectedBodyManifest);
    }

    let mut expected = BTreeSet::new();
    let mut expected_sample_count = 0usize;
    let mut body_ids = BTreeSet::new();
    for body in expected_bodies {
        if body.body_id.trim().is_empty() {
            return Err(PoseScoreError::EmptyBodyId);
        }
        if body.frame_count == 0 {
            return Err(PoseScoreError::ZeroExpectedFrameCount);
        }
        if !body_ids.insert(body.body_id.as_str()) {
            return Err(PoseScoreError::DuplicateExpectedBody);
        }
        expected_sample_count = expected_sample_count
            .checked_add(body.frame_count)
            .ok_or(PoseScoreError::ScoreOverflow)?;
        for frame_index in 0..body.frame_count {
            expected.insert((body.body_id.as_str(), frame_index));
        }
    }

    if samples.len() != expected_sample_count {
        return Err(PoseScoreError::MissingExpectedSamples);
    }

    let mut seen = BTreeSet::new();
    let mut maximum_normalized_error = 0.0_f64;
    for sample in samples {
        if !body_ids.contains(sample.body_id) {
            return Err(PoseScoreError::UnexpectedBody);
        }
        let body_frame_count = expected_bodies
            .iter()
            .find(|body| body.body_id == sample.body_id)
            .map(|body| body.frame_count)
            .unwrap_or(0);
        if sample.frame_index >= body_frame_count {
            return Err(PoseScoreError::FrameIndexOutOfRange);
        }
        if !seen.insert((sample.body_id, sample.frame_index)) {
            return Err(PoseScoreError::DuplicateBodyFrame);
        }
        if !sample.translation_error_m.is_finite()
            || sample.translation_error_m < 0.0
            || !sample.rotation_error_rad.is_finite()
            || sample.rotation_error_rad < 0.0
        {
            return Err(PoseScoreError::InvalidPoseError);
        }
        let translation = sample.translation_error_m / normalization.translation_scale_m;
        let rotation = sample.rotation_error_rad / normalization.rotation_scale_rad;
        let normalized = translation.max(rotation);
        if !normalized.is_finite() {
            return Err(PoseScoreError::ScoreOverflow);
        }
        maximum_normalized_error = maximum_normalized_error.max(normalized);
    }

    if seen.len() != expected.len() {
        return Err(PoseScoreError::MissingExpectedSamples);
    }

    let scaled = maximum_normalized_error * SCORE_UNITS_PER_NORMALIZED_ERROR;
    if !scaled.is_finite() || scaled >= u64::MAX as f64 {
        return Err(PoseScoreError::ScoreOverflow);
    }
    let quantized = scaled.ceil();
    let score = quantized as u64;

    Ok(TrialPoseScore {
        trial_id: trial_id.to_owned(),
        identity,
        score_definition_digest: normalization.score_definition_sha256.to_owned(),
        outcome,
        maximum_nonconformity_score: score,
        expected_body_count: expected_bodies.len(),
        expected_sample_count,
        observed_sample_count: seen.len(),
        incomplete_horizon: false,
    })
}

/// Exact identity tuple required by the existing #4147 calibration block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CalibrationBlockIdentity {
    pub dataset_revision: String,
    pub model_digest: String,
    pub target_digest: String,
    pub domain_digest: String,
    pub score_definition_digest: String,
    pub calibration_split_digest: String,
}

/// Complete adapter output: the exact #4147 block plus the source-bound trial
/// score records needed to replay and audit the block maximum.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CalibrationBlockReceipt {
    pub identity: CalibrationBlockIdentity,
    pub calibration_block: CalibrationBlock,
    pub trial_score_records: Vec<TrialPoseScore>,
}

/// Build the existing #4147 block from all trial maxima in one independent
/// session/article/trial exchangeability unit. Duplicate trial IDs are
/// rejected. Failed and aborted records are retained with their outcomes.
pub fn build_calibration_block(
    exchangeability_unit_id: &str,
    trials: &[TrialPoseScore],
    identity: &CalibrationBlockIdentity,
) -> Result<CalibrationBlockReceipt, PoseScoreError> {
    if exchangeability_unit_id.trim().is_empty() {
        return Err(PoseScoreError::EmptyIdentity);
    }
    if trials.is_empty() {
        return Err(PoseScoreError::EmptyTrialSet);
    }
    if identity.dataset_revision.len() != 40
        || !identity
            .dataset_revision
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        || !is_sha256(&identity.model_digest)
        || !is_sha256(&identity.target_digest)
        || !is_sha256(&identity.domain_digest)
        || !is_sha256(&identity.score_definition_digest)
        || !is_sha256(&identity.calibration_split_digest)
    {
        return Err(PoseScoreError::InvalidBlockIdentity);
    }

    let mut sorted_trials = trials.to_vec();
    sorted_trials.sort_by(|left, right| left.trial_id.as_bytes().cmp(right.trial_id.as_bytes()));
    let mut trial_ids = BTreeSet::new();
    let mut episodes = Vec::with_capacity(sorted_trials.len());
    let mut maximum_nonconformity_score = 0u64;
    for trial in &sorted_trials {
        if trial.trial_id.trim().is_empty() {
            return Err(PoseScoreError::EmptyIdentity);
        }
        if !trial_ids.insert(trial.trial_id.as_str()) {
            return Err(PoseScoreError::DuplicateTrialId);
        }
        if trial.identity.dataset_revision != identity.dataset_revision
            || trial.identity.model_digest != identity.model_digest
            || trial.identity.target_digest != identity.target_digest
            || trial.identity.domain_digest != identity.domain_digest
            || trial.score_definition_digest != identity.score_definition_digest
        {
            return Err(PoseScoreError::MixedBlockIdentity);
        }
        trial.identity.validate()?;
        if trial.incomplete_horizon {
            if trial.outcome == EpisodeOutcome::Completed
                || trial.maximum_nonconformity_score != INCOMPLETE_HORIZON_SCORE
            {
                return Err(PoseScoreError::InvalidIncompleteSentinel);
            }
        } else if trial.maximum_nonconformity_score == INCOMPLETE_HORIZON_SCORE {
            return Err(PoseScoreError::InvalidIncompleteSentinel);
        }
        maximum_nonconformity_score =
            maximum_nonconformity_score.max(trial.maximum_nonconformity_score);
        episodes.push(EpisodeRecord {
            episode_id: trial.trial_id.clone(),
            outcome: trial.outcome,
        });
    }

    Ok(CalibrationBlockReceipt {
        identity: identity.clone(),
        calibration_block: CalibrationBlock {
            exchangeability_unit_id: exchangeability_unit_id.to_owned(),
            episodes,
            maximum_nonconformity_score,
            model_digest: identity.model_digest.clone(),
            target_digest: identity.target_digest.clone(),
            domain_digest: identity.domain_digest.clone(),
            score_definition_digest: identity.score_definition_digest.clone(),
            calibration_split_digest: identity.calibration_split_digest.clone(),
        },
        trial_score_records: sorted_trials,
    })
}

/// A GAUGE-specific wrapper around the sole conformal receipt.
///
/// #4147 intentionally remains task-agnostic and therefore does not include
/// dataset revision or raw-source manifest identity. This wrapper restores
/// those physical-data boundaries without changing the generic conformal core.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoseCalibrationReceipt {
    pub conformal: CalibrationReceipt,
    pub dataset_revision: String,
    pub source_manifest_sha256: String,
    pub prediction_manifest_sha256: String,
}

/// Validate that an existing #4147 calibration receipt was computed from the
/// supplied source-bound GAUGE blocks, then retain exact dataset and manifest
/// identities alongside it.
///
/// The manifest digests are caller-computed over frozen, deterministically
/// ordered manifests. This function checks their shape, not their cryptographic
/// construction or the scientific validity of the manifest contents.
pub fn bind_pose_calibration_receipt(
    conformal: CalibrationReceipt,
    blocks: &[CalibrationBlockReceipt],
    source_manifest_sha256: &str,
    prediction_manifest_sha256: &str,
) -> Result<PoseCalibrationReceipt, PoseScoreError> {
    if blocks.is_empty() {
        return Err(PoseScoreError::EmptyCalibrationBlockSet);
    }
    if !is_sha256(source_manifest_sha256) || !is_sha256(prediction_manifest_sha256) {
        return Err(PoseScoreError::InvalidSourceManifestDigest);
    }

    let first_identity = &blocks[0].identity;
    let mut exchangeability_units = BTreeSet::new();
    let mut trial_ids = BTreeSet::new();
    let mut episode_count = 0usize;

    for block_receipt in blocks {
        let identity = &block_receipt.identity;
        if identity != first_identity {
            return Err(PoseScoreError::MixedBlockIdentity);
        }
        if !exchangeability_units.insert(
            block_receipt
                .calibration_block
                .exchangeability_unit_id
                .as_str(),
        ) {
            return Err(PoseScoreError::DuplicateExchangeabilityUnit);
        }

        let block = &block_receipt.calibration_block;
        if block.model_digest != identity.model_digest
            || block.target_digest != identity.target_digest
            || block.domain_digest != identity.domain_digest
            || block.score_definition_digest != identity.score_definition_digest
            || block.calibration_split_digest != identity.calibration_split_digest
            || block.episodes.len() != block_receipt.trial_score_records.len()
        {
            return Err(PoseScoreError::CalibrationBlockReceiptMismatch);
        }

        let mut maximum_score = 0u64;
        for (episode, trial) in block
            .episodes
            .iter()
            .zip(block_receipt.trial_score_records.iter())
        {
            if episode.episode_id != trial.trial_id
                || episode.outcome != trial.outcome
                || trial.identity.dataset_revision != identity.dataset_revision
                || trial.identity.model_digest != identity.model_digest
                || trial.identity.target_digest != identity.target_digest
                || trial.identity.domain_digest != identity.domain_digest
                || trial.score_definition_digest != identity.score_definition_digest
                || !trial_ids.insert(trial.trial_id.as_str())
            {
                return Err(PoseScoreError::CalibrationBlockReceiptMismatch);
            }
            maximum_score = maximum_score.max(trial.maximum_nonconformity_score);
            episode_count = episode_count
                .checked_add(1)
                .ok_or(PoseScoreError::ScoreOverflow)?;
        }
        if maximum_score != block.maximum_nonconformity_score {
            return Err(PoseScoreError::CalibrationBlockReceiptMismatch);
        }
    }

    if conformal.independent_unit_count != blocks.len() {
        return Err(PoseScoreError::CalibrationBlockCountMismatch);
    }
    if conformal.episode_count != episode_count {
        return Err(PoseScoreError::CalibrationEpisodeCountMismatch);
    }
    if conformal.model_digest != first_identity.model_digest
        || conformal.target_digest != first_identity.target_digest
        || conformal.domain_digest != first_identity.domain_digest
        || conformal.score_definition_digest != first_identity.score_definition_digest
        || conformal.calibration_split_digest != first_identity.calibration_split_digest
    {
        return Err(PoseScoreError::CalibrationBlockReceiptMismatch);
    }

    let replayed = calibrate(&CalibrationDataset {
        model_digest: first_identity.model_digest.clone(),
        target_digest: first_identity.target_digest.clone(),
        domain_digest: first_identity.domain_digest.clone(),
        score_definition_digest: first_identity.score_definition_digest.clone(),
        calibration_split_digest: first_identity.calibration_split_digest.clone(),
        alpha_ppm: conformal.alpha_ppm,
        blocks: blocks
            .iter()
            .map(|block| block.calibration_block.clone())
            .collect(),
        // #4147 already checked fit/test leakage when the original receipt
        // was produced. The split digest binds that frozen manifest; replay
        // here checks the rank/threshold/receipt against the source blocks.
        forbidden_episode_ids: Vec::new(),
    })
    .map_err(|_| PoseScoreError::CalibrationBlockReceiptMismatch)?;
    if replayed != conformal {
        return Err(PoseScoreError::CalibrationBlockReceiptMismatch);
    }

    Ok(PoseCalibrationReceipt {
        conformal,
        dataset_revision: first_identity.dataset_revision.clone(),
        source_manifest_sha256: source_manifest_sha256.to_owned(),
        prediction_manifest_sha256: prediction_manifest_sha256.to_owned(),
    })
}

/// Immutable application identity for a GAUGE pose calibration receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PoseSetApplicationIdentity<'a> {
    pub dataset_revision: &'a str,
    pub source_manifest_sha256: &'a str,
    pub conformal: ApplicationIdentity<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoseSetApplicationMismatch {
    DatasetRevision,
    SourceManifestDigest,
    ModelDigest,
    TargetDigest,
    DomainDigest,
    ScoreDefinitionDigest,
}

/// A prediction-set threshold for the GAUGE pose-score quantization.
///
/// A #4147 receipt whose numeric threshold reaches the reserved incomplete
/// sentinel must be treated as unbounded. It must not be displayed as a
/// finite-width pose guarantee.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoseSetThreshold {
    Finite(u64),
    Unbounded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoseSetApplicationDecision {
    Applicable { threshold: PoseSetThreshold },
    NotApplicable { reason: PoseSetApplicationMismatch },
}

/// Check both GAUGE source identity and exact #4147 application identity, then
/// map finite-sample unbounded results and the incomplete sentinel into set semantics.
pub fn check_pose_set_application(
    receipt: &PoseCalibrationReceipt,
    identity: PoseSetApplicationIdentity<'_>,
) -> PoseSetApplicationDecision {
    if receipt.dataset_revision != identity.dataset_revision {
        return PoseSetApplicationDecision::NotApplicable {
            reason: PoseSetApplicationMismatch::DatasetRevision,
        };
    }
    if receipt.source_manifest_sha256 != identity.source_manifest_sha256 {
        return PoseSetApplicationDecision::NotApplicable {
            reason: PoseSetApplicationMismatch::SourceManifestDigest,
        };
    }
    match check_application(&receipt.conformal, identity.conformal) {
        ApplicationDecision::NotApplicable { reason } => {
            let reason = match reason {
                ApplicationMismatch::ModelDigest => PoseSetApplicationMismatch::ModelDigest,
                ApplicationMismatch::TargetDigest => PoseSetApplicationMismatch::TargetDigest,
                ApplicationMismatch::DomainDigest => PoseSetApplicationMismatch::DomainDigest,
                ApplicationMismatch::ScoreDefinitionDigest => {
                    PoseSetApplicationMismatch::ScoreDefinitionDigest
                }
            };
            PoseSetApplicationDecision::NotApplicable { reason }
        }
        ApplicationDecision::Applicable {
            threshold: ConformalThreshold::Unbounded,
        } => PoseSetApplicationDecision::Applicable {
            threshold: PoseSetThreshold::Unbounded,
        },
        ApplicationDecision::Applicable {
            threshold: ConformalThreshold::Finite(INCOMPLETE_HORIZON_SCORE),
        } => PoseSetApplicationDecision::Applicable {
            threshold: PoseSetThreshold::Unbounded,
        },
        ApplicationDecision::Applicable {
            threshold: ConformalThreshold::Finite(value),
        } => PoseSetApplicationDecision::Applicable {
            threshold: PoseSetThreshold::Finite(value),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const MODEL_DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const TARGET_DIGEST: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const DOMAIN_DIGEST: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const RAW_DIGEST: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const FORECAST_DIGEST: &str =
        "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    const DATASET_REVISION: &str = "f95bb06e7d11142f88ffd3b6b56c7258deb8fff6";

    fn normalization() -> PoseScoreNormalization<'static> {
        PoseScoreNormalization {
            translation_scale_m: 0.01,
            rotation_scale_rad: 0.1,
            score_definition_sha256: DIGEST,
        }
    }

    fn score_identity() -> TrialScoreIdentity {
        TrialScoreIdentity {
            dataset_revision: DATASET_REVISION.to_owned(),
            raw_trial_sha256: RAW_DIGEST.to_owned(),
            prediction_sha256: FORECAST_DIGEST.to_owned(),
            model_digest: MODEL_DIGEST.to_owned(),
            target_digest: TARGET_DIGEST.to_owned(),
            domain_digest: DOMAIN_DIGEST.to_owned(),
        }
    }

    fn expected() -> [ExpectedBodyFrames; 2] {
        [
            ExpectedBodyFrames {
                body_id: "body-a".to_owned(),
                frame_count: 2,
            },
            ExpectedBodyFrames {
                body_id: "body-b".to_owned(),
                frame_count: 1,
            },
        ]
    }

    fn samples() -> [PoseErrorSample<'static>; 3] {
        [
            PoseErrorSample {
                body_id: "body-a",
                frame_index: 0,
                translation_error_m: 0.002,
                rotation_error_rad: 0.01,
            },
            PoseErrorSample {
                body_id: "body-a",
                frame_index: 1,
                translation_error_m: 0.004,
                rotation_error_rad: 0.02,
            },
            PoseErrorSample {
                body_id: "body-b",
                frame_index: 0,
                translation_error_m: 0.001,
                rotation_error_rad: 0.04,
            },
        ]
    }

    fn block_identity() -> CalibrationBlockIdentity {
        CalibrationBlockIdentity {
            dataset_revision: DATASET_REVISION.to_owned(),
            model_digest: MODEL_DIGEST.to_owned(),
            target_digest: TARGET_DIGEST.to_owned(),
            domain_digest: DOMAIN_DIGEST.to_owned(),
            score_definition_digest: DIGEST.to_owned(),
            calibration_split_digest:
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_owned(),
        }
    }

    fn calibration_receipt_for_blocks(
        blocks: &[CalibrationBlockReceipt],
        alpha_ppm: u32,
    ) -> CalibrationReceipt {
        let identity = &blocks[0].identity;
        calibrate(&CalibrationDataset {
            model_digest: identity.model_digest.clone(),
            target_digest: identity.target_digest.clone(),
            domain_digest: identity.domain_digest.clone(),
            score_definition_digest: identity.score_definition_digest.clone(),
            calibration_split_digest: identity.calibration_split_digest.clone(),
            alpha_ppm,
            blocks: blocks
                .iter()
                .map(|block| block.calibration_block.clone())
                .collect(),
            forbidden_episode_ids: Vec::new(),
        })
        .unwrap()
    }

    fn incomplete_blocks(count: usize) -> Vec<CalibrationBlockReceipt> {
        (0..count)
            .map(|index| {
                let trial = TrialPoseScore::incomplete(
                    format!("failed-trial-{index:02}"),
                    EpisodeOutcome::Failed,
                    score_identity(),
                    DIGEST,
                )
                .unwrap();
                build_calibration_block(&format!("session-{index:02}"), &[trial], &block_identity())
                    .unwrap()
            })
            .collect()
    }

    #[test]
    fn score_is_the_maximum_over_all_bodies_and_frames_with_upward_quantization() {
        let score = score_complete_trial(
            "trial-01",
            EpisodeOutcome::Completed,
            &expected(),
            &samples(),
            normalization(),
            score_identity(),
        )
        .unwrap();
        // Maximum normalized error is 0.04 / 0.1 = 0.4.
        assert_eq!(score.maximum_nonconformity_score, 400_000_000);
        assert_eq!(score.expected_body_count, 2);
        assert_eq!(score.expected_sample_count, 3);
        assert_eq!(score.observed_sample_count, 3);
        assert!(!score.incomplete_horizon);
    }

    #[test]
    fn input_order_does_not_change_the_trial_maximum() {
        let original = samples();
        let permuted = [original[2], original[0], original[1]];
        let a = score_complete_trial(
            "trial-01",
            EpisodeOutcome::Completed,
            &expected(),
            &original,
            normalization(),
            score_identity(),
        )
        .unwrap();
        let b = score_complete_trial(
            "trial-01",
            EpisodeOutcome::Completed,
            &expected(),
            &permuted,
            normalization(),
            score_identity(),
        )
        .unwrap();
        assert_eq!(a.maximum_nonconformity_score, b.maximum_nonconformity_score);
    }

    #[test]
    fn missing_pose_frame_fails_closed() {
        let mut rows = samples().to_vec();
        rows.pop();
        assert_eq!(
            score_complete_trial(
                "trial-01",
                EpisodeOutcome::Completed,
                &expected(),
                &rows,
                normalization(),
                score_identity(),
            ),
            Err(PoseScoreError::MissingExpectedSamples)
        );
    }

    #[test]
    fn duplicate_body_frame_fails_closed() {
        let rows = [samples()[0], samples()[1], samples()[1]];
        assert_eq!(
            score_complete_trial(
                "trial-01",
                EpisodeOutcome::Completed,
                &expected(),
                &rows,
                normalization(),
                score_identity(),
            ),
            Err(PoseScoreError::DuplicateBodyFrame)
        );
    }

    #[test]
    fn unexpected_body_fails_closed() {
        let mut rows = samples();
        rows[2] = PoseErrorSample {
            body_id: "not-in-manifest",
            ..rows[2]
        };
        assert_eq!(
            score_complete_trial(
                "trial-01",
                EpisodeOutcome::Completed,
                &expected(),
                &rows,
                normalization(),
                score_identity(),
            ),
            Err(PoseScoreError::UnexpectedBody)
        );
    }

    #[test]
    fn non_finite_or_negative_pose_error_fails_closed() {
        let mut rows = samples();
        rows[0] = PoseErrorSample {
            translation_error_m: f64::NAN,
            ..rows[0]
        };
        assert_eq!(
            score_complete_trial(
                "trial-01",
                EpisodeOutcome::Completed,
                &expected(),
                &rows,
                normalization(),
                score_identity(),
            ),
            Err(PoseScoreError::InvalidPoseError)
        );
        rows[0] = PoseErrorSample {
            rotation_error_rad: -0.1,
            ..samples()[0]
        };
        assert_eq!(
            score_complete_trial(
                "trial-01",
                EpisodeOutcome::Completed,
                &expected(),
                &rows,
                normalization(),
                score_identity(),
            ),
            Err(PoseScoreError::InvalidPoseError)
        );
    }

    #[test]
    fn zero_or_invalid_normalization_and_digest_are_rejected() {
        let mut scale = normalization();
        scale.translation_scale_m = 0.0;
        assert_eq!(
            score_complete_trial(
                "trial-01",
                EpisodeOutcome::Completed,
                &expected(),
                &samples(),
                scale,
                score_identity(),
            ),
            Err(PoseScoreError::InvalidNormalizationScale)
        );
        let mut scale = normalization();
        scale.score_definition_sha256 = "bad";
        assert_eq!(
            score_complete_trial(
                "trial-01",
                EpisodeOutcome::Completed,
                &expected(),
                &samples(),
                scale,
                score_identity(),
            ),
            Err(PoseScoreError::InvalidScoreDefinitionDigest)
        );
    }

    #[test]
    fn incomplete_failed_or_aborted_trial_uses_maximum_sentinel_and_is_retained() {
        assert_eq!(
            TrialPoseScore::incomplete(
                "trial-02",
                EpisodeOutcome::Completed,
                score_identity(),
                DIGEST
            ),
            Err(PoseScoreError::InvalidIncompleteOutcome)
        );
        let failed = TrialPoseScore::incomplete(
            "trial-02",
            EpisodeOutcome::Failed,
            score_identity(),
            DIGEST,
        )
        .unwrap();
        let aborted = TrialPoseScore::incomplete(
            "trial-03",
            EpisodeOutcome::Aborted,
            score_identity(),
            DIGEST,
        )
        .unwrap();
        let block =
            build_calibration_block("session-01", &[failed, aborted], &block_identity()).unwrap();
        assert_eq!(block.calibration_block.episodes.len(), 2);
        assert_eq!(
            block.calibration_block.maximum_nonconformity_score,
            INCOMPLETE_HORIZON_SCORE
        );
        assert_eq!(
            block.calibration_block.episodes[0].outcome,
            EpisodeOutcome::Failed
        );
        assert_eq!(
            block.calibration_block.episodes[1].outcome,
            EpisodeOutcome::Aborted
        );
        assert_eq!(block.trial_score_records.len(), 2);
    }

    #[test]
    fn calibration_block_takes_one_maximum_across_trials_in_one_unit() {
        let a = score_complete_trial(
            "trial-01",
            EpisodeOutcome::Completed,
            &expected(),
            &samples(),
            normalization(),
            score_identity(),
        )
        .unwrap();
        let mut lower = samples();
        lower[2] = PoseErrorSample {
            rotation_error_rad: 0.01,
            ..lower[2]
        };
        let b = score_complete_trial(
            "trial-02",
            EpisodeOutcome::Completed,
            &expected(),
            &lower,
            normalization(),
            score_identity(),
        )
        .unwrap();
        let block =
            build_calibration_block("session-01", &[a.clone(), b], &block_identity()).unwrap();
        assert_eq!(
            block.calibration_block.maximum_nonconformity_score,
            a.maximum_nonconformity_score
        );
        assert_eq!(block.calibration_block.episodes.len(), 2);
    }

    #[test]
    fn duplicate_trial_ids_in_one_exchangeability_block_are_rejected() {
        let trial = score_complete_trial(
            "trial-01",
            EpisodeOutcome::Completed,
            &expected(),
            &samples(),
            normalization(),
            score_identity(),
        )
        .unwrap();
        assert_eq!(
            build_calibration_block("session-01", &[trial.clone(), trial], &block_identity()),
            Err(PoseScoreError::DuplicateTrialId)
        );
    }

    #[test]
    fn block_rejects_mixed_model_domain_or_score_definition_identities() {
        let mut trial = score_complete_trial(
            "trial-01",
            EpisodeOutcome::Completed,
            &expected(),
            &samples(),
            normalization(),
            score_identity(),
        )
        .unwrap();
        trial.identity.model_digest =
            "9999999999999999999999999999999999999999999999999999999999999999".to_owned();
        assert_eq!(
            build_calibration_block("session-01", &[trial], &block_identity()),
            Err(PoseScoreError::MixedBlockIdentity)
        );
    }

    #[test]
    fn block_rejects_mixed_dataset_revisions() {
        let mut trial = score_complete_trial(
            "trial-01",
            EpisodeOutcome::Completed,
            &expected(),
            &samples(),
            normalization(),
            score_identity(),
        )
        .unwrap();
        trial.identity.dataset_revision = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned();
        assert_eq!(
            build_calibration_block("session-01", &[trial], &block_identity()),
            Err(PoseScoreError::MixedBlockIdentity)
        );
    }

    #[test]
    fn block_receipt_preserves_raw_and_forecast_digests() {
        let trial = score_complete_trial(
            "trial-01",
            EpisodeOutcome::Completed,
            &expected(),
            &samples(),
            normalization(),
            score_identity(),
        )
        .unwrap();
        let receipt = build_calibration_block("session-01", &[trial], &block_identity()).unwrap();
        assert_eq!(receipt.trial_score_records.len(), 1);
        assert_eq!(
            receipt.trial_score_records[0].identity.raw_trial_sha256,
            RAW_DIGEST
        );
        assert_eq!(
            receipt.trial_score_records[0].identity.prediction_sha256,
            FORECAST_DIGEST
        );
        assert_eq!(
            receipt.trial_score_records[0].identity.dataset_revision,
            DATASET_REVISION
        );
    }

    #[test]
    fn sentinel_threshold_becomes_unbounded_pose_set() {
        let blocks = incomplete_blocks(19);
        let conformal = calibration_receipt_for_blocks(&blocks, 50_000);
        assert_eq!(
            conformal.threshold,
            ConformalThreshold::Finite(INCOMPLETE_HORIZON_SCORE)
        );
        assert_eq!(conformal.rank_1based, 19);
        let receipt =
            bind_pose_calibration_receipt(conformal, &blocks, RAW_DIGEST, FORECAST_DIGEST).unwrap();
        let identity = PoseSetApplicationIdentity {
            dataset_revision: DATASET_REVISION,
            source_manifest_sha256: RAW_DIGEST,
            conformal: ApplicationIdentity {
                model_digest: MODEL_DIGEST,
                target_digest: TARGET_DIGEST,
                domain_digest: DOMAIN_DIGEST,
                score_definition_digest: DIGEST,
            },
        };
        let decision = check_pose_set_application(&receipt, identity);
        assert_eq!(
            decision,
            PoseSetApplicationDecision::Applicable {
                threshold: PoseSetThreshold::Unbounded
            }
        );
    }

    #[test]
    fn pose_calibration_receipt_refuses_a_different_dataset_or_source_manifest() {
        let blocks = incomplete_blocks(1);
        let conformal = calibration_receipt_for_blocks(&blocks, 50_000);
        let receipt =
            bind_pose_calibration_receipt(conformal, &blocks, RAW_DIGEST, FORECAST_DIGEST).unwrap();
        let base = PoseSetApplicationIdentity {
            dataset_revision: DATASET_REVISION,
            source_manifest_sha256: RAW_DIGEST,
            conformal: ApplicationIdentity {
                model_digest: MODEL_DIGEST,
                target_digest: TARGET_DIGEST,
                domain_digest: DOMAIN_DIGEST,
                score_definition_digest: DIGEST,
            },
        };
        assert_eq!(
            check_pose_set_application(
                &receipt,
                PoseSetApplicationIdentity {
                    dataset_revision: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    ..base
                }
            ),
            PoseSetApplicationDecision::NotApplicable {
                reason: PoseSetApplicationMismatch::DatasetRevision
            }
        );
        assert_eq!(
            check_pose_set_application(
                &receipt,
                PoseSetApplicationIdentity {
                    source_manifest_sha256: FORECAST_DIGEST,
                    ..base
                }
            ),
            PoseSetApplicationDecision::NotApplicable {
                reason: PoseSetApplicationMismatch::SourceManifestDigest
            }
        );
    }
}
