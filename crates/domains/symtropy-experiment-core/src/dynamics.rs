// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Small reference dynamics models with explicit domains and analytical comparisons.
//!
//! This module is intentionally not a general ODE solver. The first model is a one-dimensional,
//! undamped, unforced harmonic oscillator, for which the exact solution is known. It is useful as
//! an integrator regression benchmark, not as evidence that a real apparatus behaves ideally.

use serde::{Deserialize, Serialize};

pub const MAX_OSCILLATOR_STEPS: usize = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OscillatorIntegrator {
    ExplicitEuler,
    SymplecticEuler,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarmonicOscillatorSpec {
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
        let time_s = step as f64 * spec.time_step_s;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(integrator: OscillatorIntegrator, time_step_s: f64) -> HarmonicOscillatorSpec {
        HarmonicOscillatorSpec {
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
