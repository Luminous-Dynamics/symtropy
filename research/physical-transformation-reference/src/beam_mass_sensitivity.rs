// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explicit model-form sensitivity for the rotary-inertia contribution to
//! released-joint Euler-Bernoulli beam modal analysis.
//!
//! This compares two distinct, precisely named models:
//! 1. consistent translational plus rotary-inertia mass;
//! 2. the same stiffness with consistent translational mass only.
//!
//! The second case is a sensitivity counterfactual, not a claim that all
//! commercial "lumped" formulations omit rotary inertia. Industry codes use
//! different element and lumping conventions; compare them only after the
//! formulation contract is matched.

use crate::beam_mesh::assemble_released_joint_beam_mesh;
use crate::beam_modal::{BeamModalMode, BeamModalOptions, BeamModalSpectrum, solve_released_joint_beam_modes};
use crate::beam_pin_assembly::{
    BeamPinAssembly, BeamPinDofKind, BeamPinMember, BeamPinNode, canonical_assembly_request,
};
use crate::Check;

const MAX_MESH_LEVELS: usize = 8;
const MAX_REQUESTED_MODES: usize = 12;
const MIN_COMBINED_MAC_FOR_CONFIDENT_PAIRING: f64 = 0.90;
const MASS_MONOTONIC_TOLERANCE: f64 = 1.0e-8;

/// Separate MAC scores for physical translations, rotations, and a combined
/// vector in which rotations are scaled by a declared characteristic length.
#[derive(Clone, Debug, PartialEq)]
pub struct MassSensitivityMac {
    pub translation_mac: Option<f64>,
    pub rotation_mac: Option<f64>,
    pub scaled_combined_mac: f64,
}

/// Sensitivity of one baseline mode to the omission of rotary inertia.
/// Ordinal and shape-paired frequency shifts are both retained because they
/// answer different questions and should not be silently conflated.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamMassSensitivityMode {
    pub consistent_mode_ordinal: usize,
    pub no_rotary_shape_paired_mode_ordinal: usize,
    pub consistent_frequency_hz: f64,
    pub no_rotary_frequency_at_same_ordinal_hz: f64,
    pub no_rotary_frequency_shape_paired_hz: f64,
    pub relative_frequency_shift_by_ordinal: f64,
    pub relative_frequency_shift_by_shape: f64,
    pub mac: MassSensitivityMac,
    pub shape_pairing_status: &'static str,
}

/// Results for one mesh level.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamMassSensitivityPoint {
    pub elements_per_member: usize,
    pub total_element_count: usize,
    pub free_dof_count: usize,
    pub modes: Vec<BeamMassSensitivityMode>,
    pub maximum_consistent_eigen_residual: f64,
    pub maximum_no_rotary_eigen_residual: f64,
    pub minimum_ordinal_relative_frequency_shift: f64,
    pub maximum_ordinal_relative_frequency_shift: f64,
    pub confident_shape_pair_count: usize,
    pub ambiguous_shape_pair_count: usize,
}

/// Complete controlled sensitivity report. It is not a validation report:
/// no experimental or full NAFEMS beam reference is claimed here.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamMassSensitivityReport {
    pub elements_per_member: Vec<usize>,
    pub requested_mode_count: usize,
    pub rotation_characteristic_length_m: f64,
    pub minimum_combined_mac_for_confident_pairing: f64,
    pub points: Vec<BeamMassSensitivityPoint>,
    pub consistent_mass_model: &'static str,
    pub sensitivity_counterfactual: &'static str,
    pub claim_ceiling: &'static str,
}

