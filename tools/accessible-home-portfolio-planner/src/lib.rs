// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic-only staged intervention portfolio enumeration.
//! A selected intervention is a planned action, not evidence that a real constraint is satisfied.

use chrono::DateTime;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const EVALUATOR_VERSION: &str = "accessible-home-portfolio-planner/0.1.0";
const FIXTURE: &str = include_str!(
    "../../../docs/built-environment/accessible-home-portfolio-v0.1.example.json"
);
const MAX_INTERVENTIONS: usize = 20;
const MAX_QUOTE_EXAMPLES: usize = 12;
const MAX_FRONTIER_SIZE: usize = 20_000;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PortfolioInput {
    pub schema_version: String,
    pub fixture_kind: String,
    pub portfolio_id: String,
    pub claim_boundary: String,
    pub currency: String,
    pub budget_cap_upper_minor_units: u64,
    pub planning_horizon_months: u16,
    pub hard_constraints: Vec<HardConstraint>,
    pub preferences: Vec<Preference>,
    pub interventions: Vec<Intervention>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BaselineStatus {
    SatisfiedForFixture,
    Unresolved,
    Conflicted,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HardConstraint {
    pub id: String,
    pub statement: String,
    pub baseline_status: BaselineStatus,
    pub note: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Preference {
    pub id: String,
    pub statement: String,
    pub baseline_status: BaselineStatus,
    pub note: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PreferenceConflictResolution {
    pub preference_id: String,
    pub conflicting_intervention_id: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CostBasis {
    SyntheticScenario,
    Estimate,
    Quote,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "cost_status", rename_all = "snake_case")]
pub enum CostEvidence {
    KnownRange {
        basis: CostBasis,
        one_time_low_minor_units: u64,
        one_time_high_minor_units: u64,
        annual_maintenance_low_minor_units: u64,
        annual_maintenance_high_minor_units: u64,
        source_ref: String,
        observed_at: String,
    },
    Unknown {
        basis: CostBasis,
        reason: String,
        source_ref: String,
        observed_at: String,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccessGate {
    NotRequired,
    Cleared,
    Pending,
    Denied,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryState {
    Proposed,
    QuoteReceived,
    Funded,
    Installed,
    Measured,
    ProfessionallyReviewed,
}

impl DeliveryState {
    fn rank(self) -> u8 {
        match self {
            Self::Proposed => 0,
            Self::QuoteReceived => 1,
            Self::Funded => 2,
            Self::Installed => 3,
            Self::Measured => 4,
            Self::ProfessionallyReviewed => 5,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DayRange {
    pub low: u32,
    pub high: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Intervention {
    pub id: String,
    pub title: String,
    pub depends_on: Vec<String>,
    pub addresses_hard_constraint_ids: Vec<String>,
    pub supports_preference_ids: Vec<String>,
    pub conflicts_with_preference_ids: Vec<String>,
    pub resolves_preference_conflicts: Vec<PreferenceConflictResolution>,
    pub cost: CostEvidence,
    pub lead_time_days: DayRange,
    /// Relative scale 0..=5, defined within this synthetic scenario, not a universal scale.
    pub resident_disruption_points: u8,
    pub access_gate: AccessGate,
    pub delivery_state: DeliveryState,
    pub note: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ConstraintCoverage {
    pub constraint_id: String,
    pub disposition: String,
    pub selected_remediation_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PlanStage {
    pub stage: usize,
    pub intervention_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CostRangeReceipt {
    pub low_minor_units: u64,
    pub high_minor_units: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CandidatePlan {
    pub selected_intervention_ids: Vec<String>,
    pub stages: Vec<PlanStage>,
    pub one_time_cost: CostRangeReceipt,
    pub annual_maintenance_cost: CostRangeReceipt,
    /// Capital plus linearly prorated annual maintenance across the declared horizon.
    pub lifecycle_cost_over_horizon: CostRangeReceipt,
    /// Critical-path upper bound, with independent interventions in a stage assumed parallel.
    pub lead_time_days_upper: u64,
    pub resident_disruption_points: u64,
    pub unmet_preference_ids: Vec<String>,
    pub conflicted_preference_ids: Vec<String>,
    pub preference_penalty_count: usize,
    pub hard_constraint_coverage: Vec<ConstraintCoverage>,
    pub planned_remediation_not_verified_count: usize,
    pub implementation_states: BTreeMap<String, DeliveryState>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct QuoteRequiredCandidate {
    pub selected_intervention_ids: Vec<String>,
    pub unknown_cost_intervention_ids: Vec<String>,
    pub known_cost_component_high_minor_units: u64,
    pub budget_status: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PortfolioReport {
    pub report_schema_version: String,
    pub evaluator_version: String,
    pub portfolio_id: String,
    pub fixture_kind: String,
    pub currency: String,
    pub planning_horizon_months: u16,
    pub input_sha256: String,
    pub intervention_count: usize,
    pub evaluated_subset_count: u64,
    pub hard_constraint_coverage_candidate_count: u64,
    pub dependency_valid_candidate_count: u64,
    pub access_gate_blocked_candidate_count: u64,
    pub denied_permission_candidate_count: u64,
    pub pending_permission_candidate_count: u64,
    pub quote_required_candidate_count: u64,
    pub quote_required_examples: Vec<QuoteRequiredCandidate>,
    pub over_budget_candidate_count: u64,
    pub budget_uncertain_candidate_count: u64,
    pub within_budget_candidate_count: u64,
    pub pareto_frontier: Vec<CandidatePlan>,
    pub claim_limit: String,
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && (value.as_bytes()[0].is_ascii_lowercase() || value.as_bytes()[0].is_ascii_digit())
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
        })
        && !value.ends_with('-')
        && !value.contains("--")
}

fn parse_timestamp(value: &str, field: &str, errors: &mut Vec<String>) {
    if DateTime::parse_from_rfc3339(value).is_err() {
        errors.push(format!("{field} must be a valid RFC 3339 timestamp"));
    }
}

fn valid_synthetic_ref(value: &str) -> bool {
    value.strip_prefix("synthetic://").is_some_and(|tail| {
        !tail.is_empty()
            && tail.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'/' | b'_' | b'-')
            })
    })
}

fn add_id(id: &str, kind: &str, all: &mut BTreeSet<String>, errors: &mut Vec<String>) {
    if !valid_id(id) {
        errors.push(format!("{kind} ID {id:?} is not a stable lowercase identifier"));
    }
    if !all.insert(id.to_owned()) {
        errors.push(format!("duplicate ID {id:?} across portfolio entities"));
    }
}

fn validate(input: &PortfolioInput) -> Vec<String> {
    let mut errors = Vec::new();
    if input.schema_version != "0.1.0" {
        errors.push(format!("unsupported schema_version {:?}", input.schema_version));
    }
    if input.fixture_kind != "synthetic_intervention_portfolio" {
        errors.push("only synthetic_intervention_portfolio inputs are accepted".into());
    }
    if !input.claim_boundary.to_ascii_lowercase().contains("synthetic")
        || !input.claim_boundary.to_ascii_lowercase().contains("not")
    {
        errors.push("claim_boundary must explicitly state the synthetic-only limit".into());
    }
    if input.currency.len() != 3 || !input.currency.bytes().all(|byte| byte.is_ascii_uppercase()) {
        errors.push("currency must be a three-letter uppercase code".into());
    }
    if input.planning_horizon_months == 0 {
        errors.push("planning_horizon_months must be positive".into());
    }
    if input.interventions.is_empty() {
        errors.push("at least one intervention is required".into());
    }
    if input.interventions.len() > MAX_INTERVENTIONS {
        errors.push(format!(
            "intervention count {} exceeds exhaustive-search limit {MAX_INTERVENTIONS}",
            input.interventions.len()
        ));
    }
    if input.hard_constraints.is_empty() {
        errors.push("at least one hard constraint is required".into());
    }

    let mut all_ids = BTreeSet::new();
    for constraint in &input.hard_constraints {
        add_id(&constraint.id, "hard constraint", &mut all_ids, &mut errors);
        if constraint.statement.trim().is_empty() || constraint.note.trim().is_empty() {
            errors.push(format!("hard constraint {:?} must include statement and note", constraint.id));
        }
    }
    let hard_ids: BTreeSet<&str> = input.hard_constraints.iter().map(|item| item.id.as_str()).collect();

    for preference in &input.preferences {
        add_id(&preference.id, "preference", &mut all_ids, &mut errors);
        if preference.statement.trim().is_empty() || preference.note.trim().is_empty() {
            errors.push(format!("preference {:?} must include statement and note", preference.id));
        }
    }
    let preference_ids: BTreeSet<&str> = input.preferences.iter().map(|item| item.id.as_str()).collect();
    let intervention_by_id: BTreeMap<&str, &Intervention> = input.interventions.iter()
        .map(|intervention| (intervention.id.as_str(), intervention))
        .collect();

    let mut intervention_ids = BTreeSet::new();
    let mut addresses: BTreeMap<&str, usize> = BTreeMap::new();
    for intervention in &input.interventions {
        add_id(&intervention.id, "intervention", &mut all_ids, &mut errors);
        intervention_ids.insert(intervention.id.as_str());
        if intervention.title.trim().is_empty() || intervention.note.trim().is_empty() {
            errors.push(format!("intervention {:?} must include title and note", intervention.id));
        }
        if intervention.lead_time_days.low > intervention.lead_time_days.high {
            errors.push(format!("intervention {:?} lead-time lower bound exceeds upper bound", intervention.id));
        }
        if intervention.resident_disruption_points > 5 {
            errors.push(format!("intervention {:?} disruption points must be in 0..=5", intervention.id));
        }
        let mut unique_deps = BTreeSet::new();
        for dep in &intervention.depends_on {
            if !unique_deps.insert(dep.as_str()) {
                errors.push(format!("intervention {:?} repeats dependency {:?}", intervention.id, dep));
            }
            if dep == &intervention.id {
                errors.push(format!("intervention {:?} cannot depend on itself", intervention.id));
            }
        }
        let mut unique_hard = BTreeSet::new();
        for id in &intervention.addresses_hard_constraint_ids {
            if !unique_hard.insert(id.as_str()) {
                errors.push(format!("intervention {:?} repeats hard constraint {:?}", intervention.id, id));
            }
            if !hard_ids.contains(id.as_str()) {
                errors.push(format!("intervention {:?} references unknown hard constraint {:?}", intervention.id, id));
            } else {
                *addresses.entry(id.as_str()).or_default() += 1;
            }
        }
        for id in intervention.supports_preference_ids.iter().chain(&intervention.conflicts_with_preference_ids) {
            if !preference_ids.contains(id.as_str()) {
                errors.push(format!("intervention {:?} references unknown preference {:?}", intervention.id, id));
            }
        }
        for resolution in &intervention.resolves_preference_conflicts {
            if !preference_ids.contains(resolution.preference_id.as_str()) {
                errors.push(format!("intervention {:?} resolves unknown preference {:?}", intervention.id, resolution.preference_id));
            }
            if !intervention.supports_preference_ids.contains(&resolution.preference_id) {
                errors.push(format!(
                    "intervention {:?} declares conflict resolution for {:?} without supporting that preference",
                    intervention.id, resolution.preference_id
                ));
            }
            match intervention_by_id.get(resolution.conflicting_intervention_id.as_str()) {
                Some(target) if target.conflicts_with_preference_ids.contains(&resolution.preference_id) => {}
                Some(_) => errors.push(format!(
                    "intervention {:?} resolution target {:?} does not declare a conflict with preference {:?}",
                    intervention.id, resolution.conflicting_intervention_id, resolution.preference_id
                )),
                None => errors.push(format!(
                    "intervention {:?} resolves conflict with unknown intervention {:?}",
                    intervention.id, resolution.conflicting_intervention_id
                )),
            }
        }
        for id in &intervention.supports_preference_ids {
            if intervention.conflicts_with_preference_ids.contains(id) {
                errors.push(format!(
                    "intervention {:?} both supports and conflicts with preference {:?}",
                    intervention.id, id
                ));
            }
        }
        match &intervention.cost {
            CostEvidence::KnownRange {
                basis,
                one_time_low_minor_units,
                one_time_high_minor_units,
                annual_maintenance_low_minor_units,
                annual_maintenance_high_minor_units,
                source_ref,
                observed_at,
            } => {
                if one_time_low_minor_units > one_time_high_minor_units {
                    errors.push(format!("intervention {:?} capital-cost lower bound exceeds upper bound", intervention.id));
                }
                if annual_maintenance_low_minor_units > annual_maintenance_high_minor_units {
                    errors.push(format!("intervention {:?} maintenance-cost lower bound exceeds upper bound", intervention.id));
                }
                if !valid_synthetic_ref(source_ref) {
                    errors.push(format!("intervention {:?} cost source must be synthetic:// in this prototype", intervention.id));
                }
                parse_timestamp(observed_at, &format!("intervention {} cost observed_at", intervention.id), &mut errors);
                if *basis == CostBasis::Quote && intervention.delivery_state.rank() < DeliveryState::QuoteReceived.rank() {
                    errors.push(format!("intervention {:?} has quote basis but delivery_state precedes quote_received", intervention.id));
                }
            }
            CostEvidence::Unknown { reason, source_ref, observed_at, .. } => {
                if reason.trim().is_empty() {
                    errors.push(format!("intervention {:?} unknown cost requires a reason", intervention.id));
                }
                if !valid_synthetic_ref(source_ref) {
                    errors.push(format!("intervention {:?} cost source must be synthetic:// in this prototype", intervention.id));
                }
                parse_timestamp(observed_at, &format!("intervention {} cost observed_at", intervention.id), &mut errors);
            }
        }
    }

    for constraint in &input.hard_constraints {
        if constraint.baseline_status != BaselineStatus::SatisfiedForFixture
            && !addresses.contains_key(constraint.id.as_str())
        {
            errors.push(format!(
                "unresolved hard constraint {:?} has no declared remediation option",
                constraint.id
            ));
        }
    }

    for intervention in &input.interventions {
        for dep in &intervention.depends_on {
            if !intervention_ids.contains(dep.as_str()) {
                errors.push(format!("intervention {:?} depends on unknown intervention {:?}", intervention.id, dep));
            }
        }
    }

    errors.sort();
    errors.dedup();
    errors
}

fn topological_order(input: &PortfolioInput) -> Result<Vec<usize>, Vec<String>> {
    let index_by_id: BTreeMap<&str, usize> = input.interventions.iter().enumerate()
        .map(|(i, intervention)| (intervention.id.as_str(), i))
        .collect();
    let mut done = BTreeSet::new();
    let mut order = Vec::new();
    while order.len() < input.interventions.len() {
        let mut ready: Vec<usize> = (0..input.interventions.len())
            .filter(|i| !done.contains(i))
            .filter(|i| {
                input.interventions[*i].depends_on.iter().all(|dep| {
                    index_by_id.get(dep.as_str()).is_some_and(|dep_index| done.contains(dep_index))
                })
            })
            .collect();
        ready.sort_by_key(|i| input.interventions[*i].id.as_str());
        if ready.is_empty() {
            return Err(vec!["intervention dependency graph contains a cycle".into()]);
        }
        let next = ready[0];
        done.insert(next);
        order.push(next);
    }
    Ok(order)
}

fn has_all_hard_coverage(input: &PortfolioInput, selected: &BTreeSet<usize>) -> bool {
    input.hard_constraints.iter().all(|constraint| {
        constraint.baseline_status == BaselineStatus::SatisfiedForFixture
            || selected.iter().any(|index| {
                input.interventions[*index].addresses_hard_constraint_ids.contains(&constraint.id)
            })
    })
}

fn dependency_closed(input: &PortfolioInput, selected: &BTreeSet<usize>) -> bool {
    let index_by_id: BTreeMap<&str, usize> = input.interventions.iter().enumerate()
        .map(|(i, intervention)| (intervention.id.as_str(), i))
        .collect();
    selected.iter().all(|index| {
        input.interventions[*index].depends_on.iter().all(|dep| {
            index_by_id.get(dep.as_str()).is_some_and(|dep_index| selected.contains(dep_index))
        })
    })
}

fn selected_ids(input: &PortfolioInput, selected: &BTreeSet<usize>, order: &[usize]) -> Vec<String> {
    order.iter()
        .filter(|index| selected.contains(*index))
        .map(|index| input.interventions[*index].id.clone())
        .collect()
}

fn sum_cost(
    input: &PortfolioInput,
    selected: &BTreeSet<usize>,
) -> Result<(CostRangeReceipt, CostRangeReceipt), Vec<String>> {
    let mut one_time_low = 0u64;
    let mut one_time_high = 0u64;
    let mut annual_low = 0u64;
    let mut annual_high = 0u64;
    for index in selected {
        match &input.interventions[*index].cost {
            CostEvidence::KnownRange {
                one_time_low_minor_units,
                one_time_high_minor_units,
                annual_maintenance_low_minor_units,
                annual_maintenance_high_minor_units,
                ..
            } => {
                one_time_low = one_time_low.checked_add(*one_time_low_minor_units).ok_or_else(|| vec!["capital-cost sum overflow".into()])?;
                one_time_high = one_time_high.checked_add(*one_time_high_minor_units).ok_or_else(|| vec!["capital-cost sum overflow".into()])?;
                annual_low = annual_low.checked_add(*annual_maintenance_low_minor_units).ok_or_else(|| vec!["maintenance-cost sum overflow".into()])?;
                annual_high = annual_high.checked_add(*annual_maintenance_high_minor_units).ok_or_else(|| vec!["maintenance-cost sum overflow".into()])?;
            }
            CostEvidence::Unknown { .. } => return Err(vec!["cannot total unknown costs as zero".into()]),
        }
    }
    Ok((
        CostRangeReceipt { low_minor_units: one_time_low, high_minor_units: one_time_high },
        CostRangeReceipt { low_minor_units: annual_low, high_minor_units: annual_high },
    ))
}

fn permission_counts(input: &PortfolioInput, selected: &BTreeSet<usize>) -> (bool, u64, u64) {
    let mut pending = 0;
    let mut denied = 0;
    for index in selected {
        match input.interventions[*index].access_gate {
            AccessGate::NotRequired | AccessGate::Cleared => {}
            AccessGate::Pending => pending += 1,
            AccessGate::Denied => denied += 1,
        }
    }
    (pending == 0 && denied == 0, pending, denied)
}

fn build_plan(
    input: &PortfolioInput,
    selected: &BTreeSet<usize>,
    order: &[usize],
    topo_rank: &BTreeMap<usize, usize>,
) -> Result<CandidatePlan, Vec<String>> {
    let mut selected_indices: Vec<usize> = selected.iter().copied().collect();
    selected_indices.sort_by_key(|index| topo_rank[index]);
    let selected_intervention_ids = selected_ids(input, selected, order);
    let (one_time_cost, annual_maintenance_cost) = sum_cost(input, selected)?;
    let horizon_months = u64::from(input.planning_horizon_months);
    let maintenance_horizon_low = annual_maintenance_cost.low_minor_units
        .checked_mul(horizon_months)
        .ok_or_else(|| vec!["lifecycle maintenance cost overflow".into()])? / 12;
    let maintenance_horizon_high_numerator = annual_maintenance_cost.high_minor_units
        .checked_mul(horizon_months)
        .and_then(|value| value.checked_add(11))
        .ok_or_else(|| vec!["lifecycle maintenance cost overflow".into()])?;
    let maintenance_horizon_high = maintenance_horizon_high_numerator / 12;
    let lifecycle_cost_over_horizon = CostRangeReceipt {
        low_minor_units: one_time_cost.low_minor_units.checked_add(maintenance_horizon_low)
            .ok_or_else(|| vec!["lifecycle lower-cost sum overflow".into()])?,
        high_minor_units: one_time_cost.high_minor_units.checked_add(maintenance_horizon_high)
            .ok_or_else(|| vec!["lifecycle upper-cost sum overflow".into()])?,
    };

    let mut stage_of: BTreeMap<usize, usize> = BTreeMap::new();
    let mut end_days: BTreeMap<usize, u64> = BTreeMap::new();
    let mut stages: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    let mut lead_time_days_upper = 0u64;
    let mut resident_disruption_points = 0u64;
    let mut implementation_states = BTreeMap::new();
    for index in &selected_indices {
        let intervention = &input.interventions[*index];
        let stage = intervention.depends_on.iter()
            .filter_map(|dep| {
                input.interventions.iter().position(|other| other.id.as_str() == dep.as_str())
                    .and_then(|dep_index| stage_of.get(&dep_index).map(|value| *value + 1))
            })
            .max()
            .unwrap_or(0);
        let previous_end = intervention.depends_on.iter()
            .filter_map(|dep| {
                input.interventions.iter().position(|other| other.id.as_str() == dep.as_str())
                    .and_then(|dep_index| end_days.get(&dep_index).copied())
            })
            .max()
            .unwrap_or(0);
        let end = previous_end.checked_add(u64::from(intervention.lead_time_days.high))
            .ok_or_else(|| vec!["lead-time sum overflow".into()])?;
        stage_of.insert(*index, stage);
        end_days.insert(*index, end);
        stages.entry(stage).or_default().push(intervention.id.clone());
        lead_time_days_upper = lead_time_days_upper.max(end);
        resident_disruption_points = resident_disruption_points
            .checked_add(u64::from(intervention.resident_disruption_points))
            .ok_or_else(|| vec!["disruption score overflow".into()])?;
        implementation_states.insert(intervention.id.clone(), intervention.delivery_state);
    }
    let stages = stages.into_iter().map(|(stage, mut ids)| {
        ids.sort();
        PlanStage { stage: stage + 1, intervention_ids: ids }
    }).collect();

    let mut unmet_preference_ids = Vec::new();
    let mut conflicted_preference_ids = Vec::new();
    for preference in &input.preferences {
        if preference.baseline_status == BaselineStatus::SatisfiedForFixture {
            continue;
        }
        let conflicting_intervention_ids: Vec<&str> = selected.iter()
            .filter_map(|index| {
                let intervention = &input.interventions[*index];
                intervention.conflicts_with_preference_ids.contains(&preference.id)
                    .then_some(intervention.id.as_str())
            })
            .collect();
        let unresolved_conflict = conflicting_intervention_ids.iter().any(|conflicting_id| {
            !selected.iter().any(|index| {
                input.interventions[*index].resolves_preference_conflicts.iter().any(|resolution| {
                    resolution.preference_id.as_str() == preference.id.as_str()
                        && resolution.conflicting_intervention_id.as_str() == *conflicting_id
                })
            })
        });
        if unresolved_conflict {
            conflicted_preference_ids.push(preference.id.clone());
            continue;
        }
        let supported = selected.iter().any(|index| {
            input.interventions[*index].supports_preference_ids.contains(&preference.id)
        });
        if !supported {
            unmet_preference_ids.push(preference.id.clone());
        }
    }
    let preference_penalty_count = unmet_preference_ids.len() + conflicted_preference_ids.len();

    let mut hard_constraint_coverage = Vec::new();
    let mut planned_remediation_not_verified_count = 0;
    for constraint in &input.hard_constraints {
        let mut selected_remediation_ids: Vec<String> = selected.iter()
            .filter(|index| input.interventions[**index].addresses_hard_constraint_ids.contains(&constraint.id))
            .map(|index| input.interventions[*index].id.clone())
            .collect();
        selected_remediation_ids.sort();
        let disposition = if constraint.baseline_status == BaselineStatus::SatisfiedForFixture {
            "baseline_satisfied_for_fixture"
        } else {
            planned_remediation_not_verified_count += 1;
            "planned_remediation_not_verified"
        };
        hard_constraint_coverage.push(ConstraintCoverage {
            constraint_id: constraint.id.clone(),
            disposition: disposition.to_owned(),
            selected_remediation_ids,
        });
    }

    Ok(CandidatePlan {
        selected_intervention_ids,
        stages,
        one_time_cost,
        annual_maintenance_cost,
        lifecycle_cost_over_horizon,
        lead_time_days_upper,
        resident_disruption_points,
        unmet_preference_ids,
        conflicted_preference_ids,
        preference_penalty_count,
        hard_constraint_coverage,
        planned_remediation_not_verified_count,
        implementation_states,
    })
}

fn dominates(a: &CandidatePlan, b: &CandidatePlan) -> bool {
    let av = (
        a.one_time_cost.high_minor_units,
        a.lifecycle_cost_over_horizon.high_minor_units,
        a.lead_time_days_upper,
        a.resident_disruption_points,
        a.preference_penalty_count,
    );
    let bv = (
        b.one_time_cost.high_minor_units,
        b.lifecycle_cost_over_horizon.high_minor_units,
        b.lead_time_days_upper,
        b.resident_disruption_points,
        b.preference_penalty_count,
    );
    av.0 <= bv.0 && av.1 <= bv.1 && av.2 <= bv.2 && av.3 <= bv.3 && av.4 <= bv.4
        && (av.0 < bv.0 || av.1 < bv.1 || av.2 < bv.2 || av.3 < bv.3 || av.4 < bv.4)
}

fn plan_sort_key(plan: &CandidatePlan) -> (u64, u64, u64, u64, usize, String) {
    (
        plan.one_time_cost.high_minor_units,
        plan.lifecycle_cost_over_horizon.high_minor_units,
        plan.lead_time_days_upper,
        plan.resident_disruption_points,
        plan.preference_penalty_count,
        plan.selected_intervention_ids.join(","),
    )
}

pub fn evaluate_bytes(bytes: &[u8]) -> Result<PortfolioReport, Vec<String>> {
    let input: PortfolioInput = serde_json::from_slice(bytes)
        .map_err(|error| vec![format!("invalid portfolio JSON/shape: {error}")])?;
    let errors = validate(&input);
    if !errors.is_empty() {
        return Err(errors);
    }
    let order = topological_order(&input)?;
    let topo_rank: BTreeMap<usize, usize> = order.iter().enumerate().map(|(rank, index)| (*index, rank)).collect();
    let intervention_count = input.interventions.len();
    let evaluated_subset_count = 1u64 << intervention_count;
    let mut hard_constraint_coverage_candidate_count = 0u64;
    let mut dependency_valid_candidate_count = 0u64;
    let mut access_gate_blocked_candidate_count = 0u64;
    let mut denied_permission_candidate_count = 0u64;
    let mut pending_permission_candidate_count = 0u64;
    let mut quote_required_candidate_count = 0u64;
    let mut quote_required_examples = Vec::new();
    let mut over_budget_candidate_count = 0u64;
    let mut budget_uncertain_candidate_count = 0u64;
    let mut within_budget_candidate_count = 0u64;
    let mut pareto_frontier: Vec<CandidatePlan> = Vec::new();

    for mask in 0..evaluated_subset_count {
        let selected: BTreeSet<usize> = (0..intervention_count)
            .filter(|index| mask & (1u64 << index) != 0)
            .collect();
        if !has_all_hard_coverage(&input, &selected) {
            continue;
        }
        hard_constraint_coverage_candidate_count += 1;
        if !dependency_closed(&input, &selected) {
            continue;
        }
        dependency_valid_candidate_count += 1;
        let (permission_cleared, pending, denied) = permission_counts(&input, &selected);
        if !permission_cleared {
            access_gate_blocked_candidate_count += 1;
            if pending > 0 { pending_permission_candidate_count += 1; }
            if denied > 0 { denied_permission_candidate_count += 1; }
            continue;
        }

        let unknown_cost_ids: Vec<String> = selected.iter()
            .filter(|index| matches!(&input.interventions[**index].cost, CostEvidence::Unknown { .. }))
            .map(|index| input.interventions[*index].id.clone())
            .collect();
        if !unknown_cost_ids.is_empty() {
            quote_required_candidate_count += 1;
            if quote_required_examples.len() < MAX_QUOTE_EXAMPLES {
                let known_high = selected.iter().try_fold(0u64, |sum, index| {
                    match &input.interventions[*index].cost {
                        CostEvidence::KnownRange { one_time_high_minor_units, .. } => sum.checked_add(*one_time_high_minor_units),
                        CostEvidence::Unknown { .. } => Some(sum),
                    }
                }).ok_or_else(|| vec!["known component cost sum overflow".into()])?;
                quote_required_examples.push(QuoteRequiredCandidate {
                    selected_intervention_ids: selected_ids(&input, &selected, &order),
                    unknown_cost_intervention_ids: unknown_cost_ids,
                    known_cost_component_high_minor_units: known_high,
                    budget_status: "not_evaluable_until_all_required_costs_are_known".into(),
                });
            }
            continue;
        }

        let (capital, _) = sum_cost(&input, &selected)?;
        if capital.low_minor_units > input.budget_cap_upper_minor_units {
            over_budget_candidate_count += 1;
            continue;
        }
        if capital.high_minor_units > input.budget_cap_upper_minor_units {
            budget_uncertain_candidate_count += 1;
            continue;
        }
        within_budget_candidate_count += 1;
        let candidate = build_plan(&input, &selected, &order, &topo_rank)?;
        if !pareto_frontier.iter().any(|existing| dominates(existing, &candidate)) {
            pareto_frontier.retain(|existing| !dominates(&candidate, existing));
            pareto_frontier.push(candidate);
            if pareto_frontier.len() > MAX_FRONTIER_SIZE {
                return Err(vec![format!(
                    "Pareto frontier exceeds safety cap {MAX_FRONTIER_SIZE}; no truncated report was emitted"
                )]);
            }
        }
    }

    pareto_frontier.sort_by_key(plan_sort_key);

    let digest = Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(PortfolioReport {
        report_schema_version: "0.1.0".into(),
        evaluator_version: EVALUATOR_VERSION.into(),
        portfolio_id: input.portfolio_id,
        fixture_kind: input.fixture_kind,
        currency: input.currency,
        planning_horizon_months: input.planning_horizon_months,
        input_sha256: digest,
        intervention_count,
        evaluated_subset_count,
        hard_constraint_coverage_candidate_count,
        dependency_valid_candidate_count,
        access_gate_blocked_candidate_count,
        denied_permission_candidate_count,
        pending_permission_candidate_count,
        quote_required_candidate_count,
        quote_required_examples,
        over_budget_candidate_count,
        budget_uncertain_candidate_count,
        within_budget_candidate_count,
        pareto_frontier,
        claim_limit: "Synthetic planning consistency only. Coverage means a proposed intervention is mapped to a declared constraint; it does not mean the intervention works, the constraint is currently satisfied, a quote is valid, or the home is accessible or safe. Real work requires resident direction, attributable current prices, site measurement and qualified local review.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).expect("fixture JSON parses")
    }
    fn eval_value(value: &Value) -> Result<PortfolioReport, Vec<String>> {
        evaluate_bytes(&serde_json::to_vec(value).expect("fixture serializes"))
    }
    fn contains_error(errors: &[String], pattern: &str) -> bool {
        errors.iter().any(|error| error.contains(pattern))
    }

    #[test]
    fn committed_fixture_generates_frontier_and_unknown_quote_candidates() {
        let report = evaluate_bytes(FIXTURE.as_bytes()).expect("synthetic portfolio validates");
        assert_eq!(report.intervention_count, 5);
        assert_eq!(report.evaluated_subset_count, 32);
        assert!(!report.pareto_frontier.is_empty());
        assert!(report.quote_required_candidate_count > 0);
        assert!(report.pending_permission_candidate_count > 0);
    }

    #[test]
    fn hard_constraints_are_never_traded_against_cost() {
        let report = evaluate_bytes(FIXTURE.as_bytes()).expect("fixture validates");
        for plan in &report.pareto_frontier {
            let offline = plan.hard_constraint_coverage.iter()
                .find(|item| item.constraint_id == "offline-notification-path")
                .expect("offline constraint reported");
            assert_eq!(offline.disposition, "planned_remediation_not_verified");
            assert!(!offline.selected_remediation_ids.is_empty());
        }
        assert!(report.pareto_frontier.iter().all(|p| p.one_time_cost.high_minor_units <= 1_250_000));
    }

    #[test]
    fn unknown_cost_never_enters_budget_frontier_as_zero() {
        let report = evaluate_bytes(FIXTURE.as_bytes()).expect("fixture validates");
        assert!(report.quote_required_candidate_count > 0);
        assert!(report.quote_required_examples.iter().all(|candidate| {
            !candidate.unknown_cost_intervention_ids.is_empty()
                && candidate.budget_status == "not_evaluable_until_all_required_costs_are_known"
        }));
        assert!(report.pareto_frontier.iter().all(|plan| {
            plan.selected_intervention_ids.iter().all(|id| id != "unknown-cost-doorbell")
        }));
    }

    #[test]
    fn preference_conflict_does_not_become_a_hard_constraint() {
        let report = evaluate_bytes(FIXTURE.as_bytes()).expect("fixture validates");
        assert!(report.pareto_frontier.iter().any(|plan| {
            plan.selected_intervention_ids == vec!["battery-backed-local-alert".to_owned()]
                && plan.conflicted_preference_ids.contains(&"tactile-notification".to_owned())
        }));
        assert!(report.pareto_frontier.iter().any(|plan| {
            plan.selected_intervention_ids.iter().any(|id| id == "battery-backed-local-alert")
                && plan.selected_intervention_ids.iter().any(|id| id == "tactile-haptic-receiver")
                && plan.conflicted_preference_ids.is_empty()
                && plan.preference_penalty_count == 0
        }));
    }

    #[test]
    fn lifecycle_cost_uses_declared_horizon() {
        let value = fixture();
        let report = eval_value(&value).expect("fixture validates");
        let combined = report.pareto_frontier.iter().find(|plan| {
            plan.selected_intervention_ids.iter().any(|id| id == "battery-backed-local-alert")
                && plan.selected_intervention_ids.iter().any(|id| id == "tactile-haptic-receiver")
        }).expect("combined plan retained on Pareto frontier");
        assert_eq!(report.planning_horizon_months, 12);
        assert_eq!(combined.lifecycle_cost_over_horizon.low_minor_units, 990_000);
        assert_eq!(combined.lifecycle_cost_over_horizon.high_minor_units, 1_265_000);
    }

    #[test]
    fn budget_range_crossing_cap_is_uncertain_not_robustly_affordable() {
        let mut value = fixture();
        value["budget_cap_upper_minor_units"] = json!(1_000_000);
        let report = eval_value(&value).expect("fixture validates");
        assert!(report.budget_uncertain_candidate_count > 0);
        assert!(report.pareto_frontier.iter().all(|plan| {
            plan.one_time_cost.high_minor_units <= 1_000_000
        }));
    }

    #[test]
    fn pending_permission_options_are_not_budget_feasible_plans() {
        let report = evaluate_bytes(FIXTURE.as_bytes()).expect("fixture validates");
        assert!(report.pending_permission_candidate_count > 0);
        assert!(report.pareto_frontier.iter().all(|plan| {
            !plan.selected_intervention_ids.contains(&"landlord-fixed-wiring".to_owned())
        }));
    }

    #[test]
    fn frontier_contains_no_strictly_dominated_plan() {
        let report = evaluate_bytes(FIXTURE.as_bytes()).expect("fixture validates");
        for (i, candidate) in report.pareto_frontier.iter().enumerate() {
            for (j, other) in report.pareto_frontier.iter().enumerate() {
                if i != j {
                    assert!(!dominates(other, candidate));
                }
            }
        }
    }

    #[test]
    fn identical_input_produces_identical_report() {
        let first = evaluate_bytes(FIXTURE.as_bytes()).expect("first replay");
        let second = evaluate_bytes(FIXTURE.as_bytes()).expect("second replay");
        assert_eq!(first, second);
    }

    #[test]
    fn dependency_cycle_fails_closed() {
        let mut value = fixture();
        value["interventions"][0]["depends_on"] = json!(["tactile-haptic-receiver"]);
        value["interventions"][1]["depends_on"] = json!(["offline-tactile-unit"]);
        let errors = eval_value(&value).expect_err("cycle must fail");
        assert!(contains_error(&errors, "cycle"));
    }

    #[test]
    fn unknown_dependency_fails_closed() {
        let mut value = fixture();
        value["interventions"][0]["depends_on"] = json!(["missing-intervention"]);
        let errors = eval_value(&value).expect_err("unknown dependency must fail");
        assert!(contains_error(&errors, "unknown intervention"));
    }

    #[test]
    fn inverted_cost_range_fails_closed() {
        let mut value = fixture();
        value["interventions"][0]["cost"]["one_time_low_minor_units"] = json!(1_100_000);
        value["interventions"][0]["cost"]["one_time_high_minor_units"] = json!(900_000);
        let errors = eval_value(&value).expect_err("inverted quote fails");
        assert!(contains_error(&errors, "capital-cost lower bound"));
    }

    #[test]
    fn missing_hard_constraint_remediation_is_rejected() {
        let mut value = fixture();
        for intervention in value["interventions"].as_array_mut().unwrap() {
            if let Some(ids) = intervention["addresses_hard_constraint_ids"].as_array_mut() {
                ids.retain(|id| id.as_str() != Some("offline-notification-path"));
            }
        }
        let errors = eval_value(&value).expect_err("unresolved hard constraint has no remediation");
        assert!(contains_error(&errors, "no declared remediation option"));
    }

    #[test]
    fn subset_search_limit_is_enforced() {
        let mut value = fixture();
        let template = value["interventions"][0].clone();
        for index in 0..16 {
            let mut item = template.clone();
            item["id"] = json!(format!("extra-intervention-{index:02}"));
            item["title"] = json!("Synthetic bounded-search test intervention");
            item["depends_on"] = json!([]);
            value["interventions"].as_array_mut().unwrap().push(item);
        }
        let errors = eval_value(&value).expect_err("too many interventions must fail closed");
        assert!(contains_error(&errors, "exceeds exhaustive-search limit"));
    }

    #[test]
    fn exact_input_digest_changes_when_fixture_bytes_change() {
        let report = evaluate_bytes(FIXTURE.as_bytes()).expect("fixture validates");
        let mut value = fixture();
        value["claim_boundary"] = json!("Synthetic fixture altered for digest mutation, not a real home assessment.");
        let changed = eval_value(&value).expect("altered synthetic fixture valid");
        assert_ne!(report.input_sha256, changed.input_sha256);
    }
}
