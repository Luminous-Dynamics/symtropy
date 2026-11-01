// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Exact body-space/contact-space equivalence witness for a shared rigid body.
//!
//! This test-only fixture demonstrates the graph distinction tracked by
//! PHYS-CONTACT-BODY-FACTORS-001 (#3277):
//! - three local point contacts on one rigid body;
//! - a dense 3x3 contact graph after eliminating the six body-velocity DOFs;
//! - a retained six-DOF body separator in the primal representation;
//! - exact agreement of W p and J (M^-1 (J^T p));
//! - a three-dimensional contact-impulse fibre with one generalized response.
//!
//! The fixture certifies contact kinematics and the declared spatial inertia.
//! It is not yet a collision-detection, surface-normal generation, Coulomb
//! branch-enumeration, or physical time-step completeness certificate.

type Jacobian = [[i128; 6]; 9];
type Delassus = [[i128; 9]; 9];
type Gram = [[i128; 6]; 6];

const CONTACT_POINTS_IN_BODY: [[i128; 3]; 3] = [[1, 0, -1], [0, 1, -1], [1, 1, -1]];

// Each local frame is orthonormal: normal +z, tangent axes +x and +y.
const CONTACT_FRAME_AXES: [[i128; 3]; 3] = [[0, 0, 1], [1, 0, 0], [0, 1, 0]];

// Spatial twist ordering is [v_x, v_y, v_z, omega_x, omega_y, omega_z].
// M = I_6 is a valid positive-definite declared mass/inertia matrix for this
// algebraic contact-kinematics fixture, so applying M^-1 is exact identity.

