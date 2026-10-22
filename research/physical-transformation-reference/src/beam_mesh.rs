// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic mesh refinement for straight Euler-Bernoulli members joined
//! by ideal pins. Physical joints share translations but retain private
//! member-end rotations; an interior mesh rotation is shared by adjacent
//! elements of its owning physical member.
//!
//! This dense mesh ladder is bounded by MAX_DENSE_BEAM_DOFS. It is not the
//! 200-elements-per-member sparse solver required by the full NBC05 benchmark.

use crate::beam_element::{euler_bernoulli_frame_element, transform_element_to_global, Matrix6};
use crate::beam_pin_assembly::{
    assemble_released_joint_pin_truss, canonical_assembly_request, BeamPinAssembly,
    BeamPinDofDescriptor, BeamPinDofKind, BeamPinMember, BeamPinNode,
};
use crate::Check;
use std::collections::{BTreeMap, BTreeSet};

const MAX_DENSE_BEAM_DOFS: usize = 128;
const MAX_SOURCE_NODES: usize = 128;
const MAX_SOURCE_MEMBERS: usize = 512;

/// Canonical source request plus an explicit element count for every physical
/// member. It remains source-attributed and deterministic under input ordering.
pub fn canonical_refinement_request(
    nodes: &[BeamPinNode],
    members: &[BeamPinMember],
    elements_per_member: usize,
) -> Check<String> {
    validate_refinement_budget(nodes.len(), members.len(), elements_per_member)?;
    let base_request = canonical_assembly_request(nodes, members)?;
    Ok(format!(
        "schema=beam-mesh-request-v0.1\nelements_per_member={elements_per_member}\nbase_request={base_request}"
    ))
}

