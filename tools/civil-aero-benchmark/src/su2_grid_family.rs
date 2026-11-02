//! Multi-grid study composition from individually replayable SU2 run lineages.
//!
//! Every member's solver receipt, staged config/geometry/mesh, mesh parser summary, imported
//! history, convergence screen, and additional outputs are independently re-verified before the
//! quantity value is derived from the same passing terminal monitor window. The characteristic
//! spacing remains caller-supplied and disclosed. This is a lineage-bound GCI study, not complete
//! CFD admission, experimental validation, or airworthiness evidence.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::grid_convergence::{
    GciReport, GciStudy, GridFamilyKind, GridObservation, evaluate_grid_convergence,
};
use crate::integrity::sha256_hex;
use crate::su2_convergence_screen::{
    ConvergenceScreen, verify_convergence_screen,
};
use crate::su2_history::{Su2HistoryReport, verify_su2_history_import};
use crate::su2_run_binding::{
    Su2RunInputBindings, Su2RunLineageBinding, verify_su2_run_lineage_binding,
};

pub const SU2_GRID_FAMILY_SCHEMA: &str = "su2-lineage-bound-grid-family/0.1.0";
pub const SU2_GRID_FAMILY_EVIDENCE_LEVEL: &str =
    "individually_replayed_su2_run_lineages_and_terminal_monitor_derived_gci_caller_spacing_not_full_cfd_admission_or_experimental_validation";

#[derive(Debug, Clone)]
pub struct Su2GridFamilyMember {
    pub grid_id: String,
    /// Positive, unique, coarse-to-fine level identifier.
    pub grid_level: u32,
    /// Caller-supplied characteristic spacing; its derivation is disclosed by the study.
    pub characteristic_spacing: f64,
    pub run_directory: PathBuf,
    pub history: Su2HistoryReport,
    pub convergence_screen: ConvergenceScreen,
    pub input_bindings: Su2RunInputBindings,
    pub additional_output_paths: Vec<String>,
    pub lineage_binding: Su2RunLineageBinding,
}

