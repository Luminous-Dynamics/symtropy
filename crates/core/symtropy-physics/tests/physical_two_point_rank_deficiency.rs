// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Exact geometry-derived rank-deficient Delassus witness for two point contacts
//! on one rigid body. All arithmetic is integer/rational; no floating rank
//! threshold, pseudoinverse, or epsilon regularization is used.

type Matrix6 = [[i64; 6]; 6];
type Vector6 = [i64; 6];

const CONTACT_JACOBIAN: Matrix6 = [
    [1, 0, 0, 0, 0, 0],
    [0, 1, 0, 0, 0, 0],
    [0, 0, 1, 0, 0, 0],
    [1, 0, 0, 0, 0, -1],
    [0, 1, 0, 0, 0, 0],
    [0, 0, 1, 1, 0, 0],
];

// Six times M^{-1} = diag(1, 1, 1, 1, 1/2, 1/3).
const SIX_TIMES_INVERSE_MASS: Vector6 = [6, 6, 6, 6, 3, 2];

// SIX_TIMES_DELASSUS = 6 J M^{-1} J^T. The literal is also checked against
// the independently assembled Gram product below.
const EXPECTED_SIX_TIMES_DELASSUS: Matrix6 = [
    [6, 0, 0, 6, 0, 0],
    [0, 6, 0, 0, 6, 0],
    [0, 0, 6, 0, 0, 6],
    [6, 0, 0, 8, 0, 0],
    [0, 6, 0, 0, 6, 0],
    [0, 0, 6, 0, 0, 12],
];

const NULL_IMPULSE: Vector6 = [0, 1, 0, 0, -1, 0];
const BASE_IMPULSE: Vector6 = [2, 0, 0, 2, 0, 0];
// Six times q = 6 J v_free, for v_free=( -4,0,0, 0,0,2/3 ).
const SIX_TIMES_FREE_CONTACT_VELOCITY: Vector6 = [-24, 0, 0, -28, 0, 0];

fn transpose_times_vector(matrix: &Matrix6, vector: &Vector6) -> Vector6 {
    let mut result = [0; 6];
    for row in 0..6 {
        for column in 0..6 {
            result[column] += matrix[row][column] * vector[row];
        }
    }
    result
}

fn matrix_times_vector(matrix: &Matrix6, vector: &Vector6) -> Vector6 {
    let mut result = [0; 6];
    for row in 0..6 {
        for column in 0..6 {
            result[row] += matrix[row][column] * vector[column];
        }
    }
    result
}

fn dot(lhs: &Vector6, rhs: &Vector6) -> i64 {
    (0..6).map(|index| lhs[index] * rhs[index]).sum()
}

fn geometry_delassus_sixths() -> Matrix6 {
    let mut result = [[0; 6]; 6];
    for row in 0..6 {
        for column in 0..6 {
            for generalized_coordinate in 0..6 {
                result[row][column] += CONTACT_JACOBIAN[row][generalized_coordinate]
                    * SIX_TIMES_INVERSE_MASS[generalized_coordinate]
                    * CONTACT_JACOBIAN[column][generalized_coordinate];
            }
        }
    }
    result
}

fn principal_minor(matrix: &Matrix6, indices: [usize; 5]) -> i64 {
    let mut minor = vec![vec![0_i64; 5]; 5];
    for row in 0..5 {
        for column in 0..5 {
            minor[row][column] = matrix[indices[row]][indices[column]];
        }
    }
    determinant_bareiss(minor)
}

