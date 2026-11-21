//! Research-only linear detector response-matrix forward model.
//!
//! Matrix convention:
//! - rows: measured pulse-height / light-output bins;
//! - columns: true incident neutron-energy bins;
//! - each element: probability that a neutron in a true-energy bin produces
//!   an accepted event in a measured bin, including whatever geometric and
//!   detector efficiencies are explicitly included in this matrix.
//!
//! Column sums must be <= 1. Background and exposure time are separate.
//! This module does not simulate scintillation or waveforms, construct Geant4
//! matrices, perform PSD, or invert the response matrix.

use crate::ModelError;

/// Linear detector response matrix from true energy bins to measured bins.
#[derive(Debug, Clone, PartialEq)]
pub struct DetectorResponseMatrix {
    measured_bins: usize,
    true_energy_bins: usize,
    probabilities: Vec<Vec<f64>>,
    column_efficiencies: Vec<f64>,
}

impl DetectorResponseMatrix {
    /// Construct a response matrix.
    ///
    /// The matrix is row-major: probabilities[measured_bin][true_energy_bin].
    /// Each true-energy column must sum to a detection probability in [0, 1].
    pub fn new(probabilities: Vec<Vec<f64>>) -> Result<Self, ModelError> {
        if probabilities.is_empty() || probabilities[0].is_empty() {
            return Err(ModelError::InvalidParameter(
                "detector_response_matrix_must_be_nonempty",
            ));
        }
        let measured_bins = probabilities.len();
        let true_energy_bins = probabilities[0].len();

        if probabilities
            .iter()
            .any(|row| row.len() != true_energy_bins)
        {
            return Err(ModelError::MismatchedDataLengths);
        }

        for row in &probabilities {
            for value in row {
                if !value.is_finite() {
                    return Err(ModelError::NonFiniteValue("detector_response_probability"));
                }
                if *value < 0.0 || *value > 1.0 {
                    return Err(ModelError::InvalidParameter(
                        "detector_response_probability_must_be_in_0_1",
                    ));
                }
            }
        }

        let mut column_efficiencies = vec![0.0; true_energy_bins];
        for (true_bin, efficiency) in column_efficiencies.iter_mut().enumerate() {
            *efficiency = probabilities.iter().map(|row| row[true_bin]).sum::<f64>();
            if !efficiency.is_finite() || *efficiency > 1.0 + 1.0e-10 {
                return Err(ModelError::InvalidParameter(
                    "detector_response_column_efficiency_exceeds_one",
                ));
            }
        }

        Ok(Self {
            measured_bins,
            true_energy_bins,
            probabilities,
            column_efficiencies,
        })
    }

    pub fn measured_bins(&self) -> usize {
        self.measured_bins
    }

    pub fn true_energy_bins(&self) -> usize {
        self.true_energy_bins
    }

    /// Detection probability / efficiency represented by each true-energy
    /// column. Factors not included in the response matrix must be handled
    /// separately and must not be silently double-counted.
    pub fn column_efficiencies(&self) -> &[f64] {
        &self.column_efficiencies
    }

    /// Forward-project an incident neutron rate spectrum (neutrons/s/bin)
    /// into accepted measured-bin rates (counts/s/bin).
    pub fn apply(&self, true_spectrum_rate_s: &[f64]) -> Result<Vec<f64>, ModelError> {
        if true_spectrum_rate_s.len() != self.true_energy_bins {
            return Err(ModelError::MismatchedDataLengths);
        }
        for value in true_spectrum_rate_s {
            if !value.is_finite() {
                return Err(ModelError::NonFiniteValue("true_spectrum_rate_per_s"));
            }
            if *value < 0.0 {
                return Err(ModelError::InvalidParameter(
                    "true_spectrum_rate_must_be_nonnegative",
                ));
            }
        }

        let mut measured_rate = vec![0.0; self.measured_bins];
        for (measured_bin, row) in self.probabilities.iter().enumerate() {
            measured_rate[measured_bin] = row
                .iter()
                .zip(true_spectrum_rate_s.iter())
                .map(|(probability, true_rate)| probability * true_rate)
                .sum::<f64>();
            if !measured_rate[measured_bin].is_finite() {
                return Err(ModelError::InvalidParameter(
                    "detector_response_produced_nonfinite_measured_rate",
                ));
            }
        }
        Ok(measured_rate)
    }

