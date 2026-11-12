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
//! integrates exterior rays with adaptive step-doubling. A canonical Hamiltonian
//! path independently evaluates and integrates Kerr trajectories in
//! Boyer-Lindquist and horizon-regular ingoing charts. Its local 8x8 flow
//! Jacobian is generated from second-order metric AD and checked against
//! finite-difference derivatives and the canonical symplectic identity.
//!
//! This is not yet a propagated observer-screen-to-source Jacobi map or a
//! complete ray-bundle solver: tangent-map integration, screen projection,
//! caustic localization, image-branch matching, and qualified magnification
//! remain open work. The separated Mino integrator is Boyer-Lindquist and
//! stops at its horizon/chart boundary; the canonical ingoing-chart path is
//! the route for future-horizon crossing.

use core::fmt;

use super::{
    KerrBlackHole, PhysicsDomainError,
    autodiff::{DifferentiatedMetric4, JetError},
    connection::{LeviCivitaConnection4, MetricDerivatives4},
    curvature::{MetricSecondDerivatives4, RiemannTensor4},
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

/// Exact equatorial prograde circular-photon-orbit data for a Kerr spin a/M in [0,1].
///
/// This helper is also the natural reference scale for the near-extremal
/// strong-deflection crossover. Let kappa = sqrt(1-a^2). For kappa>0 the
/// critical impact parameter is b_c/M = a + 2 r_ph sqrt(Delta_ph)/(r_ph-1),
/// where r_ph/M = 1 + 2 sin^2(delta/2) + sqrt(3) sin(delta) and
/// delta=(2/3) acos(a). The trigonometric form avoids subtracting two
/// nearly equal numbers in 2[1+cos(2 acos(a)/3)] near extremality.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrEquatorialProgradePhotonOrbit {
    pub spin: f64,
    pub extremality_parameter: f64,
    pub radius_over_m: f64,
    pub critical_impact_parameter_over_m: f64,
}

impl KerrEquatorialProgradePhotonOrbit {
    pub fn from_spin(spin: f64) -> Result<Self, KerrGeodesicError> {
        validate_spin(spin)?;
        if spin < 0.0 {
            return Err(KerrGeodesicError::InvalidProgradeSpin(spin));
        }
        let extremality_parameter = ((1.0 - spin) * (1.0 + spin)).sqrt();
        if extremality_parameter == 0.0 {
            return Ok(Self {
                spin,
                extremality_parameter,
                radius_over_m: 1.0,
                critical_impact_parameter_over_m: 2.0,
            });
        }

        let delta_angle = (2.0 / 3.0) * spin.acos();
        let half_sine = (0.5 * delta_angle).sin();
        let radius_offset = 2.0 * half_sine * half_sine + 3.0_f64.sqrt() * delta_angle.sin();
        let radius_over_m = 1.0 + radius_offset;
        // Delta=(r-1)^2-kappa^2 is better conditioned than r^2-2r+a^2
        // for the near-horizon prograde photon orbit.
        let delta_over_m2 = radius_offset.mul_add(radius_offset, -extremality_parameter.powi(2));
        if !delta_over_m2.is_finite() || delta_over_m2 < 0.0 || radius_offset <= 0.0 {
            return Err(KerrGeodesicError::NonFinite("prograde_photon_orbit_delta"));
        }
        let critical_impact_parameter_over_m =
            spin + 2.0 * radius_over_m * delta_over_m2.sqrt() / radius_offset;
        if !radius_over_m.is_finite()
            || !critical_impact_parameter_over_m.is_finite()
            || critical_impact_parameter_over_m <= 0.0
        {
            return Err(KerrGeodesicError::NonFinite(
                "prograde_critical_impact_parameter",
            ));
        }
        Ok(Self {
            spin,
            extremality_parameter,
            radius_over_m,
            critical_impact_parameter_over_m,
        })
    }

    /// Impact parameter on the scattering side of the critical orbit:
    /// b/M = b_c/M + kappa*chi, where chi must be positive.
    pub fn impact_parameter_for_scaled_offset(
        self,
        scaled_offset_chi: f64,
    ) -> Result<f64, KerrGeodesicError> {
        if !scaled_offset_chi.is_finite()
            || scaled_offset_chi <= 0.0
            || self.extremality_parameter == 0.0
        {
            return Err(KerrGeodesicError::InvalidCriticalOffset(scaled_offset_chi));
        }
        Ok(self.critical_impact_parameter_over_m + self.extremality_parameter * scaled_offset_chi)
    }

    /// Equatorial (Q=0), unit-energy null constants at the scaled critical offset.
    pub fn null_constants_for_scaled_offset(
        self,
        scaled_offset_chi: f64,
    ) -> Result<KerrNullConstants, KerrGeodesicError> {
        let impact_parameter = self.impact_parameter_for_scaled_offset(scaled_offset_chi)?;
        KerrNullConstants::new(1.0, impact_parameter, 0.0)
    }

    /// Leading uniform asymptotic strong-deflection phase from the
    /// near-extremal prograde crossover variable chi:
    /// Phi_LO = kappa^(-1) ln(1 + 2 sqrt(3)/chi).
    ///
    /// This is an asymptotic reference, not an exact finite-kappa deflection.
    pub fn leading_uniform_deflection_phase(
        self,
        scaled_offset_chi: f64,
    ) -> Result<f64, KerrGeodesicError> {
        if !scaled_offset_chi.is_finite()
            || scaled_offset_chi <= 0.0
            || self.extremality_parameter == 0.0
        {
            return Err(KerrGeodesicError::InvalidCriticalOffset(scaled_offset_chi));
        }
        Ok((1.0 + 2.0 * 3.0_f64.sqrt() / scaled_offset_chi).ln() / self.extremality_parameter)
    }
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
        let beta_squared =
            eta + spin * spin * cos_i * cos_i - xi * xi * cos_i * cos_i / (sin_i * sin_i);
        if !beta_squared.is_finite() {
            return Err(KerrGeodesicError::NonFinite("screen_beta_squared"));
        }
        if beta_squared < 0.0 {
            return Err(KerrGeodesicError::NoRealAsymptoticScreen { beta_squared });
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

        // Preserve the small radial potential near a turning point. The
        // leading terms P^2 and Delta*C can nearly cancel; recover their
        // product roundoff with FMA and use a two-difference compensation.
        let p_squared = p * p;
        let p_squared_error = p.mul_add(p, -p_squared);
        let delta_term = delta * angular_term;
        let delta_term_error = delta.mul_add(angular_term, -delta_term);
        let difference = p_squared - delta_term;
        let virtual_delta = p_squared - difference;
        let virtual_p = difference + virtual_delta;
        let delta_roundoff = virtual_delta - delta_term;
        let p_roundoff = p_squared - virtual_p;
        let subtraction_error = p_roundoff + delta_roundoff;
        let value = difference + (subtraction_error + p_squared_error - delta_term_error);
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
        let angular_term = self.carter_constant_over_m2 + (angular_momentum - a * energy).powi(2);
        let delta_derivative = 2.0 * (r - 1.0);
        let value = 4.0 * energy * r * p - delta_derivative * angular_term;
        if !value.is_finite() {
            return Err(KerrGeodesicError::NonFinite("radial_potential_derivative"));
        }
        Ok(value)
    }

    /// Separated polar potential Theta(theta_bar).
    pub fn polar_potential(self, spin: f64, theta_rad: f64) -> Result<f64, KerrGeodesicError> {
        validate_spin(spin)?;
        validate_polar_angle(theta_rad)?;
        // Preserve the exact equatorial invariant submanifold. libm::cos(pi/2)
        // is a small positive value rather than mathematical zero; for Q=0
        // that roundoff otherwise makes Theta spuriously negative and causes
        // a constraint-preserving integrator to reject every step.
        let (sin_theta, cos_theta) = if theta_rad == core::f64::consts::FRAC_PI_2 {
            (1.0, 0.0)
        } else {
            (theta_rad.sin(), theta_rad.cos())
        };
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
        let (sin_theta, cos_theta) = if theta_rad == core::f64::consts::FRAC_PI_2 {
            (1.0, 0.0)
        } else {
            (theta_rad.sin(), theta_rad.cos())
        };
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
    pub fn new(radial: f64, polar: f64, azimuthal: f64) -> Result<Self, KerrGeodesicError> {
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
        Ok(Self {
            radial,
            polar,
            azimuthal,
        })
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
    /// Coordinate chart for the initial state's time/azimuth coordinates and
    /// the preferred matching Mino integration entry point.
    pub coordinate_chart: KerrMinoCoordinateChart,
    pub observer_radius_over_m: f64,
    pub observer_theta_rad: f64,
    pub initial_state: KerrMinoDynamicState,
    /// Killing energy at infinity per unit local ZAMO photon energy.
    pub infinity_energy_per_unit_local_energy: f64,
    pub local_direction: KerrLocalSkyDirection,
}

/// One canonical separation projected onto a finite source ZAMO's local screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrObserverScreenProjection {
    /// Two screen-plane components in the returned source basis.
    pub components: [f64; 2],
    /// Orthonormal local source screen basis in [radial, polar, azimuthal].
    pub local_screen_basis: [[f64; 3]; 2],
    /// Raw source-frame direction norm error before basis normalization.
    pub direction_norm_residual: f64,
}

/// Finite-observer canonical initial data plus two angular screen derivatives.
///
/// The two tangent directions are orthonormal in the observer's local sky
/// tangent plane. Their angular parameters are measured in radians. Each
/// phase-space variation uses canonical ordering (q^0..q^3,p_0..p_3) and is
/// tangent to the null Hamiltonian shell at the initial event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrObserverCanonicalSeed {
    pub state: KerrCanonicalState,
    pub coordinate_chart: KerrMinoCoordinateChart,
    pub local_screen_basis: [[f64; 3]; 2],
    pub first_phase_space_variation: [f64; 8],
    pub second_phase_space_variation: [f64; 8],
    pub infinity_energy_per_unit_local_energy: f64,
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

/// Local linearization of the canonical Kerr Hamiltonian flow.
///
/// State ordering is z = (t_bar, r_bar, theta, phi, p_t, p_r, p_theta, p_phi).
/// jacobian[i][j] = d(dot z_i)/d z_j. The matrix is generated analytically
/// from first and second metric derivatives supplied by Jet2 automatic
/// differentiation. It is the local generator for dJ/dlambda = A J, not
/// itself a propagated screen-to-source optical Jacobi map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrCanonicalLinearization {
    pub evaluation: KerrCanonicalEvaluation,
    pub jacobian: [[f64; 8]; 8],
    /// max absolute component of A^T Omega + Omega A.
    pub symplectic_lie_residual: f64,
}

/**
 * Local Sachs optical-tidal matrix from the Kerr Riemann tensor.
 *
 * Vectors use the same dimensionless contravariant coordinate basis as the
 * canonical state: [ct/M or v/M, r/M, theta, phi]. The declared Riemann
 * convention is inherited from curvature::RiemannTensor4.
 * The tidal convention is
 * T_AB = R_(mu nu rho sigma) e_A^mu k^nu e_B^rho k^sigma.
 *
 * This is a pointwise curvature observable. It is not an integrated Jacobi
 * matrix, a caustic finder, or a finite-frequency wave-optics calculation.
 */
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrOpticalTidalEvaluation {
    pub coordinate_chart: KerrMinoCoordinateChart,
    pub tidal_matrix: [[f64; 2]; 2],
    pub null_constraint_residual: f64,
    pub screen_orthonormality_residual: f64,
    pub screen_transversality_residual: f64,
    pub tidal_symmetry_residual: f64,
    /// For vacuum Kerr, abs(trace(T)) should vanish with the Ricci contraction.
    pub vacuum_trace_residual: f64,
}

