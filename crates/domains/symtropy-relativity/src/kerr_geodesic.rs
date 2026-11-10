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

use super::{
    KerrBlackHole, PhysicsDomainError,
    autodiff::{DifferentiatedMetric4, JetError},
    metric::{MetricComponents4, MetricError},
};

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

    /// Derivative dTheta/dtheta of the separated polar potential.
    pub fn polar_potential_derivative(
        self,
        spin: f64,
        theta_rad: f64,
    ) -> Result<f64, KerrGeodesicError> {
        validate_spin(spin)?;
        validate_polar_angle(theta_rad)?;
        let sin_theta = theta_rad.sin();
        let cos_theta = theta_rad.cos();
        let value = -2.0 * spin * spin * self.energy * self.energy * cos_theta * sin_theta
            + 2.0 * self.angular_momentum_z_over_m.powi(2) * cos_theta
                / (sin_theta * sin_theta * sin_theta);
        if !value.is_finite() {
            return Err(KerrGeodesicError::NonFinite("polar_potential_derivative"));
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


/// Second-order state for turning-point-regular Mino evolution.
///
/// The first integrals impose
/// `radial_velocity_over_m^2 = R(r/M)` and
/// `polar_velocity_rad^2 = Theta(theta)`. Unlike the signed square-root
/// first-order form, this state evolves continuously through a simple
/// radial/polar turning point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrMinoDynamicState {
    pub coordinates: KerrMinoState,
    pub radial_velocity_over_m: f64,
    pub polar_velocity_rad: f64,
}

impl KerrMinoDynamicState {
    pub const fn new(
        coordinates: KerrMinoState,
        radial_velocity_over_m: f64,
        polar_velocity_rad: f64,
    ) -> Self {
        Self {
            coordinates,
            radial_velocity_over_m,
            polar_velocity_rad,
        }
    }
}

/// Derivative of the second-order Mino state with respect to dimensionless gamma.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrMinoDynamicDerivative {
    pub d_time_over_m: f64,
    pub d_radius_over_m: f64,
    pub d_radial_velocity_over_m: f64,
    pub d_theta_rad: f64,
    pub d_polar_velocity_rad: f64,
    pub d_azimuth_rad: f64,
}

/// Residuals of the separated first-integral constraints.
///
/// A physical state satisfies both residuals equal to zero. During numerical
/// integration they provide independent diagnostics for constraint drift.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrMinoConstraintResiduals {
    pub radial: f64,
    pub polar: f64,
}


/// Canonical state for Kerr Hamiltonian evolution in dimensionless variables.
///
/// Coordinates are [ct/M, r/M, theta, phi]. Covariant momenta are
/// [-E, M*p_r, p_theta/M, L_z/M], with the geodesic action divided by M.
/// The Hamiltonian is H = 1/2 g_bar^(mu nu) p_bar_mu p_bar_nu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrCanonicalState {
    pub coordinates: KerrMinoState,
    pub momenta: [f64; 4],
}

/// Canonical Hamiltonian vector field (dx^mu/dlambda_bar, dp_mu/dlambda_bar).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrCanonicalDerivative {
    pub coordinates: [f64; 4],
    pub momenta: [f64; 4],
}

/// Hamiltonian and recovered Killing/Carter invariants at a canonical state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrCanonicalEvaluation {
    pub derivative: KerrCanonicalDerivative,
    pub hamiltonian: f64,
    pub energy: f64,
    pub angular_momentum_z_over_m: f64,
    pub carter_constant_over_m2: f64,
}

/// Independent canonical Hamiltonian evaluator for exterior Kerr rays.
///
/// The metric and its first derivatives are generated through the existing
/// second-order Jet2 AD path. The geodesic equations are then evaluated from
/// H=1/2*g^{mu nu}p_mu p_nu. This is independent of the separated Carter
/// radial/polar potential implementation, although both describe the same
/// Kerr metric and are intended to be cross-checked rather than treated as
/// independent physical models.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrCanonicalHamiltonian {
    hole: KerrBlackHole,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KerrGeodesicError {
    PhysicsDomain(PhysicsDomainError),
    Metric(MetricError),
    Autodiff(JetError),
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
    NoRegularIngoingBranch {
        p_plus: f64,
    },
}

impl From<PhysicsDomainError> for KerrGeodesicError {
    fn from(value: PhysicsDomainError) -> Self {
        Self::PhysicsDomain(value)
    }
}

