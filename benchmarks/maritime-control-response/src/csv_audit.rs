#![forbid(unsafe_code)]

//! Strict, dependency-light CSV payload audit for the separate Marine Engine Fault dataset.
//!
//! The audit preserves raw missing values, verifies per-file SHA-256, checks
//! the three-row header and 70/73-column layouts, validates scenario labels,
//! and records time ordering. It does not execute the publisher's normalization
//! script, infer fault onset from approximate prose, or merge this dataset's
//! labels with LiU-ICE.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub const MARINE_CSV_HEADER_ROWS: usize = 3;
pub const MARINE_CSV_BASELINE_COLUMNS: usize = 70;
pub const MARINE_CSV_SCENARIO_COLUMNS: usize = 73;
pub const MARINE_EXPECTED_CSV_FILES: usize = 16;
pub const MARINE_EXPECTED_BASELINE_FILES: usize = 1;
pub const MARINE_EXPECTED_SCENARIO_FILES: usize = 15;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarineCsvRole {
    BaselineReference,
    FaultScenario,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MarineCsvFileAudit {
    pub relative_path: String,
    pub sha256: String,
    pub byte_length: u64,
    pub data_rows: u64,
    pub column_count: usize,
    pub role: MarineCsvRole,
    pub anomaly_free_rows: Option<u64>,
    pub anomaly_rows: Option<u64>,
    pub event_latency_eligible: bool,
    /// Zero-based column selected from the publisher's role-specific header.
    pub time_column_index: usize,
    pub time_column_name: String,
    pub time_column_unit: String,
    pub time_strictly_increasing: bool,
    pub missing_channel_columns: Vec<String>,
    pub fully_missing_column_count: usize,
    pub empty_cell_count: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MarineCsvAuditReport {
    pub schema_version: String,
    pub dataset_doi: String,
    pub archive_sha256: String,
    /// All CSV members in the ZIP, including index/dictionary metadata exports.
    pub archive_csv_entry_count: usize,
    /// CSV members treated as dataset-index or variable-dictionary metadata.
    pub metadata_csv_artifact_count: usize,
    /// Recording payload CSVs only (the published one-baseline/fifteen-scenario inventory).
    pub csv_file_count: usize,
    pub baseline_file_count: usize,
    pub scenario_file_count: usize,
    pub total_data_rows: u64,
    pub fully_missing_channel_file_counts: BTreeMap<String, usize>,
    pub anomaly_only_scenario_files: Vec<String>,
    pub event_latency_eligible_scenario_files: Vec<String>,
    pub metadata_artifact_sha256: BTreeMap<String, String>,
    pub files: Vec<MarineCsvFileAudit>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsvAuditError {
    Io(String),
    InvalidArchiveDigest,
    InvalidCsvUtf8(String),
    CsvSyntax(String),
    EmptyCsv(String),
    InvalidHeaderRows(String),
    InvalidColumnCount(String),
    InconsistentRowWidth(String),
    InvalidHeaderSemantics(String),
    InvalidTimeValue(String),
    NonMonotonicTime(String),
    InvalidAnomalyLabel(String),
    InvalidRelativePath(String),
    WrongCsvInventory { expected: usize, actual: usize },
    WrongRoleInventory { baseline: usize, scenario: usize },
    MissingMetadataArtifact(String),
    DuplicateMetadataArtifact(String),
    Serialization,
}

impl fmt::Display for CsvAuditError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "dataset I/O failed: {message}"),
            Self::InvalidArchiveDigest => write!(
                formatter,
                "archive SHA-256 must be 64 lowercase hexadecimal characters"
            ),
            Self::InvalidCsvUtf8(path) => write!(formatter, "CSV is not valid UTF-8: {path}"),
            Self::CsvSyntax(path) => {
                write!(formatter, "CSV quoting or record syntax is invalid: {path}")
            }
            Self::EmptyCsv(path) => {
                write!(formatter, "CSV contains no complete data records: {path}")
            }
            Self::InvalidHeaderRows(path) => write!(
                formatter,
                "CSV does not contain exactly three complete header records: {path}"
            ),
            Self::InvalidColumnCount(path) => write!(
                formatter,
                "CSV column count is not the published 70/73 shape: {path}"
            ),
            Self::InconsistentRowWidth(path) => write!(
                formatter,
                "CSV header or data record width changes within file: {path}"
            ),
            Self::InvalidHeaderSemantics(path) => write!(
                formatter,
                "CSV role-specific time/label headers or time units do not match the published schema: {path}"
            ),
            Self::InvalidTimeValue(path) => write!(
                formatter,
                "selected time column contains a missing, non-numeric or non-finite value: {path}"
            ),
            Self::NonMonotonicTime(path) => write!(
                formatter,
                "selected time column is not strictly increasing: {path}"
            ),
            Self::InvalidAnomalyLabel(path) => write!(
                formatter,
                "Anomaly State contains a value other than 0 or 1: {path}"
            ),
            Self::InvalidRelativePath(path) => write!(
                formatter,
                "dataset path is empty, absolute or traverses outside the root: {path}"
            ),
            Self::WrongCsvInventory { expected, actual } => write!(
                formatter,
                "expected {expected} recording-data CSV files after metadata-export filtering, found {actual}"
            ),
            Self::WrongRoleInventory { baseline, scenario } => write!(
                formatter,
                "expected one 70-column baseline and fifteen 73-column scenarios; found {baseline} and {scenario}"
            ),
            Self::MissingMetadataArtifact(name) => write!(
                formatter,
                "required metadata artifact was not found: {name}"
            ),
            Self::DuplicateMetadataArtifact(name) => {
                write!(formatter, "multiple files match metadata artifact: {name}")
            }
            Self::Serialization => write!(formatter, "failed to serialize audit evidence"),
        }
    }
}

