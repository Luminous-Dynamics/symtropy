//! HBR-PGD packet-quality mapping primitives.
//!
//! Faithful to the published HBR-HQRM functional form:
//!   q_tilde = w_q^T zscore(q)
//!   sigma = sigma_min + (sigma_max - sigma_min) sigmoid(-k_sigma q_tilde)
//!   alpha = alpha_min + (alpha_max - alpha_min) sigmoid(k_alpha q_tilde)
//!   delta_hat = delta_max sigmoid(-k_delta q_tilde)
//!
//! The calibration means, scales, weights, logistic slopes and bounds are
//! caller-supplied artifacts. This module does not infer them, tune them,
//! access evaluator labels or claim the paper's exact numeric reproduction.
//! Parameter and feature-schema commitments are preserved in the output.

use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct HbrQualityCalibration {
    /// Per-feature mean from the frozen calibration cohort.
    pub feature_mean: Vec<f64>,
    /// Per-feature positive standard deviation from the frozen calibration cohort.
    pub feature_std_dev: Vec<f64>,
    /// Calibrated quality-score weights. Signs are learned/frozen externally.
    pub feature_weights: Vec<f64>,
    pub sigma_min_m: f64,
    pub sigma_max_m: f64,
    pub alpha_min: f64,
    pub alpha_max: f64,
    pub correction_max_m: f64,
    pub k_sigma: f64,
    pub k_alpha: f64,
    pub k_delta: f64,
    /// Commitment to the calibration cohort, split and frozen mapping parameters.
    pub calibration_sha256: [u8; 32],
    /// Commitment to ordered feature names, units and extraction semantics.
    pub feature_schema_sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq)]
pub struct HbrQualityParameters {
    pub normalized_quality_score: f64,
    pub heteroscedastic_scale_m: f64,
    pub robust_loss_shape_alpha: f64,
    pub nonnegative_correction_prior_m: f64,
    pub calibration_sha256: [u8; 32],
    pub feature_schema_sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HbrQualityMappingError {
    EmptyFeatureVector,
    FeatureDimensionMismatch,
    InvalidFeatureValue(usize),
    InvalidCalibrationMean(usize),
    InvalidCalibrationScale(usize),
    InvalidFeatureWeight(usize),
    InvalidSigmaBounds,
    InvalidAlphaBounds,
    InvalidCorrectionBound,
    InvalidLogisticSlope,
    MissingCalibrationCommitment,
    MissingFeatureSchemaCommitment,
    ArithmeticOverflow,
}

impl fmt::Display for HbrQualityMappingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyFeatureVector => write!(f, "at least one packet-quality feature is required"),
            Self::FeatureDimensionMismatch => write!(f, "feature, mean, standard-deviation and weight dimensions must match"),
            Self::InvalidFeatureValue(index) => write!(f, "packet-quality feature is invalid at index {index}"),
            Self::InvalidCalibrationMean(index) => write!(f, "calibration mean is invalid at index {index}"),
            Self::InvalidCalibrationScale(index) => write!(f, "calibration standard deviation must be finite and positive at index {index}"),
            Self::InvalidFeatureWeight(index) => write!(f, "quality-score weight is invalid at index {index}"),
            Self::InvalidSigmaBounds => write!(f, "heteroscedastic scale bounds must be finite, positive and ordered"),
            Self::InvalidAlphaBounds => write!(f, "robust-loss shape bounds must be finite and strictly ordered"),
            Self::InvalidCorrectionBound => write!(f, "maximum non-negative correction must be finite and nonnegative"),
            Self::InvalidLogisticSlope => write!(f, "logistic mapping slopes must be finite and strictly positive"),
            Self::MissingCalibrationCommitment => write!(f, "calibration SHA-256 commitment must be nonzero"),
            Self::MissingFeatureSchemaCommitment => write!(f, "feature-schema SHA-256 commitment must be nonzero"),
            Self::ArithmeticOverflow => write!(f, "quality mapping arithmetic produced a non-finite value"),
        }
    }
}

impl std::error::Error for HbrQualityMappingError {}

