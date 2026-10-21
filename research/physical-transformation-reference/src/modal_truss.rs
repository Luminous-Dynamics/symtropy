// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bounded 2D pin-jointed truss modal reference.
//!
//! The model uses linear axial-bar stiffness and translational mass matrices.
//! It returns mode shapes, generalized-eigenvalue residuals, mass-orthogonality
//! diagnostics and explicit near-zero modes. It is a verification kernel, not
//! an industrial-scale eigensolver or a validated structural-dynamics package.

use crate::Check;
use std::collections::{BTreeMap, BTreeSet};

const MAX_NODES: usize = 64;
const MAX_BARS: usize = 256;
const JACOBI_MAX_ITERATIONS_FACTOR: usize = 50;
const JACOBI_TOLERANCE_FACTOR: f64 = 1.0e-13;
const ZERO_MODE_RELATIVE_TOLERANCE: f64 = 1.0e-10;

/// Explicit mass model; the choice is part of every result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModalTrussMassFormulation {
    /// Consistent translational mass for a two-node 2D axial bar.
    Consistent,
    /// Row-sum-lumped mass, with half each member mass at each endpoint.
    RowSumLumped,
}

/// Geometry and restraints for a modal-analysis node.
#[derive(Clone, Debug, PartialEq)]
pub struct ModalTrussNode {
    pub id: String,
    pub x_m: f64,
    pub y_m: f64,
    pub restrained_x: bool,
    pub restrained_y: bool,
}

/// Material and section for a 2D pin-jointed axial bar.
#[derive(Clone, Debug, PartialEq)]
pub struct ModalTrussBar {
    pub id: String,
    pub node_i: String,
    pub node_j: String,
    pub area_m2: f64,
    pub youngs_modulus_pa: f64,
    pub density_kg_m3: f64,
}

/// Complete input to the bounded planar modal reference.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModalTrussInput {
    pub nodes: Vec<ModalTrussNode>,
    pub bars: Vec<ModalTrussBar>,
}

/// Mode classification deliberately preserves unresolved near-zero modes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModalModeClassification {
    /// Positive, non-negligible generalized eigenvalue.
    Elastic,
    /// Zero or near-zero value; may be a rigid-body mode or mechanism.
    NearZeroRigidBodyOrMechanism,
}

/// One mass-normalized mode over the input's unrestrained degrees of freedom.
#[derive(Clone, Debug, PartialEq)]
pub struct ModalTrussMode {
    /// Generalized eigenvalue omega squared, in rad^2/s^2.
    pub omega_squared_rad2_s2: f64,
    /// Natural frequency in Hz; zero for a classified near-zero mode.
    pub frequency_hz: f64,
    /// Components follow the free-DOF labels; sign is mathematically arbitrary.
    pub shape: Vec<f64>,
    /// Norm(K phi - lambda M phi) / (norm(K phi) + abs(lambda) norm(M phi)).
    pub normalized_eigen_residual: f64,
    pub classification: ModalModeClassification,
}

/// Modal solution with matrices and numerical diagnostics kept inspectable.
#[derive(Clone, Debug, PartialEq)]
pub struct ModalTrussResult {
    pub mass_formulation: ModalTrussMassFormulation,
    /// Labels such as C:x, C:y, in the exact reduced-matrix degree order.
    pub free_dof_labels: Vec<String>,
    pub stiffness_matrix_n_per_m: Vec<Vec<f64>>,
    pub mass_matrix_kg: Vec<Vec<f64>>,
    pub modes: Vec<ModalTrussMode>,
    /// Maximum absolute residual of Phi^T M Phi - I.
    pub maximum_mass_orthogonality_residual: f64,
    /// Modes too close to zero to be described as ordinary elastic modes.
    pub near_zero_mode_count: usize,
}

