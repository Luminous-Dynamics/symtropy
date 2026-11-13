// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Equatorial circular-null-orbit reference observables for Kerr.
//!
//! This is an analytic light-ring reference calculation, not a QNM eigenvalue
//! solver. It returns the photon-orbit radius, signed coordinate angular
//! frequency, impact parameter, circular-null residuals, and coordinate-time
//! radial Lyapunov exponent when the exterior Boyer-Lindquist calculation is
//! well-defined.
//!
//! Coordinates use x^mu=(c*t,r,theta,phi), with r and c*t in metres.
//! OrbitSense::Prograde means orbital angular momentum aligned with the signed
//! Kerr spin. At zero spin, prograde is defined as positive coordinate
//! rotation and retrograde as negative coordinate rotation.
//!
//! The extremal prograde circular-null orbit coincides with the horizon. Its
//! radius and horizon-generator frequency are returned, but impact parameter
//! and Boyer-Lindquist Lyapunov calculation are explicitly unavailable.

use core::fmt;

use super::{KerrBlackHole, PhysicsDomainError, SPEED_OF_LIGHT_M_S};

/// Orientation of an equatorial circular photon orbit relative to Kerr spin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrbitSense {
    Prograde,
    Retrograde,
}

/// Status of the metric/chart calculations for an equatorial Kerr light ring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightRingStatus {
    ExteriorBoyerLindquistValidated,
    ExtremalProgradeHorizonLimit,
}

/// Analytic and local-consistency observables for one equatorial Kerr photon orbit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrEquatorialPhotonOrbit {
    pub mass_kg: f64,
    pub dimensionless_spin: f64,
    pub sense: OrbitSense,
    pub status: LightRingStatus,
    /// Boyer-Lindquist areal radius in metres.
    pub radius_m: f64,
    /// Signed coordinate frequency d(phi)/dt in radians per second.
    pub angular_frequency_rad_s: f64,
    /// Impact parameter L/E in metres, when the exterior chart calculation applies.
    pub impact_parameter_m: Option<f64>,
    /// g_tt + 2 Omega_coord g_tphi + Omega_coord^2 g_phiphi at the orbit.
    pub null_condition_residual: Option<f64>,
    /// Partial_r g_tt + 2 Omega_coord partial_r g_tphi
    /// + Omega_coord^2 partial_r g_phiphi, in inverse metres.
    pub radial_circularity_residual_per_m: Option<f64>,
    /// Null radial potential residual divided by the sum of the magnitudes
    /// of its two defining terms; dimensionless.
    pub normalized_radial_potential_residual: Option<f64>,
    /// dR/dr residual divided by the sum of the magnitudes of its two
    /// defining terms; dimensionless.
    pub normalized_radial_potential_derivative_residual: Option<f64>,
    /// Coordinate-time radial Lyapunov exponent in inverse seconds.
    /// This is a light-ring instability scale, not a finite-l QNM frequency.
    pub radial_lyapunov_exponent_s_inv: Option<f64>,
}

/// Fail-closed errors for Kerr equatorial photon-orbit reference calculations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LightRingError {
    PhysicsDomain(PhysicsDomainError),
    NonFiniteDerivedValue(&'static str),
    InvalidCircularOrbit(&'static str),
    NonPositiveRadialInstability,
}

impl From<PhysicsDomainError> for LightRingError {
    fn from(value: PhysicsDomainError) -> Self {
        Self::PhysicsDomain(value)
    }
}

impl fmt::Display for LightRingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PhysicsDomain(error) => write!(f, "{error}"),
            Self::NonFiniteDerivedValue(name) => {
                write!(f, "derived light-ring {name} is non-finite")
            }
            Self::InvalidCircularOrbit(name) => {
                write!(f, "invalid Kerr circular-null orbit: {name}")
            }
            Self::NonPositiveRadialInstability => {
                write!(
                    f,
                    "radial null-orbit potential does not have a positive instability exponent"
                )
            }
        }
    }
}

impl std::error::Error for LightRingError {}

