// SPDX-License-Identifier: AGPL-3.0-or-later
//! A small deterministic mesh ladder for the released-joint NBC05 beam model.
//! The first pass checks frequency-ranked convergence and solver diagnostics.
//! Mode-shape MAC / cross-mesh pairing and the source's 200-element/member
//! sparse solve remain explicit future gates.

use crate::beam_mesh::assemble_released_joint_beam_mesh;
use crate::beam_modal::{BeamModalOptions, solve_released_joint_beam_modes};
use crate::beam_pin_assembly::{BeamPinMember, BeamPinNode, canonical_assembly_request};
use crate::Check;

/// A frequency-ranked snapshot for one uniform mesh density.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamMeshConvergencePoint {
    pub elements_per_member: usize,
    pub total_element_count: usize,
    pub interior_node_count: usize,
    pub free_dof_count: usize,
    pub frequencies_hz: Vec<f64>,
    /// Relative changes versus the immediately coarser requested mesh.
    /// This is ordinal-frequency pairing, not mode-shape correspondence.
    pub relative_frequency_delta_from_previous: Option<Vec<f64>>,
    pub maximum_mass_orthogonality_residual: f64,
    pub maximum_normalized_eigen_residual: f64,
    pub maximum_mass_norm_residual: f64,
}

/// Requested mesh ladder and complete result evidence. A passing report means
/// the numerical run was well-formed and the requested low frequencies did
/// not increase under these nested meshes; it does not prove convergence to
/// the continuum or reproduce the NAFEMS beam figure.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamMeshConvergenceReport {
    pub elements_per_member: Vec<usize>,
    pub requested_mode_count: usize,
    pub monotonic_relative_tolerance: f64,
    pub points: Vec<BeamMeshConvergencePoint>,
    pub claim_ceiling: &'static str,
}

/// Canonical request serialization for the mesh-convergence experiment.
pub fn canonical_beam_mesh_convergence_request(
    nodes: &[BeamPinNode],
    members: &[BeamPinMember],
    element_counts: &[usize],
    requested_modes: usize,
    monotonic_relative_tolerance: f64,
) -> Check<String> {
    validate_request(element_counts, requested_modes, monotonic_relative_tolerance)?;
    let base_request = canonical_assembly_request(nodes, members)?;
    Ok(format!(
        "schema=beam-mesh-convergence-request-v0.1|elements_per_member={}|requested_modes={requested_modes}|monotonic_relative_tolerance={monotonic_relative_tolerance}|mode_pairing=frequency_ordinal_only_MAC_not_claimed|base_request={base_request}",
        element_counts
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",")
    ))
}

/// Solve the requested mesh ladder and verify that each frequency-ranked mode
/// is non-increasing under nested refinement up to the declared tolerance.
/// Mode ordinal is the only cross-mesh pairing here; MAC and shape interpolation
/// are deliberately not inferred from these frequency comparisons.
pub fn run_beam_mesh_convergence(
    nodes: &[BeamPinNode],
    members: &[BeamPinMember],
    element_counts: &[usize],
    requested_modes: usize,
    monotonic_relative_tolerance: f64,
) -> Check<BeamMeshConvergenceReport> {
    validate_request(element_counts, requested_modes, monotonic_relative_tolerance)?;
    let mut points = Vec::with_capacity(element_counts.len());
    let mut previous_frequencies: Option<Vec<f64>> = None;

    for elements_per_member in element_counts {
        let mesh = assemble_released_joint_beam_mesh(nodes, members, *elements_per_member)?;
        let spectrum = solve_released_joint_beam_modes(
            &mesh.assembly,
            BeamModalOptions::first_modes(requested_modes),
        )?;
        let frequencies = spectrum
            .modes
            .iter()
            .map(|mode| mode.frequency_hz)
            .collect::<Vec<_>>();
        if frequencies.len() != requested_modes
            || frequencies.iter().any(|frequency| !frequency.is_finite() || *frequency <= 0.0)
            || frequencies.windows(2).any(|pair| pair[1] <= pair[0])
        {
            return Err(format!(
                "mesh with {elements_per_member} elements/member has invalid or unsorted requested frequencies"
            ));
        }

        let relative_delta = if let Some(previous) = &previous_frequencies {
            if previous.len() != frequencies.len() {
                return Err("requested frequency count changed across mesh levels".to_owned());
            }
            let mut deltas = Vec::with_capacity(frequencies.len());
            for (mode, (coarse, fine)) in previous.iter().zip(&frequencies).enumerate() {
                let delta = (fine - coarse).abs() / coarse.abs().max(f64::MIN_POSITIVE);
                if fine > coarse * (1.0 + monotonic_relative_tolerance) {
                    return Err(format!(
                        "frequency-ranked mode {} increased under refinement {} -> {}: coarse={coarse:.17e}, fine={fine:.17e}, relative increase={:.17e}",
                        mode + 1,
                        points.last().map_or(0, |point: &BeamMeshConvergencePoint| point.elements_per_member),
                        elements_per_member,
                        (fine - coarse) / coarse
                    ));
                }
                deltas.push(delta);
            }
            Some(deltas)
        } else {
            None
        };

        points.push(BeamMeshConvergencePoint {
            elements_per_member: *elements_per_member,
            total_element_count: mesh.total_element_count,
            interior_node_count: mesh.interior_node_count,
            free_dof_count: mesh.assembly.free_dof_indices.len(),
            frequencies_hz: frequencies.clone(),
            relative_frequency_delta_from_previous: relative_delta,
            maximum_mass_orthogonality_residual: spectrum.maximum_mass_orthogonality_residual,
            maximum_normalized_eigen_residual: spectrum.maximum_normalized_eigen_residual,
            maximum_mass_norm_residual: spectrum.maximum_mass_norm_residual,
        });
        previous_frequencies = Some(frequencies);
    }

    Ok(BeamMeshConvergenceReport {
        elements_per_member: element_counts.to_vec(),
        requested_mode_count: requested_modes,
        monotonic_relative_tolerance,
        points,
        claim_ceiling: "frequency-ranked 1/2/4/8-element-per-member trend only; no MAC mode pairing, 200-element/member reproduction, experimental validation or structural safety claim",
    })
}

