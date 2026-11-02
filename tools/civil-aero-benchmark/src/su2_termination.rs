//! Receipt-bound classification of native SU2 solver termination.
//!
//! SU2 v8.6.0 distinguishes steady convergence, iteration-limit exits, SIGTERM interruption,
//! and windowed time-average convergence in its Solver Exit output. A zero exit code alone is not
//! a convergence claim: SU2 intentionally saves/reports on scheduler SIGTERM and may exit zero.
//! This module parses the exact receipt-bound log bytes and records the solver's native reason.
//! A separate post-run history screen remains mandatory.

use std::fs;
use std::path::Path;

use crate::external_solver::{read_external_solver_receipt, verify_external_solver_receipt};
use crate::integrity::sha256_hex;
use crate::su2_field_preflight::{
    Su2FieldPreflightReport, Su2WindowConvergenceProtocol,
};

pub const SU2_TERMINATION_SCHEMA: &str = "su2-native-termination-evidence/0.1.0";
pub const SU2_TERMINATION_EVIDENCE_LEVEL: &str =
    "receipt_bound_su2_solver_exit_reason_and_native_criterion_summary_from_exact_stdout_stderr_bytes_not_build_attested_not_independent_history_screen_or_physical_validation";

const INTERRUPTED: &str =
    "Interrupt signal received, exiting before the convergence criteria were satisfied.";
const STEADY_SUCCESS: &str = "All convergence criteria satisfied.";
const WINDOW_SUCCESS: &str =
    "All windowed time-averaged convergence criteria are fullfilled.";
const STEADY_ITERATION_LIMIT: &str = "Maximum number of iterations reached";
const TIME_LIMIT: &str = "Maximum time reached";
const TIME_ITERATION_LIMIT: &str = "Maximum number of time iterations reached";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Su2NativeCriterionResult {
    /// The caller-declared field name, e.g. DRAG or RMS_DENSITY.
    pub configured_field: String,
    /// The field label emitted by COutput::PrintConvergenceSummary, e.g. CAUCHY_DRAG.
    pub reported_field: String,
    pub status: String,
    pub summary_line_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Su2TerminationReport {
    pub schema: String,
    pub run_id: String,
    pub solver_id: String,
    pub solver_version_claim: String,
    pub runner_receipt_sha256: String,
    pub executable_sha256: String,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub exit_section_sha256: String,
    pub mode: String,
    pub classification: String,
    pub native_criteria: Vec<Su2NativeCriterionResult>,
    pub terminal_messages: Vec<String>,
    pub termination_sha256: String,
    pub evidence_level: String,
}

impl Su2TerminationReport {
    pub fn is_converged_for_protocol(&self) -> bool {
        matches!(
            self.classification.as_str(),
            "steady_native_criteria_satisfied" | "time_window_native_criteria_satisfied"
        ) && self
            .native_criteria
            .iter()
            .all(|criterion| criterion.status == "passed")
    }

    pub fn to_csv(&self) -> String {
        let mut output = String::from(
            "schema,run_id,solver_id,solver_version_claim,runner_receipt_sha256,executable_sha256,stdout_sha256,stderr_sha256,exit_section_sha256,mode,classification,configured_field,reported_field,criterion_status,summary_line_sha256,terminal_messages,termination_sha256,evidence_level\n",
        );
        for criterion in &self.native_criteria {
            let fields = [
                csv_escape(&self.schema),
                csv_escape(&self.run_id),
                csv_escape(&self.solver_id),
                csv_escape(&self.solver_version_claim),
                self.runner_receipt_sha256.clone(),
                self.executable_sha256.clone(),
                self.stdout_sha256.clone(),
                self.stderr_sha256.clone(),
                self.exit_section_sha256.clone(),
                csv_escape(&self.mode),
                csv_escape(&self.classification),
                csv_escape(&criterion.configured_field),
                csv_escape(&criterion.reported_field),
                csv_escape(&criterion.status),
                criterion.summary_line_sha256.clone(),
                csv_escape(&self.terminal_messages.join(";")),
                self.termination_sha256.clone(),
                csv_escape(self.evidence_level),
            ];
            output.push_str(&fields.join(","));
            output.push('\n');
        }
        output
    }
}

fn append_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    append_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn canonical_bytes(report: &Su2TerminationReport) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        report.schema.as_str(),
        report.run_id.as_str(),
        report.solver_id.as_str(),
        report.solver_version_claim.as_str(),
        report.runner_receipt_sha256.as_str(),
        report.executable_sha256.as_str(),
        report.stdout_sha256.as_str(),
        report.stderr_sha256.as_str(),
        report.exit_section_sha256.as_str(),
        report.mode.as_str(),
        report.classification.as_str(),
        report.evidence_level,
    ] {
        append_string(&mut bytes, value);
    }
    append_u64(&mut bytes, report.native_criteria.len() as u64);
    for criterion in &report.native_criteria {
        append_string(&mut bytes, &criterion.configured_field);
        append_string(&mut bytes, &criterion.reported_field);
        append_string(&mut bytes, &criterion.status);
        append_string(&mut bytes, &criterion.summary_line_sha256);
    }
    append_u64(&mut bytes, report.terminal_messages.len() as u64);
    for message in &report.terminal_messages {
        append_string(&mut bytes, message);
    }
    bytes
}