impl From<MetricError> for KerrGeodesicError {
    fn from(value: MetricError) -> Self {
        Self::Metric(value)
    }
}

impl From<JetError> for KerrGeodesicError {
    fn from(value: JetError) -> Self {
        Self::Autodiff(value)
    }
}

impl fmt::Display for KerrGeodesicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PhysicsDomain(error) => write!(f, "{error}"),
            Self::Metric(error) => write!(f, "{error}"),
            Self::Autodiff(error) => write!(f, "{error}"),
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
            Self::NoRegularIngoingBranch { p_plus } => write!(
                f,
                "future-ingoing Kerr chart requires P+sqrt(R)>0, got {p_plus}"
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


    /// Evaluate the smooth second-order separated Mino equations.
    ///
    /// Differentiating the first integrals gives
    /// `d2(r/M)/dgamma2 = (1/2) dR/d(r/M)` and
    /// `d2(theta)/dgamma2 = (1/2) dTheta/dtheta`.
    /// This form remains regular at simple radial and polar turning points:
    /// the corresponding velocity passes through zero while its acceleration
    /// determines the continuation. It does not need an explicit square-root
    /// sign flip. The Boyer-Lindquist chart still excludes the horizon.
    pub fn mino_dynamic_derivative(
        self,
        state: KerrMinoDynamicState,
    ) -> Result<KerrMinoDynamicDerivative, KerrGeodesicError> {
        let coordinates = state.coordinates;
        for (name, value) in [
            ("time_over_m", coordinates.time_over_m),
            ("radius_over_m", coordinates.radius_over_m),
            ("theta_rad", coordinates.theta_rad),
            ("azimuth_rad", coordinates.azimuth_rad),
            ("radial_velocity_over_m", state.radial_velocity_over_m),
            ("polar_velocity_rad", state.polar_velocity_rad),
        ] {
            if !value.is_finite() {
                return Err(KerrGeodesicError::NonFinite(name));
            }
        }
        validate_polar_angle(coordinates.theta_rad)?;
        let r = coordinates.radius_over_m;
        let theta = coordinates.theta_rad;
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

        let radial_acceleration = 0.5 * self.radial_potential_derivative(r)?;
        let polar_acceleration =
            0.5 * self.constants.polar_potential_derivative(a, theta)?;
        let energy = self.constants.energy;
        let angular_momentum = self.constants.angular_momentum_z_over_m;
        let sin_theta = theta.sin();
        let sin_squared = sin_theta * sin_theta;
        let p = energy * (r * r + a * a) - a * angular_momentum;
        let d_time = a * (angular_momentum - a * energy * sin_squared)
            + (r * r + a * a) * p / delta;
        let d_azimuth = angular_momentum / sin_squared - a * energy + a * p / delta;
        let derivative = KerrMinoDynamicDerivative {
            d_time_over_m: d_time,
            d_radius_over_m: state.radial_velocity_over_m,
            d_radial_velocity_over_m: radial_acceleration,
            d_theta_rad: state.polar_velocity_rad,
            d_polar_velocity_rad: polar_acceleration,
            d_azimuth_rad: d_azimuth,
        };
        if [
            derivative.d_time_over_m,
            derivative.d_radius_over_m,
            derivative.d_radial_velocity_over_m,
            derivative.d_theta_rad,
            derivative.d_polar_velocity_rad,
            derivative.d_azimuth_rad,
        ]
        .iter()
        .any(|value| !value.is_finite())
        {
            return Err(KerrGeodesicError::NonFinite("mino_dynamic_derivative"));
        }
        Ok(derivative)
    }

    /// Evaluate the radial/polar first-integral residuals for a dynamic state.
    pub fn mino_constraint_residuals(
        self,
        state: KerrMinoDynamicState,
    ) -> Result<KerrMinoConstraintResiduals, KerrGeodesicError> {
        for (name, value) in [
            ("radial_velocity_over_m", state.radial_velocity_over_m),
            ("polar_velocity_rad", state.polar_velocity_rad),
        ] {
            if !value.is_finite() {
                return Err(KerrGeodesicError::NonFinite(name));
            }
        }
        let r = state.coordinates.radius_over_m;
        let theta = state.coordinates.theta_rad;
        let radial = state.radial_velocity_over_m.powi(2) - self.radial_potential(r)?;
        let polar =
            state.polar_velocity_rad.powi(2) - self.polar_potential(theta)?;
        if !radial.is_finite() || !polar.is_finite() {
            return Err(KerrGeodesicError::NonFinite("mino_constraint_residual"));
        }
        Ok(KerrMinoConstraintResiduals { radial, polar })
    }
 }


