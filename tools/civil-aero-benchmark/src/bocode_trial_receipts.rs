//! Independent admission of per-trial BOCoDe execution receipts.
//!
//! The external BOCoDe wrapper reads NumPy archives with allow_pickle=false. This Rust module
//! does not execute BOCoDe or parse NPZ internals; it verifies the receipt's canonical digest,
//! validates terminal-state invariants, and re-hashes every retained artifact under a caller
//! selected root. A verified receipt is byte-integrity evidence, not a solver signature,
//! trusted-runner attestation, objective-correctness proof, or scientific/physical validation.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path};

use crate::integrity::verify_source_file;
use crate::shapebench::parse_csv_record;

pub const BOCODE_TRIAL_RECEIPT_SCHEMA: &str = "bocode-trial-receipt/0.1.0";
pub const BOCODE_TRIAL_RECEIPT_CSV_HEADER: &str = "schema,trial_id,problem_id,algorithm,seed,source_commit,bocode_version,python_version,wrapper_script_sha256,metadata_sha256,dimension,num_objectives,num_constraints,input_type,requested_n_init,requested_iters,state,objective_evaluation_count,initial_design_count,feasible_evaluation_count,first_feasible_eval_index,best_value,archive_path,archive_bytes,archive_sha256,summary_path,summary_bytes,summary_sha256,stdout_path,stdout_bytes,stdout_sha256,stderr_path,stderr_bytes,stderr_sha256,metadata_path,metadata_bytes,metadata_file_sha256,freeze_path,freeze_bytes,freeze_sha256,environment_path,environment_bytes,environment_sha256,command_path,command_bytes,command_sha256,diagnostic_path,diagnostic_bytes,diagnostic_sha256,elapsed_monotonic_ns,child_user_cpu_ns,child_system_cpu_ns,process_exit_code,skip_detected,summary_row_present,trace_valid,diagnostic_code,receipt_sha256";

