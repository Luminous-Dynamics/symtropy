//! Exact partial-identification envelope for two-source dependence uncertainty.
//!
//! Computes global utility extrema when two scalar shared-source uncertainties
//! have known marginal standard deviations but an unknown cross-correlation in
//! an evidence-backed interval. The model has two scalar observations and one
//! scalar linear-Gaussian parameter.
//!
//! The source covariance is:
//!   C = [[sigma_1^2, rho sigma_1 sigma_2],
//!        [rho sigma_1 sigma_2, sigma_2^2]].
//!
//! The returned interval is exact over the declared rho interval. It does not
//! prove that the real physical dependence lies inside that interval. A full
//! evidence receipt must qualify that assumption before the envelope supports
//! a qualified scientific decision.

use crate::ModelError;

/// Scalar linear-Gaussian model with two observations and two shared sources.
///
/// Observation i is y_i = h_i theta + j_i1 u_1 + j_i2 u_2 + epsilon_i.
/// Independent observation noise has strictly positive diagonal variances.
#[derive(Debug, Clone, PartialEq)]
pub struct TwoSourceDependenceProblem {
    pub prior_parameter_variance: f64,
    pub observation_sensitivities: [f64; 2],
    pub independent_observation_noise_variances: [f64; 2],
    pub source_standard_deviations: [f64; 2],
    /// Rows are observations; columns are the two shared-source sensitivities.
    pub source_jacobians: [[f64; 2]; 2],
    /// Inclusive lower/upper bounds for the unknown source correlation.
    pub correlation_bounds: [f64; 2],
}

/// Exact posterior and EIG envelope over every admissible source correlation.
#[derive(Debug, Clone, PartialEq)]
pub struct TwoSourceDependenceEnvelope {
    pub correlation_bounds: [f64; 2],
    /// Global min/max of H^T R(rho)^-1 H.
    pub parameter_information_bounds: [f64; 2],
    /// Global min/max of posterior parameter variance.
    pub posterior_parameter_variance_bounds: [f64; 2],
    /// Exact linear-Gaussian EIG bounds, in nats.
    pub expected_information_gain_nats_bounds: [f64; 2],
    /// Correlations attaining minimum and maximum parameter information.
    pub worst_case_information_correlation: f64,
    pub best_case_information_correlation: f64,
    /// Endpoints plus every in-range stationary point considered.
    pub extremum_candidate_correlations: Vec<f64>,
}

