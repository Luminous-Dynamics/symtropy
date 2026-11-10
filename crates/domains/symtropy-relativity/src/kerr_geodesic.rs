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
//! derivatives, supplies a turning-point-regular second-order system, and
//! integrates exterior rays with adaptive RK4 step-doubling. A separate
//! canonical Hamiltonian vector-field evaluator provides an independent
//! formulation and supports the horizon-regular ingoing Kerr chart.
//!
//! The module is not yet a ray-bundle/Jacobi solver, does not provide finite-radius
//! observer tetrads or caustic detection, and does not integrate the canonical
//! Hamiltonian vector field. The adaptive Mino integrator uses Boyer-Lindquist
//! coordinates and deliberately stops at the horizon/chart boundary.

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

/// Coordinates for a separated Kerr null-geodesic state.
///
/// Coordinates are dimensionless. The coordinate-time and azimuth slots are
/// [ct/M,phi] in Boyer-Lindquist mode and [v/M,phi_tilde] in ingoing Kerr mode;
/// the trajectory's coordinate_chart records which interpretation applies.
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



/// Unit photon propagation direction in a local orthonormal observer frame.
///
/// Components are resolved along the ZAMO radial, increasing-theta, and
/// increasing-azimuthal basis vectors. The vector is future-directed when
/// paired with the positive local energy used by the observer constructor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrLocalSkyDirection {
    pub radial: f64,
    pub polar: f64,
    pub azimuthal: f64,
}

impl KerrLocalSkyDirection {
    pub fn new(
        radial: f64,
        polar: f64,
        azimuthal: f64,
    ) -> Result<Self, KerrGeodesicError> {
        let norm_squared = radial * radial + polar * polar + azimuthal * azimuthal;
        if !radial.is_finite() || !polar.is_finite() || !azimuthal.is_finite() {
            return Err(KerrGeodesicError::InvalidSkyDirection {
                radial,
                polar,
                azimuthal,
                norm_squared,
            });
        }
        if (norm_squared - 1.0).abs() > 256.0 * f64::EPSILON {
            return Err(KerrGeodesicError::InvalidSkyDirection {
                radial,
                polar,
                azimuthal,
                norm_squared,
            });
        }
        Ok(Self { radial, polar, azimuthal })
    }
}

/// ZAMO orthonormal tetrad in dimensionless Boyer-Lindquist coordinates.
///
/// Each tetrad vector is contravariant and ordered as
/// [ct/M, r/M, theta, phi]. The lapse, frame-dragging angular velocity, and
/// azimuthal proper length are dimensionless in M=1 units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrZamoTetrad {
    pub timelike: [f64; 4],
    pub radial: [f64; 4],
    pub polar: [f64; 4],
    pub azimuthal: [f64; 4],
    pub lapse: f64,
    pub frame_dragging_angular_velocity: f64,
    pub sqrt_g_phi_phi: f64,
}

/// A finite-radius zero-angular-momentum observer outside the Kerr horizon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrFiniteRadiusObserver {
    hole: KerrBlackHole,
    radius_over_m: f64,
    theta_rad: f64,
    tetrad: KerrZamoTetrad,
}

