//! Accepted-output and whole-floor production accounting.
//!
//! This crate composes the existing construction robot lifecycle and reservation
//! reference core. It does not schedule tasks, authenticate authorities, inspect
//! physical work, grant safety permissions, or authorize machine motion.

use std::collections::BTreeMap;

use construction_robot_interop_core::{
    ContextBinding, ReservationBook, ReservationError, ReservationOutcome, ReservationRequest,
    TaskMachine, TaskState,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FloorBinding {
    pub floor_id: String,
    pub site_epoch: String,
    pub design_revision_digest: String,
    pub target_frame_id: String,
}

impl FloorBinding {
    fn validate(&self) -> Result<(), LedgerError> {
        require_non_empty("floor_id", &self.floor_id)?;
        require_non_empty("site_epoch", &self.site_epoch)?;
        require_non_empty("design_revision_digest", &self.design_revision_digest)?;
        require_non_empty("target_frame_id", &self.target_frame_id)
    }

    fn matches_task(&self, binding: &ContextBinding) -> bool {
        self.site_epoch == binding.site_epoch
            && self.design_revision_digest == binding.design_revision_digest
            && self.target_frame_id == binding.target_frame_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkUnitSpec {
    pub work_unit_id: String,
    pub acceptance_contract_ref: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptSnapshot {
    pub work_unit_id: String,
    pub attempt_number: u32,
    pub task_id: String,
    pub state: TaskState,
    pub output_evidence_ref: Option<String>,
    pub acceptance_receipt_ref: Option<String>,
    pub rework_authorization_ref: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObserveOutcome {
    Recorded,
    Updated,
    Duplicate,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CostCategory {
    DesignAndVdcPreparation,
    SurveyAndControl,
    Labour,
    RobotOperation,
    EquipmentOwnershipOrRental,
    CraneAndHoistLogistics,
    Materials,
    Inspection,
    Rework,
    Energy,
    Maintenance,
    Financing,
    WasteAndCloseout,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostEntry {
    pub entry_id: String,
    pub task_id: Option<String>,
    pub category: CostCategory,
    pub amount_minor_units: u64,
    pub currency_code: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LabourSlice {
    pub slice_id: String,
    pub person_id: String,
    pub role: String,
    /// Integer minutes in one declared project accounting clock.
    pub start_minute: u64,
    pub end_minute: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoneyPerUnit {
    pub numerator_minor_units: u64,
    pub denominator_accepted_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptReport {
    pub work_unit_id: String,
    pub acceptance_contract_ref: String,
    pub attempt_number: u32,
    pub task_id: String,
    pub state: TaskState,
    pub output_evidence_ref: Option<String>,
    pub acceptance_receipt_ref: Option<String>,
    pub rework_authorization_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FloorReport {
    pub floor_id: String,
    pub site_epoch: String,
    pub design_revision_digest: String,
    pub target_frame_id: String,
    pub currency_code: String,
    pub required_units: u64,
    pub attempted_units: u64,
    pub first_pass_inspected_units: u64,
    pub first_pass_accepted_units: u64,
    pub finally_accepted_units: u64,
    pub rework_attempts: u64,
    pub not_finally_accepted_unit_ids: Vec<String>,
    pub attempts: Vec<AttemptReport>,
    pub total_workflow_crew_minutes: u64,
    pub crew_minutes_by_role: BTreeMap<String, u64>,
    pub total_cost_minor_units: u64,
    pub costs_by_category_minor_units: BTreeMap<CostCategory, u64>,
    pub cost_per_accepted_unit: Option<MoneyPerUnit>,
    /// Present only when all required functional units are finally accepted.
    pub accepted_floor_cost_minor_units: Option<u64>,
}

impl FloorReport {
    pub fn floor_accepted(&self) -> bool {
        self.required_units > 0
            && self.finally_accepted_units == self.required_units
            && self.not_finally_accepted_unit_ids.is_empty()
    }

    pub fn first_pass_yield_numerator_denominator(&self) -> (u64, u64) {
        (
            self.first_pass_accepted_units,
            self.first_pass_inspected_units,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LedgerError {
    EmptyField(&'static str),
    InvalidCurrencyCode,
    NoRequiredWorkUnits,
    DuplicateWorkUnit(String),
    UnknownWorkUnit(String),
    TaskContextMismatch,
    TaskIdAlreadyAssigned(String),
    AttemptNumberOverflow,
    AttemptNumberMismatch { expected: u32, received: u32 },
    PriorAttemptNotTerminal,
    PriorAttemptAlreadyAccepted,
    MissingReworkAuthorization,
    UnexpectedReworkAuthorization,
    TerminalAttemptMutation(String),
    InvalidStateProgression { from: TaskState, to: TaskState },
    MissingAcceptanceEvidence,
    DuplicateCostEntry(String),
    CurrencyMismatch { expected: String, received: String },
    UnknownTask(String),
    DuplicateLabourSlice(String),
    InvalidLabourInterval,
    OverlappingPersonTime {
        person_id: String,
        existing_slice_id: String,
        new_slice_id: String,
    },
    StaleResourceEpoch,
    ArithmeticOverflow,
    Reservation(ReservationError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FloorLedger {
    binding: FloorBinding,
    currency_code: String,
    required_units: BTreeMap<String, String>,
    attempts: BTreeMap<String, Vec<AttemptSnapshot>>,
    task_owners: BTreeMap<String, (String, u32)>,
    costs: BTreeMap<String, CostEntry>,
    labour: BTreeMap<String, LabourSlice>,
}

impl FloorLedger {
    pub fn new(
        binding: FloorBinding,
        currency_code: impl Into<String>,
        required_units: Vec<WorkUnitSpec>,
    ) -> Result<Self, LedgerError> {
        binding.validate()?;
        let currency_code = currency_code.into();
        validate_currency(&currency_code)?;
        if required_units.is_empty() {
            return Err(LedgerError::NoRequiredWorkUnits);
        }

        let mut unit_map = BTreeMap::new();
        for unit in required_units {
            require_non_empty("work_unit_id", &unit.work_unit_id)?;
            require_non_empty("acceptance_contract_ref", &unit.acceptance_contract_ref)?;
            if unit_map
                .insert(unit.work_unit_id.clone(), unit.acceptance_contract_ref)
                .is_some()
            {
                return Err(LedgerError::DuplicateWorkUnit(unit.work_unit_id));
            }
        }

        Ok(Self {
            binding,
            currency_code,
            required_units: unit_map,
            attempts: BTreeMap::new(),
            task_owners: BTreeMap::new(),
            costs: BTreeMap::new(),
            labour: BTreeMap::new(),
        })
    }

    pub fn binding(&self) -> &FloorBinding {
        &self.binding
    }

    pub fn acceptance_contract_ref(&self, work_unit_id: &str) -> Option<&str> {
        self.required_units
            .get(work_unit_id)
            .map(String::as_str)
    }

    /// Observe the authoritative state of an existing lifecycle TaskMachine.
    ///
    /// The TaskMachine owns valid state transitions and receipt binding. This ledger
    /// snapshots that state; it cannot create an acceptance transition itself.
    pub fn observe_attempt(
        &mut self,
        work_unit_id: &str,
        machine: &TaskMachine,
        rework_authorization_ref: Option<&str>,
    ) -> Result<ObserveOutcome, LedgerError> {
        if !self.required_units.contains_key(work_unit_id) {
            return Err(LedgerError::UnknownWorkUnit(work_unit_id.to_owned()));
        }

        let task = machine.task();
        if !self.binding.matches_task(&task.binding) {
            return Err(LedgerError::TaskContextMismatch);
        }
        if matches!(task.state, TaskState::IndependentlyAccepted | TaskState::IndependentlyRejected)
            && (task.output_evidence_ref.as_deref().is_none_or(str::is_empty)
                || task
                    .acceptance_receipt_ref
                    .as_deref()
                    .is_none_or(str::is_empty))
        {
            return Err(LedgerError::MissingAcceptanceEvidence);
        }

        let task_id = task.id.clone();
        let snapshot = AttemptSnapshot {
            work_unit_id: work_unit_id.to_owned(),
            attempt_number: 0,
            task_id: task_id.clone(),
            state: task.state.clone(),
            output_evidence_ref: task.output_evidence_ref.clone(),
            acceptance_receipt_ref: task.acceptance_receipt_ref.clone(),
            rework_authorization_ref: rework_authorization_ref.map(str::to_owned),
        };

        if let Some((owner_unit, owner_attempt)) = self.task_owners.get(&task_id) {
            if owner_unit != work_unit_id {
                return Err(LedgerError::TaskIdAlreadyAssigned(task_id));
            }
            let history = self
                .attempts
                .get_mut(work_unit_id)
                .expect("task owner index points to an existing work unit");
            let previous = history
                .last_mut()
                .filter(|item| item.attempt_number == *owner_attempt)
                .ok_or_else(|| LedgerError::TaskIdAlreadyAssigned(task_id.clone()))?;

            if previous.state.is_terminal() {
                if previous.state == snapshot.state
                    && previous.output_evidence_ref == snapshot.output_evidence_ref
                    && previous.acceptance_receipt_ref == snapshot.acceptance_receipt_ref
                {
                    return Ok(ObserveOutcome::Duplicate);
                }
                return Err(LedgerError::TerminalAttemptMutation(task_id));
            }
            if !valid_progression(&previous.state, &snapshot.state) {
                return Err(LedgerError::InvalidStateProgression {
                    from: previous.state.clone(),
                    to: snapshot.state,
                });
            }
            previous.state = snapshot.state;
            previous.output_evidence_ref = snapshot.output_evidence_ref;
            previous.acceptance_receipt_ref = snapshot.acceptance_receipt_ref;
            return Ok(ObserveOutcome::Updated);
        }

        if self.task_owners.contains_key(&task_id) {
            return Err(LedgerError::TaskIdAlreadyAssigned(task_id));
        }

        let history = self.attempts.entry(work_unit_id.to_owned()).or_default();
        let expected_attempt = match history.last() {
            None => {
                if rework_authorization_ref.is_some() {
                    return Err(LedgerError::UnexpectedReworkAuthorization);
                }
                1
            }
            Some(previous) => {
                if previous.state == TaskState::IndependentlyAccepted {
                    return Err(LedgerError::PriorAttemptAlreadyAccepted);
                }
                if !previous.state.is_terminal() {
                    return Err(LedgerError::PriorAttemptNotTerminal);
                }
                require_non_empty(
                    "rework_authorization_ref",
                    rework_authorization_ref.unwrap_or_default(),
                )
                .map_err(|_| LedgerError::MissingReworkAuthorization)?;
                previous
                    .attempt_number
                    .checked_add(1)
                    .ok_or(LedgerError::AttemptNumberOverflow)?
            }
        };

        let attempt_number = expected_attempt;
        let mut snapshot = snapshot;
        snapshot.attempt_number = attempt_number;
        if let Some(received) = self.task_owners.get(&task_id) {
            return Err(LedgerError::TaskIdAlreadyAssigned(format!(
                "{}:{}",
                received.0, received.1
            )));
        }
        history.push(snapshot);
        self.task_owners
            .insert(task_id, (work_unit_id.to_owned(), attempt_number));
        Ok(ObserveOutcome::Recorded)
    }

    pub fn add_cost(&mut self, entry: CostEntry) -> Result<(), LedgerError> {
        require_non_empty("cost_entry_id", &entry.entry_id)?;
        validate_currency(&entry.currency_code)?;
        if entry.currency_code != self.currency_code {
            return Err(LedgerError::CurrencyMismatch {
                expected: self.currency_code.clone(),
                received: entry.currency_code,
            });
        }
        if let Some(task_id) = entry.task_id.as_deref() {
            if !self.task_owners.contains_key(task_id) {
                return Err(LedgerError::UnknownTask(task_id.to_owned()));
            }
        }
        if self.costs.contains_key(&entry.entry_id) {
            return Err(LedgerError::DuplicateCostEntry(entry.entry_id));
        }
        self.costs.insert(entry.entry_id.clone(), entry);
        Ok(())
    }

    /// Add a person's exclusive accounting interval. Intervals are half-open [start,end).
    pub fn add_labour_slice(&mut self, slice: LabourSlice) -> Result<(), LedgerError> {
        require_non_empty("labour_slice_id", &slice.slice_id)?;
        require_non_empty("person_id", &slice.person_id)?;
        require_non_empty("role", &slice.role)?;
        if slice.start_minute >= slice.end_minute {
            return Err(LedgerError::InvalidLabourInterval);
        }
        if self.labour.contains_key(&slice.slice_id) {
            return Err(LedgerError::DuplicateLabourSlice(slice.slice_id));
        }

        for existing in self.labour.values() {
            if existing.person_id == slice.person_id
                && slice.start_minute < existing.end_minute
                && existing.start_minute < slice.end_minute
            {
                return Err(LedgerError::OverlappingPersonTime {
                    person_id: slice.person_id,
                    existing_slice_id: existing.slice_id.clone(),
                    new_slice_id: slice.slice_id,
                });
            }
        }

        self.labour.insert(slice.slice_id.clone(), slice);
        Ok(())
    }

    pub fn report(&self) -> Result<FloorReport, LedgerError> {
        let mut attempted_units = 0_u64;
        let mut first_pass_inspected = 0_u64;
        let mut first_pass_accepted = 0_u64;
        let mut finally_accepted = 0_u64;
        let mut rework_attempts = 0_u64;
        let mut not_finally_accepted = Vec::new();
        let mut attempts = Vec::new();

        for work_unit_id in self.required_units.keys() {
            match self.attempts.get(work_unit_id) {
                Some(history) if !history.is_empty() => {
                    attempted_units = checked_add(attempted_units, 1)?;
                    let first = &history[0];
                    if matches!(
                        first.state,
                        TaskState::IndependentlyAccepted | TaskState::IndependentlyRejected
                    ) {
                        first_pass_inspected = checked_add(first_pass_inspected, 1)?;
                    }
                    if first.state == TaskState::IndependentlyAccepted {
                        first_pass_accepted = checked_add(first_pass_accepted, 1)?;
                    }

                    rework_attempts = checked_add(
                        rework_attempts,
                        u64::try_from(history.len().saturating_sub(1))
                            .map_err(|_| LedgerError::ArithmeticOverflow)?,
                    )?;

                    let final_state = &history
                        .last()
                        .expect("non-empty history has a last attempt")
                        .state;
                    if *final_state == TaskState::IndependentlyAccepted {
                        finally_accepted = checked_add(finally_accepted, 1)?;
                    } else {
                        not_finally_accepted.push(work_unit_id.clone());
                    }
                    for attempt in history {
                        let acceptance_contract_ref = self
                            .required_units
                            .get(&attempt.work_unit_id)
                            .cloned()
                            .ok_or_else(|| LedgerError::UnknownWorkUnit(
                                attempt.work_unit_id.clone(),
                            ))?;
                        attempts.push(AttemptReport {
                            work_unit_id: attempt.work_unit_id.clone(),
                            acceptance_contract_ref,
                            attempt_number: attempt.attempt_number,
                            task_id: attempt.task_id.clone(),
                            state: attempt.state.clone(),
                            output_evidence_ref: attempt.output_evidence_ref.clone(),
                            acceptance_receipt_ref: attempt.acceptance_receipt_ref.clone(),
                            rework_authorization_ref: attempt.rework_authorization_ref.clone(),
                        });
                    }
                }
                _ => not_finally_accepted.push(work_unit_id.clone()),
            }
        }

        let mut total_cost = 0_u64;
        let mut costs_by_category = BTreeMap::new();
        for entry in self.costs.values() {
            total_cost = checked_add(total_cost, entry.amount_minor_units)?;
            let category_total = costs_by_category
                .get(&entry.category)
                .copied()
                .unwrap_or(0_u64);
            costs_by_category.insert(
                entry.category.clone(),
                checked_add(category_total, entry.amount_minor_units)?,
            );
        }

        let mut total_crew_minutes = 0_u64;
        let mut crew_by_role = BTreeMap::new();
        for slice in self.labour.values() {
            let duration = slice
                .end_minute
                .checked_sub(slice.start_minute)
                .ok_or(LedgerError::ArithmeticOverflow)?;
            total_crew_minutes = checked_add(total_crew_minutes, duration)?;
            let role_total = crew_by_role.get(&slice.role).copied().unwrap_or(0_u64);
            crew_by_role.insert(slice.role.clone(), checked_add(role_total, duration)?);
        }

        let required_units = u64::try_from(self.required_units.len())
            .map_err(|_| LedgerError::ArithmeticOverflow)?;
        let cost_per_accepted_unit = (finally_accepted > 0).then_some(MoneyPerUnit {
            numerator_minor_units: total_cost,
            denominator_accepted_units: finally_accepted,
        });
        let accepted_floor_cost = (finally_accepted == required_units)
            .then_some(total_cost);

        Ok(FloorReport {
            floor_id: self.binding.floor_id.clone(),
            site_epoch: self.binding.site_epoch.clone(),
            design_revision_digest: self.binding.design_revision_digest.clone(),
            target_frame_id: self.binding.target_frame_id.clone(),
            currency_code: self.currency_code.clone(),
            required_units,
            attempted_units,
            first_pass_inspected_units: first_pass_inspected,
            first_pass_accepted_units: first_pass_accepted,
            finally_accepted_units: finally_accepted,
            rework_attempts,
            not_finally_accepted_unit_ids: not_finally_accepted,
            attempts,
            total_workflow_crew_minutes: total_crew_minutes,
            crew_minutes_by_role: crew_by_role,
            total_cost_minor_units: total_cost,
            costs_by_category_minor_units: costs_by_category,
            cost_per_accepted_unit,
            accepted_floor_cost_minor_units: accepted_floor_cost,
        })
    }
}

/// Apply the existing reservation authority to a floor-bound request.
/// A granted reservation remains a scheduling fact, never safety clearance.
pub fn reserve_for_floor(
    floor: &FloorBinding,
    book: &mut ReservationBook,
    request: ReservationRequest,
    now_ms: u64,
) -> Result<ReservationOutcome, LedgerError> {
    if request.site_epoch != floor.site_epoch {
        return Err(LedgerError::StaleResourceEpoch);
    }
    book.reserve(request, now_ms).map_err(LedgerError::Reservation)
}

/// A paired deterministic counterfactual for the value of an observation.
/// Inputs are explicit synthetic outcomes; this function assigns no probabilities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationArmOutcome {
    pub policy_id: String,
    pub currency_code: String,
    pub observation_cost_minor_units: u64,
    pub rework_cost_minor_units: u64,
    pub downstream_loss_cost_minor_units: u64,
    pub delay_cost_minor_units: u64,
    pub crew_minutes: u64,
}

impl ObservationArmOutcome {
    fn total_cost_minor_units(&self) -> Result<u64, LedgerError> {
        checked_add(
            checked_add(
                self.observation_cost_minor_units,
                self.rework_cost_minor_units,
            )?,
            checked_add(
                self.downstream_loss_cost_minor_units,
                self.delay_cost_minor_units,
            )?,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationValueComparison {
    pub currency_code: String,
    pub early_total_cost_minor_units: u64,
    pub deferred_total_cost_minor_units: u64,
    /// Positive means the early observation reduced cost in this paired scenario.
    pub early_cost_advantage_minor_units: i128,
    /// Positive means the early observation reduced crew-minutes.
    pub early_crew_minutes_saved: i128,
}

pub fn compare_observation_arms(
    early: &ObservationArmOutcome,
    deferred: &ObservationArmOutcome,
) -> Result<ObservationValueComparison, LedgerError> {
    validate_currency(&early.currency_code)?;
    validate_currency(&deferred.currency_code)?;
    if early.currency_code != deferred.currency_code {
        return Err(LedgerError::CurrencyMismatch {
            expected: early.currency_code.clone(),
            received: deferred.currency_code.clone(),
        });
    }
    require_non_empty("early_policy_id", &early.policy_id)?;
    require_non_empty("deferred_policy_id", &deferred.policy_id)?;
    let early_cost = early.total_cost_minor_units()?;
    let deferred_cost = deferred.total_cost_minor_units()?;

    Ok(ObservationValueComparison {
        currency_code: early.currency_code.clone(),
        early_total_cost_minor_units: early_cost,
        deferred_total_cost_minor_units: deferred_cost,
        early_cost_advantage_minor_units: i128::from(deferred_cost)
            - i128::from(early_cost),
        early_crew_minutes_saved: i128::from(deferred.crew_minutes)
            - i128::from(early.crew_minutes),
    })
}

fn valid_progression(previous: &TaskState, next: &TaskState) -> bool {
    if previous == next {
        return true;
    }
    if previous.is_terminal() {
        return false;
    }
    match previous {
        TaskState::Submitted => true,
        TaskState::Accepted => matches!(
            next,
            TaskState::Queued
                | TaskState::Started
                | TaskState::Paused
                | TaskState::Blocked
                | TaskState::Failed
                | TaskState::Cancelled
                | TaskState::RevokedOrStale
                | TaskState::Indeterminate
        ),
        TaskState::Queued => matches!(
            next,
            TaskState::Started
                | TaskState::Paused
                | TaskState::Blocked
                | TaskState::Failed
                | TaskState::Cancelled
                | TaskState::RevokedOrStale
                | TaskState::Indeterminate
        ),
        TaskState::Started => matches!(
            next,
            TaskState::Paused
                | TaskState::Blocked
                | TaskState::ExecutorCompleted
                | TaskState::Failed
                | TaskState::Cancelled
                | TaskState::RevokedOrStale
                | TaskState::Indeterminate
        ),
        TaskState::Paused | TaskState::Blocked => matches!(
            next,
            TaskState::Started
                | TaskState::Blocked
                | TaskState::Failed
                | TaskState::Cancelled
                | TaskState::RevokedOrStale
                | TaskState::Indeterminate
        ),
        TaskState::ExecutorCompleted => matches!(
            next,
            TaskState::MeasurementPending
                | TaskState::Failed
                | TaskState::RevokedOrStale
                | TaskState::Indeterminate
        ),
        TaskState::MeasurementPending => matches!(
            next,
            TaskState::IndependentlyAccepted
                | TaskState::IndependentlyRejected
                | TaskState::Failed
                | TaskState::RevokedOrStale
                | TaskState::Indeterminate
        ),
        TaskState::IndependentlyAccepted
        | TaskState::IndependentlyRejected
        | TaskState::Failed
        | TaskState::Cancelled
        | TaskState::RevokedOrStale
        | TaskState::Indeterminate => false,
    }
}

fn validate_currency(code: &str) -> Result<(), LedgerError> {
    if code.len() != 3 || !code.bytes().all(|byte| byte.is_ascii_uppercase()) {
        return Err(LedgerError::InvalidCurrencyCode);
    }
    Ok(())
}

fn require_non_empty(name: &'static str, value: &str) -> Result<(), LedgerError> {
    if value.trim().is_empty() {
        return Err(LedgerError::EmptyField(name));
    }
    Ok(())
}

fn checked_add(left: u64, right: u64) -> Result<u64, LedgerError> {
    left.checked_add(right).ok_or(LedgerError::ArithmeticOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use construction_robot_interop_core::{
        AcceptanceReceipt, ApplyOutcome, EventEnvelope, TransitionKind,
    };

    fn floor_binding() -> FloorBinding {
        FloorBinding {
            floor_id: "floor-reference-01".to_owned(),
            site_epoch: "site-epoch-1".to_owned(),
            design_revision_digest: "design-digest-1".to_owned(),
            target_frame_id: "project-grid-1".to_owned(),
        }
    }

    fn context() -> ContextBinding {
        ContextBinding {
            site_epoch: "site-epoch-1".to_owned(),
            design_revision_digest: "design-digest-1".to_owned(),
            target_frame_id: "project-grid-1".to_owned(),
            robot_config_digest: "robot-config-digest-1".to_owned(),
        }
    }

    fn ledger(required: &[&str]) -> FloorLedger {
        FloorLedger::new(
            floor_binding(),
            "USD",
            required
                .iter()
                .map(|id| WorkUnitSpec {
                    work_unit_id: (*id).to_owned(),
                    acceptance_contract_ref: format!("acceptance-contract:{id}"),
                })
                .collect(),
        )
        .expect("valid ledger")
    }

    fn send(
        machine: &mut TaskMachine,
        actor: &str,
        sequence: u64,
        transition: TransitionKind,
    ) -> Result<ApplyOutcome, construction_robot_interop_core::ApplyError> {
        machine.apply(EventEnvelope {
            message_id: format!("{actor}-message-{sequence}"),
            payload_digest: format!("{actor}-payload-{sequence}"),
            actor_id: actor.to_owned(),
            actor_session_id: "session-1".to_owned(),
            task_sequence: sequence,
            task_id: machine.task().id.clone(),
            binding: context(),
            transition,
        })
    }

    fn completed_machine(task_id: &str, accepted: bool) -> TaskMachine {
        let mut machine = TaskMachine::new(task_id, context()).expect("valid machine");
        let actor = format!("actor-{task_id}");
        send(&mut machine, &actor, 1, TransitionKind::Accept).expect("accept task");
        send(&mut machine, &actor, 2, TransitionKind::Start).expect("start task");
        send(
            &mut machine,
            &actor,
            3,
            TransitionKind::ExecutorComplete {
                output_evidence_ref: Some(format!("output:{task_id}")),
            },
        )
        .expect("executor complete");
        send(&mut machine, &actor, 4, TransitionKind::SubmitForMeasurement)
            .expect("submit measurement");
        let receipt = AcceptanceReceipt {
            receipt_ref: format!("receipt:{task_id}"),
            task_id: task_id.to_owned(),
            binding: context(),
            output_evidence_ref: format!("output:{task_id}"),
        };
        let disposition = if accepted {
            TransitionKind::InspectionAccepted { receipt }
        } else {
            TransitionKind::InspectionRejected { receipt }
        };
        send(&mut machine, &actor, 5, disposition).expect("inspection disposition");
        machine
    }

    fn executor_completed_machine(task_id: &str) -> TaskMachine {
        let mut machine = TaskMachine::new(task_id, context()).expect("valid machine");
        let actor = format!("actor-{task_id}");
        send(&mut machine, &actor, 1, TransitionKind::Accept).expect("accept task");
        send(&mut machine, &actor, 2, TransitionKind::Start).expect("start task");
        send(
            &mut machine,
            &actor,
            3,
            TransitionKind::ExecutorComplete {
                output_evidence_ref: Some(format!("output:{task_id}")),
            },
        )
        .expect("executor complete");
        machine
    }

    #[test]
    fn executor_complete_without_inspection_earns_no_accepted_output() {
        let mut ledger = ledger(&["unit-1"]);
        let machine = executor_completed_machine("task-1");
        assert_eq!(
            ledger.observe_attempt("unit-1", &machine, None),
            Ok(ObserveOutcome::Recorded)
        );
        let report = ledger.report().expect("report");
        assert_eq!(report.finally_accepted_units, 0);
        assert!(!report.floor_accepted());
        assert_eq!(report.cost_per_accepted_unit, None);
        assert_eq!(report.accepted_floor_cost_minor_units, None);
    }

    #[test]
    fn rejected_first_pass_then_authorized_rework_can_be_finally_accepted() {
        let mut ledger = ledger(&["unit-1"]);
        let rejected = completed_machine("task-attempt-1", false);
        let accepted = completed_machine("task-attempt-2", true);

        assert_eq!(
            ledger.observe_attempt("unit-1", &rejected, None),
            Ok(ObserveOutcome::Recorded)
        );
        ledger
            .add_cost(CostEntry {
                entry_id: "rejected-work-cost".to_owned(),
                task_id: Some("task-attempt-1".to_owned()),
                category: CostCategory::Rework,
                amount_minor_units: 1_200,
                currency_code: "USD".to_owned(),
            })
            .expect("record rejected attempt cost");
        assert_eq!(
            ledger.observe_attempt("unit-1", &accepted, Some("corrective-work-order-1")),
            Ok(ObserveOutcome::Recorded)
        );

        let report = ledger.report().expect("report");
        assert_eq!(report.first_pass_inspected_units, 1);
        assert_eq!(report.first_pass_accepted_units, 0);
        assert_eq!(report.first_pass_yield_numerator_denominator(), (0, 1));
        assert_eq!(report.finally_accepted_units, 1);
        assert_eq!(report.rework_attempts, 1);
        assert!(report.floor_accepted());
        assert_eq!(report.total_cost_minor_units, 1_200);
        assert_eq!(report.accepted_floor_cost_minor_units, Some(1_200));
    }

    #[test]
    fn a_missing_required_unit_prevents_floor_acceptance_and_floor_cost() {
        let mut ledger = ledger(&["unit-1", "unit-2"]);
        let accepted = completed_machine("task-1", true);
        ledger
            .observe_attempt("unit-1", &accepted, None)
            .expect("observe unit 1");
        ledger
            .add_cost(CostEntry {
                entry_id: "floor-setup".to_owned(),
                task_id: None,
                category: CostCategory::DesignAndVdcPreparation,
                amount_minor_units: 5_000,
                currency_code: "USD".to_owned(),
            })
            .expect("setup cost");

        let report = ledger.report().expect("report");
        assert_eq!(report.required_units, 2);
        assert_eq!(report.finally_accepted_units, 1);
        assert_eq!(report.not_finally_accepted_unit_ids, vec!["unit-2"]);
        assert!(!report.floor_accepted());
        assert_eq!(report.accepted_floor_cost_minor_units, None);
        assert_eq!(report.cost_per_accepted_unit, Some(MoneyPerUnit {
            numerator_minor_units: 5_000,
            denominator_accepted_units: 1,
        }));
    }

    #[test]
    fn stale_design_site_or_frame_context_is_not_admitted() {
        let mut ledger = ledger(&["unit-1"]);
        let mut stale_context = context();
        stale_context.design_revision_digest = "old-design-digest".to_owned();
        let machine = TaskMachine::new("stale-task", stale_context).expect("valid task binding");
        assert_eq!(
            ledger.observe_attempt("unit-1", &machine, None),
            Err(LedgerError::TaskContextMismatch)
        );
    }

    #[test]
    fn task_id_cannot_be_reused_across_work_units() {
        let mut ledger = ledger(&["unit-1", "unit-2"]);
        let machine = executor_completed_machine("task-shared");
        ledger
            .observe_attempt("unit-1", &machine, None)
            .expect("first task ownership");
        assert_eq!(
            ledger.observe_attempt("unit-2", &machine, None),
            Err(LedgerError::TaskIdAlreadyAssigned("task-shared".to_owned()))
        );
    }

    #[test]
    fn overlapping_person_time_is_rejected_but_adjacent_half_open_slices_are_valid() {
        let mut ledger = ledger(&["unit-1"]);
        ledger
            .add_labour_slice(LabourSlice {
                slice_id: "slice-1".to_owned(),
                person_id: "worker-1".to_owned(),
                role: "operator".to_owned(),
                start_minute: 0,
                end_minute: 30,
            })
            .expect("first slice");

        assert_eq!(
            ledger.add_labour_slice(LabourSlice {
                slice_id: "slice-overlap".to_owned(),
                person_id: "worker-1".to_owned(),
                role: "operator".to_owned(),
                start_minute: 29,
                end_minute: 45,
            }),
            Err(LedgerError::OverlappingPersonTime {
                person_id: "worker-1".to_owned(),
                existing_slice_id: "slice-1".to_owned(),
                new_slice_id: "slice-overlap".to_owned(),
            })
        );
        ledger
            .add_labour_slice(LabourSlice {
                slice_id: "slice-adjacent".to_owned(),
                person_id: "worker-1".to_owned(),
                role: "operator".to_owned(),
                start_minute: 30,
                end_minute: 45,
            })
            .expect("adjacent half-open slice");

        let report = ledger.report().expect("report");
        assert_eq!(report.total_workflow_crew_minutes, 45);
        assert_eq!(report.crew_minutes_by_role.get("operator"), Some(&45));
    }

    #[test]
    fn duplicate_cost_currency_mismatch_and_unknown_task_are_rejected() {
        let mut ledger = ledger(&["unit-1"]);
        let duplicate = CostEntry {
            entry_id: "cost-1".to_owned(),
            task_id: None,
            category: CostCategory::Materials,
            amount_minor_units: 100,
            currency_code: "USD".to_owned(),
        };
        ledger.add_cost(duplicate.clone()).expect("first cost");
        assert_eq!(
            ledger.add_cost(duplicate),
            Err(LedgerError::DuplicateCostEntry("cost-1".to_owned()))
        );

        assert_eq!(
            ledger.add_cost(CostEntry {
                entry_id: "cost-eur".to_owned(),
                task_id: None,
                category: CostCategory::Materials,
                amount_minor_units: 100,
                currency_code: "EUR".to_owned(),
            }),
            Err(LedgerError::CurrencyMismatch {
                expected: "USD".to_owned(),
                received: "EUR".to_owned(),
            })
        );
        assert_eq!(
            ledger.add_cost(CostEntry {
                entry_id: "cost-unknown-task".to_owned(),
                task_id: Some("missing-task".to_owned()),
                category: CostCategory::Labour,
                amount_minor_units: 100,
                currency_code: "USD".to_owned(),
            }),
            Err(LedgerError::UnknownTask("missing-task".to_owned()))
        );
    }

    fn reservation(
        id: &str,
        task: &str,
        start: u64,
        end: u64,
    ) -> ReservationRequest {
        ReservationRequest {
            reservation_id: id.to_owned(),
            resource_id: "HOIST-1".to_owned(),
            task_id: task.to_owned(),
            owner_id: format!("owner:{task}"),
            site_epoch: "site-epoch-1".to_owned(),
            clock_domain_id: "site-clock-1".to_owned(),
            clock_uncertainty_ms: 0,
            interval_start_ms: start,
            interval_end_ms: end,
            expires_at_ms: end + 100,
            units: 1,
        }
    }

    #[test]
    fn floor_adapter_reuses_existing_resource_reservation_owner() {
        let floor = floor_binding();
        let mut book = ReservationBook::new("site-epoch-1", "site-clock-1", 0)
            .expect("reservation book");
        book.register_resource("HOIST-1", 1).expect("resource");
        assert_eq!(
            reserve_for_floor(&floor, &mut book, reservation("r1", "task-1", 10, 20), 0),
            Ok(ReservationOutcome::Granted)
        );
        assert!(matches!(
            reserve_for_floor(&floor, &mut book, reservation("r2", "task-2", 15, 25), 0),
            Err(LedgerError::Reservation(ReservationError::CapacityExceeded { .. }))
        ));
        assert_eq!(
            reserve_for_floor(
                &floor,
                &mut book,
                reservation("r3", "task-3", 20, 30),
                0
            ),
            Ok(ReservationOutcome::Granted)
        );
    }

    #[test]
    fn floor_epoch_mismatch_blocks_resource_reservation_before_owner_call() {
        let floor = floor_binding();
        let mut book = ReservationBook::new("site-epoch-1", "site-clock-1", 0)
            .expect("reservation book");
        book.register_resource("HOIST-1", 1).expect("resource");
        let mut request = reservation("r1", "task-1", 10, 20);
        request.site_epoch = "site-epoch-old".to_owned();
        assert_eq!(
            reserve_for_floor(&floor, &mut book, request, 0),
            Err(LedgerError::StaleResourceEpoch)
        );
    }

    #[test]
    fn observation_can_have_positive_or_negative_value_in_paired_synthetic_cases() {
        let early = ObservationArmOutcome {
            policy_id: "early-survey".to_owned(),
            currency_code: "USD".to_owned(),
            observation_cost_minor_units: 100,
            rework_cost_minor_units: 50,
            downstream_loss_cost_minor_units: 0,
            delay_cost_minor_units: 0,
            crew_minutes: 20,
        };
        let deferred = ObservationArmOutcome {
            policy_id: "deferred-detection".to_owned(),
            currency_code: "USD".to_owned(),
            observation_cost_minor_units: 0,
            rework_cost_minor_units: 300,
            downstream_loss_cost_minor_units: 800,
            delay_cost_minor_units: 500,
            crew_minutes: 60,
        };
        let comparison = compare_observation_arms(&early, &deferred).expect("compare arms");
        assert_eq!(comparison.early_cost_advantage_minor_units, 1_450);
        assert_eq!(comparison.early_crew_minutes_saved, 40);

        let expensive_early = ObservationArmOutcome {
            observation_cost_minor_units: 2_000,
            rework_cost_minor_units: 0,
            downstream_loss_cost_minor_units: 0,
            delay_cost_minor_units: 0,
            crew_minutes: 30,
            ..early
        };
        let no_observation = ObservationArmOutcome {
            policy_id: "no-observation".to_owned(),
            currency_code: "USD".to_owned(),
            observation_cost_minor_units: 0,
            rework_cost_minor_units: 0,
            downstream_loss_cost_minor_units: 0,
            delay_cost_minor_units: 0,
            crew_minutes: 10,
        };
        let negative = compare_observation_arms(&expensive_early, &no_observation)
            .expect("compare negative value case");
        assert!(negative.early_cost_advantage_minor_units < 0);
        assert!(negative.early_crew_minutes_saved < 0);
    }

    #[test]
    fn report_is_deterministic_for_identical_inputs() {
        let mut left = ledger(&["unit-1"]);
        let mut right = ledger(&["unit-1"]);
        let left_machine = completed_machine("task-1", true);
        let right_machine = completed_machine("task-1", true);
        left.observe_attempt("unit-1", &left_machine, None)
            .expect("left attempt");
        right.observe_attempt("unit-1", &right_machine, None)
            .expect("right attempt");
        for item in [&mut left, &mut right] {
            item.add_cost(CostEntry {
                entry_id: "cost-1".to_owned(),
                task_id: Some("task-1".to_owned()),
                category: CostCategory::Inspection,
                amount_minor_units: 500,
                currency_code: "USD".to_owned(),
            })
            .expect("cost");
            item.add_labour_slice(LabourSlice {
                slice_id: "labour-1".to_owned(),
                person_id: "inspector-1".to_owned(),
                role: "inspector".to_owned(),
                start_minute: 0,
                end_minute: 15,
            })
            .expect("labour");
        }
        assert_eq!(left.report(), right.report());
    }

    #[test]
    fn rejected_attempt_and_rework_costs_both_remain_in_whole_floor_total() {
        let mut ledger = ledger(&["unit-1"]);
        let rejected = completed_machine("attempt-1", false);
        let accepted = completed_machine("attempt-2", true);
        ledger
            .observe_attempt("unit-1", &rejected, None)
            .expect("rejected attempt");
        ledger
            .add_cost(CostEntry {
                entry_id: "first-pass-cost".to_owned(),
                task_id: Some("attempt-1".to_owned()),
                category: CostCategory::RobotOperation,
                amount_minor_units: 300,
                currency_code: "USD".to_owned(),
            })
            .expect("first pass cost");
        ledger
            .observe_attempt("unit-1", &accepted, Some("approved-correction-1"))
            .expect("rework attempt");
        ledger
            .add_cost(CostEntry {
                entry_id: "rework-cost".to_owned(),
                task_id: Some("attempt-2".to_owned()),
                category: CostCategory::Rework,
                amount_minor_units: 700,
                currency_code: "USD".to_owned(),
            })
            .expect("rework cost");

        let report = ledger.report().expect("report");
        assert_eq!(report.total_cost_minor_units, 1_000);
        assert_eq!(
            report.costs_by_category_minor_units.get(&CostCategory::RobotOperation),
            Some(&300)
        );
        assert_eq!(
            report.costs_by_category_minor_units.get(&CostCategory::Rework),
            Some(&700)
        );
    }
}
