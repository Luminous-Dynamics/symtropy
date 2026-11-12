#![forbid(unsafe_code)]

//! Offline audit for the public Aalto marine-engine fault dataset.
//!
//! The tool audits an already-extracted dataset directory. It does not download
//! or extract archives. Archive SHA-256 is computed when --archive is supplied;
//! publisher MD5 verification is attempted only when a local md5sum, md5, or
//! OpenSSL executable is available. Split assignments are never auto-generated.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::process::ExitCode;

const MAX_FILE_BYTES: u64 = 128 * 1024 * 1024;
const SCENARIO_COLUMNS: usize = 73;
const BASELINE_COLUMNS: usize = 70;
const HEADER_ROWS: usize = 3;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InventoryEntry {
    path: String,
    reported_size: String,
    role: String,
}

#[derive(Clone, Debug, Serialize)]
struct ArchiveAudit {
    path: String,
    size_bytes: u64,
    sha256: String,
    publisher_md5: Option<String>,
    observed_md5: Option<String>,
    publisher_md5_matches: Option<bool>,
    md5_tool_available: bool,
    zip_signature_valid: bool,
}

#[derive(Clone, Debug, Serialize)]
struct MissingColumn {
    index: usize,
    name: String,
    unit: String,
    missing_cells: u64,
}

#[derive(Clone, Debug, Serialize)]
struct CsvAudit {
    header_rows: usize,
    column_count: usize,
    data_rows: u64,
    header_names: Vec<String>,
    shorthand_symbols: Vec<String>,
    units: Vec<String>,
    anomaly_label_column: Option<usize>,
    time_abs_column: usize,
    time_abs_nonempty_rows: u64,
    time_abs_empty_rows: u64,
    time_abs_duplicate_rows: u64,
    first_time_abs: Option<String>,
    last_time_abs: Option<String>,
    anomaly_zero_rows: u64,
    anomaly_one_rows: u64,
    anomaly_invalid_rows: u64,
    anomaly_onset_transitions: u64,
    anomaly_only_expected: bool,
    anomaly_only_observed: Option<bool>,
    anomaly_onset_evaluable: Option<bool>,
    time_rel_column: Option<usize>,
    time_rel_numeric_rows: u64,
    time_rel_non_numeric_rows: u64,
    time_rel_backsteps: u64,
    time_rel_duplicates: u64,
    first_time_rel: Option<String>,
    last_time_rel: Option<String>,
    missing_header_names: Vec<usize>,
    missing_shorthand_symbols: Vec<usize>,
    missing_units: Vec<usize>,
    missing_columns: Vec<MissingColumn>,
}

#[derive(Clone, Debug, Serialize)]
struct FileAudit {
    path: String,
    role: String,
    exists: bool,
    size_bytes: Option<u64>,
    preview_reported_size: String,
    preview_size_delta_bytes: Option<i64>,
    sha256: Option<String>,
    utf8_valid: bool,
    metadata_csv_rows: Option<usize>,
    csv: Option<CsvAudit>,
    errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct SplitGroupTemplate {
    group_id: String,
    source_file: String,
    group_type: String,
    assignment: Option<String>,
    assignment_state: String,
    note: String,
}

#[derive(Clone, Debug, Serialize)]
struct SplitTemplate {
    schema_version: String,
    assignment_policy: String,
    group_key: String,
    random_window_split_allowed: bool,
    groups: Vec<SplitGroupTemplate>,
}

#[derive(Clone, Debug, Serialize)]
struct DatasetAuditReport {
    report_version: String,
    dataset_doi: Option<String>,
    dataset_version: Option<String>,
    audit_state: String,
    claim_ceiling: Option<String>,
    archive: Option<ArchiveAudit>,
    files_expected: usize,
    files_found: usize,
    scenario_csvs_audited: usize,
    baseline_csvs_audited: usize,
    total_data_rows: u64,
    file_audits: Vec<FileAudit>,
    split_template: SplitTemplate,
    errors: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Debug)]
struct Cli {
    dataset_root: PathBuf,
    manifest_path: PathBuf,
    archive_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
}

fn usage() -> &'static str {
    "usage: marine_engine_dataset_audit <extracted-dataset-root> <dataset-manifest.json> [--archive <publisher-verified.zip>] [--output <report.json>] (archive required for a successful audit)"
}

fn parse_args() -> Result<Cli, String> {
    let mut args = env::args_os().skip(1);
    let dataset_root = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| usage().to_string())?;
    let manifest_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| usage().to_string())?;
    let mut archive_path = None;
    let mut output_path = None;

    while let Some(flag) = args.next() {
        let flag = flag.to_string_lossy();
        let value = args
            .next()
            .map(PathBuf::from)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_ref() {
            "--archive" => archive_path = Some(value),
            "--output" => output_path = Some(value),
            _ => return Err(format!("unknown option: {flag}")),
        }
    }

    Ok(Cli {
        dataset_root,
        manifest_path,
        archive_path,
        output_path,
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn read_file_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let file =
        File::open(path).map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let mut limited = file.take(MAX_FILE_BYTES + 1);
    let mut bytes = Vec::new();
    limited
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(format!(
            "file exceeds safety limit of {MAX_FILE_BYTES} bytes"
        ));
    }
    Ok(bytes)
}

