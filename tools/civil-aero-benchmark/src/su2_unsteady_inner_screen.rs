//! Receipt-bound per-time-step inner residual qualification for unsteady SU2 runs.
//!
//! SU2's windowed time convergence can advance when the inner solver reaches its configured
//! INNER_ITER cap, even if the inner convergence flag is false. This independent screen requires
//! every contiguous TIME_ITER record in the imported history to satisfy the declared inner
//! residual threshold. It intentionally supports residual-based CONV_FIELD values only; coefficient
//! based inner-loop Cauchy criteria require a separately typed implementation.

use crate::integrity::sha256_hex;
use crate::su2_field_preflight::Su2FieldPreflightReport;
use crate::su2_history::Su2HistoryReport;

pub const SU2_UNSTEADY_INNER_SCREEN_SCHEMA: &str = "su2-unsteady-inner-step-screen/0.1.0";
pub const SU2_UNSTEADY_INNER_SCREEN_EVIDENCE_LEVEL: &str =
    "receipt_and_history_bound_every_contiguous_saved_time_iter_checked_against_explicit_log10_residual_threshold_residual_fields_only_not_inner_solver_native_iteration_trace_or_temporal_grid_independent_or_experimentally_validated";

#[derive(Debug, Clone, PartialEq)]
pub struct Su2UnsteadyInnerFieldResult {
    pub field: String,
    pub unit: String,
    pub threshold_log10: f64,
    pub maximum_observed_log10: f64,
    pub worst_time_iteration: u64,
    pub failed_time_step_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Su2UnsteadyInnerScreenReport {
    pub schema: String,
    pub run_id: String,
    pub solver_id: String,
    pub runner_receipt_sha256: String,
    pub executable_sha256: String,
    pub history_sha256: String,
    pub history_import_sha256: String,
    pub preflight_sha256: String,
    pub time_iteration_column: String,
    pub inner_iteration_column: String,
    pub first_time_iteration: u64,
    pub last_time_iteration: u64,
    pub contiguous_time_step_count: usize,
    pub fields: Vec<Su2UnsteadyInnerFieldResult>,
    pub status: String,
    pub screen_sha256: String,
    pub evidence_level: String,
}

fn append_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    append_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn append_f64(bytes: &mut Vec<u8>, value: f64) {
    bytes.extend_from_slice(&value.to_bits().to_le_bytes());
}

fn canonical_bytes(report: &Su2UnsteadyInnerScreenReport) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        report.schema.as_str(),
        report.run_id.as_str(),
        report.solver_id.as_str(),
        report.runner_receipt_sha256.as_str(),
        report.executable_sha256.as_str(),
        report.history_sha256.as_str(),
        report.history_import_sha256.as_str(),
        report.preflight_sha256.as_str(),
        report.time_iteration_column.as_str(),
        report.inner_iteration_column.as_str(),
        report.status.as_str(),
        report.evidence_level.as_str(),
    ] {
        append_string(&mut bytes, value);
    }
    append_u64(&mut bytes, report.first_time_iteration);
    append_u64(&mut bytes, report.last_time_iteration);
    append_u64(&mut bytes, report.contiguous_time_step_count as u64);
    append_u64(&mut bytes, report.fields.len() as u64);
    for field in &report.fields {
        append_string(&mut bytes, &field.field);
        append_string(&mut bytes, &field.unit);
        append_f64(&mut bytes, field.threshold_log10);
        append_f64(&mut bytes, field.maximum_observed_log10);
        append_u64(&mut bytes, field.worst_time_iteration);
        append_u64(&mut bytes, field.failed_time_step_count as u64);
    }
    bytes
}

