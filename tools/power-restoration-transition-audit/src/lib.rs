use power_restoration_action_proposal::{
    ActionKind, ActionProposal, SwitchState, validate_proposal,
};
use power_restoration_benchmark_case::{
    BenchmarkCase, canonical_case_sha256, validate_case,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestorationState {
    pub schema_version: String,
    pub case_sha256: String,
    pub step: u32,
    pub switch_states: BTreeMap<String, SwitchState>,
    pub crew_road_positions: BTreeMap<String, String>,
    pub mobile_source_road_positions: BTreeMap<String, String>,
    pub mobile_source_connections: BTreeMap<String, Option<String>>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionExecutionOutcome {
    Applied,
    Rejected,
    Failed,
    NotAttempted,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActionExecution {
    pub action_id: String,
    pub outcome: ActionExecutionOutcome,
    pub result_receipt_ref: Option<String>,
    pub result_receipt_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransitionEnvelope {
    pub schema_version: String,
    pub transition_id: String,
    pub case_ref: String,
    pub case_sha256: String,
    pub proposal_id: String,
    pub proposal_sha256: String,
    pub decision_step: u32,
    pub previous_state_sha256: String,
    pub successor_state_sha256: String,
    pub executions: Vec<ActionExecution>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransitionAuditReport {
    pub transition_id: String,
    pub case_sha256: String,
    pub proposal_sha256: String,
    pub previous_state_sha256: String,
    pub successor_state_sha256: String,
    pub supported_action_count: usize,
    pub applied_action_count: usize,
    pub rejected_action_count: usize,
    pub failed_action_count: usize,
    pub not_attempted_action_count: usize,
    pub result_status: String,
    pub unsupported_action_ids: Vec<String>,
    pub claim_boundary: String,
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub fn canonical_sha256<T: Serialize>(value: &T) -> Result<String, String> {
    let value = serde_json::to_value(value)
        .map_err(|error| format!("JSON serialization failed: {error}"))?;
    let bytes = serde_json::to_vec(&value)
        .map_err(|error| format!("canonical JSON serialization failed: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub fn initial_state(case: &BenchmarkCase) -> Result<RestorationState, Vec<String>> {
    let errors = validate_case(case);
    if !errors.is_empty() {
        return Err(errors);
    }
    let case_sha256 = canonical_case_sha256(case)
        .map_err(|error| vec![format!("case canonicalization failed: {error}")])?;
    Ok(RestorationState {
        schema_version: "0.1.0".to_string(),
        case_sha256,
        step: 0,
        switch_states: case.network.switches.iter()
            .map(|switch| (
                switch.switch_id.clone(),
                if switch.initially_closed { SwitchState::Closed } else { SwitchState::Open },
            ))
            .collect(),
        crew_road_positions: case.repair_crews.iter()
            .map(|crew| (crew.crew_id.clone(), crew.starting_road_node_id.clone()))
            .collect(),
        mobile_source_road_positions: case.mobile_power_sources.iter()
            .map(|source| (source.mobile_source_id.clone(), source.starting_road_node_id.clone()))
            .collect(),
        mobile_source_connections: case.mobile_power_sources.iter()
            .map(|source| (source.mobile_source_id.clone(), None))
            .collect(),
    })
}

fn validate_state_shape(
    state: &RestorationState,
    case: &BenchmarkCase,
    expected_step: u32,
) -> Vec<String> {
    let mut errors = Vec::new();
    let case_sha256 = match canonical_case_sha256(case) {
        Ok(value) => value,
        Err(error) => {
            errors.push(format!("case canonicalization failed: {error}"));
            String::new()
        }
    };
    if state.schema_version != "0.1.0"
        || state.case_sha256 != case_sha256
        || !is_lower_hex(&state.case_sha256, 64)
        || state.step != expected_step
    {
        errors.push("state version, case digest or decision step is invalid".to_string());
    }

    let expected_switches: BTreeSet<&str> = case.network.switches.iter()
        .map(|switch| switch.switch_id.as_str())
        .collect();
    let actual_switches: BTreeSet<&str> = state.switch_states.keys()
        .map(String::as_str)
        .collect();
    if expected_switches != actual_switches {
        errors.push("state switch inventory does not exactly match the frozen case".to_string());
    }

    let expected_crews: BTreeSet<&str> = case.repair_crews.iter()
        .map(|crew| crew.crew_id.as_str())
        .collect();
    let actual_crews: BTreeSet<&str> = state.crew_road_positions.keys()
        .map(String::as_str)
        .collect();
    if expected_crews != actual_crews {
        errors.push("state repair-crew inventory does not exactly match the frozen case".to_string());
    }

    let road_node_ids: BTreeSet<&str> = case.road_network.nodes.iter()
        .map(|node| node.node_id.as_str())
        .collect();
    for (crew_id, road_node_id) in &state.crew_road_positions {
        if !expected_crews.contains(crew_id.as_str()) || !road_node_ids.contains(road_node_id.as_str()) {
            errors.push(format!("crew {crew_id} has an unknown identity or road position"));
        }
    }

    let expected_mobile: BTreeSet<&str> = case.mobile_power_sources.iter()
        .map(|source| source.mobile_source_id.as_str())
        .collect();
    let actual_mobile: BTreeSet<&str> = state.mobile_source_road_positions.keys()
        .map(String::as_str)
        .collect();
    let actual_connections: BTreeSet<&str> = state.mobile_source_connections.keys()
        .map(String::as_str)
        .collect();
    if expected_mobile != actual_mobile || expected_mobile != actual_connections {
        errors.push("state mobile-source inventory does not exactly match the frozen case".to_string());
    }
    for (source_id, road_node_id) in &state.mobile_source_road_positions {
        if !expected_mobile.contains(source_id.as_str()) || !road_node_ids.contains(road_node_id.as_str()) {
            errors.push(format!("mobile source {source_id} has an unknown identity or road position"));
        }
    }
    let power_node_ids: BTreeSet<&str> = case.network.nodes.iter()
        .map(|node| node.node_id.as_str())
        .collect();
    for (source_id, connection) in &state.mobile_source_connections {
        if let Some(node_id) = connection {
            let permitted = case.mobile_power_sources.iter()
                .find(|source| source.mobile_source_id == *source_id)
                .is_some_and(|source| {
                    source.permitted_connection_node_ids.iter().any(|candidate| candidate == node_id)
                });
            let at_node = state.mobile_source_road_positions
                .get(source_id)
                .and_then(|road_position| {
                    case.road_network.nodes.iter()
                        .find(|road_node| road_node.node_id == *road_position)
                })
                .is_some_and(|road_node| road_node.power_node_id == *node_id);
            if !power_node_ids.contains(node_id.as_str()) || !permitted || !at_node {
                errors.push(format!(
                    "mobile source {source_id} is connected to an unknown/unpermitted node or is not at that connection point {node_id}"
                ));
            }
        }
    }
    errors
}

fn apply_supported_action(
    action: &power_restoration_action_proposal::Action,
    execution: &ActionExecution,
    case: &BenchmarkCase,
    next: &mut RestorationState,
) -> Result<bool, String> {
    if execution.outcome != ActionExecutionOutcome::Applied {
        return Ok(true);
    }
    match action.action_kind {
        ActionKind::SetSwitchState => {
            let switch_id = action.target_id.as_deref()
                .ok_or_else(|| "set_switch_state has no target".to_string())?;
            let desired = action.desired_switch_state
                .ok_or_else(|| "set_switch_state has no explicit desired state".to_string())?;
            let state = next.switch_states.get_mut(switch_id)
                .ok_or_else(|| format!("set_switch_state references unknown switch {switch_id}"))?;
            *state = desired;
            Ok(true)
        }
        ActionKind::ConnectMobileSource => {
            let source_id = &action.actor_id;
            let node_id = action.target_id.as_deref()
                .ok_or_else(|| "connect_mobile_source has no target".to_string())?;
            let source = case.mobile_power_sources.iter()
                .find(|source| source.mobile_source_id == *source_id)
                .ok_or_else(|| format!("unknown mobile source {source_id}"))?;
            if !source.permitted_connection_node_ids.iter().any(|allowed| allowed == node_id) {
                return Err(format!("mobile source {source_id} is not permitted at {node_id}"));
            }
            let road_position = next.mobile_source_road_positions.get(source_id)
                .ok_or_else(|| format!("missing road position for mobile source {source_id}"))?;
            let road_node = case.road_network.nodes.iter()
                .find(|node| node.node_id == *road_position)
                .ok_or_else(|| format!("unknown road node {road_position}"))?;
            if road_node.power_node_id != node_id {
                return Err(format!(
                    "mobile source {source_id} is at road node {road_position}, not at power node {node_id}"
                ));
            }
            let connection = next.mobile_source_connections.get_mut(source_id)
                .ok_or_else(|| format!("missing connection state for mobile source {source_id}"))?;
            if connection.as_deref().is_some_and(|current| current != node_id) {
                return Err(format!(
                    "mobile source {source_id} must disconnect from its current node before connecting elsewhere"
                ));
            }
            *connection = Some(node_id.to_string());
            Ok(true)
        }
        ActionKind::DisconnectMobileSource => {
            let source_id = &action.actor_id;
            let node_id = action.target_id.as_deref()
                .ok_or_else(|| "disconnect_mobile_source has no target".to_string())?;
            let connection = next.mobile_source_connections.get_mut(source_id)
                .ok_or_else(|| format!("missing connection state for mobile source {source_id}"))?;
            if connection.as_deref() != Some(node_id) {
                return Err(format!(
                    "mobile source {source_id} is not currently connected to {node_id}"
                ));
            }
            *connection = None;
            Ok(true)
        }
        ActionKind::MoveCrew | ActionKind::MoveMobileSource | ActionKind::RepairFault => {
            Ok(false)
        },
        ActionKind::ObserveScada
        | ActionKind::ObserveCalls
        | ActionKind::ObserveNetworkModel
        | ActionKind::CrewInspect
        | ActionKind::ReadFaultIndicator
        | ActionKind::TestSwitch
        | ActionKind::SimulateSwitch
        | ActionKind::ForecastLoad
        | ActionKind::EscalateToHuman
        | ActionKind::Wait => Ok(true),
    }
}

pub fn audit_transition(
    case: &BenchmarkCase,
    proposal: &ActionProposal,
    previous_state: &RestorationState,
    successor_state: &RestorationState,
    envelope: &TransitionEnvelope,
) -> Result<TransitionAuditReport, Vec<String>> {
    let mut errors = Vec::new();
    let case_sha256 = canonical_case_sha256(case)
        .map_err(|error| vec![format!("case canonicalization failed: {error}")])?;
    let proposal_sha256 = canonical_sha256(proposal)
        .map_err(|error| vec![format!("proposal canonicalization failed: {error}")])?;
    if envelope.schema_version != "0.1.0"
        || !valid_id(&envelope.transition_id)
        || envelope.case_ref != format!("power-restoration-case://{}", case.case_id)
        || envelope.case_sha256 != case_sha256
        || envelope.proposal_id != proposal.proposal_id
        || envelope.proposal_sha256 != proposal_sha256
        || envelope.decision_step != proposal.decision_step
        || !is_lower_hex(&envelope.previous_state_sha256, 64)
        || !is_lower_hex(&envelope.successor_state_sha256, 64)
    {
        errors.push("transition envelope identity or case/proposal binding is invalid".to_string());
    }
    if let Err(proposal_errors) = validate_proposal(proposal, case, &case_sha256) {
        errors.extend(proposal_errors.into_iter().map(|error| format!("proposal: {error}")));
    }
    errors.extend(validate_state_shape(previous_state, case, envelope.decision_step));
    errors.extend(validate_state_shape(successor_state, case, envelope.decision_step.saturating_add(1)));

    let previous_digest = canonical_sha256(previous_state)
        .map_err(|error| vec![format!("previous state canonicalization failed: {error}")])?;
    let successor_digest = canonical_sha256(successor_state)
        .map_err(|error| vec![format!("successor state canonicalization failed: {error}")])?;
    if envelope.previous_state_sha256 != previous_digest
        || envelope.successor_state_sha256 != successor_digest
    {
        errors.push("transition envelope state digest does not match supplied state bytes".to_string());
    }

    let mut executions = BTreeMap::new();
    for execution in &envelope.executions {
        if !valid_id(&execution.action_id)
            || executions.insert(execution.action_id.as_str(), execution).is_some()
        {
            errors.push(format!("transition has invalid or duplicate execution record {}", execution.action_id));
        }
        match (execution.result_receipt_ref.as_deref(), execution.result_receipt_sha256.as_deref()) {
            (Some(reference), Some(digest)) if valid_id(reference) && is_lower_hex(digest, 64) => {}
            (None, None) => {}
            _ => errors.push(format!(
                "action {} result receipt reference/digest must be present together and well-formed",
                execution.action_id
            )),
        }
    }

    if executions.len() != proposal.actions.len() {
        errors.push("transition must contain exactly one execution record for every proposed action".to_string());
    }
    for action in &proposal.actions {
        if !executions.contains_key(action.action_id.as_str()) {
            errors.push(format!("transition is missing execution result for action {}", action.action_id));
        }
    }
    for action_id in executions.keys() {
        if !proposal.actions.iter().any(|action| action.action_id == *action_id) {
            errors.push(format!("transition contains an execution for unknown action {action_id}"));
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut expected_successor = previous_state.clone();
    expected_successor.step = expected_successor.step.saturating_add(1);
    let mut unsupported_action_ids = Vec::new();
    let mut supported_action_count = 0;
    let mut applied_action_count = 0;
    let mut rejected_action_count = 0;
    let mut failed_action_count = 0;
    let mut not_attempted_action_count = 0;

    for action in &proposal.actions {
        let execution = executions.get(action.action_id.as_str()).expect("execution set was checked");
        if execution.outcome == ActionExecutionOutcome::Failed {
            // Failure may have partially mutated equipment or work progress.
            // The current state model cannot represent that uncertainty.
            unsupported_action_ids.push(action.action_id.clone());
            continue;
        }
        match execution.outcome {
            ActionExecutionOutcome::Applied => applied_action_count += 1,
            ActionExecutionOutcome::Rejected => rejected_action_count += 1,
            ActionExecutionOutcome::Failed => failed_action_count += 1,
            ActionExecutionOutcome::NotAttempted => not_attempted_action_count += 1,
        }
        match apply_supported_action(action, execution, case, &mut expected_successor) {
            Ok(true) => supported_action_count += 1,
            Ok(false) => unsupported_action_ids.push(action.action_id.clone()),
            Err(error) => errors.push(format!("action {}: {error}", action.action_id)),
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    if !unsupported_action_ids.is_empty() {
        return Ok(TransitionAuditReport {
            transition_id: envelope.transition_id.clone(),
            case_sha256,
            proposal_sha256,
            previous_state_sha256: previous_digest,
            successor_state_sha256: successor_digest,
            supported_action_count,
            applied_action_count,
            rejected_action_count,
            failed_action_count,
            not_attempted_action_count,
            result_status: "inconclusive_unsupported_action_semantics".to_string(),
            unsupported_action_ids,
            claim_boundary: "Only supported discrete state deltas were replayed. At least one applied crew-movement, mobile-source-movement or repair action requires a specialist transition model; no state-transition correctness or physical feasibility claim is made.".to_string(),
        });
    }

    if expected_successor != *successor_state {
        return Err(vec![
            "successor state contains an unrequested mutation or omits an action-derived state change".to_string()
        ]);
    }

    Ok(TransitionAuditReport {
        transition_id: envelope.transition_id.clone(),
        case_sha256,
        proposal_sha256,
        previous_state_sha256: previous_digest,
        successor_state_sha256: successor_digest,
        supported_action_count,
        applied_action_count,
        rejected_action_count,
        failed_action_count,
        not_attempted_action_count,
        result_status: "supported_transition_state_delta_verified".to_string(),
        unsupported_action_ids,
        claim_boundary: "Deterministic state delta for explicit switch-state, mobile-source connection and non-mutating observation actions only. This is not radiality, AC/three-phase feasibility, protection, authority, commissioning, delivered-service or field-safety qualification.".to_string(),
    })
}

pub fn parse_and_audit(
    case_json: &str,
    proposal_json: &str,
    previous_state_json: &str,
    successor_state_json: &str,
    envelope_json: &str,
) -> Result<TransitionAuditReport, Vec<String>> {
    let case: BenchmarkCase = serde_json::from_str(case_json)
        .map_err(|error| vec![format!("case JSON decode error: {error}")])?;
    let proposal: ActionProposal = serde_json::from_str(proposal_json)
        .map_err(|error| vec![format!("proposal JSON decode error: {error}")])?;
    let previous: RestorationState = serde_json::from_str(previous_state_json)
        .map_err(|error| vec![format!("previous state JSON decode error: {error}")])?;
    let successor: RestorationState = serde_json::from_str(successor_state_json)
        .map_err(|error| vec![format!("successor state JSON decode error: {error}")])?;
    let envelope: TransitionEnvelope = serde_json::from_str(envelope_json)
        .map_err(|error| vec![format!("transition envelope JSON decode error: {error}")])?;
    audit_transition(&case, &proposal, &previous, &successor, &envelope)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case() -> BenchmarkCase {
        serde_json::from_str(include_str!(
            "../../power-restoration-benchmark-case/fixtures/synthetic-restoration-case-v0.1.json"
        )).unwrap()
    }

    fn proposal(kind: ActionKind) -> ActionProposal {
        let case = case();
        let mut proposal: ActionProposal = serde_json::from_str(include_str!(
            "../../power-restoration-action-proposal/fixtures/synthetic-restoration-action-proposal-v0.1.json"
        )).unwrap();
        let action = &mut proposal.actions[0];
        action.action_kind = kind;
        action.actor_id = match kind {
            ActionKind::SetSwitchState => "GRID-OP-1".to_string(),
            ActionKind::ConnectMobileSource | ActionKind::DisconnectMobileSource => "MPS-1".to_string(),
            ActionKind::RepairFault => "CREW-A".to_string(),
            _ => "system".to_string(),
        };
        action.actor_kind = match kind {
            ActionKind::SetSwitchState => power_restoration_action_proposal::ActorKind::GridOperator,
            ActionKind::ConnectMobileSource | ActionKind::DisconnectMobileSource => power_restoration_action_proposal::ActorKind::MobilePowerSource,
            ActionKind::RepairFault | ActionKind::MoveCrew => power_restoration_action_proposal::ActorKind::RepairCrew,
            _ => power_restoration_action_proposal::ActorKind::System,
        };
        action.target_id = match kind {
            ActionKind::SetSwitchState => Some("T56".to_string()),
            ActionKind::ConnectMobileSource | ActionKind::DisconnectMobileSource => Some("N5".to_string()),
            ActionKind::ObserveScada => Some("N3".to_string()),
            ActionKind::RepairFault => Some("F23-A".to_string()),
            _ => None,
        };
        action.desired_switch_state = if kind == ActionKind::SetSwitchState {
            Some(SwitchState::Closed)
        } else {
            None
        };
        if kind == ActionKind::ObserveScada {
            action.precondition_evidence.clear();
        } else {
            action.precondition_evidence = vec![
                power_restoration_action_proposal::EvidenceReference {
                    evidence_ref: "synthetic://evidence/test".to_string(),
                    observed_step: 0,
                    evidence_sha256: "a".repeat(64),
                }
            ];
        }
        proposal
    }

    fn state(case: &BenchmarkCase) -> RestorationState {
        initial_state(case).unwrap()
    }

    fn envelope(
        case: &BenchmarkCase,
        proposal: &ActionProposal,
        before: &RestorationState,
        after: &RestorationState,
        outcome: ActionExecutionOutcome,
    ) -> TransitionEnvelope {
        TransitionEnvelope {
            schema_version: "0.1.0".to_string(),
            transition_id: "transition-0001".to_string(),
            case_ref: format!("power-restoration-case://{}", case.case_id),
            case_sha256: canonical_case_sha256(case).unwrap(),
            proposal_id: proposal.proposal_id.clone(),
            proposal_sha256: canonical_sha256(proposal).unwrap(),
            decision_step: proposal.decision_step,
            previous_state_sha256: canonical_sha256(before).unwrap(),
            successor_state_sha256: canonical_sha256(after).unwrap(),
            executions: proposal.actions.iter().map(|action| ActionExecution {
                action_id: action.action_id.clone(),
                outcome,
                result_receipt_ref: None,
                result_receipt_sha256: None,
            }).collect(),
        }
    }

    #[test]
    fn explicit_switch_state_change_is_recomputed() {
        let case = case();
        let proposal = proposal(ActionKind::SetSwitchState);
        let before = state(&case);
        let mut after = before.clone();
        after.step = 1;
        after.switch_states.insert("T56".to_string(), SwitchState::Closed);
        let envelope = envelope(&case, &proposal, &before, &after, ActionExecutionOutcome::Applied);
        let report = audit_transition(&case, &proposal, &before, &after, &envelope).unwrap();
        assert_eq!(report.result_status, "supported_transition_state_delta_verified");
        assert_eq!(report.applied_action_count, 1);
    }

    #[test]
    fn unrelated_successor_mutation_is_rejected() {
        let case = case();
        let proposal = proposal(ActionKind::SetSwitchState);
        let before = state(&case);
        let mut after = before.clone();
        after.step = 1;
        after.switch_states.insert("T56".to_string(), SwitchState::Closed);
        after.mobile_source_connections.insert("MPS-1".to_string(), Some("N5".to_string()));
        let envelope = envelope(&case, &proposal, &before, &after, ActionExecutionOutcome::Applied);
        assert!(audit_transition(&case, &proposal, &before, &after, &envelope).is_err());
    }

    #[test]
    fn mobile_source_connection_must_match_its_current_road_position() {
        let case = case();
        let mut before = state(&case);
        before.mobile_source_connections.insert("MPS-1".to_string(), Some("N5".to_string()));
        before.mobile_source_road_positions.insert("MPS-1".to_string(), "R1".to_string());
        let proposal = proposal(ActionKind::ObserveScada);
        let after = before.clone();
        let envelope = envelope(&case, &proposal, &before, &after, ActionExecutionOutcome::Applied);
        assert!(audit_transition(&case, &proposal, &before, &after, &envelope).is_err());
    }

    #[test]
    fn missing_action_execution_is_rejected() {
        let case = case();
        let proposal = proposal(ActionKind::SetSwitchState);
        let before = state(&case);
        let mut after = before.clone();
        after.step = 1;
        let mut envelope = envelope(&case, &proposal, &before, &after, ActionExecutionOutcome::Rejected);
        envelope.executions.clear();
        assert!(audit_transition(&case, &proposal, &before, &after, &envelope).is_err());
    }

    #[test]
    fn duplicate_action_execution_is_rejected() {
        let case = case();
        let proposal = proposal(ActionKind::SetSwitchState);
        let before = state(&case);
        let mut after = before.clone();
        after.step = 1;
        let mut envelope = envelope(&case, &proposal, &before, &after, ActionExecutionOutcome::Rejected);
        envelope.executions.push(envelope.executions[0].clone());
        assert!(audit_transition(&case, &proposal, &before, &after, &envelope).is_err());
    }

    #[test]
    fn switch_rejection_must_leave_state_unchanged() {
        let case = case();
        let proposal = proposal(ActionKind::SetSwitchState);
        let before = state(&case);
        let mut after = before.clone();
        after.step = 1;
        let envelope = envelope(&case, &proposal, &before, &after, ActionExecutionOutcome::Rejected);
        assert_eq!(
            audit_transition(&case, &proposal, &before, &after, &envelope).unwrap().result_status,
            "supported_transition_state_delta_verified"
        );
    }

    #[test]
    fn failed_action_is_inconclusive_when_partial_effect_cannot_be_excluded() {
        let case = case();
        let proposal = proposal(ActionKind::SetSwitchState);
        let before = state(&case);
        let mut after = before.clone();
        after.step = 1;
        let envelope = envelope(&case, &proposal, &before, &after, ActionExecutionOutcome::Failed);
        let report = audit_transition(&case, &proposal, &before, &after, &envelope).unwrap();
        assert_eq!(report.result_status, "inconclusive_unsupported_action_semantics");
        assert_eq!(report.unsupported_action_ids, vec!["repair-action-0001".to_string()]);
    }

    #[test]
    fn repair_execution_is_inconclusive_until_work_semantics_exist() {
        let case = case();
        let proposal = proposal(ActionKind::RepairFault);
        let before = state(&case);
        let mut after = before.clone();
        after.step = 1;
        let envelope = envelope(&case, &proposal, &before, &after, ActionExecutionOutcome::Applied);
        let report = audit_transition(&case, &proposal, &before, &after, &envelope).unwrap();
        assert_eq!(report.result_status, "inconclusive_unsupported_action_semantics");
        assert_eq!(report.unsupported_action_ids, vec!["repair-action-0001".to_string()]);
    }

    #[test]
    fn observation_request_is_non_mutating() {
        let case = case();
        let proposal = proposal(ActionKind::ObserveScada);
        let before = state(&case);
        let mut after = before.clone();
        after.step = 1;
        let envelope = envelope(&case, &proposal, &before, &after, ActionExecutionOutcome::Applied);
        assert_eq!(
            audit_transition(&case, &proposal, &before, &after, &envelope).unwrap().result_status,
            "supported_transition_state_delta_verified"
        );
    }

    #[test]
    fn state_digest_mismatch_is_rejected() {
        let case = case();
        let proposal = proposal(ActionKind::SetSwitchState);
        let before = state(&case);
        let mut after = before.clone();
        after.step = 1;
        let mut envelope = envelope(&case, &proposal, &before, &after, ActionExecutionOutcome::Rejected);
        envelope.successor_state_sha256 = "0".repeat(64);
        assert!(audit_transition(&case, &proposal, &before, &after, &envelope).is_err());
    }
}
