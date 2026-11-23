// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Small-hard-body-radius asymptotic oracle for the NASA 2D-Nc method.
//!
//! Hall, Baars & Casali (2023), Eq. 13, derive the small-HBR limit of 2D-Nc:
//!
//! Pc ~= (R^2 * |v_T| * w_(0,T) / (2 * sqrt(|Abar_Tc|)))
//!      * exp(-M_(0,T) / 2).
//!
//! The approximation requires R to be much smaller than the smallest
//! covariance length scale, negligible velocity uncertainty, and negligible
//! position-velocity cross-correlation. This module requires those conditions
//! to be explicitly declared by the caller; it cannot verify them.
//!
//! This is a software known-answer oracle for the stated asymptotic regime.
//! It is not the full 2D-Nc method, not a collision-probability certification,
//! and not a substitute for NASA's escalation to 3D-Nc or Monte Carlo when the
//! approximation's assumptions are not supported.
//!
//! Reference: Hall, Baars & Casali (2023), "A Multistep Probability of
//! Collision Computational Algorithm", Eq. (13), pp. 8:
//! https://ntrs.nasa.gov/api/citations/20230010175/downloads/Hall_Baars_Casali_ASC_2023_08_13_PcMultiStep_Paper.pdf

use crate::KernelError;

/// Caller declaration that the small-HBR Eq. 13 asymptotic assumptions hold.
///
/// These are explicit claims, not conditions independently proven by the
/// numerical kernel. A false or unsupported declaration invalidates the use
/// of the asymptotic approximation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmallHbrRegimeDeclaration {
    /// Caller asserts that HBR is much smaller than every covariance length
    /// scale: R << min_i(sqrt(lambda_i)).
    pub small_hbr_vs_covariance_scale: bool,
    /// Caller asserts that velocity uncertainty is negligible for this case.
    pub negligible_velocity_uncertainty: bool,
    /// Caller asserts that position-velocity cross-covariance is negligible.
    pub negligible_position_velocity_cross_covariance: bool,
}

/// Inputs to the small-HBR 2D-Nc asymptotic known-answer oracle.
///
/// All spatial values must use consistent length units (the suggested unit is
/// metres), time values must use seconds, and the covariance determinant
/// square-root must therefore have units of length cubed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmallHbrNcApproximationInputs {
    /// Combined hard-body radius R.
    pub hard_body_radius: f64,
    /// Magnitude of mean relative velocity at the reference time T.
    pub mean_relative_speed_at_t: f64,
    /// Small-HBR temporal width w_(0,T), in seconds.
    ///
    /// In Hall et al. Eq. (11), this is sqrt(2 / M_double_dot_(0,T)).
    pub time_width_at_zero_hbr_seconds: f64,
    /// sqrt(|det(Abar_Tc)|), with units of length cubed.
    pub sqrt_abs_position_covariance_determinant_at_tca: f64,
    /// Modified Mahalanobis distance M_(0,T) at the centre of the collision
    /// sphere and reference time T.
    pub modified_mahalanobis_distance_at_zero_hbr: f64,
    /// Explicit caller declaration of the approximation assumptions.
    pub regime: SmallHbrRegimeDeclaration,
}

/// Small-HBR approximation output, including log-domain tail information.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmallHbrNcApproximationReport {
    /// Natural logarithm of the approximate collision probability.
    ///
    /// This remains useful when the f64 probability underflows to zero.
    pub log_probability: f64,
    /// Approximate collision probability in f64. May be zero only when the
    /// report's underflow indicator is true.
    pub probability: f64,
    /// True when exponentiating the finite log probability underflowed to zero.
    pub probability_underflowed: bool,
    /// Formula revision / reference label.
    pub formula: &'static str,
}

