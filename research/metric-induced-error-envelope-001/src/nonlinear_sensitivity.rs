// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Local metric-aware error propagation through nonlinear maps.
//!
//! A full nonlinear envelope requires a declared second-derivative bound valid
//! over the entire input perturbation ball. Empirical curvature samples are
//! reported as estimates and are never promoted into upper bounds.

use super::operator::{OperatorConditionErrorV1, OperatorConditionProfileV1};
use super::rectangular_operator::{
    RectangularOperatorConditionReceiptV1, RectangularTangentToTangentOperator,
};
use super::{CoordinateFrame, Cotangent, Metric, MetricErrorV1};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NonlinearSensitivityErrorV1 {
    InvalidPerturbationRadius,
    InvalidEvidenceIdentity,
    InvalidCurvatureBound,
    CurvatureEvidenceRadiusExceeded,
    NonFiniteIntermediate,
    Metric(MetricErrorV1),
    Operator(OperatorConditionErrorV1),
}

impl From<MetricErrorV1> for NonlinearSensitivityErrorV1 {
    fn from(value: MetricErrorV1) -> Self {
        Self::Metric(value)
    }
}

impl From<OperatorConditionErrorV1> for NonlinearSensitivityErrorV1 {
    fn from(value: OperatorConditionErrorV1) -> Self {
        Self::Operator(value)
    }
}

impl core::fmt::Display for NonlinearSensitivityErrorV1 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for NonlinearSensitivityErrorV1 {}

