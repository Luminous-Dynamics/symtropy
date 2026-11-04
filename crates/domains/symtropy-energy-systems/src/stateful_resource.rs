// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Stateful verification of storage reserve, facility power and thermal trajectories.
//!
//! This is a deterministic verifier, not an optimizer, controller, CFD solver or
//! model-calibration tool. A PASS establishes feasibility only under the frozen
//! contracts, input traces and first-order one-zone RC model supplied here.

use core::fmt;
use std::collections::BTreeSet;

use serde::Serialize;

/// Storage contract including energy capacity and protected critical-load reserve.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StorageReserveContract {
    capacity_kwh: f64,
    initial_energy_kwh: f64,
    minimum_operating_energy_kwh: f64,
    maximum_charge_power_kw: f64,
    maximum_discharge_power_kw: f64,
    charge_efficiency: f64,
    discharge_efficiency: f64,
    critical_load_power_kw: f64,
    ride_through_seconds: u64,
    reserve_margin_fraction: f64,
}

impl StorageReserveContract {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        capacity_kwh: f64,
        initial_energy_kwh: f64,
        minimum_operating_energy_kwh: f64,
        maximum_charge_power_kw: f64,
        maximum_discharge_power_kw: f64,
        charge_efficiency: f64,
        discharge_efficiency: f64,
        critical_load_power_kw: f64,
        ride_through_seconds: u64,
        reserve_margin_fraction: f64,
    ) -> Result<Self, ResourceTrajectoryError> {
        for (name, value) in [
            ("storage capacity", capacity_kwh),
            ("initial storage energy", initial_energy_kwh),
            ("minimum operating energy", minimum_operating_energy_kwh),
            ("maximum charge power", maximum_charge_power_kw),
            ("maximum discharge power", maximum_discharge_power_kw),
            ("critical load power", critical_load_power_kw),
            ("reserve margin fraction", reserve_margin_fraction),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(ResourceTrajectoryError::InvalidQuantity { name, value });
            }
        }
        for (name, value) in [
            ("charge efficiency", charge_efficiency),
            ("discharge efficiency", discharge_efficiency),
        ] {
            if !value.is_finite() || value <= 0.0 || value > 1.0 {
                return Err(ResourceTrajectoryError::InvalidEfficiency { name, value });
            }
        }
        if capacity_kwh <= 0.0
            || initial_energy_kwh > capacity_kwh
            || minimum_operating_energy_kwh > capacity_kwh
        {
            return Err(ResourceTrajectoryError::InvalidStorageEnvelope);
        }
        if critical_load_power_kw > 0.0 && ride_through_seconds == 0 {
            return Err(ResourceTrajectoryError::InvalidStorageEnvelope);
        }
        let contract = Self {
            capacity_kwh,
            initial_energy_kwh,
            minimum_operating_energy_kwh,
            maximum_charge_power_kw,
            maximum_discharge_power_kw,
            charge_efficiency,
            discharge_efficiency,
            critical_load_power_kw,
            ride_through_seconds,
            reserve_margin_fraction,
        };
        let critical_reserve = contract.critical_ride_through_energy_kwh();
        if !critical_reserve.is_finite() {
            return Err(ResourceTrajectoryError::NumericalOverflow("critical reserve calculation"));
        }
        let reserve_floor = contract.minimum_operating_energy_kwh.max(critical_reserve);
        if !reserve_floor.is_finite() {
            return Err(ResourceTrajectoryError::NumericalOverflow("protected reserve floor"));
        }
        if contract.initial_energy_kwh < reserve_floor {
            return Err(ResourceTrajectoryError::InitialEnergyBelowReserve {
                initial_kwh: contract.initial_energy_kwh,
                reserve_kwh: reserve_floor,
            });
        }
        Ok(contract)
    }

    /// Stored energy needed for the declared critical load and ride-through time.
    pub fn critical_ride_through_energy_kwh(&self) -> f64 {
        (self.critical_load_power_kw * self.ride_through_seconds as f64 / 3_600.0)
            / self.discharge_efficiency
            * (1.0 + self.reserve_margin_fraction)
    }

    /// Protected energy floor is the greater of the operating floor and UPS reserve.
    pub fn reserve_floor_kwh(&self) -> f64 {
        self.minimum_operating_energy_kwh
            .max(self.critical_ride_through_energy_kwh())
    }

    pub const fn capacity_kwh(&self) -> f64 {
        self.capacity_kwh
    }

    pub const fn initial_energy_kwh(&self) -> f64 {
        self.initial_energy_kwh
    }

    pub const fn maximum_charge_power_kw(&self) -> f64 {
        self.maximum_charge_power_kw
    }

    pub const fn maximum_discharge_power_kw(&self) -> f64 {
        self.maximum_discharge_power_kw
    }
}

/// First-order, single-zone RC thermal model.
///
/// Thermal capacitance is in kWh/°C and ambient conductance is in kW/°C.
/// The model assumes constant heat injection, cooling extraction and ambient
/// temperature over one slot. It is not a substitute for CFD or multi-node
/// coolant/airflow models.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ThermalZoneContract {
    zone_id: String,
    initial_temperature_c: f64,
    maximum_temperature_c: f64,
    thermal_capacitance_kwh_per_c: f64,
    ambient_conductance_kw_per_c: f64,
    maximum_cooling_heat_removal_kw: f64,
    maximum_cooling_electric_power_kw: f64,
    minimum_cop: f64,
    maximum_cop: f64,
}

