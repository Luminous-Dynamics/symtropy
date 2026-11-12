// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Independent response checks for the production contact impulse path.
//!
//! The fixture deliberately uses a homogeneous cube: its equal principal
//! moments make the current scalar mean-inertia contact approximation exact.
//! It does not qualify anisotropic production inertia.

use super::*;
use crate::body::BodyType;
use symtropy_math::{Bivector, HyperBox, Point, Rotor, Transform};

fn generator(values: [f64; 3]) -> Bivector<3> {
    let mut result = Bivector::<3>::zero();
    result.set(0, 1, values[0]);
    result.set(0, 2, values[1]);
    result.set(1, 2, values[2]);
    result
}

fn qualify_case(frame_generator: Bivector<3>, dt: f64) {
    let frame = Rotor::from_bivector(&frame_generator);

    // A cube of half-extent 0.5 and mass 2 has I_x = I_y = I_z = 1/3.
    // The initial contact is at a bottom-face corner: r=(0.5,-0.5,0),
    // with an upward normal and unit downward point velocity.
    let mass = 2.0;
    let inertia = mass / 3.0 * (0.5_f64.powi(2) + 0.5_f64.powi(2));
    let local_normal = SVector::from([0.0, 1.0, 0.0]);
    let local_com = SVector::from([0.0, 0.5, 0.0]);
    let local_offset = SVector::from([0.5, -0.5, 0.0]);

    let normal = frame.rotate_vector(&local_normal);
    let com = frame.rotate_vector(&local_com);
    let offset = frame.rotate_vector(&local_offset);
    let point = com + offset;

    let mut world = PhysicsWorld::<3>::new(SVector::zeros());
    world.baumgarte = 0.0;
    world.slop = 0.0;

    let mut fixed = RigidBody::<3>::static_body(
        BodyHandle(0),
        Point::origin(),
        Box::new(HyperBox::<3>::cube(5.0)),
    );
    fixed.friction = 0.0;

    let mut moving = RigidBody::<3>::new(
        BodyHandle(1),
        BodyType::Dynamic,
        Transform {
            translation: Point(com),
            rotation: frame,
        },
        Box::new(HyperBox::<3>::cube(0.5)),
        mass,
        SVector::from_element(inertia),
    );
    moving.linear_velocity = -normal;
    moving.friction = 0.0;

    let fixed_handle = world.add_body(fixed);
    let moving_handle = world.add_body(moving);

    let mut contact = ContactManifold::single(fixed_handle, moving_handle, normal, point, 0.0);
    contact.points[0].restitution_bias = 0.0;

    let before_linear_velocity = world.bodies[1].linear_velocity;
    let before_angular_velocity = world.bodies[1].angular_velocity;
    let before_point_velocity = before_linear_velocity
        + integrator::angular_point_velocity(&before_angular_velocity, &offset);
    let before_normal_speed = before_point_velocity.dot(&normal);
    let before_energy = world.bodies[1].kinetic_energy();

    // Independently reconstruct the contact-space inverse effective mass.
    let normal_offset = offset.dot(&normal);
    let perpendicular_offset_squared =
        (offset.norm_squared() - normal_offset * normal_offset).max(0.0);
    let inverse_effective_mass = 1.0 / mass + (1.0 / inertia) * perpendicular_offset_squared;
    let expected_impulse_magnitude = -before_normal_speed / inverse_effective_mass;

    let mut callback = NoOpCallback;
    let solved = world.resolve_contact(contact, dt, &mut callback);
    let after = &world.bodies[1];
    let observed_impulse_magnitude = solved.points[0].lambda;
    let impulse = normal * observed_impulse_magnitude;
    let after_point_velocity = after.linear_velocity
        + integrator::angular_point_velocity(&after.angular_velocity, &offset);
    let after_normal_speed = after_point_velocity.dot(&normal);

    let impulse_scale = expected_impulse_magnitude.abs().max(f64::MIN_POSITIVE);
    assert!(
        (observed_impulse_magnitude - expected_impulse_magnitude).abs() / impulse_scale < 2.0e-12,
        "solver impulse disagrees with independent effective-mass oracle: \\
         observed={observed_impulse_magnitude:e}, expected={expected_impulse_magnitude:e}"
    );

    let velocity_scale = before_normal_speed.abs().max(f64::MIN_POSITIVE);
    assert!(
        after_normal_speed.abs() / velocity_scale < 2.0e-12,
        "realized contact-point normal velocity was not resolved: {after_normal_speed:e}"
    );

    // Independently check linear momentum and the offset-first angular
    // impulse used by the production resolver.
    let linear_momentum_change = (after.linear_velocity - before_linear_velocity) * mass;
    assert!(
        (linear_momentum_change - impulse).norm() / impulse.norm().max(f64::MIN_POSITIVE) < 2.0e-12,
        "linear momentum transfer does not equal the applied contact impulse"
    );

    let expected_angular_impulse = integrator::angular_impulse_from_point(&offset, &impulse);
    let angular_momentum_change = after
        .angular_velocity
        .add(&before_angular_velocity.scale(-1.0))
        .scale(inertia);
    assert!(
        angular_momentum_change
            .add(&expected_angular_impulse.scale(-1.0))
            .norm()
            / expected_angular_impulse.norm().max(f64::MIN_POSITIVE)
            < 2.0e-12,
        "angular momentum transfer does not equal r wedge J"
    );

    // For an instantaneous ideal impulse, kinetic-energy change equals
    // impulse dotted with the midpoint contact velocity.
    let after_energy = after.kinetic_energy();
    let impulse_work = impulse.dot(&((before_point_velocity + after_point_velocity) * 0.5));
    let energy_scale = before_energy
        .abs()
        .max(after_energy.abs())
        .max(impulse_work.abs())
        .max(f64::MIN_POSITIVE);
    let energy_residual = (after_energy - before_energy - impulse_work).abs() / energy_scale;
    assert!(
        energy_residual < 2.0e-12,
        "contact impulse work/energy balance residual is {energy_residual:e}"
    );
}