/// Local right-hand side for parallel transport of a two-vector Sachs screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrSachsFrameTransportDerivative {
    pub coordinate_chart: KerrMinoCoordinateChart,
    /// de_A^mu/dlambda = -Gamma^mu_(alpha beta) k^alpha e_A^beta.
    pub screen_basis_derivative: [[f64; 4]; 2],
    pub null_constraint_residual: f64,
    pub screen_orthonormality_residual: f64,
    pub screen_transversality_residual: f64,
    pub orthonormality_rate_residual: f64,
    pub transversality_rate_residual: f64,
    pub metric_compatibility_residual: f64,
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
    InvalidProgradeSpin(f64),
    InvalidCriticalOffset(f64),
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
    InvalidProjectionTolerance(f64),
    ObserverEventMismatch {
        radius_difference: f64,
        polar_difference: f64,
        tolerance: f64,
    },
    LocalNullResidual {
        residual: f64,
        tolerance: f64,
    },
    OpticalFrameConstraintViolation {
        null_residual: f64,
        orthonormality_residual: f64,
        transversality_residual: f64,
        tolerance: f64,
    },
    OpticalTidalAsymmetry {
        residual: f64,
        tolerance: f64,
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
                write!(
                    f,
                    "Kerr null-ray energy must be finite and positive, got {value}"
                )
            }
            Self::InvalidSpin(value) => {
                write!(f, "dimensionless Kerr spin {value} is outside [-1, 1]")
            }
            Self::InvalidProgradeSpin(value) => {
                write!(
                    f,
                    "equatorial prograde photon orbit requires spin in [0, 1], got {value}"
                )
            }
            Self::InvalidCriticalOffset(value) => {
                write!(
                    f,
                    "near-extremal critical offset chi must be finite and positive, got {value}"
                )
            }
            Self::InvalidRadius(value) => {
                write!(
                    f,
                    "dimensionless Kerr radius {value} must be finite and positive"
                )
            }
            Self::InvalidInclination(value) => {
                write!(
                    f,
                    "asymptotic observer inclination {value} must lie in (0, pi)"
                )
            }
            Self::InvalidPolarAngle(value) => {
                write!(f, "Boyer-Lindquist polar angle {value} must lie in (0, pi)")
            }
            Self::RadiusAtOrInsideHorizon {
                radius_over_m,
                horizon_over_m,
            } => write!(
                f,
                "Boyer-Lindquist exterior radius {radius_over_m} must exceed horizon {horizon_over_m}"
            ),
            Self::ForbiddenRadialRegion {
                radius_over_m,
                radial_potential,
            } => write!(
                f,
                "radial potential {radial_potential} is negative at r/M={radius_over_m}"
            ),
            Self::ForbiddenPolarRegion {
                theta_rad,
                polar_potential,
            } => write!(
                f,
                "polar potential {polar_potential} is negative at theta={theta_rad}"
            ),
            Self::BoyerLindquistTurningSingularity { delta } => {
                write!(
                    f,
                    "Boyer-Lindquist Mino equations are singular at Delta={delta}"
                )
            }
            Self::NoRealAsymptoticScreen { beta_squared } => {
                write!(
                    f,
                    "constants do not map to a real asymptotic screen: beta^2={beta_squared}"
                )
            }
            Self::ScreenMapInconsistent {
                actual_theta_potential,
                expected_beta_squared,
            } => write!(
                f,
                "screen map gives Theta={actual_theta_potential}, expected beta^2={expected_beta_squared}"
            ),
            Self::InvalidSkyDirection {
                radial,
                polar,
                azimuthal,
                norm_squared,
            } => write!(
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
            Self::InvalidProjectionTolerance(value) => write!(
                f,
                "screen projection tolerance must be finite and positive, got {value}"
            ),
            Self::ObserverEventMismatch {
                radius_difference,
                polar_difference,
                tolerance,
            } => write!(
                f,
                "source observer does not match ray event: radius difference {radius_difference}, polar difference {polar_difference}, tolerance {tolerance}"
            ),
            Self::LocalNullResidual {
                residual,
                tolerance,
            } => write!(
                f,
                "local source momentum is not sufficiently null: unit-direction residual {residual} exceeds {tolerance}"
            ),
            Self::OpticalFrameConstraintViolation {
                null_residual,
                orthonormality_residual,
                transversality_residual,
                tolerance,
            } => write!(
                f,
                "Sachs frame constraints failed: null={null_residual}, orthonormality={orthonormality_residual}, transversality={transversality_residual}, tolerance={tolerance}"
            ),
            Self::OpticalTidalAsymmetry {
                residual,
                tolerance,
            } => write!(
                f,
                "Sachs optical tidal matrix asymmetry {residual} exceeds {tolerance}"
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
        self.constants
            .radial_potential(self.hole.spin().get(), radius_over_m)
    }

    pub fn radial_potential_derivative(self, radius_over_m: f64) -> Result<f64, KerrGeodesicError> {
        self.constants
            .radial_potential_derivative(self.hole.spin().get(), radius_over_m)
    }

    pub fn polar_potential(self, theta_rad: f64) -> Result<f64, KerrGeodesicError> {
        self.constants
            .polar_potential(self.hole.spin().get(), theta_rad)
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
        let d_time =
            a * (angular_momentum - a * energy * sin_squared) + (r * r + a * a) * p / delta;
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
        let polar_acceleration = 0.5 * self.constants.polar_potential_derivative(a, theta)?;
        let energy = self.constants.energy;
        let angular_momentum = self.constants.angular_momentum_z_over_m;
        let sin_theta = theta.sin();
        let sin_squared = sin_theta * sin_theta;
        let p = energy * (r * r + a * a) - a * angular_momentum;
        let d_time =
            a * (angular_momentum - a * energy * sin_squared) + (r * r + a * a) * p / delta;
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
                return Err(KerrGeodesicError::NoRegularIngoingBranch {
                    radial_denominator: denominator,
                });
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
        let d_time = a * (angular_momentum - a * energy * sin_squared) + (r * r + a * a) * ratio;
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
            return Err(KerrGeodesicError::NonFinite(
                "ingoing_mino_dynamic_derivative",
            ));
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
        let polar = state.polar_velocity_rad.powi(2) - self.polar_potential(theta)?;
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
        let horizon = self.hole.outer_horizon_radius_m() / self.hole.gravitational_radius_m();
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
        let radial_denominator = p + root;
        if !radial_denominator.is_finite() || radial_denominator <= 0.0 {
            return Err(KerrGeodesicError::NoRegularIngoingBranch { radial_denominator });
        }
        let angular_term =
            constants.carter_constant_over_m2 + (angular_momentum - a * energy).powi(2);
        let radial_momentum = angular_term / radial_denominator;
        let momenta = [
            -energy,
            radial_momentum,
            polar_direction.sign() * polar_potential.sqrt(),
            angular_momentum,
        ];
        if momenta.iter().any(|value| !value.is_finite()) {
            return Err(KerrGeodesicError::NonFinite(
                "ingoing_canonical_initial_momenta",
            ));
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

    /// Evaluate the local Sachs optical-tidal matrix from the independent
    /// metric-curvature formulation in the requested Kerr coordinate chart.
    ///
    /// The null tangent and each screen row are contravariant vectors in the
    /// dimensionless chart coordinate basis. The screen rows must be
    /// orthonormal spacelike vectors transverse to the null tangent. Event
    /// and frame transport between different samples remains the caller's
    /// responsibility.
    pub fn optical_tidal_matrix(
        self,
        state: KerrCanonicalState,
        coordinate_chart: KerrMinoCoordinateChart,
        screen_basis: [[f64; 4]; 2],
        tolerance: f64,
    ) -> Result<KerrOpticalTidalEvaluation, KerrGeodesicError> {
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err(KerrGeodesicError::InvalidProjectionTolerance(tolerance));
        }
        let radius_m = state.coordinates.radius_over_m * self.hole.gravitational_radius_m();
        let differentiated = match coordinate_chart {
            KerrMinoCoordinateChart::BoyerLindquist => DifferentiatedMetric4::kerr_exterior(
                self.hole,
                radius_m,
                state.coordinates.theta_rad,
            )?,
            KerrMinoCoordinateChart::IngoingKerr => DifferentiatedMetric4::kerr_ingoing(
                self.hole,
                radius_m,
                state.coordinates.theta_rad,
            )?,
        };
        let evaluation = self.evaluate_differentiated(state, differentiated)?;
        let null_tangent = evaluation.derivative.coordinates;
        let mass_length = self.hole.gravitational_radius_m();
        let factors = [1.0, 1.0, 1.0 / mass_length, 1.0 / mass_length];
        let source_metric = differentiated.metric.covariant();
        let source_first = differentiated.first.components();
        let source_second = differentiated.second.components();

        // Convert the SI-coordinate differentiated metric to the exact
        // dimensionless canonical chart convention used by the Hamiltonian
        // flow, including radial/polar derivative scale factors.
        let mut metric_components = [[0.0; 4]; 4];
        let mut first_components = [[[0.0; 4]; 4]; 4];
        let mut second_components = [[[[0.0; 4]; 4]; 4]; 4];
        for mu in 0..4 {
            for nu in 0..4 {
                let scale = factors[mu] * factors[nu];
                metric_components[mu][nu] = source_metric[mu][nu] * scale;
                first_components[1][mu][nu] = source_first[1][mu][nu] * mass_length * scale;
                first_components[2][mu][nu] = source_first[2][mu][nu] * scale;
                second_components[1][1][mu][nu] =
                    source_second[1][1][mu][nu] * mass_length * mass_length * scale;
                second_components[1][2][mu][nu] = source_second[1][2][mu][nu] * mass_length * scale;
                second_components[2][1][mu][nu] = source_second[2][1][mu][nu] * mass_length * scale;
                second_components[2][2][mu][nu] = source_second[2][2][mu][nu] * scale;
            }
        }
        let metric = MetricComponents4::new(metric_components)?;
        let first = MetricDerivatives4::new(first_components).map_err(JetError::from)?;
        let second = MetricSecondDerivatives4::new(second_components).map_err(JetError::from)?;
        let curvature = RiemannTensor4::from_metric_derivatives(&metric, &first, &second)
            .map_err(JetError::from)?;
        let riemann_covariant = curvature.fully_covariant(&metric).map_err(JetError::from)?;
        let g = metric.covariant();

        let metric_inner = |left: [f64; 4], right: [f64; 4]| -> f64 {
            let mut value = 0.0;
            for mu in 0..4 {
                for nu in 0..4 {
                    value += g[mu][nu] * left[mu] * right[nu];
                }
            }
            value
        };
        let null_constraint_residual = metric_inner(null_tangent, null_tangent).abs();
        let mut screen_orthonormality_residual: f64 = 0.0;
        let mut screen_transversality_residual: f64 = 0.0;
        for first_screen in 0..2 {
            screen_transversality_residual = screen_transversality_residual
                .max(metric_inner(screen_basis[first_screen], null_tangent).abs());
            for second_screen in 0..2 {
                let expected = if first_screen == second_screen {
                    1.0
                } else {
                    0.0
                };
                screen_orthonormality_residual = screen_orthonormality_residual.max(
                    (metric_inner(screen_basis[first_screen], screen_basis[second_screen])
                        - expected)
                        .abs(),
                );
            }
        }
        if !null_constraint_residual.is_finite()
            || !screen_orthonormality_residual.is_finite()
            || !screen_transversality_residual.is_finite()
            || null_constraint_residual > tolerance
            || screen_orthonormality_residual > tolerance
            || screen_transversality_residual > tolerance
        {
            return Err(KerrGeodesicError::OpticalFrameConstraintViolation {
                null_residual: null_constraint_residual,
                orthonormality_residual: screen_orthonormality_residual,
                transversality_residual: screen_transversality_residual,
                tolerance,
            });
        }

        let mut tidal_matrix = [[0.0; 2]; 2];
        for first_screen in 0..2 {
            for second_screen in 0..2 {
                for mu in 0..4 {
                    for nu in 0..4 {
                        for rho in 0..4 {
                            for sigma in 0..4 {
                                tidal_matrix[first_screen][second_screen] += riemann_covariant[mu]
                                    [nu][rho][sigma]
                                    * screen_basis[first_screen][mu]
                                    * null_tangent[nu]
                                    * screen_basis[second_screen][rho]
                                    * null_tangent[sigma];
                            }
                        }
                    }
                }
            }
        }
        if tidal_matrix
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
        {
            return Err(KerrGeodesicError::NonFinite("optical_tidal_matrix"));
        }
        let tidal_symmetry_residual = (tidal_matrix[0][1] - tidal_matrix[1][0]).abs();
        let tidal_scale = tidal_matrix
            .iter()
            .flatten()
            .fold(0.0_f64, |scale, value| scale.max(value.abs()));
        if tidal_symmetry_residual > tolerance * (1.0 + tidal_scale) {
            return Err(KerrGeodesicError::OpticalTidalAsymmetry {
                residual: tidal_symmetry_residual,
                tolerance: tolerance * (1.0 + tidal_scale),
            });
        }

        Ok(KerrOpticalTidalEvaluation {
            coordinate_chart,
            tidal_matrix,
            null_constraint_residual,
            screen_orthonormality_residual,
            screen_transversality_residual,
            tidal_symmetry_residual,
            vacuum_trace_residual: (tidal_matrix[0][0] + tidal_matrix[1][1]).abs(),
        })
    }

    /// Evaluate the covariant transport derivative of a pointwise Sachs
    /// screen frame along a canonical Kerr null geodesic.
    ///
    /// This returns the ODE right-hand side for a parallel-transported screen:
    /// de_A^mu/dlambda = -Gamma^mu_(alpha beta) k^alpha e_A^beta.
    /// It does not step or renormalize the frame. The returned rate residuals
    /// independently check that the metric inner products and transversality
    /// are preserved by the supplied connection and affine geodesic tangent.
    pub fn sachs_frame_transport_derivative(
        self,
        state: KerrCanonicalState,
        coordinate_chart: KerrMinoCoordinateChart,
        screen_basis: [[f64; 4]; 2],
        tolerance: f64,
    ) -> Result<KerrSachsFrameTransportDerivative, KerrGeodesicError> {
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err(KerrGeodesicError::InvalidProjectionTolerance(tolerance));
        }
        let radius_m = state.coordinates.radius_over_m * self.hole.gravitational_radius_m();
        let differentiated = match coordinate_chart {
            KerrMinoCoordinateChart::BoyerLindquist => DifferentiatedMetric4::kerr_exterior(
                self.hole,
                radius_m,
                state.coordinates.theta_rad,
            )?,
            KerrMinoCoordinateChart::IngoingKerr => DifferentiatedMetric4::kerr_ingoing(
                self.hole,
                radius_m,
                state.coordinates.theta_rad,
            )?,
        };
        let evaluation = self.evaluate_differentiated(state, differentiated)?;
        let k = evaluation.derivative.coordinates;
        let mass_length = self.hole.gravitational_radius_m();
        let factors = [1.0, 1.0, 1.0 / mass_length, 1.0 / mass_length];
        let source_metric = differentiated.metric.covariant();
        let source_first = differentiated.first.components();
        let mut metric_components = [[0.0; 4]; 4];
        let mut first_components = [[[0.0; 4]; 4]; 4];
        for mu in 0..4 {
            for nu in 0..4 {
                let scale = factors[mu] * factors[nu];
                metric_components[mu][nu] = source_metric[mu][nu] * scale;
                first_components[1][mu][nu] = source_first[1][mu][nu] * mass_length * scale;
                first_components[2][mu][nu] = source_first[2][mu][nu] * scale;
            }
        }
        let metric = MetricComponents4::new(metric_components)?;
        let first = MetricDerivatives4::new(first_components).map_err(JetError::from)?;
        let connection = LeviCivitaConnection4::from_metric_and_derivatives(&metric, &first)
            .map_err(JetError::from)?;
        let gamma = connection.symbols();
        let g = metric.covariant();
        let d_g = first.components();
        let metric_inner = |left: [f64; 4], right: [f64; 4]| -> f64 {
            let mut value = 0.0;
            for mu in 0..4 {
                for nu in 0..4 {
                    value += g[mu][nu] * left[mu] * right[nu];
                }
            }
            value
        };
        let null_constraint_residual = metric_inner(k, k).abs();
        let mut screen_orthonormality_residual: f64 = 0.0;
        let mut screen_transversality_residual: f64 = 0.0;
        for first_screen in 0..2 {
            screen_transversality_residual = screen_transversality_residual
                .max(metric_inner(screen_basis[first_screen], k).abs());
            for second_screen in 0..2 {
                let expected = if first_screen == second_screen {
                    1.0
                } else {
                    0.0
                };
                screen_orthonormality_residual = screen_orthonormality_residual.max(
                    (metric_inner(screen_basis[first_screen], screen_basis[second_screen])
                        - expected)
                        .abs(),
                );
            }
        }
        if !null_constraint_residual.is_finite()
            || !screen_orthonormality_residual.is_finite()
            || !screen_transversality_residual.is_finite()
            || null_constraint_residual > tolerance
            || screen_orthonormality_residual > tolerance
            || screen_transversality_residual > tolerance
        {
            return Err(KerrGeodesicError::OpticalFrameConstraintViolation {
                null_residual: null_constraint_residual,
                orthonormality_residual: screen_orthonormality_residual,
                transversality_residual: screen_transversality_residual,
                tolerance,
            });
        }

        let mut screen_basis_derivative = [[0.0; 4]; 2];
        for screen in 0..2 {
            for mu in 0..4 {
                for alpha in 0..4 {
                    for beta in 0..4 {
                        screen_basis_derivative[screen][mu] -=
                            gamma[mu][alpha][beta] * k[alpha] * screen_basis[screen][beta];
                    }
                }
            }
        }
        let mut geodesic_acceleration = [0.0; 4];
        for mu in 0..4 {
            for alpha in 0..4 {
                for beta in 0..4 {
                    geodesic_acceleration[mu] -= gamma[mu][alpha][beta] * k[alpha] * k[beta];
                }
            }
        }

        let mut orthonormality_rate_residual: f64 = 0.0;
        for first_screen in 0..2 {
            for second_screen in 0..2 {
                let mut rate = 0.0;
                for alpha in 0..4 {
                    for mu in 0..4 {
                        for nu in 0..4 {
                            rate += k[alpha]
                                * d_g[alpha][mu][nu]
                                * screen_basis[first_screen][mu]
                                * screen_basis[second_screen][nu];
                        }
                    }
                }
                for mu in 0..4 {
                    for nu in 0..4 {
                        rate += g[mu][nu]
                            * screen_basis_derivative[first_screen][mu]
                            * screen_basis[second_screen][nu]
                            + g[mu][nu]
                                * screen_basis[first_screen][mu]
                                * screen_basis_derivative[second_screen][nu];
                    }
                }
                orthonormality_rate_residual = orthonormality_rate_residual.max(rate.abs());
            }
        }

        let mut transversality_rate_residual: f64 = 0.0;
        for screen in 0..2 {
            let mut rate = 0.0;
            for alpha in 0..4 {
                for mu in 0..4 {
                    for nu in 0..4 {
                        rate += k[alpha] * d_g[alpha][mu][nu] * screen_basis[screen][mu] * k[nu];
                    }
                }
            }
            for mu in 0..4 {
                for nu in 0..4 {
                    rate += g[mu][nu] * screen_basis_derivative[screen][mu] * k[nu]
                        + g[mu][nu] * screen_basis[screen][mu] * geodesic_acceleration[nu];
                }
            }
            transversality_rate_residual = transversality_rate_residual.max(rate.abs());
        }
        let metric_compatibility_residual = connection
            .metric_compatibility_relative_residual(&metric, &first)
            .map_err(JetError::from)?;
        if screen_basis_derivative
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
            || !orthonormality_rate_residual.is_finite()
            || !transversality_rate_residual.is_finite()
            || !metric_compatibility_residual.is_finite()
        {
            return Err(KerrGeodesicError::NonFinite(
                "sachs_frame_transport_derivative",
            ));
        }
        Ok(KerrSachsFrameTransportDerivative {
            coordinate_chart,
            screen_basis_derivative,
            null_constraint_residual,
            screen_orthonormality_residual,
            screen_transversality_residual,
            orthonormality_rate_residual,
            transversality_rate_residual,
            metric_compatibility_residual,
        })
    }

    /// Linearize the canonical Hamiltonian flow in exterior Boyer-Lindquist
    /// coordinates using exact second-order metric derivatives.
    pub fn linearize(
        self,
        state: KerrCanonicalState,
    ) -> Result<KerrCanonicalLinearization, KerrGeodesicError> {
        let radius_m = state.coordinates.radius_over_m * self.hole.gravitational_radius_m();
        let differentiated =
            DifferentiatedMetric4::kerr_exterior(self.hole, radius_m, state.coordinates.theta_rad)?;
        self.linearize_differentiated(state, differentiated)
    }

    /// Horizon-regular ingoing-chart counterpart to linearize.
    pub fn linearize_ingoing(
        self,
        state: KerrCanonicalState,
    ) -> Result<KerrCanonicalLinearization, KerrGeodesicError> {
        let radius_m = state.coordinates.radius_over_m * self.hole.gravitational_radius_m();
        let differentiated =
            DifferentiatedMetric4::kerr_ingoing(self.hole, radius_m, state.coordinates.theta_rad)?;
        self.linearize_differentiated(state, differentiated)
    }

    fn linearize_differentiated(
        self,
        state: KerrCanonicalState,
        differentiated: DifferentiatedMetric4,
    ) -> Result<KerrCanonicalLinearization, KerrGeodesicError> {
        let evaluation = self.evaluate_differentiated(state, differentiated)?;
        let mass_length = self.hole.gravitational_radius_m();
        let factors = [1.0, 1.0, 1.0 / mass_length, 1.0 / mass_length];
        let source_metric = differentiated.metric.covariant();
        let source_first = differentiated.first.components();
        let source_second = differentiated.second.components();

        let mut metric = [[0.0; 4]; 4];
        let mut first = [[[0.0; 4]; 4]; 4];
        let mut second = [[[[0.0; 4]; 4]; 4]; 4];
        for mu in 0..4 {
            for nu in 0..4 {
                let scale = factors[mu] * factors[nu];
                metric[mu][nu] = source_metric[mu][nu] * scale;
                first[1][mu][nu] = source_first[1][mu][nu] * mass_length * scale;
                first[2][mu][nu] = source_first[2][mu][nu] * scale;
                second[1][1][mu][nu] =
                    source_second[1][1][mu][nu] * mass_length * mass_length * scale;
                second[1][2][mu][nu] = source_second[1][2][mu][nu] * mass_length * scale;
                second[2][1][mu][nu] = source_second[2][1][mu][nu] * mass_length * scale;
                second[2][2][mu][nu] = source_second[2][2][mu][nu] * scale;
            }
        }

        let inverse = MetricComponents4::new(metric)?.inverse()?;
        // Derivatives of the inverse metric:
        // G_i = -G g_i G
        // G_ij = G g_j G g_i G + G g_i G g_j G - G g_ij G.
        let mut inverse_first = [[[0.0; 4]; 4]; 4];
        for coordinate in 1..=2 {
            for mu in 0..4 {
                for nu in 0..4 {
                    for a in 0..4 {
                        for b in 0..4 {
                            inverse_first[coordinate][mu][nu] -=
                                inverse[mu][a] * first[coordinate][a][b] * inverse[b][nu];
                        }
                    }
                }
            }
        }

        let mut inverse_second = [[[[0.0; 4]; 4]; 4]; 4];
        for i in 1..=2 {
            for j in 1..=2 {
                for mu in 0..4 {
                    for nu in 0..4 {
                        for a in 0..4 {
                            for b in 0..4 {
                                for c in 0..4 {
                                    for d in 0..4 {
                                        inverse_second[i][j][mu][nu] += inverse[mu][a]
                                            * first[j][a][b]
                                            * inverse[b][c]
                                            * first[i][c][d]
                                            * inverse[d][nu];
                                        inverse_second[i][j][mu][nu] += inverse[mu][a]
                                            * first[i][a][b]
                                            * inverse[b][c]
                                            * first[j][c][d]
                                            * inverse[d][nu];
                                    }
                                }
                            }
                        }
                        for a in 0..4 {
                            for b in 0..4 {
                                inverse_second[i][j][mu][nu] -=
                                    inverse[mu][a] * second[i][j][a][b] * inverse[b][nu];
                            }
                        }
                    }
                }
            }
        }

        let p = state.momenta;
        let mut jacobian = [[0.0; 8]; 8];
        // d(qdot^mu)/dq^k = (d_k g^mu nu) p_nu.
        // d(qdot^mu)/dp_nu = g^mu nu.
        for mu in 0..4 {
            for nu in 0..4 {
                jacobian[mu][4 + nu] = inverse[mu][nu];
            }
            for coordinate in 1..=2 {
                for nu in 0..4 {
                    jacobian[mu][coordinate] += inverse_first[coordinate][mu][nu] * p[nu];
                }
            }
        }

        // pdot_mu = -1/2 p^T (d_mu g^{-1}) p; its position derivative
        // therefore uses the second derivative of the inverse metric.
        for momentum_index in 1..=2 {
            for coordinate in 1..=2 {
                for a in 0..4 {
                    for b in 0..4 {
                        jacobian[4 + momentum_index][coordinate] -=
                            0.5 * p[a] * inverse_second[momentum_index][coordinate][a][b] * p[b];
                    }
                }
            }
            for nu in 0..4 {
                for b in 0..4 {
                    jacobian[4 + momentum_index][4 + nu] -=
                        inverse_first[momentum_index][nu][b] * p[b];
                }
            }
        }

        if jacobian.iter().flatten().any(|value| !value.is_finite()) {
            return Err(KerrGeodesicError::NonFinite("canonical_flow_linearization"));
        }
        let symplectic_lie_residual = canonical_symplectic_lie_residual(&jacobian);
        if !symplectic_lie_residual.is_finite() {
            return Err(KerrGeodesicError::NonFinite(
                "canonical_symplectic_lie_residual",
            ));
        }
        Ok(KerrCanonicalLinearization {
            evaluation,
            jacobian,
            symplectic_lie_residual,
        })
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
                radial_derivative[mu][nu] = source_first[1][mu][nu] * mass_length * scale;
                polar_derivative[mu][nu] = source_first[2][mu][nu] * scale;
            }
        }
        let metric = MetricComponents4::new(covariant)?;
        let inverse = metric.inverse()?;

        let mut velocity = [0.0; 4];
        for (velocity_component, inverse_row) in velocity.iter_mut().zip(inverse.iter()) {
            *velocity_component = inverse_row
                .iter()
                .zip(state.momenta.iter())
                .map(|(inverse_component, momentum)| inverse_component * momentum)
                .sum();
        }

        let mut momentum_derivative = [0.0; 4];
        // Stationarity and axisymmetry make dp_t/dlambda = dp_phi/dlambda = 0.
        for (coordinate, derivative) in [(1usize, &radial_derivative), (2usize, &polar_derivative)]
        {
            let mut value = 0.0;
            for (velocity_mu, derivative_row) in velocity.iter().zip(derivative.iter()) {
                for (derivative_component, velocity_nu) in
                    derivative_row.iter().zip(velocity.iter())
                {
                    value += velocity_mu * derivative_component * velocity_nu;
                }
            }
            momentum_derivative[coordinate] = 0.5 * value;
        }

        let hamiltonian = 0.5
            * state
                .momenta
                .iter()
                .zip(velocity.iter())
                .map(|(momentum, velocity_component)| momentum * velocity_component)
                .sum::<f64>();

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
            return Err(KerrGeodesicError::NonFinite(
                "canonical_hamiltonian_evaluation",
            ));
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

