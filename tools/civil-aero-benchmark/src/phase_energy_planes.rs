//! Typed, carrier-aware phase energy conservation for conceptual civil aircraft screening.
//!
//! This module deliberately separates fuel chemical energy, prime-mover shaft
//! output, generator electrical output, electrical-bus flows, internal energy
//! storage, motor/drive conversion, and propulsor shaft/useful energy.
//!
//! It is an accounting/contract layer, not an engine, battery, motor, inverter,
//! propeller, thermal or hybrid-dispatch physics model. Component maps and
//! operating limits must be supplied from separately admitted evidence.
//! Every report remains screening-only.

use std::collections::{HashMap, HashSet};

pub const PHASE_ENERGY_SCHEMA_VERSION: &str = "phase-energy-balance/0.2.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyStoreKind {
    Battery,
    Supercapacitor,
    OtherElectricalStorage,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuelStateTransition {
    pub fuel_source_id: String,
    pub evidence_sha256: String,
    pub chemical_energy_start_j: f64,
    pub chemical_energy_consumed_j: f64,
    pub chemical_energy_end_j: f64,
    pub required_reserve_j: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrimeMoverConversion {
    pub prime_mover_id: String,
    pub model_evidence_sha256: String,
    pub fuel_source_id: String,
    pub fuel_energy_input_j: f64,
    pub mechanical_output_j: f64,
    pub thermal_and_exhaust_loss_j: f64,
    /// Mechanical energy delivered directly to the propulsor shaft. This is
    /// non-zero for a parallel-hybrid/direct-drive path and zero for a pure
    /// series-hybrid generator path.
    pub direct_propulsor_shaft_j: f64,
    pub accessory_mechanical_load_j: f64,
    pub mechanical_distribution_loss_j: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneratorConversion {
    pub generator_id: String,
    pub model_evidence_sha256: String,
    pub prime_mover_id: String,
    pub mechanical_input_j: f64,
    pub electrical_output_to_bus_j: f64,
    pub conversion_loss_j: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnergyStoreTransition {
    pub store_id: String,
    pub kind: EnergyStoreKind,
    /// Exact installed-system and operating-envelope evidence identities owned
    /// by the battery/energy-store qualification work package.
    pub installed_system_evidence_sha256: String,
    pub operating_envelope_evidence_sha256: String,
    pub usable_energy_start_j: f64,
    pub usable_energy_end_j: f64,
    pub usable_capacity_j: f64,
    pub reserve_floor_j: f64,
    /// Energy delivered from the store terminals to the electrical bus.
    pub discharge_to_bus_j: f64,
    /// Electrical energy drawn from the bus to charge this store.
    pub charge_from_bus_j: f64,
    /// Store-internal energy change is not equal to bus energy when efficiency
    /// is less than one.
    pub discharge_efficiency: f64,
    pub charge_efficiency: f64,
    pub state_of_health_fraction: f64,
    pub minimum_state_of_health_fraction: f64,
    pub temperature_c: f64,
    pub minimum_temperature_c: f64,
    pub maximum_temperature_c: f64,
    pub average_discharge_power_limit_w: f64,
    pub peak_discharge_power_w: f64,
    pub peak_discharge_power_limit_w: f64,
    pub average_charge_power_limit_w: f64,
    pub peak_charge_power_w: f64,
    pub peak_charge_power_limit_w: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MotorDriveConversion {
    pub motor_id: String,
    pub model_evidence_sha256: String,
    pub bus_input_j: f64,
    pub inverter_loss_j: f64,
    pub motor_loss_j: f64,
    pub shaft_output_j: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropulsorShaftBalance {
    pub propulsor_id: String,
    pub model_evidence_sha256: String,
    pub shaft_input_j: f64,
    pub useful_propulsive_energy_j: f64,
    pub propulsor_loss_j: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BusLoads {
    pub auxiliary_load_j: f64,
    /// Electrical energy consumed by pumps, fans and other thermal-management
    /// hardware. Rejected heat is a separate thermal quantity and is not added
    /// here a second time.
    pub thermal_electrical_load_j: f64,
    pub distribution_loss_j: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhaseEnergyBalance {
    pub schema_version: &'static str,
    pub mission_id: String,
    pub phase_id: String,
    pub configuration_sha256: String,
    pub start_state_sha256: String,
    pub end_state_sha256: String,
    pub duration_s: f64,
    pub tolerance_j: f64,
    pub fuel_states: Vec<FuelStateTransition>,
    pub prime_movers: Vec<PrimeMoverConversion>,
    pub generators: Vec<GeneratorConversion>,
    pub stores: Vec<EnergyStoreTransition>,
    pub bus_loads: BusLoads,
    pub motor_drives: Vec<MotorDriveConversion>,
    pub propulsors: Vec<PropulsorShaftBalance>,
    /// Aggregate peak demand at the declared upstream source boundary.
    pub peak_source_power_w: f64,
    pub peak_source_power_limit_w: f64,
    pub continuous_source_power_limit_w: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseEnergyStatus {
    ConservedWithinDeclaredModel,
    Infeasible,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuelEnergyResidual {
    pub fuel_source_id: String,
    pub residual_j: f64,
    pub reserve_margin_j: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentEnergyResidual {
    pub component_id: String,
    pub residual_j: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoreEnergyResult {
    pub store_id: String,
    pub predicted_usable_energy_end_j: f64,
    pub actual_usable_energy_end_j: f64,
    pub transition_residual_j: f64,
    pub reserve_margin_j: f64,
    pub average_discharge_power_w: f64,
    pub average_charge_power_w: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhaseEnergyReport {
    pub schema_version: &'static str,
    pub evidence_level: &'static str,
    pub mission_id: String,
    pub phase_id: String,
    pub configuration_sha256: String,
    pub start_state_sha256: String,
    pub end_state_sha256: String,
    pub status: PhaseEnergyStatus,
    pub bus_residual_j: f64,
    pub shaft_combiner_residual_j: f64,
    pub fuel_residuals: Vec<FuelEnergyResidual>,
    pub prime_mover_fuel_residuals: Vec<ComponentEnergyResidual>,
    pub prime_mover_shaft_residuals: Vec<ComponentEnergyResidual>,
    pub generator_residuals: Vec<ComponentEnergyResidual>,
    pub store_results: Vec<StoreEnergyResult>,
    pub motor_residuals: Vec<ComponentEnergyResidual>,
    pub propulsor_residuals: Vec<ComponentEnergyResidual>,
    pub continuous_bus_supply_power_w: f64,
    pub peak_source_power_margin_w: f64,
    pub continuous_source_power_margin_w: f64,
}

impl PhaseEnergyBalance {
    /// Recompute every residual from the component records. Submitted residuals
    /// are not accepted as inputs and cannot override the independent sums.
    pub fn evaluate(
        &self,
        expected_configuration_sha256: &str,
        expected_mission_id: &str,
        expected_start_state_sha256: &str,
        expected_end_state_sha256: &str,
    ) -> Result<PhaseEnergyReport, String> {
        if self.schema_version != PHASE_ENERGY_SCHEMA_VERSION {
            return Err("unsupported phase-energy schema version".to_owned());
        }
        validate_nonempty(&self.mission_id, "mission_id")?;
        validate_nonempty(&self.phase_id, "phase_id")?;
        validate_digest(&self.configuration_sha256, "configuration_sha256")?;
        validate_digest(&self.start_state_sha256, "start_state_sha256")?;
        validate_digest(&self.end_state_sha256, "end_state_sha256")?;
        validate_digest(expected_configuration_sha256, "expected_configuration_sha256")?;
        validate_digest(expected_start_state_sha256, "expected_start_state_sha256")?;
        validate_digest(expected_end_state_sha256, "expected_end_state_sha256")?;

        if self.configuration_sha256 != expected_configuration_sha256 {
            return Err("stale phase energy: configuration digest mismatch".to_owned());
        }
        if self.mission_id != expected_mission_id {
            return Err("stale phase energy: mission identity mismatch".to_owned());
        }
        if self.start_state_sha256 != expected_start_state_sha256
            || self.end_state_sha256 != expected_end_state_sha256
        {
            return Err("stale phase energy: loading-state digest mismatch".to_owned());
        }
        if !self.duration_s.is_finite() || self.duration_s <= 0.0 {
            return Err("phase duration_s must be finite and > 0".to_owned());
        }
        if !self.tolerance_j.is_finite() || self.tolerance_j < 0.0 {
            return Err("tolerance_j must be finite and >= 0".to_owned());
        }
        for (name, value) in [
            ("peak_source_power_w", self.peak_source_power_w),
            ("peak_source_power_limit_w", self.peak_source_power_limit_w),
            ("continuous_source_power_limit_w", self.continuous_source_power_limit_w),
            ("auxiliary_load_j", self.bus_loads.auxiliary_load_j),
            ("thermal_electrical_load_j", self.bus_loads.thermal_electrical_load_j),
            ("distribution_loss_j", self.bus_loads.distribution_loss_j),
        ] {
            validate_nonnegative(value, name)?;
        }
        if self.peak_source_power_limit_w <= 0.0
            || self.continuous_source_power_limit_w <= 0.0
        {
            return Err("source power limits must be finite and > 0".to_owned());
        }

        require_unique(
            self.fuel_states.iter().map(|item| item.fuel_source_id.as_str()),
            "fuel source",
        )?;
        require_unique(
            self.prime_movers.iter().map(|item| item.prime_mover_id.as_str()),
            "prime mover",
        )?;
        require_unique(
            self.generators.iter().map(|item| item.generator_id.as_str()),
            "generator",
        )?;
        require_unique(self.stores.iter().map(|item| item.store_id.as_str()), "energy store")?;
        require_unique(
            self.motor_drives.iter().map(|item| item.motor_id.as_str()),
            "motor drive",
        )?;
        require_unique(
            self.propulsors.iter().map(|item| item.propulsor_id.as_str()),
            "propulsor",
        )?;

        let fuel_by_id: HashMap<&str, &FuelStateTransition> = self
            .fuel_states
            .iter()
            .map(|item| (item.fuel_source_id.as_str(), item))
            .collect();
        let engine_by_id: HashMap<&str, &PrimeMoverConversion> = self
            .prime_movers
            .iter()
            .map(|item| (item.prime_mover_id.as_str(), item))
            .collect();

        let mut infeasible = false;
        let mut fuel_residuals = Vec::new();
        for fuel in &self.fuel_states {
            validate_digest(&fuel.evidence_sha256, "fuel evidence SHA-256")?;
            for (name, value) in [
                ("chemical_energy_start_j", fuel.chemical_energy_start_j),
                ("chemical_energy_consumed_j", fuel.chemical_energy_consumed_j),
                ("chemical_energy_end_j", fuel.chemical_energy_end_j),
                ("required_reserve_j", fuel.required_reserve_j),
            ] {
                validate_nonnegative(value, name)?;
            }
            let residual = fuel.chemical_energy_start_j
                - fuel.chemical_energy_consumed_j
                - fuel.chemical_energy_end_j;
            let reserve_margin = fuel.chemical_energy_end_j - fuel.required_reserve_j;
            if residual.abs() > self.tolerance_j || reserve_margin < -self.tolerance_j {
                infeasible = true;
            }
            fuel_residuals.push(FuelEnergyResidual {
                fuel_source_id: fuel.fuel_source_id.clone(),
                residual_j: residual,
                reserve_margin_j: reserve_margin,
            });
        }

        let mut prime_mover_fuel_residuals = Vec::new();
        let mut prime_mover_shaft_residuals = Vec::new();
        let mut prime_mover_fuel_input_by_source = HashMap::<&str, f64>::new();
        for engine in &self.prime_movers {
            validate_digest(&engine.model_evidence_sha256, "prime mover model SHA-256")?;
            for (name, value) in [
                ("fuel_energy_input_j", engine.fuel_energy_input_j),
                ("mechanical_output_j", engine.mechanical_output_j),
                ("thermal_and_exhaust_loss_j", engine.thermal_and_exhaust_loss_j),
                ("direct_propulsor_shaft_j", engine.direct_propulsor_shaft_j),
                ("accessory_mechanical_load_j", engine.accessory_mechanical_load_j),
                ("mechanical_distribution_loss_j", engine.mechanical_distribution_loss_j),
            ] {
                validate_nonnegative(value, name)?;
            }
            if !fuel_by_id.contains_key(engine.fuel_source_id.as_str()) {
                return Err(format!(
                    "prime mover {} references missing fuel source {}",
                    engine.prime_mover_id, engine.fuel_source_id
                ));
            }
            *prime_mover_fuel_input_by_source
                .entry(engine.fuel_source_id.as_str())
                .or_insert(0.0) += engine.fuel_energy_input_j;
            let fuel_residual = engine.fuel_energy_input_j
                - engine.mechanical_output_j
                - engine.thermal_and_exhaust_loss_j;
            let generator_mechanical_input = self
                .generators
                .iter()
                .filter(|generator| generator.prime_mover_id == engine.prime_mover_id)
                .map(|generator| generator.mechanical_input_j)
                .sum::<f64>();
            let shaft_residual = engine.mechanical_output_j
                - generator_mechanical_input
                - engine.direct_propulsor_shaft_j
                - engine.accessory_mechanical_load_j
                - engine.mechanical_distribution_loss_j;
            if fuel_residual.abs() > self.tolerance_j
                || shaft_residual.abs() > self.tolerance_j
            {
                infeasible = true;
            }
            prime_mover_fuel_residuals.push(ComponentEnergyResidual {
                component_id: engine.prime_mover_id.clone(),
                residual_j: fuel_residual,
            });
            prime_mover_shaft_residuals.push(ComponentEnergyResidual {
                component_id: engine.prime_mover_id.clone(),
                residual_j: shaft_residual,
            });
        }

        for fuel in &self.fuel_states {
            let prime_mover_fuel_input = prime_mover_fuel_input_by_source
                .get(fuel.fuel_source_id.as_str())
                .copied()
                .unwrap_or(0.0);
            if (fuel.chemical_energy_consumed_j - prime_mover_fuel_input).abs()
                > self.tolerance_j
            {
                infeasible = true;
            }
        }

        for generator in &self.generators {
            validate_digest(&generator.model_evidence_sha256, "generator model SHA-256")?;
            if !engine_by_id.contains_key(generator.prime_mover_id.as_str()) {
                return Err(format!(
                    "generator {} references missing prime mover {}",
                    generator.generator_id, generator.prime_mover_id
                ));
            }
            for (name, value) in [
                ("mechanical_input_j", generator.mechanical_input_j),
                ("electrical_output_to_bus_j", generator.electrical_output_to_bus_j),
                ("conversion_loss_j", generator.conversion_loss_j),
            ] {
                validate_nonnegative(value, name)?;
            }
        }

        let generator_residuals = self
            .generators
            .iter()
            .map(|generator| {
                let residual = generator.mechanical_input_j
                    - generator.electrical_output_to_bus_j
                    - generator.conversion_loss_j;
                if residual.abs() > self.tolerance_j {
                    infeasible = true;
                }
                ComponentEnergyResidual {
                    component_id: generator.generator_id.clone(),
                    residual_j: residual,
                }
            })
            .collect::<Vec<_>>();

        let mut store_results = Vec::new();
        let mut total_store_discharge_to_bus_j = 0.0;
        let mut total_store_charge_from_bus_j = 0.0;
        for store in &self.stores {
            validate_digest(
                &store.installed_system_evidence_sha256,
                "installed-system evidence SHA-256",
            )?;
            validate_digest(
                &store.operating_envelope_evidence_sha256,
                "store operating-envelope evidence SHA-256",
            )?;
            for (name, value) in [
                ("usable_energy_start_j", store.usable_energy_start_j),
                ("usable_energy_end_j", store.usable_energy_end_j),
                ("usable_capacity_j", store.usable_capacity_j),
                ("reserve_floor_j", store.reserve_floor_j),
                ("discharge_to_bus_j", store.discharge_to_bus_j),
                ("charge_from_bus_j", store.charge_from_bus_j),
                ("average_discharge_power_limit_w", store.average_discharge_power_limit_w),
                ("peak_discharge_power_w", store.peak_discharge_power_w),
                ("peak_discharge_power_limit_w", store.peak_discharge_power_limit_w),
                ("average_charge_power_limit_w", store.average_charge_power_limit_w),
                ("peak_charge_power_w", store.peak_charge_power_w),
                ("peak_charge_power_limit_w", store.peak_charge_power_limit_w),
            ] {
                validate_nonnegative(value, name)?;
            }
            validate_efficiency(store.discharge_efficiency, "discharge_efficiency")?;
            validate_efficiency(store.charge_efficiency, "charge_efficiency")?;
            for (name, value) in [
                ("state_of_health_fraction", store.state_of_health_fraction),
                ("minimum_state_of_health_fraction", store.minimum_state_of_health_fraction),
            ] {
                if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                    return Err(format!("{name} must be finite and in [0, 1]"));
                }
            }
            for (name, value) in [
                ("temperature_c", store.temperature_c),
                ("minimum_temperature_c", store.minimum_temperature_c),
                ("maximum_temperature_c", store.maximum_temperature_c),
            ] {
                if !value.is_finite() {
                    return Err(format!("{name} must be finite"));
                }
            }
            if store.minimum_temperature_c > store.maximum_temperature_c {
                return Err("store minimum temperature exceeds maximum".to_owned());
            }
            if store.average_discharge_power_limit_w <= 0.0
                || store.peak_discharge_power_limit_w <= 0.0
                || store.average_charge_power_limit_w <= 0.0
                || store.peak_charge_power_limit_w <= 0.0
            {
                return Err("store power limits must be > 0".to_owned());
            }
            if store.usable_energy_start_j > store.usable_capacity_j + self.tolerance_j
                || store.usable_energy_end_j > store.usable_capacity_j + self.tolerance_j
            {
                infeasible = true;
            }
            let predicted_end = store.usable_energy_start_j
                - store.discharge_to_bus_j / store.discharge_efficiency
                + store.charge_from_bus_j * store.charge_efficiency;
            let residual = predicted_end - store.usable_energy_end_j;
            let reserve_margin = store.usable_energy_end_j - store.reserve_floor_j;
            let average_discharge = store.discharge_to_bus_j / self.duration_s;
            let average_charge = store.charge_from_bus_j / self.duration_s;
            if residual.abs() > self.tolerance_j
                || reserve_margin < -self.tolerance_j
                || store.state_of_health_fraction < store.minimum_state_of_health_fraction
                || store.temperature_c < store.minimum_temperature_c
                || store.temperature_c > store.maximum_temperature_c
                || average_discharge > store.average_discharge_power_limit_w
                || store.peak_discharge_power_w > store.peak_discharge_power_limit_w
                || average_charge > store.average_charge_power_limit_w
                || store.peak_charge_power_w > store.peak_charge_power_limit_w
            {
                infeasible = true;
            }
            total_store_discharge_to_bus_j += store.discharge_to_bus_j;
            total_store_charge_from_bus_j += store.charge_from_bus_j;
            store_results.push(StoreEnergyResult {
                store_id: store.store_id.clone(),
                predicted_usable_energy_end_j: predicted_end,
                actual_usable_energy_end_j: store.usable_energy_end_j,
                transition_residual_j: residual,
                reserve_margin_j: reserve_margin,
                average_discharge_power_w: average_discharge,
                average_charge_power_w: average_charge,
            });
        }

        let mut motor_residuals = Vec::new();
        let mut total_motor_bus_input_j = 0.0;
        let mut total_motor_shaft_output_j = 0.0;
        for motor in &self.motor_drives {
            validate_digest(&motor.model_evidence_sha256, "motor model SHA-256")?;
            for (name, value) in [
                ("motor bus_input_j", motor.bus_input_j),
                ("motor inverter_loss_j", motor.inverter_loss_j),
                ("motor motor_loss_j", motor.motor_loss_j),
                ("motor shaft_output_j", motor.shaft_output_j),
            ] {
                validate_nonnegative(value, name)?;
            }
            let residual =
                motor.bus_input_j - motor.inverter_loss_j - motor.motor_loss_j - motor.shaft_output_j;
            if residual.abs() > self.tolerance_j {
                infeasible = true;
            }
            total_motor_bus_input_j += motor.bus_input_j;
            total_motor_shaft_output_j += motor.shaft_output_j;
            motor_residuals.push(ComponentEnergyResidual {
                component_id: motor.motor_id.clone(),
                residual_j: residual,
            });
        }

        let total_generator_bus_output_j = self
            .generators
            .iter()
            .map(|generator| generator.electrical_output_to_bus_j)
            .sum::<f64>();
        let total_bus_supply_j = total_generator_bus_output_j + total_store_discharge_to_bus_j;
        let total_direct_mechanical_shaft_j = self
            .prime_movers
            .iter()
            .map(|engine| engine.direct_propulsor_shaft_j)
            .sum::<f64>();
        let total_propulsor_shaft_input_j =
            total_direct_mechanical_shaft_j + total_motor_shaft_output_j;
        let total_propulsor_shaft_declared_j = self
            .propulsors
            .iter()
            .map(|propulsor| propulsor.shaft_input_j)
            .sum::<f64>();

        let bus_demand_j = total_motor_bus_input_j
            + self.bus_loads.auxiliary_load_j
            + self.bus_loads.thermal_electrical_load_j
            + total_store_charge_from_bus_j
            + self.bus_loads.distribution_loss_j;
        let bus_residual_j = total_bus_supply_j - bus_demand_j;
        let shaft_combiner_residual_j =
            total_propulsor_shaft_input_j - total_propulsor_shaft_declared_j;
        if bus_residual_j.abs() > self.tolerance_j
            || shaft_combiner_residual_j.abs() > self.tolerance_j
        {
            infeasible = true;
        }

        let mut propulsor_residuals = Vec::new();
        for propulsor in &self.propulsors {
            validate_digest(&propulsor.model_evidence_sha256, "propulsor model SHA-256")?;
            for (name, value) in [
                ("propulsor shaft_input_j", propulsor.shaft_input_j),
                ("propulsor useful_propulsive_energy_j", propulsor.useful_propulsive_energy_j),
                ("propulsor propulsor_loss_j", propulsor.propulsor_loss_j),
            ] {
                validate_nonnegative(value, name)?;
            }
            let residual =
                propulsor.shaft_input_j - propulsor.useful_propulsive_energy_j - propulsor.propulsor_loss_j;
            if residual.abs() > self.tolerance_j {
                infeasible = true;
            }
            propulsor_residuals.push(ComponentEnergyResidual {
                component_id: propulsor.propulsor_id.clone(),
                residual_j: residual,
            });
        }

        let continuous_bus_supply_power_w = total_bus_supply_j / self.duration_s;
        let peak_source_power_margin_w = self.peak_source_power_limit_w - self.peak_source_power_w;
        let continuous_source_power_margin_w =
            self.continuous_source_power_limit_w - continuous_bus_supply_power_w;
        if peak_source_power_margin_w < 0.0
            || continuous_source_power_margin_w < 0.0
            || self.peak_source_power_w + self.tolerance_j < continuous_bus_supply_power_w
        {
            infeasible = true;
        }
        if self.prime_movers.is_empty()
            && self.generators.is_empty()
            && self.stores.is_empty()
        {
            return Err("phase energy balance has no declared energy source or store".to_owned());
        }
        if self.motor_drives.is_empty() || self.propulsors.is_empty() {
            return Err("phase energy balance requires at least one motor-drive and propulsor record".to_owned());
        }

        Ok(PhaseEnergyReport {
            schema_version: "phase-energy-report/0.2.0",
            evidence_level: "screening_only",
            mission_id: self.mission_id.clone(),
            phase_id: self.phase_id.clone(),
            configuration_sha256: self.configuration_sha256.clone(),
            start_state_sha256: self.start_state_sha256.clone(),
            end_state_sha256: self.end_state_sha256.clone(),
            status: if infeasible {
                PhaseEnergyStatus::Infeasible
            } else {
                PhaseEnergyStatus::ConservedWithinDeclaredModel
            },
            bus_residual_j,
            shaft_combiner_residual_j,
            fuel_residuals,
            prime_mover_fuel_residuals,
            prime_mover_shaft_residuals,
            generator_residuals,
            store_results,
            motor_residuals,
            propulsor_residuals,
            continuous_bus_supply_power_w,
            peak_source_power_margin_w,
            continuous_source_power_margin_w,
        })
    }
}

fn validate_nonnegative(value: f64, name: &str) -> Result<(), String> {
    if !value.is_finite() || value < 0.0 {
        return Err(format!("{name} must be finite and >= 0"));
    }
    Ok(())
}

fn validate_efficiency(value: f64, name: &str) -> Result<(), String> {
    if !value.is_finite() || !(0.0 < value && value <= 1.0) {
        return Err(format!("{name} must be finite and in (0, 1]"));
    }
    Ok(())
}

fn validate_digest(value: &str, name: &str) -> Result<(), String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("{name} must be 64 hexadecimal characters"));
    }
    Ok(())
}

fn validate_nonempty(value: &str, name: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{name} must not be empty"));
    }
    Ok(())
}

fn require_unique<'a>(
    values: impl Iterator<Item = &'a str>,
    label: &str,
) -> Result<(), String> {
    let mut seen = HashSet::new();
    for value in values {
        validate_nonempty(value, label)?;
        if !seen.insert(value) {
            return Err(format!("duplicate {label} ID: {value}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const MISSION: &str = "SYNTH-ENERGY-001";
    const START: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const END: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const MODEL: &str = "1111111111111111111111111111111111111111111111111111111111111111";

    fn battery_store(discharge: f64, charge: f64, start: f64, end: f64) -> EnergyStoreTransition {
        EnergyStoreTransition {
            store_id: "battery".to_owned(),
            kind: EnergyStoreKind::Battery,
            installed_system_evidence_sha256: MODEL.to_owned(),
            operating_envelope_evidence_sha256: MODEL.to_owned(),
            usable_energy_start_j: start,
            usable_energy_end_j: end,
            usable_capacity_j: 300_000_000.0,
            reserve_floor_j: 40_000_000.0,
            discharge_to_bus_j: discharge,
            charge_from_bus_j: charge,
            discharge_efficiency: 0.9,
            charge_efficiency: 0.9,
            state_of_health_fraction: 0.95,
            minimum_state_of_health_fraction: 0.8,
            temperature_c: 25.0,
            minimum_temperature_c: -20.0,
            maximum_temperature_c: 60.0,
            average_discharge_power_limit_w: 50_000.0,
            peak_discharge_power_w: 45_000.0,
            peak_discharge_power_limit_w: 60_000.0,
            average_charge_power_limit_w: 30_000.0,
            peak_charge_power_w: 10_000.0,
            peak_charge_power_limit_w: 20_000.0,
        }
    }

    fn motor(bus: f64, inverter: f64, motor_loss: f64, shaft: f64) -> MotorDriveConversion {
        MotorDriveConversion {
            motor_id: "motor".to_owned(),
            model_evidence_sha256: MODEL.to_owned(),
            bus_input_j: bus,
            inverter_loss_j: inverter,
            motor_loss_j: motor_loss,
            shaft_output_j: shaft,
        }
    }

    fn propulsor(input: f64, useful: f64, loss: f64) -> PropulsorShaftBalance {
        PropulsorShaftBalance {
            propulsor_id: "propulsor".to_owned(),
            model_evidence_sha256: MODEL.to_owned(),
            shaft_input_j: input,
            useful_propulsive_energy_j: useful,
            propulsor_loss_j: loss,
        }
    }

    fn balance() -> PhaseEnergyBalance {
        PhaseEnergyBalance {
            schema_version: PHASE_ENERGY_SCHEMA_VERSION,
            mission_id: MISSION.to_owned(),
            phase_id: "battery-only".to_owned(),
            configuration_sha256: CONFIG.to_owned(),
            start_state_sha256: START.to_owned(),
            end_state_sha256: END.to_owned(),
            duration_s: 3600.0,
            tolerance_j: 1.0,
            fuel_states: vec![],
            prime_movers: vec![],
            generators: vec![],
            stores: vec![battery_store(
                100_000_000.0,
                0.0,
                200_000_000.0,
                88_888_888.888_888_9,
            )],
            bus_loads: BusLoads {
                auxiliary_load_j: 5_000_000.0,
                thermal_electrical_load_j: 5_000_000.0,
                distribution_loss_j: 10_000_000.0,
            },
            motor_drives: vec![motor(80_000_000.0, 4_000_000.0, 4_000_000.0, 72_000_000.0)],
            propulsors: vec![propulsor(
                72_000_000.0,
                60_000_000.0,
                12_000_000.0,
            )],
            peak_source_power_w: 40_000.0,
            peak_source_power_limit_w: 50_000.0,
            continuous_source_power_limit_w: 35_000.0,
        }
    }

    fn generator_only() -> PhaseEnergyBalance {
        let mut value = balance();
        value.phase_id = "generator-only".to_owned();
        value.stores.clear();
        value.fuel_states = vec![FuelStateTransition {
            fuel_source_id: "fuel-tank".to_owned(),
            evidence_sha256: MODEL.to_owned(),
            chemical_energy_start_j: 500_000_000.0,
            chemical_energy_consumed_j: 120_000_000.0,
            chemical_energy_end_j: 380_000_000.0,
            required_reserve_j: 100_000_000.0,
        }];
        value.prime_movers = vec![PrimeMoverConversion {
            prime_mover_id: "engine".to_owned(),
            model_evidence_sha256: MODEL.to_owned(),
            fuel_source_id: "fuel-tank".to_owned(),
            fuel_energy_input_j: 120_000_000.0,
            mechanical_output_j: 40_000_000.0,
            thermal_and_exhaust_loss_j: 80_000_000.0,
            direct_propulsor_shaft_j: 0.0,
            accessory_mechanical_load_j: 0.0,
            mechanical_distribution_loss_j: 0.0,
        }];
        value.generators = vec![GeneratorConversion {
            generator_id: "generator".to_owned(),
            model_evidence_sha256: MODEL.to_owned(),
            prime_mover_id: "engine".to_owned(),
            mechanical_input_j: 40_000_000.0,
            electrical_output_to_bus_j: 34_000_000.0,
            conversion_loss_j: 6_000_000.0,
        }];
        value.bus_loads = BusLoads {
            auxiliary_load_j: 4_000_000.0,
            thermal_electrical_load_j: 2_000_000.0,
            distribution_loss_j: 3_000_000.0,
        };
        value.motor_drives = vec![motor(25_000_000.0, 1_000_000.0, 2_000_000.0, 22_000_000.0)];
        value.propulsors = vec![propulsor(22_000_000.0, 18_000_000.0, 4_000_000.0)];
        value.peak_source_power_w = 20_000.0;
        value.peak_source_power_limit_w = 30_000.0;
        value.continuous_source_power_limit_w = 15_000.0;
        value
    }

    #[test]
    fn battery_only_discharge_closes_internal_energy_and_bus_planes() {
        let value = balance();
        let report = value.evaluate(CONFIG, MISSION, START, END).expect("balanced battery-only phase");
        assert_eq!(report.status, PhaseEnergyStatus::ConservedWithinDeclaredModel);
        assert!(report.bus_residual_j.abs() < 1.0);
        assert!((report.store_results[0].transition_residual_j).abs() < 1.0);
        assert!(report.store_results[0].reserve_margin_j > 0.0);
    }

    #[test]
    fn generator_only_keeps_fuel_and_electrical_planes_distinct() {
        let report = generator_only()
            .evaluate(CONFIG, MISSION, START, END)
            .expect("balanced generator-only phase");
        assert_eq!(report.status, PhaseEnergyStatus::ConservedWithinDeclaredModel);
        assert!(report.fuel_residuals[0].reserve_margin_j > 0.0);
        assert!(report.generator_residuals[0].residual_j.abs() < 1.0);
    }

    #[test]
    fn series_hybrid_combines_generator_and_storage_at_bus() {
        let mut value = generator_only();
        value.phase_id = "series-hybrid".to_owned();
        value.prime_movers[0].mechanical_output_j = 40_000_000.0;
        value.prime_movers[0].thermal_and_exhaust_loss_j = 80_000_000.0;
        value.generators[0].electrical_output_to_bus_j = 34_000_000.0;
        value.stores = vec![battery_store(
            72_000_000.0,
            6_000_000.0,
            200_000_000.0,
            125_400_000.0,
        )];
        value.bus_loads = BusLoads {
            auxiliary_load_j: 5_000_000.0,
            thermal_electrical_load_j: 5_000_000.0,
            distribution_loss_j: 10_000_000.0,
        };
        value.motor_drives = vec![motor(80_000_000.0, 4_000_000.0, 4_000_000.0, 72_000_000.0)];
        value.propulsors = vec![propulsor(72_000_000.0, 60_000_000.0, 12_000_000.0)];
        value.peak_source_power_w = 45_000.0;
        value.peak_source_power_limit_w = 55_000.0;
        value.continuous_source_power_limit_w = 40_000.0;
        let report = value.evaluate(CONFIG, MISSION, START, END).expect("balanced series hybrid");
        assert_eq!(report.status, PhaseEnergyStatus::ConservedWithinDeclaredModel);
        assert!(report.bus_residual_j.abs() < 1.0);
        assert!(report.store_results[0].transition_residual_j.abs() < 1.0);
    }

    #[test]
    fn parallel_hybrid_keeps_direct_shaft_and_electric_bus_separate() {
        let mut value = generator_only();
        value.phase_id = "parallel-hybrid".to_owned();
        value.fuel_states[0].chemical_energy_start_j = 500_000_000.0;
        value.fuel_states[0].chemical_energy_consumed_j = 300_000_000.0;
        value.fuel_states[0].chemical_energy_end_j = 200_000_000.0;
        value.prime_movers[0].fuel_energy_input_j = 300_000_000.0;
        value.prime_movers[0].mechanical_output_j = 100_000_000.0;
        value.prime_movers[0].thermal_and_exhaust_loss_j = 200_000_000.0;
        value.prime_movers[0].direct_propulsor_shaft_j = 80_000_000.0;
        value.generators[0].mechanical_input_j = 20_000_000.0;
        value.generators[0].electrical_output_to_bus_j = 16_000_000.0;
        value.generators[0].conversion_loss_j = 4_000_000.0;
        value.stores = vec![battery_store(
            30_000_000.0,
            0.0,
            100_000_000.0,
            66_666_666.666_666_67,
        )];
        value.bus_loads = BusLoads {
            auxiliary_load_j: 5_000_000.0,
            thermal_electrical_load_j: 5_000_000.0,
            distribution_loss_j: 6_000_000.0,
        };
        value.motor_drives = vec![motor(30_000_000.0, 2_000_000.0, 1_000_000.0, 27_000_000.0)];
        value.propulsors = vec![propulsor(107_000_000.0, 90_000_000.0, 17_000_000.0)];
        value.peak_source_power_w = 50_000.0;
        value.peak_source_power_limit_w = 55_000.0;
        value.continuous_source_power_limit_w = 20_000.0;
        let report = value.evaluate(CONFIG, MISSION, START, END).expect("balanced parallel hybrid");
        assert_eq!(report.status, PhaseEnergyStatus::ConservedWithinDeclaredModel);
        assert!(report.bus_residual_j.abs() < 1.0);
        assert!(report.prime_mover_shaft_residuals[0].residual_j.abs() < 1.0);
    }

    #[test]
    fn store_charging_is_not_mistaken_for_discharge() {
        let mut value = generator_only();
        value.phase_id = "generator-charge".to_owned();
        value.prime_movers[0].mechanical_output_j = 120_000_000.0;
        value.prime_movers[0].thermal_and_exhaust_loss_j = 180_000_000.0;
        value.generators[0].mechanical_input_j = 120_000_000.0;
        value.generators[0].electrical_output_to_bus_j = 100_000_000.0;
        value.generators[0].conversion_loss_j = 20_000_000.0;
        value.fuel_states[0].chemical_energy_consumed_j = 300_000_000.0;
        value.fuel_states[0].chemical_energy_end_j = 200_000_000.0;
        value.fuel_states[0].chemical_energy_start_j = 500_000_000.0;
        value.stores = vec![battery_store(
            0.0,
            30_000_000.0,
            100_000_000.0,
            127_000_000.0,
        )];
        value.bus_loads = BusLoads {
            auxiliary_load_j: 5_000_000.0,
            thermal_electrical_load_j: 5_000_000.0,
            distribution_loss_j: 10_000_000.0,
        };
        value.motor_drives = vec![motor(50_000_000.0, 2_000_000.0, 3_000_000.0, 45_000_000.0)];
        value.propulsors = vec![propulsor(45_000_000.0, 38_000_000.0, 7_000_000.0)];
        let report = value.evaluate(CONFIG, MISSION, START, END).expect("balanced charging phase");
        assert_eq!(report.status, PhaseEnergyStatus::ConservedWithinDeclaredModel);
        assert!(report.store_results[0].transition_residual_j.abs() < 1.0);
        assert!(report.store_results[0].average_charge_power_w > 0.0);
        assert_eq!(report.store_results[0].average_discharge_power_w, 0.0);
    }

    #[test]
    fn two_stores_keep_separate_conversion_efficiencies_and_reserves() {
        let mut value = generator_only();
        value.phase_id = "two-store".to_owned();
        value.prime_movers[0].fuel_energy_input_j = 200_000_000.0;
        value.prime_movers[0].mechanical_output_j = 80_000_000.0;
        value.prime_movers[0].thermal_and_exhaust_loss_j = 120_000_000.0;
        value.generators[0].mechanical_input_j = 80_000_000.0;
        value.generators[0].electrical_output_to_bus_j = 70_000_000.0;
        value.generators[0].conversion_loss_j = 10_000_000.0;
        value.fuel_states[0].chemical_energy_consumed_j = 200_000_000.0;
        value.fuel_states[0].chemical_energy_end_j = 300_000_000.0;
        value.fuel_states[0].chemical_energy_start_j = 500_000_000.0;
        let mut capacitor = battery_store(
            10_000_000.0,
            0.0,
            40_000_000.0,
            28_888_888.888_888_89,
        );
        capacitor.store_id = "supercapacitor".to_owned();
        capacitor.kind = EnergyStoreKind::Supercapacitor;
        let mut battery = battery_store(
            20_000_000.0,
            0.0,
            200_000_000.0,
            177_777_777.777_777_77,
        );
        battery.average_discharge_power_limit_w = 30_000.0;
        value.stores = vec![battery, capacitor];
        value.bus_loads = BusLoads {
            auxiliary_load_j: 5_000_000.0,
            thermal_electrical_load_j: 5_000_000.0,
            distribution_loss_j: 10_000_000.0,
        };
        value.motor_drives = vec![motor(80_000_000.0, 4_000_000.0, 4_000_000.0, 72_000_000.0)];
        value.propulsors = vec![propulsor(72_000_000.0, 60_000_000.0, 12_000_000.0)];
        let report = value.evaluate(CONFIG, MISSION, START, END).expect("balanced battery+capacitor");
        assert_eq!(report.status, PhaseEnergyStatus::ConservedWithinDeclaredModel);
        assert_eq!(report.store_results.len(), 2);
        assert_ne!(
            report.store_results[0].reserve_margin_j,
            report.store_results[1].reserve_margin_j
        );
    }

    #[test]
    fn insufficient_storage_reserve_is_infeasible_not_a_pass() {
        let mut value = balance();
        value.stores[0].reserve_floor_j = value.stores[0].usable_energy_end_j + 1.0;
        let report = value.evaluate(CONFIG, MISSION, START, END).expect("report infeasible case");
        assert_eq!(report.status, PhaseEnergyStatus::Infeasible);
        assert!(report.store_results[0].reserve_margin_j < 0.0);
    }

    #[test]
    fn storage_power_overrun_is_infeasible() {
        let mut value = balance();
        value.stores[0].peak_discharge_power_w = 70_000.0;
        let report = value.evaluate(CONFIG, MISSION, START, END).expect("report power failure");
        assert_eq!(report.status, PhaseEnergyStatus::Infeasible);
    }

    #[test]
    fn stale_configuration_or_state_identity_is_rejected() {
        let value = balance();
        let other = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
        assert!(value.evaluate(other, MISSION, START, END).is_err());
        assert!(value.evaluate(CONFIG, MISSION, other, END).is_err());
    }

    #[test]
    fn reserve_carriers_are_not_added_into_one_total() {
        let mut value = generator_only();
        value.fuel_states[0].chemical_energy_end_j = 80_000_000.0;
        value.fuel_states[0].required_reserve_j = 100_000_000.0;
        let report = value.evaluate(CONFIG, MISSION, START, END).expect("report fuel reserve failure");
        assert_eq!(report.status, PhaseEnergyStatus::Infeasible);
        assert!(report.fuel_residuals[0].reserve_margin_j < 0.0);
    }

    #[test]
    fn missing_store_evidence_is_rejected() {
        let mut value = balance();
        value.stores[0].operating_envelope_evidence_sha256 = "missing".to_owned();
        assert!(value.evaluate(CONFIG, MISSION, START, END).is_err());
    }
}
