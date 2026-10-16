// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Capacity-qualified minimax selection over explicit edge-to-resource bindings.
//!
//! This module composes the travel-time scenario report with the independent
//! schedule-aware capacity validator. It never infers that an edge ID is a
//! physical resource ID: each use names a crosswalk record and manifest digest.
//! Every leg must have at least one explicit resource binding in every scenario.
//!
//! Capacity qualification checks concurrent assigned units over the leg's
//! evaluated [departure, arrival) interval. It does not model queues, capacity
//! induced waiting, reservations, dispatch authority, or competition between
//! different candidate paths that are alternatives rather than simultaneous jobs.
//! The crosswalk digest is shape-checked, not recomputed here; source verification
//! remains the caller's responsibility.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::{
    CargoTransferCapacityResourceKind, FifoPathEstimateV1, FifoRobustPathScoreV1,
    FifoRobustPathSelectionReportV1, FifoPathScenarioKindV1, QuantityUnit,
    ScheduledCapacityAssignmentV1, ScheduledCapacityEvidenceV1, ScheduledCapacityPlanError,
    ScheduledCapacityPolicyV1, ScheduledCapacityReportV1, validate_scheduled_capacity_plan,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoPathResourceUseV1 {
    /// Logical edge identity from the evaluated route.
    pub edge_id: String,
    /// Exact resource-crosswalk record that binds this logical edge to a resource.
    pub crosswalk_id: String,
    /// SHA-256 of the frozen crosswalk manifest. This module checks its shape only.
    pub crosswalk_manifest_sha256: String,
    /// Explicit assignment to one physical/logical capacity-bearing resource.
    pub assignment: ScheduledCapacityAssignmentV1,
}

/// The capacity evidence snapshot for one candidate path in one named scenario.
/// Evidence and assignments are scenario-local; capacity from one scenario must
/// never be borrowed to make another scenario look feasible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoScenarioCapacityPlanV1 {
    pub path_id: String,
    pub scenario_id: String,
    pub resource_uses: Vec<FifoPathResourceUseV1>,
    pub evidence: Vec<ScheduledCapacityEvidenceV1>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FifoPathCapacityFailureV1 {
    MissingScenarioPlan,
    MissingResourceUse { edge_id: String },
    UnknownEdgeBinding { edge_id: String },
    InvalidCrosswalk { edge_id: String },
    DuplicateResourceBinding { edge_id: String, resource_id: String },
    CrosswalkChangedAcrossScenarios { edge_id: String },
    ResourceWindowMismatch { edge_id: String },
    CapacityValidation(ScheduledCapacityPlanError),
}

impl fmt::Display for FifoPathCapacityFailureV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingScenarioPlan => f.write_str("candidate has no capacity plan for this scenario"),
            Self::MissingResourceUse { edge_id } => {
                write!(f, "route edge {edge_id} has no explicit physical-resource binding")
            }
            Self::UnknownEdgeBinding { edge_id } => {
                write!(f, "resource binding references edge {edge_id}, which is not in the evaluated path")
            }
            Self::InvalidCrosswalk { edge_id } => {
                write!(f, "edge {edge_id} has incomplete crosswalk identity or a malformed manifest digest")
            }
            Self::DuplicateResourceBinding { edge_id, resource_id } => {
                write!(f, "edge {edge_id} repeats resource binding {resource_id}")
            }
            Self::CrosswalkChangedAcrossScenarios { edge_id } => {
                write!(f, "edge {edge_id} maps to a different resource across paired scenarios")
            }
            Self::ResourceWindowMismatch { edge_id } => {
                write!(f, "resource operation window for edge {edge_id} differs from the evaluated leg interval")
            }
            Self::CapacityValidation(source) => write!(f, "scheduled capacity validation failed: {source}"),
        }
    }
}

impl std::error::Error for FifoPathCapacityFailureV1 {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FifoRobustCapacitySelectionErrorV1 {
    InvalidSelectionReport(&'static str),
    DuplicateCapacityPlan { path_id: String, scenario_id: String },
    UnknownCapacityPlanPath { path_id: String },
    UnknownCapacityPlanScenario { scenario_id: String },
    NoCapacityQualifiedCandidate,
}

impl fmt::Display for FifoRobustCapacitySelectionErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSelectionReport(reason) => {
                write!(f, "invalid robust path selection report: {reason}")
            }
            Self::DuplicateCapacityPlan { path_id, scenario_id } => {
                write!(f, "duplicate capacity plan for path {path_id}, scenario {scenario_id}")
            }
            Self::UnknownCapacityPlanPath { path_id } => {
                write!(f, "capacity plan references path {path_id}, which is not travel-qualified")
            }
            Self::UnknownCapacityPlanScenario { scenario_id } => {
                write!(f, "capacity plan references undeclared scenario {scenario_id}")
            }
            Self::NoCapacityQualifiedCandidate => f.write_str("no travel-qualified candidate passes mapped capacity in every scenario"),
        }
    }
}

