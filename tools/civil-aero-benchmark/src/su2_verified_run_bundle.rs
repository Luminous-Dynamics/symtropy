//! Composes one replayable SU2 dry-run audit and production-run lineage into a typed run bundle.
//!
//! The dry-run preflight and boundary audit may share one exact `SU2_CFD -d` receipt. Their
//! executable, configuration and mesh identities must match the production receipt and staged
//! inputs. This proves a coherent evidence chain; it does not authenticate the build/host, prove
//! complete CFD convergence, establish grid independence, or validate against experiment.

use std::collections::BTreeSet;
use std::path::Path;

use crate::external_solver::{read_external_solver_receipt, verify_external_solver_receipt};
use crate::integrity::sha256_hex;
use crate::su2_boundary_audit::{
    Su2BoundaryAuditReport, audit_su2_boundary_markers, verify_su2_boundary_audit,
};
use crate::su2_convergence_screen::{ConvergenceScreen, verify_convergence_screen};
use crate::su2_field_preflight::{
    Su2FieldPreflightReport, audit_su2_field_preflight,
};
use crate::su2_history::{Su2HistoryReport, verify_su2_history_import};
use crate::su2_run_binding::{
    Su2RunInputBindings, Su2RunLineageBinding, verify_su2_run_lineage_binding,
};

pub const SU2_VERIFIED_RUN_BUNDLE_SCHEMA: &str = "su2-verified-run-bundle/0.3.0";
pub const SU2_VERIFIED_RUN_BUNDLE_EVIDENCE_LEVEL: &str =
    "replayed_dry_run_multi_field_protocol_native_solver_termination_and_scheduler_signal_guard_and_every_saved_unsteady_time_step_inner_residual_screen_bound_to_production_history_and_inputs_unsigned_wall_time_only_other_resources_unknown_not_grid_independent_or_experimentally_validated";

/// A resource value is either grounded in a named measurement source or explicitly unavailable.
/// In particular, unknown resource telemetry is never encoded as zero.
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceMeasure {
    Measured {
        value: f64,
        unit: String,
        method: String,
        source_sha256: String,
        /// Standard uncertainty, when actually characterized. None means unknown, not zero.
        standard_uncertainty: Option<f64>,
    },
    Unknown {
        reason: String,
    },
}

impl ResourceMeasure {
    fn validate(&self, name: &str) -> Result<(), String> {
        match self {
            Self::Measured {
                value,
                unit,
                method,
                source_sha256,
                standard_uncertainty,
            } => {
                if !value.is_finite()
                    || *value < 0.0
                    || unit.trim().is_empty()
                    || method.trim().is_empty()
                    || !is_sha256(source_sha256)
                    || standard_uncertainty.is_some_and(|v| !v.is_finite() || v < 0.0)
                {
                    return Err(format!("resource measurement {name} is malformed"));
                }
            }
            Self::Unknown { reason } => {
                if reason.trim().is_empty() {
                    return Err(format!("unknown resource measurement {name} needs a reason"));
                }
            }
        }
        Ok(())
    }
}

/// Resource observations derived for this run. Only wall time is currently in the process
/// receipt. CPU, peak memory, GPU and energy remain unknown until a measurement source is bound.
#[derive(Debug, Clone, PartialEq)]
pub struct Su2ResourceEvidence {
    pub wall_clock_seconds: ResourceMeasure,
    pub cpu_core_hours: ResourceMeasure,
    pub peak_memory_bytes: ResourceMeasure,
    pub gpu_hours: ResourceMeasure,
    pub energy_joules: ResourceMeasure,
}

impl Su2ResourceEvidence {
    fn from_receipt(receipt_sha256: &str, elapsed_milliseconds: u64) -> Self {
        Self {
            wall_clock_seconds: ResourceMeasure::Measured {
                value: elapsed_milliseconds as f64 / 1000.0,
                unit: "s".to_owned(),
                method: "external_solver_receipt.elapsed_milliseconds".to_owned(),
                source_sha256: receipt_sha256.to_owned(),
                standard_uncertainty: None,
            },
            cpu_core_hours: ResourceMeasure::Unknown {
                reason: "external solver receipt does not measure CPU core-hours".to_owned(),
            },
            peak_memory_bytes: ResourceMeasure::Unknown {
                reason: "external solver receipt does not measure peak resident memory".to_owned(),
            },
            gpu_hours: ResourceMeasure::Unknown {
                reason: "external solver receipt does not measure GPU time".to_owned(),
            },
            energy_joules: ResourceMeasure::Unknown {
                reason: "external solver receipt does not measure energy consumption".to_owned(),
            },
        }
    }

