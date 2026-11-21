//! GAUGE-specific local identifiability reference for slope-sliding experiments.
//!
//! The idealized model separates sliding friction from a constant incline-angle
//! registration bias. It uses an explicitly scaled local Jacobian and a
//! diagonal, independent acceleration-noise model. This is a design diagnostic,
//! not a parameter estimator, posterior, or validation of the GAUGE dataset.

use std::error::Error;
use std::fmt::{Display, Formatter};

pub const DEFAULT_RANK_TOLERANCE: f64 = 1.0e-10;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParameterScales {
    /// Characteristic change in the friction coefficient represented by one
    /// unit of the scaled friction parameter.
    pub friction_coefficient: f64,
    /// Characteristic change in the angle-registration bias, in radians,
    /// represented by one unit of the scaled angle parameter.
    pub angle_bias_rad: f64,
}

impl ParameterScales {
    pub const UNIT: Self = Self {
        friction_coefficient: 1.0,
        angle_bias_rad: 1.0,
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlopeModel {
    pub friction_coefficient: f64,
    pub angle_bias_rad: f64,
    pub gravity_m_s2: f64,
    pub parameter_scales: ParameterScales,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlopeExcitation {
    /// Declared physical incline angle for this experiment, in radians.
    pub incline_angle_rad: f64,
    /// One-standard-deviation measurement noise for inferred downslope
    /// acceleration, in m/s^2.
    pub acceleration_sigma_m_s2: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentifiabilityError {
    EmptyDesign,
    FewerThanTwoCandidates,
    InvalidFrictionCoefficient,
    InvalidAngleBias,
    InvalidGravity,
    InvalidParameterScale,
    InvalidInclineAngle,
    InvalidAccelerationUncertainty,
    NoFullRankCandidatePair,
    InvalidRankTolerance,
    NonFiniteInformation,
}

impl Display for IdentifiabilityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyDesign => write!(formatter, "experiment design must not be empty"),
            Self::FewerThanTwoCandidates => {
                write!(formatter, "at least two candidate experiments are required")
            }
            Self::InvalidFrictionCoefficient => {
                write!(formatter, "friction coefficient must be finite and non-negative")
            }
            Self::InvalidAngleBias => write!(formatter, "angle bias must be finite"),
            Self::InvalidGravity => write!(formatter, "gravity must be finite and positive"),
            Self::InvalidParameterScale => {
                write!(formatter, "parameter scales must be finite and positive")
            }
            Self::InvalidInclineAngle => {
                write!(formatter, "incline plus angle bias must lie strictly inside (-pi/2, pi/2)")
            }
            Self::InvalidAccelerationUncertainty => {
                write!(formatter, "acceleration uncertainty must be finite and positive")
            }
            Self::NoFullRankCandidatePair => {
                write!(formatter, "no candidate pair produces full-rank local information")
            }
            Self::InvalidRankTolerance => {
                write!(formatter, "rank tolerance must be finite and positive")
            }
            Self::NonFiniteInformation => write!(formatter, "information matrix is non-finite"),
        }
    }
}

impl Error for IdentifiabilityError {}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FisherInformation2 {
    /// F[mu, mu] for the scaled parameter coordinates.
    pub friction_friction: f64,
    /// Symmetric off-diagonal term F[mu, delta].
    pub friction_angle_bias: f64,
    /// F[delta, delta] for the scaled parameter coordinates.
    pub angle_bias_angle_bias: f64,
}

impl FisherInformation2 {
    pub fn determinant(self) -> f64 {
        self.friction_friction * self.angle_bias_angle_bias
            - self.friction_angle_bias * self.friction_angle_bias
    }

