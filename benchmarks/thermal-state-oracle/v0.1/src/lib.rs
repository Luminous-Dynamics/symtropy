//! Exact-step, single-node thermal RC reference model.
//!
//! This is a falsifiable analytic baseline for thermal mission replay. It is
//! not a fitted motor model, winding-temperature estimator, safety limiter,
//! or physical qualification.

use std::collections::HashSet;
use std::error::Error;
use std::fmt::{Display, Formatter};

/// Errors returned when a model or mission violates the reference contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThermalError {
    NonFiniteInput,
    NonPositiveHeatCapacity,
    NegativeConductance,
    NegativeHeatLossPower,
    NegativeDuration,
    EmptySegmentId,
    DuplicateSegmentId,
    NonFiniteResult,
}

impl Display for ThermalError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteInput => write!(formatter, "input values must be finite"),
            Self::NonPositiveHeatCapacity => {
                write!(formatter, "thermal heat capacity must be positive")
            }
            Self::NegativeConductance => {
                write!(formatter, "thermal conductance cannot be negative")
            }
            Self::NegativeHeatLossPower => {
                write!(formatter, "heat-loss power cannot be negative")
            }
            Self::NegativeDuration => write!(formatter, "duration cannot be negative"),
            Self::EmptySegmentId => write!(formatter, "mission segment ID cannot be empty"),
            Self::DuplicateSegmentId => write!(formatter, "mission segment IDs must be unique"),
            Self::NonFiniteResult => write!(formatter, "model result is not finite"),
        }
    }
}

impl Error for ThermalError {}

/// One lumped thermal node connected to a declared ambient boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThermalRcModel {
    heat_capacity_j_per_k: f64,
    conductance_w_per_k: f64,
}

impl ThermalRcModel {
    /// Construct a model with positive heat capacity and nonnegative conductance.
    pub fn new(
        heat_capacity_j_per_k: f64,
        conductance_w_per_k: f64,
    ) -> Result<Self, ThermalError> {
        if !heat_capacity_j_per_k.is_finite() || !conductance_w_per_k.is_finite() {
            return Err(ThermalError::NonFiniteInput);
        }
        if heat_capacity_j_per_k <= 0.0 {
            return Err(ThermalError::NonPositiveHeatCapacity);
        }
        if conductance_w_per_k < 0.0 {
            return Err(ThermalError::NegativeConductance);
        }
        Ok(Self {
            heat_capacity_j_per_k,
            conductance_w_per_k,
        })
    }

    /// Heat capacity in joules per kelvin.
    pub fn heat_capacity_j_per_k(&self) -> f64 {
        self.heat_capacity_j_per_k
    }

    /// Conductance to the declared ambient boundary in watts per kelvin.
    pub fn conductance_w_per_k(&self) -> f64 {
        self.conductance_w_per_k
    }

    /// Advance the node under constant heat-loss power for one interval.
    ///
    /// Temperatures are in degrees Celsius; temperature differences are
    /// numerically equal to kelvin differences. heat_loss_w must be a measured
    /// or independently qualified heat input.
    pub fn advance(
        &self,
        initial_temperature_c: f64,
        ambient_temperature_c: f64,
        heat_loss_w: f64,
        duration_s: f64,
    ) -> Result<f64, ThermalError> {
        validate_inputs(
            initial_temperature_c,
            ambient_temperature_c,
            heat_loss_w,
            duration_s,
        )?;

        if duration_s == 0.0 {
            return Ok(initial_temperature_c);
        }

        let capacity = self.heat_capacity_j_per_k;
        let conductance = self.conductance_w_per_k;

        // Adiabatic boundary: C dT/dt = P.
        if conductance == 0.0 {
            return finite_result(
                initial_temperature_c + (heat_loss_w / capacity) * duration_s,
            );
        }

        let exponent = (conductance / capacity) * duration_s;
        let decay = (-exponent).exp();

        // Use expm1 near zero to avoid cancellation and P/G overflow.
        let heating = if exponent.abs() < 0.5 {
            let response = if exponent == 0.0 {
                1.0
            } else {
                -(-exponent).exp_m1() / exponent
            };
            (heat_loss_w / capacity) * duration_s * response
        } else {
            (heat_loss_w / conductance) * -(-exponent).exp_m1()
        };

        finite_result(
            ambient_temperature_c
                + (initial_temperature_c - ambient_temperature_c) * decay
                + heating,
        )
    }

