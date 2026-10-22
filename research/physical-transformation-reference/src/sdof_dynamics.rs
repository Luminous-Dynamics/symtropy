// SPDX-License-Identifier: AGPL-3.0-or-later
//! Single-degree-of-freedom free-vibration reference.
//!
//! Analytical underdamped response is evaluated independently at every time
//! sample. Numerical integration uses Newmark average acceleration (beta=1/4,
//! gamma=1/2) and reports state error, phase error and damping-work balance.

use crate::Check;

const BETA: f64 = 0.25;
const GAMMA: f64 = 0.5;
const MAX_STEPS: usize = 1_000_000;

/// Parameters and initial state of a linear SDOF oscillator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SdofSystem {
    pub mass_kg: f64,
    pub stiffness_n_per_m: f64,
    pub damping_n_s_per_m: f64,
    pub initial_displacement_m: f64,
    pub initial_velocity_m_per_s: f64,
}

/// One numerical state and its independent analytical reference.
#[derive(Clone, Debug, PartialEq)]
pub struct SdofSample {
    pub time_s: f64,
    pub displacement_m: f64,
    pub velocity_m_per_s: f64,
    pub acceleration_m_per_s2: f64,
    pub mechanical_energy_j: f64,
    pub analytical_displacement_m: f64,
    pub analytical_velocity_m_per_s: f64,
    pub normalized_displacement_error: f64,
    pub normalized_velocity_error: f64,
    pub phase_error_radians: f64,
    pub relative_energy_balance_residual: f64,
}

/// Full transient and error summary from a Newmark free-vibration run.
#[derive(Clone, Debug, PartialEq)]
pub struct SdofSimulation {
    pub system: SdofSystem,
    pub time_step_s: f64,
    pub step_count: usize,
    pub natural_frequency_hz: f64,
    pub damping_ratio: f64,
    pub damped_natural_frequency_hz: f64,
    pub samples: Vec<SdofSample>,
    pub maximum_normalized_displacement_error: f64,
    pub maximum_normalized_velocity_error: f64,
    pub maximum_phase_error_radians: f64,
    pub maximum_relative_energy_balance_residual: f64,
}

/// Frozen request for the SDOF benchmark fixture.
pub const fn canonical_request() -> &'static str {
    concat!(
        "schema=sdof-free-vibration-request-v0.1",
        "|benchmark=PHYS-SDOF-FREE-VIBRATION-001",
        "|mass_kg=2|stiffness_n_per_m=800",
        "|damping_ratio=0,0.05",
        "|initial_displacement_m=0.1|initial_velocity_m_per_s=0",
        "|time_step_s=0.01|duration_s=10",
        "|integrator=newmark-average-acceleration|beta=0.25|gamma=0.5",
        "|checks=analytical_state,phase_error,energy_balance,step_refinement",
        "|claim_ceiling=synthetic-SDOF-verification-not-BE23-BE24-reproduction"
    )
}

/// Exact displacement, velocity and acceleration for unforced underdamped
/// motion. Critical and overdamped cases are intentionally outside this model.
pub fn analytical_free_response(system: SdofSystem, time_s: f64) -> Check<(f64, f64, f64)> {
    let (omega_n, zeta, omega_d) = validate_system(system)?;
    if !time_s.is_finite() || time_s < 0.0 {
        return Err("time_s must be finite and non-negative".to_owned());
    }
    let x0 = system.initial_displacement_m;
    let v0 = system.initial_velocity_m_per_s;
    let envelope = (-zeta * omega_n * time_s).exp();
    let cosine = (omega_d * time_s).cos();
    let sine = (omega_d * time_s).sin();
    let b = (v0 + zeta * omega_n * x0) / omega_d;
    let oscillation = x0 * cosine + b * sine;
    let displacement = envelope * oscillation;
    let velocity = envelope
        * (-zeta * omega_n * oscillation
            + omega_d * (-x0 * sine + b * cosine));
    let acceleration =
        -(system.damping_n_s_per_m * velocity + system.stiffness_n_per_m * displacement)
            / system.mass_kg;
    if [displacement, velocity, acceleration].iter().all(|value| value.is_finite()) {
        Ok((displacement, velocity, acceleration))
    } else {
        Err("analytical free response produced a non-finite state".to_owned())
    }
}

