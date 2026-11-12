// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic battery-electric mission energy bookkeeping.
//!
//! This module does not predict aerodynamics, propeller performance, battery
//! chemistry, voltage sag, temperature, or aircraft safety. Shaft-power inputs
//! must come from a separately justified model or physical measurements.

use std::fmt;

pub const MODEL_VERSION: &str = "mission-energy-ledger/v0.1.0";
const SECONDS_PER_HOUR: f64 = 3_600.0;
const METRES_PER_KILOMETRE: f64 = 1_000.0;
const KILOGRAMS_PER_TONNE: f64 = 1_000.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentPurpose {
    PlannedMission,
    Reserve,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlightPhase {
    VerticalTakeoff,
    TransitionToCruise,
    Climb,
    Cruise,
    Descent,
    TransitionToHover,
    VerticalLanding,
    Hold,
    Diversion,
    Other,
}

/// One segment with constant average load and at most one peak interval.
///
/// The peak replaces average shaft power for its duration. Auxiliary DC load
/// is applied throughout the segment. Split a segment to represent multiple
/// separated peaks.
#[derive(Clone, Debug, PartialEq)]
pub struct MissionSegment {
    pub id: String,
    pub purpose: SegmentPurpose,
    pub phase: FlightPhase,
    pub duration_s: f64,
    pub distance_m: f64,
    /// Mechanical power at the propulsor shaft outside the peak interval.
    pub average_shaft_power_w: f64,
    /// Mechanical power at the propulsor shaft during the peak interval.
    pub peak_shaft_power_w: f64,
    pub peak_duration_s: f64,
    /// Auxiliary electrical load already measured at the pack DC bus.
    pub auxiliary_dc_power_w: f64,
    pub inverter_efficiency: f64,
    pub motor_efficiency: f64,
}

impl MissionSegment {
    fn validate(&self) -> Result<(), ModelError> {
        if self.id.trim().is_empty() {
            return Err(ModelError::InvalidSegment {
                segment_id: self.id.clone(),
                reason: "segment id must not be empty",
            });
        }
        for (name, value) in [
            ("duration_s", self.duration_s),
            ("distance_m", self.distance_m),
            ("average_shaft_power_w", self.average_shaft_power_w),
            ("peak_shaft_power_w", self.peak_shaft_power_w),
            ("peak_duration_s", self.peak_duration_s),
            ("auxiliary_dc_power_w", self.auxiliary_dc_power_w),
        ] {
            if !value.is_finite() || value < 0.0 {
                let reason = match name {
                    "duration_s" => "duration must be finite and non-negative",
                    "distance_m" => "distance must be finite and non-negative",
                    "average_shaft_power_w" => {
                        "average shaft power must be finite and non-negative"
                    },
                    "peak_shaft_power_w" => "peak shaft power must be finite and non-negative",
                    "peak_duration_s" => "peak duration must be finite and non-negative",
                    "auxiliary_dc_power_w" => "auxiliary DC power must be finite and non-negative",
                    _ => "invalid numeric segment input",
                };
                return Err(ModelError::InvalidSegment {
                    segment_id: self.id.clone(),
                    reason,
                });
            }
        }
        if self.duration_s <= 0.0 {
            return Err(ModelError::InvalidSegment {
                segment_id: self.id.clone(),
                reason: "duration must be greater than zero",
            });
        }
        if self.peak_duration_s > self.duration_s {
            return Err(ModelError::InvalidSegment {
                segment_id: self.id.clone(),
                reason: "peak duration must not exceed segment duration",
            });
        }
        if self.peak_shaft_power_w < self.average_shaft_power_w {
            return Err(ModelError::InvalidSegment {
                segment_id: self.id.clone(),
                reason: "peak shaft power must be at least average shaft power",
            });
        }
        if self.peak_duration_s == 0.0 && self.peak_shaft_power_w > self.average_shaft_power_w {
            return Err(ModelError::InvalidSegment {
                segment_id: self.id.clone(),
                reason: "a peak above average power requires a non-zero peak duration",
            });
        }
        for (name, efficiency) in [
            ("inverter_efficiency", self.inverter_efficiency),
            ("motor_efficiency", self.motor_efficiency),
        ] {
            if !efficiency.is_finite() || efficiency <= 0.0 || efficiency > 1.0 {
                let reason = match name {
                    "inverter_efficiency" => "inverter efficiency must be in (0, 1]",
                    "motor_efficiency" => "motor efficiency must be in (0, 1]",
                    _ => "invalid conversion efficiency",
                };
                return Err(ModelError::InvalidSegment {
                    segment_id: self.id.clone(),
                    reason,
                });
            }
        }
        let combined = self.inverter_efficiency * self.motor_efficiency;
        if !combined.is_finite() || combined <= 0.0 {
            return Err(ModelError::InvalidSegment {
                segment_id: self.id.clone(),
                reason: "combined inverter and motor efficiency must remain finite and positive",
            });
        }
        Ok(())
    }

    fn average_bus_power_w(&self) -> f64 {
        self.average_shaft_power_w / (self.inverter_efficiency * self.motor_efficiency)
            + self.auxiliary_dc_power_w
    }

    fn peak_bus_power_w(&self) -> f64 {
        self.peak_shaft_power_w / (self.inverter_efficiency * self.motor_efficiency)
            + self.auxiliary_dc_power_w
    }

    fn bus_energy_wh(&self) -> f64 {
        let non_peak_duration_s = self.duration_s - self.peak_duration_s;
        (self.average_bus_power_w() * non_peak_duration_s
            + self.peak_bus_power_w() * self.peak_duration_s)
            / SECONDS_PER_HOUR
    }
}

/// Battery capability measured at the pack DC-terminal boundary.
///
/// rated_dc_energy_wh_at_reference represents energy delivered from 100% to
/// 0% SOC under a declared reference temperature and discharge profile. It is
/// not cell-level or electrochemical energy. SOH scales it linearly in v0.1.
#[derive(Clone, Debug, PartialEq)]
pub struct BatteryEnvelope {
    pub rated_dc_energy_wh_at_reference: f64,
    pub state_of_health: f64,
    pub initial_state_of_charge: f64,
    pub minimum_state_of_charge: f64,
    pub max_continuous_dc_power_w: f64,
    pub max_peak_dc_power_w: f64,
    pub max_peak_duration_s: f64,
}

impl BatteryEnvelope {
    fn validate(&self) -> Result<(), ModelError> {
        if !self.rated_dc_energy_wh_at_reference.is_finite()
            || self.rated_dc_energy_wh_at_reference <= 0.0
        {
            return Err(ModelError::InvalidBattery("rated DC energy must be finite and positive"));
        }
        if !self.state_of_health.is_finite()
            || self.state_of_health <= 0.0
            || self.state_of_health > 1.0
        {
            return Err(ModelError::InvalidBattery("state of health must be in (0, 1]"));
        }
        if !self.initial_state_of_charge.is_finite()
            || !self.minimum_state_of_charge.is_finite()
            || self.minimum_state_of_charge < 0.0
            || self.initial_state_of_charge > 1.0
            || self.minimum_state_of_charge >= self.initial_state_of_charge
        {
            return Err(ModelError::InvalidBattery(
                "SOC must satisfy 0 <= minimum SOC < initial SOC <= 1",
            ));
        }
        if !self.max_continuous_dc_power_w.is_finite() || self.max_continuous_dc_power_w <= 0.0 {
            return Err(ModelError::InvalidBattery(
                "maximum continuous DC power must be finite and positive",
            ));
        }
        if !self.max_peak_dc_power_w.is_finite()
            || self.max_peak_dc_power_w < self.max_continuous_dc_power_w
        {
            return Err(ModelError::InvalidBattery(
                "maximum peak DC power must be finite and at least the continuous rating",
            ));
        }
        if !self.max_peak_duration_s.is_finite() || self.max_peak_duration_s < 0.0 {
            return Err(ModelError::InvalidBattery(
                "maximum peak duration must be finite and non-negative",
            ));
        }
        Ok(())
    }

    fn energy_scale_wh(&self) -> f64 {
        self.rated_dc_energy_wh_at_reference * self.state_of_health
    }

    fn available_energy_above_minimum_wh(&self) -> f64 {
        self.energy_scale_wh() * (self.initial_state_of_charge - self.minimum_state_of_charge)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MissionDefinition {
    pub mission_id: String,
    pub passenger_count: Option<u32>,
    pub payload_mass_kg: Option<f64>,
    pub segments: Vec<MissionSegment>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ModelError {
    InvalidMission(&'static str),
    InvalidBattery(&'static str),
    InvalidSegment { segment_id: String, reason: &'static str },
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMission(reason) => write!(f, "invalid mission: {reason}"),
            Self::InvalidBattery(reason) => write!(f, "invalid battery envelope: {reason}"),
            Self::InvalidSegment { segment_id, reason } => {
                write!(f, "invalid segment {segment_id:?}: {reason}")
            }
        }
    }
}

impl std::error::Error for ModelError {}

#[derive(Clone, Debug, PartialEq)]
pub enum Violation {
    ContinuousPowerExceeded { segment_id: String, required_w: f64, limit_w: f64 },
    PeakPowerExceeded { segment_id: String, required_w: f64, limit_w: f64 },
    PeakDurationExceeded { segment_id: String, required_s: f64, limit_s: f64 },
    ReserveEnergyShortfall { required_wh: f64, available_wh: f64, shortfall_wh: f64 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct SegmentEnergy {
    pub segment_id: String,
    pub purpose: SegmentPurpose,
    pub average_dc_power_w: f64,
    pub peak_dc_power_w: f64,
    pub bus_energy_wh: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnergyLedger {
    pub model_version: &'static str,
    pub mission_id: String,
    pub planned_mission_energy_wh: f64,
    pub reserve_energy_wh: f64,
    pub total_required_energy_wh: f64,
    pub available_energy_above_minimum_wh: f64,
    pub energy_margin_wh: f64,
    pub soc_after_planned_mission: f64,
    pub soc_after_planned_mission_and_reserve: f64,
    pub planned_distance_km: f64,
    pub wh_per_passenger_km: Option<f64>,
    pub wh_per_tonne_km: Option<f64>,
    pub segments: Vec<SegmentEnergy>,
    pub violations: Vec<Violation>,
}

impl EnergyLedger {
    pub fn passes_initial_screen(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Evaluate a battery-electric mission and its explicit reserve scenario.
///
/// Malformed inputs return an error. Power and reserve-energy limit breaches
/// return a ledger with violations so the user can inspect the failure.
pub fn evaluate_mission(
    mission: &MissionDefinition,
    battery: &BatteryEnvelope,
) -> Result<EnergyLedger, ModelError> {
    if mission.mission_id.trim().is_empty() {
        return Err(ModelError::InvalidMission("mission id must not be empty"));
    }
    if mission.segments.is_empty() {
        return Err(ModelError::InvalidMission("at least one segment is required"));
    }
    if mission.passenger_count == Some(0) {
        return Err(ModelError::InvalidMission("passenger_count must be positive when supplied"));
    }
    if let Some(mass) = mission.payload_mass_kg {
        if !mass.is_finite() || mass <= 0.0 {
            return Err(ModelError::InvalidMission("payload mass must be finite and positive"));
        }
    }
    battery.validate()?;

    let mut has_planned = false;
    let mut has_reserve = false;
    let mut planned_energy_wh = 0.0;
    let mut reserve_energy_wh = 0.0;
    let mut planned_distance_m = 0.0;
    let mut segment_results = Vec::with_capacity(mission.segments.len());
    let mut violations = Vec::new();

    for segment in &mission.segments {
        segment.validate()?;
        let average_bus_power_w = segment.average_bus_power_w();
        let peak_bus_power_w = segment.peak_bus_power_w();
        let bus_energy_wh = segment.bus_energy_wh();
        if !average_bus_power_w.is_finite()
            || !peak_bus_power_w.is_finite()
            || !bus_energy_wh.is_finite()
        {
            return Err(ModelError::InvalidSegment {
                segment_id: segment.id.clone(),
                reason: "calculated bus power and energy must remain finite",
            });
        }
        if average_bus_power_w > battery.max_continuous_dc_power_w {
            violations.push(Violation::ContinuousPowerExceeded {
                segment_id: segment.id.clone(),
                required_w: average_bus_power_w,
                limit_w: battery.max_continuous_dc_power_w,
            });
        }
        if segment.peak_duration_s > 0.0 {
            if peak_bus_power_w > battery.max_peak_dc_power_w {
                violations.push(Violation::PeakPowerExceeded {
                    segment_id: segment.id.clone(),
                    required_w: peak_bus_power_w,
                    limit_w: battery.max_peak_dc_power_w,
                });
            }
            if peak_bus_power_w > battery.max_continuous_dc_power_w
                && segment.peak_duration_s > battery.max_peak_duration_s
            {
                violations.push(Violation::PeakDurationExceeded {
                    segment_id: segment.id.clone(),
                    required_s: segment.peak_duration_s,
                    limit_s: battery.max_peak_duration_s,
                });
            }
        }
        match segment.purpose {
            SegmentPurpose::PlannedMission => {
                has_planned = true;
                planned_energy_wh += bus_energy_wh;
                planned_distance_m += segment.distance_m;
            }
            SegmentPurpose::Reserve => {
                has_reserve = true;
                reserve_energy_wh += bus_energy_wh;
            }
        }
        if !planned_energy_wh.is_finite()
            || !reserve_energy_wh.is_finite()
            || !planned_distance_m.is_finite()
        {
            return Err(ModelError::InvalidMission(
                "accumulated mission energy and distance must remain finite",
            ));
        }
        segment_results.push(SegmentEnergy {
            segment_id: segment.id.clone(),
            purpose: segment.purpose,
            average_dc_power_w: average_bus_power_w,
            peak_dc_power_w: peak_bus_power_w,
            bus_energy_wh,
        });
    }

    if !has_planned {
        return Err(ModelError::InvalidMission("at least one planned mission segment is required"));
    }
    if !has_reserve {
        return Err(ModelError::InvalidMission(
            "an explicit reserve segment is required; empty reserve assumptions are rejected",
        ));
    }
    if planned_energy_wh <= 0.0 || reserve_energy_wh <= 0.0 {
        return Err(ModelError::InvalidMission("planned and reserve energy must both be positive"));
    }

    let total_required_energy_wh = planned_energy_wh + reserve_energy_wh;
    if !total_required_energy_wh.is_finite() {
        return Err(ModelError::InvalidMission("total required energy must remain finite"));
    }
    let available_energy_wh = battery.available_energy_above_minimum_wh();
    let energy_margin_wh = available_energy_wh - total_required_energy_wh;
    if energy_margin_wh < 0.0 {
        violations.push(Violation::ReserveEnergyShortfall {
            required_wh: total_required_energy_wh,
            available_wh: available_energy_wh,
            shortfall_wh: -energy_margin_wh,
        });
    }

    let energy_scale_wh = battery.energy_scale_wh();
    let soc_after_planned_mission =
        battery.initial_state_of_charge - planned_energy_wh / energy_scale_wh;
    let soc_after_planned_mission_and_reserve =
        battery.initial_state_of_charge - total_required_energy_wh / energy_scale_wh;
    let planned_distance_km = planned_distance_m / METRES_PER_KILOMETRE;
    let wh_per_passenger_km = match (mission.passenger_count, planned_distance_km > 0.0) {
        (Some(passengers), true) => {
            Some(planned_energy_wh / (f64::from(passengers) * planned_distance_km))
        }
        _ => None,
    };
    let wh_per_tonne_km = match (mission.payload_mass_kg, planned_distance_km > 0.0) {
        (Some(mass_kg), true) => {
            Some(planned_energy_wh / ((mass_kg / KILOGRAMS_PER_TONNE) * planned_distance_km))
        }
        _ => None,
    };

    Ok(EnergyLedger {
        model_version: MODEL_VERSION,
        mission_id: mission.mission_id.clone(),
        planned_mission_energy_wh: planned_energy_wh,
        reserve_energy_wh,
        total_required_energy_wh,
        available_energy_above_minimum_wh: available_energy_wh,
        energy_margin_wh,
        soc_after_planned_mission,
        soc_after_planned_mission_and_reserve,
        planned_distance_km,
        wh_per_passenger_km,
        wh_per_tonne_km,
        segments: segment_results,
        violations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn battery() -> BatteryEnvelope {
        BatteryEnvelope {
            rated_dc_energy_wh_at_reference: 150_000.0,
            state_of_health: 0.9,
            initial_state_of_charge: 0.95,
            minimum_state_of_charge: 0.10,
            max_continuous_dc_power_w: 120_000.0,
            max_peak_dc_power_w: 150_000.0,
            max_peak_duration_s: 30.0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn segment(
        id: &str,
        purpose: SegmentPurpose,
        duration_s: f64,
        distance_m: f64,
        average_shaft_power_w: f64,
        peak_shaft_power_w: f64,
        peak_duration_s: f64,
        auxiliary_dc_power_w: f64,
        inverter_efficiency: f64,
        motor_efficiency: f64,
    ) -> MissionSegment {
        MissionSegment {
            id: id.to_owned(),
            purpose,
            phase: FlightPhase::Cruise,
            duration_s,
            distance_m,
            average_shaft_power_w,
            peak_shaft_power_w,
            peak_duration_s,
            auxiliary_dc_power_w,
            inverter_efficiency,
            motor_efficiency,
        }
    }

    fn mission() -> MissionDefinition {
        MissionDefinition {
            mission_id: "test-urban-evtol".to_owned(),
            passenger_count: Some(4),
            payload_mass_kg: Some(400.0),
            segments: vec![
                segment(
                    "planned-cruise",
                    SegmentPurpose::PlannedMission,
                    3_600.0,
                    80_467.2,
                    100_000.0,
                    110_000.0,
                    10.0,
                    2_000.0,
                    0.97,
                    0.95,
                ),
                segment(
                    "reserve-hold",
                    SegmentPurpose::Reserve,
                    600.0,
                    0.0,
                    20_000.0,
                    20_000.0,
                    0.0,
                    1_000.0,
                    1.0,
                    1.0,
                ),
            ],
        }
    }

    #[test]
    fn includes_auxiliaries_and_peak_interval_without_double_counting() {
        let ledger = evaluate_mission(&mission(), &battery()).unwrap();
        let average_bus_w = 100_000.0 / (0.97 * 0.95) + 2_000.0;
        let peak_bus_w = 110_000.0 / (0.97 * 0.95) + 2_000.0;
        let expected = (average_bus_w * 3_590.0 + peak_bus_w * 10.0) / SECONDS_PER_HOUR;
        assert!((ledger.planned_mission_energy_wh - expected).abs() < 1e-8);
        assert!((ledger.reserve_energy_wh - 3_500.0).abs() < 1e-8);
        assert_eq!(ledger.segments.len(), 2);
        assert!(ledger.energy_margin_wh > 0.0);
        assert!(ledger.passes_initial_screen());
    }

    #[test]
    fn reserve_shortfall_is_an_explicit_violation() {
        let mut undersized = battery();
        undersized.rated_dc_energy_wh_at_reference = 100_000.0;
        let ledger = evaluate_mission(&mission(), &undersized).unwrap();
        assert!(!ledger.passes_initial_screen());
        assert!(ledger
            .violations
            .iter()
            .any(|item| matches!(item, Violation::ReserveEnergyShortfall { .. })));
        assert!(ledger.energy_margin_wh < 0.0);
    }

    #[test]
    fn missing_reserve_profile_is_rejected() {
        let mut case = mission();
        case.segments.retain(|item| item.purpose == SegmentPurpose::PlannedMission);
        assert!(matches!(evaluate_mission(&case, &battery()), Err(ModelError::InvalidMission(_))));
    }

    #[test]
    fn continuous_peak_power_and_peak_duration_are_checked() {
        let mut case = mission();
        case.segments[0].average_shaft_power_w = 130_000.0;
        case.segments[0].peak_shaft_power_w = 180_000.0;
        case.segments[0].peak_duration_s = 45.0;
        let ledger = evaluate_mission(&case, &battery()).unwrap();
        assert!(ledger
            .violations
            .iter()
            .any(|item| matches!(item, Violation::ContinuousPowerExceeded { .. })));
        assert!(ledger
            .violations
            .iter()
            .any(|item| matches!(item, Violation::PeakPowerExceeded { .. })));
        assert!(ledger
            .violations
            .iter()
            .any(|item| matches!(item, Violation::PeakDurationExceeded { .. })));
    }

    #[test]
    fn invalid_efficiency_is_rejected() {
        let mut case = mission();
        case.segments[0].motor_efficiency = 1.01;
        assert!(matches!(
            evaluate_mission(&case, &battery()),
            Err(ModelError::InvalidSegment { reason: "motor efficiency must be in (0, 1]", .. })
        ));
    }

    #[test]
    fn non_finite_power_is_rejected() {
        let mut case = mission();
        case.segments[0].average_shaft_power_w = f64::NAN;
        assert!(matches!(
            evaluate_mission(&case, &battery()),
            Err(ModelError::InvalidSegment { .. })
        ));
    }

    #[test]
    fn computes_payload_normalized_energy_when_inputs_exist() {
        let ledger = evaluate_mission(&mission(), &battery()).unwrap();
        assert!(ledger.wh_per_passenger_km.unwrap().is_finite());
        assert!(ledger.wh_per_tonne_km.unwrap().is_finite());
        assert!(ledger.planned_distance_km > 0.0);
    }

    #[test]
    fn zero_energy_mission_is_rejected() {
        let mut case = mission();
        for item in &mut case.segments {
            item.average_shaft_power_w = 0.0;
            item.peak_shaft_power_w = 0.0;
            item.auxiliary_dc_power_w = 0.0;
        }
        assert!(matches!(evaluate_mission(&case, &battery()), Err(ModelError::InvalidMission(_))));
    }
}
