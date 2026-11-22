//! Deterministic pose-misalignment envelope from bounded clock error and motion.
//!
//! The result is a conservative first-order/constant-acceleration displacement
//! envelope for a declared time-misalignment bound. It is not a covariance,
//! probability, protection level or integrity certificate. The caller must
//! independently qualify every clock, age, motion and lever-arm bound.

use crate::student_t_pl::StudentTProtectionLevels;
use std::f64::consts::PI;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TemporalAlignmentBounds {
    /// Maximum absolute relative clock offset at the last synchronization, seconds.
    pub max_abs_offset_s: f64,
    /// Maximum absolute relative clock scale error, parts per million.
    pub max_abs_clock_drift_ppm: f64,
    /// Elapsed time since the last qualified synchronization, seconds.
    pub elapsed_since_sync_s: f64,
    /// Maximum age of the measurement relative to the output epoch, seconds.
    pub max_observation_age_s: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionEnvelope {
    /// Maximum translational speed of the navigation reference point, m/s.
    pub max_linear_speed_mps: f64,
    /// Maximum translational acceleration magnitude, m/s^2.
    pub max_linear_acceleration_mps2: f64,
    /// Maximum angular rate magnitude, rad/s.
    pub max_angular_rate_radps: f64,
    /// Maximum angular acceleration magnitude, rad/s^2.
    pub max_angular_acceleration_radps2: f64,
    /// Distance from the navigation reference point to the sensor origin, metres.
    pub sensor_lever_arm_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TemporalMisalignmentEnvelope {
    pub clock_offset_component_s: f64,
    pub clock_drift_component_s: f64,
    pub observation_age_component_s: f64,
    pub total_time_misalignment_bound_s: f64,
    pub linear_motion_displacement_bound_m: f64,
    pub angular_misalignment_bound_rad: f64,
    pub rotational_lever_arm_displacement_bound_m: f64,
    pub total_position_misalignment_bound_m: f64,
}

/// Student-t protection levels after adding a deterministic temporal-bias bound.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TemporallyInflatedStudentTProtectionLevels {
    pub base_overall_pl_m: f64,
    pub base_longitudinal_pl_m: f64,
    pub base_lateral_pl_m: f64,
    pub temporal_position_bias_bound_m: f64,
    pub inflated_overall_pl_m: f64,
    pub inflated_longitudinal_pl_m: f64,
    pub inflated_lateral_pl_m: f64,
}

/// Bounded synchronization-holdover decision under a position-bias budget.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SynchronizationHoldoverDecision {
    /// Maximum additional time before synchronization must be refreshed, seconds.
    pub maximum_additional_holdover_s: f64,
    /// The timing-derived position budget is satisfied at the current instant.
    pub budget_satisfied_now: bool,
    /// The declared maximum search horizon is still inside the position budget.
    pub budget_satisfied_at_search_limit: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TemporalAlignmentError {
    NonFiniteOrNegativeInput,
    ClockDriftBoundTooLarge,
    InvalidBaseProtectionLevel,
    InvalidPositionBudget,
    NonFiniteOutput,
}

impl fmt::Display for TemporalAlignmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteOrNegativeInput => write!(f, "temporal and motion bounds must be finite and nonnegative"),
            Self::ClockDriftBoundTooLarge => write!(f, "relative clock drift bound must not exceed 1,000,000 ppm"),
            Self::InvalidBaseProtectionLevel => write!(f, "base Student-t protection levels must be finite and nonnegative"),
            Self::InvalidPositionBudget => write!(f, "position budget and maximum holdover horizon must be finite and positive"),
            Self::NonFiniteOutput => write!(f, "temporal misalignment envelope overflowed or became non-finite"),
        }
    }
}
impl std::error::Error for TemporalAlignmentError {}

fn all_finite_nonnegative(values: &[f64]) -> bool {
    values.iter().all(|value| value.is_finite() && *value >= 0.0)
}

