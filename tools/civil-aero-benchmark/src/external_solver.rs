//! Reproducible external-solver process execution with byte-bound inputs and outputs.
//!
//! This module is an orchestration receipt, not a CFD validator. It runs one explicitly pinned
//! executable with an argument vector in a newly created run directory, records failures, and
//! permits an independent replay of the recorded file hashes. It does not authenticate the
//! caller, attest the host kernel, validate solver physics, or prove experimental accuracy.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::integrity::{sha256_file, sha256_hex};

pub const EXTERNAL_SOLVER_RECEIPT_SCHEMA: &str = "civil-external-solver-run/0.1.0";
pub const INVOCATION_MANIFEST_FILE: &str = "invocation_manifest.txt";
pub const ENVIRONMENT_MANIFEST_FILE: &str = "environment_manifest.txt";
pub const STDOUT_LOG_FILE: &str = "stdout.log";
pub const STDERR_LOG_FILE: &str = "stderr.log";
pub const OUTPUT_MANIFEST_FILE: &str = "expected_outputs.csv";
pub const RUN_RECEIPT_FILE: &str = "runner_receipt.csv";
const RUN_RECEIPT_HEADER: &str = "schema,run_id,solver_id,solver_version_claim,executable_path,executable_sha256,invocation_manifest_sha256,environment_sha256,environment_manifest_sha256,spawn_error,stdout_byte_length,stdout_sha256,stderr_byte_length,stderr_sha256,exit_code,timed_out,output_limit_exceeded,elapsed_milliseconds,expected_outputs_present,expected_outputs_missing,output_manifest_sha256,status,receipt_sha256,evidence_level";
const OUTPUT_MANIFEST_HEADER: &str = "relative_path,status,byte_length,sha256";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedInputSpec {
    pub source_path: PathBuf,
    pub relative_path: String,
    pub expected_byte_length: u64,
    pub expected_sha256: String,
    pub source_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedInputReceipt {
    pub relative_path: String,
    pub byte_length: u64,
    pub sha256: String,
    pub source_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedOutputRecord {
    pub relative_path: String,
    pub present: bool,
    pub byte_length: Option<u64>,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExternalSolverRequest {
    pub run_id: String,
    pub solver_id: String,
    /// This is a caller-supplied version label. The executable digest is checked independently.
    pub solver_version_claim: String,
    pub executable_path: PathBuf,
    /// Must not exist. The runner creates it and stages inputs before launch.
    pub run_directory: PathBuf,
    /// Each element is passed as one argv element. No shell parsing or interpolation occurs.
    pub arguments: Vec<String>,
    /// Environment is cleared by default and only these explicit values are passed to the solver.
    /// Values are hashed in the invocation manifest and are not written in plaintext.
    pub environment: BTreeMap<String, String>,
    pub staged_inputs: Vec<StagedInputSpec>,
    /// Files that the solver must create relative to the fresh run directory.
    pub expected_outputs: Vec<String>,
    pub timeout: Duration,
    /// Per-stream cap. Capturing stops at this limit and the process is terminated.
    pub max_log_bytes_per_stream: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalSolverReceipt {
    pub schema: String,
    pub run_id: String,
    pub solver_id: String,
    pub solver_version_claim: String,
    pub executable_path: String,
    pub executable_sha256: String,
    pub invocation_manifest_sha256: String,
    pub environment_sha256: String,
    pub environment_manifest_sha256: String,
    pub spawn_error: Option<String>,
    pub staged_inputs: Vec<StagedInputReceipt>,
    pub stdout_byte_length: u64,
    pub stdout_sha256: String,
    pub stderr_byte_length: u64,
    pub stderr_sha256: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub output_limit_exceeded: bool,
    pub elapsed_milliseconds: u64,
    pub outputs: Vec<ExpectedOutputRecord>,
    pub output_manifest_sha256: String,
    pub status: String,
    pub receipt_sha256: String,
    pub evidence_level: String,
}

impl ExternalSolverReceipt {
    pub fn is_process_success(&self) -> bool {
        self.status == "process_exited_zero_expected_outputs_present"
    }

    pub fn missing_output_count(&self) -> usize {
        self.outputs.iter().filter(|output| !output.present).count()
    }
}

#[derive(Debug, Clone, Copy)]
struct StreamCapture {
    bytes_written: u64,
    exceeded_limit: bool,
}

fn validate_run_id(value: &str) -> Result<(), String> {
    if value.trim().is_empty()
        || value.contains('\n')
        || value.contains('\r')
        || value.contains('\t')
        || value.contains('|')
        || value.contains(',')
    {
        return Err("run_id must be non-empty and single-line without CSV delimiters".to_owned());
    }
    Ok(())
}

fn validate_relative_path(value: &str) -> Result<PathBuf, String> {
    if value.trim().is_empty() {
        return Err("relative path must not be empty".to_owned());
    }
    let path = Path::new(value);
    if path.is_absolute() {
        return Err(format!("path {value} must be relative"));
    }
    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(format!(
                "path {value} must contain only normal root-relative components"
            ));
        }
    }
    Ok(path.to_path_buf())
}

fn canonical_regular_file_under_root(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative_path = validate_relative_path(relative)?;
    let candidate = root.join(relative_path);
    let link_metadata = fs::symlink_metadata(&candidate)
        .map_err(|error| format!("cannot inspect run artifact {relative}: {error}"))?;
    if link_metadata.file_type().is_symlink() {
        return Err(format!("run artifact {relative} must not be a symlink"));
    }
    let canonical = fs::canonicalize(&candidate)
        .map_err(|error| format!("cannot resolve run artifact {relative}: {error}"))?;
    if !canonical.starts_with(root) {
        return Err(format!(
            "run artifact {relative} resolves outside run directory"
        ));
    }
    if !fs::metadata(&canonical)
        .map_err(|error| format!("cannot inspect run artifact {relative}: {error}"))?
        .is_file()
    {
        return Err(format!("run artifact {relative} is not a regular file"));
    }
    Ok(canonical)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn hex_encode(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(value.len() * 2);
    for byte in value.as_bytes() {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn append_u64(target: &mut Vec<u8>, value: u64) {
    target.extend_from_slice(&value.to_le_bytes());
}

fn append_string(target: &mut Vec<u8>, value: &str) {
    append_u64(target, value.len() as u64);
    target.extend_from_slice(value.as_bytes());
}

fn environment_manifest(
    environment: &BTreeMap<String, String>,
) -> Result<(String, Vec<u8>), String> {
    let mut canonical = Vec::new();
    let mut lines = String::from("schema=civil-external-solver-environment/0.1.0\n");
    append_u64(&mut canonical, environment.len() as u64);
    for (key, value) in environment {
        if key.trim().is_empty() || key.contains('=') || key.contains('\0') || value.contains('\0')
        {
            return Err("environment contains an invalid key or NUL value".to_owned());
        }
        append_string(&mut canonical, key);
        append_string(&mut canonical, value);
        let value_sha256 = sha256_hex(value.as_bytes())?;
        lines.push_str(&format!(
            "key_utf8_hex={} value_sha256={}\n",
            hex_encode(key),
            value_sha256
        ));
    }
    let digest = sha256_hex(&canonical)?;
    Ok((digest, lines.into_bytes()))
}

fn invocation_manifest(
    request: &ExternalSolverRequest,
    executable_path: &Path,
    executable_sha256: &str,
    environment_sha256: &str,
    staged_inputs: &[StagedInputReceipt],
) -> Vec<u8> {
    let mut output = String::new();
    output.push_str(&format!("schema={EXTERNAL_SOLVER_RECEIPT_SCHEMA}\n"));
    output.push_str(&format!(
        "run_id_utf8_hex={}\n",
        hex_encode(&request.run_id)
    ));
    output.push_str(&format!(
        "solver_id_utf8_hex={}\n",
        hex_encode(&request.solver_id)
    ));
    output.push_str(&format!(
        "solver_version_claim_utf8_hex={}\n",
        hex_encode(&request.solver_version_claim)
    ));
    output.push_str(&format!(
        "executable_path_utf8_hex={}\n",
        hex_encode(&executable_path.to_string_lossy())
    ));
    output.push_str(&format!("executable_sha256={executable_sha256}\n"));
    output.push_str(&format!("environment_sha256={environment_sha256}\n"));
    output.push_str(&format!("target_os={}\n", std::env::consts::OS));
    output.push_str(&format!("target_arch={}\n", std::env::consts::ARCH));
    output.push_str(&format!(
        "timeout_milliseconds={}\n",
        request.timeout.as_millis()
    ));
    output.push_str(&format!(
        "max_log_bytes_per_stream={}\n",
        request.max_log_bytes_per_stream
    ));
    output.push_str(&format!("argument_count={}\n", request.arguments.len()));
    for (index, argument) in request.arguments.iter().enumerate() {
        output.push_str(&format!(
            "argument_{index}_utf8_hex={}\n",
            hex_encode(argument)
        ));
    }
    output.push_str(&format!(
        "environment_entry_count={}\n",
        request.environment.len()
    ));
    output.push_str(&format!("staged_input_count={}\n", staged_inputs.len()));
    for (index, input) in staged_inputs.iter().enumerate() {
        output.push_str(&format!(
            "input_{index}_path_utf8_hex={}\ninput_{index}_byte_length={}\ninput_{index}_sha256={}\ninput_{index}_source_ref_utf8_hex={}\n",
            hex_encode(&input.relative_path),
            input.byte_length,
            input.sha256,
            hex_encode(&input.source_reference)
        ));
    }
    output.push_str(&format!(
        "expected_output_count={}\n",
        request.expected_outputs.len()
    ));
    for (index, output_path) in request.expected_outputs.iter().enumerate() {
        output.push_str(&format!(
            "expected_output_{index}_path_utf8_hex={}\n",
            hex_encode(output_path)
        ));
    }
    output.into_bytes()
}

fn stage_inputs(
    root: &Path,
    inputs: &[StagedInputSpec],
) -> Result<Vec<StagedInputReceipt>, String> {
    let mut staged = Vec::with_capacity(inputs.len());
    let mut seen_paths = BTreeSet::new();
    for input in inputs {
        let relative = validate_relative_path(&input.relative_path)?;
        if !seen_paths.insert(relative.clone()) {
            return Err(format!(
                "duplicate staged input path {}",
                input.relative_path
            ));
        }
        if input.source_reference.trim().is_empty() || !is_sha256(&input.expected_sha256) {
            return Err(format!(
                "staged input {} has empty source reference or malformed expected SHA-256",
                input.relative_path
            ));
        }
        let source = fs::canonicalize(&input.source_path).map_err(|error| {
            format!(
                "cannot resolve staged input source {}: {error}",
                input.source_path.display()
            )
        })?;
        let metadata = fs::metadata(&source).map_err(|error| {
            format!("cannot inspect staged input {}: {error}", source.display())
        })?;
        if !metadata.is_file() {
            return Err(format!(
                "staged input {} is not a regular file",
                source.display()
            ));
        }
        if metadata.len() != input.expected_byte_length {
            return Err(format!(
                "staged input {} byte length mismatch: expected {}, got {}",
                input.relative_path,
                input.expected_byte_length,
                metadata.len()
            ));
        }
        let source_sha256 = sha256_file(&source)?;
        if !source_sha256.eq_ignore_ascii_case(&input.expected_sha256) {
            return Err(format!(
                "staged input {} SHA-256 mismatch: expected {}, got {}",
                input.relative_path, input.expected_sha256, source_sha256
            ));
        }

        let destination = root.join(&relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "cannot create staged input parent {}: {error}",
                    parent.display()
                )
            })?;
        }
        let mut source_file = File::open(&source)
            .map_err(|error| format!("cannot open staged input {}: {error}", source.display()))?;
        let mut destination_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
            .map_err(|error| {
                format!(
                    "cannot create staged input {}: {error}",
                    destination.display()
                )
            })?;
        let copied = std::io::copy(&mut source_file, &mut destination_file)
            .map_err(|error| format!("cannot stage input {}: {error}", input.relative_path))?;
        destination_file.flush().map_err(|error| {
            format!("cannot flush staged input {}: {error}", input.relative_path)
        })?;
        if copied != input.expected_byte_length {
            return Err(format!(
                "staged input {} changed size while being copied",
                input.relative_path
            ));
        }
        let staged_sha256 = sha256_file(&destination)?;
        if !staged_sha256.eq_ignore_ascii_case(&input.expected_sha256) {
            return Err(format!(
                "staged input {} digest changed during copy",
                input.relative_path
            ));
        }
        staged.push(StagedInputReceipt {
            relative_path: input.relative_path.clone(),
            byte_length: copied,
            sha256: staged_sha256,
            source_reference: input.source_reference.clone(),
        });
    }
    Ok(staged)
}

fn capture_stream<R: Read>(
    mut input: R,
    mut output: File,
    maximum_bytes: u64,
    exceeded: Arc<AtomicBool>,
) -> Result<StreamCapture, String> {
    let mut buffer = [0_u8; 8192];
    let mut bytes_seen = 0_u64;
    let mut bytes_written = 0_u64;
    loop {
        let count = input
            .read(&mut buffer)
            .map_err(|error| format!("cannot capture solver output stream: {error}"))?;
        if count == 0 {
            break;
        }
        let count_u64 =
            u64::try_from(count).map_err(|_| "stream chunk length overflow".to_owned())?;
        bytes_seen = bytes_seen
            .checked_add(count_u64)
            .ok_or_else(|| "solver output byte count overflow".to_owned())?;
        let remaining = maximum_bytes.saturating_sub(bytes_written);
        let to_write = usize::try_from(remaining.min(count_u64))
            .map_err(|_| "capture byte limit does not fit usize".to_owned())?;
        if to_write > 0 {
            output
                .write_all(&buffer[..to_write])
                .map_err(|error| format!("cannot persist solver output stream: {error}"))?;
            bytes_written += to_write as u64;
        }
        if bytes_seen > maximum_bytes {
            exceeded.store(true, Ordering::Release);
        }
    }
    output
        .flush()
        .map_err(|error| format!("cannot flush solver output stream: {error}"))?;
    output
        .sync_all()
        .map_err(|error| format!("cannot sync solver output stream: {error}"))?;
    Ok(StreamCapture {
        bytes_written,
        exceeded_limit: bytes_seen > maximum_bytes,
    })
}

fn output_records(root: &Path, expected: &[String]) -> Result<Vec<ExpectedOutputRecord>, String> {
    let mut records = Vec::with_capacity(expected.len());
    for relative in expected {
        let path = validate_relative_path(relative)?;
        let candidate = root.join(&path);
        match fs::symlink_metadata(&candidate) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                records.push(ExpectedOutputRecord {
                    relative_path: relative.clone(),
                    present: false,
                    byte_length: None,
                    sha256: None,
                });
                continue;
            }
            Err(error) => {
                return Err(format!("cannot inspect solver output {relative}: {error}"));
            }
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!("solver output {relative} must not be a symlink"));
            }
            Ok(_) => {}
        }
        let canonical = canonical_regular_file_under_root(root, relative)?;
        let metadata = fs::metadata(&canonical)
            .map_err(|error| format!("cannot inspect solver output {relative}: {error}"))?;
        records.push(ExpectedOutputRecord {
            relative_path: relative.clone(),
            present: true,
            byte_length: Some(metadata.len()),
            sha256: Some(sha256_file(&canonical)?),
        });
    }
    Ok(records)
}

