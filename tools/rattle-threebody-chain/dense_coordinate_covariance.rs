// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Algebraic coordinate-covariance preflight for the frozen three-body chain.
// This module does not implement a dense-coordinate time-step solver.

const N: usize = 9;
type Mat9 = [[f64; N]; N];
type IntMat9 = [[i64; N]; N];
type Vec9 = [f64; N];
type Jacobian = [[f64; N]; 2];

const U: [i64; N] = [1, 1, 1, 2, 1, 1, 1, 2, 3];
const V: [i64; N] = [1, 1, 1, 1, -1, -1, -1, -1, 0];

fn basis_pair(scale: i64) -> (IntMat9, IntMat9) {
    let mut basis = [[0_i64; N]; N];
    let mut inverse = [[0_i64; N]; N];
    for row in 0..N {
        for column in 0..N {
            let identity = if row == column { 1_i64 } else { 0_i64 };
            let rank_one = scale * U[row] * V[column];
            basis[row][column] = identity + rank_one;
            inverse[row][column] = identity - rank_one;
        }
    }
    (basis, inverse)
}

fn to_float(matrix: &IntMat9) -> Mat9 {
    let mut output = [[0.0; N]; N];
    for row in 0..N {
        for column in 0..N {
            output[row][column] = matrix[row][column] as f64;
        }
    }
    output
}

fn transpose(matrix: &Mat9) -> Mat9 {
    let mut output = [[0.0; N]; N];
    for row in 0..N {
        for column in 0..N {
            output[row][column] = matrix[column][row];
        }
    }
    output
}

fn multiply(left: &Mat9, right: &Mat9) -> Mat9 {
    let mut output = [[0.0; N]; N];
    for row in 0..N {
        for column in 0..N {
            for inner in 0..N {
                output[row][column] += left[row][inner] * right[inner][column];
            }
        }
    }
    output
}

fn mat_vec(matrix: &Mat9, vector: &Vec9) -> Vec9 {
    let mut output = [0.0; N];
    for row in 0..N {
        for column in 0..N {
            output[row] += matrix[row][column] * vector[column];
        }
    }
    output
}

