//! Receipt-bound audit of SU2 mesh marker tags against explicit config boundary assignments.
//! Parameterized option records use caller-declared widths and marker positions; no tuple
//! structure is inferred. Every mesh marker must map exactly once and all config markers resolve.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::external_solver::{
    ExternalSolverReceipt, INVOCATION_MANIFEST_FILE, read_external_solver_receipt,
    verify_external_solver_receipt,
};
use crate::integrity::sha256_hex;
use crate::su2_mesh::inspect_su2_mesh;

pub const SU2_BOUNDARY_AUDIT_SCHEMA: &str = "su2-boundary-marker-audit/0.1.0";
pub const SU2_BOUNDARY_AUDIT_EVIDENCE_LEVEL: &str = "receipt_bound_exact_mesh_marker_to_declared_config_boundary_option_mapping_not_boundary_physics_or_flow_validation";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Su2BoundaryOptionSpec {
    pub option_name: String,
    pub record_width: usize,
    pub marker_positions: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Su2BoundaryAssignment {
    pub marker: String,
    pub config_option: String,
    pub record_index: usize,
    pub marker_position: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Su2BoundaryAuditReport {
    pub schema: String,
    pub run_id: String,
    pub solver_id: String,
    pub runner_receipt_sha256: String,
    pub executable_sha256: String,
    pub configuration_path: String,
    pub configuration_byte_length: u64,
    pub configuration_sha256: String,
    pub mesh_path: String,
    pub mesh_byte_length: u64,
    pub mesh_sha256: String,
    pub mesh_dimension: u8,
    pub mesh_point_count: u64,
    pub mesh_volume_element_count: u64,
    pub mesh_markers: Vec<String>,
    pub boundary_option_specs: Vec<Su2BoundaryOptionSpec>,
    pub non_boundary_marker_options: Vec<String>,
    pub assignments: Vec<Su2BoundaryAssignment>,
    pub status: String,
    pub audit_sha256: String,
    pub evidence_level: String,
}

impl Su2BoundaryAuditReport {
    pub fn to_csv(&self) -> String {
        let mut output = String::from(
            "schema,run_id,solver_id,runner_receipt_sha256,executable_sha256,configuration_path,configuration_byte_length,configuration_sha256,mesh_path,mesh_byte_length,mesh_sha256,mesh_dimension,mesh_point_count,mesh_volume_element_count,mesh_markers,boundary_option_specs,non_boundary_marker_options,assigned_marker,config_option,record_index,marker_position,status,audit_sha256,evidence_level\n",
        );
        for assignment in &self.assignments {
            let fields = [
                csv_escape(&self.schema),
                csv_escape(&self.run_id),
                csv_escape(&self.solver_id),
                self.runner_receipt_sha256.clone(),
                self.executable_sha256.clone(),
                csv_escape(&self.configuration_path),
                self.configuration_byte_length.to_string(),
                self.configuration_sha256.clone(),
                csv_escape(&self.mesh_path),
                self.mesh_byte_length.to_string(),
                self.mesh_sha256.clone(),
                self.mesh_dimension.to_string(),
                self.mesh_point_count.to_string(),
                self.mesh_volume_element_count.to_string(),
                csv_escape(&self.mesh_markers.join(";")),
                csv_escape(&encode_specs(&self.boundary_option_specs)),
                csv_escape(&self.non_boundary_marker_options.join(";")),
                csv_escape(&assignment.marker),
                csv_escape(&assignment.config_option),
                assignment.record_index.to_string(),
                assignment.marker_position.to_string(),
                csv_escape(&self.status),
                self.audit_sha256.clone(),
                csv_escape(&self.evidence_level),
            ];
            output.push_str(&fields.join(","));
            output.push('\n');
        }
        output
    }
}

fn encode_specs(specs: &[Su2BoundaryOptionSpec]) -> String {
    specs
        .iter()
        .map(|spec| {
            format!(
                "{}:{}:{}",
                spec.option_name,
                spec.record_width,
                spec.marker_positions
                    .iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join("|")
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn append_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    append_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn canonical_bytes(report: &Su2BoundaryAuditReport) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        report.schema.as_str(),
        report.run_id.as_str(),
        report.solver_id.as_str(),
        report.runner_receipt_sha256.as_str(),
        report.executable_sha256.as_str(),
        report.configuration_path.as_str(),
        report.configuration_sha256.as_str(),
        report.mesh_path.as_str(),
        report.mesh_sha256.as_str(),
        report.status.as_str(),
        report.evidence_level.as_str(),
    ] {
        append_string(&mut bytes, value);
    }
    for value in [
        report.configuration_byte_length,
        report.mesh_byte_length,
        u64::from(report.mesh_dimension),
        report.mesh_point_count,
        report.mesh_volume_element_count,
    ] {
        append_u64(&mut bytes, value);
    }
    append_u64(&mut bytes, report.mesh_markers.len() as u64);
    for marker in &report.mesh_markers {
        append_string(&mut bytes, marker);
    }
    append_u64(&mut bytes, report.boundary_option_specs.len() as u64);
    for spec in &report.boundary_option_specs {
        append_string(&mut bytes, &spec.option_name);
        append_u64(&mut bytes, spec.record_width as u64);
        append_u64(&mut bytes, spec.marker_positions.len() as u64);
        for position in &spec.marker_positions {
            append_u64(&mut bytes, *position as u64);
        }
    }
    append_u64(&mut bytes, report.non_boundary_marker_options.len() as u64);
    for option in &report.non_boundary_marker_options {
        append_string(&mut bytes, option);
    }
    append_u64(&mut bytes, report.assignments.len() as u64);
    for row in &report.assignments {
        append_string(&mut bytes, &row.marker);
        append_string(&mut bytes, &row.config_option);
        append_u64(&mut bytes, row.record_index as u64);
        append_u64(&mut bytes, row.marker_position as u64);
    }
    bytes
}

fn validate_relative_path(value: &str) -> Result<PathBuf, String> {
    if value.trim().is_empty() {
        return Err("boundary audit path must not be empty".to_owned());
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|p| !matches!(p, Component::Normal(_)))
    {
        return Err(
            "boundary audit path must contain only normal run-relative components".to_owned(),
        );
    }
    Ok(path.to_path_buf())
}

fn staged_input<'a>(
    receipt: &'a ExternalSolverReceipt,
    path: &str,
) -> Result<&'a crate::external_solver::StagedInputReceipt, String> {
    validate_relative_path(path)?;
    let matches = receipt
        .staged_inputs
        .iter()
        .filter(|row| row.relative_path == path)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(format!(
            "expected exactly one staged input {path}, found {}",
            matches.len()
        ));
    }
    Ok(matches[0])
}

fn decode_hex(value: &str, name: &str) -> Result<String, String> {
    if !value.len().is_multiple_of(2) {
        return Err(format!("invocation argument {name} has odd-length hex"));
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&value[i..i + 2], 16)
                .map_err(|e| format!("invocation argument {name} has invalid hex: {e}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|e| format!("invocation argument {name} is not UTF-8: {e}"))
}

fn verify_dry_run_invocation(root: &Path, config_path: &str) -> Result<(), String> {
    let text = fs::read_to_string(root.join(INVOCATION_MANIFEST_FILE))
        .map_err(|e| format!("cannot read invocation manifest: {e}"))?;
    let mut map = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("invocation line {} is not key=value", index + 1))?;
        if map.insert(key.to_owned(), value.to_owned()).is_some() {
            return Err(format!("invocation manifest duplicates {key}"));
        }
    }
    if map.get("argument_count").map(String::as_str) != Some("2") {
        return Err("boundary audit requires exactly two dry-run argv elements".to_owned());
    }
    let flag = decode_hex(
        map.get("argument_0_utf8_hex").ok_or("missing argv[0]")?,
        "argv[0]",
    )?;
    let path = decode_hex(
        map.get("argument_1_utf8_hex").ok_or("missing argv[1]")?,
        "argv[1]",
    )?;
    if flag != "-d" || path != config_path {
        return Err("boundary audit receipt does not bind SU2_CFD -d CONFIG_PATH".to_owned());
    }
    Ok(())
}

fn parse_config(bytes: &[u8]) -> Result<BTreeMap<String, String>, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| format!("SU2 config is not UTF-8: {e}"))?;
    if !text.is_ascii() {
        return Err("SU2 config must be ASCII".to_owned());
    }
    let mut values = BTreeMap::new();
    let mut pending: Option<(usize, String)> = None;
    for (index, raw) in text.lines().enumerate() {
        let line = raw.split('%').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let line_number = index + 1;
        let complete = if let Some((start, mut accumulated)) = pending.take() {
            let continued = line.strip_suffix('\\');
            let fragment = continued.unwrap_or(line).trim();
            if !fragment.is_empty() {
                accumulated.push(' ');
                accumulated.push_str(fragment);
            }
            if continued.is_some() {
                pending = Some((start, accumulated));
                None
            } else {
                Some((start, accumulated))
            }
        } else if let Some(fragment) = line.strip_suffix('\\') {
            pending = Some((line_number, fragment.trim().to_owned()));
            None
        } else {
            Some((line_number, line.to_owned()))
        };
        if let Some((source_line, record)) = complete
            && let Some((raw_key, raw_value)) = record.split_once('=')
        {
            let key = raw_key.trim();
            if key.is_empty()
                || values
                    .insert(key.to_owned(), raw_value.trim().to_owned())
                    .is_some()
            {
                return Err(format!(
                    "SU2 config has an empty or duplicate key at line {source_line}"
                ));
            }
        }
    }
    if let Some((line, _)) = pending {
        return Err(format!(
            "SU2 config line {line} has an unterminated continuation"
        ));
    }
    Ok(values)
}

