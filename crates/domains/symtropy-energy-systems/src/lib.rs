#![forbid(unsafe_code)]

//! Deterministic 0D energy-conversion bookkeeping primitives.
//!
//! This crate is deliberately not a combustion, gas-exchange, CFD, emissions,
//! fatigue, or engine-control solver. It converts explicitly supplied fuel
//! flow, lower heating value, and conversion efficiencies into a transparent
//! steady-state energy balance. Unknown physical inputs have no defaults.

use core::fmt;
use serde::Serialize;

mod batch_evaluation;
mod duty_cycle;
mod fuel;

pub use batch_evaluation::{FuelBatchEvaluation, evaluate_fuel_batch};
pub use duty_cycle::{
    DurationSeconds, DutyCycle, DutyCycleError, DutyCycleResult, DutyCycleSegment, EnergyJoules,
    FuelMassKg,
};
pub use fuel::{
    DensityKgPerCubicMeter, EvidenceClass, EvidenceReference, FuelBatch, FuelFamily, FuelProperty,
    FuelRecordError,
};

/// Errors returned when an input is non-finite, physically out of range, or
/// violates the declared energy-conversion boundary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelError {
    InvalidQuantity {
        name: &'static str,
        value: f64,
    },
    InvalidEfficiency {
        value: f64,
    },
    AuxiliaryLoadExceedsGrossOutput {
        auxiliary_w: f64,
        gross_output_w: f64,
    },
    InvalidTolerance {
        absolute_w: f64,
        relative: f64,
    },
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQuantity { name, value } => {
                write!(f, "{name} must be finite and non-negative; got {value}")
            }
            Self::InvalidEfficiency { value } => {
                write!(f, "efficiency must be finite and in [0, 1]; got {value}")
            }
            Self::AuxiliaryLoadExceedsGrossOutput {
                auxiliary_w,
                gross_output_w,
            } => write!(
                f,
                "auxiliary load ({auxiliary_w} W) exceeds gross electrical output ({gross_output_w} W)"
            ),
            Self::InvalidTolerance {
                absolute_w,
                relative,
            } => write!(
                f,
                "balance tolerances must be finite and non-negative; got absolute={absolute_w} W, relative={relative}"
            ),
        }
    }
}

impl std::error::Error for ModelError {}

/// Non-negative power in watts.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Watts(f64);

impl Watts {
    pub fn new(value: f64) -> Result<Self, ModelError> {
        if !value.is_finite() || value < 0.0 {
            return Err(ModelError::InvalidQuantity {
                name: "power",
                value,
            });
        }
        Ok(Self(value))
    }

