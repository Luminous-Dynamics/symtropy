//! Multi-grid study composition from individually replayable SU2 run lineages.
//!
//! Every member's solver receipt, staged config/geometry/mesh, mesh parser summary, imported
//! history, convergence screen, and additional outputs are independently re-verified before the
//! quantity value is derived from the same passing terminal monitor window. Characteristic
//! spacing is derived from exact 2D linear triangle/quadrilateral cell areas. This is not full CFD
//! admission, experimental validation, or airworthiness evidence.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use crate::grid_convergence::{
    GciReport, GciStudy, GridFamilyKind, GridObservation, evaluate_grid_convergence,
};
use crate::integrity::sha256_hex;
use crate::su2_boundary_audit::Su2BoundaryAuditReport;
use crate::su2_convergence_screen::{ConvergenceScreen, verify_convergence_screen};
use crate::su2_field_preflight::Su2FieldPreflightReport;
use crate::su2_history::{Su2HistoryReport, verify_su2_history_import};
use crate::su2_mesh::{Su2MeshSpacing2d, derive_su2_characteristic_spacing_2d, inspect_su2_mesh};
use crate::su2_run_binding::{Su2RunInputBindings, Su2RunLineageBinding};
use crate::su2_verified_run_bundle::{
    Su2VerifiedRunBundleInputs, create_su2_verified_run_bundle,
};

pub const SU2_GRID_FAMILY_SCHEMA: &str = "su2-lineage-bound-grid-family/0.2.0";
pub const SU2_GRID_SPACING_DEFINITION_2D: &str =
    "h=sqrt(total_linear_2d_cell_area/number_of_2d_volume_cells)";
pub const SU2_GRID_DOMAIN_AREA_RELATIVE_TOLERANCE: f64 = 1.0e-10;
pub const SU2_GRID_FAMILY_EVIDENCE_LEVEL: &str = "individually_replayed_preflight_boundary_production_run_bundles_and_terminal_monitor_derived_gci_with_mesh_derived_2d_area_spacing_not_full_cfd_admission_or_experimental_validation";

