// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::model_adequacy::{
    assess_predictive_partition, IndependentUnitKind, ObservedStatistic, PredictiveDraw,
    PredictiveKind, SplitManifest, SplitRole, StatisticRegistry, StatisticSpec,
};
use civdyn_self_correction::transfer_adequacy::{
    assess_zero_shot_transfer, TransferAxis, TransferAssessment, TransferDomainEvidence,
};

const MODEL_DIGEST: &str = "sha256-frozen-civdyn-transfer-model";
const CALIBRATION_DIGEST: &str = "sha256-frozen-civdyn-transfer-calibration";
const DRAW_VALUES: [f64; 7] = [0.44, 0.47, 0.49, 0.50, 0.51, 0.53, 0.56];
const CSV_COLUMNS: usize = 31;

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
        "transfer adequacy receipt must match the frozen schema"
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

fn build_domain(
    domain_id: &str,
    axis: TransferAxis,
    role: SplitRole,
    independent_unit: IndependentUnitKind,
    population: &str,
    context: &str,
    source_family: &str,
    source_digest: &str,
    unit_id: &str,
    observed_value: f64,
) -> Result<TransferDomainEvidence, Box<dyn std::error::Error>> {
    let registry = StatisticRegistry {
        version: "civdyn-transfer-registry-v0.1".to_string(),
        statistics: vec![StatisticSpec {
            id: "essential-service-coverage".to_string(),
            construct: "essential household service coverage".to_string(),
            measurement_unit: "coverage_fraction".to_string(),
            population: population.to_string(),
            context: context.to_string(),
            source_family: source_family.to_string(),
            source_digest: source_digest.to_string(),
            independent_unit,
            role,
            primary: true,
            required: true,
            minimum_draws: DRAW_VALUES.len(),
        }],
    };

    // SplitManifest requires a calibration and validation partition even when
    // this domain is a Challenge evaluation. The challenge unit is disjoint
    // from those reserved identities and is the only unit scored as transfer.
    let manifest = SplitManifest {
        id: format!("{domain_id}-split-v0.1"),
        independent_unit,
        calibration_data_digest: CALIBRATION_DIGEST.to_string(),
        manifest_digest: format!("sha256-{domain_id}-split-manifest"),
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
        observation_digest: format!("sha256-{domain_id}-observation"),
    };

    let draws: Vec<PredictiveDraw> = DRAW_VALUES
        .iter()
        .enumerate()
        .map(|(index, value)| PredictiveDraw {
            kind: PredictiveKind::Split,
            statistic_id: "essential-service-coverage".to_string(),
            independent_unit_id: unit_id.to_string(),
            sample_size: 100,
            draw_index: index as u64,
            value: *value,
            model_digest: MODEL_DIGEST.to_string(),
            draw_set_digest: format!("sha256-{domain_id}-draw-set"),
            calibration_data_digest: Some(CALIBRATION_DIGEST.to_string()),
        })
        .collect();

    let assessment = assess_predictive_partition(
        &registry,
        &manifest,
        &[observation],
        &draws,
        role,
        PredictiveKind::Split,
        0.9,
    )?;

    Ok(TransferDomainEvidence {
        domain_id: domain_id.to_string(),
        model_digest: MODEL_DIGEST.to_string(),
        axis,
        registry,
        manifest,
        assessment,
    })
}

fn fixture() -> Result<Vec<TransferDomainEvidence>, Box<dyn std::error::Error>> {
    Ok(vec![
        build_domain(
            "source-region",
            TransferAxis::Region,
            SplitRole::Validation,
            IndependentUnitKind::Region,
            "source-households",
            "source-region-baseline-period",
            "source-household-survey",
            "sha256-source-region-data",
            "region-source",
            0.50,
        )?,
        build_domain(
            "target-institution",
            TransferAxis::Institution,
            SplitRole::Challenge,
            IndependentUnitKind::Site,
            "target-households",
            "target-institution-current-period",
            "target-administrative-records",
            "sha256-target-institution-data",
            "institution-target",
            0.90,
        )?,
        build_domain(
            "target-time-period",
            TransferAxis::TimePeriod,
            SplitRole::Challenge,
            IndependentUnitKind::TimeBlock,
            "source-households",
            "future-time-block",
            "source-household-survey",
            "sha256-target-time-data",
            "time-block-target",
            0.50,
        )?,
    ])
}

fn emit_report(report: &TransferAssessment) {
    for domain in &report.domains {
        for statistic in &domain.statistics {
            emit(vec![
                domain.domain_id.clone(),
                domain.axis.name().to_string(),
                domain.role.name().to_string(),
                domain.independent_unit_kind.name().to_string(),
                domain.model_digest.clone(),
                domain.split_manifest_id.clone(),
                domain.split_manifest_digest.clone(),
                domain.status.name().to_string(),
                domain.required_evidence_complete.to_string(),
                statistic.statistic_id.clone(),
                statistic.construct.clone(),
                statistic.measurement_unit.clone(),
                statistic.population.clone(),
                statistic.context.clone(),
                statistic.source_family.clone(),
                statistic.source_digest.clone(),
                statistic.primary.to_string(),
                statistic.required.to_string(),
                statistic.expected_units.to_string(),
                statistic.assessed_units.to_string(),
                statistic.within_reference_units.to_string(),
                statistic.outside_reference_units.to_string(),
                statistic.not_assessed_units.to_string(),
                statistic.status.name().to_string(),
                report.calibration_data_digest.clone(),
                format!("{:.6}", report.central_mass),
                report.evidence_complete.to_string(),
                report.claim_ceiling.name().to_string(),
                report.registry_version.clone(),
                report.source_validation_domain_count.to_string(),
                report.held_out_challenge_domain_count.to_string(),
            ]);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let domains = fixture()?;
    let report = assess_zero_shot_transfer(&domains)?;

    println!(
        "domain_id,axis,role,independent_unit_kind,model_digest,split_manifest_id,split_manifest_digest,domain_status,required_evidence_complete,statistic_id,construct,measurement_unit,population,context,source_family,source_digest,primary,required,expected_units,assessed_units,within_reference_units,outside_reference_units,not_assessed_units,statistic_status,calibration_data_digest,central_mass,evidence_complete,claim_ceiling,registry_version,source_validation_domain_count,held_out_challenge_domain_count"
    );
    emit_report(&report);
    Ok(())
}