/// Ray initial data produced by a finite-radius local observer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrObserverRaySeed {
    pub constants: KerrNullConstants,
    pub initial_state: KerrMinoDynamicState,
    /// Killing energy at infinity per unit local ZAMO photon energy.
    pub infinity_energy_per_unit_local_energy: f64,
    pub local_direction: KerrLocalSkyDirection,
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
    InvalidSkyDirection {
        radial: f64,
        polar: f64,
        azimuthal: f64,
        norm_squared: f64,
    },
    NegativeKillingEnergy(f64),
    NoRegularIngoingBranch {
        radial_denominator: f64,
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
            Self::InvalidSkyDirection { radial, polar, azimuthal, norm_squared } => write!(
                f,
                "local sky direction must be a finite unit 3-vector: ({radial},{polar},{azimuthal}), norm^2={norm_squared}"
            ),
            Self::NegativeKillingEnergy(value) => write!(
                f,
                "future-directed local photon has non-positive Killing energy at infinity: {value}"
            ),
            Self::NoRegularIngoingBranch { radial_denominator } => write!(
                f,
                "Kerr ingoing chart requires a positive regularized radial denominator, got {radial_denominator}"
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


    /// Evaluate the second-order Mino system in horizon-regular ingoing Kerr
    /// coordinates (v/M,r/M,theta,phi_tilde).
    ///
    /// For future-ingoing rays (dr/dgamma <= 0), the potentially singular
    /// combinations are rationalized using
    /// (P + dr/dgamma)/Delta = [Q+(L-aE)^2]/[P-dr/dgamma].
    /// This remains finite at the future horizon when the horizon-generator
    /// energy P is positive. Outgoing rays retain the direct expression and
    /// do not cross the future horizon in this chart.
    pub fn mino_dynamic_derivative_ingoing(
        self,
        state: KerrMinoDynamicState,
    ) -> Result<KerrMinoDynamicDerivative, KerrGeodesicError> {
        let coordinates = state.coordinates;
        for (name, value) in [
            ("ingoing_time_over_m", coordinates.time_over_m),
            ("radius_over_m", coordinates.radius_over_m),
            ("theta_rad", coordinates.theta_rad),
            ("ingoing_azimuth_rad", coordinates.azimuth_rad),
            ("radial_velocity_over_m", state.radial_velocity_over_m),
            ("polar_velocity_rad", state.polar_velocity_rad),
        ] {
            if !value.is_finite() {
                return Err(KerrGeodesicError::NonFinite(name));
            }
        }
        validate_polar_angle(coordinates.theta_rad)?;
        let r = coordinates.radius_over_m;
        if r <= 0.0 {
            return Err(KerrGeodesicError::InvalidRadius(r));
        }
        let theta = coordinates.theta_rad;
        let a = self.hole.spin().get();
        let delta = r * r - 2.0 * r + a * a;
        let radial_acceleration = 0.5 * self.radial_potential_derivative(r)?;
        let polar_acceleration = 0.5 * self.constants.polar_potential_derivative(a, theta)?;
        let energy = self.constants.energy;
        let angular_momentum = self.constants.angular_momentum_z_over_m;
        let sin_theta = theta.sin();
        let sin_squared = sin_theta * sin_theta;
        let p = energy * (r * r + a * a) - a * angular_momentum;
        let angular_term =
            self.constants.carter_constant_over_m2 + (angular_momentum - a * energy).powi(2);
        let radial_velocity = state.radial_velocity_over_m;
        let ratio = if radial_velocity <= 0.0 {
            let denominator = p - radial_velocity;
            if !denominator.is_finite() || denominator <= 0.0 {
                return Err(KerrGeodesicError::NoRegularIngoingBranch { radial_denominator: denominator });
            }
            angular_term / denominator
        } else {
            if !delta.is_finite() || delta <= 0.0 {
                return Err(KerrGeodesicError::NoRegularIngoingBranch {
                    radial_denominator: p + radial_velocity,
                });
            }
            (p + radial_velocity) / delta
        };
        let d_time = a * (angular_momentum - a * energy * sin_squared)
            + (r * r + a * a) * ratio;
        let d_azimuth = angular_momentum / sin_squared - a * energy + a * ratio;
        let derivative = KerrMinoDynamicDerivative {
            d_time_over_m: d_time,
            d_radius_over_m: radial_velocity,
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
            return Err(KerrGeodesicError::NonFinite("ingoing_mino_dynamic_derivative"));
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


/// Adaptive step-doubling integrator for the second-order Kerr Mino system.
///
/// This integrator is intended for exterior Boyer–Lindquist trajectories.
/// It crosses simple radial and polar turning points through the smooth
/// second-order system, but deliberately stops rather than crossing the
/// Boyer–Lindquist horizon or polar-axis chart singularities.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrMinoIntegrationConfig {
    pub relative_tolerance: f64,
    /// Absolute tolerances for [t/M, r/M, theta, phi, dr/dgamma, dtheta/dgamma].
    pub absolute_tolerance: [f64; 6],
    /// Maximum normalized radial/polar first-integral residual.
    pub constraint_tolerance: f64,
    pub initial_step: f64,
    pub minimum_step: f64,
    pub maximum_step: f64,
    /// Maximum total attempted step-doubling cycles (accepted + rejected).
    pub maximum_attempts: usize,
}

impl KerrMinoIntegrationConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        relative_tolerance: f64,
        absolute_tolerance: [f64; 6],
        constraint_tolerance: f64,
        initial_step: f64,
        minimum_step: f64,
        maximum_step: f64,
        maximum_attempts: usize,
    ) -> Result<Self, KerrMinoIntegrationError> {
        if !relative_tolerance.is_finite() || relative_tolerance <= 0.0 {
            return Err(KerrMinoIntegrationError::InvalidConfiguration("relative_tolerance"));
        }
        if absolute_tolerance.iter().any(|value| !value.is_finite() || *value <= 0.0) {
            return Err(KerrMinoIntegrationError::InvalidConfiguration("absolute_tolerance"));
        }
        if !constraint_tolerance.is_finite() || constraint_tolerance <= 0.0 {
            return Err(KerrMinoIntegrationError::InvalidConfiguration("constraint_tolerance"));
        }
        if !initial_step.is_finite() || initial_step <= 0.0 {
            return Err(KerrMinoIntegrationError::InvalidConfiguration("initial_step"));
        }
        if !minimum_step.is_finite() || minimum_step <= 0.0 {
            return Err(KerrMinoIntegrationError::InvalidConfiguration("minimum_step"));
        }
        if !maximum_step.is_finite() || maximum_step < minimum_step {
            return Err(KerrMinoIntegrationError::InvalidConfiguration("maximum_step"));
        }
        if initial_step < minimum_step || initial_step > maximum_step {
            return Err(KerrMinoIntegrationError::InvalidConfiguration("initial_step_bounds"));
        }
        if maximum_attempts == 0 {
            return Err(KerrMinoIntegrationError::InvalidConfiguration("maximum_attempts"));
        }
        Ok(Self {
            relative_tolerance,
            absolute_tolerance,
            constraint_tolerance,
            initial_step,
            minimum_step,
            maximum_step,
            maximum_attempts,
        })
    }

    /// Conservative starter configuration for dimensionless research rays.
    pub fn research_default() -> Self {
        Self {
            relative_tolerance: 1.0e-12,
            absolute_tolerance: [1.0e-12; 6],
            constraint_tolerance: 1.0e-5,
            initial_step: 1.0e-3,
            minimum_step: 1.0e-12,
            maximum_step: 2.0e-2,
            maximum_attempts: 200_000,
        }
    }
}

/// One accepted point on an adaptive Mino trajectory.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrMinoTrajectoryPoint {
    pub mino_parameter: f64,
    pub state: KerrMinoDynamicState,
    pub constraint_residuals: KerrMinoConstraintResiduals,
}

