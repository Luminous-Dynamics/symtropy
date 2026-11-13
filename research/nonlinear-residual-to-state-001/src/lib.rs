// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Metric-aware Newton-Kantorovich local residual-to-state envelopes.
//!
//! This module evaluates the majorant from supplied bounds. It does not
//! independently prove that inverse-Jacobian or Lipschitz bounds are valid.
//! A floating-point estimate is diagnostic only; an analytic-upper-bound
//! receipt is conditional on the producer's stated proof obligations.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundEvidenceV1 {
    FloatingPointEstimate,
    ConservativeUpperBound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeStatusV1 {
    NumericallyEstimated,
    ConservativeBoundQualified,
    Inconclusive,
    OutOfProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimTierV1 {
    DiagnosticOnly,
    ConditionalKantorovichBall,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeErrorV1 {
    EmptyIdentity,
    NonFiniteInput,
    InvalidInverseNormBound,
    InvalidCorrectionBound,
    InvalidLipschitzBound,
    InvalidNeighbourhoodRadius,
    ArithmeticOverflow,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NormBindingV1 {
    pub system_id: String,
    pub candidate_id: String,
    pub domain_metric_id: String,
    pub codomain_metric_id: String,
    pub operator_id: String,
    pub neighbourhood_id: String,
    pub norm_profile_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KantorovichInputV1 {
    pub binding: NormBindingV1,
    /// beta >= ||J^-1|| from the declared codomain norm to domain norm.
    pub inverse_operator_norm_upper: f64,
    /// eta >= ||J^-1 F(z_hat)|| in the domain norm.
    pub newton_correction_norm_upper: f64,
    /// L >= Lip(DF) over the entire declared neighbourhood.
    pub jacobian_lipschitz_upper: f64,
    /// Maximum admissible radius measured in the domain norm.
    pub neighbourhood_radius: f64,
    pub evidence: BoundEvidenceV1,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KantorovichReceiptV1 {
    pub binding: NormBindingV1,
    pub inverse_operator_norm_upper: f64,
    pub newton_correction_norm_upper: f64,
    pub jacobian_lipschitz_upper: f64,
    pub kantorovich_product: f64,
    pub state_error_radius: Option<f64>,
    pub neighbourhood_radius: f64,
    pub status: EnvelopeStatusV1,
    pub claim_tier: ClaimTierV1,
    pub theorem_hypotheses: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScalarMetricLinearizationV1 {
    pub domain_metric: f64,
    pub codomain_metric: f64,
    pub residual_value: f64,
    pub jacobian_value: f64,
    pub second_derivative_abs_upper: f64,
    pub neighbourhood_radius: f64,
}

impl ScalarMetricLinearizationV1 {
    /// Produce metric-induced beta, eta and L for a one-dimensional map.
    ///
    /// For domain metric g_x and codomain metric g_y:
    /// beta = sqrt(g_x)/(abs(F') sqrt(g_y)),
    /// eta = sqrt(g_x) abs(F)/abs(F'),
    /// L = sqrt(g_y) abs(F'')/g_x.
    ///
    /// This scalar helper is an executable covariance fixture, not a
    /// substitute for a general induced-operator and tensor-norm proof.
    pub fn into_input(
        self,
        binding: NormBindingV1,
        evidence: BoundEvidenceV1,
    ) -> Result<KantorovichInputV1, EnvelopeErrorV1> {
        for value in [
            self.domain_metric,
            self.codomain_metric,
            self.residual_value,
            self.jacobian_value,
            self.second_derivative_abs_upper,
            self.neighbourhood_radius,
        ] {
            if !value.is_finite() {
                return Err(EnvelopeErrorV1::NonFiniteInput);
            }
        }
        if self.domain_metric <= 0.0 || self.codomain_metric <= 0.0 {
            return Err(EnvelopeErrorV1::InvalidInverseNormBound);
        }
        if self.jacobian_value == 0.0 {
            return Err(EnvelopeErrorV1::InvalidInverseNormBound);
        }
        if self.second_derivative_abs_upper < 0.0 {
            return Err(EnvelopeErrorV1::InvalidLipschitzBound);
        }
        if self.neighbourhood_radius <= 0.0 {
            return Err(EnvelopeErrorV1::InvalidNeighbourhoodRadius);
        }

        let domain_scale = self.domain_metric.sqrt();
        let codomain_scale = self.codomain_metric.sqrt();
        let inverse_operator_norm_upper =
            domain_scale / (self.jacobian_value.abs() * codomain_scale);
        let newton_correction_norm_upper =
            domain_scale * self.residual_value.abs() / self.jacobian_value.abs();
        let jacobian_lipschitz_upper =
            codomain_scale * self.second_derivative_abs_upper / self.domain_metric;

        if !inverse_operator_norm_upper.is_finite()
            || !newton_correction_norm_upper.is_finite()
            || !jacobian_lipschitz_upper.is_finite()
        {
            return Err(EnvelopeErrorV1::ArithmeticOverflow);
        }

        Ok(KantorovichInputV1 {
            binding,
            inverse_operator_norm_upper,
            newton_correction_norm_upper,
            jacobian_lipschitz_upper,
            neighbourhood_radius: self.neighbourhood_radius,
            evidence,
        })
    }
}

pub fn evaluate_kantorovich_envelope(
    input: KantorovichInputV1,
) -> Result<KantorovichReceiptV1, EnvelopeErrorV1> {
    validate_binding(&input.binding)?;
    if !input.inverse_operator_norm_upper.is_finite()
        || !input.newton_correction_norm_upper.is_finite()
        || !input.jacobian_lipschitz_upper.is_finite()
        || !input.neighbourhood_radius.is_finite()
    {
        return Err(EnvelopeErrorV1::NonFiniteInput);
    }
    if input.inverse_operator_norm_upper <= 0.0 {
        return Err(EnvelopeErrorV1::InvalidInverseNormBound);
    }
    if input.newton_correction_norm_upper < 0.0 {
        return Err(EnvelopeErrorV1::InvalidCorrectionBound);
    }
    if input.jacobian_lipschitz_upper < 0.0 {
        return Err(EnvelopeErrorV1::InvalidLipschitzBound);
    }
    if input.neighbourhood_radius <= 0.0 {
        return Err(EnvelopeErrorV1::InvalidNeighbourhoodRadius);
    }

    let beta_l = input.inverse_operator_norm_upper * input.jacobian_lipschitz_upper;
    let h = beta_l * input.newton_correction_norm_upper;
    if !beta_l.is_finite() || !h.is_finite() {
        return Err(EnvelopeErrorV1::ArithmeticOverflow);
    }

    let (status, radius) = if h > 0.5 {
        (EnvelopeStatusV1::Inconclusive, None)
    } else {
        // Stable form of (1 - sqrt(1 - 2h))/(beta*L).
        // It also handles L=0 and eta=0 without dividing by beta*L.
        let radius =
            2.0 * input.newton_correction_norm_upper / (1.0 + (1.0 - 2.0 * h).max(0.0).sqrt());
        if !radius.is_finite() {
            return Err(EnvelopeErrorV1::ArithmeticOverflow);
        }
        if radius > input.neighbourhood_radius {
            (EnvelopeStatusV1::OutOfProfile, Some(radius))
        } else {
            let status = match input.evidence {
                BoundEvidenceV1::FloatingPointEstimate => EnvelopeStatusV1::NumericallyEstimated,
                BoundEvidenceV1::ConservativeUpperBound => {
                    EnvelopeStatusV1::ConservativeBoundQualified
                }
            };
            (status, Some(radius))
        }
    };

    let claim_tier = match status {
        EnvelopeStatusV1::ConservativeBoundQualified => ClaimTierV1::ConditionalKantorovichBall,
        EnvelopeStatusV1::NumericallyEstimated
        | EnvelopeStatusV1::Inconclusive
        | EnvelopeStatusV1::OutOfProfile => ClaimTierV1::DiagnosticOnly,
    };

    Ok(KantorovichReceiptV1 {
        binding: input.binding,
        inverse_operator_norm_upper: input.inverse_operator_norm_upper,
        newton_correction_norm_upper: input.newton_correction_norm_upper,
        jacobian_lipschitz_upper: input.jacobian_lipschitz_upper,
        kantorovich_product: h,
        state_error_radius: radius,
        neighbourhood_radius: input.neighbourhood_radius,
        status,
        claim_tier,
        theorem_hypotheses: vec![
            "the declared domain and codomain norms are complete and consistently bound",
            "the Jacobian at the candidate is invertible",
            "inverse_operator_norm_upper bounds the induced inverse-Jacobian norm",
            "newton_correction_norm_upper bounds the metric Newton correction",
            "jacobian_lipschitz_upper bounds the Jacobian Lipschitz constant on the full ball",
            "the derived ball lies inside the region where all bounds hold",
        ],
    })
}

fn validate_binding(binding: &NormBindingV1) -> Result<(), EnvelopeErrorV1> {
    if [
        binding.system_id.as_str(),
        binding.candidate_id.as_str(),
        binding.domain_metric_id.as_str(),
        binding.codomain_metric_id.as_str(),
        binding.operator_id.as_str(),
        binding.neighbourhood_id.as_str(),
        binding.norm_profile_id.as_str(),
    ]
    .iter()
    .any(|value| value.trim().is_empty())
    {
        return Err(EnvelopeErrorV1::EmptyIdentity);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding() -> NormBindingV1 {
        NormBindingV1 {
            system_id: "scalar-polynomial-test-v1".to_owned(),
            candidate_id: "xhat-1.4".to_owned(),
            domain_metric_id: "domain-metric-q-v1".to_owned(),
            codomain_metric_id: "residual-metric-v1".to_owned(),
            operator_id: "dF-at-xhat-v1".to_owned(),
            neighbourhood_id: "closed-ball-radius-0.3-v1".to_owned(),
            norm_profile_id: "metric-profile-v1".to_owned(),
        }
    }

    #[test]
    fn polynomial_root_error_is_inside_conservative_kantorovich_ball() {
        // F(x)=x^2-2.25, xhat=1.4, exact local root x*=1.5.
        let input = ScalarMetricLinearizationV1 {
            domain_metric: 2.0,
            codomain_metric: 3.0,
            residual_value: -0.29,
            jacobian_value: 2.8,
            second_derivative_abs_upper: 2.0,
            neighbourhood_radius: 0.3 * 2.0_f64.sqrt(),
        }
        .into_input(binding(), BoundEvidenceV1::ConservativeUpperBound)
        .unwrap();
        let receipt = evaluate_kantorovich_envelope(input).unwrap();
        assert_eq!(receipt.status, EnvelopeStatusV1::ConservativeBoundQualified);
        assert_eq!(receipt.claim_tier, ClaimTierV1::ConditionalKantorovichBall);
        let actual_error = 2.0_f64.sqrt() * (1.5 - 1.4);
        assert!(actual_error <= receipt.state_error_radius.unwrap());
        assert!(receipt.kantovich_product <= 0.5);
    }

    #[test]
    fn dense_scalar_rebasing_preserves_metric_newton_bounds_and_radius() {
        // q = s*y. The same scalar residual is represented with
        // g_y=s^2*g_q, F'_y=s*F'_q, F''_y=s^2*F''_q.
        let mut radii = Vec::new();
        for scale in [0.1_f64, 0.25, 1.0, 4.0, 10.0] {
            let request = ScalarMetricLinearizationV1 {
                domain_metric: scale * scale * 2.0,
                codomain_metric: 3.0,
                residual_value: -0.29,
                jacobian_value: scale * 2.8,
                second_derivative_abs_upper: scale * scale * 2.0,
                neighbourhood_radius: 0.3 * 2.0_f64.sqrt(),
            }
            .into_input(binding(), BoundEvidenceV1::FloatingPointEstimate)
            .unwrap();
            let receipt = evaluate_kantorovich_envelope(request).unwrap();
            radii.push((
                receipt.inverse_operator_norm_upper,
                receipt.newton_correction_norm_upper,
                receipt.jacobian_lipschitz_upper,
                receipt.state_error_radius.unwrap(),
            ));
            assert_eq!(receipt.status, EnvelopeStatusV1::NumericallyEstimated);
        }
        let reference = radii[2];
        for values in radii {
            for (observed, expected) in [
                (values.0, reference.0),
                (values.1, reference.1),
                (values.2, reference.2),
                (values.3, reference.3),
            ] {
                assert!((observed - expected).abs() < 1.0e-12);
            }
        }
    }

    #[test]
    fn tiny_residual_with_near_singular_jacobian_is_inconclusive() {
        let input = KantorovichInputV1 {
            binding: binding(),
            inverse_operator_norm_upper: 1.0e8,
            newton_correction_norm_upper: 1.0e-12,
            jacobian_lipschitz_upper: 1.0e8,
            neighbourhood_radius: 1.0,
            evidence: BoundEvidenceV1::ConservativeUpperBound,
        };
        let receipt = evaluate_kantorovich_envelope(input).unwrap();
        assert_eq!(receipt.status, EnvelopeStatusV1::Inconclusive);
        assert_eq!(receipt.state_error_radius, None);
        assert_eq!(receipt.claim_tier, ClaimTierV1::DiagnosticOnly);
    }

    #[test]
    fn ball_escape_is_out_of_profile_not_a_state_error_pass() {
        let input = KantorovichInputV1 {
            binding: binding(),
            inverse_operator_norm_upper: 1.0,
            newton_correction_norm_upper: 0.2,
            jacobian_lipschitz_upper: 0.1,
            neighbourhood_radius: 0.05,
            evidence: BoundEvidenceV1::ConservativeUpperBound,
        };
        let receipt = evaluate_kantorovich_envelope(input).unwrap();
        assert_eq!(receipt.status, EnvelopeStatusV1::OutOfProfile);
        assert!(receipt.state_error_radius.unwrap() > receipt.neighbourhood_radius);
        assert_eq!(receipt.claim_tier, ClaimTierV1::DiagnosticOnly);
    }

    #[test]
    fn zero_lipschitz_linear_system_has_newton_correction_radius() {
        let input = KantorovichInputV1 {
            binding: binding(),
            inverse_operator_norm_upper: 4.0,
            newton_correction_norm_upper: 0.125,
            jacobian_lipschitz_upper: 0.0,
            neighbourhood_radius: 0.2,
            evidence: BoundEvidenceV1::ConservativeUpperBound,
        };
        let receipt = evaluate_kantorovich_envelope(input).unwrap();
        assert_eq!(receipt.status, EnvelopeStatusV1::ConservativeBoundQualified);
        assert_eq!(receipt.state_error_radius, Some(0.125));
    }

    #[test]
    fn half_boundary_is_admitted_at_the_majorant_limit() {
        let input = KantorovichInputV1 {
            binding: binding(),
            inverse_operator_norm_upper: 1.0,
            newton_correction_norm_upper: 0.5,
            jacobian_lipschitz_upper: 1.0,
            neighbourhood_radius: 1.0,
            evidence: BoundEvidenceV1::FloatingPointEstimate,
        };
        let receipt = evaluate_kantorovich_envelope(input).unwrap();
        assert_eq!(receipt.status, EnvelopeStatusV1::NumericallyEstimated);
        assert_eq!(receipt.state_error_radius, Some(1.0));
    }

    #[test]
    fn invalid_and_nonfinite_bound_inputs_fail_closed() {
        let mut bad = KantorovichInputV1 {
            binding: binding(),
            inverse_operator_norm_upper: 1.0,
            newton_correction_norm_upper: 0.1,
            jacobian_lipschitz_upper: 1.0,
            neighbourhood_radius: 1.0,
            evidence: BoundEvidenceV1::FloatingPointEstimate,
        };
        bad.inverse_operator_norm_upper = f64::NAN;
        assert_eq!(
            evaluate_kantorovich_envelope(bad),
            Err(EnvelopeErrorV1::NonFiniteInput)
        );

        let mut missing_id = KantorovichInputV1 {
            binding: binding(),
            inverse_operator_norm_upper: 1.0,
            newton_correction_norm_upper: 0.1,
            jacobian_lipschitz_upper: 1.0,
            neighbourhood_radius: 1.0,
            evidence: BoundEvidenceV1::FloatingPointEstimate,
        };
        missing_id.binding.domain_metric_id.clear();
        assert_eq!(
            evaluate_kantorovich_envelope(missing_id),
            Err(EnvelopeErrorV1::EmptyIdentity)
        );
    }

    #[test]
    fn understated_jacobian_lipschitz_constant_is_not_a_validated_root_certificate() {
        // The API only marks a supplied L as the producer's conservative claim.
        // This numeric test demonstrates why the claim must be independently
        // audited: a wrong L value cannot be detected from one local Jacobian.
        let mut input = KantorovichInputV1 {
            binding: binding(),
            inverse_operator_norm_upper: 0.36,
            newton_correction_norm_upper: 0.104,
            jacobian_lipschitz_upper: 0.01,
            neighbourhood_radius: 0.3,
            evidence: BoundEvidenceV1::FloatingPointEstimate,
        };
        let receipt = evaluate_kantorovich_envelope(input.clone()).unwrap();
        assert_eq!(receipt.status, EnvelopeStatusV1::NumericallyEstimated);
        assert_eq!(receipt.claim_tier, ClaimTierV1::DiagnosticOnly);

        input.jacobian_lipschitz_upper = 2.0;
        input.evidence = BoundEvidenceV1::ConservativeUpperBound;
        let conservative = evaluate_kantorovich_envelope(input).unwrap();
        assert_eq!(
            conservative.status,
            EnvelopeStatusV1::ConservativeBoundQualified
        );
    }
}
