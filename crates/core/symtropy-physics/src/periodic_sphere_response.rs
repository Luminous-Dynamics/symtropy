// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Translational impulse response for periodic sphere-sphere impacts.
//!
//! This is a bounded response primitive for a single confirmed sphere impact.
//! It does not advance body positions, integrate forces, handle spin/friction,
//! or replace PhysicsWorld. The caller advances to the reported time of impact,
//! applies the returned velocities, and integrates the remaining substep.

use std::error::Error;
use std::fmt;

use nalgebra::SVector;
use crate::periodic_collision::PeriodicSphereCcdHit;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeriodicImpulseError {
    InvalidDimension,
    InvalidMass { body: &'static str },
    InvalidRestitution,
    NonFiniteInput { field: &'static str, axis: Option<usize> },
    InvalidContactNormal,
    NumericRange { field: &'static str },
}

impl fmt::Display for PeriodicImpulseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimension => write!(f, "sphere impulse response requires D >= 1"),
            Self::InvalidMass { body } => write!(f, "mass for body {body} must be finite and positive"),
            Self::InvalidRestitution => write!(f, "restitution must be finite and in [0,1]"),
            Self::NonFiniteInput { field, axis } => {
                write!(f, "non-finite impulse input {field}")?;
                if let Some(axis) = axis {
                    write!(f, " on axis {axis}")?;
                }
                Ok(())
            }
            Self::InvalidContactNormal => write!(f, "impact normal must be finite and nonzero"),
            Self::NumericRange { field } => write!(f, "numeric range failure in {field}"),
        }
    }
}

impl Error for PeriodicImpulseError {}

/// Momentum and translational kinetic-energy evidence around a single impact.
#[derive(Clone, Debug)]
pub struct PeriodicImpulseReport<const D: usize> {
    pub image_offset: symtropy_math::LatticeOffset<D>,
    pub toi_seconds: f64,
    pub toi_fraction: f64,
    pub impulse_magnitude: f64,
    pub impulse_applied: bool,
    pub velocity_a_after: SVector<f64, D>,
    pub velocity_b_after: SVector<f64, D>,
    pub momentum_before: SVector<f64, D>,
    pub momentum_after: SVector<f64, D>,
    pub kinetic_energy_before: f64,
    pub kinetic_energy_after: f64,
    pub normal_relative_speed_before: f64,
    pub normal_relative_speed_after: f64,
}

