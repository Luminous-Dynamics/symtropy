// SPDX-License-Identifier: AGPL-3.0-or-later
//! Uniform subdivision of physical members for released-joint beam modal
//! studies. Rotations are private at the original truss joints, but shared
//! between adjacent finite elements inside each continuous member. Interior
//! translations are member-private, so the two structural members meet only at
//! their original shared joints.

use crate::beam_element::{euler_bernoulli_frame_element, transform_element_to_global};
use crate::beam_pin_assembly::{
    BeamPinAssembly, BeamPinDofDescriptor, BeamPinDofKind, BeamPinMember, BeamPinNode,
    assemble_released_joint_pin_truss, canonical_assembly_request,
};
use crate::Check;
use std::collections::{BTreeMap, BTreeSet};

const MAX_ELEMENTS_PER_MEMBER: usize = 32;
const MAX_TOTAL_ELEMENTS: usize = 64;

/// Dense reference mesh result. The current cap is intended for small-mesh
/// verification/convergence studies; this is not the sparse 200-elements-per-
/// member production path described in the NAFEMS study.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamMemberMeshAssembly {
    pub assembly: BeamPinAssembly,
    pub elements_per_member: usize,
    pub total_element_count: usize,
    pub interior_node_count: usize,
    pub maximum_element_length_m: f64,
}

/// Canonical mesh request. Element count is part of the model request, not a
/// post-processing preference. The underlying geometry/section/support data
/// remains the same frozen beam-pin request.
pub fn canonical_beam_mesh_request(
    nodes: &[BeamPinNode],
    members: &[BeamPinMember],
    elements_per_member: usize,
) -> Check<String> {
    validate_mesh_resolution(members.len(), elements_per_member)?;
    let base = canonical_assembly_request(nodes, members)?;
    Ok(format!(
        "schema=beam-member-mesh-request-v0.1|elements_per_member={elements_per_member}|interior_rotations=shared_within_member|truss_joint_rotations=private_per_member_end|base_request={base}"
    ))
}

