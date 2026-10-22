// SPDX-License-Identifier: AGPL-3.0-or-later
//! Modal diagnostics over assembled released-joint Euler-Bernoulli beam DOFs.
//!
//! This module reuses the generalized symmetric eigensolver verified by the
//! axial-bar tests. It preserves zero/mechanism modes rather than dropping them.

use crate::Check;
use crate::beam_pin_assembly::{BeamMemberChain, BeamPinAssembly, BeamPinDofKind, BeamPinNode};
use crate::modal_bar::{
    Matrix, generalized_symmetric_eigenpairs, normalized_eigen_residual, quadratic_form,
};
use std::collections::BTreeMap;

const ZERO_MODE_RELATIVE_TOLERANCE: f64 = 1.0e-10;

/// Explicit modal classification. Near-zero values remain visible as
/// mechanisms/rigid modes; negative values are flagged as unstable or invalid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BeamModeClass {
    Elastic,
    NearZeroOrMechanism,
    NegativeOrUnstable,
}

/// A generalized eigenmode over the entire global DOF space. Fixed DOFs have
/// zero components; the shape is mass-normalized over the free subspace.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamModalMode {
    pub eigenvalue_rad_s_squared: f64,
    /// None for non-positive or threshold-level eigenvalues; these remain present.
    pub frequency_hz: Option<f64>,
    pub class: BeamModeClass,
    pub shape_global_dofs: Vec<f64>,
    pub normalized_eigen_residual: f64,
    pub mass_norm_residual: f64,
}

/// Modal solution plus explicit model diagnostics. This does not qualify a
/// structure as physically complete or safe.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamModalResult {
    pub modes: Vec<BeamModalMode>,
    pub zero_mode_relative_tolerance: f64,
    pub maximum_normalized_eigen_residual: f64,
    pub maximum_mass_norm_residual: f64,
    pub maximum_off_diagonal_mass_orthogonality_residual: f64,
}

/// Solve the generalized eigenproblem on the assembly's unconstrained DOFs.
///
/// The returned mode count equals the free-DOF count. Small/negative modes
/// are retained and classified instead of silently filtered.
pub fn solve_beam_pin_assembly_modes(assembly: &BeamPinAssembly) -> Check<BeamModalResult> {
    let stiffness_reduced = reduce_matrix(&assembly.stiffness_global, &assembly.free_dof_indices)?;
    let mass_reduced = reduce_matrix(&assembly.mass_global, &assembly.free_dof_indices)?;
    let eigenpairs = generalized_symmetric_eigenpairs(&stiffness_reduced, &mass_reduced)?;

    if eigenpairs.len() != assembly.free_dof_indices.len() {
        return Err("eigenpair count does not match the free-DOF count".to_owned());
    }
    let largest_magnitude = eigenpairs
        .iter()
        .map(|(eigenvalue, _)| eigenvalue.abs())
        .fold(0.0_f64, f64::max);
    if !largest_magnitude.is_finite() || largest_magnitude <= 0.0 {
        return Err("modal eigenproblem has no finite positive spectral scale".to_owned());
    }
    let zero_tolerance = largest_magnitude * ZERO_MODE_RELATIVE_TOLERANCE;

    let mut modes = Vec::with_capacity(eigenpairs.len());
    for (eigenvalue, shape_free) in eigenpairs {
        if shape_free.len() != assembly.free_dof_indices.len() {
            return Err("returned mode shape dimension differs from free-DOF map".to_owned());
        }
        let class = if eigenvalue < -zero_tolerance {
            BeamModeClass::NegativeOrUnstable
        } else if eigenvalue <= zero_tolerance {
            BeamModeClass::NearZeroOrMechanism
        } else {
            BeamModeClass::Elastic
        };
        let frequency_hz = if class == BeamModeClass::Elastic {
            let frequency = eigenvalue.sqrt() / (2.0 * std::f64::consts::PI);
            if !frequency.is_finite() || frequency <= 0.0 {
                return Err("elastic modal frequency is not finite and positive".to_owned());
            }
            Some(frequency)
        } else {
            None
        };

        let eigen_residual =
            normalized_eigen_residual(&stiffness_reduced, &mass_reduced, eigenvalue, &shape_free)?;
        let mass_norm = quadratic_form(&mass_reduced, &shape_free)?;
        let mut shape_global = vec![0.0; assembly.stiffness_global.len()];
        for (free_index, global_index) in assembly.free_dof_indices.iter().enumerate() {
            shape_global[*global_index] = shape_free[free_index];
        }
        modes.push(BeamModalMode {
            eigenvalue_rad_s_squared: eigenvalue,
            frequency_hz,
            class,
            shape_global_dofs: shape_global,
            normalized_eigen_residual: eigen_residual,
            mass_norm_residual: (mass_norm - 1.0).abs(),
        });
    }

    let max_residual = modes
        .iter()
        .map(|mode| mode.normalized_eigen_residual)
        .fold(0.0_f64, f64::max);
    let max_mass_norm = modes
        .iter()
        .map(|mode| mode.mass_norm_residual)
        .fold(0.0_f64, f64::max);
    let mut max_orthogonality = 0.0_f64;
    for first in 0..modes.len() {
        for second in (first + 1)..modes.len() {
            let inner = mass_inner_product(
                &mass_reduced,
                &extract_free_shape(&modes[first], &assembly.free_dof_indices),
                &extract_free_shape(&modes[second], &assembly.free_dof_indices),
            )?;
            max_orthogonality = max_orthogonality.max(inner.abs());
        }
    }

    Ok(BeamModalResult {
        modes,
        zero_mode_relative_tolerance: ZERO_MODE_RELATIVE_TOLERANCE,
        maximum_normalized_eigen_residual: max_residual,
        maximum_mass_norm_residual: max_mass_norm,
        maximum_off_diagonal_mass_orthogonality_residual: max_orthogonality,
    })
}

