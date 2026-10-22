// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashSet};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_CORPUS_PATH: &str = "docs/tech/CCZ_ENVIRONMENTAL_OBSERVABILITY_CORPUS_V0_1.json";
const MAX_CORPUS_BYTES: usize = 1_000_000;
const SCHEMA_ID: &str = "luminous.symtropy.ccz-environmental-observability";
const SCHEMA_VERSION: &str = "0.1.0";
const SUITE_ID: &str = "CCZ-OBSERVABILITY-SYNTH-001";
const CASE_IDS: &[&str] = &[
    "track-impact-with-valid-baseline",
    "non-detection-bounded-to-scope",
    "missing-baseline",
    "incomparable-reference-control",
    "incomparable-sampling-methods",
    "stale-sensor-calibration",
    "unknown-detection-limits",
    "plume-at-incomplete-boundary",
    "recovery-claim-insufficient-followup",
    "cumulative-exposure-incomplete",
    "source-snapshot-mismatch",
    "duplicate-sample-identities",
    "ambiguous-impact-signal",
    "far-field-impact-preserves-scope",
    "empty-samples-unknown-control-and-coverage",
    "impact-and-recovery-are-separate",
    "cumulative-exposure-complete",
    "baseline-and-calibration-unknown",
    "blank-sample-identity",
];