impl ThermalZoneContract {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        zone_id: impl Into<String>,
        initial_temperature_c: f64,
        maximum_temperature_c: f64,
        thermal_capacitance_kwh_per_c: f64,
        ambient_conductance_kw_per_c: f64,
        maximum_cooling_heat_removal_kw: f64,
        maximum_cooling_electric_power_kw: f64,
        minimum_cop: f64,
        maximum_cop: f64,
    ) -> Result<Self, ResourceTrajectoryError> {
        let zone_id = zone_id.into();
        require_id("thermal zone ID", &zone_id)?;
        if !initial_temperature_c.is_finite()
            || !maximum_temperature_c.is_finite()
            || !thermal_capacitance_kwh_per_c.is_finite()
            || !ambient_conductance_kw_per_c.is_finite()
            || !maximum_cooling_heat_removal_kw.is_finite()
            || !maximum_cooling_electric_power_kw.is_finite()
            || !minimum_cop.is_finite()
            || !maximum_cop.is_finite()
        {
            return Err(ResourceTrajectoryError::NonFiniteThermalParameter);
        }
        if maximum_temperature_c <= initial_temperature_c
            || thermal_capacitance_kwh_per_c <= 0.0
            || ambient_conductance_kw_per_c < 0.0
            || maximum_cooling_heat_removal_kw < 0.0
            || maximum_cooling_electric_power_kw < 0.0
            || minimum_cop <= 0.0
            || maximum_cop < minimum_cop
        {
            return Err(ResourceTrajectoryError::InvalidThermalEnvelope);
        }
        Ok(Self {
            zone_id,
            initial_temperature_c,
            maximum_temperature_c,
            thermal_capacitance_kwh_per_c,
            ambient_conductance_kw_per_c,
            maximum_cooling_heat_removal_kw,
            maximum_cooling_electric_power_kw,
            minimum_cop,
            maximum_cop,
        })
    }

    pub fn zone_id(&self) -> &str {
        &self.zone_id
    }

    pub const fn initial_temperature_c(&self) -> f64 {
        self.initial_temperature_c
    }

    pub const fn maximum_temperature_c(&self) -> f64 {
        self.maximum_temperature_c
    }

    pub const fn thermal_capacitance_kwh_per_c(&self) -> f64 {
        self.thermal_capacitance_kwh_per_c
    }

    pub const fn ambient_conductance_kw_per_c(&self) -> f64 {
        self.ambient_conductance_kw_per_c
    }
}

/// Frozen site-wide power limits and resource models.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatefulResourceContract {
    scenario_id: String,
    site_id: String,
    boundary_id: String,
    configuration_digest: String,
    storage: StorageReserveContract,
    thermal_zone: ThermalZoneContract,
    maximum_facility_input_power_kw: f64,
    maximum_grid_import_power_kw: f64,
}

impl StatefulResourceContract {
    pub fn new(
        scenario_id: impl Into<String>,
        site_id: impl Into<String>,
        boundary_id: impl Into<String>,
        configuration_digest: impl Into<String>,
        storage: StorageReserveContract,
        thermal_zone: ThermalZoneContract,
        maximum_facility_input_power_kw: f64,
        maximum_grid_import_power_kw: f64,
    ) -> Result<Self, ResourceTrajectoryError> {
        let contract = Self {
            scenario_id: scenario_id.into(),
            site_id: site_id.into(),
            boundary_id: boundary_id.into(),
            configuration_digest: configuration_digest.into(),
            storage,
            thermal_zone,
            maximum_facility_input_power_kw,
            maximum_grid_import_power_kw,
        };
        require_id("scenario ID", &contract.scenario_id)?;
        require_id("site ID", &contract.site_id)?;
        require_id("measurement boundary ID", &contract.boundary_id)?;
        require_id("configuration digest", &contract.configuration_digest)?;
        for (name, value) in [
            ("maximum facility input power", maximum_facility_input_power_kw),
            ("maximum grid import power", maximum_grid_import_power_kw),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(ResourceTrajectoryError::InvalidQuantity { name, value });
            }
        }
        if contract.storage.reserve_floor_kwh() > contract.storage.capacity_kwh {
            return Err(ResourceTrajectoryError::ReserveExceedsStorageCapacity {
                reserve_kwh: contract.storage.reserve_floor_kwh(),
                capacity_kwh: contract.storage.capacity_kwh,
            });
        }
        Ok(contract)
    }

    pub fn scenario_id(&self) -> &str {
        &self.scenario_id
    }

    pub fn site_id(&self) -> &str {
        &self.site_id
    }

    pub fn boundary_id(&self) -> &str {
        &self.boundary_id
    }

    pub fn configuration_digest(&self) -> &str {
        &self.configuration_digest
    }

    pub fn storage(&self) -> &StorageReserveContract {
        &self.storage
    }

    pub fn thermal_zone(&self) -> &ThermalZoneContract {
        &self.thermal_zone
    }

    pub const fn maximum_facility_input_power_kw(&self) -> f64 {
        self.maximum_facility_input_power_kw
    }

    pub const fn maximum_grid_import_power_kw(&self) -> f64 {
        self.maximum_grid_import_power_kw
    }
}