    /// Nonnegative Poisson expectation-maximization (Richardson-Lucy-style)
    /// unfolding baseline.
    ///
    /// This is an independent comparator, **not GRAVEL**. It maximizes the
    /// unregularized Poisson likelihood for the supplied response matrix and
    /// fixed background expectation. Finite iteration count is an explicit
    /// early-stopping regularization choice; noisy/ill-conditioned problems
    /// can overfit as iterations increase.
    ///
    /// observed_counts and background_expected_counts are measured-bin counts.
    /// initial_true_counts is a strictly positive starting spectrum in incident
    /// neutron counts per true-energy bin. Every true-energy column must have
    /// nonzero detection efficiency; otherwise that bin is unobservable and
    /// the method fails closed rather than manufacturing an estimate.
    pub fn poisson_em_unfold(
        &self,
        observed_counts: &[f64],
        background_expected_counts: &[f64],
        initial_true_counts: &[f64],
        iterations: usize,
    ) -> Result<Vec<f64>, ModelError> {
        if observed_counts.len() != self.measured_bins
            || background_expected_counts.len() != self.measured_bins
            || initial_true_counts.len() != self.true_energy_bins
        {
            return Err(ModelError::MismatchedDataLengths);
        }
        if iterations == 0 {
            return Err(ModelError::InvalidParameter(
                "poisson_em_iterations_must_be_positive",
            ));
        }
        for (values, name) in [
            (observed_counts, "observed_pulse_height_counts"),
            (background_expected_counts, "background_expected_counts"),
            (initial_true_counts, "initial_true_spectrum_counts"),
        ] {
            for value in values {
                if !value.is_finite() {
                    return Err(ModelError::NonFiniteValue(name));
                }
                if *value < 0.0 {
                    return Err(ModelError::InvalidParameter(
                        "poisson_em_inputs_must_be_nonnegative",
                    ));
                }
            }
        }
        if initial_true_counts.contains(&0.0) {
            return Err(ModelError::InvalidParameter(
                "poisson_em_initial_true_spectrum_must_be_strictly_positive",
            ));
        }
        if self.column_efficiencies.iter().any(|value| *value <= 0.0) {
            return Err(ModelError::InvalidParameter(
                "poisson_em_true_energy_bin_is_unobservable",
            ));
        }

        let mut estimate = initial_true_counts.to_vec();
        for _ in 0..iterations {
            let mut predicted = vec![0.0; self.measured_bins];
            for (measured_bin, row) in self.probabilities.iter().enumerate() {
                predicted[measured_bin] = background_expected_counts[measured_bin]
                    + row
                        .iter()
                        .zip(estimate.iter())
                        .map(|(probability, count)| probability * count)
                        .sum::<f64>();
            }

            let mut ratio = vec![0.0; self.measured_bins];
            for measured_bin in 0..self.measured_bins {
                let expected = predicted[measured_bin];
                let observed = observed_counts[measured_bin];
                if !expected.is_finite() {
                    return Err(ModelError::InvalidParameter(
                        "poisson_em_expected_counts_nonfinite",
                    ));
                }
                if expected <= 0.0 {
                    if observed > 0.0 {
                        return Err(ModelError::InvalidParameter(
                            "poisson_em_observation_has_zero_predicted_support",
                        ));
                    }
                    ratio[measured_bin] = 0.0;
                } else {
                    ratio[measured_bin] = observed / expected;
                }
            }

            let mut next = vec![0.0; self.true_energy_bins];
            for true_bin in 0..self.true_energy_bins {
                let backprojected = self
                    .probabilities
                    .iter()
                    .zip(ratio.iter())
                    .map(|(row, measured_ratio)| row[true_bin] * measured_ratio)
                    .sum::<f64>();
                next[true_bin] =
                    estimate[true_bin] * backprojected / self.column_efficiencies[true_bin];
                if !next[true_bin].is_finite() || next[true_bin] < 0.0 {
                    return Err(ModelError::InvalidParameter(
                        "poisson_em_update_produced_invalid_spectrum",
                    ));
                }
            }
            estimate = next;
        }
        Ok(estimate)
    }

