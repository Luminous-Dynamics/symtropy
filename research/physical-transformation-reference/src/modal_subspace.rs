// SPDX-License-Identifier: AGPL-3.0-or-later
//! Basis-invariant comparison of clustered / repeated structural modes.
//!
//! A scalar vector MAC can be low when two solvers choose different
//! eigenvectors inside the same repeated-eigenvalue eigenspace. Compare the
//! span of each cluster using mass-weighted principal angles instead. This
//! module does not decide which frequencies belong to a cluster; callers must
//! match a frequency cluster and its multiplicity before comparing subspaces.

use crate::Check;
use crate::modal_bar::{Matrix, generalized_symmetric_eigenpairs};

/// Basis-invariant agreement of two equal-dimensional modal subspaces.
///
/// Principal MAC values are the squared cosines of the mass-weighted
/// principal angles, sorted from worst to best and bounded by [0, 1].
/// The worst value is the strictest direction-specific comparison; the mean
/// summarizes the cluster but must not replace the worst value when a full
/// subspace match is required.
#[derive(Clone, Debug, PartialEq)]
pub struct ModalSubspaceComparison {
    pub mode_count: usize,
    pub principal_mac_values: Vec<f64>,
    pub mean_principal_mac: f64,
    pub worst_principal_mac: f64,
}

/// Compare two modal subspaces under the same symmetric positive-definite
/// mass inner product.
///
/// Each input is a list of mode vectors, one vector per mode. The two lists
/// must have the same non-zero count and every vector must cover every DOF in
/// mass. Their individual scaling and their choice of basis within a span do
/// not affect the result. The Gram matrices are whitened independently, then
/// eigenvalues of C-transpose-times-C yield squared cosines of principal angles.
///
/// Singular or badly conditioned mode bases are rejected rather than assigned
/// an optimistic score. Frequency pairing / cluster identity is a separate
/// input responsibility and cannot be inferred from vectors alone.
pub fn mass_weighted_modal_subspace_comparison(
    left_modes: &[Vec<f64>],
    right_modes: &[Vec<f64>],
    mass: &Matrix,
) -> Check<ModalSubspaceComparison> {
    if left_modes.is_empty() || left_modes.len() != right_modes.len() {
        return Err("modal subspaces must have the same non-zero dimension".to_owned());
    }
    let dof_count = mass.len();
    if dof_count == 0 || mass.iter().any(|row| row.len() != dof_count) {
        return Err("mass matrix must be non-empty and square".to_owned());
    }
    if left_modes.len() > dof_count {
        return Err("modal subspace dimension cannot exceed its DOF count".to_owned());
    }
    if left_modes
        .iter()
        .chain(right_modes)
        .any(|mode| mode.len() != dof_count)
    {
        return Err("all modal vectors must match the mass-matrix dimension".to_owned());
    }
    if mass.iter().flatten().any(|value| !value.is_finite())
        || left_modes
            .iter()
            .chain(right_modes)
            .flatten()
            .any(|value| !value.is_finite())
    {
        return Err("modal subspace input contains a non-finite value".to_owned());
    }
    validate_symmetric_mass(mass)?;

    // Scaling each basis vector independently preserves its span while
    // limiting overflow/underflow before mass-weighted Gram products.
    let left = scaled_basis(left_modes)?;
    let right = scaled_basis(right_modes)?;
    let left_gram = mass_gram(&left, &left, mass)?;
    let right_gram = mass_gram(&right, &right, mass)?;
    let cross_gram = mass_gram(&left, &right, mass)?;

    let left_inverse_sqrt = inverse_sqrt_positive_definite(&left_gram)?;
    let right_inverse_sqrt = inverse_sqrt_positive_definite(&right_gram)?;
    let normalized_cross = multiply(
        &multiply(&left_inverse_sqrt, &cross_gram),
        &right_inverse_sqrt,
    );
    let mut principal_mac_matrix = multiply(&transpose(&normalized_cross), &normalized_cross);
    symmetrize_in_place(&mut principal_mac_matrix);

    let identity = identity_matrix(left_modes.len());
    let eigenpairs = generalized_symmetric_eigenpairs(&principal_mac_matrix, &identity)?;
    let mut values = Vec::with_capacity(eigenpairs.len());
    for (eigenvalue, _) in eigenpairs {
        if !eigenvalue.is_finite() || eigenvalue < -1.0e-8 || eigenvalue > 1.0 + 1.0e-8 {
            return Err(format!(
                "principal MAC eigenvalue outside numerical bounds: {eigenvalue:.17e}"
            ));
        }
        values.push(eigenvalue.clamp(0.0, 1.0));
    }

    let worst = *values
        .first()
        .ok_or_else(|| "principal MAC solve returned no values".to_owned())?;
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    if !mean.is_finite() {
        return Err("mean principal MAC is non-finite".to_owned());
    }

    Ok(ModalSubspaceComparison {
        mode_count: values.len(),
        principal_mac_values: values,
        mean_principal_mac: mean,
        worst_principal_mac: worst,
    })
}

