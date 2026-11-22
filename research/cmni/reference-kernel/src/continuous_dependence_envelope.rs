//! Bracket worst-case scalar-parameter information over a continuous
//! PSD-constrained source-correlation set.
//!
//! For V(R) = D_epsilon + J D_sigma R D_sigma J^T and
//! f(R) = h^T V(R)^-1 h, convexity of the matrix-fractional function gives
//! the supporting-hyperplane bound
//!
//!   f(R) >= f(R0) - b^T (R - R0) b,
//!   b = D_sigma J^T V(R0)^-1 h.
//!
//! Maximizing b^T R b over independent pairwise correlation intervals is a
//! relaxation of the true elliptope-constrained maximum, so the tangent
//! minimum is a lower bound on min_R f(R). Any validated PSD witness R_w gives
//! f(R_w) as an upper bound on min_R f(R).
//!
//! Multiple feasible witnesses create multiple tangents; the maximum of their
//! lower bounds remains a lower bound. This brackets the continuous worst case
//! without claiming that a finite scenario grid covers the whole set.
//!
//! The mathematical inequalities hold over real arithmetic. This implementation
//! uses f64 and an explicit numerical safety margin; it is not a formal interval
//! certificate. The tangent relaxation deliberately ignores PSD coupling while
//! maximizing the tangent's quadratic term, so the lower bound may be loose.

use crate::multisource_dependence_design::{
    canonicalize_and_validate_correlation_matrix, scalar_parameter_information_and_source_gradient,
    validate_candidate, FiniteCorrelationScenario, MultiSourceDependenceDesignCandidate,
};
use crate::ModelError;

/// Inclusive correlation interval for one off-diagonal source pair.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CorrelationCoefficientBounds {
    pub lower: f64,
    pub upper: f64,
}

/// Complete pairwise interval specification for a source-correlation matrix.
///
/// The matrix is square and symmetric by entry: bounds[i][j] and bounds[j][i]
/// must match. Diagonal entries are the singleton interval [1,1].
#[derive(Debug, Clone, PartialEq)]
pub struct PairwiseCorrelationIntervalSet {
    pub bounds: Vec<Vec<CorrelationCoefficientBounds>>,
}

/// Continuous dependence envelope for the worst-case EIG of one candidate.
#[derive(Debug, Clone, PartialEq)]
pub struct ContinuousDependenceEIGEnvelope {
    pub candidate_id: String,
    pub dependence_scope_receipt_id: String,
    /// Lower/upper bracket on min_R h^T V(R)^-1 h over the continuous set.
    pub worst_case_parameter_information_bounds: [f64; 2],
    /// Lower/upper bracket on the minimum EIG in nats.
    pub worst_case_eig_nats_bounds: [f64; 2],
    /// Lower/upper bracket on posterior variance at worst-case dependence.
    pub worst_case_posterior_variance_bounds: [f64; 2],
    /// Feasible matrices used as tangent anchors for the lower bound.
    pub tangent_anchor_scenario_ids: Vec<String>,
    /// Feasible witness with the smallest observed information; this gives the
    /// upper bound on the continuous minimum.
    pub best_feasible_witness_scenario_id: String,
    pub feasible_witness_count: usize,
    /// The tangent relaxation maximizes over independent pairwise intervals and
    /// therefore ignores PSD coupling when calculating its conservative lower.
    pub lower_bound_uses_pairwise_interval_relaxation: bool,
    pub minimum_eigenvalue_tolerance: f64,
    pub jacobi_off_diagonal_tolerance: f64,
    pub relative_information_safety_margin: f64,
    /// Always false until directed-rounding interval arithmetic is implemented.
    pub formal_interval_arithmetic: bool,
}