fn last_table_cell(line: &str) -> Option<&str> {
    let cells = line
        .split('|')
        .map(str::trim)
        .filter(|cell| !cell.is_empty())
        .collect::<Vec<_>>();
    cells.last().copied()
}

fn parse_steady_native_criteria(
    exit_section: &str,
    configured_fields: &[String],
) -> Result<Vec<Su2NativeCriterionResult>, String> {
    let lines = exit_section.lines().collect::<Vec<_>>();
    let mut results = Vec::with_capacity(configured_fields.len());
    for configured in configured_fields {
        let cauchy_field = format!("CAUCHY_{configured}");
        let mut matching = Vec::new();
        for line in &lines {
            let Some(last_cell) = last_table_cell(line) else {
                continue;
            };
            if !matches!(last_cell, "Yes" | "No") {
                continue;
            }
            let has_field = line.split('|').any(|cell| {
                let field = cell.trim();
                field == configured || field == cauchy_field
            });
            if has_field {
                matching.push((*line, last_cell));
            }
        }
        if matching.len() != 1 {
            return Err(format!(
                "native SU2 convergence summary has {} rows for configured field {configured}; expected exactly one",
                matching.len()
            ));
        }
        let (line, status) = matching[0];
        results.push(Su2NativeCriterionResult {
            configured_field: configured.clone(),
            reported_field: if line.contains(&cauchy_field) {
                cauchy_field
            } else {
                configured.clone()
            },
            status: if status == "Yes" { "passed" } else { "failed" }.to_owned(),
            summary_line_sha256: sha256_hex(line.as_bytes())?,
        });
    }
    Ok(results)
}

fn exit_section(text: &str) -> Result<&str, String> {
    let marker = "Solver Exit";
    let positions = text.match_indices(marker).map(|(index, _)| index).collect::<Vec<_>>();
    if positions.len() != 1 {
        return Err(format!(
            "expected exactly one SU2 Solver Exit section, found {}",
            positions.len()
        ));
    }
    Ok(&text[positions[0]..])
}

