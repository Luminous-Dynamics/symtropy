//! Receipt-bound import of caller-declared SU2 history fields.
//!
//! Header fields and units are caller-declared because SU2 history output depends on its solver
//! and configuration. Importing a history file is not convergence assessment or validation.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::external_solver::{
    ExternalSolverReceipt, read_external_solver_receipt, verify_external_solver_receipt,
};
use crate::integrity::sha256_hex;
use crate::shapebench::parse_csv_record;

pub const SU2_HISTORY_IMPORT_SCHEMA: &str = "su2-history-import/0.1.0";
pub const SU2_HISTORY_EVIDENCE_LEVEL: &str =
    "receipt_bound_history_import_only_not_solver_convergence_or_physical_validation";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Su2HistoryField {
    pub column: String,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Su2HistorySchema {
    pub history_relative_path: String,
    pub expected_header: Vec<String>,
    pub iteration_column: String,
    pub numeric_fields: Vec<Su2HistoryField>,
    pub minimum_rows: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Su2HistoryPoint {
    pub iteration: u64,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Su2HistoryReport {
    pub schema: String,
    pub run_id: String,
    pub solver_id: String,
    pub solver_version_claim: String,
    pub executable_sha256: String,
    pub runner_receipt_sha256: String,
    pub history_relative_path: String,
    pub history_byte_length: u64,
    pub history_sha256: String,
    pub iteration_column: String,
    pub expected_header: Vec<String>,
    pub numeric_fields: Vec<Su2HistoryField>,
    pub points: Vec<Su2HistoryPoint>,
    pub import_sha256: String,
    pub evidence_level: String,
}

impl Su2HistoryReport {
    pub fn to_csv(&self) -> String {
        let mut header = vec![
            "schema".to_owned(),
            "run_id".to_owned(),
            "solver_id".to_owned(),
            "solver_version_claim".to_owned(),
            "executable_sha256".to_owned(),
            "runner_receipt_sha256".to_owned(),
            "history_relative_path".to_owned(),
            "history_byte_length".to_owned(),
            "history_sha256".to_owned(),
            "iteration_column".to_owned(),
            "iteration".to_owned(),
        ];
        for field in &self.numeric_fields {
            header.push(format!("{} [{}]", field.column, field.unit));
        }
        header.push("import_sha256".to_owned());
        header.push("evidence_level".to_owned());

        let mut output = header
            .iter()
            .map(|value| csv_escape(value))
            .collect::<Vec<_>>()
            .join(",");
        output.push('\n');
        for point in &self.points {
            let mut fields = vec![
                csv_escape(&self.schema),
                csv_escape(&self.run_id),
                csv_escape(&self.solver_id),
                csv_escape(&self.solver_version_claim),
                self.executable_sha256.clone(),
                self.runner_receipt_sha256.clone(),
                csv_escape(&self.history_relative_path),
                self.history_byte_length.to_string(),
                self.history_sha256.clone(),
                csv_escape(&self.iteration_column),
                point.iteration.to_string(),
            ];
            fields.extend(point.values.iter().map(|value| format!("{value:.17}")));
            fields.push(self.import_sha256.clone());
            fields.push(csv_escape(&self.evidence_level));
            output.push_str(&fields.join(","));
            output.push('\n');
        }
        output
    }
}

fn validate_relative_path(value: &str) -> Result<PathBuf, String> {
    if value.trim().is_empty() {
        return Err("history relative path must not be empty".to_owned());
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("history path must contain only normal run-relative components".to_owned());
    }
    Ok(path.to_path_buf())
}

fn canonical_regular_file_under_root(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let path = validate_relative_path(relative)?;
    let candidate = root.join(path);
    let metadata = fs::symlink_metadata(&candidate)
        .map_err(|error| format!("cannot inspect SU2 history file {relative}: {error}"))?;
    if metadata.file_type().is_symlink() {
        return Err("SU2 history output must not be a symlink".to_owned());
    }
    let canonical = fs::canonicalize(&candidate)
        .map_err(|error| format!("cannot resolve SU2 history output {relative}: {error}"))?;
    if !canonical.starts_with(root) {
        return Err("SU2 history output resolves outside the run directory".to_owned());
    }
    if !fs::metadata(&canonical)
        .map_err(|error| format!("cannot inspect SU2 history output {relative}: {error}"))?
        .is_file()
    {
        return Err("SU2 history output must be a regular file".to_owned());
    }
    Ok(canonical)
}

fn validate_schema(schema: &Su2HistorySchema) -> Result<(), String> {
    validate_relative_path(&schema.history_relative_path)?;
    if schema.minimum_rows < 2 {
        return Err("minimum_rows must be at least 2".to_owned());
    }
    if schema.expected_header.is_empty() {
        return Err("expected_header must contain at least one column".to_owned());
    }
    let mut header_names = BTreeSet::new();
    for name in &schema.expected_header {
        if name.trim().is_empty() || !header_names.insert(name) {
            return Err("expected_header contains an empty or duplicate column name".to_owned());
        }
    }
    if !header_names.contains(&schema.iteration_column) {
        return Err("iteration_column is absent from expected_header".to_owned());
    }
    if schema.numeric_fields.is_empty() {
        return Err("at least one numeric history field is required".to_owned());
    }
    let mut numeric_names = BTreeSet::new();
    for field in &schema.numeric_fields {
        if field.column.trim().is_empty()
            || field.unit.trim().is_empty()
            || field.column == schema.iteration_column
            || !header_names.contains(&field.column)
            || !numeric_names.insert(&field.column)
        {
            return Err(format!(
                "numeric field {:?} has an empty unit/name, duplicates another field, is the iteration column, or is absent from the header",
                field.column
            ));
        }
    }
    Ok(())
}

fn append_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    append_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn canonical_import_bytes(report: &Su2HistoryReport) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        report.schema.as_str(),
        report.run_id.as_str(),
        report.solver_id.as_str(),
        report.solver_version_claim.as_str(),
        report.executable_sha256.as_str(),
        report.runner_receipt_sha256.as_str(),
        report.history_relative_path.as_str(),
        report.history_sha256.as_str(),
        report.iteration_column.as_str(),
        report.evidence_level.as_str(),
    ] {
        append_string(&mut bytes, value);
    }
    append_u64(&mut bytes, report.history_byte_length);
    append_u64(&mut bytes, report.expected_header.len() as u64);
    for value in &report.expected_header {
        append_string(&mut bytes, value);
    }
    append_u64(&mut bytes, report.numeric_fields.len() as u64);
    for field in &report.numeric_fields {
        append_string(&mut bytes, &field.column);
        append_string(&mut bytes, &field.unit);
    }
    append_u64(&mut bytes, report.points.len() as u64);
    for point in &report.points {
        append_u64(&mut bytes, point.iteration);
        append_u64(&mut bytes, point.values.len() as u64);
        for value in &point.values {
            bytes.extend_from_slice(&value.to_bits().to_le_bytes());
        }
    }
    bytes
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

/// Verify the process receipt and its output bytes before importing declared history fields.
/// This does not infer residual convergence, force stability, grid convergence, or accuracy.
pub fn import_su2_history(
    run_directory: &Path,
    schema: &Su2HistorySchema,
) -> Result<Su2HistoryReport, String> {
    validate_schema(schema)?;
    let receipt = read_external_solver_receipt(run_directory)?;
    import_su2_history_with_receipt(run_directory, schema, &receipt)
}

fn import_su2_history_with_receipt(
    run_directory: &Path,
    schema: &Su2HistorySchema,
    receipt: &ExternalSolverReceipt,
) -> Result<Su2HistoryReport, String> {
    validate_schema(schema)?;
    if receipt.solver_id != "SU2_CFD" {
        return Err(format!(
            "expected solver_id SU2_CFD, got {}",
            receipt.solver_id
        ));
    }
    if !receipt.is_process_success() {
        return Err(format!(
            "external solver process is not successful: {}",
            receipt.status
        ));
    }
    verify_external_solver_receipt(run_directory, receipt)?;

    let output = receipt
        .outputs
        .iter()
        .find(|output| output.relative_path == schema.history_relative_path)
        .ok_or_else(|| "history path is not declared in the runner receipt".to_owned())?;
    if !output.present {
        return Err("history output is missing from the runner receipt".to_owned());
    }
    let expected_length = output
        .byte_length
        .ok_or_else(|| "history output receipt has no byte length".to_owned())?;
    let expected_sha256 = output
        .sha256
        .as_deref()
        .ok_or_else(|| "history output receipt has no SHA-256".to_owned())?;

    let root = fs::canonicalize(run_directory)
        .map_err(|error| format!("cannot resolve external solver run directory: {error}"))?;
    let history_path = canonical_regular_file_under_root(&root, &schema.history_relative_path)?;
    let bytes = fs::read(&history_path)
        .map_err(|error| format!("cannot read SU2 history output: {error}"))?;
    if bytes.len() as u64 != expected_length {
        return Err("history output byte length differs from runner receipt".to_owned());
    }
    let history_sha256 = sha256_hex(&bytes)?;
    if history_sha256 != expected_sha256 {
        return Err("history output SHA-256 differs from runner receipt".to_owned());
    }
    let text = String::from_utf8(bytes)
        .map_err(|error| format!("SU2 history output is not valid UTF-8: {error}"))?;
    let mut lines = text.lines();
    let header_line = lines
        .next()
        .ok_or_else(|| "SU2 history output is empty".to_owned())?
        .trim_end_matches('\r');
    let header = parse_csv_record(header_line)?;
    if header != schema.expected_header {
        return Err(format!(
            "SU2 history header differs from caller-declared schema: expected {:?}, got {:?}",
            schema.expected_header, header
        ));
    }
    let iteration_index = header
        .iter()
        .position(|name| name == &schema.iteration_column)
        .ok_or_else(|| "iteration column is absent from history header".to_owned())?;
    let numeric_indices = schema
        .numeric_fields
        .iter()
        .map(|field| {
            header
                .iter()
                .position(|name| name == &field.column)
                .ok_or_else(|| {
                    format!(
                        "numeric field {} is absent from history header",
                        field.column
                    )
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut points = Vec::new();
    let mut previous_iteration = None;
    for (row_index, line) in lines.enumerate() {
        let line_number = row_index + 2;
        if line.trim().is_empty() {
            return Err(format!(
                "line {line_number}: blank history records are not permitted"
            ));
        }
        let fields = parse_csv_record(line.trim_end_matches('\r'))
            .map_err(|error| format!("line {line_number}: {error}"))?;
        if fields.len() != header.len() {
            return Err(format!(
                "line {line_number}: expected {} history fields, got {}",
                header.len(),
                fields.len()
            ));
        }
        let iteration = fields[iteration_index]
            .parse::<u64>()
            .map_err(|error| format!("line {line_number}: invalid iteration: {error}"))?;
        if previous_iteration.is_some_and(|previous| iteration <= previous) {
            return Err(format!(
                "line {line_number}: iteration values must be strictly increasing"
            ));
        }
        let values = numeric_indices
            .iter()
            .zip(&schema.numeric_fields)
            .map(|(index, field)| {
                let value = fields[*index].trim().parse::<f64>().map_err(|error| {
                    format!(
                        "line {line_number}: {} is not a numeric value: {error}",
                        field.column
                    )
                })?;
                if !value.is_finite() {
                    return Err(format!(
                        "line {line_number}: {} must be finite",
                        field.column
                    ));
                }
                Ok(value)
            })
            .collect::<Result<Vec<_>, String>>()?;
        points.push(Su2HistoryPoint { iteration, values });
        previous_iteration = Some(iteration);
    }
    if points.len() < schema.minimum_rows {
        return Err(format!(
            "history contains {} rows; minimum_rows is {}",
            points.len(),
            schema.minimum_rows
        ));
    }

    let mut report = Su2HistoryReport {
        schema: SU2_HISTORY_IMPORT_SCHEMA.to_owned(),
        run_id: receipt.run_id.clone(),
        solver_id: receipt.solver_id.clone(),
        solver_version_claim: receipt.solver_version_claim.clone(),
        executable_sha256: receipt.executable_sha256.clone(),
        runner_receipt_sha256: receipt.receipt_sha256.clone(),
        history_relative_path: schema.history_relative_path.clone(),
        history_byte_length: expected_length,
        history_sha256,
        iteration_column: schema.iteration_column.clone(),
        expected_header: schema.expected_header.clone(),
        numeric_fields: schema.numeric_fields.clone(),
        points,
        import_sha256: String::new(),
        evidence_level: SU2_HISTORY_EVIDENCE_LEVEL.to_owned(),
    };
    report.import_sha256 = sha256_hex(&canonical_import_bytes(&report))?;
    Ok(report)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::external_solver::{ExternalSolverRequest, execute_external_solver};
    use crate::integrity::sha256_file;
    use std::collections::BTreeMap;
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;

    fn temporary_root(label: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "su2-history-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create test root");
        path
    }

    fn run_fixture(root: &Path, solver_id: &str, history: &str) -> PathBuf {
        fs::create_dir_all(root).expect("create fixture root");
        let executable = root.join("fake-su2-cfd.sh");
        let shell_quoted_history = history.replace('\'', "'\\''");
        let script =
            format!("#!/bin/sh\nset -eu\nprintf '%s' '{shell_quoted_history}' > history.csv\n");
        fs::write(&executable, script).expect("write fake solver");
        let mut permissions = fs::metadata(&executable)
            .expect("fake solver metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).expect("make fake solver executable");
        let run_directory = root.join("run");
        let request = ExternalSolverRequest {
            run_id: "su2-history-test-run".to_owned(),
            solver_id: solver_id.to_owned(),
            solver_version_claim: "synthetic-ci-fixture-not-a-SU2-version".to_owned(),
            executable_path: executable.clone(),
            run_directory: run_directory.clone(),
            arguments: Vec::new(),
            environment: BTreeMap::new(),
            staged_inputs: Vec::new(),
            expected_outputs: vec!["history.csv".to_owned()],
            timeout: Duration::from_secs(2),
            max_log_bytes_per_stream: 16 * 1024,
        };
        let receipt = execute_external_solver(&request).expect("create fixture process receipt");
        assert!(receipt.is_process_success());
        assert_eq!(
            receipt.executable_sha256,
            sha256_file(&executable).expect("hash fake solver")
        );
        run_directory
    }

    fn schema() -> Su2HistorySchema {
        Su2HistorySchema {
            history_relative_path: "history.csv".to_owned(),
            expected_header: vec![
                "Iteration".to_owned(),
                "CL".to_owned(),
                "CD".to_owned(),
                "RMS_RES".to_owned(),
            ],
            iteration_column: "Iteration".to_owned(),
            numeric_fields: vec![
                Su2HistoryField {
                    column: "CL".to_owned(),
                    unit: "dimensionless".to_owned(),
                },
                Su2HistoryField {
                    column: "CD".to_owned(),
                    unit: "dimensionless".to_owned(),
                },
            ],
            minimum_rows: 2,
        }
    }

    #[test]
    fn imports_only_receipt_bound_declared_finite_history_fields() {
        let root = temporary_root("positive");
        let run = run_fixture(
            &root,
            "SU2_CFD",
            "\"Iteration\",\"CL\",\"CD\",\"RMS_RES\"\n0,0.40,0.020,1e-2\n1,0.42,0.018,5e-3\n2,0.43,0.017,1e-4\n",
        );
        let report = import_su2_history(&run, &schema()).expect("valid receipt-bound history");
        assert_eq!(report.points.len(), 3);
        assert_eq!(report.points[2].iteration, 2);
        assert_eq!(report.points[2].values, vec![0.43, 0.017]);
        assert_eq!(
            report.history_sha256,
            sha256_file(&run.join("history.csv")).expect("history digest")
        );
        assert_eq!(report.evidence_level, SU2_HISTORY_EVIDENCE_LEVEL);
        assert_eq!(report.to_csv().lines().count(), 4);
        let repeat = import_su2_history(&run, &schema()).expect("deterministic repeat");
        assert_eq!(report.import_sha256, repeat.import_sha256);
        fs::remove_dir_all(root).expect("remove positive fixture");
    }

    #[test]
    fn rejects_wrong_solver_duplicate_iterations_and_nonfinite_values() {
        let root = temporary_root("negative");
        let wrong_solver = run_fixture(
            &root.join("wrong-solver"),
            "NOT_SU2",
            "\"Iteration\",\"CL\",\"CD\",\"RMS_RES\"\n0,0.4,0.02,1e-2\n1,0.42,0.018,1e-3\n",
        );
        assert!(
            import_su2_history(&wrong_solver, &schema())
                .expect_err("wrong solver id rejected")
                .contains("expected solver_id SU2_CFD")
        );

        let duplicate = run_fixture(
            &root.join("duplicate"),
            "SU2_CFD",
            "\"Iteration\",\"CL\",\"CD\",\"RMS_RES\"\n0,0.4,0.02,1e-2\n0,0.42,0.018,1e-3\n",
        );
        assert!(
            import_su2_history(&duplicate, &schema())
                .expect_err("duplicate iteration rejected")
                .contains("strictly increasing")
        );

        let nonfinite = run_fixture(
            &root.join("nonfinite"),
            "SU2_CFD",
            "\"Iteration\",\"CL\",\"CD\",\"RMS_RES\"\n0,0.4,0.02,1e-2\n1,NaN,0.018,1e-3\n",
        );
        assert!(
            import_su2_history(&nonfinite, &schema())
                .expect_err("NaN rejected")
                .contains("must be finite")
        );
        fs::remove_dir_all(root).expect("remove negative fixtures");
    }

    #[test]
    fn rejects_mutated_history_bytes_after_receipt_was_written() {
        let root = temporary_root("tamper");
        let run = run_fixture(
            &root,
            "SU2_CFD",
            "\"Iteration\",\"CL\",\"CD\",\"RMS_RES\"\n0,0.4,0.02,1e-2\n1,0.42,0.018,1e-3\n",
        );
        fs::write(
            run.join("history.csv"),
            "\"Iteration\",\"CL\",\"CD\",\"RMS_RES\"\n0,0.4,0.02,1e-2\n1,0.99,0.018,1e-3\n",
        )
        .expect("tamper with history output");
        let error = import_su2_history(&run, &schema()).expect_err("mutated output rejected");
        assert!(error.contains("one or more expected solver outputs"));
        fs::remove_dir_all(root).expect("remove tamper fixture");
    }
}
