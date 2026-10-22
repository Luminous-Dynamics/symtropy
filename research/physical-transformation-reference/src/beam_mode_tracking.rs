// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cross-mesh beam-mode correspondence using modal assurance criteria and a
//! global maximum-weight assignment. Only exact common physical stations are
//! compared. Rotation values are scaled by a canonical characteristic length;
//! translation and rotation MACs are also reported separately.

use crate::beam_mesh::assemble_released_joint_beam_mesh;
use crate::beam_modal::{BeamModalMode, BeamModalOptions, BeamModalSpectrum, solve_released_joint_beam_modes};
use crate::beam_pin_assembly::{BeamPinAssembly, BeamPinDofKind, BeamPinMember, BeamPinNode, canonical_assembly_request};
use crate::Check;
use std::collections::BTreeMap;

const MAX_ELEMENTS_PER_MEMBER: usize = 32;
const MAX_TOTAL_ELEMENTS: usize = 64;
const MAX_DIAGNOSTIC_RESIDUAL: f64 = 1.0e-8;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct StationKey {
    numerator: usize,
    denominator: usize,
}

impl StationKey {
    fn new(numerator: usize, denominator: usize) -> Self {
        let divisor = gcd(numerator, denominator).max(1);
        Self { numerator: numerator / divisor, denominator: denominator / divisor }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum PhysicalDofKey {
    NodeTranslationX { node_id: String },
    NodeTranslationY { node_id: String },
    MemberTranslationX { member_id: String, station: StationKey },
    MemberTranslationY { member_id: String, station: StationKey },
    MemberRotation { member_id: String, station: StationKey },
}

impl PhysicalDofKey {
    fn is_rotation(&self) -> bool {
        matches!(self, Self::MemberRotation { .. })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModeMacScores {
    pub translation_mac: Option<f64>,
    pub rotation_mac: Option<f64>,
    pub scaled_combined_mac: Option<f64>,
    pub common_translation_dof_count: usize,
    pub common_rotation_dof_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrackedModePair {
    pub previous_mode_ordinal: usize,
    pub current_mode_ordinal: usize,
    pub previous_frequency_hz: f64,
    pub current_frequency_hz: f64,
    pub relative_frequency_change: f64,
    pub mac: ModeMacScores,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrackedMeshLevel {
    pub elements_per_member: usize,
    pub total_element_count: usize,
    pub free_dof_count: usize,
    pub frequencies_hz: Vec<f64>,
    pub maximum_normalized_eigen_residual: f64,
    pub maximum_mass_norm_residual: f64,
    pub maximum_mass_orthogonality_residual: f64,
    pub pairs_from_previous: Option<Vec<TrackedModePair>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BeamModeTrackingReport {
    pub elements_per_member: Vec<usize>,
    pub requested_mode_count: usize,
    pub rotation_scale_reference_length_m: f64,
    pub matching_algorithm: &'static str,
    pub mac_coordinate_policy: &'static str,
    pub levels: Vec<TrackedMeshLevel>,
    pub claim_ceiling: &'static str,
}

pub fn canonical_beam_mode_tracking_request(
    nodes: &[BeamPinNode],
    members: &[BeamPinMember],
    element_counts: &[usize],
    requested_modes: usize,
) -> Check<String> {
    validate_tracking_request(members.len(), element_counts, requested_modes)?;
    let base_request = canonical_assembly_request(nodes, members)?;
    let length_scale = reference_length(nodes, members)?;
    Ok(format!(
        "schema=beam-mode-tracking-request-v0.1|elements_per_member={}|requested_modes={requested_modes}|pairing=common-physical-station-MAC|assignment=hungarian-max-weight|rotation_scale_reference_length_m={length_scale:.17e}|common_station_policy=intersection-of-exact-rational-member-stations|base_request={base_request}",
        element_counts.iter().map(usize::to_string).collect::<Vec<_>>().join(",")
    ))
}

pub fn run_beam_mode_tracking(
    nodes: &[BeamPinNode],
    members: &[BeamPinMember],
    element_counts: &[usize],
    requested_modes: usize,
) -> Check<BeamModeTrackingReport> {
    validate_tracking_request(members.len(), element_counts, requested_modes)?;
    let length_scale = reference_length(nodes, members)?;
    let mut levels: Vec<TrackedMeshLevel> = Vec::with_capacity(element_counts.len());
    let mut previous: Option<(usize, BeamPinAssembly, BeamModalSpectrum)> = None;

    for elements_per_member in element_counts {
        let mesh = assemble_released_joint_beam_mesh(nodes, members, *elements_per_member)?;
        let spectrum = solve_released_joint_beam_modes(
            &mesh.assembly,
            BeamModalOptions::first_modes(requested_modes),
        )?;
        if spectrum.modes.len() != requested_modes {
            return Err(format!("mesh {elements_per_member} returned an unexpected mode count"));
        }
        if spectrum.maximum_normalized_eigen_residual >= MAX_DIAGNOSTIC_RESIDUAL
            || spectrum.maximum_mass_norm_residual >= MAX_DIAGNOSTIC_RESIDUAL
            || spectrum.maximum_mass_orthogonality_residual >= MAX_DIAGNOSTIC_RESIDUAL
        {
            return Err(format!("mesh {elements_per_member} exceeded the declared modal diagnostic limit"));
        }

        let pairs = if let Some((previous_elements, previous_assembly, previous_spectrum)) = &previous {
            Some(match_adjacent_mesh_modes(
                previous_assembly,
                *previous_elements,
                &previous_spectrum.modes,
                &mesh.assembly,
                *elements_per_member,
                &spectrum.modes,
                length_scale,
            )?)
        } else {
            None
        };

        levels.push(TrackedMeshLevel {
            elements_per_member: *elements_per_member,
            total_element_count: mesh.total_element_count,
            free_dof_count: mesh.assembly.free_dof_indices.len(),
            frequencies_hz: spectrum.modes.iter().map(|mode| mode.frequency_hz).collect(),
            maximum_normalized_eigen_residual: spectrum.maximum_normalized_eigen_residual,
            maximum_mass_norm_residual: spectrum.maximum_mass_norm_residual,
            maximum_mass_orthogonality_residual: spectrum.maximum_mass_orthogonality_residual,
            pairs_from_previous: pairs,
        });
        previous = Some((*elements_per_member, mesh.assembly, spectrum));
    }

    Ok(BeamModeTrackingReport {
        elements_per_member: element_counts.to_vec(),
        requested_mode_count: requested_modes,
        rotation_scale_reference_length_m: length_scale,
        matching_algorithm: "Hungarian maximum-weight assignment over scaled combined MAC",
        mac_coordinate_policy: "exact common physical stations; rotations scaled by reference length; translation and rotation MAC reported separately",
        levels,
        claim_ceiling: "common-station mode-shape correspondence for small nested meshes only; no 200-element/member NAFEMS reproduction, experimental validation or structural safety",
    })
}

fn match_adjacent_mesh_modes(
    previous_assembly: &BeamPinAssembly,
    previous_elements: usize,
    previous_modes: &[BeamModalMode],
    current_assembly: &BeamPinAssembly,
    current_elements: usize,
    current_modes: &[BeamModalMode],
    rotation_scale: f64,
) -> Check<Vec<TrackedModePair>> {
    if previous_modes.is_empty() || previous_modes.len() != current_modes.len() {
        return Err("mode-pairing requires equal positive mode counts across adjacent meshes".to_owned());
    }
    let mut weights = vec![vec![0.0; current_modes.len()]; previous_modes.len()];
    let mut scores = vec![vec![None; current_modes.len()]; previous_modes.len()];

    for (previous_index, previous_mode) in previous_modes.iter().enumerate() {
        for (current_index, current_mode) in current_modes.iter().enumerate() {
            let score = common_station_mac_scores(
                previous_assembly,
                previous_elements,
                previous_mode,
                current_assembly,
                current_elements,
                current_mode,
                rotation_scale,
            )?;
            weights[previous_index][current_index] = score.scaled_combined_mac.unwrap_or(0.0);
            scores[previous_index][current_index] = Some(score);
        }
    }
    let assignment = maximum_weight_assignment(&weights)?;
    let mut pairs = Vec::with_capacity(previous_modes.len());
    for (previous_index, current_index) in assignment.into_iter().enumerate() {
        let previous_mode = &previous_modes[previous_index];
        let current_mode = &current_modes[current_index];
        let mac = scores[previous_index][current_index].clone().expect("score matrix populated");
        let relative_frequency_change =
            (current_mode.frequency_hz - previous_mode.frequency_hz) / previous_mode.frequency_hz;
        if !relative_frequency_change.is_finite() {
            return Err("tracked relative frequency change is non-finite".to_owned());
        }
        pairs.push(TrackedModePair {
            previous_mode_ordinal: previous_index + 1,
            current_mode_ordinal: current_index + 1,
            previous_frequency_hz: previous_mode.frequency_hz,
            current_frequency_hz: current_mode.frequency_hz,
            relative_frequency_change,
            mac,
        });
    }
    Ok(pairs)
}

fn common_station_mac_scores(
    previous_assembly: &BeamPinAssembly,
    previous_elements: usize,
    previous_mode: &BeamModalMode,
    current_assembly: &BeamPinAssembly,
    current_elements: usize,
    current_mode: &BeamModalMode,
    rotation_scale: f64,
) -> Check<ModeMacScores> {
    let previous_values = mode_dof_map(previous_assembly, previous_elements, previous_mode)?;
    let current_values = mode_dof_map(current_assembly, current_elements, current_mode)?;
    let mut translation_left = Vec::new();
    let mut translation_right = Vec::new();
    let mut rotation_left = Vec::new();
    let mut rotation_right = Vec::new();
    let mut combined_left = Vec::new();
    let mut combined_right = Vec::new();

    for (key, previous_value) in &previous_values {
        if let Some(current_value) = current_values.get(key) {
            if key.is_rotation() {
                rotation_left.push(*previous_value);
                rotation_right.push(*current_value);
                combined_left.push(*previous_value * rotation_scale);
                combined_right.push(*current_value * rotation_scale);
            } else {
                translation_left.push(*previous_value);
                translation_right.push(*current_value);
                combined_left.push(*previous_value);
                combined_right.push(*current_value);
            }
        }
    }
    let translation_count = translation_left.len();
    let rotation_count = rotation_left.len();
    if translation_count == 0 || rotation_count == 0 {
        return Err(format!(
            "insufficient common physical DOFs: translations={translation_count}, rotations={rotation_count}"
        ));
    }
    Ok(ModeMacScores {
        translation_mac: mac(&translation_left, &translation_right),
        rotation_mac: mac(&rotation_left, &rotation_right),
        scaled_combined_mac: mac(&combined_left, &combined_right),
        common_translation_dof_count: translation_count,
        common_rotation_dof_count: rotation_count,
    })
}

fn mode_dof_map(
    assembly: &BeamPinAssembly,
    elements_per_member: usize,
    mode: &BeamModalMode,
) -> Check<BTreeMap<PhysicalDofKey, f64>> {
    if mode.shape_global_dofs.len() != assembly.dof_descriptors.len() {
        return Err("mode-shape length does not match assembly DOF map".to_owned());
    }
    let mut result = BTreeMap::new();
    for descriptor in &assembly.dof_descriptors {
        let key = match &descriptor.kind {
            BeamPinDofKind::NodeTranslationX { node_id } => {
                PhysicalDofKey::NodeTranslationX { node_id: node_id.clone() }
            }
            BeamPinDofKind::NodeTranslationY { node_id } => {
                PhysicalDofKey::NodeTranslationY { node_id: node_id.clone() }
            }
            BeamPinDofKind::MemberInteriorTranslationX { member_id, station } => {
                PhysicalDofKey::MemberTranslationX {
                    member_id: member_id.clone(),
                    station: StationKey::new(*station, elements_per_member),
                }
            }
            BeamPinDofKind::MemberInteriorTranslationY { member_id, station } => {
                PhysicalDofKey::MemberTranslationY {
                    member_id: member_id.clone(),
                    station: StationKey::new(*station, elements_per_member),
                }
            }
            BeamPinDofKind::MemberEndRotation { member_id, at_start } => {
                PhysicalDofKey::MemberRotation {
                    member_id: member_id.clone(),
                    station: if *at_start { StationKey::new(0, 1) } else { StationKey::new(1, 1) },
                }
            }
            BeamPinDofKind::MemberInteriorRotation { member_id, station } => {
                PhysicalDofKey::MemberRotation {
                    member_id: member_id.clone(),
                    station: StationKey::new(*station, elements_per_member),
                }
            }
        };
        let value = mode.shape_global_dofs[descriptor.index];
        if !value.is_finite() {
            return Err("mode shape contains a non-finite component".to_owned());
        }
        if result.insert(key, value).is_some() {
            return Err("duplicate physical DOF key in assembly".to_owned());
        }
    }
    Ok(result)
}

fn mac(left: &[f64], right: &[f64]) -> Option<f64> {
    if left.is_empty() || left.len() != right.len() {
        return None;
    }
    let dot = left.iter().zip(right).map(|(a, b)| a * b).sum::<f64>();
    let left_norm = left.iter().map(|value| value * value).sum::<f64>();
    let right_norm = right.iter().map(|value| value * value).sum::<f64>();
    let denominator = left_norm * right_norm;
    if !dot.is_finite() || !denominator.is_finite() || denominator <= f64::MIN_POSITIVE {
        return None;
    }
    Some((dot * dot / denominator).clamp(0.0, 1.0))
}

fn maximum_weight_assignment(weights: &[Vec<f64>]) -> Check<Vec<usize>> {
    let size = weights.len();
    if size == 0
        || weights.iter().any(|row| row.len() != size
            || row.iter().any(|weight| !weight.is_finite() || !(0.0..=1.0).contains(weight)))
    {
        return Err("assignment matrix must be square with finite MAC weights in [0,1]".to_owned());
    }
    let mut u = vec![0.0; size + 1];
    let mut v = vec![0.0; size + 1];
    let mut p = vec![0usize; size + 1];
    let mut way = vec![0usize; size + 1];
    for row in 1..=size {
        p[0] = row;
        let mut column0 = 0usize;
        let mut minimum = vec![f64::INFINITY; size + 1];
        let mut used = vec![false; size + 1];
        loop {
            used[column0] = true;
            let row0 = p[column0];
            let mut delta = f64::INFINITY;
            let mut column1 = 0usize;
            for column in 1..=size {
                if !used[column] {
                    let cost = 1.0 - weights[row0 - 1][column - 1];
                    let current = cost - u[row0] - v[column];
                    if current < minimum[column] {
                        minimum[column] = current;
                        way[column] = column0;
                    }
                    if minimum[column] < delta {
                        delta = minimum[column];
                        column1 = column;
                    }
                }
            }
            if !delta.is_finite() {
                return Err("no finite augmenting path for mode assignment".to_owned());
            }
            for column in 0..=size {
                if used[column] {
                    u[p[column]] += delta;
                    v[column] -= delta;
                } else {
                    minimum[column] -= delta;
                }
            }
            column0 = column1;
            if p[column0] == 0 {
                break;
            }
        }
        loop {
            let column1 = way[column0];
            p[column0] = p[column1];
            column0 = column1;
            if column0 == 0 {
                break;
            }
        }
    }
    let mut assignment = vec![usize::MAX; size];
    for column in 1..=size {
        if p[column] == 0 || p[column] > size {
            return Err("assignment omitted a mode".to_owned());
        }
        assignment[p[column] - 1] = column - 1;
    }
    if assignment.iter().any(|column| *column == usize::MAX) {
        return Err("assignment is incomplete".to_owned());
    }
    Ok(assignment)
}

fn validate_tracking_request(
    member_count: usize,
    element_counts: &[usize],
    requested_modes: usize,
) -> Check<()> {
    if element_counts.is_empty()
        || element_counts.iter().any(|count| *count == 0 || *count > MAX_ELEMENTS_PER_MEMBER)
        || element_counts.windows(2).any(|pair| pair[1] <= pair[0] || pair[1] % pair[0] != 0)
    {
        return Err("mesh levels must be nested, strictly increasing, and in 1..=32".to_owned());
    }
    if element_counts.iter().any(|count| member_count.saturating_mul(*count) > MAX_TOTAL_ELEMENTS) {
        return Err("mesh exceeds the 64-element dense reference limit".to_owned());
    }
    if requested_modes == 0 {
        return Err("requested mode count must be positive".to_owned());
    }
    Ok(())
}

fn reference_length(nodes: &[BeamPinNode], members: &[BeamPinMember]) -> Check<f64> {
    let mut node_map = BTreeMap::new();
    for node in nodes {
        if !node.x_m.is_finite() || !node.y_m.is_finite() {
            return Err("non-finite node coordinates in mode-tracking input".to_owned());
        }
        node_map.insert(node.id.as_str(), (node.x_m, node.y_m));
    }
    let mut maximum = 0.0_f64;
    for member in members {
        let start = node_map.get(member.start_node_id.as_str())
            .ok_or_else(|| format!("missing start node {}", member.start_node_id))?;
        let end = node_map.get(member.end_node_id.as_str())
            .ok_or_else(|| format!("missing end node {}", member.end_node_id))?;
        maximum = maximum.max((end.0 - start.0).hypot(end.1 - start.1));
    }
    if !maximum.is_finite() || maximum <= 0.0 {
        return Err("reference length must be finite and positive".to_owned());
    }
    Ok(maximum)
}

fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_beam_mode_tracking_request, maximum_weight_assignment, run_beam_mode_tracking,
    };
    use crate::beam_pin_assembly::nafems_nbc05_released_joint_beam_fixture;
    use std::collections::BTreeSet;

    const ELEMENT_COUNTS: [usize; 4] = [1, 2, 4, 8];
    const REQUESTED_MODES: usize = 6;

    #[test]
    fn canonical_mode_tracking_request_matches_frozen_fixture() {
        let (nodes, members) = nafems_nbc05_released_joint_beam_fixture();
        let request = canonical_beam_mode_tracking_request(&nodes, &members, &ELEMENT_COUNTS, REQUESTED_MODES)
            .expect("canonical mode tracking request");
        assert_eq!(
            request,
            include_str!("../fixtures/beam_mode_tracking_request_v0_1.txt")
                .trim_end_matches('\\n')
        );
    }

    #[test]
    fn mode_tracking_reports_mac_and_bijective_assignments_across_nested_meshes() {
        let (nodes, members) = nafems_nbc05_released_joint_beam_fixture();
        let report = run_beam_mode_tracking(&nodes, &members, &ELEMENT_COUNTS, REQUESTED_MODES)
            .expect("nested mesh mode tracking");
        assert_eq!(report.levels.len(), ELEMENT_COUNTS.len());
        assert!((report.rotation_scale_reference_length_m - 2.0_f64.sqrt()).abs() < 1.0e-14);
        assert_eq!(report.levels[0].pairs_from_previous, None);
        let expected_common_dofs = [(6, 4), (10, 6), (18, 10)];
        for (level_index, level) in report.levels[1..].iter().enumerate() {
            let pairs = level.pairs_from_previous.as_ref().expect("pairs");
            assert_eq!(pairs.len(), REQUESTED_MODES);
            let ordinals = pairs.iter().map(|pair| pair.current_mode_ordinal).collect::<BTreeSet<_>>();
            assert_eq!(ordinals.len(), REQUESTED_MODES);
            let (expected_translation_dofs, expected_rotation_dofs) = expected_common_dofs[level_index];
            assert!(pairs.iter().all(|pair| pair.mac.common_translation_dof_count == expected_translation_dofs));
            assert!(pairs.iter().all(|pair| pair.mac.common_rotation_dof_count == expected_rotation_dofs));
            assert!(pairs.iter().all(|pair| pair.mac.scaled_combined_mac.is_none_or(|score| (0.0..=1.0).contains(&score))));
        }
    }

    #[test]
    fn reference_length_changes_combined_mac_when_translation_and_rotation_mix() {
        use super::common_station_mac_scores;
        use crate::beam_modal::BeamModalMode;
        use crate::beam_pin_assembly::{
            BeamPinAssembly, BeamPinDofDescriptor, BeamPinDofKind,
        };

        let assembly = BeamPinAssembly {
            ordered_node_ids: vec!["C".to_owned()],
            ordered_member_ids: vec!["M".to_owned()],
            dof_descriptors: vec![
                BeamPinDofDescriptor {
                    index: 0,
                    kind: BeamPinDofKind::NodeTranslationX { node_id: "C".to_owned() },
                },
                BeamPinDofDescriptor {
                    index: 1,
                    kind: BeamPinDofKind::NodeTranslationY { node_id: "C".to_owned() },
                },
                BeamPinDofDescriptor {
                    index: 2,
                    kind: BeamPinDofKind::MemberEndRotation { member_id: "M".to_owned(), at_start: true },
                },
                BeamPinDofDescriptor {
                    index: 3,
                    kind: BeamPinDofKind::MemberEndRotation { member_id: "M".to_owned(), at_start: false },
                },
            ],
            stiffness_global: vec![vec![0.0; 4]; 4],
            mass_global: vec![vec![0.0; 4]; 4],
            translational_mass_global: vec![vec![0.0; 4]; 4],
            rotary_inertia_mass_global: vec![vec![0.0; 4]; 4],
            fixed_dof_indices: vec![],
            free_dof_indices: vec![0, 1, 2, 3],
        };
        let previous = BeamModalMode {
            eigenvalue_rad_s_squared: 1.0,
            frequency_hz: 1.0,
            shape_free_dofs: vec![1.0, 0.0, 1.0, 0.0],
            shape_global_dofs: vec![1.0, 0.0, 1.0, 0.0],
            normalized_eigen_residual: 0.0,
            mass_norm_residual: 0.0,
        };
        let current = BeamModalMode {
            eigenvalue_rad_s_squared: 2.0,
            frequency_hz: 2.0,
            shape_free_dofs: vec![1.0, 0.0, 0.25, 0.0],
            shape_global_dofs: vec![1.0, 0.0, 0.25, 0.0],
            normalized_eigen_residual: 0.0,
            mass_norm_residual: 0.0,
        };
        let unit_scale = common_station_mac_scores(&assembly, 1, &previous, &assembly, 1, &current, 1.0)
            .expect("unit rotation scale");
        let long_scale = common_station_mac_scores(&assembly, 1, &previous, &assembly, 1, &current, 10.0)
            .expect("long rotation scale");
        assert_eq!(unit_scale.translation_mac, Some(1.0));
        assert_eq!(unit_scale.rotation_mac, Some(1.0));
        assert!(
            (unit_scale.scaled_combined_mac.unwrap() - long_scale.scaled_combined_mac.unwrap()).abs()
                > 0.1,
            "characteristic-length scaling must alter the translation/rotation mixture score"
        );
    }

    #[test]
    fn hungarian_assignment_finds_global_not_greedy_optimum() {
        let assignment = maximum_weight_assignment(&[
            vec![0.90, 0.80, 0.10],
            vec![0.85, 0.20, 0.10],
            vec![0.10, 0.15, 0.95],
        ]).expect("assignment");
        assert_eq!(assignment, vec![1, 0, 2]);
    }

    #[test]
    fn tracking_request_records_nested_levels_mode_count_and_rotation_scale() {
        let (nodes, members) = nafems_nbc05_released_joint_beam_fixture();
        let request = canonical_beam_mode_tracking_request(&nodes, &members, &ELEMENT_COUNTS, REQUESTED_MODES)
            .expect("canonical request");
        assert!(request.contains("elements_per_member=1,2,4,8"));
        assert!(request.contains("requested_modes=6"));
        assert!(request.contains("pairing=common-physical-station-MAC"));
        assert!(request.contains("hungarian-max-weight"));
        assert!(request.contains("rotation_scale_reference_length_m=1.41421356237309515e0"));
    }

    #[test]
    fn invalid_non_nested_meshes_and_invalid_assignment_matrices_fail_closed() {
        let (nodes, members) = nafems_nbc05_released_joint_beam_fixture();
        assert!(run_beam_mode_tracking(&nodes, &members, &[1, 3, 6], 6).is_err());
        assert!(maximum_weight_assignment(&[]).is_err());
        assert!(maximum_weight_assignment(&[vec![1.01]]).is_err());
    }
}