fn safe_relative_path(value: &str) -> Result<PathBuf, String> {
    let path = Path::new(value);
    if path.as_os_str().is_empty() {
        return Err("empty path".into());
    }
    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(format!(
                "non-relative or traversing path is not allowed: {value}"
            ));
        }
    }
    Ok(path.to_path_buf())
}

fn resolve_inventory_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let safe = safe_relative_path(relative)?;
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("cannot resolve dataset root {}: {error}", root.display()))?;
    let direct = root.join(&safe);
    if direct.is_file() {
        let canonical = fs::canonicalize(&direct)
            .map_err(|error| format!("cannot resolve {}: {error}", direct.display()))?;
        if !canonical.starts_with(&canonical_root) {
            return Err(format!(
                "inventory path escapes dataset root through a symlink: {relative}"
            ));
        }
        return Ok(canonical);
    }

    let stripped = relative
        .strip_prefix("Marine_Engine_Fault_Data/")
        .unwrap_or(relative);
    let stripped_path = safe_relative_path(stripped)?;
    let alternate = root.join(stripped_path);
    if alternate.is_file() {
        let canonical = fs::canonicalize(&alternate)
            .map_err(|error| format!("cannot resolve {}: {error}", alternate.display()))?;
        if !canonical.starts_with(&canonical_root) {
            return Err(format!(
                "inventory path escapes dataset root through a symlink: {relative}"
            ));
        }
        return Ok(canonical);
    }

    Err(format!("file not found beneath dataset root: {relative}"))
}

fn is_missing_value(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "" | "na" | "n/a" | "nan" | "null" | "none"
    )
}

fn normalize_header(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn header_index(headers: &[Vec<String>], wanted: &str) -> Option<usize> {
    let needle = normalize_header(wanted);
    let width = headers.first().map_or(0, Vec::len);
    (0..width).find(|index| {
        headers
            .iter()
            .any(|row| normalize_header(row.get(*index).map_or("", String::as_str)) == needle)
    })
}

/// Strict RFC-4180-style parser supporting escaped quotes and embedded newlines.
/// UTF-8 validation is performed before calling this function.
fn parse_csv_records(input: &str) -> Result<Vec<Vec<String>>, String> {
    if input.contains('\0') {
        return Err("CSV contains a NUL byte".into());
    }

    let bytes = input.as_bytes();
    let mut records: Vec<Vec<String>> = Vec::new();
    let mut record: Vec<String> = Vec::new();
    let mut field: Vec<u8> = Vec::new();
    let mut in_quotes = false;
    let mut quote_closed = false;
    let mut index = 0usize;

    while index < bytes.len() {
        let byte = bytes[index];

        if in_quotes {
            if byte == b'"' {
                if bytes.get(index + 1) == Some(&b'"') {
                    field.push(b'"');
                    index += 2;
                    continue;
                }
                in_quotes = false;
                quote_closed = true;
                index += 1;
                continue;
            }
            field.push(byte);
            index += 1;
            continue;
        }

        if quote_closed {
            match byte {
                b',' => {
                    record.push(
                        String::from_utf8(std::mem::take(&mut field)).map_err(|e| e.to_string())?,
                    );
                    quote_closed = false;
                    index += 1;
                }
                b'\n' => {
                    record.push(
                        String::from_utf8(std::mem::take(&mut field)).map_err(|e| e.to_string())?,
                    );
                    records.push(std::mem::take(&mut record));
                    quote_closed = false;
                    index += 1;
                }
                b'\r' => {
                    record.push(
                        String::from_utf8(std::mem::take(&mut field)).map_err(|e| e.to_string())?,
                    );
                    records.push(std::mem::take(&mut record));
                    quote_closed = false;
                    index += usize::from(bytes.get(index + 1) == Some(&b'\n')) + 1;
                }
                b' ' | b'\t' => index += 1,
                _ => {
                    return Err(format!(
                        "unexpected byte after closing quote at offset {index}"
                    ));
                }
            }
            continue;
        }

        match byte {
            b'"' if field.is_empty() => {
                in_quotes = true;
                index += 1;
            }
            b'"' => return Err(format!("quote inside unquoted field at offset {index}")),
            b',' => {
                record.push(
                    String::from_utf8(std::mem::take(&mut field)).map_err(|e| e.to_string())?,
                );
                index += 1;
            }
            b'\n' => {
                record.push(
                    String::from_utf8(std::mem::take(&mut field)).map_err(|e| e.to_string())?,
                );
                records.push(std::mem::take(&mut record));
                index += 1;
            }
            b'\r' => {
                record.push(
                    String::from_utf8(std::mem::take(&mut field)).map_err(|e| e.to_string())?,
                );
                records.push(std::mem::take(&mut record));
                index += usize::from(bytes.get(index + 1) == Some(&b'\n')) + 1;
            }
            _ => {
                field.push(byte);
                index += 1;
            }
        }
    }

    if in_quotes {
        return Err("unterminated quoted field".into());
    }
    if !field.is_empty() || !record.is_empty() || quote_closed {
        record.push(String::from_utf8(field).map_err(|e| e.to_string())?);
        records.push(record);
    }
    Ok(records)
}

fn parse_preview_size(value: &str) -> Option<u64> {
    let mut parts = value.split_whitespace();
    let amount = parts.next()?.parse::<f64>().ok()?;
    let unit = parts.next()?;
    let multiplier = match unit {
        "MB" => 1_000_000.0,
        "kB" => 1_000.0,
        _ => return None,
    };
    if !amount.is_finite() || amount < 0.0 {
        return None;
    }
    Some((amount * multiplier).round() as u64)
}

fn md5_with_available_tool(path: &Path) -> Option<String> {
    for program in ["md5sum", "md5", "openssl"] {
        let output = match program {
            "md5sum" => Command::new(program).arg(path).output(),
            "md5" => Command::new(program).arg("-q").arg(path).output(),
            _ => Command::new(program)
                .arg("dgst")
                .arg("-md5")
                .arg("-r")
                .arg(path)
                .output(),
        };
        let Ok(output) = output else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        for token in text.split_whitespace() {
            let candidate = token.trim_matches(|c: char| !c.is_ascii_hexdigit());
            if candidate.len() == 32 && candidate.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Some(candidate.to_ascii_lowercase());
            }
        }
    }
    None
}

