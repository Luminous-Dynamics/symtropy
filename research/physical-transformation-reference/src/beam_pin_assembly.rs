// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic global assembly for planar Euler-Bernoulli members connected
//! by ideal pins. Translational DOFs are shared at each joint; every member
//! end receives its own private rotational DOF, so adjacent member rotations
//! are never accidentally tied together by a common frame-node rotation.
//!
//! This is an assembly reference only. It does not solve the eigenproblem,
//! decide a structure is safe, include geometric nonlinearity, or perform a
//! complete NAFEMS NBC05 beam-model reproduction.

use crate::Check;
use crate::beam_element::{Matrix6, euler_bernoulli_frame_element, transform_element_to_global};
use std::collections::{BTreeMap, BTreeSet};

const MAX_NODES: usize = 128;
const MAX_MEMBERS: usize = 512;

/// A planar joint. Only translational restraints are shared node conditions.
/// Rotations are deliberately not stored at the node for a pin-jointed model.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamPinNode {
    pub id: String,
    pub x_m: f64,
    pub y_m: f64,
    pub restrain_x: bool,
    pub restrain_y: bool,
}

/// A straight uniform Euler-Bernoulli member with an ideal pin at each end.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamPinMember {
    pub id: String,
    pub start_node_id: String,
    pub end_node_id: String,
    pub area_m2: f64,
    pub youngs_modulus_pa: f64,
    pub density_kg_m3: f64,
    pub second_moment_m4: f64,
}

/// Kind of a global DOF. Member-end rotations are intentionally keyed by both
/// member identity and endpoint, not by node identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BeamPinDofKind {
    NodeTranslationX { node_id: String },
    NodeTranslationY { node_id: String },
    MemberEndRotation { member_id: String, at_start: bool },
    /// A rotational DOF shared by adjacent finite elements of one physical member.
    MemberInteriorRotation { member_id: String, interior_node_index: usize },
}

/// A globally indexed generalized coordinate, preserved in each receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BeamPinDofDescriptor {
    pub index: usize,
    pub kind: BeamPinDofKind,
}

/// Deterministic dense global matrices and explicit restraint/DOF maps.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamPinAssembly {
    pub ordered_node_ids: Vec<String>,
    pub ordered_member_ids: Vec<String>,
    pub dof_descriptors: Vec<BeamPinDofDescriptor>,
    pub stiffness_global: Vec<Vec<f64>>,
    pub mass_global: Vec<Vec<f64>>,
    pub translational_mass_global: Vec<Vec<f64>>,
    pub rotary_inertia_mass_global: Vec<Vec<f64>>,
    pub fixed_dof_indices: Vec<usize>,
    pub free_dof_indices: Vec<usize>,
}

/// Canonical request for deterministic, receipt-bound released-joint assembly.
/// Inputs are validated first and then sorted by stable ID; storage-order
/// permutations therefore produce identical UTF-8 bytes.
pub fn canonical_assembly_request(
    nodes: &[BeamPinNode],
    members: &[BeamPinMember],
) -> Check<String> {
    // Fail closed for malformed topology or physical inputs before hashing.
    assemble_released_joint_pin_truss(nodes, members)?;

    let mut ordered_nodes = nodes.to_vec();
    ordered_nodes.sort_by(|left, right| left.id.cmp(&right.id));
    let mut ordered_members = members.to_vec();
    ordered_members.sort_by(|left, right| left.id.cmp(&right.id));

    let mut fields = vec![
        "schema=beam-pin-assembly-request-v0.1".to_owned(),
        "fixture=PHYS-BEAM-PIN-001".to_owned(),
    ];
    for node in ordered_nodes {
        fields.push(format!(
            "node|{}|{}|{}|{}|{}",
            node.id, node.x_m, node.y_m, node.restrain_x, node.restrain_y
        ));
    }
    for member in ordered_members {
        fields.push(format!(
            "member|{}|{}|{}|{}|{}|{}|{}",
            member.id,
            member.start_node_id,
            member.end_node_id,
            member.area_m2,
            member.youngs_modulus_pa,
            member.density_kg_m3,
            member.second_moment_m4
        ));
    }
    Ok(fields.join("|"))
}

