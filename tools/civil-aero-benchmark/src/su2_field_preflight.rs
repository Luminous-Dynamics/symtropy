//! SU2 dry-run preflight: bind selected solver fields and configuration selectors to a process receipt.
//!
//! SU2 recommends SU2_CFD -d with the case configuration to discover fields available to the
//! exact solver/configuration. Unknown CONV_FIELD names may disable convergence checks, so they
//! must be rejected before a production run. This is configuration compatibility evidence only.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::external_solver::{
    ExternalSolverReceipt, INVOCATION_MANIFEST_FILE, STDERR_LOG_FILE, STDOUT_LOG_FILE,
    read_external_solver_receipt, verify_external_solver_receipt,
};
use crate::integrity::sha256_hex;
use crate::su2_mesh::inspect_su2_mesh;

pub const SU2_FIELD_PREFLIGHT_SCHEMA: &str = "su2-field-preflight/0.1.0";
pub const SU2_FIELD_PREFLIGHT_EVIDENCE_LEVEL: &str = "receipt_bound_su2_dry_run_field_discovery_and_config_presence_only_not_history_convergence_or_validation";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Su2FieldPreflightReport {
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
    pub solver_family: String,
    pub configured_convergence_field: String,
    pub history_output_selectors: Vec<String>,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub available_history_fields: Vec<String>,
    pub required_history_fields: Vec<String>,
    pub status: String,
    pub preflight_sha256: String,
    pub evidence_level: String,
}

impl Su2FieldPreflightReport {
    pub fn to_csv(&self) -> String {
        let mut output = String::from(
            "schema,run_id,solver_id,runner_receipt_sha256,executable_sha256,configuration_path,configuration_byte_length,configuration_sha256,mesh_path,mesh_byte_length,mesh_sha256,solver_family,configured_convergence_field,history_output_selectors,stdout_sha256,stderr_sha256,available_history_fields,required_history_fields,status,preflight_sha256,evidence_level\n",
        );
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
            csv_escape(&self.solver_family),
            csv_escape(&self.configured_convergence_field),
            csv_escape(&self.history_output_selectors.join(";")),
            self.stdout_sha256.clone(),
            self.stderr_sha256.clone(),
            csv_escape(&self.available_history_fields.join(";")),
            csv_escape(&self.required_history_fields.join(";")),
            csv_escape(&self.status),
            self.preflight_sha256.clone(),
            csv_escape(&self.evidence_level),
        ];
        output.push_str(&fields.join(","));
        output.push('\n');
        output
    }
}

fn validate_relative_path(value: &str) -> Result<PathBuf, String> {
    if value.trim().is_empty() {
        return Err("preflight artifact path must not be empty".to_owned());
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(
            "preflight artifact path must contain only normal run-relative components".to_owned(),
        );
    }
    Ok(path.to_path_buf())
}

fn staged_input<'a>(
    receipt: &'a ExternalSolverReceipt,
    relative_path: &str,
) -> Result<&'a crate::external_solver::StagedInputReceipt, String> {
    validate_relative_path(relative_path)?;
    receipt
        .staged_inputs
        .iter()
        .find(|input| input.relative_path == relative_path)
        .ok_or_else(|| {
            format!("preflight input {relative_path} is not staged in the process receipt")
        })
}

