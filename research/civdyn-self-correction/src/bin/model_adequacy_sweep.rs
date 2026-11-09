// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::model_adequacy::{
    AdequacyAssessment, IndependentUnitKind, ObservedStatistic, PredictiveDraw, PredictiveKind,
    SplitManifest, SplitRole, StatisticRegistry, StatisticSpec, assess_predictive_partition,
};

const UNIT_ID: &str = "region-west";
const SAMPLE_SIZE: usize = 100;
const DRAW_COUNT: usize = 20;
const CALIBRATION_DIGEST: &str = "fixture-calibration-data-v1";

fn statistic(id: &str, construct: &str, unit: &str, source_family: &str) -> StatisticSpec {
    StatisticSpec {
        id: id.to_string(),
        construct: construct.to_string(),
        measurement_unit: unit.to_string(),
        population: "synthetic-district-population".to_string(),
        context: "frozen-known-answer-v0-1".to_string(),
        source_family: source_family.to_string(),
        source_digest: format!("fixture-source-{id}-v1"),
        independent_unit: IndependentUnitKind::Region,
        role: SplitRole::Validation,
        primary: true,
        required: true,
        minimum_draws: DRAW_COUNT,
    }
}

fn observed(statistic_id: &str, unit: &str, value: f64) -> ObservedStatistic {
    ObservedStatistic {
        statistic_id: statistic_id.to_string(),
        independent_unit_id: UNIT_ID.to_string(),
        measurement_unit: unit.to_string(),
        source_digest: format!("fixture-source-{statistic_id}-v1"),
        sample_size: SAMPLE_SIZE,
        value,
        measurement_standard_error: Some(0.01),
        observation_digest: format!("fixture-observation-{statistic_id}-v1"),
    }
}

fn predictive_draws(
    kind: PredictiveKind,
    statistic_id: &str,
    values: &[f64],
) -> Vec<PredictiveDraw> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| PredictiveDraw {
            kind,
            statistic_id: statistic_id.to_string(),
            independent_unit_id: UNIT_ID.to_string(),
            sample_size: SAMPLE_SIZE,
            draw_index: index as u64,
            value: *value,
            model_digest: "fixture-model-v1".to_string(),
            draw_set_digest: format!("fixture-draw-set-{}-{statistic_id}-v1", kind.name()),
            calibration_data_digest: match kind {
                PredictiveKind::Prior => None,
                PredictiveKind::Split => Some(CALIBRATION_DIGEST.to_string()),
            },
        })
        .collect()
}

fn linear_draws(start: f64, end: f64) -> Vec<f64> {
    (0..DRAW_COUNT)
        .map(|index| start + (end - start) * index as f64 / (DRAW_COUNT - 1) as f64)
        .collect()
}

