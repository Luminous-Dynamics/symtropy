// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Matched 3D pHDAE discrete-gradient and Cartesian RATTLE campaign under
// a common acceleration, with independent analytic work/impulse/torque
// values and an exact accelerated-COM/Rodrigues trajectory.

use std::fmt::Write as _;

#[allow(dead_code)]
#[path = "../phdae-dumbbell-3d-reference/constrained_dumbbell_3d.rs"]
mod phdae;

#[allow(dead_code)]
#[path = "../dumbbell-rattle-3d-reference/cartesian_rattle_3d.rs"]
mod rattle;

const MASS_1: f64 = 1.3;
const MASS_2: f64 = 2.1;
const TOTAL_MASS: f64 = MASS_1 + MASS_2;
const LENGTH: f64 = 1.2;
const HORIZON: f64 = 1.0;
const ACCELERATION: [f64; 3] = [0.18, -0.07, 0.11];
const TIME_STEPS: [f64; 4] = [0.04, 0.02, 0.01, 0.005];

fn initial_state() -> phdae::State {
    let center = [0.25, -0.12, 0.30];
    let center_velocity = [0.17, -0.23, 0.11];
    let relative_position = [0.8, 0.8, 0.4];
    let relative_velocity = [0.2, -0.1, -0.2];
    phdae::State {
        q1: phdae::sub(
            center,
            phdae::scale(relative_position, MASS_2 / TOTAL_MASS),
        ),
        q2: phdae::add(
            center,
            phdae::scale(relative_position, MASS_1 / TOTAL_MASS),
        ),
        v1: phdae::sub(
            center_velocity,
            phdae::scale(relative_velocity, MASS_2 / TOTAL_MASS),
        ),
        v2: phdae::add(
            center_velocity,
            phdae::scale(relative_velocity, MASS_1 / TOTAL_MASS),
        ),
    }
}

fn state_error(a: phdae::State, b: phdae::State) -> f64 {
    (phdae::norm(phdae::sub(a.q1, b.q1)).powi(2)
        + phdae::norm(phdae::sub(a.q2, b.q2)).powi(2)
        + phdae::norm(phdae::sub(a.v1, b.v1)).powi(2)
        + phdae::norm(phdae::sub(a.v2, b.v2)).powi(2))
    .sqrt()
}

fn exact_forced_state(time: f64) -> phdae::State {
    let initial = initial_state();
    let free =
        phdae::exact_free_dumbbell_3d(initial, MASS_1, MASS_2, LENGTH, time);
    let displacement = phdae::scale(ACCELERATION, 0.5 * time * time);
    let velocity_change = phdae::scale(ACCELERATION, time);
    phdae::State {
        q1: phdae::add(free.q1, displacement),
        q2: phdae::add(free.q2, displacement),
        v1: phdae::add(free.v1, velocity_change),
        v2: phdae::add(free.v2, velocity_change),
    }
}

fn expected_external_work() -> f64 {
    let initial = initial_state();
    let center_velocity = phdae::scale(
        phdae::add(
            phdae::scale(initial.v1, MASS_1),
            phdae::scale(initial.v2, MASS_2),
        ),
        1.0 / TOTAL_MASS,
    );
    TOTAL_MASS
        * phdae::dot(
            ACCELERATION,
            phdae::add(
                phdae::scale(center_velocity, HORIZON),
                phdae::scale(ACCELERATION, 0.5 * HORIZON * HORIZON),
            ),
        )
}

fn expected_linear_impulse() -> [f64; 3] {
    phdae::scale(ACCELERATION, TOTAL_MASS * HORIZON)
}

fn expected_angular_impulse() -> [f64; 3] {
    let initial = initial_state();
    let center = phdae::scale(
        phdae::add(
            phdae::scale(initial.q1, MASS_1),
            phdae::scale(initial.q2, MASS_2),
        ),
        1.0 / TOTAL_MASS,
    );
    let center_velocity = phdae::scale(
        phdae::add(
            phdae::scale(initial.v1, MASS_1),
            phdae::scale(initial.v2, MASS_2),
        ),
        1.0 / TOTAL_MASS,
    );
    phdae::scale(
        phdae::add(
            phdae::cross(center, ACCELERATION),
            phdae::scale(
                phdae::cross(center_velocity, ACCELERATION),
                0.5 * HORIZON,
            ),
        ),
        TOTAL_MASS * HORIZON,
    )
}

