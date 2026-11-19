//! Deterministic task-lifecycle reference oracle for construction robot interoperability.
//!
//! This crate checks message identity/order, immutable task-context binding, task-state
//! transitions, and structural binding of external acceptance receipts. It does not
//! authenticate senders, verify signatures, assess physical work, grant motion authority,
//! coordinate live robots, or replace a site safety system. Those remain external contracts.

pub mod adapters;
pub mod resources;

use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextBinding {
    pub site_epoch: String,
    pub design_revision_digest: String,
    pub target_frame_id: String,
    pub robot_config_digest: String,
}

impl ContextBinding {
    fn validate(&self) -> Result<(), ApplyError> {
        require_non_empty("site_epoch", &self.site_epoch)?;
        require_non_empty("design_revision_digest", &self.design_revision_digest)?;
        require_non_empty("target_frame_id", &self.target_frame_id)?;
        require_non_empty("robot_config_digest", &self.robot_config_digest)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskState {
    Submitted,
    Accepted,
    Queued,
    Started,
    Paused,
    Blocked,
    ExecutorCompleted,
    MeasurementPending,
    IndependentlyAccepted,
    IndependentlyRejected,
    Failed,
    Cancelled,
    RevokedOrStale,
    Indeterminate,
}

impl TaskState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::IndependentlyAccepted
                | Self::IndependentlyRejected
                | Self::Failed
                | Self::Cancelled
                | Self::RevokedOrStale
                | Self::Indeterminate
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptanceReceipt {
    /// Opaque reference to a receipt verified by the external authority.
    pub receipt_ref: String,
    pub task_id: String,
    pub binding: ContextBinding,
    /// Must identify the exact executor output submitted for independent measurement.
    pub output_evidence_ref: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransitionKind {
    Accept,
    Queue,
    Start,
    Pause,
    Block { reason: String },
    Resume,
    ExecutorComplete { output_evidence_ref: Option<String> },
    SubmitForMeasurement,
    InspectionAccepted { receipt: AcceptanceReceipt },
    InspectionRejected { receipt: AcceptanceReceipt },
    Fail { reason: String },
    Cancel,
    Revoke { reason: String },
    MarkIndeterminate { reason: String },
}

impl TransitionKind {
    fn name(&self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Queue => "queue",
            Self::Start => "start",
            Self::Pause => "pause",
            Self::Block { .. } => "block",
            Self::Resume => "resume",
            Self::ExecutorComplete { .. } => "executor_complete",
            Self::SubmitForMeasurement => "submit_for_measurement",
            Self::InspectionAccepted { .. } => "inspection_accepted",
            Self::InspectionRejected { .. } => "inspection_rejected",
            Self::Fail { .. } => "fail",
            Self::Cancel => "cancel",
            Self::Revoke { .. } => "revoke",
            Self::MarkIndeterminate { .. } => "mark_indeterminate",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventEnvelope {
    pub message_id: String,
    /// Supplied content digest. This crate does not compute or authenticate it.
    pub payload_digest: String,
    pub actor_id: String,
    /// Changes on actor reboot/session reset.
    pub actor_session_id: String,
    /// Monotonic sequence scoped to (actor, session, task).
    pub task_sequence: u64,
    pub task_id: String,
    pub binding: ContextBinding,
    pub transition: TransitionKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskRecord {
    pub id: String,
    pub binding: ContextBinding,
    pub state: TaskState,
    pub block_reason: Option<String>,
    pub output_evidence_ref: Option<String>,
    pub acceptance_receipt_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionReconciliation {
    pub actor_id: String,
    pub prior_session_id: String,
    pub new_session_id: String,
    pub checkpoint_digest: String,
    pub sequence_baseline: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplyOutcome {
    Applied(TaskState),
    Duplicate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplyError {
    EmptyField(&'static str),
    WrongTask,
    ConflictingMessageId,
    ActorSessionReconciliationRequired,
    SessionReconciliationNotRequired,
    InvalidSequence {
        expected: u64,
        received: u64,
    },
    StaleSequence {
        last: u64,
        received: u64,
    },
    StaleContext,
    InvalidTransition {
        from: TaskState,
        event: &'static str,
    },
    MissingOutputEvidence,
    InvalidAcceptanceReceipt(&'static str),
    TaskAlreadyTerminal,
    SequenceExhausted,
}

impl fmt::Display for ApplyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ApplyError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskMachine {
    task: TaskRecord,
    seen_messages: BTreeMap<(String, String, String), String>,
    active_session_by_actor: BTreeMap<String, String>,
    last_sequence_by_actor_session: BTreeMap<(String, String), u64>,
    session_reconciliations: Vec<SessionReconciliation>,
}

impl TaskMachine {
    pub fn new(id: impl Into<String>, binding: ContextBinding) -> Result<Self, ApplyError> {
        let id = id.into();
        require_non_empty("task_id", &id)?;
        binding.validate()?;

        Ok(Self {
            task: TaskRecord {
                id,
                binding,
                state: TaskState::Submitted,
                block_reason: None,
                output_evidence_ref: None,
                acceptance_receipt_ref: None,
            },
            seen_messages: BTreeMap::new(),
            active_session_by_actor: BTreeMap::new(),
            last_sequence_by_actor_session: BTreeMap::new(),
            session_reconciliations: Vec::new(),
        })
    }

    pub fn task(&self) -> &TaskRecord {
        &self.task
    }

    pub fn session_reconciliations(&self) -> &[SessionReconciliation] {
        &self.session_reconciliations
    }

    /// Admit a new actor session only after an external reconciler has inspected the
    /// actor's recovered state and supplied its checkpoint digest and sequence baseline.
    /// This method checks that a receipt exists; it does not authenticate that receipt.
    pub fn reconcile_actor_session(
        &mut self,
        actor_id: &str,
        new_session_id: &str,
        sequence_baseline: u64,
        checkpoint_digest: &str,
    ) -> Result<(), ApplyError> {
        require_non_empty("actor_id", actor_id)?;
        require_non_empty("new_session_id", new_session_id)?;
        require_non_empty("checkpoint_digest", checkpoint_digest)?;

        let prior_session_id = self
            .active_session_by_actor
            .get(actor_id)
            .cloned()
            .ok_or(ApplyError::SessionReconciliationNotRequired)?;

        if prior_session_id == new_session_id {
            return Err(ApplyError::SessionReconciliationNotRequired);
        }

        self.active_session_by_actor
            .insert(actor_id.to_owned(), new_session_id.to_owned());
        self.last_sequence_by_actor_session.insert(
            (actor_id.to_owned(), new_session_id.to_owned()),
            sequence_baseline,
        );
        self.session_reconciliations.push(SessionReconciliation {
            actor_id: actor_id.to_owned(),
            prior_session_id,
            new_session_id: new_session_id.to_owned(),
            checkpoint_digest: checkpoint_digest.to_owned(),
            sequence_baseline,
        });
        Ok(())
    }

    pub fn apply(&mut self, event: EventEnvelope) -> Result<ApplyOutcome, ApplyError> {
        validate_event(&event)?;

        if event.task_id != self.task.id {
            return Err(ApplyError::WrongTask);
        }

        let message_key = (
            event.actor_id.clone(),
            event.actor_session_id.clone(),
            event.message_id.clone(),
        );
        if let Some(previous_digest) = self.seen_messages.get(&message_key) {
            return if previous_digest == &event.payload_digest {
                Ok(ApplyOutcome::Duplicate)
            } else {
                Err(ApplyError::ConflictingMessageId)
            };
        }

        self.admit_sequence(&event)?;

        // The message ID and sequence are consumed even if the event is rejected for
        // stale context or an invalid state transition. Reusing the same event later
        // cannot cause a delayed state change.
        self.seen_messages
            .insert(message_key, event.payload_digest.clone());

        if !matches!(
            &event.transition,
            TransitionKind::Revoke { .. } | TransitionKind::MarkIndeterminate { .. }
        ) && event.binding != self.task.binding
        {
            return Err(ApplyError::StaleContext);
        }

        let next_state = self.next_state(&event.transition)?;
        self.task.state = next_state.clone();
        Ok(ApplyOutcome::Applied(next_state))
    }

    fn admit_sequence(&mut self, event: &EventEnvelope) -> Result<(), ApplyError> {
        match self.active_session_by_actor.get(&event.actor_id) {
            None => {
                if event.task_sequence != 1 {
                    return Err(ApplyError::InvalidSequence {
                        expected: 1,
                        received: event.task_sequence,
                    });
                }
                self.active_session_by_actor
                    .insert(event.actor_id.clone(), event.actor_session_id.clone());
                self.last_sequence_by_actor_session
                    .insert((event.actor_id.clone(), event.actor_session_id.clone()), 0);
            }
            Some(active_session) if active_session != &event.actor_session_id => {
                return Err(ApplyError::ActorSessionReconciliationRequired);
            }
            Some(_) => {}
        }

        let key = (event.actor_id.clone(), event.actor_session_id.clone());
        let last = self
            .last_sequence_by_actor_session
            .get(&key)
            .copied()
            .unwrap_or(0);

        if event.task_sequence <= last {
            return Err(ApplyError::StaleSequence {
                last,
                received: event.task_sequence,
            });
        }

        let expected = last.checked_add(1).ok_or(ApplyError::SequenceExhausted)?;
        if event.task_sequence != expected {
            return Err(ApplyError::InvalidSequence {
                expected,
                received: event.task_sequence,
            });
        }

        self.last_sequence_by_actor_session
            .insert(key, event.task_sequence);
        Ok(())
    }

    fn next_state(&mut self, transition: &TransitionKind) -> Result<TaskState, ApplyError> {
        let current = self.task.state.clone();

        let next = match (&current, transition) {
            (TaskState::Submitted, TransitionKind::Accept) => TaskState::Accepted,
            (TaskState::Accepted, TransitionKind::Queue) => TaskState::Queued,
            (TaskState::Accepted | TaskState::Queued, TransitionKind::Start) => TaskState::Started,
            (TaskState::Started, TransitionKind::Pause) => TaskState::Paused,
            (TaskState::Started | TaskState::Paused, TransitionKind::Block { reason }) => {
                require_non_empty("block_reason", reason)?;
                self.task.block_reason = Some(reason.clone());
                TaskState::Blocked
            }
            (TaskState::Paused | TaskState::Blocked, TransitionKind::Resume) => {
                self.task.block_reason = None;
                TaskState::Started
            }
            (
                TaskState::Started,
                TransitionKind::ExecutorComplete {
                    output_evidence_ref,
                },
            ) => {
                if let Some(reference) = output_evidence_ref {
                    require_non_empty("output_evidence_ref", reference)?;
                }
                self.task.output_evidence_ref = output_evidence_ref.clone();
                TaskState::ExecutorCompleted
            }
            (TaskState::ExecutorCompleted, TransitionKind::SubmitForMeasurement) => {
                if self
                    .task
                    .output_evidence_ref
                    .as_deref()
                    .is_none_or(str::is_empty)
                {
                    return Err(ApplyError::MissingOutputEvidence);
                }
                TaskState::MeasurementPending
            }
            (TaskState::MeasurementPending, TransitionKind::InspectionAccepted { receipt }) => {
                self.validate_acceptance_receipt(receipt)?;
                self.task.acceptance_receipt_ref = Some(receipt.receipt_ref.clone());
                TaskState::IndependentlyAccepted
            }
            (TaskState::MeasurementPending, TransitionKind::InspectionRejected { receipt }) => {
                self.validate_acceptance_receipt(receipt)?;
                self.task.acceptance_receipt_ref = Some(receipt.receipt_ref.clone());
                TaskState::IndependentlyRejected
            }
            (
                TaskState::Submitted
                | TaskState::Accepted
                | TaskState::Queued
                | TaskState::Started
                | TaskState::Paused
                | TaskState::Blocked
                | TaskState::ExecutorCompleted
                | TaskState::MeasurementPending,
                TransitionKind::Fail { reason },
            ) => {
                require_non_empty("failure_reason", reason)?;
                TaskState::Failed
            }
            (
                TaskState::Submitted
                | TaskState::Accepted
                | TaskState::Queued
                | TaskState::Started
                | TaskState::Paused
                | TaskState::Blocked,
                TransitionKind::Cancel,
            ) => TaskState::Cancelled,
            (
                TaskState::Submitted
                | TaskState::Accepted
                | TaskState::Queued
                | TaskState::Started
                | TaskState::Paused
                | TaskState::Blocked
                | TaskState::ExecutorCompleted
                | TaskState::MeasurementPending,
                TransitionKind::Revoke { reason },
            ) => {
                require_non_empty("revocation_reason", reason)?;
                TaskState::RevokedOrStale
            }
            (
                TaskState::Submitted
                | TaskState::Accepted
                | TaskState::Queued
                | TaskState::Started
                | TaskState::Paused
                | TaskState::Blocked
                | TaskState::ExecutorCompleted
                | TaskState::MeasurementPending,
                TransitionKind::MarkIndeterminate { reason },
            ) => {
                require_non_empty("indeterminate_reason", reason)?;
                TaskState::Indeterminate
            }
            _ if current.is_terminal() => return Err(ApplyError::TaskAlreadyTerminal),
            _ => {
                return Err(ApplyError::InvalidTransition {
                    from: current,
                    event: transition.name(),
                });
            }
        };

        Ok(next)
    }

    fn validate_acceptance_receipt(&self, receipt: &AcceptanceReceipt) -> Result<(), ApplyError> {
        require_non_empty("acceptance_receipt_ref", &receipt.receipt_ref)?;
        require_non_empty("acceptance_task_id", &receipt.task_id)?;
        require_non_empty(
            "acceptance_output_evidence_ref",
            &receipt.output_evidence_ref,
        )?;

        if receipt.task_id != self.task.id {
            return Err(ApplyError::InvalidAcceptanceReceipt("task_id_mismatch"));
        }
        if receipt.binding != self.task.binding {
            return Err(ApplyError::InvalidAcceptanceReceipt(
                "context_binding_mismatch",
            ));
        }
        if Some(receipt.output_evidence_ref.as_str()) != self.task.output_evidence_ref.as_deref() {
            return Err(ApplyError::InvalidAcceptanceReceipt(
                "output_evidence_mismatch",
            ));
        }
        Ok(())
    }
}

fn validate_event(event: &EventEnvelope) -> Result<(), ApplyError> {
    require_non_empty("message_id", &event.message_id)?;
    require_non_empty("payload_digest", &event.payload_digest)?;
    require_non_empty("actor_id", &event.actor_id)?;
    require_non_empty("actor_session_id", &event.actor_session_id)?;
    require_non_empty("task_id", &event.task_id)?;
    event.binding.validate()
}

fn require_non_empty(name: &'static str, value: &str) -> Result<(), ApplyError> {
    if value.trim().is_empty() {
        return Err(ApplyError::EmptyField(name));
    }
    Ok(())
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

    fn machine() -> TaskMachine {
        TaskMachine::new("task-1", binding()).expect("valid task")
    }

    fn event(sequence: u64, transition: TransitionKind, context: ContextBinding) -> EventEnvelope {
        EventEnvelope {
            message_id: format!("message-{sequence}"),
            payload_digest: format!("payload-{sequence}"),
            actor_id: "robot-1".to_owned(),
            actor_session_id: "boot-1".to_owned(),
            task_sequence: sequence,
            task_id: "task-1".to_owned(),
            binding: context,
            transition,
        }
    }

    fn send(
        machine: &mut TaskMachine,
        sequence: u64,
        transition: TransitionKind,
    ) -> Result<ApplyOutcome, ApplyError> {
        machine.apply(event(sequence, transition, binding()))
    }

    fn receipt(machine: &TaskMachine, ref_id: &str) -> AcceptanceReceipt {
        AcceptanceReceipt {
            receipt_ref: ref_id.to_owned(),
            task_id: machine.task.id.clone(),
            binding: machine.task.binding.clone(),
            output_evidence_ref: machine
                .task
                .output_evidence_ref
                .clone()
                .expect("output evidence exists"),
        }
    }

    #[test]
    fn valid_task_lifecycle_ends_only_with_bound_acceptance_receipt() {
        let mut machine = machine();
        send(&mut machine, 1, TransitionKind::Accept).expect("accept");
        send(&mut machine, 2, TransitionKind::Queue).expect("queue");
        send(&mut machine, 3, TransitionKind::Start).expect("start");
        send(
            &mut machine,
            4,
            TransitionKind::ExecutorComplete {
                output_evidence_ref: Some("output-evidence-1".to_owned()),
            },
        )
        .expect("executor completion");
        assert_eq!(machine.task.state, TaskState::ExecutorCompleted);

        send(&mut machine, 5, TransitionKind::SubmitForMeasurement).expect("submit");
        let accepted_receipt = receipt(&machine, "acceptance-receipt-1");
        send(
            &mut machine,
            6,
            TransitionKind::InspectionAccepted {
                receipt: accepted_receipt,
            },
        )
        .expect("independent acceptance");
        assert_eq!(machine.task.state, TaskState::IndependentlyAccepted);
    }

    #[test]
    fn executor_completion_never_promotes_directly_to_acceptance() {
        let mut machine = machine();
        send(&mut machine, 1, TransitionKind::Accept).expect("accept");
        send(&mut machine, 2, TransitionKind::Start).expect("start");
        send(
            &mut machine,
            3,
            TransitionKind::ExecutorComplete {
                output_evidence_ref: Some("output-1".to_owned()),
            },
        )
        .expect("complete");

        let result = send(
            &mut machine,
            4,
            TransitionKind::InspectionAccepted {
                receipt: AcceptanceReceipt {
                    receipt_ref: "receipt-1".to_owned(),
                    task_id: "task-1".to_owned(),
                    binding: binding(),
                    output_evidence_ref: "output-1".to_owned(),
                },
            },
        );
        assert!(matches!(
            result,
            Err(ApplyError::InvalidTransition {
                event: "inspection_accepted",
                ..
            })
        ));
        assert_eq!(machine.task.state, TaskState::ExecutorCompleted);
    }

    #[test]
    fn missing_output_evidence_cannot_enter_measurement_pending() {
        let mut machine = machine();
        send(&mut machine, 1, TransitionKind::Accept).expect("accept");
        send(&mut machine, 2, TransitionKind::Start).expect("start");
        send(
            &mut machine,
            3,
            TransitionKind::ExecutorComplete {
                output_evidence_ref: None,
            },
        )
        .expect("executor may report completion without evidence");

        assert_eq!(
            send(&mut machine, 4, TransitionKind::SubmitForMeasurement),
            Err(ApplyError::MissingOutputEvidence)
        );
        assert_eq!(machine.task.state, TaskState::ExecutorCompleted);
    }

    #[test]
    fn acceptance_receipt_must_bind_exact_task_context_and_output() {
        let mut machine = machine();
        send(&mut machine, 1, TransitionKind::Accept).expect("accept");
        send(&mut machine, 2, TransitionKind::Start).expect("start");
        send(
            &mut machine,
            3,
            TransitionKind::ExecutorComplete {
                output_evidence_ref: Some("output-1".to_owned()),
            },
        )
        .expect("complete");
        send(&mut machine, 4, TransitionKind::SubmitForMeasurement).expect("measure");

        let mut wrong_receipt = receipt(&machine, "receipt-wrong");
        wrong_receipt.task_id = "different-task".to_owned();
        assert_eq!(
            send(
                &mut machine,
                5,
                TransitionKind::InspectionAccepted {
                    receipt: wrong_receipt,
                },
            ),
            Err(ApplyError::InvalidAcceptanceReceipt("task_id_mismatch"))
        );
        assert_eq!(machine.task.state, TaskState::MeasurementPending);

        let valid_receipt = receipt(&machine, "receipt-valid");
        send(
            &mut machine,
            6,
            TransitionKind::InspectionAccepted {
                receipt: valid_receipt,
            },
        )
        .expect("valid receipt");
        assert_eq!(machine.task.state, TaskState::IndependentlyAccepted);
    }

    #[test]
    fn stale_context_cannot_start_but_new_event_sequence_can_be_used() {
        let mut machine = machine();
        send(&mut machine, 1, TransitionKind::Accept).expect("accept");
        let mut stale = binding();
        stale.site_epoch = "site-epoch-0".to_owned();

        assert_eq!(
            machine.apply(event(2, TransitionKind::Start, stale)),
            Err(ApplyError::StaleContext)
        );
        assert_eq!(machine.task.state, TaskState::Accepted);
        send(&mut machine, 3, TransitionKind::Start).expect("current-context start");
        assert_eq!(machine.task.state, TaskState::Started);
    }

    #[test]
    fn identical_duplicate_is_idempotent_and_conflicting_id_is_rejected() {
        let mut machine = machine();
        let first = event(1, TransitionKind::Accept, binding());
        machine.apply(first.clone()).expect("first event");
        assert_eq!(machine.apply(first), Ok(ApplyOutcome::Duplicate));

        let conflicting = EventEnvelope {
            message_id: "message-1".to_owned(),
            payload_digest: "different-payload".to_owned(),
            actor_id: "robot-1".to_owned(),
            actor_session_id: "boot-1".to_owned(),
            task_sequence: 2,
            task_id: "task-1".to_owned(),
            binding: binding(),
            transition: TransitionKind::Queue,
        };
        assert_eq!(
            machine.apply(conflicting),
            Err(ApplyError::ConflictingMessageId)
        );
        send(&mut machine, 2, TransitionKind::Queue).expect("next valid event");
        assert_eq!(machine.task.state, TaskState::Queued);
    }

    #[test]
    fn message_ids_are_scoped_to_actor_and_session() {
        let mut machine = machine();
        let robot_event = event(1, TransitionKind::Accept, binding());
        machine.apply(robot_event).expect("robot acceptance");

        let site_event = EventEnvelope {
            message_id: "message-1".to_owned(),
            payload_digest: "site-payload-1".to_owned(),
            actor_id: "site-coordinator".to_owned(),
            actor_session_id: "site-session-1".to_owned(),
            task_sequence: 1,
            task_id: "task-1".to_owned(),
            binding: binding(),
            transition: TransitionKind::Queue,
        };
        assert_eq!(
            machine.apply(site_event),
            Ok(ApplyOutcome::Applied(TaskState::Queued))
        );
    }

    #[test]
    fn sequence_gaps_and_replays_are_rejected_without_state_regression() {
        let mut machine = machine();
        assert_eq!(
            send(&mut machine, 2, TransitionKind::Accept),
            Err(ApplyError::InvalidSequence {
                expected: 1,
                received: 2
            })
        );
        send(&mut machine, 1, TransitionKind::Accept).expect("initial event");
        send(&mut machine, 2, TransitionKind::Queue).expect("queue");

        let stale = EventEnvelope {
            message_id: "late-start".to_owned(),
            payload_digest: "late-start-payload".to_owned(),
            actor_id: "robot-1".to_owned(),
            actor_session_id: "boot-1".to_owned(),
            task_sequence: 1,
            task_id: "task-1".to_owned(),
            binding: binding(),
            transition: TransitionKind::Start,
        };
        assert!(matches!(
            machine.apply(stale),
            Err(ApplyError::StaleSequence { .. })
        ));
        assert_eq!(machine.task.state, TaskState::Queued);
    }

    #[test]
    fn actor_reboot_requires_explicit_session_reconciliation() {
        let mut machine = machine();
        let first = event(1, TransitionKind::Accept, binding());
        machine.apply(first).expect("first boot event");

        let rebooted = EventEnvelope {
            message_id: "new-boot-queue".to_owned(),
            payload_digest: "new-boot-payload".to_owned(),
            actor_id: "robot-1".to_owned(),
            actor_session_id: "boot-2".to_owned(),
            task_sequence: 1,
            task_id: "task-1".to_owned(),
            binding: binding(),
            transition: TransitionKind::Queue,
        };
        assert_eq!(
            machine.apply(rebooted.clone()),
            Err(ApplyError::ActorSessionReconciliationRequired)
        );

        machine
            .reconcile_actor_session("robot-1", "boot-2", 0, "checkpoint-digest-1")
            .expect("reconcile reboot");
        machine
            .apply(rebooted)
            .expect("new session after reconcile");
        assert_eq!(machine.task.state, TaskState::Queued);
        assert_eq!(machine.session_reconciliations().len(), 1);
    }

    #[test]
    fn new_site_epoch_can_revoke_but_cannot_resume_old_task() {
        let mut machine = machine();
        send(&mut machine, 1, TransitionKind::Accept).expect("accept");
        let mut new_epoch = binding();
        new_epoch.site_epoch = "site-epoch-2".to_owned();

        machine
            .apply(EventEnvelope {
                message_id: "revoke-old-task".to_owned(),
                payload_digest: "revoke-digest".to_owned(),
                actor_id: "site-coordinator".to_owned(),
                actor_session_id: "session-1".to_owned(),
                task_sequence: 1,
                task_id: "task-1".to_owned(),
                binding: new_epoch,
                transition: TransitionKind::Revoke {
                    reason: "site epoch changed".to_owned(),
                },
            })
            .expect("revoke under new epoch");
        assert_eq!(machine.task.state, TaskState::RevokedOrStale);
    }

    #[test]
    fn inspection_rejection_is_preserved_as_terminal_not_rewritten_as_completion() {
        let mut machine = machine();
        send(&mut machine, 1, TransitionKind::Accept).expect("accept");
        send(&mut machine, 2, TransitionKind::Start).expect("start");
        send(
            &mut machine,
            3,
            TransitionKind::ExecutorComplete {
                output_evidence_ref: Some("output-1".to_owned()),
            },
        )
        .expect("complete");
        send(&mut machine, 4, TransitionKind::SubmitForMeasurement).expect("measure");
        let rejected_receipt = receipt(&machine, "rejection-receipt-1");
        send(
            &mut machine,
            5,
            TransitionKind::InspectionRejected {
                receipt: rejected_receipt,
            },
        )
        .expect("independent rejection");
        assert_eq!(machine.task.state, TaskState::IndependentlyRejected);
        assert_eq!(
            send(&mut machine, 6, TransitionKind::Start),
            Err(ApplyError::TaskAlreadyTerminal)
        );
    }

    #[test]
    fn block_requires_reason_and_resume_requires_current_binding() {
        let mut machine = machine();
        send(&mut machine, 1, TransitionKind::Accept).expect("accept");
        send(&mut machine, 2, TransitionKind::Start).expect("start");
        send(
            &mut machine,
            3,
            TransitionKind::Block {
                reason: "material unavailable".to_owned(),
            },
        )
        .expect("block");

        let mut stale = binding();
        stale.robot_config_digest = "changed-config".to_owned();
        assert_eq!(
            machine.apply(event(4, TransitionKind::Resume, stale)),
            Err(ApplyError::StaleContext)
        );
        send(&mut machine, 5, TransitionKind::Resume).expect("resume");
        assert_eq!(machine.task.state, TaskState::Started);
        assert_eq!(machine.task.block_reason, None);
    }

    #[test]
    fn incomplete_context_is_rejected_at_task_creation() {
        let mut invalid = binding();
        invalid.target_frame_id.clear();
        assert_eq!(
            TaskMachine::new("task-1", invalid),
            Err(ApplyError::EmptyField("target_frame_id"))
        );
    }
}