fn sha256_file(path: &Path) -> Result<(u64, String), String> {
    let mut file =
        File::open(path).map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("cannot hash {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        total = total.saturating_add(read as u64);
    }
    Ok((total, format!("{:x}", hasher.finalize())))
}

fn audit_archive(path: &Path, manifest: &Value) -> Result<ArchiveAudit, String> {
    let (size_bytes, sha256) = sha256_file(path)?;
    let mut file = File::open(path)
        .map_err(|error| format!("cannot open archive {}: {error}", path.display()))?;
    let mut signature = [0u8; 4];
    file.read_exact(&mut signature)
        .map_err(|error| format!("cannot read ZIP signature: {error}"))?;
    let zip_signature_valid = signature == [0x50, 0x4b, 0x03, 0x04]
        || signature == [0x50, 0x4b, 0x05, 0x06]
        || signature == [0x50, 0x4b, 0x07, 0x08];
    let expected_md5 = manifest
        .pointer("/public_record_metadata/archive/source_published_md5")
        .and_then(Value::as_str)
        .map(str::to_ascii_lowercase);
    let observed_md5 = md5_with_available_tool(path);
    let publisher_md5_matches = expected_md5
        .as_ref()
        .zip(observed_md5.as_ref())
        .map(|(expected, observed)| expected == observed);

    Ok(ArchiveAudit {
        path: path.display().to_string(),
        size_bytes,
        sha256,
        publisher_md5: expected_md5,
        observed_md5: observed_md5.clone(),
        publisher_md5_matches,
        md5_tool_available: observed_md5.is_some(),
        zip_signature_valid,
    })
}

