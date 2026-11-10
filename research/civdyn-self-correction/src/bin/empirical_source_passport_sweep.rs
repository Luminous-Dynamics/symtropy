// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::empirical_source_passport::{
    AcquisitionState, EmpiricalSourceBundle, GeographicResolution, ObservationClass,
    RightsAuthorization, RightsStatus, RevisionStatus, SourceArtifact, SourceObservation,
    SourceSpecification, TemporalResolution, UsePurpose, qualify_source_bundle,
};

const CSV_COLUMNS: usize = 17;

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
        "empirical source passport receipt must match the frozen schema"
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

fn rights() -> RightsAuthorization {
    RightsAuthorization {
        status: RightsStatus::OpenPublic,
        policy_id: "fixture-open-license-v1".to_string(),
        permitted_purposes: vec![
            UsePurpose::Exploration,
            UsePurpose::Calibration,
            UsePurpose::Validation,
            UsePurpose::Challenge,
            UsePurpose::Publication,
        ],
        authorization_digest: Some("sha256-fixture-license".to_string()),
    }
}

fn specification(
    source_id: &str,
    construct_id: &str,
    unit: &str,
    geography: GeographicResolution,
    temporal_resolution: TemporalResolution,
) -> SourceSpecification {
    SourceSpecification {
        source_id: source_id.to_string(),
        specification_version: "fixture-spec-v1".to_string(),
        construct_id: construct_id.to_string(),
        construct_description: "synthetic source-passport conformance value".to_string(),
        native_measurement_unit: unit.to_string(),
        population: "synthetic-municipal-service-systems".to_string(),
        source_family: "synthetic-conformance-fixture".to_string(),
        geography,
        temporal_resolution,
        method_id: "synthetic-source-method-v1".to_string(),
    }
}

fn artifact(
    artifact_id: &str,
    dataset_version: &str,
    acquisition_state: AcquisitionState,
    revision_status: RevisionStatus,
) -> SourceArtifact {
    SourceArtifact {
        artifact_id: artifact_id.to_string(),
        dataset_version: dataset_version.to_string(),
        publisher: "synthetic conformance publisher".to_string(),
        landing_page: "https://example.invalid/synthetic-fixture".to_string(),
        retrieval_timestamp: "2026-11-10T12:00:00+02:00".to_string(),
        request_or_query_digest: Some("sha256-fixture-query".to_string()),
        artifact_digest: format!("sha256-{artifact_id}-artifact"),
        content_digest: format!("sha256-{artifact_id}-content"),
        supersedes_artifact_digest: None,
        acquisition_state,
        revision_status,
        rights: rights(),
    }
}

fn observation(
    source_id: &str,
    artifact_digest: &str,
    record_id: &str,
    class: ObservationClass,
    value: Option<f64>,
) -> SourceObservation {
    let method_or_default_rule_digest = match class {
        ObservationClass::Observed | ObservationClass::Estimated | ObservationClass::Imputed => {
            Some("sha256-fixture-measurement-method".to_string())
        }
        ObservationClass::Defaulted => Some("sha256-fixture-default-rule".to_string()),
        ObservationClass::Missing | ObservationClass::Suppressed => None,
    };
    let missing_or_suppression_reason = match class {
        ObservationClass::Missing => Some("not-reported".to_string()),
        ObservationClass::Suppressed => Some("privacy-threshold".to_string()),
        _ => None,
    };
    SourceObservation {
        record_id: record_id.to_string(),
        source_id: source_id.to_string(),
        artifact_digest: artifact_digest.to_string(),
        construct_id: "municipal-non-revenue-water".to_string(),
        geography_code: "ZA-SYNTHETIC-MUNICIPALITY".to_string(),
        boundary_version: "synthetic-demarcation-v1".to_string(),
        period_key: "2024".to_string(),
        measurement_unit: "fraction".to_string(),
        class,
        value,
        independent_unit_count: Some(1),
        measurement_standard_error: None,
        method_or_default_rule_digest,
        missing_or_suppression_reason,
    }
}

