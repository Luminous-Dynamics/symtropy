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
use crate::su2_field_preflight::{Su2FieldPreflightReport, Su2WindowConvergenceProtocol};

pub const SU2_TERMINATION_SCHEMA: &str = "su2-native-termination-evidence/0.3.0";
pub const SU2_TERMINATION_EVIDENCE_LEVEL: &str = "receipt_bound_su2_solver_exit_reason_per_field_steady_rows_or_aggregate_window_success_scoped_to_all_configured_fields_from_exact_full_stdout_stderr_bytes_not_build_attested_not_independent_history_screen_or_physical_validation";

const INTERRUPTED: &str =
    "Interrupt signal received, exiting before the convergence criteria were satisfied.";
const SIGNAL_INTERRUPT_PREFIX: &str = "Interrupt signal (";
const SIGNAL_INTERRUPT_SUFFIX: &str = ") received, saving files and exiting.";
const STEADY_SUCCESS: &str = "All convergence criteria satisfied.";
const WINDOW_SUCCESS: &str = "All windowed time-averaged convergence criteria are fullfilled.";
const STEADY_ITERATION_LIMIT: &str = "Maximum number of iterations reached";
const TIME_LIMIT: &str = "Maximum time reached";
const TIME_ITERATION_LIMIT: &str = "Maximum number of time iterations reached";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Su2NativeCriterionEvidenceScope {
    PerFieldNativeSummaryRow,
    AggregateWindowSuccessBanner,
}

impl Su2NativeCriterionEvidenceScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PerFieldNativeSummaryRow => "per_field_native_summary_row",
            Self::AggregateWindowSuccessBanner => "aggregate_window_success_banner",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Su2NativeCriterionResult {
    /// The configured field supported by this native evidence.
    pub configured_field: String,
    /// Literal SU2 row label for steady criteria, or the explicit aggregate sentinel for window success.
    pub reported_field: String,
    /// Whether this is a literal per-field row or a deduction from SU2's aggregate window-success banner.
    pub evidence_scope: Su2NativeCriterionEvidenceScope,
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
        ) && !self.native_criteria.is_empty()
            && self.native_criteria.iter().all(|criterion| {
                criterion.status == "passed"
                    && match self.mode.as_str() {
                        "steady" => {
                            criterion.evidence_scope
                                == Su2NativeCriterionEvidenceScope::PerFieldNativeSummaryRow
                        }
                        "time_window" => {
                            criterion.evidence_scope
                                == Su2NativeCriterionEvidenceScope::AggregateWindowSuccessBanner
                                && criterion.reported_field == "AGGREGATE_WINDOW_SUCCESS_BANNER"
                        }
                        _ => false,
                    }
            })
    }

    pub fn to_csv(&self) -> String {
        let mut output = String::from(
            "schema,run_id,solver_id,solver_version_claim,runner_receipt_sha256,executable_sha256,stdout_sha256,stderr_sha256,exit_section_sha256,mode,classification,configured_field,reported_field,evidence_scope,criterion_status,summary_line_sha256,terminal_messages,termination_sha256,evidence_level\n",
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
                csv_escape(criterion.evidence_scope.as_str()),
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
        append_string(&mut bytes, criterion.evidence_scope.as_str());
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
            reported_field: if line.contains(cauchy_field.as_str()) {
                cauchy_field
            } else {
                configured.clone()
            },
            status: if status == "Yes" { "passed" } else { "failed" }.to_owned(),
            evidence_scope: Su2NativeCriterionEvidenceScope::PerFieldNativeSummaryRow,
            summary_line_sha256: sha256_hex(line.as_bytes())?,
        });
    }
    Ok(results)
}

fn exit_section(text: &str) -> Result<&str, String> {
    let marker = "Solver Exit";
    let positions = text
        .match_indices(marker)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if positions.len() != 1 {
        return Err(format!(
            "expected exactly one SU2 Solver Exit section, found {}",
            positions.len()
        ));
    }
    Ok(&text[positions[0]..])
}

