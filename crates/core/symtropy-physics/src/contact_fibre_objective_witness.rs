// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Exact observability of redundant contact-impulse fibres.
//!
//! A contact impulse is not itself a unique physical observable when J^T has
//! a non-trivial kernel. This fixture distinguishes objectives on the physical
//! quotient from objectives that depend on a particular local allocation.
//! It is test-only and does not change production solver semantics.

type Jacobian = [[i128; 6]; 12];
type Gram = [[i128; 6]; 6];
type Impulses = [i128; 12];
type Objective = [i128; 12];

const CONTACT_POINTS: [[i128; 3]; 4] = [
    [1, 0, -1],
    [0, 1, -1],
    [1, 1, -1],
    [2, -1, -1],
];
const CONTACT_AXES: [[i128; 3]; 3] = [[0, 0, 1], [1, 0, 0], [0, 1, 0]];

// Each row z_k is one exact contact-impulse allocation direction in ker(J^T).
const NULL_BASIS: [Impulses; 6] = [
    [0, 1, -1, 0, -1, 1, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, -1, 0, 0, 1, 0, 0, 0, 0],
    [0, 0, -1, 0, 0, 0, 0, 0, 1, 0, 0, 0],
    [-2, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0],
    [0, -2, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0],
    [0, -1, -1, 0, 1, 0, 0, 0, 0, 0, 0, 1],
];

const BASE_IMPULSE: Impulses = [
    50, 0, 0, 50, 0, 0, 50, 0, 0, 50, 0, 0,
];