    fn validate(&self) -> Result<(), String> {
        self.wall_clock_seconds.validate("wall_clock_seconds")?;
        self.cpu_core_hours.validate("cpu_core_hours")?;
        self.peak_memory_bytes.validate("peak_memory_bytes")?;
        self.gpu_hours.validate("gpu_hours")?;
        self.energy_joules.validate("energy_joules")?;
        Ok(())
    }

    fn append_canonical(&self, bytes: &mut Vec<u8>) {
        for measurement in [
            &self.wall_clock_seconds,
            &self.cpu_core_hours,
            &self.peak_memory_bytes,
            &self.gpu_hours,
            &self.energy_joules,
        ] {
            append_measure(bytes, measurement);
        }
    }

    pub fn unknown_measurement_count(&self) -> usize {
        [
            &self.wall_clock_seconds,
            &self.cpu_core_hours,
            &self.peak_memory_bytes,
            &self.gpu_hours,
            &self.energy_joules,
        ]
        .into_iter()
        .filter(|measurement| matches!(measurement, ResourceMeasure::Unknown { .. }))
        .count()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Su2VerifiedRunBundleReport {
    pub schema: String,
    pub run_id: String,
    pub dry_run_id: String,
    pub solver_id: String,
    /// Caller version label; executable bytes are hashed but build source is not authenticated.
    pub solver_version_claim: String,
    pub executable_sha256: String,
    pub dry_run_receipt_sha256: String,
    pub production_receipt_sha256: String,
    pub physics_configuration_sha256: String,
    pub geometry_sha256: String,
    pub mesh_sha256: String,
    pub field_preflight_sha256: String,
    pub boundary_audit_sha256: String,
    pub history_sha256: String,
    pub history_import_sha256: String,
    pub convergence_screen_sha256: String,
    pub lineage_binding_sha256: String,
    pub termination_classification: String,
    pub termination_sha256: String,
    /// Empty for steady protocols; otherwise the digest of the per-time-step inner residual screen.
    pub unsteady_inner_screen_sha256: String,
    pub resource_evidence: Su2ResourceEvidence,
    pub resource_evidence_sha256: String,
    pub status: String,
    pub evidence_level: String,
    pub bundle_sha256: String,
}

pub struct Su2VerifiedRunBundleInputs<'a> {
    pub dry_run_directory: &'a Path,
    pub production_run_directory: &'a Path,
    pub field_preflight: &'a Su2FieldPreflightReport,
    pub boundary_audit: &'a Su2BoundaryAuditReport,
    pub history: &'a Su2HistoryReport,
    pub convergence_screen: &'a ConvergenceScreen,
    pub input_bindings: &'a Su2RunInputBindings,
    pub additional_output_paths: &'a [String],
    pub lineage_binding: &'a Su2RunLineageBinding,
}

fn append_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    append_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn append_measure(bytes: &mut Vec<u8>, measurement: &ResourceMeasure) {
    match measurement {
        ResourceMeasure::Measured {
            value,
            unit,
            method,
            source_sha256,
            standard_uncertainty,
        } => {
            bytes.push(1);
            bytes.extend_from_slice(&value.to_bits().to_le_bytes());
            append_string(bytes, unit);
            append_string(bytes, method);
            append_string(bytes, source_sha256);
            match standard_uncertainty {
                Some(value) => {
                    bytes.push(1);
                    bytes.extend_from_slice(&value.to_bits().to_le_bytes());
                }
                None => bytes.push(0),
            }
        }
        ResourceMeasure::Unknown { reason } => {
            bytes.push(0);
            append_string(bytes, reason);
        }
    }
}

fn resource_evidence_bytes(evidence: &Su2ResourceEvidence) -> Vec<u8> {
    let mut bytes = Vec::new();
    append_string(&mut bytes, "su2-resource-evidence/0.1.0");
    evidence.append_canonical(&mut bytes);
    bytes
}

fn canonical_bytes(report: &Su2VerifiedRunBundleReport) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        report.schema.as_str(),
        report.run_id.as_str(),
        report.dry_run_id.as_str(),
        report.solver_id.as_str(),
        report.solver_version_claim.as_str(),
        report.executable_sha256.as_str(),
        report.dry_run_receipt_sha256.as_str(),
        report.production_receipt_sha256.as_str(),
        report.physics_configuration_sha256.as_str(),
        report.geometry_sha256.as_str(),
        report.mesh_sha256.as_str(),
        report.field_preflight_sha256.as_str(),
        report.boundary_audit_sha256.as_str(),
        report.history_sha256.as_str(),
        report.history_import_sha256.as_str(),
        report.convergence_screen_sha256.as_str(),
        report.lineage_binding_sha256.as_str(),
        report.termination_classification.as_str(),
        report.termination_sha256.as_str(),
        report.unsteady_inner_screen_sha256.as_str(),
        report.resource_evidence_sha256.as_str(),
        report.status.as_str(),
        report.evidence_level.as_str(),
    ] {
        append_string(&mut bytes, value);
    }
    report.resource_evidence.append_canonical(&mut bytes);
    bytes
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_preflight_covers_history(
    preflight: &Su2FieldPreflightReport,
    history: &Su2HistoryReport,
    screen: &ConvergenceScreen,
) -> Result<(), String> {
    let mut imported = BTreeSet::new();
    imported.insert(history.iteration_column.as_str());
    imported.extend(
        history
            .numeric_fields
            .iter()
            .map(|field| field.column.as_str()),
    );
    let required = preflight
        .required_history_fields
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let available = preflight
        .available_history_fields
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if imported.iter().any(|field| !required.contains(field)) {
        return Err(
            "field preflight does not require every field consumed by the history importer"
                .to_owned(),
        );
    }
    if imported.iter().any(|field| !available.contains(field)) {
        return Err(
            "field preflight does not discover every field consumed by the history importer"
                .to_owned(),
        );
    }

    let mut active_convergence_fields = preflight
        .convergence_protocol
        .steady_fields
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut window_fields = BTreeSet::new();
    if let crate::su2_field_preflight::Su2WindowConvergenceProtocol::Enabled {
        fields,
        ..
    } = &preflight.convergence_protocol.window
    {
        window_fields.extend(fields.iter().map(String::as_str));
        active_convergence_fields.extend(window_fields.iter().copied());
    }
    for field in active_convergence_fields {
        if !imported.contains(field) || !required.contains(field) {
            return Err(format!(
                "configured SU2 convergence field {field} is not required and imported from production history"
            ));
        }
        let criterion = screen.results.iter().find(|row| row.column == field);
        let Some(criterion) = criterion else {
            return Err(format!(
                "configured SU2 convergence field {field} has no independent post-run screen criterion"
            ));
        };
        if criterion.status != crate::su2_convergence_screen::CriterionStatus::Passed {
            return Err(format!(
                "configured SU2 convergence field {field} did not pass its independent post-run screen"
            ));
        }
        if window_fields.contains(field)
            && criterion.kind != "monitor_terminal_range_mean_drift_and_slope"
        {
            return Err(format!(
                "time-window field {field} must have an independent terminal monitor-stability screen"
            ));
        }
    }
    Ok(())
}