/// Evaluate the small-HBR asymptotic known-answer formula from Hall et al. Eq. 13.
///
/// The calculation is performed in the log domain:
///
/// log(Pc) = 2 ln(R) + ln(|v_T|) + ln(w_(0,T)) - ln(2)
///           - ln(sqrt(|Abar_Tc|)) - M_(0,T)/2.
///
/// The caller must supply mutually consistent units and explicitly declare all
/// small-HBR assumptions. The function rejects a result above one rather than
/// silently clipping an asymptotic approximation outside its useful regime.
///
/// This function does not compute the effective TCA covariance, MMD, reference
/// time, or temporal width. Those are upstream 2D-Nc quantities and must be
/// obtained independently by the caller.
pub fn small_hbr_nc_approximation(
    inputs: SmallHbrNcApproximationInputs,
) -> Result<SmallHbrNcApproximationReport, KernelError> {
    if !inputs.regime.small_hbr_vs_covariance_scale
        || !inputs.regime.negligible_velocity_uncertainty
        || !inputs.regime.negligible_position_velocity_cross_covariance
    {
        return Err(KernelError::OutsideSmallHbrAssumptions);
    }

    let positive_inputs = [
        inputs.hard_body_radius,
        inputs.mean_relative_speed_at_t,
        inputs.time_width_at_zero_hbr_seconds,
        inputs.sqrt_abs_position_covariance_determinant_at_tca,
    ];
    if positive_inputs.iter().any(|value| !value.is_finite()) {
        return Err(KernelError::NonFiniteValue);
    }
    if positive_inputs.iter().any(|value| *value <= 0.0) {
        return Err(KernelError::InvalidSmallHbrInput);
    }
    if !inputs.modified_mahalanobis_distance_at_zero_hbr.is_finite()
    {
        return Err(KernelError::NonFiniteValue);
    }

    let log_probability = 2.0 * inputs.hard_body_radius.ln()
        + inputs.mean_relative_speed_at_t.ln()
        + inputs.time_width_at_zero_hbr_seconds.ln()
        - 2.0_f64.ln()
        - inputs.sqrt_abs_position_covariance_determinant_at_tca.ln()
        - 0.5 * inputs.modified_mahalanobis_distance_at_zero_hbr;

    if !log_probability.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    if log_probability > 0.0 {
        return Err(KernelError::ApproximationExceedsUnitProbability);
    }

    let probability = log_probability.exp();
    if !probability.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }

    Ok(SmallHbrNcApproximationReport {
        log_probability,
        probability,
        probability_underflowed: probability == 0.0,
        formula: "Hall-Baars-Casali-2023-Eq13-small-HBR",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regime() -> SmallHbrRegimeDeclaration {
        SmallHbrRegimeDeclaration {
            small_hbr_vs_covariance_scale: true,
            negligible_velocity_uncertainty: true,
            negligible_position_velocity_cross_covariance: true,
        }
    }

    fn inputs() -> SmallHbrNcApproximationInputs {
        SmallHbrNcApproximationInputs {
            hard_body_radius: 0.01,
            mean_relative_speed_at_t: 7_500.0,
            time_width_at_zero_hbr_seconds: 10.0,
            sqrt_abs_position_covariance_determinant_at_tca: 1.0e9,
            modified_mahalanobis_distance_at_zero_hbr: 25.0,
            regime: regime(),
        }
    }

    #[test]
    fn equation_thirteen_known_answer_matches_direct_formula() {
        let report = small_hbr_nc_approximation(inputs()).expect("valid small-HBR regime");
        let direct = (0.01_f64.powi(2) * 7_500.0 * 10.0 / (2.0 * 1.0e9)) * (-25.0_f64 / 2.0).exp();
        assert!((report.probability - direct).abs() <= direct * 1.0e-12);
        assert!((report.log_probability - direct.ln()).abs() <= 1.0e-12);
        // Independent high-precision reference value for the fixed inputs.
        assert!((report.probability - 1.397_494_939_529_503e-14).abs() <= 2.0e-27);
        assert!((report.log_probability - (-31.901_509_996_964_09)).abs() <= 2.0e-14);
        assert!(!report.probability_underflowed);
        assert_eq!(report.formula, "Hall-Baars-Casali-2023-Eq13-small-HBR");
    }

    #[test]
    fn coordinate_length_rescaling_leaves_dimensionless_probability_invariant() {
        let baseline = small_hbr_nc_approximation(inputs()).expect("baseline inputs");

        let mut scaled = inputs();
        let length_scale = 1_000.0;
        scaled.hard_body_radius *= length_scale;
        scaled.mean_relative_speed_at_t *= length_scale;
        scaled.sqrt_abs_position_covariance_determinant_at_tca *= length_scale.powi(3);

        let transformed = small_hbr_nc_approximation(scaled).expect("scaled inputs");
        assert!((baseline.log_probability - transformed.log_probability).abs() < 1.0e-12);
    }

    #[test]
    fn extreme_rare_event_retains_log_probability_when_f64_underflows() {
        let mut rare = inputs();
        rare.modified_mahalanobis_distance_at_zero_hbr = 2_000.0;
        let report = small_hbr_nc_approximation(rare).expect("finite log probability");
        assert!(report.log_probability.is_finite());
        assert_eq!(report.probability, 0.0);
        assert!(report.probability_underflowed);
    }

    #[test]
    fn unsupported_asymptotic_assumptions_are_rejected() {
        let mut invalid = inputs();
        invalid.regime.negligible_velocity_uncertainty = false;
        assert_eq!(
            small_hbr_nc_approximation(invalid),
            Err(KernelError::OutsideSmallHbrAssumptions)
        );

        let mut invalid = inputs();
        invalid.regime.small_hbr_vs_covariance_scale = false;
        assert_eq!(
            small_hbr_nc_approximation(invalid),
            Err(KernelError::OutsideSmallHbrAssumptions)
        );

        let mut invalid = inputs();
        invalid.regime.negligible_position_velocity_cross_covariance = false;
        assert_eq!(
            small_hbr_nc_approximation(invalid),
            Err(KernelError::OutsideSmallHbrAssumptions)
        );
    }

    #[test]
    fn invalid_positive_inputs_fail_closed() {
        let mut invalid = inputs();
        invalid.hard_body_radius = 0.0;
        assert_eq!(
            small_hbr_nc_approximation(invalid),
            Err(KernelError::InvalidSmallHbrInput)
        );

        let mut invalid = inputs();
        invalid.mean_relative_speed_at_t = f64::NAN;
        assert_eq!(
            small_hbr_nc_approximation(invalid),
            Err(KernelError::NonFiniteValue)
        );

        let mut invalid = inputs();
        invalid.sqrt_abs_position_covariance_determinant_at_tca = -1.0;
        assert_eq!(
            small_hbr_nc_approximation(invalid),
            Err(KernelError::InvalidSmallHbrInput)
        );
    }

    #[test]
    fn asymptotic_probability_above_one_is_rejected_not_clipped() {
        let mut invalid = inputs();
        invalid.hard_body_radius = 1.0e8;
        assert_eq!(
            small_hbr_nc_approximation(invalid),
            Err(KernelError::ApproximationExceedsUnitProbability)
        );
    }
}
