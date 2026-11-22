//! HBR-PGD anchor-channel sparse residual self-calibration (SRSC).
//!
//! This module implements the paper's pre-optimization group gate:
//! - residual centre = median(initial geometric residuals);
//! - residual spread = median absolute deviation (MAD);
//! - group correction prior = median(packet correction priors);
//! - anomaly score = beta1*abs(center) + beta2*spread + beta3*abs(median prior);
//! - soft gate = sigmoid(gamma*(score - threshold)).
//!
//! The resulting gate is frozen before trajectory/bias optimization. It is not
//! a learned/optimized state and does not certify that a link is truly NLOS.
//! Group IDs and source commitments are caller-supplied and must be externally
//! qualified. No evaluator truth or NLOS labels are accepted here.

use std::collections::BTreeSet;
use std::fmt;

use crate::hbr_quality_mapping::{HbrQualityMappingError, stable_sigmoid};

#[derive(Clone, Debug, PartialEq)]
pub struct HbrResidualGroupInput {
    pub anchor_channel_group_id: u64,
    pub initial_geometric_residuals_m: Vec<f64>,
    /// One non-negative packet correction prior for each residual.
    pub packet_correction_priors_m: Vec<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HbrResidualGateConfig {
    pub beta_center: f64,
    pub beta_spread: f64,
    pub beta_correction_prior: f64,
    pub logistic_slope: f64,
    pub activation_threshold: f64,
    pub source_observation_manifest_sha256: [u8; 32],
    pub quality_mapping_sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq)]
pub struct HbrResidualGate {
    pub anchor_channel_group_id: u64,
    pub observation_count: usize,
    pub residual_center_m: f64,
    pub residual_mad_m: f64,
    pub median_correction_prior_m: f64,
    pub anomaly_score: f64,
    pub soft_gate_weight: f64,
    pub source_observation_manifest_sha256: [u8; 32],
    pub quality_mapping_sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HbrResidualGateError {
    InvalidConfiguration,
    MissingSourceManifestCommitment,
    MissingQualityMappingCommitment,
    EmptyGroupSet,
    InvalidGroupIdentity,
    DuplicateGroupIdentity(u64),
    EmptyResidualGroup(u64),
    ResidualAndPriorCountMismatch(u64),
    InvalidResidual { group_id: u64, index: usize },
    InvalidCorrectionPrior { group_id: u64, index: usize },
    ArithmeticOverflow,
    QualityMapping(HbrQualityMappingError),
}

impl fmt::Display for HbrResidualGateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration => write!(f, "HBR residual-gate weights must be finite and nonnegative; logistic slope must be positive"),
            Self::MissingSourceManifestCommitment => write!(f, "source-observation manifest SHA-256 must be nonzero"),
            Self::MissingQualityMappingCommitment => write!(f, "quality-mapping SHA-256 must be nonzero"),
            Self::EmptyGroupSet => write!(f, "at least one anchor-channel group is required"),
            Self::InvalidGroupIdentity => write!(f, "anchor-channel group identity must be nonzero"),
            Self::DuplicateGroupIdentity(id) => write!(f, "duplicate anchor-channel group identity: {id}"),
            Self::EmptyResidualGroup(id) => write!(f, "anchor-channel group {id} has no initial geometric residuals"),
            Self::ResidualAndPriorCountMismatch(id) => write!(f, "residual and correction-prior counts differ for group {id}"),
            Self::InvalidResidual { group_id, index } => write!(f, "initial geometric residual is invalid in group {group_id} at index {index}"),
            Self::InvalidCorrectionPrior { group_id, index } => write!(f, "packet correction prior is invalid in group {group_id} at index {index}"),
            Self::ArithmeticOverflow => write!(f, "residual-gate arithmetic produced a non-finite value"),
            Self::QualityMapping(error) => write!(f, "quality mapping validation failed: {error}"),
        }
    }
}

impl std::error::Error for HbrResidualGateError {}

impl From<HbrQualityMappingError> for HbrResidualGateError {
    fn from(error: HbrQualityMappingError) -> Self {
        Self::QualityMapping(error)
    }
}

