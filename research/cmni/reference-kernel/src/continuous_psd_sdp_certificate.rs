//! Floating-point verifier for primal/dual witnesses of the continuous
//! scalar-parameter PSD-correlation SDP.
//!
//! Primal:
//!   minimize t
//!   subject to [ V(R)  h ; h^T  t ] >= 0,
//!              R >= 0, diag(R)=1, l_ij <= R_ij <= u_ij.
//!
//! The observation covariance V(R) is affine in the source correlation R.
//! The verifier checks a feasible primal point and a dual feasible point,
//! including all stationarity equations, nonnegative bound multipliers, PSD
//! matrices, and the primal-dual gap.
//!
//! This is a numerical f64 verification, not a formal proof. PSD, symmetry,
//! stationarity and gap checks use caller-supplied tolerances and a Jacobi
//! eigenvalue routine. A formal certificate requires directed rounding or
//! independent exact/interval arithmetic.

use crate::continuous_dependence_envelope::{
    validate_pairwise_interval_set, validate_witness_within_intervals,
    PairwiseCorrelationIntervalSet,
};
use crate::multisource_dependence_design::{
    canonicalize_and_validate_correlation_matrix, minimum_symmetric_eigenvalue, validate_candidate,
    MultiSourceDependenceDesignCandidate,
};
use crate::ModelError;

/// Dual multipliers for one off-diagonal correlation interval.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CorrelationBoundDualMultipliers {
    pub source_i: usize,
    pub source_j: usize,
    /// Multiplier for lower - R_ij <= 0.
    pub lower_multiplier: f64,
    /// Multiplier for R_ij - upper <= 0.
    pub upper_multiplier: f64,
}

/// A primal/dual candidate for the continuous scalar-information SDP.
#[derive(Debug, Clone, PartialEq)]
pub struct ContinuousDependenceSdpCertificate {
    pub certificate_id: String,
    /// External receipt reference that qualifies the source definitions and bounds.
    /// This string is not authenticated by this numerical verifier.
    pub dependence_scope_receipt_id: String,
    /// Feasible source correlation matrix R.
    pub primal_correlation_matrix: Vec<Vec<f64>>,
    /// Epigraph variable t for the block LMI.
    pub primal_epigraph_t: f64,
    /// PSD dual matrix X for [V(R) h; h^T t] >= 0.
    pub observation_lmi_dual: Vec<Vec<f64>>,
    /// PSD dual matrix Y for R >= 0.
    pub correlation_psd_dual: Vec<Vec<f64>>,
    /// One lower/upper multiplier pair for every i<j.
    pub pairwise_bound_multipliers: Vec<CorrelationBoundDualMultipliers>,
    /// Free multipliers for the equalities R_ii=1.
    pub diagonal_equality_multipliers: Vec<f64>,
}

/// Floating-point acceptance tolerances for certificate verification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SdpCertificateVerificationPolicy {
    /// Absolute tolerance scaled by the magnitude of each matrix/equation.
    pub feasibility_tolerance: f64,
}

/// Status of a checked primal/dual result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdpCertificateVerificationStatus {
    VerifiedWithinFloatingPointTolerance,
}

/// Results of the independent primal/dual verification.
#[derive(Debug, Clone, PartialEq)]
pub struct ContinuousDependenceSdpVerification {
    pub certificate_id: String,
    pub dependence_scope_receipt_id: String,
    pub primal_information_upper_bound: f64,
    pub raw_dual_information_lower_bound: f64,
    /// max(0, raw dual bound), using non-negativity of Fisher information.
    pub nonnegative_dual_information_lower_bound: f64,
    pub primal_dual_gap: f64,
    pub worst_case_eig_nats_bounds: [f64; 2],
    pub maximum_stationarity_residual: f64,
    pub minimum_eigenvalue_primal_correlation: f64,
    pub minimum_eigenvalue_observation_lmi: f64,
    pub minimum_eigenvalue_observation_dual: f64,
    pub minimum_eigenvalue_correlation_dual: f64,
    pub feasibility_tolerance: f64,
    pub status: SdpCertificateVerificationStatus,
    /// Always false: this implementation has no directed-rounding arithmetic.
    pub formal_certificate: bool,
}

