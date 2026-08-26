// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Checked constructors that bind 3D primitive collider geometry to matching
//! analytical mass properties atomically.
//!
//! Existing generic constructors remain unchanged. These helpers are an additive
//! migration path for validation and future production adoption.

use nalgebra::SVector;
use symtropy_math::{Capsule, HyperBox, Point, Sphere, Transform};

use crate::body::{BodyHandle, BodyType, RigidBody};
use crate::mass_properties_3d::{MassProperties3, MassProperties3Error};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PrimitiveBody3Error {
    NonFinitePosition,
    MassProperties(MassProperties3Error),
    UnrepresentableInverseMass,
    UnrepresentableInverseInertia { axis: usize },
    ConstructedStateMismatch,
}

impl From<MassProperties3Error> for PrimitiveBody3Error {
    fn from(value: MassProperties3Error) -> Self {
        Self::MassProperties(value)
    }
}

/// Checked dynamic solid sphere whose collider radius and inertia are derived
/// from the same validated input.
pub fn dynamic_solid_sphere_3d(
    handle: BodyHandle,
    position: Point<3>,
    radius: f64,
    mass: f64,
) -> Result<RigidBody<3>, PrimitiveBody3Error> {
    let properties = MassProperties3::solid_sphere(mass, radius)?;
    build_dynamic_body(
        handle,
        position,
        properties,
        Box::new(Sphere::<3>::new(Point::origin(), radius)),
    )
}

/// Checked dynamic solid cuboid using `HyperBox<3>` half-extents.
pub fn dynamic_solid_cuboid_3d(
    handle: BodyHandle,
    position: Point<3>,
    half_extents: [f64; 3],
    mass: f64,
) -> Result<RigidBody<3>, PrimitiveBody3Error> {
    let properties = MassProperties3::solid_cuboid(mass, half_extents)?;
    build_dynamic_body(
        handle,
        position,
        properties,
        Box::new(HyperBox::<3>::new(half_extents)),
    )
}

/// Checked dynamic solid capsule matching `symtropy_math::Capsule<3>` geometry.
pub fn dynamic_solid_capsule_3d(
    handle: BodyHandle,
    position: Point<3>,
    half_height: f64,
    radius: f64,
    axis: usize,
    mass: f64,
) -> Result<RigidBody<3>, PrimitiveBody3Error> {
    let properties = MassProperties3::solid_capsule(mass, half_height, radius, axis)?;
    build_dynamic_body(
        handle,
        position,
        properties,
        Box::new(Capsule::<3>::new(half_height, radius, axis)),
    )
}

