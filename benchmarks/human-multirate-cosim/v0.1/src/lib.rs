//! Synthetic-only reference kernel for comparing co-simulation coupling schemes.
//!
//! This crate uses an analytically solvable two-compartment exchange system.
//! All parameters and states are arbitrary mathematical quantities. They are
//! not physiological parameters, observations, or human-subject data.
//!
//! The implicit methods are monolithic numerical reference integrators for
//! this closed-form test problem. They are not presented as generic FMI masters.

use std::fmt;

/// Arbitrary two-compartment exchange system:
///
///   C_a * dx_a/dt = -G * (x_a - x_b)
///   C_b * dx_b/dt =  G * (x_a - x_b)
///
/// The capacity-weighted total C_a*x_a + C_b*x_b is conserved exactly by
/// the continuous equations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scenario {
    pub name: &'static str,
    pub capacity_a: f64,
    pub capacity_b: f64,
    pub conductance: f64,
    pub initial_a: f64,
    pub initial_b: f64,
    pub horizon: f64,
}

impl Scenario {
    pub fn validate(self) -> Result<(), SimError> {
        for (name, value) in [
            ("capacity_a", self.capacity_a),
            ("capacity_b", self.capacity_b),
            ("conductance", self.conductance),
            ("initial_a", self.initial_a),
            ("initial_b", self.initial_b),
            ("horizon", self.horizon),
        ] {
            if !value.is_finite() {
                return Err(SimError::NonFiniteParameter(name));
            }
        }
        if self.capacity_a <= 0.0 || self.capacity_b <= 0.0 {
            return Err(SimError::NonPositiveCapacity);
        }
        if self.conductance <= 0.0 {
            return Err(SimError::NonPositiveConductance);
        }
        if self.horizon <= 0.0 {
            return Err(SimError::NonPositiveHorizon);
        }
        Ok(())
    }

    pub fn coupling_rate(self) -> f64 {
        self.conductance * (1.0 / self.capacity_a + 1.0 / self.capacity_b)
    }

    pub fn conserved_total(self) -> f64 {
        self.capacity_a * self.initial_a + self.capacity_b * self.initial_b
    }

    /// Exact continuous-time solution from the frozen initial state.
    pub fn exact_state(self, time: f64) -> (f64, f64) {
        let capacity_sum = self.capacity_a + self.capacity_b;
        let weighted_mean = self.conserved_total() / capacity_sum;
        let initial_difference = self.initial_a - self.initial_b;
        let difference = initial_difference * (-self.coupling_rate() * time).exp();

        let a = weighted_mean + self.capacity_b / capacity_sum * difference;
        let b = weighted_mean - self.capacity_a / capacity_sum * difference;
        (a, b)
    }
}

/// Coupling/integration candidates in the benchmark tournament.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// Both components advance from the same stale communication-point snapshot.
    ParallelZeroOrderHold,
    /// Component A advances first; B then holds the updated A value.
    SerialAThenB,
    /// Component B advances first; A then holds the updated B value.
    SerialBThenA,
    /// One shared interface flux is debited and credited exactly once.
    ConservativeSharedFluxEuler,
    /// Monolithic implicit-midpoint reference for this linear system.
    ImplicitMidpointReference,
    /// Monolithic backward-Euler reference for this linear system.
    BackwardEulerReference,
}

impl Method {
    pub const ALL: [Self; 6] = [
        Self::ParallelZeroOrderHold,
        Self::SerialAThenB,
        Self::SerialBThenA,
        Self::ConservativeSharedFluxEuler,
        Self::ImplicitMidpointReference,
        Self::BackwardEulerReference,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ParallelZeroOrderHold => "parallel_zero_order_hold",
            Self::SerialAThenB => "serial_a_then_b",
            Self::SerialBThenA => "serial_b_then_a",
            Self::ConservativeSharedFluxEuler => "conservative_shared_flux_euler",
            Self::ImplicitMidpointReference => "implicit_midpoint_reference",
            Self::BackwardEulerReference => "backward_euler_reference",
        }
    }
}