#[derive(Debug, Clone)]
pub struct Su2GridFamilyStudy {
    pub study_id: String,
    pub case_id: String,
    /// Digest of the caller-declared task/protocol manifest. The digest is bound, not source-authenticated.
    pub protocol_manifest_sha256: String,
    pub quantity: String,
    pub quantity_field: String,
    pub quantity_unit: String,
    pub source_reference: String,
    pub spacing_definition: String,
    pub grid_family_kind: GridFamilyKind,
    pub safety_factor: f64,
    /// Ordered coarse-to-fine. Each member is independently replayed before its quantity is used.
    pub members: Vec<Su2GridFamilyMember>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Su2GridFamilyMemberAudit {
    pub grid_id: String,
    pub grid_level: u32,
    pub run_id: String,
    pub characteristic_spacing: f64,
    pub quantity_value: f64,
    pub terminal_rows_used: usize,
    pub mesh_sha256: String,
    pub history_sha256: String,
    pub history_import_sha256: String,
    pub convergence_screen_sha256: String,
    pub lineage_binding_sha256: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Su2GridFamilyReport {
    pub schema: String,
    pub study_id: String,
    pub case_id: String,
    pub protocol_manifest_sha256: String,
    pub quantity: String,
    pub quantity_field: String,
    pub quantity_unit: String,
    pub source_reference: String,
    pub spacing_definition: String,
    pub solver_id: String,
    pub solver_version_claim: String,
    pub solver_executable_sha256: String,
    pub physics_configuration_sha256: String,
    pub geometry_sha256: String,
    pub grid_family_kind: GridFamilyKind,
    pub safety_factor: f64,
    pub members: Vec<Su2GridFamilyMemberAudit>,
    pub gci_report: GciReport,
    pub gci_report_sha256: String,
    pub study_sha256: String,
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

fn append_optional_f64(bytes: &mut Vec<u8>, value: Option<f64>) {
    match value {
        Some(number) => {
            bytes.push(1);
            append_f64(bytes, number);
        }
        None => bytes.push(0),
    }
}

fn append_gci(bytes: &mut Vec<u8>, report: &GciReport) {
    for value in [
        report.quantity.as_str(),
        report.source_reference.as_str(),
        report.spacing_definition.as_str(),
        report.grid_family_kind.as_str(),
        report.evidence_level,
    ] {
        append_string(bytes, value);
    }
    append_f64(bytes, report.safety_factor);
    append_u64(bytes, report.triplets.len() as u64);
    for row in &report.triplets {
        for value in [
            row.coarse_grid_id.as_str(),
            row.medium_grid_id.as_str(),
            row.fine_grid_id.as_str(),
        ] {
            append_string(bytes, value);
        }
        for value in [
            row.h_coarse,
            row.h_medium,
            row.h_fine,
            row.phi_coarse,
            row.phi_medium,
            row.phi_fine,
            row.refinement_ratio_coarse_medium,
            row.refinement_ratio_medium_fine,
            row.observed_order,
            row.extrapolated_value,
            row.medium_gci_absolute,
            row.fine_gci_absolute,
            row.asymptotic_ratio,
        ] {
            append_f64(bytes, value);
        }
        append_optional_f64(bytes, row.medium_gci_relative_percent);
        append_optional_f64(bytes, row.fine_gci_relative_percent);
    }
}

fn canonical_bytes(report: &Su2GridFamilyReport) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        report.schema.as_str(),
        report.study_id.as_str(),
        report.case_id.as_str(),
        report.protocol_manifest_sha256.as_str(),
        report.quantity.as_str(),
        report.quantity_field.as_str(),
        report.quantity_unit.as_str(),
        report.source_reference.as_str(),
        report.spacing_definition.as_str(),
        report.solver_id.as_str(),
        report.solver_version_claim.as_str(),
        report.solver_executable_sha256.as_str(),
        report.physics_configuration_sha256.as_str(),
        report.geometry_sha256.as_str(),
        report.grid_family_kind.as_str(),
        report.evidence_level.as_str(),
    ] {
        append_string(&mut bytes, value);
    }
    append_f64(&mut bytes, report.safety_factor);
    append_u64(&mut bytes, report.members.len() as u64);
    for member in &report.members {
        for value in [
            member.grid_id.as_str(),
            member.run_id.as_str(),
            member.mesh_sha256.as_str(),
            member.history_sha256.as_str(),
            member.history_import_sha256.as_str(),
            member.convergence_screen_sha256.as_str(),
            member.lineage_binding_sha256.as_str(),
        ] {
            append_string(&mut bytes, value);
        }
        append_u64(&mut bytes, u64::from(member.grid_level));
        append_f64(&mut bytes, member.characteristic_spacing);
        append_f64(&mut bytes, member.quantity_value);
        append_u64(&mut bytes, member.terminal_rows_used as u64);
    }
    append_string(&mut bytes, &report.gci_report_sha256);
    append_gci(&mut bytes, &report.gci_report);
    bytes
}

fn terminal_monitor_value(
    history: &Su2HistoryReport,
    screen: &ConvergenceScreen,
    field_name: &str,
    unit: &str,
) -> Result<(f64, usize), String> {
    verify_su2_history_import(history)?;
    verify_convergence_screen(screen)?;
    if screen.status != "declared_thresholds_met"
        || screen.failed_count != 0
        || screen.insufficient_count != 0
    {
        return Err(format!(
            "run {} has no passing convergence screen",
            history.run_id()
        ));
    }
    if screen.run_id != history.run_id()
        || screen.solver_id != history.solver_id()
        || screen.runner_receipt_sha256 != history.runner_receipt_sha256()
        || screen.history_sha256 != history.history_sha256()
        || screen.history_import_sha256 != history.import_sha256()
    {
        return Err("history and convergence screen identities/digests differ".to_owned());
    }
    let matching_fields = history
        .numeric_fields()
        .iter()
        .enumerate()
        .filter(|(_, field)| field.column == field_name && field.unit == unit)
        .collect::<Vec<_>>();
    if matching_fields.len() != 1 {
        return Err(format!(
            "quantity field {field_name} with unit {unit} must occur exactly once in imported history"
        ));
    }
    let matching_criteria = screen
        .results
        .iter()
        .filter(|row| {
            row.kind == "monitor_terminal_range_mean_drift_and_slope"
                && row.column == field_name
                && row.unit == unit
                && row.status.as_str() == "passed"
        })
        .collect::<Vec<_>>();
    if matching_criteria.len() != 1 {
        return Err(format!(
            "quantity field {field_name} must have exactly one passing terminal monitor criterion"
        ));
    }
    let (field_index, _) = matching_fields[0];
    let terminal_rows = matching_criteria[0].rows_used;
    if terminal_rows < 2 || terminal_rows > history.points().len() {
        return Err("quantity monitor terminal window is outside imported history".to_owned());
    }
    let tail = &history.points()[history.points().len() - terminal_rows..];
    let divisor = terminal_rows as f64;
    let mean = tail
        .iter()
        .map(|point| point.values[field_index] / divisor)
        .sum::<f64>();
    if !mean.is_finite() {
        return Err("terminal monitor mean is non-finite".to_owned());
    }
    Ok((mean, terminal_rows))
}

fn validate_study_identity(study: &Su2GridFamilyStudy) -> Result<(), String> {
    for (name, value) in [
        ("study_id", study.study_id.as_str()),
        ("case_id", study.case_id.as_str()),
        ("quantity", study.quantity.as_str()),
        ("quantity_field", study.quantity_field.as_str()),
        ("quantity_unit", study.quantity_unit.as_str()),
        ("source_reference", study.source_reference.as_str()),
        ("spacing_definition", study.spacing_definition.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("{name} must not be empty"));
        }
    }
    if study.protocol_manifest_sha256.len() != 64
        || !study
            .protocol_manifest_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("protocol_manifest_sha256 must be a 64-character hexadecimal digest".to_owned());
    }
    if study.members.len() < 3 {
        return Err("at least three lineage-bound grids are required for a GCI study".to_owned());
    }
    if !study.safety_factor.is_finite()
        || study.safety_factor < study.grid_family_kind.minimum_recommended_safety_factor()
    {
        return Err(format!(
            "safety_factor must be finite and >= {:.2} for {} grids",
            study.grid_family_kind.minimum_recommended_safety_factor(),
            study.grid_family_kind.as_str()
        ));
    }
    Ok(())
}