impl std::error::Error for CsvAuditError {}

/// Audit the extracted archive directory. The archive digest is supplied by
/// the caller after independently hashing the downloaded ZIP bytes.
pub fn audit_marine_dataset_directory(
    root: &Path,
    archive_sha256: &str,
) -> Result<MarineCsvAuditReport, CsvAuditError> {
    if !is_sha256_hex(archive_sha256) {
        return Err(CsvAuditError::InvalidArchiveDigest);
    }
    if !root.is_dir() {
        return Err(CsvAuditError::Io("dataset root is not a directory".into()));
    }

    let mut all_files = Vec::new();
    collect_files(root, root, &mut all_files)?;
    all_files.sort();

    let metadata_artifact_sha256 = audit_metadata_artifacts(&all_files)?;
    let mut archive_csv_paths = all_files
        .iter()
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("csv"))
        })
        .cloned()
        .collect::<Vec<_>>();
    archive_csv_paths.sort();
    let archive_csv_entry_count = archive_csv_paths.len();
    let metadata_csv_artifact_count = archive_csv_paths
        .iter()
        .filter(|path| is_metadata_csv_artifact(path))
        .count();
    let csv_paths = archive_csv_paths
        .into_iter()
        .filter(|path| !is_metadata_csv_artifact(path))
        .collect::<Vec<_>>();

    if csv_paths.len() != MARINE_EXPECTED_CSV_FILES {
        return Err(CsvAuditError::WrongCsvInventory {
            expected: MARINE_EXPECTED_CSV_FILES,
            actual: csv_paths.len(),
        });
    }

    let mut files = Vec::with_capacity(csv_paths.len());
    for path in &csv_paths {
        let relative_path = safe_relative_path(root, path)?;
        let bytes = fs::read(path).map_err(|error| CsvAuditError::Io(error.to_string()))?;
        files.push(audit_marine_csv_bytes(&relative_path, &bytes)?);
    }
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    let baseline_file_count = files
        .iter()
        .filter(|file| file.role == MarineCsvRole::BaselineReference)
        .count();
    let scenario_file_count = files
        .iter()
        .filter(|file| file.role == MarineCsvRole::FaultScenario)
        .count();
    if baseline_file_count != MARINE_EXPECTED_BASELINE_FILES
        || scenario_file_count != MARINE_EXPECTED_SCENARIO_FILES
    {
        return Err(CsvAuditError::WrongRoleInventory {
            baseline: baseline_file_count,
            scenario: scenario_file_count,
        });
    }

    let mut fully_missing_channel_file_counts = BTreeMap::new();
    let mut anomaly_only_scenario_files = Vec::new();
    let mut event_latency_eligible_scenario_files = Vec::new();
    let mut total_data_rows = 0u64;
    for file in &files {
        total_data_rows += file.data_rows;
        for channel in &file.missing_channel_columns {
            *fully_missing_channel_file_counts
                .entry(channel.clone())
                .or_insert(0) += 1;
        }
        if file.role == MarineCsvRole::FaultScenario {
            if file.anomaly_free_rows == Some(0) {
                anomaly_only_scenario_files.push(file.relative_path.clone());
            }
            if file.event_latency_eligible {
                event_latency_eligible_scenario_files.push(file.relative_path.clone());
            }
        }
    }

    Ok(MarineCsvAuditReport {
        schema_version: "symtropy-marine-csv-audit-v0.2".into(),
        dataset_doi: "10.5281/zenodo.19857425".into(),
        archive_sha256: archive_sha256.to_owned(),
        archive_csv_entry_count,
        metadata_csv_artifact_count,
        csv_file_count: files.len(),
        baseline_file_count,
        scenario_file_count,
        total_data_rows,
        fully_missing_channel_file_counts,
        anomaly_only_scenario_files,
        event_latency_eligible_scenario_files,
        metadata_artifact_sha256,
        files,
        claim_ceiling: "Extracted CSV inventory and local file structure audited against the published 3-row/70-or-73-column record. This is not independent verification of the physical engine, fault severity, sensor calibration, publisher labels, cross-engine transfer or dataset scientific validity.".into(),
    })
}

