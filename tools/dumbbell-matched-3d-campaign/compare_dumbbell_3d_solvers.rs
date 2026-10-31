// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Matched pHDAE discrete-gradient and Cartesian RATTLE campaign for the
// same free 3D constrained dumbbell, measured against an exact Rodrigues
// trajectory oracle.

use std::fmt::Write as _;

#[allow(dead_code)]
#[path = "../phdae-dumbbell-3d-reference/constrained_dumbbell_3d.rs"]
mod phdae;

#[allow(dead_code)]
#[path = "../dumbbell-rattle-3d-reference/cartesian_rattle_3d.rs"]
mod rattle;

const MASS_1: f64 = 1.3;
const MASS_2: f64 = 2.1;
const LENGTH: f64 = 1.2;
const HORIZON: f64 = 1.0;
const TIME_STEPS: [f64; 4] = [0.04, 0.02, 0.01, 0.005];

fn initial_state() -> phdae::State {
    let center = [0.25, -0.12, 0.30];
    let center_velocity = [0.17, -0.23, 0.11];
    let relative_position = [0.8, 0.8, 0.4];
    let relative_velocity = [0.2, -0.1, -0.2];
    let total_mass = MASS_1 + MASS_2;
    phdae::State {
        q1: phdae::sub(center, phdae::scale(relative_position, MASS_2 / total_mass)),
        q2: phdae::add(center, phdae::scale(relative_position, MASS_1 / total_mass)),
        v1: phdae::sub(center_velocity, phdae::scale(relative_velocity, MASS_2 / total_mass)),
        v2: phdae::add(center_velocity, phdae::scale(relative_velocity, MASS_1 / total_mass)),
    }
}

fn state_error(a: phdae::State, b: phdae::State) -> f64 {
    (phdae::norm(phdae::sub(a.q1, b.q1)).powi(2)
        + phdae::norm(phdae::sub(a.q2, b.q2)).powi(2)
        + phdae::norm(phdae::sub(a.v1, b.v1)).powi(2)
        + phdae::norm(phdae::sub(a.v2, b.v2)).powi(2))
    .sqrt()
}

#[derive(Clone, Copy, Debug)]
struct Metrics {
    method: &'static str,
    dt: f64,
    steps: usize,
    final_state_error: f64,
    max_relative_energy_drift: f64,
    max_relative_link_constraint: f64,
    max_relative_midpoint_constraint: f64,
    max_absolute_endpoint_tangent_residual: f64,
    max_relative_linear_momentum_residual: f64,
    max_relative_angular_momentum_residual: f64,
    mean_multiplier_iterations: f64,
    max_multiplier_iterations: usize,
}

fn run_phdae(
    dt: f64,
    steps: usize,
    initial: phdae::State,
    reference: phdae::State,
) -> Metrics {
    let model = phdae::ConstrainedDumbbell3D::new(MASS_1, MASS_2, LENGTH)
        .expect("frozen pHDAE model must be valid");
    let mut state = initial;
    let energy0 = model.hamiltonian(state).expect("initial energy must be finite");
    let mut max_energy = 0.0_f64;
    let mut max_link = 0.0_f64;
    let mut max_midpoint = 0.0_f64;
    let mut max_endpoint = 0.0_f64;
    let mut max_linear = 0.0_f64;
    let mut max_angular = 0.0_f64;
    let mut total_iterations = 0usize;
    let mut maximum_iterations = 0usize;

    for _ in 0..steps {
        let receipt = model
            .step(
                state,
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                dt,
                phdae::SolverOptions::default(),
            )
            .expect("pHDAE 3D step must converge");
        state = receipt.after;
        max_energy = max_energy.max(
            (receipt.energy_after - energy0).abs()
                / energy0.abs().max(f64::MIN_POSITIVE),
        );
        max_link = max_link.max(receipt.relative_link_constraint_residual);
        max_midpoint =
            max_midpoint.max(receipt.relative_midpoint_velocity_constraint_residual);
        max_endpoint =
            max_endpoint.max(receipt.endpoint_velocity_constraint_residual.abs());
        max_linear = max_linear.max(receipt.relative_linear_momentum_residual);
        max_angular = max_angular.max(receipt.relative_angular_momentum_residual);
        total_iterations += receipt.multiplier_iterations;
        maximum_iterations = maximum_iterations.max(receipt.multiplier_iterations);
    }

    Metrics {
        method: "semi_explicit_phdae_dg_3d",
        dt,
        steps,
        final_state_error: state_error(state, reference),
        max_relative_energy_drift: max_energy,
        max_relative_link_constraint: max_link,
        max_relative_midpoint_constraint: max_midpoint,
        max_absolute_endpoint_tangent_residual: max_endpoint,
        max_relative_linear_momentum_residual: max_linear,
        max_relative_angular_momentum_residual: max_angular,
        mean_multiplier_iterations: total_iterations as f64 / steps as f64,
        max_multiplier_iterations: maximum_iterations,
    }
}

