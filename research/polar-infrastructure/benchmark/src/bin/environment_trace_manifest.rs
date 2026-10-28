use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;

/// A nullable JSON field that must still be present in the input object.
/// A plain Option<T> can accept an absent field, which is too permissive for
/// an evidence manifest contract.
#[derive(Debug)]
struct RequiredNullable<T>(Option<T>);

impl<'de, T> Deserialize<'de> for RequiredNullable<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(Self)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: String,
    artifact_kind: String,
    manifest_id: String,
    profile_id: String,
    status: String,
    evidence_class: String,
    acquisition: Acquisition,
    spatial_context: SpatialContext,
    time_axis: TimeAxis,
    trace_asset: TraceAsset,
    variables: Vec<TraceVariable>,
    quality: Quality,
    provenance: Provenance,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Acquisition {
    provider: RequiredNullable<String>,
    dataset_name: RequiredNullable<String>,
    dataset_version: RequiredNullable<String>,
    source_url: RequiredNullable<String>,
    attribution: RequiredNullable<String>,
    license_or_access_terms: RequiredNullable<String>,
    accessed_at_utc: RequiredNullable<String>,
    source_data_sha256: RequiredNullable<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SpatialContext {
    location_label: String,
    latitude_deg: RequiredNullable<f64>,
    longitude_deg: RequiredNullable<f64>,
    elevation_m: RequiredNullable<f64>,
    spatial_support: String,
    coordinate_reference: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TimeAxis {
    start_utc: RequiredNullable<String>,
    end_utc: RequiredNullable<String>,
    sample_interval_seconds: RequiredNullable<u64>,
    expected_sample_count: RequiredNullable<u64>,
    observed_sample_count: RequiredNullable<u64>,
    missing_sample_count: RequiredNullable<u64>,
    timestamp_semantics: String,
    timezone: String,
    gap_policy: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TraceAsset {
    uri: RequiredNullable<String>,
    sha256: RequiredNullable<String>,
    format: String,
    byte_size: RequiredNullable<u64>,
    immutable_archive_uri: RequiredNullable<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TraceVariable {
    variable_id: String,
    physical_quantity: String,
    canonical_unit: String,
    source_variable: RequiredNullable<String>,
    transformation: String,
    evidence_class: String,
    required_for_replay: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Quality {
    validation_status: String,
    missing_data_policy: String,
    known_biases_or_limits: Vec<String>,
    uncertainty_method: String,
    validation_evidence_uri: RequiredNullable<String>,
    validated_against: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Provenance {
    processing_code_revision: RequiredNullable<String>,
    transformation_recipe: String,
    transformation_config_sha256: RequiredNullable<String>,
    created_at_utc: RequiredNullable<String>,
}

#[derive(Debug, Serialize)]
struct ValidationReport {
    schema_version: &'static str,
    report_kind: &'static str,
    manifest_id: String,
    profile_id: String,
    manifest_status: String,
    contract_valid: bool,
    candidate_for_independent_verification: bool,
    result_scope: &'static str,
    errors: Vec<String>,
    warnings: Vec<String>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn uri_with_scheme(value: &str) -> bool {
    let Some((scheme, remainder)) = value.split_once("://") else {
        return false;
    };
    !scheme.is_empty() && !remainder.is_empty()
}

fn https_url(value: &str) -> bool {
    value
        .strip_prefix("https://")
        .is_some_and(|remainder| !remainder.is_empty() && !remainder.starts_with('/'))
}

fn utc_timestamp_shape(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 20
        && bytes.get(4) == Some(&b'-')
        && bytes.get(7) == Some(&b'-')
        && bytes.get(10) == Some(&b'T')
        && bytes.get(13) == Some(&b':')
        && bytes.get(16) == Some(&b':')
        && value.ends_with('Z')
}

fn check_nonempty(errors: &mut Vec<String>, value: &str, field: &str) {
    if !nonempty(value) {
        errors.push(format!("{field} must be non-empty"));
    }
}

fn check_sha(errors: &mut Vec<String>, value: Option<&str>, field: &str) {
    match value {
        Some(hash) if valid_sha256(hash) => {}
        Some(_) => errors.push(format!("{field} must be 64 lowercase hexadecimal characters")),
        None => errors.push(format!("{field} is required for a validated replay manifest")),
    }
}

fn check_utc_time(errors: &mut Vec<String>, value: Option<&str>, field: &str) {
    match value {
        Some(timestamp) if utc_timestamp_shape(timestamp) => {}
        Some(_) => errors.push(format!("{field} must be an RFC3339-like UTC timestamp ending in Z")),
        None => errors.push(format!("{field} is required for a validated replay manifest")),
    }
}

fn valid_evidence_class(value: &str) -> bool {
    matches!(
        value,
        "measured_field"
            | "reanalysis"
            | "mission_observed"
            | "remote_observation"
            | "validated_simulation"
            | "uncalibrated_model"
            | "synthetic_stress"
            | "hypothesized"
            | "not_evaluated"
    )
}

fn validate(manifest: &Manifest) -> ValidationReport {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if manifest.schema_version != "0.1.0" {
        errors.push("schema_version must be 0.1.0".to_string());
    }
    if manifest.artifact_kind != "symtropy-environment-trace-manifest" {
        errors.push("artifact_kind is not symtropy-environment-trace-manifest".to_string());
    }

    check_nonempty(&mut errors, &manifest.manifest_id, "manifest_id");
    check_nonempty(&mut errors, &manifest.profile_id, "profile_id");
    check_nonempty(&mut errors, &manifest.spatial_context.location_label, "spatial_context.location_label");
    check_nonempty(&mut errors, &manifest.spatial_context.spatial_support, "spatial_context.spatial_support");
    check_nonempty(&mut errors, &manifest.spatial_context.coordinate_reference, "spatial_context.coordinate_reference");
    check_nonempty(&mut errors, &manifest.trace_asset.format, "trace_asset.format");
    check_nonempty(&mut errors, &manifest.quality.missing_data_policy, "quality.missing_data_policy");
    check_nonempty(&mut errors, &manifest.quality.uncertainty_method, "quality.uncertainty_method");
    for note in &manifest.quality.known_biases_or_limits {
        check_nonempty(&mut errors, note, "quality.known_biases_or_limits[]");
    }
    check_nonempty(&mut errors, &manifest.provenance.transformation_recipe, "provenance.transformation_recipe");

    if !valid_evidence_class(&manifest.evidence_class) {
        errors.push(format!("unsupported root evidence_class {}", manifest.evidence_class));
    }
    if manifest.status == "validated_for_replay"
        && !matches!(
            manifest.evidence_class.as_str(),
            "measured_field" | "reanalysis" | "mission_observed" | "remote_observation" | "validated_simulation"
        )
    {
        errors.push("validated_for_replay requires an admitted evidence class; synthetic, hypothesized and unevaluated traces cannot claim this status".to_string());
    }
    if !matches!(
        manifest.status.as_str(),
        "template_only" | "ingested_unvalidated" | "validated_for_replay" | "blocked"
    ) {
        errors.push(format!("unsupported manifest status {}", manifest.status));
    }
    if !matches!(
        manifest.time_axis.timestamp_semantics.as_str(),
        "instantaneous" | "interval_mean" | "interval_accumulation" | "sample_event" | "mission_time" | "unknown"
    ) {
        errors.push("unsupported time_axis.timestamp_semantics".to_string());
    }
    if manifest.time_axis.timezone != "UTC" {
        errors.push("time_axis.timezone must be UTC; convert source timestamps explicitly".to_string());
    }
    if !matches!(
        manifest.time_axis.gap_policy.as_str(),
        "not_selected" | "preserve_and_report" | "bounded_interpolation_with_mask" | "reject_trace"
    ) {
        errors.push("unsupported time_axis.gap_policy".to_string());
    }
    if !matches!(
        manifest.quality.validation_status.as_str(),
        "not_evaluated" | "screened" | "validated_for_declared_use" | "rejected"
    ) {
        errors.push("unsupported quality.validation_status".to_string());
    }

    if let Some(latitude) = manifest.spatial_context.latitude_deg.0 {
        if !(-90.0..=90.0).contains(&latitude) {
            errors.push("spatial_context.latitude_deg must be in [-90, 90]".to_string());
        }
    }
    if let Some(longitude) = manifest.spatial_context.longitude_deg.0 {
        if !(-180.0..=180.0).contains(&longitude) {
            errors.push("spatial_context.longitude_deg must be in [-180, 180]".to_string());
        }
    }
    let _elevation_is_explicitly_present = manifest.spatial_context.elevation_m.0;

    if let Some(url) = manifest.acquisition.source_url.0.as_deref() {
        if !https_url(url) {
            errors.push("acquisition.source_url must be an HTTPS URL".to_string());
        }
    }
    if let Some(uri) = manifest.trace_asset.uri.0.as_deref() {
        if !uri_with_scheme(uri) {
            errors.push("trace_asset.uri must be an absolute URI with a scheme".to_string());
        }
    }
    if let Some(uri) = manifest.trace_asset.immutable_archive_uri.0.as_deref() {
        if !uri_with_scheme(uri) {
            errors.push("trace_asset.immutable_archive_uri must be an absolute URI with a scheme".to_string());
        }
    }
    if let Some(uri) = manifest.quality.validation_evidence_uri.0.as_deref() {
        if !uri_with_scheme(uri) {
            errors.push("quality.validation_evidence_uri must be an absolute URI with a scheme".to_string());
        }
    }

    let mut variable_ids = BTreeSet::new();
    for variable in &manifest.variables {
        check_nonempty(&mut errors, &variable.variable_id, "variables[].variable_id");
        check_nonempty(&mut errors, &variable.physical_quantity, "variables[].physical_quantity");
        check_nonempty(&mut errors, &variable.canonical_unit, "variables[].canonical_unit");
        check_nonempty(&mut errors, &variable.transformation, "variables[].transformation");
        if !variable_ids.insert(variable.variable_id.clone()) {
            errors.push(format!("duplicate variable_id {}", variable.variable_id));
        }
        if !valid_evidence_class(&variable.evidence_class) {
            errors.push(format!("unsupported evidence class for variable {}", variable.variable_id));
        }
    }
    if manifest.variables.is_empty() {
        errors.push("at least one variable definition is required".to_string());
    }

    match manifest.status.as_str() {
        "template_only" => {
            if manifest.acquisition.source_url.0.is_some()
                || manifest.acquisition.source_data_sha256.0.is_some()
                || manifest.time_axis.start_utc.0.is_some()
                || manifest.time_axis.end_utc.0.is_some()
                || manifest.time_axis.observed_sample_count.0.is_some()
                || manifest.trace_asset.uri.0.is_some()
                || manifest.trace_asset.sha256.0.is_some()
            {
                errors.push("template_only must not claim an acquired source, chronology or trace artifact".to_string());
            }
            warnings.push("template_only is schema metadata only; it contains no acquired trace and is not replayable".to_string());
        }
        "ingested_unvalidated" => {
            warnings.push("ingested_unvalidated cannot be admitted until data-quality and declared-use validation are recorded".to_string());
        }
        "blocked" => {
            warnings.push("blocked manifests are never candidates for replay admission".to_string());
        }
        "validated_for_replay" => {
            for (value, field) in [
                (manifest.acquisition.provider.0.as_deref(), "acquisition.provider"),
                (manifest.acquisition.dataset_name.0.as_deref(), "acquisition.dataset_name"),
                (manifest.acquisition.dataset_version.0.as_deref(), "acquisition.dataset_version"),
                (manifest.acquisition.attribution.0.as_deref(), "acquisition.attribution"),
                (manifest.acquisition.license_or_access_terms.0.as_deref(), "acquisition.license_or_access_terms"),
            ] {
                match value {
                    Some(value) if nonempty(value) => {}
                    _ => errors.push(format!("{field} is required for a validated replay manifest")),
                }
            }
            match manifest.acquisition.source_url.0.as_deref() {
                Some(url) if https_url(url) => {}
                _ => errors.push("acquisition.source_url must be a non-empty HTTPS URL for a validated replay manifest".to_string()),
            }
            check_sha(&mut errors, manifest.acquisition.source_data_sha256.0.as_deref(), "acquisition.source_data_sha256");
            check_sha(&mut errors, manifest.trace_asset.sha256.0.as_deref(), "trace_asset.sha256");

            for (value, field) in [
                (manifest.acquisition.accessed_at_utc.0.as_deref(), "acquisition.accessed_at_utc"),
                (manifest.provenance.created_at_utc.0.as_deref(), "provenance.created_at_utc"),
            ] {
                check_utc_time(&mut errors, value, field);
            }
            check_nonempty(&mut errors, manifest.provenance.processing_code_revision.0.as_deref().unwrap_or(""), "provenance.processing_code_revision");
            check_sha(
                &mut errors,
                manifest.provenance.transformation_config_sha256.0.as_deref(),
                "provenance.transformation_config_sha256",
            );

            let start = manifest.time_axis.start_utc.0.as_deref();
            let end = manifest.time_axis.end_utc.0.as_deref();
            check_utc_time(&mut errors, start, "time_axis.start_utc");
            check_utc_time(&mut errors, end, "time_axis.end_utc");
            if let (Some(start), Some(end)) = (start, end) {
                if start >= end {
                    errors.push("time_axis.start_utc must be earlier than end_utc".to_string());
                }
            }

            match manifest.time_axis.sample_interval_seconds.0 {
                Some(value) if value > 0 => {}
                _ => errors.push("time_axis.sample_interval_seconds must be positive".to_string()),
            }

            let expected = manifest.time_axis.expected_sample_count.0;
            let observed = manifest.time_axis.observed_sample_count.0;
            let missing = manifest.time_axis.missing_sample_count.0;
            match (expected, observed, missing) {
                (Some(expected), Some(observed), Some(missing)) if expected > 0 => {
                    match observed.checked_add(missing) {
                        Some(total) if total == expected => {}
                        _ => errors.push("expected_sample_count must equal observed_sample_count + missing_sample_count without overflow".to_string()),
                    }
                    if observed == 0 {
                        errors.push("observed_sample_count must be positive for replay admission".to_string());
                    }
                }
                _ => errors.push("expected, observed and missing sample counts are required for replay admission".to_string()),
            }

            if manifest.time_axis.gap_policy == "not_selected" {
                errors.push("time_axis.gap_policy must be selected before replay admission".to_string());
            }
            for (value, field) in [
                (manifest.trace_asset.uri.0.as_deref(), "trace_asset.uri"),
                (manifest.trace_asset.immutable_archive_uri.0.as_deref(), "trace_asset.immutable_archive_uri"),
            ] {
                match value {
                    Some(value) if uri_with_scheme(value) => {}
                    _ => errors.push(format!("{field} is required as an absolute URI for a validated replay manifest")),
                }
            }
            match manifest.trace_asset.byte_size.0 {
                Some(value) if value > 0 => {}
                _ => errors.push("trace_asset.byte_size must be positive".to_string()),
            }

            if manifest.quality.validation_status != "validated_for_declared_use" {
                errors.push("quality.validation_status must be validated_for_declared_use".to_string());
            }
            match manifest.quality.validation_evidence_uri.0.as_deref() {
                Some(uri) if uri_with_scheme(uri) => {}
                _ => errors.push("quality.validation_evidence_uri is required for a validated replay manifest".to_string()),
            }
            if manifest.quality.validated_against.is_empty() {
                errors.push("quality.validated_against must name at least one reference or validation method".to_string());
            }
            for reference in &manifest.quality.validated_against {
                check_nonempty(&mut errors, reference, "quality.validated_against[]");
            }

            for variable in &manifest.variables {
                if variable.required_for_replay {
                    match variable.source_variable.0.as_deref() {
                        Some(source) if nonempty(source) => {}
                        _ => errors.push(format!("required variable {} has no source_variable mapping", variable.variable_id)),
                    }
                    if matches!(variable.evidence_class.as_str(), "not_evaluated" | "hypothesized") {
                        errors.push(format!("required variable {} has no evaluated evidence basis", variable.variable_id));
                    }
                }
            }

            warnings.push("this validator checks manifest metadata only; an independent verifier must hash the actual trace bytes and review domain-model validity before execution".to_string());
        }
        _ => {}
    }

    let contract_valid = errors.is_empty();
    let candidate_for_independent_verification =
        contract_valid && manifest.status == "validated_for_replay";

    ValidationReport {
        schema_version: "0.1.0",
        report_kind: "symtropy-environment-trace-manifest-validation-report",
        manifest_id: manifest.manifest_id.clone(),
        profile_id: manifest.profile_id.clone(),
        manifest_status: manifest.status.clone(),
        contract_valid,
        candidate_for_independent_verification,
        result_scope: "manifest_shape_and_gate_metadata_only_not_data_hash_or_physical_validation",
        errors,
        warnings,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let path = args.next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: environment_trace_manifest <manifest.json>",
        )
    })?;
    if args.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected exactly one manifest path",
        )
        .into());
    }

    let raw = fs::read_to_string(&path)?;
    let manifest: Manifest = serde_json::from_str(&raw).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("manifest parse/shape failure in {}: {error}", Path::new(&path).display()),
        )
    })?;
    let report = validate(&manifest);
    println!("{}", serde_json::to_string_pretty(&report)?);
    if !report.contract_valid {
        std::process::exit(2);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEMPLATE: &str =
        include_str!("../../fixtures/environment-trace-manifest-template-2026.json");

    #[test]
    fn template_is_structurally_valid_but_not_a_replay_candidate() {
        let manifest: Manifest = serde_json::from_str(TEMPLATE).expect("template should deserialize");
        let report = validate(&manifest);
        assert!(report.contract_valid, "{:?}", report.errors);
        assert!(!report.candidate_for_independent_verification);
        assert_eq!(report.manifest_status, "template_only");
        assert!(report.warnings.iter().any(|warning| warning.contains("contains no acquired trace")));
    }

    #[test]
    fn incomplete_manifest_cannot_claim_replay_validation() {
        let mut manifest: Manifest = serde_json::from_str(TEMPLATE).expect("template should deserialize");
        manifest.status = "validated_for_replay".to_string();
        let report = validate(&manifest);
        assert!(!report.contract_valid);
        assert!(!report.candidate_for_independent_verification);
        assert!(report.errors.iter().any(|error| error.contains("source_data_sha256")));
        assert!(report.errors.iter().any(|error| error.contains("sample counts")));
    }

    #[test]
    fn duplicate_variable_ids_are_rejected() {
        let mut manifest: Manifest = serde_json::from_str(TEMPLATE).expect("template should deserialize");
        let duplicate = manifest.variables[0].variable_id.clone();
        manifest.variables[1].variable_id = duplicate;
        let report = validate(&manifest);
        assert!(!report.contract_valid);
        assert!(report.errors.iter().any(|error| error.contains("duplicate variable_id")));
    }

    #[test]
    fn unknown_fields_are_rejected_by_deserialization() {
        let mut value: serde_json::Value = serde_json::from_str(TEMPLATE).expect("template JSON should parse");
        value["unreviewed_claim"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<Manifest>(value).is_err());
    }

    #[test]
    fn mismatched_sample_counts_are_rejected() {
        let mut manifest: Manifest = serde_json::from_str(TEMPLATE).expect("template should deserialize");
        manifest.status = "validated_for_replay".to_string();
        manifest.time_axis.expected_sample_count = RequiredNullable(Some(20));
        manifest.time_axis.observed_sample_count = RequiredNullable(Some(7));
        manifest.time_axis.missing_sample_count = RequiredNullable(Some(2));
        let report = validate(&manifest);
        assert!(report.errors.iter().any(|error| error.contains("expected_sample_count must equal")));
    }

    #[test]
    fn nullable_fields_must_be_present_even_when_the_value_is_null() {
        let mut value: serde_json::Value = serde_json::from_str(TEMPLATE).expect("template JSON should parse");
        value["acquisition"].as_object_mut().expect("acquisition object").remove("source_url");
        assert!(serde_json::from_value::<Manifest>(value).is_err());
    }

    #[test]
    fn hash_validation_rejects_uppercase_or_non_hex_digests() {
        assert!(!valid_sha256(&"A".repeat(64)));
        assert!(!valid_sha256(&"g".repeat(64)));
        assert!(valid_sha256(&"a".repeat(64)));
    }
}
