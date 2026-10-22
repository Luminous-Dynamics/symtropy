// SPDX-License-Identifier: AGPL-3.0-or-later
//! Local modal sensitivity and uncertainty propagation for released-joint
//! Euler-Bernoulli beam chains.
//!
//! This is a deterministic finite-difference V&V aid, not a probability model.
//! It reports marginal first-order frequency-uncertainty contributions for
//! declared input uncertainty and abstains when mode identity is ambiguous,
//! clustered, non-elastic or insufficiently correlated. Joint-release, beam
//! theory and mass-formulation differences remain categorical model-form
//! variants; this module does not turn them into scalar error bars.

use crate::Check;
use crate::beam_modal::{
    BeamModeClass, BeamModalMode, modal_assurance_criterion, sample_beam_chain_mode_on_fixed_grid,
    solve_beam_pin_assembly_modes,
};
use crate::beam_pin_assembly::{
    BeamMemberChain, BeamPinAssembly, BeamPinNode, assemble_released_joint_beam_chains,
    canonical_beam_chain_request,
};
use std::collections::BTreeSet;

const CLUSTER_RELATIVE_FREQUENCY_GAP: f64 = 1.0e-3;
const MAXIMUM_MATCH_FREQUENCY_GAP: f64 = 0.25;
const MINIMUM_MODE_MAC: f64 = 0.70;
const MINIMUM_MAC_SEPARATION: f64 = 0.05;

/// Parameter changed by a controlled finite-difference pair.
///
/// SectionRadius scales both area and second moment as a circular solid section
/// (A proportional to r² and I proportional to r⁴). It should only be used
/// where the declared section interpretation really is circular.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum BeamUncertaintyParameter {
    YoungsModulus,
    Density,
    Area,
    SecondMoment,
    SectionRadius,
    GeometryScale,
}

impl BeamUncertaintyParameter {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::YoungsModulus => "youngs_modulus",
            Self::Density => "density",
            Self::Area => "area",
            Self::SecondMoment => "second_moment",
            Self::SectionRadius => "circular_section_radius",
            Self::GeometryScale => "geometry_scale",
        }
    }
}

/// Canonical request bytes for one deterministic modal-sensitivity evaluation.
/// The base beam-chain request is carried after an explicit UTF-8 byte length;
/// this keeps its embedded separators unambiguous and binds section, geometry,
/// supports, and mesh inputs together with the uncertainty assumptions.
pub fn canonical_modal_uncertainty_request(
    joints: &[BeamPinNode],
    members: &[BeamMemberChain],
    parameters: &[BeamInputUncertainty],
    sampling_points_per_member: usize,
) -> Check<String> {
    validate_uncertainty_parameters(parameters, sampling_points_per_member)?;
    let base_request = canonical_beam_chain_request(joints, members)?;
    let mut ordered_parameters = parameters.to_vec();
    ordered_parameters.sort_by_key(|spec| spec.parameter);
    let mut fields = vec![
        "schema=beam-modal-uncertainty-request-v0.1".to_owned(),
        "fixture=PHYS-BEAM-MODAL-UQ-001".to_owned(),
        format!("sampling_points_per_member={sampling_points_per_member}"),
        format!("base_request_utf8_bytes={}", base_request.len()),
        format!("base_request={base_request}"),
    ];
    for spec in ordered_parameters {
        fields.push(format!(
            "parameter|{}|relative_uncertainty={}|finite_difference_step={}",
            spec.parameter.as_str(),
            spec.assumed_relative_uncertainty,
            spec.finite_difference_step_relative
        ));
    }
    Ok(fields.join("|"))
}

/// A user-declared relative input uncertainty and a smaller numerical
/// finite-difference step. The uncertainty is a marginal relative standard
/// uncertainty supplied by the analyst; it is not inferred from solver error.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeamInputUncertainty {
    pub parameter: BeamUncertaintyParameter,
    pub assumed_relative_uncertainty: f64,
    pub finite_difference_step_relative: f64,
}

