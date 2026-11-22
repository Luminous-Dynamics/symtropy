//! Covariance-aware spectrum shape comparison with a profiled scale nuisance.
//!
//! This metric evaluates spectral shape under a full reference covariance matrix.
//! Absolute fluence/rate calibration must be scored separately; the profiled
//! normalization is deliberately not treated as a recovered absolute scale.

use crate::ModelError;

#[derive(Debug, Clone, PartialEq)]
pub struct SpectrumShapeScore {
    /// Nonnegative scale multiplying the predicted shape to best fit reference.
    pub profiled_scale: f64,
    pub chi_square: f64,
    pub degrees_of_freedom: usize,
    pub reduced_chi_square: f64,
}

/// Full-covariance GLS shape comparison with a profiled nonnegative scale.
///
/// Computes alpha = (p^T C^-1 y) / (p^T C^-1 p), then
/// chi2 = (y - alpha p)^T C^-1 (y - alpha p).
///
/// C must be a symmetric positive-definite covariance matrix. The metric is
/// suited to comparing versioned reference spectra with correlated uncertainty.
/// Do not interpret alpha as measured absolute fluence: it is the nuisance scale
/// removed to compare shape only. A separate absolute-normalization score is
/// required for rate or fluence claims.
pub fn covariance_profiled_spectrum_shape_chi_square(
    reference_spectrum: &[f64],
    predicted_shape: &[f64],
    covariance: &[Vec<f64>],
) -> Result<SpectrumShapeScore, ModelError> {
    let dimension = reference_spectrum.len();
    if dimension < 2
        || predicted_shape.len() != dimension
        || covariance.len() != dimension
        || covariance.iter().any(|row| row.len() != dimension)
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    for (values, name) in [
        (reference_spectrum, "reference_spectrum"),
        (predicted_shape, "predicted_spectrum_shape"),
    ] {
        for value in values {
            if !value.is_finite() {
                return Err(ModelError::NonFiniteValue(name));
            }
            if *value < 0.0 {
                return Err(ModelError::InvalidParameter(
                    "spectrum_values_must_be_nonnegative",
                ));
            }
        }
    }
    if reference_spectrum.iter().all(|value| *value == 0.0)
        || predicted_shape.iter().all(|value| *value == 0.0)
    {
        return Err(ModelError::NonPositiveValue("spectrum_total"));
    }
    for value in covariance.iter().flatten() {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue("spectrum_covariance"));
        }
    }

    let scale = covariance
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    let symmetry_tolerance = 1.0e-10 * scale.max(f64::MIN_POSITIVE);
    for (row_index, row) in covariance.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate().take(row_index) {
            if (value - covariance[column_index][row_index]).abs() > symmetry_tolerance {
                return Err(ModelError::InvalidCovariance(
                    "spectrum_covariance_must_be_symmetric",
                ));
            }
        }
    }

    let lower = cholesky_lower(covariance)?;
    let inverse_reference = solve_from_cholesky(&lower, reference_spectrum)?;
    let inverse_prediction = solve_from_cholesky(&lower, predicted_shape)?;
    let numerator = dot(predicted_shape, &inverse_reference);
    let denominator = dot(predicted_shape, &inverse_prediction);
    if !numerator.is_finite() || !denominator.is_finite() {
        return Err(ModelError::InvalidCovariance(
            "spectrum_gls_scale_is_nonfinite",
        ));
    }
    if denominator <= 0.0 {
        return Err(ModelError::InvalidCovariance(
            "spectrum_gls_scale_denominator_not_positive",
        ));
    }
    if numerator <= 0.0 {
        return Err(ModelError::InvalidParameter(
            "profiled_spectrum_scale_not_positive",
        ));
    }
    let profiled_scale = numerator / denominator;
    if !profiled_scale.is_finite() || profiled_scale <= 0.0 {
        return Err(ModelError::InvalidParameter(
            "profiled_spectrum_scale_invalid",
        ));
    }

    let residual: Vec<f64> = reference_spectrum
        .iter()
        .zip(predicted_shape.iter())
        .map(|(reference, prediction)| reference - profiled_scale * prediction)
        .collect();
    let inverse_residual = solve_from_cholesky(&lower, &residual)?;
    let mut chi_square = dot(&residual, &inverse_residual);
    if !chi_square.is_finite() {
        return Err(ModelError::InvalidCovariance(
            "spectrum_shape_chi_square_nonfinite",
        ));
    }
    if chi_square < -1.0e-10 {
        return Err(ModelError::InvalidCovariance(
            "spectrum_shape_chi_square_negative",
        ));
    }
    chi_square = chi_square.max(0.0);
    let degrees_of_freedom = dimension - 1;
    let reduced_chi_square = chi_square / degrees_of_freedom as f64;

    Ok(SpectrumShapeScore {
        profiled_scale,
        chi_square,
        degrees_of_freedom,
        reduced_chi_square,
    })
}

