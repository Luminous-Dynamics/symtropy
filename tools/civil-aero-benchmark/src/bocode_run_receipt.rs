//! Strict Rust import of normalized BOCoDe per-trial evidence rows.
//!
//! The external Python runner is untrusted input. A row is an index to immutable raw
//! artifacts and receipt bytes; it does not independently prove objective correctness,
//! solver quality, or physical validation.

use std::collections::BTreeSet;

use crate::shapebench::parse_csv_record;

pub const BOCODE_RUN_ROW_SCHEMA: &str = "bocode-single-trial-receipt/0.1.0";
pub const BOCODE_RUN_ROW_HEADER: &str = "schema,attempt_id,run_id,request_sha256,state,reason,source_commit,package_version,task_id,metadata_sha256,algorithm,seed,n_init_requested,iterations_requested,objective_evaluation_count,initial_design_count,feasible_evaluation_count,infeasible_evaluation_count,feasible_evaluation_fraction,process_exit_code,wall_seconds,cpu_user_seconds,cpu_system_seconds,peak_memory_state,gpu_state,energy_state,stdout_sha256,stderr_sha256,npz_sha256,summary_sha256,environment_freeze_sha256,receipt_sha256";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoCoDeTrialState {
    Succeeded,
    Failed,
    InvalidTrace,
    WrapperError,
}

