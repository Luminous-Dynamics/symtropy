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
const MAX_CHAIN_ELEMENTS_PER_MEMBER: usize = 32;
const MAX_CHAIN_NODES: usize = 512;
const MAX_CHAIN_DOFS: usize = 2048;

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
    NodeTranslationX {
        node_id: String,
    },
    NodeTranslationY {
        node_id: String,
    },
    MemberEndRotation {
        member_id: String,
        at_start: bool,
    },
    /// Rotation shared by adjacent elements inside one physical member chain.
    MemberInteriorRotation {
        member_id: String,
        station_index: usize,
    },
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

/// A physical member represented by a sequence of Euler-Bernoulli elements.
/// Member-end rotations are released at the physical joints; intermediate
/// station rotations are shared between adjacent elements so refinement does
/// not accidentally turn the member into many pin-ended beams.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamMemberChain {
    pub id: String,
    pub start_node_id: String,
    pub end_node_id: String,
    pub area_m2: f64,
    pub youngs_modulus_pa: f64,
    pub density_kg_m3: f64,
    pub second_moment_m4: f64,
    pub elements_per_member: usize,
}

/// Canonical request for a refined member-chain mesh. The exact segment count
/// and section/model inputs are part of the identity of the simulation.
pub fn canonical_beam_chain_request(
    joints: &[BeamPinNode],
    members: &[BeamMemberChain],
) -> Check<String> {
    assemble_released_joint_beam_chains(joints, members)?;

    let mut ordered_joints = joints.to_vec();
    ordered_joints.sort_by(|left, right| left.id.cmp(&right.id));
    let mut ordered_members = members.to_vec();
    ordered_members.sort_by(|left, right| left.id.cmp(&right.id));

    let mut fields = vec![
        "schema=beam-chain-request-v0.1".to_owned(),
        "fixture=PHYS-BEAM-CHAIN-001".to_owned(),
    ];
    for joint in ordered_joints {
        fields.push(format!(
            "joint|{}|{}|{}|{}|{}",
            joint.id, joint.x_m, joint.y_m, joint.restrain_x, joint.restrain_y
        ));
    }
    for member in ordered_members {
        fields.push(format!(
            "chain|{}|{}|{}|{}|{}|{}|{}|{}",
            member.id,
            member.start_node_id,
            member.end_node_id,
            member.area_m2,
            member.youngs_modulus_pa,
            member.density_kg_m3,
            member.second_moment_m4,
            member.elements_per_member
        ));
    }
    Ok(fields.join("|"))
}

