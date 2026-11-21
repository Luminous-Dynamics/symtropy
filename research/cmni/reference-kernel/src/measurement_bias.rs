use crate::ModelError;

/// Apply a caller-supplied log-linear feature bias to physical expected counts.
/// Raw source observations are never modified. This function does not estimate
/// coefficients, assign priors, or infer causality.
pub fn apply_feature_conditioned_multiplicative_bias(
    expected_counts: &[Vec<f64>],
    experiment_features: &[Vec<f64>],
    coefficients_by_channel: &[Vec<f64>],
) -> Result<Vec<Vec<f64>>, ModelError> {
    if expected_counts.is_empty()
        || expected_counts.len() != experiment_features.len()
        || coefficients_by_channel.is_empty()
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    let feature_count = experiment_features.first().map_or(0, Vec::len);
    if feature_count == 0
        || experiment_features
            .iter()
            .any(|row| row.len() != feature_count)
        || coefficients_by_channel
            .iter()
            .any(|row| row.len() != feature_count)
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    let channel_count = expected_counts[0].len();
    if channel_count == 0
        || expected_counts.iter().any(|row| row.len() != channel_count)
        || coefficients_by_channel.len() != channel_count
    {
        return Err(ModelError::MismatchedDataLengths);
    }

    for row in expected_counts {
        for value in row {
            if !value.is_finite() {
                return Err(ModelError::NonFiniteValue("physical_expected_count"));
            }
            if *value <= 0.0 {
                return Err(ModelError::NonPositiveValue("physical_expected_count"));
            }
        }
    }
    for value in experiment_features.iter().flatten() {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue("experiment_feature"));
        }
    }
    for value in coefficients_by_channel.iter().flatten() {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue("measurement_bias_coefficient"));
        }
    }

    let mut biased = vec![vec![0.0; channel_count]; expected_counts.len()];
    for run in 0..expected_counts.len() {
        for channel in 0..channel_count {
            let log_bias = experiment_features[run]
                .iter()
                .zip(coefficients_by_channel[channel].iter())
                .map(|(feature, coefficient)| feature * coefficient)
                .sum::<f64>();
            if !log_bias.is_finite() {
                return Err(ModelError::InvalidParameter(
                    "measurement_log_bias_nonfinite",
                ));
            }
            let factor = log_bias.exp();
            if !factor.is_finite() || factor <= 0.0 {
                return Err(ModelError::InvalidParameter(
                    "measurement_bias_factor_invalid",
                ));
            }
            let value = expected_counts[run][channel] * factor;
            if !value.is_finite() || value <= 0.0 {
                return Err(ModelError::InvalidParameter(
                    "biased_expected_count_invalid",
                ));
            }
            biased[run][channel] = value;
        }
    }
    Ok(biased)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_coefficients_preserve_physical_predictions() {
        let actual = apply_feature_conditioned_multiplicative_bias(
            &[vec![10.0, 20.0], vec![30.0, 40.0]],
            &[vec![1.0, 0.0], vec![0.0, 1.0]],
            &[vec![0.0, 0.0], vec![0.0, 0.0]],
        )
        .expect("zero bias");
        assert_eq!(actual, vec![vec![10.0, 20.0], vec![30.0, 40.0]]);
    }

    #[test]
    fn applies_log_linear_multiplicative_factor() {
        let actual = apply_feature_conditioned_multiplicative_bias(
            &[vec![100.0], vec![100.0]],
            &[vec![0.0], vec![1.0]],
            &[vec![2.0]],
        )
        .expect("bias");
        assert_eq!(actual[0][0], 100.0);
        assert!((actual[1][0] - 100.0 * 2.0_f64.exp()).abs() < 1.0e-10);
    }

    #[test]
    fn coefficients_are_channel_specific() {
        let actual = apply_feature_conditioned_multiplicative_bias(
            &[vec![10.0, 10.0]],
            &[vec![1.0]],
            &[vec![1.0], vec![-1.0]],
        )
        .expect("channel-specific bias");
        assert!((actual[0][0] - 10.0 * 1.0_f64.exp()).abs() < 1.0e-12);
        assert!((actual[0][1] - 10.0 * (-1.0_f64).exp()).abs() < 1.0e-12);
    }

    #[test]
    fn rejects_shape_mismatch() {
        assert_eq!(
            apply_feature_conditioned_multiplicative_bias(
                &[vec![1.0]],
                &[vec![1.0, 2.0]],
                &[vec![1.0]]
            ),
            Err(ModelError::MismatchedDataLengths)
        );
        assert_eq!(
            apply_feature_conditioned_multiplicative_bias(
                &[vec![1.0, 2.0]],
                &[vec![1.0]],
                &[vec![1.0]]
            ),
            Err(ModelError::MismatchedDataLengths)
        );
    }

    #[test]
    fn rejects_nonpositive_predictions() {
        assert_eq!(
            apply_feature_conditioned_multiplicative_bias(&[vec![0.0]], &[vec![1.0]], &[vec![1.0]]),
            Err(ModelError::NonPositiveValue("physical_expected_count"))
        );
    }

    #[test]
    fn rejects_nonfinite_features_and_coefficients() {
        assert_eq!(
            apply_feature_conditioned_multiplicative_bias(
                &[vec![1.0]],
                &[vec![f64::NAN]],
                &[vec![1.0]]
            ),
            Err(ModelError::NonFiniteValue("experiment_feature"))
        );
        assert_eq!(
            apply_feature_conditioned_multiplicative_bias(
                &[vec![1.0]],
                &[vec![1.0]],
                &[vec![f64::INFINITY]]
            ),
            Err(ModelError::NonFiniteValue("measurement_bias_coefficient"))
        );
    }

    #[test]
    fn rejects_overflowing_bias_exponents() {
        assert_eq!(
            apply_feature_conditioned_multiplicative_bias(
                &[vec![1.0]],
                &[vec![1000.0]],
                &[vec![1.0]]
            ),
            Err(ModelError::InvalidParameter(
                "measurement_bias_factor_invalid"
            ))
        );
    }
}
