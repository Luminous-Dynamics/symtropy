// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Deterministic stock-flow accounting baseline for socio-ecological models.
//!
//! The ledger is deliberately simpler than a civilization simulator. It
//! establishes dimensional separation and conservation before adding agents,
//! institutions, ecosystems, cognition or policy optimization.

use core::fmt;

const CONSERVATION_ABSOLUTE_TOLERANCE: f64 = 1e-9;
const CONSERVATION_RELATIVE_TOLERANCE: f64 = 1e-10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resource {
    Water,
    Energy,
    Food,
}

/// Unit-specific resource quantities. No implicit conversions are provided.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ResourceVector {
    pub water_m3: f64,
    pub energy_kwh: f64,
    pub food_kg: f64,
}

impl ResourceVector {
    pub const fn new(water_m3: f64, energy_kwh: f64, food_kg: f64) -> Self {
        Self { water_m3, energy_kwh, food_kg }
    }

    pub fn get(self, resource: Resource) -> f64 {
        match resource {
            Resource::Water => self.water_m3,
            Resource::Energy => self.energy_kwh,
            Resource::Food => self.food_kg,
        }
    }

    pub fn set(&mut self, resource: Resource, value: f64) {
        match resource {
            Resource::Water => self.water_m3 = value,
            Resource::Energy => self.energy_kwh = value,
            Resource::Food => self.food_kg = value,
        }
    }

    pub fn validate_non_negative(self, field: &'static str) -> Result<(), WorldError> {
        for (name, value) in [
            ("water_m3", self.water_m3),
            ("energy_kwh", self.energy_kwh),
            ("food_kg", self.food_kg),
        ] {
            if !value.is_finite() {
                return Err(WorldError::NonFiniteValue { field, quantity: name });
            }
            if value < 0.0 {
                return Err(WorldError::NegativeValue { field, quantity: name, value });
            }
        }
        Ok(())
    }

    fn add(self, other: Self) -> Self {
        Self::new(
            self.water_m3 + other.water_m3,
            self.energy_kwh + other.energy_kwh,
            self.food_kg + other.food_kg,
        )
    }

    fn subtract(self, other: Self) -> Self {
        Self::new(
            self.water_m3 - other.water_m3,
            self.energy_kwh - other.energy_kwh,
            self.food_kg - other.food_kg,
        )
    }

    fn minimum(self, other: Self) -> Self {
        Self::new(
            self.water_m3.min(other.water_m3),
            self.energy_kwh.min(other.energy_kwh),
            self.food_kg.min(other.food_kg),
        )
    }

    fn scale(self, factor: f64) -> Self {
        Self::new(
            self.water_m3 * factor,
            self.energy_kwh * factor,
            self.food_kg * factor,
        )
    }

    fn residual(self, other: Self) -> Self {
        self.subtract(other)
    }

    fn max_abs(self) -> f64 {
        self.water_m3.abs().max(self.energy_kwh.abs()).max(self.food_kg.abs())
    }
}

/// A region's initial inventory and per-capita essential demand.
#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    pub id: String,
    pub population: f64,
    pub opening_stock: ResourceVector,
    /// Essential demand per person per simulation tick.
    pub essential_demand_per_capita: ResourceVector,
}

impl Region {
    pub fn validate(&self) -> Result<(), WorldError> {
        if self.id.trim().is_empty() {
            return Err(WorldError::EmptyRegionId);
        }
        if !self.population.is_finite() || self.population < 0.0 {
            return Err(WorldError::InvalidPopulation { region: self.id.clone(), population: self.population });
        }
        self.opening_stock.validate_non_negative("opening_stock")?;
        self.essential_demand_per_capita.validate_non_negative("essential_demand_per_capita")?;
        Ok(())
    }
}

/// Exogenous production/renewable regeneration for one simulation tick.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Production {
    pub resources: ResourceVector,
}

