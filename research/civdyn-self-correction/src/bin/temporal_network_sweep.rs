// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::joint_adequacy::{
    assess_joint_predictive_draws, JointComponentSpec, JointMetricConfig, JointObservation,
    JointPredictiveDraw, JointStatisticSpec,
};
use civdyn_self_correction::model_adequacy::{IndependentUnitKind, SplitManifest, SplitRole};
use civdyn_self_correction::temporal_network::{
    TemporalContact, TemporalNetwork, TemporalNetworkSummary,
};

const DRAW_COUNT: usize = 20;
const CSV_COLUMNS: usize = 30;

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "temporal network receipt must match the frozen schema"
    );
    println!(
        "{}",
        fields
            .iter()
            .map(|field| csv_field(field))
            .collect::<Vec<_>>()
            .join(",")
    );
}

fn forward_path() -> Result<TemporalNetwork, Box<dyn std::error::Error>> {
    Ok(TemporalNetwork::new(
        4,
        [
            TemporalContact { tick: 1, left: 0, right: 1 },
            TemporalContact { tick: 2, left: 1, right: 2 },
            TemporalContact { tick: 3, left: 2, right: 3 },
        ],
    )?)
}

fn scrambled_path() -> Result<TemporalNetwork, Box<dyn std::error::Error>> {
    Ok(TemporalNetwork::new(
        4,
        [
            TemporalContact { tick: 1, left: 0, right: 1 },
            TemporalContact { tick: 2, left: 2, right: 3 },
            TemporalContact { tick: 3, left: 1, right: 2 },
        ],
    )?)
}

fn statistic_schema(
    observed_components: &[civdyn_self_correction::joint_adequacy::JointComponentValue],
) -> JointStatisticSpec {
    let components = observed_components
        .iter()
        .map(|component| JointComponentSpec {
            id: component.component_id.clone(),
            measurement_unit: component.measurement_unit.clone(),
            reference_center: component.value,
            reference_scale: component.value.abs().max(1.0),
        })
        .collect();
    JointStatisticSpec {
        id: "held-out-temporal-network".to_string(),
        source_family: "synthetic-temporal-network-known-answer".to_string(),
        source_digest: "sha256-temporal-network-source".to_string(),
        population: "four-node-temporal-network".to_string(),
        context: "same-union-different-event-order".to_string(),
        independent_unit: IndependentUnitKind::Network,
        role: SplitRole::Validation,
        primary: true,
        required: true,
        minimum_draws: DRAW_COUNT,
        components,
    }
}

fn split_manifest() -> SplitManifest {
    SplitManifest {
        id: "temporal-network-split-v1".to_string(),
        independent_unit: IndependentUnitKind::Network,
        calibration_data_digest: "sha256-temporal-network-calibration".to_string(),
        manifest_digest: "sha256-temporal-network-manifest".to_string(),
        calibration_units: vec!["temporal-network-calibration".to_string()],
        validation_units: vec!["temporal-network-held-out".to_string()],
        challenge_units: Vec::new(),
    }
}

fn assess_candidate(
    predicted: &TemporalNetwork,
    observed_components: &[civdyn_self_correction::joint_adequacy::JointComponentValue],
    model_digest: &str,
) -> Result<civdyn_self_correction::joint_adequacy::JointAdequacyResult, Box<dyn std::error::Error>> {
    let summary = predicted.summarize()?;
    let predicted_components = summary.joint_component_values()?;
    let statistic = statistic_schema(observed_components);
    let manifest = split_manifest();
    let observation = JointObservation {
        statistic_id: statistic.id.clone(),
        independent_unit_id: "temporal-network-held-out".to_string(),
        sample_size: 4,
        source_digest: statistic.source_digest.clone(),
        observation_digest: "sha256-temporal-network-observation".to_string(),
        split_manifest_digest: manifest.manifest_digest.clone(),
        components: observed_components.to_vec(),
    };
    let draws: Vec<JointPredictiveDraw> = (0..DRAW_COUNT)
        .map(|draw_index| JointPredictiveDraw {
            statistic_id: statistic.id.clone(),
            independent_unit_id: "temporal-network-held-out".to_string(),
            sample_size: 4,
            draw_index: draw_index as u64,
            model_digest: model_digest.to_string(),
            draw_set_digest: format!("sha256-temporal-network-draws-{model_digest}"),
            calibration_data_digest: Some(manifest.calibration_data_digest.clone()),
            split_manifest_digest: manifest.manifest_digest.clone(),
            components: predicted_components.clone(),
        })
        .collect();
    Ok(assess_joint_predictive_draws(
        &statistic,
        &manifest,
        SplitRole::Validation,
        &observation,
        &draws,
        &JointMetricConfig {
            energy_beta: 1.0,
            variogram_power: 1.0,
            pair_weights: vec![1.0; 78],
        },
    )?)
}