/// Completed exterior Kerr null trajectory and numerical diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct KerrMinoTrajectory {
    /// Coordinate chart that gives meaning to the time/azimuth slots in every state.
    pub coordinate_chart: KerrMinoCoordinateChart,
    /// Initial state followed by every accepted step endpoint.
    pub points: Vec<KerrMinoTrajectoryPoint>,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub suggested_next_step: f64,
    pub maximum_normalized_radial_constraint_residual: f64,
    pub maximum_normalized_polar_constraint_residual: f64,
    pub requested_final_parameter: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KerrMinoIntegrationError {
    Geodesic(KerrGeodesicError),
    InvalidConfiguration(&'static str),
    InvalidFinalParameter(f64),
    InitialConstraintViolation {
        radial_relative: f64,
        polar_relative: f64,
    },
    ConstraintDriftExceeded {
        mino_parameter: f64,
        radial_relative: f64,
        polar_relative: f64,
        tolerance: f64,
    },
    StepSizeUnderflow {
        mino_parameter: f64,
        attempted_step: f64,
        normalized_error: f64,
    },
    MaximumAttemptsExceeded {
        mino_parameter: f64,
        attempts: usize,
    },
    NonFiniteErrorEstimate,
}

impl From<KerrGeodesicError> for KerrMinoIntegrationError {
    fn from(value: KerrGeodesicError) -> Self {
        Self::Geodesic(value)
    }
}

impl fmt::Display for KerrMinoIntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Geodesic(error) => write!(f, "Kerr Mino integration geometry error: {error}"),
            Self::InvalidConfiguration(name) => {
                write!(f, "invalid Kerr Mino integration configuration: {name}")
            }
            Self::InvalidFinalParameter(value) => {
                write!(f, "final Mino parameter must be finite and >= 0, got {value}")
            }
            Self::InitialConstraintViolation { radial_relative, polar_relative } => write!(
                f,
                "initial Kerr null first-integral residual exceeds tolerance: radial={radial_relative:e}, polar={polar_relative:e}"
            ),
            Self::ConstraintDriftExceeded { mino_parameter, radial_relative, polar_relative, tolerance } => write!(
                f,
                "Kerr Mino first-integral drift exceeded tolerance at gamma={mino_parameter:e}: radial={radial_relative:e}, polar={polar_relative:e}, tolerance={tolerance:e}"
            ),
            Self::StepSizeUnderflow { mino_parameter, attempted_step, normalized_error } => write!(
                f,
                "Kerr Mino step underflow at gamma={mino_parameter:e}, step={attempted_step:e}, error={normalized_error:e}"
            ),
            Self::MaximumAttemptsExceeded { mino_parameter, attempts } => write!(
                f,
                "Kerr Mino integrator exceeded {attempts} attempts at gamma={mino_parameter:e}"
            ),
            Self::NonFiniteErrorEstimate => {
                write!(f, "Kerr Mino integrator produced a non-finite error estimate")
            }
        }
    }
}

impl std::error::Error for KerrMinoIntegrationError {}

/// Coordinate chart used by an integrated separated Kerr Mino trajectory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KerrMinoCoordinateChart {
    /// Exterior-only Boyer-Lindquist coordinates (ct/M,r/M,theta,phi).
    BoyerLindquist,
    /// Horizon-regular ingoing Kerr coordinates (v/M,r/M,theta,phi_tilde).
    IngoingKerr,
}

#[derive(Debug, Clone, Copy)]
struct MinoVector([f64; 6]);

impl MinoVector {
    fn from_state(state: KerrMinoDynamicState) -> Self {
        Self([
            state.coordinates.time_over_m,
            state.coordinates.radius_over_m,
            state.coordinates.theta_rad,
            state.coordinates.azimuth_rad,
            state.radial_velocity_over_m,
            state.polar_velocity_rad,
        ])
    }

    fn into_state(self) -> KerrMinoDynamicState {
        KerrMinoDynamicState::new(
            KerrMinoState::new(self.0[0], self.0[1], self.0[2], self.0[3]),
            self.0[4],
            self.0[5],
        )
    }

    fn add_scaled(self, other: Self, scale: f64) -> Self {
        let mut result = self.0;
        for (index, value) in result.iter_mut().enumerate() {
            *value += scale * other.0[index];
        }
        Self(result)
    }
}

impl KerrSeparatedNullGeodesic {
    fn mino_vector_field(
        self,
        state: MinoVector,
        chart: KerrMinoCoordinateChart,
    ) -> Result<MinoVector, KerrGeodesicError> {
        let dynamic_state = state.into_state();
        let derivative = match chart {
            KerrMinoCoordinateChart::BoyerLindquist => self.mino_dynamic_derivative(dynamic_state)?,
            KerrMinoCoordinateChart::IngoingKerr => self.mino_dynamic_derivative_ingoing(dynamic_state)?,
        };
        Ok(MinoVector([
            derivative.d_time_over_m,
            derivative.d_radius_over_m,
            derivative.d_theta_rad,
            derivative.d_azimuth_rad,
            derivative.d_radial_velocity_over_m,
            derivative.d_polar_velocity_rad,
        ]))
    }