/// Detect the signal-handler line from the complete receipt-bound log, not only the
/// later Solver Exit section. SU2 prints this line before its driver-specific exit summary.
fn signal_interrupt_message(full_log: &str) -> Result<Option<String>, String> {
    for raw_line in full_log.lines() {
        let line = raw_line.trim();
        if let Some(signal_text) = line.strip_prefix(SIGNAL_INTERRUPT_PREFIX) {
            let Some((signal_number, trailing)) = signal_text.split_once(SIGNAL_INTERRUPT_SUFFIX)
            else {
                return Err(
                    "SU2 signal-handler output has an unrecognized termination message".to_owned(),
                );
            };
            if signal_number.is_empty()
                || !signal_number.bytes().all(|byte| byte.is_ascii_digit())
                || !trailing.trim().is_empty()
            {
                return Err(
                    "SU2 signal-handler output has an invalid signal number or trailing text"
                        .to_owned(),
                );
            }
            return Ok(Some(line.to_owned()));
        }
    }
    Ok(None)
}

fn classify_full_log(
    full_log: &str,
    exit_summary: &str,
    protocol: &crate::su2_field_preflight::Su2ConvergenceProtocol,
) -> Result<(String, String, Vec<Su2NativeCriterionResult>, Vec<String>), String> {
    if let Some(signal_message) = signal_interrupt_message(full_log)? {
        let mode = if !protocol.time_domain {
            "steady"
        } else if matches!(
            &protocol.window,
            Su2WindowConvergenceProtocol::Enabled { .. }
        ) {
            "time_window"
        } else {
            "unsteady_no_window"
        };
        let conflicting_terminal_markers = [
            STEADY_SUCCESS,
            WINDOW_SUCCESS,
            STEADY_ITERATION_LIMIT,
            TIME_LIMIT,
            TIME_ITERATION_LIMIT,
        ]
        .iter()
        .filter(|marker| exit_summary.contains(**marker))
        .map(|marker| (*marker).to_owned())
        .collect::<Vec<_>>();
        if !conflicting_terminal_markers.is_empty() {
            let mut messages = conflicting_terminal_markers;
            messages.push(signal_message);
            return Ok((
                mode.to_owned(),
                "contradictory_terminal_messages".to_owned(),
                Vec::new(),
                messages,
            ));
        }
        return Ok((
            mode.to_owned(),
            "interrupted".to_owned(),
            Vec::new(),
            vec![signal_message],
        ));
    }
    classify_exit_section(exit_summary, protocol)
}

fn unique_exact_message_line<'a>(section: &'a str, message: &str) -> Result<&'a str, String> {
    let matches = section
        .lines()
        .filter(|line| line.trim() == message)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(format!(
            "expected exactly one native SU2 terminal line {message:?}, found {}",
            matches.len()
        ));
    }
    Ok(matches[0])
}

