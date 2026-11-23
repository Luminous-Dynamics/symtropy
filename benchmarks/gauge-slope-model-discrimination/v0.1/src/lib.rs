//! GAUGE-specific model-discrimination reference for fixed-angle slope sliding.
//!
//! M0 is a constant-acceleration model (constant Coulomb friction plus an
//! unknown intercept). M1 adds a local linear acceleration-speed term. The
//! intercept is profiled out analytically, so a constant acceleration offset
//! cannot masquerade as speed dependence. A detected speed trend is not
//! uniquely attributable to friction; other speed-correlated effects or
//! measurement artifacts remain competing explanations.
//!
//! Each input row is one independent, preregistered trial-level aggregate.
//! Adjacent trajectory frames are not independent observations.

use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelDiscriminationError {
    FewerThanThreeTrials,
    FewerThanTwoDesignConditions,
    EmptyTrialId,
    DuplicateTrialId,
    InvalidSpeed,
    InvalidAcceleration,
    InvalidUncertainty,
    InvalidWeight,
    InsufficientSpeedExcitation,
    NonFiniteArithmetic,
    InvalidGeometry,
}

impl Display for ModelDiscriminationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::FewerThanThreeTrials => "at least three independent trials are required",
            Self::FewerThanTwoDesignConditions => "at least two candidate conditions are required",
            Self::EmptyTrialId => "trial identity must not be empty",
            Self::DuplicateTrialId => "each row must represent a distinct physical trial",
            Self::InvalidSpeed => "speed must be finite and non-negative",
            Self::InvalidAcceleration => "acceleration must be finite",
            Self::InvalidUncertainty => "acceleration uncertainty must be finite and positive",
            Self::InvalidWeight => "inverse-variance weight is not representable",
            Self::InsufficientSpeedExcitation => {
                "speed spread is zero or numerically rank-deficient after profiling the intercept"
            }
            Self::NonFiniteArithmetic => "weighted regression produced a non-finite result",
            Self::InvalidGeometry => {
                "qualified geometry requires finite positive gravity, an admitted incline angle, and a non-zero evidence digest"
            }
        };
        formatter.write_str(message)
    }
}

impl Error for ModelDiscriminationError {}

/// One independent physical-trial summary.
///
/// trial_id must identify a physical recording/trial, not a frame or window
/// cut from a recording. Speed and acceleration must be computed using a
/// frozen, preregistered windowing procedure.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrialObservation<'a> {
    pub trial_id: &'a str,
    pub speed_m_s: f64,
    pub acceleration_m_s2: f64,
    pub acceleration_sigma_m_s2: f64,
}

/// One candidate trial condition for a prospective experiment design.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DesignCondition {
    pub speed_m_s: f64,
    pub acceleration_sigma_m_s2: f64,
}

/// Fixed-slope geometry whose angle/frame semantics have already passed
/// their independent admission gate. The digest binds the qualifying artifact;
/// this helper validates only its presence and non-zero value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QualifiedSlopeGeometry {
    pub gravity_m_s2: f64,
    pub effective_incline_angle_rad: f64,
    pub evidence_sha256: [u8; 32],
}

/// Result of comparing nested constant and linear-speed acceleration models
/// under independent Gaussian measurement errors with known standard deviations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelComparison {
    pub trial_count: usize,
    pub null_intercept_m_s2: f64,
    pub null_chi_squared: f64,
    pub alternative_intercept_m_s2: f64,
    /// Units: (m/s^2) / (m/s), numerically s^-1.
    pub alternative_slope_m_s2_per_m_s: f64,
    pub alternative_chi_squared: f64,
    pub delta_chi_squared: f64,
    /// BIC(M1) - BIC(M0) = ln(n) - DeltaChi2 for known observation sigmas.
    pub delta_bic_alternative_minus_null: f64,
    pub alternative_residual_degrees_of_freedom: usize,
}