/// One proposed interval of site operation.
///
/// Battery charge and discharge are independent non-negative requested powers;
/// using both in the same interval is a failed feasibility gate. Onsite supply
/// excludes storage. Cooling heat removal is thermal power; cooling electric
/// power is measured separately at the declared facility boundary.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatefulResourceSlot {
    sample_id: String,
    site_id: String,
    boundary_id: String,
    start_unix_seconds: i64,
    end_unix_seconds: i64,
    base_facility_power_kw: f64,
    it_power_kw: f64,
    cooling_electric_power_kw: f64,
    other_facility_power_kw: f64,
    onsite_non_storage_supply_kw: f64,
    storage_charge_power_kw: f64,
    storage_discharge_power_kw: f64,
    it_heat_injection_kw: f64,
    ambient_temperature_c: f64,
    cooling_heat_removal_kw: f64,
    power_evidence_id: String,
    storage_evidence_id: String,
    thermal_evidence_id: String,
}

impl StatefulResourceSlot {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sample_id: impl Into<String>,
        site_id: impl Into<String>,
        boundary_id: impl Into<String>,
        start_unix_seconds: i64,
        end_unix_seconds: i64,
        base_facility_power_kw: f64,
        it_power_kw: f64,
        cooling_electric_power_kw: f64,
        other_facility_power_kw: f64,
        onsite_non_storage_supply_kw: f64,
        storage_charge_power_kw: f64,
        storage_discharge_power_kw: f64,
        it_heat_injection_kw: f64,
        ambient_temperature_c: f64,
        cooling_heat_removal_kw: f64,
        power_evidence_id: impl Into<String>,
        storage_evidence_id: impl Into<String>,
        thermal_evidence_id: impl Into<String>,
    ) -> Result<Self, ResourceTrajectoryError> {
        let slot = Self {
            sample_id: sample_id.into(),
            site_id: site_id.into(),
            boundary_id: boundary_id.into(),
            start_unix_seconds,
            end_unix_seconds,
            base_facility_power_kw,
            it_power_kw,
            cooling_electric_power_kw,
            other_facility_power_kw,
            onsite_non_storage_supply_kw,
            storage_charge_power_kw,
            storage_discharge_power_kw,
            it_heat_injection_kw,
            ambient_temperature_c,
            cooling_heat_removal_kw,
            power_evidence_id: power_evidence_id.into(),
            storage_evidence_id: storage_evidence_id.into(),
            thermal_evidence_id: thermal_evidence_id.into(),
        };
        require_id("sample ID", &slot.sample_id)?;
        require_id("site ID", &slot.site_id)?;
        require_id("boundary ID", &slot.boundary_id)?;
        require_id("power evidence ID", &slot.power_evidence_id)?;
        require_id("storage evidence ID", &slot.storage_evidence_id)?;
        require_id("thermal evidence ID", &slot.thermal_evidence_id)?;
        if start_unix_seconds >= end_unix_seconds || end_unix_seconds.checked_sub(start_unix_seconds).is_none() {
            return Err(ResourceTrajectoryError::InvalidTimeWindow {
                sample_id: slot.sample_id,
                start_unix_seconds,
                end_unix_seconds,
            });
        }
        for (name, value) in [
            ("base facility power", base_facility_power_kw),
            ("IT power", it_power_kw),
            ("cooling electric power", cooling_electric_power_kw),
            ("other facility power", other_facility_power_kw),
            ("onsite non-storage supply", onsite_non_storage_supply_kw),
            ("storage charge power", storage_charge_power_kw),
            ("storage discharge power", storage_discharge_power_kw),
            ("IT heat injection", it_heat_injection_kw),
            ("cooling heat removal", cooling_heat_removal_kw),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(ResourceTrajectoryError::InvalidQuantity { name, value });
            }
        }
        if !ambient_temperature_c.is_finite() {
            return Err(ResourceTrajectoryError::NonFiniteThermalParameter);
        }
        Ok(slot)
    }

    pub fn sample_id(&self) -> &str {
        &self.sample_id
    }

    pub const fn start_unix_seconds(&self) -> i64 {
        self.start_unix_seconds
    }

    pub const fn end_unix_seconds(&self) -> i64 {
        self.end_unix_seconds
    }

    pub fn site_id(&self) -> &str {
        &self.site_id
    }

    pub fn boundary_id(&self) -> &str {
        &self.boundary_id
    }

    pub const fn base_facility_power_kw(&self) -> f64 {
        self.base_facility_power_kw
    }

    pub const fn other_facility_power_kw(&self) -> f64 {
        self.other_facility_power_kw
    }

    pub const fn onsite_non_storage_supply_kw(&self) -> f64 {
        self.onsite_non_storage_supply_kw
    }

    pub const fn ambient_temperature_c(&self) -> f64 {
        self.ambient_temperature_c
    }
}

/// A frozen, chronological sequence of resource intervals.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatefulResourceTrajectory {
    contract: StatefulResourceContract,
    slots: Vec<StatefulResourceSlot>,
}