/// Verify a primal/dual certificate for the scalar-parameter continuous
/// correlation SDP.
///
/// Let n be the observation count and m the shared-source count. The primal
/// block matrix is M(R,t)=[V(R),h;h^T,t]. The dual variables are X>=0, Y>=0,
/// nonnegative pairwise-bound multipliers, and free diagonal multipliers alpha.
///
/// Stationarity checked by this verifier:
/// - 1 - X[n,n] = 0 for t;
/// - -<X,A_ii> - Y_ii + alpha_i = 0 for every R_ii;
/// - -<X,A_ij> - 2Y_ij - lambda_lower + lambda_upper = 0 for every i<j.
///
/// The dual objective is:
/// -<X,M0> + sum_{i<j}(lambda_lower*l_ij - lambda_upper*u_ij) - sum_i alpha_i,
/// where M0=[D_epsilon,h;h^T,0].
///
/// All matrix PSD checks are performed with a small-matrix Jacobi eigenvalue
/// routine. This is a reviewable numerical check, not a formal proof.
pub fn verify_continuous_dependence_sdp_certificate(
    candidate: &MultiSourceDependenceDesignCandidate,
    correlation_intervals: &PairwiseCorrelationIntervalSet,
    certificate: &ContinuousDependenceSdpCertificate,
    policy: SdpCertificateVerificationPolicy,
) -> Result<ContinuousDependenceSdpVerification, ModelError> {
    if certificate.certificate_id.trim().is_empty() {
        return Err(ModelError::InvalidParameter(
            "sdp_certificate_id_must_be_nonempty",
        ));
    }
    if certificate.dependence_scope_receipt_id.trim().is_empty() {
        return Err(ModelError::InvalidParameter(
            "sdp_dependence_scope_receipt_must_be_nonempty",
        ));
    }
    if !policy.feasibility_tolerance.is_finite() {
        return Err(ModelError::NonFiniteValue("sdp_feasibility_tolerance"));
    }
    if policy.feasibility_tolerance <= 0.0 {
        return Err(ModelError::InvalidParameter(
            "sdp_feasibility_tolerance_must_be_positive",
        ));
    }
    if !certificate.primal_epigraph_t.is_finite() {
        return Err(ModelError::NonFiniteValue("sdp_primal_epigraph_t"));
    }
    if certificate.primal_epigraph_t < 0.0 {
        return Err(ModelError::InvalidParameter(
            "sdp_primal_epigraph_t_must_be_nonnegative",
        ));
    }

    let observation_count = candidate.observation_sensitivities.len();
    let source_count = candidate.source_standard_deviations.len();
    validate_candidate(candidate, observation_count, source_count)?;
    validate_pairwise_interval_set(correlation_intervals, source_count)?;

    let primal_correlation = canonicalize_and_validate_correlation_matrix(
        &certificate.primal_correlation_matrix,
        source_count,
    )?;
    validate_witness_within_intervals(&primal_correlation, correlation_intervals)?;
    let primal_correlation_min_eigenvalue =
        minimum_symmetric_eigenvalue(&primal_correlation)?;
    verify_psd_minimum(
        primal_correlation_min_eigenvalue,
        &primal_correlation,
        policy.feasibility_tolerance,
        "sdp_primal_correlation_matrix_not_psd",
    )?;

    let observation_covariance = build_observation_covariance(candidate, &primal_correlation)?;
    let observation_lmi = build_observation_lmi(
        &observation_covariance,
        &candidate.observation_sensitivities,
        certificate.primal_epigraph_t,
    );
    let primal_lmi_min_eigenvalue = minimum_symmetric_eigenvalue(&observation_lmi)?;
    verify_psd_minimum(
        primal_lmi_min_eigenvalue,
        &observation_lmi,
        policy.feasibility_tolerance,
        "sdp_primal_observation_lmi_not_psd",
    )?;

    let observation_dual = canonicalize_symmetric_matrix(
        &certificate.observation_lmi_dual,
        observation_count + 1,
        policy.feasibility_tolerance,
        "sdp_observation_dual",
    )?;
    let correlation_dual = canonicalize_symmetric_matrix(
        &certificate.correlation_psd_dual,
        source_count,
        policy.feasibility_tolerance,
        "sdp_correlation_dual",
    )?;
    let observation_dual_min_eigenvalue = minimum_symmetric_eigenvalue(&observation_dual)?;
    verify_psd_minimum(
        observation_dual_min_eigenvalue,
        &observation_dual,
        policy.feasibility_tolerance,
        "sdp_observation_dual_not_psd",
    )?;
    let correlation_dual_min_eigenvalue = minimum_symmetric_eigenvalue(&correlation_dual)?;
    verify_psd_minimum(
        correlation_dual_min_eigenvalue,
        &correlation_dual,
        policy.feasibility_tolerance,
        "sdp_correlation_dual_not_psd",
    )?;

    if certificate.diagonal_equality_multipliers.len() != source_count {
        return Err(ModelError::MismatchedDataLengths);
    }
    if certificate
        .diagonal_equality_multipliers
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(ModelError::NonFiniteValue(
            "sdp_diagonal_equality_multiplier",
        ));
    }

    let expected_pair_count = source_count.saturating_mul(source_count.saturating_sub(1)) / 2;
    if certificate.pairwise_bound_multipliers.len() != expected_pair_count {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut seen_pairs = std::collections::BTreeSet::new();
    for multipliers in &certificate.pairwise_bound_multipliers {
        if multipliers.source_i >= source_count
            || multipliers.source_j >= source_count
            || multipliers.source_i >= multipliers.source_j
            || !seen_pairs.insert((multipliers.source_i, multipliers.source_j))
        {
            return Err(ModelError::InvalidParameter(
                "sdp_pairwise_bound_multiplier_indices_invalid",
            ));
        }
        if !multipliers.lower_multiplier.is_finite()
            || !multipliers.upper_multiplier.is_finite()
        {
            return Err(ModelError::NonFiniteValue(
                "sdp_pairwise_bound_multiplier",
            ));
        }
        if multipliers.lower_multiplier < 0.0 || multipliers.upper_multiplier < 0.0 {
            return Err(ModelError::InvalidParameter(
                "sdp_pairwise_bound_multipliers_must_be_nonnegative",
            ));
        }
    }
    for row in 0..source_count {
        for column in (row + 1)..source_count {
            if !seen_pairs.contains(&(row, column)) {
                return Err(ModelError::InvalidParameter(
                    "sdp_pairwise_bound_multiplier_pair_missing",
                ));
            }
        }
    }

    let observation_dual_dimension = observation_count + 1;
    let observation_dual_block = observation_dual[..observation_count]
        .iter()
        .map(|row| row[..observation_count].to_vec())
        .collect::<Vec<_>>();
    let mut maximum_stationarity_residual = (1.0 - observation_dual[observation_count]
        [observation_count])
        .abs();
    let mut stationarity_scale = 1.0_f64.max(observation_dual_dimension as f64);

    for source_index in 0..source_count {
        let source_column: Vec<f64> = (0..observation_count)
            .map(|observation| candidate.source_jacobians[observation][source_index])
            .collect();
        let diagonal_coefficient = candidate.source_standard_deviations[source_index].powi(2)
            * quadratic_form(&source_column, &observation_dual_block);
        let residual = -diagonal_coefficient
            - correlation_dual[source_index][source_index]
            + certificate.diagonal_equality_multipliers[source_index];
        maximum_stationarity_residual = maximum_stationarity_residual.max(residual.abs());
        stationarity_scale = stationarity_scale
            .max(diagonal_coefficient.abs())
            .max(correlation_dual[source_index][source_index].abs())
            .max(certificate.diagonal_equality_multipliers[source_index].abs());
    }

    let mut lower_multiplier_sum = 0.0;
    let mut upper_multiplier_sum = 0.0;
    for multipliers in &certificate.pairwise_bound_multipliers {
        let source_i = multipliers.source_i;
        let source_j = multipliers.source_j;
        let column_i: Vec<f64> = (0..observation_count)
            .map(|observation| candidate.source_jacobians[observation][source_i])
            .collect();
        let column_j: Vec<f64> = (0..observation_count)
            .map(|observation| candidate.source_jacobians[observation][source_j])
            .collect();
        let cross = 2.0
            * candidate.source_standard_deviations[source_i]
            * candidate.source_standard_deviations[source_j]
            * bilinear_form(&column_i, &observation_dual_block, &column_j);
        let residual = -cross
            - 2.0 * correlation_dual[source_i][source_j]
            - multipliers.lower_multiplier
            + multipliers.upper_multiplier;
        maximum_stationarity_residual = maximum_stationarity_residual.max(residual.abs());
        stationarity_scale = stationarity_scale
            .max(cross.abs())
            .max((2.0 * correlation_dual[source_i][source_j]).abs())
            .max(multipliers.lower_multiplier.abs())
            .max(multipliers.upper_multiplier.abs());
        let bounds = correlation_intervals.bounds[source_i][source_j];
        lower_multiplier_sum += multipliers.lower_multiplier * bounds.lower;
        upper_multiplier_sum += multipliers.upper_multiplier * bounds.upper;
    }

    if maximum_stationarity_residual > policy.feasibility_tolerance * stationarity_scale {
        return Err(ModelError::InvalidParameter(
            "sdp_dual_stationarity_residual_exceeds_tolerance",
        ));
    }

    let mut observation_m0_inner_product = 0.0;
    for observation in 0..observation_count {
        observation_m0_inner_product += candidate.independent_observation_noise_variances
            [observation]
            * observation_dual[observation][observation];
        observation_m0_inner_product += 2.0
            * candidate.observation_sensitivities[observation]
            * observation_dual[observation][observation_count];
    }
    let diagonal_multiplier_sum = certificate
        .diagonal_equality_multipliers
        .iter()
        .sum::<f64>();
    let dual_objective = -observation_m0_inner_product + lower_multiplier_sum
        - upper_multiplier_sum
        - diagonal_multiplier_sum;
    if !dual_objective.is_finite() {
        return Err(ModelError::NonFiniteValue("sdp_dual_objective"));
    }

    let primal_objective = certificate.primal_epigraph_t;
    let primal_dual_gap = primal_objective - dual_objective;
    let gap_scale = primal_objective
        .abs()
        .max(dual_objective.abs())
        .max(1.0);
    if primal_dual_gap < -policy.feasibility_tolerance * gap_scale {
        return Err(ModelError::InvalidParameter(
            "sdp_dual_objective_exceeds_primal_objective",
        ));
    }

    let information_lower = dual_objective.min(primal_objective).max(0.0);
    let information_upper = primal_objective;
    let prior_variance = candidate.prior_parameter_variance;
    let eig_bounds = [
        0.5 * (1.0 + prior_variance * information_lower).ln(),
        0.5 * (1.0 + prior_variance * information_upper).ln(),
    ];
    if eig_bounds.iter().any(|value| !value.is_finite()) {
        return Err(ModelError::NonFiniteValue("sdp_certificate_eig_bounds"));
    }

    Ok(ContinuousDependenceSdpVerification {
        certificate_id: certificate.certificate_id.clone(),
        dependence_scope_receipt_id: certificate.dependence_scope_receipt_id.clone(),
        primal_information_upper_bound: primal_objective,
        raw_dual_information_lower_bound: dual_objective,
        nonnegative_dual_information_lower_bound: information_lower,
        primal_dual_gap,
        worst_case_eig_nats_bounds: eig_bounds,
        maximum_stationarity_residual,
        minimum_eigenvalue_primal_correlation: primal_correlation_min_eigenvalue,
        minimum_eigenvalue_observation_lmi: primal_lmi_min_eigenvalue,
        minimum_eigenvalue_observation_dual: observation_dual_min_eigenvalue,
        minimum_eigenvalue_correlation_dual: correlation_dual_min_eigenvalue,
        feasibility_tolerance: policy.feasibility_tolerance,
        status: SdpCertificateVerificationStatus::VerifiedWithinFloatingPointTolerance,
        formal_certificate: false,
    })
}