/// Replays every member's evidence chain, derives the scalar from the passed monitor window,
/// and computes GCI only after task/build/config/geometry identity is reconciled.
pub fn evaluate_su2_grid_family_study(
    study: &Su2GridFamilyStudy,
) -> Result<Su2GridFamilyReport, String> {
    validate_study_identity(study)?;
    let mut grid_ids = BTreeSet::new();
    let mut run_ids = BTreeSet::new();
    let mut mesh_hashes = BTreeSet::new();
    let mut member_audits = Vec::with_capacity(study.members.len());
    let mut observations = Vec::with_capacity(study.members.len());
    let mut shared_identity: Option<(String, String, String, String, String)> = None;

    for (index, member) in study.members.iter().enumerate() {
        if member.grid_id.trim().is_empty()
            || member.grid_level == 0
            || !member.characteristic_spacing.is_finite()
            || member.characteristic_spacing <= 0.0
            || !grid_ids.insert(member.grid_id.clone())
        {
            return Err(format!("grid member {index} has an invalid or duplicate grid identity"));
        }
        if !run_ids.insert(member.lineage_binding.run_id.clone()) {
            return Err(format!(
                "grid member {} duplicates run ID {}",
                member.grid_id, member.lineage_binding.run_id
            ));
        }
        if index > 0 {
            let previous = &study.members[index - 1];
            if previous.grid_level >= member.grid_level
                || previous.characteristic_spacing <= member.characteristic_spacing
            {
                return Err(format!(
                    "grid family must be strictly ordered coarse-to-fine at member {}",
                    member.grid_id
                ));
            }
        }

        verify_su2_run_lineage_binding(
            &member.run_directory,
            &member.history,
            &member.convergence_screen,
            &member.input_bindings,
            &member.additional_output_paths,
            &member.lineage_binding,
        )?;
        let binding = &member.lineage_binding;
        if binding.solver_id != "SU2_CFD" {
            return Err(format!(
                "grid member {} is not a successful SU2_CFD production run",
                member.grid_id
            ));
        }
        if !mesh_hashes.insert(binding.mesh.sha256.clone()) {
            return Err(format!(
                "grid member {} reuses mesh bytes; each grid must have a distinct mesh digest",
                member.grid_id
            ));
        }
        let identity = (
            binding.executable_sha256.clone(),
            binding.solver_version_claim.clone(),
            binding.physics_configuration.sha256.clone(),
            binding.geometry.sha256.clone(),
            member.input_bindings.physics_configuration_path.clone(),
        );
        if let Some(reference) = &shared_identity {
            if reference != &identity {
                return Err(format!(
                    "grid member {} changes solver executable/version, physics configuration or geometry identity",
                    member.grid_id
                ));
            }
        } else {
            shared_identity = Some(identity);
        }

        let (quantity_value, terminal_rows) = terminal_monitor_value(
            &member.history,
            &member.convergence_screen,
            &study.quantity_field,
            &study.quantity_unit,
        )?;
        observations.push(GridObservation {
            grid_id: member.grid_id.clone(),
            characteristic_spacing: member.characteristic_spacing,
            value: quantity_value,
        });
        member_audits.push(Su2GridFamilyMemberAudit {
            grid_id: member.grid_id.clone(),
            grid_level: member.grid_level,
            run_id: binding.run_id.clone(),
            characteristic_spacing: member.characteristic_spacing,
            quantity_value,
            terminal_rows_used: terminal_rows,
            mesh_sha256: binding.mesh.sha256.clone(),
            history_sha256: binding.history.sha256.clone(),
            history_import_sha256: binding.history_import_sha256.clone(),
            convergence_screen_sha256: binding.convergence_screen_sha256.clone(),
            lineage_binding_sha256: binding.binding_sha256.clone(),
        });
    }

    let (solver_executable_sha256, solver_version_claim, physics_configuration_sha256, geometry_sha256, _) =
        shared_identity.ok_or_else(|| "grid family has no members".to_owned())?;
    let gci_report = evaluate_grid_convergence(&GciStudy {
        quantity: study.quantity.clone(),
        source_reference: study.source_reference.clone(),
        spacing_definition: study.spacing_definition.clone(),
        grid_family_kind: study.grid_family_kind,
        safety_factor: study.safety_factor,
        observations,
    })?;
    let mut gci_canonical = Vec::new();
    append_gci(&mut gci_canonical, &gci_report);
    let gci_report_sha256 = sha256_hex(&gci_canonical)?;
    let mut report = Su2GridFamilyReport {
        schema: SU2_GRID_FAMILY_SCHEMA.to_owned(),
        study_id: study.study_id.clone(),
        case_id: study.case_id.clone(),
        protocol_manifest_sha256: study.protocol_manifest_sha256.to_ascii_lowercase(),
        quantity: study.quantity.clone(),
        quantity_field: study.quantity_field.clone(),
        quantity_unit: study.quantity_unit.clone(),
        source_reference: study.source_reference.clone(),
        spacing_definition: study.spacing_definition.clone(),
        solver_id: "SU2_CFD".to_owned(),
        solver_version_claim,
        solver_executable_sha256,
        physics_configuration_sha256,
        geometry_sha256,
        grid_family_kind: study.grid_family_kind,
        safety_factor: study.safety_factor,
        members: member_audits,
        gci_report,
        gci_report_sha256,
        study_sha256: String::new(),
        evidence_level: SU2_GRID_FAMILY_EVIDENCE_LEVEL.to_owned(),
    };
    report.study_sha256 = sha256_hex(&canonical_bytes(&report))?;
    Ok(report)
}

