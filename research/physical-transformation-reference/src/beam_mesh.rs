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

/// Canonical request for a fixed, ordered mesh-refinement ladder.
pub fn canonical_refinement_sweep_request(
    nodes: &[BeamPinNode],
    members: &[BeamPinMember],
    element_counts: &[usize],
) -> Check<String> {
    if element_counts.is_empty() {
        return Err("mesh refinement ladder must contain at least one element count".to_owned());
    }
    let mut previous = 0_usize;
    for count in element_counts {
        validate_refinement_budget(nodes.len(), members.len(), *count)?;
        if *count <= previous {
            return Err("mesh refinement element counts must be strictly increasing".to_owned());
        }
        previous = *count;
    }
    let base_request = canonical_assembly_request(nodes, members)?;
    let counts = element_counts
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!(
        "schema=beam-mesh-sweep-request-v0.1\nelements_per_member={counts}\nbase_request={base_request}"
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

/// Project a global modal shape onto fixed physical-space translational
/// samples on each source member. Five fractions (0, 1/4, 1/2, 3/4, 1) are
/// evaluated from linear axial and cubic-Hermite transverse interpolation.
/// This produces a stable 10-values-per-member vector across mesh densities,
/// suitable for sign-invariant MAC comparisons even when the DOF counts differ.
pub fn sample_member_displacement_field(
    nodes: &[BeamPinNode],
    members: &[BeamPinMember],
    elements_per_member: usize,
    assembly: &BeamPinAssembly,
    shape_global_dofs: &[f64],
) -> Check<Vec<f64>> {
    validate_refinement_budget(nodes.len(), members.len(), elements_per_member)?;
    let total_dofs = assembly.stiffness_global.len();
    if shape_global_dofs.len() != total_dofs
        || shape_global_dofs.iter().any(|value| !value.is_finite())
    {
        return Err("modal shape must be finite and match the assembly DOF count".to_owned());
    }
    if assembly.ordered_node_ids.len() * 2 > total_dofs {
        return Err("assembly has fewer DOFs than its ordered node translations".to_owned());
    }

    let mut ordered_nodes = nodes.to_vec();
    ordered_nodes.sort_by(|left, right| left.id.cmp(&right.id));
    let mut ordered_members = members.to_vec();
    ordered_members.sort_by(|left, right| left.id.cmp(&right.id));
    let mut coordinates: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    for node in &ordered_nodes {
        coordinates.insert(node.id.clone(), (node.x_m, node.y_m));
    }
    for member in &ordered_members {
        let start = ordered_nodes
            .iter()
            .find(|node| node.id == member.start_node_id)
            .ok_or_else(|| format!("missing sample start node {}", member.start_node_id))?;
        let end = ordered_nodes
            .iter()
            .find(|node| node.id == member.end_node_id)
            .ok_or_else(|| format!("missing sample end node {}", member.end_node_id))?;
        for point in 1..elements_per_member {
            let fraction = point as f64 / elements_per_member as f64;
            coordinates.insert(
                interior_node_id(&member.id, point),
                (
                    start.x_m + fraction * (end.x_m - start.x_m),
                    start.y_m + fraction * (end.y_m - start.y_m),
                ),
            );
        }
    }

    let node_index: BTreeMap<&str, usize> = assembly
        .ordered_node_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect();
    let mut rotation_index: BTreeMap<(String, usize), usize> = BTreeMap::new();
    for descriptor in &assembly.dof_descriptors {
        let key = match &descriptor.kind {
            BeamPinDofKind::MemberEndRotation { member_id, at_start } => (
                member_id.clone(),
                if *at_start { 0 } else { elements_per_member },
            ),
            BeamPinDofKind::MemberInteriorRotation {
                member_id,
                interior_node_index,
            } => (member_id.clone(), *interior_node_index),
            BeamPinDofKind::NodeTranslationX { .. }
            | BeamPinDofKind::NodeTranslationY { .. } => continue,
        };
        if key.1 > elements_per_member {
            return Err(format!(
                "rotation descriptor for member {} is outside the requested mesh",
                key.0
            ));
        }
        if rotation_index.insert(key.clone(), descriptor.index).is_some() {
            return Err(format!(
                "duplicate member rotation descriptor for {} at point {}",
                key.0, key.1
            ));
        }
    }

    let sample_fractions = [0.0_f64, 0.25, 0.5, 0.75, 1.0];
    let mut samples = Vec::with_capacity(ordered_members.len() * sample_fractions.len() * 2);
    for member in &ordered_members {
        for fraction_along_member in sample_fractions {
            let scaled_position = fraction_along_member * elements_per_member as f64;
            let segment_index = if fraction_along_member >= 1.0 {
                elements_per_member - 1
            } else {
                (scaled_position.floor() as usize).min(elements_per_member - 1)
            };
            let local_fraction = (scaled_position - segment_index as f64).clamp(0.0, 1.0);
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
            let (start_x, start_y) = coordinates
                .get(&start_id)
                .copied()
                .ok_or_else(|| format!("missing mesh coordinate for {start_id}"))?;
            let (end_x, end_y) = coordinates
                .get(&end_id)
                .copied()
                .ok_or_else(|| format!("missing mesh coordinate for {end_id}"))?;
            let start_node_index = *node_index
                .get(start_id.as_str())
                .ok_or_else(|| format!("assembly is missing node {start_id}"))?;
            let end_node_index = *node_index
                .get(end_id.as_str())
                .ok_or_else(|| format!("assembly is missing node {end_id}"))?;
            let dx = end_x - start_x;
            let dy = end_y - start_y;
            let length = dx.hypot(dy);
            if !length.is_finite() || length <= 0.0 {
                return Err(format!("sample element {start_id}->{end_id} has invalid length"));
            }
            let cosine = dx / length;
            let sine = dy / length;
            let ux_start = shape_global_dofs[2 * start_node_index];
            let uy_start = shape_global_dofs[2 * start_node_index + 1];
            let ux_end = shape_global_dofs[2 * end_node_index];
            let uy_end = shape_global_dofs[2 * end_node_index + 1];
            let u_start = cosine * ux_start + sine * uy_start;
            let v_start = -sine * ux_start + cosine * uy_start;
            let u_end = cosine * ux_end + sine * uy_end;
            let v_end = -sine * ux_end + cosine * uy_end;
            let theta_start_index = *rotation_index
                .get(&(member.id.clone(), segment_index))
                .ok_or_else(|| {
                    format!(
                        "missing member rotation {} at point {}",
                        member.id, segment_index
                    )
                })?;
            let theta_end_index = *rotation_index
                .get(&(member.id.clone(), segment_index + 1))
                .ok_or_else(|| {
                    format!(
                        "missing member rotation {} at point {}",
                        member.id,
                        segment_index + 1
                    )
                })?;
            let theta_start = shape_global_dofs[theta_start_index];
            let theta_end = shape_global_dofs[theta_end_index];

            let xi = local_fraction;
            let xi2 = xi * xi;
            let xi3 = xi2 * xi;
            let axial = (1.0 - xi) * u_start + xi * u_end;
            let transverse = (1.0 - 3.0 * xi2 + 2.0 * xi3) * v_start
                + length * (xi - 2.0 * xi2 + xi3) * theta_start
                + (3.0 * xi2 - 2.0 * xi3) * v_end
                + length * (-xi2 + xi3) * theta_end;
            samples.push(cosine * axial - sine * transverse);
            samples.push(sine * axial + cosine * transverse);
        }
    }

    if samples.iter().any(|value| !value.is_finite()) {
        return Err("sampled modal displacement field contains a non-finite value".to_owned());
    }
    Ok(samples)
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
    use std::collections::BTreeSet;

    use super::{
        assemble_refined_released_joint_pin_truss, canonical_refinement_request,
        canonical_refinement_sweep_request, sample_member_displacement_field,
    };
    use crate::beam_modal::{modal_assurance_criterion, solve_beam_pin_assembly_modes};
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
        let unique_dof_indices = result
            .dof_descriptors
            .iter()
            .map(|descriptor| descriptor.index)
            .collect::<BTreeSet<_>>();
        assert_eq!(unique_dof_indices.len(), result.dof_descriptors.len());
        assert_eq!(result.dof_descriptors.len(), result.stiffness_global.len());

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

    fn solve_small_dense(mut matrix: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Vec<f64> {
        let size = rhs.len();
        assert_eq!(matrix.len(), size);
        let scale = matrix
            .iter()
            .flatten()
            .map(|value| value.abs())
            .fold(0.0_f64, f64::max);
        assert!(scale > 0.0);

        for pivot_col in 0..size {
            let pivot_row = (pivot_col..size)
                .max_by(|left, right| {
                    matrix[*left][pivot_col]
                        .abs()
                        .total_cmp(&matrix[*right][pivot_col].abs())
                })
                .expect("pivot row");
            assert!(
                matrix[pivot_row][pivot_col].abs() > 1.0e-13 * scale,
                "rotation stiffness block is singular"
            );
            matrix.swap(pivot_col, pivot_row);
            rhs.swap(pivot_col, pivot_row);
            let pivot = matrix[pivot_col][pivot_col];

            for row in pivot_col + 1..size {
                let factor = matrix[row][pivot_col] / pivot;
                matrix[row][pivot_col] = 0.0;
                for col in pivot_col + 1..size {
                    matrix[row][col] -= factor * matrix[pivot_col][col];
                }
                rhs[row] -= factor * rhs[pivot_col];
            }
        }

        let mut solution = vec![0.0; size];
        for row in (0..size).rev() {
            let known = (row + 1..size)
                .map(|col| matrix[row][col] * solution[col])
                .sum::<f64>();
            solution[row] = (rhs[row] - known) / matrix[row][row];
        }
        solution
    }

    #[test]
    fn modal_shape_projection_has_mesh_invariant_dimension_and_mac() {
        let mut sampled_shapes = Vec::new();
        for elements_per_member in [1_usize, 2, 4, 8] {
            let assembly = assemble_refined_released_joint_pin_truss(
                &nodes(),
                &members(),
                elements_per_member,
            )
            .expect("refined beam assembly");
            let solution =
                solve_beam_pin_assembly_modes(&assembly).expect("refined beam modes");
            let shape = &solution.modes[0].shape_global_dofs;
            let sampled = sample_member_displacement_field(
                &nodes(),
                &members(),
                elements_per_member,
                &assembly,
                shape,
            )
            .expect("shape projection");
            assert_eq!(sampled.len(), members().len() * 5 * 2);
            assert!(sampled.iter().all(|value| value.is_finite()));
            sampled_shapes.push(sampled);
        }

        for pair in sampled_shapes.windows(2) {
            let mac = modal_assurance_criterion(&pair[0], &pair[1])
                .expect("equal-dimensional shape samples");
            assert!((0.0..=1.0).contains(&mac));
        }

        let first = &sampled_shapes[0];
        let sign_flipped = first.iter().map(|value| -*value).collect::<Vec<_>>();
        assert!(
            (modal_assurance_criterion(first, &sign_flipped).unwrap() - 1.0).abs()
                < 1.0e-12
        );
    }

    #[test]
    fn condensing_refined_beam_rotations_recovers_independent_axial_mesh_stiffness() {
        let source_nodes = nodes();
        let source_members = members();
        let elements_per_member = 2_usize;
        let assembly = assemble_refined_released_joint_pin_truss(
            &source_nodes,
            &source_members,
            elements_per_member,
        )
        .expect("refined beam assembly");
        let translation_dofs = 2 * assembly.ordered_node_ids.len();
        let rotation_dofs = assembly.stiffness_global.len() - translation_dofs;

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
                        assembly.stiffness_global[translation_dofs + row]
                            [translation_dofs + col]
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mut condensed = k_tt;

        for column in 0..translation_dofs {
            let rhs = (0..rotation_dofs)
                .map(|row| {
                    assembly.stiffness_global[translation_dofs + row][column]
                })
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

        // Independent axial mesh assembly from segment geometry and EA/L;
        // it does not reuse any beam element or assembled beam K block.
        let mut coordinates = BTreeMap::new();
        for node in &source_nodes {
            coordinates.insert(node.id.clone(), (node.x_m, node.y_m));
        }
        for member in &source_members {
            let start = source_nodes
                .iter()
                .find(|node| node.id == member.start_node_id)
                .expect("axial oracle start node");
            let end = source_nodes
                .iter()
                .find(|node| node.id == member.end_node_id)
                .expect("axial oracle end node");
            for point in 1..elements_per_member {
                let fraction = point as f64 / elements_per_member as f64;
                coordinates.insert(
                    super::interior_node_id(&member.id, point),
                    (
                        start.x_m + fraction * (end.x_m - start.x_m),
                        start.y_m + fraction * (end.y_m - start.y_m),
                    ),
                );
            }
        }

        let mut axial_oracle = vec![vec![0.0; translation_dofs]; translation_dofs];
        for member in &source_members {
            for segment in 0..elements_per_member {
                let start_id = if segment == 0 {
                    member.start_node_id.clone()
                } else {
                    super::interior_node_id(&member.id, segment)
                };
                let end_id = if segment + 1 == elements_per_member {
                    member.end_node_id.clone()
                } else {
                    super::interior_node_id(&member.id, segment + 1)
                };
                let (start_x, start_y) = coordinates[&start_id];
                let (end_x, end_y) = coordinates[&end_id];
                let length = (end_x - start_x).hypot(end_y - start_y);
                let cosine = (end_x - start_x) / length;
                let sine = (end_y - start_y) / length;
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
                    .position(|id| id == &start_id)
                    .expect("ordered axial oracle start node");
                let end_index = assembly
                    .ordered_node_ids
                    .iter()
                    .position(|id| id == &end_id)
                    .expect("ordered axial oracle end node");
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
            "refined pin beam Schur complement differs from independent axial mesh: {maximum_scaled_error:.17e}"
        );
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
    fn mesh_sweep_request_freezes_all_refinement_levels() {
        let request = canonical_refinement_sweep_request(&nodes(), &members(), &[1, 2, 4, 8])
            .expect("canonical refinement sweep request");
        let frozen = include_str!("../fixtures/beam_mesh_sweep_request_v0_1.txt")
            .trim_end_matches('\\n');
        assert_eq!(request, frozen);
        assert!(canonical_refinement_sweep_request(&nodes(), &members(), &[1, 4, 2]).is_err());
        assert!(canonical_refinement_sweep_request(&nodes(), &members(), &[]).is_err());
    }

    #[test]
    fn over_budget_mesh_fails_closed_before_dense_allocation() {
        assert!(
            assemble_refined_released_joint_pin_truss(&nodes(), &members(), 21).is_err()
        );
        assert!(canonical_refinement_request(&nodes(), &members(), 21).is_err());
    }
}
