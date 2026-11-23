// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Stable scalar expectations for truncated one-dimensional Gaussian variables.
//!
//! For X ~ N(mean, standard_deviation²), this module evaluates
//! E[max(-X, 0)] and preserves the logarithm when the f64 value underflows.
//!
//! The moderate-mean path uses the pure-Rust libm erfc routine. The large
//! positive tail uses an asymptotic Mills-ratio expansion to avoid cancellation.
//! This is a scalar primitive, not the full Ncdot integrand.

use crate::KernelError;
use core::f64::consts::PI;

const SQRT_TWO: f64 = core::f64::consts::SQRT_2;
const LOG_SQRT_TWO_PI: f64 = 0.5 * (2.0 * PI).ln();
const INV_SQRT_TWO_PI: f64 = 1.0 / (2.0 * PI).sqrt();
const POSITIVE_TAIL_THRESHOLD: f64 = 12.0;

/// Numerical branch used for the Gaussian positive-part expectation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaussianPositivePartBranch {
    /// Direct erfc evaluation where cancellation is controlled.
    DirectErfc,
    /// Negative-mean limit; the expectation differs from -mean only by an
    /// exponentially small tail beyond the declared branch threshold.
    NegativeMeanTail,
    /// Positive-mean asymptotic Mills-ratio expansion.
    PositiveMeanAsymptotic,
    /// Exact deterministic limit when the standard deviation is zero.
    DeterministicLimit,
}

/// Report for E[max(-X, 0)] where X is a one-dimensional Gaussian.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaussianPositivePartReport {
    /// Gaussian mean.
    pub mean: f64,
    /// Gaussian standard deviation.
    pub standard_deviation: f64,
    /// E[max(-X, 0)] as an f64 value. This may underflow to zero.
    pub expectation: f64,
    /// Natural logarithm of the expectation. Negative infinity denotes an
    /// exactly zero deterministic result; a finite very negative value remains
    /// available when the f64 expectation underflows.
    pub log_expectation: f64,
    /// True when a strictly positive mathematical expectation underflowed
    /// to zero in the f64 value field.
    pub underflowed: bool,
    /// Numerical branch used.
    pub branch: GaussianPositivePartBranch,
}

/// Evaluate E[max(-X, 0)] for X ~ N(mean, standard_deviation²).
///
/// With z = mean / standard_deviation and standard deviation > 0:
///
/// E[max(-X, 0)] = sigma * (phi(z) - z * Phi(-z)).
///
/// The zero-standard-deviation case is the exact deterministic limit
/// max(-mean, 0). For z >= 12, an asymptotic Mills-ratio expansion is used:
///
/// phi(z) - z*Phi(-z) = phi(z) * (1/z² - 3/z⁴ + 15/z⁶ - ...).
///
/// The expansion is evaluated in the log domain and truncated after twelve
/// terms. It is not interval arithmetic; accuracy is tested against an
/// independent high-precision reference grid.
pub fn gaussian_negative_part_expectation(
    mean: f64,
    standard_deviation: f64,
) -> Result<GaussianPositivePartReport, KernelError> {
    if !mean.is_finite() || !standard_deviation.is_finite() {
        return Err(KernelError::NonFiniteValue);
    }
    if standard_deviation < 0.0 {
        return Err(KernelError::InvalidNormalScale);
    }

    if standard_deviation == 0.0 {
        let expectation = (-mean).max(0.0);
        let log_expectation = if expectation > 0.0 {
            expectation.ln()
        } else {
            f64::NEG_INFINITY
        };
        return Ok(GaussianPositivePartReport {
            mean,
            standard_deviation,
            expectation,
            log_expectation,
            underflowed: false,
            branch: GaussianPositivePartBranch::DeterministicLimit,
        });
    }

    let z = mean / standard_deviation;
    if z.is_nan() {
        return Err(KernelError::NumericalOverflow);
    }

    if z == f64::NEG_INFINITY || z <= -POSITIVE_TAIL_THRESHOLD {
        let expectation = -mean;
        if !expectation.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
        return Ok(GaussianPositivePartReport {
            mean,
            standard_deviation,
            expectation,
            log_expectation: if expectation > 0.0 {
                expectation.ln()
            } else {
                f64::NEG_INFINITY
            },
            underflowed: false,
            branch: GaussianPositivePartBranch::NegativeMeanTail,
        });
    }

    if z == f64::INFINITY {
        return Ok(GaussianPositivePartReport {
            mean,
            standard_deviation,
            expectation: 0.0,
            log_expectation: f64::NEG_INFINITY,
            underflowed: true,
            branch: GaussianPositivePartBranch::PositiveMeanAsymptotic,
        });
    }

    if z >= POSITIVE_TAIL_THRESHOLD {
        return positive_tail_report(mean, standard_deviation, z);
    }

    let gaussian_density = (-0.5 * z * z).exp() * INV_SQRT_TWO_PI;
    let lower_tail_probability = 0.5 * libm::erfc(z / SQRT_TWO);
    let standardized_expectation = gaussian_density - z * lower_tail_probability;
    if !standardized_expectation.is_finite() || standardized_expectation <= 0.0 {
        return Err(KernelError::NumericalOverflow);
    }

    let log_expectation = standard_deviation.ln() + standardized_expectation.ln();
    let expectation = standard_deviation * standardized_expectation;
    if !log_expectation.is_finite() || !expectation.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }

    Ok(GaussianPositivePartReport {
        mean,
        standard_deviation,
        expectation,
        log_expectation,
        underflowed: expectation == 0.0,
        branch: GaussianPositivePartBranch::DirectErfc,
    })
}