/// Assemble a uniformly subdivided planar beam model. Each source member is
/// split into elements_per_member straight Euler-Bernoulli elements. Internal
/// mesh points share a translation and one rotation within their own physical
/// member; different members meeting at a source joint do not share rotations.
pub fn assemble_refined_released_joint_pin_truss(
    nodes: &[BeamPinNode],
    members: &[BeamPinMember],
    elements_per_member: usize,
) -> Check<BeamPinAssembly> {
    validate_refinement_budget(nodes.len(), members.len(), elements_per_member)?;

    // This validates source topology, identifiers, material/section domains,
    // connectivity and geometric non-degeneracy before refinement.
    let _validated_source = assemble_released_joint_pin_truss(nodes, members)?;

    let mut ordered_nodes = nodes.to_vec();
    ordered_nodes.sort_by(|left, right| left.id.cmp(&right.id));
    let mut ordered_members = members.to_vec();
    ordered_members.sort_by(|left, right| left.id.cmp(&right.id));
    let original_node_index: BTreeMap<String, usize> = ordered_nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), index))
        .collect();

    let mut mesh_nodes = ordered_nodes
        .iter()
        .map(|node| MeshNode {
            id: node.id.clone(),
            x_m: node.x_m,
            y_m: node.y_m,
            restrain_x: node.restrain_x,
            restrain_y: node.restrain_y,
            order_key: (0_u8, node.id.clone(), 0_usize),
        })
        .collect::<Vec<_>>();
    let mut node_ids: BTreeSet<String> =
        ordered_nodes.iter().map(|node| node.id.clone()).collect();

    for member in &ordered_members {
        let start = &ordered_nodes[*original_node_index
            .get(&member.start_node_id)
            .expect("validated start node")];
        let end = &ordered_nodes[*original_node_index
            .get(&member.end_node_id)
            .expect("validated end node")];

        for point_index in 1..elements_per_member {
            let id = interior_node_id(&member.id, point_index);
            if !node_ids.insert(id.clone()) {
                return Err(format!(
                    "generated mesh-node ID collides with an input or generated ID: {id}"
                ));
            }
            let fraction = point_index as f64 / elements_per_member as f64;
            mesh_nodes.push(MeshNode {
                id,
                x_m: start.x_m + fraction * (end.x_m - start.x_m),
                y_m: start.y_m + fraction * (end.y_m - start.y_m),
                restrain_x: false,
                restrain_y: false,
                order_key: (1_u8, member.id.clone(), point_index),
            });
        }
    }
    mesh_nodes.sort_by(|left, right| left.order_key.cmp(&right.order_key));

    let node_index: BTreeMap<String, usize> = mesh_nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), index))
        .collect();
    if node_index.len() != mesh_nodes.len() {
        return Err("mesh-node IDs are not unique after deterministic ordering".to_owned());
    }

    let translation_dofs = 2 * mesh_nodes.len();
    let rotations_per_member = elements_per_member + 1;
    let rotation_dofs = ordered_members.len() * rotations_per_member;
    let total_dofs = translation_dofs + rotation_dofs;
    if total_dofs > MAX_DENSE_BEAM_DOFS {
        return Err(format!(
            "refined dense mesh has {total_dofs} DOFs; current dense reference limit is {MAX_DENSE_BEAM_DOFS}; use a sparse modal solver for larger meshes"
        ));
    }

    let mut descriptors = Vec::with_capacity(total_dofs);
    let mut fixed = Vec::new();
    for (index, node) in mesh_nodes.iter().enumerate() {
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

    let mut stiffness = vec![vec![0.0; total_dofs]; total_dofs];
    let mut mass = vec![vec![0.0; total_dofs]; total_dofs];
    let mut translational_mass = vec![vec![0.0; total_dofs]; total_dofs];
    let mut rotary_mass = vec![vec![0.0; total_dofs]; total_dofs];

    for (member_index, member) in ordered_members.iter().enumerate() {
        let rotation_base = translation_dofs + member_index * rotations_per_member;

        // End rotations are private to each physical member at the joint.
        // Interior rotations belong to one member and are shared by its two
        // adjoining elements, preserving beam continuity rather than a hinge.
        for point_index in 0..=elements_per_member {
            let rotation_index = rotation_base + point_index;
            let kind = if point_index == 0 {
                BeamPinDofKind::MemberEndRotation {
                    member_id: member.id.clone(),
                    at_start: true,
                }
            } else if point_index == elements_per_member {
                BeamPinDofKind::MemberEndRotation {
                    member_id: member.id.clone(),
                    at_start: false,
                }
            } else {
                BeamPinDofKind::MemberInteriorRotation {
                    member_id: member.id.clone(),
                    interior_node_index: point_index,
                }
            };
            descriptors.push(BeamPinDofDescriptor {
                index: rotation_index,
                kind,
            });
        }

        for segment_index in 0..elements_per_member {
            let start_id = if segment_index == 0 {
                member.start_node_id.clone()
            } else {
                interior_node_id(&member.id, segment_index)
            };
            let end_id = if segment_index + 1 == elements_per_member {
                member.end_node_id.clone()
            } else {
                interior_node_id(&member.id, segment_index + 1)
            };
            let start_index = *node_index
                .get(&start_id)
                .expect("generated element start node exists");
            let end_index = *node_index
                .get(&end_id)
                .expect("generated element end node exists");
            let start_mesh_node = &mesh_nodes[start_index];
            let end_mesh_node = &mesh_nodes[end_index];
            let delta_x = end_mesh_node.x_m - start_mesh_node.x_m;
            let delta_y = end_mesh_node.y_m - start_mesh_node.y_m;
            let segment_length = delta_x.hypot(delta_y);

            let local = euler_bernoulli_frame_element(
                segment_length,
                member.area_m2,
                member.youngs_modulus_pa,
                member.density_kg_m3,
                member.second_moment_m4,
            )?;
            let global = transform_element_to_global(&local, delta_x, delta_y)?;
            let dof_map = [
                2 * start_index,
                2 * start_index + 1,
                rotation_base + segment_index,
                2 * end_index,
                2 * end_index + 1,
                rotation_base + segment_index + 1,
            ];
            assemble_element_matrix(&mut stiffness, &global.stiffness_global, &dof_map);
            assemble_element_matrix(&mut mass, &global.mass_global, &dof_map);
            assemble_element_matrix(
                &mut translational_mass,
                &global.translational_mass_global,
                &dof_map,
            );
            assemble_element_matrix(
                &mut rotary_mass,
                &global.rotary_inertia_mass_global,
                &dof_map,
            );
        }
    }

    descriptors.sort_by_key(|descriptor| descriptor.index);
    let fixed_set: BTreeSet<usize> = fixed.iter().copied().collect();
    let free = (0..total_dofs)
        .filter(|index| !fixed_set.contains(index))
        .collect::<Vec<_>>();
    for matrix in [&stiffness, &mass, &translational_mass, &rotary_mass] {
        ensure_finite_global_matrix(matrix)?;
        ensure_symmetric(matrix, 1.0e-10)?;
    }

    Ok(BeamPinAssembly {
        ordered_node_ids: mesh_nodes.into_iter().map(|node| node.id).collect(),
        ordered_member_ids: ordered_members.into_iter().map(|member| member.id).collect(),
        dof_descriptors: descriptors,
        stiffness_global: stiffness,
        mass_global: mass,
        translational_mass_global: translational_mass,
        rotary_inertia_mass_global: rotary_mass,
        fixed_dof_indices: fixed,
        free_dof_indices: free,
    })
}