impl StatefulResourceTrajectory {
    pub fn new(
        contract: StatefulResourceContract,
        slots: Vec<StatefulResourceSlot>,
    ) -> Result<Self, ResourceTrajectoryError> {
        if slots.is_empty() {
            return Err(ResourceTrajectoryError::EmptyTrajectory);
        }
        let mut ids = BTreeSet::new();
        let mut previous_end = None;
        for slot in &slots {
            if slot.site_id != contract.site_id {
                return Err(ResourceTrajectoryError::SiteMismatch {
                    sample_id: slot.sample_id.clone(),
                });
            }
            if slot.boundary_id != contract.boundary_id {
                return Err(ResourceTrajectoryError::BoundaryMismatch {
                    sample_id: slot.sample_id.clone(),
                });
            }
            if !ids.insert(slot.sample_id.clone()) {
                return Err(ResourceTrajectoryError::DuplicateSampleId(
                    slot.sample_id.clone(),
                ));
            }
            if let Some(expected_start) = previous_end {
                if slot.start_unix_seconds != expected_start {
                    return Err(ResourceTrajectoryError::NonContiguousWindows {
                        expected_start,
                        observed_start: slot.start_unix_seconds,
                        sample_id: slot.sample_id.clone(),
                    });
                }
            }
            previous_end = Some(slot.end_unix_seconds);
        }
        Ok(Self { contract, slots })
    }

    pub fn contract(&self) -> &StatefulResourceContract {
        &self.contract
    }

    pub fn slots(&self) -> &[StatefulResourceSlot] {
        &self.slots
    }
}

/// Feasibility gates reconstructed independently for one interval.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatefulResourceSlotReceipt {
    pub sample_id: String,
    pub start_unix_seconds: i64,
    pub end_unix_seconds: i64,
    pub duration_hours: f64,
    pub storage_energy_start_kwh: f64,
    pub storage_energy_end_kwh: f64,
    pub protected_reserve_floor_kwh: f64,
    pub reserve_margin_kwh: f64,
    pub storage_charge_power_kw: f64,
    pub storage_discharge_power_kw: f64,
    pub storage_power_limits_passed: bool,
    pub storage_direction_passed: bool,
    pub storage_capacity_passed: bool,
    pub reserve_passed: bool,
    pub zone_temperature_start_c: f64,
    pub zone_temperature_end_c: f64,
    pub maximum_zone_temperature_c: f64,
    pub cooling_cop: Option<f64>,
    pub cooling_capacity_passed: bool,
    pub cooling_cop_passed: bool,
    pub thermal_limit_passed: bool,
    pub facility_input_power_kw: f64,
    pub grid_import_power_kw: f64,
    pub facility_energy_kwh: f64,
    pub grid_import_energy_kwh: f64,
    pub facility_limit_passed: bool,
    pub grid_limit_passed: bool,
    pub power_evidence_id: String,
    pub storage_evidence_id: String,
    pub thermal_evidence_id: String,
    pub passed: bool,
}

/// Aggregate stateful-resource assurance receipt.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatefulResourceVerificationReport {
    pub scenario_id: String,
    pub site_id: String,
    pub boundary_id: String,
    pub configuration_digest: String,
    pub storage_zone_id: String,
    pub slot_count: usize,
    pub total_facility_energy_kwh: f64,
    pub total_grid_import_energy_kwh: f64,
    pub peak_facility_input_power_kw: f64,
    pub peak_grid_import_power_kw: f64,
    pub minimum_storage_energy_kwh: f64,
    pub minimum_reserve_margin_kwh: f64,
    pub maximum_zone_temperature_c: f64,
    pub slot_receipts: Vec<StatefulResourceSlotReceipt>,
    pub passed: bool,
}