/// Integrate free vibration with constant-average-acceleration Newmark-beta.
/// Supports only finite underdamped systems with positive mass and stiffness.
pub fn simulate_free_vibration(
    system: SdofSystem,
    time_step_s: f64,
    step_count: usize,
) -> Check<SdofSimulation> {
    let (omega_n, zeta, omega_d) = validate_system(system)?;
    if !time_step_s.is_finite() || time_step_s <= 0.0 {
        return Err("time_step_s must be finite and positive".to_owned());
    }
    if step_count == 0 || step_count > MAX_STEPS {
        return Err(format!("step_count must be between 1 and {MAX_STEPS}"));
    }
    if !(time_step_s * step_count as f64).is_finite() {
        return Err("simulation duration is non-finite".to_owned());
    }

    let m = system.mass_kg;
    let c = system.damping_n_s_per_m;
    let k = system.stiffness_n_per_m;
    let dt = time_step_s;
    let a0 = 1.0 / (BETA * dt * dt);
    let a1 = GAMMA / (BETA * dt);
    let a2 = 1.0 / (BETA * dt);
    let a3 = 1.0 / (2.0 * BETA) - 1.0;
    let a4 = GAMMA / BETA - 1.0;
    let a5 = dt * (GAMMA / (2.0 * BETA) - 1.0);
    let effective_stiffness = k + a0 * m + a1 * c;
    if !effective_stiffness.is_finite() || effective_stiffness <= 0.0 {
        return Err("effective Newmark stiffness must be finite and positive".to_owned());
    }

    let amplitude_scale =
        (system.initial_displacement_m.abs() + system.initial_velocity_m_per_s.abs() / omega_n)
            .max(f64::MIN_POSITIVE);
    let initial_energy = mechanical_energy(system, system.initial_displacement_m, system.initial_velocity_m_per_s);
    let energy_scale = initial_energy.max(f64::MIN_POSITIVE);
    let mut x = system.initial_displacement_m;
    let mut v = system.initial_velocity_m_per_s;
    let mut a = -(c * v + k * x) / m;
    let mut cumulative_damping_work = 0.0_f64;
    let mut samples = Vec::with_capacity(step_count + 1);
    samples.push(build_sample(system, 0.0, x, v, a, amplitude_scale, 0.0)?);

    let mut max_x_error = 0.0_f64;
    let mut max_v_error = 0.0_f64;
    let mut max_phase_error = 0.0_f64;
    let mut max_energy_balance = 0.0_f64;

    for step in 1..=step_count {
        // No external force: effective load comes entirely from prior state.
        let p_eff = m * (a0 * x + a2 * v + a3 * a) + c * (a1 * x + a4 * v + a5 * a);
        let x_next = p_eff / effective_stiffness;
        let a_next = a0 * (x_next - x) - a2 * v - a3 * a;
        let v_next = v + dt * ((1.0 - GAMMA) * a + GAMMA * a_next);
        let time_s = dt * step as f64;
        if ![x_next, v_next, a_next, time_s].iter().all(|value| value.is_finite()) {
            return Err(format!("Newmark integration produced a non-finite state at step {step}"));
        }

        cumulative_damping_work += 0.5 * c * (v * v + v_next * v_next) * dt;
        let energy = mechanical_energy(system, x_next, v_next);
        let balance = (energy - initial_energy + cumulative_damping_work).abs() / energy_scale;
        let mut sample = build_sample(
            system,
            time_s,
            x_next,
            v_next,
            a_next,
            amplitude_scale,
            (energy - initial_energy).abs() / energy_scale,
        )?;
        sample.relative_energy_balance_residual = balance;
        max_x_error = max_x_error.max(sample.normalized_displacement_error);
        max_v_error = max_v_error.max(sample.normalized_velocity_error);
        max_phase_error = max_phase_error.max(sample.phase_error_radians);
        max_energy_balance = max_energy_balance.max(balance);
        samples.push(sample);
        x = x_next;
        v = v_next;
        a = a_next;
    }

    Ok(SdofSimulation {
        system,
        time_step_s,
        step_count,
        natural_frequency_hz: omega_n / (2.0 * std::f64::consts::PI),
        damping_ratio: zeta,
        damped_natural_frequency_hz: omega_d / (2.0 * std::f64::consts::PI),
        samples,
        maximum_normalized_displacement_error: max_x_error,
        maximum_normalized_velocity_error: max_v_error,
        maximum_phase_error_radians: max_phase_error,
        maximum_relative_energy_balance_residual: max_energy_balance,
    })
}

