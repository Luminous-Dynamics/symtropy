// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Source-compatible PeakOverlapPos strict, oscillation and relaxed convergence policy.
//!
//! The pinned CARA routine evaluates this policy after updating the current
//! center-of-linearity and HBR-remediated overlap covariance. The caller
//! supplies the corresponding squared Mahalanobis changes; this module
//! preserves the source branch ordering and fractional convergence status.
//!
//! It does not calculate the Mahalanobis metric, propagate a state, adjust a
//! CoL, or decide whether the physical covariance model is adequate.

use crate::KernelError;

/// NASA CARA PeakOverlapPos convergence policy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeakOverlapConvergenceConfig {
    /// Maximum iteration count. CARA takes a one-step Coppola shortcut when
    /// this value is less than or equal to one.
    pub maximum_iterations: usize,
    /// Strict squared-Mahalanobis convergence tolerance (CARA default 1e-12).
    pub strict_md2_tolerance: f64,
    /// Relaxed squared-Mahalanobis convergence tolerance (CARA default 1e-6).
    pub relaxed_md2_tolerance: f64,
}

impl Default for PeakOverlapConvergenceConfig {
    fn default() -> Self {
        Self {
            maximum_iterations: 100,
            strict_md2_tolerance: 1.0e-12,
            relaxed_md2_tolerance: 1.0e-6,
        }
    }
}

/// Iteration observation required by the source convergence decision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeakOverlapConvergenceObservation {
    /// Zero-based CARA iteration number for the current completed update.
    pub iteration: usize,
    /// Squared Mahalanobis change from the previous overlap center.
    ///
    /// Required for iteration > 0. It is not required for the one-step
    /// Coppola shortcut or the initial iteration.
    pub current_vs_previous_md2: Option<f64>,
    /// Squared Mahalanobis change from the overlap center two iterations back.
    ///
    /// Required only when CARA's oscillation branch is eligible.
    pub current_vs_two_back_md2: Option<f64>,
    /// Iteration that last adjusted either object's CoL, if any.
    ///
    /// Strict convergence is disallowed on the same iteration as a CoL
    /// adjustment; the two-cycle test is delayed until more than two
    /// iterations after that adjustment.
    pub last_col_adjustment_iteration: Option<usize>,
}

/// Source-semantic convergence outcome; do not collapse this enum to bool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeakOverlapConvergenceStatus {
    /// The source's maxiter <= 1 original-Coppola shortcut.
    CoppolaOneStep,
    /// Strict convergence at MD2tol[0].
    Strict,
    /// Two-cycle oscillation resolved by the source's half-status result.
    OscillationResolved,
    /// Convergence accepted under the interpolated/relaxed tolerance schedule.
    Relaxed,
    /// Continue iterating; no convergence branch accepted the current state.
    NotConverged,
    /// Maximum iteration reached without an accepted convergence status.
    IterationLimitReached,
}

/// Why the source-compatible convergence controller selected its status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeakOverlapConvergenceReason {
    /// The explicit one-step Coppola compatibility mode.
    OneStepCoppolaShortcut,
    /// Current-to-previous MD2 meets strict tolerance and no CoL adjustment
    /// occurred on this iteration.
    StrictMd2Satisfied,
    /// Current-to-two-back MD2 meets strict tolerance after the oscillation delay.
    TwoCycleResolved,
    /// Current-to-previous MD2 meets the active relaxed MD2 threshold.
    RelaxedMd2Satisfied,
    /// The current iteration is the initial iteration; no previous center exists.
    InitialIteration,
    /// A CoL adjustment was made on this iteration, preventing strict convergence.
    ColAdjustmentPreventsStrictConvergence,
    /// The oscillation branch is not yet eligible.
    OscillationCheckNotYetEligible,
    /// No active MD2 threshold accepted the current change.
    Md2AboveActiveThreshold,
    /// The declared maximum iteration has been reached.
    MaximumIterationsReached,
}

/// Auditable output of one PeakOverlapPos convergence decision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeakOverlapConvergenceDecision {
    /// Typed source-semantic convergence status.
    pub status: PeakOverlapConvergenceStatus,
    /// Branch that selected the status.
    pub reason: PeakOverlapConvergenceReason,
    /// Exact numeric status returned by the pinned CARA source:
    /// 1.0 strict/one-step, 0.5 oscillation, 0.1..0.35 relaxed, 0.0 not converged.
    pub source_convergence_code: f64,
    /// Active relaxed MD2 threshold, absent when no relaxed test was reached.
    pub active_md2_cutoff: Option<f64>,
    /// Strict/relaxed schedule interpolation factor, absent outside relaxed testing.
    pub relaxed_schedule_fraction: Option<f64>,
    /// Current-to-previous squared Mahalanobis change, if available.
    pub current_vs_previous_md2: Option<f64>,
    /// Current-to-two-back squared Mahalanobis change, if the oscillation
    /// branch was evaluated.
    pub current_vs_two_back_md2: Option<f64>,
    /// Derived CARA iteration boundaries for this configuration.
    pub acceleration_iteration: usize,
    pub averaging_iteration: usize,
    pub oscillation_iteration: usize,
}