fn output_manifest_csv(outputs: &[ExpectedOutputRecord]) -> String {
    let mut content = String::from("relative_path,status,byte_length,sha256\n");
    for output in outputs {
        content.push_str(&format!(
            "{},{},{},{}\n",
            csv_escape(&output.relative_path),
            if output.present { "present" } else { "missing" },
            output
                .byte_length
                .map(|value| value.to_string())
                .unwrap_or_default(),
            output.sha256.as_deref().unwrap_or("")
        ));
    }
    content
}

fn status_label(
    spawn_error: Option<&str>,
    exit_code: Option<i32>,
    timed_out: bool,
    output_limit_exceeded: bool,
    outputs: &[ExpectedOutputRecord],
    executable_changed: bool,
) -> &'static str {
    if timed_out {
        "timed_out"
    } else if output_limit_exceeded {
        "output_limit_exceeded"
    } else if executable_changed {
        "executable_changed_during_run"
    } else if spawn_error.is_some() {
        "spawn_failed"
    } else if exit_code != Some(0) {
        "nonzero_exit"
    } else if outputs.iter().any(|output| !output.present) {
        "missing_expected_outputs"
    } else {
        "process_exited_zero_expected_outputs_present"
    }
}

fn canonical_receipt_bytes(receipt: &ExternalSolverReceipt) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        receipt.schema.as_str(),
        receipt.run_id.as_str(),
        receipt.solver_id.as_str(),
        receipt.solver_version_claim.as_str(),
        receipt.executable_path.as_str(),
        receipt.executable_sha256.as_str(),
        receipt.invocation_manifest_sha256.as_str(),
        receipt.environment_sha256.as_str(),
        receipt.stdout_sha256.as_str(),
        receipt.stderr_sha256.as_str(),
        receipt.status.as_str(),
        receipt.evidence_level.as_str(),
        receipt.output_manifest_sha256.as_str(),
    ] {
        append_string(&mut bytes, value);
    }
    append_u64(&mut bytes, receipt.stdout_byte_length);
    append_u64(&mut bytes, receipt.stderr_byte_length);
    match receipt.exit_code {
        Some(code) => {
            bytes.push(1);
            bytes.extend_from_slice(&code.to_le_bytes());
        }
        None => bytes.push(0),
    }
    bytes.push(u8::from(receipt.timed_out));
    bytes.push(u8::from(receipt.output_limit_exceeded));
    append_string(&mut bytes, receipt.spawn_error.as_deref().unwrap_or(""));
    append_string(&mut bytes, &receipt.environment_manifest_sha256);
    append_u64(&mut bytes, receipt.elapsed_milliseconds);
    append_u64(&mut bytes, receipt.staged_inputs.len() as u64);
    for input in &receipt.staged_inputs {
        append_string(&mut bytes, &input.relative_path);
        append_u64(&mut bytes, input.byte_length);
        append_string(&mut bytes, &input.sha256);
        append_string(&mut bytes, &input.source_reference);
    }
    append_u64(&mut bytes, receipt.outputs.len() as u64);
    for output in &receipt.outputs {
        append_string(&mut bytes, &output.relative_path);
        bytes.push(u8::from(output.present));
        match output.byte_length {
            Some(length) => {
                bytes.push(1);
                append_u64(&mut bytes, length);
            }
            None => bytes.push(0),
        }
        match &output.sha256 {
            Some(digest) => {
                bytes.push(1);
                append_string(&mut bytes, digest);
            }
            None => bytes.push(0),
        }
    }
    bytes
}