/// Why a mode's sensitivity result is, or is not, qualified.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SensitivityDisposition {
    Qualified,
    BaselineModeNotElastic,
    BaselineModeClustered,
    PerturbedModeClustered,
    ModeAssociationAmbiguous,
    NoQualifyingModeMatch,
}

/// Marginal sensitivity result for one reference mode and one parameter.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamModeSensitivity {
    /// One-based rank in the baseline generalized eigenspectrum.
    pub baseline_mode_index: usize,
    pub parameter: BeamUncertaintyParameter,
    pub assumed_relative_uncertainty: f64,
    pub baseline_frequency_hz: Option<f64>,
    pub plus_frequency_hz: Option<f64>,
    pub minus_frequency_hz: Option<f64>,
    /// One-based mode rank in each perturbed spectrum after shape matching.
    pub plus_matched_mode_index: Option<usize>,
    pub minus_matched_mode_index: Option<usize>,
    pub plus_mac: Option<f64>,
    pub minus_mac: Option<f64>,
    /// Central logarithmic derivative d(ln f)/d(ln p).
    pub logarithmic_frequency_sensitivity: Option<f64>,
    /// |d(ln f)/d(ln p)| times the declared relative input uncertainty.
    pub estimated_relative_frequency_uncertainty: Option<f64>,
    pub disposition: SensitivityDisposition,
}

/// Entire deterministic sensitivity report for one frozen physical model.
/// Marginal contributions must not be summed without an explicit covariance
/// model; discrete model-form alternatives must be reported separately.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamModalUncertaintyReport {
    pub baseline_mode_count: usize,
    pub sampling_points_per_member: usize,
    pub parameters: Vec<BeamInputUncertainty>,
    pub sensitivity_results: Vec<BeamModeSensitivity>,
    pub qualified_sensitivity_count: usize,
    pub abstained_sensitivity_count: usize,
    pub claim_ceiling: &'static str,
}

