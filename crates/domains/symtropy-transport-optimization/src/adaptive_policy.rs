// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Bounded adaptive route-policy selection under scenario information sets.
//!
//! Static minimax routing commits to one path before uncertainty resolves. This
//! module permits a route choice to depend on an explicitly declared information
//! set, but forbids scenario-specific actions when those scenarios are still
//! observationally indistinguishable. This is non-anticipativity for a single
//! recourse decision stage. Multi-stage scenario trees and queueing are future work.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::{
    CargoTransferCapacityResourceKind, FifoPathEstimateV1, FifoPathResourceUseV1,
    FifoPathScenarioKindV1, OperationWindowV1, QuantityUnit,
    ScheduledCapacityAssignmentV1, ScheduledCapacityEvidenceV1,
    ScheduledCapacityPolicyV1,
    ScheduledCapacityResourceSummaryV1, ScheduledCapacityAssignmentReceiptV1,
    validate_scheduled_capacity_plan,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoAdaptiveScenarioDefinitionV1 {
    pub scenario_id: String,
    pub kind: FifoPathScenarioKindV1,
    /// Scenarios in one information set must receive the same route decision.
    pub information_set_id: String,
    /// When this information became available to the decision-maker.
    pub information_available_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoAdaptiveTransportProblemV1 {
    pub scenario_set_id: String,
    pub scenario_manifest_sha256: String,
    pub time_profile_id: String,
    pub initial_departure_epoch_seconds: u64,
    pub knowledge_cutoff_epoch_seconds: u64,
    pub evaluation_time_epoch_seconds: u64,
    pub capacity_policy: ScheduledCapacityPolicyV1,
    pub scenarios: Vec<FifoAdaptiveScenarioDefinitionV1>,
}

/// A route alternative need only supply scenarios in which it is qualified.
/// To be chosen at an information set, it must be present and qualified in
/// every scenario in that information set. Missing scenarios mean unavailable,
/// never zero travel time or unlimited resource capacity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoAdaptiveRouteScenarioOptionV1 {
    pub scenario_id: String,
    pub path_estimate: FifoPathEstimateV1,
    pub resource_uses: Vec<FifoPathResourceUseV1>,
    pub evidence: Vec<ScheduledCapacityEvidenceV1>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoAdaptiveRouteAlternativeV1 {
    pub path_id: String,
    pub scenario_options: Vec<FifoAdaptiveRouteScenarioOptionV1>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoAdaptiveDemandV1 {
    pub demand_id: String,
    pub objective_weight: u64,
    pub alternatives: Vec<FifoAdaptiveRouteAlternativeV1>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoAdaptiveDecisionChoiceV1 {
    pub demand_id: String,
    pub information_set_id: String,
    pub path_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoAdaptiveScenarioScoreV1 {
    pub scenario_id: String,
    pub kind: FifoPathScenarioKindV1,
    pub weighted_elapsed_score: u64,
    pub maximum_individual_elapsed_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FifoAdaptivePolicyRejectionV1 {
    ConcurrentCapacityExceeded {
        scenario_id: String,
        resource_kind: CargoTransferCapacityResourceKind,
        resource_id: String,
        at_epoch_seconds: u64,
        attempted_units: u64,
        limit_units: u64,
    },
    InconsistentCapacityEvidence {
        scenario_id: String,
        resource_kind: CargoTransferCapacityResourceKind,
        resource_id: String,
    },
    DuplicateAssignmentId {
        scenario_id: String,
        assignment_id: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoRejectedAdaptivePolicyV1 {
    pub choices: Vec<FifoAdaptiveDecisionChoiceV1>,
    pub reason: FifoAdaptivePolicyRejectionV1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoAdaptivePolicyReportV1 {
    pub scenario_set_id: String,
    pub scenario_manifest_sha256: String,
    pub time_profile_id: String,
    pub initial_departure_epoch_seconds: u64,
    pub knowledge_cutoff_epoch_seconds: u64,
    pub evaluation_time_epoch_seconds: u64,
    pub capacity_policy_id: String,
    pub information_set_count: usize,
    pub decision_choices: Vec<FifoAdaptiveDecisionChoiceV1>,
    pub scenario_scores: Vec<FifoAdaptiveScenarioScoreV1>,
    pub worst_case_scenario_id: String,
    pub worst_case_individual_elapsed_seconds: u64,
    pub worst_case_weighted_elapsed_score: u64,
    pub nominal_weighted_elapsed_score: u64,
    /// Best jointly feasible static policy using one path per demand across
    /// every information set. None means no such static policy was supplied.
    pub static_baseline_worst_case_individual_elapsed_seconds: Option<u64>,
    pub adaptive_gain_in_worst_case_individual_seconds: Option<u64>,
    pub policies_checked: usize,
    pub feasible_policies_found: usize,
    pub infeasible_policies_found: usize,
    pub rejection_examples: Vec<FifoRejectedAdaptivePolicyV1>,
    pub exactness_scope: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FifoAdaptivePolicyErrorV1 {
    InvalidProblem(&'static str),
    InvalidScenario(String),
    DuplicateScenario(String),
    InvalidInformationSet(String),
    NoDemands,
    InvalidDemand(String),
    DuplicateDemand(String),
    NoRouteAlternatives(String),
    DuplicatePath { demand_id: String, path_id: String },
    InvalidRouteScenario {
        demand_id: String,
        path_id: String,
        scenario_id: String,
        reason: &'static str,
    },
    DuplicateRouteScenario { demand_id: String, path_id: String, scenario_id: String },
    InconsistentRouteTopology { demand_id: String, path_id: String, scenario_id: String },
    InconsistentResourceCrosswalk { demand_id: String, path_id: String, scenario_id: String, edge_id: String },
    NoRouteForInformationSet { demand_id: String, information_set_id: String },
    CombinationCountOverflow,
    CombinationLimitExceeded { required: usize, limit: usize },
    ArithmeticOverflow,
    NoFeasiblePolicy {
        policies_checked: usize,
        capacity_conflicts: usize,
        inconsistent_capacity_snapshots: usize,
        rejection_examples: Vec<FifoRejectedAdaptivePolicyV1>,
    },
}

impl fmt::Display for FifoAdaptivePolicyErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProblem(reason) => write!(f, "invalid adaptive transport problem: {reason}"),
            Self::InvalidScenario(id) => write!(f, "invalid scenario definition {id}"),
            Self::DuplicateScenario(id) => write!(f, "scenario ID {id} is duplicated"),
            Self::InvalidInformationSet(id) => write!(f, "invalid information-set definition {id}"),
            Self::NoDemands => f.write_str("adaptive transport problem has no demands"),
            Self::InvalidDemand(id) => write!(f, "demand {id} is blank or has zero objective weight"),
            Self::DuplicateDemand(id) => write!(f, "demand ID {id} is duplicated"),
            Self::NoRouteAlternatives(id) => write!(f, "demand {id} has no route alternatives"),
            Self::DuplicatePath { demand_id, path_id } => write!(f, "demand {demand_id} repeats route {path_id}"),
            Self::InvalidRouteScenario { demand_id, path_id, scenario_id, reason } =>
                write!(f, "route {path_id} for demand {demand_id} is invalid in {scenario_id}: {reason}"),
            Self::DuplicateRouteScenario { demand_id, path_id, scenario_id } =>
                write!(f, "route {path_id} for demand {demand_id} duplicates scenario {scenario_id}"),
            Self::InconsistentRouteTopology { demand_id, path_id, scenario_id } =>
                write!(f, "route {path_id} for demand {demand_id} changes topology in scenario {scenario_id}"),
            Self::InconsistentResourceCrosswalk { demand_id, path_id, scenario_id, edge_id } =>
                write!(f, "route {path_id} for demand {demand_id} remaps edge {edge_id} in scenario {scenario_id}"),
            Self::NoRouteForInformationSet { demand_id, information_set_id } =>
                write!(f, "demand {demand_id} has no route available across information set {information_set_id}"),
            Self::CombinationCountOverflow => f.write_str("adaptive policy combination count overflowed"),
            Self::CombinationLimitExceeded { required, limit } =>
                write!(f, "adaptive policy search requires {required} combinations, exceeding limit {limit}"),
            Self::ArithmeticOverflow => f.write_str("adaptive policy objective or capacity arithmetic overflowed"),
            Self::NoFeasiblePolicy { policies_checked, capacity_conflicts, inconsistent_capacity_snapshots, .. } =>
                write!(f, "no feasible adaptive policy after {policies_checked} policies ({capacity_conflicts} capacity conflicts, {inconsistent_capacity_snapshots} inconsistent evidence snapshots)"),
        }
    }
}

impl std::error::Error for FifoAdaptivePolicyErrorV1 {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct AdaptiveResourceKey {
    kind: CargoTransferCapacityResourceKind,
    id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct AdaptiveEvidenceFingerprint {
    evidence_id: String,
    capacity_scope_id: String,
    capacity_limit_units: u64,
    capacity_unit: QuantityUnit,
    authority_id: String,
    authority_profile_id: String,
    source_record_id: String,
    source_schema_profile_id: String,
    source_record_digest: String,
    revision: u64,
    supersedes_revision: Option<u64>,
    source_event_time_epoch_seconds: Option<u64>,
    observed_at_epoch_seconds: u64,
    repository_recorded_at_epoch_seconds: u64,
    valid_from_epoch_seconds: u64,
    valid_until_epoch_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AdaptiveResourceUse {
    key: AdaptiveResourceKey,
    fingerprint: AdaptiveEvidenceFingerprint,
    assignment_id: String,
    start: u64,
    end: u64,
    units: u64,
}

#[derive(Clone, Debug)]
struct PreparedAdaptiveScenario {
    path_estimate: FifoPathEstimateV1,
    resources: Vec<AdaptiveResourceUse>,
}

#[derive(Clone, Debug)]
struct PreparedRoute {
    path_id: String,
    by_scenario: BTreeMap<String, PreparedAdaptiveScenario>,
}

#[derive(Clone, Debug)]
struct PreparedDecisionOption {
    demand_id: String,
    information_set_id: String,
    path_id: String,
    objective_weight: u64,
    by_scenario: BTreeMap<String, PreparedAdaptiveScenario>,
}

#[derive(Clone, Debug)]
struct ScenarioInventory {
    definitions: BTreeMap<String, FifoAdaptiveScenarioDefinitionV1>,
    information_sets: BTreeMap<String, Vec<String>>,
    nominal_id: String,
}

fn nonempty(value: &str) -> bool { !value.trim().is_empty() }

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn checked_product(values: impl Iterator<Item = usize>) -> Option<usize> {
    values.try_fold(1_usize, usize::checked_mul)
}

fn path_estimate_consistent(estimate: &FifoPathEstimateV1) -> bool {
    if estimate.legs.is_empty()
        || !nonempty(&estimate.origin_node_id)
        || !nonempty(&estimate.destination_node_id)
        || !nonempty(&estimate.time_profile_id)
        || estimate.initial_departure_epoch_seconds > estimate.final_arrival_epoch_seconds
        || estimate.legs.first().is_none_or(|leg| leg.from_node_id != estimate.origin_node_id)
        || estimate.legs.last().is_none_or(|leg| leg.to_node_id != estimate.destination_node_id)
    {
        return false;
    }

    let mut current_node = estimate.origin_node_id.as_str();
    let mut current_time = estimate.initial_departure_epoch_seconds;
    let mut seen_edges = BTreeSet::new();
    for leg in &estimate.legs {
        if !nonempty(&leg.edge_id)
            || !nonempty(&leg.from_node_id)
            || !nonempty(&leg.to_node_id)
            || leg.from_node_id != current_node
            || !seen_edges.insert(leg.edge_id.as_str())
            || leg.estimate.time_profile_id != estimate.time_profile_id
            || leg.estimate.knowledge_cutoff_epoch_seconds != estimate.knowledge_cutoff_epoch_seconds
            || !nonempty(&leg.estimate.function_id)
            || !nonempty(&leg.estimate.source_record_id)
            || !valid_sha256(&leg.estimate.source_record_sha256)
        {
            return false;
        }
        let Some(expected_departure) = current_time.checked_add(leg.pre_leg_delay_seconds) else {
            return false;
        };
        if leg.estimate.departure_epoch_seconds != expected_departure
            || leg.estimate.arrival_epoch_seconds < leg.estimate.departure_epoch_seconds
            || leg.estimate.travel_seconds
                != leg.estimate.arrival_epoch_seconds - leg.estimate.departure_epoch_seconds
        {
            return false;
        }
        current_time = leg.estimate.arrival_epoch_seconds;
        current_node = leg.to_node_id.as_str();
    }
    current_time == estimate.final_arrival_epoch_seconds
        && estimate.final_arrival_epoch_seconds.checked_sub(estimate.initial_departure_epoch_seconds)
            == Some(estimate.total_elapsed_seconds)
}

fn path_topology(estimate: &FifoPathEstimateV1) -> Vec<(String, String, String)> {
    estimate.legs.iter()
        .map(|leg| (leg.edge_id.clone(), leg.from_node_id.clone(), leg.to_node_id.clone()))
        .collect()
}

fn inventory(problem: &FifoAdaptiveTransportProblemV1) -> Result<ScenarioInventory, FifoAdaptivePolicyErrorV1> {
    if !nonempty(&problem.scenario_set_id)
        || !valid_sha256(&problem.scenario_manifest_sha256)
        || !nonempty(&problem.time_profile_id)
        || !nonempty(&problem.capacity_policy.policy_id)
        || problem.capacity_policy.expected_time_profile_id != problem.time_profile_id
        || problem.scenarios.is_empty()
        || problem.knowledge_cutoff_epoch_seconds > problem.evaluation_time_epoch_seconds
        || problem.evaluation_time_epoch_seconds > problem.initial_departure_epoch_seconds
    {
        return Err(FifoAdaptivePolicyErrorV1::InvalidProblem(
            "identity, digest, time profile, policy, or temporal ordering is invalid",
        ));
    }
    let nominal_count = problem.scenarios.iter()
        .filter(|scenario| scenario.kind == FifoPathScenarioKindV1::Nominal)
        .count();
    if nominal_count != 1 {
        return Err(FifoAdaptivePolicyErrorV1::InvalidProblem("exactly one nominal scenario is required"));
    }

    let mut definitions = BTreeMap::new();
    let mut information_sets: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut available_at_by_set: BTreeMap<String, u64> = BTreeMap::new();
    for scenario in &problem.scenarios {
        if !nonempty(&scenario.scenario_id) || !nonempty(&scenario.information_set_id)
            || scenario.information_available_at_epoch_seconds > problem.initial_departure_epoch_seconds
        {
            return Err(FifoAdaptivePolicyErrorV1::InvalidScenario(scenario.scenario_id.clone()));
        }
        if definitions.insert(scenario.scenario_id.clone(), scenario.clone()).is_some() {
            return Err(FifoAdaptivePolicyErrorV1::DuplicateScenario(scenario.scenario_id.clone()));
        }
        if let Some(previous) = available_at_by_set.insert(
            scenario.information_set_id.clone(),
            scenario.information_available_at_epoch_seconds,
        ) {
            if previous != scenario.information_available_at_epoch_seconds {
                return Err(FifoAdaptivePolicyErrorV1::InvalidInformationSet(scenario.information_set_id.clone()));
            }
        }
        information_sets.entry(scenario.information_set_id.clone())
            .or_default().push(scenario.scenario_id.clone());
    }
    for scenarios in information_sets.values_mut() {
        scenarios.sort();
    }
    let nominal_id = definitions.iter()
        .find(|(_, scenario)| scenario.kind == FifoPathScenarioKindV1::Nominal)
        .map(|(id, _)| id.clone())
        .ok_or(FifoAdaptivePolicyErrorV1::InvalidProblem("nominal scenario is missing"))?;

    Ok(ScenarioInventory { definitions, information_sets, nominal_id })
}

fn summary_for_receipt<'a>(
    summaries: &'a [ScheduledCapacityResourceSummaryV1],
    receipt: &ScheduledCapacityAssignmentReceiptV1,
) -> Option<&'a ScheduledCapacityResourceSummaryV1> {
    let mut matched = summaries.iter().filter(|summary| {
        summary.resource_kind == receipt.resource_kind
            && summary.resource_id == receipt.resource_id
            && summary.evidence_id == receipt.evidence_id
    });
    let one = matched.next()?;
    if matched.next().is_some() { return None; }
    Some(one)
}

fn evidence_fingerprint(
    summary: &ScheduledCapacityResourceSummaryV1,
    receipt: &ScheduledCapacityAssignmentReceiptV1,
) -> AdaptiveEvidenceFingerprint {
    AdaptiveEvidenceFingerprint {
        evidence_id: receipt.evidence_id.clone(),
        capacity_scope_id: summary.capacity_scope_id.clone(),
        capacity_limit_units: summary.capacity_limit_units,
        capacity_unit: summary.capacity_unit,
        authority_id: receipt.authority_id.clone(),
        authority_profile_id: receipt.authority_profile_id.clone(),
        source_record_id: receipt.source_record_id.clone(),
        source_schema_profile_id: receipt.source_schema_profile_id.clone(),
        source_record_digest: receipt.source_record_digest.clone(),
        revision: receipt.evidence_revision,
        supersedes_revision: receipt.supersedes_revision,
        source_event_time_epoch_seconds: receipt.source_event_time_epoch_seconds,
        observed_at_epoch_seconds: receipt.capacity_observed_at_epoch_seconds,
        repository_recorded_at_epoch_seconds: receipt.repository_recorded_at_epoch_seconds,
        valid_from_epoch_seconds: receipt.evidence_valid_from_epoch_seconds,
        valid_until_epoch_seconds: receipt.evidence_valid_until_epoch_seconds,
    }
}

fn validate_scenario_option(
    problem: &FifoAdaptiveTransportProblemV1,
    inventory: &ScenarioInventory,
    demand_id: &str,
    path_id: &str,
    option: &FifoAdaptiveRouteScenarioOptionV1,
) -> Result<(PreparedAdaptiveScenario, Vec<(String, String, String)>, BTreeMap<String, Vec<(String, AdaptiveResourceKey, String, String, Option<String>, u64, QuantityUnit)>>), FifoAdaptivePolicyErrorV1> {
    let fail = |reason| FifoAdaptivePolicyErrorV1::InvalidRouteScenario {
        demand_id: demand_id.to_owned(),
        path_id: path_id.to_owned(),
        scenario_id: option.scenario_id.clone(),
        reason,
    };
    if !inventory.definitions.contains_key(&option.scenario_id) {
        return Err(fail("scenario is undeclared"));
    }
    let estimate = &option.path_estimate;
    if estimate.time_profile_id != problem.time_profile_id
        || estimate.initial_departure_epoch_seconds != problem.initial_departure_epoch_seconds
        || estimate.knowledge_cutoff_epoch_seconds != problem.knowledge_cutoff_epoch_seconds
        || !path_estimate_consistent(estimate)
    {
        return Err(fail("path estimate does not match the frozen temporal contract"));
    }

    let legs_by_edge = estimate.legs.iter()
        .map(|leg| (leg.edge_id.as_str(), leg))
        .collect::<BTreeMap<_, _>>();
    let mut uses = option.resource_uses.clone();
    uses.sort_by(|a, b| (
        a.edge_id.as_str(),
        a.assignment.resource_kind,
        a.assignment.resource_id.as_str(),
        a.assignment.assignment_id.as_str(),
    ).cmp(&(
        b.edge_id.as_str(),
        b.assignment.resource_kind,
        b.assignment.resource_id.as_str(),
        b.assignment.assignment_id.as_str(),
    )));

    let mut seen_edge_resource = BTreeSet::new();
    let mut topology_by_edge: Vec<(String, String, String)> = Vec::new();
    let mut binding_signature: BTreeMap<String, Vec<(String, AdaptiveResourceKey, String, String, Option<String>, u64, QuantityUnit)>> = BTreeMap::new();
    for use_ in &uses {
        let Some(leg) = legs_by_edge.get(use_.edge_id.as_str()) else {
            return Err(fail("resource mapping references an edge outside the evaluated route"));
        };
        if !nonempty(&use_.crosswalk_id) || !valid_sha256(&use_.crosswalk_manifest_sha256)
            || !nonempty(&use_.assignment.resource_id) || use_.edge_id.trim().is_empty()
        {
            return Err(fail("explicit crosswalk identity or digest is incomplete"));
        }
        let window = &use_.assignment.window;
        if window.time_profile_id != estimate.time_profile_id
            || window.planned_start_epoch_seconds != leg.estimate.departure_epoch_seconds
            || window.planned_end_epoch_seconds != leg.estimate.arrival_epoch_seconds
        {
            return Err(fail("resource operation window differs from the evaluated leg interval"));
        }
        if !seen_edge_resource.insert((
            use_.edge_id.clone(), use_.assignment.resource_kind, use_.assignment.resource_id.clone()
        )) {
            return Err(fail("route repeats one edge-to-resource binding"));
        }

        let key = AdaptiveResourceKey {
            kind: use_.assignment.resource_kind,
            id: use_.assignment.resource_id.clone(),
        };
        binding_signature.entry(use_.edge_id.clone()).or_default().push((
            use_.crosswalk_id.clone(), key, use_.crosswalk_manifest_sha256.clone(),
            use_.assignment.capacity_scope_id.clone(), use_.assignment.terminal_id.clone(),
            use_.assignment.capacity_units, use_.assignment.capacity_unit,
        ));
    }
    for leg in &estimate.legs {
        if !binding_signature.contains_key(&leg.edge_id) {
            return Err(fail("a route leg has no explicit physical-resource mapping"));
        }
    }
    for values in binding_signature.values_mut() { values.sort(); }

    let assignments = uses.iter().map(|use_| use_.assignment.clone()).collect::<Vec<_>>();
    let report = validate_scheduled_capacity_plan(
        &assignments,
        &option.evidence,
        problem.evaluation_time_epoch_seconds,
        &problem.capacity_policy,
    ).map_err(|_| fail("resource capacity plan failed evidence, freshness, window, unit, or capacity validation"))?;

    let mut resources = Vec::new();
    let mut assignment_ids = BTreeSet::new();
    for receipt in &report.assignments {
        if !assignment_ids.insert(receipt.assignment_id.clone()) {
            return Err(fail("capacity report repeats an assignment ID"));
        }
        let summary = summary_for_receipt(&report.resources, receipt)
            .ok_or_else(|| fail("capacity receipt has no unique resource summary"))?;
        if summary.capacity_limit_units == 0
            || summary.capacity_unit != receipt.capacity_unit
            || summary.capacity_scope_id != receipt.capacity_scope_id
            || receipt.capacity_units > summary.capacity_limit_units
            || summary.maximum_concurrent_units > summary.capacity_limit_units
            || !summary.assignment_ids.contains(&receipt.assignment_id)
            || receipt.window.planned_start_epoch_seconds < summary.first_planned_start_epoch_seconds
            || receipt.window.planned_end_epoch_seconds > summary.last_planned_end_epoch_seconds
        {
            return Err(fail("capacity receipt disagrees with its checked resource summary"));
        }
        resources.push(AdaptiveResourceUse {
            key: AdaptiveResourceKey {
                kind: receipt.resource_kind,
                id: receipt.resource_id.clone(),
            },
            fingerprint: evidence_fingerprint(summary, receipt),
            assignment_id: receipt.assignment_id.clone(),
            start: receipt.window.planned_start_epoch_seconds,
            end: receipt.window.planned_end_epoch_seconds,
            units: receipt.capacity_units,
        });
    }
    let referenced_summary_keys = report.assignments.iter()
        .map(|receipt| (receipt.resource_kind, receipt.resource_id.clone(), receipt.evidence_id.clone()))
        .collect::<BTreeSet<_>>();
    let reported_summary_keys = report.resources.iter()
        .map(|summary| (summary.resource_kind, summary.resource_id.clone(), summary.evidence_id.clone()))
        .collect::<BTreeSet<_>>();
    if report.resources.len() != reported_summary_keys.len()
        || reported_summary_keys != referenced_summary_keys
    {
        return Err(fail("capacity report contains an unused, duplicated, or unreferenced resource-evidence summary"));
    }
    topology_by_edge.extend(path_topology(estimate));
    let signature = binding_signature.into_iter().collect::<BTreeMap<_, _>>();

    Ok((PreparedAdaptiveScenario { path_estimate: estimate.clone(), resources }, topology_by_edge, signature))
}

fn joint_capacity_check(
    chosen: &[&PreparedDecisionOption],
    scenario_id: &str,
) -> Result<(), FifoAdaptivePolicyRejectionV1> {
    #[derive(Clone)]
    struct Event {
        time: u64,
        is_start: bool,
        assignment_id: String,
        units: u64,
        limit: u64,
        key: AdaptiveResourceKey,
    }

    let mut snapshots: BTreeMap<AdaptiveResourceKey, Vec<AdaptiveEvidenceFingerprint>> = BTreeMap::new();
    let mut events_by_resource: BTreeMap<AdaptiveResourceKey, Vec<Event>> = BTreeMap::new();
    let mut assignment_ids = BTreeSet::new();

    for decision in chosen {
        let Some(route_scenario) = decision.by_scenario.get(scenario_id) else { continue; };
        for use_ in &route_scenario.resources {
            if !assignment_ids.insert(use_.assignment_id.clone()) {
                return Err(FifoAdaptivePolicyRejectionV1::DuplicateAssignmentId {
                    scenario_id: scenario_id.to_owned(),
                    assignment_id: use_.assignment_id.clone(),
                });
            }
            let fingerprints = snapshots.entry(use_.key.clone()).or_default();
            for previous in fingerprints.iter() {
                let validity_overlaps = previous.valid_from_epoch_seconds < use_.fingerprint.valid_until_epoch_seconds
                    && use_.fingerprint.valid_from_epoch_seconds < previous.valid_until_epoch_seconds;
                if previous != &use_.fingerprint
                    && (previous.evidence_id == use_.fingerprint.evidence_id || validity_overlaps)
                {
                    return Err(FifoAdaptivePolicyRejectionV1::InconsistentCapacityEvidence {
                        scenario_id: scenario_id.to_owned(),
                        resource_kind: use_.key.kind,
                        resource_id: use_.key.id.clone(),
                    });
                }
            }
            if !fingerprints.contains(&use_.fingerprint) {
                fingerprints.push(use_.fingerprint.clone());
            }
            let events = events_by_resource.entry(use_.key.clone()).or_default();
            events.push(Event {
                time: use_.start, is_start: true, assignment_id: use_.assignment_id.clone(),
                units: use_.units, limit: use_.fingerprint.capacity_limit_units, key: use_.key.clone(),
            });
            events.push(Event {
                time: use_.end, is_start: false, assignment_id: use_.assignment_id.clone(),
                units: use_.units, limit: use_.fingerprint.capacity_limit_units, key: use_.key.clone(),
            });
        }
    }

    for events in events_by_resource.values_mut() {
        events.sort_by(|a, b| (a.time, a.is_start, &a.assignment_id)
            .cmp(&(b.time, b.is_start, &b.assignment_id)));
        let mut active: BTreeMap<String, u64> = BTreeMap::new();
        let mut active_units = 0_u64;
        for event in events.iter() {
            if !event.is_start {
                let Some(units) = active.remove(&event.assignment_id) else {
                    return Err(FifoAdaptivePolicyRejectionV1::DuplicateAssignmentId {
                        scenario_id: scenario_id.to_owned(),
                        assignment_id: event.assignment_id.clone(),
                    });
                };
                active_units = active_units.checked_sub(units).ok_or_else(|| {
                    FifoAdaptivePolicyRejectionV1::ConcurrentCapacityExceeded {
                        scenario_id: scenario_id.to_owned(), resource_kind: event.key.kind,
                        resource_id: event.key.id.clone(), at_epoch_seconds: event.time,
                        attempted_units: u64::MAX, limit_units: event.limit,
                    }
                })?;
                continue;
            }
            active_units = active_units.checked_add(event.units).ok_or_else(|| {
                FifoAdaptivePolicyRejectionV1::ConcurrentCapacityExceeded {
                    scenario_id: scenario_id.to_owned(), resource_kind: event.key.kind,
                    resource_id: event.key.id.clone(), at_epoch_seconds: event.time,
                    attempted_units: u64::MAX, limit_units: event.limit,
                }
            })?;
            if active_units > event.limit {
                return Err(FifoAdaptivePolicyRejectionV1::ConcurrentCapacityExceeded {
                    scenario_id: scenario_id.to_owned(), resource_kind: event.key.kind,
                    resource_id: event.key.id.clone(), at_epoch_seconds: event.time,
                    attempted_units: active_units, limit_units: event.limit,
                });
            }
            if active.insert(event.assignment_id.clone(), event.units).is_some() {
                return Err(FifoAdaptivePolicyRejectionV1::DuplicateAssignmentId {
                    scenario_id: scenario_id.to_owned(), assignment_id: event.assignment_id.clone(),
                });
            }
        }
    }
    Ok(())
}

#[derive(Clone)]
struct AdaptiveScore {
    chosen_indices: Vec<usize>,
    choice_key: Vec<(String, String, String)>,
    scenario_scores: Vec<FifoAdaptiveScenarioScoreV1>,
    worst_case_scenario_id: String,
    worst_individual: u64,
    worst_weighted: u64,
    nominal_weighted: u64,
    is_static: bool,
}

fn compare_scores(a: &AdaptiveScore, b: &AdaptiveScore) -> Ordering {
    (a.worst_individual, a.worst_weighted, a.nominal_weighted, &a.choice_key)
        .cmp(&(b.worst_individual, b.worst_weighted, b.nominal_weighted, &b.choice_key))
}

/// Enumerate all route choices per demand/information set, retaining choices
/// only when the chosen route is qualified in every scenario indistinguishable
/// at that decision point. Capacity is then checked jointly per scenario.
pub fn select_adaptive_robust_transport_policy(
    problem: &FifoAdaptiveTransportProblemV1,
    demands: &[FifoAdaptiveDemandV1],
    max_policy_combinations: usize,
) -> Result<FifoAdaptivePolicyReportV1, FifoAdaptivePolicyErrorV1> {
    let inventory = inventory(problem)?;
    if demands.is_empty() { return Err(FifoAdaptivePolicyErrorV1::NoDemands); }

    let mut demand_ids = BTreeSet::new();
    for demand in demands {
        if !nonempty(&demand.demand_id) || demand.objective_weight == 0 {
            return Err(FifoAdaptivePolicyErrorV1::InvalidDemand(demand.demand_id.clone()));
        }
        if !demand_ids.insert(demand.demand_id.clone()) {
            return Err(FifoAdaptivePolicyErrorV1::DuplicateDemand(demand.demand_id.clone()));
        }
        if demand.alternatives.is_empty() {
            return Err(FifoAdaptivePolicyErrorV1::NoRouteAlternatives(demand.demand_id.clone()));
        }
    }

    let mut sorted_demands = demands.iter().collect::<Vec<_>>();
    sorted_demands.sort_by(|a, b| a.demand_id.cmp(&b.demand_id));
    let mut prepared_by_demand: BTreeMap<String, Vec<PreparedRoute>> = BTreeMap::new();
    let mut endpoints_by_demand: BTreeMap<String, (String, String)> = BTreeMap::new();

    for demand in &sorted_demands {
        let mut alternatives = demand.alternatives.iter().collect::<Vec<_>>();
        alternatives.sort_by(|a, b| a.path_id.cmp(&b.path_id));
        let mut path_ids = BTreeSet::new();
        let mut prepared_routes = Vec::new();
        for alternative in alternatives {
            if !nonempty(&alternative.path_id) || !path_ids.insert(alternative.path_id.clone()) {
                return Err(FifoAdaptivePolicyErrorV1::DuplicatePath {
                    demand_id: demand.demand_id.clone(), path_id: alternative.path_id.clone(),
                });
            }
            if alternative.scenario_options.is_empty() {
                return Err(FifoAdaptivePolicyErrorV1::NoRouteAlternatives(demand.demand_id.clone()));
            }
            let mut by_scenario = BTreeMap::new();
            let mut expected_topology: Option<Vec<(String, String, String)>> = None;
            let mut expected_signature: Option<BTreeMap<String, Vec<(String, AdaptiveResourceKey, String, String, Option<String>, u64, QuantityUnit)>>> = None;
            let mut endpoints: Option<(String, String)> = None;
            for option in &alternative.scenario_options {
                if by_scenario.contains_key(&option.scenario_id) {
                    return Err(FifoAdaptivePolicyErrorV1::DuplicateRouteScenario {
                        demand_id: demand.demand_id.clone(), path_id: alternative.path_id.clone(),
                        scenario_id: option.scenario_id.clone(),
                    });
                }
                let (prepared, topology_rows, signature_rows) =
                    validate_scenario_option(problem, &inventory, &demand.demand_id, &alternative.path_id, option)?;
                let topology = topology_rows.iter().map(|(edge, from, to)| (edge.clone(), from.clone(), to.clone())).collect::<Vec<_>>();
                if let Some(expected) = &expected_topology {
                    if expected != &topology {
                        return Err(FifoAdaptivePolicyErrorV1::InconsistentRouteTopology {
                            demand_id: demand.demand_id.clone(), path_id: alternative.path_id.clone(),
                            scenario_id: option.scenario_id.clone(),
                        });
                    }
                } else { expected_topology = Some(topology); }

                let current_endpoints = (
                    prepared.path_estimate.origin_node_id.clone(),
                    prepared.path_estimate.destination_node_id.clone(),
                );
                if let Some(expected) = &endpoints {
                    if expected != &current_endpoints {
                        return Err(FifoAdaptivePolicyErrorV1::InconsistentRouteTopology {
                            demand_id: demand.demand_id.clone(), path_id: alternative.path_id.clone(),
                            scenario_id: option.scenario_id.clone(),
                        });
                    }
                } else { endpoints = Some(current_endpoints); }

                if let Some(expected) = &expected_signature {
                    if expected != &signature_rows {
                        let edge_id = expected.keys().chain(signature_rows.keys())
                            .find(|edge| expected.get(*edge) != signature_rows.get(*edge))
                            .cloned().unwrap_or_else(|| String::from("<unknown>"));
                        return Err(FifoAdaptivePolicyErrorV1::InconsistentResourceCrosswalk {
                            demand_id: demand.demand_id.clone(), path_id: alternative.path_id.clone(),
                            scenario_id: option.scenario_id.clone(), edge_id,
                        });
                    }
                } else { expected_signature = Some(signature_rows); }
                by_scenario.insert(option.scenario_id.clone(), prepared);
            }
            if let Some(endpoints) = endpoints {
                if let Some(expected) = endpoints_by_demand.get(&demand.demand_id) {
                    if expected != &endpoints {
                        return Err(FifoAdaptivePolicyErrorV1::InvalidRouteScenario {
                            demand_id: demand.demand_id.clone(), path_id: alternative.path_id.clone(),
                            scenario_id: "<all>".into(), reason: "route alternatives for one demand have different endpoints",
                        });
                    }
                } else { endpoints_by_demand.insert(demand.demand_id.clone(), endpoints); }
            }
            prepared_routes.push(PreparedRoute { path_id: alternative.path_id.clone(), by_scenario });
        }
        prepared_by_demand.insert(demand.demand_id.clone(), prepared_routes);
    }

    // One search variable per (demand, information set). A route must exist in
    // every scenario that shares that information set, enforcing non-anticipativity.
    let mut decision_keys: Vec<(String, String, u64, Vec<PreparedDecisionOption>)> = Vec::new();
    for demand in &sorted_demands {
        let routes = &prepared_by_demand[&demand.demand_id];
        for (information_set_id, scenario_ids) in &inventory.information_sets {
            let mut choices = Vec::new();
            for route in routes {
                if scenario_ids.iter().all(|scenario_id| route.by_scenario.contains_key(scenario_id)) {
                    choices.push(PreparedDecisionOption {
                        demand_id: demand.demand_id.clone(),
                        information_set_id: information_set_id.clone(),
                        path_id: route.path_id.clone(),
                        objective_weight: demand.objective_weight,
                        by_scenario: scenario_ids.iter().map(|scenario_id| {
                            (scenario_id.clone(), route.by_scenario[scenario_id].clone())
                        }).collect(),
                    });
                }
            }
            choices.sort_by(|a, b| a.path_id.cmp(&b.path_id));
            if choices.is_empty() {
                return Err(FifoAdaptivePolicyErrorV1::NoRouteForInformationSet {
                    demand_id: demand.demand_id.clone(), information_set_id: information_set_id.clone(),
                });
            }
            decision_keys.push((
                demand.demand_id.clone(), information_set_id.clone(), demand.objective_weight, choices
            ));
        }
    }

    let required = checked_product(decision_keys.iter().map(|(_, _, _, choices)| choices.len()))
        .ok_or(FifoAdaptivePolicyErrorV1::CombinationCountOverflow)?;
    if max_policy_combinations == 0 || required > max_policy_combinations {
        return Err(FifoAdaptivePolicyErrorV1::CombinationLimitExceeded {
            required, limit: max_policy_combinations,
        });
    }

    let mut policies_checked = 0_usize;
    let mut feasible_policies_found = 0_usize;
    let mut infeasible_policies_found = 0_usize;
    let mut capacity_conflicts = 0_usize;
    let mut inconsistent_capacity_snapshots = 0_usize;
    let mut rejection_examples = Vec::new();
    let mut best_adaptive: Option<AdaptiveScore> = None;
    let mut best_static: Option<AdaptiveScore> = None;
    let mut chosen_indices = vec![0_usize; decision_keys.len()];

    fn visit(
        index: usize,
        decision_keys: &[(String, String, u64, Vec<PreparedDecisionOption>)],
        chosen_indices: &mut [usize],
        inventory: &ScenarioInventory,
        policies_checked: &mut usize,
        feasible_policies_found: &mut usize,
        infeasible_policies_found: &mut usize,
        capacity_conflicts: &mut usize,
        inconsistent_capacity_snapshots: &mut usize,
        rejection_examples: &mut Vec<FifoRejectedAdaptivePolicyV1>,
        best_adaptive: &mut Option<AdaptiveScore>,
        best_static: &mut Option<AdaptiveScore>,
    ) -> Result<(), FifoAdaptivePolicyErrorV1> {
        if index < decision_keys.len() {
            for choice_index in 0..decision_keys[index].3.len() {
                chosen_indices[index] = choice_index;
                visit(
                    index + 1, decision_keys, chosen_indices, inventory, policies_checked,
                    feasible_policies_found, infeasible_policies_found, capacity_conflicts,
                    inconsistent_capacity_snapshots, rejection_examples, best_adaptive, best_static,
                )?;
            }
            return Ok(());
        }

        *policies_checked += 1;
        let chosen = decision_keys.iter().zip(chosen_indices.iter())
            .map(|((_, _, _, choices), index)| &choices[*index])
            .collect::<Vec<_>>();
        let choice_key = chosen.iter().map(|choice| (
            choice.demand_id.clone(), choice.information_set_id.clone(), choice.path_id.clone()
        )).collect::<Vec<_>>();
        let mut policy_failure = None;

        for scenario_id in inventory.definitions.keys() {
            let scenario_chosen = chosen.iter()
                .copied()
                .filter(|choice| choice.by_scenario.contains_key(scenario_id))
                .collect::<Vec<_>>();
            let demand_count = scenario_chosen.iter().map(|choice| choice.demand_id.as_str())
                .collect::<BTreeSet<_>>().len();
            if demand_count != decision_keys.iter().map(|(demand, _, _, _)| demand.as_str())
                .collect::<BTreeSet<_>>().len()
            {
                return Err(FifoAdaptivePolicyErrorV1::InvalidProblem(
                    "information sets do not partition scenarios into one choice per demand",
                ));
            }
            if let Err(reason) = joint_capacity_check(&scenario_chosen, scenario_id) {
                policy_failure = Some(reason);
                break;
            }
        }
        if let Some(reason) = policy_failure {
            *infeasible_policies_found += 1;
            match &reason {
                FifoAdaptivePolicyRejectionV1::ConcurrentCapacityExceeded { .. }
                | FifoAdaptivePolicyRejectionV1::DuplicateAssignmentId { .. } => *capacity_conflicts += 1,
                FifoAdaptivePolicyRejectionV1::InconsistentCapacityEvidence { .. } => *inconsistent_capacity_snapshots += 1,
            }
            if rejection_examples.len() < 32 {
                rejection_examples.push(FifoRejectedAdaptivePolicyV1 {
                    choices: choice_key.iter().map(|(demand, info, path)| FifoAdaptiveDecisionChoiceV1 {
                        demand_id: demand.clone(), information_set_id: info.clone(), path_id: path.clone()
                    }).collect(),
                    reason,
                });
            }
            return Ok(());
        }

        let mut scenario_scores = Vec::new();
        for (scenario_id, definition) in &inventory.definitions {
            let scenario_chosen = chosen.iter().copied()
                .filter(|choice| choice.by_scenario.contains_key(scenario_id))
                .collect::<Vec<_>>();
            let mut weighted_elapsed_score = 0_u64;
            let mut maximum_individual_elapsed_seconds = 0_u64;
            for choice in &scenario_chosen {
                let route_scenario = &choice.by_scenario[scenario_id];
                let elapsed = route_scenario.path_estimate.total_elapsed_seconds;
                let weighted = elapsed.checked_mul(choice.objective_weight)
                    .ok_or(FifoAdaptivePolicyErrorV1::ArithmeticOverflow)?;
                weighted_elapsed_score = weighted_elapsed_score.checked_add(weighted)
                    .ok_or(FifoAdaptivePolicyErrorV1::ArithmeticOverflow)?;
                maximum_individual_elapsed_seconds = maximum_individual_elapsed_seconds.max(elapsed);
            }
            scenario_scores.push(FifoAdaptiveScenarioScoreV1 {
                scenario_id: scenario_id.clone(), kind: definition.kind,
                weighted_elapsed_score, maximum_individual_elapsed_seconds,
            });
        }
        scenario_scores.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));
        let worst_individual = scenario_scores.iter().map(|s| s.maximum_individual_elapsed_seconds)
            .max().unwrap_or(0);
        let worst_weighted = scenario_scores.iter().map(|s| s.weighted_elapsed_score)
            .max().unwrap_or(0);
        let nominal_weighted = scenario_scores.iter()
            .find(|s| s.scenario_id == inventory.nominal_id)
            .map(|s| s.weighted_elapsed_score)
            .ok_or(FifoAdaptivePolicyErrorV1::ArithmeticOverflow)?;
        let worst_case_scenario_id = scenario_scores.iter()
            .max_by(|a, b| a.weighted_elapsed_score.cmp(&b.weighted_elapsed_score)
                .then_with(|| b.scenario_id.cmp(&a.scenario_id)))
            .map(|s| s.scenario_id.clone())
            .ok_or(FifoAdaptivePolicyErrorV1::ArithmeticOverflow)?;

        let mut same_path_by_demand: BTreeMap<String, String> = BTreeMap::new();
        let mut is_static = true;
        for choice in &chosen {
            if let Some(previous) = same_path_by_demand.insert(
                choice.demand_id.clone(), choice.path_id.clone()
            ) {
                if previous != choice.path_id { is_static = false; }
            }
        }

        let score = AdaptiveScore {
            chosen_indices: chosen_indices.to_vec(), choice_key, scenario_scores,
            worst_case_scenario_id, worst_individual, worst_weighted, nominal_weighted, is_static,
        };
        *feasible_policies_found += 1;
        if best_adaptive.as_ref().is_none_or(|current| compare_scores(&score, current) == Ordering::Less) {
            *best_adaptive = Some(score.clone());
        }
        if score.is_static
            && best_static.as_ref().is_none_or(|current| compare_scores(&score, current) == Ordering::Less)
        {
            *best_static = Some(score);
        }
        Ok(())
    }

    visit(
        0, &decision_keys, &mut chosen_indices, &inventory, &mut policies_checked,
        &mut feasible_policies_found, &mut infeasible_policies_found, &mut capacity_conflicts,
        &mut inconsistent_capacity_snapshots, &mut rejection_examples, &mut best_adaptive, &mut best_static,
    )?;

    let Some(best) = best_adaptive else {
        return Err(FifoAdaptivePolicyErrorV1::NoFeasiblePolicy {
            policies_checked, capacity_conflicts, inconsistent_capacity_snapshots, rejection_examples,
        });
    };

    let mut decision_choices = Vec::new();
    for (index, (_, _, _, options)) in decision_keys.iter().enumerate() {
        let choice = &options[best.chosen_indices[index]];
        decision_choices.push(FifoAdaptiveDecisionChoiceV1 {
            demand_id: choice.demand_id.clone(),
            information_set_id: choice.information_set_id.clone(),
            path_id: choice.path_id.clone(),
        });
    }
    decision_choices.sort_by(|a, b| (
        a.demand_id.as_str(), a.information_set_id.as_str(), a.path_id.as_str()
    ).cmp(&(
        b.demand_id.as_str(), b.information_set_id.as_str(), b.path_id.as_str()
    )));

    let static_baseline = best_static.as_ref().map(|score| score.worst_individual);
    let adaptive_gain = static_baseline.map(|baseline| baseline.saturating_sub(best.worst_individual));

    Ok(FifoAdaptivePolicyReportV1 {
        scenario_set_id: problem.scenario_set_id.clone(),
        scenario_manifest_sha256: problem.scenario_manifest_sha256.clone(),
        time_profile_id: problem.time_profile_id.clone(),
        initial_departure_epoch_seconds: problem.initial_departure_epoch_seconds,
        knowledge_cutoff_epoch_seconds: problem.knowledge_cutoff_epoch_seconds,
        evaluation_time_epoch_seconds: problem.evaluation_time_epoch_seconds,
        capacity_policy_id: problem.capacity_policy.policy_id.clone(),
        information_set_count: inventory.information_sets.len(),
        decision_choices,
        scenario_scores: best.scenario_scores,
        worst_case_scenario_id: best.worst_case_scenario_id,
        worst_case_individual_elapsed_seconds: best.worst_individual,
        worst_case_weighted_elapsed_score: best.worst_weighted,
        nominal_weighted_elapsed_score: best.nominal_weighted,
        static_baseline_worst_case_individual_elapsed_seconds: static_baseline,
        adaptive_gain_in_worst_case_individual_seconds: adaptive_gain,
        policies_checked,
        feasible_policies_found,
        infeasible_policies_found,
        rejection_examples,
        exactness_scope: "ExactSingleRecourseStageOverSuppliedScenarioQualifiedRouteAlternativesAndInformationSets",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CargoTransferCapacityResourceKind, FifoPathLegEstimateV1, ArrivalTimeEstimateV1};

    const PROFILE: &str = "UTC_UNIX_SECONDS_V1";

    fn problem(scenarios: Vec<FifoAdaptiveScenarioDefinitionV1>) -> FifoAdaptiveTransportProblemV1 {
        FifoAdaptiveTransportProblemV1 {
            scenario_set_id: "adaptive-scenarios-v1".into(),
            scenario_manifest_sha256: "a".repeat(64),
            time_profile_id: PROFILE.into(),
            initial_departure_epoch_seconds: 100,
            knowledge_cutoff_epoch_seconds: 90,
            evaluation_time_epoch_seconds: 99,
            capacity_policy: ScheduledCapacityPolicyV1 {
                policy_id: "adaptive-test-policy-v1".into(),
                expected_time_profile_id: PROFILE.into(),
                max_observation_age_seconds: 1_000,
                allowed_future_clock_skew_seconds: 0,
            },
            scenarios,
        }
    }

    fn scenario(id: &str, kind: FifoPathScenarioKindV1, information_set_id: &str) -> FifoAdaptiveScenarioDefinitionV1 {
        FifoAdaptiveScenarioDefinitionV1 {
            scenario_id: id.into(), kind, information_set_id: information_set_id.into(),
            information_available_at_epoch_seconds: 100,
        }
    }

    fn route_option(
        demand_id: &str,
        path_id: &str,
        scenario_id: &str,
        travel_seconds: u64,
        resource_id: &str,
    ) -> FifoAdaptiveRouteScenarioOptionV1 {
        let edge_id = format!("edge-{path_id}");
        let arrival = 100 + travel_seconds;
        let evidence_id = format!("capacity-{resource_id}-{scenario_id}");
        let assignment = ScheduledCapacityAssignmentV1 {
            assignment_id: format!("assignment-{demand_id}-{path_id}-{scenario_id}"),
            evidence_id: evidence_id.clone(),
            resource_kind: CargoTransferCapacityResourceKind::TransferEdge,
            resource_id: resource_id.into(),
            capacity_scope_id: "scope-adaptive-test".into(),
            capacity_units: 1,
            capacity_unit: QuantityUnit::Shipments,
            terminal_id: None,
            window: OperationWindowV1 {
                time_profile_id: PROFILE.into(),
                planned_start_epoch_seconds: 100,
                planned_end_epoch_seconds: arrival,
            },
        };
        FifoAdaptiveRouteScenarioOptionV1 {
            scenario_id: scenario_id.into(),
            path_estimate: FifoPathEstimateV1 {
                origin_node_id: format!("origin-{demand_id}"),
                destination_node_id: format!("destination-{demand_id}"),
                time_profile_id: PROFILE.into(),
                knowledge_cutoff_epoch_seconds: 90,
                initial_departure_epoch_seconds: 100,
                final_arrival_epoch_seconds: arrival,
                total_elapsed_seconds: travel_seconds,
                legs: vec![FifoPathLegEstimateV1 {
                    edge_id: edge_id.clone(),
                    from_node_id: format!("origin-{demand_id}"),
                    to_node_id: format!("destination-{demand_id}"),
                    pre_leg_delay_seconds: 0,
                    estimate: ArrivalTimeEstimateV1 {
                        function_id: format!("function-{path_id}-{scenario_id}"),
                        source_record_id: format!("travel-source-{path_id}-{scenario_id}"),
                        source_record_sha256: "b".repeat(64),
                        time_profile_id: PROFILE.into(),
                        knowledge_cutoff_epoch_seconds: 90,
                        departure_epoch_seconds: 100,
                        arrival_epoch_seconds: arrival,
                        travel_seconds,
                    },
                }],
            },
            resource_uses: vec![FifoPathResourceUseV1 {
                edge_id,
                crosswalk_id: format!("crosswalk-{path_id}"),
                crosswalk_manifest_sha256: "c".repeat(64),
                assignment,
            }],
            evidence: vec![ScheduledCapacityEvidenceV1 {
                budget_id: evidence_id,
                resource_kind: CargoTransferCapacityResourceKind::TransferEdge,
                resource_id: resource_id.into(),
                terminal_id: None,
                capacity_scope_id: "scope-adaptive-test".into(),
                capacity_units: 1,
                capacity_unit: QuantityUnit::Shipments,
                authority_id: "resource-owner".into(),
                authority_profile_id: "resource-profile-v1".into(),
                source_record_id: format!("capacity-source-{resource_id}-{scenario_id}"),
                source_schema_profile_id: "capacity-schema-v1".into(),
                source_record_digest: "d".repeat(64),
                revision: 1,
                supersedes_revision: None,
                source_event_time_epoch_seconds: Some(90),
                capacity_observed_at_epoch_seconds: Some(99),
                repository_recorded_at_epoch_seconds: Some(99),
                valid_from_epoch_seconds: Some(100),
                valid_until_epoch_seconds: Some(10_000),
                time_profile_id: PROFILE.into(),
                status: super::super::CargoTransferCapacityEvidenceStatus::ReportedCurrent,
            }],
        }
    }

    fn alternative(path_id: &str, options: Vec<FifoAdaptiveRouteScenarioOptionV1>) -> FifoAdaptiveRouteAlternativeV1 {
        FifoAdaptiveRouteAlternativeV1 { path_id: path_id.into(), scenario_options: options }
    }

    fn demand(id: &str, alternatives: Vec<FifoAdaptiveRouteAlternativeV1>) -> FifoAdaptiveDemandV1 {
        FifoAdaptiveDemandV1 { demand_id: id.into(), objective_weight: 1, alternatives }
    }

    #[test]
    fn adaptive_policy_uses_different_routes_after_distinct_observations() {
        let scenario_defs = vec![
            scenario("nominal", FifoPathScenarioKindV1::Nominal, "nominal-observed"),
            scenario("closure", FifoPathScenarioKindV1::Stress, "closure-observed"),
        ];
        let p = problem(scenario_defs);
        let a = alternative("A", vec![route_option("d1", "A", "nominal", 100, "resource-A")]);
        let b = alternative("B", vec![
            route_option("d1", "B", "nominal", 130, "resource-B"),
            route_option("d1", "B", "closure", 150, "resource-B"),
        ]);
        let c = alternative("C", vec![route_option("d1", "C", "closure", 120, "resource-C")]);
        let report = select_adaptive_robust_transport_policy(&p, &[demand("d1", vec![a, b, c])], 9).unwrap();

        let chosen = report.decision_choices.iter()
            .map(|choice| ((choice.demand_id.as_str(), choice.information_set_id.as_str()), choice.path_id.as_str()))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(chosen[&("d1", "nominal-observed")], "A");
        assert_eq!(chosen[&("d1", "closure-observed")], "C");
        assert_eq!(report.worst_case_individual_elapsed_seconds, 120);
        assert_eq!(report.static_baseline_worst_case_individual_elapsed_seconds, Some(150));
        assert_eq!(report.adaptive_gain_in_worst_case_individual_seconds, Some(30));
        assert_eq!(report.policies_checked, 4);

        // Separate finite reference table for all four information-set policies.
        // Rows: (pre-alert route, post-alert route, worst elapsed, nominal elapsed).
        let reference = [
            (("A", "B"), 150_u64, 100_u64),
            (("A", "C"), 120_u64, 100_u64),
            (("B", "B"), 150_u64, 130_u64),
            (("B", "C"), 130_u64, 130_u64),
        ];
        let expected = reference.iter()
            .min_by(|a, b| (a.1, a.1, a.2, a.0).cmp(&(b.1, b.1, b.2, b.0)))
            .expect("the policy table has alternatives");
        assert_eq!(
            (chosen[&("d1", "nominal-observed")], chosen[&("d1", "closure-observed")]),
            expected.0
        );
        assert_eq!(report.worst_case_individual_elapsed_seconds, expected.1);
        assert_eq!(report.nominal_weighted_elapsed_score, expected.2);
    }

    #[test]
    fn scenarios_in_one_information_set_must_share_one_route_choice() {
        let p = problem(vec![
            scenario("nominal", FifoPathScenarioKindV1::Nominal, "not-yet-observed"),
            scenario("delay", FifoPathScenarioKindV1::Stress, "not-yet-observed"),
        ]);
        let a = alternative("A", vec![route_option("d1", "A", "nominal", 100, "resource-A")]);
        let b = alternative("B", vec![
            route_option("d1", "B", "nominal", 130, "resource-B"),
            route_option("d1", "B", "delay", 160, "resource-B"),
        ]);
        let report = select_adaptive_robust_transport_policy(&p, &[demand("d1", vec![a, b])], 2).unwrap();
        assert_eq!(report.decision_choices.len(), 1);
        assert_eq!(report.decision_choices[0].path_id, "B");
        assert_eq!(report.worst_case_individual_elapsed_seconds, 160);
    }


    #[test]
    fn route_that_is_available_only_after_revelation_is_not_chosen_early() {
        let p = problem(vec![
            scenario("no-alert", FifoPathScenarioKindV1::Nominal, "pre-alert"),
            scenario("alert", FifoPathScenarioKindV1::Stress, "post-alert"),
        ]);
        let before = alternative("before", vec![
            route_option("d1", "before", "no-alert", 100, "resource-before")
        ]);
        let adaptive = alternative("adaptive", vec![
            route_option("d1", "adaptive", "no-alert", 130, "resource-adaptive"),
            route_option("d1", "adaptive", "alert", 140, "resource-adaptive"),
        ]);
        let after = alternative("after", vec![
            route_option("d1", "after", "alert", 110, "resource-after")
        ]);
        let report = select_adaptive_robust_transport_policy(
            &p, &[demand("d1", vec![before, adaptive, after])], 9
        ).unwrap();
        let choices = report.decision_choices.iter()
            .map(|choice| (choice.information_set_id.as_str(), choice.path_id.as_str()))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(choices["pre-alert"], "before");
        assert_eq!(choices["post-alert"], "after");
        assert_eq!(report.adaptive_gain_in_worst_case_individual_seconds, Some(30));
    }

    #[test]
    fn simultaneous_demands_are_checked_against_the_same_capacity_in_each_scenario() {
        let p = problem(vec![
            scenario("nominal", FifoPathScenarioKindV1::Nominal, "known"),
        ]);
        let d1 = demand("d1", vec![
            alternative("fast-1", vec![route_option("d1", "fast-1", "nominal", 100, "shared-resource")]),
            alternative("alt-1", vec![route_option("d1", "alt-1", "nominal", 140, "resource-alt-1")]),
        ]);
        let d2 = demand("d2", vec![
            alternative("fast-2", vec![route_option("d2", "fast-2", "nominal", 110, "shared-resource")]),
            alternative("alt-2", vec![route_option("d2", "alt-2", "nominal", 145, "resource-alt-2")]),
        ]);
        let report = select_adaptive_robust_transport_policy(&p, &[d1, d2], 4).unwrap();
        let choices = report.decision_choices.iter()
            .map(|choice| (choice.demand_id.as_str(), choice.path_id.as_str()))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(choices["d1"], "alt-1");
        assert_eq!(choices["d2"], "fast-2");
        assert_eq!(report.policies_checked, 4);
        assert_eq!(report.feasible_policies_found, 3);
        assert_eq!(report.infeasible_policies_found, 1);
    }

    #[test]
    fn over_budget_adaptive_search_fails_explicitly() {
        let p = problem(vec![
            scenario("nominal", FifoPathScenarioKindV1::Nominal, "known"),
        ]);
        let d = demand("d1", vec![
            alternative("A", vec![route_option("d1", "A", "nominal", 100, "resource-A")]),
            alternative("B", vec![route_option("d1", "B", "nominal", 120, "resource-B")]),
        ]);
        assert_eq!(
            select_adaptive_robust_transport_policy(&p, &[d], 1),
            Err(FifoAdaptivePolicyErrorV1::CombinationLimitExceeded { required: 2, limit: 1 })
        );
    }
}