impl std::error::Error for FifoRobustCapacitySelectionErrorV1 {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoQualifiedScenarioCapacityV1 {
    pub scenario_id: String,
    pub report: ScheduledCapacityReportV1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoCapacityQualifiedPathScoreV1 {
    pub path_score: FifoRobustPathScoreV1,
    pub scenario_capacity: Vec<FifoQualifiedScenarioCapacityV1>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoRejectedPathCapacityScenarioV1 {
    pub path_id: String,
    pub scenario_id: String,
    pub reason: FifoPathCapacityFailureV1,
}

/// Capacity-qualified route comparison. The two selected paths are recomputed
/// only over paths whose every named scenario has complete, valid capacity
/// evidence. Nominal and minimax scores remain travel-arrival scores; this report
/// does not add queueing delays or imply a reservation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoRobustCapacitySelectionReportV1 {
    pub scenario_set_id: String,
    pub scenario_manifest_sha256: String,
    pub time_profile_id: String,
    pub evaluation_time_epoch_seconds: u64,
    pub policy_id: String,
    pub nominal_best_path_id: String,
    pub robust_best_path_id: String,
    pub optimality_scope: &'static str,
    pub candidate_path_count: usize,
    pub capacity_qualified_path_count: usize,
    pub qualified_paths: Vec<FifoCapacityQualifiedPathScoreV1>,
    pub rejected_path_scenarios: Vec<FifoRejectedPathCapacityScenarioV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ResourceBindingFingerprint {
    resource_kind: CargoTransferCapacityResourceKind,
    resource_id: String,
    crosswalk_id: String,
    crosswalk_manifest_sha256: String,
    capacity_units: u64,
    capacity_unit: QuantityUnit,
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn path_estimate_is_self_consistent(
    estimate: &FifoPathEstimateV1,
) -> bool {
    if estimate.legs.is_empty()
        || estimate.origin_node_id.trim().is_empty()
        || estimate.destination_node_id.trim().is_empty()
        || estimate.time_profile_id.trim().is_empty()
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
        if leg.edge_id.trim().is_empty()
            || leg.from_node_id.trim().is_empty()
            || leg.to_node_id.trim().is_empty()
            || leg.from_node_id != current_node
            || !seen_edges.insert(leg.edge_id.as_str())
            || leg.estimate.time_profile_id != estimate.time_profile_id
            || leg.estimate.knowledge_cutoff_epoch_seconds
                != estimate.knowledge_cutoff_epoch_seconds
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
        && estimate.final_arrival_epoch_seconds
            .checked_sub(estimate.initial_departure_epoch_seconds)
            == Some(estimate.total_elapsed_seconds)
}

fn validate_selection_report(
    selection: &FifoRobustPathSelectionReportV1,
) -> Result<(Vec<String>, String), FifoRobustCapacitySelectionErrorV1> {
    if selection.scenario_set_id.trim().is_empty()
        || !valid_sha256(&selection.scenario_manifest_sha256)
        || selection.time_profile_id.trim().is_empty()
        || selection.scores.is_empty()
        || selection.qualified_path_count != selection.scores.len()
        || selection.candidate_path_count < selection.scores.len()
    {
        return Err(FifoRobustCapacitySelectionErrorV1::InvalidSelectionReport(
            "identity, digest, or score counts are inconsistent",
        ));
    }

    let mut scenario_kinds = BTreeMap::new();
    let first_score = &selection.scores[0];
    if first_score.path_id.trim().is_empty() || first_score.observations.is_empty() {
        return Err(FifoRobustCapacitySelectionErrorV1::InvalidSelectionReport(
            "first candidate or its scenario observations are empty",
        ));
    }
    for observation in &first_score.observations {
        if observation.scenario_id.trim().is_empty()
            || scenario_kinds.insert(observation.scenario_id.clone(), observation.kind).is_some()
        {
            return Err(FifoRobustCapacitySelectionErrorV1::InvalidSelectionReport(
                "scenario IDs are blank or duplicated",
            ));
        }
    }
    let nominal_ids = scenario_kinds
        .iter()
        .filter(|(_, kind)| **kind == FifoPathScenarioKindV1::Nominal)
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    if nominal_ids.len() != 1 {
        return Err(FifoRobustCapacitySelectionErrorV1::InvalidSelectionReport(
            "exactly one nominal scenario is required",
        ));
    }
    let nominal_id = nominal_ids[0].clone();
    let mut scenario_order = vec![nominal_id.clone()];
    scenario_order.extend(
        scenario_kinds
            .keys()
            .filter(|id| id.as_str() != nominal_id.as_str())
            .cloned(),
    );

    let mut path_ids = BTreeSet::new();
    let mut common_endpoints: Option<(String, String)> = None;
    for score in &selection.scores {
        if score.path_id.trim().is_empty() || !path_ids.insert(score.path_id.clone()) {
            return Err(FifoRobustCapacitySelectionErrorV1::InvalidSelectionReport(
                "candidate path IDs are blank or duplicated",
            ));
        }
        if score.observations.len() != scenario_kinds.len() {
            return Err(FifoRobustCapacitySelectionErrorV1::InvalidSelectionReport(
                "candidate scenario coverage is incomplete",
            ));
        }
        let mut seen_scenarios = BTreeSet::new();
        for observation in &score.observations {
            if scenario_kinds.get(&observation.scenario_id) != Some(&observation.kind)
                || !seen_scenarios.insert(observation.scenario_id.clone())
                || observation.path_estimate.time_profile_id != selection.time_profile_id
                || observation.path_estimate.initial_departure_epoch_seconds
                    != selection.initial_departure_epoch_seconds
                || observation.path_estimate.knowledge_cutoff_epoch_seconds
                    != selection.knowledge_cutoff_epoch_seconds
                || observation.path_estimate.final_arrival_epoch_seconds
                    != observation.final_arrival_epoch_seconds
                || observation.path_estimate.total_elapsed_seconds != observation.elapsed_seconds
                || !path_estimate_is_self_consistent(&observation.path_estimate)
            {
                return Err(FifoRobustCapacitySelectionErrorV1::InvalidSelectionReport(
                    "observation does not match the scenario inventory or its path estimate",
                ));
            }
            let endpoints = (
                observation.path_estimate.origin_node_id.clone(),
                observation.path_estimate.destination_node_id.clone(),
            );
            if let Some(expected) = &common_endpoints {
                if expected != &endpoints {
                    return Err(FifoRobustCapacitySelectionErrorV1::InvalidSelectionReport(
                        "candidate paths do not share one origin/destination pair",
                    ));
                }
            } else {
                common_endpoints = Some(endpoints);
            }
        }
        let nominal_arrival = score
            .observations
            .iter()
            .find(|observation| observation.scenario_id == nominal_id)
            .map(|observation| observation.final_arrival_epoch_seconds)
            .ok_or(FifoRobustCapacitySelectionErrorV1::InvalidSelectionReport(
                "nominal observation is missing",
            ))?;
        let worst = score
            .observations
            .iter()
            .max_by(|a, b| {
                a.final_arrival_epoch_seconds
                    .cmp(&b.final_arrival_epoch_seconds)
                    .then_with(|| b.scenario_id.cmp(&a.scenario_id))
            })
            .ok_or(FifoRobustCapacitySelectionErrorV1::InvalidSelectionReport(
                "scenario observations are empty",
            ))?;
        if score.nominal_arrival_epoch_seconds != nominal_arrival
            || score.worst_case_arrival_epoch_seconds != worst.final_arrival_epoch_seconds
            || score.worst_case_scenario_id != worst.scenario_id
        {
            return Err(FifoRobustCapacitySelectionErrorV1::InvalidSelectionReport(
                "reported nominal or worst-case score does not match observations",
            ));
        }
    }

    let nominal_best = selection
        .scores
        .iter()
        .min_by(|a, b| {
            a.nominal_arrival_epoch_seconds
                .cmp(&b.nominal_arrival_epoch_seconds)
                .then_with(|| a.path_id.cmp(&b.path_id))
        })
        .map(|score| score.path_id.as_str())
        .unwrap_or_default();
    let robust_best = selection
        .scores
        .iter()
        .min_by(|a, b| {
            a.worst_case_arrival_epoch_seconds
                .cmp(&b.worst_case_arrival_epoch_seconds)
                .then_with(|| a.path_id.cmp(&b.path_id))
        })
        .map(|score| score.path_id.as_str())
        .unwrap_or_default();
    if selection.nominal_best_path_id != nominal_best || selection.robust_best_path_id != robust_best {
        return Err(FifoRobustCapacitySelectionErrorV1::InvalidSelectionReport(
            "selected path IDs do not match the reported scores",
        ));
    }

    Ok((scenario_order, nominal_id))
}

fn validate_resource_bindings(
    observation: &super::FifoScenarioPathObservationV1,
    plan: &FifoScenarioCapacityPlanV1,
) -> Result<
    (
        Vec<FifoPathResourceUseV1>,
        BTreeMap<String, Vec<ResourceBindingFingerprint>>,
    ),
    FifoPathCapacityFailureV1,
> {
    let legs_by_edge = observation
        .path_estimate
        .legs
        .iter()
        .map(|leg| (leg.edge_id.as_str(), leg))
        .collect::<BTreeMap<_, _>>();
    let mut uses = plan.resource_uses.clone();
    uses.sort_by(|a, b| {
        (
            a.edge_id.as_str(),
            a.assignment.resource_kind,
            a.assignment.resource_id.as_str(),
            a.assignment.assignment_id.as_str(),
        )
            .cmp(&(
                b.edge_id.as_str(),
                b.assignment.resource_kind,
                b.assignment.resource_id.as_str(),
                b.assignment.assignment_id.as_str(),
            ))
    });

    let mut seen_edge_resource = BTreeSet::new();
    let mut signature: BTreeMap<String, Vec<ResourceBindingFingerprint>> = BTreeMap::new();
    for resource_use in &uses {
        if !legs_by_edge.contains_key(resource_use.edge_id.as_str()) {
            return Err(FifoPathCapacityFailureV1::UnknownEdgeBinding {
                edge_id: resource_use.edge_id.clone(),
            });
        }
        if resource_use.edge_id.trim().is_empty()
            || resource_use.crosswalk_id.trim().is_empty()
            || !valid_sha256(&resource_use.crosswalk_manifest_sha256)
            || resource_use.assignment.resource_id.trim().is_empty()
        {
            return Err(FifoPathCapacityFailureV1::InvalidCrosswalk {
                edge_id: resource_use.edge_id.clone(),
            });
        }
        if !seen_edge_resource.insert((
            resource_use.edge_id.clone(),
            resource_use.assignment.resource_kind,
            resource_use.assignment.resource_id.clone(),
        )) {
            return Err(FifoPathCapacityFailureV1::DuplicateResourceBinding {
                edge_id: resource_use.edge_id.clone(),
                resource_id: resource_use.assignment.resource_id.clone(),
            });
        }
        signature
            .entry(resource_use.edge_id.clone())
            .or_default()
            .push(ResourceBindingFingerprint {
                resource_kind: resource_use.assignment.resource_kind,
                resource_id: resource_use.assignment.resource_id.clone(),
                crosswalk_id: resource_use.crosswalk_id.clone(),
                crosswalk_manifest_sha256: resource_use.crosswalk_manifest_sha256.clone(),
                capacity_units: resource_use.assignment.capacity_units,
                capacity_unit: resource_use.assignment.capacity_unit,
            });
    }

    for leg in &observation.path_estimate.legs {
        if !signature.contains_key(&leg.edge_id) {
            return Err(FifoPathCapacityFailureV1::MissingResourceUse {
                edge_id: leg.edge_id.clone(),
            });
        }
    }
    for resources in signature.values_mut() {
        resources.sort();
    }
    Ok((uses, signature))
}

fn validate_operation_windows(
    observation: &super::FifoScenarioPathObservationV1,
    uses: &[FifoPathResourceUseV1],
) -> Result<(), FifoPathCapacityFailureV1> {
    let legs_by_edge = observation
        .path_estimate
        .legs
        .iter()
        .map(|leg| (leg.edge_id.as_str(), leg))
        .collect::<BTreeMap<_, _>>();
    for resource_use in uses {
        let leg = legs_by_edge[resource_use.edge_id.as_str()];
        let window = &resource_use.assignment.window;
        if window.time_profile_id != observation.path_estimate.time_profile_id
            || window.planned_start_epoch_seconds != leg.estimate.departure_epoch_seconds
            || window.planned_end_epoch_seconds != leg.estimate.arrival_epoch_seconds
        {
            return Err(FifoPathCapacityFailureV1::ResourceWindowMismatch {
                edge_id: resource_use.edge_id.clone(),
            });
        }
    }
    Ok(())
}

/// Filter the travel-qualified path set through explicit resource crosswalks and
/// the existing schedule-aware capacity validator, then recompute nominal and
/// minimax picks over the paths that pass in every scenario.
///
/// Plans must be provided for every scored path/scenario pair. Missing plans,
/// incomplete edge-resource crosswalks, cross-scenario remapping, stale evidence
/// and capacity overcommit all disqualify that path; the rejection remains in
/// the report. Extra plans for travel-unqualified or undeclared paths/scenarios
/// are an input error. All legs need at least one explicit resource use, even if
/// the caller's resource model is a track/transfer/edge-level capacity envelope.
pub fn select_capacity_qualified_robust_fifo_path(
    selection: &FifoRobustPathSelectionReportV1,
    capacity_plans: &[FifoScenarioCapacityPlanV1],
    evaluation_time_epoch_seconds: u64,
    policy: &ScheduledCapacityPolicyV1,
) -> Result<FifoRobustCapacitySelectionReportV1, FifoRobustCapacitySelectionErrorV1> {
    let (scenario_ids, _nominal_id) = validate_selection_report(selection)?;
    let path_scores = selection
        .scores
        .iter()
        .map(|score| (score.path_id.as_str(), score))
        .collect::<BTreeMap<_, _>>();
    let scenario_set = scenario_ids.iter().cloned().collect::<BTreeSet<_>>();
    let mut plans_by_key = BTreeMap::new();
    for plan in capacity_plans {
        if !path_scores.contains_key(plan.path_id.as_str()) {
            return Err(FifoRobustCapacitySelectionErrorV1::UnknownCapacityPlanPath {
                path_id: plan.path_id.clone(),
            });
        }
        if !scenario_set.contains(&plan.scenario_id) {
            return Err(FifoRobustCapacitySelectionErrorV1::UnknownCapacityPlanScenario {
                scenario_id: plan.scenario_id.clone(),
            });
        }
        let key = (plan.path_id.clone(), plan.scenario_id.clone());
        if plans_by_key.insert(key.clone(), plan).is_some() {
            return Err(FifoRobustCapacitySelectionErrorV1::DuplicateCapacityPlan {
                path_id: key.0,
                scenario_id: key.1,
            });
        }
    }

    let mut sorted_scores = selection.scores.iter().collect::<Vec<_>>();
    sorted_scores.sort_by(|a, b| a.path_id.cmp(&b.path_id));
    let mut qualified_paths = Vec::new();
    let mut rejected_path_scenarios = Vec::new();

    for path_score in sorted_scores {
        let mut baseline_signature: Option<BTreeMap<String, Vec<ResourceBindingFingerprint>>> = None;
        let mut reports = Vec::new();
        let mut path_failed = false;
        for scenario_id in &scenario_ids {
            let observation = path_score
                .observations
                .iter()
                .find(|observation| observation.scenario_id == *scenario_id)
                .ok_or(FifoRobustCapacitySelectionErrorV1::InvalidSelectionReport(
                    "scenario inventory changed after validation",
                ))?;
            let Some(plan) = plans_by_key.get(&(path_score.path_id.clone(), scenario_id.clone())) else {
                rejected_path_scenarios.push(FifoRejectedPathCapacityScenarioV1 {
                    path_id: path_score.path_id.clone(),
                    scenario_id: scenario_id.clone(),
                    reason: FifoPathCapacityFailureV1::MissingScenarioPlan,
                });
                path_failed = true;
                continue;
            };

            let (uses, signature) = match validate_resource_bindings(observation, plan) {
                Ok(result) => result,
                Err(reason) => {
                    rejected_path_scenarios.push(FifoRejectedPathCapacityScenarioV1 {
                        path_id: path_score.path_id.clone(),
                        scenario_id: scenario_id.clone(),
                        reason,
                    });
                    path_failed = true;
                    continue;
                }
            };
            if let Some(expected) = &baseline_signature {
                if expected != &signature {
                    let edge_id = expected
                        .keys()
                        .chain(signature.keys())
                        .find(|edge_id| expected.get(*edge_id) != signature.get(*edge_id))
                        .cloned()
                        .unwrap_or_else(|| String::from("<unknown>"));
                    rejected_path_scenarios.push(FifoRejectedPathCapacityScenarioV1 {
                        path_id: path_score.path_id.clone(),
                        scenario_id: scenario_id.clone(),
                        reason: FifoPathCapacityFailureV1::CrosswalkChangedAcrossScenarios { edge_id },
                    });
                    path_failed = true;
                    continue;
                }
            } else {
                baseline_signature = Some(signature);
            }

            if let Err(reason) = validate_operation_windows(observation, &uses) {
                rejected_path_scenarios.push(FifoRejectedPathCapacityScenarioV1 {
                    path_id: path_score.path_id.clone(),
                    scenario_id: scenario_id.clone(),
                    reason,
                });
                path_failed = true;
                continue;
            }

            let mut sorted_uses = uses;
            sorted_uses.sort_by(|a, b| {
                (
                    a.edge_id.as_str(),
                    a.assignment.assignment_id.as_str(),
                )
                    .cmp(&(
                        b.edge_id.as_str(),
                        b.assignment.assignment_id.as_str(),
                    ))
            });
            let assignments = sorted_uses
                .iter()
                .map(|resource_use| resource_use.assignment.clone())
                .collect::<Vec<_>>();
            let mut evidence = plan.evidence.clone();
            evidence.sort_by(|a, b| a.budget_id.cmp(&b.budget_id));
            match validate_scheduled_capacity_plan(
                &assignments,
                &evidence,
                evaluation_time_epoch_seconds,
                policy,
            ) {
                Ok(report) => reports.push(FifoQualifiedScenarioCapacityV1 {
                    scenario_id: scenario_id.clone(),
                    report,
                }),
                Err(source) => {
                    rejected_path_scenarios.push(FifoRejectedPathCapacityScenarioV1 {
                        path_id: path_score.path_id.clone(),
                        scenario_id: scenario_id.clone(),
                        reason: FifoPathCapacityFailureV1::CapacityValidation(source),
                    });
                    path_failed = true;
                }
            }
        }

        if !path_failed && reports.len() == scenario_ids.len() {
            reports.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));
            qualified_paths.push(FifoCapacityQualifiedPathScoreV1 {
                path_score: path_score.clone(),
                scenario_capacity: reports,
            });
        }
    }

    if qualified_paths.is_empty() {
        return Err(FifoRobustCapacitySelectionErrorV1::NoCapacityQualifiedCandidate);
    }
    qualified_paths.sort_by(|a, b| a.path_score.path_id.cmp(&b.path_score.path_id));
    rejected_path_scenarios.sort_by(|a, b| {
        (&a.path_id, &a.scenario_id).cmp(&(&b.path_id, &b.scenario_id))
    });

    let nominal_best_path_id = qualified_paths
        .iter()
        .min_by(|a, b| {
            a.path_score
                .nominal_arrival_epoch_seconds
                .cmp(&b.path_score.nominal_arrival_epoch_seconds)
                .then_with(|| a.path_score.path_id.cmp(&b.path_score.path_id))
        })
        .map(|path| path.path_score.path_id.clone())
        .ok_or(FifoRobustCapacitySelectionErrorV1::InvalidSelectionReport(
            "capacity-qualified path set is empty",
        ))?;
    let robust_best_path_id = qualified_paths
        .iter()
        .min_by(|a, b| {
            a.path_score
                .worst_case_arrival_epoch_seconds
                .cmp(&b.path_score.worst_case_arrival_epoch_seconds)
                .then_with(|| a.path_score.path_id.cmp(&b.path_score.path_id))
        })
        .map(|path| path.path_score.path_id.clone())
        .ok_or(FifoRobustCapacitySelectionErrorV1::InvalidSelectionReport(
            "capacity-qualified path set is empty",
        ))?;

    Ok(FifoRobustCapacitySelectionReportV1 {
        scenario_set_id: selection.scenario_set_id.clone(),
        scenario_manifest_sha256: selection.scenario_manifest_sha256.clone(),
        time_profile_id: selection.time_profile_id.clone(),
        evaluation_time_epoch_seconds,
        policy_id: policy.policy_id.clone(),
        nominal_best_path_id,
        robust_best_path_id,
        optimality_scope: "ExactMinimaxOverTravelQualifiedPathsThatPassMappedCapacityInEveryDeclaredScenario",
        candidate_path_count: selection.candidate_path_count,
        capacity_qualified_path_count: qualified_paths.len(),
        qualified_paths,
        rejected_path_scenarios,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ArrivalFunctionEvidenceStatusV1, ArrivalFunctionEvidenceV1, ArrivalTimeBreakpointV1,
        FifoArrivalTimeFunctionV1, FifoRobustCandidatePathV1, FifoPathScenarioDefinitionV1,
        FifoScenarioCandidatePathV1, FifoPathScenarioSetV1, FifoPathLegV1, select_robust_fifo_path,
        CargoTransferCapacityEvidenceStatus,
    };

    const PROFILE: &str = "UTC_UNIX_SECONDS_V1";
    const EVALUATION_TIME: u64 = 99;

    fn scenario_set() -> FifoPathScenarioSetV1 {
        FifoPathScenarioSetV1 {
            scenario_set_id: "capacity-paired-scenarios-v1".into(),
            manifest_sha256: "a".repeat(64),
            time_profile_id: PROFILE.into(),
            scenarios: vec![
                FifoPathScenarioDefinitionV1 {
                    scenario_id: "nominal".into(),
                    kind: FifoPathScenarioKindV1::Nominal,
                },
                FifoPathScenarioDefinitionV1 {
                    scenario_id: "stress".into(),
                    kind: FifoPathScenarioKindV1::Stress,
                },
            ],
        }
    }

    fn function(function_id: &str, travel: u64) -> FifoArrivalTimeFunctionV1 {
        FifoArrivalTimeFunctionV1::new(
            ArrivalFunctionEvidenceV1 {
                function_id: function_id.into(),
                time_profile_id: PROFILE.into(),
                source_record_id: format!("source-{function_id}"),
                source_record_sha256: Some("b".repeat(64)),
                source_published_at_epoch_seconds: Some(90),
                repository_recorded_at_epoch_seconds: 90,
                source_observed_at_epoch_seconds: 90,
                valid_from_epoch_seconds: 100,
                valid_until_epoch_seconds: 10_000,
                source_locator: Some(format!("manifest.json#/functions/{function_id}")),
                status: ArrivalFunctionEvidenceStatusV1::Qualified,
            },
            vec![
                ArrivalTimeBreakpointV1 {
                    departure_epoch_seconds: 100,
                    arrival_epoch_seconds: 100 + travel,
                },
                ArrivalTimeBreakpointV1 {
                    departure_epoch_seconds: 9_999,
                    arrival_epoch_seconds: 9_999 + travel,
                },
            ],
        )
        .unwrap()
    }

    fn candidate(path_id: &str, nominal: u64, stress: u64) -> FifoRobustCandidatePathV1 {
        let edge = format!("edge-{path_id}");
        FifoRobustCandidatePathV1 {
            path_id: path_id.into(),
            scenario_paths: vec![
                FifoScenarioCandidatePathV1 {
                    scenario_id: "nominal".into(),
                    legs: vec![FifoPathLegV1 {
                        edge_id: edge.clone(),
                        from_node_id: "origin".into(),
                        to_node_id: "destination".into(),
                        pre_leg_delay_seconds: 0,
                        arrival_function: function(&format!("{path_id}-nominal"), nominal),
                    }],
                },
                FifoScenarioCandidatePathV1 {
                    scenario_id: "stress".into(),
                    legs: vec![FifoPathLegV1 {
                        edge_id: edge,
                        from_node_id: "origin".into(),
                        to_node_id: "destination".into(),
                        pre_leg_delay_seconds: 0,
                        arrival_function: function(&format!("{path_id}-stress"), stress),
                    }],
                },
            ],
        }
    }

    fn policy() -> ScheduledCapacityPolicyV1 {
        ScheduledCapacityPolicyV1 {
            policy_id: "fixture-capacity-policy-v1".into(),
            expected_time_profile_id: PROFILE.into(),
            max_observation_age_seconds: 20,
            allowed_future_clock_skew_seconds: 0,
        }
    }

    fn plans_from_report(
        report: &FifoRobustPathSelectionReportV1,
    ) -> Vec<FifoScenarioCapacityPlanV1> {
        let mut plans = Vec::new();
        for score in &report.scores {
            for observation in &score.observations {
                let mut uses = Vec::new();
                let mut evidence = Vec::new();
                for leg in &observation.path_estimate.legs {
                    let budget_id = format!("budget-{}-{}-{}", score.path_id, observation.scenario_id, leg.edge_id);
                    let units = if score.path_id == "B" { 2 } else { 1 };
                    let capacity_limit = if score.path_id == "B" && observation.scenario_id == "stress" {
                        1
                    } else {
                        units
                    };
                    let assignment = ScheduledCapacityAssignmentV1 {
                        assignment_id: format!("assignment-{}-{}-{}", score.path_id, observation.scenario_id, leg.edge_id),
                        evidence_id: budget_id.clone(),
                        resource_kind: CargoTransferCapacityResourceKind::TransferEdge,
                        resource_id: format!("physical-resource-{}", leg.edge_id),
                        capacity_scope_id: "scope-fixture".into(),
                        capacity_units: units,
                        capacity_unit: QuantityUnit::Shipments,
                        terminal_id: None,
                        window: super::super::OperationWindowV1 {
                            time_profile_id: PROFILE.into(),
                            planned_start_epoch_seconds: leg.estimate.departure_epoch_seconds,
                            planned_end_epoch_seconds: leg.estimate.arrival_epoch_seconds,
                        },
                    };
                    uses.push(FifoPathResourceUseV1 {
                        edge_id: leg.edge_id.clone(),
                        crosswalk_id: "edge-resource-crosswalk-v1".into(),
                        crosswalk_manifest_sha256: "c".repeat(64),
                        assignment,
                    });
                    evidence.push(ScheduledCapacityEvidenceV1 {
                        budget_id,
                        resource_kind: CargoTransferCapacityResourceKind::TransferEdge,
                        resource_id: format!("physical-resource-{}", leg.edge_id),
                        terminal_id: None,
                        capacity_scope_id: "scope-fixture".into(),
                        capacity_units: capacity_limit,
                        capacity_unit: QuantityUnit::Shipments,
                        authority_id: "fixture-owner".into(),
                        authority_profile_id: "fixture-capacity-authority-v1".into(),
                        source_record_id: format!("capacity-source-{}-{}", score.path_id, observation.scenario_id),
                        source_schema_profile_id: "fixture-schema-v1".into(),
                        source_record_digest: "d".repeat(64),
                        revision: 1,
                        supersedes_revision: None,
                        source_event_time_epoch_seconds: Some(90),
                        capacity_observed_at_epoch_seconds: Some(EVALUATION_TIME),
                        repository_recorded_at_epoch_seconds: Some(EVALUATION_TIME),
                        valid_from_epoch_seconds: Some(100),
                        valid_until_epoch_seconds: Some(10_000),
                        time_profile_id: PROFILE.into(),
                        status: CargoTransferCapacityEvidenceStatus::ReportedCurrent,
                    });
                }
                plans.push(FifoScenarioCapacityPlanV1 {
                    path_id: score.path_id.clone(),
                    scenario_id: observation.scenario_id.clone(),
                    resource_uses: uses,
                    evidence,
                });
            }
        }
        plans
    }

    fn allow_path_b_capacity(plans: &mut [FifoScenarioCapacityPlanV1]) {
        let plan = plans.iter_mut()
            .find(|plan| plan.path_id == "B" && plan.scenario_id == "stress")
            .expect("path B stress plan exists");
        plan.evidence[0].capacity_units = 2;
    }

    fn travel_report() -> FifoRobustPathSelectionReportV1 {
        select_robust_fifo_path(
            &scenario_set(),
            &[candidate("A", 100, 500), candidate("B", 130, 200)],
            100,
            90,
        )
        .unwrap()
    }

    #[test]
    fn minimax_choice_is_recomputed_after_capacity_rejects_a_route() {
        let travel = travel_report();
        assert_eq!(travel.robust_best_path_id, "B");
        let plans = plans_from_report(&travel);
        let report = select_capacity_qualified_robust_fifo_path(
            &travel,
            &plans,
            EVALUATION_TIME,
            &policy(),
        )
        .unwrap();

        assert_eq!(report.nominal_best_path_id, "A");
        assert_eq!(report.robust_best_path_id, "A");
        assert_eq!(report.capacity_qualified_path_count, 1);
        assert!(report.rejected_path_scenarios.iter().any(|rejected| {
            rejected.path_id == "B"
                && rejected.scenario_id == "stress"
                && rejected.reason == FifoPathCapacityFailureV1::CapacityValidation(
                    ScheduledCapacityPlanError::CapacityExceeded
                )
        }));
    }

    #[test]
    fn every_leg_requires_an_explicit_resource_binding() {
        let travel = travel_report();
        let mut plans = plans_from_report(&travel);
        allow_path_b_capacity(&mut plans);
        plans.iter_mut()
            .find(|plan| plan.path_id == "A" && plan.scenario_id == "stress")
            .unwrap()
            .resource_uses
            .clear();

        let report = select_capacity_qualified_robust_fifo_path(
            &travel,
            &plans,
            EVALUATION_TIME,
            &policy(),
        )
        .unwrap();
        assert_eq!(report.capacity_qualified_path_count, 1);
        assert!(report.rejected_path_scenarios.iter().any(|rejected| {
            rejected.path_id == "A"
                && rejected.scenario_id == "stress"
                && rejected.reason == (FifoPathCapacityFailureV1::MissingResourceUse {
                    edge_id: "edge-A".into(),
                })
        }));
    }

    #[test]
    fn resource_window_must_match_the_evaluated_leg() {
        let travel = travel_report();
        let mut plans = plans_from_report(&travel);
        allow_path_b_capacity(&mut plans);
        let use_ = &mut plans.iter_mut()
            .find(|plan| plan.path_id == "A" && plan.scenario_id == "nominal")
            .unwrap()
            .resource_uses[0];
        use_.assignment.window.planned_end_epoch_seconds -= 1;

        let report = select_capacity_qualified_robust_fifo_path(
            &travel,
            &plans,
            EVALUATION_TIME,
            &policy(),
        )
        .unwrap();
        assert!(report.rejected_path_scenarios.iter().any(|rejected| {
            rejected.path_id == "A"
                && rejected.scenario_id == "nominal"
                && rejected.reason == (FifoPathCapacityFailureV1::ResourceWindowMismatch {
                    edge_id: "edge-A".into(),
                })
        }));
    }

    #[test]
    fn edge_cannot_remap_to_a_different_physical_resource_in_a_stress_scenario() {
        let travel = travel_report();
        let mut plans = plans_from_report(&travel);
        allow_path_b_capacity(&mut plans);
        let use_ = &mut plans.iter_mut()
            .find(|plan| plan.path_id == "A" && plan.scenario_id == "stress")
            .unwrap()
            .resource_uses[0];
        use_.assignment.resource_id = "substituted-resource".into();
        plans.iter_mut()
            .find(|plan| plan.path_id == "A" && plan.scenario_id == "stress")
            .unwrap()
            .evidence[0]
            .resource_id = "substituted-resource".into();

        let report = select_capacity_qualified_robust_fifo_path(
            &travel,
            &plans,
            EVALUATION_TIME,
            &policy(),
        )
        .unwrap();
        assert!(report.rejected_path_scenarios.iter().any(|rejected| {
            rejected.path_id == "A"
                && rejected.scenario_id == "stress"
                && matches!(&rejected.reason, FifoPathCapacityFailureV1::CrosswalkChangedAcrossScenarios { .. })
        }));
    }

    #[test]
    fn stale_capacity_evidence_cannot_qualify_a_route() {
        let travel = travel_report();
        let mut plans = plans_from_report(&travel);
        allow_path_b_capacity(&mut plans);
        plans.iter_mut()
            .find(|plan| plan.path_id == "A" && plan.scenario_id == "nominal")
            .unwrap()
            .evidence[0]
            .capacity_observed_at_epoch_seconds = Some(0);

        let report = select_capacity_qualified_robust_fifo_path(
            &travel,
            &plans,
            EVALUATION_TIME,
            &policy(),
        )
        .unwrap();
        assert!(report.rejected_path_scenarios.iter().any(|rejected| {
            rejected.path_id == "A"
                && rejected.scenario_id == "nominal"
                && rejected.reason == FifoPathCapacityFailureV1::CapacityValidation(
                    ScheduledCapacityPlanError::ObservationTooOld
                )
        }));
    }

    #[test]
    fn scenario_plan_and_binding_order_do_not_change_the_report() {
        let travel = travel_report();
        let mut plans = plans_from_report(&travel);
        let first = select_capacity_qualified_robust_fifo_path(
            &travel,
            &plans,
            EVALUATION_TIME,
            &policy(),
        )
        .unwrap();
        plans.reverse();
        for plan in &mut plans {
            plan.resource_uses.reverse();
            plan.evidence.reverse();
        }
        let second = select_capacity_qualified_robust_fifo_path(
            &travel,
            &plans,
            EVALUATION_TIME,
            &policy(),
        )
        .unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn malformed_empty_path_estimate_is_rejected_before_capacity_scoring() {
        let mut travel = travel_report();
        travel.scores[0].observations[0].path_estimate.legs.clear();
        let plans = plans_from_report(&travel);
        assert!(matches!(
            select_capacity_qualified_robust_fifo_path(
                &travel,
                &plans,
                EVALUATION_TIME,
                &policy(),
            ),
            Err(FifoRobustCapacitySelectionErrorV1::InvalidSelectionReport(_))
        ));
    }

    #[test]
    fn duplicate_and_unexpected_plans_are_input_errors() {
        let travel = travel_report();
        let mut plans = plans_from_report(&travel);
        let duplicate = plans[0].clone();
        plans.push(duplicate);
        assert!(matches!(
            select_capacity_qualified_robust_fifo_path(&travel, &plans, EVALUATION_TIME, &policy()),
            Err(FifoRobustCapacitySelectionErrorV1::DuplicateCapacityPlan { .. })
        ));

        plans.pop();
        plans[0].scenario_id = "undeclared".into();
        assert!(matches!(
            select_capacity_qualified_robust_fifo_path(&travel, &plans, EVALUATION_TIME, &policy()),
            Err(FifoRobustCapacitySelectionErrorV1::UnknownCapacityPlanScenario { .. })
        ));
    }
}
