// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::joint_adequacy::{
    assess_joint_predictive_draws, JointComponentSpec, JointComponentValue, JointMetricConfig,
    JointObservation, JointPredictiveDraw, JointStatisticSpec,
};
use civdyn_self_correction::model_adequacy::{IndependentUnitKind, SplitManifest, SplitRole};

const CSV_COLUMNS: usize = 23;
const SOURCE_DIGEST: &str = "fixture-joint-water-energy-source-v1";
const CALIBRATION_DIGEST: &str = "fixture-joint-calibration-v1";

fn emit(fields: Vec<String>) {
    assert_eq!(fields.len(), CSV_COLUMNS, "joint adequacy receipt schema mismatch");
    println!("{}", fields.join(","));
}

fn component(
    id: &str,
    unit: &str,
    value: f64,
) -> JointComponentValue {
    JointComponentValue {
        component_id: id.to_string(),
        measurement_unit: unit.to_string(),
        value,
    }
}

fn fixture() -> (JointStatisticSpec, SplitManifest, JointObservation, JointMetricConfig) {
    let statistic = JointStatisticSpec {
        id: "district-water-energy-state".to_string(),
        source_family: "synthetic-district-resource-source".to_string(),
        source_digest: SOURCE_DIGEST.to_string(),
        population: "synthetic-district-households".to_string(),
        context: "known-answer-same-marginals-opposite-dependence".to_string(),
        independent_unit: IndependentUnitKind::Region,
        role: SplitRole::Validation,
        primary: true,
        required: true,
        minimum_draws: 2,
        components: vec![
            JointComponentSpec {
                id: "water-service".to_string(),
                measurement_unit: "fraction".to_string(),
                reference_center: 0.0,
                reference_scale: 1.0,
            },
            JointComponentSpec {
                id: "energy-service".to_string(),
                measurement_unit: "fraction".to_string(),
                reference_center: 0.0,
                reference_scale: 1.0,
            },
        ],
    };

    let manifest = SplitManifest {
        id: "civdyn-joint-split-v0.1".to_string(),
        independent_unit: IndependentUnitKind::Region,
        calibration_data_digest: CALIBRATION_DIGEST.to_string(),
        manifest_digest: "fixture-joint-split-manifest-v1".to_string(),
        calibration_units: vec!["district-calibration".to_string()],
        validation_units: vec!["district-held-out".to_string()],
        challenge_units: vec![],
    };

    let observation = JointObservation {
        statistic_id: statistic.id.clone(),
        independent_unit_id: "district-held-out".to_string(),
        sample_size: 10,
        source_digest: SOURCE_DIGEST.to_string(),
        observation_digest: "fixture-joint-observation-v1".to_string(),
        split_manifest_digest: "fixture-joint-split-manifest-v1".to_string(),
        components: vec![
            component("water-service", "fraction", 1.0),
            component("energy-service", "fraction", 1.0),
        ],
    };

    let config = JointMetricConfig {
        energy_beta: 1.0,
        variogram_power: 1.0,
        pair_weights: vec![1.0],
    };

    (statistic, manifest, observation, config)
}

fn predictive_draws(
    model_digest: &str,
    draw_set_digest: &str,
    values: [[f64; 2]; 2],
) -> Vec<JointPredictiveDraw> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| JointPredictiveDraw {
            statistic_id: "district-water-energy-state".to_string(),
            independent_unit_id: "district-held-out".to_string(),
            sample_size: 10,
            draw_index: index as u64,
            model_digest: model_digest.to_string(),
            draw_set_digest: draw_set_digest.to_string(),
            calibration_data_digest: Some(CALIBRATION_DIGEST.to_string()),
            split_manifest_digest: "fixture-joint-split-manifest-v1".to_string(),
            components: vec![
                component("water-service", "fraction", value[0]),
                component("energy-service", "fraction", value[1]),
            ],
        })
        .collect()
}

struct JointCandidate {
    name: &'static str,
    model_digest: &'static str,
    draw_set_digest: &'static str,
    values: [[f64; 2]; 2],
}

fn emit_candidate(
    candidate: &JointCandidate,
    statistic: &JointStatisticSpec,
    manifest: &SplitManifest,
    observation: &JointObservation,
    config: &JointMetricConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let draws = predictive_draws(
        candidate.model_digest,
        candidate.draw_set_digest,
        candidate.values,
    );
    let result = assess_joint_predictive_draws(
        statistic,
        manifest,
        SplitRole::Validation,
        observation,
        &draws,
        config,
    )?;

    for pair in &result.variogram_pairs {
        let fields = vec![
            candidate.name.to_string(),
            "joint_pair".to_string(),
            result.statistic_id.clone(),
            result.independent_unit_id.clone(),
            result.independent_unit_kind.name().to_string(),
            result.sample_size.to_string(),
            result.predictive_draw_count.to_string(),
            format!("{energy_beta:.6}", energy_beta = result.energy_beta),
            format!("{energy_score:.8}", energy_score = result.energy_score),
            format!("{variogram_power:.6}", variogram_power = result.variogram_power),
            format!("{variogram_score:.8}", variogram_score = result.variogram_score),
            pair.component_a.clone(),
            pair.component_b.clone(),
            format!("{pair_weight:.6}", pair_weight = pair.weight),
            format!("{observed_pair_distance:.8}", observed_pair_distance = pair.observed_pair_distance),
            format!("{predictive_pair_distance:.8}", predictive_pair_distance = pair.predictive_pair_distance),
            format!("{weighted_contribution:.8}", weighted_contribution = pair.weighted_contribution),
            result.model_digest.clone(),
            result.draw_set_digest.clone(),
            result.source_digest.clone(),
            result.observation_digest.clone(),
            result.calibration_data_digest.clone(),
            result.split_manifest_digest.clone(),
        ];
        emit(fields);
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (statistic, manifest, observation, config) = fixture();
    println!(
        "candidate,record_type,statistic_id,unit_id,independent_unit_kind,sample_size,predictive_draws,energy_beta,energy_score,variogram_power,variogram_score,component_a,component_b,pair_weight,observed_pair_distance,predictive_pair_distance,weighted_contribution,model_digest,draw_set_digest,source_digest,observation_digest,calibration_digest,split_manifest_digest"
    );

    // Both candidates have exactly the same marginal draws {0,1} for each
    // resource. Only their cross-resource coupling differs.
    let candidates = [
        JointCandidate {
            name: "positive-dependence",
            model_digest: "sha256-joint-model-positive",
            draw_set_digest: "sha256-joint-draws-positive",
            values: [[0.0, 0.0], [1.0, 1.0]],
        },
        JointCandidate {
            name: "negative-dependence",
            model_digest: "sha256-joint-model-negative",
            draw_set_digest: "sha256-joint-draws-negative",
            values: [[0.0, 1.0], [1.0, 0.0]],
        },
    ];
    for candidate in &candidates {
        emit_candidate(candidate, &statistic, &manifest, &observation, &config)?;
    }
    Ok(())
}
