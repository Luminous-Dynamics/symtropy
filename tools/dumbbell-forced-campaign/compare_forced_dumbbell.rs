// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Matched pHDAE discrete-gradient and Cartesian RATTLE campaign under
// constant common acceleration, with an independent analytic solution.

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
const ACCELERATION: [f64; 2] = [0.18, -0.07];
const TIME_STEPS: [f64; 4] = [0.04, 0.02, 0.01, 0.005];

fn initial_state() -> phdae::State {
    phdae::State {
        q1: [-0.6, 0.2],
        q2: [0.6, 0.2],
        v1: [0.25, -0.18],
        v2: [0.25, 0.62],
    }
}

fn add(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] + b[0], a[1] + b[1]]
}

fn scale(a: [f64; 2], factor: f64) -> [f64; 2] {
    [a[0] * factor, a[1] * factor]
}

fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn norm(a: [f64; 2]) -> f64 {
    dot(a, a).sqrt()
}

fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

fn state_error(a: phdae::State, b: phdae::State) -> f64 {
    (norm(sub(a.q1, b.q1)).powi(2)
        + norm(sub(a.q2, b.q2)).powi(2)
        + norm(sub(a.v1, b.v1)).powi(2)
        + norm(sub(a.v2, b.v2)).powi(2))
    .sqrt()
}

fn exact_forced_state(time: f64) -> phdae::State {
    let initial = initial_state();
    let free = phdae::exact_free_dumbbell(initial, MASS_1, MASS_2, LENGTH, time);
    let displacement = scale(ACCELERATION, 0.5 * time * time);
    let velocity_change = scale(ACCELERATION, time);
    phdae::State {
        q1: add(free.q1, displacement),
        q2: add(free.q2, displacement),
        v1: add(free.v1, velocity_change),
        v2: add(free.v2, velocity_change),
    }
}

fn analytic_external_impulse() -> [f64; 2] {
    scale(
        ACCELERATION,
        (MASS_1 + MASS_2) * HORIZON,
    )
}

fn analytic_external_torque_impulse() -> f64 {
    let initial = initial_state();
    let total_mass = MASS_1 + MASS_2;
    let center_position = scale(
        add(
            scale(initial.q1, MASS_1),
            scale(initial.q2, MASS_2),
        ),
        1.0 / total_mass,
    );
    let center_velocity = scale(
        add(
            scale(initial.v1, MASS_1),
            scale(initial.v2, MASS_2),
        ),
        1.0 / total_mass,
    );
    total_mass
        * (cross(center_position, ACCELERATION) * HORIZON
            + 0.5 * cross(center_velocity, ACCELERATION) * HORIZON * HORIZON)
}

fn analytic_external_work() -> f64 {
    let initial = initial_state();
    let total_mass = MASS_1 + MASS_2;
    let center_velocity = scale(
        add(
            scale(initial.v1, MASS_1),
            scale(initial.v2, MASS_2),
        ),
        1.0 / total_mass,
    );
    total_mass
        * dot(
            ACCELERATION,
            add(
                scale(center_velocity, HORIZON),
                scale(ACCELERATION, 0.5 * HORIZON * HORIZON),
            ),
        )
}

#[derive(Clone, Copy, Debug)]
struct Metrics {
    method: &'static str,
    dt: f64,
    steps: usize,
    final_state_error: f64,
    max_relative_energy_balance_residual: f64,
    final_relative_energy_work_defect: f64,
    max_relative_link_constraint: f64,
    max_relative_midpoint_velocity_constraint: f64,
    max_absolute_endpoint_tangent_residual: f64,
    max_relative_linear_momentum_residual: f64,
    max_relative_angular_momentum_residual: f64,
    cumulative_external_work: f64,
    final_linear_momentum_delta: [f64; 2],
    final_angular_momentum_delta: f64,
    mean_multiplier_iterations: f64,
    max_multiplier_iterations: usize,
}