/// Nuisance-intercept-profiled design separation.
///
/// squared_standardized_separation is the weighted squared distance between
/// the hypothesized linear-speed response and its best-fitting constant
/// response. It is a deterministic separation proxy under the declared
/// diagonal Gaussian noise model, not expected information gain, a p-value,
/// or a posterior probability.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProfiledDesignScore {
    pub condition_count: usize,
    pub weighted_mean_speed_m_s: f64,
    pub weighted_centered_speed_sum: f64,
    pub squared_standardized_separation: f64,
}

#[derive(Clone, Copy, Debug, Default)]
struct CompensatedSum {
    sum: f64,
    correction: f64,
}

impl CompensatedSum {
    fn add(&mut self, value: f64) {
        let adjusted = value - self.correction;
        let next = self.sum + adjusted;
        self.correction = (next - self.sum) - adjusted;
        self.sum = next;
    }

    fn value(self) -> f64 {
        self.sum
    }
}

fn validate_sigma(sigma: f64) -> Result<f64, ModelDiscriminationError> {
    if !sigma.is_finite() || sigma <= 0.0 {
        return Err(ModelDiscriminationError::InvalidUncertainty);
    }
    let variance = sigma * sigma;
    let weight = 1.0 / variance;
    if !variance.is_finite() || variance <= 0.0 || !weight.is_finite() || weight <= 0.0 {
        return Err(ModelDiscriminationError::InvalidWeight);
    }
    Ok(weight)
}

