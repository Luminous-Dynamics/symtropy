//! Independent steady-state convergence assessment over receipt-bound SU2 history.
//!
//! The audit is separate from SU2's configured stop condition. It requires multiple equation
//! residuals and multiple engineering monitors, each with explicit units and acceptance limits.
//! Unsteady/periodic solutions require a different time-domain assessment and must not be passed
//! through this steady-state classifier.

use std::collections::BTreeSet;

use crate::integrity::sha256_hex;
use crate::su2_history::{Su2HistoryReport, verify_su2_history_import};

pub const STEADY_CONVERGENCE_SCHEMA: &str = "su2-steady-convergence-audit/0.1.0";
pub const STEADY_CONVERGENCE_EVIDENCE_LEVEL: &str =
    "steady_history_acceptance_under_declared_criteria_only_not_grid_convergence_or_validation";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidualEncoding {
    Log10,
    PositiveLinear,
}

impl ResidualEncoding {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Log10 => "log10",
            Self::PositiveLinear => "positive_linear",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResidualCriterion {
    pub field: String,
    pub unit: String,
    pub encoding: ResidualEncoding,
    pub maximum_final_log10_residual: f64,
    pub minimum_log10_reduction: f64,
    pub maximum_terminal_span_log10: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorCriterion {
    pub field: String,
    pub unit: String,
    pub absolute_mean_drift_tolerance: f64,
    pub relative_mean_drift_tolerance: f64,
    pub absolute_range_tolerance: f64,
    pub relative_range_tolerance: f64,
    /// Absolute fitted slope in field units per 1,000 outer iterations.
    pub maximum_absolute_slope_per_1000_iterations: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SteadyConvergenceCriteria {
    /// Even-sized trailing window; split into equal early/late halves.
    pub terminal_window_samples: usize,
    pub residuals: Vec<ResidualCriterion>,
    pub monitors: Vec<MonitorCriterion>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResidualConvergenceAudit {
    pub field: String,
    pub unit: String,
    pub encoding: ResidualEncoding,
    pub initial_log10_residual: f64,
    pub final_log10_residual: f64,
    pub log10_reduction: f64,
    pub terminal_span_log10: f64,
    pub final_residual_pass: bool,
    pub reduction_pass: bool,
    pub terminal_stability_pass: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorConvergenceAudit {
    pub field: String,
    pub unit: String,
    pub early_window_mean: f64,
    pub late_window_mean: f64,
    pub absolute_mean_drift: f64,
    pub allowed_mean_drift: f64,
    pub terminal_range: f64,
    pub allowed_terminal_range: f64,
    pub absolute_slope_per_1000_iterations: f64,
    pub maximum_absolute_slope_per_1000_iterations: f64,
    pub mean_drift_pass: bool,
    pub range_pass: bool,
    pub slope_pass: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SteadyConvergenceReport {
    pub run_id: String,
    pub solver_id: String,
    pub executable_sha256: String,
    pub runner_receipt_sha256: String,
    pub history_sha256: String,
    pub history_import_sha256: String,
    pub terminal_window_samples: usize,
    pub residual_audits: Vec<ResidualConvergenceAudit>,
    pub monitor_audits: Vec<MonitorConvergenceAudit>,
    pub status: String,
    pub evidence_level: String,
}

impl SteadyConvergenceReport {
    pub fn is_converged(&self) -> bool {
        self.status == "steady_history_converged_under_declared_criteria"
    }

    pub fn to_csv(&self) -> String {
        let header = [
            "schema",
            "run_id",
            "solver_id",
            "executable_sha256",
            "runner_receipt_sha256",
            "history_sha256",
            "history_import_sha256",
            "overall_status",
            "record_type",
            "field",
            "unit",
            "encoding",
            "initial_log10_residual",
            "final_log10_residual",
            "log10_reduction",
            "terminal_span_log10",
            "early_window_mean",
            "late_window_mean",
            "absolute_mean_drift",
            "allowed_mean_drift",
            "terminal_range",
            "allowed_terminal_range",
            "absolute_slope_per_1000_iterations",
            "maximum_absolute_slope_per_1000_iterations",
            "checks_passed",
            "checks_total",
            "evidence_level",
        ];
        let mut output = header.join(",");
        output.push('\n');

        let checks_total = self.residual_audits.len() * 3 + self.monitor_audits.len() * 3;
        let checks_passed = self
            .residual_audits
            .iter()
            .map(|audit| {
                audit.final_residual_pass as usize
                    + audit.reduction_pass as usize
                    + audit.terminal_stability_pass as usize
            })
            .sum::<usize>()
            + self
                .monitor_audits
                .iter()
                .map(|audit| {
                    audit.mean_drift_pass as usize
                        + audit.range_pass as usize
                        + audit.slope_pass as usize
                })
                .sum::<usize>();

        for audit in &self.residual_audits {
            let mut fields = vec![
                csv_escape(STEADY_CONVERGENCE_SCHEMA),
                csv_escape(&self.run_id),
                csv_escape(&self.solver_id),
                self.executable_sha256.clone(),
                self.runner_receipt_sha256.clone(),
                self.history_sha256.clone(),
                self.history_import_sha256.clone(),
                csv_escape(&self.status),
                "residual".to_owned(),
                csv_escape(&audit.field),
                csv_escape(&audit.unit),
                audit.encoding.as_str().to_owned(),
                decimal(audit.initial_log10_residual),
                decimal(audit.final_log10_residual),
                decimal(audit.log10_reduction),
                decimal(audit.terminal_span_log10),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                checks_passed.to_string(),
                checks_total.to_string(),
                csv_escape(&self.evidence_level),
            ];
            output.push_str(&fields.join(","));
            output.push('\n');
        }
        for audit in &self.monitor_audits {
            let fields = vec![
                csv_escape(STEADY_CONVERGENCE_SCHEMA),
                csv_escape(&self.run_id),
                csv_escape(&self.solver_id),
                self.executable_sha256.clone(),
                self.runner_receipt_sha256.clone(),
                self.history_sha256.clone(),
                self.history_import_sha256.clone(),
                csv_escape(&self.status),
                "monitor".to_owned(),
                csv_escape(&audit.field),
                csv_escape(&audit.unit),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                decimal(audit.early_window_mean),
                decimal(audit.late_window_mean),
                decimal(audit.absolute_mean_drift),
                decimal(audit.allowed_mean_drift),
                decimal(audit.terminal_range),
                decimal(audit.allowed_terminal_range),
                decimal(audit.absolute_slope_per_1000_iterations),
                decimal(audit.maximum_absolute_slope_per_1000_iterations),
                checks_passed.to_string(),
                checks_total.to_string(),
                csv_escape(&self.evidence_level),
            ];
            output.push_str(&fields.join(","));
            output.push('\n');
        }
        output
    }
}

fn decimal(value: f64) -> String {
    format!("{value:.17}")
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn validate_criteria(
    import: &Su2HistoryReport,
    criteria: &SteadyConvergenceCriteria,
) -> Result<(), String> {
    if criteria.terminal_window_samples < 4
        || criteria.terminal_window_samples % 2 != 0
        || criteria.terminal_window_samples > import.points().len()
    {
        return Err(
            "terminal_window_samples must be even, >= 4, and <= imported history length"
                .to_owned(),
        );
    }
    if criteria.residuals.is_empty() || criteria.monitors.is_empty() {
        return Err("at least one residual and one engineering monitor are required".to_owned());
    }
    let fields = import
        .numeric_fields()
        .iter()
        .map(|field| (field.column.as_str(), field.unit.as_str()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut seen = BTreeSet::new();

    for criterion in &criteria.residuals {
        if criterion.field.trim().is_empty() || criterion.unit.trim().is_empty() {
            return Err("residual field and unit must be non-empty".to_owned());
        }
        if !seen.insert(criterion.field.as_str()) {
            return Err(format!("duplicate convergence field {}", criterion.field));
        }
        if fields.get(criterion.field.as_str()).copied() != Some(criterion.unit.as_str()) {
            return Err(format!(
                "residual field {} is absent from the import or has a unit mismatch",
                criterion.field
            ));
        }
        for (name, value) in [
            (
                "maximum_final_log10_residual",
                criterion.maximum_final_log10_residual,
            ),
            (
                "minimum_log10_reduction",
                criterion.minimum_log10_reduction,
            ),
            (
                "maximum_terminal_span_log10",
                criterion.maximum_terminal_span_log10,
            ),
        ] {
            if !value.is_finite() {
                return Err(format!(
                    "residual {} criterion {name} must be finite",
                    criterion.field
                ));
            }
        }
        if criterion.maximum_final_log10_residual > 0.0
            || criterion.minimum_log10_reduction < 0.0
            || criterion.maximum_terminal_span_log10 < 0.0
        {
            return Err(format!(
                "residual {} criteria require final log10 <= 0 and non-negative reduction/span limits",
                criterion.field
            ));
        }
    }

    for criterion in &criteria.monitors {
        if criterion.field.trim().is_empty() || criterion.unit.trim().is_empty() {
            return Err("monitor field and unit must be non-empty".to_owned());
        }
        if !seen.insert(criterion.field.as_str()) {
            return Err(format!("duplicate convergence field {}", criterion.field));
        }
        if fields.get(criterion.field.as_str()).copied() != Some(criterion.unit.as_str()) {
            return Err(format!(
                "monitor field {} is absent from the import or has a unit mismatch",
                criterion.field
            ));
        }
        for (name, value) in [
            ("absolute_mean_drift_tolerance", criterion.absolute_mean_drift_tolerance),
            ("relative_mean_drift_tolerance", criterion.relative_mean_drift_tolerance),
            ("absolute_range_tolerance", criterion.absolute_range_tolerance),
            ("relative_range_tolerance", criterion.relative_range_tolerance),
            (
                "maximum_absolute_slope_per_1000_iterations",
                criterion.maximum_absolute_slope_per_1000_iterations,
            ),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(format!(
                    "monitor {} criterion {name} must be finite and non-negative",
                    criterion.field
                ));
            }
        }
    }
    Ok(())
}

fn residual_log10(value: f64, encoding: ResidualEncoding, field: &str) -> Result<f64, String> {
    match encoding {
        ResidualEncoding::Log10 => Ok(value),
        ResidualEncoding::PositiveLinear if value > 0.0 => Ok(value.log10()),
        ResidualEncoding::PositiveLinear => Err(format!(
            "residual field {field} must be strictly positive in positive_linear encoding"
        )),
    }
}

fn mean(values: &[f64]) -> f64 {
    let divisor = values.len() as f64;
    values.iter().map(|value| value / divisor).sum::<f64>()
}

fn linear_slope_per_1000_iterations(
    iterations: &[u64],
    values: &[f64],
    field: &str,
) -> Result<f64, String> {
    if iterations.len() != values.len() || iterations.len() < 2 {
        return Err(format!(
            "monitor {field} requires matching iteration/value samples"
        ));
    }
    let first_iteration = iterations[0];
    let xs = iterations
        .iter()
        .map(|value| value.saturating_sub(first_iteration) as f64)
        .collect::<Vec<_>>();
    let x_mean = mean(&xs);
    let y_mean = mean(values);
    let variance = xs.iter().map(|value| (value - x_mean).powi(2)).sum::<f64>();
    if !variance.is_finite() || variance <= 0.0 {
        return Err(format!("monitor {field} iteration coordinates are singular"));
    }
    let covariance = xs
        .iter()
        .zip(values)
        .map(|(x, y)| (x - x_mean) * (y - y_mean))
        .sum::<f64>();
    let slope = covariance / variance * 1000.0;
    if !slope.is_finite() {
        return Err(format!("monitor {field} slope is non-finite"));
    }
    Ok(slope)
}

/// Assess a steady-state SU2 history import against explicit residual and coefficient criteria.
///
/// This function re-verifies the import's canonical digest. It does not use the solver's claimed
/// stop reason as proof and does not decide temporal, grid, or experimental convergence.
pub fn assess_steady_convergence(
    import: &Su2HistoryReport,
    criteria: &SteadyConvergenceCriteria,
) -> Result<SteadyConvergenceReport, String> {
    verify_su2_history_import(import)?;
    validate_criteria(import, criteria)?;

    let field_indices = import
        .numeric_fields()
        .iter()
        .enumerate()
        .map(|(index, field)| (field.column.as_str(), index))
        .collect::<std::collections::BTreeMap<_, _>>();
    let all_points = import.points();
    let window_start = all_points.len() - criteria.terminal_window_samples;
    let terminal_points = &all_points[window_start..];
    let half = criteria.terminal_window_samples / 2;

    let mut residual_audits = Vec::with_capacity(criteria.residuals.len());
    for criterion in &criteria.residuals {
        let index = field_indices[criterion.field.as_str()];
        let initial_log10 = residual_log10(
            all_points[0].values[index],
            criterion.encoding,
            &criterion.field,
        )?;
        let final_log10 = residual_log10(
            all_points
                .last()
                .expect("verified history is non-empty")
                .values[index],
            criterion.encoding,
            &criterion.field,
        )?;
        let terminal = terminal_points
            .iter()
            .map(|point| residual_log10(point.values[index], criterion.encoding, &criterion.field))
            .collect::<Result<Vec<_>, _>>()?;
        let terminal_min = terminal.iter().copied().fold(f64::INFINITY, f64::min);
        let terminal_max = terminal.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let reduction = initial_log10 - final_log10;
        let span = terminal_max - terminal_min;
        let final_residual_pass = final_log10 <= criterion.maximum_final_log10_residual;
        let reduction_pass = reduction >= criterion.minimum_log10_reduction;
        let terminal_stability_pass = span <= criterion.maximum_terminal_span_log10;
        residual_audits.push(ResidualConvergenceAudit {
            field: criterion.field.clone(),
            unit: criterion.unit.clone(),
            encoding: criterion.encoding,
            initial_log10_residual: initial_log10,
            final_log10_residual: final_log10,
            log10_reduction: reduction,
            terminal_span_log10: span,
            final_residual_pass,
            reduction_pass,
            terminal_stability_pass,
            passed: final_residual_pass && reduction_pass && terminal_stability_pass,
        });
    }

    let iterations = terminal_points
        .iter()
        .map(|point| point.iteration)
        .collect::<Vec<_>>();
    let mut monitor_audits = Vec::with_capacity(criteria.monitors.len());
    for criterion in &criteria.monitors {
        let index = field_indices[criterion.field.as_str()];
        let values = terminal_points
            .iter()
            .map(|point| point.values[index])
            .collect::<Vec<_>>();
        let early_mean = mean(&values[..half]);
        let late_mean = mean(&values[half..]);
        let scale = early_mean.abs().max(late_mean.abs());
        let drift = (late_mean - early_mean).abs();
        let allowed_drift = criterion.absolute_mean_drift_tolerance
            + criterion.relative_mean_drift_tolerance * scale;
        let minimum = values.iter().copied().fold(f64::INFINITY, f64::min);
        let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let terminal_range = maximum - minimum;
        let allowed_range = criterion.absolute_range_tolerance
            + criterion.relative_range_tolerance * scale;
        let slope = linear_slope_per_1000_iterations(&iterations, &values, &criterion.field)?;
        let mean_drift_pass = drift <= allowed_drift;
        let range_pass = terminal_range <= allowed_range;
        let slope_pass = slope.abs() <= criterion.maximum_absolute_slope_per_1000_iterations;
        monitor_audits.push(MonitorConvergenceAudit {
            field: criterion.field.clone(),
            unit: criterion.unit.clone(),
            early_window_mean: early_mean,
            late_window_mean: late_mean,
            absolute_mean_drift: drift,
            allowed_mean_drift: allowed_drift,
            terminal_range,
            allowed_terminal_range: allowed_range,
            absolute_slope_per_1000_iterations: slope.abs(),
            maximum_absolute_slope_per_1000_iterations: criterion.maximum_absolute_slope_per_1000_iterations,
            mean_drift_pass,
            range_pass,
            slope_pass,
            passed: mean_drift_pass && range_pass && slope_pass,
        });
    }

    let passed = residual_audits.iter().all(|audit| audit.passed)
        && monitor_audits.iter().all(|audit| audit.passed);
    Ok(SteadyConvergenceReport {
        run_id: import.run_id().to_owned(),
        solver_id: import.solver_id().to_owned(),
        executable_sha256: import.executable_sha256().to_owned(),
        runner_receipt_sha256: import.runner_receipt_sha256().to_owned(),
        history_sha256: import.history_sha256().to_owned(),
        history_import_sha256: import.import_sha256().to_owned(),
        terminal_window_samples: criteria.terminal_window_samples,
        residual_audits,
        monitor_audits,
        status: if passed {
            "steady_history_converged_under_declared_criteria".to_owned()
        } else {
            "steady_history_not_converged_under_declared_criteria".to_owned()
        },
        evidence_level: STEADY_CONVERGENCE_EVIDENCE_LEVEL.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::su2_history::{Su2HistoryField, Su2HistoryPoint, su2_history_import_sha256};

    fn synthetic_import() -> Su2HistoryReport {
        let fields = vec![
            Su2HistoryField { column: "CL".to_owned(), unit: "dimensionless".to_owned() },
            Su2HistoryField { column: "CD".to_owned(), unit: "dimensionless".to_owned() },
            Su2HistoryField { column: "RMS_DENSITY".to_owned(), unit: "log10(residual)".to_owned() },
            Su2HistoryField { column: "RMS_MOMENTUM-X".to_owned(), unit: "log10(residual)".to_owned() },
        ];
        let raw = [
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
            run_id: "synthetic-steady-run".to_owned(),
            solver_id: "SU2_CFD".to_owned(),
            solver_version_claim: "fixture-not-a-version".to_owned(),
            executable_sha256: "1".repeat(64),
            runner_receipt_sha256: "2".repeat(64),
            history_relative_path: "history.csv".to_owned(),
            history_byte_length: 1024,
            history_sha256: "3".repeat(64),
            iteration_column: "Iteration".to_owned(),
            expected_header: vec![
                "Iteration".to_owned(), "CL".to_owned(), "CD".to_owned(),
                "RMS_DENSITY".to_owned(), "RMS_MOMENTUM-X".to_owned(),
            ],
            numeric_fields: fields,
            points: raw.iter().map(|(iteration, cl, cd, rho, mom)| Su2HistoryPoint {
                iteration: *iteration,
                values: vec![*cl, *cd, *rho, *mom],
            }).collect(),
            import_sha256: String::new(),
            evidence_level: crate::su2_history::SU2_HISTORY_EVIDENCE_LEVEL.to_owned(),
        };
        report.import_sha256 = su2_history_import_sha256(&report).expect("canonical digest");
        report
    }

    fn criteria() -> SteadyConvergenceCriteria {
        SteadyConvergenceCriteria {
            terminal_window_samples: 6,
            residuals: vec![
                ResidualCriterion {
                    field: "RMS_DENSITY".to_owned(),
                    unit: "log10(residual)".to_owned(),
                    encoding: ResidualEncoding::Log10,
                    maximum_final_log10_residual: -8.0,
                    minimum_log10_reduction: 6.0,
                    maximum_terminal_span_log10: 2.0,
                },
                ResidualCriterion {
                    field: "RMS_MOMENTUM-X".to_owned(),
                    unit: "log10(residual)".to_owned(),
                    encoding: ResidualEncoding::Log10,
                    maximum_final_log10_residual: -8.0,
                    minimum_log10_reduction: 6.0,
                    maximum_terminal_span_log10: 2.0,
                },
            ],
            monitors: vec![
                MonitorCriterion {
                    field: "CL".to_owned(),
                    unit: "dimensionless".to_owned(),
                    absolute_mean_drift_tolerance: 0.0003,
                    relative_mean_drift_tolerance: 0.0001,
                    absolute_range_tolerance: 0.0004,
                    relative_range_tolerance: 0.0001,
                    maximum_absolute_slope_per_1000_iterations: 0.02,
                },
                MonitorCriterion {
                    field: "CD".to_owned(),
                    unit: "dimensionless".to_owned(),
                    absolute_mean_drift_tolerance: 0.0003,
                    relative_mean_drift_tolerance: 0.0001,
                    absolute_range_tolerance: 0.0004,
                    relative_range_tolerance: 0.0001,
                    maximum_absolute_slope_per_1000_iterations: 0.02,
                },
            ],
        }
    }

    #[test]
    fn multiple_residuals_and_monitors_pass_only_when_all_declared_checks_pass() {
        let report = assess_steady_convergence(&synthetic_import(), &criteria())
            .expect("valid synthetic steady history");
        assert!(report.is_converged());
        assert_eq!(report.residual_audits.len(), 2);
        assert_eq!(report.monitor_audits.len(), 2);
        assert!(report.to_csv().lines().count() == 5);
        assert!(report.to_csv().contains("steady_history_converged_under_declared_criteria"));
    }

    #[test]
    fn a_low_final_residual_does_not_hide_insufficient_total_reduction_or_terminal_instability() {
        let mut import = synthetic_import();
        import.points[2].values[2] = -1.5;
        import.points[3].values[2] = -2.0;
        import.points[4].values[2] = -3.0;
        import.points[5].values[2] = -4.0;
        import.points[6].values[2] = -5.0;
        import.points[7].values[2] = -8.0;
        import.import_sha256 = su2_history_import_sha256(&import).expect("recompute synthetic digest");
        let mut c = criteria();
        c.residuals[0].minimum_log10_reduction = 8.0;
        let report = assess_steady_convergence(&import, &c).expect("well-formed but failing history");
        assert!(!report.is_converged());
        assert!(!report.residual_audits[0].reduction_pass);
    }

    #[test]
    fn oscillatory_monitor_fails_even_with_small_early_late_mean_drift() {
        let mut import = synthetic_import();
        let values = [0.4230, 0.4235, 0.4230, 0.4235, 0.4230, 0.4235];
        for (point, value) in import.points[2..].iter_mut().zip(values) {
            point.values[0] = value;
        }
        import.import_sha256 = su2_history_import_sha256(&import).expect("recompute synthetic digest");
        let report = assess_steady_convergence(&import, &criteria()).expect("well-formed oscillatory history");
        assert!(!report.is_converged());
        assert!(!report.monitor_audits[0].range_pass);
    }

    #[test]
    fn wrong_unit_missing_field_duplicate_field_and_odd_window_reject() {
        let import = synthetic_import();
        let mut c = criteria();
        c.monitors[0].unit = "meters".to_owned();
        assert!(assess_steady_convergence(&import, &c).is_err());
        let mut c = criteria();
        c.monitors[0].field = "MISSING".to_owned();
        assert!(assess_steady_convergence(&import, &c).is_err());
        let mut c = criteria();
        c.monitors.push(c.monitors[0].clone());
        assert!(assess_steady_convergence(&import, &c).is_err());
        let mut c = criteria();
        c.terminal_window_samples = 5;
        assert!(assess_steady_convergence(&import, &c).is_err());
    }

    #[test]
    fn positive_linear_residual_encoding_requires_positive_values() {
        let mut import = synthetic_import();
        import.numeric_fields[2].unit = "linear residual".to_owned();
        for point in &mut import.points {
            point.values[2] = 10_f64.powf(point.values[2]);
        }
        import.import_sha256 = su2_history_import_sha256(&import).expect("recompute digest");
        let mut c = criteria();
        c.residuals[0].unit = "linear residual".to_owned();
        c.residuals[0].encoding = ResidualEncoding::PositiveLinear;
        let report = assess_steady_convergence(&import, &c).expect("positive linear residual series");
        assert!(report.residual_audits[0].passed);

        import.points[0].values[2] = 0.0;
        import.import_sha256 = su2_history_import_sha256(&import).expect("recompute malformed digest");
        assert!(assess_steady_convergence(&import, &c).is_err());
    }
}