pub fn receipt_to_csv(receipt: &ExternalSolverReceipt) -> String {
    format!(
        "schema,run_id,solver_id,solver_version_claim,executable_path,executable_sha256,invocation_manifest_sha256,environment_sha256,environment_manifest_sha256,spawn_error,stdout_byte_length,stdout_sha256,stderr_byte_length,stderr_sha256,exit_code,timed_out,output_limit_exceeded,elapsed_milliseconds,expected_outputs_present,expected_outputs_missing,output_manifest_sha256,status,receipt_sha256,evidence_level\n{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
        csv_escape(&receipt.schema),
        csv_escape(&receipt.run_id),
        csv_escape(&receipt.solver_id),
        csv_escape(&receipt.solver_version_claim),
        csv_escape(&receipt.executable_path),
        receipt.executable_sha256,
        receipt.invocation_manifest_sha256,
        receipt.environment_sha256,
        receipt.environment_manifest_sha256,
        csv_escape(receipt.spawn_error.as_deref().unwrap_or("")),
        receipt.stdout_byte_length,
        receipt.stdout_sha256,
        receipt.stderr_byte_length,
        receipt.stderr_sha256,
        receipt
            .exit_code
            .map(|value| value.to_string())
            .unwrap_or_default(),
        receipt.timed_out,
        receipt.output_limit_exceeded,
        receipt.elapsed_milliseconds,
        receipt
            .outputs
            .iter()
            .filter(|output| output.present)
            .count(),
        receipt.missing_output_count(),
        receipt.output_manifest_sha256,
        receipt.status,
        receipt.receipt_sha256,
        receipt.evidence_level
    )
}