/// Create the request bytes for a reproducible model-form sensitivity sweep.
/// The element count, requested modes, rotational scaling convention, and both
/// mass-model definitions are part of the canonical request.
pub fn canonical_beam_mass_sensitivity_request(
    nodes: &[BeamPinNode],
    members: &[BeamPinMember],
    elements_per_member: &[usize],
    requested_modes: usize,
) -> Check<String> {
    validate_mesh_levels(members.len(), elements_per_member, requested_modes)?;
    let base = canonical_assembly_request(nodes, members)?;
    let rotation_scale = characteristic_length(nodes, members)?;
    Ok(format!(
        concat!(
            "schema=beam-mass-formulation-sensitivity-v0.1",
            "|fixture=PHYS-BEAM-MASS-SENS-001",
            "|elements_per_member={}",
            "|requested_modes={requested_modes}",
            "|rotation_scale_policy=max_member_length",
            "|rotation_scale_length_m={rotation_scale}",
            "|consistent_mass_model=translational_plus_rotary_inertia",
            "|sensitivity_counterfactual=consistent_translational_mass_only",
            "|pairing=maximum_weight_scaled_combined_mac",
            "|confident_pair_threshold={MIN_COMBINED_MAC_FOR_CONFIDENT_PAIRING}",
            "|mass_order_tolerance={MASS_MONOTONIC_TOLERANCE}",
            "|base_request={base}"
        ),
        elements_per_member
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",")
    ))
}

/// Run a deterministic rotary-inertia sensitivity sweep over the supplied
/// nested mesh levels. Modes are paired by maximum total scaled-combined MAC;
/// ordinal frequency shifts are reported separately as the monotonicity check
/// for removing a positive-semidefinite inertia contribution.
pub fn run_beam_mass_formulation_sensitivity(
    nodes: &[BeamPinNode],
    members: &[BeamPinMember],
    elements_per_member: &[usize],
    requested_modes: usize,
) -> Check<BeamMassSensitivityReport> {
    validate_mesh_levels(members.len(), elements_per_member, requested_modes)?;
    let rotation_scale = characteristic_length(nodes, members)?;
    let mut points = Vec::with_capacity(elements_per_member.len());

    for &element_count in elements_per_member {
        let mesh = assemble_released_joint_beam_mesh(nodes, members, element_count)?;
        let consistent = solve_released_joint_beam_modes(
            &mesh.assembly,
            BeamModalOptions::first_modes(requested_modes),
        )?;

        let mut without_rotary = mesh.assembly.clone();
        // Use the assembled translational part of the consistent mass matrix,
        // not an invented or unlabeled diagonal "lumped" approximation.
        without_rotary.mass_global = without_rotary.translational_mass_global.clone();
        let no_rotary = solve_released_joint_beam_modes(
            &without_rotary,
            BeamModalOptions::first_modes(requested_modes),
        )?;

        let mac_matrix = consistent
            .modes
            .iter()
            .map(|left| {
                no_rotary
                    .modes
                    .iter()
                    .map(|right| scaled_combined_mac(
                        &mesh.assembly,
                        left,
                        right,
                        rotation_scale,
                    ))
                    .collect::<Check<Vec<_>>>()
            })
            .collect::<Check<Vec<_>>>()?;
        let assignment = maximum_weight_assignment(&mac_matrix)?;

        let mut modes = Vec::with_capacity(requested_modes);
        let mut min_rank_shift = f64::INFINITY;
        let mut max_rank_shift = f64::NEG_INFINITY;
        let mut confident = 0usize;
        let mut ambiguous = 0usize;

        for (consistent_index, no_rotary_index) in assignment.iter().copied().enumerate() {
            let baseline = &consistent.modes[consistent_index];
            let paired = &no_rotary.modes[no_rotary_index];
            let same_rank = &no_rotary.modes[consistent_index];
            let relative_frequency_shift_by_ordinal =
                (same_rank.frequency_hz - baseline.frequency_hz) / baseline.frequency_hz;
            if !relative_frequency_shift_by_ordinal.is_finite()
                || relative_frequency_shift_by_ordinal < -MASS_MONOTONIC_TOLERANCE
            {
                return Err(format!(
                    "removing rotary inertia lowered ordinal mode {} on the {}-element/member mesh: relative shift={relative_frequency_shift_by_ordinal:.17e}",
                    consistent_index + 1,
                    element_count
                ));
            }
            min_rank_shift = min_rank_shift.min(relative_frequency_shift_by_ordinal);
            max_rank_shift = max_rank_shift.max(relative_frequency_shift_by_ordinal);
            let scores = mode_mac_scores(
                &mesh.assembly,
                baseline,
                paired,
                rotation_scale,
            )?;
            let is_confident =
                scores.scaled_combined_mac >= MIN_COMBINED_MAC_FOR_CONFIDENT_PAIRING;
            if is_confident {
                confident += 1;
            } else {
                ambiguous += 1;
            }
            modes.push(BeamMassSensitivityMode {
                consistent_mode_ordinal: baseline.ordinal,
                no_rotary_shape_paired_mode_ordinal: paired.ordinal,
                consistent_frequency_hz: baseline.frequency_hz,
                no_rotary_frequency_at_same_ordinal_hz: same_rank.frequency_hz,
                no_rotary_frequency_shape_paired_hz: paired.frequency_hz,
                relative_frequency_shift_by_ordinal,
                relative_frequency_shift_by_shape: (paired.frequency_hz
                    - baseline.frequency_hz)
                    / baseline.frequency_hz,
                mac: scores,
                shape_pairing_status: if is_confident {
                    "confident"
                } else {
                    "ambiguous"
                },
            });
        }

        points.push(BeamMassSensitivityPoint {
            elements_per_member: element_count,
            total_element_count: mesh.total_element_count,
            free_dof_count: mesh.assembly.free_dof_indices.len(),
            modes,
            maximum_consistent_eigen_residual: consistent.maximum_normalized_eigen_residual,
            maximum_no_rotary_eigen_residual: no_rotary.maximum_normalized_eigen_residual,
            minimum_ordinal_relative_frequency_shift: min_rank_shift,
            maximum_ordinal_relative_frequency_shift: max_rank_shift,
            confident_shape_pair_count: confident,
            ambiguous_shape_pair_count: ambiguous,
        });
    }

    Ok(BeamMassSensitivityReport {
        elements_per_member: elements_per_member.to_vec(),
        requested_mode_count: requested_modes,
        rotation_characteristic_length_m: rotation_scale,
        minimum_combined_mac_for_confident_pairing: MIN_COMBINED_MAC_FOR_CONFIDENT_PAIRING,
        points,
        consistent_mass_model: "consistent translational mass plus consistent cross-sectional rotary inertia",
        sensitivity_counterfactual: "same stiffness with consistent translational mass only; not a generic commercial lumped-mass model",
        claim_ceiling: "synthetic linear Euler-Bernoulli released-joint beam mass-model sensitivity only; not full NAFEMS 200-element-per-member reproduction, experimental validation, structural safety or code compliance",
    })
}

