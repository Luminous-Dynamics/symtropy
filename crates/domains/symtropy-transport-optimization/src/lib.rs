// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic, unit-checked reference model for multimodal route trade-offs.
//!
//! This is a planning/research primitive, not an autonomous dispatch controller.
//! It enumerates a bounded Pareto frontier for one demand at a time. Edge
//! capacity is checked for each request, but simultaneous multi-demand capacity
//! allocation, timetables, capital planning, stochastic scenarios, and live data
//! integration are intentionally future work. Test costs are synthetic illustrations, not local
//! tariffs or operational measurements.
//!
//! Every edge metric is non-negative and expressed per demand unit, except
//! distance and elapsed time, which describe the route. Monetary fields use
//! minor units of the network's explicitly declared currency. Risk fields are
//! comparable proxies only when every edge uses the same documented risk model.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mode {
    Walk,
    Bicycle,
    Road,
    Bus,
    BusRapidTransit,
    Rail,
    Maritime,
    InlandWaterway,
    Air,
    Drone,
    Pipeline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QuantityUnit {
    Passengers,
    Tonnes,
    Teu,
    Pallets,
    Shipments,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeOperation {
    Movement { mode: Mode },
    Transfer { from: Mode, to: Mode },
}

/// A positive, auditable edge metric is deliberately integer-valued so that
/// accumulating costs never introduces floating-point ordering instability.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EdgeMetrics {
    pub distance_metres: u64,
    pub elapsed_seconds: u64,
    pub operator_cost_minor_per_unit: u64,
    pub user_cash_cost_minor_per_unit: u64,
    pub external_cost_minor_per_unit: u64,
    pub energy_wh_per_unit: u64,
    pub ghg_grams_co2e_per_unit: u64,
    pub safety_risk_micro_units_per_unit: u64,
    pub disruption_risk_micro_units_per_unit: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub operation: EdgeOperation,
    pub capacity: u64,
    pub capacity_unit: QuantityUnit,
    pub metrics: EdgeMetrics,
}