/// Analyze a bounded small-displacement, linear-elastic 2D truss.
///
/// Bars contribute axial stiffness EA/L. The consistent mass model uses the
/// standard two-node translational mass matrix rho*A*L/6 * [[2I,I],[I,2I]];
/// row-sum lumping places rho*A*L/2 at each endpoint. Restraints remove the
/// corresponding degrees of freedom before solving K phi = omega^2 M phi.
///
/// Near-zero modes are returned and labelled instead of silently discarded.
/// A reviewer must distinguish unsupported rigid-body motion from an internal
/// mechanism before making any physical claim.
pub fn analyze_planar_modal_truss(
    input: &ModalTrussInput,
    formulation: ModalTrussMassFormulation,
) -> Check<ModalTrussResult> {
    let node_indices = validate_input(input)?;
    let all_dofs = input.nodes.len() * 2;
    let mut full_stiffness = vec![vec![0.0; all_dofs]; all_dofs];
    let mut full_mass = vec![vec![0.0; all_dofs]; all_dofs];

    for bar in &input.bars {
        let i = node_indices[bar.node_i.as_str()];
        let j = node_indices[bar.node_j.as_str()];
        let node_i = &input.nodes[i];
        let node_j = &input.nodes[j];
        let dx = node_j.x_m - node_i.x_m;
        let dy = node_j.y_m - node_i.y_m;
        let length = dx.hypot(dy);
        let cosine = dx / length;
        let sine = dy / length;
        let axial_stiffness = bar.youngs_modulus_pa * bar.area_m2 / length;
        let member_mass = bar.density_kg_m3 * bar.area_m2 * length;
        if !axial_stiffness.is_finite()
            || axial_stiffness <= 0.0
            || !member_mass.is_finite()
            || member_mass <= 0.0
        {
            return Err(format!(
                "bar {} stiffness or mass is outside the supported numeric range",
                bar.id
            ));
        }

        let direction = [cosine, sine, -cosine, -sine];
        let global_dofs = [2 * i, 2 * i + 1, 2 * j, 2 * j + 1];
        for (local_i, global_i) in global_dofs.iter().enumerate() {
            for (local_j, global_j) in global_dofs.iter().enumerate() {
                full_stiffness[*global_i][*global_j] +=
                    axial_stiffness * direction[local_i] * direction[local_j];
            }
        }

        match formulation {
            ModalTrussMassFormulation::Consistent => {
                let diagonal_mass = member_mass / 3.0;
                let off_diagonal_mass = member_mass / 6.0;
                for (dof_i, dof_j) in [(2 * i, 2 * j), (2 * i + 1, 2 * j + 1)] {
                    full_mass[dof_i][dof_i] += diagonal_mass;
                    full_mass[dof_j][dof_j] += diagonal_mass;
                    full_mass[dof_i][dof_j] += off_diagonal_mass;
                    full_mass[dof_j][dof_i] += off_diagonal_mass;
                }
            }
            ModalTrussMassFormulation::RowSumLumped => {
                let half_member_mass = member_mass / 2.0;
                for dof in [2 * i, 2 * i + 1, 2 * j, 2 * j + 1] {
                    full_mass[dof][dof] += half_member_mass;
                }
            }
        }
    }

    if full_stiffness
        .iter()
        .chain(full_mass.iter())
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err("assembled modal matrices contain non-finite values".to_owned());
    }

    let mut free_global_dofs = Vec::new();
    let mut free_dof_labels = Vec::new();
    for (node_index, node) in input.nodes.iter().enumerate() {
        if !node.restrained_x {
            free_global_dofs.push(2 * node_index);
            free_dof_labels.push(format!("{}:x", node.id));
        }
        if !node.restrained_y {
            free_global_dofs.push(2 * node_index + 1);
            free_dof_labels.push(format!("{}:y", node.id));
        }
    }
    if free_global_dofs.is_empty() {
        return Err("modal analysis requires at least one unrestrained degree of freedom".to_owned());
    }

    let free_count = free_global_dofs.len();
    let mut stiffness = vec![vec![0.0; free_count]; free_count];
    let mut mass = vec![vec![0.0; free_count]; free_count];
    for (i, global_i) in free_global_dofs.iter().enumerate() {
        for (j, global_j) in free_global_dofs.iter().enumerate() {
            stiffness[i][j] = full_stiffness[*global_i][*global_j];
            mass[i][j] = full_mass[*global_i][*global_j];
        }
    }

    let lower = cholesky_lower(&mass)?;
    let inverse_lower = invert_lower(&lower)?;
    let transformed_left = multiply(&inverse_lower, &stiffness);
    let mut symmetric = vec![vec![0.0; free_count]; free_count];
    for (i, row) in symmetric.iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            *value = (0..free_count)
                .map(|k| transformed_left[i][k] * inverse_lower[j][k])
                .sum();
        }
    }
    symmetrize(&mut symmetric);

    let (eigenvalues, transformed_modes) = symmetric_eigenpairs(symmetric)?;
    let largest_eigenvalue = eigenvalues
        .iter()
        .copied()
        .fold(0.0_f64, f64::max);
    if !largest_eigenvalue.is_finite() || largest_eigenvalue <= 0.0 {
        return Err("the free-degree stiffness system has no positive modal scale; inspect mechanisms".to_owned());
    }
    let zero_threshold = largest_eigenvalue * ZERO_MODE_RELATIVE_TOLERANCE;
    if eigenvalues
        .iter()
        .any(|value| *value < -zero_threshold)
    {
        return Err("generalized eigenproblem produced a materially negative eigenvalue".to_owned());
    }

    let mut modes = Vec::with_capacity(free_count);
    for (mode_index, eigenvalue) in eigenvalues.iter().enumerate() {
        let mut shape = vec![0.0; free_count];
        for (i, value) in shape.iter_mut().enumerate() {
            *value = (0..free_count)
                .map(|k| inverse_lower[k][i] * transformed_modes[k][mode_index])
                .sum();
        }
        let classification = if *eigenvalue <= zero_threshold {
            ModalModeClassification::NearZeroRigidBodyOrMechanism
        } else {
            ModalModeClassification::Elastic
        };
        let frequency_hz = if classification == ModalModeClassification::Elastic {
            eigenvalue.sqrt() / (2.0 * std::f64::consts::PI)
        } else {
            0.0
        };
        let normalized_eigen_residual =
            eigen_residual(&stiffness, &mass, &shape, *eigenvalue);
        if !frequency_hz.is_finite()
            || !normalized_eigen_residual.is_finite()
            || shape.iter().any(|component| !component.is_finite())
        {
            return Err("modal solution contains a non-finite frequency, shape, or residual".to_owned());
        }
        modes.push(ModalTrussMode {
            omega_squared_rad2_s2: *eigenvalue,
            frequency_hz,
            shape,
            normalized_eigen_residual,
            classification,
        });
    }

    let maximum_mass_orthogonality_residual = mass_orthogonality_residual(&mass, &modes);
    if !maximum_mass_orthogonality_residual.is_finite() {
        return Err("mass orthogonality diagnostic is non-finite".to_owned());
    }
    let near_zero_mode_count = modes
        .iter()
        .filter(|mode| mode.classification == ModalModeClassification::NearZeroRigidBodyOrMechanism)
        .count();

    Ok(ModalTrussResult {
        mass_formulation: formulation,
        free_dof_labels,
        stiffness_matrix_n_per_m: stiffness,
        mass_matrix_kg: mass,
        modes,
        maximum_mass_orthogonality_residual,
        near_zero_mode_count,
    })
}

