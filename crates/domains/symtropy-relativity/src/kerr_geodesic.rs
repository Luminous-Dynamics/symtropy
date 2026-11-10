// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Separated null-geodesic equations for the Kerr exterior.
//!
//! All public quantities in this module use geometrized, dimensionless units:
//! - M = G * mass / c^2 is the length unit;
//! - r_bar = r / M, t_bar = c*t / M, and a_bar = a / M = chi;
//! - E = -p_t is normalized to one for asymptotic observer-screen rays;
//! - L_bar = L_z / M and Q_bar = Q / M^2;
//! - the Mino parameter and its derivatives are dimensionless.
//!
//! The Boyer-Lindquist separated first integrals are
//!
//! R(r) = [E(r^2+a^2)-a L]^2 - Delta * [Q+(L-aE)^2],
//! Theta(theta) = Q + a^2 E^2 cos^2(theta) - L^2 cot^2(theta),
//! Delta = r^2 - 2r + a^2.
//!
//! A finite-radius or non-static observer must use a separately constructed
//! orthonormal tetrad. The Bardeen screen map below applies only to an
//! asymptotic observer with the declared inclination convention.
//!
//! This module evaluates the separated equations and their local Mino-time
//! derivatives. It is not yet a stepper, a turning-point event detector, a
//! Jacobi-field solver, or a wave-optics implementation. Direction signs are
//! explicit inputs so an integrator cannot silently cross a turning point.

use core::fmt;

use super::{KerrBlackHole, PhysicsDomainError};

/// Direction of motion in a separated radial or polar coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KerrMinoDirection {
    Increasing,
    Decreasing,
}

impl KerrMinoDirection {
    const fn sign(self) -> f64 {
        match self {
            Self::Increasing => 1.0,
            Self::Decreasing => -1.0,
        }
    }
}

/// A Bardeen screen position for an asymptotic observer.
///
/// The screen coordinates alpha and beta are dimensionless, in units of the
/// black-hole gravitational length M. Inclination is measured from the
/// positive spin axis and must be strictly between zero and pi. The two
/// screen axes and their handedness must be kept fixed by the caller.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrObserverScreen {
    pub alpha_over_m: f64,
    pub beta_over_m: f64,
    pub inclination_rad: f64,
}

impl KerrObserverScreen {
    pub fn new(
        alpha_over_m: f64,
        beta_over_m: f64,
        inclination_rad: f64,
    ) -> Result<Self, KerrGeodesicError> {
        if !alpha_over_m.is_finite() {
            return Err(KerrGeodesicError::NonFinite("alpha_over_m"));
        }
        if !beta_over_m.is_finite() {
            return Err(KerrGeodesicError::NonFinite("beta_over_m"));
        }
        validate_inclination(inclination_rad)?;
        Ok(Self {
            alpha_over_m,
            beta_over_m,
            inclination_rad,
        })
    }
}

/// Dimensionless separated constants for a future-directed null Kerr ray.
///
/// These are E=-p_t, L_bar=L_z/M and Q_bar=Q/M^2. E must be positive.
/// Carter's constant is not forced to be nonnegative: its admissible range
/// depends on the ray and observer inclination and is checked through Theta.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrNullConstants {
    pub energy: f64,
    pub angular_momentum_z_over_m: f64,
    pub carter_constant_over_m2: f64,
}

impl KerrNullConstants {
    pub fn new(
        energy: f64,
        angular_momentum_z_over_m: f64,
        carter_constant_over_m2: f64,
    ) -> Result<Self, KerrGeodesicError> {
        if !energy.is_finite() || energy <= 0.0 {
            return Err(KerrGeodesicError::InvalidEnergy(energy));
        }
        if !angular_momentum_z_over_m.is_finite() {
            return Err(KerrGeodesicError::NonFinite("angular_momentum_z_over_m"));
        }
        if !carter_constant_over_m2.is_finite() {
            return Err(KerrGeodesicError::NonFinite("carter_constant_over_m2"));
        }
        Ok(Self {
            energy,
            angular_momentum_z_over_m,
            carter_constant_over_m2,
        })
    }