/// Score vector. These dimensions remain separate; no composite "realism" score.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Score {
    pub steps: usize,
    pub max_abs_state_error: f64,
    pub rms_state_error: f64,
    pub max_abs_balance_residual: f64,
    pub min_state: f64,
    pub max_state: f64,
    pub final_a: f64,
    pub final_b: f64,
    pub positive_states: bool,
    pub explicit_euler_stability_number: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimError {
    InvalidAdaptiveConfig,
    AdaptiveStepUnderflow,
    AdaptiveAttemptLimitReached,
    NonFiniteParameter(&'static str),
    NonPositiveCapacity,
    NonPositiveConductance,
    NonPositiveHorizon,
    InvalidStep,
    StepDoesNotDivideHorizon,
    ExcessiveStepCount,
    NonFiniteState { step: usize },
}

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAdaptiveConfig => {
                write!(f, "adaptive step/tolerance configuration is invalid")
            }
            Self::AdaptiveStepUnderflow => {
                write!(f, "adaptive solver reached the minimum step without an acceptable step")
            }
            Self::AdaptiveAttemptLimitReached => {
                write!(f, "adaptive solver exceeded its attempt limit")
            }
            Self::NonFiniteParameter(name) => write!(f, "parameter {name} must be finite"),
            Self::NonPositiveCapacity => write!(f, "both capacities must be positive"),
            Self::NonPositiveConductance => write!(f, "conductance must be positive"),
            Self::NonPositiveHorizon => write!(f, "horizon must be positive"),
            Self::InvalidStep => write!(f, "communication step must be finite and positive"),
            Self::StepDoesNotDivideHorizon => {
                write!(f, "communication step must divide the horizon exactly")
            }
            Self::ExcessiveStepCount => write!(f, "step count exceeds the safety limit"),
            Self::NonFiniteState { step } => {
                write!(f, "non-finite state produced at integration step {step}")
            }
        }
    }
}

impl std::error::Error for SimError {}

