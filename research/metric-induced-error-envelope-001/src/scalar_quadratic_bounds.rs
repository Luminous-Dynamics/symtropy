// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Interval-derived Newton-Kantorovich eta and gamma bounds for a scalar
//! quadratic F(x) = c0 + c1*x + c2*x^2/2 with linearization A = F'(x0).
//!
//! The supplied f64 coefficients are interpreted as exact binary64 values.
//! The interval engine encloses F(x0), F'(x0), A^-1 F(x0), and A^-1*c2.
//! This certifies only this supplied scalar polynomial, not an external model.

use std::fmt;

use crate::nonlinear_kkt_certificate::NewtonKantorovichInputsV1;
use crate::outward_interval::{IntervalErrorV1, IntervalV1};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScalarQuadraticBoundErrorV1 {
    EmptyEvidenceIdentity,
    NonFiniteCoefficient,
    NonFiniteCenter,
    DerivativeIntervalContainsZero,
    Interval(IntervalErrorV1),
}

impl fmt::Display for ScalarQuadraticBoundErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for ScalarQuadraticBoundErrorV1 {}
impl From<IntervalErrorV1> for ScalarQuadraticBoundErrorV1 {
    fn from(value: IntervalErrorV1) -> Self { Self::Interval(value) }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScalarQuadraticBoundEvidenceV1 {
    pub problem_id: String,
    pub linearization_id: String,
    pub norm_identity_id: String,
    pub polynomial_model_id: String,
    pub center: f64,
    pub function_value_at_center: IntervalV1,
    pub derivative_at_center: IntervalV1,
    pub inverse_derivative_at_center: IntervalV1,
    pub eta_interval: IntervalV1,
    pub eta_upper_bound: f64,
    pub second_derivative: IntervalV1,
    pub gamma_interval: IntervalV1,
    pub gamma_upper_bound: f64,
    pub eta_receipt_id: String,
    pub gamma_receipt_id: String,
}

impl ScalarQuadraticBoundEvidenceV1 {
    pub fn newton_kantorovich_inputs(&self) -> NewtonKantorovichInputsV1 {
        NewtonKantorovichInputsV1 {
            problem_id: self.problem_id.clone(),
            linearization_id: self.linearization_id.clone(),
            norm_identity_id: self.norm_identity_id.clone(),
            eta_problem_id: self.problem_id.clone(),
            gamma_problem_id: self.problem_id.clone(),
            eta_linearization_id: self.linearization_id.clone(),
            gamma_linearization_id: self.linearization_id.clone(),
            eta_norm_identity_id: self.norm_identity_id.clone(),
            gamma_norm_identity_id: self.norm_identity_id.clone(),
            eta_receipt_id: self.eta_receipt_id.clone(),
            gamma_bound_receipt_id: self.gamma_receipt_id.clone(),
            eta_upper_bound: self.eta_upper_bound,
            preconditioned_jacobian_lipschitz_upper_bound: self.gamma_upper_bound,
        }
    }
}

fn validate_identity(value: &str) -> Result<(), ScalarQuadraticBoundErrorV1> {
    if value.trim().is_empty() {
        return Err(ScalarQuadraticBoundErrorV1::EmptyEvidenceIdentity);
    }
    Ok(())
}

/// Derive eta >= |F(x0)/F'(x0)| and gamma >= |F''/F'(x0)|
/// using outward-rounded interval arithmetic. For a quadratic, F'' is
/// constant, so the latter bounds the Lipschitz constant of F'/F'(x0).
pub fn derive_scalar_quadratic_bounds(
    problem_id: &str,
    linearization_id: &str,
    norm_identity_id: &str,
    c0: f64,
    c1: f64,
    c2: f64,
    center: f64,
) -> Result<ScalarQuadraticBoundEvidenceV1, ScalarQuadraticBoundErrorV1> {
    validate_identity(problem_id)?;
    validate_identity(linearization_id)?;
    validate_identity(norm_identity_id)?;
    if !c0.is_finite() || !c1.is_finite() || !c2.is_finite() {
        return Err(ScalarQuadraticBoundErrorV1::NonFiniteCoefficient);
    }
    if !center.is_finite() {
        return Err(ScalarQuadraticBoundErrorV1::NonFiniteCenter);
    }

    let c0_i = IntervalV1::point(c0)?;
    let c1_i = IntervalV1::point(c1)?;
    let c2_i = IntervalV1::point(c2)?;
    let x0_i = IntervalV1::point(center)?;
    let half_i = IntervalV1::point(0.5)?;

    let f0 = c0_i
        .add(c1_i.mul(x0_i)?)?
        .add(half_i.mul(c2_i)?.mul(x0_i.square()?)?)?;
    let derivative = c1_i.add(c2_i.mul(x0_i)?)?;
    if derivative.contains_zero() {
        return Err(ScalarQuadraticBoundErrorV1::DerivativeIntervalContainsZero);
    }
    let inverse_derivative = derivative.reciprocal()?;
    let eta_interval = inverse_derivative.mul(f0)?;
    let eta_upper = eta_interval.absolute_upper()?;
    let gamma_interval = inverse_derivative.mul(c2_i)?;
    let gamma_upper = gamma_interval.absolute_upper()?;

    if !eta_upper.is_finite() || !gamma_upper.is_finite() {
        return Err(ScalarQuadraticBoundErrorV1::Interval(
            IntervalErrorV1::NonFiniteIntermediate,
        ));
    }

    let model_id = format!(
        "quadratic-binary64-v0.1-{:016x}-{:016x}-{:016x}-{:016x}",
        c0.to_bits(), c1.to_bits(), c2.to_bits(), center.to_bits()
    );
    Ok(ScalarQuadraticBoundEvidenceV1 {
        problem_id: problem_id.to_owned(),
        linearization_id: linearization_id.to_owned(),
        norm_identity_id: norm_identity_id.to_owned(),
        polynomial_model_id: model_id.clone(),
        center,
        function_value_at_center: f0,
        derivative_at_center: derivative,
        inverse_derivative_at_center: inverse_derivative,
        eta_interval,
        eta_upper_bound: eta_upper,
        second_derivative: c2_i,
        gamma_interval,
        gamma_upper_bound: gamma_upper,
        eta_receipt_id: format!("eta-{model_id}"),
        gamma_receipt_id: format!("gamma-{model_id}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_conservative_eta_and_gamma_for_quadratic() {
        // F(x)=1/8 + x + x^2 at x0=-1/8:
        // F(x0)=1/64, F'(x0)=3/4, eta=1/48, gamma=8/3.
        let evidence = derive_scalar_quadratic_bounds(
            "quadratic-root-v0.1",
            "linearization-at-minus-one-eighth",
            "scalar-absolute-value-v0.1",
            0.125, 1.0, 2.0, -0.125,
        ).unwrap();
        assert!(evidence.function_value_at_center.contains(1.0 / 64.0));
        assert!(evidence.derivative_at_center.contains(0.75));
        assert!(evidence.eta_interval.contains(1.0 / 48.0));
        assert!(evidence.gamma_interval.contains(8.0 / 3.0));
        assert!(evidence.eta_upper_bound >= 1.0 / 48.0);
        assert!(evidence.gamma_upper_bound >= 8.0 / 3.0);
        let inputs = evidence.newton_kantorovich_inputs();
        assert_eq!(inputs.problem_id, evidence.problem_id);
        assert_eq!(inputs.eta_receipt_id, evidence.eta_receipt_id);
        assert_eq!(inputs.gamma_bound_receipt_id, evidence.gamma_receipt_id);
    }

    #[test]
    fn linear_residual_at_root_has_zero_eta_and_gamma() {
        let evidence = derive_scalar_quadratic_bounds(
            "linear-root-v0.1", "linearization-v0.1", "scalar-abs-v0.1",
            0.0, 2.0, 0.0, 0.0,
        ).unwrap();
        assert_eq!(evidence.eta_upper_bound, 0.0);
        assert_eq!(evidence.gamma_upper_bound, 0.0);
    }

    #[test]
    fn derivative_interval_containing_zero_is_rejected() {
        assert_eq!(
            derive_scalar_quadratic_bounds(
                "singular", "linearization", "norm", 1.0, 0.0, 2.0, 0.0
            ),
            Err(ScalarQuadraticBoundErrorV1::DerivativeIntervalContainsZero)
        );
    }

    #[test]
    fn invalid_coefficients_and_identities_are_rejected() {
        assert_eq!(
            derive_scalar_quadratic_bounds(
                "problem", "linearization", "norm", f64::NAN, 1.0, 1.0, 0.0
            ),
            Err(ScalarQuadraticBoundErrorV1::NonFiniteCoefficient)
        );
        assert_eq!(
            derive_scalar_quadratic_bounds(
                "", "linearization", "norm", 1.0, 1.0, 1.0, 0.0
            ),
            Err(ScalarQuadraticBoundErrorV1::EmptyEvidenceIdentity)
        );
    }
}
