//! Result-import contract for memory-dependent mobile-manipulation experiments.
//!
//! This module preserves the raw evaluation propositions needed for later
//! cross-policy comparisons. Terminal task success, stage completion,
//! behavior-conditioned consistency, assistance, run validity and evidence
//! class are deliberately separate fields and must never be collapsed into a
//! single score.
//!
//! It is a typed importer/validator, not a RoboCasa-Mem runner, task evaluator,
//! policy adapter, physical witness, or source of actuator authority.

#![allow(clippy::too_many_arguments)]

use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArtifactDigest([u8; 32]);

impl ArtifactDigest {
    pub fn from_bytes(bytes: [u8; 32]) -> Result<Self, ImportError> {
        if bytes.iter().all(|byte| *byte == 0) {
            return Err(ImportError::UnsetArtifactDigest);
        }
        Ok(Self(bytes))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkProtocolIdentity {
    pub benchmark_id: String,
    pub release_revision: String,
    pub source_commit: String,
    pub task_manifest_digest: ArtifactDigest,
    pub evaluator_digest: ArtifactDigest,
    pub dataset_manifest_digest: ArtifactDigest,
    pub policy_input_schema_digest: ArtifactDigest,
    pub action_schema_digest: ArtifactDigest,
}

impl BenchmarkProtocolIdentity {
    pub fn validate(&self) -> Result<(), ImportError> {
        for value in [
            &self.benchmark_id,
            &self.release_revision,
            &self.source_commit,
        ] {
            if value.trim().is_empty() {
                return Err(ImportError::MissingProtocolIdentity);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvaluationSplit {
    Development,
    Validation,
    Test,
    HeldOut,
    PhysicalConfirmation,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceClass {
    SyntheticFixture,
    Simulator,
    HardwareInLoop,
    PhysicalSupervised,
    PhysicalAutonomous,
    IndependentlyReviewedPhysical,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunDisposition {
    Completed,
    AbortedByPolicy,
    AbortedBySafety,
    AbortedByOperator,
    InfrastructureIndeterminate,
    InvalidProtocol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalOutcome {
    Succeeded,
    Failed,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeEvidenceSource {
    BenchmarkEvaluator,
    SimulatorState,
    PhysicalInstrument,
    HumanGrader,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutcomeReceipt {
    pub source: OutcomeEvidenceSource,
    pub artifact_digest: ArtifactDigest,
}

impl OutcomeReceipt {
    pub fn validate(&self) -> Result<(), ImportError> {
        if self.source == OutcomeEvidenceSource::Unknown {
            return Err(ImportError::UnknownOutcomeEvidenceSource);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageDisposition {
    Succeeded,
    Failed,
    Unresolved,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageResult {
    pub stage_id: String,
    pub disposition: StageDisposition,
    /// A receipt is mandatory for a claimed success/failure. Unresolved stages
    /// are retained without fabricating an evidence reference.
    pub evidence: Option<OutcomeReceipt>,
}

impl StageResult {
    fn validate(&self) -> Result<(), ImportError> {
        if self.stage_id.trim().is_empty() {
            return Err(ImportError::EmptyStageId);
        }
        match self.disposition {
            StageDisposition::Succeeded | StageDisposition::Failed => {
                self.evidence
                    .as_ref()
                    .ok_or_else(|| ImportError::MissingStageReceipt(self.stage_id.clone()))?
                    .validate()?;
            }
            StageDisposition::Unresolved | StageDisposition::NotApplicable => {
                if self.evidence.is_some() {
                    return Err(ImportError::UnexpectedStageReceipt(
                        self.stage_id.clone(),
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssistanceTrack {
    PolicyOnly,
    AgentAssisted,
    ToolAssisted,
    HumanAssisted,
    Mixed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisclosureCompleteness {
    Complete,
    Partial,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssistanceManifest {
    pub track: AssistanceTrack,
    pub disclosure: DisclosureCompleteness,
    pub agent_framework_id: Option<String>,
    pub tools: Vec<String>,
    pub oracle_channels: Vec<String>,
    pub human_interventions: u32,
    pub manifest_digest: ArtifactDigest,
}

impl AssistanceManifest {
    pub fn qualification_blockers(&self) -> Vec<QualificationBlocker> {
        let mut blockers = Vec::new();

        if self.disclosure != DisclosureCompleteness::Complete
            || self.track == AssistanceTrack::Unknown
        {
            blockers.push(QualificationBlocker::AssistanceDisclosureIncomplete);
            return blockers;
        }

        let has_non_policy_assistance = !self.tools.is_empty()
            || !self.oracle_channels.is_empty()
            || self.human_interventions > 0
            || self.agent_framework_id.is_some();

        match self.track {
            AssistanceTrack::PolicyOnly if has_non_policy_assistance => {
                blockers.push(QualificationBlocker::AssistanceTrackContradiction);
            }
            AssistanceTrack::AgentAssisted if self.agent_framework_id.is_none() => {
                blockers.push(QualificationBlocker::AssistanceTrackContradiction);
            }
            AssistanceTrack::ToolAssisted if self.tools.is_empty() => {
                blockers.push(QualificationBlocker::AssistanceTrackContradiction);
            }
            AssistanceTrack::HumanAssisted if self.human_interventions == 0 => {
                blockers.push(QualificationBlocker::AssistanceTrackContradiction);
            }
            AssistanceTrack::Mixed if !has_non_policy_assistance => {
                blockers.push(QualificationBlocker::AssistanceTrackContradiction);
            }
            _ => {}
        }
        blockers
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyInputManifest {
    /// Fields delivered to the acting policy, not evaluator-private state.
    pub policy_visible_fields: Vec<String>,
    /// Evaluator-only fields maintained out of the policy input channel.
    pub evaluator_private_fields: Vec<String>,
    pub observation_bundle_digest: ArtifactDigest,
    pub policy_input_digest: ArtifactDigest,
}

impl PolicyInputManifest {
    pub fn validate(&self) -> Result<(), ImportError> {
        let mut visible = BTreeSet::new();
        for field in &self.policy_visible_fields {
            if field.trim().is_empty() || !visible.insert(field.as_str()) {
                return Err(ImportError::InvalidPolicyInputManifest);
            }
        }

        let mut private = BTreeSet::new();
        for field in &self.evaluator_private_fields {
            if field.trim().is_empty() || !private.insert(field.as_str()) {
                return Err(ImportError::InvalidPolicyInputManifest);
            }
            if visible.contains(field.as_str()) {
                return Err(ImportError::EvaluatorFieldLeakedToPolicy(
                    field.clone(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorConditionedReceipt {
    pub source_interaction_id: String,
    pub target_interaction_id: String,
    pub source_sequence: u64,
    pub target_sequence: u64,
    pub predicate_id: String,
    /// The evaluator's private constraint/decision receipt.
    pub evaluator_receipt_digest: ArtifactDigest,
    /// The policy call associated with the target decision. This is not a
    /// digest of evaluator-private fields and does not imply those fields were
    /// shown to the policy.
    pub policy_input_digest: ArtifactDigest,
}

impl BehaviorConditionedReceipt {
    pub fn validate(&self) -> Result<(), ImportError> {
        if self.source_interaction_id.trim().is_empty()
            || self.target_interaction_id.trim().is_empty()
            || self.predicate_id.trim().is_empty()
        {
            return Err(ImportError::InvalidBceReceipt);
        }
        if self.source_interaction_id == self.target_interaction_id {
            return Err(ImportError::InvalidBceReceipt);
        }
        if self.target_sequence <= self.source_sequence {
            return Err(ImportError::NonLaterBceTarget);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskInstanceIdentity {
    pub task_id: String,
    pub task_instance_id: String,
    pub episode_id: String,
    pub scene_id: String,
    pub scene_seed: u64,
    pub split: EvaluationSplit,
    pub expected_stage_ids: Vec<String>,
}

impl TaskInstanceIdentity {
    pub fn validate(&self) -> Result<(), ImportError> {
        for value in [
            &self.task_id,
            &self.task_instance_id,
            &self.episode_id,
            &self.scene_id,
        ] {
            if value.trim().is_empty() {
                return Err(ImportError::MissingTaskIdentity);
            }
        }
        if self.expected_stage_ids.is_empty() {
            return Err(ImportError::EmptyExpectedStageManifest);
        }
        let mut seen = BTreeSet::new();
        for stage in &self.expected_stage_ids {
            if stage.trim().is_empty() || !seen.insert(stage.as_str()) {
                return Err(ImportError::InvalidExpectedStageManifest);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedControlIdentity {
    /// Same frozen backbone policy for a policy-only vs sidecar comparison.
    pub underlying_policy_digest: ArtifactDigest,
    pub action_executor_digest: ArtifactDigest,
    pub robot_or_embodiment_digest: ArtifactDigest,
    pub environment_config_digest: ArtifactDigest,
    pub resource_budget_digest: ArtifactDigest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyArtifactIdentity {
    pub variant_id: String,
    pub policy_id: String,
    pub checkpoint_digest: ArtifactDigest,
    pub deployment_digest: ArtifactDigest,
    pub assistance: AssistanceManifest,
}

impl PolicyArtifactIdentity {
    pub fn validate(&self) -> Result<(), ImportError> {
        if self.variant_id.trim().is_empty() || self.policy_id.trim().is_empty() {
            return Err(ImportError::MissingPolicyIdentity);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryEvaluationRecord {
    pub protocol: BenchmarkProtocolIdentity,
    pub task: TaskInstanceIdentity,
    pub policy: PolicyArtifactIdentity,
    pub matched_control: MatchedControlIdentity,
    pub evidence_class: EvidenceClass,
    pub run_disposition: RunDisposition,
    /// Terminal task outcome. Never inferred from stage completion.
    pub terminal_outcome: TerminalOutcome,
    pub terminal_receipt: Option<OutcomeReceipt>,
    /// Exactly one result per stage in the frozen task roster.
    pub stages: Vec<StageResult>,
    pub policy_input: PolicyInputManifest,
    /// Empty for non-BCE tasks. One or more receipts for each evaluated
    /// behavior-conditioned decision.
    pub bce_receipts: Vec<BehaviorConditionedReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    MissingProtocolIdentity,
    MissingTaskIdentity,
    MissingPolicyIdentity,
    EmptyExpectedStageManifest,
    InvalidExpectedStageManifest,
    EmptyStageId,
    DuplicateStageId(String),
    MissingStageResult(String),
    UnexpectedStageResult(String),
    MissingStageReceipt(String),
    UnexpectedStageReceipt(String),
    UnknownOutcomeEvidenceSource,
    MissingTerminalReceipt,
    UnexpectedTerminalReceipt,
    InvalidPolicyInputManifest,
    EvaluatorFieldLeakedToPolicy(String),
    InvalidBceReceipt,
    NonLaterBceTarget,
    UnsetArtifactDigest,
    MatchedControlMismatch,
    NonComparableRunIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualificationBlocker {
    UnknownSplit,
    UnknownEvidenceClass,
    AssistanceDisclosureIncomplete,
    AssistanceTrackContradiction,
    TerminalOutcomeUnresolved,
    StageOutcomeUnresolved,
    BceEvaluationMissing,
    PolicyInputDisclosureInvalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageCompletionCounts {
    pub succeeded: usize,
    pub failed: usize,
    pub unresolved: usize,
    pub not_applicable: usize,
}

impl StageCompletionCounts {
    pub fn denominator(&self) -> usize {
        self.succeeded + self.failed + self.unresolved
    }

    pub fn success_rate_numerator(&self) -> usize {
        self.succeeded
    }

    /// Returns a raw ratio only. Official benchmark aggregation may macro-
    /// average stage/task groups with different denominators.
    pub fn micro_success_rate(&self) -> Option<(usize, usize)> {
        let denominator = self.denominator();
        (denominator > 0).then_some((self.succeeded, denominator))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportAudit {
    pub stage_counts: StageCompletionCounts,
    pub terminal_outcome: TerminalOutcome,
    pub terminal_receipt_present: bool,
    pub bce_decision_count: usize,
    pub qualification_blockers: Vec<QualificationBlocker>,
}

impl MemoryEvaluationRecord {
    pub fn validate(&self) -> Result<ImportAudit, ImportError> {
        self.protocol.validate()?;
        self.task.validate()?;
        self.policy.validate()?;
        self.policy_input.validate()?;

        if matches!(
            self.terminal_outcome,
            TerminalOutcome::Succeeded | TerminalOutcome::Failed
        ) {
            self.terminal_receipt
                .as_ref()
                .ok_or(ImportError::MissingTerminalReceipt)?
                .validate()?;
        } else if self.terminal_receipt.is_some() {
            return Err(ImportError::UnexpectedTerminalReceipt);
        }

        let mut counts = StageCompletionCounts {
            succeeded: 0,
            failed: 0,
            unresolved: 0,
            not_applicable: 0,
        };
        let mut seen = BTreeSet::new();

        for stage in &self.stages {
            stage.validate()?;
            if !seen.insert(stage.stage_id.as_str()) {
                return Err(ImportError::DuplicateStageId(stage.stage_id.clone()));
            }
            match stage.disposition {
                StageDisposition::Succeeded => counts.succeeded += 1,
                StageDisposition::Failed => counts.failed += 1,
                StageDisposition::Unresolved => counts.unresolved += 1,
                StageDisposition::NotApplicable => counts.not_applicable += 1,
            }
        }

        for expected in &self.task.expected_stage_ids {
            if !seen.contains(expected.as_str()) {
                return Err(ImportError::MissingStageResult(expected.clone()));
            }
        }
        for actual in &seen {
            if !self
                .task
                .expected_stage_ids
                .iter()
                .any(|expected| expected == actual)
            {
                return Err(ImportError::UnexpectedStageResult((*actual).to_owned()));
            }
        }

        for receipt in &self.bce_receipts {
            receipt.validate()?;
            if receipt.policy_input_digest != self.policy_input.policy_input_digest {
                return Err(ImportError::InvalidBceReceipt);
            }
        }

        let mut blockers = Vec::new();
        if self.task.split == EvaluationSplit::Unknown {
            blockers.push(QualificationBlocker::UnknownSplit);
        }
        if self.evidence_class == EvidenceClass::Unknown {
            blockers.push(QualificationBlocker::UnknownEvidenceClass);
        }
        blockers.extend(self.policy.assistance.qualification_blockers());
        if self.terminal_outcome == TerminalOutcome::Unresolved {
            blockers.push(QualificationBlocker::TerminalOutcomeUnresolved);
        }
        if counts.unresolved > 0 {
            blockers.push(QualificationBlocker::StageOutcomeUnresolved);
        }
        if !self.bce_receipts.is_empty()
            && self.bce_receipts.iter().any(|receipt| {
                receipt.predicate_id.trim().is_empty()
            })
        {
            blockers.push(QualificationBlocker::BceEvaluationMissing);
        }
        if self.run_disposition != RunDisposition::Completed {
            // The run is retained, but it cannot be represented as a fully
            // completed benchmark episode.
            blockers.push(QualificationBlocker::TerminalOutcomeUnresolved);
        }

        Ok(ImportAudit {
            stage_counts: counts,
            terminal_outcome: self.terminal_outcome,
            terminal_receipt_present: self.terminal_receipt.is_some(),
            bce_decision_count: self.bce_receipts.len(),
            qualification_blockers: blockers,
        })
    }

    /// Returns true only if the records share every frozen comparison field
    /// other than policy variant/checkpoint/assistance. This is a matched-arm
    /// identity check, not a statistical test.
    pub fn matches_control_conditions(&self, other: &Self) -> bool {
        self.protocol == other.protocol
            && self.task == other.task
            && self.matched_control == other.matched_control
            && self.evidence_class == other.evidence_class
            && self.policy.variant_id != other.policy.variant_id
    }

    pub fn require_matched_control(&self, other: &Self) -> Result<(), ImportError> {
        if self.matches_control_conditions(other) {
            Ok(())
        } else {
            Err(ImportError::MatchedControlMismatch)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: u8) -> ArtifactDigest {
        ArtifactDigest::from_bytes([value; 32]).unwrap()
    }

    fn protocol() -> BenchmarkProtocolIdentity {
        BenchmarkProtocolIdentity {
            benchmark_id: "robocasa-mem".into(),
            release_revision: "paper-v1-provisional".into(),
            source_commit: "0123456789abcdef0123456789abcdef01234567".into(),
            task_manifest_digest: digest(1),
            evaluator_digest: digest(2),
            dataset_manifest_digest: digest(3),
            policy_input_schema_digest: digest(4),
            action_schema_digest: digest(5),
        }
    }

    fn assistance(track: AssistanceTrack) -> AssistanceManifest {
        match track {
            AssistanceTrack::PolicyOnly => AssistanceManifest {
                track,
                disclosure: DisclosureCompleteness::Complete,
                agent_framework_id: None,
                tools: vec![],
                oracle_channels: vec![],
                human_interventions: 0,
                manifest_digest: digest(6),
            },
            AssistanceTrack::AgentAssisted => AssistanceManifest {
                track,
                disclosure: DisclosureCompleteness::Complete,
                agent_framework_id: Some("agent-wrapper-v1".into()),
                tools: vec![],
                oracle_channels: vec![],
                human_interventions: 0,
                manifest_digest: digest(6),
            },
            AssistanceTrack::ToolAssisted => AssistanceManifest {
                track,
                disclosure: DisclosureCompleteness::Complete,
                agent_framework_id: None,
                tools: vec!["retrieval".into()],
                oracle_channels: vec![],
                human_interventions: 0,
                manifest_digest: digest(6),
            },
            AssistanceTrack::HumanAssisted => AssistanceManifest {
                track,
                disclosure: DisclosureCompleteness::Complete,
                agent_framework_id: None,
                tools: vec![],
                oracle_channels: vec![],
                human_interventions: 1,
                manifest_digest: digest(6),
            },
            AssistanceTrack::Mixed => AssistanceManifest {
                track,
                disclosure: DisclosureCompleteness::Complete,
                agent_framework_id: Some("agent-wrapper-v1".into()),
                tools: vec!["retrieval".into()],
                oracle_channels: vec![],
                human_interventions: 1,
                manifest_digest: digest(6),
            },
            AssistanceTrack::Unknown => AssistanceManifest {
                track,
                disclosure: DisclosureCompleteness::Unknown,
                agent_framework_id: None,
                tools: vec![],
                oracle_channels: vec![],
                human_interventions: 0,
                manifest_digest: digest(6),
            },
        }
    }

    fn receipt(source: OutcomeEvidenceSource, id: u8) -> OutcomeReceipt {
        OutcomeReceipt {
            source,
            artifact_digest: digest(id),
        }
    }

    fn record(variant: &str, track: AssistanceTrack) -> MemoryEvaluationRecord {
        MemoryEvaluationRecord {
            protocol: protocol(),
            task: TaskInstanceIdentity {
                task_id: "HeatAndInspect".into(),
                task_instance_id: "task-instance-01".into(),
                episode_id: "episode-01".into(),
                scene_id: "scene-01".into(),
                scene_seed: 51,
                split: EvaluationSplit::HeldOut,
                expected_stage_ids: vec!["heat".into(), "inspect".into(), "retrieve".into()],
            },
            policy: PolicyArtifactIdentity {
                variant_id: variant.into(),
                policy_id: format!("policy-{variant}"),
                checkpoint_digest: digest(7),
                deployment_digest: digest(8),
                assistance: assistance(track),
            },
            matched_control: MatchedControlIdentity {
                underlying_policy_digest: digest(9),
                action_executor_digest: digest(10),
                robot_or_embodiment_digest: digest(11),
                environment_config_digest: digest(12),
                resource_budget_digest: digest(13),
            },
            evidence_class: EvidenceClass::Simulator,
            run_disposition: RunDisposition::Completed,
            terminal_outcome: TerminalOutcome::Succeeded,
            terminal_receipt: Some(receipt(OutcomeEvidenceSource::BenchmarkEvaluator, 14)),
            stages: vec![
                StageResult {
                    stage_id: "heat".into(),
                    disposition: StageDisposition::Succeeded,
                    evidence: Some(receipt(OutcomeEvidenceSource::BenchmarkEvaluator, 15)),
                },
                StageResult {
                    stage_id: "inspect".into(),
                    disposition: StageDisposition::Succeeded,
                    evidence: Some(receipt(OutcomeEvidenceSource::BenchmarkEvaluator, 16)),
                },
                StageResult {
                    stage_id: "retrieve".into(),
                    disposition: StageDisposition::Failed,
                    evidence: Some(receipt(OutcomeEvidenceSource::BenchmarkEvaluator, 17)),
                },
            ],
            policy_input: PolicyInputManifest {
                policy_visible_fields: vec![
                    "instruction".into(),
                    "rgb".into(),
                    "robot_state".into(),
                ],
                evaluator_private_fields: vec![
                    "bce_expected_target".into(),
                    "terminal_success".into(),
                ],
                observation_bundle_digest: digest(18),
                policy_input_digest: digest(19),
            },
            bce_receipts: vec![],
        }
    }

    #[test]
    fn terminal_success_and_stage_completion_are_independent_metrics() {
        let audit = record("policy-only", AssistanceTrack::PolicyOnly)
            .validate()
            .unwrap();

        assert_eq!(audit.terminal_outcome, TerminalOutcome::Succeeded);
        assert_eq!(audit.stage_counts.succeeded, 2);
        assert_eq!(audit.stage_counts.failed, 1);
        assert_eq!(audit.stage_counts.micro_success_rate(), Some((2, 3)));
        assert!(audit.qualification_blockers.is_empty());
    }

    #[test]
    fn evaluator_private_fields_cannot_leak_into_policy_input_manifest() {
        let mut run = record("policy-only", AssistanceTrack::PolicyOnly);
        run.policy_input
            .policy_visible_fields
            .push("bce_expected_target".into());

        assert_eq!(
            run.validate(),
            Err(ImportError::EvaluatorFieldLeakedToPolicy(
                "bce_expected_target".into()
            ))
        );
    }

    #[test]
    fn policy_only_track_cannot_disclose_tools_or_agent_wrappers() {
        let mut run = record("policy-only", AssistanceTrack::PolicyOnly);
        run.policy.assistance.tools.push("retrieval".into());

        assert_eq!(
            run.validate().unwrap().qualification_blockers,
            vec![QualificationBlocker::AssistanceTrackContradiction]
        );
    }

    #[test]
    fn incomplete_assistance_disclosure_blocks_comparison_without_erasing_raw_metrics() {
        let mut run = record("agent", AssistanceTrack::Unknown);
        run.terminal_outcome = TerminalOutcome::Unresolved;
        run.terminal_receipt = None;

        let audit = run.validate().unwrap();
        assert_eq!(audit.stage_counts.succeeded, 2);
        assert!(audit
            .qualification_blockers
            .contains(&QualificationBlocker::AssistanceDisclosureIncomplete));
        assert!(audit
            .qualification_blockers
            .contains(&QualificationBlocker::TerminalOutcomeUnresolved));
    }

    #[test]
    fn missing_stage_is_not_silently_removed_from_the_manifest() {
        let mut run = record("policy-only", AssistanceTrack::PolicyOnly);
        run.stages.pop();

        assert_eq!(
            run.validate(),
            Err(ImportError::MissingStageResult("retrieve".into()))
        );
    }

    #[test]
    fn extra_or_duplicate_stage_ids_reject() {
        let mut extra = record("policy-only", AssistanceTrack::PolicyOnly);
        extra.stages.push(StageResult {
            stage_id: "hidden-stage".into(),
            disposition: StageDisposition::NotApplicable,
            evidence: None,
        });
        assert_eq!(
            extra.validate(),
            Err(ImportError::UnexpectedStageResult("hidden-stage".into()))
        );

        let mut duplicate = record("policy-only", AssistanceTrack::PolicyOnly);
        duplicate.stages[1].stage_id = "heat".into();
        assert_eq!(
            duplicate.validate(),
            Err(ImportError::DuplicateStageId("heat".into()))
        );
    }

    #[test]
    fn unresolved_stage_remains_in_denominator_and_blocks_full_stage_qualification() {
        let mut run = record("policy-only", AssistanceTrack::PolicyOnly);
        run.stages[2].disposition = StageDisposition::Unresolved;
        run.stages[2].evidence = None;

        let audit = run.validate().unwrap();
        assert_eq!(audit.stage_counts.succeeded, 2);
        assert_eq!(audit.stage_counts.unresolved, 1);
        assert_eq!(audit.stage_counts.micro_success_rate(), Some((2, 3)));
        assert!(audit
            .qualification_blockers
            .contains(&QualificationBlocker::StageOutcomeUnresolved));
    }

    #[test]
    fn not_applicable_stage_is_separate_from_failed_or_missing_stage() {
        let mut run = record("policy-only", AssistanceTrack::PolicyOnly);
        run.stages[2].disposition = StageDisposition::NotApplicable;
        run.stages[2].evidence = None;

        let audit = run.validate().unwrap();
        assert_eq!(audit.stage_counts.not_applicable, 1);
        assert_eq!(audit.stage_counts.denominator(), 2);
        assert_eq!(audit.stage_counts.micro_success_rate(), Some((2, 2)));
    }

    #[test]
    fn a_terminal_failure_never_rewrites_successful_stage_history() {
        let mut run = record("policy-only", AssistanceTrack::PolicyOnly);
        run.terminal_outcome = TerminalOutcome::Failed;
        run.terminal_receipt = Some(receipt(OutcomeEvidenceSource::BenchmarkEvaluator, 20));

        let audit = run.validate().unwrap();
        assert_eq!(audit.terminal_outcome, TerminalOutcome::Failed);
        assert_eq!(audit.stage_counts.succeeded, 2);
        assert_eq!(audit.stage_counts.failed, 1);
    }

    #[test]
    fn terminal_success_requires_a_separate_terminal_receipt() {
        let mut run = record("policy-only", AssistanceTrack::PolicyOnly);
        run.terminal_receipt = None;

        assert_eq!(run.validate(), Err(ImportError::MissingTerminalReceipt));
    }

    #[test]
    fn bce_receipt_is_distinct_and_bound_to_policy_call_digest() {
        let mut run = record("policy-only", AssistanceTrack::PolicyOnly);
        run.bce_receipts.push(BehaviorConditionedReceipt {
            source_interaction_id: "choose-cup-a".into(),
            target_interaction_id: "retrieve-cup-a".into(),
            source_sequence: 4,
            target_sequence: 11,
            predicate_id: "same-entity-v1".into(),
            evaluator_receipt_digest: digest(21),
            policy_input_digest: run.policy_input.policy_input_digest,
        });

        let audit = run.validate().unwrap();
        assert_eq!(audit.bce_decision_count, 1);
        assert_eq!(audit.terminal_outcome, TerminalOutcome::Succeeded);
        assert_eq!(audit.stage_counts.succeeded, 2);
    }

    #[test]
    fn bce_receipt_with_reversed_sequence_or_wrong_policy_call_rejects() {
        let mut run = record("policy-only", AssistanceTrack::PolicyOnly);
        run.bce_receipts.push(BehaviorConditionedReceipt {
            source_interaction_id: "choose-cup-a".into(),
            target_interaction_id: "retrieve-cup-a".into(),
            source_sequence: 12,
            target_sequence: 11,
            predicate_id: "same-entity-v1".into(),
            evaluator_receipt_digest: digest(21),
            policy_input_digest: run.policy_input.policy_input_digest,
        });
        assert_eq!(run.validate(), Err(ImportError::NonLaterBceTarget));

        let mut run = record("policy-only", AssistanceTrack::PolicyOnly);
        run.bce_receipts.push(BehaviorConditionedReceipt {
            source_interaction_id: "choose-cup-a".into(),
            target_interaction_id: "retrieve-cup-a".into(),
            source_sequence: 4,
            target_sequence: 11,
            predicate_id: "same-entity-v1".into(),
            evaluator_receipt_digest: digest(21),
            policy_input_digest: digest(22),
        });
        assert_eq!(run.validate(), Err(ImportError::InvalidBceReceipt));
    }

    #[test]
    fn a_noncompleted_run_is_retained_but_not_qualified_as_a_completed_episode() {
        let mut run = record("policy-only", AssistanceTrack::PolicyOnly);
        run.run_disposition = RunDisposition::InfrastructureIndeterminate;
        run.terminal_outcome = TerminalOutcome::Unresolved;
        run.terminal_receipt = None;

        let audit = run.validate().unwrap();
        assert_eq!(audit.stage_counts.succeeded, 2);
        assert!(audit
            .qualification_blockers
            .contains(&QualificationBlocker::TerminalOutcomeUnresolved));
    }

    #[test]
    fn held_out_unknown_evidence_and_unknown_split_are_explicit_blockers() {
        let mut run = record("policy-only", AssistanceTrack::PolicyOnly);
        run.task.split = EvaluationSplit::Unknown;
        run.evidence_class = EvidenceClass::Unknown;

        let audit = run.validate().unwrap();
        assert!(audit
            .qualification_blockers
            .contains(&QualificationBlocker::UnknownSplit));
        assert!(audit
            .qualification_blockers
            .contains(&QualificationBlocker::UnknownEvidenceClass));
    }

    #[test]
    fn only_exact_matched_control_conditions_can_be_paired() {
        let left = record("policy-only", AssistanceTrack::PolicyOnly);
        let right = record("agent", AssistanceTrack::AgentAssisted);
        left.require_matched_control(&right).unwrap();

        let mut changed = right.clone();
        changed.task.scene_seed += 1;
        assert_eq!(
            left.require_matched_control(&changed),
            Err(ImportError::MatchedControlMismatch)
        );

        let mut changed = right;
        changed.matched_control.resource_budget_digest = digest(99);
        assert_eq!(
            left.require_matched_control(&changed),
            Err(ImportError::MatchedControlMismatch)
        );
    }

    #[test]
    fn assistance_disclosures_are_consistent_with_declared_track() {
        let agent = record("agent", AssistanceTrack::AgentAssisted);
        assert!(agent.validate().unwrap().qualification_blockers.is_empty());

        let tool = record("tool", AssistanceTrack::ToolAssisted);
        assert!(tool.validate().unwrap().qualification_blockers.is_empty());

        let human = record("human", AssistanceTrack::HumanAssisted);
        assert!(human.validate().unwrap().qualification_blockers.is_empty());

        let mixed = record("mixed", AssistanceTrack::Mixed);
        assert!(mixed.validate().unwrap().qualification_blockers.is_empty());
    }

    #[test]
    fn invalid_run_never_counts_as_a_fully_completed_episode() {
        let mut run = record("policy-only", AssistanceTrack::PolicyOnly);
        run.run_disposition = RunDisposition::InvalidProtocol;

        let audit = run.validate().unwrap();
        assert!(audit
            .qualification_blockers
            .contains(&QualificationBlocker::TerminalOutcomeUnresolved));
    }

    #[test]
    fn unset_artifact_digest_is_rejected() {
        assert_eq!(
            ArtifactDigest::from_bytes([0; 32]),
            Err(ImportError::UnsetArtifactDigest)
        );
    }
}