#[derive(Clone, Copy, Debug)]
struct Metrics {
    method: &'static str,
    dt: f64,
    steps: usize,
    final_state_error: f64,
    max_relative_energy_balance_residual: f64,
    max_relative_cumulative_work_defect: f64,
    max_relative_link_constraint: f64,
    max_relative_midpoint_constraint: f64,
    max_absolute_endpoint_tangent_residual: f64,
    max_relative_linear_momentum_residual: f64,
    max_relative_angular_momentum_residual: f64,
    cumulative_external_work: f64,
    linear_momentum_delta: [f64; 3],
    angular_momentum_delta: [f64; 3],
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
        .expect("frozen 3D pHDAE model must be valid");
    let mut state = initial;
    let energy0 = model.hamiltonian(state).expect("initial energy must be finite");
    let force1 = phdae::scale(ACCELERATION, MASS_1);
    let force2 = phdae::scale(ACCELERATION, MASS_2);
    let mut cumulative_work = 0.0;
    let mut max_energy_balance = 0.0_f64;
    let mut max_cumulative_work_defect = 0.0_f64;
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
            .expect("forced pHDAE step must converge");
        state = receipt.after;
        cumulative_work += receipt.external_work;
        let balance_scale = 1.0
            + receipt.energy_before.abs()
            + receipt.energy_after.abs()
            + receipt.external_work.abs()
            + receipt.constraint_reaction_work.abs();
        max_energy_balance = max_energy_balance
            .max(receipt.energy_balance_residual.abs() / balance_scale);
        let cumulative_defect =
            (receipt.energy_after - energy0 - cumulative_work).abs()
                / (1.0
                    + energy0.abs()
                    + receipt.energy_after.abs()
                    + cumulative_work.abs());
        max_cumulative_work_defect =
            max_cumulative_work_defect.max(cumulative_defect);
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

    let linear_delta =
        phdae::sub(model.linear_momentum(state), model.linear_momentum(initial));
    let angular_delta =
        phdae::sub(model.angular_momentum(state), model.angular_momentum(initial));