fn cholesky_lower(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, ModelError> {
    let dimension = matrix.len();
    let mut lower = vec![vec![0.0; dimension]; dimension];
    for row in 0..dimension {
        for column in 0..=row {
            let mut value = matrix[row][column];
            for (inner, lower_value) in lower[row].iter().enumerate().take(column) {
                value -= lower_value * lower[column][inner];
            }
            if row == column {
                if !value.is_finite() || value <= 0.0 {
                    return Err(ModelError::InvalidCovariance(
                        "spectrum_covariance_must_be_positive_definite",
                    ));
                }
                lower[row][column] = value.sqrt();
            } else {
                lower[row][column] = value / lower[column][column];
            }
        }
    }
    Ok(lower)
}

fn solve_from_cholesky(
    lower: &[Vec<f64>],
    right_hand_side: &[f64],
) -> Result<Vec<f64>, ModelError> {
    let dimension = lower.len();
    if right_hand_side.len() != dimension {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut forward = vec![0.0; dimension];
    for row in 0..dimension {
        let mut value = right_hand_side[row];
        for column in 0..row {
            value -= lower[row][column] * forward[column];
        }
        forward[row] = value / lower[row][row];
    }
    let mut solution = vec![0.0; dimension];
    for row in (0..dimension).rev() {
        let mut value = forward[row];
        for column in (row + 1)..dimension {
            value -= lower[column][row] * solution[column];
        }
        solution[row] = value / lower[row][row];
    }
    if solution.iter().any(|value| !value.is_finite()) {
        return Err(ModelError::InvalidCovariance(
            "spectrum_covariance_solve_nonfinite",
        ));
    }
    Ok(solution)
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_shape_recovers_profiled_normalization_and_zero_residual() {
        let score = covariance_profiled_spectrum_shape_chi_square(
            &[2.0, 4.0, 6.0],
            &[1.0, 2.0, 3.0],
            &[
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0],
                vec![0.0, 0.0, 1.0],
            ],
        )
        .expect("shape score");
        assert!((score.profiled_scale - 2.0).abs() < 1.0e-12);
        assert!(score.chi_square < 1.0e-24);
        assert_eq!(score.degrees_of_freedom, 2);
    }

    #[test]
    fn shape_score_is_invariant_to_positive_reference_normalization() {
        let covariance = vec![vec![0.04, 0.01], vec![0.01, 0.09]];
        let first =
            covariance_profiled_spectrum_shape_chi_square(&[2.0, 5.0], &[1.0, 3.0], &covariance)
                .expect("first score");
        let second = covariance_profiled_spectrum_shape_chi_square(
            &[20.0, 50.0],
            &[1.0, 3.0],
            &[vec![4.0, 1.0], vec![1.0, 9.0]],
        )
        .expect("scaled score");
        assert!((first.chi_square - second.chi_square).abs() < 1.0e-10);
        assert!((second.profiled_scale / first.profiled_scale - 10.0).abs() < 1.0e-10);
    }

    #[test]
    fn correlated_covariance_changes_shape_score_from_diagonal_weighting() {
        let diagonal = covariance_profiled_spectrum_shape_chi_square(
            &[2.0, 5.0],
            &[1.0, 3.0],
            &[vec![0.04, 0.0], vec![0.0, 0.09]],
        )
        .expect("diagonal");
        let correlated = covariance_profiled_spectrum_shape_chi_square(
            &[2.0, 5.0],
            &[1.0, 3.0],
            &[vec![0.04, 0.02], vec![0.02, 0.09]],
        )
        .expect("correlated");
        assert!((diagonal.chi_square - correlated.chi_square).abs() > 1.0e-6);
    }

    #[test]
    fn rejects_dimension_mismatch_and_one_bin_shape() {
        assert_eq!(
            covariance_profiled_spectrum_shape_chi_square(&[1.0], &[1.0], &[vec![1.0]]),
            Err(ModelError::MismatchedDataLengths)
        );
        assert_eq!(
            covariance_profiled_spectrum_shape_chi_square(
                &[1.0, 2.0],
                &[1.0],
                &[vec![1.0, 0.0], vec![0.0, 1.0]]
            ),
            Err(ModelError::MismatchedDataLengths)
        );
    }

    #[test]
    fn rejects_non_symmetric_or_non_positive_definite_covariance() {
        assert_eq!(
            covariance_profiled_spectrum_shape_chi_square(
                &[1.0, 2.0],
                &[1.0, 1.0],
                &[vec![1.0, 0.2], vec![0.1, 1.0]]
            ),
            Err(ModelError::InvalidCovariance(
                "spectrum_covariance_must_be_symmetric"
            ))
        );
        assert_eq!(
            covariance_profiled_spectrum_shape_chi_square(
                &[1.0, 2.0],
                &[1.0, 1.0],
                &[vec![1.0, 2.0], vec![2.0, 1.0]]
            ),
            Err(ModelError::InvalidCovariance(
                "spectrum_covariance_must_be_positive_definite"
            ))
        );
    }

    #[test]
    fn rejects_zero_spectra_and_nonpositive_profiled_scale() {
        assert_eq!(
            covariance_profiled_spectrum_shape_chi_square(
                &[0.0, 0.0],
                &[1.0, 1.0],
                &[vec![1.0, 0.0], vec![0.0, 1.0]]
            ),
            Err(ModelError::NonPositiveValue("spectrum_total"))
        );
    }

    #[test]
    fn rejects_covariance_geometry_with_nonpositive_profiled_scale() {
        assert_eq!(
            covariance_profiled_spectrum_shape_chi_square(
                &[1.0, 10.0],
                &[10.0, 1.0],
                &[vec![1.0, 0.99], vec![0.99, 1.0]],
            ),
            Err(ModelError::InvalidParameter(
                "profiled_spectrum_scale_not_positive"
            ))
        );
    }

    #[test]
    fn rejects_negative_and_nonfinite_spectrum_values() {
        assert_eq!(
            covariance_profiled_spectrum_shape_chi_square(
                &[1.0, -1.0],
                &[1.0, 1.0],
                &[vec![1.0, 0.0], vec![0.0, 1.0]]
            ),
            Err(ModelError::InvalidParameter(
                "spectrum_values_must_be_nonnegative"
            ))
        );
        assert_eq!(
            covariance_profiled_spectrum_shape_chi_square(
                &[f64::NAN, 1.0],
                &[1.0, 1.0],
                &[vec![1.0, 0.0], vec![0.0, 1.0]]
            ),
            Err(ModelError::NonFiniteValue("reference_spectrum"))
        );
    }
}