fn canonical_symplectic_omega(row: usize, column: usize) -> f64 {
    if row < 4 && column == row + 4 {
        1.0
    } else if row >= 4 && column + 4 == row {
        -1.0
    } else {
        0.0
    }
}

/// Compute max absolute component of A^T Omega + Omega A.
fn canonical_symplectic_lie_residual(a: &[[f64; 8]; 8]) -> f64 {
    let mut maximum: f64 = 0.0;
    for row in 0..8 {
        for column in 0..8 {
            let mut value = 0.0;
            for (k, a_row) in a.iter().enumerate() {
                value += a_row[row] * canonical_symplectic_omega(k, column)
                    + canonical_symplectic_omega(row, k) * a[k][column];
            }
            maximum = maximum.max(value.abs());
        }
    }
    maximum
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
            return Err(KerrMinoIntegrationError::InvalidConfiguration(
                "relative_tolerance",
            ));
        }
        if absolute_tolerance
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
        {
            return Err(KerrMinoIntegrationError::InvalidConfiguration(
                "absolute_tolerance",
            ));
        }
        if !constraint_tolerance.is_finite() || constraint_tolerance <= 0.0 {
            return Err(KerrMinoIntegrationError::InvalidConfiguration(
                "constraint_tolerance",
            ));
        }
        if !initial_step.is_finite() || initial_step <= 0.0 {
            return Err(KerrMinoIntegrationError::InvalidConfiguration(
                "initial_step",
            ));
        }
        if !minimum_step.is_finite() || minimum_step <= 0.0 {
            return Err(KerrMinoIntegrationError::InvalidConfiguration(
                "minimum_step",
            ));
        }
        if !maximum_step.is_finite() || maximum_step < minimum_step {
            return Err(KerrMinoIntegrationError::InvalidConfiguration(
                "maximum_step",
            ));
        }
        if initial_step < minimum_step || initial_step > maximum_step {
            return Err(KerrMinoIntegrationError::InvalidConfiguration(
                "initial_step_bounds",
            ));
        }
        if maximum_attempts == 0 {
            return Err(KerrMinoIntegrationError::InvalidConfiguration(
                "maximum_attempts",
            ));
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
    /// Post-projection residuals of the stored state.
    pub constraint_residuals: KerrMinoConstraintResiduals,
    /// Raw first-integral residuals of the unprojected RK4 candidate.
    pub unprojected_constraint_residuals: KerrMinoConstraintResiduals,
    /// Signed changes applied to the radial/polar velocities by the explicit
    /// first-integral projection. Zero for the initial sample.
    pub radial_velocity_projection_correction: f64,
    pub polar_velocity_projection_correction: f64,
}

