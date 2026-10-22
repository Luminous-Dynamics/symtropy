use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path},
};

pub const LEGACY_SYNTHETIC_MANIFEST_SCHEMA: &str = "pcb-route-dataset-manifest-v0.1";
pub const MANIFEST_SCHEMA: &str = "pcb-route-dataset-manifest-v0.2";
pub const RUN_SCHEMA: &str = "pcb-route-run-v0.1";
pub const RECEIPT_SCHEMA: &str = "pcb-route-validation-receipt-v0.1";
pub const COMPARISON_SCHEMA: &str = "pcb-route-paired-comparison-v0.1";
pub const BOOTSTRAP_RESAMPLES: usize = 10_000;

pub mod anytime;
pub mod design_skeleton;
pub mod kicad_drc;
pub mod pcbworld_metrics;

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
    #[serde(default)]
    pub source_split_config_sha256: Option<String>,
    #[serde(default)]
    pub source_split_config_relative_path: Option<String>,
    #[serde(default)]
    pub source_split_config_url: Option<String>,
    #[serde(default)]
    pub source_split_config_git_blob_sha1: Option<String>,
    #[serde(default)]
    pub source_split_commit_sha: Option<String>,
    #[serde(default)]
    pub source_split_name: Option<String>,
    #[serde(default)]
    pub intake_metadata_sha256: Option<String>,
    #[serde(default)]
    pub license_audit_manifest_relative_path: Option<String>,
    #[serde(default)]
    pub license_audit_manifest_sha256: Option<String>,
    pub boards: Vec<BoardPin>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct IntakeSpec {
    pub schema: String,
    pub benchmark_id: String,
    pub benchmark_version: String,
    pub dataset_id: String,
    pub split_id: String,
    pub source_url: String,
    pub source_split_commit_sha: String,
    pub source_split_config_url: String,
    pub source_split_config_git_blob_sha1: String,
    pub source_split_config_relative_path: String,
    pub license_audit_manifest_relative_path: String,
    pub license_audit_manifest_sha256: String,
    pub split_name: String,
    pub boards: Vec<IntakeBoard>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct IntakeBoard {
    pub board_id: String,
    pub difficulty: String,
    pub required_nets: u32,
    /// Local dataset-root path for the exact license text independently pinned by the audit.
    pub license_evidence_relative_path: String,
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
    #[serde(default)]
    pub original_source_repository: Option<String>,
    #[serde(default)]
    pub source_metadata_path: Option<String>,
    #[serde(default)]
    pub source_metadata_git_blob_sha1: Option<String>,
    #[serde(default)]
    pub license_evidence_relative_path: Option<String>,
    #[serde(default)]
    pub license_evidence_sha256: Option<String>,
    #[serde(default)]
    pub license_evidence_url: Option<String>,
    #[serde(default)]
    pub license_evidence_source_commit_sha: Option<String>,
    #[serde(default)]
    pub license_evidence_git_blob_sha1: Option<String>,
    #[serde(default)]
    pub design_skeleton_sha256: Option<String>,
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
    pub unconnected_item_count: Option<u32>,
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
    #[serde(default)]
    pub output_board_relative_path: Option<String>,
    #[serde(default)]
    pub drc_report_relative_path: Option<String>,
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
    pub unconnected_item_count: Option<u32>,
    pub routing_drc_violation_types: Vec<String>,
    pub dangling_track_count: u32,
    pub dangling_via_count: u32,
    pub upstream_nonrouting_drc_before: u32,
    pub upstream_nonrouting_drc_after: u32,
    pub total_wirelength_mm: Option<f64>,
    pub via_count: Option<u32>,
    pub cpu_ms: Option<u64>,
    pub peak_memory_bytes: Option<u64>,
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
    pub reported_unconnected_item_total: u64,
    pub cases_missing_unconnected_item_count: usize,
    pub upstream_nonrouting_drc_before_total: u64,
    pub upstream_nonrouting_drc_after_total: u64,
    pub median_elapsed_ms_all_attempts: Option<f64>,
    pub p90_elapsed_ms_all_attempts: Option<f64>,
    pub cases_missing_cpu_ms: usize,
    pub median_cpu_ms_all_attempts: Option<f64>,
    pub p90_cpu_ms_all_attempts: Option<f64>,
    pub cases_missing_peak_memory_bytes: usize,
    pub median_peak_memory_bytes_all_attempts: Option<f64>,
    pub p90_peak_memory_bytes_all_attempts: Option<f64>,
    pub median_wirelength_mm_valid_completions: Option<f64>,
    pub median_via_count_valid_completions: Option<f64>,
}


#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct LicenseAuditManifest {
    pub schema: String,
    pub status: String,
    #[serde(default)]
    pub evidence_class: Option<String>,
    pub upstream: LicenseAuditUpstream,
    pub boards: Vec<LicenseAuditBoard>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct LicenseAuditUpstream {
    pub pcbworld_repository: String,
    pub pcbworld_commit_sha: String,
    pub d3_split_config_path: String,
    pub d3_split_config_git_blob_sha1: String,
    pub pcbench_repository: String,
    pub pcbench_commit_sha: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct LicenseAuditBoard {
    pub board_id: String,
    pub difficulty: String,
    pub pcbench_directory: String,
    pub metadata_path: String,
    pub metadata_git_blob_sha1: String,
    pub original_source_repository: String,
    #[serde(default)]
    pub pcbench_reported_licenses: Vec<LicenseAuditReportedLicense>,
    pub primary_source_license_review: String,
    pub benchmark_eligibility: String,
    #[serde(default)]
    pub primary_source_license_evidence: Option<PrimarySourceLicenseEvidence>,
    #[serde(default)]
    pub license_match_status: Option<String>,
    #[serde(default)]
    pub prepared_board_relative_path: Option<String>,
    #[serde(default)]
    pub prepared_board_sha256: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct LicenseAuditReportedLicense {
    #[serde(default)]
    pub spdx_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PrimarySourceLicenseEvidence {
    pub repository: String,
    pub file_path: String,
    pub git_blob_sha1: String,
    pub resolved_spdx_id: String,
    pub blob_url: String,
    pub source_commit_sha: String,
    pub blob_hash_reverified_at_pinned_commit: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DatasetFileVerificationEntry {
    pub board_id: String,
    pub role: String,
    pub relative_path: String,
    pub byte_len: u64,
    pub sha256: String,
    pub git_blob_sha1: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DatasetFileVerificationReceipt {
    pub schema: String,
    pub evaluator_git_head: String,
    pub dataset_manifest_sha256: String,
    #[serde(default)]
    pub license_audit_manifest_sha256: Option<String>,
    pub verified_board_count: usize,
    pub verified_license_evidence_count: usize,
    pub entries: Vec<DatasetFileVerificationEntry>,
    pub authority: String,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RunArtifactVerificationEntry {
    pub board_id: String,
    pub role: String,
    pub relative_path: String,
    pub byte_len: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RunArtifactVerificationReceipt {
    pub schema: String,
    pub evaluator_git_head: String,
    pub run_id: String,
    pub input_report_sha256: String,
    pub dataset_manifest_sha256: String,
    pub verified_artifact_count: usize,
    pub entries: Vec<RunArtifactVerificationEntry>,
    pub authority: String,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BoardDesignSkeletonVerification {
    pub board_id: String,
    pub input_board_sha256: String,
    pub output_board_sha256: String,
    pub input_skeleton_sha256: String,
    pub output_skeleton_sha256: String,
    pub skeleton_matches: bool,
    pub excluded_output_segments: usize,
    pub excluded_output_vias: usize,
    pub excluded_output_routed_arcs: usize,
    pub excluded_output_zone_fill_polygons: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DesignSkeletonRunVerificationReceipt {
    pub schema: String,
    pub evaluator_git_head: String,
    pub run_id: String,
    pub input_report_sha256: String,
    pub dataset_manifest_sha256: String,
    pub verified_board_count: usize,
    pub boards: Vec<BoardDesignSkeletonVerification>,
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
    pub completion_sign_test_two_sided_p_value: Option<f64>,
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

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SeedReportPin {
    pub seed: u64,
    pub baseline_report_sha256: String,
    pub treatment_report_sha256: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ReplicatedBoardSummary {
    pub board_id: String,
    pub paired_attempt_count: usize,
    pub baseline_valid_attempts: usize,
    pub treatment_valid_attempts: usize,
    pub baseline_completion_rate: f64,
    pub treatment_completion_rate: f64,
    pub completion_delta_percentage_points: f64,
    pub mean_elapsed_delta_ms_treatment_minus_baseline: f64,
    pub jointly_valid_attempts: usize,
    pub mean_wirelength_delta_mm_treatment_minus_baseline: Option<f64>,
    pub mean_via_delta_treatment_minus_baseline: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ReplicatedSummary {
    pub board_count: usize,
    pub seed_count: usize,
    pub paired_attempt_count: usize,
    pub baseline_valid_attempts: usize,
    pub treatment_valid_attempts: usize,
    pub baseline_completion_rate: f64,
    pub treatment_completion_rate: f64,
    pub completion_delta_percentage_points: f64,
    pub completion_treatment_better_boards: usize,
    pub completion_baseline_better_boards: usize,
    pub completion_tied_boards: usize,
    pub completion_sign_test_two_sided_p_value: Option<f64>,
    pub completion_delta_95_ci_board_clustered: ConfidenceInterval,
    pub mean_elapsed_delta_ms_treatment_minus_baseline: f64,
    pub mean_elapsed_delta_95_ci_board_clustered_ms: ConfidenceInterval,
    pub median_elapsed_delta_ms_per_attempt: f64,
    pub jointly_valid_attempts: usize,
    pub mean_wirelength_delta_mm_treatment_minus_baseline: Option<f64>,
    pub wirelength_delta_95_ci_board_clustered_mm: Option<ConfidenceInterval>,
    pub mean_via_delta_treatment_minus_baseline: Option<f64>,
    pub via_delta_95_ci_board_clustered: Option<ConfidenceInterval>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ReplicatedComparisonReceipt {
    pub schema: String,
    pub evaluator_git_head: String,
    pub dataset_manifest_sha256: String,
    pub baseline_method_id: String,
    pub treatment_method_id: String,
    pub seed_pairs: Vec<SeedReportPin>,
    pub summary: ReplicatedSummary,
    pub boards: Vec<ReplicatedBoardSummary>,
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
    let legacy_synthetic_v1 = manifest.schema == LEGACY_SYNTHETIC_MANIFEST_SCHEMA
        && manifest.evidence_class == "synthetic"
        && manifest.license_audit_manifest_relative_path.is_none()
        && manifest.license_audit_manifest_sha256.is_none();
    if manifest.schema != MANIFEST_SCHEMA && !legacy_synthetic_v1 {
        return Err(format!("unsupported dataset manifest schema/evidence combination: schema={}, evidence_class={}", manifest.schema, manifest.evidence_class));
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
    if let Some(digest) = manifest.source_split_config_sha256.as_deref() {
        require_sha256(digest, "source_split_config_sha256")?;
    }
    if let Some(commit) = manifest.source_split_commit_sha.as_deref() {
        if !is_commit_sha(commit) {
            return Err(
                "source_split_commit_sha must be a 40- or 64-character hexadecimal commit ID"
                    .into(),
            );
        }
    }
    if let Some(relative_path) = manifest.source_split_config_relative_path.as_deref() {
        validate_relative_path(relative_path, "source split config")?;
    }
    if let Some(url) = manifest.source_split_config_url.as_deref() {
        if !url.starts_with("https://") {
            return Err("source_split_config_url must use HTTPS".into());
        }
    }
    if let Some(blob) = manifest.source_split_config_git_blob_sha1.as_deref() {
        if !is_git_blob_sha1(blob) {
            return Err(
                "source_split_config_git_blob_sha1 must be a 40-character hexadecimal Git blob ID"
                    .into(),
            );
        }
    }
    if manifest.evidence_class == "external_benchmark" {
        let commit = manifest.source_split_commit_sha.as_deref().unwrap_or_default();
        let url = manifest.source_split_config_url.as_deref().unwrap_or_default();
        if !url.contains(&format!("/{commit}/")) {
            return Err(
                "external split-config URL must contain the pinned immutable source commit".into(),
            );
        }
    }
    if let Some(digest) = manifest.intake_metadata_sha256.as_deref() {
        require_sha256(digest, "intake_metadata_sha256")?;
    }
    if manifest.evidence_class == "external_benchmark"
        && (manifest.source_split_config_sha256.is_none()
            || manifest.source_split_config_relative_path.is_none()
            || manifest.source_split_config_url.is_none()
            || manifest.source_split_config_git_blob_sha1.is_none()
            || manifest.source_split_commit_sha.is_none()
            || manifest.source_split_name.is_none()
            || manifest.intake_metadata_sha256.is_none())
    {
        return Err("external manifest must bind split config SHA-256, immutable URL, source Git blob, source commit, split name and intake metadata digest".into());
    }
    if manifest.license_audit_manifest_relative_path.is_some()
        != manifest.license_audit_manifest_sha256.is_some()
    {
        return Err("license-audit manifest path and SHA-256 must be supplied together".into());
    }
    if let Some(path) = manifest.license_audit_manifest_relative_path.as_deref() {
        validate_relative_path(path, "license-audit manifest")?;
        require_sha256(
            manifest.license_audit_manifest_sha256.as_deref().unwrap_or_default(),
            "license_audit_manifest_sha256",
        )?;
    }
    if manifest.evidence_class == "external_benchmark"
        && (manifest.schema != MANIFEST_SCHEMA
            || manifest.license_audit_manifest_relative_path.is_none())
    {
        return Err("external manifest v0.2 must be bound to an immutable license-audit manifest".into());
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
        if manifest.evidence_class == "external_benchmark" {
            let license = board.license.trim().to_ascii_lowercase();
            if matches!(
                license.as_str(),
                "unknown" | "n/a" | "not-provided" | "noassertion" | "unverified"
            ) {
                return Err(format!(
                    "{}: external board license must be explicitly verified",
                    board.board_id
                ));
            }
            if !board.source_url.starts_with("https://") {
                return Err(format!(
                    "{}: external board source_url must use HTTPS",
                    board.board_id
                ));
            }
            let license_path =
                board
                    .license_evidence_relative_path
                    .as_deref()
                    .ok_or_else(|| {
                        format!(
                            "{}: external board lacks hash-bound license-evidence path",
                            board.board_id
                        )
                    })?;
            validate_relative_path(
                license_path,
                &format!("{} license evidence", board.board_id),
            )?;
            let license_url = board.license_evidence_url.as_deref().ok_or_else(|| {
                format!(
                    "{}: external board lacks license-evidence source URL",
                    board.board_id
                )
            })?;
            let source_commit = board
                .license_evidence_source_commit_sha
                .as_deref()
                .ok_or_else(|| {
                    format!(
                        "{}: external board lacks immutable license-source commit",
                        board.board_id
                    )
                })?;
            if !is_commit_sha(source_commit) {
                return Err(format!(
                    "{}: license-evidence source commit must be a 40- or 64-character hexadecimal commit ID",
                    board.board_id
                ));
            }
            if !license_url.starts_with("https://") {
                return Err(format!(
                    "{}: license-evidence source URL must use HTTPS",
                    board.board_id
                ));
            }
            if !license_url.contains(&format!("/{source_commit}/")) {
                return Err(format!(
                    "{}: license-evidence URL must contain the pinned immutable source commit",
                    board.board_id
                ));
            }
            let license_sha = board.license_evidence_sha256.as_deref().ok_or_else(|| {
                format!(
                    "{}: external board lacks license-evidence SHA-256",
                    board.board_id
                )
            })?;
            require_sha256(license_sha, "license_evidence_sha256")?;
            let git_blob_sha1 =
                board
                    .license_evidence_git_blob_sha1
                    .as_deref()
                    .ok_or_else(|| {
                        format!(
                            "{}: external board lacks license-evidence Git blob SHA-1",
                            board.board_id
                        )
                    })?;
            if !is_git_blob_sha1(git_blob_sha1) {
                return Err(format!(
                    "{}: license-evidence Git blob SHA-1 must be 40 hexadecimal characters",
                    board.board_id
                ));
            }
        }
        if manifest.evidence_class == "external_benchmark" {
            let source_repo = board.original_source_repository.as_deref().unwrap_or_default();
            if !source_repo.starts_with("https://") || board.source_url != source_repo {
                return Err(format!(
                    "{}: external source URL/repository must come from the license audit",
                    board.board_id
                ));
            }
            if board.source_metadata_path.as_deref().is_none_or(str::is_empty) {
                return Err(format!("{}: immutable PCBench metadata path is missing", board.board_id));
            }
            if !is_git_blob_sha1(board.source_metadata_git_blob_sha1.as_deref().unwrap_or_default()) {
                return Err(format!("{}: immutable PCBench metadata Git blob SHA-1 is missing or invalid", board.board_id));
            }
            if board.license_evidence_relative_path.is_none()
                || board.license_evidence_sha256.is_none()
                || board.license_evidence_url.is_none()
                || board.license_evidence_source_commit_sha.is_none()
                || board.license_evidence_git_blob_sha1.is_none()
            {
                return Err(format!("{}: external license evidence is incomplete", board.board_id));
            }
            let skeleton_sha = board.design_skeleton_sha256.as_deref()
                .ok_or_else(|| format!("{}: external manifest lacks design-skeleton SHA-256", board.board_id))?;
            require_sha256(skeleton_sha, "design_skeleton_sha256")?;
        }
        if let Some(skeleton_sha) = board.design_skeleton_sha256.as_deref() {
            require_sha256(skeleton_sha, "design_skeleton_sha256")?;
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

pub fn build_dataset_manifest(
    split_config_bytes: &[u8],
    intake_spec_bytes: &[u8],
    dataset_root: &Path,
) -> Result<DatasetManifest, String> {
    let split_config: serde_json::Value = serde_json::from_slice(split_config_bytes)
        .map_err(|error| format!("upstream split configuration JSON invalid: {error}"))?;
    let intake: IntakeSpec = serde_json::from_slice(intake_spec_bytes)
        .map_err(|error| format!("intake specification JSON/schema invalid: {error}"))?;

    if intake.schema != "pcb-route-intake-spec-v0.2" {
        return Err(format!("unsupported intake specification schema: {}", intake.schema));
    }
    if !is_commit_sha(&intake.source_split_commit_sha) {
        return Err("source_split_commit_sha must be a 40- or 64-character hexadecimal commit ID".into());
    }
    if !intake.source_split_config_url.starts_with("https://")
        || !intake.source_url.starts_with("https://")
    {
        return Err("source split config and benchmark URLs must use HTTPS".into());
    }
    if !is_git_blob_sha1(&intake.source_split_config_git_blob_sha1) {
        return Err("source_split_config_git_blob_sha1 must be a 40-character hexadecimal Git blob ID".into());
    }
    require_sha256(&intake.license_audit_manifest_sha256, "license_audit_manifest_sha256")?;
    validate_relative_path(&intake.source_split_config_relative_path, "source split config")?;
    validate_relative_path(&intake.license_audit_manifest_relative_path, "license audit manifest")?;
    if !intake.source_split_config_url.contains(&format!("/{}/", intake.source_split_commit_sha)) {
        return Err("source_split_config_url must contain the pinned immutable source commit".into());
    }
    if git_blob_sha1(split_config_bytes) != intake.source_split_config_git_blob_sha1 {
        return Err("source split-config bytes do not match the pinned upstream Git blob SHA-1".into());
    }
    if intake.split_name.trim().is_empty()
        || intake.benchmark_id.trim().is_empty()
        || intake.benchmark_version.trim().is_empty()
        || intake.dataset_id.trim().is_empty()
        || intake.split_id.trim().is_empty()
    {
        return Err("intake identity and split name must be declared".into());
    }
    if intake.boards.is_empty() {
        return Err("intake specification must contain at least one selected board".into());
    }
    let filename = split_config.get("board_filename")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "upstream split config is missing string board_filename".to_string())?;
    validate_relative_path(filename, "board_filename")?;

    let root = dataset_root.canonicalize().map_err(|error| {
        format!("cannot canonicalize dataset root {}: {error}", dataset_root.display())
    })?;
    if !root.is_dir() {
        return Err(format!("dataset root is not a directory: {}", root.display()));
    }

    let actual_split_path = root.join(&intake.source_split_config_relative_path)
        .canonicalize().map_err(|error| {
            format!("cannot resolve source split config {}: {error}", intake.source_split_config_relative_path)
        })?;
    if !actual_split_path.starts_with(&root) || !actual_split_path.is_file() {
        return Err("source split config is not a regular file contained by the dataset root".into());
    }
    let actual_split_bytes = fs::read(&actual_split_path)
        .map_err(|error| format!("cannot read source split config bytes: {error}"))?;
    if actual_split_bytes != split_config_bytes {
        return Err("dataset-root split-config bytes differ from the supplied pinned split config".into());
    }
    if git_blob_sha1(&actual_split_bytes) != intake.source_split_config_git_blob_sha1 {
        return Err("dataset-root split-config bytes do not match the pinned upstream Git blob SHA-1".into());
    }

    let audit_path = root.join(&intake.license_audit_manifest_relative_path)
        .canonicalize().map_err(|error| {
            format!("cannot resolve license-audit manifest {}: {error}", intake.license_audit_manifest_relative_path)
        })?;
    if !audit_path.starts_with(&root) || !audit_path.is_file() {
        return Err("license-audit manifest is not a regular file contained by the dataset root".into());
    }
    if audit_path == actual_split_path {
        return Err("license-audit manifest aliases the source split config".into());
    }
    let audit_bytes = fs::read(&audit_path)
        .map_err(|error| format!("cannot read license-audit manifest bytes: {error}"))?;
    if sha256_hex(&audit_bytes) != intake.license_audit_manifest_sha256 {
        return Err("license-audit manifest bytes SHA-256 does not match intake pin".into());
    }
    let audit: LicenseAuditManifest = serde_json::from_slice(&audit_bytes)
        .map_err(|error| format!("license-audit manifest JSON/schema invalid: {error}"))?;
    validate_audit_envelope(
        &audit,
        &intake.source_url,
        &intake.source_split_commit_sha,
        &intake.source_split_config_git_blob_sha1,
        &intake.benchmark_id,
        &intake.dataset_id,
    )?;
    let expected_split_url_suffix = format!(
        "/{}/{}",
        intake.source_split_commit_sha,
        audit.upstream.d3_split_config_path
    );
    if !intake.source_split_config_url.ends_with(&expected_split_url_suffix) {
        return Err("source split-config URL path differs from the pinned license-audit path".into());
    }

    let mut seen = BTreeSet::new();
    let mut used_paths = BTreeSet::new();
    used_paths.insert(actual_split_path.clone());
    used_paths.insert(audit_path.clone());
    let mut boards = Vec::with_capacity(intake.boards.len());

    for selection in &intake.boards {
        if !seen.insert(selection.board_id.as_str()) {
            return Err(format!("duplicate selected board ID: {}", selection.board_id));
        }
        if selection.required_nets == 0 {
            return Err(format!("{}: required_nets must be positive", selection.board_id));
        }
        validate_relative_path(
            &selection.license_evidence_relative_path,
            &format!("{} license evidence", selection.board_id),
        )?;
        let audited = audit.boards.iter()
            .find(|board| board.board_id == selection.board_id)
            .ok_or_else(|| format!("{} is not listed in the pinned license-audit manifest", selection.board_id))?;
        if audited.difficulty != selection.difficulty {
            return Err(format!("{}: difficulty differs from the license-audit record", selection.board_id));
        }
        if audited.primary_source_license_review.trim().is_empty() {
            return Err(format!("{}: license-audit review status is missing", selection.board_id));
        }
        let synthetic = audit.evidence_class.as_deref() == Some("synthetic");
        let required_eligibility = if synthetic {
            "eligible_for_synthetic_test_only"
        } else {
            "approved_for_benchmark_execution"
        };
        if audited.benchmark_eligibility != required_eligibility {
            return Err(format!(
                "{}: license-audit eligibility is {:?}; requires {:?}",
                selection.board_id, audited.benchmark_eligibility, required_eligibility
            ));
        }
        if !synthetic && audit.status != "approved_for_benchmark_execution" {
            return Err(format!(
                "license-audit manifest status {:?} does not authorize external benchmark execution",
                audit.status
            ));
        }
        if synthetic && audit.status != "synthetic_test_only_eligible_fixture" {
            return Err("synthetic license audit must be explicitly labelled test-only".into());
        }

        let source_repo = audited.original_source_repository.trim();
        if !source_repo.starts_with("https://") {
            return Err(format!("{}: audited original source repository must use HTTPS", selection.board_id));
        }
        let source_metadata_path = audited.metadata_path.as_str();
        validate_relative_path(source_metadata_path, &format!("{} source metadata", selection.board_id))?;
        if !source_metadata_path.starts_with("PCBs/") || !source_metadata_path.ends_with("/metadata.json") {
            return Err(format!("{}: audited PCBench metadata path is malformed", selection.board_id));
        }
        if !is_git_blob_sha1(&audited.metadata_git_blob_sha1) {
            return Err(format!("{}: audited PCBench metadata must have a 40-character Git blob SHA-1", selection.board_id));
        }

        let evidence = audited.primary_source_license_evidence.as_ref()
            .ok_or_else(|| format!("{}: audit has no primary-source license evidence", selection.board_id))?;
        if !evidence.blob_hash_reverified_at_pinned_commit
            || evidence.repository.trim() != source_repo
            || evidence.resolved_spdx_id.trim().is_empty()
        {
            return Err(format!("{}: audit source-license evidence is unverified or disagrees with the audited source repository", selection.board_id));
        }
        if !is_commit_sha(&evidence.source_commit_sha) || !is_git_blob_sha1(&evidence.git_blob_sha1) {
            return Err(format!("{}: invalid source license commit/blob pin", selection.board_id));
        }
        validate_relative_path(&evidence.file_path, &format!("{} audited license file", selection.board_id))?;
        let expected_license_url = format!(
            "{}/blob/{}/{}",
            source_repo.trim_end_matches('/'),
            evidence.source_commit_sha,
            evidence.file_path
        );
        if evidence.blob_url != expected_license_url {
            return Err(format!("{}: source license blob URL does not match the audited repository, commit and file path", selection.board_id));
        }
        if audited.license_match_status.as_deref() != Some("source_text_matches_pcbench_spdx_metadata") {
            return Err(format!("{}: source license has not been reconciled with its PCBench metadata license", selection.board_id));
        }
        let declared_spdx: Vec<&str> = audited.pcbench_reported_licenses.iter()
            .filter_map(|license| license.spdx_id.as_deref())
            .collect();
        if !declared_spdx.is_empty() && !declared_spdx.contains(&evidence.resolved_spdx_id.as_str()) {
            return Err(format!("{}: resolved license does not match the audited PCBench SPDX declaration", selection.board_id));
        }

        let relative_path = format!("{}/{}", selection.board_id, filename);
        validate_relative_path(&relative_path, &selection.board_id)?;
        let audited_prepared_path = audited.prepared_board_relative_path.as_deref()
            .ok_or_else(|| format!("{}: license audit has not pinned prepared board bytes", selection.board_id))?;
        validate_relative_path(audited_prepared_path, &format!("{} audited prepared board", selection.board_id))?;
        if audited_prepared_path != relative_path {
            return Err(format!("{}: selected board path differs from the audit-pinned prepared board path", selection.board_id));
        }
        let actual_path = root.join(&relative_path).canonicalize().map_err(|error| {
            format!("{}: cannot resolve source file {}: {error}", selection.board_id, relative_path)
        })?;
        if !actual_path.starts_with(&root) || !actual_path.is_file() {
            return Err(format!("{}: source file is not a regular file contained by the dataset root", selection.board_id));
        }
        if !used_paths.insert(actual_path.clone()) {
            return Err(format!("{}: source path aliases another pinned dataset artifact", selection.board_id));
        }
        let bytes = fs::read(&actual_path)
            .map_err(|error| format!("{}: cannot read source board bytes: {error}", selection.board_id))?;
        let actual_board_sha = sha256_hex(&bytes);
        let design_skeleton_sha256 = crate::design_skeleton::fingerprint_board_bytes(&bytes)?
            .design_skeleton_sha256;
        let expected_board_sha = audited.prepared_board_sha256.as_deref()
            .ok_or_else(|| format!("{}: audit lacks prepared-board SHA-256", selection.board_id))?;
        require_sha256(expected_board_sha, "audit prepared_board_sha256")?;
        if actual_board_sha != expected_board_sha {
            return Err(format!("{}: source-board bytes SHA-256 differs from the audit-approved prepared-board digest", selection.board_id));
        }

        let license_actual_path = root.join(&selection.license_evidence_relative_path)
            .canonicalize().map_err(|error| {
                format!("{}: cannot resolve license-evidence file {}: {error}", selection.board_id, selection.license_evidence_relative_path)
            })?;
        if !license_actual_path.starts_with(&root) || !license_actual_path.is_file() {
            return Err(format!("{}: license-evidence path is not a regular file contained by the dataset root", selection.board_id));
        }
        if !used_paths.insert(license_actual_path.clone()) {
            return Err(format!("{}: license-evidence path aliases another pinned dataset artifact", selection.board_id));
        }
        let license_bytes = fs::read(&license_actual_path)
            .map_err(|error| format!("{}: cannot read license-evidence bytes: {error}", selection.board_id))?;
        if license_bytes.is_empty() {
            return Err(format!("{}: license-evidence file is empty", selection.board_id));
        }
        if git_blob_sha1(&license_bytes) != evidence.git_blob_sha1 {
            return Err(format!("{}: license-evidence bytes do not match audit-pinned source Git blob SHA-1", selection.board_id));
        }

        boards.push(BoardPin {
            board_id: selection.board_id.clone(),
            input_board_sha256: actual_board_sha,
            required_nets: selection.required_nets,
            license: evidence.resolved_spdx_id.clone(),
            source_url: source_repo.to_string(),
            relative_path: Some(relative_path),
            original_source_repository: Some(source_repo.to_string()),
            source_metadata_path: Some(source_metadata_path.to_string()),
            source_metadata_git_blob_sha1: Some(audited.metadata_git_blob_sha1.clone()),
            license_evidence_relative_path: Some(selection.license_evidence_relative_path.clone()),
            license_evidence_sha256: Some(sha256_hex(&license_bytes)),
            license_evidence_url: Some(evidence.blob_url.clone()),
            license_evidence_source_commit_sha: Some(evidence.source_commit_sha.to_ascii_lowercase()),
            license_evidence_git_blob_sha1: Some(evidence.git_blob_sha1.clone()),
            design_skeleton_sha256: Some(design_skeleton_sha256),
        });
    }
    boards.sort_by(|a, b| a.board_id.cmp(&b.board_id));

    let evidence_class = if audit.evidence_class.as_deref() == Some("synthetic") {
        "synthetic"
    } else {
        "external_benchmark"
    };
    if evidence_class == "synthetic"
        && (!intake.benchmark_id.starts_with("synthetic-")
            || !intake.dataset_id.starts_with("synthetic-"))
    {
        return Err("synthetic audit fixtures require synthetic-prefixed benchmark and dataset IDs".into());
    }

    let manifest = DatasetManifest {
        schema: MANIFEST_SCHEMA.into(),
        benchmark_id: intake.benchmark_id.clone(),
        benchmark_version: intake.benchmark_version.clone(),
        dataset_id: intake.dataset_id.clone(),
        split_id: intake.split_id.clone(),
        evidence_class: evidence_class.into(),
        source_url: intake.source_url.clone(),
        source_split_config_sha256: Some(sha256_hex(split_config_bytes)),
        source_split_config_relative_path: Some(intake.source_split_config_relative_path.clone()),
        source_split_config_url: Some(intake.source_split_config_url.clone()),
        source_split_config_git_blob_sha1: Some(git_blob_sha1(split_config_bytes)),
        source_split_commit_sha: Some(intake.source_split_commit_sha.to_ascii_lowercase()),
        source_split_name: Some(intake.split_name.clone()),
        intake_metadata_sha256: Some(sha256_hex(intake_spec_bytes)),
        license_audit_manifest_relative_path: Some(intake.license_audit_manifest_relative_path.clone()),
        license_audit_manifest_sha256: Some(sha256_hex(&audit_bytes)),
        boards,
    };
    validate_manifest(&manifest)?;
    Ok(manifest)
}

fn validate_audit_envelope(
    audit: &LicenseAuditManifest,
    expected_pcbworld_repository: &str,
    expected_pcbworld_commit: &str,
    expected_split_blob: &str,
    benchmark_id: &str,
    dataset_id: &str,
) -> Result<(), String> {
    if audit.schema != "pcbworld-pilot-source-metadata-audit-v0.1" {
        return Err(format!("unsupported license-audit schema: {}", audit.schema));
    }
    if !is_commit_sha(&audit.upstream.pcbworld_commit_sha)
        || !is_commit_sha(&audit.upstream.pcbench_commit_sha)
        || !is_git_blob_sha1(&audit.upstream.d3_split_config_git_blob_sha1)
    {
        return Err("license-audit upstream commit/blob pins are malformed".into());
    }
    validate_relative_path(&audit.upstream.d3_split_config_path, "audit D3 split-config path")?;
    if !audit.upstream.pcbworld_repository.starts_with("https://")
        || !audit.upstream.pcbench_repository.starts_with("https://")
    {
        return Err("license-audit upstream repositories must use HTTPS".into());
    }
    if audit.upstream.pcbworld_repository != expected_pcbworld_repository
        || audit.upstream.pcbworld_commit_sha != expected_pcbworld_commit
        || audit.upstream.d3_split_config_git_blob_sha1 != expected_split_blob
    {
        return Err("license-audit upstream identity differs from the pinned PCBWorld split".into());
    }
    let synthetic = audit.evidence_class.as_deref() == Some("synthetic");
    if synthetic {
        if audit.status != "synthetic_test_only_eligible_fixture"
            || !benchmark_id.starts_with("synthetic-")
            || !dataset_id.starts_with("synthetic-")
        {
            return Err("synthetic license-audit evidence is permitted only for explicitly synthetic test-only datasets".into());
        }
    } else if audit.status != "approved_for_benchmark_execution" {
        return Err(format!(
            "license-audit status {:?} does not authorize benchmark execution",
            audit.status
        ));
    }
    let mut ids = BTreeSet::new();
    for board in &audit.boards {
        if board.board_id.trim().is_empty() || !ids.insert(board.board_id.as_str()) {
            return Err(format!("empty or duplicate board ID in license audit: {}", board.board_id));
        }
        validate_relative_path(&board.pcbench_directory, &format!("{} PCBench directory", board.board_id))?;
        validate_relative_path(&board.metadata_path, &format!("{} PCBench metadata path", board.board_id))?;
        if board.metadata_path != format!("PCBs/{}/metadata.json", board.pcbench_directory) {
            return Err(format!("{}: PCBench metadata path does not match the pinned dataset directory", board.board_id));
        }
        if !is_git_blob_sha1(&board.metadata_git_blob_sha1) {
            return Err(format!("{}: PCBench metadata Git blob SHA-1 is malformed", board.board_id));
        }
        if board.original_source_repository.trim().is_empty()
            || !board.original_source_repository.starts_with("https://")
        {
            return Err(format!("{}: audited source repository must use HTTPS", board.board_id));
        }
        if board.primary_source_license_review.trim().is_empty() {
            return Err(format!("{}: primary-source license review status is empty", board.board_id));
        }
    }
    Ok(())
}

fn validate_audit_manifest_for_manifest(
    audit_bytes: &[u8],
    manifest: &DatasetManifest,
) -> Result<LicenseAuditManifest, String> {
    let expected_audit_sha = manifest.license_audit_manifest_sha256.as_deref()
        .ok_or_else(|| "dataset manifest has no license-audit manifest SHA-256".to_string())?;
    if sha256_hex(audit_bytes) != expected_audit_sha {
        return Err("license-audit manifest bytes SHA-256 does not match the dataset manifest".into());
    }
    let audit: LicenseAuditManifest = serde_json::from_slice(audit_bytes)
        .map_err(|error| format!("license-audit manifest JSON/schema invalid: {error}"))?;
    validate_audit_envelope(
        &audit,
        &manifest.source_url,
        manifest.source_split_commit_sha.as_deref().unwrap_or_default(),
        manifest.source_split_config_git_blob_sha1.as_deref().unwrap_or_default(),
        &manifest.benchmark_id,
        &manifest.dataset_id,
    )?;
    let source_split_url = manifest.source_split_config_url.as_deref().unwrap_or_default();
    let source_split_commit = manifest.source_split_commit_sha.as_deref().unwrap_or_default();
    let expected_split_suffix = format!("/{source_split_commit}/{}", audit.upstream.d3_split_config_path);
    if !source_split_url.ends_with(&expected_split_suffix) {
        return Err("manifest source split-config URL path differs from the pinned license-audit path".into());
    }
    for pin in &manifest.boards {
        let record = audit.boards.iter()
            .find(|record| record.board_id == pin.board_id)
            .ok_or_else(|| format!("{} is missing from the pinned license audit", pin.board_id))?;
        let evidence = record.primary_source_license_evidence.as_ref()
            .ok_or_else(|| format!("{} has no primary-source license evidence", pin.board_id))?;
        let expected_status = if audit.evidence_class.as_deref() == Some("synthetic") {
            "eligible_for_synthetic_test_only"
        } else {
            "approved_for_benchmark_execution"
        };
        if record.benchmark_eligibility != expected_status {
            return Err(format!("{}: license-audit row is not eligible: {}", pin.board_id, record.benchmark_eligibility));
        }
        if pin.license != evidence.resolved_spdx_id
            || evidence.repository.trim_end_matches('/') != record.original_source_repository.trim_end_matches('/')
            || pin.source_url != record.original_source_repository
            || pin.original_source_repository.as_deref() != Some(record.original_source_repository.as_str())
            || pin.source_metadata_path.as_deref() != Some(record.metadata_path.as_str())
            || pin.source_metadata_git_blob_sha1.as_deref() != Some(record.metadata_git_blob_sha1.as_str())
            || pin.license_evidence_url.as_deref() != Some(evidence.blob_url.as_str())
            || pin.license_evidence_source_commit_sha.as_deref() != Some(evidence.source_commit_sha.as_str())
            || pin.license_evidence_git_blob_sha1.as_deref() != Some(evidence.git_blob_sha1.as_str())
            || pin.relative_path.as_deref() != record.prepared_board_relative_path.as_deref()
            || pin.input_board_sha256 != record.prepared_board_sha256.as_deref().unwrap_or_default()
        {
            return Err(format!("{}: dataset manifest identity/license fields disagree with the immutable audit row", pin.board_id));
        }
        validate_relative_path(&evidence.file_path, &format!("{} audited license file", pin.board_id))?;
        let expected_license_url = format!(
            "{}/blob/{}/{}",
            record.original_source_repository.trim_end_matches('/'),
            evidence.source_commit_sha,
            evidence.file_path
        );
        if !evidence.blob_hash_reverified_at_pinned_commit
            || !is_commit_sha(&evidence.source_commit_sha)
            || !is_git_blob_sha1(&evidence.git_blob_sha1)
            || evidence.blob_url != expected_license_url
        {
            return Err(format!("{}: immutable source-license evidence is malformed", pin.board_id));
        }
        let declared_spdx: Vec<&str> = record.pcbench_reported_licenses.iter()
            .filter_map(|license| license.spdx_id.as_deref())
            .collect();
        if record.license_match_status.as_deref() != Some("source_text_matches_pcbench_spdx_metadata")
            || (!declared_spdx.is_empty() && !declared_spdx.contains(&evidence.resolved_spdx_id.as_str()))
        {
            return Err(format!("{}: license evidence is not reconciled with the PCBench metadata record", pin.board_id));
        }
    }
    if audit.boards.iter().filter(|board| manifest.boards.iter().any(|pin| pin.board_id == board.board_id)).count() != manifest.boards.len() {
        return Err("manifest board set does not match the pinned license-audit record set".into());
    }
    Ok(audit)
}

pub fn verify_dataset_files(
    manifest_bytes: &[u8],
    dataset_root: &Path,
    evaluator_git_head: &str,
) -> Result<DatasetFileVerificationReceipt, String> {
    if !is_commit_sha(evaluator_git_head) {
        return Err("evaluator Git HEAD must be a 40- or 64-character hexadecimal commit ID".into());
    }
    let manifest: DatasetManifest = serde_json::from_slice(manifest_bytes)
        .map_err(|error| format!("dataset manifest JSON/schema invalid: {error}"))?;
    validate_manifest(&manifest)?;

    let root = dataset_root.canonicalize().map_err(|error| {
        format!("cannot canonicalize dataset root {}: {error}", dataset_root.display())
    })?;
    if !root.is_dir() {
        return Err(format!("dataset root is not a directory: {}", root.display()));
    }

    let mut entries = Vec::new();
    let mut used_paths = BTreeSet::new();
    let mut verified_license_evidence_count = 0usize;
    let mut audit_digest = None;

    if let Some(split_relative_path) = manifest.source_split_config_relative_path.as_deref() {
        validate_relative_path(split_relative_path, "source split config")?;
        let split_path = root.join(split_relative_path).canonicalize().map_err(|error| {
            format!("cannot resolve source split config {split_relative_path}: {error}")
        })?;
        if !split_path.starts_with(&root) || !split_path.is_file() {
            return Err("source split config is not a regular file contained by the dataset root".into());
        }
        if !used_paths.insert(split_path.clone()) {
            return Err("source split config aliases another pinned dataset artifact".into());
        }
        let split_bytes = fs::read(&split_path)
            .map_err(|error| format!("cannot read source split config {split_relative_path}: {error}"))?;
        let expected_sha = manifest.source_split_config_sha256.as_deref()
            .ok_or_else(|| "manifest source split-config SHA-256 is missing".to_string())?;
        if sha256_hex(&split_bytes) != expected_sha {
            return Err("source split-config bytes SHA-256 does not match the pinned manifest".into());
        }
        let expected_git_blob = manifest.source_split_config_git_blob_sha1.as_deref()
            .ok_or_else(|| "manifest source split-config Git blob SHA-1 is missing".to_string())?;
        if git_blob_sha1(&split_bytes) != expected_git_blob {
            return Err("source split-config bytes do not match the immutable source repository Git blob".into());
        }
        entries.push(DatasetFileVerificationEntry {
            board_id: "__dataset__".into(),
            role: "source_split_config".into(),
            relative_path: split_relative_path.to_string(),
            byte_len: split_bytes.len() as u64,
            sha256: expected_sha.to_string(),
            git_blob_sha1: Some(expected_git_blob.to_string()),
        });
    } else if manifest.evidence_class == "external_benchmark" {
        return Err("external manifest lacks source split-config path".into());
    }

    if let Some(audit_relative_path) = manifest.license_audit_manifest_relative_path.as_deref() {
        validate_relative_path(audit_relative_path, "license-audit manifest")?;
        let audit_path = root.join(audit_relative_path).canonicalize().map_err(|error| {
            format!("cannot resolve license-audit manifest {audit_relative_path}: {error}")
        })?;
        if !audit_path.starts_with(&root) || !audit_path.is_file() {
            return Err("license-audit manifest is not a regular file contained by the dataset root".into());
        }
        if !used_paths.insert(audit_path.clone()) {
            return Err("license-audit manifest aliases another pinned dataset artifact".into());
        }
        let audit_bytes = fs::read(&audit_path)
            .map_err(|error| format!("cannot read license-audit manifest bytes: {error}"))?;
        let audit_sha = sha256_hex(&audit_bytes);
        if audit_sha != manifest.license_audit_manifest_sha256.as_deref().unwrap_or_default() {
            return Err("license-audit manifest bytes SHA-256 does not match the dataset manifest".into());
        }
        validate_audit_manifest_for_manifest(&audit_bytes, &manifest)?;
        entries.push(DatasetFileVerificationEntry {
            board_id: "__dataset__".into(),
            role: "license_audit_manifest".into(),
            relative_path: audit_relative_path.to_string(),
            byte_len: audit_bytes.len() as u64,
            sha256: audit_sha.clone(),
            git_blob_sha1: None,
        });
        audit_digest = Some(audit_sha);
    } else if manifest.evidence_class == "external_benchmark" {
        return Err("external manifest lacks immutable license-audit manifest".into());
    }

    for board in &manifest.boards {
        let relative_path = board.relative_path.as_deref()
            .ok_or_else(|| format!("{}: dataset-relative source path missing", board.board_id))?;
        validate_relative_path(relative_path, &board.board_id)?;
        let actual_path = root.join(relative_path).canonicalize().map_err(|error| {
            format!("{}: cannot resolve source file {}: {error}", board.board_id, relative_path)
        })?;
        if !actual_path.starts_with(&root) || !actual_path.is_file() {
            return Err(format!("{}: source file is not a regular file contained by the dataset root", board.board_id));
        }
        if !used_paths.insert(actual_path.clone()) {
            return Err(format!("{}: source path aliases another pinned dataset artifact", board.board_id));
        }
        let board_bytes = fs::read(&actual_path)
            .map_err(|error| format!("{}: cannot read source file: {error}", board.board_id))?;
        let board_sha = sha256_hex(&board_bytes);
        if board_sha != board.input_board_sha256 {
            return Err(format!("{}: source bytes SHA-256 does not match the pinned manifest", board.board_id));
        }
        let computed_skeleton = crate::design_skeleton::fingerprint_board_bytes(&board_bytes)?;
        if let Some(expected_skeleton) = board.design_skeleton_sha256.as_deref() {
            if computed_skeleton.design_skeleton_sha256 != expected_skeleton {
                return Err(format!("{}: source design-skeleton SHA-256 does not match the pinned manifest", board.board_id));
            }
        } else if manifest.evidence_class == "external_benchmark" {
            return Err(format!("{}: external source board lacks a design-skeleton fingerprint", board.board_id));
        }
        entries.push(DatasetFileVerificationEntry {
            board_id: board.board_id.clone(),
            role: "source_board".into(),
            relative_path: relative_path.to_string(),
            byte_len: board_bytes.len() as u64,
            sha256: board_sha,
            git_blob_sha1: None,
        });

        if board.license_evidence_relative_path.is_some() || manifest.evidence_class == "external_benchmark" {
            let evidence_relative_path = board.license_evidence_relative_path.as_deref()
                .ok_or_else(|| format!("{}: license-evidence path is missing", board.board_id))?;
            validate_relative_path(evidence_relative_path, &format!("{} license evidence", board.board_id))?;
            let expected_evidence_sha = board.license_evidence_sha256.as_deref()
                .ok_or_else(|| format!("{}: license-evidence SHA-256 is missing", board.board_id))?;
            require_sha256(expected_evidence_sha, "license_evidence_sha256")?;
            let evidence_path = root.join(evidence_relative_path).canonicalize().map_err(|error| {
                format!("{}: cannot resolve license-evidence file {}: {error}", board.board_id, evidence_relative_path)
            })?;
            if !evidence_path.starts_with(&root) || !evidence_path.is_file() {
                return Err(format!("{}: license-evidence path is not a regular file contained by the dataset root", board.board_id));
            }
            if !used_paths.insert(evidence_path.clone()) {
                return Err(format!("{}: license-evidence path aliases another pinned dataset artifact", board.board_id));
            }
            let evidence_bytes = fs::read(&evidence_path)
                .map_err(|error| format!("{}: cannot read license-evidence file: {error}", board.board_id))?;
            if evidence_bytes.is_empty() {
                return Err(format!("{}: license-evidence file is empty", board.board_id));
            }
            let evidence_sha = sha256_hex(&evidence_bytes);
            if evidence_sha != expected_evidence_sha {
                return Err(format!("{}: license-evidence bytes SHA-256 does not match the pinned manifest", board.board_id));
            }
            let expected_git_blob = board.license_evidence_git_blob_sha1.as_deref()
                .ok_or_else(|| format!("{}: license-evidence Git blob SHA-1 is missing", board.board_id))?;
            let observed_git_blob = git_blob_sha1(&evidence_bytes);
            if observed_git_blob != expected_git_blob {
                return Err(format!("{}: license-evidence bytes do not match the immutable source repository Git blob", board.board_id));
            }
            entries.push(DatasetFileVerificationEntry {
                board_id: board.board_id.clone(),
                role: "license_evidence".into(),
                relative_path: evidence_relative_path.to_string(),
                byte_len: evidence_bytes.len() as u64,
                sha256: evidence_sha,
                git_blob_sha1: Some(observed_git_blob),
            });
            verified_license_evidence_count += 1;
        }
    }

    entries.sort_by(|a, b| a.board_id.cmp(&b.board_id)
        .then_with(|| a.role.cmp(&b.role))
        .then_with(|| a.relative_path.cmp(&b.relative_path)));
    Ok(DatasetFileVerificationReceipt {
        schema: "pcb-route-dataset-file-verification-v0.1".into(),
        evaluator_git_head: evaluator_git_head.to_ascii_lowercase(),
        dataset_manifest_sha256: sha256_hex(manifest_bytes),
        license_audit_manifest_sha256: audit_digest,
        verified_board_count: manifest.boards.len(),
        verified_license_evidence_count,
        entries,
        authority: "dataset_source_and_license_audit_and_evidence_bytes_match_pinned_manifest_only".into(),
        claim_ceiling: "This receipt verifies source-board, immutable license-audit and license-evidence bytes against a manifest. It does not determine legal applicability, route boards, authorize a benchmark score or establish physical/manufacturing quality.".into(),
    })
}

pub fn verify_run_artifacts(
    report_bytes: &[u8],
    manifest_bytes: &[u8],
    run_root: &Path,
    evaluator_git_head: &str,
) -> Result<RunArtifactVerificationReceipt, String> {
    // Validate report identity, board coverage, and all declared metrics first.
    let validated = evaluate_report_bytes(report_bytes, manifest_bytes, evaluator_git_head)?;
    let root = run_root.canonicalize().map_err(|error| {
        format!(
            "cannot canonicalize run-artifact root {}: {error}",
            run_root.display()
        )
    })?;
    if !root.is_dir() {
        return Err(format!(
            "run-artifact root is not a directory: {}",
            root.display()
        ));
    }

    let mut seen_relative = BTreeSet::new();
    let mut seen_files = BTreeSet::new();
    let mut entries = Vec::new();

    for case in &validated.report.cases {
        let is_completed = case.execution_status == ExecutionStatus::Completed;
        let pairs = [
            (
                "output_board",
                case.output_board_relative_path.as_deref(),
                case.output_board_sha256.as_deref(),
            ),
            (
                "drc_report",
                case.drc_report_relative_path.as_deref(),
                case.drc_report_sha256.as_deref(),
            ),
        ];

        for (role, relative_path, expected_sha256) in pairs {
            if relative_path.is_none() && expected_sha256.is_none() {
                if is_completed {
                    return Err(format!(
                        "{}: completed result lacks {role} path and SHA-256",
                        case.board_id
                    ));
                }
                continue;
            }
            let relative_path = relative_path.ok_or_else(|| {
                format!(
                    "{}: {role} digest is supplied but its file path is missing",
                    case.board_id
                )
            })?;
            let expected_sha256 = expected_sha256.ok_or_else(|| {
                format!(
                    "{}: {role} path is supplied but its SHA-256 is missing",
                    case.board_id
                )
            })?;
            validate_artifact_relative_path(relative_path, &case.board_id)?;
            require_sha256(
                expected_sha256,
                &format!("{} {role} SHA-256", case.board_id),
            )?;

            if !seen_relative.insert(relative_path.to_string()) {
                return Err(format!("duplicate run-artifact path: {relative_path}"));
            }
            let candidate = root.join(relative_path);
            let actual_path = candidate.canonicalize().map_err(|error| {
                format!(
                    "{}: cannot resolve {role} file {}: {error}",
                    case.board_id, relative_path
                )
            })?;
            if !actual_path.starts_with(&root) {
                return Err(format!(
                    "{}: {role} file resolves outside the run-artifact root",
                    case.board_id
                ));
            }
            if !actual_path.is_file() {
                return Err(format!(
                    "{}: {role} path is not a regular file: {relative_path}",
                    case.board_id
                ));
            }
            if !seen_files.insert(actual_path.clone()) {
                return Err(format!(
                    "multiple run-artifact roles resolve to the same file: {relative_path}"
                ));
            }

            let bytes = fs::read(&actual_path)
                .map_err(|error| format!("{}: cannot read {role} file: {error}", case.board_id))?;
            let actual_sha256 = sha256_hex(&bytes);
            if actual_sha256 != expected_sha256 {
                return Err(format!(
                    "{}: {role} file SHA-256 does not match the run report",
                    case.board_id
                ));
            }
            entries.push(RunArtifactVerificationEntry {
                board_id: case.board_id.clone(),
                role: role.to_string(),
                relative_path: relative_path.to_string(),
                byte_len: bytes.len() as u64,
                sha256: actual_sha256,
            });
        }

        if is_completed
            && (!case.drc_evaluated
                || case.output_board_sha256.is_none()
                || case.drc_report_sha256.is_none())
        {
            return Err(format!(
                "{}: completed attempt lacks required DRC/output evidence",
                case.board_id
            ));
        }
    }

    entries.sort_by(|a, b| {
        a.board_id
            .cmp(&b.board_id)
            .then_with(|| a.role.cmp(&b.role))
            .then_with(|| a.relative_path.cmp(&b.relative_path))
    });

    Ok(RunArtifactVerificationReceipt {
        schema: "pcb-route-run-artifact-verification-v0.1".into(),
        evaluator_git_head: evaluator_git_head.to_ascii_lowercase(),
        run_id: validated.report.run_id,
        input_report_sha256: sha256_hex(report_bytes),
        dataset_manifest_sha256: sha256_hex(manifest_bytes),
        verified_artifact_count: entries.len(),
        entries,
        authority: "run_report_paths_and_declared_output_digests_match_actual_file_bytes".into(),
        claim_ceiling: "This receipt proves that the declared output-board and DRC-report paths exist beneath the run root and match the report's SHA-256 values. It does not by itself prove the normalized DRC metrics were faithfully extracted, nor any physical quality, signal/power integrity, safety or production readiness.".into(),
    })
}

fn validate_artifact_relative_path(relative_path: &str, board_id: &str) -> Result<(), String> {
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
            "{board_id}: unsafe run-artifact-relative path: {relative_path}"
        ));
    }
    if Path::new(relative_path)
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "{board_id}: run-artifact path contains a non-normal component: {relative_path}"
        ));
    }
    Ok(())
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
        && (case.drc_report_sha256.is_some()
            || case.unconnected_item_count.is_some()
            || !case.routing_drc_violation_types.is_empty()
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
            if case.unconnected_item_count.is_none() {
                reasons.push("UNCONNECTED_ITEM_COUNT_MISSING".into());
            } else if case.unconnected_item_count.unwrap_or(0) > 0 {
                reasons.push("UNCONNECTED_ITEMS_REMAIN".into());
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
            } else if case.connected_nets < case.required_nets
                || case.unconnected_item_count.unwrap_or(0) > 0
            {
                Disposition::IncompleteNets
            } else if !case.drc_evaluated
                || case.unconnected_item_count.is_none()
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
        unconnected_item_count: case.unconnected_item_count,
        routing_drc_violation_types: case.routing_drc_violation_types.clone(),
        dangling_track_count: case.dangling_track_count,
        dangling_via_count: case.dangling_via_count,
        upstream_nonrouting_drc_before: case.upstream_nonrouting_drc_before,
        upstream_nonrouting_drc_after: case.upstream_nonrouting_drc_after,
        total_wirelength_mm: case.total_wirelength_mm,
        via_count: case.via_count,
        cpu_ms: case.cpu_ms,
        peak_memory_bytes: case.peak_memory_bytes,
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
    let cpu: Vec<f64> = cases
        .iter()
        .filter_map(|c| c.cpu_ms.map(|value| value as f64))
        .collect();
    let peak_memory: Vec<f64> = cases
        .iter()
        .filter_map(|c| c.peak_memory_bytes.map(|value| value as f64))
        .collect();
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
        reported_unconnected_item_total: cases
            .iter()
            .filter_map(|c| c.unconnected_item_count)
            .map(u64::from)
            .sum(),
        cases_missing_unconnected_item_count: cases
            .iter()
            .filter(|c| c.unconnected_item_count.is_none())
            .count(),
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
        cases_missing_cpu_ms: cases.iter().filter(|c| c.cpu_ms.is_none()).count(),
        median_cpu_ms_all_attempts: median(&cpu),
        p90_cpu_ms_all_attempts: percentile_nearest_rank(&cpu, 0.90),
        cases_missing_peak_memory_bytes: cases
            .iter()
            .filter(|c| c.peak_memory_bytes.is_none())
            .count(),
        median_peak_memory_bytes_all_attempts: median(&peak_memory),
        p90_peak_memory_bytes_all_attempts: percentile_nearest_rank(&peak_memory, 0.90),
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
    if baseline.report.seed != treatment.report.seed {
        return Err("cannot pair runs: declared seed differs".into());
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
            completion_sign_test_two_sided_p_value: exact_paired_sign_test_p_value(
                treatment_only,
                baseline_only,
            ),
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

pub fn compare_replicated_runs(
    baseline_runs: &[ValidatedRun],
    treatment_runs: &[ValidatedRun],
    evaluator_git_head: &str,
) -> Result<ReplicatedComparisonReceipt, String> {
    if !is_commit_sha(evaluator_git_head) {
        return Err(
            "evaluator Git HEAD must be a 40- or 64-character hexadecimal commit ID".into(),
        );
    }
    if baseline_runs.is_empty() || treatment_runs.is_empty() {
        return Err(
            "replicated comparison requires non-empty baseline and treatment series".into(),
        );
    }

    let baseline_by_seed = index_replicated_runs(baseline_runs, "baseline")?;
    let treatment_by_seed = index_replicated_runs(treatment_runs, "treatment")?;
    if baseline_by_seed.keys().ne(treatment_by_seed.keys()) {
        return Err("replicated comparison requires identical declared seed sets".into());
    }

    let baseline_first = *baseline_by_seed
        .values()
        .next()
        .expect("non-empty baseline");
    let treatment_first = *treatment_by_seed
        .values()
        .next()
        .expect("non-empty treatment");
    if baseline_first.receipt.router.method_id == treatment_first.receipt.router.method_id {
        return Err("baseline and treatment method IDs must differ".into());
    }
    if baseline_first.receipt.dataset_manifest_sha256
        != treatment_first.receipt.dataset_manifest_sha256
    {
        return Err("replicated comparison dataset manifest digest differs".into());
    }
    if baseline_first.receipt.benchmark_id != treatment_first.receipt.benchmark_id
        || baseline_first.receipt.benchmark_version != treatment_first.receipt.benchmark_version
        || baseline_first.receipt.dataset_id != treatment_first.receipt.dataset_id
        || baseline_first.receipt.split_id != treatment_first.receipt.split_id
        || baseline_first.receipt.drc_policy_sha256 != treatment_first.receipt.drc_policy_sha256
    {
        return Err("replicated comparison benchmark, split, or DRC policy differs".into());
    }
    if baseline_first.receipt.drc_engine != treatment_first.receipt.drc_engine {
        return Err("replicated comparison DRC engine pin differs".into());
    }
    if baseline_first.receipt.environment != treatment_first.receipt.environment {
        return Err("replicated comparison environment differs".into());
    }

    let mut seed_pairs = Vec::with_capacity(baseline_by_seed.len());
    for (seed, baseline) in &baseline_by_seed {
        let treatment = treatment_by_seed
            .get(seed)
            .copied()
            .ok_or_else(|| format!("treatment series lacks seed {seed}"))?;
        // Reuse the single-seed pairing gate so each seed has identical board IDs,
        // input digests, engine, DRC policy, environment and declared seed.
        compare_runs(baseline, treatment, evaluator_git_head)?;
        seed_pairs.push(SeedReportPin {
            seed: *seed,
            baseline_report_sha256: baseline.receipt.input_report_sha256.clone(),
            treatment_report_sha256: treatment.receipt.input_report_sha256.clone(),
        });
    }

    let board_ids: Vec<String> = baseline_first
        .receipt
        .cases
        .iter()
        .map(|case| case.board_id.clone())
        .collect();
    if board_ids.is_empty() {
        return Err("replicated comparison requires at least one board".into());
    }
    let seed_count = baseline_by_seed.len();
    let mut board_summaries = Vec::with_capacity(board_ids.len());
    let mut completion_cluster_deltas = Vec::with_capacity(board_ids.len());
    let mut elapsed_cluster_deltas = Vec::with_capacity(board_ids.len());
    let mut wirelength_cluster_deltas = Vec::new();
    let mut via_cluster_deltas = Vec::new();
    let mut all_elapsed_deltas = Vec::with_capacity(board_ids.len() * seed_count);
    let mut baseline_valid_attempts = 0usize;
    let mut treatment_valid_attempts = 0usize;
    let mut jointly_valid_attempts = 0usize;

    for board_id in &board_ids {
        let mut baseline_valid = 0usize;
        let mut treatment_valid = 0usize;
        let mut elapsed_deltas = Vec::with_capacity(seed_count);
        let mut wirelength_deltas = Vec::new();
        let mut via_deltas = Vec::new();

        for seed in baseline_by_seed.keys() {
            let baseline = baseline_by_seed[seed];
            let treatment = treatment_by_seed[seed];
            let baseline_case = baseline
                .receipt
                .cases
                .iter()
                .find(|case| case.board_id == *board_id)
                .ok_or_else(|| format!("baseline seed {seed} lacks board {board_id}"))?;
            let treatment_case = treatment
                .receipt
                .cases
                .iter()
                .find(|case| case.board_id == *board_id)
                .ok_or_else(|| format!("treatment seed {seed} lacks board {board_id}"))?;
            if baseline_case.input_board_sha256 != treatment_case.input_board_sha256
                || baseline_case.required_nets != treatment_case.required_nets
            {
                return Err(format!(
                    "paired input identity differs for board {board_id}"
                ));
            }

            baseline_valid += usize::from(baseline_case.valid_route_completion);
            treatment_valid += usize::from(treatment_case.valid_route_completion);
            let elapsed_delta = treatment_case.elapsed_ms as f64 - baseline_case.elapsed_ms as f64;
            elapsed_deltas.push(elapsed_delta);
            all_elapsed_deltas.push(elapsed_delta);

            if baseline_case.valid_route_completion && treatment_case.valid_route_completion {
                if let (Some(left), Some(right)) = (
                    baseline_case.total_wirelength_mm,
                    treatment_case.total_wirelength_mm,
                ) {
                    wirelength_deltas.push(right - left);
                }
                if let (Some(left), Some(right)) =
                    (baseline_case.via_count, treatment_case.via_count)
                {
                    via_deltas.push(right as f64 - left as f64);
                }
            }
        }

        baseline_valid_attempts += baseline_valid;
        treatment_valid_attempts += treatment_valid;
        jointly_valid_attempts += wirelength_deltas.len().min(via_deltas.len());

        let board_completion_delta =
            (treatment_valid as f64 - baseline_valid as f64) / seed_count as f64;
        let board_elapsed_delta = mean(&elapsed_deltas);
        completion_cluster_deltas.push(board_completion_delta);
        elapsed_cluster_deltas.push(board_elapsed_delta);

        let wirelength_delta = if wirelength_deltas.is_empty() {
            None
        } else {
            let value = mean(&wirelength_deltas);
            wirelength_cluster_deltas.push(value);
            Some(value)
        };
        let via_delta = if via_deltas.is_empty() {
            None
        } else {
            let value = mean(&via_deltas);
            via_cluster_deltas.push(value);
            Some(value)
        };
        board_summaries.push(ReplicatedBoardSummary {
            board_id: board_id.clone(),
            paired_attempt_count: seed_count,
            baseline_valid_attempts: baseline_valid,
            treatment_valid_attempts: treatment_valid,
            baseline_completion_rate: baseline_valid as f64 / seed_count as f64,
            treatment_completion_rate: treatment_valid as f64 / seed_count as f64,
            completion_delta_percentage_points: board_completion_delta * 100.0,
            mean_elapsed_delta_ms_treatment_minus_baseline: board_elapsed_delta,
            jointly_valid_attempts: wirelength_deltas.len().min(via_deltas.len()),
            mean_wirelength_delta_mm_treatment_minus_baseline: wirelength_delta,
            mean_via_delta_treatment_minus_baseline: via_delta,
        });
    }

    board_summaries.sort_by(|a, b| a.board_id.cmp(&b.board_id));
    let board_count = board_summaries.len();
    let paired_attempt_count = board_count * seed_count;
    let baseline_rate = baseline_valid_attempts as f64 / paired_attempt_count as f64;
    let treatment_rate = treatment_valid_attempts as f64 / paired_attempt_count as f64;
    let completion_treatment_better_boards = completion_cluster_deltas
        .iter()
        .filter(|delta| **delta > 0.0)
        .count();
    let completion_baseline_better_boards = completion_cluster_deltas
        .iter()
        .filter(|delta| **delta < 0.0)
        .count();
    let completion_tied_boards = completion_cluster_deltas
        .iter()
        .filter(|delta| **delta == 0.0)
        .count();
    let completion_sign_test_two_sided_p_value = exact_paired_sign_test_p_value(
        completion_treatment_better_boards,
        completion_baseline_better_boards,
    );

    let seed_material = seed_pairs
        .iter()
        .map(|pair| {
            format!(
                "{}:{}:{}",
                pair.seed, pair.baseline_report_sha256, pair.treatment_report_sha256
            )
        })
        .collect::<Vec<_>>()
        .join("|");
    let master_seed_sha = sha256_hex(
        format!(
            "{}|{}|{}|pcb-route-board-cluster-bootstrap-v0.1",
            baseline_first.receipt.dataset_manifest_sha256, seed_material, evaluator_git_head
        )
        .as_bytes(),
    );
    let (completion_seed, completion_seed_sha) =
        derive_bootstrap_seed(&master_seed_sha, "board_cluster_completion");
    let (elapsed_seed, elapsed_seed_sha) =
        derive_bootstrap_seed(&master_seed_sha, "board_cluster_elapsed_ms");
    let completion_ci = bootstrap_mean_ci(&completion_cluster_deltas, completion_seed);
    let elapsed_ci = bootstrap_mean_ci(&elapsed_cluster_deltas, elapsed_seed);

    let (wirelength_ci, wirelength_seed_sha) = if wirelength_cluster_deltas.is_empty() {
        (None, None)
    } else {
        let (seed, seed_sha) =
            derive_bootstrap_seed(&master_seed_sha, "board_cluster_wirelength_mm");
        (
            Some(bootstrap_mean_ci(&wirelength_cluster_deltas, seed)),
            Some(seed_sha),
        )
    };
    let (via_ci, via_seed_sha) = if via_cluster_deltas.is_empty() {
        (None, None)
    } else {
        let (seed, seed_sha) = derive_bootstrap_seed(&master_seed_sha, "board_cluster_vias");
        (
            Some(bootstrap_mean_ci(&via_cluster_deltas, seed)),
            Some(seed_sha),
        )
    };

    Ok(ReplicatedComparisonReceipt {
        schema: "pcb-route-replicated-comparison-v0.1".into(),
        evaluator_git_head: evaluator_git_head.to_ascii_lowercase(),
        dataset_manifest_sha256: baseline_first.receipt.dataset_manifest_sha256.clone(),
        baseline_method_id: baseline_first.receipt.router.method_id.clone(),
        treatment_method_id: treatment_first.receipt.router.method_id.clone(),
        seed_pairs,
        summary: ReplicatedSummary {
            board_count,
            seed_count,
            paired_attempt_count,
            baseline_valid_attempts,
            treatment_valid_attempts,
            baseline_completion_rate: baseline_rate,
            treatment_completion_rate: treatment_rate,
            completion_delta_percentage_points: (treatment_rate - baseline_rate) * 100.0,
            completion_treatment_better_boards,
            completion_baseline_better_boards,
            completion_tied_boards,
            completion_sign_test_two_sided_p_value,
            completion_delta_95_ci_board_clustered: ConfidenceInterval {
                lower_95: completion_ci.0 * 100.0,
                upper_95: completion_ci.1 * 100.0,
                resamples: BOOTSTRAP_RESAMPLES,
                bootstrap_seed_sha256: completion_seed_sha,
            },
            mean_elapsed_delta_ms_treatment_minus_baseline: mean(&elapsed_cluster_deltas),
            mean_elapsed_delta_95_ci_board_clustered_ms: ConfidenceInterval {
                lower_95: elapsed_ci.0,
                upper_95: elapsed_ci.1,
                resamples: BOOTSTRAP_RESAMPLES,
                bootstrap_seed_sha256: elapsed_seed_sha,
            },
            median_elapsed_delta_ms_per_attempt: median(&all_elapsed_deltas).unwrap_or(0.0),
            jointly_valid_attempts,
            mean_wirelength_delta_mm_treatment_minus_baseline: if wirelength_cluster_deltas.is_empty() {
                None
            } else {
                Some(mean(&wirelength_cluster_deltas))
            },
            wirelength_delta_95_ci_board_clustered_mm: interval_option(
                wirelength_ci,
                wirelength_seed_sha.as_deref().unwrap_or(&master_seed_sha),
            ),
            mean_via_delta_treatment_minus_baseline: if via_cluster_deltas.is_empty() {
                None
            } else {
                Some(mean(&via_cluster_deltas))
            },
            via_delta_95_ci_board_clustered: interval_option(
                via_ci,
                via_seed_sha.as_deref().unwrap_or(&master_seed_sha),
            ),
        },
        boards: board_summaries,
        authority: "board_clustered_paired_routing_metrics_only_no_physical_or_production_release_authority".into(),
        claim_ceiling: "Seed repetitions are paired within board and board IDs are the bootstrap cluster. This avoids treating repeated tries on one board as independent boards. The comparison is still valid only for the pinned dataset, rule policy, engine, environment and exact inputs; it proves no SI/PI, manufacturing or physical qualification.".into(),
    })
}

fn index_replicated_runs<'a>(
    runs: &'a [ValidatedRun],
    label: &str,
) -> Result<BTreeMap<u64, &'a ValidatedRun>, String> {
    let first = runs
        .first()
        .ok_or_else(|| format!("{label} series is empty"))?;
    let mut indexed = BTreeMap::new();

    for run in runs {
        let seed = run
            .report
            .seed
            .ok_or_else(|| format!("{label} series contains a run without a declared seed"))?;
        if run.receipt.router != first.receipt.router {
            return Err(format!("{label} series router pin changes between seeds"));
        }
        if run.receipt.benchmark_id != first.receipt.benchmark_id
            || run.receipt.benchmark_version != first.receipt.benchmark_version
            || run.receipt.dataset_id != first.receipt.dataset_id
            || run.receipt.split_id != first.receipt.split_id
            || run.receipt.dataset_manifest_sha256 != first.receipt.dataset_manifest_sha256
            || run.receipt.drc_policy_sha256 != first.receipt.drc_policy_sha256
            || run.receipt.drc_engine != first.receipt.drc_engine
            || run.receipt.environment != first.receipt.environment
        {
            return Err(format!("{label} series benchmark, dataset, DRC policy, engine or environment changes between seeds"));
        }
        if indexed.insert(seed, run).is_some() {
            return Err(format!("{label} series has duplicate seed {seed}"));
        }
    }

    Ok(indexed)
}

fn exact_paired_sign_test_p_value(positive: usize, negative: usize) -> Option<f64> {
    let trials = positive + negative;
    if trials == 0 {
        return None;
    }

    // Under the null, each discordant paired unit is equally likely to favour
    // either method. Calculate the two-sided exact Binomial(n, 0.5) tail.
    let lower_tail = positive.min(negative);
    let mut probability = 0.5_f64.powi(trials as i32);
    let mut cumulative = probability;
    for successes in 1..=lower_tail {
        probability *= (trials - successes + 1) as f64 / successes as f64;
        cumulative += probability;
    }
    Some((2.0 * cumulative).min(1.0))
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

fn interval_option(
    interval: Option<(f64, f64)>,
    bootstrap_seed_sha256: &str,
) -> Option<ConfidenceInterval> {
    interval.map(|(lower_95, upper_95)| ConfidenceInterval {
        lower_95,
        upper_95,
        resamples: BOOTSTRAP_RESAMPLES,
        bootstrap_seed_sha256: bootstrap_seed_sha256.to_string(),
    })
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

fn is_git_blob_sha1(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Compute Git's SHA-1 object identifier for a blob. SHA-256 remains the
/// primary content-integrity digest; this interoperates with upstream GitHub
/// blob IDs to prove that the copied license file matches the pinned source.
fn git_blob_sha1(bytes: &[u8]) -> String {
    let header = format!("blob {}\0", bytes.len());
    let mut hasher = Sha1::new();
    hasher.update(header.as_bytes());
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut output = String::with_capacity(40);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
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

    fn valid_run_with_seed(raw: &str, seed: u64) -> ValidatedRun {
        let mut value: serde_json::Value = serde_json::from_str(raw).expect("fixture JSON");
        value["dataset_manifest_sha256"] =
            serde_json::Value::String(sha256_hex(MANIFEST.as_bytes()));
        value["seed"] = serde_json::Value::from(seed);
        let original_id = value["run_id"].as_str().expect("run ID").to_string();
        value["run_id"] = serde_json::Value::String(format!("{original_id}-seed-{seed}"));
        let bytes = serde_json::to_vec(&value).expect("seeded fixture run");
        evaluate_report_bytes(&bytes, MANIFEST.as_bytes(), EVALUATOR_HEAD)
            .expect("seeded run validates")
    }

    #[test]
    fn dataset_intake_hashes_actual_bytes_and_rejects_path_escape() {
        let mut manifest: DatasetManifest = serde_json::from_str(MANIFEST).unwrap();
        manifest.evidence_class = "synthetic".into();
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
    fn pinned_split_manifest_builder_requires_the_immutable_license_audit() {
        let split_config = serde_json::json!({
            "board_filename": "processed_v9_guide_v3.kicad_pcb",
            "easy": { "test": ["board-easy-1", "board-easy-2"] },
            "hard": { "test": ["board-hard-1"] }
        });
        let split_config_bytes = serde_json::to_vec(&split_config).unwrap();
        let root = std::env::temp_dir().join(format!(
            "pcb-route-manifest-build-audit-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("provenance")).unwrap();
        std::fs::create_dir_all(root.join("license-evidence")).unwrap();
        std::fs::write(root.join("provenance/d3.json"), &split_config_bytes).unwrap();

        let evidence_blobs = [
            ("board-easy-1", "CC-BY-4.0", "license-evidence/board-easy-1.txt", "a"),
            ("board-hard-1", "MIT", "license-evidence/board-hard-1.txt", "b"),
        ];
        let mut audit_boards = Vec::new();
        for (id, spdx, evidence_path, commit_char) in evidence_blobs {
            std::fs::create_dir_all(root.join(id)).unwrap();
            let board_relative = format!("{id}/processed_v9_guide_v3.kicad_pcb");
            let board_bytes = format!(r#"(kicad_pcb (version 20241229) (generator "pcbnew") (layers (0 "F.Cu" signal) (25 "Edge.Cuts" user)) (net 0 "") (net 1 "VCC") (footprint "AtomFab:Connector" (layer "F.Cu") (at 10 10) (property "Reference" "{id}") (pad "1" thru_hole rect (at 0 0) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask") (net 1 "VCC"))) (gr_rect (start 0 0) (end 20 10) (layer "Edge.Cuts")))"#);
            std::fs::write(root.join(&board_relative), board_bytes.as_bytes()).unwrap();
            let license_bytes = format!("SYNTHETIC license-evidence fixture text for {spdx}\n");
            std::fs::write(root.join(evidence_path), license_bytes.as_bytes()).unwrap();
            let source_repo = format!("https://github.com/example/{id}");
            let commit = commit_char.repeat(40);
            audit_boards.push(serde_json::json!({
                "board_id": id,
                "difficulty": if id == "board-easy-1" { "easy" } else { "hard" },
                "pcbench_directory": id,
                "metadata_path": format!("PCBs/{id}/metadata.json"),
                "metadata_git_blob_sha1": "9".repeat(40),
                "original_source_repository": source_repo,
                "pcbench_reported_licenses": [{"spdx_id": spdx, "name": spdx}],
                "primary_source_license_review": "synthetic_test_only_not_legal_review",
                "benchmark_eligibility": "eligible_for_synthetic_test_only",
                "primary_source_license_evidence": {
                    "repository": source_repo,
                    "file_path": "LICENSE",
                    "git_blob_sha1": git_blob_sha1(license_bytes.as_bytes()),
                    "resolved_spdx_id": spdx,
                    "blob_url": format!("{source_repo}/blob/{commit}/LICENSE"),
                    "source_commit_sha": commit,
                    "blob_hash_reverified_at_pinned_commit": true
                },
                "license_match_status": "source_text_matches_pcbench_spdx_metadata",
                "prepared_board_relative_path": board_relative,
                "prepared_board_sha256": sha256_hex(board_bytes.as_bytes())
            }));
        }
        let audit_value = serde_json::json!({
            "schema": "pcbworld-pilot-source-metadata-audit-v0.1",
            "status": "synthetic_test_only_eligible_fixture",
            "evidence_class": "synthetic",
            "upstream": {
                "pcbworld_repository": "https://github.com/LGAI-Research/PCBWorld",
                "pcbworld_commit_sha": "c".repeat(40),
                "d3_split_config_path": "configs/datasets/d3.json",
                "d3_split_config_git_blob_sha1": git_blob_sha1(&split_config_bytes),
                "pcbench_repository": "https://github.com/PCBench/PCBench",
                "pcbench_commit_sha": "d".repeat(40)
            },
            "boards": audit_boards
        });
        let audit_bytes = serde_json::to_vec_pretty(&audit_value).unwrap();
        std::fs::write(root.join("provenance/license-audit.json"), &audit_bytes).unwrap();

        let intake = IntakeSpec {
            schema: "pcb-route-intake-spec-v0.2".into(),
            benchmark_id: "synthetic-pcbworld-bench".into(),
            benchmark_version: "v1.1.1-fixture".into(),
            dataset_id: "synthetic-pcbench-d3".into(),
            split_id: "synthetic-d3-test-pilot-v0.1".into(),
            source_url: "https://github.com/LGAI-Research/PCBWorld".into(),
            source_split_commit_sha: "c".repeat(40),
            source_split_config_url: format!(
                "https://github.com/LGAI-Research/PCBWorld/blob/{}/configs/datasets/d3.json",
                "c".repeat(40)
            ),
            source_split_config_git_blob_sha1: git_blob_sha1(&split_config_bytes),
            source_split_config_relative_path: "provenance/d3.json".into(),
            license_audit_manifest_relative_path: "provenance/license-audit.json".into(),
            license_audit_manifest_sha256: sha256_hex(&audit_bytes),
            split_name: "test".into(),
            boards: vec![
                IntakeBoard {
                    board_id: "board-easy-1".into(),
                    difficulty: "easy".into(),
                    required_nets: 4,
                    license_evidence_relative_path: "license-evidence/board-easy-1.txt".into(),
                },
                IntakeBoard {
                    board_id: "board-hard-1".into(),
                    difficulty: "hard".into(),
                    required_nets: 9,
                    license_evidence_relative_path: "license-evidence/board-hard-1.txt".into(),
                },
            ],
        };
        let intake_bytes = serde_json::to_vec(&intake).unwrap();
        let manifest = build_dataset_manifest(&split_config_bytes, &intake_bytes, &root)
            .expect("synthetic audit fixture builds a synthetic-only manifest");
        assert_eq!(manifest.boards.len(), 2);
        assert_eq!(manifest.evidence_class, "synthetic");
        assert_eq!(manifest.license_audit_manifest_sha256.as_deref(), Some(sha256_hex(&audit_bytes).as_str()));
        assert!(manifest.boards.iter().all(|board| board.license_evidence_git_blob_sha1.is_some()));

        let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
        let receipt = verify_dataset_files(&manifest_bytes, &root, EVALUATOR_HEAD)
            .expect("source, audit, prepared-board and license evidence bytes verify");
        assert_eq!(receipt.verified_board_count, 2);
        assert_eq!(receipt.verified_license_evidence_count, 2);
        assert_eq!(receipt.entries.len(), 6);
        assert_eq!(receipt.license_audit_manifest_sha256.as_deref(), Some(sha256_hex(&audit_bytes).as_str()));
        assert_eq!(receipt.entries.iter().filter(|entry| entry.role == "license_audit_manifest").count(), 1);

        // The currently published 30-board audit is metadata-only and marks
        // every row unauthorised; caller-supplied labels cannot override it.
        let actual_audit: LicenseAuditManifest = serde_json::from_slice(
            include_str!("../fixtures/pcbworld-d3-pilot-source-metadata-audit-v0_1.json").as_bytes()
        ).expect("published audit fixture");
        assert_eq!(actual_audit.status, "metadata_inventory_only_not_benchmark_manifest");
        assert!(actual_audit.boards.iter().all(|row| row.benchmark_eligibility != "approved_for_benchmark_execution"));
        let blocked = validate_audit_envelope(
            &actual_audit,
            "https://github.com/LGAI-Research/PCBWorld",
            "1989a9325796b73cca793a0c8f222dd1509256e8",
            "61d86bbf6cfd8ff8d1e13089c8d98a90f6f81bb3",
            "pcbworld-bench",
            "pcbench-d3"
        );
        assert!(blocked.unwrap_err().contains("does not authorize benchmark execution"));

        // Stale audit hash, unapproved row, unknown board, changed license
        // blob, and changed prepared-board digest all fail closed.
        let mut stale_intake = intake.clone();
        stale_intake.license_audit_manifest_sha256 = "0".repeat(64);
        assert!(build_dataset_manifest(&split_config_bytes, &serde_json::to_vec(&stale_intake).unwrap(), &root)
            .unwrap_err().contains("license-audit manifest bytes SHA-256"));

        let mut unapproved = audit_value.clone();
        unapproved["boards"][0]["benchmark_eligibility"] = serde_json::Value::String("not_authorized_pending_review".into());
        let unapproved_bytes = serde_json::to_vec_pretty(&unapproved).unwrap();
        std::fs::write(root.join("provenance/license-audit.json"), &unapproved_bytes).unwrap();
        let mut changed = intake.clone();
        changed.license_audit_manifest_sha256 = sha256_hex(&unapproved_bytes);
        assert!(build_dataset_manifest(&split_config_bytes, &serde_json::to_vec(&changed).unwrap(), &root)
            .unwrap_err().contains("license-audit eligibility"));
        std::fs::write(root.join("provenance/license-audit.json"), &audit_bytes).unwrap();

        let mut unlisted = intake.clone();
        unlisted.boards[0].board_id = "not-in-audit".into();
        assert!(build_dataset_manifest(&split_config_bytes, &serde_json::to_vec(&unlisted).unwrap(), &root)
            .unwrap_err().contains("not listed in the pinned license-audit"));

        let mut changed_repo = audit_value.clone();
        changed_repo["boards"][0]["original_source_repository"] = serde_json::Value::String("https://github.com/example/changed".into());
        let changed_repo_bytes = serde_json::to_vec_pretty(&changed_repo).unwrap();
        std::fs::write(root.join("provenance/license-audit.json"), &changed_repo_bytes).unwrap();
        changed = intake.clone();
        changed.license_audit_manifest_sha256 = sha256_hex(&changed_repo_bytes);
        assert!(build_dataset_manifest(&split_config_bytes, &serde_json::to_vec(&changed).unwrap(), &root)
            .unwrap_err().contains("disagrees with the audited source repository"));
        std::fs::write(root.join("provenance/license-audit.json"), &audit_bytes).unwrap();

        let mut wrong_blob = audit_value.clone();
        wrong_blob["boards"][0]["primary_source_license_evidence"]["git_blob_sha1"] = serde_json::Value::String("0".repeat(40));
        let wrong_blob_bytes = serde_json::to_vec_pretty(&wrong_blob).unwrap();
        std::fs::write(root.join("provenance/license-audit.json"), &wrong_blob_bytes).unwrap();
        changed = intake.clone();
        changed.license_audit_manifest_sha256 = sha256_hex(&wrong_blob_bytes);
        assert!(build_dataset_manifest(&split_config_bytes, &serde_json::to_vec(&changed).unwrap(), &root)
            .unwrap_err().contains("license-evidence bytes do not match audit-pinned"));
        std::fs::write(root.join("provenance/license-audit.json"), &audit_bytes).unwrap();

        let mut wrong_board = audit_value.clone();
        wrong_board["boards"][0]["prepared_board_sha256"] = serde_json::Value::String("0".repeat(64));
        let wrong_board_bytes = serde_json::to_vec_pretty(&wrong_board).unwrap();
        std::fs::write(root.join("provenance/license-audit.json"), &wrong_board_bytes).unwrap();
        changed = intake.clone();
        changed.license_audit_manifest_sha256 = sha256_hex(&wrong_board_bytes);
        assert!(build_dataset_manifest(&split_config_bytes, &serde_json::to_vec(&changed).unwrap(), &root)
            .unwrap_err().contains("source-board bytes SHA-256"));
        std::fs::write(root.join("provenance/license-audit.json"), &audit_bytes).unwrap();

        let mut bad_path = intake.clone();
        bad_path.boards[0].license_evidence_relative_path = "../outside.txt".into();
        assert!(build_dataset_manifest(&split_config_bytes, &serde_json::to_vec(&bad_path).unwrap(), &root)
            .unwrap_err().contains("traversal"));

        let mut alias = intake.clone();
        alias.boards[1].license_evidence_relative_path = alias.boards[0].license_evidence_relative_path.clone();
        assert!(build_dataset_manifest(&split_config_bytes, &serde_json::to_vec(&alias).unwrap(), &root)
            .unwrap_err().contains("aliases another pinned dataset artifact"));

        std::fs::write(root.join("provenance/d3.json"), b"{\"tampered\":true}\n").unwrap();
        assert!(verify_dataset_files(&manifest_bytes, &root, EVALUATOR_HEAD)
            .unwrap_err().contains("source split-config bytes SHA-256"));
        std::fs::write(root.join("provenance/d3.json"), &split_config_bytes).unwrap();
        std::fs::write(root.join("license-evidence/board-easy-1.txt"), b"modified after manifest creation\n").unwrap();
        assert!(verify_dataset_files(&manifest_bytes, &root, EVALUATOR_HEAD)
            .unwrap_err().contains("license-evidence bytes SHA-256"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn external_manifest_rejects_unverified_license_and_non_https_source() {
        let mut manifest: DatasetManifest = serde_json::from_str(MANIFEST).unwrap();
        manifest.schema = MANIFEST_SCHEMA.into();
        manifest.evidence_class = "external_benchmark".into();
        manifest.source_split_config_sha256 = Some("a".repeat(64));
        manifest.source_split_config_url = Some(format!(
            "https://github.com/example/PCBWorld/blob/{}/configs/datasets/d3.json",
            "b".repeat(40)
        ));
        manifest.source_split_config_git_blob_sha1 = Some("e".repeat(40));
        manifest.source_split_config_relative_path = Some("provenance/d3.json".into());
        manifest.source_split_commit_sha = Some("b".repeat(40));
        manifest.source_split_name = Some("test".into());
        manifest.intake_metadata_sha256 = Some("c".repeat(64));
        manifest.license_audit_manifest_relative_path = Some("provenance/license-audit.json".into());
        manifest.license_audit_manifest_sha256 = Some("d".repeat(64));

        // External boards must declare a source identity and immutable PCBench metadata pin.
        for board in &mut manifest.boards {
            board.relative_path = Some(format!("boards/{}.kicad_pcb", board.board_id));
            board.original_source_repository = Some(format!("https://example.invalid/{}", board.board_id));
            board.source_metadata_path = Some(format!("PCBs/{}/metadata.json", board.board_id));
            board.source_metadata_git_blob_sha1 = Some("a".repeat(40));
            board.source_url = board.original_source_repository.clone().unwrap();
            board.license = "unknown".into();
        }
        assert!(validate_manifest(&manifest)
            .unwrap_err()
            .contains("license must be explicitly verified"));

        for board in &mut manifest.boards {
            board.license = "CC-BY-4.0".into();
            board.source_url = format!("http://example.invalid/{}", board.board_id);
            board.original_source_repository = Some(board.source_url.clone());
        }
        assert!(validate_manifest(&manifest)
            .unwrap_err()
            .contains("source_url must use HTTPS"));

        for board in &mut manifest.boards {
            board.source_url = format!("https://example.invalid/{}", board.board_id);
            board.original_source_repository = Some(board.source_url.clone());
            board.license_evidence_relative_path = None;
            board.license_evidence_url = None;
            board.license_evidence_sha256 = None;
            board.license_evidence_source_commit_sha = None;
            board.license_evidence_git_blob_sha1 = None;
        }
        assert!(validate_manifest(&manifest)
            .unwrap_err()
            .contains("hash-bound license-evidence path"));

        // A moving or mismatched source-license URL must not satisfy the immutable source pin.
        for board in &mut manifest.boards {
            board.license_evidence_relative_path =
                Some(format!("license-evidence/{}.txt", board.board_id));
            board.license_evidence_sha256 = Some("f".repeat(64));
            board.license_evidence_source_commit_sha = Some("d".repeat(40));
            board.license_evidence_url = Some(format!(
                "https://github.com/example/{}/blob/{}/LICENSE",
                board.board_id,
                "e".repeat(40)
            ));
        }
        assert!(validate_manifest(&manifest)
            .unwrap_err()
            .contains("URL must contain the pinned immutable source commit"));

        for board in &mut manifest.boards {
            board.license_evidence_url = Some(format!(
                "https://github.com/example/{}/blob/{}/LICENSE",
                board.board_id,
                "d".repeat(40)
            ));
            board.license_evidence_git_blob_sha1 = Some("a".repeat(40));
        }
        assert!(validate_manifest(&manifest).is_ok());

        manifest.boards[0].license_evidence_source_commit_sha = None;
        assert!(validate_manifest(&manifest)
            .unwrap_err()
            .contains("immutable license-source commit"));
    }

    #[test]
    fn frozen_synthetic_fixture_has_full_denominator_and_expected_baseline() {
        let baseline = valid_run(BASELINE);
        assert_eq!(baseline.receipt.summary.cases_attempted, 6);
        assert_eq!(baseline.receipt.summary.valid_completions, 2);
        assert!((baseline.receipt.summary.completion_rate - 2.0 / 6.0).abs() < 1e-9);
        assert_eq!(baseline.receipt.summary.timeout_cases, 1);
        assert_eq!(baseline.receipt.summary.tool_error_cases, 1);
        assert_eq!(baseline.receipt.summary.reported_unconnected_item_total, 1);
        assert_eq!(
            baseline
                .receipt
                .summary
                .cases_missing_unconnected_item_count,
            2
        );
        assert_eq!(baseline.receipt.summary.cases_missing_cpu_ms, 2);
        assert_eq!(baseline.receipt.summary.cases_missing_peak_memory_bytes, 2);
        let easy = baseline
            .receipt
            .cases
            .iter()
            .find(|case| case.board_id == "easy-001")
            .expect("easy board case");
        assert_eq!(easy.cpu_ms, Some(90));
        assert_eq!(easy.peak_memory_bytes, Some(200_000_000));
        assert_eq!(
            baseline.receipt.summary.median_cpu_ms_all_attempts,
            Some(130.0)
        );
        assert_eq!(
            baseline.receipt.summary.p90_cpu_ms_all_attempts,
            Some(190.0)
        );
        assert_eq!(
            baseline
                .receipt
                .summary
                .median_peak_memory_bytes_all_attempts,
            Some(205_000_000.0)
        );
        assert_eq!(
            baseline.receipt.summary.p90_peak_memory_bytes_all_attempts,
            Some(220_000_000.0)
        );
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
    fn connected_net_summary_with_native_unconnected_items_is_not_a_valid_route() {
        let mut value: serde_json::Value = serde_json::from_str(BASELINE).unwrap();
        value["dataset_manifest_sha256"] =
            serde_json::Value::String(sha256_hex(MANIFEST.as_bytes()));
        value["cases"][0]["connected_nets"] = serde_json::Value::from(2);
        value["cases"][0]["required_nets"] = serde_json::Value::from(2);
        value["cases"][0]["unconnected_item_count"] = serde_json::Value::from(1);
        let bytes = serde_json::to_vec(&value).unwrap();
        let run = evaluate_report_bytes(&bytes, MANIFEST.as_bytes(), EVALUATOR_HEAD).unwrap();
        let case = run
            .receipt
            .cases
            .iter()
            .find(|c| c.board_id == "easy-001")
            .unwrap();
        assert_eq!(case.disposition, Disposition::IncompleteNets);
        assert!(case
            .reason_codes
            .contains(&"UNCONNECTED_ITEMS_REMAIN".to_string()));
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
        assert!(
            (comparison
                .summary
                .completion_sign_test_two_sided_p_value
                .unwrap()
                - 0.5)
                .abs()
                < 1e-12
        );
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
    fn replicated_comparison_bootstraps_boards_not_repeated_attempts() {
        let seeds = [17_u64, 23, 31];
        let baseline: Vec<ValidatedRun> = seeds
            .iter()
            .map(|seed| valid_run_with_seed(BASELINE, *seed))
            .collect();
        let treatment: Vec<ValidatedRun> = seeds
            .iter()
            .map(|seed| valid_run_with_seed(TREATMENT, *seed))
            .collect();

        let comparison = compare_replicated_runs(&baseline, &treatment, EVALUATOR_HEAD).unwrap();
        assert_eq!(comparison.summary.board_count, 6);
        assert_eq!(comparison.summary.seed_count, 3);
        assert_eq!(comparison.summary.paired_attempt_count, 18);
        assert_eq!(comparison.summary.baseline_valid_attempts, 6);
        assert_eq!(comparison.summary.treatment_valid_attempts, 12);
        assert!((comparison.summary.baseline_completion_rate - 1.0 / 3.0).abs() < 1e-9);
        assert!((comparison.summary.treatment_completion_rate - 2.0 / 3.0).abs() < 1e-9);
        assert!((comparison.summary.completion_delta_percentage_points - 100.0 / 3.0).abs() < 1e-8);
        assert_eq!(
            comparison
                .summary
                .completion_delta_95_ci_board_clustered
                .resamples,
            10_000
        );
        assert_eq!(comparison.summary.completion_treatment_better_boards, 2);
        assert_eq!(comparison.summary.completion_baseline_better_boards, 0);
        assert_eq!(comparison.summary.completion_tied_boards, 4);
        assert!(
            (comparison
                .summary
                .completion_sign_test_two_sided_p_value
                .unwrap()
                - 0.5)
                .abs()
                < 1e-12
        );
        assert_eq!(comparison.boards.len(), 6);
        assert_eq!(comparison.seed_pairs.len(), 3);
        assert!(
            comparison
                .summary
                .completion_delta_95_ci_board_clustered
                .bootstrap_seed_sha256
                != comparison
                    .summary
                    .mean_elapsed_delta_95_ci_board_clustered_ms
                    .bootstrap_seed_sha256
        );

        let mismatched = vec![
            valid_run_with_seed(TREATMENT, 17),
            valid_run_with_seed(TREATMENT, 99),
            valid_run_with_seed(TREATMENT, 31),
        ];
        assert!(
            compare_replicated_runs(&baseline, &mismatched, EVALUATOR_HEAD)
                .unwrap_err()
                .contains("identical declared seed sets")
        );

        let duplicate = vec![
            valid_run_with_seed(BASELINE, 17),
            valid_run_with_seed(BASELINE, 17),
        ];
        assert!(index_replicated_runs(&duplicate, "baseline")
            .unwrap_err()
            .contains("duplicate seed"));
    }

    #[test]
    fn exact_sign_test_uses_discordant_pairs_and_reports_small_sample_uncertainty() {
        assert!(exact_paired_sign_test_p_value(0, 0).is_none());
        assert!((exact_paired_sign_test_p_value(1, 0).unwrap() - 1.0).abs() < 1e-12);
        assert!((exact_paired_sign_test_p_value(2, 0).unwrap() - 0.5).abs() < 1e-12);
        assert!((exact_paired_sign_test_p_value(10, 0).unwrap() - (2.0 / 1024.0)).abs() < 1e-12);
        assert!((exact_paired_sign_test_p_value(4, 4).unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn git_blob_sha1_matches_the_canonical_empty_blob_id() {
        assert_eq!(
            git_blob_sha1(b""),
            "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"
        );
        assert!(!is_git_blob_sha1(&"a".repeat(39)));
        assert!(is_git_blob_sha1(&"a".repeat(40)));
        assert!(!is_git_blob_sha1(&"b".repeat(64)));
    }

    #[test]
    fn commit_id_validation_supports_sha1_and_sha256_lengths() {
        assert!(is_commit_sha(&"a".repeat(40)));
        assert!(is_commit_sha(&"b".repeat(64)));
        assert!(!is_commit_sha(&"g".repeat(40)));
        assert!(!is_commit_sha(&"a".repeat(39)));
    }

    #[test]
    fn run_artifact_verifier_binds_actual_output_and_drc_bytes() {
        let mut value: serde_json::Value =
            serde_json::from_str(BASELINE).expect("baseline fixture");
        value["dataset_manifest_sha256"] =
            serde_json::Value::String(sha256_hex(MANIFEST.as_bytes()));
        let root =
            std::env::temp_dir().join(format!("pcb-route-run-artifacts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("outputs")).unwrap();
        std::fs::create_dir_all(root.join("drc")).unwrap();

        for case in value["cases"].as_array_mut().unwrap() {
            if case["execution_status"] != "completed" {
                case["output_board_relative_path"] = serde_json::Value::Null;
                case["drc_report_relative_path"] = serde_json::Value::Null;
                continue;
            }
            let board_id = case["board_id"].as_str().unwrap().to_string();
            let output_relative = format!("outputs/{board_id}.kicad_pcb");
            let drc_relative = format!("drc/{board_id}.json");
            let output_bytes = format!("synthetic output board bytes for {board_id}\n");
            let drc_bytes = format!("{{\"synthetic_fixture\":true,\"board_id\":\"{board_id}\"}}\n");
            std::fs::write(root.join(&output_relative), output_bytes.as_bytes()).unwrap();
            std::fs::write(root.join(&drc_relative), drc_bytes.as_bytes()).unwrap();
            case["output_board_sha256"] =
                serde_json::Value::String(sha256_hex(output_bytes.as_bytes()));
            case["drc_report_sha256"] = serde_json::Value::String(sha256_hex(drc_bytes.as_bytes()));
            case["output_board_relative_path"] = serde_json::Value::String(output_relative);
            case["drc_report_relative_path"] = serde_json::Value::String(drc_relative);
        }

        let report_bytes = serde_json::to_vec(&value).unwrap();
        let receipt =
            verify_run_artifacts(&report_bytes, MANIFEST.as_bytes(), &root, EVALUATOR_HEAD)
                .expect("all declared output/DRC bytes verify");
        assert_eq!(receipt.verified_artifact_count, 8);
        assert_eq!(receipt.entries.len(), 8);
        assert!(receipt.entries.iter().all(|entry| entry.byte_len > 0));
        assert_eq!(receipt.run_id, "synthetic-baseline-001");

        let tampered_path = root.join(&receipt.entries[0].relative_path);
        std::fs::write(&tampered_path, b"tampered after report creation").unwrap();
        assert!(
            verify_run_artifacts(&report_bytes, MANIFEST.as_bytes(), &root, EVALUATOR_HEAD)
                .unwrap_err()
                .contains("SHA-256 does not match")
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn run_artifact_verifier_rejects_traversal_and_path_aliases() {
        let mut value: serde_json::Value = serde_json::from_str(BASELINE).unwrap();
        value["dataset_manifest_sha256"] =
            serde_json::Value::String(sha256_hex(MANIFEST.as_bytes()));
        value["cases"][0]["output_board_relative_path"] =
            serde_json::Value::String("../outside.kicad_pcb".into());
        value["cases"][0]["drc_report_relative_path"] =
            serde_json::Value::String("drc/easy-001.json".into());
        let bytes = serde_json::to_vec(&value).unwrap();
        let root = std::env::temp_dir().join(format!(
            "pcb-route-run-artifacts-negative-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert!(
            verify_run_artifacts(&bytes, MANIFEST.as_bytes(), &root, EVALUATOR_HEAD)
                .unwrap_err()
                .contains("unsafe run-artifact-relative path")
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
