//! Caller-declared SU2 residual and force-monitor convergence screening.
//!
//! Residual encoding is explicit: SU2 residual history fields are often log10 values, not
//! positive linear magnitudes. The screen checks final residual, total reduction and terminal
//! stability. Engineering monitors are checked for trailing range, split-window mean drift and
//! fitted trend. These checks are caller-configured and do not establish conservation, spatial or
//! temporal convergence, validation accuracy, or airworthiness.

use crate::integrity::sha256_hex;
use crate::su2_history::{Su2HistoryReport, verify_su2_history_import};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResidualAcceptance {
    Log10 {
        maximum_final_log10_residual: f64,
        minimum_log10_reduction: f64,
        maximum_terminal_span_log10: f64,
    },
    PositiveLinear {
        maximum_final_value: f64,
        maximum_final_to_initial_ratio: f64,
        maximum_terminal_range: f64,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResidualCriterion {
    pub column: String,
    pub unit: String,
    pub terminal_window_rows: usize,
    pub acceptance: ResidualAcceptance,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorCriterion {
    pub column: String,
    pub unit: String,
    /// Even-sized trailing window split into equal early and late halves.
    pub trailing_window_rows: usize,
    pub maximum_absolute_range: f64,
    pub maximum_absolute_mean_drift: f64,
    /// Absolute fitted slope in field units per 1,000 outer iterations.
    pub maximum_absolute_slope_per_1000_iterations: f64,
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
    pub observed_initial_log10_residual: Option<f64>,
    pub observed_final_log10_residual: Option<f64>,
    pub observed_log10_reduction: Option<f64>,
    pub observed_terminal_log10_span: Option<f64>,
    pub maximum_final_log10_residual: Option<f64>,
    pub minimum_log10_reduction: Option<f64>,
    pub maximum_terminal_log10_span: Option<f64>,
    pub observed_mean_drift: Option<f64>,
    pub maximum_mean_drift: Option<f64>,
    pub observed_slope_per_1000_iterations: Option<f64>,
    pub maximum_slope_per_1000_iterations: Option<f64>,
    pub status: CriterionStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConvergenceScreen {
    pub run_id: String,
    pub solver_id: String,
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

pub const CONVERGENCE_SCREEN_SCHEMA: &str = "su2-declared-convergence-screen/0.3.0";
pub const CONVERGENCE_SCREEN_EVIDENCE_LEVEL: &str = "declared_residual_encoding_terminal_stability_and_monitor_screen_only_not_complete_CFD_convergence_grid_independence_or_physical_validation";

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
        screen.solver_id.as_str(),
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
            result.observed_initial_log10_residual,
            result.observed_final_log10_residual,
            result.observed_log10_reduction,
            result.observed_terminal_log10_span,
            result.maximum_final_log10_residual,
            result.minimum_log10_reduction,
            result.maximum_terminal_log10_span,
            result.observed_mean_drift,
            result.maximum_mean_drift,
            result.observed_slope_per_1000_iterations,
            result.maximum_slope_per_1000_iterations,
        ] {
            add_number(&mut bytes, value);
        }
    }
    bytes
}

fn validate_limit(name: &str, value: f64, nonnegative: bool) -> Result<(), String> {
    if !value.is_finite() || (nonnegative && value < 0.0) {
        return Err(format!(
            "{name} must be finite and {}",
            if nonnegative {
                "non-negative"
            } else {
                "finite"
            }
        ));
    }
    Ok(())
}

fn validate_criteria(
    report: &Su2HistoryReport,
    criteria: &ConvergenceCriteria,
) -> Result<(), String> {
    if criteria.residuals.is_empty() || criteria.monitors.is_empty() {
        return Err(
            "at least one residual and one engineering monitor criterion are required".to_owned(),
        );
    }
    let field_units = report
        .numeric_fields
        .iter()
        .map(|field| (field.column.as_str(), field.unit.as_str()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut used = std::collections::BTreeSet::new();

    for criterion in &criteria.residuals {
        if criterion.column.trim().is_empty() || criterion.unit.trim().is_empty() {
            return Err("residual column and unit must be non-empty".to_owned());
        }
        if !used.insert(criterion.column.as_str()) {
            return Err(format!("duplicate convergence field {}", criterion.column));
        }
        if field_units.get(criterion.column.as_str()).copied() != Some(criterion.unit.as_str()) {
            return Err(format!(
                "residual field {} was not imported with the declared unit {}",
                criterion.column, criterion.unit
            ));
        }
        if criterion.terminal_window_rows < 2 {
            return Err(format!(
                "residual {} terminal_window_rows must be at least two",
                criterion.column
            ));
        }
        match criterion.acceptance {
            ResidualAcceptance::Log10 {
                maximum_final_log10_residual,
                minimum_log10_reduction,
                maximum_terminal_span_log10,
            } => {
                validate_limit(
                    "maximum_final_log10_residual",
                    maximum_final_log10_residual,
                    false,
                )?;
                validate_limit("minimum_log10_reduction", minimum_log10_reduction, true)?;
                validate_limit(
                    "maximum_terminal_span_log10",
                    maximum_terminal_span_log10,
                    true,
                )?;
                if maximum_final_log10_residual > 0.0 {
                    return Err("maximum_final_log10_residual must be <= 0".to_owned());
                }
            }
            ResidualAcceptance::PositiveLinear {
                maximum_final_value,
                maximum_final_to_initial_ratio,
                maximum_terminal_range,
            } => {
                validate_limit("maximum_final_value", maximum_final_value, true)?;
                validate_limit(
                    "maximum_final_to_initial_ratio",
                    maximum_final_to_initial_ratio,
                    true,
                )?;
                validate_limit("maximum_terminal_range", maximum_terminal_range, true)?;
                if maximum_final_to_initial_ratio > 1.0 {
                    return Err("maximum_final_to_initial_ratio must be <= 1".to_owned());
                }
            }
        }
    }

    for criterion in &criteria.monitors {
        if criterion.column.trim().is_empty() || criterion.unit.trim().is_empty() {
            return Err("monitor column and unit must be non-empty".to_owned());
        }
        if !used.insert(criterion.column.as_str()) {
            return Err(format!("duplicate convergence field {}", criterion.column));
        }
        if field_units.get(criterion.column.as_str()).copied() != Some(criterion.unit.as_str()) {
            return Err(format!(
                "monitor field {} was not imported with the declared unit {}",
                criterion.column, criterion.unit
            ));
        }
        if criterion.trailing_window_rows < 4 || criterion.trailing_window_rows % 2 != 0 {
            return Err(format!(
                "monitor {} trailing_window_rows must be even and at least four",
                criterion.column
            ));
        }
        validate_limit(
            "maximum_absolute_range",
            criterion.maximum_absolute_range,
            true,
        )?;
        validate_limit(
            "maximum_absolute_mean_drift",
            criterion.maximum_absolute_mean_drift,
            true,
        )?;
        validate_limit(
            "maximum_absolute_slope_per_1000_iterations",
            criterion.maximum_absolute_slope_per_1000_iterations,
            true,
        )?;
    }
    Ok(())
}

fn residual_log10(value: f64, positive_linear: bool, field: &str) -> Result<f64, String> {
    if positive_linear {
        if value <= 0.0 {
            return Err(format!(
                "linear residual field {field} must be strictly positive"
            ));
        }
        Ok(value.log10())
    } else {
        Ok(value)
    }
}

fn mean(values: &[f64]) -> f64 {
    let divisor = values.len() as f64;
    values.iter().map(|value| value / divisor).sum::<f64>()
}

fn slope_per_1000_iterations(
    iterations: &[u64],
    values: &[f64],
    field: &str,
) -> Result<f64, String> {
    if iterations.len() != values.len() || iterations.len() < 2 {
        return Err(format!(
            "monitor {field} requires at least two matching samples"
        ));
    }
    let first_iteration = iterations[0];
    let x_values = iterations
        .iter()
        .map(|value| value.saturating_sub(first_iteration) as f64)
        .collect::<Vec<_>>();
    let x_mean = mean(&x_values);
    let y_mean = mean(values);
    let variance = x_values
        .iter()
        .map(|value| (value - x_mean).powi(2))
        .sum::<f64>();
    if !variance.is_finite() || variance <= 0.0 {
        return Err(format!(
            "monitor {field} iteration coordinates are singular"
        ));
    }
    let covariance = x_values
        .iter()
        .zip(values)
        .map(|(x, y)| (x - x_mean) * (y - y_mean))
        .sum::<f64>();
    let slope = covariance / variance * 1000.0;
    if !slope.is_finite() {
        return Err(format!("monitor {field} slope is non-finite"));
    }
    Ok(slope.abs())
}

/// Apply explicit case-specific residual and engineering-monitor limits to a receipt-bound import.
///
/// Log10 residuals are never treated as linear magnitudes. A pass means only that every declared
/// criterion passed; omitted equations or monitors remain outside the claim.
pub fn screen_su2_history(
    report: &Su2HistoryReport,
    criteria: &ConvergenceCriteria,
) -> Result<ConvergenceScreen, String> {
    verify_su2_history_import(report)?;
    validate_criteria(report, criteria)?;
    let indices = report
        .numeric_fields
        .iter()
        .enumerate()
        .map(|(index, field)| (field.column.as_str(), index))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut results = Vec::new();

    for criterion in &criteria.residuals {
        let index = indices[criterion.column.as_str()];
        if report.points.len() < criterion.terminal_window_rows {
            results.push(empty_result(
                &criterion.column,
                &criterion.unit,
                "residual_terminal_window",
                report.points.len(),
                CriterionStatus::InsufficientRows,
            ));
            continue;
        }
        let values = report
            .points
            .iter()
            .map(|point| point.values[index])
            .collect::<Vec<_>>();
        let initial = values[0];
        let final_value = *values.last().expect("verified non-empty imported history");
        let terminal = &values[values.len() - criterion.terminal_window_rows..];
        let mut result = empty_result(
            &criterion.column,
            &criterion.unit,
            match criterion.acceptance {
                ResidualAcceptance::Log10 { .. } => {
                    "residual_log10_final_reduction_and_terminal_span"
                }
                ResidualAcceptance::PositiveLinear { .. } => {
                    "residual_linear_final_reduction_and_terminal_range"
                }
            },
            report.points.len(),
            CriterionStatus::Failed,
        );
        match criterion.acceptance {
            ResidualAcceptance::Log10 {
                maximum_final_log10_residual,
                minimum_log10_reduction,
                maximum_terminal_span_log10,
            } => {
                let initial_log = residual_log10(initial, false, &criterion.column)?;
                let final_log = residual_log10(final_value, false, &criterion.column)?;
                let terminal_logs = terminal
                    .iter()
                    .map(|value| residual_log10(*value, false, &criterion.column))
                    .collect::<Result<Vec<_>, _>>()?;
                let min = terminal_logs.iter().copied().fold(f64::INFINITY, f64::min);
                let max = terminal_logs
                    .iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max);
                let reduction = initial_log - final_log;
                let span = max - min;
                if !reduction.is_finite() || !span.is_finite() {
                    return Err(format!(
                        "log10 residual arithmetic overflowed for {}",
                        criterion.column
                    ));
                }
                result.observed_initial_log10_residual = Some(initial_log);
                result.observed_final_log10_residual = Some(final_log);
                result.observed_log10_reduction = Some(reduction);
                result.observed_terminal_log10_span = Some(span);
                result.maximum_final_log10_residual = Some(maximum_final_log10_residual);
                result.minimum_log10_reduction = Some(minimum_log10_reduction);
                result.maximum_terminal_log10_span = Some(maximum_terminal_span_log10);
                let passed = final_log <= maximum_final_log10_residual
                    && reduction >= minimum_log10_reduction
                    && span <= maximum_terminal_span_log10;
                result.status = if passed {
                    CriterionStatus::Passed
                } else {
                    CriterionStatus::Failed
                };
            }
            ResidualAcceptance::PositiveLinear {
                maximum_final_value,
                maximum_final_to_initial_ratio,
                maximum_terminal_range,
            } => {
                if initial <= 0.0
                    || final_value <= 0.0
                    || terminal.iter().any(|value| *value <= 0.0)
                {
                    return Err(format!(
                        "positive-linear residual {} contains a non-positive value",
                        criterion.column
                    ));
                }
                let ratio = final_value / initial;
                let minimum = terminal.iter().copied().fold(f64::INFINITY, f64::min);
                let maximum = terminal.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let range = maximum - minimum;
                if !ratio.is_finite() || !range.is_finite() {
                    return Err(format!(
                        "linear residual arithmetic overflowed for {}",
                        criterion.column
                    ));
                }
                result.observed_initial_absolute = Some(initial);
                result.observed_final_absolute = Some(final_value);
                result.observed_final_to_initial_ratio = Some(ratio);
                result.maximum_final_absolute = Some(maximum_final_value);
                result.maximum_final_to_initial_ratio = Some(maximum_final_to_initial_ratio);
                result.observed_window_range = Some(range);
                result.maximum_window_range = Some(maximum_terminal_range);
                let passed = final_value <= maximum_final_value
                    && ratio <= maximum_final_to_initial_ratio
                    && range <= maximum_terminal_range;
                result.status = if passed {
                    CriterionStatus::Passed
                } else {
                    CriterionStatus::Failed
                };
            }
        }
        results.push(result);
    }

    for criterion in &criteria.monitors {
        let index = indices[criterion.column.as_str()];
        if report.points.len() < criterion.trailing_window_rows {
            results.push(empty_result(
                &criterion.column,
                &criterion.unit,
                "monitor_terminal_range_mean_drift_and_slope",
                report.points.len(),
                CriterionStatus::InsufficientRows,
            ));
            continue;
        }
        let terminal_points =
            &report.points[report.points.len() - criterion.trailing_window_rows..];
        let values = terminal_points
            .iter()
            .map(|point| point.values[index])
            .collect::<Vec<_>>();
        let half = values.len() / 2;
        let early_mean = mean(&values[..half]);
        let late_mean = mean(&values[half..]);
        let drift = (late_mean - early_mean).abs();
        let minimum = values.iter().copied().fold(f64::INFINITY, f64::min);
        let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let range = maximum - minimum;
        let iterations = terminal_points
            .iter()
            .map(|point| point.iteration)
            .collect::<Vec<_>>();
        let slope = slope_per_1000_iterations(&iterations, &values, &criterion.column)?;
        if !early_mean.is_finite()
            || !late_mean.is_finite()
            || !drift.is_finite()
            || !range.is_finite()
        {
            return Err(format!(
                "monitor arithmetic overflowed for {}",
                criterion.column
            ));
        }
        let passed = range <= criterion.maximum_absolute_range
            && drift <= criterion.maximum_absolute_mean_drift
            && slope <= criterion.maximum_absolute_slope_per_1000_iterations;
        let mut result = empty_result(
            &criterion.column,
            &criterion.unit,
            "monitor_terminal_range_mean_drift_and_slope",
            criterion.trailing_window_rows,
            if passed {
                CriterionStatus::Passed
            } else {
                CriterionStatus::Failed
            },
        );
        result.observed_window_range = Some(range);
        result.maximum_window_range = Some(criterion.maximum_absolute_range);
        result.observed_mean_drift = Some(drift);
        result.maximum_mean_drift = Some(criterion.maximum_absolute_mean_drift);
        result.observed_slope_per_1000_iterations = Some(slope);
        result.maximum_slope_per_1000_iterations =
            Some(criterion.maximum_absolute_slope_per_1000_iterations);
        results.push(result);
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
        solver_id: report.solver_id.clone(),
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

fn empty_result(
    column: &str,
    unit: &str,
    kind: &str,
    rows_used: usize,
    status: CriterionStatus,
) -> CriterionResult {
    CriterionResult {
        kind: kind.to_owned(),
        column: column.to_owned(),
        unit: unit.to_owned(),
        rows_used,
        observed_initial_absolute: None,
        observed_final_absolute: None,
        observed_final_to_initial_ratio: None,
        maximum_final_absolute: None,
        maximum_final_to_initial_ratio: None,
        observed_window_range: None,
        maximum_window_range: None,
        observed_initial_log10_residual: None,
        observed_final_log10_residual: None,
        observed_log10_reduction: None,
        observed_terminal_log10_span: None,
        maximum_final_log10_residual: None,
        minimum_log10_reduction: None,
        maximum_terminal_log10_span: None,
        observed_mean_drift: None,
        maximum_mean_drift: None,
        observed_slope_per_1000_iterations: None,
        maximum_slope_per_1000_iterations: None,
        status,
    }
}

/// Verify aggregate status counts and recompute the canonical screen digest before downstream binding.
pub fn verify_convergence_screen(screen: &ConvergenceScreen) -> Result<(), String> {
    if screen.evidence_level != CONVERGENCE_SCREEN_EVIDENCE_LEVEL || screen.results.is_empty() {
        return Err("convergence screen evidence level or result set is invalid".to_owned());
    }
    let passed = screen
        .results
        .iter()
        .filter(|row| row.status == CriterionStatus::Passed)
        .count();
    let failed = screen
        .results
        .iter()
        .filter(|row| row.status == CriterionStatus::Failed)
        .count();
    let insufficient = screen
        .results
        .iter()
        .filter(|row| row.status == CriterionStatus::InsufficientRows)
        .count();
    if (passed, failed, insufficient)
        != (
            screen.passed_count,
            screen.failed_count,
            screen.insufficient_count,
        )
    {
        return Err(
            "convergence screen aggregate counts do not match criterion statuses".to_owned(),
        );
    }
    let expected_status = match (failed, insufficient) {
        (0, 0) => "declared_thresholds_met",
        (_, 0) => "declared_thresholds_not_met",
        (0, _) => "incomplete_insufficient_rows",
        _ => "declared_thresholds_not_met_and_incomplete",
    };
    if screen.status != expected_status {
        return Err(
            "convergence screen aggregate status does not match criterion statuses".to_owned(),
        );
    }
    let mut columns = std::collections::BTreeSet::new();
    if screen.results.iter().any(|row| {
        row.column.trim().is_empty()
            || row.unit.trim().is_empty()
            || row.kind.trim().is_empty()
            || !columns.insert(row.column.as_str())
    }) {
        return Err("convergence screen contains an empty or duplicate criterion field".to_owned());
    }
    if screen.run_id.trim().is_empty()
        || screen.solver_id.trim().is_empty()
        || !is_sha256(&screen.runner_receipt_sha256)
        || !is_sha256(&screen.history_sha256)
        || !is_sha256(&screen.history_import_sha256)
        || !is_sha256(&screen.screen_sha256)
    {
        return Err("convergence screen run identity or digest is malformed".to_owned());
    }
    if sha256_hex(&canonical_screen_bytes(screen))? != screen.screen_sha256 {
        return Err("convergence screen canonical digest mismatch".to_owned());
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub fn screen_to_csv(screen: &ConvergenceScreen) -> String {
    let header = [
        "schema",
        "run_id",
        "solver_id",
        "runner_receipt_sha256",
        "history_sha256",
        "history_import_sha256",
        "criterion_kind",
        "column",
        "unit",
        "rows_used",
        "observed_initial_absolute",
        "observed_final_absolute",
        "observed_final_to_initial_ratio",
        "maximum_final_absolute",
        "maximum_final_to_initial_ratio",
        "observed_window_range",
        "maximum_window_range",
        "observed_initial_log10_residual",
        "observed_final_log10_residual",
        "observed_log10_reduction",
        "observed_terminal_log10_span",
        "maximum_final_log10_residual",
        "minimum_log10_reduction",
        "maximum_terminal_log10_span",
        "observed_mean_drift",
        "maximum_mean_drift",
        "observed_slope_per_1000_iterations",
        "maximum_slope_per_1000_iterations",
        "criterion_status",
        "overall_status",
        "passed_count",
        "failed_count",
        "insufficient_count",
        "screen_sha256",
        "evidence_level",
    ];
    let mut output = header.join(",");
    output.push('\n');
    for row in &screen.results {
        let fields = [
            CONVERGENCE_SCREEN_SCHEMA.to_owned(),
            screen.run_id.clone(),
            screen.solver_id.clone(),
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
            optional(row.observed_initial_log10_residual),
            optional(row.observed_final_log10_residual),
            optional(row.observed_log10_reduction),
            optional(row.observed_terminal_log10_span),
            optional(row.maximum_final_log10_residual),
            optional(row.minimum_log10_reduction),
            optional(row.maximum_terminal_log10_span),
            optional(row.observed_mean_drift),
            optional(row.maximum_mean_drift),
            optional(row.observed_slope_per_1000_iterations),
            optional(row.maximum_slope_per_1000_iterations),
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
                .map(|value| csv_escape(value))
                .collect::<Vec<_>>()
                .join(","),
        );
        output.push('\n');
    }
    output
}

fn optional(value: Option<f64>) -> String {
    value
        .map(|number| number.to_string())
        .unwrap_or_default()
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::su2_history::{Su2HistoryField, Su2HistoryPoint};

    fn report() -> Su2HistoryReport {
        let fields = vec![
            Su2HistoryField {
                column: "CL".to_owned(),
                unit: "dimensionless".to_owned(),
            },
            Su2HistoryField {
                column: "CD".to_owned(),
                unit: "dimensionless".to_owned(),
            },
            Su2HistoryField {
                column: "RMS_DENSITY".to_owned(),
                unit: "log10(residual)".to_owned(),
            },
            Su2HistoryField {
                column: "RMS_MOMENTUM-X".to_owned(),
                unit: "log10(residual)".to_owned(),
            },
        ];
        let values = [
            (0, 0.4100, 0.0210, -1.0, -1.0),
            (10, 0.4200, 0.0200, -2.0, -2.0),
            (20, 0.4230, 0.01820, -6.5, -6.5),
            (30, 0.42305, 0.01815, -6.8, -6.8),
            (40, 0.42310, 0.01810, -7.1, -7.1),
            (50, 0.42310, 0.01808, -7.4, -7.4),
            (60, 0.42315, 0.01806, -7.7, -7.7),
            (70, 0.42316, 0.01805, -8.0, -8.0),
        ];
        let mut report = Su2HistoryReport {
            schema: crate::su2_history::SU2_HISTORY_IMPORT_SCHEMA.to_owned(),
            run_id: "synthetic-run".to_owned(),
            solver_id: "SU2_CFD".to_owned(),
            solver_version_claim: "synthetic-fixture-not-a-SU2-version".to_owned(),
            executable_sha256: "a".repeat(64),
            runner_receipt_sha256: "b".repeat(64),
            history_relative_path: "history.csv".to_owned(),
            history_byte_length: 10,
            history_sha256: "c".repeat(64),
            iteration_column: "Iteration".to_owned(),
            expected_header: vec![
                "Iteration".to_owned(),
                "CL".to_owned(),
                "CD".to_owned(),
                "RMS_DENSITY".to_owned(),
                "RMS_MOMENTUM-X".to_owned(),
            ],
            numeric_fields: fields,
            points: values
                .iter()
                .map(|(iteration, cl, cd, rho, momentum)| Su2HistoryPoint {
                    iteration: *iteration,
                    values: vec![*cl, *cd, *rho, *momentum],
                })
                .collect(),
            import_sha256: String::new(),
            evidence_level: crate::su2_history::SU2_HISTORY_EVIDENCE_LEVEL.to_owned(),
        };
        report.import_sha256 =
            crate::su2_history::su2_history_import_sha256(&report).expect("seal synthetic history");
        report
    }

    fn criteria() -> ConvergenceCriteria {
        ConvergenceCriteria {
            residuals: vec![
                ResidualCriterion {
                    column: "RMS_DENSITY".to_owned(),
                    unit: "log10(residual)".to_owned(),
                    terminal_window_rows: 6,
                    acceptance: ResidualAcceptance::Log10 {
                        maximum_final_log10_residual: -8.0,
                        minimum_log10_reduction: 6.0,
                        maximum_terminal_span_log10: 2.0,
                    },
                },
                ResidualCriterion {
                    column: "RMS_MOMENTUM-X".to_owned(),
                    unit: "log10(residual)".to_owned(),
                    terminal_window_rows: 6,
                    acceptance: ResidualAcceptance::Log10 {
                        maximum_final_log10_residual: -8.0,
                        minimum_log10_reduction: 6.0,
                        maximum_terminal_span_log10: 2.0,
                    },
                },
            ],
            monitors: vec![
                MonitorCriterion {
                    column: "CL".to_owned(),
                    unit: "dimensionless".to_owned(),
                    trailing_window_rows: 6,
                    maximum_absolute_range: 0.0004,
                    maximum_absolute_mean_drift: 0.0003,
                    maximum_absolute_slope_per_1000_iterations: 0.02,
                },
                MonitorCriterion {
                    column: "CD".to_owned(),
                    unit: "dimensionless".to_owned(),
                    trailing_window_rows: 6,
                    maximum_absolute_range: 0.0004,
                    maximum_absolute_mean_drift: 0.0003,
                    maximum_absolute_slope_per_1000_iterations: 0.02,
                },
            ],
        }
    }

    #[test]
    fn log10_residuals_and_multiple_force_monitors_pass_all_declared_checks() {
        let report = screen_su2_history(&report(), &criteria()).expect("valid steady history");
        assert_eq!(report.status, "declared_thresholds_met");
        assert_eq!(report.passed_count, 4);
        assert_eq!(report.failed_count, 0);
        assert!(
            report
                .results
                .iter()
                .all(|row| row.status == CriterionStatus::Passed)
        );
        assert_eq!(screen_to_csv(&report).lines().count(), 5);
    }

    #[test]
    fn low_final_log_residual_does_not_hide_insufficient_reduction_or_terminal_instability() {
        let mut history = report();
        for (index, value) in [-1.5, -2.0, -3.0, -4.0, -5.0, -8.0].iter().enumerate() {
            history.points[index + 2].values[2] = *value;
        }
        history.import_sha256 = crate::su2_history::su2_history_import_sha256(&history)
            .expect("reseal modified synthetic history");
        let screen = screen_su2_history(&history, &criteria()).expect("well-formed failed screen");
        assert_eq!(screen.status, "declared_thresholds_not_met");
        assert_eq!(screen.results[0].status, CriterionStatus::Failed);
    }

    #[test]
    fn oscillatory_monitor_fails_even_when_early_and_late_means_match() {
        let mut history = report();
        for (point, value) in history.points[2..]
            .iter_mut()
            .zip([0.4230, 0.4235, 0.4230, 0.4235, 0.4230, 0.4235])
        {
            point.values[0] = value;
        }
        history.import_sha256 = crate::su2_history::su2_history_import_sha256(&history)
            .expect("reseal oscillatory history");
        let screen =
            screen_su2_history(&history, &criteria()).expect("well-formed oscillatory history");
        assert_eq!(screen.results[2].status, CriterionStatus::Failed);
    }

    #[test]
    fn wrong_unit_missing_field_duplicate_field_and_insufficient_window_fail_closed() {
        let history = report();
        let mut invalid = criteria();
        invalid.residuals[0].unit = "dimensionless".to_owned();
        assert!(screen_su2_history(&history, &invalid).is_err());
        let mut invalid = criteria();
        invalid.monitors[0].column = "MISSING".to_owned();
        assert!(screen_su2_history(&history, &invalid).is_err());
        let mut invalid = criteria();
        invalid.monitors.push(invalid.monitors[0].clone());
        assert!(screen_su2_history(&history, &invalid).is_err());
        let mut invalid = criteria();
        invalid.residuals[0].terminal_window_rows = 100;
        assert_eq!(
            screen_su2_history(&history, &invalid)
                .expect("insufficient rows is a recorded result")
                .results[0]
                .status,
            CriterionStatus::InsufficientRows
        );
    }

    #[test]
    fn positive_linear_residual_encoding_is_explicit_and_checked() {
        let mut history = report();
        history.numeric_fields[2].unit = "linear residual".to_owned();
        for point in &mut history.points {
            point.values[2] = 10_f64.powf(point.values[2]);
        }
        history.import_sha256 = crate::su2_history::su2_history_import_sha256(&history)
            .expect("seal positive linear residual fixture");
        let mut adjusted = criteria();
        adjusted.residuals[0].unit = "linear residual".to_owned();
        adjusted.residuals[0].acceptance = ResidualAcceptance::PositiveLinear {
            maximum_final_value: 1.0e-8,
            maximum_final_to_initial_ratio: 1.0e-6,
            maximum_terminal_range: 1.0e-6,
        };
        let screen =
            screen_su2_history(&history, &adjusted).expect("explicit linear residual screen");
        assert_eq!(screen.results[0].status, CriterionStatus::Passed);

        history.points[0].values[2] = 0.0;
        history.import_sha256 = crate::su2_history::su2_history_import_sha256(&history)
            .expect("seal zero linear residual negative control");
        assert!(screen_su2_history(&history, &adjusted).is_err());
    }
}