fn inspect_csv(path: &str, role: &str, bytes: &[u8]) -> Result<CsvAudit, String> {
    let text = std::str::from_utf8(bytes).map_err(|error| format!("invalid UTF-8: {error}"))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let records = parse_csv_records(text)?;
    if records.len() <= HEADER_ROWS {
        return Err(format!(
            "expected three header rows and data rows; found {} records",
            records.len()
        ));
    }

    let expected_columns = match role {
        "scenario_csv" => SCENARIO_COLUMNS,
        "baseline_csv" => BASELINE_COLUMNS,
        _ => return Err(format!("unsupported CSV role: {role}")),
    };

    let headers = &records[..HEADER_ROWS];
    for (index, row) in headers.iter().enumerate() {
        if row.len() != expected_columns {
            return Err(format!(
                "header row {} has {} columns; expected {}",
                index + 1,
                row.len(),
                expected_columns
            ));
        }
    }

    let anomaly_label_column = if role == "scenario_csv" {
        let index = header_index(headers, "Anomaly State")
            .ok_or_else(|| "scenario CSV has no Anomaly State header".to_string())?;
        if index != 2 {
            return Err(format!(
                "Anomaly State is at column {}; expected third column",
                index + 1
            ));
        }
        Some(index)
    } else {
        None
    };

    let time_abs_column = header_index(headers, "Time_abs")
        .ok_or_else(|| "Time_abs column was not found in the three header rows".to_string())?;
    if time_abs_column != 0 {
        return Err(format!(
            "Time_abs is at column {}; expected first column",
            time_abs_column + 1
        ));
    }
    let time_rel_column = header_index(headers, "Time_rel");
    if time_rel_column != Some(1) {
        return Err(format!(
            "Time_rel is at {:?}; expected second column",
            time_rel_column.map(|index| index + 1)
        ));
    }
    let mut data_rows = 0u64;
    let mut anomaly_zero_rows = 0u64;
    let mut anomaly_one_rows = 0u64;
    let mut anomaly_invalid_rows = 0u64;
    let mut anomaly_onset_transitions = 0u64;
    let mut previous_label: Option<u8> = None;
    let mut time_abs_nonempty_rows = 0u64;
    let mut time_abs_empty_rows = 0u64;
    let mut time_abs_duplicate_rows = 0u64;
    let mut seen_time_abs = HashSet::new();
    let mut first_time_abs = None;
    let mut last_time_abs = None;
    let mut time_rel_numeric_rows = 0u64;
    let mut time_rel_non_numeric_rows = 0u64;
    let mut time_rel_backsteps = 0u64;
    let mut time_rel_duplicates = 0u64;
    let mut previous_time: Option<f64> = None;
    let mut first_time_rel = None;
    let mut last_time_rel = None;
    let mut missing_counts = vec![0u64; expected_columns];

    for (record_index, row) in records.iter().enumerate().skip(HEADER_ROWS) {
        if row.len() != expected_columns {
            return Err(format!(
                "data record {} has {} columns; expected {}",
                record_index + 1,
                row.len(),
                expected_columns
            ));
        }
        data_rows += 1;

        for (column, value) in row.iter().enumerate() {
            if is_missing_value(value) {
                missing_counts[column] += 1;
            }
        }

        let absolute_time = row[time_abs_column].trim();
        if is_missing_value(absolute_time) {
            time_abs_empty_rows += 1;
        } else {
            time_abs_nonempty_rows += 1;
            if first_time_abs.is_none() {
                first_time_abs = Some(absolute_time.to_string());
            }
            last_time_abs = Some(absolute_time.to_string());
            if !seen_time_abs.insert(absolute_time.to_string()) {
                time_abs_duplicate_rows += 1;
            }
        }

        if let Some(label_column) = anomaly_label_column {
            let label = row[label_column].trim();
            let parsed = match label {
                "0" | "0.0" => Some(0),
                "1" | "1.0" => Some(1),
                _ => None,
            };
            match parsed {
                Some(0) => anomaly_zero_rows += 1,
                Some(1) => anomaly_one_rows += 1,
                Some(_) | None => anomaly_invalid_rows += 1,
            }
            if matches!(parsed, Some(0) | Some(1)) {
                let current = parsed.expect("matched binary anomaly label");
                if previous_label == Some(0) && current == 1 {
                    anomaly_onset_transitions += 1;
                }
                previous_label = Some(current);
            }
        }

        if let Some(time_column) = time_rel_column {
            let raw = row[time_column].trim();
            if first_time_rel.is_none() {
                first_time_rel = Some(raw.to_string());
            }
            last_time_rel = Some(raw.to_string());
            match raw.parse::<f64>() {
                Ok(value) if value.is_finite() => {
                    time_rel_numeric_rows += 1;
                    if let Some(previous) = previous_time {
                        if value < previous {
                            time_rel_backsteps += 1;
                        } else if value == previous {
                            time_rel_duplicates += 1;
                        }
                    }
                    previous_time = Some(value);
                }
                _ => time_rel_non_numeric_rows += 1,
            }
        }
    }

    if data_rows == 0 {
        return Err("CSV has no data records".into());
    }

    let header_names = headers[0].clone();
    let shorthand_symbols = headers[1].clone();
    let units = headers[2].clone();
    let missing_columns = missing_counts
        .iter()
        .enumerate()
        .filter(|(_, count)| **count > 0)
        .map(|(index, count)| MissingColumn {
            index,
            name: header_names.get(index).cloned().unwrap_or_default(),
            unit: units.get(index).cloned().unwrap_or_default(),
            missing_cells: *count,
        })
        .collect::<Vec<_>>();

    let anomaly_only_expected = path.to_ascii_lowercase().contains("injector_nozzle");
    let anomaly_only_observed = anomaly_label_column
        .map(|_| anomaly_zero_rows == 0 && anomaly_one_rows > 0 && anomaly_invalid_rows == 0);
    let anomaly_onset_evaluable = anomaly_label_column
        .map(|_| anomaly_zero_rows > 0 && anomaly_one_rows > 0 && anomaly_invalid_rows == 0);

    let missing_header_names = header_names
        .iter()
        .enumerate()
        .filter_map(|(index, value)| value.trim().is_empty().then_some(index))
        .collect();
    let missing_shorthand_symbols = shorthand_symbols
        .iter()
        .enumerate()
        .filter_map(|(index, value)| value.trim().is_empty().then_some(index))
        .collect();
    let missing_units = units
        .iter()
        .enumerate()
        .filter_map(|(index, value)| value.trim().is_empty().then_some(index))
        .collect();

    if anomaly_invalid_rows > 0 {
        return Err(format!("{anomaly_invalid_rows} non-binary anomaly labels"));
    }
    if time_rel_non_numeric_rows > 0 {
        return Err(format!(
            "{time_rel_non_numeric_rows} non-numeric Time_rel values"
        ));
    }

    Ok(CsvAudit {
        header_rows: HEADER_ROWS,
        column_count: expected_columns,
        data_rows,
        header_names,
        shorthand_symbols,
        units,
        anomaly_label_column,
        time_abs_column,
        time_abs_nonempty_rows,
        time_abs_empty_rows,
        time_abs_duplicate_rows,
        first_time_abs,
        last_time_abs,
        anomaly_zero_rows,
        anomaly_one_rows,
        anomaly_invalid_rows,
        anomaly_onset_transitions,
        anomaly_only_expected,
        anomaly_only_observed,
        anomaly_onset_evaluable,
        time_rel_column,
        time_rel_numeric_rows,
        time_rel_non_numeric_rows,
        time_rel_backsteps,
        time_rel_duplicates,
        first_time_rel,
        last_time_rel,
        missing_header_names,
        missing_shorthand_symbols,
        missing_units,
        missing_columns,
    })
}