fn decode_hex(value: &str, field: &str) -> Result<String, String> {
    if value.len() % 2 != 0 {
        return Err(format!("invocation manifest {field} has odd-length hex"));
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&value[index..index + 2], 16)
                .map_err(|error| format!("invocation manifest {field} has invalid hex: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes)
        .map_err(|error| format!("invocation manifest {field} is not UTF-8: {error}"))
}

fn parse_key_values(input: &str, label: &str) -> Result<BTreeMap<String, String>, String> {
    let mut values = BTreeMap::new();
    for (index, line) in input.lines().enumerate() {
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("{label} line {} is not key=value", index + 1))?;
        if key.is_empty() || values.insert(key.to_owned(), value.to_owned()).is_some() {
            return Err(format!(
                "{label} has an empty or duplicate key on line {}",
                index + 1
            ));
        }
    }
    Ok(values)
}

fn verify_dry_run_invocation(
    root: &Path,
    receipt: &ExternalSolverReceipt,
    configuration_path: &str,
) -> Result<(), String> {
    let manifest = fs::read_to_string(root.join(INVOCATION_MANIFEST_FILE))
        .map_err(|error| format!("cannot read SU2 dry-run invocation manifest: {error}"))?;
    let values = parse_key_values(&manifest, "invocation manifest")?;
    let count = values
        .get("argument_count")
        .ok_or_else(|| "invocation manifest is missing argument_count".to_owned())?
        .parse::<usize>()
        .map_err(|error| format!("invocation manifest argument_count is invalid: {error}"))?;
    if count != 2 {
        return Err("SU2 dry-run invocation must contain exactly two arguments".to_owned());
    }
    let flag = decode_hex(
        values
            .get("argument_0_utf8_hex")
            .ok_or_else(|| "invocation manifest is missing argument_0_utf8_hex".to_owned())?,
        "argument_0_utf8_hex",
    )?;
    let config_argument = decode_hex(
        values
            .get("argument_1_utf8_hex")
            .ok_or_else(|| "invocation manifest is missing argument_1_utf8_hex".to_owned())?,
        "argument_1_utf8_hex",
    )?;
    if flag != "-d" || config_argument != configuration_path {
        return Err(format!(
            "invocation must be SU2_CFD -d {configuration_path}; observed arguments were {flag:?} {config_argument:?}"
        ));
    }
    if receipt.solver_id != "SU2_CFD_DRY_RUN" {
        return Err("preflight receipt solver_id must be SU2_CFD_DRY_RUN".to_owned());
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedConfiguration {
    solver_family: String,
    convergence_field: String,
    history_output_selectors: Vec<String>,
}

fn parse_config_value<'a>(
    values: &'a BTreeMap<String, String>,
    key: &str,
) -> Result<&'a str, String> {
    values
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("SU2 config is missing required {key}"))
}

fn strip_list_syntax(value: &str) -> &str {
    let trimmed = value.trim();
    trimmed
        .strip_prefix('(')
        .and_then(|inner| inner.strip_suffix(')'))
        .unwrap_or(trimmed)
        .trim()
}

fn insert_config_option(
    values: &mut BTreeMap<String, String>,
    line: &str,
    line_number: usize,
) -> Result<(), String> {
    let Some((raw_key, raw_value)) = line.split_once('=') else {
        return Ok(());
    };
    let key = raw_key.trim();
    if matches!(key, "SOLVER" | "CONV_FIELD" | "HISTORY_OUTPUT") {
        if values
            .insert(key.to_owned(), raw_value.trim().to_owned())
            .is_some()
        {
            return Err(format!("SU2 config line {line_number} duplicates {key}"));
        }
    }
    Ok(())
}

fn parse_configuration(bytes: &[u8]) -> Result<ParsedConfiguration, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("SU2 physics config is not UTF-8/ASCII text: {error}"))?;
    if !text.is_ascii() {
        return Err("SU2 physics config must be ASCII".to_owned());
    }
    let mut values = BTreeMap::new();
    let mut logical: Option<(usize, String)> = None;
    for (index, raw_line) in text.lines().enumerate() {
        let content = raw_line.split('%').next().unwrap_or("").trim();
        if content.is_empty() {
            continue;
        }
        let line_number = index + 1;
        if let Some((start_line, mut accumulated)) = logical.take() {
            let continuation = content.strip_suffix('\\');
            let fragment = continuation.unwrap_or(content).trim();
            if !fragment.is_empty() {
                accumulated.push(' ');
                accumulated.push_str(fragment);
            }
            if continuation.is_some() {
                logical = Some((start_line, accumulated));
            } else {
                insert_config_option(&mut values, &accumulated, start_line)?;
            }
            continue;
        }
        if content.ends_with('\\') {
            logical = Some((line_number, content.trim_end_matches('\\').trim().to_owned()));
            continue;
        }
        insert_config_option(&mut values, content, line_number)?;
    }
    if let Some((start_line, _)) = logical {
        return Err(format!(
            "SU2 config line {start_line}: unterminated line continuation"
        ));
    }
    let solver_family = parse_config_value(&values, "SOLVER")?.trim().to_owned();
    if solver_family.is_empty() {
        return Err("SU2 SOLVER value must not be empty".to_owned());
    }
    let convergence_field = strip_list_syntax(parse_config_value(&values, "CONV_FIELD")?)
        .trim()
        .trim_matches('"')
        .to_owned();
    if convergence_field.is_empty() || convergence_field.contains(',') {
        return Err("CONV_FIELD must declare exactly one non-empty field name".to_owned());
    }
    let history_output_selectors =
        strip_list_syntax(parse_config_value(&values, "HISTORY_OUTPUT")?)
            .split(',')
            .map(|value| value.trim().trim_matches('"').to_owned())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
    if history_output_selectors.is_empty() {
        return Err("HISTORY_OUTPUT must declare at least one field or group selector".to_owned());
    }
    Ok(ParsedConfiguration {
        solver_family,
        convergence_field,
        history_output_selectors,
    })
}