/// Evaluate a mode's in-plane translation on a fixed spatial sampling grid
/// along each physical beam member. Cubic Hermite interpolation is applied to
/// transverse displacement/rotation and linear interpolation to axial motion.
/// The returned vector is [ux, uy] for every sample point, ordered by member ID
/// then increasing physical arclength fraction. This makes mode shapes from
/// different mesh densities comparable without comparing incompatible nodal
/// vectors directly.
pub fn sample_beam_chain_mode_on_fixed_grid(
    joints: &[BeamPinNode],
    members: &[BeamMemberChain],
    assembly: &BeamPinAssembly,
    mode: &BeamModalMode,
    sample_points_per_member: usize,
) -> Check<Vec<f64>> {
    if sample_points_per_member < 2 || sample_points_per_member > 1025 {
        return Err("sample_points_per_member must be between 2 and 1025".to_owned());
    }
    let global_dof_count = assembly.stiffness_global.len();
    if mode.shape_global_dofs.len() != global_dof_count
        || assembly.mass_global.len() != global_dof_count
    {
        return Err("mode shape and assembly DOF dimensions do not agree".to_owned());
    }
    if mode
        .shape_global_dofs
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err("mode shape contains a non-finite component".to_owned());
    }

    let mut coordinates = BTreeMap::<String, (f64, f64)>::new();
    for joint in joints {
        if joint.id.is_empty() || !joint.x_m.is_finite() || !joint.y_m.is_finite() {
            return Err("physical joint ID or coordinates are invalid".to_owned());
        }
        if coordinates
            .insert(joint.id.clone(), (joint.x_m, joint.y_m))
            .is_some()
        {
            return Err(format!("duplicate physical joint ID: {}", joint.id));
        }
    }

    let mut member_counts = BTreeMap::<String, usize>::new();
    let mut ordered_members = members.to_vec();
    ordered_members.sort_by(|left, right| left.id.cmp(&right.id));
    for member in &ordered_members {
        if member.id.is_empty()
            || !(1..=32).contains(&member.elements_per_member)
            || !coordinates.contains_key(&member.start_node_id)
            || !coordinates.contains_key(&member.end_node_id)
            || member.start_node_id == member.end_node_id
        {
            return Err(format!("member-chain input is invalid for {}", member.id));
        }
        if member_counts
            .insert(member.id.clone(), member.elements_per_member)
            .is_some()
        {
            return Err(format!("duplicate member-chain ID: {}", member.id));
        }
    }
    if ordered_members.is_empty() {
        return Err("at least one member chain is required for spatial mode sampling".to_owned());
    }

    let mut translations = BTreeMap::<String, (Option<usize>, Option<usize>)>::new();
    let mut rotations = BTreeMap::<(String, usize), usize>::new();
    for descriptor in &assembly.dof_descriptors {
        if descriptor.index >= global_dof_count {
            return Err("DOF descriptor index is outside the global matrix".to_owned());
        }
        match &descriptor.kind {
            BeamPinDofKind::NodeTranslationX { node_id } => {
                let entry = translations.entry(node_id.clone()).or_default();
                if entry.0.replace(descriptor.index).is_some() {
                    return Err(format!("duplicate x-translation DOF for node {node_id}"));
                }
            }
            BeamPinDofKind::NodeTranslationY { node_id } => {
                let entry = translations.entry(node_id.clone()).or_default();
                if entry.1.replace(descriptor.index).is_some() {
                    return Err(format!("duplicate y-translation DOF for node {node_id}"));
                }
            }
            BeamPinDofKind::MemberEndRotation {
                member_id,
                at_start,
            } => {
                let element_count = *member_counts
                    .get(member_id)
                    .ok_or_else(|| format!("rotation references unknown member {member_id}"))?;
                let station = if *at_start { 0 } else { element_count };
                if rotations
                    .insert((member_id.clone(), station), descriptor.index)
                    .is_some()
                {
                    return Err(format!(
                        "duplicate member endpoint rotation: {member_id}/{station}"
                    ));
                }
            }
            BeamPinDofKind::MemberInteriorRotation {
                member_id,
                station_index,
            } => {
                let element_count = *member_counts
                    .get(member_id)
                    .ok_or_else(|| format!("rotation references unknown member {member_id}"))?;
                if *station_index == 0 || *station_index >= element_count {
                    return Err(format!(
                        "interior rotation station is outside member {}: {}",
                        member_id, station_index
                    ));
                }
                if rotations
                    .insert((member_id.clone(), *station_index), descriptor.index)
                    .is_some()
                {
                    return Err(format!(
                        "duplicate interior rotation: {member_id}/{station_index}"
                    ));
                }
            }
        }
    }

    let mut sampled = Vec::with_capacity(ordered_members.len() * sample_points_per_member * 2);
    for member in &ordered_members {
        let element_count = member.elements_per_member;
        let (x_start, y_start) = *coordinates
            .get(&member.start_node_id)
            .expect("validated physical start joint");
        let (x_end, y_end) = *coordinates
            .get(&member.end_node_id)
            .expect("validated physical end joint");
        let delta_x = x_end - x_start;
        let delta_y = y_end - y_start;
        let total_length = delta_x.hypot(delta_y);
        if !total_length.is_finite() || total_length <= 0.0 {
            return Err(format!("member {} has invalid physical length", member.id));
        }
        let cosine = delta_x / total_length;
        let sine = delta_y / total_length;
        let element_length = total_length / element_count as f64;

        for sample_index in 0..sample_points_per_member {
            let fraction = sample_index as f64 / (sample_points_per_member - 1) as f64;
            let scaled_station = fraction * element_count as f64;
            let element_index = if sample_index + 1 == sample_points_per_member {
                element_count - 1
            } else {
                (scaled_station.floor() as usize).min(element_count - 1)
            };
            let xi = if sample_index + 1 == sample_points_per_member {
                1.0
            } else {
                scaled_station - element_index as f64
            };

            let start_station_id = if element_index == 0 {
                member.start_node_id.clone()
            } else {
                format!("{}::station::{:04}", member.id, element_index)
            };
            let end_station_id = if element_index + 1 == element_count {
                member.end_node_id.clone()
            } else {
                format!("{}::station::{:04}", member.id, element_index + 1)
            };

            let translation_indices = |node_id: &str| -> Check<(usize, usize)> {
                let (x, y) = translations
                    .get(node_id)
                    .ok_or_else(|| format!("missing translation DOFs for node {node_id}"))?;
                Ok((
                    x.ok_or_else(|| format!("missing x translation for node {node_id}"))?,
                    y.ok_or_else(|| format!("missing y translation for node {node_id}"))?,
                ))
            };
            let (ux_i, uy_i) = translation_indices(&start_station_id)?;
            let (ux_j, uy_j) = translation_indices(&end_station_id)?;
            let theta_i = *rotations
                .get(&(member.id.clone(), element_index))
                .ok_or_else(|| {
                    format!(
                        "missing rotation DOF for member {} station {}",
                        member.id, element_index
                    )
                })?;
            let theta_j = *rotations
                .get(&(member.id.clone(), element_index + 1))
                .ok_or_else(|| {
                    format!(
                        "missing rotation DOF for member {} station {}",
                        member.id,
                        element_index + 1
                    )
                })?;

            let global_ux_i = mode.shape_global_dofs[ux_i];
            let global_uy_i = mode.shape_global_dofs[uy_i];
            let global_ux_j = mode.shape_global_dofs[ux_j];
            let global_uy_j = mode.shape_global_dofs[uy_j];
            let axial_i = cosine * global_ux_i + sine * global_uy_i;
            let transverse_i = -sine * global_ux_i + cosine * global_uy_i;
            let axial_j = cosine * global_ux_j + sine * global_uy_j;
            let transverse_j = -sine * global_ux_j + cosine * global_uy_j;
            let rotation_i = mode.shape_global_dofs[theta_i];
            let rotation_j = mode.shape_global_dofs[theta_j];

            let xi2 = xi * xi;
            let xi3 = xi2 * xi;
            let h1 = 1.0 - 3.0 * xi2 + 2.0 * xi3;
            let h2 = xi - 2.0 * xi2 + xi3;
            let h3 = 3.0 * xi2 - 2.0 * xi3;
            let h4 = -xi2 + xi3;
            let axial = (1.0 - xi) * axial_i + xi * axial_j;
            let transverse = h1 * transverse_i
                + h2 * element_length * rotation_i
                + h3 * transverse_j
                + h4 * element_length * rotation_j;
            let global_ux = cosine * axial - sine * transverse;
            let global_uy = sine * axial + cosine * transverse;
            if !global_ux.is_finite() || !global_uy.is_finite() {
                return Err("interpolated beam-mode sample is non-finite".to_owned());
            }
            sampled.push(global_ux);
            sampled.push(global_uy);
        }
    }
    Ok(sampled)
}