fn classify_exit_section(
    section: &str,
    protocol: &crate::su2_field_preflight::Su2ConvergenceProtocol,
) -> Result<(String, String, Vec<Su2NativeCriterionResult>, Vec<String>), String> {
    let steady_mode =
        !protocol.time_domain && matches!(&protocol.window, Su2WindowConvergenceProtocol::Disabled);
    let window_mode = protocol.time_domain
        && matches!(
            &protocol.window,
            Su2WindowConvergenceProtocol::Enabled { .. }
        );
    let mode = if steady_mode {
        "steady"
    } else if window_mode {
        "time_window"
    } else if protocol.time_domain {
        "unsteady_no_window"
    } else {
        "invalid_steady_window_protocol"
    };
    let terminal_markers = [
        INTERRUPTED,
        STEADY_SUCCESS,
        WINDOW_SUCCESS,
        STEADY_ITERATION_LIMIT,
        TIME_LIMIT,
        TIME_ITERATION_LIMIT,
    ];
    let observed = terminal_markers
        .iter()
        .filter(|marker| section.contains(**marker))
        .map(|marker| (*marker).to_owned())
        .collect::<Vec<_>>();
    if observed.len() > 1 {
        return Ok((
            mode.to_owned(),
            "contradictory_terminal_messages".to_owned(),
            Vec::new(),
            observed,
        ));
    }
    if !steady_mode && !window_mode {
        return Ok((
            mode.to_owned(),
            if protocol.time_domain {
                "unsupported_unsteady_termination_protocol".to_owned()
            } else {
                "invalid_steady_window_protocol".to_owned()
            },
            Vec::new(),
            Vec::new(),
        ));
    }
    if protocol.time_domain && matches!(&protocol.window, Su2WindowConvergenceProtocol::Disabled) {
        return Ok((
            "unsteady_no_window".to_owned(),
            "unsupported_unsteady_termination_protocol".to_owned(),
            Vec::new(),
            Vec::new(),
        ));
    }
    let classification = match &protocol.window {
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
                match parse_steady_native_criteria(section, &protocol.steady_fields) {
                    Ok(criteria) => {
                        let passed = criteria
                            .iter()
                            .all(|criterion| criterion.status == "passed");
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
                let success_line = unique_exact_message_line(section, WINDOW_SUCCESS)?;
                let success_digest = sha256_hex(success_line.as_bytes())?;
                (
                    "time_window".to_owned(),
                    "time_window_native_criteria_satisfied".to_owned(),
                    fields
                        .iter()
                        .map(|field| Su2NativeCriterionResult {
                            configured_field: field.clone(),
                            reported_field: "AGGREGATE_WINDOW_SUCCESS_BANNER".to_owned(),
                            evidence_scope:
                                Su2NativeCriterionEvidenceScope::AggregateWindowSuccessBanner,
                            status: "passed".to_owned(),
                            summary_line_sha256: success_digest.clone(),
                        })
                        .collect(),
                    vec![success_line.to_owned()],
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
    Ok(classification)
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
        return Err(
            "native SU2 termination evidence requires a successful SU2_CFD receipt".to_owned(),
        );
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
        classify_full_log(&combined, section, &preflight.convergence_protocol)?;

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
        return Err(
            "replayed native SU2 termination report differs from supplied report".to_owned(),
        );
    }
    if sha256_hex(&canonical_bytes(report))? != report.termination_sha256 {
        return Err("native SU2 termination canonical digest mismatch".to_owned());
    }
    Ok(())
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::su2_field_preflight::{Su2ConvergenceProtocol, Su2WindowConvergenceProtocol};

    fn steady_protocol() -> Su2ConvergenceProtocol {
        Su2ConvergenceProtocol {
            time_domain: false,
            steady_fields: vec!["DRAG".to_owned(), "RMS_DENSITY".to_owned()],
            steady_start_iteration: Some(10),
            steady_residual_min_log10: Some("-8".to_owned()),
            steady_cauchy_epsilon: Some("1E-10".to_owned()),
            steady_cauchy_elements: Some(100),
            window: Su2WindowConvergenceProtocol::Disabled,
        }
    }

    fn window_protocol() -> Su2ConvergenceProtocol {
        Su2ConvergenceProtocol {
            time_domain: true,
            steady_fields: vec!["RMS_DENSITY".to_owned()],
            steady_start_iteration: Some(10),
            steady_residual_min_log10: Some("-8".to_owned()),
            steady_cauchy_epsilon: None,
            steady_cauchy_elements: None,
            window: Su2WindowConvergenceProtocol::Enabled {
                fields: vec!["TAVG_DRAG".to_owned(), "TAVG_LIFT".to_owned()],
                window_start_iter: 500,
                convergence_start_iter: 10,
                cauchy_elements: 10,
                cauchy_epsilon: "1E-3".to_owned(),
                window_function: "HANN_SQUARE".to_owned(),
            },
        }
    }

    #[test]
    fn steady_native_success_requires_every_configured_field_to_say_yes() {
        let section = concat!(
            "----------------------------- Solver Exit -------------------------------\n",
            "All convergence criteria satisfied.\n",
            "| Convergence Field | Value | Criterion | Converged |\n",
            "| CAUCHY_DRAG | 1.0e-12 | < 1.0e-10 | Yes |\n",
            "| RMS_DENSITY | -9.0 | < -8.0 | Yes |\n",
            "-------------------------------------------------------------------------\n",
        );
        let (_, classification, criteria, _) =
            classify_exit_section(section, &steady_protocol()).expect("classify success");
        assert_eq!(classification, "steady_native_criteria_satisfied");
        assert_eq!(criteria.len(), 2);
        assert!(criteria.iter().all(|row| {
            row.status == "passed"
                && row.evidence_scope == Su2NativeCriterionEvidenceScope::PerFieldNativeSummaryRow
        }));
    }

    #[test]
    fn legacy_misleading_banner_with_no_criteria_is_not_native_convergence() {
        let section = concat!(
            "----------------------------- Solver Exit -------------------------------\n",
            "All convergence criteria satisfied.\n",
            "| Convergence Field | Value | Criterion | Converged |\n",
            "| CAUCHY_DRAG | 5.0e-2 | < 1.0e-10 | No |\n",
            "| RMS_DENSITY | -2.0 | < -8.0 | No |\n",
            "-------------------------------------------------------------------------\n",
        );
        let (_, classification, criteria, _) = classify_exit_section(section, &steady_protocol())
            .expect("classify contradictory banner");
        assert_eq!(classification, "steady_native_criteria_not_satisfied");
        assert_eq!(criteria.len(), 2);
        assert!(criteria.iter().all(|row| row.status == "failed"));
        assert!(!criteria.iter().all(|row| row.status == "passed"));
    }

    #[test]
    fn interrupt_and_iteration_limit_are_distinct_from_native_convergence() {
        let interrupted = concat!(
            "----------------------------- Solver Exit -------------------------------\n",
            "Interrupt signal received, exiting before the convergence criteria were satisfied.\n",
        );
        let (_, interrupted_class, _, _) =
            classify_exit_section(interrupted, &steady_protocol()).expect("classify interrupt");
        assert_eq!(interrupted_class, "interrupted");

        let limited = concat!(
            "----------------------------- Solver Exit -------------------------------\n",
            "Maximum number of iterations reached (ITER = 1000) before convergence.\n",
        );
        let (_, limited_class, _, _) =
            classify_exit_section(limited, &steady_protocol()).expect("classify iteration limit");
        assert_eq!(limited_class, "maximum_iterations_reached");
    }

    #[test]
    fn unsteady_endpoint_without_window_protocol_is_never_classified_as_converged() {
        let mut protocol = window_protocol();
        protocol.window = Su2WindowConvergenceProtocol::Disabled;
        let section = concat!(
            "----------------------------- Solver Exit -------------------------------\n",
            "Maximum time reached (MAX_TIME = 2.0s).\n",
        );
        let (mode, classification, criteria, messages) = classify_exit_section(section, &protocol)
            .expect("classify unsupported unsteady endpoint");
        assert_eq!(mode, "unsteady_no_window");
        assert_eq!(classification, "unsupported_unsteady_termination_protocol");
        assert!(criteria.is_empty());
        assert!(messages.is_empty());
    }

    #[test]
    fn scheduler_sigterm_before_window_success_is_always_interrupted() {
        let full_log = concat!(
            "SU2 startup output\n",
            "Interrupt signal (15) received, saving files and exiting.\n",
            "----------------------------- Solver Exit -------------------------------\n",
            "All windowed time-averaged convergence criteria are fullfilled.\n",
            "-------------------------------------------------------------------------\n",
        );
        let section = exit_section(full_log).expect("find Solver Exit section");
        let (_, classification, criteria, messages) =
            classify_full_log(full_log, section, &window_protocol())
                .expect("classify scheduler signal");
        assert_eq!(classification, "interrupted");
        assert!(criteria.is_empty());
        assert_eq!(
            messages,
            vec!["Interrupt signal (15) received, saving files and exiting."]
        );
    }

    #[test]
    fn malformed_scheduler_signal_output_is_not_ignored() {
        let full_log = concat!(
            "Interrupt signal (fifteen) received, saving files and exiting.\n",
            "----------------------------- Solver Exit -------------------------------\n",
            "All windowed time-averaged convergence criteria are fullfilled.\n",
            "-------------------------------------------------------------------------\n",
        );
        let section = exit_section(full_log).expect("find Solver Exit section");
        assert!(
            classify_full_log(full_log, section, &window_protocol())
                .expect_err("malformed signal output must reject")
                .contains("invalid signal number")
        );
    }

    #[test]
    fn steady_signal_handler_line_before_exit_section_overrides_success() {
        let full_log = concat!(
            "Interrupt signal (15) received, saving files and exiting.\n",
            "----------------------------- Solver Exit -------------------------------\n",
            "All convergence criteria satisfied.\n",
            "| Convergence Field | Value | Criterion | Converged |\n",
            "| CAUCHY_DRAG | 1.0e-12 | < 1.0e-10 | Yes |\n",
            "| RMS_DENSITY | -9.0 | < -8.0 | Yes |\n",
            "-------------------------------------------------------------------------\n",
        );
        let section = exit_section(full_log).expect("find Solver Exit section");
        let (_, classification, criteria, _) =
            classify_full_log(full_log, section, &steady_protocol())
                .expect("classify scheduler signal");
        assert_eq!(classification, "interrupted");
        assert!(criteria.is_empty());
    }

    #[test]
    fn scheduler_signal_before_success_banner_is_recorded_as_contradictory() {
        let section = concat!(
            "----------------------------- Solver Exit -------------------------------\\n",
            "All convergence criteria satisfied.\\n",
        );
        let full_log = format!(
            "{}15{}\\n{section}",
            SIGNAL_INTERRUPT_PREFIX, SIGNAL_INTERRUPT_SUFFIX
        );
        let (_, classification, rows, messages) =
            classify_full_log(&full_log, section, &steady_protocol())
                .expect("classify signal preceding successful summary");
        assert_eq!(classification, "contradictory_terminal_messages");
        assert!(rows.is_empty());
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn contradictory_terminal_messages_are_never_classified_as_converged() {
        let steady = concat!(
            "----------------------------- Solver Exit -------------------------------\\n",
            "All convergence criteria satisfied.\\n",
            "Maximum number of iterations reached (ITER = 1000).\\n",
        );
        let (_, steady_class, steady_rows, steady_messages) =
            classify_exit_section(steady, &steady_protocol())
                .expect("classify contradictory steady terminal messages");
        assert_eq!(steady_class, "contradictory_terminal_messages");
        assert!(steady_rows.is_empty());
        assert_eq!(steady_messages.len(), 2);

        let window = concat!(
            "----------------------------- Solver Exit -------------------------------\\n",
            "All windowed time-averaged convergence criteria are fullfilled.\\n",
            "Interrupt signal received, exiting before the convergence criteria were satisfied.\\n",
        );
        let (_, window_class, window_rows, window_messages) =
            classify_exit_section(window, &window_protocol())
                .expect("classify contradictory time-window terminal messages");
        assert_eq!(window_class, "contradictory_terminal_messages");
        assert!(window_rows.is_empty());
        assert_eq!(window_messages.len(), 2);
    }

    #[test]
    fn time_window_success_uses_distinct_native_exit_message_and_all_window_fields() {
        let section = concat!(
            "----------------------------- Solver Exit -------------------------------\n",
            "All windowed time-averaged convergence criteria are fullfilled.\n",
            "-------------------------------------------------------------------------\n",
        );
        let (_, classification, criteria, _) = classify_exit_section(section, &window_protocol())
            .expect("classify time-window success");
        assert_eq!(classification, "time_window_native_criteria_satisfied");
        assert_eq!(criteria.len(), 2);
        assert!(criteria.iter().all(|row| {
            row.status == "passed"
                && row.evidence_scope
                    == Su2NativeCriterionEvidenceScope::AggregateWindowSuccessBanner
                && row.reported_field == "AGGREGATE_WINDOW_SUCCESS_BANNER"
                && row.summary_line_sha256 == sha256_hex(
                    b"All windowed time-averaged convergence criteria are fullfilled."
                )
                .expect("hash exact aggregate banner")
        }));
        assert_eq!(
            criteria
                .iter()
                .map(|row| row.configured_field.as_str())
                .collect::<Vec<_>>(),
            vec!["TAVG_DRAG", "TAVG_LIFT"]
        );

        let duplicate_banner = concat!(
            "----------------------------- Solver Exit -------------------------------\\n",
            "All windowed time-averaged convergence criteria are fullfilled.\\n",
            "All windowed time-averaged convergence criteria are fullfilled.\\n",
            "-------------------------------------------------------------------------\\n",
        );
        assert!(
            classify_exit_section(duplicate_banner, &window_protocol())
                .expect_err("duplicate aggregate banner rejected")
                .contains("found 2")
        );

        let limited = concat!(
            "----------------------------- Solver Exit -------------------------------\n",
            "Maximum number of time iterations reached (TIME_ITER = 10000).\n",
        );
        let (_, limited_class, _, _) =
            classify_exit_section(limited, &window_protocol()).expect("classify time-window limit");
        assert_eq!(limited_class, "maximum_time_iterations_reached");
    }
}

