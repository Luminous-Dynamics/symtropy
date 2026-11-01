//! Caller-declared SU2 residual and force-monitor threshold screening.
//!
//! This deliberately narrow screen reports whether a recorded history meets explicit numeric
//! thresholds. It is not a universal CFD convergence detector and does not establish conservation,
//! spatial/temporal convergence, validation accuracy, or airworthiness.

use crate::integrity::sha256_hex;
use crate::su2_history::{Su2HistoryReport, verify_su2_history_import};

#[derive(Debug, Clone, PartialEq)]
pub struct ResidualCriterion {
    pub column: String,
    pub maximum_final_absolute_value: f64,
    pub maximum_final_to_initial_ratio: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorCriterion {
    pub column: String,
    pub trailing_window_rows: usize,
    pub maximum_absolute_range: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConvergenceCriteria {
    pub residuals: Vec<ResidualCriterion>,
    pub monitors: Vec<MonitorCriterion>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CriterionStatus {
    Passed,
    Failed,
    InsufficientRows,
}

impl CriterionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::InsufficientRows => "insufficient_rows",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CriterionResult {
    pub kind: String,
    pub column: String,
    pub unit: String,
    pub rows_used: usize,
    pub observed_initial_absolute: Option<f64>,
    pub observed_final_absolute: Option<f64>,
    pub observed_final_to_initial_ratio: Option<f64>,
    pub maximum_final_absolute: Option<f64>,
    pub maximum_final_to_initial_ratio: Option<f64>,
    pub observed_window_range: Option<f64>,
    pub maximum_window_range: Option<f64>,
    pub status: CriterionStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConvergenceScreen {
    pub run_id: String,
    pub runner_receipt_sha256: String,
    pub history_sha256: String,
    pub history_import_sha256: String,
    pub status: String,
    pub passed_count: usize,
    pub failed_count: usize,
    pub insufficient_count: usize,
    pub results: Vec<CriterionResult>,
    pub screen_sha256: String,
    pub evidence_level: &'static str,
}

pub const CONVERGENCE_SCREEN_SCHEMA: &str = "su2-declared-convergence-screen/0.1.0";
pub const CONVERGENCE_SCREEN_EVIDENCE_LEVEL: &str = "declared_residual_and_monitor_threshold_screen_only_not_complete_CFD_convergence_grid_independence_or_physical_validation";

fn add_string(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn add_number(bytes: &mut Vec<u8>, value: Option<f64>) {
    match value {
        Some(number) => {
            bytes.push(1);
            bytes.extend_from_slice(&number.to_bits().to_le_bytes());
        }
        None => bytes.push(0),
    }
}

fn canonical_screen_bytes(screen: &ConvergenceScreen) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        CONVERGENCE_SCREEN_SCHEMA,
        screen.run_id.as_str(),
        screen.runner_receipt_sha256.as_str(),
        screen.history_sha256.as_str(),
        screen.history_import_sha256.as_str(),
        screen.status.as_str(),
        screen.evidence_level,
    ] {
        add_string(&mut bytes, value);
    }
    bytes.extend_from_slice(&(screen.results.len() as u64).to_le_bytes());
    for result in &screen.results {
        for value in [
            result.kind.as_str(),
            result.column.as_str(),
            result.unit.as_str(),
            result.status.as_str(),
        ] {
            add_string(&mut bytes, value);
        }
        bytes.extend_from_slice(&(result.rows_used as u64).to_le_bytes());
        for value in [
            result.observed_initial_absolute,
            result.observed_final_absolute,
            result.observed_final_to_initial_ratio,
            result.maximum_final_absolute,
            result.maximum_final_to_initial_ratio,
            result.observed_window_range,
            result.maximum_window_range,
        ] {
            add_number(&mut bytes, value);
        }
    }
    bytes
}

/// Screen a receipt-bound import using explicit residual and monitor limits.
///
/// Thresholds are case/solver/configuration-specific caller declarations. A pass applies only to
/// these declared columns and thresholds; omitted equations/monitors are not implicitly covered.
pub fn screen_su2_history(
    report: &Su2HistoryReport,
    criteria: &ConvergenceCriteria,
) -> Result<ConvergenceScreen, String> {
    verify_su2_history_import(report)?;
    if criteria.residuals.is_empty() || criteria.monitors.is_empty() {
        return Err("at least one residual and one monitor criterion are required".to_owned());
    }
    let mut used = std::collections::BTreeSet::new();
    let mut results = Vec::new();

    for criterion in &criteria.residuals {
        if criterion.column.trim().is_empty()
            || !criterion.maximum_final_absolute_value.is_finite()
            || criterion.maximum_final_absolute_value < 0.0
            || !criterion.maximum_final_to_initial_ratio.is_finite()
            || !(0.0..=1.0).contains(&criterion.maximum_final_to_initial_ratio)
            || !used.insert(criterion.column.as_str())
        {
            return Err(format!(
                "invalid or duplicate residual criterion: {}",
                criterion.column
            ));
        }
        let index = report
            .numeric_fields
            .iter()
            .position(|field| field.column == criterion.column)
            .ok_or_else(|| format!("residual field {} was not imported", criterion.column))?;
        let field = &report.numeric_fields[index];
        let initial = report.points[0].values[index].abs();
        let final_value = report
            .points
            .last()
            .expect("non-empty history verified")
            .values[index]
            .abs();
        let ratio = if initial == 0.0 {
            (final_value == 0.0).then_some(0.0)
        } else {
            Some(final_value / initial)
        };
        let passed = final_value <= criterion.maximum_final_absolute_value
            && ratio.is_some_and(|value| value <= criterion.maximum_final_to_initial_ratio);
        results.push(CriterionResult {
            kind: "residual_final_absolute_and_reduction_ratio".to_owned(),
            column: criterion.column.clone(),
            unit: field.unit.clone(),
            rows_used: report.points.len(),
            observed_initial_absolute: Some(initial),
            observed_final_absolute: Some(final_value),
            observed_final_to_initial_ratio: ratio,
            maximum_final_absolute: Some(criterion.maximum_final_absolute_value),
            maximum_final_to_initial_ratio: Some(criterion.maximum_final_to_initial_ratio),
            observed_window_range: None,
            maximum_window_range: None,
            status: if passed {
                CriterionStatus::Passed
            } else {
                CriterionStatus::Failed
            },
        });
    }

    for criterion in &criteria.monitors {
        if criterion.column.trim().is_empty()
            || criterion.trailing_window_rows < 2
            || !criterion.maximum_absolute_range.is_finite()
            || criterion.maximum_absolute_range < 0.0
            || !used.insert(criterion.column.as_str())
        {
            return Err(format!(
                "invalid or duplicate monitor criterion: {}",
                criterion.column
            ));
        }
        let index = report
            .numeric_fields
            .iter()
            .position(|field| field.column == criterion.column)
            .ok_or_else(|| format!("monitor field {} was not imported", criterion.column))?;
        let field = &report.numeric_fields[index];
        if report.points.len() < criterion.trailing_window_rows {
            results.push(CriterionResult {
                kind: "trailing_window_absolute_range".to_owned(),
                column: criterion.column.clone(),
                unit: field.unit.clone(),
                rows_used: report.points.len(),
                observed_initial_absolute: None,
                observed_final_absolute: None,
                observed_final_to_initial_ratio: None,
                maximum_final_absolute: None,
                maximum_final_to_initial_ratio: None,
                observed_window_range: None,
                maximum_window_range: Some(criterion.maximum_absolute_range),
                status: CriterionStatus::InsufficientRows,
            });
            continue;
        }
        let window = report.points[report.points.len() - criterion.trailing_window_rows..]
            .iter()
            .map(|point| point.values[index]);
        let mut minimum = f64::INFINITY;
        let mut maximum = f64::NEG_INFINITY;
        for value in window {
            minimum = minimum.min(value);
            maximum = maximum.max(value);
        }
        let range = maximum - minimum;
        if !range.is_finite() {
            return Err(format!("monitor range overflowed for {}", criterion.column));
        }
        results.push(CriterionResult {
            kind: "trailing_window_absolute_range".to_owned(),
            column: criterion.column.clone(),
            unit: field.unit.clone(),
            rows_used: criterion.trailing_window_rows,
            observed_initial_absolute: None,
            observed_final_absolute: None,
            observed_final_to_initial_ratio: None,
            maximum_final_absolute: None,
            maximum_final_to_initial_ratio: None,
            observed_window_range: Some(range),
            maximum_window_range: Some(criterion.maximum_absolute_range),
            status: if range <= criterion.maximum_absolute_range {
                CriterionStatus::Passed
            } else {
                CriterionStatus::Failed
            },
        });
    }

    let passed_count = results
        .iter()
        .filter(|row| row.status == CriterionStatus::Passed)
        .count();
    let failed_count = results
        .iter()
        .filter(|row| row.status == CriterionStatus::Failed)
        .count();
    let insufficient_count = results
        .iter()
        .filter(|row| row.status == CriterionStatus::InsufficientRows)
        .count();
    let status = match (failed_count, insufficient_count) {
        (0, 0) => "declared_thresholds_met",
        (_, 0) => "declared_thresholds_not_met",
        (0, _) => "incomplete_insufficient_rows",
        _ => "declared_thresholds_not_met_and_incomplete",
    }
    .to_owned();
    let mut screen = ConvergenceScreen {
        run_id: report.run_id.clone(),
        runner_receipt_sha256: report.runner_receipt_sha256.clone(),
        history_sha256: report.history_sha256.clone(),
        history_import_sha256: report.import_sha256.clone(),
        status,
        passed_count,
        failed_count,
        insufficient_count,
        results,
        screen_sha256: String::new(),
        evidence_level: CONVERGENCE_SCREEN_EVIDENCE_LEVEL,
    };
    screen.screen_sha256 = sha256_hex(&canonical_screen_bytes(&screen))?;
    Ok(screen)
}

pub fn screen_to_csv(screen: &ConvergenceScreen) -> String {
    let mut output = "schema,run_id,runner_receipt_sha256,history_sha256,history_import_sha256,criterion_kind,column,unit,rows_used,observed_initial_absolute,observed_final_absolute,observed_final_to_initial_ratio,maximum_final_absolute,maximum_final_to_initial_ratio,observed_window_range,maximum_window_range,criterion_status,overall_status,passed_count,failed_count,insufficient_count,screen_sha256,evidence_level\n".to_owned();
    let optional = |value: Option<f64>| value.map(|number| number.to_string()).unwrap_or_default();
    for row in &screen.results {
        let fields = [
            CONVERGENCE_SCREEN_SCHEMA.to_owned(),
            screen.run_id.clone(),
            screen.runner_receipt_sha256.clone(),
            screen.history_sha256.clone(),
            screen.history_import_sha256.clone(),
            row.kind.clone(),
            row.column.clone(),
            row.unit.clone(),
            row.rows_used.to_string(),
            optional(row.observed_initial_absolute),
            optional(row.observed_final_absolute),
            optional(row.observed_final_to_initial_ratio),
            optional(row.maximum_final_absolute),
            optional(row.maximum_final_to_initial_ratio),
            optional(row.observed_window_range),
            optional(row.maximum_window_range),
            row.status.as_str().to_owned(),
            screen.status.clone(),
            screen.passed_count.to_string(),
            screen.failed_count.to_string(),
            screen.insufficient_count.to_string(),
            screen.screen_sha256.clone(),
            screen.evidence_level.to_owned(),
        ];
        output.push_str(
            &fields
                .iter()
                .map(|value| format!("\"{}\"", value.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(","),
        );
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report() -> Su2HistoryReport {
        Su2HistoryReport {
            schema: crate::su2_history::SU2_HISTORY_IMPORT_SCHEMA.to_owned(),
            run_id: "synthetic-run".to_owned(),
            solver_id: "SU2_CFD".to_owned(),
            solver_version_claim: "fixture".to_owned(),
            executable_sha256: "a".repeat(64),
            runner_receipt_sha256: "b".repeat(64),
            history_relative_path: "history.csv".to_owned(),
            history_byte_length: 10,
            history_sha256: "c".repeat(64),
            iteration_column: "Iteration".to_owned(),
            expected_header: vec![
                "Iteration".to_owned(),
                "RMS_RES".to_owned(),
                "CL".to_owned(),
            ],
            numeric_fields: vec![
                crate::su2_history::Su2HistoryField {
                    column: "RMS_RES".to_owned(),
                    unit: "1".to_owned(),
                },
                crate::su2_history::Su2HistoryField {
                    column: "CL".to_owned(),
                    unit: "1".to_owned(),
                },
            ],
            points: vec![
                crate::su2_history::Su2HistoryPoint {
                    iteration: 0,
                    values: vec![1.0, 0.4],
                },
                crate::su2_history::Su2HistoryPoint {
                    iteration: 1,
                    values: vec![0.1, 0.41],
                },
                crate::su2_history::Su2HistoryPoint {
                    iteration: 2,
                    values: vec![0.001, 0.4102],
                },
            ],
            import_sha256: String::new(),
            evidence_level: crate::su2_history::SU2_HISTORY_EVIDENCE_LEVEL.to_owned(),
        }
    }

    #[test]
    fn applies_declared_residual_reduction_and_monitor_range_limits() {
        let mut report = report();
        report.import_sha256 = crate::su2_history::su2_history_import_sha256(&report)
            .expect("seal synthetic typed history fixture");
        let criteria = ConvergenceCriteria {
            residuals: vec![ResidualCriterion {
                column: "RMS_RES".to_owned(),
                maximum_final_absolute_value: 0.002,
                maximum_final_to_initial_ratio: 0.002,
            }],
            monitors: vec![MonitorCriterion {
                column: "CL".to_owned(),
                trailing_window_rows: 2,
                maximum_absolute_range: 0.001,
            }],
        };
        let result = screen_su2_history(&report, &criteria).expect("valid criteria screen");
        assert_eq!(result.status, "declared_thresholds_met");
        assert_eq!(result.passed_count, 2);
        assert_eq!(result.failed_count, 0);
    }

    #[test]
    fn failed_threshold_and_insufficient_window_are_not_silently_passed() {
        let mut report = report();
        report.import_sha256 = crate::su2_history::su2_history_import_sha256(&report)
            .expect("seal synthetic typed history fixture");
        let criteria = ConvergenceCriteria {
            residuals: vec![ResidualCriterion {
                column: "RMS_RES".to_owned(),
                maximum_final_absolute_value: 1.0e-5,
                maximum_final_to_initial_ratio: 1.0e-4,
            }],
            monitors: vec![MonitorCriterion {
                column: "CL".to_owned(),
                trailing_window_rows: 4,
                maximum_absolute_range: 0.001,
            }],
        };
        let result = screen_su2_history(&report, &criteria).expect("audit returns failed status");
        assert_eq!(result.status, "declared_thresholds_not_met_and_incomplete");
        assert_eq!(result.failed_count, 1);
        assert_eq!(result.insufficient_count, 1);
    }
}