fn extract_available_history_fields(output: &str) -> Result<Vec<String>, String> {
    let lines = output.lines().collect::<Vec<_>>();
    let start = lines
        .iter()
        .position(|line| {
            line.to_ascii_lowercase()
                .contains("available screen/history output fields for the current configuration")
        })
        .ok_or_else(|| {
            "SU2 dry-run output is missing the available screen/history fields section".to_owned()
        })?;
    let mut tokens = BTreeSet::new();
    for line in lines.iter().skip(start + 1) {
        let normalized = line.trim();
        let lower = normalized.to_ascii_lowercase();
        if lower.starts_with("available ") && lower.contains("output fields") {
            break;
        }
        if normalized.is_empty() {
            continue;
        }
        let source = if normalized.contains('|') {
            normalized
                .trim_matches('|')
                .split('|')
                .next()
                .unwrap_or("")
                .trim()
        } else {
            normalized
        };
        let mut token = String::new();
        for character in source.chars().chain(std::iter::once(' ')) {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '-') {
                token.push(character);
            } else if !token.is_empty() {
                if token != "Field" && token != "Name" && token != "FIELD" && token != "NAME" {
                    tokens.insert(token.clone());
                }
                token.clear();
            }
        }
    }
    if tokens.is_empty() {
        return Err(
            "SU2 dry-run screen/history fields section contained no field tokens".to_owned(),
        );
    }
    Ok(tokens.into_iter().collect())
}

fn append_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    append_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn canonical_bytes(report: &Su2FieldPreflightReport) -> Vec<u8> {
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
        report.solver_family.as_str(),
        report.configured_convergence_field.as_str(),
        report.stdout_sha256.as_str(),
        report.stderr_sha256.as_str(),
        report.status.as_str(),
        report.evidence_level.as_str(),
    ] {
        append_string(&mut bytes, value);
    }
    append_u64(&mut bytes, report.configuration_byte_length);
    append_u64(&mut bytes, report.mesh_byte_length);
    append_u64(&mut bytes, report.history_output_selectors.len() as u64);
    for selector in &report.history_output_selectors {
        append_string(&mut bytes, selector);
    }
    append_u64(&mut bytes, report.available_history_fields.len() as u64);
    for field in &report.available_history_fields {
        append_string(&mut bytes, field);
    }
    append_u64(&mut bytes, report.required_history_fields.len() as u64);
    for field in &report.required_history_fields {
        append_string(&mut bytes, field);
    }
    bytes
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

