// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Sync ND physics bodies to Bevy Transform components.

use crate::projection::Projector4D;
use bevy::prelude::*;
use symtropy_physics::body::BodyHandle;

/// Bevy component that links an entity to a physics body.
///
/// Attach this to a Bevy entity (with a SpriteBundle or MeshBundle)
/// to have its Transform automatically updated from the physics world.
#[derive(Component)]
pub struct PhysicsBody {
    /// Handle to the corresponding body in PhysicsWorld.
    pub handle: BodyHandle,
    /// Visual radius for rendering (may differ from collider radius).
    pub visual_radius: f32,
}

impl PhysicsBody {
    pub fn new(handle: BodyHandle, visual_radius: f32) -> Self {
        Self {
            handle,
            visual_radius,
        }
    }
}

/// Sync the 2D physics world to Bevy transforms.
///
/// This is a Bevy system that reads from PhysicsWorld<2> (stored as a Resource)
/// and updates all entities with a PhysicsBody component.
///
/// Usage in your Bevy app:
/// ```ignore
/// app.add_systems(Update, sync_physics_2d);
/// ```
pub fn sync_physics_2d(
    physics: Res<PhysicsWorldRes2D>,
    mut query: Query<(&PhysicsBody, &mut Transform)>,
) {
    for (body_comp, mut transform) in &mut query {
        if let Some(body) = physics.world.body(body_comp.handle) {
            let pos = body.position();
            transform.translation.x = pos[0] as f32;
            transform.translation.y = pos[1] as f32;
        }
    }
}

/// Sync the 3D physics world to Bevy transforms.
pub fn sync_physics_3d(
    physics: Res<PhysicsWorldRes3D>,
    mut query: Query<(&PhysicsBody, &mut Transform)>,
) {
    for (body_comp, mut transform) in &mut query {
        if let Some(body) = physics.world.body(body_comp.handle) {
            let pos = body.position();
            transform.translation.x = pos[0] as f32;
            transform.translation.y = pos[1] as f32;
            transform.translation.z = pos[2] as f32;
        }
    }
}

/// Render-side projection metadata for a body in the 4D slice.
///
/// `alpha` is an explicit signal for a material/shader to consume; it is not
/// automatically applied to arbitrary Bevy materials. `in_slice` controls
/// standard Bevy visibility in `sync_physics_4d`. Geometry scale remains
/// geometry scale and is never overloaded to carry the fourth coordinate.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct WSliceProjectionState {
    /// Absolute W distance from the slice plane.
    pub distance_from_slice: f64,
    /// Linear slice-fade signal in [0, 1].
    pub alpha: f32,
    /// Whether the body lies inside the configured slice thickness.
    pub in_slice: bool,
}

fn w_slice_projection_state(w: f64, projector: &Projector4D) -> WSliceProjectionState {
    let invalid_config = !w.is_finite()
        || !projector.w_slice.is_finite()
        || !projector.slice_thickness.is_finite()
        || projector.slice_thickness <= 0.0;

    if invalid_config {
        return WSliceProjectionState {
            distance_from_slice: f64::INFINITY,
            alpha: 0.0,
            in_slice: false,
        };
    }

    let distance = (w - projector.w_slice).abs();
    let in_slice = distance < projector.slice_thickness;
    let alpha = if in_slice {
        (1.0 - distance / projector.slice_thickness).clamp(0.0, 1.0) as f32
    } else {
        0.0
    };

    WSliceProjectionState {
        distance_from_slice: distance,
        alpha,
        in_slice,
    }
}

