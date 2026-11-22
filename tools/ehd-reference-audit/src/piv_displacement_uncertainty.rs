//! Local PIV displacement and velocity uncertainty from a fitted correlation peak.
//!
//! The kernel inverts a supplied 2x2 negative-log-likelihood Hessian at a
//! correlation peak (local Laplace covariance), then propagates displacement,
//! pixel-scale and pulse-separation uncertainty to velocity. It does not
//! generate the correlation plane, validate the likelihood model, or claim
//! calibrated uncertainty without an external calibration receipt.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SymmetricMatrix2 {
    pub xx: f64,
    pub xy: f64,
    pub yy: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CorrelationPeakDiagnostics {
    /// Ratio between the primary correlation peak and the largest sidelobe.
    pub peak_to_sidelobe_ratio: f64,
    pub minimum_peak_to_sidelobe_ratio: f64,
    /// RMS residual of the local peak fit in the declared correlation units.
    pub fit_residual_rms: f64,
    pub maximum_fit_residual_rms: f64,
    /// External peak-selection logic must establish a unique admissible peak.
    pub unique_peak: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PivDisplacementInput {
    pub displacement_px: [f64; 2],
    /// Hessian of the local negative log likelihood, in inverse pixel squared.
    pub negative_log_likelihood_hessian_px_inverse_squared: SymmetricMatrix2,
    pub peak_diagnostics: CorrelationPeakDiagnostics,
    /// Calibration factor from pixels to metres.
    pub metres_per_pixel: f64,
    /// One-sigma absolute uncertainty of the calibration factor, m/pixel.
    pub metres_per_pixel_standard_uncertainty: f64,
    /// Inter-frame pulse separation.
    pub pulse_separation_seconds: f64,
    /// One-sigma timing uncertainty, seconds.
    pub pulse_separation_standard_uncertainty_seconds: f64,
    /// Optional independent empirical coverage/calibration receipt.
    pub calibration_receipt_id: Option<String>,
    /// Supplied by an independent, frozen calibration/coverage evaluator.
    pub calibration_coverage_gate_passed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PivPeakDisposition {
    AcceptedForLocalCovariance,
    RejectedNonUniquePeak,
    RejectedPeakToSidelobeRatio,
    RejectedPeakFitResidual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PivUncertaintyStatus {
    LocalApproximationOnly,
    CalibrationReceiptAccepted,
    UnusablePeak,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PivDisplacementUncertaintyReport {
    pub peak_disposition: PivPeakDisposition,
    pub uncertainty_status: PivUncertaintyStatus,
    pub displacement_covariance_px_squared: Option<SymmetricMatrix2>,
    pub velocity_m_s: Option<[f64; 2]>,
    pub velocity_covariance_m2_s2: Option<SymmetricMatrix2>,
    pub velocity_standard_uncertainty_m_s: Option<[f64; 2]>,
    pub correlation_velocity_uncertainty: Option<f64>,
    pub calibration_receipt_id: Option<String>,
    pub calibration_coverage_gate_passed: bool,
    pub limitation: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PivDisplacementError {
    NonFiniteInput { field: &'static str },
    InvalidPositiveInput { field: &'static str },
    InvalidUncertainty { field: &'static str },
    NonPositiveDefinitePeakHessian,
    NonFiniteDerivedValue,
    CalibrationReceiptRequired,
}

impl fmt::Display for PivDisplacementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput { field } => write!(f, "PIV displacement {field} must be finite"),
            Self::InvalidPositiveInput { field } => write!(f, "PIV displacement {field} must be positive"),
            Self::InvalidUncertainty { field } => write!(f, "PIV displacement {field} uncertainty must be finite and non-negative"),
            Self::NonPositiveDefinitePeakHessian => write!(f, "correlation peak negative-log-likelihood Hessian must be positive definite"),
            Self::NonFiniteDerivedValue => write!(f, "PIV displacement uncertainty produced a non-finite derived value"),
            Self::CalibrationReceiptRequired => write!(f, "calibration gate cannot pass without a non-empty external calibration receipt"),
        }
    }
}

impl Error for PivDisplacementError {}

fn validate_input(input: &PivDisplacementInput) -> Result<(), PivDisplacementError> {
    for (field, value) in [
        ("displacement_x", input.displacement_px[0]),
        ("displacement_y", input.displacement_px[1]),
        ("hessian_xx", input.negative_log_likelihood_hessian_px_inverse_squared.xx),
        ("hessian_xy", input.negative_log_likelihood_hessian_px_inverse_squared.xy),
        ("hessian_yy", input.negative_log_likelihood_hessian_px_inverse_squared.yy),
        ("peak_to_sidelobe_ratio", input.peak_diagnostics.peak_to_sidelobe_ratio),
        ("minimum_peak_to_sidelobe_ratio", input.peak_diagnostics.minimum_peak_to_sidelobe_ratio),
        ("fit_residual_rms", input.peak_diagnostics.fit_residual_rms),
        ("maximum_fit_residual_rms", input.peak_diagnostics.maximum_fit_residual_rms),
        ("metres_per_pixel", input.metres_per_pixel),
        ("pulse_separation_seconds", input.pulse_separation_seconds),
    ] {
        if !value.is_finite() {
            return Err(PivDisplacementError::NonFiniteInput { field });
        }
    }
    for (field, value) in [
        ("metres_per_pixel", input.metres_per_pixel),
        ("pulse_separation_seconds", input.pulse_separation_seconds),
        ("minimum_peak_to_sidelobe_ratio", input.peak_diagnostics.minimum_peak_to_sidelobe_ratio),
    ] {
        if value <= 0.0 {
            return Err(PivDisplacementError::InvalidPositiveInput { field });
        }
    }
    for (field, value) in [
        ("metres_per_pixel_standard_uncertainty", input.metres_per_pixel_standard_uncertainty),
        ("pulse_separation_standard_uncertainty_seconds", input.pulse_separation_standard_uncertainty_seconds),
        ("maximum_fit_residual_rms", input.peak_diagnostics.maximum_fit_residual_rms),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(PivDisplacementError::InvalidUncertainty { field });
        }
    }
    if input.calibration_coverage_gate_passed
        && input.calibration_receipt_id.as_deref().is_none_or(|id| id.trim().is_empty())
    {
        return Err(PivDisplacementError::CalibrationReceiptRequired);
    }
    Ok(())
}

fn assess_peak(
    diagnostics: CorrelationPeakDiagnostics,
) -> PivPeakDisposition {
    if !diagnostics.unique_peak {
        PivPeakDisposition::RejectedNonUniquePeak
    } else if diagnostics.peak_to_sidelobe_ratio < diagnostics.minimum_peak_to_sidelobe_ratio {
        PivPeakDisposition::RejectedPeakToSidelobeRatio
    } else if diagnostics.fit_residual_rms > diagnostics.maximum_fit_residual_rms {
        PivPeakDisposition::RejectedPeakFitResidual
    } else {
        PivPeakDisposition::AcceptedForLocalCovariance
    }
}

fn invert_positive_definite_2x2(
    hessian: SymmetricMatrix2,
) -> Result<SymmetricMatrix2, PivDisplacementError> {
    let determinant = hessian.xx.mul_add(hessian.yy, -hessian.xy * hessian.xy);
    if !determinant.is_finite() || hessian.xx <= 0.0 || hessian.yy <= 0.0 || determinant <= 0.0 {
        return Err(PivDisplacementError::NonPositiveDefinitePeakHessian);
    }
    let covariance = SymmetricMatrix2 {
        xx: hessian.yy / determinant,
        xy: -hessian.xy / determinant,
        yy: hessian.xx / determinant,
    };
    if !covariance.xx.is_finite() || !covariance.xy.is_finite() || !covariance.yy.is_finite() {
        return Err(PivDisplacementError::NonFiniteDerivedValue);
    }
    Ok(covariance)
}

/// Local Laplace covariance and first-order propagation to velocity.
///
/// Let d=[dx,dy], s=metres_per_pixel and t=pulse separation. For v=d*s/t,
/// the delta method includes both displacement covariance and shared
/// calibration/timing uncertainty. Calibration and timing uncertainty are
/// treated as independent of displacement and of each other in this bounded
/// reference; a full covariance receipt is future work.
pub fn estimate_piv_displacement_uncertainty(
    input: &PivDisplacementInput,
) -> Result<PivDisplacementUncertaintyReport, PivDisplacementError> {
    validate_input(input)?;
    let peak_disposition = assess_peak(input.peak_diagnostics);
    if peak_disposition != PivPeakDisposition::AcceptedForLocalCovariance {
        return Ok(PivDisplacementUncertaintyReport {
            peak_disposition,
            uncertainty_status: PivUncertaintyStatus::UnusablePeak,
            displacement_covariance_px_squared: None,
            velocity_m_s: None,
            velocity_covariance_m2_s2: None,
            velocity_standard_uncertainty_m_s: None,
            correlation_velocity_uncertainty: None,
            calibration_receipt_id: input.calibration_receipt_id.clone(),
            calibration_coverage_gate_passed: input.calibration_coverage_gate_passed,
            limitation: "peak rejected; no displacement covariance or velocity uncertainty emitted",
        });
    }

    let displacement_covariance =
        invert_positive_definite_2x2(input.negative_log_likelihood_hessian_px_inverse_squared)?;
    let scale = input.metres_per_pixel;
    let dt = input.pulse_separation_seconds;
    let dx = input.displacement_px[0];
    let dy = input.displacement_px[1];
    let velocity = [dx * scale / dt, dy * scale / dt];

    let scale_variance = input.metres_per_pixel_standard_uncertainty.powi(2);
    let time_variance = input.pulse_separation_standard_uncertainty_seconds.powi(2);
    let scale_over_time = scale / dt;
    let scale_over_time_squared = scale_over_time * scale_over_time;
    let time_derivative_squared = scale * scale / dt.powi(4);

    let var_x = scale_over_time_squared * displacement_covariance.xx
        + (dx / dt).powi(2) * scale_variance
        + dx * dx * time_derivative_squared * time_variance;
    let var_y = scale_over_time_squared * displacement_covariance.yy
        + (dy / dt).powi(2) * scale_variance
        + dy * dy * time_derivative_squared * time_variance;
    let covariance_xy = scale_over_time_squared * displacement_covariance.xy
        + (dx * dy / (dt * dt)) * scale_variance
        + (dx * dy * time_derivative_squared) * time_variance;

    if !velocity.iter().all(|value| value.is_finite())
        || !var_x.is_finite()
        || !var_y.is_finite()
        || !covariance_xy.is_finite()
        || var_x < 0.0
        || var_y < 0.0
    {
        return Err(PivDisplacementError::NonFiniteDerivedValue);
    }
    let velocity_covariance = SymmetricMatrix2 {
        xx: var_x,
        xy: covariance_xy,
        yy: var_y,
    };
    let std_uncertainty = [var_x.sqrt(), var_y.sqrt()];
    let correlation = if var_x > 0.0 && var_y > 0.0 {
        Some(covariance_xy / (var_x * var_y).sqrt())
    } else {
        None
    };
    if correlation.is_some_and(|value| !value.is_finite() || value.abs() > 1.0 + 1.0e-9) {
        return Err(PivDisplacementError::NonFiniteDerivedValue);
    }

    let uncertainty_status = if input.calibration_coverage_gate_passed
        && input.calibration_receipt_id.as_deref().is_some_and(|id| !id.trim().is_empty())
    {
        PivUncertaintyStatus::CalibrationReceiptAccepted
    } else {
        PivUncertaintyStatus::LocalApproximationOnly
    };
    Ok(PivDisplacementUncertaintyReport {
        peak_disposition,
        uncertainty_status,
        displacement_covariance_px_squared: Some(displacement_covariance),
        velocity_m_s: Some(velocity),
        velocity_covariance_m2_s2: Some(velocity_covariance),
        velocity_standard_uncertainty_m_s: Some(std_uncertainty),
        correlation_velocity_uncertainty: correlation,
        calibration_receipt_id: input.calibration_receipt_id.clone(),
        calibration_coverage_gate_passed: input.calibration_coverage_gate_passed,
        limitation: "local Hessian/Laplace approximation; not calibrated without a validated empirical coverage receipt",
    })
}

fn manufactured_input() -> PivDisplacementInput {
    PivDisplacementInput {
        displacement_px: [10.0, -5.0],
        negative_log_likelihood_hessian_px_inverse_squared: SymmetricMatrix2 {
            xx: 4.0,
            xy: 1.0,
            yy: 9.0,
        },
        peak_diagnostics: CorrelationPeakDiagnostics {
            peak_to_sidelobe_ratio: 2.5,
            minimum_peak_to_sidelobe_ratio: 1.5,
            fit_residual_rms: 0.02,
            maximum_fit_residual_rms: 0.10,
            unique_peak: true,
        },
        metres_per_pixel: 1.0e-4,
        metres_per_pixel_standard_uncertainty: 1.0e-7,
        pulse_separation_seconds: 1.0e-4,
        pulse_separation_standard_uncertainty_seconds: 1.0e-7,
        calibration_receipt_id: None,
        calibration_coverage_gate_passed: false,
    }
}

pub fn reference_self_check() -> Result<(), PivDisplacementError> {
    let report = estimate_piv_displacement_uncertainty(&manufactured_input())?;
    if report.peak_disposition != PivPeakDisposition::AcceptedForLocalCovariance
        || report.uncertainty_status != PivUncertaintyStatus::LocalApproximationOnly
        || report.displacement_covariance_px_squared.is_none()
        || report.velocity_covariance_m2_s2.is_none()
        || report.velocity_standard_uncertainty_m_s.is_none()
    {
        return Err(PivDisplacementError::NonFiniteDerivedValue);
    }
    let mut rejected = manufactured_input();
    rejected.peak_diagnostics.unique_peak = false;
    let report = estimate_piv_displacement_uncertainty(&rejected)?;
    if report.uncertainty_status != PivUncertaintyStatus::UnusablePeak
        || report.velocity_covariance_m2_s2.is_some()
    {
        return Err(PivDisplacementError::NonFiniteDerivedValue);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_definite_hessian_yields_local_displacement_covariance() {
        let report = estimate_piv_displacement_uncertainty(&manufactured_input()).unwrap();
        let covariance = report.displacement_covariance_px_squared.unwrap();
        let determinant = 4.0 * 9.0 - 1.0;
        assert!((covariance.xx - 9.0 / determinant).abs() < 1.0e-12);
        assert!((covariance.xy + 1.0 / determinant).abs() < 1.0e-12);
        assert!((covariance.yy - 4.0 / determinant).abs() < 1.0e-12);
    }

    #[test]
    fn velocity_conversion_propagates_displacement_scale_and_timing_uncertainty() {
        let report = estimate_piv_displacement_uncertainty(&manufactured_input()).unwrap();
        let velocity = report.velocity_m_s.unwrap();
        assert!((velocity[0] - 10.0).abs() < 1.0e-12);
        assert!((velocity[1] + 5.0).abs() < 1.0e-12);
        let covariance = report.velocity_covariance_m2_s2.unwrap();
        assert!(covariance.xx > 0.0 && covariance.yy > 0.0);
        assert!(covariance.xy.is_finite());
    }

    #[test]
    fn calibration_and_timing_uncertainty_create_cross_component_covariance() {
        let mut input = manufactured_input();
        input.negative_log_likelihood_hessian_px_inverse_squared.xy = 0.0;
        let report = estimate_piv_displacement_uncertainty(&input).unwrap();
        assert!(report.velocity_covariance_m2_s2.unwrap().xy < 0.0);
    }

    #[test]
    fn non_unique_correlation_peak_is_rejected_without_covariance() {
        let mut input = manufactured_input();
        input.peak_diagnostics.unique_peak = false;
        let report = estimate_piv_displacement_uncertainty(&input).unwrap();
        assert_eq!(report.peak_disposition, PivPeakDisposition::RejectedNonUniquePeak);
        assert!(report.velocity_covariance_m2_s2.is_none());
    }

    #[test]
    fn low_peak_to_sidelobe_ratio_is_rejected() {
        let mut input = manufactured_input();
        input.peak_diagnostics.peak_to_sidelobe_ratio = 1.1;
        let report = estimate_piv_displacement_uncertainty(&input).unwrap();
        assert_eq!(report.peak_disposition, PivPeakDisposition::RejectedPeakToSidelobeRatio);
    }

    #[test]
    fn poor_peak_fit_residual_is_rejected() {
        let mut input = manufactured_input();
        input.peak_diagnostics.fit_residual_rms = 0.5;
        let report = estimate_piv_displacement_uncertainty(&input).unwrap();
        assert_eq!(report.peak_disposition, PivPeakDisposition::RejectedPeakFitResidual);
    }

    #[test]
    fn non_positive_definite_hessian_is_rejected() {
        let mut input = manufactured_input();
        input.negative_log_likelihood_hessian_px_inverse_squared = SymmetricMatrix2 {
            xx: 1.0, xy: 2.0, yy: 1.0,
        };
        assert_eq!(
            estimate_piv_displacement_uncertainty(&input).unwrap_err(),
            PivDisplacementError::NonPositiveDefinitePeakHessian
        );
    }

    #[test]
    fn empirical_calibration_receipt_is_required_for_calibrated_status() {
        let mut input = manufactured_input();
        input.calibration_receipt_id = Some("coverage-receipt-v1".to_owned());
        input.calibration_coverage_gate_passed = true;
        let report = estimate_piv_displacement_uncertainty(&input).unwrap();
        assert_eq!(report.uncertainty_status, PivUncertaintyStatus::CalibrationReceiptAccepted);
    }

    #[test]
    fn local_covariance_without_coverage_receipt_is_not_called_calibrated() {
        let report = estimate_piv_displacement_uncertainty(&manufactured_input()).unwrap();
        assert_eq!(report.uncertainty_status, PivUncertaintyStatus::LocalApproximationOnly);
    }

    #[test]
    fn calibration_gate_without_receipt_is_rejected() {
        let mut input = manufactured_input();
        input.calibration_coverage_gate_passed = true;
        assert_eq!(
            estimate_piv_displacement_uncertainty(&input).unwrap_err(),
            PivDisplacementError::CalibrationReceiptRequired
        );
    }
}
