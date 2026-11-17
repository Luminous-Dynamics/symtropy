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

fn mass_metric() -> Mat9 {
    let masses = [1.3, 2.1, 0.8];
    let mut metric = [[0.0; N]; N];
    for body in 0..3 {
        for axis in 0..3 {
            let index = 3 * body + axis;
            metric[index][index] = masses[body];
        }
    }
    metric
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

#[cfg(test)]
mod tests {
    use super::*;

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
            assert!(basis_i.iter().flatten().filter(|value| **value != 0).count() >= 70);
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
            let transformed_metric =
                multiply(&transpose(&basis), &multiply(&metric, &basis));
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
            let transformed_energy =
                dot(&transformed_v, &mat_vec(&transformed_metric, &transformed_v));
            assert!((canonical_energy - transformed_energy).abs() <= 1.0e-8);

            let canonical_tangency = jacobian_vec(&jacobian, &v);
            let transformed_tangency = jacobian_vec(&transformed_jacobian, &transformed_v);
            assert!(canonical_tangency.iter().all(|value| value.abs() <= 1.0e-12));
            assert!(transformed_tangency.iter().all(|value| value.abs() <= 1.0e-11));

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
                metric_defect[row][column] = transformed_metric[row][column] - wrong_metric[row][column];
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