/// Assemble a uniformly subdivided, released-joint Euler-Bernoulli beam model.
///
/// - Translational DOFs at original truss joints are shared among attached
///   members.
/// - Internal translations and rotations belong to one original member.
/// - Adjacent elements of the same member share the interior rotation.
/// - Rotation at each original member endpoint remains private, so the three
///   original truss joints transmit no bending moment between members.
/// - Restraints apply only to original-node translations.
///
/// The dense implementation is deliberately capped at 64 total elements;
/// moving the benchmark towards the source's 200 elements/member requires a
/// sparse generalized eigensolver rather than simply raising this cap.
pub fn assemble_released_joint_beam_mesh(
    nodes: &[BeamPinNode],
    members: &[BeamPinMember],
    elements_per_member: usize,
) -> Check<BeamMemberMeshAssembly> {
    validate_mesh_resolution(members.len(), elements_per_member)?;
    // Reuse the established topology/ID/material validation for the original
    // graph before assigning any mesh-specific DOFs.
    let _base_control = assemble_released_joint_pin_truss(nodes, members)?;

    let mut ordered_nodes = nodes.to_vec();
    ordered_nodes.sort_by(|left, right| left.id.cmp(&right.id));
    let mut node_index = BTreeMap::new();
    for (index, node) in ordered_nodes.iter().enumerate() {
        node_index.insert(node.id.clone(), index);
    }

    let mut ordered_members = members.to_vec();
    ordered_members.sort_by(|left, right| left.id.cmp(&right.id));

    let interior_node_count = ordered_members.len() * (elements_per_member - 1);
    let translation_dof_count = 2 * (ordered_nodes.len() + interior_node_count);
    let rotation_dof_count = ordered_members.len() * (elements_per_member + 1);
    let total_dof_count = translation_dof_count + rotation_dof_count;

    let mut descriptors = Vec::with_capacity(total_dof_count);
    let mut fixed_dof_indices = Vec::new();

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
            fixed_dof_indices.push(x_index);
        }
        if node.restrain_y {
            fixed_dof_indices.push(y_index);
        }
    }

    // Reserve deterministic, unique internal translation pairs after original
    // node translations. Each pair is keyed by original member and station.
    let mut internal_translation = BTreeMap::new();
    let mut translation_cursor = 2 * ordered_nodes.len();
    for member in &ordered_members {
        for station in 1..elements_per_member {
            let x_index = translation_cursor;
            let y_index = x_index + 1;
            internal_translation.insert((member.id.clone(), station), [x_index, y_index]);
            descriptors.push(BeamPinDofDescriptor {
                index: x_index,
                kind: BeamPinDofKind::MemberInteriorTranslationX {
                    member_id: member.id.clone(),
                    station,
                },
            });
            descriptors.push(BeamPinDofDescriptor {
                index: y_index,
                kind: BeamPinDofKind::MemberInteriorTranslationY {
                    member_id: member.id.clone(),
                    station,
                },
            });
            translation_cursor += 2;
        }
    }
    if translation_cursor != translation_dof_count {
        return Err("internal translation DOF allocation is inconsistent".to_owned());
    }

    let mut stiffness = vec![vec![0.0; total_dof_count]; total_dof_count];
    let mut mass = vec![vec![0.0; total_dof_count]; total_dof_count];
    let mut translational_mass = vec![vec![0.0; total_dof_count]; total_dof_count];
    let mut rotary_mass = vec![vec![0.0; total_dof_count]; total_dof_count];

    let mut rotation_base = translation_dof_count;
    let mut maximum_element_length_m = 0.0_f64;
    for member in &ordered_members {
        let start_index = *node_index
            .get(&member.start_node_id)
            .expect("base validator checks start node");
        let end_index = *node_index
            .get(&member.end_node_id)
            .expect("base validator checks end node");
        let start = &ordered_nodes[start_index];
        let end = &ordered_nodes[end_index];
        let dx = end.x_m - start.x_m;
        let dy = end.y_m - start.y_m;
        let member_length = dx.hypot(dy);
        let element_length = member_length / elements_per_member as f64;
        if !element_length.is_finite() || element_length <= 0.0 {
            return Err(format!("member {} has invalid mesh segment length", member.id));
        }
        maximum_element_length_m = maximum_element_length_m.max(element_length);

        // Interior rotations are shared by the two adjacent elements of this
        // original member. Only stations 0 and n are private released rotations.
        for station in 0..=elements_per_member {
            let index = rotation_base + station;
            let kind = if station == 0 {
                BeamPinDofKind::MemberEndRotation {
                    member_id: member.id.clone(),
                    at_start: true,
                }
            } else if station == elements_per_member {
                BeamPinDofKind::MemberEndRotation {
                    member_id: member.id.clone(),
                    at_start: false,
                }
            } else {
                BeamPinDofKind::MemberInteriorRotation {
                    member_id: member.id.clone(),
                    station,
                }
            };
            descriptors.push(BeamPinDofDescriptor { index, kind });
        }

        let local = euler_bernoulli_frame_element(
            element_length,
            member.area_m2,
            member.youngs_modulus_pa,
            member.density_kg_m3,
            member.second_moment_m4,
        )?;
        let mut points = Vec::with_capacity(elements_per_member + 1);
        for station in 0..=elements_per_member {
            let fraction = station as f64 / elements_per_member as f64;
            let (x_dof, y_dof) = if station == 0 {
                (2 * start_index, 2 * start_index + 1)
            } else if station == elements_per_member {
                (2 * end_index, 2 * end_index + 1)
            } else {
                let pair = internal_translation
                    .get(&(member.id.clone(), station))
                    .expect("all interior translations were allocated");
                (pair[0], pair[1])
            };
            points.push(MeshPoint {
                x_m: start.x_m + fraction * dx,
                y_m: start.y_m + fraction * dy,
                x_dof,
                y_dof,
            });
        }

        for segment_index in 0..elements_per_member {
            let p0 = &points[segment_index];
            let p1 = &points[segment_index + 1];
            let dx_segment = p1.x_m - p0.x_m;
            let dy_segment = p1.y_m - p0.y_m;
            let global = transform_element_to_global(&local, dx_segment, dy_segment)?;
            let rotation_start = rotation_base + segment_index;
            let dof_map = [
                p0.x_dof,
                p0.y_dof,
                rotation_start,
                p1.x_dof,
                p1.y_dof,
                rotation_start + 1,
            ];
            assemble(&mut stiffness, &global.stiffness_global, &dof_map);
            assemble(&mut mass, &global.mass_global, &dof_map);
            assemble(
                &mut translational_mass,
                &global.translational_mass_global,
                &dof_map,
            );
            assemble(
                &mut rotary_mass,
                &global.rotary_inertia_mass_global,
                &dof_map,
            );
        }
        rotation_base += elements_per_member + 1;
    }

    if rotation_base != total_dof_count {
        return Err("member rotation DOF allocation is inconsistent".to_owned());
    }
    descriptors.sort_by_key(|descriptor| descriptor.index);
    let fixed_set: BTreeSet<usize> = fixed_dof_indices.iter().copied().collect();
    let free_dof_indices = (0..total_dof_count)
        .filter(|index| !fixed_set.contains(index))
        .collect::<Vec<_>>();

    for matrix in [&stiffness, &mass, &translational_mass, &rotary_mass] {
        ensure_finite(matrix)?;
        ensure_symmetric(matrix, 1.0e-10)?;
    }
    if descriptors.len() != total_dof_count
        || descriptors
            .iter()
            .enumerate()
            .any(|(expected, descriptor)| descriptor.index != expected)
    {
        return Err("global mesh DOF descriptor map is incomplete or noncanonical".to_owned());
    }

    Ok(BeamMemberMeshAssembly {
        assembly: BeamPinAssembly {
            ordered_node_ids: ordered_nodes.into_iter().map(|node| node.id).collect(),
            ordered_member_ids: ordered_members.into_iter().map(|member| member.id).collect(),
            dof_descriptors: descriptors,
            stiffness_global: stiffness,
            mass_global: mass,
            translational_mass_global: translational_mass,
            rotary_inertia_mass_global: rotary_mass,
            fixed_dof_indices,
            free_dof_indices,
        },
        elements_per_member,
        total_element_count: members.len() * elements_per_member,
        interior_node_count,
        maximum_element_length_m,
    })
}