/// Errors for malformed contracts, evidence, time windows or non-finite arithmetic.
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceTrajectoryError {
    EmptyIdentifier(&'static str),
    InvalidQuantity { name: &'static str, value: f64 },
    InvalidEfficiency { name: &'static str, value: f64 },
    InvalidStorageEnvelope,
    InitialEnergyBelowReserve { initial_kwh: f64, reserve_kwh: f64 },
    ReserveExceedsStorageCapacity { reserve_kwh: f64, capacity_kwh: f64 },
    NonFiniteThermalParameter,
    InvalidThermalEnvelope,
    InvalidTimeWindow { sample_id: String, start_unix_seconds: i64, end_unix_seconds: i64 },
    EmptyTrajectory,
    DuplicateSampleId(String),
    SiteMismatch { sample_id: String },
    BoundaryMismatch { sample_id: String },
    NonContiguousWindows { expected_start: i64, observed_start: i64, sample_id: String },
    NumericalOverflow(&'static str),
}

impl fmt::Display for ResourceTrajectoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentifier(name) => write!(f, "{name} must not be empty"),
            Self::InvalidQuantity { name, value } => {
                write!(f, "{name} must be finite and non-negative; got {value}")
            }
            Self::InvalidEfficiency { name, value } => {
                write!(f, "{name} must be finite and in (0, 1]; got {value}")
            }
            Self::InvalidStorageEnvelope => write!(f, "storage capacity or operating envelope is invalid"),
            Self::InitialEnergyBelowReserve { initial_kwh, reserve_kwh } => write!(
                f,
                "initial storage energy ({initial_kwh} kWh) is below required reserve ({reserve_kwh} kWh)"
            ),
            Self::ReserveExceedsStorageCapacity { reserve_kwh, capacity_kwh } => write!(
                f,
                "required critical reserve ({reserve_kwh} kWh) exceeds capacity ({capacity_kwh} kWh)"
            ),
            Self::NonFiniteThermalParameter => write!(f, "thermal parameters and ambient temperature must be finite"),
            Self::InvalidThermalEnvelope => write!(f, "thermal capacity, cooling or temperature envelope is invalid"),
            Self::InvalidTimeWindow { sample_id, start_unix_seconds, end_unix_seconds } => write!(
                f,
                "sample {sample_id} has invalid time window [{start_unix_seconds}, {end_unix_seconds})"
            ),
            Self::EmptyTrajectory => write!(f, "resource trajectory must contain at least one interval"),
            Self::DuplicateSampleId(id) => write!(f, "duplicate resource sample ID: {id}"),
            Self::SiteMismatch { sample_id } => write!(f, "sample {sample_id} belongs to another site"),
            Self::BoundaryMismatch { sample_id } => write!(f, "sample {sample_id} uses another measurement boundary"),
            Self::NonContiguousWindows { expected_start, observed_start, sample_id } => write!(
                f,
                "sample {sample_id} starts at {observed_start}; expected contiguous start {expected_start}"
            ),
            Self::NumericalOverflow(name) => write!(f, "non-finite resource trajectory arithmetic: {name}"),
        }
    }
}

impl std::error::Error for ResourceTrajectoryError {}

/// Reconstruct storage, thermal and site-power state over every interval.
///
/// Charge/discharge power is measured at the AC-equivalent declared site
/// boundary. Charging increases facility demand; discharging offsets grid
/// import. Export is not credited in v0.1. Thermal temperature uses the exact
/// first-order RC solution for constant interval inputs. With zero ambient
/// conductance, the update reduces to an energy/capacitance balance.
/// A PASS is limited to this model and these declared inputs.
pub fn verify_stateful_resource_trajectory(
    trajectory: &StatefulResourceTrajectory,
) -> Result<StatefulResourceVerificationReport, ResourceTrajectoryError> {
    let contract = &trajectory.contract;
    let storage = &contract.storage;
    let thermal = &contract.thermal_zone;
    let reserve_floor = storage.reserve_floor_kwh();
    let mut storage_energy = storage.initial_energy_kwh;
    let mut zone_temperature = thermal.initial_temperature_c;
    let mut total_facility_energy = 0.0;
    let mut total_grid_energy = 0.0;
    let mut peak_facility_power: f64 = 0.0;
    let mut peak_grid_power: f64 = 0.0;
    let mut minimum_storage_energy = storage_energy;
    let mut minimum_reserve_margin = storage_energy - reserve_floor;
    let mut maximum_zone_temperature = zone_temperature;
    let mut all_passed = true;
    let mut receipts = Vec::with_capacity(trajectory.slots.len());

    for slot in &trajectory.slots {
        let duration_seconds = slot.end_unix_seconds
            .checked_sub(slot.start_unix_seconds)
            .ok_or(ResourceTrajectoryError::NumericalOverflow("slot duration"))? as f64;
        let duration_hours = duration_seconds / 3_600.0;
        let storage_start = storage_energy;
        let temperature_start = zone_temperature;

        let direction_passed =
            !(slot.storage_charge_power_kw > 0.0 && slot.storage_discharge_power_kw > 0.0);
        let power_limits_passed = slot.storage_charge_power_kw
            <= storage.maximum_charge_power_kw
            && slot.storage_discharge_power_kw <= storage.maximum_discharge_power_kw;

        let charged_energy = slot.storage_charge_power_kw * duration_hours * storage.charge_efficiency;
        let discharged_energy =
            slot.storage_discharge_power_kw * duration_hours / storage.discharge_efficiency;
        let next_storage = storage_start + charged_energy - discharged_energy;
        if !next_storage.is_finite() || !charged_energy.is_finite() || !discharged_energy.is_finite() {
            return Err(ResourceTrajectoryError::NumericalOverflow("storage state"));
        }
        let capacity_passed = next_storage >= 0.0 && next_storage <= storage.capacity_kwh;
        let reserve_margin = next_storage - reserve_floor;
        let reserve_passed = reserve_margin >= 0.0;

        let facility_input_power = slot.base_facility_power_kw
            + slot.it_power_kw
            + slot.cooling_electric_power_kw
            + slot.other_facility_power_kw
            + slot.storage_charge_power_kw;
        let net_grid_power = facility_input_power
            - slot.onsite_non_storage_supply_kw
            - slot.storage_discharge_power_kw;
        if !facility_input_power.is_finite() || !net_grid_power.is_finite() {
            return Err(ResourceTrajectoryError::NumericalOverflow("site power balance"));
        }
        let grid_import_power = net_grid_power.max(0.0);

        let cooling_capacity_passed = slot.cooling_heat_removal_kw
            <= thermal.maximum_cooling_heat_removal_kw
            && slot.cooling_electric_power_kw <= thermal.maximum_cooling_electric_power_kw;
        let cooling_cop = if slot.cooling_heat_removal_kw > 0.0
            && slot.cooling_electric_power_kw > 0.0
        {
            Some(slot.cooling_heat_removal_kw / slot.cooling_electric_power_kw)
        } else {
            None
        };
        if cooling_cop.is_some_and(|cop| !cop.is_finite()) {
            return Err(ResourceTrajectoryError::NumericalOverflow("cooling COP"));
        }
        let cooling_cop_passed = if slot.cooling_heat_removal_kw == 0.0 {
            true
        } else if let Some(cop) = cooling_cop {
            cop.is_finite() && cop >= thermal.minimum_cop && cop <= thermal.maximum_cop
        } else {
            false
        };

        let net_heat_power = slot.it_heat_injection_kw - slot.cooling_heat_removal_kw;
        let next_temperature = if thermal.ambient_conductance_kw_per_c > 0.0 {
            let conductance = thermal.ambient_conductance_kw_per_c;
            let equilibrium = slot.ambient_temperature_c + net_heat_power / conductance;
            let exponent = -conductance * duration_hours / thermal.thermal_capacitance_kwh_per_c;
            equilibrium
                + (temperature_start - equilibrium) * exponent.exp()
        } else {
            temperature_start
                + net_heat_power * duration_hours / thermal.thermal_capacitance_kwh_per_c
        };
        if !next_temperature.is_finite() {
            return Err(ResourceTrajectoryError::NumericalOverflow("thermal state"));
        }
        let maximum_interval_temperature = temperature_start.max(next_temperature);
        let thermal_limit_passed =
            maximum_interval_temperature <= thermal.maximum_temperature_c;

        let facility_limit_passed =
            facility_input_power <= contract.maximum_facility_input_power_kw;
        let grid_limit_passed = grid_import_power <= contract.maximum_grid_import_power_kw;
        let facility_energy = facility_input_power * duration_hours;
        let grid_energy = grid_import_power * duration_hours;
        for (name, value) in [
            ("facility energy", facility_energy),
            ("grid energy", grid_energy),
            ("temperature maximum", maximum_interval_temperature),
            ("reserve margin", reserve_margin),
        ] {
            if !value.is_finite() {
                return Err(ResourceTrajectoryError::NumericalOverflow(name));
            }
        }

        let slot_passed = direction_passed
            && power_limits_passed
            && capacity_passed
            && reserve_passed
            && cooling_capacity_passed
            && cooling_cop_passed
            && thermal_limit_passed
            && facility_limit_passed
            && grid_limit_passed;
        all_passed &= slot_passed;

        total_facility_energy += facility_energy;
        total_grid_energy += grid_energy;
        if !total_facility_energy.is_finite() || !total_grid_energy.is_finite() {
            return Err(ResourceTrajectoryError::NumericalOverflow("trajectory energy totals"));
        }
        peak_facility_power = peak_facility_power.max(facility_input_power);
        peak_grid_power = peak_grid_power.max(grid_import_power);
        minimum_storage_energy = minimum_storage_energy.min(next_storage);
        minimum_reserve_margin = minimum_reserve_margin.min(reserve_margin);
        maximum_zone_temperature = maximum_zone_temperature.max(maximum_interval_temperature);

        receipts.push(StatefulResourceSlotReceipt {
            sample_id: slot.sample_id.clone(),
            start_unix_seconds: slot.start_unix_seconds,
            end_unix_seconds: slot.end_unix_seconds,
            duration_hours,
            storage_energy_start_kwh: storage_start,
            storage_energy_end_kwh: next_storage,
            protected_reserve_floor_kwh: reserve_floor,
            reserve_margin_kwh: reserve_margin,
            storage_charge_power_kw: slot.storage_charge_power_kw,
            storage_discharge_power_kw: slot.storage_discharge_power_kw,
            storage_power_limits_passed: power_limits_passed,
            storage_direction_passed: direction_passed,
            storage_capacity_passed: capacity_passed,
            reserve_passed,
            zone_temperature_start_c: temperature_start,
            zone_temperature_end_c: next_temperature,
            maximum_zone_temperature_c: maximum_interval_temperature,
            cooling_cop,
            cooling_capacity_passed,
            cooling_cop_passed,
            thermal_limit_passed,
            facility_input_power_kw: facility_input_power,
            grid_import_power_kw: grid_import_power,
            facility_energy_kwh: facility_energy,
            grid_import_energy_kwh: grid_energy,
            facility_limit_passed,
            grid_limit_passed,
            power_evidence_id: slot.power_evidence_id.clone(),
            storage_evidence_id: slot.storage_evidence_id.clone(),
            thermal_evidence_id: slot.thermal_evidence_id.clone(),
            passed: slot_passed,
        });
        storage_energy = next_storage;
        zone_temperature = next_temperature;
    }

    Ok(StatefulResourceVerificationReport {
        scenario_id: contract.scenario_id.clone(),
        site_id: contract.site_id.clone(),
        boundary_id: contract.boundary_id.clone(),
        configuration_digest: contract.configuration_digest.clone(),
        storage_zone_id: thermal.zone_id.clone(),
        slot_count: receipts.len(),
        total_facility_energy_kwh: total_facility_energy,
        total_grid_import_energy_kwh: total_grid_energy,
        peak_facility_input_power_kw: peak_facility_power,
        peak_grid_import_power_kw: peak_grid_power,
        minimum_storage_energy_kwh: minimum_storage_energy,
        minimum_reserve_margin_kwh: minimum_reserve_margin,
        maximum_zone_temperature_c: maximum_zone_temperature,
        slot_receipts: receipts,
        passed: all_passed,
    })
}

fn require_id(name: &'static str, value: &str) -> Result<(), ResourceTrajectoryError> {
    if value.trim().is_empty() {
        return Err(ResourceTrajectoryError::EmptyIdentifier(name));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn storage(initial_kwh: f64) -> StorageReserveContract {
        StorageReserveContract::new(
            100.0, initial_kwh, 20.0, 50.0, 50.0, 0.95, 0.95,
            10.0, 3_600, 0.20,
        ).unwrap()
    }

    fn thermal(initial_c: f64, maximum_c: f64, conductance: f64) -> ThermalZoneContract {
        ThermalZoneContract::new(
            "rack-loop-a", initial_c, maximum_c, 10.0, conductance,
            80.0, 30.0, 2.0, 8.0,
        ).unwrap()
    }

    fn contract(initial_kwh: f64, initial_c: f64, maximum_c: f64, conductance: f64) -> StatefulResourceContract {
        StatefulResourceContract::new(
            "scenario-1", "site-1", "facility-pcc", "config-digest-1",
            storage(initial_kwh), thermal(initial_c, maximum_c, conductance),
            200.0, 150.0,
        ).unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    fn slot(
        id: &str,
        start: i64,
        seconds: i64,
        base_kw: f64,
        it_kw: f64,
        cooling_electric_kw: f64,
        onsite_kw: f64,
        charge_kw: f64,
        discharge_kw: f64,
        it_heat_kw: f64,
        ambient_c: f64,
        heat_removed_kw: f64,
    ) -> StatefulResourceSlot {
        StatefulResourceSlot::new(
            id, "site-1", "facility-pcc", start, start + seconds,
            base_kw, it_kw, cooling_electric_kw, 2.0, onsite_kw,
            charge_kw, discharge_kw, it_heat_kw, ambient_c, heat_removed_kw,
            format!("power-{id}"), format!("storage-{id}"), format!("thermal-{id}"),
        ).unwrap()
    }

    fn valid_trajectory() -> StatefulResourceTrajectory {
        StatefulResourceTrajectory::new(
            contract(90.0, 25.0, 32.0, 1.0),
            vec![
                slot("s1", 10_000, 900, 10.0, 20.0, 5.0, 5.0, 0.0, 5.0, 20.0, 25.0, 25.0),
                slot("s2", 10_900, 900, 10.0, 20.0, 5.0, 5.0, 0.0, 5.0, 20.0, 25.0, 25.0),
            ],
        ).unwrap()
    }

    #[test]
    fn accepts_feasible_storage_thermal_and_site_trajectory() {
        let report = verify_stateful_resource_trajectory(&valid_trajectory()).unwrap();
        assert!(report.passed);
        assert_eq!(report.slot_count, 2);
        assert!((report.total_facility_energy_kwh - 18.5).abs() < 1e-9);
        assert!((report.total_grid_import_energy_kwh - 13.5).abs() < 1e-9);
        assert!(report.minimum_reserve_margin_kwh > 0.0);
        assert!(report.maximum_zone_temperature_c <= 32.0);
        assert!(report.slot_receipts.iter().all(|receipt| receipt.passed));
    }

    #[test]
    fn rejects_trajectory_that_consumes_critical_load_reserve() {
        let trajectory = StatefulResourceTrajectory::new(
            contract(30.0, 25.0, 32.0, 1.0),
            vec![
                slot("s1", 10_000, 900, 10.0, 20.0, 5.0, 0.0, 0.0, 20.0, 20.0, 25.0, 25.0),
                slot("s2", 10_900, 900, 10.0, 20.0, 5.0, 0.0, 0.0, 20.0, 20.0, 25.0, 25.0),
            ],
        ).unwrap();
        let report = verify_stateful_resource_trajectory(&trajectory).unwrap();
        assert!(!report.passed);
        assert!(!report.slot_receipts[1].reserve_passed);
    }

    #[test]
    fn rejects_simultaneous_charge_and_discharge() {
        let trajectory = StatefulResourceTrajectory::new(
            contract(90.0, 25.0, 32.0, 1.0),
            vec![slot("s1", 10_000, 900, 10.0, 20.0, 5.0, 0.0, 5.0, 5.0, 20.0, 25.0, 25.0)],
        ).unwrap();
        let report = verify_stateful_resource_trajectory(&trajectory).unwrap();
        assert!(!report.passed);
        assert!(!report.slot_receipts[0].storage_direction_passed);
    }

    #[test]
    fn rejects_storage_charge_power_limit_violations() {
        let trajectory = StatefulResourceTrajectory::new(
            contract(90.0, 25.0, 32.0, 1.0),
            vec![slot("s1", 10_000, 900, 10.0, 20.0, 5.0, 0.0, 60.0, 0.0, 20.0, 25.0, 25.0)],
        ).unwrap();
        let report = verify_stateful_resource_trajectory(&trajectory).unwrap();
        assert!(!report.passed);
        assert!(!report.slot_receipts[0].storage_power_limits_passed);
    }

    #[test]
    fn rejects_storage_discharge_power_limit_violations() {
        let trajectory = StatefulResourceTrajectory::new(
            contract(90.0, 25.0, 32.0, 1.0),
            vec![slot("s1", 10_000, 900, 10.0, 20.0, 5.0, 0.0, 0.0, 60.0, 20.0, 25.0, 25.0)],
        ).unwrap();
        let report = verify_stateful_resource_trajectory(&trajectory).unwrap();
        assert!(!report.passed);
        assert!(!report.slot_receipts[0].storage_power_limits_passed);
    }

    #[test]
    fn rejects_storage_capacity_overflow() {
        let trajectory = StatefulResourceTrajectory::new(
            contract(99.0, 25.0, 32.0, 1.0),
            vec![slot("s1", 10_000, 3_600, 10.0, 20.0, 5.0, 0.0, 50.0, 0.0, 20.0, 25.0, 25.0)],
        ).unwrap();
        let report = verify_stateful_resource_trajectory(&trajectory).unwrap();
        assert!(!report.passed);
        assert!(!report.slot_receipts[0].storage_capacity_passed);
    }

    #[test]
    fn rejects_thermal_runaway_over_a_long_interval() {
        let trajectory = StatefulResourceTrajectory::new(
            contract(90.0, 30.0, 32.0, 0.1),
            vec![slot("s1", 10_000, 7_200, 10.0, 50.0, 0.0, 0.0, 0.0, 0.0, 50.0, 25.0, 0.0)],
        ).unwrap();
        let report = verify_stateful_resource_trajectory(&trajectory).unwrap();
        assert!(!report.passed);
        assert!(!report.slot_receipts[0].thermal_limit_passed);
    }

    #[test]
    fn rejects_cooling_cop_outside_declared_envelope() {
        let trajectory = StatefulResourceTrajectory::new(
            contract(90.0, 25.0, 32.0, 1.0),
            vec![slot("s1", 10_000, 900, 10.0, 20.0, 1.0, 0.0, 0.0, 0.0, 20.0, 25.0, 25.0)],
        ).unwrap();
        let report = verify_stateful_resource_trajectory(&trajectory).unwrap();
        assert!(!report.passed);
        assert!(!report.slot_receipts[0].cooling_cop_passed);
    }

    #[test]
    fn rejects_thermal_extraction_without_electrical_input() {
        let trajectory = StatefulResourceTrajectory::new(
            contract(90.0, 25.0, 32.0, 1.0),
            vec![slot("s1", 10_000, 900, 10.0, 20.0, 0.0, 0.0, 0.0, 0.0, 20.0, 25.0, 25.0)],
        ).unwrap();
        let report = verify_stateful_resource_trajectory(&trajectory).unwrap();
        assert!(!report.passed);
        assert!(!report.slot_receipts[0].cooling_cop_passed);
    }

    #[test]
    fn rejects_grid_import_violation_even_when_reserve_is_preserved() {
        let mut c = contract(90.0, 25.0, 32.0, 1.0);
        c.maximum_grid_import_power_kw = 10.0;
        let trajectory = StatefulResourceTrajectory::new(
            c,
            vec![slot("s1", 10_000, 900, 40.0, 30.0, 5.0, 0.0, 0.0, 0.0, 30.0, 25.0, 25.0)],
        ).unwrap();
        let report = verify_stateful_resource_trajectory(&trajectory).unwrap();
        assert!(!report.passed);
        assert!(report.slot_receipts[0].reserve_passed);
        assert!(!report.slot_receipts[0].grid_limit_passed);
    }

    #[test]
    fn rejects_noncontiguous_time_windows() {
        let error = StatefulResourceTrajectory::new(
            contract(90.0, 25.0, 32.0, 1.0),
            vec![
                slot("s1", 10_000, 900, 10.0, 20.0, 5.0, 0.0, 0.0, 0.0, 20.0, 25.0, 25.0),
                slot("s2", 10_901, 900, 10.0, 20.0, 5.0, 0.0, 0.0, 0.0, 20.0, 25.0, 25.0),
            ],
        ).unwrap_err();
        assert!(matches!(error, ResourceTrajectoryError::NonContiguousWindows { .. }));
    }

    #[test]
    fn rejects_duplicate_sample_ids() {
        let error = StatefulResourceTrajectory::new(
            contract(90.0, 25.0, 32.0, 1.0),
            vec![
                slot("same", 10_000, 900, 10.0, 20.0, 5.0, 0.0, 0.0, 0.0, 20.0, 25.0, 25.0),
                slot("same", 10_900, 900, 10.0, 20.0, 5.0, 0.0, 0.0, 0.0, 20.0, 25.0, 25.0),
            ],
        ).unwrap_err();
        assert!(matches!(error, ResourceTrajectoryError::DuplicateSampleId(_)));
    }

    #[test]
    fn rejects_missing_evidence_and_nonfinite_inputs() {
        assert!(StatefulResourceSlot::new(
            "s1", "site-1", "facility-pcc", 10_000, 10_900,
            10.0, 20.0, 5.0, 2.0, 0.0, 0.0, 0.0,
            20.0, 25.0, 25.0, "", "storage-evidence", "thermal-evidence",
        ).is_err());
        assert!(StorageReserveContract::new(
            100.0, 90.0, 20.0, 50.0, 50.0, 0.95, 1.1,
            10.0, 3_600, 0.2,
        ).is_err());
    }

    #[test]
    fn zero_conductance_uses_energy_over_capacitance_limit() {
        let trajectory = StatefulResourceTrajectory::new(
            contract(90.0, 25.0, 32.0, 0.0),
            vec![slot("s1", 10_000, 3_600, 10.0, 20.0, 5.0, 0.0, 0.0, 0.0, 20.0, 25.0, 10.0)],
        ).unwrap();
        let report = verify_stateful_resource_trajectory(&trajectory).unwrap();
        assert!(report.passed);
        assert!((report.slot_receipts[0].zone_temperature_end_c - 26.0).abs() < 1e-9);
    }
}