fn validate_input(input: &ModalTrussInput) -> Check<BTreeMap<String, usize>> {
    if input.nodes.len() < 2 || input.nodes.len() > MAX_NODES {
        return Err(format!("node count must be between 2 and {MAX_NODES}"));
    }
    if input.bars.is_empty() || input.bars.len() > MAX_BARS {
        return Err(format!("bar count must be between 1 and {MAX_BARS}"));
    }

    let mut node_indices = BTreeMap::new();
    for (index, node) in input.nodes.iter().enumerate() {
        crate::validate_token(&node.id, "modal-truss node id")?;
        if !node.x_m.is_finite() || !node.y_m.is_finite() {
            return Err(format!("node {} has non-finite coordinates", node.id));
        }
        if node_indices.insert(node.id.clone(), index).is_some() {
            return Err(format!("duplicate modal-truss node id: {}", node.id));
        }
    }

    let mut bar_ids = BTreeSet::new();
    for bar in &input.bars {
        crate::validate_token(&bar.id, "modal-truss bar id")?;
        if !bar_ids.insert(bar.id.as_str()) {
            return Err(format!("duplicate modal-truss bar id: {}", bar.id));
        }
        let i = node_indices
            .get(&bar.node_i)
            .ok_or_else(|| format!("bar {} references missing node {}", bar.id, bar.node_i))?;
        let j = node_indices
            .get(&bar.node_j)
            .ok_or_else(|| format!("bar {} references missing node {}", bar.id, bar.node_j))?;
        if i == j {
            return Err(format!("bar {} must connect two different nodes", bar.id));
        }
        for (value, label) in [
            (bar.area_m2, "area"),
            (bar.youngs_modulus_pa, "Young's modulus"),
            (bar.density_kg_m3, "density"),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(format!("bar {} has invalid {label}", bar.id));
            }
        }
        let node_i = &input.nodes[*i];
        let node_j = &input.nodes[*j];
        if node_i.x_m == node_j.x_m && node_i.y_m == node_j.y_m {
            return Err(format!("bar {} has zero geometric length", bar.id));
        }
    }
    Ok(node_indices)
}

