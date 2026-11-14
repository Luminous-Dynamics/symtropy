// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Exact rank transition when two point contacts on one rigid body approach
//! coincidence. No floating-point singular-value threshold is used.

type Matrix6 = [[i64; 6]; 6];
type Vector6 = [i64; 6];

const SIX_TIMES_INVERSE_MASS: Vector6 = [6, 6, 6, 6, 3, 2];
const TANGENT_ONE_NULL: Vector6 = [0, 1, 0, 0, -1, 0];
const NORMAL_DUPLICATE_NULL: Vector6 = [1, 0, 0, -1, 0, 0];
const TANGENT_TWO_DUPLICATE_NULL: Vector6 = [0, 0, 1, 0, 0, -1];

fn scaled_jacobian(denominator: i64, separation_numerator: i64) -> Matrix6 {
    // Contact locations: r1=(0,0,0), r2=(0,d,0), d=k/m.
    // Each row is scaled by m, preserving exact rational geometry.
    [
        [denominator, 0, 0, 0, 0, 0],
        [0, denominator, 0, 0, 0, 0],
        [0, 0, denominator, 0, 0, 0],
        [denominator, 0, 0, 0, 0, -separation_numerator],
        [0, denominator, 0, 0, 0, 0],
        [0, 0, denominator, separation_numerator, 0, 0],
    ]
}

fn assemble_six_times_scaled_delassus(jacobian: &Matrix6) -> Matrix6 {
    // This is 6*m^2*W(d) = J_m*diag(6,6,6,6,3,2)*J_m^T.
    let mut result = [[0_i64; 6]; 6];
    for row in 0..6 {
        for column in 0..6 {
            for coordinate in 0..6 {
                result[row][column] += jacobian[row][coordinate]
                    * SIX_TIMES_INVERSE_MASS[coordinate]
                    * jacobian[column][coordinate];
            }
        }
    }
    result
}

fn determinant_bareiss(mut matrix: Vec<Vec<i64>>) -> i64 {
    let dimension = matrix.len();
    assert!(dimension > 0);
    assert!(matrix.iter().all(|row| row.len() == dimension));
    let mut sign = 1_i64;
    let mut previous_pivot = 1_i64;

    for pivot_index in 0..dimension - 1 {
        if matrix[pivot_index][pivot_index] == 0 {
            let replacement = (pivot_index + 1..dimension)
                .find(|&row| matrix[row][pivot_index] != 0)
                .expect("the selected exact Gram minor must have a pivot");
            matrix.swap(pivot_index, replacement);
            sign = -sign;
        }
        let pivot = matrix[pivot_index][pivot_index];
        let (pivot_rows, remaining_rows) = matrix.split_at_mut(pivot_index + 1);
        let pivot_row = &pivot_rows[pivot_index];
        for row in remaining_rows.iter_mut() {
            for column in pivot_index + 1..dimension {
                let numerator = row[column] * pivot - row[pivot_index] * pivot_row[column];
                assert_eq!(numerator % previous_pivot, 0);
                row[column] = numerator / previous_pivot;
            }
            row[pivot_index] = 0;
        }
        previous_pivot = pivot;
    }
    sign * matrix[dimension - 1][dimension - 1]
}

fn principal_minor(matrix: &Matrix6, indices: &[usize]) -> i64 {
    let minor = indices
        .iter()
        .map(|&row| {
            indices
                .iter()
                .map(|&column| matrix[row][column])
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    determinant_bareiss(minor)
}

fn matrix_times_vector(matrix: &Matrix6, vector: &Vector6) -> Vector6 {
    std::array::from_fn(|row| {
        (0..6)
            .map(|column| matrix[row][column] * vector[column])
            .sum()
    })
}

fn transpose_times_vector(matrix: &Matrix6, vector: &Vector6) -> Vector6 {
    std::array::from_fn(|column| (0..6).map(|row| matrix[row][column] * vector[row]).sum())
}

#[test]
fn exact_geometry_rank_changes_from_five_to_three_at_contact_coincidence() {
    // The rank is stable for any nonzero separation. Scaling by m keeps
    // rational d=k/m exact and makes the rank witness an integer minor.
    for (k, m) in [(1_i64, 1_i64), (1, 2), (1, 10), (-3, 4)] {
        assert!(m > 0 && k != 0);
        let j = scaled_jacobian(m, k);
        let w = assemble_six_times_scaled_delassus(&j);

        // A nonzero 5x5 principal minor is exactly 2592*k^4*m^6.
        // The exact common tangent-row null vector proves rank <= 5.
        let rank_five_minor = principal_minor(&w, &[0, 1, 2, 3, 5]);
        assert_eq!(rank_five_minor, 2592 * k.pow(4) * m.pow(6));
        assert_eq!(matrix_times_vector(&w, &TANGENT_ONE_NULL), [0; 6]);
        assert_eq!(transpose_times_vector(&j, &TANGENT_ONE_NULL), [0; 6]);
    }

    // At coincidence, both point Jacobian blocks are identical. A 3x3
    // principal minor proves rank >= 3; three independent exact null vectors
    // prove rank <= 3. No threshold or numerical rank routine is involved.
    let m = 7_i64;
    let j0 = scaled_jacobian(m, 0);
    let w0 = assemble_six_times_scaled_delassus(&j0);
    assert_eq!(principal_minor(&w0, &[0, 1, 2]), 216 * m.pow(6));

    for null in [
        TANGENT_ONE_NULL,
        NORMAL_DUPLICATE_NULL,
        TANGENT_TWO_DUPLICATE_NULL,
    ] {
        assert_eq!(transpose_times_vector(&j0, &null), [0; 6]);
        assert_eq!(matrix_times_vector(&w0, &null), [0; 6]);
    }

    // These three null vectors are independent: the selected 3x3 coordinate
    // minor on indices (1,2,3) has determinant -1.
    let null_minor = determinant_bareiss(vec![
        vec![
            TANGENT_ONE_NULL[1],
            NORMAL_DUPLICATE_NULL[1],
            TANGENT_TWO_DUPLICATE_NULL[1],
        ],
        vec![
            TANGENT_ONE_NULL[2],
            NORMAL_DUPLICATE_NULL[2],
            TANGENT_TWO_DUPLICATE_NULL[2],
        ],
        vec![
            TANGENT_ONE_NULL[3],
            NORMAL_DUPLICATE_NULL[3],
            TANGENT_TWO_DUPLICATE_NULL[3],
        ],
    ]);
    assert_eq!(null_minor, 1);
}