fn parse_list(value: &str, option: &str) -> Result<Vec<String>, String> {
    let inner = value
        .trim()
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| format!("{option} must use parenthesized list syntax"))?;
    let fields = inner
        .split(',')
        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_owned())
        .collect::<Vec<_>>();
    if fields.is_empty() || fields.iter().any(String::is_empty) {
        return Err(format!("{option} contains empty list fields"));
    }
    Ok(fields)
}

fn validate_specs(
    boundary: &[Su2BoundaryOptionSpec],
    non_boundary: &[String],
) -> Result<(), String> {
    let mut names = BTreeSet::new();
    for spec in boundary {
        if !spec.option_name.starts_with("MARKER_")
            || spec.record_width == 0
            || spec.marker_positions.is_empty()
            || !names.insert(spec.option_name.as_str())
        {
            return Err(format!(
                "invalid or duplicate boundary option spec {}",
                spec.option_name
            ));
        }
        let mut positions = BTreeSet::new();
        for position in &spec.marker_positions {
            if *position >= spec.record_width || !positions.insert(*position) {
                return Err(format!(
                    "{} has invalid marker position {position}",
                    spec.option_name
                ));
            }
        }
    }
    for name in non_boundary {
        if !(name.starts_with("MARKER_") || name == "GEO_MARKER") || !names.insert(name.as_str()) {
            return Err(format!("invalid or duplicate non-boundary option {name}"));
        }
    }
    Ok(())
}