const RECEIPT_FIELDS: &[&str] = &[
    "schema",
    "trial_id",
    "problem_id",
    "algorithm",
    "seed",
    "source_commit",
    "bocode_version",
    "python_version",
    "wrapper_script_sha256",
    "metadata_sha256",
    "dimension",
    "num_objectives",
    "num_constraints",
    "input_type",
    "requested_n_init",
    "requested_iters",
    "state",
    "objective_evaluation_count",
    "initial_design_count",
    "feasible_evaluation_count",
    "first_feasible_eval_index",
    "best_value",
    "archive_path",
    "archive_bytes",
    "archive_sha256",
    "summary_path",
    "summary_bytes",
    "summary_sha256",
    "stdout_path",
    "stdout_bytes",
    "stdout_sha256",
    "stderr_path",
    "stderr_bytes",
    "stderr_sha256",
    "metadata_path",
    "metadata_bytes",
    "metadata_file_sha256",
    "freeze_path",
    "freeze_bytes",
    "freeze_sha256",
    "environment_path",
    "environment_bytes",
    "environment_sha256",
    "command_path",
    "command_bytes",
    "command_sha256",
    "diagnostic_path",
    "diagnostic_bytes",
    "diagnostic_sha256",
    "elapsed_monotonic_ns",
    "child_user_cpu_ns",
    "child_system_cpu_ns",
    "process_exit_code",
    "skip_detected",
    "summary_row_present",
    "trace_valid",
    "diagnostic_code",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BocodeTrialState {
    Succeeded,
    Failed,
    InvalidTrace,
    WrapperError,
}

impl BocodeTrialState {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "invalid_trace" => Ok(Self::InvalidTrace),
            "wrapper_error" => Ok(Self::WrapperError),
            _ => Err(format!("unsupported BOCoDe trial state {value}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::InvalidTrace => "invalid_trace",
            Self::WrapperError => "wrapper_error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BocodeTrialArtifact {
    pub role: &'static str,
    /// Relative to the receipt root. "-" represents an artifact that was not produced.
    pub relative_path: String,
    pub byte_length: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BocodeTrialReceipt {
    /// Original field values in the fixed schema order; this is also the canonical digest input.
    fields: Vec<String>,
    pub trial_id: String,
    pub problem_id: String,
    pub algorithm: String,
    pub seed: u64,
    pub state: BocodeTrialState,
    pub objective_evaluation_count: u64,
    pub initial_design_count: u64,
    pub feasible_evaluation_count: u64,
    pub first_feasible_eval_index: Option<u64>,
    pub best_value: Option<f64>,
    pub skip_detected: bool,
    pub summary_row_present: bool,
    pub trace_valid: bool,
    pub diagnostic_code: String,
    pub artifacts: Vec<BocodeTrialArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedBocodeTrialReceipt {
    pub trial_id: String,
    pub state: BocodeTrialState,
    pub receipt_sha256: String,
    pub verified_artifact_count: usize,
    pub objective_evaluation_count: u64,
    pub feasible_evaluation_count: u64,
    pub evidence_level: &'static str,
}

pub const BOCODE_TRIAL_RECEIPT_EVIDENCE_LEVEL: &str =
    "receipt_and_retained_artifact_bytes_verified_runner_and_solver_not_attested_objective_correctness_and_scientific_validity_unverified";

fn field_index(name: &str) -> Result<usize, String> {
    RECEIPT_FIELDS
        .iter()
        .position(|candidate| *candidate == name)
        .ok_or_else(|| format!("internal receipt field {name} is not defined"))
}

fn field<'a>(values: &'a [String], name: &str) -> Result<&'a str, String> {
    let index = field_index(name)?;
    values
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| format!("receipt is missing field {name}"))
}

fn parse_u64(values: &[String], name: &str) -> Result<u64, String> {
    field(values, name)?
        .parse::<u64>()
        .map_err(|error| format!("receipt {name} must be an unsigned integer: {error}"))
}

fn parse_optional_u64(values: &[String], name: &str) -> Result<Option<u64>, String> {
    match field(values, name)? {
        "-" => Ok(None),
        value => value
            .parse::<u64>()
            .map(Some)
            .map_err(|error| format!("receipt {name} must be an unsigned integer or '-': {error}")),
    }
}

fn parse_bool(values: &[String], name: &str) -> Result<bool, String> {
    match field(values, name)? {
        "true" => Ok(true),
        "false" => Ok(false),
        value => Err(format!("receipt {name} must be true or false, got {value}")),
    }
}

fn is_hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn append_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    append_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn canonical_receipt_bytes(values: &[String]) -> Result<Vec<u8>, String> {
    if values.len() != RECEIPT_FIELDS.len() + 1 {
        return Err("receipt field count is invalid for canonical digest".to_owned());
    }
    let mut bytes = Vec::new();
    for (name, value) in RECEIPT_FIELDS.iter().zip(values.iter()) {
        append_string(&mut bytes, name);
        append_string(&mut bytes, value);
    }
    Ok(bytes)
}

fn validate_token(value: &str, name: &str) -> Result<(), String> {
    if value.trim().is_empty()
        || value.contains('\n')
        || value.contains('\r')
        || value.contains('\t')
        || value.contains('\0')
    {
        return Err(format!("{name} must be non-empty and single-line"));
    }
    Ok(())
}

fn artifact_from_fields(
    fields: &[String],
    role: &'static str,
    path_name: &str,
    bytes_name: &str,
    digest_name: &str,
) -> Result<BocodeTrialArtifact, String> {
    let relative_path = field(fields, path_name)?.to_owned();
    let byte_length = parse_u64(fields, bytes_name)?;
    let sha256 = field(fields, digest_name)?.to_ascii_lowercase();
    if relative_path == "-" {
        if byte_length != 0 || sha256 != "-" {
            return Err(format!(
                "absent {role} artifact must use path '-', zero bytes and digest '-'"
            ));
        }
    } else if !is_hex(&sha256, 64) {
        return Err(format!("{role} artifact requires a 64-character SHA-256 digest"));
    }
    Ok(BocodeTrialArtifact {
        role,
        relative_path,
        byte_length,
        sha256,
    })
}

impl BocodeTrialReceipt {
    pub fn parse_csv(input: &str) -> Result<Self, String> {
        let mut lines = input.lines();
        let header = lines
            .next()
            .ok_or_else(|| "BOCoDe trial receipt CSV is empty".to_owned())?
            .trim_end_matches('\r');
        if header != BOCODE_TRIAL_RECEIPT_CSV_HEADER {
            return Err(format!(
                "BOCoDe trial receipt header mismatch; expected exactly: {BOCODE_TRIAL_RECEIPT_CSV_HEADER}"
            ));
        }
        let row = lines
            .next()
            .ok_or_else(|| "BOCoDe trial receipt CSV has no data row".to_owned())?
            .trim_end_matches('\r');
        if row.trim().is_empty() || lines.any(|line| !line.trim().is_empty()) {
            return Err("BOCoDe trial receipt CSV must contain exactly one non-empty data row".to_owned());
        }
        let values = parse_csv_record(row)?;
        if values.len() != RECEIPT_FIELDS.len() + 1 {
            return Err(format!(
                "BOCoDe trial receipt expected {} fields, got {}",
                RECEIPT_FIELDS.len() + 1,
                values.len()
            ));
        }

        if field(&values, "schema")? != BOCODE_TRIAL_RECEIPT_SCHEMA {
            return Err(format!(
                "unsupported BOCoDe trial receipt schema {}",
                field(&values, "schema")?
            ));
        }
        let trial_id = field(&values, "trial_id")?.to_owned();
        let problem_id = field(&values, "problem_id")?.to_owned();
        let algorithm = field(&values, "algorithm")?.to_owned();
        validate_token(&trial_id, "trial_id")?;
        validate_token(&problem_id, "problem_id")?;
        validate_token(&algorithm, "algorithm")?;
        let seed = parse_u64(&values, "seed")?;
        if trial_id != format!("{problem_id}__{algorithm}__seed{seed}") {
            return Err("trial_id does not bind problem_id, algorithm and seed".to_owned());
        }

        let state = BocodeTrialState::parse(field(&values, "state")?)?;
        let source_commit = field(&values, "source_commit")?;
        let wrapper_sha = field(&values, "wrapper_script_sha256")?;
        let metadata_sha = field(&values, "metadata_sha256")?;
        if !is_hex(wrapper_sha, 64) || !is_hex(metadata_sha, 64) {
            return Err("wrapper script and declared task metadata require SHA-256 digests".to_owned());
        }
        if source_commit != "-" && !is_hex(source_commit, 40) {
            return Err("source_commit must be a full 40-character SHA or '-'".to_owned());
        }
        if source_commit == "-" && state != BocodeTrialState::WrapperError {
            return Err("only wrapper_error receipts may omit the observed source commit".to_owned());
        }
        if field(&values, "bocode_version")? == "-" && state == BocodeTrialState::Succeeded {
            return Err("succeeded receipt must identify the installed BOCoDe package version".to_owned());
        }
        validate_token(field(&values, "python_version")?, "python_version")?;

        let dimension = parse_u64(&values, "dimension")?;
        let num_objectives = parse_u64(&values, "num_objectives")?;
        let num_constraints = parse_u64(&values, "num_constraints")?;
        let input_type = field(&values, "input_type")?;
        if state == BocodeTrialState::Succeeded {
            if dimension == 0 || num_objectives == 0 {
                return Err("succeeded receipt requires positive dimension and objective count".to_owned());
            }
            if !matches!(input_type, "continuous" | "discrete" | "mixed") {
                return Err("succeeded receipt has unsupported input_type".to_owned());
            }
        }
        let requested_n_init = parse_u64(&values, "requested_n_init")?;
        let requested_iters = parse_u64(&values, "requested_iters")?;
        if requested_iters == 0 {
            return Err("requested_iters must be positive".to_owned());
        }

        let objective_evaluation_count = parse_u64(&values, "objective_evaluation_count")?;
        let initial_design_count = parse_u64(&values, "initial_design_count")?;
        let feasible_evaluation_count = parse_u64(&values, "feasible_evaluation_count")?;
        let first_feasible_eval_index = parse_optional_u64(&values, "first_feasible_eval_index")?;
        let best_value = match field(&values, "best_value")? {
            "-" => None,
            value => {
                let number = value
                    .parse::<f64>()
                    .map_err(|error| format!("best_value must be a finite float or '-': {error}"))?;
                if !number.is_finite() {
                    return Err("best_value must be finite".to_owned());
                }
                Some(number)
            }
        };
        if initial_design_count > objective_evaluation_count
            || feasible_evaluation_count > objective_evaluation_count
        {
            return Err("trial counts exceed observed objective evaluation count".to_owned());
        }
        match (feasible_evaluation_count, first_feasible_eval_index) {
            (0, None) => {}
            (count, Some(index)) if count > 0 && index < objective_evaluation_count => {}
            _ => return Err("first feasible index is inconsistent with feasible evaluation count".to_owned()),
        }

        let skip_detected = parse_bool(&values, "skip_detected")?;
        let summary_row_present = parse_bool(&values, "summary_row_present")?;
        let trace_valid = parse_bool(&values, "trace_valid")?;
        let diagnostic_code = field(&values, "diagnostic_code")?.to_owned();
        if !matches!(
            diagnostic_code.as_str(),
            "none"
                | "preflight_failed"
                | "subprocess_error"
                | "upstream_skip"
                | "process_nonzero"
                | "summary_row_mismatch"
                | "missing_archive"
                | "invalid_trace"
        ) {
            return Err(format!("unsupported diagnostic_code {diagnostic_code}"));
        }

        let artifacts = vec![
            artifact_from_fields(&values, "archive", "archive_path", "archive_bytes", "archive_sha256")?,
            artifact_from_fields(&values, "summary", "summary_path", "summary_bytes", "summary_sha256")?,
            artifact_from_fields(&values, "stdout", "stdout_path", "stdout_bytes", "stdout_sha256")?,
            artifact_from_fields(&values, "stderr", "stderr_path", "stderr_bytes", "stderr_sha256")?,
            artifact_from_fields(&values, "metadata", "metadata_path", "metadata_bytes", "metadata_file_sha256")?,
            artifact_from_fields(&values, "freeze", "freeze_path", "freeze_bytes", "freeze_sha256")?,
            artifact_from_fields(&values, "environment", "environment_path", "environment_bytes", "environment_sha256")?,
            artifact_from_fields(&values, "command", "command_path", "command_bytes", "command_sha256")?,
            artifact_from_fields(&values, "diagnostic", "diagnostic_path", "diagnostic_bytes", "diagnostic_sha256")?,
        ];

        let expected_digest = field(&values, "receipt_sha256")?;
        if !is_hex(expected_digest, 64) {
            return Err("receipt_sha256 must be a 64-character SHA-256 digest".to_owned());
        }
        let actual_digest = crate::integrity::sha256_hex(&canonical_receipt_bytes(&values)?)?;
        if !actual_digest.eq_ignore_ascii_case(expected_digest) {
            return Err(format!(
                "canonical receipt SHA-256 mismatch: expected {}, got {actual_digest}",
                expected_digest.to_ascii_lowercase()
            ));
        }

        let process_exit_code = match field(&values, "process_exit_code")? {
            "-" => None,
            value => Some(
                value
                    .parse::<i32>()
                    .map_err(|error| format!("process_exit_code must be integer or '-': {error}"))?,
            ),
        };
        let _elapsed = parse_optional_u64(&values, "elapsed_monotonic_ns")?;
        let _child_user = parse_optional_u64(&values, "child_user_cpu_ns")?;
        let _child_system = parse_optional_u64(&values, "child_system_cpu_ns")?;

        let artifact = |role: &str| {
            artifacts
                .iter()
                .find(|candidate| candidate.role == role)
                .expect("artifact role is fixed in receipt parser")
        };
        if artifact("stdout").relative_path == "-" || artifact("stderr").relative_path == "-" {
            return Err("stdout and stderr capture files are mandatory, including failures".to_owned());
        }
        if artifact("freeze").relative_path == "-" || artifact("environment").relative_path == "-" {
            return Err("dependency freeze and safe environment projection are mandatory".to_owned());
        }
        if state == BocodeTrialState::Succeeded {
            if process_exit_code != Some(0)
                || skip_detected
                || !summary_row_present
                || !trace_valid
                || diagnostic_code != "none"
                || artifact("archive").relative_path == "-"
                || artifact("summary").relative_path == "-"
                || artifact("metadata").relative_path == "-"
                || artifact("command").relative_path == "-"
                || artifact("diagnostic").relative_path != "-"
                || best_value.is_none()
                || objective_evaluation_count == 0
                || initial_design_count > objective_evaluation_count
                || feasible_evaluation_count > objective_evaluation_count
            {
                return Err("succeeded receipt violates terminal-state or artifact invariants".to_owned());
            }
            if field(&values, "metadata_file_sha256")? != metadata_sha {
                return Err("succeeded receipt metadata bytes differ from declared catalog digest".to_owned());
            }
        } else {
            if diagnostic_code == "none" || artifact("diagnostic").relative_path == "-" {
                return Err("non-success receipt requires a diagnostic code and retained diagnostic".to_owned());
            }
            match state {
                BocodeTrialState::Failed => {
                    if !matches!(
                        diagnostic_code.as_str(),
                        "upstream_skip" | "process_nonzero" | "summary_row_mismatch"
                    ) {
                        return Err("failed state has an incompatible diagnostic code".to_owned());
                    }
                    if diagnostic_code == "upstream_skip" && !skip_detected {
                        return Err("upstream_skip diagnostic requires skip_detected=true".to_owned());
                    }
                    if diagnostic_code == "process_nonzero" && process_exit_code.unwrap_or(0) == 0 {
                        return Err("process_nonzero diagnostic requires non-zero exit code".to_owned());
                    }
                }
                BocodeTrialState::InvalidTrace => {
                    if !matches!(diagnostic_code.as_str(), "missing_archive" | "invalid_trace") || trace_valid {
                        return Err("invalid_trace state has incompatible trace evidence".to_owned());
                    }
                }
                BocodeTrialState::WrapperError => {
                    if !matches!(diagnostic_code.as_str(), "preflight_failed" | "subprocess_error") {
                        return Err("wrapper_error state has incompatible diagnostic code".to_owned());
                    }
                }
                BocodeTrialState::Succeeded => unreachable!(),
            }
        }

        let _ = (requested_n_init, num_constraints);
        Ok(Self {
            fields: values,
            trial_id,
            problem_id,
            algorithm,
            seed,
            state,
            objective_evaluation_count,
            initial_design_count,
            feasible_evaluation_count,
            first_feasible_eval_index,
            best_value,
            skip_detected,
            summary_row_present,
            trace_valid,
            diagnostic_code,
            artifacts,
        })
    }

    pub fn receipt_sha256(&self) -> &str {
        self.fields
            .last()
            .map(String::as_str)
            .expect("parsed receipt includes terminal digest field")
    }

    pub fn field_value(&self, name: &str) -> Option<&str> {
        field(&self.fields, name).ok()
    }
}

/// Re-hash every file and reject paths that resolve outside the selected receipt root.
/// This verifies exact retained bytes, not source authorship, process isolation, or signatures.
pub fn verify_bocode_trial_receipt(
    receipt_root: &Path,
    receipt: &BocodeTrialReceipt,
) -> Result<VerifiedBocodeTrialReceipt, String> {
    let root = fs::canonicalize(receipt_root)
        .map_err(|error| format!("cannot resolve BOCoDe receipt root: {error}"))?;
    if !root.is_dir() {
        return Err("BOCoDe receipt root must be a directory".to_owned());
    }
    let mut paths = BTreeSet::new();
    for artifact in &receipt.artifacts {
        if artifact.relative_path == "-" {
            if artifact.byte_length != 0 || artifact.sha256 != "-" {
                return Err(format!(
                    "absent {} artifact must have zero bytes and '-' digest",
                    artifact.role
                ));
            }
            continue;
        }
        let relative = Path::new(&artifact.relative_path);
        if relative.as_os_str().is_empty()
            || relative.components().any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(format!(
                "{} artifact path must be a clean relative path",
                artifact.role
            ));
        }
        if !paths.insert(artifact.relative_path.clone()) {
            return Err(format!("duplicate BOCoDe artifact path {}", artifact.relative_path));
        }
        let joined = root.join(relative);
        let canonical = fs::canonicalize(&joined).map_err(|error| {
            format!(
                "cannot resolve BOCoDe {} artifact {}: {error}",
                artifact.role, artifact.relative_path
            )
        })?;
        if !canonical.starts_with(&root) {
            return Err(format!(
                "BOCoDe {} artifact {} resolves outside receipt root",
                artifact.role, artifact.relative_path
            ));
        }
        verify_source_file(
            &canonical,
            artifact.role,
            artifact.byte_length,
            &artifact.sha256,
        )?;
    }

    if receipt.state == BocodeTrialState::Succeeded
        && receipt
            .artifacts
            .iter()
            .find(|artifact| artifact.role == "metadata")
            .is_some_and(|artifact| {
                !artifact
                    .sha256
                    .eq_ignore_ascii_case(receipt.field_value("metadata_sha256").unwrap_or(""))
            })
    {
        return Err("verified metadata artifact digest differs from declared catalog digest".to_owned());
    }

    Ok(VerifiedBocodeTrialReceipt {
        trial_id: receipt.trial_id.clone(),
        state: receipt.state,
        receipt_sha256: receipt.receipt_sha256().to_owned(),
        verified_artifact_count: paths.len(),
        objective_evaluation_count: receipt.objective_evaluation_count,
        feasible_evaluation_count: receipt.feasible_evaluation_count,
        evidence_level: BOCODE_TRIAL_RECEIPT_EVIDENCE_LEVEL,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn csv_escape(value: &str) -> String {
        format!("\"{}\"", value.replace('"', "\"\""))
    }

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "bocode-trial-receipt-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create receipt fixture root");
        path
    }

    fn fixture_fields(root: &Path, succeeded: bool) -> Vec<String> {
        let metadata = br#"{"name":"Branin","dim":2,"num_objectives":1,"num_constraints":0,"input_type":"continuous"}"#;
        let artifacts = [
            ("result.npz", b"npz-byte-fixture".as_slice()),
            ("summary_task0.csv", b"algorithm,problem,seed,best\nrandom_search,Branin,7,1.5\n".as_slice()),
            ("stdout.bin", b"OK random_search / Branin / seed7".as_slice()),
            ("stderr.bin", b"".as_slice()),
            ("metadata.json", metadata.as_slice()),
            ("environment-freeze.txt", b"bocode==1.0.2.dev2\nnumpy==2.2.0\n".as_slice()),
            ("environment.json", b"{}".as_slice()),
            ("command.json", b"{\"argv\":[\"python\",\"runner.py\"]}".as_slice()),
            ("diagnostic.txt", b"".as_slice()),
        ];
        let mut artifact_values = std::collections::BTreeMap::new();
        for (path, bytes) in artifacts {
            fs::write(root.join(path), bytes).expect("write receipt artifact fixture");
            artifact_values.insert(
                path.to_owned(),
                (bytes.len().to_string(), crate::integrity::sha256_hex(bytes).unwrap()),
            );
        }
        let mut values = std::collections::BTreeMap::<String, String>::new();
        let metadata_sha = artifact_values["metadata.json"].1.clone();
        let set = |values: &mut std::collections::BTreeMap<String, String>, key: &str, value: String| {
            values.insert(key.to_owned(), value);
        };
        for (key, value) in [
            ("schema", BOCODE_TRIAL_RECEIPT_SCHEMA.to_owned()),
            ("trial_id", "Branin__random_search__seed7".to_owned()),
            ("problem_id", "Branin".to_owned()),
            ("algorithm", "random_search".to_owned()),
            ("seed", "7".to_owned()),
            ("source_commit", "0123456789abcdef0123456789abcdef01234567".to_owned()),
            ("bocode_version", "1.0.2.dev2".to_owned()),
            ("python_version", "3.12.1".to_owned()),
            ("wrapper_script_sha256", "a".repeat(64)),
            ("metadata_sha256", metadata_sha.clone()),
            ("dimension", "2".to_owned()),
            ("num_objectives", "1".to_owned()),
            ("num_constraints", "0".to_owned()),
            ("input_type", "continuous".to_owned()),
            ("requested_n_init", "10".to_owned()),
            ("requested_iters", "20".to_owned()),
            ("state", if succeeded { "succeeded" } else { "failed" }.to_owned()),
            ("objective_evaluation_count", if succeeded { "30" } else { "0" }.to_owned()),
            ("initial_design_count", if succeeded { "10" } else { "0" }.to_owned()),
            ("feasible_evaluation_count", if succeeded { "30" } else { "0" }.to_owned()),
            ("first_feasible_eval_index", if succeeded { "0" } else { "-" }.to_owned()),
            ("best_value", if succeeded { "1.5" } else { "-" }.to_owned()),
            ("elapsed_monotonic_ns", "1000000".to_owned()),
            ("child_user_cpu_ns", "500000".to_owned()),
            ("child_system_cpu_ns", "200000".to_owned()),
            ("process_exit_code", if succeeded { "0" } else { "0" }.to_owned()),
            ("skip_detected", if succeeded { "false" } else { "true" }.to_owned()),
            ("summary_row_present", if succeeded { "true" } else { "false" }.to_owned()),
            ("trace_valid", if succeeded { "true" } else { "false" }.to_owned()),
            ("diagnostic_code", if succeeded { "none" } else { "upstream_skip" }.to_owned()),
        ] {
            set(&mut values, key, value);
        }
        let roles = [
            ("archive", "result.npz", succeeded),
            ("summary", "summary_task0.csv", succeeded),
            ("stdout", "stdout.bin", true),
            ("stderr", "stderr.bin", true),
            ("metadata", "metadata.json", succeeded),
            ("freeze", "environment-freeze.txt", true),
            ("environment", "environment.json", true),
            ("command", "command.json", succeeded),
            ("diagnostic", "diagnostic.txt", !succeeded),
        ];
        for (role, path, present) in roles {
            let prefix = match role {
                "archive" => "archive",
                "summary" => "summary",
                "stdout" => "stdout",
                "stderr" => "stderr",
                "metadata" => "metadata",
                "freeze" => "freeze",
                "environment" => "environment",
                "command" => "command",
                "diagnostic" => "diagnostic",
                _ => unreachable!(),
            };
            let (bytes, digest) = if present {
                artifact_values[path].clone()
            } else {
                ("0".to_owned(), "-".to_owned())
            };
            set(
                &mut values,
                &format!("{prefix}_path"),
                if present { path.to_owned() } else { "-".to_owned() },
            );
            set(&mut values, &format!("{prefix}_bytes"), bytes);
            set(&mut values, &format!("{prefix}_sha256"), digest);
        }
        if succeeded {
            set(&mut values, "metadata_file_sha256", metadata_sha);
        } else {
            set(&mut values, "metadata_file_sha256", "-".to_owned());
        }
        // Fields not introduced by the artifact loop use exact schema spellings.
        for name in RECEIPT_FIELDS {
            values.entry((*name).to_owned()).or_insert_with(|| "-".to_owned());
        }
        RECEIPT_FIELDS
            .iter()
            .map(|name| values[*name].clone())
            .collect()
    }

    fn make_csv(mut fields: Vec<String>) -> String {
        let digest = crate::integrity::sha256_hex(
            &canonical_receipt_bytes(&fields.iter().chain(std::iter::once(&String::new())).cloned().collect::<Vec<_>>()).unwrap()
        ).unwrap();
        let digest_index = RECEIPT_FIELDS.len();
        fields.push(digest);
        let row = fields.iter().map(|value| csv_escape(value)).collect::<Vec<_>>().join(",");
        format!("{BOCODE_TRIAL_RECEIPT_CSV_HEADER}\n{row}\n")
    }

    #[test]
    fn validates_success_receipt_and_all_retained_artifact_bytes() {
        let root = temp_root("success");
        let fields = fixture_fields(&root, true);
        let csv = make_csv(fields);
        let receipt = BocodeTrialReceipt::parse_csv(&csv).unwrap();
        let verified = verify_bocode_trial_receipt(&root, &receipt).unwrap();
        assert_eq!(verified.state, BocodeTrialState::Succeeded);
        assert_eq!(verified.objective_evaluation_count, 30);
        assert_eq!(verified.feasible_evaluation_count, 30);
        assert_eq!(verified.verified_artifact_count, 8);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn records_upstream_skip_even_when_process_exit_code_is_zero() {
        let root = temp_root("skip");
        let fields = fixture_fields(&root, false);
        let csv = make_csv(fields);
        let receipt = BocodeTrialReceipt::parse_csv(&csv).unwrap();
        assert_eq!(receipt.state, BocodeTrialState::Failed);
        assert!(receipt.skip_detected);
        assert_eq!(receipt.diagnostic_code, "upstream_skip");
        let verified = verify_bocode_trial_receipt(&root, &receipt).unwrap();
        assert_eq!(verified.state, BocodeTrialState::Failed);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn detects_artifact_tampering_after_receipt_creation() {
        let root = temp_root("tamper");
        let fields = fixture_fields(&root, true);
        let csv = make_csv(fields);
        let receipt = BocodeTrialReceipt::parse_csv(&csv).unwrap();
        fs::write(root.join("stdout.bin"), b"tampered").unwrap();
        let error = verify_bocode_trial_receipt(&root, &receipt).unwrap_err();
        assert!(error.contains("byte length mismatch") || error.contains("SHA-256 mismatch"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_trial_identity_or_receipt_digest_tampering() {
        let root = temp_root("identity");
        let fields = fixture_fields(&root, true);
        let csv = make_csv(fields);
        let bad_identity = csv.replace("Branin__random_search__seed7", "other__random_search__seed7");
        assert!(BocodeTrialReceipt::parse_csv(&bad_identity).is_err());
        let bad_digest = csv.replace("random_search", "random_sarch");
        assert!(BocodeTrialReceipt::parse_csv(&bad_digest).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_receipt_artifact_path_traversal() {
        let root = temp_root("path");
        let fields = fixture_fields(&root, true);
        let csv = make_csv(fields).replace("result.npz", "../outside.npz");
        let receipt = BocodeTrialReceipt::parse_csv(&csv).unwrap();
        assert!(verify_bocode_trial_receipt(&root, &receipt).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
