// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Homogeneous lossy-medium plane-wave propagation under Maxwell's equations.
//!
//! This is an analytical material/reference model for a uniform, isotropic,
//! linear medium using SI units and the exp(+j omega t) phasor convention.
//! It is NOT an electrode-impedance model, layered-soil half-space solver,
//! near-field return-current model, guided Earth-ionosphere mode solver, or
//! reconstruction of Tesla's Colorado Springs/Wardenclyffe apparatus.
//!
//! For angular frequency omega, permittivity epsilon, permeability mu, and
//! conductivity sigma, define r = sigma/(omega*epsilon) and
//! K = omega*sqrt(mu*epsilon/2). Then:
//!   alpha = K * r / sqrt(sqrt(1+r^2) + 1)
//!   beta  = K * sqrt(sqrt(1+r^2) + 1)
//! where alpha is amplitude attenuation in Np/m and beta is phase constant
//! in rad/m. This algebraically equivalent form avoids cancellation in
//! sqrt(1+r^2)-1 when r is small.

pub mod electrode;

use std::f64::consts::PI;

/// Vacuum permittivity in F/m.
pub const EPSILON_0_F_PER_M: f64 = 8.854_187_812_8e-12;
/// Vacuum permeability in H/m.
pub const MU_0_H_PER_M: f64 = 1.256_637_062_12e-6;
/// Conversion factor from Np/m to dB/m for field amplitude.
pub const NEPERS_TO_DB: f64 = 8.685_889_638_065_037;

/// Input validation or model-domain failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelError {
    InvalidFrequency,
    InvalidPermittivity,
    InvalidPermeability,
    InvalidConductivity,
    InvalidDistance,
    InvalidGeometry,
    NotGoodConductor,
    InvalidDerivedValue,
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::InvalidFrequency => "frequency must be finite and greater than zero",
            Self::InvalidPermittivity => "relative permittivity must be finite and greater than zero",
            Self::InvalidPermeability => "relative permeability must be finite and greater than zero",
            Self::InvalidConductivity => "conductivity must be finite and non-negative",
            Self::InvalidDistance => "distance must be finite and non-negative",
            Self::InvalidGeometry => "electrode geometry is outside the declared approximation envelope",
            Self::NotGoodConductor => "good-conductor approximation is outside its declared regime",
            Self::InvalidDerivedValue => "derived propagation quantity is non-finite or invalid",
        };
        f.write_str(message)
    }
}

impl std::error::Error for ModelError {}

/// Homogeneous, isotropic, linear medium.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Medium {
    relative_permittivity: f64,
    relative_permeability: f64,
    conductivity_s_per_m: f64,
}

impl Medium {
    pub fn new(
        relative_permittivity: f64,
        relative_permeability: f64,
        conductivity_s_per_m: f64,
    ) -> Result<Self, ModelError> {
        if !relative_permittivity.is_finite() || relative_permittivity <= 0.0 {
            return Err(ModelError::InvalidPermittivity);
        }
        if !relative_permeability.is_finite() || relative_permeability <= 0.0 {
            return Err(ModelError::InvalidPermeability);
        }
        if !conductivity_s_per_m.is_finite() || conductivity_s_per_m < 0.0 {
            return Err(ModelError::InvalidConductivity);
        }
        Ok(Self { relative_permittivity, relative_permeability, conductivity_s_per_m })
    }

    pub fn relative_permittivity(self) -> f64 {
        self.relative_permittivity
    }

    pub fn relative_permeability(self) -> f64 {
        self.relative_permeability
    }

    pub fn conductivity_s_per_m(self) -> f64 {
        self.conductivity_s_per_m
    }

    pub fn permittivity_f_per_m(self) -> f64 {
        EPSILON_0_F_PER_M * self.relative_permittivity
    }

    pub fn permeability_h_per_m(self) -> f64 {
        MU_0_H_PER_M * self.relative_permeability
    }
}

