// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Analytical impedance of an ideal hemispherical electrode at the surface
//! of a homogeneous conductive half-space.
//!
//! With electrode radius a and complex bulk admittivity gamma = sigma +
//! j*omega*epsilon, radial current crosses hemispherical area 2*pi*r^2.
//! Integrating the differential impedance from a to infinity gives:
//!
//!   Z = 1 / (2*pi*a*(sigma + j*omega*epsilon)).
//!
//! This is an ideal single-electrode spreading-impedance reference only. It
//! does not include a second return electrode, finite domain, layering,
//! surface coatings, contact impedance, lead inductance, or real geometry.
//! A real two-terminal resistance measurement is not equal to this quantity.

use crate::{Medium, ModelError};
use std::f64::consts::PI;

/// Complex impedance in ohms, using the exp(+j omega t) convention.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComplexImpedance {
    /// Real (resistive) component in ohms.
    pub re_ohms: f64,
    /// Imaginary (reactive) component in ohms.
    pub im_ohms: f64,
}

impl ComplexImpedance {
    pub fn magnitude_ohms(self) -> f64 {
        self.re_ohms.hypot(self.im_ohms)
    }
}

/// Ideal hemispherical electrode spreading impedance in a uniform half-space.
///
/// Frequency may be zero for the DC resistance limit. The electrode is
/// assumed to be an ideal equipotential hemisphere of radius `radius_m`
/// embedded flush with a perfectly insulating planar boundary. The medium
/// occupies the half-space beneath it.
pub fn hemispherical_electrode_impedance(
    frequency_hz: f64,
    radius_m: f64,
    medium: Medium,
) -> Result<ComplexImpedance, ModelError> {
    if !frequency_hz.is_finite() || frequency_hz < 0.0 {
        return Err(ModelError::InvalidFrequency);
    }
    if !radius_m.is_finite() || radius_m <= 0.0 {
        return Err(ModelError::InvalidDistance);
    }

    let omega_epsilon = 2.0 * PI * frequency_hz * medium.permittivity_f_per_m();
    let sigma = medium.conductivity_s_per_m();
    let denominator = 2.0 * PI * radius_m * (sigma * sigma + omega_epsilon * omega_epsilon);
    if !omega_epsilon.is_finite() || !denominator.is_finite() || denominator <= 0.0 {
        return Err(ModelError::InvalidDerivedValue);
    }

    let result = ComplexImpedance {
        re_ohms: sigma / denominator,
        im_ohms: -omega_epsilon / denominator,
    };
    if !result.re_ohms.is_finite() || !result.im_ohms.is_finite() {
        return Err(ModelError::InvalidDerivedValue);
    }
    Ok(result)
}