fn dws_like_bundle() -> EmpiricalSourceBundle {
    let source_id = "synthetic-water-balance-fixture";
    let artifact = artifact(
        source_id,
        "fixture-2024-v1",
        AcquisitionState::SemanticallyVerified,
        RevisionStatus::Final,
    );
    let artifact_digest = artifact.artifact_digest.clone();
    EmpiricalSourceBundle {
        specification: specification(
            source_id,
            "municipal-non-revenue-water",
            "fraction",
            GeographicResolution::Municipality,
            TemporalResolution::Annual,
        ),
        artifact,
        observations: vec![
            observation(source_id, &artifact_digest, "municipality-a", ObservationClass::Observed, Some(0.32)),
            observation(source_id, &artifact_digest, "municipality-b", ObservationClass::Estimated, Some(0.28)),
            observation(source_id, &artifact_digest, "municipality-c", ObservationClass::Defaulted, Some(0.25)),
            observation(source_id, &artifact_digest, "municipality-d", ObservationClass::Missing, None),
            observation(source_id, &artifact_digest, "municipality-e", ObservationClass::Suppressed, None),
        ],
    }
}

fn under_review_bundle() -> EmpiricalSourceBundle {
    let source_id = "synthetic-municipal-finance-q2";
    let artifact = artifact(
        source_id,
        "fixture-2026-q2-provisional",
        AcquisitionState::IntegrityVerified,
        RevisionStatus::UnderReview,
    );
    let artifact_digest = artifact.artifact_digest.clone();
    EmpiricalSourceBundle {
        specification: specification(
            source_id,
            "municipal-repairs-maintenance-spend",
            "zar_per_capita",
            GeographicResolution::Municipality,
            TemporalResolution::Quarterly,
        ),
        artifact,
        observations: vec![observation(
            source_id,
            &artifact_digest,
            "municipality-a-q2",
            ObservationClass::Observed,
            Some(128.0),
        )],
    }
}

fn not_yet_acquired_bundle() -> EmpiricalSourceBundle {
    let source_id = "synthetic-hourly-system-data-request";
    let artifact = artifact(
        source_id,
        "not-yet-retrieved",
        AcquisitionState::Requested,
        RevisionStatus::Unknown,
    );
    let artifact_digest = artifact.artifact_digest.clone();
    EmpiricalSourceBundle {
        specification: specification(
            source_id,
            "hourly-demand-available-capacity",
            "mwh_per_hour",
            GeographicResolution::National,
            TemporalResolution::Hourly,
        ),
        artifact,
        observations: vec![observation(
            source_id,
            &artifact_digest,
            "synthetic-request-placeholder",
            ObservationClass::Missing,
            None,
        )],
    }
}

fn emit_bundle(bundle: &EmpiricalSourceBundle, purpose: UsePurpose) {
    let result = qualify_source_bundle(bundle, purpose);
    match result {
        Ok(receipt) => {
            let counts = receipt.observation_counts;
            emit(vec![
                receipt.source_id,
                receipt.dataset_version,
                receipt.acquisition_state.name().to_string(),
                receipt.revision_status.name().to_string(),
                receipt.rights_status.name().to_string(),
                receipt.purpose.name().to_string(),
                "eligible_for_declared_purpose".to_string(),
                receipt.artifact_digest,
                receipt.content_digest,
                receipt.geography.name().to_string(),
                receipt.temporal_resolution.name().to_string(),
                counts.observed.to_string(),
                counts.estimated.to_string(),
                counts.defaulted.to_string(),
                counts.imputed.to_string(),
                counts.missing.to_string(),
                counts.suppressed.to_string(),
            ]);
        }
        Err(error) => emit(vec![
            bundle.specification.source_id.clone(),
            bundle.artifact.dataset_version.clone(),
            bundle.artifact.acquisition_state.name().to_string(),
            bundle.artifact.revision_status.name().to_string(),
            bundle.artifact.rights.status.name().to_string(),
            purpose.name().to_string(),
            format!("not_eligible:{error}"),
            bundle.artifact.artifact_digest.clone(),
            bundle.artifact.content_digest.clone(),
            bundle.specification.geography.name().to_string(),
            bundle.specification.temporal_resolution.name().to_string(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ]),
    }
}

fn main() {
    println!(
        "source_id,dataset_version,acquisition_state,revision_status,rights_status,purpose,eligibility,artifact_digest,content_digest,geographic_resolution,temporal_resolution,observed_count,estimated_count,defaulted_count,imputed_count,missing_count,suppressed_count"
    );

    // All three records are synthetic conformance fixtures. No official
    // South African raw dataset is bundled or claimed to have been acquired.
    emit_bundle(&dws_like_bundle(), UsePurpose::Validation);
    emit_bundle(&under_review_bundle(), UsePurpose::Exploration);
    emit_bundle(&not_yet_acquired_bundle(), UsePurpose::Validation);
}