    /// Construct E=1 constants from an asymptotic observer's Bardeen screen.
    ///
    /// xi = L_z/E = -alpha sin(i)
    /// eta = Q/E^2 = beta^2 + (alpha^2-a^2) cos^2(i)
    ///
    /// The resulting polar potential at the observer equals beta^2, providing
    /// a useful convention and realizability check.
    pub fn from_asymptotic_screen(
        hole: KerrBlackHole,
        screen: KerrObserverScreen,
    ) -> Result<Self, KerrGeodesicError> {
        validate_inclination(screen.inclination_rad)?;
        let sin_i = screen.inclination_rad.sin();
        let cos_i = screen.inclination_rad.cos();
        let spin = hole.spin().get();
        let alpha = screen.alpha_over_m;
        let beta = screen.beta_over_m;

        let angular_momentum = -alpha * sin_i;
        let carter = beta * beta + (alpha * alpha - spin * spin) * cos_i * cos_i;
        let constants = Self::new(1.0, angular_momentum, carter)?;
        let theta_at_observer = constants.polar_potential(spin, screen.inclination_rad)?;
        let expected = beta * beta;
        let scale = 1.0 + expected.abs() + theta_at_observer.abs();
        if (theta_at_observer - expected).abs() > 128.0 * f64::EPSILON * scale {
            return Err(KerrGeodesicError::ScreenMapInconsistent {
                actual_theta_potential: theta_at_observer,
                expected_beta_squared: expected,
            });
        }
        Ok(constants)
    }

    /// Recover asymptotic Bardeen screen coordinates for a selected sign of
    /// polar motion. The caller supplies the orientation convention for beta.
    pub fn to_asymptotic_screen(
        self,
        hole: KerrBlackHole,
        inclination_rad: f64,
        beta_direction: KerrMinoDirection,
    ) -> Result<KerrObserverScreen, KerrGeodesicError> {
        validate_inclination(inclination_rad)?;
        let sin_i = inclination_rad.sin();
        let cos_i = inclination_rad.cos();
        let spin = hole.spin().get();
        let xi = self.angular_momentum_z_over_m / self.energy;
        let eta = self.carter_constant_over_m2 / (self.energy * self.energy);
        let alpha = -xi / sin_i;
        let beta_squared = eta + spin * spin * cos_i * cos_i - xi * xi * cos_i * cos_i / (sin_i * sin_i);
        if !beta_squared.is_finite() {
            return Err(KerrGeodesicError::NonFinite("screen_beta_squared"));
        }
        if beta_squared < 0.0 {
            return Err(KerrGeodesicError::NoRealAsymptoticScreen {
                beta_squared,
            });
        }
        let beta = beta_direction.sign() * beta_squared.sqrt();
        KerrObserverScreen::new(alpha, beta, inclination_rad)
    }

    /// Separated radial potential R(r_bar).
    ///
    /// The expression is analytic for any finite r_bar; exterior-domain
    /// enforcement belongs to the geodesic evaluator below.
    pub fn radial_potential(self, spin: f64, radius_over_m: f64) -> Result<f64, KerrGeodesicError> {
        validate_spin(spin)?;
        if !radius_over_m.is_finite() || radius_over_m <= 0.0 {
            return Err(KerrGeodesicError::InvalidRadius(radius_over_m));
        }
        let r = radius_over_m;
        let a = spin;
        let energy = self.energy;
        let angular_momentum = self.angular_momentum_z_over_m;
        let carter = self.carter_constant_over_m2;
        let delta = r * r - 2.0 * r + a * a;
        let p = energy * (r * r + a * a) - a * angular_momentum;
        let angular_term = carter + (angular_momentum - a * energy).powi(2);
        let value = p * p - delta * angular_term;
        if !value.is_finite() {
            return Err(KerrGeodesicError::NonFinite("radial_potential"));
        }
        Ok(value)
    }

    /// Derivative dR/dr_bar of the separated radial potential.
    pub fn radial_potential_derivative(
        self,
        spin: f64,
        radius_over_m: f64,
    ) -> Result<f64, KerrGeodesicError> {
        validate_spin(spin)?;
        if !radius_over_m.is_finite() || radius_over_m <= 0.0 {
            return Err(KerrGeodesicError::InvalidRadius(radius_over_m));
        }
        let r = radius_over_m;
        let a = spin;
        let energy = self.energy;
        let angular_momentum = self.angular_momentum_z_over_m;
        let p = energy * (r * r + a * a) - a * angular_momentum;
        let angular_term =
            self.carter_constant_over_m2 + (angular_momentum - a * energy).powi(2);
        let delta_derivative = 2.0 * (r - 1.0);
        let value = 4.0 * energy * r * p - delta_derivative * angular_term;
        if !value.is_finite() {
            return Err(KerrGeodesicError::NonFinite("radial_potential_derivative"));
        }
        Ok(value)
    }

