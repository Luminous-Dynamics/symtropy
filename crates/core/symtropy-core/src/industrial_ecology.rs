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

/// Governance boundary for a simulated dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndustrialGovernance {
    /// Ordinary dependency whose recurring demand may be met locally.
    Ordinary,
    /// Dependency intentionally supplied only by separately safeguarded external
    /// infrastructure. Local production and recycling must remain zero.
    SafeguardedExternal,
}

/// Mutable dependency state advanced by the simulator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialDependencyState {
    /// Canonical dependency identifier.
    pub dependency_id: String,
    /// Governance boundary.
    pub governance: IndustrialGovernance,
    /// Units required during every simulation tick.
    pub demand_units_per_tick: u64,
    /// Ordinary locally produced units available each tick.
    pub local_production_units_per_tick: u64,
    /// Units recovered by recycling each tick.
    pub recycling_units_per_tick: u64,
    /// Qualified inventory available before the next tick.
    pub inventory_units: u64,
}

impl IndustrialDependencyState {
    /// Validate one dependency state.
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

/// Capability whose availability requires all named dependencies during a tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialCapability {
    /// Canonical capability identifier.
    pub capability_id: String,
    /// Whether losing this capability ends the modeled autonomous-viability run.
    pub essential: bool,
    /// Dependency identifiers required by the capability.
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

/// Deterministic mutable industrial ecology.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialEcology {
    tick: u64,
    dependencies: BTreeMap<String, IndustrialDependencyState>,
    capabilities: Vec<IndustrialCapability>,
}

impl IndustrialEcology {
    /// Construct a validated ecology at tick zero.
    pub fn new(
        dependencies: impl IntoIterator<Item = IndustrialDependencyState>,
        capabilities: Vec<IndustrialCapability>,
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
        };
        ecology.validate()?;
        Ok(ecology)
    }

    /// Current simulation tick.
    pub const fn tick(&self) -> u64 {
        self.tick
    }

    /// Read one dependency state by canonical identifier.
    pub fn dependency(&self, dependency_id: &str) -> Option<&IndustrialDependencyState> {
        self.dependencies.get(dependency_id)
    }

    /// Validate references, identities and governance boundaries.
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
        Ok(())
    }

    /// Apply a deterministic availability change before the next tick.
    ///
    /// The change is validated on a copy and committed only if valid, so a
    /// rejected shock cannot leave the ecology in a poisoned intermediate state.
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
        self.dependencies.insert(dependency_id, candidate);
        Ok(())
    }

    /// Advance production, recycling, demand and capability availability by one tick.
    pub fn step(&mut self) -> Result<IndustrialTickReport, IndustrialEcologyError> {
        self.validate()?;
        let completed_tick = self
            .tick
            .checked_add(1)
            .ok_or(IndustrialEcologyError::ArithmeticOverflow)?;
        let mut shortages = Vec::new();

        for dependency in self.dependencies.values_mut() {
            let available = u128::from(dependency.inventory_units)
                + u128::from(dependency.local_production_units_per_tick)
                + u128::from(dependency.recycling_units_per_tick);
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

        let short_ids: BTreeSet<&str> = shortages
            .iter()
            .map(|shortage| shortage.dependency_id.as_str())
            .collect();
        let unavailable_capability_ids: Vec<String> = self
            .capabilities
            .iter()
            .filter(|capability| {
                capability
                    .dependency_ids
                    .iter()
                    .any(|dependency_id| short_ids.contains(dependency_id.as_str()))
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

        self.tick = completed_tick;
        Ok(IndustrialTickReport {
            tick: completed_tick,
            shortages,
            unavailable_capability_ids,
            essential_capabilities_available,
        })
    }

    /// Run until an essential capability fails or `max_ticks` survive successfully.
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

/// Deterministic external change applied to one dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndustrialShock {
    /// Replace ordinary local production capacity.
    SetLocalProduction {
        /// Target dependency.
        dependency_id: String,
        /// New units produced per tick.
        units_per_tick: u64,
    },
    /// Replace recycling recovery capacity.
    SetRecycling {
        /// Target dependency.
        dependency_id: String,
        /// New units recovered per tick.
        units_per_tick: u64,
    },
    /// Destroy or invalidate qualified stockpile inventory.
    LoseInventory {
        /// Target dependency.
        dependency_id: String,
        /// Units removed, saturating at zero.
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

/// Shortfall discovered during one tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyShortage {
    /// Dependency that could not meet recurring demand.
    pub dependency_id: String,
    /// Units missing during this tick.
    pub missing_units: u64,
}

/// Result of one deterministic industrial-ecology tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialTickReport {
    /// Completed tick number, starting at one.
    pub tick: u64,
    /// Dependencies that could not meet this tick's demand.
    pub shortages: Vec<DependencyShortage>,
    /// Capabilities made unavailable by those shortages.
    pub unavailable_capability_ids: Vec<String>,
    /// Whether all essential capabilities remain available this tick.
    pub essential_capabilities_available: bool,
}

/// Result of a bounded regenerative-viability experiment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialViabilityOutcome {
    /// Complete ticks survived before the first essential failure.
    pub survived_ticks: u64,
    /// First essential-failure report, or `None` if `max_ticks` survived.
    pub terminal_report: Option<IndustrialTickReport>,
}

/// Validation and simulation errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndustrialEcologyError {
    /// Identifier is empty, padded, contains control characters or is too long.
    InvalidIdentifier,
    /// Model contains no dependencies.
    NoDependencies,
    /// Model contains no capabilities.
    NoCapabilities,
    /// Model contains no essential capability.
    NoEssentialCapabilities,
    /// Model exceeds bounded cardinality.
    ModelTooLarge,
    /// Dependency has zero recurring demand.
    ZeroDemand { dependency_id: String },
    /// Duplicate dependency identifier at construction.
    DuplicateDependency { dependency_id: String },
    /// Map key and embedded dependency identity disagree.
    DependencyKeyMismatch { key: String, dependency_id: String },
    /// Capability identifier is duplicated.
    DuplicateCapability { capability_id: String },
    /// Capability declares no dependencies.
    CapabilityHasNoDependencies { capability_id: String },
    /// Capability references an unknown dependency.
    UnknownDependencyReference {
        capability_id: String,
        dependency_id: String,
    },
    /// A requested shock names an unknown dependency.
    UnknownDependency { dependency_id: String },
    /// Safeguarded dependency claims ordinary local production or recycling.
    SafeguardedDependencyClaimsLocalSupply { dependency_id: String },
    /// Integer state could not be represented exactly.
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
}
