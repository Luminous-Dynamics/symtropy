// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Cohort-indexed incidence accounting over the shared CIVDYN resource ledger.
//!
//! The assembler never invents cohort shares. Every service flow and inherited
//! stock must be assigned to a named cohort or explicitly marked unassigned.
//! Financial obligations, physical obligations and resource stocks remain
//! separate records with separate units.

use std::collections::{BTreeMap, BTreeSet};
use core::fmt;

use civdyn_resource_ledger::{Resource, ResourceVector, StepReport};

const RESOURCES: [Resource; 3] = [Resource::Water, Resource::Energy, Resource::Food];
const ABSOLUTE_TOLERANCE: f64 = 1e-9;
const RELATIVE_TOLERANCE: f64 = 1e-10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnassignedReason {
    SharedPublicAsset,
    SourceIncidenceUnavailable,
    AwaitingAllocation,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CohortId(String);

impl CohortId {
    pub fn new(value: impl Into<String>) -> Result<Self, IncidenceError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(IncidenceError::InvalidText("cohort_id"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CohortAssignment {
    Assigned(CohortId),
    Unassigned(UnassignedReason),
}

impl CohortAssignment {
    pub fn assigned(value: impl Into<String>) -> Result<Self, IncidenceError> {
        Ok(Self::Assigned(CohortId::new(value)?))
    }

    pub const fn unassigned(reason: UnassignedReason) -> Self {
        Self::Unassigned(reason)
    }

    pub const fn is_unassigned(&self) -> bool {
        matches!(self, Self::Unassigned(_))
    }

    pub fn label(&self) -> String {
        match self {
            Self::Assigned(cohort) => cohort.as_str().to_string(),
            Self::Unassigned(reason) => format!("unassigned:{reason:?}"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ResourceAmounts {
    pub water_m3: f64,
    pub energy_kwh: f64,
    pub food_kg: f64,
}

impl ResourceAmounts {
    pub const fn new(water_m3: f64, energy_kwh: f64, food_kg: f64) -> Self {
        Self {
            water_m3,
            energy_kwh,
            food_kg,
        }
    }

    pub fn get(self, resource: Resource) -> f64 {
        match resource {
            Resource::Water => self.water_m3,
            Resource::Energy => self.energy_kwh,
            Resource::Food => self.food_kg,
        }
    }

    pub fn add_resource(&mut self, resource: Resource, amount: f64) {
        match resource {
            Resource::Water => self.water_m3 += amount,
            Resource::Energy => self.energy_kwh += amount,
            Resource::Food => self.food_kg += amount,
        }
    }

    fn from_vector(vector: ResourceVector) -> Self {
        Self::new(vector.water_m3, vector.energy_kwh, vector.food_kg)
    }

    fn is_close(self, other: Self) -> bool {
        RESOURCES
            .into_iter()
            .all(|resource| close(self.get(resource), other.get(resource)))
    }
}

fn close(left: f64, right: f64) -> bool {
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= ABSOLUTE_TOLERANCE + RELATIVE_TOLERANCE * scale
}

fn validate_text(value: &str, field: &'static str) -> Result<(), IncidenceError> {
    if value.trim().is_empty() {
        return Err(IncidenceError::InvalidText(field));
    }
    Ok(())
}

fn validate_quantity(value: f64, field: &'static str) -> Result<(), IncidenceError> {
    if !value.is_finite() || value < 0.0 {
        return Err(IncidenceError::InvalidQuantity(field));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServiceAllocation {
    pub allocation_id: String,
    pub assignment: CohortAssignment,
    pub region_id: String,
    pub resource: Resource,
    pub demand: f64,
    pub served: f64,
    pub originating_action_id: String,
    pub source_evidence_ref: String,
}

impl ServiceAllocation {
    fn validate(&self, ids: &mut BTreeSet<String>) -> Result<(), IncidenceError> {
        validate_text(&self.allocation_id, "service_allocation_id")?;
        validate_text(&self.region_id, "service_region_id")?;
        validate_text(&self.originating_action_id, "service_action_id")?;
        validate_text(&self.source_evidence_ref, "service_source_ref")?;
        if !ids.insert(self.allocation_id.clone()) {
            return Err(IncidenceError::DuplicateRecordId(self.allocation_id.clone()));
        }
        validate_quantity(self.demand, "service_demand")?;
        validate_quantity(self.served, "service_served")?;
        if self.served > self.demand && !close(self.served, self.demand) {
            return Err(IncidenceError::ServedExceedsDemand {
                allocation_id: self.allocation_id.clone(),
            });
        }
        Ok(())
    }

    pub fn unmet(&self) -> f64 {
        (self.demand - self.served).max(0.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InheritedStockAllocation {
    pub allocation_id: String,
    pub assignment: CohortAssignment,
    pub region_id: String,
    pub asset_id: String,
    pub resource: Resource,
    pub closing_stock: f64,
    pub source_evidence_ref: String,
}

impl InheritedStockAllocation {
    fn validate(&self, ids: &mut BTreeSet<String>) -> Result<(), IncidenceError> {
        validate_text(&self.allocation_id, "stock_allocation_id")?;
        validate_text(&self.region_id, "stock_region_id")?;
        validate_text(&self.asset_id, "stock_asset_id")?;
        validate_text(&self.source_evidence_ref, "stock_source_ref")?;
        if !ids.insert(self.allocation_id.clone()) {
            return Err(IncidenceError::DuplicateRecordId(self.allocation_id.clone()));
        }
        validate_quantity(self.closing_stock, "inherited_closing_stock")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum QuantityUncertainty {
    Known,
    Interval { lower: f64, upper: f64 },
    Unknown { reason: String },
}

impl QuantityUncertainty {
    fn validate(&self) -> Result<(), IncidenceError> {
        match self {
            Self::Known => Ok(()),
            Self::Interval { lower, upper }
                if lower.is_finite()
                    && upper.is_finite()
                    && *lower >= 0.0
                    && *upper >= *lower =>
            {
                Ok(())
            }
            Self::Interval { .. } => Err(IncidenceError::InvalidUncertainty),
            Self::Unknown { reason } => validate_text(reason, "uncertainty_reason"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhysicalObligation {
    pub obligation_id: String,
    pub originating_action_id: String,
    pub created_tick: u64,
    pub due_tick: u64,
    pub region_id: String,
    pub assignment: CohortAssignment,
    pub measure_id: String,
    pub unit: String,
    pub quantity: f64,
    pub uncertainty: QuantityUncertainty,
    pub source_evidence_ref: String,
}

impl PhysicalObligation {
    fn validate(
        &self,
        source_tick: u64,
        ids: &mut BTreeSet<String>,
    ) -> Result<(), IncidenceError> {
        validate_text(&self.obligation_id, "physical_obligation_id")?;
        validate_text(&self.originating_action_id, "physical_obligation_action_id")?;
        validate_text(&self.region_id, "physical_obligation_region_id")?;
        validate_text(&self.measure_id, "physical_obligation_measure_id")?;
        validate_text(&self.unit, "physical_obligation_unit")?;
        validate_text(&self.source_evidence_ref, "physical_obligation_source_ref")?;
        validate_time(self.created_tick, self.due_tick, source_tick)?;
        if !ids.insert(self.obligation_id.clone()) {
            return Err(IncidenceError::DuplicateRecordId(self.obligation_id.clone()));
        }
        validate_quantity(self.quantity, "physical_obligation_quantity")?;
        self.uncertainty.validate()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinancialObligationKind {
    Principal,
    Interest,
    Tax,
    Pension,
    Contingent,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FinancialObligation {
    pub obligation_id: String,
    pub originating_action_id: String,
    pub created_tick: u64,
    pub due_tick: u64,
    pub liable_cohort: CohortAssignment,
    pub recipient_cohort: Option<CohortAssignment>,
    pub amount: f64,
    pub currency_code: String,
    pub price_base_year: u16,
    pub kind: FinancialObligationKind,
    pub uncertainty: QuantityUncertainty,
    pub source_evidence_ref: String,
}

impl FinancialObligation {
    fn validate(
        &self,
        source_tick: u64,
        ids: &mut BTreeSet<String>,
    ) -> Result<(), IncidenceError> {
        validate_text(&self.obligation_id, "financial_obligation_id")?;
        validate_text(&self.originating_action_id, "financial_obligation_action_id")?;
        validate_text(&self.source_evidence_ref, "financial_obligation_source_ref")?;
        if self.currency_code.len() != 3
            || !self.currency_code.bytes().all(|byte| byte.is_ascii_uppercase())
        {
            return Err(IncidenceError::InvalidCurrencyCode);
        }
        if self.price_base_year == 0 {
            return Err(IncidenceError::InvalidPriceBaseYear);
        }
        validate_time(self.created_tick, self.due_tick, source_tick)?;
        if !ids.insert(self.obligation_id.clone()) {
            return Err(IncidenceError::DuplicateRecordId(self.obligation_id.clone()));
        }
        validate_quantity(self.amount, "financial_obligation_amount")?;
        self.uncertainty.validate()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapitalClass {
    Produced,
    Human,
    Natural,
    Institutional,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InheritedCapitalRecord {
    pub record_id: String,
    pub assignment: CohortAssignment,
    pub region_id: String,
    pub asset_id: String,
    pub capital_class: CapitalClass,
    pub measure_id: String,
    pub unit: String,
    pub quantity: f64,
    /// Optional normalized condition measure in [0, 1].
    pub condition: Option<f64>,
    pub uncertainty: QuantityUncertainty,
    pub source_evidence_ref: String,
}

impl InheritedCapitalRecord {
    fn validate(&self, ids: &mut BTreeSet<String>) -> Result<(), IncidenceError> {
        validate_text(&self.record_id, "capital_record_id")?;
        validate_text(&self.region_id, "capital_region_id")?;
        validate_text(&self.asset_id, "capital_asset_id")?;
        validate_text(&self.measure_id, "capital_measure_id")?;
        validate_text(&self.unit, "capital_unit")?;
        validate_text(&self.source_evidence_ref, "capital_source_ref")?;
        if !ids.insert(self.record_id.clone()) {
            return Err(IncidenceError::DuplicateRecordId(self.record_id.clone()));
        }
        validate_quantity(self.quantity, "capital_quantity")?;
        if let Some(condition) = self.condition
            && (!condition.is_finite() || !(0.0..=1.0).contains(&condition))
        {
            return Err(IncidenceError::InvalidCapitalCondition);
        }
        self.uncertainty.validate()
    }
}

fn validate_time(created: u64, due: u64, source_tick: u64) -> Result<(), IncidenceError> {
    if due < created {
        return Err(IncidenceError::InvalidTimeRange);
    }
    if created > source_tick {
        return Err(IncidenceError::RecordCreatedAfterSourceTick);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegionalResourceReconciliation {
    pub region_id: String,
    pub resource: Resource,
    pub ledger_demand: f64,
    pub allocated_demand: f64,
    pub ledger_served: f64,
    pub allocated_served: f64,
    pub ledger_unmet: f64,
    pub allocated_unmet: f64,
    pub ledger_closing_stock: f64,
    pub allocated_closing_stock: f64,
}

impl RegionalResourceReconciliation {
    pub fn is_reconciled(&self) -> bool {
        close(self.ledger_demand, self.allocated_demand)
            && close(self.ledger_served, self.allocated_served)
            && close(self.ledger_unmet, self.allocated_unmet)
            && close(self.ledger_closing_stock, self.allocated_closing_stock)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CohortSummary {
    pub assignment: CohortAssignment,
    pub service_demand: ResourceAmounts,
    pub service_served: ResourceAmounts,
    pub service_unmet: ResourceAmounts,
    pub inherited_stock: ResourceAmounts,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TickIncidenceReport {
    pub tick: u64,
    pub service_allocations: Vec<ServiceAllocation>,
    pub inherited_stocks: Vec<InheritedStockAllocation>,
    pub physical_obligations: Vec<PhysicalObligation>,
    pub financial_obligations: Vec<FinancialObligation>,
    pub capital_records: Vec<InheritedCapitalRecord>,
    pub regional_reconciliations: Vec<RegionalResourceReconciliation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CohortIncidenceLedger {
    reports: Vec<TickIncidenceReport>,
}

impl CohortIncidenceLedger {
    pub fn new() -> Self {
        Self { reports: Vec::new() }
    }

    /// Assemble one cohort-incidence report from an existing resource-ledger
    /// step. No cohort shares are imputed: allocations must reconcile exactly
    /// within the ledger's declared unit-specific tolerances.
    pub fn assemble_tick(
        &mut self,
        source: &StepReport,
        service_allocations: Vec<ServiceAllocation>,
        inherited_stocks: Vec<InheritedStockAllocation>,
        physical_obligations: Vec<PhysicalObligation>,
        financial_obligations: Vec<FinancialObligation>,
        capital_records: Vec<InheritedCapitalRecord>,
    ) -> Result<&TickIncidenceReport, IncidenceError> {
        if source.regions.is_empty() {
            return Err(IncidenceError::EmptySourceReport);
        }
        if self.reports.last().is_some_and(|last| source.tick <= last.tick) {
            return Err(IncidenceError::NonIncreasingTick);
        }

        let mut region_ids = BTreeSet::new();
        for region in &source.regions {
            if !region_ids.insert(region.region_id.as_str()) {
                return Err(IncidenceError::DuplicateRegionId(region.region_id.clone()));
            }
            if !region.ledger.is_conserved() {
                return Err(IncidenceError::SourceLedgerNotConserved(region.region_id.clone()));
            }
        }

        let mut record_ids = BTreeSet::new();
        for allocation in &service_allocations {
            allocation.validate(&mut record_ids)?;
            ensure_region(&region_ids, &allocation.region_id)?;
        }
        for allocation in &inherited_stocks {
            allocation.validate(&mut record_ids)?;
            ensure_region(&region_ids, &allocation.region_id)?;
        }
        for obligation in &physical_obligations {
            obligation.validate(source.tick, &mut record_ids)?;
            ensure_region(&region_ids, &obligation.region_id)?;
        }
        for obligation in &financial_obligations {
            obligation.validate(source.tick, &mut record_ids)?;
        }
        for record in &capital_records {
            record.validate(&mut record_ids)?;
            ensure_region(&region_ids, &record.region_id)?;
        }

        let mut reconciliations = Vec::with_capacity(source.regions.len() * RESOURCES.len());
        for region in &source.regions {
            for resource in RESOURCES {
                let allocated_demand = service_allocations
                    .iter()
                    .filter(|row| row.region_id == region.region_id && row.resource == resource)
                    .map(|row| row.demand)
                    .sum::<f64>();
                let allocated_served = service_allocations
                    .iter()
                    .filter(|row| row.region_id == region.region_id && row.resource == resource)
                    .map(|row| row.served)
                    .sum::<f64>();
                let allocated_unmet = service_allocations
                    .iter()
                    .filter(|row| row.region_id == region.region_id && row.resource == resource)
                    .map(ServiceAllocation::unmet)
                    .sum::<f64>();
                let allocated_stock = inherited_stocks
                    .iter()
                    .filter(|row| row.region_id == region.region_id && row.resource == resource)
                    .map(|row| row.closing_stock)
                    .sum::<f64>();

                let expected = RegionalResourceReconciliation {
                    region_id: region.region_id.clone(),
                    resource,
                    ledger_demand: region.essential_demand.get(resource),
                    allocated_demand,
                    ledger_served: region.served.get(resource),
                    allocated_served,
                    ledger_unmet: region.unmet.get(resource),
                    allocated_unmet,
                    ledger_closing_stock: region.closing_stock.get(resource),
                    allocated_closing_stock: allocated_stock,
                };

                if !expected.is_reconciled() {
                    return Err(IncidenceError::RegionalReconciliationFailed {
                        region: region.region_id.clone(),
                        resource,
                    });
                }
                reconciliations.push(expected);
            }
        }

        self.reports.push(TickIncidenceReport {
            tick: source.tick,
            service_allocations,
            inherited_stocks,
            physical_obligations,
            financial_obligations,
            capital_records,
            regional_reconciliations: reconciliations,
        });

        Ok(self.reports.last().expect("report was just appended"))
    }

    pub fn reports(&self) -> &[TickIncidenceReport] {
        &self.reports
    }

    pub fn last(&self) -> Option<&TickIncidenceReport> {
        self.reports.last()
    }

    pub fn cohort_summaries(&self, tick: u64) -> Option<Vec<CohortSummary>> {
        let report = self.reports.iter().find(|report| report.tick == tick)?;
        let mut summaries: BTreeMap<CohortAssignment, CohortSummary> = BTreeMap::new();

        for row in &report.service_allocations {
            let summary = summaries
                .entry(row.assignment.clone())
                .or_insert_with(|| CohortSummary {
                    assignment: row.assignment.clone(),
                    service_demand: ResourceAmounts::default(),
                    service_served: ResourceAmounts::default(),
                    service_unmet: ResourceAmounts::default(),
                    inherited_stock: ResourceAmounts::default(),
                });
            summary.service_demand.add_resource(row.resource, row.demand);
            summary.service_served.add_resource(row.resource, row.served);
            summary.service_unmet.add_resource(row.resource, row.unmet());
        }

        for row in &report.inherited_stocks {
            let summary = summaries
                .entry(row.assignment.clone())
                .or_insert_with(|| CohortSummary {
                    assignment: row.assignment.clone(),
                    service_demand: ResourceAmounts::default(),
                    service_served: ResourceAmounts::default(),
                    service_unmet: ResourceAmounts::default(),
                    inherited_stock: ResourceAmounts::default(),
                });
            summary
                .inherited_stock
                .add_resource(row.resource, row.closing_stock);
        }

        Some(summaries.into_values().collect())
    }

    pub fn unassigned_record_count(&self, tick: u64) -> Option<usize> {
        let report = self.reports.iter().find(|report| report.tick == tick)?;
        Some(
            report
                .service_allocations
                .iter()
                .filter(|row| row.assignment.is_unassigned())
                .count()
                + report
                    .inherited_stocks
                    .iter()
                    .filter(|row| row.assignment.is_unassigned())
                    .count()
                + report
                    .physical_obligations
                    .iter()
                    .filter(|row| row.assignment.is_unassigned())
                    .count()
                + report
                    .financial_obligations
                    .iter()
                    .filter(|row| {
                    row.liable_cohort.is_unassigned()
                        || row
                            .recipient_cohort
                            .as_ref()
                            .is_some_and(CohortAssignment::is_unassigned)
                })
                    .count()
                + report
                    .capital_records
                    .iter()
                    .filter(|row| row.assignment.is_unassigned())
                    .count(),
        )
    }

    /// Produce a normative sensitivity view without changing physical records.
    /// Discounted equivalents are analytical values, never stock/flow quantities.
    pub fn discounted_service_view(
        &self,
        tick: u64,
        discount_rate: f64,
        reference_tick: u64,
    ) -> Result<Option<DiscountedServiceView>, IncidenceError> {
        if !discount_rate.is_finite() || discount_rate <= -1.0 {
            return Err(IncidenceError::InvalidDiscountRate);
        }
        if reference_tick > tick {
            return Err(IncidenceError::InvalidDiscountReferenceTick);
        }

        let Some(report) = self.reports.iter().find(|report| report.tick == tick) else {
            return Ok(None);
        };
        let periods = (tick - reference_tick) as f64;
        let factor = 1.0 / (1.0 + discount_rate).powf(periods);
        if !factor.is_finite() {
            return Err(IncidenceError::InvalidDiscountRate);
        }

        let rows = report
            .service_allocations
            .iter()
            .map(|row| DiscountedServiceRow {
                assignment: row.assignment.clone(),
                region_id: row.region_id.clone(),
                resource: row.resource,
                discount_factor: factor,
                discounted_demand_equivalent: row.demand * factor,
                discounted_served_equivalent: row.served * factor,
                discounted_unmet_equivalent: row.unmet() * factor,
            })
            .collect();

        Ok(Some(DiscountedServiceView {
            tick,
            reference_tick,
            discount_rate,
            rows,
        }))
    }
}

impl Default for CohortIncidenceLedger {
    fn default() -> Self {
        Self::new()
    }
}

fn ensure_region(region_ids: &BTreeSet<&str>, region: &str) -> Result<(), IncidenceError> {
    if region_ids.contains(region) {
        Ok(())
    } else {
        Err(IncidenceError::UnknownRegion(region.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscountedServiceRow {
    pub assignment: CohortAssignment,
    pub region_id: String,
    pub resource: Resource,
    pub discount_factor: f64,
    pub discounted_demand_equivalent: f64,
    pub discounted_served_equivalent: f64,
    pub discounted_unmet_equivalent: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscountedServiceView {
    pub tick: u64,
    pub reference_tick: u64,
    pub discount_rate: f64,
    pub rows: Vec<DiscountedServiceRow>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IncidenceError {
    EmptySourceReport,
    NonIncreasingTick,
    DuplicateRegionId(String),
    SourceLedgerNotConserved(String),
    InvalidText(&'static str),
    InvalidQuantity(&'static str),
    InvalidTimeRange,
    RecordCreatedAfterSourceTick,
    InvalidUncertainty,
    InvalidCurrencyCode,
    InvalidPriceBaseYear,
    InvalidCapitalCondition,
    DuplicateRecordId(String),
    ServedExceedsDemand { allocation_id: String },
    UnknownRegion(String),
    RegionalReconciliationFailed { region: String, resource: Resource },
    InvalidDiscountRate,
    InvalidDiscountReferenceTick,
}

impl fmt::Display for IncidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySourceReport => write!(f, "source step report must contain a region"),
            Self::NonIncreasingTick => write!(f, "incidence ticks must be strictly increasing"),
            Self::DuplicateRegionId(region) => write!(f, "duplicate source region id: {region}"),
            Self::SourceLedgerNotConserved(region) => {
                write!(f, "source resource ledger is not conserved for {region}")
            }
            Self::InvalidText(field) => write!(f, "{field} must not be blank"),
            Self::InvalidQuantity(field) => {
                write!(f, "{field} must be finite and nonnegative")
            }
            Self::InvalidTimeRange => write!(f, "obligation due tick precedes its creation tick"),
            Self::RecordCreatedAfterSourceTick => {
                write!(f, "record creation tick is later than the source step")
            }
            Self::InvalidUncertainty => {
                write!(f, "uncertainty interval must be finite, nonnegative and ordered")
            }
            Self::InvalidCurrencyCode => {
                write!(f, "currency code must be a three-letter uppercase code")
            }
            Self::InvalidPriceBaseYear => write!(f, "price base year must be nonzero"),
            Self::InvalidCapitalCondition => {
                write!(f, "capital condition must be finite and within [0, 1]")
            }
            Self::DuplicateRecordId(record) => write!(f, "duplicate incidence record id: {record}"),
            Self::ServedExceedsDemand { allocation_id } => {
                write!(f, "served quantity exceeds demand in allocation {allocation_id}")
            }
            Self::UnknownRegion(region) => {
                write!(f, "incidence record references unknown region: {region}")
            }
            Self::RegionalReconciliationFailed { region, resource } => {
                write!(f, "cohort allocations do not reconcile for {region}/{resource:?}")
            }
            Self::InvalidDiscountRate => {
                write!(f, "welfare discount rate must be finite and greater than -1")
            }
            Self::InvalidDiscountReferenceTick => {
                write!(f, "discount reference tick cannot be later than the report tick")
            }
        }
    }
}

impl std::error::Error for IncidenceError {}

#[cfg(test)]
mod tests {
    use super::*;
    use civdyn_resource_ledger::{
        Production, Region, ResourceVector, StepInput, TransferPolicy, World,
    };

    fn source_step() -> StepReport {
        let region = Region {
            id: "district-a".into(),
            population: 4.0,
            opening_stock: ResourceVector::new(100.0, 200.0, 80.0),
            essential_demand_per_capita: ResourceVector::new(5.0, 10.0, 4.0),
        };
        let mut world = World::new(vec![region]).expect("valid source world");
        world
            .step(
                &StepInput {
                    production: vec![Production::default()],
                    transfers: vec![],
                },
                TransferPolicy::Unrestricted,
            )
            .expect("valid source step")
    }

    fn service_rows(
        first: CohortAssignment,
        second: CohortAssignment,
    ) -> Vec<ServiceAllocation> {
        let source_ref = "ledger:tick-1:district-a";
        let action = "policy:baseline";
        vec![
            ServiceAllocation {
                allocation_id: "service-water-a".into(),
                assignment: first.clone(),
                region_id: "district-a".into(),
                resource: Resource::Water,
                demand: 5.0,
                served: 5.0,
                originating_action_id: action.into(),
                source_evidence_ref: source_ref.into(),
            },
            ServiceAllocation {
                allocation_id: "service-water-b".into(),
                assignment: second.clone(),
                region_id: "district-a".into(),
                resource: Resource::Water,
                demand: 15.0,
                served: 15.0,
                originating_action_id: action.into(),
                source_evidence_ref: source_ref.into(),
            },
            ServiceAllocation {
                allocation_id: "service-energy-a".into(),
                assignment: first.clone(),
                region_id: "district-a".into(),
                resource: Resource::Energy,
                demand: 10.0,
                served: 10.0,
                originating_action_id: action.into(),
                source_evidence_ref: source_ref.into(),
            },
            ServiceAllocation {
                allocation_id: "service-energy-b".into(),
                assignment: second.clone(),
                region_id: "district-a".into(),
                resource: Resource::Energy,
                demand: 30.0,
                served: 30.0,
                originating_action_id: action.into(),
                source_evidence_ref: source_ref.into(),
            },
            ServiceAllocation {
                allocation_id: "service-food-a".into(),
                assignment: first,
                region_id: "district-a".into(),
                resource: Resource::Food,
                demand: 4.0,
                served: 4.0,
                originating_action_id: action.into(),
                source_evidence_ref: source_ref.into(),
            },
            ServiceAllocation {
                allocation_id: "service-food-b".into(),
                assignment: second,
                region_id: "district-a".into(),
                resource: Resource::Food,
                demand: 12.0,
                served: 12.0,
                originating_action_id: action.into(),
                source_evidence_ref: source_ref.into(),
            },
        ]
    }

    fn stock_rows(
        future: CohortAssignment,
        unassigned: CohortAssignment,
    ) -> Vec<InheritedStockAllocation> {
        let source_ref = "ledger:tick-1:district-a";
        vec![
            InheritedStockAllocation {
                allocation_id: "stock-water-future".into(),
                assignment: future.clone(),
                region_id: "district-a".into(),
                asset_id: "reservoir-a".into(),
                resource: Resource::Water,
                closing_stock: 40.0,
                source_evidence_ref: source_ref.into(),
            },
            InheritedStockAllocation {
                allocation_id: "stock-water-shared".into(),
                assignment: unassigned.clone(),
                region_id: "district-a".into(),
                asset_id: "reservoir-a".into(),
                resource: Resource::Water,
                closing_stock: 40.0,
                source_evidence_ref: source_ref.into(),
            },
            InheritedStockAllocation {
                allocation_id: "stock-energy-future".into(),
                assignment: future.clone(),
                region_id: "district-a".into(),
                asset_id: "grid-storage-a".into(),
                resource: Resource::Energy,
                closing_stock: 80.0,
                source_evidence_ref: source_ref.into(),
            },
            InheritedStockAllocation {
                allocation_id: "stock-energy-shared".into(),
                assignment: unassigned.clone(),
                region_id: "district-a".into(),
                asset_id: "grid-storage-a".into(),
                resource: Resource::Energy,
                closing_stock: 80.0,
                source_evidence_ref: source_ref.into(),
            },
            InheritedStockAllocation {
                allocation_id: "stock-food-future".into(),
                assignment: future.clone(),
                region_id: "district-a".into(),
                asset_id: "food-reserve-a".into(),
                resource: Resource::Food,
                closing_stock: 32.0,
                source_evidence_ref: source_ref.into(),
            },
            InheritedStockAllocation {
                allocation_id: "stock-food-shared".into(),
                assignment: unassigned,
                region_id: "district-a".into(),
                asset_id: "food-reserve-a".into(),
                resource: Resource::Food,
                closing_stock: 32.0,
                source_evidence_ref: source_ref.into(),
            },
        ]
    }

    fn assembled(
        service: Vec<ServiceAllocation>,
        stock: Vec<InheritedStockAllocation>,
    ) -> CohortIncidenceLedger {
        let mut ledger = CohortIncidenceLedger::new();
        ledger
            .assemble_tick(&source_step(), service, stock, vec![], vec![], vec![])
            .expect("cohort allocations reconcile with the resource ledger");
        ledger
    }

    #[test]
    fn cohort_incidence_reconciles_every_native_resource_to_source_ledger() {
        let present = CohortAssignment::assigned("working-age-2026").unwrap();
        let young = CohortAssignment::assigned("future-born-2040").unwrap();
        let unknown = CohortAssignment::unassigned(UnassignedReason::SharedPublicAsset);
        let ledger = assembled(service_rows(present, young.clone()), stock_rows(young, unknown));

        let report = ledger.last().unwrap();
        assert_eq!(report.regional_reconciliations.len(), 3);
        assert!(report.regional_reconciliations.iter().all(
            RegionalResourceReconciliation::is_reconciled
        ));
    }

    #[test]
    fn future_cohort_inherits_the_preceding_closing_state_not_a_reset_baseline() {
        let present = CohortAssignment::assigned("working-age-2026").unwrap();
        let future = CohortAssignment::assigned("future-born-2040").unwrap();
        let unknown = CohortAssignment::unassigned(UnassignedReason::SharedPublicAsset);
        let ledger = assembled(service_rows(present, future.clone()), stock_rows(future.clone(), unknown));
        let summary = ledger
            .cohort_summaries(1)
            .unwrap()
            .into_iter()
            .find(|row| row.assignment == future)
            .unwrap();

        assert_eq!(summary.inherited_stock.water_m3, 40.0);
        assert_eq!(summary.inherited_stock.energy_kwh, 80.0);
        assert_eq!(summary.inherited_stock.food_kg, 32.0);
    }

    #[test]
    fn unassigned_incidence_is_visible_and_never_silently_prorated() {
        let present = CohortAssignment::assigned("working-age-2026").unwrap();
        let future = CohortAssignment::assigned("future-born-2040").unwrap();
        let unknown = CohortAssignment::unassigned(UnassignedReason::SharedPublicAsset);
        let ledger = assembled(service_rows(present, future.clone()), stock_rows(future, unknown));
        assert_eq!(ledger.unassigned_record_count(1), Some(3));

        let summary = ledger
            .cohort_summaries(1)
            .unwrap()
            .into_iter()
            .find(|row| row.assignment == CohortAssignment::unassigned(UnassignedReason::SharedPublicAsset))
            .unwrap();
        assert_eq!(summary.inherited_stock.water_m3, 40.0);
    }

    #[test]
    fn cohort_split_invariance_preserves_aggregate_physical_service() {
        let single = CohortAssignment::assigned("working-age-2026").unwrap();
        let future = CohortAssignment::assigned("future-born-2040").unwrap();
        let shared = CohortAssignment::unassigned(UnassignedReason::SharedPublicAsset);
        let original = assembled(service_rows(single.clone(), future.clone()), stock_rows(future.clone(), shared.clone()));

        let split_a = CohortAssignment::assigned("working-age-a").unwrap();
        let split_b = CohortAssignment::assigned("working-age-b").unwrap();
        let mut split_rows = service_rows(split_a.clone(), future.clone());
        for (resource, demand, served, id) in [
            (Resource::Water, 5.0, 5.0, "service-water-split"),
            (Resource::Energy, 10.0, 10.0, "service-energy-split"),
            (Resource::Food, 4.0, 4.0, "service-food-split"),
        ] {
            let original = split_rows
                .iter_mut()
                .find(|row| row.resource == resource && row.assignment == split_a)
                .expect("first subcohort allocation exists");
            original.demand = demand * 0.4;
            original.served = served * 0.4;
            split_rows.push(ServiceAllocation {
                allocation_id: id.into(),
                assignment: split_b.clone(),
                region_id: "district-a".into(),
                resource,
                demand: demand * 0.6,
                served: served * 0.6,
                originating_action_id: "policy:baseline".into(),
                source_evidence_ref: "ledger:tick-1:district-a".into(),
            });
        }
        let split = assembled(split_rows, stock_rows(future, shared));

        let original_totals: Vec<_> = original.last().unwrap().regional_reconciliations.clone();
        let split_totals: Vec<_> = split.last().unwrap().regional_reconciliations.clone();
        assert_eq!(original_totals, split_totals);
    }

    #[test]
    fn cohort_reassignment_changes_incidence_not_physical_totals() {
        let a = CohortAssignment::assigned("working-age-2026").unwrap();
        let b = CohortAssignment::assigned("older-2026").unwrap();
        let future = CohortAssignment::assigned("future-born-2040").unwrap();
        let shared = CohortAssignment::unassigned(UnassignedReason::SharedPublicAsset);

        let before = assembled(service_rows(a, future.clone()), stock_rows(future.clone(), shared.clone()));
        let after = assembled(service_rows(b, future.clone()), stock_rows(future, shared));

        assert_eq!(
            before.last().unwrap().regional_reconciliations,
            after.last().unwrap().regional_reconciliations
        );
        assert_ne!(
            before.cohort_summaries(1).unwrap(),
            after.cohort_summaries(1).unwrap()
        );
    }

    #[test]
    fn discount_rate_changes_normative_view_but_not_physical_accounts() {
        let a = CohortAssignment::assigned("working-age-2026").unwrap();
        let future = CohortAssignment::assigned("future-born-2040").unwrap();
        let shared = CohortAssignment::unassigned(UnassignedReason::SharedPublicAsset);
        let ledger = assembled(service_rows(a, future.clone()), stock_rows(future, shared));
        let physical_before = ledger.last().unwrap().clone();

        let undiscounted = ledger.discounted_service_view(1, 0.0, 0).unwrap().unwrap();
        let discounted = ledger.discounted_service_view(1, 0.10, 0).unwrap().unwrap();

        assert_ne!(
            undiscounted.rows[0].discount_factor,
            discounted.rows[0].discount_factor
        );
        assert_eq!(ledger.last().unwrap(), &physical_before);
        assert_eq!(physical_before.regional_reconciliations.len(), 3);
    }

    #[test]
    fn physical_and_financial_obligations_remain_separate_and_source_pinned() {
        let present = CohortAssignment::assigned("working-age-2026").unwrap();
        let future = CohortAssignment::assigned("future-born-2040").unwrap();
        let shared = CohortAssignment::unassigned(UnassignedReason::SharedPublicAsset);
        let mut ledger = CohortIncidenceLedger::new();
        ledger
            .assemble_tick(
                &source_step(),
                service_rows(present, future.clone()),
                stock_rows(future.clone(), shared),
                vec![PhysicalObligation {
                    obligation_id: "restoration-water-1".into(),
                    originating_action_id: "policy:defer-restoration".into(),
                    created_tick: 1,
                    due_tick: 3,
                    region_id: "district-a".into(),
                    assignment: future.clone(),
                    measure_id: "restoration-water".into(),
                    unit: "m3".into(),
                    quantity: 12.0,
                    uncertainty: QuantityUncertainty::Interval { lower: 10.0, upper: 15.0 },
                    source_evidence_ref: "policy-receipt:1".into(),
                }],
                vec![FinancialObligation {
                    obligation_id: "debt-principal-1".into(),
                    originating_action_id: "policy:debt-financed-asset".into(),
                    created_tick: 1,
                    due_tick: 8,
                    liable_cohort: future,
                    recipient_cohort: Some(CohortAssignment::assigned("working-age-2026").unwrap()),
                    amount: 500.0,
                    currency_code: "USD".into(),
                    price_base_year: 2026,
                    kind: FinancialObligationKind::Principal,
                    uncertainty: QuantityUncertainty::Known,
                    source_evidence_ref: "financial-receipt:1".into(),
                }],
                vec![],
            )
            .unwrap();

        let report = ledger.last().unwrap();
        assert_eq!(report.physical_obligations[0].quantity, 12.0);
        assert_eq!(report.physical_obligations[0].unit, "m3");
        assert_eq!(report.financial_obligations[0].amount, 500.0);
        assert_eq!(report.financial_obligations[0].currency_code, "USD");
        assert_eq!(report.regional_reconciliations.len(), 3);
    }

    #[test]
    fn missing_or_non_reconciling_service_incidence_fails_closed() {
        let a = CohortAssignment::assigned("working-age-2026").unwrap();
        let b = CohortAssignment::assigned("future-born-2040").unwrap();
        let shared = CohortAssignment::unassigned(UnassignedReason::SharedPublicAsset);
        let mut service = service_rows(a, b.clone());
        service.pop();

        let mut ledger = CohortIncidenceLedger::new();
        let result = ledger.assemble_tick(
            &source_step(),
            service,
            stock_rows(b, shared),
            vec![],
            vec![],
            vec![],
        );
        assert!(matches!(
            result,
            Err(IncidenceError::RegionalReconciliationFailed { .. })
        ));
        assert!(ledger.reports().is_empty());
    }

    #[test]
    fn invalid_future_obligation_or_duplicate_record_id_is_rejected_atomically() {
        let a = CohortAssignment::assigned("working-age-2026").unwrap();
        let b = CohortAssignment::assigned("future-born-2040").unwrap();
        let shared = CohortAssignment::unassigned(UnassignedReason::SharedPublicAsset);
        let invalid = PhysicalObligation {
            obligation_id: "bad-time".into(),
            originating_action_id: "policy:bad".into(),
            created_tick: 2,
            due_tick: 1,
            region_id: "district-a".into(),
            assignment: b.clone(),
            measure_id: "water".into(),
            unit: "m3".into(),
            quantity: 1.0,
            uncertainty: QuantityUncertainty::Known,
            source_evidence_ref: "receipt:1".into(),
        };

        let mut ledger = CohortIncidenceLedger::new();
        assert_eq!(
            ledger.assemble_tick(
                &source_step(),
                service_rows(a, b.clone()),
                stock_rows(b, shared),
                vec![invalid],
                vec![],
                vec![],
            ),
            Err(IncidenceError::InvalidTimeRange)
        );
        assert!(ledger.reports().is_empty());
    }
}