pub fn verify_su2_grid_family_report(
    study: &Su2GridFamilyStudy,
    report: &Su2GridFamilyReport,
) -> Result<(), String> {
    if report.schema != SU2_GRID_FAMILY_SCHEMA
        || report.evidence_level != SU2_GRID_FAMILY_EVIDENCE_LEVEL
    {
        return Err("unsupported SU2 grid family schema or evidence ceiling".to_owned());
    }
    let recomputed = evaluate_su2_grid_family_study(study)?;
    if recomputed != *report {
        return Err("replayed SU2 grid-family study differs from supplied report".to_owned());
    }
    if sha256_hex(&canonical_bytes(report))? != report.study_sha256 {
        return Err("SU2 grid-family canonical digest mismatch".to_owned());
    }
    Ok(())
}

pub const SU2_GRID_FAMILY_CSV_HEADER: &str =
    "schema,study_id,case_id,protocol_manifest_sha256,quantity,quantity_field,quantity_unit,source_reference,spacing_definition,solver_id,solver_version_claim,solver_executable_sha256,physics_configuration_sha256,geometry_sha256,grid_family_kind,safety_factor,grid_id,grid_level,run_id,characteristic_spacing,quantity_value,terminal_rows_used,mesh_sha256,history_sha256,history_import_sha256,convergence_screen_sha256,lineage_binding_sha256,gci_triplet_count,gci_report_sha256,study_sha256,evidence_level";