/// Bound posterior variance and linear-Gaussian EIG over a closed correlation
/// interval. The information is a ratio of a linear and quadratic polynomial
/// in rho. Its stationary points are roots of a quadratic; evaluating the
/// endpoints and every in-range stationary point gives global extrema exactly
/// (up to floating-point arithmetic), without a grid or stochastic optimizer.
pub fn bound_two_source_dependence(
    problem: &TwoSourceDependenceProblem,
) -> Result<TwoSourceDependenceEnvelope, ModelError> {
    validate_problem(problem)?;

    let prior = problem.prior_parameter_variance;
    let [h1, h2] = problem.observation_sensitivities;
    let [noise1, noise2] = problem.independent_observation_noise_variances;
    let [sigma1, sigma2] = problem.source_standard_deviations;
    let [[j11, j12], [j21, j22]] = problem.source_jacobians;
    let [rho_min, rho_max] = problem.correlation_bounds;
    let cross_scale = sigma1 * sigma2;

    // R_11(rho) = a0 + a1*rho, R_22(rho) = b0 + b1*rho,
    // R_12(rho) = c0 + c1*rho.
    let a0 = noise1 + j11 * j11 * sigma1 * sigma1 + j12 * j12 * sigma2 * sigma2;
    let a1 = 2.0 * j11 * j12 * cross_scale;
    let b0 = noise2 + j21 * j21 * sigma1 * sigma1 + j22 * j22 * sigma2 * sigma2;
    let b1 = 2.0 * j21 * j22 * cross_scale;
    let c0 = j11 * j21 * sigma1 * sigma1 + j12 * j22 * sigma2 * sigma2;
    let c1 = (j11 * j22 + j12 * j21) * cross_scale;

    // det R(rho) = d0 + d1*rho + d2*rho^2.
    let d0 = a0 * b0 - c0 * c0;
    let d1 = a0 * b1 + a1 * b0 - 2.0 * c0 * c1;
    let d2 = a1 * b1 - c1 * c1;

    // H^T adj(R(rho)) H = n0 + n1*rho.
    let n0 = h1 * h1 * b0 - 2.0 * h1 * h2 * c0 + h2 * h2 * a0;
    let n1 = h1 * h1 * b1 - 2.0 * h1 * h2 * c1 + h2 * h2 * a1;

    let coefficients = [a0, a1, b0, b1, c0, c1, d0, d1, d2, n0, n1];
    if coefficients.iter().any(|value| !value.is_finite()) {
        return Err(ModelError::InvalidParameter(
            "dependence_envelope_coefficients_nonfinite",
        ));
    }

    let mut candidate_correlations = vec![rho_min, rho_max];
    candidate_correlations.extend(stationary_points_of_information(
        n0, n1, d0, d1, d2, rho_min, rho_max,
    ));
    candidate_correlations.sort_by(f64::total_cmp);
    candidate_correlations.dedup_by(|left, right| (*left - *right).abs() <= 1.0e-12);

    let mut minimum_information = f64::INFINITY;
    let mut maximum_information = f64::NEG_INFINITY;
    let mut worst_case_information_correlation = rho_min;
    let mut best_case_information_correlation = rho_min;

    for correlation in &candidate_correlations {
        let rho = *correlation;
        let r11 = a0 + a1 * rho;
        let r22 = b0 + b1 * rho;
        let r12 = c0 + c1 * rho;
        let determinant = r11 * r22 - r12 * r12;
        let numerator = h1 * h1 * r22 - 2.0 * h1 * h2 * r12 + h2 * h2 * r11;
        if !r11.is_finite()
            || !r22.is_finite()
            || !r12.is_finite()
            || !determinant.is_finite()
            || !numerator.is_finite()
            || r11 <= 0.0
            || determinant <= 0.0
        {
            return Err(ModelError::InvalidCovariance(
                "dependence_envelope_observation_covariance",
            ));
        }
        let information = numerator / determinant;
        if !information.is_finite() {
            return Err(ModelError::InvalidParameter(
                "dependence_envelope_information_nonfinite",
            ));
        }
        let tolerance = 1.0e-12 * information.abs().max(1.0);
        if information < -tolerance {
            return Err(ModelError::InvalidParameter(
                "dependence_envelope_information_negative",
            ));
        }
        let information = information.max(0.0);
        if information < minimum_information {
            minimum_information = information;
            worst_case_information_correlation = rho;
        }
        if information > maximum_information {
            maximum_information = information;
            best_case_information_correlation = rho;
        }
    }

    let posterior_variance_min = 1.0 / (1.0 / prior + maximum_information);
    let posterior_variance_max = 1.0 / (1.0 / prior + minimum_information);
    let eig_min = 0.5 * (1.0 + prior * minimum_information).ln();
    let eig_max = 0.5 * (1.0 + prior * maximum_information).ln();
    if !posterior_variance_min.is_finite()
        || !posterior_variance_max.is_finite()
        || !eig_min.is_finite()
        || !eig_max.is_finite()
    {
        return Err(ModelError::InvalidParameter(
            "dependence_envelope_utility_nonfinite",
        ));
    }

    Ok(TwoSourceDependenceEnvelope {
        correlation_bounds: problem.correlation_bounds,
        parameter_information_bounds: [minimum_information, maximum_information],
        posterior_parameter_variance_bounds: [posterior_variance_min, posterior_variance_max],
        expected_information_gain_nats_bounds: [eig_min, eig_max],
        worst_case_information_correlation,
        best_case_information_correlation,
        extremum_candidate_correlations: candidate_correlations,
    })
}