fn positive_tail_report(
    mean: f64,
    standard_deviation: f64,
    z: f64,
) -> Result<GaussianPositivePartReport, KernelError> {
    let inverse_square = 1.0 / (z * z);
    if inverse_square == 0.0 {
        return Ok(GaussianPositivePartReport {
            mean,
            standard_deviation,
            expectation: 0.0,
            log_expectation: f64::NEG_INFINITY,
            underflowed: true,
            branch: GaussianPositivePartBranch::PositiveMeanAsymptotic,
        });
    }

    // h(z) = 1/z² - 3/z⁴ + 15/z⁶ - 105/z⁸ + ...
    // term_k = term_(k-1) * -(2k+1)/z².
    let mut term = inverse_square;
    let mut scaled_mills_remainder = term;
    for k in 1..12 {
        term *= -((2 * k + 1) as f64) * inverse_square;
        scaled_mills_remainder += term;
    }
    if !scaled_mills_remainder.is_finite() || scaled_mills_remainder <= 0.0 {
        return Err(KernelError::NumericalOverflow);
    }

    let squared = z * z;
    if !squared.is_finite() {
        return Ok(GaussianPositivePartReport {
            mean,
            standard_deviation,
            expectation: 0.0,
            log_expectation: f64::NEG_INFINITY,
            underflowed: true,
            branch: GaussianPositivePartBranch::PositiveMeanAsymptotic,
        });
    }

    let log_expectation = standard_deviation.ln()
        - 0.5 * squared
        - LOG_SQRT_TWO_PI
        + scaled_mills_remainder.ln();
    if log_expectation.is_nan() {
        return Err(KernelError::NumericalOverflow);
    }
    let expectation = log_expectation.exp();
    if !expectation.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }

    Ok(GaussianPositivePartReport {
        mean,
        standard_deviation,
        expectation,
        log_expectation,
        underflowed: expectation == 0.0,
        branch: GaussianPositivePartBranch::PositiveMeanAsymptotic,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_relative(actual: f64, expected: f64, tolerance: f64) {
        let relative = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
        assert!(
            relative <= tolerance,
            "actual={actual:.17e}, expected={expected:.17e}, relative={relative:.3e}"
        );
    }

    #[test]
    fn zero_mean_matches_closed_form() {
        let report = gaussian_negative_part_expectation(0.0, 1.0).expect("valid inputs");
        assert_relative(report.expectation, 0.398_942_280_401_432_7, 2.0e-15);
        assert_relative(report.log_expectation, -0.918_938_533_204_672_7, 2.0e-15);
        assert_eq!(report.branch, GaussianPositivePartBranch::DirectErfc);
    }

    #[test]
    fn deterministic_limit_is_exact_for_both_signs() {
        let inward = gaussian_negative_part_expectation(-4.0, 0.0).expect("deterministic inward");
        assert_eq!(inward.expectation, 4.0);
        assert_eq!(inward.log_expectation, 4.0_f64.ln());
        assert_eq!(inward.branch, GaussianPositivePartBranch::DeterministicLimit);

        let outward = gaussian_negative_part_expectation(4.0, 0.0).expect("deterministic outward");
        assert_eq!(outward.expectation, 0.0);
        assert_eq!(outward.log_expectation, f64::NEG_INFINITY);
        assert!(!outward.underflowed);
    }

    #[test]
    fn negative_mean_reference_grid_matches_independent_values() {
        for (z, expected) in [
            (-10.0, 10.000_000_000_000_000),
            (-1.0, 1.083_315_470_587_686_3),
        ] {
            let report = gaussian_negative_part_expectation(z, 1.0).expect("negative mean");
            assert_relative(report.expectation, expected, 2.0e-14);
            assert_eq!(report.branch, GaussianPositivePartBranch::DirectErfc);
        }

        let far_tail = gaussian_negative_part_expectation(-40.0, 1.0)
            .expect("far negative mean");
        assert_eq!(far_tail.expectation, 40.0);
        assert_eq!(far_tail.branch, GaussianPositivePartBranch::NegativeMeanTail);
    }

    #[test]
    fn direct_erfc_positive_mean_reference_grid_matches_independent_values() {
        for (z, expected) in [
            (1.0, 8.331_547_058_768_63e-2),
            (5.0, 5.346_165_533_832_815e-8),
            (8.0, 7.550_262_411_946_499e-17),
            (10.0, 7.474_560_254_589_328e-25),
        ] {
            let report = gaussian_negative_part_expectation(z, 1.0).expect("positive mean");
            assert_relative(report.expectation, expected, 2.0e-12);
            assert_eq!(report.branch, GaussianPositivePartBranch::DirectErfc);
        }
    }

    #[test]
    fn positive_tail_asymptotic_matches_independent_log_references() {
        for (z, expected_log) in [
            (20.0, -206.917_838_509_425_1),
            (40.0, -808.298_568_356_62),
        ] {
            let report = gaussian_negative_part_expectation(z, 1.0).expect("positive tail");
            assert!(
                (report.log_expectation - expected_log).abs() < 2.0e-10,
                "z={z}: log={:.17e}, expected={expected_log:.17e}",
                report.log_expectation
            );
            assert_eq!(report.branch, GaussianPositivePartBranch::PositiveMeanAsymptotic);
        }
    }

    #[test]
    fn positive_tail_underflow_retains_finite_log_expectation() {
        let report = gaussian_negative_part_expectation(40.0, 1.0).expect("finite log tail");
        assert_eq!(report.expectation, 0.0);
        assert!(report.underflowed);
        assert!(report.log_expectation.is_finite());
    }

    #[test]
    fn expectation_scales_linearly_with_standard_deviation() {
        let unit = gaussian_negative_part_expectation(5.0, 1.0).expect("unit scale");
        let scaled = gaussian_negative_part_expectation(10.0, 2.0).expect("scaled inputs");
        assert_relative(scaled.expectation, 2.0 * unit.expectation, 1.0e-14);
        assert_relative(scaled.log_expectation, unit.log_expectation + 2.0_f64.ln(), 1.0e-14);
    }

    #[test]
    fn invalid_inputs_fail_closed() {
        assert_eq!(
            gaussian_negative_part_expectation(f64::NAN, 1.0),
            Err(KernelError::NonFiniteValue)
        );
        assert_eq!(
            gaussian_negative_part_expectation(1.0, f64::INFINITY),
            Err(KernelError::NonFiniteValue)
        );
        assert_eq!(
            gaussian_negative_part_expectation(1.0, -1.0),
            Err(KernelError::InvalidNormalScale)
        );
    }
}
