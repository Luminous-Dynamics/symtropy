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
            return Err(ModelError::InvalidParameter("detector_response_ensemble_requires_at_least_two_matrices"));
        }
        let measured_bins = matrices[0].measured_bins();
        let true_energy_bins = matrices[0].true_energy_bins();
        if matrices.iter().any(|matrix| matrix.measured_bins() != measured_bins || matrix.true_energy_bins() != true_energy_bins) {
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
            conditional_means.push(matrix.expected_counts(true_spectrum_rate_s, background_rate_counts_s, exposure_time_s)?);
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
                    covariance += (counts[left] - mean_counts[left]) * (counts[right] - mean_counts[right]);
                }
                covariance_counts[left][right] = covariance / denominator;
            }
            covariance_counts[left][left] += mean_counts[left];
        }
        Ok(PredictiveCountMoments { mean_counts, covariance_counts })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ensemble() -> DetectorResponseEnsemble {
        let first = DetectorResponseMatrix::new(vec![vec![0.8, 0.1], vec![0.2, 0.7]]).expect("first");
        let second = DetectorResponseMatrix::new(vec![vec![0.7, 0.2], vec![0.3, 0.6]]).expect("second");
        DetectorResponseEnsemble::new(vec![first, second]).expect("ensemble")
    }

    #[test]
    fn response_ensemble_adds_calibration_covariance_to_poisson_noise() {
        let moments = ensemble().predictive_count_moments(&[100.0, 50.0], &[1.0, 2.0], 10.0).expect("moments");
        assert_eq!(moments.mean_counts, vec![835.0, 595.0]);
        assert_eq!(moments.covariance_counts, vec![vec![2085.0, -1250.0], vec![-1250.0, 1845.0]]);
    }

    #[test]
    fn identical_matrices_leave_only_poisson_counting_variance() {
        let matrix = DetectorResponseMatrix::new(vec![vec![0.5, 0.0], vec![0.0, 0.4]]).expect("matrix");
        let ensemble = DetectorResponseEnsemble::new(vec![matrix.clone(), matrix]).expect("ensemble");
        let moments = ensemble.predictive_count_moments(&[10.0, 20.0], &[0.0, 0.0], 5.0).expect("moments");
        assert_eq!(moments.mean_counts, vec![25.0, 40.0]);
        assert_eq!(moments.covariance_counts, vec![vec![25.0, 0.0], vec![0.0, 40.0]]);
    }

    #[test]
    fn response_uncertainty_can_create_cross_bin_covariance() {
        let moments = ensemble().predictive_count_moments(&[100.0, 50.0], &[1.0, 2.0], 10.0).expect("moments");
        assert_eq!(moments.covariance_counts[0][1], -1250.0);
        assert_eq!(moments.covariance_counts[0][1], moments.covariance_counts[1][0]);
    }

    #[test]
    fn ensemble_rejects_incompatible_dimensions() {
        let first = DetectorResponseMatrix::new(vec![vec![1.0, 0.0]]).expect("first");
        let second = DetectorResponseMatrix::new(vec![vec![1.0]]).expect("second");
        assert_eq!(DetectorResponseEnsemble::new(vec![first, second]), Err(ModelError::MismatchedDataLengths));
    }

    #[test]
    fn ensemble_requires_multiple_realizations() {
        let matrix = DetectorResponseMatrix::new(vec![vec![1.0]]).expect("matrix");
        assert_eq!(DetectorResponseEnsemble::new(vec![matrix]), Err(ModelError::InvalidParameter("detector_response_ensemble_requires_at_least_two_matrices")));
    }

    #[test]
    fn invalid_spectrum_is_rejected_by_forward_operator() {
        assert_eq!(ensemble().predictive_count_moments(&[-1.0, 2.0], &[0.0, 0.0], 1.0), Err(ModelError::InvalidParameter("true_spectrum_rate_must_be_nonnegative")));
    }
}