pub fn execute_external_solver(
    request: &ExternalSolverRequest,
) -> Result<ExternalSolverReceipt, String> {
    validate_run_id(&request.run_id)?;
    if request.solver_id.trim().is_empty() || request.solver_version_claim.trim().is_empty() {
        return Err("solver_id and solver_version_claim must be non-empty".to_owned());
    }
    if request.timeout.is_zero() || request.max_log_bytes_per_stream == 0 {
        return Err("timeout and per-stream log cap must be positive".to_owned());
    }
    if request.expected_outputs.is_empty() {
        return Err("at least one expected output is required".to_owned());
    }
    if request
        .arguments
        .iter()
        .any(|argument| argument.contains('\0'))
    {
        return Err("solver arguments must not contain NUL bytes".to_owned());
    }
    let mut used_paths = BTreeSet::new();
    for path in [
        INVOCATION_MANIFEST_FILE,
        ENVIRONMENT_MANIFEST_FILE,
        STDOUT_LOG_FILE,
        STDERR_LOG_FILE,
        OUTPUT_MANIFEST_FILE,
        RUN_RECEIPT_FILE,
    ] {
        used_paths.insert(PathBuf::from(path));
    }
    for input in &request.staged_inputs {
        let relative = validate_relative_path(&input.relative_path)?;
        if !used_paths.insert(relative.clone()) {
            return Err(format!(
                "staged input path {} collides with a reserved/used path",
                input.relative_path
            ));
        }
    }
    for output in &request.expected_outputs {
        let relative = validate_relative_path(output)?;
        if !used_paths.insert(relative.clone()) {
            return Err(format!(
                "expected output path {output} collides with a reserved/used path or staged input"
            ));
        }
    }
    let declared_paths = used_paths.iter().collect::<Vec<_>>();
    for left_index in 0..declared_paths.len() {
        for right_index in (left_index + 1)..declared_paths.len() {
            let left = declared_paths[left_index];
            let right = declared_paths[right_index];
            if left.starts_with(right) || right.starts_with(left) {
                return Err(format!(
                    "declared artifact paths {} and {} have a file/directory prefix collision",
                    left.display(),
                    right.display()
                ));
            }
        }
    }

    let executable = fs::canonicalize(&request.executable_path).map_err(|error| {
        format!(
            "cannot resolve solver executable {}: {error}",
            request.executable_path.display()
        )
    })?;
    if !fs::metadata(&executable)
        .map_err(|error| format!("cannot inspect solver executable: {error}"))?
        .is_file()
    {
        return Err("solver executable must resolve to a regular file".to_owned());
    }
    let executable_sha256 = sha256_file(&executable)?;
    let (environment_sha256, environment_manifest) = environment_manifest(&request.environment)?;
    let environment_manifest_sha256 = sha256_hex(&environment_manifest)?;

    // A run directory must be new: this is the primary protection against stale-output reuse.
    fs::create_dir(&request.run_directory).map_err(|error| {
        format!(
            "cannot create fresh run directory {} (it must not already exist): {error}",
            request.run_directory.display()
        )
    })?;
    let root = fs::canonicalize(&request.run_directory)
        .map_err(|error| format!("cannot resolve fresh run directory: {error}"))?;

    let staged_inputs = stage_inputs(&root, &request.staged_inputs)?;
    fs::write(root.join(ENVIRONMENT_MANIFEST_FILE), &environment_manifest)
        .map_err(|error| format!("cannot write environment manifest: {error}"))?;
    let invocation_bytes = invocation_manifest(
        request,
        &executable,
        &executable_sha256,
        &environment_sha256,
        &staged_inputs,
    );
    fs::write(root.join(INVOCATION_MANIFEST_FILE), &invocation_bytes)
        .map_err(|error| format!("cannot write invocation manifest: {error}"))?;
    let invocation_manifest_sha256 = sha256_hex(&invocation_bytes)?;

    let stdout_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(STDOUT_LOG_FILE))
        .map_err(|error| format!("cannot create stdout log: {error}"))?;
    let stderr_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(STDERR_LOG_FILE))
        .map_err(|error| format!("cannot create stderr log: {error}"))?;

    let exceeded = Arc::new(AtomicBool::new(false));
    let start = Instant::now();
    let mut timed_out = false;
    let mut spawn_error = None;
    let mut exit_code = None;
    let empty_capture = StreamCapture {
        bytes_written: 0,
        exceeded_limit: false,
    };
    let mut stdout_capture = empty_capture;
    let mut stderr_capture = empty_capture;

    let mut command = Command::new(&executable);
    command
        .args(&request.arguments)
        .current_dir(&root)
        .env_clear()
        .envs(&request.environment)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    match command.spawn() {
        Err(error) => {
            spawn_error = Some(error.to_string());
            drop(stdout_file);
            drop(stderr_file);
        }
        Ok(mut child) => {
            let max_log_bytes_per_stream = request.max_log_bytes_per_stream;
            let stdout = child
                .stdout
                .take()
                .ok_or_else(|| "solver stdout pipe missing".to_owned())?;
            let stderr = child
                .stderr
                .take()
                .ok_or_else(|| "solver stderr pipe missing".to_owned())?;
            let stdout_exceeded = Arc::clone(&exceeded);
            let stdout_thread = thread::spawn(move || {
                capture_stream(
                    stdout,
                    stdout_file,
                    max_log_bytes_per_stream,
                    stdout_exceeded,
                )
            });
            let stderr_exceeded = Arc::clone(&exceeded);
            let stderr_thread = thread::spawn(move || {
                capture_stream(
                    stderr,
                    stderr_file,
                    max_log_bytes_per_stream,
                    stderr_exceeded,
                )
            });

            let process_status = loop {
                if exceeded.load(Ordering::Acquire) {
                    let _ = child.kill();
                    break child
                        .wait()
                        .map_err(|error| format!("cannot wait after output cap: {error}"))?;
                }
                match child.try_wait() {
                    Ok(Some(status)) => break status,
                    Ok(None) => {}
                    Err(error) => {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(format!("cannot inspect solver process status: {error}"));
                    }
                }
                if start.elapsed() >= request.timeout {
                    timed_out = true;
                    let _ = child.kill();
                    break child.wait().map_err(|error| {
                        format!("cannot wait after solver timeout: {error}")
                    })?;
                }
                thread::sleep(Duration::from_millis(10));
            };
            exit_code = process_status.code();
            stdout_capture = stdout_thread
                .join()
                .map_err(|_| "stdout capture thread panicked".to_owned())??;
            stderr_capture = stderr_thread
                .join()
                .map_err(|_| "stderr capture thread panicked".to_owned())??;
        }
    }

    let elapsed_milliseconds = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    let stdout_path = root.join(STDOUT_LOG_FILE);
    let stderr_path = root.join(STDERR_LOG_FILE);
    let stdout_sha256 = sha256_file(&stdout_path)?;
    let stderr_sha256 = sha256_file(&stderr_path)?;
    let executable_after_sha256 = sha256_file(&executable)?;
    let executable_changed = executable_after_sha256 != executable_sha256;
    let output_limit_exceeded = stdout_capture.exceeded_limit
        || stderr_capture.exceeded_limit
        || exceeded.load(Ordering::Acquire);
    let outputs = output_records(&root, &request.expected_outputs)?;
    let output_manifest_content = output_manifest_csv(&outputs);
    fs::write(
        root.join(OUTPUT_MANIFEST_FILE),
        output_manifest_content.as_bytes(),
    )
    .map_err(|error| format!("cannot write expected-output manifest: {error}"))?;
    let output_manifest_sha256 = sha256_hex(output_manifest_content.as_bytes())?;
    let status = status_label(
        spawn_error.as_deref(),
        exit_code,
        timed_out,
        output_limit_exceeded,
        &outputs,
        executable_changed,
    )
    .to_owned();

    let mut receipt = ExternalSolverReceipt {
        schema: EXTERNAL_SOLVER_RECEIPT_SCHEMA.to_owned(),
        run_id: request.run_id.clone(),
        solver_id: request.solver_id.clone(),
        solver_version_claim: request.solver_version_claim.clone(),
        executable_path: executable.to_string_lossy().into_owned(),
        executable_sha256,
        invocation_manifest_sha256,
        environment_sha256,
        environment_manifest_sha256,
        spawn_error,
        staged_inputs,
        stdout_byte_length: stdout_capture.bytes_written,
        stdout_sha256,
        stderr_byte_length: stderr_capture.bytes_written,
        stderr_sha256,
        exit_code,
        timed_out,
        output_limit_exceeded,
        elapsed_milliseconds,
        outputs,
        output_manifest_sha256,
        status,
        receipt_sha256: String::new(),
        evidence_level: "process_receipt_only_caller_version_claim_unverified_no_convergence_or_physical_validation".to_owned(),
    };
    receipt.receipt_sha256 = sha256_hex(&canonical_receipt_bytes(&receipt))?;
    fs::write(root.join(RUN_RECEIPT_FILE), receipt_to_csv(&receipt))
        .map_err(|error| format!("cannot persist external solver receipt: {error}"))?;
    Ok(receipt)
}

