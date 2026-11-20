#![forbid(unsafe_code)]
//! Research-only numerical reference for a homogeneous screened thick-target
//! D-D yield integral. This is not a validated nuclear-data library, a
//! heterogeneous target model, or an experimental reproduction.

use std::fmt;

const BARN_TO_CM2: f64 = 1.0e-24;

#[derive(Debug, Clone, PartialEq)]
pub enum ModelError {
    InvalidParameter(&'static str),
    NonFiniteValue(&'static str),
    NonPositiveValue(&'static str),
    InvalidIntegrationSteps,
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParameter(name) => write!(f, "invalid parameter: {name}"),
            Self::NonFiniteValue(name) => write!(f, "non-finite value: {name}"),
            Self::NonPositiveValue(name) => write!(f, "non-positive value: {name}"),
            Self::InvalidIntegrationSteps => write!(f, "Simpson integration requires an even step count >= 2"),
        }
    }
}

impl std::error::Error for ModelError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnergyKev(f64);

impl EnergyKev {
    pub fn new(value: f64) -> Result<Self, ModelError> {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue("energy_keV"));
        }
        if value <= 0.0 {
            return Err(ModelError::NonPositiveValue("energy_keV"));
        }
        Ok(Self(value))
    }

    pub fn get(self) -> f64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GamowEnergyKev(f64);

impl GamowEnergyKev {
    pub fn new(value: f64) -> Result<Self, ModelError> {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue("gamow_energy_keV"));
        }
        if value <= 0.0 {
            return Err(ModelError::NonPositiveValue("gamow_energy_keV"));
        }
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetDensityPerCm3(f64);

impl TargetDensityPerCm3 {
    pub fn new(value: f64) -> Result<Self, ModelError> {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue("target_density_cm^-3"));
        }
        if value <= 0.0 {
            return Err(ModelError::NonPositiveValue("target_density_cm^-3"));
        }
        Ok(Self(value))
    }
}

/// Homogeneous screened thick-target yield per incident projectile.
///
/// `s_factor_keV_barn(E)` returns the astrophysical S factor in keV·barn;
/// `stopping_keV_per_cm(E)` returns positive stopping power in keV/cm;
/// `screening_keV(E)` returns a non-negative effective screening potential
/// in keV. Target number density is in cm^-3; all energies use keV. The result
/// is dimensionless (reactions per incident projectile).
///
/// Implements n_D * integral[sigma(E, Ue) / stopping(E) dE], converting barns
/// to cm^2. Inputs are deliberately supplied by the caller so sourced,
/// versioned nuclear and stopping data can be injected with their uncertainties.
pub fn homogeneous_thick_target_yield<FS, FStop, FU>(
    incident_energy: EnergyKev,
    gamow_energy: GamowEnergyKev,
    target_density: TargetDensityPerCm3,
    steps: usize,
    s_factor_keV_barn: FS,
    stopping_keV_per_cm: FStop,
    screening_keV: FU,
) -> Result<f64, ModelError>
where
    FS: Fn(f64) -> f64,
    FStop: Fn(f64) -> f64,
    FU: Fn(f64) -> f64,
{
    if steps < 2 || steps % 2 != 0 {
        return Err(ModelError::InvalidIntegrationSteps);
    }

    // E = t^2 regularizes the integrable E^-1/2 endpoint for Ue > 0.
    let t_max = incident_energy.get().sqrt();
    let h = t_max / steps as f64;
    let mut weighted_sum = 0.0;

    for i in 0..=steps {
        let t = i as f64 * h;
        let energy = t * t;
        let s_factor = checked_positive(s_factor_keV_barn(energy), "S_factor_keV_barn")?;
        let stopping = checked_positive(stopping_keV_per_cm(energy), "stopping_keV_per_cm")?;
        let screening = screening_keV(energy);

        if !screening.is_finite() {
            return Err(ModelError::NonFiniteValue("screening_keV"));
        }
        if screening < 0.0 {
            return Err(ModelError::InvalidParameter("screening_keV must be >= 0"));
        }

        // This is 2*t*sigma(t^2,Ue), the transformed cross-section integrand.
        let transformed = if t == 0.0 {
            if screening > 0.0 {
                2.0 * s_factor / screening.sqrt() * (-(gamow_energy.0 / screening).sqrt()).exp()
            } else {
                0.0
            }
        } else {
            let shifted_energy = energy + screening;
            let cross_section_barn = s_factor
                / (energy * shifted_energy).sqrt()
                * (-(gamow_energy.0 / shifted_energy).sqrt()).exp();
            2.0 * t * cross_section_barn
        };

        let integrand = target_density.0 * BARN_TO_CM2 * transformed / stopping;
        if !integrand.is_finite() || integrand < 0.0 {
            return Err(ModelError::InvalidParameter("yield integrand must be finite and >= 0"));
        }

        let coefficient = if i == 0 || i == steps {
            1.0
        } else if i % 2 == 0 {
            2.0
        } else {
            4.0
        };
        weighted_sum += coefficient * integrand;
    }

    let result = weighted_sum * h / 3.0;
    if !result.is_finite() || result < 0.0 {
        return Err(ModelError::InvalidParameter("yield result must be finite and >= 0"));
    }
    Ok(result)
}

