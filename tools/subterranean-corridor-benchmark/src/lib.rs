#![forbid(unsafe_code)]

//! Small, dependency-free reference kernel for comparing corridor investments.
//!
//! This is a planning/accounting contract, not a route optimizer, geotechnical
//! solver, thermal solver, engineering design tool, or construction approval.

const EPS: f64 = 1.0e-9;
pub const BENCHMARK_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateKind {
    ExistingNetwork,
    SurfaceRailUpgrade,
    ReuseUnderground,
    NewFreightTunnel,
    SharedUtilityTunnel,
    HighSpeedRail,
    Maglev,
}

impl CandidateKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::ExistingNetwork => "existing_network",
            Self::SurfaceRailUpgrade => "surface_rail_upgrade",
            Self::ReuseUnderground => "reuse_underground",
            Self::NewFreightTunnel => "new_freight_tunnel",
            Self::SharedUtilityTunnel => "shared_utility_tunnel",
            Self::HighSpeedRail => "high_speed_rail",
            Self::Maglev => "maglev",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceBasis {
    FreightTonne,
    PassengerTrip,
}

impl ServiceBasis {
    pub fn label(self) -> &'static str {
        match self {
            Self::FreightTonne => "freight_tonne",
            Self::PassengerTrip => "passenger_trip",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceClass {
    Synthetic,
    Sourced,
    Measured,
    IndependentlyReviewed,
}

impl EvidenceClass {
    pub fn label(self) -> &'static str {
        match self {
            Self::Synthetic => "synthetic",
            Self::Sourced => "sourced",
            Self::Measured => "measured",
            Self::IndependentlyReviewed => "independently_reviewed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateState {
    Satisfied,
    Failed,
    Unknown,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardGates {
    pub geotechnical: GateState,
    pub hydrogeological: GateState,
    pub seismic_hazard_characterization: GateState,
    pub active_fault_displacement: GateState,
    pub liquefaction_lateral_spread_and_slope: GateState,
    pub seismic_lining_and_interface_design: GateState,
    pub portal_shaft_and_station_interfaces: GateState,
    pub post_event_inspection_and_recovery: GateState,
    pub rights_and_permits: GateState,
    pub environmental: GateState,
    pub emergency_egress: GateState,
    pub service_compatibility: GateState,
}

impl HardGates {
    fn states(self) -> [(&'static str, GateState); 12] {
        [
            ("geotechnical", self.geotechnical),
            ("hydrogeological", self.hydrogeological),
            (
                "seismic_hazard_characterization",
                self.seismic_hazard_characterization,
            ),
            ("active_fault_displacement", self.active_fault_displacement),
            (
                "liquefaction_lateral_spread_and_slope",
                self.liquefaction_lateral_spread_and_slope,
            ),
            (
                "seismic_lining_and_interface_design",
                self.seismic_lining_and_interface_design,
            ),
            (
                "portal_shaft_and_station_interfaces",
                self.portal_shaft_and_station_interfaces,
            ),
            (
                "post_event_inspection_and_recovery",
                self.post_event_inspection_and_recovery,
            ),
            ("rights_and_permits", self.rights_and_permits),
            ("environmental", self.environmental),
            ("emergency_egress", self.emergency_egress),
            ("service_compatibility", self.service_compatibility),
        ]
    }

    pub fn all_satisfied() -> Self {
        Self {
            geotechnical: GateState::Satisfied,
            hydrogeological: GateState::Satisfied,
            seismic_hazard_characterization: GateState::Satisfied,
            active_fault_displacement: GateState::Satisfied,
            liquefaction_lateral_spread_and_slope: GateState::Satisfied,
            seismic_lining_and_interface_design: GateState::Satisfied,
            portal_shaft_and_station_interfaces: GateState::Satisfied,
            post_event_inspection_and_recovery: GateState::Satisfied,
            rights_and_permits: GateState::Satisfied,
            environmental: GateState::Satisfied,
            emergency_egress: GateState::Satisfied,
            service_compatibility: GateState::Satisfied,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThermalOpportunity {
    /// Upstream model's annual recoverable heat estimate, in MWh/year.
    pub recoverable_mwh_per_year: f64,
    /// Local heat demand that can accept the declared temperature grade, MWh/year.
    pub compatible_demand_mwh_per_year: f64,
    /// Declared end-to-end delivery fraction, in [0, 1].
    pub delivery_fraction: f64,
    /// Must be Satisfied before any useful-heat quantity is admitted.
    pub source_sink_gate: GateState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub candidate_id: String,
    /// Same service bundle, origin/destination, demand class and scenario family.
    pub comparison_basis_id: String,
    /// Same real-price convention and cost boundary across candidates.
    pub cost_profile_id: String,
    /// Same monetized externality method across candidates.
    pub externality_profile_id: String,
    pub kind: CandidateKind,
    pub service_basis: ServiceBasis,
    /// This is an input claim, not a source verification performed by this crate.
    pub evidence_class: EvidenceClass,
    pub source_refs: Vec<String>,
    pub price_year: u16,
    pub horizon_years: u32,
    /// Real annual discount rate as a decimal; 0.05 means 5%.
    pub discount_rate: f64,
    pub annual_demand_units: f64,
    pub annual_capacity_units: f64,
    /// Fraction of nominal capacity available over the assessment year, in [0, 1].
    pub service_availability_fraction: f64,
    pub route_distance_km: f64,
    /// Door-to-door hours, including access, waits, transfers and handling.
    pub door_to_door_hours: f64,
    pub civil_capex_usd: f64,
    pub terminal_capex_usd: f64,
    pub fleet_capex_usd: f64,
    pub utility_capex_usd: f64,
    pub annual_fixed_opex_usd: f64,
    pub annual_maintenance_usd: f64,
    /// Variable cost per service unit-km; scope must include stated empty returns.
    pub variable_cost_usd_per_unit_km: f64,
    /// Access, first/last-mile, handling and transfer cost per service unit.
    pub access_transfer_cost_usd_per_unit: f64,
    /// Annual external costs, separate from operator costs and user charges.
    pub annual_external_cost_usd: f64,
    /// Whole-chain intensities per service unit-km; include terminals/empty returns
    /// within the declared boundary or disclose their omission in the cost profile.
    pub energy_kwh_per_unit_km: f64,
    pub emissions_kg_co2e_per_unit_km: f64,
    /// One-time embodied energy for construction/refurbishment, kWh.
    pub capital_embodied_energy_kwh: f64,
    /// One-time embodied greenhouse-gas emissions for construction/refurbishment, kg CO2e.
    pub capital_embodied_emissions_kg_co2e: f64,
    pub decommissioning_cost_usd: f64,
    pub residual_value_usd: f64,
    pub gates: HardGates,
    /// Optional side ledger. V0.1 never subtracts heat value from transport cost.
    pub thermal_opportunity: Option<ThermalOpportunity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    InvalidInput,
    Ineligible,
    NeedsEvidence,
    CapacityInsufficient,
    ComparableSynthetic,
    ComparableDeclaredEvidence,
}

impl Disposition {
    pub fn label(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_INPUT",
            Self::Ineligible => "INELIGIBLE",
            Self::NeedsEvidence => "NEEDS_EVIDENCE",
            Self::CapacityInsufficient => "CAPACITY_INSUFFICIENT",
            Self::ComparableSynthetic => "COMPARABLE_SYNTHETIC_ONLY",
            Self::ComparableDeclaredEvidence => "COMPARABLE_DECLARED_EVIDENCE",
        }
    }

    fn is_comparable(self) -> bool {
        matches!(
            self,
            Self::ComparableSynthetic | Self::ComparableDeclaredEvidence
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThermalMetrics {
    pub matched_potential_mwh_per_year: f64,
    /// None means the supplied source/sink gate did not admit this quantity.
    /// Even Some(...) is not field validation when the candidate is synthetic.
    pub gate_satisfied_useful_mwh_per_year: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Metrics {
    pub candidate_id: String,
    pub comparison_basis_id: String,
    pub cost_profile_id: String,
    pub externality_profile_id: String,
    pub kind: CandidateKind,
    pub service_basis: ServiceBasis,
    pub evidence_class: EvidenceClass,
    pub price_year: u16,
    pub horizon_years: u32,
    pub discount_rate: f64,
    pub annual_demand_units: f64,
    pub total_capex_usd: f64,
    pub private_lifecycle_npv_usd: f64,
    pub externality_lifecycle_npv_usd: f64,
    pub social_lifecycle_npv_usd: f64,
    pub discounted_service_units: f64,
    pub private_cost_per_service_unit_usd: f64,
    pub social_cost_per_service_unit_usd: f64,
    pub effective_capacity_units_per_year: f64,
    pub capacity_utilization_fraction: f64,
    pub door_to_door_hours: f64,
    pub service_availability_fraction: f64,
    pub energy_kwh_per_service_unit: f64,
    pub emissions_kg_co2e_per_service_unit: f64,
    pub total_lifecycle_energy_kwh: f64,
    pub total_lifecycle_emissions_kg_co2e: f64,
    pub thermal: Option<ThermalMetrics>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation {
    pub candidate_id: String,
    pub disposition: Disposition,
    pub reasons: Vec<String>,
    pub metrics: Option<Metrics>,
}

impl Evaluation {
    pub fn is_comparable(&self) -> bool {
        self.disposition.is_comparable() && self.metrics.is_some()
    }
}

fn invalid(candidate_id: &str, reason: impl Into<String>) -> Evaluation {
    Evaluation {
        candidate_id: candidate_id.to_owned(),
        disposition: Disposition::InvalidInput,
        reasons: vec![reason.into()],
        metrics: None,
    }
}

fn finite_nonnegative(name: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() {
        return Err(format!("NON_FINITE:{name}"));
    }
    if value < 0.0 {
        return Err(format!("NEGATIVE:{name}"));
    }
    Ok(())
}

fn finite_positive(name: &str, value: f64) -> Result<(), String> {
    finite_nonnegative(name, value)?;
    if value <= 0.0 {
        return Err(format!("NOT_POSITIVE:{name}"));
    }
    Ok(())
}

fn validate(candidate: &Candidate) -> Result<(), String> {
    for (name, value) in [
        ("discount_rate", candidate.discount_rate),
        ("annual_demand_units", candidate.annual_demand_units),
        ("annual_capacity_units", candidate.annual_capacity_units),
        (
            "service_availability_fraction",
            candidate.service_availability_fraction,
        ),
        ("route_distance_km", candidate.route_distance_km),
        ("door_to_door_hours", candidate.door_to_door_hours),
        ("civil_capex_usd", candidate.civil_capex_usd),
        ("terminal_capex_usd", candidate.terminal_capex_usd),
        ("fleet_capex_usd", candidate.fleet_capex_usd),
        ("utility_capex_usd", candidate.utility_capex_usd),
        ("annual_fixed_opex_usd", candidate.annual_fixed_opex_usd),
        ("annual_maintenance_usd", candidate.annual_maintenance_usd),
        (
            "variable_cost_usd_per_unit_km",
            candidate.variable_cost_usd_per_unit_km,
        ),
        (
            "access_transfer_cost_usd_per_unit",
            candidate.access_transfer_cost_usd_per_unit,
        ),
        (
            "annual_external_cost_usd",
            candidate.annual_external_cost_usd,
        ),
        ("energy_kwh_per_unit_km", candidate.energy_kwh_per_unit_km),
        (
            "emissions_kg_co2e_per_unit_km",
            candidate.emissions_kg_co2e_per_unit_km,
        ),
        (
            "capital_embodied_energy_kwh",
            candidate.capital_embodied_energy_kwh,
        ),
        (
            "capital_embodied_emissions_kg_co2e",
            candidate.capital_embodied_emissions_kg_co2e,
        ),
        (
            "decommissioning_cost_usd",
            candidate.decommissioning_cost_usd,
        ),
        ("residual_value_usd", candidate.residual_value_usd),
    ] {
        finite_nonnegative(name, value)?;
    }

    if candidate.candidate_id.trim().is_empty()
        || candidate.comparison_basis_id.trim().is_empty()
        || candidate.cost_profile_id.trim().is_empty()
        || candidate.externality_profile_id.trim().is_empty()
    {
        return Err("EMPTY_ID_OR_PROFILE".to_owned());
    }
    if candidate.price_year < 1900 {
        return Err("INVALID_PRICE_YEAR".to_owned());
    }
    if candidate.horizon_years == 0 || candidate.horizon_years > 200 {
        return Err("HORIZON_OUT_OF_RANGE".to_owned());
    }
    if candidate.discount_rate >= 1.0 {
        return Err("DISCOUNT_RATE_OUT_OF_RANGE".to_owned());
    }
    if candidate.annual_demand_units <= 0.0
        || candidate.annual_capacity_units <= 0.0
        || candidate.route_distance_km <= 0.0
        || candidate.door_to_door_hours <= 0.0
    {
        return Err("DEMAND_CAPACITY_DISTANCE_AND_TIME_MUST_BE_POSITIVE".to_owned());
    }
    if candidate.service_availability_fraction > 1.0 {
        return Err("AVAILABILITY_OUT_OF_RANGE".to_owned());
    }

    if candidate.evidence_class != EvidenceClass::Synthetic && candidate.source_refs.is_empty() {
        return Err("DECLARED_EVIDENCE_REQUIRES_SOURCE_REFERENCES".to_owned());
    }
    let mut refs = std::collections::BTreeSet::new();
    for source_ref in &candidate.source_refs {
        if source_ref.trim().is_empty() || !refs.insert(source_ref) {
            return Err("EMPTY_OR_DUPLICATE_SOURCE_REFERENCE".to_owned());
        }
    }

    if let Some(thermal) = &candidate.thermal_opportunity {
        finite_nonnegative(
            "thermal.recoverable_mwh_per_year",
            thermal.recoverable_mwh_per_year,
        )?;
        finite_nonnegative(
            "thermal.compatible_demand_mwh_per_year",
            thermal.compatible_demand_mwh_per_year,
        )?;
        finite_nonnegative("thermal.delivery_fraction", thermal.delivery_fraction)?;
        if thermal.delivery_fraction > 1.0 {
            return Err("THERMAL_DELIVERY_FRACTION_OUT_OF_RANGE".to_owned());
        }
    }
    Ok(())
}

fn pv_annuity_factor(rate: f64, years: u32) -> f64 {
    if rate.abs() <= EPS {
        return years as f64;
    }
    (1.0 - (1.0 + rate).powi(-(years as i32))) / rate
}

fn discount_factor(rate: f64, year: u32) -> f64 {
    (1.0 + rate).powi(-(year as i32))
}

fn finite_metrics(metrics: &Metrics) -> bool {
    [
        metrics.total_capex_usd,
        metrics.private_lifecycle_npv_usd,
        metrics.externality_lifecycle_npv_usd,
        metrics.social_lifecycle_npv_usd,
        metrics.discounted_service_units,
        metrics.private_cost_per_service_unit_usd,
        metrics.social_cost_per_service_unit_usd,
        metrics.effective_capacity_units_per_year,
        metrics.capacity_utilization_fraction,
        metrics.energy_kwh_per_service_unit,
        metrics.emissions_kg_co2e_per_service_unit,
        metrics.total_lifecycle_energy_kwh,
        metrics.total_lifecycle_emissions_kg_co2e,
    ]
    .into_iter()
    .all(f64::is_finite)
        && metrics.thermal.as_ref().is_none_or(|thermal| {
            thermal.matched_potential_mwh_per_year.is_finite()
                && thermal
                    .gate_satisfied_useful_mwh_per_year
                    .is_none_or(f64::is_finite)
        })
}

/// Evaluate an alternative against its declared gates and common service demand.
///
/// Cost, energy, emissions, source references and gate states are supplied inputs.
/// This function checks arithmetic and contract consistency; it does not verify
/// that source documents are authentic or that engineering claims are true.
pub fn evaluate(candidate: &Candidate) -> Evaluation {
    if let Err(reason) = validate(candidate) {
        return invalid(&candidate.candidate_id, reason);
    }

    let mut failed = Vec::new();
    let mut unresolved = Vec::new();
    for (name, state) in candidate.gates.states() {
        match state {
            GateState::Satisfied => {}
            GateState::Failed => failed.push(format!("GATE_FAILED:{name}")),
            GateState::Unknown | GateState::NotApplicable => {
                unresolved.push(format!("NEEDS_EVIDENCE:{name}"));
            }
        }
    }
    if !failed.is_empty() {
        failed.extend(unresolved);
        return Evaluation {
            candidate_id: candidate.candidate_id.clone(),
            disposition: Disposition::Ineligible,
            reasons: failed,
            metrics: None,
        };
    }
    if !unresolved.is_empty() {
        return Evaluation {
            candidate_id: candidate.candidate_id.clone(),
            disposition: Disposition::NeedsEvidence,
            reasons: unresolved,
            metrics: None,
        };
    }

    let effective_capacity =
        candidate.annual_capacity_units * candidate.service_availability_fraction;
    if !effective_capacity.is_finite() {
        return invalid(
            &candidate.candidate_id,
            "ARITHMETIC_OVERFLOW:EFFECTIVE_CAPACITY",
        );
    }
    if effective_capacity + EPS < candidate.annual_demand_units {
        return Evaluation {
            candidate_id: candidate.candidate_id.clone(),
            disposition: Disposition::CapacityInsufficient,
            reasons: vec!["CAPACITY_SHORTFALL_AFTER_AVAILABILITY_ADJUSTMENT".to_owned()],
            metrics: None,
        };
    }

    let years = candidate.horizon_years;
    let rate = candidate.discount_rate;
    let annuity_factor = pv_annuity_factor(rate, years);
    let terminal_factor = discount_factor(rate, years);
    let total_capex = candidate.civil_capex_usd
        + candidate.terminal_capex_usd
        + candidate.fleet_capex_usd
        + candidate.utility_capex_usd;
    let annual_variable_cost = candidate.annual_demand_units
        * (candidate.route_distance_km * candidate.variable_cost_usd_per_unit_km
            + candidate.access_transfer_cost_usd_per_unit);
    let annual_private_cost =
        candidate.annual_fixed_opex_usd + candidate.annual_maintenance_usd + annual_variable_cost;
    let private_npv = total_capex
        + annual_private_cost * annuity_factor
        + candidate.decommissioning_cost_usd * terminal_factor
        - candidate.residual_value_usd * terminal_factor;
    let externality_npv = candidate.annual_external_cost_usd * annuity_factor;
    let social_npv = private_npv + externality_npv;
    let discounted_service_units = candidate.annual_demand_units * annuity_factor;
    let physical_lifetime_service_units = candidate.annual_demand_units * years as f64;
    let total_lifecycle_energy_kwh = candidate.capital_embodied_energy_kwh
        + candidate.annual_demand_units
            * candidate.route_distance_km
            * candidate.energy_kwh_per_unit_km
            * years as f64;
    let total_lifecycle_emissions_kg_co2e = candidate.capital_embodied_emissions_kg_co2e
        + candidate.annual_demand_units
            * candidate.route_distance_km
            * candidate.emissions_kg_co2e_per_unit_km
            * years as f64;
    let thermal = candidate.thermal_opportunity.as_ref().map(|heat| {
        let matched = heat
            .recoverable_mwh_per_year
            .min(heat.compatible_demand_mwh_per_year)
            * heat.delivery_fraction;
        let admitted = (heat.source_sink_gate == GateState::Satisfied).then_some(matched);
        ThermalMetrics {
            matched_potential_mwh_per_year: matched,
            gate_satisfied_useful_mwh_per_year: admitted,
        }
    });
    let metrics = Metrics {
        candidate_id: candidate.candidate_id.clone(),
        comparison_basis_id: candidate.comparison_basis_id.clone(),
        cost_profile_id: candidate.cost_profile_id.clone(),
        externality_profile_id: candidate.externality_profile_id.clone(),
        kind: candidate.kind,
        service_basis: candidate.service_basis,
        evidence_class: candidate.evidence_class,
        price_year: candidate.price_year,
        horizon_years: candidate.horizon_years,
        discount_rate: rate,
        annual_demand_units: candidate.annual_demand_units,
        total_capex_usd: total_capex,
        private_lifecycle_npv_usd: private_npv,
        externality_lifecycle_npv_usd: externality_npv,
        social_lifecycle_npv_usd: social_npv,
        discounted_service_units,
        private_cost_per_service_unit_usd: private_npv / discounted_service_units,
        social_cost_per_service_unit_usd: social_npv / discounted_service_units,
        effective_capacity_units_per_year: effective_capacity,
        capacity_utilization_fraction: candidate.annual_demand_units / effective_capacity,
        door_to_door_hours: candidate.door_to_door_hours,
        service_availability_fraction: candidate.service_availability_fraction,
        energy_kwh_per_service_unit: total_lifecycle_energy_kwh / physical_lifetime_service_units,
        emissions_kg_co2e_per_service_unit: total_lifecycle_emissions_kg_co2e
            / physical_lifetime_service_units,
        total_lifecycle_energy_kwh,
        total_lifecycle_emissions_kg_co2e,
        thermal,
    };
    if !finite_metrics(&metrics) {
        return invalid(&candidate.candidate_id, "ARITHMETIC_OVERFLOW:METRICS");
    }

    let disposition = match candidate.evidence_class {
        EvidenceClass::Synthetic => Disposition::ComparableSynthetic,
        EvidenceClass::Sourced | EvidenceClass::Measured | EvidenceClass::IndependentlyReviewed => {
            Disposition::ComparableDeclaredEvidence
        }
    };
    Evaluation {
        candidate_id: candidate.candidate_id.clone(),
        disposition,
        reasons: Vec::new(),
        metrics: Some(metrics),
    }
}

fn same_comparison_contract(a: &Metrics, b: &Metrics) -> bool {
    a.comparison_basis_id == b.comparison_basis_id
        && a.cost_profile_id == b.cost_profile_id
        && a.externality_profile_id == b.externality_profile_id
        && a.service_basis == b.service_basis
        && a.evidence_class == b.evidence_class
        && a.price_year == b.price_year
        && a.horizon_years == b.horizon_years
        && (a.discount_rate - b.discount_rate).abs() <= EPS
        && (a.annual_demand_units - b.annual_demand_units).abs() <= EPS
}

fn dominates_metrics(a: &Metrics, b: &Metrics) -> bool {
    if !same_comparison_contract(a, b) {
        return false;
    }
    let no_worse = a.social_cost_per_service_unit_usd <= b.social_cost_per_service_unit_usd + EPS
        && a.door_to_door_hours <= b.door_to_door_hours + EPS
        && a.energy_kwh_per_service_unit <= b.energy_kwh_per_service_unit + EPS
        && a.emissions_kg_co2e_per_service_unit <= b.emissions_kg_co2e_per_service_unit + EPS
        && a.service_availability_fraction + EPS >= b.service_availability_fraction;
    let strictly_better = a.social_cost_per_service_unit_usd
        < b.social_cost_per_service_unit_usd - EPS
        || a.door_to_door_hours < b.door_to_door_hours - EPS
        || a.energy_kwh_per_service_unit < b.energy_kwh_per_service_unit - EPS
        || a.emissions_kg_co2e_per_service_unit < b.emissions_kg_co2e_per_service_unit - EPS
        || a.service_availability_fraction > b.service_availability_fraction + EPS;
    no_worse && strictly_better
}

/// Return non-dominated, comparable candidates. Candidates with different
/// comparison contracts remain incomparable rather than being forced into one rank.
pub fn pareto_frontier(evaluations: &[Evaluation]) -> Vec<String> {
    let mut frontier = Vec::new();
    for candidate in evaluations.iter().filter(|e| e.is_comparable()) {
        let Some(candidate_metrics) = candidate.metrics.as_ref() else {
            continue;
        };
        let dominated = evaluations.iter().any(|other| {
            if other.candidate_id == candidate.candidate_id || !other.is_comparable() {
                return false;
            }
            other
                .metrics
                .as_ref()
                .is_some_and(|other_metrics| dominates_metrics(other_metrics, candidate_metrics))
        });
        if !dominated {
            frontier.push(candidate.candidate_id.clone());
        }
    }
    frontier.sort();
    frontier
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_candidate() -> Candidate {
        Candidate {
            candidate_id: "base".to_owned(),
            comparison_basis_id: "TEST-OD-FREIGHT-V1".to_owned(),
            cost_profile_id: "REAL-USD-2026-V1".to_owned(),
            externality_profile_id: "TEST-EXTERNALITIES-V1".to_owned(),
            kind: CandidateKind::ExistingNetwork,
            service_basis: ServiceBasis::FreightTonne,
            evidence_class: EvidenceClass::Synthetic,
            source_refs: Vec::new(),
            price_year: 2026,
            horizon_years: 10,
            discount_rate: 0.0,
            annual_demand_units: 100.0,
            annual_capacity_units: 200.0,
            service_availability_fraction: 0.9,
            route_distance_km: 10.0,
            door_to_door_hours: 2.0,
            civil_capex_usd: 100.0,
            terminal_capex_usd: 0.0,
            fleet_capex_usd: 0.0,
            utility_capex_usd: 0.0,
            annual_fixed_opex_usd: 10.0,
            annual_maintenance_usd: 10.0,
            variable_cost_usd_per_unit_km: 0.01,
            access_transfer_cost_usd_per_unit: 2.0,
            annual_external_cost_usd: 5.0,
            energy_kwh_per_unit_km: 0.2,
            emissions_kg_co2e_per_unit_km: 0.03,
            capital_embodied_energy_kwh: 40_000.0,
            capital_embodied_emissions_kg_co2e: 6_000.0,
            decommissioning_cost_usd: 40.0,
            residual_value_usd: 20.0,
            gates: HardGates::all_satisfied(),
            thermal_opportunity: None,
        }
    }

    #[test]
    fn lifecycle_ledger_matches_hand_calculated_zero_discount_case() {
        let result = evaluate(&base_candidate());
        assert_eq!(result.disposition, Disposition::ComparableSynthetic);
        let metrics = result.metrics.unwrap();
        // 100 capex + 10 * (10 fixed + 10 maintenance + 210 variable) + 40 - 20.
        assert!((metrics.private_lifecycle_npv_usd - 2_420.0).abs() < EPS);
        assert!((metrics.externality_lifecycle_npv_usd - 50.0).abs() < EPS);
        assert!((metrics.social_lifecycle_npv_usd - 2_470.0).abs() < EPS);
        assert!((metrics.discounted_service_units - 1_000.0).abs() < EPS);
        assert!((metrics.private_cost_per_service_unit_usd - 2.42).abs() < EPS);
        assert!((metrics.social_cost_per_service_unit_usd - 2.47).abs() < EPS);
        assert!((metrics.total_lifecycle_energy_kwh - 42_000.0).abs() < EPS);
        assert!((metrics.total_lifecycle_emissions_kg_co2e - 6_300.0).abs() < EPS);
        assert!((metrics.energy_kwh_per_service_unit - 42.0).abs() < EPS);
        assert!((metrics.emissions_kg_co2e_per_service_unit - 6.3).abs() < EPS);
    }

    #[test]
    fn unknown_hard_gate_fails_closed_without_rankable_metrics() {
        let mut candidate = base_candidate();
        candidate.gates.hydrogeological = GateState::Unknown;
        let result = evaluate(&candidate);
        assert_eq!(result.disposition, Disposition::NeedsEvidence);
        assert!(result.metrics.is_none());
        assert!(
            result
                .reasons
                .iter()
                .any(|r| r == "NEEDS_EVIDENCE:hydrogeological")
        );
    }

    #[test]
    fn unknown_seismic_hazard_blocks_comparison() {
        let mut candidate = base_candidate();
        candidate.gates.seismic_hazard_characterization = GateState::Unknown;
        let result = evaluate(&candidate);
        assert_eq!(result.disposition, Disposition::NeedsEvidence);
        assert!(result.metrics.is_none());
        assert!(
            result
                .reasons
                .iter()
                .any(|r| r == "NEEDS_EVIDENCE:seismic_hazard_characterization")
        );
    }

    #[test]
    fn active_fault_displacement_failure_blocks_comparison() {
        let mut candidate = base_candidate();
        candidate.gates.active_fault_displacement = GateState::Failed;
        let result = evaluate(&candidate);
        assert_eq!(result.disposition, Disposition::Ineligible);
        assert!(result.metrics.is_none());
        assert!(
            result
                .reasons
                .iter()
                .any(|r| r == "GATE_FAILED:active_fault_displacement")
        );
    }

    #[test]
    fn unknown_post_event_recovery_evidence_blocks_comparison() {
        let mut candidate = base_candidate();
        candidate.gates.post_event_inspection_and_recovery = GateState::Unknown;
        let result = evaluate(&candidate);
        assert_eq!(result.disposition, Disposition::NeedsEvidence);
        assert!(result.metrics.is_none());
    }

    #[test]
    fn a_failed_gate_overrides_unknown_gates() {
        let mut candidate = base_candidate();
        candidate.gates.geotechnical = GateState::Failed;
        candidate.gates.hydrogeological = GateState::Unknown;
        let result = evaluate(&candidate);
        assert_eq!(result.disposition, Disposition::Ineligible);
        assert!(result.metrics.is_none());
        assert!(
            result
                .reasons
                .iter()
                .any(|r| r == "GATE_FAILED:geotechnical")
        );
        assert!(
            result
                .reasons
                .iter()
                .any(|r| r == "NEEDS_EVIDENCE:hydrogeological")
        );
    }

    #[test]
    fn not_applicable_on_a_required_gate_is_not_silently_accepted() {
        let mut candidate = base_candidate();
        candidate.gates.emergency_egress = GateState::NotApplicable;
        let result = evaluate(&candidate);
        assert_eq!(result.disposition, Disposition::NeedsEvidence);
    }

    #[test]
    fn effective_capacity_must_cover_the_full_service_demand() {
        let mut candidate = base_candidate();
        candidate.service_availability_fraction = 0.4;
        let result = evaluate(&candidate);
        assert_eq!(result.disposition, Disposition::CapacityInsufficient);
        assert!(result.metrics.is_none());
    }

    #[test]
    fn non_finite_or_negative_inputs_are_rejected() {
        let mut non_finite = base_candidate();
        non_finite.energy_kwh_per_unit_km = f64::NAN;
        assert_eq!(evaluate(&non_finite).disposition, Disposition::InvalidInput);

        let mut negative = base_candidate();
        negative.civil_capex_usd = -1.0;
        assert_eq!(evaluate(&negative).disposition, Disposition::InvalidInput);
    }

    #[test]
    fn declared_non_synthetic_evidence_requires_references() {
        let mut candidate = base_candidate();
        candidate.evidence_class = EvidenceClass::Measured;
        let result = evaluate(&candidate);
        assert_eq!(result.disposition, Disposition::InvalidInput);

        candidate.source_refs = vec!["field-log-001".to_owned()];
        let result = evaluate(&candidate);
        assert_eq!(result.disposition, Disposition::ComparableDeclaredEvidence);
    }

    #[test]
    fn thermal_heat_is_a_separate_gated_ledger_not_a_transport_cost_credit() {
        let mut candidate = base_candidate();
        candidate.thermal_opportunity = Some(ThermalOpportunity {
            recoverable_mwh_per_year: 5.0,
            compatible_demand_mwh_per_year: 3.0,
            delivery_fraction: 0.8,
            source_sink_gate: GateState::Satisfied,
        });
        let qualified = evaluate(&candidate);
        let metrics = qualified.metrics.unwrap();
        let thermal = metrics.thermal.unwrap();
        assert!((thermal.matched_potential_mwh_per_year - 2.4).abs() < EPS);
        assert!((thermal.gate_satisfied_useful_mwh_per_year.unwrap() - 2.4).abs() < EPS);
        assert!((metrics.private_lifecycle_npv_usd - 2_420.0).abs() < EPS);

        candidate
            .thermal_opportunity
            .as_mut()
            .unwrap()
            .source_sink_gate = GateState::Unknown;
        let unqualified = evaluate(&candidate);
        let heat = unqualified.metrics.unwrap().thermal.unwrap();
        assert!(heat.gate_satisfied_useful_mwh_per_year.is_none());
    }

    #[test]
    fn pareto_dominance_requires_same_comparison_contract() {
        let a = evaluate(&base_candidate());
        let mut better = base_candidate();
        better.candidate_id = "better".to_owned();
        better.civil_capex_usd = 50.0;
        better.door_to_door_hours = 1.5;
        better.energy_kwh_per_unit_km = 0.1;
        better.emissions_kg_co2e_per_unit_km = 0.02;
        better.service_availability_fraction = 0.95;
        better.annual_capacity_units = 200.0;
        let b = evaluate(&better);
        assert_eq!(
            pareto_frontier(&[a.clone(), b.clone()]),
            vec!["better".to_owned()]
        );

        let mut unrelated = better;
        unrelated.candidate_id = "different-basis".to_owned();
        unrelated.comparison_basis_id = "OTHER-OD".to_owned();
        let c = evaluate(&unrelated);
        assert_eq!(
            pareto_frontier(&[a, b, c]),
            vec!["better".to_owned(), "different-basis".to_owned()]
        );
    }

    #[test]
    fn discounted_results_change_when_the_rate_changes() {
        let mut candidate = base_candidate();
        candidate.discount_rate = 0.05;
        let result = evaluate(&candidate);
        let metrics = result.metrics.unwrap();
        assert!(metrics.discounted_service_units < 1_000.0);
        assert!(metrics.discounted_service_units > 700.0);
        assert!(metrics.private_lifecycle_npv_usd < 2_420.0);
    }

    #[test]
    fn no_new_build_capex_is_a_valid_baseline_when_gates_pass() {
        let mut candidate = base_candidate();
        candidate.candidate_id = "NO-BUILD-EXISTING-SERVICE".to_owned();
        candidate.civil_capex_usd = 0.0;
        candidate.terminal_capex_usd = 0.0;
        candidate.fleet_capex_usd = 0.0;
        candidate.utility_capex_usd = 0.0;
        assert_eq!(
            evaluate(&candidate).disposition,
            Disposition::ComparableSynthetic
        );
    }

    #[test]
    fn arithmetic_overflow_does_not_emit_metrics() {
        let mut candidate = base_candidate();
        candidate.annual_demand_units = f64::MAX;
        candidate.annual_capacity_units = f64::MAX;
        candidate.service_availability_fraction = 1.0;
        candidate.horizon_years = 200;
        candidate.route_distance_km = f64::MAX;
        let result = evaluate(&candidate);
        assert_eq!(result.disposition, Disposition::InvalidInput);
        assert!(result.metrics.is_none());
    }
}
