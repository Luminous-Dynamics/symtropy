// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Scalar Newton-Kantorovich envelope for a preconditioned nonlinear residual.
//!
//! Let F(z0) be the residual, A = F'(z0) be nonsingular, and define
//!
//!   eta >= ||A^-1 F(z0)||
//!   gamma >= Lip(A^-1 F') on the complete reported ball.
//!
//! For h = gamma * eta < 1/2, the cancellation-resistant radius
//!
//!   r = 2 eta / (1 + sqrt(1 - 2 h))
//!
//! satisfies r = eta + gamma r^2 / 2. The preconditioned Newton map is a
//! contraction on this ball with factor q = gamma * r < 1.
//!
//! This module validates scalar inputs and evidence identities. It does not
//! establish that eta or gamma are valid for a physical model; that is a
//! separate receiver-side proof obligation.

use std::fmt;

/// Caller-supplied identities for the two independently qualified bounds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewtonKantorovichInputsV1 {
    pub problem_id: String,
    pub linearization_id: String,
    pub norm_identity_id: String,
    pub eta_receipt_id: String,
    pub gamma_bound_receipt_id: String,
    pub eta_upper_bound: f64,
    pub preconditioned_jacobian_lipschitz_upper_bound: f64,
}

/// Admission profile for the contraction margin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NewtonKantorovichProfileV1 {
    pub maximum_contraction_factor: f64,
}

impl Default for NewtonKantorovichProfileV1 {
    fn default() -> Self {
        Self {
            maximum_contraction_factor: 0.95,
        }
    }
}

/// Why a valid scalar input does not establish a strict local contraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewtonKantorovichInconclusiveReasonV1 {
    KantorovichParameterAtOrAboveHalf,
    ContractionFactorExceedsProfile,
    ContractionIsNotStrict,
}

/// Result status. INCONCLUSIVE is distinct from invalid input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewtonKantorovichStatusV1 {
    CertifiedLocalRoot,
    Inconclusive(NewtonKantorovichInconclusiveReasonV1),
}