/// Plane-wave propagation quantities for a single frequency and medium.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Propagation {
    pub frequency_hz: f64,
    pub angular_frequency_rad_per_s: f64,
    /// Amplitude attenuation constant alpha in nepers per metre.
    pub attenuation_np_per_m: f64,
    /// Amplitude attenuation constant in decibels per metre.
    pub attenuation_db_per_m: f64,
    /// Phase constant beta in radians per metre.
    pub phase_constant_rad_per_m: f64,
    /// Phase wavelength in metres.
    pub wavelength_m: f64,
    /// 1/alpha in metres; None if the computed medium is lossless.
    pub skin_depth_m: Option<f64>,
    /// sigma/(omega*epsilon), conduction-to-displacement-current ratio.
    pub conduction_ratio: f64,
}

impl Propagation {
    /// Field-amplitude ratio exp(-alpha*d) after distance d.
    pub fn amplitude_ratio(self, distance_m: f64) -> Result<f64, ModelError> {
        if !distance_m.is_finite() || distance_m < 0.0 {
            return Err(ModelError::InvalidDistance);
        }
        let ratio = (-self.attenuation_np_per_m * distance_m).exp();
        if !ratio.is_finite() {
            return Err(ModelError::InvalidDerivedValue);
        }
        Ok(ratio)
    }

    /// Power ratio exp(-2*alpha*d) after distance d.
    pub fn power_ratio(self, distance_m: f64) -> Result<f64, ModelError> {
        let amplitude = self.amplitude_ratio(distance_m)?;
        Ok(amplitude * amplitude)
    }
}

/// Compute homogeneous-medium plane-wave propagation constants.
///
/// Supports any finite positive frequency and the declared material model.
/// At zero conductivity the medium is lossless and skin depth is None.
pub fn propagation(frequency_hz: f64, medium: Medium) -> Result<Propagation, ModelError> {
    if !frequency_hz.is_finite() || frequency_hz <= 0.0 {
        return Err(ModelError::InvalidFrequency);
    }

    let omega = 2.0 * PI * frequency_hz;
    let epsilon = medium.permittivity_f_per_m();
    let mu = medium.permeability_h_per_m();
    let omega_epsilon = omega * epsilon;
    let mu_epsilon = mu * epsilon;
    if !omega.is_finite() || !epsilon.is_finite() || !mu.is_finite()
        || !omega_epsilon.is_finite() || omega_epsilon <= 0.0
        || !mu_epsilon.is_finite() || mu_epsilon <= 0.0
    {
        return Err(ModelError::InvalidDerivedValue);
    }

    let ratio = medium.conductivity_s_per_m / omega_epsilon;
    if !ratio.is_finite() {
        return Err(ModelError::InvalidDerivedValue);
    }
    let root = ratio.hypot(1.0);
    let root_plus_one = root + 1.0;
    let scale = omega * (mu_epsilon / 2.0).sqrt();
    let attenuation = scale * ratio / root_plus_one.sqrt();
    let phase = scale * root_plus_one.sqrt();
    if !attenuation.is_finite() || attenuation < 0.0
        || !phase.is_finite() || phase <= 0.0
    {
        return Err(ModelError::InvalidDerivedValue);
    }

    let wavelength = 2.0 * PI / phase;
    let skin_depth = if attenuation > 0.0 { Some(1.0 / attenuation) } else { None };
    let attenuation_db = NEPERS_TO_DB * attenuation;
    if !wavelength.is_finite() || wavelength <= 0.0 || !attenuation_db.is_finite()
        || skin_depth.is_some_and(|value| !value.is_finite() || value <= 0.0)
    {
        return Err(ModelError::InvalidDerivedValue);
    }

    Ok(Propagation {
        frequency_hz,
        angular_frequency_rad_per_s: omega,
        attenuation_np_per_m: attenuation,
        attenuation_db_per_m: attenuation_db,
        phase_constant_rad_per_m: phase,
        wavelength_m: wavelength,
        skin_depth_m: skin_depth,
        conduction_ratio: ratio,
    })
}

