// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SensorStatus {
    Healthy,
    Stale,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorHealth {
    pub flow: SensorStatus,
    pub load_cell: SensorStatus,
    pub accelerometer: SensorStatus,
    pub feeder_encoder: SensorStatus,
    pub composition: SensorStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FlowStatus {
    Continuous,
    Reduced,
    Intermittent,
    NoFlow,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub sensors: SensorHealth,
    pub flow_status: FlowStatus,
    pub mass_flow_rate_kg_s: Option<f64>,
    pub pocket_fill_fraction: Option<f64>,
    pub measured_gamma: Option<f64>,
    pub measured_vibration_power_w: Option<f64>,
    pub feeder_speed_fraction: Option<f64>,
    pub segregation_index: Option<f64>,
    pub fragmentation_fraction: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorConfig {
    pub material_profile_id: String,
    pub apparatus_profile_id: String,
    /// Measured minimum vibration level commissioned for this specific
    /// material batch, wall, apparatus, gravity, pressure and fill regime.
    pub gamma_min_validated: f64,
    pub gamma_max_validated: f64,
    pub gamma_step: f64,
    pub initial_gamma: f64,
    pub gamma_tracking_tolerance_fraction: f64,
    pub feeder_speed_tracking_tolerance_fraction: f64,
    pub feeder_speed_min_fraction: f64,
    pub feeder_speed_max_fraction: f64,
    pub feeder_speed_step_fraction: f64,
    pub initial_feeder_speed_fraction: f64,
    pub target_mass_flow_rate_kg_s: f64,
    pub flow_tolerance_fraction: f64,
    pub pocket_fill_target_fraction: f64,
    pub pocket_fill_tolerance_fraction: f64,
    pub stable_windows_before_gamma_trim: u32,
    pub max_segregation_index: Option<f64>,
    pub max_fragmentation_fraction: Option<f64>,
}

impl SupervisorConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.material_profile_id.trim().is_empty()
            || self.apparatus_profile_id.trim().is_empty()
        {
            return Err("material and apparatus profile IDs are required".to_owned());
        }

        for (value, name) in [
            (self.gamma_min_validated, "gamma_min_validated"),
            (self.gamma_max_validated, "gamma_max_validated"),
            (self.gamma_step, "gamma_step"),
            (self.initial_gamma, "initial_gamma"),
            (self.target_mass_flow_rate_kg_s, "target_mass_flow_rate_kg_s"),
            (self.feeder_speed_step_fraction, "feeder_speed_step_fraction"),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(format!("{name} must be finite and positive"));
            }
        }

        if !self.gamma_tracking_tolerance_fraction.is_finite()
            || !(0.0..=1.0).contains(&self.gamma_tracking_tolerance_fraction)
        {
            return Err("gamma_tracking_tolerance_fraction must be finite and in [0, 1]".to_owned());
        }

        if self.gamma_min_validated > self.gamma_max_validated
            || self.initial_gamma < self.gamma_min_validated
            || self.initial_gamma > self.gamma_max_validated
        {
            return Err("gamma commissioning bounds or initial setpoint are inconsistent".to_owned());
        }

        for (value, name) in [
            (self.feeder_speed_min_fraction, "feeder_speed_min_fraction"),
            (self.feeder_speed_max_fraction, "feeder_speed_max_fraction"),
            (self.initial_feeder_speed_fraction, "initial_feeder_speed_fraction"),
            (self.flow_tolerance_fraction, "flow_tolerance_fraction"),
            (self.pocket_fill_target_fraction, "pocket_fill_target_fraction"),
            (self.pocket_fill_tolerance_fraction, "pocket_fill_tolerance_fraction"),
        ] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(format!("{name} must be finite and in [0, 1]"));
            }
        }

        if !self.feeder_speed_tracking_tolerance_fraction.is_finite()
            || !(0.0..=1.0).contains(&self.feeder_speed_tracking_tolerance_fraction)
        {
            return Err("feeder_speed_tracking_tolerance_fraction must be finite and in [0, 1]".to_owned());
        }

        if self.pocket_fill_tolerance_fraction > self.pocket_fill_target_fraction {
            return Err("pocket_fill_tolerance_fraction must not exceed the target fraction".to_owned());
        }

        if self.feeder_speed_min_fraction > self.feeder_speed_max_fraction
            || self.initial_feeder_speed_fraction < self.feeder_speed_min_fraction
            || self.initial_feeder_speed_fraction > self.feeder_speed_max_fraction
        {
            return Err("feeder speed commissioning bounds or initial speed are inconsistent".to_owned());
        }

        if self.stable_windows_before_gamma_trim == 0 {
            return Err("stable_windows_before_gamma_trim must be at least one".to_owned());
        }

        for (value, name) in [
            (self.max_segregation_index, "max_segregation_index"),
            (self.max_fragmentation_fraction, "max_fragmentation_fraction"),
        ] {
            if let Some(value) = value {
                if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                    return Err(format!("{name} must be finite and in [0, 1]"));
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorState {
    pub commanded_gamma: f64,
    pub commanded_feeder_speed_fraction: f64,
    pub stable_windows: u32,
    pub recovery_pending: bool,
}

impl SupervisorState {
    pub fn initial(config: &SupervisorConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            commanded_gamma: config.initial_gamma,
            commanded_feeder_speed_fraction: config.initial_feeder_speed_fraction,
            stable_windows: 0,
            recovery_pending: false,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FeederCommand {
    Hold,
    SetSpeedFraction { value: f64 },
    Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VibrationCommand {
    Hold,
    SetGamma { value: f64 },
    Off,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionMode {
    Normal,
    FlowRecovery,
    QualityHold,
    SensorHold,
    CommissioningLimitReached,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub report_class: String,
    pub mode: DecisionMode,
    pub feeder_command: FeederCommand,
    pub vibration_command: VibrationCommand,
    pub commanded_gamma_after_decision: f64,
    pub commanded_feeder_speed_fraction_after_decision: f64,
    pub reason: String,
    pub energy_observation_available: bool,
    pub claim_ceiling: String,
}

pub struct FeederSupervisor {
    pub config: SupervisorConfig,
    pub state: SupervisorState,
}

impl FeederSupervisor {
    pub fn new(config: SupervisorConfig) -> Result<Self, String> {
        let state = SupervisorState::initial(&config)?;
        Ok(Self { config, state })
    }

    fn decision(
        &self,
        mode: DecisionMode,
        feeder_command: FeederCommand,
        vibration_command: VibrationCommand,
        reason: &str,
        energy_observation_available: bool,
    ) -> Decision {
        Decision {
            report_class: "lunar_regolith_feeder_supervisory_recommendation".to_owned(),
            mode,
            feeder_command,
            vibration_command,
            commanded_gamma_after_decision: self.state.commanded_gamma,
            commanded_feeder_speed_fraction_after_decision:
                self.state.commanded_feeder_speed_fraction,
            reason: reason.to_owned(),
            energy_observation_available,
            claim_ceiling: "Advisory supervisory logic only. Commands are not executed by this tool. Commissioning bounds are specific to the named material/apparatus/environment. This is not certified machine-control, functional-safety, flight-software or lunar-hardware qualification.".to_owned(),
        }
    }

    fn hold_all(&mut self, reason: &str, mode: DecisionMode) -> Decision {
        self.state.stable_windows = 0;
        self.decision(
            mode,
            FeederCommand::Stop,
            VibrationCommand::Off,
            reason,
            false,
        )
    }

    pub fn step(&mut self, observation: &Observation) -> Result<Decision, String> {
        self.config.validate()?;

        if let Some(power) = observation.measured_vibration_power_w {
            if !power.is_finite() || power < 0.0 {
                return Err("measured_vibration_power_w must be finite and non-negative".to_owned());
            }
        }
        let energy_available = observation.measured_vibration_power_w.is_some();

        let core_sensors_healthy = [
            observation.sensors.flow,
            observation.sensors.load_cell,
            observation.sensors.accelerometer,
            observation.sensors.feeder_encoder,
        ]
        .into_iter()
        .all(|status| status == SensorStatus::Healthy);

        if !core_sensors_healthy {
            return Ok(self.hold_all(
                "A critical flow, load-cell, accelerometer or feeder-encoder signal is stale or failed; stop material withdrawal and request reconciliation.",
                DecisionMode::SensorHold,
            ));
        }

        let (Some(mass_flow), Some(pocket_fill), Some(measured_gamma), Some(speed_fraction)) = (
            observation.mass_flow_rate_kg_s,
            observation.pocket_fill_fraction,
            observation.measured_gamma,
            observation.feeder_speed_fraction,
        ) else {
            return Ok(self.hold_all(
                "A required mass-flow, pocket-fill, measured-Gamma or feeder-speed observation is unknown; no new material withdrawal is recommended.",
                DecisionMode::SensorHold,
            ));
        };

        for (value, name) in [
            (mass_flow, "mass_flow_rate_kg_s"),
            (pocket_fill, "pocket_fill_fraction"),
            (measured_gamma, "measured_gamma"),
            (speed_fraction, "feeder_speed_fraction"),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(format!("{name} must be finite and non-negative"));
            }
        }

        if pocket_fill > 1.0 || speed_fraction > 1.0 {
            return Err("pocket fill and feeder speed fractions must not exceed 1.0".to_owned());
        }

        if !self.state.commanded_gamma.is_finite()
            || !self.state.commanded_feeder_speed_fraction.is_finite()
            || self.state.commanded_gamma < self.config.gamma_min_validated
            || self.state.commanded_gamma > self.config.gamma_max_validated
            || self.state.commanded_feeder_speed_fraction < self.config.feeder_speed_min_fraction
            || self.state.commanded_feeder_speed_fraction > self.config.feeder_speed_max_fraction
        {
            return Ok(self.hold_all(
                "Stored supervisor state is outside the commissioned envelope; stop and reconcile the last acknowledged actuator state.",
                DecisionMode::CommissioningLimitReached,
            ));
        }

        if measured_gamma > self.config.gamma_max_validated {
            return Ok(self.hold_all(
                "Accelerometer-measured vibration exceeds the commissioned maximum; stop feed and inspect the actuator/resonance state.",
                DecisionMode::CommissioningLimitReached,
            ));
        }

        if observation.flow_status == FlowStatus::Continuous
            && measured_gamma < self.config.gamma_min_validated
        {
            self.state.stable_windows = 0;
            self.state.commanded_gamma = self.config.gamma_min_validated;
            return Ok(self.decision(
                DecisionMode::FlowRecovery,
                FeederCommand::Stop,
                VibrationCommand::SetGamma { value: self.config.gamma_min_validated },
                "Flow appears continuous below the material/apparatus-specific validated Gamma envelope; pause withdrawal and restore the commissioned minimum.",
                energy_available,
            ));
        }

        let gamma_tracking_error = (measured_gamma - self.state.commanded_gamma).abs()
            / self.state.commanded_gamma.max(1.0e-9);
        if gamma_tracking_error > self.config.gamma_tracking_tolerance_fraction {
            return Ok(self.hold_all(
                "Measured Gamma differs from the last acknowledged target beyond the tracking tolerance; stop feed and inspect resonance, mounting and accelerometer calibration before further recovery commands.",
                DecisionMode::SensorHold,
            ));
        }

        if speed_fraction < self.config.feeder_speed_min_fraction
            || speed_fraction > self.config.feeder_speed_max_fraction
        {
            return Ok(self.hold_all(
                "Measured feeder speed lies outside the commissioned range; stop withdrawal and reconcile actuator feedback.",
                DecisionMode::CommissioningLimitReached,
            ));
        }

        let feeder_tracking_error =
            (speed_fraction - self.state.commanded_feeder_speed_fraction).abs()
                / self.state.commanded_feeder_speed_fraction.max(1.0e-9);
        if feeder_tracking_error > self.config.feeder_speed_tracking_tolerance_fraction {
            return Ok(self.hold_all(
                "Measured feeder speed differs from the last acknowledged setpoint beyond the tracking tolerance; stop withdrawal and reconcile encoder/drive feedback.",
                DecisionMode::SensorHold,
            ));
        }

        if self.config.max_segregation_index.is_some()
            || self.config.max_fragmentation_fraction.is_some()
        {
            if observation.sensors.composition != SensorStatus::Healthy {
                return Ok(self.hold_all(
                    "Composition/quality monitoring is required by this profile but is stale or failed.",
                    DecisionMode::SensorHold,
                ));
            }
        }

        if let Some(limit) = self.config.max_segregation_index {
            let Some(value) = observation.segregation_index else {
                return Ok(self.hold_all(
                    "A required segregation index is unknown; stop accepted-feedstock delivery.",
                    DecisionMode::SensorHold,
                ));
            };
            if !value.is_finite() || value < 0.0 || value > 1.0 {
                return Err("segregation_index must be finite and in [0, 1]".to_owned());
            }
            if value > limit {
                return Ok(self.hold_all(
                    "Composition segregation exceeds the configured feed specification; stop feed and request quarantine/remix assessment.",
                    DecisionMode::QualityHold,
                ));
            }
        }

        if let Some(limit) = self.config.max_fragmentation_fraction {
            let Some(value) = observation.fragmentation_fraction else {
                return Ok(self.hold_all(
                    "A required agglomerate-fragmentation metric is unknown; stop accepted-feedstock delivery.",
                    DecisionMode::SensorHold,
                ));
            };
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err("fragmentation_fraction must be finite and in [0, 1]".to_owned());
            }
            if value > limit {
                return Ok(self.hold_all(
                    "Agglomerate fragmentation exceeds the configured material-quality limit; stop feed and request material review.",
                    DecisionMode::QualityHold,
                ));
            }
        }

        if observation.flow_status == FlowStatus::Unknown {
            return Ok(self.hold_all(
                "Flow-continuity state is unknown; do not infer successful feed from feeder rotation alone.",
                DecisionMode::SensorHold,
            ));
        }

        if matches!(
            observation.flow_status,
            FlowStatus::NoFlow | FlowStatus::Intermittent
        ) {
            self.state.stable_windows = 0;
            self.state.recovery_pending = true;

            if self.state.commanded_gamma >= self.config.gamma_max_validated {
                return Ok(self.decision(
                    DecisionMode::CommissioningLimitReached,
                    FeederCommand::Stop,
                    VibrationCommand::Hold,
                    "No-flow/intermittent condition persists at the commissioned Gamma ceiling; do not increase vibration beyond the qualified envelope.",
                    energy_available,
                ));
            }

            self.state.commanded_gamma = (self.state.commanded_gamma + self.config.gamma_step)
                .max(self.config.gamma_min_validated)
                .min(self.config.gamma_max_validated);

            return Ok(self.decision(
                DecisionMode::FlowRecovery,
                FeederCommand::Stop,
                VibrationCommand::SetGamma {
                    value: self.state.commanded_gamma,
                },
                "Pause metered withdrawal and raise vibration by one commissioned step; require observed flow recovery before resuming the feeder.",
                energy_available,
            ));
        }

        if self.state.recovery_pending {
            self.state.recovery_pending = false;
            self.state.stable_windows = 0;
            self.state.commanded_feeder_speed_fraction = self
                .state
                .commanded_feeder_speed_fraction
                .clamp(
                    self.config.feeder_speed_min_fraction,
                    self.config.feeder_speed_max_fraction,
                );

            return Ok(self.decision(
                DecisionMode::Normal,
                FeederCommand::SetSpeedFraction {
                    value: self.state.commanded_feeder_speed_fraction,
                },
                VibrationCommand::Hold,
                "Continuous flow has been re-observed after recovery; resume at the last commissioned feeder-speed setpoint and re-evaluate on the next measurement window.",
                energy_available,
            ));
        }

        let lower_rate = self.config.target_mass_flow_rate_kg_s
            * (1.0 - self.config.flow_tolerance_fraction);
        let upper_rate = self.config.target_mass_flow_rate_kg_s
            * (1.0 + self.config.flow_tolerance_fraction);
        let lower_fill = self.config.pocket_fill_target_fraction
            - self.config.pocket_fill_tolerance_fraction;

        if observation.flow_status == FlowStatus::Reduced || pocket_fill < lower_fill {
            self.state.stable_windows = 0;

            if self.state.commanded_gamma < self.config.gamma_max_validated {
                self.state.commanded_gamma = (self.state.commanded_gamma
                    + self.config.gamma_step)
                    .max(self.config.gamma_min_validated)
                    .min(self.config.gamma_max_validated);

                return Ok(self.decision(
                    DecisionMode::FlowRecovery,
                    FeederCommand::Stop,
                    VibrationCommand::SetGamma {
                        value: self.state.commanded_gamma,
                    },
                    "Reduced supply or under-filled feeder pockets detected; increase vibration one commissioned step, then verify actual Gamma and pocket fill.",
                    energy_available,
                ));
            }

            return Ok(self.decision(
                DecisionMode::CommissioningLimitReached,
                FeederCommand::Stop,
                VibrationCommand::Hold,
                "Pocket fill or supply continuity remains low at the commissioned Gamma ceiling; stop withdrawal and inspect.",
                energy_available,
            ));
        }

        if mass_flow < lower_rate {
            self.state.stable_windows = 0;

            if pocket_fill >= lower_fill {
                self.state.commanded_feeder_speed_fraction =
                    (self.state.commanded_feeder_speed_fraction
                        + self.config.feeder_speed_step_fraction)
                        .min(self.config.feeder_speed_max_fraction);

                return Ok(self.decision(
                    DecisionMode::Normal,
                    FeederCommand::SetSpeedFraction {
                        value: self.state.commanded_feeder_speed_fraction,
                    },
                    VibrationCommand::Hold,
                    "Pocket fill is adequate but delivered rate is below target; adjust feeder metering, not vibration.",
                    energy_available,
                ));
            }
        }

        if mass_flow > upper_rate {
            self.state.stable_windows = 0;
            self.state.commanded_feeder_speed_fraction =
                (self.state.commanded_feeder_speed_fraction
                    - self.config.feeder_speed_step_fraction)
                    .max(self.config.feeder_speed_min_fraction);

            return Ok(self.decision(
                DecisionMode::Normal,
                FeederCommand::SetSpeedFraction {
                    value: self.state.commanded_feeder_speed_fraction,
                },
                VibrationCommand::Hold,
                "Delivered mass rate exceeds target; reduce feeder metering while keeping vibration unchanged.",
                energy_available,
            ));
        }

        self.state.stable_windows = self.state.stable_windows.saturating_add(1);

        if energy_available
            && self.state.stable_windows >= self.config.stable_windows_before_gamma_trim
            && self.state.commanded_gamma > self.config.gamma_min_validated
        {
            self.state.stable_windows = 0;
            self.state.commanded_gamma = (self.state.commanded_gamma
                - self.config.gamma_step)
                .max(self.config.gamma_min_validated);

            return Ok(self.decision(
                DecisionMode::Normal,
                FeederCommand::Hold,
                VibrationCommand::SetGamma {
                    value: self.state.commanded_gamma,
                },
                "Flow, pocket fill and mass rate are stable; reduce vibration one step toward the material/apparatus-specific validated minimum.",
                energy_available,
            ));
        }

        Ok(self.decision(
            DecisionMode::Normal,
            FeederCommand::Hold,
            VibrationCommand::Hold,
            "Flow and metered delivery are within configured limits; retain current setpoints.",
            energy_available,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> SupervisorConfig {
        SupervisorConfig {
            material_profile_id: "SYNTHETIC-MATERIAL".to_owned(),
            apparatus_profile_id: "SYNTHETIC-HOPPER-FEEDER".to_owned(),
            gamma_min_validated: 3.4,
            gamma_max_validated: 10.0,
            gamma_step: 1.0,
            initial_gamma: 4.4,
            gamma_tracking_tolerance_fraction: 0.2,
            feeder_speed_tracking_tolerance_fraction: 0.2,
            feeder_speed_min_fraction: 0.1,
            feeder_speed_max_fraction: 1.0,
            feeder_speed_step_fraction: 0.1,
            initial_feeder_speed_fraction: 0.5,
            target_mass_flow_rate_kg_s: 0.01,
            flow_tolerance_fraction: 0.1,
            pocket_fill_target_fraction: 0.9,
            pocket_fill_tolerance_fraction: 0.1,
            stable_windows_before_gamma_trim: 3,
            max_segregation_index: Some(0.15),
            max_fragmentation_fraction: Some(0.05),
        }
    }

    fn observation() -> Observation {
        Observation {
            sensors: SensorHealth {
                flow: SensorStatus::Healthy,
                load_cell: SensorStatus::Healthy,
                accelerometer: SensorStatus::Healthy,
                feeder_encoder: SensorStatus::Healthy,
                composition: SensorStatus::Healthy,
            },
            flow_status: FlowStatus::Continuous,
            mass_flow_rate_kg_s: Some(0.01),
            pocket_fill_fraction: Some(0.95),
            measured_gamma: Some(4.4),
            measured_vibration_power_w: Some(1.0),
            feeder_speed_fraction: Some(0.5),
            segregation_index: Some(0.05),
            fragmentation_fraction: Some(0.01),
        }
    }

    #[test]
    fn stale_sensor_stops_material_withdrawal() {
        let mut supervisor = FeederSupervisor::new(config()).unwrap();
        let mut obs = observation();
        obs.sensors.accelerometer = SensorStatus::Stale;
        let decision = supervisor.step(&obs).unwrap();
        assert_eq!(decision.mode, DecisionMode::SensorHold);
        assert_eq!(decision.feeder_command, FeederCommand::Stop);
        assert_eq!(decision.vibration_command, VibrationCommand::Off);
    }

    #[test]
    fn no_flow_raises_gamma_and_pauses_feeder() {
        let mut supervisor = FeederSupervisor::new(config()).unwrap();
        let mut obs = observation();
        obs.flow_status = FlowStatus::NoFlow;
        obs.mass_flow_rate_kg_s = Some(0.0);
        let decision = supervisor.step(&obs).unwrap();
        assert_eq!(decision.mode, DecisionMode::FlowRecovery);
        assert_eq!(decision.feeder_command, FeederCommand::Stop);
        assert_eq!(
            decision.vibration_command,
            VibrationCommand::SetGamma { value: 5.4 }
        );
    }

    #[test]
    fn no_flow_at_gamma_ceiling_does_not_exceed_commissioned_limit() {
        let mut cfg = config();
        cfg.initial_gamma = cfg.gamma_max_validated;
        let mut supervisor = FeederSupervisor::new(cfg).unwrap();
        let mut obs = observation();
        obs.flow_status = FlowStatus::NoFlow;
        obs.mass_flow_rate_kg_s = Some(0.0);
        obs.measured_gamma = Some(10.0);
        let decision = supervisor.step(&obs).unwrap();
        assert_eq!(decision.mode, DecisionMode::CommissioningLimitReached);
        assert_eq!(decision.feeder_command, FeederCommand::Stop);
        assert_eq!(supervisor.state.commanded_gamma, 10.0);
    }

    #[test]
    fn low_pocket_fill_raises_vibration_not_feeder_rpm() {
        let mut supervisor = FeederSupervisor::new(config()).unwrap();
        let mut obs = observation();
        obs.pocket_fill_fraction = Some(0.5);
        let decision = supervisor.step(&obs).unwrap();
        assert_eq!(
            decision.vibration_command,
            VibrationCommand::SetGamma { value: 5.4 }
        );
        assert_eq!(decision.feeder_command, FeederCommand::Stop);
        assert_eq!(supervisor.state.commanded_feeder_speed_fraction, 0.5);
    }

    #[test]
    fn adequate_pocket_fill_with_low_rate_adjusts_feeder_only() {
        let mut supervisor = FeederSupervisor::new(config()).unwrap();
        let mut obs = observation();
        obs.mass_flow_rate_kg_s = Some(0.007);
        let decision = supervisor.step(&obs).unwrap();
        assert_eq!(
            decision.feeder_command,
            FeederCommand::SetSpeedFraction { value: 0.6 }
        );
        assert_eq!(decision.vibration_command, VibrationCommand::Hold);
    }

    #[test]
    fn excessive_delivery_reduces_feeder_speed_not_gamma() {
        let mut supervisor = FeederSupervisor::new(config()).unwrap();
        let mut obs = observation();
        obs.mass_flow_rate_kg_s = Some(0.02);
        let decision = supervisor.step(&obs).unwrap();
        assert_eq!(
            decision.feeder_command,
            FeederCommand::SetSpeedFraction { value: 0.4 }
        );
        assert_eq!(decision.vibration_command, VibrationCommand::Hold);
    }

    #[test]
    fn segregation_violation_stops_feed_and_vibration() {
        let mut supervisor = FeederSupervisor::new(config()).unwrap();
        let mut obs = observation();
        obs.segregation_index = Some(0.3);
        let decision = supervisor.step(&obs).unwrap();
        assert_eq!(decision.mode, DecisionMode::QualityHold);
        assert_eq!(decision.feeder_command, FeederCommand::Stop);
        assert_eq!(decision.vibration_command, VibrationCommand::Off);
    }

    #[test]
    fn unknown_quality_metric_stops_when_quality_limit_is_enabled() {
        let mut supervisor = FeederSupervisor::new(config()).unwrap();
        let mut obs = observation();
        obs.fragmentation_fraction = None;
        let decision = supervisor.step(&obs).unwrap();
        assert_eq!(decision.mode, DecisionMode::SensorHold);
        assert_eq!(decision.feeder_command, FeederCommand::Stop);
    }

    #[test]
    fn stable_flow_trims_vibration_but_never_below_validated_minimum() {
        let mut supervisor = FeederSupervisor::new(config()).unwrap();
        let obs = observation();
        supervisor.step(&obs).unwrap();
        supervisor.step(&obs).unwrap();
        let decision = supervisor.step(&obs).unwrap();
        assert_eq!(
            decision.vibration_command,
            VibrationCommand::SetGamma { value: 3.4 }
        );
        assert_eq!(supervisor.state.commanded_gamma, 3.4);
    }

    #[test]
    fn commanded_and_measured_gamma_mismatch_stops_withdrawal() {
        let mut supervisor = FeederSupervisor::new(config()).unwrap();
        let mut obs = observation();
        obs.measured_gamma = Some(3.4);
        let decision = supervisor.step(&obs).unwrap();
        assert_eq!(decision.mode, DecisionMode::SensorHold);
        assert_eq!(decision.feeder_command, FeederCommand::Stop);
        assert_eq!(decision.vibration_command, VibrationCommand::Off);
    }

    #[test]
    fn gamma_tracking_is_checked_during_no_flow_recovery() {
        let mut supervisor = FeederSupervisor::new(config()).unwrap();
        let mut obs = observation();
        obs.flow_status = FlowStatus::NoFlow;
        obs.measured_gamma = Some(1.0);
        let decision = supervisor.step(&obs).unwrap();
        assert_eq!(decision.mode, DecisionMode::SensorHold);
        assert_eq!(decision.feeder_command, FeederCommand::Stop);
    }

    #[test]
    fn feeder_encoder_tracking_mismatch_stops_withdrawal() {
        let mut supervisor = FeederSupervisor::new(config()).unwrap();
        let mut obs = observation();
        obs.feeder_speed_fraction = Some(0.8);
        let decision = supervisor.step(&obs).unwrap();
        assert_eq!(decision.mode, DecisionMode::SensorHold);
        assert_eq!(decision.feeder_command, FeederCommand::Stop);
    }

    #[test]
    fn invalid_pocket_fill_tolerance_is_rejected() {
        let mut cfg = config();
        cfg.pocket_fill_tolerance_fraction = 0.95;
        assert!(FeederSupervisor::new(cfg).is_err());
    }

    #[test]
    fn energy_trim_is_not_recommended_without_power_measurement() {
        let mut supervisor = FeederSupervisor::new(config()).unwrap();
        let mut obs = observation();
        obs.measured_vibration_power_w = None;
        supervisor.step(&obs).unwrap();
        supervisor.step(&obs).unwrap();
        let decision = supervisor.step(&obs).unwrap();
        assert_eq!(decision.vibration_command, VibrationCommand::Hold);
        assert_eq!(supervisor.state.commanded_gamma, 4.4);
    }

    #[test]
    fn recovered_flow_resumes_at_previous_feeder_setpoint() {
        let mut supervisor = FeederSupervisor::new(config()).unwrap();
        let mut stopped = observation();
        stopped.flow_status = FlowStatus::NoFlow;
        stopped.mass_flow_rate_kg_s = Some(0.0);
        supervisor.step(&stopped).unwrap();

        let mut recovered = observation();
        recovered.measured_gamma = Some(5.4);
        let decision = supervisor.step(&recovered).unwrap();
        assert_eq!(
            decision.feeder_command,
            FeederCommand::SetSpeedFraction { value: 0.5 }
        );
        assert_eq!(decision.mode, DecisionMode::Normal);
    }

    #[test]
    fn missing_composition_sensor_stops_when_quality_limits_are_enabled() {
        let mut supervisor = FeederSupervisor::new(config()).unwrap();
        let mut obs = observation();
        obs.sensors.composition = SensorStatus::Stale;
        let decision = supervisor.step(&obs).unwrap();
        assert_eq!(decision.mode, DecisionMode::SensorHold);
    }

    #[test]
    fn invalid_observations_are_rejected() {
        let mut supervisor = FeederSupervisor::new(config()).unwrap();
        let mut obs = observation();
        obs.mass_flow_rate_kg_s = Some(f64::NAN);
        assert!(supervisor.step(&obs).is_err());
    }
}