/// Requested inter-region transfer. Actual delivery may be lower due to stock
/// availability or a policy-protected reserve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransferRequest {
    pub from_region: usize,
    pub to_region: usize,
    pub amount: ResourceVector,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StepInput {
    /// Exactly one production vector per region, in world region order.
    pub production: Vec<Production>,
    /// Applied in listed order; ordering is part of the scenario manifest.
    pub transfers: Vec<TransferRequest>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransferPolicy {
    /// Ship up to the requested amount, bounded only by available stock.
    Unrestricted,
    /// Preserve this fraction of each sender's opening stock of each resource.
    /// New production may be exported in the same tick.
    PreserveOpeningReserve { fraction: f64 },
}

impl TransferPolicy {
    fn reserve_fraction(self) -> Result<f64, WorldError> {
        match self {
            Self::Unrestricted => Ok(0.0),
            Self::PreserveOpeningReserve { fraction }
                if fraction.is_finite() && (0.0..=1.0).contains(&fraction) =>
            {
                Ok(fraction)
            }
            Self::PreserveOpeningReserve { fraction } => {
                Err(WorldError::InvalidReserveFraction(fraction))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResourceCoverage {
    pub water: Option<f64>,
    pub energy: Option<f64>,
    pub food: Option<f64>,
}

impl ResourceCoverage {
    fn from_served(served: ResourceVector, demand: ResourceVector) -> Self {
        fn ratio(served: f64, demand: f64) -> Option<f64> {
            if demand == 0.0 { None } else { Some((served / demand).clamp(0.0, 1.0)) }
        }
        Self {
            water: ratio(served.water_m3, demand.water_m3),
            energy: ratio(served.energy_kwh, demand.energy_kwh),
            food: ratio(served.food_kg, demand.food_kg),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegionLedger {
    pub beginning: ResourceVector,
    pub production: ResourceVector,
    pub imports: ResourceVector,
    pub exports: ResourceVector,
    pub served_essential_demand: ResourceVector,
    pub ending: ResourceVector,
}

impl RegionLedger {
    /// beginning + production + imports - exports - consumption - ending.
    pub fn conservation_residual(self) -> ResourceVector {
        self.beginning
            .add(self.production)
            .add(self.imports)
            .subtract(self.exports)
            .subtract(self.served_essential_demand)
            .subtract(self.ending)
    }

    pub fn is_conserved(self) -> bool {
        let residual = self.conservation_residual().max_abs();
        let scale = self.beginning.max_abs()
            .max(self.production.max_abs())
            .max(self.imports.max_abs())
            .max(self.exports.max_abs())
            .max(self.served_essential_demand.max_abs())
            .max(self.ending.max_abs())
            .max(1.0);
        residual <= CONSERVATION_ABSOLUTE_TOLERANCE
            + CONSERVATION_RELATIVE_TOLERANCE * scale
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegionOutcome {
    pub region_id: String,
    pub essential_demand: ResourceVector,
    pub served: ResourceVector,
    pub unmet: ResourceVector,
    pub coverage: ResourceCoverage,
    pub closing_stock: ResourceVector,
    pub ledger: RegionLedger,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransferReceipt {
    pub request_index: usize,
    pub from_region: usize,
    pub to_region: usize,
    pub requested: ResourceVector,
    pub delivered: ResourceVector,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StepReport {
    pub tick: u64,
    pub regions: Vec<RegionOutcome>,
    pub transfers: Vec<TransferReceipt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct World {
    regions: Vec<Region>,
    tick: u64,
}

impl World {
    pub fn new(regions: Vec<Region>) -> Result<Self, WorldError> {
        if regions.is_empty() {
            return Err(WorldError::EmptyWorld);
        }
        for region in &regions {
            region.validate()?;
        }
        Ok(Self { regions, tick: 0 })
    }

    pub fn regions(&self) -> &[Region] {
        &self.regions
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Apply a single deterministic step. If validation fails, the world is
    /// unchanged: inputs and policies are validated before state is committed.
    pub fn step(
        &mut self,
        input: &StepInput,
        policy: TransferPolicy,
    ) -> Result<StepReport, WorldError> {
        let reserve_fraction = policy.reserve_fraction()?;
        if input.production.len() != self.regions.len() {
            return Err(WorldError::ProductionCountMismatch {
                expected: self.regions.len(),
                actual: input.production.len(),
            });
        }

        for production in &input.production {
            production.resources.validate_non_negative("production")?;
        }
        for (request_index, request) in input.transfers.iter().enumerate() {
            if request.from_region >= self.regions.len() {
                return Err(WorldError::RegionIndexOutOfRange {
                    request_index,
                    index: request.from_region,
                    region_count: self.regions.len(),
                });
            }
            if request.to_region >= self.regions.len() {
                return Err(WorldError::RegionIndexOutOfRange {
                    request_index,
                    index: request.to_region,
                    region_count: self.regions.len(),
                });
            }
            if request.from_region == request.to_region {
                return Err(WorldError::SelfTransfer { request_index });
            }
            request.amount.validate_non_negative("transfer_request")?;
        }

        let beginning: Vec<ResourceVector> =
            self.regions.iter().map(|region| region.opening_stock).collect();
        let mut working: Vec<ResourceVector> = beginning
            .iter()
            .zip(input.production.iter())
            .map(|(stock, production)| stock.add(production.resources))
            .collect();

        let mut imports = vec![ResourceVector::default(); self.regions.len()];
        let mut exports = vec![ResourceVector::default(); self.regions.len()];
        let mut receipts = Vec::with_capacity(input.transfers.len());

        for (request_index, request) in input.transfers.iter().enumerate() {
            let from = request.from_region;
            let to = request.to_region;
            let mut delivered = ResourceVector::default();

            for resource in [Resource::Water, Resource::Energy, Resource::Food] {
                let opening = beginning[from].get(resource);
                let reserve_floor = opening * reserve_fraction;
                let available = (working[from].get(resource) - reserve_floor).max(0.0);
                let amount = request.amount.get(resource).min(available);

                working[from].set(resource, working[from].get(resource) - amount);
                working[to].set(resource, working[to].get(resource) + amount);
                delivered.set(resource, amount);
            }

            exports[from] = exports[from].add(delivered);
            imports[to] = imports[to].add(delivered);
            receipts.push(TransferReceipt {
                request_index,
                from_region: from,
                to_region: to,
                requested: request.amount,
                delivered,
            });
        }

        let mut outcomes = Vec::with_capacity(self.regions.len());
        let mut updated_regions = self.regions.clone();

        for index in 0..self.regions.len() {
            let region = &self.regions[index];
            let demand = region.essential_demand_per_capita.scale(region.population);
            let served = working[index].minimum(demand);
            let unmet = demand.subtract(served);
            let ending = working[index].subtract(served);
            ending.validate_non_negative("ending_stock")?;

            let ledger = RegionLedger {
                beginning: beginning[index],
                production: input.production[index].resources,
                imports: imports[index],
                exports: exports[index],
                served_essential_demand: served,
                ending,
            };
            if !ledger.is_conserved() {
                return Err(WorldError::ConservationViolation {
                    region: region.id.clone(),
                    residual: ledger.conservation_residual(),
                });
            }

            updated_regions[index].opening_stock = ending;
            outcomes.push(RegionOutcome {
                region_id: region.id.clone(),
                essential_demand: demand,
                served,
                unmet,
                coverage: ResourceCoverage::from_served(served, demand),
                closing_stock: ending,
                ledger,
            });
        }

        self.regions = updated_regions;
        self.tick = self.tick.checked_add(1).ok_or(WorldError::TickOverflow)?;

        Ok(StepReport {
            tick: self.tick,
            regions: outcomes,
            transfers: receipts,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorldError {
    EmptyWorld,
    EmptyRegionId,
    InvalidPopulation { region: String, population: f64 },
    NonFiniteValue { field: &'static str, quantity: &'static str },
    NegativeValue { field: &'static str, quantity: &'static str, value: f64 },
    InvalidReserveFraction(f64),
    ProductionCountMismatch { expected: usize, actual: usize },
    RegionIndexOutOfRange { request_index: usize, index: usize, region_count: usize },
    SelfTransfer { request_index: usize },
    ConservationViolation { region: String, residual: ResourceVector },
    TickOverflow,
}

impl fmt::Display for WorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyWorld => write!(f, "world must contain at least one region"),
            Self::EmptyRegionId => write!(f, "region id must not be empty"),
            Self::InvalidPopulation { region, population } => {
                write!(f, "region {region} has invalid population {population}")
            }
            Self::NonFiniteValue { field, quantity } => {
                write!(f, "{field}.{quantity} must be finite")
            }
            Self::NegativeValue { field, quantity, value } => {
                write!(f, "{field}.{quantity} is negative: {value}")
            }
            Self::InvalidReserveFraction(value) => {
                write!(f, "reserve fraction must be finite and in [0, 1], got {value}")
            }
            Self::ProductionCountMismatch { expected, actual } => {
                write!(f, "expected production for {expected} regions, got {actual}")
            }
            Self::RegionIndexOutOfRange { request_index, index, region_count } => {
                write!(f, "transfer {request_index} references region {index}, but world has {region_count} regions")
            }
            Self::SelfTransfer { request_index } => {
                write!(f, "transfer {request_index} has identical source and destination")
            }
            Self::ConservationViolation { region, residual } => {
                write!(f, "resource conservation failed for {region}: {residual:?}")
            }
            Self::TickOverflow => write!(f, "world tick counter overflow"),
        }
    }
}

impl std::error::Error for WorldError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(id: &str, stock: ResourceVector, demand: ResourceVector) -> Region {
        Region {
            id: id.into(),
            population: 10.0,
            opening_stock: stock,
            essential_demand_per_capita: demand,
        }
    }

    fn two_region_world() -> World {
        World::new(vec![
            region(
                "north",
                ResourceVector::new(100.0, 500.0, 200.0),
                ResourceVector::new(1.0, 2.0, 1.0),
            ),
            region(
                "south",
                ResourceVector::new(0.0, 0.0, 0.0),
                ResourceVector::new(1.0, 2.0, 1.0),
            ),
        ]).unwrap()
    }

    #[test]
    fn ledger_conserves_production_transfer_and_consumption() {
        let mut world = two_region_world();
        let report = world.step(
            &StepInput {
                production: vec![
                    Production { resources: ResourceVector::new(20.0, 50.0, 10.0) },
                    Production::default(),
                ],
                transfers: vec![TransferRequest {
                    from_region: 0,
                    to_region: 1,
                    amount: ResourceVector::new(30.0, 60.0, 30.0),
                }],
            },
            TransferPolicy::Unrestricted,
        ).unwrap();

        assert!(report.regions.iter().all(|outcome| outcome.ledger.is_conserved()));
        assert_eq!(report.regions[0].ledger.exports.water_m3, 30.0);
        assert_eq!(report.regions[1].ledger.imports.energy_kwh, 60.0);
    }

    #[test]
    fn transfers_cannot_create_resources() {
        let mut world = two_region_world();
        let report = world.step(
            &StepInput {
                production: vec![Production::default(); 2],
                transfers: vec![TransferRequest {
                    from_region: 1,
                    to_region: 0,
                    amount: ResourceVector::new(1_000.0, 1_000.0, 1_000.0),
                }],
            },
            TransferPolicy::Unrestricted,
        ).unwrap();

        assert_eq!(report.transfers[0].delivered, ResourceVector::default());
        assert!(report.regions.iter().all(|outcome| outcome.ledger.is_conserved()));
    }

    #[test]
    fn reserve_policy_preserves_fraction_of_opening_stock() {
        let mut world = two_region_world();
        let report = world.step(
            &StepInput {
                production: vec![Production::default(); 2],
                transfers: vec![TransferRequest {
                    from_region: 0,
                    to_region: 1,
                    amount: ResourceVector::new(100.0, 500.0, 200.0),
                }],
            },
            TransferPolicy::PreserveOpeningReserve { fraction: 0.25 },
        ).unwrap();

        assert_eq!(report.transfers[0].delivered, ResourceVector::new(75.0, 375.0, 150.0));
    }

    #[test]
    fn essential_service_coverage_is_disaggregated() {
        let mut world = two_region_world();
        let report = world.step(
            &StepInput {
                production: vec![Production::default(); 2],
                transfers: vec![],
            },
            TransferPolicy::Unrestricted,
        ).unwrap();

        assert_eq!(report.regions[0].coverage.water, Some(1.0));
        assert_eq!(report.regions[1].coverage.water, Some(0.0));
        assert_eq!(report.regions[1].unmet.food_kg, 10.0);
    }

    #[test]
    fn zero_demand_is_undefined_not_perfect_coverage() {
        let mut world = World::new(vec![region(
            "no-demand",
            ResourceVector::default(),
            ResourceVector::default(),
        )]).unwrap();

        let report = world.step(
            &StepInput { production: vec![Production::default()], transfers: vec![] },
            TransferPolicy::Unrestricted,
        ).unwrap();

        assert_eq!(report.regions[0].coverage.water, None);
        assert_eq!(report.regions[0].coverage.energy, None);
        assert_eq!(report.regions[0].coverage.food, None);
    }

    #[test]
    fn production_shock_reduces_service_without_negative_stock() {
        let mut world = World::new(vec![region(
            "basin",
            ResourceVector::default(),
            ResourceVector::new(1.0, 1.0, 1.0),
        )]).unwrap();

        let report = world.step(
            &StepInput {
                production: vec![Production {
                    resources: ResourceVector::new(2.0, 15.0, 10.0),
                }],
                transfers: vec![],
            },
            TransferPolicy::Unrestricted,
        ).unwrap();

        assert_eq!(report.regions[0].coverage.water, Some(0.2));
        assert_eq!(report.regions[0].coverage.energy, Some(1.0));
        assert_eq!(report.regions[0].closing_stock.water_m3, 0.0);
        assert_eq!(report.regions[0].closing_stock.energy_kwh, 5.0);
    }

    #[test]
    fn deterministic_inputs_produce_identical_worlds_and_reports() {
        let input = StepInput {
            production: vec![
                Production { resources: ResourceVector::new(3.0, 4.0, 5.0) },
                Production::default(),
            ],
            transfers: vec![TransferRequest {
                from_region: 0,
                to_region: 1,
                amount: ResourceVector::new(2.0, 3.0, 4.0),
            }],
        };
        let mut a = two_region_world();
        let mut b = two_region_world();

        assert_eq!(
            a.step(&input, TransferPolicy::PreserveOpeningReserve { fraction: 0.1 }).unwrap(),
            b.step(&input, TransferPolicy::PreserveOpeningReserve { fraction: 0.1 }).unwrap()
        );
        assert_eq!(a, b);
    }

    #[test]
    fn invalid_reserve_fraction_is_rejected_without_mutating_world() {
        let mut world = two_region_world();
        let before = world.clone();
        assert_eq!(
            world.step(
                &StepInput { production: vec![Production::default(); 2], transfers: vec![] },
                TransferPolicy::PreserveOpeningReserve { fraction: 1.1 }
            ),
            Err(WorldError::InvalidReserveFraction(1.1))
        );
        assert_eq!(world, before);
    }

    #[test]
    fn invalid_transfer_endpoints_are_rejected_without_mutation() {
        let mut world = two_region_world();
        let before = world.clone();
        assert_eq!(
            world.step(
                &StepInput {
                    production: vec![Production::default(); 2],
                    transfers: vec![TransferRequest {
                        from_region: 0,
                        to_region: 2,
                        amount: ResourceVector::new(1.0, 0.0, 0.0),
                    }],
                },
                TransferPolicy::Unrestricted
            ),
            Err(WorldError::RegionIndexOutOfRange {
                request_index: 0,
                index: 2,
                region_count: 2,
            })
        );
        assert_eq!(world, before);
    }

    #[test]
    fn transfer_to_self_is_rejected() {
        let mut world = two_region_world();
        assert_eq!(
            world.step(
                &StepInput {
                    production: vec![Production::default(); 2],
                    transfers: vec![TransferRequest {
                        from_region: 1,
                        to_region: 1,
                        amount: ResourceVector::default(),
                    }],
                },
                TransferPolicy::Unrestricted
            ),
            Err(WorldError::SelfTransfer { request_index: 0 })
        );
    }

    #[test]
    fn empty_world_and_negative_stocks_are_rejected() {
        assert_eq!(World::new(vec![]), Err(WorldError::EmptyWorld));
        assert_eq!(
            World::new(vec![region("bad", ResourceVector::new(-1.0, 0.0, 0.0), ResourceVector::default())]),
            Err(WorldError::NegativeValue {
                field: "opening_stock",
                quantity: "water_m3",
                value: -1.0,
            })
        );
    }

    #[test]
    fn production_count_must_match_region_count() {
        let mut world = two_region_world();
        assert_eq!(
            world.step(
                &StepInput { production: vec![Production::default()], transfers: vec![] },
                TransferPolicy::Unrestricted
            ),
            Err(WorldError::ProductionCountMismatch { expected: 2, actual: 1 })
        );
    }

    #[test]
    fn world_tick_advances_only_after_successful_step() {
        let mut world = two_region_world();
        assert_eq!(world.tick(), 0);
        let _ = world.step(
            &StepInput { production: vec![Production::default(); 2], transfers: vec![] },
            TransferPolicy::Unrestricted
        ).unwrap();
        assert_eq!(world.tick(), 1);
    }
}
