// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::model_adequacy::IndependentUnitKind;
use civdyn_self_correction::source_passport::{
    AcquisitionStatus, CompatibilityReport, CompatibilityStatus, GeographySupport,
    JoinAlignmentPlan, ObservationOrigin, ObservationPassport, RegisteredPurpose,
    RightsStatus, SourceArtifactPassport, TemporalResolution, TemporalSupport, qualify_source,
    TransformKind, TransformReceipt, assess_observation_compatibility,
};

const CSV_COLUMNS: usize = 14;

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
        "source passport receipt must match the frozen schema"
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

fn period(start: i64, end: i64, basis: &str, resolution: TemporalResolution) -> TemporalSupport {
    TemporalSupport {
        start_inclusive: start,
        end_exclusive: end,
        time_basis_id: basis.to_string(),
        resolution,
    }
}

fn geography(id: &str, kind: &str, version: &str, digest: &str) -> GeographySupport {
    GeographySupport {
        geography_id: id.to_string(),
        geography_kind: kind.to_string(),
        boundary_version: version.to_string(),
        boundary_digest: digest.to_string(),
    }
}

fn passport(
    id: &str,
    digest: &str,
    rights: RightsStatus,
    acquisition: AcquisitionStatus,
    source_unit: &str,
    native_unit: &str,
    period: TemporalSupport,
    geography: GeographySupport,
) -> SourceArtifactPassport {
    let acquired = acquisition == AcquisitionStatus::Acquired;
    SourceArtifactPassport {
        artifact_id: id.to_string(),
        publisher: format!("publisher-{id}"),
        dataset_title: format!("dataset-{id}"),
        dataset_version: "2026-v1".to_string(),
        source_family: format!("source-family-{id}"),
        landing_page: format!("https://example.invalid/{id}"),
        acquisition_locator: format!("source://{id}"),
        acquisition_status: acquisition,
        retrieved_at_epoch: acquired.then_some(1_800_000_000),
        artifact_digest: acquired.then(|| digest.to_string()),
        transformation_digest: acquired.then(|| format!("sha256-transform-{id}")),
        terms_digest: acquired.then(|| format!("sha256-terms-{id}")),
        rights_status: rights,
        permitted_purposes: vec![
            RegisteredPurpose::ScenarioAssumption.name().to_string(),
            RegisteredPurpose::EmpiricalValidation.name().to_string(),
        ],
        restriction_receipt_digest: Some(format!("sha256-restriction-{id}")),
        source_unit: source_unit.to_string(),
        native_measurement_unit: native_unit.to_string(),
        temporal_support: period,
        geography,
        sampling_unit: IndependentUnitKind::Household,
        sample_design_digest: acquired.then(|| format!("sha256-design-{id}")),
        weights_digest: acquired.then(|| format!("sha256-weights-{id}")),
        release_status: "synthetic-source-snapshot".to_string(),
    }
}

fn observation(
    id: &str,
    artifact_id: &str,
    artifact_digest: &str,
    origin: ObservationOrigin,
    temporal_support: TemporalSupport,
    geography: GeographySupport,
    unit: &str,
) -> ObservationPassport {
    ObservationPassport {
        observation_id: id.to_string(),
        artifact_id: artifact_id.to_string(),
        artifact_digest: artifact_digest.to_string(),
        variable_id: "essential-service-access".to_string(),
        construct: "household essential service access".to_string(),
        measurement_unit: unit.to_string(),
        origin,
        source_method_id: format!("method-{id}"),
        temporal_support,
        geography,
        analytical_unit: IndependentUnitKind::Region,
        sampling_unit: IndependentUnitKind::Household,
        sample_size: Some(500),
        sample_design_digest: Some(format!("sha256-design-{artifact_id}")),
        weights_digest: Some(format!("sha256-weights-{artifact_id}")),
        parent_observation_digests: Vec::new(),
        transformation_digest: None,
    }
}

