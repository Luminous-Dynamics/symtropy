// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact small-portfolio selection under shared physical-resource capacity.
//!
//! Per-route capacity qualification is not enough when multiple demands compete
//! for the same track, berth, terminal, transfer station, charging point, or
//! other resource. This module enumerates one individually qualified route per
//! demand and checks the selected portfolio jointly in every declared scenario.
//!
//! Exactness is limited to the supplied route options and paired scenarios. The
//! implementation is an auditable reference solver for bounded portfolios, not
//! a scalable VRP solver, live scheduler, reservation system, or dispatch engine.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::{
    CargoTransferCapacityResourceKind, FifoCapacityQualifiedPathScoreV1,
    FifoPathScenarioKindV1, FifoRobustCapacitySelectionReportV1,
    QuantityUnit, ScheduledCapacityAssignmentReceiptV1, ScheduledCapacityResourceSummaryV1,
};

/// Each demand supplies routes already qualified individually by the capacity
/// selector. objective_weight is an explicit policy input, never inferred.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoRobustDemandCapacityOptionsV1 {
    pub demand_id: String,
    pub objective_weight: u64,
    pub selection: FifoRobustCapacitySelectionReportV1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoRobustPortfolioDemandChoiceV1 {
    pub demand_id: String,
    pub path_id: String,
    pub objective_weight: u64,
    pub nominal_elapsed_seconds: u64,
    pub worst_case_elapsed_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoRobustPortfolioScenarioScoreV1 {
    pub scenario_id: String,
    pub kind: FifoPathScenarioKindV1,
    /// Sum of objective_weight * elapsed_seconds across selected demands.
    /// This is a declared weighted objective, not a physical unit unless every
    /// weight has the same explicit domain meaning.
    pub weighted_elapsed_score: u64,
    pub maximum_individual_elapsed_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FifoPortfolioInfeasibilityV1 {
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
pub struct FifoRejectedPortfolioCombinationV1 {
    pub choices: Vec<(String, String)>,
    pub reason: FifoPortfolioInfeasibilityV1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoRobustPortfolioReportV1 {
    pub scenario_set_id: String,
    pub scenario_manifest_sha256: String,
    pub time_profile_id: String,
    pub evaluation_time_epoch_seconds: u64,
    pub policy_id: String,
    pub demand_choices: Vec<FifoRobustPortfolioDemandChoiceV1>,
    pub scenario_scores: Vec<FifoRobustPortfolioScenarioScoreV1>,
    /// Scenario with the greatest weighted aggregate score; ID breaks ties.
    pub worst_case_scenario_id: String,
    /// Lexicographic objective: minimize the slowest single-demand elapsed
    /// time over all scenarios, then worst-case weighted aggregate elapsed,
    /// then nominal weighted aggregate elapsed, then stable demand/path IDs.
    pub worst_case_individual_elapsed_seconds: u64,
    pub worst_case_weighted_elapsed_score: u64,
    pub nominal_weighted_elapsed_score: u64,
    pub combinations_checked: usize,
    pub feasible_combinations_found: usize,
    pub infeasible_combinations_found: usize,
    /// First 32 deterministic examples; aggregate counts cover every combination.
    pub infeasibility_examples: Vec<FifoRejectedPortfolioCombinationV1>,
    pub exactness_scope: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FifoRobustPortfolioErrorV1 {
    EmptyDemands,
    InvalidDemand { demand_id: String },
    DuplicateDemandId(String),
    NoIndividuallyQualifiedPath { demand_id: String },
    IncompatibleScenarioSet { demand_id: String },
    IncompatibleEvaluationPolicy { demand_id: String },
    InvalidQualifiedPath { demand_id: String, path_id: String, reason: &'static str },
    DuplicatePathId { demand_id: String, path_id: String },
    CombinationCountOverflow,
    CombinationLimitExceeded { required: usize, limit: usize },
    ObjectiveArithmeticOverflow,
    NoFeasiblePortfolio {
        combinations_checked: usize,
        capacity_conflicts: usize,
        inconsistent_capacity_snapshots: usize,
    },
}

impl fmt::Display for FifoRobustPortfolioErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDemands => f.write_str("portfolio has no demands"),
            Self::InvalidDemand { demand_id } => {
                write!(f, "demand {demand_id} has a blank identity or zero objective weight")
            }
            Self::DuplicateDemandId(id) => write!(f, "portfolio repeats demand ID {id}"),
            Self::NoIndividuallyQualifiedPath { demand_id } => {
                write!(f, "demand {demand_id} has no individually capacity-qualified path")
            }
            Self::IncompatibleScenarioSet { demand_id } => {
                write!(f, "demand {demand_id} uses a different scenario set or manifest")
            }
            Self::IncompatibleEvaluationPolicy { demand_id } => {
                write!(f, "demand {demand_id} uses a different evaluation time, time profile, or capacity policy")
            }
            Self::InvalidQualifiedPath { demand_id, path_id, reason } => {
                write!(f, "path {path_id} for demand {demand_id} is invalid: {reason}")
            }
            Self::DuplicatePathId { demand_id, path_id } => {
                write!(f, "demand {demand_id} repeats path option ID {path_id}")
            }
            Self::CombinationCountOverflow => f.write_str("route-option combination count overflowed"),
            Self::CombinationLimitExceeded { required, limit } => {
                write!(f, "exact portfolio search requires {required} combinations, exceeding limit {limit}")
            }
            Self::ObjectiveArithmeticOverflow => {
                f.write_str("portfolio objective or concurrent resource arithmetic overflowed")
            }
            Self::NoFeasiblePortfolio { combinations_checked, capacity_conflicts, inconsistent_capacity_snapshots } => write!(
                f,
                "no portfolio is jointly feasible after {combinations_checked} combinations ({capacity_conflicts} capacity conflicts, {inconsistent_capacity_snapshots} inconsistent evidence snapshots)"
            ),
        }
    }
}

impl std::error::Error for FifoRobustPortfolioErrorV1 {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ResourceKey {
    kind: CargoTransferCapacityResourceKind,
    id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CapacitySnapshotFingerprint {
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
struct PreparedResourceUse {
    key: ResourceKey,
    fingerprint: CapacitySnapshotFingerprint,
    assignment_id: String,
    start: u64,
    end: u64,
    units: u64,
}

#[derive(Clone, Debug)]
struct PreparedOption {
    demand_id: String,
    path_id: String,
    objective_weight: u64,
    scenario_uses: BTreeMap<String, Vec<PreparedResourceUse>>,
    elapsed_by_scenario: BTreeMap<String, u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScenarioInventory {
    kinds: BTreeMap<String, FifoPathScenarioKindV1>,
    nominal_id: String,
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn checked_product(values: impl Iterator<Item = usize>) -> Option<usize> {
    values.try_fold(1_usize, usize::checked_mul)
}

fn scenario_inventory(
    selection: &FifoRobustCapacitySelectionReportV1,
    demand_id: &str,
) -> Result<ScenarioInventory, FifoRobustPortfolioErrorV1> {
    let Some(first) = selection.qualified_paths.first() else {
        return Err(FifoRobustPortfolioErrorV1::NoIndividuallyQualifiedPath {
            demand_id: demand_id.to_owned(),
        });
    };
    let mut kinds = BTreeMap::new();
    for observation in &first.path_score.observations {
        if observation.scenario_id.trim().is_empty()
            || kinds.insert(observation.scenario_id.clone(), observation.kind).is_some()
        {
            return Err(FifoRobustPortfolioErrorV1::InvalidQualifiedPath {
                demand_id: demand_id.to_owned(),
                path_id: first.path_score.path_id.clone(),
                reason: "scenario IDs are blank or duplicated",
            });
        }
    }
    let nominal_ids = kinds.iter()
        .filter(|(_, kind)| **kind == FifoPathScenarioKindV1::Nominal)
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    if nominal_ids.len() != 1 {
        return Err(FifoRobustPortfolioErrorV1::InvalidQualifiedPath {
            demand_id: demand_id.to_owned(),
            path_id: first.path_score.path_id.clone(),
            reason: "exactly one nominal scenario is required",
        });
    }
    Ok(ScenarioInventory {
        kinds,
        nominal_id: nominal_ids[0].clone(),
    })
}

fn summary_for_receipt<'a>(
    summaries: &'a [ScheduledCapacityResourceSummaryV1],
    receipt: &ScheduledCapacityAssignmentReceiptV1,
) -> Option<&'a ScheduledCapacityResourceSummaryV1> {
    let mut matched = summaries.iter().filter(|summary| {
        summary.resource_kind == receipt.resource_kind
            && summary.resource_id == receipt.resource_id
    });
    let one = matched.next()?;
    if matched.next().is_some() { return None; }
    Some(one)
}

fn fingerprint(
    summary: &ScheduledCapacityResourceSummaryV1,
    receipt: &ScheduledCapacityAssignmentReceiptV1,
) -> CapacitySnapshotFingerprint {
    CapacitySnapshotFingerprint {
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

fn validate_path_score(
    demand_id: &str,
    path: &FifoCapacityQualifiedPathScoreV1,
    selection: &FifoRobustCapacitySelectionReportV1,
    inventory: &ScenarioInventory,
) -> Result<(), FifoRobustPortfolioErrorV1> {
    let path_id = &path.path_score.path_id;
    let fail = |reason| FifoRobustPortfolioErrorV1::InvalidQualifiedPath {
        demand_id: demand_id.to_owned(),
        path_id: path_id.clone(),
        reason,
    };
    if path_id.trim().is_empty() || path.path_score.observations.len() != inventory.kinds.len() {
        return Err(fail("path ID or scenario coverage is invalid"));
    }
    let mut seen = BTreeSet::new();
    let mut common_endpoints: Option<(String, String)> = None;
    for observation in &path.path_score.observations {
        if inventory.kinds.get(&observation.scenario_id) != Some(&observation.kind)
            || !seen.insert(observation.scenario_id.clone())
            || observation.path_estimate.time_profile_id != selection.time_profile_id
            || observation.path_estimate.final_arrival_epoch_seconds
                != observation.final_arrival_epoch_seconds
            || observation.path_estimate.total_elapsed_seconds != observation.elapsed_seconds
            || observation.path_estimate.legs.is_empty()
        {
            return Err(fail("scenario observation is inconsistent or empty"));
        }
        let endpoints = (
            observation.path_estimate.origin_node_id.clone(),
            observation.path_estimate.destination_node_id.clone(),
        );
        if let Some(expected) = &common_endpoints {
            if expected != &endpoints {
                return Err(fail("path endpoints change across scenarios"));
            }
        } else {
            common_endpoints = Some(endpoints);
        }
    }
    let nominal = path.path_score.observations.iter()
        .find(|o| o.scenario_id == inventory.nominal_id)
        .map(|o| o.final_arrival_epoch_seconds)
        .ok_or_else(|| fail("nominal observation is missing"))?;
    let worst = path.path_score.observations.iter()
        .max_by(|a, b| a.final_arrival_epoch_seconds.cmp(&b.final_arrival_epoch_seconds)
            .then_with(|| b.scenario_id.cmp(&a.scenario_id)))
        .ok_or_else(|| fail("scenario observations are empty"))?;
    if path.path_score.nominal_arrival_epoch_seconds != nominal
        || path.path_score.worst_case_arrival_epoch_seconds != worst.final_arrival_epoch_seconds
        || path.path_score.worst_case_scenario_id != worst.scenario_id
    {
        return Err(fail("nominal or worst-case path score does not match the scenario observations"));
    }
    if path.scenario_capacity.len() != inventory.kinds.len() {
        return Err(fail("scenario capacity report count is incomplete"));
    }
    let mut seen_capacity = BTreeSet::new();
    for scenario_report in &path.scenario_capacity {
        if inventory.kinds.get(&scenario_report.scenario_id).is_none()
            || !seen_capacity.insert(scenario_report.scenario_id.clone())
            || scenario_report.report.policy_id != selection.policy_id
            || scenario_report.report.time_profile_id != selection.time_profile_id
            || scenario_report.report.evaluation_time_epoch_seconds
                != selection.evaluation_time_epoch_seconds
        {
            return Err(fail("scenario capacity report does not match the shared evaluation contract"));
        }
        let observation = path.path_score.observations.iter()
            .find(|o| o.scenario_id == scenario_report.scenario_id)
            .ok_or_else(|| fail("capacity report references an unknown route observation"))?;
        if scenario_report.report.assignments.len() < observation.path_estimate.legs.len() {
            return Err(fail("capacity receipts do not cover every evaluated route leg"));
        }
    }
    Ok(())
}

fn prepare_option(
    demand_id: &str,
    objective_weight: u64,
    selection: &FifoRobustCapacitySelectionReportV1,
    path: &FifoCapacityQualifiedPathScoreV1,
    inventory: &ScenarioInventory,
) -> Result<PreparedOption, FifoRobustPortfolioErrorV1> {
    validate_path_score(demand_id, path, selection, inventory)?;
    let mut scenario_uses: BTreeMap<String, Vec<PreparedResourceUse>> = BTreeMap::new();
    let mut elapsed_by_scenario = BTreeMap::new();

    for observation in &path.path_score.observations {
        elapsed_by_scenario.insert(observation.scenario_id.clone(), observation.elapsed_seconds);
    }
    for scenario_capacity in &path.scenario_capacity {
        let report = &scenario_capacity.report;
        let mut assignment_ids = BTreeSet::new();
        let mut snapshot_by_resource: BTreeMap<ResourceKey, CapacitySnapshotFingerprint> = BTreeMap::new();
        let mut uses = Vec::new();
        for receipt in &report.assignments {
            if receipt.assignment_id.trim().is_empty()
                || receipt.evidence_id.trim().is_empty()
                || receipt.resource_id.trim().is_empty()
                || receipt.capacity_scope_id.trim().is_empty()
                || receipt.capacity_units == 0
                || receipt.window.time_profile_id != selection.time_profile_id
                || receipt.window.planned_start_epoch_seconds >= receipt.window.planned_end_epoch_seconds
                || !assignment_ids.insert(receipt.assignment_id.clone())
            {
                return Err(FifoRobustPortfolioErrorV1::InvalidQualifiedPath {
                    demand_id: demand_id.to_owned(),
                    path_id: path.path_score.path_id.clone(),
                    reason: "capacity receipt identity, quantity, interval, or uniqueness is invalid",
                });
            }
            let summary = summary_for_receipt(&report.resources, receipt)
                .ok_or_else(|| FifoRobustPortfolioErrorV1::InvalidQualifiedPath {
                    demand_id: demand_id.to_owned(),
                    path_id: path.path_score.path_id.clone(),
                    reason: "capacity receipt has no unique resource summary",
                })?;
            if summary.evidence_id != receipt.evidence_id
                || summary.resource_kind != receipt.resource_kind
                || summary.resource_id != receipt.resource_id
                || summary.capacity_scope_id != receipt.capacity_scope_id
                || summary.capacity_unit != receipt.capacity_unit
                || summary.capacity_limit_units == 0
                || receipt.capacity_units > summary.capacity_limit_units
                || !summary.assignment_ids.contains(&receipt.assignment_id)
                || summary.maximum_concurrent_units > summary.capacity_limit_units
                || receipt.window.planned_start_epoch_seconds
                    < summary.first_planned_start_epoch_seconds
                || receipt.window.planned_end_epoch_seconds
                    > summary.last_planned_end_epoch_seconds
            {
                return Err(FifoRobustPortfolioErrorV1::InvalidQualifiedPath {
                    demand_id: demand_id.to_owned(),
                    path_id: path.path_score.path_id.clone(),
                    reason: "capacity receipt and resource summary disagree",
                });
            }
            let resource_key = ResourceKey {
                kind: receipt.resource_kind,
                id: receipt.resource_id.clone(),
            };
            let source_fingerprint = fingerprint(summary, receipt);
            if let Some(previous) = snapshot_by_resource.insert(resource_key.clone(), source_fingerprint.clone()) {
                if previous != source_fingerprint {
                    return Err(FifoRobustPortfolioErrorV1::InvalidQualifiedPath {
                        demand_id: demand_id.to_owned(),
                        path_id: path.path_score.path_id.clone(),
                        reason: "one scenario contains conflicting capacity evidence for a resource",
                    });
                }
            }
            uses.push(PreparedResourceUse {
                key: resource_key,
                fingerprint: source_fingerprint,
                assignment_id: receipt.assignment_id.clone(),
                start: receipt.window.planned_start_epoch_seconds,
                end: receipt.window.planned_end_epoch_seconds,
                units: receipt.capacity_units,
            });
        }
        if report.resources.len() != snapshot_by_resource.len() {
            return Err(FifoRobustPortfolioErrorV1::InvalidQualifiedPath {
                demand_id: demand_id.to_owned(),
                path_id: path.path_score.path_id.clone(),
                reason: "resource summaries include unused or unreported resources",
            });
        }
        uses.sort_by(|a, b| {
            (&a.key, a.start, a.end, &a.assignment_id)
                .cmp(&(&b.key, b.start, b.end, &b.assignment_id))
        });
        scenario_uses.insert(scenario_capacity.scenario_id.clone(), uses);
    }

    Ok(PreparedOption {
        demand_id: demand_id.to_owned(),
        path_id: path.path_score.path_id.clone(),
        objective_weight,
        scenario_uses,
        elapsed_by_scenario,
    })
}

fn joint_capacity_check(
    chosen: &[&PreparedOption],
    scenario_id: &str,
) -> Result<(), FifoPortfolioInfeasibilityV1> {
    #[derive(Clone)]
    struct Event {
        time: u64,
        is_start: bool,
        assignment_id: String,
        units: u64,
        limit: u64,
        key: ResourceKey,
    }

    let mut snapshots: BTreeMap<ResourceKey, CapacitySnapshotFingerprint> = BTreeMap::new();
    let mut events_by_resource: BTreeMap<ResourceKey, Vec<Event>> = BTreeMap::new();
    let mut assignment_ids = BTreeSet::new();

    for option in chosen {
        if let Some(uses) = option.scenario_uses.get(scenario_id) {
            for use_ in uses {
                if !assignment_ids.insert(use_.assignment_id.clone()) {
                    return Err(FifoPortfolioInfeasibilityV1::DuplicateAssignmentId {
                        scenario_id: scenario_id.to_owned(),
                        assignment_id: use_.assignment_id.clone(),
                    });
                }
                if let Some(previous) = snapshots.get(&use_.key) {
                    if previous != &use_.fingerprint {
                        return Err(FifoPortfolioInfeasibilityV1::InconsistentCapacityEvidence {
                            scenario_id: scenario_id.to_owned(),
                            resource_kind: use_.key.kind,
                            resource_id: use_.key.id.clone(),
                        });
                    }
                } else {
                    snapshots.insert(use_.key.clone(), use_.fingerprint.clone());
                }
                let limit = use_.fingerprint.capacity_limit_units;
                let events = events_by_resource.entry(use_.key.clone()).or_default();
                events.push(Event {
                    time: use_.start,
                    is_start: true,
                    assignment_id: use_.assignment_id.clone(),
                    units: use_.units,
                    limit,
                    key: use_.key.clone(),
                });
                events.push(Event {
                    time: use_.end,
                    is_start: false,
                    assignment_id: use_.assignment_id.clone(),
                    units: use_.units,
                    limit,
                    key: use_.key.clone(),
                });
            }
        }
    }

    for events in events_by_resource.values_mut() {
        // End events sort before start events at the same timestamp: [start,end).
        events.sort_by(|a, b| (a.time, a.is_start, &a.assignment_id)
            .cmp(&(b.time, b.is_start, &b.assignment_id)));
        let mut active: BTreeMap<String, u64> = BTreeMap::new();
        let mut active_units = 0_u64;
        for event in events.iter() {
            if !event.is_start {
                let Some(units) = active.remove(&event.assignment_id) else {
                    return Err(FifoPortfolioInfeasibilityV1::DuplicateAssignmentId {
                        scenario_id: scenario_id.to_owned(),
                        assignment_id: event.assignment_id.clone(),
                    });
                };
                active_units = active_units.checked_sub(units).ok_or_else(|| {
                    FifoPortfolioInfeasibilityV1::ConcurrentCapacityExceeded {
                        scenario_id: scenario_id.to_owned(),
                        resource_kind: event.key.kind,
                        resource_id: event.key.id.clone(),
                        at_epoch_seconds: event.time,
                        attempted_units: u64::MAX,
                        limit_units: event.limit,
                    }
                })?;
                continue;
            }
            active_units = active_units.checked_add(event.units).ok_or_else(|| {
                FifoPortfolioInfeasibilityV1::ConcurrentCapacityExceeded {
                    scenario_id: scenario_id.to_owned(),
                    resource_kind: event.key.kind,
                    resource_id: event.key.id.clone(),
                    at_epoch_seconds: event.time,
                    attempted_units: u64::MAX,
                    limit_units: event.limit,
                }
            })?;
            if active_units > event.limit {
                return Err(FifoPortfolioInfeasibilityV1::ConcurrentCapacityExceeded {
                    scenario_id: scenario_id.to_owned(),
                    resource_kind: event.key.kind,
                    resource_id: event.key.id.clone(),
                    at_epoch_seconds: event.time,
                    attempted_units: active_units,
                    limit_units: event.limit,
                });
            }
            if active.insert(event.assignment_id.clone(), event.units).is_some() {
                return Err(FifoPortfolioInfeasibilityV1::DuplicateAssignmentId {
                    scenario_id: scenario_id.to_owned(),
                    assignment_id: event.assignment_id.clone(),
                });
            }
        }
    }
    Ok(())
}

fn scenario_score(
    chosen: &[&PreparedOption],
    scenario_id: &str,
    kind: FifoPathScenarioKindV1,
) -> Result<FifoRobustPortfolioScenarioScoreV1, FifoRobustPortfolioErrorV1> {
    let mut weighted_elapsed_score = 0_u64;
    let mut maximum_individual_elapsed_seconds = 0_u64;
    for option in chosen {
        let elapsed = *option.elapsed_by_scenario.get(scenario_id)
            .ok_or_else(|| FifoRobustPortfolioErrorV1::InvalidQualifiedPath {
                demand_id: option.demand_id.clone(),
                path_id: option.path_id.clone(),
                reason: "elapsed time is missing for a scenario",
            })?;
        let weighted = elapsed.checked_mul(option.objective_weight)
            .ok_or(FifoRobustPortfolioErrorV1::ObjectiveArithmeticOverflow)?;
        weighted_elapsed_score = weighted_elapsed_score.checked_add(weighted)
            .ok_or(FifoRobustPortfolioErrorV1::ObjectiveArithmeticOverflow)?;
        maximum_individual_elapsed_seconds = maximum_individual_elapsed_seconds.max(elapsed);
    }
    Ok(FifoRobustPortfolioScenarioScoreV1 {
        scenario_id: scenario_id.to_owned(),
        kind,
        weighted_elapsed_score,
        maximum_individual_elapsed_seconds,
    })
}

#[derive(Clone)]
struct PortfolioScore {
    chosen: Vec<usize>,
    choices_key: Vec<(String, String)>,
    scenario_scores: Vec<FifoRobustPortfolioScenarioScoreV1>,
    worst_case_scenario_id: String,
    worst_individual: u64,
    worst_weighted: u64,
    nominal_weighted: u64,
}

fn compare_scores(a: &PortfolioScore, b: &PortfolioScore) -> Ordering {
    (
        a.worst_individual,
        a.worst_weighted,
        a.nominal_weighted,
        &a.choices_key,
    )
        .cmp(&(
            b.worst_individual,
            b.worst_weighted,
            b.nominal_weighted,
            &b.choices_key,
        ))
}

/// Search every combination of one individually capacity-qualified path per
/// demand. All route options must share one scenario manifest, time profile,
/// evaluation time and capacity policy. Objective order: worst individual
/// elapsed time across all scenarios, worst weighted aggregate elapsed score,
/// nominal weighted aggregate score, then stable IDs. max_combinations is a hard
/// guard, not a pruning heuristic.
pub fn select_joint_robust_transport_portfolio(
    demands: &[FifoRobustDemandCapacityOptionsV1],
    max_combinations: usize,
) -> Result<FifoRobustPortfolioReportV1, FifoRobustPortfolioErrorV1> {
    if demands.is_empty() {
        return Err(FifoRobustPortfolioErrorV1::EmptyDemands);
    }

    let mut demand_ids = BTreeSet::new();
    for demand in demands {
        if demand.demand_id.trim().is_empty() || demand.objective_weight == 0 {
            return Err(FifoRobustPortfolioErrorV1::InvalidDemand {
                demand_id: demand.demand_id.clone(),
            });
        }
        if !demand_ids.insert(demand.demand_id.clone()) {
            return Err(FifoRobustPortfolioErrorV1::DuplicateDemandId(demand.demand_id.clone()));
        }
        if demand.selection.qualified_paths.is_empty()
            || demand.selection.capacity_qualified_path_count != demand.selection.qualified_paths.len()
        {
            return Err(FifoRobustPortfolioErrorV1::NoIndividuallyQualifiedPath {
                demand_id: demand.demand_id.clone(),
            });
        }
        if demand.selection.scenario_set_id.trim().is_empty()
            || !valid_sha256(&demand.selection.scenario_manifest_sha256)
            || demand.selection.time_profile_id.trim().is_empty()
            || demand.selection.policy_id.trim().is_empty()
        {
            return Err(FifoRobustPortfolioErrorV1::InvalidDemand {
                demand_id: demand.demand_id.clone(),
            });
        }
    }

    let first_demand = &demands[0];
    let inventory = scenario_inventory(&first_demand.selection, &first_demand.demand_id)?;
    for demand in demands.iter().skip(1) {
        let selection = &demand.selection;
        if selection.scenario_set_id != first_demand.selection.scenario_set_id
            || selection.scenario_manifest_sha256 != first_demand.selection.scenario_manifest_sha256
        {
            return Err(FifoRobustPortfolioErrorV1::IncompatibleScenarioSet {
                demand_id: demand.demand_id.clone(),
            });
        }
        if selection.time_profile_id != first_demand.selection.time_profile_id
            || selection.evaluation_time_epoch_seconds
                != first_demand.selection.evaluation_time_epoch_seconds
            || selection.policy_id != first_demand.selection.policy_id
        {
            return Err(FifoRobustPortfolioErrorV1::IncompatibleEvaluationPolicy {
                demand_id: demand.demand_id.clone(),
            });
        }
        let demand_inventory = scenario_inventory(selection, &demand.demand_id)?;
        if demand_inventory != inventory {
            return Err(FifoRobustPortfolioErrorV1::IncompatibleScenarioSet {
                demand_id: demand.demand_id.clone(),
            });
        }
    }

    let mut sorted_demands = demands.iter().collect::<Vec<_>>();
    sorted_demands.sort_by(|a, b| a.demand_id.cmp(&b.demand_id));

    let required = checked_product(sorted_demands.iter().map(|d| d.selection.qualified_paths.len()))
        .ok_or(FifoRobustPortfolioErrorV1::CombinationCountOverflow)?;
    if max_combinations == 0 || required > max_combinations {
        return Err(FifoRobustPortfolioErrorV1::CombinationLimitExceeded {
            required,
            limit: max_combinations,
        });
    }

    let mut prepared_demands: Vec<Vec<PreparedOption>> = Vec::new();
    for demand in &sorted_demands {
        let mut paths = demand.selection.qualified_paths.iter().collect::<Vec<_>>();
        paths.sort_by(|a, b| a.path_score.path_id.cmp(&b.path_score.path_id));
        let mut path_ids = BTreeSet::new();
        let mut prepared = Vec::new();
        let mut common_endpoints: Option<(String, String)> = None;
        for path in paths {
            if !path_ids.insert(path.path_score.path_id.clone()) {
                return Err(FifoRobustPortfolioErrorV1::DuplicatePathId {
                    demand_id: demand.demand_id.clone(),
                    path_id: path.path_score.path_id.clone(),
                });
            }
            validate_path_score(&demand.demand_id, path, &demand.selection, &inventory)?;
            if let Some(first) = path.path_score.observations.first() {
                let endpoints = (
                    first.path_estimate.origin_node_id.clone(),
                    first.path_estimate.destination_node_id.clone(),
                );
                if let Some(expected) = &common_endpoints {
                    if expected != &endpoints {
                        return Err(FifoRobustPortfolioErrorV1::InvalidQualifiedPath {
                            demand_id: demand.demand_id.clone(),
                            path_id: path.path_score.path_id.clone(),
                            reason: "options for one demand have different origin/destination pairs",
                        });
                    }
                } else {
                    common_endpoints = Some(endpoints);
                }
            }
            prepared.push(prepare_option(
                &demand.demand_id,
                demand.objective_weight,
                &demand.selection,
                path,
                &inventory,
            )?);
        }
        prepared_demands.push(prepared);
    }

    let mut combinations_checked = 0_usize;
    let mut feasible_combinations_found = 0_usize;
    let mut infeasible_combinations_found = 0_usize;
    let mut capacity_conflicts = 0_usize;
    let mut inconsistent_capacity_snapshots = 0_usize;
    let mut infeasibility_examples = Vec::new();
    let mut best: Option<PortfolioScore> = None;
    let mut chosen_indices = vec![0_usize; prepared_demands.len()];

    fn visit(
        index: usize,
        prepared_demands: &[Vec<PreparedOption>],
        chosen_indices: &mut [usize],
        inventory: &ScenarioInventory,
        combinations_checked: &mut usize,
        feasible_combinations_found: &mut usize,
        infeasible_combinations_found: &mut usize,
        capacity_conflicts: &mut usize,
        inconsistent_capacity_snapshots: &mut usize,
        infeasibility_examples: &mut Vec<FifoRejectedPortfolioCombinationV1>,
        best: &mut Option<PortfolioScore>,
    ) -> Result<(), FifoRobustPortfolioErrorV1> {
        if index < prepared_demands.len() {
            for option_index in 0..prepared_demands[index].len() {
                chosen_indices[index] = option_index;
                visit(
                    index + 1,
                    prepared_demands,
                    chosen_indices,
                    inventory,
                    combinations_checked,
                    feasible_combinations_found,
                    infeasible_combinations_found,
                    capacity_conflicts,
                    inconsistent_capacity_snapshots,
                    infeasibility_examples,
                    best,
                )?;
            }
            return Ok(());
        }

        *combinations_checked += 1;
        let chosen = prepared_demands.iter().zip(chosen_indices.iter())
            .map(|(options, choice)| &options[*choice])
            .collect::<Vec<_>>();
        let choice_key = chosen.iter()
            .map(|option| (option.demand_id.clone(), option.path_id.clone()))
            .collect::<Vec<_>>();

        for scenario_id in inventory.kinds.keys() {
            if let Err(reason) = joint_capacity_check(&chosen, scenario_id) {
                *infeasible_combinations_found += 1;
                match &reason {
                    FifoPortfolioInfeasibilityV1::ConcurrentCapacityExceeded { .. } => {
                        *capacity_conflicts += 1;
                    }
                    FifoPortfolioInfeasibilityV1::InconsistentCapacityEvidence { .. } => {
                        *inconsistent_capacity_snapshots += 1;
                    }
                    FifoPortfolioInfeasibilityV1::DuplicateAssignmentId { .. } => {
                        *capacity_conflicts += 1;
                    }
                }
                if infeasibility_examples.len() < 32 {
                    infeasibility_examples.push(FifoRejectedPortfolioCombinationV1 {
                        choices: choice_key,
                        reason,
                    });
                }
                return Ok(());
            }
        }

        let mut scenario_scores = Vec::new();
        for (scenario_id, kind) in &inventory.kinds {
            scenario_scores.push(scenario_score(&chosen, scenario_id, *kind)?);
        }
        scenario_scores.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));
        let worst_individual = scenario_scores.iter()
            .map(|s| s.maximum_individual_elapsed_seconds)
            .max()
            .unwrap_or(0);
        let worst_weighted = scenario_scores.iter()
            .map(|s| s.weighted_elapsed_score)
            .max()
            .unwrap_or(0);
        let nominal_weighted = scenario_scores.iter()
            .find(|s| s.scenario_id == inventory.nominal_id)
            .map(|s| s.weighted_elapsed_score)
            .ok_or(FifoRobustPortfolioErrorV1::ObjectiveArithmeticOverflow)?;
        let worst_case_scenario_id = scenario_scores.iter()
            .max_by(|a, b| a.weighted_elapsed_score.cmp(&b.weighted_elapsed_score)
                .then_with(|| b.scenario_id.cmp(&a.scenario_id)))
            .map(|s| s.scenario_id.clone())
            .ok_or(FifoRobustPortfolioErrorV1::ObjectiveArithmeticOverflow)?;

        let score = PortfolioScore {
            chosen: chosen_indices.to_vec(),
            choices_key: choice_key,
            scenario_scores,
            worst_case_scenario_id,
            worst_individual,
            worst_weighted,
            nominal_weighted,
        };
        *feasible_combinations_found += 1;
        if best.as_ref().is_none_or(|current| compare_scores(&score, current) == Ordering::Less) {
            *best = Some(score);
        }
        Ok(())
    }

    visit(
        0,
        &prepared_demands,
        &mut chosen_indices,
        &inventory,
        &mut combinations_checked,
        &mut feasible_combinations_found,
        &mut infeasible_combinations_found,
        &mut capacity_conflicts,
        &mut inconsistent_capacity_snapshots,
        &mut infeasibility_examples,
        &mut best,
    )?;

    let Some(best) = best else {
        return Err(FifoRobustPortfolioErrorV1::NoFeasiblePortfolio {
            combinations_checked,
            capacity_conflicts,
            inconsistent_capacity_snapshots,
        });
    };

    let mut demand_choices = Vec::new();
    for (index, options) in prepared_demands.iter().enumerate() {
        let option = &options[best.chosen[index]];
        let nominal_elapsed_seconds = *option.elapsed_by_scenario.get(&inventory.nominal_id)
            .ok_or(FifoRobustPortfolioErrorV1::ObjectiveArithmeticOverflow)?;
        let worst_case_elapsed_seconds = *option.elapsed_by_scenario.values().max()
            .ok_or(FifoRobustPortfolioErrorV1::ObjectiveArithmeticOverflow)?;
        demand_choices.push(FifoRobustPortfolioDemandChoiceV1 {
            demand_id: option.demand_id.clone(),
            path_id: option.path_id.clone(),
            objective_weight: option.objective_weight,
            nominal_elapsed_seconds,
            worst_case_elapsed_seconds,
        });
    }
    demand_choices.sort_by(|a, b| a.demand_id.cmp(&b.demand_id));

    Ok(FifoRobustPortfolioReportV1 {
        scenario_set_id: first_demand.selection.scenario_set_id.clone(),
        scenario_manifest_sha256: first_demand.selection.scenario_manifest_sha256.clone(),
        time_profile_id: first_demand.selection.time_profile_id.clone(),
        evaluation_time_epoch_seconds: first_demand.selection.evaluation_time_epoch_seconds,
        policy_id: first_demand.selection.policy_id.clone(),
        demand_choices,
        scenario_scores: best.scenario_scores,
        worst_case_scenario_id: best.worst_case_scenario_id,
        worst_case_individual_elapsed_seconds: best.worst_individual,
        worst_case_weighted_elapsed_score: best.worst_weighted,
        nominal_weighted_elapsed_score: best.nominal_weighted,
        combinations_checked,
        feasible_combinations_found,
        infeasible_combinations_found,
        infeasibility_examples,
        exactness_scope: "ExactLexicographicPortfolioSelectionOverSuppliedIndividuallyQualifiedPathsAndDeclaredScenarios",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ArrivalFunctionEvidenceStatusV1, ArrivalFunctionEvidenceV1, ArrivalTimeBreakpointV1,
        CargoTransferCapacityEvidenceStatus, FifoArrivalTimeFunctionV1,
        FifoPathLegV1, FifoPathScenarioDefinitionV1, FifoPathScenarioSetV1,
        FifoRobustCandidatePathV1, FifoScenarioCandidatePathV1, ScheduledCapacityAssignmentV1,
        ScheduledCapacityEvidenceV1, ScheduledCapacityPolicyV1, OperationWindowV1,
        select_robust_fifo_path, select_capacity_qualified_robust_fifo_path,
    };

    const PROFILE: &str = "UTC_UNIX_SECONDS_V1";
    const EVALUATION_TIME: u64 = 99;

    fn scenario_set() -> FifoPathScenarioSetV1 {
        FifoPathScenarioSetV1 {
            scenario_set_id: "portfolio-scenarios-v1".into(),
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

    fn candidate(
        path_id: &str,
        origin: &str,
        destination: &str,
        nominal: u64,
        stress: u64,
    ) -> FifoRobustCandidatePathV1 {
        let edge = format!("edge-{path_id}");
        FifoRobustCandidatePathV1 {
            path_id: path_id.into(),
            scenario_paths: vec![
                FifoScenarioCandidatePathV1 {
                    scenario_id: "nominal".into(),
                    legs: vec![FifoPathLegV1 {
                        edge_id: edge.clone(),
                        from_node_id: origin.into(),
                        to_node_id: destination.into(),
                        pre_leg_delay_seconds: 0,
                        arrival_function: function(&format!("{path_id}-nominal"), nominal),
                    }],
                },
                FifoScenarioCandidatePathV1 {
                    scenario_id: "stress".into(),
                    legs: vec![FifoPathLegV1 {
                        edge_id: edge,
                        from_node_id: origin.into(),
                        to_node_id: destination.into(),
                        pre_leg_delay_seconds: 0,
                        arrival_function: function(&format!("{path_id}-stress"), stress),
                    }],
                },
            ],
        }
    }

    fn policy() -> ScheduledCapacityPolicyV1 {
        ScheduledCapacityPolicyV1 {
            policy_id: "portfolio-capacity-policy-v1".into(),
            expected_time_profile_id: PROFILE.into(),
            max_observation_age_seconds: 1_000,
            allowed_future_clock_skew_seconds: 0,
        }
    }

    fn demand_selection(
        demand_id: &str,
        candidates: Vec<(String, u64, u64, String, String)>,
        origin: &str,
        destination: &str,
        departure: u64,
    ) -> FifoRobustCapacitySelectionReportV1 {
        let paths = candidates.iter().map(|(path, nominal, stress, _, _)| {
            candidate(path, origin, destination, *nominal, *stress)
        }).collect::<Vec<_>>();
        let travel = select_robust_fifo_path(&scenario_set(), &paths, departure, 90).unwrap();
        let mut plans = Vec::new();
        for score in &travel.scores {
            let original = candidates.iter().find(|(id, _, _, _, _)| id == &score.path_id).unwrap();
            for observation in &score.observations {
                let resource_id = if observation.scenario_id == "nominal" {
                    original.3.clone()
                } else {
                    original.4.clone()
                };
                let budget_id = format!("snapshot-{resource_id}-v1");
                let assignment = ScheduledCapacityAssignmentV1 {
                    assignment_id: format!("assignment-{demand_id}-{}-{}", score.path_id, observation.scenario_id),
                    evidence_id: budget_id.clone(),
                    resource_kind: CargoTransferCapacityResourceKind::TransferEdge,
                    resource_id: resource_id.clone(),
                    capacity_scope_id: "shared-capacity-scope-v1".into(),
                    capacity_units: 1,
                    capacity_unit: QuantityUnit::Shipments,
                    terminal_id: None,
                    window: OperationWindowV1 {
                        time_profile_id: PROFILE.into(),
                        planned_start_epoch_seconds: observation.path_estimate.legs[0].estimate.departure_epoch_seconds,
                        planned_end_epoch_seconds: observation.path_estimate.legs[0].estimate.arrival_epoch_seconds,
                    },
                };
                plans.push(crate::FifoScenarioCapacityPlanV1 {
                    path_id: score.path_id.clone(),
                    scenario_id: observation.scenario_id.clone(),
                    resource_uses: vec![crate::FifoPathResourceUseV1 {
                        edge_id: observation.path_estimate.legs[0].edge_id.clone(),
                        crosswalk_id: "crosswalk-shared-v1".into(),
                        crosswalk_manifest_sha256: "c".repeat(64),
                        assignment,
                    }],
                    evidence: vec![ScheduledCapacityEvidenceV1 {
                        budget_id,
                        resource_kind: CargoTransferCapacityResourceKind::TransferEdge,
                        resource_id,
                        terminal_id: None,
                        capacity_scope_id: "shared-capacity-scope-v1".into(),
                        capacity_units: 1,
                        capacity_unit: QuantityUnit::Shipments,
                        authority_id: "resource-owner".into(),
                        authority_profile_id: "resource-capacity-profile-v1".into(),
                        source_record_id: format!("record-{resource_id}-v1"),
                        source_schema_profile_id: "resource-schema-v1".into(),
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
                    }],
                });
            }
        }
        select_capacity_qualified_robust_fifo_path(&travel, &plans, EVALUATION_TIME, &policy()).unwrap()
    }

    fn demand(
        demand_id: &str,
        candidates: Vec<(String, u64, u64, String, String)>,
        origin: &str,
        destination: &str,
    ) -> FifoRobustDemandCapacityOptionsV1 {
        demand_at(demand_id, candidates, origin, destination, 100)
    }

    fn demand_at(
        demand_id: &str,
        candidates: Vec<(String, u64, u64, String, String)>,
        origin: &str,
        destination: &str,
        departure: u64,
    ) -> FifoRobustDemandCapacityOptionsV1 {
        FifoRobustDemandCapacityOptionsV1 {
            demand_id: demand_id.into(),
            objective_weight: 1,
            selection: demand_selection(demand_id, candidates, origin, destination, departure),
        }
    }

    #[test]
    fn jointly_selects_alternatives_to_avoid_overbooking_a_shared_bottleneck() {
        let d1 = demand(
            "demand-1",
            vec![
                ("shared-fast".into(), 100, 100, "bridge".into(), "bridge".into()),
                ("alternate-1".into(), 140, 140, "spur-1".into(), "spur-1".into()),
            ],
            "origin-1",
            "dest-1",
        );
        let d2 = demand(
            "demand-2",
            vec![
                ("shared-fast".into(), 110, 110, "bridge".into(), "bridge".into()),
                ("alternate-2".into(), 145, 145, "spur-2".into(), "spur-2".into()),
            ],
            "origin-2",
            "dest-2",
        );
        let report = select_joint_robust_transport_portfolio(&[d1, d2], 4).unwrap();
        assert_eq!(report.combinations_checked, 4);
        assert_eq!(report.feasible_combinations_found, 3);
        assert_eq!(report.infeasible_combinations_found, 1);
        let chosen = report.demand_choices.iter()
            .map(|choice| (choice.demand_id.as_str(), choice.path_id.as_str()))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(chosen["demand-1"], "alternate-1");
        assert_eq!(chosen["demand-2"], "shared-fast");
        assert!(report.infeasibility_examples.iter().any(|example| {
            matches!(&example.reason, FifoPortfolioInfeasibilityV1::ConcurrentCapacityExceeded { .. })
        }));
    }

    #[test]
    fn overlapping_single_option_demands_fail_closed() {
        let d1 = demand(
            "demand-1",
            vec![("only-route-1".into(), 100, 100, "one-bridge".into(), "one-bridge".into())],
            "origin-1",
            "dest-1",
        );
        let d2 = demand(
            "demand-2",
            vec![("only-route-2".into(), 100, 100, "one-bridge".into(), "one-bridge".into())],
            "origin-2",
            "dest-2",
        );
        assert!(matches!(
            select_joint_robust_transport_portfolio(&[d1, d2], 1),
            Err(FifoRobustPortfolioErrorV1::NoFeasiblePortfolio {
                combinations_checked: 1,
                capacity_conflicts: 1,
                ..
            })
        ));
    }

    #[test]
    fn capacity_is_shared_only_when_intervals_do_not_overlap() {
        let d1 = demand(
            "demand-1",
            vec![("early".into(), 100, 100, "bridge".into(), "bridge".into())],
            "origin-1",
            "dest-1",
        );
        let d2 = demand_at(
            "demand-2",
            vec![("later".into(), 100, 100, "bridge".into(), "bridge".into())],
            "origin-2",
            "dest-2",
            200,
        );
        let report = select_joint_robust_transport_portfolio(&[d1, d2], 1).unwrap();
        assert_eq!(report.feasible_combinations_found, 1);
        assert_eq!(report.infeasible_combinations_found, 0);
    }

    #[test]
    fn demand_and_option_order_do_not_change_the_selection() {
        let d1 = demand(
            "demand-1",
            vec![
                ("shared-fast".into(), 100, 100, "bridge".into(), "bridge".into()),
                ("alternate-1".into(), 140, 140, "spur-1".into(), "spur-1".into()),
            ],
            "origin-1",
            "dest-1",
        );
        let d2 = demand(
            "demand-2",
            vec![
                ("shared-fast".into(), 110, 110, "bridge".into(), "bridge".into()),
                ("alternate-2".into(), 145, 145, "spur-2".into(), "spur-2".into()),
            ],
            "origin-2",
            "dest-2",
        );
        let first = select_joint_robust_transport_portfolio(&[d1.clone(), d2.clone()], 4).unwrap();
        let second = select_joint_robust_transport_portfolio(&[d2, d1], 4).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn exact_search_refuses_an_excessive_cartesian_product() {
        let d1 = demand(
            "demand-1",
            vec![
                ("a".into(), 100, 100, "a".into(), "a".into()),
                ("b".into(), 110, 110, "b".into(), "b".into()),
            ],
            "origin-1",
            "dest-1",
        );
        let d2 = demand(
            "demand-2",
            vec![
                ("c".into(), 100, 100, "c".into(), "c".into()),
                ("d".into(), 110, 110, "d".into(), "d".into()),
            ],
            "origin-2",
            "dest-2",
        );
        assert_eq!(
            select_joint_robust_transport_portfolio(&[d1, d2], 3),
            Err(FifoRobustPortfolioErrorV1::CombinationLimitExceeded { required: 4, limit: 3 })
        );
    }
}