/// Completed Kerr null trajectory and numerical diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct KerrMinoTrajectory {
    /// Coordinate chart that gives meaning to the time/azimuth slots in every state.
    pub coordinate_chart: KerrMinoCoordinateChart,
    /// Initial state followed by every accepted step endpoint.
    pub points: Vec<KerrMinoTrajectoryPoint>,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub suggested_next_step: f64,
    /// Maximum post-projection radial/polar invariant residual.
    pub maximum_normalized_radial_constraint_residual: f64,
    pub maximum_normalized_polar_constraint_residual: f64,
    /// Maximum pre-projection residual across accepted RK4 candidates.
    pub maximum_unprojected_normalized_radial_constraint_residual: f64,
    pub maximum_unprojected_normalized_polar_constraint_residual: f64,
    /// Maximum absolute velocity correction applied at an accepted step.
    pub maximum_radial_velocity_projection_correction: f64,
    pub maximum_polar_velocity_projection_correction: f64,
    pub requested_final_parameter: f64,
}

/// Conservative horizon-capture classification derived from an integrated trajectory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KerrMinoCaptureStatus {
    /// An ingoing-chart trajectory contains an outside-to-inside outer-horizon crossing.
    CapturedByHorizon,
    /// No crossing appears in the integrated interval; this does not establish escape.
    NoCrossingWithinIntegratedInterval,
    /// Boyer–Lindquist coordinates cannot qualify a future-horizon crossing.
    NotQualifiableInBoyerLindquist,
    /// The trajectory's first-integral residuals exceed the requested ceiling.
    ConstraintDriftExceeded,
    /// No complete trajectory points are available.
    IncompleteTrajectory,
}

