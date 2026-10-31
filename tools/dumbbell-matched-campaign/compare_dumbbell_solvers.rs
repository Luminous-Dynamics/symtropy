// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Matched-budget comparison of pHDAE discrete gradient and Cartesian RATTLE
// on one free constrained dumbbell with an independent analytic solution.

use std::fmt::Write as _;

#[allow(dead_code)]
#[path = "../phdae-dumbbell-reference/constrained_dumbbell.rs"]
mod phdae;

#[allow(dead_code)]
#[path = "../dumbbell-rattle-reference/cartesian_rattle.rs"]
mod rattle;

const MASS_1: f64 = 1.3;
const MASS_2: f64 = 2.1;
const LENGTH: f64 = 1.2;
const HORIZON: f64 = 1.0;
const TIME_STEPS: [f64; 4] = [0.04, 0.02, 0.01, 0.005];

fn initial_state() -> phdae::State {
    phdae::State {
        q1: [-0.6, 0.2],
        q2: [0.6, 0.2],
        v1: [0.25, -0.18],
        v2: [0.25, 0.62],
    }
}

fn rotate(a: [f64; 2], angle: f64) -> [f64; 2] {
    let (sin, cos) = angle.sin_cos();
    [cos * a[0] - sin * a[1], sin * a[0] + cos * a[1]]
}

fn norm(a: [f64; 2]) -> f64 {
    (a[0] * a[0] + a[1] * a[1]).sqrt()
}

fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn state_error(a: phdae::State, b: phdae::State) -> f64 {
    (norm(sub(a.q1, b.q1)).powi(2)
        + norm(sub(a.q2, b.q2)).powi(2)
        + norm(sub(a.v1, b.v1)).powi(2)
        + norm(sub(a.v2, b.v2)).powi(2))
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
    max_relative_midpoint_velocity_constraint: f64,
    max_absolute_endpoint_tangent_residual: f64,
    max_relative_linear_momentum_residual: f64,
    max_relative_angular_momentum_residual: f64,
    mean_multiplier_iterations: f64,
    max_multiplier_iterations: usize,
}

fn run_phdae(
    initial: phdae::State,
    reference: phdae::State,
    dt: f64,
    steps: usize,
) -> Metrics {
    let model = phdae::ConstrainedDumbbell::new(MASS_1, MASS_2, LENGTH)
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
                [0.0, 0.0],
                [0.0, 0.0],
                dt,
                phdae::SolverOptions::default(),
            )
            .expect("pHDAE step must converge");
        state = receipt.after;
        max_energy = max_energy.max(
            (receipt.energy_after - energy0).abs()
                / energy0.abs().max(f64::MIN_POSITIVE),
        );
        max_link = max_link.max(receipt.relative_link_constraint_residual);
        max_midpoint = max_midpoint
            .max(receipt.relative_midpoint_velocity_constraint_residual);
        max_endpoint =
            max_endpoint.max(receipt.endpoint_velocity_constraint_residual.abs());
        max_linear = max_linear.max(receipt.relative_linear_momentum_residual);
        max_angular = max_angular.max(receipt.relative_angular_momentum_residual);
        total_iterations += receipt.multiplier_iterations;
        maximum_iterations = maximum_iterations.max(receipt.multiplier_iterations);
    }

    Metrics {
        method: "semi_explicit_phdae_dg",
        dt,
        steps,
        final_state_error: state_error(state, reference),
        max_relative_energy_drift: max_energy,
        max_relative_link_constraint: max_link,
        max_relative_midpoint_velocity_constraint: max_midpoint,
        max_absolute_endpoint_tangent_residual: max_endpoint,
        max_relative_linear_momentum_residual: max_linear,
        max_relative_angular_momentum_residual: max_angular,
        mean_multiplier_iterations: total_iterations as f64 / steps as f64,
        max_multiplier_iterations: maximum_iterations,
    }
}

fn run_rattle(
    initial: phdae::State,
    reference: phdae::State,
    dt: f64,
    steps: usize,
) -> Metrics {
    let model = rattle::CartesianRattleDumbbell::new(MASS_1, MASS_2, LENGTH)
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
        let before = state;
        let receipt = model.step(state, dt).expect("RATTLE step must be admissible");
        state = receipt.after;
        max_energy = max_energy.max(
            (receipt.energy_after - energy0).abs()
                / energy0.abs().max(f64::MIN_POSITIVE),
        );
        max_link =
            max_link.max(receipt.relative_link_constraint_residual);

        // A comparable midpoint diagnostic is computed from the same
        // Cartesian observables; unlike pHDAE, RATTLE does not enforce it
        // as its algebraic multiplier equation.
        let r0 = sub(before.q2, before.q1);
        let r1 = sub(state.q2, state.q1);
        let w0 = sub(before.v2, before.v1);
        let w1 = sub(state.v2, state.v1);
        let rbar = [(r0[0] + r1[0]) * 0.5, (r0[1] + r1[1]) * 0.5];
        let wbar = [(w0[0] + w1[0]) * 0.5, (w0[1] + w1[1]) * 0.5];
        let midpoint = dot(rbar, wbar).abs()
            / (1.0 + norm(rbar) * norm(wbar));
        max_midpoint = max_midpoint.max(midpoint);
        max_endpoint =
            max_endpoint.max(receipt.endpoint_velocity_constraint_residual.abs());
        max_linear = max_linear.max(receipt.relative_linear_momentum_residual);
        max_angular = max_angular.max(receipt.relative_angular_momentum_residual);
    }

    Metrics {
        method: "cartesian_rattle",
        dt,
        steps,
        final_state_error: state_error(
            phdae::State {
                q1: state.q1,
                q2: state.q2,
                v1: state.v1,
                v2: state.v2,
            },
            reference,
        ),
        max_relative_energy_drift: max_energy,
        max_relative_link_constraint: max_link,
        max_relative_midpoint_velocity_constraint: max_midpoint,
        max_absolute_endpoint_tangent_residual: max_endpoint,
        max_relative_linear_momentum_residual: max_linear,
        max_relative_angular_momentum_residual: max_angular,
        mean_multiplier_iterations: 0.0,
        max_multiplier_iterations: 0,
    }
}

