// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Matched coupled pHDAE and Cartesian SHAKE/RATTLE open-chain comparison.
// Both methods use the same masses, geometry, initial state, time horizon,
// timestep ladder and independent continuous constrained-RK4 reference.

use std::fmt::Write as _;

#[allow(dead_code)]
#[path = "../phdae-threebody-chain/three_body_chain.rs"]
mod phdae;

#[allow(dead_code)]
#[path = "../rattle-threebody-chain/cartesian_rattle_chain.rs"]
mod rattle;

const BODY_COUNT: usize = 3;
const EDGE_COUNT: usize = 2;
const MASSES: [f64; BODY_COUNT] = [1.3, 2.1, 0.8];
const HORIZON: f64 = 1.0;
const REFERENCE_DT: f64 = 1.0 / 40_000.0;
const REFERENCE_STEPS: usize = 40_000;
const TIME_STEPS: [f64; 4] = [0.02, 0.01, 0.005, 0.0025];
const EDGES: [(usize, usize); EDGE_COUNT] = [(0, 1), (1, 2)];

#[derive(Clone, Copy, Debug)]
struct Frame {
    q: [[f64; 3]; BODY_COUNT],
    v: [[f64; 3]; BODY_COUNT],
}

#[derive(Clone, Copy, Debug)]
struct Metrics {
    method: &'static str,
    dt: f64,
    steps: usize,
    final_state_error: f64,
    max_relative_energy_drift: f64,
    max_relative_step_energy_metric: f64,
    max_relative_link0: f64,
    max_relative_link1: f64,
    max_relative_midpoint0: f64,
    max_relative_midpoint1: f64,
    max_relative_endpoint0: f64,
    max_relative_endpoint1: f64,
    max_relative_linear_momentum: f64,
    max_relative_angular_momentum: f64,
    geometric_rank: usize,
    geometric_rcond: f64,
    method_condition_indicator: f64,
    mean_iterations: f64,
    max_iterations: usize,
    max_backtracks: usize,
}

fn initial_frame() -> Frame {
    Frame {
        q: [
            [-0.45, 0.10, 0.30],
            [0.25, 0.60, 0.60],
            [0.05, 1.40, 1.10],
        ],
        v: [
            [0.0723809523809524, -0.262857142857143, 0.313174603174603],
            [0.1723809523809524, -0.242857142857143, 0.0465079365079363],
            [0.3223809523809524, -0.142857142857143, -0.0534920634920637],
        ],
    }
}