fn run_rattle(
    dt: f64,
    steps: usize,
    initial: phdae::State,
    reference: phdae::State,
) -> Metrics {
    let model = rattle::CartesianRattleDumbbell3D::new(MASS_1, MASS_2, LENGTH)
        .expect("frozen RATTLE model must be valid");
    let mut state = rattle::State {
        q1: initial.q1,
        q2: initial.q2,
        v1: initial.v1,
        v2: initial.v2,
    };
    let energy0 = model.hamiltonian(state).expect("initial energy must be finite");
    let mut max_energy = 0.0_f64;
    let mut max_link = 0.0_f64;
    let mut max_midpoint = 0.0_f64;
    let mut max_endpoint = 0.0_f64;
    let mut max_linear = 0.0_f64;
    let mut max_angular = 0.0_f64;

    for _ in 0..steps {
        let receipt = model.step(state, dt).expect("RATTLE 3D step must be admissible");
        state = receipt.after;
        max_energy = max_energy.max(
            (receipt.energy_after - energy0).abs()
                / energy0.abs().max(f64::MIN_POSITIVE),
        );
        max_link = max_link.max(receipt.relative_link_constraint_residual);
        let r0 = rattle::sub(receipt.before.q2, receipt.before.q1);
        let r1 = rattle::sub(state.q2, state.q1);
        let w0 = rattle::sub(receipt.before.v2, receipt.before.v1);
        let w1 = rattle::sub(state.v2, state.v1);
        let rbar = rattle::scale(rattle::add(r0, r1), 0.5);
        let wbar = rattle::scale(rattle::add(w0, w1), 0.5);
        max_midpoint = max_midpoint.max(
            rattle::dot(rbar, wbar).abs()
                / (1.0 + rattle::norm(rbar) * rattle::norm(wbar)),
        );
        max_endpoint =
            max_endpoint.max(receipt.endpoint_velocity_constraint_residual.abs());
        max_linear = max_linear.max(receipt.relative_linear_momentum_residual);
        max_angular = max_angular.max(receipt.relative_angular_momentum_residual);
    }

    let final_state = phdae::State {
        q1: state.q1,
        q2: state.q2,
        v1: state.v1,
        v2: state.v2,
    };
    Metrics {
        method: "cartesian_rattle_3d",
        dt,
        steps,
        final_state_error: state_error(final_state, reference),
        max_relative_energy_drift: max_energy,
        max_relative_link_constraint: max_link,
        max_relative_midpoint_constraint: max_midpoint,
        max_absolute_endpoint_tangent_residual: max_endpoint,
        max_relative_linear_momentum_residual: max_linear,
        max_relative_angular_momentum_residual: max_angular,
        mean_multiplier_iterations: 0.0,
        max_multiplier_iterations: 0,
    }
}