fn dot(left: &Vec9, right: &Vec9) -> f64 {
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

fn max_abs(matrix: &Mat9) -> f64 {
    matrix
        .iter()
        .flat_map(|row| row.iter())
        .map(|value| value.abs())
        .fold(0.0, f64::max)
}

fn vector_max_abs(left: &Vec9, right: &Vec9) -> f64 {
    left.iter()
        .zip(right.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max)
}

fn mass_metric_with_masses(masses: [f64; 3]) -> Mat9 {
    let mut metric = [[0.0; N]; N];
    for body in 0..3 {
        for axis in 0..3 {
            let index = 3 * body + axis;
            metric[index][index] = masses[body];
        }
    }
    metric
}

fn mass_metric() -> Mat9 {
    mass_metric_with_masses([1.3, 2.1, 0.8])
}

fn initial_position() -> Vec9 {
    [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.5]
}

fn initial_velocity() -> Vec9 {
    [0.1, 0.2, -0.1, 0.1, 0.5, 0.0, 0.3, 0.6, -0.2]
}

fn force_covector() -> Vec9 {
    [0.2, -0.1, 0.05, -0.15, 0.08, 0.12, 0.05, 0.03, -0.09]
}

fn constraint_jacobian() -> Jacobian {
    [
        [-1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.0, -1.0, -0.5, 0.0, 1.0, 0.5],
    ]
}

fn jacobian_transform(jacobian: &Jacobian, basis: &Mat9) -> Jacobian {
    let mut transformed = [[0.0; N]; 2];
    for row in 0..2 {
        for column in 0..N {
            for inner in 0..N {
                transformed[row][column] += jacobian[row][inner] * basis[inner][column];
            }
        }
    }
    transformed
}

fn jacobian_vec(jacobian: &Jacobian, vector: &Vec9) -> [f64; 2] {
    let mut output = [0.0; 2];
    for row in 0..2 {
        for column in 0..N {
            output[row] += jacobian[row][column] * vector[column];
        }
    }
    output
}

fn jacobian_reaction(jacobian: &Jacobian, lambda: [f64; 2]) -> Vec9 {
    let mut output = [0.0; N];
    for row in 0..2 {
        for column in 0..N {
            output[column] += jacobian[row][column] * lambda[row];
        }
    }
    output
}

fn generator(axis: usize) -> Mat9 {
    let blocks = [
        [[0.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]],
        [[0.0, 0.0, 1.0], [0.0, 0.0, 0.0], [-1.0, 0.0, 0.0]],
        [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
    ];
    let block = blocks[axis];
    let mut matrix = [[0.0; N]; N];
    for body in 0..3 {
        for row in 0..3 {
            for column in 0..3 {
                matrix[3 * body + row][3 * body + column] = block[row][column];
            }
        }
    }
    matrix
}

fn metric_skew_defect(generator: &Mat9, metric: &Mat9) -> f64 {
    let left = multiply(&transpose(generator), metric);
    let right = multiply(metric, generator);
    let mut defect = [[0.0; N]; N];
    for row in 0..N {
        for column in 0..N {
            defect[row][column] = left[row][column] + right[row][column];
        }
    }
    max_abs(&defect)
}

fn cholesky_positive_definite(matrix: &Mat9) -> bool {
    let mut lower = [[0.0; N]; N];
    for row in 0..N {
        for column in 0..=row {
            let mut value = matrix[row][column];
            for inner in 0..column {
                value -= lower[row][inner] * lower[column][inner];
            }
            if row == column {
                if !value.is_finite() || value <= 0.0 {
                    return false;
                }
                lower[row][column] = value.sqrt();
            } else {
                lower[row][column] = value / lower[column][column];
            }
        }
    }
    true
}

fn assert_close(actual: &Vec9, expected: &Vec9, tolerance: f64) {
    assert!(vector_max_abs(actual, expected) <= tolerance);
}

type RelativeMaps = [[[f64; N]; 3]; 2];

fn relative_maps(basis: &Mat9) -> RelativeMaps {
    let mut maps = [[[0.0; N]; 3]; 2];
    let edges = [(0_usize, 1_usize), (1_usize, 2_usize)];
    for (edge, (from, to)) in edges.into_iter().enumerate() {
        for axis in 0..3 {
            for column in 0..N {
                maps[edge][axis][column] =
                    basis[3 * to + axis][column] - basis[3 * from + axis][column];
            }
        }
    }
    maps
}

fn apply_relative_map(map: &[[f64; N]; 3], y: &Vec9) -> [f64; 3] {
    let mut relative = [0.0; 3];
    for axis in 0..3 {
        for coordinate in 0..N {
            relative[axis] += map[axis][coordinate] * y[coordinate];
        }
    }
    relative
}

fn dense_constraint_jacobian(maps: &RelativeMaps, y: &Vec9) -> Jacobian {
    let mut jacobian = [[0.0; N]; 2];
    for edge in 0..2 {
        let relative = apply_relative_map(&maps[edge], y);
        for coordinate in 0..N {
            for axis in 0..3 {
                jacobian[edge][coordinate] += relative[axis] * maps[edge][axis][coordinate];
            }
        }
    }
    jacobian
}

fn solve_dense_metric(metric: &Mat9, rhs: &Vec9) -> Vec9 {
    let mut augmented = [[0.0; N + 1]; N];
    for row in 0..N {
        for column in 0..N {
            augmented[row][column] = metric[row][column];
        }
        augmented[row][N] = rhs[row];
    }

    for pivot in 0..N {
        let mut selected = pivot;
        for row in (pivot + 1)..N {
            if augmented[row][pivot].abs() > augmented[selected][pivot].abs() {
                selected = row;
            }
        }
        assert!(
            augmented[selected][pivot].abs() > 1.0e-14,
            "transformed mass metric lost numerical rank"
        );
        augmented.swap(pivot, selected);

        let diagonal = augmented[pivot][pivot];
        for column in pivot..=N {
            augmented[pivot][column] /= diagonal;
        }
        for row in 0..N {
            if row == pivot {
                continue;
            }
            let factor = augmented[row][pivot];
            for column in pivot..=N {
                augmented[row][column] -= factor * augmented[pivot][column];
            }
        }
    }

    let mut solution = [0.0; N];
    for row in 0..N {
        solution[row] = augmented[row][N];
    }
    solution
}

fn solve_2x2_dense(matrix: [[f64; 2]; 2], rhs: [f64; 2]) -> [f64; 2] {
    let determinant = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
    let scale = matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
    assert!(determinant.is_finite() && determinant.abs() > 1.0e-14 * scale * scale);
    [
        (rhs[0] * matrix[1][1] - matrix[0][1] * rhs[1]) / determinant,
        (matrix[0][0] * rhs[1] - rhs[0] * matrix[1][0]) / determinant,
    ]
}

fn dense_position_residuals(maps: &RelativeMaps, y: &Vec9, lengths: [f64; 2]) -> [f64; 2] {
    let mut residuals = [0.0; 2];
    for edge in 0..2 {
        let relative = apply_relative_map(&maps[edge], y);
        residuals[edge] = 0.5
            * (relative.iter().map(|value| value * value).sum::<f64>()
                - lengths[edge] * lengths[edge]);
    }
    residuals
}

/// A genuine dense-coordinate SHAKE/RATTLE step. The generalized velocity,
/// mass metric, constraint Jacobians, KKT solves and momentum projection all
/// remain in y-coordinates; S is used only to define the represented
/// constraint maps and to lift the final result for comparison.
fn dense_coordinate_rattle_step(
    basis: &Mat9,
    inverse: &Mat9,
    masses: [f64; 3],
    lengths: [f64; 2],
    q0: &Vec9,
    vq0: &Vec9,
    dt: f64,
) -> (Vec9, Vec9, [f64; 2], [f64; 2], [f64; 2]) {
    let metric_q = mass_metric_with_masses(masses);
    let metric_y = multiply(&transpose(basis), &multiply(&metric_q, basis));
    let maps = relative_maps(basis);
    let y0 = mat_vec(inverse, q0);
    let vy0 = mat_vec(inverse, vq0);
    let j0 = dense_constraint_jacobian(&maps, &y0);

    // Compute M_y^-1 J_y(y_0)^T by solving the dense SPD metric directly.
    let mut minv_j0_t = [[0.0; N]; 2];
    for edge in 0..2 {
        let mut covector = [0.0; N];
        covector.copy_from_slice(&j0[edge]);
        minv_j0_t[edge] = solve_dense_metric(&metric_y, &covector);
    }

    let mut y_free = y0;
    for coordinate in 0..N {
        y_free[coordinate] += dt * vy0[coordinate];
    }
    let mut multipliers = [0.0; 2];
    let mut y_plus = y_free;
    let mut residuals = dense_position_residuals(&maps, &y_plus, lengths);

    for _ in 0..32 {
        let scaled_norm = (0..2)
            .map(|edge| {
                let value = residuals[edge] / (1.0 + lengths[edge] * lengths[edge]);
                value * value
            })
            .sum::<f64>()
            .sqrt();
        if scaled_norm <= 2.0e-13 {
            break;
        }

        let j_plus = dense_constraint_jacobian(&maps, &y_plus);
        let mut newton = [[0.0; 2]; 2];
        for row in 0..2 {
            for column in 0..2 {
                let contraction = (0..N)
                    .map(|coordinate| j_plus[row][coordinate] * minv_j0_t[column][coordinate])
                    .sum::<f64>();
                newton[row][column] = -0.5 * dt * dt * contraction;
            }
        }
        let delta = solve_2x2_dense(newton, [-residuals[0], -residuals[1]]);
        let mut fraction = 1.0;
        let mut accepted = false;
        for _ in 0..20 {
            let candidate_multipliers = [
                multipliers[0] + fraction * delta[0],
                multipliers[1] + fraction * delta[1],
            ];
            let mut candidate = y_free;
            for coordinate in 0..N {
                candidate[coordinate] -= 0.5
                    * dt
                    * dt
                    * (minv_j0_t[0][coordinate] * candidate_multipliers[0]
                        + minv_j0_t[1][coordinate] * candidate_multipliers[1]);
            }
            let candidate_residuals = dense_position_residuals(&maps, &candidate, lengths);
            let candidate_norm = (0..2)
                .map(|edge| {
                    let value = candidate_residuals[edge] / (1.0 + lengths[edge] * lengths[edge]);
                    value * value
                })
                .sum::<f64>()
                .sqrt();
            if candidate_norm < scaled_norm {
                multipliers = candidate_multipliers;
                y_plus = candidate;
                residuals = candidate_residuals;
                accepted = true;
                break;
            }
            fraction *= 0.5;
        }
        assert!(
            accepted,
            "dense-coordinate SHAKE Newton line search stalled"
        );
    }
    let final_position_norm = (0..2)
        .map(|edge| {
            let value = residuals[edge] / (1.0 + lengths[edge] * lengths[edge]);
            value * value
        })
        .sum::<f64>()
        .sqrt();
    assert!(
        final_position_norm <= 2.0e-13,
        "dense-coordinate SHAKE did not converge"
    );

    // RATTLE endpoint momentum projection, entirely in the dense y metric.
    let mut momentum_half = mat_vec(&metric_y, &vy0);
    for coordinate in 0..N {
        momentum_half[coordinate] -=
            0.5 * dt * (j0[0][coordinate] * multipliers[0] + j0[1][coordinate] * multipliers[1]);
    }
    let j_plus = dense_constraint_jacobian(&maps, &y_plus);
    let mut minv_jplus_t = [[0.0; N]; 2];
    for edge in 0..2 {
        let mut covector = [0.0; N];
        covector.copy_from_slice(&j_plus[edge]);
        minv_jplus_t[edge] = solve_dense_metric(&metric_y, &covector);
    }
    let mut velocity_gram = [[0.0; 2]; 2];
    for row in 0..2 {
        for column in 0..2 {
            velocity_gram[row][column] = (0..N)
                .map(|coordinate| j_plus[row][coordinate] * minv_jplus_t[column][coordinate])
                .sum();
        }
    }
    let minv_momentum_half = solve_dense_metric(&metric_y, &momentum_half);
    let velocity_rhs = [
        2.0 / dt
            * (0..N)
                .map(|coordinate| j_plus[0][coordinate] * minv_momentum_half[coordinate])
                .sum::<f64>(),
        2.0 / dt
            * (0..N)
                .map(|coordinate| j_plus[1][coordinate] * minv_momentum_half[coordinate])
                .sum::<f64>(),
    ];
    let velocity_multipliers = solve_2x2_dense(velocity_gram, velocity_rhs);
    let mut momentum_plus = momentum_half;
    for coordinate in 0..N {
        momentum_plus[coordinate] -= 0.5
            * dt
            * (j_plus[0][coordinate] * velocity_multipliers[0]
                + j_plus[1][coordinate] * velocity_multipliers[1]);
    }
    let vy_plus = solve_dense_metric(&metric_y, &momentum_plus);

    let q_plus = mat_vec(basis, &y_plus);
    let vq_plus = mat_vec(basis, &vy_plus);
    let lifted_residuals = dense_position_residuals(&maps, &y_plus, lengths);
    (
        q_plus,
        vq_plus,
        multipliers,
        velocity_multipliers,
        lifted_residuals,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dense_generalized_coordinate_rattle_step_matches_independent_cartesian_step() {
        let q0 = initial_position();
        let vq0 = initial_velocity();
        let masses = [1.3, 2.1, 0.8];
        let lengths = [1.0_f64, 1.25_f64.sqrt()];
        let state = super::super::State {
            q: [
                [q0[0], q0[1], q0[2]],
                [q0[3], q0[4], q0[5]],
                [q0[6], q0[7], q0[8]],
            ],
            v: [
                [vq0[0], vq0[1], vq0[2]],
                [vq0[3], vq0[4], vq0[5]],
                [vq0[6], vq0[7], vq0[8]],
            ],
        };
        let cartesian = super::super::CartesianRattleChain::new(masses, lengths).unwrap();
        let dt = 0.01;
        let reference = cartesian
            .step(state, dt, super::super::SolverOptions::default())
            .unwrap();

        for scale in [1_i64, 2_i64] {
            let (basis_i, inverse_i) = basis_pair(scale);
            let basis = to_float(&basis_i);
            let inverse = to_float(&inverse_i);
            let (q_plus, v_plus, position_lambda, velocity_lambda, residuals) =
                dense_coordinate_rattle_step(&basis, &inverse, masses, lengths, &q0, &vq0, dt);
            let reference_q = [
                reference.after.q[0][0],
                reference.after.q[0][1],
                reference.after.q[0][2],
                reference.after.q[1][0],
                reference.after.q[1][1],
                reference.after.q[1][2],
                reference.after.q[2][0],
                reference.after.q[2][1],
                reference.after.q[2][2],
            ];
            let reference_v = [
                reference.after.v[0][0],
                reference.after.v[0][1],
                reference.after.v[0][2],
                reference.after.v[1][0],
                reference.after.v[1][1],
                reference.after.v[1][2],
                reference.after.v[2][0],
                reference.after.v[2][1],
                reference.after.v[2][2],
            ];
            assert!(vector_max_abs(&q_plus, &reference_q) <= 2.0e-8);
            assert!(vector_max_abs(&v_plus, &reference_v) <= 2.0e-8);
            assert!((position_lambda[0] - reference.position_multipliers[0]).abs() <= 2.0e-8);
            assert!((position_lambda[1] - reference.position_multipliers[1]).abs() <= 2.0e-8);
            assert!((velocity_lambda[0] - reference.velocity_multipliers[0]).abs() <= 2.0e-8);
            assert!((velocity_lambda[1] - reference.velocity_multipliers[1]).abs() <= 2.0e-8);
            assert!(residuals.iter().all(|residual| residual.abs() <= 2.0e-12));

            let endpoint_jacobian = constraint_jacobian_at(&q_plus);
            let endpoint_tangency = jacobian_vec(&endpoint_jacobian, &v_plus);
            assert!(endpoint_tangency.iter().all(|value| value.abs() <= 2.0e-9));
        }
    }

    fn flatten_state(state: &super::super::State) -> (Vec9, Vec9) {
        (
            [
                state.q[0][0],
                state.q[0][1],
                state.q[0][2],
                state.q[1][0],
                state.q[1][1],
                state.q[1][2],
                state.q[2][0],
                state.q[2][1],
                state.q[2][2],
            ],
            [
                state.v[0][0],
                state.v[0][1],
                state.v[0][2],
                state.v[1][0],
                state.v[1][1],
                state.v[1][2],
                state.v[2][0],
                state.v[2][1],
                state.v[2][2],
            ],
        )
    }

    fn state_from_flat(q: &Vec9, v: &Vec9) -> super::super::State {
        super::super::State {
            q: [[q[0], q[1], q[2]], [q[3], q[4], q[5]], [q[6], q[7], q[8]]],
            v: [[v[0], v[1], v[2]], [v[3], v[4], v[5]], [v[6], v[7], v[8]]],
        }
    }

    #[test]
    fn dense_coordinate_rattle_covariance_survives_a_32_step_trajectory() {
        let masses = [1.3, 2.1, 0.8];
        let lengths = [1.0_f64, 1.25_f64.sqrt()];
        let dt = 0.01;
        let cartesian = super::super::CartesianRattleChain::new(masses, lengths).unwrap();
        let options = super::super::SolverOptions::default();

        for scale in [1_i64, 2_i64] {
            let (basis_i, inverse_i) = basis_pair(scale);
            let basis = to_float(&basis_i);
            let inverse = to_float(&inverse_i);
            let mut dense_q = initial_position();
            let mut dense_v = initial_velocity();
            let mut cartesian_state = state_from_flat(&dense_q, &dense_v);
            let mut maximum_position_error = 0.0_f64;
            let mut maximum_velocity_error = 0.0_f64;
            let mut maximum_position_multiplier_error = 0.0_f64;
            let mut maximum_velocity_multiplier_error = 0.0_f64;
            let mut maximum_endpoint_tangency = 0.0_f64;
            let mut maximum_constraint_residual = 0.0_f64;

            for _step in 0..32 {
                let reference = cartesian.step(cartesian_state, dt, options).unwrap();
                let (
                    dense_q_next,
                    dense_v_next,
                    dense_position_lambda,
                    dense_velocity_lambda,
                    residuals,
                ) = dense_coordinate_rattle_step(
                    &basis, &inverse, masses, lengths, &dense_q, &dense_v, dt,
                );
                let (reference_q, reference_v) = flatten_state(&reference.after);

                maximum_position_error =
                    maximum_position_error.max(vector_max_abs(&dense_q_next, &reference_q));
                maximum_velocity_error =
                    maximum_velocity_error.max(vector_max_abs(&dense_v_next, &reference_v));
                for edge in 0..2 {
                    maximum_position_multiplier_error = maximum_position_multiplier_error.max(
                        (dense_position_lambda[edge] - reference.position_multipliers[edge]).abs(),
                    );
                    maximum_velocity_multiplier_error = maximum_velocity_multiplier_error.max(
                        (dense_velocity_lambda[edge] - reference.velocity_multipliers[edge]).abs(),
                    );
                    maximum_constraint_residual =
                        maximum_constraint_residual.max(residuals[edge].abs());
                }

                let endpoint_jacobian = constraint_jacobian_at(&dense_q_next);
                let endpoint_tangency = jacobian_vec(&endpoint_jacobian, &dense_v_next);
                for value in endpoint_tangency {
                    maximum_endpoint_tangency = maximum_endpoint_tangency.max(value.abs());
                }

                dense_q = dense_q_next;
                dense_v = dense_v_next;
                cartesian_state = reference.after;
            }

            assert!(
                maximum_position_error <= 2.0e-7,
                "scale={scale}, position={maximum_position_error:e}"
            );
            assert!(
                maximum_velocity_error <= 2.0e-7,
                "scale={scale}, velocity={maximum_velocity_error:e}"
            );
            assert!(
                maximum_position_multiplier_error <= 2.0e-7,
                "scale={scale}, position multiplier={maximum_position_multiplier_error:e}"
            );
            assert!(
                maximum_velocity_multiplier_error <= 2.0e-7,
                "scale={scale}, velocity multiplier={maximum_velocity_multiplier_error:e}"
            );
            assert!(
                maximum_constraint_residual <= 2.0e-12,
                "scale={scale}, constraint residual={maximum_constraint_residual:e}"
            );
            assert!(
                maximum_endpoint_tangency <= 2.0e-9,
                "scale={scale}, endpoint tangency={maximum_endpoint_tangency:e}"
            );
        }
    }

    fn constraint_jacobian_at(q: &Vec9) -> Jacobian {
        let first = [q[3] - q[0], q[4] - q[1], q[5] - q[2]];
        let second = [q[6] - q[3], q[7] - q[4], q[8] - q[5]];
        let mut jacobian = [[0.0; N]; 2];
        for axis in 0..3 {
            jacobian[0][axis] = -first[axis];
            jacobian[0][3 + axis] = first[axis];
            jacobian[1][3 + axis] = -second[axis];
            jacobian[1][6 + axis] = second[axis];
        }
        jacobian
    }

    fn integrate_cartesian(
        chain: &super::super::CartesianRattleChain,
        mut state: super::super::State,
        dt: f64,
        steps: usize,
        options: super::super::SolverOptions,
    ) -> super::super::State {
        for _ in 0..steps {
            state = chain.step(state, dt, options).unwrap().after;
        }
        state
    }

    #[test]
    fn stronger_dense_shear_exposes_conditioning_error_without_changing_physical_step() {
        let masses = [1.3, 2.1, 0.8];
        let lengths = [1.0_f64, 1.25_f64.sqrt()];
        let q0 = initial_position();
        let v0 = initial_velocity();
        let state = state_from_flat(&q0, &v0);
        let chain = super::super::CartesianRattleChain::new(masses, lengths).unwrap();
        let reference = chain
            .step(
                state,
                0.0025,
                super::super::SolverOptions::default(),
            )
            .unwrap();

        let (basis_i, inverse_i) = basis_pair(3);
        let basis = to_float(&basis_i);
        let inverse = to_float(&inverse_i);
        let transformed_metric = multiply(
            &transpose(&basis),
            &multiply(&mass_metric_with_masses(masses), &basis),
        );
        assert!(cholesky_positive_definite(&transformed_metric));

        let (dense_q, dense_v, _, _, residuals) =
            dense_coordinate_rattle_step(&basis, &inverse, masses, lengths, &q0, &v0, 0.0025);
        let (reference_q, reference_v) = flatten_state(&reference.after);
        let position_error = vector_max_abs(&dense_q, &reference_q);
        let velocity_error = vector_max_abs(&dense_v, &reference_v);
        let maximum_residual = residuals
            .iter()
            .map(|residual| residual.abs())
            .fold(0.0_f64, f64::max);
        let endpoint_jacobian = constraint_jacobian_at(&dense_q);
        let maximum_tangency = jacobian_vec(&endpoint_jacobian, &dense_v)
            .iter()
            .map(|value| value.abs())
            .fold(0.0_f64, f64::max);

        assert!(
            position_error <= 2.0e-5,
            "scale=3, position error={position_error:e}"
        );
        assert!(
            velocity_error <= 2.0e-5,
            "scale=3, velocity error={velocity_error:e}"
        );
        assert!(
            maximum_residual <= 2.0e-12,
            "scale=3, link residual={maximum_residual:e}"
        );
        assert!(
            maximum_tangency <= 2.0e-7,
            "scale=3, endpoint tangency={maximum_tangency:e}"
        );
    }

    #[test]
    fn dense_coordinate_rattle_tracks_cartesian_over_refinement_and_longer_horizon() {
        let masses = [1.3, 2.1, 0.8];
        let lengths = [1.0_f64, 1.25_f64.sqrt()];
        let horizon = 0.32_f64;
        let chain = super::super::CartesianRattleChain::new(masses, lengths).unwrap();
        let options = super::super::SolverOptions::default();
        let q0 = initial_position();
        let v0 = initial_velocity();
        let initial = state_from_flat(&q0, &v0);

        // The Cartesian implementation is independently stepped at three
        // resolutions over the same physical horizon.
        let coarse = integrate_cartesian(&chain, initial, 0.01, 32, options);
        let fine = integrate_cartesian(&chain, initial, 0.005, 64, options);
        let reference = integrate_cartesian(&chain, initial, 0.000625, 512, options);
        let (coarse_q, coarse_v) = flatten_state(&coarse);
        let (fine_q, fine_v) = flatten_state(&fine);
        let (reference_q, reference_v) = flatten_state(&reference);
        let coarse_error = vector_max_abs(&coarse_q, &reference_q)
            .max(vector_max_abs(&coarse_v, &reference_v));
        let fine_error = vector_max_abs(&fine_q, &reference_q)
            .max(vector_max_abs(&fine_v, &reference_v));
        assert!(coarse_error > 1.0e-12, "refinement fixture must be nontrivial");
        assert!(
            fine_error < coarse_error * 0.4,
            "expected second-order refinement: coarse={coarse_error:e}, fine={fine_error:e}"
        );

        // Independently execute the dense-coordinate step for the longer,
        // finer trajectory. Both dense bases must track the Cartesian oracle.
        let dt = horizon / 64.0;
        for scale in [1_i64, 2_i64] {
            let (basis_i, inverse_i) = basis_pair(scale);
            let basis = to_float(&basis_i);
            let inverse = to_float(&inverse_i);
            let mut dense_q = q0;
            let mut dense_v = v0;
            let mut cartesian_state = initial;
            let mut maximum_position_error = 0.0_f64;
            let mut maximum_velocity_error = 0.0_f64;
            let mut maximum_position_residual = 0.0_f64;
            let mut maximum_endpoint_tangency = 0.0_f64;

            for _step in 0..64 {
                let reference_step = chain.step(cartesian_state, dt, options).unwrap();
                let (dense_q_next, dense_v_next, _, _, residuals) =
                    dense_coordinate_rattle_step(
                        &basis, &inverse, masses, lengths, &dense_q, &dense_v, dt,
                    );
                let (reference_q, reference_v) = flatten_state(&reference_step.after);
                maximum_position_error =
                    maximum_position_error.max(vector_max_abs(&dense_q_next, &reference_q));
                maximum_velocity_error =
                    maximum_velocity_error.max(vector_max_abs(&dense_v_next, &reference_v));
                for residual in residuals {
                    maximum_position_residual = maximum_position_residual.max(residual.abs());
                }
                let endpoint_jacobian = constraint_jacobian_at(&dense_q_next);
                for tangent in jacobian_vec(&endpoint_jacobian, &dense_v_next) {
                    maximum_endpoint_tangency =
                        maximum_endpoint_tangency.max(tangent.abs());
                }
                dense_q = dense_q_next;
                dense_v = dense_v_next;
                cartesian_state = reference_step.after;
            }

            assert!(
                maximum_position_error <= 3.0e-7,
                "scale={scale}, 64-step position error={maximum_position_error:e}"
            );
            assert!(
                maximum_velocity_error <= 3.0e-7,
                "scale={scale}, 64-step velocity error={maximum_velocity_error:e}"
            );
            assert!(
                maximum_position_residual <= 3.0e-12,
                "scale={scale}, link residual={maximum_position_residual:e}"
            );
            assert!(
                maximum_endpoint_tangency <= 3.0e-9,
                "scale={scale}, tangent defect={maximum_endpoint_tangency:e}"
            );
        }
    }

    #[test]
    fn both_dense_integer_basis_changes_have_exact_inverse_witnesses() {
        for scale in [1_i64, 2_i64] {
            let (basis_i, inverse_i) = basis_pair(scale);
            for row in 0..N {
                for column in 0..N {
                    let mut left = 0_i64;
                    let mut right = 0_i64;
                    for inner in 0..N {
                        left += basis_i[row][inner] * inverse_i[inner][column];
                        right += inverse_i[row][inner] * basis_i[inner][column];
                    }
                    let expected = if row == column { 1_i64 } else { 0_i64 };
                    assert_eq!(left, expected);
                    assert_eq!(right, expected);
                }
            }
            assert!(
                basis_i
                    .iter()
                    .flatten()
                    .filter(|value| **value != 0)
                    .count()
                    >= 70
            );
        }
    }

    #[test]
    fn transformed_state_force_metric_and_constraint_laws_match_frozen_outputs() {
        let q = initial_position();
        let v = initial_velocity();
        let force = force_covector();
        let metric = mass_metric();
        let jacobian = constraint_jacobian();

        let expected = [
            (
                [1.0, 1.0, 1.0, 3.0, 1.0, 1.0, 2.0, 3.0, 3.5],
                [1.2, 1.3, 1.0, 2.3, 1.6, 1.1, 1.4, 2.8, 3.1],
                [0.09, -0.21, -0.06, -0.26, 0.19, 0.23, 0.16, 0.14, -0.09],
                [
                    [0.0, 1.0, 1.0, 2.0, -1.0, -1.0, -1.0, -1.0, 0.0],
                    [2.0, 2.0, 2.0, 2.0, -3.0, -2.5, -2.0, -1.0, 0.5],
                ],
            ),
            (
                [2.0, 2.0, 2.0, 5.0, 2.0, 2.0, 3.0, 5.0, 6.5],
                [2.3, 2.4, 2.1, 4.5, 2.7, 2.2, 2.5, 5.0, 6.4],
                [-0.02, -0.32, -0.17, -0.37, 0.3, 0.34, 0.27, 0.25, -0.09],
                [
                    [1.0, 2.0, 2.0, 3.0, -2.0, -2.0, -2.0, -2.0, 0.0],
                    [4.0, 4.0, 4.0, 4.0, -5.0, -4.5, -4.0, -3.0, 0.5],
                ],
            ),
        ];

        for (case_index, scale) in [1_i64, 2_i64].into_iter().enumerate() {
            let (basis_i, inverse_i) = basis_pair(scale);
            let basis = to_float(&basis_i);
            let inverse = to_float(&inverse_i);
            let transformed_metric = multiply(&transpose(&basis), &multiply(&metric, &basis));
            let transformed_q = mat_vec(&inverse, &q);
            let transformed_v = mat_vec(&inverse, &v);
            let transformed_force = mat_vec(&transpose(&basis), &force);
            let transformed_jacobian = jacobian_transform(&jacobian, &basis);

            assert_close(&transformed_q, &expected[case_index].0, 1.0e-12);
            assert_close(&transformed_v, &expected[case_index].1, 1.0e-12);
            assert_close(&transformed_force, &expected[case_index].2, 1.0e-12);
            for row in 0..2 {
                assert!(
                    vector_max_abs(&transformed_jacobian[row], &expected[case_index].3[row])
                        <= 1.0e-12
                );
            }
            assert!(cholesky_positive_definite(&transformed_metric));

            let canonical_energy = dot(&v, &mat_vec(&metric, &v));
            let transformed_energy = dot(
                &transformed_v,
                &mat_vec(&transformed_metric, &transformed_v),
            );
            assert!((canonical_energy - transformed_energy).abs() <= 1.0e-8);

            let canonical_tangency = jacobian_vec(&jacobian, &v);
            let transformed_tangency = jacobian_vec(&transformed_jacobian, &transformed_v);
            assert!(canonical_tangency
                .iter()
                .all(|value| value.abs() <= 1.0e-12));
            assert!(transformed_tangency
                .iter()
                .all(|value| value.abs() <= 1.0e-11));

            let canonical_momentum = mat_vec(&metric, &v);
            let transformed_momentum = mat_vec(&transpose(&basis), &canonical_momentum);
            let delta_y = [0.3, -0.7, 0.2, 0.5, -0.1, 0.9, -0.4, 0.6, 0.25];
            let delta_q = mat_vec(&basis, &delta_y);
            let canonical_pairing = dot(&canonical_momentum, &delta_q);
            let transformed_pairing = dot(&transformed_momentum, &delta_y);
            assert!((canonical_pairing - transformed_pairing).abs() <= 1.0e-10);

            let canonical_power = dot(&force, &v);
            let transformed_power = dot(&transformed_force, &transformed_v);
            assert!((canonical_power - transformed_power).abs() <= 1.0e-10);

            let lambda = [0.37, -0.21];
            let canonical_reaction = jacobian_reaction(&jacobian, lambda);
            let transformed_reaction = jacobian_reaction(&transformed_jacobian, lambda);
            let expected_transformed_reaction = mat_vec(&transpose(&basis), &canonical_reaction);
            assert!(
                vector_max_abs(&transformed_reaction, &expected_transformed_reaction) <= 1.0e-10
            );

            for axis in 0..3 {
                let generator_q = generator(axis);
                let generator_y = multiply(&inverse, &multiply(&generator_q, &basis));
                assert!(metric_skew_defect(&generator_y, &transformed_metric) <= 1.0e-7);
            }
        }
    }

    #[test]
    fn wrong_force_metric_jacobian_and_generator_laws_are_detected() {
        let (basis_i, inverse_i) = basis_pair(1);
        let basis = to_float(&basis_i);
        let inverse = to_float(&inverse_i);
        let metric = mass_metric();
        let force = force_covector();
        let jacobian = constraint_jacobian();
        let transformed_metric = multiply(&transpose(&basis), &multiply(&metric, &basis));
        let transformed_force = mat_vec(&transpose(&basis), &force);
        let force_as_vector = mat_vec(&inverse, &force);
        assert!(vector_max_abs(&transformed_force, &force_as_vector) > 1.0e-2);

        let wrong_metric = multiply(&transpose(&inverse), &multiply(&metric, &inverse));
        let mut metric_defect = [[0.0; N]; N];
        for row in 0..N {
            for column in 0..N {
                metric_defect[row][column] =
                    transformed_metric[row][column] - wrong_metric[row][column];
            }
        }
        assert!(max_abs(&metric_defect) > 1.0);

        let transformed_jacobian = jacobian_transform(&jacobian, &basis);
        let wrong_jacobian = jacobian_transform(&jacobian, &inverse);
        assert!(vector_max_abs(&transformed_jacobian[0], &wrong_jacobian[0]) > 1.0);
        assert!(vector_max_abs(&transformed_jacobian[1], &wrong_jacobian[1]) > 1.0);

        let generator_q = generator(2);
        let correct_generator = multiply(&inverse, &multiply(&generator_q, &basis));
        let wrong_generator = multiply(&basis, &multiply(&generator_q, &inverse));
        assert!(metric_skew_defect(&correct_generator, &transformed_metric) <= 1.0e-7);
        assert!(metric_skew_defect(&wrong_generator, &transformed_metric) > 1.0);
        let mut identity = [[0.0; N]; N];
        for index in 0..N {
            identity[index][index] = 1.0;
        }
        assert!(metric_skew_defect(&correct_generator, &identity) > 1.0);
    }

    #[test]
    fn singular_basis_control_has_no_two_sided_inverse() {
        let (mut basis, inverse) = basis_pair(1);
        for column in 0..N {
            basis[0][column] = 0;
        }
        let mut product = [[0_i64; N]; N];
        for row in 0..N {
            for column in 0..N {
                for inner in 0..N {
                    product[row][column] += basis[row][inner] * inverse[inner][column];
                }
            }
        }
        assert!(product.iter().enumerate().any(|(row, values)| {
            values
                .iter()
                .enumerate()
                .any(|(column, value)| *value != if row == column { 1 } else { 0 })
        }));
    }
}
