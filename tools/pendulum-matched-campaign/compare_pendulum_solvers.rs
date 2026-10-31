// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Matched-budget pendulum solver comparison. This runner reuses the two
// standalone reference implementations in adjacent tool directories.

use std::fmt::Write as _;

#[allow(dead_code)]
#[path = "../pendulum-intrinsic-reference/pendulum_discrete_gradient.rs"]
mod intrinsic;

#[allow(dead_code)]
#[path = "../pendulum-rattle-reference/rattle_cartesian.rs"]
mod rattle;

const MASS: f64 = 1.0;
const LENGTH: f64 = 1.0;
const GRAVITY: f64 = 9.81;
const HORIZON: f64 = 1.0;
const INITIAL_ANGLE: f64 = 1.1;
const INITIAL_ANGULAR_MOMENTUM: f64 = 0.15;
const REFERENCE_STEPS: usize = 80_000;
const TIME_STEPS: [f64; 4] = [0.04, 0.02, 0.01, 0.005];

#[derive(Clone, Copy, Debug)]
struct IntrinsicState {
    theta: f64,
    ell: f64,
}

#[derive(Clone, Copy, Debug)]
struct Metrics {
    method: &'static str,
    dt: f64,
    steps: usize,
    final_state_error: f64,
    final_relative_energy_drift: f64,
    max_relative_energy_drift: f64,
    max_relative_position_constraint: f64,
    max_relative_tangent_constraint: f64,
    mean_nonlinear_iterations: f64,
    max_nonlinear_iterations: usize,
}

fn intrinsic_initial() -> IntrinsicState {
    IntrinsicState {
        theta: INITIAL_ANGLE,
        ell: INITIAL_ANGULAR_MOMENTUM,
    }
}

fn angle_energy(state: IntrinsicState) -> f64 {
    state.ell * state.ell / (2.0 * MASS * LENGTH * LENGTH)
        + MASS * GRAVITY * LENGTH * (1.0 - state.theta.cos())
}

fn to_cartesian(state: IntrinsicState) -> rattle::State {
    let theta = state.theta;
    rattle::State {
        position: [LENGTH * theta.sin(), -LENGTH * theta.cos()],
        momentum: [
            state.ell / LENGTH * theta.cos(),
            state.ell / LENGTH * theta.sin(),
        ],
    }
}

fn cartesian_metrics(state: rattle::State) -> (f64, f64) {
    let q = state.position;
    let p = state.momentum;
    let q_squared = q[0] * q[0] + q[1] * q[1];
    let p_norm = (p[0] * p[0] + p[1] * p[1]).sqrt();
    let q_dot_p = q[0] * p[0] + q[1] * p[1];
    let position = (q_squared - LENGTH * LENGTH).abs()
        / (1.0 + q_squared + LENGTH * LENGTH);
    let tangent =
        q_dot_p.abs() / (LENGTH * p_norm).max(f64::MIN_POSITIVE);
    (position, tangent)
}

fn state_error(actual: rattle::State, reference: rattle::State) -> f64 {
    let dx = actual.position[0] - reference.position[0];
    let dy = actual.position[1] - reference.position[1];
    let dpx = actual.momentum[0] - reference.momentum[0];
    let dpy = actual.momentum[1] - reference.momentum[1];
    (dx * dx + dy * dy + dpx * dpx + dpy * dpy).sqrt()
}