/// Classify native SU2 termination using the exact stdout/stderr bytes captured by the production
/// process receipt. Unknown or contradictory messages are retained as unclassified and cannot be
/// admitted by the verified run bundle.
pub fn classify_su2_termination(
    production_run_directory: &Path,
    preflight: &Su2FieldPreflightReport,
) -> Result<Su2TerminationReport, String> {
    let receipt = read_external_solver_receipt(production_run_directory)?;
    verify_external_solver_receipt(production_run_directory, &receipt)?;
    if receipt.solver_id != "SU2_CFD" || !receipt.is_process_success() {
        return Err("native SU2 termination evidence requires a successful SU2_CFD receipt".to_owned());
    }
    let root = fs::canonicalize(production_run_directory)
        .map_err(|error| format!("cannot resolve SU2 production run directory: {error}"))?;
    let stdout = fs::read(root.join("stdout.log"))
        .map_err(|error| format!("cannot read production stdout: {error}"))?;
    let stderr = fs::read(root.join("stderr.log"))
        .map_err(|error| format!("cannot read production stderr: {error}"))?;
    let stdout_text = std::str::from_utf8(&stdout)
        .map_err(|error| format!("production stdout is not UTF-8: {error}"))?;
    let stderr_text = std::str::from_utf8(&stderr)
        .map_err(|error| format!("production stderr is not UTF-8: {error}"))?;
    let combined = format!("{stdout_text}\n{stderr_text}");
    let section = exit_section(&combined)?;
    let section_sha256 = sha256_hex(section.as_bytes())?;
    let stdout_sha256 = sha256_hex(&stdout)?;
    let stderr_sha256 = sha256_hex(&stderr)?;
    if stdout_sha256 != receipt.stdout_sha256 || stderr_sha256 != receipt.stderr_sha256 {
        return Err("production stdout/stderr digest differs from process receipt".to_owned());
    }

    let (mode, classification, native_criteria, terminal_messages) =
        match &preflight.convergence_protocol.window {
            Su2WindowConvergenceProtocol::Disabled => {
                if section.contains(INTERRUPTED) {
                    (
                        "steady".to_owned(),
                        "interrupted".to_owned(),
                        Vec::new(),
                        vec![INTERRUPTED.to_owned()],
                    )
                } else if section.contains(STEADY_ITERATION_LIMIT) {
                    (
                        "steady".to_owned(),
                        "maximum_iterations_reached".to_owned(),
                        Vec::new(),
                        vec![STEADY_ITERATION_LIMIT.to_owned()],
                    )
                } else if section.contains(STEADY_SUCCESS) {
                    match parse_steady_native_criteria(
                        section,
                        &preflight.convergence_protocol.steady_fields,
                    ) {
                        Ok(criteria) => {
                            let passed = criteria.iter().all(|criterion| criterion.status == "passed");
                            (
                                "steady".to_owned(),
                                if passed {
                                    "steady_native_criteria_satisfied"
                                } else {
                                    "steady_native_criteria_not_satisfied"
                                }
                                .to_owned(),
                                criteria,
                                vec![STEADY_SUCCESS.to_owned()],
                            )
                        }
                        Err(error) => (
                            "steady".to_owned(),
                            "unclassified_native_steady_summary".to_owned(),
                            Vec::new(),
                            vec![error],
                        ),
                    }
                } else {
                    (
                        "steady".to_owned(),
                        "unclassified_steady_termination".to_owned(),
                        Vec::new(),
                        Vec::new(),
                    )
                }
            }
            Su2WindowConvergenceProtocol::Enabled { fields, .. } => {
                if section.contains(INTERRUPTED) {
                    (
                        "time_window".to_owned(),
                        "interrupted".to_owned(),
                        Vec::new(),
                        vec![INTERRUPTED.to_owned()],
                    )
                } else if section.contains(WINDOW_SUCCESS) {
                    let success_digest = sha256_hex(WINDOW_SUCCESS.as_bytes())?;
                    (
                        "time_window".to_owned(),
                        "time_window_native_criteria_satisfied".to_owned(),
                        fields
                            .iter()
                            .map(|field| Su2NativeCriterionResult {
                                configured_field: field.clone(),
                                reported_field: field.clone(),
                                status: "passed".to_owned(),
                                summary_line_sha256: success_digest.clone(),
                            })
                            .collect(),
                        vec![WINDOW_SUCCESS.to_owned()],
                    )
                } else if section.contains(TIME_LIMIT) {
                    (
                        "time_window".to_owned(),
                        "maximum_time_reached".to_owned(),
                        Vec::new(),
                        vec![TIME_LIMIT.to_owned()],
                    )
                } else if section.contains(TIME_ITERATION_LIMIT) {
                    (
                        "time_window".to_owned(),
                        "maximum_time_iterations_reached".to_owned(),
                        Vec::new(),
                        vec![TIME_ITERATION_LIMIT.to_owned()],
                    )
                } else {
                    (
                        "time_window".to_owned(),
                        "unclassified_time_window_termination".to_owned(),
                        Vec::new(),
                        Vec::new(),
                    )
                }
            }
        };

    let mut report = Su2TerminationReport {
        schema: SU2_TERMINATION_SCHEMA.to_owned(),
        run_id: receipt.run_id,
        solver_id: receipt.solver_id,
        solver_version_claim: receipt.solver_version_claim,
        runner_receipt_sha256: receipt.receipt_sha256,
        executable_sha256: receipt.executable_sha256,
        stdout_sha256,
        stderr_sha256,
        exit_section_sha256: section_sha256,
        mode,
        classification,
        native_criteria,
        terminal_messages,
        termination_sha256: String::new(),
        evidence_level: SU2_TERMINATION_EVIDENCE_LEVEL.to_owned(),
    };
    report.termination_sha256 = sha256_hex(&canonical_bytes(&report))?;
    Ok(report)
}

/// Replay native termination classification against the same production process receipt.
pub fn verify_su2_termination(
    production_run_directory: &Path,
    preflight: &Su2FieldPreflightReport,
    report: &Su2TerminationReport,
) -> Result<(), String> {
    if report.schema != SU2_TERMINATION_SCHEMA
        || report.evidence_level != SU2_TERMINATION_EVIDENCE_LEVEL
    {
        return Err("unsupported native SU2 termination schema or evidence ceiling".to_owned());
    }
    let recomputed = classify_su2_termination(production_run_directory, preflight)?;
    if recomputed != *report {
        return Err("replayed native SU2 termination report differs from supplied report".to_owned());
    }
    if sha256_hex(&canonical_bytes(report))? != report.termination_sha256 {
        return Err("native SU2 termination canonical digest mismatch".to_owned());
    }
    Ok(())
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}