fn receipt_row(
    candidate: &str,
    summary: &TemporalNetworkSummary,
    result: &civdyn_self_correction::joint_adequacy::JointAdequacyResult,
) -> Vec<String> {
    let static_summary = &summary.static_union;
    let degree_sequence = static_summary
        .degree_sequence
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(";");
    let mean_path = static_summary
        .mean_finite_shortest_path
        .map(|value| format!("{value:.8}"))
        .unwrap_or_default();
    let mean_temporal_delay = summary
        .mean_earliest_arrival_delay
        .map(|value| format!("{value:.8}"))
        .unwrap_or_default();
    let reachable_pairs = summary
        .reachable_pairs
        .iter()
        .map(|pair| {
            format!(
                "{}>{}@{}",
                pair.source, pair.destination, pair.arrival_tick
            )
        })
        .collect::<Vec<_>>()
        .join(";");

    vec![
        candidate.to_string(),
        "undirected_unweighted_instantaneous_contacts".to_string(),
        static_summary.node_count.to_string(),
        summary.event_count.to_string(),
        summary.unique_edge_count.to_string(),
        summary.active_tick_count.to_string(),
        degree_sequence,
        format!("{:.8}", static_summary.mean_degree),
        format!("{:.8}", static_summary.degree_variance),
        static_summary.component_count.to_string(),
        format!("{:.8}", static_summary.largest_component_share),
        format!("{:.8}", static_summary.global_transitivity),
        mean_path,
        static_summary.reachable_unordered_pairs.to_string(),
        summary.temporal_reachable_ordered_pairs.to_string(),
        format!("{:.8}", summary.temporal_reachability_fraction),
        mean_temporal_delay,
        reachable_pairs,
        format!("{:.8}", result.energy_beta),
        format!("{:.16}", result.energy_score),
        format!("{:.8}", result.variogram_power),
        format!("{:.16}", result.variogram_score),
        result.predictive_draw_count.to_string(),
        result.model_digest.clone(),
        result.draw_set_digest.clone(),
        result.source_digest.clone(),
        result.observation_digest.clone(),
        result.calibration_data_digest.clone(),
        result.split_manifest_digest.clone(),
        result.independent_unit_kind.name().to_string(),
    ]
}

fn emit_summary_row(
    candidate: &str,
    graph: &TemporalNetwork,
    observed_components: &[civdyn_self_correction::joint_adequacy::JointComponentValue],
    model_digest: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let summary = graph.summarize()?;
    let result = assess_candidate(graph, observed_components, model_digest)?;
    emit(receipt_row(candidate, &summary, &result));
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let observed_graph = forward_path()?;
    let observed_summary = observed_graph.summarize()?;
    let observed_components = observed_summary.joint_component_values()?;

    println!(
        "candidate,contact_semantics,node_count,event_count,unique_edge_count,active_tick_count,degree_sequence,mean_degree,degree_variance,component_count,largest_component_share,global_transitivity,mean_finite_shortest_path,static_reachable_unordered_pairs,temporal_reachable_ordered_pairs,temporal_reachability_fraction,mean_earliest_arrival_delay,temporal_pair_receipt,energy_beta,energy_score,variogram_power,variogram_score,draw_count,model_digest,draw_set_digest,source_digest,observation_digest,calibration_digest,split_manifest_digest,independent_unit_kind"
    );

    emit_summary_row(
        "temporal_order_correct",
        &observed_graph,
        &observed_components,
        "sha256-temporal-model-correct",
    )?;
    emit_summary_row(
        "same_union_wrong_order",
        &scrambled_path()?,
        &observed_components,
        "sha256-temporal-model-wrong",
    )?;
    Ok(())
}
