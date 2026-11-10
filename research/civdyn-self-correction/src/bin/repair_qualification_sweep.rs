// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::model_adequacy::{
    assess_predictive_partition, IndependentUnitKind, ObservedStatistic, PredictiveDraw,
    PredictiveKind, SplitManifest, SplitRole, StatisticRegistry, StatisticSpec,
};
use civdyn_self_correction::repair_qualification::{
    qualify_repair, RepairEvidence, RepairQualificationInput, RepairQualificationReport,
};

const INITIAL_MODEL: &str = "sha256-repair-sweep-initial-model";
const REPAIRED_MODEL: &str = "sha256-repair-sweep-repaired-model";
const INITIAL_TRAINING: &str = "sha256-repair-sweep-initial-training";
const REPAIR_TRAINING: &str = "sha256-repair-sweep-repair-training";
const SOURCE_DIGEST: &str = "sha256-repair-sweep-challenge-source";
const OBSERVATION_DIGEST: &str = "sha256-repair-sweep-challenge-observation";
const CSV_COLUMNS: usize = 27;
const DRAWS: [f64; 5] = [0.45, 0.48, 0.50, 0.52, 0.55];

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

fn evidence(
    domain_id: &str,
    role: SplitRole,
    model_digest: &str,
    calibration_digest: &str,
    source_digest: &str,
    observation_digest: &str,
    observed_value: f64,
    draws: &[f64],
) -> Result<RepairEvidence, Box<dyn std::error::Error>> {
    let independent_unit = IndependentUnitKind::Region;
    let unit_id = match role {
        SplitRole::Validation => "development-region",
        SplitRole::Challenge => "sealed-challenge-region",
        SplitRole::Calibration => return Err("calibration role is not evaluated".into()),
    };
    let registry = StatisticRegistry {
        version: "repair-sweep-registry-v1".to_string(),
        statistics: vec![StatisticSpec {
            id: "essential-service-coverage".to_string(),
            construct: "essential household service coverage".to_string(),
            measurement_unit: "coverage_fraction".to_string(),
            population: "synthetic-households".to_string(),
            context: "paired-fresh-challenge-repair".to_string(),
            source_family: "synthetic-repair-sweep".to_string(),
            source_digest: source_digest.to_string(),
            independent_unit,
            role,
            primary: true,
            required: true,
            minimum_draws: draws.len(),
        }],
    };
    let manifest = SplitManifest {
        id: format!("{domain_id}-split-v1"),
        independent_unit,
        calibration_data_digest: calibration_digest.to_string(),
        manifest_digest: format!("sha256-{domain_id}-manifest"),
        calibration_units: vec![format!("{domain_id}-calibration-reserved")],
        validation_units: vec![if role == SplitRole::Validation {
            unit_id.to_string()
        } else {
            format!("{domain_id}-validation-reserved")
        }],
        challenge_units: if role == SplitRole::Challenge {
            vec![unit_id.to_string()]
        } else {
            Vec::new()
        },
    };
    let observation = ObservedStatistic {
        statistic_id: "essential-service-coverage".to_string(),
        independent_unit_id: unit_id.to_string(),
        measurement_unit: "coverage_fraction".to_string(),
        source_digest: source_digest.to_string(),
        sample_size: 100,
        value: observed_value,
        measurement_standard_error: Some(0.01),
        observation_digest: observation_digest.to_string(),
    };
    let predictive_draws: Vec<_> = draws
        .iter()
        .enumerate()
        .map(|(index, value)| PredictiveDraw {
            kind: PredictiveKind::Split,
            statistic_id: "essential-service-coverage".to_string(),
            independent_unit_id: unit_id.to_string(),
            sample_size: 100,
            draw_index: index as u64,
            value: *value,
            model_digest: model_digest.to_string(),
            draw_set_digest: format!("sha256-{domain_id}-draw-set"),
            calibration_data_digest: Some(calibration_digest.to_string()),
        })
        .collect();
    let assessment = assess_predictive_partition(
        &registry,
        &manifest,
        &[observation],
        &predictive_draws,
        role,
        PredictiveKind::Split,
        0.9,
    )?;

    Ok(RepairEvidence {
        domain_id: domain_id.to_string(),
        model_digest: model_digest.to_string(),
        registry,
        manifest,
        assessment,
    })
}