fn cross(left: [i128; 3], right: [i128; 3]) -> [i128; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn contact_jacobian() -> Jacobian {
    let mut jacobian = [[0_i128; 6]; 12];
    for (contact, point) in CONTACT_POINTS.iter().copied().enumerate() {
        for (axis_index, axis) in CONTACT_AXES.iter().copied().enumerate() {
            let moment = cross(point, axis);
            let row = contact * 3 + axis_index;
            jacobian[row] = [axis[0], axis[1], axis[2], moment[0], moment[1], moment[2]];
        }
    }
    jacobian
}

fn gram(jacobian: &Jacobian) -> Gram {
    let mut result = [[0_i128; 6]; 6];
    for row in jacobian {
        for (left, left_value) in row.iter().enumerate() {
            for (right, right_value) in row.iter().enumerate() {
                result[left][right] += left_value * right_value;
            }
        }
    }
    result
}

fn scatter(jacobian: &Jacobian, impulse: &Impulses) -> [i128; 6] {
    let mut result = [0_i128; 6];
    for (row, value) in jacobian.iter().zip(impulse) {
        for (axis, coefficient) in row.iter().enumerate() {
            result[axis] += coefficient * value;
        }
    }
    result
}

fn dot(left: &Objective, right: &Impulses) -> i128 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

/// Exact support-function interval of c^T(p0 + Z alpha), alpha in [-1,1]^6.
fn objective_range_on_fibre(
    objective: &Objective,
    base: &Impulses,
    basis: &[Impulses; 6],
) -> [i128; 2] {
    let center = dot(objective, base);
    let radius: i128 = basis
        .iter()
        .map(|direction| dot(objective, direction).abs())
        .sum();
    [center - radius, center + radius]
}

fn objective_is_fibre_invariant(
    objective: &Objective,
    basis: &[Impulses; 6],
) -> bool {
    basis
        .iter()
        .all(|direction| dot(objective, direction) == 0)
}

// Fraction-free Bareiss determinant. Every division must be exact.
#[allow(clippy::needless_range_loop)]
fn determinant_bareiss(mut matrix: Gram) -> i128 {
    let mut previous_pivot = 1_i128;
    let mut sign = 1_i128;
    for pivot_index in 0..5 {
        let pivot_row = (pivot_index..6)
            .find(|row| matrix[*row][pivot_index] != 0)
            .expect("the exact minor has a nonzero pivot");
        if pivot_row != pivot_index {
            matrix.swap(pivot_row, pivot_index);
            sign = -sign;
        }
        let pivot = matrix[pivot_index][pivot_index];
        for row in (pivot_index + 1)..6 {
            for column in (pivot_index + 1)..6 {
                let numerator = matrix[row][column] * pivot
                    - matrix[row][pivot_index] * matrix[pivot_index][column];
                assert_eq!(numerator % previous_pivot, 0);
                matrix[row][column] = numerator / previous_pivot;
            }
            matrix[row][pivot_index] = 0;
        }
        previous_pivot = pivot;
    }
    sign * matrix[5][5]
}

fn permute_impulse_blocks(impulse: &Impulses, order: [usize; 4]) -> Impulses {
    std::array::from_fn(|index| {
        let new_contact = index / 3;
        let local_axis = index % 3;
        impulse[order[new_contact] * 3 + local_axis]
    })
}

fn permute_jacobian_blocks(jacobian: &Jacobian, order: [usize; 4]) -> Jacobian {
    std::array::from_fn(|index| jacobian[order[index / 3] * 3 + index % 3])
}

#[cfg(test)]
#[allow(clippy::needless_range_loop)]
mod tests {
    use super::*;

    const KERNEL_MINOR: Gram = [
        [1, 0, 0, 0, -2, -1],
        [-1, 0, -1, 0, 0, -1],
        [0, 0, 0, 1, 0, 0],
        [-1, -1, 0, 0, 1, 1],
        [1, 0, 0, 0, 0, 0],
        [0, 1, 0, 0, 0, 0],
    ];

    fn build_candidate(corner: u8, base: Impulses, basis: &[Impulses; 6]) -> Impulses {
        let mut candidate = base;
        for (index, direction) in basis.iter().enumerate() {
            let sign = if corner & (1 << index) == 0 { -1 } else { 1 };
            for coordinate in 0..12 {
                candidate[coordinate] += sign * direction[coordinate];
            }
        }
        candidate
    }

    #[test]
    fn quotient_objectives_are_invariant_while_local_contact_loads_have_exact_ranges() {
        let jacobian = contact_jacobian();
        let gram = gram(&jacobian);

        // det(J^T J)=456 proves rank(J)=6. The six supplied null vectors
        // annihilate J^T, and their selected 6x6 minor has determinant one.
        assert_eq!(determinant_bareiss(gram), 456);
        assert_eq!(determinant_bareiss(KERNEL_MINOR), 1);
        for direction in NULL_BASIS {
            assert_eq!(scatter(&jacobian, &direction), [0_i128; 6]);
        }

        // The total normal impulse is the z-linear-momentum observable:
        // c = J e_z. It depends only on J^T p and is invariant on the fibre.
        let vertical_body_observable: Objective =
            std::array::from_fn(|row| jacobian[row][2]);
        assert_eq!(
            objective_range_on_fibre(&vertical_body_observable, &BASE_IMPULSE, &NULL_BASIS),
            [200, 200]
        );
        assert!(objective_is_fibre_invariant(
            &vertical_body_observable,
            &NULL_BASIS
        ));

        // The first contact's normal load is not a body-space observable.
        // Its exact range follows from one nonzero support coefficient:
        // e_0^T z_4=-2, hence 50 +/- 2.
        let mut first_contact_normal = [0_i128; 12];
        first_contact_normal[0] = 1;
        assert_eq!(
            NULL_BASIS.map(|direction| dot(&first_contact_normal, &direction)),
            [0, 0, 0, -2, 0, 0]
        );
        assert!(!objective_is_fibre_invariant(
            &first_contact_normal,
            &NULL_BASIS
        ));
        assert_eq!(
            objective_range_on_fibre(&first_contact_normal, &BASE_IMPULSE, &NULL_BASIS),
            [48, 52]
        );

        // Exhaust all 64 vertices and independently recover both objective
        // ranges. Also prove every equivalent allocation produces the same
        // generalized body impulse.
        let expected_body_impulse = scatter(&jacobian, &BASE_IMPULSE);
        assert_eq!(expected_body_impulse, [0, 0, 200, 50, -200, 0]);
        let mut observed_vertical = [i128::MAX, i128::MIN];
        let mut observed_first_normal = [i128::MAX, i128::MIN];
        for corner in 0_u8..64 {
            let candidate = build_candidate(corner, BASE_IMPULSE, &NULL_BASIS);
            assert_eq!(scatter(&jacobian, &candidate), expected_body_impulse);

            let vertical_value = dot(&vertical_body_observable, &candidate);
            observed_vertical[0] = observed_vertical[0].min(vertical_value);
            observed_vertical[1] = observed_vertical[1].max(vertical_value);

            let first_normal_value = dot(&first_contact_normal, &candidate);
            observed_first_normal[0] = observed_first_normal[0].min(first_normal_value);
            observed_first_normal[1] = observed_first_normal[1].max(first_normal_value);
        }
        assert_eq!(observed_vertical, [200, 200]);
        assert_eq!(observed_first_normal, [48, 52]);
    }

    #[test]
    fn objective_classification_and_ranges_follow_contact_identity_under_permutation() {
        let jacobian = contact_jacobian();
        // New order: old contacts [2, 1, 0, 3]. The old first contact is now
        // block 2, so its local normal objective is coordinate 6.
        let order = [2, 1, 0, 3];
        let permuted_jacobian = permute_jacobian_blocks(&jacobian, order);
        let permuted_base = permute_impulse_blocks(&BASE_IMPULSE, order);
        let permuted_basis =
            NULL_BASIS.map(|direction| permute_impulse_blocks(&direction, order));

        let vertical_body_observable: Objective =
            std::array::from_fn(|row| permuted_jacobian[row][2]);
        assert_eq!(
            objective_range_on_fibre(
                &vertical_body_observable,
                &permuted_base,
                &permuted_basis
            ),
            [200, 200]
        );
        assert!(objective_is_fibre_invariant(
            &vertical_body_observable,
            &permuted_basis
        ));

        let mut old_first_contact_normal = [0_i128; 12];
        old_first_contact_normal[6] = 1;
        assert_eq!(
            objective_range_on_fibre(
                &old_first_contact_normal,
                &permuted_base,
                &permuted_basis
            ),
            [48, 52]
        );

        // The reordered null vectors remain a complete kernel basis because
        // block permutation is invertible and the selected minor's rank is
        // unchanged. Recheck all 64 outputs rather than assuming labels.
        for direction in permuted_basis {
            assert_eq!(scatter(&permuted_jacobian, &direction), [0_i128; 6]);
        }
    }
}
