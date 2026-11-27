//! Static scenario-robust, evidence-bounded preservation portfolio reference.
//! No probabilities are inferred; no plan is authorized or executed; alerts are
//! never cleared by this crate. A robust result is conditional on its frozen
//! scenario catalog and externally qualified bounds.

use std::collections::{BTreeMap, BTreeSet, HashMap};

pub const MAX_CANDIDATE_ACTIONS: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceStatus { Verified, Unverified, Invalid }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Disposition { NoActionRequired, Feasible, Infeasible, EvidenceIncomplete, InvalidEvidence }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExclusionReason {
    MissingScenarioBounds, ScenarioBoundsUnverified, ScenarioBoundsInvalid,
    MissingPrerequisiteEvidence, MissingApprovalAuthority, DestructiveAction, SourceNotPreserved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractError {
    InvalidScenarioSetId, InvalidScenarioSetDigest, EmptyScenarioCatalog,
    InvalidScenarioId, DuplicateScenarioId, InvalidTargetId, InvalidCurrencyCode,
    InvalidAlertId, DuplicateAlertId, InvalidEvidenceId, DuplicateEvidenceId,
    DuplicatePrerequisiteEvidence, InvalidApprovalId, DuplicateApprovalId, DuplicateActionApproval,
    TooManyCandidates, InvalidActionId,
    DuplicateActionId, CurrencyMismatch, EmptyRequiredApprovals, EmptyPostconditions,
    InvalidPostconditionId, DuplicatePostcondition, DuplicateCoverage, DuplicateDependency,
    UnknownAlert, UnknownDependency, SelfDependency, DuplicateScenarioBound,
    UnknownScenarioBound, InvalidReceiptId, InvalidReceiptDigest, DependencyCycle,
    ArithmeticOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioCatalog {
    pub scenario_set_id: String,
    pub scenario_set_sha256: String,
    pub scenario_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    pub target_id: String,
    pub unresolved_alert_ids: Vec<String>,
    pub available_evidence_ids: Vec<String>,
    pub available_approval_ids: Vec<String>,
    pub currency_code: String,
    pub max_cost_microunits: u64,
    pub max_energy_millijoules: u64,
    pub max_duration_ms: u64,
    pub max_downtime_ms: u64,
    pub max_actions: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioBound {
    pub scenario_id: String,
    pub cost_upper_microunits: u64,
    pub energy_upper_millijoules: u64,
    pub duration_upper_ms: u64,
    pub downtime_upper_ms: u64,
    pub evidence_status: EvidenceStatus,
    pub receipt_id: String,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionCandidate {
    pub action_id: String,
    pub addresses_alert_ids: Vec<String>,
    pub prerequisite_evidence_ids: Vec<String>,
    pub depends_on_action_ids: Vec<String>,
    pub currency_code: String,
    pub scenario_bounds: Vec<ScenarioBound>,
    pub required_approvals: Vec<String>,
    pub required_postcondition_evidence: Vec<String>,
    pub preserves_source_until_acceptance: bool,
    pub destructive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExcludedCandidate {
    pub action_id: String,
    pub reasons: Vec<ExclusionReason>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Objectives {
    pub worst_case_cost_microunits: u64,
    pub worst_case_energy_millijoules: u64,
    pub worst_case_duration_ms: u64,
    pub worst_case_downtime_ms: u64,
    pub action_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimitingScenarios {
    pub cost: Vec<String>,
    pub energy: Vec<String>,
    pub duration: Vec<String>,
    pub downtime: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundReceiptRef {
    pub action_id: String,
    pub scenario_id: String,
    pub receipt_id: String,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub scenario_set_id: String,
    pub scenario_set_sha256: String,
    pub action_ids: Vec<String>,
    pub addressed_alert_ids: Vec<String>,
    pub required_prerequisite_evidence_ids: Vec<String>,
    pub required_approvals: Vec<String>,
    pub required_postcondition_evidence: Vec<String>,
    pub bound_receipts: Vec<BoundReceiptRef>,
    pub objectives: Objectives,
    pub limiting_scenarios: LimitingScenarios,
    pub auto_execute: bool,
    pub preserves_source_until_acceptance: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub target_id: String,
    pub scenario_set_id: String,
    pub scenario_set_sha256: String,
    pub disposition: Disposition,
    pub pareto_frontier: Vec<Plan>,
    pub excluded_candidates: Vec<ExcludedCandidate>,
    pub enumerated_subsets: u64,
}

#[derive(Debug, Clone, Copy, Default)]
struct Totals { cost: u128, energy: u128, duration: u128, downtime: u128 }

impl Totals {
    fn add(&mut self, bound: &ScenarioBound) -> Result<(), ContractError> {
        self.cost = self.cost.checked_add(u128::from(bound.cost_upper_microunits)).ok_or(ContractError::ArithmeticOverflow)?;
        self.energy = self.energy.checked_add(u128::from(bound.energy_upper_millijoules)).ok_or(ContractError::ArithmeticOverflow)?;
        self.duration = self.duration.checked_add(u128::from(bound.duration_upper_ms)).ok_or(ContractError::ArithmeticOverflow)?;
        self.downtime = self.downtime.checked_add(u128::from(bound.downtime_upper_ms)).ok_or(ContractError::ArithmeticOverflow)?;
        Ok(())
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && value.bytes().all(|b| {
        b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-')
    })
}
fn valid_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn valid_currency(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|b| b.is_ascii_uppercase())
}
fn duplicate<T: PartialEq>(values: &[T]) -> bool {
    values.iter().enumerate().any(|(i, value)| values[..i].contains(value))
}
fn to_u64(value: u128) -> Result<u64, ContractError> {
    u64::try_from(value).map_err(|_| ContractError::ArithmeticOverflow)
}

fn visit(
    index: usize,
    actions: &[ActionCandidate],
    by_id: &HashMap<String, usize>,
    state: &mut [u8],
) -> Result<(), ContractError> {
    match state[index] {
        1 => return Err(ContractError::DependencyCycle),
        2 => return Ok(()),
        _ => {}
    }
    state[index] = 1;
    for id in &actions[index].depends_on_action_ids {
        let dep = *by_id.get(id).ok_or(ContractError::UnknownDependency)?;
        visit(dep, actions, by_id, state)?;
    }
    state[index] = 2;
    Ok(())
}

fn validate(
    policy: &Policy,
    catalog: &ScenarioCatalog,
    actions: &[ActionCandidate],
) -> Result<(), ContractError> {
    if !valid_id(&catalog.scenario_set_id) { return Err(ContractError::InvalidScenarioSetId); }
    if !valid_hash(&catalog.scenario_set_sha256) { return Err(ContractError::InvalidScenarioSetDigest); }
    if catalog.scenario_ids.is_empty() { return Err(ContractError::EmptyScenarioCatalog); }
    if catalog.scenario_ids.iter().any(|id| !valid_id(id)) { return Err(ContractError::InvalidScenarioId); }
    if duplicate(&catalog.scenario_ids) { return Err(ContractError::DuplicateScenarioId); }
    if !valid_id(&policy.target_id) { return Err(ContractError::InvalidTargetId); }
    if !valid_currency(&policy.currency_code) { return Err(ContractError::InvalidCurrencyCode); }
    if policy.unresolved_alert_ids.iter().any(|id| !valid_id(id)) { return Err(ContractError::InvalidAlertId); }
    if duplicate(&policy.unresolved_alert_ids) { return Err(ContractError::DuplicateAlertId); }
    if policy.available_evidence_ids.iter().any(|id| !valid_id(id)) { return Err(ContractError::InvalidEvidenceId); }
    if duplicate(&policy.available_evidence_ids) { return Err(ContractError::DuplicateEvidenceId); }
    if policy.available_approval_ids.iter().any(|id| !valid_id(id)) { return Err(ContractError::InvalidApprovalId); }
    if duplicate(&policy.available_approval_ids) { return Err(ContractError::DuplicateApprovalId); }
    if actions.len() > MAX_CANDIDATE_ACTIONS { return Err(ContractError::TooManyCandidates); }

    let mut by_id = HashMap::new();
    for (index, action) in actions.iter().enumerate() {
        if !valid_id(&action.action_id) { return Err(ContractError::InvalidActionId); }
        if by_id.insert(action.action_id.clone(), index).is_some() { return Err(ContractError::DuplicateActionId); }
        if action.currency_code != policy.currency_code { return Err(ContractError::CurrencyMismatch); }
        if action.addresses_alert_ids.iter().any(|id| !valid_id(id)) { return Err(ContractError::InvalidAlertId); }
        if duplicate(&action.addresses_alert_ids) { return Err(ContractError::DuplicateCoverage); }
        if action.prerequisite_evidence_ids.iter().any(|id| !valid_id(id)) { return Err(ContractError::InvalidEvidenceId); }
        if duplicate(&action.prerequisite_evidence_ids) { return Err(ContractError::DuplicatePrerequisiteEvidence); }
        if action.required_approvals.is_empty() { return Err(ContractError::EmptyRequiredApprovals); }
        if action.required_approvals.iter().any(|id| !valid_id(id)) { return Err(ContractError::InvalidApprovalId); }
        if duplicate(&action.required_approvals) { return Err(ContractError::DuplicateActionApproval); }
        if action.required_postcondition_evidence.is_empty() { return Err(ContractError::EmptyPostconditions); }
        if action.required_postcondition_evidence.iter().any(|id| !valid_id(id)) { return Err(ContractError::InvalidPostconditionId); }
        if duplicate(&action.required_postcondition_evidence) { return Err(ContractError::DuplicatePostcondition); }
        if duplicate(&action.depends_on_action_ids) { return Err(ContractError::DuplicateDependency); }
        for dependency in &action.depends_on_action_ids {
            if dependency == &action.action_id { return Err(ContractError::SelfDependency); }
        }
        let mut seen = BTreeSet::new();
        for bound in &action.scenario_bounds {
            if !catalog.scenario_ids.contains(&bound.scenario_id) { return Err(ContractError::UnknownScenarioBound); }
            if !seen.insert(bound.scenario_id.clone()) { return Err(ContractError::DuplicateScenarioBound); }
            if !valid_id(&bound.receipt_id) { return Err(ContractError::InvalidReceiptId); }
            if !valid_hash(&bound.receipt_sha256) { return Err(ContractError::InvalidReceiptDigest); }
        }
    }
    let alerts: BTreeSet<&str> = policy.unresolved_alert_ids.iter().map(String::as_str).collect();
    for action in actions {
        if action.addresses_alert_ids.iter().any(|id| !alerts.contains(id.as_str())) {
            return Err(ContractError::UnknownAlert);
        }
        for dependency in &action.depends_on_action_ids {
            if !by_id.contains_key(dependency) { return Err(ContractError::UnknownDependency); }
        }
    }
    let mut state = vec![0; actions.len()];
    for i in 0..actions.len() { visit(i, actions, &by_id, &mut state)?; }
    Ok(())
}

fn reasons(policy: &Policy, catalog: &ScenarioCatalog, action: &ActionCandidate) -> Vec<ExclusionReason> {
    let mut out = BTreeSet::new();
    if action.scenario_bounds.len() != catalog.scenario_ids.len()
        || catalog.scenario_ids.iter().any(|id| !action.scenario_bounds.iter().any(|b| &b.scenario_id == id)) {
        out.insert(ExclusionReason::MissingScenarioBounds);
    }
    for bound in &action.scenario_bounds {
        match bound.evidence_status {
            EvidenceStatus::Verified => {},
            EvidenceStatus::Unverified => { out.insert(ExclusionReason::ScenarioBoundsUnverified); },
            EvidenceStatus::Invalid => { out.insert(ExclusionReason::ScenarioBoundsInvalid); },
        }
    }
    if action.prerequisite_evidence_ids.iter().any(|id| !policy.available_evidence_ids.contains(id)) {
        out.insert(ExclusionReason::MissingPrerequisiteEvidence);
    }
    if action.required_approvals.iter().any(|id| !policy.available_approval_ids.contains(id)) {
        out.insert(ExclusionReason::MissingApprovalAuthority);
    }
    if action.destructive { out.insert(ExclusionReason::DestructiveAction); }
    if !action.preserves_source_until_acceptance { out.insert(ExclusionReason::SourceNotPreserved); }
    out.into_iter().collect()
}

fn dominates(a: &Plan, b: &Plan) -> bool {
    let x = &a.objectives;
    let y = &b.objectives;
    let no_worse = x.worst_case_cost_microunits <= y.worst_case_cost_microunits
        && x.worst_case_energy_millijoules <= y.worst_case_energy_millijoules
        && x.worst_case_duration_ms <= y.worst_case_duration_ms
        && x.worst_case_downtime_ms <= y.worst_case_downtime_ms
        && x.action_count <= y.action_count;
    let better = x.worst_case_cost_microunits < y.worst_case_cost_microunits
        || x.worst_case_energy_millijoules < y.worst_case_energy_millijoules
        || x.worst_case_duration_ms < y.worst_case_duration_ms
        || x.worst_case_downtime_ms < y.worst_case_downtime_ms
        || x.action_count < y.action_count;
    no_worse && better
}

fn plan_order(a: &Plan, b: &Plan) -> std::cmp::Ordering {
    let x = &a.objectives;
    let y = &b.objectives;
    x.worst_case_cost_microunits.cmp(&y.worst_case_cost_microunits)
        .then_with(|| x.worst_case_energy_millijoules.cmp(&y.worst_case_energy_millijoules))
        .then_with(|| x.worst_case_duration_ms.cmp(&y.worst_case_duration_ms))
        .then_with(|| x.worst_case_downtime_ms.cmp(&y.worst_case_downtime_ms))
        .then_with(|| x.action_count.cmp(&y.action_count))
        .then_with(|| a.action_ids.cmp(&b.action_ids))
}

fn make_plan(
    policy: &Policy,
    catalog: &ScenarioCatalog,
    selected: &[&ActionCandidate],
) -> Result<Option<Plan>, ContractError> {
    let mut by_scenario: BTreeMap<String, Totals> = BTreeMap::new();
    for scenario in &catalog.scenario_ids {
        let mut total = Totals::default();
        for action in selected {
            let bound = action.scenario_bounds.iter().find(|b| &b.scenario_id == scenario)
                .ok_or(ContractError::ArithmeticOverflow)?;
            total.add(bound)?;
        }
        if total.cost > u128::from(policy.max_cost_microunits)
            || total.energy > u128::from(policy.max_energy_millijoules)
            || total.duration > u128::from(policy.max_duration_ms)
            || total.downtime > u128::from(policy.max_downtime_ms) {
            return Ok(None);
        }
        by_scenario.insert(scenario.clone(), total);
    }
    let worst = Totals {
        cost: by_scenario.values().map(|t| t.cost).max().unwrap_or(0),
        energy: by_scenario.values().map(|t| t.energy).max().unwrap_or(0),
        duration: by_scenario.values().map(|t| t.duration).max().unwrap_or(0),
        downtime: by_scenario.values().map(|t| t.downtime).max().unwrap_or(0),
    };
    let limiting = |field: fn(&Totals) -> u128, value: u128| -> Vec<String> {
        by_scenario.iter().filter(|(_, totals)| field(totals) == value)
            .map(|(id, _)| id.clone()).collect()
    };

    let mut ordered = selected.to_vec();
    ordered.sort_by(|a, b| a.action_id.cmp(&b.action_id));
    let mut action_ids = Vec::new();
    let mut alerts = BTreeSet::new();
    let mut prereqs = BTreeSet::new();
    let mut approvals = BTreeSet::new();
    let mut postconditions = BTreeSet::new();
    let mut bound_receipts = Vec::new();
    for action in &ordered {
        action_ids.push(action.action_id.clone());
        alerts.extend(action.addresses_alert_ids.iter().cloned());
        prereqs.extend(action.prerequisite_evidence_ids.iter().cloned());
        approvals.extend(action.required_approvals.iter().cloned());
        postconditions.extend(action.required_postcondition_evidence.iter().cloned());
        for bound in &action.scenario_bounds {
            bound_receipts.push(BoundReceiptRef {
                action_id: action.action_id.clone(), scenario_id: bound.scenario_id.clone(),
                receipt_id: bound.receipt_id.clone(), receipt_sha256: bound.receipt_sha256.clone(),
            });
        }
    }
    bound_receipts.sort_by(|a, b| a.action_id.cmp(&b.action_id).then_with(|| a.scenario_id.cmp(&b.scenario_id)));
    Ok(Some(Plan {
        scenario_set_id: catalog.scenario_set_id.clone(),
        scenario_set_sha256: catalog.scenario_set_sha256.clone(),
        action_ids,
        addressed_alert_ids: alerts.into_iter().collect(),
        required_prerequisite_evidence_ids: prereqs.into_iter().collect(),
        required_approvals: approvals.into_iter().collect(),
        required_postcondition_evidence: postconditions.into_iter().collect(),
        bound_receipts,
        objectives: Objectives {
            worst_case_cost_microunits: to_u64(worst.cost)?,
            worst_case_energy_millijoules: to_u64(worst.energy)?,
            worst_case_duration_ms: to_u64(worst.duration)?,
            worst_case_downtime_ms: to_u64(worst.downtime)?,
            action_count: selected.len(),
        },
        limiting_scenarios: LimitingScenarios {
            cost: limiting(|t| t.cost, worst.cost),
            energy: limiting(|t| t.energy, worst.energy),
            duration: limiting(|t| t.duration, worst.duration),
            downtime: limiting(|t| t.downtime, worst.downtime),
        },
        auto_execute: false,
        preserves_source_until_acceptance: true,
    }))
}

pub fn plan_robust_portfolio(
    policy: &Policy,
    catalog: &ScenarioCatalog,
    actions: &[ActionCandidate],
) -> Result<Receipt, ContractError> {
    validate(policy, catalog, actions)?;
    if policy.unresolved_alert_ids.is_empty() && actions.is_empty() {
        return Ok(Receipt {
            target_id: policy.target_id.clone(), scenario_set_id: catalog.scenario_set_id.clone(),
            scenario_set_sha256: catalog.scenario_set_sha256.clone(), disposition: Disposition::NoActionRequired,
            pareto_frontier: Vec::new(), excluded_candidates: Vec::new(), enumerated_subsets: 0,
        });
    }

    let mut eligible = Vec::new();
    let mut excluded = Vec::new();
    for action in actions {
        let why = reasons(policy, catalog, action);
        if why.is_empty() { eligible.push(action); }
        else { excluded.push(ExcludedCandidate { action_id: action.action_id.clone(), reasons: why }); }
    }
    eligible.sort_by(|a, b| a.action_id.cmp(&b.action_id));
    excluded.sort_by(|a, b| a.action_id.cmp(&b.action_id));

    let count = 1_u64.checked_shl(u32::try_from(eligible.len()).map_err(|_| ContractError::TooManyCandidates)?)
        .ok_or(ContractError::TooManyCandidates)?;
    let mut frontier: Vec<Plan> = Vec::new();
    let mut enumerated = 0_u64;
    for mask in 1..count {
        enumerated += 1;
        let selected: Vec<&ActionCandidate> = eligible.iter().enumerate()
            .filter_map(|(i, action)| ((mask >> i) & 1 == 1).then_some(*action)).collect();
        if selected.len() > policy.max_actions { continue; }
        let ids: BTreeSet<&str> = selected.iter().map(|a| a.action_id.as_str()).collect();
        if selected.iter().any(|a| a.depends_on_action_ids.iter().any(|dep| !ids.contains(dep.as_str()))) {
            continue;
        }
        let coverage: BTreeSet<&str> = selected.iter()
            .flat_map(|a| a.addresses_alert_ids.iter().map(String::as_str)).collect();
        if !policy.unresolved_alert_ids.iter().all(|id| coverage.contains(id.as_str())) { continue; }
        let Some(candidate) = make_plan(policy, catalog, &selected)? else { continue; };
        if frontier.iter().any(|existing| dominates(existing, &candidate)) { continue; }
        frontier.retain(|existing| !dominates(&candidate, existing));
        frontier.push(candidate);
    }
    frontier.sort_by(plan_order);
    let disposition = if !frontier.is_empty() { Disposition::Feasible }
    else if excluded.iter().any(|c| c.reasons.contains(&ExclusionReason::ScenarioBoundsInvalid)) { Disposition::InvalidEvidence }
    else if excluded.iter().any(|c| c.reasons.iter().any(|r| matches!(r,
        ExclusionReason::MissingScenarioBounds | ExclusionReason::ScenarioBoundsUnverified
        | ExclusionReason::MissingPrerequisiteEvidence | ExclusionReason::MissingApprovalAuthority))) {
        Disposition::EvidenceIncomplete
    } else { Disposition::Infeasible };

    Ok(Receipt {
        target_id: policy.target_id.clone(), scenario_set_id: catalog.scenario_set_id.clone(),
        scenario_set_sha256: catalog.scenario_set_sha256.clone(), disposition,
        pareto_frontier: frontier, excluded_candidates: excluded, enumerated_subsets: enumerated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog(ids: &[&str]) -> ScenarioCatalog {
        ScenarioCatalog { scenario_set_id: "stress-v0.1".into(), scenario_set_sha256: "a".repeat(64),
            scenario_ids: ids.iter().map(|id| (*id).to_string()).collect() }
    }
    fn policy(alerts: &[&str]) -> Policy {
        Policy { target_id: "archive-A".into(), unresolved_alert_ids: alerts.iter().map(|s| (*s).to_string()).collect(),
            available_evidence_ids: vec!["calibration-A".into()],
            available_approval_ids: vec!["archive-owner".into()], currency_code: "USD".into(),
            max_cost_microunits: 10_000, max_energy_millijoules: 10_000,
            max_duration_ms: 10_000, max_downtime_ms: 3_000, max_actions: 6 }
    }
    fn bound(id: &str, cost: u64, energy: u64, duration: u64, downtime: u64, status: EvidenceStatus) -> ScenarioBound {
        ScenarioBound { scenario_id: id.into(), cost_upper_microunits: cost, energy_upper_millijoules: energy,
            duration_upper_ms: duration, downtime_upper_ms: downtime, evidence_status: status,
            receipt_id: format!("receipt-{id}"), receipt_sha256: "b".repeat(64) }
    }
    fn verified(id: &str, cost: u64, energy: u64, duration: u64, downtime: u64) -> ScenarioBound {
        bound(id, cost, energy, duration, downtime, EvidenceStatus::Verified)
    }
    fn action(id: &str, covers: &[&str], bounds: Vec<ScenarioBound>) -> ActionCandidate {
        ActionCandidate { action_id: id.into(), addresses_alert_ids: covers.iter().map(|s| (*s).to_string()).collect(),
            prerequisite_evidence_ids: vec!["calibration-A".into()], depends_on_action_ids: Vec::new(),
            currency_code: "USD".into(), scenario_bounds: bounds, required_approvals: vec!["archive-owner".into()],
            required_postcondition_evidence: vec![format!("post-{id}")],
            preserves_source_until_acceptance: true, destructive: false }
    }

    #[test]
    fn one_scenario_reproduces_nominal_objectives() {
        let result = plan_robust_portfolio(&policy(&["alert-a"]), &catalog(&["nominal"]),
            &[action("nominal", &["alert-a"], vec![verified("nominal", 4000, 100, 100, 20)])]).unwrap();
        assert_eq!(result.disposition, Disposition::Feasible);
        assert_eq!(result.pareto_frontier[0].objectives.worst_case_cost_microunits, 4000);
        assert!(!result.pareto_frontier[0].auto_execute);
    }

    #[test]
    fn outage_scenario_excludes_the_nominally_cheapest_action() {
        let actions = [
            action("cheap", &["alert-a"], vec![verified("normal", 1000, 100, 100, 20),
                verified("reader", 9000, 100, 100, 20), verified("grid", 1200, 100, 100, 20)]),
            action("resilient", &["alert-a"], vec![verified("normal", 4000, 100, 100, 20),
                verified("reader", 6000, 100, 100, 20), verified("grid", 5000, 100, 100, 20)])];
        let mut p = policy(&["alert-a"]); p.max_cost_microunits = 7000;
        let result = plan_robust_portfolio(&p, &catalog(&["normal", "reader", "grid"]), &actions).unwrap();
        assert_eq!(result.pareto_frontier.len(), 1);
        assert_eq!(result.pareto_frontier[0].action_ids, vec!["resilient"]);
        assert_eq!(result.pareto_frontier[0].limiting_scenarios.cost, vec!["reader"]);
    }

    #[test]
    fn robust_frontier_keeps_cost_energy_tradeoffs_and_prunes_dominated_actions() {
        let actions = [
            action("cheap", &["alert-a"], vec![verified("n", 2000, 7000, 300, 100), verified("o", 3000, 9000, 400, 150)]),
            action("low-energy", &["alert-a"], vec![verified("n", 5000, 2000, 500, 200), verified("o", 5200, 2500, 450, 180)]),
            action("dominated", &["alert-a"], vec![verified("n", 6000, 8000, 700, 300), verified("o", 6500, 9000, 800, 350)])];
        let result = plan_robust_portfolio(&policy(&["alert-a"]), &catalog(&["n", "o"]), &actions).unwrap();
        assert_eq!(result.pareto_frontier.len(), 2);
        assert_eq!(result.pareto_frontier[0].action_ids, vec!["cheap"]);
        assert_eq!(result.pareto_frontier[1].action_ids, vec!["low-energy"]);
    }

    #[test]
    fn missing_scenario_evidence_never_falls_back_to_nominal() {
        let candidate = action("partial", &["alert-a"], vec![verified("normal", 10, 1, 1, 1)]);
        let result = plan_robust_portfolio(&policy(&["alert-a"]), &catalog(&["normal", "outage"]), &[candidate]).unwrap();
        assert_eq!(result.disposition, Disposition::EvidenceIncomplete);
        assert!(result.pareto_frontier.is_empty());
        assert!(result.excluded_candidates[0].reasons.contains(&ExclusionReason::MissingScenarioBounds));
    }

    #[test]
    fn invalid_and_unverified_scenario_bounds_are_distinct() {
        let unverified = action("unverified", &["alert-a"], vec![bound("n", 1, 1, 1, 1, EvidenceStatus::Unverified)]);
        let invalid = action("invalid", &["alert-a"], vec![bound("n", 1, 1, 1, 1, EvidenceStatus::Invalid)]);
        assert_eq!(plan_robust_portfolio(&policy(&["alert-a"]), &catalog(&["n"]), &[unverified]).unwrap().disposition,
            Disposition::EvidenceIncomplete);
        assert_eq!(plan_robust_portfolio(&policy(&["alert-a"]), &catalog(&["n"]), &[invalid]).unwrap().disposition,
            Disposition::InvalidEvidence);
    }

    #[test]
    fn all_tied_limiting_scenarios_are_reported() {
        let candidate = action("tie", &["alert-a"], vec![verified("n", 1000, 10, 5, 1),
            verified("outage", 5000, 20, 10, 2), verified("grid", 5000, 20, 10, 2)]);
        let result = plan_robust_portfolio(&policy(&["alert-a"]), &catalog(&["n", "outage", "grid"]), &[candidate]).unwrap();
        assert_eq!(result.pareto_frontier[0].limiting_scenarios.cost, vec!["grid", "outage"]);
    }

    #[test]
    fn cap_violation_in_any_scenario_makes_portfolio_infeasible() {
        let candidate = action("overrun", &["alert-a"], vec![verified("n", 1000, 1, 1, 1), verified("o", 6000, 1, 1, 1)]);
        let mut p = policy(&["alert-a"]); p.max_cost_microunits = 5000;
        let result = plan_robust_portfolio(&p, &catalog(&["n", "o"]), &[candidate]).unwrap();
        assert_eq!(result.disposition, Disposition::Infeasible);
    }

    #[test]
    fn dependency_closure_and_source_retention_are_enforced() {
        let prep = action("offline-kit", &[], vec![verified("n", 100, 10, 10, 0)]);
        let mut replacement = action("replace-reader", &["alert-reader"], vec![verified("n", 200, 20, 20, 1)]);
        replacement.depends_on_action_ids = vec!["offline-kit".into()];
        let result = plan_robust_portfolio(&policy(&["alert-reader"]), &catalog(&["n"]), &[prep, replacement]).unwrap();
        assert_eq!(result.pareto_frontier[0].action_ids, vec!["offline-kit", "replace-reader"]);
    }

    #[test]
    fn dependency_cycles_are_rejected() {
        let mut a = action("a", &["alert-a"], vec![verified("n", 1, 1, 1, 1)]);
        let mut b = action("b", &[], vec![verified("n", 1, 1, 1, 1)]);
        a.depends_on_action_ids = vec!["b".into()]; b.depends_on_action_ids = vec!["a".into()];
        assert_eq!(plan_robust_portfolio(&policy(&["alert-a"]), &catalog(&["n"]), &[a, b]),
            Err(ContractError::DependencyCycle));
    }

    #[test]
    fn destructive_or_source_discarding_candidates_are_not_selected() {
        let mut erase = action("erase", &["alert-a"], vec![verified("n", 1, 1, 1, 1)]);
        erase.destructive = true;
        let mut discard = action("discard", &["alert-a"], vec![verified("n", 1, 1, 1, 1)]);
        discard.preserves_source_until_acceptance = false;
        let result = plan_robust_portfolio(&policy(&["alert-a"]), &catalog(&["n"]), &[erase, discard]).unwrap();
        assert_eq!(result.disposition, Disposition::Infeasible);
        assert_eq!(result.excluded_candidates.len(), 2);
    }

    #[test]
    fn permutation_of_scenarios_and_actions_preserves_frontier() {
        let a = action("a", &["alert-a"], vec![verified("n", 1000, 200, 300, 40), verified("o", 2000, 100, 200, 20)]);
        let mut b = action("b", &["alert-a"], vec![verified("n", 1500, 150, 250, 30), verified("o", 1500, 250, 150, 30)]);
        let forward = plan_robust_portfolio(&policy(&["alert-a"]), &catalog(&["n", "o"]), &[a.clone(), b.clone()]).unwrap();
        b.scenario_bounds.reverse();
        let reverse = plan_robust_portfolio(&policy(&["alert-a"]), &catalog(&["o", "n"]), &[b, a]).unwrap();
        assert_eq!(forward.pareto_frontier, reverse.pareto_frontier);
    }

    #[test]
    fn duplicate_and_unknown_scenario_rows_are_rejected() {
        let duplicate = action("dup", &["alert-a"], vec![verified("n", 1, 1, 1, 1), verified("n", 2, 2, 2, 2)]);
        assert_eq!(plan_robust_portfolio(&policy(&["alert-a"]), &catalog(&["n"]), &[duplicate]),
            Err(ContractError::DuplicateScenarioBound));
        let unknown = action("unknown", &["alert-a"], vec![verified("elsewhere", 1, 1, 1, 1)]);
        assert_eq!(plan_robust_portfolio(&policy(&["alert-a"]), &catalog(&["n"]), &[unknown]),
            Err(ContractError::UnknownScenarioBound));
    }

    #[test]
    fn exact_hard_cap_boundary_is_feasible() {
        let candidate = action("boundary", &["alert-a"], vec![verified("n", 3000, 600, 300, 100), verified("o", 2000, 500, 250, 80)]);
        let mut p = policy(&["alert-a"]); p.max_cost_microunits = 3000; p.max_energy_millijoules = 600;
        p.max_duration_ms = 300; p.max_downtime_ms = 100;
        assert_eq!(plan_robust_portfolio(&p, &catalog(&["n", "o"]), &[candidate]).unwrap().disposition,
            Disposition::Feasible);
    }

    #[test]
    fn empty_alerts_and_no_actions_are_no_action_required() {
        let result = plan_robust_portfolio(&policy(&[]), &catalog(&["nominal"]), &[]).unwrap();
        assert_eq!(result.disposition, Disposition::NoActionRequired);
    }
}
