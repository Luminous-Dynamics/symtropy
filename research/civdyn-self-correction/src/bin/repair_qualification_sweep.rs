// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::model_adequacy::{
    assess_predictive_partition, IndependentUnitKind, ObservedStatistic, PredictiveDraw,
    PredictiveKind, SplitManifest, SplitRole, StatisticRegistry, StatisticSpec,
};
use civdyn_self_correction::repair_qualification::{
    qualify_repair_on_fresh_challenge, RepairQualificationInput, RepairQualificationReport,
};

const BASELINE_MODEL: &str = "sha256-civdyn-baseline-network-model";
const REPAIRED_MODEL: &str = "sha256-civdyn-repaired-network-model";
const CALIBRATION: &str = "sha256-civdyn-repair-calibration";
const SOURCE: &str = "sha256-civdyn-fresh-network-source";
const DRAW_COUNT: usize = 5;
const CSV_COLUMNS: usize = 27;

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
        "repair qualification receipt must match the frozen schema"
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

fn registry() -> StatisticRegistry {
    StatisticRegistry {
        version: "civdyn-repair-challenge-v0.1".to_string(),
        statistics: vec![StatisticSpec {
            id: "held-out-transitivity".to_string(),
            construct: "global network transitivity".to_string(),
            measurement_unit: "ratio".to_string(),
            population: "fresh-challenge-networks".to_string(),
            context: "statistic-held-out-network-repair".to_string(),
            source_family: "synthetic-network-challenge".to_string(),
            source_digest: SOURCE.to_string(),
            independent_unit: IndependentUnitKind::Network,
            role: SplitRole::Challenge,
            primary: true,
            required: true,
            minimum_draws: DRAW_COUNT,
        }],
    }
}

fn manifest() -> SplitManifest {
    SplitManifest {
        id: "civdyn-fresh-network-challenge-v1".to_string(),
        independent_unit: IndependentUnitKind::Network,
        calibration_data_digest: CALIBRATION.to_string(),
        manifest_digest: "sha256-civdyn-fresh-network-challenge-manifest".to_string(),
        calibration_units: vec!["calibration-network".to_string()],
        validation_units: vec!["reserved-validation-network".to_string()],
        challenge_units: vec!["network-a".to_string(), "network-b".to_string()],
    }
}

fn assessment(
    model_digest: &str,
    values_a: [f64; DRAW_COUNT],
    values_b: [f64; DRAW_COUNT],
) -> Result<civdyn_self_correction::model_adequacy::AdequacyAssessment, Box<dyn std::error::Error>> {
    let registry = registry();
    let manifest = manifest();
    let observations = vec![
        ObservedStatistic {
            statistic_id: "held-out-transitivity".to_string(),
            independent_unit_id: "network-a".to_string(),
            measurement_unit: "ratio".to_string(),
            source_digest: SOURCE.to_string(),
            sample_size: 1,
            value: 0.8,
            measurement_standard_error: Some(0.01),
            observation_digest: "sha256-civdyn-network-a-observation".to_string(),
        },
        ObservedStatistic {
            statistic_id: "held-out-transitivity".to_string(),
            independent_unit_id: "network-b".to_string(),
            measurement_unit: "ratio".to_string(),
            source_digest: SOURCE.to_string(),
            sample_size: 1,
            value: 0.2,
            measurement_standard_error: Some(0.01),
            observation_digest: "sha256-civdyn-network-b-observation".to_string(),
        },
    ];

    let draws = [("network-a", values_a), ("network-b", values_b)]
        .into_iter()
        .flat_map(|(unit_id, values)| {
            values.into_iter().enumerate().map(move |(draw_index, value)| PredictiveDraw {
                kind: PredictiveKind::Split,
                statistic_id: "held-out-transitivity".to_string(),
                independent_unit_id: unit_id.to_string(),
                sample_size: 1,
                draw_index: draw_index as u64,
                value,
                model_digest: model_digest.to_string(),
                draw_set_digest: format!("sha256-{model_digest}-{unit_id}-draws"),
                calibration_data_digest: Some(CALIBRATION.to_string()),
            })
        })
        .collect::<Vec<_>>();

    Ok(assess_predictive_partition(
        &registry,
        &manifest,
        &observations,
        &draws,
        SplitRole::Challenge,
        PredictiveKind::Split,
        0.9,
    )?)
}

