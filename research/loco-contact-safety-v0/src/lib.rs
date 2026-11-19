//! Marginal-Gaussian, Bonferroni chance bounds for contact-aware actions.
//!
//! This is a one-step screening oracle. It does not prove forward invariance or
//! certify a physical robot. The probability interpretation is conditional on
//! correctly calibrated Gaussian predictive marginals for the declared stratum.

pub const CONSTRAINT_COUNT: usize = 5;
pub const MIN_SUPPORTED_ALPHA: f64 = 1.0e-12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConstraintKind {
    EnergyUpper,
    SupportMarginLower,
    PeakNormalForceUpper,
    ImpactImpulseUpper,
    ActuatorLoadFractionUpper,
}

const CANONICAL_CONSTRAINTS: [ConstraintKind; CONSTRAINT_COUNT] = [
    ConstraintKind::EnergyUpper,
    ConstraintKind::SupportMarginLower,
    ConstraintKind::PeakNormalForceUpper,
    ConstraintKind::ImpactImpulseUpper,
    ConstraintKind::ActuatorLoadFractionUpper,
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstraintPrediction {
    pub kind: ConstraintKind,
    pub mean: f64,
    pub variance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SafetyLimits {
    pub maximum_energy_joules: f64,
    pub minimum_support_margin: f64,
    pub maximum_peak_normal_force_newtons: f64,
    pub maximum_impact_impulse_newton_seconds: f64,
    pub maximum_actuator_load_fraction: f64,
}

impl SafetyLimits {
    fn validate(self) -> Result<(), ChanceSafetyError> {
        if !self.maximum_energy_joules.is_finite()
            || self.maximum_energy_joules <= 0.0
            || !self.minimum_support_margin.is_finite()
            || !self.maximum_peak_normal_force_newtons.is_finite()
            || self.maximum_peak_normal_force_newtons <= 0.0
            || !self.maximum_impact_impulse_newton_seconds.is_finite()
            || self.maximum_impact_impulse_newton_seconds <= 0.0
            || !self.maximum_actuator_load_fraction.is_finite()
            || !(0.0..=1.0).contains(&self.maximum_actuator_load_fraction)
        {
            return Err(ChanceSafetyError::InvalidSafetyLimits);
        }
        Ok(())
    }

    fn ordered_values(self) -> [f64; CONSTRAINT_COUNT] {
        [
            self.maximum_energy_joules,
            self.minimum_support_margin,
            self.maximum_peak_normal_force_newtons,
            self.maximum_impact_impulse_newton_seconds,
            self.maximum_actuator_load_fraction,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationStatus {
    CalibratedForDeclaredStratum,
    Uncalibrated,
    OutOfDistribution,
    CalibrationExpired,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PredictionProvenance {
    pub predictor_id: String,
    pub model_digest: String,
    pub calibration_id: String,
    pub calibration_stratum_id: String,
    pub status: CalibrationStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionRiskPrediction {
    pub action_id: String,
    pub expected_information_gain_nats: f64,
    pub provenance: PredictionProvenance,
    /// Must follow CANONICAL_CONSTRAINTS order. A reordered or duplicated
    /// constraint set is rejected rather than silently remapped.
    pub constraints: [ConstraintPrediction; CONSTRAINT_COUNT],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstraintAssessment {
    pub kind: ConstraintKind,
    pub mean: f64,
    pub standard_deviation: f64,
    pub conservative_bound: f64,
    pub limit: f64,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChanceAssessment {
    pub action_id: String,
    pub expected_information_gain_nats: f64,
    pub familywise_alpha: f64,
    pub per_constraint_alpha: f64,
    pub gaussian_quantile: f64,
    pub admitted: bool,
    /// Some(alpha) only when every constraint passed under a calibrated input.
    /// None means the risk budget was not earned.
    pub familywise_violation_probability_upper_bound: Option<f64>,
    pub constraints: [ConstraintAssessment; CONSTRAINT_COUNT],
    pub predictor_id: String,
    pub model_digest: String,
    pub calibration_id: String,
    pub calibration_stratum_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChanceSafetyError {
    EmptyIdentifier,
    UncalibratedPrediction,
    OutOfDistributionPrediction,
    ExpiredCalibration,
    InvalidRiskBudget,
    InvalidProbability,
    InvalidSafetyLimits,
    InvalidInformationGain,
    InvalidPredictionMean,
    InvalidPredictionVariance,
    ConstraintOrderMismatch,
    NumericalFailure,
}

/// Inverse standard-normal CDF using the Acklam rational approximation.
/// The supported open probability interval avoids endpoint overflow and loss
/// of significance in 1-p. The approximation is validated by known-answer tests.
pub fn standard_normal_quantile(probability: f64) -> Result<f64, ChanceSafetyError> {
    if !probability.is_finite() || !(0.0..1.0).contains(&probability) {
        return Err(ChanceSafetyError::InvalidProbability);
    }

    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];

    let lower_tail = 0.024_25;
    let upper_tail = 1.0 - lower_tail;

    let quantile = if probability < lower_tail {
        let q = (-2.0 * probability.ln()).sqrt();
        let numerator = (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q) + C[5];
        let denominator = (((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0;
        numerator / denominator
    } else if probability <= upper_tail {
        let q = probability - 0.5;
        let r = q * q;
        let numerator = (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q;
        let denominator = (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r) + 1.0;
        numerator / denominator
    } else {
        let q = (-2.0 * (1.0 - probability).ln()).sqrt();
        let numerator = (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q) + C[5];
        let denominator = (((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0;
        -numerator / denominator
    };

    if quantile.is_finite() {
        Ok(quantile)
    } else {
        Err(ChanceSafetyError::NumericalFailure)
    }
}

/// Assess one candidate. Returns an explicit error instead of a probability
/// guarantee if model/calibration provenance is missing or invalid.
pub fn assess_action(
    prediction: &ActionRiskPrediction,
    limits: SafetyLimits,
    familywise_alpha: f64,
) -> Result<ChanceAssessment, ChanceSafetyError> {
    limits.validate()?;
    validate_provenance(&prediction.provenance)?;
    if prediction.action_id.trim().is_empty() {
        return Err(ChanceSafetyError::EmptyIdentifier);
    }
    if !prediction.expected_information_gain_nats.is_finite()
        || prediction.expected_information_gain_nats < 0.0
    {
        return Err(ChanceSafetyError::InvalidInformationGain);
    }
    if !familywise_alpha.is_finite() || !(MIN_SUPPORTED_ALPHA..1.0).contains(&familywise_alpha) {
        return Err(ChanceSafetyError::InvalidRiskBudget);
    }

    for (prediction_constraint, expected_kind) in
        prediction.constraints.iter().zip(CANONICAL_CONSTRAINTS)
    {
        if prediction_constraint.kind != expected_kind {
            return Err(ChanceSafetyError::ConstraintOrderMismatch);
        }
        if !prediction_constraint.mean.is_finite() {
            return Err(ChanceSafetyError::InvalidPredictionMean);
        }
        if !prediction_constraint.variance.is_finite() || prediction_constraint.variance < 0.0 {
            return Err(ChanceSafetyError::InvalidPredictionVariance);
        }
    }

    let per_constraint_alpha = familywise_alpha / CONSTRAINT_COUNT as f64;
    let gaussian_quantile = standard_normal_quantile(1.0 - per_constraint_alpha)?;
    if !gaussian_quantile.is_finite() || gaussian_quantile <= 0.0 {
        return Err(ChanceSafetyError::NumericalFailure);
    }

    let limits = limits.ordered_values();
    let mut assessments = [ConstraintAssessment {
        kind: ConstraintKind::EnergyUpper,
        mean: 0.0,
        standard_deviation: 0.0,
        conservative_bound: 0.0,
        limit: 0.0,
        passed: false,
    }; CONSTRAINT_COUNT];

    let mut all_passed = true;
    for (index, (prediction_constraint, limit)) in
        prediction.constraints.iter().zip(limits).enumerate()
    {
        let standard_deviation = prediction_constraint.variance.sqrt();
        let is_lower_bound = prediction_constraint.kind == ConstraintKind::SupportMarginLower;
        let conservative_bound = if is_lower_bound {
            prediction_constraint.mean - gaussian_quantile * standard_deviation
        } else {
            prediction_constraint.mean + gaussian_quantile * standard_deviation
        };
        if !conservative_bound.is_finite() {
            return Err(ChanceSafetyError::NumericalFailure);
        }
        let passed = if is_lower_bound {
            conservative_bound >= limit
        } else {
            conservative_bound <= limit
        };
        all_passed &= passed;
        assessments[index] = ConstraintAssessment {
            kind: prediction_constraint.kind,
            mean: prediction_constraint.mean,
            standard_deviation,
            conservative_bound,
            limit,
            passed,
        };
    }

    Ok(ChanceAssessment {
        action_id: prediction.action_id.clone(),
        expected_information_gain_nats: prediction.expected_information_gain_nats,
        familywise_alpha,
        per_constraint_alpha,
        gaussian_quantile,
        admitted: all_passed,
        familywise_violation_probability_upper_bound: all_passed.then_some(familywise_alpha),
        constraints: assessments,
        predictor_id: prediction.provenance.predictor_id.clone(),
        model_digest: prediction.provenance.model_digest.clone(),
        calibration_id: prediction.provenance.calibration_id.clone(),
        calibration_stratum_id: prediction.provenance.calibration_stratum_id.clone(),
    })
}

fn validate_provenance(provenance: &PredictionProvenance) -> Result<(), ChanceSafetyError> {
    if provenance.predictor_id.trim().is_empty()
        || provenance.model_digest.trim().is_empty()
        || provenance.calibration_id.trim().is_empty()
        || provenance.calibration_stratum_id.trim().is_empty()
    {
        return Err(ChanceSafetyError::EmptyIdentifier);
    }
    match provenance.status {
        CalibrationStatus::CalibratedForDeclaredStratum => Ok(()),
        CalibrationStatus::Uncalibrated => Err(ChanceSafetyError::UncalibratedPrediction),
        CalibrationStatus::OutOfDistribution => Err(ChanceSafetyError::OutOfDistributionPrediction),
        CalibrationStatus::CalibrationExpired => Err(ChanceSafetyError::ExpiredCalibration),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> SafetyLimits {
        SafetyLimits {
            maximum_energy_joules: 5.0,
            minimum_support_margin: 0.05,
            maximum_peak_normal_force_newtons: 20.0,
            maximum_impact_impulse_newton_seconds: 2.0,
            maximum_actuator_load_fraction: 0.9,
        }
    }

    fn prediction(
        action_id: &str,
        means: [f64; CONSTRAINT_COUNT],
        variances: [f64; CONSTRAINT_COUNT],
        information_gain: f64,
    ) -> ActionRiskPrediction {
        ActionRiskPrediction {
            action_id: action_id.to_owned(),
            expected_information_gain_nats: information_gain,
            provenance: PredictionProvenance {
                predictor_id: "contact-model-v1".to_owned(),
                model_digest: "sha256:model".to_owned(),
                calibration_id: "calibration-v4".to_owned(),
                calibration_stratum_id: "terrain-flat-speed-medium".to_owned(),
                status: CalibrationStatus::CalibratedForDeclaredStratum,
            },
            constraints: [
                ConstraintPrediction {
                    kind: ConstraintKind::EnergyUpper,
                    mean: means[0],
                    variance: variances[0],
                },
                ConstraintPrediction {
                    kind: ConstraintKind::SupportMarginLower,
                    mean: means[1],
                    variance: variances[1],
                },
                ConstraintPrediction {
                    kind: ConstraintKind::PeakNormalForceUpper,
                    mean: means[2],
                    variance: variances[2],
                },
                ConstraintPrediction {
                    kind: ConstraintKind::ImpactImpulseUpper,
                    mean: means[3],
                    variance: variances[3],
                },
                ConstraintPrediction {
                    kind: ConstraintKind::ActuatorLoadFractionUpper,
                    mean: means[4],
                    variance: variances[4],
                },
            ],
        }
    }

    #[test]
    fn normal_quantile_matches_known_one_sided_values() {
        let z_99 = standard_normal_quantile(0.99).unwrap();
        let z_998 = standard_normal_quantile(0.998).unwrap();
        assert!((z_99 - 2.326_347_874_040_840_8).abs() < 2.0e-8);
        assert!((z_998 - 2.878_161_739_095_483).abs() < 2.0e-8);
    }

    #[test]
    fn bonferroni_alpha_is_divided_across_five_constraints() {
        let action = prediction("safe", [1.0, 0.5, 5.0, 0.2, 0.3], [0.01; 5], 0.8);
        let assessment = assess_action(&action, limits(), 0.05).unwrap();
        assert!((assessment.per_constraint_alpha - 0.01).abs() < 1.0e-15);
        assert!((assessment.gaussian_quantile - 2.326_347_874_040_840_8).abs() < 2.0e-8);
        assert!(assessment.admitted);
        assert_eq!(
            assessment.familywise_violation_probability_upper_bound,
            Some(0.05)
        );
    }

    #[test]
    fn candidate_exactly_at_all_conservative_limits_is_admitted() {
        let alpha = 0.05;
        let z = standard_normal_quantile(1.0 - alpha / 5.0).unwrap();
        let sigma = 0.1;
        let action = prediction(
            "boundary",
            [
                limits().maximum_energy_joules - z * sigma,
                limits().minimum_support_margin + z * sigma,
                limits().maximum_peak_normal_force_newtons - z * sigma,
                limits().maximum_impact_impulse_newton_seconds - z * sigma,
                limits().maximum_actuator_load_fraction - z * sigma,
            ],
            [sigma * sigma; CONSTRAINT_COUNT],
            1.0,
        );
        let assessment = assess_action(&action, limits(), alpha).unwrap();
        assert!(assessment.admitted);
        assert!(
            assessment
                .constraints
                .iter()
                .all(|constraint| constraint.passed)
        );
    }

    #[test]
    fn each_individual_physical_constraint_can_reject_an_action() {
        let baseline = [1.0, 0.5, 5.0, 0.2, 0.3];
        for index in 0..CONSTRAINT_COUNT {
            let mut means = baseline;
            means[index] = match index {
                0 => 6.0,
                1 => 0.0,
                2 => 25.0,
                3 => 3.0,
                _ => 1.1,
            };
            let action = prediction("violating", means, [0.0; 5], 10.0);
            let assessment = assess_action(&action, limits(), 0.05).unwrap();
            assert!(!assessment.admitted, "constraint index {index}");
            assert_eq!(
                assessment.familywise_violation_probability_upper_bound,
                None
            );
            assert!(!assessment.constraints[index].passed);
        }
    }

    #[test]
    fn high_information_unsafe_candidate_is_rejected_before_ranking() {
        let high_eig_unsafe = prediction(
            "high-eig-unsafe",
            [1.0, -0.2, 5.0, 0.2, 0.3],
            [0.01; 5],
            100.0,
        );
        let lower_eig_safe =
            prediction("lower-eig-safe", [1.0, 0.5, 5.0, 0.2, 0.3], [0.01; 5], 0.1);
        let unsafe_result = assess_action(&high_eig_unsafe, limits(), 0.05).unwrap();
        let safe_result = assess_action(&lower_eig_safe, limits(), 0.05).unwrap();
        assert!(!unsafe_result.admitted);
        assert!(safe_result.admitted);
        assert!(
            unsafe_result.expected_information_gain_nats
                > safe_result.expected_information_gain_nats
        );
    }

    #[test]
    fn a_stricter_alpha_shrinks_the_admissible_set_at_a_boundary_case() {
        let action = prediction("uncertain", [4.75, 0.30, 10.0, 0.5, 0.5], [0.01; 5], 2.0);
        let loose = assess_action(&action, limits(), 0.10).unwrap();
        let strict = assess_action(&action, limits(), 0.01).unwrap();
        assert!(strict.gaussian_quantile > loose.gaussian_quantile);
        assert!(loose.admitted);
        assert!(!strict.admitted);
        for (strict_bound, loose_bound) in strict.constraints.iter().zip(loose.constraints.iter()) {
            match strict_bound.kind {
                ConstraintKind::SupportMarginLower => {
                    assert!(strict_bound.conservative_bound <= loose_bound.conservative_bound);
                }
                _ => {
                    assert!(strict_bound.conservative_bound >= loose_bound.conservative_bound);
                }
            }
        }
    }

    #[test]
    fn uncalibrated_out_of_distribution_and_expired_predictions_abstain() {
        for status in [
            CalibrationStatus::Uncalibrated,
            CalibrationStatus::OutOfDistribution,
            CalibrationStatus::CalibrationExpired,
        ] {
            let mut action = prediction("uncertain", [1.0, 0.5, 5.0, 0.2, 0.3], [0.0; 5], 0.2);
            action.provenance.status = status;
            let result = assess_action(&action, limits(), 0.05);
            assert!(result.is_err());
        }
    }

    #[test]
    fn zero_variance_reduces_to_deterministic_limit_check() {
        let safe = prediction("zero-var-safe", [1.0, 0.5, 5.0, 0.2, 0.3], [0.0; 5], 0.0);
        let unsafe_action = prediction("zero-var-unsafe", [6.0, 0.5, 5.0, 0.2, 0.3], [0.0; 5], 0.0);
        assert!(assess_action(&safe, limits(), 0.05).unwrap().admitted);
        assert!(
            !assess_action(&unsafe_action, limits(), 0.05)
                .unwrap()
                .admitted
        );
    }

    #[test]
    fn invalid_variance_alpha_and_identity_fail_closed() {
        let mut action = prediction("invalid", [1.0, 0.5, 5.0, 0.2, 0.3], [0.0; 5], 0.0);
        action.constraints[2].variance = -0.1;
        assert_eq!(
            assess_action(&action, limits(), 0.05),
            Err(ChanceSafetyError::InvalidPredictionVariance)
        );

        let action = prediction("invalid-alpha", [1.0, 0.5, 5.0, 0.2, 0.3], [0.0; 5], 0.0);
        assert_eq!(
            assess_action(&action, limits(), 0.0),
            Err(ChanceSafetyError::InvalidRiskBudget)
        );
        assert_eq!(
            assess_action(&action, limits(), 1.0),
            Err(ChanceSafetyError::InvalidRiskBudget)
        );
    }

    #[test]
    fn extreme_supported_risk_budgets_return_finite_quantiles() {
        let alpha = MIN_SUPPORTED_ALPHA;
        let z = standard_normal_quantile(1.0 - alpha / 5.0).unwrap();
        assert!(z.is_finite());
        assert!(z > 7.0);
    }

    #[test]
    fn reordered_constraints_are_rejected() {
        let mut action = prediction("reordered", [1.0, 0.5, 5.0, 0.2, 0.3], [0.0; 5], 0.0);
        action.constraints.swap(0, 1);
        assert_eq!(
            assess_action(&action, limits(), 0.05),
            Err(ChanceSafetyError::ConstraintOrderMismatch)
        );
    }
}
