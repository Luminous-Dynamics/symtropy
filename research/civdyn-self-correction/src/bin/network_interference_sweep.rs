// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::network_adequacy::UndirectedNetwork;
use civdyn_self_correction::network_interference::{
    assess_contrast_support, assess_exposure_mapping, enumerate_two_stage_saturation_design,
    target_effect_bounds_over_completions, ClusteredNetwork, ExposureMapping,
    InterferenceMechanism,
};
use std::error::Error;

const SOURCE_ID: &str = "synthetic-network-interference-known-truth-v1";
const NETWORK_ID: &str = "four-node-two-cluster-cross-edge-v1";
const DESIGN_ID: &str = "uniform-cluster-saturation-then-uniform-within-cluster-v1";
const SPLIT_ID: &str = "synthetic-reference-not-empirical-validation";

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn emit(fields: &[String]) {
    println!(
        "{}",
        fields
            .iter()
            .map(|field| csv_field(field))
            .collect::<Vec<_>>()
            .join(",")
    );
}

fn optional(value: Option<String>) -> String {
    value.unwrap_or_default()
}

fn main() -> Result<(), Box<dyn Error>> {
    println!(
        "record_type,candidate,mapping_id,assignment_count,probability_mass,exposure_cell_count,nonconstant_exposure_cells,maximum_within_cell_outcome_range,mean_within_cell_outcome_range,direct_effect_truth,within_spillover_truth,between_spillover_truth,direct_supported_units,within_supported_units,between_supported_units,completion_count,target_unit,effect_lower,effect_upper,point_identified,source_id,network_id,design_id,split_id"
    );

    let graph =
        UndirectedNetwork::new(4, [(0, 1), (2, 3), (0, 2), (1, 3)])?;
    let clustered = ClusteredNetwork::new(graph, vec![0, 0, 1, 1])?;
    let assignments = enumerate_two_stage_saturation_design(&clustered)?;
    let probability_mass: f64 = assignments
        .iter()
        .map(|assignment| assignment.probability)
        .sum();
    let mechanism =
        InterferenceMechanism::new(vec![0.0; 4], 0.4, 0.2, 0.6)?;
    let support = assess_contrast_support(&clustered, &assignments)?;

    let mappings = [
        ExposureMapping::FullNetwork,
        ExposureMapping::WithinClusterOnly,
        ExposureMapping::WeightedNeighborMean {
            within_weight: 1,
            between_weight: 1,
        },
        ExposureMapping::Threshold {
            minimum_treated_neighbors: 1,
        },
        ExposureMapping::TruncatedCanonicalNeighbors {
            maximum_neighbors: 1,
        },
    ];

    for mapping in &mappings {
        let summary =
            assess_exposure_mapping(&clustered, &assignments, &mechanism, mapping)?;
        emit(&[
            "exposure_mapping".to_string(),
            mapping.id().to_string(),
            summary.mapping_id.to_string(),
            assignments.len().to_string(),
            format!("{probability_mass:.16}"),
            summary.exposure_cell_count.to_string(),
            summary.nonconstant_exposure_cell_count.to_string(),
            format!("{:.16}", summary.maximum_within_cell_outcome_range),
            format!("{:.16}", summary.mean_within_cell_outcome_range),
            format!("{:.16}", mechanism.known_effects().direct_effect),
            format!(
                "{:.16}",
                mechanism
                    .known_effects()
                    .within_cluster_spillover_per_treated_neighbor
            ),
            format!(
                "{:.16}",
                mechanism
                    .known_effects()
                    .between_cluster_spillover_per_treated_neighbor
            ),
            support.direct_supported_units.to_string(),
            support
                .within_cluster_spillover_supported_units
                .to_string(),
            support
                .between_cluster_spillover_supported_units
                .to_string(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            SOURCE_ID.to_string(),
            NETWORK_ID.to_string(),
            DESIGN_ID.to_string(),
            SPLIT_ID.to_string(),
        ]);
    }

    let observed_partial =
        UndirectedNetwork::new(4, [(0, 1), (2, 3)])?;
    let completion_a =
        UndirectedNetwork::new(4, [(0, 1), (2, 3), (0, 2), (1, 3)])?;
    let completion_b =
        UndirectedNetwork::new(4, [(0, 1), (2, 3), (0, 3), (1, 2)])?;
    let bounds = target_effect_bounds_over_completions(
        &observed_partial,
        &[completion_a, completion_b],
        &[0, 0, 1, 1],
        &mechanism,
        &[true, false, false, false],
        3,
    )?;

    emit(&[
        "partial_network_bounds".to_string(),
        "two_admissible_completions".to_string(),
        "network_completion_effect_set".to_string(),
        assignments.len().to_string(),
        format!("{probability_mass:.16}"),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        format!("{:.16}", mechanism.known_effects().direct_effect),
        format!(
            "{:.16}",
            mechanism
                .known_effects()
                .within_cluster_spillover_per_treated_neighbor
        ),
        format!(
            "{:.16}",
            mechanism
                .known_effects()
                .between_cluster_spillover_per_treated_neighbor
        ),
        support.direct_supported_units.to_string(),
        support
            .within_cluster_spillover_supported_units
            .to_string(),
        support
            .between_cluster_spillover_supported_units
            .to_string(),
        bounds.completion_count.to_string(),
        bounds.target_unit.to_string(),
        format!("{:.16}", bounds.lower),
        format!("{:.16}", bounds.upper),
        bounds.point_identified.to_string(),
        SOURCE_ID.to_string(),
        NETWORK_ID.to_string(),
        DESIGN_ID.to_string(),
        SPLIT_ID.to_string(),
    ]);

    Ok(())
}