/// Apply the pinned CARA PeakOverlapPos convergence branch order.
///
/// Branch order is material and is preserved exactly:
/// 1. `maximum_iterations <= 1` => one-step strict status 1.0.
/// 2. Iteration zero => continue; no previous-center comparison exists.
/// 3. Strict MD2 test, unless the current iteration performed a CoL adjustment.
/// 4. Maximum-iteration failure.
/// 5. After the oscillation delay and more than two iterations after the last
///    CoL adjustment, test the two-cycle MD2; if it passes, return 0.5.
/// 6. Otherwise apply the strict-to-relaxed logarithmic tolerance schedule and
///    return `0.1 + omfrc/4` if the current MD2 passes.
/// 7. Otherwise continue with numeric source status 0.0.
///
/// This function does not decide whether the supplied MD2 values were
/// calculated from the correct covariance or event/model identity. That
/// provenance must be bound by the caller's iteration trace.
pub fn decide_peak_overlap_convergence(
    observation: PeakOverlapConvergenceObservation,
    config: PeakOverlapConvergenceConfig,
) -> Result<PeakOverlapConvergenceDecision, KernelError> {
    validate_config(config)?;
    if observation
        .last_col_adjustment_iteration
        .is_some_and(|last_adjustment| last_adjustment > observation.iteration)
        || observation
            .current_vs_two_back_md2
            .is_some_and(|value| !value.is_finite() || value < 0.0)
    {
        return Err(KernelError::InvalidPeakOverlapConvergenceObservation);
    }
    let acceleration_iteration =
        ((0.25 * config.maximum_iterations as f64).round() as usize).min(25);
    let averaging_iteration = ((0.35 * config.maximum_iterations as f64).round() as usize).min(35);
    let oscillation_iteration =
        ((0.15 * config.maximum_iterations as f64).round() as usize).min(15);

    if config.maximum_iterations <= 1 {
        return Ok(decision(
            PeakOverlapConvergenceStatus::CoppolaOneStep,
            PeakOverlapConvergenceReason::OneStepCoppolaShortcut,
            1.0,
            None,
            None,
            None,
            None,
            acceleration_iteration,
            averaging_iteration,
            oscillation_iteration,
        ));
    }

    if observation.iteration == 0 {
        return Ok(decision(
            PeakOverlapConvergenceStatus::NotConverged,
            PeakOverlapConvergenceReason::InitialIteration,
            0.0,
            None,
            None,
            None,
            None,
            acceleration_iteration,
            averaging_iteration,
            oscillation_iteration,
        ));
    }

    let current_vs_previous_md2 = observation
        .current_vs_previous_md2
        .ok_or(KernelError::InvalidPeakOverlapConvergenceObservation)?;
    validate_md2(current_vs_previous_md2)?;

    let adjustment_on_current_iteration =
        observation.last_col_adjustment_iteration == Some(observation.iteration);
    if current_vs_previous_md2 <= config.strict_md2_tolerance && !adjustment_on_current_iteration {
        return Ok(decision(
            PeakOverlapConvergenceStatus::Strict,
            PeakOverlapConvergenceReason::StrictMd2Satisfied,
            1.0,
            Some(config.strict_md2_tolerance),
            None,
            Some(current_vs_previous_md2),
            None,
            acceleration_iteration,
            averaging_iteration,
            oscillation_iteration,
        ));
    }

    if observation.iteration >= config.maximum_iterations {
        return Ok(decision(
            PeakOverlapConvergenceStatus::IterationLimitReached,
            PeakOverlapConvergenceReason::MaximumIterationsReached,
            0.0,
            None,
            None,
            Some(current_vs_previous_md2),
            None,
            acceleration_iteration,
            averaging_iteration,
            oscillation_iteration,
        ));
    }

    let oscillation_eligible = observation.iteration > oscillation_iteration
        && observation
            .last_col_adjustment_iteration
            .is_none_or(|last_adjustment| {
                observation.iteration > last_adjustment.saturating_add(2)
            });

    if !oscillation_eligible {
        let reason = if adjustment_on_current_iteration {
            PeakOverlapConvergenceReason::ColAdjustmentPreventsStrictConvergence
        } else {
            PeakOverlapConvergenceReason::OscillationCheckNotYetEligible
        };
        return Ok(decision(
            PeakOverlapConvergenceStatus::NotConverged,
            reason,
            0.0,
            None,
            None,
            Some(current_vs_previous_md2),
            None,
            acceleration_iteration,
            averaging_iteration,
            oscillation_iteration,
        ));
    }

    let current_vs_two_back_md2 = observation
        .current_vs_two_back_md2
        .ok_or(KernelError::InvalidPeakOverlapConvergenceObservation)?;
    validate_md2(current_vs_two_back_md2)?;

    if current_vs_two_back_md2 <= config.strict_md2_tolerance {
        return Ok(decision(
            PeakOverlapConvergenceStatus::OscillationResolved,
            PeakOverlapConvergenceReason::TwoCycleResolved,
            0.5,
            Some(config.strict_md2_tolerance),
            None,
            Some(current_vs_previous_md2),
            Some(current_vs_two_back_md2),
            acceleration_iteration,
            averaging_iteration,
            oscillation_iteration,
        ));
    }

    let (active_md2_cutoff, omfrc, schedule_fraction) = if observation.iteration
        > averaging_iteration
    {
        (config.relaxed_md2_tolerance, 0.0, None)
    } else if observation.iteration <= acceleration_iteration {
        (config.strict_md2_tolerance, 1.0, None)
    } else {
        // For a valid integer iteration, this branch implies
        // averaging_iteration > acceleration_iteration.
        let denominator = averaging_iteration - acceleration_iteration;
        if denominator == 0 {
            return Err(KernelError::InvalidPeakOverlapConvergenceConfig);
        }
        let fraction = (observation.iteration - acceleration_iteration) as f64 / denominator as f64;
        let relaxed_fraction = 1.0 - fraction;
        let cutoff = (relaxed_fraction * config.strict_md2_tolerance.ln()
            + fraction * config.relaxed_md2_tolerance.ln())
        .exp();
        (cutoff, relaxed_fraction, Some(fraction))
    };

    if current_vs_previous_md2 <= active_md2_cutoff {
        let convergence_code = 0.1 + omfrc / 4.0;
        return Ok(decision(
            PeakOverlapConvergenceStatus::Relaxed,
            PeakOverlapConvergenceReason::RelaxedMd2Satisfied,
            convergence_code,
            Some(active_md2_cutoff),
            schedule_fraction,
            Some(current_vs_previous_md2),
            Some(current_vs_two_back_md2),
            acceleration_iteration,
            averaging_iteration,
            oscillation_iteration,
        ));
    }

    Ok(decision(
        PeakOverlapConvergenceStatus::NotConverged,
        PeakOverlapConvergenceReason::Md2AboveActiveThreshold,
        0.0,
        Some(active_md2_cutoff),
        schedule_fraction,
        Some(current_vs_previous_md2),
        Some(current_vs_two_back_md2),
        acceleration_iteration,
        averaging_iteration,
        oscillation_iteration,
    ))
}

