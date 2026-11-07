// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Uncertainty-aware numerical refinement diagnostics.
//!
//! Three-level Richardson extrapolation estimates the asymptotic order and
//! fine-level discretization error for a scalar observable computed on
//! geometrically refined resolutions. It requires monotone convergence; if
//! the sequence is oscillatory, non-geometric, degenerate, or non-asymptotic,
//! the routine returns a typed error rather than manufacturing an error bar.
//!
//! The estimate covers only the numerical refinement component represented by
//! the chosen resolution parameter. It is not a complete uncertainty budget:
//! model-form error, parameter uncertainty, experimental measurement
//! uncertainty, stochastic variability, and implementation defects remain
//! separate evidence sources.
//!
//! For temporal refinement, the resolution is a timestep in seconds. For
//! spatial refinement it may be a characteristic grid spacing in metres. The
//! three samples must share the same observable, units, equations, solver
//! profile, initial/boundary conditions and output time.

/// Safety factor applied to the extrapolated fine-level discretization error.
///
/// This is the conventional 1.25 factor used for a three-level asymptotic
/// refinement estimate. It is not a probability or a 95% confidence interval.
pub const RICHARDSON_SAFETY_FACTOR: f64 = 1.25;

/// One scalar observable evaluated at a declared numerical resolution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RefinementSample {
    /// Resolution parameter (for example timestep in seconds or grid spacing
    /// in metres). Coarse > medium > fine.
    pub resolution: f64,
    /// The same scalar observable at this resolution, in consistent units.
    pub quantity: f64,
}

/// Failure modes for three-level refinement analysis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RichardsonError {
    /// Resolution or observable input is NaN or infinity.
    NonFiniteInput,
    /// A resolution is not strictly positive.
    NonPositiveResolution,
    /// Resolution levels are not strictly ordered coarse > medium > fine.
    InvalidResolutionOrder,
    /// The two successive refinement ratios are not equal within tolerance.
    NonGeometricRefinement,
    /// The observable differences do not show monotone convergence.
    NonMonotoneOrOscillatory,
    /// The estimated order is zero, negative or non-finite.
    InvalidObservedOrder,
    /// Extrapolation arithmetic overflowed or became non-finite.
    ArithmeticOverflow,
}

/// Richardson estimate from three geometrically refined samples.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RichardsonEstimate {
    /// Common refinement ratio, coarse/medium = medium/fine.
    pub refinement_ratio: f64,
    /// Observed asymptotic order estimated from the three samples.
    pub observed_order: f64,
    /// Extrapolated zero-resolution-limit observable.
    pub extrapolated_value: f64,
    /// Estimated absolute error of the fine-level result, in the observable's
    /// own units, after applying the safety factor.
    pub fine_level_absolute_error: f64,
    /// Estimated relative fine-level error; None when the extrapolated value
    /// is too close to zero for a stable relative normalization.
    pub fine_level_relative_error: Option<f64>,
    /// Safety factor used for the reported error estimate.
    pub safety_factor: f64,
}