/// Audit one CSV payload. The file name is retained as an identity only; fault
/// class, load or injection timestamp are not inferred from its name.
pub fn audit_marine_csv_bytes(
    relative_path: &str,
    bytes: &[u8],
) -> Result<MarineCsvFileAudit, CsvAuditError> {
    validate_relative_path(relative_path)?;
    let content = std::str::from_utf8(bytes)
        .map_err(|_| CsvAuditError::InvalidCsvUtf8(relative_path.to_owned()))?;
    let records = parse_csv_records(content.as_bytes(), relative_path)?;
    if records.len() <= MARINE_CSV_HEADER_ROWS {
        return Err(CsvAuditError::EmptyCsv(relative_path.to_owned()));
    }

    let column_count = records[0].len();
    let role = match column_count {
        MARINE_CSV_BASELINE_COLUMNS => MarineCsvRole::BaselineReference,
        MARINE_CSV_SCENARIO_COLUMNS => MarineCsvRole::FaultScenario,
        _ => return Err(CsvAuditError::InvalidColumnCount(relative_path.to_owned())),
    };
    for header in records.iter().take(MARINE_CSV_HEADER_ROWS) {
        if header.len() != column_count {
            return Err(CsvAuditError::InvalidHeaderRows(relative_path.to_owned()));
        }
    }
    if records
        .iter()
        .skip(MARINE_CSV_HEADER_ROWS)
        .any(|record| record.len() != column_count)
    {
        return Err(CsvAuditError::InconsistentRowWidth(
            relative_path.to_owned(),
        ));
    }

    let headers = records[0]
        .iter()
        .map(|value| normalize_header(value))
        .collect::<Vec<_>>();
    let (time_column_index, time_column_name, time_column_unit) = match role {
        MarineCsvRole::BaselineReference => {
            // Reference_Data.csv has its own publisher schema: one "Time"
            // column in seconds. It does not contain the scenario-only
            // Time_abs/Time_rel/Anomaly State prefix.
            if headers.first().map(String::as_str) != Some("Time")
                || normalize_header(&records[2][0]) != "sec"
            {
                return Err(CsvAuditError::InvalidHeaderSemantics(
                    relative_path.to_owned(),
                ));
            }
            (0usize, "Time".to_owned(), "sec".to_owned())
        }
        MarineCsvRole::FaultScenario => {
            if headers.first().map(String::as_str) != Some("Time_abs")
                || headers.get(1).map(String::as_str) != Some("Time_rel")
                || headers.get(2).map(String::as_str) != Some("Anomaly State")
                || normalize_header(&records[2][0]) != "sec"
                || normalize_header(&records[2][1]) != "sec"
            {
                return Err(CsvAuditError::InvalidHeaderSemantics(
                    relative_path.to_owned(),
                ));
            }
            (1usize, "Time_rel".to_owned(), "sec".to_owned())
        }
    };

    let data_records = &records[MARINE_CSV_HEADER_ROWS..];
    let data_rows = data_records.len() as u64;
    let mut empty_cells_per_column = vec![0u64; column_count];
    let mut empty_cell_count = 0u64;
    let mut previous_time = None;
    let mut anomaly_free_rows = 0u64;
    let mut anomaly_rows = 0u64;

    for record in data_records {
        for (index, value) in record.iter().enumerate() {
            if value.trim().is_empty() {
                empty_cells_per_column[index] += 1;
                empty_cell_count += 1;
            }
        }

        let time_text = record[time_column_index].trim();
        let time_value = time_text
            .parse::<f64>()
            .map_err(|_| CsvAuditError::InvalidTimeValue(relative_path.to_owned()))?;
        if !time_value.is_finite() {
            return Err(CsvAuditError::InvalidTimeValue(relative_path.to_owned()));
        }
        if previous_time.is_some_and(|previous| time_value <= previous) {
            return Err(CsvAuditError::NonMonotonicTime(relative_path.to_owned()));
        }
        previous_time = Some(time_value);

        if role == MarineCsvRole::FaultScenario {
            match record[2].trim() {
                "0" => anomaly_free_rows += 1,
                "1" => anomaly_rows += 1,
                _ => {
                    return Err(CsvAuditError::InvalidAnomalyLabel(relative_path.to_owned()));
                }
            }
        }
    }

    let fully_missing_columns = empty_cells_per_column
        .iter()
        .enumerate()
        .filter(|(_, missing_count)| **missing_count == data_rows)
        .map(|(index, _)| headers[index].clone())
        .collect::<Vec<_>>();
    let missing_channel_columns = fully_missing_columns
        .iter()
        .filter(|name| {
            name.as_str() == "Compressor Filter Loss" || name.as_str() == "Turbine Back Pressure"
        })
        .cloned()
        .collect::<Vec<_>>();

    let input_sha256 = sha256_bytes(bytes);
    Ok(MarineCsvFileAudit {
        relative_path: relative_path.to_owned(),
        sha256: input_sha256,
        byte_length: bytes.len() as u64,
        data_rows,
        column_count,
        role,
        anomaly_free_rows: (role == MarineCsvRole::FaultScenario).then_some(anomaly_free_rows),
        anomaly_rows: (role == MarineCsvRole::FaultScenario).then_some(anomaly_rows),
        event_latency_eligible: role == MarineCsvRole::FaultScenario
            && anomaly_free_rows > 0
            && anomaly_rows > 0,
        time_column_index,
        time_column_name,
        time_column_unit,
        time_strictly_increasing: true,
        missing_channel_columns,
        fully_missing_column_count: fully_missing_columns.len(),
        empty_cell_count,
    })
}

