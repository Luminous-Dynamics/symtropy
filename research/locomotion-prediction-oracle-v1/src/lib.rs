// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A small, dependency-free oracle for blinded locomotion-model predictions.
//!
//! This crate validates prediction receipts and screens whether declared model
//! predictions are distinguishable under a declared observation model. It does
//! not run a biological model, authenticate external signatures, calculate
//! expected information gain, or establish biological truth.
//!
//! SHA-256 values are references computed by the upstream artifact/receipt
//! authority. This crate validates their syntax and declared binding; it does
//! not recompute or authenticate those digests.

#![forbid(unsafe_code)]
// Dense matrix routines intentionally use explicit row/column indices. The
// indices couple several matrices and triangular factors in the same operation.
#![allow(clippy::needless_range_loop)]

use std::collections::BTreeSet;

const MAX_MODELS_PER_INTERVENTION: usize = 16;
const MAX_FEATURE_DIMENSION: usize = 64;
const MAX_ARTIFACTS_PER_MODEL: usize = 32;
const MAX_EVALUATION_ARTIFACTS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceTier {
    Synthetic,
    Phenomenological,
    ConnectomeConstrained,
    PerturbationGrounded,
    ReplicatedBiological,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFamily {
    SyntheticOracle,
    PhenomenologicalController,
    RuleBasedController,
    LearnedController,
    ConnectomeCircuit,
    ConnectomeGroundedReflex,
    ConnectomeHybrid,
    MechanisticHybrid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationTier {
    SyntheticHarness,
    InSilicoEmbodied,
    MeasuredIntervention,
    IndependentReplication,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ArtifactRole {
    Connectome,
    BiologicalSource,
    SensorMapping,
    EffectorMapping,
    BodyMorphology,
    PhysicsConfiguration,
    MeasuredDataset,
    IndependentReplication,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRef {
    pub role: ArtifactRole,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelProvenance {
    pub model_sha256: String,
    pub code_sha256: String,
    pub parameter_sha256: String,
    pub configuration_sha256: String,
    pub artifacts: Vec<ArtifactRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelForecast {
    pub model_id: String,
    pub intervention_id: String,
    pub model_family: ModelFamily,
    pub evidence_tier: EvidenceTier,
    /// Ordered IDs from the frozen feature schema. Order is semantically binding.
    pub feature_ids: Vec<String>,
    /// Predictive feature means in the exact order of feature_ids.
    pub mean: Vec<f64>,
    /// Predictive covariance in the exact order of feature_ids.
    pub covariance: Vec<Vec<f64>>,
    /// Digest of the raw time-resolved trajectory artifact.
    pub raw_trajectory_sha256: String,
    /// Digest of the preregistered feature extraction result.
    pub feature_summary_sha256: String,
    /// Digest of the complete forecast payload and provenance envelope.
    pub forecast_sha256: String,
    /// Append-only evidence-log sequence at which this forecast was committed.
    pub commit_sequence: u64,
    pub provenance: ModelProvenance,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PredictionBatch {
    pub protocol_id: String,
    pub intervention_id: String,
    pub protocol_sha256: String,
    pub split_sha256: String,
    pub scenario_sha256: String,
    pub feature_schema_sha256: String,
    /// Digest of the complete batch, supplied by the external receipt authority.
    pub batch_sha256: String,
    /// Digest of the commit event in the declared append-only evidence log.
    pub commit_receipt_sha256: String,
    pub event_log_id: String,
    pub commit_sequence: u64,
    pub evaluation_tier: EvaluationTier,
    /// Shared body/physics/data artifacts for this matched evaluation.
    pub evaluation_artifacts: Vec<ArtifactRef>,
    /// Covariance of one measured feature vector for Gaussian forecasts.
    pub observation_covariance: Vec<Vec<f64>>,
    /// Covariance of observation error in a pairwise prediction difference.
    pub pairwise_observation_difference_covariance: Vec<Vec<f64>>,
    pub forecasts: Vec<ModelForecast>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnblindingReceipt {
    pub event_log_id: String,
    pub sequence: u64,
    pub outcomes_sha256: String,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    InvalidIdentifier(&'static str),
    InvalidDigest(&'static str),
    MissingArtifactRole(ArtifactRole),
    ConnectomeFamilyBelowEvidenceTier,
    ConnectomeTierRequiresConnectomeFamily,
    TooFewModels,
    DuplicateModelId,
    InterventionMismatch,
    FeatureSchemaMismatch,
    DuplicateFeatureId,
    DimensionMismatch,
    NonFiniteValue,
    InvalidCovariance,
    NonPositiveDefiniteCovariance,
    NegativeDistributionDistance,
    InvalidPolicy,
    ResourceLimitExceeded,
    DuplicateArtifactRole,
    InvalidCommitSequence,
    ForecastCommittedAfterBatch,
    CommitSequenceAfterUnblinding,
    EventLogMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiscriminationPolicy {
    /// Minimum pairwise mean-only squared Mahalanobis separation required to
    /// call a pair resolved for this benchmark. Freeze it before unblinding.
    pub minimum_pair_mean_d2: f64,
    /// Minimum Gaussian Bhattacharyya distance for pairwise distribution separation.
    pub minimum_gaussian_bhattacharyya_distance: f64,
    /// Fixed diagonal regularizer added before covariance factorization.
    pub covariance_regularization: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscriminationStatus {
    /// Every candidate model pair exceeds the preregistered mean-separation threshold.
    AllPairsMeanSeparated,
    /// Some pairs show mean separation, but at least one pair remains unresolved.
    SomePairsMeanSeparated,
    /// No candidate pair exceeds the preregistered mean-separation threshold.
    NoPairsMeanSeparated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaussianDiscriminationStatus {
    AllPairsGaussianSeparated,
    SomePairsGaussianSeparated,
    NoPairsGaussianSeparated,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PairSeparation {
    pub model_a: String,
    pub model_b: String,
    pub mean_d2: f64,
    pub mean_separated: bool,
    pub gaussian_bhattacharyya_distance: f64,
    pub gaussian_distributions_separated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscriminationResult {
    pub status: DiscriminationStatus,
    pub minimum_pair_mean_d2: f64,
    pub mean_separated_pair_count: usize,
    pub mean_unseparated_pair_count: usize,
    pub gaussian_status: GaussianDiscriminationStatus,
    pub minimum_pair_gaussian_bhattacharyya_distance: f64,
    pub gaussian_separated_pair_count: usize,
    pub gaussian_unseparated_pair_count: usize,
    pub pairwise: Vec<PairSeparation>,
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

fn validate_identifier(value: &str, field: &'static str) -> Result<(), ValidationError> {
    if valid_identifier(value) {
        Ok(())
    } else {
        Err(ValidationError::InvalidIdentifier(field))
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_digest(value: &str, field: &'static str) -> Result<(), ValidationError> {
    if valid_sha256(value) {
        Ok(())
    } else {
        Err(ValidationError::InvalidDigest(field))
    }
}

fn validate_artifact_set(
    artifacts: &[ArtifactRef],
    maximum: usize,
) -> Result<BTreeSet<ArtifactRole>, ValidationError> {
    if artifacts.len() > maximum {
        return Err(ValidationError::ResourceLimitExceeded);
    }

    let mut roles = BTreeSet::new();
    for artifact in artifacts {
        validate_digest(&artifact.sha256, "artifact")?;
        if !roles.insert(artifact.role) {
            return Err(ValidationError::DuplicateArtifactRole);
        }
    }
    Ok(roles)
}

fn is_connectome_family(family: ModelFamily) -> bool {
    matches!(
        family,
        ModelFamily::ConnectomeCircuit
            | ModelFamily::ConnectomeGroundedReflex
            | ModelFamily::ConnectomeHybrid
    )
}

fn validate_provenance(
    provenance: &ModelProvenance,
    family: ModelFamily,
    evidence_tier: EvidenceTier,
    evaluation_tier: EvaluationTier,
) -> Result<(), ValidationError> {
    validate_digest(&provenance.model_sha256, "model")?;
    validate_digest(&provenance.code_sha256, "code")?;
    validate_digest(&provenance.parameter_sha256, "parameters")?;
    validate_digest(&provenance.configuration_sha256, "configuration")?;

    let roles = validate_artifact_set(&provenance.artifacts, MAX_ARTIFACTS_PER_MODEL)?;
    let connectome_family = is_connectome_family(family);

    if connectome_family && evidence_tier < EvidenceTier::ConnectomeConstrained {
        return Err(ValidationError::ConnectomeFamilyBelowEvidenceTier);
    }
    if evidence_tier == EvidenceTier::ConnectomeConstrained && !connectome_family {
        return Err(ValidationError::ConnectomeTierRequiresConnectomeFamily);
    }

    let mut required_roles = BTreeSet::new();

    if connectome_family || evidence_tier == EvidenceTier::ConnectomeConstrained {
        required_roles.insert(ArtifactRole::Connectome);
        required_roles.insert(ArtifactRole::BiologicalSource);
        required_roles.insert(ArtifactRole::SensorMapping);
        required_roles.insert(ArtifactRole::EffectorMapping);
    }

    match evidence_tier {
        EvidenceTier::Synthetic | EvidenceTier::Phenomenological => {}
        EvidenceTier::ConnectomeConstrained => {}
        EvidenceTier::PerturbationGrounded => {
            required_roles.insert(ArtifactRole::BiologicalSource);
            required_roles.insert(ArtifactRole::MeasuredDataset);
        }
        EvidenceTier::ReplicatedBiological => {
            required_roles.insert(ArtifactRole::BiologicalSource);
            required_roles.insert(ArtifactRole::MeasuredDataset);
            required_roles.insert(ArtifactRole::IndependentReplication);
        }
    }

    if evaluation_tier == EvaluationTier::InSilicoEmbodied {
        required_roles.insert(ArtifactRole::SensorMapping);
        required_roles.insert(ArtifactRole::EffectorMapping);
    }

    for role in required_roles {
        if !roles.contains(&role) {
            return Err(ValidationError::MissingArtifactRole(role));
        }
    }

    Ok(())
}

fn validate_evaluation_artifacts(batch: &PredictionBatch) -> Result<(), ValidationError> {
    let roles = validate_artifact_set(&batch.evaluation_artifacts, MAX_EVALUATION_ARTIFACTS)?;

    let required_roles: &[ArtifactRole] = match batch.evaluation_tier {
        EvaluationTier::SyntheticHarness => &[],
        EvaluationTier::InSilicoEmbodied => &[
            ArtifactRole::BodyMorphology,
            ArtifactRole::PhysicsConfiguration,
        ],
        EvaluationTier::MeasuredIntervention => &[
            ArtifactRole::BiologicalSource,
            ArtifactRole::MeasuredDataset,
        ],
        EvaluationTier::IndependentReplication => &[
            ArtifactRole::BiologicalSource,
            ArtifactRole::MeasuredDataset,
            ArtifactRole::IndependentReplication,
        ],
    };

    for role in required_roles {
        if !roles.contains(role) {
            return Err(ValidationError::MissingArtifactRole(*role));
        }
    }
    Ok(())
}

/// Check that a covariance matrix is finite, symmetric and positive semidefinite.
/// A semidefinite matrix is permitted; screening adds a preregistered positive
/// regularizer before factorization.
fn validate_covariance(matrix: &[Vec<f64>]) -> Result<(), ValidationError> {
    let dimension = matrix.len();
    if dimension == 0 || matrix.iter().any(|row| row.len() != dimension) {
        return Err(ValidationError::DimensionMismatch);
    }

    for row in matrix {
        if row.iter().any(|value| !value.is_finite()) {
            return Err(ValidationError::NonFiniteValue);
        }
    }

    let scale = matrix
        .iter()
        .flat_map(|row| row.iter())
        .map(|value| value.abs())
        .fold(1.0e-300_f64, f64::max);
    let tolerance = scale * 1.0e-10;

    for row in 0..dimension {
        for column in 0..dimension {
            if (matrix[row][column] - matrix[column][row]).abs() > tolerance {
                return Err(ValidationError::InvalidCovariance);
            }
        }
    }

    // Semidefinite Cholesky check. A zero pivot is valid only if the residual
    // in that pivot column is also zero within the declared tolerance.
    let mut lower = vec![vec![0.0_f64; dimension]; dimension];
    for row in 0..dimension {
        for column in 0..=row {
            let mut residual = matrix[row][column];
            for inner in 0..column {
                residual -= lower[row][inner] * lower[column][inner];
            }

            if row == column {
                if residual < -tolerance {
                    return Err(ValidationError::InvalidCovariance);
                }
                lower[row][column] = if residual > tolerance {
                    residual.sqrt()
                } else {
                    0.0
                };
            } else if lower[column][column] > tolerance {
                lower[row][column] = residual / lower[column][column];
            } else if residual.abs() > tolerance {
                return Err(ValidationError::InvalidCovariance);
            }
        }
    }

    Ok(())
}

pub fn validate_prediction_batch(batch: &PredictionBatch) -> Result<(), ValidationError> {
    validate_identifier(&batch.protocol_id, "protocol_id")?;
    validate_identifier(&batch.intervention_id, "intervention_id")?;
    validate_identifier(&batch.event_log_id, "event_log_id")?;
    validate_digest(&batch.protocol_sha256, "protocol")?;
    validate_digest(&batch.split_sha256, "split")?;
    validate_digest(&batch.scenario_sha256, "scenario")?;
    validate_digest(&batch.feature_schema_sha256, "feature_schema")?;
    validate_digest(&batch.batch_sha256, "batch")?;
    validate_digest(&batch.commit_receipt_sha256, "commit_receipt")?;

    if batch.forecasts.len() < 2 {
        return Err(ValidationError::TooFewModels);
    }
    if batch.forecasts.len() > MAX_MODELS_PER_INTERVENTION {
        return Err(ValidationError::ResourceLimitExceeded);
    }
    if batch.commit_sequence == 0 {
        return Err(ValidationError::InvalidCommitSequence);
    }
    if batch.pairwise_observation_difference_covariance.len() > MAX_FEATURE_DIMENSION
        || batch.observation_covariance.len() > MAX_FEATURE_DIMENSION
    {
        return Err(ValidationError::ResourceLimitExceeded);
    }

    validate_evaluation_artifacts(batch)?;
    validate_covariance(&batch.pairwise_observation_difference_covariance)?;
    validate_covariance(&batch.observation_covariance)?;

    let dimension = batch.pairwise_observation_difference_covariance.len();
    if batch.observation_covariance.len() != dimension {
        return Err(ValidationError::DimensionMismatch);
    }
    let mut model_ids = BTreeSet::new();
    let mut canonical_feature_ids: Option<&[String]> = None;

    for forecast in &batch.forecasts {
        validate_identifier(&forecast.model_id, "model_id")?;
        validate_identifier(&forecast.intervention_id, "forecast_intervention_id")?;
        if forecast.intervention_id != batch.intervention_id {
            return Err(ValidationError::InterventionMismatch);
        }
        if !model_ids.insert(forecast.model_id.as_str()) {
            return Err(ValidationError::DuplicateModelId);
        }

        validate_digest(&forecast.raw_trajectory_sha256, "raw_trajectory")?;
        validate_digest(&forecast.feature_summary_sha256, "feature_summary")?;
        validate_digest(&forecast.forecast_sha256, "forecast")?;
        if forecast.commit_sequence == 0 {
            return Err(ValidationError::InvalidCommitSequence);
        }
        if forecast.commit_sequence > batch.commit_sequence {
            return Err(ValidationError::ForecastCommittedAfterBatch);
        }

        validate_provenance(
            &forecast.provenance,
            forecast.model_family,
            forecast.evidence_tier,
            batch.evaluation_tier,
        )?;

        if dimension > MAX_FEATURE_DIMENSION {
            return Err(ValidationError::ResourceLimitExceeded);
        }
        if forecast.feature_ids.len() != dimension || forecast.mean.len() != dimension {
            return Err(ValidationError::DimensionMismatch);
        }
        if forecast.covariance.len() != dimension {
            return Err(ValidationError::DimensionMismatch);
        }
        if forecast.mean.iter().any(|value| !value.is_finite()) {
            return Err(ValidationError::NonFiniteValue);
        }

        let mut feature_ids = BTreeSet::new();
        for feature_id in &forecast.feature_ids {
            validate_identifier(feature_id, "feature_id")?;
            if !feature_ids.insert(feature_id.as_str()) {
                return Err(ValidationError::DuplicateFeatureId);
            }
        }

        if let Some(canonical) = canonical_feature_ids {
            if canonical != forecast.feature_ids.as_slice() {
                return Err(ValidationError::FeatureSchemaMismatch);
            }
        } else {
            canonical_feature_ids = Some(forecast.feature_ids.as_slice());
        }

        validate_covariance(&forecast.covariance)?;
    }

    Ok(())
}

/// Verify that an independently anchored unblinding event occurred strictly
/// after the prediction batch was committed to the same append-only log.
///
/// Log signatures, inclusion proofs and digest computation belong to the
/// external evidence authority; this function checks only the supplied fields.
pub fn validate_unblinding_order(
    batch: &PredictionBatch,
    unblinding: &UnblindingReceipt,
) -> Result<(), ValidationError> {
    validate_prediction_batch(batch)?;
    validate_identifier(&unblinding.event_log_id, "unblinding_event_log_id")?;
    validate_digest(&unblinding.outcomes_sha256, "outcomes")?;
    validate_digest(&unblinding.receipt_sha256, "unblinding_receipt")?;
    if unblinding.sequence == 0 {
        return Err(ValidationError::InvalidCommitSequence);
    }

    if unblinding.event_log_id != batch.event_log_id {
        return Err(ValidationError::EventLogMismatch);
    }
    if unblinding.sequence <= batch.commit_sequence {
        return Err(ValidationError::CommitSequenceAfterUnblinding);
    }

    Ok(())
}

fn cholesky_positive_definite(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, ValidationError> {
    let dimension = matrix.len();
    let mut lower = vec![vec![0.0_f64; dimension]; dimension];

    for row in 0..dimension {
        for column in 0..=row {
            let mut residual = matrix[row][column];
            for inner in 0..column {
                residual -= lower[row][inner] * lower[column][inner];
            }

            if row == column {
                if !residual.is_finite() || residual <= 0.0 {
                    return Err(ValidationError::NonPositiveDefiniteCovariance);
                }
                lower[row][column] = residual.sqrt();
            } else {
                lower[row][column] = residual / lower[column][column];
            }
        }
    }

    Ok(lower)
}

fn log_determinant_from_cholesky(lower: &[Vec<f64>]) -> Result<f64, ValidationError> {
    let mut log_determinant = 0.0_f64;
    for index in 0..lower.len() {
        let diagonal = lower[index][index];
        if !diagonal.is_finite() || diagonal <= 0.0 {
            return Err(ValidationError::NonPositiveDefiniteCovariance);
        }
        log_determinant += 2.0 * diagonal.ln();
    }
    if log_determinant.is_finite() {
        Ok(log_determinant)
    } else {
        Err(ValidationError::NonFiniteValue)
    }
}

fn quadratic_form_from_cholesky(lower: &[Vec<f64>], delta: &[f64]) -> Result<f64, ValidationError> {
    let mut whitened = vec![0.0_f64; delta.len()];
    for row in 0..delta.len() {
        let mut residual = delta[row];
        for column in 0..row {
            residual -= lower[row][column] * whitened[column];
        }
        whitened[row] = residual / lower[row][row];
    }
    let value = whitened.iter().map(|item| item * item).sum::<f64>();
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ValidationError::NonFiniteValue)
    }
}

/// Bhattacharyya distance between two Gaussian predictive distributions after
/// adding the declared single-observation measurement covariance.
///
/// This is exact only under the multivariate-Gaussian approximation and
/// independent predictive errors. It is not EIG or posterior model probability.
fn pairwise_gaussian_bhattacharyya(
    left: &ModelForecast,
    right: &ModelForecast,
    observation_covariance: &[Vec<f64>],
    regularization: f64,
) -> Result<f64, ValidationError> {
    let dimension = left.mean.len();
    let mut left_covariance = vec![vec![0.0_f64; dimension]; dimension];
    let mut right_covariance = vec![vec![0.0_f64; dimension]; dimension];
    let mut average_covariance = vec![vec![0.0_f64; dimension]; dimension];

    for row in 0..dimension {
        for column in 0..dimension {
            left_covariance[row][column] =
                left.covariance[row][column] + observation_covariance[row][column];
            right_covariance[row][column] =
                right.covariance[row][column] + observation_covariance[row][column];
            average_covariance[row][column] =
                0.5 * (left_covariance[row][column] + right_covariance[row][column]);
        }
        left_covariance[row][row] += regularization;
        right_covariance[row][row] += regularization;
        average_covariance[row][row] += regularization;
    }

    let left_factor = cholesky_positive_definite(&left_covariance)?;
    let right_factor = cholesky_positive_definite(&right_covariance)?;
    let average_factor = cholesky_positive_definite(&average_covariance)?;

    let delta = left
        .mean
        .iter()
        .zip(&right.mean)
        .map(|(left_value, right_value)| left_value - right_value)
        .collect::<Vec<_>>();
    let mean_term = quadratic_form_from_cholesky(&average_factor, &delta)? / 8.0;

    let average_logdet = log_determinant_from_cholesky(&average_factor)?;
    let left_logdet = log_determinant_from_cholesky(&left_factor)?;
    let right_logdet = log_determinant_from_cholesky(&right_factor)?;
    let covariance_term = 0.5 * (average_logdet - 0.5 * (left_logdet + right_logdet));
    let distance = mean_term + covariance_term;

    if !distance.is_finite() {
        return Err(ValidationError::NonFiniteValue);
    }

    // Bhattacharyya distance is nonnegative analytically. Permit only
    // round-off-sized negative residue; reject materially negative results.
    let numerical_scale = average_logdet.abs()
        + 0.5 * (left_logdet.abs() + right_logdet.abs())
        + mean_term.abs()
        + 1.0;
    let roundoff_tolerance = 1.0e-12 * numerical_scale;
    if distance < -roundoff_tolerance {
        return Err(ValidationError::NegativeDistributionDistance);
    }
    Ok(distance.max(0.0))
}

fn pairwise_mean_d2(
    left: &ModelForecast,
    right: &ModelForecast,
    pairwise_observation_difference_covariance: &[Vec<f64>],
    regularization: f64,
) -> Result<f64, ValidationError> {
    let dimension = left.mean.len();
    let mut pooled = vec![vec![0.0_f64; dimension]; dimension];

    for row in 0..dimension {
        for column in 0..dimension {
            pooled[row][column] = left.covariance[row][column]
                + right.covariance[row][column]
                + pairwise_observation_difference_covariance[row][column];
        }
        pooled[row][row] += regularization;
    }

    let lower = cholesky_positive_definite(&pooled)?;

    // Solve L z = (mu_left - mu_right); D² is zᵀz.
    let mut whitened_delta = vec![0.0_f64; dimension];
    for row in 0..dimension {
        let mut residual = left.mean[row] - right.mean[row];
        for column in 0..row {
            residual -= lower[row][column] * whitened_delta[column];
        }
        whitened_delta[row] = residual / lower[row][row];
    }

    let d2 = whitened_delta
        .iter()
        .map(|value| value * value)
        .sum::<f64>();
    if d2.is_finite() {
        Ok(d2)
    } else {
        Err(ValidationError::NonFiniteValue)
    }
}

/// Compute pairwise noise-aware model separation for a single intervention.
///
/// This is a mean-only screening heuristic based on squared Mahalanobis
/// distance. Predictive and observation-difference covariances only whiten the
/// difference between predicted means; covariance-only model differences are
/// not detected. It assumes independent model-predictive errors; correlated
/// paired errors require an explicit cross-model covariance term. It is not
/// expected information gain, a likelihood-ratio test, a p-value, or a
/// posterior probability that any model is true.
pub fn screen_intervention(
    batch: &PredictionBatch,
    policy: DiscriminationPolicy,
) -> Result<DiscriminationResult, ValidationError> {
    validate_prediction_batch(batch)?;
    if !policy.minimum_pair_mean_d2.is_finite()
        || policy.minimum_pair_mean_d2 <= 0.0
        || !policy.minimum_gaussian_bhattacharyya_distance.is_finite()
        || policy.minimum_gaussian_bhattacharyya_distance <= 0.0
        || !policy.covariance_regularization.is_finite()
        || policy.covariance_regularization <= 0.0
    {
        return Err(ValidationError::InvalidPolicy);
    }

    let mut pairwise = Vec::new();
    for left_index in 0..batch.forecasts.len() {
        for right_index in (left_index + 1)..batch.forecasts.len() {
            let left = &batch.forecasts[left_index];
            let right = &batch.forecasts[right_index];
            let mean_d2 = pairwise_mean_d2(
                left,
                right,
                &batch.pairwise_observation_difference_covariance,
                policy.covariance_regularization,
            )?;
            let gaussian_bhattacharyya_distance = pairwise_gaussian_bhattacharyya(
                left,
                right,
                &batch.observation_covariance,
                policy.covariance_regularization,
            )?;
            pairwise.push(PairSeparation {
                model_a: left.model_id.clone(),
                model_b: right.model_id.clone(),
                mean_d2,
                mean_separated: mean_d2 >= policy.minimum_pair_mean_d2,
                gaussian_bhattacharyya_distance,
                gaussian_distributions_separated: gaussian_bhattacharyya_distance
                    >= policy.minimum_gaussian_bhattacharyya_distance,
            });
        }
    }

    let mean_separated_pair_count = pairwise.iter().filter(|pair| pair.mean_separated).count();
    let mean_unseparated_pair_count = pairwise.len() - mean_separated_pair_count;
    let minimum_pair_mean_d2 = pairwise
        .iter()
        .map(|pair| pair.mean_d2)
        .fold(f64::INFINITY, f64::min);

    let gaussian_separated_pair_count = pairwise
        .iter()
        .filter(|pair| pair.gaussian_distributions_separated)
        .count();
    let gaussian_unseparated_pair_count = pairwise.len() - gaussian_separated_pair_count;
    let minimum_pair_gaussian_bhattacharyya_distance = pairwise
        .iter()
        .map(|pair| pair.gaussian_bhattacharyya_distance)
        .fold(f64::INFINITY, f64::min);

    let status = if mean_unseparated_pair_count == 0 {
        DiscriminationStatus::AllPairsMeanSeparated
    } else if mean_separated_pair_count == 0 {
        DiscriminationStatus::NoPairsMeanSeparated
    } else {
        DiscriminationStatus::SomePairsMeanSeparated
    };
    let gaussian_status = if gaussian_unseparated_pair_count == 0 {
        GaussianDiscriminationStatus::AllPairsGaussianSeparated
    } else if gaussian_separated_pair_count == 0 {
        GaussianDiscriminationStatus::NoPairsGaussianSeparated
    } else {
        GaussianDiscriminationStatus::SomePairsGaussianSeparated
    };

    Ok(DiscriminationResult {
        status,
        minimum_pair_mean_d2,
        mean_separated_pair_count,
        mean_unseparated_pair_count,
        gaussian_status,
        minimum_pair_gaussian_bhattacharyya_distance,
        gaussian_separated_pair_count,
        gaussian_unseparated_pair_count,
        pairwise,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest() -> String {
        "a".repeat(64)
    }

    fn provenance() -> ModelProvenance {
        ModelProvenance {
            model_sha256: digest(),
            code_sha256: digest(),
            parameter_sha256: digest(),
            configuration_sha256: digest(),
            artifacts: Vec::new(),
        }
    }

    fn diagonal_covariance(dimension: usize, variance: f64) -> Vec<Vec<f64>> {
        let mut covariance = vec![vec![0.0_f64; dimension]; dimension];
        for index in 0..dimension {
            covariance[index][index] = variance;
        }
        covariance
    }

    fn forecast(model_id: &str, means: Vec<f64>, variance: f64) -> ModelForecast {
        let dimension = means.len();
        ModelForecast {
            model_id: model_id.to_string(),
            intervention_id: "intervention-a".to_string(),
            model_family: ModelFamily::SyntheticOracle,
            evidence_tier: EvidenceTier::Synthetic,
            feature_ids: (0..dimension)
                .map(|index| format!("feature-{index}"))
                .collect(),
            mean: means,
            covariance: diagonal_covariance(dimension, variance),
            raw_trajectory_sha256: digest(),
            feature_summary_sha256: digest(),
            forecast_sha256: digest(),
            commit_sequence: 10,
            provenance: provenance(),
        }
    }

    fn batch(forecasts: Vec<ModelForecast>) -> PredictionBatch {
        let dimension = forecasts[0].mean.len();
        PredictionBatch {
            protocol_id: "protocol-v1".to_string(),
            intervention_id: "intervention-a".to_string(),
            protocol_sha256: digest(),
            split_sha256: digest(),
            scenario_sha256: digest(),
            feature_schema_sha256: digest(),
            batch_sha256: digest(),
            commit_receipt_sha256: digest(),
            event_log_id: "evidence-log-1".to_string(),
            commit_sequence: 20,
            evaluation_tier: EvaluationTier::SyntheticHarness,
            evaluation_artifacts: Vec::new(),
            observation_covariance: diagonal_covariance(dimension, 0.0),
            pairwise_observation_difference_covariance: diagonal_covariance(dimension, 0.0),
            forecasts,
        }
    }

    fn policy(minimum_pair_mean_d2: f64) -> DiscriminationPolicy {
        DiscriminationPolicy {
            minimum_pair_mean_d2,
            minimum_gaussian_bhattacharyya_distance: 0.05,
            covariance_regularization: 1.0,
        }
    }

    fn add_sensor_effector_artifacts(forecast: &mut ModelForecast) {
        forecast.provenance.artifacts = vec![
            ArtifactRef {
                role: ArtifactRole::SensorMapping,
                sha256: digest(),
            },
            ArtifactRef {
                role: ArtifactRole::EffectorMapping,
                sha256: digest(),
            },
        ];
    }

    fn add_connectome_artifacts(forecast: &mut ModelForecast) {
        forecast.model_family = ModelFamily::ConnectomeCircuit;
        forecast.provenance.artifacts = vec![
            ArtifactRef {
                role: ArtifactRole::Connectome,
                sha256: digest(),
            },
            ArtifactRef {
                role: ArtifactRole::BiologicalSource,
                sha256: digest(),
            },
            ArtifactRef {
                role: ArtifactRole::SensorMapping,
                sha256: digest(),
            },
            ArtifactRef {
                role: ArtifactRole::EffectorMapping,
                sha256: digest(),
            },
        ];
    }

    #[test]
    fn accepts_a_well_formed_blind_prediction_batch() {
        let candidate = batch(vec![
            forecast("local-oscillator", vec![0.0], 0.1),
            forecast("coupled-circuit", vec![2.0], 0.1),
        ]);
        assert_eq!(validate_prediction_batch(&candidate), Ok(()));
    }

    #[test]
    fn rejects_malformed_digest_references() {
        let mut candidate = batch(vec![
            forecast("model-a", vec![0.0], 0.1),
            forecast("model-b", vec![1.0], 0.1),
        ]);
        candidate.forecasts[0].raw_trajectory_sha256 = "not-a-digest".to_string();
        assert_eq!(
            validate_prediction_batch(&candidate),
            Err(ValidationError::InvalidDigest("raw_trajectory"))
        );
    }

    #[test]
    fn rejects_duplicate_model_ids() {
        let candidate = batch(vec![
            forecast("same-model", vec![0.0], 0.1),
            forecast("same-model", vec![1.0], 0.1),
        ]);
        assert_eq!(
            validate_prediction_batch(&candidate),
            Err(ValidationError::DuplicateModelId)
        );
    }

    #[test]
    fn rejects_forecast_for_a_different_intervention() {
        let mut second = forecast("model-b", vec![1.0], 0.1);
        second.intervention_id = "different-intervention".to_string();
        let candidate = batch(vec![forecast("model-a", vec![0.0], 0.1), second]);
        assert_eq!(
            validate_prediction_batch(&candidate),
            Err(ValidationError::InterventionMismatch)
        );
    }

    #[test]
    fn rejects_feature_schema_reordering() {
        let mut second = forecast("model-b", vec![1.0, 2.0], 0.1);
        second.feature_ids.reverse();
        let candidate = batch(vec![forecast("model-a", vec![0.0, 1.0], 0.1), second]);
        assert_eq!(
            validate_prediction_batch(&candidate),
            Err(ValidationError::FeatureSchemaMismatch)
        );
    }

    #[test]
    fn rejects_non_positive_semidefinite_covariance() {
        let mut first = forecast("model-a", vec![0.0, 0.0], 0.1);
        first.covariance = vec![vec![1.0, 2.0], vec![2.0, 1.0]];
        let candidate = batch(vec![first, forecast("model-b", vec![1.0, 1.0], 0.1)]);
        assert_eq!(
            validate_prediction_batch(&candidate),
            Err(ValidationError::InvalidCovariance)
        );
    }

    #[test]
    fn requires_connectome_and_mapping_artifacts_for_connectome_tier() {
        let mut first = forecast("connectome-model", vec![0.0], 0.1);
        first.model_family = ModelFamily::ConnectomeCircuit;
        first.evidence_tier = EvidenceTier::ConnectomeConstrained;
        assert_eq!(
            validate_prediction_batch(&batch(vec![
                first,
                forecast("phenomenological-model", vec![1.0], 0.1),
            ])),
            Err(ValidationError::MissingArtifactRole(
                ArtifactRole::Connectome
            ))
        );
    }

    #[test]
    fn accepts_declared_connectome_provenance() {
        let mut first = forecast("connectome-model", vec![0.0], 0.1);
        first.model_family = ModelFamily::ConnectomeCircuit;
        first.evidence_tier = EvidenceTier::ConnectomeConstrained;
        add_connectome_artifacts(&mut first);
        let candidate = batch(vec![
            first,
            forecast("phenomenological-model", vec![1.0], 0.1),
        ]);
        assert_eq!(validate_prediction_batch(&candidate), Ok(()));
    }

    #[test]
    fn embodied_non_connectome_baselines_do_not_require_connectome_artifacts() {
        let mut candidate = batch(vec![
            forecast("rule-based", vec![0.0], 0.1),
            forecast("learned-policy", vec![1.0], 0.1),
        ]);
        candidate.evaluation_tier = EvaluationTier::InSilicoEmbodied;
        candidate.evaluation_artifacts = vec![
            ArtifactRef {
                role: ArtifactRole::BodyMorphology,
                sha256: digest(),
            },
            ArtifactRef {
                role: ArtifactRole::PhysicsConfiguration,
                sha256: digest(),
            },
        ];
        for forecast in &mut candidate.forecasts {
            forecast.model_family = ModelFamily::RuleBasedController;
            forecast.evidence_tier = EvidenceTier::Phenomenological;
            add_sensor_effector_artifacts(forecast);
        }
        assert_eq!(validate_prediction_batch(&candidate), Ok(()));
    }

    #[test]
    fn embodied_evaluation_requires_body_and_physics_artifacts() {
        let mut candidate = batch(vec![
            forecast("rule-based", vec![0.0], 0.1),
            forecast("learned-policy", vec![1.0], 0.1),
        ]);
        candidate.evaluation_tier = EvaluationTier::InSilicoEmbodied;
        candidate.evaluation_artifacts = vec![ArtifactRef {
            role: ArtifactRole::BodyMorphology,
            sha256: digest(),
        }];
        for forecast in &mut candidate.forecasts {
            forecast.model_family = ModelFamily::RuleBasedController;
            forecast.evidence_tier = EvidenceTier::Phenomenological;
            add_sensor_effector_artifacts(forecast);
        }
        assert_eq!(
            validate_prediction_batch(&candidate),
            Err(ValidationError::MissingArtifactRole(
                ArtifactRole::PhysicsConfiguration
            ))
        );
    }

    #[test]
    fn connectome_family_cannot_claim_only_phenomenological_evidence() {
        let mut first = forecast("connectome-model", vec![0.0], 0.1);
        first.model_family = ModelFamily::ConnectomeCircuit;
        assert_eq!(
            validate_prediction_batch(&batch(vec![
                first,
                forecast("phenomenological-model", vec![1.0], 0.1),
            ])),
            Err(ValidationError::ConnectomeFamilyBelowEvidenceTier)
        );
    }

    #[test]
    fn connectome_evidence_cannot_be_claimed_by_rule_based_family() {
        let mut first = forecast("rule-based", vec![0.0], 0.1);
        first.model_family = ModelFamily::RuleBasedController;
        first.evidence_tier = EvidenceTier::ConnectomeConstrained;
        assert_eq!(
            validate_prediction_batch(&batch(vec![
                first,
                forecast("phenomenological-model", vec![1.0], 0.1),
            ])),
            Err(ValidationError::ConnectomeTierRequiresConnectomeFamily)
        );
    }

    #[test]
    fn measured_evaluation_requires_source_and_dataset_at_batch_level() {
        let mut candidate = batch(vec![
            forecast("rule-based", vec![0.0], 0.1),
            forecast("learned-policy", vec![1.0], 0.1),
        ]);
        candidate.evaluation_tier = EvaluationTier::MeasuredIntervention;
        candidate.evaluation_artifacts = vec![ArtifactRef {
            role: ArtifactRole::BiologicalSource,
            sha256: digest(),
        }];
        assert_eq!(
            validate_prediction_batch(&candidate),
            Err(ValidationError::MissingArtifactRole(
                ArtifactRole::MeasuredDataset
            ))
        );
    }

    #[test]
    fn perturbation_grounded_non_connectome_model_does_not_require_connectome() {
        let mut first = forecast("learned-policy", vec![0.0], 0.1);
        first.model_family = ModelFamily::LearnedController;
        first.evidence_tier = EvidenceTier::PerturbationGrounded;
        first.provenance.artifacts = vec![
            ArtifactRef {
                role: ArtifactRole::BiologicalSource,
                sha256: digest(),
            },
            ArtifactRef {
                role: ArtifactRole::MeasuredDataset,
                sha256: digest(),
            },
        ];
        let mut candidate = batch(vec![
            first,
            forecast("phenomenological-model", vec![1.0], 0.1),
        ]);
        candidate.evaluation_tier = EvaluationTier::MeasuredIntervention;
        candidate.evaluation_artifacts = vec![
            ArtifactRef {
                role: ArtifactRole::BiologicalSource,
                sha256: digest(),
            },
            ArtifactRef {
                role: ArtifactRole::MeasuredDataset,
                sha256: digest(),
            },
        ];
        assert_eq!(validate_prediction_batch(&candidate), Ok(()));
    }

    #[test]
    fn independent_replication_evaluation_requires_replication_artifact() {
        let mut candidate = batch(vec![
            forecast("model-a", vec![0.0], 0.1),
            forecast("model-b", vec![1.0], 0.1),
        ]);
        candidate.evaluation_tier = EvaluationTier::IndependentReplication;
        candidate.evaluation_artifacts = vec![
            ArtifactRef {
                role: ArtifactRole::BiologicalSource,
                sha256: digest(),
            },
            ArtifactRef {
                role: ArtifactRole::MeasuredDataset,
                sha256: digest(),
            },
        ];
        assert_eq!(
            validate_prediction_batch(&candidate),
            Err(ValidationError::MissingArtifactRole(
                ArtifactRole::IndependentReplication
            ))
        );
    }

    #[test]
    fn rejects_evaluation_artifact_count_above_the_resource_limit() {
        let mut candidate = batch(vec![
            forecast("model-a", vec![0.0], 0.1),
            forecast("model-b", vec![1.0], 0.1),
        ]);
        candidate.evaluation_artifacts = (0..=MAX_EVALUATION_ARTIFACTS)
            .map(|index| ArtifactRef {
                role: ArtifactRole::Connectome,
                sha256: format!("{:064x}", index + 1),
            })
            .collect();
        assert_eq!(
            validate_prediction_batch(&candidate),
            Err(ValidationError::ResourceLimitExceeded)
        );
    }

    #[test]
    fn rejects_unblinding_before_prediction_commit() {
        let candidate = batch(vec![
            forecast("model-a", vec![0.0], 0.1),
            forecast("model-b", vec![1.0], 0.1),
        ]);
        let receipt = UnblindingReceipt {
            event_log_id: "evidence-log-1".to_string(),
            sequence: 20,
            outcomes_sha256: digest(),
            receipt_sha256: digest(),
        };
        assert_eq!(
            validate_unblinding_order(&candidate, &receipt),
            Err(ValidationError::CommitSequenceAfterUnblinding)
        );
    }

    #[test]
    fn accepts_unblinding_strictly_after_prediction_commit() {
        let candidate = batch(vec![
            forecast("model-a", vec![0.0], 0.1),
            forecast("model-b", vec![1.0], 0.1),
        ]);
        let receipt = UnblindingReceipt {
            event_log_id: "evidence-log-1".to_string(),
            sequence: 21,
            outcomes_sha256: digest(),
            receipt_sha256: digest(),
        };
        assert_eq!(validate_unblinding_order(&candidate, &receipt), Ok(()));
    }

    #[test]
    fn rejects_unblinding_from_another_evidence_log() {
        let candidate = batch(vec![
            forecast("model-a", vec![0.0], 0.1),
            forecast("model-b", vec![1.0], 0.1),
        ]);
        let receipt = UnblindingReceipt {
            event_log_id: "different-log".to_string(),
            sequence: 21,
            outcomes_sha256: digest(),
            receipt_sha256: digest(),
        };
        assert_eq!(
            validate_unblinding_order(&candidate, &receipt),
            Err(ValidationError::EventLogMismatch)
        );
    }

    #[test]
    fn computes_known_answer_squared_mahalanobis_distance() {
        let candidate = batch(vec![
            forecast("model-a", vec![0.0], 0.0),
            forecast("model-b", vec![2.0], 0.0),
        ]);
        let result = screen_intervention(&candidate, policy(1.0)).expect("screen");
        assert_eq!(result.status, DiscriminationStatus::AllPairsMeanSeparated);
        assert!((result.pairwise[0].mean_d2 - 4.0).abs() < 1.0e-12);
    }

    #[test]
    fn identical_negative_control_predictions_have_no_mean_separation() {
        let candidate = batch(vec![
            forecast("true-connectome", vec![0.0, 1.0], 0.1),
            forecast("rewired-connectome", vec![0.0, 1.0], 0.1),
        ]);
        let result = screen_intervention(&candidate, policy(1.0)).expect("screen");
        assert_eq!(result.status, DiscriminationStatus::NoPairsMeanSeparated);
        assert_eq!(result.minimum_pair_mean_d2, 0.0);
    }

    #[test]
    fn reports_partial_mean_separability_instead_of_forcing_a_winner() {
        let candidate = batch(vec![
            forecast("independent-oscillators", vec![0.0], 0.0),
            forecast("coupled-circuit", vec![10.0], 0.0),
            forecast("connectome-hybrid", vec![10.0], 0.0),
        ]);
        let result = screen_intervention(&candidate, policy(5.0)).expect("screen");
        assert_eq!(result.status, DiscriminationStatus::SomePairsMeanSeparated);
        assert_eq!(result.mean_separated_pair_count, 2);
        assert_eq!(result.mean_unseparated_pair_count, 1);
    }

    #[test]
    fn separates_all_pairs_in_a_synthetic_known_answer_set() {
        let candidate = batch(vec![
            forecast("model-a", vec![0.0], 0.0),
            forecast("model-b", vec![10.0], 0.0),
            forecast("model-c", vec![20.0], 0.0),
        ]);
        let result = screen_intervention(&candidate, policy(5.0)).expect("screen");
        assert_eq!(result.status, DiscriminationStatus::AllPairsMeanSeparated);
        assert_eq!(result.mean_separated_pair_count, 3);
        assert_eq!(result.mean_unseparated_pair_count, 0);
    }

    #[test]
    fn covariance_only_differences_are_not_mean_separation() {
        let candidate = batch(vec![
            forecast("narrow-model", vec![1.0], 0.01),
            forecast("wide-model", vec![1.0], 100.0),
        ]);
        let result = screen_intervention(&candidate, policy(0.05)).expect("screen");
        assert_eq!(result.status, DiscriminationStatus::NoPairsMeanSeparated);
        assert_eq!(result.minimum_pair_mean_d2, 0.0);
        assert_eq!(
            result.gaussian_status,
            GaussianDiscriminationStatus::AllPairsGaussianSeparated
        );
        assert!(result.minimum_pair_gaussian_bhattacharyya_distance > 0.05);
    }

    #[test]
    fn predictive_uncertainty_reduces_apparent_separation() {
        let precise = batch(vec![
            forecast("model-a", vec![0.0], 0.0),
            forecast("model-b", vec![2.0], 0.0),
        ]);
        let uncertain = batch(vec![
            forecast("model-a", vec![0.0], 100.0),
            forecast("model-b", vec![2.0], 100.0),
        ]);
        let precise_result = screen_intervention(&precise, policy(0.05)).expect("precise");
        let uncertain_result = screen_intervention(&uncertain, policy(0.05)).expect("uncertain");
        assert!(precise_result.minimum_pair_mean_d2 > uncertain_result.minimum_pair_mean_d2);
        assert_eq!(
            precise_result.status,
            DiscriminationStatus::AllPairsMeanSeparated
        );
        assert_eq!(
            uncertain_result.status,
            DiscriminationStatus::NoPairsMeanSeparated
        );
        assert_eq!(
            precise_result.gaussian_status,
            GaussianDiscriminationStatus::AllPairsGaussianSeparated
        );
        assert_eq!(
            uncertain_result.gaussian_status,
            GaussianDiscriminationStatus::NoPairsGaussianSeparated
        );
    }

    #[test]
    fn gaussian_bhattacharyya_distance_has_a_one_dimensional_known_answer() {
        let candidate = batch(vec![
            forecast("model-a", vec![0.0], 0.0),
            forecast("model-b", vec![2.0], 0.0),
        ]);
        let result = screen_intervention(&candidate, policy(1.0)).expect("screen");
        assert!((result.pairwise[0].gaussian_bhattacharyya_distance - 0.5).abs() < 1.0e-12);
    }

    #[test]
    fn rejects_non_finite_predictions() {
        let candidate = batch(vec![
            forecast("model-a", vec![0.0], 0.1),
            forecast("model-b", vec![f64::NAN], 0.1),
        ]);
        assert_eq!(
            validate_prediction_batch(&candidate),
            Err(ValidationError::NonFiniteValue)
        );
    }

    #[test]
    fn rejects_forecast_committed_after_batch() {
        let mut second = forecast("model-b", vec![1.0], 0.1);
        second.commit_sequence = 21;
        let candidate = batch(vec![forecast("model-a", vec![0.0], 0.1), second]);
        assert_eq!(
            validate_prediction_batch(&candidate),
            Err(ValidationError::ForecastCommittedAfterBatch)
        );
    }

    #[test]
    fn rejects_zero_batch_commit_sequence() {
        let mut candidate = batch(vec![
            forecast("model-a", vec![0.0], 0.1),
            forecast("model-b", vec![1.0], 0.1),
        ]);
        candidate.commit_sequence = 0;
        assert_eq!(
            validate_prediction_batch(&candidate),
            Err(ValidationError::InvalidCommitSequence)
        );
    }

    #[test]
    fn rejects_duplicate_artifact_roles() {
        let mut candidate = forecast("connectome-model", vec![0.0], 0.1);
        candidate.evidence_tier = EvidenceTier::ConnectomeConstrained;
        add_connectome_artifacts(&mut candidate);
        let duplicate_role = candidate.provenance.artifacts[0].clone();
        candidate.provenance.artifacts.push(duplicate_role);
        assert_eq!(
            validate_prediction_batch(&batch(vec![
                candidate,
                forecast("phenomenological-model", vec![1.0], 0.1),
            ])),
            Err(ValidationError::DuplicateArtifactRole)
        );
    }

    #[test]
    fn rejects_model_count_above_the_resource_limit() {
        let forecasts = (0..=MAX_MODELS_PER_INTERVENTION)
            .map(|index| forecast(&format!("model-{index}"), vec![index as f64], 0.1))
            .collect();
        assert_eq!(
            validate_prediction_batch(&batch(forecasts)),
            Err(ValidationError::ResourceLimitExceeded)
        );
    }

    #[test]
    fn rejects_feature_dimension_above_the_resource_limit() {
        let dimension = MAX_FEATURE_DIMENSION + 1;
        let forecasts = vec![
            forecast("model-a", vec![0.0; dimension], 0.1),
            forecast("model-b", vec![1.0; dimension], 0.1),
        ];
        assert_eq!(
            validate_prediction_batch(&batch(forecasts)),
            Err(ValidationError::ResourceLimitExceeded)
        );
    }

    #[test]
    fn rejects_model_provenance_artifact_count_above_the_resource_limit() {
        let mut first = forecast("model-a", vec![0.0], 0.1);
        first.provenance.artifacts = (0..=MAX_ARTIFACTS_PER_MODEL)
            .map(|index| ArtifactRef {
                role: match index % 8 {
                    0 => ArtifactRole::Connectome,
                    1 => ArtifactRole::BiologicalSource,
                    2 => ArtifactRole::SensorMapping,
                    3 => ArtifactRole::EffectorMapping,
                    4 => ArtifactRole::BodyMorphology,
                    5 => ArtifactRole::PhysicsConfiguration,
                    6 => ArtifactRole::MeasuredDataset,
                    _ => ArtifactRole::IndependentReplication,
                },
                sha256: format!("{:064x}", index + 1),
            })
            .collect();
        assert_eq!(
            validate_prediction_batch(&batch(vec![first, forecast("model-b", vec![1.0], 0.1),])),
            Err(ValidationError::ResourceLimitExceeded)
        );
    }

    #[test]
    fn gaussian_bhattacharyya_distance_is_symmetric_in_model_order() {
        let forward = batch(vec![
            forecast("model-a", vec![0.5, 1.0], 0.2),
            forecast("model-b", vec![2.0, -1.0], 1.5),
        ]);
        let reverse = batch(vec![
            forecast("model-b", vec![2.0, -1.0], 1.5),
            forecast("model-a", vec![0.5, 1.0], 0.2),
        ]);
        let forward_result = screen_intervention(&forward, policy(0.05)).expect("forward");
        let reverse_result = screen_intervention(&reverse, policy(0.05)).expect("reverse");
        let forward_distance = forward_result.pairwise[0].gaussian_bhattacharyya_distance;
        let reverse_distance = reverse_result.pairwise[0].gaussian_bhattacharyya_distance;
        assert!((forward_distance - reverse_distance).abs() < 1.0e-12);
    }

    #[test]
    fn rejects_invalid_discrimination_policy() {
        let candidate = batch(vec![
            forecast("model-a", vec![0.0], 0.1),
            forecast("model-b", vec![1.0], 0.1),
        ]);
        let invalid = DiscriminationPolicy {
            minimum_pair_mean_d2: 1.0,
            minimum_gaussian_bhattacharyya_distance: 0.05,
            covariance_regularization: 0.0,
        };
        assert_eq!(
            screen_intervention(&candidate, invalid),
            Err(ValidationError::InvalidPolicy)
        );
    }
}
