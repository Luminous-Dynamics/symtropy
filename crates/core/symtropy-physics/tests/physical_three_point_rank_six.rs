// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Exact three-point rigid-body contact island with a three-dimensional
//! multiplier nullspace and a positive-dimensional admissible allocation set.

type Jacobian9x6 = [[i64; 6]; 9];
type Matrix9 = [[i64; 9]; 9];
type Vector9 = [i64; 9];
type Vector6 = [i64; 6];

const J: Jacobian9x6 = [
    [1, 0, 0, 0, 0, 0],
    [0, 1, 0, 0, 0, 0],
    [0, 0, 1, 0, 0, 0],
    [1, 0, 0, 0, 0, -1],
    [0, 1, 0, 0, 0, 0],
    [0, 0, 1, 1, 0, 0],
    [1, 0, 0, 0, 1, 0],
    [0, 1, 0, -1, 0, 0],
    [0, 0, 1, 0, 0, 0],
];

// Six times M^{-1}=diag(1,1,1,1,1/2,1/3).
const SIX_TIMES_INVERSE_MASS: Vector6 = [6, 6, 6, 6, 3, 2];

const EXPECTED_SIX_TIMES_DELASSUS: Matrix9 = [
    [6, 0, 0, 6, 0, 0, 6, 0, 0],
    [0, 6, 0, 0, 6, 0, 0, 6, 0],
    [0, 0, 6, 0, 0, 6, 0, 0, 6],
    [6, 0, 0, 8, 0, 0, 6, 0, 0],
    [0, 6, 0, 0, 6, 0, 0, 6, 0],
    [0, 0, 6, 0, 0, 12, 0, -6, 6],
    [6, 0, 0, 6, 0, 0, 9, 0, 0],
    [0, 6, 0, 0, 6, -6, 0, 12, 0],
    [0, 0, 6, 0, 0, 6, 0, 0, 6],
];

const NULL_A: Vector9 = [0, 1, 0, 0, -1, 0, 0, 0, 0];
const NULL_B: Vector9 = [0, 0, 1, 0, 0, 0, 0, 0, -1];
const NULL_C: Vector9 = [0, -1, -1, 0, 0, 1, 0, 1, 0];
const BASE_IMPULSE: Vector9 = [2, 0, 0, 2, 0, 0, 2, 0, 0];
const SIX_TIMES_FREE_CONTACT_VELOCITY: Vector9 = [-36, 0, 0, -40, 0, 0, -42, 0, 0];
const SIX_TIMES_FREE_GENERALIZED_VELOCITY: Vector6 = [-36, 0, 0, 0, -6, 4];

fn transpose_times_vector(vector: &Vector9) -> Vector6 {
    let mut result = [0_i64; 6];
    for row in 0..9 {
        for column in 0..6 {
            result[column] += J[row][column] * vector[row];
        }
    }
    result
}

fn jacobian_times_vector(vector: &Vector6) -> Vector9 {
    let mut result = [0_i64; 9];
    for row in 0..9 {
        for column in 0..6 {
            result[row] += J[row][column] * vector[column];
        }
    }
    result
}

fn delassus_times_vector(matrix: &Matrix9, vector: &Vector9) -> Vector9 {
    let mut result = [0_i64; 9];
    for row in 0..9 {
        for column in 0..9 {
            result[row] += matrix[row][column] * vector[column];
        }
    }
    result
}

fn dot(lhs: &Vector9, rhs: &Vector9) -> i64 {
    (0..9).map(|index| lhs[index] * rhs[index]).sum()
}