impl KerrMinoTrajectory {
    /// Qualify whether this trajectory crossed the future outer horizon.
    ///
    /// A capture is returned only for an explicitly ingoing Kerr trajectory
    /// whose sampled points include a transition from r>r_+ to r<r_+, and
    /// whose recorded radial/polar first-integral residuals satisfy the caller's
    /// ceiling. A trajectory that ends outside the horizon is never labelled
    /// as escaped; its interval may simply be too short.
    pub fn capture_status(
        &self,
        hole: KerrBlackHole,
        maximum_constraint_residual: f64,
    ) -> KerrMinoCaptureStatus {
        if self.points.len() < 2 {
            return KerrMinoCaptureStatus::IncompleteTrajectory;
        }
        if self.coordinate_chart != KerrMinoCoordinateChart::IngoingKerr {
            return KerrMinoCaptureStatus::NotQualifiableInBoyerLindquist;
        }
        if !maximum_constraint_residual.is_finite()
            || maximum_constraint_residual <= 0.0
            || self.maximum_normalized_radial_constraint_residual > maximum_constraint_residual
            || self.maximum_normalized_polar_constraint_residual > maximum_constraint_residual
            || self.maximum_unprojected_normalized_radial_constraint_residual
                > maximum_constraint_residual
            || self.maximum_unprojected_normalized_polar_constraint_residual
                > maximum_constraint_residual
        {
            return KerrMinoCaptureStatus::ConstraintDriftExceeded;
        }
        let horizon = hole.outer_horizon_radius_m() / hole.gravitational_radius_m();
        let mut previous_radius = self.points[0].state.coordinates.radius_over_m;
        if previous_radius <= horizon {
            return KerrMinoCaptureStatus::NoCrossingWithinIntegratedInterval;
        }
        for point in self.points.iter().skip(1) {
            let radius = point.state.coordinates.radius_over_m;
            if previous_radius > horizon && radius < horizon {
                return KerrMinoCaptureStatus::CapturedByHorizon;
            }
            previous_radius = radius;
        }
        KerrMinoCaptureStatus::NoCrossingWithinIntegratedInterval
    }
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
        next_step: f64,
        normalized_local_error: f64,
        normalized_constraint_error: f64,
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
                write!(
                    f,
                    "final Mino parameter must be finite and >= 0, got {value}"
                )
            }
            Self::InitialConstraintViolation {
                radial_relative,
                polar_relative,
            } => write!(
                f,
                "initial Kerr null first-integral residual exceeds tolerance: radial={radial_relative:e}, polar={polar_relative:e}"
            ),
            Self::ConstraintDriftExceeded {
                mino_parameter,
                radial_relative,
                polar_relative,
                tolerance,
            } => write!(
                f,
                "Kerr Mino first-integral drift exceeded tolerance at gamma={mino_parameter:e}: radial={radial_relative:e}, polar={polar_relative:e}, tolerance={tolerance:e}"
            ),
            Self::StepSizeUnderflow {
                mino_parameter,
                attempted_step,
                normalized_error,
            } => write!(
                f,
                "Kerr Mino step underflow at gamma={mino_parameter:e}, step={attempted_step:e}, error={normalized_error:e}"
            ),
            Self::MaximumAttemptsExceeded {
                mino_parameter,
                attempts,
                next_step,
                normalized_local_error,
                normalized_constraint_error,
            } => write!(
                f,
                "Kerr Mino integrator exceeded {attempts} attempts at gamma={mino_parameter:e}; next_step={next_step:e}, local_error={normalized_local_error:e}, constraint_error={normalized_constraint_error:e}"
            ),
            Self::NonFiniteErrorEstimate => {
                write!(
                    f,
                    "Kerr Mino integrator produced a non-finite error estimate"
                )
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
            KerrMinoCoordinateChart::BoyerLindquist => {
                self.mino_dynamic_derivative(dynamic_state)?
            }
            KerrMinoCoordinateChart::IngoingKerr => {
                self.mino_dynamic_derivative_ingoing(dynamic_state)?
            }
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
        chart: KerrMinoCoordinateChart,
    ) -> Result<MinoVector, KerrGeodesicError> {
        let k1 = self.mino_vector_field(state, chart)?;
        let k2 = self.mino_vector_field(state.add_scaled(k1, 0.5 * step), chart)?;
        let k3 = self.mino_vector_field(state.add_scaled(k2, 0.5 * step), chart)?;
        let k4 = self.mino_vector_field(state.add_scaled(k3, step), chart)?;
        let mut result = state;
        for index in 0..6 {
            result.0[index] +=
                step * (k1.0[index] + 2.0 * k2.0[index] + 2.0 * k3.0[index] + k4.0[index]) / 6.0;
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
            KerrMinoCoordinateChart::BoyerLindquist,
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
            KerrMinoCoordinateChart::IngoingKerr,
        )
    }

    fn integrate_adaptive_in_chart(
        self,
        initial: KerrMinoDynamicState,
        final_parameter: f64,
        config: KerrMinoIntegrationConfig,
        chart: KerrMinoCoordinateChart,
    ) -> Result<KerrMinoTrajectory, KerrMinoIntegrationError> {
        if !final_parameter.is_finite() || final_parameter < 0.0 {
            return Err(KerrMinoIntegrationError::InvalidFinalParameter(
                final_parameter,
            ));
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
            unprojected_constraint_residuals: initial_residuals,
            radial_velocity_projection_correction: 0.0,
            polar_velocity_projection_correction: 0.0,
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
                maximum_unprojected_normalized_radial_constraint_residual: initial_radial_relative,
                maximum_unprojected_normalized_polar_constraint_residual: initial_polar_relative,
                maximum_radial_velocity_projection_correction: 0.0,
                maximum_polar_velocity_projection_correction: 0.0,
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
        let mut max_unprojected_radial_residual = initial_radial_relative;
        let mut max_unprojected_polar_residual = initial_polar_relative;
        let mut max_radial_projection_correction = 0.0_f64;
        let mut max_polar_projection_correction = 0.0_f64;
        let mut last_local_error = 0.0_f64;
        let mut last_constraint_error = 0.0_f64;

        while gamma < final_parameter {
            if attempts >= config.maximum_attempts {
                return Err(KerrMinoIntegrationError::MaximumAttemptsExceeded {
                    mino_parameter: gamma,
                    attempts,
                    next_step: step,
                    normalized_local_error: last_local_error,
                    normalized_constraint_error: last_constraint_error,
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
                    + config.relative_tolerance
                        * vector.0[index].abs().max(two_half.0[index].abs());
                let component_error = (two_half.0[index] - full.0[index]).abs() / (15.0 * scale);
                error_norm = error_norm.max(component_error);
            }

            let unprojected_candidate = two_half.into_state();
            let unprojected_residuals = self.mino_constraint_residuals(unprojected_candidate)?;
            let radial_potential =
                self.radial_potential(unprojected_candidate.coordinates.radius_over_m)?;
            let polar_potential =
                self.polar_potential(unprojected_candidate.coordinates.theta_rad)?;
            let unprojected_radial_relative = normalized_constraint_residual(
                unprojected_candidate.radial_velocity_over_m.powi(2),
                radial_potential,
                unprojected_residuals.radial,
            );
            let unprojected_polar_relative = normalized_constraint_residual(
                unprojected_candidate.polar_velocity_rad.powi(2),
                polar_potential,
                unprojected_residuals.polar,
            );
            if !error_norm.is_finite() {
                return Err(KerrMinoIntegrationError::NonFiniteErrorEstimate);
            }

            // Constraint-stabilized Mino step:
            // the smooth second-order RK4 update controls the local truncation
            // error, while a separately reported velocity projection restores
            // the exact separated first integrals at accepted endpoints.
            // The raw pre-projection residual remains an independent acceptance
            // channel and is never overwritten in the stored receipt.
            let physical_radial_region = radial_potential.is_finite() && radial_potential >= 0.0;
            let physical_polar_region = polar_potential.is_finite() && polar_potential >= 0.0;
            let constraint_error = if physical_radial_region && physical_polar_region {
                (unprojected_radial_relative / config.constraint_tolerance)
                    .max(unprojected_polar_relative / config.constraint_tolerance)
            } else {
                f64::INFINITY
            };
            last_local_error = error_norm;
            last_constraint_error = constraint_error;
            if error_norm <= 1.0 && constraint_error <= 1.0 {
                let radial_acceleration = 0.5
                    * self.radial_potential_derivative(
                        unprojected_candidate.coordinates.radius_over_m,
                    )?;
                let polar_acceleration = 0.5
                    * self.constants.polar_potential_derivative(
                        self.hole.spin().get(),
                        unprojected_candidate.coordinates.theta_rad,
                    )?;
                let radial_sign = if unprojected_candidate.radial_velocity_over_m > 0.0 {
                    1.0
                } else if unprojected_candidate.radial_velocity_over_m < 0.0 {
                    -1.0
                } else {
                    radial_acceleration.signum()
                };
                let polar_sign = if unprojected_candidate.polar_velocity_rad > 0.0 {
                    1.0
                } else if unprojected_candidate.polar_velocity_rad < 0.0 {
                    -1.0
                } else {
                    polar_acceleration.signum()
                };
                let projected_radial_velocity = radial_sign * radial_potential.sqrt();
                let projected_polar_velocity = polar_sign * polar_potential.sqrt();
                let radial_projection_correction =
                    projected_radial_velocity - unprojected_candidate.radial_velocity_over_m;
                let polar_projection_correction =
                    projected_polar_velocity - unprojected_candidate.polar_velocity_rad;
                let candidate = KerrMinoDynamicState::new(
                    unprojected_candidate.coordinates,
                    projected_radial_velocity,
                    projected_polar_velocity,
                );
                let residuals = self.mino_constraint_residuals(candidate)?;
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

                gamma += trial_step;
                vector = MinoVector::from_state(candidate);
                accepted_steps += 1;
                max_radial_residual = max_radial_residual.max(radial_relative);
                max_polar_residual = max_polar_residual.max(polar_relative);
                max_unprojected_radial_residual =
                    max_unprojected_radial_residual.max(unprojected_radial_relative);
                max_unprojected_polar_residual =
                    max_unprojected_polar_residual.max(unprojected_polar_relative);
                max_radial_projection_correction =
                    max_radial_projection_correction.max(radial_projection_correction.abs());
                max_polar_projection_correction =
                    max_polar_projection_correction.max(polar_projection_correction.abs());
                points.push(KerrMinoTrajectoryPoint {
                    mino_parameter: gamma,
                    state: candidate,
                    constraint_residuals: residuals,
                    unprojected_constraint_residuals: unprojected_residuals,
                    radial_velocity_projection_correction: radial_projection_correction,
                    polar_velocity_projection_correction: polar_projection_correction,
                });
                let factor = if error_norm <= 1.0e-14 {
                    5.0
                } else {
                    (0.9 * error_norm.powf(-0.2)).clamp(0.2, 5.0)
                };
                step = (trial_step * factor).clamp(config.minimum_step, config.maximum_step);
            } else {
                // Constraint drift is an independent acceptance channel, not
                // a terminal physics error. Reject the trial and refine it;
                // only report underflow when the configured minimum step
                // cannot satisfy both local truncation and physical constraints.
                rejected_steps += 1;
                let factor = if error_norm > 1.0 {
                    (0.9 * error_norm.powf(-0.2)).clamp(0.1, 0.5)
                } else {
                    (0.8 * constraint_error.powf(-0.25)).clamp(0.1, 0.5)
                };
                let reduced = trial_step * factor;
                if reduced < config.minimum_step {
                    return Err(KerrMinoIntegrationError::StepSizeUnderflow {
                        mino_parameter: gamma,
                        attempted_step: reduced,
                        normalized_error: error_norm.max(constraint_error),
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
            maximum_unprojected_normalized_radial_constraint_residual:
                max_unprojected_radial_residual,
            maximum_unprojected_normalized_polar_constraint_residual:
                max_unprojected_polar_residual,
            maximum_radial_velocity_projection_correction: max_radial_projection_correction,
            maximum_polar_velocity_projection_correction: max_polar_projection_correction,
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
        let a_function = (r * r + spin * spin).powi(2) - spin * spin * delta * sin_squared;
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

    /// Lift local [radial, polar, azimuthal] screen vectors into
    /// contravariant coordinate vectors in the requested observer chart.
    ///
    /// This pointwise chart conversion is not screen parallel transport.
    pub fn screen_basis_to_chart_coordinates(
        self,
        local_screen_basis: [[f64; 3]; 2],
        coordinate_chart: KerrMinoCoordinateChart,
    ) -> [[f64; 4]; 2] {
        let tetrad = self.tetrad;
        let radius = self.radius_over_m;
        let spin = self.hole.spin().get();
        let delta = radius * radius - 2.0 * radius + spin * spin;
        core::array::from_fn(|screen_index| {
            let local = local_screen_basis[screen_index];
            let mut vector = core::array::from_fn(|mu| {
                local[0] * tetrad.radial[mu]
                    + local[1] * tetrad.polar[mu]
                    + local[2] * tetrad.azimuthal[mu]
            });
            if coordinate_chart == KerrMinoCoordinateChart::IngoingKerr {
                let radial_component = vector[1];
                vector[0] += (radius * radius + spin * spin) / delta * radial_component;
                vector[3] += spin / delta * radial_component;
            }
            vector
        })
    }

    /// Construct a canonical Kerr state and two local-sky tangent vectors.
    ///
    /// The screen basis is defined deterministically from the local ZAMO
    /// orthonormal frame [radial, polar, azimuthal]: the azimuthal reference
    /// axis is used except within a cone about that axis, where the polar
    /// reference axis is used. The resulting pair is orthonormal and tangent
    /// to the unit direction. A basis seam exists where the reference axis
    /// switches; callers should preserve the returned basis as part of any
    /// image/branch identity.
    ///
    /// The local photon energy is one. Inward rays are transformed from
    /// Boyer-Lindquist covector components to the regular ingoing Kerr chart.
    pub fn canonical_seed(
        self,
        direction: KerrLocalSkyDirection,
    ) -> Result<KerrObserverCanonicalSeed, KerrGeodesicError> {
        // Reuse the existing observer-domain and null-potential qualification.
        let ray = self.ray_seed(direction)?;
        let r = self.radius_over_m;
        let theta = self.theta_rad;
        let a = self.hole.spin().get();
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();
        let sin_squared = sin_theta * sin_theta;
        let sigma = r * r + a * a * cos_theta * cos_theta;
        let delta = r * r - 2.0 * r + a * a;
        let g_tt = -(1.0 - 2.0 * r / sigma);
        let g_t_phi = -2.0 * a * r * sin_squared / sigma;
        let g_phi_phi = self.tetrad.sqrt_g_phi_phi.powi(2);

        let screen_basis = local_sky_tangent_basis(direction);
        let tetrad = self.tetrad;
        let p_contravariant: [f64; 4] = core::array::from_fn(|mu| {
            tetrad.timelike[mu]
                + direction.radial * tetrad.radial[mu]
                + direction.polar * tetrad.polar[mu]
                + direction.azimuthal * tetrad.azimuthal[mu]
        });
        let p_t = g_tt * p_contravariant[0] + g_t_phi * p_contravariant[3];
        let p_phi = g_t_phi * p_contravariant[0] + g_phi_phi * p_contravariant[3];
        let p_r = (sigma / delta) * p_contravariant[1];
        let p_theta = sigma * p_contravariant[2];
        let momenta_bl = [p_t, p_r, p_theta, p_phi];

        let coordinate_chart = ray.coordinate_chart;
        let transform_covector = |mut momenta: [f64; 4]| -> [f64; 4] {
            if coordinate_chart == KerrMinoCoordinateChart::IngoingKerr {
                // p_r^(in) = p_r^(BL) - p_t (r^2+a^2)/Delta - p_phi a/Delta.
                momenta[1] -= (momenta[0] * (r * r + a * a) + momenta[3] * a) / delta;
            }
            momenta
        };
        let momenta = transform_covector(momenta_bl);
        let coordinates = KerrMinoState::new(0.0, r, theta, 0.0);
        let state = KerrCanonicalState {
            coordinates,
            momenta,
        };

        let mut variations = [[0.0; 8]; 2];
        for screen_index in 0..2 {
            let local_delta = screen_basis[screen_index];
            let delta_contravariant = [
                local_delta[0] * tetrad.radial[0]
                    + local_delta[1] * tetrad.polar[0]
                    + local_delta[2] * tetrad.azimuthal[0],
                local_delta[0] * tetrad.radial[1]
                    + local_delta[1] * tetrad.polar[1]
                    + local_delta[2] * tetrad.azimuthal[1],
                local_delta[0] * tetrad.radial[2]
                    + local_delta[1] * tetrad.polar[2]
                    + local_delta[2] * tetrad.azimuthal[2],
                local_delta[0] * tetrad.radial[3]
                    + local_delta[1] * tetrad.polar[3]
                    + local_delta[2] * tetrad.azimuthal[3],
            ];
            let delta_momenta_bl = [
                g_t_phi * delta_contravariant[3],
                (sigma / delta) * delta_contravariant[1],
                sigma * delta_contravariant[2],
                g_phi_phi * delta_contravariant[3],
            ];
            let delta_momenta = transform_covector(delta_momenta_bl);
            variations[screen_index][4..].copy_from_slice(&delta_momenta);
        }

        let values = [
            ray.infinity_energy_per_unit_local_energy,
            state.momenta[0],
            state.momenta[1],
            state.momenta[2],
            state.momenta[3],
            variations[0][4],
            variations[0][5],
            variations[0][6],
            variations[0][7],
            variations[1][4],
            variations[1][5],
            variations[1][6],
            variations[1][7],
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(KerrGeodesicError::NonFinite(
                "finite_observer_canonical_seed",
            ));
        }
        Ok(KerrObserverCanonicalSeed {
            state,
            coordinate_chart,
            local_screen_basis: screen_basis,
            first_phase_space_variation: variations[0],
            second_phase_space_variation: variations[1],
            infinity_energy_per_unit_local_energy: ray.infinity_energy_per_unit_local_energy,
        })
    }

    /// Project a canonical phase-space variation into this observer's
    /// two-dimensional local screen.
    ///
    /// The supplied canonical state must refer to this observer event (r,theta).
    /// In the ingoing chart, both the covector and coordinate variation are
    /// transformed back to Boyer-Lindquist components before projection.
    /// The result is the observer-rest-space separation with its component
    /// parallel to the local photon direction removed.
    pub fn project_canonical_variation_to_screen(
        self,
        state: KerrCanonicalState,
        variation: [f64; 8],
        coordinate_chart: KerrMinoCoordinateChart,
        tolerance: f64,
    ) -> Result<KerrObserverScreenProjection, KerrGeodesicError> {
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err(KerrGeodesicError::InvalidProjectionTolerance(tolerance));
        }
        let radius_difference = (state.coordinates.radius_over_m - self.radius_over_m).abs();
        let polar_difference = (state.coordinates.theta_rad - self.theta_rad).abs();
        if radius_difference > tolerance * (1.0 + self.radius_over_m.abs())
            || polar_difference > tolerance
        {
            return Err(KerrGeodesicError::ObserverEventMismatch {
                radius_difference,
                polar_difference,
                tolerance,
            });
        }
        if variation.iter().any(|value| !value.is_finite()) {
            return Err(KerrGeodesicError::NonFinite("source_screen_variation"));
        }

        let r = self.radius_over_m;
        let theta = self.theta_rad;
        let a = self.hole.spin().get();
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();
        let sin_squared = sin_theta * sin_theta;
        let sigma = r * r + a * a * cos_theta * cos_theta;
        let delta = r * r - 2.0 * r + a * a;
        if !delta.is_finite() || delta <= 0.0 {
            return Err(KerrGeodesicError::BoyerLindquistTurningSingularity { delta });
        }
        let g_t_phi = -2.0 * a * r * sin_squared / sigma;
        let g_phi_phi = self.tetrad.sqrt_g_phi_phi.powi(2);

        let mut p = state.momenta;
        let mut dq = [variation[0], variation[1], variation[2], variation[3]];
        if coordinate_chart == KerrMinoCoordinateChart::IngoingKerr {
            // Inverse covector transform of p_r^(in) = p_r^(BL) - ...
            p[1] += (p[0] * (r * r + a * a) + p[3] * a) / delta;
            // t = v - r_*(r), phi = phi_tilde - int(a/Delta)dr.
            dq[0] -= (r * r + a * a) / delta * dq[1];
            dq[3] -= a / delta * dq[1];
        }
        if p.iter().any(|value| !value.is_finite()) || dq.iter().any(|value| !value.is_finite()) {
            return Err(KerrGeodesicError::NonFinite(
                "source_screen_chart_transform",
            ));
        }

        let energy_local =
            -(p[0] + self.tetrad.frame_dragging_angular_velocity * p[3]) / self.tetrad.lapse;
        if !energy_local.is_finite() || energy_local <= 0.0 {
            return Err(KerrGeodesicError::NegativeKillingEnergy(energy_local));
        }
        let direction_raw = [
            (delta / sigma).sqrt() * p[1] / energy_local,
            p[2] / sigma.sqrt() / energy_local,
            p[3] / self.tetrad.sqrt_g_phi_phi / energy_local,
        ];
        let direction_norm =
            (direction_raw[0].powi(2) + direction_raw[1].powi(2) + direction_raw[2].powi(2)).sqrt();
        let direction_norm_residual = (direction_norm - 1.0).abs();
        if !direction_norm.is_finite()
            || direction_norm <= 0.0
            || direction_norm_residual > tolerance
        {
            return Err(KerrGeodesicError::LocalNullResidual {
                residual: direction_norm_residual,
                tolerance,
            });
        }
        let direction = [
            direction_raw[0] / direction_norm,
            direction_raw[1] / direction_norm,
            direction_raw[2] / direction_norm,
        ];
        let basis = local_sky_tangent_basis(KerrLocalSkyDirection {
            radial: direction[0],
            polar: direction[1],
            azimuthal: direction[2],
        });

        // Convert the coordinate displacement into the source ZAMO's
        // orthonormal spatial components by contracting with the tetrad.
        let spatial = [
            (sigma / delta).sqrt() * dq[1],
            sigma.sqrt() * dq[2],
            (g_t_phi * dq[0] + g_phi_phi * dq[3]) / self.tetrad.sqrt_g_phi_phi,
        ];
        let components = [
            basis[0][0] * spatial[0] + basis[0][1] * spatial[1] + basis[0][2] * spatial[2],
            basis[1][0] * spatial[0] + basis[1][1] * spatial[1] + basis[1][2] * spatial[2],
        ];
        if components.iter().any(|value| !value.is_finite()) {
            return Err(KerrGeodesicError::NonFinite("source_screen_projection"));
        }
        Ok(KerrObserverScreenProjection {
            components,
            local_screen_basis: basis,
            direction_norm_residual,
        })
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
            + cos_theta.powi(2) * (angular_momentum.powi(2) / sin_squared - a * a);
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
            return Err(KerrGeodesicError::NonFinite(
                "finite_observer_screen_normalization",
            ));
        }
        Ok(KerrObserverRaySeed {
            constants,
            coordinate_chart: if direction.radial < 0.0 {
                KerrMinoCoordinateChart::IngoingKerr
            } else {
                KerrMinoCoordinateChart::BoyerLindquist
            },
            observer_radius_over_m: self.radius_over_m,
            observer_theta_rad: self.theta_rad,
            initial_state: state,
            infinity_energy_per_unit_local_energy: energy_at_infinity,
            local_direction: direction,
        })
    }
}

fn local_sky_cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn local_sky_normalize(vector: [f64; 3]) -> Option<[f64; 3]> {
    let norm = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if !norm.is_finite() || norm <= 64.0 * f64::EPSILON {
        None
    } else {
        Some([vector[0] / norm, vector[1] / norm, vector[2] / norm])
    }
}

/// Deterministic orthonormal tangent frame on the observer's local unit sky.
fn local_sky_tangent_basis(direction: KerrLocalSkyDirection) -> [[f64; 3]; 2] {
    let n = [direction.radial, direction.polar, direction.azimuthal];
    let reference = if direction.azimuthal.abs() < 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let first = local_sky_normalize(local_sky_cross(reference, n))
        .expect("reference axis is selected away from parallel sky direction");
    let second = local_sky_normalize(local_sky_cross(n, first))
        .expect("unit sky direction and first tangent define a nondegenerate screen");
    [first, second]
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
    if !inclination.is_finite() || inclination <= 0.0 || inclination >= core::f64::consts::PI {
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
    use crate::light_ring::{KerrEquatorialPhotonOrbit, OrbitSense};
    use crate::{DimensionlessSpin, MassKg};

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

    fn canonical_field(evaluation: KerrCanonicalEvaluation) -> [f64; 8] {
        [
            evaluation.derivative.coordinates[0],
            evaluation.derivative.coordinates[1],
            evaluation.derivative.coordinates[2],
            evaluation.derivative.coordinates[3],
            evaluation.derivative.momenta[0],
            evaluation.derivative.momenta[1],
            evaluation.derivative.momenta[2],
            evaluation.derivative.momenta[3],
        ]
    }

    fn perturb_canonical_state(
        state: KerrCanonicalState,
        component: usize,
        amount: f64,
    ) -> KerrCanonicalState {
        let mut q = state.coordinates;
        let mut p = state.momenta;
        match component {
            0 => q.time_over_m += amount,
            1 => q.radius_over_m += amount,
            2 => q.theta_rad += amount,
            3 => q.azimuth_rad += amount,
            4..=7 => p[component - 4] += amount,
            _ => unreachable!("canonical state has eight components"),
        }
        KerrCanonicalState {
            coordinates: KerrMinoState::new(
                q.time_over_m,
                q.radius_over_m,
                q.theta_rad,
                q.azimuth_rad,
            ),
            momenta: p,
        }
    }

    #[test]
    fn sachs_parallel_transport_rhs_preserves_screen_constraints_in_boyer_lindquist_chart() {
        let black_hole = hole(0.0);
        let observer =
            KerrFiniteRadiusObserver::new(black_hole, 10.0, core::f64::consts::FRAC_PI_2).unwrap();
        let direction = KerrLocalSkyDirection::new(0.8, 0.6, 0.0).unwrap();
        let seed = observer.canonical_seed(direction).unwrap();
        let screen_basis = observer.screen_basis_to_chart_coordinates(
            seed.local_screen_basis,
            KerrMinoCoordinateChart::BoyerLindquist,
        );
        let result = KerrCanonicalHamiltonian::new(black_hole)
            .sachs_frame_transport_derivative(
                seed.state,
                KerrMinoCoordinateChart::BoyerLindquist,
                screen_basis,
                1e-8,
            )
            .unwrap();
        assert!(result.null_constraint_residual < 1e-10);
        assert!(result.screen_orthonormality_residual < 1e-10);
        assert!(result.screen_transversality_residual < 1e-10);
        assert!(result.orthonormality_rate_residual < 1e-9);
        assert!(result.transversality_rate_residual < 1e-9);
        assert!(result.metric_compatibility_residual < 1e-10);
    }

    #[test]
    fn sachs_parallel_transport_rhs_preserves_screen_constraints_in_ingoing_chart() {
        let black_hole = hole(0.7);
        let observer =
            KerrFiniteRadiusObserver::new(black_hole, 10.0, core::f64::consts::FRAC_PI_2).unwrap();
        let direction = KerrLocalSkyDirection::new(-1.0, 0.0, 0.0).unwrap();
        let seed = observer.canonical_seed(direction).unwrap();
        assert_eq!(seed.coordinate_chart, KerrMinoCoordinateChart::IngoingKerr);
        let screen_basis = observer
            .screen_basis_to_chart_coordinates(seed.local_screen_basis, seed.coordinate_chart);
        let result = KerrCanonicalHamiltonian::new(black_hole)
            .sachs_frame_transport_derivative(seed.state, seed.coordinate_chart, screen_basis, 1e-8)
            .unwrap();
        assert!(result.null_constraint_residual < 1e-9);
        assert!(result.screen_orthonormality_residual < 1e-9);
        assert!(result.screen_transversality_residual < 1e-9);
        assert!(result.orthonormality_rate_residual < 1e-8);
        assert!(result.transversality_rate_residual < 1e-8);
        assert!(
            result.metric_compatibility_residual < 1e-9,
            "metric compatibility residual={:.16e}",
            result.metric_compatibility_residual
        );
    }

    #[test]
    fn radial_schwarzschild_sachs_optical_tidal_matrix_is_zero() {
        let black_hole = hole(0.0);
        let observer =
            KerrFiniteRadiusObserver::new(black_hole, 10.0, core::f64::consts::FRAC_PI_2).unwrap();
        let direction = KerrLocalSkyDirection::new(-1.0, 0.0, 0.0).unwrap();
        let seed = observer.canonical_seed(direction).unwrap();
        let screen_basis = observer
            .screen_basis_to_chart_coordinates(seed.local_screen_basis, seed.coordinate_chart);
        let result = KerrCanonicalHamiltonian::new(black_hole)
            .optical_tidal_matrix(seed.state, seed.coordinate_chart, screen_basis, 1e-9)
            .unwrap();
        assert!(result.null_constraint_residual < 1e-10);
        assert!(result.screen_orthonormality_residual < 1e-10);
        assert!(result.screen_transversality_residual < 1e-10);
        assert!(result.tidal_symmetry_residual < 1e-10);
        assert!(result.vacuum_trace_residual < 1e-10);
        for value in result.tidal_matrix.iter().flatten() {
            assert!(value.abs() < 1e-9, "radial Schwarzschild tidal={value:e}");
        }
    }

    #[test]
    fn nonradial_schwarzschild_sachs_matrix_is_symmetric_and_vacuum_trace_free() {
        let black_hole = hole(0.0);
        let observer =
            KerrFiniteRadiusObserver::new(black_hole, 10.0, core::f64::consts::FRAC_PI_2).unwrap();
        let direction = KerrLocalSkyDirection::new(-0.8, 0.6, 0.0).unwrap();
        let seed = observer.canonical_seed(direction).unwrap();
        let screen_basis = observer
            .screen_basis_to_chart_coordinates(seed.local_screen_basis, seed.coordinate_chart);
        let result = KerrCanonicalHamiltonian::new(black_hole)
            .optical_tidal_matrix(seed.state, seed.coordinate_chart, screen_basis, 1e-8)
            .unwrap();
        assert!(result.tidal_symmetry_residual < 1e-8);
        assert!(result.vacuum_trace_residual < 1e-8);
        assert!(
            result
                .tidal_matrix
                .iter()
                .flatten()
                .any(|value| value.abs() > 1e-10),
            "nonradial Schwarzschild ray should have nonzero Weyl optical tide"
        );
    }

    #[test]
    fn sachs_optical_tidal_rejects_nontransverse_screen_frame() {
        let black_hole = hole(0.0);
        let observer =
            KerrFiniteRadiusObserver::new(black_hole, 10.0, core::f64::consts::FRAC_PI_2).unwrap();
        let direction = KerrLocalSkyDirection::new(-1.0, 0.0, 0.0).unwrap();
        let seed = observer.canonical_seed(direction).unwrap();
        let mut screen_basis = observer
            .screen_basis_to_chart_coordinates(seed.local_screen_basis, seed.coordinate_chart);
        screen_basis[0] = observer.tetrad().timelike;
        assert!(matches!(
            KerrCanonicalHamiltonian::new(black_hole).optical_tidal_matrix(
                seed.state,
                seed.coordinate_chart,
                screen_basis,
                1e-9,
            ),
            Err(KerrGeodesicError::OpticalFrameConstraintViolation { .. })
        ));
    }

    #[test]
    fn canonical_flow_linearization_matches_independent_central_differences() {
        let black_hole = hole(0.7);
        let canonical = KerrCanonicalHamiltonian::new(black_hole);
        let constants = KerrNullConstants::new(1.0, 2.0, 3.0).unwrap();
        let coordinates = KerrMinoState::new(0.0, 10.0, 1.0, 0.0);
        let state = canonical
            .state_from_separated(
                coordinates,
                constants,
                KerrMinoDirection::Decreasing,
                KerrMinoDirection::Increasing,
            )
            .unwrap();
        let linearization = canonical.linearize(state).unwrap();
        assert!(linearization.symplectic_lie_residual < 2e-10);

        // This finite-difference bundle is only an independent test oracle;
        // production linearization uses the second-order AD metric derivatives.
        for column in 0..8 {
            let step = 2e-5
                * match column {
                    0 | 3 => 1.0,
                    1 => state.coordinates.radius_over_m.abs().max(1.0),
                    2 => 1.0,
                    4..=7 => state.momenta[column - 4].abs().max(1.0),
                    _ => unreachable!(),
                };
            let plus = perturb_canonical_state(state, column, step);
            let minus = perturb_canonical_state(state, column, -step);
            let plus_field = canonical_field(canonical.evaluate(plus).unwrap());
            let minus_field = canonical_field(canonical.evaluate(minus).unwrap());
            for row in 0..8 {
                let finite_difference = (plus_field[row] - minus_field[row]) / (2.0 * step);
                close(linearization.jacobian[row][column], finite_difference, 2e-6);
            }
        }
    }

    #[test]
    fn ingoing_chart_flow_linearization_preserves_hamiltonian_structure() {
        let black_hole = hole(0.9);
        let canonical = KerrCanonicalHamiltonian::new(black_hole);
        let constants = KerrNullConstants::new(1.0, 2.0, 3.0).unwrap();
        let coordinates = KerrMinoState::new(0.0, 3.0, 1.0, 0.2);
        let state = canonical
            .ingoing_state_from_separated(coordinates, constants, KerrMinoDirection::Increasing)
            .unwrap();
        let linearization = canonical.linearize_ingoing(state).unwrap();
        assert!(linearization.symplectic_lie_residual < 2e-9);
        close(linearization.evaluation.hamiltonian, 0.0, 2e-9);
    }

    #[test]
    fn asymptotic_screen_map_round_trips_in_equatorial_and_inclined_views() {
        for inclination in [core::f64::consts::FRAC_PI_2, core::f64::consts::FRAC_PI_3] {
            for (alpha, beta) in [(-8.0, 1.5), (0.0, 2.0), (5.0, -0.75)] {
                let screen = KerrObserverScreen::new(alpha, beta, inclination).unwrap();
                let constants =
                    KerrNullConstants::from_asymptotic_screen(hole(0.7), screen).unwrap();
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
                let constants = KerrNullConstants::new(1.0, impact_parameter_m / rg, 0.0).unwrap();
                close(
                    constants
                        .radial_potential(spin, orbit.radius_m / rg)
                        .unwrap(),
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
    fn exact_equatorial_polar_invariant_is_not_corrupted_by_cos_pi_over_two_roundoff() {
        for (spin, angular_momentum) in [(0.0, 6.0), (0.9, 2.0), (-0.8, 1.5)] {
            let constants = KerrNullConstants::new(1.0, angular_momentum, 0.0).unwrap();
            let theta = core::f64::consts::FRAC_PI_2;
            assert_eq!(constants.polar_potential(spin, theta).unwrap(), 0.0);
            assert_eq!(
                constants.polar_potential_derivative(spin, theta).unwrap(),
                0.0
            );
            let geodesic = KerrSeparatedNullGeodesic::new(hole(spin), constants);
            let state = KerrMinoDynamicState::new(
                KerrMinoState::new(0.0, 10.0, theta, 0.0),
                -geodesic.radial_potential(10.0).unwrap().sqrt(),
                0.0,
            );
            let derivative = geodesic.mino_dynamic_derivative(state).unwrap();
            assert_eq!(derivative.d_polar_velocity_rad, 0.0);
            assert_eq!(derivative.d_theta_rad, 0.0);
        }
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
        let horizon_state =
            KerrMinoState::new(0.0, horizon_over_m, core::f64::consts::FRAC_PI_2, 0.0);
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
        let carter = angular_momentum * angular_momentum * theta_turn.cos().powi(2)
            / theta_turn.sin().powi(2)
            - spin * spin * theta_turn.cos().powi(2);
        let geodesic = KerrSeparatedNullGeodesic::new(
            hole(spin),
            KerrNullConstants::new(1.0, angular_momentum, carter).unwrap(),
        );
        let state =
            KerrMinoDynamicState::new(KerrMinoState::new(0.0, 10.0, theta_turn, 0.0), 0.0, 0.0);
        let residuals = geodesic.mino_constraint_residuals(state).unwrap();
        close(residuals.polar, 0.0, 2e-14);
        let derivative = geodesic.mino_dynamic_derivative(state).unwrap();
        assert!(derivative.d_polar_velocity_rad > 0.0);
        assert_eq!(derivative.d_theta_rad, 0.0);
    }

    #[test]
    fn second_order_mino_system_recovers_schwarzschild_circular_photon_orbit() {
        let constants = KerrNullConstants::new(1.0, 3.0 * 3.0_f64.sqrt(), 0.0).unwrap();
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
            let coordinates = KerrMinoState::new(0.0, 10.0, 1.0, 0.0);
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
            close(
                hamiltonian.derivative.coordinates[0] * sigma,
                mino.d_time_over_m,
                2e-11,
            );
            close(
                hamiltonian.derivative.coordinates[1] * sigma,
                mino.d_radius_over_m,
                2e-11,
            );
            close(
                hamiltonian.derivative.coordinates[2] * sigma,
                mino.d_theta_rad,
                2e-11,
            );
            close(
                hamiltonian.derivative.coordinates[3] * sigma,
                mino.d_azimuth_rad,
                2e-11,
            );
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
                .ingoing_state_from_separated(coordinates, constants, KerrMinoDirection::Increasing)
                .unwrap();
            let bl = canonical.evaluate(bl_state).unwrap();
            let ingoing = canonical.evaluate_ingoing(ingoing_state).unwrap();
            let r = coordinates.radius_over_m;
            let theta = coordinates.theta_rad;
            let delta = r * r - 2.0 * r + spin * spin;
            let expected_v = bl.derivative.coordinates[0]
                + (r * r + spin * spin) / delta * bl.derivative.coordinates[1];
            let expected_phi =
                bl.derivative.coordinates[3] + spin / delta * bl.derivative.coordinates[1];
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
                .ingoing_state_from_separated(coordinates, constants, KerrMinoDirection::Increasing)
                .unwrap();
            let evaluation = canonical.evaluate_ingoing(state).unwrap();
            assert!(evaluation.hamiltonian.is_finite());
            assert!(
                evaluation
                    .derivative
                    .coordinates
                    .iter()
                    .all(|v| v.is_finite())
            );
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
        let coordinates = KerrMinoState::new(0.0, 2.5, core::f64::consts::FRAC_PI_2, 0.0);
        let constants = KerrNullConstants::new(1.0, 10.0, 0.0).unwrap();
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
        // The turning-point residual is evaluated from two nearly cancelling
        // radial terms. Keep the declared 2e-7 envelope explicit rather than
        // requiring the adaptive controller to chase sub-ulp residual changes.
        let config = KerrMinoIntegrationConfig::new(
            1.0e-9,
            [1.0e-11; 6],
            2.0e-7,
            1.0e-3,
            1.0e-14,
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
        let final_radius = trajectory
            .points
            .last()
            .unwrap()
            .state
            .coordinates
            .radius_over_m;
        close(minimum_radius, 4.453_363_193_811_354, 2.0e-4);
        assert!(final_radius > radius);
        assert!(trajectory.accepted_steps > 0);
        assert!(trajectory.rejected_steps > 0);
        assert!(trajectory.maximum_normalized_radial_constraint_residual <= 2.0e-7);
        assert!(trajectory.maximum_normalized_polar_constraint_residual <= 2.0e-7);
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
                [
                    -(1.0 - 2.0 * r / sigma),
                    0.0,
                    0.0,
                    -2.0 * a * r * sin2 / sigma,
                ],
                [0.0, sigma / delta, 0.0, 0.0],
                [0.0, 0.0, sigma, 0.0],
                [
                    -2.0 * a * r * sin2 / sigma,
                    0.0,
                    0.0,
                    tetrad.sqrt_g_phi_phi.powi(2),
                ],
            ];
            let vectors = [
                tetrad.timelike,
                tetrad.radial,
                tetrad.polar,
                tetrad.azimuthal,
            ];
            for i in 0..4 {
                for j in 0..4 {
                    let mut product = 0.0;
                    for mu in 0..4 {
                        for nu in 0..4 {
                            product += g[mu][nu] * vectors[i][mu] * vectors[j][nu];
                        }
                    }
                    let expected = if i == j {
                        if i == 0 { -1.0 } else { 1.0 }
                    } else {
                        0.0
                    };
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
            let residual = geodesic
                .mino_constraint_residuals(seed.initial_state)
                .unwrap();
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
        let observer =
            KerrFiniteRadiusObserver::new(hole(spin), 100_000.0, core::f64::consts::FRAC_PI_2)
                .unwrap();
        let radial = -(1.0_f64 - 0.006_f64.powi(2) - 0.008_f64.powi(2)).sqrt();
        let direction = KerrLocalSkyDirection::new(radial, 0.006, -0.008).unwrap();
        let seed = observer.ray_seed(direction).unwrap();
        let screen = seed
            .constants
            .to_asymptotic_screen(
                hole(spin),
                core::f64::consts::FRAC_PI_2,
                KerrMinoDirection::Increasing,
            )
            .unwrap();
        // The finite-radius impact coordinates tend to (-r*n_phi, r*n_theta)
        // in the asymptotic limit; errors vanish as 1/r.
        close(screen.alpha_over_m / 100_000.0, 0.008, 2.0e-4);
        close(screen.beta_over_m / 100_000.0, 0.006, 2.0e-4);
    }

    fn normalize_sky(vector: [f64; 3]) -> [f64; 3] {
        let norm = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
        [vector[0] / norm, vector[1] / norm, vector[2] / norm]
    }

    #[test]
    fn finite_observer_canonical_screen_seed_is_null_and_tangent_to_null_shell() {
        for (spin, direction) in [
            (
                0.7,
                KerrLocalSkyDirection::new(
                    -(1.0_f64 - 0.006_f64.powi(2) - 0.008_f64.powi(2)).sqrt(),
                    0.006,
                    -0.008,
                )
                .unwrap(),
            ),
            (0.9, KerrLocalSkyDirection::new(-0.8, 0.6, 0.0).unwrap()),
        ] {
            let observer = KerrFiniteRadiusObserver::new(hole(spin), 10.0, 1.0).unwrap();
            let seed = observer.canonical_seed(direction).unwrap();
            assert_eq!(seed.coordinate_chart, KerrMinoCoordinateChart::IngoingKerr);
            let evaluation = KerrCanonicalHamiltonian::new(hole(spin))
                .evaluate_ingoing(seed.state)
                .unwrap();
            close(evaluation.hamiltonian, 0.0, 2e-11);

            let n = [direction.radial, direction.polar, direction.azimuthal];
            let basis = seed.local_screen_basis;
            for basis_vector in &basis {
                close(
                    basis_vector.iter().zip(n).map(|(a, b)| a * b).sum(),
                    0.0,
                    2e-14,
                );
                close(basis_vector.iter().map(|x| x * x).sum(), 1.0, 2e-14);
            }
            close(
                basis[0].iter().zip(basis[1]).map(|(a, b)| a * b).sum(),
                0.0,
                2e-14,
            );

            for variation in [
                seed.first_phase_space_variation,
                seed.second_phase_space_variation,
            ] {
                let shell_tangent = evaluation
                    .derivative
                    .coordinates
                    .iter()
                    .zip(&variation[4..])
                    .map(|(qdot, dp)| qdot * dp)
                    .sum::<f64>();
                assert!(
                    shell_tangent.abs() < 2e-11,
                    "null-shell tangent residual={shell_tangent:.4e}"
                );
            }

            // Central finite difference of the exact local-tetrad seed map.
            let epsilon = 1e-6;
            let analytic_variations = [
                seed.first_phase_space_variation,
                seed.second_phase_space_variation,
            ];
            for (tangent, analytic) in basis.iter().zip(analytic_variations.iter()) {
                let plus = normalize_sky([
                    n[0] + epsilon * tangent[0],
                    n[1] + epsilon * tangent[1],
                    n[2] + epsilon * tangent[2],
                ]);
                let minus = normalize_sky([
                    n[0] - epsilon * tangent[0],
                    n[1] - epsilon * tangent[1],
                    n[2] - epsilon * tangent[2],
                ]);
                let plus_seed = observer
                    .canonical_seed(KerrLocalSkyDirection::new(plus[0], plus[1], plus[2]).unwrap())
                    .unwrap();
                let minus_seed = observer
                    .canonical_seed(
                        KerrLocalSkyDirection::new(minus[0], minus[1], minus[2]).unwrap(),
                    )
                    .unwrap();
                for ((plus_momentum, minus_momentum), analytic_component) in plus_seed
                    .state
                    .momenta
                    .iter()
                    .zip(minus_seed.state.momenta.iter())
                    .zip(analytic.iter().skip(4))
                {
                    let numerical = (plus_momentum - minus_momentum) / (2.0 * epsilon);
                    close(*analytic_component, numerical, 2e-7);
                }
            }
        }
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
                bl.d_time_over_m
                    + (radius * radius + spin * spin) / delta * state.radial_velocity_over_m,
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
            assert!(
                [
                    derivative.d_time_over_m,
                    derivative.d_radius_over_m,
                    derivative.d_radial_velocity_over_m,
                    derivative.d_theta_rad,
                    derivative.d_polar_velocity_rad,
                    derivative.d_azimuth_rad,
                ]
                .iter()
                .all(|value| value.is_finite())
            );
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
    fn capture_classifier_requires_ingoing_chart_and_actual_horizon_crossing() {
        let spin = 0.9;
        let black_hole = hole(spin);
        let constants = KerrNullConstants::new(1.0, 2.0, 0.0).unwrap();
        let geodesic = KerrSeparatedNullGeodesic::new(black_hole, constants);
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
        let captured = geodesic
            .integrate_adaptive_ingoing(initial, 1.460_728_111_791_428, config)
            .unwrap();
        assert_eq!(
            captured.capture_status(black_hole, 1.0e-5),
            KerrMinoCaptureStatus::CapturedByHorizon
        );
        // The equatorial invariant-plane and compensated radial-potential
        // fixes now keep this same trajectory below the tighter 1e-8 ceiling.
        assert_eq!(
            captured.capture_status(black_hole, 1.0e-8),
            KerrMinoCaptureStatus::CapturedByHorizon
        );
        // Keep the fail-closed classifier control independent of the current
        // solver's achieved accuracy: inject an explicitly unqualified receipt.
        let mut deliberately_unqualified = captured.clone();
        deliberately_unqualified.maximum_unprojected_normalized_radial_constraint_residual = 1.0e-4;
        assert_eq!(
            deliberately_unqualified.capture_status(black_hole, 1.0e-5),
            KerrMinoCaptureStatus::ConstraintDriftExceeded
        );
        let incomplete_interval = geodesic
            .integrate_adaptive_ingoing(initial, 0.1, config)
            .unwrap();
        assert_eq!(
            incomplete_interval.capture_status(black_hole, 1.0e-5),
            KerrMinoCaptureStatus::NoCrossingWithinIntegratedInterval
        );
        let boyer_lindquist = geodesic.integrate_adaptive(initial, 0.1, config).unwrap();
        assert_eq!(
            boyer_lindquist.capture_status(black_hole, 1.0e-5),
            KerrMinoCaptureStatus::NotQualifiableInBoyerLindquist
        );
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

#[cfg(test)]
mod near_extremal_prograde_photon_orbit_tests {
    use super::*;

    fn close(actual: f64, expected: f64, absolute_tolerance: f64) {
        assert!(
            (actual - expected).abs() <= absolute_tolerance,
            "actual={actual:.15e}, expected={expected:.15e}, tolerance={absolute_tolerance:.3e}"
        );
    }

    #[test]
    fn prograde_photon_orbit_matches_schwarzschild_and_spin_point_nine_references() {
        let schwarzschild = KerrEquatorialProgradePhotonOrbit::from_spin(0.0).unwrap();
        close(schwarzschild.radius_over_m, 3.0, 2e-14);
        close(
            schwarzschild.critical_impact_parameter_over_m,
            3.0 * 3.0_f64.sqrt(),
            2e-13,
        );
        let spin_nine = KerrEquatorialProgradePhotonOrbit::from_spin(0.9).unwrap();
        close(spin_nine.radius_over_m, 1.557_854_627_423_382_9, 2e-14);
        close(
            spin_nine.critical_impact_parameter_over_m,
            2.844_421_403_476_169,
            2e-13,
        );
    }

    #[test]
    fn prograde_photon_orbit_has_the_correct_extremal_limit() {
        let extremal = KerrEquatorialProgradePhotonOrbit::from_spin(1.0).unwrap();
        assert_eq!(extremal.extremality_parameter, 0.0);
        assert_eq!(extremal.radius_over_m, 1.0);
        assert_eq!(extremal.critical_impact_parameter_over_m, 2.0);
    }

    #[test]
    fn near_extremal_critical_impact_parameter_has_sqrt_three_scaling() {
        let mut previous_error = f64::INFINITY;
        for kappa in [1e-1_f64, 3e-2, 1e-2, 3e-3, 1e-3] {
            let spin = (1.0 - kappa * kappa).sqrt();
            let orbit = KerrEquatorialProgradePhotonOrbit::from_spin(spin).unwrap();
            let ratio =
                (orbit.critical_impact_parameter_over_m - 2.0) / orbit.extremality_parameter;
            let error = (ratio - 3.0_f64.sqrt()).abs();
            assert!(error < previous_error);
            previous_error = error;
        }
        assert!(previous_error < 4e-4);
    }

    #[test]
    fn near_extremal_crossover_offset_constructs_equatorial_null_constants() {
        let kappa: f64 = 1e-3;
        let orbit =
            KerrEquatorialProgradePhotonOrbit::from_spin((1.0 - kappa * kappa).sqrt()).unwrap();
        let constants = orbit.null_constants_for_scaled_offset(0.5).unwrap();
        close(constants.energy, 1.0, 0.0);
        close(
            constants.angular_momentum_z_over_m,
            orbit.critical_impact_parameter_over_m + 0.5 * orbit.extremality_parameter,
            1e-14,
        );
        close(constants.carter_constant_over_m2, 0.0, 0.0);
        let phase = orbit.leading_uniform_deflection_phase(0.5).unwrap();
        close(
            phase * orbit.extremality_parameter,
            (1.0 + 2.0 * 3.0_f64.sqrt() / 0.5).ln(),
            1e-13,
        );
    }

    #[test]
    fn near_extremal_prograde_helper_rejects_wrong_branch_and_nonpositive_offsets() {
        assert!(matches!(
            KerrEquatorialProgradePhotonOrbit::from_spin(-0.5),
            Err(KerrGeodesicError::InvalidProgradeSpin(-0.5))
        ));
        let orbit = KerrEquatorialProgradePhotonOrbit::from_spin(0.9).unwrap();
        for chi in [0.0, -0.5, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                orbit.impact_parameter_for_scaled_offset(chi),
                Err(KerrGeodesicError::InvalidCriticalOffset(_))
            ));
        }
        let extremal = KerrEquatorialProgradePhotonOrbit::from_spin(1.0).unwrap();
        assert!(matches!(
            extremal.leading_uniform_deflection_phase(1.0),
            Err(KerrGeodesicError::InvalidCriticalOffset(1.0))
        ));
    }
}
