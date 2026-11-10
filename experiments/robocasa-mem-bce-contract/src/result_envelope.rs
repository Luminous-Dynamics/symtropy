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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatasetRightsStatus {
    Verified,
    RestrictedButAuthorized,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkProtocolIdentity {
    pub dataset_rights_status: DatasetRightsStatus,
    pub benchmark_id: String,
    pub release_revision: String,
    pub source_commit: String,
    pub task_manifest_digest: ArtifactDigest,
    pub evaluator_digest: ArtifactDigest,
    pub dataset_manifest_digest: ArtifactDigest,
    pub dataset_rights_manifest_digest: ArtifactDigest,
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
        if self.source_commit.len() != 40
            || !self.source_commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(ImportError::MissingProtocolIdentity);
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
    fn has_unknown_source(&self) -> bool {
        self.source == OutcomeEvidenceSource::Unknown
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
                    .ok_or_else(|| ImportError::MissingStageReceipt(self.stage_id.clone()))?;
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
            if field.trim().is_empty() || !visible.insert(field.clone()) {
                return Err(ImportError::InvalidPolicyInputManifest);
            }
        }

        let mut private = BTreeSet::new();
        for field in &self.evaluator_private_fields {
            if field.trim().is_empty() || !private.insert(field.clone()) {
                return Err(ImportError::InvalidPolicyInputManifest);
            }
            if visible.contains(field) {
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
    pub bce_required: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrainingOverlapStatus {
    AuditedNoKnownOverlap,
    KnownOverlapDisclosed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyArtifactIdentity {
    pub training_data_manifest_digest: ArtifactDigest,
    pub training_overlap_status: TrainingOverlapStatus,
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
    StageOrderMismatch,
    MissingStageReceipt(String),
    UnexpectedStageReceipt(String),
    MissingTerminalReceipt,
    UnexpectedTerminalReceipt,
    InvalidPolicyInputManifest,
    EvaluatorFieldLeakedToPolicy(String),
    InvalidBceReceipt,
    NonLaterBceTarget,
    DuplicateBceReceipt,
    UnsetArtifactDigest,
    MatchedControlMismatch,
    EmptyAggregate,
    MixedAggregateGroup,
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
    DatasetRightsUnresolved,
    TrainingOverlapUnknown,
    UnknownOutcomeEvidenceSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageRateCount {
    pub stage_id: String,
    pub succeeded: usize,
    pub failed: usize,
    pub unresolved: usize,
    pub not_applicable: usize,
}

impl StageRateCount {
    pub fn denominator(&self) -> usize {
        self.succeeded + self.failed + self.unresolved
    }

    pub fn success_rate(&self) -> Option<(usize, usize)> {
        let denominator = self.denominator();
        (denominator > 0).then_some((self.succeeded, denominator))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageCompletionCounts {
    pub succeeded: usize,
    pub failed: usize,
    pub unresolved: usize,
    pub not_applicable: usize,
    /// Preserve the numerator/denominator for each predefined evaluation
    /// stage. These denominators may differ across a task's episode set.
    pub per_stage: Vec<StageRateCount>,
}

impl StageCompletionCounts {
    pub fn denominator(&self) -> usize {
        self.succeeded + self.failed + self.unresolved
    }

    pub fn success_rate_numerator(&self) -> usize {
        self.succeeded
    }

    /// Pooled stage ratio. Diagnostic only: this is not the paper's SCR macro
    /// aggregation when stage denominators differ.
    pub fn micro_success_rate(&self) -> Option<(usize, usize)> {
        let denominator = self.denominator();
        (denominator > 0).then_some((self.succeeded, denominator))
    }

    /// Macro-average of per-stage success rates (the SCR form described in
    /// the RoboCasa-Mem paper). Returns None if any predefined stage has no
    /// applicable observations, so missing denominators cannot disappear.
    pub fn macro_stage_success_rate(&self) -> Option<f64> {
        if self.per_stage.is_empty() {
            return None;
        }
        let rates: Option<Vec<f64>> = self
            .per_stage
            .iter()
            .map(|stage| {
                let (successes, denominator) = stage.success_rate()?;
                Some(successes as f64 / denominator as f64)
            })
            .collect();
        let rates = rates?;
        Some(rates.iter().sum::<f64>() / rates.len() as f64)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalOutcomeCounts {
    pub succeeded: usize,
    pub failed: usize,
    pub unresolved: usize,
}

impl TerminalOutcomeCounts {
    pub fn attempt_count(&self) -> usize {
        self.succeeded + self.failed + self.unresolved
    }

    /// All-attempt terminal success ratio; unresolved attempts remain in the
    /// denominator as not established successes.
    pub fn success_rate_over_attempts(&self) -> Option<(usize, usize)> {
        let denominator = self.attempt_count();
        (denominator > 0).then_some((self.succeeded, denominator))
    }

    /// Resolved-only ratio, published alongside the all-attempt denominator.
    pub fn success_rate_over_resolved(&self) -> Option<(usize, usize)> {
        let denominator = self.succeeded + self.failed;
        (denominator > 0).then_some((self.succeeded, denominator))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryEvaluationAggregate {
    pub terminal_counts: TerminalOutcomeCounts,
    pub stage_counts: StageCompletionCounts,
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
                .ok_or(ImportError::MissingTerminalReceipt)?;
        } else if self.terminal_receipt.is_some() {
            return Err(ImportError::UnexpectedTerminalReceipt);
        }

        let mut counts = StageCompletionCounts {
            succeeded: 0,
            failed: 0,
            unresolved: 0,
            not_applicable: 0,
            per_stage: Vec::with_capacity(self.stages.len()),
        };
        let mut seen = BTreeSet::new();

        for stage in &self.stages {
            stage.validate()?;
            if !seen.insert(stage.stage_id.clone()) {
                return Err(ImportError::DuplicateStageId(stage.stage_id.clone()));
            }
            let mut stage_count = StageRateCount {
                stage_id: stage.stage_id.clone(),
                succeeded: 0,
                failed: 0,
                unresolved: 0,
                not_applicable: 0,
            };
            match stage.disposition {
                StageDisposition::Succeeded => {
                    stage_count.succeeded = 1;
                    counts.succeeded += 1;
                }
                StageDisposition::Failed => {
                    stage_count.failed = 1;
                    counts.failed += 1;
                }
                StageDisposition::Unresolved => {
                    stage_count.unresolved = 1;
                    counts.unresolved += 1;
                }
                StageDisposition::NotApplicable => {
                    stage_count.not_applicable = 1;
                    counts.not_applicable += 1;
                }
            }
            counts.per_stage.push(stage_count);
        }

        for expected in &self.task.expected_stage_ids {
            if !seen.contains(expected) {
                return Err(ImportError::MissingStageResult(expected.clone()));
            }
        }
        for actual in &seen {
            if !self.task.expected_stage_ids.contains(actual) {
                return Err(ImportError::UnexpectedStageResult(actual.clone()));
            }
        }
        let actual_stage_order: Vec<&str> =
            self.stages.iter().map(|stage| stage.stage_id.as_str()).collect();
        let expected_stage_order: Vec<&str> = self
            .task
            .expected_stage_ids
            .iter()
            .map(String::as_str)
            .collect();
        if actual_stage_order != expected_stage_order {
            return Err(ImportError::StageOrderMismatch);
        }

        let mut bce_pairs = BTreeSet::new();
        for receipt in &self.bce_receipts {
            receipt.validate()?;
            if receipt.policy_input_digest != self.policy_input.policy_input_digest {
                return Err(ImportError::InvalidBceReceipt);
            }
            if !bce_pairs.insert((
                receipt.source_interaction_id.as_str(),
                receipt.target_interaction_id.as_str(),
            )) {
                return Err(ImportError::DuplicateBceReceipt);
            }
        }

        let mut blockers = Vec::new();
        if self.protocol.dataset_rights_status == DatasetRightsStatus::Unknown {
            blockers.push(QualificationBlocker::DatasetRightsUnresolved);
        }
        if self.policy.training_overlap_status == TrainingOverlapStatus::Unknown {
            blockers.push(QualificationBlocker::TrainingOverlapUnknown);
        }
        if self.task.bce_required && self.bce_receipts.is_empty() {
            blockers.push(QualificationBlocker::BceEvaluationMissing);
        }
        if self.terminal_receipt.as_ref().is_some_and(OutcomeReceipt::has_unknown_source)
            || self.stages.iter().any(|stage| {
                stage
                    .evidence
                    .as_ref()
                    .is_some_and(OutcomeReceipt::has_unknown_source)
            })
        {
            blockers.push(QualificationBlocker::UnknownOutcomeEvidenceSource);
        }
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

    /// Aggregate a frozen task/policy/split arm across episode records.
    /// Task, protocol, stage roster, policy and control identities must match;
    /// per-episode ID, scene ID and seed may differ.
    pub fn aggregate_task_records(
        records: &[MemoryEvaluationRecord],
    ) -> Result<MemoryEvaluationAggregate, ImportError> {
        let first = records.first().ok_or(ImportError::EmptyAggregate)?;
        first.validate()?;

        let mut aggregate = MemoryEvaluationAggregate {
            terminal_counts: TerminalOutcomeCounts {
                succeeded: 0,
                failed: 0,
                unresolved: 0,
            },
            stage_counts: StageCompletionCounts {
                succeeded: 0,
                failed: 0,
                unresolved: 0,
                not_applicable: 0,
                per_stage: first
                    .task
                    .expected_stage_ids
                    .iter()
                    .map(|stage_id| StageRateCount {
                        stage_id: stage_id.clone(),
                        succeeded: 0,
                        failed: 0,
                        unresolved: 0,
                        not_applicable: 0,
                    })
                    .collect(),
            },
        };

        for record in records {
            record.validate()?;
            if record.protocol != first.protocol
                || record.task.task_id != first.task.task_id
                || record.task.split != first.task.split
                || record.task.expected_stage_ids != first.task.expected_stage_ids
                || record.task.bce_required != first.task.bce_required
                || record.policy.variant_id != first.policy.variant_id
                || record.policy.policy_id != first.policy.policy_id
                || record.policy.checkpoint_digest != first.policy.checkpoint_digest
                || record.policy.deployment_digest != first.policy.deployment_digest
                || record.policy.training_data_manifest_digest
                    != first.policy.training_data_manifest_digest
                || record.policy.training_overlap_status != first.policy.training_overlap_status
                || record.policy.assistance != first.policy.assistance
                || record.matched_control.underlying_policy_digest
                    != first.matched_control.underlying_policy_digest
                || record.matched_control.action_executor_digest
                    != first.matched_control.action_executor_digest
                || record.matched_control.robot_or_embodiment_digest
                    != first.matched_control.robot_or_embodiment_digest
                || record.matched_control.resource_budget_digest
                    != first.matched_control.resource_budget_digest
                || record.evidence_class != first.evidence_class
            {
                return Err(ImportError::MixedAggregateGroup);
            }

            match record.terminal_outcome {
                TerminalOutcome::Succeeded => {
                    aggregate.terminal_counts.succeeded += 1;
                }
                TerminalOutcome::Failed => {
                    aggregate.terminal_counts.failed += 1;
                }
                TerminalOutcome::Unresolved => {
                    aggregate.terminal_counts.unresolved += 1;
                }
            }

            for (aggregate_stage, result) in aggregate
                .stage_counts
                .per_stage
                .iter_mut()
                .zip(record.stages.iter())
            {
                match result.disposition {
                    StageDisposition::Succeeded => {
                        aggregate_stage.succeeded += 1;
                        aggregate.stage_counts.succeeded += 1;
                    }
                    StageDisposition::Failed => {
                        aggregate_stage.failed += 1;
                        aggregate.stage_counts.failed += 1;
                    }
                    StageDisposition::Unresolved => {
                        aggregate_stage.unresolved += 1;
                        aggregate.stage_counts.unresolved += 1;
                    }
                    StageDisposition::NotApplicable => {
                        aggregate_stage.not_applicable += 1;
                        aggregate.stage_counts.not_applicable += 1;
                    }
                }
            }
        }

        Ok(aggregate)
    }

    /// Returns true only if the records share every frozen comparison field
    /// other than policy variant/checkpoint/assistance. This is a matched-arm
    /// identity check, not a statistical test.
    pub fn matches_control_conditions(&self, other: &Self) -> bool {
        self.protocol == other.protocol
            && self.task.task_id == other.task.task_id
            && self.task.task_instance_id == other.task.task_instance_id
            && self.task.scene_id == other.task.scene_id
            && self.task.scene_seed == other.task.scene_seed
            && self.task.split == other.task.split
            && self.task.expected_stage_ids == other.task.expected_stage_ids
            && self.task.bce_required == other.task.bce_required
            && self.matched_control == other.matched_control
            && self.evidence_class == other.evidence_class
            && self.policy_input.observation_bundle_digest
                == other.policy_input.observation_bundle_digest
            && self.policy_input.policy_visible_fields
                == other.policy_input.policy_visible_fields
            && self.policy_input.evaluator_private_fields
                == other.policy_input.evaluator_private_fields
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
            dataset_rights_status: DatasetRightsStatus::Verified,
            benchmark_id: "robocasa-mem".into(),
            release_revision: "paper-v1-provisional".into(),
            source_commit: "0123456789abcdef0123456789abcdef01234567".into(),
            task_manifest_digest: digest(1),
            evaluator_digest: digest(2),
            dataset_manifest_digest: digest(3),
            dataset_rights_manifest_digest: digest(24),
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
                bce_required: false,
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
                training_data_manifest_digest: digest(23),
                training_overlap_status: TrainingOverlapStatus::AuditedNoKnownOverlap,
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
    fn matched_episodes_may_have_distinct_run_ids_for_the_same_scene_seed() {
        let left = record("policy-only", AssistanceTrack::PolicyOnly);
        let mut right = record("agent", AssistanceTrack::AgentAssisted);
        right.task.episode_id = "episode-02".into();

        left.require_matched_control(&right).unwrap();
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
    fn unresolved_dataset_rights_and_training_overlap_block_qualification() {
        let mut run = record("policy-only", AssistanceTrack::PolicyOnly);
        run.protocol.dataset_rights_status = DatasetRightsStatus::Unknown;
        run.policy.training_overlap_status = TrainingOverlapStatus::Unknown;

        let audit = run.validate().unwrap();
        assert!(audit
            .qualification_blockers
            .contains(&QualificationBlocker::DatasetRightsUnresolved));
        assert!(audit
            .qualification_blockers
            .contains(&QualificationBlocker::TrainingOverlapUnknown));
        assert_eq!(audit.stage_counts.succeeded, 2);
    }

    #[test]
    fn known_training_overlap_can_be_disclosed_without_erasing_the_result() {
        let mut run = record("policy-only", AssistanceTrack::PolicyOnly);
        run.policy.training_overlap_status = TrainingOverlapStatus::KnownOverlapDisclosed;

        let audit = run.validate().unwrap();
        assert!(!audit
            .qualification_blockers
            .contains(&QualificationBlocker::TrainingOverlapUnknown));
    }

    #[test]
    fn unknown_outcome_source_is_preserved_as_a_qualification_blocker() {
        let mut run = record("policy-only", AssistanceTrack::PolicyOnly);
        run.stages[0].evidence = Some(receipt(OutcomeEvidenceSource::Unknown, 31));
        let audit = run.validate().unwrap();
        assert!(audit
            .qualification_blockers
            .contains(&QualificationBlocker::UnknownOutcomeEvidenceSource));
        assert_eq!(audit.stage_counts.succeeded, 2);
    }

    #[test]
    fn bce_required_task_without_evaluator_receipts_is_blocked() {
        let mut run = record("policy-only", AssistanceTrack::PolicyOnly);
        run.task.bce_required = true;
        let audit = run.validate().unwrap();
        assert!(audit
            .qualification_blockers
            .contains(&QualificationBlocker::BceEvaluationMissing));
        assert_eq!(audit.terminal_outcome, TerminalOutcome::Succeeded);
    }

    #[test]
    fn task_stage_order_is_preserved_not_sorted_by_importer() {
        let mut run = record("policy-only", AssistanceTrack::PolicyOnly);
        run.stages.swap(0, 1);
        assert_eq!(run.validate(), Err(ImportError::StageOrderMismatch));
    }

    #[test]
    fn duplicate_bce_receipt_pairs_reject() {
        let mut run = record("policy-only", AssistanceTrack::PolicyOnly);
        let receipt = BehaviorConditionedReceipt {
            source_interaction_id: "choose-cup-a".into(),
            target_interaction_id: "retrieve-cup-a".into(),
            source_sequence: 4,
            target_sequence: 11,
            predicate_id: "same-entity-v1".into(),
            evaluator_receipt_digest: digest(21),
            policy_input_digest: run.policy_input.policy_input_digest,
        };
        run.bce_receipts = vec![receipt.clone(), receipt];
        assert_eq!(run.validate(), Err(ImportError::DuplicateBceReceipt));
    }

    #[test]
    fn matched_arms_require_identical_policy_visible_observation_manifest() {
        let left = record("policy-only", AssistanceTrack::PolicyOnly);
        let mut right = record("agent", AssistanceTrack::AgentAssisted);
        right.policy_input.observation_bundle_digest = digest(99);
        assert_eq!(
            left.require_matched_control(&right),
            Err(ImportError::MatchedControlMismatch)
        );
    }

    #[test]
    fn unset_artifact_digest_is_rejected() {
        assert_eq!(
            ArtifactDigest::from_bytes([0; 32]),
            Err(ImportError::UnsetArtifactDigest)
        );
    }
}