fn validate_problem(problem: &TwoSourceDependenceProblem) -> Result<(), ModelError> {
    if !problem.prior_parameter_variance.is_finite()
        || problem.observation_sensitivities.iter().any(|value| !value.is_finite())
        || problem
            .independent_observation_noise_variances
            .iter()
            .any(|value| !value.is_finite())
        || problem
            .source_standard_deviations
            .iter()
            .any(|value| !value.is_finite())
        || problem
            .source_jacobians
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
        || problem.correlation_bounds.iter().any(|value| !value.is_finite())
    {
        return Err(ModelError::NonFiniteValue("dependence_envelope_input"));
    }
    if problem.prior_parameter_variance <= 0.0
        || problem
            .independent_observation_noise_variances
            .iter()
            .any(|value| *value <= 0.0)
        || problem
            .source_standard_deviations
            .iter()
            .any(|value| *value < 0.0)
    {
        return Err(ModelError::InvalidParameter(
            "dependence_envelope_variances_and_noise_invalid",
        ));
    }
    let [rho_min, rho_max] = problem.correlation_bounds;
    if rho_min < -1.0 || rho_max > 1.0 || rho_min > rho_max {
        return Err(ModelError::InvalidParameter(
            "dependence_envelope_correlation_bounds_invalid",
        ));
    }
    Ok(())
}