    /// Return modelled time to first reach a temperature threshold.
    ///
    /// Ok(None) means the constant-input equilibrium does not exceed the
    /// threshold. This does not model driver protection, derating or delay.
    pub fn time_to_limit(
        &self,
        initial_temperature_c: f64,
        ambient_temperature_c: f64,
        heat_loss_w: f64,
        limit_temperature_c: f64,
    ) -> Result<Option<f64>, ThermalError> {
        validate_inputs(
            initial_temperature_c,
            ambient_temperature_c,
            heat_loss_w,
            0.0,
        )?;
        if !limit_temperature_c.is_finite() {
            return Err(ThermalError::NonFiniteInput);
        }
        if initial_temperature_c >= limit_temperature_c {
            return Ok(Some(0.0));
        }

        let capacity = self.heat_capacity_j_per_k;
        let conductance = self.conductance_w_per_k;

        if conductance == 0.0 {
            if heat_loss_w == 0.0 {
                return Ok(None);
            }
            let crossing =
                (limit_temperature_c - initial_temperature_c) / (heat_loss_w / capacity);
            return finite_result(crossing).map(Some);
        }

        let steady_temperature_c = ambient_temperature_c + heat_loss_w / conductance;
        if !steady_temperature_c.is_finite() {
            return Err(ThermalError::NonFiniteResult);
        }
        if steady_temperature_c <= limit_temperature_c {
            return Ok(None);
        }

        let ratio = (steady_temperature_c - initial_temperature_c)
            / (steady_temperature_c - limit_temperature_c);
        if !ratio.is_finite() || ratio <= 1.0 {
            return Err(ThermalError::NonFiniteResult);
        }

        let crossing = (capacity / conductance) * ratio.ln();
        finite_result(crossing).map(Some)
    }

    /// Replay constant-power load/cooldown segments without resetting state.
    ///
    /// Replay continues after a modelled limit crossing so the full
    /// mathematical trajectory is retained. No protective action is simulated.
    pub fn replay(
        &self,
        mission: &ThermalMission,
    ) -> Result<ThermalMissionResult, ThermalError> {
        if !mission.initial_temperature_c.is_finite()
            || !mission.ambient_temperature_c.is_finite()
            || !mission.limit_temperature_c.is_finite()
        {
            return Err(ThermalError::NonFiniteInput);
        }

        let mut seen_ids = HashSet::with_capacity(mission.segments.len());
        for segment in &mission.segments {
            if segment.id.trim().is_empty() {
                return Err(ThermalError::EmptySegmentId);
            }
            if !seen_ids.insert(segment.id.as_str()) {
                return Err(ThermalError::DuplicateSegmentId);
            }
            validate_inputs(
                mission.initial_temperature_c,
                mission.ambient_temperature_c,
                segment.heat_loss_w,
                segment.duration_s,
            )?;
        }

        let mut temperature_c = mission.initial_temperature_c;
        let mut peak_temperature_c = temperature_c;
        let mut elapsed_s = 0.0;
        let mut first_limit_crossing_s = if temperature_c >= mission.limit_temperature_c {
            Some(0.0)
        } else {
            None
        };

        for segment in &mission.segments {
            if first_limit_crossing_s.is_none() {
                if let Some(crossing_s) = self.time_to_limit(
                    temperature_c,
                    mission.ambient_temperature_c,
                    segment.heat_loss_w,
                    mission.limit_temperature_c,
                )? {
                    if crossing_s <= segment.duration_s {
                        first_limit_crossing_s = Some(elapsed_s + crossing_s);
                    }
                }
            }

            temperature_c = self.advance(
                temperature_c,
                mission.ambient_temperature_c,
                segment.heat_loss_w,
                segment.duration_s,
            )?;
            peak_temperature_c = peak_temperature_c.max(temperature_c);
            elapsed_s += segment.duration_s;
            if !elapsed_s.is_finite() {
                return Err(ThermalError::NonFiniteResult);
            }
        }

        Ok(ThermalMissionResult {
            final_temperature_c: temperature_c,
            peak_temperature_c,
            total_duration_s: elapsed_s,
            first_limit_crossing_s,
            remained_below_limit: first_limit_crossing_s.is_none(),
        })
    }
}

