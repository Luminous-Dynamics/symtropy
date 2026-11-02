// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Quasi-steady-state duty-cycle integration for generator comparisons.
//!
//! Each segment is treated as a steady operating point held for its declared
//! duration. This does not model start-up, ramps, transient combustion, thermal
//! inertia, aftertreatment warm-up, or dynamic governor response.

use core::fmt;

use crate::{
    evaluate, FuelInput, GeneratorModel, ModelError, EngineModel, Watts,
};

/// Positive duration of one quasi-steady-state segment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DurationSeconds(f64);

impl DurationSeconds {
    pub fn new(value: f64) -> Result<Self, DutyCycleError> {
        if !value.is_finite() || value <= 0.0 {
            return Err(DutyCycleError::InvalidDuration { value });
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Non-negative integrated energy in joules.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnergyJoules(f64);

impl EnergyJoules {
    fn new(value: f64, quantity: &'static str) -> Result<Self, DutyCycleError> {
        if !value.is_finite() || value < 0.0 {
            return Err(DutyCycleError::InvalidIntegratedValue { quantity, value });
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Integrated fuel mass in kilograms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FuelMassKg(f64);

impl FuelMassKg {
    fn new(value: f64) -> Result<Self, DutyCycleError> {
        if !value.is_finite() || value < 0.0 {
            return Err(DutyCycleError::InvalidIntegratedValue {
                quantity: "fuel mass",
                value,
            });
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> f64 {
        self.0
    }
}

/// One steady-state engine-generator point held for a positive duration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DutyCycleSegment {
    duration: DurationSeconds,
    fuel: FuelInput,
    engine: EngineModel,
    generator: GeneratorModel,
}

impl DutyCycleSegment {
    pub const fn new(
        duration: DurationSeconds,
        fuel: FuelInput,
        engine: EngineModel,
        generator: GeneratorModel,
    ) -> Self {
        Self {
            duration,
            fuel,
            engine,
            generator,
        }
    }

    pub const fn duration(self) -> DurationSeconds {
        self.duration
    }
}

/// A non-empty ordered sequence of quasi-steady-state segments.
#[derive(Debug, Clone, PartialEq)]
pub struct DutyCycle {
    segments: Vec<DutyCycleSegment>,
}

impl DutyCycle {
    pub fn new(segments: Vec<DutyCycleSegment>) -> Result<Self, DutyCycleError> {
        if segments.is_empty() {
            return Err(DutyCycleError::EmptyDutyCycle);
        }
        Ok(Self { segments })
    }

    pub fn segments(&self) -> &[DutyCycleSegment] {
        &self.segments
    }

    /// Integrate the declared steady-state points over time.
    ///
    /// Segment order is retained in the input contract, but this version sums
    /// energies only. It does not infer transition behavior between segments.
    pub fn evaluate(&self) -> Result<DutyCycleResult, DutyCycleError> {
        let mut total_duration_s = 0.0;
        let mut total_fuel_mass_kg = 0.0;
        let mut chemical_input_j = 0.0;
        let mut brake_work_j = 0.0;
        let mut engine_rejected_j = 0.0;
        let mut gross_electrical_j = 0.0;
        let mut generator_loss_j = 0.0;
        let mut auxiliary_energy_j = 0.0;
        let mut net_electrical_j = 0.0;

        for segment in &self.segments {
            let duration_s = segment.duration.get();
            let point = evaluate(segment.fuel, segment.engine, segment.generator)?;

            total_duration_s += duration_s;
            total_fuel_mass_kg += segment.fuel.mass_flow().get() * duration_s;

            chemical_input_j += point.balance.chemical_input_power.get() * duration_s;
            brake_work_j += point.engine.brake_power.get() * duration_s;
            engine_rejected_j += point.engine.rejected_power.get() * duration_s;
            gross_electrical_j += point.generator.gross_electrical_power.get() * duration_s;
            generator_loss_j += point.generator.conversion_loss_power.get() * duration_s;
            auxiliary_energy_j += point.generator.auxiliary_load.get() * duration_s;
            net_electrical_j += point.generator.net_electrical_power.get() * duration_s;
        }

        if !total_duration_s.is_finite() {
            return Err(DutyCycleError::InvalidIntegratedValue {
                quantity: "total duration",
                value: total_duration_s,
            });
        }

        Ok(DutyCycleResult {
            segment_count: self.segments.len(),
            duration: DurationSeconds(total_duration_s),
            fuel_mass: FuelMassKg::new(total_fuel_mass_kg)?,
            chemical_input: EnergyJoules::new(chemical_input_j, "chemical input energy")?,
            brake_work: EnergyJoules::new(brake_work_j, "brake work")?,
            engine_rejected: EnergyJoules::new(engine_rejected_j, "engine rejected energy")?,
            gross_electrical: EnergyJoules::new(
                gross_electrical_j,
                "gross electrical energy",
            )?,
            generator_loss: EnergyJoules::new(generator_loss_j, "generator loss energy")?,
            auxiliary_energy: EnergyJoules::new(auxiliary_energy_j, "auxiliary energy")?,
            net_electrical: EnergyJoules::new(
                net_electrical_j,
                "net electrical energy",
            )?,
        })
    }
}

/// Integrated result for one duty cycle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DutyCycleResult {
    segment_count: usize,
    duration: DurationSeconds,
    fuel_mass: FuelMassKg,
    chemical_input: EnergyJoules,
    brake_work: EnergyJoules,
    engine_rejected: EnergyJoules,
    gross_electrical: EnergyJoules,
    generator_loss: EnergyJoules,
    auxiliary_energy: EnergyJoules,
    net_electrical: EnergyJoules,
}

impl DutyCycleResult {
    pub const fn segment_count(self) -> usize {
        self.segment_count
    }

    pub const fn duration(self) -> DurationSeconds {
        self.duration
    }

    pub const fn fuel_mass(self) -> FuelMassKg {
        self.fuel_mass
    }

    pub const fn chemical_input(self) -> EnergyJoules {
        self.chemical_input
    }

    pub const fn brake_work(self) -> EnergyJoules {
        self.brake_work
    }

    pub const fn engine_rejected(self) -> EnergyJoules {
        self.engine_rejected
    }

    pub const fn gross_electrical(self) -> EnergyJoules {
        self.gross_electrical
    }

    pub const fn generator_loss(self) -> EnergyJoules {
        self.generator_loss
    }

    pub const fn auxiliary_energy(self) -> EnergyJoules {
        self.auxiliary_energy
    }

    pub const fn net_electrical(self) -> EnergyJoules {
        self.net_electrical
    }

    /// Integrated net electrical efficiency over total chemical fuel energy.
    pub fn net_electrical_efficiency(self) -> Option<f64> {
        let input = self.chemical_input.get();
        if input == 0.0 {
            None
        } else {
            Some(self.net_electrical.get() / input)
        }
    }

    /// Residual of the integrated first-law bookkeeping identity.
    pub fn energy_balance_residual_j(self) -> f64 {
        self.chemical_input.get()
            - self.engine_rejected.get()
            - self.generator_loss.get()
            - self.auxiliary_energy.get()
            - self.net_electrical.get()
    }

    /// Test integrated first-law closure using explicit absolute and relative
    /// tolerances. These are numerical tolerances, not measurement uncertainty.
    pub fn energy_balance_is_closed(
        self,
        absolute_tolerance_j: f64,
        relative_tolerance: f64,
    ) -> Result<bool, DutyCycleError> {
        if !absolute_tolerance_j.is_finite()
            || absolute_tolerance_j < 0.0
            || !relative_tolerance.is_finite()
            || relative_tolerance < 0.0
        {
            return Err(DutyCycleError::InvalidTolerance {
                absolute: absolute_tolerance_j,
                relative: relative_tolerance,
            });
        }

        let allowed = absolute_tolerance_j.max(relative_tolerance * self.chemical_input.get());
        Ok(self.energy_balance_residual_j().abs() <= allowed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DutyCycleError {
    EmptyDutyCycle,
    InvalidDuration { value: f64 },
    InvalidIntegratedValue {
        quantity: &'static str,
        value: f64,
    },
    InvalidTolerance {
        absolute: f64,
        relative: f64,
    },
    Model(ModelError),
}

impl From<ModelError> for DutyCycleError {
    fn from(value: ModelError) -> Self {
        Self::Model(value)
    }
}

impl fmt::Display for DutyCycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDutyCycle => write!(f, "duty cycle must contain at least one segment"),
            Self::InvalidDuration { value } => {
                write!(f, "segment duration must be finite and positive; got {value}")
            }
            Self::InvalidIntegratedValue { quantity, value } => {
                write!(f, "integrated {quantity} must be finite and non-negative; got {value}")
            }
            Self::InvalidTolerance { absolute, relative } => {
                write!(
                    f,
                    "balance tolerances must be finite and non-negative; got absolute={absolute} J, relative={relative}"
                )
            }
            Self::Model(error) => write!(f, "steady-state model error: {error}"),
        }
    }
}

impl std::error::Error for DutyCycleError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Efficiency, LowerHeatingValueJPerKg, MassFlowKgPerSecond};

    fn segment(
        duration_s: f64,
        mass_flow_kg_s: f64,
        engine_efficiency: f64,
        generator_efficiency: f64,
        auxiliary_w: f64,
    ) -> DutyCycleSegment {
        let fuel = FuelInput::new(
            MassFlowKgPerSecond::new(mass_flow_kg_s).unwrap(),
            LowerHeatingValueJPerKg::new(43_000_000.0).unwrap(),
        );
        let engine = EngineModel::new(Efficiency::new(engine_efficiency).unwrap());
        let generator = GeneratorModel::new(
            Efficiency::new(generator_efficiency).unwrap(),
            Watts::new(auxiliary_w).unwrap(),
        );

        DutyCycleSegment::new(
            DurationSeconds::new(duration_s).unwrap(),
            fuel,
            engine,
            generator,
        )
    }

    #[test]
    fn integrates_fuel_mass_and_net_electrical_energy() {
        let cycle = DutyCycle::new(vec![
            segment(60.0, 0.01, 0.45, 0.96, 5_760.0),
            segment(30.0, 0.005, 0.40, 0.95, 1_700.0),
        ])
        .unwrap();

        let result = cycle.evaluate().unwrap();

        assert_eq!(result.segment_count(), 2);
        assert!((result.duration().get() - 90.0).abs() < 1e-12);
        assert!((result.fuel_mass().get() - 0.75).abs() < 1e-12);
        assert!((result.chemical_input().get() - 38_700_000.0).abs() < 1e-6);
        assert!((result.net_electrical().get() - 13_200_000.0).abs() < 1e-6);
    }

    #[test]
    fn integrated_energy_balance_closes() {
        let cycle = DutyCycle::new(vec![
            segment(60.0, 0.01, 0.45, 0.96, 5_760.0),
            segment(30.0, 0.005, 0.40, 0.95, 1_700.0),
        ])
        .unwrap();

        let result = cycle.evaluate().unwrap();

        assert!(result.energy_balance_is_closed(1e-6, 1e-12).unwrap());
        assert!(result.energy_balance_residual_j().abs() < 1e-6);
    }

    #[test]
    fn calculates_energy_weighted_not_arithmetic_mean_efficiency() {
        let cycle = DutyCycle::new(vec![
            segment(60.0, 0.01, 0.45, 0.96, 5_760.0),
            segment(30.0, 0.005, 0.40, 0.95, 1_700.0),
        ])
        .unwrap();

        let result = cycle.evaluate().unwrap();
        let expected = 13_200_000.0 / 38_700_000.0;

        assert!((result.net_electrical_efficiency().unwrap() - expected).abs() < 1e-12);
    }

    #[test]
    fn rejects_empty_cycle_and_invalid_duration() {
        assert!(matches!(
            DutyCycle::new(Vec::new()),
            Err(DutyCycleError::EmptyDutyCycle)
        ));
        assert!(DurationSeconds::new(0.0).is_err());
        assert!(DurationSeconds::new(f64::NAN).is_err());
        assert!(DurationSeconds::new(-1.0).is_err());
    }

    #[test]
    fn rejects_cycle_that_overflows_integrated_energy() {
        let cycle = DutyCycle::new(vec![segment(
            f64::MAX,
            1.0,
            0.45,
            0.96,
            0.0,
        )])
        .unwrap();

        assert!(matches!(
            cycle.evaluate(),
            Err(DutyCycleError::InvalidIntegratedValue { .. })
        ));
    }

    #[test]
    fn rejects_negative_or_non_finite_balance_tolerances() {
        let cycle = DutyCycle::new(vec![segment(
            60.0,
            0.01,
            0.45,
            0.96,
            5_760.0,
        )])
        .unwrap();
        let result = cycle.evaluate().unwrap();

        assert!(result.energy_balance_is_closed(-1.0, 0.0).is_err());
        assert!(result.energy_balance_is_closed(0.0, f64::NAN).is_err());
    }
}