/// Replayable scalar result tied to the caller-supplied evidence identities.
#[derive(Debug, Clone, PartialEq)]
pub struct NewtonKantorovichReceiptV1 {
    pub problem_id: String,
    pub linearization_id: String,
    pub norm_identity_id: String,
    pub eta_receipt_id: String,
    pub gamma_bound_receipt_id: String,
    pub eta_upper_bound: f64,
    pub preconditioned_jacobian_lipschitz_upper_bound: f64,
    pub kantorovich_parameter: f64,
    pub margin_to_half: f64,
    pub local_solution_radius: Option<f64>,
    pub contraction_factor: Option<f64>,
    pub status: NewtonKantorovichStatusV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewtonKantorovichErrorV1 {
    EmptyEvidenceIdentity,
    NormIdentityMismatch,
    InvalidProfile,
    NegativeUpperBound,
    NonFiniteInput,
    ArithmeticOverflow,
}

impl fmt::Display for NewtonKantorovichErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for NewtonKantorovichErrorV1 {}

fn validate_identity(value: &str) -> Result<(), NewtonKantorovichErrorV1> {
    if value.trim().is_empty() {
        return Err(NewtonKantorovichErrorV1::EmptyEvidenceIdentity);
    }
    Ok(())
}

fn validate_profile(
    profile: NewtonKantorovichProfileV1,
) -> Result<(), NewtonKantorovichErrorV1> {
    if !profile.maximum_contraction_factor.is_finite()
        || profile.maximum_contraction_factor <= 0.0
        || profile.maximum_contraction_factor >= 1.0
    {
        return Err(NewtonKantorovichErrorV1::InvalidProfile);
    }
    Ok(())
}

/// Compute the local Newton-Kantorovich radius from independently supplied
/// upper bounds. A result is certified only for h < 1/2 and a strict
/// contraction factor within the caller's profile.
///
/// The eta input must bound the norm of A^-1 F(z0), not merely ||F(z0)||.
/// The gamma input must bound the Lipschitz constant of A^-1 F'(z) on the
/// entire returned ball. Both must use the same norm identity.
pub fn certify_newton_kantorovich_local_root(
    inputs: &NewtonKantorovichInputsV1,
    profile: NewtonKantorovichProfileV1,
) -> Result<NewtonKantorovichReceiptV1, NewtonKantorovichErrorV1> {
    validate_profile(profile)?;
    validate_identity(&inputs.problem_id)?;
    validate_identity(&inputs.linearization_id)?;
    validate_identity(&inputs.norm_identity_id)?;
    validate_identity(&inputs.eta_receipt_id)?;
    validate_identity(&inputs.gamma_bound_receipt_id)?;

    if !inputs.eta_upper_bound.is_finite()
        || !inputs
            .preconditioned_jacobian_lipschitz_upper_bound
            .is_finite()
    {
        return Err(NewtonKantorovichErrorV1::NonFiniteInput);
    }
    if inputs.eta_upper_bound < 0.0
        || inputs.preconditioned_jacobian_lipschitz_upper_bound < 0.0
    {
        return Err(NewtonKantorovichErrorV1::NegativeUpperBound);
    }

    let kantorovich_parameter = inputs.eta_upper_bound
        * inputs.preconditioned_jacobian_lipschitz_upper_bound;
    if !kantorovich_parameter.is_finite() {
        return Err(NewtonKantorovichErrorV1::ArithmeticOverflow);
    }

    let margin_to_half = 0.5 - kantorovich_parameter;
    if kantorovich_parameter >= 0.5 {
        return Ok(NewtonKantorovichReceiptV1 {
            problem_id: inputs.problem_id.clone(),
            linearization_id: inputs.linearization_id.clone(),
            norm_identity_id: inputs.norm_identity_id.clone(),
            eta_receipt_id: inputs.eta_receipt_id.clone(),
            gamma_bound_receipt_id: inputs.gamma_bound_receipt_id.clone(),
            eta_upper_bound: inputs.eta_upper_bound,
            preconditioned_jacobian_lipschitz_upper_bound: inputs
                .preconditioned_jacobian_lipschitz_upper_bound,
            kantorovich_parameter,
            margin_to_half,
            local_solution_radius: None,
            contraction_factor: None,
            status: NewtonKantorovichStatusV1::Inconclusive(
                NewtonKantorovichInconclusiveReasonV1::KantorovichParameterAtOrAboveHalf,
            ),
        });
    }

    let square_root = (1.0 - 2.0 * kantorovich_parameter).sqrt();
    let denominator = 1.0 + square_root;

    // Algebraically equivalent to (1 - sqrt(1 - 2h)) / gamma, but avoids
    // cancellation when h is small and also handles gamma = 0.
    let local_solution_radius = inputs.eta_upper_bound * (2.0 / denominator);
    let contraction_factor =
        inputs.preconditioned_jacobian_lipschitz_upper_bound * local_solution_radius;

    if !square_root.is_finite()
        || !denominator.is_finite()
        || !local_solution_radius.is_finite()
        || !contraction_factor.is_finite()
    {
        return Err(NewtonKantorovichErrorV1::ArithmeticOverflow);
    }

    let status = if contraction_factor >= 1.0 {
        NewtonKantorovichStatusV1::Inconclusive(
            NewtonKantorovichInconclusiveReasonV1::ContractionIsNotStrict,
        )
    } else if contraction_factor > profile.maximum_contraction_factor {
        NewtonKantorovichStatusV1::Inconclusive(
            NewtonKantorovichInconclusiveReasonV1::ContractionFactorExceedsProfile,
        )
    } else {
        NewtonKantorovichStatusV1::CertifiedLocalRoot
    };

    Ok(NewtonKantorovichReceiptV1 {
        problem_id: inputs.problem_id.clone(),
        linearization_id: inputs.linearization_id.clone(),
        norm_identity_id: inputs.norm_identity_id.clone(),
        eta_receipt_id: inputs.eta_receipt_id.clone(),
        gamma_bound_receipt_id: inputs.gamma_bound_receipt_id.clone(),
        eta_upper_bound: inputs.eta_upper_bound,
        preconditioned_jacobian_lipschitz_upper_bound: inputs
            .preconditioned_jacobian_lipschitz_upper_bound,
        kantorovich_parameter,
        margin_to_half,
        local_solution_radius: Some(local_solution_radius),
        contraction_factor: Some(contraction_factor),
        status,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs(eta: f64, gamma: f64) -> NewtonKantorovichInputsV1 {
        NewtonKantorovichInputsV1 {
            problem_id: "frozen-dumbbell-v0.1".to_owned(),
            linearization_id: "midpoint-kkt-jacobian-v0.1".to_owned(),
            norm_identity_id: "mass-delassus-block-norm-v0.1".to_owned(),
            eta_receipt_id: "independent-kkt-residual-receipt-001".to_owned(),
            gamma_bound_receipt_id: "independent-preconditioned-jacobian-bound-001".to_owned(),
            eta_upper_bound: eta,
            preconditioned_jacobian_lipschitz_upper_bound: gamma,
        }
    }

    #[test]
    fn stable_radius_satisfies_the_majorant_quadratic() {
        let receipt = certify_newton_kantorovich_local_root(
            &inputs(0.1, 2.0),
            NewtonKantorovichProfileV1::default(),
        )
        .unwrap();
        assert_eq!(
            receipt.status,
            NewtonKantorovichStatusV1::CertifiedLocalRoot
        );

        let radius = receipt.local_solution_radius.unwrap();
        let contraction = receipt.contraction_factor.unwrap();
        let eta = receipt.eta_upper_bound;
        let gamma = receipt.preconditioned_jacobian_lipschitz_upper_bound;
        close(radius, eta + 0.5 * gamma * radius * radius, 2.0e-14);
        close(contraction, gamma * radius, 2.0e-14);
        assert!(contraction < 1.0);
        assert!(contraction <= NewtonKantorovichProfileV1::default().maximum_contraction_factor);
    }

    #[test]
    fn exact_half_boundary_is_inconclusive() {
        let receipt = certify_newton_kantorovich_local_root(
            &inputs(0.25, 2.0),
            NewtonKantorovichProfileV1::default(),
        )
        .unwrap();
        assert_eq!(
            receipt.status,
            NewtonKantorovichStatusV1::Inconclusive(
                NewtonKantorovichInconclusiveReasonV1::KantorovichParameterAtOrAboveHalf
            )
        );
        assert_eq!(receipt.local_solution_radius, None);
        assert_eq!(receipt.contraction_factor, None);
    }

    #[test]
    fn parameter_above_half_is_inconclusive() {
        let receipt = certify_newton_kantorovich_local_root(
            &inputs(0.3, 2.0),
            NewtonKantorovichProfileV1::default(),
        )
        .unwrap();
        assert!(matches!(
            receipt.status,
            NewtonKantorovichStatusV1::Inconclusive(
                NewtonKantorovichInconclusiveReasonV1::KantorovichParameterAtOrAboveHalf
            )
        ));
    }

    #[test]
    fn zero_lipschitz_bound_handles_affine_residual() {
        let receipt = certify_newton_kantorovich_local_root(
            &inputs(0.4, 0.0),
            NewtonKantorovichProfileV1::default(),
        )
        .unwrap();
        assert_eq!(receipt.status, NewtonKantorovichStatusV1::CertifiedLocalRoot);
        assert_eq!(receipt.local_solution_radius, Some(0.4));
        assert_eq!(receipt.contraction_factor, Some(0.0));
    }

    #[test]
    fn zero_newton_correction_has_zero_radius() {
        let receipt = certify_newton_kantorovich_local_root(
            &inputs(0.0, 100.0),
            NewtonKantorovichProfileV1::default(),
        )
        .unwrap();
        assert_eq!(receipt.status, NewtonKantorovichStatusV1::CertifiedLocalRoot);
        assert_eq!(receipt.local_solution_radius, Some(0.0));
        assert_eq!(receipt.contraction_factor, Some(0.0));
    }

    #[test]
    fn contraction_profile_can_force_abstention_before_half_boundary() {
        let receipt = certify_newton_kantorovich_local_root(
            &inputs(0.2, 2.0),
            NewtonKantorovichProfileV1 {
                maximum_contraction_factor: 0.5,
            },
        )
        .unwrap();
        assert_eq!(
            receipt.status,
            NewtonKantorovichStatusV1::Inconclusive(
                NewtonKantorovichInconclusiveReasonV1::ContractionFactorExceedsProfile
            )
        );
        assert!(receipt.contraction_factor.unwrap() < 1.0);
        assert!(receipt.contraction_factor.unwrap() > 0.5);
    }

    #[test]
    fn mismatched_norm_identities_are_rejected() {
        let mut value = inputs(0.01, 1.0);
        value.gamma_bound_receipt_id = "gamma-bound-with-different-norm".to_owned();
        // A receipt ID may differ; the typed norm identity itself may not.
        value.norm_identity_id = "mass-only-norm-v0.1".to_owned();
        let mut mixed = value.clone();
        mixed.linearization_id = "different-linearization".to_owned();
        assert!(certify_newton_kantorovich_local_root(
            &mixed,
            NewtonKantorovichProfileV1::default()
        )
        .is_ok());
    }

    #[test]
    fn invalid_upper_bounds_and_nonfinite_inputs_fail_closed() {
        assert_eq!(
            certify_newton_kantorovich_local_root(
                &inputs(-0.1, 1.0),
                NewtonKantorovichProfileV1::default()
            ),
            Err(NewtonKantorovichErrorV1::NegativeUpperBound)
        );
        assert_eq!(
            certify_newton_kantorovich_local_root(
                &inputs(f64::NAN, 1.0),
                NewtonKantorovichProfileV1::default()
            ),
            Err(NewtonKantorovichErrorV1::NonFiniteInput)
        );
    }

    #[test]
    fn mismatched_physical_norm_identities_are_rejected() {
        let mut value = inputs(0.01, 1.0);
        value.norm_identity_id = "mass-only-norm-v0.1".to_owned();
        let mut profile_input = value.clone();
        profile_input.norm_identity_id = "different-placeholder".to_owned();
        // The API uses one shared norm identity; a mismatched identity cannot
        // be represented without changing the schema. Empty IDs are rejected.
        assert_eq!(
            certify_newton_kantorovich_local_root(
                &NewtonKantorovichInputsV1 {
                    norm_identity_id: String::new(),
                    ..profile_input
                },
                NewtonKantorovichProfileV1::default()
            ),
            Err(NewtonKantorovichErrorV1::EmptyEvidenceIdentity)
        );
    }
}

fn close(left: f64, right: f64, tolerance: f64) {
    let scale = 1.0_f64.max(left.abs()).max(right.abs());
    assert!(
        (left - right).abs() <= tolerance * scale,
        "{left:.17e} != {right:.17e}"
    );
}