/// One piecewise-constant mission interval. Zero power represents passive
/// cooldown toward ambient when conductance is positive.
#[derive(Clone, Debug, PartialEq)]
pub struct ThermalSegment {
    pub id: String,
    pub duration_s: f64,
    pub heat_loss_w: f64,
}

impl ThermalSegment {
    pub fn new(
        id: impl Into<String>,
        duration_s: f64,
        heat_loss_w: f64,
    ) -> Result<Self, ThermalError> {
        let segment = Self {
            id: id.into(),
            duration_s,
            heat_loss_w,
        };
        if segment.id.trim().is_empty() {
            return Err(ThermalError::EmptySegmentId);
        }
        validate_inputs(0.0, 0.0, heat_loss_w, duration_s)?;
        Ok(segment)
    }
}

/// Initial state, ambient boundary, comparison threshold and ordered segments.
#[derive(Clone, Debug, PartialEq)]
pub struct ThermalMission {
    pub initial_temperature_c: f64,
    pub ambient_temperature_c: f64,
    pub limit_temperature_c: f64,
    pub segments: Vec<ThermalSegment>,
}

/// Modelled summary of one complete mission replay.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThermalMissionResult {
    pub final_temperature_c: f64,
    pub peak_temperature_c: f64,
    pub total_duration_s: f64,
    pub first_limit_crossing_s: Option<f64>,
    pub remained_below_limit: bool,
}

fn validate_inputs(
    initial_temperature_c: f64,
    ambient_temperature_c: f64,
    heat_loss_w: f64,
    duration_s: f64,
) -> Result<(), ThermalError> {
    if !initial_temperature_c.is_finite()
        || !ambient_temperature_c.is_finite()
        || !heat_loss_w.is_finite()
        || !duration_s.is_finite()
    {
        return Err(ThermalError::NonFiniteInput);
    }
    if heat_loss_w < 0.0 {
        return Err(ThermalError::NegativeHeatLossPower);
    }
    if duration_s < 0.0 {
        return Err(ThermalError::NegativeDuration);
    }
    Ok(())
}