impl KerrCanonicalHamiltonian {
    pub const fn new(hole: KerrBlackHole) -> Self {
        Self { hole }
    }

    pub const fn black_hole(self) -> KerrBlackHole {
        self.hole
    }

    /// Construct canonical momenta from separated constants and explicit
    /// radial/polar branch signs at one exterior event.
    ///
    /// For the declared normalization:
    /// p_t=-E, p_r=sign_r*sqrt(R)/Delta,
    /// p_theta=sign_theta*sqrt(Theta), p_phi=L_z/M.
    pub fn state_from_separated(
        self,
        coordinates: KerrMinoState,
        constants: KerrNullConstants,
        radial_direction: KerrMinoDirection,
        polar_direction: KerrMinoDirection,
    ) -> Result<KerrCanonicalState, KerrGeodesicError> {
        let geodesic = KerrSeparatedNullGeodesic::new(self.hole, constants);
        let r = coordinates.radius_over_m;
        let theta = coordinates.theta_rad;
        if !r.is_finite() || r <= 0.0 {
            return Err(KerrGeodesicError::InvalidRadius(r));
        }
        let horizon = self.hole.outer_horizon_radius_m()
            / self.hole.gravitational_radius_m();
        if r <= horizon {
            return Err(KerrGeodesicError::RadiusAtOrInsideHorizon {
                radius_over_m: r,
                horizon_over_m: horizon,
            });
        }
        let delta = r * r - 2.0 * r + self.hole.spin().get().powi(2);
        if !delta.is_finite() || delta <= 0.0 {
            return Err(KerrGeodesicError::BoyerLindquistTurningSingularity { delta });
        }
        let radial = geodesic.radial_potential(r)?;
        if radial < 0.0 {
            return Err(KerrGeodesicError::ForbiddenRadialRegion {
                radius_over_m: r,
                radial_potential: radial,
            });
        }
        let polar = geodesic.polar_potential(theta)?;
        if polar < 0.0 {
            return Err(KerrGeodesicError::ForbiddenPolarRegion {
                theta_rad: theta,
                polar_potential: polar,
            });
        }
        let momenta = [
            -constants.energy,
            radial_direction.sign() * radial.sqrt() / delta,
            polar_direction.sign() * polar.sqrt(),
            constants.angular_momentum_z_over_m,
        ];
        if momenta.iter().any(|value| !value.is_finite()) {
            return Err(KerrGeodesicError::NonFinite("canonical_initial_momenta"));
        }
        Ok(KerrCanonicalState {
            coordinates,
            momenta,
        })
    }

    /// Construct canonical momenta for a future-directed inward ray in
    /// ingoing Kerr coordinates (v/M,r/M,theta,phi_tilde).
    ///
    /// The radial momentum is transformed from the inward Boyer-Lindquist
    /// branch using
    ///
    /// p_r^(in) = (P - sqrt(R))/Delta
    ///           = [Q+(L-aE)^2] / [P+sqrt(R)].
    ///
    /// The rationalized expression remains finite at the future horizon when
    /// P>0. This method is intended for future-directed rays that fall inward;
    /// it does not initialize the outgoing branch at the future horizon.
    pub fn ingoing_state_from_separated(
        self,
        coordinates: KerrMinoState,
        constants: KerrNullConstants,
        polar_direction: KerrMinoDirection,
    ) -> Result<KerrCanonicalState, KerrGeodesicError> {
        let r = coordinates.radius_over_m;
        if !r.is_finite() || r <= 0.0 {
            return Err(KerrGeodesicError::InvalidRadius(r));
        }
        validate_polar_angle(coordinates.theta_rad)?;
        let a = self.hole.spin().get();
        let energy = constants.energy;
        let angular_momentum = constants.angular_momentum_z_over_m;
        let radial_potential = constants.radial_potential(a, r)?;
        if radial_potential < 0.0 {
            return Err(KerrGeodesicError::ForbiddenRadialRegion {
                radius_over_m: r,
                radial_potential,
            });
        }
        let polar_potential = constants.polar_potential(a, coordinates.theta_rad)?;
        if polar_potential < 0.0 {
            return Err(KerrGeodesicError::ForbiddenPolarRegion {
                theta_rad: coordinates.theta_rad,
                polar_potential,
            });
        }
        let p = energy * (r * r + a * a) - a * angular_momentum;
        let root = radial_potential.sqrt();
        let p_plus = p + root;
        if !p_plus.is_finite() || p_plus <= 0.0 {
            return Err(KerrGeodesicError::NoRegularIngoingBranch { p_plus });
        }
        let angular_term =
            constants.carter_constant_over_m2 + (angular_momentum - a * energy).powi(2);
        let radial_momentum = angular_term / p_plus;
        let momenta = [
            -energy,
            radial_momentum,
            polar_direction.sign() * polar_potential.sqrt(),
            angular_momentum,
        ];
        if momenta.iter().any(|value| !value.is_finite()) {
            return Err(KerrGeodesicError::NonFinite("ingoing_canonical_initial_momenta"));
        }
        Ok(KerrCanonicalState {
            coordinates,
            momenta,
        })
    }