/// Apply a frozen HBR-HQRM quality mapping to one packet's quality vector.
///
/// The learned convention is that a larger normalized quality score denotes
/// better measurement quality. Therefore sigma and delta_hat decrease while
/// alpha increases as q_tilde increases, exactly as in the paper's logistic
/// mappings. The caller's calibration weights determine the sign semantics.
pub fn map_hbr_packet_quality(
    quality_features: &[f64],
    calibration: &HbrQualityCalibration,
) -> Result<HbrQualityParameters, HbrQualityMappingError> {
    if quality_features.is_empty() {
        return Err(HbrQualityMappingError::EmptyFeatureVector);
    }
    let dimension = quality_features.len();
    if calibration.feature_mean.len() != dimension
        || calibration.feature_std_dev.len() != dimension
        || calibration.feature_weights.len() != dimension
    {
        return Err(HbrQualityMappingError::FeatureDimensionMismatch);
    }

    if !calibration.calibration_sha256.iter().any(|byte| *byte != 0) {
        return Err(HbrQualityMappingError::MissingCalibrationCommitment);
    }
    if !calibration.feature_schema_sha256.iter().any(|byte| *byte != 0) {
        return Err(HbrQualityMappingError::MissingFeatureSchemaCommitment);
    }

    if !calibration.sigma_min_m.is_finite()
        || !calibration.sigma_max_m.is_finite()
        || calibration.sigma_min_m <= 0.0
        || calibration.sigma_max_m < calibration.sigma_min_m
    {
        return Err(HbrQualityMappingError::InvalidSigmaBounds);
    }
    if !calibration.alpha_min.is_finite()
        || !calibration.alpha_max.is_finite()
        || calibration.alpha_max <= calibration.alpha_min
    {
        return Err(HbrQualityMappingError::InvalidAlphaBounds);
    }
    if !calibration.correction_max_m.is_finite() || calibration.correction_max_m < 0.0 {
        return Err(HbrQualityMappingError::InvalidCorrectionBound);
    }
    if !calibration.k_sigma.is_finite()
        || calibration.k_sigma <= 0.0
        || !calibration.k_alpha.is_finite()
        || calibration.k_alpha <= 0.0
        || !calibration.k_delta.is_finite()
        || calibration.k_delta <= 0.0
    {
        return Err(HbrQualityMappingError::InvalidLogisticSlope);
    }

    let mut quality_score = 0.0_f64;
    for index in 0..dimension {
        let feature = quality_features[index];
        let mean = calibration.feature_mean[index];
        let standard_deviation = calibration.feature_std_dev[index];
        let weight = calibration.feature_weights[index];

        if !feature.is_finite() {
            return Err(HbrQualityMappingError::InvalidFeatureValue(index));
        }
        if !mean.is_finite() {
            return Err(HbrQualityMappingError::InvalidCalibrationMean(index));
        }
        if !standard_deviation.is_finite() || standard_deviation <= 0.0 {
            return Err(HbrQualityMappingError::InvalidCalibrationScale(index));
        }
        if !weight.is_finite() {
            return Err(HbrQualityMappingError::InvalidFeatureWeight(index));
        }

        let standardized = (feature - mean) / standard_deviation;
        let weighted = weight * standardized;
        if !standardized.is_finite() || !weighted.is_finite() {
            return Err(HbrQualityMappingError::ArithmeticOverflow);
        }
        quality_score += weighted;
        if !quality_score.is_finite() {
            return Err(HbrQualityMappingError::ArithmeticOverflow);
        }
    }

    let sigma_fraction = stable_sigmoid(-calibration.k_sigma * quality_score)?;
    let alpha_fraction = stable_sigmoid(calibration.k_alpha * quality_score)?;
    let correction_fraction = stable_sigmoid(-calibration.k_delta * quality_score)?;

    let sigma = calibration.sigma_min_m
        + (calibration.sigma_max_m - calibration.sigma_min_m) * sigma_fraction;
    let alpha = calibration.alpha_min
        + (calibration.alpha_max - calibration.alpha_min) * alpha_fraction;
    let correction = calibration.correction_max_m * correction_fraction;

    if !sigma.is_finite() || !alpha.is_finite() || !correction.is_finite() {
        return Err(HbrQualityMappingError::ArithmeticOverflow);
    }
    if sigma < calibration.sigma_min_m
        || sigma > calibration.sigma_max_m
        || alpha < calibration.alpha_min
        || alpha > calibration.alpha_max
        || correction < 0.0
        || correction > calibration.correction_max_m
    {
        return Err(HbrQualityMappingError::ArithmeticOverflow);
    }

    Ok(HbrQualityParameters {
        normalized_quality_score: quality_score,
        heteroscedastic_scale_m: sigma,
        robust_loss_shape_alpha: alpha,
        nonnegative_correction_prior_m: correction,
        calibration_sha256: calibration.calibration_sha256,
        feature_schema_sha256: calibration.feature_schema_sha256,
    })
}

