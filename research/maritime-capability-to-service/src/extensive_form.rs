//! Bounded extensive-form R1 policy enumeration over frozen scenario paths.
//!
//! This v0.1 reference enumerates one action per observable information set,
//! replays additive stage contributions over exogenous scenario paths, and
//! returns a vector-valued Pareto frontier. It does not model action-dependent
//! state transitions or constitute a live port simulator.

use std::collections::{BTreeMap, BTreeSet};

use crate::kite::Gate;
use crate::kite_oracle::{AdverseEnvelope, DecisionContext, OutcomeVector, adverse_envelope};
use crate::policy_tree::PolicyInformationSetKey;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceUseContribution {
    pub crane_minutes: u64,
    pub shore_power_wh: u64,
    pub yard_slot_hours: u64,
    pub labor_hours: u64,
    pub freshwater_litres: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StageOutcomeContribution {
    pub receiver_accepted_units: u64,
    pub on_time_accepted_units: u64,
    pub acceptance_time_unknown_units: u64,
    /// Horizon residual, normally recorded only on the final stage.
    pub unmet_units: u64,
    /// Horizon unresolved residual, normally recorded only on the final stage.
    pub unresolved_units: u64,
    pub late_accepted_units: u64,
    pub protected_floor_breach_unit_hours: u64,
    pub service_deficit_unit_hours: u64,
    /// Absolute time from the common disruption epoch, if first attained here.
    pub time_to_minimum_viable_service: Option<u64>,
    /// Absolute time from the common disruption epoch, if first attained here.
    pub time_to_full_service: Option<u64>,
    pub recovery_cost_units: u64,
    pub emissions_units: u64,
    pub requalification_actions: u64,
    pub replanning_invocations: u64,
    pub plan_churn: u64,
    pub decision_latency_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyActionAlternative {
    pub action_id: String,
    pub admissibility: Gate,
    pub hard_constraints_passed: bool,
    pub outcome: StageOutcomeContribution,
    pub resource_use: ResourceUseContribution,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyDecisionStage {
    pub decision_epoch: u64,
    pub decision_cutoff: u64,
    /// Commitment to the full canonical observable state at this information set.
    pub observable_state_digest: String,
    pub alternatives: Vec<PolicyActionAlternative>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyScenarioPath {
    pub scenario_id: String,
    pub context: DecisionContext,
    pub stages: Vec<PolicyDecisionStage>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceUseVector {
    pub crane_minutes: u64,
    pub shore_power_wh: u64,
    pub yard_slot_hours: u64,
    pub labor_hours: u64,
    pub freshwater_litres: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ResourceDimension {
    CraneMinutes,
    ShorePowerWh,
    YardSlotHours,
    LaborHours,
    FreshwaterLitres,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceAdverseEnvelope {
    pub bounds: ResourceUseVector,
    pub worst_scenario_by_dimension: BTreeMap<ResourceDimension, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensiveFormPolicy {
    pub choices: BTreeMap<PolicyInformationSetKey, String>,
    pub service_adverse_envelope: AdverseEnvelope,
    pub resource_adverse_envelope: ResourceAdverseEnvelope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensiveFormOracleResult {
    pub scenario_count: usize,
    pub stage_decision_count: usize,
    pub information_set_count: usize,
    pub candidate_policy_count: u64,
    pub evaluated_policy_count: u64,
    pub pareto_frontier: Vec<ExtensiveFormPolicy>,
}

#[derive(Clone, Debug)]
struct InformationGroup {
    key: PolicyInformationSetKey,
    common_actions: Vec<String>,
}

fn is_sha256_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn key_for(stage: &PolicyDecisionStage) -> PolicyInformationSetKey {
    PolicyInformationSetKey {
        decision_epoch: stage.decision_epoch,
        decision_cutoff: stage.decision_cutoff,
        observable_state_digest: stage.observable_state_digest.clone(),
    }
}

fn add_u64(target: &mut u64, increment: u64, field: &'static str) -> Result<(), String> {
    *target = target
        .checked_add(increment)
        .ok_or_else(|| format!("cumulative {field} overflow"))?;
    Ok(())
}

fn validate_stage_alternative(
    scenario_id: &str,
    stage: &PolicyDecisionStage,
    action: &PolicyActionAlternative,
    expected_units: u64,
) -> Result<(), String> {
    if action.action_id.trim().is_empty() {
        return Err(format!(
            "{scenario_id} at epoch {}: action ID must be non-empty",
            stage.decision_epoch
        ));
    }
    if action.outcome.receiver_accepted_units > expected_units
        || action.outcome.unmet_units > expected_units
        || action.outcome.unresolved_units > expected_units
    {
        return Err(format!(
            "{scenario_id} / {}: stage service quantity exceeds the fixed denominator",
            action.action_id
        ));
    }
    let classified_acceptance = action
        .outcome
        .on_time_accepted_units
        .checked_add(action.outcome.late_accepted_units)
        .and_then(|value| value.checked_add(action.outcome.acceptance_time_unknown_units))
        .ok_or_else(|| "stage accepted-unit classification overflow".to_owned())?;
    if classified_acceptance != action.outcome.receiver_accepted_units {
        return Err(format!(
            "{scenario_id} / {}: accepted units are not fully classified by timing",
            action.action_id
        ));
    }
    if action.admissibility == Gate::Yes
        && action.hard_constraints_passed
        && action.outcome.protected_floor_breach_unit_hours > 0
    {
        return Err(format!(
            "{scenario_id} / {}: feasible action conflicts with protected-floor accounting",
            action.action_id
        ));
    }
    if let (Some(minimum), Some(full)) = (
        action.outcome.time_to_minimum_viable_service,
        action.outcome.time_to_full_service,
    ) {
        if minimum > full {
            return Err(format!(
                "{scenario_id} / {}: minimum viable service cannot follow full service",
                action.action_id
            ));
        }
    }
    Ok(())
}

fn stage_schedule(
    stages: &[PolicyDecisionStage],
    scenario_id: &str,
) -> Result<Vec<(u64, u64)>, String> {
    if stages.is_empty() {
        return Err(format!("{scenario_id}: scenario path has no decision stages"));
    }
    let mut schedule = Vec::with_capacity(stages.len());
    let mut previous: Option<(u64, u64)> = None;
    for stage in stages {
        let current = (stage.decision_epoch, stage.decision_cutoff);
        if let Some(prior) = previous {
            if current.0 <= prior.0 {
                return Err(format!(
                    "{scenario_id}: decision epochs must be strictly increasing"
                ));
            }
            if current.1 <= prior.1 {
                return Err(format!(
                    "{scenario_id}: decision cutoffs must be strictly increasing"
                ));
            }
        }
        schedule.push(current);
        previous = Some(current);
    }
    Ok(schedule)
}

fn add_stage_outcome(
    totals: &mut OutcomeVector,
    resources: &mut ResourceUseVector,
    contribution: &StageOutcomeContribution,
    resource_use: &ResourceUseContribution,
) -> Result<(), String> {
    add_u64(
        &mut totals.receiver_accepted_units,
        contribution.receiver_accepted_units,
        "receiver accepted units",
    )?;
    add_u64(
        &mut totals.on_time_accepted_units,
        contribution.on_time_accepted_units,
        "on-time accepted units",
    )?;
    add_u64(
        &mut totals.acceptance_time_unknown_units,
        contribution.acceptance_time_unknown_units,
        "acceptance-time-unknown units",
    )?;
    add_u64(
        &mut totals.unmet_units,
        contribution.unmet_units,
        "unmet units",
    )?;
    add_u64(
        &mut totals.unresolved_units,
        contribution.unresolved_units,
        "unresolved units",
    )?;
    add_u64(
        &mut totals.late_accepted_units,
        contribution.late_accepted_units,
        "late accepted units",
    )?;
    add_u64(
        &mut totals.protected_floor_breach_unit_hours,
        contribution.protected_floor_breach_unit_hours,
        "protected-floor breach unit-hours",
    )?;
    add_u64(
        &mut totals.service_deficit_unit_hours,
        contribution.service_deficit_unit_hours,
        "service deficit unit-hours",
    )?;
    add_u64(
        &mut totals.recovery_cost_units,
        contribution.recovery_cost_units,
        "recovery cost",
    )?;
    add_u64(
        &mut totals.emissions_units,
        contribution.emissions_units,
        "emissions",
    )?;
    add_u64(
        &mut totals.requalification_actions,
        contribution.requalification_actions,
        "requalification actions",
    )?;
    add_u64(
        &mut totals.replanning_invocations,
        contribution.replanning_invocations,
        "replanning invocations",
    )?;
    add_u64(&mut totals.plan_churn, contribution.plan_churn, "plan churn")?;
    add_u64(
        &mut totals.decision_latency_ms,
        contribution.decision_latency_ms,
        "decision latency",
    )?;

    if let Some(value) = contribution.time_to_minimum_viable_service {
        totals.time_to_minimum_viable_service =
            totals.time_to_minimum_viable_service.min(value);
    }
    if let Some(value) = contribution.time_to_full_service {
        totals.time_to_full_service = totals.time_to_full_service.min(value);
    }

    add_u64(
        &mut resources.crane_minutes,
        resource_use.crane_minutes,
        "crane minutes",
    )?;
    add_u64(
        &mut resources.shore_power_wh,
        resource_use.shore_power_wh,
        "shore power Wh",
    )?;
    add_u64(
        &mut resources.yard_slot_hours,
        resource_use.yard_slot_hours,
        "yard slot-hours",
    )?;
    add_u64(
        &mut resources.labor_hours,
        resource_use.labor_hours,
        "labor hours",
    )?;
    add_u64(
        &mut resources.freshwater_litres,
        resource_use.freshwater_litres,
        "freshwater litres",
    )?;
    Ok(())
}

fn evaluate_policy(
    paths: &[PolicyScenarioPath],
    choices: &BTreeMap<PolicyInformationSetKey, String>,
) -> Result<(AdverseEnvelope, ResourceAdverseEnvelope), String> {
    let mut service_outcomes: Vec<(&str, OutcomeVector)> = Vec::with_capacity(paths.len());
    let mut resource_outcomes: Vec<(&str, ResourceUseVector)> = Vec::with_capacity(paths.len());

    for path in paths {
        let deadline = path.context.service_deadline;
        let not_reached = deadline
            .checked_add(1)
            .ok_or_else(|| "service deadline cannot represent horizon-plus-one".to_owned())?;
        let mut total = OutcomeVector {
            receiver_accepted_units: 0,
            on_time_accepted_units: 0,
            acceptance_time_unknown_units: 0,
            unmet_units: 0,
            late_accepted_units: 0,
            unresolved_units: 0,
            protected_floor_breach_unit_hours: 0,
            service_deficit_unit_hours: 0,
            time_to_minimum_viable_service: not_reached,
            time_to_full_service: not_reached,
            recovery_cost_units: 0,
            emissions_units: 0,
            requalification_actions: 0,
            replanning_invocations: 0,
            plan_churn: 0,
            decision_latency_ms: 0,
        };
        let mut resources = ResourceUseVector {
            crane_minutes: 0,
            shore_power_wh: 0,
            yard_slot_hours: 0,
            labor_hours: 0,
            freshwater_litres: 0,
        };

        for stage in &path.stages {
            let key = key_for(stage);
            let action_id = choices.get(&key).ok_or_else(|| {
                format!(
                    "policy omits information-set action for {} at epoch {}",
                    path.scenario_id, stage.decision_epoch
                )
            })?;
            let alternative = stage
                .alternatives
                .iter()
                .find(|alternative| alternative.action_id == *action_id)
                .ok_or_else(|| {
                    format!(
                        "{} at epoch {}: selected action '{}' is not available",
                        path.scenario_id, stage.decision_epoch, action_id
                    )
                })?;
            if alternative.admissibility != Gate::Yes
                || !alternative.hard_constraints_passed
                || alternative.outcome.protected_floor_breach_unit_hours > 0
            {
                return Err(format!(
                    "{} at epoch {}: selected action is not hard-feasible",
                    path.scenario_id, stage.decision_epoch
                ));
            }
            add_stage_outcome(
                &mut total,
                &mut resources,
                &alternative.outcome,
                &alternative.resource_use,
            )?;
        }

        let timing_classification = total
            .on_time_accepted_units
            .checked_add(total.late_accepted_units)
            .and_then(|value| value.checked_add(total.acceptance_time_unknown_units))
            .ok_or_else(|| "horizon accepted-unit classification overflow".to_owned())?;
        let service_classification = total
            .receiver_accepted_units
            .checked_add(total.unmet_units)
            .and_then(|value| value.checked_add(total.unresolved_units))
            .ok_or_else(|| "horizon service partition overflow".to_owned())?;

        if timing_classification != total.receiver_accepted_units
            || service_classification != path.context.expected_units
            || total.receiver_accepted_units > path.context.expected_units
        {
            return Err(format!(
                "{}: complete horizon service or acceptance-time partition is invalid",
                path.scenario_id
            ));
        }
        if total.time_to_minimum_viable_service == not_reached
            && total.time_to_full_service != not_reached
        {
            return Err(format!(
                "{}: full service cannot precede minimum viable service",
                path.scenario_id
            ));
        }
        if total.time_to_minimum_viable_service > total.time_to_full_service {
            return Err(format!(
                "{}: minimum viable service cannot follow full service",
                path.scenario_id
            ));
        }
        service_outcomes.push((path.scenario_id.as_str(), total));
        resource_outcomes.push((path.scenario_id.as_str(), resources));
    }

    Ok((
        adverse_envelope(&service_outcomes)?,
        resource_adverse_envelope(&resource_outcomes)?,
    ))
}

fn resource_adverse_envelope(
    outcomes: &[(&str, ResourceUseVector)],
) -> Result<ResourceAdverseEnvelope, String> {
    let Some((first_scenario, first)) = outcomes.first() else {
        return Err("cannot calculate empty resource envelope".to_owned());
    };
    let mut bounds = first.clone();
    let mut witnesses = BTreeMap::new();
    for dimension in [
        ResourceDimension::CraneMinutes,
        ResourceDimension::ShorePowerWh,
        ResourceDimension::YardSlotHours,
        ResourceDimension::LaborHours,
        ResourceDimension::FreshwaterLitres,
    ] {
        witnesses.insert(dimension, (*first_scenario).to_owned());
    }

    macro_rules! update_max {
        ($field:ident, $dimension:ident, $scenario:expr) => {{
            let witness = witnesses
                .get(&ResourceDimension::$dimension)
                .map(String::as_str)
                .unwrap_or(first_scenario);
            if $scenario.1.$field > bounds.$field
                || ($scenario.1.$field == bounds.$field && $scenario.0 < witness)
            {
                bounds.$field = $scenario.1.$field;
                witnesses.insert(ResourceDimension::$dimension, $scenario.0.to_owned());
            }
        }};
    }

    for scenario in outcomes.iter().skip(1) {
        update_max!(crane_minutes, CraneMinutes, scenario);
        update_max!(shore_power_wh, ShorePowerWh, scenario);
        update_max!(yard_slot_hours, YardSlotHours, scenario);
        update_max!(labor_hours, LaborHours, scenario);
        update_max!(freshwater_litres, FreshwaterLitres, scenario);
    }

    Ok(ResourceAdverseEnvelope {
        bounds,
        worst_scenario_by_dimension: witnesses,
    })
}

fn policy_dominates(left: &ExtensiveFormPolicy, right: &ExtensiveFormPolicy) -> bool {
    let a = &left.service_adverse_envelope.bounds;
    let b = &right.service_adverse_envelope.bounds;
    let service_no_worse = a.receiver_accepted_units >= b.receiver_accepted_units
        && a.on_time_accepted_units >= b.on_time_accepted_units
        && a.acceptance_time_unknown_units <= b.acceptance_time_unknown_units
        && a.unmet_units <= b.unmet_units
        && a.late_accepted_units <= b.late_accepted_units
        && a.unresolved_units <= b.unresolved_units
        && a.protected_floor_breach_unit_hours <= b.protected_floor_breach_unit_hours
        && a.service_deficit_unit_hours <= b.service_deficit_unit_hours
        && a.time_to_minimum_viable_service <= b.time_to_minimum_viable_service
        && a.time_to_full_service <= b.time_to_full_service
        && a.recovery_cost_units <= b.recovery_cost_units
        && a.emissions_units <= b.emissions_units
        && a.requalification_actions <= b.requalification_actions
        && a.replanning_invocations <= b.replanning_invocations
        && a.plan_churn <= b.plan_churn
        && a.decision_latency_ms <= b.decision_latency_ms;

    let resource_no_worse = left.resource_adverse_envelope.bounds.crane_minutes
        <= right.resource_adverse_envelope.bounds.crane_minutes
        && left.resource_adverse_envelope.bounds.shore_power_wh
            <= right.resource_adverse_envelope.bounds.shore_power_wh
        && left.resource_adverse_envelope.bounds.yard_slot_hours
            <= right.resource_adverse_envelope.bounds.yard_slot_hours
        && left.resource_adverse_envelope.bounds.labor_hours
            <= right.resource_adverse_envelope.bounds.labor_hours
        && left.resource_adverse_envelope.bounds.freshwater_litres
            <= right.resource_adverse_envelope.bounds.freshwater_litres;

    let service_strict = a.receiver_accepted_units > b.receiver_accepted_units
        || a.on_time_accepted_units > b.on_time_accepted_units
        || a.acceptance_time_unknown_units < b.acceptance_time_unknown_units
        || a.unmet_units < b.unmet_units
        || a.late_accepted_units < b.late_accepted_units
        || a.unresolved_units < b.unresolved_units
        || a.protected_floor_breach_unit_hours < b.protected_floor_breach_unit_hours
        || a.service_deficit_unit_hours < b.service_deficit_unit_hours
        || a.time_to_minimum_viable_service < b.time_to_minimum_viable_service
        || a.time_to_full_service < b.time_to_full_service
        || a.recovery_cost_units < b.recovery_cost_units
        || a.emissions_units < b.emissions_units
        || a.requalification_actions < b.requalification_actions
        || a.replanning_invocations < b.replanning_invocations
        || a.plan_churn < b.plan_churn
        || a.decision_latency_ms < b.decision_latency_ms;

    let resource_strict = left.resource_adverse_envelope.bounds.crane_minutes
        < right.resource_adverse_envelope.bounds.crane_minutes
        || left.resource_adverse_envelope.bounds.shore_power_wh
            < right.resource_adverse_envelope.bounds.shore_power_wh
        || left.resource_adverse_envelope.bounds.yard_slot_hours
            < right.resource_adverse_envelope.bounds.yard_slot_hours
        || left.resource_adverse_envelope.bounds.labor_hours
            < right.resource_adverse_envelope.bounds.labor_hours
        || left.resource_adverse_envelope.bounds.freshwater_litres
            < right.resource_adverse_envelope.bounds.freshwater_litres;

    service_no_worse && resource_no_worse && (service_strict || resource_strict)
}

/// Solve a bounded extensive-form R1 policy tree by exhaustive enumeration.
///
/// One common action is chosen per information set. Scenario paths and stage
/// outcomes are frozen exogenous inputs; actions change the accumulated outcome
/// but do not generate new observations or alter later path transitions.
pub fn solve_extensive_form_r1(
    paths: &[PolicyScenarioPath],
    max_candidate_policies: u64,
) -> Result<ExtensiveFormOracleResult, String> {
    if paths.is_empty() {
        return Err("extensive-form R1 requires at least one scenario path".to_owned());
    }
    if max_candidate_policies == 0 {
        return Err("max_candidate_policies must be positive".to_owned());
    }
    let context = &paths[0].context;
    if context.instance_id.trim().is_empty()
        || context.instance_version.trim().is_empty()
        || context.service_id.trim().is_empty()
        || context.expected_units == 0
    {
        return Err("scenario context has empty identity or zero expected units".to_owned());
    }

    let expected_schedule = stage_schedule(&paths[0].stages, &paths[0].scenario_id)?;
    let mut scenario_ids = BTreeSet::new();
    let mut grouped: BTreeMap<PolicyInformationSetKey, Vec<&PolicyDecisionStage>> =
        BTreeMap::new();
    let mut stage_decision_count = 0usize;

    for path in paths {
        if path.scenario_id.trim().is_empty()
            || !scenario_ids.insert(path.scenario_id.as_str())
        {
            return Err("scenario IDs must be non-empty and unique".to_owned());
        }
        if path.context != *context {
            return Err(format!(
                "{}: context differs from the frozen service obligation",
                path.scenario_id
            ));
        }
        if stage_schedule(&path.stages, &path.scenario_id)? != expected_schedule {
            return Err(format!(
                "{}: scenario epoch/cutoff schedule differs from the frozen tree",
                path.scenario_id
            ));
        }

        for stage in &path.stages {
            stage_decision_count = stage_decision_count
                .checked_add(1)
                .ok_or_else(|| "stage decision count overflow".to_owned())?;
            if !is_sha256_digest(&stage.observable_state_digest) {
                return Err(format!(
                    "{} at epoch {}: malformed observable-state SHA-256 commitment",
                    path.scenario_id, stage.decision_epoch
                ));
            }
            if stage.alternatives.is_empty() {
                return Err(format!(
                    "{} at epoch {}: action catalogue is empty",
                    path.scenario_id, stage.decision_epoch
                ));
            }
            let mut action_ids = BTreeSet::new();
            for alternative in &stage.alternatives {
                if !action_ids.insert(alternative.action_id.as_str()) {
                    return Err(format!(
                        "{} at epoch {}: duplicate action ID",
                        path.scenario_id, stage.decision_epoch
                    ));
                }
                validate_stage_alternative(
                    &path.scenario_id,
                    stage,
                    alternative,
                    context.expected_units,
                )?;
            }
            grouped.entry(key_for(stage)).or_default().push(stage);
        }
    }

    let mut groups = Vec::with_capacity(grouped.len());
    let mut candidate_policy_count = 1u64;

    for (key, stages) in grouped {
        let first_catalog: BTreeSet<&str> = stages[0]
            .alternatives
            .iter()
            .map(|alternative| alternative.action_id.as_str())
            .collect();
        for stage in stages.iter().skip(1) {
            let catalog: BTreeSet<&str> = stage
                .alternatives
                .iter()
                .map(|alternative| alternative.action_id.as_str())
                .collect();
            if catalog != first_catalog {
                return Err(format!(
                    "information set at epoch {} has inconsistent action catalogues",
                    key.decision_epoch
                ));
            }
        }

        let common_actions: Vec<String> = first_catalog
            .iter()
            .filter(|action_id| {
                stages.iter().all(|stage| {
                    stage
                        .alternatives
                        .iter()
                        .find(|alternative| alternative.action_id.as_str() == **action_id)
                        .is_some_and(|alternative| {
                            alternative.admissibility == Gate::Yes
                                && alternative.hard_constraints_passed
                                && alternative
                                    .outcome
                                    .protected_floor_breach_unit_hours
                                    == 0
                        })
                })
            })
            .map(|action_id| (*action_id).to_owned())
            .collect();

        if common_actions.is_empty() {
            return Err(format!(
                "information set at epoch {} has no common feasible action; add a declared safe-abstention action if appropriate",
                key.decision_epoch
            ));
        }

        candidate_policy_count = candidate_policy_count
            .checked_mul(common_actions.len() as u64)
            .ok_or_else(|| "extensive-form candidate policy count overflow".to_owned())?;
        if candidate_policy_count > max_candidate_policies {
            return Err(format!(
                "extensive-form search requires {candidate_policy_count} policies, exceeding limit {max_candidate_policies}"
            ));
        }
        groups.push(InformationGroup {
            key,
            common_actions,
        });
    }

    let policy_capacity = usize::try_from(candidate_policy_count)
        .map_err(|_| "candidate policy count exceeds addressable memory capacity".to_owned())?;
    let mut all_policies = Vec::with_capacity(policy_capacity);
    let mut choices = BTreeMap::new();
    enumerate(
        0,
        &groups,
        paths,
        &mut choices,
        &mut all_policies,
    )?;
    if all_policies.len() as u64 != candidate_policy_count {
        return Err("extensive-form policy enumeration count mismatch".to_owned());
    }

    let pareto_frontier: Vec<ExtensiveFormPolicy> = all_policies
        .iter()
        .filter(|candidate| {
            !all_policies
                .iter()
                .any(|other| other != *candidate && policy_dominates(other, candidate))
        })
        .cloned()
        .collect();

    Ok(ExtensiveFormOracleResult {
        scenario_count: paths.len(),
        stage_decision_count,
        information_set_count: groups.len(),
        candidate_policy_count,
        evaluated_policy_count: all_policies.len() as u64,
        pareto_frontier,
    })
}

fn enumerate(
    group_index: usize,
    groups: &[InformationGroup],
    paths: &[PolicyScenarioPath],
    choices: &mut BTreeMap<PolicyInformationSetKey, String>,
    output: &mut Vec<ExtensiveFormPolicy>,
) -> Result<(), String> {
    if group_index == groups.len() {
        let (service_adverse_envelope, resource_adverse_envelope) =
            evaluate_policy(paths, choices)?;
        output.push(ExtensiveFormPolicy {
            choices: choices.clone(),
            service_adverse_envelope,
            resource_adverse_envelope,
        });
        return Ok(());
    }

    let group = &groups[group_index];
    for action_id in &group.common_actions {
        choices.insert(group.key.clone(), action_id.clone());
        enumerate(group_index + 1, groups, paths, choices, output)?;
    }
    choices.remove(&group.key);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> DecisionContext {
        DecisionContext {
            instance_id: "R1-EXTENSIVE-TINY".to_owned(),
            instance_version: "v0.1".to_owned(),
            service_id: "ESSENTIAL-SERVICE".to_owned(),
            service_deadline: 100,
            expected_units: 10,
        }
    }

    fn contribution(accepted: u64, unmet: u64, cost: u64) -> StageOutcomeContribution {
        StageOutcomeContribution {
            receiver_accepted_units: accepted,
            on_time_accepted_units: accepted,
            acceptance_time_unknown_units: 0,
            unmet_units: unmet,
            unresolved_units: 0,
            late_accepted_units: 0,
            protected_floor_breach_unit_hours: 0,
            service_deficit_unit_hours: unmet * 2,
            time_to_minimum_viable_service: if accepted >= 5 { Some(60) } else { None },
            time_to_full_service: if accepted >= 10 { Some(90) } else { None },
            recovery_cost_units: cost,
            emissions_units: 2,
            requalification_actions: 0,
            replanning_invocations: 1,
            plan_churn: 1,
            decision_latency_ms: 100,
        }
    }

    fn resources(cranes: u64, power: u64, yard: u64) -> ResourceUseContribution {
        ResourceUseContribution {
            crane_minutes: cranes,
            shore_power_wh: power,
            yard_slot_hours: yard,
            labor_hours: 1,
            freshwater_litres: 10,
        }
    }

    fn alternative(
        id: &str,
        accepted: u64,
        unmet: u64,
        cost: u64,
        resource_use: ResourceUseContribution,
    ) -> PolicyActionAlternative {
        PolicyActionAlternative {
            action_id: id.to_owned(),
            admissibility: Gate::Yes,
            hard_constraints_passed: true,
            outcome: contribution(accepted, unmet, cost),
            resource_use,
        }
    }

    fn stage(
        epoch: u64,
        cutoff: u64,
        digest: &str,
        alternatives: Vec<PolicyActionAlternative>,
    ) -> PolicyDecisionStage {
        PolicyDecisionStage {
            decision_epoch: epoch,
            decision_cutoff: cutoff,
            observable_state_digest: digest.to_owned(),
            alternatives,
        }
    }

    fn path(id: &str, stages: Vec<PolicyDecisionStage>) -> PolicyScenarioPath {
        PolicyScenarioPath {
            scenario_id: id.to_owned(),
            context: context(),
            stages,
        }
    }

    #[test]
    fn extensive_form_enumerates_one_action_per_information_set_and_adapts_after_divergence() {
        let paths = vec![
            path(
                "S1",
                vec![
                    stage(
                        1,
                        10,
                        &"a".repeat(64),
                        vec![
                            alternative("LOCAL", 0, 0, 3, resources(5, 10, 2)),
                            alternative("IMPORT", 0, 0, 5, resources(3, 5, 1)),
                        ],
                    ),
                    stage(
                        2,
                        20,
                        &"b".repeat(64),
                        vec![
                            alternative("LOCAL", 10, 0, 10, resources(10, 20, 5)),
                            alternative("IMPORT", 8, 2, 4, resources(5, 10, 3)),
                        ],
                    ),
                ],
            ),
            path(
                "S2",
                vec![
                    stage(
                        1,
                        10,
                        &"a".repeat(64),
                        vec![
                            alternative("LOCAL", 0, 0, 3, resources(5, 10, 2)),
                            alternative("IMPORT", 0, 0, 5, resources(3, 5, 1)),
                        ],
                    ),
                    stage(
                        2,
                        20,
                        &"c".repeat(64),
                        vec![
                            alternative("LOCAL", 7, 3, 10, resources(12, 25, 6)),
                            alternative("IMPORT", 10, 0, 6, resources(6, 12, 3)),
                        ],
                    ),
                ],
            ),
        ];

        let result = solve_extensive_form_r1(&paths, 16).expect("extensive-form R1");
        assert_eq!(result.scenario_count, 2);
        assert_eq!(result.stage_decision_count, 4);
        assert_eq!(result.information_set_count, 3);
        assert_eq!(result.candidate_policy_count, 8);
        assert_eq!(result.evaluated_policy_count, 8);
        assert!(!result.pareto_frontier.is_empty());
        for policy in &result.pareto_frontier {
            assert_eq!(policy.choices.len(), 3);
            assert_eq!(
                policy
                    .choices
                    .keys()
                    .filter(|key| key.decision_epoch == 1)
                    .count(),
                1,
                "identical initial observations must share one policy choice"
            );
        }
        assert!(
            result.pareto_frontier.iter().any(|policy| {
                let later: Vec<&str> = policy
                    .choices
                    .iter()
                    .filter(|(key, _)| key.decision_epoch == 2)
                    .map(|(_, action)| action.as_str())
                    .collect();
                later.len() == 2 && later[0] != later[1]
            }),
            "the extensive-form frontier must include a policy that adapts after observed divergence"
        );
    }

    #[test]
    fn identical_information_stages_with_different_action_catalogues_are_rejected() {
        let first = path(
            "S1",
            vec![stage(
                1,
                10,
                &"a".repeat(64),
                vec![alternative("LOCAL", 10, 0, 1, resources(1, 1, 1))],
            )],
        );
        let second = path(
            "S2",
            vec![stage(
                1,
                10,
                &"a".repeat(64),
                vec![alternative("IMPORT", 10, 0, 1, resources(1, 1, 1))],
            )],
        );
        assert!(
            solve_extensive_form_r1(&[first, second], 10)
                .unwrap_err()
                .contains("inconsistent action catalogues")
        );
    }

    #[test]
    fn service_partition_and_timing_are_exact_at_the_horizon() {
        let path = path(
            "S1",
            vec![
                stage(
                    1,
                    10,
                    &"a".repeat(64),
                    vec![alternative("A", 4, 0, 1, resources(1, 1, 1))],
                ),
                stage(
                    2,
                    20,
                    &"b".repeat(64),
                    vec![alternative("B", 5, 0, 1, resources(1, 1, 1))],
                ),
            ],
        );
        assert!(solve_extensive_form_r1(&[path], 10).is_err());
    }

    #[test]
    fn search_budget_is_checked_before_enumeration() {
        let path = path(
            "S1",
            vec![
                stage(
                    1,
                    10,
                    &"a".repeat(64),
                    vec![
                        alternative("A", 0, 0, 1, resources(1, 1, 1)),
                        alternative("B", 0, 0, 1, resources(1, 1, 1)),
                    ],
                ),
                stage(
                    2,
                    20,
                    &"b".repeat(64),
                    vec![
                        alternative("C", 10, 0, 1, resources(1, 1, 1)),
                        alternative("D", 9, 1, 1, resources(1, 1, 1)),
                    ],
                ),
            ],
        );
        assert!(
            solve_extensive_form_r1(&[path], 3)
                .unwrap_err()
                .contains("exceeding limit")
        );
    }

    #[test]
    fn action_indeterminate_or_floor_breaking_is_not_in_common_action_set() {
        let mut blocked = alternative("LOCAL", 10, 0, 1, resources(1, 1, 1));
        blocked.admissibility = Gate::Indeterminate;
        let safe = alternative("ABSTAIN", 0, 10, 0, resources(0, 0, 0));
        let paths = vec![
            path(
                "S1",
                vec![stage(
                    1,
                    10,
                    &"a".repeat(64),
                    vec![safe.clone(), alternative("LOCAL", 10, 0, 1, resources(1, 1, 1))],
                )],
            ),
            path(
                "S2",
                vec![stage(
                    1,
                    10,
                    &"a".repeat(64),
                    vec![safe, blocked],
                )],
            ),
        ];
        let result = solve_extensive_form_r1(&paths, 10).expect("safe action remains");
        assert_eq!(result.candidate_policy_count, 1);
        assert_eq!(result.pareto_frontier[0].choices.values().next().map(String::as_str), Some("ABSTAIN"));
    }

    #[test]
    fn scenario_order_does_not_change_extensive_form_result() {
        let first = path(
            "A",
            vec![stage(
                1,
                10,
                &"a".repeat(64),
                vec![
                    alternative("A", 10, 0, 5, resources(5, 5, 1)),
                    alternative("B", 9, 1, 3, resources(3, 3, 1)),
                ],
            )],
        );
        let second = path(
            "B",
            vec![stage(
                1,
                10,
                &"a".repeat(64),
                vec![
                    alternative("A", 8, 2, 5, resources(5, 5, 1)),
                    alternative("B", 9, 1, 3, resources(3, 3, 1)),
                ],
            )],
        );
        let forward = solve_extensive_form_r1(&[first.clone(), second.clone()], 10).unwrap();
        let reverse = solve_extensive_form_r1(&[second, first], 10).unwrap();
        assert_eq!(forward, reverse);
    }
}