/// Apply a frictionless Newton restitution impulse for a confirmed periodic
/// sphere-sphere impact.
///
/// The hit normal is expected to point from A toward the selected periodic
/// image of B. The image offset affects collision geometry, but masses and
/// velocities remain attached to the original bodies. If the bodies are
/// already separating along the normal, the function returns an unchanged
/// velocity result with zero impulse; callers must not double-resolve a hit.
pub fn resolve_periodic_sphere_impact<const D: usize>(
    impact: &PeriodicSphereCcdHit<D>,
    velocity_a: &SVector<f64, D>,
    mass_a: f64,
    velocity_b: &SVector<f64, D>,
    mass_b: f64,
    restitution: f64,
) -> Result<PeriodicImpulseReport<D>, PeriodicImpulseError> {
    if D == 0 {
        return Err(PeriodicImpulseError::InvalidDimension);
    }
    if !mass_a.is_finite() || mass_a <= 0.0 {
        return Err(PeriodicImpulseError::InvalidMass { body: "A" });
    }
    if !mass_b.is_finite() || mass_b <= 0.0 {
        return Err(PeriodicImpulseError::InvalidMass { body: "B" });
    }
    if !restitution.is_finite() || !(0.0..=1.0).contains(&restitution) {
        return Err(PeriodicImpulseError::InvalidRestitution);
    }
    for axis in 0..D {
        if !velocity_a[axis].is_finite() {
            return Err(PeriodicImpulseError::NonFiniteInput {
                field: "velocity_a",
                axis: Some(axis),
            });
        }
        if !velocity_b[axis].is_finite() {
            return Err(PeriodicImpulseError::NonFiniteInput {
                field: "velocity_b",
                axis: Some(axis),
            });
        }
        if !impact.normal[axis].is_finite() {
            return Err(PeriodicImpulseError::NonFiniteInput {
                field: "impact normal",
                axis: Some(axis),
            });
        }
    }
    if !impact.toi_seconds.is_finite()
        || impact.toi_seconds < 0.0
        || !impact.toi_fraction.is_finite()
        || !(0.0..=1.0).contains(&impact.toi_fraction)
    {
        return Err(PeriodicImpulseError::NonFiniteInput {
            field: "time of impact",
            axis: None,
        });
    }

    let normal_length = impact.normal.norm();
    if !normal_length.is_finite() || normal_length <= 1e-12 {
        return Err(PeriodicImpulseError::InvalidContactNormal);
    }
    let normal = impact.normal / normal_length;

    let momentum_before = velocity_a * mass_a + velocity_b * mass_b;
    let kinetic_energy_before =
        0.5 * mass_a * velocity_a.norm_squared() + 0.5 * mass_b * velocity_b.norm_squared();
    let relative_velocity_before = velocity_b - velocity_a;
    let normal_relative_speed_before = relative_velocity_before.dot(&normal);

    if !momentum_before.iter().all(|v| v.is_finite())
        || !kinetic_energy_before.is_finite()
        || !normal_relative_speed_before.is_finite()
    {
        return Err(PeriodicImpulseError::NumericRange {
            field: "pre-impact diagnostics",
        });
    }

    let denominator = 1.0 / mass_a + 1.0 / mass_b;
    if !denominator.is_finite() || denominator <= 0.0 {
        return Err(PeriodicImpulseError::NumericRange {
            field: "inverse-mass sum",
        });
    }

    let impulse_magnitude = if normal_relative_speed_before < 0.0 {
        -(1.0 + restitution) * normal_relative_speed_before / denominator
    } else {
        0.0
    };
    if !impulse_magnitude.is_finite() || impulse_magnitude < 0.0 {
        return Err(PeriodicImpulseError::NumericRange {
            field: "impulse magnitude",
        });
    }

    let velocity_a_after = velocity_a - normal * (impulse_magnitude / mass_a);
    let velocity_b_after = velocity_b + normal * (impulse_magnitude / mass_b);
    if !velocity_a_after.iter().all(|v| v.is_finite())
        || !velocity_b_after.iter().all(|v| v.is_finite())
    {
        return Err(PeriodicImpulseError::NumericRange {
            field: "post-impact velocity",
        });
    }

    let momentum_after = velocity_a_after * mass_a + velocity_b_after * mass_b;
    let kinetic_energy_after = 0.5 * mass_a * velocity_a_after.norm_squared()
        + 0.5 * mass_b * velocity_b_after.norm_squared();
    let normal_relative_speed_after =
        (velocity_b_after - velocity_a_after).dot(&normal);

    if !momentum_after.iter().all(|v| v.is_finite())
        || !kinetic_energy_after.is_finite()
        || !normal_relative_speed_after.is_finite()
    {
        return Err(PeriodicImpulseError::NumericRange {
            field: "post-impact diagnostics",
        });
    }

    Ok(PeriodicImpulseReport {
        image_offset: impact.image_offset,
        toi_seconds: impact.toi_seconds,
        toi_fraction: impact.toi_fraction,
        impulse_magnitude,
        impulse_applied: impulse_magnitude > 0.0,
        velocity_a_after,
        velocity_b_after,
        momentum_before,
        momentum_after,
        kinetic_energy_before,
        kinetic_energy_after,
        normal_relative_speed_before,
        normal_relative_speed_after,
    })
}

#[cfg(test)]
mod tests {
    use super::{PeriodicImpulseError, resolve_periodic_sphere_impact};
    use crate::periodic_collision::sphere_sphere_ccd_periodic;
    use nalgebra::SVector;
    use symtropy_math::PeriodicBox;