fn render(kind: PredictiveKind, assessment: &AdequacyAssessment) {
    for result in &assessment.statistics {
        let coverage = assessment
            .coverage
            .iter()
            .find(|summary| summary.statistic_id == result.statistic_id);
        println!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            kind.name(),
            result.statistic_id,
            result.independent_unit_id,
            result.status.name(),
            result
                .observed_value
                .map(|value| format!("{value:.6}"))
                .unwrap_or_default(),
            result
                .measurement_standard_error
                .map(|value| format!("{value:.6}"))
                .unwrap_or_default(),
            result
                .lower_reference
                .map(|value| format!("{value:.6}"))
                .unwrap_or_default(),
            result
                .upper_reference
                .map(|value| format!("{value:.6}"))
                .unwrap_or_default(),
            result
                .predictive_rank
                .map(|value| format!("{value:.6}"))
                .unwrap_or_default(),
            result.draw_count,
            result.measurement_unit,
            assessment.domain_status.name(),
            result.source_digest,
            result.observation_digest.as_deref().unwrap_or_default(),
            result.model_digest.as_deref().unwrap_or_default(),
            result.draw_set_digest.as_deref().unwrap_or_default(),
            result
                .draw_dependence
                .map(|dependence| dependence.name())
                .unwrap_or_default(),
            result
                .single_spc_upper_tail_estimate
                .map(|value| format!("{value:.6}"))
                .unwrap_or_default(),
            result
                .single_spc_exceedance_count
                .map(|value| value.to_string())
                .unwrap_or_default(),
            result
                .single_spc_mc_wilson_lower_95
                .map(|value| format!("{value:.6}"))
                .unwrap_or_default(),
            result
                .single_spc_mc_wilson_upper_95
                .map(|value| format!("{value:.6}"))
                .unwrap_or_default(),
            coverage
                .map(|summary| summary.independent_unit.name())
                .unwrap_or_default(),
            coverage
                .map(|summary| summary.expected_independent_units.to_string())
                .unwrap_or_default(),
            coverage
                .map(|summary| summary.assessed_independent_units.to_string())
                .unwrap_or_default(),
            coverage
                .map(|summary| summary.covered_independent_units.to_string())
                .unwrap_or_default(),
            coverage
                .and_then(|summary| summary.empirical_coverage)
                .map(|value| format!("{value:.6}"))
                .unwrap_or_default(),
            coverage
                .and_then(|summary| summary.wilson_lower_95)
                .map(|value| format!("{value:.6}"))
                .unwrap_or_default(),
            coverage
                .and_then(|summary| summary.wilson_upper_95)
                .map(|value| format!("{value:.6}"))
                .unwrap_or_default(),
            assessment.required_evidence_complete,
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = StatisticRegistry {
        version: "civdyn-statistics-v0.1".to_string(),
        statistics: vec![
            statistic(
                "household-water-mean",
                "household water service mean",
                "m3_per_household_day",
                "synthetic-household-water-v1",
            ),
            statistic(
                "health-income-joint",
                "health-income dependence",
                "correlation",
                "synthetic-health-income-v1",
            ),
            statistic(
                "rare-service-failure-rate",
                "rare essential-service failure rate",
                "probability",
                "synthetic-service-tail-v1",
            ),
        ],
    };

    let manifest = SplitManifest {
        id: "civdyn-adequacy-fixture-split-v1".to_string(),
        independent_unit: IndependentUnitKind::Region,
        calibration_data_digest: CALIBRATION_DIGEST.to_string(),
        manifest_digest: "fixture-split-manifest-v1".to_string(),
        calibration_units: vec!["region-calibration".to_string()],
        validation_units: vec![UNIT_ID.to_string()],
        challenge_units: vec!["region-challenge".to_string()],
    };

    let observations = vec![
        observed("household-water-mean", "m3_per_household_day", 8.0),
        observed("health-income-joint", "correlation", 0.85),
        observed("rare-service-failure-rate", "probability", 0.04),
    ];

    let mut prior_draws = Vec::new();
    prior_draws.extend(predictive_draws(
        PredictiveKind::Prior,
        "household-water-mean",
        &linear_draws(7.7, 8.3),
    ));
    prior_draws.extend(predictive_draws(
        PredictiveKind::Prior,
        "health-income-joint",
        &linear_draws(0.20, 0.50),
    ));
    prior_draws.extend(predictive_draws(
        PredictiveKind::Prior,
        "rare-service-failure-rate",
        &linear_draws(0.01, 0.06),
    ));

    let mut split_draws = Vec::new();
    split_draws.extend(predictive_draws(
        PredictiveKind::Split,
        "household-water-mean",
        &linear_draws(7.8, 8.2),
    ));
    split_draws.extend(predictive_draws(
        PredictiveKind::Split,
        "health-income-joint",
        &linear_draws(0.15, 0.45),
    ));
    split_draws.extend(predictive_draws(
        PredictiveKind::Split,
        "rare-service-failure-rate",
        &linear_draws(0.005, 0.025),
    ));

    println!(
        "kind,statistic_id,unit_id,status,observed,measurement_se,lower,upper,rank,n_draws,measurement_unit,domain_status,source_digest,observation_digest,model_digest,draw_set_digest,draw_dependence,single_spc_upper_tail_estimate,single_spc_exceedance_count,single_spc_mc_wilson_lower_95,single_spc_mc_wilson_upper_95,independent_unit,expected_units,assessed_units,covered_units,empirical_coverage,wilson_lower_95,wilson_upper_95,required_evidence_complete"
    );

    let prior = assess_predictive_partition(
        &registry,
        &manifest,
        &observations,
        &prior_draws,
        SplitRole::Validation,
        PredictiveKind::Prior,
        0.9,
    )?;
    render(PredictiveKind::Prior, &prior);

    let split = assess_predictive_partition(
        &registry,
        &manifest,
        &observations,
        &split_draws,
        SplitRole::Validation,
        PredictiveKind::Split,
        0.9,
    )?;
    render(PredictiveKind::Split, &split);

    Ok(())
}
