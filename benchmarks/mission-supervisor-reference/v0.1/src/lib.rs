// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Evidence-gated mission supervisor reference.
//!
//! This crate does not command hardware or authenticate observers. It enforces
//! deterministic state transitions after the caller has authenticated and
//! canonicalized evidence receipts. A command acknowledgement is never a
//! completion receipt; ambiguous outcomes hold the mutation gate.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{Display, Formatter};

pub const MAX_CONFIDENCE_BASIS_POINTS: u16 = 10_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Digest(pub [u8; 32]);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SupervisorError {
    InvalidMissionId,
    EmptyMission,
    InvalidConfidenceThreshold,
    InvalidSubgoalId,
    DuplicateSubgoalId,
    UnknownDependency,
    DependencyMustPrecedeSubgoal,
    DuplicateDependency,
    InvalidPredicateId,
    InvalidRecoveryPlanSet,
    UnknownSubgoal,
    DependencyNotComplete,
    ActionAlreadyInFlight,
    WrongActiveSubgoal,
    InvalidStateTransition,
    ActionIdReused,
    InvalidActionId,
    ActionIdMismatch,
    ActionAttemptMismatch,
    UntrustedObserver,
    InvalidConfidence,
    StaleSourceSequence,
    SourceTimestampRegression,
    SourceTimeBasisMismatch,
    InvalidTimeQualification,
    EventIdConflict,
    Halted,
    RecoveryPlanNotApproved,
    UnsafeRecoveryPermit,
}

impl Display for SupervisorError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMissionId => write!(formatter, "mission ID must not be empty"),
            Self::EmptyMission => write!(formatter, "mission must contain at least one subgoal"),
            Self::InvalidConfidenceThreshold => write!(
                formatter,
                "confidence threshold must be in 0..=10000 basis points"
            ),
            Self::InvalidSubgoalId => write!(formatter, "subgoal ID must not be empty"),
            Self::DuplicateSubgoalId => write!(formatter, "subgoal IDs must be unique"),
            Self::UnknownDependency => write!(formatter, "subgoal dependency does not exist"),
            Self::DependencyMustPrecedeSubgoal => write!(
                formatter,
                "dependencies must appear before dependent subgoals"
            ),
            Self::DuplicateDependency => {
                write!(formatter, "dependency IDs must be unique per subgoal")
            }
            Self::InvalidPredicateId => write!(formatter, "predicate ID must not be empty"),
            Self::InvalidRecoveryPlanSet => {
                write!(formatter, "recovery plan digests must be unique")
            }
            Self::UnknownSubgoal => write!(formatter, "subgoal does not exist"),
            Self::DependencyNotComplete => write!(
                formatter,
                "all declared dependencies must be complete before dispatch"
            ),
            Self::ActionAlreadyInFlight => {
                write!(formatter, "another action has unresolved effects")
            }
            Self::WrongActiveSubgoal => {
                write!(formatter, "receipt does not refer to the active subgoal")
            }
            Self::InvalidStateTransition => write!(
                formatter,
                "operation is invalid for the current subgoal state"
            ),
            Self::ActionIdReused => write!(formatter, "action IDs must be unique within a mission"),
            Self::InvalidActionId => write!(formatter, "action ID must not be empty"),
            Self::ActionIdMismatch => {
                write!(
                    formatter,
                    "receipt action ID does not match the active action"
                )
            }
            Self::ActionAttemptMismatch => {
                write!(
                    formatter,
                    "receipt attempt does not match the active attempt"
                )
            }
            Self::UntrustedObserver => write!(
                formatter,
                "agent self-reports cannot authorize state transitions"
            ),
            Self::InvalidConfidence => {
                write!(formatter, "confidence must be in 0..=10000 basis points")
            }
            Self::StaleSourceSequence => write!(formatter, "source sequence is stale or replayed"),
            Self::SourceTimestampRegression => {
                write!(formatter, "source observation timestamp regressed")
            }
            Self::SourceTimeBasisMismatch => {
                write!(formatter, "source timestamp comparison basis changed")
            }
            Self::InvalidTimeQualification => {
                write!(formatter, "receipt time qualification is incomplete")
            }
            Self::EventIdConflict => write!(
                formatter,
                "event ID was reused with a different digest; supervisor halted"
            ),
            Self::Halted => write!(formatter, "supervisor is halted"),
            Self::RecoveryPlanNotApproved => write!(
                formatter,
                "recovery plan digest is not pre-approved for this subgoal"
            ),
            Self::UnsafeRecoveryPermit => write!(
                formatter,
                "recovery permit does not prove a safe, sufficiently confident recovery"
            ),
        }
    }
}

impl Error for SupervisorError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubgoalSpec {
    pub id: String,
    pub predicate_id: String,
    pub depends_on: Vec<String>,
    /// Allowlist of independently reviewed recovery plan digests.
    pub approved_recovery_plans: Vec<Digest>,
}