/// Recompute both dry-run audits and the production lineage, then compose their matching
/// executable/configuration/mesh identities into one deterministic run-bundle report.
pub fn create_su2_verified_run_bundle(
    inputs: &Su2VerifiedRunBundleInputs<'_>,
) -> Result<Su2VerifiedRunBundleReport, String> {
    let preflight = inputs.field_preflight;
    let boundary = inputs.boundary_audit;
    let history = inputs.history;
    let screen = inputs.convergence_screen;
    let lineage = inputs.lineage_binding;

    let replayed_preflight = audit_su2_field_preflight(
        inputs.dry_run_directory,
        &preflight.configuration_path,
        &preflight.mesh_path,
        &preflight.required_history_fields,
    )?;
    if replayed_preflight != *preflight {
        return Err("replayed SU2 field preflight differs from supplied report".to_owned());
    }
    let replayed_boundary = audit_su2_boundary_markers(
        inputs.dry_run_directory,
        &boundary.configuration_path,
        &boundary.mesh_path,
        &boundary.boundary_option_specs,
        &boundary.non_boundary_marker_options,
    )?;
    if replayed_boundary != *boundary {
        return Err("replayed SU2 boundary audit differs from supplied report".to_owned());
    }
    verify_su2_boundary_audit(boundary)?;
    verify_su2_history_import(history)?;
    verify_convergence_screen(screen)?;
    verify_su2_run_lineage_binding(
        inputs.production_run_directory,
        history,
        screen,
        inputs.input_bindings,
        inputs.additional_output_paths,
        lineage,
    )?;
    validate_preflight_covers_history(preflight, history, screen)?;

    let dry_run_receipt = read_external_solver_receipt(inputs.dry_run_directory)?;
    verify_external_solver_receipt(inputs.dry_run_directory, &dry_run_receipt)?;
    let production_receipt = read_external_solver_receipt(inputs.production_run_directory)?;
    verify_external_solver_receipt(inputs.production_run_directory, &production_receipt)?;
    let termination = crate::su2_termination::classify_su2_termination(
        inputs.production_run_directory,
        preflight,
    )?;
    if !termination.is_converged_for_protocol() {
        return Err(format!(
            "native SU2 termination is not converged for the declared protocol: {}",
            termination.classification
        ));
    }
    let unsteady_inner_screen = if preflight.convergence_protocol.time_domain {
        Some(crate::su2_unsteady_inner_screen::screen_su2_unsteady_inner_convergence(
            preflight,
            history,
        )?)
    } else {
        None
    };

    if dry_run_receipt.solver_id != "SU2_CFD_DRY_RUN"
        || !dry_run_receipt.is_process_success()
        || production_receipt.solver_id != "SU2_CFD"
        || !production_receipt.is_process_success()
    {
        return Err("run bundle requires successful SU2_CFD_DRY_RUN and SU2_CFD receipts".to_owned());
    }
    if preflight.run_id != dry_run_receipt.run_id
        || boundary.run_id != dry_run_receipt.run_id
        || preflight.solver_id != dry_run_receipt.solver_id
        || boundary.solver_id != dry_run_receipt.solver_id
        || preflight.runner_receipt_sha256 != dry_run_receipt.receipt_sha256
        || boundary.runner_receipt_sha256 != dry_run_receipt.receipt_sha256
    {
        return Err("field preflight and boundary audit do not share the exact dry-run receipt".to_owned());
    }
    if preflight.executable_sha256 != dry_run_receipt.executable_sha256
        || boundary.executable_sha256 != dry_run_receipt.executable_sha256
        || dry_run_receipt.executable_sha256 != production_receipt.executable_sha256
    {
        return Err("dry-run and production executable identities differ".to_owned());
    }
    if preflight.configuration_path != inputs.input_bindings.physics_configuration_path
        || boundary.configuration_path != inputs.input_bindings.physics_configuration_path
        || preflight.mesh_path != inputs.input_bindings.mesh_path
        || boundary.mesh_path != inputs.input_bindings.mesh_path
        || preflight.configuration_sha256 != boundary.configuration_sha256
        || preflight.mesh_sha256 != boundary.mesh_sha256
        || preflight.configuration_sha256 != lineage.physics_configuration.sha256
        || boundary.configuration_sha256 != lineage.physics_configuration.sha256
        || preflight.mesh_sha256 != lineage.mesh.sha256
        || boundary.mesh_sha256 != lineage.mesh.sha256
    {
        return Err("dry-run configuration/mesh identities differ from production lineage".to_owned());
    }
    if production_receipt.run_id != lineage.run_id
        || production_receipt.receipt_sha256 != lineage.runner_receipt_sha256
        || production_receipt.executable_sha256 != lineage.executable_sha256
        || production_receipt.solver_version_claim != lineage.solver_version_claim
        || history.run_id() != production_receipt.run_id
        || history.runner_receipt_sha256() != production_receipt.receipt_sha256
        || history.executable_sha256() != production_receipt.executable_sha256
        || screen.runner_receipt_sha256 != production_receipt.receipt_sha256
        || termination.runner_receipt_sha256 != production_receipt.receipt_sha256
        || termination.run_id != production_receipt.run_id
        || termination.executable_sha256 != production_receipt.executable_sha256
    {
        return Err("production receipt, history, screen, termination and lineage identities differ".to_owned());
    }
    let expected_termination_mode = match (
        preflight.convergence_protocol.time_domain,
        &preflight.convergence_protocol.window,
    ) {
        (false, crate::su2_field_preflight::Su2WindowConvergenceProtocol::Disabled) => "steady",
        (true, crate::su2_field_preflight::Su2WindowConvergenceProtocol::Enabled { .. }) => {
            "time_window"
        }
        (true, crate::su2_field_preflight::Su2WindowConvergenceProtocol::Disabled) => {
            "unsteady_no_window"
        }
        (false, crate::su2_field_preflight::Su2WindowConvergenceProtocol::Enabled { .. }) => {
            return Err("time-window convergence is incompatible with TIME_DOMAIN=NO".to_owned());
        }
    };
    if termination.mode != expected_termination_mode {
        return Err("native solver termination mode differs from declared convergence protocol".to_owned());
    }
    if screen.status != "declared_thresholds_met"
        || screen.failed_count != 0
        || screen.insufficient_count != 0
        || screen.passed_count != screen.results.len()
    {
        return Err("all declared convergence screen criteria must pass for a verified run bundle".to_owned());
    }
    if !is_sha256(&lineage.geometry.sha256) {
        return Err("production geometry digest is malformed".to_owned());
    }

    let resource_evidence = Su2ResourceEvidence::from_receipt(
        &production_receipt.receipt_sha256,
        production_receipt.elapsed_milliseconds,
    );
    resource_evidence.validate()?;
    let resource_evidence_sha256 = sha256_hex(&resource_evidence_bytes(&resource_evidence))?;
    let mut report = Su2VerifiedRunBundleReport {
        schema: SU2_VERIFIED_RUN_BUNDLE_SCHEMA.to_owned(),
        run_id: production_receipt.run_id.clone(),
        dry_run_id: dry_run_receipt.run_id.clone(),
        solver_id: production_receipt.solver_id.clone(),
        solver_version_claim: production_receipt.solver_version_claim.clone(),
        executable_sha256: production_receipt.executable_sha256.clone(),
        dry_run_receipt_sha256: dry_run_receipt.receipt_sha256.clone(),
        production_receipt_sha256: production_receipt.receipt_sha256.clone(),
        physics_configuration_sha256: lineage.physics_configuration.sha256.clone(),
        geometry_sha256: lineage.geometry.sha256.clone(),
        mesh_sha256: lineage.mesh.sha256.clone(),
        field_preflight_sha256: preflight.preflight_sha256.clone(),
        boundary_audit_sha256: boundary.audit_sha256.clone(),
        history_sha256: history.history_sha256().to_owned(),
        history_import_sha256: history.import_sha256().to_owned(),
        convergence_screen_sha256: screen.screen_sha256.clone(),
        lineage_binding_sha256: lineage.binding_sha256.clone(),
        termination_classification: termination.classification.clone(),
        termination_sha256: termination.termination_sha256.clone(),
        unsteady_inner_screen_sha256: unsteady_inner_screen
            .as_ref()
            .map(|screen| screen.screen_sha256.clone())
            .unwrap_or_default(),
        resource_evidence,
        resource_evidence_sha256,
        status: "run_identity_preflight_boundary_history_and_declared_screen_replayed".to_owned(),
        evidence_level: SU2_VERIFIED_RUN_BUNDLE_EVIDENCE_LEVEL.to_owned(),
        bundle_sha256: String::new(),
    };
    report.bundle_sha256 = sha256_hex(&canonical_bytes(&report))?;
    Ok(report)
}

