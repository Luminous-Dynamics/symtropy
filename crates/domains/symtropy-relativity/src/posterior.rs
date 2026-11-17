// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic, sample-preserving transforms from Kerr remnant posteriors
//! to horizon observables. This module does not perform Bayesian inference,
//! resampling, marginalization, or waveform fitting.

use core::fmt;

use super::{DimensionlessSpin, KerrBlackHole, MassKg, PhysicsDomainError};

/// One remnant posterior draw. The sample identifier is carried through
/// unchanged so downstream evidence can trace every derived observable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrRemnantSample {
    pub sample_id: u64,
    pub mass: MassKg,
    pub dimensionless_spin: DimensionlessSpin,
}

impl KerrRemnantSample {
    pub fn from_si(
        sample_id: u64,
        mass_kg: f64,
        dimensionless_spin: f64,
    ) -> Result<Self, PosteriorTransformError> {
        Ok(Self {
            sample_id,
            mass: MassKg::new(mass_kg)?,
            dimensionless_spin: DimensionlessSpin::new(dimensionless_spin)?,
        })
    }
}

/// Kerr horizon observables derived from exactly one input posterior draw.
///
/// All output quantities are deterministic functions of the paired mass and
/// spin in the input sample. The mapping preserves input order and sample ID;
/// it does not independently shuffle or resample mass and spin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrHorizonPosteriorSample {
    pub sample_id: u64,
    pub mass_kg: f64,
    pub dimensionless_spin: f64,
    pub horizon_radius_m: f64,
    pub horizon_angular_velocity_rad_s: f64,
    pub surface_gravity_rate_s_inv: f64,
    pub direct_wave_reference_frequency_hz: f64,
}

/// Transform joint (mass, spin) samples into correlated Kerr horizon
/// observables. Empty input maps to empty output; callers must decide whether
/// an empty posterior is admissible for their analysis.
pub fn map_kerr_horizon_posterior(
    samples: &[KerrRemnantSample],
) -> Result<Vec<KerrHorizonPosteriorSample>, PosteriorTransformError> {
    let mut output = Vec::with_capacity(samples.len());

    for sample in samples {
        let hole = KerrBlackHole::new(sample.mass, sample.dimensionless_spin);
        let derived = KerrHorizonPosteriorSample {
            sample_id: sample.sample_id,
            mass_kg: sample.mass.get(),
            dimensionless_spin: sample.dimensionless_spin.get(),
            horizon_radius_m: hole.outer_horizon_radius_m(),
            horizon_angular_velocity_rad_s: hole.horizon_angular_velocity_rad_s(),
            surface_gravity_rate_s_inv: hole.surface_gravity_rate_s_inv(),
            direct_wave_reference_frequency_hz: hole.direct_wave_reference_frequency_hz(),
        };

        if !derived.horizon_radius_m.is_finite()
            || !derived.horizon_angular_velocity_rad_s.is_finite()
            || !derived.surface_gravity_rate_s_inv.is_finite()
            || !derived.direct_wave_reference_frequency_hz.is_finite()
        {
            return Err(PosteriorTransformError::NonFiniteDerivedValues {
                sample_id: sample.sample_id,
            });
        }

        output.push(derived);
    }

    Ok(output)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PosteriorTransformError {
    PhysicsDomain(PhysicsDomainError),
    NonFiniteDerivedValues { sample_id: u64 },
}

impl From<PhysicsDomainError> for PosteriorTransformError {
    fn from(value: PhysicsDomainError) -> Self {
        Self::PhysicsDomain(value)
    }
}

impl fmt::Display for PosteriorTransformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PhysicsDomain(error) => write!(f, "{error}"),
            Self::NonFiniteDerivedValues { sample_id } => {
                write!(
                    f,
                    "Kerr horizon transform produced non-finite values for sample {sample_id}"
                )
            }
        }
    }
}

impl std::error::Error for PosteriorTransformError {}

#[cfg(test)]
mod tests {
    use super::*;

    const SOLAR_MASS_KG: f64 = 1.988_47e30;

    #[test]
    fn mapping_preserves_sample_identity_order_and_joint_pairs() {
        let samples = [
            KerrRemnantSample::from_si(91, 70.0 * SOLAR_MASS_KG, 0.2).unwrap(),
            KerrRemnantSample::from_si(17, 62.0 * SOLAR_MASS_KG, -0.7).unwrap(),
            KerrRemnantSample::from_si(42, 85.0 * SOLAR_MASS_KG, 0.95).unwrap(),
        ];
        let mapped = map_kerr_horizon_posterior(&samples).unwrap();

        assert_eq!(mapped.len(), samples.len());
        for (input, output) in samples.iter().zip(mapped.iter()) {
            assert_eq!(input.sample_id, output.sample_id);
            assert_eq!(input.mass.get(), output.mass_kg);
            assert_eq!(input.dimensionless_spin.get(), output.dimensionless_spin);
        }
        assert!(mapped[0].horizon_angular_velocity_rad_s > 0.0);
        assert!(mapped[1].horizon_angular_velocity_rad_s < 0.0);
        assert!(mapped[1].direct_wave_reference_frequency_hz > 0.0);
        assert!(mapped[2].horizon_angular_velocity_rad_s > 0.0);
        assert!(mapped[2].surface_gravity_rate_s_inv < mapped[0].surface_gravity_rate_s_inv);
    }

    #[test]
    fn nonrotating_and_extremal_samples_retain_correct_limits() {
        let samples = [
            KerrRemnantSample::from_si(1, SOLAR_MASS_KG, 0.0).unwrap(),
            KerrRemnantSample::from_si(2, SOLAR_MASS_KG, 1.0).unwrap(),
        ];
        let mapped = map_kerr_horizon_posterior(&samples).unwrap();

        assert_eq!(mapped[0].horizon_angular_velocity_rad_s, 0.0);
        assert_eq!(mapped[0].direct_wave_reference_frequency_hz, 0.0);
        assert_eq!(mapped[1].surface_gravity_rate_s_inv, 0.0);
        assert!(mapped[1].direct_wave_reference_frequency_hz > 0.0);
    }

    #[test]
    fn empty_input_is_preserved_as_empty() {
        assert!(map_kerr_horizon_posterior(&[]).unwrap().is_empty());
    }

    #[test]
    fn invalid_mass_and_spin_are_rejected_before_mapping() {
        assert!(matches!(
            KerrRemnantSample::from_si(4, 0.0, 0.4),
            Err(PosteriorTransformError::PhysicsDomain(
                PhysicsDomainError::NonPositive("mass_kg")
            ))
        ));
        assert!(matches!(
            KerrRemnantSample::from_si(5, SOLAR_MASS_KG, 1.01),
            Err(PosteriorTransformError::PhysicsDomain(
                PhysicsDomainError::SpinOutOfRange(_)
            ))
        ));
    }
}
