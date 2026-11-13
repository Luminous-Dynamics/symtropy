//! Finite, evidence-bounded construction recovery policy reference evaluator.
//!
//! A policy branch is a pre-declared proposal template. Evaluation can emit only
//! Hold or Proposal; it never grants a resource, issues a permit, starts a robot,
//! authorizes a lift, or accepts physical work.
//!
//! Unknown / incomplete / invalidated / unavailable observation outcomes fail closed
//! to Hold. All proposals require revalidation and external authority before execution.

use std::collections::BTreeSet;

use construction_observation_planning_core::ObservationOutcome;
use construction_robot_interop_core::ContextBinding;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationStatus {
    QualifiedClear,
    Finding,
    Incomplete,
    Invalidated,
    Unavailable,
}

impl From<&ObservationOutcome> for ObservationStatus {
    fn from(outcome: &ObservationOutcome) -> Self {
        match outcome {
            ObservationOutcome::QualifiedClear { .. } => Self::QualifiedClear,
            ObservationOutcome::Finding { .. } => Self::Finding,
            ObservationOutcome::Incomplete { .. } => Self::Incomplete,
            ObservationOutcome::Invalidated { .. } => Self::Invalidated,
            ObservationOutcome::Unavailable { .. } => Self::Unavailable,
        }
    }
}

