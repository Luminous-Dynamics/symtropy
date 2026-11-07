//! Traceable measurement-system record admission for excavation experiments.
//!
//! This crate validates instrument, calibration, domain, sample, timebase,
//! measurement-model and uncertainty-evidence records. It deliberately does
//! not calculate a combined uncertainty, calibrate instruments, certify a
//! laboratory or promote synthetic/online estimates into physical truth.
#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationReport {
    pub case_id: String,
    pub evidence_disposition: String,
    pub channel_count: usize,
    pub calibration_count: usize,
    pub measurement_count: usize,
    pub reference_comparison_count: usize,
    pub primary_reference_channel_count: usize,
    pub secondary_estimator_channel_count: usize,
    pub independent_validation_comparison_count: usize,
    pub out_of_domain_comparison_count: usize,
    pub combined_uncertainty_computed: bool,
    pub limitations: Vec<String>,
}

fn push(errors: &mut Vec<Violation>, code: &str, path: &str, detail: impl Into<String>) {
    errors.push(Violation {
        code: code.to_owned(),
        path: path.to_owned(),
        detail: detail.into(),
    });
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn finite(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|v| v.is_finite())
}

fn same_domain(a: &Value, b: &Value) -> bool {
    [
        "material_cohort_ref",
        "path_configuration_ref",
        "geometry_profile_ref",
        "environment_profile_ref",
    ]
    .iter()
    .all(|key| text(a, key).is_some() && text(a, key) == text(b, key))
}

fn role_matches_evidence(channel_role: &str, evidence_role: &str) -> bool {
    match channel_role {
        "PRIMARY_REFERENCE" => evidence_role == "INDEPENDENT_REFERENCE_OBSERVATION",
        "SECONDARY_ESTIMATOR" => evidence_role == "ONLINE_ESTIMATOR_OBSERVATION",
        "DIAGNOSTIC_SENSOR" => evidence_role == "DIAGNOSTIC_OBSERVATION",
        "PARTICLE_CHARACTERIZATION" => evidence_role == "PARTICLE_SAMPLE_RESULT",
        "TIMEBASE" => evidence_role == "TIMEBASE_OBSERVATION",
        _ => false,
    }
}

fn date_inclusive(value: &str, start: &str, end: &str) -> bool {
    // Dates are required to be ISO-8601 YYYY-MM-DD strings by the schema.
    // Lexical ordering is therefore chronological for this fixed-width form.
    start <= value && value <= end
}

fn validate_uncertainty(
    uncertainty: &Value,
    linearity: &str,
    path: &str,
    errors: &mut Vec<Violation>,
) {
    let method = text(uncertainty, "method").unwrap_or("");
    let Some(components) = uncertainty.get("components").and_then(Value::as_array) else {
        push(
            errors,
            "MISSING_UNCERTAINTY_COMPONENTS",
            &format!("{path}.components"),
            "uncertainty components must be an explicit array",
        );
        return;
    };
    if components.is_empty() {
        push(
            errors,
            "EMPTY_UNCERTAINTY_COMPONENTS",
            &format!("{path}.components"),
            "an evaluated uncertainty budget requires at least one component",
        );
    }

    if linearity == "NONLINEAR" && method == "LINEARIZED_GUM" {
        push(
            errors,
            "NONLINEAR_MODEL_WITH_LINEARIZED_ONLY_UQ",
            &format!("{path}.method"),
            "a nonlinear measurement model cannot be represented by linearized GUM alone; provide Monte Carlo propagation or an explicit conservative interval method",
        );
    }
    if linearity == "UNKNOWN" && method == "LINEARIZED_GUM" {
        push(
            errors,
            "UNKNOWN_LINEARTY_WITH_LINEARIZED_UQ",
            &format!("{path}.method"),
            "unknown model linearity cannot silently qualify a linearized uncertainty calculation",
        );
    }
    if linearity == "LINEAR" && method == "MONTE_CARLO" {
        // Allowed. The choice may be deliberate, conservative or needed for
        // non-Gaussian/correlated inputs; no unnecessary rejection.
    }

    if method == "MONTE_CARLO" {
        if text(uncertainty, "monte_carlo_result_ref").is_none()
            || uncertainty
                .get("monte_carlo_sample_count")
                .and_then(Value::as_u64)
                .is_none_or(|n| n == 0)
        {
            push(
                errors,
                "INCOMPLETE_MONTE_CARLO_EVIDENCE",
                path,
                "Monte Carlo uncertainty requires a result artifact and positive sample count",
            );
        }
    }
    if method == "LINEARIZED_GUM" {
        for (index, component) in components.iter().enumerate() {
            if finite(component, "sensitivity_coefficient").is_none() {
                push(
                    errors,
                    "MISSING_SENSITIVITY_COEFFICIENT",
                    &format!("{path}.components[{index}].sensitivity_coefficient"),
                    "linearized uncertainty component requires a finite sensitivity coefficient",
                );
            }
        }
    }
    if method == "NOT_EVALUATED" {
        push(
            errors,
            "UNCERTAINTY_NOT_EVALUATED",
            &format!("{path}.method"),
            "measurement is recorded but cannot be admitted as qualification evidence without an evaluated uncertainty method",
        );
    }
    if !matches!(
        method,
        "LINEARIZED_GUM" | "MONTE_CARLO" | "INTERVAL_BOUNDS_ONLY" | "NOT_EVALUATED"
    ) {
        push(
            errors,
            "UNKNOWN_UNCERTAINTY_METHOD",
            &format!("{path}.method"),
            "uncertainty method is outside v0.1 vocabulary",
        );
    }
    if method == "INTERVAL_BOUNDS_ONLY"
        && text(uncertainty, "component_set_ref").is_none()
    {
        push(
            errors,
            "MISSING_INTERVAL_BOUNDS_EVIDENCE",
            &format!("{path}.component_set_ref"),
            "interval-bounds uncertainty requires a versioned bound set",
        );
    }

    let mut component_ids = BTreeSet::new();
    let mut correlation_groups: BTreeMap<String, usize> = BTreeMap::new();
    for (index, component) in components.iter().enumerate() {
        let component_path = format!("{path}.components[{index}]");
        let Some(component_id) = text(component, "component_id") else {
            push(
                errors,
                "MISSING_UNCERTAINTY_COMPONENT_ID",
                &format!("{component_path}.component_id"),
                "component_id is required",
            );
            continue;
        };
        if !component_ids.insert(component_id.to_owned()) {
            push(
                errors,
                "DUPLICATE_UNCERTAINTY_COMPONENT_ID",
                &format!("{component_path}.component_id"),
                "uncertainty component IDs must be unique",
            );
        }
        match finite(component, "standard_uncertainty") {
            Some(u) if u >= 0.0 => {}
            _ => push(
                errors,
                "INVALID_STANDARD_UNCERTAINTY",
                &format!("{component_path}.standard_uncertainty"),
                "standard uncertainty must be finite and non-negative",
            ),
        }
        for key in ["unit", "distribution", "source_ref", "uncertainty_type"] {
            if text(component, key).is_none() {
                push(
                    errors,
                    "MISSING_UNCERTAINTY_COMPONENT_PROVENANCE",
                    &format!("{component_path}.{key}"),
                    "component type, unit, distribution and source reference are required",
                );
            }
        }
        if let Some(group) = text(component, "correlation_group_ref") {
            *correlation_groups.entry(group.to_owned()).or_default() += 1;
        }
    }

    let has_correlated_components = correlation_groups.values().any(|count| *count > 1);
    if has_correlated_components
        && (text(uncertainty, "correlation_evidence_ref").is_none()
            || text(uncertainty, "covariance_model_ref").is_none())
    {
        push(
            errors,
            "MISSING_CORRELATION_EVIDENCE",
            path,
            "components sharing a correlation group require explicit correlation evidence and a covariance-model reference",
        );
    }
    if text(uncertainty, "between_group_independence_ref").is_none() {
        push(
            errors,
            "MISSING_BETWEEN_GROUP_DEPENDENCE_ASSUMPTION",
            &format!("{path}.between_group_independence_ref"),
            "independence across distinct uncertainty groups must be an explicit reviewed assumption",
        );
    }
}