    fn rk4_mino_step(
        self,
        state: MinoVector,
        step: f64,
        chart: KerrMinoChart,
    ) -> Result<MinoVector, KerrGeodesicError> {
        let k1 = self.mino_vector_field(state, chart)?;
        let k2 = self.mino_vector_field(state.add_scaled(k1, 0.5 * step), chart)?;
        let k3 = self.mino_vector_field(state.add_scaled(k2, 0.5 * step), chart)?;
        let k4 = self.mino_vector_field(state.add_scaled(k3, step), chart)?;
        let mut result = state;
        for index in 0..6 {
            result.0[index] += step
                * (k1.0[index] + 2.0 * k2.0[index] + 2.0 * k3.0[index] + k4.0[index])
                / 6.0;
        }
        if result.0.iter().any(|value| !value.is_finite()) {
            return Err(KerrGeodesicError::NonFinite("rk4_mino_state"));
        }
        Ok(result)
    }

    /// Integrate from gamma=0 to `final_parameter` using adaptive RK4 step-doubling.
    ///
    /// The full-step vs two-half-step discrepancy estimates the local RK4
    /// error (divided by 15). The two-half-step solution is accepted. Step
    /// control also enforces a normalized bound on both separated first
    /// integrals, so the solver cannot claim a qualified null ray merely
    /// because the coordinate error estimate is small.
    /// Integrate an exterior trajectory in Boyer–Lindquist coordinates.
    pub fn integrate_adaptive(
        self,
        initial: KerrMinoDynamicState,
        final_parameter: f64,
        config: KerrMinoIntegrationConfig,
    ) -> Result<KerrMinoTrajectory, KerrMinoIntegrationError> {
        self.integrate_adaptive_in_chart(
            initial,
            final_parameter,
            config,
            KerrMinoChart::BoyerLindquist,
        )
    }

    /// Integrate a future-ingoing ray in horizon-regular ingoing Kerr
    /// coordinates. The trajectory may cross the outer horizon, but remains
    /// restricted to r>0 and 0<theta<pi.
    pub fn integrate_adaptive_ingoing(
        self,
        initial: KerrMinoDynamicState,
        final_parameter: f64,
        config: KerrMinoIntegrationConfig,
    ) -> Result<KerrMinoTrajectory, KerrMinoIntegrationError> {
        self.integrate_adaptive_in_chart(
            initial,
            final_parameter,
            config,
            KerrMinoChart::Ingoing,
        )
    }