fn validate_mesh_levels(
    member_count: usize,
    element_counts: &[usize],
    requested_modes: usize,
) -> Check<()> {
    if member_count == 0 || element_counts.is_empty() || element_counts.len() > MAX_MESH_LEVELS {
        return Err(format!(
            "member count must be positive and mesh levels must be in 1..={MAX_MESH_LEVELS}"
        ));
    }
    if requested_modes == 0 || requested_modes > MAX_REQUESTED_MODES {
        return Err(format!(
            "requested modes must be in 1..={MAX_REQUESTED_MODES}"
        ));
    }
    let mut previous = 0usize;
    for &count in element_counts {
        // Reuse the mesh assembler's exact per-member and global dense caps.
        if count <= previous {
            return Err("mesh levels must be strictly increasing and unique".to_owned());
        }
        if count > 32 || member_count.saturating_mul(count) > 64 {
            return Err("mesh level exceeds the dense beam reference assembly cap".to_owned());
        }
        previous = count;
    }
    Ok(())
}

fn characteristic_length(nodes: &[BeamPinNode], members: &[BeamPinMember]) -> Check<f64> {
    let by_id = nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut maximum = 0.0_f64;
    for member in members {
        let start = by_id
            .get(member.start_node_id.as_str())
            .ok_or_else(|| format!("missing start node {}", member.start_node_id))?;
        let end = by_id
            .get(member.end_node_id.as_str())
            .ok_or_else(|| format!("missing end node {}", member.end_node_id))?;
        maximum = maximum.max((end.x_m - start.x_m).hypot(end.y_m - start.y_m));
    }
    if maximum.is_finite() && maximum > 0.0 {
        Ok(maximum)
    } else {
        Err("maximum member length must be finite and positive".to_owned())
    }
}