fn run_phdae(dt: f64, steps: usize, reference: phdae::State) -> Metrics {
    let model = phdae::ConstrainedDumbbell::new(MASS_1, MASS_2, LENGTH)
        .expect("frozen pHDAE model must be valid");
    let initial = initial_state();
    let mut state = initial;
    let force1 = scale(ACCELERATION, MASS_1);
    let force2 = scale(ACCELERATION, MASS_2);
    let energy0 = model.hamiltonian(state).expect("initial energy must be finite");
    let mut cumulative_work = 0.0;
    let mut max_energy_balance = 0.0_f64;
    let mut max_energy_work_defect = 0.0_f64;
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
                force1,
                force2,
                dt,
                phdae::SolverOptions::default(),
            )
            .expect("pHDAE forced step must converge");
        state = receipt.after;
        cumulative_work += receipt.external_work;
        max_energy_balance =
            max_energy_balance.max(receipt.relative_energy_balance_residual);
        let work_defect = (receipt.energy_after - energy0 - cumulative_work).abs()
            / (1.0
                + energy0.abs()
                + receipt.energy_after.abs()
                + cumulative_work.abs());
        max_energy_work_defect = max_energy_work_defect.max(work_defect);
        max_link = max_link.max(receipt.relative_link_constraint_residual);
        max_midpoint =
            max_midpoint.max(receipt.relative_midpoint_velocity_constraint_residual);
        max_endpoint = max_endpoint.max(receipt.endpoint_velocity_constraint_residual.abs());
        max_linear = max_linear.max(receipt.relative_linear_momentum_residual);
        max_angular = max_angular.max(receipt.relative_angular_momentum_residual);
        total_iterations += receipt.multiplier_iterations;
        maximum_iterations = maximum_iterations.max(receipt.multiplier_iterations);
    }

    let initial_linear_momentum = model.linear_momentum(initial);
    let final_linear_momentum = model.linear_momentum(state);
    let initial_angular_momentum = model.angular_momentum(initial);
    let final_angular_momentum = model.angular_momentum(state);

    Metrics {
        method: "semi_explicit_phdae_dg",
        dt,
        steps,
        final_state_error: state_error(state, reference),
        max_relative_energy_balance_residual: max_energy_balance,
        final_relative_energy_work_defect: max_energy_work_defect,
        max_relative_link_constraint: max_link,
        max_relative_midpoint_velocity_constraint: max_midpoint,
        max_absolute_endpoint_tangent_residual: max_endpoint,
        max_relative_linear_momentum_residual: max_linear,
        max_relative_angular_momentum_residual: max_angular,
        cumulative_external_work: cumulative_work,
        final_linear_momentum_delta: sub(final_linear_momentum, initial_linear_momentum),
        final_angular_momentum_delta: final_angular_momentum - initial_angular_momentum,
        mean_multiplier_iterations: total_iterations as f64 / steps as f64,
        max_multiplier_iterations: maximum_iterations,
    }
}

