//! Propagation of response-matrix ensemble uncertainty into count moments.
//!
//! Each ensemble member is an equally weighted calibrated response realization.
//! Conditional measured-bin counts are independent Poisson variables.
//! Between-realization covariance captures shared calibration effects that can
//! correlate the measured bins.

use crate::{DetectorResponseMatrix, ModelError};

#[derive(Debug, Clone, PartialEq)]
pub struct DetectorResponseEnsemble {
    matrices: Vec<DetectorResponseMatrix>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PredictiveCountMoments {
    pub mean_counts: Vec<f64>,
    /// E[Cov(Y|R)] + Cov(E[Y|R]); Poisson plus response-calibration covariance.
    pub covariance_counts: Vec<Vec<f64>>,
}

impl DetectorResponseEnsemble {
    pub fn new(matrices: Vec<DetectorResponseMatrix>) -> Result<Self, ModelError> {
        if matrices.len() < 2 {
            return Err(ModelError::InvalidParameter(
                "detector_response_ensemble_requires_at_least_two_matrices",
            ));
        }
        let measured_bins = matrices[0].measured_bins();
        let true_energy_bins = matrices[0].true_energy_bins();
        if matrices.iter().any(|matrix| {
            matrix.measured_bins() != measured_bins || matrix.true_energy_bins() != true_energy_bins
        }) {
            return Err(ModelError::MismatchedDataLengths);
        }
        Ok(Self { matrices })
    }

    pub fn realization_count(&self) -> usize {
        self.matrices.len()
    }

    /// Background is held fixed; represent material background uncertainty in a
    /// separate ensemble dimension. Matrices must come from a declared
    /// calibration distribution or bootstrap, not post-hoc selection.
    pub fn predictive_count_moments(
        &self,
        true_spectrum_rate_s: &[f64],
        background_rate_counts_s: &[f64],
        exposure_time_s: f64,
    ) -> Result<PredictiveCountMoments, ModelError> {
        let mut conditional_means = Vec::with_capacity(self.matrices.len());
        for matrix in &self.matrices {
            conditional_means.push(matrix.expected_counts(
                true_spectrum_rate_s,
                background_rate_counts_s,
                exposure_time_s,
            )?);
        }
        let measured_bins = conditional_means[0].len();
        let realization_count = conditional_means.len();
        let mut mean_counts = vec![0.0; measured_bins];
        for counts in &conditional_means {
            for bin in 0..measured_bins {
                mean_counts[bin] += counts[bin] / realization_count as f64;
            }
        }

        let mut covariance_counts = vec![vec![0.0; measured_bins]; measured_bins];
        let denominator = (realization_count - 1) as f64;
        for left in 0..measured_bins {
            for right in 0..measured_bins {
                let mut covariance = 0.0;
                for counts in &conditional_means {
                    covariance +=
                        (counts[left] - mean_counts[left]) * (counts[right] - mean_counts[right]);
                }
                covariance_counts[left][right] = covariance / denominator;
            }
            covariance_counts[left][left] += mean_counts[left];
        }
        Ok(PredictiveCountMoments {
            mean_counts,
            covariance_counts,
        })
    }