impl KerrEquatorialPhotonOrbit {
    /// Calculate an equatorial circular-null orbit in a Kerr background.
    ///
    /// The radius uses the closed-form equatorial photon-orbit solution.
    /// The angular frequency follows the corresponding signed circular-orbit
    /// branch. For non-extremal cases, local null/radial conditions, L/E, and
    /// the coordinate-time Lyapunov exponent are independently evaluated from
    /// the equatorial Kerr metric and null radial potential.
    pub fn calculate(hole: KerrBlackHole, sense: OrbitSense) -> Result<Self, LightRingError> {
        let mass_kg = hole.mass().get();
        let chi = hole.spin().get();
        let absolute_spin = chi.abs();
        let gravitational_radius = hole.gravitational_radius_m();
        let spin_length = hole.kerr_parameter_m();

        let spin_root = ((1.0 - absolute_spin) * (1.0 + absolute_spin)).sqrt();
        let is_extremal = spin_root == 0.0;
        let spin_orientation = if chi < 0.0 { -1.0 } else { 1.0 };

        let (radius_in_gravitational_units, coordinate_rotation_sign, denominator) = match sense {
            OrbitSense::Prograde => {
                let angle = (-absolute_spin).acos();
                let radius = 2.0 * (1.0 + (2.0 / 3.0 * angle).cos());
                let sign = spin_orientation;
                let frequency_denominator = radius.powf(1.5) + absolute_spin;
                (radius, sign, frequency_denominator)
            }
            OrbitSense::Retrograde => {
                let angle = absolute_spin.acos();
                let radius = 2.0 * (1.0 + (2.0 / 3.0 * angle).cos());
                let sign = -spin_orientation;
                let frequency_denominator = radius.powf(1.5) - absolute_spin;
                (radius, sign, frequency_denominator)
            }
        };

        if !radius_in_gravitational_units.is_finite()
            || !denominator.is_finite()
            || denominator <= 0.0
        {
            return Err(LightRingError::NonFiniteDerivedValue(
                "photon-orbit radius/frequency denominator",
            ));
        }

        let radius_m = gravitational_radius * radius_in_gravitational_units;
        let angular_frequency_rad_s =
            coordinate_rotation_sign * SPEED_OF_LIGHT_M_S / (gravitational_radius * denominator);

        if !radius_m.is_finite() || !angular_frequency_rad_s.is_finite() {
            return Err(LightRingError::NonFiniteDerivedValue(
                "photon-orbit radius or angular frequency",
            ));
        }

        // The extremal prograde orbit lies on the future horizon generator.
        // Boyer-Lindquist metric inversion / affine-to-coordinate-time
        // Lyapunov conversion is singular at this boundary; return the
        // analytically meaningful radius and frequency without inventing the
        // remaining quantities.
        if is_extremal && sense == OrbitSense::Prograde {
            return Ok(Self {
                mass_kg,
                dimensionless_spin: chi,
                sense,
                status: LightRingStatus::ExtremalProgradeHorizonLimit,
                radius_m,
                angular_frequency_rad_s,
                impact_parameter_m: None,
                null_condition_residual: None,
                radial_circularity_residual_per_m: None,
                normalized_radial_potential_residual: None,
                normalized_radial_potential_derivative_residual: None,
                radial_lyapunov_exponent_s_inv: None,
            });
        }

        let radius = radius_m;
        let a = spin_length;
        let omega_coordinate = angular_frequency_rad_s / SPEED_OF_LIGHT_M_S;

        // Equatorial Boyer-Lindquist metric at theta=pi/2.
        let g_tt = -(1.0 - 2.0 * gravitational_radius / radius);
        let g_t_phi = -2.0 * gravitational_radius * a / radius;
        let g_phi_phi = radius * radius + a * a + 2.0 * gravitational_radius * a * a / radius;

        // Exact radial derivatives of the same equatorial metric components.
        let dg_tt_dr = -2.0 * gravitational_radius / radius.powi(2);
        let dg_t_phi_dr = 2.0 * gravitational_radius * a / radius.powi(2);
        let dg_phi_phi_dr = 2.0 * radius - 2.0 * gravitational_radius * a * a / radius.powi(2);

        let null_residual = g_tt
            + 2.0 * omega_coordinate * g_t_phi
            + omega_coordinate * omega_coordinate * g_phi_phi;
        let radial_residual = dg_tt_dr
            + 2.0 * omega_coordinate * dg_t_phi_dr
            + omega_coordinate * omega_coordinate * dg_phi_phi_dr;

        // Normalize the tangent with u^(ct)=1. The conserved specific energy
        // and angular momentum are E=-u_t and L=u_phi.
        let energy = -(g_tt + omega_coordinate * g_t_phi);
        let angular_momentum = g_t_phi + omega_coordinate * g_phi_phi;
        if !energy.is_finite() || !angular_momentum.is_finite() || energy <= 0.0 {
            return Err(LightRingError::InvalidCircularOrbit(
                "non-positive or non-finite Killing energy",
            ));
        }
        let impact_parameter = angular_momentum / energy;

        // Null radial potential for E=1 in the equatorial Kerr plane:
        // R(r)=[r^2+a^2-a*b]^2-Delta*(b-a)^2.
        // At a circular orbit R=R'=0, while
        // R''=8*r^2+4*(r^2+a^2-a*b)-2*(b-a)^2.
        let r_plus = hole.outer_horizon_radius_m();
        let r_minus = hole.inner_horizon_radius_m();
        let delta = (radius - r_plus) * (radius - r_minus);
        if !delta.is_finite() || delta <= 0.0 {
            return Err(LightRingError::InvalidCircularOrbit(
                "Boyer-Lindquist radial potential is not in the exterior",
            ));
        }

        let p = radius * radius + a * a - a * impact_parameter;
        let q = impact_parameter - a;
        let radial_potential = p * p - delta * q * q;
        let radial_potential_derivative =
            4.0 * radius * p - 2.0 * (radius - gravitational_radius) * q * q;
        let radial_potential_second_derivative = 8.0 * radius * radius + 4.0 * p - 2.0 * q * q;
        let radial_potential_scale = (p * p).abs() + (delta * q * q).abs();
        let radial_potential_derivative_scale =
            (4.0 * radius * p).abs() + (2.0 * (radius - gravitational_radius) * q * q).abs();
        if radial_potential_scale <= 0.0 || radial_potential_derivative_scale <= 0.0 {
            return Err(LightRingError::InvalidCircularOrbit(
                "null radial potential residual normalization is zero",
            ));
        }
        let normalized_radial_potential_residual = radial_potential / radial_potential_scale;
        let normalized_radial_potential_derivative_residual =
            radial_potential_derivative / radial_potential_derivative_scale;

        let radial_instability_squared = radial_potential_second_derivative / 2.0;
        if !radial_instability_squared.is_finite() || radial_instability_squared <= 0.0 {
            return Err(LightRingError::NonPositiveRadialInstability);
        }

        // For E=1, the equatorial geodesic first integral gives
        // d(ct)/d(lambda) = [a(b-a)+(r^2+a^2)P/Delta]/r^2.
        // Convert the affine radial exponent to coordinate time and then to s^-1.
        let dct_dlambda = (a * q + (radius * radius + a * a) * p / delta) / (radius * radius);
        if !dct_dlambda.is_finite() || dct_dlambda <= 0.0 {
            return Err(LightRingError::InvalidCircularOrbit(
                "non-positive/non-finite coordinate-time tangent",
            ));
        }

        let affine_exponent_per_m = radial_instability_squared.sqrt() / (radius * radius);
        let lyapunov_exponent_s_inv = SPEED_OF_LIGHT_M_S * affine_exponent_per_m / dct_dlambda;

        let values = [
            null_residual,
            radial_residual,
            impact_parameter,
            radial_potential,
            radial_potential_derivative,
            normalized_radial_potential_residual,
            normalized_radial_potential_derivative_residual,
            lyapunov_exponent_s_inv,
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(LightRingError::NonFiniteDerivedValue(
                "metric residual, impact parameter, radial potential, or Lyapunov exponent",
            ));
        }

        Ok(Self {
            mass_kg,
            dimensionless_spin: chi,
            sense,
            status: LightRingStatus::ExteriorBoyerLindquistValidated,
            radius_m,
            angular_frequency_rad_s,
            impact_parameter_m: Some(impact_parameter),
            null_condition_residual: Some(null_residual),
            radial_circularity_residual_per_m: Some(radial_residual),
            normalized_radial_potential_residual: Some(normalized_radial_potential_residual),
            normalized_radial_potential_derivative_residual: Some(
                normalized_radial_potential_derivative_residual,
            ),
            radial_lyapunov_exponent_s_inv: Some(lyapunov_exponent_s_inv),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DimensionlessSpin, MassKg};

    const SOLAR_MASS_KG: f64 = 1.988_47e30;

    fn hole(chi: f64) -> KerrBlackHole {
        KerrBlackHole::new(
            MassKg::new(SOLAR_MASS_KG).unwrap(),
            DimensionlessSpin::new(chi).unwrap(),
        )
    }

    fn close(actual: f64, expected: f64, relative_tolerance: f64) {
        let allowed = if expected == 0.0 {
            relative_tolerance
        } else {
            relative_tolerance * expected.abs()
        };
        assert!(
            (actual - expected).abs() <= allowed,
            "actual={actual:.16e}, expected={expected:.16e}, allowed={allowed:.3e}"
        );
    }

    #[test]
    fn schwarzschild_photon_orbit_has_known_radius_frequency_and_lyapunov_rate() {
        let black_hole = hole(0.0);
        let orbit = KerrEquatorialPhotonOrbit::calculate(black_hole, OrbitSense::Prograde).unwrap();
        let rg = black_hole.gravitational_radius_m();
        let expected_frequency = SPEED_OF_LIGHT_M_S / (3.0 * 3.0_f64.sqrt() * rg);

        close(orbit.radius_m, 3.0 * rg, 2e-14);
        close(orbit.angular_frequency_rad_s, expected_frequency, 2e-14);
        close(
            orbit.radial_lyapunov_exponent_s_inv.unwrap(),
            expected_frequency,
            2e-13,
        );
        assert_eq!(
            orbit.status,
            LightRingStatus::ExteriorBoyerLindquistValidated
        );
    }

    #[test]
    fn retrograde_schwarzschild_orbit_is_the_opposite_orientation() {
        let black_hole = hole(0.0);
        let prograde =
            KerrEquatorialPhotonOrbit::calculate(black_hole, OrbitSense::Prograde).unwrap();
        let retrograde =
            KerrEquatorialPhotonOrbit::calculate(black_hole, OrbitSense::Retrograde).unwrap();
        close(prograde.radius_m, retrograde.radius_m, 1e-14);
        close(
            prograde.angular_frequency_rad_s,
            -retrograde.angular_frequency_rad_s,
            1e-14,
        );
        close(
            prograde.radial_lyapunov_exponent_s_inv.unwrap(),
            retrograde.radial_lyapunov_exponent_s_inv.unwrap(),
            1e-14,
        );
    }

    #[test]
    fn prograde_and_retrograde_kerr_branches_have_expected_radius_ordering() {
        for chi in [0.2, 0.5, 0.9, -0.2, -0.5, -0.9] {
            let prograde =
                KerrEquatorialPhotonOrbit::calculate(hole(chi), OrbitSense::Prograde).unwrap();
            let retrograde =
                KerrEquatorialPhotonOrbit::calculate(hole(chi), OrbitSense::Retrograde).unwrap();
            assert!(prograde.radius_m < retrograde.radius_m);
            assert!(prograde.radial_lyapunov_exponent_s_inv.unwrap() > 0.0);
            assert!(retrograde.radial_lyapunov_exponent_s_inv.unwrap() > 0.0);
            assert_eq!(
                prograde.angular_frequency_rad_s.is_sign_positive(),
                chi > 0.0
            );
            assert_eq!(
                retrograde.angular_frequency_rad_s.is_sign_negative(),
                chi > 0.0
            );
        }
    }

    #[test]
    fn local_metric_null_and_radial_circularity_residuals_are_small() {
        for chi in [-0.99, -0.8, -0.3, 0.0, 0.3, 0.8, 0.99] {
            for sense in [OrbitSense::Prograde, OrbitSense::Retrograde] {
                let orbit = KerrEquatorialPhotonOrbit::calculate(hole(chi), sense).unwrap();
                let null = orbit.null_condition_residual.unwrap().abs();
                let radial = orbit.radial_circularity_residual_per_m.unwrap().abs();
                let rg = hole(chi).gravitational_radius_m();
                assert!(null < 2e-13, "chi={chi}, sense={sense:?}, null={null:e}");
                assert!(
                    radial * rg < 2e-13,
                    "chi={chi}, sense={sense:?}, normalized radial={:e}",
                    radial * rg
                );
                assert!(
                    orbit.normalized_radial_potential_residual.unwrap().abs() < 2e-12,
                    "chi={chi}, sense={sense:?}, normalized R={:e}",
                    orbit.normalized_radial_potential_residual.unwrap()
                );
                assert!(
                    orbit
                        .normalized_radial_potential_derivative_residual
                        .unwrap()
                        .abs()
                        < 2e-12,
                    "chi={chi}, sense={sense:?}, normalized R'={:e}",
                    orbit
                        .normalized_radial_potential_derivative_residual
                        .unwrap()
                );
            }
        }
    }

    #[test]
    fn retrograde_branch_remains_exterior_at_extremal_spin() {
        let black_hole = hole(1.0);
        let orbit =
            KerrEquatorialPhotonOrbit::calculate(black_hole, OrbitSense::Retrograde).unwrap();
        close(
            orbit.radius_m,
            4.0 * black_hole.gravitational_radius_m(),
            2e-14,
        );
        assert!(orbit.radial_lyapunov_exponent_s_inv.unwrap() > 0.0);
    }

    #[test]
    fn extremal_prograde_returns_horizon_limit_without_boyer_lindquist_lyapunov() {
        let black_hole = hole(1.0);
        let orbit = KerrEquatorialPhotonOrbit::calculate(black_hole, OrbitSense::Prograde).unwrap();
        let rg = black_hole.gravitational_radius_m();
        close(orbit.radius_m, rg, 1e-14);
        close(
            orbit.angular_frequency_rad_s,
            SPEED_OF_LIGHT_M_S / (2.0 * rg),
            2e-14,
        );
        assert_eq!(orbit.status, LightRingStatus::ExtremalProgradeHorizonLimit);
        assert!(orbit.impact_parameter_m.is_none());
        assert!(orbit.radial_lyapunov_exponent_s_inv.is_none());
    }

    #[test]
    fn near_extremal_light_ring_remains_finite_and_unstable() {
        for chi in [0.99, 0.999, 0.999_999, -0.999_999] {
            let orbit =
                KerrEquatorialPhotonOrbit::calculate(hole(chi), OrbitSense::Prograde).unwrap();
            assert!(orbit.radius_m.is_finite());
            assert!(orbit.angular_frequency_rad_s.is_finite());
            assert!(orbit.radial_lyapunov_exponent_s_inv.unwrap().is_finite());
            assert!(orbit.radial_lyapunov_exponent_s_inv.unwrap() > 0.0);
            assert!(orbit.normalized_radial_potential_residual.unwrap().abs() < 2e-8);
            assert!(
                orbit
                    .normalized_radial_potential_derivative_residual
                    .unwrap()
                    .abs()
                    < 2e-8
            );
        }
    }

    #[test]
    fn light_ring_results_reject_nonfinite_derived_orbit_when_mass_is_invalid() {
        // MassKg is validated at construction, so this assertion documents the
        // fail-closed input boundary at the public type level.
        assert!(MassKg::new(f64::NAN).is_err());
    }
}