fn resolve_assignments(
    config: &BTreeMap<String, String>,
    mesh_markers: &[String],
    boundary: &[Su2BoundaryOptionSpec],
    non_boundary: &[String],
) -> Result<Vec<Su2BoundaryAssignment>, String> {
    validate_specs(boundary, non_boundary)?;
    let specs = boundary
        .iter()
        .map(|s| (s.option_name.as_str(), s))
        .collect::<BTreeMap<_, _>>();
    let ignored = non_boundary
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if boundary.is_empty() || mesh_markers.is_empty() {
        return Err(
            "boundary audit requires at least one boundary option and one mesh marker".to_owned(),
        );
    }
    for option in config
        .keys()
        .filter(|key| key.starts_with("MARKER_") || *key == "GEO_MARKER")
    {
        if !specs.contains_key(option.as_str()) && !ignored.contains(option.as_str()) {
            return Err(format!("unclassified SU2 marker option {option}"));
        }
    }
    let mesh = mesh_markers
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for option in non_boundary {
        if let Some(value) = config.get(option) {
            for marker in parse_list(value, option)? {
                if !mesh.contains(marker.as_str()) {
                    return Err(format!(
                        "non-boundary option {option} references marker {marker} absent from mesh"
                    ));
                }
            }
        }
    }
    let mut assigned = BTreeSet::new();
    let mut rows = Vec::new();
    for spec in boundary {
        let value = config.get(&spec.option_name).ok_or_else(|| {
            format!(
                "config is missing declared boundary option {}",
                spec.option_name
            )
        })?;
        let fields = parse_list(value, &spec.option_name)?;
        if !fields.len().is_multiple_of(spec.record_width) {
            return Err(format!(
                "{} values are not divisible by record width {}",
                spec.option_name, spec.record_width
            ));
        }
        for (record_index, record) in fields.chunks(spec.record_width).enumerate() {
            for position in &spec.marker_positions {
                let marker = &record[*position];
                if !mesh.contains(marker.as_str()) {
                    return Err(format!(
                        "{} references marker {marker} absent from mesh",
                        spec.option_name
                    ));
                }
                if !assigned.insert(marker.clone()) {
                    return Err(format!("mesh marker {marker} is assigned more than once"));
                }
                rows.push(Su2BoundaryAssignment {
                    marker: marker.clone(),
                    config_option: spec.option_name.clone(),
                    record_index,
                    marker_position: *position,
                });
            }
        }
    }
    let missing = mesh
        .difference(&assigned.iter().map(String::as_str).collect::<BTreeSet<_>>())
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "mesh markers have no boundary assignment: {}",
            missing.join(",")
        ));
    }
    rows.sort_by(|a, b| a.marker.cmp(&b.marker));
    Ok(rows)
}