fn cholesky_lower(matrix: &[Vec<f64>]) -> Check<Vec<Vec<f64>>> {
    let n = matrix.len();
    let mut lower = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let correction: f64 = (0..j).map(|k| lower[i][k] * lower[j][k]).sum();
            let value = matrix[i][j] - correction;
            if i == j {
                if !value.is_finite() || value <= 0.0 {
                    return Err("reduced mass matrix is not numerically positive definite".to_owned());
                }
                lower[i][j] = value.sqrt();
            } else {
                lower[i][j] = value / lower[j][j];
            }
        }
    }
    Ok(lower)
}

fn invert_lower(lower: &[Vec<f64>]) -> Check<Vec<Vec<f64>>> {
    let n = lower.len();
    let mut inverse = vec![vec![0.0; n]; n];
    for (row, lower_row) in lower.iter().enumerate() {
        let (prior, current_and_after) = inverse.split_at_mut(row);
        let inverse_row = &mut current_and_after[0];
        for (column, cell) in inverse_row.iter_mut().enumerate() {
            let correction: f64 = prior
                .iter()
                .enumerate()
                .map(|(k, inverse_row)| lower_row[k] * inverse_row[column])
                .sum();
            let rhs = if row == column { 1.0 } else { 0.0 };
            *cell = (rhs - correction) / lower_row[row];
            if !cell.is_finite() {
                return Err("mass inverse factor is non-finite".to_owned());
            }
        }
    }
    Ok(inverse)
}

fn multiply(left: &[Vec<f64>], right: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = left.len();
    let mut result = vec![vec![0.0; n]; n];
    for i in 0..n {
        for k in 0..n {
            let value = left[i][k];
            for j in 0..n {
                result[i][j] += value * right[k][j];
            }
        }
    }
    result
}

fn symmetrize(matrix: &mut [Vec<f64>]) {
    let n = matrix.len();
    for i in 0..n {
        let (above, at_and_below) = matrix.split_at_mut(i);
        let row = &mut at_and_below[0];
        for (above_row, row_value) in above.iter_mut().zip(row.iter_mut()) {
            let average = 0.5 * (*row_value + above_row[i]);
            *row_value = average;
            above_row[i] = average;
        }
    }
}

