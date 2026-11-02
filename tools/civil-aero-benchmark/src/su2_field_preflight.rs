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

pub const SU2_FIELD_PREFLIGHT_SCHEMA: &str = "su2-field-preflight/0.2.0";
pub const SU2_FIELD_PREFLIGHT_EVIDENCE_LEVEL: &str = "receipt_bound_su2_dry_run_multi_field_steady_and_explicit_time_window_configuration_discovery_only_not_history_convergence_or_validation";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Su2WindowConvergenceProtocol {
    Disabled,
    Enabled {
        fields: Vec<String>,
        window_start_iter: u64,
        convergence_start_iter: u64,
        cauchy_elements: u64,
        cauchy_epsilon: String,
        window_function: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Su2ConvergenceProtocol {
    pub time_domain: bool,
    pub steady_fields: Vec<String>,
    pub steady_start_iteration: Option<u64>,
    pub steady_residual_min_log10: Option<String>,
    pub steady_cauchy_epsilon: Option<String>,
    pub steady_cauchy_elements: Option<u64>,
    pub window: Su2WindowConvergenceProtocol,
}

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
    /// Backward-compatible first field; use convergence_protocol.steady_fields for the full set.
    pub configured_convergence_field: String,
    pub convergence_protocol: Su2ConvergenceProtocol,
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
        let window_enabled = matches!(
            &self.convergence_protocol.window,
            Su2WindowConvergenceProtocol::Enabled { .. }
        );
        let mut output = String::from(
            "schema,run_id,solver_id,runner_receipt_sha256,executable_sha256,configuration_path,configuration_byte_length,configuration_sha256,mesh_path,mesh_byte_length,mesh_sha256,solver_family,configured_convergence_field,steady_convergence_fields,steady_start_iteration,steady_residual_min_log10,steady_cauchy_epsilon,steady_cauchy_elements,time_domain,window_cauchy_enabled,window_convergence_fields,window_start_iter,window_convergence_start_iter,window_cauchy_elements,window_cauchy_epsilon,window_function,history_output_selectors,stdout_sha256,stderr_sha256,available_history_fields,required_history_fields,status,preflight_sha256,evidence_level\n",
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
            csv_escape(&self.convergence_protocol.steady_fields.join(";")),
            format_optional_u64(self.convergence_protocol.steady_start_iteration),
            optional_string(&self.convergence_protocol.steady_residual_min_log10),
            optional_string(&self.convergence_protocol.steady_cauchy_epsilon),
            format_optional_u64(self.convergence_protocol.steady_cauchy_elements),
            if self.convergence_protocol.time_domain { "YES".to_owned() } else { "NO".to_owned() },
            if window_enabled { "YES".to_owned() } else { "NO".to_owned() },
            csv_escape(&window_fields(&self.convergence_protocol.window)),
            optional_window_u64(&self.convergence_protocol.window, 0),
            optional_window_u64(&self.convergence_protocol.window, 1),
            optional_window_u64(&self.convergence_protocol.window, 2),
            csv_escape(&window_epsilon(&self.convergence_protocol.window)),
            csv_escape(&window_function(&self.convergence_protocol.window)),
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
    if !value.len().is_multiple_of(2) {
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
    convergence_protocol: Su2ConvergenceProtocol,
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
    if matches!(
        key,
        "SOLVER"
            | "CONV_FIELD"
            | "CONV_RESIDUAL_MINVAL"
            | "CONV_STARTITER"
            | "CONV_CAUCHY_EPS"
            | "CONV_CAUCHY_ELEMS"
            | "HISTORY_OUTPUT"
            | "WINDOW_CAUCHY_CRIT"
            | "CONV_WINDOW_FIELD"
            | "WINDOW_START_ITER"
            | "CONV_WINDOW_STARTITER"
            | "CONV_WINDOW_CAUCHY_EPS"
            | "CONV_WINDOW_CAUCHY_ELEMS"
            | "WINDOW_FUNCTION"
            | "RESTART_ITER"
            | "TIME_DOMAIN"
    )
        && values
            .insert(key.to_owned(), raw_value.trim().to_owned())
            .is_some()
    {
        return Err(format!("SU2 config line {line_number} duplicates {key}"));
    }
    Ok(())
}

fn parse_field_list(value: &str, label: &str) -> Result<Vec<String>, String> {
    let fields = strip_list_syntax(value)
        .split(',')
        .map(|field| field.trim().trim_matches('"').trim().to_owned())
        .collect::<Vec<_>>();
    let mut unique = BTreeSet::new();
    if fields.is_empty()
        || fields.iter().any(String::is_empty)
        || fields.iter().any(|field| !unique.insert(field.clone()))
    {
        return Err(format!("{label} must contain non-empty, unique field names"));
    }
    Ok(fields)
}

fn optional_u64(values: &BTreeMap<String, String>, key: &str) -> Result<Option<u64>, String> {
    values
        .get(key)
        .map(|value| {
            value.trim().parse::<u64>().map_err(|error| {
                format!("SU2 config {key} must be an unsigned integer: {error}")
            })
        })
        .transpose()
}

fn optional_positive_decimal(
    values: &BTreeMap<String, String>,
    key: &str,
) -> Result<Option<String>, String> {
    values
        .get(key)
        .map(|value| {
            let raw = value.trim();
            let parsed = raw
                .parse::<f64>()
                .map_err(|error| format!("SU2 config {key} must be numeric: {error}"))?;
            if !parsed.is_finite() || parsed <= 0.0 {
                return Err(format!("SU2 config {key} must be finite and > 0"));
            }
            Ok(raw.to_owned())
        })
        .transpose()
}

fn required_u64(values: &BTreeMap<String, String>, key: &str) -> Result<u64, String> {
    optional_u64(values, key)?.ok_or_else(|| format!("SU2 config is missing required {key}"))
}

fn required_positive_decimal(values: &BTreeMap<String, String>, key: &str) -> Result<String, String> {
    optional_positive_decimal(values, key)?
        .ok_or_else(|| format!("SU2 config is missing required {key}"))
}

fn parse_config_boolean(values: &BTreeMap<String, String>, key: &str) -> Result<bool, String> {
    let raw = parse_config_value(values, key)?;
    match raw.trim().trim_matches('"').to_ascii_uppercase().as_str() {
        "YES" | "TRUE" => Ok(true),
        "NO" | "FALSE" => Ok(false),
        _ => Err(format!("{key} must be explicitly YES or NO")),
    }
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
            logical = Some((
                line_number,
                content.trim_end_matches('\\').trim().to_owned(),
            ));
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
    let time_domain = parse_config_boolean(&values, "TIME_DOMAIN")?;
    let steady_fields = parse_field_list(
        parse_config_value(&values, "CONV_FIELD")?,
        "CONV_FIELD",
    )?;
    let history_output_selectors =
        parse_field_list(parse_config_value(&values, "HISTORY_OUTPUT")?, "HISTORY_OUTPUT")?;

    let steady_start_iteration = optional_u64(&values, "CONV_STARTITER")?;
    let steady_residual_min_log10 = values
        .get("CONV_RESIDUAL_MINVAL")
        .map(|value| {
            let raw = value.trim();
            let parsed = raw.parse::<f64>().map_err(|error| {
                format!("SU2 config CONV_RESIDUAL_MINVAL must be numeric: {error}")
            })?;
            if !parsed.is_finite() {
                return Err("SU2 config CONV_RESIDUAL_MINVAL must be finite".to_owned());
            }
            Ok(raw.to_owned())
        })
        .transpose()?;
    let steady_cauchy_epsilon = optional_positive_decimal(&values, "CONV_CAUCHY_EPS")?;
    let steady_cauchy_elements = optional_u64(&values, "CONV_CAUCHY_ELEMS")?;
    if steady_cauchy_elements == Some(0) {
        return Err("SU2 config CONV_CAUCHY_ELEMS must be > 0".to_owned());
    }

    let window_enabled = parse_config_boolean(&values, "WINDOW_CAUCHY_CRIT")?;
    if window_enabled && !time_domain {
        return Err("WINDOW_CAUCHY_CRIT=YES requires TIME_DOMAIN=YES".to_owned());
    }
    let window = if window_enabled {
        let fields = parse_field_list(
            parse_config_value(&values, "CONV_WINDOW_FIELD")?,
            "CONV_WINDOW_FIELD",
        )?;
        let window_start_iter = required_u64(&values, "WINDOW_START_ITER")?;
        let convergence_start_iter = required_u64(&values, "CONV_WINDOW_STARTITER")?;
        let cauchy_elements = required_u64(&values, "CONV_WINDOW_CAUCHY_ELEMS")?;
        if cauchy_elements == 0 {
            return Err("SU2 config CONV_WINDOW_CAUCHY_ELEMS must be > 0".to_owned());
        }
        let cauchy_epsilon = required_positive_decimal(&values, "CONV_WINDOW_CAUCHY_EPS")?;
        let window_function = parse_config_value(&values, "WINDOW_FUNCTION")?
            .trim()
            .trim_matches('"')
            .to_ascii_uppercase();
        if !matches!(
            window_function.as_str(),
            "SQUARE" | "HANN" | "HANN_SQUARE" | "BUMP"
        ) {
            return Err(format!("unsupported SU2 WINDOW_FUNCTION {window_function}"));
        }
        if let Some(restart_iter) = optional_u64(&values, "RESTART_ITER")?
            && window_start_iter < restart_iter
        {
            return Err("WINDOW_START_ITER must not precede RESTART_ITER".to_owned());
        }
        Su2WindowConvergenceProtocol::Enabled {
            fields,
            window_start_iter,
            convergence_start_iter,
            cauchy_elements,
            cauchy_epsilon,
            window_function,
        }
    } else {
        Su2WindowConvergenceProtocol::Disabled
    };

    if time_domain && matches!(window, Su2WindowConvergenceProtocol::Disabled) {
        return Err(
            "TIME_DOMAIN=YES requires an explicit, supported WINDOW_CAUCHY_CRIT=YES protocol for this admission path"
                .to_owned(),
        );
    }
    let convergence_protocol = Su2ConvergenceProtocol {
        time_domain,
        steady_fields,
        steady_start_iteration,
        steady_residual_min_log10,
        steady_cauchy_epsilon,
        steady_cauchy_elements,
        window,
    };
    Ok(ParsedConfiguration {
        solver_family,
        convergence_protocol,
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
    bytes.push(u8::from(report.convergence_protocol.time_domain));
    append_u64(&mut bytes, report.convergence_protocol.steady_fields.len() as u64);
    for field in &report.convergence_protocol.steady_fields {
        append_string(&mut bytes, field);
    }
    append_optional_u64(&mut bytes, report.convergence_protocol.steady_start_iteration);
    append_optional_string(&mut bytes, &report.convergence_protocol.steady_residual_min_log10);
    append_optional_string(&mut bytes, &report.convergence_protocol.steady_cauchy_epsilon);
    append_optional_u64(&mut bytes, report.convergence_protocol.steady_cauchy_elements);
    match &report.convergence_protocol.window {
        Su2WindowConvergenceProtocol::Disabled => bytes.push(0),
        Su2WindowConvergenceProtocol::Enabled {
            fields,
            window_start_iter,
            convergence_start_iter,
            cauchy_elements,
            cauchy_epsilon,
            window_function,
        } => {
            bytes.push(1);
            append_u64(&mut bytes, fields.len() as u64);
            for field in fields {
                append_string(&mut bytes, field);
            }
            append_u64(&mut bytes, *window_start_iter);
            append_u64(&mut bytes, *convergence_start_iter);
            append_u64(&mut bytes, *cauchy_elements);
            append_string(&mut bytes, cauchy_epsilon);
            append_string(&mut bytes, window_function);
        }
    }
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

fn append_optional_u64(bytes: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(value) => {
            bytes.push(1);
            append_u64(bytes, value);
        }
        None => bytes.push(0),
    }
}

fn append_optional_string(bytes: &mut Vec<u8>, value: &Option<String>) {
    match value {
        Some(value) => {
            bytes.push(1);
            append_string(bytes, value);
        }
        None => bytes.push(0),
    }
}

fn format_optional_u64(value: Option<u64>) -> String {
    value.map(|number| number.to_string()).unwrap_or_default()
}

fn optional_string(value: &Option<String>) -> String {
    value.clone().unwrap_or_default()
}

fn window_fields(window: &Su2WindowConvergenceProtocol) -> String {
    match window {
        Su2WindowConvergenceProtocol::Disabled => String::new(),
        Su2WindowConvergenceProtocol::Enabled { fields, .. } => fields.join(";"),
    }
}

fn optional_window_u64(window: &Su2WindowConvergenceProtocol, selector: u8) -> String {
    match window {
        Su2WindowConvergenceProtocol::Disabled => String::new(),
        Su2WindowConvergenceProtocol::Enabled {
            window_start_iter,
            convergence_start_iter,
            cauchy_elements,
            ..
        } => match selector {
            0 => window_start_iter.to_string(),
            1 => convergence_start_iter.to_string(),
            2 => cauchy_elements.to_string(),
            _ => String::new(),
        },
    }
}

fn window_epsilon(window: &Su2WindowConvergenceProtocol) -> String {
    match window {
        Su2WindowConvergenceProtocol::Disabled => String::new(),
        Su2WindowConvergenceProtocol::Enabled { cauchy_epsilon, .. } => cauchy_epsilon.clone(),
    }
}

fn window_function(window: &Su2WindowConvergenceProtocol) -> String {
    match window {
        Su2WindowConvergenceProtocol::Disabled => String::new(),
        Su2WindowConvergenceProtocol::Enabled { window_function, .. } => window_function.clone(),
    }
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
    let mut convergence_fields = config.convergence_protocol.steady_fields.clone();
    if let Su2WindowConvergenceProtocol::Enabled { fields, .. } =
        &config.convergence_protocol.window
    {
        convergence_fields.extend(fields.iter().cloned());
    }
    let missing_convergence_fields = convergence_fields
        .iter()
        .filter(|field| !available.contains(field.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !missing_convergence_fields.is_empty() {
        return Err(format!(
            "configured SU2 convergence fields are unavailable in the exact solver/configuration dry-run inventory; SU2 may ignore unknown fields: {}",
            missing_convergence_fields.join(",")
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
        configured_convergence_field: config.convergence_protocol.steady_fields[0].clone(),
        convergence_protocol: config.convergence_protocol,
        history_output_selectors: config.history_output_selectors,
        stdout_sha256: receipt.stdout_sha256.clone(),
        stderr_sha256: receipt.stderr_sha256.clone(),
        available_history_fields,
        required_history_fields: required.into_iter().collect(),
        status: "all_declared_steady_and_window_convergence_fields_recognized".to_owned(),
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
        let configuration = format!(
            "SOLVER= NAVIER_STOKES\\nCONV_FIELD= {configured_field}\\nHISTORY_OUTPUT= (ITER, RMS_RES, AERO_COEFF)\\nTIME_DOMAIN= NO\nWINDOW_CAUCHY_CRIT= NO\\n"
        );
        fixture_run_with_config(root, &configuration, inventory)
    }

    fn fixture_run_with_config(root: &Path, configuration: &str, inventory: &str) -> PathBuf {
        fs::create_dir_all(root).expect("create fixture root");
        let config = root.join("config.cfg");
        let mesh = root.join("mesh.su2");
        fs::write(&config, configuration).expect("write config");
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
            "all_declared_steady_and_window_convergence_fields_recognized"
        );
        assert_eq!(
            report.convergence_protocol.window,
            Su2WindowConvergenceProtocol::Disabled
        );
        assert_eq!(report.available_history_fields.len(), 6);
        assert_eq!(report.required_history_fields.len(), 4);
        fs::remove_dir_all(root).expect("remove test fixture");
    }

    #[test]
    fn parses_su2_backslash_continued_history_output_lists() {
        let configuration = parse_configuration(
            b"SOLVER= NAVIER_STOKES\nCONV_FIELD= (RMS_DENSITY, CD)\nHISTORY_OUTPUT= (ITER, \\\n  RMS_RES, \\\n  AERO_COEFF)\nTIME_DOMAIN= NO\nWINDOW_CAUCHY_CRIT= NO\n",
        )
        .expect("valid continued configuration");
        assert_eq!(
            configuration.history_output_selectors,
            vec!["ITER", "RMS_RES", "AERO_COEFF"]
        );
        let incomplete = parse_configuration(
            b"SOLVER= NAVIER_STOKES\nCONV_FIELD= RMS_DENSITY\nHISTORY_OUTPUT= (ITER, \\\n",
        );
        assert!(
            incomplete
                .expect_err("unterminated continuation rejected")
                .contains("unterminated")
        );
    }

    #[test]
    fn dry_run_preflight_requires_every_steady_and_window_field_from_exact_inventory() {
        let root = root();
        let config = "SOLVER= NAVIER_STOKES\\nCONV_FIELD= (RMS_DENSITY, CD)\\nHISTORY_OUTPUT= (ITER, RMS_RES, AERO_COEFF)\\nTIME_DOMAIN= YES\nWINDOW_CAUCHY_CRIT= YES\\nCONV_WINDOW_FIELD= (TAVG_DRAG, TAVG_LIFT)\\nWINDOW_START_ITER= 500\\nCONV_WINDOW_STARTITER= 10\\nCONV_WINDOW_CAUCHY_ELEMS= 10\\nCONV_WINDOW_CAUCHY_EPS= 1E-4\\nWINDOW_FUNCTION= HANN_SQUARE\\n";
        let inventory = "Available screen/history output fields for the current configuration:\\nITER\\nRMS_DENSITY\\nCD\\nTAVG_DRAG\\nTAVG_LIFT\\nAvailable volume output fields for the current configuration:\\nPRESSURE\\n";
        let run = fixture_run_with_config(&root.join("complete"), config, inventory);
        let report = audit_su2_field_preflight(
            &run,
            "config.cfg",
            "mesh.su2",
            &[
                "RMS_DENSITY".to_owned(),
                "CD".to_owned(),
                "TAVG_DRAG".to_owned(),
                "TAVG_LIFT".to_owned(),
            ],
        )
        .expect("all configured convergence fields are discovered");
        assert_eq!(
            report.convergence_protocol.steady_fields,
            vec!["RMS_DENSITY", "CD"]
        );
        assert!(matches!(
            &report.convergence_protocol.window,
            Su2WindowConvergenceProtocol::Enabled { fields, .. }
                if fields == &vec!["TAVG_DRAG".to_owned(), "TAVG_LIFT".to_owned()]
        ));

        let missing_run = fixture_run_with_config(&root.join("missing"), config, INVENTORY);
        assert!(
            audit_su2_field_preflight(
                &missing_run,
                "config.cfg",
                "mesh.su2",
                &[
                    "RMS_DENSITY".to_owned(),
                    "CD".to_owned(),
                    "TAVG_DRAG".to_owned(),
                    "TAVG_LIFT".to_owned(),
                ],
            )
            .expect_err("a missing time-window field must reject preflight")
            .contains("TAVG_DRAG,TAVG_LIFT")
        );
        fs::remove_dir_all(root).expect("remove window convergence fixtures");
    }

    #[test]
    fn parses_multi_field_steady_and_time_window_convergence_protocols() {
        let configuration = parse_configuration(
            b"SOLVER= NAVIER_STOKES\nCONV_FIELD= (RMS_DENSITY, CD)\nHISTORY_OUTPUT= (ITER, RMS_RES, AERO_COEFF)\nCONV_STARTITER= 25\nCONV_RESIDUAL_MINVAL= -8\nCONV_CAUCHY_EPS= 1E-10\nCONV_CAUCHY_ELEMS= 100\nTIME_DOMAIN= YES\nWINDOW_CAUCHY_CRIT= YES\nCONV_WINDOW_FIELD= (TAVG_DRAG, TAVG_LIFT)\nWINDOW_START_ITER= 500\nCONV_WINDOW_STARTITER= 10\nCONV_WINDOW_CAUCHY_ELEMS= 10\nCONV_WINDOW_CAUCHY_EPS= 1E-4\nWINDOW_FUNCTION= HANN_SQUARE\nRESTART_ITER= 100\n",
        )
        .expect("well-formed multi-field steady and time-window protocol");
        assert_eq!(
            configuration.convergence_protocol.steady_fields,
            vec!["RMS_DENSITY", "CD"]
        );
        assert_eq!(
            configuration.convergence_protocol.steady_start_iteration,
            Some(25)
        );
        match configuration.convergence_protocol.window {
            Su2WindowConvergenceProtocol::Enabled {
                fields,
                window_start_iter,
                convergence_start_iter,
                cauchy_elements,
                cauchy_epsilon,
                window_function,
            } => {
                assert_eq!(fields, vec!["TAVG_DRAG", "TAVG_LIFT"]);
                assert_eq!(window_start_iter, 500);
                assert_eq!(convergence_start_iter, 10);
                assert_eq!(cauchy_elements, 10);
                assert_eq!(cauchy_epsilon, "1E-4");
                assert_eq!(window_function, "HANN_SQUARE");
            }
            Su2WindowConvergenceProtocol::Disabled => {
                panic!("window criterion should be enabled")
            }
        }

        let missing_window_fields = parse_configuration(
            b"SOLVER= NAVIER_STOKES\nCONV_FIELD= DRAG\nHISTORY_OUTPUT= (ITER, AERO_COEFF)\nTIME_DOMAIN= YES\nWINDOW_CAUCHY_CRIT= YES\n",
        );
        assert!(
            missing_window_fields
                .expect_err("enabled time convergence requires declared fields")
                .contains("CONV_WINDOW_FIELD")
        );

        let missing_window_inventory = parse_configuration(
            b"SOLVER= NAVIER_STOKES\nCONV_FIELD= DRAG\nHISTORY_OUTPUT= (ITER, AERO_COEFF)\nTIME_DOMAIN= YES\nWINDOW_CAUCHY_CRIT= YES\nCONV_WINDOW_FIELD= (TAVG_DRAG, TAVG_LIFT)\nWINDOW_START_ITER= 500\nCONV_WINDOW_STARTITER= 10\nCONV_WINDOW_CAUCHY_ELEMS= 10\nCONV_WINDOW_CAUCHY_EPS= 1E-4\nWINDOW_FUNCTION= SQUARE\n",
        )
        .expect("configuration parser preserves enabled window fields");
        assert_eq!(
            match missing_window_inventory.convergence_protocol.window {
                Su2WindowConvergenceProtocol::Enabled { fields, .. } => fields,
                Su2WindowConvergenceProtocol::Disabled => Vec::new(),
            },
            vec!["TAVG_DRAG", "TAVG_LIFT"]
        );

        let invalid_restart = parse_configuration(
            b"SOLVER= NAVIER_STOKES\nCONV_FIELD= DRAG\nHISTORY_OUTPUT= (ITER, AERO_COEFF)\nTIME_DOMAIN= YES\nWINDOW_CAUCHY_CRIT= YES\nCONV_WINDOW_FIELD= TAVG_DRAG\nWINDOW_START_ITER= 5\nCONV_WINDOW_STARTITER= 10\nCONV_WINDOW_CAUCHY_ELEMS= 10\nCONV_WINDOW_CAUCHY_EPS= 1E-4\nWINDOW_FUNCTION= SQUARE\nRESTART_ITER= 10\n",
        );
        assert!(
            invalid_restart
                .expect_err("window average cannot begin before restart")
                .contains("must not precede RESTART_ITER")
        );
    }

    #[test]
    fn rejects_unknown_convergence_field_and_missing_history_field() {
        let root = root();
        let unknown = fixture_run(&root.join("unknown"), "RMS_NOT_A_FIELD", INVENTORY);
        assert!(
            audit_su2_field_preflight(&unknown, "config.cfg", "mesh.su2", &["DRAG".to_owned()])
                .expect_err("unknown CONV_FIELD rejected")
                .contains("SU2 may ignore unknown fields")
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