    /// Log predictive probability of measured integer counts after marginalizing
    /// over the equally weighted finite response-matrix ensemble.
    ///
    /// This is a mixture of conditional independent-Poisson likelihoods in log
    /// space. A response realization assigning zero mean to a positive observed
    /// count has zero likelihood for that realization but does not invalidate
    /// other supported realizations. If every realization assigns zero support,
    /// the method fails closed. Background is treated as fixed.
    pub fn log_predictive_probability(
        &self,
        observed_counts: &[u64],
        true_spectrum_rate_s: &[f64],
        background_rate_counts_s: &[f64],
        exposure_time_s: f64,
    ) -> Result<f64, ModelError> {
        if observed_counts.len() != self.matrices[0].measured_bins() {
            return Err(ModelError::MismatchedDataLengths);
        }
        let mut log_likelihoods = Vec::with_capacity(self.matrices.len());
        for matrix in &self.matrices {
            let expected = matrix.expected_counts(
                true_spectrum_rate_s,
                background_rate_counts_s,
                exposure_time_s,
            )?;
            if expected
                .iter()
                .any(|value| !value.is_finite() || *value < 0.0)
            {
                return Err(ModelError::InvalidParameter(
                    "response_ensemble_expected_count_is_invalid",
                ));
            }
            log_likelihoods.push(poisson_vector_log_probability(observed_counts, &expected)?);
        }

        let maximum = log_likelihoods
            .iter()
            .copied()
            .filter(|value| value.is_finite())
            .fold(f64::NEG_INFINITY, f64::max);
        if !maximum.is_finite() {
            return Err(ModelError::InvalidParameter(
                "observations_have_zero_support_under_response_ensemble",
            ));
        }
        let scaled_sum: f64 = log_likelihoods
            .iter()
            .filter(|value| value.is_finite())
            .map(|value| (*value - maximum).exp())
            .sum();
        let log_probability = maximum + scaled_sum.ln() - (self.matrices.len() as f64).ln();
        if !log_probability.is_finite() {
            return Err(ModelError::InvalidParameter(
                "response_ensemble_log_probability_is_nonfinite",
            ));
        }
        Ok(log_probability)
    }
}

fn poisson_vector_log_probability(
    observed_counts: &[u64],
    expected_counts: &[f64],
) -> Result<f64, ModelError> {
    if observed_counts.is_empty() || observed_counts.len() != expected_counts.len() {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut log_probability = 0.0;
    for (observed, expected) in observed_counts.iter().zip(expected_counts) {
        if !expected.is_finite() || *expected < 0.0 {
            return Err(ModelError::InvalidParameter(
                "poisson_expected_count_must_be_finite_and_nonnegative",
            ));
        }
        if *expected == 0.0 {
            if *observed == 0 {
                continue;
            }
            return Ok(f64::NEG_INFINITY);
        }
        log_probability += (*observed as f64) * expected.ln() - expected - log_factorial(*observed);
    }
    if !log_probability.is_finite() {
        return Err(ModelError::InvalidParameter(
            "poisson_vector_log_probability_is_nonfinite",
        ));
    }
    Ok(log_probability)
}

fn log_factorial(value: u64) -> f64 {
    if value < 2 {
        return 0.0;
    }
    if value < 64 {
        return (2..=value).map(|term| (term as f64).ln()).sum();
    }
    // Stirling expansion with Bernoulli corrections; expressed in inverse
    // powers to avoid overflow for the full u64 domain.
    let n = value as f64;
    let inverse = 1.0 / n;
    let correction = inverse / 12.0 - inverse.powi(3) / 360.0 + inverse.powi(5) / 1260.0
        - inverse.powi(7) / 1680.0
        + inverse.powi(9) / 1188.0;
    (n + 0.5) * n.ln() - n + 0.5 * (2.0 * std::f64::consts::PI).ln() + correction
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ensemble() -> DetectorResponseEnsemble {
        let first =
            DetectorResponseMatrix::new(vec![vec![0.8, 0.1], vec![0.2, 0.7]]).expect("first");
        let second =
            DetectorResponseMatrix::new(vec![vec![0.7, 0.2], vec![0.3, 0.6]]).expect("second");
        DetectorResponseEnsemble::new(vec![first, second]).expect("ensemble")
    }

    #[test]
    fn response_ensemble_adds_calibration_covariance_to_poisson_noise() {
        let moments = ensemble()
            .predictive_count_moments(&[100.0, 50.0], &[1.0, 2.0], 10.0)
            .expect("moments");
        assert_eq!(moments.mean_counts, vec![835.0, 595.0]);
        assert_eq!(
            moments.covariance_counts,
            vec![vec![2085.0, -1250.0], vec![-1250.0, 1845.0]]
        );
    }

    #[test]
    fn identical_matrices_leave_only_poisson_counting_variance() {
        let matrix =
            DetectorResponseMatrix::new(vec![vec![0.5, 0.0], vec![0.0, 0.4]]).expect("matrix");
        let ensemble =
            DetectorResponseEnsemble::new(vec![matrix.clone(), matrix]).expect("ensemble");
        let moments = ensemble
            .predictive_count_moments(&[10.0, 20.0], &[0.0, 0.0], 5.0)
            .expect("moments");
        assert_eq!(moments.mean_counts, vec![25.0, 40.0]);
        assert_eq!(
            moments.covariance_counts,
            vec![vec![25.0, 0.0], vec![0.0, 40.0]]
        );
    }

    #[test]
    fn response_uncertainty_can_create_cross_bin_covariance() {
        let moments = ensemble()
            .predictive_count_moments(&[100.0, 50.0], &[1.0, 2.0], 10.0)
            .expect("moments");
        assert_eq!(moments.covariance_counts[0][1], -1250.0);
        assert_eq!(
            moments.covariance_counts[0][1],
            moments.covariance_counts[1][0]
        );
    }

    #[test]
    fn ensemble_rejects_incompatible_dimensions() {
        let first = DetectorResponseMatrix::new(vec![vec![1.0, 0.0]]).expect("first");
        let second = DetectorResponseMatrix::new(vec![vec![1.0]]).expect("second");
        assert_eq!(
            DetectorResponseEnsemble::new(vec![first, second]),
            Err(ModelError::MismatchedDataLengths)
        );
    }

    #[test]
    fn ensemble_requires_multiple_realizations() {
        let matrix = DetectorResponseMatrix::new(vec![vec![1.0]]).expect("matrix");
        assert_eq!(
            DetectorResponseEnsemble::new(vec![matrix]),
            Err(ModelError::InvalidParameter(
                "detector_response_ensemble_requires_at_least_two_matrices"
            ))
        );
    }

    #[test]
    fn invalid_spectrum_is_rejected_by_forward_operator() {
        assert_eq!(
            ensemble().predictive_count_moments(&[-1.0, 2.0], &[0.0, 0.0], 1.0),
            Err(ModelError::InvalidParameter(
                "true_spectrum_rate_must_be_nonnegative"
            ))
        );
    }

    #[test]
    fn duplicated_response_ensemble_matches_single_poisson_likelihood() {
        let matrix =
            DetectorResponseMatrix::new(vec![vec![1.0, 0.0], vec![0.0, 1.0]]).expect("identity");
        let ensemble =
            DetectorResponseEnsemble::new(vec![matrix.clone(), matrix]).expect("ensemble");
        let actual = ensemble
            .log_predictive_probability(&[10, 20], &[10.0, 20.0], &[0.0, 0.0], 1.0)
            .expect("log predictive probability");
        let expected = 10.0_f64 * 10.0_f64.ln() - 10.0 - log_factorial(10)
            + 20.0_f64 * 20.0_f64.ln()
            - 20.0
            - log_factorial(20);
        assert!((actual - expected).abs() < 1.0e-12);
    }

    #[test]
    fn response_mixture_uses_log_mean_exp_of_conditional_likelihoods() {
        let ensemble = ensemble();
        let observed = [840, 600];
        let true_rate = [100.0, 50.0];
        let background = [1.0, 2.0];
        let exposure = 10.0;
        let actual = ensemble
            .log_predictive_probability(&observed, &true_rate, &background, exposure)
            .expect("mixture log probability");
        let first =
            poisson_vector_log_probability(&observed, &[860.0, 570.0]).expect("first likelihood");
        let second =
            poisson_vector_log_probability(&observed, &[810.0, 620.0]).expect("second likelihood");
        let maximum = first.max(second);
        let expected =
            maximum + ((first - maximum).exp() + (second - maximum).exp()).ln() - 2.0_f64.ln();
        assert!((actual - expected).abs() < 1.0e-12);
    }

    #[test]
    fn zero_support_for_one_response_draw_does_not_discard_other_draws() {
        let supported = DetectorResponseMatrix::new(vec![vec![1.0]]).expect("supported");
        let unsupported = DetectorResponseMatrix::new(vec![vec![0.0]]).expect("zero efficiency");
        let ensemble =
            DetectorResponseEnsemble::new(vec![supported, unsupported]).expect("ensemble");
        let actual = ensemble
            .log_predictive_probability(&[5], &[10.0], &[0.0], 1.0)
            .expect("one draw supports observation");
        let single = poisson_vector_log_probability(&[5], &[10.0]).expect("single likelihood");
        assert!((actual - (single - 2.0_f64.ln())).abs() < 1.0e-12);
    }

    #[test]
    fn response_ensemble_fails_when_all_draws_assign_zero_support() {
        let first = DetectorResponseMatrix::new(vec![vec![0.0]]).expect("first");
        let second = DetectorResponseMatrix::new(vec![vec![0.0]]).expect("second");
        let ensemble = DetectorResponseEnsemble::new(vec![first, second]).expect("ensemble");
        assert_eq!(
            ensemble.log_predictive_probability(&[1], &[10.0], &[0.0], 1.0),
            Err(ModelError::InvalidParameter(
                "observations_have_zero_support_under_response_ensemble"
            ))
        );
    }

    #[test]
    fn response_ensemble_log_probability_rejects_count_dimension_mismatch() {
        let ensemble = ensemble();
        assert_eq!(
            ensemble.log_predictive_probability(&[1], &[100.0, 50.0], &[0.0, 0.0], 1.0),
            Err(ModelError::MismatchedDataLengths)
        );
    }
}