fn validate_request(
    element_counts: &[usize],
    requested_modes: usize,
    monotonic_relative_tolerance: f64,
) -> Check<()> {
    if element_counts.is_empty()
        || element_counts.iter().any(|count| *count == 0 || *count > 32)
        || element_counts.windows(2).any(|pair| pair[1] <= pair[0])
    {
        return Err("mesh ladder must be strictly increasing with each level in 1..=32".to_owned());
    }
    if requested_modes == 0 {
        return Err("requested mode count must be positive".to_owned());
    }
    if !monotonic_relative_tolerance.is_finite()
        || monotonic_relative_tolerance < 0.0
        || monotonic_relative_tolerance > 1.0e-3
    {
        return Err("monotonic tolerance must be finite and in [0, 1e-3]".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_beam_mesh_convergence_request, run_beam_mesh_convergence,
    };
    use crate::beam_pin_assembly::nafems_nbc05_released_joint_beam_fixture;

    const ELEMENT_COUNTS: [usize; 4] = [1, 2, 4, 8];
    const REQUESTED_MODES: usize = 6;
    const MONOTONIC_TOLERANCE: f64 = 1.0e-8;

    #[test]
    fn canonical_mesh_convergence_request_matches_frozen_fixture() {
        let (nodes, members) = nafems_nbc05_released_joint_beam_fixture();
        let request = canonical_beam_mesh_convergence_request(
            &nodes,
            &members,
            &ELEMENT_COUNTS,
            REQUESTED_MODES,
            MONOTONIC_TOLERANCE,
        )
        .expect("canonical mesh convergence request");
        assert_eq!(
            request,
            include_str!("../fixtures/beam_mesh_convergence_request_v0_1.txt")
                .trim_end_matches('\\n')
        );
    }

    #[test]
    fn four_level_mesh_ladder_reports_sorted_modes_residuals_and_refinement_deltas() {
        let (nodes, members) = nafems_nbc05_released_joint_beam_fixture();
        let report = run_beam_mesh_convergence(
            &nodes,
            &members,
            &ELEMENT_COUNTS,
            REQUESTED_MODES,
            MONOTONIC_TOLERANCE,
        )
        .expect("mesh convergence ladder");

        assert_eq!(report.points.len(), 4);
        assert_eq!(report.points.iter().map(|point| point.total_element_count).collect::<Vec<_>>(), vec![2, 4, 8, 16]);
        assert_eq!(report.points.iter().map(|point| point.free_dof_count).collect::<Vec<_>>(), vec![6, 12, 24, 48]);
        assert!(report.points[0].relative_frequency_delta_from_previous.is_none());
        assert!(report.points[1..].iter().all(|point| point.relative_frequency_delta_from_previous.as_ref().is_some_and(|deltas| deltas.len() == REQUESTED_MODES)));
        for point in &report.points[1..] {
            for delta in point.relative_frequency_delta_from_previous.as_ref().expect("refinement deltas") {
                assert!(delta.is_finite() && (0.0..1.0).contains(delta));
            }
        }
        for point in &report.points {
            assert_eq!(point.frequencies_hz.len(), REQUESTED_MODES);
            assert!(point.maximum_normalized_eigen_residual < 1.0e-8);
            assert!(point.maximum_mass_norm_residual < 1.0e-8);
            assert!(point.maximum_mass_orthogonality_residual < 1.0e-8);
            assert!(point.frequencies_hz.windows(2).all(|pair| pair[0] < pair[1]));
        }
    }

    #[test]
    fn request_digest_changes_when_mesh_ladder_or_mode_count_changes() {
        let (nodes, members) = nafems_nbc05_released_joint_beam_fixture();
        let four_levels = canonical_beam_mesh_convergence_request(
            &nodes,
            &members,
            &ELEMENT_COUNTS,
            REQUESTED_MODES,
            MONOTONIC_TOLERANCE,
        )
        .expect("canonical request");
        let three_levels = canonical_beam_mesh_convergence_request(
            &nodes,
            &members,
            &[1, 2, 4],
            REQUESTED_MODES,
            MONOTONIC_TOLERANCE,
        )
        .expect("shorter request");
        let five_modes = canonical_beam_mesh_convergence_request(
            &nodes,
            &members,
            &ELEMENT_COUNTS,
            5,
            MONOTONIC_TOLERANCE,
        )
        .expect("five mode request");
        assert_ne!(four_levels, three_levels);
        assert_ne!(four_levels, five_modes);
    }

    #[test]
    fn invalid_mesh_ladders_and_mode_counts_fail_closed() {
        let (nodes, members) = nafems_nbc05_released_joint_beam_fixture();
        assert!(run_beam_mesh_convergence(&nodes, &members, &[], 1, MONOTONIC_TOLERANCE).is_err());
        assert!(run_beam_mesh_convergence(&nodes, &members, &[1, 4, 2], 1, MONOTONIC_TOLERANCE).is_err());
        assert!(run_beam_mesh_convergence(&nodes, &members, &[1, 2], 0, MONOTONIC_TOLERANCE).is_err());
        assert!(run_beam_mesh_convergence(&nodes, &members, &[1, 2], 1, f64::NAN).is_err());
        assert!(run_beam_mesh_convergence(&nodes, &members, &[1, 33], 1, MONOTONIC_TOLERANCE).is_err());
    }
}
