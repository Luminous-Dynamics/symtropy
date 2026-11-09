// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::model_adequacy::{
    IndependentUnitKind, ObservedStatistic, PredictiveDraw, PredictiveKind, SplitManifest,
    SplitRole, StatisticRegistry, StatisticSpec,
};
use civdyn_self_correction::split_predictive::{
    divided_split_predictive_check, single_split_predictive_check, DividedSpcResult,
    SingleSpcResult, SpcTail,
};

const CSV_COLUMNS: usize = 23;
const UNIT_COUNT: usize = 10;
const DRAW_COUNT: usize = 100;
const MONTE_CARLO_REPLICATES: usize = 1_999;
const CALIBRATION_DIGEST: &str = "fixture-calibration-v1";
const MODEL_DIGEST: &str = "fixture-model-v1";
const SOURCE_DIGEST: &str = "fixture-source-water-flow-v1";

fn emit(fields: Vec<String>) {
    assert_eq!(fields.len(), CSV_COLUMNS, "SPC receipt schema mismatch");
    println!("{}", fields.join(","));
}

fn single_row(result: &SingleSpcResult) -> Vec<String> {
    vec![
        "single_spc".to_string(),
        result.statistic_id.clone(),
        result.independent_unit_id.clone(),
        result.tail.name().to_string(),
        format!("{:.8}", result.predictive_p_value),
        format!("{:.8}", result.randomized_lower_tail_rank),
        format!("{:.8}", result.observed_value),
        result.sample_size.to_string(),
        result.predictive_draw_count.to_string(),
        result.model_digest.clone(),
        result.draw_set_digest.clone(),
        result.source_digest.clone(),
        result.observation_digest.clone(),
        result.calibration_data_digest.clone(),
        result.split_manifest_digest.clone(),
        result.randomization_seed.to_string(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        result.independent_unit_kind.name().to_string(),
    ]
}

fn divided_row(result: &DividedSpcResult, source_digest: &str) -> Vec<String> {
    vec![
        "divided_spc".to_string(),
        result.statistic_id.clone(),
        String::new(),
        result.tail.name().to_string(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        result.model_digest.clone(),
        String::new(),
        source_digest.to_string(),
        String::new(),
        result.calibration_data_digest.clone(),
        result.split_manifest_digest.clone(),
        String::new(),
        format!("{:.8}", result.ks_statistic),
        format!("{:.8}", result.uniformity_p_value),
        result.unit_count.to_string(),
        result.sample_size.to_string(),
        result.monte_carlo_replicates.to_string(),
        result.monte_carlo_seed.to_string(),
        result.independent_unit_kind.name().to_string(),
    ]
}

fn fixture() -> (
    StatisticRegistry,
    SplitManifest,
    Vec<ObservedStatistic>,
    Vec<PredictiveDraw>,
) {
    let validation_units: Vec<String> = (0..UNIT_COUNT)
        .map(|index| format!("region-{index:02}"))
        .collect();

    let registry = StatisticRegistry {
        version: "civdyn-spc-statistics-v0.1".to_string(),
        statistics: vec![StatisticSpec {
            id: "basin-water-flow".to_string(),
            construct: "basin water flow".to_string(),
            measurement_unit: "m3_per_day".to_string(),
            population: "synthetic-regions".to_string(),
            context: "frozen-spc-known-answer-v0.1".to_string(),
            source_family: "synthetic-water-flow-source".to_string(),
            source_digest: SOURCE_DIGEST.to_string(),
            independent_unit: IndependentUnitKind::Region,
            role: SplitRole::Validation,
            primary: true,
            required: true,
            minimum_draws: DRAW_COUNT,
        }],
    };

    let manifest = SplitManifest {
        id: "civdyn-spc-split-v0.1".to_string(),
        independent_unit: IndependentUnitKind::Region,
        calibration_data_digest: CALIBRATION_DIGEST.to_string(),
        manifest_digest: "fixture-spc-manifest-v1".to_string(),
        calibration_units: vec!["calibration-region".to_string()],
        validation_units: validation_units.clone(),
        challenge_units: vec![],
    };

    let observations: Vec<ObservedStatistic> = validation_units
        .iter()
        .enumerate()
        .map(|(index, unit_id)| ObservedStatistic {
            statistic_id: "basin-water-flow".to_string(),
            independent_unit_id: unit_id.clone(),
            measurement_unit: "m3_per_day".to_string(),
            source_digest: SOURCE_DIGEST.to_string(),
            sample_size: 200,
            value: -0.9 + index as f64 * 0.18,
            measurement_standard_error: Some(0.02),
            observation_digest: format!("fixture-observation-{unit_id}"),
        })
        .collect();

    let base_values: Vec<f64> = (0..DRAW_COUNT)
        .map(|index| -1.0 + index as f64 * (2.0 / (DRAW_COUNT - 1) as f64))
        .collect();
    let draws: Vec<PredictiveDraw> = validation_units
        .iter()
        .flat_map(|unit_id| {
            base_values
                .iter()
                .enumerate()
                .map(|(index, value)| PredictiveDraw {
                    kind: PredictiveKind::Split,
                    statistic_id: "basin-water-flow".to_string(),
                    independent_unit_id: unit_id.clone(),
                    sample_size: 200,
                    draw_index: index as u64,
                    value: *value,
                    model_digest: MODEL_DIGEST.to_string(),
                    draw_set_digest: format!("fixture-draw-set-{unit_id}"),
                    calibration_data_digest: Some(CALIBRATION_DIGEST.to_string()),
                })
        })
        .collect();

    (registry, manifest, observations, draws)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (registry, manifest, observations, draws) = fixture();
    let mut checks = Vec::with_capacity(UNIT_COUNT);

    for unit_id in &manifest.validation_units {
        let check = single_split_predictive_check(
            &registry,
            &manifest,
            &observations,
            &draws,
            SplitRole::Validation,
            "basin-water-flow",
            unit_id,
            SpcTail::Lower,
            0x4356_4459_4e53_5043,
        )?;
        checks.push(check);
    }

    let divided = divided_split_predictive_check(
        &manifest,
        SplitRole::Validation,
        &checks,
        MONTE_CARLO_REPLICATES,
        0x4449_5649_4445_4453,
    )?;

    println!(
        "record_type,statistic_id,unit_id,tail,predictive_p_value,randomized_lower_rank,observed_value,sample_size,predictive_draws,model_digest,draw_set_digest,source_digest,observation_digest,calibration_digest,split_manifest_digest,randomization_seed,ks_statistic,uniformity_p_value,unit_count,sample_size_across_units,mc_replicates,mc_seed,independent_unit_kind"
    );
    for check in &checks {
        emit(single_row(check));
    }
    emit(divided_row(&divided, SOURCE_DIGEST));

    Ok(())
}
