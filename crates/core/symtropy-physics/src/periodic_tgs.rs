// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! TGS contact kinematics and effective-mass helpers for periodic image pairs.
//!
//! The equations mirror PhysicsWorld::resolve_contact, but consume the
//! image-aware lever arms supplied by PeriodicContactGeometry. This is a
//! reference adapter; it is not yet called by PhysicsWorld.

use std::error::Error;
use std::fmt;

use nalgebra::SVector;
use symtropy_math::Bivector;

use crate::periodic_contact_cache::PeriodicContactGeometry;

const UNIT_NORMAL_TOLERANCE: f64 = 1.0e-8;
const NEGATIVE_PERPENDICULAR_TOLERANCE: f64 = 1.0e-10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodicTgsError {
    NonFiniteInput { field: &'static str },
    NegativeInput { field: &'static str },
    NonUnitNormal,
    InvalidPerpendicularTerm,
    InvalidEffectiveMass,
    NonFiniteOutput { field: &'static str },
}

impl fmt::Display for PeriodicTgsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput { field } => write!(f, "non-finite TGS input: {field}"),
            Self::NegativeInput { field } => write!(f, "negative TGS input: {field}"),
            Self::NonUnitNormal => write!(f, "TGS contact normal must be unit length"),
            Self::InvalidPerpendicularTerm => {
                write!(f, "TGS perpendicular lever-arm term is materially negative")
            }
            Self::InvalidEffectiveMass => write!(f, "TGS effective-mass denominator must be positive"),
            Self::NonFiniteOutput { field } => write!(f, "non-finite TGS output: {field}"),
        }
    }
}

impl Error for PeriodicTgsError {}

fn validate_unit_normal<const D: usize>(
    normal: &SVector<f64, D>,
) -> Result<(), PeriodicTgsError> {
    if !normal.iter().all(|value| value.is_finite()) {
        return Err(PeriodicTgsError::NonFiniteInput {
            field: "contact normal",
        });
    }
    let norm_squared = normal.norm_squared();
    if !norm_squared.is_finite() {
        return Err(PeriodicTgsError::NonFiniteInput {
            field: "contact normal norm",
        });
    }
    if (norm_squared - 1.0).abs() > UNIT_NORMAL_TOLERANCE {
        return Err(PeriodicTgsError::NonUnitNormal);
    }
    Ok(())
}

fn perpendicular_squared<const D: usize>(
    lever_arm: &SVector<f64, D>,
    normal: &SVector<f64, D>,
) -> Result<f64, PeriodicTgsError> {
    let norm_squared = lever_arm.norm_squared();
    let normal_projection = lever_arm.dot(normal);
    let value = norm_squared - normal_projection * normal_projection;
    if !norm_squared.is_finite() || !normal_projection.is_finite() || !value.is_finite() {
        return Err(PeriodicTgsError::NonFiniteOutput {
            field: "perpendicular lever arm",
        });
    }
    if value < -NEGATIVE_PERPENDICULAR_TOLERANCE {
        return Err(PeriodicTgsError::InvalidPerpendicularTerm);
    }
    Ok(value.max(0.0))
}