fn campaign_csv() -> String {
    let initial = initial_state();
    let reference = phdae::exact_free_dumbbell(
        initial,
        MASS_1,
        MASS_2,
        LENGTH,
        HORIZON,
    );
    let mut output = String::new();
    writeln!(
        &mut output,
        "method,dt,steps,final_state_error,max_relative_energy_drift,max_relative_link_constraint,max_relative_midpoint_velocity_constraint,max_absolute_endpoint_tangent_residual,max_relative_linear_momentum_residual,max_relative_angular_momentum_residual,mean_multiplier_iterations,max_multiplier_iterations"
    )
    .expect("writing to String cannot fail");

    for dt in TIME_STEPS {
        let steps = (HORIZON / dt).round() as usize;
        for result in [
            run_phdae(initial, reference, dt, steps),
            run_rattle(initial, reference, dt, steps),
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
                result.max_relative_midpoint_velocity_constraint,
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
    const METHODS: [&str; 2] = ["semi_explicit_phdae_dg", "cartesian_rattle"];
    const HEADER: &str = "method,dt,steps,final_state_error,max_relative_energy_drift,max_relative_link_constraint,max_relative_midpoint_velocity_constraint,max_absolute_endpoint_tangent_residual,max_relative_linear_momentum_residual,max_relative_angular_momentum_residual,mean_multiplier_iterations,max_multiplier_iterations";

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
        for (i, field) in fields[3..11].iter().enumerate() {
            let value: f64 = field
                .parse()
                .map_err(|_| format!("invalid numeric metric at line {line_number}"))?;
            if !value.is_finite() || value < 0.0 {
                return Err(format!("non-finite or negative metric at line {line_number}"));
            }
            metrics[i] = value;
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
            return Err(format!("RATTLE must not report Newton iterations at line {line_number}"));
        }
        if metrics[1] > 1.0e-8
            || metrics[2] > 1.0e-10
            || metrics[4] > 1.0e-8
            || metrics[5] > 1.0e-8
            || metrics[6] > 1.0e-8
        {
            return Err(format!("physical residual outside campaign envelope at line {line_number}"));
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
    fn same_horizon_and_step_ladder_are_used_for_both_methods() {
        let csv = campaign_csv();
        assert_eq!(csv.lines().count(), 9);
        audit_campaign(&csv).unwrap();
        for method in ["semi_explicit_phdae_dg", "cartesian_rattle"] {
            for dt in TIME_STEPS {
                let needle = format!("{method},{dt:.8},");
                assert!(csv.lines().any(|line| line.starts_with(&needle)));
            }
        }
    }

    #[test]
    fn receipt_auditor_rejects_nan_duplicate_and_missing_row() {
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
        let duplicate = format!("{}\n{}", good, rows[1]);
        assert!(audit_campaign(&duplicate).is_err());

        let truncated = rows[..rows.len() - 1].join("\n");
        assert!(audit_campaign(&truncated).is_err());
    }

    #[test]
    fn pHDAE_and_RATTLE_both_match_the_analytic_free_solution_at_second_order() {
        let csv = campaign_csv();
        for line in csv.lines().skip(1) {
            let fields: Vec<&str> = line.split(',').collect();
            let error: f64 = fields[3].parse().unwrap();
            assert!(error.is_finite() && error >= 0.0);
        }
        let rows: Vec<&str> = csv.lines().skip(1).collect();
        for method in ["semi_explicit_phdae_dg", "cartesian_rattle"] {
            let errors: Vec<f64> = rows
                .iter()
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
    fn both_methods_preserve_link_linear_and_angular_momentum_maps() {
        let csv = campaign_csv();
        for line in csv.lines().skip(1) {
            let fields: Vec<&str> = line.split(',').collect();
            let link: f64 = fields[5].parse().unwrap();
            let endpoint_tangent: f64 = fields[7].parse().unwrap();
            let linear: f64 = fields[8].parse().unwrap();
            let angular: f64 = fields[9].parse().unwrap();
            assert!(link < 1.0e-10, "{line}");
            assert!(endpoint_tangent < 2.0e-10, "{line}");
            assert!(linear < 1.0e-8, "{line}");
            assert!(angular < 1.0e-8, "{line}");
        }
    }
}

#[cfg(not(test))]
fn main() {
    let csv = campaign_csv();
    audit_campaign(&csv).expect("matched dumbbell campaign must pass receipt audit");
    print!("{csv}");
}