    fn integrate_adaptive_in_chart(
        self,
        initial: KerrMinoDynamicState,
        final_parameter: f64,
        config: KerrMinoIntegrationConfig,
        chart: KerrMinoChart,
    ) -> Result<KerrMinoTrajectory, KerrMinoIntegrationError> {
        if !final_parameter.is_finite() || final_parameter < 0.0 {
            return Err(KerrMinoIntegrationError::InvalidFinalParameter(final_parameter));
        }
        let initial_residuals = self.mino_constraint_residuals(initial)?;
        let initial_radial_relative = normalized_constraint_residual(
            initial.radial_velocity_over_m.powi(2),
            self.radial_potential(initial.coordinates.radius_over_m)?,
            initial_residuals.radial,
        );
        let initial_polar_relative = normalized_constraint_residual(
            initial.polar_velocity_rad.powi(2),
            self.polar_potential(initial.coordinates.theta_rad)?,
            initial_residuals.polar,
        );
        if initial_radial_relative > config.constraint_tolerance
            || initial_polar_relative > config.constraint_tolerance
        {
            return Err(KerrMinoIntegrationError::InitialConstraintViolation {
                radial_relative: initial_radial_relative,
                polar_relative: initial_polar_relative,
            });
        }

        let mut points = vec![KerrMinoTrajectoryPoint {
            mino_parameter: 0.0,
            state: initial,
            constraint_residuals: initial_residuals,
        }];
        if final_parameter == 0.0 {
            return Ok(KerrMinoTrajectory {
                coordinate_chart: chart,
                points,
                accepted_steps: 0,
                rejected_steps: 0,
                suggested_next_step: 0.0,
                maximum_normalized_radial_constraint_residual: initial_radial_relative,
                maximum_normalized_polar_constraint_residual: initial_polar_relative,
                requested_final_parameter: final_parameter,
            });
        }

        let mut gamma = 0.0;
        let mut vector = MinoVector::from_state(initial);
        let mut step = config.initial_step.min(final_parameter);
        let mut attempts = 0usize;
        let mut accepted_steps = 0usize;
        let mut rejected_steps = 0usize;
        let mut max_radial_residual = initial_radial_relative;
        let mut max_polar_residual = initial_polar_relative;

        while gamma < final_parameter {
            if attempts >= config.maximum_attempts {
                return Err(KerrMinoIntegrationError::MaximumAttemptsExceeded {
                    mino_parameter: gamma,
                    attempts,
                });
            }
            attempts += 1;
            let remaining = final_parameter - gamma;
            let trial_step = step.min(remaining);
            if trial_step < config.minimum_step && remaining > config.minimum_step {
                return Err(KerrMinoIntegrationError::StepSizeUnderflow {
                    mino_parameter: gamma,
                    attempted_step: trial_step,
                    normalized_error: f64::INFINITY,
                });
            }

            let full = self.rk4_mino_step(vector, trial_step, chart)?;
            let half = self.rk4_mino_step(vector, 0.5 * trial_step, chart)?;
            let two_half = self.rk4_mino_step(half, 0.5 * trial_step, chart)?;
            let mut error_norm: f64 = 0.0;
            for index in 0..6 {
                let scale = config.absolute_tolerance[index]
                    + config.relative_tolerance * vector.0[index].abs().max(two_half.0[index].abs());
                let component_error =
                    (two_half.0[index] - full.0[index]).abs() / (15.0 * scale);
                error_norm = error_norm.max(component_error);
            }

            let candidate = two_half.into_state();
            let residuals = self.mino_constraint_residuals(candidate)?;
            let radial_potential = self.radial_potential(candidate.coordinates.radius_over_m)?;
            let polar_potential = self.polar_potential(candidate.coordinates.theta_rad)?;
            let radial_relative = normalized_constraint_residual(
                candidate.radial_velocity_over_m.powi(2),
                radial_potential,
                residuals.radial,
            );
            let polar_relative = normalized_constraint_residual(
                candidate.polar_velocity_rad.powi(2),
                polar_potential,
                residuals.polar,
            );
            if !error_norm.is_finite() {
                return Err(KerrMinoIntegrationError::NonFiniteErrorEstimate);
            }

            if error_norm <= 1.0 {
                let candidate_gamma = gamma + trial_step;
                if radial_relative > config.constraint_tolerance
                    || polar_relative > config.constraint_tolerance
                {
                    return Err(KerrMinoIntegrationError::ConstraintDriftExceeded {
                        mino_parameter: candidate_gamma,
                        radial_relative,
                        polar_relative,
                        tolerance: config.constraint_tolerance,
                    });
                }
                gamma = candidate_gamma;
                vector = two_half;
                accepted_steps += 1;
                max_radial_residual = max_radial_residual.max(radial_relative);
                max_polar_residual = max_polar_residual.max(polar_relative);
                points.push(KerrMinoTrajectoryPoint {
                    mino_parameter: gamma,
                    state: candidate,
                    constraint_residuals: residuals,
                });
                let factor = if error_norm <= 1.0e-14 {
                    5.0
                } else {
                    (0.9 * error_norm.powf(-0.2)).clamp(0.2, 5.0)
                };
                step = (trial_step * factor).clamp(config.minimum_step, config.maximum_step);
            } else {
                rejected_steps += 1;
                let factor = (0.9 * error_norm.powf(-0.2)).clamp(0.1, 0.5);
                let reduced = trial_step * factor;
                if reduced < config.minimum_step {
                    return Err(KerrMinoIntegrationError::StepSizeUnderflow {
                        mino_parameter: gamma,
                        attempted_step: reduced,
                        normalized_error: error_norm,
                    });
                }
                step = reduced;
            }
        }

        Ok(KerrMinoTrajectory {
            coordinate_chart: chart,
            points,
            accepted_steps,
            rejected_steps,
            suggested_next_step: step,
            maximum_normalized_radial_constraint_residual: max_radial_residual,
            maximum_normalized_polar_constraint_residual: max_polar_residual,
            requested_final_parameter: final_parameter,
        })
    }
}


impl KerrFiniteRadiusObserver {
    /// Construct a ZAMO tetrad at a finite Boyer-Lindquist event outside r_+.
    pub fn new(
        hole: KerrBlackHole,
        radius_over_m: f64,
        theta_rad: f64,
    ) -> Result<Self, KerrGeodesicError> {
        if !radius_over_m.is_finite() || radius_over_m <= 0.0 {
            return Err(KerrGeodesicError::InvalidRadius(radius_over_m));
        }
        validate_polar_angle(theta_rad)?;
        let spin = hole.spin().get();
        let horizon = hole.outer_horizon_radius_m() / hole.gravitational_radius_m();
        if radius_over_m <= horizon {
            return Err(KerrGeodesicError::RadiusAtOrInsideHorizon {
                radius_over_m,
                horizon_over_m: horizon,
            });
        }
        let r = radius_over_m;
        let theta = theta_rad;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();
        let sin_squared = sin_theta * sin_theta;
        let sigma = r * r + spin * spin * cos_theta * cos_theta;
        let delta = r * r - 2.0 * r + spin * spin;
        let a_function = (r * r + spin * spin).powi(2)
            - spin * spin * delta * sin_squared;
        let g_t_phi = -2.0 * spin * r * sin_squared / sigma;
        let g_phi_phi = a_function * sin_squared / sigma;
        let omega = -g_t_phi / g_phi_phi;
        let lapse_squared = sigma * delta / a_function;
        if !sigma.is_finite()
            || !delta.is_finite()
            || !a_function.is_finite()
            || !g_phi_phi.is_finite()
            || !lapse_squared.is_finite()
            || sigma <= 0.0
            || delta <= 0.0
            || a_function <= 0.0
            || g_phi_phi <= 0.0
            || lapse_squared <= 0.0
        {
            return Err(KerrGeodesicError::NonFinite("ZAMO_tetrad_metric_factors"));
        }
        let lapse = lapse_squared.sqrt();
        let sqrt_g_phi_phi = g_phi_phi.sqrt();
        let tetrad = KerrZamoTetrad {
            timelike: [1.0 / lapse, 0.0, 0.0, omega / lapse],
            radial: [0.0, (delta / sigma).sqrt(), 0.0, 0.0],
            polar: [0.0, 0.0, 1.0 / sigma.sqrt(), 0.0],
            azimuthal: [0.0, 0.0, 0.0, 1.0 / sqrt_g_phi_phi],
            lapse,
            frame_dragging_angular_velocity: omega,
            sqrt_g_phi_phi,
        };
        if [
            tetrad.timelike,
            tetrad.radial,
            tetrad.polar,
            tetrad.azimuthal,
        ]
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
        {
            return Err(KerrGeodesicError::NonFinite("ZAMO_tetrad"));
        }
        Ok(Self {
            hole,
            radius_over_m,
            theta_rad,
            tetrad,
        })
    }

