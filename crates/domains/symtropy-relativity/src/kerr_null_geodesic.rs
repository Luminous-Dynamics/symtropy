// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Separated Kerr null-geodesic potentials in geometrized, mass-normalized units.
//!
//! All coordinates and constants in this module are dimensionless with M=1:
//! - rho = r/M;
//! - chi = a/M, signed dimensionless spin;
//! - xi = L_z/(M E);
//! - eta = Q/(M^2 E^2), using the Carter convention
//!   Theta = Q + a^2 E^2 cos^2(theta) - L_z^2 cot^2(theta) for null rays.
//!
//! The null specialization (E=1, M=1) is
//! R(rho) = [rho^2 + chi^2 - chi*xi]^2
//!          - Delta*[eta + (xi-chi)^2],
//! Theta(theta) = eta + chi^2*cos^2(theta) - xi^2*cot^2(theta),
//! Delta = rho^2 - 2*rho + chi^2.
//!
//! This module evaluates the separated potentials and their radial
//! derivatives. It does not integrate geodesics, handle turning-point events,
//! construct observer tetrads, or compute a Jacobi map.

use core::fmt;

use super::{DimensionlessSpin, PhysicsDomainError};

/// Dimensionless Kerr null-geodesic constants for E=1 and M=1.
///
/// `xi` is the dimensionless axial impact parameter L_z/(M E), and `eta`
/// is the dimensionless Carter constant Q/(M^2 E^2) in the null convention
/// documented at module level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrNullGeodesicConstants {
    spin: DimensionlessSpin,
    xi: f64,
    eta: f64,
}

impl KerrNullGeodesicConstants {
    /// Construct from the signed dimensionless spin and normalized constants.
    pub fn new(
        spin: DimensionlessSpin,
        xi: f64,
        eta: f64,
    ) -> Result<Self, KerrNullPotentialError> {
        if !xi.is_finite() {
            return Err(KerrNullPotentialError::NonFinite("xi"));
        }
        if !eta.is_finite() {
            return Err(KerrNullPotentialError::NonFinite("eta"));
        }
        Ok(Self { spin, xi, eta })
    }

    /// Recover the separated constants from Bardeen screen coordinates at
    /// an asymptotic observer.
    ///
    /// Inputs alpha/M and beta/M use the declared screen handedness and spin
    /// axis convention. The standard mapping is
    /// xi = -alpha*sin(i),
    /// eta = beta^2 + (alpha^2-chi^2)*cos^2(i).
    ///
    /// This is not a finite-radius observer mapping; finite-radius observers
    /// require an explicitly constructed orthonormal tetrad.
    pub fn from_asymptotic_screen(
        spin: DimensionlessSpin,
        inclination_rad: f64,
        alpha_over_mass: f64,
        beta_over_mass: f64,
    ) -> Result<Self, KerrNullPotentialError> {
        validate_finite(inclination_rad, "inclination_rad")?;
        validate_finite(alpha_over_mass, "alpha_over_mass")?;
        validate_finite(beta_over_mass, "beta_over_mass")?;

        if !(0.0..=core::f64::consts::PI).contains(&inclination_rad) {
            return Err(KerrNullPotentialError::InclinationOutOfRange(
                inclination_rad,
            ));
        }

        let chi = spin.get();
        let sin_i = inclination_rad.sin();
        let cos_i = inclination_rad.cos();
        let xi = -alpha_over_mass * sin_i;
        let eta = beta_over_mass.powi(2)
            + (alpha_over_mass.powi(2) - chi.powi(2)) * cos_i.powi(2);

        Self::new(spin, xi, eta)
    }

    pub const fn spin(self) -> DimensionlessSpin {
        self.spin
    }

    pub const fn xi(self) -> f64 {
        self.xi
    }

    pub const fn eta(self) -> f64 {
        self.eta
    }