fn symmetric_eigenpairs(mut matrix: Vec<Vec<f64>>) -> Check<(Vec<f64>, Vec<Vec<f64>>)> {
    let n = matrix.len();
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err("transformed stiffness matrix contains non-finite values".to_owned());
    }
    let scale = matrix
        .iter()
        .enumerate()
        .map(|(index, row)| row[index].abs())
        .fold(0.0_f64, f64::max);
    if !scale.is_finite() || scale <= 0.0 {
        return Err("transformed stiffness matrix has no positive finite scale".to_owned());
    }
    let tolerance = scale * JACOBI_TOLERANCE_FACTOR;
    let max_iterations = JACOBI_MAX_ITERATIONS_FACTOR * n * n;
    let mut eigenvectors = vec![vec![0.0; n]; n];
    for (i, row) in eigenvectors.iter_mut().enumerate() {
        row[i] = 1.0;
    }

    for _ in 0..max_iterations {
        let mut largest = 0.0_f64;
        let mut pivot = None;
        for (i, row) in matrix.iter().enumerate() {
            for (j, value) in row.iter().enumerate().skip(i + 1) {
                let candidate = value.abs();
                if candidate > largest {
                    largest = candidate;
                    pivot = Some((i, j));
                }
            }
        }
        if largest <= tolerance {
            let mut order: Vec<usize> = (0..n).collect();
            order.sort_by(|left, right| matrix[*left][*left].total_cmp(&matrix[*right][*right]));
            let values: Vec<f64> = order.iter().map(|index| matrix[*index][*index]).collect();
            if values.iter().any(|value| !value.is_finite()) {
                return Err("generalized eigenproblem produced a non-finite eigenvalue".to_owned());
            }
            let sorted_vectors: Vec<Vec<f64>> = (0..n)
                .map(|row| order.iter().map(|column| eigenvectors[row][*column]).collect())
                .collect();
            return Ok((values, sorted_vectors));
        }
        let (p, q) = pivot.ok_or_else(|| "Jacobi eigensolver has no pivot".to_owned())?;
        let app = matrix[p][p];
        let aqq = matrix[q][q];
        let apq = matrix[p][q];
        let tau = (aqq - app) / (2.0 * apq);
        let t = if tau >= 0.0 {
            1.0 / (tau + tau.hypot(1.0))
        } else {
            -1.0 / (-tau + tau.hypot(1.0))
        };
        let cosine = 1.0 / (1.0 + t * t).sqrt();
        let sine = t * cosine;

        let mut changed_columns = Vec::with_capacity(n.saturating_sub(2));
        for (k, row) in matrix.iter_mut().enumerate() {
            if k != p && k != q {
                let akp = row[p];
                let akq = row[q];
                let new_p = cosine * akp - sine * akq;
                let new_q = sine * akp + cosine * akq;
                row[p] = new_p;
                row[q] = new_q;
                changed_columns.push((k, new_p, new_q));
            }
        }
        for (k, new_p, new_q) in changed_columns {
            matrix[p][k] = new_p;
            matrix[q][k] = new_q;
        }
        matrix[p][p] =
            cosine * cosine * app - 2.0 * sine * cosine * apq + sine * sine * aqq;
        matrix[q][q] =
            sine * sine * app + 2.0 * sine * cosine * apq + cosine * cosine * aqq;
        matrix[p][q] = 0.0;
        matrix[q][p] = 0.0;

        for row in &mut eigenvectors {
            let old_p = row[p];
            let old_q = row[q];
            row[p] = cosine * old_p - sine * old_q;
            row[q] = sine * old_p + cosine * old_q;
        }
    }
    Err("Jacobi eigensolver did not converge within its iteration budget".to_owned())
}

fn eigen_residual(
    stiffness: &[Vec<f64>],
    mass: &[Vec<f64>],
    shape: &[f64],
    eigenvalue: f64,
) -> f64 {
    let k_phi = matrix_vector(stiffness, shape);
    let m_phi = matrix_vector(mass, shape);
    let residual: Vec<f64> = k_phi
        .iter()
        .zip(m_phi.iter())
        .map(|(k_value, m_value)| k_value - eigenvalue * m_value)
        .collect();
    let numerator = euclidean_norm(&residual);
    let denominator = euclidean_norm(&k_phi) + eigenvalue.abs() * euclidean_norm(&m_phi);
    if denominator == 0.0 {
        if numerator == 0.0 {
            0.0
        } else {
            f64::INFINITY
        }
    } else {
        numerator / denominator
    }
}

fn mass_orthogonality_residual(mass: &[Vec<f64>], modes: &[ModalTrussMode]) -> f64 {
    let mass_modes: Vec<Vec<f64>> = modes
        .iter()
        .map(|mode| matrix_vector(mass, &mode.shape))
        .collect();
    let mut maximum = 0.0_f64;
    for (i, left) in modes.iter().enumerate() {
        for (j, mass_right) in mass_modes.iter().enumerate() {
            let actual: f64 = left
                .shape
                .iter()
                .zip(mass_right.iter())
                .map(|(a, b)| a * b)
                .sum();
            let expected = if i == j { 1.0 } else { 0.0 };
            maximum = maximum.max((actual - expected).abs());
        }
    }
    maximum
}

fn matrix_vector(matrix: &[Vec<f64>], vector: &[f64]) -> Vec<f64> {
    matrix
        .iter()
        .map(|row| row.iter().zip(vector.iter()).map(|(a, b)| a * b).sum())
        .collect()
}

fn euclidean_norm(vector: &[f64]) -> f64 {
    vector.iter().map(|value| value * value).sum::<f64>().sqrt()
}

#[cfg(test)]
mod tests {
    use super::{
        ModalModeClassification, ModalTrussBar, ModalTrussInput, ModalTrussMassFormulation,
        ModalTrussNode, analyze_planar_modal_truss,
    };

