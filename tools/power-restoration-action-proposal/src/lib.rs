use power_restoration_benchmark_case::{BenchmarkCase, canonical_case_sha256};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlannerIdentity {
    pub planner_id: String,
    pub planner_revision: String,
    pub source_commit: String,
    pub configuration_sha256: String,
    pub prompt_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DecisionObservation {
    pub observation_ref: String,
    pub observed_step: u32,
    pub observation_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceReference {
    pub evidence_ref: String,
    pub observed_step: u32,
    pub evidence_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Action {
    pub action_id: String,
    pub actor_kind: ActorKind,
    pub actor_id: String,
    pub action_kind: ActionKind,
    pub target_id: Option<String>,
    pub desired_switch_state: Option<SwitchState>,
    pub decision_step: u32,
    pub observation_ref: String,
    pub precondition_evidence: Vec<EvidenceReference>,
    pub authority_requirement_ref: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    RepairCrew,
    MobilePowerSource,
    GridOperator,
    System,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    MoveCrew,
    RepairFault,
    MoveMobileSource,
    ConnectMobileSource,
    DisconnectMobileSource,
    SetSwitchState,
    Wait,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SwitchState {
    Open,
    Closed,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActionProposal {
    pub schema_version: String,
    pub proposal_id: String,
    pub case_ref: String,
    pub case_sha256: String,
    pub planner: PlannerIdentity,
    pub observation: DecisionObservation,
    pub decision_step: u32,
    pub actions: Vec<Action>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProposalValidationReport {
    pub proposal_id: String,
    pub case_sha256: String,
    pub observation_ref: String,
    pub decision_step: u32,
    pub action_count: usize,
    pub result_status: String,
    pub claim_boundary: String,
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn actor_exists(case: &BenchmarkCase, kind: ActorKind, actor_id: &str) -> bool {
    match kind {
        ActorKind::RepairCrew => case.repair_crews.iter().any(|crew| crew.crew_id == actor_id),
        ActorKind::MobilePowerSource => case.mobile_power_sources.iter().any(|source| source.mobile_source_id == actor_id),
        ActorKind::GridOperator | ActorKind::System => valid_id(actor_id),
    }
}

fn validate_action(action: &Action, case: &BenchmarkCase, proposal: &ActionProposal, errors: &mut Vec<String>) {
    if !valid_id(&action.action_id)
        || !valid_id(&action.actor_id)
        || !valid_id(&action.observation_ref)
        || !valid_id(&action.authority_requirement_ref)
    {
        errors.push(format!("action {} is missing identity, observation or authority reference", action.action_id));
    }
    if !actor_exists(case, action.actor_kind, &action.actor_id) {
        errors.push(format!("action {} references an unknown actor {}", action.action_id, action.actor_id));
    }
    if action.decision_step != proposal.decision_step
        || action.observation_ref != proposal.observation.observation_ref
    {
        errors.push(format!("action {} does not reference the proposal decision snapshot", action.action_id));
    }
    if action.decision_step >= case.timebase.horizon_steps {
        errors.push(format!("action {} decision step is outside the case horizon", action.action_id));
    }
    if action.precondition_evidence.is_empty() {
        errors.push(format!("action {} has no precondition evidence references", action.action_id));
    }
    let mut evidence_ids = BTreeSet::new();
    for evidence in &action.precondition_evidence {
        if !valid_id(&evidence.evidence_ref)
            || !is_lower_hex(&evidence.evidence_sha256, 64)
            || !evidence_ids.insert(evidence.evidence_ref.as_str())
        {
            errors.push(format!("action {} has invalid or duplicate precondition evidence", action.action_id));
        }
        if evidence.observed_step > action.decision_step {
            errors.push(format!("action {} uses evidence from a future step", action.action_id));
        }
    }

    let target = action.target_id.as_deref();
    match action.action_kind {
        ActionKind::MoveCrew => {
            if action.actor_kind != ActorKind::RepairCrew
                || target.is_none_or(|id| !case.road_network.nodes.iter().any(|node| node.node_id == id))
                || action.desired_switch_state.is_some()
            {
                errors.push(format!("action {} has an invalid move_crew actor or road-node target", action.action_id));
            }
        }
        ActionKind::RepairFault => {
            let crew = case.repair_crews.iter().find(|crew| crew.crew_id == action.actor_id);
            let fault = target.and_then(|id| case.faults.iter().find(|fault| fault.fault_id == id));
            if action.actor_kind != ActorKind::RepairCrew
                || fault.is_none()
                || crew.is_none_or(|crew| fault.is_none_or(|fault| !crew.skill_refs.contains(&fault.required_skill_ref)))
                || action.desired_switch_state.is_some()
            {
                errors.push(format!("action {} has an invalid repair actor, fault target or required skill", action.action_id));
            }
        }
        ActionKind::MoveMobileSource => {
            if action.actor_kind != ActorKind::MobilePowerSource
                || target.is_none_or(|id| !case.road_network.nodes.iter().any(|node| node.node_id == id))
                || action.desired_switch_state.is_some()
            {
                errors.push(format!("action {} has an invalid mobile-source move actor or road-node target", action.action_id));
            }
        }
        ActionKind::ConnectMobileSource | ActionKind::DisconnectMobileSource => {
            let mobile = case.mobile_power_sources.iter().find(|source| source.mobile_source_id == action.actor_id);
            if action.actor_kind != ActorKind::MobilePowerSource
                || target.is_none_or(|id| !case.network.nodes.iter().any(|node| node.node_id == id))
                || mobile.is_none_or(|source| target.is_none_or(|id| !source.permitted_connection_node_ids.iter().any(|allowed| allowed == id)))
                || action.desired_switch_state.is_some()
            {
                errors.push(format!("action {} has an invalid mobile connection actor or unpermitted power node", action.action_id));
            }
        }
        ActionKind::SetSwitchState => {
            if action.actor_kind != ActorKind::GridOperator
                || target.is_none_or(|id| !case.network.switches.iter().any(|switch| switch.switch_id == id))
                || action.desired_switch_state.is_none()
            {
                errors.push(format!("action {} must set a known switch to an explicit state using a grid-operator actor", action.action_id));
            }
        }
        ActionKind::Wait => {
            if action.actor_kind != ActorKind::System
                || target.is_some()
                || action.desired_switch_state.is_some()
            {
                errors.push(format!("action {} wait action must use system actor and no target/state", action.action_id));
            }
        }
    }
}

pub fn validate_proposal(
    proposal: &ActionProposal,
    case: &BenchmarkCase,
    expected_case_sha256: &str,
) -> Result<ProposalValidationReport, Vec<String>> {
    let mut errors = Vec::new();
    let actual_case_sha256 = canonical_case_sha256(case)
        .map_err(|error| vec![format!("case canonicalization failed: {error}")])?;
    if expected_case_sha256 != actual_case_sha256
        || proposal.case_sha256 != expected_case_sha256
        || !is_lower_hex(&proposal.case_sha256, 64)
    {
        errors.push("proposal case digest does not match the frozen case digest".to_string());
    }
    if proposal.schema_version != "0.1.0"
        || !valid_id(&proposal.proposal_id)
        || !valid_id(&proposal.case_ref)
    {
        errors.push("proposal schema, proposal ID or case reference is invalid".to_string());
    }
    let planner = &proposal.planner;
    if !valid_id(&planner.planner_id)
        || !valid_id(&planner.planner_revision)
        || !is_lower_hex(&planner.source_commit, 40)
        || !is_lower_hex(&planner.configuration_sha256, 64)
        || planner.prompt_sha256.as_ref().is_some_and(|digest| !is_lower_hex(digest, 64))
    {
        errors.push("planner identity, source commit or configuration/prompt digest is invalid".to_string());
    }
    let observation = &proposal.observation;
    if !valid_id(&observation.observation_ref)
        || !is_lower_hex(&observation.observation_sha256, 64)
        || observation.observed_step > proposal.decision_step
        || proposal.decision_step >= case.timebase.horizon_steps
    {
        errors.push("observation identity/digest or proposal decision step is invalid".to_string());
    }
    if proposal.actions.is_empty() {
        errors.push("proposal must contain at least one action".to_string());
    }
    if !proposal.actions.iter().all(|action| action.decision_step == proposal.decision_step) {
        errors.push("all actions in one proposal must share the same decision step".to_string());
    }
    let mut action_ids = BTreeSet::new();
    for action in &proposal.actions {
        if !action_ids.insert(action.action_id.as_str()) {
            errors.push(format!("duplicate action ID {}", action.action_id));
        }
        validate_action(action, case, proposal, &mut errors);
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(ProposalValidationReport {
        proposal_id: proposal.proposal_id.clone(),
        case_sha256: actual_case_sha256,
        observation_ref: proposal.observation.observation_ref.clone(),
        decision_step: proposal.decision_step,
        action_count: proposal.actions.len(),
        result_status: "proposal_contract_valid".to_string(),
        claim_boundary: "Proposal shape/reference integrity only; not executable or physically feasible".to_string(),
    })
}

pub fn parse_and_validate(
    proposal_json: &str,
    case_json: &str,
) -> Result<ProposalValidationReport, Vec<String>> {
    let proposal: ActionProposal = serde_json::from_str(proposal_json)
        .map_err(|error| vec![format!("proposal JSON decode error: {error}")])?;
    let case: BenchmarkCase = serde_json::from_str(case_json)
        .map_err(|error| vec![format!("case JSON decode error: {error}")])?;
    let digest = canonical_case_sha256(&case)
        .map_err(|error| vec![format!("case canonicalization failed: {error}")])?;
    validate_proposal(&proposal, &case, &digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case() -> BenchmarkCase {
        serde_json::from_str(include_str!("../../power-restoration-benchmark-case/fixtures/synthetic-restoration-case-v0.1.json"))
            .expect("case fixture must deserialize")
    }

    fn proposal() -> ActionProposal {
        let case = case();
        let digest = canonical_case_sha256(&case).unwrap();
        let proposal: ActionProposal = serde_json::from_str(include_str!("../fixtures/synthetic-restoration-action-proposal-v0.1.json"))
            .expect("proposal fixture must deserialize");
        let mut proposal = proposal;
        proposal.case_sha256 = digest;
        proposal
    }

    fn evidence(step: u32) -> EvidenceReference {
        EvidenceReference {
            evidence_ref: format!("synthetic://evidence/step-{step}"),
            observed_step: step,
            evidence_sha256: "a".repeat(64),
        }
    }

    fn single_action(kind: ActionKind, actor_kind: ActorKind, actor_id: &str, target: Option<&str>) -> ActionProposal {
        let mut proposal = proposal();
        proposal.actions = vec![Action {
            action_id: "action-1".to_string(),
            actor_kind,
            actor_id: actor_id.to_string(),
            action_kind: kind,
            target_id: target.map(str::to_string),
            desired_switch_state: None,
            decision_step: proposal.decision_step,
            observation_ref: proposal.observation.observation_ref.clone(),
            precondition_evidence: vec![evidence(proposal.decision_step)],
            authority_requirement_ref: "synthetic://authority/requirement-1".to_string(),
        }];
        proposal
    }

    #[test]
    fn baseline_repair_proposal_is_structurally_valid() {
        let case = case();
        let proposal = proposal();
        let digest = canonical_case_sha256(&case).unwrap();
        assert_eq!(validate_proposal(&proposal, &case, &digest).unwrap().result_status, "proposal_contract_valid");
    }

    #[test]
    fn case_digest_mismatch_is_rejected() {
        let case = case();
        let mut proposal = proposal();
        proposal.case_sha256 = "0".repeat(64);
        let digest = canonical_case_sha256(&case).unwrap();
        assert!(validate_proposal(&proposal, &case, &digest).is_err());
    }

    #[test]
    fn unknown_crew_is_rejected() {
        let case = case();
        let proposal = single_action(ActionKind::MoveCrew, ActorKind::RepairCrew, "crew-unknown", Some("R1"));
        let digest = canonical_case_sha256(&case).unwrap();
        assert!(validate_proposal(&proposal, &case, &digest).is_err());
    }

    #[test]
    fn unknown_fault_is_rejected() {
        let case = case();
        let proposal = single_action(ActionKind::RepairFault, ActorKind::RepairCrew, "CREW-A", Some("FAULT-UNKNOWN"));
        let digest = canonical_case_sha256(&case).unwrap();
        assert!(validate_proposal(&proposal, &case, &digest).is_err());
    }

    #[test]
    fn repair_skill_mismatch_is_rejected() {
        let mut case = case();
        case.repair_crews[0].skill_refs.clear();
        let proposal = single_action(ActionKind::RepairFault, ActorKind::RepairCrew, "CREW-A", Some("F23-A"));
        let digest = canonical_case_sha256(&case).unwrap();
        assert!(validate_proposal(&proposal, &case, &digest).is_err());
    }

    #[test]
    fn crew_move_requires_known_road_node() {
        let case = case();
        let proposal = single_action(ActionKind::MoveCrew, ActorKind::RepairCrew, "CREW-A", Some("R999"));
        let digest = canonical_case_sha256(&case).unwrap();
        assert!(validate_proposal(&proposal, &case, &digest).is_err());
    }

    #[test]
    fn mobile_move_requires_known_road_node() {
        let case = case();
        let proposal = single_action(ActionKind::MoveMobileSource, ActorKind::MobilePowerSource, "MPS-1", Some("R999"));
        let digest = canonical_case_sha256(&case).unwrap();
        assert!(validate_proposal(&proposal, &case, &digest).is_err());
    }

    #[test]
    fn mobile_connection_must_be_permitted() {
        let case = case();
        let proposal = single_action(ActionKind::ConnectMobileSource, ActorKind::MobilePowerSource, "MPS-1", Some("N2"));
        let digest = canonical_case_sha256(&case).unwrap();
        assert!(validate_proposal(&proposal, &case, &digest).is_err());
    }

    #[test]
    fn permitted_mobile_connection_is_structurally_valid() {
        let case = case();
        let proposal = single_action(ActionKind::ConnectMobileSource, ActorKind::MobilePowerSource, "MPS-1", Some("N5"));
        let digest = canonical_case_sha256(&case).unwrap();
        assert!(validate_proposal(&proposal, &case, &digest).is_ok());
    }

    #[test]
    fn switch_state_requires_explicit_idempotent_target_state() {
        let case = case();
        let mut proposal = single_action(ActionKind::SetSwitchState, ActorKind::GridOperator, "operator-1", Some("T56"));
        proposal.actions[0].desired_switch_state = Some(SwitchState::Closed);
        let digest = canonical_case_sha256(&case).unwrap();
        assert!(validate_proposal(&proposal, &case, &digest).is_ok());
        proposal.actions[0].desired_switch_state = None;
        assert!(validate_proposal(&proposal, &case, &digest).is_err());
    }

    #[test]
    fn toggle_is_not_in_the_action_vocabulary() {
        let input = r#"{"schemaVersion":"0.1.0","proposalId":"p","caseRef":"c","caseSha256":"0000000000000000000000000000000000000000000000000000000000000000","planner":{"plannerId":"p","plannerRevision":"1","sourceCommit":"0000000000000000000000000000000000000000","configurationSha256":"0000000000000000000000000000000000000000000000000000000000000000","promptSha256":null},"observation":{"observationRef":"o","observedStep":0,"observationSha256":"0000000000000000000000000000000000000000000000000000000000000000"},"decisionStep":0,"actions":[{"actionId":"a","actorKind":"grid_operator","actorId":"op","actionKind":"toggle_switch","targetId":"T56","desiredSwitchState":null,"decisionStep":0,"observationRef":"o","preconditionEvidence":[{"evidenceRef":"e","observedStep":0,"evidenceSha256":"0000000000000000000000000000000000000000000000000000000000000000"}],"authorityRequirementRef":"auth"}]}"#;
        assert!(serde_json::from_str::<ActionProposal>(input).is_err());
    }

    #[test]
    fn wait_must_not_have_target_or_switch_state() {
        let case = case();
        let mut proposal = single_action(ActionKind::Wait, ActorKind::System, "system", None);
        proposal.actions[0].target_id = Some("N1".to_string());
        let digest = canonical_case_sha256(&case).unwrap();
        assert!(validate_proposal(&proposal, &case, &digest).is_err());
    }

    #[test]
    fn future_precondition_evidence_is_rejected() {
        let case = case();
        let mut proposal = proposal();
        proposal.actions[0].precondition_evidence = vec![evidence(proposal.decision_step + 1)];
        let digest = canonical_case_sha256(&case).unwrap();
        assert!(validate_proposal(&proposal, &case, &digest).is_err());
    }

    #[test]
    fn action_must_reference_exact_observation_snapshot() {
        let case = case();
        let mut proposal = proposal();
        proposal.actions[0].observation_ref = "other-snapshot".to_string();
        let digest = canonical_case_sha256(&case).unwrap();
        assert!(validate_proposal(&proposal, &case, &digest).is_err());
    }

    #[test]
    fn action_must_use_proposal_decision_step() {
        let case = case();
        let mut proposal = proposal();
        proposal.actions[0].decision_step += 1;
        let digest = canonical_case_sha256(&case).unwrap();
        assert!(validate_proposal(&proposal, &case, &digest).is_err());
    }

    #[test]
    fn duplicate_action_ids_are_rejected() {
        let case = case();
        let mut proposal = proposal();
        proposal.actions.push(proposal.actions[0].clone());
        let digest = canonical_case_sha256(&case).unwrap();
        assert!(validate_proposal(&proposal, &case, &digest).is_err());
    }

    #[test]
    fn empty_authority_reference_is_rejected() {
        let case = case();
        let mut proposal = proposal();
        proposal.actions[0].authority_requirement_ref.clear();
        let digest = canonical_case_sha256(&case).unwrap();
        assert!(validate_proposal(&proposal, &case, &digest).is_err());
    }

    #[test]
    fn missing_precondition_evidence_is_rejected() {
        let case = case();
        let mut proposal = proposal();
        proposal.actions[0].precondition_evidence.clear();
        let digest = canonical_case_sha256(&case).unwrap();
        assert!(validate_proposal(&proposal, &case, &digest).is_err());
    }

    #[test]
    fn invalid_planner_commit_and_configuration_hash_are_rejected() {
        let case = case();
        let mut proposal = proposal();
        proposal.planner.source_commit = "bad".to_string();
        proposal.planner.configuration_sha256 = "bad".to_string();
        let digest = canonical_case_sha256(&case).unwrap();
        assert!(validate_proposal(&proposal, &case, &digest).is_err());
    }

    #[test]
    fn duplicate_evidence_references_are_rejected() {
        let case = case();
        let mut proposal = proposal();
        proposal.actions[0].precondition_evidence.push(proposal.actions[0].precondition_evidence[0].clone());
        let digest = canonical_case_sha256(&case).unwrap();
        assert!(validate_proposal(&proposal, &case, &digest).is_err());
    }

    #[test]
    fn deterministic_proposal_receipt() {
        let case = case();
        let proposal = proposal();
        let digest = canonical_case_sha256(&case).unwrap();
        assert_eq!(
            serde_json::to_string(&validate_proposal(&proposal, &case, &digest).unwrap()).unwrap(),
            serde_json::to_string(&validate_proposal(&proposal, &case, &digest).unwrap()).unwrap()
        );
    }
}