/// Validate traceability, calibration validity, measurement context and
/// uncertainty-evidence admission for a complete measurement-system case.
pub fn validate_case(case: &Value, parent: &Value) -> Vec<Violation> {
    let mut errors = Vec::new();

    if case.get("schema_version").and_then(Value::as_u64) != Some(1) {
        push(&mut errors, "UNSUPPORTED_SCHEMA_VERSION", "$.schema_version", "schema_version must be exactly 1");
    }
    if text(case, "case_id").is_none() {
        push(&mut errors, "MISSING_CASE_ID", "$.case_id", "case_id is required");
    }
    if !matches!(
        text(case, "evidence_class"),
        Some("SYNTHETIC_CONFORMANCE_FIXTURE" | "MEASURED")
    ) {
        push(&mut errors, "INVALID_EVIDENCE_CLASS", "$.evidence_class", "evidence class is outside v0.1 vocabulary");
    }

    let Some(context) = case.get("parent_context") else {
        push(&mut errors, "MISSING_PARENT_CONTEXT", "$.parent_context", "exact #3876 trial/configuration parent identity is required");
        return errors;
    };
    if text(context, "parent_case_id") != text(parent, "case_id") {
        push(&mut errors, "PARENT_CASE_MISMATCH", "$.parent_context.parent_case_id", "measurement case must bind to the exact supplied #3876 parent case");
    }
    let parent_profile = parent.get("profile");
    let intervals = parent.get("intervals").and_then(Value::as_array);
    let interval_id = text(context, "parent_interval_id");
    let matching_intervals: Vec<&Value> = intervals
        .into_iter()
        .flatten()
        .filter(|interval| text(interval, "interval_id") == interval_id)
        .collect();
    if matching_intervals.len() != 1 {
        push(&mut errors, "PARENT_INTERVAL_NOT_UNIQUE", "$.parent_context.parent_interval_id", "parent interval must resolve to exactly one interval");
    }
    if let Some(interval) = matching_intervals.first() {
        for (key, expected) in [
            ("trial_id", text(interval, "trial_id")),
            ("machine_configuration_ref", text(interval, "configuration_ref")),
        ] {
            if text(context, key) != expected || expected.is_none() {
                push(&mut errors, "PARENT_CONTEXT_MISMATCH", &format!("$.parent_context.{key}"), "parent trial/configuration identity mismatch");
            }
        }
    }
    if let Some(profile) = parent_profile {
        for key in [
            "material_cohort_ref",
            "machine_configuration_ref",
            "transport_configuration_ref",
            "clock_sync_protocol_ref",
        ] {
            if text(context, key) != text(profile, key) || text(profile, key).is_none() {
                push(&mut errors, "PARENT_PROFILE_MISMATCH", &format!("$.parent_context.{key}"), "measurement context must match exact parent profile identity");
            }
        }
    } else {
        push(&mut errors, "PARENT_PROFILE_MISSING", "$.parent.profile", "parent profile is required");
    }

    let Some(timebase) = case.get("timebase") else {
        push(&mut errors, "MISSING_TIMEBASE", "$.timebase", "common DAQ timebase evidence is required");
        return errors;
    };
    for key in ["timebase_channel_id", "clock_sync_ref", "sync_protocol_ref", "latency_validation_ref"] {
        if text(timebase, key).is_none() {
            push(&mut errors, "MISSING_TIMEBASE_PROVENANCE", &format!("$.timebase.{key}"), "timebase channel, sync protocol, clock and latency validation references are required");
        }
    }
    match finite(timebase, "time_uncertainty_ms") {
        Some(v) if v >= 0.0 => {}
        _ => push(&mut errors, "INVALID_TIMEBASE_UNCERTAINTY", "$.timebase.time_uncertainty_ms", "timebase uncertainty must be finite and non-negative"),
    }

    let Some(channels) = case.get("channels").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_CHANNELS", "$.channels", "channels must be an array");
        return errors;
    };
    let mut channel_map: BTreeMap<String, &Value> = BTreeMap::new();
    for (index, channel) in channels.iter().enumerate() {
        let path = format!("$.channels[{index}]");
        let Some(id) = text(channel, "channel_id") else {
            push(&mut errors, "MISSING_CHANNEL_ID", &format!("{path}.channel_id"), "channel_id is required");
            continue;
        };
        if channel_map.insert(id.to_owned(), channel).is_some() {
            push(&mut errors, "DUPLICATE_CHANNEL_ID", &format!("{path}.channel_id"), "channel identifiers must be unique");
        }
        for key in [
            "role", "measurand", "technique", "station_id", "unit",
            "instrument_ref", "manufacturer_ref", "model_ref", "serial_ref",
            "source_artifact_ref",
        ] {
            if text(channel, key).is_none() {
                push(&mut errors, "MISSING_CHANNEL_IDENTITY", &format!("{path}.{key}"), "channel role, measurand, technique, station, unit, instrument and raw source identity are required");
            }
        }
        if channel.get("domain").is_none() {
            push(&mut errors, "MISSING_CHANNEL_DOMAIN", &format!("{path}.domain"), "material/path/geometry/environment domain is required");
        }
        match text(channel, "role") {
            Some("PRIMARY_REFERENCE")
                if !matches!(text(channel, "technique"), Some("GRAVIMETRIC_CATCH_AND_WEIGH" | "STATIC_BALANCE" | "LOAD_CELL_HOPPER")) =>
            {
                push(&mut errors, "PRIMARY_REFERENCE_TECHNIQUE_NOT_GRAVIMETRIC", &format!("{path}.technique"), "primary mass-flow reference must use an independently weighed gravimetric technique; online estimators cannot self-designate as reference truth");
            }
            Some("TIMEBASE") if text(channel, "technique") != Some("DAQ_TIMEBASE") => {
                push(&mut errors, "TIMEBASE_TECHNIQUE_MISMATCH", &format!("{path}.technique"), "TIMEBASE role requires a DAQ_TIMEBASE technique");
            }
            _ => {}
        }
    }

    let timebase_id = text(timebase, "timebase_channel_id");
    match timebase_id.and_then(|id| channel_map.get(id)) {
        Some(channel) if text(channel, "role") == Some("TIMEBASE") => {}
        _ => push(&mut errors, "TIMEBASE_CHANNEL_NOT_REGISTERED", "$.timebase.timebase_channel_id", "timebase must reference a registered TIMEBASE channel"),
    }
    let timebase_channel = timebase_id.and_then(|id| channel_map.get(id).copied());

    let Some(calibrations) = case.get("calibrations").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_CALIBRATIONS", "$.calibrations", "calibrations must be an array");
        return errors;
    };
    let mut calibration_map: BTreeMap<String, &Value> = BTreeMap::new();
    for (index, calibration) in calibrations.iter().enumerate() {
        let path = format!("$.calibrations[{index}]");
        let Some(id) = text(calibration, "calibration_id") else {
            push(&mut errors, "MISSING_CALIBRATION_ID", &format!("{path}.calibration_id"), "calibration_id is required");
            continue;
        };
        if calibration_map.insert(id.to_owned(), calibration).is_some() {
            push(&mut errors, "DUPLICATE_CALIBRATION_ID", &format!("{path}.calibration_id"), "calibration IDs must be unique");
        }
        let channel_id = text(calibration, "channel_id");
        let Some(channel) = channel_id.and_then(|id| channel_map.get(id).copied()) else {
            push(&mut errors, "UNKNOWN_CALIBRATION_CHANNEL", &format!("{path}.channel_id"), "calibration must reference a registered channel");
            continue;
        };
        for key in [
            "certificate_ref", "issuing_laboratory_ref", "accreditation_scope_ref",
            "method_ref", "traceability_chain_ref", "source_artifact_ref",
        ] {
            if text(calibration, key).is_none() {
                push(&mut errors, "MISSING_CALIBRATION_TRACEABILITY", &format!("{path}.{key}"), "calibration certificate, laboratory, scope, method, traceability chain and source artifact are required");
            }
        }
        if text(calibration, "status") != Some("VALID") {
            push(&mut errors, "CALIBRATION_NOT_VALID", &format!("{path}.status"), "only explicitly VALID calibration records can support qualification evidence");
        }
        if text(calibration, "as_found_status") != Some("PASS")
            || text(calibration, "as_left_status") != Some("PASS")
        {
            push(&mut errors, "CALIBRATION_AS_FOUND_OR_LEFT_FAILED", &path, "as-found and as-left states must both be PASS for admitted qualification evidence");
        }
        if text(calibration, "unit") != text(channel, "unit") {
            push(&mut errors, "CALIBRATION_UNIT_MISMATCH", &format!("{path}.unit"), "calibration range unit must match the registered channel unit");
        }
        match (finite(calibration, "range_lower"), finite(calibration, "range_upper")) {
            (Some(lo), Some(hi)) if hi > lo => {}
            _ => push(&mut errors, "INVALID_CALIBRATION_RANGE", &path, "calibration range must be finite and ordered with upper > lower"),
        }
        if !same_domain(
            calibration.get("domain").unwrap_or(&Value::Null),
            channel.get("domain").unwrap_or(&Value::Null),
        ) {
            push(&mut errors, "CALIBRATION_DOMAIN_MISMATCH", &format!("{path}.domain"), "calibration domain must match the declared channel domain; transfer requires separate validation evidence");
        }
    }

    let Some(measurements) = case.get("measurement_records").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_MEASUREMENT_RECORDS", "$.measurement_records", "measurement_records must be an array");
        return errors;
    };
    let mut measurement_map: BTreeMap<String, &Value> = BTreeMap::new();
    for (index, measurement) in measurements.iter().enumerate() {
        let path = format!("$.measurement_records[{index}]");
        let Some(id) = text(measurement, "measurement_id") else {
            push(&mut errors, "MISSING_MEASUREMENT_ID", &format!("{path}.measurement_id"), "measurement_id is required");
            continue;
        };
        if measurement_map.insert(id.to_owned(), measurement).is_some() {
            push(&mut errors, "DUPLICATE_MEASUREMENT_ID", &format!("{path}.measurement_id"), "measurement identifiers must be unique");
        }
        let channel_id = text(measurement, "channel_id");
        let Some(channel) = channel_id.and_then(|id| channel_map.get(id).copied()) else {
            push(&mut errors, "UNKNOWN_MEASUREMENT_CHANNEL", &format!("{path}.channel_id"), "measurement must reference a registered channel");
            continue;
        };
        if text(measurement, "station_id") != text(channel, "station_id") {
            push(&mut errors, "MEASUREMENT_STATION_MISMATCH", &format!("{path}.station_id"), "measurement station must match exact channel station");
        }
        if text(measurement, "unit") != text(channel, "unit") {
            push(&mut errors, "MEASUREMENT_UNIT_MISMATCH", &format!("{path}.unit"), "measurement unit must match channel unit");
        }
        if !same_domain(
            measurement.get("domain").unwrap_or(&Value::Null),
            channel.get("domain").unwrap_or(&Value::Null),
        ) {
            push(&mut errors, "MEASUREMENT_DOMAIN_MISMATCH", &format!("{path}.domain"), "measurement material/path/geometry/environment domain must match registered channel domain");
        }
        let calibration_id = text(measurement, "calibration_id");
        let Some(calibration) = calibration_id.and_then(|id| calibration_map.get(id).copied()) else {
            push(&mut errors, "UNKNOWN_MEASUREMENT_CALIBRATION", &format!("{path}.calibration_id"), "measurement must reference a registered calibration record");
            continue;
        };
        if text(calibration, "channel_id") != channel_id {
            push(&mut errors, "CALIBRATION_CHANNEL_MISMATCH", &format!("{path}.calibration_id"), "measurement calibration must belong to the exact measured channel");
        }
        if let (Some(value), Some(lo), Some(hi)) = (
            finite(measurement, "value"),
            finite(calibration, "range_lower"),
            finite(calibration, "range_upper"),
        ) {
            if value < lo || value > hi {
                push(&mut errors, "MEASUREMENT_OUTSIDE_CALIBRATION_RANGE", &format!("{path}.value"), "measurement value lies outside the calibration range");
            }
        } else {
            push(&mut errors, "INVALID_MEASUREMENT_VALUE", &format!("{path}.value"), "measurement value must be finite");
        }
        if let (Some(date), Some(start), Some(end)) = (
            text(measurement, "measurement_date"),
            text(calibration, "valid_from"),
            text(calibration, "valid_until"),
        ) {
            if !date_inclusive(date, start, end) {
                push(&mut errors, "CALIBRATION_OUTSIDE_VALIDITY", &format!("{path}.measurement_date"), "measurement date must fall inside calibration validity interval");
            }
        }
        if text(calibration, "status") != Some("VALID") {
            push(&mut errors, "MEASUREMENT_USES_INVALID_CALIBRATION", &format!("{path}.calibration_id"), "measurement uses a non-valid calibration");
        }

        if text(measurement, "timebase_channel_id") != timebase_id
            || text(measurement, "clock_sync_ref") != text(timebase, "clock_sync_ref")
        {
            push(&mut errors, "MEASUREMENT_CLOCK_MISMATCH", &path, "measurement must bind to the case's exact shared timebase and clock sync reference");
        }
        if timebase_channel.is_none_or(|channel| text(channel, "role") != Some("TIMEBASE")) {
            push(&mut errors, "MEASUREMENT_TIMEBASE_UNAVAILABLE", &path, "measurement timebase channel is not registered with TIMEBASE role");
        }
        match (finite(measurement, "latency_ms"), finite(measurement, "time_uncertainty_ms")) {
            (Some(latency), Some(uncertainty)) if latency >= 0.0 && uncertainty >= 0.0 => {}
            _ => push(&mut errors, "INVALID_MEASUREMENT_TIMING", &path, "channel latency and timing uncertainty must be finite and non-negative"),
        }
        match (
            measurement.get("window_start_ms").and_then(Value::as_u64),
            measurement.get("window_end_ms").and_then(Value::as_u64),
        ) {
            (Some(start), Some(end)) if end > start => {}
            _ => push(&mut errors, "INVALID_MEASUREMENT_WINDOW", &path, "measurement window must have end > start"),
        }
        for key in ["measurement_window_ref", "source_artifact_ref"] {
            if text(measurement, key).is_none() {
                push(&mut errors, "MISSING_MEASUREMENT_PROVENANCE", &format!("{path}.{key}"), "measurement window and immutable source artifact references are required");
            }
        }
        let evidence_role = text(measurement, "evidence_role").unwrap_or("");
        if !role_matches_evidence(text(channel, "role").unwrap_or(""), evidence_role) {
            push(&mut errors, "MEASUREMENT_ROLE_EVIDENCE_MISMATCH", &format!("{path}.evidence_role"), "evidence role cannot promote a secondary estimator, diagnostic sensor or synthetic signal into primary reference truth");
        }
        if text(channel, "role") == Some("PARTICLE_CHARACTERIZATION")
            && (text(measurement, "sample_protocol_ref").is_none()
                || text(measurement, "sample_lineage_ref").is_none())
        {
            push(&mut errors, "PARTICLE_SAMPLE_PROVENANCE_INCOMPLETE", &path, "particle characterization requires an explicit sample protocol and sample lineage");
        }

        let Some(model) = measurement.get("measurement_model") else {
            push(&mut errors, "MISSING_MEASUREMENT_MODEL", &format!("{path}.measurement_model"), "measurement model mapping and digest are required");
            continue;
        };
        for key in ["mapping_kind", "linearity", "equation_ref", "model_digest_ref", "calibration_domain_ref"] {
            if text(model, key).is_none() {
                push(&mut errors, "INCOMPLETE_MEASUREMENT_MODEL", &format!("{path}.measurement_model.{key}"), "equation, model digest, mapping kind, linearity and calibration-domain reference are required");
            }
        }
        let Some(uncertainty) = measurement.get("uncertainty") else {
            push(&mut errors, "MISSING_UNCERTAINTY_BUDGET", &format!("{path}.uncertainty"), "measurement uncertainty budget is required");
            continue;
        };
        validate_uncertainty(
            uncertainty,
            text(model, "linearity").unwrap_or("UNKNOWN"),
            &format!("{path}.uncertainty"),
            &mut errors,
        );
    }

    let Some(comparisons) = case.get("reference_comparisons").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_REFERENCE_COMPARISONS", "$.reference_comparisons", "reference_comparisons must be an array");
        return errors;
    };
    let mut comparison_ids = BTreeSet::new();
    for (index, comparison) in comparisons.iter().enumerate() {
        let path = format!("$.reference_comparisons[{index}]");
        let Some(id) = text(comparison, "comparison_id") else {
            push(&mut errors, "MISSING_REFERENCE_COMPARISON_ID", &format!("{path}.comparison_id"), "comparison_id is required");
            continue;
        };
        if !comparison_ids.insert(id.to_owned()) {
            push(&mut errors, "DUPLICATE_REFERENCE_COMPARISON_ID", &format!("{path}.comparison_id"), "comparison IDs must be unique");
        }
        let reference_id = text(comparison, "reference_measurement_id");
        let estimator_id = text(comparison, "estimator_measurement_id");
        let (Some(reference), Some(estimator)) = (
            reference_id.and_then(|id| measurement_map.get(id).copied()),
            estimator_id.and_then(|id| measurement_map.get(id).copied()),
        ) else {
            push(&mut errors, "UNKNOWN_REFERENCE_COMPARISON_MEASUREMENT", &path, "reference comparison must resolve to two declared measurements");
            continue;
        };
        if reference_id == estimator_id {
            push(&mut errors, "SELF_REFERENCE_COMPARISON", &path, "a measurement cannot validate itself");
        }
        let reference_channel_id = text(reference, "channel_id").unwrap_or("");
        let estimator_channel_id = text(estimator, "channel_id").unwrap_or("");
        let reference_channel = channel_map.get(reference_channel_id).copied();
        let estimator_channel = channel_map.get(estimator_channel_id).copied();
        if reference_channel.is_none_or(|channel| text(channel, "role") != Some("PRIMARY_REFERENCE")) {
            push(&mut errors, "COMPARISON_REFERENCE_NOT_PRIMARY", &path, "reference side must be a registered PRIMARY_REFERENCE channel");
        }
        if estimator_channel.is_none_or(|channel| text(channel, "role") != Some("SECONDARY_ESTIMATOR")) {
            push(&mut errors, "COMPARISON_ESTIMATOR_NOT_SECONDARY", &path, "estimator side must be a registered SECONDARY_ESTIMATOR channel");
        }
        for key in ["material_cohort_ref", "path_configuration_ref", "geometry_profile_ref"] {
            let declared = text(comparison, key);
            let reference_value = text(&reference["domain"], key);
            let estimator_value = text(&estimator["domain"], key);
            if declared.is_none()
                || declared != reference_value
                || declared != estimator_value
            {
                push(&mut errors, "REFERENCE_COMPARISON_DOMAIN_MISMATCH", &format!("{path}.{key}"), "declared comparison material/path/geometry must match both reference and estimator records exactly");
            }
        }
        if text(reference, "unit") != text(estimator, "unit")
            || text(reference_channel.unwrap_or(&Value::Null), "measurand")
                != text(estimator_channel.unwrap_or(&Value::Null), "measurand")
        {
            push(&mut errors, "REFERENCE_COMPARISON_MEASURAND_MISMATCH", &path, "reference and estimator must report the same measurand and unit");
        }
        if text(comparison, "measurement_window_ref") != text(reference, "measurement_window_ref")
            || text(comparison, "measurement_window_ref") != text(estimator, "measurement_window_ref")
        {
            push(&mut errors, "REFERENCE_COMPARISON_WINDOW_MISMATCH", &format!("{path}.measurement_window_ref"), "reference and estimator must compare the same declared measurement window");
        }
        match text(comparison, "dataset_partition") {
            Some("VALIDATION" | "PROSPECTIVE_HOLDOUT")
                if comparison.get("independent_of_model_training").and_then(Value::as_bool) != Some(true) =>
            {
                push(&mut errors, "VALIDATION_NOT_INDEPENDENT", &format!("{path}.independent_of_model_training"), "validation/holdout comparisons must be independent of model training");
            }
            Some("CALIBRATION" | "VALIDATION" | "PROSPECTIVE_HOLDOUT") => {}
            _ => push(&mut errors, "INVALID_COMPARISON_PARTITION", &format!("{path}.dataset_partition"), "comparison partition must be CALIBRATION, VALIDATION or PROSPECTIVE_HOLDOUT"),
        }
        for key in ["residual_artifact_ref", "adjudication_ref"] {
            if text(comparison, key).is_none() {
                push(&mut errors, "REFERENCE_COMPARISON_PROVENANCE_MISSING", &format!("{path}.{key}"), "residual artifact and independent adjudication references are required");
            }
        }
    }

    errors
}