/// Export every sliding GCI triplet; the member CSV intentionally contains only its digest.
pub fn gci_report_to_csv(report: &Su2GridFamilyReport) -> String {
    crate::grid_convergence::report_to_csv(&report.gci_report)
}

pub fn report_to_csv(report: &Su2GridFamilyReport) -> String {
    let mut output = format!("{SU2_GRID_FAMILY_CSV_HEADER}\n");
    for member in &report.members {
        let fields = [
            csv_escape(&report.schema),
            csv_escape(&report.study_id),
            csv_escape(&report.case_id),
            report.protocol_manifest_sha256.clone(),
            csv_escape(&report.quantity),
            csv_escape(&report.quantity_field),
            csv_escape(&report.quantity_unit),
            csv_escape(&report.source_reference),
            csv_escape(&report.spacing_definition),
            csv_escape(&report.solver_id),
            csv_escape(&report.solver_version_claim),
            report.solver_executable_sha256.clone(),
            report.physics_configuration_sha256.clone(),
            report.geometry_sha256.clone(),
            report.grid_family_kind.as_str().to_owned(),
            report.safety_factor.to_string(),
            csv_escape(&member.grid_id),
            member.grid_level.to_string(),
            csv_escape(&member.run_id),
            member.characteristic_spacing.to_string(),
            member.quantity_value.to_string(),
            member.terminal_rows_used.to_string(),
            member.mesh_sha256.clone(),
            member.history_sha256.clone(),
            member.history_import_sha256.clone(),
            member.convergence_screen_sha256.clone(),
            member.lineage_binding_sha256.clone(),
            report.gci_report.triplets.len().to_string(),
            report.gci_report_sha256.clone(),
            report.study_sha256.clone(),
            csv_escape(&report.evidence_level),
        ];
        output.push_str(&fields.join(","));
        output.push('\n');
    }
    output
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::external_solver::{ExternalSolverRequest, StagedInputSpec, execute_external_solver};
    use crate::integrity::sha256_file;
    use crate::su2_convergence_screen::{
        ConvergenceCriteria, MonitorCriterion, ResidualAcceptance, ResidualCriterion,
        screen_su2_history,
    };
    use crate::su2_history::{
        Su2HistoryField, Su2HistorySchema, import_su2_history,
    };
    use crate::su2_run_binding::{
        Su2RunInputBindings, bind_su2_run_lineage,
    };
    use std::collections::BTreeMap;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;

    fn temporary_root() -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "su2-grid-family-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create test root");
        path
    }

    fn stage_spec(
        source_path: &Path,
        relative_path: &str,
        source_reference: &str,
    ) -> StagedInputSpec {
        let bytes = fs::read(source_path).expect("read staged fixture");
        StagedInputSpec {
            source_path: source_path.to_path_buf(),
            relative_path: relative_path.to_owned(),
            expected_byte_length: bytes.len() as u64,
            expected_sha256: sha256_hex(&bytes).expect("hash staged fixture"),
            source_reference: source_reference.to_owned(),
        }
    }

    fn build_fixture_study(root: &Path) -> Su2GridFamilyStudy {
        let shared = root.join("shared");
        fs::create_dir_all(&shared).expect("create shared inputs");
        let config = shared.join("config.cfg");
        let geometry = shared.join("geometry.dat");
        fs::write(
            &config,
            "SOLVER= NAVIER_STOKES\nCONV_FIELD= RMS_DENSITY\nHISTORY_OUTPUT= (ITER, RMS_RES, AERO_COEFF)\nMESH_FILENAME= mesh.su2\n",
        )
        .expect("write physics config");
        fs::write(&geometry, "synthetic fixed geometry identity\n").expect("write geometry");

        let executable = shared.join("fake-su2-cfd");
        let solver = r#"#!/bin/sh
set -eu
printf '"Iteration","CL","CD","RMS_DENSITY","RMS_MOMENTUM-X"\n' > history.csv
printf '0,0.42310,%s,-1,-1\n' "$GRID_CD" >> history.csv
printf '10,0.42310,%s,-2,-2\n' "$GRID_CD" >> history.csv
printf '20,0.42310,%s,-6.0,-6.0\n' "$GRID_CD" >> history.csv
printf '30,0.42310,%s,-6.4,-6.4\n' "$GRID_CD" >> history.csv
printf '40,0.42310,%s,-6.8,-6.8\n' "$GRID_CD" >> history.csv
printf '50,0.42310,%s,-7.2,-7.2\n' "$GRID_CD" >> history.csv
printf '60,0.42310,%s,-7.6,-7.6\n' "$GRID_CD" >> history.csv
printf '70,0.42310,%s,-8.0,-8.0\n' "$GRID_CD" >> history.csv
printf 'synthetic force output; not CFD evidence\n' > forces.dat
printf 'synthetic solver stdout\n'
"#;
        fs::write(&executable, solver).expect("write fake solver");
        let mut permissions = fs::metadata(&executable).expect("solver metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).expect("make solver executable");

        let history_schema = Su2HistorySchema {
            history_relative_path: "history.csv".to_owned(),
            expected_header: vec![
                "Iteration".to_owned(),
                "CL".to_owned(),
                "CD".to_owned(),
                "RMS_DENSITY".to_owned(),
                "RMS_MOMENTUM-X".to_owned(),
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
                Su2HistoryField {
                    column: "RMS_DENSITY".to_owned(),
                    unit: "log10(residual)".to_owned(),
                },
                Su2HistoryField {
                    column: "RMS_MOMENTUM-X".to_owned(),
                    unit: "log10(residual)".to_owned(),
                },
            ],
            minimum_rows: 8,
        };
        let criteria = ConvergenceCriteria {
            residuals: ["RMS_DENSITY", "RMS_MOMENTUM-X"]
                .into_iter()
                .map(|column| ResidualCriterion {
                    column: column.to_owned(),
                    unit: "log10(residual)".to_owned(),
                    terminal_window_rows: 6,
                    acceptance: ResidualAcceptance::Log10 {
                        maximum_final_log10_residual: -8.0,
                        minimum_log10_reduction: 6.0,
                        maximum_terminal_span_log10: 2.0,
                    },
                })
                .collect(),
            monitors: ["CL", "CD"]
                .into_iter()
                .map(|column| MonitorCriterion {
                    column: column.to_owned(),
                    unit: "dimensionless".to_owned(),
                    trailing_window_rows: 6,
                    maximum_absolute_range: 0.0004,
                    maximum_absolute_mean_drift: 0.0003,
                    maximum_absolute_slope_per_1000_iterations: 0.02,
                })
                .collect(),
        };
        let input_bindings = Su2RunInputBindings {
            physics_configuration_path: "config.cfg".to_owned(),
            geometry_path: "geometry.dat".to_owned(),
            mesh_path: "mesh.su2".to_owned(),
        };

        let spacings = [4.0_f64, 2.0, 1.0];
        let mut members = Vec::new();
        for (index, spacing) in spacings.into_iter().enumerate() {
            let level = u32::try_from(index + 1).expect("three levels fit u32");
            let mesh = shared.join(format!("mesh-{level}.su2"));
            let extent = 1.0 / f64::from(level);
            let mesh_text = format!(
                "NDIME= 2\nNELEM= 2\n5 0 1 2\n5 0 2 3\nNPOIN= 4\n0.0 0.0\n{extent} 0.0\n{extent} {extent}\n0.0 {extent}\nNMARK= 1\nMARKER_TAG= wall\nMARKER_ELEMS= 4\n3 0 1\n3 1 2\n3 2 3\n3 3 0\n"
            );
            fs::write(&mesh, mesh_text).expect("write grid mesh");
            let run_directory = root.join(format!("run-{level}"));
            let drag = 0.02 + 0.008 * spacing.powi(2);
            let request = ExternalSolverRequest {
                run_id: format!("su2-grid-run-{level}"),
                solver_id: "SU2_CFD".to_owned(),
                solver_version_claim: "synthetic-ci-fixture-not-a-SU2-version".to_owned(),
                executable_path: executable.clone(),
                run_directory: run_directory.clone(),
                arguments: vec!["-f".to_owned(), "config.cfg".to_owned()],
                environment: BTreeMap::from([("GRID_CD".to_owned(), format!("{drag:.12}"))]),
                staged_inputs: vec![
                    stage_spec(&config, "config.cfg", "synthetic-fixed-physics-config"),
                    stage_spec(&geometry, "geometry.dat", "synthetic-fixed-geometry"),
                    stage_spec(&mesh, "mesh.su2", "synthetic-grid-mesh"),
                ],
                expected_outputs: vec!["history.csv".to_owned(), "forces.dat".to_owned()],
                timeout: Duration::from_secs(2),
                max_log_bytes_per_stream: 16 * 1024,
            };
            let receipt = execute_external_solver(&request).expect("execute fake solver");
            assert!(receipt.is_process_success());
            assert_eq!(
                receipt.executable_sha256,
                sha256_file(&executable).expect("hash fixture executable")
            );

            let history =
                import_su2_history(&run_directory, &history_schema).expect("import SU2 history");
            let screen =
                screen_su2_history(&history, &criteria).expect("screen history");
            assert_eq!(screen.status, "declared_thresholds_met");
            let additional_output_paths = vec!["forces.dat".to_owned()];
            let binding = bind_su2_run_lineage(
                &run_directory,
                &history,
                &screen,
                &input_bindings,
                &additional_output_paths,
            )
            .expect("bind complete run lineage");
            members.push(Su2GridFamilyMember {
                grid_id: format!("grid-{level}"),
                grid_level: level,
                characteristic_spacing: spacing,
                run_directory,
                history,
                convergence_screen: screen,
                input_bindings: input_bindings.clone(),
                additional_output_paths,
                lineage_binding: binding,
            });
        }

        Su2GridFamilyStudy {
            study_id: "synthetic-3-grid-study".to_owned(),
            case_id: "synthetic-OAT15A-M0.73-Re3M-alpha1.50-periodic".to_owned(),
            protocol_manifest_sha256: sha256_hex(b"synthetic protocol fixture").expect("protocol digest"),
            quantity: "section drag coefficient".to_owned(),
            quantity_field: "CD".to_owned(),
            quantity_unit: "dimensionless".to_owned(),
            source_reference: "synthetic fake-solver known answer".to_owned(),
            spacing_definition: "explicit synthetic characteristic spacing".to_owned(),
            grid_family_kind: GridFamilyKind::UnstructuredOrHybrid,
            safety_factor: 3.0,
            members,
        }
    }

    #[test]
    fn lineage_bound_three_grid_study_derives_quantity_and_replays_gci() {
        let root = temporary_root();
        let study = build_fixture_study(&root);
        let report = evaluate_su2_grid_family_study(&study).expect("lineage-bound GCI study");
        assert_eq!(report.members.len(), 3);
        assert_eq!(report.gci_report.triplets.len(), 1);
        assert!((report.members[0].quantity_value - 0.148).abs() < 1.0e-11);
        assert!((report.members[1].quantity_value - 0.052).abs() < 1.0e-11);
        assert!((report.members[2].quantity_value - 0.028).abs() < 1.0e-11);
        assert!((report.gci_report.triplets[0].observed_order - 2.0).abs() < 1.0e-10);
        assert!((report.gci_report.triplets[0].extrapolated_value - 0.02).abs() < 1.0e-10);
        assert_eq!(report_to_csv(&report).lines().count(), 4);
        assert_eq!(gci_report_to_csv(&report).lines().count(), 2);
        verify_su2_grid_family_report(&study, &report).expect("independent lineage replay");

        let mut reordered = study.clone();
        reordered.members.swap(0, 1);
        assert!(
            evaluate_su2_grid_family_study(&reordered)
                .expect_err("non-refining run order rejected")
                .contains("coarse-to-fine")
        );
        fs::remove_dir_all(root).expect("remove synthetic study artifacts");
    }

    #[test]
    fn rejects_incomplete_or_inconsistent_family_identity() {
        let study = Su2GridFamilyStudy {
            study_id: "study".to_owned(),
            case_id: "case".to_owned(),
            protocol_manifest_sha256: "0".repeat(64),
            quantity: "drag".to_owned(),
            quantity_field: "CD".to_owned(),
            quantity_unit: "dimensionless".to_owned(),
            source_reference: "synthetic fixture".to_owned(),
            spacing_definition: "synthetic volume-based characteristic spacing".to_owned(),
            grid_family_kind: GridFamilyKind::UnstructuredOrHybrid,
            safety_factor: 3.0,
            members: Vec::new(),
        };
        assert!(
            evaluate_su2_grid_family_study(&study)
                .expect_err("empty family rejected")
                .contains("at least three")
        );
    }
}

        let study = Su2GridFamilyStudy {
            study_id: "study".to_owned(),
            case_id: "case".to_owned(),
            protocol_manifest_sha256: "0".repeat(64),
            quantity: "drag".to_owned(),
            quantity_field: "CD".to_owned(),
            quantity_unit: "dimensionless".to_owned(),
            source_reference: "synthetic fixture".to_owned(),
            spacing_definition: "synthetic volume-based characteristic spacing".to_owned(),
            grid_family_kind: GridFamilyKind::UnstructuredOrHybrid,
            safety_factor: 3.0,
            members: Vec::new(),
        };
        assert!(
            evaluate_su2_grid_family_study(&study)
                .expect_err("empty family rejected")
                .contains("at least three")
        );
    }
}