fn rk4_step(state: IntrinsicState, dt: f64) -> IntrinsicState {
    let rhs = |s: IntrinsicState| {
        (
            s.ell / (MASS * LENGTH * LENGTH),
            -MASS * GRAVITY * LENGTH * s.theta.sin(),
        )
    };
    let (a1q, a1p) = rhs(state);
    let (a2q, a2p) = rhs(IntrinsicState {
        theta: state.theta + 0.5 * dt * a1q,
        ell: state.ell + 0.5 * dt * a1p,
    });
    let (a3q, a3p) = rhs(IntrinsicState {
        theta: state.theta + 0.5 * dt * a2q,
        ell: state.ell + 0.5 * dt * a2p,
    });
    let (a4q, a4p) = rhs(IntrinsicState {
        theta: state.theta + dt * a3q,
        ell: state.ell + dt * a3p,
    });
    IntrinsicState {
        theta: state.theta + dt * (a1q + 2.0 * a2q + 2.0 * a3q + a4q) / 6.0,
        ell: state.ell + dt * (a1p + 2.0 * a2p + 2.0 * a3p + a4p) / 6.0,
    }
}

fn rk4_advance(mut state: IntrinsicState, dt: f64, steps: usize) -> IntrinsicState {
    for _ in 0..steps {
        state = rk4_step(state, dt);
    }
    state
}

fn run_intrinsic(dt: f64, steps: usize, reference: rattle::State) -> Metrics {
    let model = intrinsic::IntrinsicPendulum::new(MASS, LENGTH, GRAVITY, 0.0)
        .expect("frozen intrinsic model must be valid");
    let mut state = intrinsic::State {
        angle: INITIAL_ANGLE,
        angular_momentum: INITIAL_ANGULAR_MOMENTUM,
    };
    let energy0 = model.hamiltonian(state).expect("initial energy must be finite");
    let mut max_energy_drift = 0.0_f64;
    let mut max_position = 0.0_f64;
    let mut max_tangent = 0.0_f64;
    let mut total_iterations = 0usize;
    let mut max_iterations = 0usize;

    for _ in 0..steps {
        let receipt = model
            .step(state, 0.0, dt, intrinsic::SolverOptions::default())
            .expect("intrinsic reference step must converge");
        state = receipt.after;
        let energy = receipt.energy_after;
        max_energy_drift =
            max_energy_drift.max((energy - energy0).abs() / energy0.abs().max(f64::MIN_POSITIVE));
        let cart = to_cartesian(IntrinsicState {
            theta: state.angle,
            ell: state.angular_momentum,
        });
        let (position, tangent) = cartesian_metrics(cart);
        max_position = max_position.max(position);
        max_tangent = max_tangent.max(tangent);
        total_iterations += receipt.newton_iterations;
        max_iterations = max_iterations.max(receipt.newton_iterations);
    }

    let final_state = to_cartesian(IntrinsicState {
        theta: state.angle,
        ell: state.angular_momentum,
    });
    let final_energy = model.hamiltonian(state).expect("final energy must be finite");
    Metrics {
        method: "intrinsic_discrete_gradient",
        dt,
        steps,
        final_state_error: state_error(final_state, reference),
        final_relative_energy_drift: (final_energy - energy0).abs()
            / energy0.abs().max(f64::MIN_POSITIVE),
        max_relative_energy_drift: max_energy_drift,
        max_relative_position_constraint: max_position,
        max_relative_tangent_constraint: max_tangent,
        mean_nonlinear_iterations: total_iterations as f64 / steps as f64,
        max_nonlinear_iterations: max_iterations,
    }
}