/// Run one method against the analytic solution using a fixed communication step.
///
/// Failed input validation occurs before any local integration state is advanced.
pub fn simulate(
    scenario: Scenario,
    method: Method,
    communication_step: f64,
) -> Result<Score, SimError> {
    scenario.validate()?;
    if !communication_step.is_finite() || communication_step <= 0.0 {
        return Err(SimError::InvalidStep);
    }

    let raw_steps = scenario.horizon / communication_step;
    let rounded_steps = raw_steps.round();
    let tolerance = 1.0e-10 * raw_steps.abs().max(1.0);
    if !raw_steps.is_finite() || (raw_steps - rounded_steps).abs() > tolerance {
        return Err(SimError::StepDoesNotDivideHorizon);
    }
    if rounded_steps < 1.0 || rounded_steps > 10_000_000.0 {
        return Err(SimError::ExcessiveStepCount);
    }

    let steps = rounded_steps as usize;
    let mut a = scenario.initial_a;
    let mut b = scenario.initial_b;
    let initial_total = scenario.conserved_total();

    let mut max_abs_state_error: f64 = 0.0;
    let mut sum_squared_error = 0.0;
    let mut error_samples = 0_usize;
    let mut max_abs_balance_residual: f64 = 0.0;
    let mut min_state = a.min(b);
    let mut max_state = a.max(b);

    for index in 0..steps {
        let (next_a, next_b) = match method {
            Method::ParallelZeroOrderHold => {
                let decay_a =
                    (-scenario.conductance * communication_step / scenario.capacity_a).exp();
                let decay_b =
                    (-scenario.conductance * communication_step / scenario.capacity_b).exp();

                // Deliberately both use the same old a,b snapshot.
                (b + (a - b) * decay_a, a + (b - a) * decay_b)
            }
            Method::SerialAThenB => {
                let decay_a =
                    (-scenario.conductance * communication_step / scenario.capacity_a).exp();
                let decay_b =
                    (-scenario.conductance * communication_step / scenario.capacity_b).exp();
                let updated_a = b + (a - b) * decay_a;
                let updated_b = updated_a + (b - updated_a) * decay_b;
                (updated_a, updated_b)
            }
            Method::SerialBThenA => {
                let decay_a =
                    (-scenario.conductance * communication_step / scenario.capacity_a).exp();
                let decay_b =
                    (-scenario.conductance * communication_step / scenario.capacity_b).exp();
                let updated_b = a + (b - a) * decay_b;
                let updated_a = updated_b + (a - updated_b) * decay_a;
                (updated_a, updated_b)
            }
            Method::ConservativeSharedFluxEuler => {
                // One integrated interface transfer, paired debit and credit.
                let shared_flux = scenario.conductance * (a - b) * communication_step;
                (
                    a - shared_flux / scenario.capacity_a,
                    b + shared_flux / scenario.capacity_b,
                )
            }
            Method::ImplicitMidpointReference => {
                let difference = a - b;
                let rate = scenario.coupling_rate();
                let amplification = (1.0 - 0.5 * rate * communication_step)
                    / (1.0 + 0.5 * rate * communication_step);
                let next_difference = difference * amplification;
                let capacity_sum = scenario.capacity_a + scenario.capacity_b;
                let weighted_mean = initial_total / capacity_sum;
                (
                    weighted_mean + scenario.capacity_b / capacity_sum * next_difference,
                    weighted_mean - scenario.capacity_a / capacity_sum * next_difference,
                )
            }
            Method::BackwardEulerReference => {
                let difference = a - b;
                let next_difference =
                    difference / (1.0 + scenario.coupling_rate() * communication_step);
                let capacity_sum = scenario.capacity_a + scenario.capacity_b;
                let weighted_mean = initial_total / capacity_sum;
                (
                    weighted_mean + scenario.capacity_b / capacity_sum * next_difference,
                    weighted_mean - scenario.capacity_a / capacity_sum * next_difference,
                )
            }
        };

        if !next_a.is_finite() || !next_b.is_finite() {
            return Err(SimError::NonFiniteState { step: index + 1 });
        }

        a = next_a;
        b = next_b;
        let time = (index + 1) as f64 * communication_step;
        let (exact_a, exact_b) = scenario.exact_state(time);

        for (actual, expected) in [(a, exact_a), (b, exact_b)] {
            let error = actual - expected;
            max_abs_state_error = max_abs_state_error.max(error.abs());
            sum_squared_error += error * error;
            error_samples += 1;
        }

        let residual = (scenario.capacity_a * a + scenario.capacity_b * b - initial_total).abs();
        max_abs_balance_residual = max_abs_balance_residual.max(residual);
        min_state = min_state.min(a).min(b);
        max_state = max_state.max(a).max(b);
    }

    let rms_state_error = (sum_squared_error / error_samples as f64).sqrt();

    Ok(Score {
        steps,
        max_abs_state_error,
        rms_state_error,
        max_abs_balance_residual,
        min_state,
        max_state,
        final_a: a,
        final_b: b,
        positive_states: min_state >= 0.0,
        explicit_euler_stability_number: scenario.coupling_rate() * communication_step,
    })
}


/// Controls for adaptive conservative shared-flux Euler subcycling.
///
/// Tolerances bound the local step-doubling estimate, not the global trajectory
/// error. The latter is measured independently against the analytic oracle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdaptiveConfig {
    pub initial_step: f64,
    pub min_step: f64,
    pub max_step: f64,
    pub absolute_tolerance: f64,
    pub relative_tolerance: f64,
    pub max_attempts: usize,
}