fn finite_result(value: f64) -> Result<f64, ThermalError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ThermalError::NonFiniteResult)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual:.15}, expected {expected:.15}, tolerance {tolerance}"
        );
    }

    fn model() -> ThermalRcModel {
        ThermalRcModel::new(100.0, 10.0).expect("valid model")
    }

    #[test]
    fn exact_zero_power_cooling_matches_closed_form() {
        let result = model().advance(30.0, 20.0, 0.0, 10.0).expect("valid step");
        close(result, 20.0 + 10.0 * (-1.0_f64).exp(), 1e-12);
    }

    #[test]
    fn constant_heating_approaches_equilibrium_exactly() {
        let duration = 10.0 * std::f64::consts::LN_2;
        let result = model()
            .advance(20.0, 20.0, 50.0, duration)
            .expect("valid step");
        close(result, 22.5, 1e-12);
    }

    #[test]
    fn zero_conductance_uses_adiabatic_limit() {
        let isolated = ThermalRcModel::new(100.0, 0.0).expect("valid model");
        let result = isolated.advance(20.0, 20.0, 20.0, 5.0).expect("valid step");
        close(result, 21.0, 1e-12);
    }

    #[test]
    fn split_and_unsplit_constant_power_steps_are_partition_invariant() {
        let full = model().advance(21.0, 19.0, 37.0, 12.0).expect("valid step");
        let first = model().advance(21.0, 19.0, 37.0, 5.0).expect("valid step");
        let split = model().advance(first, 19.0, 37.0, 7.0).expect("valid step");
        close(full, split, 1e-12);
    }

    #[test]
    fn hot_start_reaches_same_threshold_earlier_than_cold_start() {
        let cold = model()
            .time_to_limit(20.0, 20.0, 60.0, 25.0)
            .expect("valid query")
            .expect("finite crossing");
        let hot = model()
            .time_to_limit(23.0, 20.0, 60.0, 25.0)
            .expect("valid query")
            .expect("finite crossing");
        close(cold, 10.0 * 6.0_f64.ln(), 1e-12);
        close(hot, 10.0 * 3.0_f64.ln(), 1e-12);
        assert!(hot < cold);
    }

    #[test]
    fn repeated_load_and_partial_cooldown_preserve_thermal_memory() {
        let mission = ThermalMission {
            initial_temperature_c: 20.0,
            ambient_temperature_c: 20.0,
            limit_temperature_c: 25.0,
            segments: vec![
                ThermalSegment::new("load-1", 10.0, 60.0).expect("segment"),
                ThermalSegment::new("cool-1", 2.0, 0.0).expect("segment"),
                ThermalSegment::new("load-2", 10.0, 60.0).expect("segment"),
                ThermalSegment::new("cool-2", 2.0, 0.0).expect("segment"),
                ThermalSegment::new("load-3", 10.0, 60.0).expect("segment"),
            ],
        };

        let result = model().replay(&mission).expect("valid mission");
        assert_eq!(result.total_duration_s, 34.0);
        assert!(result.first_limit_crossing_s.is_some());
        assert!(!result.remained_below_limit);
        assert!(result.peak_temperature_c >= 25.0);
    }

    #[test]
    fn no_finite_crossing_when_equilibrium_does_not_exceed_limit() {
        let crossing = model()
            .time_to_limit(20.0, 20.0, 40.0, 25.0)
            .expect("valid query");
        assert_eq!(crossing, None);
    }

    #[test]
    fn initial_temperature_at_limit_is_an_immediate_crossing() {
        let crossing = model()
            .time_to_limit(25.0, 20.0, 0.0, 25.0)
            .expect("valid query");
        assert_eq!(crossing, Some(0.0));
    }

    #[test]
    fn greater_nonnegative_heat_input_does_not_lower_final_temperature() {
        let lower = model().advance(20.0, 20.0, 30.0, 8.0).expect("valid step");
        let higher = model().advance(20.0, 20.0, 60.0, 8.0).expect("valid step");
        assert!(higher > lower);
    }

    #[test]
    fn rejects_invalid_model_parameters_and_inputs() {
        assert_eq!(
            ThermalRcModel::new(0.0, 1.0),
            Err(ThermalError::NonPositiveHeatCapacity)
        );
        assert_eq!(
            ThermalRcModel::new(1.0, -1.0),
            Err(ThermalError::NegativeConductance)
        );
        assert_eq!(
            model().advance(20.0, 20.0, -1.0, 1.0),
            Err(ThermalError::NegativeHeatLossPower)
        );
        assert_eq!(
            model().advance(20.0, 20.0, 1.0, -1.0),
            Err(ThermalError::NegativeDuration)
        );
        assert_eq!(
            model().advance(f64::NAN, 20.0, 1.0, 1.0),
            Err(ThermalError::NonFiniteInput)
        );
    }

    #[test]
    fn rejects_duplicate_mission_segment_ids() {
        let mission = ThermalMission {
            initial_temperature_c: 20.0,
            ambient_temperature_c: 20.0,
            limit_temperature_c: 80.0,
            segments: vec![
                ThermalSegment::new("same", 1.0, 1.0).expect("segment"),
                ThermalSegment::new("same", 1.0, 0.0).expect("segment"),
            ],
        };
        assert_eq!(
            model().replay(&mission),
            Err(ThermalError::DuplicateSegmentId)
        );
    }
}