    /// Dimensionless Kerr Delta = rho^2 - 2*rho + chi^2.
    pub fn delta(self, rho: f64) -> Result<f64, KerrNullPotentialError> {
        validate_finite(rho, "rho")?;
        let value = rho * rho - 2.0 * rho + self.spin.get().powi(2);
        finite_result(value, "Delta")
    }

    /// Dimensionless radial potential R(rho) for a null geodesic.
    pub fn radial_potential(self, rho: f64) -> Result<f64, KerrNullPotentialError> {
        validate_finite(rho, "rho")?;
        let chi = self.spin.get();
        let p = rho * rho + chi * chi - chi * self.xi;
        let c = self.eta + (self.xi - chi).powi(2);
        let value = p.powi(2) - self.delta(rho)? * c;
        finite_result(value, "radial_potential")
    }

    /// First derivative dR/drho of the null radial potential.
    pub fn radial_potential_derivative(
        self,
        rho: f64,
    ) -> Result<f64, KerrNullPotentialError> {
        validate_finite(rho, "rho")?;
        let chi = self.spin.get();
        let p = rho * rho + chi * chi - chi * self.xi;
        let c = self.eta + (self.xi - chi).powi(2);
        let value = 4.0 * rho * p - 2.0 * (rho - 1.0) * c;
        finite_result(value, "radial_potential_derivative")
    }

    /// Second derivative d^2R/drho^2 of the null radial potential.
    pub fn radial_potential_second_derivative(
        self,
        rho: f64,
    ) -> Result<f64, KerrNullPotentialError> {
        validate_finite(rho, "rho")?;
        let chi = self.spin.get();
        let p = rho * rho + chi * chi - chi * self.xi;
        let c = self.eta + (self.xi - chi).powi(2);
        let value = 4.0 * p + 8.0 * rho * rho - 2.0 * c;
        finite_result(value, "radial_potential_second_derivative")
    }

    /// Dimensionless polar potential Theta(theta) for a null geodesic.
    ///
    /// The coordinate polar axis is excluded because cot(theta) is singular
    /// there. A negative value denotes a forbidden polar direction for the
    /// supplied conserved constants; it is not clamped to zero.
    pub fn polar_potential(self, theta_rad: f64) -> Result<f64, KerrNullPotentialError> {
        validate_finite(theta_rad, "theta_rad")?;
        if theta_rad <= 0.0 || theta_rad >= core::f64::consts::PI {
            return Err(KerrNullPotentialError::PolarAxisCoordinateSingularity);
        }

        let chi = self.spin.get();
        let cosine = theta_rad.cos();
        let sine = theta_rad.sin();
        let cotangent = cosine / sine;
        let value = self.eta + chi.powi(2) * cosine.powi(2)
            - self.xi.powi(2) * cotangent.powi(2);
        finite_result(value, "polar_potential")
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KerrNullPotentialError {
    PhysicsDomain(PhysicsDomainError),
    NonFinite(&'static str),
    InclinationOutOfRange(f64),
    PolarAxisCoordinateSingularity,
}

impl From<PhysicsDomainError> for KerrNullPotentialError {
    fn from(value: PhysicsDomainError) -> Self {
        Self::PhysicsDomain(value)
    }
}

impl fmt::Display for KerrNullPotentialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PhysicsDomain(error) => write!(f, "{error}"),
            Self::NonFinite(name) => write!(f, "{name} must be finite"),
            Self::InclinationOutOfRange(value) => {
                write!(f, "observer inclination {value} radians is outside [0, pi]")
            }
            Self::PolarAxisCoordinateSingularity => {
                write!(f, "Boyer-Lindquist polar potential excludes theta = 0 and pi")
            }
        }
    }
}

impl std::error::Error for KerrNullPotentialError {}

fn validate_finite(value: f64, name: &'static str) -> Result<(), KerrNullPotentialError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(KerrNullPotentialError::NonFinite(name))
    }
}