/// Compute the normal-direction TGS effective-mass denominator for one
/// periodic contact point.
///
/// inv_inertia_avg_a/b use the same isotropic mean-of-axes convention as the
/// current PhysicsWorld solver. A future anisotropic solver should replace
/// this parameterization with its tensor-aware quadratic form.
pub fn periodic_tgs_normal_denominator<const D: usize>(
    geometry: &PeriodicContactGeometry<D>,
    normal: &SVector<f64, D>,
    inv_mass_a: f64,
    inv_mass_b: f64,
    inv_inertia_avg_a: f64,
    inv_inertia_avg_b: f64,
    compliance_alpha: f64,
) -> Result<f64, PeriodicTgsError> {
    validate_unit_normal(normal)?;
    for (field, value) in [
        ("inverse mass A", inv_mass_a),
        ("inverse mass B", inv_mass_b),
        ("average inverse inertia A", inv_inertia_avg_a),
        ("average inverse inertia B", inv_inertia_avg_b),
        ("compliance alpha", compliance_alpha),
    ] {
        if !value.is_finite() {
            return Err(PeriodicTgsError::NonFiniteInput { field });
        }
        if value < 0.0 {
            return Err(PeriodicTgsError::NegativeInput { field });
        }
    }

    let perpendicular_a = perpendicular_squared(&geometry.lever_arm_a, normal)?;
    let perpendicular_b = perpendicular_squared(&geometry.lever_arm_b, normal)?;
    let denominator = inv_mass_a
        + inv_mass_b
        + compliance_alpha
        + inv_inertia_avg_a * perpendicular_a
        + inv_inertia_avg_b * perpendicular_b;
    if !denominator.is_finite() {
        return Err(PeriodicTgsError::NonFiniteOutput {
            field: "effective mass denominator",
        });
    }
    if denominator <= 1.0e-15 {
        return Err(PeriodicTgsError::InvalidEffectiveMass);
    }
    Ok(denominator)
}

/// Relative normal velocity at the selected periodic contact point.
///
/// Angular velocities are applied to the actual image-aware lever arms. The
/// result follows the PhysicsWorld convention: (v_point_b - v_point_a) · n,
/// where n points from the actual A body toward the selected image of B.
pub fn periodic_tgs_relative_normal_velocity<const D: usize>(
    geometry: &PeriodicContactGeometry<D>,
    normal: &SVector<f64, D>,
    linear_velocity_a: &SVector<f64, D>,
    angular_velocity_a: &Bivector<D>,
    linear_velocity_b: &SVector<f64, D>,
    angular_velocity_b: &Bivector<D>,
) -> Result<f64, PeriodicTgsError> {
    validate_unit_normal(normal)?;
    if !linear_velocity_a.iter().all(|value| value.is_finite()) {
        return Err(PeriodicTgsError::NonFiniteInput {
            field: "linear velocity A",
        });
    }
    if !linear_velocity_b.iter().all(|value| value.is_finite()) {
        return Err(PeriodicTgsError::NonFiniteInput {
            field: "linear velocity B",
        });
    }

    let point_velocity_a =
        linear_velocity_a + angular_velocity_a.apply_to_vector(&geometry.lever_arm_a);
    let point_velocity_b =
        linear_velocity_b + angular_velocity_b.apply_to_vector(&geometry.lever_arm_b);
    if !point_velocity_a.iter().all(|value| value.is_finite())
        || !point_velocity_b.iter().all(|value| value.is_finite())
    {
        return Err(PeriodicTgsError::NonFiniteOutput {
            field: "contact point velocity",
        });
    }
    let relative_normal_velocity = (point_velocity_b - point_velocity_a).dot(normal);
    if !relative_normal_velocity.is_finite() {
        return Err(PeriodicTgsError::NonFiniteOutput {
            field: "relative normal velocity",
        });
    }
    Ok(relative_normal_velocity)
}

#[cfg(test)]
mod tests {
    use super::{
        PeriodicTgsError, periodic_tgs_normal_denominator,
        periodic_tgs_relative_normal_velocity,
    };
    use crate::body::BodyHandle;
    use crate::periodic_contact_cache::periodic_contact_geometry;
    use nalgebra::SVector;
    use symtropy_math::{Bivector, LatticeOffset, PeriodicBox};

    fn domain() -> PeriodicBox<2> {
        PeriodicBox::new(
            SVector::zeros(),
            SVector::from([10.0, 0.0]),
            [true, false],
        )
        .unwrap()
    }

    fn close(a: f64, b: f64, tolerance: f64) {
        assert!(
            (a - b).abs() <= tolerance,
            "expected {a:.15e} ~= {b:.15e}"
        );
    }

