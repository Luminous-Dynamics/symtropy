// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Exact redundant-contact witness for semidefinite Delassus operators.
//!
//! Two duplicate 3D contact Jacobians produce a rank-three 6x6 Delassus
//! matrix. A continuum of nonnegative frictionless normal-impulse allocations
//! gives the same generalized body impulse. Nullspace shifts in tangent
//! coordinates, however, can preserve the body impulse while violating the
//! contact-local friction law. This module is a test-only exact integer oracle.

type Matrix6 = [[i64; 6]; 6];
type Vector6 = [i64; 6];
type Jacobian6x3 = [[i64; 3]; 6];

const IMPULSE_DENOMINATOR: i64 = 4;

/// J has two identical copies of the identity contact frame:
/// J = [ I ; I ]. The generalized inverse mass is I_3.
fn duplicate_contact_jacobian() -> Jacobian6x3 {
    [
        [1, 0, 0],
        [0, 1, 0],
        [0, 0, 1],
        [1, 0, 0],
        [0, 1, 0],
        [0, 0, 1],
    ]
}

/// Construct W = J J^T using integer arithmetic only.
fn assemble_delassus(j: &Jacobian6x3) -> Matrix6 {
    let mut w = [[0_i64; 6]; 6];
    for (row_index, row) in j.iter().enumerate() {
        for (column_index, column) in j.iter().enumerate() {
            w[row_index][column_index] = row
                .iter()
                .zip(column)
                .map(|(left, right)| left * right)
                .sum();
        }
    }
    w
}

fn multiply_matrix_vector(matrix: &Matrix6, vector: &Vector6) -> Vector6 {
    core::array::from_fn(|row| {
        matrix[row]
            .iter()
            .zip(vector)
            .map(|(entry, value)| entry * value)
            .sum()
    })
}

/// Compute J^T p without division. The caller chooses a common denominator.
fn generalized_impulse_numerator(j: &Jacobian6x3, p: &Vector6) -> [i64; 3] {
    core::array::from_fn(|dof| {
        j.iter()
            .zip(p)
            .map(|(row, value)| row[dof] * value)
            .sum()
    })
}

/// Scaled numerator for p(t) = (t,0,0,1-t,0,0), t = k/4.
fn normal_allocation_numerator(k: i64) -> Vector6 {
    [k, 0, 0, IMPULSE_DENOMINATOR - k, 0, 0]
}