fn parse_csv_records(bytes: &[u8], relative_path: &str) -> Result<Vec<Vec<String>>, CsvAuditError> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum State {
        FieldStart,
        Unquoted,
        Quoted,
        AfterQuote,
    }

    let mut records = Vec::new();
    let mut fields = Vec::<String>::new();
    let mut field = Vec::<u8>::new();
    let mut state = State::FieldStart;
    let mut index = 0usize;

    while index < bytes.len() {
        let byte = bytes[index];
        match state {
            State::FieldStart => match byte {
                b'"' => state = State::Quoted,
                b',' => {
                    fields.push(String::new());
                }
                b'\n' => {
                    fields.push(String::new());
                    records.push(std::mem::take(&mut fields));
                }
                b'\r' => {
                    if bytes.get(index + 1) != Some(&b'\n') {
                        return Err(CsvAuditError::CsvSyntax(relative_path.to_owned()));
                    }
                    fields.push(String::new());
                    records.push(std::mem::take(&mut fields));
                    index += 1;
                }
                _ => {
                    field.push(byte);
                    state = State::Unquoted;
                }
            },
            State::Unquoted => match byte {
                b',' => {
                    fields.push(decode_field(&mut field, relative_path)?);
                    state = State::FieldStart;
                }
                b'\n' => {
                    fields.push(decode_field(&mut field, relative_path)?);
                    records.push(std::mem::take(&mut fields));
                    state = State::FieldStart;
                }
                b'\r' => {
                    if bytes.get(index + 1) != Some(&b'\n') {
                        return Err(CsvAuditError::CsvSyntax(relative_path.to_owned()));
                    }
                    fields.push(decode_field(&mut field, relative_path)?);
                    records.push(std::mem::take(&mut fields));
                    state = State::FieldStart;
                    index += 1;
                }
                b'"' => return Err(CsvAuditError::CsvSyntax(relative_path.to_owned())),
                _ => field.push(byte),
            },
            State::Quoted => match byte {
                b'"' if bytes.get(index + 1) == Some(&b'"') => {
                    field.push(b'"');
                    index += 1;
                }
                b'"' => state = State::AfterQuote,
                _ => field.push(byte),
            },
            State::AfterQuote => match byte {
                b',' => {
                    fields.push(decode_field(&mut field, relative_path)?);
                    state = State::FieldStart;
                }
                b'\n' => {
                    fields.push(decode_field(&mut field, relative_path)?);
                    records.push(std::mem::take(&mut fields));
                    state = State::FieldStart;
                }
                b'\r' => {
                    if bytes.get(index + 1) != Some(&b'\n') {
                        return Err(CsvAuditError::CsvSyntax(relative_path.to_owned()));
                    }
                    fields.push(decode_field(&mut field, relative_path)?);
                    records.push(std::mem::take(&mut fields));
                    state = State::FieldStart;
                    index += 1;
                }
                _ => return Err(CsvAuditError::CsvSyntax(relative_path.to_owned())),
            },
        }
        index += 1;
    }

    if state == State::Quoted {
        return Err(CsvAuditError::CsvSyntax(relative_path.to_owned()));
    }
    if state != State::FieldStart || !fields.is_empty() || !field.is_empty() {
        fields.push(decode_field(&mut field, relative_path)?);
        records.push(fields);
    }
    Ok(records)
}

