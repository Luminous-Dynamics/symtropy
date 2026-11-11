//! Versioned execution receipts for black-hole numerical studies.
//!
//! This crate validates the execution envelope and its internal identities.
//! It does not execute a solver, authenticate a machine, or prove a physical result.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA_VERSION: &str = "1.0.0";
pub const INPUT_SET_CANONICALIZATION: &str = "utf8_bytewise_name_tab_lowercase_sha256_lf_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus { IllustrativeNotExecuted, Completed, Failed, Interrupted }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkingTreeState { Clean, Dirty, Unknown }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodeIdentity {
    pub repository: String,
    pub revision: String,
    pub working_tree: WorkingTreeState,
    pub source_tree_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildIdentity {
    pub command: String,
    pub compiler: String,
    pub compiler_version: String,
    pub target_triple: String,
    pub profile: String,
    pub flags: Vec<String>,
    pub dependency_lock_sha256: String,
    pub executable_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedDigest { pub name: String, pub sha256: String }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentDigest {
    /// Name only; secret values are never stored in the receipt.
    pub name: String,
    /// SHA-256 of the exact UTF-8 value supplied to the process.
    pub value_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Invocation {
    pub argv: Vec<String>,
    pub working_directory: String,
    pub input_files: Vec<NamedDigest>,
    pub input_set_format: String,
    pub input_set_sha256: String,
    pub environment: Vec<EnvironmentDigest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeIdentity {
    pub operating_system: String,
    pub kernel: String,
    pub architecture: String,
    pub runtime: String,
    pub container_image_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Accelerator { pub model: String, pub driver: String, pub memory_bytes: Option<u64> }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HardwareIdentity {
    pub cpu_model: String,
    pub logical_cpus: u32,
    pub memory_bytes: Option<u64>,
    pub accelerators: Vec<Accelerator>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArithmeticIdentity {
    pub format: String,
    pub precision_bits: u32,
    pub rounding_mode: String,
    pub deterministic_reductions: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolverIdentity {
    pub solver_id: String,
    pub solver_version: String,
    pub algorithm: String,
    pub settings_sha256: String,
    pub parameters: BTreeMap<String, String>,
    pub stopping_criteria: String,
    pub max_steps: Option<u64>,
    pub random_seed: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionResult {
    /// Unix epoch nanoseconds. Null for a non-executed illustrative fixture.
    pub started_at_unix_ns: Option<u64>,
    pub finished_at_unix_ns: Option<u64>,
    pub elapsed_ns: Option<u64>,
    pub exit_code: Option<i32>,
    pub stdout_sha256: Option<String>,
    pub stderr_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactDigest {
    /// Relative path inside the preserved run bundle; traversal is forbidden.
    pub path: String,
    pub role: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionManifest {
    pub schema_version: String,
    pub manifest_id: String,
    pub evidence_status: EvidenceStatus,
    pub method_id: String,
    pub implementation_id: String,
    pub implementation_sha256: String,
    pub code: CodeIdentity,
    pub build: BuildIdentity,
    pub invocation: Invocation,
    pub runtime: RuntimeIdentity,
    pub hardware: HardwareIdentity,
    pub arithmetic: ArithmeticIdentity,
    pub solver: SolverIdentity,
    pub result: ExecutionResult,
    pub outputs: Vec<ArtifactDigest>,
}

#[derive(Debug)]
pub enum ExecutionManifestError { Json(serde_json::Error), Invalid(Vec<String>) }
impl std::fmt::Display for ExecutionManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(error) => write!(f, "invalid execution manifest JSON: {error}"),
            Self::Invalid(errors) => write!(f, "execution manifest failed validation: {}", errors.join("; ")),
        }
    }
}
impl std::error::Error for ExecutionManifestError {}

pub fn sha256_hex(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }

/// Canonical digest shared with bh-response-manifest v0.2.0.
pub fn canonical_input_set_sha256(input_files: &[NamedDigest]) -> String {
    let mut entries: Vec<&NamedDigest> = input_files.iter().collect();
    entries.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    let mut canonical = Vec::new();
    for entry in entries {
        canonical.extend_from_slice(entry.name.as_bytes());
        canonical.push(b'\t');
        canonical.extend_from_slice(entry.sha256.to_ascii_lowercase().as_bytes());
        canonical.push(b'\n');
    }
    sha256_hex(&canonical)
}

fn non_empty(value: &str) -> bool { !value.trim().is_empty() }
fn is_sha256(value: &str) -> bool { value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()) }
fn optional_digest(value: Option<&str>, field: &str, errors: &mut Vec<String>) {
    if value.is_some_and(|v| !is_sha256(v)) { errors.push(format!("{field} must be a 64-character SHA-256 digest")); }
}
fn unique_names<'a>(values: impl IntoIterator<Item = &'a str>, field: &str, errors: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    for value in values {
        if !non_empty(value) { errors.push(format!("{field} contains an empty name")); }
        else if !seen.insert(value) { errors.push(format!("{field} contains duplicate name '{value}'")); }
    }
}
fn safe_relative_path(path: &str) -> bool {
    if path.is_empty() || path == "." || path.starts_with('/') || path.starts_with('\\') { return false; }
    let normalized = path.replace('\\', "/");
    !normalized.split('/').any(|part| part == ".." || part.is_empty())
}

impl ExecutionManifest {
    /// Validate format, identities and execution-state invariants.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.schema_version != SCHEMA_VERSION { errors.push(format!("schema_version must be {SCHEMA_VERSION}")); }
        for (field, value) in [
            ("manifest_id", self.manifest_id.as_str()), ("method_id", self.method_id.as_str()),
            ("implementation_id", self.implementation_id.as_str()), ("repository", self.code.repository.as_str()),
            ("revision", self.code.revision.as_str()), ("build.command", self.build.command.as_str()),
            ("build.compiler", self.build.compiler.as_str()), ("build.compiler_version", self.build.compiler_version.as_str()),
            ("build.target_triple", self.build.target_triple.as_str()), ("build.profile", self.build.profile.as_str()),
            ("invocation.working_directory", self.invocation.working_directory.as_str()),
            ("runtime.operating_system", self.runtime.operating_system.as_str()), ("runtime.kernel", self.runtime.kernel.as_str()),
            ("runtime.architecture", self.runtime.architecture.as_str()), ("runtime.runtime", self.runtime.runtime.as_str()),
            ("hardware.cpu_model", self.hardware.cpu_model.as_str()), ("arithmetic.format", self.arithmetic.format.as_str()),
            ("arithmetic.rounding_mode", self.arithmetic.rounding_mode.as_str()), ("solver.solver_id", self.solver.solver_id.as_str()),
            ("solver.solver_version", self.solver.solver_version.as_str()), ("solver.algorithm", self.solver.algorithm.as_str()),
            ("solver.stopping_criteria", self.solver.stopping_criteria.as_str()),
        ] { if !non_empty(value) { errors.push(format!("{field} must be non-empty")); } }

        for (field, value) in [
            ("implementation_sha256", self.implementation_sha256.as_str()),
            ("build.dependency_lock_sha256", self.build.dependency_lock_sha256.as_str()),
            ("solver.settings_sha256", self.solver.settings_sha256.as_str()),
        ] { if !is_sha256(value) { errors.push(format!("{field} must be SHA-256")); } }
        optional_digest(self.code.source_tree_sha256.as_deref(), "code.source_tree_sha256", &mut errors);
        optional_digest(self.build.executable_sha256.as_deref(), "build.executable_sha256", &mut errors);
        optional_digest(self.runtime.container_image_sha256.as_deref(), "runtime.container_image_sha256", &mut errors);
        if !self.code.revision.chars().all(|c| c.is_ascii_hexdigit()) || ![40, 64].contains(&self.code.revision.len()) {
            errors.push("code.revision must be a 40- or 64-character hexadecimal revision".into());
        }
        if self.arithmetic.precision_bits == 0 { errors.push("arithmetic.precision_bits must be positive".into()); }
        if self.hardware.logical_cpus == 0 { errors.push("hardware.logical_cpus must be positive".into()); }
        if self.invocation.argv.is_empty() || self.invocation.argv.iter().any(|x| !non_empty(x)) {
            errors.push("invocation.argv must contain a non-empty executable argument".into());
        }
        if self.solver.parameters.keys().any(|k| !non_empty(k)) { errors.push("solver parameter names must be non-empty".into()); }

        unique_names(self.invocation.input_files.iter().map(|x| x.name.as_str()), "input_files", &mut errors);
        for input in &self.invocation.input_files {
            if input.name.chars().any(|c| matches!(c, '\t' | '\r' | '\n' | '\0')) {
                errors.push(format!("input '{}' contains a forbidden canonicalization delimiter", input.name));
            }
            if !is_sha256(&input.sha256) { errors.push(format!("input '{}' has invalid SHA-256", input.name)); }
        }
        if self.invocation.input_set_format != INPUT_SET_CANONICALIZATION {
            errors.push(format!("invocation.input_set_format must be {INPUT_SET_CANONICALIZATION}"));
        }
        if !is_sha256(&self.invocation.input_set_sha256) {
            errors.push("invocation.input_set_sha256 must be SHA-256".into());
        } else if canonical_input_set_sha256(&self.invocation.input_files) != self.invocation.input_set_sha256.to_ascii_lowercase() {
            errors.push("invocation.input_set_sha256 does not match canonical input_files".into());
        }

        unique_names(self.invocation.environment.iter().map(|x| x.name.as_str()), "environment", &mut errors);
        for item in &self.invocation.environment {
            if !is_sha256(&item.value_sha256) { errors.push(format!("environment variable '{}' has invalid value SHA-256", item.name)); }
        }
        optional_digest(self.result.stdout_sha256.as_deref(), "result.stdout_sha256", &mut errors);
        optional_digest(self.result.stderr_sha256.as_deref(), "result.stderr_sha256", &mut errors);
        let (start, finish) = (self.result.started_at_unix_ns, self.result.finished_at_unix_ns);
        if let (Some(start), Some(finish)) = (start, finish) {
            if finish < start { errors.push("result.finish precedes result.start".into()); }
            if self.result.elapsed_ns.is_some_and(|n| n != finish.saturating_sub(start)) {
                errors.push("result.elapsed_ns does not equal finish minus start".into());
            }
        } else if start.is_some() != finish.is_some() { errors.push("result start and finish timestamps must both be present or both be null".into()); }

        match self.evidence_status {
            EvidenceStatus::IllustrativeNotExecuted => {
                if start.is_some() || finish.is_some() || self.result.elapsed_ns.is_some() || self.result.exit_code.is_some()
                    || self.result.stdout_sha256.is_some() || self.result.stderr_sha256.is_some() || !self.outputs.is_empty() {
                    errors.push("illustrative_not_executed must not contain execution results or output artifacts".into());
                }
            }
            EvidenceStatus::Completed => {
                if start.is_none() || finish.is_none() || self.result.elapsed_ns.is_none() { errors.push("completed execution requires start, finish and elapsed time".into()); }
                if self.result.exit_code != Some(0) { errors.push("completed execution requires exit_code = 0".into()); }
                if self.result.stdout_sha256.is_none() || self.result.stderr_sha256.is_none() { errors.push("completed execution requires stdout and stderr digests".into()); }
                if self.outputs.is_empty() { errors.push("completed execution requires at least one output artifact".into()); }
            }
            EvidenceStatus::Failed | EvidenceStatus::Interrupted => {
                if start.is_none() { errors.push("failed/interrupted execution requires a start timestamp".into()); }
            }
        }
        unique_names(self.outputs.iter().map(|x| x.path.as_str()), "output paths", &mut errors);
        for output in &self.outputs {
            if !safe_relative_path(&output.path) { errors.push(format!("output path '{}' must be a safe relative path", output.path)); }
            if !non_empty(&output.role) { errors.push(format!("output '{}' role must be non-empty", output.path)); }
            if !is_sha256(&output.sha256) { errors.push(format!("output '{}' has invalid SHA-256", output.path)); }
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
}

pub fn parse_and_validate_json(input: &str) -> Result<ExecutionManifest, ExecutionManifestError> {
    let manifest: ExecutionManifest = serde_json::from_str(input).map_err(ExecutionManifestError::Json)?;
    manifest.validate().map_err(ExecutionManifestError::Invalid)?;
    Ok(manifest)
}