fn is_frictionless_contact_admissible(p: &Vector6, normal_index: usize) -> bool {
    p[normal_index] >= 0
        && p[normal_index + 1] == 0
        && p[normal_index + 2] == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    const FREE_VELOCITY_NUMERATOR: Vector6 = [-4, 0, 0, -4, 0, 0];

    #[test]
    fn duplicate_jacobian_produces_exact_psd_rank_three_delassus_matrix() {
        let j = duplicate_contact_jacobian();
        let w = assemble_delassus(&j);

        // The last three Jacobian rows duplicate the first three, while the
        // leading 3x3 minor is exactly I. Therefore rank(J) = rank(W) = 3.
        assert_eq!([j[3], j[4], j[5]], [j[0], j[1], j[2]]);
        assert_eq!([j[0], j[1], j[2]], [[1, 0, 0], [0, 1, 0], [0, 0, 1]]);
        assert_eq!(w[0][0..3], [1, 0, 0]);
        assert_eq!(w[1][0..3], [0, 1, 0]);
        assert_eq!(w[2][0..3], [0, 0, 1]);
        assert_eq!(w[3], w[0]);
        assert_eq!(w[4], w[1]);
        assert_eq!(w[5], w[2]);

        // W = J J^T, so x^T W x = ||J^T x||^2 >= 0. This exact integer
        // panel checks the Gram identity for varied signed test vectors.
        for x in [
            [0, 0, 0, 0, 0, 0],
            [1, -2, 3, 4, 5, -6],
            [-7, 3, 1, 2, -4, 8],
            [11, 0, -5, -11, 2, 5],
        ] {
            let wx = multiply_matrix_vector(&w, &x);
            let quadratic = x.iter().zip(wx).map(|(a, b)| a * b).sum::<i64>();
            let gram_square = (x[0] + x[3]).pow(2)
                + (x[1] + x[4]).pow(2)
                + (x[2] + x[5]).pow(2);
            assert_eq!(quadratic, gram_square);
            assert!(quadratic >= 0);
        }
    }

    #[test]
    fn duplicate_contact_nullspace_preserves_generalized_impulse_and_velocity() {
        let j = duplicate_contact_jacobian();
        let w = assemble_delassus(&j);
        let normal_null = [1, 0, 0, -1, 0, 0];
        let tangent_null = [0, 1, 0, 0, -1, 0];

        assert_eq!(multiply_matrix_vector(&w, &normal_null), [0; 6]);
        assert_eq!(multiply_matrix_vector(&w, &tangent_null), [0; 6]);
        assert_eq!(generalized_impulse_numerator(&j, &normal_null), [0; 3]);
        assert_eq!(generalized_impulse_numerator(&j, &tangent_null), [0; 3]);
    }

    #[test]
    fn continuum_of_frictionless_multiplier_allocations_has_one_body_impulse() {
        let j = duplicate_contact_jacobian();
        let w = assemble_delassus(&j);

        // q = J v_free with v_free = (-1,0,0). Both contact points therefore
        // have the same negative free normal velocity and zero tangent velocity.
        let mut observed_impulses = Vec::new();
        for k in 0..=IMPULSE_DENOMINATOR {
            let p = normal_allocation_numerator(k);
            assert!(is_frictionless_contact_admissible(&p, 0));
            assert!(is_frictionless_contact_admissible(&p, 3));

            // All quantities use the common denominator four.
            let wp = multiply_matrix_vector(&w, &p);
            let post_velocity_numerator = core::array::from_fn::<_, 6, _>(|index| {
                FREE_VELOCITY_NUMERATOR[index] + wp[index]
            });
            assert_eq!(post_velocity_numerator, [0; 6]);

            // The two local normal impulses are nonnegative and both normal
            // complementarity products vanish because u_n = 0.
            assert!(p[0] >= 0 && p[3] >= 0);
            assert_eq!(p[0] * post_velocity_numerator[0], 0);
            assert_eq!(p[3] * post_velocity_numerator[3], 0);

            // mu = 0 makes each local Coulomb disk a singleton {j_t = 0}.
            assert_eq!([p[1], p[2], p[4], p[5]], [0; 4]);

            // J^T p is (1,0,0) for every allocation, despite different p.
            let body_impulse = generalized_impulse_numerator(&j, &p);
            assert_eq!(body_impulse, [IMPULSE_DENOMINATOR, 0, 0]);
            observed_impulses.push(p);
        }

        assert_eq!(observed_impulses.len(), 5);
        assert_ne!(observed_impulses[0], observed_impulses[4]);

        // Exact island energy: p^T q = -1 and p^T W p = +1.
        // Thus Delta K = -1 + 1/2 = -1/2, for every allocation.
        for p in observed_impulses {
            let wp = multiply_matrix_vector(&w, &p);
            let linear_work_numerator = p
                .iter()
                .zip(FREE_VELOCITY_NUMERATOR)
                .map(|(left, right)| left * right)
                .sum::<i64>();
            let quadratic_work_numerator =
                p.iter().zip(wp).map(|(left, right)| left * right).sum::<i64>();
            assert_eq!(linear_work_numerator, -16);
            assert_eq!(quadratic_work_numerator, 16);
            assert_eq!(
                2 * linear_work_numerator + quadratic_work_numerator,
                -16
            );
        }
    }

    #[test]
    fn body_impulse_equivalence_does_not_authorize_local_friction_quotienting() {
        let j = duplicate_contact_jacobian();
        let w = assemble_delassus(&j);
        let admissible = normal_allocation_numerator(2);
        let tangent_null = [0, 1, 0, 0, -1, 0];
        let shifted = core::array::from_fn(|index| admissible[index] + tangent_null[index]);

        // The nullspace shift leaves both Wp and J^T p exactly unchanged.
        assert_eq!(
            multiply_matrix_vector(&w, &admissible),
            multiply_matrix_vector(&w, &shifted)
        );
        assert_eq!(
            generalized_impulse_numerator(&j, &admissible),
            generalized_impulse_numerator(&j, &shifted)
        );

        // But for mu = 0 the shifted tangent impulses violate both local
        // contact disks. Physical-outcome equivalence is not local-law equivalence.
        assert!(is_frictionless_contact_admissible(&admissible, 0));
        assert!(is_frictionless_contact_admissible(&admissible, 3));
        assert!(!is_frictionless_contact_admissible(&shifted, 0));
        assert!(!is_frictionless_contact_admissible(&shifted, 3));
    }

    #[test]
    fn normal_nullspace_allocations_outside_the_nonnegative_segment_are_rejected() {
        let j = duplicate_contact_jacobian();
        let w = assemble_delassus(&j);
        let below = normal_allocation_numerator(-1);
        let above = normal_allocation_numerator(IMPULSE_DENOMINATOR + 1);

        assert_eq!(multiply_matrix_vector(&w, &below), [4, 0, 0, 4, 0, 0]);
        assert_eq!(multiply_matrix_vector(&w, &above), [4, 0, 0, 4, 0, 0]);
        assert!(!is_frictionless_contact_admissible(&below, 0));
        assert!(!is_frictionless_contact_admissible(&above, 3));
    }
}