fn decode_field(field: &mut Vec<u8>, relative_path: &str) -> Result<String, CsvAuditError> {
    let bytes = std::mem::take(field);
    String::from_utf8(bytes).map_err(|_| CsvAuditError::InvalidCsvUtf8(relative_path.to_owned()))
}

fn normalize_header(value: &str) -> String {
    value.trim().trim_start_matches('\u{feff}').to_owned()
}

fn collect_files(
    root: &Path,
    directory: &Path,
    output: &mut Vec<PathBuf>,
) -> Result<(), CsvAuditError> {
    let entries = fs::read_dir(directory).map_err(|error| CsvAuditError::Io(error.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|error| CsvAuditError::Io(error.to_string()))?;
        let file_type = entry
            .file_type()
            .map_err(|error| CsvAuditError::Io(error.to_string()))?;
        let path = entry.path();
        if file_type.is_symlink() {
            return Err(CsvAuditError::InvalidRelativePath(
                path.display().to_string(),
            ));
        }
        if file_type.is_dir() {
            collect_files(root, &path, output)?;
        } else if file_type.is_file() {
            let _ = safe_relative_path(root, &path)?;
            output.push(path);
        }
    }
    Ok(())
}

/// Metadata exports are CSV files in this ZIP but are not engine recordings.
/// Keep their hashes in metadata_artifact_sha256 and exclude them from the
/// one-baseline/fifteen-scenario data inventory.
fn is_metadata_csv_artifact(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let lower = name.to_ascii_lowercase();
    lower.contains("dataset_index")
        || lower.contains("variable_dictionary")
        || lower.contains("normaliz")
        || lower.contains("normalis")
}

