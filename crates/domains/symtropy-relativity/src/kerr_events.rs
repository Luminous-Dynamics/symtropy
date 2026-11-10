// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Event localization for accepted Kerr Mino-trajectory samples.
//!
//! This module localizes the first sampled outside-to-inside outer-horizon
//! crossing in the cubic-Hermite interpolant formed from accepted endpoint
//! radii and radial Mino velocities. The root bracket is a solver-precision
//! bound for that interpolant only. It is not, by itself, a bound on the
//! difference between the interpolant crossing and the exact geodesic event.
//! That interpolation error must be qualified by trajectory-step refinement.

use core::fmt;

use crate::{
    KerrBlackHole,
    kerr_geodesic::{KerrMinoCoordinateChart, KerrMinoTrajectory},
};

/// A localized outside-to-inside crossing of the future outer horizon.
///
/// The event is a root of the cubic-Hermite interpolant over one accepted
/// integration segment. `parameter_bracket` and `root_solver_error_bound`
/// describe bisection uncertainty for that interpolant; they do not include
/// the interpolation error of the accepted-step trajectory.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrHorizonEvent {
    /// Index of the lower endpoint in `trajectory.points`.
    pub segment_index: usize,
    /// Estimated dimensionless Mino parameter at the interpolant crossing.
    pub mino_parameter: f64,
    /// Lower and upper parameter values bracketing the interpolant root.
    pub parameter_bracket: [f64; 2],
    /// Half-width of the root bracket in dimensionless Mino parameter.
    pub root_solver_error_bound: f64,
    /// Absolute radial residual of the interpolant at the reported estimate.
    pub interpolant_radius_residual_over_m: f64,
    /// Dimensionless outer-horizon radius r_+/M.
    pub horizon_radius_over_m: f64,
}

/// Fail-closed errors for horizon-event localization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KerrHorizonEventError {
    NotIngoingKerrChart,
    IncompleteTrajectory,
    InvalidParameterTolerance(f64),
    NonFiniteTrajectoryValue { point_index: usize },
    NonIncreasingMinoParameter { point_index: usize },
    NonMonotoneCrossingBracket {
        segment_index: usize,
        maximum_normalized_radial_derivative: f64,
    },
    RootPrecisionExhausted {
        segment_index: usize,
        current_bracket_width: f64,
        requested_tolerance: f64,
    },
}

impl fmt::Display for KerrHorizonEventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotIngoingKerrChart => write!(
                f,
                "future outer-horizon event localization requires ingoing Kerr coordinates"
            ),
            Self::IncompleteTrajectory => {
                write!(f, "at least two accepted trajectory points are required")
            }
            Self::InvalidParameterTolerance(value) => write!(
                f,
                "Mino-parameter root tolerance must be finite and positive, got {value}"
            ),
            Self::NonFiniteTrajectoryValue { point_index } => write!(
                f,
                "trajectory point {point_index} contains a non-finite parameter, radius or radial velocity"
            ),
            Self::NonIncreasingMinoParameter { point_index } => write!(
                f,
                "Mino parameter is not strictly increasing at trajectory point {point_index}"
            ),
            Self::NonMonotoneCrossingBracket {
                segment_index,
                maximum_normalized_radial_derivative,
            } => write!(
                f,
                "accepted segment {segment_index} has a non-monotone horizon-crossing interpolant (maximum d(r/M)/d(theta)={maximum_normalized_radial_derivative:e})"
            ),
            Self::RootPrecisionExhausted {
                segment_index,
                current_bracket_width,
                requested_tolerance,
            } => write!(
                f,
                "horizon event root on segment {segment_index} reached floating-point resolution before requested tolerance (bracket={current_bracket_width:e}, requested={requested_tolerance:e})"
            ),
        }
    }
}

impl std::error::Error for KerrHorizonEventError {}