type Check<T> = Result<T, String>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EvidenceIntegrity {
    Valid,
    InvalidEvidence,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Comparability {
    AdequateForDeclaredQuestion,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ImpactResult {
    ImpactEvidenceWithinScope,
    NoImpactDetectedWithinScope,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum RecoveryResult {
    NotClaimed,
    RecoveryNotEstablished,
    RecoveryEvidenceSufficient,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CumulativeExposureResult {
    NotClaimed,
    CompleteForDeclaredChannels,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum BaselineStatus {
    Available,
    Missing,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReferenceControlStatus {
    Comparable,
    Missing,
    Incomparable,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum SamplingMethodStatus {
    Comparable,
    Incomparable,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DetectionLimitStatus {
    Declared,
    Missing,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CalibrationStatus {
    Current,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum SpatialCoverage {
    AdequateForDeclaredQuestion,
    Incomplete,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ImpactSignal {
    Detected,
    NotDetected,
    Ambiguous,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum RecoveryFollowup {
    Sufficient,
    Insufficient,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ExposureCoverage {
    Complete,
    Incomplete,
    Unknown,
    NotAssessed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum SpatialScope {
    DirectTrack,
    NearField,
    FarField,
    Regional,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum ReasonCode {
    SourceSnapshotMismatch,
    DuplicateSampleIdentity,
    MissingSampleIdentity,
    SampleSetEmpty,
    BaselineMissing,
    BaselineUnknown,
    ReferenceControlMissing,
    ReferenceControlIncomparable,
    ReferenceControlUnknown,
    SamplingMethodsIncomparable,
    SamplingMethodsUnknown,
    DetectionLimitsMissing,
    DetectionLimitsUnknown,
    SensorCalibrationStale,
    SensorCalibrationUnknown,
    SpatialCoverageIncomplete,
    SpatialCoverageUnknown,
    PlumeAtMonitoringBoundary,
    ImpactSignalDetectedWithinScope,
    NoImpactDetectedWithinScope,
    ImpactSignalAmbiguous,
    RecoveryFollowupInsufficient,
    RecoveryFollowupUnknown,
    CumulativeSoundIncomplete,
    CumulativeSoundUnknown,
    CumulativeLightIncomplete,
    CumulativeLightUnknown,
    CumulativeSedimentIncomplete,
    CumulativeSedimentUnknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Evidence {
    source_snapshot_id: String,
    expected_source_snapshot_id: String,
    sample_ids: Vec<String>,
    baseline: BaselineStatus,
    reference_control: ReferenceControlStatus,
    sampling_methods: SamplingMethodStatus,
    detection_limits: DetectionLimitStatus,
    sensor_calibration: CalibrationStatus,
    spatial_coverage: SpatialCoverage,
    plume_at_monitoring_boundary: bool,
    impact_signal: ImpactSignal,
    scope: SpatialScope,
    recovery_claimed: bool,
    recovery_followup: Option<RecoveryFollowup>,
    cumulative_exposure_claimed: bool,
    cumulative_sound: ExposureCoverage,
    cumulative_light: ExposureCoverage,
    cumulative_sediment: ExposureCoverage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Assessment {
    scope: SpatialScope,
    evidence_integrity: EvidenceIntegrity,
    comparability: Comparability,
    impact: ImpactResult,
    recovery: RecoveryResult,
    cumulative_exposure: CumulativeExposureResult,
    reason_codes: Vec<ReasonCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    description: String,
    evidence: Evidence,
    expected: Assessment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_id: String,
    schema_version: String,
    suite_id: String,
    evidence_class: String,
    claim_ceiling: String,
    note: String,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct CaseResult {
    scenario_id: String,
    passed: bool,
    expected: Assessment,
    actual: Assessment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct VerificationReceipt {
    schema_id: String,
    schema_version: String,
    tool_version: String,
    suite_id: String,
    corpus_sha256: String,
    source_commit: String,
    evidence_class: String,
    claim_ceiling: String,
    case_count: usize,
    passed_count: usize,
    failed_count: usize,
    result: String,
    cases: Vec<CaseResult>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn assess(evidence: &Evidence) -> Assessment {
    let mut reasons = BTreeSet::new();
    let mut invalid = false;
    if evidence.source_snapshot_id != evidence.expected_source_snapshot_id {
        reasons.insert(ReasonCode::SourceSnapshotMismatch);
        invalid = true;
    }
    let unique: HashSet<&str> = evidence.sample_ids.iter().map(String::as_str).collect();
    if unique.len() != evidence.sample_ids.len() {
        reasons.insert(ReasonCode::DuplicateSampleIdentity);
        invalid = true;
    }
    if evidence.sample_ids.iter().any(|sample_id| sample_id.trim().is_empty()) {
        reasons.insert(ReasonCode::MissingSampleIdentity);
        invalid = true;
    }
    let integrity = if invalid {
        EvidenceIntegrity::InvalidEvidence
    } else {
        EvidenceIntegrity::Valid
    };

    let mut comparison_gap = false;
    if evidence.sample_ids.is_empty() {
        reasons.insert(ReasonCode::SampleSetEmpty);
        comparison_gap = true;
    }
    match evidence.baseline {
        BaselineStatus::Available => {}
        BaselineStatus::Missing => {
            reasons.insert(ReasonCode::BaselineMissing);
            comparison_gap = true;
        }
        BaselineStatus::Unknown => {
            reasons.insert(ReasonCode::BaselineUnknown);
            comparison_gap = true;
        }
    }
    match evidence.reference_control {
        ReferenceControlStatus::Comparable => {}
        ReferenceControlStatus::Missing => {
            reasons.insert(ReasonCode::ReferenceControlMissing);
            comparison_gap = true;
        }
        ReferenceControlStatus::Incomparable => {
            reasons.insert(ReasonCode::ReferenceControlIncomparable);
            comparison_gap = true;
        }
        ReferenceControlStatus::Unknown => {
            reasons.insert(ReasonCode::ReferenceControlUnknown);
            comparison_gap = true;
        }
    }
    match evidence.sampling_methods {
        SamplingMethodStatus::Comparable => {}
        SamplingMethodStatus::Incomparable => {
            reasons.insert(ReasonCode::SamplingMethodsIncomparable);
            comparison_gap = true;
        }
        SamplingMethodStatus::Unknown => {
            reasons.insert(ReasonCode::SamplingMethodsUnknown);
            comparison_gap = true;
        }
    }
    match evidence.detection_limits {
        DetectionLimitStatus::Declared => {}
        DetectionLimitStatus::Missing => {
            reasons.insert(ReasonCode::DetectionLimitsMissing);
            comparison_gap = true;
        }
        DetectionLimitStatus::Unknown => {
            reasons.insert(ReasonCode::DetectionLimitsUnknown);
            comparison_gap = true;
        }
    }
    match evidence.sensor_calibration {
        CalibrationStatus::Current => {}
        CalibrationStatus::Stale => {
            reasons.insert(ReasonCode::SensorCalibrationStale);
            comparison_gap = true;
        }
        CalibrationStatus::Unknown => {
            reasons.insert(ReasonCode::SensorCalibrationUnknown);
            comparison_gap = true;
        }
    }
    match evidence.spatial_coverage {
        SpatialCoverage::AdequateForDeclaredQuestion => {}
        SpatialCoverage::Incomplete => {
            reasons.insert(ReasonCode::SpatialCoverageIncomplete);
            comparison_gap = true;
        }
        SpatialCoverage::Unknown => {
            reasons.insert(ReasonCode::SpatialCoverageUnknown);
            comparison_gap = true;
        }
    }
    if evidence.plume_at_monitoring_boundary {
        reasons.insert(ReasonCode::PlumeAtMonitoringBoundary);
    }

    let comparability = if invalid || comparison_gap {
        Comparability::Indeterminate
    } else {
        Comparability::AdequateForDeclaredQuestion
    };

    let impact = if invalid || comparability == Comparability::Indeterminate {
        ImpactResult::Indeterminate
    } else {
        match evidence.impact_signal {
            ImpactSignal::Detected => {
                reasons.insert(ReasonCode::ImpactSignalDetectedWithinScope);
                ImpactResult::ImpactEvidenceWithinScope
            }
            ImpactSignal::NotDetected => {
                reasons.insert(ReasonCode::NoImpactDetectedWithinScope);
                ImpactResult::NoImpactDetectedWithinScope
            }
            ImpactSignal::Ambiguous => {
                reasons.insert(ReasonCode::ImpactSignalAmbiguous);
                ImpactResult::Indeterminate
            }
        }
    };

    let recovery = if !evidence.recovery_claimed {
        RecoveryResult::NotClaimed
    } else if invalid || comparability == Comparability::Indeterminate {
        RecoveryResult::Indeterminate
    } else {
        match evidence.recovery_followup {
            Some(RecoveryFollowup::Sufficient) => RecoveryResult::RecoveryEvidenceSufficient,
            Some(RecoveryFollowup::Insufficient) => {
                reasons.insert(ReasonCode::RecoveryFollowupInsufficient);
                RecoveryResult::RecoveryNotEstablished
            }
            Some(RecoveryFollowup::Unknown) | None => {
                reasons.insert(ReasonCode::RecoveryFollowupUnknown);
                RecoveryResult::Indeterminate
            }
        }
    };

    let cumulative_exposure = if !evidence.cumulative_exposure_claimed {
        CumulativeExposureResult::NotClaimed
    } else if invalid {
        CumulativeExposureResult::Indeterminate
    } else {
        let mut complete = true;
        match evidence.cumulative_sound {
            ExposureCoverage::Complete => {}
            ExposureCoverage::Incomplete => {
                reasons.insert(ReasonCode::CumulativeSoundIncomplete);
                complete = false;
            }
            ExposureCoverage::Unknown | ExposureCoverage::NotAssessed => {
                reasons.insert(ReasonCode::CumulativeSoundUnknown);
                complete = false;
            }
        }
        match evidence.cumulative_light {
            ExposureCoverage::Complete => {}
            ExposureCoverage::Incomplete => {
                reasons.insert(ReasonCode::CumulativeLightIncomplete);
                complete = false;
            }
            ExposureCoverage::Unknown | ExposureCoverage::NotAssessed => {
                reasons.insert(ReasonCode::CumulativeLightUnknown);
                complete = false;
            }
        }
        match evidence.cumulative_sediment {
            ExposureCoverage::Complete => {}
            ExposureCoverage::Incomplete => {
                reasons.insert(ReasonCode::CumulativeSedimentIncomplete);
                complete = false;
            }
            ExposureCoverage::Unknown | ExposureCoverage::NotAssessed => {
                reasons.insert(ReasonCode::CumulativeSedimentUnknown);
                complete = false;
            }
        }
        if complete {
            CumulativeExposureResult::CompleteForDeclaredChannels
        } else {
            CumulativeExposureResult::Indeterminate
        }
    };

    Assessment {
        scope: evidence.scope,
        evidence_integrity: integrity,
        comparability,
        impact,
        recovery,
        cumulative_exposure,
        reason_codes: reasons.into_iter().collect(),
    }
}

fn validate_corpus(corpus: &Corpus) -> Check<()> {
    if corpus.schema_id != SCHEMA_ID || corpus.schema_version != SCHEMA_VERSION {
        return Err("unsupported schema ID or version".into());
    }
    if corpus.suite_id != SUITE_ID {
        return Err("unexpected suite ID".into());
    }
    if corpus.evidence_class != "SyntheticAdversarial" {
        return Err("corpus must be explicitly SyntheticAdversarial".into());
    }
    if corpus.claim_ceiling.trim().is_empty() || corpus.note.trim().is_empty() {
        return Err("claim ceiling and synthetic-data note must be present".into());
    }
    if corpus.scenarios.len() != CASE_IDS.len() {
        return Err(format!("expected {} frozen cases, got {}", CASE_IDS.len(), corpus.scenarios.len()));
    }
    let mut seen = HashSet::new();
    for (idx, scenario) in corpus.scenarios.iter().enumerate() {
        if scenario.scenario_id != CASE_IDS[idx] {
            return Err(format!("case {idx} must be {} but was {}", CASE_IDS[idx], scenario.scenario_id));
        }
        if !seen.insert(&scenario.scenario_id) {
            return Err(format!("duplicate scenario ID: {}", scenario.scenario_id));
        }
        if scenario.description.trim().is_empty() {
            return Err(format!("scenario {} has an empty description", scenario.scenario_id));
        }
        if scenario.evidence.source_snapshot_id.trim().is_empty()
            || scenario.evidence.expected_source_snapshot_id.trim().is_empty()
        {
            return Err(format!("scenario {} must name both source snapshot IDs", scenario.scenario_id));
        }
        if scenario.evidence.recovery_claimed != scenario.evidence.recovery_followup.is_some() {
            return Err(format!("scenario {} must declare recovery follow-up iff recovery is claimed", scenario.scenario_id));
        }
        if scenario.evidence.cumulative_exposure_claimed {
            if [scenario.evidence.cumulative_sound, scenario.evidence.cumulative_light, scenario.evidence.cumulative_sediment]
                .contains(&ExposureCoverage::NotAssessed)
            {
                return Err(format!("scenario {} must state each claimed cumulative exposure channel", scenario.scenario_id));
            }
        } else if [scenario.evidence.cumulative_sound, scenario.evidence.cumulative_light, scenario.evidence.cumulative_sediment]
            .iter().any(|state| *state != ExposureCoverage::NotAssessed)
        {
            return Err(format!("scenario {} supplies cumulative coverage without declaring that assessment", scenario.scenario_id));
        }
        if scenario.expected.scope != scenario.evidence.scope {
            return Err(format!("scenario {} expected scope does not match evidence scope", scenario.scenario_id));
        }
    }
    Ok(())
}

fn corpus_from_bytes(bytes: &[u8]) -> Check<Corpus> {
    if bytes.len() > MAX_CORPUS_BYTES {
        return Err(format!("corpus exceeds {MAX_CORPUS_BYTES} byte limit"));
    }
    let corpus: Corpus = serde_json::from_slice(bytes).map_err(|e| format!("invalid corpus JSON: {e}"))?;
    validate_corpus(&corpus)?;
    Ok(corpus)
}

fn source_commit(explicit: Option<String>) -> Check<String> {
    let candidate = explicit.or_else(|| env::var("CCZ_SOURCE_COMMIT").ok())
        .or_else(|| env::var("GITHUB_SHA").ok())
        .or_else(|| {
            Command::new("git").args(["rev-parse", "HEAD"]).output().ok()
                .filter(|out| out.status.success())
                .and_then(|out| String::from_utf8(out.stdout).ok())
                .map(|s| s.trim().to_owned())
        })
        .ok_or_else(|| "source commit is required (use --source-commit, CCZ_SOURCE_COMMIT, GITHUB_SHA, or run in a git checkout)".to_owned())?;
    if !(40..=64).contains(&candidate.len()) || !candidate.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("source commit must be a 40-64 character hexadecimal Git object ID".into());
    }
    Ok(candidate.to_ascii_lowercase())
}

fn build_receipt(corpus: &Corpus, bytes: &[u8], commit: String) -> VerificationReceipt {
    let cases: Vec<CaseResult> = corpus.scenarios.iter().map(|scenario| {
        let actual = assess(&scenario.evidence);
        CaseResult {
            scenario_id: scenario.scenario_id.clone(),
            passed: actual == scenario.expected,
            expected: scenario.expected.clone(),
            actual,
        }
    }).collect();
    let passed_count = cases.iter().filter(|case| case.passed).count();
    let failed_count = cases.len() - passed_count;
    VerificationReceipt {
        schema_id: SCHEMA_ID.to_owned(),
        schema_version: SCHEMA_VERSION.to_owned(),
        tool_version: VERSION.to_owned(),
        suite_id: corpus.suite_id.clone(),
        corpus_sha256: sha256_hex(bytes),
        source_commit: commit,
        evidence_class: corpus.evidence_class.clone(),
        claim_ceiling: corpus.claim_ceiling.clone(),
        case_count: cases.len(),
        passed_count,
        failed_count,
        result: if failed_count == 0 { "PASS".into() } else { "FAIL".into() },
        cases,
    }
}

fn run() -> Check<ExitCode> {
    let mut corpus_path = PathBuf::from(DEFAULT_CORPUS_PATH);
    let mut receipt_path: Option<PathBuf> = None;
    let mut commit: Option<String> = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--corpus" => {
                corpus_path = PathBuf::from(args.next().ok_or("--corpus requires a path")?);
            }
            "--receipt" => {
                receipt_path = Some(PathBuf::from(args.next().ok_or("--receipt requires a path")?));
            }
            "--source-commit" => {
                commit = Some(args.next().ok_or("--source-commit requires a Git SHA")?);
            }
            "--help" | "-h" => {
                println!("ccz-observability-bench [--corpus PATH] [--receipt PATH] [--source-commit SHA]");
                println!("Verifies synthetic environmental evidence classifications; not an ecological model or operational approval.");
                return Ok(ExitCode::SUCCESS);
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }

    let bytes = fs::read(&corpus_path).map_err(|e| format!("cannot read {}: {e}", corpus_path.display()))?;
    let corpus = corpus_from_bytes(&bytes)?;
    let commit = source_commit(commit)?;
    let receipt = build_receipt(&corpus, &bytes, commit);
    let json = serde_json::to_string_pretty(&receipt).map_err(|e| format!("cannot encode receipt: {e}"))? + "\n";
    if let Some(path) = receipt_path {
        fs::write(&path, &json).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    } else {
        print!("{json}");
    }

    if receipt.failed_count == 0 {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("ccz-observability-bench: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frozen_corpus() -> Corpus {
        corpus_from_bytes(include_bytes!("../../../docs/tech/CCZ_ENVIRONMENTAL_OBSERVABILITY_CORPUS_V0_1.json"))
            .expect("frozen synthetic corpus must parse and validate")
    }

    fn case(id: &str) -> Scenario {
        frozen_corpus().scenarios.into_iter()
            .find(|scenario| scenario.scenario_id == id)
            .expect("test fixture exists")
    }

    #[test]
    fn frozen_corpus_matches_all_preregistered_outcomes() {
        let corpus = frozen_corpus();
        let receipt = build_receipt(&corpus, include_bytes!("../../../docs/tech/CCZ_ENVIRONMENTAL_OBSERVABILITY_CORPUS_V0_1.json"), "a".repeat(40));
        assert_eq!(receipt.case_count, CASE_IDS.len());
        assert_eq!(receipt.passed_count, CASE_IDS.len());
        assert_eq!(receipt.failed_count, 0);
        assert_eq!(receipt.result, "PASS");
        assert_eq!(receipt.source_commit.len(), 40);
    }

    #[test]
    fn missing_baseline_never_becomes_no_impact() {
        let scenario = case("missing-baseline");
        let result = assess(&scenario.evidence);
        assert_eq!(result.comparability, Comparability::Indeterminate);
        assert_eq!(result.impact, ImpactResult::Indeterminate);
        assert!(result.reason_codes.contains(&ReasonCode::BaselineMissing));
    }

    #[test]
    fn boundary_plume_with_incomplete_coverage_stays_indeterminate() {
        let scenario = case("plume-at-incomplete-boundary");
        let result = assess(&scenario.evidence);
        assert_eq!(result.comparability, Comparability::Indeterminate);
        assert_eq!(result.impact, ImpactResult::Indeterminate);
        assert!(result.reason_codes.contains(&ReasonCode::PlumeAtMonitoringBoundary));
        assert!(result.reason_codes.contains(&ReasonCode::SpatialCoverageIncomplete));
    }

    #[test]
    fn recovery_is_not_established_by_insufficient_followup() {
        let scenario = case("recovery-claim-insufficient-followup");
        let result = assess(&scenario.evidence);
        assert_eq!(result.recovery, RecoveryResult::RecoveryNotEstablished);
    }

    #[test]
    fn cumulative_channels_are_independent_from_local_impact_result() {
        let scenario = case("cumulative-exposure-incomplete");
        let result = assess(&scenario.evidence);
        assert_eq!(result.impact, ImpactResult::NoImpactDetectedWithinScope);
        assert_eq!(result.cumulative_exposure, CumulativeExposureResult::Indeterminate);
        assert!(result.reason_codes.contains(&ReasonCode::CumulativeSoundIncomplete));
        assert!(result.reason_codes.contains(&ReasonCode::CumulativeLightUnknown));
    }

    #[test]
    fn source_mismatch_invalidates_evidence() {
        let scenario = case("source-snapshot-mismatch");
        let result = assess(&scenario.evidence);
        assert_eq!(result.evidence_integrity, EvidenceIntegrity::InvalidEvidence);
        assert_eq!(result.impact, ImpactResult::Indeterminate);
        assert!(result.reason_codes.contains(&ReasonCode::SourceSnapshotMismatch));
    }

    #[test]
    fn duplicate_sample_identity_invalidates_evidence() {
        let scenario = case("duplicate-sample-identities");
        let result = assess(&scenario.evidence);
        assert_eq!(result.evidence_integrity, EvidenceIntegrity::InvalidEvidence);
        assert!(result.reason_codes.contains(&ReasonCode::DuplicateSampleIdentity));
    }

    #[test]
    fn blank_sample_identity_invalidates_evidence() {
        let scenario = case("blank-sample-identity");
        let result = assess(&scenario.evidence);
        assert_eq!(result.evidence_integrity, EvidenceIntegrity::InvalidEvidence);
        assert!(result.reason_codes.contains(&ReasonCode::MissingSampleIdentity));
    }

    #[test]
    fn single_fault_mutations_downgrade_all_comparability_gates() {
        let good = case("non-detection-bounded-to-scope").evidence;
        let mut mutations = Vec::new();

        let mut e = good.clone(); e.baseline = BaselineStatus::Missing; mutations.push(e);
        let mut e = good.clone(); e.baseline = BaselineStatus::Unknown; mutations.push(e);
        let mut e = good.clone(); e.reference_control = ReferenceControlStatus::Missing; mutations.push(e);
        let mut e = good.clone(); e.reference_control = ReferenceControlStatus::Incomparable; mutations.push(e);
        let mut e = good.clone(); e.reference_control = ReferenceControlStatus::Unknown; mutations.push(e);
        let mut e = good.clone(); e.sampling_methods = SamplingMethodStatus::Incomparable; mutations.push(e);
        let mut e = good.clone(); e.sampling_methods = SamplingMethodStatus::Unknown; mutations.push(e);
        let mut e = good.clone(); e.detection_limits = DetectionLimitStatus::Missing; mutations.push(e);
        let mut e = good.clone(); e.detection_limits = DetectionLimitStatus::Unknown; mutations.push(e);
        let mut e = good.clone(); e.sensor_calibration = CalibrationStatus::Stale; mutations.push(e);
        let mut e = good.clone(); e.sensor_calibration = CalibrationStatus::Unknown; mutations.push(e);
        let mut e = good.clone(); e.spatial_coverage = SpatialCoverage::Incomplete; mutations.push(e);
        let mut e = good; e.spatial_coverage = SpatialCoverage::Unknown; mutations.push(e);

        for mutated in mutations {
            let result = assess(&mutated);
            assert_eq!(result.comparability, Comparability::Indeterminate);
            assert_eq!(result.impact, ImpactResult::Indeterminate);
        }
    }

    #[test]
    fn each_cumulative_exposure_channel_is_an_independent_gate() {
        let mut e = case("cumulative-exposure-complete").evidence;
        e.cumulative_sound = ExposureCoverage::Incomplete;
        assert_eq!(assess(&e).cumulative_exposure, CumulativeExposureResult::Indeterminate);
        e.cumulative_sound = ExposureCoverage::Complete;
        e.cumulative_light = ExposureCoverage::Unknown;
        assert_eq!(assess(&e).cumulative_exposure, CumulativeExposureResult::Indeterminate);
        e.cumulative_light = ExposureCoverage::Complete;
        e.cumulative_sediment = ExposureCoverage::Incomplete;
        assert_eq!(assess(&e).cumulative_exposure, CumulativeExposureResult::Indeterminate);
    }

    #[test]
    fn far_field_impact_keeps_the_declared_scope() {
        let scenario = case("far-field-impact-preserves-scope");
        let result = assess(&scenario.evidence);
        assert_eq!(result.scope, SpatialScope::FarField);
        assert_eq!(result.impact, ImpactResult::ImpactEvidenceWithinScope);
    }

    #[test]
    fn cumulative_completeness_does_not_imply_ecological_safety() {
        let scenario = case("cumulative-exposure-complete");
        let result = assess(&scenario.evidence);
        assert_eq!(result.cumulative_exposure, CumulativeExposureResult::CompleteForDeclaredChannels);
        assert_eq!(result.impact, ImpactResult::NoImpactDetectedWithinScope);
    }

    #[test]
    fn strict_structs_reject_unknown_fields() {
        let raw = r#"{"schema_id":"x","schema_version":"0.1.0","suite_id":"x","evidence_class":"SyntheticAdversarial","claim_ceiling":"x","note":"x","scenarios":[],"owner":"country-x"}"#;
        assert!(serde_json::from_str::<Corpus>(raw).is_err());
    }

    #[test]
    fn report_has_no_wall_clock_field_and_is_deterministic() {
        let corpus = frozen_corpus();
        let bytes = include_bytes!("../../../docs/tech/CCZ_ENVIRONMENTAL_OBSERVABILITY_CORPUS_V0_1.json");
        let first = build_receipt(&corpus, bytes, "b".repeat(40));
        let second = build_receipt(&corpus, bytes, "b".repeat(40));
        assert_eq!(serde_json::to_vec(&first).unwrap(), serde_json::to_vec(&second).unwrap());
        assert_eq!(first.corpus_sha256, sha256_hex(bytes));
    }

    #[test]
    fn malformed_commit_is_rejected() {
        assert!(source_commit(Some("queued".into())).is_err());
        assert!(source_commit(Some("a".repeat(40))).is_ok());
    }
}