pub fn screen_su2_unsteady_inner_convergence(
    preflight: &Su2FieldPreflightReport,
    history: &Su2HistoryReport,
) -> Result<Su2UnsteadyInnerScreenReport, String> {
    let protocol = &preflight.convergence_protocol;
    if !protocol.time_domain {
        return Err("unsteady inner-step screen requires TIME_DOMAIN=YES".to_owned());
    }
    if protocol.steady_fields.is_empty() {
        return Err("unsteady inner-step screen requires at least one CONV_FIELD".to_owned());
    }
    if history.iteration_column != "TIME_ITER" {
        return Err("unsteady inner-step screen requires TIME_ITER as the history iteration column".to_owned());
    }
    let threshold_text = protocol
        .steady_residual_min_log10
        .as_deref()
        .ok_or_else(|| {
            "unsteady inner-step screen requires an explicit CONV_RESIDUAL_MINVAL".to_owned()
        })?;
    let threshold = threshold_text
        .parse::<f64>()
        .map_err(|error| format!("CONV_RESIDUAL_MINVAL is invalid: {error}"))?;
    if !threshold.is_finite() {
        return Err("CONV_RESIDUAL_MINVAL must be finite".to_owned());
    }

    let fields = history.numeric_fields();
    let inner_index = fields
        .iter()
        .position(|field| field.column == "INNER_ITER")
        .ok_or_else(|| "unsteady history must import INNER_ITER".to_owned())?;
    if fields[inner_index].unit != "iterations" {
        return Err("INNER_ITER must have the declared unit 'iterations'".to_owned());
    }
    let (window_start_iter, convergence_start_iter, cauchy_elements) =
        match &protocol.window {
            crate::su2_field_preflight::Su2WindowConvergenceProtocol::Enabled {
                window_start_iter,
                convergence_start_iter,
                cauchy_elements,
                ..
            } => (*window_start_iter, *convergence_start_iter, *cauchy_elements),
            crate::su2_field_preflight::Su2WindowConvergenceProtocol::Disabled => {
                return Err(
                    "unsteady inner-step screen requires an enabled windowed convergence protocol"
                        .to_owned(),
                );
            }
        };
    let points = history.points();
    if points.len() < 2 {
        return Err("unsteady inner-step screen requires at least two TIME_ITER records".to_owned());
    }
    let first_time_iteration = points.first().expect("non-empty points").iteration;
    let last_time_iteration = points.last().expect("non-empty points").iteration;
    if first_time_iteration > window_start_iter {
        return Err(format!(
            "unsteady history begins at TIME_ITER {first_time_iteration} after WINDOW_START_ITER {window_start_iter}"
        ));
    }
    let required_terminal_iteration = window_start_iter
        .checked_add(convergence_start_iter.max(cauchy_elements))
        .ok_or_else(|| "window convergence iteration range overflowed".to_owned())?;
    if last_time_iteration < required_terminal_iteration {
        return Err(format!(
            "unsteady history ends at TIME_ITER {last_time_iteration} before the configured window convergence period ends at {required_terminal_iteration}"
        ));
    }

    for pair in points.windows(2) {
        let expected = pair[0]
            .iteration
            .checked_add(1)
            .ok_or_else(|| "TIME_ITER overflow while checking continuity".to_owned())?;
        if pair[1].iteration != expected {
            return Err(format!(
                "unsteady history has a TIME_ITER gap: {} followed by {}",
                pair[0].iteration, pair[1].iteration
            ));
        }
    }

    let mut results = Vec::with_capacity(protocol.steady_fields.len());
    for field_name in &protocol.steady_fields {
        let expected_unit = if field_name.starts_with("REL_RMS_") {
            "log10(relative residual)"
        } else if field_name.starts_with("RMS_") {
            "log10(residual)"
        } else {
            return Err(format!(
                "unsteady inner-step screen supports residual CONV_FIELD values only; unsupported field {field_name}"
            ));
        };
        let field_index = fields
            .iter()
            .position(|field| field.column == *field_name)
            .ok_or_else(|| {
                format!("unsteady history does not import configured inner residual {field_name}")
            })?;
        if fields[field_index].unit != expected_unit {
            return Err(format!(
                "inner residual {field_name} must use unit {expected_unit:?}, observed {:?}",
                fields[field_index].unit
            ));
        }

        let mut maximum_observed = f64::NEG_INFINITY;
        let mut worst_time_iteration = points[0].iteration;
        let mut failed_count = 0usize;
        for point in points {
            if point.values.len() != fields.len() {
                return Err("unsteady history point width differs from imported field schema".to_owned());
            }
            let inner_iterations = point.values[inner_index];
            if !inner_iterations.is_finite()
                || inner_iterations < 1.0
                || inner_iterations.fract() != 0.0
            {
                return Err(format!(
                    "INNER_ITER is invalid at TIME_ITER {}",
                    point.iteration
                ));
            }
            let observed = point.values[field_index];
            if !observed.is_finite() {
                return Err(format!(
                    "inner residual {field_name} is non-finite at TIME_ITER {}",
                    point.iteration
                ));
            }
            if observed > maximum_observed {
                maximum_observed = observed;
                worst_time_iteration = point.iteration;
            }
            if observed > threshold {
                failed_count += 1;
            }
        }
        results.push(Su2UnsteadyInnerFieldResult {
            field: field_name.clone(),
            unit: expected_unit.to_owned(),
            threshold_log10: threshold,
            maximum_observed_log10: maximum_observed,
            worst_time_iteration,
            failed_time_step_count: failed_count,
        });
    }

    if results.iter().any(|result| result.failed_time_step_count > 0) {
        let failed = results
            .iter()
            .filter(|result| result.failed_time_step_count > 0)
            .map(|result| {
                format!(
                    "{}:{}",
                    result.field, result.failed_time_step_count
                )
            })
            .collect::<Vec<_>>();
        return Err(format!(
            "unsteady inner residual threshold failed at sampled TIME_ITER records: {}",
            failed.join(",")
        ));
    }

    let mut report = Su2UnsteadyInnerScreenReport {
        schema: SU2_UNSTEADY_INNER_SCREEN_SCHEMA.to_owned(),
        run_id: history.run_id().to_owned(),
        solver_id: history.solver_id().to_owned(),
        runner_receipt_sha256: history.runner_receipt_sha256().to_owned(),
        executable_sha256: history.executable_sha256().to_owned(),
        history_sha256: history.history_sha256().to_owned(),
        history_import_sha256: history.import_sha256().to_owned(),
        preflight_sha256: preflight.preflight_sha256.clone(),
        time_iteration_column: history.iteration_column.clone(),
        inner_iteration_column: "INNER_ITER".to_owned(),
        first_time_iteration,
        last_time_iteration,
        contiguous_time_step_count: points.len(),
        fields: results,
        status: "all_contiguous_saved_time_steps_passed_inner_residual_thresholds".to_owned(),
        screen_sha256: String::new(),
        evidence_level: SU2_UNSTEADY_INNER_SCREEN_EVIDENCE_LEVEL.to_owned(),
    };
    report.screen_sha256 = sha256_hex(&canonical_bytes(&report))?;
    Ok(report)
}