    Metrics {
        method: "semi_explicit_phdae_dg_3d",
        dt,
        steps,
        final_state_error: state_error(state, reference),
        max_relative_energy_balance_residual: max_energy_balance,
        max_relative_cumulative_work_defect: max_cumulative_work_defect,
        max_relative_link_constraint: max_link,
        max_relative_midpoint_constraint: max_midpoint,
        max_absolute_endpoint_tangent_residual: max_endpoint,
        max_relative_linear_momentum_residual: max_linear,
        max_relative_angular_momentum_residual: max_angular,
        cumulative_external_work: cumulative_work,
        linear_momentum_delta: linear_delta,
        angular_momentum_delta: angular_delta,
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
        .expect("frozen 3D RATTLE model must be valid");
    let mut state = rattle::State {
        q1: initial.q1,
        q2: initial.q2,
        v1: initial.v1,
        v2: initial.v2,
    };
    let initial_rattle = state;
    let energy0 = model.hamiltonian(state).expect("initial energy must be finite");
    let mut cumulative_work = 0.0;
    let mut max_energy_balance = 0.0_f64;
    let mut max_cumulative_work_defect = 0.0_f64;
    let mut max_link = 0.0_f64;
    let mut max_midpoint = 0.0_f64;
    let mut max_endpoint = 0.0_f64;
    let mut max_linear = 0.0_f64;
    let mut max_angular = 0.0_f64;

    for _ in 0..steps {
        let receipt = model
            .step_with_accelerations(state, ACCELERATION, ACCELERATION, dt)
            .expect("forced RATTLE step must be admissible");
        state = receipt.after;
        cumulative_work += receipt.external_work;
        let balance_scale = 1.0
            + receipt.energy_before.abs()
            + receipt.energy_after.abs()
            + receipt.external_work.abs();
        max_energy_balance = max_energy_balance
            .max(receipt.energy_balance_residual.abs() / balance_scale);
        let cumulative_defect =
            (receipt.energy_after - energy0 - cumulative_work).abs()
                / (1.0
                    + energy0.abs()
                    + receipt.energy_after.abs()
                    + cumulative_work.abs());
        max_cumulative_work_defect =
            max_cumulative_work_defect.max(cumulative_defect);
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
    let linear_delta = rattle::sub(
        model.linear_momentum(state),
        model.linear_momentum(initial_rattle),
    );
    let angular_delta = rattle::sub(
        model.angular_momentum(state),
        model.angular_momentum(initial_rattle),
    );

    Metrics {
        method: "cartesian_rattle_3d",
        dt,
        steps,
        final_state_error: state_error(final_state, reference),
        max_relative_energy_balance_residual: max_energy_balance,
        max_relative_cumulative_work_defect: max_cumulative_work_defect,
        max_relative_link_constraint: max_link,
        max_relative_midpoint_constraint: max_midpoint,
        max_absolute_endpoint_tangent_residual: max_endpoint,
        max_relative_linear_momentum_residual: max_linear,
        max_relative_angular_momentum_residual: max_angular,
        cumulative_external_work: cumulative_work,
        linear_momentum_delta: linear_delta,
        angular_momentum_delta: angular_delta,
        mean_multiplier_iterations: 0.0,
        max_multiplier_iterations: 0,
    }
}

fn campaign_csv() -> String {
    let initial = initial_state();
    let reference = exact_forced_state(HORIZON);
    let mut output = String::new();
    writeln!(
        &mut output,
        "method,dt,steps,final_state_error,max_relative_energy_balance_residual,max_relative_cumulative_work_defect,max_relative_link_constraint,max_relative_midpoint_constraint,max_absolute_endpoint_tangent_residual,max_relative_linear_momentum_residual,max_relative_angular_momentum_residual,cumulative_external_work,delta_p_x,delta_p_y,delta_p_z,delta_j_x,delta_j_y,delta_j_z,mean_multiplier_iterations,max_multiplier_iterations"
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
                "{},{:.8},{},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.6},{}",
                result.method,
                result.dt,
                result.steps,
                result.final_state_error,
                result.max_relative_energy_balance_residual,
                result.max_relative_cumulative_work_defect,
                result.max_relative_link_constraint,
                result.max_relative_midpoint_constraint,
                result.max_absolute_endpoint_tangent_residual,
                result.max_relative_linear_momentum_residual,
                result.max_relative_angular_momentum_residual,
                result.cumulative_external_work,
                result.linear_momentum_delta[0],
                result.linear_momentum_delta[1],
                result.linear_momentum_delta[2],
                result.angular_momentum_delta[0],
                result.angular_momentum_delta[1],
                result.angular_momentum_delta[2],
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
    const HEADER: &str = "method,dt,steps,final_state_error,max_relative_energy_balance_residual,max_relative_cumulative_work_defect,max_relative_link_constraint,max_relative_midpoint_constraint,max_absolute_endpoint_tangent_residual,max_relative_linear_momentum_residual,max_relative_angular_momentum_residual,cumulative_external_work,delta_p_x,delta_p_y,delta_p_z,delta_j_x,delta_j_y,delta_j_z,mean_multiplier_iterations,max_multiplier_iterations";

    let expected_work = expected_external_work();
    let expected_impulse = expected_linear_impulse();
    let expected_torque = expected_angular_impulse();
    let mut lines = csv.lines();
    if lines.next() != Some(HEADER) {
        return Err("unexpected or missing CSV header".to_string());
    }
    let mut seen = [[false; 4]; 2];
    let mut count = 0usize;

    for (row, line) in lines.enumerate() {
        let line_number = row + 2;
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 20 {
            return Err(format!("expected 20 columns at line {line_number}"));
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
        let actual_work: f64 = fields[11]
            .parse()
            .map_err(|_| format!("invalid external work at line {line_number}"))?;
        if !actual_work.is_finite()
            || (actual_work - expected_work).abs()
                > 1.0e-10 * (1.0 + expected_work.abs())
        {
            return Err(format!("signed external-work receipt mismatch at line {line_number}"));
        }

        for axis in 0..3 {
            let impulse: f64 = fields[12 + axis]
                .parse()
                .map_err(|_| format!("invalid linear impulse component at line {line_number}"))?;
            let torque: f64 = fields[15 + axis]
                .parse()
                .map_err(|_| format!("invalid angular impulse component at line {line_number}"))?;
            if !impulse.is_finite()
                || !torque.is_finite()
                || (impulse - expected_impulse[axis]).abs()
                    > 1.0e-10 * (1.0 + expected_impulse[axis].abs())
                || (torque - expected_torque[axis]).abs()
                    > 1.0e-10 * (1.0 + expected_torque[axis].abs())
            {
                return Err(format!("signed vector impulse/torque mismatch at line {line_number}, axis {axis}"));
            }
        }

        let mean_iterations: f64 = fields[18]
            .parse()
            .map_err(|_| format!("invalid mean iterations at line {line_number}"))?;
        let max_iterations: usize = fields[19]
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
            return Err(format!("RATTLE reports unexpected multiplier iterations at line {line_number}"));
        }

        if metrics[1] > 1.0e-8
            || metrics[2] > 1.0e-8
            || metrics[3] > 1.0e-10
            || metrics[4] > 1.0e-8
            || metrics[5] > 1.0e-8
            || metrics[6] > 1.0e-8
            || metrics[7] > 1.0e-8
        {
            return Err(format!("energy, constraint or momentum residual outside envelope at line {line_number}"));
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
    fn same_frozen_problem_horizon_and_step_ladder() {
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
    fn both_methods_converge_against_accelerated_rodrigues_solution() {
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
                    "forced 3D trajectory should be second order for {method}: {errors:?}"
                );
            }
        }
    }

    #[test]
    fn signed_work_impulse_and_torque_auditor_rejects_sign_flips_and_nan() {
        let good = campaign_csv();
        let rows: Vec<&str> = good.lines().collect();

        for column in 11..18 {
            let corrupted = good
                .lines()
                .enumerate()
                .map(|(index, line)| {
                    if index == 1 {
                        let mut fields: Vec<String> =
                            line.split(',').map(str::to_string).collect();
                        let value: f64 = fields[column].parse().unwrap();
                        fields[column] = format!("{:.17e}", -value);
                        fields.join(",")
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                audit_campaign(&corrupted).is_err(),
                "wrong sign accepted in column {column}"
            );
        }

        let poisoned = good
            .lines()
            .enumerate()
            .map(|(index, line)| {
                if index == 1 {
                    let mut fields: Vec<String> =
                        line.split(',').map(str::to_string).collect();
                    fields[15] = "NaN".to_string();
                    fields.join(",")
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(audit_campaign(&poisoned).is_err());

        let duplicate = format!("{}\n{}", good.trim_end(), rows[1]);
        assert!(audit_campaign(&duplicate).is_err());
        let truncated = rows[..rows.len() - 1].join("\n");
        assert!(audit_campaign(&truncated).is_err());
    }

    #[test]
    fn campaign_is_byte_identical_in_process() {
        assert_eq!(campaign_csv(), campaign_csv());
    }

    #[test]
    fn report_residuals_and_signed_ledger_are_finite() {
        let csv = campaign_csv();
        audit_campaign(&csv).unwrap();
        for row in csv.lines().skip(1) {
            let fields: Vec<&str> = row.split(',').collect();
            for field in &fields[3..18] {
                let value: f64 = field.parse().unwrap();
                assert!(value.is_finite(), "{row}");
            }
        }
    }
}

#[cfg(not(test))]
fn main() {
    let csv = campaign_csv();
    audit_campaign(&csv).expect("forced 3D dumbbell campaign must pass receipt audit");
    print!("{csv}");
}