fn build_observation_covariance(
    candidate: &MultiSourceDependenceDesignCandidate,
    correlation: &[Vec<f64>],
) -> Result<Vec<Vec<f64>>, ModelError> {
    let observation_count = candidate.observation_sensitivities.len();
    let source_count = candidate.source_standard_deviations.len();
    let mut covariance = vec![vec![0.0; observation_count]; observation_count];
    for row in 0..observation_count {
        for column in 0..observation_count {
            covariance[row][column] = if row == column {
                candidate.independent_observation_noise_variances[row]
            } else {
                0.0
            };
            for source_row in 0..source_count {
                for source_column in 0..source_count {
                    let source_covariance = candidate.source_standard_deviations[source_row]
                        * correlation[source_row][source_column]
                        * candidate.source_standard_deviations[source_column];
                    covariance[row][column] += candidate.source_jacobians[row][source_row]
                        * source_covariance
                        * candidate.source_jacobians[column][source_column];
                }
            }
        }
    }
    Ok(covariance)
}

fn build_observation_lmi(
    covariance: &[Vec<f64>],
    observation_sensitivities: &[f64],
    epigraph_t: f64,
) -> Vec<Vec<f64>> {
    let dimension = observation_sensitivities.len();
    let mut matrix = vec![vec![0.0; dimension + 1]; dimension + 1];
    for row in 0..dimension {
        for column in 0..dimension {
            matrix[row][column] = covariance[row][column];
        }
        matrix[row][dimension] = observation_sensitivities[row];
        matrix[dimension][row] = observation_sensitivities[row];
    }
    matrix[dimension][dimension] = epigraph_t;
    matrix
}