    pub fn trace(self) -> f64 {
        self.friction_friction + self.angle_bias_angle_bias
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InformationSummary {
    pub fisher: FisherInformation2,
    /// Eigenvalues of the 2x2 Fisher information matrix, ascending.
    pub minimum_eigenvalue: f64,
    pub maximum_eigenvalue: f64,
    /// Singular values of the weighted, scaled sensitivity Jacobian.
    pub minimum_singular_value: f64,
    pub maximum_singular_value: f64,
    pub effective_rank: u8,
    pub condition_number: Option<f64>,
    /// Natural log determinant exists only for numerically full-rank designs.
    pub log_determinant: Option<f64>,
    pub experiment_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DOptimalPair {
    pub first_candidate_index: usize,
    pub second_candidate_index: usize,
    pub information: InformationSummary,
}

/// Predict downslope acceleration for an ideal rigid block with Coulomb
/// sliding friction. This is a deliberately simplified analytic design model.
pub fn predict_downslope_acceleration(
    model: SlopeModel,
    excitation: SlopeExcitation,
) -> Result<f64, IdentifiabilityError> {
    validate_model(model)?;
    validate_excitation(model, excitation)?;

    let phi = excitation.incline_angle_rad + model.angle_bias_rad;
    let acceleration = model.gravity_m_s2
        * (phi.sin() - model.friction_coefficient * phi.cos());
    if acceleration.is_finite() {
        Ok(acceleration)
    } else {
        Err(IdentifiabilityError::NonFiniteInformation)
    }
}

/// Return the local sensitivity row for the scaled coordinates:
/// [mu / friction_scale, delta / angle_bias_scale].
///
/// The physical partial derivatives are:
/// da/dmu = -g cos(phi)
/// da/ddelta = g (cos(phi) + mu sin(phi))
pub fn scaled_sensitivity(
    model: SlopeModel,
    excitation: SlopeExcitation,
) -> Result<[f64; 2], IdentifiabilityError> {
    validate_model(model)?;
    validate_excitation(model, excitation)?;

    let phi = excitation.incline_angle_rad + model.angle_bias_rad;
    let derivative_friction = -model.gravity_m_s2 * phi.cos();
    let derivative_angle = model.gravity_m_s2
        * (phi.cos() + model.friction_coefficient * phi.sin());
    let row = [
        derivative_friction * model.parameter_scales.friction_coefficient,
        derivative_angle * model.parameter_scales.angle_bias_rad,
    ];
    if row.iter().all(|value| value.is_finite()) {
        Ok(row)
    } else {
        Err(IdentifiabilityError::NonFiniteInformation)
    }
}

/// Calculate F = J^T Sigma^-1 J for independent acceleration observations.
///
/// Scaling is explicit through SlopeModel::parameter_scales. This function
/// computes local information only; it does not claim global identifiability.
pub fn fisher_information(
    model: SlopeModel,
    experiments: &[SlopeExcitation],
) -> Result<InformationSummary, IdentifiabilityError> {
    if experiments.is_empty() {
        return Err(IdentifiabilityError::EmptyDesign);
    }
    validate_model(model)?;

    let mut fisher = FisherInformation2::default();
    for experiment in experiments {
        validate_excitation(model, *experiment)?;
        let row = scaled_sensitivity(model, *experiment)?;
        let inverse_variance =
            1.0 / (experiment.acceleration_sigma_m_s2 * experiment.acceleration_sigma_m_s2);
        fisher.friction_friction += inverse_variance * row[0] * row[0];
        fisher.friction_angle_bias += inverse_variance * row[0] * row[1];
        fisher.angle_bias_angle_bias += inverse_variance * row[1] * row[1];
    }

    summarize(fisher, experiments.len(), DEFAULT_RANK_TOLERANCE)
}

/// Select the pair with maximum D-optimal log(det(F)) from a declared finite
/// candidate set. Every candidate contributes exactly one trial; all pair
/// comparisons therefore use an equal two-trial measurement budget.
pub fn select_d_optimal_pair(
    model: SlopeModel,
    candidates: &[SlopeExcitation],
) -> Result<DOptimalPair, IdentifiabilityError> {
    if candidates.len() < 2 {
        return Err(IdentifiabilityError::FewerThanTwoCandidates);
    }
    validate_model(model)?;

    let mut best: Option<DOptimalPair> = None;
    for first in 0..candidates.len() {
        for second in (first + 1)..candidates.len() {
            let pair = [candidates[first], candidates[second]];
            let information = fisher_information(model, &pair)?;
            let Some(log_determinant) = information.log_determinant else {
                continue;
            };

            let replace_best = match best {
                None => true,
                Some(current) => {
                    let current_score = current
                        .information
                        .log_determinant
                        .expect("stored pair is full rank");
                    log_determinant > current_score + 1.0e-12
                }
            };
            if replace_best {
                best = Some(DOptimalPair {
                    first_candidate_index: first,
                    second_candidate_index: second,
                    information,
                });
            }
        }
    }

    best.ok_or(IdentifiabilityError::NoFullRankCandidatePair)
}

pub fn summarize(
    fisher: FisherInformation2,
    experiment_count: usize,
    rank_tolerance: f64,
) -> Result<InformationSummary, IdentifiabilityError> {
    if !rank_tolerance.is_finite() || rank_tolerance <= 0.0 {
        return Err(IdentifiabilityError::InvalidRankTolerance);
    }
    let entries = [
        fisher.friction_friction,
        fisher.friction_angle_bias,
        fisher.angle_bias_angle_bias,
    ];
    if entries.iter().any(|value| !value.is_finite()) {
        return Err(IdentifiabilityError::NonFiniteInformation);
    }

    // For a symmetric 2x2 matrix, the eigenvalue gap is stable via hypot.
    let trace = fisher.trace();
    let gap = (fisher.friction_friction - fisher.angle_bias_angle_bias)
        .hypot(2.0 * fisher.friction_angle_bias);
    let maximum_eigenvalue = 0.5 * (trace + gap);
    if maximum_eigenvalue < 0.0 {
        return Err(IdentifiabilityError::NonFiniteInformation);
    }

    // Compute the smaller eigenvalue from det/lambda_max to reduce
    // cancellation for near-rank-one information matrices.
    let determinant = fisher.determinant();
    let mut minimum_eigenvalue = if maximum_eigenvalue > 0.0 {
        determinant / maximum_eigenvalue
    } else {
        0.0
    };

    // A Gram matrix is positive-semidefinite. Permit only roundoff-sized
    // negative values; reject a materially inconsistent matrix.
    if minimum_eigenvalue < 0.0 {
        if minimum_eigenvalue.abs() <= rank_tolerance * maximum_eigenvalue.max(1.0) {
            minimum_eigenvalue = 0.0;
        } else {
            return Err(IdentifiabilityError::NonFiniteInformation);
        }
    }

    let effective_rank = if maximum_eigenvalue <= 0.0 {
        0
    } else if minimum_eigenvalue <= rank_tolerance * maximum_eigenvalue {
        1
    } else {
        2
    };

    let condition_number = if effective_rank == 2 {
        Some(maximum_eigenvalue / minimum_eigenvalue)
    } else {
        None
    };
    let log_determinant = if effective_rank == 2 {
        Some(maximum_eigenvalue.ln() + minimum_eigenvalue.ln())
    } else {
        None
    };

    Ok(InformationSummary {
        fisher,
        minimum_eigenvalue,
        maximum_eigenvalue,
        minimum_singular_value: minimum_eigenvalue.sqrt(),
        maximum_singular_value: maximum_eigenvalue.sqrt(),
        effective_rank,
        condition_number,
        log_determinant,
        experiment_count,
    })
}

fn validate_model(model: SlopeModel) -> Result<(), IdentifiabilityError> {
    if !model.friction_coefficient.is_finite() || model.friction_coefficient < 0.0 {
        return Err(IdentifiabilityError::InvalidFrictionCoefficient);
    }
    if !model.angle_bias_rad.is_finite() {
        return Err(IdentifiabilityError::InvalidAngleBias);
    }
    if !model.gravity_m_s2.is_finite() || model.gravity_m_s2 <= 0.0 {
        return Err(IdentifiabilityError::InvalidGravity);
    }
    let scales = model.parameter_scales;
    if !scales.friction_coefficient.is_finite()
        || scales.friction_coefficient <= 0.0
        || !scales.angle_bias_rad.is_finite()
        || scales.angle_bias_rad <= 0.0
    {
        return Err(IdentifiabilityError::InvalidParameterScale);
    }
    Ok(())
}

fn validate_excitation(
    model: SlopeModel,
    excitation: SlopeExcitation,
) -> Result<(), IdentifiabilityError> {
    let phi = excitation.incline_angle_rad + model.angle_bias_rad;
    if !excitation.incline_angle_rad.is_finite()
        || !phi.is_finite()
        || phi <= -std::f64::consts::FRAC_PI_2
        || phi >= std::f64::consts::FRAC_PI_2
    {
        return Err(IdentifiabilityError::InvalidInclineAngle);
    }
    if !excitation.acceleration_sigma_m_s2.is_finite()
        || excitation.acceleration_sigma_m_s2 <= 0.0
    {
        return Err(IdentifiabilityError::InvalidAccelerationUncertainty);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn model() -> SlopeModel {
        SlopeModel {
            friction_coefficient: 0.25,
            angle_bias_rad: 0.02,
            gravity_m_s2: 9.81,
            parameter_scales: ParameterScales::UNIT,
        }
    }

    fn excitation(degrees: f64, sigma: f64) -> SlopeExcitation {
        SlopeExcitation {
            incline_angle_rad: degrees.to_radians(),
            acceleration_sigma_m_s2: sigma,
        }
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual:.12e}, expected {expected:.12e}, tolerance {tolerance:.3e}"
        );
    }

    #[test]
    fn known_answer_sensitivity_at_thirty_degrees() {
        let row = scaled_sensitivity(model(), excitation(30.0, 0.05))
            .expect("valid sensitivity");
        close(row[0], -8.39591667, 1.0e-7);
        close(row[1], 9.66439714, 1.0e-7);
    }

    #[test]
    fn repeated_measurements_at_one_angle_remain_rank_one() {
        let repeated = vec![excitation(30.0, 0.05); 12];
        let information = fisher_information(model(), &repeated).expect("valid repeated design");
        assert_eq!(information.effective_rank, 1);
        assert_eq!(information.experiment_count, 12);
        assert_eq!(information.log_determinant, None);
        assert_eq!(information.condition_number, None);
    }

    #[test]
    fn two_well_separated_inclinations_produce_full_local_rank() {
        let information = fisher_information(
            model(),
            &[excitation(5.0, 0.05), excitation(75.0, 0.05)],
        )
        .expect("valid diverse design");
        assert_eq!(information.effective_rank, 2);
        assert!(information.minimum_eigenvalue > 0.0);
        assert!(information.condition_number.unwrap() < 20.0);
        assert!(information.log_determinant.is_some());
    }

    #[test]
    fn nearly_identical_inclinations_are_full_rank_but_ill_conditioned() {
        let information = fisher_information(
            model(),
            &[excitation(30.0, 0.05), excitation(30.001, 0.05)],
        )
        .expect("valid near-duplicate design");
        assert_eq!(information.effective_rank, 2);
        assert!(information.condition_number.unwrap() > 500_000.0);
    }

    #[test]
    fn d_optimal_pair_selects_from_the_declared_equal_budget_candidates() {
        let candidates = [
            excitation(30.0, 0.05),
            excitation(5.0, 0.05),
            excitation(75.0, 0.05),
            excitation(40.0, 0.05),
        ];
        let selected = select_d_optimal_pair(model(), &candidates)
            .expect("at least one full-rank candidate pair");
        assert_eq!(selected.information.experiment_count, 2);
        assert!(selected.first_candidate_index < selected.second_candidate_index);
        assert_ne!(
            candidates[selected.first_candidate_index].incline_angle_rad,
            candidates[selected.second_candidate_index].incline_angle_rad
        );

        let selected_score = selected.information.log_determinant.unwrap();
        for first in 0..candidates.len() {
            for second in (first + 1)..candidates.len() {
                let score = fisher_information(
                    model(),
                    &[candidates[first], candidates[second]],
                )
                .expect("candidate pair valid")
                .log_determinant;
                if let Some(score) = score {
                    assert!(selected_score + 1.0e-12 >= score);
                }
            }
        }
    }

    #[test]
    fn doubling_acceleration_noise_reduces_information_by_four() {
        let single_noise =
            fisher_information(model(), &[excitation(5.0, 0.05), excitation(75.0, 0.05)])
                .expect("valid design");
        let double_noise =
            fisher_information(model(), &[excitation(5.0, 0.10), excitation(75.0, 0.10)])
                .expect("valid design");

        close(
            double_noise.fisher.friction_friction,
            single_noise.fisher.friction_friction / 4.0,
            1.0e-9,
        );
        close(
            double_noise.fisher.friction_angle_bias,
            single_noise.fisher.friction_angle_bias / 4.0,
            1.0e-9,
        );
        close(
            double_noise.fisher.angle_bias_angle_bias,
            single_noise.fisher.angle_bias_angle_bias / 4.0,
            1.0e-9,
        );
    }

    #[test]
    fn input_order_does_not_change_information() {
        let first = excitation(5.0, 0.05);
        let second = excitation(75.0, 0.05);
        let a = fisher_information(model(), &[first, second]).expect("valid design");
        let b = fisher_information(model(), &[second, first]).expect("valid design");
        close(a.fisher.friction_friction, b.fisher.friction_friction, 1.0e-10);
        close(a.fisher.friction_angle_bias, b.fisher.friction_angle_bias, 1.0e-10);
        close(
            a.fisher.angle_bias_angle_bias,
            b.fisher.angle_bias_angle_bias,
            1.0e-10,
        );
        assert_eq!(a.effective_rank, b.effective_rank);
    }

    #[test]
    fn parameter_scales_change_information_in_declared_coordinates() {
        let mut scaled = model();
        scaled.parameter_scales = ParameterScales {
            friction_coefficient: 0.1,
            angle_bias_rad: 0.01,
        };
        let unit = fisher_information(scaled, &[excitation(5.0, 0.05), excitation(75.0, 0.05)])
            .expect("valid scaled design");
        let mut unscaled = scaled;
        unscaled.parameter_scales = ParameterScales::UNIT;
        let raw = fisher_information(
            unscaled,
            &[excitation(5.0, 0.05), excitation(75.0, 0.05)],
        )
        .expect("valid unscaled design");
        close(
            unit.fisher.friction_friction,
            raw.fisher.friction_friction * 0.01,
            1.0e-9,
        );
        close(
            unit.fisher.angle_bias_angle_bias,
            raw.fisher.angle_bias_angle_bias * 0.0001,
            1.0e-9,
        );
    }

    #[test]
    fn rejects_invalid_physical_parameters_and_measurements() {
        let mut invalid = model();
        invalid.friction_coefficient = -0.1;
        assert_eq!(
            fisher_information(invalid, &[excitation(30.0, 0.05)]),
            Err(IdentifiabilityError::InvalidFrictionCoefficient)
        );

        invalid = model();
        invalid.gravity_m_s2 = 0.0;
        assert_eq!(
            fisher_information(invalid, &[excitation(30.0, 0.05)]),
            Err(IdentifiabilityError::InvalidGravity)
        );

        invalid = model();
        invalid.parameter_scales.angle_bias_rad = 0.0;
        assert_eq!(
            fisher_information(invalid, &[excitation(30.0, 0.05)]),
            Err(IdentifiabilityError::InvalidParameterScale)
        );

        assert_eq!(
            fisher_information(model(), &[excitation(30.0, 0.0)]),
            Err(IdentifiabilityError::InvalidAccelerationUncertainty)
        );
        assert_eq!(
            fisher_information(model(), &[excitation(90.0, 0.05)]),
            Err(IdentifiabilityError::InvalidInclineAngle)
        );
    }

    #[test]
    fn empty_and_non_identifying_candidate_sets_fail_closed() {
        assert_eq!(
            fisher_information(model(), &[]),
            Err(IdentifiabilityError::EmptyDesign)
        );
        assert_eq!(
            select_d_optimal_pair(model(), &[excitation(30.0, 0.05)]),
            Err(IdentifiabilityError::FewerThanTwoCandidates)
        );
        assert_eq!(
            select_d_optimal_pair(
                model(),
                &[excitation(30.0, 0.05), excitation(30.0, 0.05)],
            ),
            Err(IdentifiabilityError::NoFullRankCandidatePair)
        );
    }

    #[test]
    fn invalid_rank_tolerance_is_rejected() {
        assert_eq!(
            summarize(FisherInformation2::default(), 1, 0.0),
            Err(IdentifiabilityError::InvalidRankTolerance)
        );
        assert!(PI.is_finite());
    }
}
