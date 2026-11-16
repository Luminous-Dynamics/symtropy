//! Analytical reference equations for a single-species, rigid, isothermal
//! pressure compartment.
//!
//! This is an S1 ideal-gas bookkeeping model, not a leak-flow solver, pressure
//! vessel stress analysis, geomechanics model, life-support controller, or
//! safety certification. Mass transfers are prescribed inputs (kg per interval);
//! no leak conductance or fluid-dynamic law is inferred here.

use crate::Measurement;

const UNIVERSAL_GAS_CONSTANT_J_PER_MOL_K: f64 = 8.314_462_618_153_24;

#[derive(Clone, Debug, PartialEq)]
pub struct GasCompartment {
    /// Fixed internal volume in cubic metres.
    pub volume_m3: f64,
    /// Uniform gas temperature in kelvin.
    pub temperature_k: f64,
    /// Molar mass of the single represented gas species in kg/mol.
    pub molar_mass_kg_per_mol: f64,
    /// Total gas inventory in kilograms.
    pub gas_mass_kg: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MassTransfer {
    /// Known gas mass entering during the interval, kg.
    pub inflow_kg: f64,
    /// Known gas mass leaving during the interval, kg.
    pub outflow_kg: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PressureStatus {
    WithinTolerance,
    OutsideTolerance,
    Unquantified,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PressureComparison {
    pub predicted_pressure_pa: f64,
    pub observed_pressure_pa: f64,
    /// Observed minus predicted. Positive means measured pressure is higher.
    pub residual_pa: f64,
    pub observed_standard_uncertainty_pa: Option<f64>,
    pub status: PressureStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PressureModelError {
    InvalidVolume,
    InvalidTemperature,
    InvalidMolarMass,
    InvalidGasMass,
    InvalidMassTransfer,
    OutflowExceedsAvailableInventory,
    InvalidTolerance,
    InvalidZScore,
    NumericalOverflow,
}

impl std::fmt::Display for PressureModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::InvalidVolume => "compartment volume must be finite and > 0",
            Self::InvalidTemperature => "gas temperature must be finite and > 0 K",
            Self::InvalidMolarMass => "gas molar mass must be finite and > 0 kg/mol",
            Self::InvalidGasMass => "gas mass must be finite and >= 0 kg",
            Self::InvalidMassTransfer => "inflow/outflow must be finite and >= 0 kg",
            Self::OutflowExceedsAvailableInventory => {
                "outflow exceeds initial inventory plus supplied inflow"
            }
            Self::InvalidTolerance => "absolute pressure tolerance must be finite and >= 0 Pa",
            Self::InvalidZScore => "z-score must be finite and >= 0",
            Self::NumericalOverflow => "non-finite result in ideal-gas pressure calculation",
        };
        write!(f, "{message}")
    }
}

impl std::error::Error for PressureModelError {}

impl GasCompartment {
    pub fn validate(&self) -> Result<(), PressureModelError> {
        if !self.volume_m3.is_finite() || self.volume_m3 <= 0.0 {
            return Err(PressureModelError::InvalidVolume);
        }
        if !self.temperature_k.is_finite() || self.temperature_k <= 0.0 {
            return Err(PressureModelError::InvalidTemperature);
        }
        if !self.molar_mass_kg_per_mol.is_finite()
            || self.molar_mass_kg_per_mol <= 0.0
        {
            return Err(PressureModelError::InvalidMolarMass);
        }
        if !self.gas_mass_kg.is_finite() || self.gas_mass_kg < 0.0 {
            return Err(PressureModelError::InvalidGasMass);
        }
        Ok(())
    }

    /// Calculate absolute pressure in pascals using pV = nRT and n = m/M.
    pub fn ideal_gas_pressure_pa(&self) -> Result<f64, PressureModelError> {
        self.validate()?;
        let specific_gas_constant =
            UNIVERSAL_GAS_CONSTANT_J_PER_MOL_K / self.molar_mass_kg_per_mol;
        let pressure = self.gas_mass_kg * specific_gas_constant
            * self.temperature_k / self.volume_m3;
        if pressure.is_finite() && pressure >= 0.0 {
            Ok(pressure)
        } else {
            Err(PressureModelError::NumericalOverflow)
        }
    }