pub fn audit_su2_boundary_markers(
    run_directory: &Path,
    configuration_path: &str,
    mesh_path: &str,
    boundary_options: &[Su2BoundaryOptionSpec],
    non_boundary_options: &[String],
) -> Result<Su2BoundaryAuditReport, String> {
    if configuration_path == mesh_path {
        return Err("configuration and mesh paths must be distinct".to_owned());
    }
    let config_rel = validate_relative_path(configuration_path)?;
    let mesh_rel = validate_relative_path(mesh_path)?;
    let receipt = read_external_solver_receipt(run_directory)?;
    if receipt.solver_id != "SU2_CFD_DRY_RUN" || !receipt.is_process_success() {
        return Err("boundary audit requires a successful SU2_CFD_DRY_RUN receipt".to_owned());
    }
    verify_external_solver_receipt(run_directory, &receipt)?;
    let root = fs::canonicalize(run_directory)
        .map_err(|e| format!("cannot resolve run directory: {e}"))?;
    verify_dry_run_invocation(&root, configuration_path)?;
    let config_staged = staged_input(&receipt, configuration_path)?;
    let mesh_staged = staged_input(&receipt, mesh_path)?;
    let config_bytes =
        fs::read(root.join(&config_rel)).map_err(|e| format!("cannot read staged config: {e}"))?;
    let mesh_bytes =
        fs::read(root.join(&mesh_rel)).map_err(|e| format!("cannot read staged mesh: {e}"))?;
    if config_bytes.len() as u64 != config_staged.byte_length
        || sha256_hex(&config_bytes)? != config_staged.sha256
    {
        return Err("staged config bytes differ from receipt".to_owned());
    }
    if mesh_bytes.len() as u64 != mesh_staged.byte_length
        || sha256_hex(&mesh_bytes)? != mesh_staged.sha256
    {
        return Err("staged mesh bytes differ from receipt".to_owned());
    }
    let config = parse_config(&config_bytes)?;
    let mesh = inspect_su2_mesh(&mesh_bytes)?;
    let mesh_markers = mesh
        .markers
        .iter()
        .map(|m| m.tag.clone())
        .collect::<Vec<_>>();
    let assignments = resolve_assignments(
        &config,
        &mesh_markers,
        boundary_options,
        non_boundary_options,
    )?;
    let mut report = Su2BoundaryAuditReport {
        schema: SU2_BOUNDARY_AUDIT_SCHEMA.to_owned(),
        run_id: receipt.run_id.clone(),
        solver_id: receipt.solver_id.clone(),
        runner_receipt_sha256: receipt.receipt_sha256.clone(),
        executable_sha256: receipt.executable_sha256.clone(),
        configuration_path: configuration_path.to_owned(),
        configuration_byte_length: config_bytes.len() as u64,
        configuration_sha256: sha256_hex(&config_bytes)?,
        mesh_path: mesh_path.to_owned(),
        mesh_byte_length: mesh_bytes.len() as u64,
        mesh_sha256: sha256_hex(&mesh_bytes)?,
        mesh_dimension: mesh.dimension,
        mesh_point_count: mesh.point_count,
        mesh_volume_element_count: mesh.volume_element_count,
        mesh_markers,
        boundary_option_specs: boundary_options.to_vec(),
        non_boundary_marker_options: non_boundary_options.to_vec(),
        assignments,
        status: "every_mesh_marker_mapped_once_and_config_marker_references_resolve".to_owned(),
        audit_sha256: String::new(),
        evidence_level: SU2_BOUNDARY_AUDIT_EVIDENCE_LEVEL.to_owned(),
    };
    report.audit_sha256 = sha256_hex(&canonical_bytes(&report))?;
    Ok(report)
}

