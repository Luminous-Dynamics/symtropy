//! Finite same-information (R1) decision oracle for KITE/MCSC.
//!
//! Version 0.1 solves one frozen decision slice at a time. It enumerates one
//! action per distinct observable information set and never chooses differently
//! for scenarios with the same as-of evidence. It is not a multistage optimizer.

use std::collections::{BTreeMap, BTreeSet};

use crate::kite::{AsOfSnapshot, Gate};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionContext {
    pub instance_id: String,
    pub instance_version: String,
    pub service_id: String,
    pub service_deadline: u64,
    pub expected_units: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutcomeVector {
    pub receiver_accepted_units: u64,
    pub on_time_accepted_units: u64,
    pub acceptance_time_unknown_units: u64,
    pub unmet_units: u64,
    pub late_accepted_units: u64,
    pub unresolved_units: u64,
    pub protected_floor_breach_unit_hours: u64,
    pub service_deficit_unit_hours: u64,
    pub time_to_minimum_viable_service: u64,
    pub time_to_full_service: u64,
    pub recovery_cost_units: u64,
    pub emissions_units: u64,
    pub requalification_actions: u64,
    pub replanning_invocations: u64,
    pub plan_churn: u64,
    pub decision_latency_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum OutcomeDimension {
    ReceiverAcceptedUnits,
    OnTimeAcceptedUnits,
    AcceptanceTimeUnknownUnits,
    UnmetUnits,
    LateAcceptedUnits,
    UnresolvedUnits,
    ProtectedFloorBreachUnitHours,
    ServiceDeficitUnitHours,
    TimeToMinimumViableService,
    TimeToFullService,
    RecoveryCostUnits,
    EmissionsUnits,
    RequalificationActions,
    ReplanningInvocations,
    PlanChurn,
    DecisionLatencyMs,
}

/// Component-wise adverse bounds with the scenario witnessing each dimension.
/// This is not necessarily a realizable joint scenario outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdverseEnvelope {
    pub bounds: OutcomeVector,
    pub worst_scenario_by_dimension: BTreeMap<OutcomeDimension, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionOutcome {
    pub action_id: String,
    pub admissibility: Gate,
    pub hard_constraints_passed: bool,
    pub outcome: OutcomeVector,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioDecision {
    pub scenario_id: String,
    pub decision_epoch: u64,
    pub context: DecisionContext,
    /// SHA-256 of the canonical complete observable evidence state at this cutoff.
    /// The caller must include payload commitments, rights, authority and freshness dispositions.
    pub observable_state_digest: String,
    pub as_of: AsOfSnapshot,
    pub actions: Vec<ActionOutcome>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct InformationSetKey {
    pub decision_epoch: u64,
    pub decision_cutoff: u64,
    pub signature: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct R1Policy {
    pub choices: BTreeMap<InformationSetKey, String>,
    pub adverse_envelope: AdverseEnvelope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct R1OracleResult {
    pub candidate_policy_count: u64,
    pub pareto_frontier: Vec<R1Policy>,
}

struct InformationGroup {
    key: InformationSetKey,
    common_actions: Vec<String>,
}

/// Return a deterministic information-set key from a KITE as-of snapshot and
/// the caller-supplied SHA-256 commitment to the complete canonical observable
/// state. The commitment must include evidence payload/value digests, rights,
/// authority, freshness and all admissibility-relevant dispositions. This
/// module validates its encoding and binds it into the key; the owner adapter
/// remains responsible for computing and authenticating the committed bundle.
pub fn information_set_key(
    decision_epoch: u64,
    snapshot: &AsOfSnapshot,
    observable_state_digest: &str,
) -> InformationSetKey {
    let mut signature = format!(
        "cutoff={}|observable_state_sha256={observable_state_digest}",
        snapshot.decision_cutoff
    );
    for (logical_id, evidence) in &snapshot.selected {
        append_signature_entry(
            &mut signature,
            logical_id,
            &evidence.vintage_id,
            evidence.revision,
            evidence.visible,
            evidence.admissible,
            "SELECTED",
        );
    }
    for evidence in &snapshot.withheld {
        if evidence.visible != Gate::No {
            append_signature_entry(
                &mut signature,
                &evidence.logical_id,
                &evidence.vintage_id,
                evidence.revision,
                evidence.visible,
                evidence.admissible,
                match evidence.disposition {
                    crate::kite::VintageDisposition::Selected => "SELECTED",
                    crate::kite::VintageDisposition::NotYetAvailable => "NOT_YET_AVAILABLE",
                    crate::kite::VintageDisposition::NotYetIngested => "NOT_YET_INGESTED",
                    crate::kite::VintageDisposition::NotYetValidated => "NOT_YET_VALIDATED",
                    crate::kite::VintageDisposition::SupersededAsOfCutoff => "SUPERSEDED",
                },
            );
        }
    }
    InformationSetKey {
        decision_epoch,
        decision_cutoff: snapshot.decision_cutoff,
        signature,
    }
}

fn is_sha256_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn append_signature_entry(
    target: &mut String,
    logical_id: &str,
    vintage_id: &str,
    revision: u64,
    visible: Gate,
    admissible: Gate,
    disposition: &str,
) {
    use std::fmt::Write as _;
    let _ = write!(
        target,
        "|{}:{logical_id}|{}:{vintage_id}|{revision}|{visible:?}|{admissible:?}|{disposition}",
        logical_id.len(),
        vintage_id.len()
    );
}

/// Solve the finite R1 decision slice by exhaustive enumeration.
///
/// Scenarios with the same information-set key must select the same action.
/// An action is selectable only when it is admissible and passes all declared
/// hard constraints in every scenario sharing that information set.
/// Every scenario must share an identical service/instance context.
///
/// The max_candidate_policies parameter bounds exhaustive enumeration before
/// search begins. No probabilities or scalar utility weights are introduced.
pub fn solve_r1_finite(
    decisions: &[ScenarioDecision],
    max_candidate_policies: u64,
) -> Result<R1OracleResult, String> {
    if decisions.is_empty() {
        return Err("R1 requires at least one scenario decision".to_owned());
    }
    if max_candidate_policies == 0 {
        return Err("max_candidate_policies must be positive".to_owned());
    }

    let expected_context = &decisions[0].context;
    if expected_context.instance_id.trim().is_empty()
        || expected_context.instance_version.trim().is_empty()
        || expected_context.service_id.trim().is_empty()
        || expected_context.expected_units == 0
    {
        return Err("decision context has empty identity or zero expected units".to_owned());
    }

    let mut scenario_ids = BTreeSet::new();
    let decision_epoch = decisions[0].decision_epoch;
    let decision_cutoff = decisions[0].as_of.decision_cutoff;
    let mut grouped: BTreeMap<InformationSetKey, Vec<&ScenarioDecision>> = BTreeMap::new();

    for decision in decisions {
        if decision.scenario_id.trim().is_empty() || !scenario_ids.insert(decision.scenario_id.as_str()) {
            return Err("scenario IDs must be non-empty and unique per decision slice".to_owned());
        }
        if !is_sha256_digest(&decision.observable_state_digest) {
            return Err(format!(
                "{}: observable_state_digest must be 64 lowercase hexadecimal SHA-256 characters",
                decision.scenario_id
            ));
        }
        if decision.context != *expected_context {
            return Err(format!(
                "{}: scenario context differs from the frozen service obligation",
                decision.scenario_id
            ));
        }
        if decision.decision_epoch != decision_epoch {
            return Err("one R1 v0.1 solve must use a single decision epoch".to_owned());
        }
        if decision.as_of.decision_cutoff != decision_cutoff {
            return Err("all scenarios must share the same decision cutoff".to_owned());
        }
        if decision.actions.is_empty() {
            return Err(format!("{}: action catalog is empty", decision.scenario_id));
        }

        let mut action_ids = BTreeSet::new();
        for action in &decision.actions {
            if action.action_id.trim().is_empty() || !action_ids.insert(action.action_id.as_str()) {
                return Err(format!(
                    "{}: action IDs must be non-empty and unique",
                    decision.scenario_id
                ));
            }
            let outcome = &action.outcome;
            let classified_acceptances = outcome
                .on_time_accepted_units
                .checked_add(outcome.late_accepted_units)
                .and_then(|units| units.checked_add(outcome.acceptance_time_unknown_units))
                .ok_or_else(|| "accepted-unit classification overflow".to_owned())?;
            let classified_service = outcome
                .receiver_accepted_units
                .checked_add(outcome.unmet_units)
                .and_then(|units| units.checked_add(outcome.unresolved_units))
                .ok_or_else(|| "service denominator arithmetic overflow".to_owned())?;
            if outcome.receiver_accepted_units > expected_context.expected_units
                || classified_acceptances != outcome.receiver_accepted_units
                || outcome.unmet_units > expected_context.expected_units
                || outcome.unresolved_units > expected_context.expected_units
                || classified_service != expected_context.expected_units
                || outcome.time_to_minimum_viable_service > outcome.time_to_full_service
            {
                return Err(format!(
                    "{} / {}: service quantities or recovery times violate the frozen outcome contract",
                    decision.scenario_id, action.action_id
                ));
            }
            if action.hard_constraints_passed
                && action.outcome.protected_floor_breach_unit_hours > 0
            {
                return Err(format!(
                    "{} / {}: protected-floor breach conflicts with hard_constraints_passed",
                    decision.scenario_id, action.action_id
                ));
            }
        }

        let key = information_set_key(
            decision.decision_epoch,
            &decision.as_of,
            &decision.observable_state_digest,
        );
        grouped.entry(key).or_default().push(decision);
    }

    let mut groups = Vec::new();
    let mut candidate_policy_count = 1_u64;

    for (key, scenarios) in grouped {
        let first_ids: BTreeSet<&str> = scenarios[0]
            .actions
            .iter()
            .map(|action| action.action_id.as_str())
            .collect();

        for scenario in scenarios.iter().skip(1) {
            let ids: BTreeSet<&str> = scenario
                .actions
                .iter()
                .map(|action| action.action_id.as_str())
                .collect();
            if ids != first_ids {
                return Err(format!(
                    "{}: action catalog differs inside one indistinguishable information set",
                    scenario.scenario_id
                ));
            }
        }

        let common_actions: Vec<String> = first_ids
            .iter()
            .filter(|action_id| {
                scenarios.iter().all(|scenario| {
                    scenario
                        .actions
                        .iter()
                        .find(|action| action.action_id.as_str() == **action_id)
                        .is_some_and(|action| {
                            action.admissibility == Gate::Yes
                                && action.hard_constraints_passed
                                && action.outcome.protected_floor_breach_unit_hours == 0
                        })
                })
            })
            .map(|action_id| (*action_id).to_owned())
            .collect();

        if common_actions.is_empty() {
            return Err(format!(
                "information set at epoch {} has no action admissible across every compatible scenario; include an explicit safe-abstention action if one exists",
                key.decision_epoch
            ));
        }

        candidate_policy_count = candidate_policy_count
            .checked_mul(common_actions.len() as u64)
            .ok_or_else(|| "R1 candidate policy count overflow".to_owned())?;
        if candidate_policy_count > max_candidate_policies {
            return Err(format!(
                "R1 exhaustive search requires {candidate_policy_count} policies, exceeding limit {max_candidate_policies}"
            ));
        }

        groups.push(InformationGroup {
            key,
            common_actions,
        });
    }

    let mut all_policies = Vec::new();
    let mut choices = BTreeMap::new();
    enumerate_policies(0, &groups, decisions, &mut choices, &mut all_policies)?;

    if all_policies.len() as u64 != candidate_policy_count {
        return Err("internal R1 enumeration count mismatch".to_owned());
    }

    let pareto_frontier = all_policies
        .iter()
        .filter(|candidate| {
            !all_policies
                .iter()
                .any(|other| {
                    other != *candidate
                        && dominates(
                            &other.adverse_envelope.bounds,
                            &candidate.adverse_envelope.bounds,
                        )
                })
        })
        .cloned()
        .collect();

    Ok(R1OracleResult {
        candidate_policy_count,
        pareto_frontier,
    })
}

fn enumerate_policies(
    index: usize,
    groups: &[InformationGroup],
    all_decisions: &[ScenarioDecision],
    choices: &mut BTreeMap<InformationSetKey, String>,
    output: &mut Vec<R1Policy>,
) -> Result<(), String> {
    if index == groups.len() {
        let mut scenario_outcomes: Vec<(&str, OutcomeVector)> =
            Vec::with_capacity(all_decisions.len());
        for decision in all_decisions {
            let key = information_set_key(
                decision.decision_epoch,
                &decision.as_of,
                &decision.observable_state_digest,
            );
            let action_id = choices
                .get(&key)
                .ok_or_else(|| "R1 policy omitted an information-set action".to_owned())?;
            let action = decision
                .actions
                .iter()
                .find(|action| action.action_id == *action_id)
                .ok_or_else(|| "R1 policy selected an action absent from a scenario".to_owned())?;
            if action.admissibility != Gate::Yes
                || !action.hard_constraints_passed
                || action.outcome.protected_floor_breach_unit_hours > 0
            {
                return Err("R1 enumeration selected a non-admissible or hard-constraint-violating action".to_owned());
            }
            scenario_outcomes.push((decision.scenario_id.as_str(), action.outcome.clone()));
        }

        output.push(R1Policy {
            choices: choices.clone(),
            adverse_envelope: adverse_envelope(&scenario_outcomes)?,
        });
        return Ok(());
    }

    let group = &groups[index];
    for action_id in &group.common_actions {
        choices.insert(group.key.clone(), action_id.clone());
        enumerate_policies(index + 1, groups, all_decisions, choices, output)?;
    }
    choices.remove(&group.key);
    Ok(())
}

fn adverse_envelope(
    outcomes: &[(&str, OutcomeVector)],
) -> Result<AdverseEnvelope, String> {
    let Some((first_scenario, first)) = outcomes.first() else {
        return Err("cannot calculate an empty adverse envelope".to_owned());
    };
    let mut bounds = first.clone();
    let mut witnesses = BTreeMap::new();
    for dimension in [
        OutcomeDimension::ReceiverAcceptedUnits,
        OutcomeDimension::OnTimeAcceptedUnits,
        OutcomeDimension::AcceptanceTimeUnknownUnits,
        OutcomeDimension::UnmetUnits,
        OutcomeDimension::LateAcceptedUnits,
        OutcomeDimension::UnresolvedUnits,
        OutcomeDimension::ProtectedFloorBreachUnitHours,
        OutcomeDimension::ServiceDeficitUnitHours,
        OutcomeDimension::TimeToMinimumViableService,
        OutcomeDimension::TimeToFullService,
        OutcomeDimension::RecoveryCostUnits,
        OutcomeDimension::EmissionsUnits,
        OutcomeDimension::RequalificationActions,
        OutcomeDimension::ReplanningInvocations,
        OutcomeDimension::PlanChurn,
        OutcomeDimension::DecisionLatencyMs,
    ] {
        witnesses.insert(dimension, (*first_scenario).to_owned());
    }

    macro_rules! adverse_min {
        ($field:ident, $dimension:ident, $scenario:expr) => {{
            let current_witness = witnesses
                .get(&OutcomeDimension::$dimension)
                .map(String::as_str)
                .unwrap_or(first_scenario);
            if $scenario.1.$field < bounds.$field
                || ($scenario.1.$field == bounds.$field && $scenario.0 < current_witness)
            {
                bounds.$field = $scenario.1.$field;
                witnesses.insert(OutcomeDimension::$dimension, $scenario.0.to_owned());
            }
        }};
    }
    macro_rules! adverse_max {
        ($field:ident, $dimension:ident, $scenario:expr) => {{
            let current_witness = witnesses
                .get(&OutcomeDimension::$dimension)
                .map(String::as_str)
                .unwrap_or(first_scenario);
            if $scenario.1.$field > bounds.$field
                || ($scenario.1.$field == bounds.$field && $scenario.0 < current_witness)
            {
                bounds.$field = $scenario.1.$field;
                witnesses.insert(OutcomeDimension::$dimension, $scenario.0.to_owned());
            }
        }};
    }

    // Lexicographic scenario-ID tie breaking makes witness selection invariant
    // to input ordering when two scenarios share the same adverse value.
    for scenario in outcomes.iter().skip(1) {
        adverse_min!(receiver_accepted_units, ReceiverAcceptedUnits, scenario);
        adverse_min!(on_time_accepted_units, OnTimeAcceptedUnits, scenario);
        adverse_max!(acceptance_time_unknown_units, AcceptanceTimeUnknownUnits, scenario);
        adverse_max!(unmet_units, UnmetUnits, scenario);
        adverse_max!(late_accepted_units, LateAcceptedUnits, scenario);
        adverse_max!(unresolved_units, UnresolvedUnits, scenario);
        adverse_max!(protected_floor_breach_unit_hours, ProtectedFloorBreachUnitHours, scenario);
        adverse_max!(service_deficit_unit_hours, ServiceDeficitUnitHours, scenario);
        adverse_max!(time_to_minimum_viable_service, TimeToMinimumViableService, scenario);
        adverse_max!(time_to_full_service, TimeToFullService, scenario);
        adverse_max!(recovery_cost_units, RecoveryCostUnits, scenario);
        adverse_max!(emissions_units, EmissionsUnits, scenario);
        adverse_max!(requalification_actions, RequalificationActions, scenario);
        adverse_max!(replanning_invocations, ReplanningInvocations, scenario);
        adverse_max!(plan_churn, PlanChurn, scenario);
        adverse_max!(decision_latency_ms, DecisionLatencyMs, scenario);
    }

    Ok(AdverseEnvelope {
        bounds,
        worst_scenario_by_dimension: witnesses,
    })
}

/// True when left is no worse on every predeclared dimension and strictly
/// better on at least one. Accepted service is maximized; other dimensions
/// are minimized. No metric is converted into a weighted scalar.
pub fn dominates(left: &OutcomeVector, right: &OutcomeVector) -> bool {
    let no_worse = left.receiver_accepted_units >= right.receiver_accepted_units
        && left.on_time_accepted_units >= right.on_time_accepted_units
        && left.acceptance_time_unknown_units <= right.acceptance_time_unknown_units
        && left.unmet_units <= right.unmet_units
        && left.late_accepted_units <= right.late_accepted_units
        && left.unresolved_units <= right.unresolved_units
        && left.protected_floor_breach_unit_hours <= right.protected_floor_breach_unit_hours
        && left.service_deficit_unit_hours <= right.service_deficit_unit_hours
        && left.time_to_minimum_viable_service <= right.time_to_minimum_viable_service
        && left.time_to_full_service <= right.time_to_full_service
        && left.recovery_cost_units <= right.recovery_cost_units
        && left.emissions_units <= right.emissions_units
        && left.requalification_actions <= right.requalification_actions
        && left.replanning_invocations <= right.replanning_invocations
        && left.plan_churn <= right.plan_churn
        && left.decision_latency_ms <= right.decision_latency_ms;

    let strictly_better = left.receiver_accepted_units > right.receiver_accepted_units
        || left.on_time_accepted_units > right.on_time_accepted_units
        || left.acceptance_time_unknown_units < right.acceptance_time_unknown_units
        || left.unmet_units < right.unmet_units
        || left.late_accepted_units < right.late_accepted_units
        || left.unresolved_units < right.unresolved_units
        || left.protected_floor_breach_unit_hours < right.protected_floor_breach_unit_hours
        || left.service_deficit_unit_hours < right.service_deficit_unit_hours
        || left.time_to_minimum_viable_service < right.time_to_minimum_viable_service
        || left.time_to_full_service < right.time_to_full_service
        || left.recovery_cost_units < right.recovery_cost_units
        || left.emissions_units < right.emissions_units
        || left.requalification_actions < right.requalification_actions
        || left.replanning_invocations < right.replanning_invocations
        || left.plan_churn < right.plan_churn
        || left.decision_latency_ms < right.decision_latency_ms;

    no_worse && strictly_better
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kite::{
        select_as_of, AuthorityState, ClockQuality, EvidenceState, EvidenceVintage, RightsState,
    };

    fn snapshot(vintage_id: &str, first_available: u64) -> AsOfSnapshot {
        let item = EvidenceVintage {
            logical_id: "FEEDER".to_owned(),
            vintage_id: vintage_id.to_owned(),
            revision: 1,
            supersedes_vintage_id: None,
            event_time: 20,
            source_observed_at: first_available - 1,
            first_available_at: first_available,
            ingested_at: first_available + 1,
            validated_at: first_available + 2,
            valid_until: None,
            rights: RightsState::Granted,
            authority: AuthorityState::Valid,
            evidence_state: EvidenceState::Observed,
            clock_quality: ClockQuality::Trusted,
            feature_uses_future_data: false,
        };
        select_as_of(&[item], 100).expect("as-of snapshot")
    }

    fn context() -> DecisionContext {
        DecisionContext {
            instance_id: "MCSC-R1-TINY".to_owned(),
            instance_version: "v0.1".to_owned(),
            service_id: "ESSENTIAL-SERVICE".to_owned(),
            service_deadline: 120,
            expected_units: 10,
        }
    }

    fn outcome(accepted: u64, cost: u64) -> OutcomeVector {
        OutcomeVector {
            receiver_accepted_units: accepted,
            on_time_accepted_units: accepted,
            acceptance_time_unknown_units: 0,
            unmet_units: 10 - accepted,
            late_accepted_units: 0,
            unresolved_units: 0,
            protected_floor_breach_unit_hours: 0,
            service_deficit_unit_hours: 0,
            time_to_minimum_viable_service: 2,
            time_to_full_service: 4,
            recovery_cost_units: cost,
            emissions_units: 5,
            requalification_actions: 0,
            replanning_invocations: 1,
            plan_churn: 1,
            decision_latency_ms: 100,
        }
    }

    fn action(id: &str, accepted: u64, cost: u64) -> ActionOutcome {
        ActionOutcome {
            action_id: id.to_owned(),
            admissibility: Gate::Yes,
            hard_constraints_passed: true,
            outcome: outcome(accepted, cost),
        }
    }

    fn decision(id: &str, snap: AsOfSnapshot, actions: Vec<ActionOutcome>) -> ScenarioDecision {
        let digest_seed = snap
            .selected
            .values()
            .map(|item| item.vintage_id.bytes().map(u64::from).sum::<u64>())
            .sum::<u64>();
        ScenarioDecision {
            scenario_id: id.to_owned(),
            decision_epoch: 1,
            context: context(),
            observable_state_digest: format!("{digest_seed:064x}"),
            as_of: snap,
            actions,
        }
    }

    #[test]
    fn indistinguishable_scenarios_must_share_one_action() {
        let shared = snapshot("FEEDER-V1", 60);
        let result = solve_r1_finite(
            &[
                decision("S1", shared.clone(), vec![action("LOCAL", 10, 20), action("IMPORT", 9, 10)]),
                decision("S2", shared, vec![action("LOCAL", 8, 30), action("IMPORT", 9, 15)]),
            ],
            100,
        ).expect("R1 result");

        assert_eq!(result.candidate_policy_count, 2);
        assert_eq!(result.pareto_frontier.len(), 1);
        assert_eq!(
            result.pareto_frontier[0].choices.values().next().map(String::as_str),
            Some("IMPORT")
        );
        assert_eq!(result.pareto_frontier[0].adverse_envelope.bounds.receiver_accepted_units, 9);
        assert_eq!(result.pareto_frontier[0].adverse_envelope.bounds.recovery_cost_units, 15);
    }

    #[test]
    fn pareto_frontier_preserves_service_cost_tradeoffs_without_scalarization() {
        let result = solve_r1_finite(
            &[decision(
                "S1",
                snapshot("FEEDER-V1", 60),
                vec![action("HIGH-SERVICE", 10, 20), action("LOW-COST", 8, 10)],
            )],
            100,
        ).expect("R1 result");
        assert_eq!(result.candidate_policy_count, 2);
        assert_eq!(result.pareto_frontier.len(), 2);
    }

    #[test]
    fn distinct_information_sets_can_select_different_actions() {
        let result = solve_r1_finite(
            &[
                decision("S1", snapshot("FEEDER-V1", 60), vec![action("LOCAL", 10, 20), action("IMPORT", 7, 10)]),
                decision("S2", snapshot("FEEDER-V2", 70), vec![action("LOCAL", 7, 10), action("IMPORT", 10, 20)]),
            ],
            100,
        ).expect("R1 result");

        assert_eq!(result.candidate_policy_count, 4);
        assert_eq!(result.pareto_frontier.len(), 1);
        let chosen: BTreeSet<&str> = result.pareto_frontier[0].choices.values().map(String::as_str).collect();
        assert_eq!(chosen, BTreeSet::from(["LOCAL", "IMPORT"]));
    }

    #[test]
    fn action_indeterminate_in_one_compatible_scenario_is_not_r1_admissible() {
        let mut blocked = action("LOCAL", 10, 10);
        blocked.admissibility = Gate::Indeterminate;
        let result = solve_r1_finite(
            &[
                decision("S1", snapshot("FEEDER-V1", 60), vec![action("ABSTAIN", 0, 0), action("LOCAL", 10, 10)]),
                decision("S2", snapshot("FEEDER-V1", 60), vec![action("ABSTAIN", 0, 0), blocked]),
            ],
            100,
        ).expect("safe abstention remains");
        assert_eq!(result.candidate_policy_count, 1);
        assert_eq!(
            result.pareto_frontier[0].choices.values().next().map(String::as_str),
            Some("ABSTAIN")
        );
    }

    #[test]
    fn protected_floor_failure_is_never_admitted_as_a_feasible_action() {
        let mut unsafe_action = action("CHEAP", 10, 1);
        unsafe_action.outcome.protected_floor_breach_unit_hours = 1;
        unsafe_action.hard_constraints_passed = false;
        let result = solve_r1_finite(
            &[decision(
                "S1",
                snapshot("FEEDER-V1", 60),
                vec![action("ABSTAIN", 0, 5), unsafe_action],
            )],
            100,
        ).expect("abstention remains feasible");
        assert_eq!(result.candidate_policy_count, 1);
        assert_eq!(
            result.pareto_frontier[0].choices.values().next().map(String::as_str),
            Some("ABSTAIN")
        );
    }

    #[test]
    fn action_catalog_must_match_within_an_information_set() {
        let result = solve_r1_finite(
            &[
                decision("S1", snapshot("FEEDER-V1", 60), vec![action("A", 5, 5)]),
                decision("S2", snapshot("FEEDER-V1", 60), vec![action("B", 5, 5)]),
            ],
            100,
        );
        assert!(result.is_err());
    }

    #[test]
    fn service_context_must_match_across_scenarios() {
        let first = decision("S1", snapshot("FEEDER-V1", 60), vec![action("A", 5, 5)]);
        let mut second = decision("S2", snapshot("FEEDER-V1", 60), vec![action("A", 5, 5)]);
        second.context.expected_units = 11;
        assert!(solve_r1_finite(&[first, second], 100).is_err());
    }

    #[test]
    fn service_denominator_cannot_drop_unclassified_units() {
        let mut incomplete = action("PARTIAL", 5, 5);
        incomplete.outcome.unmet_units = 4;
        let result = solve_r1_finite(
            &[decision(
                "S1",
                snapshot("FEEDER-V1", 60),
                vec![incomplete],
            )],
            100,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("frozen outcome contract"));
    }

    #[test]
    fn observable_payload_commitment_separates_same_ids_with_different_values() {
        let shared = snapshot("FEEDER-V1", 60);
        let first = decision("S1", shared.clone(), vec![action("A", 10, 5), action("B", 9, 4)]);
        let mut second = decision("S2", shared, vec![action("A", 9, 5), action("B", 10, 4)]);
        second.observable_state_digest = "b".repeat(64);
        let result = solve_r1_finite(&[first, second], 100).expect("distinct observations");
        assert_eq!(result.candidate_policy_count, 4);
    }

    #[test]
    fn malformed_observation_commitment_is_rejected() {
        let mut item = decision("S1", snapshot("FEEDER-V1", 60), vec![action("A", 10, 5)]);
        item.observable_state_digest = "not-a-digest".to_owned();
        assert!(solve_r1_finite(&[item], 100).is_err());
    }

    #[test]
    fn accepted_units_require_explicit_timing_classification() {
        let mut item = action("PARTIAL-TIME", 5, 5);
        item.outcome.on_time_accepted_units = 4;
        item.outcome.late_accepted_units = 0;
        item.outcome.acceptance_time_unknown_units = 0;
        let result = solve_r1_finite(
            &[decision("S1", snapshot("FEEDER-V1", 60), vec![item])],
            100,
        );
        assert!(result.is_err());
    }

    #[test]
    fn exhaustive_policy_budget_is_checked_before_enumeration() {
        let result = solve_r1_finite(
            &[decision(
                "S1",
                snapshot("FEEDER-V1", 60),
                vec![action("A", 5, 5), action("B", 5, 4), action("C", 5, 3)],
            )],
            2,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceeding limit"));
    }

    #[test]
    fn scenario_ids_must_be_unique_within_a_decision_slice() {
        let snap = snapshot("FEEDER-V1", 60);
        let first = decision("S1", snap.clone(), vec![action("A", 5, 5)]);
        let second = decision("S1", snap, vec![action("A", 6, 5)]);
        assert!(solve_r1_finite(&[first, second], 100).is_err());
    }
}
