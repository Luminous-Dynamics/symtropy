// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::joint_adequacy::{
    assess_joint_predictive_draws, JointComponentSpec, JointMetricConfig, JointObservation,
    JointPredictiveDraw, JointStatisticSpec,
};
use civdyn_self_correction::model_adequacy::{IndependentUnitKind, SplitManifest, SplitRole};
use civdyn_self_correction::network_adequacy::{
    TemporalNetworkSequence, UndirectedNetwork,
};

const DRAW_COUNT: usize = 20;
const CSV_COLUMNS: usize = 26;
const SOURCE_DIGEST: &str = "sha256-temporal-network-source";
const CALIBRATION_DIGEST: &str = "sha256-temporal-network-calibration";
const MANIFEST_DIGEST: &str = "sha256-temporal-network-manifest";

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

fn three_disjoint_edges() -> Result<UndirectedNetwork, Box<dyn std::error::Error>> {
    Ok(UndirectedNetwork::new(6, [(0, 1), (2, 3), (4, 5)])?)
}

fn sequence(reversed: bool) -> Result<TemporalNetworkSequence, Box<dyn std::error::Error>> {
    let mut snapshots = vec![cycle_six()?, two_triangles()?, three_disjoint_edges()?];
    if reversed {
        snapshots.reverse();
    }
    Ok(TemporalNetworkSequence::new(
        vec![10, 20, 30],
        snapshots,
    )?)
}

fn schema() -> JointStatisticSpec {
    let components = [
        ("mean-component-count", "components", 2.0, 1.0),
        ("component-count-trend", "components_per_tick", 0.1, 1.0),
        ("mean-degree", "edges_per_node", 5.0 / 3.0, 1.0),
        ("mean-degree-trend", "edges_per_node_per_tick", -0.05, 1.0),
        ("mean-transitivity", "ratio", 1.0 / 3.0, 1.0),
        ("transitivity-trend", "ratio_per_tick", 0.0, 1.0),
        ("mean-edge-jaccard", "ratio", 11.0 / 28.0, 1.0),
        ("mean-edge-turnover", "ratio", 17.0 / 28.0, 1.0),
        ("mean-edge-addition-rate", "dyad_fraction_per_transition", 0.1, 1.0),
        ("mean-edge-removal-rate", "dyad_fraction_per_transition", 0.2, 1.0),
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
        id: "temporal-network-path".to_string(),
        source_family: "synthetic-temporal-network-known-answer".to_string(),
        source_digest: SOURCE_DIGEST.to_string(),
        population: "six-node-temporal-network".to_string(),
        context: "same-snapshot-multiset-reversed-order".to_string(),
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
        id: "temporal-network-split-v1".to_string(),
        independent_unit: IndependentUnitKind::Network,
        calibration_data_digest: CALIBRATION_DIGEST.to_string(),
        manifest_digest: MANIFEST_DIGEST.to_string(),
        calibration_units: vec!["network-calibration".to_string()],
        validation_units: vec!["network-held-out".to_string()],
        challenge_units: Vec::new(),
    }
}

fn assess(
    candidate: &str,
    predicted: &TemporalNetworkSequence,
    observed: &TemporalNetworkSequence,
    model_digest: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let statistic = schema();
    let split = manifest();
    let summary = predicted.summarize()?;
    let observed_summary = observed.summarize()?;
    let observation = JointObservation {
        statistic_id: statistic.id.clone(),
        independent_unit_id: "network-held-out".to_string(),
        sample_size: observed_summary.snapshot_count,
        source_digest: statistic.source_digest.clone(),
        observation_digest: "sha256-temporal-network-observation".to_string(),
        split_manifest_digest: split.manifest_digest.clone(),
        components: observed_summary.joint_component_values(),
    };
    let components = summary.joint_component_values();
    let draws: Vec<JointPredictiveDraw> = (0..DRAW_COUNT)
        .map(|draw_index| JointPredictiveDraw {
            statistic_id: statistic.id.clone(),
            independent_unit_id: "network-held-out".to_string(),
            sample_size: observed_summary.snapshot_count,
            draw_index: draw_index as u64,
            model_digest: model_digest.to_string(),
            draw_set_digest: format!("sha256-temporal-draws-{model_digest}"),
            calibration_data_digest: Some(split.calibration_data_digest.clone()),
            split_manifest_digest: split.manifest_digest.clone(),
            components: components.clone(),
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
            pair_weights: vec![1.0; 45],
        },
    )?;

    Ok(vec![
        candidate.to_string(),
        "undirected_unweighted_simple_stable_node_universe".to_string(),
        summary.snapshot_count.to_string(),
        summary.start_tick.to_string(),
        summary.end_tick.to_string(),
        format!("{:.8}", summary.mean_component_count),
        format!("{:.8}", summary.component_count_trend_per_tick),
        format!("{:.8}", summary.mean_degree),
        format!("{:.8}", summary.mean_degree_trend_per_tick),
        format!("{:.8}", summary.mean_global_transitivity),
        format!("{:.8}", summary.transitivity_trend_per_tick),
        format!("{:.8}", summary.mean_adjacent_edge_jaccard),
        format!("{:.8}", summary.mean_adjacent_edge_turnover),
        format!("{:.8}", summary.mean_edge_addition_rate),
        format!("{:.8}", summary.mean_edge_removal_rate),
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
    let observed = sequence(false)?;
    let correct = sequence(false)?;
    let reversed = sequence(true)?;

    println!(
        "candidate,graph_semantics,snapshot_count,start_tick,end_tick,mean_component_count,component_count_trend_per_tick,mean_degree,mean_degree_trend_per_tick,mean_transitivity,transitivity_trend_per_tick,mean_edge_jaccard,mean_edge_turnover,mean_edge_addition_rate,mean_edge_removal_rate,energy_beta,energy_score,variogram_power,variogram_score,draw_count,model_digest,draw_set_digest,source_digest,observation_digest,calibration_digest,split_manifest_digest"
    );

    emit(assess(
        "temporal_order_correct",
        &correct,
        &observed,
        "sha256-temporal-model-correct",
    )?);
    emit(assess(
        "same_snapshots_reversed_order",
        &reversed,
        &observed,
        "sha256-temporal-model-reversed",
    )?);
    Ok(())
}