pub fn verify_su2_unsteady_inner_screen(
    preflight: &Su2FieldPreflightReport,
    history: &Su2HistoryReport,
    report: &Su2UnsteadyInnerScreenReport,
) -> Result<(), String> {
    if report.schema != SU2_UNSTEADY_INNER_SCREEN_SCHEMA
        || report.evidence_level != SU2_UNSTEADY_INNER_SCREEN_EVIDENCE_LEVEL
    {
        return Err("unsupported SU2 unsteady inner-screen schema or evidence ceiling".to_owned());
    }
    let recomputed = screen_su2_unsteady_inner_convergence(preflight, history)?;
    if recomputed != *report {
        return Err("replayed SU2 unsteady inner screen differs from supplied report".to_owned());
    }
    if sha256_hex(&canonical_bytes(report))? != report.screen_sha256 {
        return Err("SU2 unsteady inner screen canonical digest mismatch".to_owned());
    }
    Ok(())
}

pub fn report_to_csv(report: &Su2UnsteadyInnerScreenReport) -> String {
    let header = "schema,run_id,solver_id,runner_receipt_sha256,executable_sha256,history_sha256,history_import_sha256,preflight_sha256,time_iteration_column,inner_iteration_column,first_time_iteration,last_time_iteration,contiguous_time_step_count,field,unit,threshold_log10,maximum_observed_log10,worst_time_iteration,failed_time_step_count,status,screen_sha256,evidence_level";
    let mut output = format!("{header}\n");
    for field in &report.fields {
        let values = [
            report.schema.clone(),
            report.run_id.clone(),
            report.solver_id.clone(),
            report.runner_receipt_sha256.clone(),
            report.executable_sha256.clone(),
            report.history_sha256.clone(),
            report.history_import_sha256.clone(),
            report.preflight_sha256.clone(),
            report.time_iteration_column.clone(),
            report.inner_iteration_column.clone(),
            report.first_time_iteration.to_string(),
            report.last_time_iteration.to_string(),
            report.contiguous_time_step_count.to_string(),
            field.field.clone(),
            field.unit.clone(),
            field.threshold_log10.to_string(),
            field.maximum_observed_log10.to_string(),
            field.worst_time_iteration.to_string(),
            field.failed_time_step_count.to_string(),
            report.status.clone(),
            report.screen_sha256.clone(),
            report.evidence_level.clone(),
        ];
        output.push_str(&values.iter().map(|value| format!("\"{}\"", value)).collect::<Vec<_>>().join(","));
        output.push('\n');
    }
    output
}