    /// Separated polar potential Theta(theta_bar).
    pub fn polar_potential(
        self,
        spin: f64,
        theta_rad: f64,
    ) -> Result<f64, KerrGeodesicError> {
        validate_spin(spin)?;
        validate_polar_angle(theta_rad)?;
        let sin_theta = theta_rad.sin();
        let cos_theta = theta_rad.cos();
        let value = self.carter_constant_over_m2
            + spin * spin * self.energy * self.energy * cos_theta * cos_theta
            - self.angular_momentum_z_over_m.powi(2) * cos_theta * cos_theta
                / (sin_theta * sin_theta);
        if !value.is_finite() {
            return Err(KerrGeodesicError::NonFinite("polar_potential"));
        }
        Ok(value)
    }
}

/// Coordinates for the separated Kerr null-geodesic reference.
///
/// Coordinates are dimensionless: [ct/M, r/M, theta, phi].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrMinoState {
    pub time_over_m: f64,
    pub radius_over_m: f64,
    pub theta_rad: f64,
    pub azimuth_rad: f64,
}

impl KerrMinoState {
    pub const fn new(
        time_over_m: f64,
        radius_over_m: f64,
        theta_rad: f64,
        azimuth_rad: f64,
    ) -> Self {
        Self {
            time_over_m,
            radius_over_m,
            theta_rad,
            azimuth_rad,
        }
    }
}

/// First derivatives with respect to the declared dimensionless Mino parameter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrMinoDerivative {
    pub d_time_over_m: f64,
    pub d_radius_over_m: f64,
    pub d_theta_rad: f64,
    pub d_azimuth_rad: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KerrGeodesicError {
    PhysicsDomain(PhysicsDomainError),
    NonFinite(&'static str),
    InvalidEnergy(f64),
    InvalidSpin(f64),
    InvalidRadius(f64),
    InvalidInclination(f64),
    InvalidPolarAngle(f64),
    RadiusAtOrInsideHorizon {
        radius_over_m: f64,
        horizon_over_m: f64,
    },
    ForbiddenRadialRegion {
        radius_over_m: f64,
        radial_potential: f64,
    },
    ForbiddenPolarRegion {
        theta_rad: f64,
        polar_potential: f64,
    },
    BoyerLindquistTurningSingularity {
        delta: f64,
    },
    NoRealAsymptoticScreen {
        beta_squared: f64,
    },
    ScreenMapInconsistent {
        actual_theta_potential: f64,
        expected_beta_squared: f64,
    },
}

impl From<PhysicsDomainError> for KerrGeodesicError {
    fn from(value: PhysicsDomainError) -> Self {
        Self::PhysicsDomain(value)
    }
}

impl fmt::Display for KerrGeodesicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PhysicsDomain(error) => write!(f, "{error}"),
            Self::NonFinite(name) => write!(f, "derived Kerr geodesic {name} is non-finite"),
            Self::InvalidEnergy(value) => {
                write!(f, "Kerr null-ray energy must be finite and positive, got {value}")
            }
            Self::InvalidSpin(value) => {
                write!(f, "dimensionless Kerr spin {value} is outside [-1, 1]")
            }
            Self::InvalidRadius(value) => {
                write!(f, "dimensionless Kerr radius {value} must be finite and positive")
            }
            Self::InvalidInclination(value) => {
                write!(f, "asymptotic observer inclination {value} must lie in (0, pi)")
            }
            Self::InvalidPolarAngle(value) => {
                write!(f, "Boyer-Lindquist polar angle {value} must lie in (0, pi)")
            }
            Self::RadiusAtOrInsideHorizon { radius_over_m, horizon_over_m } => write!(
                f,
                "Boyer-Lindquist exterior radius {radius_over_m} must exceed horizon {horizon_over_m}"
            ),
            Self::ForbiddenRadialRegion { radius_over_m, radial_potential } => write!(
                f,
                "radial potential {radial_potential} is negative at r/M={radius_over_m}"
            ),
            Self::ForbiddenPolarRegion { theta_rad, polar_potential } => write!(
                f,
                "polar potential {polar_potential} is negative at theta={theta_rad}"
            ),
            Self::BoyerLindquistTurningSingularity { delta } => {
                write!(f, "Boyer-Lindquist Mino equations are singular at Delta={delta}")
            }
            Self::NoRealAsymptoticScreen { beta_squared } => {
                write!(f, "constants do not map to a real asymptotic screen: beta^2={beta_squared}")
            }
            Self::ScreenMapInconsistent { actual_theta_potential, expected_beta_squared } => write!(
                f,
                "screen map gives Theta={actual_theta_potential}, expected beta^2={expected_beta_squared}"
            ),
        }
    }
}

