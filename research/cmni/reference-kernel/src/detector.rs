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

        if probabilities.iter().any(|row| row.len() != true_energy_bins) {
            return Err(ModelError::MismatchedDataLengths);
        }

        for row in &probabilities {
            for value in row {
                if !value.is_finite() {
                    return Err(ModelError::NonFiniteValue(
                        "detector_response_probability",
                    ));
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
            *efficiency = probabilities
                .iter()
                .map(|row| row[true_bin])
                .sum::<f64>();
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
    pub fn apply(
        &self,
        true_spectrum_rate_s: &[f64],
    ) -> Result<Vec<f64>, ModelError> {
        if true_spectrum_rate_s.len() != self.true_energy_bins {
            return Err(ModelError::MismatchedDataLengths);
        }
        for value in true_spectrum_rate_s {
            if !value.is_finite() {
                return Err(ModelError::NonFiniteValue(
                    "true_spectrum_rate_per_s",
                ));
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
            return Err(ModelError::NonPositiveValue(
                "detector_exposure_time_s",
            ));
        }
        for value in background_rate_counts_s {
            if !value.is_finite() {
                return Err(ModelError::NonFiniteValue(
                    "background_rate_counts_s",
                ));
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
    fn identity_response_recovers_true_spectrum() {
        let response = DetectorResponseMatrix::new(vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ]).expect("identity response");
        let measured = response.apply(&[10.0, 20.0, 30.0]).expect("forward response");
        assert_eq!(measured, vec![10.0, 20.0, 30.0]);
        assert_eq!(response.column_efficiencies(), &[1.0, 1.0, 1.0]);
    }

    #[test]
    fn response_matrix_models_energy_migration_and_efficiency_loss() {
        let response = DetectorResponseMatrix::new(vec![
            vec![0.5, 0.1],
            vec![0.3, 0.6],
        ]).expect("valid response");
        let measured = response.apply(&[100.0, 50.0]).expect("apply");
        assert_eq!(measured, vec![55.0, 60.0]);
        assert_eq!(response.column_efficiencies(), &[0.8, 0.7]);
    }

    #[test]
    fn expected_counts_add_background_after_response() {
        let response = DetectorResponseMatrix::new(vec![
            vec![0.4, 0.0],
            vec![0.0, 0.7],
        ]).expect("response");
        let counts = response.expected_counts(&[10.0, 10.0], &[2.0, 3.0], 10.0)
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
        assert_eq!(
            matrix.apply(&[1.0]),
            Err(ModelError::MismatchedDataLengths)
        );
    }

    #[test]
    fn expected_counts_reject_invalid_background_and_exposure() {
        let matrix = DetectorResponseMatrix::new(vec![vec![1.0]]).expect("matrix");
        assert_eq!(
            matrix.expected_counts(&[1.0], &[-0.1], 1.0),
            Err(ModelError::InvalidParameter("background_rate_must_be_nonnegative"))
        );
        assert_eq!(
            matrix.expected_counts(&[1.0], &[0.0], 0.0),
            Err(ModelError::NonPositiveValue("detector_exposure_time_s"))
        );
    }
}