fn audit_metadata_artifacts(paths: &[PathBuf]) -> Result<BTreeMap<String, String>, CsvAuditError> {
    let categories = [
        ("dataset_index", &["dataset_index"][..]),
        ("variable_dictionary", &["variable_dictionary"][..]),
        ("normalization_script", &["normaliz", "normalis"][..]),
    ];
    let mut result = BTreeMap::new();
    for (artifact_name, needles) in categories {
        let matches = paths
            .iter()
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        let lower = name.to_ascii_lowercase();
                        needles.iter().any(|needle| lower.contains(needle))
                    })
            })
            .collect::<Vec<_>>();
        if matches.is_empty() {
            return Err(CsvAuditError::MissingMetadataArtifact(
                artifact_name.to_owned(),
            ));
        }
        if matches.len() != 1 {
            return Err(CsvAuditError::DuplicateMetadataArtifact(
                artifact_name.to_owned(),
            ));
        }
        let bytes = fs::read(matches[0]).map_err(|error| CsvAuditError::Io(error.to_string()))?;
        result.insert(artifact_name.to_owned(), sha256_bytes(&bytes));
    }
    Ok(result)
}

fn safe_relative_path(root: &Path, path: &Path) -> Result<String, CsvAuditError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| CsvAuditError::InvalidRelativePath(path.display().to_string()))?;
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::RootDir
            )
        })
    {
        return Err(CsvAuditError::InvalidRelativePath(
            path.display().to_string(),
        ));
    }
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn validate_relative_path(relative_path: &str) -> Result<(), CsvAuditError> {
    let path = Path::new(relative_path);
    if relative_path.trim().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::RootDir
            )
        })
    {
        return Err(CsvAuditError::InvalidRelativePath(relative_path.to_owned()));
    }
    Ok(())
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_csv(columns: usize, labels: Option<[&str; 2]>) -> Vec<u8> {
        let mut names = (0..columns)
            .map(|index| format!("Channel {index}"))
            .collect::<Vec<_>>();
        if labels.is_some() {
            names[0] = "Time_abs".into();
            names[1] = "Time_rel".into();
            names[2] = "Anomaly State".into();
        } else {
            names[0] = "Time".into();
        }
        let mut row_one = (0..columns)
            .map(|index| format!("signal_{index}"))
            .collect::<Vec<_>>();
        let mut row_two = (0..columns).map(|_| "unit".to_owned()).collect::<Vec<_>>();
        if labels.is_some() {
            row_one[0] = String::new();
            row_one[1] = String::new();
            row_two[0] = "sec".into();
            row_two[1] = "sec".into();
        } else {
            row_one[0] = "T".into();
            row_two[0] = "sec".into();
        }
        let mut rows = vec![names, row_one, row_two];
        for (index, time) in ["0.0", "0.05"].iter().enumerate() {
            let mut row = vec!["2026-01-01T00:00:00Z".to_owned(); columns];
            if let Some(label_values) = labels {
                row[0] = "2026-01-01T00:00:00Z".into();
                row[1] = (*time).into();
                row[2] = label_values[index].into();
            } else {
                row[0] = (*time).into();
            }
            rows.push(row);
        }
        rows.into_iter()
            .map(|row| row.join(","))
            .collect::<Vec<_>>()
            .join("\n")
            .into_bytes()
    }

    #[test]
    fn metadata_csv_exports_are_not_misclassified_as_engine_recordings() {
        assert!(is_metadata_csv_artifact(Path::new("dataset_index.csv")));
        assert!(is_metadata_csv_artifact(Path::new(
            "variable_dictionary.csv"
        )));
        assert!(is_metadata_csv_artifact(Path::new(
            "normalization_script.csv"
        )));
        assert!(!is_metadata_csv_artifact(Path::new("scenario_01.csv")));
    }

    #[test]
    fn scenario_csv_validates_three_headers_binary_labels_and_time() {
        let bytes = fixture_csv(73, Some(["0", "1"]));
        let result = audit_marine_csv_bytes("scenario.csv", &bytes).unwrap();
        assert_eq!(result.role, MarineCsvRole::FaultScenario);
        assert_eq!(result.data_rows, 2);
        assert_eq!(result.anomaly_free_rows, Some(1));
        assert_eq!(result.anomaly_rows, Some(1));
        assert!(result.event_latency_eligible);
        assert_eq!(result.column_count, 73);
        assert_eq!(result.time_column_index, 1);
        assert_eq!(result.time_column_name, "Time_rel");
        assert_eq!(result.time_column_unit, "sec");
        assert!(result.time_strictly_increasing);
        assert_eq!(result.sha256, sha256_bytes(&bytes));
    }

    #[test]
    fn baseline_csv_has_no_fault_label_and_uses_seventy_columns() {
        let bytes = fixture_csv(70, None);
        let result = audit_marine_csv_bytes("baseline.csv", &bytes).unwrap();
        assert_eq!(result.role, MarineCsvRole::BaselineReference);
        assert_eq!(result.column_count, 70);
        assert_eq!(result.time_column_index, 0);
        assert_eq!(result.time_column_name, "Time");
        assert_eq!(result.time_column_unit, "sec");
        assert!(result.time_strictly_increasing);
        assert_eq!(result.anomaly_free_rows, None);
        assert!(!result.event_latency_eligible);
    }

    #[test]
    fn baseline_and_scenario_time_columns_are_not_interchangeable() {
        let mut wrong_baseline = fixture_csv(70, None);
        let source = String::from_utf8(wrong_baseline.clone()).unwrap();
        let mut rows = source.lines().map(str::to_owned).collect::<Vec<_>>();
        let mut header = rows[0].split(',').map(str::to_owned).collect::<Vec<_>>();
        header[0] = "Time_rel".into();
        rows[0] = header.join(",");
        wrong_baseline = rows.join("\n").into_bytes();
        assert_eq!(
            audit_marine_csv_bytes("wrong-baseline.csv", &wrong_baseline),
            Err(CsvAuditError::InvalidHeaderSemantics(
                "wrong-baseline.csv".into()
            ))
        );

        let mut wrong_units = fixture_csv(73, Some(["0", "1"]));
        let source = String::from_utf8(wrong_units.clone()).unwrap();
        let mut rows = source.lines().map(str::to_owned).collect::<Vec<_>>();
        let mut units = rows[2].split(',').map(str::to_owned).collect::<Vec<_>>();
        units[1] = "rpm".into();
        rows[2] = units.join(",");
        wrong_units = rows.join("\n").into_bytes();
        assert_eq!(
            audit_marine_csv_bytes("wrong-units.csv", &wrong_units),
            Err(CsvAuditError::InvalidHeaderSemantics(
                "wrong-units.csv".into()
            ))
        );
    }

    #[test]
    fn anomaly_only_injection_file_is_not_latency_eligible() {
        let bytes = fixture_csv(73, Some(["1", "1"]));
        let result = audit_marine_csv_bytes("nozzle.csv", &bytes).unwrap();
        assert_eq!(result.anomaly_free_rows, Some(0));
        assert_eq!(result.anomaly_rows, Some(2));
        assert!(!result.event_latency_eligible);
    }

    #[test]
    fn full_column_missingness_is_retained_not_imputed() {
        let mut bytes = fixture_csv(73, Some(["0", "1"]));
        let text = String::from_utf8(bytes.clone()).unwrap();
        let mut records = text.lines().map(str::to_owned).collect::<Vec<_>>();
        for record in records.iter_mut().skip(3) {
            let mut fields = record.split(',').map(str::to_owned).collect::<Vec<_>>();
            fields[10].clear();
            *record = fields.join(",");
        }
        bytes = records.join("\n").into_bytes();
        let result = audit_marine_csv_bytes("missing.csv", &bytes).unwrap();
        assert_eq!(result.fully_missing_column_count, 1);
        assert_eq!(result.empty_cell_count, 2);
    }

    #[test]
    fn quoted_commas_and_escaped_quotes_are_parsed_without_shifting_columns() {
        let csv = br#"Time_abs,Time_rel,Anomaly State,Description
symbol,time,label,description
s,s,1,text
t0,0.0,0,"pump, ""service"" state"
t1,0.1,1,normal
"#;
        let records = parse_csv_records(csv, "quoted.csv").unwrap();
        assert_eq!(records.len(), 5);
        assert_eq!(records[3][3], "pump, \"service\" state");
    }

    #[test]
    fn malformed_quotes_and_non_binary_labels_fail_closed() {
        assert!(matches!(
            parse_csv_records(b"a,b\n1,\"unterminated", "bad.csv"),
            Err(CsvAuditError::CsvSyntax(_))
        ));
        let bytes = fixture_csv(73, Some(["0", "unknown"]));
        assert_eq!(
            audit_marine_csv_bytes("bad-label.csv", &bytes),
            Err(CsvAuditError::InvalidAnomalyLabel("bad-label.csv".into()))
        );
    }

    #[test]
    fn non_monotonic_time_and_wrong_column_count_are_rejected() {
        let mut bytes = fixture_csv(73, Some(["0", "1"]));
        let text = String::from_utf8(bytes.clone()).unwrap();
        let mut records = text.lines().map(str::to_owned).collect::<Vec<_>>();
        let mut second_data = records[4].split(',').map(str::to_owned).collect::<Vec<_>>();
        second_data[1] = "0.0".into();
        records[4] = second_data.join(",");
        bytes = records.join("\n").into_bytes();
        assert_eq!(
            audit_marine_csv_bytes("time.csv", &bytes),
            Err(CsvAuditError::NonMonotonicTime("time.csv".into()))
        );

        assert_eq!(
            audit_marine_csv_bytes("wrong.csv", &fixture_csv(72, None)),
            Err(CsvAuditError::InvalidColumnCount("wrong.csv".into()))
        );
    }

    #[test]
    fn unsafe_relative_paths_are_rejected() {
        assert_eq!(
            audit_marine_csv_bytes("../escape.csv", &fixture_csv(70, None)),
            Err(CsvAuditError::InvalidRelativePath("../escape.csv".into()))
        );
        assert_eq!(
            audit_marine_csv_bytes("/absolute.csv", &fixture_csv(70, None)),
            Err(CsvAuditError::InvalidRelativePath("/absolute.csv".into()))
        );
    }
}