fn run_rattle(dt: f64, steps: usize, reference: rattle::State) -> Metrics {
    let model = rattle::CartesianPendulum::new(MASS, LENGTH, [0.0, -GRAVITY])
        .expect("frozen RATTLE model must be valid");
    let mut state = to_cartesian(intrinsic_initial());
    let energy0 = model.hamiltonian(state).expect("initial energy must be finite");
    let mut max_energy_drift = 0.0_f64;
    let mut max_position = 0.0_f64;
    let mut max_tangent = 0.0_f64;
    let mut total_iterations = 0usize;
    let mut max_iterations = 0usize;

    for _ in 0..steps {
        let receipt = model.step(state, dt).expect("RATTLE step must converge");
        state = receipt.after;
        let energy = receipt.energy_after;
        max_energy_drift =
            max_energy_drift.max((energy - energy0).abs() / energy0.abs().max(f64::MIN_POSITIVE));
        max_position =
            max_position.max(receipt.relative_position_constraint_residual);
        max_tangent =
            max_tangent.max(receipt.relative_tangent_constraint_residual);
        total_iterations += receipt.position_newton_iterations;
        max_iterations = max_iterations.max(receipt.position_newton_iterations);
    }
    let final_energy = model.hamiltonian(state).expect("final energy must be finite");
    Metrics {
        method: "cartesian_rattle",
        dt,
        steps,
        final_state_error: state_error(state, reference),
        final_relative_energy_drift: (final_energy - energy0).abs()
            / energy0.abs().max(f64::MIN_POSITIVE),
        max_relative_energy_drift: max_energy_drift,
        max_relative_position_constraint: max_position,
        max_relative_tangent_constraint: max_tangent,
        mean_nonlinear_iterations: total_iterations as f64 / steps as f64,
        max_nonlinear_iterations: max_iterations,
    }
}

fn run_symplectic_euler(dt: f64, steps: usize, reference: rattle::State) -> Metrics {
    let mut state = intrinsic_initial();
    let energy0 = angle_energy(state);
    let mut max_energy_drift = 0.0_f64;
    let mut max_position = 0.0_f64;
    let mut max_tangent = 0.0_f64;
    for _ in 0..steps {
        // Kick-drift symplectic Euler in canonical angle/momentum coordinates.
        state.ell -= dt * MASS * GRAVITY * LENGTH * state.theta.sin();
        state.theta += dt * state.ell / (MASS * LENGTH * LENGTH);
        let energy = angle_energy(state);
        max_energy_drift =
            max_energy_drift.max((energy - energy0).abs() / energy0.abs().max(f64::MIN_POSITIVE));
        let (position, tangent) = cartesian_metrics(to_cartesian(state));
        max_position = max_position.max(position);
        max_tangent = max_tangent.max(tangent);
    }
    Metrics {
        method: "symplectic_euler",
        dt,
        steps,
        final_state_error: state_error(to_cartesian(state), reference),
        final_relative_energy_drift: (angle_energy(state) - energy0).abs()
            / energy0.abs().max(f64::MIN_POSITIVE),
        max_relative_energy_drift: max_energy_drift,
        max_relative_position_constraint: max_position,
        max_relative_tangent_constraint: max_tangent,
        mean_nonlinear_iterations: 0.0,
        max_nonlinear_iterations: 0,
    }
}

fn run_rk4(dt: f64, steps: usize, reference: rattle::State) -> Metrics {
    let mut state = intrinsic_initial();
    let energy0 = angle_energy(state);
    let mut max_energy_drift = 0.0_f64;
    let mut max_position = 0.0_f64;
    let mut max_tangent = 0.0_f64;
    for _ in 0..steps {
        state = rk4_step(state, dt);
        let energy = angle_energy(state);
        max_energy_drift =
            max_energy_drift.max((energy - energy0).abs() / energy0.abs().max(f64::MIN_POSITIVE));
        let (position, tangent) = cartesian_metrics(to_cartesian(state));
        max_position = max_position.max(position);
        max_tangent = max_tangent.max(tangent);
    }
    Metrics {
        method: "rk4",
        dt,
        steps,
        final_state_error: state_error(to_cartesian(state), reference),
        final_relative_energy_drift: (angle_energy(state) - energy0).abs()
            / energy0.abs().max(f64::MIN_POSITIVE),
        max_relative_energy_drift: max_energy_drift,
        max_relative_position_constraint: max_position,
        max_relative_tangent_constraint: max_tangent,
        mean_nonlinear_iterations: 0.0,
        max_nonlinear_iterations: 0,
    }
}