fn validate_symmetric_mass(mass: &Matrix) -> Check<()> {
    let scale = mass
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max)
        .max(f64::MIN_POSITIVE);
    for row in 0..mass.len() {
        for col in 0..mass.len() {
            if (mass[row][col] - mass[col][row]).abs() > 1.0e-12 * scale {
                return Err(format!("mass matrix is not symmetric at ({row}, {col})"));
            }
        }
    }
    Ok(())
}

fn scaled_basis(modes: &[Vec<f64>]) -> Check<Matrix> {
    let mut basis = Vec::with_capacity(modes.len());
    for mode in modes {
        let scale = mode.iter().map(|value| value.abs()).fold(0.0_f64, f64::max);
        if !scale.is_finite() || scale <= 0.0 {
            return Err("modal basis contains a zero or invalid vector".to_owned());
        }
        basis.push(mode.iter().map(|value| value / scale).collect::<Vec<_>>());
    }
    Ok(basis)
}

fn mass_gram(left: &Matrix, right: &Matrix, mass: &Matrix) -> Check<Matrix> {
    let mut result = vec![vec![0.0; right.len()]; left.len()];
    for (right_index, right_mode) in right.iter().enumerate() {
        let mass_right = matrix_vector_product(mass, right_mode)?;
        for (left_index, left_mode) in left.iter().enumerate() {
            let value = left_mode
                .iter()
                .zip(&mass_right)
                .map(|(a, b)| a * b)
                .sum::<f64>();
            if !value.is_finite() {
                return Err("mass-weighted modal Gram matrix is non-finite".to_owned());
            }
            result[left_index][right_index] = value;
        }
    }
    Ok(result)
}

fn inverse_sqrt_positive_definite(matrix: &Matrix) -> Check<Matrix> {
    if matrix.is_empty() || matrix.iter().any(|row| row.len() != matrix.len()) {
        return Err("modal Gram matrix must be non-empty and square".to_owned());
    }
    let identity = identity_matrix(matrix.len());
    let eigenpairs = generalized_symmetric_eigenpairs(matrix, &identity)?;
    let largest = eigenpairs
        .iter()
        .map(|(eigenvalue, _)| *eigenvalue)
        .fold(0.0_f64, f64::max);
    if !largest.is_finite() || largest <= 0.0 {
        return Err("modal basis Gram matrix is not positive definite".to_owned());
    }

    let minimum = eigenpairs
        .iter()
        .map(|(eigenvalue, _)| *eigenvalue)
        .fold(f64::INFINITY, f64::min);
    if !minimum.is_finite() || minimum <= largest * 1.0e-12 {
        return Err("modal basis is rank deficient or too ill-conditioned".to_owned());
    }

    let size = matrix.len();
    let mut inverse_sqrt = vec![vec![0.0; size]; size];
    for (eigenvalue, eigenvector) in eigenpairs {
        let weight = 1.0 / eigenvalue.sqrt();
        for row in 0..size {
            for col in 0..size {
                inverse_sqrt[row][col] += weight * eigenvector[row] * eigenvector[col];
            }
        }
    }
    if inverse_sqrt.iter().flatten().all(|value| value.is_finite()) {
        Ok(inverse_sqrt)
    } else {
        Err("inverse square-root of modal Gram matrix is non-finite".to_owned())
    }
}

