// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Convention-explicit analytic and numerical reference calculations for
//! stationary black-hole spacetimes.
//!
//! The crate currently provides validated SI-valued black-hole parameters,
//! Schwarzschild/Kerr metric charts, differential geometry and curvature,
//! Schwarzschild timelike/null geodesics, analytic Kerr light-ring observables,
//! and a separated Kerr null-geodesic first-integral kernel.
//!
//! The Kerr kernel evaluates Carter-separated radial/polar potentials,
//! asymptotic Bardeen screen constants and Mino-time derivatives. It is not
//! yet a complete Kerr ray stepper: turning-point event handling, independent
//! Hamiltonian evolution, Jacobi fields and caustic maps remain separate work.
//! It does not implement waveform generation or numerical-relativity evolution.
//!
//! Conventions:
//! - SI units are used at the public API boundary.
//! - The dimensionless spin is chi = c J / (G M^2), signed relative to a
//!   declared axis and constrained to [-1, 1].
//! - Kerr's geometrized rotation parameter is a = chi G M / c^2.
//! - Radius arguments are Boyer-Lindquist radial coordinates for Kerr
//!   formulae and Schwarzschild areal radius for Schwarzschild formulae.
//! - The Kerr polar angle theta is in radians in [0, pi].
//!
//! The returned Kretschmann scalar has units m^-4. In geometrized notation,
//! K = 48 M^2 / r^6 for Schwarzschild; converting M to the length GM/c^2
//! gives the SI expression implemented here.

use core::fmt;

/// Fixed-size second-order automatic differentiation for metric derivatives.
pub mod autodiff;
/// Levi-Civita connection coefficients from explicit metric derivatives.
pub mod connection;
/// Convention-explicit Riemann, Ricci, and Einstein curvature tensors.
pub mod curvature;
/// Schwarzschild timelike/null geodesics with Christoffel and Hamiltonian formulations.
pub mod geodesic;
/// Separated Kerr null-geodesic potentials, asymptotic screen mapping and Mino derivatives.
pub mod kerr_geodesic;
/// Adaptive canonical Hamiltonian integration of Kerr null trajectories.
pub mod kerr_integrator;
/// Kerr equatorial circular-null-orbit and light-ring observables.
pub mod light_ring;
/// Validated stationary black-hole metric components in a declared chart.
pub mod metric;
/// Posterior-preserving transforms for Kerr remnant horizon observables.
pub mod posterior;
/// Noise-weighted frequency-domain waveform comparison and error diagnostics.
pub mod waveform_metrics;

/// Newtonian gravitational constant in SI units (m^3 kg^-1 s^-2).
pub const GRAVITATIONAL_CONSTANT_M3_KG_S2: f64 = 6.674_30e-11;
/// Speed of light in vacuum in SI units (m s^-1), exact by definition.
pub const SPEED_OF_LIGHT_M_S: f64 = 299_792_458.0;

/// A positive, finite mass expressed in kilograms.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MassKg(f64);

