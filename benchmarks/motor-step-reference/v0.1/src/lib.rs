//! Scalar generalized-coordinate reference for bounded motor impulse steps.
//!
//! Each MotorStepBudget represents exactly one elapsed physics interval. Solver
//! iterations update one accumulated impulse and return incremental impulses;
//! finish() consumes the budget and produces one aggregate mechanical-work
//! receipt. This is a reference oracle, not a production multidimensional
//! constraint solver or an electrical energy model.

use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotorStepError {
    NonFiniteInput,
    NonPositiveTimeStep,
    NegativeMaximumEffort,
    InvalidInverseMass,
    NonFiniteResult,
}

impl Display for MotorStepError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteInput => write!(formatter, "motor step inputs must be finite"),
            Self::NonPositiveTimeStep => write!(formatter, "time step must be positive"),
            Self::NegativeMaximumEffort => write!(formatter, "maximum effort cannot be negative"),
            Self::InvalidInverseMass => {
                write!(formatter, "effective inverse mass must be finite and positive")
            }
            Self::NonFiniteResult => write!(formatter, "motor step result is not finite"),
        }
    }
}

impl Error for MotorStepError {}

/// Result of one projected solver iteration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotorIteration {
    pub delta_impulse: f64,
    pub accumulated_impulse: f64,
    pub relative_velocity_before: f64,
    pub relative_velocity_after: f64,
    pub generalized_mechanical_work_j: f64,
}

/// One aggregate receipt for one motorized generalized coordinate and one step.
///
/// Generalized mechanical work is not electrical consumption or battery
/// recovery. Conversion losses, bus power, regeneration and auxiliary
/// electronics are outside this scalar mechanical-port contract.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotorStepReceipt {
    pub maximum_effort: f64,
    pub duration_s: f64,
    pub maximum_impulse: f64,
    pub total_impulse: f64,
    pub generalized_mechanical_work_j: f64,
    pub solver_iterations: u32,
    pub nonzero_impulse_iterations: u32,
}

/// Accumulated impulse budget for one scalar generalized coordinate and one
/// elapsed interval. Construct a fresh budget once per world step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotorStepBudget {
    maximum_effort: f64,
    duration_s: f64,
    maximum_impulse: f64,
    accumulated_impulse: f64,
    generalized_mechanical_work_j: f64,
    solver_iterations: u32,
    nonzero_impulse_iterations: u32,
}

impl MotorStepBudget {
    /// Create a fresh per-step budget. Effort may be force or torque if its
    /// units match the caller's generalized coordinate.
    pub fn new(maximum_effort: f64, duration_s: f64) -> Result<Self, MotorStepError> {
        if !maximum_effort.is_finite() || !duration_s.is_finite() {
            return Err(MotorStepError::NonFiniteInput);
        }
        if maximum_effort < 0.0 {
            return Err(MotorStepError::NegativeMaximumEffort);
        }
        if duration_s <= 0.0 {
            return Err(MotorStepError::NonPositiveTimeStep);
        }

        let maximum_impulse = maximum_effort * duration_s;
        if !maximum_impulse.is_finite() {
            return Err(MotorStepError::NonFiniteResult);
        }

        Ok(Self {
            maximum_effort,
            duration_s,
            maximum_impulse,
            accumulated_impulse: 0.0,
            generalized_mechanical_work_j: 0.0,
            solver_iterations: 0,
            nonzero_impulse_iterations: 0,
        })
    }

    pub fn maximum_impulse(&self) -> f64 {
        self.maximum_impulse
    }

    pub fn accumulated_impulse(&self) -> f64 {
        self.accumulated_impulse
    }

    /// Project an iterative velocity-servo update into the single step budget.
    ///
    /// effective_inverse_mass is the scalar generalized inverse inertia:
    /// inverse mass for translation or the relevant inverse inertia for a
    /// rotational coordinate. target_relative_velocity may include a
    /// separately qualified position-control bias.
    pub fn apply_velocity_target(
        &mut self,
        target_relative_velocity: f64,
        current_relative_velocity: f64,
        effective_inverse_mass: f64,
    ) -> Result<MotorIteration, MotorStepError> {
        if !target_relative_velocity.is_finite()
            || !current_relative_velocity.is_finite()
            || !effective_inverse_mass.is_finite()
        {
            return Err(MotorStepError::NonFiniteInput);
        }
        if effective_inverse_mass <= 0.0 {
            return Err(MotorStepError::InvalidInverseMass);
        }

        let desired_increment =
            (target_relative_velocity - current_relative_velocity) / effective_inverse_mass;
        if !desired_increment.is_finite() {
            return Err(MotorStepError::NonFiniteResult);
        }

        let candidate_total = self.accumulated_impulse + desired_increment;
        if !candidate_total.is_finite() {
            return Err(MotorStepError::NonFiniteResult);
        }
        let projected_total = candidate_total.clamp(-self.maximum_impulse, self.maximum_impulse);
        let delta_impulse = projected_total - self.accumulated_impulse;
        let velocity_after = current_relative_velocity + effective_inverse_mass * delta_impulse;
        let work = delta_impulse * (current_relative_velocity + velocity_after) * 0.5;

        if !velocity_after.is_finite() || !work.is_finite() {
            return Err(MotorStepError::NonFiniteResult);
        }

        self.accumulated_impulse = projected_total;
        self.generalized_mechanical_work_j += work;
        self.solver_iterations = self.solver_iterations.saturating_add(1);
        if delta_impulse != 0.0 {
            self.nonzero_impulse_iterations =
                self.nonzero_impulse_iterations.saturating_add(1);
        }

        if !self.generalized_mechanical_work_j.is_finite() {
            return Err(MotorStepError::NonFiniteResult);
        }

        Ok(MotorIteration {
            delta_impulse,
            accumulated_impulse: self.accumulated_impulse,
            relative_velocity_before: current_relative_velocity,
            relative_velocity_after: velocity_after,
            generalized_mechanical_work_j: work,
        })
    }