fn campaign_csv() -> String {
    let reference_intrinsic = rk4_advance(
        intrinsic_initial(),
        HORIZON / REFERENCE_STEPS as f64,
        REFERENCE_STEPS,
    );
    let reference_cartesian = to_cartesian(reference_intrinsic);
    let mut output = String::new();
    writeln!(
        &mut output,
        "method,dt,steps,final_state_error,final_relative_energy_drift,max_relative_energy_drift,max_relative_position_constraint,max_relative_tangent_constraint,mean_nonlinear_iterations,max_nonlinear_iterations"
    )
    .expect("writing to String cannot fail");

    for dt in TIME_STEPS {
        let steps = (HORIZON / dt).round() as usize;
        for result in [
            run_intrinsic(dt, steps, reference_cartesian),
            run_rattle(dt, steps, reference_cartesian),
            run_symplectic_euler(dt, steps, reference_cartesian),
            run_rk4(dt, steps, reference_cartesian),
        ] {
            writeln!(
                &mut output,
                "{},{:.8},{},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.6},{}",
                result.method,
                result.dt,
                result.steps,
                result.final_state_error,
                result.final_relative_energy_drift,
                result.max_relative_energy_drift,
                result.max_relative_position_constraint,
                result.max_relative_tangent_constraint,
                result.mean_nonlinear_iterations,
                result.max_nonlinear_iterations
            )
            .expect("writing to String cannot fail");
        }
    }
    output
}

