// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::joint_adequacy::{
    assess_joint_predictive_draws, JointComponentSpec, JointMetricConfig, JointObservation,
    JointPredictiveDraw, JointStatisticSpec,
};
use civdyn_self_correction::model_adequacy::{IndependentUnitKind, SplitManifest, SplitRole};
use civdyn_self_correction::network_adequacy::UndirectedNetwork;

const DRAW_COUNT: usize = 20;
const CSV_COLUMNS: usize = 23;

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "network adequacy receipt must match the frozen schema"
    );
    println!("{}", fields.join(","));
}

fn cycle_six() -> Result<UndirectedNetwork, Box<dyn std::error::Error>> {
    Ok(UndirectedNetwork::new(
        6,
        [(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 0)],
    )?)
}

fn two_triangles() -> Result<UndirectedNetwork, Box<dyn std::error::Error>> {
    Ok(UndirectedNetwork::new(
        6,
        [(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3)],
    )?)
}

fn schema() -> JointStatisticSpec {
    let components = [
        ("mean-degree", "edges_per_node", 2.0, 1.0),
        ("degree-variance", "degree_squared", 0.0, 1.0),
        ("global-transitivity", "ratio", 0.0, 1.0),
        ("component-count", "components", 1.0, 1.0),
        ("largest-component-share", "ratio", 1.0, 1.0),
        ("mean-finite-shortest-path", "hops", 1.8, 1.0),
        ("reachable-unordered-pairs", "pairs", 15.0, 15.0),
    ]
    .into_iter()
    .map(|(id, unit, center, scale)| JointComponentSpec {
        id: id.to_string(),
        measurement_unit: unit.to_string(),
        reference_center: center,
        reference_scale: scale,
    })
    .collect();

    JointStatisticSpec {
        id: "held-out-network-topology".to_string(),
        source_family: "synthetic-network-known-answer".to_string(),
        source_digest: "sha256-network-source".to_string(),
        population: "six-node-network-fixture".to_string(),
        context: "same-degree-opposite-topology".to_string(),
        independent_unit: IndependentUnitKind::Network,
        role: SplitRole::Validation,
        primary: true,
        required: true,
        minimum_draws: DRAW_COUNT,
        components,
    }
}

fn manifest() -> SplitManifest {
    SplitManifest {
        id: "network-split-v1".to_string(),
        independent_unit: IndependentUnitKind::Network,
        calibration_data_digest: "sha256-network-calibration".to_string(),
        manifest_digest: "sha256-network-manifest".to_string(),
        calibration_units: vec!["network-calibration".to_string()],
        validation_units: vec!["network-held-out".to_string()],
        challenge_units: Vec::new(),
    }
}

fn assess(
    candidate: &str,
    graph: &UndirectedNetwork,
    observed: &UndirectedNetwork,
    model_digest: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let statistic = schema();
    let split = manifest();
    let summary = graph.summarize();
    let observation = JointObservation {
        statistic_id: statistic.id.clone(),
        independent_unit_id: "network-held-out".to_string(),
        sample_size: observed.node_count(),
        source_digest: statistic.source_digest.clone(),
        observation_digest: "sha256-network-observation".to_string(),
        split_manifest_digest: split.manifest_digest.clone(),
        components: observed.summarize().joint_component_values()?,
    };
    let predictive_components = summary.joint_component_values()?;
    let draws: Vec<JointPredictiveDraw> = (0..DRAW_COUNT)
        .map(|draw_index| JointPredictiveDraw {
            statistic_id: statistic.id.clone(),
            independent_unit_id: "network-held-out".to_string(),
            sample_size: observed.node_count(),
            draw_index: draw_index as u64,
            model_digest: model_digest.to_string(),
            draw_set_digest: format!("sha256-network-draws-{model_digest}"),
            calibration_data_digest: Some(split.calibration_data_digest.clone()),
            split_manifest_digest: split.manifest_digest.clone(),
            components: predictive_components.clone(),
        })
        .collect();

    let result = assess_joint_predictive_draws(
        &statistic,
        &split,
        SplitRole::Validation,
        &observation,
        &draws,
        &JointMetricConfig {
            energy_beta: 1.0,
            variogram_power: 1.0,
            pair_weights: vec![1.0; 21],
        },
    )?;

    let degree_sequence = summary
        .degree_sequence
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(";");
    let degree_histogram = summary
        .degree_histogram
        .iter()
        .map(|(degree, count)| format!("{degree}:{count}"))
        .collect::<Vec<_>>()
        .join(";");
    let mean_path = summary
        .mean_finite_shortest_path
        .map(|value| format!("{value:.6}"))
        .unwrap_or_default();

    Ok(vec![
        candidate.to_string(),
        result.independent_unit_kind.name().to_string(),
        "undirected_unweighted_simple".to_string(),
        summary.node_count.to_string(),
        summary.edge_count.to_string(),
        degree_sequence,
        degree_histogram,
        summary.component_count.to_string(),
        format!("{:.6}", summary.largest_component_share),
        format!("{:.6}", summary.global_transitivity),
        summary.reachable_unordered_pairs.to_string(),
        mean_path,
        format!("{:.6}", result.energy_beta),
        format!("{:.16}", result.energy_score),
        format!("{:.6}", result.variogram_power),
        format!("{:.16}", result.variogram_score),
        result.predictive_draw_count.to_string(),
        result.model_digest,
        result.draw_set_digest,
        result.source_digest,
        result.observation_digest,
        result.calibration_data_digest,
        result.split_manifest_digest,
    ])
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let observed = cycle_six()?;
    let correct = cycle_six()?;
    let degree_matched_wrong_topology = two_triangles()?;

    println!(
        "candidate,independent_unit,graph_semantics,node_count,edge_count,degree_sequence,degree_histogram,component_count,largest_component_share,global_transitivity,reachable_unordered_pairs,mean_finite_shortest_path,energy_beta,energy_score,variogram_power,variogram_score,draw_count,model_digest,draw_set_digest,source_digest,observation_digest,calibration_digest,split_manifest_digest"
    );

    emit(assess(
        "topology_correct",
        &correct,
        &observed,
        "sha256-network-model-correct",
    )?);
    emit(assess(
        "same_degree_wrong_topology",
        &degree_matched_wrong_topology,
        &observed,
        "sha256-network-model-wrong",
    )?);
    Ok(())
}
