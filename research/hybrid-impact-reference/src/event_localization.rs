//! Bracketed event-localization baseline for the 1D analytical impact model.
//!
//! This is an independent numerical root-localization lane. It deliberately
//! does not use the closed-form impact time to perform the bisection. The
//! closed-form solution is used only as an external reference in tests and
//! benchmark analysis.

use super::{ImpactError, ImpactParameters, first_impact};

/// Input or model error when establishing an impact bracket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalizationError {
    InvalidModel(ImpactError),
    NonFiniteTimeBound,
    NegativeTimeBound,
    InvalidTimeBracket,
    InvalidTimeTolerance,
    InvalidPositionTolerance,
    ZeroIterationBudget,
    NonFiniteTrajectoryPosition,
    NoDownwardCrossingInBracket,
}

/// Result status for a bracketed event-time query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalizationStatus {
    /// Both declared time-bracket width and guard-residual tolerances were met.
    Converged,
    /// The iteration budget ended before both tolerances were met.
    IterationLimit,
    /// Adjacent representable floating-point times prevented further bisection.
    TimeResolutionExhausted,
}

/// A bracket and diagnostics for one downward crossing of the plane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpactLocalization {
    /// Lower endpoint; trajectory position is strictly above the plane.
    pub lower_time_s: f64,
    /// Upper endpoint; trajectory position is on or below the plane.
    pub upper_time_s: f64,
    /// Midpoint of the returned bracket.
    pub estimated_event_time_s: f64,
    /// Absolute guard value at the estimated event time, in metres.
    pub absolute_guard_residual_m: f64,
    /// Width of the final event-time bracket, in seconds.
    pub bracket_width_s: f64,
    /// Number of bisection updates performed.
    pub iterations: u32,
    pub status: LocalizationStatus,
}

/// Evaluate the exact free-flight position from the initial condition.
fn free_flight_position(
    parameters: ImpactParameters,
    time_s: f64,
) -> Result<f64, LocalizationError> {
    let t_squared = time_s * time_s;
    let initial_velocity_term = parameters.initial_velocity_m_s * time_s;
    let gravity_term = 0.5 * parameters.gravity_m_s2 * t_squared;
    let position = parameters.initial_height_m + initial_velocity_term - gravity_term;
    if position.is_finite() {
        Ok(position)
    } else {
        Err(LocalizationError::NonFiniteTrajectoryPosition)
    }
}