fn qualify_two_point_manifold(frame_generator: Bivector<3>) {
    let frame = Rotor::from_bivector(&frame_generator);
    let mass = 2.0;
    let inertia = 1.0 / 3.0;
    let local_normal = SVector::from([0.0, 1.0, 0.0]);
    let normal = frame.rotate_vector(&local_normal);
    let com = frame.rotate_vector(&SVector::from([0.0, 0.5, 0.0]));
    let offsets = [
        frame.rotate_vector(&SVector::from([0.5, -0.5, 0.0])),
        frame.rotate_vector(&SVector::from([-0.5, -0.5, 0.0])),
    ];

    let mut world = PhysicsWorld::<3>::new(SVector::zeros());
    world.baumgarte = 0.0;
    world.slop = 0.0;

    let mut fixed = RigidBody::<3>::static_body(
        BodyHandle(0),
        Point::origin(),
        Box::new(HyperBox::<3>::cube(5.0)),
    );
    fixed.friction = 0.0;
    let mut moving = RigidBody::<3>::new(
        BodyHandle(1),
        BodyType::Dynamic,
        Transform {
            translation: Point(com),
            rotation: frame,
        },
        Box::new(HyperBox::<3>::cube(0.5)),
        mass,
        SVector::from_element(inertia),
    );
    moving.linear_velocity = -normal;
    moving.friction = 0.0;

    let fixed_handle = world.add_body(fixed);
    let moving_handle = world.add_body(moving);
    let mut contact = ContactManifold::single(
        fixed_handle,
        moving_handle,
        normal,
        com + offsets[0],
        0.0,
    );
    contact.points.push(crate::contact::ContactPoint {
        position: com + offsets[1],
        depth: 0.0,
        lambda: 0.0,
        restitution_bias: 0.0,
    });

    let before_linear_velocity = world.bodies[1].linear_velocity;
    let before_angular_velocity = world.bodies[1].angular_velocity;
    let before_energy = world.bodies[1].kinetic_energy();
    let free_speed = offsets.map(|offset| {
        (before_linear_velocity
            + integrator::angular_point_velocity(&before_angular_velocity, &offset))
        .dot(&normal)
    });

    // Independently construct the 2x2 Delassus operator:
    // W_ij = 1/m + (r_i x n)^T I^-1 (r_j x n).
    let rotational_rows = offsets.map(|offset| offset.cross(&normal));
    let mut effective_mass = [[0.0; 2]; 2];
    for i in 0..2 {
        for j in 0..2 {
            effective_mass[i][j] =
                1.0 / mass + rotational_rows[i].dot(&rotational_rows[j]) / inertia;
        }
    }
    let rhs = [-free_speed[0], -free_speed[1]];
    let determinant = effective_mass[0][0] * effective_mass[1][1]
        - effective_mass[0][1] * effective_mass[1][0];
    assert!(determinant > 0.0, "independent Delassus matrix must be SPD");
    let expected_impulses = [
        (rhs[0] * effective_mass[1][1] - effective_mass[0][1] * rhs[1])
            / determinant,
        (effective_mass[0][0] * rhs[1] - effective_mass[1][0] * rhs[0])
            / determinant,
    ];

    // The production world uses sequential impulses across solver_iterations.
    // Repeat the exact resolver kernel to isolate its manifold convergence.
    let mut callback = NoOpCallback;
    for _ in 0..12 {
        contact = world.resolve_contact(contact, 1.0 / 60.0, &mut callback);
    }

    let after = &world.bodies[1];
    let observed_impulses = [contact.points[0].lambda, contact.points[1].lambda];
    let after_speeds = offsets.map(|offset| {
        (after.linear_velocity
            + integrator::angular_point_velocity(&after.angular_velocity, &offset))
        .dot(&normal)
    });
    for i in 0..2 {
        assert!(
            (observed_impulses[i] - expected_impulses[i]).abs()
                / expected_impulses[i].abs().max(f64::MIN_POSITIVE)
                < 2.0e-12,
            "contact {i} impulse disagrees with the independent coupled Delassus solution: observed={:e}, expected={:e}",
            observed_impulses[i],
            expected_impulses[i]
        );
        assert!(
            after_speeds[i].abs() < 2.0e-12,
            "contact {i} retains normal velocity {:e}",
            after_speeds[i]
        );
    }

    // The resultant linear and angular impulses must equal the measured
    // momentum changes, independently of the contact iteration ordering.
    let resultant_impulse = normal * observed_impulses.iter().sum::<f64>();
    let linear_momentum_change = (after.linear_velocity - before_linear_velocity) * mass;
    assert!(
        (linear_momentum_change - resultant_impulse).norm()
            / resultant_impulse.norm().max(f64::MIN_POSITIVE)
            < 2.0e-12,
        "two-point linear momentum transfer is inconsistent"
    );

    let expected_angular_impulse = integrator::angular_impulse_from_point(
        &offsets[0],
        &(normal * observed_impulses[0]),
    )
    .add(&integrator::angular_impulse_from_point(
        &offsets[1],
        &(normal * observed_impulses[1]),
    ));
    let angular_momentum_change = after
        .angular_velocity
        .add(&before_angular_velocity.scale(-1.0))
        .scale(inertia);
    let angular_scale = integrator::angular_impulse_from_point(&offsets[0], &normal)
        .norm()
        .max(integrator::angular_impulse_from_point(&offsets[1], &normal).norm());
    assert!(
        angular_momentum_change
            .add(&expected_angular_impulse.scale(-1.0))
            .norm()
            / angular_scale.max(f64::MIN_POSITIVE)
            < 2.0e-12,
        "two-point angular momentum transfer is inconsistent"
    );

    let impulse_work = (0..2)
        .map(|i| observed_impulses[i] * (free_speed[i] + after_speeds[i]) * 0.5)
        .sum::<f64>();
    let after_energy = after.kinetic_energy();
    let energy_scale = before_energy.abs().max(after_energy.abs()).max(impulse_work.abs());
    assert!(
        (after_energy - before_energy - impulse_work).abs()
            / energy_scale.max(f64::MIN_POSITIVE)
            < 2.0e-12,
        "two-point impulse work does not balance kinetic energy"
    );
}

#[test]
fn production_two_point_contact_converges_to_independent_delassus_oracle() {
    for frame in [
        generator([0.0, 0.0, 0.0]),
        generator([0.31, -0.17, 0.23]),
        generator([-0.41, 0.28, 0.11]),
    ] {
        qualify_two_point_manifold(frame);
    }
}

#[test]
fn production_contact_response_matches_independent_oracle_across_frames_and_steps() {
    let frames = [
        generator([0.0, 0.0, 0.0]),
        generator([0.31, -0.17, 0.23]),
        generator([-0.41, 0.28, 0.11]),
    ];

    for frame in frames {
        for dt in [1.0 / 30.0, 1.0 / 60.0, 1.0 / 240.0] {
            qualify_case(frame, dt);
        }
    }
}