fn campaign_csv() -> String {
    let initial = initial_state();
    let reference =
        phdae::exact_free_dumbbell_3d(initial, MASS_1, MASS_2, LENGTH, HORIZON);
    let mut output = String::new();
    writeln!(
        &mut output,
        "method,dt,steps,final_state_error,max_relative_energy_drift,max_relative_link_constraint,max_relative_midpoint_constraint,max_absolute_endpoint_tangent_residual,max_relative_linear_momentum_residual,max_relative_angular_momentum_residual,mean_multiplier_iterations,max_multiplier_iterations"
    )
    .expect("writing to String cannot fail");

    for dt in TIME_STEPS {
        let steps = (HORIZON / dt).round() as usize;
        for result in [
            run_phdae(dt, steps, initial, reference),
            run_rattle(dt, steps, initial, reference),
        ] {
            writeln!(
                &mut output,
                "{},{:.8},{},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.6},{}",
                result.method,
                result.dt,
                result.steps,
                result.final_state_error,
                result.max_relative_energy_drift,
                result.max_relative_link_constraint,
                result.max_relative_midpoint_constraint,
                result.max_absolute_endpoint_tangent_residual,
                result.max_relative_linear_momentum_residual,
                result.max_relative_angular_momentum_residual,
                result.mean_multiplier_iterations,
                result.max_multiplier_iterations
            )
            .expect("writing to String cannot fail");
        }
    }
    output
}