/// Estimate observed order and fine-level discretization error.
///
/// Inputs must represent the same scalar observable at three geometrically
/// refined resolutions. The refinement ratios must agree within 1e-6 relative.
/// The method assumes a monotone asymptotic error model:
///
/// `Q(h) = Q_exact + C * h^p`.
///
/// Non-monotone/oscillatory sequences are rejected. A caller should refine
/// further or investigate solver regimes rather than interpreting a failed
/// estimate as zero error.
pub fn richardson_three_level(
    coarse: RefinementSample,
    medium: RefinementSample,
    fine: RefinementSample,
) -> Result<RichardsonEstimate, RichardsonError> {
    let inputs = [
        coarse.resolution,
        medium.resolution,
        fine.resolution,
        coarse.quantity,
        medium.quantity,
        fine.quantity,
    ];
    if inputs.iter().any(|value| !value.is_finite()) {
        return Err(RichardsonError::NonFiniteInput);
    }

    if coarse.resolution <= 0.0 || medium.resolution <= 0.0 || fine.resolution <= 0.0 {
        return Err(RichardsonError::NonPositiveResolution);
    }
    if !(coarse.resolution > medium.resolution && medium.resolution > fine.resolution) {
        return Err(RichardsonError::InvalidResolutionOrder);
    }

    let coarse_ratio = coarse.resolution / medium.resolution;
    let fine_ratio = medium.resolution / fine.resolution;
    if !coarse_ratio.is_finite() || !fine_ratio.is_finite() {
        return Err(RichardsonError::ArithmeticOverflow);
    }
    let ratio_scale = coarse_ratio.abs().max(fine_ratio.abs()).max(1.0);
    if (coarse_ratio - fine_ratio).abs() > 1e-6 * ratio_scale || coarse_ratio <= 1.0 {
        return Err(RichardsonError::NonGeometricRefinement);
    }

    let coarse_delta = coarse.quantity - medium.quantity;
    let fine_delta = medium.quantity - fine.quantity;
    if !coarse_delta.is_finite() || !fine_delta.is_finite() {
        return Err(RichardsonError::ArithmeticOverflow);
    }
    if coarse_delta == 0.0
        || fine_delta == 0.0
        || coarse_delta.signum() != fine_delta.signum()
    {
        return Err(RichardsonError::NonMonotoneOrOscillatory);
    }

    let difference_ratio = coarse_delta.abs() / fine_delta.abs();
    let observed_order = difference_ratio.ln() / coarse_ratio.ln();
    if !observed_order.is_finite() || observed_order <= 0.0 {
        return Err(RichardsonError::InvalidObservedOrder);
    }

    let refinement_power = coarse_ratio.powf(observed_order);
    let denominator = refinement_power - 1.0;
    if !refinement_power.is_finite() || !denominator.is_finite() || denominator <= 0.0 {
        return Err(RichardsonError::ArithmeticOverflow);
    }

    let extrapolated_value = fine.quantity + (fine.quantity - medium.quantity) / denominator;
    let raw_fine_error = (extrapolated_value - fine.quantity).abs();
    let fine_level_absolute_error = RICHARDSON_SAFETY_FACTOR * raw_fine_error;
    if !extrapolated_value.is_finite() || !fine_level_absolute_error.is_finite() {
        return Err(RichardsonError::ArithmeticOverflow);
    }

    let fine_level_relative_error = if extrapolated_value.abs() > f64::EPSILON {
        Some(fine_level_absolute_error / extrapolated_value.abs())
    } else {
        None
    };

    Ok(RichardsonEstimate {
        refinement_ratio: coarse_ratio,
        observed_order,
        extrapolated_value,
        fine_level_absolute_error,
        fine_level_relative_error,
        safety_factor: RICHARDSON_SAFETY_FACTOR,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_first_order_limit_and_observed_order() {
        // Q(h) = 2 + 3h, so p=1 and Q_exact=2 exactly.
        let estimate = richardson_three_level(
            RefinementSample {
                resolution: 0.1,
                quantity: 2.3,
            },
            RefinementSample {
                resolution: 0.05,
                quantity: 2.15,
            },
            RefinementSample {
                resolution: 0.025,
                quantity: 2.075,
            },
        )
        .unwrap();

        assert!((estimate.observed_order - 1.0).abs() < 1e-12);
        assert!((estimate.extrapolated_value - 2.0).abs() < 1e-12);
        assert!((estimate.fine_level_absolute_error - 0.09375).abs() < 1e-12);
        assert_eq!(estimate.safety_factor, RICHARDSON_SAFETY_FACTOR);
    }

    #[test]
    fn recovers_second_order_limit_and_observed_order() {
        // Q(h) = 1.2 + 4h^2, so p=2 and Q_exact=1.2 exactly.
        let estimate = richardson_three_level(
            RefinementSample {
                resolution: 0.1,
                quantity: 1.24,
            },
            RefinementSample {
                resolution: 0.05,
                quantity: 1.21,
            },
            RefinementSample {
                resolution: 0.025,
                quantity: 1.2025,
            },
        )
        .unwrap();

        assert!((estimate.observed_order - 2.0).abs() < 1e-12);
        assert!((estimate.extrapolated_value - 1.2).abs() < 1e-12);
        assert!((estimate.fine_level_absolute_error - 0.003125).abs() < 1e-12);
    }

    #[test]
    fn rejects_non_geometric_or_oscillatory_refinement() {
        let coarse = RefinementSample {
            resolution: 0.1,
            quantity: 1.3,
        };
        let medium = RefinementSample {
            resolution: 0.06,
            quantity: 1.15,
        };
        let fine = RefinementSample {
            resolution: 0.025,
            quantity: 1.075,
        };
        assert_eq!(
            richardson_three_level(coarse, medium, fine),
            Err(RichardsonError::NonGeometricRefinement)
        );

        assert_eq!(
            richardson_three_level(
                RefinementSample {
                    resolution: 0.1,
                    quantity: 1.0,
                },
                RefinementSample {
                    resolution: 0.05,
                    quantity: 0.9,
                },
                RefinementSample {
                    resolution: 0.025,
                    quantity: 0.95,
                },
            ),
            Err(RichardsonError::NonMonotoneOrOscillatory)
        );
    }

    #[test]
    fn rejects_invalid_input_and_resolution_order() {
        assert_eq!(
            richardson_three_level(
                RefinementSample {
                    resolution: 0.1,
                    quantity: f64::NAN,
                },
                RefinementSample {
                    resolution: 0.05,
                    quantity: 1.0,
                },
                RefinementSample {
                    resolution: 0.025,
                    quantity: 1.1,
                },
            ),
            Err(RichardsonError::NonFiniteInput)
        );

        assert_eq!(
            richardson_three_level(
                RefinementSample {
                    resolution: 0.05,
                    quantity: 1.0,
                },
                RefinementSample {
                    resolution: 0.1,
                    quantity: 1.1,
                },
                RefinementSample {
                    resolution: 0.025,
                    quantity: 1.2,
                },
            ),
            Err(RichardsonError::InvalidResolutionOrder)
        );
    }

    #[test]
    fn relative_error_is_omitted_for_zero_extrapolated_quantity() {
        // Q(h)=h^2 gives Q_exact=0; absolute error is meaningful but relative
        // error is undefined and must remain absent.
        let estimate = richardson_three_level(
            RefinementSample {
                resolution: 0.1,
                quantity: 0.01,
            },
            RefinementSample {
                resolution: 0.05,
                quantity: 0.0025,
            },
            RefinementSample {
                resolution: 0.025,
                quantity: 0.000625,
            },
        )
        .unwrap();
        assert!(estimate.extrapolated_value.abs() < 1e-15);
        assert!(estimate.fine_level_absolute_error.is_finite());
        assert_eq!(estimate.fine_level_relative_error, None);
    }
}