    const E: f64 = 200.0e9;
    const DENSITY: f64 = 7_800.0;
    const AREA_AC: f64 = 0.015811388300841896;
    const AREA_BC: f64 = 0.042426406871192854;

    fn asymmetric_two_bar_fixture() -> ModalTrussInput {
        ModalTrussInput {
            nodes: vec![
                ModalTrussNode {
                    id: "A".to_owned(),
                    x_m: 0.0,
                    y_m: 0.0,
                    restrained_x: true,
                    restrained_y: true,
                },
                ModalTrussNode {
                    id: "B".to_owned(),
                    x_m: 4.0,
                    y_m: 0.0,
                    restrained_x: true,
                    restrained_y: true,
                },
                ModalTrussNode {
                    id: "C".to_owned(),
                    x_m: 1.0,
                    y_m: 3.0,
                    restrained_x: false,
                    restrained_y: false,
                },
            ],
            bars: vec![
                ModalTrussBar {
                    id: "AC".to_owned(),
                    node_i: "A".to_owned(),
                    node_j: "C".to_owned(),
                    area_m2: AREA_AC,
                    youngs_modulus_pa: E,
                    density_kg_m3: DENSITY,
                },
                ModalTrussBar {
                    id: "BC".to_owned(),
                    node_i: "B".to_owned(),
                    node_j: "C".to_owned(),
                    area_m2: AREA_BC,
                    youngs_modulus_pa: E,
                    density_kg_m3: DENSITY,
                },
            ],
        }
    }

    fn close_relative(actual: f64, expected: f64, tolerance: f64, label: &str) {
        let allowed = tolerance * expected.abs().max(1.0e-14);
        assert!(
            (actual - expected).abs() <= allowed,
            "{label}: actual={actual:.12e}, expected={expected:.12e}, tolerance={allowed:.3e}"
        );
    }

    fn eigenvalue_oracle(formulation: ModalTrussMassFormulation) -> [f64; 2] {
        let length_ac = 10.0_f64.sqrt();
        let length_bc = 18.0_f64.sqrt();
        let stiffness_ac = E * AREA_AC / length_ac;
        let stiffness_bc = E * AREA_BC / length_bc;
        let c_ac = 1.0 / length_ac;
        let s_ac = 3.0 / length_ac;
        let c_bc = -3.0 / length_bc;
        let s_bc = 3.0 / length_bc;
        let kxx = stiffness_ac * c_ac * c_ac + stiffness_bc * c_bc * c_bc;
        let kxy = stiffness_ac * c_ac * s_ac + stiffness_bc * c_bc * s_bc;
        let kyy = stiffness_ac * s_ac * s_ac + stiffness_bc * s_bc * s_bc;
        let discriminant = ((kxx - kyy).powi(2) + 4.0 * kxy * kxy).sqrt();
        let volume_sum = AREA_AC * length_ac + AREA_BC * length_bc;
        let mass_per_free_dof = match formulation {
            ModalTrussMassFormulation::Consistent => DENSITY * volume_sum / 3.0,
            ModalTrussMassFormulation::RowSumLumped => DENSITY * volume_sum / 2.0,
        };
        [
            (kxx + kyy - discriminant) / (2.0 * mass_per_free_dof),
            (kxx + kyy + discriminant) / (2.0 * mass_per_free_dof),
        ]
    }