fn canonicalize_symmetric_matrix(
    matrix: &[Vec<f64>],
    dimension: usize,
    tolerance: f64,
    name: &'static str,
) -> Result<Vec<Vec<f64>>, ModelError> {
    if matrix.len() != dimension || matrix.iter().any(|row| row.len() != dimension) {
        return Err(ModelError::MismatchedDataLengths);
    }
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(ModelError::NonFiniteValue(name));
    }
    let mut canonical = vec![vec![0.0; dimension]; dimension];
    for row in 0..dimension {
        for column in row..dimension {
            let upper = matrix[row][column];
            let lower = matrix[column][row];
            let scale = upper.abs().max(lower.abs()).max(1.0);
            if (upper - lower).abs() > tolerance * scale {
                return Err(ModelError::InvalidCovariance(
                    "sdp_certificate_matrix_must_be_symmetric",
                ));
            }
            let value = if row == column {
                upper
            } else {
                0.5 * (upper + lower)
            };
            canonical[row][column] = value;
            canonical[column][row] = value;
        }
    }
    Ok(canonical)
}

fn verify_psd_minimum(
    minimum_eigenvalue: f64,
    matrix: &[Vec<f64>],
    tolerance: f64,
    error: &'static str,
) -> Result<(), ModelError> {
    if !minimum_eigenvalue.is_finite() {
        return Err(ModelError::NonFiniteValue("sdp_matrix_minimum_eigenvalue"));
    }
    let scale = matrix
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(1.0_f64, f64::max);
    if minimum_eigenvalue < -tolerance * scale {
        return Err(ModelError::InvalidCovariance(error));
    }
    Ok(())
}