/// First-order two-electrode impedance approximation in a uniform half-space.
///
/// For two small hemispherical electrodes at centre separation d, with radii
/// a1 and a2 much smaller than d, the half-space point-source Green function
/// gives the geometry factor:
///
///   Z ~= (1/(2*pi*(sigma + j*omega*epsilon)))
///        * (1/a1 + 1/a2 - 2/d).
///
/// This far-separated point-electrode approximation is admitted only when
/// max(a1,a2)/d <= 0.1. It is not an exact finite-electrode solution and is
/// not valid for plates, rods, water mains, layered soils, nearby electrodes,
/// contact impedance, finite ground grids, or dynamic wave propagation.
pub fn two_hemispherical_electrode_impedance_approx(
    frequency_hz: f64,
    radius_1_m: f64,
    radius_2_m: f64,
    separation_m: f64,
    medium: Medium,
) -> Result<ComplexImpedance, ModelError> {
    if !frequency_hz.is_finite() || frequency_hz < 0.0 {
        return Err(ModelError::InvalidFrequency);
    }
    if !radius_1_m.is_finite() || radius_1_m <= 0.0
        || !radius_2_m.is_finite() || radius_2_m <= 0.0
        || !separation_m.is_finite() || separation_m <= 0.0
    {
        return Err(ModelError::InvalidDistance);
    }

    let largest_radius = radius_1_m.max(radius_2_m);
    if largest_radius / separation_m > 0.1 {
        return Err(ModelError::InvalidGeometry);
    }

    let geometry_factor = (1.0 / radius_1_m + 1.0 / radius_2_m
        - 2.0 / separation_m) / (2.0 * PI);
    if !geometry_factor.is_finite() || geometry_factor <= 0.0 {
        return Err(ModelError::InvalidGeometry);
    }

    let omega_epsilon = 2.0 * PI * frequency_hz * medium.permittivity_f_per_m();
    let sigma = medium.conductivity_s_per_m();
    let denominator = sigma * sigma + omega_epsilon * omega_epsilon;
    if !omega_epsilon.is_finite() || !denominator.is_finite() || denominator <= 0.0 {
        return Err(ModelError::InvalidDerivedValue);
    }

    let result = ComplexImpedance {
        re_ohms: geometry_factor * sigma / denominator,
        im_ohms: -geometry_factor * omega_epsilon / denominator,
    };
    if !result.re_ohms.is_finite() || !result.im_ohms.is_finite() {
        return Err(ModelError::InvalidDerivedValue);
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dc_limit_matches_hemispherical_spreading_resistance() {
        let medium = Medium::new(10.0, 1.0, 0.01).unwrap();
        let impedance = hemispherical_electrode_impedance(0.0, 0.1, medium).unwrap();
        let expected = 1.0 / (2.0 * PI * 0.1 * 0.01);
        assert!((impedance.re_ohms - expected).abs() / expected < 1e-12);
        assert_eq!(impedance.im_ohms, 0.0);
    }

    #[test]
    fn lossless_medium_has_ideal_capacitive_impedance() {
        let medium = Medium::new(10.0, 1.0, 0.0).unwrap();
        let frequency = 1_000.0;
        let radius = 0.1;
        let impedance = hemispherical_electrode_impedance(frequency, radius, medium).unwrap();
        let capacitance = 2.0 * PI * radius * medium.permittivity_f_per_m();
        let expected_reactance = -1.0 / (2.0 * PI * frequency * capacitance);
        assert_eq!(impedance.re_ohms, 0.0);
        assert!((impedance.im_ohms - expected_reactance).abs() / expected_reactance.abs() < 1e-12);
    }

    #[test]
    fn rejects_invalid_frequency_radius_and_no_dc_return_path() {
        let conductive = Medium::new(10.0, 1.0, 0.01).unwrap();
        assert_eq!(hemispherical_electrode_impedance(-1.0, 0.1, conductive), Err(ModelError::InvalidFrequency));
        assert_eq!(hemispherical_electrode_impedance(0.0, 0.0, conductive), Err(ModelError::InvalidDistance));
        let lossless = Medium::new(10.0, 1.0, 0.0).unwrap();
        assert_eq!(hemispherical_electrode_impedance(0.0, 0.1, lossless), Err(ModelError::InvalidDerivedValue));
    }
    #[test]
    fn two_electrode_dc_limit_matches_point_source_geometry_factor() {
        let medium = Medium::new(10.0, 1.0, 0.01).unwrap();
        let impedance = two_hemispherical_electrode_impedance_approx(
            0.0, 0.1, 0.1, 10.0, medium,
        ).unwrap();
        let expected = (1.0 / 0.1 + 1.0 / 0.1 - 2.0 / 10.0)
            / (2.0 * PI * 0.01);
        assert!((impedance.re_ohms - expected).abs() / expected < 1e-12);
        assert_eq!(impedance.im_ohms, 0.0);
    }

    #[test]
    fn two_electrode_mutual_term_reduces_impedance() {
        let medium = Medium::new(10.0, 1.0, 0.01).unwrap();
        let close = two_hemispherical_electrode_impedance_approx(
            0.0, 0.1, 0.1, 10.0, medium,
        ).unwrap();
        let far = two_hemispherical_electrode_impedance_approx(
            0.0, 0.1, 0.1, 1.0e8, medium,
        ).unwrap();
        assert!(close.re_ohms < far.re_ohms);
        assert!((far.re_ohms - 1.0 / (PI * 0.1 * 0.01)).abs() < 1e-5);
    }

    #[test]
    fn two_electrode_approximation_rejects_near_geometry() {
        let medium = Medium::new(10.0, 1.0, 0.01).unwrap();
        assert_eq!(
            two_hemispherical_electrode_impedance_approx(0.0, 0.1, 0.1, 0.5, medium),
            Err(ModelError::InvalidGeometry)
        );
    }

}