impl std::error::Error for KerrGeodesicError {}

/// Separated Kerr null geodesic evaluator on the exterior Boyer-Lindquist chart.
///
/// This is a first-integral reference kernel, not a full ray integrator. It
/// validates both separated potentials and returns one Mino derivative for
/// explicit radial/polar branch signs. A caller must detect a turning point
/// and switch the corresponding branch; this type never hides a sign flip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrSeparatedNullGeodesic {
    hole: KerrBlackHole,
    constants: KerrNullConstants,
}

impl KerrSeparatedNullGeodesic {
    pub const fn new(hole: KerrBlackHole, constants: KerrNullConstants) -> Self {
        Self { hole, constants }
    }

    pub const fn black_hole(self) -> KerrBlackHole {
        self.hole
    }

    pub const fn constants(self) -> KerrNullConstants {
        self.constants
    }

    pub fn radial_potential(self, radius_over_m: f64) -> Result<f64, KerrGeodesicError> {
        self.constants.radial_potential(self.hole.spin().get(), radius_over_m)
    }

    pub fn radial_potential_derivative(
        self,
        radius_over_m: f64,
    ) -> Result<f64, KerrGeodesicError> {
        self.constants
            .radial_potential_derivative(self.hole.spin().get(), radius_over_m)
    }

    pub fn polar_potential(self, theta_rad: f64) -> Result<f64, KerrGeodesicError> {
        self.constants.polar_potential(self.hole.spin().get(), theta_rad)
    }

    /// Evaluate the dimensionless Mino-time derivative in the exterior chart.
    ///
    /// The equations use
    /// P = E(r^2+a^2)-aL,
    /// dt/dgamma = a(L-aE sin^2(theta)) + (r^2+a^2)P/Delta,
    /// dphi/dgamma = L/sin^2(theta)-aE+aP/Delta.
    ///
    /// No turning-point branch is inferred. Negative separated potentials
    /// return an error instead of being clamped to zero.
    pub fn mino_derivative(
        self,
        state: KerrMinoState,
        radial_direction: KerrMinoDirection,
        polar_direction: KerrMinoDirection,
    ) -> Result<KerrMinoDerivative, KerrGeodesicError> {
        for (name, value) in [
            ("time_over_m", state.time_over_m),
            ("radius_over_m", state.radius_over_m),
            ("theta_rad", state.theta_rad),
            ("azimuth_rad", state.azimuth_rad),
        ] {
            if !value.is_finite() {
                return Err(KerrGeodesicError::NonFinite(name));
            }
        }
        validate_polar_angle(state.theta_rad)?;

        let r = state.radius_over_m;
        let theta = state.theta_rad;
        let a = self.hole.spin().get();
        let horizon = self.hole.outer_horizon_radius_m() / self.hole.gravitational_radius_m();
        if r <= horizon {
            return Err(KerrGeodesicError::RadiusAtOrInsideHorizon {
                radius_over_m: r,
                horizon_over_m: horizon,
            });
        }

        let delta = r * r - 2.0 * r + a * a;
        if !delta.is_finite() || delta <= 0.0 {
            return Err(KerrGeodesicError::BoyerLindquistTurningSingularity { delta });
        }

        let radial = self.radial_potential(r)?;
        if radial < 0.0 {
            return Err(KerrGeodesicError::ForbiddenRadialRegion {
                radius_over_m: r,
                radial_potential: radial,
            });
        }
        let polar = self.polar_potential(theta)?;
        if polar < 0.0 {
            return Err(KerrGeodesicError::ForbiddenPolarRegion {
                theta_rad: theta,
                polar_potential: polar,
            });
        }

        let energy = self.constants.energy;
        let angular_momentum = self.constants.angular_momentum_z_over_m;
        let sin_theta = theta.sin();
        let sin_squared = sin_theta * sin_theta;
        let p = energy * (r * r + a * a) - a * angular_momentum;
        let d_time = a * (angular_momentum - a * energy * sin_squared)
            + (r * r + a * a) * p / delta;
        let d_azimuth = angular_momentum / sin_squared - a * energy + a * p / delta;
        let derivative = KerrMinoDerivative {
            d_time_over_m: d_time,
            d_radius_over_m: radial_direction.sign() * radial.sqrt(),
            d_theta_rad: polar_direction.sign() * polar.sqrt(),
            d_azimuth_rad: d_azimuth,
        };

        if !derivative.d_time_over_m.is_finite()
            || !derivative.d_radius_over_m.is_finite()
            || !derivative.d_theta_rad.is_finite()
            || !derivative.d_azimuth_rad.is_finite()
        {
            return Err(KerrGeodesicError::NonFinite("mino_derivative"));
        }
        Ok(derivative)
    }
}