impl MassKg {
    pub fn new(value: f64) -> Result<Self, PhysicsDomainError> {
        if !value.is_finite() {
            return Err(PhysicsDomainError::NonFinite("mass_kg"));
        }
        if value <= 0.0 {
            return Err(PhysicsDomainError::NonPositive("mass_kg"));
        }
        // Keep the derived Kerr/Schwarzschild length scale representable.
        // The factor 5 bounds r_+^2 + a^2 for every |chi| <= 1.
        let rg = GRAVITATIONAL_CONSTANT_M3_KG_S2 * value / SPEED_OF_LIGHT_M_S.powi(2);
        let rg_squared = rg * rg;
        if !rg.is_finite()
            || rg <= 0.0
            || !rg_squared.is_finite()
            || rg_squared == 0.0
            || !(5.0 * rg_squared).is_finite()
        {
            return Err(PhysicsDomainError::ComputationNonFinite(
                "mass-derived black-hole length scale",
            ));
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Signed dimensionless Kerr spin chi = c J / (G M^2).
///
/// Sign is retained because it determines orientation relative to the
/// coordinate axis. Horizon radii depend on chi squared.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct DimensionlessSpin(f64);

impl DimensionlessSpin {
    pub fn new(value: f64) -> Result<Self, PhysicsDomainError> {
        if !value.is_finite() {
            return Err(PhysicsDomainError::NonFinite("dimensionless_spin"));
        }
        if !(-1.0..=1.0).contains(&value) {
            return Err(PhysicsDomainError::SpinOutOfRange(value));
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> f64 {
        self.0
    }
}

/// A typed domain error. Invalid physical inputs never silently become NaN.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PhysicsDomainError {
    NonFinite(&'static str),
    NonPositive(&'static str),
    SpinOutOfRange(f64),
    PolarAngleOutOfRange(f64),
    RadiusAtOrInsideHorizon { radius_m: f64, horizon_m: f64 },
    ComputationNonFinite(&'static str),
}

impl fmt::Display for PhysicsDomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite(name) => write!(f, "{name} must be finite"),
            Self::NonPositive(name) => write!(f, "{name} must be positive"),
            Self::SpinOutOfRange(value) => {
                write!(f, "dimensionless Kerr spin {value} is outside [-1, 1]")
            }
            Self::PolarAngleOutOfRange(value) => {
                write!(f, "polar angle {value} radians is outside [0, pi]")
            }
            Self::RadiusAtOrInsideHorizon {
                radius_m,
                horizon_m,
            } => write!(
                f,
                "static-observer radius {radius_m} m must exceed horizon {horizon_m} m"
            ),
            Self::ComputationNonFinite(name) => {
                write!(f, "computed {name} is outside the finite f64 range")
            }
        }
    }
}

impl std::error::Error for PhysicsDomainError {}

/// A non-rotating Schwarzschild black hole.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SchwarzschildBlackHole {
    mass: MassKg,
}

impl SchwarzschildBlackHole {
    pub const fn new(mass: MassKg) -> Self {
        Self { mass }
    }

    pub const fn mass(self) -> MassKg {
        self.mass
    }

    /// Gravitational radius GM/c^2 in metres.
    pub fn gravitational_radius_m(self) -> f64 {
        gravitational_radius_m(self.mass)
    }

    /// Event-horizon areal radius 2GM/c^2 in metres.
    pub fn horizon_radius_m(self) -> f64 {
        2.0 * self.gravitational_radius_m()
    }

    /// Photon-sphere radius 3GM/c^2 in metres.
    pub fn photon_sphere_radius_m(self) -> f64 {
        3.0 * self.gravitational_radius_m()
    }

    /// Innermost stable circular-orbit radius 6GM/c^2 in metres
    /// for a test particle on a circular orbit around a Schwarzschild hole.
    pub fn isco_radius_m(self) -> f64 {
        6.0 * self.gravitational_radius_m()
    }

    /// Kretschmann scalar R_abcd R^abcd in m^-4 at areal radius r > 0.
    ///
    /// The invariant is finite for every r > 0, including the event horizon;
    /// the physical curvature singularity is at r = 0.
    pub fn kretschmann_scalar_m4(self, radius_m: f64) -> Result<f64, PhysicsDomainError> {
        validate_positive_finite(radius_m, "radius_m")?;
        let rg = self.gravitational_radius_m();
        // Algebraically equivalent to 48 r_g^2 / r^6, but avoids
        // overflowing r^6 for large, weak-curvature exterior radii.
        let ratio = rg / radius_m;
        let direct = 48.0 * ratio * ratio / radius_m.powi(4);
        if direct.is_finite() && direct > 0.0 {
            return Ok(direct);
        }

        // Log-domain fallback covers cases where intermediate powers overflow
        // or underflow even though the final invariant is representable.
        let log_value = 48.0_f64.ln() + 2.0 * rg.ln() - 6.0 * radius_m.ln();
        let min_positive = f64::from_bits(1);
        if log_value < min_positive.ln() {
            return Ok(0.0);
        }
        if log_value > f64::MAX.ln() {
            return Err(PhysicsDomainError::ComputationNonFinite(
                "kretschmann_scalar_m4",
            ));
        }
        let value = log_value.exp();
        if value.is_finite() {
            Ok(value)
        } else {
            Err(PhysicsDomainError::ComputationNonFinite(
                "kretschmann_scalar_m4",
            ))
        }
    }

    /// Schwarzschild lapse sqrt(1 - r_s/r) for a static observer outside
    /// the horizon. It is not defined for a static observer at or inside
    /// the horizon.
    pub fn static_observer_lapse(self, radius_m: f64) -> Result<f64, PhysicsDomainError> {
        validate_positive_finite(radius_m, "radius_m")?;
        let horizon_m = self.horizon_radius_m();
        if radius_m <= horizon_m {
            return Err(PhysicsDomainError::RadiusAtOrInsideHorizon {
                radius_m,
                horizon_m,
            });
        }
        Ok((1.0 - horizon_m / radius_m).sqrt())
    }

    /// Frequency ratio nu_infinity / nu_emitted for a photon emitted by a
    /// static observer at radius r and received by a static observer at
    /// infinity. This is 1 / (1 + z) under the stated setup.
    pub fn static_emission_frequency_ratio_to_infinity(
        self,
        emission_radius_m: f64,
    ) -> Result<f64, PhysicsDomainError> {
        self.static_observer_lapse(emission_radius_m)
    }

    /// Redshift factor 1 + z for the same static-emitter / infinity-receiver
    /// configuration. This is not a generic formula for moving emitters.
    pub fn static_emission_redshift_to_infinity(
        self,
        emission_radius_m: f64,
    ) -> Result<f64, PhysicsDomainError> {
        let redshift = 1.0 / self.static_observer_lapse(emission_radius_m)?;
        if redshift.is_finite() {
            Ok(redshift)
        } else {
            Err(PhysicsDomainError::ComputationNonFinite(
                "static_emission_redshift_to_infinity",
            ))
        }
    }
}

/// A Kerr black hole with signed dimensionless spin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrBlackHole {
    mass: MassKg,
    spin: DimensionlessSpin,
}

impl KerrBlackHole {
    pub const fn new(mass: MassKg, spin: DimensionlessSpin) -> Self {
        Self { mass, spin }
    }

    pub const fn mass(self) -> MassKg {
        self.mass
    }

    pub const fn spin(self) -> DimensionlessSpin {
        self.spin
    }

    /// Gravitational length GM/c^2 in metres.
    pub fn gravitational_radius_m(self) -> f64 {
        gravitational_radius_m(self.mass)
    }

    /// Geometrized Kerr parameter a = chi GM/c^2 in metres.
    pub fn kerr_parameter_m(self) -> f64 {
        self.spin.get() * self.gravitational_radius_m()
    }

    /// Outer Boyer-Lindquist horizon radius in metres.
    pub fn outer_horizon_radius_m(self) -> f64 {
        let rg = self.gravitational_radius_m();
        rg * (1.0 + (1.0 - self.spin.get().powi(2)).sqrt())
    }

    /// Horizon angular velocity Omega_H in radians per second.
    ///
    /// Uses Omega_H = a c / (r_+^2 + a^2), with a and r_+ in metres.
    pub fn horizon_angular_velocity_rad_s(self) -> f64 {
        let a = self.kerr_parameter_m();
        let r_plus = self.outer_horizon_radius_m();
        a * SPEED_OF_LIGHT_M_S / (r_plus * r_plus + a * a)
    }

    /// Kerr surface-gravity decay rate in inverse seconds.
    ///
    /// This is the geometrized surface-gravity frequency converted by c:
    /// kappa_t = c sqrt(r_g^2 - a^2) / (2 r_g r_+).
    /// It vanishes in the extremal-spin limit.
    pub fn surface_gravity_rate_s_inv(self) -> f64 {
        let rg = self.gravitational_radius_m();
        let chi = self.spin.get();
        let root = (1.0 - chi * chi).sqrt();
        // Equivalent to c*sqrt(r_g^2-a^2)/(2*r_g*r_+), but avoids
        // subtracting nearly equal squared length scales close to extremality.
        SPEED_OF_LIGHT_M_S * root / (2.0 * rg * (1.0 + root))
    }

    /// Positive reference frequency in Hz for a proposed direct-wave
    /// component whose angular-frequency magnitude is 2*|Omega_H|.
    /// The signed horizon angular velocity remains available separately.
    /// This is a theory-derived reference, not a waveform or measurement.
    pub fn direct_wave_reference_frequency_hz(self) -> f64 {
        self.horizon_angular_velocity_rad_s().abs() / core::f64::consts::PI
    }

    /// Inner Boyer-Lindquist horizon radius in metres.
    ///
    /// For the extremal case |chi| = 1, the two horizons coincide.
    pub fn inner_horizon_radius_m(self) -> f64 {
        let rg = self.gravitational_radius_m();
        let chi = self.spin.get();
        // Rationalized form avoids cancellation for small |chi|:
        // 1 - sqrt(1 - chi^2) = chi^2 / (1 + sqrt(1 - chi^2)).
        let root = (1.0 - chi * chi).sqrt();
        rg * (chi * chi) / (1.0 + root)
    }

    /// Outer stationary-limit surface (ergosurface) at polar angle theta.
    ///
    /// theta is measured from the positive spin axis and must lie in [0, pi].
    pub fn ergosurface_radius_m(self, theta_rad: f64) -> Result<f64, PhysicsDomainError> {
        if !theta_rad.is_finite() {
            return Err(PhysicsDomainError::NonFinite("theta_rad"));
        }
        if !(0.0..=core::f64::consts::PI).contains(&theta_rad) {
            return Err(PhysicsDomainError::PolarAngleOutOfRange(theta_rad));
        }
        let rg = self.gravitational_radius_m();
        let chi = self.spin.get();
        let radicand = 1.0 - chi * chi * theta_rad.cos().powi(2);
        // The analytic radicand is non-negative on the validated domain.
        // max(0) protects only against a tiny negative rounding artifact.
        Ok(rg * (1.0 + radicand.max(0.0).sqrt()))
    }
}

fn gravitational_radius_m(mass: MassKg) -> f64 {
    GRAVITATIONAL_CONSTANT_M3_KG_S2 * mass.get() / SPEED_OF_LIGHT_M_S.powi(2)
}

fn validate_positive_finite(value: f64, name: &'static str) -> Result<(), PhysicsDomainError> {
    if !value.is_finite() {
        return Err(PhysicsDomainError::NonFinite(name));
    }
    if value <= 0.0 {
        return Err(PhysicsDomainError::NonPositive(name));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOLAR_MASS_KG: f64 = 1.988_47e30;

    fn solar_mass() -> MassKg {
        MassKg::new(SOLAR_MASS_KG).expect("fixture mass is valid")
    }

    fn close(actual: f64, expected: f64, relative_tolerance: f64) {
        let allowed = if expected == 0.0 {
            relative_tolerance
        } else {
            relative_tolerance * expected.abs()
        };
        assert!(
            (actual - expected).abs() <= allowed,
            "actual={actual:.16e}, expected={expected:.16e}, rel_tol={relative_tolerance:.3e}"
        );
    }

    #[test]
    fn schwarzschild_reference_radii_have_correct_ratios() {
        let hole = SchwarzschildBlackHole::new(solar_mass());
        let rg = hole.gravitational_radius_m();
        close(hole.horizon_radius_m(), 2.0 * rg, 1e-14);
        close(hole.photon_sphere_radius_m(), 3.0 * rg, 1e-14);
        close(hole.isco_radius_m(), 6.0 * rg, 1e-14);
        close(hole.horizon_radius_m(), 2_953.0, 2e-3);
    }

    #[test]
    fn schwarzschild_kretschmann_has_correct_scaling_and_units() {
        let hole = SchwarzschildBlackHole::new(solar_mass());
        let r = 8.0 * hole.gravitational_radius_m();
        let k1 = hole.kretschmann_scalar_m4(r).expect("r > 0");
        let k2 = hole.kretschmann_scalar_m4(2.0 * r).expect("2r > 0");
        close(k2 / k1, 1.0 / 64.0, 1e-14);
        assert!(k1.is_finite() && k1 > 0.0);
    }

    #[test]
    fn kretschmann_log_fallback_preserves_representable_extreme_scale() {
        let mass = MassKg::new(5.0e104).unwrap();
        let hole = SchwarzschildBlackHole::new(mass);
        let radius = 1.0e78;
        let value = hole.kretschmann_scalar_m4(radius).unwrap();
        let expected_log =
            48.0_f64.ln() + 2.0 * hole.gravitational_radius_m().ln() - 6.0 * radius.ln();
        assert!(value.is_finite() && value > 0.0);
        let relative_error = (value.ln() - expected_log).abs();
        assert!(
            relative_error < 1e-12,
            "log-relative error={relative_error:e}"
        );
    }

    #[test]
    fn schwarzschild_static_observer_redshift_and_lapse_are_reciprocal() {
        let hole = SchwarzschildBlackHole::new(solar_mass());
        let r = 2.0 * hole.horizon_radius_m();
        let lapse = hole.static_observer_lapse(r).expect("outside horizon");
        close(lapse, 1.0 / 2.0_f64.sqrt(), 1e-14);
        let redshift = hole
            .static_emission_redshift_to_infinity(r)
            .expect("outside horizon");
        close(redshift, 2.0_f64.sqrt(), 1e-14);
        let ratio = hole
            .static_emission_frequency_ratio_to_infinity(r)
            .expect("outside horizon");
        close(ratio, lapse, 1e-14);
    }

    #[test]
    fn static_observer_is_rejected_at_and_inside_horizon() {
        let hole = SchwarzschildBlackHole::new(solar_mass());
        for r in [hole.horizon_radius_m(), 0.5 * hole.horizon_radius_m()] {
            assert!(matches!(
                hole.static_observer_lapse(r),
                Err(PhysicsDomainError::RadiusAtOrInsideHorizon { .. })
            ));
        }
    }

    #[test]
    fn kerr_zero_spin_reduces_to_schwarzschild_horizon() {
        let hole = KerrBlackHole::new(solar_mass(), DimensionlessSpin::new(0.0).unwrap());
        let rg = hole.gravitational_radius_m();
        close(hole.outer_horizon_radius_m(), 2.0 * rg, 1e-14);
        close(hole.inner_horizon_radius_m(), 0.0, 1e-14);
        close(
            hole.ergosurface_radius_m(core::f64::consts::FRAC_PI_2)
                .unwrap(),
            2.0 * rg,
            1e-14,
        );
        close(hole.ergosurface_radius_m(0.0).unwrap(), 2.0 * rg, 1e-14);
    }

    #[test]
    fn kerr_extremal_limit_has_coincident_horizons() {
        let hole = KerrBlackHole::new(solar_mass(), DimensionlessSpin::new(1.0).unwrap());
        let rg = hole.gravitational_radius_m();
        close(hole.outer_horizon_radius_m(), rg, 1e-14);
        close(hole.inner_horizon_radius_m(), rg, 1e-14);
    }

    #[test]
    fn kerr_horizon_thermodynamic_rates_have_correct_limits() {
        let nonrotating = KerrBlackHole::new(solar_mass(), DimensionlessSpin::new(0.0).unwrap());
        let rg = nonrotating.gravitational_radius_m();
        close(nonrotating.horizon_angular_velocity_rad_s(), 0.0, 1e-14);
        close(
            nonrotating.surface_gravity_rate_s_inv(),
            SPEED_OF_LIGHT_M_S / (4.0 * rg),
            1e-14,
        );
        close(nonrotating.direct_wave_reference_frequency_hz(), 0.0, 1e-14);

        let extremal = KerrBlackHole::new(solar_mass(), DimensionlessSpin::new(1.0).unwrap());
        close(
            extremal.horizon_angular_velocity_rad_s(),
            SPEED_OF_LIGHT_M_S / (2.0 * rg),
            1e-14,
        );
        close(extremal.surface_gravity_rate_s_inv(), 0.0, 1e-14);
        assert!(extremal.direct_wave_reference_frequency_hz() > 0.0);
    }

    #[test]
    fn small_spin_inner_horizon_avoids_subtractive_cancellation() {
        let hole = KerrBlackHole::new(solar_mass(), DimensionlessSpin::new(1.0e-8).unwrap());
        let rg = hole.gravitational_radius_m();
        let expected = rg * 0.5e-16;
        let actual = hole.inner_horizon_radius_m();
        close(actual, expected, 2e-8);
        assert!(actual > 0.0);
    }

    #[test]
    fn near_extremal_surface_gravity_remains_positive_and_finite() {
        let hole = KerrBlackHole::new(solar_mass(), DimensionlessSpin::new(1.0 - 1.0e-12).unwrap());
        let kappa = hole.surface_gravity_rate_s_inv();
        assert!(kappa.is_finite() && kappa > 0.0);
        let exact_extremal = KerrBlackHole::new(solar_mass(), DimensionlessSpin::new(1.0).unwrap());
        assert_eq!(exact_extremal.surface_gravity_rate_s_inv(), 0.0);
    }

    #[test]
    fn signed_spin_is_preserved_but_horizon_radii_are_even() {
        let positive = KerrBlackHole::new(solar_mass(), DimensionlessSpin::new(0.8).unwrap());
        let negative = KerrBlackHole::new(solar_mass(), DimensionlessSpin::new(-0.8).unwrap());
        close(
            positive.kerr_parameter_m(),
            -negative.kerr_parameter_m(),
            1e-14,
        );
        close(
            positive.outer_horizon_radius_m(),
            negative.outer_horizon_radius_m(),
            1e-14,
        );
        close(
            positive.inner_horizon_radius_m(),
            negative.inner_horizon_radius_m(),
            1e-14,
        );
    }

    #[test]
    fn kerr_ergosurface_meets_horizon_at_poles_and_lies_outside_at_equator() {
        let hole = KerrBlackHole::new(solar_mass(), DimensionlessSpin::new(0.9).unwrap());
        close(
            hole.ergosurface_radius_m(0.0).unwrap(),
            hole.outer_horizon_radius_m(),
            1e-14,
        );
        assert!(
            hole.ergosurface_radius_m(core::f64::consts::FRAC_PI_2)
                .unwrap()
                > hole.outer_horizon_radius_m()
        );
    }

    #[test]
    fn mass_constructor_rejects_derived_length_underflow_and_overflow() {
        assert!(matches!(
            MassKg::new(f64::from_bits(1)),
            Err(PhysicsDomainError::ComputationNonFinite(_))
        ));
        assert!(matches!(
            MassKg::new(f64::MAX),
            Err(PhysicsDomainError::ComputationNonFinite(_))
        ));
    }

    #[test]
    fn invalid_mass_spin_radius_and_angle_are_rejected() {
        assert!(matches!(
            MassKg::new(0.0),
            Err(PhysicsDomainError::NonPositive("mass_kg"))
        ));
        assert!(matches!(
            MassKg::new(f64::NAN),
            Err(PhysicsDomainError::NonFinite("mass_kg"))
        ));
        assert!(matches!(
            DimensionlessSpin::new(1.000_001),
            Err(PhysicsDomainError::SpinOutOfRange(_))
        ));
        assert!(matches!(
            DimensionlessSpin::new(f64::INFINITY),
            Err(PhysicsDomainError::NonFinite("dimensionless_spin"))
        ));
        let hole = SchwarzschildBlackHole::new(solar_mass());
        assert!(matches!(
            hole.kretschmann_scalar_m4(-1.0),
            Err(PhysicsDomainError::NonPositive("radius_m"))
        ));
        let kerr = KerrBlackHole::new(solar_mass(), DimensionlessSpin::new(0.5).unwrap());
        assert!(matches!(
            kerr.ergosurface_radius_m(core::f64::consts::PI + 0.01),
            Err(PhysicsDomainError::PolarAngleOutOfRange(_))
        ));
    }

    #[test]
    fn schwarzschild_curvature_is_finite_at_the_horizon_but_not_at_singularity() {
        let hole = SchwarzschildBlackHole::new(solar_mass());
        assert!(
            hole.kretschmann_scalar_m4(hole.horizon_radius_m())
                .unwrap()
                .is_finite()
        );
        assert!(matches!(
            hole.kretschmann_scalar_m4(0.0),
            Err(PhysicsDomainError::NonPositive("radius_m"))
        ));
    }
}
