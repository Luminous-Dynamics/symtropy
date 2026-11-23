// SPDX-License-Identifier: AGPL-3.0-or-later

//! Compact-support spatial covariance for PIV interrogation-window errors.
//!
//! Adds Cov(delta_u_i, delta_u_j) = K(x_i,x_j) C_component to the local
//! covariance and shared-mode model used by the field-functional reducer.
//! The caller must exclude this same component from per-sample local
//! covariance to avoid double counting. Kernel parameters require an external
//! calibration receipt; overlap percentage alone is not a fitted covariance.

use std::error::Error;
use std::fmt;

use crate::functional::{
    Covariance3, FieldFunctionalInput, FieldFunctionalReport,
    propagate_field_functional_uncertainty,
};

#[derive(Clone, Debug, PartialEq)]
pub struct SpatialKernelCalibrationReceipt {
    pub receipt_id: String,
    pub protocol_fingerprint: String,
    /// Number of independent acquisitions used to estimate spatial error
    /// correlation for this processing configuration.
    pub independent_acquisition_count: usize,
    pub maximum_absolute_correlation_fit_error: f64,
    pub maximum_allowed_absolute_correlation_fit_error: f64,
    /// The current nonnegative Wendland family cannot represent a negative
    /// correlation lobe. A different kernel is required when one is resolved.
    pub negative_correlation_lobe_detected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpatialErrorKernel {
    pub model_id: String,
    /// Must match the fingerprint in the calibration receipt exactly.
    pub protocol_fingerprint: String,
    pub calibration_receipt: SpatialKernelCalibrationReceipt,
    pub length_scales_m: [f64; 3],
    pub component_covariance_m2_s2: Covariance3,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpatialFieldFunctionalReport {
    pub base_report: FieldFunctionalReport,
    pub kernel_model_id: String,
    pub parameter_calibration_receipt_id: String,
    pub calibration_protocol_fingerprint: String,
    pub calibration_acquisition_count: usize,
    pub maximum_absolute_correlation_fit_error: f64,
    pub correlated_sample_pair_count: usize,
    pub spatial_covariance_contribution: [[f64; 4]; 4],
    pub total_functional_covariance: [[f64; 4]; 4],
    pub total_standard_uncertainties: [f64; 4],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpatialFunctionalError {
    BaseFunctional(String),
    CoordinateCountMismatch { expected: usize, actual: usize },
    NonFiniteCoordinate { sample_index: usize },
    EmptyKernelIdentity { field: &'static str },
    UnsupportedKernel { model_id: String },
    KernelProtocolMismatch,
    InsufficientCalibrationAcquisitions { actual: usize },
    InvalidCalibrationDiagnostics,
    CorrelationFitErrorExceedsLimit { actual: f64, maximum: f64 },
    NegativeCorrelationLobeRequiresDifferentModel,
    InvalidLengthScale { axis: usize },
    NonFiniteKernelCovariance,
    KernelCovarianceNotPositiveSemidefinite,
    NonFiniteDerivedValue,
    NegativeTotalVariance { functional_index: usize },
}

impl fmt::Display for SpatialFunctionalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BaseFunctional(error) => {
                write!(f, "base field-functional propagation failed: {error}")
            }
            Self::CoordinateCountMismatch { expected, actual } => {
                write!(f, "coordinate count {actual}; expected {expected}")
            }
            Self::NonFiniteCoordinate { sample_index } => {
                write!(f, "non-finite coordinate at sample {sample_index}")
            }
            Self::EmptyKernelIdentity { field } => write!(f, "kernel {field} must be non-empty"),
            Self::UnsupportedKernel { model_id } => write!(f, "unsupported kernel {model_id}"),
            Self::KernelProtocolMismatch => write!(
                f,
                "spatial kernel protocol differs from its calibration receipt"
            ),
            Self::InsufficientCalibrationAcquisitions { actual } => write!(
                f,
                "spatial kernel has only {actual} independent calibration acquisitions; at least 20 are required by this reference gate"
            ),
            Self::InvalidCalibrationDiagnostics => {
                write!(f, "spatial kernel calibration diagnostics are invalid")
            }
            Self::CorrelationFitErrorExceedsLimit { actual, maximum } => write!(
                f,
                "spatial kernel correlation fit error {actual} exceeds maximum {maximum}"
            ),
            Self::NegativeCorrelationLobeRequiresDifferentModel => write!(
                f,
                "observed negative spatial correlation lobe is not represented by the current Wendland model"
            ),
            Self::InvalidLengthScale { axis } => write!(f, "invalid length scale on axis {axis}"),
            Self::NonFiniteKernelCovariance => write!(f, "kernel covariance is non-finite"),
            Self::KernelCovarianceNotPositiveSemidefinite => write!(
                f,
                "kernel component covariance is not positive semidefinite"
            ),
            Self::NonFiniteDerivedValue => write!(f, "spatial functional covariance is non-finite"),
            Self::NegativeTotalVariance { functional_index } => {
                write!(f, "negative variance for functional {functional_index}")
            }
        }
    }
}

impl Error for SpatialFunctionalError {}

fn validate_psd(c: Covariance3) -> Result<(), SpatialFunctionalError> {
    let a = [c.xx, c.xy, c.xz, c.yy, c.yz, c.zz];
    if !a.iter().all(|v| v.is_finite()) {
        return Err(SpatialFunctionalError::NonFiniteKernelCovariance);
    }
    let scale = a.iter().map(|v| v.abs()).fold(0.0_f64, f64::max).max(1.0);
    let t1 = 1.0e-12 * scale;
    let t2 = 1.0e-12 * scale * scale;
    let t3 = 1.0e-12 * scale * scale * scale;
    let mxy = c.xx * c.yy - c.xy * c.xy;
    let mxz = c.xx * c.zz - c.xz * c.xz;
    let myz = c.yy * c.zz - c.yz * c.yz;
    let det = c.xx * (c.yy * c.zz - c.yz * c.yz) - c.xy * (c.xy * c.zz - c.xz * c.yz)
        + c.xz * (c.xy * c.yz - c.xz * c.yy);
    if c.xx < -t1
        || c.yy < -t1
        || c.zz < -t1
        || mxy < -t2
        || mxz < -t2
        || myz < -t2
        || !det.is_finite()
        || det < -t3
    {
        return Err(SpatialFunctionalError::KernelCovarianceNotPositiveSemidefinite);
    }
    Ok(())
}

fn bilinear(c: Covariance3, left: [f64; 3], right: [f64; 3]) -> f64 {
    let cr = [
        c.xx * right[0] + c.xy * right[1] + c.xz * right[2],
        c.xy * right[0] + c.yy * right[1] + c.yz * right[2],
        c.xz * right[0] + c.yz * right[1] + c.zz * right[2],
    ];
    left[0] * cr[0] + left[1] * cr[1] + left[2] * cr[2]
}

fn jacobian(u: [f64; 3], area: [f64; 3], rho: f64) -> [[f64; 3]; 4] {
    let normal_flux = u[0] * area[0] + u[1] * area[1] + u[2] * area[2];
    let mut j = [[0.0; 3]; 4];
    for axis in 0..3 {
        j[0][axis] = rho * area[axis];
    }
    for k in 0..3 {
        for l in 0..3 {
            let identity = if k == l { normal_flux } else { 0.0 };
            j[k + 1][l] = rho * (identity + u[k] * area[l]);
        }
    }
    j
}

/// Wendland C2 compact-support correlation in R^3:
/// phi(r) = (1-r)^4(4r+1) for r<1, otherwise zero.
/// Anisotropy is supplied by the axis-specific length scales.
fn wendland_c2(r: f64) -> f64 {
    if r >= 1.0 {
        0.0
    } else {
        (1.0 - r).powi(4) * (4.0 * r + 1.0)
    }
}

/// Add a compact-support spatially correlated velocity-error component.
/// Reference implementation is O(N^2); use neighbor lists or matrix-free
/// evaluation for large fields without changing the covariance contract.
pub fn propagate_spatially_correlated_functional_uncertainty(
    base: &FieldFunctionalInput,
    coordinates_m: &[[f64; 3]],
    kernel: &SpatialErrorKernel,
) -> Result<SpatialFieldFunctionalReport, SpatialFunctionalError> {
    if kernel.model_id.trim().is_empty() {
        return Err(SpatialFunctionalError::EmptyKernelIdentity { field: "model_id" });
    }
    if kernel.protocol_fingerprint.trim().is_empty()
        || kernel.calibration_receipt.receipt_id.trim().is_empty()
        || kernel
            .calibration_receipt
            .protocol_fingerprint
            .trim()
            .is_empty()
    {
        return Err(SpatialFunctionalError::EmptyKernelIdentity {
            field: "calibration_receipt_or_protocol_fingerprint",
        });
    }
    if kernel.protocol_fingerprint != kernel.calibration_receipt.protocol_fingerprint {
        return Err(SpatialFunctionalError::KernelProtocolMismatch);
    }
    if kernel.calibration_receipt.independent_acquisition_count < 20 {
        return Err(
            SpatialFunctionalError::InsufficientCalibrationAcquisitions {
                actual: kernel.calibration_receipt.independent_acquisition_count,
            },
        );
    }
    let fit_error = kernel
        .calibration_receipt
        .maximum_absolute_correlation_fit_error;
    let maximum_fit_error = kernel
        .calibration_receipt
        .maximum_allowed_absolute_correlation_fit_error;
    if !fit_error.is_finite()
        || !maximum_fit_error.is_finite()
        || fit_error < 0.0
        || maximum_fit_error <= 0.0
    {
        return Err(SpatialFunctionalError::InvalidCalibrationDiagnostics);
    }
    if fit_error > maximum_fit_error {
        return Err(SpatialFunctionalError::CorrelationFitErrorExceedsLimit {
            actual: fit_error,
            maximum: maximum_fit_error,
        });
    }
    if kernel
        .calibration_receipt
        .negative_correlation_lobe_detected
    {
        return Err(SpatialFunctionalError::NegativeCorrelationLobeRequiresDifferentModel);
    }
    if kernel.model_id != "wendland_c2_anisotropic_3d" {
        return Err(SpatialFunctionalError::UnsupportedKernel {
            model_id: kernel.model_id.clone(),
        });
    }
    if coordinates_m.len() != base.samples.len() {
        return Err(SpatialFunctionalError::CoordinateCountMismatch {
            expected: base.samples.len(),
            actual: coordinates_m.len(),
        });
    }
    for (i, x) in coordinates_m.iter().enumerate() {
        if !x.iter().all(|v| v.is_finite()) {
            return Err(SpatialFunctionalError::NonFiniteCoordinate { sample_index: i });
        }
    }
    for (axis, length) in kernel.length_scales_m.iter().enumerate() {
        if !length.is_finite() || *length <= 0.0 {
            return Err(SpatialFunctionalError::InvalidLengthScale { axis });
        }
    }
    validate_psd(kernel.component_covariance_m2_s2)?;

    let base_report = propagate_field_functional_uncertainty(base)
        .map_err(|e| SpatialFunctionalError::BaseFunctional(e.to_string()))?;
    let js: Vec<[[f64; 3]; 4]> = base
        .samples
        .iter()
        .map(|s| jacobian(s.velocity_m_s, s.oriented_area_vector_m2, s.density_kg_m3))
        .collect();

    let mut contribution = [[0.0_f64; 4]; 4];
    let mut pair_count = 0usize;
    let n = base.samples.len();
    for i in 0..n {
        for j in 0..n {
            let dx = (coordinates_m[i][0] - coordinates_m[j][0]) / kernel.length_scales_m[0];
            let dy = (coordinates_m[i][1] - coordinates_m[j][1]) / kernel.length_scales_m[1];
            let dz = (coordinates_m[i][2] - coordinates_m[j][2]) / kernel.length_scales_m[2];
            let r = (dx * dx + dy * dy + dz * dz).sqrt();
            if !r.is_finite() {
                return Err(SpatialFunctionalError::NonFiniteDerivedValue);
            }
            let k = wendland_c2(r);
            if k == 0.0 {
                continue;
            }
            if j >= i {
                pair_count += 1;
            }
            for a in 0..4 {
                for b in 0..4 {
                    contribution[a][b] +=
                        k * bilinear(kernel.component_covariance_m2_s2, js[i][a], js[j][b]);
                }
            }
        }
    }

    let mut total = base_report.functional_covariance;
    for a in 0..4 {
        for b in 0..4 {
            contribution[a][b] *= base.covariance_multiplier;
            total[a][b] += contribution[a][b];
            if !total[a][b].is_finite() {
                return Err(SpatialFunctionalError::NonFiniteDerivedValue);
            }
        }
    }
    let mut std = [0.0_f64; 4];
    for a in 0..4 {
        if total[a][a] < -1.0e-12 {
            return Err(SpatialFunctionalError::NegativeTotalVariance {
                functional_index: a,
            });
        }
        std[a] = total[a][a].max(0.0).sqrt();
    }

    Ok(SpatialFieldFunctionalReport {
        base_report,
        kernel_model_id: kernel.model_id.clone(),
        parameter_calibration_receipt_id: kernel.calibration_receipt.receipt_id.clone(),
        calibration_protocol_fingerprint: kernel.protocol_fingerprint.clone(),
        calibration_acquisition_count: kernel.calibration_receipt.independent_acquisition_count,
        maximum_absolute_correlation_fit_error: kernel
            .calibration_receipt
            .maximum_absolute_correlation_fit_error,
        correlated_sample_pair_count: pair_count,
        spatial_covariance_contribution: contribution,
        total_functional_covariance: total,
        total_standard_uncertainties: std,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functional::{FaceVelocitySample, FieldFunctionalInput};

    fn base(samples: Vec<FaceVelocitySample>) -> FieldFunctionalInput {
        FieldFunctionalInput {
            run_id: "spatial-test".into(),
            control_surface_id: "surface-1".into(),
            sign_convention_id: "outward-v1".into(),
            covariance_multiplier: 1.0,
            samples,
            shared_modes: Vec::new(),
        }
    }

    fn face() -> FaceVelocitySample {
        FaceVelocitySample {
            velocity_m_s: [2.0, 0.0, 0.0],
            oriented_area_vector_m2: [1.0, 0.0, 0.0],
            density_kg_m3: 1.0,
            local_velocity_covariance_m2_s2: Covariance3 {
                xx: 0.0,
                xy: 0.0,
                xz: 0.0,
                yy: 0.0,
                yz: 0.0,
                zz: 0.0,
            },
        }
    }

    fn kernel(c: Covariance3) -> SpatialErrorKernel {
        SpatialErrorKernel {
            model_id: "wendland_c2_anisotropic_3d".into(),
            protocol_fingerprint: "piv-processing-32x32-75overlap-v1".into(),
            calibration_receipt: SpatialKernelCalibrationReceipt {
                receipt_id: "fit-receipt-001".into(),
                protocol_fingerprint: "piv-processing-32x32-75overlap-v1".into(),
                independent_acquisition_count: 24,
                maximum_absolute_correlation_fit_error: 0.02,
                maximum_allowed_absolute_correlation_fit_error: 0.05,
                negative_correlation_lobe_detected: false,
            },
            length_scales_m: [1.0; 3],
            component_covariance_m2_s2: c,
        }
    }

    #[test]
    fn overlapping_windows_add_cross_sample_mass_covariance() {
        let input = base(vec![face(), face()]);
        let c = Covariance3 {
            xx: 0.01,
            xy: 0.0,
            xz: 0.0,
            yy: 0.0,
            yz: 0.0,
            zz: 0.0,
        };
        let result = propagate_spatially_correlated_functional_uncertainty(
            &input,
            &[[0.0; 3], [0.5, 0.0, 0.0]],
            &kernel(c),
        )
        .unwrap();
        let expected = 0.01 * (2.0 + 2.0 * 0.1875);
        assert!((result.spatial_covariance_contribution[0][0] - expected).abs() < 1.0e-12);
        assert_eq!(result.correlated_sample_pair_count, 3);
    }

    #[test]
    fn points_outside_compact_support_have_no_cross_covariance() {
        let input = base(vec![face(), face()]);
        let c = Covariance3 {
            xx: 0.01,
            xy: 0.0,
            xz: 0.0,
            yy: 0.0,
            yz: 0.0,
            zz: 0.0,
        };
        let result = propagate_spatially_correlated_functional_uncertainty(
            &input,
            &[[0.0; 3], [2.0, 0.0, 0.0]],
            &kernel(c),
        )
        .unwrap();
        assert!((result.spatial_covariance_contribution[0][0] - 0.02).abs() < 1.0e-12);
        assert_eq!(result.correlated_sample_pair_count, 2);
    }

    #[test]
    fn anisotropic_length_scale_changes_the_correlation() {
        let input = base(vec![face(), face()]);
        let c = Covariance3 {
            xx: 0.01,
            xy: 0.0,
            xz: 0.0,
            yy: 0.0,
            yz: 0.0,
            zz: 0.0,
        };
        let mut k = kernel(c);
        k.length_scales_m = [2.0, 1.0, 1.0];
        let result = propagate_spatially_correlated_functional_uncertainty(
            &input,
            &[[0.0; 3], [1.0, 0.0, 0.0]],
            &k,
        )
        .unwrap();
        let expected = 0.01 * (2.0 + 2.0 * 0.1875);
        assert!((result.spatial_covariance_contribution[0][0] - expected).abs() < 1.0e-12);
    }

    #[test]
    fn cross_component_covariance_reaches_integrated_cross_functionals() {
        let mut sample = face();
        sample.velocity_m_s = [1.0, 1.0, 0.0];
        sample.oriented_area_vector_m2 = [1.0, 1.0, 0.0];
        let input = base(vec![sample]);
        let c = Covariance3 {
            xx: 0.01,
            xy: 0.005,
            xz: 0.0,
            yy: 0.02,
            yz: 0.0,
            zz: 0.0,
        };
        let result =
            propagate_spatially_correlated_functional_uncertainty(&input, &[[0.0; 3]], &kernel(c))
                .unwrap();
        assert!((result.spatial_covariance_contribution[0][0] - 0.04).abs() < 1.0e-12);
        assert!((result.spatial_covariance_contribution[0][1] - 0.07).abs() < 1.0e-12);
    }

    #[test]
    fn base_local_covariance_is_retained_and_augmented() {
        let mut sample = face();
        sample.local_velocity_covariance_m2_s2.xx = 0.25;
        let input = base(vec![sample]);
        let c = Covariance3 {
            xx: 0.01,
            xy: 0.0,
            xz: 0.0,
            yy: 0.0,
            yz: 0.0,
            zz: 0.0,
        };
        let result =
            propagate_spatially_correlated_functional_uncertainty(&input, &[[0.0; 3]], &kernel(c))
                .unwrap();
        assert!((result.base_report.functional_covariance[0][0] - 0.25).abs() < 1.0e-12);
        assert!((result.total_functional_covariance[0][0] - 0.26).abs() < 1.0e-12);
    }

    #[test]
    fn missing_fit_receipt_and_nonfinite_coordinates_are_rejected() {
        let input = base(vec![face()]);
        let c = Covariance3 {
            xx: 0.01,
            xy: 0.0,
            xz: 0.0,
            yy: 0.0,
            yz: 0.0,
            zz: 0.0,
        };
        let mut bad = kernel(c);
        bad.calibration_receipt.receipt_id.clear();
        assert!(matches!(
            propagate_spatially_correlated_functional_uncertainty(&input, &[[0.0; 3]], &bad),
            Err(SpatialFunctionalError::EmptyKernelIdentity { .. })
        ));
        assert!(matches!(
            propagate_spatially_correlated_functional_uncertainty(
                &input,
                &[[f64::NAN, 0.0, 0.0]],
                &kernel(c)
            ),
            Err(SpatialFunctionalError::NonFiniteCoordinate { .. })
        ));
    }

    #[test]
    fn low_acquisition_count_poor_fit_and_negative_lobe_are_rejected() {
        let input = base(vec![face()]);
        let c = Covariance3 {
            xx: 0.01,
            xy: 0.0,
            xz: 0.0,
            yy: 0.0,
            yz: 0.0,
            zz: 0.0,
        };

        let mut too_few = kernel(c);
        too_few.calibration_receipt.independent_acquisition_count = 19;
        assert!(matches!(
            propagate_spatially_correlated_functional_uncertainty(&input, &[[0.0; 3]], &too_few),
            Err(SpatialFunctionalError::InsufficientCalibrationAcquisitions { actual: 19 })
        ));

        let mut poor_fit = kernel(c);
        poor_fit
            .calibration_receipt
            .maximum_absolute_correlation_fit_error = 0.08;
        assert!(matches!(
            propagate_spatially_correlated_functional_uncertainty(&input, &[[0.0; 3]], &poor_fit),
            Err(SpatialFunctionalError::CorrelationFitErrorExceedsLimit { .. })
        ));

        let mut negative_lobe = kernel(c);
        negative_lobe
            .calibration_receipt
            .negative_correlation_lobe_detected = true;
        assert!(matches!(
            propagate_spatially_correlated_functional_uncertainty(
                &input,
                &[[0.0; 3]],
                &negative_lobe
            ),
            Err(SpatialFunctionalError::NegativeCorrelationLobeRequiresDifferentModel)
        ));
    }

    #[test]
    fn target_protocol_must_match_spatial_kernel_calibration_protocol() {
        let input = base(vec![face()]);
        let c = Covariance3 {
            xx: 0.01,
            xy: 0.0,
            xz: 0.0,
            yy: 0.0,
            yz: 0.0,
            zz: 0.0,
        };
        let mut k = kernel(c);
        k.protocol_fingerprint = "different-grid-v2".into();
        assert!(matches!(
            propagate_spatially_correlated_functional_uncertainty(&input, &[[0.0; 3]], &k),
            Err(SpatialFunctionalError::KernelProtocolMismatch)
        ));
    }

    #[test]
    fn invalid_length_scale_and_nonfinite_fit_threshold_are_rejected() {
        let input = base(vec![face()]);
        let c = Covariance3 {
            xx: 0.01,
            xy: 0.0,
            xz: 0.0,
            yy: 0.0,
            yz: 0.0,
            zz: 0.0,
        };
        let mut invalid_scale = kernel(c);
        invalid_scale.length_scales_m[1] = 0.0;
        assert!(matches!(
            propagate_spatially_correlated_functional_uncertainty(
                &input,
                &[[0.0; 3]],
                &invalid_scale
            ),
            Err(SpatialFunctionalError::InvalidLengthScale { axis: 1 })
        ));

        let mut invalid_diagnostic = kernel(c);
        invalid_diagnostic
            .calibration_receipt
            .maximum_allowed_absolute_correlation_fit_error = f64::NAN;
        assert!(matches!(
            propagate_spatially_correlated_functional_uncertainty(
                &input,
                &[[0.0; 3]],
                &invalid_diagnostic
            ),
            Err(SpatialFunctionalError::InvalidCalibrationDiagnostics)
        ));
    }

    #[test]
    fn spatial_covariance_output_is_symmetric() {
        let mut a = face();
        a.velocity_m_s = [1.0, 0.5, 0.0];
        a.oriented_area_vector_m2 = [1.0, 0.2, 0.0];
        let mut b = face();
        b.velocity_m_s = [0.8, -0.1, 0.3];
        b.oriented_area_vector_m2 = [0.5, 0.8, 0.1];
        let input = base(vec![a, b]);
        let c = Covariance3 {
            xx: 0.02,
            xy: 0.003,
            xz: 0.001,
            yy: 0.015,
            yz: 0.002,
            zz: 0.01,
        };
        let result = propagate_spatially_correlated_functional_uncertainty(
            &input,
            &[[0.0; 3], [0.25, 0.15, 0.0]],
            &kernel(c),
        )
        .unwrap();
        for i in 0..4 {
            for j in 0..4 {
                assert!(
                    (result.total_functional_covariance[i][j]
                        - result.total_functional_covariance[j][i])
                        .abs()
                        < 1.0e-12
                );
            }
        }
    }

    #[test]
    fn non_psd_kernel_component_covariance_is_rejected() {
        let input = base(vec![face()]);
        let c = Covariance3 {
            xx: 1.0,
            xy: 0.9,
            xz: 0.9,
            yy: 1.0,
            yz: -0.9,
            zz: 1.0,
        };
        assert!(matches!(
            propagate_spatially_correlated_functional_uncertainty(&input, &[[0.0; 3]], &kernel(c)),
            Err(SpatialFunctionalError::KernelCovarianceNotPositiveSemidefinite)
        ));
    }
}