fn audit_campaign(csv: &str) -> Result<(), String> {
    const METHODS: [&str; 4] = [
        "intrinsic_discrete_gradient",
        "cartesian_rattle",
        "symplectic_euler",
        "rk4",
    ];
    const HEADER: &str = "method,dt,steps,final_state_error,final_relative_energy_drift,max_relative_energy_drift,max_relative_position_constraint,max_relative_tangent_constraint,mean_nonlinear_iterations,max_nonlinear_iterations";

    let mut lines = csv.lines();
    if lines.next() != Some(HEADER) {
        return Err("unexpected or missing CSV header".to_string());
    }
    let mut seen = [[false; 4]; 4];
    let mut record_count = 0usize;

    for (row_index, line) in lines.enumerate() {
        if line.trim().is_empty() {
            return Err(format!("empty CSV row at line {}", row_index + 2));
        }
        let columns: Vec<&str> = line.split(',').collect();
        if columns.len() != 10 {
            return Err(format!("expected 10 columns at line {}", row_index + 2));
        }
        let method_index = METHODS
            .iter()
            .position(|method| *method == columns[0])
            .ok_or_else(|| format!("unknown method at line {}", row_index + 2))?;
        let dt: f64 = columns[1]
            .parse()
            .map_err(|_| format!("invalid timestep at line {}", row_index + 2))?;
        let dt_index = TIME_STEPS
            .iter()
            .position(|expected| (dt - expected).abs() <= 1.0e-10)
            .ok_or_else(|| format!("unexpected timestep at line {}", row_index + 2))?;
        if seen[method_index][dt_index] {
            return Err(format!("duplicate method/timestep at line {}", row_index + 2));
        }
        let steps: usize = columns[2]
            .parse()
            .map_err(|_| format!("invalid step count at line {}", row_index + 2))?;
        let expected_steps = (HORIZON / dt).round() as usize;
        if steps != expected_steps {
            return Err(format!("step count does not match horizon at line {}", row_index + 2));
        }

        let mut metrics = [0.0_f64; 6];
        for (metric_index, column) in columns[3..9].iter().enumerate() {
            let value: f64 = column
                .parse()
                .map_err(|_| format!("invalid metric at line {}", row_index + 2))?;
            if !value.is_finite() || value < 0.0 {
                return Err(format!("non-finite or negative metric at line {}", row_index + 2));
            }
            metrics[metric_index] = value;
        }
        let max_iterations: usize = columns[9]
            .parse()
            .map_err(|_| format!("invalid iteration count at line {}", row_index + 2))?;

        let final_energy_drift = metrics[1];
        let maximum_energy_drift = metrics[2];
        let position_residual = metrics[3];
        let tangent_residual = metrics[4];
        let mean_iterations = metrics[5];

        if final_energy_drift > maximum_energy_drift + 1.0e-14 {
            return Err(format!("final energy drift exceeds maximum at line {}", row_index + 2));
        }
        if position_residual > 1.0e-10 || tangent_residual > 1.0e-10 {
            return Err(format!("constraint residual exceeds bound at line {}", row_index + 2));
        }
        if mean_iterations > max_iterations as f64 || max_iterations > 32 {
            return Err(format!("iteration accounting is inconsistent at line {}", row_index + 2));
        }
        if (method_index == 2 || method_index == 3) && max_iterations != 0 {
            return Err(format!("explicit method reports nonlinear iterations at line {}", row_index + 2));
        }

        seen[method_index][dt_index] = true;
        record_count += 1;
    }

    if record_count != METHODS.len() * TIME_STEPS.len() {
        return Err(format!("expected 16 result rows, found {record_count}"));
    }
    if seen.iter().any(|method_rows| method_rows.iter().any(|present| !present)) {
        return Err("missing method/timestep pair".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_report_auditor_accepts_the_frozen_campaign() {
        audit_campaign(&campaign_csv()).unwrap();
    }

    #[test]
    fn independent_report_auditor_rejects_nan_duplicate_and_missing_rows() {
        let good = campaign_csv();
        let poisoned = good.replacen("4.00000000", "NaN", 1);
        assert!(audit_campaign(&poisoned).is_err());

        let mut lines: Vec<&str> = good.lines().collect();
        let duplicate = format!("{}\n{}", good, lines[1]);
        assert!(audit_campaign(&duplicate).is_err());

        lines.pop();
        let truncated = lines.join("\n");
        assert!(audit_campaign(&truncated).is_err());
    }

    #[test]
    fn campaign_contains_all_methods_at_every_matched_step_size() {
        let csv = campaign_csv();
        assert_eq!(csv.lines().count(), 17);
        for method in [
            "intrinsic_discrete_gradient",
            "cartesian_rattle",
            "symplectic_euler",
            "rk4",
        ] {
            for dt in TIME_STEPS {
                let needle = format!("{method},{dt:.8},");
                assert!(csv.lines().any(|line| line.starts_with(&needle)));
            }
        }
    }

    #[test]
    fn campaign_replay_is_byte_identical() {
        assert_eq!(campaign_csv(), campaign_csv());
    }

    #[test]
    fn all_methods_keep_the_intrinsic_circle_geometry() {
        let csv = campaign_csv();
        for line in csv.lines().skip(1) {
            let columns: Vec<&str> = line.split(',').collect();
            let position_residual: f64 = columns[6].parse().unwrap();
            let tangent_residual: f64 = columns[7].parse().unwrap();
            assert!(position_residual < 1.0e-10, "{line}");
            assert!(tangent_residual < 1.0e-10, "{line}");
        }
    }

    #[test]
    fn intrinsic_gradient_has_lower_energy_drift_than_symplectic_euler_on_frozen_case() {
        let csv = campaign_csv();
        let mut dg_drift = None;
        let mut symplectic_drift = None;
        for line in csv.lines().skip(1) {
            let columns: Vec<&str> = line.split(',').collect();
            if columns[1] == "0.00500000" {
                let drift: f64 = columns[5].parse().unwrap();
                match columns[0] {
                    "intrinsic_discrete_gradient" => dg_drift = Some(drift),
                    "symplectic_euler" => symplectic_drift = Some(drift),
                    _ => {}
                }
            }
        }
        assert!(
            dg_drift.unwrap() < symplectic_drift.unwrap(),
            "comparison is descriptive for this frozen fixture, not a universal ranking"
        );
    }
}

#[cfg(not(test))]
fn main() {
    let csv = campaign_csv();
    audit_campaign(&csv).expect("campaign report must pass independent structural audit");
    print!("{csv}");
}