    /// Expected measured-bin counts for a finite acquisition.
    ///
    /// Background is supplied as a separate measured-bin rate in counts/s.
    /// The result is an expectation, not a Poisson draw; use a count-space
    /// likelihood for measured integer observations.
    pub fn expected_counts(
        &self,
        true_spectrum_rate_s: &[f64],
        background_rate_counts_s: &[f64],
        exposure_time_s: f64,
    ) -> Result<Vec<f64>, ModelError> {
        if background_rate_counts_s.len() != self.measured_bins {
            return Err(ModelError::MismatchedDataLengths);
        }
        if !exposure_time_s.is_finite() {
            return Err(ModelError::NonFiniteValue("detector_exposure_time_s"));
        }
        if exposure_time_s <= 0.0 {
            return Err(ModelError::NonPositiveValue("detector_exposure_time_s"));
        }
        for value in background_rate_counts_s {
            if !value.is_finite() {
                return Err(ModelError::NonFiniteValue("background_rate_counts_s"));
            }
            if *value < 0.0 {
                return Err(ModelError::InvalidParameter(
                    "background_rate_must_be_nonnegative",
                ));
            }
        }

        let signal_rate = self.apply(true_spectrum_rate_s)?;
        Ok(signal_rate
            .iter()
            .zip(background_rate_counts_s.iter())
            .map(|(signal, background)| (signal + background) * exposure_time_s)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poisson_em_unfold_recovers_identity_response_counts() {
        let response =
            DetectorResponseMatrix::new(vec![vec![1.0, 0.0], vec![0.0, 1.0]]).expect("identity");
        let result = response
            .poisson_em_unfold(&[10.0, 20.0], &[0.0, 0.0], &[1.0, 1.0], 1)
            .expect("identity unfolding");
        assert_eq!(result, vec![10.0, 20.0]);
    }

    #[test]
    fn poisson_em_unfold_recovers_a_migrated_synthetic_spectrum() {
        let response =
            DetectorResponseMatrix::new(vec![vec![0.8, 0.2], vec![0.2, 0.8]]).expect("response");
        let result = response
            .poisson_em_unfold(&[90.0, 60.0], &[0.0, 0.0], &[75.0, 75.0], 50)
            .expect("Poisson EM");
        assert!((result[0] - 100.0).abs() < 1.0e-5);
        assert!((result[1] - 50.0).abs() < 1.0e-5);
    }

    #[test]
    fn poisson_em_unfold_accounts_for_known_background() {
        let response =
            DetectorResponseMatrix::new(vec![vec![1.0, 0.0], vec![0.0, 1.0]]).expect("identity");
        let result = response
            .poisson_em_unfold(&[10.0, 20.0], &[2.0, 4.0], &[1.0, 1.0], 100)
            .expect("background-aware EM");
        assert!((result[0] - 8.0).abs() < 1.0e-5);
        assert!((result[1] - 16.0).abs() < 1.0e-5);
    }

    #[test]
    fn poisson_em_unfold_rejects_unobservable_true_energy_bins() {
        let response = DetectorResponseMatrix::new(vec![vec![1.0, 0.0], vec![0.0, 0.0]])
            .expect("matrix with unobservable bin");
        assert_eq!(
            response.poisson_em_unfold(&[5.0, 0.0], &[0.0, 0.0], &[1.0, 1.0], 5),
            Err(ModelError::InvalidParameter(
                "poisson_em_true_energy_bin_is_unobservable"
            ))
        );
    }

    #[test]
    fn poisson_em_unfold_rejects_zero_iterations_and_bad_initial_spectrum() {
        let response =
            DetectorResponseMatrix::new(vec![vec![1.0, 0.0], vec![0.0, 1.0]]).expect("identity");
        assert!(response
            .poisson_em_unfold(&[1.0, 1.0], &[0.0, 0.0], &[1.0, 1.0], 0)
            .is_err());
        assert!(response
            .poisson_em_unfold(&[1.0, 1.0], &[0.0, 0.0], &[0.0, 1.0], 5)
            .is_err());
    }

    #[test]
    fn identity_response_recovers_true_spectrum() {
        let response = DetectorResponseMatrix::new(vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ])
        .expect("identity response");
        let measured = response
            .apply(&[10.0, 20.0, 30.0])
            .expect("forward response");
        assert_eq!(measured, vec![10.0, 20.0, 30.0]);
        assert_eq!(response.column_efficiencies(), &[1.0, 1.0, 1.0]);
    }

    #[test]
    fn response_matrix_models_energy_migration_and_efficiency_loss() {
        let response = DetectorResponseMatrix::new(vec![vec![0.5, 0.1], vec![0.3, 0.6]])
            .expect("valid response");
        let measured = response.apply(&[100.0, 50.0]).expect("apply");
        assert_eq!(measured, vec![55.0, 60.0]);
        assert_eq!(response.column_efficiencies(), &[0.8, 0.7]);
    }

    #[test]
    fn expected_counts_add_background_after_response() {
        let response =
            DetectorResponseMatrix::new(vec![vec![0.4, 0.0], vec![0.0, 0.7]]).expect("response");
        let counts = response
            .expected_counts(&[10.0, 10.0], &[2.0, 3.0], 10.0)
            .expect("expected counts");
        assert_eq!(counts, vec![60.0, 100.0]);
    }

    #[test]
    fn response_matrix_rejects_negative_and_nonfinite_probabilities() {
        assert!(DetectorResponseMatrix::new(vec![vec![1.1]]).is_err());
        assert!(DetectorResponseMatrix::new(vec![vec![-0.1]]).is_err());
        assert_eq!(
            DetectorResponseMatrix::new(vec![vec![f64::NAN]]),
            Err(ModelError::NonFiniteValue("detector_response_probability"))
        );
    }

    #[test]
    fn response_matrix_rejects_column_efficiency_above_one() {
        assert_eq!(
            DetectorResponseMatrix::new(vec![vec![0.7], vec![0.4]]),
            Err(ModelError::InvalidParameter(
                "detector_response_column_efficiency_exceeds_one"
            ))
        );
    }

    #[test]
    fn response_matrix_rejects_ragged_rows_and_spectrum_mismatch() {
        assert_eq!(
            DetectorResponseMatrix::new(vec![vec![0.5, 0.5], vec![1.0]]),
            Err(ModelError::MismatchedDataLengths)
        );
        let matrix = DetectorResponseMatrix::new(vec![vec![1.0, 0.0]]).expect("matrix");
        assert_eq!(matrix.apply(&[1.0]), Err(ModelError::MismatchedDataLengths));
    }

    #[test]
    fn expected_counts_reject_invalid_background_and_exposure() {
        let matrix = DetectorResponseMatrix::new(vec![vec![1.0]]).expect("matrix");
        assert_eq!(
            matrix.expected_counts(&[1.0], &[-0.1], 1.0),
            Err(ModelError::InvalidParameter(
                "background_rate_must_be_nonnegative"
            ))
        );
        assert_eq!(
            matrix.expected_counts(&[1.0], &[0.0], 0.0),
            Err(ModelError::NonPositiveValue("detector_exposure_time_s"))
        );
    }
}
