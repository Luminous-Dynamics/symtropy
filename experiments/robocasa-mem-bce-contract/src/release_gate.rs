//! Release qualification gate for native RoboCasa-Mem evaluation.
//!
//! This module prevents source inspection, historical wheel checks, or partial
//! asset validation from being mislabeled as a reproducible current release.
//! It qualifies the preconditions for a native policy evaluation; it does not
//! execute the simulator/policy or establish a benchmark result.

use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationRights {
    Authorized,
    Restricted,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledWheelAcceptance {
    /// Digest of the wheel being proposed for the current run.
    pub candidate_wheel_sha256: String,
    /// Digest of the wheel actually exercised by installed-wheel acceptance.
    pub tested_wheel_sha256: String,
    /// Digest of the current runtime source/package closure.
    pub candidate_runtime_sha256: String,
    /// Digest of the runtime closure exercised by the acceptance run.
    pub tested_runtime_sha256: String,
    pub expected_cases: usize,
    pub passed_cases: usize,
    pub failed_cases: usize,
    pub clean_environment_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSmokeEvidence {
    pub checkpoint_sha256: String,
    /// Digest of the policy-serving/runtime configuration used for the smoke.
    pub candidate_runtime_sha256: String,
    pub tested_runtime_sha256: String,
    pub episodes: usize,
    pub successful_episodes: usize,
    pub failed_episodes: usize,
    pub checkpoint_rights: EvaluationRights,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoboCasaMemReleaseEvidence {
    pub release_id: String,
    pub package_version: String,
    /// Source revision actually checked out and audited.
    pub candidate_source_commit: String,
    /// Source revision declared by the candidate release manifest.
    pub manifest_source_commit: String,
    pub source_file_manifest_sha256: String,
    pub suite_manifest_sha256: String,
    pub evaluation_input_lock_sha256: String,
    pub simulator_source_lock_sha256: String,

    pub expected_task_count: usize,
    pub manifest_task_count: usize,
    pub expected_suite_count: usize,
    pub verified_suite_count: usize,
    pub expected_input_count: usize,
    pub resolved_input_count: usize,
    pub suite_bytes_preserved: bool,
    pub release_ready_attested: bool,

    pub asset_manifest_sha256: String,
    pub asset_archive_sha256: String,
    /// The distributed asset manifest's own task-coverage assertion.
    pub asset_manifest_task_coverage_verified: bool,
    /// Independent receipt for coverage over the declared official task inputs.
    pub asset_receipt_task_coverage_verified: bool,
    pub asset_verified_task_count: usize,
    pub asset_archive_available: bool,
    pub clean_candidate_installation_verified: bool,

    pub dataset_rights: EvaluationRights,
    pub asset_rights: EvaluationRights,

    pub installed_wheel: InstalledWheelAcceptance,
    pub model_smoke: ModelSmokeEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReleaseGateBlocker {
    EmptyReleaseIdentity,
    InvalidSourceCommit,
    InvalidArtifactDigest,
    SourceRevisionMismatch,
    SuiteBytesNotPreserved,
    ReleaseNotMarkedReady,
    TaskCountMismatch,
    SuiteCountMismatch,
    EvaluationInputCountMismatch,
    DatasetRightsUnknown,
    DatasetRightsRestricted,
    AssetRightsUnknown,
    AssetRightsRestricted,
    AssetArchiveUnavailable,
    AssetManifestCoverageUnverified,
    AssetReceiptCoverageUnverified,
    AssetCoverageCountMismatch,
    CleanCandidateInstallationUnverified,
    CandidateWheelDigestMismatch,
    CandidateRuntimeDigestMismatch,
    InstalledWheelWasNotReaccepted,
    InstalledWheelCaseCountMismatch,
    InstalledWheelAcceptanceFailedCases,
    InstalledWheelCleanEnvironmentMissing,
    CheckpointRightsUnknown,
    CheckpointRightsRestricted,
    ModelCheckpointDigestMismatch,
    ModelSmokeRuntimeDigestMismatch,
    ModelSmokeMissing,
    ModelSmokeFailedEpisodes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReleaseQualificationStage {
    SourceIdentityAudited,
    EvaluationInputsFrozen,
    AssetClosureQualified,
    CurrentWheelAccepted,
    ModelInferenceSmokeQualified,
    NativeEvaluationReady,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseQualificationReport {
    pub stages: Vec<ReleaseQualificationStage>,
    pub blockers: Vec<ReleaseGateBlocker>,
    pub native_evaluation_ready: bool,
}

impl ReleaseQualificationReport {
    pub fn has_blocker(&self, blocker: ReleaseGateBlocker) -> bool {
        self.blockers.contains(&blocker)
    }
}

/// Evaluate the release preconditions. Every required artifact is identified by
/// a digest and the current wheel/runtime must match the wheel/runtime actually
/// exercised by installed-wheel acceptance. A historical pass is insufficient.
pub fn qualify_release(
    evidence: &RoboCasaMemReleaseEvidence,
) -> Result<ReleaseQualificationReport, ReleaseGateBlocker> {
    if evidence.release_id.trim().is_empty() || evidence.package_version.trim().is_empty() {
        return Err(ReleaseGateBlocker::EmptyReleaseIdentity);
    }

    if !is_git_commit(&evidence.candidate_source_commit)
        || !is_git_commit(&evidence.manifest_source_commit)
    {
        return Err(ReleaseGateBlocker::InvalidSourceCommit);
    }

    let digests = [
        evidence.source_file_manifest_sha256.as_str(),
        evidence.suite_manifest_sha256.as_str(),
        evidence.evaluation_input_lock_sha256.as_str(),
        evidence.simulator_source_lock_sha256.as_str(),
        evidence.asset_manifest_sha256.as_str(),
        evidence.asset_archive_sha256.as_str(),
        evidence.installed_wheel.candidate_wheel_sha256.as_str(),
        evidence.installed_wheel.tested_wheel_sha256.as_str(),
        evidence.installed_wheel.candidate_runtime_sha256.as_str(),
        evidence.installed_wheel.tested_runtime_sha256.as_str(),
        evidence.model_smoke.checkpoint_sha256.as_str(),
        evidence.model_smoke.candidate_runtime_sha256.as_str(),
        evidence.model_smoke.tested_runtime_sha256.as_str(),
    ];
    if digests.iter().any(|digest| !is_sha256_hex(digest)) {
        return Err(ReleaseGateBlocker::InvalidArtifactDigest);
    }

    let mut blockers = BTreeSet::new();

    if evidence.candidate_source_commit != evidence.manifest_source_commit {
        blockers.insert(ReleaseGateBlocker::SourceRevisionMismatch);
    }
    if !evidence.suite_bytes_preserved {
        blockers.insert(ReleaseGateBlocker::SuiteBytesNotPreserved);
    }
    if !evidence.release_ready_attested {
        blockers.insert(ReleaseGateBlocker::ReleaseNotMarkedReady);
    }
    if evidence.manifest_task_count != evidence.expected_task_count
        || evidence.asset_verified_task_count != evidence.expected_task_count
    {
        blockers.insert(ReleaseGateBlocker::TaskCountMismatch);
    }
    if evidence.verified_suite_count != evidence.expected_suite_count
        || evidence.expected_suite_count == 0
    {
        blockers.insert(ReleaseGateBlocker::SuiteCountMismatch);
    }
    if evidence.resolved_input_count != evidence.expected_input_count
        || evidence.expected_input_count == 0
    {
        blockers.insert(ReleaseGateBlocker::EvaluationInputCountMismatch);
    }

    add_rights_blockers(
        &mut blockers,
        evidence.dataset_rights,
        ReleaseGateBlocker::DatasetRightsUnknown,
        ReleaseGateBlocker::DatasetRightsRestricted,
    );
    add_rights_blockers(
        &mut blockers,
        evidence.asset_rights,
        ReleaseGateBlocker::AssetRightsUnknown,
        ReleaseGateBlocker::AssetRightsRestricted,
    );

    if !evidence.asset_archive_available {
        blockers.insert(ReleaseGateBlocker::AssetArchiveUnavailable);
    }
    if !evidence.asset_manifest_task_coverage_verified {
        blockers.insert(ReleaseGateBlocker::AssetManifestCoverageUnverified);
    }
    if !evidence.asset_receipt_task_coverage_verified {
        blockers.insert(ReleaseGateBlocker::AssetReceiptCoverageUnverified);
    }
    if !evidence.clean_candidate_installation_verified {
        blockers.insert(ReleaseGateBlocker::CleanCandidateInstallationUnverified);
    }

    let wheel = &evidence.installed_wheel;
    if wheel.candidate_wheel_sha256 != wheel.tested_wheel_sha256 {
        blockers.insert(ReleaseGateBlocker::CandidateWheelDigestMismatch);
    }
    if wheel.candidate_runtime_sha256 != wheel.tested_runtime_sha256 {
        blockers.insert(ReleaseGateBlocker::CandidateRuntimeDigestMismatch);
        blockers.insert(ReleaseGateBlocker::InstalledWheelWasNotReaccepted);
    }
    if wheel.expected_cases != evidence.expected_input_count
        || wheel.passed_cases != wheel.expected_cases
    {
        blockers.insert(ReleaseGateBlocker::InstalledWheelCaseCountMismatch);
    }
    if wheel.failed_cases > 0 {
        blockers.insert(ReleaseGateBlocker::InstalledWheelAcceptanceFailedCases);
    }
    if !wheel.clean_environment_verified {
        blockers.insert(ReleaseGateBlocker::InstalledWheelCleanEnvironmentMissing);
    }

    let smoke = &evidence.model_smoke;
    add_rights_blockers(
        &mut blockers,
        smoke.checkpoint_rights,
        ReleaseGateBlocker::CheckpointRightsUnknown,
        ReleaseGateBlocker::CheckpointRightsRestricted,
    );
    if smoke.checkpoint_sha256.trim().is_empty() {
        blockers.insert(ReleaseGateBlocker::ModelCheckpointDigestMismatch);
    }
    if smoke.candidate_runtime_sha256 != wheel.candidate_runtime_sha256
        || smoke.tested_runtime_sha256 != smoke.candidate_runtime_sha256
    {
        blockers.insert(ReleaseGateBlocker::ModelSmokeRuntimeDigestMismatch);
    }
    if smoke.episodes == 0 {
        blockers.insert(ReleaseGateBlocker::ModelSmokeMissing);
    } else if smoke.successful_episodes + smoke.failed_episodes != smoke.episodes
        || smoke.failed_episodes > 0
    {
        blockers.insert(ReleaseGateBlocker::ModelSmokeFailedEpisodes);
    }

    let source_audited = evidence.candidate_source_commit == evidence.manifest_source_commit;
    let evaluation_inputs_frozen = !blockers.iter().any(|blocker| {
        matches!(
            blocker,
            ReleaseGateBlocker::SuiteBytesNotPreserved
                | ReleaseGateBlocker::SuiteCountMismatch
                | ReleaseGateBlocker::EvaluationInputCountMismatch
        )
    });
    let asset_closure_qualified = !blockers.iter().any(|blocker| {
        matches!(
            blocker,
            ReleaseGateBlocker::AssetArchiveUnavailable
                | ReleaseGateBlocker::AssetManifestCoverageUnverified
                | ReleaseGateBlocker::AssetReceiptCoverageUnverified
                | ReleaseGateBlocker::AssetCoverageCountMismatch
                | ReleaseGateBlocker::CleanCandidateInstallationUnverified
                | ReleaseGateBlocker::AssetRightsUnknown
                | ReleaseGateBlocker::AssetRightsRestricted
        )
    });
    let current_wheel_accepted = !blockers.iter().any(|blocker| {
        matches!(
            blocker,
            ReleaseGateBlocker::CandidateWheelDigestMismatch
                | ReleaseGateBlocker::CandidateRuntimeDigestMismatch
                | ReleaseGateBlocker::InstalledWheelWasNotReaccepted
                | ReleaseGateBlocker::InstalledWheelCaseCountMismatch
                | ReleaseGateBlocker::InstalledWheelAcceptanceFailedCases
                | ReleaseGateBlocker::InstalledWheelCleanEnvironmentMissing
        )
    });
    let model_inference_smoke_qualified = !blockers.iter().any(|blocker| {
        matches!(
            blocker,
            ReleaseGateBlocker::CheckpointRightsUnknown
                | ReleaseGateBlocker::CheckpointRightsRestricted
                | ReleaseGateBlocker::ModelCheckpointDigestMismatch
                | ReleaseGateBlocker::ModelSmokeRuntimeDigestMismatch
                | ReleaseGateBlocker::ModelSmokeMissing
                | ReleaseGateBlocker::ModelSmokeFailedEpisodes
        )
    });

    let mut stages = Vec::new();
    if source_audited {
        stages.push(ReleaseQualificationStage::SourceIdentityAudited);
    }
    if evaluation_inputs_frozen {
        stages.push(ReleaseQualificationStage::EvaluationInputsFrozen);
    }
    if asset_closure_qualified {
        stages.push(ReleaseQualificationStage::AssetClosureQualified);
    }
    if current_wheel_accepted {
        stages.push(ReleaseQualificationStage::CurrentWheelAccepted);
    }
    if model_inference_smoke_qualified {
        stages.push(ReleaseQualificationStage::ModelInferenceSmokeQualified);
    }

    let native_evaluation_ready = blockers.is_empty();
    if native_evaluation_ready {
        stages.push(ReleaseQualificationStage::NativeEvaluationReady);
    }

    Ok(ReleaseQualificationReport {
        stages,
        blockers: blockers.into_iter().collect(),
        native_evaluation_ready,
    })
}

fn add_rights_blockers(
    blockers: &mut BTreeSet<ReleaseGateBlocker>,
    rights: EvaluationRights,
    unknown: ReleaseGateBlocker,
    restricted: ReleaseGateBlocker,
) {
    match rights {
        EvaluationRights::Authorized => {}
        EvaluationRights::Unknown => {
            blockers.insert(unknown);
        }
        EvaluationRights::Restricted => {
            blockers.insert(restricted);
        }
    }
}

fn is_git_commit(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence() -> RoboCasaMemReleaseEvidence {
        RoboCasaMemReleaseEvidence {
            release_id: "robocasa-mem-0.1.0".to_owned(),
            package_version: "0.1.0".to_owned(),
            candidate_source_commit: "1".repeat(40),
            manifest_source_commit: "1".repeat(40),
            source_file_manifest_sha256: "2".repeat(64),
            suite_manifest_sha256: "3".repeat(64),
            evaluation_input_lock_sha256: "4".repeat(64),
            simulator_source_lock_sha256: "5".repeat(64),
            expected_task_count: 9,
            manifest_task_count: 9,
            expected_suite_count: 27,
            verified_suite_count: 27,
            expected_input_count: 450,
            resolved_input_count: 450,
            suite_bytes_preserved: true,
            release_ready_attested: true,
            asset_manifest_sha256: "6".repeat(64),
            asset_archive_sha256: "7".repeat(64),
            asset_manifest_task_coverage_verified: true,
            asset_receipt_task_coverage_verified: true,
            asset_verified_task_count: 9,
            asset_archive_available: true,
            clean_candidate_installation_verified: true,
            dataset_rights: EvaluationRights::Authorized,
            asset_rights: EvaluationRights::Authorized,
            installed_wheel: InstalledWheelAcceptance {
                candidate_wheel_sha256: "8".repeat(64),
                tested_wheel_sha256: "8".repeat(64),
                candidate_runtime_sha256: "9".repeat(64),
                tested_runtime_sha256: "9".repeat(64),
                expected_cases: 450,
                passed_cases: 450,
                failed_cases: 0,
                clean_environment_verified: true,
            },
            model_smoke: ModelSmokeEvidence {
                checkpoint_sha256: "a".repeat(64),
                candidate_runtime_sha256: "9".repeat(64),
                tested_runtime_sha256: "9".repeat(64),
                episodes: 1,
                successful_episodes: 1,
                failed_episodes: 0,
                checkpoint_rights: EvaluationRights::Authorized,
            },
        }
    }

    #[test]
    fn complete_current_release_preconditions_allow_native_evaluation() {
        let report = qualify_release(&evidence()).unwrap();
        assert!(report.native_evaluation_ready);
        assert!(report.blockers.is_empty());
        assert!(report
            .stages
            .contains(&ReleaseQualificationStage::NativeEvaluationReady));
    }

    #[test]
    fn historical_wheel_pass_does_not_qualify_a_changed_runtime() {
        let mut candidate = evidence();
        candidate.installed_wheel.tested_runtime_sha256 = "a".repeat(64);

        let report = qualify_release(&candidate).unwrap();
        assert!(!report.native_evaluation_ready);
        assert!(report.has_blocker(ReleaseGateBlocker::CandidateRuntimeDigestMismatch));
        assert!(report.has_blocker(ReleaseGateBlocker::InstalledWheelWasNotReaccepted));
    }

    #[test]
    fn source_revision_mismatch_and_unpreserved_suite_bytes_block_release() {
        let mut candidate = evidence();
        candidate.manifest_source_commit = "b".repeat(40);
        candidate.suite_bytes_preserved = false;

        let report = qualify_release(&candidate).unwrap();
        assert!(report.has_blocker(ReleaseGateBlocker::SourceRevisionMismatch));
        assert!(report.has_blocker(ReleaseGateBlocker::SuiteBytesNotPreserved));
    }

    #[test]
    fn asset_manifest_and_external_receipt_must_both_confirm_coverage() {
        let mut candidate = evidence();
        candidate.asset_manifest_task_coverage_verified = false;

        let report = qualify_release(&candidate).unwrap();
        assert!(report.has_blocker(ReleaseGateBlocker::AssetManifestCoverageUnverified));
        assert!(!report.native_evaluation_ready);
    }

    #[test]
    fn asset_installation_must_be_verified_on_clean_candidate() {
        let mut candidate = evidence();
        candidate.clean_candidate_installation_verified = false;

        let report = qualify_release(&candidate).unwrap();
        assert!(report.has_blocker(ReleaseGateBlocker::CleanCandidateInstallationUnverified));
    }

    #[test]
    fn historical_partial_acceptance_does_not_meet_full_official_input_count() {
        let mut candidate = evidence();
        candidate.installed_wheel.passed_cases = 449;
        candidate.installed_wheel.expected_cases = 450;

        let report = qualify_release(&candidate).unwrap();
        assert!(report.has_blocker(ReleaseGateBlocker::InstalledWheelCaseCountMismatch));
    }

    #[test]
    fn failed_installed_wheel_acceptance_blocks_even_with_other_cases_passed() {
        let mut candidate = evidence();
        candidate.installed_wheel.failed_cases = 1;

        let report = qualify_release(&candidate).unwrap();
        assert!(report.has_blocker(ReleaseGateBlocker::InstalledWheelAcceptanceFailedCases));
    }

    #[test]
    fn unknown_or_restricted_rights_never_become_authorized() {
        let mut candidate = evidence();
        candidate.dataset_rights = EvaluationRights::Unknown;
        candidate.asset_rights = EvaluationRights::Restricted;
        candidate.model_smoke.checkpoint_rights = EvaluationRights::Unknown;

        let report = qualify_release(&candidate).unwrap();
        assert!(report.has_blocker(ReleaseGateBlocker::DatasetRightsUnknown));
        assert!(report.has_blocker(ReleaseGateBlocker::AssetRightsRestricted));
        assert!(report.has_blocker(ReleaseGateBlocker::CheckpointRightsUnknown));
    }

    #[test]
    fn model_smoke_must_use_exact_checkpoint_and_candidate_runtime() {
        let mut candidate = evidence();
        candidate.model_smoke.candidate_runtime_sha256 = "a".repeat(64);

        let report = qualify_release(&candidate).unwrap();
        assert!(report.has_blocker(ReleaseGateBlocker::ModelSmokeRuntimeDigestMismatch));
    }

    #[test]
    fn source_and_artifact_digest_shape_is_validated_before_qualification() {
        let mut candidate = evidence();
        candidate.source_file_manifest_sha256 = "not-a-digest".to_owned();
        assert_eq!(
            qualify_release(&candidate),
            Err(ReleaseGateBlocker::InvalidArtifactDigest)
        );

        candidate = evidence();
        candidate.candidate_source_commit = "bad".to_owned();
        assert_eq!(
            qualify_release(&candidate),
            Err(ReleaseGateBlocker::InvalidSourceCommit)
        );
    }

    #[test]
    fn blocker_order_is_canonical() {
        let mut candidate = evidence();
        candidate.suite_bytes_preserved = false;
        candidate.dataset_rights = EvaluationRights::Unknown;
        candidate.release_ready_attested = false;
        candidate.asset_archive_available = false;

        let first = qualify_release(&candidate).unwrap();
        let second = qualify_release(&candidate).unwrap();
        assert_eq!(first.blockers, second.blockers);
        assert!(first
            .blockers
            .windows(2)
            .all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn model_smoke_cannot_claim_success_with_missing_or_failed_episode_results() {
        let mut candidate = evidence();
        candidate.model_smoke.episodes = 2;
        candidate.model_smoke.successful_episodes = 1;
        candidate.model_smoke.failed_episodes = 0;

        let report = qualify_release(&candidate).unwrap();
        assert!(report.has_blocker(ReleaseGateBlocker::ModelSmokeFailedEpisodes));
    }

    #[test]
    fn release_ready_attestation_is_required_beyond_successful_component_checks() {
        let mut candidate = evidence();
        candidate.release_ready_attested = false;

        let report = qualify_release(&candidate).unwrap();
        assert!(report.has_blocker(ReleaseGateBlocker::ReleaseNotMarkedReady));
        assert!(!report.native_evaluation_ready);
    }
}
