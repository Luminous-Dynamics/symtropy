// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Cohort-indexed incidence and inheritance accounting.
//!
//! The shared physical resource ledger remains the only stock-flow authority.
//! This crate attaches services, liabilities and inherited capital to cohorts;
//! it does not create another physical simulator or a composite welfare score.
//! Physical resources, financial liabilities and capital-condition measures
//! remain separate types with separate units and validation rules.

use civdyn_resource_ledger::{RegionOutcome, ResourceVector};
use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

const NUMERIC_TOLERANCE: f64 = 1e-9;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CohortId(String);

impl CohortId {
    pub fn new(value: impl Into<String>) -> Result<Self, CohortAccountingError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(CohortAccountingError::EmptyIdentifier("cohort_id"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cohort {
    pub id: CohortId,
    pub birth_start_year: i32,
    pub birth_end_year_exclusive: i32,
    /// Declared demographic assumption, not an observed population claim.
    pub population_assumption: f64,
}

impl Cohort {
    fn validate(&self) -> Result<(), CohortAccountingError> {
        if self.birth_start_year >= self.birth_end_year_exclusive {
            return Err(CohortAccountingError::InvalidCohortInterval {
                cohort_id: self.id.clone(),
            });
        }
        if !self.population_assumption.is_finite() || self.population_assumption <= 0.0 {
            return Err(CohortAccountingError::InvalidPopulation {
                cohort_id: self.id.clone(),
                population: self.population_assumption,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimulationHorizon {
    pub start_year: i32,
    pub end_year_exclusive: i32,
}

impl SimulationHorizon {
    fn validate(self) -> Result<(), CohortAccountingError> {
        if self.start_year >= self.end_year_exclusive {
            return Err(CohortAccountingError::InvalidHorizon);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CohortServiceKind {
    HouseholdService,
    AssetMaintenance,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CohortServiceRecord {
    pub service_kind: CohortServiceKind,
    pub cohort_id: CohortId,
    pub region_id: String,
    pub year: i32,
    /// Resource quantities served to this cohort in the interval.
    pub served: ResourceVector,
    /// Resource quantities of essential demand not served.
    pub unmet: ResourceVector,
    pub people_exposed: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalLiabilityKind {
    Maintenance,
    Replacement,
    ReserveReplenishment,
    EcologicalRestoration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiabilityAssignment {
    Assigned(CohortId),
    Unassigned { reason: String },
}

impl LiabilityAssignment {
    fn assigned_cohort(&self) -> Option<&CohortId> {
        match self {
            Self::Assigned(cohort_id) => Some(cohort_id),
            Self::Unassigned { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhysicalLiability {
    pub obligation_id: String,
    pub originating_action_id: String,
    pub created_year: i32,
    pub due_year: i32,
    pub kind: PhysicalLiabilityKind,
    pub assignment: LiabilityAssignment,
    /// Native resource units only: water m³, energy kWh, food kg.
    pub quantity: ResourceVector,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FinancialLiability {
    pub obligation_id: String,
    pub originating_action_id: String,
    pub created_year: i32,
    pub due_year: i32,
    pub assignment: LiabilityAssignment,
    pub currency_code: String,
    /// Price/base year for the real or nominal amount; interpretation is
    /// declared by the scenario rather than inferred by this crate.
    pub price_base_year: i32,
    pub amount: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapitalClass {
    Produced,
    Natural,
    Human,
    Institutional,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapitalMeasure {
    pub asset_id: String,
    pub metric_id: String,
    pub capital_class: CapitalClass,
    pub unit: String,
    /// E.g. a bounded condition index, area, capacity or source-defined metric.
    /// It is not added to resource or financial quantities.
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CohortInheritance {
    pub cohort_id: CohortId,
    /// Year in which the cohort enters the model.
    pub inherited_on_year: i32,
    /// Immediately preceding state boundary; this state is shared by cohorts
    /// and must not be summed across inheritance records.
    pub source_closing_year: i32,
    pub inherited_resource_stocks: ResourceVector,
    pub inherited_capital: Vec<CapitalMeasure>,
    /// References existing financial-liability records; does not duplicate
    /// their amounts in the financial ledger.
    pub inherited_financial_obligation_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoneyAmount {
    pub amount: f64,
    pub discount_factor: f64,
    pub discounted_amount: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscountedFinancialLiability {
    pub obligation_id: String,
    pub cohort_id: Option<CohortId>,
    pub currency_code: String,
    pub price_base_year: i32,
    pub due_year: i32,
    pub value: MoneyAmount,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CohortIncidenceLedger {
    pub scenario_id: String,
    pub horizon: SimulationHorizon,
    pub cohorts: Vec<Cohort>,
    pub services: Vec<CohortServiceRecord>,
    pub physical_liabilities: Vec<PhysicalLiability>,
    pub financial_liabilities: Vec<FinancialLiability>,
    pub inheritances: Vec<CohortInheritance>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CohortAccountingError {
    EmptyIdentifier(&'static str),
    InvalidCohortInterval { cohort_id: CohortId },
    InvalidPopulation { cohort_id: CohortId, population: f64 },
    InvalidHorizon,
    DuplicateCohort(CohortId),
    UnknownCohort(CohortId),
    EmptyRegionId,
    InvalidYear { year: i32 },
    InvalidQuantity { field: &'static str },
    InvalidPeopleExposed { value: f64 },
    EmptyObligationId,
    DuplicateObligation(String),
    InvalidDueDate { obligation_id: String },
    EmptyUnassignedReason { obligation_id: String },
    InvalidCurrency(String),
    InvalidMoneyAmount { obligation_id: String },
    InvalidDiscountRate(f64),
    PriceYearMismatch {
        obligation_id: String,
        expected: i32,
        actual: i32,
    },
    DuplicateInheritance(CohortId),
    InvalidInheritanceBoundary { cohort_id: CohortId },
    UnknownInheritedObligation {
        cohort_id: CohortId,
        obligation_id: String,
    },
    ServiceReceiptMismatch {
        region_id: String,
        year: i32,
        resource: &'static str,
        cohort_total: f64,
        ledger_total: f64,
    },
    MissingServiceRecords { region_id: String, year: i32 },
}

impl fmt::Display for CohortAccountingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentifier(field) => write!(f, "{field} must not be empty"),
            Self::InvalidCohortInterval { cohort_id } => {
                write!(f, "invalid birth interval for cohort {}", cohort_id.as_str())
            }
            Self::InvalidPopulation {
                cohort_id,
                population,
            } => write!(
                f,
                "invalid population assumption {population} for cohort {}",
                cohort_id.as_str()
            ),
            Self::InvalidHorizon => write!(f, "simulation horizon must have positive length"),
            Self::DuplicateCohort(id) => write!(f, "duplicate cohort {}", id.as_str()),
            Self::UnknownCohort(id) => write!(f, "unknown cohort {}", id.as_str()),
            Self::EmptyRegionId => write!(f, "region_id must not be empty"),
            Self::InvalidYear { year } => write!(f, "invalid year {year}"),
            Self::InvalidQuantity { field } => write!(f, "invalid non-negative quantity in {field}"),
            Self::InvalidPeopleExposed { value } => write!(f, "invalid people_exposed value {value}"),
            Self::EmptyObligationId => write!(f, "obligation_id must not be empty"),
            Self::DuplicateObligation(id) => write!(f, "duplicate obligation {id}"),
            Self::InvalidDueDate { obligation_id } => {
                write!(f, "obligation {obligation_id} has due_year before created_year")
            }
            Self::EmptyUnassignedReason { obligation_id } => {
                write!(f, "unassigned obligation {obligation_id} needs an explicit reason")
            }
            Self::InvalidCurrency(code) => write!(f, "currency code must be three uppercase ASCII letters: {code}"),
            Self::InvalidMoneyAmount { obligation_id } => {
                write!(f, "invalid money amount for obligation {obligation_id}")
            }
            Self::InvalidDiscountRate(rate) => write!(f, "invalid real discount rate {rate}"),
            Self::PriceYearMismatch {
                obligation_id,
                expected,
                actual,
            } => write!(
                f,
                "obligation {obligation_id} price year {actual} does not match valuation year {expected}"
            ),
            Self::DuplicateInheritance(id) => {
                write!(f, "duplicate inheritance snapshot for cohort {}", id.as_str())
            }
            Self::InvalidInheritanceBoundary { cohort_id } => write!(
                f,
                "invalid inheritance boundary for cohort {}",
                cohort_id.as_str()
            ),
            Self::UnknownInheritedObligation {
                cohort_id,
                obligation_id,
            } => write!(
                f,
                "cohort {} references unknown inherited obligation {obligation_id}",
                cohort_id.as_str()
            ),
            Self::ServiceReceiptMismatch {
                region_id,
                year,
                resource,
                cohort_total,
                ledger_total,
            } => write!(
                f,
                "cohort service does not reconcile for {region_id} in {year} ({resource}: {cohort_total} vs {ledger_total})"
            ),
            Self::MissingServiceRecords { region_id, year } => {
                write!(f, "no cohort service records for {region_id} in {year}")
            }
        }
    }
}

impl std::error::Error for CohortAccountingError {}

fn validate_resource_vector(
    value: ResourceVector,
    field: &'static str,
) -> Result<(), CohortAccountingError> {
    if !value.water_m3.is_finite()
        || value.water_m3 < 0.0
        || !value.energy_kwh.is_finite()
        || value.energy_kwh < 0.0
        || !value.food_kg.is_finite()
        || value.food_kg < 0.0
    {
        return Err(CohortAccountingError::InvalidQuantity { field });
    }
    Ok(())
}

fn add_resource_vectors(left: ResourceVector, right: ResourceVector) -> ResourceVector {
    ResourceVector::new(
        left.water_m3 + right.water_m3,
        left.energy_kwh + right.energy_kwh,
        left.food_kg + right.food_kg,
    )
}

fn approximately_equal(left: f64, right: f64) -> bool {
    (left - right).abs()
        <= NUMERIC_TOLERANCE + NUMERIC_TOLERANCE * left.abs().max(right.abs())
}

impl CohortIncidenceLedger {
    pub fn validate(&self) -> Result<(), CohortAccountingError> {
        if self.scenario_id.trim().is_empty() {
            return Err(CohortAccountingError::EmptyIdentifier("scenario_id"));
        }
        self.horizon.validate()?;

        let mut cohort_ids = BTreeSet::new();
        for cohort in &self.cohorts {
            cohort.validate()?;
            if !cohort_ids.insert(cohort.id.clone()) {
                return Err(CohortAccountingError::DuplicateCohort(cohort.id.clone()));
            }
        }
        let known_cohort = |id: &CohortId| cohort_ids.contains(id);

        for record in &self.services {
            if !known_cohort(&record.cohort_id) {
                return Err(CohortAccountingError::UnknownCohort(record.cohort_id.clone()));
            }
            if record.region_id.trim().is_empty() {
                return Err(CohortAccountingError::EmptyRegionId);
            }
            if record.year < self.horizon.start_year || record.year >= self.horizon.end_year_exclusive {
                return Err(CohortAccountingError::InvalidYear { year: record.year });
            }
            validate_resource_vector(record.served, "cohort_service.served")?;
            validate_resource_vector(record.unmet, "cohort_service.unmet")?;
            if !record.people_exposed.is_finite() || record.people_exposed < 0.0 {
                return Err(CohortAccountingError::InvalidPeopleExposed {
                    value: record.people_exposed,
                });
            }
        }

        let mut obligation_ids = BTreeSet::new();
        for obligation in &self.physical_liabilities {
            validate_obligation_header(
                &obligation.obligation_id,
                &obligation.originating_action_id,
                obligation.created_year,
                obligation.due_year,
                &obligation.assignment,
                &known_cohort,
            )?;
            if !obligation_ids.insert(obligation.obligation_id.clone()) {
                return Err(CohortAccountingError::DuplicateObligation(
                    obligation.obligation_id.clone(),
                ));
            }
            validate_resource_vector(obligation.quantity, "physical_liability.quantity")?;
        }

        for obligation in &self.financial_liabilities {
            validate_obligation_header(
                &obligation.obligation_id,
                &obligation.originating_action_id,
                obligation.created_year,
                obligation.due_year,
                &obligation.assignment,
                &known_cohort,
            )?;
            if !obligation_ids.insert(obligation.obligation_id.clone()) {
                return Err(CohortAccountingError::DuplicateObligation(
                    obligation.obligation_id.clone(),
                ));
            }
            if obligation.currency_code.len() != 3
                || !obligation
                    .currency_code
                    .bytes()
                    .all(|value| value.is_ascii_uppercase())
            {
                return Err(CohortAccountingError::InvalidCurrency(
                    obligation.currency_code.clone(),
                ));
            }
            if !obligation.amount.is_finite() || obligation.amount < 0.0 {
                return Err(CohortAccountingError::InvalidMoneyAmount {
                    obligation_id: obligation.obligation_id.clone(),
                });
            }
            if obligation.price_base_year < obligation.created_year {
                return Err(CohortAccountingError::InvalidYear {
                    year: obligation.price_base_year,
                });
            }
        }

        let mut inheritance_ids = BTreeSet::new();
        for inheritance in &self.inheritances {
            let cohort = self
                .cohorts
                .iter()
                .find(|cohort| cohort.id == inheritance.cohort_id)
                .ok_or_else(|| {
                    CohortAccountingError::UnknownCohort(inheritance.cohort_id.clone())
                })?;
            if !inheritance_ids.insert(inheritance.cohort_id.clone()) {
                return Err(CohortAccountingError::DuplicateInheritance(
                    inheritance.cohort_id.clone(),
                ));
            }
            if inheritance.inherited_on_year != cohort.birth_start_year
                || inheritance.source_closing_year + 1 != inheritance.inherited_on_year
            {
                return Err(CohortAccountingError::InvalidInheritanceBoundary {
                    cohort_id: inheritance.cohort_id.clone(),
                });
            }
            validate_resource_vector(
                inheritance.inherited_resource_stocks,
                "cohort_inheritance.resource_stocks",
            )?;
            for measure in &inheritance.inherited_capital {
                if measure.asset_id.trim().is_empty()
                    || measure.metric_id.trim().is_empty()
                    || measure.unit.trim().is_empty()
                    || !measure.value.is_finite()
                {
                    return Err(CohortAccountingError::InvalidQuantity {
                        field: "cohort_inheritance.capital_measure",
                    });
                }
            }
            for obligation_id in &inheritance.inherited_financial_obligation_ids {
                if !self
                    .financial_liabilities
                    .iter()
                    .any(|obligation| obligation.obligation_id == *obligation_id)
                {
                    return Err(CohortAccountingError::UnknownInheritedObligation {
                        cohort_id: inheritance.cohort_id.clone(),
                        obligation_id: obligation_id.clone(),
                    });
                }
            }
        }

        for cohort in &self.cohorts {
            if cohort.birth_start_year >= self.horizon.end_year_exclusive
                && !inheritance_ids.contains(&cohort.id)
            {
                return Err(CohortAccountingError::InvalidInheritanceBoundary {
                    cohort_id: cohort.id.clone(),
                });
            }
        }
        Ok(())
    }

    /// Reconcile all cohort service rows for a region/year against one
    /// authoritative resource-ledger region report. Cohort splitting cannot
    /// change the total served or unmet physical quantity.
    pub fn reconcile_region_service(
        &self,
        region: &RegionOutcome,
        year: i32,
    ) -> Result<(), CohortAccountingError> {
        let records: Vec<_> = self
            .services
            .iter()
            .filter(|record| record.region_id == region.region_id && record.year == year)
            .collect();
        if records.is_empty() {
            return Err(CohortAccountingError::MissingServiceRecords {
                region_id: region.region_id.clone(),
                year,
            });
        }
        let mut served = ResourceVector::default();
        let mut unmet = ResourceVector::default();
        for record in records {
            served = add_resource_vectors(served, record.served);
            unmet = add_resource_vectors(unmet, record.unmet);
        }
        compare_component("served.water_m3", served.water_m3, region.served.water_m3, &region.region_id, year)?;
        compare_component("served.energy_kwh", served.energy_kwh, region.served.energy_kwh, &region.region_id, year)?;
        compare_component("served.food_kg", served.food_kg, region.served.food_kg, &region.region_id, year)?;
        compare_component("unmet.water_m3", unmet.water_m3, region.unmet.water_m3, &region.region_id, year)?;
        compare_component("unmet.energy_kwh", unmet.energy_kwh, region.unmet.energy_kwh, &region.region_id, year)?;
        compare_component("unmet.food_kg", unmet.food_kg, region.unmet.food_kg, &region.region_id, year)?;
        Ok(())
    }

    pub fn total_service_by_region_year(
        &self,
    ) -> BTreeMap<(String, i32), (ResourceVector, ResourceVector)> {
        let mut totals = BTreeMap::new();
        for record in &self.services {
            let entry = totals
                .entry((record.region_id.clone(), record.year))
                .or_insert((ResourceVector::default(), ResourceVector::default()));
            entry.0 = add_resource_vectors(entry.0, record.served);
            entry.1 = add_resource_vectors(entry.1, record.unmet);
        }
        totals
    }

    /// Total outstanding physical liabilities in native resource units.
    pub fn total_physical_liabilities(&self) -> ResourceVector {
        self.physical_liabilities.iter().fold(
            ResourceVector::default(),
            |total, liability| add_resource_vectors(total, liability.quantity),
        )
    }

    /// Discount financial obligations only. Physical resource/capital records
    /// are never inputs to this transformation.
    pub fn discounted_financial_liabilities(
        &self,
        valuation_year: i32,
        real_discount_rate: f64,
    ) -> Result<Vec<DiscountedFinancialLiability>, CohortAccountingError> {
        if !real_discount_rate.is_finite() || real_discount_rate <= -1.0 {
            return Err(CohortAccountingError::InvalidDiscountRate(
                real_discount_rate,
            ));
        }
        let mut output = Vec::with_capacity(self.financial_liabilities.len());
        for obligation in &self.financial_liabilities {
            if obligation.price_base_year != valuation_year {
                return Err(CohortAccountingError::PriceYearMismatch {
                    obligation_id: obligation.obligation_id.clone(),
                    expected: valuation_year,
                    actual: obligation.price_base_year,
                });
            }
            if obligation.due_year < valuation_year {
                return Err(CohortAccountingError::InvalidYear {
                    year: obligation.due_year,
                });
            }
            let years = obligation.due_year - valuation_year;
            let factor = (1.0 + real_discount_rate).powi(-years);
            output.push(DiscountedFinancialLiability {
                obligation_id: obligation.obligation_id.clone(),
                cohort_id: obligation.assignment.assigned_cohort().cloned(),
                currency_code: obligation.currency_code.clone(),
                price_base_year: obligation.price_base_year,
                due_year: obligation.due_year,
                value: MoneyAmount {
                    amount: obligation.amount,
                    discount_factor: factor,
                    discounted_amount: obligation.amount * factor,
                },
            });
        }
        Ok(output)
    }
}

fn validate_obligation_header<F>(
    obligation_id: &str,
    action_id: &str,
    created_year: i32,
    due_year: i32,
    assignment: &LiabilityAssignment,
    known_cohort: &F,
) -> Result<(), CohortAccountingError>
where
    F: Fn(&CohortId) -> bool,
{
    if obligation_id.trim().is_empty() || action_id.trim().is_empty() {
        return Err(CohortAccountingError::EmptyObligationId);
    }
    if due_year < created_year {
        return Err(CohortAccountingError::InvalidDueDate {
            obligation_id: obligation_id.to_string(),
        });
    }
    match assignment {
        LiabilityAssignment::Assigned(cohort_id) if !known_cohort(cohort_id) => {
            return Err(CohortAccountingError::UnknownCohort(cohort_id.clone()));
        }
        LiabilityAssignment::Unassigned { reason } if reason.trim().is_empty() => {
            return Err(CohortAccountingError::EmptyUnassignedReason {
                obligation_id: obligation_id.to_string(),
            });
        }
        _ => {}
    }
    Ok(())
}

fn compare_component(
    resource: &'static str,
    cohort_total: f64,
    ledger_total: f64,
    region_id: &str,
    year: i32,
) -> Result<(), CohortAccountingError> {
    if !approximately_equal(cohort_total, ledger_total) {
        return Err(CohortAccountingError::ServiceReceiptMismatch {
            region_id: region_id.to_string(),
            year,
            resource,
            cohort_total,
            ledger_total,
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntheticPolicy {
    MaintainNow,
    DeferMaintenanceAndExportWater,
}

impl SyntheticPolicy {
    pub const ALL: [Self; 2] = [Self::MaintainNow, Self::DeferMaintenanceAndExportWater];

    pub const fn name(self) -> &'static str {
        match self {
            Self::MaintainNow => "maintain_now",
            Self::DeferMaintenanceAndExportWater => "defer_maintenance_and_export_water",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CohortScenarioOutcome {
    pub policy: SyntheticPolicy,
    pub region_ledgers_conserved: bool,
    pub current_household_water_served_m3: f64,
    pub current_household_water_unmet_m3: f64,
    pub current_maintenance_energy_served_kwh: f64,
    pub future_inherited_water_m3: f64,
    pub future_inherited_energy_kwh: f64,
    pub future_maintenance_energy_liability_kwh: f64,
    pub future_financial_liability_zar: f64,
    pub future_pump_condition_index: f64,
    pub assigned_liability_records: usize,
    pub unassigned_liability_records: usize,
    pub cohort_service_reconciled: bool,
}

pub fn run_synthetic_cohort_policy(
    policy: SyntheticPolicy,
) -> Result<CohortScenarioOutcome, CohortAccountingError> {
    use civdyn_resource_ledger::{
        Production, Region, ResourceVector, StepInput, TransferPolicy, TransferRequest, World,
    };

    let older_id = CohortId::new("cohort_older_1945_1965")?;
    let current_id = CohortId::new("cohort_current_1980_2000")?;
    let future_id = CohortId::new("cohort_future_2050_2070")?;

    let cohorts = vec![
        Cohort {
            id: older_id.clone(),
            birth_start_year: 1945,
            birth_end_year_exclusive: 1965,
            population_assumption: 1.0,
        },
        Cohort {
            id: current_id.clone(),
            birth_start_year: 1980,
            birth_end_year_exclusive: 2000,
            population_assumption: 1.0,
        },
        Cohort {
            id: future_id.clone(),
            birth_start_year: 2050,
            birth_end_year_exclusive: 2070,
            population_assumption: 1.0,
        },
    ];

    // Maintenance performed now is an explicit energy transfer and served
    // demand in the shared physical ledger; deferred maintenance stays as a
    // future obligation rather than being treated as free current work.
    let export_water_m3 = match policy {
        SyntheticPolicy::MaintainNow => 0.0,
        SyntheticPolicy::DeferMaintenanceAndExportWater => 30.0,
    };
    let maintenance_energy_now_kwh = match policy {
        SyntheticPolicy::MaintainNow => 40.0,
        SyntheticPolicy::DeferMaintenanceAndExportWater => 0.0,
    };
    let mut world = World::new(vec![
        Region {
            id: "basin_households".to_string(),
            population: 2.0,
            opening_stock: ResourceVector::new(100.0, 100.0, 0.0),
            essential_demand_per_capita: ResourceVector::new(10.0, 0.0, 0.0),
        },
        Region {
            id: "industry".to_string(),
            population: 0.0,
            opening_stock: ResourceVector::default(),
            essential_demand_per_capita: ResourceVector::default(),
        },
        Region {
            id: "pump_maintenance".to_string(),
            population: 1.0,
            opening_stock: ResourceVector::default(),
            essential_demand_per_capita: ResourceVector::new(
                0.0,
                maintenance_energy_now_kwh,
                0.0,
            ),
        },
    ])
    .map_err(|_| CohortAccountingError::InvalidQuantity {
        field: "resource_world",
    })?;

    let mut transfers = vec![TransferRequest {
        from_region: 0,
        to_region: 1,
        amount: ResourceVector::new(export_water_m3, 0.0, 0.0),
    }];
    if maintenance_energy_now_kwh > 0.0 {
        transfers.push(TransferRequest {
            from_region: 0,
            to_region: 2,
            amount: ResourceVector::new(0.0, maintenance_energy_now_kwh, 0.0),
        });
    }
    let report = world
        .step(
            &StepInput {
                production: vec![Production::default(); 3],
                transfers,
            },
            TransferPolicy::Unrestricted,
        )
        .map_err(|_| CohortAccountingError::InvalidQuantity {
            field: "resource_world_step",
        })?;
    let basin = report
        .regions
        .first()
        .ok_or(CohortAccountingError::InvalidQuantity {
            field: "resource_region_report",
        })?;
    let maintenance_region = report
        .regions
        .get(2)
        .ok_or(CohortAccountingError::InvalidQuantity {
            field: "maintenance_region_report",
        })?;

    let physical_liabilities = match policy {
        SyntheticPolicy::MaintainNow => Vec::new(),
        SyntheticPolicy::DeferMaintenanceAndExportWater => vec![PhysicalLiability {
            obligation_id: "maintain_pump_2050".to_string(),
            originating_action_id: "defer_pump_maintenance".to_string(),
            created_year: 2026,
            due_year: 2050,
            kind: PhysicalLiabilityKind::Maintenance,
            assignment: LiabilityAssignment::Assigned(future_id.clone()),
            quantity: ResourceVector::new(0.0, 40.0, 0.0),
        }],
    };

    let financial_liabilities = match policy {
        SyntheticPolicy::MaintainNow => Vec::new(),
        SyntheticPolicy::DeferMaintenanceAndExportWater => vec![FinancialLiability {
            obligation_id: "deferred_debt_service_2050".to_string(),
            originating_action_id: "defer_fiscal_consolidation".to_string(),
            created_year: 2026,
            due_year: 2050,
            assignment: LiabilityAssignment::Assigned(future_id.clone()),
            currency_code: "ZAR".to_string(),
            price_base_year: 2026,
            amount: 1_000_000.0,
        }],
    };

    let pump_condition = match policy {
        SyntheticPolicy::MaintainNow => 0.95,
        SyntheticPolicy::DeferMaintenanceAndExportWater => 0.60,
    };
    let financial_ids = financial_liabilities
        .iter()
        .map(|liability| liability.obligation_id.clone())
        .collect();
    let inheritances = vec![CohortInheritance {
        cohort_id: future_id.clone(),
        inherited_on_year: 2050,
        source_closing_year: 2049,
        inherited_resource_stocks: basin.closing_stock,
        inherited_capital: vec![CapitalMeasure {
            asset_id: "basin_pump_001".to_string(),
            metric_id: "pump_condition".to_string(),
            capital_class: CapitalClass::Produced,
            unit: "index_0_1".to_string(),
            value: pump_condition,
        }],
        inherited_financial_obligation_ids: financial_ids,
    }];

    let ledger = CohortIncidenceLedger {
        scenario_id: policy.name().to_string(),
        horizon: SimulationHorizon {
            start_year: 2026,
            end_year_exclusive: 2027,
        },
        cohorts,
        services: vec![
            CohortServiceRecord {
                service_kind: CohortServiceKind::HouseholdService,
                cohort_id: older_id,
                region_id: "basin_households".to_string(),
                year: 2026,
                served: ResourceVector::new(10.0, 0.0, 0.0),
                unmet: ResourceVector::default(),
                people_exposed: 1.0,
            },
            CohortServiceRecord {
                service_kind: CohortServiceKind::HouseholdService,
                cohort_id: current_id.clone(),
                region_id: "basin_households".to_string(),
                year: 2026,
                served: ResourceVector::new(10.0, 0.0, 0.0),
                unmet: ResourceVector::default(),
                people_exposed: 1.0,
            },
            CohortServiceRecord {
                service_kind: CohortServiceKind::AssetMaintenance,
                cohort_id: current_id,
                region_id: "pump_maintenance".to_string(),
                year: 2026,
                served: ResourceVector::new(0.0, maintenance_energy_now_kwh, 0.0),
                unmet: ResourceVector::default(),
                people_exposed: 1.0,
            },
        ],
        physical_liabilities,
        financial_liabilities,
        inheritances,
    };

    ledger.validate()?;
    ledger.reconcile_region_service(basin, 2026)?;
    ledger.reconcile_region_service(maintenance_region, 2026)?;
    let cohort_service_reconciled = true;
    let region_ledgers_conserved = report
        .regions
        .iter()
        .all(|region| region.ledger.is_conserved());

    let future = ledger
        .inheritances
        .iter()
        .find(|record| record.cohort_id == future_id)
        .ok_or(CohortAccountingError::InvalidQuantity {
            field: "future_inheritance",
        })?;
    let future_pump_condition = future
        .inherited_capital
        .iter()
        .find(|measure| measure.metric_id == "pump_condition")
        .map(|measure| measure.value)
        .unwrap_or_default();

    Ok(CohortScenarioOutcome {
        policy,
        region_ledgers_conserved,
        current_household_water_served_m3: basin.served.water_m3,
        current_household_water_unmet_m3: basin.unmet.water_m3,
        current_maintenance_energy_served_kwh: maintenance_region.served.energy_kwh,
        future_inherited_water_m3: future.inherited_resource_stocks.water_m3,
        future_inherited_energy_kwh: future.inherited_resource_stocks.energy_kwh,
        future_maintenance_energy_liability_kwh: ledger.total_physical_liabilities().energy_kwh,
        future_financial_liability_zar: ledger
            .financial_liabilities
            .iter()
            .filter(|liability| {
                liability.assignment.assigned_cohort() == Some(&future_id)
                    && liability.currency_code == "ZAR"
            })
            .map(|liability| liability.amount)
            .sum(),
        future_pump_condition_index: future_pump_condition,
        assigned_liability_records: ledger
            .physical_liabilities
            .iter()
            .filter(|liability| liability.assignment.assigned_cohort().is_some())
            .count()
            + ledger
                .financial_liabilities
                .iter()
                .filter(|liability| liability.assignment.assigned_cohort().is_some())
                .count(),
        unassigned_liability_records: ledger
            .physical_liabilities
            .iter()
            .filter(|liability| matches!(liability.assignment, LiabilityAssignment::Unassigned { .. }))
            .count()
            + ledger
                .financial_liabilities
                .iter()
                .filter(|liability| matches!(liability.assignment, LiabilityAssignment::Unassigned { .. }))
                .count(),
        cohort_service_reconciled,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(policy: SyntheticPolicy) -> CohortScenarioOutcome {
        run_synthetic_cohort_policy(policy).expect("synthetic cohort policy must validate")
    }

    fn base_ledger() -> CohortIncidenceLedger {
        let maintain = outcome(SyntheticPolicy::MaintainNow);
        assert!(maintain.region_ledger_conserved);
        assert!(maintain.cohort_service_reconciled);
        let _ = maintain;
        let older = CohortId::new("older").expect("id");
        let future = CohortId::new("future").expect("id");
        CohortIncidenceLedger {
            scenario_id: "metamorphic_fixture".to_string(),
            horizon: SimulationHorizon {
                start_year: 2026,
                end_year_exclusive: 2030,
            },
            cohorts: vec![
                Cohort {
                    id: older.clone(),
                    birth_start_year: 1950,
                    birth_end_year_exclusive: 1970,
                    population_assumption: 10.0,
                },
                Cohort {
                    id: future.clone(),
                    birth_start_year: 2030,
                    birth_end_year_exclusive: 2050,
                    population_assumption: 10.0,
                },
            ],
            services: vec![CohortServiceRecord {
                cohort_id: older,
                region_id: "basin".to_string(),
                year: 2026,
                served: ResourceVector::new(20.0, 0.0, 0.0),
                unmet: ResourceVector::default(),
                people_exposed: 10.0,
            }],
            physical_liabilities: vec![PhysicalLiability {
                obligation_id: "pump_debt".to_string(),
                originating_action_id: "policy_x".to_string(),
                created_year: 2026,
                due_year: 2030,
                kind: PhysicalLiabilityKind::Maintenance,
                assignment: LiabilityAssignment::Assigned(future.clone()),
                quantity: ResourceVector::new(0.0, 40.0, 0.0),
            }],
            financial_liabilities: vec![FinancialLiability {
                obligation_id: "fiscal_debt".to_string(),
                originating_action_id: "policy_x".to_string(),
                created_year: 2026,
                due_year: 2030,
                assignment: LiabilityAssignment::Assigned(future.clone()),
                currency_code: "ZAR".to_string(),
                price_base_year: 2026,
                amount: 1_000.0,
            }],
            inheritances: vec![CohortInheritance {
                cohort_id: future,
                inherited_on_year: 2030,
                source_closing_year: 2029,
                inherited_resource_stocks: ResourceVector::new(80.0, 60.0, 0.0),
                inherited_capital: vec![CapitalMeasure {
                    asset_id: "pump".to_string(),
                    metric_id: "condition".to_string(),
                    capital_class: CapitalClass::Produced,
                    unit: "index_0_1".to_string(),
                    value: 0.6,
                }],
                inherited_financial_obligation_ids: vec!["fiscal_debt".to_string()],
            }],
        }
    }

    #[test]
    fn matched_policies_keep_current_household_service_identical() {
        let maintained = outcome(SyntheticPolicy::MaintainNow);
        let deferred = outcome(SyntheticPolicy::DeferMaintenanceAndExportWater);
        assert_eq!(maintained.current_household_water_served_m3, 20.0);
        assert_eq!(deferred.current_household_water_served_m3, 20.0);
        assert_eq!(maintained.current_household_water_unmet_m3, 0.0);
        assert_eq!(deferred.current_household_water_unmet_m3, 0.0);
        assert!(maintained.cohort_service_reconciled);
        assert!(deferred.cohort_service_reconciled);
    }

    #[test]
    fn deferred_policy_exposes_inherited_water_maintenance_and_money_separately() {
        let maintained = outcome(SyntheticPolicy::MaintainNow);
        let deferred = outcome(SyntheticPolicy::DeferMaintenanceAndExportWater);
        assert_eq!(maintained.future_inherited_water_m3, 80.0);
        assert_eq!(deferred.future_inherited_water_m3, 50.0);
        assert_eq!(maintained.current_maintenance_energy_served_kwh, 40.0);
        assert_eq!(deferred.current_maintenance_energy_served_kwh, 0.0);
        assert_eq!(maintained.future_inherited_energy_kwh, 60.0);
        assert_eq!(deferred.future_inherited_energy_kwh, 100.0);
        assert_eq!(maintained.future_maintenance_energy_liability_kwh, 0.0);
        assert_eq!(deferred.future_maintenance_energy_liability_kwh, 40.0);
        assert_eq!(maintained.future_financial_liability_zar, 0.0);
        assert_eq!(deferred.future_financial_liability_zar, 1_000_000.0);
        assert_eq!(maintained.future_pump_condition_index, 0.95);
        assert_eq!(deferred.future_pump_condition_index, 0.60);
        assert!(maintained.region_ledgers_conserved);
        assert!(deferred.region_ledgers_conserved);
    }

    #[test]
    fn discount_rate_changes_money_report_but_never_physical_liabilities() {
        let ledger = base_ledger();
        ledger.validate().expect("fixture validates");
        let before = ledger.total_physical_liabilities();
        let low = ledger
            .discounted_financial_liabilities(2026, 0.01)
            .expect("low-rate discount");
        let high = ledger
            .discounted_financial_liabilities(2026, 0.08)
            .expect("high-rate discount");
        let after = ledger.total_physical_liabilities();
        assert_eq!(before.water_m3, after.water_m3);
        assert_eq!(before.energy_kwh, after.energy_kwh);
        assert_eq!(before.food_kg, after.food_kg);
        assert!(low[0].value.discounted_amount > high[0].value.discounted_amount);
        assert_eq!(ledger.financial_liabilities[0].amount, 1_000.0);
    }

    #[test]
    fn cohort_service_splitting_preserves_physical_totals() {
        let mut ledger = base_ledger();
        let before = ledger.total_service_by_region_year();
        let original = ledger.services.remove(0);
        let id_a = CohortId::new("older_a").expect("id");
        let id_b = CohortId::new("older_b").expect("id");
        ledger.cohorts.push(Cohort {
            id: id_a.clone(),
            birth_start_year: 1950,
            birth_end_year_exclusive: 1960,
            population_assumption: 5.0,
        });
        ledger.cohorts.push(Cohort {
            id: id_b.clone(),
            birth_start_year: 1960,
            birth_end_year_exclusive: 1970,
            population_assumption: 5.0,
        });
        for cohort_id in [id_a, id_b] {
            ledger.services.push(CohortServiceRecord {
                cohort_id,
                region_id: original.region_id.clone(),
                year: original.year,
                served: ResourceVector::new(10.0, 0.0, 0.0),
                unmet: ResourceVector::default(),
                people_exposed: 5.0,
            });
        }
        let after = ledger.total_service_by_region_year();
        assert_eq!(before, after);
    }

    #[test]
    fn liability_reassignment_changes_incidence_not_physical_total() {
        let mut ledger = base_ledger();
        let before = ledger.total_physical_liabilities();
        let new_cohort = CohortId::new("older").expect("id");
        ledger.physical_liabilities[0].assignment =
            LiabilityAssignment::Assigned(new_cohort.clone());
        ledger.validate().expect("reassigned liability remains valid");
        let after = ledger.total_physical_liabilities();
        assert_eq!(before.energy_kwh, after.energy_kwh);
        assert_eq!(before.water_m3, after.water_m3);
        assert_eq!(before.food_kg, after.food_kg);
    }

    #[test]
    fn inheritance_references_financial_liabilities_without_double_counting() {
        let ledger = base_ledger();
        ledger.validate().expect("fixture validates");
        assert_eq!(ledger.financial_liabilities.len(), 1);
        assert_eq!(
            ledger.inheritances[0].inherited_financial_obligation_ids,
            vec!["fiscal_debt".to_string()]
        );
        let total_financial_debt: f64 = ledger
            .financial_liabilities
            .iter()
            .map(|liability| liability.amount)
            .sum();
        assert_eq!(total_financial_debt, 1_000.0);
    }

    #[test]
    fn unassigned_liability_is_explicit_and_not_discarded() {
        let mut ledger = base_ledger();
        ledger.physical_liabilities.push(PhysicalLiability {
            obligation_id: "unknown_maintenance".to_string(),
            originating_action_id: "unknown_decision".to_string(),
            created_year: 2026,
            due_year: 2030,
            kind: PhysicalLiabilityKind::Maintenance,
            assignment: LiabilityAssignment::Unassigned {
                reason: "source does not identify the liable cohort".to_string(),
            },
            quantity: ResourceVector::new(0.0, 5.0, 0.0),
        });
        ledger.validate().expect("unassigned reason is explicit");
        assert_eq!(ledger.total_physical_liabilities().energy_kwh, 45.0);
    }
}
