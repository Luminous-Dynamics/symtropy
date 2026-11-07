// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Analytic equatorial circular-null-orbit (light-ring) reference quantities
//! for a stationary Kerr background.
//!
//! Orientation is defined relative to the black-hole spin:
//! - Prograde: orbital angular momentum is aligned with the spin.
//! - Retrograde: orbital angular momentum is anti-aligned with the spin.
//!
//! For zero spin the two radii coincide and the coordinate angular frequencies
//! have opposite signs. Returned angular frequency uses the (ct,r,theta,phi)
//! coordinate orientation; its unit is rad/s. The positive cyclic frequency
//! is |Omega|/(2*pi) in Hz.
//!
//! These are background geodesic properties. No empirical connection to a
//! measured non-QNM ringdown component is assumed by this module.

use super::{KerrBlackHole, SPEED_OF_LIGHT_M_S};

/// Circular equatorial null orbit direction relative to the black-hole spin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightRingBranch {
    Prograde,
    Retrograde,
}

/// Analytic equatorial Kerr photon-orbit reference values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrEquatorialLightRing {
    /// Areal/Boyer-Lindquist radial coordinate in metres.
    pub radius_m: f64,
    /// Signed d(phi)/dt in radians per second for x^mu=(ct,r,theta,phi).
    pub angular_frequency_rad_s: f64,
    /// Nonnegative cyclic frequency |Omega|/(2*pi) in hertz.
    pub frequency_hz: f64,
    /// The orientation requested relative to the spin axis.
    pub branch: LightRingBranch,
}

/// Return the equatorial circular-null-orbit radius and coordinate angular
/// frequency for the selected Kerr branch.
///
/// Let M = GM_SI/c^2, chi be the signed dimensionless spin, and d=+/-1 be
/// the coordinate angular orientation. The branch selects d aligned or
/// anti-aligned with chi. The dimensionless radius is
///
/// r/M = 2 [1 + cos((2/3) acos(-d*chi))],
///
/// and Omega = d*c / (M * ((r/M)^(3/2) + d*chi)).
///
/// The formula includes the Schwarzschild and extremal Kerr limits.
pub fn kerr_equatorial_light_ring(
    hole: KerrBlackHole,
    branch: LightRingBranch,
) -> KerrEquatorialLightRing {
    let chi = hole.spin().get();
    let spin_orientation = if chi < 0.0 { -1.0 } else { 1.0 };
    let branch_sign = match branch {
        LightRingBranch::Prograde => 1.0,
        LightRingBranch::Retrograde => -1.0,
    };
    let coordinate_orientation = branch_sign * spin_orientation;

    let rg = hole.gravitational_radius_m();
    let argument = (-coordinate_orientation * chi).clamp(-1.0, 1.0);
    let angle = (2.0 / 3.0) * argument.acos();
    let dimensionless_radius = 2.0 * (1.0 + angle.cos());
    let radius_m = rg * dimensionless_radius;

    let denominator = dimensionless_radius.powf(1.5) + coordinate_orientation * chi;
    let angular_frequency_rad_s =
        coordinate_orientation * SPEED_OF_LIGHT_M_S / (rg * denominator);
    let frequency_hz = angular_frequency_rad_s.abs() / (2.0 * core::f64::consts::PI);

    KerrEquatorialLightRing {
        radius_m,
        angular_frequency_rad_s,
        frequency_hz,
        branch,
    }
}

#[cfg(test)]
mod tests {
    use super::super::{DimensionlessSpin, MassKg};
    use super::*;

    const SOLAR_MASS_KG: f64 = 1.988_47e30;

    fn solar_mass() -> MassKg {
        MassKg::new(SOLAR_MASS_KG).unwrap()
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        let allowed = if expected == 0.0 {
            tolerance
        } else {
            tolerance * expected.abs()
        };
        assert!(
            (actual - expected).abs() <= allowed,
            "actual={actual:.16e}, expected={expected:.16e}, allowed={allowed:.3e}"
        );
    }