fn repair_input(
    repair_plan_digest: &str,
    baseline: civdyn_self_correction::model_adequacy::AdequacyAssessment,
    repaired: civdyn_self_correction::model_adequacy::AdequacyAssessment,
) -> RepairQualificationInput {
    RepairQualificationInput {
        baseline_model_digest: BASELINE_MODEL.to_string(),
        repaired_model_digest: REPAIRED_MODEL.to_string(),
        repair_plan_digest: repair_plan_digest.to_string(),
        diagnostic_manifest_digest: "sha256-civdyn-diagnostic-manifest".to_string(),
        diagnostic_statistic_ids: vec![
            "calibration-water-mean".to_string(),
            "calibration-energy-service-rate".to_string(),
        ],
        diagnostic_observation_digests: vec![
            "sha256-civdyn-diagnostic-water-observation".to_string(),
            "sha256-civdyn-diagnostic-energy-observation".to_string(),
        ],
        challenge_registry: registry(),
        challenge_manifest: manifest(),
        baseline_assessment: baseline,
        repaired_assessment: repaired,
    }
}

fn emit_report(
    scenario: &str,
    report: &RepairQualificationReport,
) {
    for transition in &report.transitions {
        emit(vec![
            scenario.to_string(),
            transition.statistic_id.clone(),
            transition.independent_unit_id.clone(),
            transition.primary.to_string(),
            transition.required.to_string(),
            transition.baseline_status.name().to_string(),
            transition.repaired_status.name().to_string(),
            transition.transition.name().to_string(),
            transition.observation_digest.clone().unwrap_or_default(),
            transition.source_digest.clone(),
            transition.sample_size.to_string(),
            report.baseline_model_digest.clone(),
            report.repaired_model_digest.clone(),
            report.repair_plan_digest.clone(),
            report.diagnostic_manifest_digest.clone(),
            report.challenge_manifest_id.clone(),
            report.challenge_manifest_digest.clone(),
            report.calibration_data_digest.clone(),
            format!("{:.6}", report.central_mass),
            report.evidence_complete.to_string(),
            report.status.name().to_string(),
            report.primary_improvement_count.to_string(),
            report.primary_regression_count.to_string(),
            report.primary_persistent_discrepancy_count.to_string(),
            report.primary_baseline_discrepancy_count.to_string(),
            report.primary_repaired_discrepancy_count.to_string(),
            report.challenge_unit_count.to_string(),
        ]);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "scenario,statistic_id,independent_unit_id,primary,required,baseline_status,repaired_status,transition,observation_digest,source_digest,sample_size,baseline_model_digest,repaired_model_digest,repair_plan_digest,diagnostic_manifest_digest,challenge_manifest_id,challenge_manifest_digest,calibration_data_digest,central_mass,evidence_complete,overall_status,primary_improvement_count,primary_regression_count,primary_persistent_discrepancy_count,primary_baseline_discrepancy_count,primary_repaired_discrepancy_count,challenge_unit_count"
    );

    let baseline_failing = assessment(
        BASELINE_MODEL,
        [0.1, 0.2, 0.2, 0.3, 0.4],
        [0.6, 0.7, 0.75, 0.8, 0.9],
    )?;

    let partially_repaired = assessment(
        REPAIRED_MODEL,
        [0.7, 0.75, 0.8, 0.85, 0.9],
        [0.6, 0.7, 0.75, 0.8, 0.9],
    )?;
    let partial_report = qualify_repair_on_fresh_challenge(&repair_input(
        "sha256-repair-plan-partial",
        baseline_failing.clone(),
        partially_repaired,
    ))?;
    emit_report("improved_not_restored", &partial_report);

    let fully_repaired = assessment(
        REPAIRED_MODEL,
        [0.7, 0.75, 0.8, 0.85, 0.9],
        [0.1, 0.15, 0.2, 0.25, 0.3],
    )?;
    let restored_report = qualify_repair_on_fresh_challenge(&repair_input(
        "sha256-repair-plan-restored",
        baseline_failing,
        fully_repaired,
    ))?;
    emit_report("restored_on_fresh_challenge", &restored_report);

    let baseline_passing = assessment(
        BASELINE_MODEL,
        [0.7, 0.75, 0.8, 0.85, 0.9],
        [0.1, 0.15, 0.2, 0.25, 0.3],
    )?;
    let regressed = assessment(
        REPAIRED_MODEL,
        [0.1, 0.2, 0.2, 0.3, 0.4],
        [0.1, 0.15, 0.2, 0.25, 0.3],
    )?;
    let regression_report = qualify_repair_on_fresh_challenge(&repair_input(
        "sha256-repair-plan-regression",
        baseline_passing,
        regressed,
    ))?;
    emit_report("regression_detected", &regression_report);

    Ok(())
}