// Fraction-free Bareiss elimination: exact for this small integer matrix.
fn determinant_bareiss(mut matrix: Vec<Vec<i64>>) -> i64 {
    let dimension = matrix.len();
    assert!(dimension > 0);
    let mut sign = 1_i64;
    let mut previous_pivot = 1_i64;

    for pivot_index in 0..dimension - 1 {
        if matrix[pivot_index][pivot_index] == 0 {
            let replacement = (pivot_index + 1..dimension)
                .find(|&row| matrix[row][pivot_index] != 0)
                .expect("the selected minor must have a nonzero pivot");
            matrix.swap(pivot_index, replacement);
            sign = -sign;
        }

        let pivot = matrix[pivot_index][pivot_index];
        let (pivot_rows, remaining_rows) = matrix.split_at_mut(pivot_index + 1);
        let pivot_row = &pivot_rows[pivot_index];
        for row in remaining_rows.iter_mut() {
            let row_pivot = row[pivot_index];
            for (column, value) in row.iter_mut().enumerate().skip(pivot_index + 1) {
                let numerator = *value * pivot - row_pivot * pivot_row[column];
                assert_eq!(
                    numerator % previous_pivot,
                    0,
                    "Bareiss division must remain exact"
                );
                *value = numerator / previous_pivot;
            }
            row[pivot_index] = 0;
        }
        previous_pivot = pivot;
    }

    sign * matrix[dimension - 1][dimension - 1]
}

fn leading_principal_minor(matrix: &Matrix6, size: usize) -> i64 {
    assert!((1..=6).contains(&size));
    let minor = matrix
        .iter()
        .take(size)
        .map(|row| row.iter().take(size).copied().collect::<Vec<_>>())
        .collect::<Vec<_>>();
    determinant_bareiss(minor)
}

fn quadratic_form(matrix: &Matrix6, vector: &Vector6) -> i64 {
    dot(vector, &matrix_times_vector(matrix, vector))
}

fn assert_zero(vector: &Vector6) {
    assert_eq!(*vector, [0; 6]);
}

#[test]
fn two_point_rigid_body_delassus_is_exact_psd_with_rank_five() {
    // The two contacts are at r1=(0,0,0), r2=(0,1,0) on a unit-mass rigid
    // body with principal inertia diag(1,2,3). Each contact frame is
    // (normal,tangent1,tangent2)=(ex,ey,ez).
    let w6 = geometry_delassus_sixths();
    assert_eq!(w6, EXPECTED_SIX_TIMES_DELASSUS);
    for (row_index, row) in w6.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate() {
            assert_eq!(*value, w6[column_index][row_index]);
        }
    }

    // W=J M^{-1} J^T is PSD by the exact Gram identity. The deterministic
    // integer panel checks that the assembled matrix and the Gram quadratic
    // form agree; the positive diagonal in M^{-1} is the analytic PSD proof.
    for seed in 0..64_i64 {
        let x = [
            seed - 17,
            3 * seed - 11,
            7 - seed,
            2 * seed + 1,
            13 - 2 * seed,
            seed % 9 - 4,
        ];
        let jtx = transpose_times_vector(&CONTACT_JACOBIAN, &x);
        let gram_value: i64 = (0..6)
            .map(|coordinate| SIX_TIMES_INVERSE_MASS[coordinate] * jtx[coordinate].pow(2))
            .sum();
        assert_eq!(quadratic_form(&w6, &x), gram_value);
        assert!(gram_value >= 0);
    }

    // The contact-impulse null vector is the difference between the two
    // identical y-tangent Jacobian rows.
    assert_zero(&matrix_times_vector(&w6, &NULL_IMPULSE));
    assert_zero(&transpose_times_vector(&CONTACT_JACOBIAN, &NULL_IMPULSE));

    // A nonzero 5x5 minor proves rank >= 5; the exact null vector proves
    // rank <= 5. This is an exact rank-five result, not a singular-value cutoff.
    let minor = principal_minor(&w6, [0, 2, 3, 4, 5]);
    assert_eq!(minor, 2592);
}