    #[test]
    fn tgs_denominator_and_relative_speed_are_body_order_invariant() {
        let domain = domain();
        let centre_a = SVector::from([0.5, 0.0]);
        let centre_b = SVector::from([9.0, 0.0]);
        let point_ab = SVector::from([0.0, 0.5]);
        let normal_ab = SVector::from([-1.0, 0.0]);

        let geometry_ab = periodic_contact_geometry(
            &domain,
            BodyHandle(2),
            &centre_a,
            BodyHandle(7),
            &centre_b,
            LatticeOffset::new([-1, 0]),
            &point_ab,
        )
        .unwrap();

        let inv_mass_a = 0.5;
        let inv_mass_b = 0.25;
        let inv_inertia_a = 2.0;
        let inv_inertia_b = 3.0;
        let alpha = 0.1;
        let denominator_ab = periodic_tgs_normal_denominator(
            &geometry_ab,
            &normal_ab,
            inv_mass_a,
            inv_mass_b,
            inv_inertia_a,
            inv_inertia_b,
            alpha,
        )
        .unwrap();

        // rA_perp² = rB_perp² = 0.25, giving .75 + .1 + .5 + .75 = 2.1.
        close(denominator_ab, 2.1, 1e-12);

        let angular_a = Bivector::from_wedge(
            &SVector::from([1.0, 0.0]),
            &SVector::from([0.0, 1.0]),
        );
        let angular_b = Bivector::from_wedge(
            &SVector::from([0.5, 0.0]),
            &SVector::from([0.0, 1.0]),
        );
        let velocity_a = SVector::from([0.0, 0.0]);
        let velocity_b = SVector::from([1.0, 0.0]);

        let speed_ab = periodic_tgs_relative_normal_velocity(
            &geometry_ab,
            &normal_ab,
            &velocity_a,
            &angular_a,
            &velocity_b,
            &angular_b,
        )
        .unwrap();

        let point_ba = SVector::from([10.0, 0.5]);
        let normal_ba = -normal_ab;
        let geometry_ba = periodic_contact_geometry(
            &domain,
            BodyHandle(7),
            &centre_b,
            BodyHandle(2),
            &centre_a,
            LatticeOffset::new([1, 0]),
            &point_ba,
        )
        .unwrap();
        let denominator_ba = periodic_tgs_normal_denominator(
            &geometry_ba,
            &normal_ba,
            inv_mass_b,
            inv_mass_a,
            inv_inertia_b,
            inv_inertia_a,
            alpha,
        )
        .unwrap();
        let speed_ba = periodic_tgs_relative_normal_velocity(
            &geometry_ba,
            &normal_ba,
            &velocity_b,
            &angular_b,
            &velocity_a,
            &angular_a,
        )
        .unwrap();

        close(denominator_ba, denominator_ab, 1e-12);
        close(speed_ba, speed_ab, 1e-12);
        assert!(speed_ab < 0.0, "fixture should be approaching");
    }

    #[test]
    fn invalid_normal_and_negative_mass_terms_are_rejected() {
        let domain = domain();
        let geometry = periodic_contact_geometry(
            &domain,
            BodyHandle(2),
            &SVector::zeros(),
            BodyHandle(7),
            &SVector::zeros(),
            LatticeOffset::zero(),
            &SVector::zeros(),
        )
        .unwrap();

        assert_eq!(
            periodic_tgs_normal_denominator(
                &geometry,
                &SVector::from([2.0, 0.0]),
                1.0,
                1.0,
                1.0,
                1.0,
                0.0,
            ),
            Err(PeriodicTgsError::NonUnitNormal)
        );
        assert_eq!(
            periodic_tgs_normal_denominator(
                &geometry,
                &SVector::from([1.0, 0.0]),
                -1.0,
                1.0,
                1.0,
                1.0,
                0.0,
            ),
            Err(PeriodicTgsError::NegativeInput {
                field: "inverse mass A"
            })
        );
    }
}