    /// Apply an already quantified net mass transfer. This does not estimate
    /// flow from a pressure differential; that requires a separately validated
    /// orifice/conductance/rarefied-flow model.
    pub fn apply_known_mass_transfer(
        &self,
        transfer: &MassTransfer,
    ) -> Result<Self, PressureModelError> {
        self.validate()?;
        if !transfer.inflow_kg.is_finite()
            || !transfer.outflow_kg.is_finite()
            || transfer.inflow_kg < 0.0
            || transfer.outflow_kg < 0.0
        {
            return Err(PressureModelError::InvalidMassTransfer);
        }

        let available = self.gas_mass_kg + transfer.inflow_kg;
        if !available.is_finite() {
            return Err(PressureModelError::NumericalOverflow);
        }
        if transfer.outflow_kg > available {
            return Err(PressureModelError::OutflowExceedsAvailableInventory);
        }

        let new_mass = available - transfer.outflow_kg;
        if !new_mass.is_finite() || new_mass < 0.0 {
            return Err(PressureModelError::NumericalOverflow);
        }

        Ok(Self {
            gas_mass_kg: new_mass,
            ..self.clone()
        })
    }
}

/// Compare a deterministic model prediction with one observed absolute
/// pressure measurement. Only observation uncertainty is represented here;
/// uncertainty in volume, temperature, gas composition, mass inventory and
/// model form is not propagated. Such omitted uncertainty must be added before
/// the result is used for an engineering qualification.
pub fn compare_pressure_observation(
    predicted_pressure_pa: f64,
    observed_pressure: &Measurement,
    absolute_tolerance_pa: f64,
    z_score: f64,
) -> Result<PressureComparison, PressureModelError> {
    if !predicted_pressure_pa.is_finite() || predicted_pressure_pa < 0.0 {
        return Err(PressureModelError::NumericalOverflow);
    }
    if !absolute_tolerance_pa.is_finite() || absolute_tolerance_pa < 0.0 {
        return Err(PressureModelError::InvalidTolerance);
    }
    if !z_score.is_finite() || z_score < 0.0 {
        return Err(PressureModelError::InvalidZScore);
    }
    if !observed_pressure.value.is_finite() || observed_pressure.value < 0.0 {
        return Err(PressureModelError::NumericalOverflow);
    }
    if let Some(sigma) = observed_pressure.standard_uncertainty {
        if !sigma.is_finite() || sigma < 0.0 {
            return Err(PressureModelError::NumericalOverflow);
        }
    }

    let residual = observed_pressure.value - predicted_pressure_pa;
    if !residual.is_finite() {
        return Err(PressureModelError::NumericalOverflow);
    }

    let status = match observed_pressure.standard_uncertainty {
        Some(sigma) => {
            let threshold = absolute_tolerance_pa + z_score * sigma;
            if !threshold.is_finite() {
                return Err(PressureModelError::NumericalOverflow);
            }
            if residual.abs() <= threshold {
                PressureStatus::WithinTolerance
            } else {
                PressureStatus::OutsideTolerance
            }
        }
        None => PressureStatus::Unquantified,
    };

    Ok(PressureComparison {
        predicted_pressure_pa,
        observed_pressure_pa: observed_pressure.value,
        residual_pa: residual,
        observed_standard_uncertainty_pa: observed_pressure.standard_uncertainty,
        status,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_m3_oxygen() -> GasCompartment {
        GasCompartment {
            volume_m3: 1.0,
            temperature_k: 293.15,
            molar_mass_kg_per_mol: 0.031998,
            gas_mass_kg: 0.031998,
        }
    }

    #[test]
    fn ideal_gas_reference_matches_one_mole_identity() {
        let pressure = one_m3_oxygen().ideal_gas_pressure_pa().unwrap();
        let expected = UNIVERSAL_GAS_CONSTANT_J_PER_MOL_K * 293.15;
        assert!((pressure - expected).abs() < 1e-9);
    }

    #[test]
    fn known_mass_loss_reduces_pressure_proportionally_when_temperature_is_fixed() {
        let initial = one_m3_oxygen();
        let initial_pressure = initial.ideal_gas_pressure_pa().unwrap();
        let after = initial
            .apply_known_mass_transfer(&MassTransfer {
                inflow_kg: 0.0,
                outflow_kg: initial.gas_mass_kg * 0.10,
            })
            .unwrap();
        let after_pressure = after.ideal_gas_pressure_pa().unwrap();
        assert!((after_pressure / initial_pressure - 0.90).abs() < 1e-12);
    }

    #[test]
    fn transfer_cannot_remove_more_gas_than_available() {
        let initial = one_m3_oxygen();
        let result = initial.apply_known_mass_transfer(&MassTransfer {
            inflow_kg: 0.0,
            outflow_kg: initial.gas_mass_kg + 0.001,
        });
        assert!(matches!(
            result,
            Err(PressureModelError::OutflowExceedsAvailableInventory)
        ));
    }

    #[test]
    fn missing_observation_uncertainty_is_unquantified() {
        let pressure = one_m3_oxygen().ideal_gas_pressure_pa().unwrap();
        let observed = Measurement::unquantified_uncertainty(pressure);
        let comparison = compare_pressure_observation(pressure, &observed, 1.0, 3.0).unwrap();
        assert_eq!(comparison.status, PressureStatus::Unquantified);
    }

    #[test]
    fn seeded_pressure_deviation_is_detected() {
        let pressure = one_m3_oxygen().ideal_gas_pressure_pa().unwrap();
        let observed = Measurement::measured(pressure + 500.0, 1.0, None);
        let comparison = compare_pressure_observation(pressure, &observed, 5.0, 3.0).unwrap();
        assert_eq!(comparison.status, PressureStatus::OutsideTolerance);
        assert!((comparison.residual_pa - 500.0).abs() < 1e-9);
    }
}