fn inventory_from_manifest(manifest: &Value) -> Result<Vec<InventoryEntry>, String> {
    let values = manifest
        .pointer("/public_record_metadata/archive/file_inventory")
        .and_then(Value::as_array)
        .ok_or_else(|| "manifest has no archive.file_inventory array".to_string())?;
    let entries = values
        .iter()
        .cloned()
        .map(|value| serde_json::from_value(value).map_err(|error| error.to_string()))
        .collect::<Result<Vec<InventoryEntry>, String>>()?;

    let mut seen = HashSet::new();
    let mut counts = [0usize; 4];
    for entry in &entries {
        safe_relative_path(&entry.path)?;
        if !seen.insert(entry.path.as_str()) {
            return Err(format!("duplicate inventory path: {}", entry.path));
        }
        if parse_preview_size(&entry.reported_size).is_none() {
            return Err(format!("invalid preview size for {}", entry.path));
        }
        let slot = match entry.role.as_str() {
            "scenario_csv" => 0,
            "baseline_csv" => 1,
            "metadata" => 2,
            "license" => 3,
            other => return Err(format!("unsupported inventory role: {other}")),
        };
        counts[slot] += 1;
    }
    if entries.len() != 22 || counts != [15, 1, 5, 1] {
        return Err(format!(
            "unexpected v1 inventory: {} entries, role counts {:?}; expected 22 and [15, 1, 5, 1]",
            entries.len(),
            counts
        ));
    }
    Ok(entries)
}

fn archive_integrity_verified(archive: Option<&ArchiveAudit>) -> bool {
    archive.is_some_and(|item| item.zip_signature_valid && item.publisher_md5_matches == Some(true))
}

fn build_split_template(entries: &[InventoryEntry]) -> SplitTemplate {
    let groups = entries
        .iter()
        .filter(|entry| entry.role == "scenario_csv" || entry.role == "baseline_csv")
        .map(|entry| {
            let is_baseline = entry.role == "baseline_csv";
            SplitGroupTemplate {
                group_id: entry.path.clone(),
                source_file: entry.path.clone(),
                group_type: if is_baseline {
                    "baseline_reference_recording".into()
                } else {
                    "original_recording_file".into()
                },
                assignment: is_baseline.then(|| "reference_only".into()),
                assignment_state: if is_baseline {
                    "preassigned_reference_only".into()
                } else {
                    "unassigned_requires_review".into()
                },
                note: if is_baseline {
                    "Keep the single reference recording outside supervised train/validation/test partitions; use only for explicitly declared baseline false-alarm analysis and report its single-campaign limitation.".into()
                } else {
                    "Keep the complete physical fault recording together. Assign only after dataset-index, intervention, load and label audit; never split adjacent rows/windows across partitions.".into()
                },
            }
        })
        .collect();

    SplitTemplate {
        schema_version: "0.1.0".into(),
        assignment_policy: "explicit_group_assignment_after_payload_audit".into(),
        group_key: "original_recording_file_and_physical_fault_run".into(),
        random_window_split_allowed: false,
        groups,
    }
}