fn cross(left: [i128; 3], right: [i128; 3]) -> [i128; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn contact_jacobian() -> Jacobian {
    let mut jacobian = [[0_i128; 6]; 9];
    for (contact, point) in CONTACT_POINTS_IN_BODY.iter().copied().enumerate() {
        for (axis_index, axis) in CONTACT_FRAME_AXES.iter().copied().enumerate() {
            let moment = cross(point, axis);
            let row = contact * 3 + axis_index;
            jacobian[row] = [axis[0], axis[1], axis[2], moment[0], moment[1], moment[2]];
        }
    }
    jacobian
}

fn assemble_delassus(jacobian: &Jacobian) -> Delassus {
    let mut delassus = [[0_i128; 9]; 9];
    for (row, row_values) in jacobian.iter().enumerate() {
        for (column, column_values) in jacobian.iter().enumerate() {
            delassus[row][column] = row_values
                .iter()
                .zip(column_values.iter())
                .map(|(left, right)| left * right)
                .sum();
        }
    }
    delassus
}

fn scatter_contact_impulses(jacobian: &Jacobian, impulses: &[i128; 9]) -> [i128; 6] {
    let mut body_wrench = [0_i128; 6];
    for (row, impulse) in jacobian.iter().zip(impulses.iter()) {
        for (axis, coefficient) in row.iter().enumerate() {
            body_wrench[axis] += coefficient * impulse;
        }
    }
    body_wrench
}

fn gather_body_velocity(jacobian: &Jacobian, body_velocity: &[i128; 6]) -> [i128; 9] {
    let mut contact_velocity = [0_i128; 9];
    for (contact, row) in jacobian.iter().enumerate() {
        contact_velocity[contact] = row
            .iter()
            .zip(body_velocity.iter())
            .map(|(coefficient, velocity)| coefficient * velocity)
            .sum();
    }
    contact_velocity
}

fn apply_delassus(delassus: &Delassus, impulses: &[i128; 9]) -> [i128; 9] {
    let mut velocity = [0_i128; 9];
    for (row_index, row) in delassus.iter().enumerate() {
        velocity[row_index] = row
            .iter()
            .zip(impulses.iter())
            .map(|(coefficient, impulse)| coefficient * impulse)
            .sum();
    }
    velocity
}

fn body_gram(jacobian: &Jacobian) -> Gram {
    let mut gram = [[0_i128; 6]; 6];
    for row in jacobian {
        for (left_axis, left_value) in row.iter().enumerate() {
            for (right_axis, right_value) in row.iter().enumerate() {
                gram[left_axis][right_axis] += left_value * right_value;
            }
        }
    }
    gram
}

// Bareiss fraction-free elimination. Every division is asserted exact, so the
// resulting determinant is an integer identity rather than a floating rank
// estimate. Index-coupled loops are intentional for the fixed-size pivoting.
#[allow(clippy::needless_range_loop)]
fn determinant_bareiss(mut matrix: Gram) -> i128 {
    let mut previous_pivot = 1_i128;
    let mut sign = 1_i128;
    for pivot_index in 0..5 {
        let pivot_row = (pivot_index..6)
            .find(|row| matrix[*row][pivot_index] != 0)
            .expect("the Gram matrix has a nonzero pivot");
        if pivot_row != pivot_index {
            matrix.swap(pivot_row, pivot_index);
            sign = -sign;
        }
        let pivot = matrix[pivot_index][pivot_index];
        for row in (pivot_index + 1)..6 {
            for column in (pivot_index + 1)..6 {
                let numerator = matrix[row][column] * pivot
                    - matrix[row][pivot_index] * matrix[pivot_index][column];
                assert_eq!(
                    numerator % previous_pivot,
                    0,
                    "Bareiss division must be exact"
                );
                matrix[row][column] = numerator / previous_pivot;
            }
            matrix[row][pivot_index] = 0;
        }
        previous_pivot = pivot;
    }
    sign * matrix[5][5]
}

#[cfg(test)]
#[allow(clippy::needless_range_loop)]
mod tests {
    use super::*;

    const NULL_FIBRE_BASIS: [[i128; 9]; 3] = [
        [0, 1, -1, 0, -1, 1, 0, 0, 0],
        [0, 0, 0, 0, -1, 0, 0, 1, 0],
        [0, 0, -1, 0, 0, 0, 0, 0, 1],
    ];

    #[test]
    fn one_body_contacts_form_dense_delassus_clique_but_retain_six_dof_separator() {
        let jacobian = contact_jacobian();
        let delassus = assemble_delassus(&jacobian);

        // The declared contact frames are orthonormal.
        for (left_index, left_axis) in CONTACT_FRAME_AXES.iter().enumerate() {
            for (right_index, right_axis) in CONTACT_FRAME_AXES.iter().enumerate() {
                let dot: i128 = left_axis
                    .iter()
                    .zip(right_axis.iter())
                    .map(|(left, right)| left * right)
                    .sum();
                assert_eq!(dot, if left_index == right_index { 1 } else { 0 });
            }
        }

        // W = J J^T is exactly symmetric and PSD because M^-1 = I_6.
        for (row_index, row) in delassus.iter().enumerate() {
            for (column_index, value) in row.iter().enumerate() {
                assert_eq!(value, &delassus[column_index][row_index]);
            }
        }

        // Every off-diagonal 3x3 contact block is nonzero: the contact-space
        // graph is a three-vertex clique, despite only six latent body DOFs.
        for left_contact in 0..3 {
            for right_contact in (left_contact + 1)..3 {
                let block_frobenius_squared: i128 = (0..3)
                    .flat_map(|left_axis| {
                        (0..3).map(move |right_axis| {
                            delassus[left_contact * 3 + left_axis][right_contact * 3 + right_axis]
                                .pow(2)
                        })
                    })
                    .sum();
                assert!(
                    block_frobenius_squared > 0,
                    "every pair of contacts must be coupled through the shared body"
                );
            }
        }

        // J has six independent columns iff J^T J is positive definite.
        // Its Gram matrix is PSD by construction and its exact determinant is
        // 12, so rank(J)=rank(W)=6 and the nine-coordinate W has nullity 3.
        let gram = body_gram(&jacobian);
        assert_eq!(determinant_bareiss(gram), 12);

        // Three exact null-fibre directions preserve the generalized body
        // wrench. Their 3x3 coordinate minor is the identity, so they are
        // linearly independent. Together with rank(J)=6, they span the full
        // three-dimensional kernel of J^T and W.
        assert_eq!(
            [
                [
                    NULL_FIBRE_BASIS[0][1],
                    NULL_FIBRE_BASIS[1][1],
                    NULL_FIBRE_BASIS[2][1],
                ],
                [
                    NULL_FIBRE_BASIS[0][7],
                    NULL_FIBRE_BASIS[1][7],
                    NULL_FIBRE_BASIS[2][7],
                ],
                [
                    NULL_FIBRE_BASIS[0][8],
                    NULL_FIBRE_BASIS[1][8],
                    NULL_FIBRE_BASIS[2][8],
                ],
            ],
            [[1, 0, 0], [0, 1, 0], [0, 0, 1]]
        );
        for null_direction in NULL_FIBRE_BASIS {
            assert_eq!(
                scatter_contact_impulses(&jacobian, &null_direction),
                [0_i128; 6]
            );
            assert_eq!(apply_delassus(&delassus, &null_direction), [0_i128; 9]);
        }

        let impulses = [2_i128, -3, 5, 7, -11, 13, -17, 19, -23];
        let body_wrench = scatter_contact_impulses(&jacobian, &impulses);
        // M^-1 is the identity in this fixture: this is the retained
        // six-dimensional body-velocity separator.
        let body_velocity = body_wrench;
        let matrix_free_velocity = gather_body_velocity(&jacobian, &body_velocity);
        let contact_space_velocity = apply_delassus(&delassus, &impulses);
        assert_eq!(matrix_free_velocity, contact_space_velocity);

        // Verify the operator identity column-by-column, not just for one
        // selected impulse vector.
        for basis_index in 0..9 {
            let mut basis = [0_i128; 9];
            basis[basis_index] = 1;
            assert_eq!(
                gather_body_velocity(&jacobian, &scatter_contact_impulses(&jacobian, &basis)),
                apply_delassus(&delassus, &basis)
            );
        }
    }

    #[test]
    fn sticking_law_has_a_three_dimensional_admissible_impulse_fibre() {
        let jacobian = contact_jacobian();
        let delassus = assemble_delassus(&jacobian);

        // Start from a strictly sticking response with positive normal loads.
        // q is chosen as -W p0, so u=q+Wp0 is exactly zero at every contact.
        // With mu=1 and zero tangential impulse, all three contacts are
        // strictly inside their Coulomb disks.
        let p0 = [20_i128, 0, 0, 30, 0, 0, 40, 0, 0];
        let response0 = apply_delassus(&delassus, &p0);
        let body_impulse0 = scatter_contact_impulses(&jacobian, &p0);
        let body_free_velocity = body_impulse0.map(|value| -value);
        let free_velocity = gather_body_velocity(&jacobian, &body_free_velocity);
        assert_eq!(free_velocity, response0.map(|value| -value));

        // M=I_6: the impulse update takes the body from v_free to v_post=0.
        // This realizes q=J v_free rather than treating q as an independent
        // contact-space vector.
        let body_post_velocity0: [i128; 6] =
            std::array::from_fn(|index| body_free_velocity[index] + body_impulse0[index]);
        assert_eq!(body_post_velocity0, [0_i128; 6]);

        let twice_energy0: i128 = p0
            .iter()
            .zip(free_velocity.iter())
            .map(|(impulse, free)| 2 * impulse * free)
            .sum::<i128>()
            + p0.iter()
                .zip(response0.iter())
                .map(|(impulse, response)| impulse * response)
                .sum::<i128>();
        assert!(twice_energy0 < 0);

        // The three kernel coordinates span a 3D affine family p0 + Z alpha.
        // Every vertex of [-1,1]^3 is checked exactly. Since each local
        // Coulomb disk is convex and all vertices are strictly interior,
        // the whole parameter cube is an admissible sticking fibre.
        for corner in 0_u8..8 {
            let mut candidate = p0;
            for (basis_index, basis_vector) in NULL_FIBRE_BASIS.iter().enumerate() {
                let coefficient = if corner & (1 << basis_index) == 0 {
                    -1_i128
                } else {
                    1_i128
                };
                for (impulse, direction) in candidate.iter_mut().zip(basis_vector.iter()) {
                    *impulse += coefficient * direction;
                }
            }

            let body_impulse = scatter_contact_impulses(&jacobian, &candidate);
            assert_eq!(
                body_impulse, body_impulse0,
                "generalized body impulse must be fibre-invariant"
            );
            let body_post_velocity: [i128; 6] =
                std::array::from_fn(|index| body_free_velocity[index] + body_impulse[index]);
            assert_eq!(body_post_velocity, [0_i128; 6]);

            let response = apply_delassus(&delassus, &candidate);
            assert_eq!(
                response, response0,
                "contact response must be fibre-invariant"
            );
            let post_contact_velocity: [i128; 9] =
                std::array::from_fn(|index| free_velocity[index] + response[index]);
            assert_eq!(post_contact_velocity, [0_i128; 9]);

            for contact in 0..3 {
                let offset = contact * 3;
                let normal_impulse = candidate[offset];
                let tangent_1 = candidate[offset + 1];
                let tangent_2 = candidate[offset + 2];
                assert!(normal_impulse > 0, "each contact remains loaded");
                assert!(
                    tangent_1 * tangent_1 + tangent_2 * tangent_2 < normal_impulse * normal_impulse,
                    "sticking impulse must remain strictly inside its mu=1 Coulomb disk"
                );
            }

            let twice_energy: i128 = candidate
                .iter()
                .zip(free_velocity.iter())
                .map(|(impulse, free)| 2 * impulse * free)
                .sum::<i128>()
                + candidate
                    .iter()
                    .zip(response.iter())
                    .map(|(impulse, post)| impulse * post)
                    .sum::<i128>();
            assert_eq!(
                twice_energy, twice_energy0,
                "all points in the multiplier fibre must share the same kinetic-energy change"
            );
            assert!(twice_energy < 0);
        }
    }

    #[test]
    fn exact_geometry_fixture_has_expected_rank_and_contact_coupling() {
        let jacobian = contact_jacobian();
        let delassus = assemble_delassus(&jacobian);
        assert_eq!(
            body_gram(&jacobian),
            [
                [3, 0, 0, 0, -3, -2],
                [0, 3, 0, 3, 0, 2],
                [0, 0, 3, 2, -2, 0],
                [0, 3, 2, 5, -1, 2],
                [-3, 0, -2, -1, 5, 2],
                [-2, 2, 0, 2, 2, 4],
            ]
        );
        assert_eq!(determinant_bareiss(body_gram(&jacobian)), 12);
        assert_eq!(delassus.len(), 9);
    }
}