/// Good-conductor approximation delta ~= sqrt(2/(omega*mu*sigma)).
///
/// It is returned only for sigma/(omega*epsilon) >= 100. It is a reference
/// approximation, not a separate propagation model.
pub fn good_conductor_skin_depth(
    frequency_hz: f64,
    medium: Medium,
) -> Result<f64, ModelError> {
    let exact = propagation(frequency_hz, medium)?;
    if exact.conduction_ratio < 100.0 {
        return Err(ModelError::NotGoodConductor);
    }
    let denominator = exact.angular_frequency_rad_per_s
        * medium.permeability_h_per_m()
        * medium.conductivity_s_per_m;
    let depth = (2.0 / denominator).sqrt();
    if !depth.is_finite() || depth <= 0.0 {
        return Err(ModelError::InvalidDerivedValue);
    }
    Ok(depth)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference_medium(conductivity: f64) -> Medium {
        Medium::new(10.0, 1.0, conductivity).unwrap()
    }

    #[test]
    fn lossless_vacuum_matches_free_space_wave_number() {
        let vacuum = Medium::new(1.0, 1.0, 0.0).unwrap();
        let result = propagation(1.0e6, vacuum).unwrap();
        let c = 1.0 / (MU_0_H_PER_M * EPSILON_0_F_PER_M).sqrt();
        let expected_beta = 2.0 * PI * 1.0e6 / c;
        assert_eq!(result.attenuation_np_per_m, 0.0);
        assert_eq!(result.skin_depth_m, None);
        assert!((result.phase_constant_rad_per_m - expected_beta).abs() / expected_beta < 1e-12);
        assert!((result.amplitude_ratio(123.0).unwrap() - 1.0).abs() < 1e-15);
    }

    #[test]
    fn lossy_reference_matches_known_value() {
        let result = propagation(10_000.0, reference_medium(0.01)).unwrap();
        let depth = result.skin_depth_m.unwrap();
        assert!((depth - 50.343_213_737_5).abs() < 1e-8);
        assert!((result.attenuation_db_per_m - 0.172_533_475_58).abs() < 1e-10);
        assert!((result.amplitude_ratio(depth).unwrap() - (-1.0_f64).exp()).abs() < 1e-12);
        assert!((result.power_ratio(depth).unwrap() - (-2.0_f64).exp()).abs() < 1e-12);
    }

    #[test]
    fn good_conductor_approximation_matches_exact_formula_in_its_regime() {
        let medium = reference_medium(0.01);
        let exact = propagation(10_000.0, medium).unwrap();
        let approximate = good_conductor_skin_depth(10_000.0, medium).unwrap();
        let exact_depth = exact.skin_depth_m.unwrap();
        assert!(exact.conduction_ratio >= 100.0);
        assert!((approximate - exact_depth).abs() / exact_depth < 0.001);
    }

    #[test]
    fn approximation_rejects_low_conduction_ratio() {
        let lossless = Medium::new(1.0, 1.0, 0.0).unwrap();
        assert_eq!(good_conductor_skin_depth(1.0e6, lossless), Err(ModelError::NotGoodConductor));
    }

    #[test]
    fn frequency_and_conductivity_increase_attenuation() {
        let low_frequency = propagation(1_000.0, reference_medium(0.01)).unwrap().skin_depth_m.unwrap();
        let high_frequency = propagation(10_000.0, reference_medium(0.01)).unwrap().skin_depth_m.unwrap();
        let low_conductivity = propagation(10_000.0, reference_medium(0.001)).unwrap().skin_depth_m.unwrap();
        let high_conductivity = propagation(10_000.0, reference_medium(0.01)).unwrap().skin_depth_m.unwrap();
        assert!(high_frequency < low_frequency);
        assert!(high_conductivity < low_conductivity);
    }

    #[test]
    fn rejects_invalid_inputs_and_distances() {
        assert_eq!(Medium::new(0.0, 1.0, 0.01), Err(ModelError::InvalidPermittivity));
        assert_eq!(Medium::new(1.0, 0.0, 0.01), Err(ModelError::InvalidPermeability));
        assert_eq!(Medium::new(1.0, 1.0, -0.1), Err(ModelError::InvalidConductivity));
        assert_eq!(propagation(f64::NAN, reference_medium(0.01)), Err(ModelError::InvalidFrequency));
        let valid = propagation(1.0e4, reference_medium(0.01)).unwrap();
        assert_eq!(valid.amplitude_ratio(-1.0), Err(ModelError::InvalidDistance));
        assert_eq!(valid.power_ratio(f64::INFINITY), Err(ModelError::InvalidDistance));
    }
}