fn mode_mac_scores(
    assembly: &BeamPinAssembly,
    left: &BeamModalMode,
    right: &BeamModalMode,
    rotation_scale: f64,
) -> Check<MassSensitivityMac> {
    if left.shape_global_dofs.len() != assembly.dof_descriptors.len()
        || right.shape_global_dofs.len() != assembly.dof_descriptors.len()
    {
        return Err("mode vectors do not match the assembly DOF descriptor count".to_owned());
    }
    let mut left_translation = Vec::new();
    let mut right_translation = Vec::new();
    let mut left_rotation = Vec::new();
    let mut right_rotation = Vec::new();
    let mut left_combined = Vec::new();
    let mut right_combined = Vec::new();

    for descriptor in &assembly.dof_descriptors {
        let index = descriptor.index;
        match &descriptor.kind {
            BeamPinDofKind::NodeTranslationX { .. }
            | BeamPinDofKind::NodeTranslationY { .. }
            | BeamPinDofKind::MemberInteriorTranslationX { .. }
            | BeamPinDofKind::MemberInteriorTranslationY { .. } => {
                left_translation.push(left.shape_global_dofs[index]);
                right_translation.push(right.shape_global_dofs[index]);
                left_combined.push(left.shape_global_dofs[index]);
                right_combined.push(right.shape_global_dofs[index]);
            }
            BeamPinDofKind::MemberEndRotation { .. }
            | BeamPinDofKind::MemberInteriorRotation { .. } => {
                left_rotation.push(left.shape_global_dofs[index]);
                right_rotation.push(right.shape_global_dofs[index]);
                left_combined.push(left.shape_global_dofs[index] * rotation_scale);
                right_combined.push(right.shape_global_dofs[index] * rotation_scale);
            }
        }
    }

    Ok(MassSensitivityMac {
        translation_mac: mac(&left_translation, &right_translation),
        rotation_mac: mac(&left_rotation, &right_rotation),
        scaled_combined_mac: mac(&left_combined, &right_combined)
            .ok_or_else(|| "combined mode MAC is undefined for a zero-norm vector".to_owned())?,
    })
}

fn scaled_combined_mac(
    assembly: &BeamPinAssembly,
    left: &BeamModalMode,
    right: &BeamModalMode,
    rotation_scale: f64,
) -> Check<f64> {
    Ok(mode_mac_scores(assembly, left, right, rotation_scale)?.scaled_combined_mac)
}

fn mac(left: &[f64], right: &[f64]) -> Option<f64> {
    if left.is_empty() || left.len() != right.len() {
        return None;
    }
    let dot = left.iter().zip(right).map(|(a, b)| a * b).sum::<f64>();
    let left_norm = left.iter().map(|value| value * value).sum::<f64>();
    let right_norm = right.iter().map(|value| value * value).sum::<f64>();
    let denominator = left_norm * right_norm;
    if !dot.is_finite()
        || !denominator.is_finite()
        || denominator <= 0.0
    {
        return None;
    }
    let value = (dot * dot / denominator).clamp(0.0, 1.0);
    value.is_finite().then_some(value)
}