impl BoCoDeTrialState {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "invalid_trace" => Ok(Self::InvalidTrace),
            "wrapper_error" => Ok(Self::WrapperError),
            _ => Err(format!("unknown BOCoDe trial state {value}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::InvalidTrace => "invalid_trace",
            Self::WrapperError => "wrapper_error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoCoDeResourceState {
    Unknown,
    Measured,
}

impl BoCoDeResourceState {
    fn parse(value: &str, field: &str) -> Result<Self, String> {
        match value {
            "unknown" => Ok(Self::Unknown),
            "measured" => Ok(Self::Measured),
            _ => Err(format!("{field} must be unknown or measured")),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoCoDeRunLedgerRow {
    pub attempt_id: String,
    pub run_id: String,
    pub request_sha256: String,
    pub state: BoCoDeTrialState,
    pub reason: String,
    pub source_commit: String,
    pub package_version: String,
    pub task_id: String,
    pub metadata_sha256: String,
    pub algorithm: String,
    pub seed: u64,
    pub n_init_requested: u64,
    pub iterations_requested: u64,
    pub objective_evaluation_count: Option<u64>,
    pub initial_design_count: Option<u64>,
    pub feasible_evaluation_count: Option<u64>,
    pub infeasible_evaluation_count: Option<u64>,
    pub feasible_evaluation_fraction: Option<f64>,
    pub process_exit_code: Option<i32>,
    pub wall_seconds: Option<f64>,
    pub cpu_user_seconds: Option<f64>,
    pub cpu_system_seconds: Option<f64>,
    pub peak_memory_state: BoCoDeResourceState,
    pub gpu_state: BoCoDeResourceState,
    pub energy_state: BoCoDeResourceState,
    pub stdout_sha256: Option<String>,
    pub stderr_sha256: Option<String>,
    pub npz_sha256: Option<String>,
    pub summary_sha256: Option<String>,
    pub environment_freeze_sha256: Option<String>,
    pub receipt_sha256: String,
}

fn required<'a>(fields: &'a [String], index: usize, field: &str) -> Result<&'a str, String> {
    fields
        .get(index)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{field} must not be empty"))
}

fn parse_u64(value: &str, field: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|error| format!("{field} must be a non-negative integer: {error}"))
}

fn parse_optional_u64(value: &str, field: &str) -> Result<Option<u64>, String> {
    if value.is_empty() {
        Ok(None)
    } else {
        parse_u64(value, field).map(Some)
    }
}

fn parse_optional_i32(value: &str, field: &str) -> Result<Option<i32>, String> {
    if value.is_empty() {
        Ok(None)
    } else {
        value
            .parse::<i32>()
            .map(Some)
            .map_err(|error| format!("{field} must be an integer: {error}"))
    }
}

fn parse_optional_f64(value: &str, field: &str) -> Result<Option<f64>, String> {
    if value.is_empty() {
        return Ok(None);
    }
    let parsed = value
        .parse::<f64>()
        .map_err(|error| format!("{field} must be a decimal number: {error}"))?;
    if !parsed.is_finite() || parsed < 0.0 {
        return Err(format!("{field} must be finite and non-negative"));
    }
    Ok(Some(parsed))
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_optional_sha(value: &str, field: &str) -> Result<Option<String>, String> {
    if value.is_empty() {
        return Ok(None);
    }
    if !valid_hex(value, 64) {
        return Err(format!(
            "{field} must be a 64-character hexadecimal SHA-256"
        ));
    }
    Ok(Some(value.to_ascii_lowercase()))
}

impl BoCoDeRunLedgerRow {
    fn parse(fields: &[String], line_number: usize) -> Result<Self, String> {
        if fields.len() != 32 {
            return Err(format!(
                "line {line_number}: expected 32 run ledger columns, got {}",
                fields.len()
            ));
        }
        if required(fields, 0, "schema")? != BOCODE_RUN_ROW_SCHEMA {
            return Err(format!("line {line_number}: unsupported run row schema"));
        }
        let row = Self {
            attempt_id: required(fields, 1, "attempt_id")?.to_owned(),
            run_id: required(fields, 2, "run_id")?.to_owned(),
            request_sha256: required(fields, 3, "request_sha256")?.to_ascii_lowercase(),
            state: BoCoDeTrialState::parse(required(fields, 4, "state")?)?,
            reason: required(fields, 5, "reason")?.to_owned(),
            source_commit: required(fields, 6, "source_commit")?.to_ascii_lowercase(),
            package_version: required(fields, 7, "package_version")?.to_owned(),
            task_id: required(fields, 8, "task_id")?.to_owned(),
            metadata_sha256: required(fields, 9, "metadata_sha256")?.to_ascii_lowercase(),
            algorithm: required(fields, 10, "algorithm")?.to_owned(),
            seed: parse_u64(required(fields, 11, "seed")?, "seed")?,
            n_init_requested: parse_u64(
                required(fields, 12, "n_init_requested")?,
                "n_init_requested",
            )?,
            iterations_requested: parse_u64(
                required(fields, 13, "iterations_requested")?,
                "iterations_requested",
            )?,
            objective_evaluation_count: parse_optional_u64(
                &fields[14],
                "objective_evaluation_count",
            )?,
            initial_design_count: parse_optional_u64(&fields[15], "initial_design_count")?,
            feasible_evaluation_count: parse_optional_u64(
                &fields[16],
                "feasible_evaluation_count",
            )?,
            infeasible_evaluation_count: parse_optional_u64(
                &fields[17],
                "infeasible_evaluation_count",
            )?,
            feasible_evaluation_fraction: parse_optional_f64(
                &fields[18],
                "feasible_evaluation_fraction",
            )?,
            process_exit_code: parse_optional_i32(&fields[19], "process_exit_code")?,
            wall_seconds: parse_optional_f64(&fields[20], "wall_seconds")?,
            cpu_user_seconds: parse_optional_f64(&fields[21], "cpu_user_seconds")?,
            cpu_system_seconds: parse_optional_f64(&fields[22], "cpu_system_seconds")?,
            peak_memory_state: BoCoDeResourceState::parse(&fields[23], "peak_memory_state")?,
            gpu_state: BoCoDeResourceState::parse(&fields[24], "gpu_state")?,
            energy_state: BoCoDeResourceState::parse(&fields[25], "energy_state")?,
            stdout_sha256: parse_optional_sha(&fields[26], "stdout_sha256")?,
            stderr_sha256: parse_optional_sha(&fields[27], "stderr_sha256")?,
            npz_sha256: parse_optional_sha(&fields[28], "npz_sha256")?,
            summary_sha256: parse_optional_sha(&fields[29], "summary_sha256")?,
            environment_freeze_sha256: parse_optional_sha(
                &fields[30],
                "environment_freeze_sha256",
            )?,
            receipt_sha256: required(fields, 31, "receipt_sha256")?.to_ascii_lowercase(),
        };
        if !valid_hex(&row.source_commit, 40)
            || !valid_hex(&row.request_sha256, 64)
            || !valid_hex(&row.metadata_sha256, 64)
            || !valid_hex(&row.receipt_sha256, 64)
        {
            return Err(format!(
                "line {line_number}: source commit or receipt identity digest is malformed"
            ));
        }
        if row.iterations_requested == 0 {
            return Err(format!(
                "line {line_number}: iterations_requested must be positive"
            ));
        }
        if let Some(fraction) = row.feasible_evaluation_fraction {
            if !(0.0..=1.0).contains(&fraction) {
                return Err(format!(
                    "line {line_number}: feasible_evaluation_fraction must be in [0,1]"
                ));
            }
        }
        match (
            row.feasible_evaluation_count,
            row.infeasible_evaluation_count,
            row.objective_evaluation_count,
        ) {
            (Some(feasible), Some(infeasible), Some(total))
                if feasible.checked_add(infeasible) != Some(total) =>
            {
                return Err(format!(
                    "line {line_number}: feasible + infeasible evaluation counts differ from total"
                ));
            }
            _ => {}
        }
        if let Some(count) = row.objective_evaluation_count {
            if row
                .initial_design_count
                .is_some_and(|initial| initial > count)
            {
                return Err(format!(
                    "line {line_number}: initial design count exceeds observed evaluations"
                ));
            }
        }
        match row.state {
            BoCoDeTrialState::Succeeded => {
                if row.process_exit_code != Some(0)
                    || row
                        .objective_evaluation_count
                        .is_none_or(|count| count == 0)
                    || row.npz_sha256.is_none()
                    || row.summary_sha256.is_none()
                    || row.wall_seconds.is_none()
                {
                    return Err(format!(
                        "line {line_number}: successful trial lacks a zero exit, raw trace/summary, observed evaluations or wall time"
                    ));
                }
            }
            BoCoDeTrialState::InvalidTrace => {
                if row.npz_sha256.is_none() {
                    return Err(format!(
                        "line {line_number}: invalid_trace must retain the NPZ artifact"
                    ));
                }
            }
            BoCoDeTrialState::Failed | BoCoDeTrialState::WrapperError => {}
        }
        Ok(row)
    }
}

/// Parse exactly one normalized row per attempt. Distinct attempts for the same
/// task/method/seed are allowed; repeated attempt_id or run_id values are not.
pub fn parse_bocode_run_ledger_csv(input: &str) -> Result<Vec<BoCoDeRunLedgerRow>, String> {
    let mut lines = input.lines();
    let header = lines
        .next()
        .ok_or_else(|| "BOCoDe run ledger CSV is empty".to_owned())?
        .trim_end_matches('\r');
    if header != BOCODE_RUN_ROW_HEADER {
        return Err("BOCoDe run ledger CSV header mismatch".to_owned());
    }
    let mut rows = Vec::new();
    let mut attempt_ids = BTreeSet::new();
    let mut run_ids = BTreeSet::new();
    for (index, line) in lines.enumerate() {
        let line_number = index + 2;
        if line.trim().is_empty() {
            return Err(format!(
                "line {line_number}: blank run ledger rows are not permitted"
            ));
        }
        let fields = parse_csv_record(line.trim_end_matches('\r'))
            .map_err(|error| format!("line {line_number}: {error}"))?;
        let row = BoCoDeRunLedgerRow::parse(&fields, line_number)?;
        if !attempt_ids.insert(row.attempt_id.clone()) {
            return Err(format!("line {line_number}: duplicate attempt_id"));
        }
        if !run_ids.insert(row.run_id.clone()) {
            return Err(format!("line {line_number}: duplicate run_id"));
        }
        rows.push(row);
    }
    if rows.is_empty() {
        return Err("BOCoDe run ledger requires at least one trial row".to_owned());
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn success_row() -> String {
        [
            BOCODE_RUN_ROW_SCHEMA,
            "attempt-1",
            &"a".repeat(64),
            &"b".repeat(64),
            "succeeded",
            "verified trial",
            &"c".repeat(40),
            "1.0.2.dev2",
            "Fixture",
            &"d".repeat(64),
            "random_search",
            "7",
            "1",
            "10",
            "11",
            "1",
            "11",
            "0",
            "1.0",
            "0",
            "1.5",
            "0.5",
            "0.1",
            "unknown",
            "unknown",
            "unknown",
            &"e".repeat(64),
            &"f".repeat(64),
            &"1".repeat(64),
            &"2".repeat(64),
            &"3".repeat(64),
            &"4".repeat(64),
        ]
        .join(",")
    }

    #[test]
    fn parses_successful_trial_and_preserves_unknown_resource_states() {
        let csv = format!("{BOCODE_RUN_ROW_HEADER}\n{}\n", success_row());
        let rows = parse_bocode_run_ledger_csv(&csv).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].state, BoCoDeTrialState::Succeeded);
        assert_eq!(rows[0].objective_evaluation_count, Some(11));
        assert_eq!(rows[0].peak_memory_state, BoCoDeResourceState::Unknown);
    }

    #[test]
    fn rejects_false_success_and_inconsistent_evaluation_accounting() {
        let mut fields = parse_csv_record(&success_row()).unwrap();
        fields[19] = "1".to_owned();
        let false_success = format!("{BOCODE_RUN_ROW_HEADER}\n{}\n", fields.join(","));
        assert!(
            parse_bocode_run_ledger_csv(&false_success)
                .unwrap_err()
                .contains("successful trial lacks")
        );

        let mut fields = parse_csv_record(&success_row()).unwrap();
        fields[16] = "10".to_owned();
        let wrong_counts = format!("{BOCODE_RUN_ROW_HEADER}\n{}\n", fields.join(","));
        assert!(
            parse_bocode_run_ledger_csv(&wrong_counts)
                .unwrap_err()
                .contains("evaluation counts differ")
        );
    }

    #[test]
    fn duplicate_attempts_are_not_deduplicated_but_duplicate_ids_are_rejected() {
        let first = success_row();
        let mut fields = parse_csv_record(&first).unwrap();
        fields[1] = "attempt-2".to_owned();
        fields[2] = "9".repeat(64);
        let second = fields.join(",");
        let csv = format!("{BOCODE_RUN_ROW_HEADER}\n{first}\n{second}\n");
        assert_eq!(parse_bocode_run_ledger_csv(&csv).unwrap().len(), 2);

        let duplicate = format!("{BOCODE_RUN_ROW_HEADER}\n{first}\n{first}\n");
        assert!(
            parse_bocode_run_ledger_csv(&duplicate)
                .unwrap_err()
                .contains("duplicate attempt_id")
        );
    }

    #[cfg(unix)]
    #[test]
    fn independent_python_receipt_verifier_passes_and_rejects_tampering() {
        let scripts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts");
        let compile = std::process::Command::new("python3")
            .arg("-m")
            .arg("py_compile")
            .arg(scripts.join("run-bocode-single-trial.py"))
            .arg(scripts.join("verify-bocode-run-receipt.py"))
            .arg(scripts.join("test-bocode-run-receipt.py"))
            .arg(scripts.join("aggregate-bocode-campaign.py"))
            .output()
            .expect("compile BOCoDe run receipt scripts");
        assert!(
            compile.status.success(),
            "BOCoDe receipt script compilation failed: {}{}",
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );

        let self_test = scripts.join("test-bocode-run-receipt.py");
        let output = std::process::Command::new("python3")
            .arg(&self_test)
            .output()
            .expect("run BOCoDe receipt verifier synthetic self-test");
        assert!(
            output.status.success(),
            "BOCoDe receipt self-test failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