fn parse_csv_record(line: &str) -> Result<Vec<String>, String> {
    let characters = line.chars().collect::<Vec<_>>();
    let mut fields = Vec::new();
    let mut index = 0;
    loop {
        let mut field = String::new();
        if characters.get(index) == Some(&'"') {
            index += 1;
            let mut closed = false;
            while index < characters.len() {
                if characters[index] == '"' {
                    if characters.get(index + 1) == Some(&'"') {
                        field.push('"');
                        index += 2;
                    } else {
                        index += 1;
                        closed = true;
                        break;
                    }
                } else {
                    field.push(characters[index]);
                    index += 1;
                }
            }
            if !closed {
                return Err("CSV contains an unterminated quoted field".to_owned());
            }
            if index < characters.len() && characters[index] != ',' {
                return Err("CSV contains data after a closing quote".to_owned());
            }
        } else {
            while index < characters.len() && characters[index] != ',' {
                if characters[index] == '"' {
                    return Err("CSV contains a quote inside an unquoted field".to_owned());
                }
                field.push(characters[index]);
                index += 1;
            }
        }
        fields.push(field);
        if index == characters.len() {
            break;
        }
        if characters[index] != ',' {
            return Err("CSV field delimiter is malformed".to_owned());
        }
        index += 1;
        if index == characters.len() {
            fields.push(String::new());
            break;
        }
    }
    Ok(fields)
}

fn decode_hex_utf8(value: &str, field: &str) -> Result<String, String> {
    if !value.len().is_multiple_of(2) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "{field} must be an even-length hexadecimal UTF-8 value"
        ));
    }
    let mut bytes = Vec::with_capacity(value.len() / 2);
    for pair in value.as_bytes().chunks_exact(2) {
        let digit = |byte: u8| -> u8 {
            match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => 0,
            }
        };
        bytes.push((digit(pair[0]) << 4) | digit(pair[1]));
    }
    String::from_utf8(bytes).map_err(|_| format!("{field} is not valid UTF-8"))
}

fn parse_key_value_manifest(input: &str) -> Result<BTreeMap<String, String>, String> {
    let mut values = BTreeMap::new();
    for (index, line) in input.lines().enumerate() {
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("manifest line {} is not key=value", index + 1))?;
        if key.trim().is_empty() || values.insert(key.to_owned(), value.to_owned()).is_some() {
            return Err(format!(
                "manifest line {} has an empty or duplicate key",
                index + 1
            ));
        }
    }
    Ok(values)
}

fn manifest_value<'a>(values: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, String> {
    values
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("invocation manifest is missing {key}"))
}

fn parse_manifest_u64(values: &BTreeMap<String, String>, key: &str) -> Result<u64, String> {
    manifest_value(values, key)?
        .parse::<u64>()
        .map_err(|error| format!("invocation manifest {key} is not an unsigned integer: {error}"))
}

fn parse_manifest_count(values: &BTreeMap<String, String>, key: &str) -> Result<usize, String> {
    usize::try_from(parse_manifest_u64(values, key)?)
        .map_err(|_| format!("invocation manifest {key} does not fit usize"))
}

fn parse_manifest_string(values: &BTreeMap<String, String>, key: &str) -> Result<String, String> {
    decode_hex_utf8(manifest_value(values, key)?, key)
}

fn parse_expected_output_manifest(input: &str) -> Result<Vec<ExpectedOutputRecord>, String> {
    let mut lines = input.lines();
    let header = lines
        .next()
        .ok_or_else(|| "expected-output manifest is empty".to_owned())?;
    if header != OUTPUT_MANIFEST_HEADER {
        return Err("expected-output manifest header does not match schema".to_owned());
    }
    let mut outputs = Vec::new();
    for (index, line) in lines.enumerate() {
        let fields = parse_csv_record(line)
            .map_err(|error| format!("expected-output manifest row {}: {error}", index + 2))?;
        if fields.len() != 4 {
            return Err(format!(
                "expected-output manifest row {} must have four columns",
                index + 2
            ));
        }
        validate_relative_path(&fields[0])?;
        let (present, byte_length, sha256) = match fields[1].as_str() {
            "present" => {
                let length = fields[2].parse::<u64>().map_err(|error| {
                    format!(
                        "expected-output manifest row {} has invalid byte length: {error}",
                        index + 2
                    )
                })?;
                if !is_sha256(&fields[3]) {
                    return Err(format!(
                        "expected-output manifest row {} has malformed SHA-256",
                        index + 2
                    ));
                }
                (true, Some(length), Some(fields[3].to_ascii_lowercase()))
            }
            "missing" if fields[2].is_empty() && fields[3].is_empty() => (false, None, None),
            _ => {
                return Err(format!(
                    "expected-output manifest row {} has inconsistent presence fields",
                    index + 2
                ));
            }
        };
        outputs.push(ExpectedOutputRecord {
            relative_path: fields[0].clone(),
            present,
            byte_length,
            sha256,
        });
    }
    if outputs.is_empty() {
        return Err(
            "expected-output manifest must contain at least one declared output".to_owned(),
        );
    }
    Ok(outputs)
}