/// Caller-declared evidence for a uniform second-derivative bound over the
/// entire perturbation ball. The evidence identity is a reference, not a
/// cryptographically verified proof or interval artifact in this module.
#[derive(Debug, Clone, PartialEq)]
pub enum CurvatureEvidenceV1 {
    AnalyticUpperBound {
        second_derivative_norm_bound: f64,
        /// Largest input tangent-metric radius on which the bound is valid.
        valid_radius: f64,
        evidence_id: String,
    },
    IntervalUpperBound {
        second_derivative_norm_upper: f64,
        /// Largest input tangent-metric radius covered by the interval enclosure.
        valid_radius: f64,
        evidence_id: String,
    },
    /// Sampled or finite-difference curvature; diagnostic only.
    EmpiricalEstimate {
        second_derivative_norm_estimate: f64,
        /// Largest radius covered by the empirical sampling design.
        valid_radius: f64,
        evidence_id: String,
    },
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurvatureEvidenceClassV1 {
    AnalyticUpperBound,
    IntervalUpperBound,
    EmpiricalEstimate,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonlinearSensitivityStatusV1 {
    /// The estimate is conditional on a caller-declared analytic remainder bound.
    DeclaredAnalyticRemainderBound,
    /// The estimate is conditional on a caller-declared interval remainder bound.
    DeclaredIntervalRemainderBound,
    /// Both terms are estimates; this is not an upper bound.
    EmpiricalEstimateOnly,
    /// Only the first-order local sensitivity is available.
    FirstOrderOnly,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NonlinearOutputErrorReceiptV1 {
    pub operator_condition: RectangularOperatorConditionReceiptV1,
    pub perturbation_radius: f64,
    pub derivative_operator_norm_estimate: f64,
    pub first_order_output_error_estimate: f64,
    pub curvature_evidence_class: CurvatureEvidenceClassV1,
    pub curvature_evidence_id: Option<String>,
    pub curvature_evidence_valid_radius: Option<f64>,
    pub curvature_norm_value: Option<f64>,
    pub nonlinear_remainder_estimate: Option<f64>,
    /// Populated only for analytic/interval evidence declarations.
    pub nonlinear_remainder_upper_estimate: Option<f64>,
    /// Populated for empirical and declared-bound cases; absent when curvature is missing.
    pub total_output_error_estimate: Option<f64>,
    /// Conditional on the declared evidence and numerical Jacobian estimate.
    pub total_output_error_upper_estimate: Option<f64>,
    pub status: NonlinearSensitivityStatusV1,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObservableSensitivityReceiptV1<Domain: CoordinateFrame, const N: usize> {
    pub operator_condition: RectangularOperatorConditionReceiptV1,
    pub pulled_back_gradient: Cotangent<Domain, N>,
    pub output_gradient_dual_norm: f64,
    pub input_gradient_dual_norm: f64,
    pub perturbation_radius: f64,
    pub first_order_observable_change_estimate: f64,
    pub curvature_evidence_class: CurvatureEvidenceClassV1,
    pub curvature_evidence_id: Option<String>,
    pub curvature_evidence_valid_radius: Option<f64>,
    pub curvature_norm_value: Option<f64>,
    pub nonlinear_remainder_estimate: Option<f64>,
    pub nonlinear_remainder_upper_estimate: Option<f64>,
    pub total_observable_change_estimate: Option<f64>,
    /// Conditional on the declared composite-observable curvature evidence.
    pub total_observable_change_upper_estimate: Option<f64>,
    pub status: NonlinearSensitivityStatusV1,
}

#[derive(Debug, Clone, PartialEq)]
struct CurvatureContributionV1 {
    class: CurvatureEvidenceClassV1,
    evidence_id: Option<String>,
    valid_radius: Option<f64>,
    norm_value: Option<f64>,
    remainder_estimate: Option<f64>,
    remainder_upper_estimate: Option<f64>,
    total_is_upper_estimate: bool,
}

fn validate_radius(radius: f64) -> Result<(), NonlinearSensitivityErrorV1> {
    if !radius.is_finite() || radius < 0.0 {
        return Err(NonlinearSensitivityErrorV1::InvalidPerturbationRadius);
    }
    Ok(())
}

fn validate_evidence_radius(
    requested_radius: f64,
    valid_radius: f64,
) -> Result<(), NonlinearSensitivityErrorV1> {
    if !valid_radius.is_finite() || valid_radius < 0.0 {
        return Err(NonlinearSensitivityErrorV1::InvalidCurvatureBound);
    }
    if requested_radius > valid_radius {
        return Err(NonlinearSensitivityErrorV1::CurvatureEvidenceRadiusExceeded);
    }
    Ok(())
}

fn curvature_contribution(
    evidence: &CurvatureEvidenceV1,
    radius: f64,
    first_order: f64,
) -> Result<CurvatureContributionV1, NonlinearSensitivityErrorV1> {
    validate_radius(radius)?;
    if !first_order.is_finite() {
        return Err(NonlinearSensitivityErrorV1::NonFiniteIntermediate);
    }

    let compute_remainder = |norm: f64| -> Result<f64, NonlinearSensitivityErrorV1> {
        if !norm.is_finite() || norm < 0.0 {
            return Err(NonlinearSensitivityErrorV1::InvalidCurvatureBound);
        }
        let radius_squared = radius * radius;
        let remainder = 0.5 * norm * radius_squared;
        if !radius_squared.is_finite() || !remainder.is_finite() {
            return Err(NonlinearSensitivityErrorV1::NonFiniteIntermediate);
        }
        Ok(remainder)
    };

    match evidence {
        CurvatureEvidenceV1::AnalyticUpperBound {
            second_derivative_norm_bound,
            valid_radius,
            evidence_id,
        } => {
            validate_evidence_radius(radius, *valid_radius)?;
            if evidence_id.trim().is_empty() {
                return Err(NonlinearSensitivityErrorV1::InvalidEvidenceIdentity);
            }
            let remainder = compute_remainder(*second_derivative_norm_bound)?;
            if !(first_order + remainder).is_finite() {
                return Err(NonlinearSensitivityErrorV1::NonFiniteIntermediate);
            }
            Ok(CurvatureContributionV1 {
                class: CurvatureEvidenceClassV1::AnalyticUpperBound,
                evidence_id: Some(evidence_id.clone()),
                valid_radius: Some(*valid_radius),
                norm_value: Some(*second_derivative_norm_bound),
                remainder_estimate: Some(remainder),
                remainder_upper_estimate: Some(remainder),
                total_is_upper_estimate: true,
            })
        }
        CurvatureEvidenceV1::IntervalUpperBound {
            second_derivative_norm_upper,
            valid_radius,
            evidence_id,
        } => {
            validate_evidence_radius(radius, *valid_radius)?;
            if evidence_id.trim().is_empty() {
                return Err(NonlinearSensitivityErrorV1::InvalidEvidenceIdentity);
            }
            let remainder = compute_remainder(*second_derivative_norm_upper)?;
            if !(first_order + remainder).is_finite() {
                return Err(NonlinearSensitivityErrorV1::NonFiniteIntermediate);
            }
            Ok(CurvatureContributionV1 {
                class: CurvatureEvidenceClassV1::IntervalUpperBound,
                evidence_id: Some(evidence_id.clone()),
                valid_radius: Some(*valid_radius),
                norm_value: Some(*second_derivative_norm_upper),
                remainder_estimate: Some(remainder),
                remainder_upper_estimate: Some(remainder),
                total_is_upper_estimate: true,
            })
        }
        CurvatureEvidenceV1::EmpiricalEstimate {
            second_derivative_norm_estimate,
            valid_radius,
            evidence_id,
        } => {
            validate_evidence_radius(radius, *valid_radius)?;
            if evidence_id.trim().is_empty() {
                return Err(NonlinearSensitivityErrorV1::InvalidEvidenceIdentity);
            }
            let remainder = compute_remainder(*second_derivative_norm_estimate)?;
            if !(first_order + remainder).is_finite() {
                return Err(NonlinearSensitivityErrorV1::NonFiniteIntermediate);
            }
            Ok(CurvatureContributionV1 {
                class: CurvatureEvidenceClassV1::EmpiricalEstimate,
                evidence_id: Some(evidence_id.clone()),
                valid_radius: Some(*valid_radius),
                norm_value: Some(*second_derivative_norm_estimate),
                remainder_estimate: Some(remainder),
                remainder_upper_estimate: None,
                total_is_upper_estimate: false,
            })
        }
        CurvatureEvidenceV1::Missing => Ok(CurvatureContributionV1 {
            class: CurvatureEvidenceClassV1::Missing,
            evidence_id: None,
            valid_radius: None,
            norm_value: None,
            remainder_estimate: None,
            remainder_upper_estimate: None,
            total_is_upper_estimate: false,
        }),
    }
}

fn status_for(class: CurvatureEvidenceClassV1) -> NonlinearSensitivityStatusV1 {
    match class {
        CurvatureEvidenceClassV1::AnalyticUpperBound => {
            NonlinearSensitivityStatusV1::DeclaredAnalyticRemainderBound
        }
        CurvatureEvidenceClassV1::IntervalUpperBound => {
            NonlinearSensitivityStatusV1::DeclaredIntervalRemainderBound
        }
        CurvatureEvidenceClassV1::EmpiricalEstimate => {
            NonlinearSensitivityStatusV1::EmpiricalEstimateOnly
        }
        CurvatureEvidenceClassV1::Missing => NonlinearSensitivityStatusV1::FirstOrderOnly,
    }
}

/// Propagate a metric-bounded input perturbation through a local nonlinear map.
///
/// For input perturbation radius delta, Jacobian J and a declared uniform
/// second-derivative norm bound H valid over the entire ball:
///
///     ||F(x+e)-F(x)||_Y <= ||J||_(X->Y) delta + 0.5 H delta^2.
///
/// The Jacobian norm is a numerical singular-value estimate. The total field
/// is therefore conditional on this estimate and on the caller-supplied
/// curvature evidence; no formal floating-point proof is performed here.
pub fn propagate_output_error<
    Domain: CoordinateFrame,
    Codomain: CoordinateFrame,
    const K: usize,
    const N: usize,
>(
    jacobian: &RectangularTangentToTangentOperator<Domain, Codomain, K, N>,
    input_metric: &Metric<Domain, N>,
    output_metric: &Metric<Codomain, K>,
    perturbation_radius: f64,
    curvature: &CurvatureEvidenceV1,
    operator_profile: OperatorConditionProfileV1,
) -> Result<NonlinearOutputErrorReceiptV1, NonlinearSensitivityErrorV1> {
    validate_radius(perturbation_radius)?;
    let operator_condition = jacobian.assess(input_metric, output_metric, operator_profile)?;
    let derivative_norm = operator_condition.largest_singular_value;
    if !derivative_norm.is_finite() || derivative_norm < 0.0 {
        return Err(NonlinearSensitivityErrorV1::NonFiniteIntermediate);
    }
    let first_order = derivative_norm * perturbation_radius;
    if !first_order.is_finite() {
        return Err(NonlinearSensitivityErrorV1::NonFiniteIntermediate);
    }

    let contribution = curvature_contribution(curvature, perturbation_radius, first_order)?;
    let total_estimate = contribution
        .remainder_estimate
        .map(|remainder| first_order + remainder);
    let total_upper = if contribution.total_is_upper_estimate {
        contribution
            .remainder_upper_estimate
            .map(|remainder| first_order + remainder)
    } else {
        None
    };

    Ok(NonlinearOutputErrorReceiptV1 {
        operator_condition,
        perturbation_radius,
        derivative_operator_norm_estimate: derivative_norm,
        first_order_output_error_estimate: first_order,
        curvature_evidence_class: contribution.class,
        curvature_evidence_id: contribution.evidence_id,
        curvature_evidence_valid_radius: contribution.valid_radius,
        curvature_norm_value: contribution.norm_value,
        nonlinear_remainder_estimate: contribution.remainder_estimate,
        nonlinear_remainder_upper_estimate: contribution.remainder_upper_estimate,
        total_output_error_estimate: total_estimate,
        total_output_error_upper_estimate: total_upper,
        status: status_for(contribution.class),
    })
}

/// Compute observable-specific local sensitivity through the adjoint pullback.
///
/// For scalar observable phi with output covector g_Y, the first-order change
/// is bounded by ||J^T g_Y||_(G_X^-1) * delta. The curvature evidence must bound
/// the Hessian of the composite scalar observable phi composed with F over the
/// entire input ball; a bound on F's Hessian alone is not sufficient.
pub fn observable_sensitivity<
    Domain: CoordinateFrame,
    Codomain: CoordinateFrame,
    const K: usize,
    const N: usize,
>(
    jacobian: &RectangularTangentToTangentOperator<Domain, Codomain, K, N>,
    input_metric: &Metric<Domain, N>,
    output_metric: &Metric<Codomain, K>,
    output_gradient: &Cotangent<Codomain, K>,
    perturbation_radius: f64,
    observable_curvature: &CurvatureEvidenceV1,
    operator_profile: OperatorConditionProfileV1,
) -> Result<ObservableSensitivityReceiptV1<Domain, N>, NonlinearSensitivityErrorV1> {
    validate_radius(perturbation_radius)?;
    let operator_condition = jacobian.assess(input_metric, output_metric, operator_profile)?;
    let output_gradient_dual_norm = output_metric.cotangent_norm(output_gradient)?;

    let mut pulled_back = [0.0; N];
    for (domain_index, value) in pulled_back.iter_mut().enumerate() {
        for (codomain_index, row) in jacobian.matrix().iter().enumerate() {
            *value += row[domain_index] * output_gradient.components()[codomain_index];
        }
        if !value.is_finite() {
            return Err(NonlinearSensitivityErrorV1::NonFiniteIntermediate);
        }
    }
    let pulled_back_gradient = Cotangent::<Domain, N>::new(pulled_back)?;
    let input_gradient_dual_norm = input_metric.cotangent_norm(&pulled_back_gradient)?;
    let first_order = input_gradient_dual_norm * perturbation_radius;
    if !first_order.is_finite() {
        return Err(NonlinearSensitivityErrorV1::NonFiniteIntermediate);
    }

    let contribution =
        curvature_contribution(observable_curvature, perturbation_radius, first_order)?;
    let total_estimate = contribution
        .remainder_estimate
        .map(|remainder| first_order + remainder);
    let total_upper = if contribution.total_is_upper_estimate {
        contribution
            .remainder_upper_estimate
            .map(|remainder| first_order + remainder)
    } else {
        None
    };

    Ok(ObservableSensitivityReceiptV1 {
        operator_condition,
        pulled_back_gradient,
        output_gradient_dual_norm,
        input_gradient_dual_norm,
        perturbation_radius,
        first_order_observable_change_estimate: first_order,
        curvature_evidence_class: contribution.class,
        curvature_evidence_id: contribution.evidence_id,
        curvature_evidence_valid_radius: contribution.valid_radius,
        curvature_norm_value: contribution.norm_value,
        nonlinear_remainder_estimate: contribution.remainder_estimate,
        nonlinear_remainder_upper_estimate: contribution.remainder_upper_estimate,
        total_observable_change_estimate: total_estimate,
        total_observable_change_upper_estimate: total_upper,
        status: status_for(contribution.class),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rectangular_operator::{
        RectangularOperatorStatusV1, RectangularTangentToTangentOperator,
    };
    use crate::{CartesianQ, DenseY, MetricProfileV1, Rebasis, Tangent};

    fn metric_profile() -> MetricProfileV1 {
        MetricProfileV1::default()
    }

    fn operator_profile() -> OperatorConditionProfileV1 {
        OperatorConditionProfileV1 {
            relative_jacobi_tolerance: 1.0e-13,
            relative_rank_tolerance: 1.0e-12,
            maximum_condition_estimate: 1.0e12,
            maximum_jacobi_sweeps: 64,
        }
    }

    fn close(left: f64, right: f64, tolerance: f64) {
        let scale = 1.0_f64.max(left.abs()).max(right.abs());
        assert!(
            (left - right).abs() <= tolerance * scale,
            "left={left:.16e}, right={right:.16e}"
        );
    }

    fn identity_metric<F: CoordinateFrame, const N: usize>() -> Metric<F, N> {
        let matrix = std::array::from_fn(|row| {
            std::array::from_fn(|column| if row == column { 1.0 } else { 0.0 })
        });
        Metric::admit(matrix, metric_profile()).unwrap()
    }

    fn transform_rectangular_operator<const K: usize, const N: usize>(
        operator_q: &[[f64; N]; K],
        domain_basis: &Rebasis<CartesianQ, DenseY, N>,
        codomain_basis: &Rebasis<CartesianQ, DenseY, K>,
    ) -> [[f64; N]; K] {
        // A_y = S_Y^-1 A_q S_X, using frame-typed tangent maps column-by-column.
        let mut result = [[0.0; N]; K];
        for column in 0..N {
            let mut domain_column = [0.0; N];
            for row in 0..N {
                domain_column[row] = domain_basis.matrix()[row][column];
            }
            let codomain_column: [f64; K] = std::array::from_fn(|row| {
                (0..N)
                    .map(|inner| operator_q[row][inner] * domain_column[inner])
                    .sum()
            });
            let source_vector = Tangent::<CartesianQ, K>::new(codomain_column).unwrap();
            let target_vector = codomain_basis.tangent_to_to(&source_vector).unwrap();
            for row in 0..K {
                result[row][column] = target_vector.components()[row];
            }
        }
        result
    }

    #[test]
    fn quadratic_step_map_has_an_analytic_second_order_envelope() {
        // F(x) = x + x^2, so F'(0)=1 and |F''|=2.
        let input = identity_metric::<CartesianQ, 1>();
        let output = identity_metric::<DenseY, 1>();
        let jacobian =
            RectangularTangentToTangentOperator::<CartesianQ, DenseY, 1, 1>::new([[1.0]]).unwrap();
        let receipt = propagate_output_error(
            &jacobian,
            &input,
            &output,
            0.1,
            &CurvatureEvidenceV1::AnalyticUpperBound {
                second_derivative_norm_bound: 2.0,
                valid_radius: 0.1,
                evidence_id: "quadratic-F-second-derivative-v1".to_owned(),
            },
            operator_profile(),
        )
        .unwrap();

        close(receipt.derivative_operator_norm_estimate, 1.0, 1.0e-14);
        close(receipt.first_order_output_error_estimate, 0.1, 1.0e-14);
        close(
            receipt.nonlinear_remainder_upper_estimate.unwrap(),
            0.01,
            1.0e-14,
        );
        close(
            receipt.total_output_error_upper_estimate.unwrap(),
            0.11,
            1.0e-14,
        );
        assert_eq!(
            receipt.status,
            NonlinearSensitivityStatusV1::DeclaredAnalyticRemainderBound
        );

        for perturbation in [-0.1_f64, 0.1_f64] {
            let actual = (perturbation + perturbation * perturbation).abs();
            assert!(actual <= receipt.total_output_error_upper_estimate.unwrap() + 1.0e-14);
        }
    }

    #[test]
    fn interval_curvature_is_the_second_supported_remainder_class() {
        let input = identity_metric::<CartesianQ, 1>();
        let output = identity_metric::<DenseY, 1>();
        let jacobian =
            RectangularTangentToTangentOperator::<CartesianQ, DenseY, 1, 1>::new([[3.0]]).unwrap();
        let receipt = propagate_output_error(
            &jacobian,
            &input,
            &output,
            0.02,
            &CurvatureEvidenceV1::IntervalUpperBound {
                second_derivative_norm_upper: 4.0,
                valid_radius: 0.02,
                evidence_id: "interval-hessian-enclosure-004".to_owned(),
            },
            operator_profile(),
        )
        .unwrap();
        close(receipt.first_order_output_error_estimate, 0.06, 1.0e-14);
        close(
            receipt.nonlinear_remainder_upper_estimate.unwrap(),
            0.0008,
            1.0e-14,
        );
        close(
            receipt.total_output_error_upper_estimate.unwrap(),
            0.0608,
            1.0e-14,
        );
        assert_eq!(
            receipt.status,
            NonlinearSensitivityStatusV1::DeclaredIntervalRemainderBound
        );
    }

    #[test]
    fn empirical_curvature_remains_an_estimate_not_an_upper_bound() {
        let input = identity_metric::<CartesianQ, 1>();
        let output = identity_metric::<DenseY, 1>();
        let jacobian =
            RectangularTangentToTangentOperator::<CartesianQ, DenseY, 1, 1>::new([[2.0]]).unwrap();
        let receipt = propagate_output_error(
            &jacobian,
            &input,
            &output,
            0.1,
            &CurvatureEvidenceV1::EmpiricalEstimate {
                second_derivative_norm_estimate: 5.0,
                valid_radius: 0.1,
                evidence_id: "sampled-curvature-20-points".to_owned(),
            },
            operator_profile(),
        )
        .unwrap();

        assert!(receipt.total_output_error_estimate.is_some());
        assert_eq!(receipt.total_output_error_upper_estimate, None);
        assert_eq!(
            receipt.status,
            NonlinearSensitivityStatusV1::EmpiricalEstimateOnly
        );
    }

    #[test]
    fn missing_curvature_reports_first_order_only() {
        let input = identity_metric::<CartesianQ, 1>();
        let output = identity_metric::<DenseY, 1>();
        let jacobian =
            RectangularTangentToTangentOperator::<CartesianQ, DenseY, 1, 1>::new([[2.0]]).unwrap();
        let receipt = propagate_output_error(
            &jacobian,
            &input,
            &output,
            0.1,
            &CurvatureEvidenceV1::Missing,
            operator_profile(),
        )
        .unwrap();

        close(receipt.first_order_output_error_estimate, 0.2, 1.0e-14);
        assert_eq!(receipt.total_output_error_estimate, None);
        assert_eq!(receipt.total_output_error_upper_estimate, None);
        assert_eq!(receipt.status, NonlinearSensitivityStatusV1::FirstOrderOnly);
    }

    #[test]
    fn adjoint_pullback_gives_observable_specific_metric_sensitivity() {
        let input_metric = identity_metric::<CartesianQ, 2>();
        let output_metric = identity_metric::<DenseY, 2>();
        let jacobian = RectangularTangentToTangentOperator::<CartesianQ, DenseY, 2, 2>::new([
            [2.0, 1.0],
            [0.0, 3.0],
        ])
        .unwrap();
        let gradient = Cotangent::<DenseY, 2>::new([1.0, -2.0]).unwrap();

        let receipt = observable_sensitivity(
            &jacobian,
            &input_metric,
            &output_metric,
            &gradient,
            0.1,
            &CurvatureEvidenceV1::AnalyticUpperBound {
                second_derivative_norm_bound: 0.0,
                valid_radius: 0.1,
                evidence_id: "affine-observable-local-bound".to_owned(),
            },
            operator_profile(),
        )
        .unwrap();

        // J^T g = [2, -5]; the input dual norm is sqrt(29).
        close(receipt.pulled_back_gradient.components()[0], 2.0, 1.0e-14);
        close(receipt.pulled_back_gradient.components()[1], -5.0, 1.0e-14);
        close(receipt.input_gradient_dual_norm, 29.0_f64.sqrt(), 1.0e-14);
        close(
            receipt.first_order_observable_change_estimate,
            29.0_f64.sqrt() * 0.1,
            1.0e-14,
        );
    }

    #[test]
    fn output_and_adjoint_sensitivity_are_invariant_under_dense_rebasing() {
        let profile = metric_profile();
        let operator_profile = operator_profile();
        let metric_x_q = Metric::<CartesianQ, 2>::admit([[2.0, 0.0], [0.0, 3.0]], profile).unwrap();
        let metric_y_q = Metric::<CartesianQ, 2>::admit([[4.0, 0.0], [0.0, 2.0]], profile).unwrap();
        let sx = Rebasis::<CartesianQ, DenseY, 2>::new([[1.0, 2.0], [0.0, 1.0]], profile).unwrap();
        let sy = Rebasis::<CartesianQ, DenseY, 2>::new([[1.0, -1.0], [0.0, 1.0]], profile).unwrap();
        let metric_x_y = sx.metric_to_to(&metric_x_q, profile).unwrap();
        let metric_y_y = sy.metric_to_to(&metric_y_q, profile).unwrap();
        let operator_q = [[1.0, 2.0], [-1.0, 3.0]];
        let operator_y = transform_rectangular_operator(&operator_q, &sx, &sy);
        let jacobian_q =
            RectangularTangentToTangentOperator::<CartesianQ, CartesianQ, 2, 2>::new(operator_q)
                .unwrap();
        let jacobian_y =
            RectangularTangentToTangentOperator::<DenseY, DenseY, 2, 2>::new(operator_y).unwrap();

        let curvature = CurvatureEvidenceV1::AnalyticUpperBound {
            second_derivative_norm_bound: 1.25,
            valid_radius: 0.05,
            evidence_id: "same-physical-ball-hessian-v1".to_owned(),
        };
        let output_q = propagate_output_error(
            &jacobian_q,
            &metric_x_q,
            &metric_y_q,
            0.05,
            &curvature,
            operator_profile,
        )
        .unwrap();
        let output_y = propagate_output_error(
            &jacobian_y,
            &metric_x_y,
            &metric_y_y,
            0.05,
            &curvature,
            operator_profile,
        )
        .unwrap();

        close(
            output_q.derivative_operator_norm_estimate,
            output_y.derivative_operator_norm_estimate,
            1.0e-10,
        );
        close(
            output_q.total_output_error_upper_estimate.unwrap(),
            output_y.total_output_error_upper_estimate.unwrap(),
            1.0e-10,
        );

        let gradient_q = Cotangent::<CartesianQ, 2>::new([0.75, -1.25]).unwrap();
        let gradient_y = sy.cotangent_to_to(&gradient_q).unwrap();
        let observable_q = observable_sensitivity(
            &jacobian_q,
            &metric_x_q,
            &metric_y_q,
            &gradient_q,
            0.05,
            &CurvatureEvidenceV1::IntervalUpperBound {
                second_derivative_norm_upper: 0.75,
                valid_radius: 0.05,
                evidence_id: "composite-observable-interval-v1".to_owned(),
            },
            operator_profile,
        )
        .unwrap();
        let observable_y = observable_sensitivity(
            &jacobian_y,
            &metric_x_y,
            &metric_y_y,
            &gradient_y,
            0.05,
            &CurvatureEvidenceV1::IntervalUpperBound {
                second_derivative_norm_upper: 0.75,
                valid_radius: 0.05,
                evidence_id: "composite-observable-interval-v1".to_owned(),
            },
            operator_profile,
        )
        .unwrap();

        close(
            observable_q.input_gradient_dual_norm,
            observable_y.input_gradient_dual_norm,
            1.0e-10,
        );
        close(
            observable_q.total_observable_change_upper_estimate.unwrap(),
            observable_y.total_observable_change_upper_estimate.unwrap(),
            1.0e-10,
        );
    }

    #[test]
    fn rank_deficiency_does_not_invalidate_a_forward_norm_estimate() {
        let input = identity_metric::<CartesianQ, 3>();
        let output = identity_metric::<DenseY, 2>();
        let jacobian = RectangularTangentToTangentOperator::<CartesianQ, DenseY, 2, 3>::new([
            [1.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
        ])
        .unwrap();
        let receipt = propagate_output_error(
            &jacobian,
            &input,
            &output,
            0.1,
            &CurvatureEvidenceV1::AnalyticUpperBound {
                second_derivative_norm_bound: 0.0,
                valid_radius: 0.1,
                evidence_id: "linear-rank-two-map-v1".to_owned(),
            },
            operator_profile(),
        )
        .unwrap();
        close(receipt.derivative_operator_norm_estimate, 2.0, 1.0e-12);
        close(
            receipt.total_output_error_upper_estimate.unwrap(),
            0.2,
            1.0e-12,
        );
        assert_eq!(
            receipt.operator_condition.status,
            RectangularOperatorStatusV1::NonInjective
        );
    }

    #[test]
    fn invalid_radius_curvature_and_evidence_identity_fail_closed() {
        let input = identity_metric::<CartesianQ, 1>();
        let output = identity_metric::<DenseY, 1>();
        let jacobian =
            RectangularTangentToTangentOperator::<CartesianQ, DenseY, 1, 1>::new([[1.0]]).unwrap();
        assert_eq!(
            propagate_output_error(
                &jacobian,
                &input,
                &output,
                f64::NAN,
                &CurvatureEvidenceV1::Missing,
                operator_profile()
            ),
            Err(NonlinearSensitivityErrorV1::InvalidPerturbationRadius)
        );
        assert_eq!(
            propagate_output_error(
                &jacobian,
                &input,
                &output,
                0.1,
                &CurvatureEvidenceV1::AnalyticUpperBound {
                    second_derivative_norm_bound: f64::INFINITY,
                    valid_radius: 0.1,
                    evidence_id: "invalid".to_owned()
                },
                operator_profile()
            ),
            Err(NonlinearSensitivityErrorV1::InvalidCurvatureBound)
        );
        assert_eq!(
            propagate_output_error(
                &jacobian,
                &input,
                &output,
                0.1,
                &CurvatureEvidenceV1::AnalyticUpperBound {
                    second_derivative_norm_bound: 1.0,
                    valid_radius: 0.1,
                    evidence_id: " ".to_owned()
                },
                operator_profile()
            ),
            Err(NonlinearSensitivityErrorV1::InvalidEvidenceIdentity)
        );
    }

    #[test]
    fn curvature_evidence_cannot_be_reused_beyond_its_validity_radius() {
        let input = identity_metric::<CartesianQ, 1>();
        let output = identity_metric::<DenseY, 1>();
        let jacobian =
            RectangularTangentToTangentOperator::<CartesianQ, DenseY, 1, 1>::new([[1.0]]).unwrap();

        assert_eq!(
            propagate_output_error(
                &jacobian,
                &input,
                &output,
                0.2,
                &CurvatureEvidenceV1::AnalyticUpperBound {
                    second_derivative_norm_bound: 2.0,
                    valid_radius: 0.1,
                    evidence_id: "bound-only-on-radius-0.1".to_owned(),
                },
                operator_profile()
            ),
            Err(NonlinearSensitivityErrorV1::CurvatureEvidenceRadiusExceeded)
        );
    }

    #[test]
    fn empirical_observable_curvature_never_becomes_a_bound() {
        let input = identity_metric::<CartesianQ, 1>();
        let output = identity_metric::<DenseY, 1>();
        let jacobian =
            RectangularTangentToTangentOperator::<CartesianQ, DenseY, 1, 1>::new([[2.0]]).unwrap();
        let gradient = Cotangent::<DenseY, 1>::new([3.0]).unwrap();
        let receipt = observable_sensitivity(
            &jacobian,
            &input,
            &output,
            &gradient,
            0.1,
            &CurvatureEvidenceV1::EmpiricalEstimate {
                second_derivative_norm_estimate: 10.0,
                valid_radius: 0.1,
                evidence_id: "finite-difference-curvature-samples".to_owned(),
            },
            operator_profile(),
        )
        .unwrap();
        assert_eq!(receipt.total_observable_change_upper_estimate, None);
        assert_eq!(
            receipt.status,
            NonlinearSensitivityStatusV1::EmpiricalEstimateOnly
        );
    }
}
