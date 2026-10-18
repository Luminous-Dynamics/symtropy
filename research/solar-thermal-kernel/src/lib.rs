// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Isolated solar-capture and sensible-thermal-storage research kernel.
//! This is a bounded fixture model, not a validated plant model.
//! Units: energy J, power W, time s, absolute temperature K, mass kg,
//! and specific heat J/(kg K). Store energy is measured relative to Tmin.

use std::{error::Error, fmt, fmt::Write as _};

pub mod cascade;
pub mod loss_integrator;
pub mod pinch;
pub mod scenario_risk;
pub mod solar_input;
pub mod temporal;

#[derive(Clone, Debug, PartialEq)]
pub enum ModelError {
    NonFinite(&'static str),
    MustBePositive(&'static str),
    MustBeNonNegative(&'static str),
    FractionOutsideUnitInterval,
    HeatCapacityCurveTooShort,
    HeatCapacityTemperaturesNotIncreasing,
    TemperatureOutsideHeatCapacityCurve,
    InvalidTemperatureRange,
    InitialTemperatureOutOfRange,
    ReceiverLossExceedsAbsorbedEnergy,
    InvalidDischargeEfficiency,
    EnergyAboveStoreCapacity,
    TemperatureBelowReference,
    ReportMismatch(&'static str),
    InvalidHotStreamOrder,
    InvalidColdStreamOrder,
    NoProcessStreams,
    EmptyTemporalSchedule,
    PeriodDurationMismatch,
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite(n) => write!(f, "{n} must be finite"),
            Self::MustBePositive(n) => write!(f, "{n} must be positive"),
            Self::MustBeNonNegative(n) => write!(f, "{n} must be non-negative"),
            Self::FractionOutsideUnitInterval => write!(f, "fraction must be in [0, 1]"),
            Self::HeatCapacityCurveTooShort => write!(f, "cp(T) requires at least two points"),
            Self::HeatCapacityTemperaturesNotIncreasing => write!(f, "cp(T) temperatures must strictly increase"),
            Self::TemperatureOutsideHeatCapacityCurve => write!(f, "temperature is outside declared cp(T) bounds"),
            Self::InvalidTemperatureRange => write!(f, "minimum temperature must be below maximum"),
            Self::InitialTemperatureOutOfRange => write!(f, "store temperature is outside configured bounds"),
            Self::ReceiverLossExceedsAbsorbedEnergy => write!(f, "receiver loss exceeds absorbed solar energy"),
            Self::InvalidDischargeEfficiency => write!(f, "exchanger effectiveness must be in (0, 1]"),
            Self::EnergyAboveStoreCapacity => write!(f, "requested stored energy exceeds configured capacity"),
            Self::TemperatureBelowReference => write!(f, "thermal exergy requires the final temperature to be at or above the reference temperature"),
            Self::ReportMismatch(n) => write!(f, "independent report verification failed: {n}"),
            Self::InvalidHotStreamOrder => write!(f, "hot stream supply temperature must exceed target temperature"),
            Self::InvalidColdStreamOrder => write!(f, "cold stream target temperature must exceed supply temperature"),
            Self::NoProcessStreams => write!(f, "pinch analysis requires at least one hot and one cold process stream"),
            Self::EmptyTemporalSchedule => write!(f, "temporal schedule must contain at least one period"),
            Self::PeriodDurationMismatch => write!(f, "discharge request duration must match its containing period"),
        }
    }
}
impl Error for ModelError {}

fn finite(x: f64, name: &'static str) -> Result<f64, ModelError> {
    if x.is_finite() { Ok(x) } else { Err(ModelError::NonFinite(name)) }
}
fn nonnegative(x: f64, name: &'static str) -> Result<f64, ModelError> {
    finite(x, name)?;
    if x < 0.0 { Err(ModelError::MustBeNonNegative(name)) } else { Ok(x) }
}
fn positive(x: f64, name: &'static str) -> Result<f64, ModelError> {
    finite(x, name)?;
    if x <= 0.0 { Err(ModelError::MustBePositive(name)) } else { Ok(x) }
}
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1.0e-8 * a.abs().max(b.abs()).max(1.0)
}
fn check(name: &'static str, actual: f64, expected: f64) -> Result<(), ModelError> {
    if close(actual, expected) { Ok(()) } else { Err(ModelError::ReportMismatch(name)) }
}

macro_rules! nonnegative_unit {
    ($name:ident, $field:literal) => {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct $name(f64);
        impl $name {
            pub fn new(value: f64) -> Result<Self, ModelError> {
                nonnegative(value, $field).map(Self)
            }
            pub fn value(self) -> f64 { self.0 }
        }
    };
}
macro_rules! positive_unit {
    ($name:ident, $field:literal) => {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct $name(f64);
        impl $name {
            pub fn new(value: f64) -> Result<Self, ModelError> {
                positive(value, $field).map(Self)
            }
            pub fn value(self) -> f64 { self.0 }
        }
    };
}
nonnegative_unit!(EnergyJ, "energy_j");
nonnegative_unit!(ExergyJ, "exergy_j");
nonnegative_unit!(SpecificExergyJPerKg, "specific_exergy_j_per_kg");
positive_unit!(Kelvin, "temperature_k");
nonnegative_unit!(TemperatureDifferenceK, "temperature_difference_k");
positive_unit!(MassKg, "mass_kg");
positive_unit!(SpecificHeatJPerKgK, "specific_heat_j_per_kg_k");
positive_unit!(PowerW, "power_w");
positive_unit!(Seconds, "duration_s");
nonnegative_unit!(ThermalConductanceWPerK, "thermal_conductance_w_per_k");

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fraction(f64);
impl Fraction {
    pub fn new(value: f64) -> Result<Self, ModelError> {
        finite(value, "fraction")?;
        if !(0.0..=1.0).contains(&value) {
            return Err(ModelError::FractionOutsideUnitInterval);
        }
        Ok(Self(value))
    }
    pub fn value(self) -> f64 { self.0 }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeatCapacityPoint {
    temperature: Kelvin,
    specific_heat: SpecificHeatJPerKgK,
}
impl HeatCapacityPoint {
    pub fn new(temperature: Kelvin, specific_heat: SpecificHeatJPerKgK) -> Self {
        Self { temperature, specific_heat }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HeatCapacityCurve {
    points: Vec<HeatCapacityPoint>,
}
impl HeatCapacityCurve {
    pub fn new(points: Vec<HeatCapacityPoint>) -> Result<Self, ModelError> {
        if points.len() < 2 { return Err(ModelError::HeatCapacityCurveTooShort); }
        if points.windows(2).any(|w| w[0].temperature.value() >= w[1].temperature.value()) {
            return Err(ModelError::HeatCapacityTemperaturesNotIncreasing);
        }
        Ok(Self { points })
    }

    pub fn temperature_bounds(&self) -> (Kelvin, Kelvin) {
        (self.points[0].temperature, self.points[self.points.len() - 1].temperature)
    }

    /// Integrates piecewise-linear cp(T) with exact trapezoids. Result is signed J/kg.
    pub fn specific_enthalpy(&self, from: Kelvin, to: Kelvin) -> Result<f64, ModelError> {
        let (minimum, maximum) = self.temperature_bounds();
        for t in [from, to] {
            if t.value() < minimum.value() || t.value() > maximum.value() {
                return Err(ModelError::TemperatureOutsideHeatCapacityCurve);
            }
        }
        if from.value() == to.value() { return Ok(0.0); }
        if from.value() > to.value() {
            return Ok(-self.integrate_forward(to.value(), from.value()));
        }
        Ok(self.integrate_forward(from.value(), to.value()))
    }

    /// Computes sensible thermal exergy per kg relative to a reference
    /// environment temperature. Assumes a homogeneous solid/liquid store in
    /// local equilibrium with cp(T) defined by this curve:
    ///
    /// exergy / kg = integral[cp(T) * (1 - T_ref / T), T_ref..T]
    ///
    /// Both temperatures must lie within the measured/declared cp(T) curve,
    /// and T >= T_ref. This is not latent or chemical exergy.
    pub fn specific_exergy(
        &self,
        reference_temperature: Kelvin,
        temperature: Kelvin,
    ) -> Result<SpecificExergyJPerKg, ModelError> {
        let (minimum, maximum) = self.temperature_bounds();
        for t in [reference_temperature, temperature] {
            if t.value() < minimum.value() || t.value() > maximum.value() {
                return Err(ModelError::TemperatureOutsideHeatCapacityCurve);
            }
        }
        if temperature.value() < reference_temperature.value() {
            return Err(ModelError::TemperatureBelowReference);
        }
        if temperature.value() == reference_temperature.value() {
            return SpecificExergyJPerKg::new(0.0);
        }

        let sensible_enthalpy =
            self.specific_enthalpy(reference_temperature, temperature)?;
        let cp_over_t = self.integrate_cp_over_temperature(
            reference_temperature.value(),
            temperature.value(),
        );
        let exergy = sensible_enthalpy
            - reference_temperature.value() * cp_over_t;
        // Roundoff near the reference temperature can produce a tiny negative.
        SpecificExergyJPerKg::new(exergy.max(0.0))
    }

    fn integrate_cp_over_temperature(&self, low: f64, high: f64) -> f64 {
        let mut sum = 0.0;
        for pair in self.points.windows(2) {
            let t0 = pair[0].temperature.value();
            let t1 = pair[1].temperature.value();
            let a = low.max(t0);
            let b = high.min(t1);
            if b <= a {
                continue;
            }
            let cp0 = pair[0].specific_heat.value();
            let cp1 = pair[1].specific_heat.value();
            let slope = (cp1 - cp0) / (t1 - t0);
            // On this segment, cp(T) = intercept + slope * T.
            let intercept = cp0 - slope * t0;
            sum += intercept * (b / a).ln() + slope * (b - a);
        }
        sum
    }

    fn integrate_forward(&self, low: f64, high: f64) -> f64 {
        let mut sum = 0.0;
        for pair in self.points.windows(2) {
            let t0 = pair[0].temperature.value();
            let t1 = pair[1].temperature.value();
            let a = low.max(t0);
            let b = high.min(t1);
            if b <= a { continue; }
            let cp0 = pair[0].specific_heat.value();
            let cp1 = pair[1].specific_heat.value();
            let slope = (cp1 - cp0) / (t1 - t0);
            let cp_a = cp0 + slope * (a - t0);
            let cp_b = cp0 + slope * (b - t0);
            sum += 0.5 * (cp_a + cp_b) * (b - a);
        }
        sum
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollectorInput {
    /// Incident interval energy after choosing the appropriate irradiance basis.
    /// This is not raw GHI/DNI data; radiometry and aperture geometry are out of scope.
    pub incident_solar: EnergyJ,
    pub optical_efficiency: Fraction,
    /// Interval-integrated receiver thermal loss at the stated boundary.
    pub receiver_loss: EnergyJ,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollectorReport {
    pub incident_solar: EnergyJ,
    pub optical_loss: EnergyJ,
    pub absorbed_solar: EnergyJ,
    pub receiver_loss: EnergyJ,
    pub useful_captured_heat: EnergyJ,
}

pub fn capture_solar(input: CollectorInput) -> Result<CollectorReport, ModelError> {
    let absorbed = input.incident_solar.value() * input.optical_efficiency.value();
    if input.receiver_loss.value() > absorbed {
        return Err(ModelError::ReceiverLossExceedsAbsorbedEnergy);
    }
    Ok(CollectorReport {
        incident_solar: input.incident_solar,
        optical_loss: EnergyJ::new((input.incident_solar.value() - absorbed).max(0.0))?,
        absorbed_solar: EnergyJ::new(absorbed)?,
        receiver_loss: input.receiver_loss,
        useful_captured_heat: EnergyJ::new((absorbed - input.receiver_loss.value()).max(0.0))?,
    })
}

/// Recomputes the collector report from the input, rather than trusting reported totals.
pub fn verify_collector_report(input: CollectorInput, claimed: CollectorReport) -> Result<(), ModelError> {
    let absorbed = input.incident_solar.value() * input.optical_efficiency.value();
    if input.receiver_loss.value() > absorbed {
        return Err(ModelError::ReceiverLossExceedsAbsorbedEnergy);
    }
    let optical_loss = input.incident_solar.value() - absorbed;
    let useful = absorbed - input.receiver_loss.value();
    check("collector.incident", claimed.incident_solar.value(), input.incident_solar.value())?;
    check("collector.absorbed", claimed.absorbed_solar.value(), absorbed)?;
    check("collector.optical_loss", claimed.optical_loss.value(), optical_loss)?;
    check("collector.receiver_loss", claimed.receiver_loss.value(), input.receiver_loss.value())?;
    check("collector.useful_heat", claimed.useful_captured_heat.value(), useful)?;
    check("collector.balance",
        claimed.optical_loss.value() + claimed.receiver_loss.value() + claimed.useful_captured_heat.value(),
        input.incident_solar.value())
}

#[derive(Clone, Debug, PartialEq)]
pub struct StoreConfigSpec {
    pub mass: MassKg,
    pub heat_capacity: HeatCapacityCurve,
    pub min_temperature: Kelvin,
    pub max_temperature: Kelvin,
    pub initial_temperature: Kelvin,
    pub max_charge_power: PowerW,
    pub max_discharge_power: PowerW,
    pub standing_loss_conductance: ThermalConductanceWPerK,
}
#[derive(Clone, Debug, PartialEq)]
pub struct StoreConfig {
    mass: MassKg,
    heat_capacity: HeatCapacityCurve,
    min_temperature: Kelvin,
    max_temperature: Kelvin,
    initial_temperature: Kelvin,
    max_charge_power: PowerW,
    max_discharge_power: PowerW,
    standing_loss_conductance: ThermalConductanceWPerK,
}
impl StoreConfig {
    pub fn new(s: StoreConfigSpec) -> Result<Self, ModelError> {
        if s.min_temperature.value() >= s.max_temperature.value() {
            return Err(ModelError::InvalidTemperatureRange);
        }
        let (curve_min, curve_max) = s.heat_capacity.temperature_bounds();
        if s.min_temperature.value() < curve_min.value() || s.max_temperature.value() > curve_max.value() {
            return Err(ModelError::TemperatureOutsideHeatCapacityCurve);
        }
        if s.initial_temperature.value() < s.min_temperature.value()
            || s.initial_temperature.value() > s.max_temperature.value() {
            return Err(ModelError::InitialTemperatureOutOfRange);
        }
        Ok(Self {
            mass: s.mass, heat_capacity: s.heat_capacity, min_temperature: s.min_temperature,
            max_temperature: s.max_temperature, initial_temperature: s.initial_temperature,
            max_charge_power: s.max_charge_power, max_discharge_power: s.max_discharge_power,
            standing_loss_conductance: s.standing_loss_conductance,
        })
    }
    pub fn initial_state(&self) -> StoreState { StoreState { temperature: self.initial_temperature } }
    pub fn stored_energy(&self, state: StoreState) -> Result<EnergyJ, ModelError> {
        stored_energy_at(self, state.temperature)
    }

    /// Sensible thermal exergy relative to the stated environment temperature.
    /// The reference and state temperatures must lie within cp(T)'s domain.
    pub fn stored_exergy(
        &self,
        state: StoreState,
        reference_temperature: Kelvin,
    ) -> Result<ExergyJ, ModelError> {
        stored_energy_at(self, state.temperature)?;
        let specific = self.heat_capacity
            .specific_exergy(reference_temperature, state.temperature)?;
        ExergyJ::new(self.mass.value() * specific.value())
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StoreState {
    pub temperature: Kelvin,
}
fn stored_energy_at(c: &StoreConfig, t: Kelvin) -> Result<EnergyJ, ModelError> {
    if t.value() < c.min_temperature.value() || t.value() > c.max_temperature.value() {
        return Err(ModelError::InitialTemperatureOutOfRange);
    }
    let h = c.heat_capacity.specific_enthalpy(c.min_temperature, t)?;
    EnergyJ::new(c.mass.value() * h.max(0.0))
}
fn temperature_at_energy(c: &StoreConfig, target: EnergyJ) -> Result<Kelvin, ModelError> {
    let cap = stored_energy_at(c, c.max_temperature)?.value();
    if target.value() > cap + 1.0e-8 * cap.max(1.0) {
        return Err(ModelError::EnergyAboveStoreCapacity);
    }
    let target = target.value().min(cap).max(0.0);
    let (mut low, mut high) = (c.min_temperature.value(), c.max_temperature.value());
    for _ in 0..80 {
        let mid = 0.5 * (low + high);
        if stored_energy_at(c, Kelvin::new(mid)?)?.value() < target { low = mid; } else { high = mid; }
    }
    Kelvin::new(0.5 * (low + high))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChargeReport {
    pub temperature_before: Kelvin,
    pub temperature_after: Kelvin,
    pub requested_input: EnergyJ,
    pub accepted_input: EnergyJ,
    pub rejected_input: EnergyJ,
}
pub fn charge_store(c: &StoreConfig, s: StoreState, input: EnergyJ, dt: Seconds)
    -> Result<(StoreState, ChargeReport), ModelError> {
    let before = stored_energy_at(c, s.temperature)?.value();
    let capacity = stored_energy_at(c, c.max_temperature)?.value();
    let room = (capacity - before).max(0.0);
    let accepted = input.value().min(c.max_charge_power.value() * dt.value()).min(room);
    let after_t = temperature_at_energy(c, EnergyJ::new(before + accepted)?)?;
    let report = ChargeReport {
        temperature_before: s.temperature, temperature_after: after_t, requested_input: input,
        accepted_input: EnergyJ::new(accepted)?,
        rejected_input: EnergyJ::new((input.value() - accepted).max(0.0))?,
    };
    Ok((StoreState { temperature: after_t }, report))
}
pub fn verify_charge_report(c: &StoreConfig, s: StoreState, input: EnergyJ, dt: Seconds,
    r: ChargeReport) -> Result<(), ModelError> {
    let before = stored_energy_at(c, s.temperature)?.value();
    let cap = stored_energy_at(c, c.max_temperature)?.value();
    let room = (cap - before).max(0.0);
    let expected_in = input.value().min(room).min(c.max_charge_power.value() * dt.value());
    let expected_out = (input.value() - expected_in).max(0.0);
    let expected_t = temperature_at_energy(c, EnergyJ::new(before + expected_in)?)?;
    check("charge.temp_before", r.temperature_before.value(), s.temperature.value())?;
    check("charge.temp_after", r.temperature_after.value(), expected_t.value())?;
    check("charge.input", r.requested_input.value(), input.value())?;
    check("charge.accepted", r.accepted_input.value(), expected_in)?;
    check("charge.rejected", r.rejected_input.value(), expected_out)?;
    check("charge.store_balance", before + r.accepted_input.value(),
        stored_energy_at(c, r.temperature_after)?.value())?;
    check("charge.input_split", r.accepted_input.value() + r.rejected_input.value(), input.value())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StandingLossReport {
    pub temperature_before: Kelvin,
    pub temperature_after: Kelvin,
    pub ambient_temperature: Kelvin,
    pub heat_lost: EnergyJ,
}
pub fn apply_standing_loss(c: &StoreConfig, s: StoreState, ambient: Kelvin, dt: Seconds)
    -> Result<(StoreState, StandingLossReport), ModelError> {
    let before = stored_energy_at(c, s.temperature)?.value();
    // Explicit Euler from interval-start temperature. No ambient heat gain or stratification.
    let d_t = (s.temperature.value() - ambient.value()).max(0.0);
    let loss = (c.standing_loss_conductance.value() * d_t * dt.value()).min(before);
    let after_t = temperature_at_energy(c, EnergyJ::new((before - loss).max(0.0))?)?;
    Ok((StoreState { temperature: after_t }, StandingLossReport {
        temperature_before: s.temperature, temperature_after: after_t,
        ambient_temperature: ambient, heat_lost: EnergyJ::new(loss)?,
    }))
}
pub fn verify_standing_loss_report(c: &StoreConfig, s: StoreState, ambient: Kelvin, dt: Seconds,
    r: StandingLossReport) -> Result<(), ModelError> {
    let before = stored_energy_at(c, s.temperature)?.value();
    let d_t = (s.temperature.value() - ambient.value()).max(0.0);
    let expected_loss = (c.standing_loss_conductance.value() * d_t * dt.value()).min(before);
    let after_t = temperature_at_energy(c, EnergyJ::new((before - expected_loss).max(0.0))?)?;
    check("loss.temp_before", r.temperature_before.value(), s.temperature.value())?;
    check("loss.temp_after", r.temperature_after.value(), after_t.value())?;
    check("loss.ambient", r.ambient_temperature.value(), ambient.value())?;
    check("loss.heat", r.heat_lost.value(), expected_loss)?;
    check("loss.balance", before, stored_energy_at(c, r.temperature_after)?.value() + r.heat_lost.value())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DischargeReport {
    pub temperature_before: Kelvin,
    pub temperature_after: Kelvin,
    pub requested_useful_heat: EnergyJ,
    pub source_heat_withdrawn: EnergyJ,
    pub useful_heat_delivered: EnergyJ,
    pub heat_exchanger_loss: EnergyJ,
    pub unmet_useful_heat: EnergyJ,
    pub minimum_source_temperature: Kelvin,
    pub blocked_by_temperature: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DischargeRequest {
    pub requested_useful_heat: EnergyJ,
    pub required_supply_temperature: Kelvin,
    pub minimum_approach: TemperatureDifferenceK,
    pub heat_exchanger_effectiveness: Fraction,
    pub duration: Seconds,
}

pub fn discharge_to_load(c: &StoreConfig, s: StoreState, request: DischargeRequest)
    -> Result<(StoreState, DischargeReport), ModelError> {
    let requested_useful_heat = request.requested_useful_heat;
    let load_supply = request.required_supply_temperature;
    let approach = request.minimum_approach;
    let effectiveness = request.heat_exchanger_effectiveness;
    let dt = request.duration;
    if effectiveness.value() <= 0.0 { return Err(ModelError::InvalidDischargeEfficiency); }
    let cutoff = load_supply.value() + approach.value();
    let floor = Kelvin::new(cutoff.max(c.min_temperature.value()))?;
    let before = stored_energy_at(c, s.temperature)?.value();
    let blocked = requested_useful_heat.value() > 0.0 && s.temperature.value() <= cutoff;
    if blocked || requested_useful_heat.value() == 0.0 {
        return Ok((s, DischargeReport {
            temperature_before: s.temperature, temperature_after: s.temperature,
            requested_useful_heat, source_heat_withdrawn: EnergyJ::new(0.0)?,
            useful_heat_delivered: EnergyJ::new(0.0)?, heat_exchanger_loss: EnergyJ::new(0.0)?,
            unmet_useful_heat: requested_useful_heat, minimum_source_temperature: floor, blocked_by_temperature: blocked,
        }));
    }
    let floor_energy = stored_energy_at(c, floor)?.value();
    let available = (before - floor_energy).max(0.0);
    let power_limit = c.max_discharge_power.value() * dt.value();
    let source_needed = requested_useful_heat.value() / effectiveness.value();
    let withdrawn = source_needed.min(available).min(power_limit);
    let delivered = (withdrawn * effectiveness.value()).min(requested_useful_heat.value());
    let loss = (withdrawn - delivered).max(0.0);
    let after_t = temperature_at_energy(c, EnergyJ::new((before - withdrawn).max(0.0))?)?;
    Ok((StoreState { temperature: after_t }, DischargeReport {
        temperature_before: s.temperature, temperature_after: after_t, requested_useful_heat,
        source_heat_withdrawn: EnergyJ::new(withdrawn)?, useful_heat_delivered: EnergyJ::new(delivered)?,
        heat_exchanger_loss: EnergyJ::new(loss)?,
        unmet_useful_heat: EnergyJ::new((requested_useful_heat.value() - delivered).max(0.0))?,
        minimum_source_temperature: floor, blocked_by_temperature: false,
    }))
}
pub fn verify_discharge_report(c: &StoreConfig, s: StoreState, request: DischargeRequest, r: DischargeReport)
    -> Result<(), ModelError> {
    let requested_useful_heat = request.requested_useful_heat;
    let load_supply = request.required_supply_temperature;
    let approach = request.minimum_approach;
    let effectiveness = request.heat_exchanger_effectiveness;
    let dt = request.duration;
    if effectiveness.value() <= 0.0 { return Err(ModelError::InvalidDischargeEfficiency); }
    let cutoff = load_supply.value() + approach.value();
    let floor = Kelvin::new(cutoff.max(c.min_temperature.value()))?;
    let before = stored_energy_at(c, s.temperature)?.value();
    let blocked = requested_useful_heat.value() > 0.0 && s.temperature.value() <= cutoff;
    let (withdrawn, delivered, loss, unmet, after_t) = if blocked || requested_useful_heat.value() == 0.0 {
        (0.0, 0.0, 0.0, requested_useful_heat.value(), s.temperature)
    } else {
        let floor_energy = stored_energy_at(c, floor)?.value();
        let available = (before - floor_energy).max(0.0);
        let power_limit = c.max_discharge_power.value() * dt.value();
        let source_needed = requested_useful_heat.value() / effectiveness.value();
        let w = source_needed.min(available).min(power_limit);
        let d = (w * effectiveness.value()).min(requested_useful_heat.value());
        let l = (w - d).max(0.0);
        let u = (requested_useful_heat.value() - d).max(0.0);
        let t = temperature_at_energy(c, EnergyJ::new((before - w).max(0.0))?)?;
        (w, d, l, u, t)
    };
    check("discharge.temp_before", r.temperature_before.value(), s.temperature.value())?;
    check("discharge.temp_after", r.temperature_after.value(), after_t.value())?;
    check("discharge.request", r.requested_useful_heat.value(), requested_useful_heat.value())?;
    check("discharge.withdrawn", r.source_heat_withdrawn.value(), withdrawn)?;
    check("discharge.delivered", r.useful_heat_delivered.value(), delivered)?;
    check("discharge.loss", r.heat_exchanger_loss.value(), loss)?;
    check("discharge.unmet", r.unmet_useful_heat.value(), unmet)?;
    check("discharge.cutoff", r.minimum_source_temperature.value(), floor.value())?;
    if r.blocked_by_temperature != blocked { return Err(ModelError::ReportMismatch("discharge.temperature_block")); }
    check("discharge.store_balance", before,
        stored_energy_at(c, r.temperature_after)?.value() + r.source_heat_withdrawn.value())?;
    check("discharge.exchange_balance", r.source_heat_withdrawn.value(),
        r.useful_heat_delivered.value() + r.heat_exchanger_loss.value())?;
    check("discharge.service_balance", requested_useful_heat.value(),
        r.useful_heat_delivered.value() + r.unmet_useful_heat.value())
}

/// Runs a fixed, synthetic end-to-end fixture and emits deterministic JSON.
///
/// The receipt records source inputs and intermediate boundaries, then marks
/// verification true only after each recomputation path accepts the reports.
/// It is a mechanics smoke artifact, not an engineering performance claim.
pub fn canonical_demo_receipt() -> Result<String, ModelError> {
    let collector_input = CollectorInput {
        incident_solar: EnergyJ::new(1_000_000.0)?,
        optical_efficiency: Fraction::new(0.8)?,
        receiver_loss: EnergyJ::new(100_000.0)?,
    };
    let collector = capture_solar(collector_input)?;
    verify_collector_report(collector_input, collector)?;

    let curve = HeatCapacityCurve::new(vec![
        HeatCapacityPoint::new(
            Kelvin::new(300.0)?,
            SpecificHeatJPerKgK::new(1_000.0)?,
        ),
        HeatCapacityPoint::new(
            Kelvin::new(500.0)?,
            SpecificHeatJPerKgK::new(2_000.0)?,
        ),
    ])?;
    let config = StoreConfig::new(StoreConfigSpec {
        mass: MassKg::new(1.0)?,
        heat_capacity: curve,
        min_temperature: Kelvin::new(300.0)?,
        max_temperature: Kelvin::new(500.0)?,
        initial_temperature: Kelvin::new(300.0)?,
        max_charge_power: PowerW::new(10_000.0)?,
        max_discharge_power: PowerW::new(10_000.0)?,
        standing_loss_conductance: ThermalConductanceWPerK::new(10.0)?,
    })?;

    let initial = config.initial_state();
    let (charged, charge) = charge_store(
        &config,
        initial,
        collector.useful_captured_heat,
        Seconds::new(100.0)?,
    )?;
    verify_charge_report(
        &config,
        initial,
        collector.useful_captured_heat,
        Seconds::new(100.0)?,
        charge,
    )?;

    let (after_loss, loss) = apply_standing_loss(
        &config,
        charged,
        Kelvin::new(300.0)?,
        Seconds::new(10.0)?,
    )?;
    verify_standing_loss_report(
        &config,
        charged,
        Kelvin::new(300.0)?,
        Seconds::new(10.0)?,
        loss,
    )?;

    let demand = EnergyJ::new(10_000.0)?;
    let supply_temp = Kelvin::new(450.0)?;
    let approach = TemperatureDifferenceK::new(10.0)?;
    let effectiveness = Fraction::new(0.9)?;
    let duration = Seconds::new(100.0)?;
    let discharge_request = DischargeRequest {
        requested_useful_heat: demand,
        required_supply_temperature: supply_temp,
        minimum_approach: approach,
        heat_exchanger_effectiveness: effectiveness,
        duration,
    };
    let (final_state, discharge) =
        discharge_to_load(&config, after_loss, discharge_request)?;
    verify_discharge_report(&config, after_loss, discharge_request, discharge)?;

    // Canonical analytic two-store cascade: cp = 1000 J/(kg K) in each store.
    let cascade_high_curve = HeatCapacityCurve::new(vec![
        HeatCapacityPoint::new(Kelvin::new(300.0)?, SpecificHeatJPerKgK::new(1_000.0)?),
        HeatCapacityPoint::new(Kelvin::new(500.0)?, SpecificHeatJPerKgK::new(1_000.0)?),
    ])?;
    let cascade_high_config = StoreConfig::new(StoreConfigSpec {
        mass: MassKg::new(1.0)?,
        heat_capacity: cascade_high_curve,
        min_temperature: Kelvin::new(300.0)?,
        max_temperature: Kelvin::new(500.0)?,
        initial_temperature: Kelvin::new(450.0)?,
        max_charge_power: PowerW::new(10_000.0)?,
        max_discharge_power: PowerW::new(10_000.0)?,
        standing_loss_conductance: ThermalConductanceWPerK::new(0.0)?,
    })?;
    let cascade_low_curve = HeatCapacityCurve::new(vec![
        HeatCapacityPoint::new(Kelvin::new(300.0)?, SpecificHeatJPerKgK::new(1_000.0)?),
        HeatCapacityPoint::new(Kelvin::new(360.0)?, SpecificHeatJPerKgK::new(1_000.0)?),
    ])?;
    let cascade_low_config = StoreConfig::new(StoreConfigSpec {
        mass: MassKg::new(1.0)?,
        heat_capacity: cascade_low_curve,
        min_temperature: Kelvin::new(300.0)?,
        max_temperature: Kelvin::new(360.0)?,
        initial_temperature: Kelvin::new(300.0)?,
        max_charge_power: PowerW::new(10_000.0)?,
        max_discharge_power: PowerW::new(10_000.0)?,
        standing_loss_conductance: ThermalConductanceWPerK::new(0.0)?,
    })?;
    let cascade_high_start = cascade_high_config.initial_state();
    let cascade_low_start = cascade_low_config.initial_state();
    let cascade_request = cascade::CascadeTransferRequest {
        requested_low_store_heat: EnergyJ::new(40_000.0)?,
        minimum_approach: TemperatureDifferenceK::new(10.0)?,
        heat_exchanger_effectiveness: Fraction::new(0.8)?,
        duration: Seconds::new(10.0)?,
    };
    let (cascade_high_end, cascade_low_end, cascade_report) =
        cascade::transfer_to_lower_temperature_store(
            &cascade_high_config,
            cascade_high_start,
            &cascade_low_config,
            cascade_low_start,
            cascade_request,
        )?;
    cascade::verify_cascade_transfer_report(
        &cascade_high_config,
        cascade_high_start,
        &cascade_low_config,
        cascade_low_start,
        cascade_request,
        cascade_report,
    )?;

    // Canonical static pinch target fixture: one hot and one cold sensible stream.
    let pinch_problem = pinch::PinchProblem::new(
        vec![pinch::HotProcessStream::new(
            Kelvin::new(423.15)?,
            Kelvin::new(323.15)?,
            EnergyJ::new(100_000.0)?,
        )?],
        vec![pinch::ColdProcessStream::new(
            Kelvin::new(393.15)?,
            Kelvin::new(493.15)?,
            EnergyJ::new(120_000.0)?,
        )?],
        TemperatureDifferenceK::new(20.0)?,
    )?;
    let pinch_targets = pinch::target_pinch_utilities(&pinch_problem)?;
    pinch::verify_pinch_targets(&pinch_problem, &pinch_targets)?;

    // Canonical two-period day/night fixture for independent schedule auditing.
    let temporal_curve = HeatCapacityCurve::new(vec![
        HeatCapacityPoint::new(Kelvin::new(300.0)?, SpecificHeatJPerKgK::new(1_000.0)?),
        HeatCapacityPoint::new(Kelvin::new(400.0)?, SpecificHeatJPerKgK::new(1_000.0)?),
    ])?;
    let temporal_config = StoreConfig::new(StoreConfigSpec {
        mass: MassKg::new(10.0)?,
        heat_capacity: temporal_curve,
        min_temperature: Kelvin::new(300.0)?,
        max_temperature: Kelvin::new(400.0)?,
        initial_temperature: Kelvin::new(300.0)?,
        max_charge_power: PowerW::new(5_000.0)?,
        max_discharge_power: PowerW::new(1_000.0)?,
        standing_loss_conductance: ThermalConductanceWPerK::new(0.1)?,
    })?;
    let temporal_duration = Seconds::new(3_600.0)?;
    let temporal_schedule = vec![
        temporal::TemporalPeriod {
            duration: temporal_duration,
            ambient_temperature: Kelvin::new(300.0)?,
            action: temporal::PeriodAction::Charge {
                available_heat: EnergyJ::new(900_000.0)?,
            },
        },
        temporal::TemporalPeriod {
            duration: temporal_duration,
            ambient_temperature: Kelvin::new(300.0)?,
            action: temporal::PeriodAction::Discharge {
                request: DischargeRequest {
                    requested_useful_heat: EnergyJ::new(300_000.0)?,
                    required_supply_temperature: Kelvin::new(340.0)?,
                    minimum_approach: TemperatureDifferenceK::new(10.0)?,
                    heat_exchanger_effectiveness: Fraction::new(0.9)?,
                    duration: temporal_duration,
                },
            },
        },
    ];
    let temporal_report = temporal::simulate_schedule(&temporal_config, &temporal_schedule)?;
    temporal::verify_schedule_report(&temporal_config, &temporal_schedule, &temporal_report)?;

    // Separate direct-beam input fixture. It is audited as incident energy,
    // not conflated with the independent collector-performance fixture below.
    let dni_input = solar_input::DirectBeamIntervalInput {
        direct_normal_irradiance: solar_input::IrradianceWPerM2::new(800.0)?,
        aperture_area: solar_input::ApertureAreaM2::new(10.0)?,
        incidence_cosine: Fraction::new(0.5)?,
        unshaded_fraction: Fraction::new(0.8)?,
        duration: Seconds::new(3_600.0)?,
    };
    let dni_report = solar_input::integrate_direct_beam(dni_input)?;
    solar_input::verify_direct_beam_interval(dni_input, dni_report)?;

    // Synthetic interval-BNI conversion fixture: 100 Wh/m² over 15 minutes.
    let bni_irradiation = solar_input::DirectNormalIrradiationWhPerM2::new(100.0)?;
    let bni_duration = Seconds::new(900.0)?;
    let bni_report = solar_input::mean_dni_from_interval_irradiation(
        bni_irradiation, bni_duration,
    )?;
    solar_input::verify_mean_dni_report(bni_irradiation, bni_duration, bni_report)?;

    // Canonical hand-calculable service-risk fixture. Scenario outcomes are
    // caller-supplied; this receipt tests metric arithmetic, not forecasting.
    let risk_outcomes = [
        scenario_risk::ScenarioServiceOutcome {
            scenario_id: 10,
            probability: scenario_risk::ScenarioProbability::new(0.80)
                .map_err(|_| ModelError::ReportMismatch("risk.probability_0"))?,
            requested_useful_heat: EnergyJ::new(100.0)?,
            delivered_useful_heat: EnergyJ::new(100.0)?,
        },
        scenario_risk::ScenarioServiceOutcome {
            scenario_id: 20,
            probability: scenario_risk::ScenarioProbability::new(0.17)
                .map_err(|_| ModelError::ReportMismatch("risk.probability_1"))?,
            requested_useful_heat: EnergyJ::new(100.0)?,
            delivered_useful_heat: EnergyJ::new(80.0)?,
        },
        scenario_risk::ScenarioServiceOutcome {
            scenario_id: 30,
            probability: scenario_risk::ScenarioProbability::new(0.03)
                .map_err(|_| ModelError::ReportMismatch("risk.probability_2"))?,
            requested_useful_heat: EnergyJ::new(100.0)?,
            delivered_useful_heat: EnergyJ::new(20.0)?,
        },
    ];
    let risk_service_floor = Fraction::new(0.80)?;
    let risk_report = scenario_risk::summarize_scenario_service_risk(
        &risk_outcomes, risk_service_floor,
    ).map_err(|_| ModelError::ReportMismatch("risk.summary"))?;
    scenario_risk::verify_scenario_service_risk_report(
        &risk_outcomes, risk_service_floor, &risk_report,
    ).map_err(|_| ModelError::ReportMismatch("risk.verification"))?;

    let mut json = String::from(
        "{\"schema\":\"solar-thermal-kernel-report-v0.1\",\"fixture_id\":\"analytic-sensible-store-001\",\"claim_level\":\"synthetic-mechanics-only\",\"source_binding\":\"not-source-hash-bound\",\"assumptions_id\":\"analytic-cp-linear-two-point-v0.1\""
    );
    fn number(out: &mut String, key: &str, value: f64) {
        write!(out, ",\"{key}\":{value:.9}").expect("writing to a String cannot fail");
    }
    json.push_str(",\"bni_fixture_id\":\"interval-bni-to-mean-dni-001\"");
    number(&mut json, "bni_interval_irradiation_wh_per_m2", 100.0);
    number(&mut json, "bni_interval_duration_s", 900.0);
    number(&mut json, "bni_interval_mean_dni_w_per_m2", bni_report.interval_mean_irradiance.value());

    json.push_str(",\"risk_fixture_id\":\"scenario-service-risk-001\"");
    number(&mut json, "risk_service_floor_fraction", risk_service_floor.value());
    number(&mut json, "risk_scenario_count", risk_outcomes.len() as f64);
    for (index, outcome) in risk_outcomes.iter().enumerate() {
        number(&mut json, &format!("risk_scenario_{index}_id"), outcome.scenario_id as f64);
        number(&mut json, &format!("risk_scenario_{index}_probability"), outcome.probability.value());
        number(&mut json, &format!("risk_scenario_{index}_requested_heat_j"), outcome.requested_useful_heat.value());
        number(&mut json, &format!("risk_scenario_{index}_delivered_heat_j"), outcome.delivered_useful_heat.value());
    }
    number(&mut json, "risk_expected_unmet_useful_heat_j", risk_report.expected_unmet_useful_heat.value());
    number(&mut json, "risk_worst_unmet_useful_heat_j", risk_report.worst_unmet_useful_heat.value());
    number(&mut json, "risk_expected_shortfall_fraction", risk_report.expected_shortfall_fraction);
    number(&mut json, "risk_worst_shortfall_fraction", risk_report.worst_shortfall_fraction);
    number(&mut json, "risk_p95_shortfall_fraction", risk_report.p95_shortfall_fraction);
    number(&mut json, "risk_cvar95_shortfall_fraction", risk_report.cvar95_shortfall_fraction);
    number(&mut json, "risk_probability_below_service_floor", risk_report.probability_below_service_floor);
    number(&mut json, "risk_worst_case_scenario_id", risk_report.worst_case_scenario_id as f64);

    json.push_str(",\"dni_fixture_id\":\"direct-beam-interval-001\"");
    number(&mut json, "dni_direct_normal_irradiance_w_per_m2", 800.0);
    number(&mut json, "dni_aperture_area_m2", 10.0);
    number(&mut json, "dni_incidence_cosine", 0.5);
    number(&mut json, "dni_unshaded_fraction", 0.8);
    number(&mut json, "dni_duration_s", 3_600.0);
    number(&mut json, "dni_incident_power_w", dni_report.incident_power_at_aperture.value());
    number(&mut json, "dni_incident_energy_j", dni_report.incident_energy.value());

    number(&mut json, "exergy_reference_temperature_k", 300.0);
    number(&mut json, "optical_efficiency", 0.8);
    number(&mut json, "store_mass_kg", 1.0);
    number(&mut json, "store_min_temperature_k", 300.0);
    number(&mut json, "store_max_temperature_k", 500.0);
    number(&mut json, "store_initial_temperature_k", 300.0);
    number(&mut json, "cp_curve_low_temperature_k", 300.0);
    number(&mut json, "cp_curve_low_j_per_kg_k", 1_000.0);
    number(&mut json, "cp_curve_high_temperature_k", 500.0);
    number(&mut json, "cp_curve_high_j_per_kg_k", 2_000.0);
    number(&mut json, "maximum_charge_power_w", 10_000.0);
    number(&mut json, "store_charge_effectiveness", 1.0);
    number(&mut json, "maximum_discharge_power_w", 10_000.0);
    number(&mut json, "standing_loss_conductance_w_per_k", 10.0);
    number(&mut json, "standing_loss_ambient_temperature_k", 300.0);
    number(&mut json, "charge_duration_s", 100.0);
    number(&mut json, "standing_loss_duration_s", 10.0);
    number(&mut json, "load_supply_temperature_k", 450.0);
    number(&mut json, "minimum_heat_exchanger_approach_k", 10.0);
    number(&mut json, "heat_exchanger_effectiveness", 0.9);
    number(&mut json, "discharge_duration_s", 100.0);
    number(&mut json, "incident_solar_j", collector.incident_solar.value());
    number(&mut json, "optical_loss_j", collector.optical_loss.value());
    number(&mut json, "absorbed_solar_j", collector.absorbed_solar.value());
    number(&mut json, "receiver_loss_j", collector.receiver_loss.value());
    number(&mut json, "useful_captured_heat_j", collector.useful_captured_heat.value());
    number(&mut json, "store_energy_accepted_j", charge.accepted_input.value());
    number(&mut json, "store_energy_rejected_j", charge.rejected_input.value());
    number(&mut json, "store_temperature_after_charge_k", charge.temperature_after.value());
    number(&mut json, "standing_loss_j", loss.heat_lost.value());
    number(&mut json, "store_temperature_after_loss_k", loss.temperature_after.value());
    number(&mut json, "stored_exergy_before_discharge_j", config.stored_exergy(after_loss, Kelvin::new(300.0)?)?.value());
    number(&mut json, "requested_useful_heat_j", discharge.requested_useful_heat.value());
    number(&mut json, "source_heat_withdrawn_j", discharge.source_heat_withdrawn.value());
    number(&mut json, "useful_heat_delivered_j", discharge.useful_heat_delivered.value());
    number(&mut json, "heat_exchanger_loss_j", discharge.heat_exchanger_loss.value());
    number(&mut json, "unmet_useful_heat_j", discharge.unmet_useful_heat.value());
    number(&mut json, "final_store_temperature_k", final_state.temperature.value());
    number(&mut json, "final_store_exergy_j", config.stored_exergy(final_state, Kelvin::new(300.0)?)?.value());

    json.push_str(",\"cascade_fixture_id\":\"two-temperature-sensible-001\"");
    number(&mut json, "cascade_high_store_mass_kg", 1.0);
    number(&mut json, "cascade_high_store_min_temperature_k", 300.0);
    number(&mut json, "cascade_high_store_max_temperature_k", 500.0);
    number(&mut json, "cascade_high_store_initial_temperature_k", 450.0);
    number(&mut json, "cascade_high_cp_low_temperature_k", 300.0);
    number(&mut json, "cascade_high_cp_low_j_per_kg_k", 1_000.0);
    number(&mut json, "cascade_high_cp_high_temperature_k", 500.0);
    number(&mut json, "cascade_high_cp_high_j_per_kg_k", 1_000.0);
    number(&mut json, "cascade_high_max_discharge_power_w", 10_000.0);
    number(&mut json, "cascade_low_store_mass_kg", 1.0);
    number(&mut json, "cascade_low_store_min_temperature_k", 300.0);
    number(&mut json, "cascade_low_store_max_temperature_k", 360.0);
    number(&mut json, "cascade_low_store_initial_temperature_k", 300.0);
    number(&mut json, "cascade_low_cp_low_temperature_k", 300.0);
    number(&mut json, "cascade_low_cp_low_j_per_kg_k", 1_000.0);
    number(&mut json, "cascade_low_cp_high_temperature_k", 360.0);
    number(&mut json, "cascade_low_cp_high_j_per_kg_k", 1_000.0);
    number(&mut json, "cascade_requested_low_store_heat_j", cascade_report.requested_low_store_heat.value());
    number(&mut json, "cascade_minimum_approach_k", 10.0);
    number(&mut json, "cascade_heat_exchanger_effectiveness", 0.8);
    number(&mut json, "cascade_duration_s", 10.0);
    number(&mut json, "cascade_source_heat_withdrawn_j", cascade_report.source_heat_withdrawn.value());
    number(&mut json, "cascade_low_store_heat_accepted_j", cascade_report.low_store_heat_accepted.value());
    number(&mut json, "cascade_heat_exchanger_loss_j", cascade_report.heat_exchanger_loss.value());
    number(&mut json, "cascade_unmet_low_store_heat_j", cascade_report.unmet_low_store_heat.value());
    number(&mut json, "cascade_minimum_source_temperature_k", cascade_report.minimum_source_temperature.value());
    number(&mut json, "cascade_high_store_final_temperature_k", cascade_high_end.temperature.value());
    number(&mut json, "cascade_low_store_final_temperature_k", cascade_low_end.temperature.value());
    json.push_str(",\"cascade_blocked_by_temperature\":false");

    json.push_str(",\"pinch_fixture_id\":\"static-sensible-pinch-001\"");
    number(&mut json, "pinch_hot_supply_temperature_k", 423.15);
    number(&mut json, "pinch_hot_target_temperature_k", 323.15);
    number(&mut json, "pinch_hot_available_heat_j", 100_000.0);
    number(&mut json, "pinch_cold_supply_temperature_k", 393.15);
    number(&mut json, "pinch_cold_target_temperature_k", 493.15);
    number(&mut json, "pinch_cold_required_heat_j", 120_000.0);
    number(&mut json, "pinch_minimum_approach_k", 20.0);
    number(&mut json, "pinch_minimum_hot_utility_j", pinch_targets.minimum_hot_utility.value());
    number(&mut json, "pinch_minimum_cold_utility_j", pinch_targets.minimum_cold_utility.value());
    number(&mut json, "pinch_maximum_process_heat_recovery_j", pinch_targets.maximum_process_heat_recovery.value());
    number(&mut json, "pinch_interval_count", pinch_targets.intervals.len() as f64);
    number(&mut json, "pinch_point_count", pinch_targets.pinch_points.len() as f64);
    number(&mut json, "pinch_shifted_temperature_k", pinch_targets.pinch_points.first().map_or(0.0, |p| p.shifted_temperature.value()));
    number(&mut json, "pinch_hot_side_temperature_k", pinch_targets.pinch_points.first().map_or(0.0, |p| p.hot_side_temperature.value()));
    number(&mut json, "pinch_cold_side_temperature_k", pinch_targets.pinch_points.first().map_or(0.0, |p| p.cold_side_temperature.value()));
    json.push_str(",\"temporal_fixture_id\":\"analytic-day-night-storage-001\"");
    number(&mut json, "temporal_store_mass_kg", 10.0);
    number(&mut json, "temporal_store_min_temperature_k", 300.0);
    number(&mut json, "temporal_store_max_temperature_k", 400.0);
    number(&mut json, "temporal_store_initial_temperature_k", 300.0);
    number(&mut json, "temporal_cp_j_per_kg_k", 1_000.0);
    number(&mut json, "temporal_max_charge_power_w", 5_000.0);
    number(&mut json, "temporal_max_discharge_power_w", 1_000.0);
    number(&mut json, "temporal_standing_loss_conductance_w_per_k", 0.1);
    number(&mut json, "temporal_period_duration_s", 3_600.0);
    number(&mut json, "temporal_ambient_temperature_k", 300.0);
    number(&mut json, "temporal_charge_available_heat_j", 900_000.0);
    number(&mut json, "temporal_load_supply_temperature_k", 340.0);
    number(&mut json, "temporal_minimum_approach_k", 10.0);
    number(&mut json, "temporal_heat_exchanger_effectiveness", 0.9);
    number(&mut json, "temporal_requested_useful_heat_j", 300_000.0);

    number(&mut json, "temporal_initial_stored_energy_j", temporal_report.initial_stored_energy.value());
    number(&mut json, "temporal_final_stored_energy_j", temporal_report.final_stored_energy.value());
    number(&mut json, "temporal_accepted_charge_heat_j", temporal_report.accepted_charge_heat.value());
    number(&mut json, "temporal_rejected_charge_heat_j", temporal_report.rejected_charge_heat.value());
    number(&mut json, "temporal_source_heat_withdrawn_j", temporal_report.source_heat_withdrawn.value());
    number(&mut json, "temporal_useful_heat_delivered_j", temporal_report.useful_heat_delivered.value());
    number(&mut json, "temporal_heat_exchanger_loss_j", temporal_report.heat_exchanger_loss.value());
    number(&mut json, "temporal_unmet_useful_heat_j", temporal_report.unmet_useful_heat.value());
    number(&mut json, "temporal_standing_heat_loss_j", temporal_report.standing_heat_loss.value());
    number(&mut json, "temporal_energy_balance_residual_j", temporal_report.energy_balance_residual_j);

    let temporal_charge = match temporal_report.periods[0].action {
        temporal::PeriodActionReport::Charge(report) => report,
        temporal::PeriodActionReport::Discharge(_) | temporal::PeriodActionReport::Idle => return Err(ModelError::ReportMismatch("temporal.fixture_action_0")),
    };
    number(&mut json, "temporal_period_0_temperature_before_k", temporal_report.periods[0].state_before.temperature.value());
    number(&mut json, "temporal_period_0_temperature_after_loss_k", temporal_report.periods[0].state_after_standing_loss.temperature.value());
    number(&mut json, "temporal_period_0_temperature_after_action_k", temporal_report.periods[0].state_after.temperature.value());
    number(&mut json, "temporal_period_0_standing_loss_j", temporal_report.periods[0].standing_loss.heat_lost.value());
    number(&mut json, "temporal_period_0_charge_requested_j", temporal_charge.requested_input.value());
    number(&mut json, "temporal_period_0_charge_accepted_j", temporal_charge.accepted_input.value());
    number(&mut json, "temporal_period_0_charge_rejected_j", temporal_charge.rejected_input.value());

    let temporal_discharge = match temporal_report.periods[1].action {
        temporal::PeriodActionReport::Discharge(report) => report,
        temporal::PeriodActionReport::Charge(_) | temporal::PeriodActionReport::Idle => return Err(ModelError::ReportMismatch("temporal.fixture_action_1")),
    };
    number(&mut json, "temporal_period_1_temperature_before_k", temporal_report.periods[1].state_before.temperature.value());
    number(&mut json, "temporal_period_1_temperature_after_loss_k", temporal_report.periods[1].state_after_standing_loss.temperature.value());
    number(&mut json, "temporal_period_1_temperature_after_action_k", temporal_report.periods[1].state_after.temperature.value());
    number(&mut json, "temporal_period_1_standing_loss_j", temporal_report.periods[1].standing_loss.heat_lost.value());
    number(&mut json, "temporal_period_1_useful_heat_requested_j", temporal_discharge.requested_useful_heat.value());
    number(&mut json, "temporal_period_1_source_heat_withdrawn_j", temporal_discharge.source_heat_withdrawn.value());
    number(&mut json, "temporal_period_1_useful_heat_delivered_j", temporal_discharge.useful_heat_delivered.value());
    number(&mut json, "temporal_period_1_heat_exchanger_loss_j", temporal_discharge.heat_exchanger_loss.value());
    number(&mut json, "temporal_period_1_unmet_useful_heat_j", temporal_discharge.unmet_useful_heat.value());
    json.push_str(",\"temporal_period_1_blocked_by_temperature\":false");
    json.push_str(",\"all_verifiers_passed\":true,\"hardware_validated\":false}");
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(x: f64) -> EnergyJ { EnergyJ::new(x).unwrap() }
    fn k(x: f64) -> Kelvin { Kelvin::new(x).unwrap() }
    fn dt(x: f64) -> Seconds { Seconds::new(x).unwrap() }
    fn curve() -> HeatCapacityCurve {
        HeatCapacityCurve::new(vec![
            HeatCapacityPoint::new(k(300.0), SpecificHeatJPerKgK::new(1_000.0).unwrap()),
            HeatCapacityPoint::new(k(500.0), SpecificHeatJPerKgK::new(2_000.0).unwrap()),
        ]).unwrap()
    }
    fn cfg(start: f64) -> StoreConfig {
        StoreConfig::new(StoreConfigSpec {
            mass: MassKg::new(1.0).unwrap(), heat_capacity: curve(),
            min_temperature: k(300.0), max_temperature: k(500.0), initial_temperature: k(start),
            max_charge_power: PowerW::new(10_000.0).unwrap(),
            max_discharge_power: PowerW::new(10_000.0).unwrap(),
            standing_loss_conductance: ThermalConductanceWPerK::new(10.0).unwrap(),
        }).unwrap()
    }
    fn spec(c: &StoreConfig, start: f64) -> StoreConfigSpec {
        StoreConfigSpec {
            mass: c.mass, heat_capacity: c.heat_capacity.clone(),
            min_temperature: c.min_temperature, max_temperature: c.max_temperature,
            initial_temperature: k(start), max_charge_power: c.max_charge_power,
            max_discharge_power: c.max_discharge_power,
            standing_loss_conductance: c.standing_loss_conductance,
        }
    }

    #[test]
    fn receipt_is_deterministic_and_explicitly_synthetic() {
        let first = canonical_demo_receipt().unwrap();
        let second = canonical_demo_receipt().unwrap();
        assert_eq!(first, second);
        assert!(first.starts_with("{\"schema\":\"solar-thermal-kernel-report-v0.1\""));
        assert!(first.contains("\"all_verifiers_passed\":true"));
        assert!(first.contains("\"hardware_validated\":false"));
        assert!(first.contains("\"claim_level\":\"synthetic-mechanics-only\""));
        assert!(first.contains("\"source_binding\":\"not-source-hash-bound\""));
        assert!(first.contains("\"assumptions_id\":\"analytic-cp-linear-two-point-v0.1\""));
        assert!(first.contains("\"stored_exergy_before_discharge_j\""));
        assert!(first.contains("\"final_store_exergy_j\""));
        assert!(first.contains("\"exergy_reference_temperature_k\":300.000000000"));
        assert!(first.contains("\"optical_efficiency\":0.800000000"));
        assert!(first.contains("\"risk_fixture_id\":\"scenario-service-risk-001\""));
        assert!(first.contains("\"risk_expected_unmet_useful_heat_j\":5.800000000"));
        assert!(first.contains("\"risk_p95_shortfall_fraction\":0.200000000"));
        assert!(first.contains("\"risk_cvar95_shortfall_fraction\":0.560000000"));
        assert!(first.contains("\"risk_probability_below_service_floor\":0.030000000"));
        assert!(first.contains("\"dni_fixture_id\":\"direct-beam-interval-001\""));
        assert!(first.contains("\"dni_incident_power_w\":3200.000000000"));
        assert!(first.contains("\"dni_incident_energy_j\":11520000.000000000"));
        assert!(first.contains("\"bni_fixture_id\":\"interval-bni-to-mean-dni-001\""));
        assert!(first.contains("\"bni_interval_mean_dni_w_per_m2\":400.000000000"));
        assert!(first.contains("\"store_initial_temperature_k\":300.000000000"));
        assert!(first.contains("\"store_charge_effectiveness\":1.000000000"));
        assert!(first.contains("\"standing_loss_ambient_temperature_k\":300.000000000"));
        assert!(first.contains("\"useful_captured_heat_j\":700000.000000000"));
        assert!(first.contains("\"unmet_useful_heat_j\":0.000000000"));
        assert!(first.contains("\"pinch_fixture_id\":\"static-sensible-pinch-001\""));
        assert!(first.contains("\"pinch_minimum_hot_utility_j\":110000.000000000"));
        assert!(first.contains("\"pinch_minimum_cold_utility_j\":90000.000000000"));
        assert!(first.contains("\"pinch_maximum_process_heat_recovery_j\":10000.000000000"));
        assert!(first.contains("\"pinch_point_count\":1.000000000"));
        assert!(first.contains("\"temporal_fixture_id\":\"analytic-day-night-storage-001\""));
        assert!(first.contains("\"temporal_accepted_charge_heat_j\":900000.000000000"));
        assert!(first.contains("\"temporal_standing_heat_loss_j\":32400.000000000"));
        assert!(first.contains("\"temporal_useful_heat_delivered_j\":300000.000000000"));
        assert!(first.contains("\"temporal_unmet_useful_heat_j\":0.000000000"));
    }

    #[test]
    fn exergy_is_less_than_energy_and_matches_variable_cp_integral() {
        let c = curve();
        // cp(T) = 5*T - 500 J/(kg K), from 300 K to 400 K.
        // Integral cp dT = 125,000 J/kg.
        // Integral cp/T dT = 500 - 500*ln(4/3).
        // B = 125,000 - 300*(500 - 500*ln(4/3)).
        let exergy = c.specific_exergy(k(300.0), k(400.0)).unwrap().value();
        assert!(close(exergy, 18_152.3108684));
        assert!(exergy > 0.0);
        assert!(exergy < c.specific_enthalpy(k(300.0), k(400.0)).unwrap());
    }

    #[test]
    fn exergy_is_zero_at_reference_and_monotone_above_it() {
        let c = curve();
        assert_eq!(c.specific_exergy(k(300.0), k(300.0)).unwrap().value(), 0.0);
        let mut last = 0.0;
        for temp in [320.0, 340.0, 360.0, 380.0, 400.0, 440.0, 480.0, 500.0] {
            let now = c.specific_exergy(k(300.0), k(temp)).unwrap().value();
            assert!(now >= last, "exergy must be monotonic for T >= T_ref");
            last = now;
        }
    }

    #[test]
    fn exergy_rejects_final_temperature_below_reference() {
        assert_eq!(
            curve().specific_exergy(k(350.0), k(340.0)),
            Err(ModelError::TemperatureBelowReference)
        );
    }

    #[test]
    fn stored_exergy_scales_with_store_mass() {
        let one_kg = cfg(400.0);
        let one = one_kg.stored_exergy(one_kg.initial_state(), k(300.0)).unwrap().value();
        let two_kg = StoreConfig::new(StoreConfigSpec {
            mass: MassKg::new(2.0).unwrap(),
            ..spec(&one_kg, 400.0)
        }).unwrap();
        let two = two_kg.stored_exergy(two_kg.initial_state(), k(300.0)).unwrap().value();
        assert!(close(two, 2.0 * one));
    }

    #[test]
    fn collector_energy_balance_fixture() {
        let i = CollectorInput { incident_solar: e(1_000.0), optical_efficiency: Fraction::new(0.8).unwrap(), receiver_loss: e(100.0) };
        let r = capture_solar(i).unwrap();
        assert!(close(r.optical_loss.value(), 200.0));
        assert!(close(r.absorbed_solar.value(), 800.0));
        assert!(close(r.useful_captured_heat.value(), 700.0));
        verify_collector_report(i, r).unwrap();
    }
    #[test]
    fn rejects_receiver_loss_larger_than_absorbed_input() {
        let i = CollectorInput { incident_solar: e(1_000.0), optical_efficiency: Fraction::new(0.5).unwrap(), receiver_loss: e(501.0) };
        assert_eq!(capture_solar(i), Err(ModelError::ReceiverLossExceedsAbsorbedEnergy));
    }
    #[test]
    fn integrates_variable_heat_capacity() {
        let c = curve();
        assert!(close(c.specific_enthalpy(k(300.0), k(400.0)).unwrap(), 125_000.0));
        assert!(close(c.specific_enthalpy(k(400.0), k(300.0)).unwrap(), -125_000.0));
    }
    #[test]
    fn refuses_heat_capacity_extrapolation() {
        assert_eq!(curve().specific_enthalpy(k(250.0), k(400.0)), Err(ModelError::TemperatureOutsideHeatCapacityCurve));
    }
    #[test]
    fn charge_raises_store_to_hand_computable_temperature() {
        let c = cfg(300.0); let s = c.initial_state();
        let (after, r) = charge_store(&c, s, e(125_000.0), dt(100.0)).unwrap();
        assert!(close(after.temperature.value(), 400.0));
        assert!(close(r.accepted_input.value(), 125_000.0));
        assert!(close(r.rejected_input.value(), 0.0));
        verify_charge_report(&c, s, e(125_000.0), dt(100.0), r).unwrap();
    }
    #[test]
    fn charge_obeys_power_limit() {
        let c = StoreConfig::new(StoreConfigSpec { max_charge_power: PowerW::new(1_000.0).unwrap(), ..spec(&cfg(300.0), 300.0) }).unwrap();
        let s = c.initial_state();
        let (_, r) = charge_store(&c, s, e(20_000.0), dt(10.0)).unwrap();
        assert!(close(r.accepted_input.value(), 10_000.0));
        assert!(close(r.rejected_input.value(), 10_000.0));
        verify_charge_report(&c, s, e(20_000.0), dt(10.0), r).unwrap();
    }
    #[test]
    fn standing_loss_conserves_stored_energy() {
        let c = cfg(400.0); let s = c.initial_state();
        let (after, r) = apply_standing_loss(&c, s, k(300.0), dt(20.0)).unwrap();
        assert!(close(r.heat_lost.value(), 20_000.0));
        assert!(after.temperature.value() < s.temperature.value());
        verify_standing_loss_report(&c, s, k(300.0), dt(20.0), r).unwrap();
    }
    #[test]
    fn discharge_satisfies_temperature_cutoff_and_closes_exchange_balance() {
        let c = cfg(400.0); let s = c.initial_state(); let eff = Fraction::new(0.9).unwrap();
        let approach = TemperatureDifferenceK::new(10.0).unwrap();
        let (after, r) = discharge_to_load(&c, s, DischargeRequest { requested_useful_heat: e(50_000.0), required_supply_temperature: k(350.0), minimum_approach: approach, heat_exchanger_effectiveness: eff, duration: dt(100.0) }).unwrap();
        assert!(close(r.useful_heat_delivered.value(), 50_000.0));
        assert!(r.temperature_after.value() >= 360.0);
        assert!(after.temperature.value() < s.temperature.value());
        verify_discharge_report(&c, s, DischargeRequest { requested_useful_heat: e(50_000.0), required_supply_temperature: k(350.0), minimum_approach: approach, heat_exchanger_effectiveness: eff, duration: dt(100.0) }, r).unwrap();
    }
    #[test]
    fn blocks_load_when_source_is_too_cold() {
        let c = cfg(365.0); let s = c.initial_state(); let approach = TemperatureDifferenceK::new(10.0).unwrap();
        let (after, r) = discharge_to_load(&c, s, DischargeRequest { requested_useful_heat: e(10_000.0), required_supply_temperature: k(355.0), minimum_approach: approach, heat_exchanger_effectiveness: Fraction::new(0.95).unwrap(), duration: dt(10.0) }).unwrap();
        assert!(r.blocked_by_temperature);
        assert!(close(r.useful_heat_delivered.value(), 0.0));
        assert!(close(r.unmet_useful_heat.value(), 10_000.0));
        assert_eq!(after, s);
        verify_discharge_report(&c, s, DischargeRequest { requested_useful_heat: e(10_000.0), required_supply_temperature: k(355.0), minimum_approach: approach, heat_exchanger_effectiveness: Fraction::new(0.95).unwrap(), duration: dt(10.0) }, r).unwrap();
    }
    #[test]
    fn blocks_delivery_when_required_temperature_exceeds_store_maximum() {
        let c = cfg(400.0);
        let s = c.initial_state();
        let request = DischargeRequest {
            requested_useful_heat: e(10_000.0),
            required_supply_temperature: k(600.0),
            minimum_approach: TemperatureDifferenceK::new(10.0).unwrap(),
            heat_exchanger_effectiveness: Fraction::new(0.9).unwrap(),
            duration: dt(100.0),
        };
        let (after, report) = discharge_to_load(&c, s, request).unwrap();
        assert!(report.blocked_by_temperature);
        assert_eq!(after, s);
        assert!(close(report.useful_heat_delivered.value(), 0.0));
        assert!(close(report.unmet_useful_heat.value(), 10_000.0));
        verify_discharge_report(&c, s, request, report).unwrap();
    }

    #[test]
    fn verifier_rejects_tampered_collector_report() {
        let i = CollectorInput { incident_solar: e(1_000.0), optical_efficiency: Fraction::new(0.8).unwrap(), receiver_loss: e(100.0) };
        let r = capture_solar(i).unwrap();
        let bad = CollectorReport { useful_captured_heat: e(800.0), ..r };
        assert_eq!(verify_collector_report(i, bad), Err(ModelError::ReportMismatch("collector.useful_heat")));
    }
    #[test]
    fn verifier_rejects_tampered_charge_report() {
        let c = cfg(300.0); let s = c.initial_state();
        let (_, r) = charge_store(&c, s, e(125_000.0), dt(100.0)).unwrap();
        let bad = ChargeReport { accepted_input: e(120_000.0), ..r };
        assert!(matches!(verify_charge_report(&c, s, e(125_000.0), dt(100.0), bad), Err(ModelError::ReportMismatch(_))));
    }
    #[test]
    fn verifier_rejects_tampered_discharge_report() {
        let c = cfg(400.0); let s = c.initial_state(); let approach = TemperatureDifferenceK::new(10.0).unwrap(); let eff = Fraction::new(0.9).unwrap();
        let (_, r) = discharge_to_load(&c, s, DischargeRequest { requested_useful_heat: e(50_000.0), required_supply_temperature: k(350.0), minimum_approach: approach, heat_exchanger_effectiveness: eff, duration: dt(100.0) }).unwrap();
        let bad = DischargeReport { useful_heat_delivered: e(49_000.0), ..r };
        assert!(matches!(verify_discharge_report(&c, s, DischargeRequest { requested_useful_heat: e(50_000.0), required_supply_temperature: k(350.0), minimum_approach: approach, heat_exchanger_effectiveness: eff, duration: dt(100.0) }, bad), Err(ModelError::ReportMismatch(_))));
    }
    #[test]
    fn rejects_zero_kelvin_and_efficiency_above_one() {
        assert_eq!(Kelvin::new(0.0), Err(ModelError::MustBePositive("temperature_k")));
        assert_eq!(Fraction::new(1.001), Err(ModelError::FractionOutsideUnitInterval));
    }
    #[test]
    fn rejects_non_monotone_cp_curve() {
        let r = HeatCapacityCurve::new(vec![
            HeatCapacityPoint::new(k(400.0), SpecificHeatJPerKgK::new(1_000.0).unwrap()),
            HeatCapacityPoint::new(k(399.0), SpecificHeatJPerKgK::new(1_100.0).unwrap()),
        ]);
        assert_eq!(r, Err(ModelError::HeatCapacityTemperaturesNotIncreasing));
    }
    #[test]
    fn deterministic_fixtures_replay_identically() {
        let i = CollectorInput { incident_solar: e(1_000.0), optical_efficiency: Fraction::new(0.8).unwrap(), receiver_loss: e(100.0) };
        assert_eq!(capture_solar(i), capture_solar(i));
        let c = cfg(300.0);
        assert_eq!(charge_store(&c, c.initial_state(), e(125_000.0), dt(100.0)), charge_store(&c, c.initial_state(), e(125_000.0), dt(100.0)));
    }
    #[test]
    fn discharge_obeys_power_limit_and_keeps_unmet_demand_visible() {
        let c = StoreConfig::new(StoreConfigSpec { max_discharge_power: PowerW::new(1_000.0).unwrap(), ..spec(&cfg(400.0), 400.0) }).unwrap();
        let s = c.initial_state(); let approach = TemperatureDifferenceK::new(5.0).unwrap(); let eff = Fraction::new(1.0).unwrap();
        let (after, r) = discharge_to_load(&c, s, DischargeRequest { requested_useful_heat: e(50_000.0), required_supply_temperature: k(310.0), minimum_approach: approach, heat_exchanger_effectiveness: eff, duration: dt(10.0) }).unwrap();
        assert!(close(r.source_heat_withdrawn.value(), 10_000.0));
        assert!(close(r.useful_heat_delivered.value(), 10_000.0));
        assert!(close(r.unmet_useful_heat.value(), 40_000.0));
        assert!(after.temperature.value() >= 315.0);
        verify_discharge_report(&c, s, DischargeRequest { requested_useful_heat: e(50_000.0), required_supply_temperature: k(310.0), minimum_approach: approach, heat_exchanger_effectiveness: eff, duration: dt(10.0) }, r).unwrap();
    }
}