/// Canonical request covering a deterministic list of mesh levels for a
/// fixed member-chain geometry/material set. The element-count envelope is
/// sorted and unique, and every level is validated by the assembly routine.
pub fn canonical_beam_chain_sweep_request(
    joints: &[BeamPinNode],
    members: &[BeamMemberChain],
    element_counts_per_member: &[usize],
) -> Check<String> {
    if element_counts_per_member.is_empty() {
        return Err("mesh sweep must include at least one element count".to_owned());
    }
    let mut levels = element_counts_per_member.to_vec();
    levels.sort_unstable();
    if levels.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("mesh sweep element counts must be unique".to_owned());
    }

    for level in &levels {
        let mut level_members = members.to_vec();
        for member in &mut level_members {
            member.elements_per_member = *level;
        }
        assemble_released_joint_beam_chains(joints, &level_members)?;
    }

    let mut ordered_joints = joints.to_vec();
    ordered_joints.sort_by(|left, right| left.id.cmp(&right.id));
    let mut ordered_members = members.to_vec();
    ordered_members.sort_by(|left, right| left.id.cmp(&right.id));

    let mut fields = vec![
        "schema=beam-chain-modal-sweep-request-v0.1".to_owned(),
        "fixture=PHYS-BEAM-CHAIN-MODAL-001".to_owned(),
        format!(
            "element_counts_per_member={}",
            levels
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ),
    ];
    for joint in ordered_joints {
        fields.push(format!(
            "joint|{}|{}|{}|{}|{}",
            joint.id, joint.x_m, joint.y_m, joint.restrain_x, joint.restrain_y
        ));
    }
    for member in ordered_members {
        fields.push(format!(
            "chain|{}|{}|{}|{}|{}|{}|{}",
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

/// Assemble a refined planar beam mesh while releasing rotation only at
/// physical member endpoints. Every chain interior station has one rotation
/// DOF shared by its two adjacent segments. Translational and rotary mass
/// contributions remain separately inspectable in the returned assembly.
///
/// The current dense reference implementation is deliberately capped at
/// 32 elements per physical member; a 200-elements-per-member NAFEMS study
/// requires a sparse/partial eigensolver and is not implied by this function.
pub fn assemble_released_joint_beam_chains(
    joints: &[BeamPinNode],
    members: &[BeamMemberChain],
) -> Check<BeamPinAssembly> {
    if joints.is_empty() || joints.len() > MAX_NODES {
        return Err(format!("joint count must be between 1 and {MAX_NODES}"));
    }
    if members.is_empty() || members.len() > MAX_MEMBERS {
        return Err(format!(
            "member-chain count must be between 1 and {MAX_MEMBERS}"
        ));
    }

    let mut ordered_joints = joints.to_vec();
    ordered_joints.sort_by(|left, right| left.id.cmp(&right.id));
    let mut joint_index = BTreeMap::new();
    for (index, joint) in ordered_joints.iter().enumerate() {
        validate_identifier("joint ID", &joint.id)?;
        if !joint.x_m.is_finite() || !joint.y_m.is_finite() {
            return Err(format!("joint {} has non-finite coordinates", joint.id));
        }
        if joint_index.insert(joint.id.clone(), index).is_some() {
            return Err(format!("duplicate joint ID: {}", joint.id));
        }
    }

    let mut ordered_members = members.to_vec();
    ordered_members.sort_by(|left, right| left.id.cmp(&right.id));
    let mut member_ids = BTreeSet::new();
    let mut used_joints = BTreeSet::new();
    for member in &ordered_members {
        validate_identifier("member-chain ID", &member.id)?;
        validate_identifier("member start joint ID", &member.start_node_id)?;
        validate_identifier("member end joint ID", &member.end_node_id)?;
        if !member_ids.insert(member.id.as_str()) {
            return Err(format!("duplicate member-chain ID: {}", member.id));
        }
        if member.start_node_id == member.end_node_id {
            return Err(format!(
                "member chain {} has identical endpoints",
                member.id
            ));
        }
        if !joint_index.contains_key(&member.start_node_id)
            || !joint_index.contains_key(&member.end_node_id)
        {
            return Err(format!(
                "member chain {} references an unknown joint",
                member.id
            ));
        }
        if !(1..=MAX_CHAIN_ELEMENTS_PER_MEMBER).contains(&member.elements_per_member) {
            return Err(format!(
                "member {} element count must be between 1 and {}",
                member.id, MAX_CHAIN_ELEMENTS_PER_MEMBER
            ));
        }
        used_joints.insert(member.start_node_id.clone());
        used_joints.insert(member.end_node_id.clone());
    }
    for joint in &ordered_joints {
        if !used_joints.contains(&joint.id) {
            return Err(format!(
                "isolated physical joint is not allowed: {}",
                joint.id
            ));
        }
    }

    let mut all_nodes = ordered_joints.clone();
    let mut station_ids = BTreeMap::<String, Vec<String>>::new();
    for member in &ordered_members {
        let start_index = *joint_index
            .get(&member.start_node_id)
            .expect("validated start joint");
        let end_index = *joint_index
            .get(&member.end_node_id)
            .expect("validated end joint");
        let start = &ordered_joints[start_index];
        let end = &ordered_joints[end_index];
        let element_count = member.elements_per_member;
        let mut ids = Vec::with_capacity(element_count + 1);
        ids.push(member.start_node_id.clone());

        for station in 1..element_count {
            let fraction = station as f64 / element_count as f64;
            let id = format!("{}::station::{station:04}", member.id);
            validate_identifier("generated interior station ID", &id)?;
            let station_node = BeamPinNode {
                id: id.clone(),
                x_m: start.x_m + (end.x_m - start.x_m) * fraction,
                y_m: start.y_m + (end.y_m - start.y_m) * fraction,
                restrain_x: false,
                restrain_y: false,
            };
            if !station_node.x_m.is_finite() || !station_node.y_m.is_finite() {
                return Err(format!(
                    "member {} generated a non-finite interior coordinate",
                    member.id
                ));
            }
            ids.push(id);
            all_nodes.push(station_node);
        }
        ids.push(member.end_node_id.clone());
        station_ids.insert(member.id.clone(), ids);
    }

    if all_nodes.len() > MAX_CHAIN_NODES {
        return Err(format!(
            "refined beam mesh node count exceeds {MAX_CHAIN_NODES}"
        ));
    }
    all_nodes.sort_by(|left, right| left.id.cmp(&right.id));
    for pair in all_nodes.windows(2) {
        if pair[0].id == pair[1].id {
            return Err(format!(
                "generated/physical station ID collision: {}",
                pair[0].id
            ));
        }
    }
    let mut all_node_index = BTreeMap::<String, usize>::new();
    for (index, node) in all_nodes.iter().enumerate() {
        all_node_index.insert(node.id.clone(), index);
    }

    let translation_dof_count = 2 * all_nodes.len();
    let rotation_dof_count: usize = ordered_members
        .iter()
        .map(|member| member.elements_per_member + 1)
        .sum();
    let dof_count = translation_dof_count + rotation_dof_count;
    if dof_count > MAX_CHAIN_DOFS {
        return Err(format!(
            "refined beam mesh DOF count exceeds {MAX_CHAIN_DOFS}"
        ));
    }

    let mut descriptors = Vec::with_capacity(dof_count);
    let mut fixed = Vec::new();
    for (index, node) in all_nodes.iter().enumerate() {
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

    let mut member_rotation_indices = BTreeMap::<String, Vec<usize>>::new();
    let mut next_rotation_index = translation_dof_count;
    for member in &ordered_members {
        let mut indices = Vec::with_capacity(member.elements_per_member + 1);
        for station in 0..=member.elements_per_member {
            let index = next_rotation_index;
            next_rotation_index += 1;
            let kind = if station == 0 {
                BeamPinDofKind::MemberEndRotation {
                    member_id: member.id.clone(),
                    at_start: true,
                }
            } else if station == member.elements_per_member {
                BeamPinDofKind::MemberEndRotation {
                    member_id: member.id.clone(),
                    at_start: false,
                }
            } else {
                BeamPinDofKind::MemberInteriorRotation {
                    member_id: member.id.clone(),
                    station_index: station,
                }
            };
            descriptors.push(BeamPinDofDescriptor { index, kind });
            indices.push(index);
        }
        member_rotation_indices.insert(member.id.clone(), indices);
    }
    debug_assert_eq!(next_rotation_index, dof_count);

    let mut global_stiffness = vec![vec![0.0; dof_count]; dof_count];
    let mut global_mass = vec![vec![0.0; dof_count]; dof_count];
    let mut global_translational_mass = vec![vec![0.0; dof_count]; dof_count];
    let mut global_rotary_mass = vec![vec![0.0; dof_count]; dof_count];

    for member in &ordered_members {
        let ids = station_ids
            .get(&member.id)
            .expect("station list created for each member");
        let rotations = member_rotation_indices
            .get(&member.id)
            .expect("rotation list created for each member");

        for element_index in 0..member.elements_per_member {
            let start_id = &ids[element_index];
            let end_id = &ids[element_index + 1];
            let start_index = *all_node_index
                .get(start_id)
                .expect("interior/start station inserted");
            let end_index = *all_node_index
                .get(end_id)
                .expect("interior/end station inserted");
            let start = &all_nodes[start_index];
            let end = &all_nodes[end_index];
            let dx = end.x_m - start.x_m;
            let dy = end.y_m - start.y_m;
            let local = euler_bernoulli_frame_element(
                dx.hypot(dy),
                member.area_m2,
                member.youngs_modulus_pa,
                member.density_kg_m3,
                member.second_moment_m4,
            )?;
            let global = transform_element_to_global(&local, dx, dy)?;
            let dof_map = [
                2 * start_index,
                2 * start_index + 1,
                rotations[element_index],
                2 * end_index,
                2 * end_index + 1,
                rotations[element_index + 1],
            ];
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
        ordered_node_ids: all_nodes.into_iter().map(|node| node.id).collect(),
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
    use std::collections::{BTreeMap, BTreeSet};

    use super::{
        BeamMemberChain, BeamPinDofKind, BeamPinMember, BeamPinNode,
        assemble_released_joint_beam_chains, assemble_released_joint_pin_truss,
        canonical_beam_chain_request,
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

    fn chains(elements_per_member: usize) -> Vec<BeamMemberChain> {
        members()
            .into_iter()
            .map(|member| BeamMemberChain {
                id: member.id,
                start_node_id: member.start_node_id,
                end_node_id: member.end_node_id,
                area_m2: member.area_m2,
                youngs_modulus_pa: member.youngs_modulus_pa,
                density_kg_m3: member.density_kg_m3,
                second_moment_m4: member.second_moment_m4,
                elements_per_member,
            })
            .collect()
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

    #[test]
    fn one_element_per_chain_matches_original_released_joint_assembly() {
        let original = assemble_released_joint_pin_truss(&nodes(), &members())
            .expect("single-element pin assembly");
        let chain = assemble_released_joint_beam_chains(&nodes(), &chains(1))
            .expect("one-element member chains");
        assert_eq!(original.ordered_node_ids, chain.ordered_node_ids);
        assert_eq!(original.ordered_member_ids, chain.ordered_member_ids);
        assert_eq!(original.dof_descriptors, chain.dof_descriptors);
        assert_eq!(original.fixed_dof_indices, chain.fixed_dof_indices);
        assert_eq!(original.free_dof_indices, chain.free_dof_indices);
        assert_eq!(original.stiffness_global, chain.stiffness_global);
        assert_eq!(original.mass_global, chain.mass_global);
        assert_eq!(
            original.translational_mass_global,
            chain.translational_mass_global
        );
        assert_eq!(
            original.rotary_inertia_mass_global,
            chain.rotary_inertia_mass_global
        );
    }

    #[test]
    fn mesh_refinement_shares_interior_rotations_without_creating_artificial_pins() {
        for elements_per_member in [2_usize, 4, 8, 16] {
            let assembly =
                assemble_released_joint_beam_chains(&nodes(), &chains(elements_per_member))
                    .expect("valid refined member-chain mesh");
            assert_eq!(assembly.free_dof_indices.len(), 6 * elements_per_member);
            assert_eq!(
                assembly.ordered_node_ids.len(),
                3 + 2 * (elements_per_member - 1)
            );

            let interior_rotations = assembly
                .dof_descriptors
                .iter()
                .filter_map(|descriptor| match &descriptor.kind {
                    BeamPinDofKind::MemberInteriorRotation {
                        member_id,
                        station_index,
                    } => Some((member_id.clone(), *station_index, descriptor.index)),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(interior_rotations.len(), 2 * (elements_per_member - 1));
            let unique_stations = interior_rotations
                .iter()
                .map(|(member_id, station_index, _)| (member_id.clone(), *station_index))
                .collect::<BTreeSet<_>>();
            assert_eq!(unique_stations.len(), interior_rotations.len());

            let mut translation_dofs = BTreeMap::<String, Vec<usize>>::new();
            for descriptor in &assembly.dof_descriptors {
                match &descriptor.kind {
                    BeamPinDofKind::NodeTranslationX { node_id }
                    | BeamPinDofKind::NodeTranslationY { node_id } => {
                        translation_dofs
                            .entry(node_id.clone())
                            .or_default()
                            .push(descriptor.index);
                    }
                    _ => {}
                }
            }

            let member_inputs = chains(elements_per_member);
            for (member_id, station_index, index) in &interior_rotations {
                assert!(assembly.stiffness_global[*index][*index] > 0.0);
                assert!(assembly.mass_global[*index][*index] > 0.0);
                let member = member_inputs
                    .iter()
                    .find(|member| member.id == *member_id)
                    .expect("member chain in test fixture");
                let previous_station = if *station_index == 1 {
                    member.start_node_id.clone()
                } else {
                    format!("{}::station::{:04}", member.id, station_index - 1)
                };
                let next_station = if *station_index + 1 == elements_per_member {
                    member.end_node_id.clone()
                } else {
                    format!("{}::station::{:04}", member.id, station_index + 1)
                };
                let previous_dofs = translation_dofs
                    .get(&previous_station)
                    .expect("previous station translation DOFs");
                let next_dofs = translation_dofs
                    .get(&next_station)
                    .expect("next station translation DOFs");
                assert!(
                    previous_dofs
                        .iter()
                        .any(|dof| { assembly.stiffness_global[*index][*dof].abs() > 0.0 }),
                    "rotation {member_id}/{station_index} must couple to the previous station"
                );
                assert!(
                    next_dofs
                        .iter()
                        .any(|dof| { assembly.stiffness_global[*index][*dof].abs() > 0.0 }),
                    "rotation {member_id}/{station_index} must couple to the next station"
                );
            }
            assert_eq!(
                assembly
                    .dof_descriptors
                    .iter()
                    .filter(|descriptor| matches!(
                        &descriptor.kind,
                        BeamPinDofKind::MemberEndRotation { .. }
                    ))
                    .count(),
                4
            );
        }
    }

    #[test]
    fn member_chain_request_is_frozen_and_storage_order_invariant() {
        let expected =
            include_str!("../fixtures/beam_chain_request_v0_1.txt").trim_end_matches('\n');
        let first = canonical_beam_chain_request(&nodes(), &chains(4))
            .expect("canonical beam-chain request");
        assert_eq!(first, expected);

        let mut reversed_nodes = nodes();
        reversed_nodes.reverse();
        let mut reversed_chains = chains(4);
        reversed_chains.reverse();
        assert_eq!(
            canonical_beam_chain_request(&reversed_nodes, &reversed_chains)
                .expect("canonical reversed beam-chain request"),
            first
        );
    }

    #[test]
    fn canonical_mesh_sweep_request_covers_levels_and_is_order_invariant() {
        let expected =
            include_str!("../fixtures/beam_chain_sweep_request_v0_1.txt").trim_end_matches('\n');
        let first = canonical_beam_chain_sweep_request(&nodes(), &chains(1), &[1, 2, 4, 8])
            .expect("canonical sweep request");
        assert_eq!(first, expected);

        let mut reversed_nodes = nodes();
        reversed_nodes.reverse();
        let mut reversed_chains = chains(1);
        reversed_chains.reverse();
        assert_eq!(
            canonical_beam_chain_sweep_request(&reversed_nodes, &reversed_chains, &[8, 4, 2, 1])
                .expect("canonical reversed sweep request"),
            first
        );
        assert!(canonical_beam_chain_sweep_request(&nodes(), &chains(1), &[1, 2, 2]).is_err());
        assert!(canonical_beam_chain_sweep_request(&nodes(), &chains(1), &[]).is_err());
    }

    #[test]
    fn member_chain_invalid_mesh_envelope_fails_closed() {
        assert!(assemble_released_joint_beam_chains(&nodes(), &chains(0)).is_err());
        assert!(assemble_released_joint_beam_chains(&nodes(), &chains(33)).is_err());

        let mut missing_joint = chains(2);
        missing_joint[0].end_node_id = "unknown".to_owned();
        assert!(assemble_released_joint_beam_chains(&nodes(), &missing_joint).is_err());
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