fn geometry_delassus_sixths() -> Matrix9 {
    let mut result = [[0_i64; 9]; 9];
    for row in 0..9 {
        for column in 0..9 {
            for coordinate in 0..6 {
                result[row][column] +=
                    J[row][coordinate] * SIX_TIMES_INVERSE_MASS[coordinate] * J[column][coordinate];
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
        for row in pivot_index + 1..dimension {
            for column in pivot_index + 1..dimension {
                let numerator = matrix[row][column] * pivot
                    - matrix[row][pivot_index] * matrix[pivot_index][column];
                assert_eq!(numerator % previous_pivot, 0);
                matrix[row][column] = numerator / previous_pivot;
            }
        }
        for row in pivot_index + 1..dimension {
            matrix[row][pivot_index] = 0;
        }
        previous_pivot = pivot;
    }
    sign * matrix[dimension - 1][dimension - 1]
}

fn six_by_six_gram_minor(matrix: &Matrix9, indices: [usize; 6]) -> i64 {
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

fn leading_principal_minor(matrix: &Matrix9, size: usize) -> i64 {
    assert!((1..=9).contains(&size));
    let minor = matrix
        .iter()
        .take(size)
        .map(|row| row.iter().take(size).copied().collect::<Vec<_>>())
        .collect::<Vec<_>>();
    determinant_bareiss(minor)
}

fn quadratic_form(matrix: &Matrix9, vector: &Vector9) -> i64 {
    dot(vector, &delassus_times_vector(matrix, vector))
}

fn assert_zero(vector: &Vector9) {
    assert_eq!(*vector, [0; 9]);
}

#[test]
fn three_non_collinear_point_contacts_have_rank_six_and_nullity_three() {
    // Contacts at (0,0,0), (0,1,0), (0,0,1) with normal/tangent frame
    // (ex,ey,ez). This is a non-collinear three-point rigid-body island.
    let w6 = geometry_delassus_sixths();
    assert_eq!(w6, EXPECTED_SIX_TIMES_DELASSUS);
    for row in 0..9 {
        for column in 0..9 {
            assert_eq!(w6[row][column], w6[column][row]);
        }
    }

    // The Gram identity proves PSD exactly. A deterministic integer panel
    // checks the assembled quadratic form against J^T x and positive M^-1.
    for seed in 0..48_i64 {
        let x: Vector9 = std::array::from_fn(|index| {
            ((seed * (index as i64 + 3) + 5 * index as i64) % 23) - 11
        });
        let jtx = transpose_times_vector(&x);
        let gram_value: i64 = (0..6)
            .map(|coordinate| SIX_TIMES_INVERSE_MASS[coordinate] * jtx[coordinate].pow(2))
            .sum();
        assert_eq!(quadratic_form(&w6, &x), gram_value);
        assert!(gram_value >= 0);
    }

    // Exact rank six: the selected 6x6 principal Gram minor is 7776.
    // Since J has six columns, this proves full column rank; W has rank six.
    assert_eq!(six_by_six_gram_minor(&w6, [0, 1, 2, 3, 5, 6]), 7776);

    // These three independent exact vectors form a basis of ker(J^T)=ker(W).
    for null in [NULL_A, NULL_B, NULL_C] {
        assert_eq!(transpose_times_vector(&null), [0; 6]);
        assert_zero(&delassus_times_vector(&w6, &null));
    }
    // Their coordinates 4, 8 and 5 respectively certify linear independence.
    let independence_minor = determinant_bareiss(vec![
        vec![NULL_A[4], NULL_B[4], NULL_C[4]],
        vec![NULL_A[8], NULL_B[8], NULL_C[8]],
        vec![NULL_A[5], NULL_B[5], NULL_C[5]],
    ]);
    assert_eq!(independence_minor, 1);
    assert_eq!(9 - 6, 3, "contact multiplier nullity is three");
}

#[test]
fn three_contact_island_has_a_three_dimensional_admissible_allocation_fibre() {
    let w6 = geometry_delassus_sixths();
    let q6 = SIX_TIMES_FREE_CONTACT_VELOCITY;
    assert_eq!(delassus_times_vector(&w6, &BASE_IMPULSE), [
        36, 0, 0, 40, 0, 0, 42, 0, 0
    ]);
    assert_eq!(
        jacobian_times_vector(&SIX_TIMES_FREE_GENERALIZED_VELOCITY),
        q6,
        "q must be generated by a physical free generalized twist"
    );

    // p=p0+a*nA+b*nB+c*nC. For the rational panel a=A/d, b=B/d,
    // c=C/d, the integer vector P=d*p is used throughout.
    for (a, b, c, d) in [
        (0_i64, 0_i64, 0_i64, 1_i64),
        (1, 0, 0, 4),
        (0, -1, 0, 4),
        (0, 0, 1, 4),
        (1, -1, 1, 4),
        (2, 0, -1, 5),
        (-1, 2, 1, 6),
    ] {
        assert!(d > 0);
        let mut p = [0_i64; 9];
        for index in 0..9 {
            p[index] = d * BASE_IMPULSE[index]
                + a * NULL_A[index]
                + b * NULL_B[index]
                + c * NULL_C[index];
        }

        // q+Wp=0 and J^T p is the same generalized body impulse as p0.
        let mut post_velocity = delassus_times_vector(&w6, &p);
        for index in 0..9 {
            post_velocity[index] += d * q6[index];
        }
        assert_zero(&post_velocity);
        let body_impulse = transpose_times_vector(&p);
        let expected_body_impulse: Vector6 =
            std::array::from_fn(|index| d * transpose_times_vector(&BASE_IMPULSE)[index]);
        assert_eq!(body_impulse, expected_body_impulse);

        // Local Coulomb disks, with mu=1/2 and normal impulse 2 at each point.
        let disk_slacks = [
            p[0].pow(2) - 4 * (p[1].pow(2) + p[2].pow(2)),
            p[3].pow(2) - 4 * (p[4].pow(2) + p[5].pow(2)),
            p[6].pow(2) - 4 * (p[7].pow(2) + p[8].pow(2)),
        ];
        assert!(p[0] > 0 && p[3] > 0 && p[6] > 0);
        assert!(disk_slacks.iter().all(|&slack| slack > 0));

        // The exact kinetic-energy change remains -59/3 throughout the fibre.
        // With sixth-scaled W and q, the common numerator equals -236*d^2.
        let energy_numerator = 2 * d * dot(&p, &q6) + quadratic_form(&w6, &p);
        assert_eq!(energy_numerator, -236 * d.pow(2));

        // p0 is orthogonal to the three-dimensional nullspace. In these
        // contact coordinates it is the unique minimum-Euclidean-norm
        // representative, but that selection does not erase the fibre.
        let norm_increment =
            dot(&p, &p) - d.pow(2) * dot(&BASE_IMPULSE, &BASE_IMPULSE);
        assert_eq!(
            norm_increment,
            2 * a.pow(2) + 2 * b.pow(2) + 4 * c.pow(2) - 2 * a * c - 2 * b * c
        );
        assert!(norm_increment >= 0);
    }

    // The three null parameters describe a genuine open neighbourhood around
    // zero: the base point has strict slack 4 on all disks, so small perturbations
    // in a,b,c remain locally admissible. No unique contact impulse exists.
}
#[test]
fn proximal_cone_metric_lifts_rank_six_nine_by_nine_delassus_to_exact_spd() {
    let w6 = geometry_delassus_sixths();
    assert_eq!(six_by_six_gram_minor(&w6, [0, 1, 2, 3, 5, 6]), 7776);

    // For gamma in {2,1,1/2}, 6(W+gamma^{-1}I)=6W+shift*I
    // with shift in {3,6,12}. Exact Sylvester minors prove the proximal
    // inner QP metric SPD even though the unregularized physical W has nullity 3.
    for shift in [3_i64, 6, 12] {
        let lifted = std::array::from_fn(|row| {
            std::array::from_fn(|column| {
                w6[row][column] + if row == column { shift } else { 0 }
            })
        });
        for size in 1..=9 {
            let determinant = leading_principal_minor(&lifted, size);
            assert!(
                determinant > 0,
                "W_gamma leading minor size={size}, shift={shift}: {determinant}"
            );
        }
    }
}