fn validate_spin(spin: f64) -> Result<(), KerrGeodesicError> {
    if !spin.is_finite() || !(-1.0..=1.0).contains(&spin) {
        return Err(KerrGeodesicError::InvalidSpin(spin));
    }
    Ok(())
}

fn validate_inclination(inclination: f64) -> Result<(), KerrGeodesicError> {
    if !inclination.is_finite()
        || inclination <= 0.0
        || inclination >= core::f64::consts::PI
    {
        return Err(KerrGeodesicError::InvalidInclination(inclination));
    }
    Ok(())
}

fn validate_polar_angle(theta: f64) -> Result<(), KerrGeodesicError> {
    if !theta.is_finite() || theta <= 0.0 || theta >= core::f64::consts::PI {
        return Err(KerrGeodesicError::InvalidPolarAngle(theta));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DimensionlessSpin, MassKg};
    use crate::light_ring::{KerrEquatorialPhotonOrbit, OrbitSense};

    const SOLAR_MASS_KG: f64 = 1.988_47e30;

    fn hole(spin: f64) -> KerrBlackHole {
        KerrBlackHole::new(
            MassKg::new(SOLAR_MASS_KG).unwrap(),
            DimensionlessSpin::new(spin).unwrap(),
        )
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance * (1.0 + expected.abs()),
            "actual={actual:.16e}, expected={expected:.16e}, tolerance={tolerance:.3e}"
        );
    }

    #[test]
    fn asymptotic_screen_map_round_trips_in_equatorial_and_inclined_views() {
        for inclination in [
            core::f64::consts::FRAC_PI_2,
            core::f64::consts::FRAC_PI_3,
        ] {
            for (alpha, beta) in [(-8.0, 1.5), (0.0, 2.0), (5.0, -0.75)] {
                let screen = KerrObserverScreen::new(alpha, beta, inclination).unwrap();
                let constants = KerrNullConstants::from_asymptotic_screen(hole(0.7), screen)
                    .unwrap();
                let beta_direction = if beta.is_sign_negative() {
                    KerrMinoDirection::Decreasing
                } else {
                    KerrMinoDirection::Increasing
                };
                let recovered = constants
                    .to_asymptotic_screen(hole(0.7), inclination, beta_direction)
                    .unwrap();
                close(recovered.alpha_over_m, alpha, 2e-14);
                close(recovered.beta_over_m, beta, 2e-14);
            }
        }
    }

    #[test]
    fn schwarzschild_critical_impact_is_a_double_radial_root() {
        let b_critical = 3.0 * 3.0_f64.sqrt();
        let constants = KerrNullConstants::new(1.0, b_critical, 0.0).unwrap();
        let r = 3.0;
        close(constants.radial_potential(0.0, r).unwrap(), 0.0, 2e-14);
        close(
            constants.radial_potential_derivative(0.0, r).unwrap(),
            0.0,
            2e-14,
        );
    }

    #[test]
    fn kerr_prograde_and_retrograde_light_rings_are_null_radial_double_roots() {
        for spin in [-0.9, -0.5, 0.0, 0.5, 0.9] {
            for sense in [OrbitSense::Prograde, OrbitSense::Retrograde] {
                let orbit = KerrEquatorialPhotonOrbit::calculate(hole(spin), sense).unwrap();
                let Some(impact_parameter_m) = orbit.impact_parameter_m else {
                    continue;
                };
                let rg = hole(spin).gravitational_radius_m();
                let constants = KerrNullConstants::new(
                    1.0,
                    impact_parameter_m / rg,
                    0.0,
                )
                .unwrap();
                close(
                    constants.radial_potential(spin, orbit.radius_m / rg).unwrap(),
                    0.0,
                    5e-10,
                );
                close(
                    constants
                        .radial_potential_derivative(spin, orbit.radius_m / rg)
                        .unwrap(),
                    0.0,
                    5e-10,
                );
            }
        }
    }

    #[test]
    fn radial_schwarzschild_null_ray_has_expected_boyer_lindquist_time_ratio() {
        let black_hole = hole(0.0);
        let geodesic = KerrSeparatedNullGeodesic::new(
            black_hole,
            KerrNullConstants::new(1.0, 0.0, 0.0).unwrap(),
        );
        let state = KerrMinoState::new(0.0, 10.0, core::f64::consts::FRAC_PI_2, 0.0);
        let derivative = geodesic
            .mino_derivative(
                state,
                KerrMinoDirection::Increasing,
                KerrMinoDirection::Increasing,
            )
            .unwrap();
        let expected_ratio = 1.0 / (1.0 - 2.0 / 10.0);
        close(
            derivative.d_time_over_m / derivative.d_radius_over_m,
            expected_ratio,
            2e-14,
        );
    }

    #[test]
    fn forbidden_radial_region_is_not_clamped_to_a_turning_point() {
        let constants =
            KerrNullConstants::new(1.0, 3.0 * 3.0_f64.sqrt(), 0.0).unwrap();
        let geodesic = KerrSeparatedNullGeodesic::new(hole(0.0), constants);
        let state = KerrMinoState::new(0.0, 2.5, core::f64::consts::FRAC_PI_2, 0.0);
        assert!(matches!(
            geodesic.mino_derivative(
                state,
                KerrMinoDirection::Increasing,
                KerrMinoDirection::Increasing
            ),
            Err(KerrGeodesicError::ForbiddenRadialRegion { .. })
        ));
    }

    #[test]
    fn horizon_and_polar_axis_are_explicitly_rejected() {
        let black_hole = hole(0.8);
        let constants = KerrNullConstants::new(1.0, 2.0, 1.0).unwrap();
        let geodesic = KerrSeparatedNullGeodesic::new(black_hole, constants);
        let horizon_over_m =
            black_hole.outer_horizon_radius_m() / black_hole.gravitational_radius_m();
        let horizon_state = KerrMinoState::new(
            0.0,
            horizon_over_m,
            core::f64::consts::FRAC_PI_2,
            0.0,
        );
        assert!(matches!(
            geodesic.mino_derivative(
                horizon_state,
                KerrMinoDirection::Increasing,
                KerrMinoDirection::Increasing
            ),
            Err(KerrGeodesicError::RadiusAtOrInsideHorizon { .. })
        ));
        assert!(matches!(
            constants.polar_potential(0.8, 0.0),
            Err(KerrGeodesicError::InvalidPolarAngle(_))
        ));
    }

    #[test]
    fn screen_map_rejects_polar_axis_observer_until_a_regular_tetrad_limit_is_defined() {
        assert!(matches!(
            KerrObserverScreen::new(1.0, 1.0, 0.0),
            Err(KerrGeodesicError::InvalidInclination(_))
        ));
    }

    #[test]
    fn separated_derivative_preserves_the_explicit_branch_signs() {
        let geodesic = KerrSeparatedNullGeodesic::new(
            hole(0.5),
            KerrNullConstants::new(1.0, 2.0, 3.0).unwrap(),
        );
        let state = KerrMinoState::new(0.0, 10.0, 1.0, 0.0);
        let outward = geodesic
            .mino_derivative(
                state,
                KerrMinoDirection::Increasing,
                KerrMinoDirection::Increasing,
            )
            .unwrap();
        let inward = geodesic
            .mino_derivative(
                state,
                KerrMinoDirection::Decreasing,
                KerrMinoDirection::Increasing,
            )
            .unwrap();
        close(outward.d_radius_over_m, -inward.d_radius_over_m, 2e-14);
        close(outward.d_time_over_m, inward.d_time_over_m, 2e-14);
        close(outward.d_azimuth_rad, inward.d_azimuth_rad, 2e-14);
    }
}