fn run_rattle(dt: f64, steps: usize, reference: phdae::State) -> Metrics {
    let model = rattle::CartesianRattleDumbbell::new(MASS_1, MASS_2, LENGTH)
        .expect("frozen RATTLE model must be valid");
    let initial = initial_state();
    let mut state = rattle::State {
        q1: initial.q1,
        q2: initial.q2,
        v1: initial.v1,
        v2: initial.v2,
    };
    let energy0 = model.hamiltonian(state).expect("initial energy must be finite");
    let force_acceleration = ACCELERATION;
    let mut cumulative_work = 0.0;
    let mut max_energy_balance = 0.0_f64;
    let mut max_energy_work_defect = 0.0_f64;
    let mut max_link = 0.0_f64;
    let mut max_midpoint = 0.0_f64;
    let mut max_endpoint = 0.0_f64;
    let mut max_linear = 0.0_f64;
    let mut max_angular = 0.0_f64;

    for _ in 0..steps {
        let receipt = model
            .step_with_accelerations(
                state,
                force_acceleration,
                force_acceleration,
                dt,
            )
            .expect("RATTLE forced step must be admissible");
        state = receipt.after;
        cumulative_work += receipt.external_work;
        max_energy_balance =
            max_energy_balance.max(receipt.relative_energy_balance_residual);
        let work_defect = (receipt.energy_after - energy0 - cumulative_work).abs()
            / (1.0
                + energy0.abs()
                + receipt.energy_after.abs()
                + cumulative_work.abs());
        max_energy_work_defect = max_energy_work_defect.max(work_defect);
        max_link = max_link.max(receipt.relative_link_constraint_residual);
        let r0 = sub(receipt.before.q2, receipt.before.q1);
        let r1 = sub(state.q2, state.q1);
        let w0 = sub(receipt.before.v2, receipt.before.v1);
        let w1 = sub(state.v2, state.v1);
        let rbar = scale(add(r0, r1), 0.5);
        let wbar = scale(add(w0, w1), 0.5);
        max_midpoint = max_midpoint.max(
            (dot(rbar, wbar)).abs()
                / (1.0 + norm(rbar) * norm(wbar)),
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
    let initial_rattle = rattle::State {
        q1: initial.q1,
        q2: initial.q2,
        v1: initial.v1,
        v2: initial.v2,
    };
    let initial_linear_momentum = model.linear_momentum(initial_rattle);
    let final_linear_momentum = model.linear_momentum(state);
    let initial_angular_momentum = model.angular_momentum(initial_rattle);
    let final_angular_momentum = model.angular_momentum(state);
    Metrics {
        method: "cartesian_rattle",
        dt,
        steps,
        final_state_error: state_error(final_state, reference),
        max_relative_energy_balance_residual: max_energy_balance,
        final_relative_energy_work_defect: max_energy_work_defect,
        max_relative_link_constraint: max_link,
        max_relative_midpoint_velocity_constraint: max_midpoint,
        max_absolute_endpoint_tangent_residual: max_endpoint,
        max_relative_linear_momentum_residual: max_linear,
        max_relative_angular_momentum_residual: max_angular,
        cumulative_external_work: cumulative_work,
        final_linear_momentum_delta: sub(final_linear_momentum, initial_linear_momentum),
        final_angular_momentum_delta: final_angular_momentum - initial_angular_momentum,
        mean_multiplier_iterations: 0.0,
        max_multiplier_iterations: 0,
    }
}

fn campaign_csv() -> String {
    let reference = exact_forced_state(HORIZON);
    let mut output = String::new();
    writeln!(
        &mut output,
        "method,dt,steps,final_state_error,max_relative_energy_balance_residual,final_relative_energy_work_defect,max_relative_link_constraint,max_relative_midpoint_velocity_constraint,max_absolute_endpoint_tangent_residual,max_relative_linear_momentum_residual,max_relative_angular_momentum_residual,cumulative_external_work,linear_momentum_delta_x,linear_momentum_delta_y,angular_momentum_delta,mean_multiplier_iterations,max_multiplier_iterations"
    )
    .expect("writing to String cannot fail");

    for dt in TIME_STEPS {
        let steps = (HORIZON / dt).round() as usize;
        for result in [
            run_phdae(dt, steps, reference),
            run_rattle(dt, steps, reference),
        ] {
            writeln!(
                &mut output,
                "{},{:.8},{},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.6},{}",
                result.method,
                result.dt,
                result.steps,
                result.final_state_error,
                result.max_relative_energy_balance_residual,
                result.final_relative_energy_work_defect,
                result.max_relative_link_constraint,
                result.max_relative_midpoint_velocity_constraint,
                result.max_absolute_endpoint_tangent_residual,
                result.max_relative_linear_momentum_residual,
                result.max_relative_angular_momentum_residual,
                result.cumulative_external_work,
                result.final_linear_momentum_delta[0],
                result.final_linear_momentum_delta[1],
                result.final_angular_momentum_delta,
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
    const HEADER: &str = "method,dt,steps,final_state_error,max_relative_energy_balance_residual,final_relative_energy_work_defect,max_relative_link_constraint,max_relative_midpoint_velocity_constraint,max_absolute_endpoint_tangent_residual,max_relative_linear_momentum_residual,max_relative_angular_momentum_residual,cumulative_external_work,linear_momentum_delta_x,linear_momentum_delta_y,angular_momentum_delta,mean_multiplier_iterations,max_multiplier_iterations";

    let mut lines = csv.lines();
    if lines.next() != Some(HEADER) {
        return Err("unexpected or missing CSV header".to_string());
    }
    let mut seen = [[false; 4]; 2];
    let expected_work = analytic_external_work();
    let mut count = 0usize;

    for (row, line) in lines.enumerate() {
        let line_number = row + 2;
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 17 {
            return Err(format!("expected 17 fields at line {line_number}"));
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
            .position(|value| (dt - value).abs() < 1.0e-10)
            .ok_or_else(|| format!("unknown timestep at line {line_number}"))?;
        if seen[method][step_index] {
            return Err(format!("duplicate method/timestep at line {line_number}"));
        }
        let steps: usize = fields[2]
            .parse()
            .map_err(|_| format!("invalid step count at line {line_number}"))?;
        if steps != (HORIZON / dt).round() as usize {
            return Err(format!("incorrect step count at line {line_number}"));
        }

        let mut nonnegative_metrics = [0.0_f64; 8];
        for (i, field) in fields[3..11].iter().enumerate() {
            let value: f64 = field
                .parse()
                .map_err(|_| format!("invalid metric at line {line_number}"))?;
            if !value.is_finite() || value < 0.0 {
                return Err(format!("non-finite or negative metric at line {line_number}"));
            }
            nonnegative_metrics[i] = value;
        }
        let actual_work: f64 = fields[11]
            .parse()
            .map_err(|_| format!("invalid external work at line {line_number}"))?;
        if !actual_work.is_finite()
            || (actual_work - expected_work).abs()
                > 1.0e-10 * (1.0 + expected_work.abs())
        {
            return Err(format!("signed external-work receipt mismatch at line {line_number}"));
        }

        let actual_impulse_x: f64 = fields[12]
            .parse()
            .map_err(|_| format!("invalid x impulse at line {line_number}"))?;
        let actual_impulse_y: f64 = fields[13]
            .parse()
            .map_err(|_| format!("invalid y impulse at line {line_number}"))?;
        let actual_torque_impulse: f64 = fields[14]
            .parse()
            .map_err(|_| format!("invalid torque impulse at line {line_number}"))?;
        let expected_impulse = analytic_external_impulse();
        let expected_torque = analytic_external_torque_impulse();
        if !actual_impulse_x.is_finite()
            || !actual_impulse_y.is_finite()
            || !actual_torque_impulse.is_finite()
            || (actual_impulse_x - expected_impulse[0]).abs()
                > 1.0e-10 * (1.0 + expected_impulse[0].abs())
            || (actual_impulse_y - expected_impulse[1]).abs()
                > 1.0e-10 * (1.0 + expected_impulse[1].abs())
            || (actual_torque_impulse - expected_torque).abs()
                > 1.0e-10 * (1.0 + expected_torque.abs())
        {
            return Err(format!("signed external impulse/torque mismatch at line {line_number}"));
        }

        let mean_iterations: f64 = fields[15]
            .parse()
            .map_err(|_| format!("invalid mean iterations at line {line_number}"))?;
        let max_iterations: usize = fields[16]
            .parse()
            .map_err(|_| format!("invalid max iterations at line {line_number}"))?;
        if !mean_iterations.is_finite()
            || mean_iterations < 0.0
            || mean_iterations > max_iterations as f64
            || max_iterations > 32
        {
            return Err(format!("invalid iteration accounting at line {line_number}"));
        }
        if method == 1 && max_iterations != 0 {
            return Err(format!("RATTLE reports unexpected Newton iterations at line {line_number}"));
        }

        if nonnegative_metrics[1] > 1.0e-8
            || nonnegative_metrics[2] > 1.0e-8
            || nonnegative_metrics[3] > 1.0e-10
            || nonnegative_metrics[4] > 1.0e-8
            || nonnegative_metrics[5] > 1.0e-8
            || nonnegative_metrics[6] > 1.0e-8
            || nonnegative_metrics[7] > 1.0e-8
        {
            return Err(format!("physical balance or constraint outside envelope at line {line_number}"));
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
    fn common_acceleration_campaign_uses_identical_horizons() {
        let csv = campaign_csv();
        audit_campaign(&csv).unwrap();
        assert_eq!(csv.lines().count(), 9);
        for method in ["semi_explicit_phdae_dg", "cartesian_rattle"] {
            for dt in TIME_STEPS {
                let key = format!("{method},{dt:.8},");
                assert!(csv.lines().any(|line| line.starts_with(&key)));
            }
        }
    }

    #[test]
    fn both_methods_converge_against_analytic_accelerated_trajectory() {
        let csv = campaign_csv();
        for method in ["semi_explicit_phdae_dg", "cartesian_rattle"] {
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
                    "forced trajectory must converge at second order for {method}: {errors:?}"
                );
            }
        }
    }

    #[test]
    fn signed_work_auditor_rejects_wrong_sign_and_nan() {
        let good = campaign_csv();
        let rows: Vec<&str> = good.lines().collect();
        let wrong_sign = good
            .lines()
            .enumerate()
            .map(|(index, line)| {
                if index == 1 {
                    let mut columns: Vec<String> =
                        line.split(',').map(str::to_string).collect();
                    let work: f64 = columns[11].parse().unwrap();
                    columns[11] = format!("{:.17e}", -work);
                    columns.join(",")
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(audit_campaign(&wrong_sign).is_err());

        let poisoned = good
            .lines()
            .enumerate()
            .map(|(index, line)| {
                if index == 1 {
                    let mut columns: Vec<String> =
                        line.split(',').map(str::to_string).collect();
                    columns[11] = "NaN".to_string();
                    columns.join(",")
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(audit_campaign(&poisoned).is_err());
        let wrong_impulse = good
            .lines()
            .enumerate()
            .map(|(index, line)| {
                if index == 1 {
                    let mut columns: Vec<String> =
                        line.split(',').map(str::to_string).collect();
                    let impulse_x: f64 = columns[12].parse().unwrap();
                    columns[12] = format!("{:.17e}", -impulse_x);
                    columns.join(",")
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(audit_campaign(&wrong_impulse).is_err());

        let wrong_torque = good
            .lines()
            .enumerate()
            .map(|(index, line)| {
                if index == 1 {
                    let mut columns: Vec<String> =
                        line.split(',').map(str::to_string).collect();
                    let torque: f64 = columns[14].parse().unwrap();
                    columns[14] = format!("{:.17e}", -torque);
                    columns.join(",")
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(audit_campaign(&wrong_torque).is_err());

        let duplicate = format!("{}\n{}", good.trim_end(), rows[1]);
        assert!(audit_campaign(&duplicate).is_err());
        let truncated = rows[..rows.len() - 1].join("\n");
        assert!(audit_campaign(&truncated).is_err());
    }

    #[test]
    fn report_has_finite_force_work_and_momentum_residuals() {
        let csv = campaign_csv();
        for line in csv.lines().skip(1) {
            let fields: Vec<&str> = line.split(',').collect();
            let work: f64 = fields[11].parse().unwrap();
            let linear: f64 = fields[9].parse().unwrap();
            let angular: f64 = fields[10].parse().unwrap();
            assert!(work.is_finite());
            assert!(linear < 1.0e-8, "{line}");
            assert!(angular < 1.0e-8, "{line}");
        }
    }
}

#[cfg(not(test))]
fn main() {
    let csv = campaign_csv();
    audit_campaign(&csv).expect("forced dumbbell campaign must pass receipt audit");
    print!("{csv}");
}
