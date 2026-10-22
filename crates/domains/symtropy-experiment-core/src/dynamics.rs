// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Small reference dynamics models with explicit domains and analytical comparisons.
//!
//! This module is intentionally not a general ODE solver. The first model is a one-dimensional,
//! undamped, unforced harmonic oscillator, for which the exact solution is known. It is useful as
//! an integrator regression benchmark, not as evidence that a real apparatus behaves ideally.

use serde::{Deserialize, Serialize};

pub const OSCILLATOR_SCHEMA_VERSION: &str = "0.1";
pub const MAX_OSCILLATOR_STEPS: usize = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OscillatorIntegrator {
    ExplicitEuler,
    SymplecticEuler,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarmonicOscillatorSpec {
    pub schema_version: String,
    pub id: String,
    pub title: String,
    pub mass_kg: f64,
    pub stiffness_n_per_m: f64,
    pub initial_position_m: f64,
    pub initial_velocity_mps: f64,
    pub duration_s: f64,
    pub time_step_s: f64,
    pub integrator: OscillatorIntegrator,
    pub assumptions: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OscillatorSample {
    pub time_s: f64,
    pub position_m: f64,
    pub velocity_mps: f64,
    pub acceleration_mps2: f64,
    pub energy_j: f64,
    pub analytic_position_m: f64,
    pub analytic_velocity_mps: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConvergencePoint {
    pub time_step_s: f64,
    pub step_count: usize,
    pub max_position_error_m: f64,
    pub max_velocity_error_mps: f64,
    pub max_relative_energy_deviation: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConvergenceOrderEstimate {
    pub coarse_time_step_s: f64,
    pub fine_time_step_s: f64,
    pub coarse_position_error_m: f64,
    pub fine_position_error_m: f64,
    /// Observed order from this pair of runs; negative values indicate worsening error.
    pub observed_order: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OscillatorConvergenceReport {
    pub experiment_id: String,
    pub integrator: OscillatorIntegrator,
    /// Ordered from coarsest to finest time step.
    pub points: Vec<ConvergencePoint>,
    pub observed_orders: Vec<ConvergenceOrderEstimate>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarmonicOscillatorResult {
    pub experiment_id: String,
    pub integrator: OscillatorIntegrator,
    pub angular_frequency_rad_per_s: f64,
    pub step_count: usize,
    pub initial_energy_j: f64,
    pub max_relative_energy_deviation: f64,
    pub max_position_error_m: f64,
    pub max_velocity_error_mps: f64,
    pub samples: Vec<OscillatorSample>,
    pub assumptions: Vec<String>,
    pub limitations: Vec<String>,
}

fn finite_positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

impl HarmonicOscillatorSpec {
    pub fn validate(&self) -> Result<usize, String> {
        if self.schema_version != OSCILLATOR_SCHEMA_VERSION {
            return Err(format!(
                "schema_version '{}' is unsupported; expected '{}'",
                self.schema_version, OSCILLATOR_SCHEMA_VERSION
            ));
        }
        if self.id.trim().is_empty() || self.title.trim().is_empty() {
            return Err("experiment id and title must be non-empty".to_owned());
        }
        if !finite_positive(self.mass_kg) {
            return Err("mass_kg must be finite and greater than zero".to_owned());
        }
        if !finite_positive(self.stiffness_n_per_m) {
            return Err("stiffness_n_per_m must be finite and greater than zero".to_owned());
        }
        if !self.initial_position_m.is_finite() || !self.initial_velocity_mps.is_finite() {
            return Err("initial position and velocity must be finite".to_owned());
        }
        if !finite_positive(self.duration_s) || !finite_positive(self.time_step_s) {
            return Err("duration_s and time_step_s must be finite and greater than zero".to_owned());
        }
        if self.assumptions.is_empty() || self.assumptions.iter().any(|item| item.trim().is_empty()) {
            return Err("explicit non-empty model assumptions are required".to_owned());
        }
        if self.limitations.is_empty() || self.limitations.iter().any(|item| item.trim().is_empty()) {
            return Err("explicit non-empty model limitations are required".to_owned());
        }

        let ratio = self.duration_s / self.time_step_s;
        if !ratio.is_finite() || ratio < 1.0 || ratio > MAX_OSCILLATOR_STEPS as f64 {
            return Err(format!(
                "duration/time_step must be between 1 and {MAX_OSCILLATOR_STEPS} steps"
            ));
        }
        let nearest = ratio.round();
        if (ratio - nearest).abs() > 1e-10 * ratio.max(1.0) {
            return Err("duration_s must be an integer multiple of time_step_s within tolerance".to_owned());
        }

        let omega = (self.stiffness_n_per_m / self.mass_kg).sqrt();
        if !omega.is_finite() || omega <= 0.0 {
            return Err("natural angular frequency must be finite and positive".to_owned());
        }
        if self.integrator == OscillatorIntegrator::SymplecticEuler
            && self.time_step_s * omega >= 2.0
        {
            return Err("symplectic Euler requires time_step_s * angular_frequency < 2 for this oscillator".to_owned());
        }

        Ok(nearest as usize)
    }
}

/// Simulate an ideal harmonic oscillator and compare every sample with its analytical solution.
///
/// The equation is x'' + (k/m)x = 0, with angular frequency sqrt(k/m). The reported errors are
/// numerical discrepancies from this exact ideal model, not real-world measurement errors.
pub fn simulate_harmonic_oscillator(
    spec: &HarmonicOscillatorSpec,
) -> Result<HarmonicOscillatorResult, String> {
    let step_count = spec.validate()?;
    let omega = (spec.stiffness_n_per_m / spec.mass_kg).sqrt();
    let acceleration = |position_m: f64| -(spec.stiffness_n_per_m / spec.mass_kg) * position_m;
    let energy = |position_m: f64, velocity_mps: f64| {
        0.5 * spec.mass_kg * velocity_mps * velocity_mps
            + 0.5 * spec.stiffness_n_per_m * position_m * position_m
    };
    let analytic = |time_s: f64| {
        let (sin, cos) = (omega * time_s).sin_cos();
        let position = spec.initial_position_m * cos
            + spec.initial_velocity_mps / omega * sin;
        let velocity = -spec.initial_position_m * omega * sin
            + spec.initial_velocity_mps * cos;
        (position, velocity)
    };

    let initial_energy_j = energy(spec.initial_position_m, spec.initial_velocity_mps);
    if !initial_energy_j.is_finite() {
        return Err("initial oscillator energy is not finite".to_owned());
    }
    let mut position_m = spec.initial_position_m;
    let mut velocity_mps = spec.initial_velocity_mps;
    let mut samples = Vec::with_capacity(step_count + 1);
    let mut max_position_error_m: f64 = 0.0;
    let mut max_velocity_error_mps: f64 = 0.0;
    let mut max_relative_energy_deviation: f64 = 0.0;

    for step in 0..=step_count {
        let time_s = if step == step_count { spec.duration_s } else { step as f64 * spec.time_step_s };
        let (analytic_position_m, analytic_velocity_mps) = analytic(time_s);
        let acceleration_mps2 = acceleration(position_m);
        let energy_j = energy(position_m, velocity_mps);
        for value in [
            time_s,
            position_m,
            velocity_mps,
            acceleration_mps2,
            energy_j,
            analytic_position_m,
            analytic_velocity_mps,
        ] {
            if !value.is_finite() {
                return Err(format!("non-finite oscillator result at step {step}"));
            }
        }
        max_position_error_m = max_position_error_m.max((position_m - analytic_position_m).abs());
        max_velocity_error_mps =
            max_velocity_error_mps.max((velocity_mps - analytic_velocity_mps).abs());
        if initial_energy_j > 0.0 {
            max_relative_energy_deviation = max_relative_energy_deviation
                .max((energy_j - initial_energy_j).abs() / initial_energy_j);
        } else if energy_j != 0.0 {
            return Err("zero-energy initial condition produced nonzero energy".to_owned());
        }

        samples.push(OscillatorSample {
            time_s,
            position_m,
            velocity_mps,
            acceleration_mps2,
            energy_j,
            analytic_position_m,
            analytic_velocity_mps,
        });

        if step == step_count {
            break;
        }

        let old_position_m = position_m;
        let old_velocity_mps = velocity_mps;
        let acceleration_now = acceleration(old_position_m);
        match spec.integrator {
            OscillatorIntegrator::ExplicitEuler => {
                position_m = old_position_m + spec.time_step_s * old_velocity_mps;
                velocity_mps = old_velocity_mps + spec.time_step_s * acceleration_now;
            }
            OscillatorIntegrator::SymplecticEuler => {
                velocity_mps = old_velocity_mps + spec.time_step_s * acceleration_now;
                position_m = old_position_m + spec.time_step_s * velocity_mps;
            }
        }
    }

    Ok(HarmonicOscillatorResult {
        experiment_id: spec.id.clone(),
        integrator: spec.integrator,
        angular_frequency_rad_per_s: omega,
        step_count,
        initial_energy_j,
        max_relative_energy_deviation,
        max_position_error_m,
        max_velocity_error_mps,
        samples,
        assumptions: spec.assumptions.clone(),
        limitations: spec.limitations.clone(),
    })
}

/// Compare timestep-refinement runs against the oscillator's closed-form solution.
///
/// Timestep sizes must be ordered coarse-to-fine and must each divide the duration according
/// to the spec's validation rule. The function reports observed pairwise orders rather than
/// asserting a particular order; a failed/concerning order remains visible in the report.
pub fn analyze_harmonic_oscillator_convergence(
    base: &HarmonicOscillatorSpec,
    time_steps_s: &[f64],
) -> Result<OscillatorConvergenceReport, String> {
    if time_steps_s.len() < 3 {
        return Err("convergence analysis requires at least three time-step sizes".to_owned());
    }
    for pair in time_steps_s.windows(2) {
        if !pair[0].is_finite() || !pair[1].is_finite() || pair[0] <= pair[1] || pair[1] <= 0.0 {
            return Err("time-step sizes must be finite, positive, distinct, and ordered coarse-to-fine".to_owned());
        }
    }

    let mut points = Vec::with_capacity(time_steps_s.len());
    for time_step_s in time_steps_s {
        let mut spec = base.clone();
        spec.time_step_s = *time_step_s;
        let result = simulate_harmonic_oscillator(&spec)?;
        points.push(ConvergencePoint {
            time_step_s: *time_step_s,
            step_count: result.step_count,
            max_position_error_m: result.max_position_error_m,
            max_velocity_error_mps: result.max_velocity_error_mps,
            max_relative_energy_deviation: result.max_relative_energy_deviation,
        });
    }

    let mut observed_orders = Vec::with_capacity(points.len() - 1);
    for pair in points.windows(2) {
        let coarse = &pair[0];
        let fine = &pair[1];
        if coarse.max_position_error_m <= 0.0 || fine.max_position_error_m <= 0.0 {
            return Err("observed order is undefined when position error is zero".to_owned());
        }
        let numerator = (coarse.max_position_error_m / fine.max_position_error_m).ln();
        let denominator = (coarse.time_step_s / fine.time_step_s).ln();
        let observed_order = numerator / denominator;
        if !observed_order.is_finite() {
            return Err("observed convergence order is not finite".to_owned());
        }
        observed_orders.push(ConvergenceOrderEstimate {
            coarse_time_step_s: coarse.time_step_s,
            fine_time_step_s: fine.time_step_s,
            coarse_position_error_m: coarse.max_position_error_m,
            fine_position_error_m: fine.max_position_error_m,
            observed_order,
        });
    }

    Ok(OscillatorConvergenceReport {
        experiment_id: base.id.clone(),
        integrator: base.integrator,
        points,
        observed_orders,
        limitations: vec![
            "Observed order is estimated from maximum position error against the exact solution of the ideal oscillator.".to_owned(),
            "This finite-horizon result applies to the supplied model, initial state, and time-step sequence; it is not a universal solver-order claim.".to_owned(),
            "No real measurements, damping, external forcing, model-form uncertainty, or instrument error are represented.".to_owned(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(integrator: OscillatorIntegrator, time_step_s: f64) -> HarmonicOscillatorSpec {
        HarmonicOscillatorSpec {
            schema_version: OSCILLATOR_SCHEMA_VERSION.to_owned(),
            id: "synthetic:harmonic-oscillator".to_owned(),
            title: "Synthetic undamped harmonic oscillator".to_owned(),
            mass_kg: 1.0,
            stiffness_n_per_m: 4.0,
            initial_position_m: 1.0,
            initial_velocity_mps: 0.0,
            duration_s: 10.0,
            time_step_s,
            integrator,
            assumptions: vec!["linear restoring force F=-k*x".to_owned()],
            limitations: vec![
                "No damping, forcing, nonlinear stiffness, or measurement noise".to_owned(),
                "Synthetic reference case, not a physical laboratory result".to_owned(),
            ],
        }
    }

    #[test]
    fn oscillator_starts_at_declared_state_and_has_known_frequency() {
        let result = simulate_harmonic_oscillator(&spec(OscillatorIntegrator::SymplecticEuler, 0.01))
            .unwrap();
        assert_eq!(result.samples.len(), 1001);
        assert_eq!(result.samples[0].position_m, 1.0);
        assert_eq!(result.samples[0].velocity_mps, 0.0);
        assert!((result.angular_frequency_rad_per_s - 2.0).abs() < 1e-12);
        assert_eq!(result.samples.last().unwrap().time_s, 10.0);
    }

    #[test]
    fn symplectic_euler_has_bounded_energy_error_on_reference_oscillator() {
        let result = simulate_harmonic_oscillator(&spec(OscillatorIntegrator::SymplecticEuler, 0.01))
            .unwrap();
        assert!(result.max_relative_energy_deviation < 0.02);
        assert!(result.max_position_error_m.is_finite());
    }

    #[test]
    fn explicit_euler_energy_growth_is_detected_on_reference_oscillator() {
        let result = simulate_harmonic_oscillator(&spec(OscillatorIntegrator::ExplicitEuler, 0.01))
            .unwrap();
        assert!(result.samples.last().unwrap().energy_j > result.initial_energy_j * 1.2);
    }

    #[test]
    fn halving_time_step_reduces_oscillator_position_error() {
        let coarse = simulate_harmonic_oscillator(&spec(OscillatorIntegrator::SymplecticEuler, 0.02))
            .unwrap();
        let fine = simulate_harmonic_oscillator(&spec(OscillatorIntegrator::SymplecticEuler, 0.01))
            .unwrap();
        assert!(fine.max_position_error_m < coarse.max_position_error_m);
    }

    #[test]
    fn convergence_report_estimates_first_order_for_symplectic_euler() {
        let base = spec(OscillatorIntegrator::SymplecticEuler, 0.01);
        let report = analyze_harmonic_oscillator_convergence(
            &base,
            &[0.04, 0.02, 0.01, 0.005],
        )
        .unwrap();
        assert_eq!(report.points.len(), 4);
        assert_eq!(report.observed_orders.len(), 3);
        assert!(report.points.windows(2).all(|pair| {
            pair[1].max_position_error_m < pair[0].max_position_error_m
        }));
        assert!(report.observed_orders.iter().all(|order| {
            order.observed_order > 0.8 && order.observed_order < 1.3
        }));
    }

    #[test]
    fn convergence_report_rejects_bad_step_sequences_and_too_few_points() {
        let base = spec(OscillatorIntegrator::SymplecticEuler, 0.01);
        assert!(analyze_harmonic_oscillator_convergence(&base, &[0.02, 0.01]).is_err());
        assert!(analyze_harmonic_oscillator_convergence(&base, &[0.01, 0.02, 0.005]).is_err());
    }

    #[test]
    fn unsupported_oscillator_schema_version_is_rejected() {
        let mut spec = spec(OscillatorIntegrator::SymplecticEuler, 0.01);
        spec.schema_version = "99.0".to_owned();
        assert!(spec.validate().is_err());
    }

    #[test]
    fn invalid_step_alignment_and_unstable_symplectic_step_are_rejected() {
        let mut unaligned = spec(OscillatorIntegrator::SymplecticEuler, 0.03);
        unaligned.duration_s = 10.0;
        assert!(unaligned.validate().is_err());

        let unstable = spec(OscillatorIntegrator::SymplecticEuler, 1.0);
        assert!(unstable.validate().is_err());
    }

    #[test]
    fn invalid_mass_and_nonfinite_initial_state_are_rejected() {
        let mut zero_mass = spec(OscillatorIntegrator::SymplecticEuler, 0.01);
        zero_mass.mass_kg = 0.0;
        assert!(zero_mass.validate().is_err());

        let mut infinite_state = spec(OscillatorIntegrator::SymplecticEuler, 0.01);
        infinite_state.initial_position_m = f64::INFINITY;
        assert!(infinite_state.validate().is_err());
    }
}
