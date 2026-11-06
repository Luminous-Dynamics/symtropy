// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Temporal service-cascade reference over an explicit dependency DAG.
//!
//! This crate models service functionality, not physical inventories or flows.
//! Physical stocks, transfers and conservation remain the responsibility of
//! the CivDyn resource ledger. Inputs here are exogenous component availability
//! traces, and outputs are downstream service trajectories.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

const TOLERANCE: f64 = 1e-12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyMode {
    /// Every input in the group must meet its declared functional requirement.
    AllOf,
    /// Any input can provide the group's required function (redundant supply).
    AnyOf,
}

impl DependencyMode {
    pub const fn name(self) -> &'static str {
        match self {
            Self::AllOf => "all_of",
            Self::AnyOf => "any_of",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DependencyInput {
    pub parent_id: String,
    /// Parent level at which this dependency can support full child function.
    pub full_service_fraction: f64,
    /// Below this fraction the dependency supplies no function to the child.
    pub minimum_operating_fraction: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DependencyGroup {
    pub mode: DependencyMode,
    pub inputs: Vec<DependencyInput>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServiceNode {
    pub id: String,
    pub service_type: String,
    /// Demand weight for an externally consumed service. Set to zero for
    /// intermediate assets to avoid counting the same downstream service twice.
    pub beneficiary_demand_weight: f64,
    /// Multiple groups are ANDed; each group may itself be ALL-of or ANY-of.
    pub dependency_groups: Vec<DependencyGroup>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scenario {
    pub id: String,
    pub nodes: Vec<ServiceNode>,
    /// Tick-major exogenous own availability, in original node order, each [0, 1].
    pub own_availability_by_tick: Vec<Vec<f64>>,
    /// First tick after the declared exogenous shock window.
    pub recovery_start_tick: usize,
    pub recovery_threshold: f64,
    pub sustained_recovery_steps: usize,
}

impl Scenario {
    pub fn validate(&self) -> Result<(), CascadeError> {
        if self.id.trim().is_empty() {
            return Err(CascadeError::EmptyScenarioId);
        }
        if self.nodes.is_empty() {
            return Err(CascadeError::EmptyNetwork);
        }
        if self.own_availability_by_tick.is_empty() {
            return Err(CascadeError::EmptyTrajectory);
        }
        if self.recovery_start_tick > self.own_availability_by_tick.len() {
            return Err(CascadeError::InvalidRecoveryStart {
                recovery_start_tick: self.recovery_start_tick,
                tick_count: self.own_availability_by_tick.len(),
            });
        }
        if !self.recovery_threshold.is_finite() || !(0.0..=1.0).contains(&self.recovery_threshold) {
            return Err(CascadeError::InvalidRecoveryThreshold(
                self.recovery_threshold,
            ));
        }
        if self.sustained_recovery_steps == 0 {
            return Err(CascadeError::ZeroSustainedRecoverySteps);
        }

        let mut ids = BTreeMap::new();
        let mut total_demand_weight = 0.0_f64;
        for (index, node) in self.nodes.iter().enumerate() {
            if node.id.trim().is_empty() {
                return Err(CascadeError::EmptyNodeId { index });
            }
            if node.service_type.trim().is_empty() {
                return Err(CascadeError::EmptyServiceType {
                    node_id: node.id.clone(),
                });
            }
            if ids.insert(node.id.as_str(), index).is_some() {
                return Err(CascadeError::DuplicateNodeId(node.id.clone()));
            }
            if !node.beneficiary_demand_weight.is_finite() || node.beneficiary_demand_weight < 0.0 {
                return Err(CascadeError::InvalidDemandWeight {
                    node_id: node.id.clone(),
                    value: node.beneficiary_demand_weight,
                });
            }
            total_demand_weight += node.beneficiary_demand_weight;
            if !total_demand_weight.is_finite() {
                return Err(CascadeError::DemandWeightOverflow);
            }

            for group in &node.dependency_groups {
                if group.inputs.is_empty() {
                    return Err(CascadeError::EmptyDependencyGroup {
                        node_id: node.id.clone(),
                    });
                }
                let mut parent_ids = BTreeSet::new();
                for input in &group.inputs {
                    if input.parent_id.trim().is_empty() {
                        return Err(CascadeError::EmptyParentId {
                            node_id: node.id.clone(),
                        });
                    }
                    if !parent_ids.insert(input.parent_id.as_str()) {
                        return Err(CascadeError::DuplicateDependency {
                            node_id: node.id.clone(),
                            parent_id: input.parent_id.clone(),
                        });
                    }
                }
            }
        }
        if total_demand_weight <= 0.0 {
            return Err(CascadeError::NoBeneficiaryDemand);
        }

        for node in &self.nodes {
            for group in &node.dependency_groups {
                for input in &group.inputs {
                    if !ids.contains_key(input.parent_id.as_str()) {
                        return Err(CascadeError::UnknownParent {
                            node_id: node.id.clone(),
                            parent_id: input.parent_id.clone(),
                        });
                    }
                    if !input.full_service_fraction.is_finite()
                        || !(0.0..=1.0).contains(&input.full_service_fraction)
                        || input.full_service_fraction <= 0.0
                    {
                        return Err(CascadeError::InvalidFullServiceFraction {
                            node_id: node.id.clone(),
                            parent_id: input.parent_id.clone(),
                            value: input.full_service_fraction,
                        });
                    }
                    if !input.minimum_operating_fraction.is_finite()
                        || input.minimum_operating_fraction < 0.0
                        || input.minimum_operating_fraction > input.full_service_fraction
                    {
                        return Err(CascadeError::InvalidMinimumOperatingFraction {
                            node_id: node.id.clone(),
                            parent_id: input.parent_id.clone(),
                            value: input.minimum_operating_fraction,
                            full_service_fraction: input.full_service_fraction,
                        });
                    }
                }
            }
        }

        for (tick, values) in self.own_availability_by_tick.iter().enumerate() {
            if values.len() != self.nodes.len() {
                return Err(CascadeError::AvailabilityWidthMismatch {
                    tick,
                    expected: self.nodes.len(),
                    actual: values.len(),
                });
            }
            for (node_index, value) in values.iter().enumerate() {
                if !value.is_finite() || !(0.0..=1.0).contains(value) {
                    return Err(CascadeError::InvalidAvailability {
                        tick,
                        node_id: self.nodes[node_index].id.clone(),
                        value: *value,
                    });
                }
            }
        }

        self.topological_order()?;
        Ok(())
    }

    /// Stable Kahn topological ordering. Cycles fail closed rather than relying
    /// on update order or an undocumented fixed-point approximation.
    pub fn topological_order(&self) -> Result<Vec<usize>, CascadeError> {
        let index_by_id = self
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id.as_str(), index))
            .collect::<BTreeMap<_, _>>();
        let mut outgoing = vec![Vec::new(); self.nodes.len()];
        let mut indegree = vec![0usize; self.nodes.len()];

        for (child_index, node) in self.nodes.iter().enumerate() {
            let mut parents = BTreeSet::new();
            for group in &node.dependency_groups {
                for input in &group.inputs {
                    let Some(parent_index) = index_by_id.get(input.parent_id.as_str()).copied()
                    else {
                        return Err(CascadeError::UnknownParent {
                            node_id: node.id.clone(),
                            parent_id: input.parent_id.clone(),
                        });
                    };
                    parents.insert(parent_index);
                }
            }
            for parent_index in parents {
                outgoing[parent_index].push(child_index);
                indegree[child_index] += 1;
            }
        }

        let mut ready = BTreeSet::new();
        for (index, count) in indegree.iter().enumerate() {
            if *count == 0 {
                ready.insert(index);
            }
        }

        let mut order = Vec::with_capacity(self.nodes.len());
        while let Some(index) = ready.pop_first() {
            order.push(index);
            for child in &outgoing[index] {
                indegree[*child] -= 1;
                if indegree[*child] == 0 {
                    ready.insert(*child);
                }
            }
        }

        if order.len() != self.nodes.len() {
            return Err(CascadeError::DependencyCycle);
        }
        Ok(order)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TickOutcome {
    pub tick_index: usize,
    /// Service functionality in [0, 1], in original node order.
    pub service_levels: Vec<f64>,
    /// Demand-weighted unmet service for this tick; no physical units are mixed.
    pub weighted_service_deficit: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeSummary {
    pub node_id: String,
    pub service_type: String,
    pub beneficiary_demand_weight: f64,
    pub minimum_service: f64,
    /// Sum of (1 - service level) over ticks; node-specific service-tick units.
    pub deficit_area_service_ticks: f64,
    pub ticks_below_50_percent: usize,
    pub ticks_below_90_percent: usize,
    /// Sustained service recovery measured from recovery_start_tick.
    pub recovery_after_ticks: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioOutcome {
    pub scenario_id: String,
    pub ticks: Vec<TickOutcome>,
    pub nodes: Vec<NodeSummary>,
    /// Sum of demand_weight * (1 - service) across endpoint services and ticks.
    pub demand_weighted_service_deficit_area: f64,
    pub total_beneficiary_demand_weight: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeCriticality {
    pub node_id: String,
    pub baseline_service_deficit_area: f64,
    pub outage_service_deficit_area: f64,
    /// Added demand-weighted service deficit from forcing this component off.
    pub marginal_service_deficit_area: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CascadeError {
    EmptyScenarioId,
    EmptyNetwork,
    EmptyTrajectory,
    InvalidRecoveryStart {
        recovery_start_tick: usize,
        tick_count: usize,
    },
    InvalidRecoveryThreshold(f64),
    ZeroSustainedRecoverySteps,
    EmptyNodeId {
        index: usize,
    },
    EmptyServiceType {
        node_id: String,
    },
    DuplicateNodeId(String),
    InvalidDemandWeight {
        node_id: String,
        value: f64,
    },
    DemandWeightOverflow,
    NoBeneficiaryDemand,
    EmptyDependencyGroup {
        node_id: String,
    },
    EmptyParentId {
        node_id: String,
    },
    DuplicateDependency {
        node_id: String,
        parent_id: String,
    },
    UnknownParent {
        node_id: String,
        parent_id: String,
    },
    InvalidFullServiceFraction {
        node_id: String,
        parent_id: String,
        value: f64,
    },
    InvalidMinimumOperatingFraction {
        node_id: String,
        parent_id: String,
        value: f64,
        full_service_fraction: f64,
    },
    AvailabilityWidthMismatch {
        tick: usize,
        expected: usize,
        actual: usize,
    },
    InvalidAvailability {
        tick: usize,
        node_id: String,
        value: f64,
    },
    DependencyCycle,
    InvalidOutageWindow {
        start: usize,
        end: usize,
        tick_count: usize,
    },
    NonMonotoneOutageCounterfactual {
        node_id: String,
        marginal_deficit: f64,
    },
}

impl fmt::Display for CascadeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyScenarioId => write!(f, "scenario ID must not be empty"),
            Self::EmptyNetwork => write!(f, "service dependency network must not be empty"),
            Self::EmptyTrajectory => write!(f, "availability trajectory must not be empty"),
            Self::InvalidRecoveryStart {
                recovery_start_tick,
                tick_count,
            } => write!(
                f,
                "recovery start tick {recovery_start_tick} exceeds trajectory length {tick_count}"
            ),
            Self::InvalidRecoveryThreshold(value) => {
                write!(
                    f,
                    "recovery threshold must be finite and in [0, 1], got {value}"
                )
            }
            Self::ZeroSustainedRecoverySteps => {
                write!(f, "sustained recovery steps must be greater than zero")
            }
            Self::EmptyNodeId { index } => write!(f, "node at index {index} has an empty ID"),
            Self::EmptyServiceType { node_id } => {
                write!(f, "node {node_id} has an empty service type")
            }
            Self::DuplicateNodeId(id) => write!(f, "duplicate service node ID: {id}"),
            Self::InvalidDemandWeight { node_id, value } => {
                write!(f, "node {node_id} has invalid demand weight {value}")
            }
            Self::DemandWeightOverflow => write!(f, "total beneficiary demand weight overflowed"),
            Self::NoBeneficiaryDemand => {
                write!(
                    f,
                    "at least one endpoint must have positive beneficiary demand"
                )
            }
            Self::EmptyDependencyGroup { node_id } => {
                write!(f, "node {node_id} has an empty dependency group")
            }
            Self::EmptyParentId { node_id } => {
                write!(f, "node {node_id} has an empty parent ID")
            }
            Self::DuplicateDependency { node_id, parent_id } => {
                write!(f, "node {node_id} repeats dependency parent {parent_id}")
            }
            Self::UnknownParent { node_id, parent_id } => {
                write!(f, "node {node_id} references unknown parent {parent_id}")
            }
            Self::InvalidFullServiceFraction {
                node_id,
                parent_id,
                value,
            } => write!(
                f,
                "node {node_id} has invalid full-service fraction {value} for parent {parent_id}"
            ),
            Self::InvalidMinimumOperatingFraction {
                node_id,
                parent_id,
                value,
                full_service_fraction,
            } => write!(
                f,
                "node {node_id} has minimum operating fraction {value} outside [0, {full_service_fraction}] for parent {parent_id}"
            ),
            Self::AvailabilityWidthMismatch {
                tick,
                expected,
                actual,
            } => write!(
                f,
                "tick {tick} has {actual} availability values; expected {expected}"
            ),
            Self::InvalidAvailability {
                tick,
                node_id,
                value,
            } => write!(
                f,
                "node {node_id} has invalid own availability {value} at tick {tick}"
            ),
            Self::DependencyCycle => {
                write!(f, "service dependency graph contains a cycle")
            }
            Self::InvalidOutageWindow {
                start,
                end,
                tick_count,
            } => write!(
                f,
                "invalid outage window [{start}, {end}) for {tick_count} ticks"
            ),
            Self::NonMonotoneOutageCounterfactual {
                node_id,
                marginal_deficit,
            } => write!(
                f,
                "outage counterfactual for {node_id} reduced service deficit by {marginal_deficit}"
            ),
        }
    }
}

impl std::error::Error for CascadeError {}

pub fn run_scenario(scenario: &Scenario) -> Result<ScenarioOutcome, CascadeError> {
    scenario.validate()?;
    let order = scenario.topological_order()?;
    let node_count = scenario.nodes.len();
    let tick_count = scenario.own_availability_by_tick.len();
    let total_beneficiary_demand_weight = scenario
        .nodes
        .iter()
        .map(|node| node.beneficiary_demand_weight)
        .sum::<f64>();

    let node_index_by_id = scenario
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    let mut ticks = Vec::with_capacity(tick_count);
    for (tick_index, own_availability) in scenario.own_availability_by_tick.iter().enumerate() {
        let mut service_levels = vec![0.0; node_count];

        for node_index in &order {
            let node = &scenario.nodes[*node_index];
            let mut dependency_capacity = 1.0_f64;
            for group in &node.dependency_groups {
                let mut group_values = Vec::with_capacity(group.inputs.len());
                for input in &group.inputs {
                    let parent_index = node_index_by_id
                        .get(input.parent_id.as_str())
                        .copied()
                        .expect("validated dependency parent must exist");
                    let parent_service = service_levels[parent_index];
                    let factor = dependency_factor(parent_service, input);
                    group_values.push(factor);
                }
                let group_capacity = match group.mode {
                    DependencyMode::AllOf => group_values.iter().copied().fold(1.0_f64, f64::min),
                    DependencyMode::AnyOf => group_values.iter().copied().fold(0.0_f64, f64::max),
                };
                dependency_capacity = dependency_capacity.min(group_capacity);
            }

            service_levels[*node_index] =
                (own_availability[*node_index] * dependency_capacity).clamp(0.0, 1.0);
        }

        let weighted_service_deficit = scenario
            .nodes
            .iter()
            .zip(&service_levels)
            .map(|(node, service)| node.beneficiary_demand_weight * (1.0 - service))
            .sum::<f64>();

        ticks.push(TickOutcome {
            tick_index,
            service_levels,
            weighted_service_deficit,
        });
    }

    let mut nodes = Vec::with_capacity(node_count);
    for (node_index, node) in scenario.nodes.iter().enumerate() {
        let levels = ticks
            .iter()
            .map(|tick| tick.service_levels[node_index])
            .collect::<Vec<_>>();
        let minimum_service = levels.iter().copied().fold(1.0_f64, f64::min);
        let deficit_area_service_ticks = levels.iter().map(|service| 1.0 - service).sum::<f64>();
        let ticks_below_50_percent = levels
            .iter()
            .filter(|service| **service < 0.5 - TOLERANCE)
            .count();
        let ticks_below_90_percent = levels
            .iter()
            .filter(|service| **service < 0.9 - TOLERANCE)
            .count();
        let mut recovery_streak = 0usize;
        let mut recovery_after_ticks = None;
        for (tick_index, service) in levels.iter().enumerate().skip(scenario.recovery_start_tick) {
            if *service >= scenario.recovery_threshold {
                recovery_streak += 1;
            } else {
                recovery_streak = 0;
            }
            if recovery_after_ticks.is_none()
                && recovery_streak >= scenario.sustained_recovery_steps
            {
                recovery_after_ticks = Some(tick_index + 1 - scenario.recovery_start_tick);
            }
        }

        nodes.push(NodeSummary {
            node_id: node.id.clone(),
            service_type: node.service_type.clone(),
            beneficiary_demand_weight: node.beneficiary_demand_weight,
            minimum_service,
            deficit_area_service_ticks,
            ticks_below_50_percent,
            ticks_below_90_percent,
            recovery_after_ticks,
        });
    }

    let demand_weighted_service_deficit_area = ticks
        .iter()
        .map(|tick| tick.weighted_service_deficit)
        .sum::<f64>();

    Ok(ScenarioOutcome {
        scenario_id: scenario.id.clone(),
        ticks,
        nodes,
        demand_weighted_service_deficit_area,
        total_beneficiary_demand_weight,
    })
}

pub fn node_criticalities(
    scenario: &Scenario,
    outage_start_tick: usize,
    outage_end_tick: usize,
) -> Result<Vec<NodeCriticality>, CascadeError> {
    scenario.validate()?;
    if outage_start_tick >= outage_end_tick
        || outage_end_tick > scenario.own_availability_by_tick.len()
    {
        return Err(CascadeError::InvalidOutageWindow {
            start: outage_start_tick,
            end: outage_end_tick,
            tick_count: scenario.own_availability_by_tick.len(),
        });
    }
    let baseline = run_scenario(scenario)?;
    let mut results = Vec::with_capacity(scenario.nodes.len());

    for node_index in 0..scenario.nodes.len() {
        let mut counterfactual = scenario.clone();
        for tick in outage_start_tick..outage_end_tick {
            counterfactual.own_availability_by_tick[tick][node_index] = 0.0;
        }
        let outage = run_scenario(&counterfactual)?;
        let marginal = outage.demand_weighted_service_deficit_area
            - baseline.demand_weighted_service_deficit_area;
        if marginal < -TOLERANCE {
            return Err(CascadeError::NonMonotoneOutageCounterfactual {
                node_id: scenario.nodes[node_index].id.clone(),
                marginal_deficit: marginal,
            });
        }
        results.push(NodeCriticality {
            node_id: scenario.nodes[node_index].id.clone(),
            baseline_service_deficit_area: baseline.demand_weighted_service_deficit_area,
            outage_service_deficit_area: outage.demand_weighted_service_deficit_area,
            marginal_service_deficit_area: marginal.max(0.0),
        });
    }

    Ok(results)
}

fn dependency_factor(parent_service: f64, input: &DependencyInput) -> f64 {
    if parent_service + TOLERANCE < input.minimum_operating_fraction {
        0.0
    } else {
        (parent_service / input.full_service_fraction).clamp(0.0, 1.0)
    }
}

pub fn reference_scenario() -> Scenario {
    let nodes = vec![
        node("grid", "power", 0.0, vec![]),
        node("fuel", "fuel", 0.0, vec![]),
        node(
            "water_treatment",
            "water_treatment",
            0.0,
            vec![all_of(vec![dep("grid", 0.5, 0.1)])],
        ),
        node(
            "water_pump",
            "water_pumping",
            0.0,
            vec![
                all_of(vec![dep("water_treatment", 0.7, 0.2)]),
                all_of(vec![dep("grid", 0.25, 0.1)]),
            ],
        ),
        node(
            "telecom",
            "communications",
            0.0,
            vec![all_of(vec![dep("grid", 0.4, 0.1)])],
        ),
        node(
            "road_access",
            "transport",
            0.0,
            vec![any_of(vec![dep("fuel", 0.5, 0.1), dep("grid", 0.2, 0.05)])],
        ),
        node(
            "hospital",
            "healthcare",
            2.0,
            vec![all_of(vec![
                dep("grid", 0.8, 0.2),
                dep("water_pump", 0.6, 0.2),
                dep("telecom", 0.5, 0.1),
                dep("road_access", 0.2, 0.1),
            ])],
        ),
        node(
            "payments",
            "payments",
            1.0,
            vec![all_of(vec![
                dep("grid", 0.5, 0.1),
                dep("telecom", 0.75, 0.2),
            ])],
        ),
        node(
            "household_water",
            "water_service",
            5.0,
            vec![all_of(vec![dep("water_pump", 0.8, 0.2)])],
        ),
    ];

    // The grid is unavailable for two ticks. Fuel remains available, providing
    // an explicit alternate input for road access.
    let mut availability = vec![vec![1.0; nodes.len()]; 6];
    availability[1][0] = 0.0;
    availability[2][0] = 0.0;

    Scenario {
        id: "power_outage_with_fuel_redundancy".into(),
        nodes,
        own_availability_by_tick: availability,
        recovery_start_tick: 3,
        recovery_threshold: 0.95,
        sustained_recovery_steps: 2,
    }
}

fn node(
    id: &str,
    service_type: &str,
    beneficiary_demand_weight: f64,
    dependency_groups: Vec<DependencyGroup>,
) -> ServiceNode {
    ServiceNode {
        id: id.into(),
        service_type: service_type.into(),
        beneficiary_demand_weight,
        dependency_groups,
    }
}

fn dep(
    parent_id: &str,
    full_service_fraction: f64,
    minimum_operating_fraction: f64,
) -> DependencyInput {
    DependencyInput {
        parent_id: parent_id.into(),
        full_service_fraction,
        minimum_operating_fraction,
    }
}

fn all_of(inputs: Vec<DependencyInput>) -> DependencyGroup {
    DependencyGroup {
        mode: DependencyMode::AllOf,
        inputs,
    }
}

fn any_of(inputs: Vec<DependencyInput>) -> DependencyGroup {
    DependencyGroup {
        mode: DependencyMode::AnyOf,
        inputs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_network_provides_full_service() {
        let mut scenario = reference_scenario();
        scenario.own_availability_by_tick = vec![vec![1.0; scenario.nodes.len()]; 4];
        scenario.recovery_start_tick = 0;
        let outcome = run_scenario(&scenario).unwrap();
        assert_eq!(outcome.demand_weighted_service_deficit_area, 0.0);
        assert!(outcome.nodes.iter().all(|node| node.minimum_service == 1.0));
    }

    #[test]
    fn power_outage_cascades_to_dependent_services_but_fuel_redundancy_preserves_roads() {
        let scenario = reference_scenario();
        let outcome = run_scenario(&scenario).unwrap();
        let node_index = |id: &str| {
            scenario
                .nodes
                .iter()
                .position(|node| node.id == id)
                .unwrap()
        };
        for tick in [1, 2] {
            assert_eq!(outcome.ticks[tick].service_levels[node_index("grid")], 0.0);
            assert_eq!(
                outcome.ticks[tick].service_levels[node_index("hospital")],
                0.0
            );
            assert_eq!(
                outcome.ticks[tick].service_levels[node_index("payments")],
                0.0
            );
            assert_eq!(
                outcome.ticks[tick].service_levels[node_index("household_water")],
                0.0
            );
            assert_eq!(
                outcome.ticks[tick].service_levels[node_index("road_access")],
                1.0
            );
        }
        assert_eq!(outcome.demand_weighted_service_deficit_area, 16.0);
    }

    #[test]
    fn partial_parent_degradation_produces_partial_downstream_service() {
        let mut scenario = reference_scenario();
        scenario.own_availability_by_tick = vec![vec![1.0; scenario.nodes.len()]; 3];
        scenario.own_availability_by_tick[1][0] = 0.25;
        scenario.recovery_start_tick = 2;
        let outcome = run_scenario(&scenario).unwrap();
        let treatment = scenario
            .nodes
            .iter()
            .position(|node| node.id == "water_treatment")
            .unwrap();
        assert!((outcome.ticks[1].service_levels[treatment] - 0.5).abs() < 1e-12);
        assert!(outcome.ticks[1].service_levels[treatment] > 0.0);
        assert!(outcome.ticks[1].service_levels[treatment] < 1.0);
    }

    #[test]
    fn any_of_dependency_keeps_road_service_when_grid_is_lost_but_fuel_survives() {
        let scenario = reference_scenario();
        let outcome = run_scenario(&scenario).unwrap();
        let road = scenario
            .nodes
            .iter()
            .position(|node| node.id == "road_access")
            .unwrap();
        assert_eq!(outcome.ticks[1].service_levels[road], 1.0);
    }

    #[test]
    fn sustained_recovery_requires_two_consecutive_ticks() {
        let scenario = reference_scenario();
        let outcome = run_scenario(&scenario).unwrap();
        let hospital = outcome
            .nodes
            .iter()
            .find(|node| node.node_id == "hospital")
            .unwrap();
        assert_eq!(hospital.recovery_after_ticks, Some(2));
    }

    #[test]
    fn counterfactual_criticality_distinguishes_grid_from_redundant_fuel() {
        let mut scenario = reference_scenario();
        scenario.own_availability_by_tick = vec![vec![1.0; scenario.nodes.len()]; 4];
        scenario.recovery_start_tick = 0;
        let criticalities = node_criticalities(&scenario, 1, 2).unwrap();
        let grid = criticalities
            .iter()
            .find(|node| node.node_id == "grid")
            .unwrap();
        let fuel = criticalities
            .iter()
            .find(|node| node.node_id == "fuel")
            .unwrap();
        assert!(grid.marginal_service_deficit_area > 0.0);
        assert_eq!(fuel.marginal_service_deficit_area, 0.0);
    }

    #[test]
    fn dependency_cycles_fail_closed() {
        let mut scenario = reference_scenario();
        scenario.nodes[0].dependency_groups = vec![all_of(vec![dep("hospital", 0.5, 0.1)])];
        assert!(matches!(
            scenario.validate(),
            Err(CascadeError::DependencyCycle)
        ));
    }

    #[test]
    fn input_availability_and_dependencies_are_validated() {
        let mut scenario = reference_scenario();
        scenario.own_availability_by_tick[0][0] = f64::NAN;
        assert!(matches!(
            scenario.validate(),
            Err(CascadeError::InvalidAvailability { .. })
        ));

        let mut invalid = reference_scenario();
        invalid.nodes[2].dependency_groups[0].inputs[0].parent_id = "missing".into();
        assert!(matches!(
            invalid.validate(),
            Err(CascadeError::UnknownParent { .. })
        ));
    }

    #[test]
    fn node_declaration_order_does_not_change_results() {
        let scenario = reference_scenario();
        let baseline = run_scenario(&scenario).unwrap();
        let mut reversed = scenario.clone();
        reversed.nodes.reverse();
        for tick in &mut reversed.own_availability_by_tick {
            tick.reverse();
        }
        let result = run_scenario(&reversed).unwrap();
        assert!(
            (baseline.demand_weighted_service_deficit_area
                - result.demand_weighted_service_deficit_area)
                .abs()
                < 1e-12
        );
    }
}
