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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceTier {
    Synthetic,
    Phenomenological,
    ConnectomeConstrained,
    InSilicoEmbodied,
    PerturbationGrounded,
    ReplicatedBiological,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Observation/measurement covariance for the declared feature vector.
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
    TooFewModels,
    DuplicateModelId,
    InterventionMismatch,
    FeatureSchemaMismatch,
    DuplicateFeatureId,
    DimensionMismatch,
    NonFiniteValue,
    InvalidCovariance,
    NonPositiveDefinitePooledCovariance,
    InvalidPolicy,
    ForecastCommittedAfterBatch,
    CommitSequenceAfterUnblinding,
    EventLogMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiscriminationPolicy {
    /// Minimum pairwise squared Mahalanobis separation required to call a pair
    /// resolved for this benchmark. Freeze it before unblinding.
    pub minimum_pair_d2: f64,
    /// Fixed diagonal regularizer for the pooled covariance.
    pub covariance_regularization: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscriminationStatus {
    /// Every candidate model pair exceeds the preregistered resolution threshold.
    FullySeparated,
    /// Some pairs separate, but at least one pair remains unresolved.
    PartiallySeparated,
    /// No candidate pair exceeds the preregistered resolution threshold.
    Unresolved,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PairSeparation {
    pub model_a: String,
    pub model_b: String,
    pub d2: f64,
    pub resolved: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscriminationResult {
    pub status: DiscriminationStatus,
    pub minimum_pair_d2: f64,
    pub resolved_pair_count: usize,
    pub unresolved_pair_count: usize,
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

fn has_role(provenance: &ModelProvenance, role: ArtifactRole) -> bool {
    provenance.artifacts.iter().any(|artifact| artifact.role == role)
}

fn validate_provenance(
    provenance: &ModelProvenance,
    tier: EvidenceTier,
) -> Result<(), ValidationError> {
    validate_digest(&provenance.model_sha256, "model")?;
    validate_digest(&provenance.code_sha256, "code")?;
    validate_digest(&provenance.parameter_sha256, "parameters")?;
    validate_digest(&provenance.configuration_sha256, "configuration")?;

    for artifact in &provenance.artifacts {
        validate_digest(&artifact.sha256, "artifact")?;
    }

    let required_roles: &[ArtifactRole] = match tier {
        EvidenceTier::Synthetic | EvidenceTier::Phenomenological => &[],
        EvidenceTier::ConnectomeConstrained => &[
            ArtifactRole::Connectome,
            ArtifactRole::BiologicalSource,
            ArtifactRole::SensorMapping,
            ArtifactRole::EffectorMapping,
        ],
        EvidenceTier::InSilicoEmbodied => &[
            ArtifactRole::Connectome,
            ArtifactRole::BiologicalSource,
            ArtifactRole::SensorMapping,
            ArtifactRole::EffectorMapping,
            ArtifactRole::BodyMorphology,
            ArtifactRole::PhysicsConfiguration,
        ],
        EvidenceTier::PerturbationGrounded => &[
            ArtifactRole::Connectome,
            ArtifactRole::BiologicalSource,
            ArtifactRole::SensorMapping,
            ArtifactRole::EffectorMapping,
            ArtifactRole::BodyMorphology,
            ArtifactRole::PhysicsConfiguration,
            ArtifactRole::MeasuredDataset,
        ],
        EvidenceTier::ReplicatedBiological => &[
            ArtifactRole::Connectome,
            ArtifactRole::BiologicalSource,
            ArtifactRole::SensorMapping,
            ArtifactRole::EffectorMapping,
            ArtifactRole::BodyMorphology,
            ArtifactRole::PhysicsConfiguration,
            ArtifactRole::MeasuredDataset,
            ArtifactRole::IndependentReplication,
        ],
    };

    for role in required_roles {
        if !has_role(provenance, *role) {
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

    validate_covariance(&batch.pairwise_observation_difference_covariance)?;

    let dimension = batch.pairwise_observation_difference_covariance.len();
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
        if forecast.commit_sequence > batch.commit_sequence {
            return Err(ValidationError::ForecastCommittedAfterBatch);
        }

        validate_provenance(&forecast.provenance, forecast.evidence_tier)?;

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

    if unblinding.event_log_id != batch.event_log_id {
        return Err(ValidationError::EventLogMismatch);
    }
    if unblinding.sequence <= batch.commit_sequence {
        return Err(ValidationError::CommitSequenceAfterUnblinding);
    }

    Ok(())
}

fn cholesky_positive_definite(
    matrix: &[Vec<f64>],
) -> Result<Vec<Vec<f64>>, ValidationError> {
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
                    return Err(ValidationError::NonPositiveDefinitePooledCovariance);
                }
                lower[row][column] = residual.sqrt();
            } else {
                lower[row][column] = residual / lower[column][column];
            }
        }
    }

    Ok(lower)
}

fn pairwise_d2(
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

    let d2 = whitened_delta.iter().map(|value| value * value).sum::<f64>();
    if d2.is_finite() {
        Ok(d2)
    } else {
        Err(ValidationError::NonFiniteValue)
    }
}

/// Compute pairwise noise-aware model separation for a single intervention.
///
/// This is a screening heuristic based on squared Mahalanobis distance using
/// the sum of both predictive covariances and the covariance of observation
/// error in their difference. It assumes independent model-predictive errors;
/// correlated paired errors require an explicit cross-model covariance term.
/// It is not expected information gain, a likelihood-ratio test, a p-value, or
/// a posterior probability that any model is true.
pub fn screen_intervention(
    batch: &PredictionBatch,
    policy: DiscriminationPolicy,
) -> Result<DiscriminationResult, ValidationError> {
    validate_prediction_batch(batch)?;
    if !policy.minimum_pair_d2.is_finite()
        || policy.minimum_pair_d2 <= 0.0
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
            let d2 = pairwise_d2(
                left,
                right,
                &batch.pairwise_observation_difference_covariance,
                policy.covariance_regularization,
            )?;
            pairwise.push(PairSeparation {
                model_a: left.model_id.clone(),
                model_b: right.model_id.clone(),
                d2,
                resolved: d2 >= policy.minimum_pair_d2,
            });
        }
    }

    let resolved_pair_count = pairwise.iter().filter(|pair| pair.resolved).count();
    let unresolved_pair_count = pairwise.len() - resolved_pair_count;
    let minimum_pair_d2 = pairwise
        .iter()
        .map(|pair| pair.d2)
        .fold(f64::INFINITY, f64::min);

    let status = if unresolved_pair_count == 0 {
        DiscriminationStatus::FullySeparated
    } else if resolved_pair_count == 0 {
        DiscriminationStatus::Unresolved
    } else {
        DiscriminationStatus::PartiallySeparated
    };

    Ok(DiscriminationResult {
        status,
        minimum_pair_d2,
        resolved_pair_count,
        unresolved_pair_count,
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
            evidence_tier: EvidenceTier::Synthetic,
            feature_ids: (0..dimension).map(|index| format!("feature-{index}")).collect(),
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
            pairwise_observation_difference_covariance: diagonal_covariance(dimension, 0.0),
            forecasts,
        }
    }

    fn policy(minimum_pair_d2: f64) -> DiscriminationPolicy {
        DiscriminationPolicy {
            minimum_pair_d2,
            covariance_regularization: 1.0,
        }
    }

    fn add_connectome_artifacts(forecast: &mut ModelForecast) {
        forecast.provenance.artifacts = vec![
            ArtifactRef { role: ArtifactRole::Connectome, sha256: digest() },
            ArtifactRef { role: ArtifactRole::BiologicalSource, sha256: digest() },
            ArtifactRef { role: ArtifactRole::SensorMapping, sha256: digest() },
            ArtifactRef { role: ArtifactRole::EffectorMapping, sha256: digest() },
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
        first.evidence_tier = EvidenceTier::ConnectomeConstrained;
        assert_eq!(
            validate_prediction_batch(&batch(vec![
                first,
                forecast("phenomenological-model", vec![1.0], 0.1),
            ])),
            Err(ValidationError::MissingArtifactRole(ArtifactRole::Connectome))
        );
    }

    #[test]
    fn accepts_declared_connectome_provenance() {
        let mut first = forecast("connectome-model", vec![0.0], 0.1);
        first.evidence_tier = EvidenceTier::ConnectomeConstrained;
        add_connectome_artifacts(&mut first);
        let candidate = batch(vec![first, forecast("phenomenological-model", vec![1.0], 0.1)]);
        assert_eq!(validate_prediction_batch(&candidate), Ok(()));
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
        assert_eq!(result.status, DiscriminationStatus::FullySeparated);
        assert!((result.pairwise[0].d2 - 4.0).abs() < 1.0e-12);
    }

    #[test]
    fn identical_negative_control_predictions_are_unresolved() {
        let candidate = batch(vec![
            forecast("true-connectome", vec![0.0, 1.0], 0.1),
            forecast("rewired-connectome", vec![0.0, 1.0], 0.1),
        ]);
        let result = screen_intervention(&candidate, policy(1.0)).expect("screen");
        assert_eq!(result.status, DiscriminationStatus::Unresolved);
        assert_eq!(result.minimum_pair_d2, 0.0);
    }

    #[test]
    fn reports_partial_separability_instead_of_forcing_a_winner() {
        let candidate = batch(vec![
            forecast("independent-oscillators", vec![0.0], 0.0),
            forecast("coupled-circuit", vec![10.0], 0.0),
            forecast("connectome-hybrid", vec![10.0], 0.0),
        ]);
        let result = screen_intervention(&candidate, policy(5.0)).expect("screen");
        assert_eq!(result.status, DiscriminationStatus::PartiallySeparated);
        assert_eq!(result.resolved_pair_count, 2);
        assert_eq!(result.unresolved_pair_count, 1);
    }

    #[test]
    fn fully_separates_a_synthetic_known_answer_set() {
        let candidate = batch(vec![
            forecast("model-a", vec![0.0], 0.0),
            forecast("model-b", vec![10.0], 0.0),
            forecast("model-c", vec![20.0], 0.0),
        ]);
        let result = screen_intervention(&candidate, policy(5.0)).expect("screen");
        assert_eq!(result.status, DiscriminationStatus::FullySeparated);
        assert_eq!(result.resolved_pair_count, 3);
        assert_eq!(result.unresolved_pair_count, 0);
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
        assert!(precise_result.minimum_pair_d2 > uncertain_result.minimum_pair_d2);
        assert_eq!(precise_result.status, DiscriminationStatus::FullySeparated);
        assert_eq!(uncertain_result.status, DiscriminationStatus::Unresolved);
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
    fn rejects_invalid_discrimination_policy() {
        let candidate = batch(vec![
            forecast("model-a", vec![0.0], 0.1),
            forecast("model-b", vec![1.0], 0.1),
        ]);
        let invalid = DiscriminationPolicy {
            minimum_pair_d2: 1.0,
            covariance_regularization: 0.0,
        };
        assert_eq!(
            screen_intervention(&candidate, invalid),
            Err(ValidationError::InvalidPolicy)
        );
    }
}