/// Independent offline replay entry point. All source run directories are re-read and re-hashed.
pub fn verify_su2_verified_run_bundle(
    inputs: &Su2VerifiedRunBundleInputs<'_>,
    report: &Su2VerifiedRunBundleReport,
) -> Result<(), String> {
    if report.schema != SU2_VERIFIED_RUN_BUNDLE_SCHEMA
        || report.evidence_level != SU2_VERIFIED_RUN_BUNDLE_EVIDENCE_LEVEL
    {
        return Err("unsupported SU2 verified run bundle schema or evidence ceiling".to_owned());
    }
    let recomputed = create_su2_verified_run_bundle(inputs)?;
    if recomputed != *report {
        return Err("replayed SU2 run bundle differs from supplied report".to_owned());
    }
    if sha256_hex(&canonical_bytes(report))? != report.bundle_sha256 {
        return Err("SU2 verified run bundle canonical digest mismatch".to_owned());
    }
    Ok(())
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn measurement_csv(measurement: &ResourceMeasure) -> (String, String, String, String, String) {
    match measurement {
        ResourceMeasure::Measured {
            value,
            unit,
            method,
            source_sha256,
            standard_uncertainty,
        } => (
            "measured".to_owned(),
            value.to_string(),
            unit.clone(),
            method.clone(),
            format!(
                "{}:{}",
                source_sha256,
                standard_uncertainty.map(|v| v.to_string()).unwrap_or_default()
            ),
        ),
        ResourceMeasure::Unknown { reason } => (
            "unknown".to_owned(),
            String::new(),
            String::new(),
            String::new(),
            reason.clone(),
        ),
    }
}

pub fn report_to_csv(report: &Su2VerifiedRunBundleReport) -> String {
    let measures = [
        measurement_csv(&report.resource_evidence.wall_clock_seconds),
        measurement_csv(&report.resource_evidence.cpu_core_hours),
        measurement_csv(&report.resource_evidence.peak_memory_bytes),
        measurement_csv(&report.resource_evidence.gpu_hours),
        measurement_csv(&report.resource_evidence.energy_joules),
    ];
    let mut fields = vec![
        report.schema.clone(),
        report.run_id.clone(),
        report.dry_run_id.clone(),
        report.solver_id.clone(),
        report.solver_version_claim.clone(),
        report.executable_sha256.clone(),
        report.dry_run_receipt_sha256.clone(),
        report.production_receipt_sha256.clone(),
        report.physics_configuration_sha256.clone(),
        report.geometry_sha256.clone(),
        report.mesh_sha256.clone(),
        report.field_preflight_sha256.clone(),
        report.boundary_audit_sha256.clone(),
        report.history_sha256.clone(),
        report.history_import_sha256.clone(),
        report.convergence_screen_sha256.clone(),
        report.lineage_binding_sha256.clone(),
        report.termination_classification.clone(),
        report.termination_sha256.clone(),
        report.unsteady_inner_screen_sha256.clone(),
    ];
    for measure in measures {
        fields.extend([measure.0, measure.1, measure.2, measure.3, measure.4]);
    }
    fields.extend([
        report.resource_evidence_sha256.clone(),
        report.status.clone(),
        report.evidence_level.clone(),
        report.bundle_sha256.clone(),
    ]);
    let header = [
        "schema", "run_id", "dry_run_id", "solver_id", "solver_version_claim",
        "executable_sha256", "dry_run_receipt_sha256", "production_receipt_sha256",
        "physics_configuration_sha256", "geometry_sha256", "mesh_sha256",
        "field_preflight_sha256", "boundary_audit_sha256", "history_sha256",
        "history_import_sha256", "convergence_screen_sha256", "lineage_binding_sha256",
        "termination_classification", "termination_sha256", "unsteady_inner_screen_sha256",
        "wall_time_status", "wall_time_value", "wall_time_unit", "wall_time_method", "wall_time_source_uncertainty",
        "cpu_status", "cpu_value", "cpu_unit", "cpu_method", "cpu_source_uncertainty",
        "peak_memory_status", "peak_memory_value", "peak_memory_unit", "peak_memory_method", "peak_memory_source_uncertainty",
        "gpu_status", "gpu_value", "gpu_unit", "gpu_method", "gpu_source_uncertainty",
        "energy_status", "energy_value", "energy_unit", "energy_method", "energy_source_uncertainty",
        "resource_evidence_sha256", "status", "evidence_level", "bundle_sha256",
    ];
    format!(
        "{}\n{}\n",
        header.join(","),
        fields.iter().map(|value| csv_escape(value)).collect::<Vec<_>>().join(",")
    )
}