fn build_sample(
    system: SdofSystem,
    time_s: f64,
    displacement: f64,
    velocity: f64,
    acceleration: f64,
    amplitude_scale: f64,
    relative_energy_balance_residual: f64,
) -> Check<SdofSample> {
    let (exact_x, exact_v, _) = analytical_free_response(system, time_s)?;
    let (omega_n, zeta, omega_d) = validate_system(system)?;
    let angle_n = phase_angle(displacement, velocity, zeta, omega_n, omega_d);
    let angle_e = phase_angle(exact_x, exact_v, zeta, omega_n, omega_d);
    let delta = angle_n - angle_e;
    let phase_error = delta.sin().atan2(delta.cos()).abs();
    Ok(SdofSample {
        time_s,
        displacement_m: displacement,
        velocity_m_per_s: velocity,
        acceleration_m_per_s2: acceleration,
        mechanical_energy_j: mechanical_energy(system, displacement, velocity),
        analytical_displacement_m: exact_x,
        analytical_velocity_m_per_s: exact_v,
        normalized_displacement_error: (displacement - exact_x).abs() / amplitude_scale,
        normalized_velocity_error: (velocity - exact_v).abs() / (omega_n * amplitude_scale),
        phase_error_radians: phase_error,
        relative_energy_balance_residual,
    })
}

fn validate_system(system: SdofSystem) -> Check<(f64, f64, f64)> {
    for (name, value) in [
        ("mass_kg", system.mass_kg),
        ("stiffness_n_per_m", system.stiffness_n_per_m),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("{name} must be finite and positive"));
        }
    }
    if !system.damping_n_s_per_m.is_finite() || system.damping_n_s_per_m < 0.0 {
        return Err("damping_n_s_per_m must be finite and non-negative".to_owned());
    }
    if !system.initial_displacement_m.is_finite() || !system.initial_velocity_m_per_s.is_finite() {
        return Err("initial displacement and velocity must be finite".to_owned());
    }
    let omega_n = (system.stiffness_n_per_m / system.mass_kg).sqrt();
    let zeta = system.damping_n_s_per_m
        / (2.0 * (system.stiffness_n_per_m * system.mass_kg).sqrt());
    if !omega_n.is_finite() || omega_n <= 0.0 || !zeta.is_finite() || zeta >= 1.0 {
        return Err("system must have finite natural frequency and damping ratio below one".to_owned());
    }
    let omega_d = omega_n * (1.0 - zeta * zeta).sqrt();
    if !omega_d.is_finite() || omega_d <= 0.0 {
        return Err("damped natural frequency must be finite and positive".to_owned());
    }
    Ok((omega_n, zeta, omega_d))
}

fn mechanical_energy(system: SdofSystem, displacement: f64, velocity: f64) -> f64 {
    0.5 * system.mass_kg * velocity * velocity
        + 0.5 * system.stiffness_n_per_m * displacement * displacement
}

fn phase_angle(x: f64, v: f64, zeta: f64, omega_n: f64, omega_d: f64) -> f64 {
    // The common exponential envelope cancels in atan2; v + zeta*omega_n*x
    // yields the rotating component of the underdamped state.
    let transformed_v = (v + zeta * omega_n * x) / omega_d;
    (-transformed_v).atan2(x)
}

#[cfg(test)]
mod tests {
    use super::{SdofSystem, analytical_free_response, canonical_request, simulate_free_vibration};

    fn undamped() -> SdofSystem {
        SdofSystem {
            mass_kg: 2.0,
            stiffness_n_per_m: 800.0,
            damping_n_s_per_m: 0.0,
            initial_displacement_m: 0.1,
            initial_velocity_m_per_s: 0.0,
        }
    }

    fn underdamped() -> SdofSystem {
        let mut system = undamped();
        system.damping_n_s_per_m =
            2.0 * 0.05 * (system.mass_kg * system.stiffness_n_per_m).sqrt();
        system
    }