/// Assemble the global matrices of a planar beam model with ideal pin joints.
///
/// Nodes and members are sorted by stable ID before DOF assignment, so
/// permutation of input storage does not alter the resulting matrices or
/// descriptors. Translation is shared per node. Each of the two rotations for
/// each member is distinct/private; equilibrium of that unshared rotation
/// yields the ideal moment-release condition. Supports restrain translations
/// only, consistent with the pin-support abstraction.
pub fn assemble_released_joint_pin_truss(
    nodes: &[BeamPinNode],
    members: &[BeamPinMember],
) -> Check<BeamPinAssembly> {
    if nodes.is_empty() || nodes.len() > MAX_NODES {
        return Err(format!("node count must be between 1 and {MAX_NODES}"));
    }
    if members.is_empty() || members.len() > MAX_MEMBERS {
        return Err(format!("member count must be between 1 and {MAX_MEMBERS}"));
    }

    let mut ordered_nodes = nodes.to_vec();
    ordered_nodes.sort_by(|left, right| left.id.cmp(&right.id));
    let mut node_by_id = BTreeMap::new();
    for (index, node) in ordered_nodes.iter().enumerate() {
        validate_identifier("node ID", &node.id)?;
        if !node.x_m.is_finite() || !node.y_m.is_finite() {
            return Err(format!("node {} has non-finite coordinates", node.id));
        }
        if node_by_id.insert(node.id.as_str(), index).is_some() {
            return Err(format!("duplicate node ID: {}", node.id));
        }
    }

    let mut ordered_members = members.to_vec();
    ordered_members.sort_by(|left, right| left.id.cmp(&right.id));
    let mut member_ids = BTreeSet::new();
    let mut used_nodes = BTreeSet::new();
    for member in &ordered_members {
        validate_identifier("member ID", &member.id)?;
        validate_identifier("start node ID", &member.start_node_id)?;
        validate_identifier("end node ID", &member.end_node_id)?;
        if !member_ids.insert(member.id.as_str()) {
            return Err(format!("duplicate member ID: {}", member.id));
        }
        if member.start_node_id == member.end_node_id {
            return Err(format!(
                "member {} has the same start and end node",
                member.id
            ));
        }
        if !node_by_id.contains_key(member.start_node_id.as_str()) {
            return Err(format!(
                "member {} references unknown start node {}",
                member.id, member.start_node_id
            ));
        }
        if !node_by_id.contains_key(member.end_node_id.as_str()) {
            return Err(format!(
                "member {} references unknown end node {}",
                member.id, member.end_node_id
            ));
        }
        used_nodes.insert(member.start_node_id.as_str());
        used_nodes.insert(member.end_node_id.as_str());
    }
    for node in &ordered_nodes {
        if !used_nodes.contains(node.id.as_str()) {
            return Err(format!("isolated node is not allowed: {}", node.id));
        }
    }

    let translation_dof_count = ordered_nodes.len() * 2;
    let dof_count = translation_dof_count + ordered_members.len() * 2;
    let mut descriptors = Vec::with_capacity(dof_count);
    let mut fixed = Vec::new();

    for (index, node) in ordered_nodes.iter().enumerate() {
        let x_index = 2 * index;
        let y_index = x_index + 1;
        descriptors.push(BeamPinDofDescriptor {
            index: x_index,
            kind: BeamPinDofKind::NodeTranslationX {
                node_id: node.id.clone(),
            },
        });
        descriptors.push(BeamPinDofDescriptor {
            index: y_index,
            kind: BeamPinDofKind::NodeTranslationY {
                node_id: node.id.clone(),
            },
        });
        if node.restrain_x {
            fixed.push(x_index);
        }
        if node.restrain_y {
            fixed.push(y_index);
        }
    }

    let mut global_stiffness = vec![vec![0.0; dof_count]; dof_count];
    let mut global_mass = vec![vec![0.0; dof_count]; dof_count];
    let mut global_translational_mass = vec![vec![0.0; dof_count]; dof_count];
    let mut global_rotary_mass = vec![vec![0.0; dof_count]; dof_count];

    for (member_index, member) in ordered_members.iter().enumerate() {
        let start = &ordered_nodes[*node_by_id
            .get(member.start_node_id.as_str())
            .expect("validated start node")];
        let end = &ordered_nodes[*node_by_id
            .get(member.end_node_id.as_str())
            .expect("validated end node")];

        let local = euler_bernoulli_frame_element(
            (end.x_m - start.x_m).hypot(end.y_m - start.y_m),
            member.area_m2,
            member.youngs_modulus_pa,
            member.density_kg_m3,
            member.second_moment_m4,
        )?;
        let global = transform_element_to_global(&local, end.x_m - start.x_m, end.y_m - start.y_m)?;

        let start_node_index = *node_by_id
            .get(member.start_node_id.as_str())
            .expect("validated start node");
        let end_node_index = *node_by_id
            .get(member.end_node_id.as_str())
            .expect("validated end node");
        let private_rotation_base = translation_dof_count + 2 * member_index;
        let dof_map = [
            2 * start_node_index,
            2 * start_node_index + 1,
            private_rotation_base,
            2 * end_node_index,
            2 * end_node_index + 1,
            private_rotation_base + 1,
        ];

        descriptors.push(BeamPinDofDescriptor {
            index: private_rotation_base,
            kind: BeamPinDofKind::MemberEndRotation {
                member_id: member.id.clone(),
                at_start: true,
            },
        });
        descriptors.push(BeamPinDofDescriptor {
            index: private_rotation_base + 1,
            kind: BeamPinDofKind::MemberEndRotation {
                member_id: member.id.clone(),
                at_start: false,
            },
        });

        assemble_element_matrix(&mut global_stiffness, &global.stiffness_global, &dof_map);
        assemble_element_matrix(&mut global_mass, &global.mass_global, &dof_map);
        assemble_element_matrix(
            &mut global_translational_mass,
            &global.translational_mass_global,
            &dof_map,
        );
        assemble_element_matrix(
            &mut global_rotary_mass,
            &global.rotary_inertia_mass_global,
            &dof_map,
        );
    }

    descriptors.sort_by_key(|descriptor| descriptor.index);
    let fixed_set: BTreeSet<usize> = fixed.iter().copied().collect();
    let free = (0..dof_count)
        .filter(|index| !fixed_set.contains(index))
        .collect::<Vec<_>>();

    for matrix in [
        &global_stiffness,
        &global_mass,
        &global_translational_mass,
        &global_rotary_mass,
    ] {
        ensure_finite_global_matrix(matrix)?;
        ensure_symmetric(matrix, 1.0e-10)?;
    }

    Ok(BeamPinAssembly {
        ordered_node_ids: ordered_nodes.into_iter().map(|node| node.id).collect(),
        ordered_member_ids: ordered_members
            .into_iter()
            .map(|member| member.id)
            .collect(),
        dof_descriptors: descriptors,
        stiffness_global: global_stiffness,
        mass_global: global_mass,
        translational_mass_global: global_translational_mass,
        rotary_inertia_mass_global: global_rotary_mass,
        fixed_dof_indices: fixed,
        free_dof_indices: free,
    })
}