    #[test]
    fn asymmetric_supported_truss_matches_independent_frequency_and_matrix_oracle() {
        for formulation in [
            ModalTrussMassFormulation::Consistent,
            ModalTrussMassFormulation::RowSumLumped,
        ] {
            let result =
                analyze_planar_modal_truss(&asymmetric_two_bar_fixture(), formulation)
                    .expect("well-constrained asymmetric modal truss");
            let length_ac = 10.0_f64.sqrt();
            let length_bc = 18.0_f64.sqrt();
            let k_ac = E * AREA_AC / length_ac;
            let k_bc = E * AREA_BC / length_bc;
            let expected = [
                [
                    k_ac / 10.0 + 9.0 * k_bc / 18.0,
                    3.0 * k_ac / 10.0 - 9.0 * k_bc / 18.0,
                ],
                [
                    3.0 * k_ac / 10.0 - 9.0 * k_bc / 18.0,
                    9.0 * k_ac / 10.0 + 9.0 * k_bc / 18.0,
                ],
            ];
            assert_eq!(result.free_dof_labels, ["C:x", "C:y"]);
            for (i, row) in expected.iter().enumerate() {
                for (j, value) in row.iter().enumerate() {
                    close_relative(
                        result.stiffness_matrix_n_per_m[i][j],
                        *value,
                        1.0e-10,
                        "independent reduced stiffness",
                    );
                }
            }
            for (mode, eigenvalue) in result
                .modes
                .iter()
                .zip(eigenvalue_oracle(formulation).iter())
            {
                close_relative(
                    mode.omega_squared_rad2_s2,
                    *eigenvalue,
                    1.0e-10,
                    "independent generalized eigenvalue",
                );
                close_relative(
                    mode.frequency_hz,
                    eigenvalue.sqrt() / (2.0 * std::f64::consts::PI),
                    1.0e-10,
                    "frequency derived from independent eigenvalue",
                );
                assert_eq!(mode.classification, ModalModeClassification::Elastic);
                assert!(mode.normalized_eigen_residual < 1.0e-10);
            }
            assert!(result.maximum_mass_orthogonality_residual < 1.0e-10);
            assert_eq!(result.near_zero_mode_count, 0);
        }
    }

    #[test]
    fn consistent_and_lumped_mass_choices_have_expected_frequency_ratio() {
        let consistent = analyze_planar_modal_truss(
            &asymmetric_two_bar_fixture(),
            ModalTrussMassFormulation::Consistent,
        )
        .expect("consistent modal solution");
        let lumped = analyze_planar_modal_truss(
            &asymmetric_two_bar_fixture(),
            ModalTrussMassFormulation::RowSumLumped,
        )
        .expect("lumped modal solution");
        for (consistent_mode, lumped_mode) in consistent.modes.iter().zip(lumped.modes.iter()) {
            close_relative(
                consistent_mode.frequency_hz / lumped_mode.frequency_hz,
                (3.0_f64 / 2.0).sqrt(),
                1.0e-10,
                "consistent/lumped frequency ratio",
            );
        }
    }

    #[test]
    fn mass_normalized_modes_match_independent_two_by_two_eigenvectors() {
        let result = analyze_planar_modal_truss(
            &asymmetric_two_bar_fixture(),
            ModalTrussMassFormulation::Consistent,
        )
        .expect("consistent modal solution");
        let kxx = result.stiffness_matrix_n_per_m[0][0];
        let kxy = result.stiffness_matrix_n_per_m[0][1];
        for (mode, eigenvalue) in result
            .modes
            .iter()
            .zip(eigenvalue_oracle(ModalTrussMassFormulation::Consistent).iter())
        {
            let raw = [1.0, (*eigenvalue - kxx) / kxy];
            let norm = (raw[0] * raw[0] + raw[1] * raw[1]).sqrt();
            let expected = [raw[0] / norm, raw[1] / norm];
            let actual_norm = (mode.shape[0] * mode.shape[0] + mode.shape[1] * mode.shape[1]).sqrt();
            let actual = [mode.shape[0] / actual_norm, mode.shape[1] / actual_norm];
            let dot = actual[0] * expected[0] + actual[1] * expected[1];
            let modal_assurance = dot * dot;
            assert!(modal_assurance > 1.0 - 1.0e-10);
        }
    }

    #[test]
    fn near_zero_mechanism_modes_are_reported_not_silently_dropped() {
        let input = ModalTrussInput {
            nodes: vec![
                ModalTrussNode {
                    id: "fixed".to_owned(),
                    x_m: 0.0,
                    y_m: 0.0,
                    restrained_x: true,
                    restrained_y: true,
                },
                ModalTrussNode {
                    id: "free".to_owned(),
                    x_m: 1.0,
                    y_m: 0.0,
                    restrained_x: false,
                    restrained_y: false,
                },
            ],
            bars: vec![ModalTrussBar {
                id: "bar".to_owned(),
                node_i: "fixed".to_owned(),
                node_j: "free".to_owned(),
                area_m2: 0.01,
                youngs_modulus_pa: E,
                density_kg_m3: DENSITY,
            }],
        };
        let result = analyze_planar_modal_truss(&input, ModalTrussMassFormulation::Consistent)
            .expect("mass-positive single-bar system");
        assert_eq!(result.near_zero_mode_count, 1);
        assert_eq!(
            result.modes[0].classification,
            ModalModeClassification::NearZeroRigidBodyOrMechanism
        );
        assert_eq!(result.modes[0].frequency_hz, 0.0);
        assert_eq!(result.modes[1].classification, ModalModeClassification::Elastic);
    }