/// Bound the worst-case information across all PSD correlation matrices inside
/// a pairwise interval set.
///
/// At least one complete correlation matrix must be supplied as a feasible
/// witness. Each witness is validated for PSD and membership in the pairwise
/// bounds. Every witness is used as both:
/// - a feasible point, yielding an upper bound on the continuous minimum; and
/// - an anchor for a convex supporting hyperplane, yielding a lower bound.
///
/// For source-coordinate vector b, the relaxed maximum is
///
///   sum_i b_i^2 + 2 sum_{i<j} max(b_i*b_j*l_ij, b_i*b_j*u_ij).
///
/// This routine does not generate witnesses or solve the continuous SDP.
pub fn bound_continuous_source_dependence(
    candidate: &MultiSourceDependenceDesignCandidate,
    correlation_intervals: &PairwiseCorrelationIntervalSet,
    feasible_witnesses: &[FiniteCorrelationScenario],
    dependence_scope_receipt_id: &str,
) -> Result<ContinuousDependenceEIGEnvelope, ModelError> {
    if candidate.candidate_id.trim().is_empty() {
        return Err(ModelError::InvalidParameter(
            "continuous_dependence_candidate_id_must_be_nonempty",
        ));
    }
    if dependence_scope_receipt_id.trim().is_empty() {
        return Err(ModelError::InvalidParameter(
            "continuous_dependence_scope_receipt_must_be_nonempty",
        ));
    }
    if feasible_witnesses.is_empty() {
        return Err(ModelError::InvalidParameter(
            "continuous_dependence_requires_at_least_one_feasible_witness",
        ));
    }

    let observation_count = candidate.observation_sensitivities.len();
    let source_count = candidate.source_standard_deviations.len();
    validate_candidate(candidate, observation_count, source_count)?;
    validate_pairwise_interval_set(correlation_intervals, source_count)?;

    let mut seen_witness_ids = std::collections::BTreeSet::new();
    let mut best_feasible_information = f64::INFINITY;
    let mut best_feasible_witness_scenario_id = String::new();
    let mut best_tangent_lower_bound = 0.0_f64;
    let mut tangent_anchor_scenario_ids = Vec::with_capacity(feasible_witnesses.len());

    for witness in feasible_witnesses {
        if witness.scenario_id.trim().is_empty()
            || !seen_witness_ids.insert(witness.scenario_id.as_str())
        {
            return Err(ModelError::InvalidParameter(
                "continuous_dependence_witness_ids_must_be_nonempty_and_unique",
            ));
        }

        let correlation = canonicalize_and_validate_correlation_matrix(
            &witness.correlation_matrix,
            source_count,
        )?;
        validate_witness_within_intervals(&correlation, correlation_intervals)?;

        let (anchor_information, source_gradient) =
            scalar_parameter_information_and_source_gradient(candidate, &correlation)?;
        if anchor_information < best_feasible_information {
            best_feasible_information = anchor_information;
            best_feasible_witness_scenario_id = witness.scenario_id.clone();
        }

        let anchor_quadratic = correlation_quadratic_form(&source_gradient, &correlation);
        let relaxed_quadratic_maximum =
            relaxed_pairwise_quadratic_maximum(&source_gradient, correlation_intervals);
        let raw_tangent_lower =
            anchor_information + anchor_quadratic - relaxed_quadratic_maximum;

        let arithmetic_scale = anchor_information
            .abs()
            .max(anchor_quadratic.abs())
            .max(relaxed_quadratic_maximum.abs())
            .max(1.0);
        let numerical_safety_margin = 1.0e-12 * arithmetic_scale;
        let conservative_tangent_lower =
            (raw_tangent_lower - numerical_safety_margin).max(0.0);
        best_tangent_lower_bound = best_tangent_lower_bound.max(conservative_tangent_lower);
        tangent_anchor_scenario_ids.push(witness.scenario_id.clone());
    }

    let numerical_ordering_tolerance = 1.0e-10 * best_feasible_information.abs().max(1.0);
    if best_tangent_lower_bound > best_feasible_information + numerical_ordering_tolerance {
        return Err(ModelError::InvalidParameter(
            "continuous_dependence_lower_bound_exceeds_feasible_upper_bound",
        ));
    }
    let lower_information = best_tangent_lower_bound.min(best_feasible_information);
    let upper_information = best_feasible_information;

    let prior_variance = candidate.prior_parameter_variance;
    let lower_eig = 0.5 * (1.0 + prior_variance * lower_information).ln();
    let upper_eig = 0.5 * (1.0 + prior_variance * upper_information).ln();
    let lower_posterior_variance =
        1.0 / (1.0 / prior_variance + upper_information);
    let upper_posterior_variance =
        1.0 / (1.0 / prior_variance + lower_information);
    if [
        lower_eig,
        upper_eig,
        lower_posterior_variance,
        upper_posterior_variance,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return Err(ModelError::InvalidParameter(
            "continuous_dependence_envelope_nonfinite",
        ));
    }

    Ok(ContinuousDependenceEIGEnvelope {
        candidate_id: candidate.candidate_id.clone(),
        dependence_scope_receipt_id: dependence_scope_receipt_id.to_owned(),
        worst_case_parameter_information_bounds: [lower_information, upper_information],
        worst_case_eig_nats_bounds: [lower_eig, upper_eig],
        worst_case_posterior_variance_bounds: [
            lower_posterior_variance,
            upper_posterior_variance,
        ],
        tangent_anchor_scenario_ids,
        best_feasible_witness_scenario_id,
        feasible_witness_count: feasible_witnesses.len(),
        lower_bound_uses_pairwise_interval_relaxation: true,
        minimum_eigenvalue_tolerance: 1.0e-10,
        jacobi_off_diagonal_tolerance: 1.0e-14,
        relative_information_safety_margin: 1.0e-12,
        formal_interval_arithmetic: false,
    })
}