fn assemble_element_matrix(global: &mut [Vec<f64>], element: &Matrix6, dof_map: &[usize; 6]) {
    for row in 0..6 {
        for col in 0..6 {
            global[dof_map[row]][dof_map[col]] += element[row][col];
        }
    }
}

fn validate_identifier(name: &str, value: &str) -> Check<()> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().any(char::is_control)
        || value.contains('|')
        || value.contains('\n')
        || value.contains('\r')
    {
        Err(format!("{name} is empty or non-canonical"))
    } else {
        Ok(())
    }
}

fn ensure_finite_global_matrix(matrix: &[Vec<f64>]) -> Check<()> {
    if matrix.iter().flatten().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err("global assembly contains a non-finite matrix entry".to_owned())
    }
}

fn ensure_symmetric(matrix: &[Vec<f64>], tolerance: f64) -> Check<()> {
    for row in 0..matrix.len() {
        for col in 0..matrix.len() {
            let left = matrix[row][col];
            let right = matrix[col][row];
            let scale = left.abs().max(right.abs()).max(1.0);
            if (left - right).abs() > tolerance * scale {
                return Err(format!("global matrix is not symmetric at ({row}, {col})"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        BeamPinDofKind, BeamPinMember, BeamPinNode, assemble_released_joint_pin_truss,
        canonical_assembly_request,
    };

    fn nodes() -> Vec<BeamPinNode> {
        vec![
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
        ]
    }

    fn members() -> Vec<BeamPinMember> {
        vec![
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
        ]
    }

    #[test]
    fn pin_joints_share_translations_but_not_member_rotations() {
        let result = assemble_released_joint_pin_truss(&nodes(), &members())
            .expect("valid released-joint beam assembly");
        assert_eq!(result.ordered_node_ids, vec!["A", "B", "C"]);
        assert_eq!(result.ordered_member_ids, vec!["AC", "BC"]);
        assert_eq!(result.stiffness_global.len(), 10);
        assert_eq!(result.mass_global.len(), 10);
        assert_eq!(result.fixed_dof_indices, vec![0, 1, 2, 3]);
        assert_eq!(result.free_dof_indices.len(), 6);

        let rotation_descriptors = result
            .dof_descriptors
            .iter()
            .filter_map(|descriptor| match &descriptor.kind {
                BeamPinDofKind::MemberEndRotation {
                    member_id,
                    at_start,
                } => Some((member_id.clone(), *at_start, descriptor.index)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(rotation_descriptors.len(), 4);
        let unique_indices = rotation_descriptors
            .iter()
            .map(|(_, _, index)| *index)
            .collect::<BTreeSet<_>>();
        assert_eq!(unique_indices.len(), 4);
        assert!(rotation_descriptors.iter().all(|(_, _, index)| *index >= 6));
    }

    #[test]
    fn every_member_end_rotation_has_stiffness_and_mass_support() {
        let result = assemble_released_joint_pin_truss(&nodes(), &members())
            .expect("valid released-joint assembly");
        let rotation_indices = result
            .dof_descriptors
            .iter()
            .filter_map(|descriptor| match &descriptor.kind {
                BeamPinDofKind::MemberEndRotation { .. } => Some(descriptor.index),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(rotation_indices.len(), 4);
        assert_eq!(
            rotation_indices
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len(),
            4
        );
        for index in rotation_indices {
            assert!(result.stiffness_global[index][index] > 0.0);
            assert!(result.mass_global[index][index] > 0.0);
        }
    }

    #[test]
    fn assembly_is_invariant_to_input_storage_order() {
        let first = assemble_released_joint_pin_truss(&nodes(), &members()).expect("ordered input");
        let mut reversed_nodes = nodes();
        reversed_nodes.reverse();
        let mut reversed_members = members();
        reversed_members.reverse();
        let second = assemble_released_joint_pin_truss(&reversed_nodes, &reversed_members)
            .expect("reversed input");
        assert_eq!(first, second);
    }

    #[test]
    fn global_matrices_are_symmetric_and_mass_terms_remain_separable() {
        let result = assemble_released_joint_pin_truss(&nodes(), &members())
            .expect("valid released-joint assembly");
        assert_eq!(result.stiffness_global.len(), result.mass_global.len());
        assert_eq!(
            result.mass_global.len(),
            result.translational_mass_global.len()
        );
        assert_eq!(
            result.mass_global.len(),
            result.rotary_inertia_mass_global.len()
        );

        for row in 0..result.mass_global.len() {
            for col in 0..result.mass_global.len() {
                let scale_k = result.stiffness_global[row][col]
                    .abs()
                    .max(result.stiffness_global[col][row].abs())
                    .max(1.0);
                assert!(
                    (result.stiffness_global[row][col] - result.stiffness_global[col][row]).abs()
                        < 1.0e-10 * scale_k
                );
                assert!(
                    (result.mass_global[row][col] - result.mass_global[col][row]).abs() < 1.0e-10
                );
                assert!(
                    (result.mass_global[row][col]
                        - result.translational_mass_global[row][col]
                        - result.rotary_inertia_mass_global[row][col])
                        .abs()
                        < 1.0e-12
                );
            }
        }
    }

    #[test]
    fn canonical_request_matches_frozen_fixture_and_input_permutation() {
        let first =
            canonical_assembly_request(&nodes(), &members()).expect("canonical valid request");
        assert_eq!(
            first,
            include_str!("../fixtures/beam_pin_assembly_request_v0_1.txt").trim_end_matches('\n')
        );

        let mut reversed_nodes = nodes();
        reversed_nodes.reverse();
        let mut reversed_members = members();
        reversed_members.reverse();
        assert_eq!(
            canonical_assembly_request(&reversed_nodes, &reversed_members)
                .expect("canonical reversed request"),
            first
        );
    }

    fn solve_small_dense(mut matrix: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Vec<f64> {
        let n = rhs.len();
        assert_eq!(matrix.len(), n);
        let matrix_scale = matrix
            .iter()
            .flatten()
            .map(|value| value.abs())
            .fold(0.0_f64, f64::max);
        assert!(matrix_scale > 0.0);

        for pivot_col in 0..n {
            let pivot_row = (pivot_col..n)
                .max_by(|left, right| {
                    matrix[*left][pivot_col]
                        .abs()
                        .total_cmp(&matrix[*right][pivot_col].abs())
                })
                .expect("pivot row exists");
            assert!(
                matrix[pivot_row][pivot_col].abs() > 1.0e-13 * matrix_scale,
                "test oracle encountered a singular rotational block"
            );
            matrix.swap(pivot_col, pivot_row);
            rhs.swap(pivot_col, pivot_row);

            let pivot = matrix[pivot_col][pivot_col];
            for row in (pivot_col + 1)..n {
                let factor = matrix[row][pivot_col] / pivot;
                matrix[row][pivot_col] = 0.0;
                for col in (pivot_col + 1)..n {
                    matrix[row][col] -= factor * matrix[pivot_col][col];
                }
                rhs[row] -= factor * rhs[pivot_col];
            }
        }

        let mut solution = vec![0.0; n];
        for row in (0..n).rev() {
            let known = ((row + 1)..n)
                .map(|col| matrix[row][col] * solution[col])
                .sum::<f64>();
            solution[row] = (rhs[row] - known) / matrix[row][row];
        }
        solution
    }

    #[test]
    fn condensing_private_pin_rotations_recovers_independent_axial_truss_stiffness() {
        let input_nodes = nodes();
        let input_members = members();
        let assembly = assemble_released_joint_pin_truss(&input_nodes, &input_members)
            .expect("valid released-joint assembly");
        let translation_dofs = assembly.ordered_node_ids.len() * 2;
        let rotation_dofs = assembly.stiffness_global.len() - translation_dofs;
        assert_eq!(translation_dofs, 6);
        assert_eq!(rotation_dofs, 4);

        // Independent Schur complement: K_condensed = K_tt - K_tr K_rr^-1 K_rt.
        // The private member-end rotations are free, so zero end moments must
        // leave only each bar's EA/L axial centreline response.
        let k_tt = (0..translation_dofs)
            .map(|row| {
                (0..translation_dofs)
                    .map(|col| assembly.stiffness_global[row][col])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let k_rr = (0..rotation_dofs)
            .map(|row| {
                (0..rotation_dofs)
                    .map(|col| {
                        assembly.stiffness_global[translation_dofs + row][translation_dofs + col]
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mut condensed = k_tt;

        for column in 0..translation_dofs {
            let rhs = (0..rotation_dofs)
                .map(|row| assembly.stiffness_global[translation_dofs + row][column])
                .collect::<Vec<_>>();
            let rotation_response = solve_small_dense(k_rr.clone(), rhs);
            for row in 0..translation_dofs {
                let correction = (0..rotation_dofs)
                    .map(|rotation| {
                        assembly.stiffness_global[row][translation_dofs + rotation]
                            * rotation_response[rotation]
                    })
                    .sum::<f64>();
                condensed[row][column] -= correction;
            }
        }

        // Independent direct axial-bar stiffness assembly; no beam matrices or
        // assembled beam translational blocks are reused for this oracle.
        let mut axial_oracle = vec![vec![0.0; translation_dofs]; translation_dofs];
        for member in &input_members {
            let start = input_nodes
                .iter()
                .find(|node| node.id == member.start_node_id)
                .expect("oracle start node");
            let end = input_nodes
                .iter()
                .find(|node| node.id == member.end_node_id)
                .expect("oracle end node");
            let length = (end.x_m - start.x_m).hypot(end.y_m - start.y_m);
            let cosine = (end.x_m - start.x_m) / length;
            let sine = (end.y_m - start.y_m) / length;
            let factor = member.youngs_modulus_pa * member.area_m2 / length;
            let c2 = cosine * cosine;
            let s2 = sine * sine;
            let cs = cosine * sine;
            let block = [
                [c2, cs, -c2, -cs],
                [cs, s2, -cs, -s2],
                [-c2, -cs, c2, cs],
                [-cs, -s2, cs, s2],
            ];
            let start_index = assembly
                .ordered_node_ids
                .iter()
                .position(|id| id == &member.start_node_id)
                .expect("ordered oracle start node");
            let end_index = assembly
                .ordered_node_ids
                .iter()
                .position(|id| id == &member.end_node_id)
                .expect("ordered oracle end node");
            let dof_map = [
                2 * start_index,
                2 * start_index + 1,
                2 * end_index,
                2 * end_index + 1,
            ];
            for row in 0..4 {
                for col in 0..4 {
                    axial_oracle[dof_map[row]][dof_map[col]] += factor * block[row][col];
                }
            }
        }

        let scale = axial_oracle
            .iter()
            .flatten()
            .map(|value| value.abs())
            .fold(0.0_f64, f64::max);
        let maximum_relative_matrix_error = condensed
            .iter()
            .flatten()
            .zip(axial_oracle.iter().flatten())
            .map(|(actual, expected)| (actual - expected).abs() / scale)
            .fold(0.0_f64, f64::max);
        assert!(
            maximum_relative_matrix_error < 1.0e-8,
            "released-joint Schur complement differs from independent axial-truss oracle: {maximum_relative_matrix_error:.17e}"
        );
    }

    fn solve_small_dense(mut matrix: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Vec<f64> {
        let n = rhs.len();
        assert_eq!(matrix.len(), n);
        let matrix_scale = matrix
            .iter()
            .flatten()
            .map(|value| value.abs())
            .fold(0.0_f64, f64::max);
        assert!(matrix_scale > 0.0);

        for pivot_col in 0..n {
            let pivot_row = (pivot_col..n)
                .max_by(|left, right| {
                    matrix[*left][pivot_col]
                        .abs()
                        .total_cmp(&matrix[*right][pivot_col].abs())
                })
                .expect("pivot row exists");
            assert!(
                matrix[pivot_row][pivot_col].abs() > 1.0e-13 * matrix_scale,
                "test oracle encountered a singular rotational block"
            );
            matrix.swap(pivot_col, pivot_row);
            rhs.swap(pivot_col, pivot_row);

            let pivot = matrix[pivot_col][pivot_col];
            for row in (pivot_col + 1)..n {
                let factor = matrix[row][pivot_col] / pivot;
                matrix[row][pivot_col] = 0.0;
                for col in (pivot_col + 1)..n {
                    matrix[row][col] -= factor * matrix[pivot_col][col];
                }
                rhs[row] -= factor * rhs[pivot_col];
            }
        }

        let mut solution = vec![0.0; n];
        for row in (0..n).rev() {
            let known = ((row + 1)..n)
                .map(|col| matrix[row][col] * solution[col])
                .sum::<f64>();
            solution[row] = (rhs[row] - known) / matrix[row][row];
        }
        solution
    }

    #[test]
    fn condensing_private_pin_rotations_recovers_independent_axial_truss_stiffness() {
        let input_nodes = nodes();
        let input_members = members();
        let assembly = assemble_released_joint_pin_truss(&input_nodes, &input_members)
            .expect("valid released-joint assembly");
        let translation_dofs = assembly.ordered_node_ids.len() * 2;
        let rotation_dofs = assembly.stiffness_global.len() - translation_dofs;
        assert_eq!(translation_dofs, 6);
        assert_eq!(rotation_dofs, 4);

        // Independent Schur complement: K_condensed = K_tt - K_tr K_rr^-1 K_rt.
        // With zero end moments, each pin-ended beam's translational stiffness
        // must reduce to its EA/L axial-centreline contribution.
        let k_tt = (0..translation_dofs)
            .map(|row| {
                (0..translation_dofs)
                    .map(|col| assembly.stiffness_global[row][col])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let k_rr = (0..rotation_dofs)
            .map(|row| {
                (0..rotation_dofs)
                    .map(|col| {
                        assembly.stiffness_global[translation_dofs + row][translation_dofs + col]
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mut condensed = k_tt;

        for column in 0..translation_dofs {
            let rhs = (0..rotation_dofs)
                .map(|row| assembly.stiffness_global[translation_dofs + row][column])
                .collect::<Vec<_>>();
            let rotation_response = solve_small_dense(k_rr.clone(), rhs);
            for row in 0..translation_dofs {
                let correction = (0..rotation_dofs)
                    .map(|rotation| {
                        assembly.stiffness_global[row][translation_dofs + rotation]
                            * rotation_response[rotation]
                    })
                    .sum::<f64>();
                condensed[row][column] -= correction;
            }
        }

        // Independent direct axial-truss oracle; it does not reuse the beam
        // stiffness matrices or any production matrix-assembly block.
        let mut axial_oracle = vec![vec![0.0; translation_dofs]; translation_dofs];
        for member in &input_members {
            let start = input_nodes
                .iter()
                .find(|node| node.id == member.start_node_id)
                .expect("oracle start node");
            let end = input_nodes
                .iter()
                .find(|node| node.id == member.end_node_id)
                .expect("oracle end node");
            let length = (end.x_m - start.x_m).hypot(end.y_m - start.y_m);
            let cosine = (end.x_m - start.x_m) / length;
            let sine = (end.y_m - start.y_m) / length;
            let factor = member.youngs_modulus_pa * member.area_m2 / length;
            let c2 = cosine * cosine;
            let s2 = sine * sine;
            let cs = cosine * sine;
            let block = [
                [c2, cs, -c2, -cs],
                [cs, s2, -cs, -s2],
                [-c2, -cs, c2, cs],
                [-cs, -s2, cs, s2],
            ];
            let start_index = assembly
                .ordered_node_ids
                .iter()
                .position(|id| id == &member.start_node_id)
                .expect("ordered oracle start node");
            let end_index = assembly
                .ordered_node_ids
                .iter()
                .position(|id| id == &member.end_node_id)
                .expect("ordered oracle end node");
            let dof_map = [
                2 * start_index,
                2 * start_index + 1,
                2 * end_index,
                2 * end_index + 1,
            ];
            for row in 0..4 {
                for col in 0..4 {
                    axial_oracle[dof_map[row]][dof_map[col]] += factor * block[row][col];
                }
            }
        }

        let scale = axial_oracle
            .iter()
            .flatten()
            .map(|value| value.abs())
            .fold(0.0_f64, f64::max);
        let maximum_scaled_error = condensed
            .iter()
            .flatten()
            .zip(axial_oracle.iter().flatten())
            .map(|(actual, expected)| (actual - expected).abs() / scale)
            .fold(0.0_f64, f64::max);
        assert!(
            maximum_scaled_error < 1.0e-8,
            "released-joint Schur complement differs from independent axial-truss oracle: {maximum_scaled_error:.17e}"
        );
    }

    #[test]
    fn undeclared_references_and_isolated_nodes_fail_closed() {
        let mut bad_members = members();
        bad_members[0].start_node_id = "MISSING".to_owned();
        assert!(assemble_released_joint_pin_truss(&nodes(), &bad_members).is_err());

        let mut bad_nodes = nodes();
        bad_nodes.push(BeamPinNode {
            id: "D".to_owned(),
            x_m: 2.0,
            y_m: 2.0,
            restrain_x: false,
            restrain_y: false,
        });
        assert!(assemble_released_joint_pin_truss(&bad_nodes, &members()).is_err());
    }
}
