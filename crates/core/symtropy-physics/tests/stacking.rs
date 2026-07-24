// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Regression test for the lever-arm angular contact response (Fix 3).
//!
//! Before this fix, `resolve_contact` computed relative velocity from
//! `linear_velocity` only and never applied an angular impulse, so
//! multi-point manifolds (e.g. a box resting on a plane) would just
//! repeatedly push the center of mass without any lever-arm torque. That
//! made resting boxes rely entirely on the multi-point manifold's redundant
//! points to avoid pivoting, with no ability to correct rotational drift
//! once disturbed -- and no torque contribution from off-center friction at
//! all (which, per-point, can itself run away without careful load-sharing;
//! see the "friction must be per-point, not per-manifold" fix alongside this
//! test).
//!
//! This test rests two boxes side-by-side on a ground half-space (a 4-point
//! manifold each, via the analytical `HalfSpace` fast path) and asserts both
//! settle into a stable, non-jittering configuration.
//!
//! NOTE on scope: an earlier version of this test stacked box B directly on
//! top of box A. That surfaced a real, but separate and pre-existing, bug in
//! `epa_3d`'s bounding-sphere fallback (triggered whenever GJK's terminating
//! simplex has fewer than 4 points) significantly overestimating the
//! penetration depth for box-vs-box contact in some configurations -- e.g.
//! reporting ~1.4 units of penetration for two half-extent-0.5 boxes that
//! were only fractionally overlapping. That was a GJK/EPA convex-geometry
//! accuracy gap unrelated to the 5 solver fixes originally landed in this
//! change (CCD, restitution, lever-arm contact response, hinge motor,
//! isotropic-inertia TODO), so it was left out of scope here as a follow-up
//! (P2.2); this test remained scoped to the box-vs-halfspace path (fully
//! analytical, no GJK/EPA involved) so it reliably guarded the angular
//! contact code without also depending on that separate, pre-existing gap.
//!
//! P2.2 is now fixed for the axis-aligned case (`world.rs`'s
//! `contact_box_vs_box`, an analytical SAT fast path that bypasses GJK/EPA
//! entirely for HyperBox-vs-HyperBox pairs -- see
//! SYMTROPY_IMPROVEMENT_PLAN_2026-07-21.md P2.2 for why a general GJK/EPA
//! fix was reverted in favor of this narrower, lower-risk one) -- see
//! `boxes_stack_without_explosive_impulse` below for the box-on-box
//! regression this file was originally missing.

use nalgebra::SVector;
use symtropy_math::{HalfSpace, HyperBox, Point, Transform};
use symtropy_physics::PhysicsWorld;
use symtropy_physics::body::{BodyHandle, BodyType, RigidBody};

/// Analytical solid-cuboid inertia (per-axis principal moments), matching the
/// convention used by `RigidBody::dynamic_sphere` (one scalar per axis).
fn box_inertia(mass: f64, half_extents: [f64; 3]) -> SVector<f64, 3> {
    let [hx, hy, hz] = half_extents;
    let ixx = (mass / 3.0) * (hy * hy + hz * hz);
    let iyy = (mass / 3.0) * (hx * hx + hz * hz);
    let izz = (mass / 3.0) * (hx * hx + hy * hy);
    SVector::from([ixx, iyy, izz])
}

fn make_box(pos: Point<3>, half_extents: [f64; 3], mass: f64) -> RigidBody<3> {
    let inertia = box_inertia(mass, half_extents);
    RigidBody::new(
        BodyHandle(0), // overwritten by PhysicsWorld::add_body
        BodyType::Dynamic,
        Transform::from_translation(pos),
        Box::new(HyperBox::<3>::new(half_extents)),
        mass,
        inertia,
    )
}