/// Evaluate only the evidence disposition and coverage. This intentionally
/// does not calculate combined standard uncertainty or qualify an instrument.
pub fn evaluate_case(case: &Value, parent: &Value) -> Result<EvaluationReport, Vec<Violation>> {
    let errors = validate_case(case, parent);
    if !errors.is_empty() {
        return Err(errors);
    }
    let channels = case["channels"].as_array().expect("validated channels");
    let calibrations = case["calibrations"].as_array().expect("validated calibrations");
    let measurements = case["measurement_records"].as_array().expect("validated measurements");
    let comparisons = case["reference_comparisons"].as_array().expect("validated comparisons");

    let primary_count = channels
        .iter()
        .filter(|channel| text(channel, "role") == Some("PRIMARY_REFERENCE"))
        .count();
    let estimator_count = channels
        .iter()
        .filter(|channel| text(channel, "role") == Some("SECONDARY_ESTIMATOR"))
        .count();
    let validation_count = comparisons
        .iter()
        .filter(|comparison| {
            matches!(
                text(comparison, "dataset_partition"),
                Some("VALIDATION" | "PROSPECTIVE_HOLDOUT")
            ) && comparison.get("independent_of_model_training").and_then(Value::as_bool)
                == Some(true)
        })
        .count();

    let evidence_disposition = if text(case, "evidence_class")
        == Some("SYNTHETIC_CONFORMANCE_FIXTURE")
    {
        "SYNTHETIC_ONLY"
    } else if validation_count == 0 {
        "CALIBRATION_OR_MEASUREMENT_RECORDS_ONLY"
    } else {
        // A passing record validator is not a formal instrument qualification.
        // A responsible metrologist must review the scope, results and method.
        "INDEPENDENT_VALIDATION_EVIDENCE_PRESENT_REVIEW_REQUIRED"
    };

    Ok(EvaluationReport {
        case_id: text(case, "case_id").expect("validated case ID").to_owned(),
        evidence_disposition: evidence_disposition.to_owned(),
        channel_count: channels.len(),
        calibration_count: calibrations.len(),
        measurement_count: measurements.len(),
        reference_comparison_count: comparisons.len(),
        primary_reference_channel_count: primary_count,
        secondary_estimator_channel_count: estimator_count,
        independent_validation_comparison_count: validation_count,
        out_of_domain_comparison_count: 0,
        combined_uncertainty_computed: false,
        limitations: vec![
            "No combined standard uncertainty is calculated by this crate".to_owned(),
            "No instrument or laboratory is certified by a passing manifest".to_owned(),
            "Synthetic fixture evidence cannot qualify a physical sensor".to_owned(),
            "Secondary online estimators remain secondary until metrologist-reviewed independent validation on the declared domain".to_owned(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn parent() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/rock-extraction-portfolio/fixtures/synthetic-fragment-transport-case.v0_1.json"
        ))
        .expect("parent fixture parses")
    }

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/rock-extraction-portfolio/fixtures/synthetic-measurement-system-case.v0_1.json"
        ))
        .expect("measurement fixture parses")
    }

    fn has(errors: &[Violation], code: &str) -> bool {
        errors.iter().any(|error| error.code == code)
    }

    fn calibration_mut(case: &mut Value, id: &str) -> &mut Value {
        case["calibrations"].as_array_mut().unwrap()
            .iter_mut().find(|item| item["calibration_id"] == id).unwrap()
    }

    fn channel_mut(case: &mut Value, id: &str) -> &mut Value {
        case["channels"].as_array_mut().unwrap()
            .iter_mut().find(|item| item["channel_id"] == id).unwrap()
    }

    fn measurement_mut(case: &mut Value, id: &str) -> &mut Value {
        case["measurement_records"].as_array_mut().unwrap()
            .iter_mut().find(|item| item["measurement_id"] == id).unwrap()
    }

    #[test]
    fn synthetic_measurement_manifest_is_valid() {
        assert!(validate_case(&fixture(), &parent()).is_empty());
    }

    #[test]
    fn structural_report_counts_traceable_measurement_records() {
        let report = evaluate_case(&fixture(), &parent()).unwrap();
        assert_eq!(report.channel_count, 6);
        assert_eq!(report.calibration_count, 6);
        assert_eq!(report.measurement_count, 6);
        assert_eq!(report.reference_comparison_count, 2);
        assert_eq!(report.primary_reference_channel_count, 1);
        assert_eq!(report.secondary_estimator_channel_count, 1);
        assert_eq!(report.independent_validation_comparison_count, 1);
        assert_eq!(report.evidence_disposition, "SYNTHETIC_ONLY");
        assert!(!report.combined_uncertainty_computed);
    }

    #[test]
    fn parent_case_identity_must_match() {
        let mut case = fixture();
        case["parent_context"]["parent_case_id"] = serde_json::json!("OTHER");
        assert!(has(&validate_case(&case, &parent()), "PARENT_CASE_MISMATCH"));
    }

    #[test]
    fn parent_trial_and_material_cohort_must_match() {
        let mut case = fixture();
        case["parent_context"]["trial_id"] = serde_json::json!("OTHER-TRIAL");
        assert!(has(&validate_case(&case, &parent()), "PARENT_CONTEXT_MISMATCH"));
    }

    #[test]
    fn timebase_channel_must_be_registered() {
        let mut case = fixture();
        case["timebase"]["timebase_channel_id"] = serde_json::json!("missing-clock");
        assert!(has(&validate_case(&case, &parent()), "TIMEBASE_CHANNEL_NOT_REGISTERED"));
    }

    #[test]
    fn duplicate_channel_id_is_rejected() {
        let mut case = fixture();
        let duplicate = case["channels"][0].clone();
        case["channels"].as_array_mut().unwrap().push(duplicate);
        assert!(has(&validate_case(&case, &parent()), "DUPLICATE_CHANNEL_ID"));
    }

    #[test]
    fn duplicate_calibration_id_is_rejected() {
        let mut case = fixture();
        let duplicate = case["calibrations"][0].clone();
        case["calibrations"].as_array_mut().unwrap().push(duplicate);
        assert!(has(&validate_case(&case, &parent()), "DUPLICATE_CALIBRATION_ID"));
    }

    #[test]
    fn calibration_must_be_valid() {
        let mut case = fixture();
        calibration_mut(&mut case, "cal-ecc-flow-estimator-v1")["status"] =
            serde_json::json!("EXPIRED");
        assert!(has(&validate_case(&case, &parent()), "CALIBRATION_NOT_VALID"));
    }

    #[test]
    fn calibration_certificate_and_traceability_are_required() {
        let mut case = fixture();
        calibration_mut(&mut case, "cal-ecc-flow-estimator-v1")
            .as_object_mut().unwrap().remove("traceability_chain_ref");
        assert!(has(&validate_case(&case, &parent()), "MISSING_CALIBRATION_TRACEABILITY"));
    }

    #[test]
    fn calibration_as_found_failure_is_rejected() {
        let mut case = fixture();
        calibration_mut(&mut case, "cal-line-dp-v1")["as_found_status"] =
            serde_json::json!("FAIL");
        assert!(has(&validate_case(&case, &parent()), "CALIBRATION_AS_FOUND_OR_LEFT_FAILED"));
    }

    #[test]
    fn measurement_date_must_be_inside_calibration_validity() {
        let mut case = fixture();
        measurement_mut(&mut case, "measurement-ecc-flow-estimator-001")["measurement_date"] =
            serde_json::json!("2028-01-01");
        assert!(has(&validate_case(&case, &parent()), "CALIBRATION_OUTSIDE_VALIDITY"));
    }

    #[test]
    fn calibration_domain_must_match_channel_domain() {
        let mut case = fixture();
        calibration_mut(&mut case, "cal-ecc-flow-estimator-v1")["domain"]["material_cohort_ref"] =
            serde_json::json!("other-rock");
        assert!(has(&validate_case(&case, &parent()), "CALIBRATION_DOMAIN_MISMATCH"));
    }

    #[test]
    fn measurement_value_must_be_within_calibration_range() {
        let mut case = fixture();
        measurement_mut(&mut case, "measurement-ecc-flow-estimator-001")["value"] =
            serde_json::json!(1000.0);
        assert!(has(&validate_case(&case, &parent()), "MEASUREMENT_OUTSIDE_CALIBRATION_RANGE"));
    }

    #[test]
    fn measurement_must_bind_exact_channel_calibration() {
        let mut case = fixture();
        measurement_mut(&mut case, "measurement-ecc-flow-estimator-001")["calibration_id"] =
            serde_json::json!("cal-line-dp-v1");
        assert!(has(&validate_case(&case, &parent()), "CALIBRATION_CHANNEL_MISMATCH"));
    }

    #[test]
    fn measurement_unit_must_match_channel() {
        let mut case = fixture();
        measurement_mut(&mut case, "measurement-ecc-flow-estimator-001")["unit"] =
            serde_json::json!("g/s");
        assert!(has(&validate_case(&case, &parent()), "MEASUREMENT_UNIT_MISMATCH"));
    }

    #[test]
    fn measurement_clock_must_match_shared_timebase() {
        let mut case = fixture();
        measurement_mut(&mut case, "measurement-ecc-flow-estimator-001")["clock_sync_ref"] =
            serde_json::json!("other-clock");
        assert!(has(&validate_case(&case, &parent()), "MEASUREMENT_CLOCK_MISMATCH"));
    }

    #[test]
    fn particle_characterization_requires_sample_lineage() {
        let mut case = fixture();
        measurement_mut(&mut case, "measurement-dynamic-psd-001")["sample_protocol_ref"] =
            Value::Null;
        assert!(has(&validate_case(&case, &parent()), "PARTICLE_SAMPLE_PROVENANCE_INCOMPLETE"));
    }

    #[test]
    fn secondary_estimator_cannot_claim_primary_reference_role() {
        let mut case = fixture();
        measurement_mut(&mut case, "measurement-ecc-flow-estimator-001")["evidence_role"] =
            serde_json::json!("INDEPENDENT_REFERENCE_OBSERVATION");
        assert!(has(&validate_case(&case, &parent()), "MEASUREMENT_ROLE_EVIDENCE_MISMATCH"));
    }

    #[test]
    fn nonlinear_model_cannot_use_linearized_gum_only() {
        let mut case = fixture();
        measurement_mut(&mut case, "measurement-ecc-flow-estimator-001")["uncertainty"]["method"] =
            serde_json::json!("LINEARIZED_GUM");
        assert!(has(&validate_case(&case, &parent()), "NONLINEAR_MODEL_WITH_LINEARIZED_ONLY_UQ"));
    }

    #[test]
    fn monte_carlo_requires_result_artifact_and_sample_count() {
        let mut case = fixture();
        let u = &mut measurement_mut(&mut case, "measurement-ecc-flow-estimator-001")["uncertainty"];
        u["monte_carlo_result_ref"] = Value::Null;
        u["monte_carlo_sample_count"] = Value::Null;
        assert!(has(&validate_case(&case, &parent()), "INCOMPLETE_MONTE_CARLO_EVIDENCE"));
    }

    #[test]
    fn linearized_gum_requires_sensitivity_coefficients() {
        let mut case = fixture();
        measurement_mut(&mut case, "measurement-gravimetric-discharge-001")["uncertainty"]["components"][0]
            .as_object_mut().unwrap().remove("sensitivity_coefficient");
        assert!(has(&validate_case(&case, &parent()), "MISSING_SENSITIVITY_COEFFICIENT"));
    }

    #[test]
    fn correlated_components_require_covariance_evidence() {
        let mut case = fixture();
        let u = &mut measurement_mut(&mut case, "measurement-gravimetric-discharge-001")["uncertainty"];
        u["components"][0]["correlation_group_ref"] = serde_json::json!("shared-load-cell-chain");
        u["components"][1]["correlation_group_ref"] = serde_json::json!("shared-load-cell-chain");
        assert!(has(&validate_case(&case, &parent()), "MISSING_CORRELATION_EVIDENCE"));
    }

    #[test]
    fn uncertainty_source_and_units_are_required() {
        let mut case = fixture();
        measurement_mut(&mut case, "measurement-gravimetric-discharge-001")["uncertainty"]["components"][0]["source_ref"] =
            Value::Null;
        assert!(has(&validate_case(&case, &parent()), "MISSING_UNCERTAINTY_COMPONENT_PROVENANCE"));
    }

    #[test]
    fn reference_comparison_requires_primary_reference_and_secondary_estimator() {
        let mut case = fixture();
        case["reference_comparisons"][0]["reference_measurement_id"] =
            serde_json::json!("measurement-line-dp-001");
        assert!(has(&validate_case(&case, &parent()), "COMPARISON_REFERENCE_NOT_PRIMARY"));
    }

    #[test]
    fn validation_must_be_independent_of_model_training() {
        let mut case = fixture();
        case["reference_comparisons"][1]["independent_of_model_training"] =
            serde_json::json!(false);
        assert!(has(&validate_case(&case, &parent()), "VALIDATION_NOT_INDEPENDENT"));
    }

    #[test]
    fn reference_and_estimator_must_share_measurement_window() {
        let mut case = fixture();
        measurement_mut(&mut case, "measurement-ecc-flow-estimator-001")["measurement_window_ref"] =
            serde_json::json!("different-window");
        assert!(has(&validate_case(&case, &parent()), "REFERENCE_COMPARISON_WINDOW_MISMATCH"));
    }

    #[test]
    fn synthetic_evidence_never_becomes_physical_qualification() {
        let report = evaluate_case(&fixture(), &parent()).unwrap();
        assert_eq!(report.evidence_disposition, "SYNTHETIC_ONLY");
    }

    #[test]
    fn deterministic_replay() {
        let case = fixture();
        let parent = parent();
        assert_eq!(
            evaluate_case(&case, &parent).unwrap(),
            evaluate_case(&case, &parent).unwrap()
        );
    }
}