/// Numerically stable logistic sigmoid.
pub(crate) fn stable_sigmoid(value: f64) -> Result<f64, HbrQualityMappingError> {
    if !value.is_finite() {
        return Err(HbrQualityMappingError::ArithmeticOverflow);
    }
    let result = if value >= 0.0 {
        1.0 / (1.0 + (-value).exp())
    } else {
        let exponential = value.exp();
        exponential / (1.0 + exponential)
    };
    result
        .is_finite()
        .then_some(result)
        .ok_or(HbrQualityMappingError::ArithmeticOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calibration() -> HbrQualityCalibration {
        HbrQualityCalibration {
            feature_mean: vec![0.0, 0.0],
            feature_std_dev: vec![1.0, 2.0],
            feature_weights: vec![1.0, 0.5],
            sigma_min_m: 0.052,
            sigma_max_m: 0.200,
            alpha_min: -1.5,
            alpha_max: 1.8,
            correction_max_m: 1.25,
            k_sigma: 1.1,
            k_alpha: 0.9,
            k_delta: 1.3,
            calibration_sha256: [0x11; 32],
            feature_schema_sha256: [0x22; 32],
        }
    }

    #[test]
    fn maps_packet_quality_to_paper_defined_bounded_parameters() {
        let result = map_hbr_packet_quality(&[0.0, 0.0], &calibration()).unwrap();
        assert_eq!(result.normalized_quality_score, 0.0);
        assert_eq!(result.heteroscedastic_scale_m, 0.126);
        assert_eq!(result.robust_loss_shape_alpha, 0.15);
        assert_eq!(result.nonnegative_correction_prior_m, 0.625);
        assert_eq!(result.calibration_sha256, [0x11; 32]);
        assert_eq!(result.feature_schema_sha256, [0x22; 32]);
    }

    #[test]
    fn higher_quality_score_reduces_scale_and_correction_and_increases_alpha() {
        let calibration = calibration();
        let lower = map_hbr_packet_quality(&[-2.0, -2.0], &calibration).unwrap();
        let higher = map_hbr_packet_quality(&[2.0, 2.0], &calibration).unwrap();
        assert!(higher.normalized_quality_score > lower.normalized_quality_score);
        assert!(higher.heteroscedastic_scale_m < lower.heteroscedastic_scale_m);
        assert!(higher.robust_loss_shape_alpha > lower.robust_loss_shape_alpha);
        assert!(
            higher.nonnegative_correction_prior_m < lower.nonnegative_correction_prior_m
        );
    }

    #[test]
    fn zero_score_maps_to_midpoints_of_the_configured_intervals() {
        let result = map_hbr_packet_quality(&[0.0, 0.0], &calibration()).unwrap();
        assert!((result.heteroscedastic_scale_m - 0.126).abs() < 1e-12);
        assert!((result.robust_loss_shape_alpha - 0.15).abs() < 1e-12);
        assert!((result.nonnegative_correction_prior_m - 0.625).abs() < 1e-12);
    }

    #[test]
    fn calibration_and_feature_dimensions_must_match() {
        let mut invalid = calibration();
        invalid.feature_weights.pop();
        assert_eq!(
            map_hbr_packet_quality(&[1.0, 2.0], &invalid),
            Err(HbrQualityMappingError::FeatureDimensionMismatch)
        );
        assert_eq!(
            map_hbr_packet_quality(&[], &calibration()),
            Err(HbrQualityMappingError::EmptyFeatureVector)
        );
    }

    #[test]
    fn rejects_nonfinite_features_means_scales_and_weights() {
        let mut invalid = calibration();
        invalid.feature_mean[0] = f64::NAN;
        assert_eq!(
            map_hbr_packet_quality(&[1.0, 2.0], &invalid),
            Err(HbrQualityMappingError::InvalidCalibrationMean(0))
        );

        invalid = calibration();
        invalid.feature_std_dev[1] = 0.0;
        assert_eq!(
            map_hbr_packet_quality(&[1.0, 2.0], &invalid),
            Err(HbrQualityMappingError::InvalidCalibrationScale(1))
        );

        invalid = calibration();
        invalid.feature_weights[0] = f64::INFINITY;
        assert_eq!(
            map_hbr_packet_quality(&[1.0, 2.0], &invalid),
            Err(HbrQualityMappingError::InvalidFeatureWeight(0))
        );

        assert_eq!(
            map_hbr_packet_quality(&[f64::NAN, 0.0], &calibration()),
            Err(HbrQualityMappingError::InvalidFeatureValue(0))
        );
    }

    #[test]
    fn validates_bounds_slopes_and_provenance_commitments() {
        let mut invalid = calibration();
        invalid.sigma_min_m = 0.0;
        assert_eq!(
            map_hbr_packet_quality(&[0.0, 0.0], &invalid),
            Err(HbrQualityMappingError::InvalidSigmaBounds)
        );

        invalid = calibration();
        invalid.alpha_max = invalid.alpha_min;
        assert_eq!(
            map_hbr_packet_quality(&[0.0, 0.0], &invalid),
            Err(HbrQualityMappingError::InvalidAlphaBounds)
        );

        invalid = calibration();
        invalid.correction_max_m = -0.1;
        assert_eq!(
            map_hbr_packet_quality(&[0.0, 0.0], &invalid),
            Err(HbrQualityMappingError::InvalidCorrectionBound)
        );

        invalid = calibration();
        invalid.k_delta = 0.0;
        assert_eq!(
            map_hbr_packet_quality(&[0.0, 0.0], &invalid),
            Err(HbrQualityMappingError::InvalidLogisticSlope)
        );

        invalid = calibration();
        invalid.calibration_sha256 = [0; 32];
        assert_eq!(
            map_hbr_packet_quality(&[0.0, 0.0], &invalid),
            Err(HbrQualityMappingError::MissingCalibrationCommitment)
        );

        invalid = calibration();
        invalid.feature_schema_sha256 = [0; 32];
        assert_eq!(
            map_hbr_packet_quality(&[0.0, 0.0], &invalid),
            Err(HbrQualityMappingError::MissingFeatureSchemaCommitment)
        );
    }

    #[test]
    fn quality_mapping_remains_bounded_at_extreme_finite_scores() {
        let positive = map_hbr_packet_quality(&[1.0e100, 1.0e100], &calibration()).unwrap();
        let negative = map_hbr_packet_quality(&[-1.0e100, -1.0e100], &calibration()).unwrap();
        assert!(positive.heteroscedastic_scale_m >= 0.052);
        assert!(positive.heteroscedastic_scale_m <= 0.200);
        assert!(positive.robust_loss_shape_alpha >= -1.5);
        assert!(positive.robust_loss_shape_alpha <= 1.8);
        assert!(positive.nonnegative_correction_prior_m >= 0.0);
        assert!(positive.nonnegative_correction_prior_m <= 1.25);
        assert!(negative.heteroscedastic_scale_m >= 0.052);
        assert!(negative.heteroscedastic_scale_m <= 0.200);
    }

    #[test]
    fn overflow_in_standardization_or_score_fails_closed() {
        let mut invalid = calibration();
        invalid.feature_std_dev[0] = f64::MIN_POSITIVE;
        assert_eq!(
            map_hbr_packet_quality(&[f64::MAX, 0.0], &invalid),
            Err(HbrQualityMappingError::ArithmeticOverflow)
        );
    }
}