/// Score vector for the adaptive conservative method.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdaptiveScore {
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub max_accepted_local_error_norm: f64,
    pub max_abs_state_error: f64,
    pub rms_state_error: f64,
    pub max_abs_balance_residual: f64,
    pub min_state: f64,
    pub max_state: f64,
    pub min_accepted_step: f64,
    pub max_accepted_step: f64,
    pub positive_states: bool,
}

/// One explicit Euler step using one shared interface transfer.
fn shared_flux_euler(
    scenario: Scenario,
    a: f64,
    b: f64,
    step: f64,
) -> (f64, f64) {
    let flux = scenario.conductance * (a - b) * step;
    (
        a - flux / scenario.capacity_a,
        b + flux / scenario.capacity_b,
    )
}

/// Adaptive first-order subcycling with step-doubling error estimation.
///
/// A full step is compared with two half steps. Accepted state is the two-half
/// result. One interface flux is paired as a source debit and destination
/// credit for every substep, preserving the capacity-weighted invariant.
///
/// This is a benchmark reference for this two-compartment system, not a generic
/// FMU algorithm. Error tolerances apply to the local step-doubling estimate;
/// global error is reported separately against the analytic solution.
pub fn simulate_adaptive_conservative_flux(
    scenario: Scenario,
    config: AdaptiveConfig,
) -> Result<AdaptiveScore, SimError> {
    scenario.validate()?;
    if !config.initial_step.is_finite()
        || !config.min_step.is_finite()
        || !config.max_step.is_finite()
        || !config.absolute_tolerance.is_finite()
        || !config.relative_tolerance.is_finite()
        || config.min_step <= 0.0
        || config.max_step < config.min_step
        || config.initial_step < config.min_step
        || config.initial_step > config.max_step
        || config.absolute_tolerance <= 0.0
        || config.relative_tolerance < 0.0
        || config.max_attempts == 0
    {
        return Err(SimError::InvalidAdaptiveConfig);
    }

    let initial_total = scenario.conserved_total();
    let positivity_required = scenario.initial_a >= 0.0 && scenario.initial_b >= 0.0;
    let mut time = 0.0;
    let mut a = scenario.initial_a;
    let mut b = scenario.initial_b;
    let mut step = config.initial_step;

    let mut accepted_steps = 0_usize;
    let mut rejected_steps = 0_usize;
    let mut attempts = 0_usize;
    let mut max_accepted_local_error_norm: f64 = 0.0;
    let mut max_abs_state_error: f64 = 0.0;
    let mut sum_squared_error = 0.0;
    let mut error_samples = 0_usize;
    let mut max_abs_balance_residual: f64 = 0.0;
    let mut min_state = a.min(b);
    let mut max_state = a.max(b);
    let mut min_accepted_step = f64::INFINITY;
    let mut max_accepted_step: f64 = 0.0;

    while time < scenario.horizon {
        if attempts >= config.max_attempts {
            return Err(SimError::AdaptiveAttemptLimitReached);
        }
        attempts += 1;

        let remaining = scenario.horizon - time;
        let attempted_step = step.min(remaining);
        if !attempted_step.is_finite() || attempted_step <= 0.0 {
            return Err(SimError::AdaptiveStepUnderflow);
        }

        let coarse = shared_flux_euler(scenario, a, b, attempted_step);
        let first_half = shared_flux_euler(scenario, a, b, attempted_step * 0.5);
        let fine = shared_flux_euler(
            scenario,
            first_half.0,
            first_half.1,
            attempted_step * 0.5,
        );

        let finite = coarse.0.is_finite()
            && coarse.1.is_finite()
            && fine.0.is_finite()
            && fine.1.is_finite();

        let error_norm = if finite {
            let scale_a = config.absolute_tolerance
                + config.relative_tolerance * a.abs().max(fine.0.abs());
            let scale_b = config.absolute_tolerance
                + config.relative_tolerance * b.abs().max(fine.1.abs());
            ((fine.0 - coarse.0).abs() / scale_a)
                .max((fine.1 - coarse.1).abs() / scale_b)
        } else {
            f64::INFINITY
        };

        let positivity_ok = !positivity_required
            || (fine.0 >= 0.0 && fine.1 >= 0.0);
        let accepted = finite && error_norm <= 1.0 && positivity_ok;

        if accepted {
            a = fine.0;
            b = fine.1;
            time += attempted_step;
            accepted_steps += 1;
            max_accepted_local_error_norm =
                max_accepted_local_error_norm.max(error_norm);
            min_accepted_step = min_accepted_step.min(attempted_step);
            max_accepted_step = max_accepted_step.max(attempted_step);

            let (exact_a, exact_b) = scenario.exact_state(time);
            for (actual, expected) in [(a, exact_a), (b, exact_b)] {
                let error = actual - expected;
                max_abs_state_error = max_abs_state_error.max(error.abs());
                sum_squared_error += error * error;
                error_samples += 1;
            }

            let residual =
                (scenario.capacity_a * a + scenario.capacity_b * b - initial_total).abs();
            max_abs_balance_residual = max_abs_balance_residual.max(residual);
            min_state = min_state.min(a).min(b);
            max_state = max_state.max(a).max(b);

            let factor = if error_norm < 1.0e-16 {
                2.0
            } else {
                (0.9 / error_norm.sqrt()).clamp(0.2, 2.0)
            };
            step = (attempted_step * factor).clamp(config.min_step, config.max_step);
        } else {
            rejected_steps += 1;
            if attempted_step <= config.min_step * (1.0 + 1.0e-12) {
                return Err(SimError::AdaptiveStepUnderflow);
            }
            let factor = if error_norm.is_finite() && error_norm > 0.0 {
                (0.9 / error_norm.sqrt()).clamp(0.1, 0.5)
            } else {
                0.5
            };
            step = (attempted_step * factor).max(config.min_step);
        }
    }

    let rms_state_error = (sum_squared_error / error_samples as f64).sqrt();

    Ok(AdaptiveScore {
        accepted_steps,
        rejected_steps,
        max_accepted_local_error_norm,
        max_abs_state_error,
        rms_state_error,
        max_abs_balance_residual,
        min_state,
        max_state,
        min_accepted_step,
        max_accepted_step,
        positive_states: min_state >= 0.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asymmetric() -> Scenario {
        Scenario {
            name: "asymmetric",
            capacity_a: 1.0,
            capacity_b: 4.0,
            conductance: 0.5,
            initial_a: 1.0,
            initial_b: 0.0,
            horizon: 8.0,
        }
    }

    fn assert_small(value: f64, tolerance: f64) {
        assert!(
            value.abs() <= tolerance,
            "value {value} exceeds {tolerance}"
        );
    }

    #[test]
    fn analytic_solution_preserves_capacity_weighted_total() {
        let scenario = asymmetric();
        let initial = scenario.conserved_total();
        for t in [0.0, 0.1, 1.0, 4.0, 8.0, 100.0] {
            let (a, b) = scenario.exact_state(t);
            assert_small(
                scenario.capacity_a * a + scenario.capacity_b * b - initial,
                1.0e-12,
            );
        }
    }

    #[test]
    fn shared_flux_euler_preserves_total_even_when_step_is_inaccurate() {
        let score = simulate(asymmetric(), Method::ConservativeSharedFluxEuler, 4.0).unwrap();
        assert_small(score.max_abs_balance_residual, 1.0e-12);
        assert!(
            !score.positive_states,
            "large explicit step should expose positivity loss"
        );
    }

    #[test]
    fn parallel_and_serial_staggering_expose_balance_leakage() {
        for method in [
            Method::ParallelZeroOrderHold,
            Method::SerialAThenB,
            Method::SerialBThenA,
        ] {
            let score = simulate(asymmetric(), method, 1.0).unwrap();
            assert!(score.max_abs_balance_residual > 1.0e-4);
        }
    }

    #[test]
    fn serial_component_order_changes_the_result() {
        let a_first = simulate(asymmetric(), Method::SerialAThenB, 1.0).unwrap();
        let b_first = simulate(asymmetric(), Method::SerialBThenA, 1.0).unwrap();
        assert!(
            (a_first.final_a - b_first.final_a).abs() > 1.0e-5
                || (a_first.final_b - b_first.final_b).abs() > 1.0e-5
        );
    }

    #[test]
    fn midpoint_and_backward_euler_preserve_the_invariant() {
        for method in [
            Method::ImplicitMidpointReference,
            Method::BackwardEulerReference,
        ] {
            let score = simulate(asymmetric(), method, 2.0).unwrap();
            assert_small(score.max_abs_balance_residual, 1.0e-12);
        }
    }

    #[test]
    fn midpoint_error_decreases_under_step_refinement() {
        let coarse = simulate(asymmetric(), Method::ImplicitMidpointReference, 1.0).unwrap();
        let fine = simulate(asymmetric(), Method::ImplicitMidpointReference, 0.125).unwrap();
        assert!(fine.max_abs_state_error < coarse.max_abs_state_error);
    }

    #[test]
    fn backward_euler_remains_positive_for_a_large_step() {
        let score = simulate(asymmetric(), Method::BackwardEulerReference, 8.0).unwrap();
        assert!(score.positive_states);
        assert!(score.final_a >= 0.0 && score.final_b >= 0.0);
    }

    #[test]
    fn explicit_euler_stability_number_identifies_the_linear_limit() {
        let scenario = asymmetric();
        let below = simulate(scenario, Method::ConservativeSharedFluxEuler, 2.0).unwrap();
        let above = simulate(scenario, Method::ConservativeSharedFluxEuler, 4.0).unwrap();
        assert!(below.explicit_euler_stability_number < 2.0);
        assert!(above.explicit_euler_stability_number > 2.0);
    }

    #[test]
    fn invalid_step_is_rejected() {
        assert_eq!(
            simulate(asymmetric(), Method::BackwardEulerReference, f64::NAN),
            Err(SimError::InvalidStep)
        );
    }

    #[test]
    fn invalid_scenario_is_rejected_before_simulation() {
        let mut scenario = asymmetric();
        scenario.capacity_a = 0.0;
        assert_eq!(
            simulate(scenario, Method::BackwardEulerReference, 1.0),
            Err(SimError::NonPositiveCapacity)
        );
    }

    #[test]
    fn non_dividing_communication_step_is_rejected() {
        assert_eq!(
            simulate(asymmetric(), Method::BackwardEulerReference, 0.3),
            Err(SimError::StepDoesNotDivideHorizon)
        );
    }

    #[test]
    fn adaptive_flux_preserves_balance_and_rejects_large_initial_steps() {
        let scenario = asymmetric();
        let score = simulate_adaptive_conservative_flux(
            scenario,
            AdaptiveConfig {
                initial_step: 8.0,
                min_step: 1.0e-8,
                max_step: 8.0,
                absolute_tolerance: 1.0e-7,
                relative_tolerance: 1.0e-6,
                max_attempts: 100_000,
            },
        )
        .unwrap();

        assert!(score.accepted_steps > 0);
        assert!(score.rejected_steps > 0);
        assert!(score.positive_states);
        assert_small(score.max_abs_balance_residual, 1.0e-12);
        assert!(score.max_abs_state_error < 5.0e-4);
        assert!(score.max_accepted_local_error_norm <= 1.0);
    }

    #[test]
    fn adaptive_flux_rejects_invalid_tolerances() {
        let result = simulate_adaptive_conservative_flux(
            asymmetric(),
            AdaptiveConfig {
                initial_step: 1.0,
                min_step: 1.0e-4,
                max_step: 2.0,
                absolute_tolerance: 0.0,
                relative_tolerance: 1.0e-6,
                max_attempts: 100,
            },
        );
        assert_eq!(result, Err(SimError::InvalidAdaptiveConfig));
    }