/// Recompute a saved boundary audit from its original dry-run receipt and exact schema.
pub fn replay_su2_boundary_audit(
    run_directory: &Path,
    report: &Su2BoundaryAuditReport,
) -> Result<(), String> {
    verify_su2_boundary_audit(report)?;
    let recomputed = audit_su2_boundary_markers(
        run_directory,
        &report.configuration_path,
        &report.mesh_path,
        &report.boundary_option_specs,
        &report.non_boundary_marker_options,
    )?;
    if recomputed != *report {
        return Err("replayed SU2 boundary audit differs from supplied report".to_owned());
    }
    Ok(())
}

pub fn verify_su2_boundary_audit(report: &Su2BoundaryAuditReport) -> Result<(), String> {
    if report.schema != SU2_BOUNDARY_AUDIT_SCHEMA
        || report.status != "every_mesh_marker_mapped_once_and_config_marker_references_resolve"
        || report.evidence_level != SU2_BOUNDARY_AUDIT_EVIDENCE_LEVEL
    {
        return Err("boundary audit schema/status/evidence ceiling is invalid".to_owned());
    }
    validate_specs(
        &report.boundary_option_specs,
        &report.non_boundary_marker_options,
    )?;
    let mesh = report
        .mesh_markers
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let spec_map = report
        .boundary_option_specs
        .iter()
        .map(|spec| (spec.option_name.as_str(), spec))
        .collect::<BTreeMap<_, _>>();
    let assigned = report
        .assignments
        .iter()
        .map(|assignment| {
            let spec = spec_map
                .get(assignment.config_option.as_str())
                .ok_or_else(|| {
                    format!(
                        "assignment references undeclared boundary option {}",
                        assignment.config_option
                    )
                })?;
            if !spec.marker_positions.contains(&assignment.marker_position) {
                return Err(format!(
                    "assignment for marker {} has invalid tuple marker position",
                    assignment.marker
                ));
            }
            Ok(assignment.marker.as_str())
        })
        .collect::<Result<BTreeSet<_>, String>>()?;
    if mesh.len() != report.mesh_markers.len()
        || assigned.len() != report.assignments.len()
        || mesh != assigned
    {
        return Err(
            "boundary audit does not cover every unique mesh marker exactly once".to_owned(),
        );
    }
    if sha256_hex(&canonical_bytes(report))? != report.audit_sha256 {
        return Err("boundary audit canonical digest mismatch".to_owned());
    }
    Ok(())
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_all_mesh_markers_exactly_once_with_explicit_ignored_options() {
        let config = parse_config(
            b"MARKER_SYM= (sym_top)\nMARKER_FAR= (farfield)\nMARKER_HEATFLUX= (wall, 0.0)\nMARKER_MONITORING= (wall)\n",
        )
        .expect("config");
        let rows = resolve_assignments(
            &config,
            &[
                "farfield".to_owned(),
                "sym_top".to_owned(),
                "wall".to_owned(),
            ],
            &[
                Su2BoundaryOptionSpec {
                    option_name: "MARKER_SYM".to_owned(),
                    record_width: 1,
                    marker_positions: vec![0],
                },
                Su2BoundaryOptionSpec {
                    option_name: "MARKER_FAR".to_owned(),
                    record_width: 1,
                    marker_positions: vec![0],
                },
                Su2BoundaryOptionSpec {
                    option_name: "MARKER_HEATFLUX".to_owned(),
                    record_width: 2,
                    marker_positions: vec![0],
                },
            ],
            &["MARKER_MONITORING".to_owned()],
        )
        .expect("complete mapping");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].marker, "farfield");
        assert_eq!(rows[1].marker, "sym_top");
        assert_eq!(rows[2].marker, "wall");
    }

    #[test]
    fn supports_parameterized_and_paired_boundary_records() {
        let config = parse_config(
            b"MARKER_HEATFLUX= (wall, 0.0)\nMARKER_PERIODIC= (periodic_a, periodic_b, 0,0,0,0,0,0,1,0,0)\n",
        ).expect("config");
        let rows = resolve_assignments(
            &config,
            &[
                "periodic_a".to_owned(),
                "periodic_b".to_owned(),
                "wall".to_owned(),
            ],
            &[
                Su2BoundaryOptionSpec {
                    option_name: "MARKER_HEATFLUX".to_owned(),
                    record_width: 2,
                    marker_positions: vec![0],
                },
                Su2BoundaryOptionSpec {
                    option_name: "MARKER_PERIODIC".to_owned(),
                    record_width: 11,
                    marker_positions: vec![0, 1],
                },
            ],
            &[],
        )
        .expect("mapping");
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn rejects_missing_or_orphan_or_duplicate_mesh_marker_assignments() {
        let options = [Su2BoundaryOptionSpec {
            option_name: "MARKER_SYM".to_owned(),
            record_width: 1,
            marker_positions: vec![0],
        }];
        let missing = parse_config(b"MARKER_SYM= (sym_top)\n").expect("config");
        assert!(
            resolve_assignments(
                &missing,
                &["sym_top".to_owned(), "farfield".to_owned()],
                &options,
                &[]
            )
            .expect_err("missing tag")
            .contains("no boundary assignment")
        );
        let orphan = parse_config(b"MARKER_SYM= (ghost)\n").expect("config");
        assert!(
            resolve_assignments(&orphan, &["sym_top".to_owned()], &options, &[])
                .expect_err("orphan tag")
                .contains("absent from mesh")
        );
        let duplicate =
            parse_config(b"MARKER_SYM= (sym_top)\nMARKER_FAR= (sym_top)\n").expect("config");
        assert!(
            resolve_assignments(
                &duplicate,
                &["sym_top".to_owned()],
                &[
                    options[0].clone(),
                    Su2BoundaryOptionSpec {
                        option_name: "MARKER_FAR".to_owned(),
                        record_width: 1,
                        marker_positions: vec![0]
                    },
                ],
                &[]
            )
            .expect_err("duplicate tag")
            .contains("assigned more than once")
        );
    }

    #[test]
    fn rejects_unclassified_marker_options_bad_tuple_width_and_duplicate_config_keys() {
        let config =
            parse_config(b"MARKER_SYM= (sym_top)\nMARKER_UNKNOWN= (mystery)\n").expect("config");
        let spec = [Su2BoundaryOptionSpec {
            option_name: "MARKER_SYM".to_owned(),
            record_width: 1,
            marker_positions: vec![0],
        }];
        assert!(
            resolve_assignments(&config, &["sym_top".to_owned()], &spec, &[])
                .expect_err("unknown option")
                .contains("unclassified")
        );
        let bad = parse_config(b"MARKER_HEATFLUX= (wall, 0.0, orphan)\n").expect("config");
        let heat = [Su2BoundaryOptionSpec {
            option_name: "MARKER_HEATFLUX".to_owned(),
            record_width: 2,
            marker_positions: vec![0],
        }];
        assert!(
            resolve_assignments(&bad, &["wall".to_owned()], &heat, &[])
                .expect_err("bad tuple")
                .contains("not divisible")
        );
        assert!(parse_config(b"MARKER_SYM= (a)\nMARKER_SYM= (b)\n").is_err());
    }
}