/// Sign-invariant modal assurance criterion between samples evaluated on the
/// same physical grid. It ranges from 0 (orthogonal sample vectors) to 1
/// (identical up to sign/scale). It is a comparison metric, not a validation
/// oracle; the sampling grid and mode-pairing policy remain part of the request.
pub fn modal_assurance_criterion(left: &[f64], right: &[f64]) -> Check<f64> {
    if left.is_empty() || left.len() != right.len() {
        return Err("MAC vectors must be non-empty and have equal dimensions".to_owned());
    }
    if left.iter().chain(right).any(|value| !value.is_finite()) {
        return Err("MAC inputs contain a non-finite value".to_owned());
    }
    let left_scale = left.iter().map(|value| value.abs()).fold(0.0_f64, f64::max);
    let right_scale = right
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if !left_scale.is_finite()
        || !right_scale.is_finite()
        || left_scale <= 0.0
        || right_scale <= 0.0
    {
        return Err("MAC vectors must have finite positive norm".to_owned());
    }

    // Normalize by the largest absolute component first to avoid overflow or
    // underflow when a valid mode vector carries an arbitrary scale factor.
    let left_normalized = left
        .iter()
        .map(|value| value / left_scale)
        .collect::<Vec<_>>();
    let right_normalized = right
        .iter()
        .map(|value| value / right_scale)
        .collect::<Vec<_>>();
    let left_norm = left_normalized
        .iter()
        .map(|value| value * value)
        .sum::<f64>();
    let right_norm = right_normalized
        .iter()
        .map(|value| value * value)
        .sum::<f64>();
    let dot = left_normalized
        .iter()
        .zip(&right_normalized)
        .map(|(a, b)| a * b)
        .sum::<f64>();
    let mac = (dot * dot / (left_norm * right_norm)).clamp(0.0, 1.0);
    if mac.is_finite() {
        Ok(mac)
    } else {
        Err("MAC result is non-finite".to_owned())
    }
}