fn validate_trial_rows(
    observations: &[TrialObservation<'_>],
) -> Result<Vec<f64>, ModelDiscriminationError> {
    if observations.len() < 3 {
        return Err(ModelDiscriminationError::FewerThanThreeTrials);
    }

    let mut weights = Vec::with_capacity(observations.len());
    for (index, observation) in observations.iter().enumerate() {
        if observation.trial_id.trim().is_empty() {
            return Err(ModelDiscriminationError::EmptyTrialId);
        }
        if observations[..index]
            .iter()
            .any(|previous| previous.trial_id == observation.trial_id)
        {
            return Err(ModelDiscriminationError::DuplicateTrialId);
        }
        if !observation.speed_m_s.is_finite() || observation.speed_m_s < 0.0 {
            return Err(ModelDiscriminationError::InvalidSpeed);
        }
        if !observation.acceleration_m_s2.is_finite() {
            return Err(ModelDiscriminationError::InvalidAcceleration);
        }
        weights.push(validate_sigma(observation.acceleration_sigma_m_s2)?);
    }
    Ok(weights)
}

/// Compare constant-acceleration M0 against local linear-speed M1.
///
/// M0: a_i = b0 + epsilon_i
/// M1: a_i = b0 + b1 * v_i + epsilon_i
///
/// The fit uses inverse-variance weighted least squares. Supplied sigmas are
/// treated as known measurement standard deviations. One independent
/// aggregate per trial ID is required; within-trial autocorrelation, unknown
/// variance, and article/session random effects are outside this v0.1 model.
pub fn compare_constant_vs_speed_dependent(
    observations: &[TrialObservation<'_>],
) -> Result<ModelComparison, ModelDiscriminationError> {
    let weights = validate_trial_rows(observations)?;

    let mut sum_w = CompensatedSum::default();
    let mut sum_wv = CompensatedSum::default();
    let mut sum_wa = CompensatedSum::default();

    for (observation, weight) in observations.iter().zip(weights.iter().copied()) {
        sum_w.add(weight);
        sum_wv.add(weight * observation.speed_m_s);
        sum_wa.add(weight * observation.acceleration_m_s2);
    }

    let total_weight = sum_w.value();
    let mean_speed = sum_wv.value() / total_weight;
    let mean_acceleration = sum_wa.value() / total_weight;
    if !total_weight.is_finite()
        || total_weight <= 0.0
        || !mean_speed.is_finite()
        || !mean_acceleration.is_finite()
    {
        return Err(ModelDiscriminationError::NonFiniteArithmetic);
    }

    let mut centered_speed_information = CompensatedSum::default();
    let mut centered_speed_acceleration = CompensatedSum::default();
    let mut null_chi_squared = CompensatedSum::default();

    for (observation, weight) in observations.iter().zip(weights.iter().copied()) {
        let centered_speed = observation.speed_m_s - mean_speed;
        let centered_acceleration = observation.acceleration_m_s2 - mean_acceleration;
        centered_speed_information.add(weight * centered_speed * centered_speed);
        centered_speed_acceleration.add(weight * centered_speed * centered_acceleration);
        let residual = observation.acceleration_m_s2 - mean_acceleration;
        null_chi_squared.add(weight * residual * residual);
    }

    let sxx = centered_speed_information.value();
    let scale = total_weight * mean_speed.abs().max(1.0).powi(2);
    if !sxx.is_finite() || sxx <= f64::EPSILON * scale {
        return Err(ModelDiscriminationError::InsufficientSpeedExcitation);
    }

    let slope = centered_speed_acceleration.value() / sxx;
    let intercept = mean_acceleration - slope * mean_speed;
    if !slope.is_finite() || !intercept.is_finite() {
        return Err(ModelDiscriminationError::NonFiniteArithmetic);
    }

    let mut alternative_chi_squared = CompensatedSum::default();
    for (observation, weight) in observations.iter().zip(weights.iter().copied()) {
        let predicted = intercept + slope * observation.speed_m_s;
        let residual = observation.acceleration_m_s2 - predicted;
        alternative_chi_squared.add(weight * residual * residual);
    }

    let null_chi_squared = null_chi_squared.value();
    let alternative_chi_squared = alternative_chi_squared.value();
    if !null_chi_squared.is_finite()
        || !alternative_chi_squared.is_finite()
        || alternative_chi_squared < 0.0
    {
        return Err(ModelDiscriminationError::NonFiniteArithmetic);
    }

    let raw_delta = null_chi_squared - alternative_chi_squared;
    let delta_chi_squared = if raw_delta < 0.0 && raw_delta.abs() <= 1.0e-10 {
        0.0
    } else if raw_delta < 0.0 {
        return Err(ModelDiscriminationError::NonFiniteArithmetic);
    } else {
        raw_delta
    };
    let delta_bic = (observations.len() as f64).ln() - delta_chi_squared;
    if !delta_bic.is_finite() {
        return Err(ModelDiscriminationError::NonFiniteArithmetic);
    }

    Ok(ModelComparison {
        trial_count: observations.len(),
        null_intercept_m_s2: mean_acceleration,
        null_chi_squared,
        alternative_intercept_m_s2: intercept,
        alternative_slope_m_s2_per_m_s: slope,
        alternative_chi_squared,
        delta_chi_squared,
        delta_bic_alternative_minus_null: delta_bic,
        alternative_residual_degrees_of_freedom: observations.len() - 2,
    })
}

/// Score a prospective speed design after profiling out the constant
/// acceleration intercept.
///
/// For hypothesized M1 slope beta and independent errors with weights
/// w_i = 1/sigma_i^2, the result is
///
/// lambda = beta^2 * sum_i w_i * (v_i - weighted_mean(v))^2.
///
/// Repeated speeds have zero discrimination power regardless of the shared
/// intercept. Larger speed spread and lower measurement noise increase the
/// declared-model separation.
pub fn score_speed_design(
    conditions: &[DesignCondition],
    hypothesized_acceleration_speed_slope: f64,
) -> Result<ProfiledDesignScore, ModelDiscriminationError> {
    if conditions.len() < 2 {
        return Err(ModelDiscriminationError::FewerThanTwoDesignConditions);
    }
    if !hypothesized_acceleration_speed_slope.is_finite() {
        return Err(ModelDiscriminationError::NonFiniteArithmetic);
    }

    let mut sum_w = CompensatedSum::default();
    let mut sum_wv = CompensatedSum::default();
    let mut weights = Vec::with_capacity(conditions.len());

    for condition in conditions {
        if !condition.speed_m_s.is_finite() || condition.speed_m_s < 0.0 {
            return Err(ModelDiscriminationError::InvalidSpeed);
        }
        let weight = validate_sigma(condition.acceleration_sigma_m_s2)?;
        weights.push(weight);
        sum_w.add(weight);
        sum_wv.add(weight * condition.speed_m_s);
    }

    let total_weight = sum_w.value();
    let mean_speed = sum_wv.value() / total_weight;
    if !total_weight.is_finite() || total_weight <= 0.0 || !mean_speed.is_finite() {
        return Err(ModelDiscriminationError::NonFiniteArithmetic);
    }

    let mut centered = CompensatedSum::default();
    for (condition, weight) in conditions.iter().zip(weights.iter().copied()) {
        let difference = condition.speed_m_s - mean_speed;
        centered.add(weight * difference * difference);
    }
    let centered = centered.value();
    let scale = total_weight * mean_speed.abs().max(1.0).powi(2);
    if !centered.is_finite() || centered <= f64::EPSILON * scale {
        return Err(ModelDiscriminationError::InsufficientSpeedExcitation);
    }

    let separation = hypothesized_acceleration_speed_slope.powi(2) * centered;
    if !separation.is_finite() {
        return Err(ModelDiscriminationError::NonFiniteArithmetic);
    }

    Ok(ProfiledDesignScore {
        condition_count: conditions.len(),
        weighted_mean_speed_m_s: mean_speed,
        weighted_centered_speed_sum: centered,
        squared_standardized_separation: separation,
    })
}

/// Convert the fitted acceleration-speed slope to the local friction
/// coefficient k_v in mu(v) = mu_0 + k_v * v, under an explicitly admitted
/// fixed-angle Coulomb model. This conversion is conditional on that model;
/// a trend alone does not prove friction is its causal source.
///
/// The conversion is unavailable without explicitly supplied qualified
/// geometry. k_v has units s/m and equals -b1/(g*cos(phi)).
pub fn friction_speed_coefficient_s_per_m(
    acceleration_speed_slope_m_s2_per_m_s: f64,
    geometry: Option<QualifiedSlopeGeometry>,
) -> Result<Option<f64>, ModelDiscriminationError> {
    let Some(geometry) = geometry else {
        return Ok(None);
    };
    if !geometry.gravity_m_s2.is_finite()
        || geometry.gravity_m_s2 <= 0.0
        || !geometry.effective_incline_angle_rad.is_finite()
        || geometry.effective_incline_angle_rad <= -std::f64::consts::FRAC_PI_2
        || geometry.effective_incline_angle_rad >= std::f64::consts::FRAC_PI_2
        || geometry.evidence_sha256.iter().all(|byte| *byte == 0)
        || !acceleration_speed_slope_m_s2_per_m_s.is_finite()
    {
        return Err(ModelDiscriminationError::InvalidGeometry);
    }
    let denominator = geometry.gravity_m_s2 * geometry.effective_incline_angle_rad.cos();
    let coefficient = -acceleration_speed_slope_m_s2_per_m_s / denominator;
    if !denominator.is_finite() || denominator <= 0.0 || !coefficient.is_finite() {
        return Err(ModelDiscriminationError::InvalidGeometry);
    }
    Ok(Some(coefficient))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn speed_dependent_fixture() -> [TrialObservation<'static>; 5] {
        [
            TrialObservation {
                trial_id: "trial-001",
                speed_m_s: 0.5,
                acceleration_m_s2: 1.875,
                acceleration_sigma_m_s2: 0.02,
            },
            TrialObservation {
                trial_id: "trial-002",
                speed_m_s: 1.0,
                acceleration_m_s2: 1.749,
                acceleration_sigma_m_s2: 0.02,
            },
            TrialObservation {
                trial_id: "trial-003",
                speed_m_s: 1.5,
                acceleration_m_s2: 1.625,
                acceleration_sigma_m_s2: 0.02,
            },
            TrialObservation {
                trial_id: "trial-004",
                speed_m_s: 2.0,
                acceleration_m_s2: 1.501,
                acceleration_sigma_m_s2: 0.02,
            },
            TrialObservation {
                trial_id: "trial-005",
                speed_m_s: 2.5,
                acceleration_m_s2: 1.374,
                acceleration_sigma_m_s2: 0.02,
            },
        ]
    }

    #[test]
    fn speed_dependent_fixture_recovers_signed_slope_and_prefers_m1() {
        let result = compare_constant_vs_speed_dependent(&speed_dependent_fixture()).unwrap();
        assert!((result.alternative_slope_m_s2_per_m_s + 0.2506).abs() < 0.002);
        assert!(result.delta_chi_squared > 100.0);
        assert!(result.delta_bic_alternative_minus_null < 0.0);
        assert_eq!(result.alternative_residual_degrees_of_freedom, 3);
    }

    #[test]
    fn constant_fixture_does_not_pay_for_an_unneeded_slope() {
        let rows = [
            TrialObservation {
                trial_id: "constant-1",
                speed_m_s: 0.4,
                acceleration_m_s2: 1.25,
                acceleration_sigma_m_s2: 0.03,
            },
            TrialObservation {
                trial_id: "constant-2",
                speed_m_s: 0.8,
                acceleration_m_s2: 1.25,
                acceleration_sigma_m_s2: 0.03,
            },
            TrialObservation {
                trial_id: "constant-3",
                speed_m_s: 1.2,
                acceleration_m_s2: 1.25,
                acceleration_sigma_m_s2: 0.03,
            },
            TrialObservation {
                trial_id: "constant-4",
                speed_m_s: 1.6,
                acceleration_m_s2: 1.25,
                acceleration_sigma_m_s2: 0.03,
            },
        ];
        let result = compare_constant_vs_speed_dependent(&rows).unwrap();
        assert!(result.alternative_slope_m_s2_per_m_s.abs() < 1.0e-12);
        assert!(result.delta_chi_squared.abs() < 1.0e-12);
        assert!(result.delta_bic_alternative_minus_null > 0.0);
    }

    #[test]
    fn repeated_speeds_have_zero_profiled_design_information() {
        let conditions = [
            DesignCondition {
                speed_m_s: 1.0,
                acceleration_sigma_m_s2: 0.02,
            },
            DesignCondition {
                speed_m_s: 1.0,
                acceleration_sigma_m_s2: 0.03,
            },
            DesignCondition {
                speed_m_s: 1.0,
                acceleration_sigma_m_s2: 0.02,
            },
        ];
        assert_eq!(
            score_speed_design(&conditions, -0.25),
            Err(ModelDiscriminationError::InsufficientSpeedExcitation)
        );
    }

    #[test]
    fn wider_speed_spread_is_more_discriminating_at_equal_trial_budget() {
        let narrow = [
            DesignCondition {
                speed_m_s: 0.9,
                acceleration_sigma_m_s2: 0.02,
            },
            DesignCondition {
                speed_m_s: 1.0,
                acceleration_sigma_m_s2: 0.02,
            },
            DesignCondition {
                speed_m_s: 1.1,
                acceleration_sigma_m_s2: 0.02,
            },
        ];
        let wide = [
            DesignCondition {
                speed_m_s: 0.5,
                acceleration_sigma_m_s2: 0.02,
            },
            DesignCondition {
                speed_m_s: 1.5,
                acceleration_sigma_m_s2: 0.02,
            },
            DesignCondition {
                speed_m_s: 2.5,
                acceleration_sigma_m_s2: 0.02,
            },
        ];
        let narrow_score = score_speed_design(&narrow, -0.25).unwrap();
        let wide_score = score_speed_design(&wide, -0.25).unwrap();
        assert!(
            wide_score.squared_standardized_separation
                > narrow_score.squared_standardized_separation
        );
        assert_eq!(wide_score.condition_count, narrow_score.condition_count);
    }

    #[test]
    fn doubling_uncertainty_quarters_profiled_separation() {
        let conditions = [
            DesignCondition {
                speed_m_s: 0.5,
                acceleration_sigma_m_s2: 0.02,
            },
            DesignCondition {
                speed_m_s: 1.0,
                acceleration_sigma_m_s2: 0.02,
            },
            DesignCondition {
                speed_m_s: 2.0,
                acceleration_sigma_m_s2: 0.02,
            },
        ];
        let doubled = conditions.map(|condition| DesignCondition {
            acceleration_sigma_m_s2: condition.acceleration_sigma_m_s2 * 2.0,
            ..condition
        });
        let original = score_speed_design(&conditions, -0.25).unwrap();
        let noisier = score_speed_design(&doubled, -0.25).unwrap();
        assert!(
            (noisier.squared_standardized_separation * 4.0
                - original.squared_standardized_separation)
                .abs()
                < 1.0e-10
        );
    }

    #[test]
    fn duplicate_trial_ids_are_rejected_as_pseudoreplication() {
        let rows = [
            TrialObservation {
                trial_id: "same-recording",
                speed_m_s: 0.5,
                acceleration_m_s2: 1.8,
                acceleration_sigma_m_s2: 0.02,
            },
            TrialObservation {
                trial_id: "same-recording",
                speed_m_s: 1.0,
                acceleration_m_s2: 1.7,
                acceleration_sigma_m_s2: 0.02,
            },
            TrialObservation {
                trial_id: "third-recording",
                speed_m_s: 1.5,
                acceleration_m_s2: 1.6,
                acceleration_sigma_m_s2: 0.02,
            },
        ];
        assert_eq!(
            compare_constant_vs_speed_dependent(&rows),
            Err(ModelDiscriminationError::DuplicateTrialId)
        );
    }

    #[test]
    fn invalid_measurements_fail_closed() {
        let rows = [
            TrialObservation {
                trial_id: "a",
                speed_m_s: 0.5,
                acceleration_m_s2: 1.8,
                acceleration_sigma_m_s2: 0.02,
            },
            TrialObservation {
                trial_id: "b",
                speed_m_s: -1.0,
                acceleration_m_s2: 1.7,
                acceleration_sigma_m_s2: 0.02,
            },
            TrialObservation {
                trial_id: "c",
                speed_m_s: 1.5,
                acceleration_m_s2: 1.6,
                acceleration_sigma_m_s2: 0.02,
            },
        ];
        assert_eq!(
            compare_constant_vs_speed_dependent(&rows),
            Err(ModelDiscriminationError::InvalidSpeed)
        );
    }

    #[test]
    fn coefficient_conversion_is_withheld_without_admitted_geometry() {
        assert_eq!(
            friction_speed_coefficient_s_per_m(-0.25, None).unwrap(),
            None
        );
    }

    #[test]
    fn coefficient_conversion_requires_a_nonzero_evidence_digest() {
        let geometry = QualifiedSlopeGeometry {
            gravity_m_s2: 9.81,
            effective_incline_angle_rad: 0.5,
            evidence_sha256: [0; 32],
        };
        assert_eq!(
            friction_speed_coefficient_s_per_m(-0.25, Some(geometry)),
            Err(ModelDiscriminationError::InvalidGeometry)
        );
    }

    #[test]
    fn qualified_geometry_maps_slope_with_declared_units() {
        let mut digest = [0; 32];
        digest[0] = 1;
        let geometry = QualifiedSlopeGeometry {
            gravity_m_s2: 9.81,
            effective_incline_angle_rad: 0.5,
            evidence_sha256: digest,
        };
        let coefficient = friction_speed_coefficient_s_per_m(-0.25, Some(geometry))
            .unwrap()
            .unwrap();
        let expected = 0.25 / (9.81 * 0.5_f64.cos());
        assert!((coefficient - expected).abs() < 1.0e-12);
    }

    #[test]
    fn input_permutation_preserves_the_fit() {
        let original = speed_dependent_fixture();
        let permuted = [
            original[4],
            original[2],
            original[0],
            original[3],
            original[1],
        ];
        let a = compare_constant_vs_speed_dependent(&original).unwrap();
        let b = compare_constant_vs_speed_dependent(&permuted).unwrap();
        assert!(
            (a.alternative_slope_m_s2_per_m_s - b.alternative_slope_m_s2_per_m_s).abs() < 1.0e-12
        );
        assert!((a.delta_chi_squared - b.delta_chi_squared).abs() < 1.0e-10);
    }
}