fn audit_dataset(cli: &Cli) -> Result<DatasetAuditReport, String> {
    let manifest_bytes = read_file_bounded(&cli.manifest_path)?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("invalid dataset manifest JSON: {error}"))?;
    let inventory = inventory_from_manifest(&manifest)?;

    let archive = match &cli.archive_path {
        Some(path) => Some(audit_archive(path, &manifest)?),
        None => None,
    };

    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut file_audits = Vec::with_capacity(inventory.len());
    let mut files_found = 0usize;
    let mut scenario_csvs_audited = 0usize;
    let mut baseline_csvs_audited = 0usize;
    let mut total_data_rows = 0u64;

    for entry in &inventory {
        let mut file_errors = Vec::new();
        let resolved = match resolve_inventory_path(&cli.dataset_root, &entry.path) {
            Ok(path) => Some(path),
            Err(error) => {
                file_errors.push(error.clone());
                errors.push(error);
                None
            }
        };
        let exists = resolved.is_some();

        let mut size_bytes = None;
        let mut sha256 = None;
        let mut utf8_valid = false;
        let mut metadata_csv_rows = None;
        let mut csv = None;
        let mut preview_size_delta_bytes = None;

        if let Some(path) = resolved {
            files_found += 1;
            match fs::metadata(&path) {
                Ok(metadata) => {
                    size_bytes = Some(metadata.len());
                    if let Some(expected) = parse_preview_size(&entry.reported_size) {
                        preview_size_delta_bytes = Some(metadata.len() as i64 - expected as i64);
                    }
                }
                Err(error) => file_errors.push(format!("cannot stat {}: {error}", path.display())),
            }

            if size_bytes.is_none() {
                file_errors.push("could not determine file size; refusing unbounded read".into());
            } else if size_bytes.is_some_and(|size| size > MAX_FILE_BYTES) {
                file_errors.push(format!(
                    "file exceeds safety limit of {MAX_FILE_BYTES} bytes"
                ));
            } else {
                match read_file_bounded(&path) {
                    Ok(bytes) => {
                        sha256 = Some(sha256_hex(&bytes));
                        utf8_valid = std::str::from_utf8(&bytes).is_ok();

                        if entry.role == "scenario_csv" || entry.role == "baseline_csv" {
                            match inspect_csv(&entry.path, &entry.role, &bytes) {
                                Ok(audit) => {
                                    total_data_rows =
                                        total_data_rows.saturating_add(audit.data_rows);
                                    if entry.role == "scenario_csv" {
                                        scenario_csvs_audited += 1;
                                    } else {
                                        baseline_csvs_audited += 1;
                                    }
                                    if audit.time_rel_backsteps > 0 {
                                        file_errors.push(format!(
                                            "Time_rel has {} backsteps",
                                            audit.time_rel_backsteps
                                        ));
                                    }
                                    if audit.time_abs_duplicate_rows > 0 {
                                        warnings.push(format!(
                                            "{}: Time_abs contains {} duplicate timestamps",
                                            entry.path, audit.time_abs_duplicate_rows
                                        ));
                                    }
                                    if audit.time_rel_duplicates > 0 {
                                        warnings.push(format!(
                                            "{}: Time_rel contains {} duplicate timestamps",
                                            entry.path, audit.time_rel_duplicates
                                        ));
                                    }
                                    if !audit.missing_header_names.is_empty()
                                        || !audit.missing_shorthand_symbols.is_empty()
                                        || !audit.missing_units.is_empty()
                                    {
                                        warnings.push(format!(
                                            "{}: incomplete header metadata (names {}, symbols {}, units {})",
                                            entry.path,
                                            audit.missing_header_names.len(),
                                            audit.missing_shorthand_symbols.len(),
                                            audit.missing_units.len()
                                        ));
                                    }
                                    if audit.time_abs_empty_rows > 0 {
                                        file_errors.push(format!(
                                            "Time_abs has {} empty rows",
                                            audit.time_abs_empty_rows
                                        ));
                                    }
                                    if audit.anomaly_only_expected
                                        && audit.anomaly_only_observed != Some(true)
                                    {
                                        file_errors.push("the anomaly-only nozzle recording contains pre-anomaly or invalid labels".into());
                                    }
                                    if !audit.anomaly_only_expected
                                        && audit.anomaly_onset_evaluable != Some(true)
                                    {
                                        file_errors.push("recording expected to contain both pre-anomaly and anomaly labels does not support onset evaluation".into());
                                    }
                                    csv = Some(audit);
                                }
                                Err(error) => file_errors.push(error),
                            }
                        } else if entry.path.to_ascii_lowercase().ends_with(".csv") {
                            match std::str::from_utf8(&bytes)
                                .map_err(|error| error.to_string())
                                .and_then(parse_csv_records)
                            {
                                Ok(records) => metadata_csv_rows = Some(records.len()),
                                Err(error) => {
                                    file_errors.push(format!("metadata CSV parse failed: {error}"))
                                }
                            }
                        } else if !utf8_valid {
                            file_errors.push("metadata text is not valid UTF-8".into());
                        }
                    }
                    Err(error) => file_errors.push(error),
                }
            }
        }

        for error in &file_errors {
            errors.push(format!("{}: {error}", entry.path));
        }

        file_audits.push(FileAudit {
            path: entry.path.clone(),
            role: entry.role.clone(),
            exists,
            size_bytes,
            preview_reported_size: entry.reported_size.clone(),
            preview_size_delta_bytes,
            sha256,
            utf8_valid,
            metadata_csv_rows,
            csv,
            errors: file_errors,
        });
    }

    if files_found != inventory.len() {
        warnings.push(format!(
            "{} of {} inventory files were found",
            files_found,
            inventory.len()
        ));
    }
    if scenario_csvs_audited != 15 {
        warnings.push(format!(
            "audited {} of 15 expected scenario CSVs",
            scenario_csvs_audited
        ));
    }
    if baseline_csvs_audited != 1 {
        warnings.push(format!(
            "audited {} of 1 expected baseline CSV",
            baseline_csvs_audited
        ));
    }
    if archive
        .as_ref()
        .is_some_and(|item| !item.zip_signature_valid)
    {
        errors.push(
            "archive does not have a recognized ZIP local/end-of-central-directory signature"
                .into(),
        );
    }
    if archive
        .as_ref()
        .is_some_and(|item| item.publisher_md5_matches == Some(false))
    {
        errors.push("archive MD5 does not match the publisher-published value".into());
    }
    if archive
        .as_ref()
        .is_some_and(|item| item.publisher_md5_matches.is_none())
    {
        warnings.push("publisher MD5 was not independently verified; ensure an md5sum, md5 or openssl executable is available".into());
    }
    if archive.is_none() {
        warnings.push(
            "archive bytes were not supplied; only extracted member files can be audited".into(),
        );
    } else {
        warnings.push("the auditor does not prove that the extracted tree was produced from the supplied ZIP; retain the extraction command, tool version and extraction log separately".into());
    }

    let all_expected_csvs = scenario_csvs_audited == 15 && baseline_csvs_audited == 1;
    let archive_integrity_verified = archive_integrity_verified(archive.as_ref());
    let audit_state = if errors.is_empty()
        && files_found == inventory.len()
        && all_expected_csvs
        && archive_integrity_verified
    {
        "content_audited_pending_independent_review"
    } else {
        "incomplete_or_invalid"
    };

    Ok(DatasetAuditReport {
        report_version: "0.1.0".into(),
        dataset_doi: manifest
            .pointer("/dataset/doi")
            .and_then(Value::as_str)
            .map(str::to_string),
        dataset_version: manifest
            .pointer("/dataset/version")
            .and_then(Value::as_str)
            .map(str::to_string),
        audit_state: audit_state.into(),
        claim_ceiling: manifest
            .get("claim_ceiling")
            .and_then(Value::as_str)
            .map(str::to_string),
        archive,
        files_expected: inventory.len(),
        files_found,
        scenario_csvs_audited,
        baseline_csvs_audited,
        total_data_rows,
        file_audits,
        split_template: build_split_template(&inventory),
        errors,
        warnings,
    })
}