/// Compute a deterministic position/orientation error envelope from bounded
/// relative clock offset, clock scale drift, observation age and motion.
///
/// Relative skew is bounded by:
/// `offset + drift_ppm * 1e-6 * elapsed_since_sync + observation_age`.
///
/// Translational displacement is bounded by `v_max * dt + 0.5 * a_max * dt^2`.
/// Angular displacement uses the analogous rate/acceleration expression.
/// Rotational lever-arm displacement is bounded by the chord length
/// `2 * r * sin(min(theta, pi)/2)`.
///
/// The result is a deterministic sensitivity bound, not a probability or PL.
pub fn temporal_misalignment_envelope(
    clock: TemporalAlignmentBounds,
    motion: MotionEnvelope,
) -> Result<TemporalMisalignmentEnvelope, TemporalAlignmentError> {
    let temporal = [
        clock.max_abs_offset_s,
        clock.max_abs_clock_drift_ppm,
        clock.elapsed_since_sync_s,
        clock.max_observation_age_s,
    ];
    let physical = [
        motion.max_linear_speed_mps,
        motion.max_linear_acceleration_mps2,
        motion.max_angular_rate_radps,
        motion.max_angular_acceleration_radps2,
        motion.sensor_lever_arm_m,
    ];
    if !all_finite_nonnegative(&temporal) || !all_finite_nonnegative(&physical) {
        return Err(TemporalAlignmentError::NonFiniteOrNegativeInput);
    }
    if clock.max_abs_clock_drift_ppm > 1_000_000.0 {
        return Err(TemporalAlignmentError::ClockDriftBoundTooLarge);
    }

    let clock_drift_component_s =
        clock.max_abs_clock_drift_ppm * 1.0e-6 * clock.elapsed_since_sync_s;
    let total_time_misalignment_bound_s = clock.max_abs_offset_s
        + clock_drift_component_s
        + clock.max_observation_age_s;
    let dt = total_time_misalignment_bound_s;
    let linear_motion_displacement_bound_m =
        motion.max_linear_speed_mps * dt
            + 0.5 * motion.max_linear_acceleration_mps2 * dt * dt;
    let angular_misalignment_bound_rad =
        motion.max_angular_rate_radps * dt
            + 0.5 * motion.max_angular_acceleration_radps2 * dt * dt;
    let bounded_angle = angular_misalignment_bound_rad.min(PI);
    let rotational_lever_arm_displacement_bound_m =
        2.0 * motion.sensor_lever_arm_m * (0.5 * bounded_angle).sin();
    let total_position_misalignment_bound_m =
        linear_motion_displacement_bound_m + rotational_lever_arm_displacement_bound_m;

    let output = TemporalMisalignmentEnvelope {
        clock_offset_component_s: clock.max_abs_offset_s,
        clock_drift_component_s,
        observation_age_component_s: clock.max_observation_age_s,
        total_time_misalignment_bound_s,
        linear_motion_displacement_bound_m,
        angular_misalignment_bound_rad,
        rotational_lever_arm_displacement_bound_m,
        total_position_misalignment_bound_m,
    };
    let values = [
        output.clock_drift_component_s,
        output.total_time_misalignment_bound_s,
        output.linear_motion_displacement_bound_m,
        output.angular_misalignment_bound_rad,
        output.rotational_lever_arm_displacement_bound_m,
        output.total_position_misalignment_bound_m,
    ];
    if !values.iter().all(|value| value.is_finite() && *value >= 0.0) {
        return Err(TemporalAlignmentError::NonFiniteOutput);
    }
    Ok(output)
}