fn reduce_matrix(matrix: &[Vec<f64>], free_indices: &[usize]) -> Check<Matrix> {
    if matrix.iter().any(|row| row.len() != matrix.len()) {
        return Err("global matrix must be square".to_owned());
    }
    if free_indices.iter().any(|index| *index >= matrix.len()) {
        return Err("free-DOF map contains an out-of-range index".to_owned());
    }
    Ok(free_indices
        .iter()
        .map(|row| {
            free_indices
                .iter()
                .map(|col| matrix[*row][*col])
                .collect::<Vec<_>>()
        })
        .collect())
}

fn extract_free_shape(mode: &BeamModalMode, free_indices: &[usize]) -> Vec<f64> {
    free_indices
        .iter()
        .map(|index| mode.shape_global_dofs[*index])
        .collect()
}

fn mass_inner_product(mass: &Matrix, left: &[f64], right: &[f64]) -> Check<f64> {
    if left.len() != mass.len()
        || right.len() != mass.len()
        || mass.iter().any(|row| row.len() != mass.len())
    {
        return Err("mass inner-product dimensions do not agree".to_owned());
    }
    let mut result = 0.0_f64;
    for row in 0..mass.len() {
        for col in 0..mass.len() {
            result += left[row] * mass[row][col] * right[col];
        }
    }
    if result.is_finite() {
        Ok(result)
    } else {
        Err("modal mass inner product is non-finite".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use crate::beam_pin_assembly::{
        BeamMemberChain, BeamPinDofKind, BeamPinMember, BeamPinNode,
        assemble_released_joint_beam_chains, assemble_released_joint_pin_truss,
    };

    use super::{
        BeamModeClass, modal_assurance_criterion, sample_beam_chain_mode_on_fixed_grid,
        solve_beam_pin_assembly_modes,
    };

    fn coordinates_for_chain_node(
        node_id: &str,
        nodes: &[BeamPinNode],
        members: &[BeamMemberChain],
        elements_per_member: usize,
    ) -> (f64, f64) {
        if let Some(node) = nodes.iter().find(|node| node.id == node_id) {
            return (node.x_m, node.y_m);
        }
        for member in members {
            let prefix = format!("{}::station::", member.id);
            if let Some(station_text) = node_id.strip_prefix(&prefix) {
                let station_index = station_text.parse::<usize>().expect("numeric station ID");
                let start = nodes
                    .iter()
                    .find(|node| node.id == member.start_node_id)
                    .expect("member start node");
                let end = nodes
                    .iter()
                    .find(|node| node.id == member.end_node_id)
                    .expect("member end node");
                let fraction = station_index as f64 / elements_per_member as f64;
                return (
                    start.x_m + (end.x_m - start.x_m) * fraction,
                    start.y_m + (end.y_m - start.y_m) * fraction,
                );
            }
        }
        panic!("node coordinate not found for {node_id}");
    }

    fn chain_fixture(elements_per_member: usize) -> (Vec<BeamPinNode>, Vec<BeamMemberChain>) {
        let nodes = vec![
            BeamPinNode {
                id: "A".to_owned(),
                x_m: 0.0,
                y_m: 1.0,
                restrain_x: true,
                restrain_y: true,
            },
            BeamPinNode {
                id: "B".to_owned(),
                x_m: 0.0,
                y_m: 0.0,
                restrain_x: true,
                restrain_y: true,
            },
            BeamPinNode {
                id: "C".to_owned(),
                x_m: 1.0,
                y_m: 1.0,
                restrain_x: false,
                restrain_y: false,
            },
        ];
        let members = ["AC", "BC"]
            .into_iter()
            .map(|id| BeamMemberChain {
                id: id.to_owned(),
                start_node_id: if id == "AC" { "A" } else { "B" }.to_owned(),
                end_node_id: "C".to_owned(),
                area_m2: 3.141592653589793e-4,
                youngs_modulus_pa: 210.0e9,
                density_kg_m3: 7800.0,
                second_moment_m4: 7.853981633974483e-9,
                elements_per_member,
            })
            .collect();
        (nodes, members)
    }

    fn fixture() -> crate::beam_pin_assembly::BeamPinAssembly {
        let nodes = vec![
            BeamPinNode {
                id: "A".to_owned(),
                x_m: 0.0,
                y_m: 1.0,
                restrain_x: true,
                restrain_y: true,
            },
            BeamPinNode {
                id: "B".to_owned(),
                x_m: 0.0,
                y_m: 0.0,
                restrain_x: true,
                restrain_y: true,
            },
            BeamPinNode {
                id: "C".to_owned(),
                x_m: 1.0,
                y_m: 1.0,
                restrain_x: false,
                restrain_y: false,
            },
        ];
        let members = vec![
            BeamPinMember {
                id: "AC".to_owned(),
                start_node_id: "A".to_owned(),
                end_node_id: "C".to_owned(),
                area_m2: 3.141592653589793e-4,
                youngs_modulus_pa: 210.0e9,
                density_kg_m3: 7800.0,
                second_moment_m4: 7.853981633974483e-9,
            },
            BeamPinMember {
                id: "BC".to_owned(),
                start_node_id: "B".to_owned(),
                end_node_id: "C".to_owned(),
                area_m2: 3.141592653589793e-4,
                youngs_modulus_pa: 210.0e9,
                density_kg_m3: 7800.0,
                second_moment_m4: 7.853981633974483e-9,
            },
        ];
        assemble_released_joint_pin_truss(&nodes, &members).expect("beam pin assembly fixture")
    }

    #[test]
    fn independent_diagonal_generalized_eigenproblem_matches_exact_roots() {
        // K phi = lambda M phi, with a diagonal system whose exact roots are
        // lambda=[4,9]. This isolates eigenvalue ordering and mass normalization
        // from the frame-element assembly and any external solver.
        let stiffness = vec![vec![4.0, 0.0], vec![0.0, 36.0]];
        let mass = vec![vec![1.0, 0.0], vec![0.0, 4.0]];
        let eigenpairs = crate::modal_bar::generalized_symmetric_eigenpairs(&stiffness, &mass)
            .expect("diagonal generalized eigenproblem");
        assert_eq!(eigenpairs.len(), 2);
        assert!((eigenpairs[0].0 - 4.0).abs() < 1.0e-12);
        assert!((eigenpairs[1].0 - 9.0).abs() < 1.0e-12);
        assert!(eigenpairs[0].0 < eigenpairs[1].0);

        for (eigenvalue, shape) in &eigenpairs {
            assert!(
                crate::modal_bar::normalized_eigen_residual(&stiffness, &mass, *eigenvalue, shape)
                    .expect("finite residual")
                    < 1.0e-12
            );
            assert!(
                (crate::modal_bar::quadratic_form(&mass, shape).expect("modal norm") - 1.0).abs()
                    < 1.0e-12
            );
        }
    }

    #[test]
    fn modes_preserve_all_free_dofs_and_have_small_residuals() {
        let assembly = fixture();
        let result = solve_beam_pin_assembly_modes(&assembly).expect("modal result");
        assert_eq!(result.modes.len(), assembly.free_dof_indices.len());
        assert!(
            result
                .modes
                .iter()
                .all(|mode| mode.class == BeamModeClass::Elastic)
        );
        assert!(result.modes.windows(2).all(|pair| {
            pair[0].frequency_hz.expect("elastic mode")
                < pair[1].frequency_hz.expect("elastic mode")
        }));
        assert!(result.maximum_normalized_eigen_residual < 1.0e-10);
        assert!(result.maximum_mass_norm_residual < 1.0e-10);
        assert!(result.maximum_off_diagonal_mass_orthogonality_residual < 1.0e-10);
        for mode in result.modes {
            assert!(mode.shape_global_dofs.iter().all(|value| value.is_finite()));
            for fixed_dof in &assembly.fixed_dof_indices {
                assert_eq!(mode.shape_global_dofs[*fixed_dof], 0.0);
            }
        }
    }

    #[test]
    fn mac_is_sign_invariant_and_fail_closed_on_incompatible_vectors() {
        assert!(
            (modal_assurance_criterion(&[1.0, -2.0, 3.0], &[-2.0, 4.0, -6.0])
                .expect("sign invariant MAC")
                - 1.0)
                .abs()
                < 1.0e-14
        );
        assert!(
            modal_assurance_criterion(&[1.0, 0.0], &[0.0, 1.0])
                .expect("orthogonal MAC")
                .abs()
                < 1.0e-14
        );
        assert!(modal_assurance_criterion(&[1.0], &[1.0, 2.0]).is_err());
        assert!(modal_assurance_criterion(&[0.0, 0.0], &[1.0, 0.0]).is_err());
        assert!(
            (modal_assurance_criterion(&[1.0e200, -2.0e200], &[-1.0e-200, 2.0e-200])
                .expect("scale-safe MAC")
                - 1.0)
                .abs()
                < 1.0e-14
        );
    }

    #[test]
    fn fixed_grid_mode_sampling_is_invariant_for_rigid_translation_across_meshes() {
        let (nodes_1, members_1) = chain_fixture(1);
        let assembly_1 = assemble_released_joint_beam_chains(&nodes_1, &members_1)
            .expect("coarse member-chain assembly");
        let (nodes_4, members_4) = chain_fixture(4);
        let assembly_4 = assemble_released_joint_beam_chains(&nodes_4, &members_4)
            .expect("refined member-chain assembly");

        let make_translation = |assembly: &crate::beam_pin_assembly::BeamPinAssembly| {
            let mut shape = vec![0.0; assembly.stiffness_global.len()];
            for descriptor in &assembly.dof_descriptors {
                match &descriptor.kind {
                    crate::beam_pin_assembly::BeamPinDofKind::NodeTranslationX { .. } => {
                        shape[descriptor.index] = 0.25;
                    }
                    crate::beam_pin_assembly::BeamPinDofKind::NodeTranslationY { .. } => {
                        shape[descriptor.index] = -0.5;
                    }
                    _ => {}
                }
            }
            super::BeamModalMode {
                eigenvalue_rad_s_squared: 1.0,
                frequency_hz: Some(1.0 / (2.0 * std::f64::consts::PI)),
                class: BeamModeClass::Elastic,
                shape_global_dofs: shape,
                normalized_eigen_residual: 0.0,
                mass_norm_residual: 0.0,
            }
        };
        let mode_1 = make_translation(&assembly_1);
        let mode_4 = make_translation(&assembly_4);
        let samples_1 =
            sample_beam_chain_mode_on_fixed_grid(&nodes_1, &members_1, &assembly_1, &mode_1, 17)
                .expect("coarse samples");
        let samples_4 =
            sample_beam_chain_mode_on_fixed_grid(&nodes_4, &members_4, &assembly_4, &mode_4, 17)
                .expect("refined samples");
        assert_eq!(samples_1.len(), 2 * 17 * 2);
        assert_eq!(samples_1, samples_4);
        assert!(
            (modal_assurance_criterion(&samples_1, &samples_4).expect("same-grid MAC") - 1.0).abs()
                < 1.0e-14
        );
    }

    #[test]
    fn fixed_grid_mode_sampling_is_invariant_for_rigid_rotation_across_meshes() {
        let (nodes_1, members_1) = chain_fixture(1);
        let assembly_1 = assemble_released_joint_beam_chains(&nodes_1, &members_1)
            .expect("coarse member-chain assembly");
        let (nodes_4, members_4) = chain_fixture(4);
        let assembly_4 = assemble_released_joint_beam_chains(&nodes_4, &members_4)
            .expect("refined member-chain assembly");
        let angular_rate = 0.015_f64;

        let make_rigid_rotation = |assembly: &crate::beam_pin_assembly::BeamPinAssembly,
                                   nodes: &[BeamPinNode],
                                   members: &[BeamMemberChain],
                                   elements_per_member: usize| {
            let mut shape = vec![0.0; assembly.stiffness_global.len()];
            for descriptor in &assembly.dof_descriptors {
                match &descriptor.kind {
                    BeamPinDofKind::NodeTranslationX { node_id } => {
                        let (_, y) = coordinates_for_chain_node(
                            node_id,
                            nodes,
                            members,
                            elements_per_member,
                        );
                        shape[descriptor.index] = -angular_rate * y;
                    }
                    BeamPinDofKind::NodeTranslationY { node_id } => {
                        let (x, _) = coordinates_for_chain_node(
                            node_id,
                            nodes,
                            members,
                            elements_per_member,
                        );
                        shape[descriptor.index] = angular_rate * x;
                    }
                    BeamPinDofKind::MemberEndRotation { .. }
                    | BeamPinDofKind::MemberInteriorRotation { .. } => {
                        shape[descriptor.index] = angular_rate;
                    }
                }
            }
            super::BeamModalMode {
                eigenvalue_rad_s_squared: 1.0,
                frequency_hz: Some(1.0 / (2.0 * std::f64::consts::PI)),
                class: BeamModeClass::Elastic,
                shape_global_dofs: shape,
                normalized_eigen_residual: 0.0,
                mass_norm_residual: 0.0,
            }
        };
        let mode_1 = make_rigid_rotation(&assembly_1, &nodes_1, &members_1, 1);
        let mode_4 = make_rigid_rotation(&assembly_4, &nodes_4, &members_4, 4);
        let samples_1 =
            sample_beam_chain_mode_on_fixed_grid(&nodes_1, &members_1, &assembly_1, &mode_1, 17)
                .expect("coarse rigid-rotation samples");
        let samples_4 =
            sample_beam_chain_mode_on_fixed_grid(&nodes_4, &members_4, &assembly_4, &mode_4, 17)
                .expect("refined rigid-rotation samples");
        assert_eq!(samples_1.len(), samples_4.len());
        for (coarse, refined) in samples_1.iter().zip(&samples_4) {
            assert!((coarse - refined).abs() < 1.0e-12);
        }
        assert!(
            (modal_assurance_criterion(&samples_1, &samples_4).expect("rigid-rotation MAC") - 1.0)
                .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn cantilever_bending_eigenvalues_match_independent_closed_form_two_by_two_oracle() {
        use crate::beam_element::euler_bernoulli_frame_element;

        let length_m = 2.0_f64;
        let area_m2 = 0.004_f64;
        let youngs_modulus_pa = 70.0e9_f64;
        let density_kg_m3 = 2700.0_f64;
        let second_moment_m4 = 2.0e-6_f64;
        let element = euler_bernoulli_frame_element(
            length_m,
            area_m2,
            youngs_modulus_pa,
            density_kg_m3,
            second_moment_m4,
        )
        .expect("cantilever beam element");

        // The root DOFs are fixed. Select only the free transverse-tip
        // displacement and rotation; axial motion decouples from bending.
        let indices = [4_usize, 5_usize];
        let stiffness = indices
            .iter()
            .map(|row| {
                indices
                    .iter()
                    .map(|col| element.stiffness_local[*row][*col])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mass = indices
            .iter()
            .map(|row| {
                indices
                    .iter()
                    .map(|col| element.mass_local[*row][*col])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        // Independent quadratic formula for det(K - lambda M) = 0:
        // det(M)*lambda^2 - trace(adj(M)*K)*lambda + det(K) = 0.
        let det_m = mass[0][0] * mass[1][1] - mass[0][1] * mass[1][0];
        let trace_adj_m_k = mass[1][1] * stiffness[0][0] + mass[0][0] * stiffness[1][1]
            - mass[0][1] * stiffness[1][0]
            - mass[1][0] * stiffness[0][1];
        let det_k = stiffness[0][0] * stiffness[1][1] - stiffness[0][1] * stiffness[1][0];
        assert!(det_m > 0.0 && det_k > 0.0);
        let discriminant = trace_adj_m_k * trace_adj_m_k - 4.0 * det_m * det_k;
        assert!(discriminant > 0.0);
        let root = discriminant.sqrt();
        let expected = [
            (trace_adj_m_k - root) / (2.0 * det_m),
            (trace_adj_m_k + root) / (2.0 * det_m),
        ];

        let actual = crate::modal_bar::generalized_symmetric_eigenpairs(&stiffness, &mass)
            .expect("cantilever generalized eigenproblem");
        assert_eq!(actual.len(), 2);
        for index in 0..2 {
            let scale = expected[index].abs().max(1.0);
            assert!(
                (actual[index].0 - expected[index]).abs() / scale < 1.0e-10,
                "cantilever bending eigenvalue {} differs: actual={:.17e}, expected={:.17e}",
                index + 1,
                actual[index].0,
                expected[index]
            );
            assert!(
                crate::modal_bar::normalized_eigen_residual(
                    &stiffness,
                    &mass,
                    actual[index].0,
                    &actual[index].1
                )
                .expect("cantilever mode residual")
                    < 1.0e-10
            );
            assert!(
                (crate::modal_bar::quadratic_form(&mass, &actual[index].1)
                    .expect("cantilever modal mass")
                    - 1.0)
                    .abs()
                    < 1.0e-10
            );
        }
    }

    #[test]
    fn refined_beam_chains_preserve_modes_and_diagnostics_at_multiple_mesh_levels() {
        let nodes = vec![
            BeamPinNode {
                id: "A".to_owned(),
                x_m: 0.0,
                y_m: 1.0,
                restrain_x: true,
                restrain_y: true,
            },
            BeamPinNode {
                id: "B".to_owned(),
                x_m: 0.0,
                y_m: 0.0,
                restrain_x: true,
                restrain_y: true,
            },
            BeamPinNode {
                id: "C".to_owned(),
                x_m: 1.0,
                y_m: 1.0,
                restrain_x: false,
                restrain_y: false,
            },
        ];

        for elements_per_member in [1_usize, 2, 4, 8] {
            let members = ["AC", "BC"]
                .into_iter()
                .map(|id| BeamMemberChain {
                    id: id.to_owned(),
                    start_node_id: if id == "AC" { "A" } else { "B" }.to_owned(),
                    end_node_id: "C".to_owned(),
                    area_m2: 3.141592653589793e-4,
                    youngs_modulus_pa: 210.0e9,
                    density_kg_m3: 7800.0,
                    second_moment_m4: 7.853981633974483e-9,
                    elements_per_member,
                })
                .collect::<Vec<_>>();
            let assembly =
                assemble_released_joint_beam_chains(&nodes, &members).expect("refined assembly");
            let result =
                solve_beam_pin_assembly_modes(&assembly).expect("refined mesh modal result");

            assert_eq!(assembly.free_dof_indices.len(), 6 * elements_per_member);
            assert_eq!(result.modes.len(), assembly.free_dof_indices.len());
            assert!(
                result
                    .modes
                    .iter()
                    .all(|mode| mode.class == BeamModeClass::Elastic)
            );
            assert!(result.modes.windows(2).all(|pair| {
                pair[0].frequency_hz.expect("elastic mode")
                    < pair[1].frequency_hz.expect("elastic mode")
            }));
            assert!(result.maximum_normalized_eigen_residual < 1.0e-9);
            assert!(result.maximum_mass_norm_residual < 1.0e-9);
            assert!(result.maximum_off_diagonal_mass_orthogonality_residual < 1.0e-9);
        }
    }

    #[test]
    fn negative_eigenvalues_are_reported_as_unstable_not_silently_dropped() {
        let mut assembly = fixture();
        let last_global = *assembly.free_dof_indices.last().expect("free DOF");
        // Create a synthetic negative-stiffness direction while preserving a
        // positive mass matrix. This is a classification test, not a physical
        // post-buckling or stability calculation.
        for row in 0..assembly.stiffness_global.len() {
            assembly.stiffness_global[row][last_global] = 0.0;
            assembly.stiffness_global[last_global][row] = 0.0;
        }
        assembly.stiffness_global[last_global][last_global] = -1.0e8;

        let result = solve_beam_pin_assembly_modes(&assembly).expect("classified unstable mode");
        assert_eq!(result.modes.len(), assembly.free_dof_indices.len());
        assert!(result.modes.iter().any(|mode| {
            mode.class == BeamModeClass::NegativeOrUnstable && mode.frequency_hz.is_none()
        }));
    }

    #[test]
    fn zero_eigenvalues_are_reported_not_silently_dropped() {
        let mut assembly = fixture();
        let free = assembly.free_dof_indices.clone();
        let last_global = *free.last().expect("free DOF");
        // Introduce a synthetic mechanism in a cloned matrix by zeroing one
        // free row/column; the remaining DOFs retain the positive spectral scale.
        for row in 0..assembly.stiffness_global.len() {
            assembly.stiffness_global[row][last_global] = 0.0;
            assembly.stiffness_global[last_global][row] = 0.0;
        }
        let result = solve_beam_pin_assembly_modes(&assembly).expect("classified mechanism");
        assert_eq!(result.modes.len(), assembly.free_dof_indices.len());
        assert!(result.modes.iter().any(|mode| {
            mode.class == BeamModeClass::NearZeroOrMechanism && mode.frequency_hz.is_none()
        }));
    }
}