    #[test]
    fn canonical_request_is_frozen() {
        assert_eq!(
            canonical_request(),
            include_str!("../fixtures/sdof_free_vibration_request_v0_1.txt")
                .trim_end_matches('\\n')
        );
    }

    #[test]
    fn undamped_analytical_solution_matches_cosine_known_answer() {
        let system = undamped();
        let omega = (system.stiffness_n_per_m / system.mass_kg).sqrt();
        for time_s in [0.0, 0.1, 0.25, 1.0, 2.75] {
            let (x, v, a) = analytical_free_response(system, time_s).expect("analytical response");
            assert!((x - system.initial_displacement_m * (omega * time_s).cos()).abs() < 1.0e-13);
            assert!((v + system.initial_displacement_m * omega * (omega * time_s).sin()).abs() < 1.0e-12);
            assert!((a + omega * omega * x).abs() < 1.0e-11);
        }
    }

    #[test]
    fn average_acceleration_preserves_undamped_energy() {
        let result = simulate_free_vibration(undamped(), 0.01, 1000).expect("10-second run");
        assert_eq!(result.samples.len(), 1001);
        assert_eq!(result.damping_ratio, 0.0);
        assert!(result.maximum_relative_energy_balance_residual < 1.0e-10);
        assert!(result.maximum_normalized_displacement_error < 1.0e-3);
        assert!(result.maximum_phase_error_radians < 2.0e-3);
    }

    #[test]
    fn underdamped_state_phase_and_energy_balance_converge_when_halving_step() {
        let coarse = simulate_free_vibration(underdamped(), 0.02, 500).expect("coarse run");
        let fine = simulate_free_vibration(underdamped(), 0.01, 1000).expect("fine run");
        assert!(fine.maximum_normalized_displacement_error < coarse.maximum_normalized_displacement_error / 2.5);
        assert!(fine.maximum_normalized_velocity_error < coarse.maximum_normalized_velocity_error / 2.5);
        assert!(fine.maximum_phase_error_radians < coarse.maximum_phase_error_radians / 2.5);
        assert!(fine.maximum_relative_energy_balance_residual < coarse.maximum_relative_energy_balance_residual);
        assert!(fine.maximum_normalized_displacement_error < 2.0e-3);
        assert!(fine.maximum_phase_error_radians < 2.0e-3);
    }

    #[test]
    fn underdamped_energy_decays_and_damping_ratio_is_correct() {
        let result = simulate_free_vibration(underdamped(), 0.01, 1000).expect("damped run");
        assert!((result.damping_ratio - 0.05).abs() < 1.0e-14);
        assert!(result.samples.last().unwrap().mechanical_energy_j
            < result.samples[0].mechanical_energy_j * 0.2);
        assert!(result.maximum_relative_energy_balance_residual < 1.0e-3);
    }

    #[test]
    fn natural_frequency_obeys_stiffness_and_mass_scaling() {
        let base = simulate_free_vibration(undamped(), 0.01, 10).expect("base run");
        let mut stiff = undamped();
        stiff.stiffness_n_per_m *= 4.0;
        let mut heavy = undamped();
        heavy.mass_kg *= 4.0;
        let stiff_run = simulate_free_vibration(stiff, 0.01, 10).expect("stiff run");
        let heavy_run = simulate_free_vibration(heavy, 0.01, 10).expect("heavy run");
        assert!(((stiff_run.natural_frequency_hz / base.natural_frequency_hz) - 2.0).abs() < 1.0e-14);
        assert!(((heavy_run.natural_frequency_hz / base.natural_frequency_hz) - 0.5).abs() < 1.0e-14);
    }

    #[test]
    fn critical_damping_and_invalid_inputs_fail_closed() {
        let mut system = underdamped();
        system.damping_n_s_per_m =
            2.0 * (system.mass_kg * system.stiffness_n_per_m).sqrt();
        assert!(analytical_free_response(system, 0.1).is_err());
        let mut invalid = undamped();
        invalid.mass_kg = f64::NAN;
        assert!(simulate_free_vibration(invalid, 0.01, 10).is_err());
        assert!(simulate_free_vibration(undamped(), 0.0, 10).is_err());
        assert!(simulate_free_vibration(undamped(), 0.01, 0).is_err());
    }
}