impl Edge {
    #[allow(clippy::too_many_arguments)]
    pub fn movement(
        id: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
        mode: Mode,
        capacity: u64,
        capacity_unit: QuantityUnit,
        metrics: EdgeMetrics,
    ) -> Self {
        Self {
            id: id.into(),
            from: from.into(),
            to: to.into(),
            operation: EdgeOperation::Movement { mode },
            capacity,
            capacity_unit,
            metrics,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn transfer(
        id: impl Into<String>,
        terminal: impl Into<String>,
        from_mode: Mode,
        to_mode: Mode,
        capacity: u64,
        capacity_unit: QuantityUnit,
        metrics: EdgeMetrics,
    ) -> Self {
        let terminal = terminal.into();
        Self {
            id: id.into(),
            from: terminal.clone(),
            to: terminal,
            operation: EdgeOperation::Transfer {
                from: from_mode,
                to: to_mode,
            },
            capacity,
            capacity_unit,
            metrics,
        }
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CargoModuleFamily {
    IsoSeries1Container,
    SwapBody,
    Pallet,
    RollCage,
    ReturnableCrate,
    TankContainer,
    FlatRack,
    FlexibleIntermediateBulkContainer,
    AircraftUld,
    ColdChainPod,
    Custom,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HandlingInterface {
    IsoCornerFittings,
    TopLift,
    ForkPockets,
    GrapplerRecess,
    AircraftRestraint,
    RollerDeck,
    TankCoupling,
    BulkDischarge,
    ManualHandling,
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProfileConformity {
    Verified {
        standard_id: String,
        edition: String,
        evidence_digest: String,
    },
    DeclaredUnverified,
    NonConforming,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TemperatureBand {
    pub min_millicelsius: i32,
    pub max_millicelsius: i32,
}

impl TemperatureBand {
    pub fn validate(self) -> Result<Self, CargoError> {
        if self.min_millicelsius > self.max_millicelsius {
            return Err(CargoError::InvalidTemperatureBand);
        }
        Ok(self)
    }

    fn contains_band(self, required: Self) -> bool {
        self.min_millicelsius <= required.min_millicelsius
            && self.max_millicelsius >= required.max_millicelsius
    }

    fn contains(self, observed_millicelsius: i32) -> bool {
        (self.min_millicelsius..=self.max_millicelsius).contains(&observed_millicelsius)
    }
}

/// Versioned physical cargo-unit contract. Dimensions use millimetres, mass
/// uses grams, volume uses litres, and temperatures use thousandths of °C.
/// Profile conformity is data, not a claim inferred from approximate geometry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoModuleProfile {
    pub profile_id: String,
    pub family: CargoModuleFamily,
    pub conformity: ProfileConformity,
    pub outer_dimensions_mm: [u32; 3],
    pub usable_volume_litres: u64,
    pub tare_mass_grams: u64,
    pub max_payload_mass_grams: u64,
    pub max_gross_mass_grams: u64,
    pub interfaces: BTreeSet<HandlingInterface>,
    pub permitted_modes: BTreeSet<Mode>,
    pub required_temperature_millicelsius: Option<TemperatureBand>,
    pub requires_power: bool,
    pub requires_intact_seal: bool,
}

impl CargoModuleProfile {
    pub fn validate(&self) -> Result<(), CargoError> {
        if self.profile_id.trim().is_empty() {
            return Err(CargoError::InvalidProfile("profile identity is empty"));
        }
        if self.outer_dimensions_mm.contains(&0) || self.usable_volume_litres == 0 {
            return Err(CargoError::InvalidProfile(
                "dimensions and usable volume must be positive",
            ));
        }
        if self.max_gross_mass_grams < self.tare_mass_grams
            || self
                .tare_mass_grams
                .checked_add(self.max_payload_mass_grams)
                .is_none_or(|total| total > self.max_gross_mass_grams)
        {
            return Err(CargoError::InvalidProfile(
                "tare plus rated payload exceeds maximum gross mass",
            ));
        }
        if self.interfaces.is_empty() || self.permitted_modes.is_empty() {
            return Err(CargoError::InvalidProfile(
                "at least one handling interface and permitted mode are required",
            ));
        }
        if let Some(band) = self.required_temperature_millicelsius {
            band.validate()?;
            if !self.requires_power {
                // Passive insulated modules are permitted; this boolean means
                // the module itself requires external power, not refrigeration.
            }
        }
        match &self.conformity {
            ProfileConformity::Verified {
                standard_id,
                edition,
                evidence_digest,
            } if standard_id.trim().is_empty()
                || edition.trim().is_empty()
                || evidence_digest.trim().is_empty() =>
            {
                return Err(CargoError::InvalidProfile(
                    "verified profile requires standard, edition and evidence digest",
                ));
            }
            ProfileConformity::Verified { .. }
            | ProfileConformity::DeclaredUnverified
            | ProfileConformity::NonConforming => {}
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CargoSealStatus {
    Intact,
    Broken,
    NotPresent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CargoServiceability {
    Serviceable,
    InspectionRequired,
    Damaged,
    Quarantined,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoModuleInstance {
    pub module_id: String,
    pub profile_id: String,
    pub payload_mass_grams: u64,
    pub occupied_volume_litres: u64,
    pub contents_digest: String,
    pub seal_status: CargoSealStatus,
    pub serviceability: CargoServiceability,
    /// Required when the profile declares a controlled temperature band.
    pub observed_temperature_millicelsius: Option<i32>,
}

impl CargoModuleInstance {
    pub fn validate_against(
        &self,
        profile: &CargoModuleProfile,
    ) -> Result<u64, CargoError> {
        profile.validate()?;
        if self.module_id.trim().is_empty()
            || self.profile_id != profile.profile_id
            || self.contents_digest.trim().is_empty()
        {
            return Err(CargoError::InvalidInstance(
                "module/profile identity and contents digest are required",
            ));
        }
        if self.payload_mass_grams > profile.max_payload_mass_grams {
            return Err(CargoError::PayloadExceeded);
        }
        if self.occupied_volume_litres > profile.usable_volume_litres {
            return Err(CargoError::VolumeExceeded);
        }
        if profile.requires_intact_seal && self.seal_status != CargoSealStatus::Intact {
            return Err(CargoError::SealRequirementFailed);
        }
        if self.serviceability != CargoServiceability::Serviceable {
            return Err(CargoError::ModuleNotServiceable);
        }
        if let Some(required_band) = profile.required_temperature_millicelsius {
            let Some(observed) = self.observed_temperature_millicelsius else {
                return Err(CargoError::TemperatureEvidenceMissing);
            };
            if !required_band.contains(observed) {
                return Err(CargoError::TemperatureOutOfRange);
            }
        }
        profile
            .tare_mass_grams
            .checked_add(self.payload_mass_grams)
            .filter(|gross| *gross <= profile.max_gross_mass_grams)
            .ok_or(CargoError::GrossMassExceeded)
    }
}

/// The declared limits and interfaces of a specific vehicle position, rack,
/// aircraft ULD position, crane, or terminal handling station.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandlingEquipmentProfile {
    pub equipment_id: String,
    pub mode: Mode,
    pub max_module_dimensions_mm: [u32; 3],
    pub max_module_gross_mass_grams: u64,
    pub supported_families: BTreeSet<CargoModuleFamily>,
    pub interfaces: BTreeSet<HandlingInterface>,
    pub available_temperature_millicelsius: Option<TemperatureBand>,
    pub supplies_module_power: bool,
    pub requires_verified_aircraft_uld: bool,
}

impl HandlingEquipmentProfile {
    pub fn validate(&self) -> Result<(), CargoError> {
        if self.equipment_id.trim().is_empty()
            || self.max_module_dimensions_mm.contains(&0)
            || self.max_module_gross_mass_grams == 0
            || self.supported_families.is_empty()
            || self.interfaces.is_empty()
        {
            return Err(CargoError::InvalidEquipment(
                "identity, dimensions, mass, supported families and interfaces are required",
            ));
        }
        if let Some(band) = self.available_temperature_millicelsius {
            band.validate()?;
        }
        Ok(())
    }
}

/// Check one actual module against one piece of actual handling equipment.
/// This does not certify the equipment or prove route-wide feasibility.
pub fn check_module_compatibility(
    profile: &CargoModuleProfile,
    instance: &CargoModuleInstance,
    equipment: &HandlingEquipmentProfile,
) -> Result<(), CargoError> {
    let gross_mass = instance.validate_against(profile)?;
    equipment.validate()?;

    if !profile.permitted_modes.contains(&equipment.mode) {
        return Err(CargoError::ModeNotPermitted);
    }
    if profile.conformity == ProfileConformity::NonConforming {
        return Err(CargoError::ProfileNonConforming);
    }
    if equipment.mode == Mode::Air
        && equipment.requires_verified_aircraft_uld
        && (profile.family != CargoModuleFamily::AircraftUld
            || !matches!(profile.conformity, ProfileConformity::Verified { .. }))
    {
        return Err(CargoError::AircraftUldNotVerified);
    }
    if !equipment.supported_families.contains(&profile.family) {
        return Err(CargoError::FamilyNotSupported);
    }
    if profile
        .interfaces
        .iter()
        .any(|interface| !equipment.interfaces.contains(interface))
    {
        return Err(CargoError::HandlingInterfaceMismatch);
    }
    if profile
        .outer_dimensions_mm
        .iter()
        .zip(equipment.max_module_dimensions_mm)
        .any(|(module, limit)| *module > limit)
    {
        return Err(CargoError::DimensionsExceeded);
    }
    if gross_mass > equipment.max_module_gross_mass_grams {
        return Err(CargoError::EquipmentGrossMassExceeded);
    }
    if profile.requires_power && !equipment.supplies_module_power {
        return Err(CargoError::ModulePowerUnavailable);
    }
    if let Some(required) = profile.required_temperature_millicelsius {
        let Some(available) = equipment.available_temperature_millicelsius else {
            return Err(CargoError::TemperatureControlUnavailable);
        };
        if !available.contains_band(required) {
            return Err(CargoError::TemperatureControlUnavailable);
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CargoError {
    InvalidProfile(&'static str),
    InvalidInstance(&'static str),
    InvalidEquipment(&'static str),
    InvalidTemperatureBand,
    PayloadExceeded,
    VolumeExceeded,
    GrossMassExceeded,
    SealRequirementFailed,
    ModuleNotServiceable,
    TemperatureEvidenceMissing,
    TemperatureOutOfRange,
    ModeNotPermitted,
    ProfileNonConforming,
    AircraftUldNotVerified,
    FamilyNotSupported,
    HandlingInterfaceMismatch,
    DimensionsExceeded,
    EquipmentGrossMassExceeded,
    ModulePowerUnavailable,
    TemperatureControlUnavailable,
}

impl fmt::Display for CargoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProfile(reason) => write!(f, "invalid cargo module profile: {reason}"),
            Self::InvalidInstance(reason) => write!(f, "invalid cargo module instance: {reason}"),
            Self::InvalidEquipment(reason) => {
                write!(f, "invalid cargo handling equipment profile: {reason}")
            }
            Self::InvalidTemperatureBand => write!(f, "temperature band has minimum above maximum"),
            Self::PayloadExceeded => write!(f, "cargo payload exceeds the module profile rating"),
            Self::VolumeExceeded => write!(f, "cargo volume exceeds usable module volume"),
            Self::GrossMassExceeded => write!(f, "module gross mass exceeds its profile rating"),
            Self::SealRequirementFailed => write!(f, "required module seal is not intact"),
            Self::ModuleNotServiceable => write!(f, "module is not currently serviceable"),
            Self::TemperatureEvidenceMissing => {
                write!(f, "temperature-controlled module has no current temperature evidence")
            }
            Self::TemperatureOutOfRange => {
                write!(f, "observed cargo temperature is outside the required band")
            }
            Self::ModeNotPermitted => write!(f, "module profile does not permit this transport mode"),
            Self::ProfileNonConforming => write!(f, "module profile is marked nonconforming"),
            Self::AircraftUldNotVerified => {
                write!(f, "air transport requires a verified aircraft ULD profile")
            }
            Self::FamilyNotSupported => write!(f, "equipment does not support this module family"),
            Self::HandlingInterfaceMismatch => {
                write!(f, "module handling/locking interfaces do not match equipment")
            }
            Self::DimensionsExceeded => {
                write!(f, "module envelope exceeds the equipment profile")
            }
            Self::EquipmentGrossMassExceeded => {
                write!(f, "module gross mass exceeds equipment position rating")
            }
            Self::ModulePowerUnavailable => write!(f, "module power requirement cannot be met"),
            Self::TemperatureControlUnavailable => {
                write!(f, "equipment cannot satisfy the module temperature requirement")
            }
        }
    }
}

impl std::error::Error for CargoError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Network {
    pub currency_code: String,
    pub nodes: BTreeSet<String>,
    pub edges: Vec<Edge>,
}

impl Network {
    pub fn new<I, S>(
        nodes: I,
        currency_code: impl Into<String>,
    ) -> Result<Self, ModelError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut node_set = BTreeSet::new();
        for node in nodes {
            let node = node.into();
            if node.trim().is_empty() {
                return Err(ModelError::InvalidNodeIdentity);
            }
            if !node_set.insert(node.clone()) {
                return Err(ModelError::DuplicateNode(node));
            }
        }

        let network = Self {
            currency_code: currency_code.into(),
            nodes: node_set,
            edges: Vec::new(),
        };
        network.validate()?;
        Ok(network)
    }

    pub fn add_edge(&mut self, edge: Edge) {
        self.edges.push(edge);
    }

    pub fn validate(&self) -> Result<(), ModelError> {
        if self.nodes.is_empty() {
            return Err(ModelError::EmptyNetwork);
        }
        if self.nodes.iter().any(|node| node.trim().is_empty()) {
            return Err(ModelError::InvalidNodeIdentity);
        }
        if self.currency_code.len() != 3
            || !self.currency_code.bytes().all(|byte| byte.is_ascii_uppercase())
        {
            return Err(ModelError::InvalidCurrencyCode);
        }

        let mut edge_ids = BTreeSet::new();
        for edge in &self.edges {
            if edge.id.trim().is_empty() || !edge_ids.insert(edge.id.as_str()) {
                return Err(ModelError::DuplicateOrEmptyEdgeId(edge.id.clone()));
            }
            if !self.nodes.contains(&edge.from) {
                return Err(ModelError::UnknownNode(edge.from.clone()));
            }
            if !self.nodes.contains(&edge.to) {
                return Err(ModelError::UnknownNode(edge.to.clone()));
            }

            match edge.operation {
                EdgeOperation::Movement { .. } if edge.from == edge.to => {
                    return Err(ModelError::MovementSelfLoop(edge.id.clone()));
                }
                EdgeOperation::Transfer { from, to } if from == to => {
                    return Err(ModelError::RedundantTransfer(edge.id.clone()));
                }
                EdgeOperation::Movement { .. } | EdgeOperation::Transfer { .. } => {}
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Demand {
    pub id: String,
    pub origin: String,
    pub destination: String,
    pub quantity: u64,
    pub unit: QuantityUnit,
    pub max_elapsed_seconds: Option<u64>,
    pub max_transfers: Option<u32>,
}

impl Demand {
    pub fn validate(&self, network: &Network) -> Result<(), ModelError> {
        if self.id.trim().is_empty() || self.quantity == 0 {
            return Err(ModelError::InvalidDemand);
        }
        if !network.nodes.contains(&self.origin) {
            return Err(ModelError::UnknownNode(self.origin.clone()));
        }
        if !network.nodes.contains(&self.destination) {
            return Err(ModelError::UnknownNode(self.destination.clone()));
        }
        Ok(())
    }
}

/// Cumulative per-unit trade-off vector. Every field is minimized when
/// constructing the Pareto frontier; policies may choose from that frontier
/// using explicitly declared constraints or weights after inspection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct RouteMetrics {
    pub distance_metres: u64,
    pub elapsed_seconds: u64,
    pub operator_cost_minor_per_unit: u64,
    pub user_cash_cost_minor_per_unit: u64,
    pub external_cost_minor_per_unit: u64,
    pub energy_wh_per_unit: u64,
    pub ghg_grams_co2e_per_unit: u64,
    pub safety_risk_micro_units_per_unit: u64,
    pub disruption_risk_micro_units_per_unit: u64,
}

impl RouteMetrics {
    fn checked_add(self, other: EdgeMetrics) -> Result<Self, ModelError> {
        Ok(Self {
            distance_metres: add(self.distance_metres, other.distance_metres)?,
            elapsed_seconds: add(self.elapsed_seconds, other.elapsed_seconds)?,
            operator_cost_minor_per_unit: add(
                self.operator_cost_minor_per_unit,
                other.operator_cost_minor_per_unit,
            )?,
            user_cash_cost_minor_per_unit: add(
                self.user_cash_cost_minor_per_unit,
                other.user_cash_cost_minor_per_unit,
            )?,
            external_cost_minor_per_unit: add(
                self.external_cost_minor_per_unit,
                other.external_cost_minor_per_unit,
            )?,
            energy_wh_per_unit: add(self.energy_wh_per_unit, other.energy_wh_per_unit)?,
            ghg_grams_co2e_per_unit: add(
                self.ghg_grams_co2e_per_unit,
                other.ghg_grams_co2e_per_unit,
            )?,
            safety_risk_micro_units_per_unit: add(
                self.safety_risk_micro_units_per_unit,
                other.safety_risk_micro_units_per_unit,
            )?,
            disruption_risk_micro_units_per_unit: add(
                self.disruption_risk_micro_units_per_unit,
                other.disruption_risk_micro_units_per_unit,
            )?,
        })
    }

    fn dominates(self, other: Self) -> bool {
        self.distance_metres <= other.distance_metres
            && self.elapsed_seconds <= other.elapsed_seconds
            && self.operator_cost_minor_per_unit <= other.operator_cost_minor_per_unit
            && self.user_cash_cost_minor_per_unit <= other.user_cash_cost_minor_per_unit
            && self.external_cost_minor_per_unit <= other.external_cost_minor_per_unit
            && self.energy_wh_per_unit <= other.energy_wh_per_unit
            && self.ghg_grams_co2e_per_unit <= other.ghg_grams_co2e_per_unit
            && self.safety_risk_micro_units_per_unit <= other.safety_risk_micro_units_per_unit
            && self.disruption_risk_micro_units_per_unit
                <= other.disruption_risk_micro_units_per_unit
    }

    fn strictly_better_than(self, other: Self) -> bool {
        self != other
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AggregateMetrics {
    pub distance_metres: u64,
    pub elapsed_seconds: u64,
    pub operator_cost_minor_units: u64,
    pub user_cash_cost_minor_units: u64,
    pub external_cost_minor_units: u64,
    pub energy_wh: u64,
    pub ghg_grams_co2e: u64,
    pub safety_risk_micro_units: u64,
    pub disruption_risk_micro_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub edge_ids: Vec<String>,
    pub transfer_count: u32,
    pub metrics_per_unit: RouteMetrics,
}

impl Route {
    /// Scale unit-sensitive outcomes to the full demand quantity. Distance and
    /// elapsed time are route-level measures and are not multiplied by quantity.
    pub fn aggregate(&self, quantity: u64) -> Result<AggregateMetrics, ModelError> {
        if quantity == 0 {
            return Err(ModelError::InvalidDemand);
        }
        Ok(AggregateMetrics {
            distance_metres: self.metrics_per_unit.distance_metres,
            elapsed_seconds: self.metrics_per_unit.elapsed_seconds,
            operator_cost_minor_units: multiply(
                self.metrics_per_unit.operator_cost_minor_per_unit,
                quantity,
            )?,
            user_cash_cost_minor_units: multiply(
                self.metrics_per_unit.user_cash_cost_minor_per_unit,
                quantity,
            )?,
            external_cost_minor_units: multiply(
                self.metrics_per_unit.external_cost_minor_per_unit,
                quantity,
            )?,
            energy_wh: multiply(self.metrics_per_unit.energy_wh_per_unit, quantity)?,
            ghg_grams_co2e: multiply(
                self.metrics_per_unit.ghg_grams_co2e_per_unit,
                quantity,
            )?,
            safety_risk_micro_units: multiply(
                self.metrics_per_unit.safety_risk_micro_units_per_unit,
                quantity,
            )?,
            disruption_risk_micro_units: multiply(
                self.metrics_per_unit.disruption_risk_micro_units_per_unit,
                quantity,
            )?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchLimits {
    pub max_labels_per_state: usize,
    pub max_generated_labels: usize,
}

impl Default for SearchLimits {
    fn default() -> Self {
        Self {
            max_labels_per_state: 256,
            max_generated_labels: 20_000,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelError {
    EmptyNetwork,
    InvalidCurrencyCode,
    InvalidNodeIdentity,
    DuplicateNode(String),
    DuplicateOrEmptyEdgeId(String),
    UnknownNode(String),
    MovementSelfLoop(String),
    RedundantTransfer(String),
    InvalidDemand,
    InvalidSearchLimits,
    LabelLimitExceeded { state: String, limit: usize },
    MetricOverflow,
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyNetwork => write!(f, "network must contain at least one node"),
            Self::InvalidCurrencyCode => {
                write!(f, "currency code must be three uppercase ASCII letters")
            }
            Self::InvalidNodeIdentity => write!(f, "node identity must not be empty"),
            Self::DuplicateNode(node) => write!(f, "duplicate network node {node}"),
            Self::DuplicateOrEmptyEdgeId(edge) => {
                write!(f, "edge identity is empty or duplicated: {edge}")
            }
            Self::UnknownNode(node) => write!(f, "unknown network node {node}"),
            Self::MovementSelfLoop(edge) => write!(f, "movement edge {edge} cannot be a self-loop"),
            Self::RedundantTransfer(edge) => write!(f, "transfer edge {edge} must change mode"),
            Self::InvalidDemand => write!(f, "demand ID and positive quantity are required"),
            Self::InvalidSearchLimits => write!(f, "search label limits must be positive"),
            Self::LabelLimitExceeded { state, limit } => write!(
                f,
                "Pareto label limit {limit} exceeded at {state}; frontier was not truncated"
            ),
            Self::MetricOverflow => write!(f, "route metric arithmetic overflowed"),
        }
    }
}

impl std::error::Error for ModelError {}

/// Enumerate nondominated feasible routes for a single demand.
///
/// Mode changes require an explicit transfer edge. Each movement and transfer
/// checks the demand's unit and required capacity. A nonzero demand deadline or
/// transfer cap prunes infeasible routes. Search limits fail closed instead of
/// returning a silently truncated frontier.
pub fn pareto_routes(
    network: &Network,
    demand: &Demand,
    limits: SearchLimits,
) -> Result<Vec<Route>, ModelError> {
    network.validate()?;
    demand.validate(network)?;
    if limits.max_labels_per_state == 0 || limits.max_generated_labels == 0 {
        return Err(ModelError::InvalidSearchLimits);
    }

    let mut outgoing: BTreeMap<String, Vec<&Edge>> = BTreeMap::new();
    for edge in &network.edges {
        outgoing.entry(edge.from.clone()).or_default().push(edge);
    }
    for edges in outgoing.values_mut() {
        edges.sort_by(|left, right| left.id.cmp(&right.id));
    }

    let start = SearchLabel {
        node: demand.origin.clone(),
        current_mode: None,
        transfer_count: 0,
        metrics: RouteMetrics::default(),
        edge_ids: Vec::new(),
    };
    let start_key = StateKey::from(&start);
    let mut labels: BTreeMap<StateKey, Vec<SearchLabel>> = BTreeMap::new();
    labels.insert(start_key, vec![start.clone()]);
    let mut pending = VecDeque::from([start]);
    let mut generated_labels = 1_usize;

    while let Some(label) = pending.pop_front() {
        let label_key = StateKey::from(&label);
        if !labels
            .get(&label_key)
            .is_some_and(|state_labels| state_labels.contains(&label))
        {
            continue;
        }
        if label.node == demand.destination {
            continue;
        }

        let Some(edges) = outgoing.get(&label.node) else {
            continue;
        };
        for edge in edges {
            if edge.capacity_unit != demand.unit || edge.capacity < demand.quantity {
                continue;
            }

            let (next_mode, next_transfers) = match edge.operation {
                EdgeOperation::Movement { mode } => {
                    if label.current_mode.is_some_and(|current| current != mode) {
                        continue;
                    }
                    (Some(mode), label.transfer_count)
                }
                EdgeOperation::Transfer { from, to } => {
                    if label.current_mode != Some(from) {
                        continue;
                    }
                    let transfers = label
                        .transfer_count
                        .checked_add(1)
                        .ok_or(ModelError::MetricOverflow)?;
                    if demand.max_transfers.is_some_and(|limit| transfers > limit) {
                        continue;
                    }
                    (Some(to), transfers)
                }
            };

            let metrics = label.metrics.checked_add(edge.metrics)?;
            if demand
                .max_elapsed_seconds
                .is_some_and(|limit| metrics.elapsed_seconds > limit)
            {
                continue;
            }

            let mut edge_ids = label.edge_ids.clone();
            edge_ids.push(edge.id.clone());
            let candidate = SearchLabel {
                node: edge.to.clone(),
                current_mode: next_mode,
                transfer_count: next_transfers,
                metrics,
                edge_ids,
            };
            let key = StateKey::from(&candidate);
            let state_labels = labels.entry(key.clone()).or_default();

            if insert_nondominated(state_labels, candidate.clone()) {
                generated_labels = generated_labels
                    .checked_add(1)
                    .ok_or(ModelError::MetricOverflow)?;
                if state_labels.len() > limits.max_labels_per_state {
                    return Err(ModelError::LabelLimitExceeded {
                        state: key.to_string(),
                        limit: limits.max_labels_per_state,
                    });
                }
                if generated_labels > limits.max_generated_labels {
                    return Err(ModelError::LabelLimitExceeded {
                        state: key.to_string(),
                        limit: limits.max_generated_labels,
                    });
                }
                pending.push_back(candidate);
            }
        }
    }

    let mut destination_labels: Vec<_> = labels
        .values()
        .flat_map(|state_labels| state_labels.iter())
        .filter(|label| label.node == demand.destination)
        .cloned()
        .collect();
    retain_nondominated(&mut destination_labels);
    destination_labels.sort_by(|left, right| {
        left.metrics
            .cmp(&right.metrics)
            .then_with(|| left.transfer_count.cmp(&right.transfer_count))
            .then_with(|| left.edge_ids.cmp(&right.edge_ids))
    });

    Ok(destination_labels
        .into_iter()
        .map(|label| Route {
            edge_ids: label.edge_ids,
            transfer_count: label.transfer_count,
            metrics_per_unit: label.metrics,
        })
        .collect())
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StateKey {
    node: String,
    current_mode: Option<Mode>,
}

impl From<&SearchLabel> for StateKey {
    fn from(label: &SearchLabel) -> Self {
        Self {
            node: label.node.clone(),
            current_mode: label.current_mode,
        }
    }
}

impl fmt::Display for StateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{:?}", self.node, self.current_mode)
    }
}

impl PartialOrd for StateKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StateKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.node
            .cmp(&other.node)
            .then_with(|| self.current_mode.cmp(&other.current_mode))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SearchLabel {
    node: String,
    current_mode: Option<Mode>,
    transfer_count: u32,
    metrics: RouteMetrics,
    edge_ids: Vec<String>,
}

impl SearchLabel {
    /// Dominance also accounts for transfers. Equal metric vectors preserve
    /// only the lexicographically first edge sequence for deterministic output.
    fn dominates(&self, other: &Self) -> bool {
        if self.transfer_count > other.transfer_count || !self.metrics.dominates(other.metrics) {
            return false;
        }
        self.metrics.strictly_better_than(other.metrics)
            || self.transfer_count < other.transfer_count
            || self.edge_ids <= other.edge_ids
    }
}

fn insert_nondominated(labels: &mut Vec<SearchLabel>, candidate: SearchLabel) -> bool {
    if labels.iter().any(|existing| existing.dominates(&candidate)) {
        return false;
    }
    labels.retain(|existing| !candidate.dominates(existing));
    labels.push(candidate);
    true
}

fn retain_nondominated(labels: &mut Vec<SearchLabel>) {
    let mut kept: Vec<SearchLabel> = Vec::new();
    for candidate in labels.drain(..) {
        if kept.iter().any(|existing| existing.dominates(&candidate)) {
            continue;
        }
        kept.retain(|existing| !candidate.dominates(existing));
        kept.push(candidate);
    }
    *labels = kept;
}

fn add(left: u64, right: u64) -> Result<u64, ModelError> {
    left.checked_add(right).ok_or(ModelError::MetricOverflow)
}

fn multiply(value: u64, quantity: u64) -> Result<u64, ModelError> {
    value
        .checked_mul(quantity)
        .ok_or(ModelError::MetricOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn passenger_demand(quantity: u64) -> Demand {
        Demand {
            id: "commute-group".to_owned(),
            origin: "A".to_owned(),
            destination: "B".to_owned(),
            quantity,
            unit: QuantityUnit::Passengers,
            max_elapsed_seconds: None,
            max_transfers: None,
        }
    }

    fn metrics(
        operator: u64,
        user_cash: u64,
        seconds: u64,
        distance: u64,
        energy: u64,
        ghg: u64,
        external: u64,
        safety: u64,
        disruption: u64,
    ) -> EdgeMetrics {
        EdgeMetrics {
            distance_metres: distance,
            elapsed_seconds: seconds,
            operator_cost_minor_per_unit: operator,
            user_cash_cost_minor_per_unit: user_cash,
            external_cost_minor_per_unit: external,
            energy_wh_per_unit: energy,
            ghg_grams_co2e_per_unit: ghg,
            safety_risk_micro_units_per_unit: safety,
            disruption_risk_micro_units_per_unit: disruption,
        }
    }

    fn competing_mode_network() -> Network {
        let mut network = Network::new(["A", "Hub", "B"], "ZAR").unwrap();
        network.add_edge(Edge::movement(
            "air-direct",
            "A",
            "B",
            Mode::Air,
            100,
            QuantityUnit::Passengers,
            metrics(500, 600, 1_200, 100_000, 500, 1_000, 100, 2, 8),
        ));
        network.add_edge(Edge::movement(
            "road-feeder",
            "A",
            "Hub",
            Mode::Road,
            100,
            QuantityUnit::Passengers,
            metrics(80, 20, 1_000, 40_000, 30, 150, 10, 4, 6),
        ));
        network.add_edge(Edge::transfer(
            "road-rail-transfer",
            "Hub",
            Mode::Road,
            Mode::Rail,
            100,
            QuantityUnit::Passengers,
            metrics(10, 2, 300, 0, 2, 5, 1, 1, 2),
        ));
        network.add_edge(Edge::movement(
            "rail-linehaul",
            "Hub",
            "B",
            Mode::Rail,
            100,
            QuantityUnit::Passengers,
            metrics(60, 30, 2_200, 300_000, 20, 100, 8, 2, 3),
        ));
        network
    }

    #[test]
    fn returns_pareto_tradeoff_instead_of_hiding_time_cost_tradeoff() {
        let network = competing_mode_network();
        let routes =
            pareto_routes(&network, &passenger_demand(1), SearchLimits::default()).unwrap();

        assert_eq!(routes.len(), 2);
        assert!(routes.iter().any(|route| route.edge_ids == vec!["air-direct".to_owned()]));
        let intermodal = routes
            .iter()
            .find(|route| route.edge_ids == vec![
                "road-feeder".to_owned(),
                "road-rail-transfer".to_owned(),
                "rail-linehaul".to_owned(),
            ])
            .unwrap();
        assert_eq!(intermodal.transfer_count, 1);
        assert_eq!(intermodal.metrics_per_unit.operator_cost_minor_per_unit, 150);
        assert_eq!(intermodal.metrics_per_unit.ghg_grams_co2e_per_unit, 255);
        assert_eq!(intermodal.metrics_per_unit.elapsed_seconds, 3_500);
    }

    #[test]
    fn aggregates_per_unit_costs_but_not_route_time_or_distance() {
        let routes = pareto_routes(
            &competing_mode_network(),
            &passenger_demand(12),
            SearchLimits::default(),
        )
        .unwrap();
        let intermodal = routes
            .iter()
            .find(|route| route.edge_ids.len() == 3)
            .unwrap();
        let total = intermodal.aggregate(12).unwrap();

        assert_eq!(total.operator_cost_minor_units, 1_800);
        assert_eq!(total.user_cash_cost_minor_units, 624);
        assert_eq!(total.elapsed_seconds, 3_500);
        assert_eq!(total.distance_metres, 340_000);
        assert_eq!(total.ghg_grams_co2e, 3_060);
    }

    #[test]
    fn dominated_route_is_removed_from_the_frontier() {
        let mut network = Network::new(["A", "B"], "ZAR").unwrap();
        network.add_edge(Edge::movement(
            "good",
            "A",
            "B",
            Mode::Road,
            10,
            QuantityUnit::Passengers,
            metrics(10, 5, 600, 10_000, 100, 100, 1, 1, 1),
        ));
        network.add_edge(Edge::movement(
            "bad",
            "A",
            "B",
            Mode::Road,
            10,
            QuantityUnit::Passengers,
            metrics(20, 10, 900, 12_000, 200, 200, 2, 2, 2),
        ));
        let routes =
            pareto_routes(&network, &passenger_demand(1), SearchLimits::default()).unwrap();

        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].edge_ids, vec!["good".to_owned()]);
    }

    #[test]
    fn mode_change_requires_an_explicit_transfer_edge() {
        let mut network = Network::new(["A", "Hub", "B"], "ZAR").unwrap();
        network.add_edge(Edge::movement(
            "road",
            "A",
            "Hub",
            Mode::Road,
            10,
            QuantityUnit::Passengers,
            EdgeMetrics::default(),
        ));
        network.add_edge(Edge::movement(
            "rail-without-transfer",
            "Hub",
            "B",
            Mode::Rail,
            10,
            QuantityUnit::Passengers,
            EdgeMetrics::default(),
        ));
        assert!(pareto_routes(&network, &passenger_demand(1), SearchLimits::default())
            .unwrap()
            .is_empty());

        network.add_edge(Edge::transfer(
            "explicit-transfer",
            "Hub",
            Mode::Road,
            Mode::Rail,
            10,
            QuantityUnit::Passengers,
            metrics(1, 1, 60, 0, 1, 1, 0, 0, 0),
        ));
        let routes =
            pareto_routes(&network, &passenger_demand(1), SearchLimits::default()).unwrap();
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].transfer_count, 1);
        assert_eq!(routes[0].metrics_per_unit.elapsed_seconds, 60);
    }

    #[test]
    fn capacity_and_quantity_units_are_fail_closed() {
        let mut network = Network::new(["A", "B"], "ZAR").unwrap();
        network.add_edge(Edge::movement(
            "too-small",
            "A",
            "B",
            Mode::Road,
            2,
            QuantityUnit::Passengers,
            EdgeMetrics::default(),
        ));
        assert!(pareto_routes(&network, &passenger_demand(3), SearchLimits::default())
            .unwrap()
            .is_empty());

        let cargo = Demand {
            id: "ore".to_owned(),
            origin: "A".to_owned(),
            destination: "B".to_owned(),
            quantity: 3,
            unit: QuantityUnit::Tonnes,
            max_elapsed_seconds: None,
            max_transfers: None,
        };
        assert!(pareto_routes(&network, &cargo, SearchLimits::default())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn deadline_and_transfer_limits_prune_infeasible_routes() {
        let network = competing_mode_network();
        let mut demand = passenger_demand(1);
        demand.max_elapsed_seconds = Some(2_000);
        let routes = pareto_routes(&network, &demand, SearchLimits::default()).unwrap();
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].edge_ids, vec!["air-direct".to_owned()]);

        demand.max_elapsed_seconds = None;
        demand.max_transfers = Some(0);
        let routes = pareto_routes(&network, &demand, SearchLimits::default()).unwrap();
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].edge_ids, vec!["air-direct".to_owned()]);

    }

    #[test]
    fn insertion_order_does_not_change_frontier_or_tie_breaking() {
        let original = competing_mode_network();
        let mut reversed = competing_mode_network();
        reversed.edges.reverse();
        let demand = passenger_demand(1);

        let left = pareto_routes(&original, &demand, SearchLimits::default()).unwrap();
        let right = pareto_routes(&reversed, &demand, SearchLimits::default()).unwrap();
        assert_eq!(left, right);
    }

    #[test]
    fn malformed_currency_and_unknown_endpoints_fail_closed() {
        assert_eq!(
            Network::new(["A", "B"], "zar"),
            Err(ModelError::InvalidCurrencyCode)
        );

        let mut network = Network::new(["A", "B"], "ZAR").unwrap();
        network.add_edge(Edge::movement(
            "cost-edge",
            "A",
            "B",
            Mode::Road,
            10,
            QuantityUnit::Passengers,
            metrics(u64::MAX, 0, 1, 1, 0, 0, 0, 0, 0),
        ));
        network.add_edge(Edge::movement(
            "extra",
            "B",
            "C",
            Mode::Road,
            10,
            QuantityUnit::Passengers,
            EdgeMetrics::default(),
        ));
        assert_eq!(network.validate(), Err(ModelError::UnknownNode("C".to_owned())));
    }

    #[test]
    fn validation_rejects_blank_nodes_inserted_after_construction() {
        let mut network = Network::new(["A", "B"], "ZAR").unwrap();
        network.nodes.insert("   ".to_owned());
        assert_eq!(network.validate(), Err(ModelError::InvalidNodeIdentity));
    }

    #[test]
    fn demand_deadline_does_not_overflow_and_bad_search_limits_fail_closed() {
        let mut network = Network::new(["A", "B", "C"], "ZAR").unwrap();
        network.add_edge(Edge::movement(
            "first",
            "A",
            "B",
            Mode::Road,
            10,
            QuantityUnit::Passengers,
            metrics(0, 0, u64::MAX, 0, 0, 0, 0, 0, 0),
        ));
        network.add_edge(Edge::movement(
            "second",
            "B",
            "C",
            Mode::Road,
            10,
            QuantityUnit::Passengers,
            metrics(0, 0, 1, 0, 0, 0, 0, 0, 0),
        ));
        let demand = Demand {
            id: "overflow-time".to_owned(),
            origin: "A".to_owned(),
            destination: "C".to_owned(),
            quantity: 1,
            unit: QuantityUnit::Passengers,
            max_elapsed_seconds: None,
            max_transfers: None,
        };
        assert_eq!(
            pareto_routes(&network, &demand, SearchLimits::default()),
            Err(ModelError::MetricOverflow)
        );
        assert_eq!(
            pareto_routes(
                &network,
                &passenger_demand(1),
                SearchLimits {
                    max_labels_per_state: 0,
                    max_generated_labels: 10,
                },
            ),
            Err(ModelError::InvalidSearchLimits)
        );
    }


    #[test]
    fn cargo_route_respects_tonne_capacity_and_reports_whole_shipment_totals() {
        let mut network = Network::new(["Mine", "Port", "Market"], "ZAR").unwrap();
        network.add_edge(Edge::movement(
            "rail-to-port",
            "Mine",
            "Port",
            Mode::Rail,
            1_000,
            QuantityUnit::Tonnes,
            metrics(20, 25, 6_000, 500_000, 300, 50, 10, 1, 4),
        ));
        network.add_edge(Edge::transfer(
            "rail-maritime-transfer",
            "Port",
            Mode::Rail,
            Mode::Maritime,
            500,
            QuantityUnit::Tonnes,
            metrics(4, 5, 1_800, 0, 10, 1, 1, 0, 2),
        ));
        network.add_edge(Edge::movement(
            "ship-linehaul",
            "Port",
            "Market",
            Mode::Maritime,
            500,
            QuantityUnit::Tonnes,
            metrics(15, 35, 20_000, 1_000_000, 250, 25, 5, 2, 8),
        ));
        network.add_edge(Edge::movement(
            "air-cargo-direct",
            "Mine",
            "Market",
            Mode::Air,
            200,
            QuantityUnit::Tonnes,
            metrics(1_000, 1_200, 1_200, 1_500_000, 2_500, 2_000, 250, 10, 12),
        ));
        let demand = Demand {
            id: "ore-shipment".to_owned(),
            origin: "Mine".to_owned(),
            destination: "Market".to_owned(),
            quantity: 100,
            unit: QuantityUnit::Tonnes,
            max_elapsed_seconds: None,
            max_transfers: Some(1),
        };

        let routes = pareto_routes(&network, &demand, SearchLimits::default()).unwrap();
        assert_eq!(routes.len(), 2);
        let rail_sea = routes
            .iter()
            .find(|route| route.edge_ids.len() == 3)
            .unwrap();
        assert_eq!(rail_sea.metrics_per_unit.operator_cost_minor_per_unit, 39);
        assert_eq!(rail_sea.metrics_per_unit.ghg_grams_co2e_per_unit, 76);
        assert_eq!(rail_sea.metrics_per_unit.elapsed_seconds, 27_800);

        let totals = rail_sea.aggregate(demand.quantity).unwrap();
        assert_eq!(totals.operator_cost_minor_units, 3_900);
        assert_eq!(totals.ghg_grams_co2e, 7_600);

        let mut oversize = demand;
        oversize.quantity = 400;
        let routes = pareto_routes(&network, &oversize, SearchLimits::default()).unwrap();
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].edge_ids, vec![
            "rail-to-port".to_owned(),
            "rail-maritime-transfer".to_owned(),
            "ship-linehaul".to_owned(),
        ]);
    }



    #[test]
    fn aggregate_cost_overflow_is_rejected() {
        let route = Route {
            edge_ids: vec!["synthetic".to_owned()],
            transfer_count: 0,
            metrics_per_unit: RouteMetrics {
                operator_cost_minor_per_unit: u64::MAX,
                ..RouteMetrics::default()
            },
        };
        assert_eq!(route.aggregate(2), Err(ModelError::MetricOverflow));
    }

}