#[test]
fn physically_admissible_impulses_form_a_continuum_with_one_body_response() {
    let w6 = geometry_delassus_sixths();
    let q6 = SIX_TIMES_FREE_CONTACT_VELOCITY;
    let w6p0 = matrix_times_vector(&w6, &BASE_IMPULSE);
    assert_eq!(
        w6p0,
        [24, 0, 0, 28, 0, 0],
        "q=-Wp0 must come from the declared free rigid-body twist"
    );
    assert_eq!(
        matrix_times_vector(&CONTACT_JACOBIAN, &[-24, 0, 0, 0, 0, 4]),
        q6,
        "six times the free rigid-body twist must map to six times q"
    );

    // p(alpha)=p0+alpha*n. For alpha=a/d, P=d*p(alpha) is integral.
    // Check strict interior samples and the two closed-cone endpoints exactly.
    for (a, d) in [
        (-6_i64, 7_i64),
        (-1, 2),
        (0, 1),
        (1, 3),
        (6, 7),
        (-1, 1),
        (1, 1),
    ] {
        assert!(d > 0 && a.abs() <= d);
        let mut scaled_impulse = [0_i64; 6];
        for index in 0..6 {
            scaled_impulse[index] = d * BASE_IMPULSE[index] + a * NULL_IMPULSE[index];
        }

        // q + W p(alpha) = 0, represented without fractions as
        // d*(6q) + (6W)*(d*p(alpha)) = 0.
        let mut post_velocity_numerator = matrix_times_vector(&w6, &scaled_impulse);
        for index in 0..6 {
            post_velocity_numerator[index] += d * q6[index];
        }
        assert_zero(&post_velocity_numerator);

        // The generalized body impulse J^T p(alpha) is invariant because
        // J^T*n=0. The contact-coordinate multipliers nevertheless differ.
        let body_impulse_numerator = transpose_times_vector(&CONTACT_JACOBIAN, &scaled_impulse);
        let base_body_impulse = transpose_times_vector(&CONTACT_JACOBIAN, &BASE_IMPULSE);
        let expected_body_impulse: Vector6 =
            std::array::from_fn(|index| d * base_body_impulse[index]);
        assert_eq!(body_impulse_numerator, expected_body_impulse);

        assert!(scaled_impulse[0] > 0 && scaled_impulse[3] > 0);
        // mu=1/2. Multiply each disk slack by 4*d^2 to remain integral.
        let disk_one_slack =
            scaled_impulse[0].pow(2) - 4 * (scaled_impulse[1].pow(2) + scaled_impulse[2].pow(2));
        let disk_two_slack =
            scaled_impulse[3].pow(2) - 4 * (scaled_impulse[4].pow(2) + scaled_impulse[5].pow(2));
        assert!(disk_one_slack >= 0);
        assert!(disk_two_slack >= 0);
        if a.abs() < d {
            assert!(disk_one_slack > 0 && disk_two_slack > 0);
        } else {
            assert_eq!(disk_one_slack, 0);
            assert_eq!(disk_two_slack, 0);
        }

        // Delta K = p^T q + 1/2 p^T W p = -26/3 for every alpha.
        // With P=d*p and sixth-scaled W,q, multiply by 12*d^2:
        // 2*d*P^T*q6 + P^T*W6*P = -104*d^2.
        let energy_numerator =
            2 * d * dot(&scaled_impulse, &q6) + quadratic_form(&w6, &scaled_impulse);
        assert_eq!(energy_numerator, -104 * d.pow(2));

        // The base allocation is the unique minimum-Euclidean-norm point on
        // this affine fibre in the declared contact coordinates:
        // p0 is orthogonal to n and ||p(alpha)||^2-||p0||^2=2*alpha^2.
        let norm_increment =
            dot(&scaled_impulse, &scaled_impulse) - d.pow(2) * dot(&BASE_IMPULSE, &BASE_IMPULSE);
        assert_eq!(norm_increment, 2 * a.pow(2));
    }

    // Any |alpha|>1 violates both local Coulomb disks, even though the body
    // response remains unchanged under the nullspace shift.
    let outside = [2_i64, 0, 0, 2, 0, 0];
    let alpha_two = [outside[0], 2, 0, outside[3], -2, 0];
    let outside_disk_one = alpha_two[0].pow(2) - 4 * (alpha_two[1].pow(2) + alpha_two[2].pow(2));
    let outside_disk_two = alpha_two[3].pow(2) - 4 * (alpha_two[4].pow(2) + alpha_two[5].pow(2));
    assert!(outside_disk_one < 0 && outside_disk_two < 0);
}