/// Locate the first downward crossing of y=0 by bisection.
///
/// The caller supplies a bracket [lower_time_s, upper_time_s] satisfying:
/// - y(lower_time_s) > 0;
/// - y(upper_time_s) <= 0.
///
/// For the constant-gravity point-mass model with positive initial height,
/// the free-flight parabola has exactly one positive root, so this bracket
/// isolates the first impact. The returned status distinguishes convergence,
/// an exhausted iteration budget, and loss of representable time resolution.
///
/// This is a bisection baseline, not the production event locator for a
/// general hybrid system. The model and event surface are intentionally fixed.
pub fn bisect_first_impact(
    parameters: ImpactParameters,
    lower_time_s: f64,
    upper_time_s: f64,
    time_tolerance_s: f64,
    position_tolerance_m: f64,
    max_iterations: u32,
) -> Result<ImpactLocalization, LocalizationError> {
    first_impact(parameters).map_err(LocalizationError::InvalidModel)?;

    if !lower_time_s.is_finite() || !upper_time_s.is_finite() {
        return Err(LocalizationError::NonFiniteTimeBound);
    }
    if lower_time_s < 0.0 || upper_time_s < 0.0 {
        return Err(LocalizationError::NegativeTimeBound);
    }
    if lower_time_s >= upper_time_s {
        return Err(LocalizationError::InvalidTimeBracket);
    }
    if !time_tolerance_s.is_finite() || time_tolerance_s <= 0.0 {
        return Err(LocalizationError::InvalidTimeTolerance);
    }
    if !position_tolerance_m.is_finite() || position_tolerance_m <= 0.0 {
        return Err(LocalizationError::InvalidPositionTolerance);
    }
    if max_iterations == 0 {
        return Err(LocalizationError::ZeroIterationBudget);
    }

    let lower_position = free_flight_position(parameters, lower_time_s)?;
    let upper_position = free_flight_position(parameters, upper_time_s)?;
    if lower_position <= 0.0 || upper_position > 0.0 {
        return Err(LocalizationError::NoDownwardCrossingInBracket);
    }

    let mut lower = lower_time_s;
    let mut upper = upper_time_s;
    let mut iterations = 0;

    let status = loop {
        let width = upper - lower;
        let midpoint = lower + 0.5 * width;
        let midpoint_position = free_flight_position(parameters, midpoint)?;
        let residual = midpoint_position.abs();

        if width <= time_tolerance_s && residual <= position_tolerance_m {
            break LocalizationStatus::Converged;
        }

        if iterations >= max_iterations {
            break LocalizationStatus::IterationLimit;
        }

        if midpoint <= lower || midpoint >= upper {
            break LocalizationStatus::TimeResolutionExhausted;
        }

        if midpoint_position > 0.0 {
            lower = midpoint;
        } else {
            upper = midpoint;
        }
        iterations += 1;
    };

    let bracket_width_s = upper - lower;
    let estimated_event_time_s = lower + 0.5 * bracket_width_s;
    let final_position =
        free_flight_position(parameters, estimated_event_time_s)?;

    Ok(ImpactLocalization {
        lower_time_s: lower,
        upper_time_s: upper,
        estimated_event_time_s,
        absolute_guard_residual_m: final_position.abs(),
        bracket_width_s,
        iterations,
        status,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::first_impact;

    fn fixture() -> ImpactParameters {
        ImpactParameters {
            mass_kg: 3.0,
            gravity_m_s2: 2.0,
            initial_height_m: 1.0,
            initial_velocity_m_s: 0.0,
            restitution: 0.5,
        }
    }

    #[test]
    fn bisection_converges_to_analytic_first_impact() {
        let parameters = fixture();
        let analytic = first_impact(parameters).expect("analytic reference");
        let result =
            bisect_first_impact(parameters, 0.0, 2.0, 1.0e-10, 1.0e-10, 128)
                .expect("valid bracket");

        assert_eq!(result.status, LocalizationStatus::Converged);
        assert!(result.iterations > 0);
        assert!(result.bracket_width_s <= 1.0e-10);
        assert!(result.absolute_guard_residual_m <= 1.0e-10);
        assert!(result.lower_time_s <= analytic.impact_time_s);
        assert!(result.upper_time_s >= analytic.impact_time_s);
    }

    #[test]
    fn event_exactly_at_upper_step_boundary_is_retained_in_bracket() {
        let parameters = fixture();
        let result =
            bisect_first_impact(parameters, 0.5, 1.0, 1.0e-12, 1.0e-12, 128)
                .expect("upper endpoint is impact");

        assert_eq!(result.status, LocalizationStatus::Converged);
        assert!(result.lower_time_s <= 1.0);
        assert_eq!(result.upper_time_s, 1.0);
    }

    #[test]
    fn event_just_after_a_step_boundary_is_found_by_next_interval() {
        let mut parameters = fixture();
        parameters.initial_height_m = 1.000_000_1;
        let analytic = first_impact(parameters).expect("analytic reference");
        let result =
            bisect_first_impact(parameters, 1.0, 1.1, 1.0e-10, 1.0e-10, 128)
                .expect("crossing is in the second step");

        assert_eq!(result.status, LocalizationStatus::Converged);
        assert!(result.estimated_event_time_s > 1.0);
        assert!(
            (result.estimated_event_time_s - analytic.impact_time_s).abs()
                <= result.bracket_width_s
        );
    }

    #[test]
    fn event_just_before_boundary_is_not_claimed_by_later_interval() {
        let mut parameters = fixture();
        parameters.initial_height_m = 0.999_999_9;
        assert_eq!(
            bisect_first_impact(parameters, 1.0, 1.1, 1.0e-10, 1.0e-10, 128),
            Err(LocalizationError::NoDownwardCrossingInBracket)
        );
    }

    #[test]
    fn initial_downward_velocity_is_supported() {
        let mut parameters = fixture();
        parameters.initial_velocity_m_s = -3.0;
        let analytic = first_impact(parameters).expect("analytic reference");
        let result =
            bisect_first_impact(parameters, 0.0, 0.5, 1.0e-10, 1.0e-10, 128)
                .expect("crossing bracket");

        assert_eq!(result.status, LocalizationStatus::Converged);
        assert!(
            (result.estimated_event_time_s - analytic.impact_time_s).abs()
                <= result.bracket_width_s
        );
    }

    #[test]
    fn iteration_limit_is_visible_and_returns_remaining_bracket() {
        let result =
            bisect_first_impact(fixture(), 0.0, 2.0, 1.0e-16, 1.0e-16, 1)
                .expect("valid bracket");

        assert_eq!(result.status, LocalizationStatus::IterationLimit);
        assert!(result.bracket_width_s > 1.0e-16);
    }

    #[test]
    fn rejects_invalid_or_non_crossing_brackets() {
        assert_eq!(
            bisect_first_impact(fixture(), 1.0, 0.0, 1.0e-8, 1.0e-8, 64),
            Err(LocalizationError::InvalidTimeBracket)
        );
        assert_eq!(
            bisect_first_impact(fixture(), 0.0, 0.5, 1.0e-8, 1.0e-8, 64),
            Err(LocalizationError::NoDownwardCrossingInBracket)
        );
    }
}