/// Audit a receipt-bound SU2 -d run before a production solve.
///
/// Required fields must include every output selected by the subsequent history importer and
/// every residual/monitor field used by the convergence screen. This preflight proves availability
/// for the pinned executable and solver/configuration pair; it does not prove the output was written
/// or that any simulation value converged.
pub fn audit_su2_field_preflight(
    run_directory: &Path,
    configuration_path: &str,
    mesh_path: &str,
    required_history_fields: &[String],
) -> Result<Su2FieldPreflightReport, String> {
    let config_relative = validate_relative_path(configuration_path)?;
    let mesh_relative = validate_relative_path(mesh_path)?;
    if configuration_path == mesh_path {
        return Err("configuration and mesh paths must be distinct".to_owned());
    }
    if required_history_fields.is_empty() {
        return Err("at least one required history field must be declared".to_owned());
    }
    let mut required = BTreeSet::new();
    for field in required_history_fields {
        if field.trim().is_empty() || !required.insert(field.clone()) {
            return Err("required history fields must be non-empty and unique".to_owned());
        }
    }

    let receipt = read_external_solver_receipt(run_directory)?;
    if receipt.solver_id != "SU2_CFD_DRY_RUN" || !receipt.is_process_success() {
        return Err(
            "SU2 field preflight requires a successful SU2_CFD_DRY_RUN process receipt".to_owned(),
        );
    }
    verify_external_solver_receipt(run_directory, &receipt)?;
    let root = fs::canonicalize(run_directory)
        .map_err(|error| format!("cannot resolve SU2 preflight run directory: {error}"))?;
    verify_dry_run_invocation(&root, &receipt, configuration_path)?;

    let config_input = staged_input(&receipt, configuration_path)?;
    let mesh_input = staged_input(&receipt, mesh_path)?;
    let config_bytes = fs::read(root.join(&config_relative))
        .map_err(|error| format!("cannot read staged SU2 config: {error}"))?;
    let mesh_bytes = fs::read(root.join(&mesh_relative))
        .map_err(|error| format!("cannot read staged SU2 mesh: {error}"))?;
    if config_bytes.len() as u64 != config_input.byte_length
        || sha256_hex(&config_bytes)? != config_input.sha256
    {
        return Err("staged SU2 configuration bytes differ from receipt".to_owned());
    }
    if mesh_bytes.len() as u64 != mesh_input.byte_length
        || sha256_hex(&mesh_bytes)? != mesh_input.sha256
    {
        return Err("staged SU2 mesh bytes differ from receipt".to_owned());
    }
    inspect_su2_mesh(&mesh_bytes)?;

    let config = parse_configuration(&config_bytes)?;
    let stdout = fs::read(root.join(STDOUT_LOG_FILE))
        .map_err(|error| format!("cannot read SU2 dry-run stdout: {error}"))?;
    let stderr = fs::read(root.join(STDERR_LOG_FILE))
        .map_err(|error| format!("cannot read SU2 dry-run stderr: {error}"))?;
    let mut combined = stdout.clone();
    combined.push(b'\n');
    combined.extend_from_slice(&stderr);
    let combined_text = String::from_utf8(combined)
        .map_err(|error| format!("SU2 dry-run output is not valid UTF-8: {error}"))?;
    let available_history_fields = extract_available_history_fields(&combined_text)?;
    let available = available_history_fields
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if !available.contains(&config.convergence_field) {
        return Err(format!(
            "CONV_FIELD {} is not present in the exact solver/configuration dry-run field inventory; SU2 may ignore it and disable convergence checking",
            config.convergence_field
        ));
    }
    let missing = required
        .iter()
        .filter(|field| !available.contains(field.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "required history fields are unavailable in SU2 dry-run: {}",
            missing.join(",")
        ));
    }

    let mut report = Su2FieldPreflightReport {
        schema: SU2_FIELD_PREFLIGHT_SCHEMA.to_owned(),
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
        solver_family: config.solver_family,
        configured_convergence_field: config.convergence_field,
        history_output_selectors: config.history_output_selectors,
        stdout_sha256: receipt.stdout_sha256.clone(),
        stderr_sha256: receipt.stderr_sha256.clone(),
        available_history_fields,
        required_history_fields: required.into_iter().collect(),
        status: "required_fields_available_and_conv_field_recognized".to_owned(),
        preflight_sha256: String::new(),
        evidence_level: SU2_FIELD_PREFLIGHT_EVIDENCE_LEVEL.to_owned(),
    };
    report.preflight_sha256 = sha256_hex(&canonical_bytes(&report))?;
    Ok(report)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::external_solver::{ExternalSolverRequest, StagedInputSpec, execute_external_solver};
    use crate::integrity::{sha256_file, sha256_hex};
    use std::collections::BTreeMap;
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;

    fn root() -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "su2-field-preflight-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create test root");
        path
    }

    fn fixture_run(root: &Path, configured_field: &str, inventory: &str) -> PathBuf {
        fs::create_dir_all(root).expect("create fixture root");
        let config = root.join("config.cfg");
        let mesh = root.join("mesh.su2");
        fs::write(
            &config,
            format!(
                "SOLVER= NAVIER_STOKES\nCONV_FIELD= {configured_field}\nHISTORY_OUTPUT= (ITER, RMS_RES, AERO_COEFF)\n"
            ),
        )
        .expect("write config");
        fs::write(
            &mesh,
            "NDIME= 2\nNELEM= 1\n5 0 1 2\nNPOIN= 3\n0 0\n1 0\n0 1\nNMARK= 0\n",
        )
        .expect("write mesh");

        let executable = root.join("fake-su2-cfd.sh");
        fs::write(
            &executable,
            format!("#!/bin/sh\nset -eu\nprintf '%s\\n' '{inventory}'\n"),
        )
        .expect("write fake dry-run executable");
        let mut permissions = fs::metadata(&executable)
            .expect("script metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).expect("make script executable");

        let config_bytes = fs::read(&config).expect("read config");
        let mesh_bytes = fs::read(&mesh).expect("read mesh");
        let staged_inputs = vec![
            StagedInputSpec {
                source_path: config.clone(),
                relative_path: "config.cfg".to_owned(),
                expected_byte_length: config_bytes.len() as u64,
                expected_sha256: sha256_hex(&config_bytes).expect("config digest"),
                source_reference: "synthetic_config_fixture".to_owned(),
            },
            StagedInputSpec {
                source_path: mesh.clone(),
                relative_path: "mesh.su2".to_owned(),
                expected_byte_length: mesh_bytes.len() as u64,
                expected_sha256: sha256_hex(&mesh_bytes).expect("mesh digest"),
                source_reference: "synthetic_mesh_fixture".to_owned(),
            },
        ];
        let run_directory = root.join("run");
        let request = ExternalSolverRequest {
            run_id: "su2-field-preflight-fixture".to_owned(),
            solver_id: "SU2_CFD_DRY_RUN".to_owned(),
            solver_version_claim: "synthetic-dry-run-not-SU2".to_owned(),
            executable_path: executable.clone(),
            run_directory: run_directory.clone(),
            arguments: vec!["-d".to_owned(), "config.cfg".to_owned()],
            environment: BTreeMap::new(),
            staged_inputs,
            expected_outputs: Vec::new(),
            timeout: Duration::from_secs(2),
            max_log_bytes_per_stream: 16 * 1024,
        };
        let receipt = execute_external_solver(&request).expect("execute dry-run fixture");
        assert!(receipt.is_process_success());
        assert_eq!(receipt.status, "process_exited_zero_no_outputs_declared");
        assert_eq!(
            receipt.executable_sha256,
            sha256_file(&executable).expect("hash fixture executable")
        );
        run_directory
    }

    const INVENTORY: &str = "Available screen/history output fields for the current configuration:\nITER\nINNER_ITER\nRMS_DENSITY\nRMS_MOMENTUM-X\nDRAG\nCD\nAvailable volume output fields for the current configuration:\nPRESSURE\n";

    #[test]
    fn receipt_bound_dry_run_recognizes_convergence_and_history_fields() {
        let root = root();
        let run = fixture_run(&root, "RMS_DENSITY", INVENTORY);
        let report = audit_su2_field_preflight(
            &run,
            "config.cfg",
            "mesh.su2",
            &[
                "RMS_DENSITY".to_owned(),
                "RMS_MOMENTUM-X".to_owned(),
                "DRAG".to_owned(),
                "CD".to_owned(),
            ],
        )
        .expect("field preflight passes");
        assert_eq!(
            report.status,
            "required_fields_available_and_conv_field_recognized"
        );
        assert_eq!(report.available_history_fields.len(), 6);
        assert_eq!(report.required_history_fields.len(), 4);
        fs::remove_dir_all(root).expect("remove test fixture");
    }

    #[test]
    fn parses_su2_backslash_continued_history_output_lists() {
        let configuration = parse_configuration(
            b"SOLVER= NAVIER_STOKES\nCONV_FIELD= RMS_DENSITY\nHISTORY_OUTPUT= (ITER, \\\n  RMS_RES, \\\n  AERO_COEFF)\n",
        )
        .expect("valid continued configuration");
        assert_eq!(
            configuration.history_output_selectors,
            vec!["ITER", "RMS_RES", "AERO_COEFF"]
        );
        let incomplete = parse_configuration(
            b"SOLVER= NAVIER_STOKES\nCONV_FIELD= RMS_DENSITY\nHISTORY_OUTPUT= (ITER, \\\n",
        );
        assert!(incomplete.expect_err("unterminated continuation rejected").contains("unterminated"));
    }

    #[test]
    fn rejects_unknown_convergence_field_and_missing_history_field() {
        let root = root();
        let unknown = fixture_run(&root.join("unknown"), "RMS_NOT_A_FIELD", INVENTORY);
        assert!(
            audit_su2_field_preflight(&unknown, "config.cfg", "mesh.su2", &["DRAG".to_owned()])
                .expect_err("unknown CONV_FIELD rejected")
                .contains("SU2 may ignore it")
        );

        let missing = fixture_run(&root.join("missing"), "RMS_DENSITY", INVENTORY);
        assert!(
            audit_su2_field_preflight(
                &missing,
                "config.cfg",
                "mesh.su2",
                &["RMS_DENSITY".to_owned(), "MISSING_FIELD".to_owned()]
            )
            .expect_err("missing monitor rejected")
            .contains("required history fields are unavailable")
        );
        fs::remove_dir_all(root).expect("remove negative fixtures");
    }
}