    #[test]
    fn schwarzschild_photon_orbits_have_exact_radius_and_frequency_limits() {
        let hole = KerrBlackHole::new(solar_mass(), DimensionlessSpin::new(0.0).unwrap());
        let rg = hole.gravitational_radius_m();
        let prograde = kerr_equatorial_light_ring(hole, LightRingBranch::Prograde);
        let retrograde = kerr_equatorial_light_ring(hole, LightRingBranch::Retrograde);

        close(prograde.radius_m, 3.0 * rg, 2e-15);
        close(retrograde.radius_m, 3.0 * rg, 2e-15);
        close(
            prograde.angular_frequency_rad_s,
            SPEED_OF_LIGHT_M_S / (3.0_f64.sqrt() * 3.0 * rg),
            2e-15,
        );
        close(
            retrograde.angular_frequency_rad_s,
            -prograde.angular_frequency_rad_s,
            2e-15,
        );
        close(
            prograde.frequency_hz,
            retrograde.frequency_hz,
            2e-15,
        );
    }

    #[test]
    fn positive_spin_prograde_orbit_is_smaller_and_faster_than_retrograde() {
        let hole = KerrBlackHole::new(solar_mass(), DimensionlessSpin::new(0.5).unwrap());
        let prograde = kerr_equatorial_light_ring(hole, LightRingBranch::Prograde);
        let retrograde = kerr_equatorial_light_ring(hole, LightRingBranch::Retrograde);

        assert!(prograde.radius_m < retrograde.radius_m);
        assert!(prograde.frequency_hz > retrograde.frequency_hz);
        assert!(prograde.angular_frequency_rad_s > 0.0);
        assert!(retrograde.angular_frequency_rad_s < 0.0);
    }

    #[test]
    fn negative_spin_reverses_coordinate_sign_without_swapping_branch_physics() {
        let positive =
            KerrBlackHole::new(solar_mass(), DimensionlessSpin::new(0.7).unwrap());
        let negative =
            KerrBlackHole::new(solar_mass(), DimensionlessSpin::new(-0.7).unwrap());

        let positive_pro = kerr_equatorial_light_ring(positive, LightRingBranch::Prograde);
        let negative_pro = kerr_equatorial_light_ring(negative, LightRingBranch::Prograde);
        let positive_retro =
            kerr_equatorial_light_ring(positive, LightRingBranch::Retrograde);
        let negative_retro =
            kerr_equatorial_light_ring(negative, LightRingBranch::Retrograde);

        close(positive_pro.radius_m, negative_pro.radius_m, 2e-14);
        close(positive_retro.radius_m, negative_retro.radius_m, 2e-14);
        close(
            positive_pro.angular_frequency_rad_s,
            -negative_pro.angular_frequency_rad_s,
            2e-14,
        );
        close(
            positive_retro.angular_frequency_rad_s,
            -negative_retro.angular_frequency_rad_s,
            2e-14,
        );
    }

    #[test]
    fn extremal_kerr_prograde_and_retrograde_limits_are_distinct() {
        let hole = KerrBlackHole::new(solar_mass(), DimensionlessSpin::new(1.0).unwrap());
        let rg = hole.gravitational_radius_m();
        let prograde = kerr_equatorial_light_ring(hole, LightRingBranch::Prograde);
        let retrograde = kerr_equatorial_light_ring(hole, LightRingBranch::Retrograde);

        close(prograde.radius_m, rg, 2e-15);
        close(retrograde.radius_m, 4.0 * rg, 2e-15);
        close(
            prograde.angular_frequency_rad_s,
            SPEED_OF_LIGHT_M_S / (2.0 * rg),
            2e-15,
        );
        close(
            retrograde.angular_frequency_rad_s,
            -SPEED_OF_LIGHT_M_S / (7.0 * rg),
            2e-15,
        );
    }

    #[test]
    fn near_extremal_light_ring_quantities_remain_finite_and_ordered() {
        let hole = KerrBlackHole::new(
            solar_mass(),
            DimensionlessSpin::new(1.0 - f64::EPSILON).unwrap(),
        );
        let prograde = kerr_equatorial_light_ring(hole, LightRingBranch::Prograde);
        let retrograde = kerr_equatorial_light_ring(hole, LightRingBranch::Retrograde);

        assert!(prograde.radius_m.is_finite() && prograde.radius_m > 0.0);
        assert!(retrograde.radius_m.is_finite() && retrograde.radius_m > prograde.radius_m);
        assert!(prograde.frequency_hz.is_finite() && prograde.frequency_hz > 0.0);
        assert!(retrograde.frequency_hz.is_finite() && retrograde.frequency_hz > 0.0);
    }
}