fn decision(
    status: PeakOverlapConvergenceStatus,
    reason: PeakOverlapConvergenceReason,
    source_convergence_code: f64,
    active_md2_cutoff: Option<f64>,
    relaxed_schedule_fraction: Option<f64>,
    current_vs_previous_md2: Option<f64>,
    current_vs_two_back_md2: Option<f64>,
    acceleration_iteration: usize,
    averaging_iteration: usize,
    oscillation_iteration: usize,
) -> PeakOverlapConvergenceDecision {
    PeakOverlapConvergenceDecision {
        status,
        reason,
        source_convergence_code,
        active_md2_cutoff,
        relaxed_schedule_fraction,
        current_vs_previous_md2,
        current_vs_two_back_md2,
        acceleration_iteration,
        averaging_iteration,
        oscillation_iteration,
    }
}

fn validate_md2(value: f64) -> Result<(), KernelError> {
    if !value.is_finite() || value < 0.0 {
        Err(KernelError::InvalidPeakOverlapConvergenceObservation)
    } else {
        Ok(())
    }
}

fn validate_config(config: PeakOverlapConvergenceConfig) -> Result<(), KernelError> {
    if !config.strict_md2_tolerance.is_finite()
        || config.strict_md2_tolerance <= 0.0
        || !config.relaxed_md2_tolerance.is_finite()
        || config.relaxed_md2_tolerance < config.strict_md2_tolerance
    {
        return Err(KernelError::InvalidPeakOverlapConvergenceConfig);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> PeakOverlapConvergenceConfig {
        PeakOverlapConvergenceConfig::default()
    }

    fn observation(iteration: usize, md2: f64) -> PeakOverlapConvergenceObservation {
        PeakOverlapConvergenceObservation {
            iteration,
            current_vs_previous_md2: (iteration > 0).then_some(md2),
            current_vs_two_back_md2: None,
            last_col_adjustment_iteration: None,
        }
    }

    #[test]
    fn default_configuration_reproduces_pinned_iteration_boundaries() {
        let value = config();
        let result = decide_peak_overlap_convergence(observation(1, 1.0), value).unwrap();
        assert_eq!(result.acceleration_iteration, 25);
        assert_eq!(result.averaging_iteration, 35);
        assert_eq!(result.oscillation_iteration, 15);
    }

    #[test]
    fn maximum_iterations_one_uses_strict_coppola_shortcut() {
        let value = PeakOverlapConvergenceConfig {
            maximum_iterations: 1,
            ..config()
        };
        let result = decide_peak_overlap_convergence(observation(0, 100.0), value).unwrap();
        assert_eq!(result.status, PeakOverlapConvergenceStatus::CoppolaOneStep);
        assert_eq!(result.source_convergence_code, 1.0);
    }

    #[test]
    fn maximum_iterations_zero_preserves_source_shortcut_behavior() {
        let value = PeakOverlapConvergenceConfig {
            maximum_iterations: 0,
            ..config()
        };
        let result =
            decide_peak_overlap_convergence(observation(0, 100.0), value).unwrap();
        assert_eq!(result.status, PeakOverlapConvergenceStatus::CoppolaOneStep);
    }

    #[test]
    fn initial_iteration_does_not_require_previous_md2() {
        let result = decide_peak_overlap_convergence(observation(0, 0.0), config()).unwrap();
        assert_eq!(result.status, PeakOverlapConvergenceStatus::NotConverged);
        assert_eq!(
            result.reason,
            PeakOverlapConvergenceReason::InitialIteration
        );
        assert_eq!(result.source_convergence_code, 0.0);
    }

    #[test]
    fn strict_tolerance_returns_numeric_one_when_no_adjustment_occurred() {
        let result = decide_peak_overlap_convergence(observation(1, 0.5e-12), config()).unwrap();
        assert_eq!(result.status, PeakOverlapConvergenceStatus::Strict);
        assert_eq!(result.source_convergence_code, 1.0);
        assert_eq!(result.active_md2_cutoff, Some(1.0e-12));
    }

    #[test]
    fn strict_convergence_is_forbidden_on_the_col_adjustment_iteration() {
        let mut input = observation(20, 0.5e-12);
        input.last_col_adjustment_iteration = Some(20);
        let result = decide_peak_overlap_convergence(input, config()).unwrap();
        assert_eq!(result.status, PeakOverlapConvergenceStatus::NotConverged);
        assert_eq!(
            result.reason,
            PeakOverlapConvergenceReason::ColAdjustmentPreventsStrictConvergence
        );
        assert_eq!(result.source_convergence_code, 0.0);
    }

    #[test]
    fn maximum_iteration_failure_occurs_after_the_strict_check() {
        let mut input = observation(100, 2.0e-12);
        let result = decide_peak_overlap_convergence(input, config()).unwrap();
        assert_eq!(
            result.status,
            PeakOverlapConvergenceStatus::IterationLimitReached
        );
        assert_eq!(result.source_convergence_code, 0.0);

        input.current_vs_previous_md2 = Some(0.5e-12);
        let strict = decide_peak_overlap_convergence(input, config()).unwrap();
        assert_eq!(strict.status, PeakOverlapConvergenceStatus::Strict);
    }

    #[test]
    fn two_cycle_oscillation_returns_numeric_half_after_delay() {
        let mut input = observation(16, 1.0e-4);
        input.current_vs_two_back_md2 = Some(0.5e-12);
        let result = decide_peak_overlap_convergence(input, config()).unwrap();
        assert_eq!(
            result.status,
            PeakOverlapConvergenceStatus::OscillationResolved
        );
        assert_eq!(result.source_convergence_code, 0.5);
    }

    #[test]
    fn oscillation_check_is_not_run_until_iteration_exceeds_boundary() {
        let mut input = observation(15, 1.0e-4);
        input.current_vs_two_back_md2 = Some(0.0);
        let result = decide_peak_overlap_convergence(input, config()).unwrap();
        assert_eq!(result.status, PeakOverlapConvergenceStatus::NotConverged);
        assert_eq!(
            result.reason,
            PeakOverlapConvergenceReason::OscillationCheckNotYetEligible
        );
        assert_eq!(result.current_vs_two_back_md2, None);
    }

    #[test]
    fn oscillation_check_waits_more_than_two_iterations_after_col_adjustment() {
        let mut input = observation(20, 1.0e-4);
        input.last_col_adjustment_iteration = Some(18);
        input.current_vs_two_back_md2 = Some(0.0);
        let at_two = decide_peak_overlap_convergence(input, config()).unwrap();
        assert_eq!(at_two.status, PeakOverlapConvergenceStatus::NotConverged);

        input.iteration = 21;
        let after_two = decide_peak_overlap_convergence(input, config()).unwrap();
        assert_eq!(
            after_two.status,
            PeakOverlapConvergenceStatus::OscillationResolved
        );
    }

    #[test]
    fn interpolation_midpoint_uses_geometric_tolerance_mean() {
        let mut input = observation(30, 5.0e-10);
        input.current_vs_two_back_md2 = Some(1.0e-6);
        let result = decide_peak_overlap_convergence(input, config()).unwrap();
        assert_eq!(result.status, PeakOverlapConvergenceStatus::Relaxed);
        assert!((result.active_md2_cutoff.unwrap() - 1.0e-9).abs() < 1.0e-22);
        assert!((result.source_convergence_code - 0.225).abs() < 1.0e-15);
        assert_eq!(result.relaxed_schedule_fraction, Some(0.5));
    }

    #[test]
    fn late_iterations_use_relaxed_tolerance_and_numeric_point_one() {
        let mut input = observation(36, 5.0e-7);
        input.current_vs_two_back_md2 = Some(1.0e-4);
        let result = decide_peak_overlap_convergence(input, config()).unwrap();
        assert_eq!(result.status, PeakOverlapConvergenceStatus::Relaxed);
        assert_eq!(result.active_md2_cutoff, Some(1.0e-6));
        assert_eq!(result.source_convergence_code, 0.1);
    }

    #[test]
    fn adjustment_gate_does_not_get_collapsed_when_two_cycle_is_available() {
        let mut input = observation(22, 2.0e-4);
        input.last_col_adjustment_iteration = Some(20);
        input.current_vs_two_back_md2 = Some(0.2e-12);
        let not_yet_eligible = decide_peak_overlap_convergence(input, config()).unwrap();
        assert_eq!(
            not_yet_eligible.status,
            PeakOverlapConvergenceStatus::NotConverged
        );

        input.iteration = 23;
        let result = decide_peak_overlap_convergence(input, config()).unwrap();
        assert_eq!(
            result.status,
            PeakOverlapConvergenceStatus::OscillationResolved
        );
        assert_eq!(result.source_convergence_code, 0.5);
    }

    #[test]
    fn eligible_oscillation_branch_requires_two_back_md2() {
        let input = observation(16, 1.0e-4);
        assert_eq!(
            decide_peak_overlap_convergence(input, config()),
            Err(KernelError::InvalidPeakOverlapConvergenceObservation)
        );
    }

    #[test]
    fn future_col_adjustment_iteration_is_rejected() {
        let mut input = observation(10, 1.0e-4);
        input.last_col_adjustment_iteration = Some(11);
        assert_eq!(
            decide_peak_overlap_convergence(input, config()),
            Err(KernelError::InvalidPeakOverlapConvergenceObservation)
        );
    }

    #[test]
    fn invalid_optional_two_back_md2_is_rejected_even_before_oscillation_window() {
        let mut input = observation(1, 1.0e-4);
        input.current_vs_two_back_md2 = Some(f64::NAN);
        assert_eq!(
            decide_peak_overlap_convergence(input, config()),
            Err(KernelError::InvalidPeakOverlapConvergenceObservation)
        );
    }

    #[test]
    fn negative_nonfinite_md2_and_invalid_tolerances_fail_closed() {
        let mut input = observation(1, -1.0);
        assert_eq!(
            decide_peak_overlap_convergence(input, config()),
            Err(KernelError::InvalidPeakOverlapConvergenceObservation)
        );
        input = observation(1, f64::NAN);
        assert_eq!(
            decide_peak_overlap_convergence(input, config()),
            Err(KernelError::InvalidPeakOverlapConvergenceObservation)
        );
        assert_eq!(
            decide_peak_overlap_convergence(
                observation(1, 1.0),
                PeakOverlapConvergenceConfig {
                    strict_md2_tolerance: 0.0,
                    ..config()
                }
            ),
            Err(KernelError::InvalidPeakOverlapConvergenceConfig)
        );
    }
}
