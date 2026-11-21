//! Closed-form expected position error for a two-observation linearized model.
//!
//! The estimator uses nominal bearings while the data are generated using
//! scenario-perturbed true bearings. This separates model-mismatch bias from
//! measurement-noise variance. It is a local synthetic model, not a physical
//! range solver or a calibrated navigation integrity bound.

use std::collections::BTreeMap;
use std::fmt;

use crate::correlated::{correlated_fisher, CorrelatedRangeAction};
use crate::robust::PerturbationScenario;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExpectedPositionError {
    pub bias_squared_m2: f64,
    pub noise_variance_m2: f64,
    pub expected_squared_error_m2: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OutcomeError {
    WrongObservationCount,
    MissingPerturbation(String),
    InvalidPerturbation(String),
    SingularNominalGeometry,
    NonFiniteOutcome,
    CorrelatedModel(String),
}

impl fmt::Display for OutcomeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongObservationCount => write!(f, "exactly two observations are required for this closed-form 2D evaluator"),
            Self::MissingPerturbation(id) => write!(f, "scenario is missing action: {id}"),
            Self::InvalidPerturbation(id) => write!(f, "invalid perturbation for action: {id}"),
            Self::SingularNominalGeometry => write!(f, "nominal observation matrix is singular"),
            Self::NonFiniteOutcome => write!(f, "expected error calculation became non-finite"),
            Self::CorrelatedModel(message) => write!(f, "correlated measurement model invalid: {message}"),
        }
    }
}
impl std::error::Error for OutcomeError {}