    /// Consume the step budget and emit its single aggregate receipt.
    ///
    /// Moving self into this function prevents a caller from finishing and
    /// emitting a second receipt from the same budget value.
    pub fn finish(self) -> MotorStepReceipt {
        MotorStepReceipt {
            maximum_effort: self.maximum_effort,
            duration_s: self.duration_s,
            maximum_impulse: self.maximum_impulse,
            total_impulse: self.accumulated_impulse,
            generalized_mechanical_work_j: self.generalized_mechanical_work_j,
            solver_iterations: self.solver_iterations,
            nonzero_impulse_iterations: self.nonzero_impulse_iterations,
        }
    }
}

/// Effective scalar inverse mass/inertia for a relative coordinate.
///
/// Static and kinematic endpoints contribute zero inverse mass. A lone
/// dynamic endpoint therefore receives the full relative response.
pub fn effective_inverse_mass(
    inverse_mass_a: f64,
    inverse_mass_b: f64,
) -> Result<f64, MotorStepError> {
    if !inverse_mass_a.is_finite() || !inverse_mass_b.is_finite() {
        return Err(MotorStepError::NonFiniteInput);
    }
    if inverse_mass_a < 0.0 || inverse_mass_b < 0.0 {
        return Err(MotorStepError::InvalidInverseMass);
    }
    let effective = inverse_mass_a + inverse_mass_b;
    if !effective.is_finite() || effective <= 0.0 {
        return Err(MotorStepError::InvalidInverseMass);
    }
    Ok(effective)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual:.15e}, expected {expected:.15e}, tolerance {tolerance:.3e}"
        );
    }

    fn run_iterations(iterations: u32) -> MotorStepReceipt {
        let mut budget = MotorStepBudget::new(100.0, 0.1).expect("valid budget");
        let inverse_mass = 0.5;
        let mut velocity = 0.0;
        for _ in 0..iterations {
            let result = budget
                .apply_velocity_target(3.0, velocity, inverse_mass)
                .expect("valid iteration");
            velocity = result.relative_velocity_after;
        }
        budget.finish()
    }

    #[test]
    fn iteration_count_does_not_multiply_motor_impulse_or_work() {
        let reference = run_iterations(1);
        for iterations in [2, 4, 8] {
            let result = run_iterations(iterations);
            close(result.total_impulse, reference.total_impulse, 1.0e-12);
            close(
                result.generalized_mechanical_work_j,
                reference.generalized_mechanical_work_j,
                1.0e-12,
            );
            assert_eq!(result.solver_iterations, iterations);
            assert_eq!(result.nonzero_impulse_iterations, 1);
        }
        close(reference.total_impulse, 6.0, 1.0e-12);
        close(reference.generalized_mechanical_work_j, 9.0, 1.0e-12);
    }

    #[test]
    fn accumulated_impulse_never_exceeds_effort_times_dt() {
        let mut budget = MotorStepBudget::new(2.0, 0.1).expect("valid budget");
        let mut velocity = 0.0;
        for _ in 0..8 {
            let result = budget
                .apply_velocity_target(10.0, velocity, 0.5)
                .expect("valid iteration");
            velocity = result.relative_velocity_after;
        }
        let receipt = budget.finish();
        close(receipt.maximum_impulse, 0.2, 1.0e-12);
        close(receipt.total_impulse, 0.2, 1.0e-12);
        assert!(receipt.total_impulse.abs() <= receipt.maximum_impulse + 1.0e-12);
    }

    #[test]
    fn passive_disturbance_reuses_only_the_remaining_step_budget() {
        let mut budget = MotorStepBudget::new(5.0, 0.1).expect("valid budget");
        let first = budget
            .apply_velocity_target(0.4, 0.0, 1.0)
            .expect("first motor iteration");
        close(first.delta_impulse, 0.4, 1.0e-12);

        // A passive constraint then changes relative velocity by -0.2 m/s.
        let disturbed_velocity = first.relative_velocity_after - 0.2;
        let second = budget
            .apply_velocity_target(1.0, disturbed_velocity, 1.0)
            .expect("second motor iteration");

        close(second.delta_impulse, 0.1, 1.0e-12);
        close(second.accumulated_impulse, 0.5, 1.0e-12);
        let receipt = budget.finish();
        close(receipt.total_impulse, receipt.maximum_impulse, 1.0e-12);
    }

    #[test]
    fn static_endpoint_contributes_zero_inverse_mass_not_half_impulse() {
        let inverse_mass = effective_inverse_mass(0.0, 0.25).expect("fixed plus dynamic");
        close(inverse_mass, 0.25, 1.0e-12);

        let mut budget = MotorStepBudget::new(100.0, 0.1).expect("valid budget");
        let result = budget
            .apply_velocity_target(2.0, 0.0, inverse_mass)
            .expect("valid iteration");
        close(result.delta_impulse, 8.0, 1.0e-12);
        close(result.relative_velocity_after, 2.0, 1.0e-12);
    }

    #[test]
    fn positive_midpoint_work_equals_exact_kinetic_energy_change() {
        let inverse_mass = 0.2;
        let velocity_before = 1.0;
        let target_velocity = 3.0;
        let mut budget = MotorStepBudget::new(100.0, 0.1).expect("valid budget");
        let iteration = budget
            .apply_velocity_target(target_velocity, velocity_before, inverse_mass)
            .expect("valid iteration");

        let effective_mass = 1.0 / inverse_mass;
        let delta_kinetic_energy = 0.5 * effective_mass
            * (iteration.relative_velocity_after.powi(2) - velocity_before.powi(2));
        close(iteration.generalized_mechanical_work_j, 20.0, 1.0e-12);
        close(iteration.generalized_mechanical_work_j, delta_kinetic_energy, 1.0e-12);
    }

    #[test]
    fn negative_braking_work_matches_kinetic_energy_loss() {
        let inverse_mass = 0.5;
        let velocity_before = 3.0;
        let target_velocity = 1.0;
        let mut budget = MotorStepBudget::new(100.0, 0.1).expect("valid budget");
        let iteration = budget
            .apply_velocity_target(target_velocity, velocity_before, inverse_mass)
            .expect("valid iteration");

        let effective_mass = 1.0 / inverse_mass;
        let delta_kinetic_energy = 0.5 * effective_mass
            * (iteration.relative_velocity_after.powi(2) - velocity_before.powi(2));
        close(iteration.generalized_mechanical_work_j, -8.0, 1.0e-12);
        close(iteration.generalized_mechanical_work_j, delta_kinetic_energy, 1.0e-12);
    }

    #[test]
    fn finish_emits_one_aggregate_receipt_after_multiple_iterations() {
        let mut budget = MotorStepBudget::new(10.0, 0.1).expect("valid budget");
        let _ = budget.apply_velocity_target(0.2, 0.0, 1.0).expect("iteration");
        let _ = budget.apply_velocity_target(0.5, 0.2, 1.0).expect("iteration");
        let receipt = budget.finish();
        assert_eq!(receipt.solver_iterations, 2);
        close(receipt.total_impulse, 0.5, 1.0e-12);
        assert_eq!(receipt.nonzero_impulse_iterations, 2);
    }

    #[test]
    fn rejects_invalid_step_and_inverse_mass_inputs() {
        assert_eq!(
            MotorStepBudget::new(1.0, 0.0),
            Err(MotorStepError::NonPositiveTimeStep)
        );
        assert_eq!(
            MotorStepBudget::new(-1.0, 0.1),
            Err(MotorStepError::NegativeMaximumEffort)
        );
        assert_eq!(
            MotorStepBudget::new(f64::INFINITY, 0.1),
            Err(MotorStepError::NonFiniteInput)
        );
        assert_eq!(
            effective_inverse_mass(0.0, 0.0),
            Err(MotorStepError::InvalidInverseMass)
        );
        assert_eq!(
            effective_inverse_mass(-1.0, 1.0),
            Err(MotorStepError::InvalidInverseMass)
        );
        let mut budget = MotorStepBudget::new(1.0, 0.1).expect("budget");
        assert_eq!(
            budget.apply_velocity_target(1.0, 0.0, 0.0),
            Err(MotorStepError::InvalidInverseMass)
        );
        assert_eq!(
            budget.apply_velocity_target(f64::NAN, 0.0, 1.0),
            Err(MotorStepError::NonFiniteInput)
        );
    }
}
