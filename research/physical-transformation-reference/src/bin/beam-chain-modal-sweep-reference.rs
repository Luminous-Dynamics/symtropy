// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic modal mesh-refinement receipt for a two-member released-joint
//! Euler-Bernoulli beam assembly. These meshes are synthetic verification cases,
//! not the 200-elements-per-member NAFEMS reference model.

use physical_transformation_reference::beam_modal::{
    BeamModeClass, modal_assurance_criterion, sample_beam_chain_mode_on_fixed_grid,
    solve_beam_pin_assembly_modes,
};
use physical_transformation_reference::beam_pin_assembly::{
    BeamMemberChain, BeamPinNode, assemble_released_joint_beam_chains,
    canonical_beam_chain_sweep_request,
};
use physical_transformation_reference::modal_subspace::{
    cluster_mode_frequencies_hz, mass_weighted_modal_subspace_comparison,
};
use std::env;
use std::process::ExitCode;

const FROZEN_REQUEST: &str =
    include_str!("../../fixtures/beam_chain_subspace_sweep_request_v0_1.txt");
const ELEMENT_COUNTS: [usize; 4] = [1, 2, 4, 8];
const SAMPLE_POINTS_PER_MEMBER: usize = 17;
const FREQUENCY_CLUSTER_RELATIVE_GAP: f64 = 1.0e-3;