fn finite_result(value: f64, name: &'static str) -> Result<f64, KerrNullPotentialError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(KerrNullPotentialError::NonFinite(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spin(value: f64) -> DimensionlessSpin {
        DimensionlessSpin::new(value).unwrap()
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual:.16e}, expected={expected:.16e}, tolerance={tolerance:.3e}"
        );
    }

    #[test]
    fn screen_mapping_uses_declared_bardeen_convention() {
        let constants = KerrNullGeodesicConstants::from_asymptotic_screen(
            spin(0.5),
            core::f64::consts::FRAC_PI_3,
            2.0,
            1.0,
        )
        .unwrap();

        close(constants.xi(), -3.0_f64.sqrt(), 2.0e-15);
        close(constants.eta(), 1.9375, 2.0e-15);
    }

    #[test]
    fn null_radial_potential_and_derivatives_match_hand_calculation() {
        let constants = KerrNullGeodesicConstants::new(spin(0.5), 2.0, 3.0).unwrap();

        close(constants.delta(6.0).unwrap(), 24.25, 1.0e-14);
        close(constants.radial_potential(6.0).unwrap(), 1115.25, 1.0e-12);
        close(
            constants.radial_potential_derivative(6.0).unwrap(),
            793.5,
            1.0e-12,
        );
        close(
            constants.radial_potential_second_derivative(6.0).unwrap(),
            418.5,
            1.0e-12,
        );
    }

    #[test]
    fn null_polar_potential_includes_the_spin_cosine_term() {
        let constants = KerrNullGeodesicConstants::new(spin(0.5), 2.0, 3.0).unwrap();

        close(
            constants.polar_potential(core::f64::consts::FRAC_PI_2).unwrap(),
            3.0,
            1.0e-14,
        );
        close(
            constants.polar_potential(core::f64::consts::FRAC_PI_3).unwrap(),
            1.729_166_666_666_666_7,
            2.0e-14,
        );
    }

    #[test]
    fn schwarzschild_polar_limit_does_not_depend_on_spin_term() {
        let constants = KerrNullGeodesicConstants::new(spin(0.0), 2.0, 3.0).unwrap();
        close(
            constants.polar_potential(core::f64::consts::FRAC_PI_3).unwrap(),
            5.0 / 3.0,
            2.0e-14,
        );
    }

    #[test]
    fn null_and_unit_mass_timelike_polar_forms_are_not_conflated() {
        let constants = KerrNullGeodesicConstants::new(spin(0.5), 2.0, 3.0).unwrap();
        let theta = core::f64::consts::FRAC_PI_3;
        let null_value = constants.polar_potential(theta).unwrap();
        let old_unit_mass_form = constants.eta()
            - theta.cos().powi(2)
                * (constants.spin().get().powi(2) * (1.0 - 1.0_f64.powi(2))
                    + constants.xi().powi(2) / theta.sin().powi(2));
        assert!((null_value - old_unit_mass_form).abs() > 1.0e-3);
    }

    #[test]
    fn rejects_non_finite_and_singular_screen_inputs() {
        assert!(matches!(
            KerrNullGeodesicConstants::new(spin(0.5), f64::NAN, 0.0),
            Err(KerrNullPotentialError::NonFinite("xi"))
        ));
        assert!(matches!(
            KerrNullGeodesicConstants::from_asymptotic_screen(
                spin(0.5),
                core::f64::consts::PI + 0.1,
                0.0,
                1.0
            ),
            Err(KerrNullPotentialError::InclinationOutOfRange(_))
        ));
        let constants = KerrNullGeodesicConstants::new(spin(0.5), 2.0, 3.0).unwrap();
        assert_eq!(
            constants.polar_potential(0.0),
            Err(KerrNullPotentialError::PolarAxisCoordinateSingularity)
        );
    }

    #[test]
    fn negative_polar_potential_is_not_clamped() {
        let constants = KerrNullGeodesicConstants::new(spin(0.5), 5.0, -2.0).unwrap();
        assert!(constants.polar_potential(core::f64::consts::FRAC_PI_2).unwrap() < 0.0);
    }
}