fn stationary_points_of_information(
    n0: f64,
    n1: f64,
    d0: f64,
    d1: f64,
    d2: f64,
    lower: f64,
    upper: f64,
) -> Vec<f64> {
    // Derivative numerator:
    // (n1*d0 - n0*d1) - 2*n0*d2*rho - n1*d2*rho^2.
    let a = -n1 * d2;
    let b = -2.0 * n0 * d2;
    let c = n1 * d0 - n0 * d1;
    let coefficient_scale = a.abs().max(b.abs()).max(c.abs());
    if coefficient_scale == 0.0 || !coefficient_scale.is_finite() {
        return Vec::new();
    }
    let tolerance = 1.0e-12 * coefficient_scale;

    let mut roots = Vec::with_capacity(2);
    if a.abs() <= tolerance {
        if b.abs() > tolerance {
            roots.push(-c / b);
        }
    } else {
        let four_ac = 4.0 * a * c;
        let discriminant = b * b - four_ac;
        let discriminant_tolerance =
            1.0e-12 * (b * b).abs().max(four_ac.abs()).max(f64::MIN_POSITIVE);
        if discriminant >= -discriminant_tolerance {
            let square_root = discriminant.max(0.0).sqrt();
            let q = -0.5 * (b + if b >= 0.0 { square_root } else { -square_root });
            if q.abs() <= tolerance {
                roots.push(-b / (2.0 * a));
            } else {
                roots.push(q / a);
                roots.push(c / q);
            }
        }
    }

    roots
        .into_iter()
        .filter(|root| root.is_finite() && *root >= lower && *root <= upper)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interior_extremum_problem() -> TwoSourceDependenceProblem {
        TwoSourceDependenceProblem {
            prior_parameter_variance: 1.0,
            observation_sensitivities: [2.0, 1.0],
            independent_observation_noise_variances: [1.0, 1.0],
            source_standard_deviations: [1.0, 1.0],
            source_jacobians: [[1.0, 1.0], [1.0, -1.0]],
            correlation_bounds: [-0.75, 0.75],
        }
    }

    #[test]
    fn exact_envelope_finds_an_interior_dependence_extremum() {
        let result = bound_two_source_dependence(&interior_extremum_problem())
            .expect("bounded unknown dependence");
        assert_eq!(result.extremum_candidate_correlations.len(), 3);
        assert!((result.worst_case_information_correlation - 0.5).abs() < 1.0e-12);
        assert!((result.best_case_information_correlation + 0.75).abs() < 1.0e-12);
        assert!((result.parameter_information_bounds[0] - 1.5).abs() < 1.0e-12);
        assert!((result.parameter_information_bounds[1] - 26.0 / 9.0).abs() < 1.0e-12);
        assert!((result.posterior_parameter_variance_bounds[0] - 9.0 / 35.0).abs() < 1.0e-12);
        assert!((result.posterior_parameter_variance_bounds[1] - 0.4).abs() < 1.0e-12);
        assert!(
            (result.expected_information_gain_nats_bounds[0] - 0.5 * 2.5_f64.ln()).abs()
                < 1.0e-12
        );
        assert!(
            (result.expected_information_gain_nats_bounds[1] - 0.5 * (35.0_f64 / 9.0).ln()).abs()
                < 1.0e-12
        );
    }

    #[test]
    fn exact_known_correlation_collapses_the_envelope() {
        let mut problem = interior_extremum_problem();
        problem.correlation_bounds = [0.25, 0.25];
        let result = bound_two_source_dependence(&problem).expect("fixed cross-source correlation");
        assert_eq!(result.extremum_candidate_correlations, vec![0.25]);
        assert!(
            (result.expected_information_gain_nats_bounds[0]
                - result.expected_information_gain_nats_bounds[1])
                .abs()
                < 1.0e-12
        );
        assert!(
            (result.posterior_parameter_variance_bounds[0]
                - result.posterior_parameter_variance_bounds[1])
                .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn full_unknown_dependence_is_bounded_by_psd_correlation_interval() {
        let mut problem = interior_extremum_problem();
        problem.correlation_bounds = [-1.0, 1.0];
        let result = bound_two_source_dependence(&problem)
            .expect("full PSD-compatible correlation interval");
        assert_eq!(
            result.extremum_candidate_correlations.first().copied(),
            Some(-1.0)
        );
        assert_eq!(
            result.extremum_candidate_correlations.last().copied(),
            Some(1.0)
        );
        assert!(
            result.expected_information_gain_nats_bounds[0]
                <= result.expected_information_gain_nats_bounds[1]
        );
    }

    #[test]
    fn exact_envelope_contains_independently_assembled_dense_grid() {
        let problem = interior_extremum_problem();
        let envelope =
            bound_two_source_dependence(&problem).expect("bounded dependence envelope");
        let [sigma1, sigma2] = problem.source_standard_deviations;
        let [[j11, j12], [j21, j22]] = problem.source_jacobians;
        let [h1, h2] = problem.observation_sensitivities;
        let [rho_min, rho_max] = problem.correlation_bounds;

        for step in 0..=10_000 {
            let rho = rho_min + (rho_max - rho_min) * step as f64 / 10_000.0;
            let source_covariance = [
                [sigma1 * sigma1, rho * sigma1 * sigma2],
                [rho * sigma1 * sigma2, sigma2 * sigma2],
            ];
            let jacobians = [[j11, j12], [j21, j22]];
            let mut covariance = [[0.0; 2]; 2];
            for row in 0..2 {
                for column in 0..2 {
                    covariance[row][column] = if row == column {
                        problem.independent_observation_noise_variances[row]
                    } else {
                        0.0
                    };
                    for source_row in 0..2 {
                        for source_column in 0..2 {
                            covariance[row][column] += jacobians[row][source_row]
                                * source_covariance[source_row][source_column]
                                * jacobians[column][source_column];
                        }
                    }
                }
            }
            let determinant =
                covariance[0][0] * covariance[1][1] - covariance[0][1] * covariance[1][0];
            let numerator = h1 * h1 * covariance[1][1]
                - h1 * h2 * covariance[0][1]
                - h2 * h1 * covariance[1][0]
                + h2 * h2 * covariance[0][0];
            let information = numerator / determinant;
            assert!(
                information >= envelope.parameter_information_bounds[0] - 1.0e-10,
                "information {information} below envelope at rho={rho}"
            );
            assert!(
                information <= envelope.parameter_information_bounds[1] + 1.0e-10,
                "information {information} above envelope at rho={rho}"
            );
        }
    }

    #[test]
    fn malformed_dependence_intervals_and_nonpositive_noise_fail_closed() {
        let mut problem = interior_extremum_problem();
        problem.correlation_bounds = [-1.1, 0.5];
        assert_eq!(
            bound_two_source_dependence(&problem),
            Err(ModelError::InvalidParameter(
                "dependence_envelope_correlation_bounds_invalid"
            ))
        );
        let mut problem = interior_extremum_problem();
        problem.independent_observation_noise_variances[0] = 0.0;
        assert_eq!(
            bound_two_source_dependence(&problem),
            Err(ModelError::InvalidParameter(
                "dependence_envelope_variances_and_noise_invalid"
            ))
        );
    }

    #[test]
    fn zero_source_variance_reduces_to_known_independent_noise() {
        let mut problem = interior_extremum_problem();
        problem.source_standard_deviations = [0.0, 0.0];
        problem.correlation_bounds = [-1.0, 1.0];
        let result = bound_two_source_dependence(&problem).expect("zero source uncertainty");
        assert_eq!(result.parameter_information_bounds[0], result.parameter_information_bounds[1]);
        assert_eq!(result.posterior_parameter_variance_bounds[0], result.posterior_parameter_variance_bounds[1]);
        assert_eq!(result.worst_case_information_correlation, result.best_case_information_correlation);
    }
}
