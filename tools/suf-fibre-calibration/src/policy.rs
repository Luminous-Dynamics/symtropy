// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Budgeted, receipt-gated advisory selection of follow-up measurements.
//! This module ranks actions only. It never executes, authorizes or dispatches.

use crate::decision::MeasurementAssessment;
use core::fmt;

/// Errors in the advisory action selector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyError {
    InvalidIdentifier(&'static str),
    InvalidBudgetOrResource(&'static str),
    InvalidReceiptDigest(&'static str),
}

/// Result type for advisory action selection.
pub type PolicyResult<T> = Result<T, PolicyError>;

impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(name) => write!(f, "invalid policy identifier: {name}"),
            Self::InvalidBudgetOrResource(name) => {
                write!(f, "invalid policy budget/resource value: {name}")
            }
            Self::InvalidReceiptDigest(name) => {
                write!(f, "invalid SHA-256 receipt reference: {name}")
            }
        }
    }
}

impl std::error::Error for PolicyError {}

/// A candidate action with a precomputed EVSI assessment and declared resource
/// and authority evidence. Receipt digests must be independently authenticated
/// and scope-checked by the caller.
#[derive(Clone, Debug, PartialEq)]
pub struct AdvisoryActionCandidate {
    pub assessment: MeasurementAssessment,
    pub resource_cost: f64,
    pub resource_unit: String,
    pub owner_authorization_receipt_sha256: Option<String>,
    pub safety_approval_receipt_sha256: Option<String>,
}

/// Why a candidate was not selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionRejectionReason {
    MissingOwnerAuthorization,
    MissingSafetyApproval,
    NoComparableNetValue,
    NonPositiveNetValue,
    ResourceUnitMismatch,
    ExceedsBudget,
}

/// Decision record for one candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdvisoryActionDecision {
    pub action_id: String,
    pub selected: bool,
    pub rejection_reasons: Vec<ActionRejectionReason>,
}

/// Advisory plan, not a work order or dispatch instruction.
#[derive(Clone, Debug, PartialEq)]
pub struct AdvisoryActionPlan {
    pub resource_unit: String,
    pub budget: f64,
    pub selected_resource_cost: f64,
    pub selected_action_ids: Vec<String>,
    pub decisions: Vec<AdvisoryActionDecision>,
    pub advisory_only: bool,
}

#[derive(Clone, Debug)]
struct RankedCandidate {
    original_index: usize,
    action_id: String,
    net_value: f64,
    resource_cost: f64,
}

