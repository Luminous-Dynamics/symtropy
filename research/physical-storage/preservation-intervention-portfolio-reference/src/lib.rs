//! Deterministic evidence-bounded Pareto planning for preservation actions.
//!
//! This crate selects non-executing action portfolios from externally supplied
//! upper bounds. It does not authenticate evidence, measure cost/energy/time,
//! authorize actions, execute remediation, or clear watch alerts. "Addresses"
//! is a planning relation only; a separate verified postcondition/remediation
//! receipt must be consumed by Preservation Watch to clear an alert.

use std::collections::{BTreeSet, HashMap, HashSet};

pub const MAX_CANDIDATE_ACTIONS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundsEvidenceStatus {
    Verified,
    Unverified,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PortfolioDisposition {
    NoActionRequired,
    Feasible,
    Infeasible,
    EvidenceIncomplete,
    InvalidEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExclusionReason {
    BoundsUnverified,
    BoundsInvalid,
    MissingPrerequisiteEvidence,
    DestructiveAction,
    SourceNotPreserved,
    MissingApprovalAuthority,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortfolioPolicy {
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
pub struct ActionCandidate {
    pub action_id: String,
    /// Alert IDs this proposed action is intended to address. This is not a
    /// claim that the alert has been remediated.
    pub addresses_alert_ids: Vec<String>,
    pub prerequisite_evidence_ids: Vec<String>,
    pub depends_on_action_ids: Vec<String>,
    /// Conservative upper bounds, produced by an external qualified estimator.
    pub currency_code: String,
    pub cost_upper_microunits: u64,
    pub energy_upper_millijoules: u64,
    pub duration_upper_ms: u64,
    pub downtime_upper_ms: u64,
    pub bounds_status: BoundsEvidenceStatus,
    pub bounds_receipt_id: String,
    pub bounds_receipt_sha256: String,
    pub required_approvals: Vec<String>,
    /// Evidence required after the action. This is not produced by the planner.
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
pub struct BoundsEvidenceRef {
    pub action_id: String,
    pub receipt_id: String,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortfolioPlan {
    pub action_ids: Vec<String>,
    /// "Addresses" is prospective coverage, never a cleared-alert list.
    pub addressed_alert_ids: Vec<String>,
    pub required_prerequisite_evidence_ids: Vec<String>,
    pub required_approvals: Vec<String>,
    pub required_postcondition_evidence: Vec<String>,
    pub bounds_evidence: Vec<BoundsEvidenceRef>,
    pub cost_upper_microunits: u64,
    pub energy_upper_millijoules: u64,
    pub duration_upper_ms: u64,
    pub downtime_upper_ms: u64,
    pub auto_execute: bool,
    pub preserves_source_until_acceptance: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortfolioReceipt {
    pub target_id: String,
    pub disposition: PortfolioDisposition,
    /// Exact non-dominated alternatives; no scalar weighted score is emitted.
    pub pareto_frontier: Vec<PortfolioPlan>,
    pub excluded_candidates: Vec<ExcludedCandidate>,
    pub enumerated_subsets: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortfolioContractError {
    EmptyTargetId,
    InvalidCurrencyCode,
    EmptyAlertId,
    DuplicateAlertId,
    EmptyEvidenceId,
    DuplicateEvidenceId,
    EmptyApprovalId,
    DuplicateAvailableApproval,
    EmptyActionId,
    DuplicateActionId,
    CurrencyMismatch,
    InvalidBoundsReceiptId,
    InvalidBoundsReceiptDigest,
    EmptyRequiredApprovals,
    EmptyApprovalId,
    EmptyPostconditionEvidence,
    EmptyPostconditionId,
    DuplicateActionCoverage,
    DuplicatePrerequisite,
    DuplicateDependency,
    DuplicateApproval,
    DuplicatePostcondition,
    UnknownAddressedAlert,
    UnknownActionDependency,
    SelfDependency,
    DependencyCycle,
    TooManyCandidates,
    NoActionsWhenAlertsClear,
    ArithmeticOverflow,
}

fn is_currency_code(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|b| b.is_ascii_uppercase())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn has_duplicate<T: PartialEq>(values: &[T]) -> bool {
    values.iter().enumerate().any(|(i, value)| values[..i].contains(value))
}

fn detect_dependency_cycle(
    index: usize,
    actions: &[ActionCandidate],
    by_id: &HashMap<String, usize>,
    state: &mut [u8],
) -> Result<(), PortfolioContractError> {
    match state[index] {
        1 => return Err(PortfolioContractError::DependencyCycle),
        2 => return Ok(()),
        _ => {}
    }
    state[index] = 1;
    for dependency in &actions[index].depends_on_action_ids {
        let dep_index = *by_id.get(dependency)
            .ok_or(PortfolioContractError::UnknownActionDependency)?;
        detect_dependency_cycle(dep_index, actions, by_id, state)?;
    }
    state[index] = 2;
    Ok(())
}

fn dominates(left: &PortfolioPlan, right: &PortfolioPlan) -> bool {
    let no_worse = left.cost_upper_microunits <= right.cost_upper_microunits
        && left.energy_upper_millijoules <= right.energy_upper_millijoules
        && left.duration_upper_ms <= right.duration_upper_ms
        && left.downtime_upper_ms <= right.downtime_upper_ms
        && left.action_ids.len() <= right.action_ids.len();
    let strictly_better = left.cost_upper_microunits < right.cost_upper_microunits
        || left.energy_upper_millijoules < right.energy_upper_millijoules
        || left.duration_upper_ms < right.duration_upper_ms
        || left.downtime_upper_ms < right.downtime_upper_ms
        || left.action_ids.len() < right.action_ids.len();
    no_worse && strictly_better
}

fn plan_order(left: &PortfolioPlan, right: &PortfolioPlan) -> std::cmp::Ordering {
    left.cost_upper_microunits.cmp(&right.cost_upper_microunits)
        .then_with(|| left.energy_upper_millijoules.cmp(&right.energy_upper_millijoules))
        .then_with(|| left.duration_upper_ms.cmp(&right.duration_upper_ms))
        .then_with(|| left.downtime_upper_ms.cmp(&right.downtime_upper_ms))
        .then_with(|| left.action_ids.cmp(&right.action_ids))
}

fn validate_contract(
    policy: &PortfolioPolicy,
    actions: &[ActionCandidate],
) -> Result<HashMap<String, usize>, PortfolioContractError> {
    if policy.target_id.trim().is_empty() {
        return Err(PortfolioContractError::EmptyTargetId);
    }
    if !is_currency_code(&policy.currency_code) {
        return Err(PortfolioContractError::InvalidCurrencyCode);
    }
    if has_duplicate(&policy.unresolved_alert_ids) {
        return Err(PortfolioContractError::DuplicateAlertId);
    }
    if policy.unresolved_alert_ids.iter().any(|id| id.trim().is_empty()) {
        return Err(PortfolioContractError::EmptyAlertId);
    }
    if has_duplicate(&policy.available_evidence_ids) {
        return Err(PortfolioContractError::DuplicateEvidenceId);
    }
    if policy.available_evidence_ids.iter().any(|id| id.trim().is_empty()) {
        return Err(PortfolioContractError::EmptyEvidenceId);
    }
    if has_duplicate(&policy.available_approval_ids) {
        return Err(PortfolioContractError::DuplicateAvailableApproval);
    }
    if policy.available_approval_ids.iter().any(|id| id.trim().is_empty()) {
        return Err(PortfolioContractError::EmptyApprovalId);
    }
    if actions.len() > MAX_CANDIDATE_ACTIONS {
        return Err(PortfolioContractError::TooManyCandidates);
    }
    if policy.unresolved_alert_ids.is_empty() && !actions.is_empty() {
        return Err(PortfolioContractError::NoActionsWhenAlertsClear);
    }

    let known_alerts: HashSet<&str> =
        policy.unresolved_alert_ids.iter().map(String::as_str).collect();
    let mut by_id = HashMap::with_capacity(actions.len());

    for (index, action) in actions.iter().enumerate() {
        if action.action_id.trim().is_empty() {
            return Err(PortfolioContractError::EmptyActionId);
        }
        if by_id.insert(action.action_id.clone(), index).is_some() {
            return Err(PortfolioContractError::DuplicateActionId);
        }
        if action.currency_code != policy.currency_code {
            return Err(PortfolioContractError::CurrencyMismatch);
        }
        if action.bounds_receipt_id.trim().is_empty() {
            return Err(PortfolioContractError::InvalidBoundsReceiptId);
        }
        if !is_sha256_hex(&action.bounds_receipt_sha256) {
            return Err(PortfolioContractError::InvalidBoundsReceiptDigest);
        }
        if action.required_approvals.is_empty() {
            return Err(PortfolioContractError::EmptyRequiredApprovals);
        }
        if action.required_postcondition_evidence.is_empty() {
            return Err(PortfolioContractError::EmptyPostconditionEvidence);
        }
        if has_duplicate(&action.addresses_alert_ids) {
            return Err(PortfolioContractError::DuplicateActionCoverage);
        }
        if has_duplicate(&action.prerequisite_evidence_ids) {
            return Err(PortfolioContractError::DuplicatePrerequisite);
        }
        if has_duplicate(&action.depends_on_action_ids) {
            return Err(PortfolioContractError::DuplicateDependency);
        }
        if has_duplicate(&action.required_approvals) {
            return Err(PortfolioContractError::DuplicateApproval);
        }
        if has_duplicate(&action.required_postcondition_evidence) {
            return Err(PortfolioContractError::DuplicatePostcondition);
        }
        if action.addresses_alert_ids.iter().any(|id| id.trim().is_empty()) {
            return Err(PortfolioContractError::EmptyAlertId);
        }
        if action.prerequisite_evidence_ids.iter().any(|id| id.trim().is_empty()) {
            return Err(PortfolioContractError::EmptyEvidenceId);
        }
        if action.required_approvals.iter().any(|id| id.trim().is_empty()) {
            return Err(PortfolioContractError::EmptyApprovalId);
        }
        if action.required_postcondition_evidence.iter().any(|id| id.trim().is_empty()) {
            return Err(PortfolioContractError::EmptyPostconditionId);
        }
        for alert_id in &action.addresses_alert_ids {
            if !known_alerts.contains(alert_id.as_str()) {
                return Err(PortfolioContractError::UnknownAddressedAlert);
            }
        }
        for dependency in &action.depends_on_action_ids {
            if dependency == &action.action_id {
                return Err(PortfolioContractError::SelfDependency);
            }
        }
    }

    for action in actions {
        for dependency in &action.depends_on_action_ids {
            if !by_id.contains_key(dependency) {
                return Err(PortfolioContractError::UnknownActionDependency);
            }
        }
    }

    let mut state = vec![0u8; actions.len()];
    for index in 0..actions.len() {
        detect_dependency_cycle(index, actions, &by_id, &mut state)?;
    }
    Ok(by_id)
}

/// Generate every feasible action subset and return the exact Pareto frontier.
///
/// The candidate count is capped at 16 so exhaustive enumeration is bounded.
/// Only Verified upper bounds with available prerequisites are selectable.
/// Destructive or source-discarding actions are always excluded.
pub fn plan_portfolio(
    policy: &PortfolioPolicy,
    actions: &[ActionCandidate],
) -> Result<PortfolioReceipt, PortfolioContractError> {
    validate_contract(policy, actions)?;
    if policy.unresolved_alert_ids.is_empty() {
        return Ok(PortfolioReceipt {
            target_id: policy.target_id.clone(),
            disposition: PortfolioDisposition::NoActionRequired,
            pareto_frontier: Vec::new(),
            excluded_candidates: Vec::new(),
            enumerated_subsets: 0,
        });
    }

    let available: HashSet<&str> =
        policy.available_evidence_ids.iter().map(String::as_str).collect();
    let available_approvals: HashSet<&str> =
        policy.available_approval_ids.iter().map(String::as_str).collect();
    let mut eligible = vec![true; actions.len()];
    let mut excluded_candidates = Vec::new();

    for (index, action) in actions.iter().enumerate() {
        let mut reasons = Vec::new();
        match action.bounds_status {
            BoundsEvidenceStatus::Verified => {}
            BoundsEvidenceStatus::Unverified => reasons.push(ExclusionReason::BoundsUnverified),
            BoundsEvidenceStatus::Invalid => reasons.push(ExclusionReason::BoundsInvalid),
        }
        if action.prerequisite_evidence_ids.iter().any(
            |id| !available.contains(id.as_str())
        ) {
            reasons.push(ExclusionReason::MissingPrerequisiteEvidence);
        }
        if action.required_approvals.iter().any(
            |id| !available_approvals.contains(id.as_str())
        ) {
            reasons.push(ExclusionReason::MissingApprovalAuthority);
        }
        if action.destructive {
            reasons.push(ExclusionReason::DestructiveAction);
        }
        if !action.preserves_source_until_acceptance {
            reasons.push(ExclusionReason::SourceNotPreserved);
        }
        if !reasons.is_empty() {
            eligible[index] = false;
            excluded_candidates.push(ExcludedCandidate {
                action_id: action.action_id.clone(),
                reasons,
            });
        }
    }

    let mut candidate_plans = Vec::new();
    let subset_count = 1u64 << actions.len();
    let mut enumerated_subsets = 0u64;
    let required_alerts: BTreeSet<&str> =
        policy.unresolved_alert_ids.iter().map(String::as_str).collect();

    for mask in 1u64..subset_count {
        enumerated_subsets += 1;
        let selected: Vec<usize> = (0..actions.len())
            .filter(|index| (mask & (1u64 << index)) != 0)
            .collect();
        if selected.len() > policy.max_actions {
            continue;
        }
        if selected.iter().any(|index| !eligible[*index]) {
            continue;
        }

        let selected_ids: HashSet<&str> = selected.iter()
            .map(|index| actions[*index].action_id.as_str())
            .collect();
        let dependencies_present = selected.iter().all(|index| {
            actions[*index].depends_on_action_ids.iter()
                .all(|dependency| selected_ids.contains(dependency.as_str()))
        });
        if !dependencies_present {
            continue;
        }

        let mut cost = 0u64;
        let mut energy = 0u64;
        let mut duration = 0u64;
        let mut downtime = 0u64;
        let mut covered = BTreeSet::new();
        let mut prerequisite_ids = BTreeSet::new();
        let mut approval_ids = BTreeSet::new();
        let mut postconditions = BTreeSet::new();
        let mut action_ids = Vec::with_capacity(selected.len());
        let mut bounds_evidence = Vec::with_capacity(selected.len());

        for index in &selected {
            let action = &actions[*index];
            cost = cost.checked_add(action.cost_upper_microunits)
                .ok_or(PortfolioContractError::ArithmeticOverflow)?;
            energy = energy.checked_add(action.energy_upper_millijoules)
                .ok_or(PortfolioContractError::ArithmeticOverflow)?;
            duration = duration.checked_add(action.duration_upper_ms)
                .ok_or(PortfolioContractError::ArithmeticOverflow)?;
            downtime = downtime.checked_add(action.downtime_upper_ms)
                .ok_or(PortfolioContractError::ArithmeticOverflow)?;
            covered.extend(action.addresses_alert_ids.iter().map(String::as_str));
            prerequisite_ids.extend(action.prerequisite_evidence_ids.iter().cloned());
            approval_ids.extend(action.required_approvals.iter().cloned());
            postconditions.extend(action.required_postcondition_evidence.iter().cloned());
            action_ids.push(action.action_id.clone());
            bounds_evidence.push(BoundsEvidenceRef {
                action_id: action.action_id.clone(),
                receipt_id: action.bounds_receipt_id.clone(),
                receipt_sha256: action.bounds_receipt_sha256.clone(),
            });
        }

        if cost > policy.max_cost_microunits
            || energy > policy.max_energy_millijoules
            || duration > policy.max_duration_ms
            || downtime > policy.max_downtime_ms
            || !required_alerts.is_subset(&covered)
        {
            continue;
        }

        action_ids.sort();
        bounds_evidence.sort_by(|left, right| left.action_id.cmp(&right.action_id));
        candidate_plans.push(PortfolioPlan {
            action_ids,
            addressed_alert_ids: covered.into_iter().map(str::to_string).collect(),
            required_prerequisite_evidence_ids: prerequisite_ids.into_iter().collect(),
            required_approvals: approval_ids.into_iter().collect(),
            required_postcondition_evidence: postconditions.into_iter().collect(),
            bounds_evidence,
            cost_upper_microunits: cost,
            energy_upper_millijoules: energy,
            duration_upper_ms: duration,
            downtime_upper_ms: downtime,
            auto_execute: false,
            preserves_source_until_acceptance: true,
        });
    }

    let pareto_frontier: Vec<PortfolioPlan> = candidate_plans.iter()
        .filter(|candidate| !candidate_plans.iter().any(
            |other| other.action_ids != candidate.action_ids && dominates(other, candidate)
        ))
        .cloned()
        .collect();

    let mut pareto_frontier = pareto_frontier;
    pareto_frontier.sort_by(plan_order);
    excluded_candidates.sort_by(|left, right| left.action_id.cmp(&right.action_id));

    let disposition = if !pareto_frontier.is_empty() {
        PortfolioDisposition::Feasible
    } else if excluded_candidates.iter().any(|candidate| {
        candidate.reasons.contains(&ExclusionReason::BoundsInvalid)
    }) {
        PortfolioDisposition::InvalidEvidence
    } else if excluded_candidates.iter().any(|candidate| candidate.reasons.iter().any(
        |reason| matches!(
            reason,
            ExclusionReason::BoundsUnverified | ExclusionReason::MissingPrerequisiteEvidence
        )
    )) {
        PortfolioDisposition::EvidenceIncomplete
    } else {
        PortfolioDisposition::Infeasible
    };

    Ok(PortfolioReceipt {
        target_id: policy.target_id.clone(),
        disposition,
        pareto_frontier,
        excluded_candidates,
        enumerated_subsets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(alerts: &[&str]) -> PortfolioPolicy {
        PortfolioPolicy {
            target_id: "archive-A".into(),
            unresolved_alert_ids: alerts.iter().map(|id| (*id).to_string()).collect(),
            available_evidence_ids: vec!["evidence-fixity".into(), "evidence-trust".into()],
            available_approval_ids: vec!["archive_owner".into(), "independent_reviewer".into()],
            currency_code: "USD".into(),
            max_cost_microunits: 20_000_000,
            max_energy_millijoules: 100_000,
            max_duration_ms: 20_000,
            max_downtime_ms: 5_000,
            max_actions: 8,
        }
    }

    fn action(id: &str, covers: &[&str], cost: u64, energy: u64, duration: u64, downtime: u64) -> ActionCandidate {
        ActionCandidate {
            action_id: id.into(),
            addresses_alert_ids: covers.iter().map(|item| (*item).to_string()).collect(),
            prerequisite_evidence_ids: vec!["evidence-fixity".into()],
            depends_on_action_ids: Vec::new(),
            currency_code: "USD".into(),
            cost_upper_microunits: cost,
            energy_upper_millijoules: energy,
            duration_upper_ms: duration,
            downtime_upper_ms: downtime,
            bounds_status: BoundsEvidenceStatus::Verified,
            bounds_receipt_id: format!("bounds-{id}"),
            bounds_receipt_sha256: "a".repeat(64),
            required_approvals: vec!["archive_owner".into(), "independent_reviewer".into()],
            required_postcondition_evidence: vec![format!("postcondition-{id}")],
            preserves_source_until_acceptance: true,
            destructive: false,
        }
    }

    #[test]
    fn one_verified_action_produces_a_non_executing_plan() {
        let result = plan_portfolio(&policy(&["alert-fixity"]), &[
            action("verify-fixity", &["alert-fixity"], 4_000_000, 10_000, 1_000, 0),
        ]).unwrap();
        assert_eq!(result.disposition, PortfolioDisposition::Feasible);
        assert_eq!(result.pareto_frontier.len(), 1);
        let plan = &result.pareto_frontier[0];
        assert!(!plan.auto_execute);
        assert!(plan.preserves_source_until_acceptance);
        assert_eq!(plan.addressed_alert_ids, vec!["alert-fixity"]);
    }

    #[test]
    fn exact_pareto_frontier_keeps_incomparable_tradeoffs_and_removes_dominated_plan() {
        let result = plan_portfolio(&policy(&["alert-fixity"]), &[
            action("low-cost", &["alert-fixity"], 3_000_000, 40_000, 900, 100),
            action("low-energy", &["alert-fixity"], 5_000_000, 20_000, 1_500, 500),
            action("dominated", &["alert-fixity"], 6_000_000, 50_000, 2_000, 700),
        ]).unwrap();
        assert_eq!(result.disposition, PortfolioDisposition::Feasible);
        assert_eq!(result.pareto_frontier.len(), 2);
        assert_eq!(
            result.pareto_frontier.iter().map(|plan| plan.action_ids[0].as_str()).collect::<Vec<_>>(),
            vec!["low-cost", "low-energy"]
        );
    }

    #[test]
    fn multiple_alerts_require_complete_coverage() {
        let result = plan_portfolio(&policy(&["alert-fixity", "alert-trust"]), &[
            action("verify-fixity", &["alert-fixity"], 2_000_000, 2_000, 100, 0),
            action("refresh-trust", &["alert-trust"], 3_000_000, 3_000, 200, 0),
        ]).unwrap();
        assert_eq!(result.disposition, PortfolioDisposition::Feasible);
        assert_eq!(result.pareto_frontier.len(), 1);
        assert_eq!(result.pareto_frontier[0].action_ids, vec!["refresh-trust", "verify-fixity"]);
        assert_eq!(result.pareto_frontier[0].addressed_alert_ids, vec!["alert-fixity", "alert-trust"]);
    }

    #[test]
    fn dependency_actions_are_selected_as_a_closed_set() {
        let prep = action("prepare-reader", &[], 1_000_000, 1_000, 100, 0);
        let mut read = action("replace-reader", &["alert-reader"], 2_000_000, 2_000, 300, 20);
        read.depends_on_action_ids = vec!["prepare-reader".into()];
        let result = plan_portfolio(&policy(&["alert-reader"]), &[prep, read]).unwrap();
        assert_eq!(result.disposition, PortfolioDisposition::Feasible);
        assert_eq!(result.pareto_frontier[0].action_ids, vec!["prepare-reader", "replace-reader"]);
    }

    #[test]
    fn missing_dependency_prevents_a_portfolio() {
        let mut read = action("replace-reader", &["alert-reader"], 2_000_000, 2_000, 300, 20);
        read.depends_on_action_ids = vec!["prepare-reader".into()];
        assert_eq!(
            plan_portfolio(&policy(&["alert-reader"]), &[read]),
            Err(PortfolioContractError::UnknownActionDependency)
        );
    }

    #[test]
    fn missing_prerequisite_evidence_yields_evidence_incomplete() {
        let mut candidate = action("verify-fixity", &["alert-fixity"], 1_000_000, 1_000, 100, 0);
        candidate.prerequisite_evidence_ids = vec!["missing-calibration".into()];
        let result = plan_portfolio(&policy(&["alert-fixity"]), &[candidate]).unwrap();
        assert_eq!(result.disposition, PortfolioDisposition::EvidenceIncomplete);
        assert!(result.pareto_frontier.is_empty());
    }

    #[test]
    fn unavailable_approval_authority_excludes_action_and_requires_evidence_completion() {
        let mut p = policy(&["alert-fixity"]);
        p.available_approval_ids = vec!["archive_owner".into()];
        let candidate = action("verify-fixity", &["alert-fixity"], 1, 1, 1, 1);
        let result = plan_portfolio(&p, &[candidate]).unwrap();
        assert_eq!(result.disposition, PortfolioDisposition::EvidenceIncomplete);
        assert_eq!(
            result.excluded_candidates[0].reasons,
            vec![ExclusionReason::MissingApprovalAuthority]
        );
    }

    #[test]
    fn unverified_bounds_are_not_optimized_as_if_measured() {
        let mut candidate = action("create-copy", &["alert-redundancy"], 1, 1, 1, 1);
        candidate.bounds_status = BoundsEvidenceStatus::Unverified;
        let result = plan_portfolio(&policy(&["alert-redundancy"]), &[candidate]).unwrap();
        assert_eq!(result.disposition, PortfolioDisposition::EvidenceIncomplete);
        assert_eq!(result.excluded_candidates[0].reasons, vec![ExclusionReason::BoundsUnverified]);
    }

    #[test]
    fn budget_caps_are_hard_constraints() {
        let mut p = policy(&["alert-fixity"]);
        p.max_cost_microunits = 999;
        let result = plan_portfolio(&p, &[
            action("verify-fixity", &["alert-fixity"], 1_000, 1, 1, 0),
        ]).unwrap();
        assert_eq!(result.disposition, PortfolioDisposition::Infeasible);
        assert!(result.pareto_frontier.is_empty());
    }

    #[test]
    fn destructive_or_source_discarding_actions_are_never_selected() {
        let mut destructive = action("overwrite-source", &["alert-fixity"], 1, 1, 1, 0);
        destructive.destructive = true;
        let mut source_drop = action("delete-source", &["alert-fixity"], 1, 1, 1, 0);
        source_drop.preserves_source_until_acceptance = false;
        let result = plan_portfolio(&policy(&["alert-fixity"]), &[destructive, source_drop]).unwrap();
        assert_eq!(result.disposition, PortfolioDisposition::Infeasible);
        assert!(result.pareto_frontier.is_empty());
        assert!(result.excluded_candidates.iter().all(|item| !item.reasons.is_empty()));
    }

    #[test]
    fn cycles_in_action_dependencies_are_rejected() {
        let mut first = action("first", &["alert-fixity"], 1, 1, 1, 1);
        let mut second = action("second", &[], 1, 1, 1, 1);
        first.depends_on_action_ids = vec!["second".into()];
        second.depends_on_action_ids = vec!["first".into()];
        assert_eq!(
            plan_portfolio(&policy(&["alert-fixity"]), &[first, second]),
            Err(PortfolioContractError::DependencyCycle)
        );
    }

    #[test]
    fn mixed_currency_candidates_are_rejected() {
        let mut candidate = action("verify-fixity", &["alert-fixity"], 1, 1, 1, 1);
        candidate.currency_code = "ZAR".into();
        assert_eq!(
            plan_portfolio(&policy(&["alert-fixity"]), &[candidate]),
            Err(PortfolioContractError::CurrencyMismatch)
        );
    }

    #[test]
    fn empty_alert_set_is_a_no_action_state() {
        let mut p = policy(&[]);
        p.available_evidence_ids.clear();
        let result = plan_portfolio(&p, &[]).unwrap();
        assert_eq!(result.disposition, PortfolioDisposition::NoActionRequired);
        assert!(result.pareto_frontier.is_empty());
    }

    #[test]
    fn duplicate_bounds_receipt_is_allowed_only_if_ids_are_distinct_actions() {
        // Bounds receipt identity is scoped to an action; the planner does not
        // claim that a shared receipt authenticates multiple cost envelopes.
        let first = action("verify-a", &["alert-a"], 1, 1, 1, 1);
        let mut second = action("verify-b", &["alert-a"], 2, 2, 2, 2);
        second.bounds_receipt_id = first.bounds_receipt_id.clone();
        second.cost_upper_microunits = 2;
        second.energy_upper_millijoules = 0;
        second.duration_upper_ms = 0;
        second.downtime_upper_ms = 0;
        let result = plan_portfolio(&policy(&["alert-a"]), &[first, second]).unwrap();
        assert_eq!(result.disposition, PortfolioDisposition::Feasible);
        assert_eq!(result.pareto_frontier.len(), 2);
    }

    #[test]
    fn action_count_cap_rejects_unbounded_candidate_sets() {
        let candidates: Vec<ActionCandidate> = (0..=MAX_CANDIDATE_ACTIONS)
            .map(|index| action(&format!("action-{index}"), &["alert-a"], 1, 1, 1, 1))
            .collect();
        assert_eq!(
            plan_portfolio(&policy(&["alert-a"]), &candidates),
            Err(PortfolioContractError::TooManyCandidates)
        );
    }

    #[test]
    fn plan_action_limit_is_enforced_after_dependency_and_coverage_checks() {
        let mut p = policy(&["alert-a", "alert-b"]);
        p.max_actions = 1;
        let candidates = [
            action("address-a", &["alert-a"], 1, 1, 1, 0),
            action("address-b", &["alert-b"], 1, 1, 1, 0),
        ];
        let result = plan_portfolio(&p, &candidates).unwrap();
        assert_eq!(result.disposition, PortfolioDisposition::Infeasible);
        assert!(result.pareto_frontier.is_empty());
    }

    #[test]
    fn arithmetic_overflow_is_rejected_not_saturated() {
        let candidates = [
            action("address-a", &["alert-a"], u64::MAX, 1, 1, 1),
            action("address-b", &["alert-b"], 1, 1, 1, 1),
        ];
        assert_eq!(
            plan_portfolio(&policy(&["alert-a", "alert-b"]), &candidates),
            Err(PortfolioContractError::ArithmeticOverflow)
        );
    }

    #[test]
    fn invalid_bounds_evidence_is_distinct_from_missing_evidence() {
        let mut candidate = action("create-copy", &["alert-redundancy"], 1, 1, 1, 1);
        candidate.bounds_status = BoundsEvidenceStatus::Invalid;
        let result = plan_portfolio(&policy(&["alert-redundancy"]), &[candidate]).unwrap();
        assert_eq!(result.disposition, PortfolioDisposition::InvalidEvidence);
        assert_eq!(result.excluded_candidates[0].reasons, vec![ExclusionReason::BoundsInvalid]);
    }

    #[test]
    fn equal_resource_portfolio_with_more_actions_is_dominated() {
        let mut direct = action("direct", &["alert-a"], 1_000, 1_000, 1_000, 100);
        direct.addresses_alert_ids = vec!["alert-a".into()];
        let helper = action("helper", &[], 0, 0, 0, 0);
        let mut combined = direct.clone();
        combined.action_id = "combined".into();
        combined.addresses_alert_ids = vec!["alert-a".into()];
        let mut helper = helper;
        helper.action_id = "helper".into();
        let mut dependent = combined;
        dependent.depends_on_action_ids = vec!["helper".into()];
        let result = plan_portfolio(
            &policy(&["alert-a"]),
            &[direct, helper, dependent],
        ).unwrap();
        assert!(result.pareto_frontier.iter().all(|plan| {
            plan.action_ids != vec!["combined", "helper"]
        }));
    }

    #[test]
    fn candidate_cannot_claim_to_address_unknown_alert() {
        let candidate = action("verify-fixity", &["not-in-policy"], 1, 1, 1, 1);
        assert_eq!(
            plan_portfolio(&policy(&["alert-fixity"]), &[candidate]),
            Err(PortfolioContractError::UnknownAddressedAlert)
        );
    }

    #[test]
    fn empty_prerequisite_and_approval_ids_are_rejected() {
        let mut no_evidence_id = action("verify-fixity", &["alert-fixity"], 1, 1, 1, 1);
        no_evidence_id.prerequisite_evidence_ids = vec!["".into()];
        assert_eq!(
            plan_portfolio(&policy(&["alert-fixity"]), &[no_evidence_id]),
            Err(PortfolioContractError::EmptyEvidenceId)
        );

        let mut no_approval_id = action("verify-fixity", &["alert-fixity"], 1, 1, 1, 1);
        no_approval_id.required_approvals = vec!["".into()];
        assert_eq!(
            plan_portfolio(&policy(&["alert-fixity"]), &[no_approval_id]),
            Err(PortfolioContractError::EmptyApprovalId)
        );
    }

    #[test]
    fn outputs_are_deterministic_across_candidate_input_order() {
        let first = action("verify-a", &["alert-fixity"], 1, 2, 3, 4);
        let second = action("verify-b", &["alert-fixity"], 2, 1, 3, 4);
        let p = policy(&["alert-fixity"]);
        let forward = plan_portfolio(&p, &[first.clone(), second.clone()]).unwrap();
        let reverse = plan_portfolio(&p, &[second, first]).unwrap();
        assert_eq!(forward.pareto_frontier, reverse.pareto_frontier);
    }
}