/// Compute frozen HBR-SRSC gates for a set of anchor-channel groups.
///
/// Input order does not affect output order or gate values. The median and raw
/// MAD definitions match the paper's group-level equations; no Gaussian
/// 1.4826 scaling factor is silently applied.
pub fn precompute_hbr_residual_gates(
    groups: &[HbrResidualGroupInput],
    config: HbrResidualGateConfig,
) -> Result<Vec<HbrResidualGate>, HbrResidualGateError> {
    if groups.is_empty() {
        return Err(HbrResidualGateError::EmptyGroupSet);
    }
    if !config.beta_center.is_finite()
        || config.beta_center < 0.0
        || !config.beta_spread.is_finite()
        || config.beta_spread < 0.0
        || !config.beta_correction_prior.is_finite()
        || config.beta_correction_prior < 0.0
        || !config.logistic_slope.is_finite()
        || config.logistic_slope <= 0.0
        || !config.activation_threshold.is_finite()
    {
        return Err(HbrResidualGateError::InvalidConfiguration);
    }
    if config.source_observation_manifest_sha256.iter().all(|byte| *byte == 0) {
        return Err(HbrResidualGateError::MissingSourceManifestCommitment);
    }
    if config.quality_mapping_sha256.iter().all(|byte| *byte == 0) {
        return Err(HbrResidualGateError::MissingQualityMappingCommitment);
    }

    let mut canonical_groups = groups.to_vec();
    canonical_groups.sort_by_key(|group| group.anchor_channel_group_id);
    let mut identities = BTreeSet::new();
    let mut gates = Vec::with_capacity(canonical_groups.len());

    for group in &canonical_groups {
        if group.anchor_channel_group_id == 0 {
            return Err(HbrResidualGateError::InvalidGroupIdentity);
        }
        if !identities.insert(group.anchor_channel_group_id) {
            return Err(HbrResidualGateError::DuplicateGroupIdentity(
                group.anchor_channel_group_id,
            ));
        }
        if group.initial_geometric_residuals_m.is_empty() {
            return Err(HbrResidualGateError::EmptyResidualGroup(
                group.anchor_channel_group_id,
            ));
        }
        if group.initial_geometric_residuals_m.len()
            != group.packet_correction_priors_m.len()
        {
            return Err(HbrResidualGateError::ResidualAndPriorCountMismatch(
                group.anchor_channel_group_id,
            ));
        }
        for (index, residual) in group.initial_geometric_residuals_m.iter().enumerate() {
            if !residual.is_finite() {
                return Err(HbrResidualGateError::InvalidResidual {
                    group_id: group.anchor_channel_group_id,
                    index,
                });
            }
        }
        for (index, prior) in group.packet_correction_priors_m.iter().enumerate() {
            if !prior.is_finite() || *prior < 0.0 {
                return Err(HbrResidualGateError::InvalidCorrectionPrior {
                    group_id: group.anchor_channel_group_id,
                    index,
                });
            }
        }

        let center = median(&group.initial_geometric_residuals_m)?;
        let absolute_deviations: Vec<f64> = group
            .initial_geometric_residuals_m
            .iter()
            .map(|value| (value - center).abs())
            .collect();
        if !absolute_deviations.iter().all(|value| value.is_finite()) {
            return Err(HbrResidualGateError::ArithmeticOverflow);
        }
        let mad = median(&absolute_deviations)?;
        let median_prior = median(&group.packet_correction_priors_m)?;

        let anomaly_score = config.beta_center * center.abs()
            + config.beta_spread * mad
            + config.beta_correction_prior * median_prior;
        if !anomaly_score.is_finite() {
            return Err(HbrResidualGateError::ArithmeticOverflow);
        }
        let gate_argument =
            config.logistic_slope * (anomaly_score - config.activation_threshold);
        let soft_gate_weight =
            stable_sigmoid(gate_argument).map_err(HbrResidualGateError::from)?;

        gates.push(HbrResidualGate {
            anchor_channel_group_id: group.anchor_channel_group_id,
            observation_count: group.initial_geometric_residuals_m.len(),
            residual_center_m: center,
            residual_mad_m: mad,
            median_correction_prior_m: median_prior,
            anomaly_score,
            soft_gate_weight,
            source_observation_manifest_sha256: config.source_observation_manifest_sha256,
            quality_mapping_sha256: config.quality_mapping_sha256,
        });
    }

    Ok(gates)
}

