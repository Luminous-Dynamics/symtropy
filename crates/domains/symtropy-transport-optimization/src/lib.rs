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

mod capacity_window;
pub use capacity_window::*;
mod time_dependent;
pub use time_dependent::*;

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

/// Dimension-axis permutation relative to the declared upright frame. XYZ is
/// upright; YXZ swaps the floor-plane axes; other variants tip a vertical axis.
/// The profile and specific equipment must both explicitly allow the selected
/// permutation. This is geometric screening, not a stability or load approval.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CargoOrientation {
    XYZ,
    XZY,
    YXZ,
    YZX,
    ZXY,
    ZYX,
}

impl CargoOrientation {
    pub fn permute_dimensions(self, dimensions_mm: [u32; 3]) -> [u32; 3] {
        let [x, y, z] = dimensions_mm;
        match self {
            Self::XYZ => [x, y, z],
            Self::XZY => [x, z, y],
            Self::YXZ => [y, x, z],
            Self::YZX => [y, z, x],
            Self::ZXY => [z, x, y],
            Self::ZYX => [z, y, x],
        }
    }
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
pub struct AircraftUldEligibilityEvidence {
    pub uld_type_code: String,
    pub aircraft_type_code: String,
    pub aircraft_position_id: String,
    pub approval_reference: String,
    pub max_gross_mass_grams: u64,
    pub evidence_digest: String,
}

impl AircraftUldEligibilityEvidence {
    fn validate(&self) -> Result<(), CargoError> {
        if self.uld_type_code.trim().is_empty()
            || self.aircraft_type_code.trim().is_empty()
            || self.aircraft_position_id.trim().is_empty()
            || self.approval_reference.trim().is_empty()
            || self.evidence_digest.trim().is_empty()
            || self.max_gross_mass_grams == 0
        {
            return Err(CargoError::InvalidProfile(
                "aircraft ULD eligibility evidence is incomplete",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoModuleProfile {
    pub profile_id: String,
    pub family: CargoModuleFamily,
    pub conformity: ProfileConformity,
    pub outer_dimensions_mm: [u32; 3],
    pub permitted_orientations: BTreeSet<CargoOrientation>,
    pub usable_volume_litres: u64,
    pub tare_mass_grams: u64,
    pub max_payload_mass_grams: u64,
    pub max_gross_mass_grams: u64,
    pub interfaces: BTreeSet<HandlingInterface>,
    pub permitted_modes: BTreeSet<Mode>,
    pub required_temperature_millicelsius: Option<TemperatureBand>,
    pub requires_power: bool,
    pub requires_intact_seal: bool,
    pub requires_verified_load_securing: bool,
    pub aircraft_uld_eligibility: Option<AircraftUldEligibilityEvidence>,
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
        if self.permitted_orientations.is_empty() {
            return Err(CargoError::InvalidProfile(
                "at least one permitted module orientation is required",
            ));
        }
        if let Some(evidence) = &self.aircraft_uld_eligibility {
            evidence.validate()?;
            if self.family != CargoModuleFamily::AircraftUld {
                return Err(CargoError::InvalidProfile(
                    "aircraft ULD eligibility evidence requires the aircraft ULD family",
                ));
            }
        }
        if self.max_payload_mass_grams == 0 {
            return Err(CargoError::InvalidProfile(
                "maximum payload mass must be positive",
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoCustodyState {
    pub module_id: String,
    pub current_custodian_id: String,
    pub current_location_id: String,
    pub revision: u64,
    pub contents_digest: String,
    pub seal_status: CargoSealStatus,
    pub last_receipt_digest: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoCustodyHandoff {
    pub module_id: String,
    pub expected_revision: u64,
    pub from_custodian_id: String,
    pub to_custodian_id: String,
    pub from_location_id: String,
    pub to_location_id: String,
    pub contents_digest_before: String,
    pub contents_digest_after: String,
    pub seal_status_before: CargoSealStatus,
    pub seal_status_after: CargoSealStatus,
    pub seal_transition_evidence_digest: Option<String>,
    pub receipt_digest: String,
}

/// Applies one proposed handoff to a single current-state snapshot. Persistence,
/// issuer authentication and multi-writer serialization remain the caller's
/// responsibility. Reapplying a stale receipt fails its revision check.
pub fn apply_cargo_custody_handoff(
    current: &CargoCustodyState,
    handoff: &CargoCustodyHandoff,
) -> Result<CargoCustodyState, CargoCustodyError> {
    if current.module_id.trim().is_empty()
        || current.current_custodian_id.trim().is_empty()
        || current.current_location_id.trim().is_empty()
        || current.contents_digest.trim().is_empty()
        || current
            .last_receipt_digest
            .as_deref()
            .is_some_and(str::is_empty)
    {
        return Err(CargoCustodyError::InvalidCurrentState);
    }
    if handoff.module_id.trim().is_empty()
        || handoff.from_custodian_id.trim().is_empty()
        || handoff.to_custodian_id.trim().is_empty()
        || handoff.from_location_id.trim().is_empty()
        || handoff.to_location_id.trim().is_empty()
        || handoff.receipt_digest.trim().is_empty()
    {
        return Err(CargoCustodyError::InvalidHandoff);
    }
    if handoff.expected_revision != current.revision {
        return Err(CargoCustodyError::StaleRevision);
    }
    if handoff.module_id != current.module_id
        || handoff.from_custodian_id != current.current_custodian_id
        || handoff.from_location_id != current.current_location_id
    {
        return Err(CargoCustodyError::AuthorityMismatch);
    }
    if handoff.contents_digest_before != current.contents_digest
        || handoff.contents_digest_after != current.contents_digest
    {
        return Err(CargoCustodyError::ContentsDigestMismatch);
    }
    if handoff.seal_status_before != current.seal_status {
        return Err(CargoCustodyError::SealStateMismatch);
    }
    if handoff.seal_status_after != handoff.seal_status_before
        && handoff
            .seal_transition_evidence_digest
            .as_deref()
            .is_none_or(str::is_empty)
    {
        return Err(CargoCustodyError::SealTransitionEvidenceMissing);
    }
    let revision = current
        .revision
        .checked_add(1)
        .ok_or(CargoCustodyError::RevisionOverflow)?;

    Ok(CargoCustodyState {
        module_id: current.module_id.clone(),
        current_custodian_id: handoff.to_custodian_id.clone(),
        current_location_id: handoff.to_location_id.clone(),
        revision,
        contents_digest: current.contents_digest.clone(),
        seal_status: handoff.seal_status_after,
        last_receipt_digest: Some(handoff.receipt_digest.clone()),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CargoCustodyError {
    InvalidCurrentState,
    InvalidHandoff,
    StaleRevision,
    AuthorityMismatch,
    ContentsDigestMismatch,
    SealStateMismatch,
    SealTransitionEvidenceMissing,
    RevisionOverflow,
}

impl fmt::Display for CargoCustodyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCurrentState => write!(f, "current cargo custody state is incomplete"),
            Self::InvalidHandoff => {
                write!(f, "cargo handoff is missing required identity/evidence")
            }
            Self::StaleRevision => {
                write!(f, "cargo handoff revision is stale or was already applied")
            }
            Self::AuthorityMismatch => {
                write!(f, "cargo handoff does not match current custody authority")
            }
            Self::ContentsDigestMismatch => {
                write!(f, "cargo handoff changes or mismatches the contents digest")
            }
            Self::SealStateMismatch => {
                write!(f, "cargo handoff seal state does not match current state")
            }
            Self::SealTransitionEvidenceMissing => {
                write!(f, "a seal-state change requires evidence")
            }
            Self::RevisionOverflow => write!(f, "cargo custody revision overflowed"),
        }
    }
}

impl std::error::Error for CargoCustodyError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CargoServiceability {
    Serviceable,
    InspectionRequired,
    Damaged,
    Quarantined,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadSecuringStatus {
    Verified { record_digest: String },
    Unverified,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoModuleInstance {
    pub module_id: String,
    pub profile_id: String,
    pub payload_mass_grams: u64,
    pub occupied_volume_litres: u64,
    pub contents_digest: String,
    pub seal_status: CargoSealStatus,
    pub load_securing_status: LoadSecuringStatus,
    pub serviceability: CargoServiceability,
    /// Required when the profile declares a controlled temperature band.
    pub observed_temperature_millicelsius: Option<i32>,
}

impl CargoModuleInstance {
    pub fn validate_against(&self, profile: &CargoModuleProfile) -> Result<u64, CargoError> {
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
        if profile.requires_verified_load_securing {
            match &self.load_securing_status {
                LoadSecuringStatus::Verified { record_digest }
                    if !record_digest.trim().is_empty() => {}
                LoadSecuringStatus::Verified { .. } | LoadSecuringStatus::Unverified => {
                    return Err(CargoError::LoadSecuringEvidenceMissing);
                }
                LoadSecuringStatus::Failed => return Err(CargoError::LoadSecuringFailed),
            }
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
    pub accepted_orientations: BTreeSet<CargoOrientation>,
    pub max_module_gross_mass_grams: u64,
    pub supported_families: BTreeSet<CargoModuleFamily>,
    pub interfaces: BTreeSet<HandlingInterface>,
    pub available_temperature_millicelsius: Option<TemperatureBand>,
    pub supplies_module_power: bool,
    pub requires_verified_aircraft_uld: bool,
    pub aircraft_type_code: Option<String>,
    pub aircraft_position_id: Option<String>,
    pub accepted_uld_type_codes: BTreeSet<String>,
}

impl HandlingEquipmentProfile {
    pub fn validate(&self) -> Result<(), CargoError> {
        if self.equipment_id.trim().is_empty()
            || self.max_module_dimensions_mm.contains(&0)
            || self.accepted_orientations.is_empty()
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
        if self.requires_verified_aircraft_uld
            && (self.mode != Mode::Air
                || self.aircraft_type_code.as_deref().is_none_or(str::is_empty)
                || self
                    .aircraft_position_id
                    .as_deref()
                    .is_none_or(str::is_empty)
                || self.accepted_uld_type_codes.is_empty())
        {
            return Err(CargoError::InvalidEquipment(
                "verified ULD policy requires aircraft type, position and accepted ULD codes",
            ));
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
    if equipment.mode == Mode::Air && equipment.requires_verified_aircraft_uld {
        if profile.family != CargoModuleFamily::AircraftUld
            || !matches!(&profile.conformity, ProfileConformity::Verified { .. })
        {
            return Err(CargoError::AircraftUldNotVerified);
        }
        let Some(approval) = &profile.aircraft_uld_eligibility else {
            return Err(CargoError::AircraftUldNotVerified);
        };
        let aircraft_type_code = equipment.aircraft_type_code.as_deref().unwrap_or_default();
        let aircraft_position_id = equipment
            .aircraft_position_id
            .as_deref()
            .unwrap_or_default();
        if approval.aircraft_type_code != aircraft_type_code
            || approval.aircraft_position_id != aircraft_position_id
            || approval.max_gross_mass_grams < gross_mass
            || !equipment
                .accepted_uld_type_codes
                .contains(&approval.uld_type_code)
        {
            return Err(CargoError::AircraftUldIncompatible);
        }
    }
    if !equipment.supported_families.contains(&profile.family) {
        return Err(CargoError::FamilyNotSupported);
    }
    if !profile
        .interfaces
        .iter()
        .any(|interface| equipment.interfaces.contains(interface))
    {
        return Err(CargoError::HandlingInterfaceMismatch);
    }
    let compatible_orientations: Vec<_> = profile
        .permitted_orientations
        .intersection(&equipment.accepted_orientations)
        .copied()
        .collect();
    if compatible_orientations.is_empty() {
        return Err(CargoError::OrientationNotSupported);
    }
    if !compatible_orientations.iter().any(|orientation| {
        orientation
            .permute_dimensions(profile.outer_dimensions_mm)
            .iter()
            .zip(equipment.max_module_dimensions_mm)
            .all(|(module, limit)| *module <= limit)
    }) {
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

/// Terminal transfer equipment is modeled independently from a vehicle position.
/// A crane, conveyor, roller deck, or cross-dock station can support a mode pair
/// without being part of either vehicle's physical loading envelope. This is a
/// compatibility declaration only; it does not certify safe loading or securement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoTransferEquipmentProfile {
    pub equipment_id: String,
    pub terminal_id: String,
    pub supported_mode_pairs: BTreeSet<(Mode, Mode)>,
    pub max_module_dimensions_mm: [u32; 3],
    pub accepted_orientations: BTreeSet<CargoOrientation>,
    pub max_module_gross_mass_grams: u64,
    pub supported_families: BTreeSet<CargoModuleFamily>,
    pub interfaces: BTreeSet<HandlingInterface>,
    pub available_temperature_millicelsius: Option<TemperatureBand>,
    pub supplies_module_power: bool,
}

impl CargoTransferEquipmentProfile {
    pub fn validate(&self) -> Result<(), CargoError> {
        if self.equipment_id.trim().is_empty()
            || self.terminal_id.trim().is_empty()
            || self.supported_mode_pairs.is_empty()
            || self.max_module_dimensions_mm.contains(&0)
            || self.accepted_orientations.is_empty()
            || self.max_module_gross_mass_grams == 0
            || self.supported_families.is_empty()
            || self.interfaces.is_empty()
        {
            return Err(CargoError::InvalidEquipment(
                "transfer equipment requires identity, terminal, mode pairs, dimensions, mass, families and interfaces",
            ));
        }
        if let Some(band) = self.available_temperature_millicelsius {
            band.validate()?;
        }
        Ok(())
    }
}

/// Binds a proposed module transfer to one named equipment resource and the
/// capacity unit used by the corresponding transfer edge. This is a planning
/// binding, not a reservation or a claim that the equipment is available in a
/// particular time window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoTransferResourceBinding {
    /// Unique within one planning batch; prevents replaying one assignment twice.
    pub assignment_id: String,
    /// Capacity accounting scope, such as a named shift or frozen scenario window.
    /// It is an identifier, not a timestamp or proof of live availability.
    pub capacity_scope_id: String,
    pub module_id: String,
    pub module_profile_id: String,
    pub transfer_edge_id: String,
    pub equipment_id: String,
    pub terminal_id: String,
    /// Capacity consumed by this proposed transfer in the edge's declared unit.
    pub capacity_units: u64,
    pub capacity_unit: QuantityUnit,
}

/// Available capacity budget for one named terminal-equipment resource in one
/// explicit planning scope. Capacity must use the same unit as its bindings.
/// This is supplied planning data, not proof of a live reservation/availability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoTransferEquipmentCapacity {
    pub equipment_id: String,
    pub terminal_id: String,
    pub capacity_scope_id: String,
    pub capacity_units: u64,
    pub capacity_unit: QuantityUnit,
}

/// Source qualification as reported by the adapter. These states are claims
/// about source lifecycle, not cryptographic or physical-world verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CargoTransferCapacityEvidenceStatus {
    ReportedCurrent,
    Superseded,
    Revoked,
    Conflicted,
}

/// Provenance and time envelope for one equipment-capacity snapshot.
///
/// All timestamp fields use Unix epoch seconds under the declared time profile.
/// The source business-event timestamp is intentionally not used as the capacity
/// observation time. Repository record time describes bookkeeping only; freshness
/// is evaluated from the explicit capacity observation timestamp.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoTransferCapacityEvidenceV1 {
    pub budget_id: String,
    pub budget: CargoTransferEquipmentCapacity,
    pub authority_id: String,
    pub authority_profile_id: String,
    pub source_record_id: String,
    pub source_schema_profile_id: String,
    pub source_record_digest: String,
    pub revision: u64,
    pub supersedes_revision: Option<u64>,
    pub source_event_time_epoch_seconds: Option<u64>,
    pub capacity_observed_at_epoch_seconds: Option<u64>,
    pub repository_recorded_at_epoch_seconds: Option<u64>,
    pub valid_from_epoch_seconds: Option<u64>,
    pub valid_until_epoch_seconds: Option<u64>,
    pub time_profile_id: String,
    pub status: CargoTransferCapacityEvidenceStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CargoTransferCapacityFreshnessPolicy {
    pub expected_time_profile_id: &'static str,
    pub max_observation_age_seconds: u64,
    pub allowed_future_clock_skew_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CargoTransferCapacityEvidenceError {
    InvalidIdentityOrProvenance,
    InvalidRevisionLineage,
    TimeProfileMismatch,
    MissingObservationTime,
    MissingRepositoryRecordTime,
    MissingValidityInterval,
    InvalidValidityInterval,
    NotYetValid,
    Expired,
    ObservationTooOld,
    TimestampInFuture,
    NotCurrent,
    DuplicateBudgetIdentity,
    UnreferencedBudgetEvidence,
}

impl fmt::Display for CargoTransferCapacityEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentityOrProvenance => {
                write!(f, "capacity evidence identity or provenance is incomplete")
            }
            Self::InvalidRevisionLineage => {
                write!(f, "capacity evidence revision lineage is invalid")
            }
            Self::TimeProfileMismatch => {
                write!(
                    f,
                    "capacity evidence time profile does not match the consumer policy"
                )
            }
            Self::MissingObservationTime => {
                write!(
                    f,
                    "capacity evidence has no explicit capacity observation time"
                )
            }
            Self::MissingRepositoryRecordTime => {
                write!(f, "capacity evidence has no repository record time")
            }
            Self::MissingValidityInterval => {
                write!(f, "capacity evidence has no declared validity interval")
            }
            Self::InvalidValidityInterval => {
                write!(
                    f,
                    "capacity evidence validity interval is empty or reversed"
                )
            }
            Self::NotYetValid => write!(f, "capacity evidence validity interval has not started"),
            Self::Expired => write!(f, "capacity evidence validity interval has expired"),
            Self::ObservationTooOld => {
                write!(
                    f,
                    "capacity observation exceeds the requested freshness policy"
                )
            }
            Self::TimestampInFuture => {
                write!(
                    f,
                    "capacity evidence timestamp exceeds allowed future clock skew"
                )
            }
            Self::NotCurrent => {
                write!(f, "capacity evidence is superseded, revoked, or conflicted")
            }
            Self::DuplicateBudgetIdentity => {
                write!(f, "capacity evidence repeats a budget identity")
            }
            Self::UnreferencedBudgetEvidence => {
                write!(
                    f,
                    "capacity evidence is not referenced by a proposed assignment"
                )
            }
        }
    }
}

impl std::error::Error for CargoTransferCapacityEvidenceError {}

/// Validate evidence structure, source-reported lifecycle, explicit validity,
/// and capacity observation freshness against a caller-supplied evaluation time.
/// This function is deterministic and reads no ambient clock. It does not
/// authenticate the issuer, recompute the source digest, prove physical truth,
/// or establish live availability.
pub fn validate_cargo_transfer_capacity_evidence(
    evidence: &CargoTransferCapacityEvidenceV1,
    evaluation_time_epoch_seconds: u64,
    policy: CargoTransferCapacityFreshnessPolicy,
) -> Result<(), CargoTransferCapacityEvidenceError> {
    let budget = &evidence.budget;
    if evidence.budget_id.trim().is_empty()
        || budget.equipment_id.trim().is_empty()
        || budget.terminal_id.trim().is_empty()
        || budget.capacity_scope_id.trim().is_empty()
        || budget.capacity_units == 0
        || evidence.authority_id.trim().is_empty()
        || evidence.authority_profile_id.trim().is_empty()
        || evidence.source_record_id.trim().is_empty()
        || evidence.source_schema_profile_id.trim().is_empty()
        || evidence.source_record_digest.trim().is_empty()
        || evidence.time_profile_id.trim().is_empty()
        || evidence.revision == 0
    {
        return Err(CargoTransferCapacityEvidenceError::InvalidIdentityOrProvenance);
    }
    if evidence.time_profile_id != policy.expected_time_profile_id {
        return Err(CargoTransferCapacityEvidenceError::TimeProfileMismatch);
    }

    if evidence
        .supersedes_revision
        .is_some_and(|previous| previous >= evidence.revision)
    {
        return Err(CargoTransferCapacityEvidenceError::InvalidRevisionLineage);
    }

    if evidence.status != CargoTransferCapacityEvidenceStatus::ReportedCurrent {
        return Err(CargoTransferCapacityEvidenceError::NotCurrent);
    }

    let observation = evidence
        .capacity_observed_at_epoch_seconds
        .ok_or(CargoTransferCapacityEvidenceError::MissingObservationTime)?;
    let recorded = evidence
        .repository_recorded_at_epoch_seconds
        .ok_or(CargoTransferCapacityEvidenceError::MissingRepositoryRecordTime)?;
    let valid_from = evidence
        .valid_from_epoch_seconds
        .ok_or(CargoTransferCapacityEvidenceError::MissingValidityInterval)?;
    let valid_until = evidence
        .valid_until_epoch_seconds
        .ok_or(CargoTransferCapacityEvidenceError::MissingValidityInterval)?;

    if valid_from >= valid_until {
        return Err(CargoTransferCapacityEvidenceError::InvalidValidityInterval);
    }
    if evaluation_time_epoch_seconds < valid_from {
        return Err(CargoTransferCapacityEvidenceError::NotYetValid);
    }
    if evaluation_time_epoch_seconds >= valid_until {
        return Err(CargoTransferCapacityEvidenceError::Expired);
    }

    let latest_accepted_time =
        evaluation_time_epoch_seconds.saturating_add(policy.allowed_future_clock_skew_seconds);
    if observation > latest_accepted_time || recorded > latest_accepted_time {
        return Err(CargoTransferCapacityEvidenceError::TimestampInFuture);
    }
    if evaluation_time_epoch_seconds.saturating_sub(observation)
        > policy.max_observation_age_seconds
    {
        return Err(CargoTransferCapacityEvidenceError::ObservationTooOld);
    }

    Ok(())
}

/// Capacity use emitted by deterministic batch validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoTransferCapacityUse {
    pub resource_kind: CargoTransferCapacityResourceKind,
    pub resource_id: String,
    pub capacity_scope_id: String,
    pub capacity_units_used: u64,
    pub capacity_units_limit: u64,
    pub capacity_unit: QuantityUnit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CargoTransferCapacityResourceKind {
    TransferEdge,
    Equipment,
}

/// A reproducible capacity accounting summary for a proposed planning batch.
/// The numeric-only validator emits this after capacity checks; callers needing
/// combined physical checks should use `validate_cargo_transfer_plan`. Time-window
/// scheduling and live availability remain outside this report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoTransferCapacityReport {
    pub assignment_count: usize,
    pub capacity_uses: Vec<CargoTransferCapacityUse>,
}

/// Reject duplicate assignments, missing/ambiguous resource budgets, capacity
/// unit mismatches, and overcommitment of either a transfer edge or a shared
/// equipment resource across multiple edges in the same planning scope.
///
/// The input edge IDs must be unique. Equipment capacity is grouped by
/// equipment ID + planning scope (not by edge), so two edges cannot each spend
/// the same crane's full declared capacity. Inputs and outputs are deterministic.
pub fn validate_cargo_transfer_capacity_plan(
    bindings: &[CargoTransferResourceBinding],
    edges: &[Edge],
    equipment_capacities: &[CargoTransferEquipmentCapacity],
) -> Result<CargoTransferCapacityReport, CargoError> {
    use std::collections::btree_map::Entry;

    let mut edges_by_id = BTreeMap::new();
    for edge in edges {
        if edge.id.trim().is_empty() || edges_by_id.insert(edge.id.clone(), edge).is_some() {
            return Err(CargoError::TransferCapacityPlanInvalid);
        }
    }

    let mut equipment_budgets = BTreeMap::new();
    for budget in equipment_capacities {
        if budget.equipment_id.trim().is_empty()
            || budget.terminal_id.trim().is_empty()
            || budget.capacity_scope_id.trim().is_empty()
            || budget.capacity_units == 0
        {
            return Err(CargoError::TransferCapacityPlanInvalid);
        }
        let key = (
            budget.equipment_id.as_str(),
            budget.capacity_scope_id.as_str(),
        );
        if equipment_budgets.insert(key, budget).is_some() {
            return Err(CargoError::TransferCapacityBudgetDuplicate);
        }
    }

    let mut assignments = BTreeSet::new();
    let mut edge_used: BTreeMap<(String, String, QuantityUnit), u64> = BTreeMap::new();
    let mut equipment_used: BTreeMap<(String, String, QuantityUnit), u64> = BTreeMap::new();

    for binding in bindings {
        if binding.assignment_id.trim().is_empty()
            || binding.capacity_scope_id.trim().is_empty()
            || binding.module_id.trim().is_empty()
            || binding.module_profile_id.trim().is_empty()
            || binding.transfer_edge_id.trim().is_empty()
            || binding.equipment_id.trim().is_empty()
            || binding.terminal_id.trim().is_empty()
            || binding.capacity_units == 0
        {
            return Err(CargoError::TransferResourceBindingInvalid);
        }
        if !assignments.insert(binding.assignment_id.as_str()) {
            return Err(CargoError::TransferAssignmentIdDuplicate);
        }

        let edge = edges_by_id
            .get(&binding.transfer_edge_id)
            .ok_or(CargoError::TransferResourceBindingMismatch)?;
        if !matches!(edge.operation, EdgeOperation::Transfer { .. })
            || edge.from != edge.to
            || edge.from != binding.terminal_id
        {
            return Err(CargoError::TransferTerminalMismatch);
        }
        if edge.capacity == 0 {
            return Err(CargoError::TransferCapacityUnavailable);
        }
        if edge.capacity_unit != binding.capacity_unit {
            return Err(CargoError::TransferCapacityUnitMismatch);
        }
        if binding.capacity_units > edge.capacity {
            return Err(CargoError::TransferCapacityExceeded);
        }

        let budget = equipment_budgets
            .get(&(
                binding.equipment_id.as_str(),
                binding.capacity_scope_id.as_str(),
            ))
            .ok_or(CargoError::TransferCapacityBudgetMissing)?;
        if budget.terminal_id != binding.terminal_id {
            return Err(CargoError::TransferResourceBindingMismatch);
        }
        if budget.capacity_unit != binding.capacity_unit {
            return Err(CargoError::TransferCapacityUnitMismatch);
        }

        let edge_key = (
            binding.transfer_edge_id.clone(),
            binding.capacity_scope_id.clone(),
            binding.capacity_unit,
        );
        let used_edge = match edge_used.entry(edge_key) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(0),
        };
        *used_edge = used_edge
            .checked_add(binding.capacity_units)
            .ok_or(CargoError::TransferCapacityArithmeticOverflow)?;
        if *used_edge > edge.capacity {
            return Err(CargoError::TransferCapacityExceeded);
        }

        let equipment_key = (
            binding.equipment_id.clone(),
            binding.capacity_scope_id.clone(),
            binding.capacity_unit,
        );
        let used_equipment = match equipment_used.entry(equipment_key) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(0),
        };
        *used_equipment = used_equipment
            .checked_add(binding.capacity_units)
            .ok_or(CargoError::TransferCapacityArithmeticOverflow)?;
        if *used_equipment > budget.capacity_units {
            return Err(CargoError::TransferEquipmentCapacityExceeded);
        }
    }

    let mut capacity_uses = Vec::with_capacity(edge_used.len() + equipment_used.len());
    for ((resource_id, capacity_scope_id, capacity_unit), used) in edge_used {
        let edge = edges_by_id
            .get(&resource_id)
            .ok_or(CargoError::TransferResourceBindingMismatch)?;
        capacity_uses.push(CargoTransferCapacityUse {
            resource_kind: CargoTransferCapacityResourceKind::TransferEdge,
            resource_id,
            capacity_scope_id,
            capacity_units_used: used,
            capacity_units_limit: edge.capacity,
            capacity_unit,
        });
    }
    for ((resource_id, capacity_scope_id, capacity_unit), used) in equipment_used {
        let capacity_units_limit = equipment_budgets
            .get(&(resource_id.as_str(), capacity_scope_id.as_str()))
            .map(|budget| budget.capacity_units)
            .ok_or(CargoError::TransferCapacityBudgetMissing)?;
        capacity_uses.push(CargoTransferCapacityUse {
            resource_kind: CargoTransferCapacityResourceKind::Equipment,
            resource_id,
            capacity_scope_id,
            capacity_units_used: used,
            capacity_units_limit,
            capacity_unit,
        });
    }
    capacity_uses.sort_by(|left, right| {
        (
            left.resource_kind,
            left.capacity_scope_id.as_str(),
            left.resource_id.as_str(),
            left.capacity_unit,
        )
            .cmp(&(
                right.resource_kind,
                right.capacity_scope_id.as_str(),
                right.resource_id.as_str(),
                right.capacity_unit,
            ))
    });

    Ok(CargoTransferCapacityReport {
        assignment_count: assignments.len(),
        capacity_uses,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CargoTransferPlanEvidenceError {
    Evidence(CargoTransferCapacityEvidenceError),
    Plan(CargoError),
}

impl fmt::Display for CargoTransferPlanEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evidence(error) => write!(f, "capacity evidence rejected: {error}"),
            Self::Plan(error) => write!(f, "cargo transfer plan rejected: {error}"),
        }
    }
}

impl std::error::Error for CargoTransferPlanEvidenceError {}

impl From<CargoTransferCapacityEvidenceError> for CargoTransferPlanEvidenceError {
    fn from(error: CargoTransferCapacityEvidenceError) -> Self {
        Self::Evidence(error)
    }
}

impl From<CargoError> for CargoTransferPlanEvidenceError {
    fn from(error: CargoError) -> Self {
        Self::Plan(error)
    }
}

/// One assignment's inputs for the combined physical-and-capacity plan gate.
/// References keep canonical profiles/equipment outside the proposal while the
/// binding explicitly names the selected module, edge, equipment and scope.
#[derive(Clone, Copy, Debug)]
pub struct CargoTransferPlanEntry<'a> {
    pub profile: &'a CargoModuleProfile,
    pub instance: &'a CargoModuleInstance,
    pub equipment: &'a CargoTransferEquipmentProfile,
    pub binding: &'a CargoTransferResourceBinding,
}

/// Validate a complete transfer proposal through both existing gates: first
/// aggregate edge/equipment capacity, then each assignment's physical transfer
/// compatibility. Prefer this API over calling the numeric capacity validator
/// alone when a proposed batch must be considered feasible.
///
/// The caller supplies authoritative graph edges and equipment budgets for the
/// planning scope. This still does not establish budget freshness, reserve a
/// time window, or prove live availability.
pub fn validate_cargo_transfer_plan(
    entries: &[CargoTransferPlanEntry<'_>],
    edges: &[Edge],
    equipment_capacities: &[CargoTransferEquipmentCapacity],
) -> Result<CargoTransferCapacityReport, CargoError> {
    let bindings: Vec<_> = entries
        .iter()
        .map(|entry| (*entry.binding).clone())
        .collect();
    let report = validate_cargo_transfer_capacity_plan(&bindings, edges, equipment_capacities)?;
    let edges_by_id: BTreeMap<_, _> = edges.iter().map(|edge| (edge.id.as_str(), edge)).collect();

    for entry in entries {
        let edge = edges_by_id
            .get(entry.binding.transfer_edge_id.as_str())
            .copied()
            .ok_or(CargoError::TransferResourceBindingMismatch)?;
        check_cargo_transfer_compatibility(
            entry.profile,
            entry.instance,
            edge,
            entry.equipment,
            entry.binding,
        )?;
    }

    Ok(report)
}

/// Full result for a plan evaluated against evidence-bearing capacity inputs.
/// The evidence is preserved so downstream receipts/replay can bind the result
/// to exact input identities. “Freshness-qualified” is not source authentication.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoTransferPlanEvidenceReport {
    pub capacity_report: CargoTransferCapacityReport,
    pub freshness_qualified_budget_evidence: Vec<CargoTransferCapacityEvidenceV1>,
}

/// Validate a proposed transfer plan using evidence-bearing equipment budgets.
/// Every supplied budget must be referenced by an assignment and pass the
/// explicit freshness policy before the combined physical/capacity gate runs.
/// The result preserves the precise evidence inputs in deterministic order.
pub fn validate_cargo_transfer_plan_with_evidence(
    entries: &[CargoTransferPlanEntry<'_>],
    edges: &[Edge],
    equipment_budgets: &[CargoTransferCapacityEvidenceV1],
    evaluation_time_epoch_seconds: u64,
    policy: CargoTransferCapacityFreshnessPolicy,
) -> Result<CargoTransferPlanEvidenceReport, CargoTransferPlanEvidenceError> {
    let mut budget_ids = BTreeSet::new();
    let referenced_resources: BTreeSet<_> = entries
        .iter()
        .map(|entry| {
            (
                entry.binding.equipment_id.as_str(),
                entry.binding.capacity_scope_id.as_str(),
            )
        })
        .collect();
    let mut capacities = Vec::with_capacity(equipment_budgets.len());
    let mut accepted_evidence = Vec::with_capacity(equipment_budgets.len());

    for evidence in equipment_budgets {
        validate_cargo_transfer_capacity_evidence(evidence, evaluation_time_epoch_seconds, policy)?;
        if !budget_ids.insert(evidence.budget_id.as_str()) {
            return Err(CargoTransferCapacityEvidenceError::DuplicateBudgetIdentity.into());
        }
        if !referenced_resources.contains(&(
            evidence.budget.equipment_id.as_str(),
            evidence.budget.capacity_scope_id.as_str(),
        )) {
            return Err(CargoTransferCapacityEvidenceError::UnreferencedBudgetEvidence.into());
        }
        capacities.push(evidence.budget.clone());
        accepted_evidence.push(evidence.clone());
    }

    capacities.sort_by(|left, right| {
        (&left.equipment_id, &left.capacity_scope_id)
            .cmp(&(&right.equipment_id, &right.capacity_scope_id))
    });
    accepted_evidence.sort_by(|left, right| {
        (
            left.budget.capacity_scope_id.as_str(),
            left.budget.equipment_id.as_str(),
            left.budget_id.as_str(),
            left.revision,
        )
            .cmp(&(
                right.budget.capacity_scope_id.as_str(),
                right.budget.equipment_id.as_str(),
                right.budget_id.as_str(),
                right.revision,
            ))
    });

    let capacity_report = validate_cargo_transfer_plan(entries, edges, &capacities)?;
    Ok(CargoTransferPlanEvidenceReport {
        capacity_report,
        freshness_qualified_budget_evidence: accepted_evidence,
    })
}

/// Validate one module against one explicit intermodal transfer edge, the
/// selected terminal equipment, and a resource binding. Movement-edge/fleet
/// compatibility is checked separately with check_module_compatibility; this
/// function does not schedule equipment, allocate shared capacity across
/// multiple requests, or perform safety-critical load calculations.
pub fn check_cargo_transfer_compatibility(
    profile: &CargoModuleProfile,
    instance: &CargoModuleInstance,
    edge: &Edge,
    equipment: &CargoTransferEquipmentProfile,
    binding: &CargoTransferResourceBinding,
) -> Result<(), CargoError> {
    let (from_mode, to_mode) = match edge.operation {
        EdgeOperation::Transfer { from, to } => (from, to),
        EdgeOperation::Movement { .. } => return Err(CargoError::TransferEdgeRequired),
    };
    equipment.validate()?;
    if edge.from != edge.to || edge.from != equipment.terminal_id {
        return Err(CargoError::TransferTerminalMismatch);
    }
    if edge.capacity == 0 {
        return Err(CargoError::TransferCapacityUnavailable);
    }
    if binding.module_id.trim().is_empty()
        || binding.module_profile_id.trim().is_empty()
        || binding.transfer_edge_id.trim().is_empty()
        || binding.equipment_id.trim().is_empty()
        || binding.terminal_id.trim().is_empty()
        || binding.capacity_units == 0
    {
        return Err(CargoError::TransferResourceBindingInvalid);
    }
    if binding.module_id != instance.module_id
        || binding.module_profile_id != profile.profile_id
        || binding.transfer_edge_id != edge.id
        || binding.equipment_id != equipment.equipment_id
        || binding.terminal_id != equipment.terminal_id
    {
        return Err(CargoError::TransferResourceBindingMismatch);
    }
    if binding.capacity_unit != edge.capacity_unit {
        return Err(CargoError::TransferCapacityUnitMismatch);
    }
    if binding.capacity_units > edge.capacity {
        return Err(CargoError::TransferCapacityExceeded);
    }
    if !equipment
        .supported_mode_pairs
        .contains(&(from_mode, to_mode))
    {
        return Err(CargoError::TransferModePairUnsupported);
    }
    if !profile.permitted_modes.contains(&from_mode) || !profile.permitted_modes.contains(&to_mode)
    {
        return Err(CargoError::ModeNotPermitted);
    }

    let gross_mass = instance.validate_against(profile)?;
    if profile.conformity == ProfileConformity::NonConforming {
        return Err(CargoError::ProfileNonConforming);
    }
    if !equipment.supported_families.contains(&profile.family) {
        return Err(CargoError::FamilyNotSupported);
    }
    if !profile
        .interfaces
        .iter()
        .any(|interface| equipment.interfaces.contains(interface))
    {
        return Err(CargoError::HandlingInterfaceMismatch);
    }
    let compatible_orientations: Vec<_> = profile
        .permitted_orientations
        .intersection(&equipment.accepted_orientations)
        .copied()
        .collect();
    if compatible_orientations.is_empty() {
        return Err(CargoError::OrientationNotSupported);
    }
    if !compatible_orientations.iter().any(|orientation| {
        orientation
            .permute_dimensions(profile.outer_dimensions_mm)
            .iter()
            .zip(equipment.max_module_dimensions_mm)
            .all(|(module, limit)| *module <= limit)
    }) {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingConstraint {
    Mass,
    Volume,
    MassAndVolume,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CargoUtilization {
    /// Integer basis points: 10_000 means 100%.
    pub mass_basis_points: u16,
    /// Integer basis points: 10_000 means 100%.
    pub volume_basis_points: u16,
    /// The more-used dimension is the constraint that will fill first.
    pub binding_constraint: BindingConstraint,
}

impl CargoModuleInstance {
    pub fn utilization_against(
        &self,
        profile: &CargoModuleProfile,
    ) -> Result<CargoUtilization, CargoError> {
        self.validate_against(profile)?;
        if profile.max_payload_mass_grams == 0 || profile.usable_volume_litres == 0 {
            return Err(CargoError::InvalidProfile(
                "payload and usable-volume ratings must be positive for utilization",
            ));
        }

        let mass_basis_points = (u128::from(self.payload_mass_grams) * 10_000
            / u128::from(profile.max_payload_mass_grams)) as u16;
        let volume_basis_points = (u128::from(self.occupied_volume_litres) * 10_000
            / u128::from(profile.usable_volume_litres)) as u16;
        let mass_used =
            u128::from(self.payload_mass_grams) * u128::from(profile.usable_volume_litres);
        let volume_used =
            u128::from(self.occupied_volume_litres) * u128::from(profile.max_payload_mass_grams);

        let binding_constraint = match mass_used.cmp(&volume_used) {
            std::cmp::Ordering::Greater => BindingConstraint::Mass,
            std::cmp::Ordering::Less => BindingConstraint::Volume,
            std::cmp::Ordering::Equal => BindingConstraint::MassAndVolume,
        };

        Ok(CargoUtilization {
            mass_basis_points,
            volume_basis_points,
            binding_constraint,
        })
    }
}

/// Lifecycle economics for a reusable module, per declared planning scenario.
/// Monetary values are integer minor currency units; empty return energy and
/// emissions are counted explicitly. This is a scenario calculator, not a
/// statistical forecast or evidence of actual savings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReusableModuleEconomics {
    pub module_profile_id: String,
    pub currency_code: String,
    pub planned_delivery_cycles: u64,
    pub acquisition_cost_minor_units: u64,
    pub replacement_cost_minor_units: u64,
    pub cleaning_and_inspection_cost_per_cycle_minor_units: u64,
    pub repair_cost_per_cycle_minor_units: u64,
    pub empty_reposition_cost_per_cycle_minor_units: u64,
    pub baseline_handling_cost_avoided_per_cycle_minor_units: u64,
    pub expected_module_loss_rate_parts_per_million: u32,
    pub empty_return_energy_wh_per_cycle: u64,
    pub empty_return_ghg_grams_co2e_per_cycle: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReusableModuleEconomicsResult {
    pub planned_delivery_cycles: u64,
    /// Conservative integer replacement allowance: fractional expected losses
    /// round up so a plan cannot under-budget replacements by truncation.
    pub replacement_allowance: u64,
    pub lifecycle_reusable_cost_minor_units: u64,
    pub baseline_handling_cost_avoided_minor_units: u64,
    /// Positive means the reusable plan costs more than the declared baseline;
    /// negative means a modeled saving, not a proven real-world saving.
    pub net_incremental_cost_minor_units: i128,
    pub average_reusable_cost_per_delivery_minor_units: u64,
    pub empty_return_energy_wh: u64,
    pub empty_return_ghg_grams_co2e: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CargoEconomicsError {
    InvalidInput(&'static str),
    ArithmeticOverflow,
}

impl fmt::Display for CargoEconomicsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(reason) => write!(f, "invalid reusable-module economics: {reason}"),
            Self::ArithmeticOverflow => write!(f, "reusable-module economics overflowed"),
        }
    }
}

impl std::error::Error for CargoEconomicsError {}

pub fn estimate_reusable_module_economics(
    input: &ReusableModuleEconomics,
) -> Result<ReusableModuleEconomicsResult, CargoEconomicsError> {
    if input.module_profile_id.trim().is_empty() {
        return Err(CargoEconomicsError::InvalidInput(
            "module profile identity is required",
        ));
    }
    if input.currency_code.len() != 3
        || !input
            .currency_code
            .bytes()
            .all(|byte| byte.is_ascii_uppercase())
    {
        return Err(CargoEconomicsError::InvalidInput(
            "currency code must be three uppercase ASCII letters",
        ));
    }
    if input.planned_delivery_cycles == 0 {
        return Err(CargoEconomicsError::InvalidInput(
            "planned delivery cycles must be positive",
        ));
    }
    if input.expected_module_loss_rate_parts_per_million > 1_000_000 {
        return Err(CargoEconomicsError::InvalidInput(
            "module loss rate cannot exceed one million parts per million",
        ));
    }

    let cycles = input.planned_delivery_cycles;
    let loss_numerator = u128::from(cycles)
        .checked_mul(u128::from(
            input.expected_module_loss_rate_parts_per_million,
        ))
        .ok_or(CargoEconomicsError::ArithmeticOverflow)?;
    let replacement_allowance_u128 = loss_numerator
        .checked_add(999_999)
        .ok_or(CargoEconomicsError::ArithmeticOverflow)?
        / 1_000_000;
    let replacement_allowance = u64::try_from(replacement_allowance_u128)
        .map_err(|_| CargoEconomicsError::ArithmeticOverflow)?;

    let per_cycle_operating_cost = input
        .cleaning_and_inspection_cost_per_cycle_minor_units
        .checked_add(input.repair_cost_per_cycle_minor_units)
        .and_then(|total| total.checked_add(input.empty_reposition_cost_per_cycle_minor_units))
        .ok_or(CargoEconomicsError::ArithmeticOverflow)?;
    let operating_cost = per_cycle_operating_cost
        .checked_mul(cycles)
        .ok_or(CargoEconomicsError::ArithmeticOverflow)?;
    let replacement_cost = input
        .replacement_cost_minor_units
        .checked_mul(replacement_allowance)
        .ok_or(CargoEconomicsError::ArithmeticOverflow)?;
    let lifecycle_cost = input
        .acquisition_cost_minor_units
        .checked_add(replacement_cost)
        .and_then(|total| total.checked_add(operating_cost))
        .ok_or(CargoEconomicsError::ArithmeticOverflow)?;
    let avoided_baseline_cost = input
        .baseline_handling_cost_avoided_per_cycle_minor_units
        .checked_mul(cycles)
        .ok_or(CargoEconomicsError::ArithmeticOverflow)?;
    let quotient = lifecycle_cost / cycles;
    let average_cost = if lifecycle_cost % cycles == 0 {
        quotient
    } else {
        quotient + 1
    };
    let empty_return_energy_wh = input
        .empty_return_energy_wh_per_cycle
        .checked_mul(cycles)
        .ok_or(CargoEconomicsError::ArithmeticOverflow)?;
    let empty_return_ghg_grams_co2e = input
        .empty_return_ghg_grams_co2e_per_cycle
        .checked_mul(cycles)
        .ok_or(CargoEconomicsError::ArithmeticOverflow)?;

    Ok(ReusableModuleEconomicsResult {
        planned_delivery_cycles: cycles,
        replacement_allowance,
        lifecycle_reusable_cost_minor_units: lifecycle_cost,
        baseline_handling_cost_avoided_minor_units: avoided_baseline_cost,
        net_incremental_cost_minor_units: i128::from(lifecycle_cost)
            - i128::from(avoided_baseline_cost),
        average_reusable_cost_per_delivery_minor_units: average_cost,
        empty_return_energy_wh,
        empty_return_ghg_grams_co2e,
    })
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
    AircraftUldIncompatible,
    OrientationNotSupported,
    LoadSecuringEvidenceMissing,
    LoadSecuringFailed,
    FamilyNotSupported,
    HandlingInterfaceMismatch,
    DimensionsExceeded,
    EquipmentGrossMassExceeded,
    ModulePowerUnavailable,
    TemperatureControlUnavailable,
    TransferEdgeRequired,
    TransferTerminalMismatch,
    TransferModePairUnsupported,
    TransferCapacityUnavailable,
    TransferResourceBindingInvalid,
    TransferResourceBindingMismatch,
    TransferCapacityUnitMismatch,
    TransferCapacityExceeded,
    TransferAssignmentIdDuplicate,
    TransferCapacityBudgetMissing,
    TransferCapacityBudgetDuplicate,
    TransferEquipmentCapacityExceeded,
    TransferCapacityArithmeticOverflow,
    TransferCapacityPlanInvalid,
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
                write!(
                    f,
                    "temperature-controlled module has no current temperature evidence"
                )
            }
            Self::TemperatureOutOfRange => {
                write!(f, "observed cargo temperature is outside the required band")
            }
            Self::ModeNotPermitted => {
                write!(f, "module profile does not permit this transport mode")
            }
            Self::ProfileNonConforming => write!(f, "module profile is marked nonconforming"),
            Self::AircraftUldNotVerified => {
                write!(f, "air transport requires a verified aircraft ULD profile")
            }
            Self::AircraftUldIncompatible => {
                write!(
                    f,
                    "aircraft ULD evidence does not match the aircraft position"
                )
            }
            Self::OrientationNotSupported => {
                write!(
                    f,
                    "module orientation is not supported by handling equipment"
                )
            }
            Self::LoadSecuringEvidenceMissing => {
                write!(
                    f,
                    "required load-securing evidence is missing or unverified"
                )
            }
            Self::LoadSecuringFailed => write!(f, "cargo load-securing inspection failed"),
            Self::FamilyNotSupported => write!(f, "equipment does not support this module family"),
            Self::HandlingInterfaceMismatch => {
                write!(
                    f,
                    "module handling/locking interfaces do not match equipment"
                )
            }
            Self::DimensionsExceeded => {
                write!(f, "module envelope exceeds the equipment profile")
            }
            Self::EquipmentGrossMassExceeded => {
                write!(f, "module gross mass exceeds equipment position rating")
            }
            Self::ModulePowerUnavailable => write!(f, "module power requirement cannot be met"),
            Self::TemperatureControlUnavailable => {
                write!(
                    f,
                    "equipment cannot satisfy the module temperature requirement"
                )
            }
            Self::TransferEdgeRequired => {
                write!(f, "cargo transfer compatibility requires a transfer edge")
            }
            Self::TransferTerminalMismatch => {
                write!(
                    f,
                    "transfer edge terminal does not match assigned transfer equipment"
                )
            }
            Self::TransferModePairUnsupported => {
                write!(
                    f,
                    "transfer equipment does not support this transport-mode pair"
                )
            }
            Self::TransferCapacityUnavailable => {
                write!(f, "transfer edge has no declared positive capacity")
            }
            Self::TransferResourceBindingInvalid => {
                write!(
                    f,
                    "transfer resource binding is incomplete or consumes zero capacity"
                )
            }
            Self::TransferResourceBindingMismatch => {
                write!(
                    f,
                    "transfer resource binding does not match the selected module, profile, edge, equipment and terminal"
                )
            }
            Self::TransferCapacityUnitMismatch => {
                write!(
                    f,
                    "transfer resource binding capacity unit does not match the edge"
                )
            }
            Self::TransferCapacityExceeded => {
                write!(
                    f,
                    "proposed transfer consumes more capacity than the edge declares"
                )
            }
            Self::TransferAssignmentIdDuplicate => {
                write!(
                    f,
                    "transfer assignment ID is duplicated in the planning batch"
                )
            }
            Self::TransferCapacityBudgetMissing => {
                write!(
                    f,
                    "transfer equipment has no capacity budget for this planning scope"
                )
            }
            Self::TransferCapacityBudgetDuplicate => {
                write!(
                    f,
                    "transfer equipment has duplicate capacity budgets for one planning scope"
                )
            }
            Self::TransferEquipmentCapacityExceeded => {
                write!(
                    f,
                    "proposed transfers exceed shared equipment capacity in the planning scope"
                )
            }
            Self::TransferCapacityArithmeticOverflow => {
                write!(
                    f,
                    "transfer capacity aggregation overflowed integer arithmetic"
                )
            }
            Self::TransferCapacityPlanInvalid => {
                write!(
                    f,
                    "transfer capacity plan contains an invalid or duplicate resource definition"
                )
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
    pub fn new<I, S>(nodes: I, currency_code: impl Into<String>) -> Result<Self, ModelError>
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
            || !self
                .currency_code
                .bytes()
                .all(|byte| byte.is_ascii_uppercase())
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
            ghg_grams_co2e: multiply(self.metrics_per_unit.ghg_grams_co2e_per_unit, quantity)?,
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

    let start_key = StateKey {
        node: demand.origin.clone(),
        current_mode: None,
    };
    let start = SearchLabel {
        node: demand.origin.clone(),
        current_mode: None,
        transfer_count: 0,
        metrics: RouteMetrics::default(),
        edge_ids: Vec::new(),
        visited_states: BTreeSet::from([start_key.clone()]),
    };
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

            // A nonnegative-cost route never needs to revisit its origin or an
            // already visited (node, mode) state. Explicitly reject such loops
            // so zero-cost cycles cannot pollute the Pareto frontier.
            if edge.to == demand.origin
                || label.visited_states.contains(&StateKey {
                    node: edge.to.clone(),
                    current_mode: next_mode,
                })
            {
                continue;
            }
            let key = StateKey {
                node: edge.to.clone(),
                current_mode: next_mode,
            };
            let mut visited_states = label.visited_states.clone();
            visited_states.insert(key.clone());
            let mut edge_ids = label.edge_ids.clone();
            edge_ids.push(edge.id.clone());
            let candidate = SearchLabel {
                node: edge.to.clone(),
                current_mode: next_mode,
                transfer_count: next_transfers,
                metrics,
                edge_ids,
                visited_states,
            };
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
    visited_states: BTreeSet<StateKey>,
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

    #[allow(clippy::too_many_arguments)]
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
    fn zero_cost_cycles_cannot_replace_a_direct_route_in_the_pareto_frontier() {
        let mut network = Network::new(["A", "B", "C"], "ZAR").unwrap();
        network.add_edge(Edge::movement(
            "a-loop-out",
            "A",
            "B",
            Mode::Road,
            10,
            QuantityUnit::Passengers,
            EdgeMetrics::default(),
        ));
        network.add_edge(Edge::movement(
            "b-loop-back",
            "B",
            "A",
            Mode::Road,
            10,
            QuantityUnit::Passengers,
            EdgeMetrics::default(),
        ));
        network.add_edge(Edge::movement(
            "z-direct",
            "A",
            "C",
            Mode::Road,
            10,
            QuantityUnit::Passengers,
            EdgeMetrics::default(),
        ));

        let mut demand = passenger_demand(1);
        demand.destination = "C".to_owned();
        let routes = pareto_routes(&network, &demand, SearchLimits::default()).unwrap();
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].edge_ids, vec!["z-direct".to_owned()]);
    }

    #[test]
    fn returns_pareto_tradeoff_instead_of_hiding_time_cost_tradeoff() {
        let network = competing_mode_network();
        let routes =
            pareto_routes(&network, &passenger_demand(1), SearchLimits::default()).unwrap();

        assert_eq!(routes.len(), 2);
        assert!(
            routes
                .iter()
                .any(|route| route.edge_ids == vec!["air-direct".to_owned()])
        );
        let intermodal = routes
            .iter()
            .find(|route| {
                route.edge_ids
                    == vec![
                        "road-feeder".to_owned(),
                        "road-rail-transfer".to_owned(),
                        "rail-linehaul".to_owned(),
                    ]
            })
            .unwrap();
        assert_eq!(intermodal.transfer_count, 1);
        assert_eq!(
            intermodal.metrics_per_unit.operator_cost_minor_per_unit,
            150
        );
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
        assert!(
            pareto_routes(&network, &passenger_demand(1), SearchLimits::default())
                .unwrap()
                .is_empty()
        );

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
        assert!(
            pareto_routes(&network, &passenger_demand(3), SearchLimits::default())
                .unwrap()
                .is_empty()
        );

        let cargo = Demand {
            id: "ore".to_owned(),
            origin: "A".to_owned(),
            destination: "B".to_owned(),
            quantity: 3,
            unit: QuantityUnit::Tonnes,
            max_elapsed_seconds: None,
            max_transfers: None,
        };
        assert!(
            pareto_routes(&network, &cargo, SearchLimits::default())
                .unwrap()
                .is_empty()
        );
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
        assert_eq!(
            network.validate(),
            Err(ModelError::UnknownNode("C".to_owned()))
        );
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
        assert_eq!(
            routes[0].edge_ids,
            vec![
                "rail-to-port".to_owned(),
                "rail-maritime-transfer".to_owned(),
                "ship-linehaul".to_owned(),
            ]
        );
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

    fn iso_container_profile() -> CargoModuleProfile {
        CargoModuleProfile {
            profile_id: "iso-series1-example-v1".to_owned(),
            family: CargoModuleFamily::IsoSeries1Container,
            conformity: ProfileConformity::Verified {
                standard_id: "ISO 668".to_owned(),
                edition: "2020".to_owned(),
                evidence_digest: "sha256:profile-evidence".to_owned(),
            },
            outer_dimensions_mm: [6_000, 2_400, 2_500],
            permitted_orientations: BTreeSet::from([CargoOrientation::XYZ]),
            usable_volume_litres: 30_000,
            tare_mass_grams: 2_000_000,
            max_payload_mass_grams: 20_000_000,
            max_gross_mass_grams: 22_000_000,
            interfaces: BTreeSet::from([
                HandlingInterface::IsoCornerFittings,
                HandlingInterface::ForkPockets,
            ]),
            permitted_modes: BTreeSet::from([Mode::Road, Mode::Rail, Mode::Maritime]),
            required_temperature_millicelsius: None,
            requires_power: false,
            requires_intact_seal: true,
            requires_verified_load_securing: true,
            aircraft_uld_eligibility: None,
        }
    }

    fn cargo_instance(profile: &CargoModuleProfile) -> CargoModuleInstance {
        CargoModuleInstance {
            module_id: "container-42".to_owned(),
            profile_id: profile.profile_id.clone(),
            payload_mass_grams: 10_000_000,
            occupied_volume_litres: 12_000,
            contents_digest: "sha256:contents".to_owned(),
            seal_status: CargoSealStatus::Intact,
            load_securing_status: LoadSecuringStatus::Verified {
                record_digest: "sha256:securing-inspection".to_owned(),
            },
            serviceability: CargoServiceability::Serviceable,
            observed_temperature_millicelsius: None,
        }
    }

    fn cargo_transfer_equipment() -> CargoTransferEquipmentProfile {
        CargoTransferEquipmentProfile {
            equipment_id: "hub-cross-dock-crane-3".to_owned(),
            terminal_id: "Hub".to_owned(),
            supported_mode_pairs: BTreeSet::from([(Mode::Road, Mode::Rail)]),
            max_module_dimensions_mm: [6_100, 2_500, 2_600],
            accepted_orientations: BTreeSet::from([CargoOrientation::XYZ]),
            max_module_gross_mass_grams: 25_000_000,
            supported_families: BTreeSet::from([CargoModuleFamily::IsoSeries1Container]),
            interfaces: BTreeSet::from([HandlingInterface::IsoCornerFittings]),
            available_temperature_millicelsius: None,
            supplies_module_power: false,
        }
    }

    fn road_rail_transfer_edge() -> Edge {
        Edge::transfer(
            "road-rail-at-hub",
            "Hub",
            Mode::Road,
            Mode::Rail,
            100,
            QuantityUnit::Shipments,
            metrics(10, 0, 120, 0, 2, 1, 0, 1, 1),
        )
    }

    fn cargo_transfer_resource_binding() -> CargoTransferResourceBinding {
        CargoTransferResourceBinding {
            assignment_id: "assignment-1".to_owned(),
            capacity_scope_id: "shift-2026-10-13-day".to_owned(),
            module_id: "container-42".to_owned(),
            module_profile_id: "iso-series1-example-v1".to_owned(),
            transfer_edge_id: "road-rail-at-hub".to_owned(),
            equipment_id: "hub-cross-dock-crane-3".to_owned(),
            terminal_id: "Hub".to_owned(),
            capacity_units: 1,
            capacity_unit: QuantityUnit::Shipments,
        }
    }

    fn rail_equipment() -> HandlingEquipmentProfile {
        HandlingEquipmentProfile {
            equipment_id: "rail-wagon-position-7".to_owned(),
            mode: Mode::Rail,
            max_module_dimensions_mm: [6_100, 2_500, 2_600],
            accepted_orientations: BTreeSet::from([CargoOrientation::XYZ]),
            max_module_gross_mass_grams: 25_000_000,
            supported_families: BTreeSet::from([CargoModuleFamily::IsoSeries1Container]),
            interfaces: BTreeSet::from([HandlingInterface::IsoCornerFittings]),
            available_temperature_millicelsius: None,
            supplies_module_power: false,
            requires_verified_aircraft_uld: false,
            aircraft_type_code: None,
            aircraft_position_id: None,
            accepted_uld_type_codes: BTreeSet::new(),
        }
    }

    #[test]
    fn cargo_transfer_accepts_explicit_mode_pair_terminal_and_interface() {
        let profile = iso_container_profile();
        let instance = cargo_instance(&profile);
        let edge = road_rail_transfer_edge();
        let equipment = cargo_transfer_equipment();
        assert_eq!(
            check_cargo_transfer_compatibility(
                &profile,
                &instance,
                &edge,
                &equipment,
                &cargo_transfer_resource_binding(),
            ),
            Ok(())
        );
    }

    #[test]
    fn combined_cargo_transfer_plan_accepts_physical_fit_within_shared_capacity() {
        let profile = iso_container_profile();
        let instance = cargo_instance(&profile);
        let edge = road_rail_transfer_edge();
        let equipment = cargo_transfer_equipment();
        let binding = cargo_transfer_resource_binding();
        let entries = [CargoTransferPlanEntry {
            profile: &profile,
            instance: &instance,
            equipment: &equipment,
            binding: &binding,
        }];
        let budgets = [equipment_capacity(
            "hub-cross-dock-crane-3",
            3,
            QuantityUnit::Shipments,
        )];

        let report = validate_cargo_transfer_plan(&entries, &[edge], &budgets).unwrap();
        assert_eq!(report.assignment_count, 1);
    }

    #[test]
    fn combined_cargo_transfer_plan_rejects_physical_mismatch_even_when_capacity_fits() {
        let profile = iso_container_profile();
        let instance = cargo_instance(&profile);
        let edge = road_rail_transfer_edge();
        let mut equipment = cargo_transfer_equipment();
        equipment.supported_mode_pairs = BTreeSet::from([(Mode::Rail, Mode::Maritime)]);
        let binding = cargo_transfer_resource_binding();
        let budgets = [equipment_capacity(
            "hub-cross-dock-crane-3",
            3,
            QuantityUnit::Shipments,
        )];
        assert!(
            validate_cargo_transfer_capacity_plan(
                std::slice::from_ref(&binding),
                std::slice::from_ref(&edge),
                &budgets,
            )
            .is_ok()
        );
        let entries = [CargoTransferPlanEntry {
            profile: &profile,
            instance: &instance,
            equipment: &equipment,
            binding: &binding,
        }];

        assert_eq!(
            validate_cargo_transfer_plan(&entries, &[edge], &budgets),
            Err(CargoError::TransferModePairUnsupported)
        );
    }

    #[test]
    fn cargo_transfer_rejects_wrong_terminal_and_unsupported_mode_pair() {
        let profile = iso_container_profile();
        let instance = cargo_instance(&profile);
        let edge = road_rail_transfer_edge();

        let mut equipment = cargo_transfer_equipment();
        equipment.terminal_id = "Different-terminal".to_owned();
        assert_eq!(
            check_cargo_transfer_compatibility(
                &profile,
                &instance,
                &edge,
                &equipment,
                &cargo_transfer_resource_binding(),
            ),
            Err(CargoError::TransferTerminalMismatch)
        );

        let mut equipment = cargo_transfer_equipment();
        equipment.supported_mode_pairs = BTreeSet::from([(Mode::Rail, Mode::Maritime)]);
        assert_eq!(
            check_cargo_transfer_compatibility(
                &profile,
                &instance,
                &edge,
                &equipment,
                &cargo_transfer_resource_binding(),
            ),
            Err(CargoError::TransferModePairUnsupported)
        );
    }

    #[test]
    fn cargo_transfer_rejects_movement_edges_and_zero_capacity() {
        let profile = iso_container_profile();
        let instance = cargo_instance(&profile);
        let equipment = cargo_transfer_equipment();

        let movement = Edge::movement(
            "road-movement",
            "Hub",
            "Market",
            Mode::Road,
            100,
            QuantityUnit::Shipments,
            metrics(10, 0, 120, 5_000, 2, 1, 0, 1, 1),
        );
        assert_eq!(
            check_cargo_transfer_compatibility(
                &profile,
                &instance,
                &movement,
                &equipment,
                &cargo_transfer_resource_binding(),
            ),
            Err(CargoError::TransferEdgeRequired)
        );

        let mut no_capacity = road_rail_transfer_edge();
        no_capacity.capacity = 0;
        assert_eq!(
            check_cargo_transfer_compatibility(
                &profile,
                &instance,
                &no_capacity,
                &equipment,
                &cargo_transfer_resource_binding(),
            ),
            Err(CargoError::TransferCapacityUnavailable)
        );
    }

    #[test]
    fn cargo_transfer_rejects_missing_interface_and_equipment_overload() {
        let profile = iso_container_profile();
        let instance = cargo_instance(&profile);
        let edge = road_rail_transfer_edge();

        let mut equipment = cargo_transfer_equipment();
        equipment.interfaces = BTreeSet::from([HandlingInterface::TankCoupling]);
        assert_eq!(
            check_cargo_transfer_compatibility(
                &profile,
                &instance,
                &edge,
                &equipment,
                &cargo_transfer_resource_binding(),
            ),
            Err(CargoError::HandlingInterfaceMismatch)
        );

        let mut equipment = cargo_transfer_equipment();
        equipment.max_module_gross_mass_grams = 1;
        assert_eq!(
            check_cargo_transfer_compatibility(
                &profile,
                &instance,
                &edge,
                &equipment,
                &cargo_transfer_resource_binding(),
            ),
            Err(CargoError::EquipmentGrossMassExceeded)
        );
    }

    #[test]
    fn cargo_transfer_rejects_stale_resource_binding_units_and_capacity() {
        let profile = iso_container_profile();
        let instance = cargo_instance(&profile);
        let edge = road_rail_transfer_edge();
        let equipment = cargo_transfer_equipment();

        let mut binding = cargo_transfer_resource_binding();
        binding.transfer_edge_id = "different-edge".to_owned();
        assert_eq!(
            check_cargo_transfer_compatibility(&profile, &instance, &edge, &equipment, &binding),
            Err(CargoError::TransferResourceBindingMismatch)
        );

        let mut binding = cargo_transfer_resource_binding();
        binding.module_id = "different-container".to_owned();
        assert_eq!(
            check_cargo_transfer_compatibility(&profile, &instance, &edge, &equipment, &binding),
            Err(CargoError::TransferResourceBindingMismatch)
        );

        let mut binding = cargo_transfer_resource_binding();
        binding.module_profile_id = "different-profile".to_owned();
        assert_eq!(
            check_cargo_transfer_compatibility(&profile, &instance, &edge, &equipment, &binding),
            Err(CargoError::TransferResourceBindingMismatch)
        );

        let mut binding = cargo_transfer_resource_binding();
        binding.equipment_id = "different-equipment".to_owned();
        assert_eq!(
            check_cargo_transfer_compatibility(&profile, &instance, &edge, &equipment, &binding),
            Err(CargoError::TransferResourceBindingMismatch)
        );

        let mut binding = cargo_transfer_resource_binding();
        binding.terminal_id = "different-terminal".to_owned();
        assert_eq!(
            check_cargo_transfer_compatibility(&profile, &instance, &edge, &equipment, &binding),
            Err(CargoError::TransferResourceBindingMismatch)
        );

        let mut binding = cargo_transfer_resource_binding();
        binding.equipment_id.clear();
        assert_eq!(
            check_cargo_transfer_compatibility(&profile, &instance, &edge, &equipment, &binding),
            Err(CargoError::TransferResourceBindingInvalid)
        );

        let mut binding = cargo_transfer_resource_binding();
        binding.capacity_unit = QuantityUnit::Teu;
        assert_eq!(
            check_cargo_transfer_compatibility(&profile, &instance, &edge, &equipment, &binding),
            Err(CargoError::TransferCapacityUnitMismatch)
        );

        let mut binding = cargo_transfer_resource_binding();
        binding.capacity_units = 101;
        assert_eq!(
            check_cargo_transfer_compatibility(&profile, &instance, &edge, &equipment, &binding),
            Err(CargoError::TransferCapacityExceeded)
        );

        let mut binding = cargo_transfer_resource_binding();
        binding.capacity_units = 0;
        assert_eq!(
            check_cargo_transfer_compatibility(&profile, &instance, &edge, &equipment, &binding),
            Err(CargoError::TransferResourceBindingInvalid)
        );
    }

    fn second_road_rail_transfer_edge() -> Edge {
        Edge::transfer(
            "road-rail-at-hub-2",
            "Hub",
            Mode::Road,
            Mode::Rail,
            2,
            QuantityUnit::Shipments,
            metrics(10, 0, 120, 0, 2, 1, 0, 1, 1),
        )
    }

    fn equipment_capacity(
        equipment_id: &str,
        capacity_units: u64,
        capacity_unit: QuantityUnit,
    ) -> CargoTransferEquipmentCapacity {
        CargoTransferEquipmentCapacity {
            equipment_id: equipment_id.to_owned(),
            terminal_id: "Hub".to_owned(),
            capacity_scope_id: "shift-2026-10-13-day".to_owned(),
            capacity_units,
            capacity_unit,
        }
    }

    fn capacity_evidence(
        budget: CargoTransferEquipmentCapacity,
        observed_at: Option<u64>,
    ) -> CargoTransferCapacityEvidenceV1 {
        CargoTransferCapacityEvidenceV1 {
            budget_id: "budget-crane-3-shift-a-r3".to_owned(),
            budget,
            authority_id: "hub-operator-1".to_owned(),
            authority_profile_id: "terminal-capacity-authority-v1".to_owned(),
            source_record_id: "snapshot-91".to_owned(),
            source_schema_profile_id: "capacity-snapshot-v1".to_owned(),
            source_record_digest: "sha256:capacity-snapshot-91".to_owned(),
            revision: 3,
            supersedes_revision: Some(2),
            source_event_time_epoch_seconds: Some(985),
            capacity_observed_at_epoch_seconds: observed_at,
            repository_recorded_at_epoch_seconds: Some(995),
            valid_from_epoch_seconds: Some(900),
            valid_until_epoch_seconds: Some(1_200),
            time_profile_id: "unix-epoch-utc-seconds-v1".to_owned(),
            status: CargoTransferCapacityEvidenceStatus::ReportedCurrent,
        }
    }

    fn capacity_freshness_policy() -> CargoTransferCapacityFreshnessPolicy {
        CargoTransferCapacityFreshnessPolicy {
            expected_time_profile_id: "unix-epoch-utc-seconds-v1",
            max_observation_age_seconds: 100,
            allowed_future_clock_skew_seconds: 5,
        }
    }

    fn second_transfer_binding() -> CargoTransferResourceBinding {
        CargoTransferResourceBinding {
            assignment_id: "assignment-2".to_owned(),
            capacity_scope_id: "shift-2026-10-13-day".to_owned(),
            module_id: "container-43".to_owned(),
            module_profile_id: "iso-series1-example-v1".to_owned(),
            transfer_edge_id: "road-rail-at-hub-2".to_owned(),
            equipment_id: "hub-cross-dock-crane-3".to_owned(),
            terminal_id: "Hub".to_owned(),
            capacity_units: 1,
            capacity_unit: QuantityUnit::Shipments,
        }
    }

    #[test]
    fn capacity_evidence_accepts_fresh_observation_and_requires_separate_observation_time() {
        let budget = equipment_capacity("hub-cross-dock-crane-3", 3, QuantityUnit::Shipments);
        let evidence = capacity_evidence(budget, Some(980));
        assert_eq!(
            validate_cargo_transfer_capacity_evidence(
                &evidence,
                1_000,
                capacity_freshness_policy(),
            ),
            Ok(())
        );

        let missing_observation = capacity_evidence(evidence.budget.clone(), None);
        assert_eq!(
            validate_cargo_transfer_capacity_evidence(
                &missing_observation,
                1_000,
                capacity_freshness_policy(),
            ),
            Err(CargoTransferCapacityEvidenceError::MissingObservationTime)
        );

        let mut wrong_time_profile = capacity_evidence(evidence.budget.clone(), Some(980));
        wrong_time_profile.time_profile_id = "local-clock-unspecified".to_owned();
        assert_eq!(
            validate_cargo_transfer_capacity_evidence(
                &wrong_time_profile,
                1_000,
                capacity_freshness_policy(),
            ),
            Err(CargoTransferCapacityEvidenceError::TimeProfileMismatch)
        );
    }

    #[test]
    fn capacity_evidence_rejects_expired_old_future_and_malformed_inputs() {
        let budget = equipment_capacity("hub-cross-dock-crane-3", 3, QuantityUnit::Shipments);

        let expired = capacity_evidence(budget.clone(), Some(980));
        assert_eq!(
            validate_cargo_transfer_capacity_evidence(&expired, 1_200, capacity_freshness_policy(),),
            Err(CargoTransferCapacityEvidenceError::Expired)
        );

        let old = capacity_evidence(budget.clone(), Some(800));
        assert_eq!(
            validate_cargo_transfer_capacity_evidence(&old, 1_000, capacity_freshness_policy()),
            Err(CargoTransferCapacityEvidenceError::ObservationTooOld)
        );

        let future = capacity_evidence(budget.clone(), Some(1_010));
        assert_eq!(
            validate_cargo_transfer_capacity_evidence(&future, 1_000, capacity_freshness_policy(),),
            Err(CargoTransferCapacityEvidenceError::TimestampInFuture)
        );

        let mut invalid_interval = capacity_evidence(budget, Some(980));
        invalid_interval.valid_until_epoch_seconds = Some(900);
        assert_eq!(
            validate_cargo_transfer_capacity_evidence(
                &invalid_interval,
                1_000,
                capacity_freshness_policy(),
            ),
            Err(CargoTransferCapacityEvidenceError::InvalidValidityInterval)
        );
    }

    #[test]
    fn capacity_evidence_rejects_revoked_conflicted_and_invalid_revision_lineage() {
        let budget = equipment_capacity("hub-cross-dock-crane-3", 3, QuantityUnit::Shipments);
        let mut revoked = capacity_evidence(budget.clone(), Some(980));
        revoked.status = CargoTransferCapacityEvidenceStatus::Revoked;
        assert_eq!(
            validate_cargo_transfer_capacity_evidence(&revoked, 1_000, capacity_freshness_policy(),),
            Err(CargoTransferCapacityEvidenceError::NotCurrent)
        );

        let mut conflicted = capacity_evidence(budget.clone(), Some(980));
        conflicted.status = CargoTransferCapacityEvidenceStatus::Conflicted;
        assert_eq!(
            validate_cargo_transfer_capacity_evidence(
                &conflicted,
                1_000,
                capacity_freshness_policy(),
            ),
            Err(CargoTransferCapacityEvidenceError::NotCurrent)
        );

        let mut invalid_revision = capacity_evidence(budget, Some(980));
        invalid_revision.supersedes_revision = Some(3);
        assert_eq!(
            validate_cargo_transfer_capacity_evidence(
                &invalid_revision,
                1_000,
                capacity_freshness_policy(),
            ),
            Err(CargoTransferCapacityEvidenceError::InvalidRevisionLineage)
        );
    }

    #[test]
    fn plan_with_capacity_evidence_preserves_provenance_in_deterministic_report() {
        let profile = iso_container_profile();
        let instance = cargo_instance(&profile);
        let edge = road_rail_transfer_edge();
        let equipment = cargo_transfer_equipment();
        let binding = cargo_transfer_resource_binding();
        let entries = [CargoTransferPlanEntry {
            profile: &profile,
            instance: &instance,
            equipment: &equipment,
            binding: &binding,
        }];
        let budget = equipment_capacity("hub-cross-dock-crane-3", 3, QuantityUnit::Shipments);
        let evidence = capacity_evidence(budget, Some(980));
        let report = validate_cargo_transfer_plan_with_evidence(
            &entries,
            &[edge],
            std::slice::from_ref(&evidence),
            1_000,
            capacity_freshness_policy(),
        )
        .unwrap();

        assert_eq!(report.capacity_report.assignment_count, 1);
        assert_eq!(
            report.freshness_qualified_budget_evidence,
            vec![evidence.clone()]
        );
        let retained = &report.freshness_qualified_budget_evidence[0];
        assert_eq!(retained.source_event_time_epoch_seconds, Some(985));
        assert_eq!(retained.capacity_observed_at_epoch_seconds, Some(980));
        assert_eq!(retained.repository_recorded_at_epoch_seconds, Some(995));
        assert_eq!(retained.source_record_digest, "sha256:capacity-snapshot-91");
    }

    #[test]
    fn plan_with_capacity_evidence_rejects_stale_budget_even_when_numeric_plan_fits() {
        let profile = iso_container_profile();
        let instance = cargo_instance(&profile);
        let edge = road_rail_transfer_edge();
        let equipment = cargo_transfer_equipment();
        let binding = cargo_transfer_resource_binding();
        let entries = [CargoTransferPlanEntry {
            profile: &profile,
            instance: &instance,
            equipment: &equipment,
            binding: &binding,
        }];
        let budget = equipment_capacity("hub-cross-dock-crane-3", 3, QuantityUnit::Shipments);
        assert!(
            validate_cargo_transfer_plan(
                &entries,
                std::slice::from_ref(&edge),
                std::slice::from_ref(&budget),
            )
            .is_ok()
        );

        let stale_evidence = capacity_evidence(budget, Some(800));
        assert_eq!(
            validate_cargo_transfer_plan_with_evidence(
                &entries,
                &[edge],
                &[stale_evidence],
                1_000,
                capacity_freshness_policy(),
            ),
            Err(CargoTransferPlanEvidenceError::Evidence(
                CargoTransferCapacityEvidenceError::ObservationTooOld,
            ))
        );
    }

    #[test]
    fn plan_with_capacity_evidence_rejects_duplicate_budget_ids_and_not_yet_valid() {
        let profile = iso_container_profile();
        let instance = cargo_instance(&profile);
        let edge = road_rail_transfer_edge();
        let equipment = cargo_transfer_equipment();
        let binding = cargo_transfer_resource_binding();
        let entries = [CargoTransferPlanEntry {
            profile: &profile,
            instance: &instance,
            equipment: &equipment,
            binding: &binding,
        }];
        let budget = equipment_capacity("hub-cross-dock-crane-3", 3, QuantityUnit::Shipments);
        let evidence = capacity_evidence(budget.clone(), Some(980));
        assert_eq!(
            validate_cargo_transfer_plan_with_evidence(
                &entries,
                std::slice::from_ref(&edge),
                &[evidence.clone(), evidence],
                1_000,
                capacity_freshness_policy(),
            ),
            Err(CargoTransferPlanEvidenceError::Evidence(
                CargoTransferCapacityEvidenceError::DuplicateBudgetIdentity,
            ))
        );

        let mut early = capacity_evidence(budget, Some(980));
        early.valid_from_epoch_seconds = Some(1_001);
        assert_eq!(
            validate_cargo_transfer_plan_with_evidence(
                &entries,
                std::slice::from_ref(&edge),
                &[early],
                1_000,
                capacity_freshness_policy(),
            ),
            Err(CargoTransferPlanEvidenceError::Evidence(
                CargoTransferCapacityEvidenceError::NotYetValid,
            ))
        );

        let unrelated = capacity_evidence(
            equipment_capacity("unreferenced-crane", 3, QuantityUnit::Shipments),
            Some(980),
        );
        assert_eq!(
            validate_cargo_transfer_plan_with_evidence(
                &entries,
                &[edge],
                &[unrelated],
                1_000,
                capacity_freshness_policy(),
            ),
            Err(CargoTransferPlanEvidenceError::Evidence(
                CargoTransferCapacityEvidenceError::UnreferencedBudgetEvidence,
            ))
        );
    }

    #[test]
    fn cargo_transfer_capacity_plan_accepts_shared_resource_within_budget() {
        let mut first = cargo_transfer_resource_binding();
        first.capacity_units = 1;
        let second = second_transfer_binding();
        let budget = equipment_capacity("hub-cross-dock-crane-3", 2, QuantityUnit::Shipments);
        let report = validate_cargo_transfer_capacity_plan(
            &[first, second],
            &[road_rail_transfer_edge(), second_road_rail_transfer_edge()],
            &[budget],
        )
        .unwrap();

        assert_eq!(report.assignment_count, 2);
        let equipment_use = report
            .capacity_uses
            .iter()
            .find(|usage| usage.resource_kind == CargoTransferCapacityResourceKind::Equipment)
            .unwrap();
        assert_eq!(equipment_use.capacity_units_used, 2);
        assert_eq!(equipment_use.capacity_units_limit, 2);
        assert_eq!(equipment_use.capacity_unit, QuantityUnit::Shipments);
    }

    #[test]
    fn cargo_transfer_capacity_plan_rejects_shared_equipment_overcommit_across_edges() {
        let first = cargo_transfer_resource_binding();
        let mut second = second_transfer_binding();
        second.capacity_units = 1;
        let budget = equipment_capacity("hub-cross-dock-crane-3", 1, QuantityUnit::Shipments);
        assert_eq!(
            validate_cargo_transfer_capacity_plan(
                &[first, second],
                &[road_rail_transfer_edge(), second_road_rail_transfer_edge()],
                &[budget],
            ),
            Err(CargoError::TransferEquipmentCapacityExceeded)
        );
    }

    #[test]
    fn cargo_transfer_capacity_plan_rejects_edge_overcommit_and_duplicate_assignment() {
        let first = cargo_transfer_resource_binding();
        let mut second = cargo_transfer_resource_binding();
        second.assignment_id = "assignment-2".to_owned();
        second.module_id = "container-43".to_owned();
        second.capacity_units = 1;
        let budget = equipment_capacity("hub-cross-dock-crane-3", 10, QuantityUnit::Shipments);
        let mut edge = road_rail_transfer_edge();
        edge.capacity = 1;
        assert_eq!(
            validate_cargo_transfer_capacity_plan(
                &[first.clone(), second.clone()],
                &[edge],
                std::slice::from_ref(&budget),
            ),
            Err(CargoError::TransferCapacityExceeded)
        );

        second.assignment_id = first.assignment_id.clone();
        assert_eq!(
            validate_cargo_transfer_capacity_plan(
                &[first, second],
                &[road_rail_transfer_edge()],
                &[budget],
            ),
            Err(CargoError::TransferAssignmentIdDuplicate)
        );
    }

    #[test]
    fn cargo_transfer_capacity_plan_rejects_missing_and_ambiguous_budgets() {
        let binding = cargo_transfer_resource_binding();
        assert_eq!(
            validate_cargo_transfer_capacity_plan(
                std::slice::from_ref(&binding),
                &[road_rail_transfer_edge()],
                &[],
            ),
            Err(CargoError::TransferCapacityBudgetMissing)
        );

        let budget = equipment_capacity("hub-cross-dock-crane-3", 2, QuantityUnit::Shipments);
        assert_eq!(
            validate_cargo_transfer_capacity_plan(
                &[binding],
                &[road_rail_transfer_edge()],
                &[budget.clone(), budget],
            ),
            Err(CargoError::TransferCapacityBudgetDuplicate)
        );
    }

    #[test]
    fn cargo_transfer_capacity_plan_rejects_duplicate_edges_invalid_scope_and_wrong_terminal() {
        let binding = cargo_transfer_resource_binding();
        let edge = road_rail_transfer_edge();
        let budget = equipment_capacity("hub-cross-dock-crane-3", 2, QuantityUnit::Shipments);
        assert_eq!(
            validate_cargo_transfer_capacity_plan(
                std::slice::from_ref(&binding),
                &[edge.clone(), edge.clone()],
                std::slice::from_ref(&budget),
            ),
            Err(CargoError::TransferCapacityPlanInvalid)
        );

        let mut missing_scope = binding.clone();
        missing_scope.capacity_scope_id.clear();
        assert_eq!(
            validate_cargo_transfer_capacity_plan(
                &[missing_scope],
                std::slice::from_ref(&edge),
                std::slice::from_ref(&budget),
            ),
            Err(CargoError::TransferResourceBindingInvalid)
        );

        let mut wrong_terminal = budget.clone();
        wrong_terminal.terminal_id = "Other-terminal".to_owned();
        assert_eq!(
            validate_cargo_transfer_capacity_plan(&[binding], &[edge], &[wrong_terminal]),
            Err(CargoError::TransferResourceBindingMismatch)
        );
    }

    #[test]
    fn cargo_transfer_capacity_plan_rejects_capacity_unit_mismatch_and_overflow() {
        let mut binding = cargo_transfer_resource_binding();
        binding.capacity_unit = QuantityUnit::Teu;
        let budget = equipment_capacity("hub-cross-dock-crane-3", 2, QuantityUnit::Teu);
        assert_eq!(
            validate_cargo_transfer_capacity_plan(
                &[binding],
                &[road_rail_transfer_edge()],
                &[budget],
            ),
            Err(CargoError::TransferCapacityUnitMismatch)
        );

        let mut first = cargo_transfer_resource_binding();
        first.capacity_units = u64::MAX;
        let mut second = second_transfer_binding();
        second.capacity_units = 1;
        let huge_edge = Edge::transfer(
            "road-rail-at-hub",
            "Hub",
            Mode::Road,
            Mode::Rail,
            u64::MAX,
            QuantityUnit::Shipments,
            metrics(10, 0, 120, 0, 2, 1, 0, 1, 1),
        );
        let second_edge = Edge::transfer(
            "road-rail-at-hub-2",
            "Hub",
            Mode::Road,
            Mode::Rail,
            u64::MAX,
            QuantityUnit::Shipments,
            metrics(10, 0, 120, 0, 2, 1, 0, 1, 1),
        );
        let budget =
            equipment_capacity("hub-cross-dock-crane-3", u64::MAX, QuantityUnit::Shipments);
        assert_eq!(
            validate_cargo_transfer_capacity_plan(
                &[first, second],
                &[huge_edge, second_edge],
                &[budget],
            ),
            Err(CargoError::TransferCapacityArithmeticOverflow)
        );
    }

    #[test]
    fn module_profile_rejects_mass_rating_overflow() {
        let mut profile = iso_container_profile();
        profile.tare_mass_grams = u64::MAX;
        profile.max_payload_mass_grams = 1;
        profile.max_gross_mass_grams = u64::MAX;
        assert_eq!(
            profile.validate(),
            Err(CargoError::InvalidProfile(
                "tare plus rated payload exceeds maximum gross mass"
            ))
        );
    }

    #[test]
    fn cargo_utilization_reports_mass_and_volume_and_binding_dimension() {
        let profile = iso_container_profile();
        let mut instance = cargo_instance(&profile);
        instance.payload_mass_grams = profile.max_payload_mass_grams / 2;
        instance.occupied_volume_litres = profile.usable_volume_litres / 4;
        let utilization = instance.utilization_against(&profile).unwrap();
        assert_eq!(utilization.mass_basis_points, 5_000);
        assert_eq!(utilization.volume_basis_points, 2_500);
        assert_eq!(utilization.binding_constraint, BindingConstraint::Mass);

        instance.occupied_volume_litres = profile.usable_volume_litres / 2;
        let utilization = instance.utilization_against(&profile).unwrap();
        assert_eq!(
            utilization.binding_constraint,
            BindingConstraint::MassAndVolume
        );
    }

    fn reusable_economics_fixture() -> ReusableModuleEconomics {
        ReusableModuleEconomics {
            module_profile_id: "returnable-container-v1".to_owned(),
            currency_code: "ZAR".to_owned(),
            planned_delivery_cycles: 100,
            acquisition_cost_minor_units: 100,
            replacement_cost_minor_units: 100,
            cleaning_and_inspection_cost_per_cycle_minor_units: 5,
            repair_cost_per_cycle_minor_units: 1,
            empty_reposition_cost_per_cycle_minor_units: 20,
            baseline_handling_cost_avoided_per_cycle_minor_units: 40,
            expected_module_loss_rate_parts_per_million: 10_000,
            empty_return_energy_wh_per_cycle: 300,
            empty_return_ghg_grams_co2e_per_cycle: 10,
        }
    }

    #[test]
    fn reusable_module_economics_includes_empty_returns_and_replacement_allowance() {
        let result = estimate_reusable_module_economics(&reusable_economics_fixture()).unwrap();
        assert_eq!(result.planned_delivery_cycles, 100);
        assert_eq!(result.replacement_allowance, 1);
        assert_eq!(result.lifecycle_reusable_cost_minor_units, 2_800);
        assert_eq!(result.baseline_handling_cost_avoided_minor_units, 4_000);
        assert_eq!(result.net_incremental_cost_minor_units, -1_200);
        assert_eq!(result.average_reusable_cost_per_delivery_minor_units, 28);
        assert_eq!(result.empty_return_energy_wh, 30_000);
        assert_eq!(result.empty_return_ghg_grams_co2e, 1_000);
    }

    #[test]
    fn reusable_module_economics_rejects_invalid_inputs_and_overflow() {
        let mut input = reusable_economics_fixture();
        input.planned_delivery_cycles = 0;
        assert_eq!(
            estimate_reusable_module_economics(&input),
            Err(CargoEconomicsError::InvalidInput(
                "planned delivery cycles must be positive"
            ))
        );

        let mut input = reusable_economics_fixture();
        input.expected_module_loss_rate_parts_per_million = 1_000_001;
        assert_eq!(
            estimate_reusable_module_economics(&input),
            Err(CargoEconomicsError::InvalidInput(
                "module loss rate cannot exceed one million parts per million"
            ))
        );

        let mut input = reusable_economics_fixture();
        input.planned_delivery_cycles = 2;
        input.cleaning_and_inspection_cost_per_cycle_minor_units = u64::MAX;
        assert_eq!(
            estimate_reusable_module_economics(&input),
            Err(CargoEconomicsError::ArithmeticOverflow)
        );
    }

    #[test]
    fn reusable_module_economics_does_not_claim_savings_when_returns_are_expensive() {
        let mut input = reusable_economics_fixture();
        input.empty_reposition_cost_per_cycle_minor_units = 100;
        input.baseline_handling_cost_avoided_per_cycle_minor_units = 5;
        let result = estimate_reusable_module_economics(&input).unwrap();
        assert!(result.net_incremental_cost_minor_units > 0);
    }

    #[test]
    fn modular_cargo_accepts_matching_verified_profile_and_equipment() {
        let profile = iso_container_profile();
        let instance = cargo_instance(&profile);
        let equipment = rail_equipment();
        assert!(check_module_compatibility(&profile, &instance, &equipment).is_ok());
    }

    #[test]
    fn modular_cargo_rejects_capacity_interface_and_mode_mismatches() {
        let profile = iso_container_profile();
        let instance = cargo_instance(&profile);
        let mut equipment = rail_equipment();
        equipment.interfaces = BTreeSet::from([HandlingInterface::AircraftRestraint]);
        assert_eq!(
            check_module_compatibility(&profile, &instance, &equipment),
            Err(CargoError::HandlingInterfaceMismatch)
        );

        let mut equipment = rail_equipment();
        equipment.max_module_dimensions_mm = [5_000, 2_000, 2_000];
        assert_eq!(
            check_module_compatibility(&profile, &instance, &equipment),
            Err(CargoError::DimensionsExceeded)
        );

        let mut equipment = rail_equipment();
        equipment.max_module_gross_mass_grams = 5_000_000;
        assert_eq!(
            check_module_compatibility(&profile, &instance, &equipment),
            Err(CargoError::EquipmentGrossMassExceeded)
        );

        let mut equipment = rail_equipment();
        equipment.mode = Mode::Air;
        assert_eq!(
            check_module_compatibility(&profile, &instance, &equipment),
            Err(CargoError::ModeNotPermitted)
        );
    }

    #[test]
    fn modular_cargo_rejects_overweight_overvolume_and_unsealed_instances() {
        let profile = iso_container_profile();
        let mut instance = cargo_instance(&profile);
        instance.payload_mass_grams = profile.max_payload_mass_grams + 1;
        assert_eq!(
            instance.validate_against(&profile),
            Err(CargoError::PayloadExceeded)
        );

        let mut instance = cargo_instance(&profile);
        instance.occupied_volume_litres = profile.usable_volume_litres + 1;
        assert_eq!(
            instance.validate_against(&profile),
            Err(CargoError::VolumeExceeded)
        );

        let mut instance = cargo_instance(&profile);
        instance.seal_status = CargoSealStatus::Broken;
        assert_eq!(
            instance.validate_against(&profile),
            Err(CargoError::SealRequirementFailed)
        );
    }

    #[test]
    fn air_cargo_requires_verified_aircraft_uld_profile() {
        let mut profile = iso_container_profile();
        profile.profile_id = "air-uld-example-v1".to_owned();
        profile.family = CargoModuleFamily::AircraftUld;
        profile.conformity = ProfileConformity::DeclaredUnverified;
        profile.outer_dimensions_mm = [3_000, 2_000, 1_600];
        profile.usable_volume_litres = 8_000;
        profile.tare_mass_grams = 100_000;
        profile.max_payload_mass_grams = 500_000;
        profile.max_gross_mass_grams = 600_000;
        profile.interfaces = BTreeSet::from([HandlingInterface::AircraftRestraint]);
        profile.permitted_modes = BTreeSet::from([Mode::Air, Mode::Road]);

        let mut instance = cargo_instance(&profile);
        instance.payload_mass_grams = 100_000;
        instance.occupied_volume_litres = 2_000;
        let equipment = HandlingEquipmentProfile {
            equipment_id: "certified-aircraft-position-1".to_owned(),
            mode: Mode::Air,
            max_module_dimensions_mm: [3_100, 2_100, 1_700],
            accepted_orientations: BTreeSet::from([CargoOrientation::XYZ]),
            max_module_gross_mass_grams: 700_000,
            supported_families: BTreeSet::from([CargoModuleFamily::AircraftUld]),
            interfaces: BTreeSet::from([HandlingInterface::AircraftRestraint]),
            available_temperature_millicelsius: None,
            supplies_module_power: false,
            requires_verified_aircraft_uld: true,
            aircraft_type_code: Some("A350F".to_owned()),
            aircraft_position_id: Some("position-1".to_owned()),
            accepted_uld_type_codes: BTreeSet::from(["AMW".to_owned()]),
        };
        assert_eq!(
            check_module_compatibility(&profile, &instance, &equipment),
            Err(CargoError::AircraftUldNotVerified)
        );
    }

    #[test]
    fn cargo_compatibility_uses_only_declared_safe_orientations() {
        let mut profile = iso_container_profile();
        profile.outer_dimensions_mm = [6_000, 2_400, 2_500];
        profile.permitted_orientations =
            BTreeSet::from([CargoOrientation::XYZ, CargoOrientation::YXZ]);
        let instance = cargo_instance(&profile);
        let mut equipment = rail_equipment();
        equipment.accepted_orientations = BTreeSet::from([CargoOrientation::YXZ]);
        equipment.max_module_dimensions_mm = [2_600, 6_100, 2_500];
        assert!(check_module_compatibility(&profile, &instance, &equipment).is_ok());

        equipment.accepted_orientations = BTreeSet::from([CargoOrientation::ZXY]);
        assert_eq!(
            check_module_compatibility(&profile, &instance, &equipment),
            Err(CargoError::OrientationNotSupported)
        );
    }

    #[test]
    fn required_load_securing_evidence_fails_closed() {
        let profile = iso_container_profile();
        let mut instance = cargo_instance(&profile);
        instance.load_securing_status = LoadSecuringStatus::Unverified;
        assert_eq!(
            instance.validate_against(&profile),
            Err(CargoError::LoadSecuringEvidenceMissing)
        );

        instance.load_securing_status = LoadSecuringStatus::Failed;
        assert_eq!(
            instance.validate_against(&profile),
            Err(CargoError::LoadSecuringFailed)
        );
    }

    #[test]
    fn verified_aircraft_uld_requires_aircraft_position_specific_evidence() {
        let mut profile = iso_container_profile();
        profile.profile_id = "verified-air-uld-v1".to_owned();
        profile.family = CargoModuleFamily::AircraftUld;
        profile.conformity = ProfileConformity::Verified {
            standard_id: "IATA ULDR".to_owned(),
            edition: "2026".to_owned(),
            evidence_digest: "sha256:airworthiness-evidence".to_owned(),
        };
        profile.outer_dimensions_mm = [3_000, 2_000, 1_600];
        profile.permitted_orientations = BTreeSet::from([CargoOrientation::XYZ]);
        profile.usable_volume_litres = 8_000;
        profile.tare_mass_grams = 100_000;
        profile.max_payload_mass_grams = 500_000;
        profile.max_gross_mass_grams = 600_000;
        profile.interfaces = BTreeSet::from([HandlingInterface::AircraftRestraint]);
        profile.permitted_modes = BTreeSet::from([Mode::Air, Mode::Road]);
        profile.aircraft_uld_eligibility = Some(AircraftUldEligibilityEvidence {
            uld_type_code: "AMW".to_owned(),
            aircraft_type_code: "A350F".to_owned(),
            aircraft_position_id: "position-1".to_owned(),
            approval_reference: "operator-approved-load-manual-entry".to_owned(),
            max_gross_mass_grams: 550_000,
            evidence_digest: "sha256:uld-position-proof".to_owned(),
        });

        let mut instance = cargo_instance(&profile);
        instance.payload_mass_grams = 100_000;
        instance.occupied_volume_litres = 2_000;
        let mut equipment = HandlingEquipmentProfile {
            equipment_id: "aircraft-position-1".to_owned(),
            mode: Mode::Air,
            max_module_dimensions_mm: [3_100, 2_100, 1_700],
            accepted_orientations: BTreeSet::from([CargoOrientation::XYZ]),
            max_module_gross_mass_grams: 600_000,
            supported_families: BTreeSet::from([CargoModuleFamily::AircraftUld]),
            interfaces: BTreeSet::from([HandlingInterface::AircraftRestraint]),
            available_temperature_millicelsius: None,
            supplies_module_power: false,
            requires_verified_aircraft_uld: true,
            aircraft_type_code: Some("A350F".to_owned()),
            aircraft_position_id: Some("position-1".to_owned()),
            accepted_uld_type_codes: BTreeSet::from(["AMW".to_owned()]),
        };
        assert!(check_module_compatibility(&profile, &instance, &equipment).is_ok());

        equipment.aircraft_position_id = Some("position-2".to_owned());
        assert_eq!(
            check_module_compatibility(&profile, &instance, &equipment),
            Err(CargoError::AircraftUldIncompatible)
        );
    }

    fn cargo_custody_state() -> CargoCustodyState {
        CargoCustodyState {
            module_id: "container-42".to_owned(),
            current_custodian_id: "warehouse-a".to_owned(),
            current_location_id: "depot-a".to_owned(),
            revision: 4,
            contents_digest: "sha256:contents".to_owned(),
            seal_status: CargoSealStatus::Intact,
            last_receipt_digest: Some("sha256:previous-receipt".to_owned()),
        }
    }

    fn cargo_custody_handoff() -> CargoCustodyHandoff {
        CargoCustodyHandoff {
            module_id: "container-42".to_owned(),
            expected_revision: 4,
            from_custodian_id: "warehouse-a".to_owned(),
            to_custodian_id: "carrier-b".to_owned(),
            from_location_id: "depot-a".to_owned(),
            to_location_id: "rail-terminal-b".to_owned(),
            contents_digest_before: "sha256:contents".to_owned(),
            contents_digest_after: "sha256:contents".to_owned(),
            seal_status_before: CargoSealStatus::Intact,
            seal_status_after: CargoSealStatus::Intact,
            seal_transition_evidence_digest: None,
            receipt_digest: "sha256:handoff-receipt".to_owned(),
        }
    }

    #[test]
    fn cargo_custody_handoff_transfers_authority_exactly_one_revision() {
        let current = cargo_custody_state();
        let receipt = cargo_custody_handoff();
        let next = apply_cargo_custody_handoff(&current, &receipt).unwrap();
        assert_eq!(next.current_custodian_id, "carrier-b");
        assert_eq!(next.current_location_id, "rail-terminal-b");
        assert_eq!(next.revision, 5);
        assert_eq!(next.contents_digest, current.contents_digest);
        assert_eq!(next.seal_status, CargoSealStatus::Intact);
        assert_eq!(
            next.last_receipt_digest.as_deref(),
            Some("sha256:handoff-receipt")
        );

        assert_eq!(
            apply_cargo_custody_handoff(&next, &receipt),
            Err(CargoCustodyError::StaleRevision)
        );
    }

    #[test]
    fn cargo_custody_handoff_rejects_authority_or_contents_mismatch() {
        let current = cargo_custody_state();
        let mut receipt = cargo_custody_handoff();
        receipt.from_custodian_id = "spoofed-warehouse".to_owned();
        assert_eq!(
            apply_cargo_custody_handoff(&current, &receipt),
            Err(CargoCustodyError::AuthorityMismatch)
        );

        let mut receipt = cargo_custody_handoff();
        receipt.contents_digest_after = "sha256:changed-content".to_owned();
        assert_eq!(
            apply_cargo_custody_handoff(&current, &receipt),
            Err(CargoCustodyError::ContentsDigestMismatch)
        );
    }

    #[test]
    fn cargo_custody_seal_change_requires_explicit_evidence() {
        let current = cargo_custody_state();
        let mut receipt = cargo_custody_handoff();
        receipt.seal_status_after = CargoSealStatus::Broken;
        assert_eq!(
            apply_cargo_custody_handoff(&current, &receipt),
            Err(CargoCustodyError::SealTransitionEvidenceMissing)
        );

        receipt.seal_transition_evidence_digest = Some("sha256:seal-open-log".to_owned());
        let next = apply_cargo_custody_handoff(&current, &receipt).unwrap();
        assert_eq!(next.seal_status, CargoSealStatus::Broken);
    }

    #[test]
    fn cargo_custody_revision_overflow_fails_closed() {
        let mut current = cargo_custody_state();
        current.revision = u64::MAX;
        let mut handoff = cargo_custody_handoff();
        handoff.expected_revision = u64::MAX;
        assert_eq!(
            apply_cargo_custody_handoff(&current, &handoff),
            Err(CargoCustodyError::RevisionOverflow)
        );
    }

    #[test]
    fn cold_chain_rejects_missing_or_out_of_range_temperature_evidence() {
        let mut profile = iso_container_profile();
        profile.profile_id = "cold-chain-pod-v1".to_owned();
        profile.family = CargoModuleFamily::ColdChainPod;
        profile.requires_power = true;
        profile.required_temperature_millicelsius = Some(TemperatureBand {
            min_millicelsius: 2_000,
            max_millicelsius: 8_000,
        });
        let mut instance = cargo_instance(&profile);
        assert_eq!(
            instance.validate_against(&profile),
            Err(CargoError::TemperatureEvidenceMissing)
        );
        instance.observed_temperature_millicelsius = Some(10_000);
        assert_eq!(
            instance.validate_against(&profile),
            Err(CargoError::TemperatureOutOfRange)
        );
        instance.observed_temperature_millicelsius = Some(5_000);
        assert!(instance.validate_against(&profile).is_ok());

        let equipment = rail_equipment();
        assert_eq!(
            check_module_compatibility(&profile, &instance, &equipment),
            Err(CargoError::FamilyNotSupported)
        );
        let mut cold_equipment = equipment;
        cold_equipment.supported_families = BTreeSet::from([CargoModuleFamily::ColdChainPod]);
        cold_equipment.supplies_module_power = true;
        cold_equipment.available_temperature_millicelsius = Some(TemperatureBand {
            min_millicelsius: 1_000,
            max_millicelsius: 9_000,
        });
        assert!(check_module_compatibility(&profile, &instance, &cold_equipment).is_ok());
    }
}