/// Sync the 4D physics world to Bevy transforms and the configured W slice.
///
/// The first three coordinates become Bevy translation. The fourth coordinate
/// is evaluated against the required `Projector4D` resource. Out-of-slice
/// bodies are hidden when they have a `Visibility` component; in-slice bodies
/// return to inherited visibility. If present, `WSliceProjectionState` receives
/// the distance/fade values for material or diagnostic systems.
///
/// This system owns `Visibility` for entities carrying `PhysicsBody` that
/// participate in the 4D projection. Callers should attach
/// `WSliceProjectionState` when they need the fade signal. That signal is
/// descriptive until a material/shader explicitly consumes it.
pub fn sync_physics_4d(
    physics: Res<PhysicsWorldRes4D>,
    projector: Res<Projector4D>,
    mut query: Query<(
        &PhysicsBody,
        &mut Transform,
        Option<&mut Visibility>,
        Option<&mut WSliceProjectionState>,
    )>,
) {
    for (body_comp, mut transform, visibility, slice_state) in &mut query {
        if let Some(body) = physics.world.body(body_comp.handle) {
            let pos = body.position();
            transform.translation.x = pos[0] as f32;
            transform.translation.y = pos[1] as f32;
            transform.translation.z = pos[2] as f32;

            let projected = w_slice_projection_state(pos[3], &projector);
            if let Some(mut state) = slice_state {
                *state = projected;
            }
            if let Some(mut visibility) = visibility {
                *visibility = if projected.in_slice {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
        }
    }
}

/// Bevy Resource wrapping a 2D physics world.
#[derive(Resource)]
pub struct PhysicsWorldRes2D {
    pub world: symtropy_physics::PhysicsWorld<2>,
}

/// Bevy Resource wrapping a 3D physics world.
#[derive(Resource)]
pub struct PhysicsWorldRes3D {
    pub world: symtropy_physics::PhysicsWorld<3>,
}

/// Bevy Resource wrapping a 4D physics world.
#[derive(Resource)]
pub struct PhysicsWorldRes4D {
    pub world: symtropy_physics::PhysicsWorld<4>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn w_slice_projection_is_fully_opaque_on_the_slice_plane() {
        let projector = Projector4D::new(2.0, 1.0, 1.0);
        let state = w_slice_projection_state(2.0, &projector);
        assert_eq!(state.distance_from_slice, 0.0);
        assert_eq!(state.alpha, 1.0);
        assert!(state.in_slice);
    }

    #[test]
    fn w_slice_projection_fades_linearly_inside_the_slice() {
        let projector = Projector4D::new(0.0, 2.0, 1.0);
        let state = w_slice_projection_state(1.0, &projector);
        assert_eq!(state.distance_from_slice, 1.0);
        assert_eq!(state.alpha, 0.5);
        assert!(state.in_slice);
    }

    #[test]
    fn w_slice_boundary_is_excluded_and_has_zero_alpha() {
        let projector = Projector4D::new(0.0, 1.0, 1.0);
        let at_boundary = w_slice_projection_state(1.0, &projector);
        let outside = w_slice_projection_state(1.5, &projector);
        assert!(!at_boundary.in_slice);
        assert_eq!(at_boundary.alpha, 0.0);
        assert!(!outside.in_slice);
        assert_eq!(outside.alpha, 0.0);
    }

    #[test]
    fn malformed_slice_configuration_and_nonfinite_w_fail_closed() {
        for thickness in [0.0, -1.0, f64::INFINITY, f64::NAN] {
            let projector = Projector4D::new(0.0, thickness, 1.0);
            let state = w_slice_projection_state(0.0, &projector);
            assert!(!state.in_slice);
            assert_eq!(state.alpha, 0.0);
        }

        let projector = Projector4D::new(f64::NAN, 1.0, 1.0);
        let state = w_slice_projection_state(0.0, &projector);
        assert!(!state.in_slice);
        assert_eq!(state.alpha, 0.0);

        let projector = Projector4D::new(0.0, 1.0, 1.0);
        let state = w_slice_projection_state(f64::NAN, &projector);
        assert!(!state.in_slice);
        assert_eq!(state.alpha, 0.0);
        assert_eq!(state.distance_from_slice, f64::INFINITY);
    }

    #[test]
    fn four_dimensional_sync_applies_slice_visibility_without_scaling_geometry() {
        let mut physics_world: symtropy_physics::PhysicsWorld<4> =
            symtropy_physics::PhysicsWorld::new(nalgebra::SVector::zeros());
        let in_slice = physics_world.add_body(symtropy_physics::RigidBody::<4>::dynamic_sphere(
            BodyHandle(0),
            symtropy_math::Point::new([1.0, 2.0, 3.0, 0.5]),
            0.1,
            1.0,
        ));
        let outside_slice =
            physics_world.add_body(symtropy_physics::RigidBody::<4>::dynamic_sphere(
                BodyHandle(0),
                symtropy_math::Point::new([4.0, 5.0, 6.0, 1.5]),
                0.1,
                1.0,
            ));

        let mut app = App::new();
        app.insert_resource(PhysicsWorldRes4D {
            world: physics_world,
        });
        app.insert_resource(Projector4D::new(0.0, 1.0, 1.0));
        app.add_systems(Update, sync_physics_4d);

        let original_scale = Vec3::new(2.0, 3.0, 4.0);
        let inside_entity = app
            .world_mut()
            .spawn((
                PhysicsBody::new(in_slice, 0.1),
                Transform::from_scale(original_scale),
                Visibility::Hidden,
                WSliceProjectionState {
                    distance_from_slice: f64::INFINITY,
                    alpha: 0.0,
                    in_slice: false,
                },
            ))
            .id();
        let outside_entity = app
            .world_mut()
            .spawn((
                PhysicsBody::new(outside_slice, 0.1),
                Transform::from_scale(original_scale),
                Visibility::Inherited,
                WSliceProjectionState {
                    distance_from_slice: 0.0,
                    alpha: 1.0,
                    in_slice: true,
                },
            ))
            .id();

        app.update();

        let inside_visibility = app.world().get::<Visibility>(inside_entity).unwrap();
        assert_eq!(*inside_visibility, Visibility::Inherited);
        let inside_state = app
            .world()
            .get::<WSliceProjectionState>(inside_entity)
            .unwrap();
        assert!(inside_state.in_slice);
        assert_eq!(inside_state.alpha, 0.5);
        let inside_transform = app.world().get::<Transform>(inside_entity).unwrap();
        assert_eq!(inside_transform.translation, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(inside_transform.scale, original_scale);

        let outside_visibility = app.world().get::<Visibility>(outside_entity).unwrap();
        assert_eq!(*outside_visibility, Visibility::Hidden);
        let outside_state = app
            .world()
            .get::<WSliceProjectionState>(outside_entity)
            .unwrap();
        assert!(!outside_state.in_slice);
        assert_eq!(outside_state.alpha, 0.0);
        let outside_transform = app.world().get::<Transform>(outside_entity).unwrap();
        assert_eq!(outside_transform.translation, Vec3::new(4.0, 5.0, 6.0));
        assert_eq!(outside_transform.scale, original_scale);
    }

    #[test]
    fn physics_body_component_creation() {
        let pb = PhysicsBody::new(BodyHandle(42), 16.0);
        assert_eq!(pb.handle, BodyHandle(42));
        assert_eq!(pb.visual_radius, 16.0);
    }

    #[test]
    fn physics_world_res_2d() {
        let res = PhysicsWorldRes2D {
            world: symtropy_physics::PhysicsWorld::new(nalgebra::SVector::from([0.0, -9.81])),
        };
        assert_eq!(res.world.body_count(), 0);
    }

    #[test]
    fn physics_world_res_3d() {
        let res = PhysicsWorldRes3D {
            world: symtropy_physics::PhysicsWorld::new(nalgebra::SVector::from([0.0, -9.81, 0.0])),
        };
        assert_eq!(res.world.body_count(), 0);
    }

    #[test]
    fn physics_world_res_4d() {
        let res = PhysicsWorldRes4D {
            world: symtropy_physics::PhysicsWorld::new(nalgebra::SVector::from([
                0.0, -9.81, 0.0, 0.0,
            ])),
        };
        assert_eq!(res.world.body_count(), 0);
    }
}