    /// Evaluate H and Hamilton's equations in the exterior Boyer–Lindquist chart.
    pub fn evaluate(
        self,
        state: KerrCanonicalState,
    ) -> Result<KerrCanonicalEvaluation, KerrGeodesicError> {
        let radius_m = state.coordinates.radius_over_m * self.hole.gravitational_radius_m();
        let differentiated =
            DifferentiatedMetric4::kerr_exterior(self.hole, radius_m, state.coordinates.theta_rad)?;
        self.evaluate_differentiated(state, differentiated)
    }

    /// Evaluate H and Hamilton's equations in horizon-regular ingoing Kerr
    /// coordinates (v/M,r/M,theta,phi_tilde). This chart is regular at and
    /// through the future outer horizon for future-ingoing rays.
    pub fn evaluate_ingoing(
        self,
        state: KerrCanonicalState,
    ) -> Result<KerrCanonicalEvaluation, KerrGeodesicError> {
        let radius_m = state.coordinates.radius_over_m * self.hole.gravitational_radius_m();
        let differentiated =
            DifferentiatedMetric4::kerr_ingoing(self.hole, radius_m, state.coordinates.theta_rad)?;
        self.evaluate_differentiated(state, differentiated)
    }

    fn evaluate_differentiated(
        self,
        state: KerrCanonicalState,
        differentiated: DifferentiatedMetric4,
    ) -> Result<KerrCanonicalEvaluation, KerrGeodesicError> {
        let coordinates = state.coordinates;
        for (name, value) in [
            ("canonical_time_over_m", coordinates.time_over_m),
            ("canonical_radius_over_m", coordinates.radius_over_m),
            ("canonical_theta_rad", coordinates.theta_rad),
            ("canonical_azimuth_rad", coordinates.azimuth_rad),
        ] {
            if !value.is_finite() {
                return Err(KerrGeodesicError::NonFinite(name));
            }
        }
        for (index, value) in state.momenta.iter().enumerate() {
            if !value.is_finite() {
                return Err(KerrGeodesicError::NonFinite(match index {
                    0 => "canonical_p_t",
                    1 => "canonical_p_r",
                    2 => "canonical_p_theta",
                    _ => "canonical_p_phi",
                }));
            }
        }
        validate_polar_angle(coordinates.theta_rad)?;
        let mass_length = self.hole.gravitational_radius_m();

        // If x_SI = M*[tbar,rbar,theta/M,phi/M], then
        // gbar = diag(1,1,1/M,1/M) * g_SI * diag(1,1,1/M,1/M).
        let factors = [1.0, 1.0, 1.0 / mass_length, 1.0 / mass_length];
        let mut covariant = [[0.0; 4]; 4];
        let mut radial_derivative = [[0.0; 4]; 4];
        let mut polar_derivative = [[0.0; 4]; 4];
        let source_metric = differentiated.metric.covariant();
        let source_first = differentiated.first.components();
        for mu in 0..4 {
            for nu in 0..4 {
                let scale = factors[mu] * factors[nu];
                covariant[mu][nu] = source_metric[mu][nu] * scale;
                radial_derivative[mu][nu] =
                    source_first[1][mu][nu] * mass_length * scale;
                polar_derivative[mu][nu] = source_first[2][mu][nu] * scale;
            }
        }
        let metric = MetricComponents4::new(covariant)?;
        let inverse = metric.inverse()?;

        let mut velocity = [0.0; 4];
        for mu in 0..4 {
            for nu in 0..4 {
                velocity[mu] += inverse[mu][nu] * state.momenta[nu];
            }
        }

        let mut momentum_derivative = [0.0; 4];
        // Stationarity and axisymmetry make dp_t/dlambda = dp_phi/dlambda = 0.
        for (coordinate, derivative) in [
            (1usize, &radial_derivative),
            (2usize, &polar_derivative),
        ] {
            let mut value = 0.0;
            for mu in 0..4 {
                for nu in 0..4 {
                    value += velocity[mu] * derivative[mu][nu] * velocity[nu];
                }
            }
            momentum_derivative[coordinate] = 0.5 * value;
        }

        let mut hamiltonian = 0.0;
        for mu in 0..4 {
            hamiltonian += state.momenta[mu] * velocity[mu];
        }
        hamiltonian *= 0.5;

        let energy = -state.momenta[0];
        let angular_momentum = state.momenta[3];
        let sin_theta = coordinates.theta_rad.sin();
        let cos_theta = coordinates.theta_rad.cos();
        let carter = state.momenta[2].powi(2)
            + cos_theta.powi(2)
                * (angular_momentum.powi(2) / sin_theta.powi(2)
                    - self.hole.spin().get().powi(2) * energy.powi(2));

        let values = [
            hamiltonian,
            energy,
            angular_momentum,
            carter,
            velocity[0],
            velocity[1],
            velocity[2],
            velocity[3],
            momentum_derivative[0],
            momentum_derivative[1],
            momentum_derivative[2],
            momentum_derivative[3],
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(KerrGeodesicError::NonFinite("canonical_hamiltonian_evaluation"));
        }

        Ok(KerrCanonicalEvaluation {
            derivative: KerrCanonicalDerivative {
                coordinates: velocity,
                momenta: momentum_derivative,
            },
            hamiltonian,
            energy,
            angular_momentum_z_over_m: angular_momentum,
            carter_constant_over_m2: carter,
        })
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
        let constants = KerrNullConstants::new(1.0, 6.0, 0.0).unwrap();
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
    fn forbidden_polar_region_is_not_clamped_to_zero() {
        let constants = KerrNullConstants::new(1.0, 4.0, 0.0).unwrap();
        assert!(matches!(
            constants.polar_potential(0.5, core::f64::consts::FRAC_PI_4),
            Ok(value) if value < 0.0
        ));
        let geodesic = KerrSeparatedNullGeodesic::new(hole(0.5), constants);
        let state = KerrMinoState::new(0.0, 10.0, core::f64::consts::FRAC_PI_4, 0.0);
        assert!(matches!(
            geodesic.mino_derivative(
                state,
                KerrMinoDirection::Increasing,
                KerrMinoDirection::Increasing
            ),
            Err(KerrGeodesicError::ForbiddenPolarRegion { .. })
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
    fn second_order_mino_system_is_regular_at_radial_turning_point() {
        let constants = KerrNullConstants::new(1.0, 6.0, 0.0).unwrap();
        let geodesic = KerrSeparatedNullGeodesic::new(hole(0.0), constants);
        // Outer turning point for Schwarzschild L_z/M=6, Q=0.
        let r_turn = 4.453_363_193_811_354;
        let state = KerrMinoDynamicState::new(
            KerrMinoState::new(0.0, r_turn, core::f64::consts::FRAC_PI_2, 0.0),
            0.0,
            0.0,
        );
        let residuals = geodesic.mino_constraint_residuals(state).unwrap();
        close(residuals.radial, 0.0, 2e-12);
        close(residuals.polar, 0.0, 2e-14);
        let derivative = geodesic.mino_dynamic_derivative(state).unwrap();
        assert!(derivative.d_radial_velocity_over_m > 0.0);
        assert_eq!(derivative.d_radius_over_m, 0.0);
    }

    #[test]
    fn second_order_mino_system_is_regular_at_polar_turning_point() {
        let theta_turn = core::f64::consts::FRAC_PI_4;
        let spin = 0.5;
        let angular_momentum = 4.0;
        let carter = angular_momentum * angular_momentum
            * theta_turn.cos().powi(2) / theta_turn.sin().powi(2)
            - spin * spin * theta_turn.cos().powi(2);
        let geodesic = KerrSeparatedNullGeodesic::new(
            hole(spin),
            KerrNullConstants::new(1.0, angular_momentum, carter).unwrap(),
        );
        let state = KerrMinoDynamicState::new(
            KerrMinoState::new(0.0, 10.0, theta_turn, 0.0),
            0.0,
            0.0,
        );
        let residuals = geodesic.mino_constraint_residuals(state).unwrap();
        close(residuals.polar, 0.0, 2e-14);
        let derivative = geodesic.mino_dynamic_derivative(state).unwrap();
        assert!(derivative.d_polar_velocity_rad > 0.0);
        assert_eq!(derivative.d_theta_rad, 0.0);
    }

    #[test]
    fn second_order_mino_system_recovers_schwarzschild_circular_photon_orbit() {
        let constants =
            KerrNullConstants::new(1.0, 3.0 * 3.0_f64.sqrt(), 0.0).unwrap();
        let geodesic = KerrSeparatedNullGeodesic::new(hole(0.0), constants);
        let state = KerrMinoDynamicState::new(
            KerrMinoState::new(0.0, 3.0, core::f64::consts::FRAC_PI_2, 0.0),
            0.0,
            0.0,
        );
        let residuals = geodesic.mino_constraint_residuals(state).unwrap();
        close(residuals.radial, 0.0, 2e-14);
        close(residuals.polar, 0.0, 2e-14);
        let derivative = geodesic.mino_dynamic_derivative(state).unwrap();
        close(derivative.d_radial_velocity_over_m, 0.0, 2e-14);
        close(derivative.d_polar_velocity_rad, 0.0, 2e-14);
    }


    #[test]
    fn separated_and_independent_canonical_hamiltonian_paths_agree() {
        for spin in [0.0, 0.5, 0.9] {
            let black_hole = hole(spin);
            let constants = KerrNullConstants::new(1.0, 2.0, 3.0).unwrap();
            let coordinates =
                KerrMinoState::new(0.0, 10.0, 1.0, 0.0);
            let separated = KerrSeparatedNullGeodesic::new(black_hole, constants);
            let canonical = KerrCanonicalHamiltonian::new(black_hole);
            let state = canonical
                .state_from_separated(
                    coordinates,
                    constants,
                    KerrMinoDirection::Increasing,
                    KerrMinoDirection::Increasing,
                )
                .unwrap();
            let hamiltonian = canonical.evaluate(state).unwrap();
            let mino = separated
                .mino_derivative(
                    coordinates,
                    KerrMinoDirection::Increasing,
                    KerrMinoDirection::Increasing,
                )
                .unwrap();
            let r = coordinates.radius_over_m;
            let theta = coordinates.theta_rad;
            let sigma = r * r + spin * spin * theta.cos().powi(2);
            close(hamiltonian.hamiltonian, 0.0, 2e-11);
            close(hamiltonian.energy, 1.0, 2e-14);
            close(hamiltonian.angular_momentum_z_over_m, 2.0, 2e-14);
            close(hamiltonian.carter_constant_over_m2, 3.0, 2e-12);
            close(hamiltonian.derivative.coordinates[0] * sigma, mino.d_time_over_m, 2e-11);
            close(hamiltonian.derivative.coordinates[1] * sigma, mino.d_radius_over_m, 2e-11);
            close(hamiltonian.derivative.coordinates[2] * sigma, mino.d_theta_rad, 2e-11);
            close(hamiltonian.derivative.coordinates[3] * sigma, mino.d_azimuth_rad, 2e-11);
            close(hamiltonian.derivative.momenta[0], 0.0, 2e-14);
            close(hamiltonian.derivative.momenta[3], 0.0, 2e-14);
        }
    }

    #[test]
    fn bl_and_ingoing_hamiltonian_vector_fields_match_outside_horizon() {
        for spin in [0.0, 0.5, 0.9] {
            let black_hole = hole(spin);
            let constants = KerrNullConstants::new(1.0, 2.0, 3.0).unwrap();
            let coordinates = KerrMinoState::new(0.0, 10.0, 1.0, 0.0);
            let canonical = KerrCanonicalHamiltonian::new(black_hole);
            let bl_state = canonical
                .state_from_separated(
                    coordinates,
                    constants,
                    KerrMinoDirection::Decreasing,
                    KerrMinoDirection::Increasing,
                )
                .unwrap();
            let ingoing_state = canonical
                .ingoing_state_from_separated(
                    coordinates,
                    constants,
                    KerrMinoDirection::Increasing,
                )
                .unwrap();
            let bl = canonical.evaluate(bl_state).unwrap();
            let ingoing = canonical.evaluate_ingoing(ingoing_state).unwrap();
            let r = coordinates.radius_over_m;
            let theta = coordinates.theta_rad;
            let delta = r * r - 2.0 * r + spin * spin;
            let expected_v = bl.derivative.coordinates[0]
                + (r * r + spin * spin) / delta * bl.derivative.coordinates[1];
            let expected_phi = bl.derivative.coordinates[3]
                + spin / delta * bl.derivative.coordinates[1];
            close(ingoing.derivative.coordinates[0], expected_v, 2e-11);
            close(ingoing.derivative.coordinates[3], expected_phi, 2e-11);
            let _ = theta;
            close(ingoing.hamiltonian, 0.0, 2e-11);
            close(bl.hamiltonian, ingoing.hamiltonian, 2e-11);
            close(bl.energy, ingoing.energy, 2e-14);
            close(
                bl.angular_momentum_z_over_m,
                ingoing.angular_momentum_z_over_m,
                2e-14,
            );
            close(
                bl.carter_constant_over_m2,
                ingoing.carter_constant_over_m2,
                2e-11,
            );
            close(
                ingoing.derivative.coordinates[1],
                bl.derivative.coordinates[1],
                2e-11,
            );
            close(
                ingoing.derivative.coordinates[2],
                bl.derivative.coordinates[2],
                2e-11,
            );
        }
    }

    #[test]
    fn ingoing_hamiltonian_remains_finite_at_and_inside_outer_horizon() {
        let spin = 0.9;
        let black_hole = hole(spin);
        let canonical = KerrCanonicalHamiltonian::new(black_hole);
        let constants = KerrNullConstants::new(1.0, 2.0, 3.0).unwrap();
        let horizon = 1.0 + (1.0 - spin * spin).sqrt();
        for radius in [horizon + 0.1, horizon, 0.9 * horizon, 0.5 * horizon] {
            let coordinates = KerrMinoState::new(0.0, radius, 1.0, 0.0);
            let state = canonical
                .ingoing_state_from_separated(
                    coordinates,
                    constants,
                    KerrMinoDirection::Increasing,
                )
                .unwrap();
            let evaluation = canonical.evaluate_ingoing(state).unwrap();
            assert!(evaluation.hamiltonian.is_finite());
            assert!(evaluation.derivative.coordinates.iter().all(|v| v.is_finite()));
            assert!(evaluation.derivative.momenta.iter().all(|v| v.is_finite()));
            close(evaluation.hamiltonian, 0.0, 2e-10);
            close(evaluation.energy, 1.0, 2e-14);
            close(evaluation.angular_momentum_z_over_m, 2.0, 2e-14);
            close(evaluation.carter_constant_over_m2, 3.0, 2e-10);
        }
    }

    #[test]
    fn canonical_hamiltonian_reports_constraint_drift_not_a_fake_null_state() {
        let black_hole = hole(0.7);
        let canonical = KerrCanonicalHamiltonian::new(black_hole);
        let state = KerrCanonicalState {
            coordinates: KerrMinoState::new(0.0, 10.0, 1.0, 0.0),
            momenta: [-1.0, 0.0, 0.0, 0.0],
        };
        let evaluation = canonical.evaluate(state).unwrap();
        assert!(evaluation.hamiltonian.is_finite());
        assert!(evaluation.hamiltonian.abs() > 1e-8);
    }

    #[test]
    fn canonical_initialization_rejects_forbidden_turning_regions() {
        let black_hole = hole(0.0);
        let canonical = KerrCanonicalHamiltonian::new(black_hole);
        let coordinates =
            KerrMinoState::new(0.0, 2.5, core::f64::consts::FRAC_PI_2, 0.0);
        let constants = KerrNullConstants::new(1.0, 3.0 * 3.0_f64.sqrt(), 0.0).unwrap();
        assert!(matches!(
            canonical.state_from_separated(
                coordinates,
                constants,
                KerrMinoDirection::Increasing,
                KerrMinoDirection::Increasing
            ),
            Err(KerrGeodesicError::ForbiddenRadialRegion { .. })
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