fn matrix_vector_product(matrix: &Matrix, vector: &[f64]) -> Check<Vec<f64>> {
    if matrix.len() != vector.len() || matrix.iter().any(|row| row.len() != vector.len()) {
        return Err("matrix/vector dimensions do not agree".to_owned());
    }
    let output = matrix
        .iter()
        .map(|row| row.iter().zip(vector).map(|(a, b)| a * b).sum::<f64>())
        .collect::<Vec<_>>();
    if output.iter().all(|value| value.is_finite()) {
        Ok(output)
    } else {
        Err("matrix/vector product is non-finite".to_owned())
    }
}

fn identity_matrix(size: usize) -> Matrix {
    let mut identity = vec![vec![0.0; size]; size];
    for index in 0..size {
        identity[index][index] = 1.0;
    }
    identity
}

fn transpose(matrix: &Matrix) -> Matrix {
    let mut output = vec![vec![0.0; matrix.len()]; matrix.len()];
    for row in 0..matrix.len() {
        for col in 0..matrix.len() {
            output[col][row] = matrix[row][col];
        }
    }
    output
}

fn multiply(left: &Matrix, right: &Matrix) -> Matrix {
    let mut output = vec![vec![0.0; right.len()]; left.len()];
    for row in 0..left.len() {
        for inner in 0..right.len() {
            for col in 0..right.len() {
                output[row][col] += left[row][inner] * right[inner][col];
            }
        }
    }
    output
}

