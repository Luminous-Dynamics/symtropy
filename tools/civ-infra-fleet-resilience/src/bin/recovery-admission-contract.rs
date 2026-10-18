use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::{process, string::ToString};

const EMBEDDED_FIXTURE: &str = include_str!(
    "../../../../docs/research/civ-infra-resilience/fixtures/fleet-resilience-v0.1.json"
);
const MAX_CONTRACT_TICK: u32 = 256;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum TaskState {
    Pending,
    Ready,
    Running,
    Interrupted,
    AwaitingEvidence,
    ReworkRequired,
    Accepted,
    Rejected,
    Aborted,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum Action {
    Start,
    ProtectiveStop,
    Resume,
    Reassign,
    SubmitForAcceptance,
    Accept,
    RejectInspection,
    BeginRework,
    Abort,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum UtilityEnvelope {
    Clear,
    ApprovedAlternative,
    UnknownOrUnsafe,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Authorization {
    NotRequired,
    Granted,
    Pending,
    Declined,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum InspectionOutcome {
    NotPerformed,
    Pending,
    Passed,
    Failed,
}

#[derive(Clone, Debug)]
struct TaskSpec {
    task_id: String,
    requires: Vec<String>,
    acceptance_evidence: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct PolicyObservation {
    observation_id: String,
    case_id: String,
    name: String,
    observable_tick: u32,
    affected_resource_ids: Vec<String>,
    affected_task_ids: Vec<String>,
    invalidated_evidence: Vec<String>,
    required_recovery_evidence: Vec<String>,
}

#[derive(Clone, Debug)]
struct Contract {
    fixture_id: String,
    fixture_sha256: String,
    resource_ids: BTreeSet<String>,
    evidence_vocabulary: BTreeSet<String>,
    tasks: BTreeMap<String, TaskSpec>,
    observations: BTreeMap<String, PolicyObservation>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct Evidence {
    evidence_id: String,
    evidence_kind: String,
    observed_at_tick: u32,
    current: bool,
    valid: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct TaskReceipt {
    task_id: String,
    state: TaskState,
    evidence: Vec<Evidence>,
}

#[derive(Clone, Debug)]
struct GateSnapshot {
    resource_available: bool,
    capacity_admissible: bool,
    capability_current: bool,
    calibration_current: bool,
    localization_admissible: bool,
    communication_current: bool,
    energy_reserve_feasible: bool,
    utility_envelope: UtilityEnvelope,
    authorization: Authorization,
    inspection: InspectionOutcome,
    work_complete: bool,
}

#[derive(Clone, Debug)]
struct Proposal {
    attempt_id: String,
    task_id: String,
    state: TaskState,
    action: Action,
    tick: u32,
    resource_id: Option<String>,
    previous_resource_id: Option<String>,
    observation_ids: Vec<String>,
    predecessor_receipts: Vec<TaskReceipt>,
    evidence: Vec<Evidence>,
    gates: GateSnapshot,
}

#[derive(Clone, Debug, Serialize)]
struct Decision {
    admitted: bool,
    next_state: Option<TaskState>,
    reason_codes: Vec<String>,
}

#[derive(Clone, Debug)]
struct Scenario {
    scenario_id: String,
    case_id: Option<String>,
    expected_admitted: bool,
    proposal: Proposal,
}

#[derive(Clone, Debug, Serialize)]
struct ScenarioResult {
    scenario_id: String,
    case_id: Option<String>,
    expected_admitted: bool,
    admitted: bool,
    next_state: Option<TaskState>,
    reason_codes: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct ContractReport {
    schema_version: String,
    fixture_id: String,
    fixture_sha256: String,
    classification: String,
    validation_status: String,
    contract_suite_executed: bool,
    scenario_count: usize,
    admitted_count: usize,
    blocked_count: usize,
    expectation_mismatch_count: usize,
    all_expected_decisions_match: bool,
    policy_simulation_executed: bool,
    policy_runs_performed: u32,
    physical_control_executed: bool,
    dispatch_authorized: bool,
    input_receipts_cryptographically_verified: bool,
    immutable_history_store_executed: bool,
    gate_inputs_are_synthetic_assertions: bool,
    claim_ceiling: String,
    scenario_results: Vec<ScenarioResult>,
}

impl GateSnapshot {
    fn safe() -> Self {
        Self {
            resource_available: true,
            capacity_admissible: true,
            capability_current: true,
            calibration_current: true,
            localization_admissible: true,
            communication_current: true,
            energy_reserve_feasible: true,
            utility_envelope: UtilityEnvelope::Clear,
            authorization: Authorization::NotRequired,
            inspection: InspectionOutcome::NotPerformed,
            work_complete: true,
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn string_field(value: &Value, key: &str, path: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{path}.{key} must be a string"))
}

fn string_array(value: &Value, key: &str, path: &str) -> Result<Vec<String>, String> {
    let items = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{path}.{key} must be an array"))?;
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            item.as_str()
                .map(ToString::to_string)
                .ok_or_else(|| format!("{path}.{key}[{index}] must be a string"))
        })
        .collect()
}

fn tick_field(value: &Value, key: &str, path: &str) -> Result<u32, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|tick| u32::try_from(tick).ok())
        .ok_or_else(|| format!("{path}.{key} must be a bounded unsigned tick"))
}

fn build_contract(fixture_bytes: &[u8]) -> Result<Contract, String> {
    let root: Value = serde_json::from_slice(fixture_bytes)
        .map_err(|error| format!("cannot parse embedded manifest: {error}"))?;
    if string_field(&root, "schema_version", "fixture")? != "civ-infra-fleet-resilience/0.1" {
        return Err("unexpected fleet-resilience schema version".to_string());
    }
    if string_field(&root, "classification", "fixture")? != "synthetic_only" {
        return Err("fixture must remain explicitly synthetic_only".to_string());
    }
    let fixture_id = string_field(&root, "fixture_id", "fixture")?;

    let mut resource_ids = BTreeSet::new();
    let resources = root
        .get("resources")
        .and_then(Value::as_array)
        .ok_or_else(|| "fixture.resources must be an array".to_string())?;
    for (index, resource) in resources.iter().enumerate() {
        let id = string_field(resource, "resource_id", &format!("resources[{index}]"))?;
        if id.is_empty() || !resource_ids.insert(id.clone()) {
            return Err(format!("empty or duplicate resource id: {id}"));
        }
    }

    let evidence_vocabulary: BTreeSet<String> =
        string_array(&root, "evidence_vocabulary", "fixture")?
            .into_iter()
            .collect();
    if evidence_vocabulary.is_empty() {
        return Err("evidence vocabulary must not be empty".to_string());
    }

    let task_items = root
        .get("task_graph")
        .and_then(Value::as_array)
        .ok_or_else(|| "fixture.task_graph must be an array".to_string())?;
    let mut tasks = BTreeMap::new();
    for (index, item) in task_items.iter().enumerate() {
        let path = format!("task_graph[{index}]");
        let task_id = string_field(item, "task_id", &path)?;
        let requires = string_array(item, "requires", &path)?;
        let acceptance_evidence = string_array(item, "acceptance_evidence", &path)?;
        if task_id.is_empty()
            || acceptance_evidence.is_empty()
            || acceptance_evidence
                .iter()
                .any(|kind| !evidence_vocabulary.contains(kind))
        {
            return Err(format!("invalid task definition: {task_id}"));
        }
        if tasks
            .insert(
                task_id.clone(),
                TaskSpec {
                    task_id: task_id.clone(),
                    requires,
                    acceptance_evidence,
                },
            )
            .is_some()
        {
            return Err(format!("duplicate task id: {task_id}"));
        }
    }
    for task in tasks.values() {
        for required_task in &task.requires {
            if !tasks.contains_key(required_task) {
                return Err(format!(
                    "task {} has unknown predecessor {}",
                    task.task_id, required_task
                ));
            }
        }
    }

    let case_items = root
        .get("disturbance_cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "fixture.disturbance_cases must be an array".to_string())?;
    let mut observations = BTreeMap::new();
    for (index, item) in case_items.iter().enumerate() {
        let path = format!("disturbance_cases[{index}]");
        // The policy projection intentionally reads observable_tick only.
        // The injection timestamp is not represented in PolicyObservation.
        let case_id = string_field(item, "case_id", &path)?;
        let name = string_field(item, "name", &path)?;
        let injection_tick = tick_field(item, "injection_tick", &path)?;
        let observable_tick = tick_field(item, "observable_tick", &path)?;
        if injection_tick > observable_tick || injection_tick > MAX_CONTRACT_TICK {
            return Err(format!(
                "observation {case_id} precedes its injection or exceeds the contract horizon"
            ));
        }
        let affected_resource_ids = string_array(item, "affected_resources", &path)?;
        let affected_task_ids = string_array(item, "affected_tasks", &path)?;
        let invalidated_evidence = string_array(item, "invalidates", &path)?;
        let required_recovery_evidence = string_array(item, "recovery_evidence", &path)?;
        if case_id.is_empty()
            || name.is_empty()
            || observable_tick > MAX_CONTRACT_TICK
            || affected_task_ids.is_empty()
            || required_recovery_evidence.is_empty()
        {
            return Err(format!("invalid disturbance observation: {case_id}"));
        }
        if affected_task_ids.iter().any(|id| !tasks.contains_key(id))
            || affected_resource_ids
                .iter()
                .any(|id| !resource_ids.contains(id))
            || required_recovery_evidence
                .iter()
                .any(|kind| !evidence_vocabulary.contains(kind))
        {
            return Err(format!("observation {case_id} has unresolved references"));
        }
        let observation = PolicyObservation {
            observation_id: case_id.clone(),
            case_id: case_id.clone(),
            name,
            observable_tick,
            affected_resource_ids,
            affected_task_ids,
            invalidated_evidence,
            required_recovery_evidence,
        };
        if observations.insert(case_id.clone(), observation).is_some() {
            return Err(format!("duplicate observation id: {case_id}"));
        }
    }
    if observations.len() != 13 {
        return Err(format!(
            "expected 13 typed observations, found {}",
            observations.len()
        ));
    }

    Ok(Contract {
        fixture_id,
        fixture_sha256: sha256_hex(fixture_bytes),
        resource_ids,
        evidence_vocabulary,
        tasks,
        observations,
    })
}

fn action_uses_resource(action: Action) -> bool {
    matches!(action, Action::Start | Action::Resume | Action::Reassign)
}

fn action_uses_policy_observations(action: Action) -> bool {
    !matches!(action, Action::ProtectiveStop | Action::Abort)
}

fn transition_target(state: TaskState, action: Action) -> Option<TaskState> {
    use Action::*;
    use TaskState::*;
    match (state, action) {
        (Pending | Ready, Start) => Some(Running),
        (Running, ProtectiveStop) => Some(Interrupted),
        (Interrupted | ReworkRequired, Resume) => Some(Running),
        (Interrupted | ReworkRequired, Reassign) => Some(Running),
        (Running, SubmitForAcceptance) => Some(AwaitingEvidence),
        (AwaitingEvidence, Accept) => Some(Accepted),
        (AwaitingEvidence, RejectInspection) => Some(Rejected),
        (Rejected, BeginRework) => Some(ReworkRequired),
        (
            Pending | Ready | Running | Interrupted | AwaitingEvidence | ReworkRequired | Rejected,
            Abort,
        ) => Some(Aborted),
        _ => None,
    }
}

fn task_ancestor_closure(contract: &Contract, task_id: &str) -> BTreeSet<String> {
    fn visit(contract: &Contract, task_id: &str, seen: &mut BTreeSet<String>) {
        if !seen.insert(task_id.to_string()) {
            return;
        }
        if let Some(task) = contract.tasks.get(task_id) {
            for predecessor in &task.requires {
                visit(contract, predecessor, seen);
            }
        }
    }
    let mut seen = BTreeSet::new();
    visit(contract, task_id, &mut seen);
    seen
}

fn relevant_observations<'a>(
    contract: &'a Contract,
    task_id: &str,
    resource_id: Option<&str>,
    tick: u32,
) -> Vec<&'a PolicyObservation> {
    let task_closure = task_ancestor_closure(contract, task_id);
    contract
        .observations
        .values()
        .filter(|observation| observation.observable_tick <= tick)
        .filter(|observation| {
            observation
                .affected_task_ids
                .iter()
                .any(|id| task_closure.contains(id))
                || resource_id.is_some_and(|resource| {
                    observation
                        .affected_resource_ids
                        .iter()
                        .any(|id| id == resource)
                })
        })
        .collect()
}

fn evidence_satisfies(evidence: &[Evidence], kind: &str, tick: u32, minimum_tick: u32) -> bool {
    evidence.iter().any(|item| {
        item.evidence_kind == kind
            && item.current
            && item.valid
            && item.observed_at_tick >= minimum_tick
            && item.observed_at_tick <= tick
    })
}

fn active_observations<'a>(
    contract: &'a Contract,
    proposal: &Proposal,
) -> Vec<&'a PolicyObservation> {
    proposal
        .observation_ids
        .iter()
        .filter_map(|id| contract.observations.get(id))
        .filter(|observation| observation.observable_tick <= proposal.tick)
        .collect()
}

fn required_evidence_tick(observation_refs: &[&PolicyObservation], evidence_kind: &str) -> u32 {
    observation_refs
        .iter()
        .filter(|observation| {
            observation
                .invalidated_evidence
                .iter()
                .any(|kind| kind == evidence_kind)
        })
        .map(|observation| observation.observable_tick)
        .max()
        .unwrap_or(0)
}

fn recovery_evidence_is_required(proposal: &Proposal, observation: &PolicyObservation) -> bool {
    if matches!(proposal.action, Action::Accept) {
        return true;
    }
    // Post-work receipts must not be required before rework has begun.
    // Final acceptance still requires every recovery receipt, including reinspection.
    if observation.case_id == "R-007" && proposal.action == Action::BeginRework {
        return false;
    }
    !(observation.case_id == "R-007"
        && proposal.state == TaskState::ReworkRequired
        && matches!(proposal.action, Action::Resume | Action::Reassign))
}

fn check_predecessor_receipts(
    contract: &Contract,
    proposal: &Proposal,
    task: &TaskSpec,
    observation_refs: &[&PolicyObservation],
    reasons: &mut BTreeSet<String>,
) {
    for predecessor_id in &task.requires {
        let matches: Vec<&TaskReceipt> = proposal
            .predecessor_receipts
            .iter()
            .filter(|receipt| receipt.task_id == *predecessor_id)
            .collect();
        if matches.is_empty() {
            reasons.insert(format!("predecessor_receipt_missing:{predecessor_id}"));
            continue;
        }
        if matches.len() > 1 {
            reasons.insert(format!("predecessor_receipt_duplicated:{predecessor_id}"));
            continue;
        }
        let receipt = matches[0];
        if receipt.state != TaskState::Accepted {
            reasons.insert(format!("predecessor_not_accepted:{predecessor_id}"));
        }
        let Some(predecessor) = contract.tasks.get(predecessor_id) else {
            reasons.insert(format!("unknown_predecessor:{predecessor_id}"));
            continue;
        };
        for kind in &predecessor.acceptance_evidence {
            let minimum_tick = required_evidence_tick(observation_refs, kind);
            if !evidence_satisfies(&receipt.evidence, kind, proposal.tick, minimum_tick) {
                reasons.insert(format!(
                    "predecessor_acceptance_evidence_missing_or_stale:{predecessor_id}:{kind}"
                ));
            }
        }
    }
    for receipt in &proposal.predecessor_receipts {
        if !task.requires.iter().any(|id| id == &receipt.task_id) {
            reasons.insert(format!(
                "unexpected_predecessor_receipt:{}",
                receipt.task_id
            ));
        }
        if receipt
            .evidence
            .iter()
            .any(|item| item.observed_at_tick > proposal.tick)
        {
            reasons.insert(format!(
                "future_dated_predecessor_evidence:{}",
                receipt.task_id
            ));
        }
    }
}

fn assess(contract: &Contract, proposal: &Proposal) -> Decision {
    let mut reasons = BTreeSet::new();
    let Some(task) = contract.tasks.get(&proposal.task_id) else {
        return Decision {
            admitted: false,
            next_state: None,
            reason_codes: vec![format!("unknown_task:{}", proposal.task_id)],
        };
    };
    if proposal.tick > MAX_CONTRACT_TICK {
        reasons.insert("tick_outside_contract_horizon".to_string());
    }
    let target_state = transition_target(proposal.state, proposal.action);
    if target_state.is_none() {
        reasons.insert("invalid_state_transition".to_string());
    }

    let mut observation_ids = BTreeSet::new();
    for id in &proposal.observation_ids {
        if !observation_ids.insert(id.clone()) {
            reasons.insert(format!("duplicate_observation_reference:{id}"));
        }
    }

    let resource_for_context = if action_uses_resource(proposal.action) {
        proposal.resource_id.as_deref()
    } else {
        None
    };
    if action_uses_policy_observations(proposal.action) {
        for observation in relevant_observations(
            contract,
            &proposal.task_id,
            resource_for_context,
            proposal.tick,
        ) {
            if !observation_ids.contains(&observation.observation_id) {
                reasons.insert(format!(
                    "visible_observation_omitted:{}",
                    observation.observation_id
                ));
            }
        }
    }

    for observation_id in &proposal.observation_ids {
        match contract.observations.get(observation_id) {
            None => {
                reasons.insert(format!("unknown_observation:{observation_id}"));
            }
            Some(observation) => {
                if proposal.tick < observation.observable_tick {
                    reasons.insert(format!("observation_not_yet_visible:{observation_id}"));
                }
                let closure = task_ancestor_closure(contract, &proposal.task_id);
                let task_relevant = observation
                    .affected_task_ids
                    .iter()
                    .any(|id| closure.contains(id));
                let resource_relevant = resource_for_context.is_some_and(|resource| {
                    observation
                        .affected_resource_ids
                        .iter()
                        .any(|id| id == resource)
                });
                if !task_relevant && !resource_relevant {
                    reasons.insert(format!("irrelevant_observation:{observation_id}"));
                }
            }
        }
    }

    let visible_refs = active_observations(contract, proposal);
    let mut evidence_ids = BTreeSet::new();
    for evidence in &proposal.evidence {
        if evidence.evidence_id.is_empty() || !evidence_ids.insert(evidence.evidence_id.clone()) {
            reasons.insert(format!(
                "empty_or_duplicate_evidence_id:{}",
                evidence.evidence_id
            ));
        }
        if !contract
            .evidence_vocabulary
            .contains(&evidence.evidence_kind)
        {
            reasons.insert(format!("unknown_evidence_kind:{}", evidence.evidence_kind));
        }
        if evidence.observed_at_tick > proposal.tick {
            reasons.insert(format!("future_dated_evidence:{}", evidence.evidence_kind));
        }
    }

    if matches!(
        proposal.action,
        Action::Start
            | Action::Resume
            | Action::Reassign
            | Action::SubmitForAcceptance
            | Action::Accept
    ) {
        check_predecessor_receipts(contract, proposal, task, &visible_refs, &mut reasons);
    }

    if action_uses_resource(proposal.action) {
        match proposal.resource_id.as_deref() {
            None => {
                reasons.insert("resource_required_for_execution".to_string());
            }
            Some(resource_id) if !contract.resource_ids.contains(resource_id) => {
                reasons.insert(format!("unknown_resource:{resource_id}"));
            }
            Some(_) => {}
        }
        if !proposal.gates.resource_available {
            reasons.insert("resource_not_available".to_string());
        }
        if !proposal.gates.capacity_admissible {
            reasons.insert("resource_capacity_overcommitment".to_string());
        }
        if !proposal.gates.capability_current {
            reasons.insert("capability_profile_stale".to_string());
        }
        if !proposal.gates.calibration_current {
            reasons.insert("calibration_stale".to_string());
        }
        if !proposal.gates.localization_admissible {
            reasons.insert("localization_out_of_profile".to_string());
        }
        if !proposal.gates.communication_current {
            reasons.insert("communication_state_stale".to_string());
        }
        if !proposal.gates.energy_reserve_feasible {
            reasons.insert("energy_return_reserve_infeasible".to_string());
        }
        if matches!(
            proposal.gates.utility_envelope,
            UtilityEnvelope::UnknownOrUnsafe
        ) {
            reasons.insert("unknown_utility_is_not_clear".to_string());
        }
        if matches!(
            proposal.gates.authorization,
            Authorization::Pending | Authorization::Declined
        ) {
            reasons.insert("authorization_not_granted".to_string());
        }
        if proposal.action == Action::Reassign {
            match proposal.previous_resource_id.as_deref() {
                None => {
                    reasons.insert("reassignment_missing_previous_resource".to_string());
                }
                Some(previous) if previous == proposal.resource_id.as_deref().unwrap_or("") => {
                    reasons.insert("reassignment_did_not_change_resource".to_string());
                }
                Some(previous) if !contract.resource_ids.contains(previous) => {
                    reasons.insert(format!("unknown_previous_resource:{previous}"));
                }
                Some(_) => {}
            }
        }
    }

    if matches!(proposal.action, Action::SubmitForAcceptance) && !proposal.gates.work_complete {
        reasons.insert("cannot_submit_incomplete_work_for_acceptance".to_string());
    }

    if matches!(proposal.action, Action::Accept) {
        if !proposal.gates.work_complete {
            reasons.insert("cannot_accept_incomplete_work".to_string());
        }
        if proposal.gates.inspection != InspectionOutcome::Passed {
            reasons.insert("acceptance_requires_successful_inspection".to_string());
        }
        for kind in &task.acceptance_evidence {
            let minimum_tick = required_evidence_tick(&visible_refs, kind);
            if !evidence_satisfies(&proposal.evidence, kind, proposal.tick, minimum_tick) {
                reasons.insert(format!("acceptance_evidence_missing_or_stale:{kind}"));
            }
        }
    }

    if matches!(proposal.action, Action::RejectInspection)
        && proposal.gates.inspection != InspectionOutcome::Failed
    {
        reasons.insert("rejection_requires_failed_inspection".to_string());
    }

    if matches!(proposal.action, Action::BeginRework) {
        if proposal.gates.inspection != InspectionOutcome::Failed {
            reasons.insert("rework_requires_preserved_failed_inspection".to_string());
        }
        if !visible_refs
            .iter()
            .any(|observation| observation.case_id == "R-007")
        {
            reasons.insert("rework_requires_visible_rejection_observation:R-007".to_string());
        }
    }

    for observation in &visible_refs {
        if !recovery_evidence_is_required(proposal, observation) {
            continue;
        }
        for kind in &observation.required_recovery_evidence {
            if !evidence_satisfies(
                &proposal.evidence,
                kind,
                proposal.tick,
                observation.observable_tick,
            ) {
                reasons.insert(format!(
                    "recovery_evidence_missing_or_stale:{}:{kind}",
                    observation.case_id
                ));
            }
        }
    }

    if matches!(proposal.action, Action::ProtectiveStop) && proposal.state != TaskState::Running {
        reasons.insert("protective_stop_requires_running_state".to_string());
    }

    let reason_codes: Vec<String> = reasons.into_iter().collect();
    let admitted = reason_codes.is_empty();
    Decision {
        admitted,
        next_state: if admitted { target_state } else { None },
        reason_codes,
    }
}

fn evidence_map_for_attempt(
    contract: &Contract,
    task_id: &str,
    tick: u32,
    observations: &[&PolicyObservation],
) -> Vec<Evidence> {
    let mut required_ticks: BTreeMap<String, u32> = BTreeMap::new();
    if let Some(task) = contract.tasks.get(task_id) {
        for kind in &task.acceptance_evidence {
            required_ticks.entry(kind.clone()).or_insert(0);
        }
    }
    for observation in observations {
        for kind in &observation.required_recovery_evidence {
            required_ticks
                .entry(kind.clone())
                .and_modify(|existing| *existing = (*existing).max(observation.observable_tick))
                .or_insert(observation.observable_tick);
        }
        for kind in &observation.invalidated_evidence {
            required_ticks
                .entry(kind.clone())
                .and_modify(|existing| *existing = (*existing).max(observation.observable_tick))
                .or_insert(observation.observable_tick);
        }
    }
    required_ticks
        .into_iter()
        .enumerate()
        .map(|(index, (kind, minimum_tick))| Evidence {
            evidence_id: format!("evidence-{index:03}-{kind}"),
            evidence_kind: kind,
            observed_at_tick: minimum_tick.min(tick),
            current: true,
            valid: true,
        })
        .collect()
}

fn predecessor_receipts(contract: &Contract, task_id: &str, tick: u32) -> Vec<TaskReceipt> {
    let Some(task) = contract.tasks.get(task_id) else {
        return Vec::new();
    };
    task.requires
        .iter()
        .filter_map(|predecessor_id| {
            let predecessor = contract.tasks.get(predecessor_id)?;
            Some(TaskReceipt {
                task_id: predecessor.task_id.clone(),
                state: TaskState::Accepted,
                evidence: predecessor
                    .acceptance_evidence
                    .iter()
                    .enumerate()
                    .map(|(index, kind)| Evidence {
                        evidence_id: format!("receipt-{predecessor_id}-{index:02}"),
                        evidence_kind: kind.clone(),
                        observed_at_tick: tick,
                        current: true,
                        valid: true,
                    })
                    .collect(),
            })
        })
        .collect()
}

fn make_proposal(
    contract: &Contract,
    attempt_id: &str,
    task_id: &str,
    state: TaskState,
    action: Action,
    tick: u32,
    resource_id: Option<String>,
) -> Proposal {
    let resource_context = if action_uses_resource(action) {
        resource_id.as_deref()
    } else {
        None
    };
    let visible = relevant_observations(contract, task_id, resource_context, tick);
    let mut gates = GateSnapshot::safe();
    gates.inspection = if action == Action::Accept {
        InspectionOutcome::Passed
    } else {
        InspectionOutcome::NotPerformed
    };
    gates.work_complete = matches!(action, Action::SubmitForAcceptance | Action::Accept);
    if visible
        .iter()
        .any(|observation| observation.case_id == "R-011")
    {
        gates.utility_envelope = UtilityEnvelope::ApprovedAlternative;
    }
    if visible
        .iter()
        .any(|observation| observation.case_id == "R-012")
    {
        gates.authorization = Authorization::Granted;
    }
    Proposal {
        attempt_id: attempt_id.to_string(),
        task_id: task_id.to_string(),
        state,
        action,
        tick,
        resource_id,
        previous_resource_id: None,
        observation_ids: visible
            .iter()
            .map(|observation| observation.observation_id.clone())
            .collect(),
        predecessor_receipts: predecessor_receipts(contract, task_id, tick),
        evidence: evidence_map_for_attempt(contract, task_id, tick, &visible),
        gates,
    }
}

fn first_resource_for_observation(observation: &PolicyObservation) -> Option<String> {
    observation.affected_resource_ids.first().cloned()
}

fn build_scenarios(contract: &Contract) -> Result<Vec<Scenario>, String> {
    let mut scenarios = Vec::new();
    for observation in contract.observations.values() {
        let task_id = observation
            .affected_task_ids
            .first()
            .ok_or_else(|| format!("{} has no task for contract scenario", observation.case_id))?;

        let positive = make_proposal(
            contract,
            &format!("{}-accept-after-recovery", observation.case_id),
            task_id,
            TaskState::AwaitingEvidence,
            Action::Accept,
            observation.observable_tick,
            None,
        );
        scenarios.push(Scenario {
            scenario_id: format!("{}-accept-after-recovery", observation.case_id),
            case_id: Some(observation.case_id.clone()),
            expected_admitted: true,
            proposal: positive.clone(),
        });

        let mut missing_recovery = positive;
        let missing_kind = observation
            .required_recovery_evidence
            .first()
            .ok_or_else(|| format!("{} has no recovery evidence", observation.case_id))?;
        missing_recovery
            .evidence
            .retain(|item| item.evidence_kind != *missing_kind);
        scenarios.push(Scenario {
            scenario_id: format!("{}-missing-recovery-evidence", observation.case_id),
            case_id: Some(observation.case_id.clone()),
            expected_admitted: false,
            proposal: missing_recovery,
        });

        if observation.observable_tick > 0 {
            let tick = observation.observable_tick - 1;
            let resource_id = first_resource_for_observation(observation)
                .or_else(|| contract.resource_ids.iter().next().cloned());
            let mut premature = make_proposal(
                contract,
                &format!("{}-pre-observation", observation.case_id),
                task_id,
                TaskState::Interrupted,
                Action::Resume,
                tick,
                resource_id,
            );
            if !premature
                .observation_ids
                .contains(&observation.observation_id)
            {
                premature
                    .observation_ids
                    .push(observation.observation_id.clone());
                premature.observation_ids.sort();
            }
            scenarios.push(Scenario {
                scenario_id: format!("{}-future-observation-reference", observation.case_id),
                case_id: Some(observation.case_id.clone()),
                expected_admitted: false,
                proposal: premature,
            });
        }
    }

    let mut missing_predecessor = make_proposal(
        contract,
        "global-missing-predecessor",
        "trench-segment-01",
        TaskState::Ready,
        Action::Start,
        0,
        Some("excavator-01".to_string()),
    );
    missing_predecessor.predecessor_receipts.clear();
    scenarios.push(Scenario {
        scenario_id: "global-missing-predecessor".to_string(),
        case_id: None,
        expected_admitted: false,
        proposal: missing_predecessor,
    });

    let mut stale_capability = make_proposal(
        contract,
        "global-stale-capability",
        "trench-segment-01",
        TaskState::Ready,
        Action::Start,
        1,
        Some("excavator-01".to_string()),
    );
    stale_capability.gates.capability_current = false;
    scenarios.push(Scenario {
        scenario_id: "global-stale-capability".to_string(),
        case_id: None,
        expected_admitted: false,
        proposal: stale_capability,
    });

    let mut capacity_conflict = make_proposal(
        contract,
        "global-capacity-conflict",
        "place-water-pipe-01",
        TaskState::Interrupted,
        Action::Resume,
        2,
        Some("pipe-clamp-a".to_string()),
    );
    capacity_conflict.gates.capacity_admissible = false;
    scenarios.push(Scenario {
        scenario_id: "global-capacity-conflict".to_string(),
        case_id: None,
        expected_admitted: false,
        proposal: capacity_conflict,
    });

    let mut unknown_utility = make_proposal(
        contract,
        "global-unknown-utility",
        "trench-segment-01",
        TaskState::Ready,
        Action::Start,
        0,
        Some("excavator-01".to_string()),
    );
    unknown_utility.gates.utility_envelope = UtilityEnvelope::UnknownOrUnsafe;
    scenarios.push(Scenario {
        scenario_id: "global-unknown-utility".to_string(),
        case_id: None,
        expected_admitted: false,
        proposal: unknown_utility,
    });

    let mut declined_approval = make_proposal(
        contract,
        "global-declined-approval",
        "trench-segment-01",
        TaskState::Ready,
        Action::Start,
        1,
        Some("excavator-01".to_string()),
    );
    declined_approval.gates.authorization = Authorization::Declined;
    scenarios.push(Scenario {
        scenario_id: "global-declined-approval".to_string(),
        case_id: None,
        expected_admitted: false,
        proposal: declined_approval,
    });

    let mut missing_acceptance = make_proposal(
        contract,
        "global-missing-acceptance-evidence",
        "survey-segment-01",
        TaskState::AwaitingEvidence,
        Action::Accept,
        0,
        None,
    );
    missing_acceptance
        .evidence
        .retain(|item| item.evidence_kind != "survey-receipt");
    scenarios.push(Scenario {
        scenario_id: "global-missing-acceptance-evidence".to_string(),
        case_id: None,
        expected_admitted: false,
        proposal: missing_acceptance,
    });

    let mut failed_inspection = make_proposal(
        contract,
        "global-failed-inspection",
        "survey-segment-01",
        TaskState::AwaitingEvidence,
        Action::Accept,
        0,
        None,
    );
    failed_inspection.gates.inspection = InspectionOutcome::Failed;
    scenarios.push(Scenario {
        scenario_id: "global-failed-inspection".to_string(),
        case_id: None,
        expected_admitted: false,
        proposal: failed_inspection,
    });

    let mut unknown_resource = make_proposal(
        contract,
        "global-unknown-resource",
        "survey-segment-01",
        TaskState::Ready,
        Action::Start,
        0,
        Some("unlisted-device".to_string()),
    );
    unknown_resource.predecessor_receipts.clear();
    scenarios.push(Scenario {
        scenario_id: "global-unknown-resource".to_string(),
        case_id: None,
        expected_admitted: false,
        proposal: unknown_resource,
    });

    let mut future_evidence = make_proposal(
        contract,
        "global-future-evidence",
        "survey-segment-01",
        TaskState::AwaitingEvidence,
        Action::Accept,
        0,
        None,
    );
    if let Some(item) = future_evidence.evidence.first_mut() {
        item.observed_at_tick = 1;
    }
    scenarios.push(Scenario {
        scenario_id: "global-future-evidence".to_string(),
        case_id: None,
        expected_admitted: false,
        proposal: future_evidence,
    });

    let mut terminal_restart = make_proposal(
        contract,
        "global-terminal-restart",
        "survey-segment-01",
        TaskState::Accepted,
        Action::Start,
        0,
        Some("survey-rover-01".to_string()),
    );
    terminal_restart.predecessor_receipts.clear();
    scenarios.push(Scenario {
        scenario_id: "global-terminal-restart".to_string(),
        case_id: None,
        expected_admitted: false,
        proposal: terminal_restart,
    });

    let protective_stop = Proposal {
        attempt_id: "global-protective-stop-before-observation".to_string(),
        task_id: "place-water-pipe-01".to_string(),
        state: TaskState::Running,
        action: Action::ProtectiveStop,
        tick: 2,
        resource_id: None,
        previous_resource_id: None,
        observation_ids: Vec::new(),
        predecessor_receipts: Vec::new(),
        evidence: Vec::new(),
        gates: GateSnapshot::safe(),
    };
    scenarios.push(Scenario {
        scenario_id: "global-protective-stop-before-observation".to_string(),
        case_id: Some("R-008".to_string()),
        expected_admitted: true,
        proposal: protective_stop,
    });

    let mut same_resource_reassign = make_proposal(
        contract,
        "global-same-resource-reassign",
        "survey-segment-01",
        TaskState::Interrupted,
        Action::Reassign,
        0,
        Some("survey-rover-01".to_string()),
    );
    same_resource_reassign.previous_resource_id = Some("survey-rover-01".to_string());
    scenarios.push(Scenario {
        scenario_id: "global-same-resource-reassign".to_string(),
        case_id: None,
        expected_admitted: false,
        proposal: same_resource_reassign,
    });

    let duplicate_observation = contract
        .observations
        .get("R-001")
        .ok_or_else(|| "missing R-001".to_string())?;
    let mut duplicate_ref = make_proposal(
        contract,
        "global-duplicate-observation-reference",
        "place-water-pipe-01",
        TaskState::AwaitingEvidence,
        Action::Accept,
        duplicate_observation.observable_tick,
        None,
    );
    duplicate_ref
        .observation_ids
        .push(duplicate_observation.observation_id.clone());
    scenarios.push(Scenario {
        scenario_id: "global-duplicate-observation-reference".to_string(),
        case_id: Some("R-001".to_string()),
        expected_admitted: false,
        proposal: duplicate_ref,
    });

    Ok(scenarios)
}

fn run_contract_suite(contract: &Contract) -> Result<ContractReport, String> {
    let scenarios = build_scenarios(contract)?;
    let scenario_results: Vec<ScenarioResult> = scenarios
        .iter()
        .map(|scenario| {
            let decision = assess(contract, &scenario.proposal);
            ScenarioResult {
                scenario_id: scenario.scenario_id.clone(),
                case_id: scenario.case_id.clone(),
                expected_admitted: scenario.expected_admitted,
                admitted: decision.admitted,
                next_state: decision.next_state,
                reason_codes: decision.reason_codes,
            }
        })
        .collect();
    let admitted_count = scenario_results.iter().filter(|item| item.admitted).count();
    let blocked_count = scenario_results.len() - admitted_count;
    let expectation_mismatch_count = scenario_results
        .iter()
        .filter(|item| item.expected_admitted != item.admitted)
        .count();
    Ok(ContractReport {
        schema_version: "luminous.civ_infra.recovery_admission_contract.v0.1".to_string(),
        fixture_id: contract.fixture_id.clone(),
        fixture_sha256: contract.fixture_sha256.clone(),
        classification: "synthetic_only".to_string(),
        validation_status: "typed_task_admission_contract_checked".to_string(),
        contract_suite_executed: true,
        scenario_count: scenario_results.len(),
        admitted_count,
        blocked_count,
        expectation_mismatch_count,
        all_expected_decisions_match: expectation_mismatch_count == 0,
        policy_simulation_executed: false,
        policy_runs_performed: 0,
        physical_control_executed: false,
        dispatch_authorized: false,
        input_receipts_cryptographically_verified: false,
        immutable_history_store_executed: false,
        gate_inputs_are_synthetic_assertions: true,
        claim_ceiling: "Deterministic synthetic admission semantics only. Gate snapshots and predecessor receipts are synthetic caller-supplied assertions; this binary does not verify cryptographic receipt authenticity or implement durable immutable history. No B0/B1/B2 rollout, physical safety, device control, dispatch authorization, or field-productivity inference.".to_string(),
        scenario_results,
    })
}

fn main() {
    let contract = match build_contract(EMBEDDED_FIXTURE.as_bytes()) {
        Ok(contract) => contract,
        Err(error) => {
            eprintln!("recovery-admission-contract: {error}");
            process::exit(1);
        }
    };
    let report = match run_contract_suite(&contract) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("recovery-admission-contract: {error}");
            process::exit(1);
        }
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("contract report serializes")
    );
    if !report.all_expected_decisions_match {
        process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> Contract {
        build_contract(EMBEDDED_FIXTURE.as_bytes()).expect("canonical fixture should parse")
    }

    fn has_reason(decision: &Decision, prefix: &str) -> bool {
        decision
            .reason_codes
            .iter()
            .any(|reason| reason.starts_with(prefix))
    }

    #[test]
    fn canonical_contract_suite_matches_all_expected_decisions() {
        let report = run_contract_suite(&contract()).expect("suite should run");
        assert!(report.all_expected_decisions_match);
        assert_eq!(report.policy_simulation_executed, false);
        assert_eq!(report.policy_runs_performed, 0);
        assert!(!report.physical_control_executed);
        assert!(!report.dispatch_authorized);
        assert!(!report.input_receipts_cryptographically_verified);
        assert!(!report.immutable_history_store_executed);
        assert!(report.gate_inputs_are_synthetic_assertions);
        assert!(report.scenario_count >= 40);
        for case_id in 1..=13 {
            let id = format!("R-{case_id:03}");
            assert!(report.scenario_results.iter().any(|result| {
                result.case_id.as_deref() == Some(id.as_str())
                    && result.scenario_id.ends_with("accept-after-recovery")
                    && result.admitted
            }));
            assert!(report.scenario_results.iter().any(|result| {
                result.case_id.as_deref() == Some(id.as_str())
                    && result.scenario_id.ends_with("missing-recovery-evidence")
                    && !result.admitted
            }));
        }
    }

    #[test]
    fn all_frozen_state_action_pairs_match_transition_table() {
        let states = [
            TaskState::Pending,
            TaskState::Ready,
            TaskState::Running,
            TaskState::Interrupted,
            TaskState::AwaitingEvidence,
            TaskState::ReworkRequired,
            TaskState::Accepted,
            TaskState::Rejected,
            TaskState::Aborted,
        ];
        let actions = [
            Action::Start,
            Action::ProtectiveStop,
            Action::Resume,
            Action::Reassign,
            Action::SubmitForAcceptance,
            Action::Accept,
            Action::RejectInspection,
            Action::BeginRework,
            Action::Abort,
        ];
        let allowed = [
            (TaskState::Pending, Action::Start, TaskState::Running),
            (TaskState::Ready, Action::Start, TaskState::Running),
            (
                TaskState::Running,
                Action::ProtectiveStop,
                TaskState::Interrupted,
            ),
            (TaskState::Interrupted, Action::Resume, TaskState::Running),
            (
                TaskState::ReworkRequired,
                Action::Resume,
                TaskState::Running,
            ),
            (TaskState::Interrupted, Action::Reassign, TaskState::Running),
            (
                TaskState::ReworkRequired,
                Action::Reassign,
                TaskState::Running,
            ),
            (
                TaskState::Running,
                Action::SubmitForAcceptance,
                TaskState::AwaitingEvidence,
            ),
            (
                TaskState::AwaitingEvidence,
                Action::Accept,
                TaskState::Accepted,
            ),
            (
                TaskState::AwaitingEvidence,
                Action::RejectInspection,
                TaskState::Rejected,
            ),
            (
                TaskState::Rejected,
                Action::BeginRework,
                TaskState::ReworkRequired,
            ),
            (TaskState::Pending, Action::Abort, TaskState::Aborted),
            (TaskState::Ready, Action::Abort, TaskState::Aborted),
            (TaskState::Running, Action::Abort, TaskState::Aborted),
            (TaskState::Interrupted, Action::Abort, TaskState::Aborted),
            (
                TaskState::AwaitingEvidence,
                Action::Abort,
                TaskState::Aborted,
            ),
            (TaskState::ReworkRequired, Action::Abort, TaskState::Aborted),
            (TaskState::Rejected, Action::Abort, TaskState::Aborted),
        ];
        for state in states {
            for action in actions {
                let expected = allowed
                    .iter()
                    .find(|(known_state, known_action, _)| {
                        *known_state == state && *known_action == action
                    })
                    .map(|(_, _, target)| *target);
                assert_eq!(
                    transition_target(state, action),
                    expected,
                    "unexpected transition for {state:?} + {action:?}"
                );
            }
        }
    }

    #[test]
    fn unknown_utility_never_defaults_to_clear() {
        let mut proposal = make_proposal(
            &contract(),
            "test-unknown-utility",
            "trench-segment-01",
            TaskState::Ready,
            Action::Start,
            0,
            Some("excavator-01".to_string()),
        );
        proposal.gates.utility_envelope = UtilityEnvelope::UnknownOrUnsafe;
        let decision = assess(&contract(), &proposal);
        assert!(!decision.admitted);
        assert!(has_reason(&decision, "unknown_utility_is_not_clear"));
    }

    #[test]
    fn declined_human_approval_cannot_be_overridden_by_replanning() {
        let mut proposal = make_proposal(
            &contract(),
            "test-declined-approval",
            "trench-segment-01",
            TaskState::Ready,
            Action::Start,
            1,
            Some("excavator-01".to_string()),
        );
        proposal.gates.authorization = Authorization::Declined;
        let decision = assess(&contract(), &proposal);
        assert!(!decision.admitted);
        assert!(has_reason(&decision, "authorization_not_granted"));
    }

    #[test]
    fn stale_capability_and_calibration_each_fail_closed() {
        let contract = contract();
        let mut stale_capability = make_proposal(
            &contract,
            "test-stale-capability",
            "trench-segment-01",
            TaskState::Ready,
            Action::Start,
            1,
            Some("excavator-01".to_string()),
        );
        stale_capability.gates.capability_current = false;
        assert!(has_reason(
            &assess(&contract, &stale_capability),
            "capability_profile_stale"
        ));

        let mut stale_calibration = make_proposal(
            &contract,
            "test-stale-calibration",
            "place-water-pipe-01",
            TaskState::Interrupted,
            Action::Resume,
            2,
            Some("utility-cart-01".to_string()),
        );
        stale_calibration.gates.calibration_current = false;
        assert!(has_reason(
            &assess(&contract, &stale_calibration),
            "calibration_stale"
        ));
    }

    #[test]
    fn task_cannot_start_without_accepted_predecessor_receipts() {
        let contract = contract();
        let mut proposal = make_proposal(
            &contract,
            "test-missing-predecessor",
            "trench-segment-01",
            TaskState::Ready,
            Action::Start,
            0,
            Some("excavator-01".to_string()),
        );
        proposal.predecessor_receipts.clear();
        let decision = assess(&contract, &proposal);
        assert!(!decision.admitted);
        assert!(has_reason(&decision, "predecessor_receipt_missing"));
    }

    #[test]
    fn accepted_predecessor_without_its_required_evidence_is_not_trusted() {
        let contract = contract();
        let mut proposal = make_proposal(
            &contract,
            "test-predecessor-without-evidence",
            "trench-segment-01",
            TaskState::Ready,
            Action::Start,
            0,
            Some("excavator-01".to_string()),
        );
        proposal.predecessor_receipts[0].evidence.clear();
        let decision = assess(&contract, &proposal);
        assert!(!decision.admitted);
        assert!(has_reason(
            &decision,
            "predecessor_acceptance_evidence_missing_or_stale"
        ));
    }

    #[test]
    fn failed_inspection_cannot_be_converted_into_acceptance() {
        let contract = contract();
        let mut proposal = make_proposal(
            &contract,
            "test-failed-inspection",
            "survey-segment-01",
            TaskState::AwaitingEvidence,
            Action::Accept,
            0,
            None,
        );
        proposal.gates.inspection = InspectionOutcome::Failed;
        let decision = assess(&contract, &proposal);
        assert!(!decision.admitted);
        assert!(has_reason(
            &decision,
            "acceptance_requires_successful_inspection"
        ));
    }

    #[test]
    fn protective_stop_does_not_wait_for_planner_observation() {
        let proposal = Proposal {
            attempt_id: "test-stop-before-observation".to_string(),
            task_id: "place-water-pipe-01".to_string(),
            state: TaskState::Running,
            action: Action::ProtectiveStop,
            tick: 2,
            resource_id: None,
            previous_resource_id: None,
            observation_ids: Vec::new(),
            predecessor_receipts: Vec::new(),
            evidence: Vec::new(),
            gates: GateSnapshot::safe(),
        };
        let decision = assess(&contract(), &proposal);
        assert!(decision.admitted, "{:?}", decision.reason_codes);
        assert_eq!(decision.next_state, Some(TaskState::Interrupted));
    }

    #[test]
    fn failed_inspection_rework_can_begin_before_postwork_receipts_exist() {
        let contract = contract();
        let mut begin = make_proposal(
            &contract,
            "test-begin-rework",
            "place-water-pipe-01",
            TaskState::Rejected,
            Action::BeginRework,
            7,
            None,
        );
        begin.gates.inspection = InspectionOutcome::Failed;
        begin.evidence.retain(|item| {
            !matches!(
                item.evidence_kind.as_str(),
                "corrective-work-receipt" | "new-as-built-observation" | "successful-reinspection"
            )
        });
        let begin_decision = assess(&contract, &begin);
        assert!(begin_decision.admitted, "{:?}", begin_decision.reason_codes);

        let mut resume = make_proposal(
            &contract,
            "test-resume-rework",
            "place-water-pipe-01",
            TaskState::ReworkRequired,
            Action::Resume,
            8,
            Some("utility-cart-01".to_string()),
        );
        resume.evidence.retain(|item| {
            !matches!(
                item.evidence_kind.as_str(),
                "corrective-work-receipt" | "new-as-built-observation" | "successful-reinspection"
            )
        });
        let decision = assess(&contract, &resume);
        assert!(decision.admitted, "{:?}", decision.reason_codes);
        assert_eq!(decision.next_state, Some(TaskState::Running));
    }

    #[test]
    fn final_acceptance_after_rework_requires_successful_reinspection() {
        let contract = contract();
        let mut proposal = make_proposal(
            &contract,
            "test-rework-acceptance",
            "place-water-pipe-01",
            TaskState::AwaitingEvidence,
            Action::Accept,
            8,
            None,
        );
        proposal
            .evidence
            .retain(|item| item.evidence_kind != "successful-reinspection");
        let decision = assess(&contract, &proposal);
        assert!(!decision.admitted);
        assert!(has_reason(
            &decision,
            "recovery_evidence_missing_or_stale:R-007:successful-reinspection"
        ));
    }

    #[test]
    fn future_dated_evidence_cannot_satisfy_acceptance() {
        let contract = contract();
        let mut proposal = make_proposal(
            &contract,
            "test-future-evidence",
            "survey-segment-01",
            TaskState::AwaitingEvidence,
            Action::Accept,
            0,
            None,
        );
        proposal.evidence[0].observed_at_tick = 1;
        let decision = assess(&contract, &proposal);
        assert!(!decision.admitted);
        assert!(has_reason(&decision, "future_dated_evidence"));
    }

    #[test]
    fn omitted_current_observation_blocks_execution() {
        let contract = contract();
        let mut proposal = make_proposal(
            &contract,
            "test-omitted-visible-event",
            "trench-segment-01",
            TaskState::Ready,
            Action::Start,
            1,
            Some("excavator-01".to_string()),
        );
        proposal.observation_ids.retain(|id| id != "R-013");
        let decision = assess(&contract, &proposal);
        assert!(!decision.admitted);
        assert!(has_reason(&decision, "visible_observation_omitted:R-013"));
    }

    #[test]
    fn reassignment_must_choose_a_different_declared_resource() {
        let contract = contract();
        let mut proposal = make_proposal(
            &contract,
            "test-same-resource",
            "survey-segment-01",
            TaskState::Interrupted,
            Action::Reassign,
            0,
            Some("survey-rover-01".to_string()),
        );
        proposal.previous_resource_id = Some("survey-rover-01".to_string());
        let decision = assess(&contract, &proposal);
        assert!(!decision.admitted);
        assert!(has_reason(
            &decision,
            "reassignment_did_not_change_resource"
        ));
    }

    #[test]
    fn unknown_evidence_kind_fails_closed() {
        let contract = contract();
        let mut proposal = make_proposal(
            &contract,
            "test-unknown-evidence",
            "survey-segment-01",
            TaskState::AwaitingEvidence,
            Action::Accept,
            0,
            None,
        );
        proposal.evidence.push(Evidence {
            evidence_id: "forged-kind".to_string(),
            evidence_kind: "planner-says-safe".to_string(),
            observed_at_tick: 0,
            current: true,
            valid: true,
        });
        let decision = assess(&contract, &proposal);
        assert!(!decision.admitted);
        assert!(has_reason(
            &decision,
            "unknown_evidence_kind:planner-says-safe"
        ));
    }

    #[test]
    fn duplicate_observation_refs_are_rejected() {
        let contract = contract();
        let mut proposal = make_proposal(
            &contract,
            "test-duplicate-observation",
            "place-water-pipe-01",
            TaskState::AwaitingEvidence,
            Action::Accept,
            2,
            None,
        );
        proposal.observation_ids.push("R-001".to_string());
        let decision = assess(&contract, &proposal);
        assert!(!decision.admitted);
        assert!(has_reason(
            &decision,
            "duplicate_observation_reference:R-001"
        ));
    }

    #[test]
    fn accepted_task_is_terminal_and_cannot_restart() {
        let contract = contract();
        let proposal = make_proposal(
            &contract,
            "test-terminal-restart",
            "survey-segment-01",
            TaskState::Accepted,
            Action::Start,
            0,
            Some("survey-rover-01".to_string()),
        );
        let decision = assess(&contract, &proposal);
        assert!(!decision.admitted);
        assert!(has_reason(&decision, "invalid_state_transition"));
    }

    #[test]
    fn one_thousand_seeded_gate_perturbations_fail_closed_and_replay_deterministically() {
        let contract = contract();
        let baseline = make_proposal(
            &contract,
            "property-baseline",
            "trench-segment-01",
            TaskState::Ready,
            Action::Start,
            1,
            Some("excavator-01".to_string()),
        );
        let baseline_decision = assess(&contract, &baseline);
        assert!(
            baseline_decision.admitted,
            "the controlled baseline must be admissible: {:?}",
            baseline_decision.reason_codes
        );

        let mut state = 0xC1F1_1A5E_2026_u64;
        let mut covered = BTreeSet::new();
        for index in 0..1_000 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let gate = state % 9;
            let mut proposal = baseline.clone();
            proposal.attempt_id = format!("gate-property-{index:04}");
            match gate {
                0 => {
                    proposal.gates.resource_available = false;
                    covered.insert("resource_available");
                }
                1 => {
                    proposal.gates.capacity_admissible = false;
                    covered.insert("capacity_admissible");
                }
                2 => {
                    proposal.gates.capability_current = false;
                    covered.insert("capability_current");
                }
                3 => {
                    proposal.gates.calibration_current = false;
                    covered.insert("calibration_current");
                }
                4 => {
                    proposal.gates.localization_admissible = false;
                    covered.insert("localization_admissible");
                }
                5 => {
                    proposal.gates.communication_current = false;
                    covered.insert("communication_current");
                }
                6 => {
                    proposal.gates.energy_reserve_feasible = false;
                    covered.insert("energy_reserve_feasible");
                }
                7 => {
                    proposal.gates.utility_envelope = UtilityEnvelope::UnknownOrUnsafe;
                    covered.insert("utility_envelope");
                }
                _ => {
                    proposal.gates.authorization = Authorization::Declined;
                    covered.insert("authorization");
                }
            }

            let first = assess(&contract, &proposal);
            let replay = assess(&contract, &proposal);
            assert!(
                !first.admitted,
                "unsafe gate mutation {gate} admitted at index {index}"
            );
            assert_eq!(first.admitted, replay.admitted);
            assert_eq!(first.next_state, replay.next_state);
            assert_eq!(first.reason_codes, replay.reason_codes);
            assert!(first.reason_codes.len() > 0);
            assert!(first.next_state.is_none());
        }
        assert_eq!(covered.len(), 9, "all nine gates must be exercised");
    }

    #[test]
    fn fixture_rejects_observation_before_injection_without_exposing_injection_tick() {
        let mut fixture: Value =
            serde_json::from_slice(EMBEDDED_FIXTURE.as_bytes()).expect("fixture should parse");
        fixture["disturbance_cases"][7]["injection_tick"] = serde_json::json!(5);
        let altered = serde_json::to_vec(&fixture).expect("altered fixture should serialize");
        assert!(build_contract(&altered).is_err());

        let contract = contract();
        let visible = serde_json::to_string(&contract.observations["R-008"])
            .expect("policy observation should serialize");
        assert!(!visible.contains("injection_tick"));
    }

    #[test]
    fn report_digest_binds_the_exact_embedded_fixture_bytes() {
        let contract = contract();
        assert_eq!(
            contract.fixture_sha256,
            sha256_hex(EMBEDDED_FIXTURE.as_bytes())
        );
    }
}