    pub const fn zero() -> Self {
        Self(0.0)
    }

    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Fuel mass flow in kilograms per second.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct MassFlowKgPerSecond(f64);

impl MassFlowKgPerSecond {
    pub fn new(value: f64) -> Result<Self, ModelError> {
        if !value.is_finite() || value < 0.0 {
            return Err(ModelError::InvalidQuantity {
                name: "fuel mass flow",
                value,
            });
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Fuel lower heating value in joules per kilogram.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct LowerHeatingValueJPerKg(f64);

impl LowerHeatingValueJPerKg {
    pub fn new(value: f64) -> Result<Self, ModelError> {
        if !value.is_finite() || value <= 0.0 {
            return Err(ModelError::InvalidQuantity {
                name: "lower heating value",
                value,
            });
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Dimensionless conversion efficiency bounded to the physical interval [0, 1].
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Efficiency(f64);

impl Efficiency {
    pub fn new(value: f64) -> Result<Self, ModelError> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(ModelError::InvalidEfficiency { value });
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Explicitly supplied fuel flow and lower heating value.
///
/// Fuel identity, batch provenance, composition, uncertainty and lifecycle
/// factors belong in the versioned evidence/input schema layered above this
/// minimal arithmetic primitive. They are never inferred here.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct FuelInput {
    mass_flow: MassFlowKgPerSecond,
    lower_heating_value: LowerHeatingValueJPerKg,
}

impl FuelInput {
    pub const fn mass_flow(self) -> MassFlowKgPerSecond {
        self.mass_flow
    }

    pub const fn lower_heating_value(self) -> LowerHeatingValueJPerKg {
        self.lower_heating_value
    }

    pub const fn new(
        mass_flow: MassFlowKgPerSecond,
        lower_heating_value: LowerHeatingValueJPerKg,
    ) -> Self {
        Self {
            mass_flow,
            lower_heating_value,
        }
    }

    /// Chemical input power = mass flow × lower heating value.
    pub fn chemical_input_power(self) -> Result<Watts, ModelError> {
        Watts::new(self.mass_flow.get() * self.lower_heating_value.get())
    }
}

/// A steady-state conversion factor from chemical input power to brake power.
///
/// This is a declared map point / scenario parameter, not a predictive engine
/// model and not a fuel-specific calibration.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct EngineModel {
    brake_efficiency: Efficiency,
}

impl EngineModel {
    pub const fn new(brake_efficiency: Efficiency) -> Self {
        Self { brake_efficiency }
    }

    pub fn convert(self, chemical_input_power: Watts) -> EngineResult {
        let brake_power = chemical_input_power.get() * self.brake_efficiency.get();
        EngineResult {
            chemical_input_power,
            brake_power: Watts(brake_power),
            rejected_power: Watts(chemical_input_power.get() - brake_power),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct EngineResult {
    pub chemical_input_power: Watts,
    pub brake_power: Watts,
    /// Residual engine losses (heat, friction, pumping and other unmodelled
    /// rejection); this is not a measured recoverable-heat stream.
    pub rejected_power: Watts,
}

/// A steady-state generator conversion factor and explicitly supplied
/// electrical auxiliary load.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct GeneratorModel {
    electrical_efficiency: Efficiency,
    auxiliary_load: Watts,
}

impl GeneratorModel {
    pub const fn new(electrical_efficiency: Efficiency, auxiliary_load: Watts) -> Self {
        Self {
            electrical_efficiency,
            auxiliary_load,
        }
    }

    pub fn convert(self, shaft_power: Watts) -> Result<GeneratorResult, ModelError> {
        let gross = shaft_power.get() * self.electrical_efficiency.get();
        if self.auxiliary_load.get() > gross {
            return Err(ModelError::AuxiliaryLoadExceedsGrossOutput {
                auxiliary_w: self.auxiliary_load.get(),
                gross_output_w: gross,
            });
        }

        Ok(GeneratorResult {
            shaft_power,
            gross_electrical_power: Watts(gross),
            conversion_loss_power: Watts(shaft_power.get() - gross),
            auxiliary_load: self.auxiliary_load,
            net_electrical_power: Watts(gross - self.auxiliary_load.get()),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct GeneratorResult {
    pub shaft_power: Watts,
    pub gross_electrical_power: Watts,
    pub conversion_loss_power: Watts,
    pub auxiliary_load: Watts,
    pub net_electrical_power: Watts,
}

/// A first-law bookkeeping ledger for the modeled conversion chain.
///
/// The balance is:
/// chemical input = engine rejection + generator loss + auxiliaries + net AC.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct EnergyBalance {
    pub chemical_input_power: Watts,
    pub engine_rejected_power: Watts,
    pub generator_conversion_loss: Watts,
    pub auxiliary_load: Watts,
    pub net_electrical_output: Watts,
}

impl EnergyBalance {
    /// Positive means the ledger has unexplained input; negative means it
    /// claims more accounted output/loss than the declared chemical input.
    pub fn residual_w(self) -> f64 {
        self.chemical_input_power.get()
            - self.engine_rejected_power.get()
            - self.generator_conversion_loss.get()
            - self.auxiliary_load.get()
            - self.net_electrical_output.get()
    }

    /// Check closure using caller-declared absolute and relative tolerances.
    pub fn is_closed(
        self,
        absolute_tolerance_w: f64,
        relative_tolerance: f64,
    ) -> Result<bool, ModelError> {
        if !absolute_tolerance_w.is_finite()
            || absolute_tolerance_w < 0.0
            || !relative_tolerance.is_finite()
            || relative_tolerance < 0.0
        {
            return Err(ModelError::InvalidTolerance {
                absolute_w: absolute_tolerance_w,
                relative: relative_tolerance,
            });
        }

        let allowed =
            absolute_tolerance_w.max(relative_tolerance * self.chemical_input_power.get());
        Ok(self.residual_w().abs() <= allowed)
    }
}

/// Result for one steady-state bookkeeping point.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PowerSystemResult {
    pub engine: EngineResult,
    pub generator: GeneratorResult,
    pub balance: EnergyBalance,
}

impl PowerSystemResult {
    /// Net electrical efficiency is undefined when there is no chemical input.
    pub fn net_electrical_efficiency(self) -> Option<f64> {
        let input = self.balance.chemical_input_power.get();
        if input == 0.0 {
            None
        } else {
            Some(self.balance.net_electrical_output.get() / input)
        }
    }
}

/// Evaluate one explicitly parameterized steady-state fuel → engine → generator
/// chain. No default fuel properties, efficiency maps or auxiliary loads exist.
pub fn evaluate(
    fuel: FuelInput,
    engine: EngineModel,
    generator: GeneratorModel,
) -> Result<PowerSystemResult, ModelError> {
    let chemical_input_power = fuel.chemical_input_power()?;
    let engine_result = engine.convert(chemical_input_power);
    let generator_result = generator.convert(engine_result.brake_power)?;

    let balance = EnergyBalance {
        chemical_input_power,
        engine_rejected_power: engine_result.rejected_power,
        generator_conversion_loss: generator_result.conversion_loss_power,
        auxiliary_load: generator_result.auxiliary_load,
        net_electrical_output: generator_result.net_electrical_power,
    };

    Ok(PowerSystemResult {
        engine: engine_result,
        generator: generator_result,
        balance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (FuelInput, EngineModel, GeneratorModel) {
        let fuel = FuelInput::new(
            MassFlowKgPerSecond::new(0.01).unwrap(),
            LowerHeatingValueJPerKg::new(43_000_000.0).unwrap(),
        );
        let engine = EngineModel::new(Efficiency::new(0.45).unwrap());
        let generator =
            GeneratorModel::new(Efficiency::new(0.96).unwrap(), Watts::new(5_760.0).unwrap());
        (fuel, engine, generator)
    }

    #[test]
    fn chemical_input_and_net_output_match_hand_calculated_fixture() {
        let (fuel, engine, generator) = fixture();
        let result = evaluate(fuel, engine, generator).unwrap();

        assert!((result.balance.chemical_input_power.get() - 430_000.0).abs() < 1e-9);
        assert!((result.engine.brake_power.get() - 193_500.0).abs() < 1e-9);
        assert!((result.generator.gross_electrical_power.get() - 185_760.0).abs() < 1e-9);
        assert!((result.generator.net_electrical_power.get() - 180_000.0).abs() < 1e-9);
        let expected_efficiency = 180_000.0 / 430_000.0;
        let actual_efficiency = result.net_electrical_efficiency().unwrap();
        assert!((actual_efficiency - expected_efficiency).abs() < 1e-12);
    }

    #[test]
    fn energy_ledger_closes_for_hand_calculated_fixture() {
        let (fuel, engine, generator) = fixture();
        let result = evaluate(fuel, engine, generator).unwrap();

        assert!(result.balance.is_closed(1e-8, 1e-12).unwrap());
        assert!(result.balance.residual_w().abs() < 1e-8);
    }

    #[test]
    fn rejects_efficiency_outside_physical_interval() {
        assert!(matches!(
            Efficiency::new(1.000_001),
            Err(ModelError::InvalidEfficiency { .. })
        ));
        assert!(matches!(
            Efficiency::new(-0.001),
            Err(ModelError::InvalidEfficiency { .. })
        ));
        assert!(matches!(
            Efficiency::new(f64::NAN),
            Err(ModelError::InvalidEfficiency { .. })
        ));
    }

    #[test]
    fn rejects_non_finite_and_negative_fuel_inputs() {
        assert!(MassFlowKgPerSecond::new(f64::INFINITY).is_err());
        assert!(MassFlowKgPerSecond::new(-0.1).is_err());
        assert!(LowerHeatingValueJPerKg::new(f64::NAN).is_err());
        assert!(LowerHeatingValueJPerKg::new(0.0).is_err());
    }

    #[test]
    fn rejects_auxiliaries_larger_than_gross_generation() {
        let fuel = FuelInput::new(
            MassFlowKgPerSecond::new(0.001).unwrap(),
            LowerHeatingValueJPerKg::new(10_000_000.0).unwrap(),
        );
        let engine = EngineModel::new(Efficiency::new(0.2).unwrap());
        let generator =
            GeneratorModel::new(Efficiency::new(0.9).unwrap(), Watts::new(5_000.0).unwrap());

        assert!(matches!(
            evaluate(fuel, engine, generator),
            Err(ModelError::AuxiliaryLoadExceedsGrossOutput { .. })
        ));
    }

    #[test]
    fn zero_fuel_input_has_no_defined_net_efficiency() {
        let fuel = FuelInput::new(
            MassFlowKgPerSecond::new(0.0).unwrap(),
            LowerHeatingValueJPerKg::new(43_000_000.0).unwrap(),
        );
        let engine = EngineModel::new(Efficiency::new(0.45).unwrap());
        let generator = GeneratorModel::new(Efficiency::new(0.96).unwrap(), Watts::zero());
        let result = evaluate(fuel, engine, generator).unwrap();

        assert_eq!(result.generator.net_electrical_power, Watts::zero());
        assert_eq!(result.net_electrical_efficiency(), None);
    }

    #[test]
    fn rejects_invalid_balance_tolerances() {
        let (fuel, engine, generator) = fixture();
        let balance = evaluate(fuel, engine, generator).unwrap().balance;

        assert!(balance.is_closed(-1.0, 0.0).is_err());
        assert!(balance.is_closed(0.0, f64::NAN).is_err());
    }
}