impl SubgoalSpec {
    pub fn new(
        id: impl Into<String>,
        predicate_id: impl Into<String>,
        depends_on: Vec<String>,
        approved_recovery_plans: Vec<Digest>,
    ) -> Self {
        Self {
            id: id.into(),
            predicate_id: predicate_id.into(),
            depends_on,
            approved_recovery_plans,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissionPlan {
    mission_id: String,
    min_confidence_basis_points: u16,
    subgoals: Vec<SubgoalSpec>,
}

impl MissionPlan {
    /// Dependencies must be declared earlier in the supplied plan order.
    /// Requiring a topological order makes cycle rejection simple and auditable.
    pub fn new(
        mission_id: impl Into<String>,
        min_confidence_basis_points: u16,
        subgoals: Vec<SubgoalSpec>,
    ) -> Result<Self, SupervisorError> {
        let mission_id = mission_id.into();
        if mission_id.trim().is_empty() {
            return Err(SupervisorError::InvalidMissionId);
        }
        if !(1..=MAX_CONFIDENCE_BASIS_POINTS).contains(&min_confidence_basis_points) {
            return Err(SupervisorError::InvalidConfidenceThreshold);
        }
        if subgoals.is_empty() {
            return Err(SupervisorError::EmptyMission);
        }

        let mut seen = BTreeMap::<String, usize>::new();
        for (index, subgoal) in subgoals.iter().enumerate() {
            if subgoal.id.trim().is_empty() {
                return Err(SupervisorError::InvalidSubgoalId);
            }
            if subgoal.predicate_id.trim().is_empty() {
                return Err(SupervisorError::InvalidPredicateId);
            }
            if seen.insert(subgoal.id.clone(), index).is_some() {
                return Err(SupervisorError::DuplicateSubgoalId);
            }
            let unique_dependencies: BTreeSet<&str> =
                subgoal.depends_on.iter().map(String::as_str).collect();
            if unique_dependencies.len() != subgoal.depends_on.len() {
                return Err(SupervisorError::DuplicateDependency);
            }
            let unique_recovery_plans: BTreeSet<Digest> =
                subgoal.approved_recovery_plans.iter().copied().collect();
            if unique_recovery_plans.len() != subgoal.approved_recovery_plans.len() {
                return Err(SupervisorError::InvalidRecoveryPlanSet);
            }
        }

        for (index, subgoal) in subgoals.iter().enumerate() {
            for dependency in &subgoal.depends_on {
                match seen.get(dependency) {
                    None => return Err(SupervisorError::UnknownDependency),
                    Some(dependency_index) if *dependency_index >= index => {
                        return Err(SupervisorError::DependencyMustPrecedeSubgoal);
                    }
                    Some(_) => {}
                }
            }
        }

        Ok(Self {
            mission_id,
            min_confidence_basis_points,
            subgoals,
        })
    }

    pub fn mission_id(&self) -> &str {
        &self.mission_id
    }

    pub fn min_confidence_basis_points(&self) -> u16 {
        self.min_confidence_basis_points
    }

    pub fn subgoals(&self) -> &[SubgoalSpec] {
        &self.subgoals
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObserverRole {
    AgentSelfReport,
    IndependentSensor,
    IndependentEvaluator,
}

impl ObserverRole {
    fn is_independent(self) -> bool {
        matches!(self, Self::IndependentSensor | Self::IndependentEvaluator)
    }
}

/// Provenance for normalized observed_at_ms. The destination is a qualified
/// time scale/profile plus an explicit epoch, not an assumed clock. M1 consumes
/// a caller-qualified result; it never performs clock conversion or authenticates
/// these opaque commitments.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceiptTimeQualification {
    pub source_clock_id: String,
    pub source_boot_epoch_id: String,
    pub comparison_basis_id: String,
    pub comparison_epoch_id: String,
    /// Commitment to the raw source clock reading.
    pub source_time_commitment: Digest,
    /// Commitment to the exact clock-correlation profile and parameters.
    pub correlation_commitment: Digest,
    /// Commitment to the exact normalized destination reading.
    pub normalized_time_commitment: Digest,
    /// Conservative hard error bound of the normalized instant, rounded up.
    pub uncertainty_bound_nanoseconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceiptEnvelope {
    pub event_id: String,
    pub content_digest: Digest,
    pub source_id: String,
    pub source_sequence: u64,
    /// Milliseconds in the qualified destination time scale and epoch declared below.
    pub observed_at_ms: u64,
    pub time_qualification: ReceiptTimeQualification,
    pub observer_role: ObserverRole,
    pub confidence_basis_points: u16,
}

impl ReceiptEnvelope {
    fn validate(&self) -> Result<(), SupervisorError> {
        if !self.observer_role.is_independent() {
            return Err(SupervisorError::UntrustedObserver);
        }
        if self.event_id.trim().is_empty() || self.source_id.trim().is_empty() {
            return Err(SupervisorError::InvalidStateTransition);
        }
        if self.confidence_basis_points > MAX_CONFIDENCE_BASIS_POINTS {
            return Err(SupervisorError::InvalidConfidence);
        }
        let time = &self.time_qualification;
        if time.source_clock_id.trim().is_empty()
            || time.source_boot_epoch_id.trim().is_empty()
            || time.comparison_basis_id.trim().is_empty()
            || time.comparison_epoch_id.trim().is_empty()
        {
            return Err(SupervisorError::InvalidTimeQualification);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionDisposition {
    Acknowledged,
    KnownNoEffect,
    OutcomeUnknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GoalVerdict {
    Satisfied,
    NotSatisfied,
    Ambiguous,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoalEvidence {
    pub envelope: ReceiptEnvelope,
    pub subgoal_id: String,
    /// Action and attempt that produced this observation.
    pub action_id: String,
    pub attempt: u32,
    pub predicate_id: String,
    pub verdict: GoalVerdict,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconciliationVerdict {
    GoalSatisfied,
    NoEffect,
    PartialEffect,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconciliationEvidence {
    pub envelope: ReceiptEnvelope,
    pub subgoal_id: String,
    /// Action and attempt being reconciled.
    pub action_id: String,
    pub attempt: u32,
    pub verdict: ReconciliationVerdict,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryPermit {
    pub envelope: ReceiptEnvelope,
    pub subgoal_id: String,
    /// Recovery authority applies only to this exact failed action attempt.
    pub action_id: String,
    pub attempt: u32,
    pub recovery_plan_digest: Digest,
    pub safe_to_mutate: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionTicket {
    pub mission_id: String,
    pub subgoal_id: String,
    pub action_id: String,
    pub attempt: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubgoalPhase {
    Pending,
    Executing,
    AwaitingEvidence,
    ReconciliationRequired,
    RecoveryRequired,
    Complete,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecisionKind {
    ActionIssued {
        action_id: String,
        attempt: u32,
    },
    ExecutionAcknowledged {
        action_id: String,
    },
    ExecutionNoEffect {
        action_id: String,
    },
    ExecutionOutcomeUnknown {
        action_id: String,
    },
    GoalCompleted {
        event_id: String,
        evidence_digest: Digest,
    },
    GoalNotSatisfied {
        event_id: String,
    },
    EvidenceAmbiguous {
        event_id: String,
    },
    ReconciliationCompleted {
        event_id: String,
    },
    ReconciliationNoEffect {
        event_id: String,
    },
    ReconciliationPartialEffect {
        event_id: String,
    },
    ReconciliationUnknown {
        event_id: String,
    },
    RecoveryAuthorized {
        event_id: String,
        plan_digest: Digest,
        next_attempt: u32,
    },
    EventIdentityConflict {
        event_id: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decision {
    pub mission_id: String,
    pub subgoal_id: String,
    pub kind: DecisionKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RuntimeState {
    Pending,
    Executing(ActionTicket),
    AwaitingEvidence(ActionTicket),
    ReconciliationRequired(ActionTicket),
    RecoveryRequired(ActionTicket),
    Complete {
        event_id: String,
        evidence_digest: Digest,
    },
}

#[derive(Clone, Debug)]
struct SubgoalRuntime {
    spec: SubgoalSpec,
    state: RuntimeState,
    attempts: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiptDisposition {
    Applied,
    Duplicate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourceCursor {
    sequence: u64,
    observed_at_ms: u64,
    source_clock_id: String,
    source_boot_epoch_id: String,
    comparison_basis_id: String,
    comparison_epoch_id: String,
    correlation_commitment: Digest,
}

/// Single-writer deterministic state machine. It allows at most one unresolved
/// external mutation across the mission at any time.
#[derive(Clone, Debug)]
pub struct MissionSupervisor {
    plan: MissionPlan,
    states: BTreeMap<String, SubgoalRuntime>,
    active_subgoal: Option<String>,
    used_action_ids: BTreeSet<String>,
    seen_events: BTreeMap<String, Digest>,
    source_cursors: BTreeMap<String, SourceCursor>,
    decisions: Vec<Decision>,
    halted: bool,
}

impl MissionSupervisor {
    pub fn new(plan: MissionPlan) -> Self {
        let states = plan
            .subgoals
            .iter()
            .cloned()
            .map(|spec| {
                (
                    spec.id.clone(),
                    SubgoalRuntime {
                        spec,
                        state: RuntimeState::Pending,
                        attempts: 0,
                    },
                )
            })
            .collect();
        Self {
            plan,
            states,
            active_subgoal: None,
            used_action_ids: BTreeSet::new(),
            seen_events: BTreeMap::new(),
            source_cursors: BTreeMap::new(),
            decisions: Vec::new(),
            halted: false,
        }
    }

    pub fn dispatch_action(
        &mut self,
        subgoal_id: &str,
        action_id: impl Into<String>,
    ) -> Result<ActionTicket, SupervisorError> {
        self.ensure_running()?;
        let action_id = action_id.into();
        if action_id.trim().is_empty() {
            return Err(SupervisorError::InvalidActionId);
        }
        if self.used_action_ids.contains(&action_id) {
            return Err(SupervisorError::ActionIdReused);
        }
        let runtime = self
            .states
            .get(subgoal_id)
            .ok_or(SupervisorError::UnknownSubgoal)?;
        if self
            .active_subgoal
            .as_deref()
            .is_some_and(|active| active != subgoal_id)
        {
            return Err(SupervisorError::ActionAlreadyInFlight);
        }
        if runtime.state != RuntimeState::Pending {
            return Err(SupervisorError::InvalidStateTransition);
        }
        for dependency in &runtime.spec.depends_on {
            if !matches!(
                self.states.get(dependency).map(|x| &x.state),
                Some(RuntimeState::Complete { .. })
            ) {
                return Err(SupervisorError::DependencyNotComplete);
            }
        }

        let runtime = self.states.get_mut(subgoal_id).expect("checked above");
        runtime.attempts = runtime.attempts.saturating_add(1);
        let ticket = ActionTicket {
            mission_id: self.plan.mission_id.clone(),
            subgoal_id: subgoal_id.to_owned(),
            action_id: action_id.clone(),
            attempt: runtime.attempts,
        };
        runtime.state = RuntimeState::Executing(ticket.clone());
        self.active_subgoal = Some(subgoal_id.to_owned());
        self.used_action_ids.insert(action_id.clone());
        self.decisions.push(Decision {
            mission_id: self.plan.mission_id.clone(),
            subgoal_id: subgoal_id.to_owned(),
            kind: DecisionKind::ActionIssued {
                action_id,
                attempt: ticket.attempt,
            },
        });
        Ok(ticket)
    }

    /// An acknowledgement means the command transport accepted the command.
    /// It never proves that the task predicate is satisfied.
    pub fn report_execution(
        &mut self,
        ticket: &ActionTicket,
        disposition: ExecutionDisposition,
    ) -> Result<(), SupervisorError> {
        self.ensure_running()?;
        self.ensure_ticket_active(ticket)?;
        let runtime = self
            .states
            .get_mut(&ticket.subgoal_id)
            .ok_or(SupervisorError::UnknownSubgoal)?;
        if runtime.state != RuntimeState::Executing(ticket.clone()) {
            return Err(SupervisorError::InvalidStateTransition);
        }
        let kind = match disposition {
            ExecutionDisposition::Acknowledged => {
                runtime.state = RuntimeState::AwaitingEvidence(ticket.clone());
                DecisionKind::ExecutionAcknowledged {
                    action_id: ticket.action_id.clone(),
                }
            }
            ExecutionDisposition::KnownNoEffect => {
                runtime.state = RuntimeState::RecoveryRequired(ticket.clone());
                DecisionKind::ExecutionNoEffect {
                    action_id: ticket.action_id.clone(),
                }
            }
            ExecutionDisposition::OutcomeUnknown => {
                runtime.state = RuntimeState::ReconciliationRequired(ticket.clone());
                DecisionKind::ExecutionOutcomeUnknown {
                    action_id: ticket.action_id.clone(),
                }
            }
        };
        self.decisions.push(Decision {
            mission_id: self.plan.mission_id.clone(),
            subgoal_id: ticket.subgoal_id.clone(),
            kind,
        });
        Ok(())
    }

    pub fn observe_goal(
        &mut self,
        evidence: GoalEvidence,
    ) -> Result<ReceiptDisposition, SupervisorError> {
        self.ensure_running()?;
        let subgoal_id = evidence.subgoal_id.clone();
        evidence.envelope.validate()?;
        if let Some(disposition) = self.check_existing_event(&evidence.envelope)? {
            return Ok(disposition);
        }
        self.ensure_active_subgoal(&subgoal_id)?;
        let active_ticket = self.current_ticket(&subgoal_id)?;
        if evidence.action_id != active_ticket.action_id {
            return Err(SupervisorError::ActionIdMismatch);
        }
        if evidence.attempt != active_ticket.attempt {
            return Err(SupervisorError::ActionAttemptMismatch);
        }
        let runtime = self
            .states
            .get(&subgoal_id)
            .ok_or(SupervisorError::UnknownSubgoal)?;
        if evidence.predicate_id != runtime.spec.predicate_id {
            return Err(SupervisorError::InvalidStateTransition);
        }
        if !matches!(
            &runtime.state,
            RuntimeState::AwaitingEvidence(_) | RuntimeState::ReconciliationRequired(_)
        ) {
            return Err(SupervisorError::InvalidStateTransition);
        }
        match self.register_event(&evidence.envelope)? {
            ReceiptDisposition::Duplicate => return Ok(ReceiptDisposition::Duplicate),
            ReceiptDisposition::Applied => {}
        }

        if evidence.envelope.confidence_basis_points < self.plan.min_confidence_basis_points {
            let ticket = self.current_ticket(&subgoal_id)?;
            self.states.get_mut(&subgoal_id).expect("present").state =
                RuntimeState::ReconciliationRequired(ticket);
            self.decisions.push(Decision {
                mission_id: self.plan.mission_id.clone(),
                subgoal_id,
                kind: DecisionKind::EvidenceAmbiguous {
                    event_id: evidence.envelope.event_id,
                },
            });
            return Ok(ReceiptDisposition::Applied);
        }

        let ticket = self.current_ticket(&subgoal_id)?;
        let kind = match evidence.verdict {
            GoalVerdict::Satisfied => {
                self.states.get_mut(&subgoal_id).expect("present").state = RuntimeState::Complete {
                    event_id: evidence.envelope.event_id.clone(),
                    evidence_digest: evidence.envelope.content_digest,
                };
                self.active_subgoal = None;
                DecisionKind::GoalCompleted {
                    event_id: evidence.envelope.event_id,
                    evidence_digest: evidence.envelope.content_digest,
                }
            }
            GoalVerdict::NotSatisfied => {
                self.states.get_mut(&subgoal_id).expect("present").state =
                    RuntimeState::RecoveryRequired(ticket);
                DecisionKind::GoalNotSatisfied {
                    event_id: evidence.envelope.event_id,
                }
            }
            GoalVerdict::Ambiguous => {
                self.states.get_mut(&subgoal_id).expect("present").state =
                    RuntimeState::ReconciliationRequired(ticket);
                DecisionKind::EvidenceAmbiguous {
                    event_id: evidence.envelope.event_id,
                }
            }
        };
        self.decisions.push(Decision {
            mission_id: self.plan.mission_id.clone(),
            subgoal_id,
            kind,
        });
        Ok(ReceiptDisposition::Applied)
    }

    pub fn reconcile(
        &mut self,
        evidence: ReconciliationEvidence,
    ) -> Result<ReceiptDisposition, SupervisorError> {
        self.ensure_running()?;
        let subgoal_id = evidence.subgoal_id.clone();
        evidence.envelope.validate()?;
        if let Some(disposition) = self.check_existing_event(&evidence.envelope)? {
            return Ok(disposition);
        }
        self.ensure_active_subgoal(&subgoal_id)?;
        let active_ticket = self.current_ticket(&subgoal_id)?;
        if evidence.action_id != active_ticket.action_id {
            return Err(SupervisorError::ActionIdMismatch);
        }
        if evidence.attempt != active_ticket.attempt {
            return Err(SupervisorError::ActionAttemptMismatch);
        }
        if !matches!(
            &self
                .states
                .get(&subgoal_id)
                .ok_or(SupervisorError::UnknownSubgoal)?
                .state,
            RuntimeState::ReconciliationRequired(_)
        ) {
            return Err(SupervisorError::InvalidStateTransition);
        }
        match self.register_event(&evidence.envelope)? {
            ReceiptDisposition::Duplicate => return Ok(ReceiptDisposition::Duplicate),
            ReceiptDisposition::Applied => {}
        }
        if evidence.envelope.confidence_basis_points < self.plan.min_confidence_basis_points {
            self.decisions.push(Decision {
                mission_id: self.plan.mission_id.clone(),
                subgoal_id,
                kind: DecisionKind::ReconciliationUnknown {
                    event_id: evidence.envelope.event_id,
                },
            });
            return Ok(ReceiptDisposition::Applied);
        }

        let ticket = self.current_ticket(&subgoal_id)?;
        let kind = match evidence.verdict {
            ReconciliationVerdict::GoalSatisfied => {
                self.states.get_mut(&subgoal_id).expect("present").state = RuntimeState::Complete {
                    event_id: evidence.envelope.event_id.clone(),
                    evidence_digest: evidence.envelope.content_digest,
                };
                self.active_subgoal = None;
                DecisionKind::ReconciliationCompleted {
                    event_id: evidence.envelope.event_id,
                }
            }
            ReconciliationVerdict::NoEffect => {
                self.states.get_mut(&subgoal_id).expect("present").state =
                    RuntimeState::RecoveryRequired(ticket);
                DecisionKind::ReconciliationNoEffect {
                    event_id: evidence.envelope.event_id,
                }
            }
            ReconciliationVerdict::PartialEffect => {
                self.states.get_mut(&subgoal_id).expect("present").state =
                    RuntimeState::RecoveryRequired(ticket);
                DecisionKind::ReconciliationPartialEffect {
                    event_id: evidence.envelope.event_id,
                }
            }
            ReconciliationVerdict::Unknown => DecisionKind::ReconciliationUnknown {
                event_id: evidence.envelope.event_id,
            },
        };
        self.decisions.push(Decision {
            mission_id: self.plan.mission_id.clone(),
            subgoal_id,
            kind,
        });
        Ok(ReceiptDisposition::Applied)
    }

    pub fn authorize_recovery(
        &mut self,
        permit: RecoveryPermit,
    ) -> Result<ReceiptDisposition, SupervisorError> {
        self.ensure_running()?;
        let subgoal_id = permit.subgoal_id.clone();
        permit.envelope.validate()?;
        if let Some(disposition) = self.check_existing_event(&permit.envelope)? {
            return Ok(disposition);
        }
        self.ensure_active_subgoal(&subgoal_id)?;
        let runtime = self
            .states
            .get(&subgoal_id)
            .ok_or(SupervisorError::UnknownSubgoal)?;
        if !matches!(&runtime.state, RuntimeState::RecoveryRequired(_)) {
            return Err(SupervisorError::InvalidStateTransition);
        }
        let active_ticket = self.current_ticket(&subgoal_id)?;
        if permit.action_id != active_ticket.action_id {
            return Err(SupervisorError::ActionIdMismatch);
        }
        if permit.attempt != active_ticket.attempt {
            return Err(SupervisorError::ActionAttemptMismatch);
        }
        if !runtime
            .spec
            .approved_recovery_plans
            .contains(&permit.recovery_plan_digest)
        {
            return Err(SupervisorError::RecoveryPlanNotApproved);
        }
        match self.register_event(&permit.envelope)? {
            ReceiptDisposition::Duplicate => return Ok(ReceiptDisposition::Duplicate),
            ReceiptDisposition::Applied => {}
        }
        if !permit.safe_to_mutate
            || permit.envelope.confidence_basis_points < self.plan.min_confidence_basis_points
        {
            return Err(SupervisorError::UnsafeRecoveryPermit);
        }

        let runtime = self.states.get_mut(&subgoal_id).expect("present");
        runtime.state = RuntimeState::Pending;
        let next_attempt = runtime.attempts.saturating_add(1);
        self.decisions.push(Decision {
            mission_id: self.plan.mission_id.clone(),
            subgoal_id,
            kind: DecisionKind::RecoveryAuthorized {
                event_id: permit.envelope.event_id,
                plan_digest: permit.recovery_plan_digest,
                next_attempt,
            },
        });
        Ok(ReceiptDisposition::Applied)
    }

    pub fn phase(&self, subgoal_id: &str) -> Result<SubgoalPhase, SupervisorError> {
        let state = &self
            .states
            .get(subgoal_id)
            .ok_or(SupervisorError::UnknownSubgoal)?
            .state;
        Ok(match state {
            RuntimeState::Pending => SubgoalPhase::Pending,
            RuntimeState::Executing(_) => SubgoalPhase::Executing,
            RuntimeState::AwaitingEvidence(_) => SubgoalPhase::AwaitingEvidence,
            RuntimeState::ReconciliationRequired(_) => SubgoalPhase::ReconciliationRequired,
            RuntimeState::RecoveryRequired(_) => SubgoalPhase::RecoveryRequired,
            RuntimeState::Complete { .. } => SubgoalPhase::Complete,
        })
    }

    pub fn active_subgoal(&self) -> Option<&str> {
        self.active_subgoal.as_deref()
    }

    pub fn is_complete(&self) -> bool {
        self.states
            .values()
            .all(|runtime| matches!(&runtime.state, RuntimeState::Complete { .. }))
    }

    pub fn is_halted(&self) -> bool {
        self.halted
    }

    pub fn decisions(&self) -> &[Decision] {
        &self.decisions
    }

    pub fn mission_id(&self) -> &str {
        self.plan.mission_id()
    }

    fn ensure_running(&self) -> Result<(), SupervisorError> {
        if self.halted {
            Err(SupervisorError::Halted)
        } else {
            Ok(())
        }
    }

    fn ensure_active_subgoal(&self, subgoal_id: &str) -> Result<(), SupervisorError> {
        if self.active_subgoal.as_deref() == Some(subgoal_id) {
            Ok(())
        } else {
            Err(SupervisorError::WrongActiveSubgoal)
        }
    }

    fn ensure_ticket_active(&self, ticket: &ActionTicket) -> Result<(), SupervisorError> {
        if ticket.mission_id != self.plan.mission_id
            || self.active_subgoal.as_deref() != Some(ticket.subgoal_id.as_str())
        {
            return Err(SupervisorError::WrongActiveSubgoal);
        }
        Ok(())
    }

    fn current_ticket(&self, subgoal_id: &str) -> Result<ActionTicket, SupervisorError> {
        let runtime = self
            .states
            .get(subgoal_id)
            .ok_or(SupervisorError::UnknownSubgoal)?;
        match &runtime.state {
            RuntimeState::Executing(ticket)
            | RuntimeState::AwaitingEvidence(ticket)
            | RuntimeState::ReconciliationRequired(ticket)
            | RuntimeState::RecoveryRequired(ticket) => Ok(ticket.clone()),
            RuntimeState::Pending | RuntimeState::Complete { .. } => {
                Err(SupervisorError::InvalidStateTransition)
            }
        }
    }

    fn check_existing_event(
        &mut self,
        envelope: &ReceiptEnvelope,
    ) -> Result<Option<ReceiptDisposition>, SupervisorError> {
        if let Some(previous_digest) = self.seen_events.get(&envelope.event_id) {
            if previous_digest == &envelope.content_digest {
                return Ok(Some(ReceiptDisposition::Duplicate));
            }
            self.halted = true;
            self.decisions.push(Decision {
                mission_id: self.plan.mission_id.clone(),
                subgoal_id: self.active_subgoal.clone().unwrap_or_default(),
                kind: DecisionKind::EventIdentityConflict {
                    event_id: envelope.event_id.clone(),
                },
            });
            return Err(SupervisorError::EventIdConflict);
        }
        Ok(None)
    }

    fn register_event(
        &mut self,
        envelope: &ReceiptEnvelope,
    ) -> Result<ReceiptDisposition, SupervisorError> {
        if let Some(previous_digest) = self.seen_events.get(&envelope.event_id) {
            if previous_digest == &envelope.content_digest {
                return Ok(ReceiptDisposition::Duplicate);
            }
            self.halted = true;
            self.decisions.push(Decision {
                mission_id: self.plan.mission_id.clone(),
                subgoal_id: self.active_subgoal.clone().unwrap_or_default(),
                kind: DecisionKind::EventIdentityConflict {
                    event_id: envelope.event_id.clone(),
                },
            });
            return Err(SupervisorError::EventIdConflict);
        }

        if let Some(cursor) = self.source_cursors.get(&envelope.source_id) {
            if envelope.source_sequence <= cursor.sequence {
                return Err(SupervisorError::StaleSourceSequence);
            }
            let time = &envelope.time_qualification;
            if time.source_clock_id != cursor.source_clock_id
                || time.source_boot_epoch_id != cursor.source_boot_epoch_id
                || time.comparison_basis_id != cursor.comparison_basis_id
                || time.comparison_epoch_id != cursor.comparison_epoch_id
                || time.correlation_commitment != cursor.correlation_commitment
            {
                return Err(SupervisorError::SourceTimeBasisMismatch);
            }
            if envelope.observed_at_ms < cursor.observed_at_ms {
                return Err(SupervisorError::SourceTimestampRegression);
            }
        }

        self.seen_events
            .insert(envelope.event_id.clone(), envelope.content_digest);
        self.source_cursors.insert(
            envelope.source_id.clone(),
            SourceCursor {
                sequence: envelope.source_sequence,
                observed_at_ms: envelope.observed_at_ms,
                source_clock_id: envelope.time_qualification.source_clock_id.clone(),
                source_boot_epoch_id: envelope.time_qualification.source_boot_epoch_id.clone(),
                comparison_basis_id: envelope.time_qualification.comparison_basis_id.clone(),
                comparison_epoch_id: envelope.time_qualification.comparison_epoch_id.clone(),
                correlation_commitment: envelope.time_qualification.correlation_commitment,
            },
        );
        Ok(ReceiptDisposition::Applied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> Digest {
        Digest([byte; 32])
    }

    fn envelope(
        event_id: &str,
        source_id: &str,
        sequence: u64,
        byte: u8,
        confidence: u16,
        role: ObserverRole,
    ) -> ReceiptEnvelope {
        ReceiptEnvelope {
            event_id: event_id.to_owned(),
            content_digest: digest(byte),
            source_id: source_id.to_owned(),
            source_sequence: sequence,
            observed_at_ms: sequence * 10,
            time_qualification: ReceiptTimeQualification {
                source_clock_id: format!("source-clock-{source_id}"),
                source_boot_epoch_id: format!("source-boot-{source_id}"),
                comparison_basis_id: "tai".into(),
                comparison_epoch_id: "tai-epoch-1970".into(),
                source_time_commitment: digest(byte.wrapping_add(1)),
                correlation_commitment: digest(byte.wrapping_add(2)),
                normalized_time_commitment: digest(byte.wrapping_add(3)),
                uncertainty_bound_nanoseconds: 6_000_000,
            },
            observer_role: role,
            confidence_basis_points: confidence,
        }
    }

    fn plan() -> MissionPlan {
        MissionPlan::new(
            "mission-a",
            9_000,
            vec![
                SubgoalSpec::new("pick", "object-in-gripper", vec![], vec![digest(90)]),
                SubgoalSpec::new(
                    "place",
                    "object-on-target",
                    vec!["pick".to_owned()],
                    vec![digest(91)],
                ),
            ],
        )
        .expect("valid plan")
    }

    fn started_supervisor() -> (MissionSupervisor, ActionTicket) {
        let mut supervisor = MissionSupervisor::new(plan());
        let ticket = supervisor
            .dispatch_action("pick", "pick-action-1")
            .expect("dispatch");
        supervisor
            .report_execution(&ticket, ExecutionDisposition::Acknowledged)
            .expect("acknowledgement");
        (supervisor, ticket)
    }

    fn satisfied_pick(event_id: &str, sequence: u64, byte: u8) -> GoalEvidence {
        GoalEvidence {
            envelope: envelope(
                event_id,
                "wrist-sensor",
                sequence,
                byte,
                9_800,
                ObserverRole::IndependentSensor,
            ),
            subgoal_id: "pick".to_owned(),
            action_id: "pick-action-1".to_owned(),
            attempt: 1,
            predicate_id: "object-in-gripper".to_owned(),
            verdict: GoalVerdict::Satisfied,
        }
    }

    #[test]
    fn plan_requires_dependencies_to_precede_dependents() {
        let invalid = MissionPlan::new(
            "bad",
            9_000,
            vec![
                SubgoalSpec::new("place", "on-target", vec!["pick".to_owned()], vec![]),
                SubgoalSpec::new("pick", "in-gripper", vec![], vec![]),
            ],
        );
        assert_eq!(invalid, Err(SupervisorError::DependencyMustPrecedeSubgoal));
    }

    #[test]
    fn dependent_subgoal_cannot_start_before_verified_predecessor() {
        let mut supervisor = MissionSupervisor::new(plan());
        assert_eq!(
            supervisor.dispatch_action("place", "place-action-1"),
            Err(SupervisorError::DependencyNotComplete)
        );
    }

    #[test]
    fn transport_acknowledgement_does_not_complete_goal() {
        let (supervisor, _) = started_supervisor();
        assert_eq!(supervisor.phase("pick"), Ok(SubgoalPhase::AwaitingEvidence));
        assert!(!supervisor.is_complete());
    }

    #[test]
    fn only_independent_goal_evidence_can_complete_subgoal() {
        let (mut supervisor, _) = started_supervisor();
        let mut evidence = satisfied_pick("event-1", 1, 1);
        evidence.envelope.observer_role = ObserverRole::AgentSelfReport;
        assert_eq!(
            supervisor.observe_goal(evidence),
            Err(SupervisorError::UntrustedObserver)
        );
        assert_eq!(supervisor.phase("pick"), Ok(SubgoalPhase::AwaitingEvidence));
    }

    #[test]
    fn evidence_must_match_subgoal_and_predicate() {
        let (mut supervisor, _) = started_supervisor();
        let mut evidence = satisfied_pick("event-1", 1, 1);
        evidence.predicate_id = "wrong-predicate".to_owned();
        assert_eq!(
            supervisor.observe_goal(evidence),
            Err(SupervisorError::InvalidStateTransition)
        );
        assert_eq!(supervisor.phase("pick"), Ok(SubgoalPhase::AwaitingEvidence));
    }

    #[test]
    fn low_confidence_is_held_for_reconciliation() {
        let (mut supervisor, _) = started_supervisor();
        let mut evidence = satisfied_pick("event-1", 1, 1);
        evidence.envelope.confidence_basis_points = 8_999;
        supervisor.observe_goal(evidence).expect("receipt accepted");
        assert_eq!(
            supervisor.phase("pick"),
            Ok(SubgoalPhase::ReconciliationRequired)
        );
        assert!(
            supervisor
                .dispatch_action("place", "place-action-1")
                .is_err()
        );
    }

    #[test]
    fn identical_duplicate_event_is_idempotent() {
        let (mut supervisor, _) = started_supervisor();
        let evidence = satisfied_pick("event-1", 1, 1);
        assert_eq!(
            supervisor.observe_goal(evidence.clone()),
            Ok(ReceiptDisposition::Applied)
        );
        assert_eq!(
            supervisor.observe_goal(evidence),
            Ok(ReceiptDisposition::Duplicate)
        );
        assert_eq!(supervisor.phase("pick"), Ok(SubgoalPhase::Complete));
        assert_eq!(supervisor.decisions().len(), 3);
    }

    #[test]
    fn source_timestamp_cannot_cross_time_basis_or_correlation_revision() {
        let mut supervisor = MissionSupervisor::new(plan());
        let ticket = supervisor
            .dispatch_action("pick", "pick-action-1")
            .expect("dispatch");
        supervisor
            .report_execution(&ticket, ExecutionDisposition::Acknowledged)
            .expect("ack");
        let first = envelope(
            "basis-first",
            "sensor-a",
            1,
            31,
            9_900,
            ObserverRole::IndependentSensor,
        );
        supervisor
            .observe_goal(GoalEvidence {
                envelope: first,
                subgoal_id: "pick".to_owned(),
                action_id: ticket.action_id.clone(),
                attempt: ticket.attempt,
                predicate_id: "object-in-gripper".to_owned(),
                verdict: GoalVerdict::Ambiguous,
            })
            .expect("first receipt");

        let mut changed_basis = envelope(
            "basis-second",
            "sensor-a",
            2,
            32,
            9_900,
            ObserverRole::IndependentSensor,
        );
        changed_basis.time_qualification.comparison_basis_id = "unix-time@1".into();
        assert_eq!(
            supervisor.observe_goal(GoalEvidence {
                envelope: changed_basis,
                subgoal_id: "pick".to_owned(),
                action_id: ticket.action_id,
                attempt: ticket.attempt,
                predicate_id: "object-in-gripper".to_owned(),
                verdict: GoalVerdict::NotSatisfied,
            }),
            Err(SupervisorError::SourceTimeBasisMismatch)
        );
        assert_eq!(
            supervisor.phase("pick"),
            Ok(SubgoalPhase::ReconciliationRequired)
        );
    }

    #[test]
    fn source_time_mapping_commitment_cannot_change_between_receipts() {
        let mut supervisor = MissionSupervisor::new(plan());
        let ticket = supervisor
            .dispatch_action("pick", "pick-action-1")
            .expect("dispatch");
        supervisor
            .report_execution(&ticket, ExecutionDisposition::Acknowledged)
            .expect("ack");
        let first = envelope(
            "mapping-first",
            "sensor-mapping",
            1,
            41,
            9_900,
            ObserverRole::IndependentSensor,
        );
        supervisor
            .observe_goal(GoalEvidence {
                envelope: first,
                subgoal_id: "pick".to_owned(),
                action_id: ticket.action_id.clone(),
                attempt: ticket.attempt,
                predicate_id: "object-in-gripper".to_owned(),
                verdict: GoalVerdict::Ambiguous,
            })
            .expect("first receipt");

        let mut changed_mapping = envelope(
            "mapping-second",
            "sensor-mapping",
            2,
            42,
            9_900,
            ObserverRole::IndependentSensor,
        );
        changed_mapping.time_qualification.correlation_commitment = digest(99);
        assert_eq!(
            supervisor.observe_goal(GoalEvidence {
                envelope: changed_mapping,
                subgoal_id: "pick".to_owned(),
                action_id: ticket.action_id.clone(),
                attempt: ticket.attempt,
                predicate_id: "object-in-gripper".to_owned(),
                verdict: GoalVerdict::Ambiguous,
            }),
            Err(SupervisorError::SourceTimeBasisMismatch)
        );

        let mut changed_source_clock = envelope(
            "source-clock-second",
            "sensor-mapping",
            2,
            43,
            9_900,
            ObserverRole::IndependentSensor,
        );
        changed_source_clock.time_qualification.source_clock_id = "new-source-clock".into();
        assert_eq!(
            supervisor.observe_goal(GoalEvidence {
                envelope: changed_source_clock,
                subgoal_id: "pick".to_owned(),
                action_id: ticket.action_id.clone(),
                attempt: ticket.attempt,
                predicate_id: "object-in-gripper".to_owned(),
                verdict: GoalVerdict::Ambiguous,
            }),
            Err(SupervisorError::SourceTimeBasisMismatch)
        );
        assert_eq!(
            supervisor.phase("pick"),
            Ok(SubgoalPhase::ReconciliationRequired)
        );
    }

    #[test]
    fn event_id_reuse_with_different_digest_halts_supervisor() {
        let (mut supervisor, _) = started_supervisor();
        supervisor
            .observe_goal(satisfied_pick("event-1", 1, 1))
            .expect("first receipt");
        let conflict = GoalEvidence {
            envelope: envelope(
                "event-1",
                "second-sensor",
                1,
                2,
                9_900,
                ObserverRole::IndependentSensor,
            ),
            subgoal_id: "place".to_owned(),
            action_id: "place-action-1".to_owned(),
            attempt: 1,
            predicate_id: "object-on-target".to_owned(),
            verdict: GoalVerdict::Satisfied,
        };
        assert_eq!(
            supervisor.observe_goal(conflict),
            Err(SupervisorError::EventIdConflict)
        );
        assert!(supervisor.is_halted());
        assert_eq!(
            supervisor.dispatch_action("place", "place-action-1"),
            Err(SupervisorError::Halted)
        );
    }

    #[test]
    fn stale_source_sequence_is_rejected() {
        let (mut supervisor, _) = started_supervisor();
        let mut first = satisfied_pick("event-1", 2, 1);
        first.verdict = GoalVerdict::Ambiguous;
        supervisor.observe_goal(first).expect("first receipt");
        let duplicate_sequence = GoalEvidence {
            envelope: envelope(
                "event-2",
                "wrist-sensor",
                2,
                2,
                9_800,
                ObserverRole::IndependentSensor,
            ),
            subgoal_id: "pick".to_owned(),
            action_id: "pick-action-1".to_owned(),
            attempt: 1,
            predicate_id: "object-in-gripper".to_owned(),
            verdict: GoalVerdict::Satisfied,
        };
        assert_eq!(
            supervisor.observe_goal(duplicate_sequence),
            Err(SupervisorError::StaleSourceSequence)
        );
        assert_eq!(
            supervisor.phase("pick"),
            Ok(SubgoalPhase::ReconciliationRequired)
        );
    }

    #[test]
    fn ambiguous_transport_blocks_until_reconciled() {
        let mut supervisor = MissionSupervisor::new(plan());
        let ticket = supervisor
            .dispatch_action("pick", "pick-action-1")
            .expect("dispatch");
        supervisor
            .report_execution(&ticket, ExecutionDisposition::OutcomeUnknown)
            .expect("unknown transport");
        assert_eq!(
            supervisor.phase("pick"),
            Ok(SubgoalPhase::ReconciliationRequired)
        );
        assert_eq!(
            supervisor.dispatch_action("place", "place-action-1"),
            Err(SupervisorError::ActionAlreadyInFlight)
        );
    }

    #[test]
    fn reconciled_goal_satisfaction_completes_without_second_action() {
        let mut supervisor = MissionSupervisor::new(plan());
        let ticket = supervisor
            .dispatch_action("pick", "pick-action-1")
            .expect("dispatch");
        supervisor
            .report_execution(&ticket, ExecutionDisposition::OutcomeUnknown)
            .expect("unknown transport");
        supervisor
            .reconcile(ReconciliationEvidence {
                envelope: envelope(
                    "reconcile-1",
                    "scene-evaluator",
                    1,
                    3,
                    9_900,
                    ObserverRole::IndependentEvaluator,
                ),
                subgoal_id: "pick".to_owned(),
                action_id: "pick-action-1".to_owned(),
                attempt: 1,
                verdict: ReconciliationVerdict::GoalSatisfied,
            })
            .expect("reconcile");
        assert_eq!(supervisor.phase("pick"), Ok(SubgoalPhase::Complete));
        assert!(supervisor.active_subgoal().is_none());
    }

    #[test]
    fn no_effect_reconciliation_requires_explicit_recovery_permit() {
        let mut supervisor = MissionSupervisor::new(plan());
        let ticket = supervisor
            .dispatch_action("pick", "pick-action-1")
            .expect("dispatch");
        supervisor
            .report_execution(&ticket, ExecutionDisposition::OutcomeUnknown)
            .expect("unknown transport");
        supervisor
            .reconcile(ReconciliationEvidence {
                envelope: envelope(
                    "reconcile-1",
                    "scene-evaluator",
                    1,
                    3,
                    9_900,
                    ObserverRole::IndependentEvaluator,
                ),
                subgoal_id: "pick".to_owned(),
                action_id: "pick-action-1".to_owned(),
                attempt: 1,
                verdict: ReconciliationVerdict::NoEffect,
            })
            .expect("reconcile no effect");
        assert_eq!(supervisor.phase("pick"), Ok(SubgoalPhase::RecoveryRequired));
        assert_eq!(
            supervisor.dispatch_action("pick", "pick-action-2"),
            Err(SupervisorError::InvalidStateTransition)
        );
    }

    #[test]
    fn unapproved_or_unsafe_recovery_permit_is_rejected() {
        let mut supervisor = MissionSupervisor::new(plan());
        let ticket = supervisor
            .dispatch_action("pick", "pick-action-1")
            .expect("dispatch");
        supervisor
            .report_execution(&ticket, ExecutionDisposition::KnownNoEffect)
            .expect("no effect");
        let unapproved = RecoveryPermit {
            envelope: envelope(
                "recovery-1",
                "safety-monitor",
                1,
                4,
                9_900,
                ObserverRole::IndependentSensor,
            ),
            subgoal_id: "pick".to_owned(),
            action_id: "pick-action-1".to_owned(),
            attempt: 1,
            recovery_plan_digest: digest(77),
            safe_to_mutate: true,
        };
        assert_eq!(
            supervisor.authorize_recovery(unapproved),
            Err(SupervisorError::RecoveryPlanNotApproved)
        );
        let unsafe_permit = RecoveryPermit {
            envelope: envelope(
                "recovery-2",
                "safety-monitor",
                2,
                5,
                9_900,
                ObserverRole::IndependentSensor,
            ),
            subgoal_id: "pick".to_owned(),
            action_id: "pick-action-1".to_owned(),
            attempt: 1,
            recovery_plan_digest: digest(90),
            safe_to_mutate: false,
        };
        assert_eq!(
            supervisor.authorize_recovery(unsafe_permit),
            Err(SupervisorError::UnsafeRecoveryPermit)
        );
        assert_eq!(supervisor.phase("pick"), Ok(SubgoalPhase::RecoveryRequired));
    }

    #[test]
    fn approved_recovery_requires_new_action_identity() {
        let mut supervisor = MissionSupervisor::new(plan());
        let first = supervisor
            .dispatch_action("pick", "pick-action-1")
            .expect("dispatch");
        supervisor
            .report_execution(&first, ExecutionDisposition::KnownNoEffect)
            .expect("known no effect");
        supervisor
            .authorize_recovery(RecoveryPermit {
                envelope: envelope(
                    "recovery-1",
                    "safety-monitor",
                    1,
                    6,
                    9_900,
                    ObserverRole::IndependentSensor,
                ),
                subgoal_id: "pick".to_owned(),
                action_id: "pick-action-1".to_owned(),
                attempt: 1,
                recovery_plan_digest: digest(90),
                safe_to_mutate: true,
            })
            .expect("safe recovery permit");
        let second = supervisor
            .dispatch_action("pick", "pick-action-2")
            .expect("new dispatch");
        assert_eq!(second.attempt, 2);
        assert_ne!(first.action_id, second.action_id);
    }

    #[test]
    fn delayed_goal_evidence_from_prior_action_attempt_is_rejected() {
        let mut supervisor = MissionSupervisor::new(plan());
        let first = supervisor
            .dispatch_action("pick", "pick-action-1")
            .expect("first dispatch");
        supervisor
            .report_execution(&first, ExecutionDisposition::KnownNoEffect)
            .expect("first attempt had no effect");
        supervisor
            .authorize_recovery(RecoveryPermit {
                envelope: envelope(
                    "recovery-1",
                    "safety-monitor",
                    1,
                    10,
                    9_900,
                    ObserverRole::IndependentSensor,
                ),
                subgoal_id: "pick".to_owned(),
                action_id: first.action_id.clone(),
                attempt: first.attempt,
                recovery_plan_digest: digest(90),
                safe_to_mutate: true,
            })
            .expect("first attempt recovery");
        let second = supervisor
            .dispatch_action("pick", "pick-action-2")
            .expect("second dispatch");
        supervisor
            .report_execution(&second, ExecutionDisposition::Acknowledged)
            .expect("second acknowledgement");

        let stale_action = GoalEvidence {
            envelope: envelope(
                "late-old-action",
                "late-observer",
                1,
                11,
                9_900,
                ObserverRole::IndependentSensor,
            ),
            subgoal_id: "pick".to_owned(),
            action_id: first.action_id.clone(),
            attempt: first.attempt,
            predicate_id: "object-in-gripper".to_owned(),
            verdict: GoalVerdict::Satisfied,
        };
        assert_eq!(
            supervisor.observe_goal(stale_action),
            Err(SupervisorError::ActionIdMismatch)
        );

        let stale_attempt = GoalEvidence {
            envelope: envelope(
                "late-old-attempt",
                "late-observer",
                2,
                12,
                9_900,
                ObserverRole::IndependentSensor,
            ),
            subgoal_id: "pick".to_owned(),
            action_id: second.action_id.clone(),
            attempt: first.attempt,
            predicate_id: "object-in-gripper".to_owned(),
            verdict: GoalVerdict::Satisfied,
        };
        assert_eq!(
            supervisor.observe_goal(stale_attempt),
            Err(SupervisorError::ActionAttemptMismatch)
        );
        assert_eq!(supervisor.phase("pick"), Ok(SubgoalPhase::AwaitingEvidence));
        assert_eq!(supervisor.active_subgoal(), Some("pick"));
    }

    #[test]
    fn delayed_reconciliation_from_prior_attempt_is_rejected() {
        let mut supervisor = MissionSupervisor::new(plan());
        let first = supervisor
            .dispatch_action("pick", "pick-action-1")
            .expect("first dispatch");
        supervisor
            .report_execution(&first, ExecutionDisposition::OutcomeUnknown)
            .expect("unknown first outcome");
        supervisor
            .reconcile(ReconciliationEvidence {
                envelope: envelope(
                    "first-reconcile",
                    "scene-evaluator",
                    1,
                    13,
                    9_900,
                    ObserverRole::IndependentEvaluator,
                ),
                subgoal_id: "pick".to_owned(),
                action_id: first.action_id.clone(),
                attempt: first.attempt,
                verdict: ReconciliationVerdict::NoEffect,
            })
            .expect("first reconciliation");
        supervisor
            .authorize_recovery(RecoveryPermit {
                envelope: envelope(
                    "first-recovery",
                    "safety-monitor",
                    1,
                    14,
                    9_900,
                    ObserverRole::IndependentSensor,
                ),
                subgoal_id: "pick".to_owned(),
                action_id: first.action_id.clone(),
                attempt: first.attempt,
                recovery_plan_digest: digest(90),
                safe_to_mutate: true,
            })
            .expect("recover first attempt");
        let second = supervisor
            .dispatch_action("pick", "pick-action-2")
            .expect("second dispatch");
        supervisor
            .report_execution(&second, ExecutionDisposition::OutcomeUnknown)
            .expect("unknown second outcome");

        let stale = ReconciliationEvidence {
            envelope: envelope(
                "late-reconcile-old",
                "late-evaluator",
                1,
                15,
                9_900,
                ObserverRole::IndependentEvaluator,
            ),
            subgoal_id: "pick".to_owned(),
            action_id: first.action_id,
            attempt: 1,
            verdict: ReconciliationVerdict::GoalSatisfied,
        };
        assert_eq!(
            supervisor.reconcile(stale),
            Err(SupervisorError::ActionIdMismatch)
        );
        assert_eq!(
            supervisor.phase("pick"),
            Ok(SubgoalPhase::ReconciliationRequired)
        );
    }

    #[test]
    fn recovery_permit_must_match_exact_failed_action_attempt() {
        let mut supervisor = MissionSupervisor::new(plan());
        let first = supervisor
            .dispatch_action("pick", "pick-action-1")
            .expect("first dispatch");
        supervisor
            .report_execution(&first, ExecutionDisposition::KnownNoEffect)
            .expect("first no effect");
        supervisor
            .authorize_recovery(RecoveryPermit {
                envelope: envelope(
                    "recovery-1",
                    "safety-monitor",
                    1,
                    16,
                    9_900,
                    ObserverRole::IndependentSensor,
                ),
                subgoal_id: "pick".to_owned(),
                action_id: first.action_id.clone(),
                attempt: first.attempt,
                recovery_plan_digest: digest(90),
                safe_to_mutate: true,
            })
            .expect("recover first");
        let second = supervisor
            .dispatch_action("pick", "pick-action-2")
            .expect("second dispatch");
        supervisor
            .report_execution(&second, ExecutionDisposition::KnownNoEffect)
            .expect("second no effect");

        assert_eq!(
            supervisor.authorize_recovery(RecoveryPermit {
                envelope: envelope(
                    "stale-recovery-permit",
                    "safety-monitor",
                    2,
                    17,
                    9_900,
                    ObserverRole::IndependentSensor,
                ),
                subgoal_id: "pick".to_owned(),
                action_id: first.action_id,
                attempt: 1,
                recovery_plan_digest: digest(90),
                safe_to_mutate: true,
            }),
            Err(SupervisorError::ActionIdMismatch)
        );
        assert_eq!(supervisor.phase("pick"), Ok(SubgoalPhase::RecoveryRequired));
    }

    #[test]
    fn not_satisfied_observation_never_retries_automatically() {
        let (mut supervisor, _) = started_supervisor();
        supervisor
            .observe_goal(GoalEvidence {
                envelope: envelope(
                    "event-1",
                    "wrist-sensor",
                    1,
                    1,
                    9_800,
                    ObserverRole::IndependentSensor,
                ),
                subgoal_id: "pick".to_owned(),
                action_id: "pick-action-1".to_owned(),
                attempt: 1,
                predicate_id: "object-in-gripper".to_owned(),
                verdict: GoalVerdict::NotSatisfied,
            })
            .expect("negative observation");
        assert_eq!(supervisor.phase("pick"), Ok(SubgoalPhase::RecoveryRequired));
        assert!(supervisor.dispatch_action("pick", "pick-action-2").is_err());
    }

    #[test]
    fn mission_completes_only_after_every_predicate_is_observed() {
        let (mut supervisor, _) = started_supervisor();
        supervisor
            .observe_goal(satisfied_pick("event-1", 1, 1))
            .expect("pick evidence");
        assert_eq!(
            supervisor.dispatch_action("place", "place-action-1"),
            Ok(ActionTicket {
                mission_id: "mission-a".to_owned(),
                subgoal_id: "place".to_owned(),
                action_id: "place-action-1".to_owned(),
                attempt: 1,
            })
        );
        let ticket = ActionTicket {
            mission_id: "mission-a".to_owned(),
            subgoal_id: "place".to_owned(),
            action_id: "place-action-1".to_owned(),
            attempt: 1,
        };
        supervisor
            .report_execution(&ticket, ExecutionDisposition::Acknowledged)
            .expect("acknowledge place");
        assert!(!supervisor.is_complete());
        supervisor
            .observe_goal(GoalEvidence {
                envelope: envelope(
                    "event-2",
                    "overhead-camera",
                    1,
                    2,
                    9_700,
                    ObserverRole::IndependentSensor,
                ),
                subgoal_id: "place".to_owned(),
                action_id: "place-action-1".to_owned(),
                attempt: 1,
                predicate_id: "object-on-target".to_owned(),
                verdict: GoalVerdict::Satisfied,
            })
            .expect("place evidence");
        assert!(supervisor.is_complete());
        assert!(supervisor.active_subgoal().is_none());
    }

    #[test]
    fn deterministic_commands_produce_identical_decision_logs() {
        fn run() -> Vec<Decision> {
            let (mut supervisor, _) = started_supervisor();
            supervisor
                .observe_goal(satisfied_pick("event-1", 1, 1))
                .expect("goal evidence");
            supervisor.decisions().to_vec()
        }
        assert_eq!(run(), run());
    }
}