/// Localize the first sampled future-horizon crossing in an ingoing Kerr
/// trajectory using cubic-Hermite interpolation and safeguarded bisection.
///
/// Only an adjacent accepted-step pair with r_left > r_+ and r_right <= r_+
/// is eligible. The interpolant must be strictly decreasing over the full
/// segment; otherwise the routine refuses to choose among potentially
/// multiple roots. This deliberately favors a typed unresolved result over
/// silently selecting a later crossing.
///
/// `parameter_tolerance` controls only the bisection bracket width in the
/// dimensionless Mino parameter. The returned value is not an exact-geodesic
/// event-time uncertainty bound; repeat the integration at tighter step
/// tolerances and compare the localized event to estimate interpolation error.
pub fn localize_first_outer_horizon_crossing(
    trajectory: &KerrMinoTrajectory,
    hole: KerrBlackHole,
    parameter_tolerance: f64,
) -> Result<Option<KerrHorizonEvent>, KerrHorizonEventError> {
    if trajectory.coordinate_chart != KerrMinoCoordinateChart::IngoingKerr {
        return Err(KerrHorizonEventError::NotIngoingKerrChart);
    }
    if trajectory.points.len() < 2 {
        return Err(KerrHorizonEventError::IncompleteTrajectory);
    }
    if !parameter_tolerance.is_finite() || parameter_tolerance <= 0.0 {
        return Err(KerrHorizonEventError::InvalidParameterTolerance(
            parameter_tolerance,
        ));
    }

    for (index, point) in trajectory.points.iter().enumerate() {
        if !point.mino_parameter.is_finite()
            || !point.state.coordinates.radius_over_m.is_finite()
            || !point.state.radial_velocity_over_m.is_finite()
        {
            return Err(KerrHorizonEventError::NonFiniteTrajectoryValue {
                point_index: index,
            });
        }
        if index > 0
            && point.mino_parameter <= trajectory.points[index - 1].mino_parameter
        {
            return Err(KerrHorizonEventError::NonIncreasingMinoParameter {
                point_index: index,
            });
        }
    }

    let gravitational_radius = hole.gravitational_radius_m();
    let horizon_radius_over_m = hole.outer_horizon_radius_m() / gravitational_radius;

    for segment_index in 0..trajectory.points.len() - 1 {
        let left = trajectory.points[segment_index];
        let right = trajectory.points[segment_index + 1];
        let left_radius = left.state.coordinates.radius_over_m;
        let right_radius = right.state.coordinates.radius_over_m;

        if left_radius <= horizon_radius_over_m || right_radius > horizon_radius_over_m {
            continue;
        }

        let step = right.mino_parameter - left.mino_parameter;
        let left_velocity = left.state.radial_velocity_over_m;
        let right_velocity = right.state.radial_velocity_over_m;

        // Cubic Hermite polynomial p(theta) = r(theta) - r_+, theta in [0,1].
        let d0 = left_radius - horizon_radius_over_m;
        let d1 = right_radius - horizon_radius_over_m;
        let a = 2.0 * d0 - 2.0 * d1 + step * (left_velocity + right_velocity);
        let b = -3.0 * d0 + 3.0 * d1 - step * (2.0 * left_velocity + right_velocity);
        let c = step * left_velocity;

        let radial_residual = |theta: f64| {
            ((a * theta + b) * theta + c) * theta + d0
        };
        let normalized_radial_derivative = |theta: f64| {
            (3.0 * a * theta + 2.0 * b) * theta + c
        };

        // The derivative is quadratic. Its maximum on [0,1] occurs at an
        // endpoint or, when concave, at its interior vertex.
        let mut maximum_derivative = normalized_radial_derivative(0.0)
            .max(normalized_radial_derivative(1.0));
        if a < 0.0 {
            let vertex = -b / (3.0 * a);
            if (0.0..=1.0).contains(&vertex) {
                maximum_derivative =
                    maximum_derivative.max(normalized_radial_derivative(vertex));
            }
        }
        if !maximum_derivative.is_finite() || maximum_derivative >= 0.0 {
            return Err(KerrHorizonEventError::NonMonotoneCrossingBracket {
                segment_index,
                maximum_normalized_radial_derivative: maximum_derivative,
            });
        }

        // The accepted endpoints bracket one unique root because the
        // interpolant is strictly decreasing and has opposite endpoint signs.
        let mut lower = 0.0;
        let mut upper = 1.0;
        for _ in 0..2048 {
            let parameter_width = step * (upper - lower);
            if parameter_width <= parameter_tolerance {
                break;
            }
            let middle = 0.5 * (lower + upper);
            if middle == lower || middle == upper {
                return Err(KerrHorizonEventError::RootPrecisionExhausted {
                    segment_index,
                    current_bracket_width: parameter_width,
                    requested_tolerance: parameter_tolerance,
                });
            }
            if radial_residual(middle) > 0.0 {
                lower = middle;
            } else {
                upper = middle;
            }
        }

        let final_bracket_width = step * (upper - lower);
        if final_bracket_width > parameter_tolerance {
            return Err(KerrHorizonEventError::RootPrecisionExhausted {
                segment_index,
                current_bracket_width: final_bracket_width,
                requested_tolerance: parameter_tolerance,
            });
        }

        let middle = 0.5 * (lower + upper);
        let mino_parameter = left.mino_parameter + step * middle;
        let lower_parameter = left.mino_parameter + step * lower;
        let upper_parameter = left.mino_parameter + step * upper;

        return Ok(Some(KerrHorizonEvent {
            segment_index,
            mino_parameter,
            parameter_bracket: [lower_parameter, upper_parameter],
            root_solver_error_bound: 0.5 * (upper_parameter - lower_parameter),
            interpolant_radius_residual_over_m: radial_residual(middle).abs(),
            horizon_radius_over_m,
        }));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DimensionlessSpin, MassKg,
        kerr_geodesic::{
            KerrMinoConstraintResiduals, KerrMinoDynamicState, KerrMinoState,
            KerrMinoTrajectoryPoint,
        },
    };

    const SOLAR_MASS_KG: f64 = 1.988_47e30;

    fn hole(spin: f64) -> KerrBlackHole {
        KerrBlackHole::new(
            MassKg::new(SOLAR_MASS_KG).unwrap(),
            DimensionlessSpin::new(spin).unwrap(),
        )
    }

    fn point(gamma: f64, radius: f64, radial_velocity: f64) -> KerrMinoTrajectoryPoint {
        KerrMinoTrajectoryPoint {
            mino_parameter: gamma,
            state: KerrMinoDynamicState::new(
                KerrMinoState::new(
                    0.0,
                    radius,
                    core::f64::consts::FRAC_PI_2,
                    0.0,
                ),
                radial_velocity,
                0.0,
            ),
            constraint_residuals: KerrMinoConstraintResiduals {
                radial: 0.0,
                polar: 0.0,
            },
        }
    }

    fn trajectory(
        chart: KerrMinoCoordinateChart,
        points: Vec<KerrMinoTrajectoryPoint>,
    ) -> KerrMinoTrajectory {
        KerrMinoTrajectory {
            coordinate_chart: chart,
            points,
            accepted_steps: 1,
            rejected_steps: 0,
            suggested_next_step: 0.1,
            maximum_normalized_radial_constraint_residual: 0.0,
            maximum_normalized_polar_constraint_residual: 0.0,
            requested_final_parameter: 2.0,
        }
    }

    #[test]
    fn localizes_linear_ingoing_crossing_inside_accepted_step() {
        let black_hole = hole(0.9);
        let horizon = black_hole.outer_horizon_radius_m()
            / black_hole.gravitational_radius_m();
        let ray = trajectory(
            KerrMinoCoordinateChart::IngoingKerr,
            vec![
                point(0.0, horizon + 0.5, -0.5),
                point(2.0, horizon - 0.5, -0.5),
            ],
        );

        let event = localize_first_outer_horizon_crossing(&ray, black_hole, 1.0e-10)
            .unwrap()
            .unwrap();
        assert_eq!(event.segment_index, 0);
        assert!((event.mino_parameter - 1.0).abs() <= 1.0e-10);
        assert!(event.parameter_bracket[0] <= event.mino_parameter);
        assert!(event.mino_parameter <= event.parameter_bracket[1]);
        assert!(event.root_solver_error_bound <= 5.0e-11);
        assert!(event.interpolant_radius_residual_over_m <= 1.0e-10);
    }

    #[test]
    fn returns_none_when_no_sampled_horizon_bracket_exists() {
        let black_hole = hole(0.9);
        let horizon = black_hole.outer_horizon_radius_m()
            / black_hole.gravitational_radius_m();
        let ray = trajectory(
            KerrMinoCoordinateChart::IngoingKerr,
            vec![
                point(0.0, horizon + 2.0, -1.0),
                point(1.0, horizon + 1.0, -1.0),
                point(2.0, horizon + 0.5, -0.5),
            ],
        );
        assert_eq!(
            localize_first_outer_horizon_crossing(&ray, black_hole, 1.0e-10).unwrap(),
            None
        );
    }

    #[test]
    fn refuses_boyer_lindquist_chart_and_invalid_tolerance() {
        let black_hole = hole(0.9);
        let horizon = black_hole.outer_horizon_radius_m()
            / black_hole.gravitational_radius_m();
        let points = vec![
            point(0.0, horizon + 0.5, -0.5),
            point(2.0, horizon - 0.5, -0.5),
        ];
        let boyer_lindquist =
            trajectory(KerrMinoCoordinateChart::BoyerLindquist, points.clone());
        assert!(matches!(
            localize_first_outer_horizon_crossing(&boyer_lindquist, black_hole, 1.0e-10),
            Err(KerrHorizonEventError::NotIngoingKerrChart)
        ));
        let ingoing = trajectory(KerrMinoCoordinateChart::IngoingKerr, points);
        assert!(matches!(
            localize_first_outer_horizon_crossing(&ingoing, black_hole, 0.0),
            Err(KerrHorizonEventError::InvalidParameterTolerance(_))
        ));
    }

    #[test]
    fn reports_when_requested_root_tolerance_exceeds_float_resolution() {
        let black_hole = hole(0.9);
        let horizon = black_hole.outer_horizon_radius_m()
            / black_hole.gravitational_radius_m();
        let ray = trajectory(
            KerrMinoCoordinateChart::IngoingKerr,
            vec![
                point(0.0, horizon + 0.5, -0.5),
                point(2.0, horizon - 0.5, -0.5),
            ],
        );
        assert!(matches!(
            localize_first_outer_horizon_crossing(&ray, black_hole, f64::MIN_POSITIVE),
            Err(KerrHorizonEventError::RootPrecisionExhausted { .. })
        ));
    }

    #[test]
    fn refuses_a_nonmonotone_interpolant_instead_of_guessing_a_root() {
        let black_hole = hole(0.9);
        let horizon = black_hole.outer_horizon_radius_m()
            / black_hole.gravitational_radius_m();
        let ray = trajectory(
            KerrMinoCoordinateChart::IngoingKerr,
            vec![
                point(0.0, horizon + 0.5, 2.0),
                point(2.0, horizon - 0.5, 2.0),
            ],
        );
        assert!(matches!(
            localize_first_outer_horizon_crossing(&ray, black_hole, 1.0e-10),
            Err(KerrHorizonEventError::NonMonotoneCrossingBracket { .. })
        ));
    }
}