fn quadratic_form(vector: &[f64], matrix: &[Vec<f64>]) -> f64 {
    bilinear_form(vector, matrix, vector)
}

fn bilinear_form(left: &[f64], matrix: &[Vec<f64>], right: &[f64]) -> f64 {
    left.iter()
        .enumerate()
        .map(|(row, left_value)| {
            right
                .iter()
                .enumerate()
                .map(|(column, right_value)| left_value * matrix[row][column] * right_value)
                .sum::<f64>()
        })
        .sum::<f64>()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECEIPT: &str = "receipt:exact-two-source-sdp-example";

    fn simple_problem() -> (
        MultiSourceDependenceDesignCandidate,
        PairwiseCorrelationIntervalSet,
    ) {
        (
            MultiSourceDependenceDesignCandidate {
                candidate_id: "single-observation".to_owned(),
                prior_parameter_variance: 1.0,
                observation_sensitivities: vec![1.0],
                independent_observation_noise_variances: vec![1.0],
                source_standard_deviations: vec![1.0, 1.0],
                source_jacobians: vec![vec![1.0, 1.0]],
            },
            PairwiseCorrelationIntervalSet {
                bounds: vec![
                    vec![
                        CorrelationCoefficientBounds {
                            lower: 1.0,
                            upper: 1.0,
                        },
                        CorrelationCoefficientBounds {
                            lower: -0.5,
                            upper: 0.5,
                        },
                    ],
                    vec![
                        CorrelationCoefficientBounds {
                            lower: -0.5,
                            upper: 0.5,
                        },
                        CorrelationCoefficientBounds {
                            lower: 1.0,
                            upper: 1.0,
                        },
                    ],
                ],
            },
        )
    }

    fn exact_certificate() -> ContinuousDependenceSdpCertificate {
        ContinuousDependenceSdpCertificate {
            certificate_id: "analytic-rho-plus-half".to_owned(),
            dependence_scope_receipt_id: RECEIPT.to_owned(),
            primal_correlation_matrix: vec![vec![1.0, 0.5], vec![0.5, 1.0]],
            primal_epigraph_t: 0.25,
            observation_lmi_dual: vec![vec![0.0625, -0.25], vec![-0.25, 1.0]],
            correlation_psd_dual: vec![vec![0.0, 0.0], vec![0.0, 0.0]],
            pairwise_bound_multipliers: vec![CorrelationBoundDualMultipliers {
                source_i: 0,
                source_j: 1,
                lower_multiplier: 0.0,
                upper_multiplier: 0.125,
            }],
            diagonal_equality_multipliers: vec![0.0625, 0.0625],
        }
    }

    fn policy() -> SdpCertificateVerificationPolicy {
        SdpCertificateVerificationPolicy {
            feasibility_tolerance: 1.0e-8,
        }
    }

    #[test]
    fn verifies_analytic_primal_dual_certificate_with_zero_gap() {
        let (candidate, bounds) = simple_problem();
        let result = verify_continuous_dependence_sdp_certificate(
            &candidate,
            &bounds,
            &exact_certificate(),
            policy(),
        )
        .expect("analytic primal/dual certificate");

        assert!((result.primal_information_upper_bound - 0.25).abs() < 1.0e-12);
        assert!((result.raw_dual_information_lower_bound - 0.25).abs() < 1.0e-12);
        assert!(result.primal_dual_gap.abs() < 1.0e-12);
        assert!(
            (result.worst_case_eig_nats_bounds[0] - 0.5 * 1.25_f64.ln()).abs() < 1.0e-12
        );
        assert!(result.maximum_stationarity_residual < 1.0e-12);
        assert!(!result.formal_certificate);
        assert_eq!(
            result.status,
            SdpCertificateVerificationStatus::VerifiedWithinFloatingPointTolerance
        );
    }

    #[test]
    fn rejects_non_psd_dual_matrix() {
        let (candidate, bounds) = simple_problem();
        let mut certificate = exact_certificate();
        certificate.observation_lmi_dual = vec![vec![1.0, 0.0], vec![0.0, -0.1]];
        assert!(matches!(
            verify_continuous_dependence_sdp_certificate(
                &candidate,
                &bounds,
                &certificate,
                policy(),
            ),
            Err(ModelError::InvalidCovariance("sdp_observation_dual_not_psd"))
        ));
    }

    #[test]
    fn rejects_incorrect_stationarity_even_when_psd_matrices_are_valid() {
        let (candidate, bounds) = simple_problem();
        let mut certificate = exact_certificate();
        certificate.pairwise_bound_multipliers[0].upper_multiplier = 0.0;
        assert_eq!(
            verify_continuous_dependence_sdp_certificate(
                &candidate,
                &bounds,
                &certificate,
                policy(),
            ),
            Err(ModelError::InvalidParameter(
                "sdp_dual_stationarity_residual_exceeds_tolerance"
            ))
        );
    }

    #[test]
    fn rejects_primal_correlation_outside_declared_interval() {
        let (candidate, bounds) = simple_problem();
        let mut certificate = exact_certificate();
        certificate.primal_correlation_matrix = vec![vec![1.0, 0.75], vec![0.75, 1.0]];
        assert_eq!(
            verify_continuous_dependence_sdp_certificate(
                &candidate,
                &bounds,
                &certificate,
                policy(),
            ),
            Err(ModelError::InvalidParameter(
                "continuous_dependence_witness_outside_pairwise_bounds"
            ))
        );
    }

    #[test]
    fn rejects_incomplete_pairwise_multiplier_coverage() {
        let (candidate, bounds) = simple_problem();
        let mut certificate = exact_certificate();
        certificate.pairwise_bound_multipliers.clear();
        assert_eq!(
            verify_continuous_dependence_sdp_certificate(
                &candidate,
                &bounds,
                &certificate,
                policy(),
            ),
            Err(ModelError::MismatchedDataLengths)
        );
    }

    #[test]
    fn rejects_dual_bound_that_exceeds_primal_epigraph() {
        let (candidate, bounds) = simple_problem();
        let mut certificate = exact_certificate();
        certificate.primal_epigraph_t = 0.2;
        assert_eq!(
            verify_continuous_dependence_sdp_certificate(
                &candidate,
                &bounds,
                &certificate,
                policy(),
            ),
            Err(ModelError::InvalidCovariance(
                "sdp_primal_observation_lmi_not_psd"
            ))
        );
    }
}