#[test]
fn resting_boxes_settle_without_jitter() {
    let gravity = SVector::from([0.0, -9.81, 0.0]);
    let mut world = PhysicsWorld::<3>::new(gravity);
    world.solver_iterations = 8;

    // Ground: half-space with normal +Y through the origin (top at y=0).
    let ground = RigidBody::<3>::static_body(
        BodyHandle(0),
        Point::origin(),
        Box::new(HalfSpace::<3>::new(SVector::from([0.0, 1.0, 0.0]), 0.0)),
    );
    world.add_body(ground);

    let half = [0.5, 0.5, 0.5];
    // Two boxes resting side-by-side, each with its own 4-point manifold
    // against the ground. Box B is dropped from a small height with a
    // deliberate X offset so any un-arrested lever-arm/torque error would
    // show up as toppling rather than a perfectly symmetric configuration.
    let a = world.add_body(make_box(Point::new([-1.0, 0.5, 0.0]), half, 1.0));
    let b = world.add_body(make_box(Point::new([1.05, 0.8, 0.02]), half, 1.0));

    let dt = 1.0 / 60.0;

    // Let both boxes settle.
    for _ in 0..600 {
        world.step(dt);
    }

    let pos_a = world.body(a).unwrap().position();
    let pos_b = world.body(b).unwrap().position();

    assert!(
        (pos_a[1] - 0.5).abs() < 0.05,
        "box A should rest at y≈0.5, got y={}",
        pos_a[1]
    );
    assert!(
        (pos_b[1] - 0.5).abs() < 0.05,
        "box B should rest at y≈0.5, got y={}",
        pos_b[1]
    );

    // Track frame-to-frame motion over the next second: a genuinely settled
    // configuration should not keep jittering (small oscillating position
    // deltas), which is exactly what the lever-arm torque fix guards against
    // for multi-point manifolds.
    let mut max_delta = 0.0_f64;
    let mut prev_a = pos_a;
    let mut prev_b = pos_b;
    for _ in 0..60 {
        world.step(dt);
        let cur_a = world.body(a).unwrap().position();
        let cur_b = world.body(b).unwrap().position();
        max_delta = max_delta
            .max((cur_a - prev_a).norm())
            .max((cur_b - prev_b).norm());
        prev_a = cur_a;
        prev_b = cur_b;
    }

    assert!(
        max_delta < 0.01,
        "resting boxes should not jitter once settled; max frame-to-frame delta = {max_delta}"
    );

    // Velocities should be small (settled or asleep) -- no perpetual bounce
    // or energy injection from the new angular contact response.
    let va = world.body(a).unwrap().linear_velocity.norm();
    let vb = world.body(b).unwrap().linear_velocity.norm();
    assert!(va < 0.5, "box A should be nearly at rest, v={va}");
    assert!(vb < 0.5, "box B should be nearly at rest, v={vb}");

    // Angular velocity must not have run away either (regression guard for
    // the "friction must be per-point, not per-manifold" fix: concentrating
    // friction's torque at a single point caused unbounded spin growth).
    let wa = world.body(a).unwrap().angular_velocity.norm();
    let wb = world.body(b).unwrap().angular_velocity.norm();
    assert!(
        wa < 0.5,
        "box A angular velocity should not run away, w={wa}"
    );
    assert!(
        wb < 0.5,
        "box B angular velocity should not run away, w={wb}"
    );

    // Both bodies must remain finite (no NaN/Inf explosion from the new
    // lever-arm torque code).
    assert!(pos_a.iter().all(|v| v.is_finite()));
    assert!(pos_b.iter().all(|v| v.is_finite()));
}

/// P2.2 regression: box B dropped directly on top of box A, resting on a
/// ground half-space. Box-on-box contact used to go through the generic
/// GJK/EPA path (unlike box-vs-halfspace, which has its own analytical fast
/// path) and hit `epa_3d`'s bounding-sphere-fallback bug there: GJK
/// terminates with a degenerate (<4 point) simplex for axis-aligned
/// face-to-face box contact, and the fallback badly overestimated
/// penetration depth (~1.4 units for two half-extent-0.5 boxes only
/// fractionally overlapping), which the solver would try to resolve in a
/// single frame -- an explosive impulse large enough to send a stacked box
/// flying rather than letting it settle. Box-vs-box now bypasses GJK/EPA
/// entirely via the analytical `contact_box_vs_box` SAT fast path
/// (`world.rs`), which reports the exact true penetration depth.
#[test]
fn boxes_stack_without_explosive_impulse() {
    let gravity = SVector::from([0.0, -9.81, 0.0]);
    let mut world = PhysicsWorld::<3>::new(gravity);
    world.solver_iterations = 8;

    let ground = RigidBody::<3>::static_body(
        BodyHandle(0),
        Point::origin(),
        Box::new(HalfSpace::<3>::new(SVector::from([0.0, 1.0, 0.0]), 0.0)),
    );
    world.add_body(ground);

    let half = [0.5, 0.5, 0.5];
    let a = world.add_body(make_box(Point::new([0.0, 0.5, 0.0]), half, 1.0));
    // B starts already overlapping A slightly (y=1.45 vs. A's top at y=1.0,
    // half-extent 0.5 each -> true initial penetration 0.05) to force the
    // box-vs-box path on the very first step, before any CCD/contact
    // generation from a falling approach has a chance to mask the bug.
    let b = world.add_body(make_box(Point::new([0.0, 1.45, 0.0]), half, 1.0));

    let dt = 1.0 / 60.0;
    let mut max_speed = 0.0_f64;
    for _ in 0..300 {
        world.step(dt);
        let va = world.body(a).unwrap().linear_velocity.norm();
        let vb = world.body(b).unwrap().linear_velocity.norm();
        max_speed = max_speed.max(va).max(vb);
    }

    let pos_a = world.body(a).unwrap().position();
    let pos_b = world.body(b).unwrap().position();

    assert!(
        pos_a.iter().all(|v| v.is_finite()),
        "box A exploded: {pos_a:?}"
    );
    assert!(
        pos_b.iter().all(|v| v.is_finite()),
        "box B exploded: {pos_b:?}"
    );

    // With the old bounding-sphere fallback (depth ~1.4-1.7 for this
    // configuration), a single-frame impulse resolving that overlap at
    // solver_iterations=8 would send a 1kg box to well over 10 m/s. The
    // true 0.05-unit overlap should never need anywhere close to that.
    assert!(
        max_speed < 5.0,
        "peak speed {max_speed} m/s -- looks like an explosive impulse from an \
         overestimated penetration depth"
    );

    // Box B should settle on top of A (both near y=0.5 and y=1.5), not end
    // up beside/through it.
    assert!(
        (pos_a[1] - 0.5).abs() < 0.2,
        "box A should stay near y=0.5, got y={}",
        pos_a[1]
    );
    assert!(
        (pos_b[1] - 1.5).abs() < 0.3,
        "box B should settle stacked near y=1.5, got y={}",
        pos_b[1]
    );
}