#[derive(Clone, Debug)]
struct MeshNode {
    id: String,
    x_m: f64,
    y_m: f64,
    restrain_x: bool,
    restrain_y: bool,
    order_key: (u8, String, usize),
}

fn interior_node_id(member_id: &str, point_index: usize) -> String {
    format!("__mesh__{member_id}__{point_index:04}")
}

fn validate_refinement_budget(
    node_count: usize,
    member_count: usize,
    elements_per_member: usize,
) -> Check<()> {
    if node_count == 0 || node_count > MAX_SOURCE_NODES {
        return Err(format!("source node count must be between 1 and {MAX_SOURCE_NODES}"));
    }
    if member_count == 0 || member_count > MAX_SOURCE_MEMBERS {
        return Err(format!(
            "source member count must be between 1 and {MAX_SOURCE_MEMBERS}"
        ));
    }
    if elements_per_member == 0 || elements_per_member > MAX_DENSE_BEAM_DOFS {
        return Err(format!(
            "elements_per_member must be between 1 and {MAX_DENSE_BEAM_DOFS}"
        ));
    }

    let interior_count = member_count
        .checked_mul(elements_per_member - 1)
        .ok_or_else(|| "interior node count overflow".to_owned())?;
    let total_nodes = node_count
        .checked_add(interior_count)
        .ok_or_else(|| "total mesh-node count overflow".to_owned())?;
    let translation_dofs = total_nodes
        .checked_mul(2)
        .ok_or_else(|| "translation DOF count overflow".to_owned())?;
    let rotations_per_member = elements_per_member
        .checked_add(1)
        .ok_or_else(|| "rotations-per-member count overflow".to_owned())?;
    let rotation_dofs = member_count
        .checked_mul(rotations_per_member)
        .ok_or_else(|| "rotation DOF count overflow".to_owned())?;
    let total_dofs = translation_dofs
        .checked_add(rotation_dofs)
        .ok_or_else(|| "total DOF count overflow".to_owned())?;
    if total_dofs > MAX_DENSE_BEAM_DOFS {
        return Err(format!(
            "refined mesh requires {total_dofs} DOFs; dense reference limit is {MAX_DENSE_BEAM_DOFS}"
        ));
    }
    Ok(())
}

fn assemble_element_matrix(
    global: &mut [Vec<f64>],
    element: &Matrix6,
    dof_map: &[usize; 6],
) {
    for row in 0..6 {
        for col in 0..6 {
            global[dof_map[row]][dof_map[col]] += element[row][col];
        }
    }
}