#[derive(Debug, Clone)]
pub struct Su2GridFamilyMember {
    pub grid_id: String,
    /// Positive, unique, coarse-to-fine level identifier.
    pub grid_level: u32,
    pub dry_run_directory: PathBuf,
    pub field_preflight: Su2FieldPreflightReport,
    pub boundary_audit: Su2BoundaryAuditReport,
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
    pub mesh_coordinate_unit: String,
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
    pub mesh_cell_count: u64,
    pub mesh_total_cell_area: f64,
    pub mesh_mean_cell_area: f64,
    pub mesh_minimum_cell_area: f64,
    pub mesh_maximum_cell_area: f64,
    pub mesh_cell_area_coefficient_of_variation: f64,
    pub quantity_value: f64,
    pub terminal_rows_used: usize,
    pub mesh_sha256: String,
    pub history_sha256: String,
    pub history_import_sha256: String,
    pub convergence_screen_sha256: String,
    pub lineage_binding_sha256: String,
    pub field_preflight_sha256: String,
    pub boundary_audit_sha256: String,
    pub verified_run_bundle_sha256: String,
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
    pub mesh_coordinate_unit: String,
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
        report.mesh_coordinate_unit.as_str(),
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
            member.field_preflight_sha256.as_str(),
            member.boundary_audit_sha256.as_str(),
            member.verified_run_bundle_sha256.as_str(),
        ] {
            append_string(&mut bytes, value);
        }
        append_u64(&mut bytes, u64::from(member.grid_level));
        append_f64(&mut bytes, member.characteristic_spacing);
        append_u64(&mut bytes, member.mesh_cell_count);
        append_f64(&mut bytes, member.mesh_total_cell_area);
        append_f64(&mut bytes, member.mesh_mean_cell_area);
        append_f64(&mut bytes, member.mesh_minimum_cell_area);
        append_f64(&mut bytes, member.mesh_maximum_cell_area);
        append_f64(&mut bytes, member.mesh_cell_area_coefficient_of_variation);
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
        ("mesh_coordinate_unit", study.mesh_coordinate_unit.as_str()),
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
        return Err(
            "protocol_manifest_sha256 must be a 64-character hexadecimal digest".to_owned(),
        );
    }
    if study.spacing_definition != SU2_GRID_SPACING_DEFINITION_2D {
        return Err(format!(
            "spacing_definition must be the mesh-derived 2D definition {SU2_GRID_SPACING_DEFINITION_2D}"
        ));
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

fn validate_mesh_family_invariants(
    reference_area: Option<f64>,
    reference_markers: Option<&[String]>,
    current_area: f64,
    current_markers: &[String],
    grid_id: &str,
) -> Result<(), String> {
    if !current_area.is_finite() || current_area <= 0.0 {
        return Err(format!("grid member {grid_id} has invalid total mesh area"));
    }
    let current_set = current_markers
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if current_set.len() != current_markers.len() {
        return Err(format!(
            "grid member {grid_id} has duplicate mesh marker tags"
        ));
    }
    if let Some(reference) = reference_markers {
        let reference_set = reference
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if reference_set != current_set {
            return Err(format!(
                "grid member {grid_id} changes the mesh marker-tag set"
            ));
        }
    }
    if let Some(reference_area) = reference_area {
        if !reference_area.is_finite() || reference_area <= 0.0 {
            return Err("reference grid has invalid total mesh area".to_owned());
        }
        let relative_difference = (current_area - reference_area).abs() / reference_area;
        if !relative_difference.is_finite()
            || relative_difference > SU2_GRID_DOMAIN_AREA_RELATIVE_TOLERANCE
        {
            return Err(format!(
                "grid member {grid_id} changes total 2D mesh domain area by {:.6e} relative; limit is {:.6e}",
                relative_difference, SU2_GRID_DOMAIN_AREA_RELATIVE_TOLERANCE
            ));
        }
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
    let mut spacing_metrics = Vec::<Su2MeshSpacing2d>::with_capacity(study.members.len());
    let mut shared_marker_tags: Option<Vec<String>> = None;
    let mut shared_domain_area: Option<f64> = None;
    let mut shared_identity: Option<(String, String, String, String, String)> = None;

    for (index, member) in study.members.iter().enumerate() {
        if member.grid_id.trim().is_empty()
            || member.grid_level == 0
            || !grid_ids.insert(member.grid_id.clone())
        {
            return Err(format!(
                "grid member {index} has an invalid or duplicate grid identity"
            ));
        }
        if !run_ids.insert(member.lineage_binding.run_id.clone()) {
            return Err(format!(
                "grid member {} duplicates run ID {}",
                member.grid_id, member.lineage_binding.run_id
            ));
        }
        if index > 0 {
            let previous = &study.members[index - 1];
            if previous.grid_level >= member.grid_level {
                return Err(format!(
                    "grid family levels must be strictly increasing coarse-to-fine at member {}",
                    member.grid_id
                ));
            }
        }

        let verified_bundle = create_su2_verified_run_bundle(&Su2VerifiedRunBundleInputs {
            dry_run_directory: &member.dry_run_directory,
            production_run_directory: &member.run_directory,
            field_preflight: &member.field_preflight,
            boundary_audit: &member.boundary_audit,
            history: &member.history,
            convergence_screen: &member.convergence_screen,
            input_bindings: &member.input_bindings,
            additional_output_paths: &member.additional_output_paths,
            lineage_binding: &member.lineage_binding,
        })?;
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
        let mesh_bytes = fs::read(member.run_directory.join(&member.input_bindings.mesh_path))
            .map_err(|error| format!("cannot read staged mesh for {}: {error}", member.grid_id))?;
        let mesh_spacing =
            derive_su2_characteristic_spacing_2d(&mesh_bytes, &study.mesh_coordinate_unit)?;
        if mesh_spacing.mesh_byte_length != binding.mesh.byte_length
            || mesh_spacing.mesh_sha256 != binding.mesh.sha256
        {
            return Err(format!(
                "grid member {} mesh spacing input differs from verified lineage artifact",
                member.grid_id
            ));
        }
        let mesh_summary = inspect_su2_mesh(&mesh_bytes)?;
        let mut marker_tags = mesh_summary
            .markers
            .iter()
            .map(|marker| marker.tag.clone())
            .collect::<Vec<_>>();
        marker_tags.sort();
        validate_mesh_family_invariants(
            shared_domain_area,
            shared_marker_tags.as_deref(),
            mesh_spacing.total_cell_area,
            &marker_tags,
            &member.grid_id,
        )?;
        shared_marker_tags.get_or_insert(marker_tags);
        shared_domain_area.get_or_insert(mesh_spacing.total_cell_area);
        if let Some(previous) = spacing_metrics.last()
            && previous.characteristic_spacing <= mesh_spacing.characteristic_spacing
        {
            return Err(format!(
                "mesh-derived spacing must refine coarse-to-fine at member {}",
                member.grid_id
            ));
        }
        spacing_metrics.push(mesh_spacing);
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
        let mesh_spacing = spacing_metrics
            .last()
            .expect("mesh spacing was just derived for this member");
        observations.push(GridObservation {
            grid_id: member.grid_id.clone(),
            characteristic_spacing: mesh_spacing.characteristic_spacing,
            value: quantity_value,
        });
        member_audits.push(Su2GridFamilyMemberAudit {
            grid_id: member.grid_id.clone(),
            grid_level: member.grid_level,
            run_id: binding.run_id.clone(),
            characteristic_spacing: mesh_spacing.characteristic_spacing,
            mesh_cell_count: mesh_spacing.cell_count,
            mesh_total_cell_area: mesh_spacing.total_cell_area,
            mesh_mean_cell_area: mesh_spacing.mean_cell_area,
            mesh_minimum_cell_area: mesh_spacing.minimum_cell_area,
            mesh_maximum_cell_area: mesh_spacing.maximum_cell_area,
            mesh_cell_area_coefficient_of_variation: mesh_spacing
                .cell_area_coefficient_of_variation,
            quantity_value,
            terminal_rows_used: terminal_rows,
            mesh_sha256: binding.mesh.sha256.clone(),
            history_sha256: binding.history.sha256.clone(),
            history_import_sha256: binding.history_import_sha256.clone(),
            convergence_screen_sha256: binding.convergence_screen_sha256.clone(),
            lineage_binding_sha256: binding.binding_sha256.clone(),
            field_preflight_sha256: member.field_preflight.preflight_sha256.clone(),
            boundary_audit_sha256: member.boundary_audit.audit_sha256.clone(),
            verified_run_bundle_sha256: verified_bundle.bundle_sha256,
        });
    }

    let (
        solver_executable_sha256,
        solver_version_claim,
        physics_configuration_sha256,
        geometry_sha256,
        _,
    ) = shared_identity.ok_or_else(|| "grid family has no members".to_owned())?;
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
        mesh_coordinate_unit: study.mesh_coordinate_unit.clone(),
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

pub const SU2_GRID_FAMILY_CSV_HEADER: &str = "schema,study_id,case_id,protocol_manifest_sha256,quantity,quantity_field,quantity_unit,source_reference,spacing_definition,mesh_coordinate_unit,solver_id,solver_version_claim,solver_executable_sha256,physics_configuration_sha256,geometry_sha256,grid_family_kind,safety_factor,grid_id,grid_level,run_id,characteristic_spacing,mesh_cell_count,mesh_total_cell_area,mesh_mean_cell_area,mesh_minimum_cell_area,mesh_maximum_cell_area,mesh_cell_area_coefficient_of_variation,quantity_value,terminal_rows_used,mesh_sha256,history_sha256,history_import_sha256,convergence_screen_sha256,lineage_binding_sha256,field_preflight_sha256,boundary_audit_sha256,verified_run_bundle_sha256,gci_triplet_count,gci_report_sha256,study_sha256,evidence_level";

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
            csv_escape(&report.mesh_coordinate_unit),
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
            member.mesh_cell_count.to_string(),
            member.mesh_total_cell_area.to_string(),
            member.mesh_mean_cell_area.to_string(),
            member.mesh_minimum_cell_area.to_string(),
            member.mesh_maximum_cell_area.to_string(),
            member.mesh_cell_area_coefficient_of_variation.to_string(),
            member.quantity_value.to_string(),
            member.terminal_rows_used.to_string(),
            member.mesh_sha256.clone(),
            member.history_sha256.clone(),
            member.history_import_sha256.clone(),
            member.convergence_screen_sha256.clone(),
            member.lineage_binding_sha256.clone(),
            member.field_preflight_sha256.clone(),
            member.boundary_audit_sha256.clone(),
            member.verified_run_bundle_sha256.clone(),
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
    use crate::su2_boundary_audit::{Su2BoundaryOptionSpec, audit_su2_boundary_markers};
    use crate::su2_field_preflight::audit_su2_field_preflight;
    use crate::su2_convergence_screen::{
        ConvergenceCriteria, MonitorCriterion, ResidualAcceptance, ResidualCriterion,
        screen_su2_history,
    };
    use crate::su2_history::{Su2HistoryField, Su2HistorySchema, import_su2_history};
    use crate::su2_run_binding::bind_su2_run_lineage;
    use std::collections::BTreeMap;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use std::time::Duration;

    fn temporary_root() -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("su2-grid-family-{}-{nonce}", std::process::id()));
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

    fn unit_square_triangular_mesh(divisions: usize) -> String {
        let mut mesh = format!("NDIME= 2\nNELEM= {}\n", 2 * divisions * divisions);
        let node = |i: usize, j: usize| j * (divisions + 1) + i;
        for j in 0..divisions {
            for i in 0..divisions {
                let lower_left = node(i, j);
                let lower_right = node(i + 1, j);
                let upper_right = node(i + 1, j + 1);
                let upper_left = node(i, j + 1);
                mesh.push_str(&format!("5 {lower_left} {lower_right} {upper_right}\n"));
                mesh.push_str(&format!("5 {lower_left} {upper_right} {upper_left}\n"));
            }
        }
        mesh.push_str(&format!("NPOIN= {}\n", (divisions + 1) * (divisions + 1)));
        for j in 0..=divisions {
            for i in 0..=divisions {
                let x = i as f64 / divisions as f64;
                let y = j as f64 / divisions as f64;
                mesh.push_str(&format!("{x:.8} {y:.8}\n"));
            }
        }
        mesh.push_str("NMARK= 1\nMARKER_TAG= wall\n");
        mesh.push_str(&format!("MARKER_ELEMS= {}\n", 4 * divisions));
        for i in 0..divisions {
            mesh.push_str(&format!("3 {} {}\n", node(i, 0), node(i + 1, 0)));
        }
        for j in 0..divisions {
            mesh.push_str(&format!(
                "3 {} {}\n",
                node(divisions, j),
                node(divisions, j + 1)
            ));
        }
        for i in (0..divisions).rev() {
            mesh.push_str(&format!(
                "3 {} {}\n",
                node(i + 1, divisions),
                node(i, divisions)
            ));
        }
        for j in (0..divisions).rev() {
            mesh.push_str(&format!("3 {} {}\n", node(0, j + 1), node(0, j)));
        }
        mesh
    }

    fn build_fixture_study(root: &Path) -> Su2GridFamilyStudy {
        let shared = root.join("shared");
        fs::create_dir_all(&shared).expect("create shared inputs");
        let config = shared.join("config.cfg");
        let geometry = shared.join("geometry.dat");
        fs::write(
            &config,
            r#"SOLVER= NAVIER_STOKES
CONV_FIELD= RMS_DENSITY
CONV_RESIDUAL_MINVAL= -8
HISTORY_OUTPUT= (TIME_ITER, INNER_ITER, RMS_RES, AERO_COEFF, TAVG_AERO_COEFF)
TIME_DOMAIN= YES
WINDOW_CAUCHY_CRIT= YES
CONV_WINDOW_FIELD= (TAVG_DRAG, TAVG_LIFT)
WINDOW_START_ITER= 10
CONV_WINDOW_STARTITER= 10
CONV_WINDOW_CAUCHY_ELEMS= 6
CONV_WINDOW_CAUCHY_EPS= 1E-3
WINDOW_FUNCTION= SQUARE
MESH_FILENAME= mesh.su2
MARKER_HEATFLUX= (wall, 0.0)
MARKER_MONITORING= (wall)
"#,
        )
        .expect("write physics config");
        fs::write(&geometry, "synthetic fixed geometry identity\n").expect("write geometry");

        let executable = shared.join("fake-su2-cfd");
        let solver = r#"#!/bin/sh
set -eu
if [ "${1:-}" = "-d" ]; then
  printf '%s\\n' 'Available screen/history output fields for the current configuration:' 'TIME_ITER' 'INNER_ITER' 'CL' 'CD' 'TAVG_DRAG' 'TAVG_LIFT' 'RMS_DENSITY' 'RMS_MOMENTUM-X'
  exit 0
fi
printf '"TIME_ITER","INNER_ITER","CL","CD","TAVG_DRAG","TAVG_LIFT","RMS_DENSITY","RMS_MOMENTUM-X"\\n' > history.csv
printf '10,5,0.42310,%s,%s,0.42310,-8.2,-1\\n' "$GRID_CD" "$GRID_CD" >> history.csv
printf '11,5,0.42310,%s,%s,0.42310,-8.3,-2\\n' "$GRID_CD" "$GRID_CD" >> history.csv
printf '12,5,0.42310,%s,%s,0.42310,-8.4,-6.0\\n' "$GRID_CD" "$GRID_CD" >> history.csv
printf '13,5,0.42310,%s,%s,0.42310,-8.5,-6.4\\n' "$GRID_CD" "$GRID_CD" >> history.csv
printf '14,5,0.42310,%s,%s,0.42310,-8.6,-6.8\\n' "$GRID_CD" "$GRID_CD" >> history.csv
printf '15,5,0.42310,%s,%s,0.42310,-8.7,-7.2\\n' "$GRID_CD" "$GRID_CD" >> history.csv
printf '16,5,0.42310,%s,%s,0.42310,-8.8,-7.6\\n' "$GRID_CD" "$GRID_CD" >> history.csv
printf '17,5,0.42310,%s,%s,0.42310,-8.9,-8.0\\n' "$GRID_CD" "$GRID_CD" >> history.csv
printf 'synthetic force output; not CFD evidence\\n' > forces.dat
printf '%s\\n' '----------------------------- Solver Exit -------------------------------' 'All windowed time-averaged convergence criteria are fullfilled.' '-------------------------------------------------------------------------'
"#;
        fs::write(&executable, solver).expect("write fake solver");
        let mut permissions = fs::metadata(&executable)
            .expect("solver metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).expect("make solver executable");

        let history_schema = Su2HistorySchema {
            history_relative_path: "history.csv".to_owned(),
            expected_header: vec![
                "TIME_ITER".to_owned(),
                "INNER_ITER".to_owned(),
                "CL".to_owned(),
                "CD".to_owned(),
                "TAVG_DRAG".to_owned(),
                "TAVG_LIFT".to_owned(),
                "RMS_DENSITY".to_owned(),
                "RMS_MOMENTUM-X".to_owned(),
            ],
            iteration_column: "TIME_ITER".to_owned(),
            numeric_fields: vec![
                Su2HistoryField {
                    column: "INNER_ITER".to_owned(),
                    unit: "iterations".to_owned(),
                },
                Su2HistoryField {
                    column: "CL".to_owned(),
                    unit: "dimensionless".to_owned(),
                },
                Su2HistoryField {
                    column: "CD".to_owned(),
                    unit: "dimensionless".to_owned(),
                },
                Su2HistoryField {
                    column: "TAVG_DRAG".to_owned(),
                    unit: "dimensionless".to_owned(),
                },
                Su2HistoryField {
                    column: "TAVG_LIFT".to_owned(),
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
                .map(|column| {
                    let (maximum_final_log10_residual, minimum_log10_reduction, maximum_terminal_span_log10) =
                        if column == "RMS_DENSITY" {
                            (-8.0, 0.5, 1.0)
                        } else {
                            (-8.0, 6.0, 2.0)
                        };
                    ResidualCriterion {
                        column: column.to_owned(),
                        unit: "log10(residual)".to_owned(),
                        terminal_window_rows: 6,
                        acceptance: ResidualAcceptance::Log10 {
                            maximum_final_log10_residual,
                            minimum_log10_reduction,
                            maximum_terminal_span_log10,
                        },
                    }
                })
                .collect(),
            monitors: ["CL", "CD", "TAVG_DRAG", "TAVG_LIFT"]
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

        let mut members = Vec::new();
        for index in 0..3 {
            let level = u32::try_from(index + 1).expect("three levels fit u32");
            let mesh = shared.join(format!("mesh-{level}.su2"));
            let divisions = [1_usize, 2, 4][index];
            let divisions_f64 =
                f64::from(u32::try_from(divisions).expect("fixture divisions fit u32"));
            let expected_spacing = (1.0 / (2.0 * divisions_f64.powi(2))).sqrt();
            let mesh_text = unit_square_triangular_mesh(divisions);
            fs::write(&mesh, mesh_text).expect("write grid mesh");
            let dry_run_directory = root.join(format!("dry-run-{level}"));
            let dry_run_request = ExternalSolverRequest {
                run_id: format!("su2-grid-dry-run-{level}"),
                solver_id: "SU2_CFD_DRY_RUN".to_owned(),
                solver_version_claim: "synthetic-ci-fixture-not-a-SU2-version".to_owned(),
                executable_path: executable.clone(),
                run_directory: dry_run_directory.clone(),
                arguments: vec!["-d".to_owned(), "config.cfg".to_owned()],
                environment: BTreeMap::new(),
                staged_inputs: vec![
                    stage_spec(&config, "config.cfg", "synthetic-fixed-physics-config"),
                    stage_spec(&mesh, "mesh.su2", "synthetic-grid-mesh"),
                ],
                expected_outputs: Vec::new(),
                timeout: Duration::from_secs(2),
                max_log_bytes_per_stream: 16 * 1024,
            };
            let dry_run_receipt =
                execute_external_solver(&dry_run_request).expect("execute fake SU2 dry run");
            assert!(dry_run_receipt.is_process_success());
            let required_history_fields = vec![
                "TIME_ITER".to_owned(),
                "INNER_ITER".to_owned(),
                "CL".to_owned(),
                "CD".to_owned(),
                "TAVG_DRAG".to_owned(),
                "TAVG_LIFT".to_owned(),
                "RMS_DENSITY".to_owned(),
                "RMS_MOMENTUM-X".to_owned(),
            ];
            let field_preflight = audit_su2_field_preflight(
                &dry_run_directory,
                "config.cfg",
                "mesh.su2",
                &required_history_fields,
            )
            .expect("dry-run field preflight");
            let boundary_audit = audit_su2_boundary_markers(
                &dry_run_directory,
                "config.cfg",
                "mesh.su2",
                &[Su2BoundaryOptionSpec {
                    option_name: "MARKER_HEATFLUX".to_owned(),
                    record_width: 2,
                    marker_positions: vec![0],
                }],
                &["MARKER_MONITORING".to_owned()],
            )
            .expect("dry-run boundary audit");

            let run_directory = root.join(format!("run-{level}"));
            let drag = 0.02 + 0.008 * expected_spacing.powi(2);
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
            let screen = screen_su2_history(&history, &criteria).expect("screen history");
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
                dry_run_directory,
                field_preflight,
                boundary_audit,
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
            protocol_manifest_sha256: sha256_hex(b"synthetic protocol fixture")
                .expect("protocol digest"),
            quantity: "section drag coefficient".to_owned(),
            quantity_field: "CD".to_owned(),
            quantity_unit: "dimensionless".to_owned(),
            source_reference: "synthetic fake-solver known answer".to_owned(),
            spacing_definition: SU2_GRID_SPACING_DEFINITION_2D.to_owned(),
            mesh_coordinate_unit: "m".to_owned(),
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
        assert!((report.members[0].quantity_value - 0.024).abs() < 1.0e-11);
        assert!((report.members[1].quantity_value - 0.021).abs() < 1.0e-11);
        assert!((report.members[2].quantity_value - 0.02025).abs() < 1.0e-11);
        assert!((report.members[0].characteristic_spacing - 0.5_f64.sqrt()).abs() < 1.0e-12);
        assert!((report.members[1].characteristic_spacing - 0.125_f64.sqrt()).abs() < 1.0e-12);
        assert!((report.members[2].characteristic_spacing - 0.03125_f64.sqrt()).abs() < 1.0e-12);
        assert_eq!(report.members[0].mesh_cell_count, 2);
        assert_eq!(report.members[1].mesh_cell_count, 8);
        assert_eq!(report.members[2].mesh_cell_count, 32);
        assert!((report.gci_report.triplets[0].observed_order - 2.0).abs() < 1.0e-10);
        assert!((report.gci_report.triplets[0].extrapolated_value - 0.02).abs() < 1.0e-10);
        assert_eq!(report_to_csv(&report).lines().count(), 4);
        assert_eq!(gci_report_to_csv(&report).lines().count(), 2);
        verify_su2_grid_family_report(&study, &report).expect("independent run-bundle replay");

        let member = &study.members[0];
        let bundle_inputs = Su2VerifiedRunBundleInputs {
            dry_run_directory: &member.dry_run_directory,
            production_run_directory: &member.run_directory,
            field_preflight: &member.field_preflight,
            boundary_audit: &member.boundary_audit,
            history: &member.history,
            convergence_screen: &member.convergence_screen,
            input_bindings: &member.input_bindings,
            additional_output_paths: &member.additional_output_paths,
            lineage_binding: &member.lineage_binding,
        };
        assert!(matches!(
            &member.field_preflight.convergence_protocol.window,
            crate::su2_field_preflight::Su2WindowConvergenceProtocol::Enabled {
                fields,
                window_start_iter: 10,
                convergence_start_iter: 10,
                cauchy_elements: 6,
                cauchy_epsilon,
                window_function,
            } if fields == &vec!["TAVG_DRAG".to_owned(), "TAVG_LIFT".to_owned()]
                && cauchy_epsilon == "1E-3"
                && window_function == "SQUARE"
        ));
        let inner_screen = crate::su2_unsteady_inner_screen::screen_su2_unsteady_inner_convergence(
            &member.field_preflight,
            &member.history,
        )
        .expect("all saved time steps satisfy the inner residual threshold");
        assert_eq!(inner_screen.contiguous_time_step_count, 8);
        assert_eq!(inner_screen.fields[0].field, "RMS_DENSITY");
        assert_eq!(inner_screen.fields[0].failed_time_step_count, 0);
        let mut failing_inner_history = member.history.clone();
        let residual_index = failing_inner_history
            .numeric_fields
            .iter()
            .position(|field| field.column == "RMS_DENSITY")
            .expect("RMS_DENSITY imported");
        failing_inner_history.points[0].values[residual_index] = -7.0;
        assert!(
            crate::su2_unsteady_inner_screen::screen_su2_unsteady_inner_convergence(
                &member.field_preflight,
                &failing_inner_history,
            )
            .expect_err("a failed inner step must not be averaged away")
            .contains("exceeded threshold")
        );
        let mut gapped_inner_history = member.history.clone();
        gapped_inner_history.points[1].iteration += 1;
        assert!(
            crate::su2_unsteady_inner_screen::screen_su2_unsteady_inner_convergence(
                &member.field_preflight,
                &gapped_inner_history,
            )
            .expect_err("a missing time step must reject")
            .contains("TIME_ITER gap")
        );
        let bundle = create_su2_verified_run_bundle(&bundle_inputs)
            .expect("compose dry-run, per-time-step inner, window and production run evidence");
        assert_eq!(
            bundle.unsteady_inner_screen_sha256,
            inner_screen.screen_sha256
        );
        crate::su2_verified_run_bundle::verify_su2_verified_run_bundle(
            &bundle_inputs,
            &bundle,
        )
        .expect("replay complete SU2 run bundle");
        assert_eq!(bundle.resource_evidence.unknown_measurement_count(), 4);
        assert_eq!(
            bundle.resource_evidence_sha256.len(),
            64,
            "resource evidence has a canonical digest"
        );
        assert_eq!(report.members[0].verified_run_bundle_sha256, bundle.bundle_sha256);

        #[cfg(target_os = "linux")]
        {
            let oracle_directory = root.join("independent-run-bundle-oracle");
            fs::create_dir_all(&oracle_directory).expect("create oracle directory");
            let bundle_csv = oracle_directory.join("bundle.csv");
            let preflight_csv = oracle_directory.join("preflight.csv");
            let boundary_csv = oracle_directory.join("boundary.csv");
            let history_csv = oracle_directory.join("history-import.csv");
            let screen_csv = oracle_directory.join("screen.csv");
            let lineage_csv = oracle_directory.join("lineage.csv");
            let inner_screen_csv = oracle_directory.join("unsteady-inner-screen.csv");
            fs::write(
                &bundle_csv,
                crate::su2_verified_run_bundle::report_to_csv(&bundle),
            )
            .expect("write composed bundle report");
            fs::write(&preflight_csv, member.field_preflight.to_csv())
                .expect("write preflight report");
            fs::write(&boundary_csv, member.boundary_audit.to_csv())
                .expect("write boundary report");
            fs::write(&history_csv, member.history.to_csv()).expect("write history import");
            fs::write(
                &screen_csv,
                crate::su2_convergence_screen::screen_to_csv(&member.convergence_screen),
            )
            .expect("write convergence screen");
            fs::write(
                &lineage_csv,
                crate::su2_run_binding::binding_to_csv(&member.lineage_binding),
            )
            .expect("write production lineage");
            fs::write(
                &inner_screen_csv,
                crate::su2_unsteady_inner_screen::report_to_csv(&inner_screen),
            )
            .expect("write unsteady inner-step screen");
            let preflight_oracle = std::process::Command::new("python3")
                .arg(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/scripts/verify-su2-field-preflight-independent.py"
                ))
                .arg(&member.dry_run_directory)
                .arg(&preflight_csv)
                .output()
                .expect("execute independent SU2 multi-mode preflight oracle");
            assert!(
                preflight_oracle.status.success(),
                "independent preflight oracle failed: {}",
                String::from_utf8_lossy(&preflight_oracle.stderr)
            );
            assert!(
                String::from_utf8_lossy(&preflight_oracle.stdout)
                    .contains("window_enabled=True")
            );

            let oracle = std::process::Command::new("python3")
                .arg(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/scripts/verify-su2-run-bundle-independent.py"
                ))
                .arg(&member.dry_run_directory)
                .arg(&member.run_directory)
                .arg(&bundle_csv)
                .arg(&preflight_csv)
                .arg(&boundary_csv)
                .arg(&history_csv)
                .arg(&screen_csv)
                .arg(&lineage_csv)
                .arg(&inner_screen_csv)
                .output()
                .expect("execute independent SU2 run-bundle oracle");
            assert!(
                oracle.status.success(),
                "independent oracle failed: {}",
                String::from_utf8_lossy(&oracle.stderr)
            );
            assert!(
                String::from_utf8_lossy(&oracle.stdout)
                    .contains("independent_su2_verified_run_bundle_oracle=pass")
            );
        }

        let mut wrong_dry_run = study.clone();
        wrong_dry_run.members[0].dry_run_directory =
            wrong_dry_run.members[1].dry_run_directory.clone();
        assert!(
            evaluate_su2_grid_family_study(&wrong_dry_run)
                .expect_err("another grid's dry-run receipt must be rejected")
                .contains("replayed SU2 field preflight differs")
        );

        let mut altered_boundary = study.clone();
        altered_boundary.members[0].boundary_audit.assignments[0].marker =
            "not-the-mesh-marker".to_owned();
        assert!(
            evaluate_su2_grid_family_study(&altered_boundary)
                .expect_err("mutated boundary assignment must be rejected")
                .contains("replayed SU2 boundary audit differs")
        );

        let mut altered_preflight = study.clone();
        altered_preflight.members[0]
            .field_preflight
            .available_history_fields
            .retain(|field| field != "CL");
        assert!(
            evaluate_su2_grid_family_study(&altered_preflight)
                .expect_err("omitted discovered history field must be rejected")
                .contains("replayed SU2 field preflight differs")
        );

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
    fn mesh_family_invariants_reject_domain_changes_and_marker_set_changes() {
        let reference_markers = vec!["farfield".to_owned(), "wall".to_owned()];
        validate_mesh_family_invariants(
            Some(1.0),
            Some(&reference_markers),
            1.0 + 1.0e-12,
            &["wall".to_owned(), "farfield".to_owned()],
            "grid-2",
        )
        .expect("same domain and marker set accepted");

        assert!(
            validate_mesh_family_invariants(
                Some(1.0),
                Some(&reference_markers),
                1.01,
                &reference_markers,
                "grid-2",
            )
            .expect_err("changed domain area rejected")
            .contains("changes total 2D mesh domain area")
        );
        assert!(
            validate_mesh_family_invariants(
                Some(1.0),
                Some(&reference_markers),
                1.0,
                &["farfield".to_owned(), "symmetry".to_owned()],
                "grid-2",
            )
            .expect_err("changed marker set rejected")
            .contains("changes the mesh marker-tag set")
        );
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
            spacing_definition: SU2_GRID_SPACING_DEFINITION_2D.to_owned(),
            mesh_coordinate_unit: "m".to_owned(),
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
