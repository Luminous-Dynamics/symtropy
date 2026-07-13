// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Integration test: verify that `install_triangle_mesh_narrowphase` wires the
//! BVH narrowphase correctly so that a sphere dropping onto a flat triangle mesh
//! (a single quad = 2 triangles) produces contact manifolds.
//!
//! Before the P2.1 fix the `try_mesh_contact` path in `world.rs` silently
//! returned `None` for every pair (the old downcast could never succeed), so
//! mesh bodies always fell through to the GJK convex-hull path. After the fix
//! the BVH narrowphase fires and produces ≥1 contact manifold.

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_mesh::{Triangle, TriangleMesh, install_triangle_mesh_narrowphase};
use symtropy_physics::PhysicsWorld;
use symtropy_physics::body::{BodyHandle, RigidBody};

/// Build a minimal flat quad mesh at y = 0 (two triangles, 4×4 plane).
fn flat_quad_mesh() -> TriangleMesh {
    let v = [
        SVector::<f64, 3>::from([-2.0, 0.0, -2.0]),
        SVector::<f64, 3>::from([2.0, 0.0, -2.0]),
        SVector::<f64, 3>::from([2.0, 0.0, 2.0]),
        SVector::<f64, 3>::from([-2.0, 0.0, 2.0]),
    ];
    TriangleMesh::new(vec![
        Triangle {
            vertices: [v[0], v[1], v[2]],
        },
        Triangle {
            vertices: [v[0], v[2], v[3]],
        },
    ])
}

#[test]
fn mesh_narrowphase_registered_produces_contacts() {
    let gravity = SVector::from([0.0, -9.81, 0.0]);
    let mut world = PhysicsWorld::<3>::new(gravity);

    // Wire the BVH narrowphase — this is the call under test.
    install_triangle_mesh_narrowphase(&mut world);

    // Static mesh floor at origin.
    world.add_body(RigidBody::static_body(
        BodyHandle(0),
        Point::origin(),
        Box::new(flat_quad_mesh()),
    ));

    // Dynamic sphere slightly overlapping the mesh (y=0.4, radius=0.5 → 0.1 unit overlap).
    let sphere_h = world.add_body(RigidBody::dynamic_sphere(
        BodyHandle(0),
        Point::new([0.0, 0.4, 0.0]),
        0.5,
        1.0,
    ));

    // Run one step so the narrowphase executes.
    world.step(1.0 / 60.0);

    assert!(
        !world.contacts.is_empty(),
        "Expected ≥1 contact from mesh BVH narrowphase, got 0. \
         `install_triangle_mesh_narrowphase` may not have fired correctly."
    );

    // Sphere must remain finite (no NaN explosion).
    let pos = world.body(sphere_h).unwrap().position();
    assert!(
        pos.iter().all(|v| v.is_finite()),
        "Sphere position became non-finite"
    );
}

#[test]
fn no_nan_corruption_without_mesh_narrowphase() {
    // Without registering the narrowphase, a sphere vs. a TriangleMesh body
    // falls through to the GJK convex-hull path.  Verify no NaN corruption
    // and the world doesn't panic.
    let before = symtropy_physics::nan_zeroed_count();

    let gravity = SVector::from([0.0, -9.81, 0.0]);
    let mut world = PhysicsWorld::<3>::new(gravity);
    // Intentionally NOT calling install_triangle_mesh_narrowphase.

    world.add_body(RigidBody::static_body(
        BodyHandle(0),
        Point::origin(),
        Box::new(flat_quad_mesh()),
    ));
    world.add_body(RigidBody::dynamic_sphere(
        BodyHandle(0),
        Point::new([0.0, 0.4, 0.0]),
        0.5,
        1.0,
    ));
    world.step(1.0 / 60.0);

    assert_eq!(
        symtropy_physics::nan_zeroed_count(),
        before,
        "NaN zeroing occurred during convex-hull fallback step"
    );
}