fn main() -> ExitCode {
    match run() {
        Ok(receipt) => {
            println!("{receipt}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("beam-chain-modal-sweep-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let source_commit = parse_source_commit()?;
    let nodes = fixture_nodes();
    let templates = fixture_members(1);
    let base_request = canonical_beam_chain_sweep_request(&nodes, &templates, &ELEMENT_COUNTS)?;
    let request = format!(
        "{base_request}|sample_points_per_member={SAMPLE_POINTS_PER_MEMBER}|frequency_cluster_relative_gap={FREQUENCY_CLUSTER_RELATIVE_GAP:.3}|shape_metric=mass-weighted-principal-angles|cluster_policy=same-rank-compare-only-when-partition-matches"
    );
    if request != FROZEN_REQUEST.trim_end_matches('\n') {
        return Err("canonical mesh-sweep/subspace request differs from frozen fixture".to_owned());
    }
    let sampled_mass = sampled_shape_mass_matrix(&nodes, &templates, SAMPLE_POINTS_PER_MEMBER)?;

    let mut mesh_receipts = Vec::with_capacity(ELEMENT_COUNTS.len());
    let mut previous_frequencies: Option<Vec<f64>> = None;
    let mut previous_sampled_modes: Option<Vec<Vec<f64>>> = None;
    for elements_per_member in ELEMENT_COUNTS {
        let members = fixture_members(elements_per_member);
        let assembly = assemble_released_joint_beam_chains(&nodes, &members)?;
        let result = solve_beam_pin_assembly_modes(&assembly)?;
        if result.modes.len() != assembly.free_dof_indices.len()
            || result.modes.len() != 6 * elements_per_member
        {
            return Err(format!(
                "mode count differs from expected free-DOF count at mesh {elements_per_member}"
            ));
        }
        if result.modes.iter().any(|mode| {
            mode.class != BeamModeClass::Elastic
                || mode.frequency_hz.is_none()
                || !mode.normalized_eigen_residual.is_finite()
                || !mode.mass_norm_residual.is_finite()
        }) {
            return Err(format!(
                "mesh {elements_per_member} contains a non-elastic or non-finite modal result"
            ));
        }
        if result.maximum_normalized_eigen_residual >= 1.0e-9
            || result.maximum_mass_norm_residual >= 1.0e-9
            || result.maximum_off_diagonal_mass_orthogonality_residual >= 1.0e-9
        {
            return Err(format!(
                "modal diagnostic gate failed at mesh {elements_per_member}"
            ));
        }
        let current_frequencies = result
            .modes
            .iter()
            .map(|mode| mode.frequency_hz.expect("elastic mode"))
            .collect::<Vec<_>>();
        // Compare the six lowest modes on fixed physical sample locations,
        // not incompatible nodal vectors from different meshes. Preserve the
        // full 6x6 MAC matrix; frequency-rank pairing is not silently blessed.
        let current_sampled_modes = result
            .modes
            .iter()
            .take(6)
            .map(|mode| sample_beam_chain_mode_on_fixed_grid(
                    &nodes,
                    &members,
                    &assembly,
                    mode,
                    SAMPLE_POINTS_PER_MEMBER,
                ))
            .collect::<Result<Vec<_>, _>>()?;
        if current_sampled_modes.len() != 6 {
            return Err("each mesh must provide six modes for the low-mode MAC grid".to_owned());
        }
        let mac_matrix = if let Some(previous_modes) = &previous_sampled_modes {
            Some(
                previous_modes
                    .iter()
                    .map(|previous_mode| {
                        current_sampled_modes
                            .iter()
                            .map(|current_mode| {
                                modal_assurance_criterion(previous_mode, current_mode)
                            })
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            )
        } else {
            None
        };
        let cluster_comparison = match (&previous_frequencies, &previous_sampled_modes) {
            (Some(previous_frequency_values), Some(previous_modes)) => Some(
                cluster_subspace_comparison_json(
                    &previous_frequency_values[..6],
                    &current_frequencies[..6],
                    previous_modes,
                    &current_sampled_modes,
                    &sampled_mass,
                )?,
            ),
            _ => None,
        };
        mesh_receipts.push(mesh_json(
            elements_per_member,
            assembly.stiffness_global.len(),
            assembly.free_dof_indices.len(),
            &result,
            previous_frequencies.as_deref(),
            mac_matrix.as_deref(),
            cluster_comparison.as_deref(),
        ));
        previous_frequencies = Some(current_frequencies);
        previous_sampled_modes = Some(current_sampled_modes);
    }

    Ok(format!(
        concat!(
            "{{",
            "\"schema_version\":\"beam-chain-modal-sweep-receipt-v0.1\",",
            "\"benchmark_id\":\"PHYS-BEAM-CHAIN-MODAL-001\",",
            "\"source_commit\":{},",
            "\"request_canonical\":{},",
            "\"evidence_class\":\"deterministic-synthetic-mesh-sweep\",",
            "\"result\":\"pass\",",
            "\"element_counts_per_member\":[1,2,4,8],",
            "\"mesh_results\":[{}],",
            "\"claim_ceiling\":\"one-element to eight-elements-per-member synthetic Euler-Bernoulli model only; not a mesh-converged NBC05 200-element beam model, physical validation, structural safety or code compliance\"",
            "}}"
        ),
        json_quote(&source_commit),
        json_quote(&request),
        mesh_receipts.join(","),
    ))
}

fn fixture_nodes() -> Vec<BeamPinNode> {
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

fn fixture_members(elements_per_member: usize) -> Vec<BeamMemberChain> {
    ["AC", "BC"]
        .into_iter()
        .map(|id| BeamMemberChain {
            id: id.to_owned(),
            start_node_id: if id == "AC" { "A" } else { "B" }.to_owned(),
            end_node_id: "C".to_owned(),
            area_m2: 3.141592653589793e-4,
            youngs_modulus_pa: 210.0e9,
            density_kg_m3: 7800.0,
            second_moment_m4: 7.853981633974483e-9,
            elements_per_member,
        })
        .collect()
}

fn mesh_json(
    elements_per_member: usize,
    global_dof_count: usize,
    free_dof_count: usize,
    result: &physical_transformation_reference::beam_modal::BeamModalResult,
    previous_frequencies: Option<&[f64]>,
    mac_matrix: Option<&[Vec<f64>]>,
    cluster_comparison: Option<&str>,
) -> String {
    let frequencies = result
        .modes
        .iter()
        .map(|mode| {
            float_json(
                mode.frequency_hz
                    .expect("receipt only includes elastic modes"),
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let relative_changes = previous_frequencies.map(|previous| {
        previous
            .iter()
            .zip(result.modes.iter())
            .map(|(previous, mode)| {
                let current = mode
                    .frequency_hz
                    .expect("receipt only includes elastic modes");
                ((current - previous) / previous).abs()
            })
            .collect::<Vec<_>>()
    });
    let relative_changes_json = relative_changes
        .as_ref()
        .map(|values| {
            format!(
                "[{}]",
                values
                    .iter()
                    .map(|value| float_json(*value))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        })
        .unwrap_or_else(|| "null".to_owned());
    let maximum_relative_change_json = relative_changes
        .as_ref()
        .and_then(|values| values.iter().copied().reduce(f64::max))
        .map(float_json)
        .unwrap_or_else(|| "null".to_owned());
    let mac_matrix_json = mac_matrix
        .map(f64_matrix_json)
        .unwrap_or_else(|| "null".to_owned());
    let cluster_comparison_json = cluster_comparison.unwrap_or("null");
    format!(
        concat!(
            "{{",
            "\"elements_per_member\":{},",
            "\"global_dof_count\":{},",
            "\"free_dof_count\":{},",
            "\"mode_count\":{},",
            "\"maximum_normalized_eigen_residual\":{},",
            "\"maximum_mass_norm_residual\":{},",
            "\"maximum_off_diagonal_mass_orthogonality_residual\":{},",
            "\"frequencies_hz\":[{}],",
            "\"common_low_mode_frequency_relative_changes_from_previous_mesh\":{},",
            "\"max_common_low_mode_frequency_relative_change\":{},",
            "\"mode_shape_mac_low_six_from_previous_mesh\":{},",
            "\"clustered_mode_subspace_comparison_from_previous_mesh\":{}",
            "}}"
        ),
        elements_per_member,
        global_dof_count,
        free_dof_count,
        result.modes.len(),
        float_json(result.maximum_normalized_eigen_residual),
        float_json(result.maximum_mass_norm_residual),
        float_json(result.maximum_off_diagonal_mass_orthogonality_residual),
        frequencies,
        relative_changes_json,
        maximum_relative_change_json,
        mac_matrix_json,
        cluster_comparison_json,
    )
}

fn sampled_shape_mass_matrix(
    nodes: &[BeamPinNode],
    members: &[BeamMemberChain],
    sample_points_per_member: usize,
) -> Result<Vec<Vec<f64>>, String> {
    if sample_points_per_member < 2 {
        return Err("shape-mass quadrature requires at least two sample points".to_owned());
    }
    let mut ordered_members = members.to_vec();
    ordered_members.sort_by(|left, right| left.id.cmp(&right.id));
    let mut diagonal = Vec::with_capacity(ordered_members.len() * sample_points_per_member * 2);
    for member in &ordered_members {
        let start = nodes
            .iter()
            .find(|node| node.id == member.start_node_id)
            .ok_or_else(|| format!("missing start node {}", member.start_node_id))?;
        let end = nodes
            .iter()
            .find(|node| node.id == member.end_node_id)
            .ok_or_else(|| format!("missing end node {}", member.end_node_id))?;
        let length_m = (end.x_m - start.x_m).hypot(end.y_m - start.y_m);
        let total_mass_kg = member.density_kg_m3 * member.area_m2 * length_m;
        if !length_m.is_finite()
            || length_m <= 0.0
            || !total_mass_kg.is_finite()
            || total_mass_kg <= 0.0
        {
            return Err(format!("invalid sampled line-mass inputs for member {}", member.id));
        }
        for sample_index in 0..sample_points_per_member {
            let endpoint_weight = if sample_index == 0 || sample_index + 1 == sample_points_per_member {
                0.5
            } else {
                1.0
            };
            let weight =
                total_mass_kg * endpoint_weight / (sample_points_per_member - 1) as f64;
            diagonal.push(weight);
            diagonal.push(weight);
        }
    }

    if diagonal.iter().any(|weight| !weight.is_finite() || *weight <= 0.0) {
        return Err("sampled shape-mass quadrature generated an invalid weight".to_owned());
    }
    let size = diagonal.len();
    let mut mass = vec![vec![0.0; size]; size];
    for (index, weight) in diagonal.into_iter().enumerate() {
        mass[index][index] = weight;
    }
    Ok(mass)
}

fn cluster_subspace_comparison_json(
    previous_frequencies: &[f64],
    current_frequencies: &[f64],
    previous_sampled_modes: &[Vec<f64>],
    current_sampled_modes: &[Vec<f64>],
    sampled_mass: &[Vec<f64>],
) -> Result<String, String> {
    if previous_frequencies.len() != 6
        || current_frequencies.len() != 6
        || previous_sampled_modes.len() != 6
        || current_sampled_modes.len() != 6
    {
        return Err("cluster comparison expects exactly six common low modes".to_owned());
    }
    let previous_clusters = cluster_mode_frequencies_hz(
        previous_frequencies,
        FREQUENCY_CLUSTER_RELATIVE_GAP,
    )?;
    let current_clusters = cluster_mode_frequencies_hz(
        current_frequencies,
        FREQUENCY_CLUSTER_RELATIVE_GAP,
    )?;
    let previous_json = cluster_ranges_json(&previous_clusters);
    let current_json = cluster_ranges_json(&current_clusters);
    if previous_clusters != current_clusters {
        return Ok(format!(
            concat!(
                "{{",
                "\"status\":\"abstain-cluster-partition-changed\",",
                "\"relative_gap_tolerance\":{},",
                "\"previous_clusters\":{},",
                "\"current_clusters\":{},",
                "\"comparisons\":[]",
                "}}"
            ),
            float_json(FREQUENCY_CLUSTER_RELATIVE_GAP),
            previous_json,
            current_json,
        ));
    }

    let mut comparisons = Vec::with_capacity(previous_clusters.len());
    for range in &previous_clusters {
        let comparison = mass_weighted_modal_subspace_comparison(
            &previous_sampled_modes[range.clone()].to_vec(),
            &current_sampled_modes[range.clone()].to_vec(),
            sampled_mass,
        )?;
        let values_json = format!(
            "[{}]",
            comparison
                .principal_mac_values
                .iter()
                .map(|value| float_json(*value))
                .collect::<Vec<_>>()
                .join(",")
        );
        comparisons.push(format!(
            concat!(
                "{{",
                "\"mode_indices\":[{},{}],",
                "\"multiplicity\":{},",
                "\"principal_mac_values\":{},",
                "\"worst_principal_mac\":{},",
                "\"mean_principal_mac\":{}",
                "}}"
            ),
            range.start,
            range.end,
            range.len(),
            values_json,
            float_json(comparison.worst_principal_mac),
            float_json(comparison.mean_principal_mac),
        ));
    }
    Ok(format!(
        concat!(
            "{{",
            "\"status\":\"compared\",",
            "\"relative_gap_tolerance\":{},",
            "\"previous_clusters\":{},",
            "\"current_clusters\":{},",
            "\"matching_policy\":\"same sorted-rank ranges only; abstain if partition changes\",",
            "\"comparisons\":[{}]",
            "}}"
        ),
        float_json(FREQUENCY_CLUSTER_RELATIVE_GAP),
        previous_json,
        current_json,
        comparisons.join(","),
    ))
}

fn cluster_ranges_json(ranges: &[std::ops::Range<usize>]) -> String {
    format!(
        "[{}]",
        ranges
            .iter()
            .map(|range| format!("[{},{}]", range.start, range.end))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn f64_matrix_json(matrix: &[Vec<f64>]) -> String {
    format!(
        "[{}]",
        matrix
            .iter()
            .map(|row| {
                format!(
                    "[{}]",
                    row.iter()
                        .map(|value| float_json(*value))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn parse_source_commit() -> Result<String, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err(
            "usage: beam-chain-modal-sweep-reference --source-commit <40-char-git-sha>".to_owned(),
        );
    }
    let value = &args[1];
    if value.len() != 40 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err("--source-commit must be a 40-character Git SHA-1".to_owned());
    }
    Ok(value.to_ascii_lowercase())
}

fn float_json(value: f64) -> String {
    format!("{value:.17e}")
}

fn json_quote(value: &str) -> String {
    let mut escaped = String::from("\"");
    for character in value.chars() {
        match character {
            '\"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped.push('\"');
    escaped
}

#[cfg(test)]
mod tests {
    use super::parse_source_commit;

    #[test]
    fn source_commit_argument_is_required_and_strict() {
        assert!(parse_source_commit().is_err());
    }
}