impl ObservationStatus {
    fn is_unknown(self) -> bool {
        matches!(self, Self::Incomplete | Self::Invalidated | Self::Unavailable)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TriggerKey {
    Observation {
        region_id: String,
        status: ObservationStatus,
    },
    ResourceUnavailable {
        resource_id: String,
    },
    MaterialUnavailable {
        lot_id: String,
    },
    InspectionRejected {
        work_unit_id: String,
    },
    SiteEpochChanged {
        previous_epoch: String,
        current_epoch: String,
    },
}

impl TriggerKey {
    fn validate(&self) -> Result<(), PolicyError> {
        match self {
            Self::Observation { region_id, .. } => require_non_empty("region_id", region_id),
            Self::ResourceUnavailable { resource_id } => {
                require_non_empty("resource_id", resource_id)
            }
            Self::MaterialUnavailable { lot_id } => require_non_empty("lot_id", lot_id),
            Self::InspectionRejected { work_unit_id } => {
                require_non_empty("work_unit_id", work_unit_id)
            }
            Self::SiteEpochChanged {
                previous_epoch,
                current_epoch,
            } => {
                require_non_empty("previous_epoch", previous_epoch)?;
                require_non_empty("current_epoch", current_epoch)?;
                if previous_epoch == current_epoch {
                    return Err(PolicyError::InvalidEpochTransition);
                }
                Ok(())
            }
        }
    }

    fn requires_hold_without_new_epoch_policy(&self) -> bool {
        matches!(
            self,
            Self::Observation {
                status: ObservationStatus::Incomplete
                    | ObservationStatus::Invalidated
                    | ObservationStatus::Unavailable,
                ..
            } | Self::SiteEpochChanged { .. }
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TriggerEvidence {
    pub trigger: TriggerKey,
    pub event_ref: String,
    pub binding: ContextBinding,
    pub observed_at_ms: u64,
}

impl TriggerEvidence {
    fn validate(&self) -> Result<(), PolicyError> {
        self.trigger.validate()?;
        require_non_empty("event_ref", &self.event_ref)?;
        validate_binding(&self.binding)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityRole {
    SiteSupervisor,
    LiftingSupervisor,
    StructuralEngineer,
    MaterialAuthority,
    QualityInspector,
    SafetyOfficer,
    ProjectEngineer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Explicitly stop branch selection and reconcile physical/evidence state.
    HoldAndReconcile {
        reason: String,
    },
    /// Reorder only declared independent tasks; the schedule proof is an external
    /// reference and must be checked against the current task/resource owners.
    ResequenceIndependentTasks {
        task_ids: Vec<String>,
        schedule_proof_ref: String,
    },
    /// Propose a previously qualified alternative resource; no resource is granted.
    SwitchToQualifiedResource {
        resource_id: String,
        capability_qualification_ref: String,
    },
    /// Propose only an explicitly approved substitution reference.
    SubstituteApprovedMaterial {
        from_lot_id: String,
        to_lot_id: String,
        substitution_approval_ref: String,
    },
    /// Propose repair followed by independent reinspection.
    CorrectAndReinspect {
        work_unit_id: String,
        repair_procedure_ref: String,
    },
    /// Stop affected dependent work and request accountable external disposition.
    RequestProfessionalDisposition {
        work_unit_id: String,
        authority: AuthorityRole,
    },
    /// Propose an explicit stop for affected tasks; this is not a safety controller.
    StopAffectedWork {
        task_ids: Vec<String>,
    },
}

impl RecoveryAction {
    fn is_hold(&self) -> bool {
        matches!(self, Self::HoldAndReconcile { .. })
    }

    fn validate(&self) -> Result<(), PolicyError> {
        match self {
            Self::HoldAndReconcile { reason } => require_non_empty("hold_reason", reason),
            Self::ResequenceIndependentTasks {
                task_ids,
                schedule_proof_ref,
            } => {
                validate_unique_non_empty("task_ids", task_ids, 2)?;
                require_non_empty("schedule_proof_ref", schedule_proof_ref)
            }
            Self::SwitchToQualifiedResource {
                resource_id,
                capability_qualification_ref,
            } => {
                require_non_empty("resource_id", resource_id)?;
                require_non_empty(
                    "capability_qualification_ref",
                    capability_qualification_ref,
                )
            }
            Self::SubstituteApprovedMaterial {
                from_lot_id,
                to_lot_id,
                substitution_approval_ref,
            } => {
                require_non_empty("from_lot_id", from_lot_id)?;
                require_non_empty("to_lot_id", to_lot_id)?;
                if from_lot_id == to_lot_id {
                    return Err(PolicyError::NoOpMaterialSubstitution);
                }
                require_non_empty(
                    "substitution_approval_ref",
                    substitution_approval_ref,
                )
            }
            Self::CorrectAndReinspect {
                work_unit_id,
                repair_procedure_ref,
            } => {
                require_non_empty("work_unit_id", work_unit_id)?;
                require_non_empty("repair_procedure_ref", repair_procedure_ref)
            }
            Self::RequestProfessionalDisposition { work_unit_id, .. } => {
                require_non_empty("work_unit_id", work_unit_id)
            }
            Self::StopAffectedWork { task_ids } => {
                validate_unique_non_empty("task_ids", task_ids, 1)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyBranch {
    pub branch_id: String,
    pub trigger: TriggerKey,
    pub action: RecoveryAction,
    /// Opaque references to externally owned qualification records. The evaluator
    /// checks presence/identity binding only; it does not authenticate their contents.
    pub qualification_refs: Vec<String>,
    /// Every non-hold proposal names the approval roles required before execution.
    pub required_approvals: Vec<AuthorityRole>,
    pub max_evidence_age_ms: u64,
}

impl PolicyBranch {
    fn validate(&self) -> Result<(), PolicyError> {
        require_non_empty("branch_id", &self.branch_id)?;
        self.trigger.validate()?;
        self.action.validate()?;

        if !self.action.is_hold() && self.qualification_refs.is_empty() {
            return Err(PolicyError::MissingQualificationReferences);
        }
        for reference in &self.qualification_refs {
            require_non_empty("qualification_ref", reference)?;
        }

        let mut approvals = BTreeSet::new();
        for approval in &self.required_approvals {
            if !approvals.insert(*approval as u8) {
                return Err(PolicyError::DuplicateApprovalRole);
            }
        }
        if !self.action.is_hold() && self.required_approvals.is_empty() {
            return Err(PolicyError::MissingExternalApproval);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContingentPolicy {
    pub policy_id: String,
    pub binding: ContextBinding,
    pub branches: Vec<PolicyBranch>,
}

impl ContingentPolicy {
    pub fn validate(&self) -> Result<(), PolicyError> {
        require_non_empty("policy_id", &self.policy_id)?;
        validate_binding(&self.binding)?;

        let mut branch_ids = BTreeSet::new();
        for branch in &self.branches {
            branch.validate()?;
            if !branch_ids.insert(branch.branch_id.as_str()) {
                return Err(PolicyError::DuplicateBranch(branch.branch_id.clone()));
            }
            if self
                .branches
                .iter()
                .filter(|other| other.trigger == branch.trigger)
                .count()
                > 1
            {
                return Err(PolicyError::AmbiguousTrigger(branch.trigger.clone()));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionContext {
    pub binding: ContextBinding,
    pub decision_time_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HoldReason {
    NoTriggerEvidence,
    PolicyContextStale,
    TriggerContextMismatch,
    FutureEvidence,
    NoMatchingBranch,
    EvidenceTooOld,
    ObservationUnknown,
    SiteEpochChanged,
    BranchRequestsHold,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyDecision {
    Hold {
        policy_id: String,
        reason: HoldReason,
        trigger_ref: Option<String>,
    },
    /// A proposal is not executable. The caller must revalidate current resource,
    /// occupancy, capability and authority state and obtain external approvals.
    Proposal {
        policy_id: String,
        branch_id: String,
        action: RecoveryAction,
        required_approvals: Vec<AuthorityRole>,
        qualification_refs: Vec<String>,
        trigger_ref: String,
        must_revalidate_before_execution: bool,
    },
}

pub fn evaluate_policy(
    policy: &ContingentPolicy,
    context: &DecisionContext,
    evidence: Option<&TriggerEvidence>,
) -> Result<PolicyDecision, PolicyError> {
    policy.validate()?;
    validate_binding(&context.binding)?;

    if policy.binding != context.binding {
        return Ok(hold(policy, HoldReason::PolicyContextStale, None));
    }

    let Some(evidence) = evidence else {
        return Ok(hold(policy, HoldReason::NoTriggerEvidence, None));
    };
    evidence.validate()?;

    if evidence.binding != context.binding {
        return Ok(hold(
            policy,
            HoldReason::TriggerContextMismatch,
            Some(evidence.event_ref.clone()),
        ));
    }
    if evidence.observed_at_ms > context.decision_time_ms {
        return Ok(hold(
            policy,
            HoldReason::FutureEvidence,
            Some(evidence.event_ref.clone()),
        ));
    }
    if evidence.trigger.requires_hold_without_new_epoch_policy() {
        let reason = if matches!(evidence.trigger, TriggerKey::SiteEpochChanged { .. }) {
            HoldReason::SiteEpochChanged
        } else {
            HoldReason::ObservationUnknown
        };
        return Ok(hold(
            policy,
            reason,
            Some(evidence.event_ref.clone()),
        ));
    }

    let Some(branch) = policy
        .branches
        .iter()
        .find(|branch| branch.trigger == evidence.trigger)
    else {
        return Ok(hold(
            policy,
            HoldReason::NoMatchingBranch,
            Some(evidence.event_ref.clone()),
        ));
    };

    if context.decision_time_ms - evidence.observed_at_ms > branch.max_evidence_age_ms {
        return Ok(hold(
            policy,
            HoldReason::EvidenceTooOld,
            Some(evidence.event_ref.clone()),
        ));
    }

    if branch.action.is_hold() {
        return Ok(hold(
            policy,
            HoldReason::BranchRequestsHold,
            Some(evidence.event_ref.clone()),
        ));
    }

    Ok(PolicyDecision::Proposal {
        policy_id: policy.policy_id.clone(),
        branch_id: branch.branch_id.clone(),
        action: branch.action.clone(),
        required_approvals: branch.required_approvals.clone(),
        qualification_refs: branch.qualification_refs.clone(),
        trigger_ref: evidence.event_ref.clone(),
        must_revalidate_before_execution: true,
    })
}

fn hold(
    policy: &ContingentPolicy,
    reason: HoldReason,
    trigger_ref: Option<String>,
) -> PolicyDecision {
    PolicyDecision::Hold {
        policy_id: policy.policy_id.clone(),
        reason,
        trigger_ref,
    }
}

fn validate_binding(binding: &ContextBinding) -> Result<(), PolicyError> {
    require_non_empty("site_epoch", &binding.site_epoch)?;
    require_non_empty(
        "design_revision_digest",
        &binding.design_revision_digest,
    )?;
    require_non_empty("target_frame_id", &binding.target_frame_id)?;
    require_non_empty("robot_config_digest", &binding.robot_config_digest)
}

fn validate_unique_non_empty(
    name: &'static str,
    values: &[String],
    minimum: usize,
) -> Result<(), PolicyError> {
    if values.len() < minimum {
        return Err(PolicyError::TooFewValues(name));
    }
    let mut unique = BTreeSet::new();
    for value in values {
        require_non_empty(name, value)?;
        if !unique.insert(value.as_str()) {
            return Err(PolicyError::DuplicateValue(name));
        }
    }
    Ok(())
}

fn require_non_empty(name: &'static str, value: &str) -> Result<(), PolicyError> {
    if value.trim().is_empty() {
        return Err(PolicyError::EmptyField(name));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyError {
    EmptyField(&'static str),
    TooFewValues(&'static str),
    DuplicateValue(&'static str),
    DuplicateBranch(String),
    AmbiguousTrigger(TriggerKey),
    DuplicateApprovalRole,
    MissingQualificationReferences,
    MissingExternalApproval,
    InvalidEpochTransition,
    NoOpMaterialSubstitution,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding() -> ContextBinding {
        ContextBinding {
            site_epoch: "site-epoch-1".to_owned(),
            design_revision_digest: "design-sha256-1".to_owned(),
            target_frame_id: "project-grid-1".to_owned(),
            robot_config_digest: "robot-config-sha256-1".to_owned(),
        }
    }

    fn policy(branches: Vec<PolicyBranch>) -> ContingentPolicy {
        ContingentPolicy {
            policy_id: "floor-recovery-policy-01".to_owned(),
            binding: binding(),
            branches,
        }
    }

    fn hoist_trigger() -> TriggerKey {
        TriggerKey::ResourceUnavailable {
            resource_id: "HOIST-1".to_owned(),
        }
    }

    fn qualified_branch() -> PolicyBranch {
        PolicyBranch {
            branch_id: "hoist-alternate-01".to_owned(),
            trigger: hoist_trigger(),
            action: RecoveryAction::SwitchToQualifiedResource {
                resource_id: "HOIST-2".to_owned(),
                capability_qualification_ref: "capability-qualification:hoist-2".to_owned(),
            },
            qualification_refs: vec![
                "alternate-resource-approval:hoist-2".to_owned(),
                "lift-envelope-review:floor-01".to_owned(),
            ],
            required_approvals: vec![
                AuthorityRole::LiftingSupervisor,
                AuthorityRole::SiteSupervisor,
            ],
            max_evidence_age_ms: 60_000,
        }
    }

    fn context() -> DecisionContext {
        DecisionContext {
            binding: binding(),
            decision_time_ms: 1_000,
        }
    }

    fn evidence(trigger: TriggerKey) -> TriggerEvidence {
        TriggerEvidence {
            trigger,
            event_ref: "event:hoist-outage-01".to_owned(),
            binding: binding(),
            observed_at_ms: 900,
        }
    }

    fn assert_hold(decision: PolicyDecision, reason: HoldReason) {
        assert!(matches!(
            decision,
            PolicyDecision::Hold {
                reason: actual,
                ..
            } if actual == reason
        ));
    }

    #[test]
    fn qualified_trigger_emits_proposal_with_approval_and_revalidation() {
        let policy = policy(vec![qualified_branch()]);
        let decision = evaluate_policy(
            &policy,
            &context(),
            Some(&evidence(hoist_trigger())),
        )
        .expect("policy evaluation");

        assert_eq!(
            decision,
            PolicyDecision::Proposal {
                policy_id: "floor-recovery-policy-01".to_owned(),
                branch_id: "hoist-alternate-01".to_owned(),
                action: qualified_branch().action,
                required_approvals: vec![
                    AuthorityRole::LiftingSupervisor,
                    AuthorityRole::SiteSupervisor,
                ],
                qualification_refs: vec![
                    "alternate-resource-approval:hoist-2".to_owned(),
                    "lift-envelope-review:floor-01".to_owned(),
                ],
                trigger_ref: "event:hoist-outage-01".to_owned(),
                must_revalidate_before_execution: true,
            }
        );
    }

    #[test]
    fn no_trigger_evidence_holds() {
        let decision = evaluate_policy(&policy(vec![qualified_branch()]), &context(), None)
            .expect("no-evidence hold");
        assert_hold(decision, HoldReason::NoTriggerEvidence);
    }

    #[test]
    fn unmatched_trigger_holds_instead_of_inventing_a_recovery() {
        let policy = policy(vec![qualified_branch()]);
        let trigger = TriggerKey::MaterialUnavailable {
            lot_id: "LOT-77".to_owned(),
        };
        let decision =
            evaluate_policy(&policy, &context(), Some(&evidence(trigger)))
                .expect("unmatched trigger hold");
        assert_hold(decision, HoldReason::NoMatchingBranch);
    }

    #[test]
    fn incomplete_observation_stays_unknown_even_when_no_recovery_branch_exists() {
        let policy = policy(vec![qualified_branch()]);
        let trigger = TriggerKey::Observation {
            region_id: "workface-A".to_owned(),
            status: ObservationStatus::Incomplete,
        };
        let decision =
            evaluate_policy(&policy, &context(), Some(&evidence(trigger)))
                .expect("unknown observation hold");
        assert_hold(decision, HoldReason::ObservationUnknown);
    }

    #[test]
    fn invalidated_and_unavailable_observations_hold() {
        for status in [
            ObservationStatus::Invalidated,
            ObservationStatus::Unavailable,
        ] {
            let trigger = TriggerKey::Observation {
                region_id: "workface-A".to_owned(),
                status,
            };
            let decision = evaluate_policy(
                &policy(vec![qualified_branch()]),
                &context(),
                Some(&evidence(trigger)),
            )
            .expect("unknown status hold");
            assert_hold(decision, HoldReason::ObservationUnknown);
        }
    }

    #[test]
    fn site_epoch_change_always_holds_under_the_old_policy() {
        let trigger = TriggerKey::SiteEpochChanged {
            previous_epoch: "site-epoch-1".to_owned(),
            current_epoch: "site-epoch-2".to_owned(),
        };
        let decision = evaluate_policy(
            &policy(vec![qualified_branch()]),
            &context(),
            Some(&evidence(trigger)),
        )
        .expect("epoch transition hold");
        assert_hold(decision, HoldReason::SiteEpochChanged);
    }

    #[test]
    fn stale_policy_context_holds_before_branch_selection() {
        let mut policy = policy(vec![qualified_branch()]);
        policy.binding.site_epoch = "old-site-epoch".to_owned();
        let decision = evaluate_policy(
            &policy,
            &context(),
            Some(&evidence(hoist_trigger())),
        )
        .expect("stale policy hold");
        assert_hold(decision, HoldReason::PolicyContextStale);
    }

    #[test]
    fn mismatched_evidence_context_holds() {
        let mut evidence = evidence(hoist_trigger());
        evidence.binding.target_frame_id = "old-frame".to_owned();
        let decision =
            evaluate_policy(&policy(vec![qualified_branch()]), &context(), Some(&evidence))
                .expect("mismatched context hold");
        assert_hold(decision, HoldReason::TriggerContextMismatch);
    }

    #[test]
    fn future_or_over_age_evidence_holds() {
        let mut future = evidence(hoist_trigger());
        future.observed_at_ms = 1_001;
        let decision =
            evaluate_policy(&policy(vec![qualified_branch()]), &context(), Some(&future))
                .expect("future evidence hold");
        assert_hold(decision, HoldReason::FutureEvidence);

        let mut old = evidence(hoist_trigger());
        old.observed_at_ms = 0;
        let decision =
            evaluate_policy(&policy(vec![qualified_branch()]), &context(), Some(&old))
                .expect("old evidence hold");
        assert_hold(decision, HoldReason::EvidenceTooOld);
    }

    #[test]
    fn mutation_proposal_without_external_approval_is_invalid() {
        let mut branch = qualified_branch();
        branch.required_approvals.clear();
        assert_eq!(
            policy(vec![branch]).validate(),
            Err(PolicyError::MissingExternalApproval)
        );
    }

    #[test]
    fn mutation_proposal_without_qualification_reference_is_invalid() {
        let mut branch = qualified_branch();
        branch.qualification_refs.clear();
        assert_eq!(
            policy(vec![branch]).validate(),
            Err(PolicyError::MissingQualificationReferences)
        );
    }

    #[test]
    fn duplicate_branches_and_ambiguous_triggers_are_rejected() {
        let first = qualified_branch();
        let mut duplicate_id = qualified_branch();
        duplicate_id.trigger = TriggerKey::MaterialUnavailable {
            lot_id: "LOT-2".to_owned(),
        };
        assert_eq!(
            policy(vec![first.clone(), duplicate_id]).validate(),
            Err(PolicyError::DuplicateBranch(
                "hoist-alternate-01".to_owned()
            ))
        );

        let mut other = qualified_branch();
        other.branch_id = "second-hoist-branch".to_owned();
        assert_eq!(
            policy(vec![first, other]).validate(),
            Err(PolicyError::AmbiguousTrigger(hoist_trigger()))
        );
    }

    #[test]
    fn finding_trigger_can_only_emit_a_proposal_not_independent_acceptance() {
        let mut branch = qualified_branch();
        branch.branch_id = "inspection-rework-01".to_owned();
        branch.trigger = TriggerKey::Observation {
            region_id: "workface-A".to_owned(),
            status: ObservationStatus::Finding,
        };
        branch.action = RecoveryAction::CorrectAndReinspect {
            work_unit_id: "panel-A".to_owned(),
            repair_procedure_ref: "repair-procedure:panel-A".to_owned(),
        };
        branch.required_approvals = vec![AuthorityRole::QualityInspector];
        branch.qualification_refs = vec!["approved-repair-scope:panel-A".to_owned()];

        let decision = evaluate_policy(
            &policy(vec![branch]),
            &context(),
            Some(&evidence(TriggerKey::Observation {
                region_id: "workface-A".to_owned(),
                status: ObservationStatus::Finding,
            })),
        )
        .expect("finding branch proposal");

        assert!(matches!(
            decision,
            PolicyDecision::Proposal {
                action: RecoveryAction::CorrectAndReinspect { .. },
                required_approvals,
                must_revalidate_before_execution: true,
                ..
            } if required_approvals == vec![AuthorityRole::QualityInspector]
        ));
    }
}