fn maximum_weight_assignment(weights: &[Vec<f64>]) -> Check<Vec<usize>> {
    let size = weights.len();
    if size == 0
        || weights.iter().any(|row| {
            row.len() != size
                || row
                    .iter()
                    .any(|weight| !weight.is_finite() || !(0.0..=1.0).contains(weight))
        })
    {
        return Err("mode pairing matrix must be square with finite MAC values in [0,1]".to_owned());
    }

    // Hungarian algorithm for a maximum-weight perfect assignment. Conversion
    // to cost preserves one-to-one mode correspondence across the two models.
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
                return Err("mode pairing assignment has no augmenting path".to_owned());
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
            return Err("mode pairing omitted a model mode".to_owned());
        }
        assignment[p[column] - 1] = column - 1;
    }
    if assignment.iter().any(|index| *index == usize::MAX) {
        return Err("mode pairing assignment is incomplete".to_owned());
    }
    Ok(assignment)
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_beam_mass_sensitivity_request, run_beam_mass_formulation_sensitivity,
    };
    use crate::beam_pin_assembly::nafems_nbc05_released_joint_beam_fixture;

    #[test]
    fn rotary_inertia_sensitivity_is_explicit_and_rank_frequency_is_monotone() {
        let (nodes, members) = nafems_nbc05_released_joint_beam_fixture();
        let report = run_beam_mass_formulation_sensitivity(&nodes, &members, &[1, 2, 4, 8], 6)
            .expect("mass model sensitivity report");
        assert_eq!(report.points.len(), 4);
        assert_eq!(
            report.consistent_mass_model,
            "consistent translational mass plus consistent cross-sectional rotary inertia"
        );
        assert!(report.points.iter().all(|point| {
            point.maximum_consistent_eigen_residual < 1.0e-8
                && point.maximum_no_rotary_eigen_residual < 1.0e-8
                && point.modes.len() == 6
                && point
                    .modes
                    .iter()
                    .all(|mode| mode.relative_frequency_shift_by_ordinal >= -1.0e-8)
        }));
        assert!(report.points.iter().any(|point| {
            point.maximum_ordinal_relative_frequency_shift > 1.0e-6
        }));
        assert!(report.points.iter().all(|point| {
            point.confident_shape_pair_count + point.ambiguous_shape_pair_count == 6
        }));
    }

    #[test]
    fn mass_sensitivity_pairing_is_one_to_one_and_mac_is_bounded() {
        let (nodes, members) = nafems_nbc05_released_joint_beam_fixture();
        let report = run_beam_mass_formulation_sensitivity(&nodes, &members, &[2], 6)
            .expect("mass model sensitivity report");
        let paired_ordinals = report.points[0]
            .modes
            .iter()
            .map(|mode| mode.no_rotary_shape_paired_mode_ordinal)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(paired_ordinals.len(), 6);
        assert!(report.points[0].modes.iter().all(|mode| {
            (0.0..=1.0).contains(&mode.mac.scaled_combined_mac)
                && mode.mac.translation_mac.is_none_or(|mac| (0.0..=1.0).contains(&mac))
                && mode.mac.rotation_mac.is_none_or(|mac| (0.0..=1.0).contains(&mac))
        }));
    }

    #[test]
    fn canonical_request_pins_model_variants_and_mesh_ladder() {
        let (nodes, members) = nafems_nbc05_released_joint_beam_fixture();
        let request = canonical_beam_mass_sensitivity_request(&nodes, &members, &[1, 2, 4, 8], 6)
            .expect("canonical mass sensitivity request");
        assert_eq!(
            request,
            include_str!("../fixtures/beam_mass_sensitivity_request_v0_1.txt")
                .trim_end_matches('\n')
        );
    }

    #[test]
    fn invalid_or_ambiguous_mesh_ladders_fail_closed() {
        let (nodes, members) = nafems_nbc05_released_joint_beam_fixture();
        assert!(canonical_beam_mass_sensitivity_request(&nodes, &members, &[], 6).is_err());
        assert!(canonical_beam_mass_sensitivity_request(&nodes, &members, &[1, 4, 2], 6).is_err());
        assert!(canonical_beam_mass_sensitivity_request(&nodes, &members, &[1, 1], 6).is_err());
        assert!(canonical_beam_mass_sensitivity_request(&nodes, &members, &[1, 64], 6).is_err());
        assert!(canonical_beam_mass_sensitivity_request(&nodes, &members, &[1, 2], 0).is_err());
    }
}