fn validate_mesh_resolution(member_count: usize, elements_per_member: usize) -> Check<()> {
    if elements_per_member == 0 || elements_per_member > MAX_ELEMENTS_PER_MEMBER {
        return Err(format!(
            "elements_per_member must be between 1 and {MAX_ELEMENTS_PER_MEMBER}"
        ));
    }
    let total = member_count.saturating_mul(elements_per_member);
    if total == 0 || total > MAX_TOTAL_ELEMENTS {
        return Err(format!(
            "total mesh element count must be between 1 and {MAX_TOTAL_ELEMENTS}"
        ));
    }
    Ok(())
}

struct MeshPoint {
    x_m: f64,
    y_m: f64,
    x_dof: usize,
    y_dof: usize,
}

fn assemble(global: &mut [Vec<f64>], element: &[[f64; 6]; 6], map: &[usize; 6]) {
    for row in 0..6 {
        for col in 0..6 {
            global[map[row]][map[col]] += element[row][col];
        }
    }
}

fn ensure_finite(matrix: &[Vec<f64>]) -> Check<()> {
    if matrix.iter().flatten().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err("beam mesh contains non-finite global matrix entries".to_owned())
    }
}

fn ensure_symmetric(matrix: &[Vec<f64>], tolerance: f64) -> Check<()> {
    for row in 0..matrix.len() {
        for col in 0..matrix.len() {
            let left = matrix[row][col];
            let right = matrix[col][row];
            let scale = left.abs().max(right.abs()).max(1.0);
            if (left - right).abs() > tolerance * scale {
                return Err(format!("beam mesh matrix is not symmetric at ({row}, {col})"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{assemble_released_joint_beam_mesh, canonical_beam_mesh_request};
    use crate::beam_pin_assembly::{
        assemble_released_joint_pin_truss, nafems_nbc05_released_joint_beam_fixture,
        BeamPinDofKind,
    };

    #[test]
    fn one_element_per_member_matches_the_frozen_pin_assembly() {
        let (nodes, members) = nafems_nbc05_released_joint_beam_fixture();
        let baseline = assemble_released_joint_pin_truss(&nodes, &members)
            .expect("single-element pin assembly");
        let mesh = assemble_released_joint_beam_mesh(&nodes, &members, 1)
            .expect("one-element mesh assembly");
        assert_eq!(mesh.assembly, baseline);
        assert_eq!(mesh.total_element_count, 2);
        assert_eq!(mesh.interior_node_count, 0);
    }

    #[test]
    fn internal_rotations_are_shared_within_members_but_not_at_truss_joints() {
        let (nodes, members) = nafems_nbc05_released_joint_beam_fixture();
        let mesh = assemble_released_joint_beam_mesh(&nodes, &members, 2)
            .expect("two elements per member");
        assert_eq!(mesh.total_element_count, 4);
        assert_eq!(mesh.interior_node_count, 2);
        assert_eq!(mesh.assembly.stiffness_global.len(), 16);
        assert_eq!(mesh.assembly.free_dof_indices.len(), 12);

        let member_end_rotation_count = mesh.assembly.dof_descriptors.iter()
            .filter(|descriptor| matches!(&descriptor.kind, BeamPinDofKind::MemberEndRotation { .. }))
            .count();
        let interior_rotation_count = mesh.assembly.dof_descriptors.iter()
            .filter(|descriptor| matches!(&descriptor.kind, BeamPinDofKind::MemberInteriorRotation { .. }))
            .count();
        let interior_translation_count = mesh.assembly.dof_descriptors.iter()
            .filter(|descriptor| matches!(&descriptor.kind,
                BeamPinDofKind::MemberInteriorTranslationX { .. } |
                BeamPinDofKind::MemberInteriorTranslationY { .. }))
            .count();
        assert_eq!(member_end_rotation_count, 4);
        assert_eq!(interior_rotation_count, 2);
        assert_eq!(interior_translation_count, 4);
        for descriptor in &mesh.assembly.dof_descriptors {
            if matches!(
                &descriptor.kind,
                BeamPinDofKind::MemberInteriorRotation { .. }
                    | BeamPinDofKind::MemberInteriorTranslationX { .. }
                    | BeamPinDofKind::MemberInteriorTranslationY { .. }
            ) {
                assert!(
                    mesh.assembly.stiffness_global[descriptor.index][descriptor.index] > 0.0,
                    "internal mesh DOF {} must have bending/axial stiffness",
                    descriptor.index
                );
                assert!(
                    mesh.assembly.mass_global[descriptor.index][descriptor.index] > 0.0,
                    "internal mesh DOF {} must have inertia",
                    descriptor.index
                );
            }
        }

        let ac_end = mesh.assembly.dof_descriptors.iter().find_map(|descriptor| {
            match &descriptor.kind {
                BeamPinDofKind::MemberEndRotation { member_id, at_start: false } if member_id == "AC" => Some(descriptor.index),
                _ => None,
            }
        }).expect("AC end rotation");
        let bc_end = mesh.assembly.dof_descriptors.iter().find_map(|descriptor| {
            match &descriptor.kind {
                BeamPinDofKind::MemberEndRotation { member_id, at_start: false } if member_id == "BC" => Some(descriptor.index),
                _ => None,
            }
        }).expect("BC end rotation");
        assert_ne!(ac_end, bc_end);
        assert_eq!(mesh.assembly.stiffness_global[ac_end][bc_end], 0.0);
    }

    #[test]
    fn assembly_is_deterministic_under_input_permutation_and_request_tracks_mesh_density() {
        let (nodes, members) = nafems_nbc05_released_joint_beam_fixture();
        let baseline = assemble_released_joint_beam_mesh(&nodes, &members, 4)
            .expect("baseline mesh");
        let mut reversed_nodes = nodes.clone();
        reversed_nodes.reverse();
        let mut reversed_members = members.clone();
        reversed_members.reverse();
        let permuted = assemble_released_joint_beam_mesh(&reversed_nodes, &reversed_members, 4)
            .expect("permuted mesh");
        assert_eq!(baseline, permuted);

        let request2 = canonical_beam_mesh_request(&nodes, &members, 2).expect("request two");
        let request4 = canonical_beam_mesh_request(&nodes, &members, 4).expect("request four");
        assert_ne!(request2, request4);
    }

    #[test]
    fn invalid_mesh_resolutions_fail_closed() {
        let (nodes, members) = nafems_nbc05_released_joint_beam_fixture();
        assert!(assemble_released_joint_beam_mesh(&nodes, &members, 0).is_err());
        assert!(assemble_released_joint_beam_mesh(&nodes, &members, 33).is_err());
        assert!(canonical_beam_mesh_request(&nodes, &members, 0).is_err());
    }
}