fn checked_positive(value: f64, name: &'static str) -> Result<f64, ModelError> {
    if !value.is_finite() {
        return Err(ModelError::NonFiniteValue(name));
    }
    if value <= 0.0 {
        return Err(ModelError::NonPositiveValue(name));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_yield(screening: f64, steps: usize) -> Result<f64, ModelError> {
        homogeneous_thick_target_yield(
            EnergyKev::new(6.5)?,
            GamowEnergyKev::new(986.0)?,
            TargetDensityPerCm3::new(5.0e22)?,
            steps,
            |_| 50.0,        // synthetic constant S factor, keV·barn
            |_| 1.0e8,       // synthetic constant stopping power, keV/cm
            |_| screening,   // synthetic effective screening potential, keV
        )
    }

    #[test]
    fn bare_reference_is_finite_and_positive() {
        let value = synthetic_yield(0.0, 4096).expect("valid synthetic reference");
        assert!(value.is_finite());
        assert!(value > 0.0);
    }

    #[test]
    fn increasing_screening_increases_reference_yield() {
        let bare = synthetic_yield(0.0, 4096).expect("bare");
        let screened = synthetic_yield(0.5, 4096).expect("screened");
        assert!(screened > bare);
    }

    #[test]
    fn numerical_integral_converges_under_step_refinement() {
        let coarse = synthetic_yield(0.25, 1024).expect("coarse");
        let fine = synthetic_yield(0.25, 2048).expect("fine");
        let relative_error = (coarse - fine).abs() / fine;
        assert!(relative_error < 1.0e-6, "relative error={relative_error:e}");
    }

    #[test]
    fn rejects_zero_stopping_power() {
        let result = homogeneous_thick_target_yield(
            EnergyKev::new(6.5).expect("energy"),
            GamowEnergyKev::new(986.0).expect("Gamow energy"),
            TargetDensityPerCm3::new(5.0e22).expect("density"),
            128,
            |_| 50.0,
            |_| 0.0,
            |_| 0.0,
        );
        assert_eq!(result, Err(ModelError::NonPositiveValue("stopping_keV_per_cm")));
    }

    #[test]
    fn rejects_odd_or_insufficient_integration_steps() {
        assert_eq!(synthetic_yield(0.0, 1), Err(ModelError::InvalidIntegrationSteps));
        assert_eq!(synthetic_yield(0.0, 3), Err(ModelError::InvalidIntegrationSteps));
    }

    #[test]
    fn rejects_negative_screening() {
        let result = homogeneous_thick_target_yield(
            EnergyKev::new(6.5).expect("energy"),
            GamowEnergyKev::new(986.0).expect("Gamow energy"),
            TargetDensityPerCm3::new(5.0e22).expect("density"),
            128,
            |_| 50.0,
            |_| 1.0e8,
            |_| -0.1,
        );
        assert_eq!(result, Err(ModelError::InvalidParameter("screening_keV must be >= 0")));
    }
}