fn run_scenario(
    scenario: &str,
    baseline_draws: &[f64],
    repaired_draws: &[f64],
    challenge_observed: f64,
) -> Result<RepairQualificationReport, Box<dyn std::error::Error>> {
    let diagnostic = evidence(
        &format!("{scenario}-diagnostic"),
        SplitRole::Validation,
        INITIAL_MODEL,
        INITIAL_TRAINING,
        "sha256-repair-sweep-diagnostic-source",
        "sha256-repair-sweep-diagnostic-observation",
        0.9,
        &DRAWS,
    )?;
    let baseline = evidence(
        &format!("{scenario}-baseline"),
        SplitRole::Challenge,
        INITIAL_MODEL,
        INITIAL_TRAINING,
        SOURCE_DIGEST,
        OBSERVATION_DIGEST,
        challenge_observed,
        baseline_draws,
    )?;
    let repaired = evidence(
        &format!("{scenario}-repaired"),
        SplitRole::Challenge,
        REPAIRED_MODEL,
        REPAIR_TRAINING,
        SOURCE_DIGEST,
        OBSERVATION_DIGEST,
        challenge_observed,
        repaired_draws,
    )?;

    let training_sources = vec!["sha256-repair-sweep-diagnostic-source".to_string()];
    let training_observations =
        vec!["sha256-repair-sweep-diagnostic-observation".to_string()];
    Ok(qualify_repair(&RepairQualificationInput {
        diagnostic: &diagnostic,
        baseline_challenge: &baseline,
        repaired_challenge: &repaired,
        initial_calibration_data_digest: INITIAL_TRAINING,
        repair_training_data_digest: REPAIR_TRAINING,
        repair_training_source_digests: &training_sources,
        repair_training_observation_digests: &training_observations,
        repair_action_digest: "sha256-repair-action-known-answer",
        protocol_digest: "sha256-repair-protocol-v0.1",
        maximum_interval_width_ratio: 1.0,
    })?)
}

fn emit_report(scenario: &str, report: &RepairQualificationReport) {
    for result in &report.results {
        emit(vec![
            scenario.to_string(),
            report.claim_ceiling.name().to_string(),
            report.diagnostic_primary_discrepancies.to_string(),
            report.corrected_primary_statistics.to_string(),
            report.residual_primary_discrepancies.to_string(),
            report.regressed_primary_statistics.to_string(),
            report.width_budget_failures.to_string(),
            report.required_evidence_complete.to_string(),
            report.initial_model_digest.clone(),
            report.repaired_model_digest.clone(),
            report.initial_calibration_data_digest.clone(),
            report.repair_training_data_digest.clone(),
            report.diagnostic_split_manifest_digest.clone(),
            report.baseline_challenge_manifest_digest.clone(),
            report.repaired_challenge_manifest_digest.clone(),
            report.repair_action_digest.clone(),
            report.protocol_digest.clone(),
            format!("{:.6}", report.central_mass),
            format!("{:.6}", report.maximum_interval_width_ratio),
            result.statistic_id.clone(),
            result.independent_unit_id.clone(),
            result.baseline_status.name().to_string(),
            result.repaired_status.name().to_string(),
            result.baseline_interval_width.map(|value| format!("{value:.8}")).unwrap_or_default(),
            result.repaired_interval_width.map(|value| format!("{value:.8}")).unwrap_or_default(),
            result.interval_width_ratio.map(|value| format!("{value:.8}")).unwrap_or_default(),
            result.outcome.name().to_string(),
        ]);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "scenario,claim_ceiling,diagnostic_primary_discrepancies,corrected_primary_statistics,residual_primary_discrepancies,regressed_primary_statistics,width_budget_failures,required_evidence_complete,initial_model_digest,repaired_model_digest,initial_calibration_data_digest,repair_training_data_digest,diagnostic_split_manifest_digest,baseline_challenge_manifest_digest,repaired_challenge_manifest_digest,repair_action_digest,protocol_digest,central_mass,maximum_interval_width_ratio,statistic_id,independent_unit_id,baseline_status,repaired_status,baseline_interval_width,repaired_interval_width,interval_width_ratio,row_outcome"
    );

    let corrected = run_scenario(
        "corrected",
        &DRAWS,
        &[0.85, 0.88, 0.90, 0.92, 0.95],
        0.9,
    )?;
    emit_report("corrected", &corrected);

    let residual = run_scenario(
        "residual",
        &DRAWS,
        &[0.50, 0.52, 0.54, 0.56, 0.58],
        0.9,
    )?;
    emit_report("residual", &residual);

    let width_exceeded = run_scenario(
        "width_exceeded",
        &DRAWS,
        &[0.30, 0.50, 0.70, 0.90, 1.10],
        0.9,
    )?;
    emit_report("width_exceeded", &width_exceeded);

    let regression = run_scenario(
        &[0.45, 0.48, 0.50, 0.52, 0.55],
        &[0.90, 0.92, 0.94, 0.96, 0.98],
        0.5,
    )?;
    emit_report("regression", &regression);
    Ok(())
}