fn build_dynamic_body(
    handle: BodyHandle,
    position: Point<3>,
    properties: MassProperties3,
    collider: Box<dyn symtropy_math::Shape<3>>,
) -> Result<RigidBody<3>, PrimitiveBody3Error> {
    if !position.0.iter().all(|value| value.is_finite()) {
        return Err(PrimitiveBody3Error::NonFinitePosition);
    }
    if properties.center_of_mass != SVector::zeros() {
        // Primitive constructors in this tranche are centered. Refuse to hide
        // a future non-zero COM behind a transform convention that has not been
        // reviewed yet.
        return Err(PrimitiveBody3Error::ConstructedStateMismatch);
    }

    let inverse_mass = 1.0 / properties.mass;
    if !inverse_mass.is_finite() || inverse_mass <= 0.0 {
        return Err(PrimitiveBody3Error::UnrepresentableInverseMass);
    }

    let moments = properties.principal_inertia.moments();
    let mut inverse_moments = [0.0_f64; 3];
    for (axis, moment) in moments.iter().copied().enumerate() {
        let inverse = 1.0 / moment;
        if !inverse.is_finite() || inverse <= 0.0 {
            return Err(PrimitiveBody3Error::UnrepresentableInverseInertia { axis });
        }
        inverse_moments[axis] = inverse;
    }

    let body = RigidBody::new(
        handle,
        BodyType::Dynamic,
        Transform::from_translation(position),
        collider,
        properties.mass,
        SVector::from(moments),
    );

    if body.mass != properties.mass
        || body.inv_mass != inverse_mass
        || body.inertia != SVector::from(moments)
        || body.inv_inertia != SVector::from(inverse_moments)
        || body.position() != position.0
    {
        return Err(PrimitiveBody3Error::ConstructedStateMismatch);
    }

    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::angular_dynamics::angular_vector_to_bivector;

    #[test]
    fn sphere_collider_and_mass_properties_share_radius() {
        let body = dynamic_solid_sphere_3d(
            BodyHandle(5),
            Point::new([1.0, 2.0, 3.0]),
            2.0,
            10.0,
        )
        .unwrap();

        let sphere = body
            .collider
            .as_any()
            .downcast_ref::<Sphere<3>>()
            .expect("sphere collider");
        assert_eq!(sphere.radius, 2.0);
        assert_eq!(body.inertia, SVector::from([16.0, 16.0, 16.0]));
        assert_eq!(body.position(), SVector::from([1.0, 2.0, 3.0]));
    }

    #[test]
    fn cuboid_collider_and_mass_properties_share_half_extents() {
        let body = dynamic_solid_cuboid_3d(
            BodyHandle(1),
            Point::origin(),
            [1.0, 2.0, 3.0],
            12.0,
        )
        .unwrap();

        let cuboid = body
            .collider
            .as_any()
            .downcast_ref::<HyperBox<3>>()
            .expect("cuboid collider");
        assert_eq!(cuboid.half_extents, [1.0, 2.0, 3.0]);
        assert_eq!(body.inertia, SVector::from([52.0, 40.0, 20.0]));
    }

    #[test]
    fn capsule_collider_and_mass_properties_share_geometry_and_axis() {
        let body = dynamic_solid_capsule_3d(
            BodyHandle(2),
            Point::origin(),
            2.0,
            0.5,
            1,
            10.0,
        )
        .unwrap();

        let capsule = body
            .collider
            .as_any()
            .downcast_ref::<Capsule<3>>()
            .expect("capsule collider");
        assert_eq!(capsule.half_height, 2.0);
        assert_eq!(capsule.radius, 0.5);
        assert_eq!(capsule.axis, 1);
        assert!((body.inertia[1] - 1.214_285_714_285_714_2).abs() < 1.0e-12);
        assert!((body.inertia[0] - 18.892_857_142_857_142).abs() < 1.0e-12);
        assert!((body.inertia[2] - body.inertia[0]).abs() < 1.0e-12);
    }

    #[test]
    fn constructed_cuboid_flows_into_checked_exact_energy() {
        let mut body = dynamic_solid_cuboid_3d(
            BodyHandle(3),
            Point::origin(),
            [1.0, 2.0, 3.0],
            12.0,
        )
        .unwrap();
        body.angular_velocity = angular_vector_to_bivector(&SVector::from([1.0, 0.0, 0.0]));
        assert!((body.kinetic_energy_3d_checked().unwrap() - 26.0).abs() < 1.0e-12);
    }

    #[test]
    fn invalid_position_or_geometry_never_returns_partial_body() {
        assert!(matches!(
            dynamic_solid_sphere_3d(
                BodyHandle(0),
                Point::new([f64::NAN, 0.0, 0.0]),
                1.0,
                1.0,
            ),
            Err(PrimitiveBody3Error::NonFinitePosition)
        ));
        assert!(matches!(
            dynamic_solid_cuboid_3d(BodyHandle(0), Point::origin(), [1.0, 0.0, 1.0], 1.0),
            Err(PrimitiveBody3Error::MassProperties(
                MassProperties3Error::InvalidHalfExtent { axis: 1 }
            ))
        ));
    }

    #[test]
    fn unrepresentable_inverse_mass_is_rejected() {
        let tiny_mass = f64::from_bits(1);
        assert!(matches!(
            dynamic_solid_sphere_3d(BodyHandle(0), Point::origin(), 1.0, tiny_mass),
            Err(PrimitiveBody3Error::MassProperties(_))
                | Err(PrimitiveBody3Error::UnrepresentableInverseMass)
                | Err(PrimitiveBody3Error::UnrepresentableInverseInertia { .. })
        ));
    }

    #[test]
    fn inverse_state_matches_analytical_state() {
        let body = dynamic_solid_cuboid_3d(
            BodyHandle(9),
            Point::origin(),
            [0.5, 1.0, 2.0],
            6.0,
        )
        .unwrap();
        assert_eq!(body.inv_mass, 1.0 / body.mass);
        for axis in 0..3 {
            assert_eq!(body.inv_inertia[axis], 1.0 / body.inertia[axis]);
            assert!(body.inv_inertia[axis].is_finite());
            assert!(body.inv_inertia[axis] > 0.0);
        }
    }
}
