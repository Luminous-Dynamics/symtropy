//! Explicit configuration for the integration benchmark's simulated plant.
//!
//! This module keeps simulator assumptions visible in experiment receipts. The optional
//! actuator lag is an internal stress parameter with a first-order command filter; it is
//! not a calibrated rotor/motor model and must not be presented as real-flight fidelity.

use serde_json::{json, Value};

use crate::symtropy_flight::SymtropyFlightSimulator;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlantConfiguration {
    /// None means direct command-level actuation. Some(tau) applies an exponential
    /// first-order lag to thrust and each body-moment channel.
    pub actuator_lag_time_constant_s: Option<f64>,
}

impl Default for PlantConfiguration {
    fn default() -> Self {
        Self {
            actuator_lag_time_constant_s: None,
        }
    }
}

impl PlantConfiguration {
    /// Read an explicit environment variable. An unset value or "0" preserves direct
    /// actuation; a positive finite value configures a lag in seconds.
    pub fn from_env(name: &str) -> Result<Self, String> {
        let raw = match std::env::var(name) {
            Ok(value) => value,
            Err(std::env::VarError::NotPresent) => return Ok(Self::default()),
            Err(error) => return Err(format!("could not read {name}: {error}")),
        };
        let value = raw
            .trim()
            .parse::<f64>()
            .map_err(|error| format!("{name} must be a number of seconds: {error}"))?;
        if !value.is_finite() || value < 0.0 {
            return Err(format!("{name} must be finite and non-negative"));
        }
        Ok(Self {
            actuator_lag_time_constant_s: if value == 0.0 { None } else { Some(value) },
        })
    }

    pub fn create_simulator(&self) -> Result<SymtropyFlightSimulator, String> {
        let mut simulator = SymtropyFlightSimulator::new();
        simulator
            .set_actuator_lag_time_constant_s(self.actuator_lag_time_constant_s)
            .map_err(str::to_owned)?;
        Ok(simulator)
    }

    pub fn to_json(self) -> Value {
        json!({
            "simulator": "SymtropyFlightSimulator_spherical_body_v0.1",
            "actuator_model": if self.actuator_lag_time_constant_s.is_some() {
                "first_order_exponential_command_filter"
            } else {
                "direct_command_application"
            },
            "actuator_lag_time_constant_s": self.actuator_lag_time_constant_s,
            "initial_filter_state": "hover_thrust_zero_body_moments_on_each_episode_reset",
            "claim_ceiling": "internal_actuator_lag_stress_configuration_not_calibrated_real_flight_model"
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_and_unset_style_configuration_means_direct_actuation() {
        let config = PlantConfiguration::default();
        assert_eq!(config.actuator_lag_time_constant_s, None);
        assert_eq!(
            config.to_json()["actuator_model"].as_str(),
            Some("direct_command_application")
        );
    }

    #[test]
    fn serialization_keeps_lag_value_and_claim_ceiling_explicit() {
        let config = PlantConfiguration {
            actuator_lag_time_constant_s: Some(0.020),
        };
        assert_eq!(
            config.to_json()["actuator_lag_time_constant_s"].as_f64(),
            Some(0.020)
        );
        assert_eq!(
            config.to_json()["claim_ceiling"].as_str(),
            Some("internal_actuator_lag_stress_configuration_not_calibrated_real_flight_model")
        );
    }
}
