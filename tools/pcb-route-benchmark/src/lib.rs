use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path},
};

pub const MANIFEST_SCHEMA: &str = "pcb-route-dataset-manifest-v0.1";
pub const RUN_SCHEMA: &str = "pcb-route-run-v0.1";
pub const RECEIPT_SCHEMA: &str = "pcb-route-validation-receipt-v0.1";
pub const COMPARISON_SCHEMA: &str = "pcb-route-paired-comparison-v0.1";
pub const BOOTSTRAP_RESAMPLES: usize = 10_000;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DatasetManifest {
    pub schema: String,
    pub benchmark_id: String,
    pub benchmark_version: String,
    pub dataset_id: String,
    pub split_id: String,
    pub evidence_class: String,
    pub source_url: String,
    pub boards: Vec<BoardPin>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BoardPin {
    pub board_id: String,
    pub input_board_sha256: String,
    pub required_nets: u32,
    pub license: String,
    pub source_url: String,
    pub relative_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RouterPin {
    pub method_id: String,
    pub family: String,
    pub version: String,
    pub source_commit_sha: Option<String>,
    pub binary_sha256: Option<String>,
    pub config_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EnginePin {
    pub name: String,
    pub version: String,
    pub source_commit_sha: Option<String>,
    pub binary_sha256: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentPin {
    pub os: String,
    pub architecture: String,
    pub cpu_model: String,
    pub logical_cores: u32,
    pub memory_bytes: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RunReport {
    pub schema: String,
    pub run_id: String,
    pub benchmark_id: String,
    pub benchmark_version: String,
    pub dataset_id: String,
    pub split_id: String,
    pub dataset_manifest_sha256: String,
    pub drc_policy_sha256: String,
    pub router: RouterPin,
    pub drc_engine: EnginePin,
    pub environment: EnvironmentPin,
    pub seed: Option<u64>,
    pub cases: Vec<BoardAttempt>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BoardAttempt {
    pub board_id: String,
    pub input_board_sha256: String,
    pub execution_status: ExecutionStatus,
    pub drc_evaluated: bool,
    pub drc_report_sha256: Option<String>,
    pub required_nets: u32,
    pub connected_nets: u32,
    pub routing_drc_violation_types: Vec<String>,
    pub dangling_track_count: u32,
    pub dangling_via_count: u32,
    pub upstream_nonrouting_drc_before: u32,
    pub upstream_nonrouting_drc_after: u32,
    pub total_wirelength_mm: Option<f64>,
    pub via_count: Option<u32>,
    pub elapsed_ms: u64,
    pub cpu_ms: Option<u64>,
    pub peak_memory_bytes: Option<u64>,
    pub output_board_sha256: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Completed,
    Timeout,
    ToolError,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    ValidComplete,
    InvalidRoute,
    IncompleteNets,
    EvidenceIncomplete,
    Timeout,
    ToolError,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EvaluatedCase {
    pub board_id: String,
    pub input_board_sha256: String,
    pub output_board_sha256: Option<String>,
    pub drc_report_sha256: Option<String>,
    pub disposition: Disposition,
    pub valid_route_completion: bool,
    pub reason_codes: Vec<String>,
    pub required_nets: u32,
    pub connected_nets: u32,
    pub routing_drc_violation_types: Vec<String>,
    pub dangling_track_count: u32,
    pub dangling_via_count: u32,
    pub upstream_nonrouting_drc_before: u32,
    pub upstream_nonrouting_drc_after: u32,
    pub total_wirelength_mm: Option<f64>,
    pub via_count: Option<u32>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RunSummary {
    pub evidence_class: String,
    pub cases_attempted: usize,
    pub valid_completions: usize,
    pub completion_rate: f64,
    pub invalid_route_cases: usize,
    pub incomplete_net_cases: usize,
    pub evidence_incomplete_cases: usize,
    pub timeout_cases: usize,
    pub tool_error_cases: usize,
    pub routing_drc_violation_histogram: BTreeMap<String, usize>,
    pub upstream_nonrouting_drc_before_total: u64,
    pub upstream_nonrouting_drc_after_total: u64,
    pub median_elapsed_ms_all_attempts: Option<f64>,
    pub p90_elapsed_ms_all_attempts: Option<f64>,
    pub median_wirelength_mm_valid_completions: Option<f64>,
    pub median_via_count_valid_completions: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DatasetFileVerificationEntry {
    pub board_id: String,
    pub relative_path: String,
    pub byte_len: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DatasetFileVerificationReceipt {
    pub schema: String,
    pub evaluator_git_head: String,
    pub dataset_manifest_sha256: String,
    pub verified_board_count: usize,
    pub entries: Vec<DatasetFileVerificationEntry>,
    pub authority: String,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ValidationReceipt {
    pub schema: String,
    pub run_id: String,
    pub input_report_sha256: String,
    pub dataset_manifest_sha256: String,
    pub evaluator_git_head: String,
    pub benchmark_id: String,
    pub benchmark_version: String,
    pub dataset_id: String,
    pub split_id: String,
    pub drc_policy_sha256: String,
    pub router: RouterPin,
    pub drc_engine: EnginePin,
    pub environment: EnvironmentPin,
    pub summary: RunSummary,
    pub cases: Vec<EvaluatedCase>,
    pub authority: String,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone)]
pub struct ValidatedRun {
    pub report: RunReport,
    pub receipt: ValidationReceipt,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ConfidenceInterval {
    pub lower_95: f64,
    pub upper_95: f64,
    pub resamples: usize,
    pub bootstrap_seed_sha256: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BoardPair {
    pub board_id: String,
    pub baseline_disposition: Disposition,
    pub treatment_disposition: Disposition,
    pub baseline_valid: bool,
    pub treatment_valid: bool,
    pub elapsed_delta_ms_treatment_minus_baseline: i128,
    pub wirelength_delta_mm_treatment_minus_baseline: Option<f64>,
    pub via_delta_treatment_minus_baseline: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PairedSummary {
    pub paired_board_count: usize,
    pub baseline_valid_completions: usize,
    pub treatment_valid_completions: usize,
    pub baseline_completion_rate: f64,
    pub treatment_completion_rate: f64,
    pub completion_delta_percentage_points: f64,
    pub treatment_only_valid_completions: usize,
    pub baseline_only_valid_completions: usize,
    pub both_valid: usize,
    pub neither_valid: usize,
    pub completion_delta_95_ci_percentage_points: ConfidenceInterval,
    pub median_elapsed_delta_ms_treatment_minus_baseline: Option<f64>,
    pub mean_elapsed_delta_ms_treatment_minus_baseline: f64,
    pub mean_elapsed_delta_95_ci_ms: ConfidenceInterval,
    pub jointly_valid_board_count: usize,
    pub median_wirelength_delta_mm_treatment_minus_baseline: Option<f64>,
    pub median_wirelength_delta_95_ci_mm: Option<ConfidenceInterval>,
    pub median_via_delta_treatment_minus_baseline: Option<f64>,
    pub median_via_delta_95_ci: Option<ConfidenceInterval>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ComparisonReceipt {
    pub schema: String,
    pub evaluator_git_head: String,
    pub baseline_report_sha256: String,
    pub treatment_report_sha256: String,
    pub dataset_manifest_sha256: String,
    pub baseline_method_id: String,
    pub treatment_method_id: String,
    pub summary: PairedSummary,
    pub board_pairs: Vec<BoardPair>,
    pub authority: String,
    pub claim_ceiling: String,
}

pub fn evaluate_report_bytes(
    report_bytes: &[u8],
    manifest_bytes: &[u8],
    evaluator_git_head: &str,
) -> Result<ValidatedRun, String> {
    if !is_commit_sha(evaluator_git_head) {
        return Err(
            "evaluator Git HEAD must be a 40- or 64-character hexadecimal commit ID".into(),
        );
    }
    let manifest: DatasetManifest = serde_json::from_slice(manifest_bytes)
        .map_err(|error| format!("dataset manifest JSON/schema invalid: {error}"))?;
    validate_manifest(&manifest)?;

    let report: RunReport = serde_json::from_slice(report_bytes)
        .map_err(|error| format!("run report JSON/schema invalid: {error}"))?;
    validate_report(&report, &manifest, manifest_bytes)?;

    let mut cases: Vec<EvaluatedCase> = report.cases.iter().map(evaluate_case).collect();
    cases.sort_by(|a, b| a.board_id.cmp(&b.board_id));
    let summary = summarize_cases(&cases, &manifest.evidence_class);
    let receipt = ValidationReceipt {
        schema: RECEIPT_SCHEMA.into(),
        run_id: report.run_id.clone(),
        input_report_sha256: sha256_hex(report_bytes),
        dataset_manifest_sha256: report.dataset_manifest_sha256.clone(),
        evaluator_git_head: evaluator_git_head.to_ascii_lowercase(),
        benchmark_id: report.benchmark_id.clone(),
        benchmark_version: report.benchmark_version.clone(),
        dataset_id: report.dataset_id.clone(),
        split_id: report.split_id.clone(),
        drc_policy_sha256: report.drc_policy_sha256.clone(),
        router: report.router.clone(),
        drc_engine: report.drc_engine.clone(),
        environment: report.environment.clone(),
        summary,
        cases,
        authority: "geometric_route_evidence_validation_and_paired_metrics_only_no_physical_or_production_release_authority".into(),
        claim_ceiling: "A valid completion requires all required nets connected, evaluated zero routing DRC violations, zero dangling tracks/vias, output-board digest, wirelength and via count. It does not establish SI/PI, manufacturability, physical quality, safety or production-release readiness.".into(),
    };
    Ok(ValidatedRun { report, receipt })
}

fn validate_manifest(manifest: &DatasetManifest) -> Result<(), String> {
    if manifest.schema != MANIFEST_SCHEMA {
        return Err(format!("unsupported manifest schema: {}", manifest.schema));
    }
    for (field, value) in [
        ("benchmark_id", manifest.benchmark_id.as_str()),
        ("benchmark_version", manifest.benchmark_version.as_str()),
        ("dataset_id", manifest.dataset_id.as_str()),
        ("split_id", manifest.split_id.as_str()),
        ("source_url", manifest.source_url.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("manifest {field} must not be empty"));
        }
    }
    if !matches!(
        manifest.evidence_class.as_str(),
        "synthetic" | "external_benchmark"
    ) {
        return Err("evidence_class must be synthetic or external_benchmark".into());
    }
    if manifest.boards.is_empty() {
        return Err("manifest must contain at least one board".into());
    }
    let mut ids = BTreeSet::new();
    for board in &manifest.boards {
        if board.board_id.trim().is_empty()
            || board.license.trim().is_empty()
            || board.source_url.trim().is_empty()
        {
            return Err("each board requires a non-empty ID, license and source URL".into());
        }
        if !ids.insert(board.board_id.as_str()) {
            return Err(format!(
                "duplicate board ID in manifest: {}",
                board.board_id
            ));
        }
        if let Some(relative_path) = board.relative_path.as_deref() {
            validate_relative_path(relative_path, &board.board_id)?;
        } else if manifest.evidence_class == "external_benchmark" {
            return Err(format!(
                "{}: external benchmark board needs a dataset-relative source path",
                board.board_id
            ));
        }
        require_sha256(&board.input_board_sha256, "input_board_sha256")?;
        if board.required_nets == 0 {
            return Err(format!(
                "{}: required_nets must be positive",
                board.board_id
            ));
        }
    }
    Ok(())
}

fn validate_relative_path(relative_path: &str, board_id: &str) -> Result<(), String> {
    if relative_path.trim().is_empty()
        || relative_path.contains(char::from(92))
        || relative_path.contains(':')
        || relative_path.bytes().any(|byte| byte == 0)
        || Path::new(relative_path).is_absolute()
        || relative_path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(format!(
            "{board_id}: unsafe dataset-relative source path: {relative_path}"
        ));
    }
    if Path::new(relative_path)
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "{board_id}: source path contains a non-normal component: {relative_path}"
        ));
    }
    Ok(())
}

pub fn verify_dataset_files(
    manifest_bytes: &[u8],
    dataset_root: &Path,
    evaluator_git_head: &str,
) -> Result<DatasetFileVerificationReceipt, String> {
    if !is_commit_sha(evaluator_git_head) {
        return Err(
            "evaluator Git HEAD must be a 40- or 64-character hexadecimal commit ID".into(),
        );
    }
    let manifest: DatasetManifest = serde_json::from_slice(manifest_bytes)
        .map_err(|error| format!("dataset manifest JSON/schema invalid: {error}"))?;
    validate_manifest(&manifest)?;

    let root = dataset_root.canonicalize().map_err(|error| {
        format!(
            "cannot canonicalize dataset root {}: {error}",
            dataset_root.display()
        )
    })?;
    if !root.is_dir() {
        return Err(format!(
            "dataset root is not a directory: {}",
            root.display()
        ));
    }

    let mut entries = Vec::with_capacity(manifest.boards.len());
    for board in &manifest.boards {
        let relative_path = board
            .relative_path
            .as_deref()
            .ok_or_else(|| format!("{}: dataset-relative source path missing", board.board_id))?;
        validate_relative_path(relative_path, &board.board_id)?;
        let actual_path = root.join(relative_path).canonicalize().map_err(|error| {
            format!(
                "{}: cannot resolve source file {}: {error}",
                board.board_id, relative_path
            )
        })?;
        if !actual_path.starts_with(&root) {
            return Err(format!(
                "{}: source file resolves outside the dataset root",
                board.board_id
            ));
        }
        if !actual_path.is_file() {
            return Err(format!(
                "{}: source path is not a regular file: {}",
                board.board_id, relative_path
            ));
        }
        let bytes = fs::read(&actual_path).map_err(|error| {
            format!(
                "{}: cannot read source file {}: {error}",
                board.board_id, relative_path
            )
        })?;
        let digest = sha256_hex(&bytes);
        if digest != board.input_board_sha256 {
            return Err(format!(
                "{}: source bytes SHA-256 does not match the pinned manifest",
                board.board_id
            ));
        }
        entries.push(DatasetFileVerificationEntry {
            board_id: board.board_id.clone(),
            relative_path: relative_path.to_string(),
            byte_len: bytes.len() as u64,
            sha256: digest,
        });
    }
    entries.sort_by(|a, b| a.board_id.cmp(&b.board_id));

    Ok(DatasetFileVerificationReceipt {
        schema: "pcb-route-dataset-file-verification-v0.1".into(),
        evaluator_git_head: evaluator_git_head.to_ascii_lowercase(),
        dataset_manifest_sha256: sha256_hex(manifest_bytes),
        verified_board_count: entries.len(),
        entries,
        authority: "dataset_source_bytes_match_pinned_manifest_only".into(),
        claim_ceiling: "This receipt verifies source file paths and byte digests under the declared dataset root. It does not route boards, validate board semantics, authorize a benchmark score, or establish manufacturing/physical quality.".into(),
    })
}

fn validate_report(
    report: &RunReport,
    manifest: &DatasetManifest,
    manifest_bytes: &[u8],
) -> Result<(), String> {
    if report.schema != RUN_SCHEMA {
        return Err(format!("unsupported run schema: {}", report.schema));
    }
    if report.run_id.trim().is_empty() {
        return Err("run_id must not be empty".into());
    }
    if report.dataset_manifest_sha256 != sha256_hex(manifest_bytes) {
        return Err("dataset_manifest_sha256 does not match the supplied manifest bytes".into());
    }
    if report.benchmark_id != manifest.benchmark_id
        || report.benchmark_version != manifest.benchmark_version
        || report.dataset_id != manifest.dataset_id
        || report.split_id != manifest.split_id
    {
        return Err("run benchmark/dataset identity differs from the pinned manifest".into());
    }
    require_sha256(&report.drc_policy_sha256, "drc_policy_sha256")?;
    validate_router(&report.router)?;
    validate_engine(&report.drc_engine)?;
    if report.environment.os.trim().is_empty()
        || report.environment.architecture.trim().is_empty()
        || report.environment.cpu_model.trim().is_empty()
        || report.environment.logical_cores == 0
        || report.environment.memory_bytes == 0
    {
        return Err("OS, architecture, CPU, logical-core count and memory must be declared".into());
    }
    if report.cases.len() != manifest.boards.len() {
        return Err(format!(
            "attempt count mismatch: report={}, manifest={}",
            report.cases.len(),
            manifest.boards.len()
        ));
    }

    let pins: BTreeMap<&str, &BoardPin> = manifest
        .boards
        .iter()
        .map(|b| (b.board_id.as_str(), b))
        .collect();
    let mut seen = BTreeSet::new();
    for case in &report.cases {
        if !seen.insert(case.board_id.as_str()) {
            return Err(format!("duplicate board attempt: {}", case.board_id));
        }
        let pin = pins
            .get(case.board_id.as_str())
            .ok_or_else(|| format!("board not in pinned dataset: {}", case.board_id))?;
        validate_case(case)?;
        if case.input_board_sha256 != pin.input_board_sha256 {
            return Err(format!(
                "{}: input board digest differs from pinned manifest",
                case.board_id
            ));
        }
        if case.required_nets != pin.required_nets {
            return Err(format!(
                "{}: required_nets differs from pinned manifest",
                case.board_id
            ));
        }
    }
    if seen.len() != pins.len() {
        return Err("report omits one or more pinned board IDs".into());
    }
    Ok(())
}

fn validate_router(router: &RouterPin) -> Result<(), String> {
    if router.method_id.trim().is_empty()
        || router.family.trim().is_empty()
        || router.version.trim().is_empty()
    {
        return Err("router method_id, family and version are required".into());
    }
    require_sha256(&router.config_sha256, "router config_sha256")?;
    validate_optional_commit(
        router.source_commit_sha.as_deref(),
        "router source_commit_sha",
    )?;
    if let Some(hash) = router.binary_sha256.as_deref() {
        require_sha256(hash, "router binary_sha256")?;
    }
    if router.source_commit_sha.is_none() && router.binary_sha256.is_none() {
        return Err("router must pin a source commit or binary digest".into());
    }
    Ok(())
}

fn validate_engine(engine: &EnginePin) -> Result<(), String> {
    if engine.name.trim().is_empty() || engine.version.trim().is_empty() {
        return Err("DRC engine name and version are required".into());
    }
    validate_optional_commit(
        engine.source_commit_sha.as_deref(),
        "engine source_commit_sha",
    )?;
    if let Some(hash) = engine.binary_sha256.as_deref() {
        require_sha256(hash, "engine binary_sha256")?;
    }
    if engine.source_commit_sha.is_none() && engine.binary_sha256.is_none() {
        return Err("DRC engine must pin a source commit or binary digest".into());
    }
    Ok(())
}

fn validate_optional_commit(value: Option<&str>, label: &str) -> Result<(), String> {
    if let Some(value) = value {
        if !is_commit_sha(value) {
            return Err(format!(
                "{label} must be a 40- or 64-character hexadecimal commit ID"
            ));
        }
    }
    Ok(())
}

fn validate_case(case: &BoardAttempt) -> Result<(), String> {
    require_sha256(&case.input_board_sha256, "input_board_sha256")?;
    if case.required_nets == 0 || case.connected_nets > case.required_nets {
        return Err(format!(
            "{}: invalid required/connected net count",
            case.board_id
        ));
    }
    if case.elapsed_ms == 0 {
        return Err(format!("{}: elapsed_ms must be positive", case.board_id));
    }
    if let Some(value) = case.total_wirelength_mm {
        if !value.is_finite() || value < 0.0 {
            return Err(format!(
                "{}: wirelength must be finite and non-negative",
                case.board_id
            ));
        }
    }
    if let Some(hash) = case.output_board_sha256.as_deref() {
        require_sha256(hash, "output_board_sha256")?;
    }
    if let Some(hash) = case.drc_report_sha256.as_deref() {
        require_sha256(hash, "drc_report_sha256")?;
    }
    if case
        .routing_drc_violation_types
        .iter()
        .any(|v| v.trim().is_empty())
    {
        return Err(format!("{}: empty DRC violation type", case.board_id));
    }
    if !case.drc_evaluated
        && (!case.routing_drc_violation_types.is_empty()
            || case.dangling_track_count > 0
            || case.dangling_via_count > 0)
    {
        return Err(format!(
            "{}: cannot report DRC findings when DRC was not evaluated",
            case.board_id
        ));
    }
    Ok(())
}

fn evaluate_case(case: &BoardAttempt) -> EvaluatedCase {
    let mut reasons = Vec::new();
    let disposition = match case.execution_status {
        ExecutionStatus::Timeout => {
            reasons.push("EXECUTION_TIMEOUT".to_string());
            Disposition::Timeout
        }
        ExecutionStatus::ToolError => {
            reasons.push("TOOL_ERROR".to_string());
            Disposition::ToolError
        }
        ExecutionStatus::Completed => {
            if !case.routing_drc_violation_types.is_empty() {
                reasons.push("ROUTING_DRC_VIOLATIONS".into());
            }
            if case.dangling_track_count > 0 || case.dangling_via_count > 0 {
                reasons.push("DANGLING_ROUTING_ITEMS".into());
            }
            if case.connected_nets < case.required_nets {
                reasons.push("UNROUTED_NETS".into());
            }
            if !case.drc_evaluated {
                reasons.push("ROUTE_DRC_NOT_EVALUATED".into());
            }
            if case.drc_report_sha256.is_none() {
                reasons.push("DRC_REPORT_DIGEST_MISSING".into());
            }
            if case.output_board_sha256.is_none() {
                reasons.push("OUTPUT_BOARD_DIGEST_MISSING".into());
            }
            if case.total_wirelength_mm.is_none() {
                reasons.push("WIRELENGTH_MISSING".into());
            }
            if case.via_count.is_none() {
                reasons.push("VIA_COUNT_MISSING".into());
            }
            if !case.routing_drc_violation_types.is_empty()
                || case.dangling_track_count > 0
                || case.dangling_via_count > 0
            {
                Disposition::InvalidRoute
            } else if case.connected_nets < case.required_nets {
                Disposition::IncompleteNets
            } else if !case.drc_evaluated
                || case.drc_report_sha256.is_none()
                || case.output_board_sha256.is_none()
                || case.total_wirelength_mm.is_none()
                || case.via_count.is_none()
            {
                Disposition::EvidenceIncomplete
            } else {
                Disposition::ValidComplete
            }
        }
    };

    EvaluatedCase {
        board_id: case.board_id.clone(),
        input_board_sha256: case.input_board_sha256.clone(),
        output_board_sha256: case.output_board_sha256.clone(),
        drc_report_sha256: case.drc_report_sha256.clone(),
        disposition,
        valid_route_completion: disposition == Disposition::ValidComplete,
        reason_codes: reasons,
        required_nets: case.required_nets,
        connected_nets: case.connected_nets,
        routing_drc_violation_types: case.routing_drc_violation_types.clone(),
        dangling_track_count: case.dangling_track_count,
        dangling_via_count: case.dangling_via_count,
        upstream_nonrouting_drc_before: case.upstream_nonrouting_drc_before,
        upstream_nonrouting_drc_after: case.upstream_nonrouting_drc_after,
        total_wirelength_mm: case.total_wirelength_mm,
        via_count: case.via_count,
        elapsed_ms: case.elapsed_ms,
    }
}

fn summarize_cases(cases: &[EvaluatedCase], evidence_class: &str) -> RunSummary {
    let valid: Vec<&EvaluatedCase> = cases.iter().filter(|c| c.valid_route_completion).collect();
    let mut histogram = BTreeMap::new();
    for case in cases {
        for ty in &case.routing_drc_violation_types {
            *histogram.entry(ty.clone()).or_insert(0usize) += 1;
        }
    }
    let elapsed: Vec<f64> = cases.iter().map(|c| c.elapsed_ms as f64).collect();
    let wirelength: Vec<f64> = valid.iter().filter_map(|c| c.total_wirelength_mm).collect();
    let vias: Vec<f64> = valid
        .iter()
        .filter_map(|c| c.via_count.map(|n| n as f64))
        .collect();
    let n = cases.len();
    RunSummary {
        evidence_class: evidence_class.to_string(),
        cases_attempted: n,
        valid_completions: valid.len(),
        completion_rate: valid.len() as f64 / n as f64,
        invalid_route_cases: cases
            .iter()
            .filter(|c| c.disposition == Disposition::InvalidRoute)
            .count(),
        incomplete_net_cases: cases
            .iter()
            .filter(|c| c.disposition == Disposition::IncompleteNets)
            .count(),
        evidence_incomplete_cases: cases
            .iter()
            .filter(|c| c.disposition == Disposition::EvidenceIncomplete)
            .count(),
        timeout_cases: cases
            .iter()
            .filter(|c| c.disposition == Disposition::Timeout)
            .count(),
        tool_error_cases: cases
            .iter()
            .filter(|c| c.disposition == Disposition::ToolError)
            .count(),
        routing_drc_violation_histogram: histogram,
        upstream_nonrouting_drc_before_total: cases
            .iter()
            .map(|c| c.upstream_nonrouting_drc_before as u64)
            .sum(),
        upstream_nonrouting_drc_after_total: cases
            .iter()
            .map(|c| c.upstream_nonrouting_drc_after as u64)
            .sum(),
        median_elapsed_ms_all_attempts: median(&elapsed),
        p90_elapsed_ms_all_attempts: percentile_nearest_rank(&elapsed, 0.90),
        median_wirelength_mm_valid_completions: median(&wirelength),
        median_via_count_valid_completions: median(&vias),
    }
}

pub fn compare_runs(
    baseline: &ValidatedRun,
    treatment: &ValidatedRun,
    evaluator_git_head: &str,
) -> Result<ComparisonReceipt, String> {
    if !is_commit_sha(evaluator_git_head) {
        return Err(
            "evaluator Git HEAD must be a 40- or 64-character hexadecimal commit ID".into(),
        );
    }
    let left = &baseline.receipt;
    let right = &treatment.receipt;
    for (name, a, b) in [
        (
            "benchmark_id",
            left.benchmark_id.as_str(),
            right.benchmark_id.as_str(),
        ),
        (
            "benchmark_version",
            left.benchmark_version.as_str(),
            right.benchmark_version.as_str(),
        ),
        (
            "dataset_id",
            left.dataset_id.as_str(),
            right.dataset_id.as_str(),
        ),
        ("split_id", left.split_id.as_str(), right.split_id.as_str()),
        (
            "manifest digest",
            left.dataset_manifest_sha256.as_str(),
            right.dataset_manifest_sha256.as_str(),
        ),
        (
            "DRC policy",
            left.drc_policy_sha256.as_str(),
            right.drc_policy_sha256.as_str(),
        ),
    ] {
        if a != b {
            return Err(format!("cannot pair runs: {name} differs"));
        }
    }
    if left.drc_engine != right.drc_engine {
        return Err("cannot pair runs: DRC engine pin differs".into());
    }
    if left.environment != right.environment {
        return Err("cannot pair runs: environment differs".into());
    }
    if left.router.method_id == right.router.method_id {
        return Err("baseline and treatment method IDs must differ".into());
    }

    let base: BTreeMap<String, &EvaluatedCase> =
        left.cases.iter().map(|c| (c.board_id.clone(), c)).collect();
    let treat: BTreeMap<String, &EvaluatedCase> = right
        .cases
        .iter()
        .map(|c| (c.board_id.clone(), c))
        .collect();
    if base.keys().ne(treat.keys()) {
        return Err("cannot pair runs: board ID sets differ".into());
    }

    let mut pairs = Vec::with_capacity(base.len());
    let mut completion_delta = Vec::with_capacity(base.len());
    let mut elapsed_delta = Vec::with_capacity(base.len());
    let mut wire_delta = Vec::new();
    let mut via_delta = Vec::new();
    let (mut treatment_only, mut baseline_only, mut both, mut neither) =
        (0usize, 0usize, 0usize, 0usize);

    for (board_id, b) in base.iter() {
        let b = *b;
        let t = treat
            .get(board_id)
            .copied()
            .ok_or_else(|| format!("paired result missing board {board_id}"))?;
        if b.input_board_sha256 != t.input_board_sha256 || b.required_nets != t.required_nets {
            return Err(format!("{board_id}: paired inputs differ"));
        }
        match (b.valid_route_completion, t.valid_route_completion) {
            (false, true) => treatment_only += 1,
            (true, false) => baseline_only += 1,
            (true, true) => both += 1,
            (false, false) => neither += 1,
        }
        completion_delta
            .push(bool_as_f64(t.valid_route_completion) - bool_as_f64(b.valid_route_completion));
        elapsed_delta.push(t.elapsed_ms as f64 - b.elapsed_ms as f64);
        let wire = match (
            b.valid_route_completion
                .then_some(b.total_wirelength_mm)
                .flatten(),
            t.valid_route_completion
                .then_some(t.total_wirelength_mm)
                .flatten(),
        ) {
            (Some(bw), Some(tw)) => {
                wire_delta.push(tw - bw);
                Some(tw - bw)
            }
            _ => None,
        };
        let vias = match (
            b.valid_route_completion.then_some(b.via_count).flatten(),
            t.valid_route_completion.then_some(t.via_count).flatten(),
        ) {
            (Some(bv), Some(tv)) => {
                via_delta.push(tv as f64 - bv as f64);
                Some(tv as i64 - bv as i64)
            }
            _ => None,
        };
        pairs.push(BoardPair {
            board_id: board_id.clone(),
            baseline_disposition: b.disposition,
            treatment_disposition: t.disposition,
            baseline_valid: b.valid_route_completion,
            treatment_valid: t.valid_route_completion,
            elapsed_delta_ms_treatment_minus_baseline: t.elapsed_ms as i128 - b.elapsed_ms as i128,
            wirelength_delta_mm_treatment_minus_baseline: wire,
            via_delta_treatment_minus_baseline: vias,
        });
    }

    let seed_material = format!(
        "{}|{}|{}|pcb-route-bootstrap-v0.1",
        left.input_report_sha256, right.input_report_sha256, left.dataset_manifest_sha256
    );
    let seed_hex = sha256_hex(seed_material.as_bytes());
    let (completion_seed, completion_seed_sha) =
        derive_bootstrap_seed(&seed_hex, "completion_delta");
    let (elapsed_seed, elapsed_seed_sha) = derive_bootstrap_seed(&seed_hex, "elapsed_mean_delta");
    let (wirelength_seed, wirelength_seed_sha) =
        derive_bootstrap_seed(&seed_hex, "wirelength_median_delta");
    let (via_seed, via_seed_sha) = derive_bootstrap_seed(&seed_hex, "via_median_delta");
    let (completion_lower, completion_upper) =
        bootstrap_mean_ci(&completion_delta, completion_seed);
    let (elapsed_lower, elapsed_upper) = bootstrap_mean_ci(&elapsed_delta, elapsed_seed);
    let wirelength_ci = bootstrap_median_ci(&wire_delta, wirelength_seed);
    let via_ci = bootstrap_median_ci(&via_delta, via_seed);
    let n = pairs.len();
    let baseline_valid = left.summary.valid_completions;
    let treatment_valid = right.summary.valid_completions;

    Ok(ComparisonReceipt {
        schema: COMPARISON_SCHEMA.into(),
        evaluator_git_head: evaluator_git_head.to_ascii_lowercase(),
        baseline_report_sha256: left.input_report_sha256.clone(),
        treatment_report_sha256: right.input_report_sha256.clone(),
        dataset_manifest_sha256: left.dataset_manifest_sha256.clone(),
        baseline_method_id: left.router.method_id.clone(),
        treatment_method_id: right.router.method_id.clone(),
        summary: PairedSummary {
            paired_board_count: n,
            baseline_valid_completions: baseline_valid,
            treatment_valid_completions: treatment_valid,
            baseline_completion_rate: baseline_valid as f64 / n as f64,
            treatment_completion_rate: treatment_valid as f64 / n as f64,
            completion_delta_percentage_points: 100.0 * (treatment_valid as f64 - baseline_valid as f64) / n as f64,
            treatment_only_valid_completions: treatment_only,
            baseline_only_valid_completions: baseline_only,
            both_valid: both,
            neither_valid: neither,
            completion_delta_95_ci_percentage_points: ConfidenceInterval {
                lower_95: completion_lower * 100.0,
                upper_95: completion_upper * 100.0,
                resamples: BOOTSTRAP_RESAMPLES,
                bootstrap_seed_sha256: completion_seed_sha,
            },
            median_elapsed_delta_ms_treatment_minus_baseline: median(&elapsed_delta),
            mean_elapsed_delta_ms_treatment_minus_baseline: mean(&elapsed_delta),
            mean_elapsed_delta_95_ci_ms: ConfidenceInterval {
                lower_95: elapsed_lower,
                upper_95: elapsed_upper,
                resamples: BOOTSTRAP_RESAMPLES,
                bootstrap_seed_sha256: elapsed_seed_sha,
            },
            jointly_valid_board_count: wire_delta.len(),
            median_wirelength_delta_mm_treatment_minus_baseline: median(&wire_delta),
            median_wirelength_delta_95_ci_mm: interval_option(wirelength_ci, &wirelength_seed_sha),
            median_via_delta_treatment_minus_baseline: median(&via_delta),
            median_via_delta_95_ci: interval_option(via_ci, &via_seed_sha),
        },
        board_pairs: pairs,
        authority: "paired_geometric_routing_metrics_only_no_physical_or_production_release_authority".into(),
        claim_ceiling: "Paired comparison is meaningful only for the pinned identical dataset, source boards, DRC policy, engine and environment. This receipt does not establish electrical performance, manufacturing quality or production readiness.".into(),
    })
}

fn bool_as_f64(value: bool) -> f64 {
    if value {
        1.0
    } else {
        0.0
    }
}

fn derive_bootstrap_seed(base_seed_sha256: &str, metric: &str) -> (u64, String) {
    let seed_sha256 = sha256_hex(format!("{base_seed_sha256}|{metric}").as_bytes());
    let seed = u64::from_str_radix(&seed_sha256[..16], 16).unwrap_or(0xA5A5_5A5A);
    (seed, seed_sha256)
}

fn bootstrap_median_ci(values: &[f64], seed: u64) -> Option<(f64, f64)> {
    if values.is_empty() {
        return None;
    }
    let mut state = seed;
    let mut samples = Vec::with_capacity(BOOTSTRAP_RESAMPLES);
    let mut sample = Vec::with_capacity(values.len());
    for _ in 0..BOOTSTRAP_RESAMPLES {
        sample.clear();
        for _ in 0..values.len() {
            sample.push(values[(next_random(&mut state) % values.len() as u64) as usize]);
        }
        samples.push(median(&sample).unwrap_or(0.0));
    }
    samples.sort_by(f64::total_cmp);
    Some((quantile(&samples, 0.025), quantile(&samples, 0.975)))
}

fn bootstrap_mean_ci(values: &[f64], seed: u64) -> (f64, f64) {
    let mut state = seed;
    let mut samples = Vec::with_capacity(BOOTSTRAP_RESAMPLES);
    let mut sample = Vec::with_capacity(values.len());
    for _ in 0..BOOTSTRAP_RESAMPLES {
        sample.clear();
        for _ in 0..values.len() {
            sample.push(values[(next_random(&mut state) % values.len() as u64) as usize]);
        }
        samples.push(mean(&sample));
    }
    samples.sort_by(f64::total_cmp);
    (quantile(&samples, 0.025), quantile(&samples, 0.975))
}

fn next_random(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut values = values.to_vec();
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len() % 2 == 1 {
        Some(values[middle])
    } else {
        Some((values[middle - 1] + values[middle]) / 2.0)
    }
}

fn percentile_nearest_rank(values: &[f64], percentile: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut values = values.to_vec();
    values.sort_by(f64::total_cmp);
    let rank = ((percentile * values.len() as f64).ceil() as usize).max(1);
    Some(values[(rank - 1).min(values.len() - 1)])
}

fn quantile(values: &[f64], percentile: f64) -> f64 {
    let position = percentile.clamp(0.0, 1.0) * (values.len() - 1) as f64;
    let low = position.floor() as usize;
    let high = position.ceil() as usize;
    if low == high {
        values[low]
    } else {
        values[low] + (position - low as f64) * (values[high] - values[low])
    }
}

fn require_sha256(value: &str, name: &str) -> Result<(), String> {
    if !is_sha256(value) {
        return Err(format!("{name} must be a 64-character hexadecimal SHA-256"));
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn is_commit_sha(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = include_str!("../fixtures/synthetic-dataset-manifest-v0_1.json");
    const BASELINE: &str = include_str!("../fixtures/synthetic-baseline-run-v0_1.json");
    const TREATMENT: &str = include_str!("../fixtures/synthetic-treatment-run-v0_1.json");
    const EVALUATOR_HEAD: &str = "abababababababababababababababababababab";

    fn valid_run(raw: &str) -> ValidatedRun {
        let mut value: serde_json::Value = serde_json::from_str(raw).expect("fixture JSON");
        value["dataset_manifest_sha256"] =
            serde_json::Value::String(sha256_hex(MANIFEST.as_bytes()));
        let bytes = serde_json::to_vec(&value).expect("canonical fixture run");
        evaluate_report_bytes(&bytes, MANIFEST.as_bytes(), EVALUATOR_HEAD).expect("run validates")
    }

    #[test]
    fn dataset_intake_hashes_actual_bytes_and_rejects_path_escape() {
        let mut manifest: DatasetManifest = serde_json::from_str(MANIFEST).unwrap();
        manifest.evidence_class = "external_benchmark".into();
        let root =
            std::env::temp_dir().join(format!("pcb-route-benchmark-intake-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        for board in &mut manifest.boards {
            let relative_path = format!("{}.kicad_pcb", board.board_id);
            let bytes = format!("synthetic source bytes for {}\n", board.board_id);
            std::fs::write(root.join(&relative_path), bytes.as_bytes()).unwrap();
            board.input_board_sha256 = sha256_hex(bytes.as_bytes());
            board.relative_path = Some(relative_path);
        }
        let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
        let receipt = verify_dataset_files(&manifest_bytes, &root, EVALUATOR_HEAD)
            .expect("source files verify");
        assert_eq!(receipt.verified_board_count, 6);
        assert_eq!(receipt.entries.len(), 6);
        assert_eq!(receipt.dataset_manifest_sha256, sha256_hex(&manifest_bytes));

        let first_path = root.join(&receipt.entries[0].relative_path);
        std::fs::write(&first_path, b"tampered after manifest creation").unwrap();
        assert!(verify_dataset_files(&manifest_bytes, &root, EVALUATOR_HEAD)
            .unwrap_err()
            .contains("source bytes SHA-256"));

        manifest.boards[0].relative_path = Some("../outside.kicad_pcb".into());
        let escaped_manifest = serde_json::to_vec(&manifest).unwrap();
        assert!(
            verify_dataset_files(&escaped_manifest, &root, EVALUATOR_HEAD)
                .unwrap_err()
                .contains("unsafe dataset-relative source path")
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn frozen_synthetic_fixture_has_full_denominator_and_expected_baseline() {
        let baseline = valid_run(BASELINE);
        assert_eq!(baseline.receipt.summary.cases_attempted, 6);
        assert_eq!(baseline.receipt.summary.valid_completions, 2);
        assert!((baseline.receipt.summary.completion_rate - 2.0 / 6.0).abs() < 1e-9);
        assert_eq!(baseline.receipt.summary.timeout_cases, 1);
        assert_eq!(baseline.receipt.summary.tool_error_cases, 1);
    }

    #[test]
    fn connected_net_with_crossing_is_not_a_valid_route() {
        let baseline = valid_run(BASELINE);
        let crossing = baseline
            .receipt
            .cases
            .iter()
            .find(|c| c.board_id == "crossing-003")
            .unwrap();
        assert_eq!(crossing.disposition, Disposition::InvalidRoute);
        assert!(!crossing.valid_route_completion);
    }

    #[test]
    fn clean_connected_route_without_raw_drc_digest_is_evidence_incomplete() {
        let mut value: serde_json::Value = serde_json::from_str(BASELINE).unwrap();
        value["dataset_manifest_sha256"] =
            serde_json::Value::String(sha256_hex(MANIFEST.as_bytes()));
        value["cases"][0]["drc_report_sha256"] = serde_json::Value::Null;
        let bytes = serde_json::to_vec(&value).unwrap();
        let run = evaluate_report_bytes(&bytes, MANIFEST.as_bytes(), EVALUATOR_HEAD).unwrap();
        let case = run
            .receipt
            .cases
            .iter()
            .find(|c| c.board_id == "easy-001")
            .unwrap();
        assert_eq!(case.disposition, Disposition::EvidenceIncomplete);
        assert!(case
            .reason_codes
            .contains(&"DRC_REPORT_DIGEST_MISSING".to_string()));
        assert!(!case.valid_route_completion);
    }

    #[test]
    fn nonrouting_upstream_drc_is_reported_separately() {
        let baseline = valid_run(BASELINE);
        let upstream = baseline
            .receipt
            .cases
            .iter()
            .find(|c| c.board_id == "upstream-drc-006")
            .unwrap();
        assert!(upstream.valid_route_completion);
        assert_eq!(
            baseline
                .receipt
                .summary
                .upstream_nonrouting_drc_before_total,
            3
        );
        assert_eq!(
            baseline.receipt.summary.upstream_nonrouting_drc_after_total,
            3
        );
    }

    #[test]
    fn missing_or_duplicate_board_rows_reject() {
        let mut value: serde_json::Value = serde_json::from_str(BASELINE).unwrap();
        value["cases"].as_array_mut().unwrap().pop();
        value["dataset_manifest_sha256"] =
            serde_json::Value::String(sha256_hex(MANIFEST.as_bytes()));
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(
            evaluate_report_bytes(&bytes, MANIFEST.as_bytes(), EVALUATOR_HEAD)
                .unwrap_err()
                .contains("attempt count mismatch")
        );

        value = serde_json::from_str(BASELINE).unwrap();
        let duplicate = value["cases"][0].clone();
        value["cases"][1] = duplicate;
        value["dataset_manifest_sha256"] =
            serde_json::Value::String(sha256_hex(MANIFEST.as_bytes()));
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(
            evaluate_report_bytes(&bytes, MANIFEST.as_bytes(), EVALUATOR_HEAD)
                .unwrap_err()
                .contains("duplicate board attempt")
        );
    }

    #[test]
    fn altered_manifest_digest_and_board_hash_reject() {
        let mut value: serde_json::Value = serde_json::from_str(BASELINE).unwrap();
        value["dataset_manifest_sha256"] = serde_json::Value::String("0".repeat(64));
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(
            evaluate_report_bytes(&bytes, MANIFEST.as_bytes(), EVALUATOR_HEAD)
                .unwrap_err()
                .contains("dataset_manifest_sha256")
        );

        value = serde_json::from_str(BASELINE).unwrap();
        value["dataset_manifest_sha256"] =
            serde_json::Value::String(sha256_hex(MANIFEST.as_bytes()));
        value["cases"][0]["input_board_sha256"] = serde_json::Value::String("f".repeat(64));
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(
            evaluate_report_bytes(&bytes, MANIFEST.as_bytes(), EVALUATOR_HEAD)
                .unwrap_err()
                .contains("input board digest differs")
        );
    }

    #[test]
    fn paired_comparison_rejects_engine_mismatch_and_reports_controlled_delta() {
        let baseline = valid_run(BASELINE);
        let treatment = valid_run(TREATMENT);
        let comparison = compare_runs(&baseline, &treatment, EVALUATOR_HEAD).unwrap();
        assert_eq!(comparison.summary.paired_board_count, 6);
        assert_eq!(comparison.summary.baseline_valid_completions, 2);
        assert_eq!(comparison.summary.treatment_valid_completions, 4);
        assert!(
            (comparison.summary.completion_delta_percentage_points - (100.0 / 3.0)).abs() < 1e-8
        );
        assert_eq!(comparison.summary.treatment_only_valid_completions, 2);
        assert_eq!(comparison.summary.baseline_only_valid_completions, 0);
        assert_eq!(comparison.summary.jointly_valid_board_count, 2);
        assert_eq!(
            comparison
                .summary
                .completion_delta_95_ci_percentage_points
                .resamples,
            BOOTSTRAP_RESAMPLES
        );
        assert!(comparison
            .summary
            .median_wirelength_delta_95_ci_mm
            .is_some());
        assert!(comparison.summary.median_via_delta_95_ci.is_some());
        assert_ne!(
            comparison
                .summary
                .completion_delta_95_ci_percentage_points
                .bootstrap_seed_sha256,
            comparison
                .summary
                .mean_elapsed_delta_95_ci_ms
                .bootstrap_seed_sha256
        );

        let mut value: serde_json::Value = serde_json::from_str(TREATMENT).unwrap();
        value["dataset_manifest_sha256"] =
            serde_json::Value::String(sha256_hex(MANIFEST.as_bytes()));
        value["drc_engine"]["version"] = serde_json::Value::String("different-engine".into());
        let bytes = serde_json::to_vec(&value).unwrap();
        let altered = evaluate_report_bytes(&bytes, MANIFEST.as_bytes(), EVALUATOR_HEAD).unwrap();
        assert!(compare_runs(&baseline, &altered, EVALUATOR_HEAD)
            .unwrap_err()
            .contains("DRC engine pin differs"));
    }

    #[test]
    fn commit_id_validation_supports_sha1_and_sha256_lengths() {
        assert!(is_commit_sha(&"a".repeat(40)));
        assert!(is_commit_sha(&"b".repeat(64)));
        assert!(!is_commit_sha(&"g".repeat(40)));
        assert!(!is_commit_sha(&"a".repeat(39)));
    }
}