    pub const fn tetrad(self) -> KerrZamoTetrad {
        self.tetrad
    }

    pub const fn radius_over_m(self) -> f64 {
        self.radius_over_m
    }

    pub const fn theta_rad(self) -> f64 {
        self.theta_rad
    }

    /// Convert a local future-directed photon direction into E=1 Carter
    /// constants and a turning-point-ready Mino initial state.
    ///
    /// The local photon energy is set to one. The resulting conserved
    /// energy at infinity is then used to normalize the constants to E=1.
    /// Negative-Killing-energy rays inside the ergoregion are rejected by
    /// this API because KerrNullConstants currently represents E>0 rays.
    pub fn ray_seed(
        self,
        direction: KerrLocalSkyDirection,
    ) -> Result<KerrObserverRaySeed, KerrGeodesicError> {
        let tetrad = self.tetrad;
        let p_contravariant: [f64; 4] = core::array::from_fn(|mu| {
            tetrad.timelike[mu]
                + direction.radial * tetrad.radial[mu]
                + direction.polar * tetrad.polar[mu]
                + direction.azimuthal * tetrad.azimuthal[mu]
        });
        let r = self.radius_over_m;
        let theta = self.theta_rad;
        let a = self.hole.spin().get();
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();
        let sin_squared = sin_theta * sin_theta;
        let sigma = r * r + a * a * cos_theta * cos_theta;
        let g_tt = -(1.0 - 2.0 * r / sigma);
        let g_t_phi = -2.0 * a * r * sin_squared / sigma;
        let g_phi_phi = tetrad.sqrt_g_phi_phi.powi(2);
        let p_t = g_tt * p_contravariant[0] + g_t_phi * p_contravariant[3];
        let p_phi = g_t_phi * p_contravariant[0] + g_phi_phi * p_contravariant[3];
        let p_theta = sigma * p_contravariant[2];
        let energy_at_infinity = -p_t;
        if !energy_at_infinity.is_finite() || energy_at_infinity <= 0.0 {
            return Err(KerrGeodesicError::NegativeKillingEnergy(energy_at_infinity));
        }
        let angular_momentum = p_phi / energy_at_infinity;
        let carter = (p_theta / energy_at_infinity).powi(2)
            + cos_theta.powi(2)
                * (angular_momentum.powi(2) / sin_squared - a * a);
        let constants = KerrNullConstants::new(1.0, angular_momentum, carter)?;
        let geodesic = KerrSeparatedNullGeodesic::new(self.hole, constants);
        let radial_potential = geodesic.radial_potential(r)?;
        let polar_potential = geodesic.polar_potential(theta)?;
        let scale = 1.0 + radial_potential.abs() + polar_potential.abs();
        if radial_potential < -512.0 * f64::EPSILON * scale {
            return Err(KerrGeodesicError::ForbiddenRadialRegion {
                radius_over_m: r,
                radial_potential,
            });
        }
        if polar_potential < -512.0 * f64::EPSILON * scale {
            return Err(KerrGeodesicError::ForbiddenPolarRegion {
                theta_rad: theta,
                polar_potential,
            });
        }
        let radial_sign = direction.radial.signum();
        let polar_sign = direction.polar.signum();
        let state = KerrMinoDynamicState::new(
            KerrMinoState::new(0.0, r, theta, 0.0),
            radial_sign * radial_potential.max(0.0).sqrt(),
            polar_sign * polar_potential.max(0.0).sqrt(),
        );
        let residuals = geodesic.mino_constraint_residuals(state)?;
        let radial_relative = normalized_constraint_residual(
            state.radial_velocity_over_m.powi(2),
            radial_potential,
            residuals.radial,
        );
        let polar_relative = normalized_constraint_residual(
            state.polar_velocity_rad.powi(2),
            polar_potential,
            residuals.polar,
        );
        if radial_relative > 1.0e-10 || polar_relative > 1.0e-10 {
            return Err(KerrGeodesicError::NonFinite("finite_observer_screen_normalization"));
        }
        Ok(KerrObserverRaySeed {
            constants,
            initial_state: state,
            infinity_energy_per_unit_local_energy: energy_at_infinity,
            local_direction: direction,
        })
    }
}