fn median(values: &[f64]) -> Result<f64, HbrResidualGateError> {
    if values.is_empty() || !values.iter().all(|value| value.is_finite()) {
        return Err(HbrResidualGateError::ArithmeticOverflow);
    }
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    let middle = ordered.len() / 2;
    let result = if ordered.len().is_multiple_of(2) {
        ordered[middle - 1] / 2.0 + ordered[middle] / 2.0
    } else {
        ordered[middle]
    };
    result
        .is_finite()
        .then_some(result)
        .ok_or(HbrResidualGateError::ArithmeticOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> HbrResidualGateConfig {
        HbrResidualGateConfig {
            beta_center: 1.0,
            beta_spread: 2.0,
            beta_correction_prior: 0.5,
            logistic_slope: 3.0,
            activation_threshold: 0.5,
            source_observation_manifest_sha256: [0x11; 32],
            quality_mapping_sha256: [0x22; 32],
        }
    }

    fn group(id: u64, residuals: &[f64], priors: &[f64]) -> HbrResidualGroupInput {
        HbrResidualGroupInput {
            anchor_channel_group_id: id,
            initial_geometric_residuals_m: residuals.to_vec(),
            packet_correction_priors_m: priors.to_vec(),
        }
    }

    #[test]
    fn computes_median_mad_prior_score_and_paper_soft_gate() {
        let result = precompute_hbr_residual_gates(
            &[group(7, &[1.0, 2.0, 100.0], &[0.2, 0.4, 0.9])],
            config(),
        )
        .unwrap();
        let gate = &result[0];
        assert_eq!(gate.anchor_channel_group_id, 7);
        assert_eq!(gate.observation_count, 3);
        assert_eq!(gate.residual_center_m, 2.0);
        assert_eq!(gate.residual_mad_m, 1.0);
        assert_eq!(gate.median_correction_prior_m, 0.4);
        assert_eq!(gate.anomaly_score, 4.2);
        let expected = 1.0 / (1.0 + (-3.0_f64 * (4.2 - 0.5)).exp());
        assert!((gate.soft_gate_weight - expected).abs() < 1e-12);
        assert!(gate.soft_gate_weight > 0.5);
    }

    #[test]
    fn larger_initial_anomaly_score_increases_soft_gate() {
        let result = precompute_hbr_residual_gates(
            &[
                group(1, &[0.01, 0.02, 0.03], &[0.0, 0.0, 0.0]),
                group(2, &[0.8, 1.0, 1.2], &[0.4, 0.5, 0.6]),
            ],
            config(),
        )
        .unwrap();
        assert!(result[1].anomaly_score > result[0].anomaly_score);
        assert!(result[1].soft_gate_weight > result[0].soft_gate_weight);
    }

    #[test]
    fn median_and_mad_are_invariant_to_packet_order() {
        let a = precompute_hbr_residual_gates(
            &[group(3, &[1.0, 9.0, 2.0, 8.0], &[0.1, 0.9, 0.2, 0.8])],
            config(),
        )
        .unwrap();
        let b = precompute_hbr_residual_gates(
            &[group(3, &[8.0, 2.0, 9.0, 1.0], &[0.8, 0.2, 0.9, 0.1])],
            config(),
        )
        .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn group_output_is_canonical_under_input_group_order() {
        let first = precompute_hbr_residual_gates(
            &[
                group(9, &[1.0, 2.0], &[0.1, 0.2]),
                group(3, &[4.0, 5.0], &[0.3, 0.4]),
            ],
            config(),
        )
        .unwrap();
        let second = precompute_hbr_residual_gates(
            &[
                group(3, &[4.0, 5.0], &[0.3, 0.4]),
                group(9, &[1.0, 2.0], &[0.1, 0.2]),
            ],
            config(),
        )
        .unwrap();
        assert_eq!(first, second);
        assert_eq!(first[0].anchor_channel_group_id, 3);
    }

    #[test]
    fn zero_anomaly_weight_and_extreme_scores_remain_finite() {
        let mut cfg = config();
        cfg.beta_center = 0.0;
        cfg.beta_spread = 0.0;
        cfg.beta_correction_prior = 0.0;
        let result = precompute_hbr_residual_gates(
            &[group(1, &[f64::MAX, f64::MAX], &[f64::MAX, f64::MAX])],
            cfg,
        )
        .unwrap();
        assert_eq!(result[0].anomaly_score, 0.0);
        assert!((result[0].soft_gate_weight - 1.0 / (1.0 + (1.5_f64).exp())).abs() < 1e-12);
    }

    #[test]
    fn rejects_empty_duplicate_and_malformed_groups() {
        assert_eq!(
            precompute_hbr_residual_gates(&[], config()),
            Err(HbrResidualGateError::EmptyGroupSet)
        );
        assert_eq!(
            precompute_hbr_residual_gates(&[group(0, &[1.0], &[0.1])], config()),
            Err(HbrResidualGateError::InvalidGroupIdentity)
        );
        assert_eq!(
            precompute_hbr_residual_gates(
                &[group(1, &[1.0], &[0.1]), group(1, &[2.0], &[0.2])],
                config()
            ),
            Err(HbrResidualGateError::DuplicateGroupIdentity(1))
        );
        assert_eq!(
            precompute_hbr_residual_gates(&[group(1, &[], &[])], config()),
            Err(HbrResidualGateError::EmptyResidualGroup(1))
        );
        assert_eq!(
            precompute_hbr_residual_gates(&[group(1, &[1.0, 2.0], &[0.1])], config()),
            Err(HbrResidualGateError::ResidualAndPriorCountMismatch(1))
        );
    }

    #[test]
    fn rejects_nonfinite_residuals_and_negative_correction_priors() {
        assert_eq!(
            precompute_hbr_residual_gates(&[group(1, &[1.0, f64::NAN], &[0.1, 0.2])], config()),
            Err(HbrResidualGateError::InvalidResidual {
                group_id: 1,
                index: 1,
            })
        );
        assert_eq!(
            precompute_hbr_residual_gates(&[group(1, &[1.0], &[-0.1])], config()),
            Err(HbrResidualGateError::InvalidCorrectionPrior {
                group_id: 1,
                index: 0,
            })
        );
    }

    #[test]
    fn validates_config_and_evidence_commitments() {
        let mut invalid = config();
        invalid.beta_spread = -1.0;
        assert_eq!(
            precompute_hbr_residual_gates(&[group(1, &[1.0], &[0.1])], invalid),
            Err(HbrResidualGateError::InvalidConfiguration)
        );

        invalid = config();
        invalid.logistic_slope = 0.0;
        assert_eq!(
            precompute_hbr_residual_gates(&[group(1, &[1.0], &[0.1])], invalid),
            Err(HbrResidualGateError::InvalidConfiguration)
        );

        invalid = config();
        invalid.source_observation_manifest_sha256 = [0; 32];
        assert_eq!(
            precompute_hbr_residual_gates(&[group(1, &[1.0], &[0.1])], invalid),
            Err(HbrResidualGateError::MissingSourceManifestCommitment)
        );

        invalid = config();
        invalid.quality_mapping_sha256 = [0; 32];
        assert_eq!(
            precompute_hbr_residual_gates(&[group(1, &[1.0], &[0.1])], invalid),
            Err(HbrResidualGateError::MissingQualityMappingCommitment)
        );
    }

    #[test]
    fn median_overflow_and_gate_overflow_fail_closed() {
        assert_eq!(
            precompute_hbr_residual_gates(
                &[group(1, &[-f64::MAX, f64::MAX], &[0.1, 0.2])],
                config()
            ),
            Err(HbrResidualGateError::ArithmeticOverflow)
        );
        let mut invalid = config();
        invalid.beta_center = f64::MAX;
        invalid.beta_spread = f64::MAX;
        assert_eq!(
            precompute_hbr_residual_gates(&[group(1, &[f64::MAX, -f64::MAX], &[0.1, 0.2])], invalid),
            Err(HbrResidualGateError::ArithmeticOverflow)
        );
    }
}