fn plan(
    target_period: TemporalSupport,
    target_geography: GeographySupport,
    target_unit: &str,
) -> JoinAlignmentPlan {
    JoinAlignmentPlan {
        target_period,
        target_geography,
        target_measurement_unit: target_unit.to_string(),
        target_analytical_unit: IndependentUnitKind::Region,
        minimum_spatial_coverage: 0.95,
        transforms: Vec::new(),
    }
}

fn emit_report(
    scenario: &str,
    report: &CompatibilityReport,
    left_source: &SourceArtifactPassport,
    right_source: &SourceArtifactPassport,
) {
    let left_qualification = qualify_source(left_source, report.purpose).status.name();
    let right_qualification = qualify_source(right_source, report.purpose).status.name();
    emit(vec![
        scenario.to_string(),
        report.status.name().to_string(),
        format!("{:?}", report.reasons)
            .replace('[', "")
            .replace(']', "")
            .replace('"', "")
            .replace(", ", ";"),
        report.purpose.name().to_string(),
        report.left_observation_id.clone(),
        report.right_observation_id.clone(),
        report.left_artifact_digest.clone(),
        report.right_artifact_digest.clone(),
        left_qualification.to_string(),
        right_qualification.to_string(),
        report.explicit_transform_digests.join(";"),
        report.reasons.contains(&civdyn_self_correction::source_passport::CompatibilityReason::CategoryDefaultNotEligibleForEmpiricalValidation).to_string(),
        report.reasons.contains(&civdyn_self_correction::source_passport::CompatibilityReason::TemporalSupportMismatch).to_string(),
        report.reasons.contains(&civdyn_self_correction::source_passport::CompatibilityReason::GeographyMismatch).to_string(),
    ]);
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "scenario,compatibility_status,reasons,purpose,left_observation_id,right_observation_id,left_artifact_digest,right_artifact_digest,left_source_qualification,right_source_qualification,explicit_transform_digests,category_default_rejected,temporal_mismatch,geography_mismatch"
    );

    let year = period(2025, 2026, "calendar-year", TemporalResolution::CalendarYear);
    let province = geography("province-a", "province", "MDB-2026", "sha256-mdb-2026");

    // GHS and a second household survey: identical provincial/year construct,
    // native unit and geography. Survey design and weights remain explicit.
    let ghs = passport(
        "ghs-2025",
        "sha256-ghs-2025",
        RightsStatus::Permitted,
        AcquisitionStatus::Acquired,
        "households",
        "percent",
        year.clone(),
        province.clone(),
    );
    let other_survey = passport(
        "other-survey-2025",
        "sha256-other-survey",
        RightsStatus::Permitted,
        AcquisitionStatus::Acquired,
        "households",
        "percent",
        year.clone(),
        province.clone(),
    );
    let ghs_obs = observation(
        "ghs-provincial-access",
        "ghs-2025",
        "sha256-ghs-2025",
        ObservationOrigin::SurveyEstimate,
        year.clone(),
        province.clone(),
        "percent",
    );
    let other_obs = observation(
        "other-provincial-access",
        "other-survey-2025",
        "sha256-other-survey",
        ObservationOrigin::SurveyEstimate,
        year.clone(),
        province.clone(),
        "percent",
    );
    let report = assess_observation_compatibility(
        &ghs_obs,
        &ghs,
        &other_obs,
        &other_survey,
        RegisteredPurpose::EmpiricalValidation,
        &plan(year.clone(), province.clone(), "percent"),
    );
    emit_report("matched_provincial_survey", &report, &ghs, &other_survey);

    // Treasury quarterly finance and DWS daily NRW cannot be treated as if
    // they were already aligned to an annual municipal service state.
    let treasury_period = period(2025, 2026, "financial-year", TemporalResolution::Quarter);
    let dws_period = period(10, 11, "calendar-day-index", TemporalResolution::Day);
    let treasury = passport(
        "treasury-q2",
        "sha256-treasury-q2",
        RightsStatus::Permitted,
        AcquisitionStatus::Acquired,
        "municipal-submissions",
        "percent",
        treasury_period.clone(),
        province.clone(),
    );
    let dws = passport(
        "dws-daily",
        "sha256-dws-daily",
        RightsStatus::Permitted,
        AcquisitionStatus::Acquired,
        "water-service-authority",
        "percent",
        dws_period.clone(),
        province.clone(),
    );
    let treasury_obs = observation(
        "treasury-quarterly-maintenance",
        "treasury-q2",
        "sha256-treasury-q2",
        ObservationOrigin::AdministrativeSubmission,
        treasury_period,
        province.clone(),
        "percent",
    );
    let dws_obs = observation(
        "dws-daily-nrw",
        "dws-daily",
        "sha256-dws-daily",
        ObservationOrigin::AdministrativeSubmission,
        dws_period,
        province.clone(),
        "percent",
    );
    let report = assess_observation_compatibility(
        &treasury_obs,
        &treasury,
        &dws_obs,
        &dws,
        RegisteredPurpose::EmpiricalValidation,
        &plan(year.clone(), province.clone(), "percent"),
    );
    emit_report("treasury_dws_temporal_mismatch", &report, &treasury, &dws);

    // DWS category defaults may be retained as scenario assumptions but are
    // not direct empirical observations for measured-validation scoring.
    let dws_default = passport(
        "dws-category-default",
        "sha256-dws-default",
        RightsStatus::Permitted,
        AcquisitionStatus::Acquired,
        "municipal-category-default",
        "percent",
        year.clone(),
        province.clone(),
    );
    let admin_source = passport(
        "admin-service-report",
        "sha256-admin",
        RightsStatus::Permitted,
        AcquisitionStatus::Acquired,
        "service-authority",
        "percent",
        year.clone(),
        province.clone(),
    );
    let default_obs = observation(
        "dws-default-value",
        "dws-category-default",
        "sha256-dws-default",
        ObservationOrigin::CategoryDefault,
        year.clone(),
        province.clone(),
        "percent",
    );
    let admin_obs = observation(
        "admin-measured-value",
        "admin-service-report",
        "sha256-admin",
        ObservationOrigin::AdministrativeSubmission,
        year.clone(),
        province.clone(),
        "percent",
    );
    let scenario_report = assess_observation_compatibility(
        &default_obs,
        &dws_default,
        &admin_obs,
        &admin_source,
        RegisteredPurpose::ScenarioAssumption,
        &plan(year.clone(), province.clone(), "percent"),
    );
    emit_report("dws_default_scenario_assumption", &scenario_report, &dws_default, &admin_source);
    let measured_report = assess_observation_compatibility(
        &default_obs,
        &dws_default,
        &admin_obs,
        &admin_source,
        RegisteredPurpose::EmpiricalValidation,
        &plan(year.clone(), province.clone(), "percent"),
    );
    emit_report("dws_default_measured_validation", &measured_report, &dws_default, &admin_source);

    // Eskom has a published request process, but no dataset has yet been
    // acquired into this synthetic fixture. It cannot be empirically scored.
    let eskom_pending = passport(
        "eskom-hourly-request",
        "not-yet-acquired",
        RightsStatus::Permitted,
        AcquisitionStatus::AcquisitionPending,
        "national-hour",
        "MW",
        period(1, 2, "UTC-hour", TemporalResolution::Hour),
        geography("south-africa", "country", "country-boundary-v1", "sha256-country-boundary"),
    );
    let other_grid = passport(
        "other-grid-source",
        "sha256-other-grid",
        RightsStatus::Permitted,
        AcquisitionStatus::Acquired,
        "national-hour",
        "MW",
        period(1, 2, "UTC-hour", TemporalResolution::Hour),
        geography("south-africa", "country", "country-boundary-v1", "sha256-country-boundary"),
    );
    let eskom_obs = observation(
        "eskom-hourly-demand",
        "eskom-hourly-request",
        "not-yet-acquired",
        ObservationOrigin::DirectMeasurement,
        period(1, 2, "UTC-hour", TemporalResolution::Hour),
        geography("south-africa", "country", "country-boundary-v1", "sha256-country-boundary"),
        "MW",
    );
    let other_grid_obs = observation(
        "other-grid-demand",
        "other-grid-source",
        "sha256-other-grid",
        ObservationOrigin::DirectMeasurement,
        period(1, 2, "UTC-hour", TemporalResolution::Hour),
        geography("south-africa", "country", "country-boundary-v1", "sha256-country-boundary"),
        "MW",
    );
    let report = assess_observation_compatibility(
        &eskom_obs,
        &eskom_pending,
        &other_grid_obs,
        &other_grid,
        RegisteredPurpose::EmpiricalValidation,
        &plan(
            period(1, 2, "UTC-hour", TemporalResolution::Hour),
            geography("south-africa", "country", "country-boundary-v1", "sha256-country-boundary"),
            "MW",
        ),
    );
    emit_report("eskom_acquisition_pending", &report, &eskom_pending, &other_grid);

    // Boundary changes require an explicit, source-bound MDB crosswalk.
    let old_geo = geography("municipality-001", "municipality", "MDB-2021", "sha256-mdb-2021");
    let new_geo = geography("municipality-001", "municipality", "MDB-2026", "sha256-mdb-2026");
    let old_muni = passport(
        "old-muni-data",
        "sha256-old-muni",
        RightsStatus::Permitted,
        AcquisitionStatus::Acquired,
        "municipality",
        "percent",
        year.clone(),
        old_geo.clone(),
    );
    let new_muni = passport(
        "new-muni-data",
        "sha256-new-muni",
        RightsStatus::Permitted,
        AcquisitionStatus::Acquired,
        "municipality",
        "percent",
        year.clone(),
        new_geo.clone(),
    );
    let old_obs = observation(
        "old-boundary-observation",
        "old-muni-data",
        "sha256-old-muni",
        ObservationOrigin::AdministrativeSubmission,
        year.clone(),
        old_geo,
        "percent",
    );
    let new_obs = observation(
        "new-boundary-observation",
        "new-muni-data",
        "sha256-new-muni",
        ObservationOrigin::AdministrativeSubmission,
        year.clone(),
        new_geo.clone(),
        "percent",
    );
    let mut crosswalk_plan = plan(year.clone(), new_geo.clone(), "percent");
    crosswalk_plan.transforms.push(TransformReceipt {
        transform_id: "mdb-2021-to-2026".to_string(),
        transform_kind: TransformKind::SpatialCrosswalk,
        input_artifact_digest: "sha256-old-muni".to_string(),
        method_id: "mdb-official-crosswalk".to_string(),
        receipt_digest: "sha256-mdb-crosswalk-receipt".to_string(),
        target_period: None,
        target_geography: Some(new_geo),
        target_measurement_unit: None,
        target_analytical_unit: None,
        coverage_fraction: Some(0.98),
    });
    let report = assess_observation_compatibility(
        &old_obs,
        &old_muni,
        &new_obs,
        &new_muni,
        RegisteredPurpose::EmpiricalValidation,
        &crosswalk_plan,
    );
    emit_report("municipal_boundary_crosswalk", &report, &old_muni, &new_muni);

    // Unknown rights stop the join even when every statistical field matches.
    let unknown_rights = passport(
        "unknown-rights",
        "sha256-unknown-rights",
        RightsStatus::Unknown,
        AcquisitionStatus::Acquired,
        "households",
        "percent",
        year.clone(),
        province.clone(),
    );
    let unknown_obs = observation(
        "unknown-rights-observation",
        "unknown-rights",
        "sha256-unknown-rights",
        ObservationOrigin::SurveyEstimate,
        year.clone(),
        province.clone(),
        "percent",
    );
    let report = assess_observation_compatibility(
        &unknown_obs,
        &unknown_rights,
        &other_obs,
        &other_survey,
        RegisteredPurpose::EmpiricalValidation,
        &plan(year, province, "percent"),
    );
    emit_report("unknown_rights_blocked", &report, &unknown_rights, &other_survey);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run()
}