fn normalized_constraint_residual(first_term: f64, second_term: f64, residual: f64) -> f64 {
    residual.abs() / (1.0 + first_term.abs() + second_term.abs())
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
    fn adaptive_mino_integrator_crosses_schwarzschild_scattering_turning_point() {
        let constants = KerrNullConstants::new(1.0, 6.0, 0.0).unwrap();
        let geodesic = KerrSeparatedNullGeodesic::new(hole(0.0), constants);
        let radius = 10.0;
        let radial = geodesic.radial_potential(radius).unwrap();
        let initial = KerrMinoDynamicState::new(
            KerrMinoState::new(0.0, radius, core::f64::consts::FRAC_PI_2, 0.0),
            -radial.sqrt(),
            0.0,
        );
        let config = KerrMinoIntegrationConfig::new(
            1.0e-14,
            [1.0e-14; 6],
            3.0e-8,
            1.0e-3,
            1.0e-12,
            2.0e-2,
            200_000,
        )
        .unwrap();
        let trajectory = geodesic.integrate_adaptive(initial, 0.6, config).unwrap();
        let minimum_radius = trajectory
            .points
            .iter()
            .map(|point| point.state.coordinates.radius_over_m)
            .fold(f64::INFINITY, f64::min);
        let final_radius = trajectory.points.last().unwrap().state.coordinates.radius_over_m;
        close(minimum_radius, 4.453_363_193_811_354, 2.0e-4);
        assert!(final_radius > radius);
        assert!(trajectory.accepted_steps > 0);
        assert!(trajectory.rejected_steps > 0);
        assert!(trajectory.maximum_normalized_radial_constraint_residual <= 2.0e-8);
        assert!(trajectory.maximum_normalized_polar_constraint_residual <= 2.0e-8);
    }

    #[test]
    fn adaptive_mino_integrator_fails_closed_on_invalid_initial_null_constraints() {
        let constants = KerrNullConstants::new(1.0, 6.0, 0.0).unwrap();
        let geodesic = KerrSeparatedNullGeodesic::new(hole(0.0), constants);
        let initial = KerrMinoDynamicState::new(
            KerrMinoState::new(0.0, 10.0, core::f64::consts::FRAC_PI_2, 0.0),
            0.0,
            0.0,
        );
        let result = geodesic.integrate_adaptive(
            initial,
            0.1,
            KerrMinoIntegrationConfig::research_default(),
        );
        assert!(matches!(
            result,
            Err(KerrMinoIntegrationError::InitialConstraintViolation { .. })
        ));
    }


    #[test]
    fn finite_radius_zamo_tetrad_is_orthonormal_for_subextremal_kerr() {
        for spin in [0.0, 0.5, 0.9, -0.9] {
            let observer = KerrFiniteRadiusObserver::new(hole(spin), 10.0, 0.8).unwrap();
            let tetrad = observer.tetrad();
            let r = 10.0;
            let theta: f64 = 0.8;
            let a = spin;
            let sigma = r * r + a * a * theta.cos().powi(2);
            let delta = r * r - 2.0 * r + a * a;
            let sin2 = theta.sin().powi(2);
            let g = [
                [-(1.0 - 2.0 * r / sigma), 0.0, 0.0, -2.0 * a * r * sin2 / sigma],
                [0.0, sigma / delta, 0.0, 0.0],
                [0.0, 0.0, sigma, 0.0],
                [-2.0 * a * r * sin2 / sigma, 0.0, 0.0, tetrad.sqrt_g_phi_phi.powi(2)],
            ];
            let vectors = [tetrad.timelike, tetrad.radial, tetrad.polar, tetrad.azimuthal];
            for i in 0..4 {
                for j in 0..4 {
                    let mut product = 0.0;
                    for mu in 0..4 {
                        for nu in 0..4 {
                            product += g[mu][nu] * vectors[i][mu] * vectors[j][nu];
                        }
                    }
                    let expected = if i == j { if i == 0 { -1.0 } else { 1.0 } } else { 0.0 };
                    close(product, expected, 5.0e-13);
                }
            }
        }
    }

    #[test]
    fn finite_radius_observer_maps_unit_sky_direction_to_null_carter_data() {
        for spin in [0.0, 0.5, 0.9] {
            let observer = KerrFiniteRadiusObserver::new(hole(spin), 10.0, 1.0).unwrap();
            let direction = KerrLocalSkyDirection::new(-0.8, 0.6, 0.0).unwrap();
            let seed = observer.ray_seed(direction).unwrap();
            let geodesic = KerrSeparatedNullGeodesic::new(hole(spin), seed.constants);
            let residual = geodesic.mino_constraint_residuals(seed.initial_state).unwrap();
            close(residual.radial, 0.0, 2.0e-11);
            close(residual.polar, 0.0, 2.0e-11);
            let canonical = KerrCanonicalHamiltonian::new(hole(spin));
            let state = canonical
                .state_from_separated(
                    seed.initial_state.coordinates,
                    seed.constants,
                    KerrMinoDirection::Decreasing,
                    KerrMinoDirection::Increasing,
                )
                .unwrap();
            let h = canonical.evaluate(state).unwrap().hamiltonian;
            close(h, 0.0, 3.0e-11);
            assert!(seed.infinity_energy_per_unit_local_energy > 0.0);
        }
    }

    #[test]
    fn finite_radius_screen_mapping_approaches_bardeen_asymptotic_coordinates() {
        let spin = 0.7;
        let observer = KerrFiniteRadiusObserver::new(hole(spin), 100_000.0, core::f64::consts::FRAC_PI_2).unwrap();
        let radial = -(1.0_f64 - 0.006_f64.powi(2) - 0.008_f64.powi(2)).sqrt();
        let direction = KerrLocalSkyDirection::new(radial, 0.006, -0.008).unwrap();
        let seed = observer.ray_seed(direction).unwrap();
        let screen = seed.constants.to_asymptotic_screen(
            hole(spin),
            core::f64::consts::FRAC_PI_2,
            KerrMinoDirection::Increasing,
        ).unwrap();
        // The finite-radius impact coordinates tend to (-r*n_phi, r*n_theta)
        // in the asymptotic limit; errors vanish as 1/r.
        close(screen.alpha_over_m / 100_000.0, 0.008, 2.0e-4);
        close(screen.beta_over_m / 100_000.0, 0.006, 2.0e-4);
    }

    #[test]
    fn finite_radius_observer_rejects_nonunit_sky_directions_and_axis() {
        assert!(matches!(
            KerrLocalSkyDirection::new(0.1, 0.2, 0.3),
            Err(KerrGeodesicError::InvalidSkyDirection { .. })
        ));
        assert!(matches!(
            KerrFiniteRadiusObserver::new(hole(0.8), 10.0, 0.0),
            Err(KerrGeodesicError::InvalidPolarAngle(_))
        ));
    }


    #[test]
    fn ingoing_mino_derivative_matches_boyer_lindquist_chart_transform() {
        for spin in [0.0, 0.5, 0.9] {
            let constants = KerrNullConstants::new(1.0, 2.0, 0.0).unwrap();
            let geodesic = KerrSeparatedNullGeodesic::new(hole(spin), constants);
            let radius = 10.0;
            let theta = core::f64::consts::FRAC_PI_2;
            let radial = geodesic.radial_potential(radius).unwrap();
            let state = KerrMinoDynamicState::new(
                KerrMinoState::new(0.0, radius, theta, 0.0),
                -radial.sqrt(),
                0.0,
            );
            let bl = geodesic.mino_dynamic_derivative(state).unwrap();
            let ingoing = geodesic.mino_dynamic_derivative_ingoing(state).unwrap();
            let delta = radius * radius - 2.0 * radius + spin * spin;
            close(
                ingoing.d_time_over_m,
                bl.d_time_over_m + (radius * radius + spin * spin) / delta
                    * state.radial_velocity_over_m,
                3e-12,
            );
            close(
                ingoing.d_azimuth_rad,
                bl.d_azimuth_rad + spin / delta * state.radial_velocity_over_m,
                3e-12,
            );
        }
    }

    #[test]
    fn ingoing_mino_derivative_is_finite_at_and_inside_the_outer_horizon() {
        let spin = 0.9;
        let black_hole = hole(spin);
        let constants = KerrNullConstants::new(1.0, 2.0, 0.0).unwrap();
        let geodesic = KerrSeparatedNullGeodesic::new(black_hole, constants);
        let horizon = 1.0 + (1.0 - spin * spin).sqrt();
        for radius in [horizon + 0.1, horizon, 1.2, 1.0] {
            let radial = geodesic.radial_potential(radius).unwrap();
            let state = KerrMinoDynamicState::new(
                KerrMinoState::new(0.0, radius, core::f64::consts::FRAC_PI_2, 0.0),
                -radial.sqrt(),
                0.0,
            );
            let derivative = geodesic.mino_dynamic_derivative_ingoing(state).unwrap();
            assert!([
                derivative.d_time_over_m,
                derivative.d_radius_over_m,
                derivative.d_radial_velocity_over_m,
                derivative.d_theta_rad,
                derivative.d_polar_velocity_rad,
                derivative.d_azimuth_rad,
            ].iter().all(|value| value.is_finite()));
        }
    }

    #[test]
    fn adaptive_ingoing_mino_integrator_crosses_future_outer_horizon() {
        let spin = 0.9;
        let constants = KerrNullConstants::new(1.0, 2.0, 0.0).unwrap();
        let geodesic = KerrSeparatedNullGeodesic::new(hole(spin), constants);
        let radius = 10.0;
        let radial = geodesic.radial_potential(radius).unwrap();
        let initial = KerrMinoDynamicState::new(
            KerrMinoState::new(0.0, radius, core::f64::consts::FRAC_PI_2, 0.0),
            -radial.sqrt(),
            0.0,
        );
        let config = KerrMinoIntegrationConfig::new(
            1.0e-12,
            [1.0e-12; 6],
            1.0e-5,
            1.0e-3,
            1.0e-12,
            2.0e-2,
            200_000,
        )
        .unwrap();
        let trajectory = geodesic
            .integrate_adaptive_ingoing(initial, 1.460_728_111_791_428, config)
            .unwrap();
        let final_state = trajectory.points.last().unwrap().state;
        let horizon = 1.0 + (1.0 - spin * spin).sqrt();
        assert!(final_state.coordinates.radius_over_m < horizon);
        close(final_state.coordinates.radius_over_m, 1.0, 1.0e-6);
        assert!(trajectory.maximum_normalized_radial_constraint_residual < 1.0e-5);
        assert!(trajectory.maximum_normalized_polar_constraint_residual < 1.0e-5);
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
