// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::calibration_profile::{
    assess_calibration_profile, CalibrationProfileAssessment, CalibrationProfileEvidence,
};
use civdyn_self_correction::model_adequacy::{
    assess_predictive_partition, IndependentUnitKind, ObservedStatistic, PredictiveDraw,
    PredictiveKind, SplitManifest, SplitRole, StatisticRegistry, StatisticSpec,
};

const MODEL: &str = "sha256-calibration-profile-model";
const CALIBRATION: &str = "sha256-calibration-profile-calibration";
const MASSES: [f64; 4] = [0.50, 0.80, 0.90, 0.95];
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
        "calibration profile receipt must match the frozen schema"
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
        version: "calibration-profile-v1".to_string(),
        statistics: vec![
            StatisticSpec {
                id: "service-coverage-group-a".to_string(),
                construct: "essential service coverage group A".to_string(),
                measurement_unit: "coverage_fraction".to_string(),
                population: "group-a".to_string(),
                context: "synthetic-calibration-fixture".to_string(),
                source_family: "synthetic-service-measurements".to_string(),
                source_digest: "sha256-calibration-observations".to_string(),
                independent_unit: IndependentUnitKind::Region,
                role: SplitRole::Validation,
                primary: true,
                required: true,
                minimum_draws: 101,
            },
            StatisticSpec {
                id: "service-coverage-group-b".to_string(),
                construct: "essential service coverage group B".to_string(),
                measurement_unit: "coverage_fraction".to_string(),
                population: "group-b".to_string(),
                context: "synthetic-calibration-fixture".to_string(),
                source_family: "synthetic-service-measurements".to_string(),
                source_digest: "sha256-calibration-observations".to_string(),
                independent_unit: IndependentUnitKind::Region,
                role: SplitRole::Validation,
                primary: true,
                required: true,
                minimum_draws: 101,
            },
        ],
    }
}

fn manifest() -> SplitManifest {
    SplitManifest {
        id: "calibration-profile-split-v1".to_string(),
        independent_unit: IndependentUnitKind::Region,
        calibration_data_digest: CALIBRATION.to_string(),
        manifest_digest: "sha256-calibration-profile-manifest".to_string(),
        calibration_units: vec!["calibration-region".to_string()],
        validation_units: (0..200)
            .map(|index| format!("region-{index:03}"))
            .collect(),
        challenge_units: Vec::new(),
    }
}

fn group_a_ranks() -> Vec<f64> {
    let mut ranks = Vec::with_capacity(200);
    ranks.extend(std::iter::repeat_n(0.50, 80));
    ranks.extend(std::iter::repeat_n(0.15, 20));
    ranks.extend(std::iter::repeat_n(0.85, 20));
    ranks.extend(std::iter::repeat_n(0.075, 10));
    ranks.extend(std::iter::repeat_n(0.925, 10));
    ranks.extend(std::iter::repeat_n(0.0375, 10));
    ranks.extend(std::iter::repeat_n(0.9625, 10));
    ranks.extend(std::iter::repeat_n(0.01, 20));
    ranks.extend(std::iter::repeat_n(0.99, 20));
    assert_eq!(ranks.len(), 200);
    ranks
}

fn group_b_ranks() -> Vec<f64> {
    let mut ranks = Vec::with_capacity(200);
    ranks.extend(std::iter::repeat_n(0.50, 120));
    ranks.extend(std::iter::repeat_n(0.15, 30));
    ranks.extend(std::iter::repeat_n(0.85, 30));
    ranks.extend(std::iter::repeat_n(0.075, 8));
    ranks.extend(std::iter::repeat_n(0.925, 8));
    ranks.extend(std::iter::repeat_n(0.0375, 1));
    ranks.extend(std::iter::repeat_n(0.9625, 1));
    ranks.extend(std::iter::repeat_n(0.01, 1));
    ranks.extend(std::iter::repeat_n(0.99, 1));
    assert_eq!(ranks.len(), 200);
    ranks
}

