// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Offline, fail-closed importer for locally archived VIEWS API responses.
//! No network requests are performed here. The caller must provide a reviewed
//! column mapping, matching codebook, immutable outcome snapshot, and spatial
//! block map. Numeric values are never presumed to be probabilities.

use super::{
    sha256_hex, valid_commit_sha, validate_dataset_json, AnalysisUnit, EvidenceClass,
    ForecastDataset, ForecastOutcome, ForecastRecord, PrivacyPolicy, ResamplingPolicy,
    SourceContract, TemporalSplit,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

const MANIFEST_SCHEMA: &[u8] =
    include_bytes!("../../../docs/schemas/pirb-views-offline-import-v0.1.schema.json");

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum VariableKind {
    BinaryEventProbability,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ImportSource {
    provider: String,
    forecast_archive_id: String,
    model_id: String,
    model_version: String,
    forecast_variable: String,
    codebook_variable_key: String,
    variable_kind: VariableKind,
    target_definition: String,
    analysis_unit: AnalysisUnit,
    threshold_definition: String,
    codebook_required_phrase: String,
    forecast_source_reference: String,
    forecast_license_reference: String,
    forecast_attribution: String,
    outcome_source_id: String,
    outcome_release_id: String,
    outcome_definition: String,
    outcome_source_reference: String,
    outcome_license_reference: String,
    outcome_attribution: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ColumnMapping {
    year_index: usize,
    month_index: usize,
    unit_key_index: usize,
    probability_index: usize,
    metadata_model_index: usize,
    unit_id_prefix: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ResamplingSettings {
    temporal_block_length_months: usize,
    minimum_spatial_blocks: usize,
    minimum_temporal_blocks: usize,
    replicates: usize,
    minimum_valid_replicates: usize,
    seed: u64,
    confidence_level: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReleaseManifest {
    run_id: String,
    query_uri: String,
    response_paths: Vec<String>,
    codebook_path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ImportManifest {
    schema_version: String,
    dataset_id: String,
    evidence_class: EvidenceClass,
    source: ImportSource,
    partition: String,
    column_mapping: ColumnMapping,
    privacy: PrivacyPolicy,
    temporal_split: TemporalSplit,
    lead_months: i64,
    spatial_blocking_method_reference: String,
    resampling: ResamplingSettings,
    releases: Vec<ReleaseManifest>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct OutcomeSnapshot {
    schema_version: String,
    rows: Vec<OutcomeSnapshotRow>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct OutcomeSnapshotRow {
    unit_id: String,
    target_month: i64,
    outcome: ForecastOutcome,
    outcome_available_month: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SpatialBlockMap {
    schema_version: String,
    entries: Vec<SpatialBlockEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SpatialBlockEntry {
    unit_id: String,
    spatial_block_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ApiMetadata {
    models: Vec<String>,
    row_count: usize,
    page_count: usize,
    page_cur: usize,
    next_page: String,
    prev_page: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ApiPage {
    metadata: ApiMetadata,
    data: Vec<Vec<Value>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReleaseImportSummary {
    run_id: String,
    query_uri: String,
    input_cutoff_month: i64,
    target_month: i64,
    response_page_count: usize,
    response_page_sha256: Vec<String>,
    codebook_sha256: String,
    source_rows_received: usize,
    records_selected: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ViewsImportReceipt {
    receipt_version: String,
    evaluator: String,
    result: String,
    source_commit: String,
    source_commit_sha_valid: bool,
    rustc_version: String,
    cargo_version: String,
    dependency_lock_sha256: String,
    dataset_id: String,
    evidence_class: EvidenceClass,
    partition: String,
    forecast_archive_id: String,
    forecast_archive_sha256: String,
    manifest_sha256: String,
    outcome_snapshot_sha256: String,
    spatial_block_map_sha256: String,
    normalized_dataset_sha256: String,
    analysis_unit: AnalysisUnit,
    forecast_variable: String,
    codebook_variable_key: String,
    target_definition: String,
    threshold_definition: String,
    lead_months: i64,
    release_count: usize,
    forecast_record_count: usize,
    unit_count: usize,
    unknown_outcome_count: usize,
    query_uris: Vec<String>,
    releases: Vec<ReleaseImportSummary>,
    execution_claim: String,
    claim_ceiling: String,
}

pub struct ViewsImportBundle {
    pub normalized_dataset_bytes: Vec<u8>,
    pub receipt: ViewsImportReceipt,
}

#[derive(Debug, Clone, Serialize)]
struct ArchiveReleaseCommitment {
    run_id: String,
    query_uri: String,
    response_page_sha256: Vec<String>,
    codebook_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
struct ArchiveCommitment {
    commitment_version: String,
    releases: Vec<ArchiveReleaseCommitment>,
}

/// Return all source paths referenced by the manifest after rejecting path
/// traversal. The caller reads those files relative to the manifest location.
pub fn referenced_source_paths(manifest_bytes: &[u8]) -> Result<Vec<String>, Vec<String>> {
    let manifest = parse_manifest(manifest_bytes)?;
    let mut paths = BTreeSet::new();
    for release in &manifest.releases {
        validate_relative_path(&release.codebook_path)?;
        paths.insert(release.codebook_path.clone());
        for path in &release.response_paths {
            validate_relative_path(path)?;
            paths.insert(path.clone());
        }
    }
    Ok(paths.into_iter().collect())
}

fn validate_relative_path(path: &str) -> Result<(), Vec<String>> {
    let parsed = Path::new(path);
    if parsed.as_os_str().is_empty()
        || parsed.is_absolute()
        || parsed.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(vec![format!(
            "source path {path:?} must be safe and relative to the manifest directory"
        )]);
    }
    Ok(())
}

fn parse_manifest(input: &[u8]) -> Result<ImportManifest, Vec<String>> {
    let schema: Value = serde_json::from_slice(MANIFEST_SCHEMA)
        .map_err(|error| vec![format!("checked-in import schema JSON is invalid: {error}")])?;
    jsonschema::draft202012::meta::validate(&schema).map_err(|error| {
        vec![format!(
            "Draft 2020-12 import schema meta-validation failed: {error}"
        )]
    })?;
    let instance: Value = serde_json::from_slice(input)
        .map_err(|error| vec![format!("import manifest JSON is invalid: {error}")])?;
    let validator = jsonschema::draft202012::new(&schema)
        .map_err(|error| vec![format!("cannot build import-manifest validator: {error}")])?;
    let errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    if !errors.is_empty() {
        return Err(errors);
    }
    serde_json::from_value(instance)
        .map_err(|error| vec![format!("cannot decode import manifest: {error}")])
}

fn month_index(year: i64, month: i64) -> Result<i64, String> {
    if year < 1980 || !(1..=12).contains(&month) {
        return Err(format!("invalid calendar year/month {year}-{month:02}"));
    }
    year.checked_mul(12)
        .and_then(|value| value.checked_add(month - 1))
        .ok_or_else(|| "calendar month index overflows i64".to_owned())
}

fn api_month_id(year: i64, month: i64) -> Result<i64, String> {
    year.checked_sub(1980)
        .and_then(|value| value.checked_mul(12))
        .and_then(|value| value.checked_add(month))
        .ok_or_else(|| "VIEWS API month ID overflows i64".to_owned())
}

/// Run IDs have the documented form model+version_YEAR_MONTH_tNN.
fn parse_run_id(run_id: &str, model_prefix: &str) -> Result<(i64, i64), String> {
    let prefix = format!("{model_prefix}_");
    let suffix = run_id.strip_prefix(&prefix).ok_or_else(|| {
        format!("run ID {run_id:?} does not begin with expected prefix {model_prefix:?}")
    })?;
    let fields: Vec<&str> = suffix.split('_').collect();
    if fields.len() != 3 {
        return Err(format!(
            "run ID {run_id:?} must end with YEAR_MONTH_tNN"
        ));
    }
    let year: i64 = fields[0]
        .parse()
        .map_err(|_| format!("run ID {run_id:?} has invalid release year"))?;
    let month: i64 = fields[1]
        .parse()
        .map_err(|_| format!("run ID {run_id:?} has invalid release month"))?;
    let attempt = fields[2];
    if attempt.len() < 2
        || !attempt.starts_with('t')
        || !attempt[1..].bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(format!("run ID {run_id:?} has invalid attempt suffix"));
    }
    let cutoff = month_index(year, month)?;
    let origin = cutoff
        .checked_add(1)
        .ok_or_else(|| "forecast-origin month overflows i64".to_owned())?;
    Ok((cutoff, origin))
}

fn raw_cell<'a>(row: &'a [Value], index: usize, run_id: &str) -> Result<&'a Value, String> {
    row.get(index).ok_or_else(|| {
        format!(
            "run {run_id:?} data row is shorter than declared column index {index}"
        )
    })
}

fn cell_i64(row: &[Value], index: usize, run_id: &str, label: &str) -> Result<i64, String> {
    raw_cell(row, index, run_id)?
        .as_i64()
        .ok_or_else(|| format!("run {run_id:?} {label} column must be an integer"))
}

fn cell_key(row: &[Value], index: usize, run_id: &str) -> Result<String, String> {
    let value = raw_cell(row, index, run_id)?;
    let key = if let Some(value) = value.as_str() {
        value.to_owned()
    } else if let Some(value) = value.as_i64() {
        value.to_string()
    } else {
        return Err(format!(
            "run {run_id:?} unit-key column must be a string or integer"
        ));
    };
    if key.is_empty()
        || !key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(format!(
            "run {run_id:?} unit key contains unsupported characters"
        ));
    }
    Ok(key)
}

/// Import locally archived API response files into the normalized PIRB schema.
/// No requests, source discovery, or license decisions happen in this function.
pub fn import_views_archive(
    manifest_bytes: &[u8],
    outcomes_bytes: &[u8],
    block_map_bytes: &[u8],
    source_files: &BTreeMap<String, Vec<u8>>,
    source_commit: &str,
    rustc_version: &str,
    cargo_version: &str,
    lock_bytes: &[u8],
) -> Result<ViewsImportBundle, Vec<String>> {
    if source_commit != "UNPINNED_LOCAL_RUN" && !valid_commit_sha(source_commit) {
        return Err(vec![
            "source commit must be a lowercase 40-character Git SHA or UNPINNED_LOCAL_RUN"
                .to_owned(),
        ]);
    }
    let manifest = parse_manifest(manifest_bytes)?;
    if manifest.schema_version != "0.1" {
        return Err(vec!["import manifest schema_version must be 0.1".to_owned()]);
    }
    if manifest.source.codebook_variable_key != "main_dich"
        || manifest.source.variable_kind != VariableKind::BinaryEventProbability
    {
        return Err(vec![
            "probability import accepts only main_dich with explicit binary-event-probability semantics; count/log-count variables are rejected".to_owned(),
        ]);
    }
    if !matches!(manifest.partition.as_str(), "predict" | "eval" | "calib") {
        return Err(vec![
            "partition must be predict, eval, or calib".to_owned(),
        ]);
    }
    if manifest.evidence_class == EvidenceClass::AuthorizedHistorical
        && [
            manifest.source.forecast_source_reference.as_str(),
            manifest.source.forecast_license_reference.as_str(),
            manifest.source.forecast_attribution.as_str(),
            manifest.source.outcome_source_reference.as_str(),
            manifest.source.outcome_license_reference.as_str(),
            manifest.source.outcome_attribution.as_str(),
            manifest.spatial_blocking_method_reference.as_str(),
        ]
        .iter()
        .any(|reference| reference.trim().is_empty() || reference.starts_with("synthetic:"))
    {
        return Err(vec![
            "authorized historical import requires non-synthetic source, license, attribution, and spatial-method references".to_owned(),
        ]);
    }
    if !(1..=36).contains(&manifest.lead_months) {
        return Err(vec![
            "lead_months must be in the documented VIEWS range 1..=36".to_owned(),
        ]);
    }
    if manifest.source.forecast_variable.trim().is_empty()
        || manifest.source.model_id.trim().is_empty()
        || manifest.source.model_version.trim().is_empty()
        || manifest.source.target_definition.trim().is_empty()
        || manifest.source.threshold_definition.trim().is_empty()
        || manifest.source.codebook_required_phrase.trim().is_empty()
        || manifest.spatial_blocking_method_reference.trim().is_empty()
    {
        return Err(vec![
            "model, variable, target, threshold, codebook phrase and spatial-method fields must not be empty".to_owned(),
        ]);
    }

    let mapping = &manifest.column_mapping;
    let mut data_indices = BTreeSet::new();
    for (label, index) in [
        ("year", mapping.year_index),
        ("month", mapping.month_index),
        ("unit key", mapping.unit_key_index),
        ("probability", mapping.probability_index),
    ] {
        if !data_indices.insert(index) {
            return Err(vec![format!(
                "column mapping reuses data index {index} for {label}"
            )]);
        }
    }
    if mapping.metadata_model_index != 0 && mapping.metadata_model_index >= 64 {
        return Err(vec![
            "metadata_model_index is implausibly large; verify the response-model mapping"
                .to_owned(),
        ]);
    }
    if !mapping.unit_id_prefix.starts_with("unit-") || !mapping.unit_id_prefix.ends_with('-') {
        return Err(vec![
            "unit_id_prefix must begin with unit- and end with -".to_owned(),
        ]);
    }
    if manifest.releases.is_empty() {
        return Err(vec!["manifest must include at least one frozen release".to_owned()]);
    }

    let manifest_sha256 = sha256_hex(manifest_bytes);
    let outcome_sha256 = sha256_hex(outcomes_bytes);
    let block_map_sha256 = sha256_hex(block_map_bytes);

    let outcomes: OutcomeSnapshot = serde_json::from_slice(outcomes_bytes)
        .map_err(|error| vec![format!("outcome snapshot JSON is invalid: {error}")])?;
    if outcomes.schema_version != "0.1" {
        return Err(vec!["outcome snapshot schema_version must be 0.1".to_owned()]);
    }
    let mut outcome_by_key = BTreeMap::new();
    for row in outcomes.rows {
        if row.unit_id.trim().is_empty() || row.target_month <= 0 {
            return Err(vec![
                "outcome snapshot rows need a non-empty unit_id and positive target_month"
                    .to_owned(),
            ]);
        }
        if row.outcome == ForecastOutcome::Unknown && row.outcome_available_month.is_some() {
            return Err(vec![
                "unknown outcome rows must not claim an availability month".to_owned(),
            ]);
        }
        if row.outcome != ForecastOutcome::Unknown && row.outcome_available_month.is_none() {
            return Err(vec![
                "known outcome rows require an outcome availability month".to_owned(),
            ]);
        }
        let key = (row.unit_id.clone(), row.target_month);
        if outcome_by_key.insert(key, row).is_some() {
            return Err(vec![
                "outcome snapshot has duplicate unit/target-month keys".to_owned(),
            ]);
        }
    }

    let blocks: SpatialBlockMap = serde_json::from_slice(block_map_bytes)
        .map_err(|error| vec![format!("spatial block-map JSON is invalid: {error}")])?;
    if blocks.schema_version != "0.1" {
        return Err(vec!["spatial block-map schema_version must be 0.1".to_owned()]);
    }
    let mut block_by_unit = BTreeMap::new();
    for entry in blocks.entries {
        if !entry.unit_id.starts_with("unit-")
            || entry.spatial_block_id.trim().is_empty()
            || !entry
                .spatial_block_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err(vec![
                "spatial block-map entries require normalized unit IDs and opaque alphanumeric block IDs".to_owned(),
            ]);
        }
        if block_by_unit
            .insert(entry.unit_id, entry.spatial_block_id)
            .is_some()
        {
            return Err(vec![
                "spatial block-map contains a duplicate unit ID".to_owned(),
            ]);
        }
    }
    if block_by_unit.is_empty() {
        return Err(vec![
            "spatial block-map must contain at least one mapping".to_owned(),
        ]);
    }

    let model_prefix = format!("{}{}", manifest.source.model_id, manifest.source.model_version);
    let required_unit_path = match manifest.source.analysis_unit {
        AnalysisUnit::CountryMonth => "/cm/",
        AnalysisUnit::PrioGridMonth => "/pgm/",
    };
    let model_variable = &manifest.source.forecast_variable;
    let mut records = Vec::new();
    let mut release_summaries = Vec::new();
    let mut archive_releases = Vec::new();
    let mut used_outcome_keys = BTreeSet::new();
    let mut used_forecast_ids = BTreeSet::new();
    let mut used_forecast_keys = BTreeSet::new();
    let mut seen_run_ids = BTreeSet::new();
    let mut source_paths_used = BTreeSet::new();

    for release in &manifest.releases {
        if !seen_run_ids.insert(release.run_id.clone()) {
            return Err(vec![format!("duplicate release run ID {:?}", release.run_id)]);
        }
        let (input_cutoff_month, forecast_origin_month) =
            parse_run_id(&release.run_id, &model_prefix).map_err(|error| vec![error])?;
        let expected_prefix = format!("https://api.viewsforecasting.org/{}/", release.run_id);
        if !release.query_uri.starts_with(&expected_prefix) {
            return Err(vec![format!(
                "query URI for {:?} must identify that exact immutable API run",
                release.run_id
            )]);
        }
        if !release.query_uri.contains(required_unit_path) {
            return Err(vec![format!(
                "query URI for {:?} does not match declared analysis unit",
                release.run_id
            )]);
        }
        let expected_suffix = format!("/{model_variable}");
        let query_without_args = release.query_uri.split('?').next().unwrap_or("");
        if !query_without_args.ends_with(&expected_suffix) {
            return Err(vec![format!(
                "query URI for {:?} does not terminate at declared probability variable {:?}",
                release.run_id, model_variable
            )]);
        }

        validate_relative_path(&release.codebook_path)?;
        source_paths_used.insert(release.codebook_path.clone());
        let codebook_bytes = source_files
            .get(&release.codebook_path)
            .ok_or_else(|| {
                vec![format!(
                    "missing archived codebook file {:?}",
                    release.codebook_path
                )]
            })?;
        let codebook_sha256 = sha256_hex(codebook_bytes);
        let codebook: Value = serde_json::from_slice(codebook_bytes)
            .map_err(|error| vec![format!("invalid codebook JSON: {error}")])?;
        let codebook_entry = codebook
            .get(&manifest.source.codebook_variable_key)
            .ok_or_else(|| vec!["codebook lacks required main_dich entry".to_owned()])?;
        let description = codebook_entry
            .get("description")
            .and_then(Value::as_str)
            .ok_or_else(|| vec!["main_dich codebook entry lacks a description".to_owned()])?;
        let unit_name = codebook_entry
            .pointer("/unit/name")
            .and_then(Value::as_str)
            .unwrap_or("");
        let unit_range = codebook_entry
            .pointer("/unit/range")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !unit_name.eq_ignore_ascii_case("probability") || unit_range != "0-1" {
            return Err(vec![
                "main_dich codebook entry must declare Probability with range 0-1; fatality counts are not probabilities".to_owned(),
            ]);
        }
        if !description
            .to_lowercase()
            .contains(&manifest.source.codebook_required_phrase.to_lowercase())
        {
            return Err(vec![
                "codebook description does not contain the manifest's predeclared threshold phrase"
                    .to_owned(),
            ]);
        }
        let geography_phrase = match manifest.source.analysis_unit {
            AnalysisUnit::CountryMonth => "country-month",
            AnalysisUnit::PrioGridMonth => "prio-grid-month",
        };
        if !description.to_lowercase().contains(geography_phrase) {
            return Err(vec![
                "codebook description does not match declared analysis unit".to_owned(),
            ]);
        }

        let mut pages = Vec::new();
        let mut page_numbers = BTreeSet::new();
        let mut expected_page_count = None;
        let mut expected_row_count = None;

        for path in &release.response_paths {
            validate_relative_path(path)?;
            source_paths_used.insert(path.clone());
            let bytes = source_files.get(path).ok_or_else(|| {
                vec![format!("missing archived API response file {path:?}")]
            })?;
            let page: ApiPage = serde_json::from_slice(bytes).map_err(|error| {
                vec![format!(
                    "API response {path:?} is not a supported metadata/data envelope: {error}"
                )]
            })?;
            if page.metadata.models.get(mapping.metadata_model_index) != Some(model_variable) {
                return Err(vec![format!(
                    "response {path:?} metadata model at declared index does not match {model_variable:?}"
                )]);
            }
            if page.metadata.page_count == 0
                || page.metadata.page_cur == 0
                || page.metadata.page_cur > page.metadata.page_count
                || !page_numbers.insert(page.metadata.page_cur)
            {
                return Err(vec![format!(
                    "response {path:?} has invalid or duplicate pagination metadata"
                )]);
            }
            if let Some(count) = expected_page_count {
                if count != page.metadata.page_count {
                    return Err(vec![
                        "response pages disagree on metadata.page_count".to_owned(),
                    ]);
                }
            } else {
                expected_page_count = Some(page.metadata.page_count);
            }
            if let Some(count) = expected_row_count {
                if count != page.metadata.row_count {
                    return Err(vec![
                        "response pages disagree on metadata.row_count".to_owned(),
                    ]);
                }
            } else {
                expected_row_count = Some(page.metadata.row_count);
            }
            if page.metadata.page_cur == 1 && !page.metadata.prev_page.is_empty() {
                return Err(vec![
                    "first response page must have empty metadata.prev_page".to_owned(),
                ]);
            }
            if page.metadata.page_cur > 1 && page.metadata.prev_page.is_empty() {
                return Err(vec![
                    "non-first response page must include metadata.prev_page".to_owned(),
                ]);
            }
            if page.metadata.page_cur == page.metadata.page_count
                && !page.metadata.next_page.is_empty()
            {
                return Err(vec![
                    "last response page must have empty metadata.next_page".to_owned(),
                ]);
            }
            if page.metadata.page_cur < page.metadata.page_count
                && page.metadata.next_page.is_empty()
            {
                return Err(vec![
                    "non-final response page must include metadata.next_page".to_owned(),
                ]);
            }
            pages.push((page.metadata.page_cur, path.clone(), page));
        }

        let page_count = expected_page_count
            .ok_or_else(|| vec![format!("release {:?} has no pages", release.run_id)])?;
        let row_count = expected_row_count.unwrap_or(0);
        if pages.len() != page_count || page_numbers.len() != page_count {
            return Err(vec![format!(
                "release {:?} is incomplete: manifest has {} page files but metadata declares {page_count}",
                release.run_id,
                pages.len()
            )]);
        }
        pages.sort_by_key(|(page_number, _, _)| *page_number);
        let page_hashes: Vec<String> = pages
            .iter()
            .map(|(_, path, _)| sha256_hex(source_files.get(path).expect("page path was validated")))
            .collect();
        let actual_rows: usize = pages.iter().map(|(_, _, page)| page.data.len()).sum();
        if actual_rows != row_count {
            return Err(vec![format!(
                "release {:?} received {actual_rows} rows but metadata.row_count declares {row_count}",
                release.run_id
            )]);
        }

        let expected_target = input_cutoff_month
            .checked_add(manifest.lead_months)
            .ok_or_else(|| vec!["target month overflows i64".to_owned()])?;
        let mut release_records = Vec::new();
        for (_, path, page) in &pages {
            for raw in &page.data {
                let year = cell_i64(raw, mapping.year_index, &release.run_id, "year")
                    .map_err(|error| vec![error])?;
                let month = cell_i64(raw, mapping.month_index, &release.run_id, "month")
                    .map_err(|error| vec![error])?;
                let target_month = month_index(year, month).map_err(|error| vec![error])?;
                if target_month != expected_target {
                    return Err(vec![format!(
                        "response {path:?} target month {target_month} does not equal run cutoff plus declared lead ({expected_target})"
                    )]);
                }
                let expected_api_month = api_month_id(year, month).map_err(|error| vec![error])?;
                let expected_filter = format!("month={expected_api_month}");
                if !release.query_uri.split('?').nth(1).is_some_and(|query| {
                    query.split('&').any(|field| field == expected_filter)
                }) {
                    return Err(vec![format!(
                        "query URI for {:?} is not pinned to the exact target month ({expected_filter})",
                        release.run_id
                    )]);
                }
                let raw_unit = cell_key(raw, mapping.unit_key_index, &release.run_id)
                    .map_err(|error| vec![error])?;
                let unit_id = format!("{}{}", mapping.unit_id_prefix, raw_unit);
                if !unit_id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
                {
                    return Err(vec![
                        "normalized unit ID contains unsupported characters".to_owned(),
                    ]);
                }
                let block_id = block_by_unit.get(&unit_id).ok_or_else(|| {
                    vec![format!("unit {unit_id:?} has no governed spatial-block mapping")]
                })?;
                let probability = raw_cell(raw, mapping.probability_index, &release.run_id)
                    .map_err(|error| vec![error])?
                    .as_f64()
                    .ok_or_else(|| {
                        vec![format!(
                            "run {:?} probability cell is not numeric",
                            release.run_id
                        )]
                    })?;
                if !probability.is_finite() || !(0.0..=1.0).contains(&probability) {
                    return Err(vec![format!(
                        "run {:?} probability must be finite and within [0, 1]",
                        release.run_id
                    )]);
                }
                let outcome_key = (unit_id.clone(), target_month);
                let outcome = outcome_by_key.get(&outcome_key).ok_or_else(|| {
                    vec![format!(
                        "outcome snapshot has no row for {unit_id:?} at target month {target_month}"
                    )]
                })?;
                if !used_outcome_keys.insert(outcome_key.clone()) {
                    return Err(vec![format!(
                        "duplicate forecast unit/target row for {unit_id:?} at {target_month}"
                    )]);
                }
                let forecast_id = format!(
                    "forecast-{}-{}",
                    release.run_id.replace('_', "-"),
                    unit_id.strip_prefix("unit-").unwrap_or(&unit_id)
                );
                if !used_forecast_ids.insert(forecast_id.clone()) {
                    return Err(vec![format!("duplicate generated forecast ID {forecast_id:?}")]);
                }
                let forecast_key = (unit_id.clone(), target_month, manifest.lead_months);
                if !used_forecast_keys.insert(forecast_key) {
                    return Err(vec![
                        "duplicate unit/target/horizon forecast key".to_owned(),
                    ]);
                }
                release_records.push(ForecastRecord {
                    forecast_id,
                    forecast_release_id: release.run_id.clone(),
                    unit_id,
                    spatial_block_id: block_id.clone(),
                    forecast_origin_month,
                    input_cutoff_month,
                    target_month,
                    lead_months: manifest.lead_months,
                    probability,
                    outcome: outcome.outcome,
                    outcome_available_month: outcome.outcome_available_month,
                });
            }
        }
        release_records.sort_by(|left, right| left.forecast_id.cmp(&right.forecast_id));
        records.extend(release_records.iter().cloned());
        release_summaries.push(ReleaseImportSummary {
            run_id: release.run_id.clone(),
            query_uri: release.query_uri.clone(),
            input_cutoff_month,
            target_month: expected_target,
            response_page_count: page_count,
            response_page_sha256: page_hashes.clone(),
            codebook_sha256: codebook_sha256.clone(),
            source_rows_received: row_count,
            records_selected: release_records.len(),
        });
        archive_releases.push(ArchiveReleaseCommitment {
            run_id: release.run_id.clone(),
            query_uri: release.query_uri.clone(),
            response_page_sha256: page_hashes,
            codebook_sha256,
        });
    }

    if used_outcome_keys.len() != outcome_by_key.len() {
        return Err(vec![
            "outcome snapshot contains rows not used by the declared forecast inventory; use a snapshot matching the selected units and target months".to_owned(),
        ]);
    }
    let declared_source_paths: BTreeSet<String> = source_files.keys().cloned().collect();
    if source_paths_used != declared_source_paths {
        return Err(vec![
            "provided source files do not exactly match the manifest's referenced source-path inventory".to_owned(),
        ]);
    }
    records.sort_by(|left, right| left.forecast_id.cmp(&right.forecast_id));
    let forecast_record_count = records.len();
    let unique_units: BTreeSet<String> =
        records.iter().map(|record| record.unit_id.clone()).collect();
    let unknown_outcome_count = records
        .iter()
        .filter(|record| record.outcome == ForecastOutcome::Unknown)
        .count();
    release_summaries.sort_by(|left, right| left.run_id.cmp(&right.run_id));
    archive_releases.sort_by(|left, right| left.run_id.cmp(&right.run_id));

    let archive_commitment = ArchiveCommitment {
        commitment_version: "0.1".to_owned(),
        releases: archive_releases,
    };
    let archive_bytes = serde_json::to_vec(&archive_commitment)
        .map_err(|error| vec![format!("cannot serialize archive commitment: {error}")])?;
    let forecast_archive_sha256 = sha256_hex(&archive_bytes);
    let forecast_source_reference = format!(
        "{}; partition={}; manifest_sha256={}",
        manifest.source.forecast_source_reference, manifest.partition, manifest_sha256
    );
    let dataset = ForecastDataset {
        schema_version: "0.1".to_owned(),
        dataset_id: manifest.dataset_id.clone(),
        evidence_class: manifest.evidence_class,
        source: SourceContract {
            provider: manifest.source.provider.clone(),
            forecast_archive_id: manifest.source.forecast_archive_id.clone(),
            model_id: manifest.source.model_id.clone(),
            model_version: manifest.source.model_version.clone(),
            forecast_variable: manifest.source.forecast_variable.clone(),
            target_definition: manifest.source.target_definition.clone(),
            analysis_unit: manifest.source.analysis_unit,
            threshold_definition: manifest.source.threshold_definition.clone(),
            forecast_source_reference,
            forecast_archive_sha256: forecast_archive_sha256.clone(),
            outcome_source_id: manifest.source.outcome_source_id.clone(),
            outcome_release_id: manifest.source.outcome_release_id.clone(),
            outcome_definition: manifest.source.outcome_definition.clone(),
            outcome_source_reference: manifest.source.outcome_source_reference.clone(),
            outcome_release_sha256: outcome_sha256.clone(),
            license_reference: format!(
                "{}; outcome-license={}",
                manifest.source.forecast_license_reference,
                manifest.source.outcome_license_reference
            ),
            attribution: format!(
                "{}; outcome-attribution={}",
                manifest.source.forecast_attribution,
                manifest.source.outcome_attribution
            ),
        },
        privacy: manifest.privacy,
        temporal_split: manifest.temporal_split,
        lead_months: manifest.lead_months,
        resampling: ResamplingPolicy {
            spatial_blocking_method_reference: manifest.spatial_blocking_method_reference,
            spatial_block_map_sha256: block_map_sha256.clone(),
            temporal_block_length_months: manifest.resampling.temporal_block_length_months,
            minimum_spatial_blocks: manifest.resampling.minimum_spatial_blocks,
            minimum_temporal_blocks: manifest.resampling.minimum_temporal_blocks,
            replicates: manifest.resampling.replicates,
            minimum_valid_replicates: manifest.resampling.minimum_valid_replicates,
            seed: manifest.resampling.seed,
            confidence_level: manifest.resampling.confidence_level,
        },
        records: records.clone(),
    };
    let mut normalized_dataset_bytes = serde_json::to_vec_pretty(&dataset)
        .map_err(|error| vec![format!("cannot serialize normalized dataset: {error}")])?;
    normalized_dataset_bytes.push(b'\n');
    validate_dataset_json(&normalized_dataset_bytes).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| format!("normalized dataset failed PIRB validation: {error}"))
            .collect::<Vec<_>>()
    })?;

    let normalized_dataset_sha256 = sha256_hex(&normalized_dataset_bytes);
    let query_uris: Vec<String> = release_summaries
        .iter()
        .map(|summary| summary.query_uri.clone())
        .collect();
    let receipt = ViewsImportReceipt {
        receipt_version: "0.1".to_owned(),
        evaluator: "PIRB frozen VIEWS offline importer".to_owned(),
        result: "pass".to_owned(),
        source_commit: source_commit.to_owned(),
        source_commit_sha_valid: valid_commit_sha(source_commit),
        rustc_version: rustc_version.to_owned(),
        cargo_version: cargo_version.to_owned(),
        dependency_lock_sha256: sha256_hex(lock_bytes),
        dataset_id: manifest.dataset_id,
        evidence_class: manifest.evidence_class,
        partition: manifest.partition,
        forecast_archive_id: manifest.source.forecast_archive_id,
        forecast_archive_sha256,
        manifest_sha256,
        outcome_snapshot_sha256: outcome_sha256,
        spatial_block_map_sha256: block_map_sha256,
        normalized_dataset_sha256,
        analysis_unit: manifest.source.analysis_unit,
        forecast_variable: manifest.source.forecast_variable,
        codebook_variable_key: manifest.source.codebook_variable_key,
        target_definition: manifest.source.target_definition,
        threshold_definition: manifest.source.threshold_definition,
        lead_months: manifest.lead_months,
        release_count: release_summaries.len(),
        forecast_record_count,
        unit_count: unique_units.len(),
        unknown_outcome_count,
        query_uris,
        releases: release_summaries,
        execution_claim: "offline_source_mapping_and_normalized_contract_conformance_only"
            .to_owned(),
        claim_ceiling: "A passing receipt establishes deterministic mapping of supplied local response files, codebook and outcome snapshot into PIRB's normalized schema. It does not authenticate source truth, permission, license interpretation, outcome completeness, forecast skill, response effectiveness, causal impact, peace impact, or deployment safety.".to_owned(),
    };
    Ok(ViewsImportBundle {
        normalized_dataset_bytes,
        receipt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &[u8] =
        include_bytes!("../../../docs/fixtures/views-import-manifest-synthetic-v0.1.json");
    const OUTCOMES: &[u8] =
        include_bytes!("../../../docs/fixtures/views-outcomes-synthetic-v0.1.json");
    const BLOCK_MAP: &[u8] =
        include_bytes!("../../../docs/fixtures/views-spatial-block-map-synthetic-v0.1.json");
    const CODEBOOK: &[u8] =
        include_bytes!("../../../docs/fixtures/views-codebook-synthetic-v0.1.json");
    const RESPONSE_2020_03: &[u8] =
        include_bytes!("../../../docs/fixtures/views-api-response-synthetic-2020-03.json");
    const RESPONSE_2020_04: &[u8] =
        include_bytes!("../../../docs/fixtures/views-api-response-synthetic-2020-04.json");
    const RESPONSE_2020_05: &[u8] =
        include_bytes!("../../../docs/fixtures/views-api-response-synthetic-2020-05.json");
    const RESPONSE_2020_07: &[u8] =
        include_bytes!("../../../docs/fixtures/views-api-response-synthetic-2020-07.json");
    const RESPONSE_2020_08: &[u8] =
        include_bytes!("../../../docs/fixtures/views-api-response-synthetic-2020-08.json");
    const RESPONSE_2020_10: &[u8] =
        include_bytes!("../../../docs/fixtures/views-api-response-synthetic-2020-10.json");
    const RESPONSE_2020_11: &[u8] =
        include_bytes!("../../../docs/fixtures/views-api-response-synthetic-2020-11.json");
    const LOCK: &[u8] = b"synthetic-lock";

    fn source_map() -> BTreeMap<String, Vec<u8>> {
        [
            ("views-codebook-synthetic-v0.1.json", CODEBOOK),
            ("views-api-response-synthetic-2020-03.json", RESPONSE_2020_03),
            ("views-api-response-synthetic-2020-04.json", RESPONSE_2020_04),
            ("views-api-response-synthetic-2020-05.json", RESPONSE_2020_05),
            ("views-api-response-synthetic-2020-07.json", RESPONSE_2020_07),
            ("views-api-response-synthetic-2020-08.json", RESPONSE_2020_08),
            ("views-api-response-synthetic-2020-10.json", RESPONSE_2020_10),
            ("views-api-response-synthetic-2020-11.json", RESPONSE_2020_11),
        ]
        .into_iter()
        .map(|(path, bytes)| (path.to_owned(), bytes.to_vec()))
        .collect()
    }

    fn import() -> ViewsImportBundle {
        import_views_archive(
            MANIFEST,
            OUTCOMES,
            BLOCK_MAP,
            &source_map(),
            "abcdef0123456789abcdef0123456789abcdef01",
            "rustc 1.96.0",
            "cargo 1.96.0",
            LOCK,
        )
        .expect("synthetic VIEWS archive should import")
    }

    #[test]
    fn offline_archive_normalizes_all_release_rows_and_binds_hashes() {
        let bundle = import();
        let normalized: Value = serde_json::from_slice(&bundle.normalized_dataset_bytes).unwrap();
        assert_eq!(bundle.receipt.result, "pass");
        assert_eq!(bundle.receipt.release_count, 7);
        assert_eq!(bundle.receipt.forecast_record_count, 28);
        assert_eq!(bundle.receipt.unit_count, 4);
        assert_eq!(bundle.receipt.unknown_outcome_count, 1);
        assert_eq!(
            normalized["source"]["forecast_archive_sha256"],
            bundle.receipt.forecast_archive_sha256
        );
        assert_eq!(
            normalized["source"]["outcome_release_sha256"],
            bundle.receipt.outcome_snapshot_sha256
        );
        assert_eq!(
            normalized["resampling"]["spatial_block_map_sha256"],
            bundle.receipt.spatial_block_map_sha256
        );
        assert_eq!(bundle.receipt.partition, "eval");
        assert_eq!(normalized["lead_months"], 1);
        assert_eq!(normalized["records"].as_array().unwrap().len(), 28);
        assert_eq!(
            normalized["records"][0]["forecast_release_id"],
            "fatalities003_2020_03_t01"
        );
    }

    #[test]
    fn adapter_refuses_count_variable_even_if_numeric_values_are_bounded() {
        let mut manifest: Value = serde_json::from_slice(MANIFEST).unwrap();
        manifest["source"]["codebook_variable_key"] = Value::String("main_mean".to_owned());
        let bytes = serde_json::to_vec(&manifest).unwrap();
        let result = import_views_archive(
            &bytes,
            OUTCOMES,
            BLOCK_MAP,
            &source_map(),
            "abcdef0123456789abcdef0123456789abcdef01",
            "rustc 1.96.0",
            "cargo 1.96.0",
            LOCK,
        );
        assert!(
            result
                .unwrap_err()
                .iter()
                .any(|error| error.contains("only main_dich"))
        );
    }

    #[test]
    fn adapter_rejects_mismatched_model_metadata() {
        let mut sources = source_map();
        let mut page: Value = serde_json::from_slice(RESPONSE_2020_03).unwrap();
        page["metadata"]["models"][0] = Value::String("wrong_model".to_owned());
        sources.insert(
            "views-api-response-synthetic-2020-03.json".to_owned(),
            serde_json::to_vec(&page).unwrap(),
        );
        let result = import_views_archive(
            MANIFEST,
            OUTCOMES,
            BLOCK_MAP,
            &sources,
            "abcdef0123456789abcdef0123456789abcdef01",
            "rustc 1.96.0",
            "cargo 1.96.0",
            LOCK,
        );
        assert!(
            result
                .unwrap_err()
                .iter()
                .any(|error| error.contains("metadata model"))
        );
    }

    #[test]
    fn changing_probability_changes_archive_and_normalized_commitments() {
        let mut sources = source_map();
        let mut page: Value = serde_json::from_slice(RESPONSE_2020_03).unwrap();
        page["data"][0][3] = Value::from(0.99);
        sources.insert(
            "views-api-response-synthetic-2020-03.json".to_owned(),
            serde_json::to_vec(&page).unwrap(),
        );
        let changed = import_views_archive(
            MANIFEST,
            OUTCOMES,
            BLOCK_MAP,
            &sources,
            "abcdef0123456789abcdef0123456789abcdef01",
            "rustc 1.96.0",
            "cargo 1.96.0",
            LOCK,
        )
        .unwrap();
        let original = import();
        assert_ne!(
            changed.receipt.forecast_archive_sha256,
            original.receipt.forecast_archive_sha256
        );
        assert_ne!(
            changed.receipt.normalized_dataset_sha256,
            original.receipt.normalized_dataset_sha256
        );
    }

    #[test]
    fn path_traversal_is_rejected() {
        assert!(validate_relative_path("../secret.json").is_err());
        assert!(validate_relative_path("/etc/passwd").is_err());
        assert!(validate_relative_path("responses/response.json").is_ok());
    }

    #[test]
    fn target_month_must_match_frozen_run_horizon_and_query_filter() {
        let mut sources = source_map();
        let mut page: Value = serde_json::from_slice(RESPONSE_2020_03).unwrap();
        page["data"][0][1] = Value::from(5);
        sources.insert(
            "views-api-response-synthetic-2020-03.json".to_owned(),
            serde_json::to_vec(&page).unwrap(),
        );
        let result = import_views_archive(
            MANIFEST,
            OUTCOMES,
            BLOCK_MAP,
            &sources,
            "abcdef0123456789abcdef0123456789abcdef01",
            "rustc 1.96.0",
            "cargo 1.96.0",
            LOCK,
        );
        assert!(
            result
                .unwrap_err()
                .iter()
                .any(|error| error.contains("target month"))
        );
    }

    #[test]
    fn mismatched_outcome_snapshot_is_rejected() {
        let mut outcomes: Value = serde_json::from_slice(OUTCOMES).unwrap();
        outcomes["rows"][0]["unit_id"] = Value::String("unit-country-UNKNOWN".to_owned());
        let bytes = serde_json::to_vec(&outcomes).unwrap();
        let result = import_views_archive(
            MANIFEST,
            &bytes,
            BLOCK_MAP,
            &source_map(),
            "abcdef0123456789abcdef0123456789abcdef01",
            "rustc 1.96.0",
            "cargo 1.96.0",
            LOCK,
        );
        assert!(
            result
                .unwrap_err()
                .iter()
                .any(|error| error.contains("outcome snapshot has no row"))
        );
    }

    #[test]
    fn incomplete_response_pages_are_rejected() {
        let mut manifest: Value = serde_json::from_slice(MANIFEST).unwrap();
        manifest["releases"][0]["response_paths"] = Value::Array(vec![]);
        let bytes = serde_json::to_vec(&manifest).unwrap();
        let result = import_views_archive(
            &bytes,
            OUTCOMES,
            BLOCK_MAP,
            &source_map(),
            "abcdef0123456789abcdef0123456789abcdef01",
            "rustc 1.96.0",
            "cargo 1.96.0",
            LOCK,
        );
        assert!(result.is_err());
    }
}
