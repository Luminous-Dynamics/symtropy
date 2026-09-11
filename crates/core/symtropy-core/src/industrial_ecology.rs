// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Deterministic regenerative industrial-ecology simulation.
//!
//! This module advances qualified inventory, ordinary production and recycling
//! period by period and records when dependencies or capabilities become
//! unavailable. It is a simulation primitive, not a manufacturing controller:
//! there are no process recipes, mining operations, nuclear-fuel-cycle operations
//! or physical actuator paths here.

use std::collections::{BTreeMap, BTreeSet};

const MAX_MODEL_ITEMS: usize = 4096;
const MAX_ID_LEN: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndustrialGovernance {
    Ordinary,
    /// Supplied only by separately safeguarded external infrastructure.
    SafeguardedExternal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialDependencyState {
    pub dependency_id: String,
    pub governance: IndustrialGovernance,
    pub demand_units_per_tick: u64,
    pub local_production_units_per_tick: u64,
    pub recycling_units_per_tick: u64,
    /// Qualified inventory available before the next tick.
    pub inventory_units: u64,
}

impl IndustrialDependencyState {
    pub fn validate(&self) -> Result<(), IndustrialEcologyError> {
        validate_id(&self.dependency_id)?;
        if self.demand_units_per_tick == 0 {
            return Err(IndustrialEcologyError::ZeroDemand {
                dependency_id: self.dependency_id.clone(),
            });
        }
        if self.governance == IndustrialGovernance::SafeguardedExternal
            && (self.local_production_units_per_tick != 0
                || self.recycling_units_per_tick != 0)
        {
            return Err(IndustrialEcologyError::SafeguardedDependencyClaimsLocalSupply {
                dependency_id: self.dependency_id.clone(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialCapability {
    pub capability_id: String,
    pub essential: bool,
    pub dependency_ids: BTreeSet<String>,
}

impl IndustrialCapability {
    fn validate(&self) -> Result<(), IndustrialEcologyError> {
        validate_id(&self.capability_id)?;
        if self.dependency_ids.is_empty() {
            return Err(IndustrialEcologyError::CapabilityHasNoDependencies {
                capability_id: self.capability_id.clone(),
            });
        }
        for dependency_id in &self.dependency_ids {
            validate_id(dependency_id)?;
        }
        Ok(())
    }
}

/// Which simulated local flow depends on prerequisite availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IndustrialFlowKind {
    Production,
    Recycling,
}

/// Simulation-only prerequisite relationship for one positive local flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialFlowPrerequisite {
    pub dependency_id: String,
    pub flow_kind: IndustrialFlowKind,
    pub prerequisite_dependency_ids: BTreeSet<String>,
}

impl IndustrialFlowPrerequisite {
    fn validate(&self) -> Result<(), IndustrialEcologyError> {
        validate_id(&self.dependency_id)?;
        if self.prerequisite_dependency_ids.is_empty() {
            return Err(IndustrialEcologyError::FlowPrerequisiteHasNoDependencies {
                dependency_id: self.dependency_id.clone(),
                flow_kind: self.flow_kind,
            });
        }
        for prerequisite in &self.prerequisite_dependency_ids {
            validate_id(prerequisite)?;
            if prerequisite == &self.dependency_id {
                return Err(IndustrialEcologyError::FlowPrerequisiteSelfDependency {
                    dependency_id: self.dependency_id.clone(),
                    flow_kind: self.flow_kind,
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockedIndustrialFlow {
    pub dependency_id: String,
    pub flow_kind: IndustrialFlowKind,
    pub blocking_prerequisite_ids: Vec<String>,
}

/// Deterministic mutable industrial ecology.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialEcology {
    tick: u64,
    dependencies: BTreeMap<String, IndustrialDependencyState>,
    capabilities: Vec<IndustrialCapability>,
    flow_prerequisites: Option<BTreeMap<(String, IndustrialFlowKind), IndustrialFlowPrerequisite>>,
    last_shortage_ids: BTreeSet<String>,
}

impl IndustrialEcology {
    pub fn new(
        dependencies: impl IntoIterator<Item = IndustrialDependencyState>,
        capabilities: Vec<IndustrialCapability>,
    ) -> Result<Self, IndustrialEcologyError> {
        Self::build(dependencies, capabilities, None)
    }

    /// Construct an ecology where every initially positive local production/recycling
    /// flow is gated by explicit prerequisite dependencies.
    ///
    /// A prerequisite shortage observed in tick N suppresses the dependent flow in
    /// tick N+1. Later capacity loss may set a modeled flow to zero without deleting
    /// its prerequisite relationship; enabling a previously unmodeled positive flow
    /// still fails closed. Qualification/bootstrap evidence remains owned by
    /// higher-level design/evidence systems.
    pub fn new_with_flow_prerequisites(
        dependencies: impl IntoIterator<Item = IndustrialDependencyState>,
        capabilities: Vec<IndustrialCapability>,
        flow_prerequisites: Vec<IndustrialFlowPrerequisite>,
    ) -> Result<Self, IndustrialEcologyError> {
        let dependencies: Vec<_> = dependencies.into_iter().collect();
        let mut map = BTreeMap::new();
        for flow in flow_prerequisites {
            flow.validate()?;
            let key = (flow.dependency_id.clone(), flow.flow_kind);
            if map.insert(key.clone(), flow).is_some() {
                return Err(IndustrialEcologyError::DuplicateFlowPrerequisite {
                    dependency_id: key.0,
                    flow_kind: key.1,
                });
            }
        }
        let expected = positive_flow_keys(&dependencies);
        let actual: BTreeSet<_> = map.keys().cloned().collect();
        if expected != actual {
            return Err(IndustrialEcologyError::FlowPrerequisiteCoverageMismatch);
        }
        Self::build(dependencies, capabilities, Some(map))
    }

    fn build(
        dependencies: impl IntoIterator<Item = IndustrialDependencyState>,
        capabilities: Vec<IndustrialCapability>,
        flow_prerequisites: Option<
            BTreeMap<(String, IndustrialFlowKind), IndustrialFlowPrerequisite>,
        >,
    ) -> Result<Self, IndustrialEcologyError> {
        let mut dependency_map = BTreeMap::new();
        for dependency in dependencies {
            dependency.validate()?;
            let id = dependency.dependency_id.clone();
            if dependency_map.insert(id.clone(), dependency).is_some() {
                return Err(IndustrialEcologyError::DuplicateDependency {
                    dependency_id: id,
                });
            }
        }
        let ecology = Self {
            tick: 0,
            dependencies: dependency_map,
            capabilities,
            flow_prerequisites,
            last_shortage_ids: BTreeSet::new(),
        };
        ecology.validate()?;
        Ok(ecology)
    }

    pub const fn tick(&self) -> u64 {
        self.tick
    }

    pub fn dependency(&self, dependency_id: &str) -> Option<&IndustrialDependencyState> {
        self.dependencies.get(dependency_id)
    }

    pub fn validate(&self) -> Result<(), IndustrialEcologyError> {
        if self.dependencies.is_empty() {
            return Err(IndustrialEcologyError::NoDependencies);
        }
        if self.capabilities.is_empty() {
            return Err(IndustrialEcologyError::NoCapabilities);
        }
        if self.dependencies.len() > MAX_MODEL_ITEMS || self.capabilities.len() > MAX_MODEL_ITEMS {
            return Err(IndustrialEcologyError::ModelTooLarge);
        }

        for (key, dependency) in &self.dependencies {
            validate_id(key)?;
            dependency.validate()?;
            if key != &dependency.dependency_id {
                return Err(IndustrialEcologyError::DependencyKeyMismatch {
                    key: key.clone(),
                    dependency_id: dependency.dependency_id.clone(),
                });
            }
        }

        let mut capability_ids = BTreeSet::new();
        let mut has_essential = false;
        for capability in &self.capabilities {
            capability.validate()?;
            has_essential |= capability.essential;
            if !capability_ids.insert(capability.capability_id.clone()) {
                return Err(IndustrialEcologyError::DuplicateCapability {
                    capability_id: capability.capability_id.clone(),
                });
            }
            for dependency_id in &capability.dependency_ids {
                if !self.dependencies.contains_key(dependency_id) {
                    return Err(IndustrialEcologyError::UnknownDependencyReference {
                        capability_id: capability.capability_id.clone(),
                        dependency_id: dependency_id.clone(),
                    });
                }
            }
        }
        if !has_essential {
            return Err(IndustrialEcologyError::NoEssentialCapabilities);
        }

        for shortage_id in &self.last_shortage_ids {
            if !self.dependencies.contains_key(shortage_id) {
                return Err(IndustrialEcologyError::UnknownDependency {
                    dependency_id: shortage_id.clone(),
                });
            }
        }

        if let Some(flow_prerequisites) = &self.flow_prerequisites {
            if flow_prerequisites.len() > MAX_MODEL_ITEMS {
                return Err(IndustrialEcologyError::ModelTooLarge);
            }
            let current_positive = positive_flow_keys(self.dependencies.values());
            let modeled: BTreeSet<_> = flow_prerequisites.keys().cloned().collect();
            if !current_positive.is_subset(&modeled) {
                return Err(IndustrialEcologyError::FlowPrerequisiteCoverageMismatch);
            }
            for ((dependency_id, _flow_kind), flow) in flow_prerequisites {
                flow.validate()?;
                self.dependencies.get(dependency_id).ok_or_else(|| {
                    IndustrialEcologyError::UnknownDependency {
                        dependency_id: dependency_id.clone(),
                    }
                })?;
                for prerequisite in &flow.prerequisite_dependency_ids {
                    if !self.dependencies.contains_key(prerequisite) {
                        return Err(IndustrialEcologyError::UnknownFlowPrerequisite {
                            dependency_id: dependency_id.clone(),
                            prerequisite_dependency_id: prerequisite.clone(),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// Apply a deterministic availability change before the next tick.
    ///
    /// Candidate state is validated before commit, so a rejected shock is atomic.
    pub fn apply_shock(&mut self, shock: IndustrialShock) -> Result<(), IndustrialEcologyError> {
        let dependency_id = shock.dependency_id().to_string();
        let mut candidate = self
            .dependencies
            .get(&dependency_id)
            .cloned()
            .ok_or_else(|| IndustrialEcologyError::UnknownDependency {
                dependency_id: dependency_id.clone(),
            })?;

        match shock {
            IndustrialShock::SetLocalProduction { units_per_tick, .. } => {
                candidate.local_production_units_per_tick = units_per_tick;
            }
            IndustrialShock::SetRecycling { units_per_tick, .. } => {
                candidate.recycling_units_per_tick = units_per_tick;
            }
            IndustrialShock::LoseInventory { units, .. } => {
                candidate.inventory_units = candidate.inventory_units.saturating_sub(units);
            }
        }
        candidate.validate()?;

        if self.flow_prerequisites.is_some() {
            let old = self.dependencies.insert(dependency_id.clone(), candidate);
            let validation = self.validate();
            if let Err(error) = validation {
                if let Some(previous) = old {
                    self.dependencies.insert(dependency_id, previous);
                }
                return Err(error);
            }
            return Ok(());
        }
        self.dependencies.insert(dependency_id, candidate);
        Ok(())
    }

    /// Advance the ecology by one tick atomically.
    ///
    /// All dependency transitions are staged in a candidate map. Arithmetic or
    /// validation failure leaves inventory, shortage memory, and tick unchanged.
    pub fn step(&mut self) -> Result<IndustrialTickReport, IndustrialEcologyError> {
        self.validate()?;
        let completed_tick = self
            .tick
            .checked_add(1)
            .ok_or(IndustrialEcologyError::ArithmeticOverflow)?;
        let mut candidate_dependencies = self.dependencies.clone();
        let prior_shortages = self.last_shortage_ids.clone();
        let mut shortages = Vec::new();
        let mut blocked_flows = Vec::new();

        for dependency in candidate_dependencies.values_mut() {
            let production = self.effective_flow(
                &dependency.dependency_id,
                IndustrialFlowKind::Production,
                dependency.local_production_units_per_tick,
                &prior_shortages,
                &mut blocked_flows,
            );
            let recycling = self.effective_flow(
                &dependency.dependency_id,
                IndustrialFlowKind::Recycling,
                dependency.recycling_units_per_tick,
                &prior_shortages,
                &mut blocked_flows,
            );
            let available = u128::from(dependency.inventory_units)
                + u128::from(production)
                + u128::from(recycling);
            let demand = u128::from(dependency.demand_units_per_tick);
            if available >= demand {
                dependency.inventory_units = u64::try_from(available - demand)
                    .map_err(|_| IndustrialEcologyError::ArithmeticOverflow)?;
            } else {
                dependency.inventory_units = 0;
                shortages.push(DependencyShortage {
                    dependency_id: dependency.dependency_id.clone(),
                    missing_units: u64::try_from(demand - available)
                        .map_err(|_| IndustrialEcologyError::ArithmeticOverflow)?,
                });
            }
        }

        let short_ids: BTreeSet<String> = shortages
            .iter()
            .map(|shortage| shortage.dependency_id.clone())
            .collect();
        let unavailable_capability_ids: Vec<String> = self
            .capabilities
            .iter()
            .filter(|capability| {
                capability
                    .dependency_ids
                    .iter()
                    .any(|dependency_id| short_ids.contains(dependency_id))
            })
            .map(|capability| capability.capability_id.clone())
            .collect();
        let unavailable: BTreeSet<&str> = unavailable_capability_ids
            .iter()
            .map(String::as_str)
            .collect();
        let essential_capabilities_available = self
            .capabilities
            .iter()
            .filter(|capability| capability.essential)
            .all(|capability| !unavailable.contains(capability.capability_id.as_str()));

        self.dependencies = candidate_dependencies;
        self.last_shortage_ids = short_ids;
        self.tick = completed_tick;
        Ok(IndustrialTickReport {
            tick: completed_tick,
            shortages,
            blocked_flows,
            unavailable_capability_ids,
            essential_capabilities_available,
        })
    }

    fn effective_flow(
        &self,
        dependency_id: &str,
        flow_kind: IndustrialFlowKind,
        configured_units: u64,
        prior_shortages: &BTreeSet<String>,
        blocked_flows: &mut Vec<BlockedIndustrialFlow>,
    ) -> u64 {
        if configured_units == 0 {
            return 0;
        }
        let Some(flow_prerequisites) = &self.flow_prerequisites else {
            return configured_units;
        };
        let key = (dependency_id.to_string(), flow_kind);
        let Some(flow) = flow_prerequisites.get(&key) else {
            return configured_units;
        };
        let blocking_prerequisite_ids: Vec<String> = flow
            .prerequisite_dependency_ids
            .iter()
            .filter(|prerequisite| prior_shortages.contains(*prerequisite))
            .cloned()
            .collect();
        if blocking_prerequisite_ids.is_empty() {
            configured_units
        } else {
            blocked_flows.push(BlockedIndustrialFlow {
                dependency_id: dependency_id.to_string(),
                flow_kind,
                blocking_prerequisite_ids,
            });
            0
        }
    }

    pub fn run_until_essential_failure(
        &mut self,
        max_ticks: u64,
    ) -> Result<IndustrialViabilityOutcome, IndustrialEcologyError> {
        let mut survived_ticks = 0;
        for _ in 0..max_ticks {
            let report = self.step()?;
            if !report.essential_capabilities_available {
                return Ok(IndustrialViabilityOutcome {
                    survived_ticks,
                    terminal_report: Some(report),
                });
            }
            survived_ticks += 1;
        }
        Ok(IndustrialViabilityOutcome {
            survived_ticks,
            terminal_report: None,
        })
    }
}

fn positive_flow_keys(
    dependencies: impl IntoIterator<Item = impl std::borrow::Borrow<IndustrialDependencyState>>,
) -> BTreeSet<(String, IndustrialFlowKind)> {
    let mut flows = BTreeSet::new();
    for dependency in dependencies {
        let dependency = dependency.borrow();
        if dependency.local_production_units_per_tick > 0 {
            flows.insert((dependency.dependency_id.clone(), IndustrialFlowKind::Production));
        }
        if dependency.recycling_units_per_tick > 0 {
            flows.insert((dependency.dependency_id.clone(), IndustrialFlowKind::Recycling));
        }
    }
    flows
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndustrialShock {
    SetLocalProduction {
        dependency_id: String,
        units_per_tick: u64,
    },
    SetRecycling {
        dependency_id: String,
        units_per_tick: u64,
    },
    /// Destroy or invalidate qualified stockpile inventory.
    LoseInventory {
        dependency_id: String,
        units: u64,
    },
}

impl IndustrialShock {
    fn dependency_id(&self) -> &str {
        match self {
            Self::SetLocalProduction { dependency_id, .. }
            | Self::SetRecycling { dependency_id, .. }
            | Self::LoseInventory { dependency_id, .. } => dependency_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyShortage {
    pub dependency_id: String,
    pub missing_units: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialTickReport {
    pub tick: u64,
    pub shortages: Vec<DependencyShortage>,
    pub blocked_flows: Vec<BlockedIndustrialFlow>,
    pub unavailable_capability_ids: Vec<String>,
    pub essential_capabilities_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialViabilityOutcome {
    /// Complete ticks survived before the first essential failure.
    pub survived_ticks: u64,
    pub terminal_report: Option<IndustrialTickReport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndustrialEcologyError {
    InvalidIdentifier,
    NoDependencies,
    NoCapabilities,
    NoEssentialCapabilities,
    ModelTooLarge,
    ZeroDemand { dependency_id: String },
    DuplicateDependency { dependency_id: String },
    DependencyKeyMismatch { key: String, dependency_id: String },
    DuplicateCapability { capability_id: String },
    CapabilityHasNoDependencies { capability_id: String },
    UnknownDependencyReference {
        capability_id: String,
        dependency_id: String,
    },
    UnknownDependency { dependency_id: String },
    SafeguardedDependencyClaimsLocalSupply { dependency_id: String },
    FlowPrerequisiteHasNoDependencies {
        dependency_id: String,
        flow_kind: IndustrialFlowKind,
    },
    FlowPrerequisiteSelfDependency {
        dependency_id: String,
        flow_kind: IndustrialFlowKind,
    },
    DuplicateFlowPrerequisite {
        dependency_id: String,
        flow_kind: IndustrialFlowKind,
    },
    UnknownFlowPrerequisite {
        dependency_id: String,
        prerequisite_dependency_id: String,
    },
    FlowPrerequisiteCoverageMismatch,
    ArithmeticOverflow,
}

fn validate_id(value: &str) -> Result<(), IndustrialEcologyError> {
    if value.is_empty()
        || value.len() > MAX_ID_LEN
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        Err(IndustrialEcologyError::InvalidIdentifier)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dep(
        id: &str,
        governance: IndustrialGovernance,
        demand: u64,
        production: u64,
        recycling: u64,
        inventory: u64,
    ) -> IndustrialDependencyState {
        IndustrialDependencyState {
            dependency_id: id.into(),
            governance,
            demand_units_per_tick: demand,
            local_production_units_per_tick: production,
            recycling_units_per_tick: recycling,
            inventory_units: inventory,
        }
    }

    fn ecology(dependencies: Vec<IndustrialDependencyState>) -> IndustrialEcology {
        let ids = dependencies
            .iter()
            .map(|dependency| dependency.dependency_id.clone())
            .collect();
        IndustrialEcology::new(
            dependencies,
            vec![IndustrialCapability {
                capability_id: "essential-operation".into(),
                essential: true,
                dependency_ids: ids,
            }],
        )
        .unwrap()
    }

    #[test]
    fn safeguarded_service_limits_an_otherwise_regenerative_ecology() {
        let mut sim = ecology(vec![
            dep("structural-material", IndustrialGovernance::Ordinary, 100, 80, 20, 0),
            dep("electronics", IndustrialGovernance::Ordinary, 10, 9, 0, 1_000),
            dep(
                "qualified-reactor-fuel-service",
                IndustrialGovernance::SafeguardedExternal,
                1,
                0,
                0,
                30,
            ),
        ]);
        let outcome = sim.run_until_essential_failure(100).unwrap();
        assert_eq!(outcome.survived_ticks, 30);
        let terminal = outcome.terminal_report.unwrap();
        assert_eq!(terminal.tick, 31);
        assert_eq!(
            terminal.shortages,
            vec![DependencyShortage {
                dependency_id: "qualified-reactor-fuel-service".into(),
                missing_units: 1,
            }]
        );
    }

    #[test]
    fn recycling_can_close_material_loop_for_a_long_bounded_run() {
        let mut sim = ecology(vec![dep(
            "steel",
            IndustrialGovernance::Ordinary,
            100,
            80,
            20,
            0,
        )]);
        let outcome = sim.run_until_essential_failure(10_000).unwrap();
        assert_eq!(outcome.survived_ticks, 10_000);
        assert!(outcome.terminal_report.is_none());
    }

    #[test]
    fn production_loss_shortens_inventory_buffer() {
        let initial = dep("electronics", IndustrialGovernance::Ordinary, 10, 9, 0, 100);
        let mut baseline = ecology(vec![initial.clone()]);
        assert_eq!(
            baseline.run_until_essential_failure(200).unwrap().survived_ticks,
            100
        );

        let mut shocked = ecology(vec![initial]);
        shocked
            .apply_shock(IndustrialShock::SetLocalProduction {
                dependency_id: "electronics".into(),
                units_per_tick: 0,
            })
            .unwrap();
        assert_eq!(
            shocked.run_until_essential_failure(200).unwrap().survived_ticks,
            10
        );
    }

    #[test]
    fn rejected_safeguarded_shock_does_not_poison_state() {
        let mut sim = ecology(vec![dep(
            "reactor-service",
            IndustrialGovernance::SafeguardedExternal,
            1,
            0,
            0,
            10,
        )]);
        let before = sim.dependency("reactor-service").unwrap().clone();
        assert_eq!(
            sim.apply_shock(IndustrialShock::SetLocalProduction {
                dependency_id: "reactor-service".into(),
                units_per_tick: 1,
            }),
            Err(IndustrialEcologyError::SafeguardedDependencyClaimsLocalSupply {
                dependency_id: "reactor-service".into(),
            })
        );
        assert_eq!(sim.dependency("reactor-service"), Some(&before));
        assert!(sim.validate().is_ok());
    }

    #[test]
    fn failed_tick_is_transactional() {
        let mut sim = ecology(vec![
            dep("a-first", IndustrialGovernance::Ordinary, 1, 0, 0, 5),
            dep(
                "z-overflow",
                IndustrialGovernance::Ordinary,
                1,
                u64::MAX,
                0,
                u64::MAX,
            ),
        ]);
        let before = sim.clone();
        assert_eq!(sim.step(), Err(IndustrialEcologyError::ArithmeticOverflow));
        assert_eq!(sim, before);
    }

    #[test]
    fn nonessential_shortage_does_not_end_essential_viability() {
        let essential = dep("steel", IndustrialGovernance::Ordinary, 10, 10, 0, 0);
        let optional = dep("luxury-sensor", IndustrialGovernance::Ordinary, 1, 0, 0, 0);
        let mut sim = IndustrialEcology::new(
            [essential, optional],
            vec![
                IndustrialCapability {
                    capability_id: "core".into(),
                    essential: true,
                    dependency_ids: BTreeSet::from(["steel".into()]),
                },
                IndustrialCapability {
                    capability_id: "optional-science".into(),
                    essential: false,
                    dependency_ids: BTreeSet::from(["luxury-sensor".into()]),
                },
            ],
        )
        .unwrap();
        let report = sim.step().unwrap();
        assert!(report.essential_capabilities_available);
        assert_eq!(report.unavailable_capability_ids, vec!["optional-science"]);
    }

    #[test]
    fn identical_initial_state_and_shocks_are_deterministic() {
        let initial = ecology(vec![dep(
            "electronics",
            IndustrialGovernance::Ordinary,
            10,
            8,
            0,
            50,
        )]);
        let mut a = initial.clone();
        let mut b = initial;
        let shock = IndustrialShock::SetRecycling {
            dependency_id: "electronics".into(),
            units_per_tick: 1,
        };
        a.apply_shock(shock.clone()).unwrap();
        b.apply_shock(shock).unwrap();
        assert_eq!(
            a.run_until_essential_failure(100).unwrap(),
            b.run_until_essential_failure(100).unwrap()
        );
    }

    #[test]
    fn prerequisite_shortage_blocks_downstream_flow_on_next_tick() {
        let dependencies = vec![
            dep("metrology", IndustrialGovernance::Ordinary, 1, 0, 0, 0),
            dep("widgets", IndustrialGovernance::Ordinary, 10, 10, 0, 0),
        ];
        let capabilities = vec![IndustrialCapability {
            capability_id: "widget-service".into(),
            essential: true,
            dependency_ids: BTreeSet::from(["widgets".into()]),
        }];
        let flow = IndustrialFlowPrerequisite {
            dependency_id: "widgets".into(),
            flow_kind: IndustrialFlowKind::Production,
            prerequisite_dependency_ids: BTreeSet::from(["metrology".into()]),
        };
        let mut sim = IndustrialEcology::new_with_flow_prerequisites(
            dependencies,
            capabilities,
            vec![flow],
        )
        .unwrap();

        let first = sim.step().unwrap();
        assert!(first.essential_capabilities_available);
        assert_eq!(
            first.shortages,
            vec![DependencyShortage {
                dependency_id: "metrology".into(),
                missing_units: 1,
            }]
        );
        assert!(first.blocked_flows.is_empty());

        let second = sim.step().unwrap();
        assert!(!second.essential_capabilities_available);
        assert_eq!(
            second.blocked_flows,
            vec![BlockedIndustrialFlow {
                dependency_id: "widgets".into(),
                flow_kind: IndustrialFlowKind::Production,
                blocking_prerequisite_ids: vec!["metrology".into()],
            }]
        );
        assert!(second.shortages.iter().any(|shortage| shortage.dependency_id == "widgets"));
    }

    #[test]
    fn flow_prerequisite_constructor_requires_exact_initial_positive_flow_coverage() {
        let dependency = dep("widgets", IndustrialGovernance::Ordinary, 10, 10, 0, 0);
        let result = IndustrialEcology::new_with_flow_prerequisites(
            [dependency],
            vec![IndustrialCapability {
                capability_id: "widgets".into(),
                essential: true,
                dependency_ids: BTreeSet::from(["widgets".into()]),
            }],
            Vec::new(),
        );
        assert_eq!(result, Err(IndustrialEcologyError::FlowPrerequisiteCoverageMismatch));
    }

    #[test]
    fn capacity_can_drop_to_zero_without_erasing_the_prerequisite_relationship() {
        let dependencies = vec![
            dep("metrology", IndustrialGovernance::Ordinary, 1, 0, 0, 5),
            dep("widgets", IndustrialGovernance::Ordinary, 10, 10, 0, 20),
        ];
        let capabilities = vec![IndustrialCapability {
            capability_id: "widget-service".into(),
            essential: true,
            dependency_ids: BTreeSet::from(["widgets".into()]),
        }];
        let flow = IndustrialFlowPrerequisite {
            dependency_id: "widgets".into(),
            flow_kind: IndustrialFlowKind::Production,
            prerequisite_dependency_ids: BTreeSet::from(["metrology".into()]),
        };
        let mut sim = IndustrialEcology::new_with_flow_prerequisites(
            dependencies,
            capabilities,
            vec![flow],
        )
        .unwrap();
        assert_eq!(
            sim.apply_shock(IndustrialShock::SetLocalProduction {
                dependency_id: "widgets".into(),
                units_per_tick: 0,
            }),
            Ok(())
        );
        assert!(sim.validate().is_ok());
        assert_eq!(
            sim.apply_shock(IndustrialShock::SetLocalProduction {
                dependency_id: "widgets".into(),
                units_per_tick: 10,
            }),
            Ok(())
        );
        assert!(sim.validate().is_ok());
    }

    #[test]
    fn previously_unmodeled_flow_cannot_be_enabled_under_prerequisite_mode() {
        let dependencies = vec![
            dep("metrology", IndustrialGovernance::Ordinary, 1, 0, 0, 5),
            dep("widgets", IndustrialGovernance::Ordinary, 10, 10, 0, 20),
        ];
        let capabilities = vec![IndustrialCapability {
            capability_id: "widget-service".into(),
            essential: true,
            dependency_ids: BTreeSet::from(["widgets".into()]),
        }];
        let flow = IndustrialFlowPrerequisite {
            dependency_id: "widgets".into(),
            flow_kind: IndustrialFlowKind::Production,
            prerequisite_dependency_ids: BTreeSet::from(["metrology".into()]),
        };
        let mut sim = IndustrialEcology::new_with_flow_prerequisites(
            dependencies,
            capabilities,
            vec![flow],
        )
        .unwrap();
        let before = sim.clone();
        assert_eq!(
            sim.apply_shock(IndustrialShock::SetRecycling {
                dependency_id: "widgets".into(),
                units_per_tick: 1,
            }),
            Err(IndustrialEcologyError::FlowPrerequisiteCoverageMismatch)
        );
        assert_eq!(sim, before);
    }
}