    fn domain() -> PeriodicBox<2> {
        PeriodicBox::new(
            SVector::zeros(),
            SVector::from([10.0, 0.0]),
            [true, false],
        )
        .unwrap()
    }

    fn seam_impact() -> crate::periodic_collision::PeriodicSphereCcdHit<2> {
        sphere_sphere_ccd_periodic(
            &domain(),
            &SVector::from([0.5, 0.0]),
            &SVector::zeros(),
            0.5,
            &SVector::from([9.0, 0.0]),
            &SVector::from([2.0, 0.0]),
            0.5,
            1.0,
            8,
        )
        .unwrap()
        .unwrap()
    }

    fn close(a: f64, b: f64, tolerance: f64) {
        assert!(
            (a - b).abs() <= tolerance,
            "expected {a:.15e} ~= {b:.15e} with tolerance {tolerance:.3e}"
        );
    }

    #[test]
    fn elastic_seam_impact_conserves_momentum_and_kinetic_energy() {
        let impact = seam_impact();
        let result = resolve_periodic_sphere_impact(
            &impact,
            &SVector::zeros(),
            1.0,
            &SVector::from([2.0, 0.0]),
            1.0,
            1.0,
        )
        .unwrap();

        assert!(result.impulse_applied);
        assert_eq!(result.image_offset.periods, [-1, 0]);
        close(result.toi_seconds, 0.25, 1e-12);
        close(result.momentum_after[0], result.momentum_before[0], 1e-12);
        close(result.momentum_after[1], result.momentum_before[1], 1e-12);
        close(result.kinetic_energy_after, result.kinetic_energy_before, 1e-12);
        close(result.velocity_a_after[0], 2.0, 1e-12);
        close(result.velocity_b_after[0], 0.0, 1e-12);
    }

    #[test]
    fn inelastic_seam_impact_conserves_momentum_and_does_not_add_energy() {
        let impact = seam_impact();
        let result = resolve_periodic_sphere_impact(
            &impact,
            &SVector::zeros(),
            1.0,
            &SVector::from([2.0, 0.0]),
            1.0,
            0.0,
        )
        .unwrap();

        close(result.momentum_after[0], result.momentum_before[0], 1e-12);
        close(result.momentum_after[1], result.momentum_before[1], 1e-12);
        assert!(result.kinetic_energy_after <= result.kinetic_energy_before + 1e-12);
        close(result.velocity_a_after[0], 1.0, 1e-12);
        close(result.velocity_b_after[0], 1.0, 1e-12);
    }

    #[test]
    fn separating_bodies_receive_no_second_impulse() {
        let impact = seam_impact();
        let velocity_a = SVector::from([1.0, 0.0]);
        let velocity_b = SVector::from([-1.0, 0.0]);
        let result = resolve_periodic_sphere_impact(
            &impact,
            &velocity_a,
            1.0,
            &velocity_b,
            1.0,
            1.0,
        )
        .unwrap();

        assert!(!result.impulse_applied);
        assert_eq!(result.impulse_magnitude, 0.0);
        assert_eq!(result.velocity_a_after, velocity_a);
        assert_eq!(result.velocity_b_after, velocity_b);
    }

    #[test]
    fn invalid_restitution_and_mass_fail_closed() {
        let impact = seam_impact();
        assert_eq!(
            resolve_periodic_sphere_impact(
                &impact,
                &SVector::zeros(),
                1.0,
                &SVector::zeros(),
                1.0,
                1.5
            )
            .unwrap_err(),
            PeriodicImpulseError::InvalidRestitution
        );
        assert_eq!(
            resolve_periodic_sphere_impact(
                &impact,
                &SVector::zeros(),
                0.0,
                &SVector::zeros(),
                1.0,
                1.0
            )
            .unwrap_err(),
            PeriodicImpulseError::InvalidMass { body: "A" }
        );
    }
}