fn ensure_finite_global_matrix(matrix: &[Vec<f64>]) -> Check<()> {
    if matrix.iter().flatten().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err("mesh-refined global matrix contains a non-finite entry".to_owned())
    }
}

fn ensure_symmetric(matrix: &[Vec<f64>], tolerance: f64) -> Check<()> {
    for row in 0..matrix.len() {
        for col in 0..matrix.len() {
            let left = matrix[row][col];
            let right = matrix[col][row];
            let scale = left.abs().max(right.abs()).max(1.0);
            if (left - right).abs() > tolerance * scale {
                return Err(format!(
                    "mesh-refined global matrix is not symmetric at ({row}, {col})"
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{assemble_refined_released_joint_pin_truss, canonical_refinement_request};
    use crate::beam_pin_assembly::{
        assemble_released_joint_pin_truss, BeamPinDofKind, BeamPinMember, BeamPinNode,
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
    fn one_element_mesh_is_exactly_the_existing_assembly() {
        let baseline = assemble_released_joint_pin_truss(&nodes(), &members())
            .expect("single-element assembly");
        let refined =
            assemble_refined_released_joint_pin_truss(&nodes(), &members(), 1)
                .expect("one-element mesh");
        assert_eq!(refined, baseline);
    }

    #[test]
    fn internal_rotations_are_shared_per_member_but_private_at_joints() {
        let result = assemble_refined_released_joint_pin_truss(&nodes(), &members(), 2)
            .expect("two elements per member");
        assert_eq!(result.ordered_node_ids.len(), 5);
        assert_eq!(result.stiffness_global.len(), 16);
        let end_rotations = result
            .dof_descriptors
            .iter()
            .filter(|descriptor| matches!(&descriptor.kind, BeamPinDofKind::MemberEndRotation { .. }))
            .count();
        let internal_rotations = result
            .dof_descriptors
            .iter()
            .filter(|descriptor| matches!(&descriptor.kind, BeamPinDofKind::MemberInteriorRotation { .. }))
            .count();
        assert_eq!(end_rotations, 4);
        assert_eq!(internal_rotations, 2);

        for descriptor in &result.dof_descriptors {
            if matches!(
                &descriptor.kind,
                BeamPinDofKind::MemberEndRotation { .. } | BeamPinDofKind::MemberInteriorRotation { .. }
            ) {
                assert!(result.stiffness_global[descriptor.index][descriptor.index] > 0.0);
                assert!(result.mass_global[descriptor.index][descriptor.index] > 0.0);
            }
        }
    }

    #[test]
    fn refined_assembly_is_invariant_to_input_storage_order() {
        let first =
            assemble_refined_released_joint_pin_truss(&nodes(), &members(), 4)
                .expect("ordered inputs");
        let mut reversed_nodes = nodes();
        reversed_nodes.reverse();
        let mut reversed_members = members();
        reversed_members.reverse();
        let second =
            assemble_refined_released_joint_pin_truss(&reversed_nodes, &reversed_members, 4)
                .expect("reversed inputs");
        assert_eq!(first, second);
    }

    #[test]
    fn mesh_request_is_frozen_and_includes_element_count() {
        let request =
            canonical_refinement_request(&nodes(), &members(), 2).expect("canonical mesh request");
        let frozen = include_str!("../fixtures/beam_mesh_request_v0_1.txt")
            .trim_end_matches('\n');
        assert_eq!(request, frozen);
        assert_ne!(
            canonical_refinement_request(&nodes(), &members(), 2).unwrap(),
            canonical_refinement_request(&nodes(), &members(), 4).unwrap()
        );
    }

    #[test]
    fn over_budget_mesh_fails_closed_before_dense_allocation() {
        assert!(
            assemble_refined_released_joint_pin_truss(&nodes(), &members(), 21).is_err()
        );
        assert!(canonical_refinement_request(&nodes(), &members(), 21).is_err());
    }
}
