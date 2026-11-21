//! Research-only depth-observation operators for instrument-aware inference.
//!
//! A latent material profile is not an instrument measurement. Each instrument
//! samples a depth distribution described by a calibrated response kernel.
//! The weights in each kernel must be nonnegative and sum to one; instrument
//! efficiency, absolute calibration and covariance are separate model terms.

use crate::ModelError;

/// One normalized depth-response kernel associated with a nominal measurement
/// depth. Weights correspond to the caller's ordered latent depth cells and
/// include cell integration / sampling response.
#[derive(Debug, Clone, PartialEq)]
pub struct DepthResponseKernel {
    nominal_depth_nm: f64,
    weights: Vec<f64>,
}

impl DepthResponseKernel {
    pub fn new(nominal_depth_nm: f64, weights: Vec<f64>) -> Result<Self, ModelError> {
        if !nominal_depth_nm.is_finite() {
            return Err(ModelError::NonFiniteValue("nominal_measurement_depth_nm"));
        }
        if nominal_depth_nm < 0.0 {
            return Err(ModelError::InvalidParameter(
                "nominal_measurement_depth_nm_must_be_nonnegative",
            ));
        }
        if weights.is_empty() {
            return Err(ModelError::InvalidParameter(
                "depth_response_kernel_must_not_be_empty",
            ));
        }
        let mut sum = 0.0;
        for weight in &weights {
            if !weight.is_finite() {
                return Err(ModelError::NonFiniteValue("depth_response_weight"));
            }
            if *weight < 0.0 {
                return Err(ModelError::InvalidParameter(
                    "depth_response_weights_must_be_nonnegative",
                ));
            }
            sum += *weight;
        }
        if !sum.is_finite() || (sum - 1.0).abs() > 1.0e-8 {
            return Err(ModelError::InvalidParameter(
                "depth_response_weights_must_sum_to_one",
            ));
        }
        Ok(Self {
            nominal_depth_nm,
            weights,
        })
    }

    pub fn nominal_depth_nm(&self) -> f64 {
        self.nominal_depth_nm
    }

    pub fn weights(&self) -> &[f64] {
        &self.weights
    }
}

/// One instrument-level observation after applying a depth-response kernel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DepthObservedValue {
    pub nominal_depth_nm: f64,
    pub value: f64,
}

/// Forward-project a latent depth profile through a set of independently
/// calibrated instrument-response kernels.
///
/// Each kernel row is normalized and may be non-Gaussian, asymmetric,
/// energy-dependent or otherwise instrument-specific. The function does not
/// infer or regularize the latent profile, normalize malformed kernels, or
/// model efficiency/noise; those belong in the instrument likelihood layer.
pub fn apply_depth_observation_operator(
    latent_profile: &[f64],
    kernels: &[DepthResponseKernel],
) -> Result<Vec<DepthObservedValue>, ModelError> {
    if latent_profile.is_empty() || kernels.is_empty() {
        return Err(ModelError::InvalidParameter(
            "latent_profile_and_depth_kernels_must_not_be_empty",
        ));
    }
    for value in latent_profile {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue("latent_depth_profile_value"));
        }
    }

    let mut observations = Vec::with_capacity(kernels.len());
    for kernel in kernels {
        if kernel.weights.len() != latent_profile.len() {
            return Err(ModelError::MismatchedDataLengths);
        }
        let value = latent_profile
            .iter()
            .zip(kernel.weights.iter())
            .map(|(profile_value, weight)| profile_value * weight)
            .sum::<f64>();
        if !value.is_finite() {
            return Err(ModelError::InvalidParameter(
                "depth_observation_operator_produced_nonfinite_value",
            ));
        }
        observations.push(DepthObservedValue {
            nominal_depth_nm: kernel.nominal_depth_nm,
            value,
        });
    }
    Ok(observations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_response_kernels_recover_the_latent_profile() {
        let profile = [1.0, 4.0, 9.0];
        let kernels = [
            DepthResponseKernel::new(5.0, vec![1.0, 0.0, 0.0]).expect("identity 1"),
            DepthResponseKernel::new(15.0, vec![0.0, 1.0, 0.0]).expect("identity 2"),
            DepthResponseKernel::new(25.0, vec![0.0, 0.0, 1.0]).expect("identity 3"),
        ];
        let observed =
            apply_depth_observation_operator(&profile, &kernels).expect("identity response");
        assert_eq!(
            observed.iter().map(|x| x.value).collect::<Vec<_>>(),
            profile
        );
        assert_eq!(observed[1].nominal_depth_nm, 15.0);
    }

    #[test]
    fn broad_response_kernel_smooths_a_narrow_surface_feature() {
        let latent = [10.0, 0.0, 0.0, 0.0];
        let narrow =
            DepthResponseKernel::new(0.0, vec![1.0, 0.0, 0.0, 0.0]).expect("narrow kernel");
        let broad =
            DepthResponseKernel::new(0.0, vec![0.5, 0.3, 0.15, 0.05]).expect("broad kernel");
        let result = apply_depth_observation_operator(&latent, &[narrow, broad])
            .expect("response projection");
        assert!((result[0].value - 10.0).abs() < 1.0e-12);
        assert!((result[1].value - 5.0).abs() < 1.0e-12);
        assert!(result[1].value < result[0].value);
    }

    #[test]
    fn response_kernels_reject_non_normalized_weights() {
        assert_eq!(
            DepthResponseKernel::new(10.0, vec![0.2, 0.2, 0.2]),
            Err(ModelError::InvalidParameter(
                "depth_response_weights_must_sum_to_one"
            ))
        );
    }

    #[test]
    fn response_operator_rejects_dimension_mismatch() {
        let profile = [1.0, 2.0, 3.0];
        let kernel = DepthResponseKernel::new(10.0, vec![0.5, 0.5]).expect("valid two-cell kernel");
        assert_eq!(
            apply_depth_observation_operator(&profile, &[kernel]),
            Err(ModelError::MismatchedDataLengths)
        );
    }

    #[test]
    fn response_kernels_reject_negative_and_nonfinite_weights() {
        assert!(DepthResponseKernel::new(0.0, vec![1.1, -0.1]).is_err());
        assert_eq!(
            DepthResponseKernel::new(0.0, vec![f64::NAN, 0.0]),
            Err(ModelError::NonFiniteValue("depth_response_weight"))
        );
    }

    #[test]
    fn response_operator_preserves_constant_profiles() {
        let profile = [7.5, 7.5, 7.5, 7.5];
        let kernels = [
            DepthResponseKernel::new(0.0, vec![0.7, 0.2, 0.1, 0.0]).expect("kernel 1"),
            DepthResponseKernel::new(20.0, vec![0.0, 0.2, 0.4, 0.4]).expect("kernel 2"),
        ];
        let observed =
            apply_depth_observation_operator(&profile, &kernels).expect("constant profile");
        assert!(observed
            .iter()
            .all(|item| (item.value - 7.5).abs() < 1.0e-12));
    }

    #[test]
    fn response_operator_rejects_nonfinite_latent_profile() {
        let kernel = DepthResponseKernel::new(0.0, vec![0.5, 0.5]).expect("kernel");
        assert_eq!(
            apply_depth_observation_operator(&[1.0, f64::INFINITY], &[kernel]),
            Err(ModelError::NonFiniteValue("latent_depth_profile_value"))
        );
    }
}