/// Load a completed run's receipt and records from disk, then verify the full bundle. This is the
/// offline entry point for a verifier that did not run the solver.
pub fn read_external_solver_receipt(run_directory: &Path) -> Result<ExternalSolverReceipt, String> {
    let root = fs::canonicalize(run_directory)
        .map_err(|error| format!("cannot resolve external solver run directory: {error}"))?;
    let receipt_path = canonical_regular_file_under_root(&root, RUN_RECEIPT_FILE)?;
    let receipt_text = fs::read_to_string(receipt_path)
        .map_err(|error| format!("cannot read external solver receipt: {error}"))?;
    let receipt_lines = receipt_text.lines().collect::<Vec<_>>();
    if receipt_lines.len() != 2 {
        return Err("runner receipt must contain exactly one header and one data row".to_owned());
    }
    let header = parse_csv_record(receipt_lines[0])?;
    if header != parse_csv_record(RUN_RECEIPT_HEADER)? {
        return Err("runner receipt header does not match schema".to_owned());
    }
    let fields = parse_csv_record(receipt_lines[1])?;
    if fields.len() != 24 {
        return Err(format!(
            "runner receipt must have 24 columns, got {}",
            fields.len()
        ));
    }

    let parse_u64_field = |index: usize, name: &str| -> Result<u64, String> {
        fields[index]
            .parse::<u64>()
            .map_err(|error| format!("runner receipt {name} is not an unsigned integer: {error}"))
    };
    let parse_bool_field = |index: usize, name: &str| -> Result<bool, String> {
        match fields[index].as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(format!("runner receipt {name} must be true or false")),
        }
    };
    let parse_digest_field = |index: usize, name: &str| -> Result<String, String> {
        if !is_sha256(&fields[index]) {
            return Err(format!("runner receipt {name} is not a SHA-256 digest"));
        }
        Ok(fields[index].to_ascii_lowercase())
    };

    let invocation_path = canonical_regular_file_under_root(&root, INVOCATION_MANIFEST_FILE)?;
    let invocation_text = fs::read_to_string(invocation_path)
        .map_err(|error| format!("cannot read invocation manifest: {error}"))?;
    let invocation_digest = sha256_hex(invocation_text.as_bytes())?;
    let invocation = parse_key_value_manifest(&invocation_text)?;
    if manifest_value(&invocation, "schema")? != EXTERNAL_SOLVER_RECEIPT_SCHEMA {
        return Err("invocation manifest schema does not match".to_owned());
    }
    let run_id = parse_manifest_string(&invocation, "run_id_utf8_hex")?;
    let solver_id = parse_manifest_string(&invocation, "solver_id_utf8_hex")?;
    let solver_version_claim = parse_manifest_string(&invocation, "solver_version_claim_utf8_hex")?;
    let executable_path = parse_manifest_string(&invocation, "executable_path_utf8_hex")?;
    let executable_sha256 = manifest_value(&invocation, "executable_sha256")?.to_ascii_lowercase();
    let environment_sha256 =
        manifest_value(&invocation, "environment_sha256")?.to_ascii_lowercase();
    let argument_count = parse_manifest_count(&invocation, "argument_count")?;
    let environment_count = parse_manifest_count(&invocation, "environment_entry_count")?;
    let staged_count = parse_manifest_count(&invocation, "staged_input_count")?;
    let expected_count = parse_manifest_count(&invocation, "expected_output_count")?;
    if argument_count > 100_000
        || environment_count > 100_000
        || staged_count > 10_000
        || expected_count > 10_000
    {
        return Err("invocation manifest declares an unreasonable record count".to_owned());
    }
    let expected_manifest_keys = 15_usize
        .checked_add(argument_count)
        .and_then(|count| count.checked_add(staged_count.checked_mul(4)?))
        .and_then(|count| count.checked_add(expected_count))
        .ok_or_else(|| "invocation manifest key count overflow".to_owned())?;
    if invocation.len() != expected_manifest_keys {
        return Err("invocation manifest contains missing or unrecognized fields".to_owned());
    }
    if parse_manifest_u64(&invocation, "timeout_milliseconds")? == 0
        || parse_manifest_u64(&invocation, "max_log_bytes_per_stream")? == 0
    {
        return Err("invocation manifest has invalid resource limits".to_owned());
    }
    for index in 0..argument_count {
        let argument = parse_manifest_string(&invocation, &format!("argument_{index}_utf8_hex"))?;
        if argument.contains('\0') {
            return Err("invocation manifest contains an argument with NUL".to_owned());
        }
    }

    let mut staged_inputs = Vec::with_capacity(staged_count);
    let mut seen_input_paths = BTreeSet::new();
    for index in 0..staged_count {
        let relative_path =
            parse_manifest_string(&invocation, &format!("input_{index}_path_utf8_hex"))?;
        validate_relative_path(&relative_path)?;
        if !seen_input_paths.insert(relative_path.clone()) {
            return Err("invocation manifest contains duplicate staged-input paths".to_owned());
        }
        let byte_length = parse_manifest_u64(&invocation, &format!("input_{index}_byte_length"))?;
        let sha256 =
            manifest_value(&invocation, &format!("input_{index}_sha256"))?.to_ascii_lowercase();
        if !is_sha256(&sha256) {
            return Err(format!(
                "invocation manifest input {index} has malformed SHA-256"
            ));
        }
        let source_reference =
            parse_manifest_string(&invocation, &format!("input_{index}_source_ref_utf8_hex"))?;
        staged_inputs.push(StagedInputReceipt {
            relative_path,
            byte_length,
            sha256,
            source_reference,
        });
    }

    let mut expected_paths = Vec::with_capacity(expected_count);
    let mut seen_output_paths = BTreeSet::new();
    for index in 0..expected_count {
        let path = parse_manifest_string(
            &invocation,
            &format!("expected_output_{index}_path_utf8_hex"),
        )?;
        validate_relative_path(&path)?;
        if !seen_output_paths.insert(path.clone()) {
            return Err("invocation manifest contains duplicate expected-output paths".to_owned());
        }
        expected_paths.push(path);
    }
    let environment_path = canonical_regular_file_under_root(&root, ENVIRONMENT_MANIFEST_FILE)?;
    let environment_text = fs::read_to_string(environment_path)
        .map_err(|error| format!("cannot read environment manifest: {error}"))?;
    let environment_lines = environment_text.lines().collect::<Vec<_>>();
    if environment_lines.first().copied() != Some("schema=civil-external-solver-environment/0.1.0")
        || environment_lines.len() != environment_count + 1
    {
        return Err(
            "environment manifest entry count or schema differs from invocation".to_owned(),
        );
    }
    let mut seen_environment_keys = BTreeSet::new();
    for (index, line) in environment_lines.iter().skip(1).enumerate() {
        let (key_hex, value_sha256) = line
            .strip_prefix("key_utf8_hex=")
            .and_then(|rest| rest.split_once(" value_sha256="))
            .ok_or_else(|| format!("environment manifest entry {} is malformed", index + 1))?;
        let key = decode_hex_utf8(key_hex, "environment key")?;
        if key.trim().is_empty()
            || key.contains('=')
            || !seen_environment_keys.insert(key)
            || !is_sha256(value_sha256)
        {
            return Err(format!(
                "environment manifest entry {} is duplicate or malformed",
                index + 1
            ));
        }
    }

    let output_path = canonical_regular_file_under_root(&root, OUTPUT_MANIFEST_FILE)?;
    let output_text = fs::read_to_string(output_path)
        .map_err(|error| format!("cannot read expected-output manifest: {error}"))?;
    let outputs = parse_expected_output_manifest(&output_text)?;
    if outputs
        .iter()
        .map(|output| output.relative_path.as_str())
        .collect::<Vec<_>>()
        != expected_paths
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    {
        return Err(
            "expected-output manifest paths differ from frozen invocation paths".to_owned(),
        );
    }

    let receipt = ExternalSolverReceipt {
        schema: fields[0].clone(),
        run_id: fields[1].clone(),
        solver_id: fields[2].clone(),
        solver_version_claim: fields[3].clone(),
        executable_path: fields[4].clone(),
        executable_sha256: parse_digest_field(5, "executable_sha256")?,
        invocation_manifest_sha256: parse_digest_field(6, "invocation_manifest_sha256")?,
        environment_sha256: parse_digest_field(7, "environment_sha256")?,
        environment_manifest_sha256: parse_digest_field(8, "environment_manifest_sha256")?,
        spawn_error: if fields[9].is_empty() {
            None
        } else {
            Some(fields[9].clone())
        },
        stdout_byte_length: parse_u64_field(10, "stdout_byte_length")?,
        stdout_sha256: parse_digest_field(11, "stdout_sha256")?,
        stderr_byte_length: parse_u64_field(12, "stderr_byte_length")?,
        stderr_sha256: parse_digest_field(13, "stderr_sha256")?,
        exit_code: if fields[14].is_empty() {
            None
        } else {
            Some(
                fields[14]
                    .parse::<i32>()
                    .map_err(|error| format!("runner receipt exit_code is invalid: {error}"))?,
            )
        },
        timed_out: parse_bool_field(15, "timed_out")?,
        output_limit_exceeded: parse_bool_field(16, "output_limit_exceeded")?,
        elapsed_milliseconds: parse_u64_field(17, "elapsed_milliseconds")?,
        outputs,
        output_manifest_sha256: parse_digest_field(20, "output_manifest_sha256")?,
        status: fields[21].clone(),
        receipt_sha256: parse_digest_field(22, "receipt_sha256")?,
        evidence_level: fields[23].clone(),
        staged_inputs,
    };
    if receipt.run_id != run_id
        || receipt.solver_id != solver_id
        || receipt.solver_version_claim != solver_version_claim
        || receipt.executable_path != executable_path
        || receipt.executable_sha256 != executable_sha256
        || receipt.environment_sha256 != environment_sha256
        || receipt.invocation_manifest_sha256 != invocation_digest
    {
        return Err("receipt identity fields disagree with invocation manifest".to_owned());
    }
    if !is_sha256(&environment_sha256) {
        return Err("invocation manifest environment digest is malformed".to_owned());
    }
    let present = receipt
        .outputs
        .iter()
        .filter(|output| output.present)
        .count();
    let missing = receipt.missing_output_count();
    if fields[18] != present.to_string() || fields[19] != missing.to_string() {
        return Err("receipt output counts disagree with output manifest".to_owned());
    }
    if receipt.evidence_level
        != "process_receipt_only_caller_version_claim_unverified_no_convergence_or_physical_validation"
    {
        return Err("runner receipt has an unknown evidence ceiling".to_owned());
    }
    verify_external_solver_receipt(&root, &receipt)?;
    Ok(receipt)
}