/// Expected squared error for a square two-observation linearized geometry.
///
/// The nominal estimator uses H_nom^-1. The data are generated from H_true,
/// where each bearing and private sigma is perturbed by the supplied scenario.
/// Shared additive common-mode errors are included in the measurement
/// covariance. For two independent coordinates and exactly two full-rank
/// observations, generalized least squares reduces to this inverse mapping.
pub fn expected_two_observation_error(
    actions: &[CorrelatedRangeAction],
    scenario: &PerturbationScenario,
    true_position_m: [f64; 2],
) -> Result<ExpectedPositionError, OutcomeError> {
    if actions.len() != 2 {
        return Err(OutcomeError::WrongObservationCount);
    }
    if !true_position_m.iter().all(|value| value.is_finite()) {
        return Err(OutcomeError::NonFiniteOutcome);
    }
    correlated_fisher(actions).map_err(|error| OutcomeError::CorrelatedModel(error.to_string()))?;

    let mut perturbations = BTreeMap::new();
    for perturbation in &scenario.actions {
        if !perturbation.bearing_delta_rad.is_finite()
            || !perturbation.private_sigma_scale.is_finite()
            || perturbation.private_sigma_scale <= 0.0
            || perturbations.insert(perturbation.action_id.clone(), perturbation).is_some()
        {
            return Err(OutcomeError::InvalidPerturbation(perturbation.action_id.clone()));
        }
    }

    let mut nominal_h = [[0.0; 2]; 2];
    let mut true_h = [[0.0; 2]; 2];
    let mut sigma = [0.0; 2];
    let mut common_groups: [Option<(&str, f64)>; 2] = [None, None];

    for index in 0..2 {
        let action = &actions[index];
        let perturbation = perturbations
            .get(&action.id)
            .ok_or_else(|| OutcomeError::MissingPerturbation(action.id.clone()))?;
        let true_bearing = action.line_of_sight_rad + perturbation.bearing_delta_rad;
        sigma[index] = action.independent_sigma_m * perturbation.private_sigma_scale;
        if !true_bearing.is_finite() || !sigma[index].is_finite() || sigma[index] <= 0.0 {
            return Err(OutcomeError::InvalidPerturbation(action.id.clone()));
        }
        nominal_h[index] = [action.line_of_sight_rad.cos(), action.line_of_sight_rad.sin()];
        true_h[index] = [true_bearing.cos(), true_bearing.sin()];
        if let (Some(group), Some(common_sigma)) =
            (&action.common_mode_group, action.common_mode_sigma_m)
        {
            common_groups[index] = Some((group.as_str(), common_sigma));
        }
    }

    let determinant = nominal_h[0][0] * nominal_h[1][1]
        - nominal_h[0][1] * nominal_h[1][0];
    if !determinant.is_finite() || determinant.abs() <= 1.0e-12 {
        return Err(OutcomeError::SingularNominalGeometry);
    }

    let inverse = [
        [nominal_h[1][1] / determinant, -nominal_h[0][1] / determinant],
        [-nominal_h[1][0] / determinant, nominal_h[0][0] / determinant],
    ];

    let observed_mean = [
        true_h[0][0] * true_position_m[0] + true_h[0][1] * true_position_m[1],
        true_h[1][0] * true_position_m[0] + true_h[1][1] * true_position_m[1],
    ];
    let estimated_mean = [
        inverse[0][0] * observed_mean[0] + inverse[0][1] * observed_mean[1],
        inverse[1][0] * observed_mean[0] + inverse[1][1] * observed_mean[1],
    ];
    let bias = [
        estimated_mean[0] - true_position_m[0],
        estimated_mean[1] - true_position_m[1],
    ];
    let bias_squared_m2 = bias[0] * bias[0] + bias[1] * bias[1];

    let mut r = [[sigma[0] * sigma[0], 0.0], [0.0, sigma[1] * sigma[1]]];
    if let (Some((group_a, tau_a)), Some((group_b, tau_b))) =
        (common_groups[0], common_groups[1])
    {
        if group_a == group_b {
            if (tau_a - tau_b).abs() > 1.0e-12 {
                return Err(OutcomeError::CorrelatedModel(
                    "same common-mode group has inconsistent sigma".to_owned(),
                ));
            }
            let variance = tau_a * tau_a;
            r[0][0] += variance;
            r[1][1] += variance;
            r[0][1] += variance;
            r[1][0] += variance;
    }

    let c00 = inverse[0][0] * inverse[0][0] * r[0][0]
        + 2.0 * inverse[0][0] * inverse[0][1] * r[0][1]
        + inverse[0][1] * inverse[0][1] * r[1][1];
    let c11 = inverse[1][0] * inverse[1][0] * r[0][0]
        + 2.0 * inverse[1][0] * inverse[1][1] * r[0][1]
        + inverse[1][1] * inverse[1][1] * r[1][1];
    let noise_variance_m2 = c00 + c11;
    let expected_squared_error_m2 = bias_squared_m2 + noise_variance_m2;

    if !bias_squared_m2.is_finite()
        || !noise_variance_m2.is_finite()
        || !expected_squared_error_m2.is_finite()
    {
        return Err(OutcomeError::NonFiniteOutcome);
    }

    Ok(ExpectedPositionError {
        bias_squared_m2,
        noise_variance_m2,
        expected_squared_error_m2,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: &str, delta: f64) -> crate::robust::ActionPerturbation {
        crate::robust::ActionPerturbation {
            action_id: id.to_owned(),
            bearing_delta_rad: delta,
            private_sigma_scale: 1.0,
        }
    }

    #[test]
    fn robust_geometry_improves_expected_error_under_bearing_miscalibration() {
        let fragile = vec![
            CorrelatedRangeAction::independent("fragile-east", 0.0, 0.2, 1),
            CorrelatedRangeAction::independent("fragile-north", std::f64::consts::FRAC_PI_2, 0.2, 1),
        ];
        let stable = vec![
            CorrelatedRangeAction::independent("stable-diagonal", std::f64::consts::FRAC_PI_4, 0.205, 1),
            CorrelatedRangeAction::independent("stable-antidiagonal", 3.0 * std::f64::consts::FRAC_PI_4, 0.205, 1),
        ];
        let scenario = PerturbationScenario {
            scenario_id: "adverse-bearing-error".to_owned(),
            actions: vec![
                p("fragile-east", 0.35),
                p("fragile-north", -0.35),
                p("stable-diagonal", 0.01),
                p("stable-antidiagonal", -0.01),
            ],
        };
        let truth = [10.0, -5.0];
        let fragile_error = expected_two_observation_error(&fragile, &scenario, truth).unwrap();
        let stable_error = expected_two_observation_error(&stable, &scenario, truth).unwrap();

        let fragile_nominal_fim = correlated_fisher(&fragile).unwrap();
        let stable_nominal_fim = correlated_fisher(&stable).unwrap();
        let fragile_a = fragile_nominal_fim.a_optimal_trace_inverse(1.0e-10).unwrap();
        let stable_a = stable_nominal_fim.a_optimal_trace_inverse(1.0e-10).unwrap();

        assert!(stable_a <= fragile_a * 1.06);
        assert!(stable_error.expected_squared_error_m2 < fragile_error.expected_squared_error_m2 * 0.01);
        assert!(fragile_error.bias_squared_m2 > stable_error.bias_squared_m2);
    }

    #[test]
    fn common_mode_noise_increases_realized_error_variance() {
        let independent = vec![
            CorrelatedRangeAction::independent("a", 0.0, 0.5, 1),
            CorrelatedRangeAction::independent("b", std::f64::consts::FRAC_PI_2, 0.5, 1),
        ];
        let correlated = vec![
            CorrelatedRangeAction::with_common_mode("a", 0.0, 0.5, "shared-clock", 0.8, 1),
            CorrelatedRangeAction::with_common_mode("b", std::f64::consts::FRAC_PI_2, 0.5, "shared-clock", 0.8, 1),
        ];
        let scenario = PerturbationScenario {
            scenario_id: "nominal".to_owned(),
            actions: vec![p("a", 0.0), p("b", 0.0)],
        };
        let independent_error = expected_two_observation_error(&independent, &scenario, [1.0, 2.0]).unwrap();
        let correlated_error = expected_two_observation_error(&correlated, &scenario, [1.0, 2.0]).unwrap();
        assert!(correlated_error.noise_variance_m2 > independent_error.noise_variance_m2);
    }
}