fn audit_campaign(csv: &str) -> Result<(), String> {
    const METHODS: [&str; 2] = ["semi_explicit_phdae_dg_3d", "cartesian_rattle_3d"];
    const HEADER: &str = "method,dt,steps,final_state_error,max_relative_energy_drift,max_relative_link_constraint,max_relative_midpoint_constraint,max_absolute_endpoint_tangent_residual,max_relative_linear_momentum_residual,max_relative_angular_momentum_residual,mean_multiplier_iterations,max_multiplier_iterations";

    let mut lines = csv.lines();
    if lines.next() != Some(HEADER) {
        return Err("unexpected or missing CSV header".to_string());
    }
    let mut seen = [[false; 4]; 2];
    let mut count = 0usize;

    for (row, line) in lines.enumerate() {
        let line_number = row + 2;
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 12 {
            return Err(format!("expected 12 columns at line {line_number}"));
        }
        let method = METHODS
            .iter()
            .position(|name| *name == fields[0])
            .ok_or_else(|| format!("unknown method at line {line_number}"))?;
        let dt: f64 = fields[1]
            .parse()
            .map_err(|_| format!("invalid timestep at line {line_number}"))?;
        let step_index = TIME_STEPS
            .iter()
            .position(|expected| (dt - expected).abs() < 1.0e-10)
            .ok_or_else(|| format!("unexpected timestep at line {line_number}"))?;
        if seen[method][step_index] {
            return Err(format!("duplicate method/timestep at line {line_number}"));
        }
        let steps: usize = fields[2]
            .parse()
            .map_err(|_| format!("invalid step count at line {line_number}"))?;
        if steps != (HORIZON / dt).round() as usize {
            return Err(format!("incorrect step count at line {line_number}"));
        }

        let mut metrics = [0.0_f64; 8];
        for (index, field) in fields[3..11].iter().enumerate() {
            let value: f64 = field
                .parse()
                .map_err(|_| format!("invalid metric at line {line_number}"))?;
            if !value.is_finite() || value < 0.0 {
                return Err(format!("non-finite or negative metric at line {line_number}"));
            }
            metrics[index] = value;
        }
        let mean_iterations: f64 = fields[10]
            .parse()
            .map_err(|_| format!("invalid mean iteration count at line {line_number}"))?;
        let max_iterations: usize = fields[11]
            .parse()
            .map_err(|_| format!("invalid max iteration count at line {line_number}"))?;
        if !mean_iterations.is_finite()
            || mean_iterations < 0.0
            || mean_iterations > max_iterations as f64
            || max_iterations > 32
        {
            return Err(format!("invalid iteration accounting at line {line_number}"));
        }
        if method == 1 && max_iterations != 0 {
            return Err(format!("RATTLE reports multiplier iterations at line {line_number}"));
        }
        if metrics[1] > 1.0e-8
            || metrics[2] > 1.0e-10
            || metrics[3] > 1.0e-10
            || metrics[4] > 1.0e-8
            || metrics[5] > 1.0e-8
            || metrics[6] > 1.0e-8
        {
            return Err(format!("constraint or momentum residual outside envelope at line {line_number}"));
        }

        seen[method][step_index] = true;
        count += 1;
    }
    if count != 8 || seen.iter().any(|row| row.iter().any(|present| !present)) {
        return Err("campaign must contain exactly one row per method/timestep".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_initial_state_horizon_and_timestep_ladder() {
        let csv = campaign_csv();
        audit_campaign(&csv).unwrap();
        assert_eq!(csv.lines().count(), 9);
        for method in ["semi_explicit_phdae_dg_3d", "cartesian_rattle_3d"] {
            for dt in TIME_STEPS {
                let prefix = format!("{method},{dt:.8},");
                assert!(csv.lines().any(|line| line.starts_with(&prefix)));
            }
        }
    }

    #[test]
    fn campaign_replay_is_byte_identical() {
        assert_eq!(campaign_csv(), campaign_csv());
    }

    #[test]
    fn both_methods_converge_against_the_exact_rodrigues_trajectory() {
        let csv = campaign_csv();
        for method in ["semi_explicit_phdae_dg_3d", "cartesian_rattle_3d"] {
            let errors: Vec<f64> = csv
                .lines()
                .skip(1)
                .filter(|line| line.starts_with(method))
                .map(|line| line.split(',').nth(3).unwrap().parse().unwrap())
                .collect();
            assert_eq!(errors.len(), 4);
            for pair in errors.windows(2) {
                let ratio = pair[0] / pair[1];
                assert!(
                    (3.6..4.4).contains(&ratio),
                    "expected second-order convergence for {method}: {errors:?}"
                );
            }
        }
    }

    #[test]
    fn all_methods_preserve_the_declared_constraint_and_momentum_envelopes() {
        let csv = campaign_csv();
        for line in csv.lines().skip(1) {
            let fields: Vec<&str> = line.split(',').collect();
            let link: f64 = fields[5].parse().unwrap();
            let midpoint: f64 = fields[6].parse().unwrap();
            let endpoint: f64 = fields[7].parse().unwrap();
            let linear: f64 = fields[8].parse().unwrap();
            let angular: f64 = fields[9].parse().unwrap();
            assert!(link < 1.0e-10, "{line}");
            assert!(midpoint < 1.0e-8, "{line}");
            assert!(endpoint < 1.0e-8, "{line}");
            assert!(linear < 1.0e-8, "{line}");
            assert!(angular < 1.0e-8, "{line}");
        }
    }

    #[test]
    fn audit_rejects_nan_duplicate_and_missing_rows() {
        let good = campaign_csv();
        let poisoned = good
            .lines()
            .enumerate()
            .map(|(index, line)| {
                if index == 1 {
                    let mut columns: Vec<String> =
                        line.split(',').map(str::to_string).collect();
                    columns[3] = "NaN".to_string();
                    columns.join(",")
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(audit_campaign(&poisoned).is_err());
        let rows: Vec<&str> = good.lines().collect();
        let duplicate = format!("{}\n{}", good.trim_end(), rows[1]);
        assert!(audit_campaign(&duplicate).is_err());
        let truncated = rows[..rows.len() - 1].join("\n");
        assert!(audit_campaign(&truncated).is_err());
    }
}

#[cfg(not(test))]
fn main() {
    let csv = campaign_csv();
    audit_campaign(&csv).expect("3D matched campaign must pass receipt audit");
    print!("{csv}");
}