/// Evaluate deterministic plus/minus perturbations for each requested input.
///
/// The baseline and perturbed spectra retain all solver-classified modes. An
/// elastic mode is tracked using its displacement field sampled on a fixed
/// normalized-arclength grid, a MAC gate and a frequency-gap gate. Repeated or
/// near-repeated modes are not force-matched as isolated eigenvectors.
pub fn evaluate_beam_modal_uncertainty(
    joints: &[BeamPinNode],
    members: &[BeamMemberChain],
    parameters: &[BeamInputUncertainty],
    sampling_points_per_member: usize,
) -> Check<BeamModalUncertaintyReport> {
    validate_uncertainty_parameters(parameters, sampling_points_per_member)?;

    // Stable ordering means the report bytes don't depend on the caller's list.
    let mut ordered_specs = parameters.to_vec();
    ordered_specs.sort_by_key(|spec| spec.parameter);

    let baseline_assembly = assemble_released_joint_beam_chains(joints, members)?;
    let baseline_modal = solve_beam_pin_assembly_modes(&baseline_assembly)?;
    let baseline_cluster_mask = clustered_mode_mask(&baseline_modal.modes);
    let baseline_shapes = sample_elastic_mode_shapes(
        joints,
        members,
        &baseline_assembly,
        &baseline_modal.modes,
        sampling_points_per_member,
    )?;

    let mut sensitivity_results = Vec::with_capacity(
        baseline_modal.modes.len().saturating_mul(ordered_specs.len()),
    );
    for spec in &ordered_specs {
        let step = spec.finite_difference_step_relative;
        let (plus_joints, plus_members) =
            perturb_model(joints, members, spec.parameter, 1.0 + step)?;
        let (minus_joints, minus_members) =
            perturb_model(joints, members, spec.parameter, 1.0 - step)?;
        let plus_assembly = assemble_released_joint_beam_chains(&plus_joints, &plus_members)?;
        let minus_assembly =
            assemble_released_joint_beam_chains(&minus_joints, &minus_members)?;
        let plus_modal = solve_beam_pin_assembly_modes(&plus_assembly)?;
        let minus_modal = solve_beam_pin_assembly_modes(&minus_assembly)?;
        if plus_modal.modes.len() != baseline_modal.modes.len()
            || minus_modal.modes.len() != baseline_modal.modes.len()
        {
            return Err("perturbation changed the free-DOF count".to_owned());
        }
        let plus_cluster_mask = clustered_mode_mask(&plus_modal.modes);
        let minus_cluster_mask = clustered_mode_mask(&minus_modal.modes);

        let plus_shapes = sample_elastic_mode_shapes(
            &plus_joints,
            &plus_members,
            &plus_assembly,
            &plus_modal.modes,
            sampling_points_per_member,
        )?;
        let minus_shapes = sample_elastic_mode_shapes(
            &minus_joints,
            &minus_members,
            &minus_assembly,
            &minus_modal.modes,
            sampling_points_per_member,
        )?;

        for (zero_based_index, baseline_mode) in baseline_modal.modes.iter().enumerate() {
            let baseline_frequency = baseline_mode.frequency_hz;
            if baseline_mode.class != BeamModeClass::Elastic || baseline_frequency.is_none() {
                sensitivity_results.push(empty_sensitivity(
                    zero_based_index,
                    *spec,
                    baseline_frequency,
                    SensitivityDisposition::BaselineModeNotElastic,
                ));
                continue;
            }
            if baseline_cluster_mask[zero_based_index] {
                sensitivity_results.push(empty_sensitivity(
                    zero_based_index,
                    *spec,
                    baseline_frequency,
                    SensitivityDisposition::BaselineModeClustered,
                ));
                continue;
            }
            let baseline_shape = baseline_shapes[zero_based_index]
                .as_ref()
                .ok_or_else(|| "elastic baseline mode is missing a sampled shape".to_owned())?;
            let baseline_frequency = baseline_frequency.expect("elastic frequency validated");
            let plus_match = match_perturbed_mode(
                baseline_shape,
                baseline_frequency,
                &plus_modal.modes,
                &plus_shapes,
                &plus_cluster_mask,
            )?;
            let minus_match = match_perturbed_mode(
                baseline_shape,
                baseline_frequency,
                &minus_modal.modes,
                &minus_shapes,
                &minus_cluster_mask,
            )?;

            let (plus_match, minus_match) = match (plus_match, minus_match) {
                (ModeMatchOutcome::Matched(plus), ModeMatchOutcome::Matched(minus)) => {
                    (plus, minus)
                }
                (ModeMatchOutcome::Clustered, _) | (_, ModeMatchOutcome::Clustered) => {
                    sensitivity_results.push(empty_sensitivity(
                        zero_based_index,
                        *spec,
                        Some(baseline_frequency),
                        SensitivityDisposition::PerturbedModeClustered,
                    ));
                    continue;
                }
                (ModeMatchOutcome::Ambiguous, _) | (_, ModeMatchOutcome::Ambiguous) => {
                    sensitivity_results.push(empty_sensitivity(
                        zero_based_index,
                        *spec,
                        Some(baseline_frequency),
                        SensitivityDisposition::ModeAssociationAmbiguous,
                    ));
                    continue;
                }
                _ => {
                    sensitivity_results.push(empty_sensitivity(
                        zero_based_index,
                        *spec,
                        Some(baseline_frequency),
                        SensitivityDisposition::NoQualifyingModeMatch,
                    ));
                    continue;
                }
            };

            let denominator = ((1.0 + step) / (1.0 - step)).ln();
            let logarithmic_sensitivity =
                (plus_match.frequency_hz / minus_match.frequency_hz).ln() / denominator;
            if !logarithmic_sensitivity.is_finite() {
                return Err("logarithmic modal sensitivity is non-finite".to_owned());
            }
            sensitivity_results.push(BeamModeSensitivity {
                baseline_mode_index: zero_based_index + 1,
                parameter: spec.parameter,
                assumed_relative_uncertainty: spec.assumed_relative_uncertainty,
                baseline_frequency_hz: Some(baseline_frequency),
                plus_frequency_hz: Some(plus_match.frequency_hz),
                minus_frequency_hz: Some(minus_match.frequency_hz),
                plus_matched_mode_index: Some(plus_match.zero_based_index + 1),
                minus_matched_mode_index: Some(minus_match.zero_based_index + 1),
                plus_mac: Some(plus_match.mac),
                minus_mac: Some(minus_match.mac),
                logarithmic_frequency_sensitivity: Some(logarithmic_sensitivity),
                estimated_relative_frequency_uncertainty: Some(
                    logarithmic_sensitivity.abs() * spec.assumed_relative_uncertainty,
                ),
                disposition: SensitivityDisposition::Qualified,
            });
        }
    }

    let qualified_count = sensitivity_results
        .iter()
        .filter(|result| result.disposition == SensitivityDisposition::Qualified)
        .count();
    let abstained_count = sensitivity_results.len() - qualified_count;
    Ok(BeamModalUncertaintyReport {
        baseline_mode_count: baseline_modal.modes.len(),
        sampling_points_per_member,
        parameters: ordered_specs,
        sensitivity_results,
        qualified_sensitivity_count: qualified_count,
        abstained_sensitivity_count: abstained_count,
        claim_ceiling: "deterministic local sensitivity propagation only; not a probabilistic confidence interval, experimental validation, model-form uncertainty bound or structural safety result",
    })
}