fn lengths(frame: Frame) -> [f64; EDGE_COUNT] {
    [
        norm(sub(frame.q[1], frame.q[0])),
        norm(sub(frame.q[2], frame.q[1])),
    ]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: [f64; 3], factor: f64) -> [f64; 3] {
    [a[0] * factor, a[1] * factor, a[2] * factor]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn edge_position(frame: Frame, edge: usize) -> [f64; 3] {
    let (from, to) = EDGES[edge];
    sub(frame.q[to], frame.q[from])
}

fn edge_velocity(frame: Frame, edge: usize) -> [f64; 3] {
    let (from, to) = EDGES[edge];
    sub(frame.v[to], frame.v[from])
}

fn state_error(a: Frame, b: Frame) -> f64 {
    let mut sum = 0.0;
    for body in 0..BODY_COUNT {
        sum += dot(sub(a.q[body], b.q[body]), sub(a.q[body], b.q[body]));
        sum += dot(sub(a.v[body], b.v[body]), sub(a.v[body], b.v[body]));
    }
    sum.sqrt()
}

fn energy(frame: Frame) -> f64 {
    let mut result = 0.0;
    for body in 0..BODY_COUNT {
        result += 0.5 * MASSES[body] * dot(frame.v[body], frame.v[body]);
    }
    result
}

fn linear_momentum(frame: Frame) -> [f64; 3] {
    let mut result = [0.0; 3];
    for body in 0..BODY_COUNT {
        result = add(result, scale(frame.v[body], MASSES[body]));
    }
    result
}

fn angular_momentum(frame: Frame) -> [f64; 3] {
    let mut result = [0.0; 3];
    for body in 0..BODY_COUNT {
        result = add(
            result,
            scale(cross(frame.q[body], frame.v[body]), MASSES[body]),
        );
    }
    result
}

fn gradient(edge: usize, body: usize, relative_position: [f64; 3]) -> [f64; 3] {
    let (from, to) = EDGES[edge];
    if body == from {
        scale(relative_position, -1.0)
    } else if body == to {
        relative_position
    } else {
        [0.0; 3]
    }
}

fn solve_reference_2x2(matrix: [[f64; 2]; 2], rhs: [f64; 2]) -> [f64; 2] {
    let determinant =
        matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
    assert!(determinant.is_finite() && determinant.abs() > 1.0e-13);
    [
        (rhs[0] * matrix[1][1] - matrix[0][1] * rhs[1]) / determinant,
        (matrix[0][0] * rhs[1] - rhs[0] * matrix[1][0]) / determinant,
    ]
}

/// Independent acceleration-level constrained ODE:
/// qdot=v; M vdot = -G(q)^T lambda; G M^-1 G^T lambda = ||G v||^2.
/// This does not call either discrete solver.
fn continuous_rhs(frame: Frame) -> Frame {
    let mut gram = [[0.0; EDGE_COUNT]; EDGE_COUNT];
    let mut rhs = [0.0; EDGE_COUNT];
    for row in 0..EDGE_COUNT {
        let r_row = edge_position(frame, row);
        let w_row = edge_velocity(frame, row);
        rhs[row] = dot(w_row, w_row);
        for column in 0..EDGE_COUNT {
            let r_column = edge_position(frame, column);
            for body in 0..BODY_COUNT {
                gram[row][column] += dot(
                    gradient(row, body, r_row),
                    gradient(column, body, r_column),
                ) / MASSES[body];
            }
        }
    }
    let multipliers = solve_reference_2x2(gram, rhs);
    let mut acceleration = [[0.0; 3]; BODY_COUNT];
    for body in 0..BODY_COUNT {
        for edge in 0..EDGE_COUNT {
            acceleration[body] = sub(
                acceleration[body],
                scale(
                    gradient(edge, body, edge_position(frame, edge)),
                    multipliers[edge] / MASSES[body],
                ),
            );
        }
    }
    Frame {
        q: frame.v,
        v: acceleration,
    }
}

fn add_scaled(frame: Frame, derivative: Frame, factor: f64) -> Frame {
    let mut result = frame;
    for body in 0..BODY_COUNT {
        result.q[body] = add(frame.q[body], scale(derivative.q[body], factor));
        result.v[body] = add(frame.v[body], scale(derivative.v[body], factor));
    }
    result
}

fn rk4_reference(mut frame: Frame, dt: f64, steps: usize) -> Frame {
    for _ in 0..steps {
        let k1 = continuous_rhs(frame);
        let k2 = continuous_rhs(add_scaled(frame, k1, 0.5 * dt));
        let k3 = continuous_rhs(add_scaled(frame, k2, 0.5 * dt));
        let k4 = continuous_rhs(add_scaled(frame, k3, dt));
        for body in 0..BODY_COUNT {
            frame.q[body] = add(
                frame.q[body],
                scale(
                    add(
                        add(k1.q[body], scale(k2.q[body], 2.0)),
                        add(scale(k3.q[body], 2.0), k4.q[body]),
                    ),
                    dt / 6.0,
                ),
            );
            frame.v[body] = add(
                frame.v[body],
                scale(
                    add(
                        add(k1.v[body], scale(k2.v[body], 2.0)),
                        add(scale(k3.v[body], 2.0), k4.v[body]),
                    ),
                    dt / 6.0,
                ),
            );
        }
    }
    frame
}

fn to_phdae(frame: Frame) -> phdae::State {
    phdae::State {
        q: frame.q,
        v: frame.v,
    }
}

fn from_phdae(state: phdae::State) -> Frame {
    Frame {
        q: state.q,
        v: state.v,
    }
}

fn to_rattle(frame: Frame) -> rattle::State {
    rattle::State {
        q: frame.q,
        v: frame.v,
    }
}

fn from_rattle(state: rattle::State) -> Frame {
    Frame {
        q: state.q,
        v: state.v,
    }
}

fn geometric_rank(frame: Frame) -> (usize, f64) {
    use rattle::rank::{ConstraintGraph3, DistanceEdge, GraphBody, RankPolicy};
    let bodies = [
        GraphBody { id: 10, mass: MASSES[0], position: frame.q[0] },
        GraphBody { id: 20, mass: MASSES[1], position: frame.q[1] },
        GraphBody { id: 30, mass: MASSES[2], position: frame.q[2] },
    ];
    let edges = vec![
        DistanceEdge { id: 101, from_body_id: 10, to_body_id: 20, length: lengths(frame)[0] },
        DistanceEdge { id: 202, from_body_id: 20, to_body_id: 30, length: lengths(frame)[1] },
    ];
    let graph = ConstraintGraph3::new(1, bodies, edges, RankPolicy::default())
        .expect("shared chain geometry must be valid");
    let receipt = graph.assess_geometry().expect("constraint Gram must be finite");
    (receipt.numerical_rank, receipt.reciprocal_condition_indicator)
}

fn run_phdae(
    dt: f64,
    steps: usize,
    initial: Frame,
    reference: Frame,
    initial_geometry: (usize, f64),
) -> Metrics {
    let model = phdae::ThreeBodyChain::new(MASSES, lengths(initial))
        .expect("frozen pHDAE chain model must be valid");
    let mut state = to_phdae(initial);
    let energy0 = model.hamiltonian(state).expect("initial energy must be finite");
    let mut max_energy_drift = 0.0_f64;
    let mut max_energy_metric = 0.0_f64;
    let mut max_link = [0.0_f64; EDGE_COUNT];
    let mut max_midpoint = [0.0_f64; EDGE_COUNT];
    let mut max_endpoint = [0.0_f64; EDGE_COUNT];
    let mut max_linear = 0.0_f64;
    let mut max_angular = 0.0_f64;
    let mut min_condition = f64::INFINITY;
    let mut total_iterations = 0usize;
    let mut maximum_iterations = 0usize;

    for _ in 0..steps {
        let receipt = model
            .step(state, [[0.0; 3]; BODY_COUNT], dt, phdae::SolverOptions::default())
            .expect("pHDAE chain step must converge");
        state = receipt.after;
        max_energy_drift = max_energy_drift.max(
            (receipt.energy_after - energy0).abs()
                / energy0.abs().max(f64::MIN_POSITIVE),
        );
        max_energy_metric =
            max_energy_metric.max(receipt.relative_energy_balance_residual);
        for edge in 0..EDGE_COUNT {
            max_link[edge] =
                max_link[edge].max(receipt.relative_link_constraint_residuals[edge]);
            max_midpoint[edge] =
                max_midpoint[edge].max(receipt.relative_midpoint_constraint_residuals[edge]);
            max_endpoint[edge] =
                max_endpoint[edge].max(receipt.relative_endpoint_velocity_constraint_residuals[edge]);
        }
        max_linear = max_linear.max(receipt.relative_linear_momentum_residual);
        max_angular = max_angular.max(receipt.relative_angular_momentum_residual);
        min_condition = min_condition
            .min(receipt.normalized_multiplier_jacobian_determinant);
        total_iterations += receipt.multiplier_iterations;
        maximum_iterations = maximum_iterations.max(receipt.multiplier_iterations);
    }

    let final_state = from_phdae(state);
    Metrics {
        method: "coupled_phdae_discrete_gradient",
        dt,
        steps,
        final_state_error: state_error(final_state, reference),
        max_relative_energy_drift: max_energy_drift,
        max_relative_step_energy_metric: max_energy_metric,
        max_relative_link0: max_link[0],
        max_relative_link1: max_link[1],
        max_relative_midpoint0: max_midpoint[0],
        max_relative_midpoint1: max_midpoint[1],
        max_relative_endpoint0: max_endpoint[0],
        max_relative_endpoint1: max_endpoint[1],
        max_relative_linear_momentum: max_linear,
        max_relative_angular_momentum: max_angular,
        geometric_rank: initial_geometry.0,
        geometric_rcond: initial_geometry.1,
        method_condition_indicator: min_condition,
        mean_iterations: total_iterations as f64 / steps as f64,
        max_iterations: maximum_iterations,
        max_backtracks: 0,
    }
}

fn run_rattle(
    dt: f64,
    steps: usize,
    initial: Frame,
    reference: Frame,
    initial_geometry: (usize, f64),
) -> Metrics {
    let model = rattle::CartesianRattleChain::new(MASSES, lengths(initial))
        .expect("frozen RATTLE chain model must be valid");
    let mut state = to_rattle(initial);
    let energy0 = model.hamiltonian(state).expect("initial energy must be finite");
    let mut max_energy_drift = 0.0_f64;
    let mut max_energy_metric = 0.0_f64;
    let mut max_link = [0.0_f64; EDGE_COUNT];
    let mut max_midpoint = [0.0_f64; EDGE_COUNT];
    let mut max_endpoint = [0.0_f64; EDGE_COUNT];
    let mut max_linear = 0.0_f64;
    let mut max_angular = 0.0_f64;
    let mut min_condition = f64::INFINITY;
    let mut total_iterations = 0usize;
    let mut maximum_iterations = 0usize;
    let mut maximum_backtracks = 0usize;

    for _ in 0..steps {
        let receipt = model
            .step(state, dt, rattle::SolverOptions::default())
            .expect("RATTLE chain step must be admissible");
        state = receipt.after;
        max_energy_drift = max_energy_drift.max(
            (receipt.energy_after - energy0).abs()
                / energy0.abs().max(f64::MIN_POSITIVE),
        );
        max_energy_metric =
            max_energy_metric.max(receipt.relative_energy_defect);
        for edge in 0..EDGE_COUNT {
            max_link[edge] =
                max_link[edge].max(receipt.relative_link_constraint_residuals[edge]);
            max_midpoint[edge] =
                max_midpoint[edge].max(receipt.relative_midpoint_constraint_residuals[edge]);
            max_endpoint[edge] =
                max_endpoint[edge].max(receipt.relative_endpoint_velocity_constraint_residuals[edge]);
        }
        max_linear = max_linear.max(receipt.relative_linear_momentum_residual);
        max_angular = max_angular.max(receipt.relative_angular_momentum_residual);
        min_condition = min_condition.min(receipt.position_jacobian_rcond);
        total_iterations += receipt.newton_iterations;
        maximum_iterations = maximum_iterations.max(receipt.newton_iterations);
        maximum_backtracks = maximum_backtracks.max(receipt.backtracks);
    }

    let final_state = from_rattle(state);
    Metrics {
        method: "cartesian_shake_rattle",
        dt,
        steps,
        final_state_error: state_error(final_state, reference),
        max_relative_energy_drift: max_energy_drift,
        max_relative_step_energy_metric: max_energy_metric,
        max_relative_link0: max_link[0],
        max_relative_link1: max_link[1],
        max_relative_midpoint0: max_midpoint[0],
        max_relative_midpoint1: max_midpoint[1],
        max_relative_endpoint0: max_endpoint[0],
        max_relative_endpoint1: max_endpoint[1],
        max_relative_linear_momentum: max_linear,
        max_relative_angular_momentum: max_angular,
        geometric_rank: initial_geometry.0,
        geometric_rcond: initial_geometry.1,
        method_condition_indicator: min_condition,
        mean_iterations: total_iterations as f64 / steps as f64,
        max_iterations: maximum_iterations,
        max_backtracks: maximum_backtracks,
    }
}

fn campaign_csv() -> String {
    let initial = initial_frame();
    let reference = rk4_reference(initial, REFERENCE_DT, REFERENCE_STEPS);
    let geometry = geometric_rank(initial);
    let mut output = String::new();
    writeln!(
        &mut output,
        "method,dt,steps,final_state_error,max_relative_energy_drift,max_relative_step_energy_metric,max_relative_link0,max_relative_link1,max_relative_midpoint0,max_relative_midpoint1,max_relative_endpoint0,max_relative_endpoint1,max_relative_linear_momentum,max_relative_angular_momentum,geometric_rank,geometric_rcond,method_condition_indicator,mean_iterations,max_iterations,max_backtracks"
    )
    .expect("writing to String cannot fail");

    for dt in TIME_STEPS {
        let steps = (HORIZON / dt).round() as usize;
        for result in [
            run_phdae(dt, steps, initial, reference, geometry),
            run_rattle(dt, steps, initial, reference, geometry),
        ] {
            writeln!(
                &mut output,
                "{},{:.8},{},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{},{:.17e},{:.17e},{:.6},{},{}",
                result.method,
                result.dt,
                result.steps,
                result.final_state_error,
                result.max_relative_energy_drift,
                result.max_relative_step_energy_metric,
                result.max_relative_link0,
                result.max_relative_link1,
                result.max_relative_midpoint0,
                result.max_relative_midpoint1,
                result.max_relative_endpoint0,
                result.max_relative_endpoint1,
                result.max_relative_linear_momentum,
                result.max_relative_angular_momentum,
                result.geometric_rank,
                result.geometric_rcond,
                result.method_condition_indicator,
                result.mean_iterations,
                result.max_iterations,
                result.max_backtracks
            )
            .expect("writing to String cannot fail");
        }
    }
    output
}

fn audit_campaign(csv: &str) -> Result<(), String> {
    const METHODS: [&str; 2] = [
        "coupled_phdae_discrete_gradient",
        "cartesian_shake_rattle",
    ];
    const HEADER: &str = "method,dt,steps,final_state_error,max_relative_energy_drift,max_relative_step_energy_metric,max_relative_link0,max_relative_link1,max_relative_midpoint0,max_relative_midpoint1,max_relative_endpoint0,max_relative_endpoint1,max_relative_linear_momentum,max_relative_angular_momentum,geometric_rank,geometric_rcond,method_condition_indicator,mean_iterations,max_iterations,max_backtracks";
    let mut seen = [[false; 4]; 2];
    let mut lines = csv.lines();
    if lines.next() != Some(HEADER) {
        return Err("unexpected or missing CSV header".to_string());
    }
    let mut count = 0usize;
    for (row, line) in lines.enumerate() {
        let line_number = row + 2;
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 20 {
            return Err(format!("expected 20 columns at line {line_number}"));
        }
        let method = METHODS
            .iter()
            .position(|value| *value == fields[0])
            .ok_or_else(|| format!("unknown solver at line {line_number}"))?;
        let dt: f64 = fields[1]
            .parse()
            .map_err(|_| format!("invalid timestep at line {line_number}"))?;
        let dt_index = TIME_STEPS
            .iter()
            .position(|expected| (dt - expected).abs() < 1.0e-10)
            .ok_or_else(|| format!("unexpected timestep at line {line_number}"))?;
        if seen[method][dt_index] {
            return Err(format!("duplicate method/timestep at line {line_number}"));
        }
        let steps: usize = fields[2]
            .parse()
            .map_err(|_| format!("invalid step count at line {line_number}"))?;
        if steps != (HORIZON / dt).round() as usize {
            return Err(format!("wrong horizon/step count at line {line_number}"));
        }

        let mut values = [0.0_f64; 11];
        for index in 0..11 {
            let value: f64 = fields[3 + index]
                .parse()
                .map_err(|_| format!("invalid metric at line {line_number}"))?;
            if !value.is_finite() || value < 0.0 {
                return Err(format!("non-finite or negative metric at line {line_number}"));
            }
            values[index] = value;
        }
        let geometric_rank: usize = fields[14]
            .parse()
            .map_err(|_| format!("invalid geometric rank at line {line_number}"))?;
        let geometric_rcond: f64 = fields[15]
            .parse()
            .map_err(|_| format!("invalid geometric rcond at line {line_number}"))?;
        let method_condition: f64 = fields[16]
            .parse()
            .map_err(|_| format!("invalid method condition at line {line_number}"))?;
        let mean_iterations: f64 = fields[17]
            .parse()
            .map_err(|_| format!("invalid mean iteration count at line {line_number}"))?;
        let max_iterations: usize = fields[18]
            .parse()
            .map_err(|_| format!("invalid max iteration count at line {line_number}"))?;
        let max_backtracks: usize = fields[19]
            .parse()
            .map_err(|_| format!("invalid backtrack count at line {line_number}"))?;

        if geometric_rank != 2
            || !geometric_rcond.is_finite()
            || geometric_rcond <= 1.0e-12
            || !method_condition.is_finite()
            || method_condition <= 1.0e-12
            || !mean_iterations.is_finite()
            || mean_iterations < 0.0
            || mean_iterations > max_iterations as f64
            || max_iterations > 40
            || max_backtracks > 28
        {
            return Err(format!("rank/condition/iteration audit failed at line {line_number}"));
        }

        if values[2] > 1.0e-9
            || values[3] > 1.0e-9
            || values[4] > 1.0e-7
            || values[5] > 1.0e-7
            || values[6] > 1.0e-7
            || values[7] > 1.0e-7
            || values[8] > 1.0e-8
            || values[9] > 1.0e-8
            || values[10] > 1.0e-4
        {
            return Err(format!("constraint, momentum or energy envelope failed at line {line_number}"));
        }

        seen[method][dt_index] = true;
        count += 1;
    }
    if count != 8 || seen.iter().any(|row| row.iter().any(|value| !value)) {
        return Err("campaign must contain exactly one row per method/timestep".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_initial_state_masses_edges_horizon_and_dt_ladder() {
        let csv = campaign_csv();
        audit_campaign(&csv).unwrap();
        assert_eq!(csv.lines().count(), 9);
        for method in [
            "coupled_phdae_discrete_gradient",
            "cartesian_shake_rattle",
        ] {
            for dt in TIME_STEPS {
                let prefix = format!("{method},{dt:.8},");
                assert!(csv.lines().any(|line| line.starts_with(&prefix)));
            }
        }
    }

    #[test]
    fn both_methods_converge_at_second_order_against_common_reference() {
        let csv = campaign_csv();
        for method in [
            "coupled_phdae_discrete_gradient",
            "cartesian_shake_rattle",
        ] {
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
    fn report_auditor_rejects_nan_duplicate_and_missing_rows() {
        let good = campaign_csv();
        let poisoned = good
            .lines()
            .enumerate()
            .map(|(index, line)| {
                if index == 1 {
                    let mut fields: Vec<String> =
                        line.split(',').map(str::to_string).collect();
                    fields[3] = "NaN".to_string();
                    fields.join(",")
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

    #[test]
    fn campaign_replay_is_byte_identical() {
        assert_eq!(campaign_csv(), campaign_csv());
    }
}

#[cfg(not(test))]
fn main() {
    let csv = campaign_csv();
    audit_campaign(&csv).expect("matched pHDAE/RATTLE chain campaign must pass");
    print!("{csv}");
}