/// Select positive-net-value feasible actions under one explicit resource
/// budget. Ranking is net value descending, then action ID ascending for
/// deterministic ties. Economic net value must already be comparable; all
/// resource costs must use the requested budget unit.
///
/// This function only produces advice. A digest-shaped receipt is not proof
/// of issuer authenticity, scope, expiry or permission to perform field work.
pub fn select_advisory_actions(
    candidates: &[AdvisoryActionCandidate],
    budget: f64,
    resource_unit: &str,
) -> PolicyResult<AdvisoryActionPlan> {
    if resource_unit.trim().is_empty() {
        return Err(PolicyError::InvalidIdentifier("resource_unit"));
    }
    if !budget.is_finite() || budget < 0.0 {
        return Err(PolicyError::InvalidBudgetOrResource("budget"));
    }

    let mut seen_action_ids: Vec<&str> = Vec::with_capacity(candidates.len());
    let mut decisions: Vec<AdvisoryActionDecision> = Vec::with_capacity(candidates.len());
    let mut ranked = Vec::with_capacity(candidates.len());

    for (index, candidate) in candidates.iter().enumerate() {
        let action_id = candidate.assessment.action_id.trim();
        if action_id.is_empty() {
            return Err(PolicyError::InvalidIdentifier("action_id"));
        }
        if seen_action_ids.contains(&action_id) {
            return Err(PolicyError::InvalidIdentifier("duplicate action_id"));
        }
        seen_action_ids.push(action_id);

        if !candidate.resource_cost.is_finite() || candidate.resource_cost < 0.0 {
            return Err(PolicyError::InvalidBudgetOrResource("resource_cost"));
        }
        if candidate.resource_unit.trim().is_empty() {
            return Err(PolicyError::InvalidIdentifier("candidate.resource_unit"));
        }
        validate_optional_digest(
            candidate.owner_authorization_receipt_sha256.as_deref(),
            "owner_authorization_receipt_sha256",
        )?;
        validate_optional_digest(
            candidate.safety_approval_receipt_sha256.as_deref(),
            "safety_approval_receipt_sha256",
        )?;

        let mut rejection_reasons = Vec::new();
        if candidate.owner_authorization_receipt_sha256.is_none() {
            rejection_reasons.push(ActionRejectionReason::MissingOwnerAuthorization);
        }
        if candidate.safety_approval_receipt_sha256.is_none() {
            rejection_reasons.push(ActionRejectionReason::MissingSafetyApproval);
        }
        if candidate.resource_unit != resource_unit {
            rejection_reasons.push(ActionRejectionReason::ResourceUnitMismatch);
        }
        match candidate.assessment.net_value {
            None => rejection_reasons.push(ActionRejectionReason::NoComparableNetValue),
            Some(value) if !value.is_finite() => {
                return Err(PolicyError::InvalidBudgetOrResource("net_value"));
            }
            Some(value) if value <= 0.0 => {
                rejection_reasons.push(ActionRejectionReason::NonPositiveNetValue);
            }
            Some(_) => {}
        }

        decisions.push(AdvisoryActionDecision {
            action_id: action_id.to_owned(),
            selected: false,
            rejection_reasons,
        });

        if decisions[index].rejection_reasons.is_empty() {
            ranked.push(RankedCandidate {
                original_index: index,
                action_id: action_id.to_owned(),
                net_value: candidate.assessment.net_value.unwrap_or(0.0),
                resource_cost: candidate.resource_cost,
            });
        }
    }

    ranked.sort_by(|left, right| {
        right
            .net_value
            .total_cmp(&left.net_value)
            .then_with(|| left.action_id.cmp(&right.action_id))
    });

    let mut remaining_budget = budget;
    let mut selected_resource_cost = 0.0;
    let mut selected_action_ids = Vec::new();

    for candidate in ranked {
        if candidate.resource_cost <= remaining_budget {
            remaining_budget -= candidate.resource_cost;
            selected_resource_cost += candidate.resource_cost;
            selected_action_ids.push(candidate.action_id);
            decisions[candidate.original_index].selected = true;
        } else {
            decisions[candidate.original_index]
                .rejection_reasons
                .push(ActionRejectionReason::ExceedsBudget);
        }
    }

    Ok(AdvisoryActionPlan {
        resource_unit: resource_unit.to_owned(),
        budget,
        selected_resource_cost,
        selected_action_ids,
        decisions,
        advisory_only: true,
    })
}