fn symmetrize_in_place(matrix: &mut Matrix) {
    for row in 0..matrix.len() {
        for col in (row + 1)..matrix.len() {
            let average = 0.5 * (matrix[row][col] + matrix[col][row]);
            matrix[row][col] = average;
            matrix[col][row] = average;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mass_weighted_modal_subspace_comparison;
    use crate::beam_modal::modal_assurance_criterion;

    #[test]
    fn repeated_mode_cluster_is_invariant_to_basis_rotation_and_scaling() {
        let mass = vec![
            vec![2.0, 0.0, 0.0, 0.0],
            vec![0.0, 3.0, 0.0, 0.0],
            vec![0.0, 0.0, 5.0, 0.0],
            vec![0.0, 0.0, 0.0, 7.0],
        ];
        let left = vec![
            vec![1.0 / 2.0_f64.sqrt(), 0.0, 0.0, 0.0],
            vec![0.0, 1.0 / 3.0_f64.sqrt(), 0.0, 0.0],
        ];
        let angle: f64 = std::f64::consts::FRAC_PI_4;
        let right = vec![
            vec![
                10.0 * angle.cos() / 2.0_f64.sqrt(),
                10.0 * angle.sin() / 3.0_f64.sqrt(),
                0.0,
                0.0,
            ],
            vec![
                -0.25 * angle.sin() / 2.0_f64.sqrt(),
                0.25 * angle.cos() / 3.0_f64.sqrt(),
                0.0,
                0.0,
            ],
        ];

        // Individual vectors look different under the arbitrary basis choice.
        let vector_mac = modal_assurance_criterion(&left[0], &right[0]).expect("vector MAC");
        assert!(vector_mac < 0.7);

        let comparison =
            mass_weighted_modal_subspace_comparison(&left, &right, &mass).expect("same subspace");
        assert_eq!(comparison.mode_count, 2);
        assert!(
            comparison
                .principal_mac_values
                .iter()
                .all(|value| (value - 1.0).abs() < 1.0e-10)
        );
        assert!((comparison.mean_principal_mac - 1.0).abs() < 1.0e-10);
        assert!((comparison.worst_principal_mac - 1.0).abs() < 1.0e-10);
    }

    #[test]
    fn principal_angles_detect_a_changed_subspace_not_just_frequency_agreement() {
        let mass = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        let left = vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]];
        let right = vec![vec![1.0, 0.0, 0.0], vec![0.0, 0.0, 1.0]];
        let comparison =
            mass_weighted_modal_subspace_comparison(&left, &right, &mass).expect("subspace angle");
        assert!((comparison.worst_principal_mac).abs() < 1.0e-12);
        assert!((comparison.mean_principal_mac - 0.5).abs() < 1.0e-12);
        assert!((comparison.principal_mac_values[1] - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn comparison_is_invariant_to_mass_units_and_basis_scaling() {
        let mass = vec![
            vec![2.0, 0.0, 0.0],
            vec![0.0, 3.0, 0.0],
            vec![0.0, 0.0, 5.0],
        ];
        let left = vec![vec![0.5, 0.0, 0.0], vec![0.0, 1.0 / 3.0_f64.sqrt(), 0.0]];
        let right = vec![vec![0.0, 0.0, 1.0 / 5.0_f64.sqrt()], vec![0.0, 1.0 / 3.0_f64.sqrt(), 0.0]];
        let baseline = mass_weighted_modal_subspace_comparison(&left, &right, &mass)
            .expect("baseline units");
        let scaled_mass = mass
            .iter()
            .map(|row| row.iter().map(|value| value * 1.0e12).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let scaled_modes = vec![
            left[0].iter().map(|value| value * 1.0e90).collect::<Vec<_>>(),
            left[1].iter().map(|value| value * 1.0e-90).collect::<Vec<_>>(),
        ];
        let scaled = mass_weighted_modal_subspace_comparison(&scaled_modes, &right, &scaled_mass)
            .expect("scaled units");
        assert!((baseline.mean_principal_mac - scaled.mean_principal_mac).abs() < 1.0e-10);
        assert!((baseline.worst_principal_mac - scaled.worst_principal_mac).abs() < 1.0e-10);
    }

    #[test]
    fn comparison_uses_the_mass_inner_product_not_euclidean_mac() {
        let mass = vec![vec![2.0, 1.0], vec![1.0, 2.0]];
        let left = vec![vec![1.0, 0.0]];
        let right = vec![vec![0.0, 1.0]];
        let comparison =
            mass_weighted_modal_subspace_comparison(&left, &right, &mass)
                .expect("mass-weighted one-dimensional subspaces");
        // e1^T M e2 = 1; each squared M-norm is 2; MAC_M = 1/(2*2).
        assert!((comparison.worst_principal_mac - 0.25).abs() < 1.0e-12);
        assert!((comparison.mean_principal_mac - 0.25).abs() < 1.0e-12);
    }

    #[test]
    fn malformed_singular_or_nonfinite_inputs_fail_closed() {
        let mass = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        assert!(mass_weighted_modal_subspace_comparison(&[], &[], &mass).is_err());
        assert!(
            mass_weighted_modal_subspace_comparison(
                &[vec![1.0, 0.0]],
                &[vec![1.0, 0.0], vec![0.0, 1.0]],
                &mass,
            )
            .is_err()
        );
        assert!(
            mass_weighted_modal_subspace_comparison(
                &[vec![1.0, 0.0]],
                &[vec![0.0, 1.0]],
                &vec![vec![1.0, 0.0], vec![0.0, -1.0]],
            )
            .is_err()
        );
        assert!(
            mass_weighted_modal_subspace_comparison(
                &[vec![1.0, 0.0]],
                &[vec![f64::NAN, 0.0]],
                &mass,
            )
            .is_err()
        );
        assert!(
            mass_weighted_modal_subspace_comparison(
                &[vec![1.0, 0.0], vec![2.0, 0.0]],
                &[vec![1.0, 0.0], vec![0.0, 1.0]],
                &mass,
            )
            .is_err()
        );
    }
}