fn main() -> ExitCode {
    let cli = match parse_args() {
        Ok(cli) => cli,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };

    let report = match audit_dataset(&cli) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };

    let json = match serde_json::to_string_pretty(&report) {
        Ok(json) => json,
        Err(error) => {
            eprintln!("cannot serialize audit report: {error}");
            return ExitCode::from(2);
        }
    };

    let write_result = match &cli.output_path {
        Some(path) => fs::write(path, format!("{json}\n"))
            .map_err(|error| format!("cannot write {}: {error}", path.display())),
        None => {
            println!("{json}");
            Ok(())
        }
    };
    if let Err(error) = write_result {
        eprintln!("{error}");
        return ExitCode::from(2);
    }

    // An incomplete payload audit is a distinct non-zero result, never a pass.
    if report.audit_state != "content_audited_pending_independent_review"
        || !report.errors.is_empty()
    {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_commas_and_escaped_quotes() {
        let parsed =
            parse_csv_records("name,description\nengine,\"large, marine \"\"diesel\"\"\"\n")
                .unwrap();
        assert_eq!(parsed[1][0], "engine");
        assert_eq!(parsed[1][1], "large, marine \"diesel\"");
    }

    #[test]
    fn parses_embedded_newlines_and_crlf() {
        let parsed = parse_csv_records("a,b\r\n1,\"line one\nline two\"\r\n").unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[1][1], "line one\nline two");
    }

    #[test]
    fn rejects_unterminated_quoted_field() {
        assert!(parse_csv_records("a,b\n1,\"unfinished").is_err());
    }

    #[test]
    fn rejects_quote_inside_unquoted_field() {
        assert!(parse_csv_records("a,b\n1,not\"quoted").is_err());
    }

    #[test]
    fn parses_trailing_empty_cells() {
        let parsed = parse_csv_records("a,b,c\n1,2,\n").unwrap();
        assert_eq!(parsed[1][0], "1");
        assert_eq!(parsed[1][1], "2");
        assert!(parsed[1][2].is_empty());
    }

    #[test]
    fn identifies_header_across_three_rows() {
        let headers = vec![
            vec![
                "Absolute time".into(),
                "Relative time".into(),
                "Anomaly State".into(),
            ],
            vec!["Time_abs".into(), "Time_rel".into(), "Anomaly_State".into()],
            vec!["s".into(), "s".into(), "1".into()],
        ];
        assert_eq!(header_index(&headers, "Anomaly State"), Some(2));
        assert_eq!(header_index(&headers, "Time_rel"), Some(1));
    }

    #[test]
    fn rejects_path_traversal() {
        assert!(safe_relative_path("../outside.csv").is_err());
        assert!(safe_relative_path("/absolute/file.csv").is_err());
        assert!(safe_relative_path("safe/file.csv").is_ok());
    }

    #[test]
    fn recognizes_common_missing_value_tokens() {
        for token in ["", "  ", "NA", "n/a", "NaN", "null", "None"] {
            assert!(is_missing_value(token), "token: {token}");
        }
        for token in ["0", "0.0", "-1", "not available?"] {
            assert!(!is_missing_value(token), "token: {token}");
        }
    }

    #[test]
    fn parses_decimal_preview_sizes() {
        assert_eq!(parse_preview_size("3.4 MB"), Some(3_400_000));
        assert_eq!(parse_preview_size("7.7 kB"), Some(7_700));
        assert_eq!(parse_preview_size("4 GB"), None);
    }

    #[test]
    fn split_template_never_assigns_a_partition() {
        let entries = vec![
            InventoryEntry {
                path: "a.csv".into(),
                reported_size: "1 MB".into(),
                role: "scenario_csv".into(),
            },
            InventoryEntry {
                path: "b.csv".into(),
                reported_size: "1 MB".into(),
                role: "baseline_csv".into(),
            },
        ];
        let template = build_split_template(&entries);
        assert_eq!(template.groups.len(), 2);
        assert_eq!(template.groups[0].assignment, None);
        assert_eq!(
            template.groups[1].assignment.as_deref(),
            Some("reference_only")
        );
        assert_eq!(
            template.groups[1].assignment_state,
            "preassigned_reference_only"
        );
        assert!(!template.random_window_split_allowed);
    }

    fn synthetic_scenario_csv(rows: &[(&str, &str, &str)]) -> String {
        let mut names = vec!["channel".to_string(); SCENARIO_COLUMNS];
        names[0] = "Time_abs".into();
        names[1] = "Time_rel".into();
        names[2] = "Anomaly State".into();
        let symbols = names
            .iter()
            .map(|name| format!("sym_{name}"))
            .collect::<Vec<_>>();
        let units = vec!["unit".to_string(); SCENARIO_COLUMNS];
        let mut output = format!(
            "{}\n{}\n{}\n",
            names.join(","),
            symbols.join(","),
            units.join(",")
        );
        for (absolute, relative, label) in rows {
            let mut cells = vec!["value".to_string(); SCENARIO_COLUMNS];
            cells[0] = (*absolute).into();
            cells[1] = (*relative).into();
            cells[2] = (*label).into();
            cells[4] = String::new();
            output.push_str(&cells.join(","));
            output.push('\n');
        }
        output
    }

    #[test]
    fn audits_scenario_label_transitions_missingness_and_time() {
        let data = synthetic_scenario_csv(&[("100", "0", "0"), ("101", "1", "1")]);
        let audit = inspect_csv("AC_Fouling_40_Load.csv", "scenario_csv", data.as_bytes()).unwrap();
        assert_eq!(audit.column_count, SCENARIO_COLUMNS);
        assert_eq!(audit.data_rows, 2);
        assert_eq!(audit.anomaly_zero_rows, 1);
        assert_eq!(audit.anomaly_one_rows, 1);
        assert_eq!(audit.anomaly_onset_transitions, 1);
        assert_eq!(audit.anomaly_onset_evaluable, Some(true));
        assert_eq!(audit.time_rel_backsteps, 0);
        assert_eq!(audit.missing_columns.len(), 1);
        assert_eq!(audit.missing_columns[0].index, 4);
        assert_eq!(audit.missing_columns[0].missing_cells, 2);
    }

    #[test]
    fn flags_non_monotonic_relative_time_without_reordering_samples() {
        let data = synthetic_scenario_csv(&[("100", "2", "0"), ("101", "1", "1")]);
        let audit = inspect_csv("AC_Fouling_40_Load.csv", "scenario_csv", data.as_bytes()).unwrap();
        assert_eq!(audit.time_rel_backsteps, 1);
        assert_eq!(audit.first_time_rel.as_deref(), Some("2"));
        assert_eq!(audit.last_time_rel.as_deref(), Some("1"));
    }

    #[test]
    fn both_injector_nozzle_files_are_anomaly_only_and_not_onset_evaluable() {
        let data = synthetic_scenario_csv(&[("100", "0", "1"), ("101", "1", "1")]);
        for path in [
            "Injector_Nozzle/Clogged_Injector_Nozzle1_40_60_85_Load.csv",
            "Injector_Nozzle/Clogged_Injector_Nozzle2_LoadProgram.csv",
        ] {
            let audit = inspect_csv(path, "scenario_csv", data.as_bytes()).unwrap();
            assert!(audit.anomaly_only_expected, "{path}");
            assert_eq!(audit.anomaly_only_observed, Some(true), "{path}");
            assert_eq!(audit.anomaly_onset_transitions, 0, "{path}");
            assert_eq!(audit.anomaly_onset_evaluable, Some(false), "{path}");
        }
    }

    #[test]
    fn tracks_duplicate_absolute_timestamps() {
        let data = synthetic_scenario_csv(&[("100", "0", "0"), ("100", "1", "1")]);
        let audit = inspect_csv("AC_Fouling_40_Load.csv", "scenario_csv", data.as_bytes()).unwrap();
        assert_eq!(audit.time_abs_nonempty_rows, 2);
        assert_eq!(audit.time_abs_empty_rows, 0);
        assert_eq!(audit.time_abs_duplicate_rows, 1);
        assert_eq!(audit.first_time_abs.as_deref(), Some("100"));
        assert_eq!(audit.last_time_abs.as_deref(), Some("100"));
    }

    #[test]
    fn counts_missing_absolute_timestamps_without_reordering() {
        let data = synthetic_scenario_csv(&[("", "0", "0"), ("101", "1", "1")]);
        let audit = inspect_csv("AC_Fouling_40_Load.csv", "scenario_csv", data.as_bytes()).unwrap();
        assert_eq!(audit.time_abs_empty_rows, 1);
        assert_eq!(audit.time_abs_nonempty_rows, 1);
    }

    #[test]
    fn refuses_to_treat_missing_or_unverified_archive_as_a_pass() {
        assert!(!archive_integrity_verified(None));
        let archive = ArchiveAudit {
            path: "dataset.zip".into(),
            size_bytes: 1,
            sha256: "0".repeat(64),
            publisher_md5: Some("0".repeat(32)),
            observed_md5: None,
            publisher_md5_matches: None,
            md5_tool_available: false,
            zip_signature_valid: true,
        };
        assert!(!archive_integrity_verified(Some(&archive)));
        let mut mismatch = archive.clone();
        mismatch.publisher_md5_matches = Some(false);
        assert!(!archive_integrity_verified(Some(&mismatch)));
        let mut valid = archive;
        valid.publisher_md5_matches = Some(true);
        assert!(archive_integrity_verified(Some(&valid)));
    }

    #[test]
    fn rejects_duplicate_paths_in_dataset_inventory() {
        let item = serde_json::json!({
            "path":"Marine_Engine_Fault_Data/Reference_Data.csv",
            "reported_size":"14.8 MB",
            "role":"baseline_csv"
        });
        let entries = vec![item; 22];
        let manifest = serde_json::json!({
            "public_record_metadata":{"archive":{"file_inventory":entries}}
        });
        assert!(inventory_from_manifest(&manifest).is_err());
    }

    #[test]
    fn sha256_is_stable_for_known_input() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