#[test]
fn fixed_sliding_response_collapses_the_nullspace_fibre_to_one_admissible_point() {
    let w6 = geometry_delassus_sixths();
    let sliding_impulse = [2_i64, -1, 0, 2, -1, 0];
    let six_times_free_twist = [-24_i64, 18, 0, 0, 0, 4];
    let q6 = matrix_times_vector(&CONTACT_JACOBIAN, &six_times_free_twist);
    assert_eq!(q6, [-24, 18, 0, -28, 18, 0]);

    // The common post-contact velocity is u=(0,1,0,0,1,0): both contacts
    // slide in +tangent-1. With mu=1/2 and lambda_n=2, maximum dissipation
    // requires j_t=(-1,0) at each contact and tau=1.
    let w6p = matrix_times_vector(&w6, &sliding_impulse);
    let mut post_velocity_numerator = q6;
    for index in 0..6 {
        post_velocity_numerator[index] += w6p[index];
    }
    assert_eq!(post_velocity_numerator, [0, 6, 0, 0, 6, 0]);
    assert_zero(&matrix_times_vector(&w6, &NULL_IMPULSE));
    assert_zero(&transpose_times_vector(&CONTACT_JACOBIAN, &NULL_IMPULSE));

    // Along the algebraic response fibre p(alpha)=p0+alpha*n, with alpha=a/d,
    // contact-1 and contact-2 tangential impulses become (-d+a) and (-d-a).
    // Their disk slacks have opposite signs for every nonzero a:
    //   slack1 = 4*(2*a*d-a^2), slack2 = 4*(-2*a*d-a^2).
    // Hence the fixed sliding response has exactly one locally admissible
    // multiplier representative, alpha=0, despite dim ker(J^T)=1.
    for (a, d) in [(-3_i64, 4_i64), (-1, 4), (0, 1), (1, 4), (3, 4)] {
        let mut p = [0_i64; 6];
        for index in 0..6 {
            p[index] = d * sliding_impulse[index] + a * NULL_IMPULSE[index];
        }
        let slack_one = p[0].pow(2) - 4 * p[1].pow(2);
        let slack_two = p[3].pow(2) - 4 * p[4].pow(2);
        if a == 0 {
            assert_eq!(slack_one, 0);
            assert_eq!(slack_two, 0);
        } else {
            assert!(slack_one < 0 || slack_two < 0);
        }

        let body_impulse = transpose_times_vector(&CONTACT_JACOBIAN, &p);
        let sliding_body_impulse = transpose_times_vector(&CONTACT_JACOBIAN, &sliding_impulse);
        let expected_body_impulse: Vector6 =
            std::array::from_fn(|index| d * sliding_body_impulse[index]);
        assert_eq!(body_impulse, expected_body_impulse);
        let mut response = matrix_times_vector(&w6, &p);
        for index in 0..6 {
            response[index] += d * q6[index];
        }
        assert_eq!(response, [0, 6 * d, 0, 0, 6 * d, 0]);
    }
}
#[test]
fn proximal_cone_metric_lifts_rank_five_delassus_to_exact_spd() {
    let w6 = geometry_delassus_sixths();
    assert_eq!(principal_minor(&w6, [0, 2, 3, 4, 5]), 2592);

    // W_gamma = W + gamma^{-1} I. Since w6=6W, integer shifts
    // 3, 6 and 12 correspond to gamma=2, 1 and 1/2 respectively.
    // Sylvester's criterion is checked exactly on every leading principal
    // minor. This regularizes the inner cone QP only; physical W stays PSD.
    for shift in [3_i64, 6, 12] {
        let lifted = std::array::from_fn(|row| {
            std::array::from_fn(|column| w6[row][column] + if row == column { shift } else { 0 })
        });
        for size in 1..=6 {
            let determinant = leading_principal_minor(&lifted, size);
            assert!(
                determinant > 0,
                "W_gamma leading minor size={size}, shift={shift}: {determinant}"
            );
        }
    }
}