/// Find the maximum additional synchronization holdover before the temporal
/// position-bias envelope exceeds a declared budget.
///
/// The search is monotonic because elapsed clock drift can only increase the
/// total temporal bound under the declared nonnegative inputs. The function
/// returns the maximum horizon within `max_search_horizon_s`; if the budget is
/// already exceeded it returns zero. If the envelope remains below budget for
/// the whole horizon, the result is capped at that declared search limit.
///
/// This is a scheduling aid, not a clock synchronizer or a guarantee that a
/// later synchronization will succeed. The clock/motion/age bounds must be
/// qualified independently and re-evaluated after resets or ODD changes.
pub fn maximum_sync_holdover_under_position_budget(
    clock_at_decision: TemporalAlignmentBounds,
    motion: MotionEnvelope,
    max_position_bias_budget_m: f64,
    max_search_horizon_s: f64,
) -> Result<SynchronizationHoldoverDecision, TemporalAlignmentError> {
    if !max_position_bias_budget_m.is_finite()
        || max_position_bias_budget_m <= 0.0
        || !max_search_horizon_s.is_finite()
        || max_search_horizon_s <= 0.0
    {
        return Err(TemporalAlignmentError::InvalidPositionBudget);
    }
    let at_elapsed = |additional_s: f64| {
        let mut clock = clock_at_decision;
        clock.elapsed_since_sync_s += additional_s;
        temporal_misalignment_envelope(clock, motion)
    };
    let now = at_elapsed(0.0)?;
    if now.total_position_misalignment_bound_m > max_position_bias_budget_m {
        return Ok(SynchronizationHoldoverDecision {
            maximum_additional_holdover_s: 0.0,
            budget_satisfied_now: false,
            budget_satisfied_at_search_limit: false,
        });
    }
    let at_limit = at_elapsed(max_search_horizon_s)?;
    if at_limit.total_position_misalignment_bound_m <= max_position_bias_budget_m {
        return Ok(SynchronizationHoldoverDecision {
            maximum_additional_holdover_s: max_search_horizon_s,
            budget_satisfied_now: true,
            budget_satisfied_at_search_limit: true,
        });
    }
    let mut low = 0.0_f64;
    let mut high = max_search_horizon_s;
    for _ in 0..80 {
        let midpoint = low + (high - low) * 0.5;
        if at_elapsed(midpoint)?.total_position_misalignment_bound_m <= max_position_bias_budget_m {
            low = midpoint;
        } else {
            high = midpoint;
        }
    }
    Ok(SynchronizationHoldoverDecision {
        maximum_additional_holdover_s: low,
        budget_satisfied_now: true,
        budget_satisfied_at_search_limit: false,
    })
}
/// Add a deterministic temporal misregistration bound to an existing Student-t PL.
///
/// For any direction, the absolute projection of the position-bias vector is
/// no larger than its Euclidean norm. Therefore `PL_total = PL_stat + delta_p`
/// is a conservative triangle-inequality composition when `delta_p` is a
/// validated hard bound. This function deliberately does not RSS-combine
/// timing and estimator terms or assume their independence.
pub fn inflate_student_t_protection_levels_by_temporal_bound(
    base: StudentTProtectionLevels,
    temporal: TemporalMisalignmentEnvelope,
) -> Result<TemporallyInflatedStudentTProtectionLevels, TemporalAlignmentError> {
    let base_values = [
        base.overall_pl_m,
        base.longitudinal_pl_m,
        base.lateral_pl_m,
        base.maximum_eigenvalue,
        base.degrees_of_freedom,
        base.k_tir_2d,
    ];
    if !base_values.iter().all(|value| value.is_finite())
        || base.overall_pl_m < 0.0
        || base.longitudinal_pl_m < 0.0
        || base.lateral_pl_m < 0.0
        || base.maximum_eigenvalue < 0.0
        || base.degrees_of_freedom <= 2.0
        || base.k_tir_2d <= 0.0
    {
        return Err(TemporalAlignmentError::InvalidBaseProtectionLevel);
    }
    let bias = temporal.total_position_misalignment_bound_m;
    if !bias.is_finite() || bias < 0.0 {
        return Err(TemporalAlignmentError::NonFiniteOutput);
    }
    let result = TemporallyInflatedStudentTProtectionLevels {
        base_overall_pl_m: base.overall_pl_m,
        base_longitudinal_pl_m: base.longitudinal_pl_m,
        base_lateral_pl_m: base.lateral_pl_m,
        temporal_position_bias_bound_m: bias,
        inflated_overall_pl_m: base.overall_pl_m + bias,
        inflated_longitudinal_pl_m: base.longitudinal_pl_m + bias,
        inflated_lateral_pl_m: base.lateral_pl_m + bias,
    };
    let inflated = [result.inflated_overall_pl_m, result.inflated_longitudinal_pl_m, result.inflated_lateral_pl_m];
    if !inflated.iter().all(|value| value.is_finite()) {
        return Err(TemporalAlignmentError::NonFiniteOutput);
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;

    fn clock(offset: f64, drift_ppm: f64, elapsed: f64, age: f64) -> TemporalAlignmentBounds {
        TemporalAlignmentBounds {
            max_abs_offset_s: offset,
            max_abs_clock_drift_ppm: drift_ppm,
            elapsed_since_sync_s: elapsed,
            max_observation_age_s: age,
        }
    }

    fn motion(speed: f64, accel: f64, omega: f64, alpha: f64, lever: f64) -> MotionEnvelope {
        MotionEnvelope {
            max_linear_speed_mps: speed,
            max_linear_acceleration_mps2: accel,
            max_angular_rate_radps: omega,
            max_angular_acceleration_radps2: alpha,
            sensor_lever_arm_m: lever,
        }
    }

    #[test]
    fn zero_clock_age_and_motion_produce_zero_envelope() {
        let result = temporal_misalignment_envelope(clock(0.0, 0.0, 0.0, 0.0), motion(0.0, 0.0, 0.0, 0.0, 0.0)).unwrap();
        assert_eq!(result.total_time_misalignment_bound_s, 0.0);
        assert_eq!(result.total_position_misalignment_bound_m, 0.0);
        assert_eq!(result.angular_misalignment_bound_rad, 0.0);
    }

    #[test]
    fn offset_and_age_propagate_through_linear_motion_bound() {
        let result = temporal_misalignment_envelope(clock(0.01, 0.0, 0.0, 0.02), motion(2.0, 0.0, 0.0, 0.0, 0.0)).unwrap();
        assert!((result.total_time_misalignment_bound_s - 0.03).abs() < 1.0e-12);
        assert!((result.linear_motion_displacement_bound_m - 0.06).abs() < 1.0e-12);
        assert_eq!(result.rotational_lever_arm_displacement_bound_m, 0.0);
    }

    #[test]
    fn clock_scale_drift_accumulates_since_last_qualified_sync() {
        let result = temporal_misalignment_envelope(clock(0.0, 50.0, 1_000.0, 0.0), motion(3.0, 0.0, 0.0, 0.0, 0.0)).unwrap();
        assert!((result.clock_drift_component_s - 0.05).abs() < 1.0e-12);
        assert!((result.linear_motion_displacement_bound_m - 0.15).abs() < 1.0e-12);
    }

    #[test]
    fn angular_skew_propagates_to_sensor_lever_arm_position_error() {
        let result = temporal_misalignment_envelope(clock(0.0, 0.0, 0.0, 0.1), motion(0.0, 0.0, 1.0, 0.0, 1.0)).unwrap();
        let expected = 2.0 * (0.05_f64).sin();
        assert!((result.angular_misalignment_bound_rad - 0.1).abs() < 1.0e-12);
        assert!((result.rotational_lever_arm_displacement_bound_m - expected).abs() < 1.0e-12);
    }

    #[test]
    fn acceleration_bounds_contribute_quadratically() {
        let result = temporal_misalignment_envelope(clock(0.0, 0.0, 0.0, 0.1), motion(0.0, 2.0, 0.0, 2.0, 0.5)).unwrap();
        assert!((result.linear_motion_displacement_bound_m - 0.01).abs() < 1.0e-12);
        assert!((result.angular_misalignment_bound_rad - 0.01).abs() < 1.0e-12);
        assert!((result.rotational_lever_arm_displacement_bound_m - 2.0 * 0.5 * (0.005_f64).sin()).abs() < 1.0e-12);
    }

    #[test]
    fn increasing_clock_age_or_observation_age_never_reduces_the_bound() {
        let fast_sync = temporal_misalignment_envelope(clock(0.001, 100.0, 10.0, 0.01), motion(5.0, 1.0, 0.4, 0.2, 0.4)).unwrap();
        let stale_sync = temporal_misalignment_envelope(clock(0.001, 100.0, 100.0, 0.01), motion(5.0, 1.0, 0.4, 0.2, 0.4)).unwrap();
        let stale_observation = temporal_misalignment_envelope(clock(0.001, 100.0, 10.0, 0.05), motion(5.0, 1.0, 0.4, 0.2, 0.4)).unwrap();
        assert!(stale_sync.total_position_misalignment_bound_m > fast_sync.total_position_misalignment_bound_m);
        assert!(stale_observation.total_position_misalignment_bound_m > fast_sync.total_position_misalignment_bound_m);
    }

    #[test]
    fn sync_holdover_solver_finds_budget_crossing_under_clock_drift() {
        let decision = maximum_sync_holdover_under_position_budget(
            clock(0.001, 100.0, 0.0, 0.0),
            motion(5.0, 0.0, 0.0, 0.0, 0.0),
            0.01,
            100.0,
        ).unwrap();
        assert!(decision.budget_satisfied_now);
        assert!(!decision.budget_satisfied_at_search_limit);
        assert!(decision.maximum_additional_holdover_s > 0.0);
        assert!(decision.maximum_additional_holdover_s < 20.0);
        let at_boundary = temporal_misalignment_envelope(
            clock(0.001, 100.0, decision.maximum_additional_holdover_s, 0.0),
            motion(5.0, 0.0, 0.0, 0.0, 0.0),
        ).unwrap();
        assert!(at_boundary.total_position_misalignment_bound_m <= 0.01 + 1.0e-9);
    }

    #[test]
    fn sync_holdover_returns_zero_when_budget_is_already_exceeded() {
        let decision = maximum_sync_holdover_under_position_budget(
            clock(0.1, 100.0, 10.0, 0.1),
            motion(10.0, 1.0, 1.0, 1.0, 1.0),
            0.01,
            100.0,
        ).unwrap();
        assert_eq!(decision.maximum_additional_holdover_s, 0.0);
        assert!(!decision.budget_satisfied_now);
    }

    #[test]
    fn sync_holdover_caps_at_search_limit_when_clock_drift_is_zero() {
        let decision = maximum_sync_holdover_under_position_budget(
            clock(0.001, 0.0, 10.0, 0.0),
            motion(1.0, 0.0, 0.0, 0.0, 0.0),
            0.01,
            250.0,
        ).unwrap();
        assert_eq!(decision.maximum_additional_holdover_s, 250.0);
        assert!(decision.budget_satisfied_at_search_limit);
    }

    #[test]
    fn invalid_sync_budget_or_search_horizon_fails_closed() {
        assert_eq!(
            maximum_sync_holdover_under_position_budget(clock(0.0, 0.0, 0.0, 0.0), motion(0.0, 0.0, 0.0, 0.0, 0.0), 0.0, 10.0),
            Err(TemporalAlignmentError::InvalidPositionBudget)
        );
        assert_eq!(
            maximum_sync_holdover_under_position_budget(clock(0.0, 0.0, 0.0, 0.0), motion(0.0, 0.0, 0.0, 0.0, 0.0), 0.1, f64::NAN),
            Err(TemporalAlignmentError::InvalidPositionBudget)
        );
    }
    #[test]
    fn temporal_bias_is_added_to_statistical_pl_without_independence_assumption() {
        let temporal = temporal_misalignment_envelope(
            clock(0.01, 0.0, 0.0, 0.02),
            motion(2.0, 0.0, 0.0, 0.0, 0.0),
        ).unwrap();
        let base = StudentTProtectionLevels {
            degrees_of_freedom: 5.0,
            k_tir_2d: 2.0,
            maximum_eigenvalue: 4.0,
            overall_pl_m: 4.0,
            longitudinal_pl_m: 3.0,
            lateral_pl_m: 2.0,
        };
        let combined = inflate_student_t_protection_levels_by_temporal_bound(base, temporal).unwrap();
        assert!((combined.temporal_position_bias_bound_m - 0.06).abs() < 1.0e-12);
        assert!((combined.inflated_overall_pl_m - 4.06).abs() < 1.0e-12);
        assert!((combined.inflated_longitudinal_pl_m - 3.06).abs() < 1.0e-12);
        assert!((combined.inflated_lateral_pl_m - 2.06).abs() < 1.0e-12);
    }

    #[test]
    fn invalid_base_protection_level_and_overflow_inflation_fail_closed() {
        let zero = temporal_misalignment_envelope(clock(0.0, 0.0, 0.0, 0.0), motion(0.0, 0.0, 0.0, 0.0, 0.0)).unwrap();
        let invalid = StudentTProtectionLevels {
            degrees_of_freedom: 5.0,
            k_tir_2d: 2.0,
            maximum_eigenvalue: 4.0,
            overall_pl_m: f64::NAN,
            longitudinal_pl_m: 3.0,
            lateral_pl_m: 2.0,
        };
        assert_eq!(inflate_student_t_protection_levels_by_temporal_bound(invalid, zero), Err(TemporalAlignmentError::InvalidBaseProtectionLevel));
        let overflow_temporal = temporal_misalignment_envelope(clock(0.0, 0.0, 0.0, 1.0e200), motion(1.0e200, 0.0, 0.0, 0.0, 0.0));
        assert_eq!(overflow_temporal, Err(TemporalAlignmentError::NonFiniteOutput));
    }
    #[test]
    fn invalid_inputs_and_unreasonable_clock_drift_fail_closed() {
        assert_eq!(temporal_misalignment_envelope(clock(-0.1, 0.0, 0.0, 0.0), motion(1.0, 0.0, 0.0, 0.0, 0.0)), Err(TemporalAlignmentError::NonFiniteOrNegativeInput));
        assert_eq!(temporal_misalignment_envelope(clock(0.0, 1_000_001.0, 1.0, 0.0), motion(1.0, 0.0, 0.0, 0.0, 0.0)), Err(TemporalAlignmentError::ClockDriftBoundTooLarge));
        assert_eq!(temporal_misalignment_envelope(clock(0.0, f64::NAN, 1.0, 0.0), motion(1.0, 0.0, 0.0, 0.0, 0.0)), Err(TemporalAlignmentError::NonFiniteOrNegativeInput));
    }

    #[test]
    fn arithmetic_overflow_fails_closed() {
        assert_eq!(temporal_misalignment_envelope(clock(0.0, 0.0, 0.0, 1.0e200), motion(1.0e200, 0.0, 0.0, 0.0, 0.0)), Err(TemporalAlignmentError::NonFiniteOutput));
    }
}