/// Recompute the receipt digest and verify every persisted byte. This verifies file integrity
/// against a supplied receipt; it does not authenticate who created that receipt.
pub fn verify_external_solver_receipt(
    run_directory: &Path,
    receipt: &ExternalSolverReceipt,
) -> Result<(), String> {
    if receipt.schema != EXTERNAL_SOLVER_RECEIPT_SCHEMA {
        return Err("unsupported external solver receipt schema".to_owned());
    }
    let root = fs::canonicalize(run_directory)
        .map_err(|error| format!("cannot resolve external solver run directory: {error}"))?;
    if !root.is_dir() {
        return Err("external solver run directory must be a directory".to_owned());
    }
    let invocation_path = canonical_regular_file_under_root(&root, INVOCATION_MANIFEST_FILE)?;
    if sha256_file(&invocation_path)? != receipt.invocation_manifest_sha256 {
        return Err("invocation manifest SHA-256 differs from receipt".to_owned());
    }
    let environment_path = canonical_regular_file_under_root(&root, ENVIRONMENT_MANIFEST_FILE)?;
    if sha256_file(&environment_path)? != receipt.environment_manifest_sha256 {
        return Err("environment manifest SHA-256 differs from receipt".to_owned());
    }
    for input in &receipt.staged_inputs {
        let canonical = canonical_regular_file_under_root(&root, &input.relative_path)?;
        let metadata = fs::metadata(&canonical).map_err(|error| {
            format!(
                "cannot inspect staged input {}: {error}",
                input.relative_path
            )
        })?;
        if !metadata.is_file()
            || metadata.len() != input.byte_length
            || sha256_file(&canonical)? != input.sha256
        {
            return Err(format!(
                "staged input {} differs from receipt",
                input.relative_path
            ));
        }
    }
    for (path, expected, expected_length) in [
        (
            STDOUT_LOG_FILE,
            receipt.stdout_sha256.as_str(),
            receipt.stdout_byte_length,
        ),
        (
            STDERR_LOG_FILE,
            receipt.stderr_sha256.as_str(),
            receipt.stderr_byte_length,
        ),
    ] {
        let file = canonical_regular_file_under_root(&root, path)?;
        let metadata = fs::metadata(&file)
            .map_err(|error| format!("cannot inspect persisted {path}: {error}"))?;
        if metadata.len() != expected_length || sha256_file(&file)? != expected {
            return Err(format!("persisted {path} differs from receipt"));
        }
    }
    let output_manifest_path = canonical_regular_file_under_root(&root, OUTPUT_MANIFEST_FILE)?;
    let output_manifest_bytes = fs::read(&output_manifest_path)
        .map_err(|error| format!("cannot read expected-output manifest: {error}"))?;
    if sha256_hex(&output_manifest_bytes)? != receipt.output_manifest_sha256 {
        return Err("expected-output manifest SHA-256 differs from receipt".to_owned());
    }
    if output_manifest_bytes != output_manifest_csv(&receipt.outputs).as_bytes() {
        return Err("expected-output manifest content differs from receipt records".to_owned());
    }
    let actual_outputs = output_records(
        &root,
        &receipt
            .outputs
            .iter()
            .map(|row| row.relative_path.clone())
            .collect::<Vec<_>>(),
    )?;
    if actual_outputs != receipt.outputs {
        return Err("one or more expected solver outputs differ from receipt".to_owned());
    }
    let receipt_path = canonical_regular_file_under_root(&root, RUN_RECEIPT_FILE)?;
    let persisted_receipt = fs::read_to_string(receipt_path)
        .map_err(|error| format!("cannot read persisted runner receipt: {error}"))?;
    if persisted_receipt != receipt_to_csv(receipt) {
        return Err("persisted runner receipt differs from supplied receipt".to_owned());
    }
    let expected_receipt = sha256_hex(&canonical_receipt_bytes(receipt))?;
    if expected_receipt != receipt.receipt_sha256 {
        return Err("external solver receipt digest does not recompute".to_owned());
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn temporary_root(label: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "civil-aero-external-solver-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create test root");
        path
    }

    fn fake_solver(root: &Path, body: &str) -> PathBuf {
        let path = root.join("fake-solver.sh");
        let script = format!("#!/bin/sh\nset -eu\n{body}\n");
        fs::write(&path, script).expect("write fake solver");
        let mut permissions = fs::metadata(&path).expect("script metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).expect("make fake solver executable");
        path
    }

    fn request(
        root: &Path,
        executable: PathBuf,
        run_name: &str,
        arguments: Vec<String>,
        outputs: Vec<String>,
    ) -> ExternalSolverRequest {
        ExternalSolverRequest {
            run_id: run_name.to_owned(),
            solver_id: "fake-solver".to_owned(),
            solver_version_claim: "fixture-v0".to_owned(),
            executable_path: executable,
            run_directory: root.join(run_name),
            arguments,
            environment: BTreeMap::new(),
            staged_inputs: Vec::new(),
            expected_outputs: if outputs.is_empty() {
                vec!["expected.dat".to_owned()]
            } else {
                outputs
            },
            timeout: Duration::from_secs(2),
            max_log_bytes_per_stream: 16 * 1024,
        }
    }

    #[test]
    fn argv_is_passed_literally_inputs_are_staged_and_receipt_replays() {
        let root = temporary_root("positive");
        let executable = fake_solver(
            &root,
            "printf '%s\\n' \"$@\" > args.txt\nprintf 'history\\n' > history.csv\nprintf 'solver started\\n'",
        );
        let input_path = root.join("case.cfg");
        fs::write(&input_path, "CONFIG=PINNED\n").expect("write pinned input");
        let input_bytes = fs::read(&input_path).expect("read pinned input");
        let input_sha = sha256_hex(&input_bytes).expect("hash pinned input");
        let mut spec = request(
            &root,
            executable,
            "positive-run",
            vec![
                "argument with spaces".to_owned(),
                "$(not-a-shell-command)".to_owned(),
            ],
            vec!["history.csv".to_owned(), "args.txt".to_owned()],
        );
        spec.environment.insert(
            "AERO_TEST_MODE".to_owned(),
            "secret-fixture-value".to_owned(),
        );
        spec.staged_inputs.push(StagedInputSpec {
            source_path: input_path,
            relative_path: "input/case.cfg".to_owned(),
            expected_byte_length: input_bytes.len() as u64,
            expected_sha256: input_sha,
            source_reference: "synthetic-test-input".to_owned(),
        });
        let receipt = execute_external_solver(&spec).expect("run fake solver");
        assert!(receipt.is_process_success());
        assert_eq!(
            fs::read_to_string(spec.run_directory.join("args.txt")).expect("args"),
            "argument with spaces\n$(not-a-shell-command)\n"
        );
        assert!(spec.run_directory.join("input/case.cfg").is_file());
        let environment_text =
            fs::read_to_string(spec.run_directory.join(ENVIRONMENT_MANIFEST_FILE))
                .expect("environment manifest");
        assert!(!environment_text.contains("secret-fixture-value"));
        verify_external_solver_receipt(&spec.run_directory, &receipt)
            .expect("independent byte replay");
        let loaded = read_external_solver_receipt(&spec.run_directory)
            .expect("offline reader reconstructs and verifies persisted receipt");
        assert_eq!(loaded, receipt);
        fs::write(spec.run_directory.join(STDOUT_LOG_FILE), "tampered").expect("mutate raw stdout");
        assert!(
            verify_external_solver_receipt(&spec.run_directory, &receipt)
                .expect_err("mutated log must fail replay")
                .contains("persisted stdout.log differs")
        );
        fs::remove_dir_all(root).expect("remove positive fixture");
    }

    #[test]
    fn nonzero_exit_and_missing_outputs_are_retained_as_failed_receipts() {
        let root = temporary_root("nonzero");
        let executable = fake_solver(&root, "printf 'failed\\n' >&2\nexit 17");
        let spec = request(
            &root,
            executable,
            "failed-run",
            Vec::new(),
            vec!["history.csv".to_owned()],
        );
        let receipt =
            execute_external_solver(&spec).expect("failed process should still produce receipt");
        assert_eq!(receipt.status, "nonzero_exit");
        assert_eq!(receipt.exit_code, Some(17));
        assert_eq!(receipt.missing_output_count(), 1);
        assert!(
            fs::read_to_string(spec.run_directory.join(STDERR_LOG_FILE))
                .expect("stderr")
                .contains("failed")
        );
        verify_external_solver_receipt(&spec.run_directory, &receipt)
            .expect("failed receipt replays");
        fs::remove_dir_all(root).expect("remove nonzero fixture");
    }

    #[test]
    fn timeout_and_output_limits_terminate_the_process_and_preserve_logs() {
        let root = temporary_root("limits");
        let sleeper = fake_solver(&root, "exec sleep 2");
        let mut timed = request(&root, sleeper, "timeout-run", Vec::new(), Vec::new());
        timed.timeout = Duration::from_millis(30);
        let receipt = execute_external_solver(&timed).expect("timeout is a receipt status");
        assert_eq!(receipt.status, "timed_out");
        verify_external_solver_receipt(&timed.run_directory, &receipt)
            .expect("timeout receipt replays");

        let noisy = fake_solver(&root, "while :; do printf 1234567890; done");
        let mut capped = request(&root, noisy, "output-cap-run", Vec::new(), Vec::new());
        capped.max_log_bytes_per_stream = 100;
        capped.timeout = Duration::from_secs(2);
        let receipt = execute_external_solver(&capped).expect("output cap is a receipt status");
        assert_eq!(receipt.status, "output_limit_exceeded");
        assert!(receipt.stdout_byte_length <= 100);
        verify_external_solver_receipt(&capped.run_directory, &receipt)
            .expect("capped receipt replays");
        fs::remove_dir_all(root).expect("remove limit fixture");
    }

    #[test]
    fn stale_run_directories_and_bad_input_digests_are_rejected() {
        let root = temporary_root("preflight");
        let executable = fake_solver(&root, "exit 0");
        let existing = root.join("existing-run");
        fs::create_dir(&existing).expect("make stale directory");
        let stale = request(
            &root,
            executable.clone(),
            "existing-run",
            Vec::new(),
            Vec::new(),
        );
        assert!(
            execute_external_solver(&stale)
                .expect_err("stale run directory rejected")
                .contains("must not already exist")
        );

        let input = root.join("input.dat");
        fs::write(&input, "bytes").expect("write input");
        let mut bad = request(&root, executable, "bad-input-run", Vec::new(), Vec::new());
        bad.staged_inputs.push(StagedInputSpec {
            source_path: input,
            relative_path: "input.dat".to_owned(),
            expected_byte_length: 5,
            expected_sha256: "0".repeat(64),
            source_reference: "synthetic-test-input".to_owned(),
        });
        assert!(
            execute_external_solver(&bad)
                .expect_err("bad input digest rejected")
                .contains("SHA-256 mismatch")
        );
        fs::remove_dir_all(root).expect("remove preflight fixture");
    }

    #[test]
    fn output_path_traversal_and_reserved_collisions_fail_closed() {
        let root = temporary_root("paths");
        let executable = fake_solver(&root, "exit 0");
        let traversal = request(
            &root,
            executable.clone(),
            "traversal-run",
            Vec::new(),
            vec!["../escape.csv".to_owned()],
        );
        assert!(
            execute_external_solver(&traversal)
                .expect_err("traversal rejected")
                .contains("normal root-relative")
        );
        let collision = request(
            &root,
            executable.clone(),
            "collision-run",
            Vec::new(),
            vec![STDOUT_LOG_FILE.to_owned()],
        );
        assert!(
            execute_external_solver(&collision)
                .expect_err("reserved path collision rejected")
                .contains("collides")
        );
        let prefix = request(
            &root,
            executable,
            "prefix-run",
            Vec::new(),
            vec![format!("{STDOUT_LOG_FILE}/nested.csv")],
        );
        assert!(
            execute_external_solver(&prefix)
                .expect_err("file/directory prefix collision rejected")
                .contains("prefix collision")
        );
        fs::remove_dir_all(root).expect("remove paths fixture");
    }
}
