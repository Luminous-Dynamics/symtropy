// SPDX-License-Identifier: AGPL-3.0-or-later
//! Release-aware planar beam mesh assembly.
//!
//! Interior mesh nodes retain rotational continuity, while explicitly released
//! physical endpoints receive private rotations. Translational DOFs remain
//! shared at geometric nodes. This makes mesh refinement different from
//! accidentally pinning every element boundary.
//!
//! Assembly/topology only: no modal solver, mechanism classifier, convergence
//! claim, or complete NAFEMS NBC05 reproduction.

use crate::beam_element::{euler_bernoulli_frame_element, transform_element_to_global, Matrix6};
use crate::Check;
use std::collections::{BTreeMap, BTreeSet};

const MAX_NODES: usize = 512;
const MAX_ELEMENTS: usize = 1024;

#[derive(Clone, Debug, PartialEq)]
pub struct BeamMeshNode {
    pub id: String,
    pub x_m: f64,
    pub y_m: f64,
    pub restrain_x: bool,
    pub restrain_y: bool,
    pub restrain_rotation: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BeamMeshElement {
    pub id: String,
    pub start_node_id: String,
    pub end_node_id: String,
    /// A released element endpoint receives a private member-end rotation.
    pub release_start: bool,
    pub release_end: bool,
    pub area_m2: f64,
    pub youngs_modulus_pa: f64,
    pub density_kg_m3: f64,
    pub second_moment_m4: f64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BeamMeshDofKind {
    NodeTranslationX { node_id: String },
    NodeTranslationY { node_id: String },
    NodeRotation { node_id: String },
    ElementEndRotation { element_id: String, at_start: bool },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BeamMeshDofDescriptor {
    pub index: usize,
    pub kind: BeamMeshDofKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BeamMeshAssembly {
    pub ordered_node_ids: Vec<String>,
    pub ordered_element_ids: Vec<String>,
    pub dof_descriptors: Vec<BeamMeshDofDescriptor>,
    pub stiffness_global: Vec<Vec<f64>>,
    pub mass_global: Vec<Vec<f64>>,
    pub translational_mass_global: Vec<Vec<f64>>,
    pub rotary_inertia_mass_global: Vec<Vec<f64>>,
    pub fixed_dof_indices: Vec<usize>,
    pub free_dof_indices: Vec<usize>,
}

/// Canonical UTF-8 request. Inputs are validated first, then sorted by stable ID.
pub fn canonical_beam_mesh_request(
    nodes: &[BeamMeshNode],
    elements: &[BeamMeshElement],
) -> Check<String> {
    assemble_beam_mesh(nodes, elements)?;
    let mut ordered_nodes = nodes.to_vec();
    ordered_nodes.sort_by(|left, right| left.id.cmp(&right.id));
    let mut ordered_elements = elements.to_vec();
    ordered_elements.sort_by(|left, right| left.id.cmp(&right.id));
    let mut fields = vec![
        "schema=beam-mesh-assembly-request-v0.1".to_owned(),
        "fixture=PHYS-BEAM-MESH-CONTINUITY-001".to_owned(),
    ];
    for node in ordered_nodes {
        fields.push(format!(
            "node|{}|{}|{}|{}|{}|{}",
            node.id, node.x_m, node.y_m, node.restrain_x, node.restrain_y, node.restrain_rotation
        ));
    }
    for element in ordered_elements {
        fields.push(format!(
            "element|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            element.id,
            element.start_node_id,
            element.end_node_id,
            element.release_start,
            element.release_end,
            element.area_m2,
            element.youngs_modulus_pa,
            element.density_kg_m3,
            element.second_moment_m4,
            "euler-bernoulli-consistent-translational-plus-rotary-mass"
        ));
    }
    Ok(fields.join("|"))
}

/// Assemble a 2D Euler-Bernoulli beam mesh with explicit endpoint releases.
///
/// Deterministic DOF order:
/// 1. x/y translations in sorted node-ID order;
/// 2. shared node rotations for nodes with at least one unreleased endpoint;
/// 3. private rotations for released endpoints in sorted element-ID order,
///    start endpoint before end endpoint.
pub fn assemble_beam_mesh(
    nodes: &[BeamMeshNode],
    elements: &[BeamMeshElement],
) -> Check<BeamMeshAssembly> {
    if nodes.is_empty() || nodes.len() > MAX_NODES {
        return Err(format!("node count must be between 1 and {MAX_NODES}"));
    }
    if elements.is_empty() || elements.len() > MAX_ELEMENTS {
        return Err(format!("element count must be between 1 and {MAX_ELEMENTS}"));
    }

    let mut ordered_nodes = nodes.to_vec();
    ordered_nodes.sort_by(|left, right| left.id.cmp(&right.id));
    let mut node_index_by_id = BTreeMap::new();
    for (index, node) in ordered_nodes.iter().enumerate() {
        validate_id("node ID", &node.id)?;
        if !node.x_m.is_finite() || !node.y_m.is_finite() {
            return Err(format!("node {} has non-finite coordinates", node.id));
        }
        if node_index_by_id.insert(node.id.as_str(), index).is_some() {
            return Err(format!("duplicate node ID: {}", node.id));
        }
    }

    let mut ordered_elements = elements.to_vec();
    ordered_elements.sort_by(|left, right| left.id.cmp(&right.id));
    let mut element_ids = BTreeSet::new();
    let mut used_nodes = BTreeSet::new();
    let mut node_requires_rotation = vec![false; ordered_nodes.len()];
    for element in &ordered_elements {
        validate_id("element ID", &element.id)?;
        validate_id("start node ID", &element.start_node_id)?;
        validate_id("end node ID", &element.end_node_id)?;
        if !element_ids.insert(element.id.as_str()) {
            return Err(format!("duplicate element ID: {}", element.id));
        }
        if element.start_node_id == element.end_node_id {
            return Err(format!("element {} has identical endpoint IDs", element.id));
        }
        let start_index = *node_index_by_id.get(element.start_node_id.as_str()).ok_or_else(|| {
            format!("element {} references unknown start node {}", element.id, element.start_node_id)
        })?;
        let end_index = *node_index_by_id.get(element.end_node_id.as_str()).ok_or_else(|| {
            format!("element {} references unknown end node {}", element.id, element.end_node_id)
        })?;
        used_nodes.insert(element.start_node_id.as_str());
        used_nodes.insert(element.end_node_id.as_str());
        if !element.release_start {
            node_requires_rotation[start_index] = true;
        }
        if !element.release_end {
            node_requires_rotation[end_index] = true;
        }
        let length = (ordered_nodes[end_index].x_m - ordered_nodes[start_index].x_m)
            .hypot(ordered_nodes[end_index].y_m - ordered_nodes[start_index].y_m);
        if !length.is_finite() || length <= 0.0 {
            return Err(format!("element {} has non-positive length", element.id));
        }
    }
    for node in &ordered_nodes {
        if !used_nodes.contains(node.id.as_str()) {
            return Err(format!("isolated node is not allowed: {}", node.id));
        }
    }
    for (index, node) in ordered_nodes.iter().enumerate() {
        if node.restrain_rotation && !node_requires_rotation[index] {
            return Err(format!(
                "node {} restrains rotation but has no shared rotational DOF",
                node.id
            ));
        }
    }

    let translation_count = 2 * ordered_nodes.len();
    let mut descriptors = Vec::new();
    let mut fixed = Vec::new();
    for (node_index, node) in ordered_nodes.iter().enumerate() {
        let x_index = 2 * node_index;
        let y_index = x_index + 1;
        descriptors.push(BeamMeshDofDescriptor {
            index: x_index,
            kind: BeamMeshDofKind::NodeTranslationX { node_id: node.id.clone() },
        });
        descriptors.push(BeamMeshDofDescriptor {
            index: y_index,
            kind: BeamMeshDofKind::NodeTranslationY { node_id: node.id.clone() },
        });
        if node.restrain_x {
            fixed.push(x_index);
        }
        if node.restrain_y {
            fixed.push(y_index);
        }
    }

    let mut next_rotation_index = translation_count;
    let mut shared_rotation_by_node = BTreeMap::new();
    for (node_index, node) in ordered_nodes.iter().enumerate() {
        if node_requires_rotation[node_index] {
            let index = next_rotation_index;
            next_rotation_index += 1;
            shared_rotation_by_node.insert(node.id.as_str(), index);
            descriptors.push(BeamMeshDofDescriptor {
                index,
                kind: BeamMeshDofKind::NodeRotation { node_id: node.id.clone() },
            });
            if node.restrain_rotation {
                fixed.push(index);
            }
        }
    }

    let mut released_rotation_by_endpoint = BTreeMap::new();
    for element in &ordered_elements {
        for (at_start, released) in [(true, element.release_start), (false, element.release_end)] {
            if released {
                let index = next_rotation_index;
                next_rotation_index += 1;
                released_rotation_by_endpoint.insert((element.id.as_str(), at_start), index);
                descriptors.push(BeamMeshDofDescriptor {
                    index,
                    kind: BeamMeshDofKind::ElementEndRotation {
                        element_id: element.id.clone(),
                        at_start,
                    },
                });
            }
        }
    }

    let dof_count = next_rotation_index;
    let mut stiffness = zero_matrix(dof_count);
    let mut mass = zero_matrix(dof_count);
    let mut translational_mass = zero_matrix(dof_count);
    let mut rotary_mass = zero_matrix(dof_count);

    for element in &ordered_elements {
        let start_index = *node_index_by_id
            .get(element.start_node_id.as_str())
            .expect("validated start node");
        let end_index = *node_index_by_id
            .get(element.end_node_id.as_str())
            .expect("validated end node");
        let start = &ordered_nodes[start_index];
        let end = &ordered_nodes[end_index];
        let dx = end.x_m - start.x_m;
        let dy = end.y_m - start.y_m;
        let length = dx.hypot(dy);
        let local = euler_bernoulli_frame_element(
            length,
            element.area_m2,
            element.youngs_modulus_pa,
            element.density_kg_m3,
            element.second_moment_m4,
        )?;
        let global = transform_element_to_global(&local, dx, dy)?;

        let start_rotation = if element.release_start {
            *released_rotation_by_endpoint
                .get(&(element.id.as_str(), true))
                .expect("released start rotation allocated")
        } else {
            *shared_rotation_by_node
                .get(element.start_node_id.as_str())
                .expect("shared start rotation allocated")
        };
        let end_rotation = if element.release_end {
            *released_rotation_by_endpoint
                .get(&(element.id.as_str(), false))
                .expect("released end rotation allocated")
        } else {
            *shared_rotation_by_node
                .get(element.end_node_id.as_str())
                .expect("shared end rotation allocated")
        };
        let map = [
            2 * start_index,
            2 * start_index + 1,
            start_rotation,
            2 * end_index,
            2 * end_index + 1,
            end_rotation,
        ];
        assemble_matrix(&mut stiffness, &global.stiffness_global, &map);
        assemble_matrix(&mut mass, &global.mass_global, &map);
        assemble_matrix(&mut translational_mass, &global.translational_mass_global, &map);
        assemble_matrix(&mut rotary_mass, &global.rotary_inertia_mass_global, &map);
    }

    descriptors.sort_by_key(|descriptor| descriptor.index);
    fixed.sort_unstable();
    fixed.dedup();
    let fixed_set = fixed.iter().copied().collect::<BTreeSet<_>>();
    let free = (0..dof_count)
        .filter(|index| !fixed_set.contains(index))
        .collect::<Vec<_>>();
    for matrix in [&stiffness, &mass, &translational_mass, &rotary_mass] {
        ensure_finite_matrix(matrix)?;
        ensure_symmetric(matrix, 1.0e-10)?;
    }

    Ok(BeamMeshAssembly {
        ordered_node_ids: ordered_nodes.into_iter().map(|node| node.id).collect(),
        ordered_element_ids: ordered_elements.into_iter().map(|element| element.id).collect(),
        dof_descriptors: descriptors,
        stiffness_global: stiffness,
        mass_global: mass,
        translational_mass_global: translational_mass,
        rotary_inertia_mass_global: rotary_mass,
        fixed_dof_indices: fixed,
        free_dof_indices: free,
    })
}

fn validate_id(name: &str, value: &str) -> Check<()> {
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

fn zero_matrix(size: usize) -> Vec<Vec<f64>> {
    vec![vec![0.0; size]; size]
}

fn assemble_matrix(global: &mut [Vec<f64>], element: &Matrix6, map: &[usize; 6]) {
    for row in 0..6 {
        for col in 0..6 {
            global[map[row]][map[col]] += element[row][col];
        }
    }
}

fn ensure_finite_matrix(matrix: &[Vec<f64>]) -> Check<()> {
    if matrix.iter().flatten().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err("beam mesh assembly contains a non-finite entry".to_owned())
    }
}

fn ensure_symmetric(matrix: &[Vec<f64>], tolerance: f64) -> Check<()> {
    for row in 0..matrix.len() {
        for col in (row + 1)..matrix.len() {
            let left = matrix[row][col];
            let right = matrix[col][row];
            let scale = left.abs().max(right.abs()).max(1.0);
            if (left - right).abs() > tolerance * scale {
                return Err(format!("beam mesh matrix is not symmetric at ({row},{col})"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use super::{assemble_beam_mesh, canonical_beam_mesh_request, BeamMeshDofKind, BeamMeshElement, BeamMeshNode};

    fn nodes() -> Vec<BeamMeshNode> {
        vec![
            BeamMeshNode { id: "A".into(), x_m: 0.0, y_m: 1.0, restrain_x: true, restrain_y: true, restrain_rotation: false },
            BeamMeshNode { id: "B".into(), x_m: 0.0, y_m: 0.0, restrain_x: true, restrain_y: true, restrain_rotation: false },
            BeamMeshNode { id: "C".into(), x_m: 1.0, y_m: 1.0, restrain_x: false, restrain_y: false, restrain_rotation: false },
            BeamMeshNode { id: "D".into(), x_m: 0.5, y_m: 1.0, restrain_x: false, restrain_y: false, restrain_rotation: false },
            BeamMeshNode { id: "E".into(), x_m: 0.5, y_m: 0.5, restrain_x: false, restrain_y: false, restrain_rotation: false },
        ]
    }

    fn elements() -> Vec<BeamMeshElement> {
        let radius_m = 0.01_f64;
        let area_m2 = std::f64::consts::PI * radius_m.powi(2);
        let second_moment_m4 = std::f64::consts::PI * radius_m.powi(4) / 4.0;
        let make = |id: &str, start: &str, end: &str, release_start, release_end| BeamMeshElement {
            id: id.to_owned(), start_node_id: start.to_owned(), end_node_id: end.to_owned(),
            release_start, release_end, area_m2, youngs_modulus_pa: 210.0e9,
            density_kg_m3: 7800.0, second_moment_m4,
        };
        vec![
            make("AC-1", "A", "D", true, false),
            make("AC-2", "D", "C", false, true),
            make("BC-1", "B", "E", true, false),
            make("BC-2", "E", "C", false, true),
        ]
    }

    #[test]
    fn refinement_keeps_interior_rotation_continuous_and_joint_releases_private() {
        let result = assemble_beam_mesh(&nodes(), &elements()).expect("refined beam mesh");
        assert_eq!(result.ordered_node_ids, vec!["A", "B", "C", "D", "E"]);
        assert_eq!(result.ordered_element_ids, vec!["AC-1", "AC-2", "BC-1", "BC-2"]);
        assert_eq!(result.stiffness_global.len(), 16);
        assert_eq!(result.fixed_dof_indices, vec![0, 1, 2, 3]);
        assert_eq!(result.free_dof_indices.len(), 12);

        let shared_rotations = result.dof_descriptors.iter().filter_map(|descriptor| match &descriptor.kind {
            BeamMeshDofKind::NodeRotation { node_id } => Some(node_id.as_str()),
            _ => None,
        }).collect::<BTreeSet<_>>();
        assert_eq!(shared_rotations, BTreeSet::from(["D", "E"]));

        let private_rotations = result.dof_descriptors.iter().filter_map(|descriptor| match &descriptor.kind {
            BeamMeshDofKind::ElementEndRotation { element_id, at_start } => Some((element_id.as_str(), *at_start)),
            _ => None,
        }).collect::<BTreeSet<_>>();
        assert_eq!(private_rotations, BTreeSet::from([
            ("AC-1", true), ("AC-2", false), ("BC-1", true), ("BC-2", false),
        ]));
    }

    #[test]
    fn assembly_is_storage_order_invariant_and_canonical_bytes_are_pinned() {
        let first = assemble_beam_mesh(&nodes(), &elements()).expect("ordered mesh");
        let request = canonical_beam_mesh_request(&nodes(), &elements()).expect("canonical request");
        assert_eq!(request, include_str!("../fixtures/beam_mesh_continuity_request_v0_1.txt").trim_end_matches('\n'));

        let mut reversed_nodes = nodes();
        reversed_nodes.reverse();
        let mut reversed_elements = elements();
        reversed_elements.reverse();
        assert_eq!(assemble_beam_mesh(&reversed_nodes, &reversed_elements).expect("reversed mesh"), first);
        assert_eq!(canonical_beam_mesh_request(&reversed_nodes, &reversed_elements).expect("canonical reversed request"), request);
    }

    #[test]
    fn invalid_release_and_topology_declarations_fail_closed() {
        let mut bad = elements();
        bad[0].start_node_id = "MISSING".to_owned();
        assert!(assemble_beam_mesh(&nodes(), &bad).is_err());

        let mut restrained_rotation = nodes();
        restrained_rotation[0].restrain_rotation = true;
        assert!(assemble_beam_mesh(&restrained_rotation, &elements()).is_err());

        let mut isolated = nodes();
        isolated.push(BeamMeshNode {
            id: "Z".to_owned(), x_m: 4.0, y_m: 4.0, restrain_x: false, restrain_y: false, restrain_rotation: false,
        });
        assert!(assemble_beam_mesh(&isolated, &elements()).is_err());
    }
}