    #[test]
    fn scale_metamorphics_hold_for_area_material_and_length() {
        let reference = analyze_planar_modal_truss(
            &asymmetric_two_bar_fixture(),
            ModalTrussMassFormulation::Consistent,
        )
        .expect("reference modal solution");

        let mut area_scaled = asymmetric_two_bar_fixture();
        for bar in &mut area_scaled.bars {
            bar.area_m2 *= 7.0;
        }
        let area_result =
            analyze_planar_modal_truss(&area_scaled, ModalTrussMassFormulation::Consistent)
                .expect("area-scaled modal solution");

        let mut material_scaled = asymmetric_two_bar_fixture();
        for bar in &mut material_scaled.bars {
            bar.youngs_modulus_pa *= 9.0;
            bar.density_kg_m3 *= 9.0;
        }
        let material_result = analyze_planar_modal_truss(
            &material_scaled,
            ModalTrussMassFormulation::Consistent,
        )
        .expect("E/rho-scaled modal solution");

        let mut length_scaled = asymmetric_two_bar_fixture();
        for node in &mut length_scaled.nodes {
            node.x_m *= 2.0;
            node.y_m *= 2.0;
        }
        let length_result =
            analyze_planar_modal_truss(&length_scaled, ModalTrussMassFormulation::Consistent)
                .expect("length-scaled modal solution");

        for (original, scaled) in reference.modes.iter().zip(area_result.modes.iter()) {
            close_relative(scaled.frequency_hz, original.frequency_hz, 1.0e-10, "area scale");
        }
        for (original, scaled) in reference.modes.iter().zip(material_result.modes.iter()) {
            close_relative(
                scaled.frequency_hz,
                original.frequency_hz,
                1.0e-10,
                "joint E/rho scale",
            );
        }
        for (original, scaled) in reference.modes.iter().zip(length_result.modes.iter()) {
            close_relative(
                scaled.frequency_hz,
                original.frequency_hz / 2.0,
                1.0e-10,
                "inverse-length scale",
            );
        }
    }

    #[test]
    fn input_order_does_not_change_sorted_frequencies() {
        let original = asymmetric_two_bar_fixture();
        let mut reordered = original.clone();
        reordered.nodes.reverse();
        reordered.bars.reverse();
        let a = analyze_planar_modal_truss(&original, ModalTrussMassFormulation::Consistent)
            .expect("original order");
        let b = analyze_planar_modal_truss(&reordered, ModalTrussMassFormulation::Consistent)
            .expect("reordered input");
        assert_eq!(a.modes.len(), b.modes.len());
        for (left, right) in a.modes.iter().zip(b.modes.iter()) {
            close_relative(left.frequency_hz, right.frequency_hz, 1.0e-10, "order invariant");
        }
    }

    #[test]
    fn malformed_and_unsupported_inputs_fail_closed() {
        let mut input = asymmetric_two_bar_fixture();
        input.bars[0].density_kg_m3 = 0.0;
        assert!(analyze_planar_modal_truss(&input, ModalTrussMassFormulation::Consistent).is_err());

        input = asymmetric_two_bar_fixture();
        input.nodes[1].id = "A".to_owned();
        assert!(analyze_planar_modal_truss(&input, ModalTrussMassFormulation::Consistent).is_err());

        input = asymmetric_two_bar_fixture();
        input.bars[0].node_j = "missing".to_owned();
        assert!(analyze_planar_modal_truss(&input, ModalTrussMassFormulation::Consistent).is_err());

        input = asymmetric_two_bar_fixture();
        input.nodes[2].x_m = input.nodes[0].x_m;
        input.nodes[2].y_m = input.nodes[0].y_m;
        assert!(analyze_planar_modal_truss(&input, ModalTrussMassFormulation::Consistent).is_err());

        input = asymmetric_two_bar_fixture();
        for node in &mut input.nodes {
            node.restrained_x = true;
            node.restrained_y = true;
        }
        assert!(analyze_planar_modal_truss(&input, ModalTrussMassFormulation::Consistent).is_err());
    }
}