fn validate_pairwise_interval_set(
    intervals: &PairwiseCorrelationIntervalSet,
    source_count: usize,
) -> Result<(), ModelError> {
    if intervals.bounds.len() != source_count
        || intervals.bounds.iter().any(|row| row.len() != source_count)
    {
        return Err(ModelError::MismatchedDataLengths);
    }

    for row in 0..source_count {
        for column in 0..source_count {
            let interval = intervals.bounds[row][column];
            if !interval.lower.is_finite() || !interval.upper.is_finite() {
                return Err(ModelError::NonFiniteValue(
                    "continuous_dependence_pairwise_interval",
                ));
            }
            if interval.lower < -1.0
                || interval.upper > 1.0
                || interval.lower > interval.upper
            {
                return Err(ModelError::InvalidParameter(
                    "continuous_dependence_pairwise_interval_invalid",
                ));
            }
            if row == column {
                if !nearly_equal(interval.lower, 1.0)
                    || !nearly_equal(interval.upper, 1.0)
                {
                    return Err(ModelError::InvalidParameter(
                        "continuous_dependence_diagonal_bounds_must_be_one",
                    ));
                }
            } else {
                let transpose = intervals.bounds[column][row];
                if !nearly_equal(interval.lower, transpose.lower)
                    || !nearly_equal(interval.upper, transpose.upper)
                {
                    return Err(ModelError::InvalidParameter(
                        "continuous_dependence_pairwise_bounds_must_be_symmetric",
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_witness_within_intervals(
    correlation: &[Vec<f64>],
    intervals: &PairwiseCorrelationIntervalSet,
) -> Result<(), ModelError> {
    for row in 0..correlation.len() {
        for column in 0..correlation.len() {
            let interval = intervals.bounds[row][column];
            let value = correlation[row][column];
            if value < interval.lower - 1.0e-12 || value > interval.upper + 1.0e-12 {
                return Err(ModelError::InvalidParameter(
                    "continuous_dependence_witness_outside_pairwise_bounds",
                ));
            }
        }
    }
    Ok(())
}

fn correlation_quadratic_form(vector: &[f64], correlation: &[Vec<f64>]) -> f64 {
    let mut value = 0.0;
    for (row, row_value) in vector.iter().enumerate() {
        for (column, column_value) in vector.iter().enumerate() {
            value += row_value * correlation[row][column] * column_value;
        }
    }
    value
}

/// Maximum of b^T R b over the pairwise interval relaxation, with R_ii=1.
fn relaxed_pairwise_quadratic_maximum(
    vector: &[f64],
    intervals: &PairwiseCorrelationIntervalSet,
) -> f64 {
    let mut maximum = vector.iter().map(|value| value * value).sum::<f64>();
    for row in 0..vector.len() {
        for column in (row + 1)..vector.len() {
            let coefficient = 2.0 * vector[row] * vector[column];
            let interval = intervals.bounds[row][column];
            maximum += (coefficient * interval.lower).max(coefficient * interval.upper);
        }
    }
    maximum
}

fn nearly_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1.0e-12 * left.abs().max(right.abs()).max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scalar_candidate() -> MultiSourceDependenceDesignCandidate {
        MultiSourceDependenceDesignCandidate {
            candidate_id: "candidate-a".to_owned(),
            prior_parameter_variance: 1.0,
            observation_sensitivities: vec![1.0],
            independent_observation_noise_variances: vec![1.0],
            source_standard_deviations: vec![1.0, 1.0],
            source_jacobians: vec![vec![1.0, 1.0]],
        }
    }

    fn interval_set(lower: f64, upper: f64) -> PairwiseCorrelationIntervalSet {
        PairwiseCorrelationIntervalSet {
            bounds: vec![
                vec![
                    CorrelationCoefficientBounds {
                        lower: 1.0,
                        upper: 1.0,
                    },
                    CorrelationCoefficientBounds { lower, upper },
                ],
                vec![
                    CorrelationCoefficientBounds { lower, upper },
                    CorrelationCoefficientBounds {
                        lower: 1.0,
                        upper: 1.0,
                    },
                ],
            ],
        }
    }

    fn witness(id: &str, correlation: f64) -> FiniteCorrelationScenario {
        FiniteCorrelationScenario {
            scenario_id: id.to_owned(),
            correlation_matrix: vec![vec![1.0, correlation], vec![correlation, 1.0]],
        }
    }

    const RECEIPT: &str = "receipt:continuous-source-correlation-range-v1";

    #[test]
    fn one_observation_bound_is_tight_at_adverse_correlation_endpoint() {
        let result = bound_continuous_source_dependence(
            &scalar_candidate(),
            &interval_set(-0.5, 0.5),
            &[witness("worst-correlation", 0.5), witness("best-correlation", -0.5)],
            RECEIPT,
        )
        .expect("continuous scalar correlation envelope");

        // V = 1 + [1 1] R [1 1]^T = 3 + 2 rho; minimum information is 1/4.
        assert!((result.worst_case_parameter_information_bounds[0] - 0.25).abs() < 1.0e-12);
        assert!((result.worst_case_parameter_information_bounds[1] - 0.25).abs() < 1.0e-12);
        assert!(
            (result.worst_case_eig_nats_bounds[0] - 0.5 * 1.25_f64.ln()).abs() < 1.0e-12
        );
        assert_eq!(result.best_feasible_witness_scenario_id, "worst-correlation");
        assert!(!result.formal_interval_arithmetic);
    }

    #[test]
    fn tangent_bracket_contains_dense_continuous_correlation_sweep() {
        let candidate = MultiSourceDependenceDesignCandidate {
            candidate_id: "two-observation".to_owned(),
            prior_parameter_variance: 1.0,
            observation_sensitivities: vec![-2.0, -2.0],
            independent_observation_noise_variances: vec![1.0, 1.0],
            source_standard_deviations: vec![1.0, 1.0],
            source_jacobians: vec![vec![-1.0, 1.0], vec![0.0, 2.0]],
        };
        let bounds = interval_set(-0.75, 0.75);
        let result = bound_continuous_source_dependence(
            &candidate,
            &bounds,
            &[witness("rho-negative", -0.75), witness("rho-positive", 0.75)],
            RECEIPT,
        )
        .expect("two-observation continuous envelope");

        let mut dense_minimum = f64::INFINITY;
        for step in 0..=10_000 {
            let rho = -0.75 + 1.5 * step as f64 / 10_000.0;
            let v11 = 3.0 - 2.0 * rho;
            let v22 = 5.0;
            let v12 = 2.0 - 2.0 * rho;
            let determinant = v11 * v22 - v12 * v12;
            let h1 = -2.0;
            let h2 = -2.0;
            let information =
                (h1 * h1 * v22 - 2.0 * h1 * h2 * v12 + h2 * h2 * v11) / determinant;
            dense_minimum = dense_minimum.min(information);
            assert!(
                information >= result.worst_case_parameter_information_bounds[0] - 1.0e-10
            );
            assert!(
                information <= result.worst_case_parameter_information_bounds[1] + 1.0e-10
            );
        }
        assert!(
            result.worst_case_parameter_information_bounds[0]
                <= dense_minimum + 1.0e-12
        );
        assert!(
            result.worst_case_parameter_information_bounds[1]
                >= dense_minimum - 1.0e-12
        );
    }

    #[test]
    fn non_psd_or_out_of_interval_witnesses_are_rejected() {
        let invalid_psd = FiniteCorrelationScenario {
            scenario_id: "non-psd".to_owned(),
            correlation_matrix: vec![
                vec![1.0, -0.9, -0.9],
                vec![-0.9, 1.0, -0.9],
                vec![-0.9, -0.9, 1.0],
            ],
        };
        let three_source = MultiSourceDependenceDesignCandidate {
            candidate_id: "three-source".to_owned(),
            prior_parameter_variance: 1.0,
            observation_sensitivities: vec![1.0],
            independent_observation_noise_variances: vec![1.0],
            source_standard_deviations: vec![1.0, 1.0, 1.0],
            source_jacobians: vec![vec![1.0, 1.0, 1.0]],
        };
        let bounds = PairwiseCorrelationIntervalSet {
            bounds: vec![
                vec![
                    CorrelationCoefficientBounds { lower: 1.0, upper: 1.0 },
                    CorrelationCoefficientBounds { lower: -0.9, upper: 0.0 },
                    CorrelationCoefficientBounds { lower: -0.9, upper: 0.0 },
                ],
                vec![
                    CorrelationCoefficientBounds { lower: -0.9, upper: 0.0 },
                    CorrelationCoefficientBounds { lower: 1.0, upper: 1.0 },
                    CorrelationCoefficientBounds { lower: -0.9, upper: 0.0 },
                ],
                vec![
                    CorrelationCoefficientBounds { lower: -0.9, upper: 0.0 },
                    CorrelationCoefficientBounds { lower: -0.9, upper: 0.0 },
                    CorrelationCoefficientBounds { lower: 1.0, upper: 1.0 },
                ],
            ],
        };
        assert_eq!(
            bound_continuous_source_dependence(&three_source, &bounds, &[invalid_psd], RECEIPT),
            Err(ModelError::InvalidCovariance(
                "source_correlation_matrix_must_be_positive_semidefinite"
            ))
        );

        assert_eq!(
            bound_continuous_source_dependence(
                &scalar_candidate(),
                &interval_set(-0.5, 0.5),
                &[witness("outside", 0.75)],
                RECEIPT,
            ),
            Err(ModelError::InvalidParameter(
                "continuous_dependence_witness_outside_pairwise_bounds"
            ))
        );
    }

    #[test]
    fn malformed_intervals_and_empty_witnesses_fail_closed() {
        let mut invalid = interval_set(-0.5, 0.5);
        invalid.bounds[0][1].upper = 1.2;
        invalid.bounds[1][0].upper = 1.2;
        assert_eq!(
            bound_continuous_source_dependence(
                &scalar_candidate(),
                &invalid,
                &[witness("valid", 0.0)],
                RECEIPT,
            ),
            Err(ModelError::InvalidParameter(
                "continuous_dependence_pairwise_interval_invalid"
            ))
        );
        assert_eq!(
            bound_continuous_source_dependence(
                &scalar_candidate(),
                &interval_set(-0.5, 0.5),
                &[],
                RECEIPT,
            ),
            Err(ModelError::InvalidParameter(
                "continuous_dependence_requires_at_least_one_feasible_witness"
            ))
        );
    }

    #[test]
    fn additional_feasible_tangents_cannot_weaken_lower_bound() {
        let candidate = MultiSourceDependenceDesignCandidate {
            candidate_id: "two-observation".to_owned(),
            prior_parameter_variance: 1.0,
            observation_sensitivities: vec![1.0, -0.5],
            independent_observation_noise_variances: vec![1.0, 1.0],
            source_standard_deviations: vec![1.0, 1.0],
            source_jacobians: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
        };
        let bounds = interval_set(-0.75, 0.75);
        let one_anchor = bound_continuous_source_dependence(
            &candidate,
            &bounds,
            &[witness("zero", 0.0)],
            RECEIPT,
        )
        .expect("single tangent anchor");
        let three_anchors = bound_continuous_source_dependence(
            &candidate,
            &bounds,
            &[
                witness("negative", -0.75),
                witness("zero", 0.0),
                witness("positive", 0.75),
            ],
            RECEIPT,
        )
        .expect("three tangent anchors");
        assert!(
            three_anchors.worst_case_parameter_information_bounds[0] + 1.0e-12
                >= one_anchor.worst_case_parameter_information_bounds[0]
        );
        assert_eq!(three_anchors.feasible_witness_count, 3);
    }
}