fn evidence() -> CalibrationProfileEvidence {
    let registry = registry();
    let manifest = manifest();
    let ranks = [
        ("service-coverage-group-a", group_a_ranks()),
        ("service-coverage-group-b", group_b_ranks()),
    ];
    let mut observations = Vec::with_capacity(400);
    let mut draws = Vec::with_capacity(40_400);

    for (statistic_id, statistic_ranks) in &ranks {
        for (index, rank) in statistic_ranks.iter().enumerate() {
            let unit_id = format!("region-{index:03}");
            observations.push(ObservedStatistic {
                statistic_id: (*statistic_id).to_string(),
                independent_unit_id: unit_id.clone(),
                measurement_unit: "coverage_fraction".to_string(),
                source_digest: "sha256-calibration-observations".to_string(),
                sample_size: 101,
                value: rank * 100.0,
                measurement_standard_error: Some(0.01),
                observation_digest: format!("sha256-observation-{statistic_id}-{unit_id}"),
            });
            for draw_index in 0..=100_u64 {
                draws.push(PredictiveDraw {
                    kind: PredictiveKind::Split,
                    statistic_id: (*statistic_id).to_string(),
                    independent_unit_id: unit_id.clone(),
                    sample_size: 101,
                    draw_index,
                    value: draw_index as f64,
                    model_digest: MODEL.to_string(),
                    draw_set_digest: format!("sha256-draws-{statistic_id}-{unit_id}"),
                    calibration_data_digest: Some(CALIBRATION.to_string()),
                });
            }
        }
    }

    let assessments = MASSES
        .iter()
        .map(|central_mass| {
            assess_predictive_partition(
                &registry,
                &manifest,
                &observations,
                &draws,
                SplitRole::Validation,
                PredictiveKind::Split,
                *central_mass,
            )
            .expect("valid synthetic split-predictive calibration assessment")
        })
        .collect();

    CalibrationProfileEvidence {
        model_digest: MODEL.to_string(),
        registry,
        manifest,
        assessments,
    }
}

fn optional(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.8}"))
        .unwrap_or_default()
}

fn emit_report(report: &CalibrationProfileAssessment) {
    for point in &report.points {
        emit(vec![
            report.registry_version.clone(),
            report.model_digest.clone(),
            report.split_manifest_id.clone(),
            report.split_manifest_digest.clone(),
            report.calibration_data_digest.clone(),
            report.role.name().to_string(),
            report.predictive_kind.name().to_string(),
            report.independent_unit.name().to_string(),
            format!("{:.2}", point.central_mass),
            point.statistic_id.clone(),
            point.construct.clone(),
            point.measurement_unit.clone(),
            point.population.clone(),
            point.context.clone(),
            point.source_family.clone(),
            point.source_digest.clone(),
            point.primary.to_string(),
            point.required.to_string(),
            point.expected_independent_units.to_string(),
            point.assessed_independent_units.to_string(),
            point.covered_independent_units.to_string(),
            point.not_assessed_independent_units.to_string(),
            optional(point.empirical_coverage),
            optional(point.signed_calibration_gap),
            optional(point.wilson_lower_95),
            optional(point.wilson_upper_95),
            optional(point.mean_interval_width),
            point.status.name().to_string(),
            report.evidence_complete.to_string(),
            report.claim_ceiling.name().to_string(),
        ]);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let report = assess_calibration_profile(&evidence())?;
    println!(
        "registry_version,model_digest,split_manifest_id,split_manifest_digest,calibration_data_digest,role,predictive_kind,independent_unit,central_mass,statistic_id,construct,measurement_unit,population,context,source_family,source_digest,primary,required,expected_units,assessed_units,covered_units,not_assessed_units,empirical_coverage,signed_calibration_gap,wilson_lower_95,wilson_upper_95,mean_interval_width,point_status,evidence_complete,claim_ceiling"
    );
    emit_report(&report);
    Ok(())
}