fn validate_optional_digest(value: Option<&str>, field: &'static str) -> PolicyResult<()> {
    if let Some(value) = value {
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(PolicyError::InvalidReceiptDigest(field));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::{AcquisitionDisposition, MeasurementAssessment};

    const RECEIPT: &str =
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn assessment(id: &str, net_value: Option<f64>) -> MeasurementAssessment {
        MeasurementAssessment {
            action_id: id.to_owned(),
            prior_minimum_expected_loss: 10.0,
            expected_posterior_minimum_loss: 5.0,
            expected_value_of_sample_information: 6.0,
            expected_information_gain_bits: 0.7,
            net_value,
            disposition: match net_value {
                Some(value) if value > 0.0 => AcquisitionDisposition::Acquire,
                Some(_) => AcquisitionDisposition::DoNotAcquire,
                None => AcquisitionDisposition::NoCostBasedRecommendation,
            },
        }
    }

    fn candidate(id: &str, net_value: Option<f64>, cost: f64) -> AdvisoryActionCandidate {
        AdvisoryActionCandidate {
            assessment: assessment(id, net_value),
            resource_cost: cost,
            resource_unit: "crew-hour".to_owned(),
            owner_authorization_receipt_sha256: Some(RECEIPT.to_owned()),
            safety_approval_receipt_sha256: Some(RECEIPT.to_owned()),
        }
    }

    #[test]
    fn selects_highest_net_value_actions_within_fixed_budget() {
        let candidates = vec![
            candidate("survey-B", Some(8.0), 3.0),
            candidate("survey-A", Some(5.0), 2.0),
            candidate("survey-C", Some(3.0), 2.0),
        ];
        let plan = select_advisory_actions(&candidates, 5.0, "crew-hour")
            .expect("valid plan");
        assert_eq!(plan.selected_action_ids, vec!["survey-B", "survey-A"]);
        assert!((plan.selected_resource_cost - 5.0).abs() < 1e-12);
        assert!(plan.advisory_only);
        assert_eq!(
            plan.decisions[2].rejection_reasons,
            vec![ActionRejectionReason::ExceedsBudget]
        );
    }

    #[test]
    fn missing_owner_authorization_prevents_selection() {
        let mut action = candidate("survey-unauthorized", Some(10.0), 1.0);
        action.owner_authorization_receipt_sha256 = None;
        let plan = select_advisory_actions(&[action], 2.0, "crew-hour")
            .expect("valid plan");
        assert!(plan.selected_action_ids.is_empty());
        assert_eq!(
            plan.decisions[0].rejection_reasons,
            vec![ActionRejectionReason::MissingOwnerAuthorization]
        );
    }

    #[test]
    fn missing_safety_approval_prevents_selection() {
        let mut action = candidate("survey-unsafe", Some(10.0), 1.0);
        action.safety_approval_receipt_sha256 = None;
        let plan = select_advisory_actions(&[action], 2.0, "crew-hour")
            .expect("valid plan");
        assert!(plan.selected_action_ids.is_empty());
        assert_eq!(
            plan.decisions[0].rejection_reasons,
            vec![ActionRejectionReason::MissingSafetyApproval]
        );
    }

    #[test]
    fn incomparable_economic_units_prevent_selection() {
        let action = candidate("survey-incomparable", None, 1.0);
        let plan = select_advisory_actions(&[action], 2.0, "crew-hour")
            .expect("valid plan");
        assert!(plan.selected_action_ids.is_empty());
        assert_eq!(
            plan.decisions[0].rejection_reasons,
            vec![ActionRejectionReason::NoComparableNetValue]
        );
    }

    #[test]
    fn non_positive_net_value_is_not_selected() {
        let action = candidate("survey-no-value", Some(0.0), 1.0);
        let plan = select_advisory_actions(&[action], 2.0, "crew-hour")
            .expect("valid plan");
        assert!(plan.selected_action_ids.is_empty());
        assert_eq!(
            plan.decisions[0].rejection_reasons,
            vec![ActionRejectionReason::NonPositiveNetValue]
        );
    }

    #[test]
    fn resource_unit_mismatch_prevents_selection() {
        let mut action = candidate("survey-other-resource", Some(10.0), 1.0);
        action.resource_unit = "km".to_owned();
        let plan = select_advisory_actions(&[action], 2.0, "crew-hour")
            .expect("valid plan");
        assert!(plan.selected_action_ids.is_empty());
        assert_eq!(
            plan.decisions[0].rejection_reasons,
            vec![ActionRejectionReason::ResourceUnitMismatch]
        );
    }

    #[test]
    fn rejects_invalid_authorization_receipt_digest() {
        let mut action = candidate("survey-bad-receipt", Some(10.0), 1.0);
        action.owner_authorization_receipt_sha256 = Some("not-a-digest".to_owned());
        assert_eq!(
            select_advisory_actions(&[action], 2.0, "crew-hour"),
            Err(PolicyError::InvalidReceiptDigest(
                "owner_authorization_receipt_sha256"
            ))
        );
    }

    #[test]
    fn rejects_duplicate_action_ids() {
        let actions = vec![
            candidate("same", Some(10.0), 1.0),
            candidate("same", Some(9.0), 1.0),
        ];
        assert_eq!(
            select_advisory_actions(&actions, 2.0, "crew-hour"),
            Err(PolicyError::InvalidIdentifier("duplicate action_id"))
        );
    }

    #[test]
    fn ties_use_action_id_for_deterministic_selection() {
        let actions = vec![
            candidate("zeta", Some(4.0), 1.0),
            candidate("alpha", Some(4.0), 1.0),
        ];
        let plan = select_advisory_actions(&actions, 1.0, "crew-hour")
            .expect("valid plan");
        assert_eq!(plan.selected_action_ids, vec!["alpha"]);
    }

    #[test]
    fn zero_budget_selects_no_positive_cost_action() {
        let plan = select_advisory_actions(
            &[candidate("survey", Some(4.0), 1.0)],
            0.0,
            "crew-hour",
        )
        .expect("valid empty plan");
        assert!(plan.selected_action_ids.is_empty());
        assert_eq!(
            plan.decisions[0].rejection_reasons,
            vec![ActionRejectionReason::ExceedsBudget]
        );
    }

    #[test]
    fn rejects_non_finite_budget() {
        assert_eq!(
            select_advisory_actions(&[], f64::NAN, "crew-hour"),
            Err(PolicyError::InvalidBudgetOrResource("budget"))
        );
    }
}