fn validate_uncertainty_parameters(
    parameters: &[BeamInputUncertainty],
    sampling_points_per_member: usize,
) -> Check<()> {
    if parameters.is_empty() {
        return Err("at least one uncertainty parameter is required".to_owned());
    }
    if !(2..=257).contains(&sampling_points_per_member) {
        return Err("sampling points per member must be between 2 and 257".to_owned());
    }

    let mut seen_parameters = BTreeSet::new();
    for spec in parameters {
        if !seen_parameters.insert(spec.parameter) {
            return Err(format!(
                "duplicate uncertainty parameter: {}",
                spec.parameter.as_str()
            ));
        }
        if !spec.assumed_relative_uncertainty.is_finite()
            || spec.assumed_relative_uncertainty < 0.0
            || spec.assumed_relative_uncertainty >= 1.0
        {
            return Err(format!(
                "relative uncertainty for {} must be finite and in [0, 1)",
                spec.parameter.as_str()
            ));
        }
        if !spec.finite_difference_step_relative.is_finite()
            || spec.finite_difference_step_relative <= 0.0
            || spec.finite_difference_step_relative >= 0.25
        {
            return Err(format!(
                "finite-difference step for {} must be finite and in (0, 0.25)",
                spec.parameter.as_str()
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
struct ModeMatch {
    zero_based_index: usize,
    frequency_hz: f64,
    mac: f64,
}

#[derive(Clone, Copy, Debug)]
enum ModeMatchOutcome {
    Matched(ModeMatch),
    Clustered,
    Ambiguous,
    Missing,
}

fn match_perturbed_mode(
    baseline_shape: &[f64],
    baseline_frequency_hz: f64,
    candidates: &[BeamModalMode],
    candidate_shapes: &[Option<Vec<f64>>],
    cluster_mask: &[bool],
) -> Check<ModeMatchOutcome> {
    if candidates.len() != candidate_shapes.len() || candidates.len() != cluster_mask.len() {
        return Err("perturbed mode diagnostics have inconsistent lengths".to_owned());
    }
    let mut matches = Vec::new();
    for (zero_based_index, candidate) in candidates.iter().enumerate() {
        if candidate.class != BeamModeClass::Elastic {
            continue;
        }
        let Some(frequency_hz) = candidate.frequency_hz else {
            continue;
        };
        let relative_frequency_gap =
            (frequency_hz - baseline_frequency_hz).abs() / baseline_frequency_hz;
        if relative_frequency_gap > MAXIMUM_MATCH_FREQUENCY_GAP {
            continue;
        }
        let Some(shape) = candidate_shapes[zero_based_index].as_ref() else {
            continue;
        };
        let mac = modal_assurance_criterion(baseline_shape, shape)?;
        if mac >= MINIMUM_MODE_MAC {
            matches.push(ModeMatch {
                zero_based_index,
                frequency_hz,
                mac,
            });
        }
    }
    matches.sort_by(|left, right| right.mac.total_cmp(&left.mac));
    let Some(best) = matches.first().copied() else {
        return Ok(ModeMatchOutcome::Missing);
    };
    if matches
        .get(1)
        .is_some_and(|second| best.mac - second.mac < MINIMUM_MAC_SEPARATION)
    {
        return Ok(ModeMatchOutcome::Ambiguous);
    }
    if cluster_mask[best.zero_based_index] {
        return Ok(ModeMatchOutcome::Clustered);
    }
    Ok(ModeMatchOutcome::Matched(best))
}

fn clustered_mode_mask(modes: &[BeamModalMode]) -> Vec<bool> {
    let elastic_indices = modes
        .iter()
        .enumerate()
        .filter_map(|(index, mode)| {
            (mode.class == BeamModeClass::Elastic)
                .then_some(mode.frequency_hz.map(|frequency| (index, frequency)))
                .flatten()
        })
        .collect::<Vec<_>>();
    let mut clustered = vec![false; modes.len()];
    for pair in elastic_indices.windows(2) {
        let lower_gap =
            (pair[1].1 - pair[0].1).abs() / pair[0].1.abs().max(pair[1].1.abs());
        if lower_gap <= CLUSTER_RELATIVE_FREQUENCY_GAP {
            clustered[pair[0].0] = true;
            clustered[pair[1].0] = true;
        }
    }
    clustered
}

fn sample_elastic_mode_shapes(
    joints: &[BeamPinNode],
    members: &[BeamMemberChain],
    assembly: &BeamPinAssembly,
    modes: &[BeamModalMode],
    sampling_points: usize,
) -> Check<Vec<Option<Vec<f64>>>> {
    modes
        .iter()
        .map(|mode| {
            if mode.class != BeamModeClass::Elastic || mode.frequency_hz.is_none() {
                Ok(None)
            } else {
                sample_beam_chain_mode_on_fixed_grid(
                    joints,
                    members,
                    assembly,
                    mode,
                    sampling_points,
                )
                .map(Some)
            }
        })
        .collect()
}

fn perturb_model(
    joints: &[BeamPinNode],
    members: &[BeamMemberChain],
    parameter: BeamUncertaintyParameter,
    factor: f64,
) -> Check<(Vec<BeamPinNode>, Vec<BeamMemberChain>)> {
    if !factor.is_finite() || factor <= 0.0 {
        return Err("perturbation factor must be finite and positive".to_owned());
    }
    let mut perturbed_joints = joints.to_vec();
    let mut perturbed_members = members.to_vec();
    match parameter {
        BeamUncertaintyParameter::GeometryScale => {
            for joint in &mut perturbed_joints {
                joint.x_m *= factor;
                joint.y_m *= factor;
            }
        }
        BeamUncertaintyParameter::YoungsModulus => {
            for member in &mut perturbed_members {
                member.youngs_modulus_pa *= factor;
            }
        }
        BeamUncertaintyParameter::Density => {
            for member in &mut perturbed_members {
                member.density_kg_m3 *= factor;
            }
        }
        BeamUncertaintyParameter::Area => {
            for member in &mut perturbed_members {
                member.area_m2 *= factor;
            }
        }
        BeamUncertaintyParameter::SecondMoment => {
            for member in &mut perturbed_members {
                member.second_moment_m4 *= factor;
            }
        }
        BeamUncertaintyParameter::SectionRadius => {
            for member in &mut perturbed_members {
                member.area_m2 *= factor.powi(2);
                member.second_moment_m4 *= factor.powi(4);
            }
        }
    }
    Ok((perturbed_joints, perturbed_members))
}

fn empty_sensitivity(
    zero_based_mode_index: usize,
    spec: BeamInputUncertainty,
    baseline_frequency_hz: Option<f64>,
    disposition: SensitivityDisposition,
) -> BeamModeSensitivity {
    BeamModeSensitivity {
        baseline_mode_index: zero_based_mode_index + 1,
        parameter: spec.parameter,
        assumed_relative_uncertainty: spec.assumed_relative_uncertainty,
        baseline_frequency_hz,
        plus_frequency_hz: None,
        minus_frequency_hz: None,
        plus_matched_mode_index: None,
        minus_matched_mode_index: None,
        plus_mac: None,
        minus_mac: None,
        logarithmic_frequency_sensitivity: None,
        estimated_relative_frequency_uncertainty: None,
        disposition,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BeamInputUncertainty, BeamUncertaintyParameter, SensitivityDisposition,
        canonical_modal_uncertainty_request, evaluate_beam_modal_uncertainty,
    };
    use crate::beam_pin_assembly::{BeamMemberChain, BeamPinNode};

    fn joints() -> Vec<BeamPinNode> {
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

    fn members(elements_per_member: usize) -> Vec<BeamMemberChain> {
        let radius_m = 0.01_f64;
        let area_m2 = std::f64::consts::PI * radius_m.powi(2);
        let second_moment_m4 = std::f64::consts::PI * radius_m.powi(4) / 4.0;
        vec![
            BeamMemberChain {
                id: "AC".to_owned(),
                start_node_id: "A".to_owned(),
                end_node_id: "C".to_owned(),
                area_m2,
                youngs_modulus_pa: 210.0e9,
                density_kg_m3: 7800.0,
                second_moment_m4,
                elements_per_member,
            },
            BeamMemberChain {
                id: "BC".to_owned(),
                start_node_id: "B".to_owned(),
                end_node_id: "C".to_owned(),
                area_m2,
                youngs_modulus_pa: 210.0e9,
                density_kg_m3: 7800.0,
                second_moment_m4,
                elements_per_member,
            },
        ]
    }

    fn spec(
        parameter: BeamUncertaintyParameter,
        uncertainty: f64,
    ) -> BeamInputUncertainty {
        BeamInputUncertainty {
            parameter,
            assumed_relative_uncertainty: uncertainty,
            finite_difference_step_relative: 1.0e-3,
        }
    }

    #[test]
    fn elastic_mode_sensitivity_matches_exact_material_scaling_laws() {
        let report = evaluate_beam_modal_uncertainty(
            &joints(),
            &members(1),
            &[
                spec(BeamUncertaintyParameter::YoungsModulus, 0.05),
                spec(BeamUncertaintyParameter::Density, 0.03),
            ],
            17,
        )
        .expect("valid sensitivity report");
        let elastic_count = report
            .sensitivity_results
            .iter()
            .filter(|entry| entry.disposition == SensitivityDisposition::Qualified)
            .map(|entry| entry.baseline_mode_index)
            .max()
            .unwrap_or(0);
        assert!(elastic_count > 0);
        for entry in report
            .sensitivity_results
            .iter()
            .filter(|entry| entry.disposition == SensitivityDisposition::Qualified)
        {
            match entry.parameter {
                BeamUncertaintyParameter::YoungsModulus => {
                    assert!(
                        (entry.logarithmic_frequency_sensitivity.unwrap() - 0.5).abs() < 1.0e-5
                    );
                    assert!(
                        (entry.estimated_relative_frequency_uncertainty.unwrap() - 0.025).abs()
                            < 1.0e-6
                    );
                }
                BeamUncertaintyParameter::Density => {
                    assert!(
                        (entry.logarithmic_frequency_sensitivity.unwrap() + 0.5).abs() < 1.0e-5
                    );
                    assert!(
                        (entry.estimated_relative_frequency_uncertainty.unwrap() - 0.015).abs()
                            < 1.0e-6
                    );
                }
                _ => panic!("unexpected parameter in exact scaling-law test"),
            }
        }
    }

    #[test]
    fn canonical_request_binds_mesh_uncertainty_and_is_storage_order_invariant() {
        let parameters = [
            spec(BeamUncertaintyParameter::Density, 0.03),
            spec(BeamUncertaintyParameter::YoungsModulus, 0.05),
        ];
        let first = canonical_modal_uncertainty_request(&joints(), &members(1), &parameters, 17)
            .expect("canonical request");
        assert_eq!(
            first,
            include_str!("../fixtures/modal_uncertainty_request_v0_1.txt")
                .trim_end_matches('\n')
        );
        let mut reversed_joints = joints();
        reversed_joints.reverse();
        let mut reversed_members = members(1);
        reversed_members.reverse();
        let reversed_parameters = [parameters[1], parameters[0]];
        let second = canonical_modal_uncertainty_request(
            &reversed_joints,
            &reversed_members,
            &reversed_parameters,
            17,
        )
        .expect("canonical reversed request");
        assert_eq!(first, second);
    }

    #[test]
    fn sensitivity_report_is_invariant_to_input_storage_order() {
        let params = [
            spec(BeamUncertaintyParameter::SecondMoment, 0.05),
            spec(BeamUncertaintyParameter::YoungsModulus, 0.04),
            spec(BeamUncertaintyParameter::Density, 0.03),
        ];
        let first =
            evaluate_beam_modal_uncertainty(&joints(), &members(1), &params, 17).expect("first");
        let mut reverse_joints = joints();
        reverse_joints.reverse();
        let mut reverse_members = members(1);
        reverse_members.reverse();
        let reverse_params = [params[2], params[1], params[0]];
        let second = evaluate_beam_modal_uncertainty(
            &reverse_joints,
            &reverse_members,
            &reverse_params,
            17,
        )
        .expect("reordered");
        assert_eq!(first, second);
    }

    fn synthetic_mode(frequency_hz: f64, shape: Vec<f64>) -> crate::beam_modal::BeamModalMode {
        crate::beam_modal::BeamModalMode {
            eigenvalue_rad_s_squared: (2.0 * std::f64::consts::PI * frequency_hz).powi(2),
            frequency_hz: Some(frequency_hz),
            class: crate::beam_modal::BeamModeClass::Elastic,
            shape_global_dofs: shape,
            normalized_eigen_residual: 0.0,
            mass_norm_residual: 0.0,
        }
    }

    #[test]
    fn clustered_eigenmodes_abstain_from_individual_mode_sensitivity() {
        let baseline_shape = vec![1.0, 0.0];
        let candidates = vec![
            synthetic_mode(100.0, vec![1.0, 0.0]),
            synthetic_mode(100.05, vec![0.0, 1.0]),
        ];
        let cluster_mask = clustered_mode_mask(&candidates);
        assert_eq!(cluster_mask, vec![true, true]);
        let outcome = match_perturbed_mode(
            &baseline_shape,
            100.0,
            &candidates,
            &[Some(vec![1.0, 0.0]), Some(vec![0.0, 1.0])],
            &cluster_mask,
        )
        .expect("clustered-mode classification");
        assert!(matches!(outcome, ModeMatchOutcome::Clustered));
    }

    #[test]
    fn equal_mac_candidates_abstain_instead_of_falling_back_to_frequency_rank() {
        let baseline_shape = vec![1.0, 0.0];
        let candidates = vec![
            synthetic_mode(99.0, vec![1.0, 0.0]),
            synthetic_mode(101.0, vec![-1.0, 0.0]),
        ];
        let cluster_mask = clustered_mode_mask(&candidates);
        assert_eq!(cluster_mask, vec![false, false]);
        let outcome = match_perturbed_mode(
            &baseline_shape,
            100.0,
            &candidates,
            &[Some(vec![1.0, 0.0]), Some(vec![-1.0, 0.0])],
            &cluster_mask,
        )
        .expect("ambiguous-mode classification");
        assert!(matches!(outcome, ModeMatchOutcome::Ambiguous));
    }

    #[test]
    fn invalid_uncertainty_specs_fail_closed() {
        assert!(
            evaluate_beam_modal_uncertainty(
                &joints(),
                &members(1),
                &[spec(BeamUncertaintyParameter::YoungsModulus, 1.0)],
                17,
            )
            .is_err()
        );
        assert!(
            evaluate_beam_modal_uncertainty(
                &joints(),
                &members(1),
                &[
                    spec(BeamUncertaintyParameter::Density, 0.05),
                    spec(BeamUncertaintyParameter::Density, 0.10),
                ],
                17,
            )
            .is_err()
        );
    }
}

