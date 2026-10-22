//! Controller-neutral benchmark scenario catalogue for flight-control robustness.
//!
//! Training, validation and held-out test families are intentionally explicit.
//! Test families must never be used to select gains or residual authority.
use serde::Serialize;
use symthaea_multirotor::FlightSetpoint;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioSplit {
    Train,
    Validation,
    Test,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DisturbanceProfile {
    None,
    XImpulse,
    YImpulse,
    ZImpulse,
    AlternatingXy,
    SineGustXy,
}

/// One predeclared static-target scenario and deterministic disturbance profile.
/// Disturbance magnitude is normalized to vehicle weight, not to an arbitrary
/// force in Newtons, so the stress level remains interpretable if mass changes.
#[derive(Debug, Clone, Serialize)]
pub struct FlightScenario {
    pub id: &'static str,
    pub family: &'static str,
    pub split: ScenarioSplit,
    pub setpoint: FlightSetpoint,
    pub disturbance: DisturbanceProfile,
    pub pulse_start_step: usize,
    pub pulse_duration_steps: usize,
    pub disturbance_fraction_of_weight: f64,
}

impl FlightScenario {
    /// Return the world-frame external force applied for this fixed evaluation step.
    /// The seed changes pulse onset deterministically but not the scenario family.
    pub fn force_at(&self, seed: u64, step: usize, mass_kg: f64) -> [f64; 3] {
        if self.disturbance == DisturbanceProfile::None
            || !mass_kg.is_finite()
            || mass_kg <= 0.0
            || !self.disturbance_fraction_of_weight.is_finite()
            || self.disturbance_fraction_of_weight <= 0.0
        {
            return [0.0; 3];
        }

        let force_n = self.disturbance_fraction_of_weight * mass_kg * 9.81;
        let onset = self.pulse_start_step + (seed % 17) as usize;
        let pulse_active = (onset..onset.saturating_add(self.pulse_duration_steps)).contains(&step);

        match self.disturbance {
            DisturbanceProfile::None => [0.0; 3],
            DisturbanceProfile::XImpulse if pulse_active => [force_n, 0.0, 0.0],
            DisturbanceProfile::YImpulse if pulse_active => [0.0, -force_n, 0.0],
            DisturbanceProfile::ZImpulse if pulse_active => [0.0, 0.0, force_n],
            DisturbanceProfile::AlternatingXy if pulse_active => {
                if ((step - onset) / 10) % 2 == 0 {
                    [force_n, 0.0, 0.0]
                } else {
                    [0.0, -force_n, 0.0]
                }
            }
            DisturbanceProfile::SineGustXy if step >= onset => {
                let phase = (step - onset) as f64 * 0.075;
                [force_n * phase.sin(), force_n * 0.7 * (phase * 0.83).cos(), 0.0]
            }
            _ => [0.0; 3],
        }
    }
}

/// Full fixed catalogue. Do not silently move a case between splits after seeing results.
pub fn scenario_catalogue() -> Vec<FlightScenario> {
    vec![
        FlightScenario {
            id: "train_hover_10cm",
            family: "hover",
            split: ScenarioSplit::Train,
            setpoint: FlightSetpoint { position: [0.0, 0.0, 0.10], yaw: 0.0 },
            disturbance: DisturbanceProfile::None,
            pulse_start_step: 50,
            pulse_duration_steps: 40,
            disturbance_fraction_of_weight: 0.0,
        },
        FlightScenario {
            id: "train_altitude_20cm",
            family: "altitude_step",
            split: ScenarioSplit::Train,
            setpoint: FlightSetpoint { position: [0.0, 0.0, 0.20], yaw: 0.0 },
            disturbance: DisturbanceProfile::None,
            pulse_start_step: 50,
            pulse_duration_steps: 40,
            disturbance_fraction_of_weight: 0.0,
        },
        FlightScenario {
            id: "train_x_step_5pct",
            family: "lateral_step",
            split: ScenarioSplit::Train,
            setpoint: FlightSetpoint { position: [0.05, 0.0, 0.12], yaw: 0.0 },
            disturbance: DisturbanceProfile::XImpulse,
            pulse_start_step: 60,
            pulse_duration_steps: 35,
            disturbance_fraction_of_weight: 0.05,
        },
        FlightScenario {
            id: "train_y_step_5pct",
            family: "lateral_step",
            split: ScenarioSplit::Train,
            setpoint: FlightSetpoint { position: [0.0, 0.05, 0.15], yaw: 0.0 },
            disturbance: DisturbanceProfile::YImpulse,
            pulse_start_step: 65,
            pulse_duration_steps: 35,
            disturbance_fraction_of_weight: 0.05,
        },
        FlightScenario {
            id: "validation_diagonal_10pct",
            family: "combined_axis",
            split: ScenarioSplit::Validation,
            setpoint: FlightSetpoint { position: [0.06, -0.06, 0.18], yaw: 0.0 },
            disturbance: DisturbanceProfile::AlternatingXy,
            pulse_start_step: 60,
            pulse_duration_steps: 50,
            disturbance_fraction_of_weight: 0.10,
        },
        FlightScenario {
            id: "validation_sustained_gust_10pct",
            family: "sustained_gust",
            split: ScenarioSplit::Validation,
            setpoint: FlightSetpoint { position: [0.05, 0.02, 0.18], yaw: 0.0 },
            disturbance: DisturbanceProfile::SineGustXy,
            pulse_start_step: 30,
            pulse_duration_steps: 0,
            disturbance_fraction_of_weight: 0.10,
        },
        FlightScenario {
            id: "test_far_diagonal_15pct",
            family: "heldout_combined_target",
            split: ScenarioSplit::Test,
            setpoint: FlightSetpoint { position: [0.12, 0.09, 0.26], yaw: 0.0 },
            disturbance: DisturbanceProfile::AlternatingXy,
            pulse_start_step: 70,
            pulse_duration_steps: 60,
            disturbance_fraction_of_weight: 0.15,
        },
        FlightScenario {
            id: "test_altitude_lateral_15pct",
            family: "heldout_altitude_lateral",
            split: ScenarioSplit::Test,
            setpoint: FlightSetpoint { position: [-0.10, 0.07, 0.28], yaw: 0.0 },
            disturbance: DisturbanceProfile::SineGustXy,
            pulse_start_step: 40,
            pulse_duration_steps: 0,
            disturbance_fraction_of_weight: 0.15,
        },
        FlightScenario {
            id: "test_vertical_impulse_20pct",
            family: "heldout_vertical_disturbance",
            split: ScenarioSplit::Test,
            setpoint: FlightSetpoint { position: [0.0, -0.06, 0.22], yaw: 0.0 },
            disturbance: DisturbanceProfile::ZImpulse,
            pulse_start_step: 80,
            pulse_duration_steps: 30,
            disturbance_fraction_of_weight: 0.20,
        },
    ]
}

pub fn scenarios_for_split(split: ScenarioSplit) -> Vec<FlightScenario> {
    scenario_catalogue().into_iter().filter(|case| case.split == split).collect()
}

pub fn training_setpoints() -> Vec<FlightSetpoint> {
    scenarios_for_split(ScenarioSplit::Train).into_iter().map(|case| case.setpoint).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_has_stable_ids_and_disjoint_split_membership() {
        let first = scenario_catalogue();
        let second = scenario_catalogue();
        assert_eq!(first.iter().map(|c| c.id).collect::<Vec<_>>(), second.iter().map(|c| c.id).collect::<Vec<_>>());
        for split in [ScenarioSplit::Train, ScenarioSplit::Validation, ScenarioSplit::Test] {
            assert!(!scenarios_for_split(split).is_empty());
        }
        assert_eq!(scenarios_for_split(ScenarioSplit::Train).len(), 4);
        assert_eq!(scenarios_for_split(ScenarioSplit::Validation).len(), 2);
        assert_eq!(scenarios_for_split(ScenarioSplit::Test).len(), 3);
        let ids = first.iter().map(|c| c.id).collect::<std::collections::HashSet<_>>();
        assert_eq!(ids.len(), first.len(), "scenario identifiers must be unique");
    }

    #[test]
    fn zero_disturbance_is_exactly_zero_and_seeded_pulses_are_reproducible() {
        let train_hover = scenario_catalogue().into_iter().find(|c| c.id == "train_hover_10cm").unwrap();
        assert_eq!(train_hover.force_at(1, 75, 0.027), [0.0; 3]);

        let pulse = scenario_catalogue().into_iter().find(|c| c.id == "test_vertical_impulse_20pct").unwrap();
        assert_eq!(pulse.force_at(17, 85, 0.027), pulse.force_at(17, 85, 0.027));
        assert_eq!(pulse.force_at(17, 85, 0.027), [0.0, 0.0, 0.027 * 9.81 * 0.20]);
    }

    #[test]
    fn stress_force_scales_with_vehicle_weight() {
        let case = scenario_catalogue().into_iter().find(|c| c.id == "test_far_diagonal_15pct").unwrap();
        let force = case.force_at(0, 70, 0.027);
        assert!((force[0] - 0.15 * 0.027 * 9.81).abs() < 1e-12);
        assert_eq!(case.force_at(0, 70, 0.054)[0], force[0] * 2.0);
    }

    #[test]
    fn non_finite_or_non_positive_mass_fails_closed_to_no_external_force() {
        let case = scenario_catalogue().into_iter().find(|c| c.id == "test_vertical_impulse_20pct").unwrap();
        assert_eq!(case.force_at(0, 100, f64::NAN), [0.0; 3]);
        assert_eq!(case.force_at(0, 100, 0.0), [0.0; 3]);
        assert_eq!(case.force_at(0, 100, -1.0), [0.0; 3]);
    }
}
