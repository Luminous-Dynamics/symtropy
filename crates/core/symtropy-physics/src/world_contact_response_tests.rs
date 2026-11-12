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
    let mut contact =
        ContactManifold::single(fixed_handle, moving_handle, normal, com + offsets[0], 0.0);
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
    for (row, rotational_row) in effective_mass.iter_mut().zip(rotational_rows.iter()) {
        for (entry, other_row) in row.iter_mut().zip(rotational_rows.iter()) {
            *entry = 1.0 / mass + rotational_row.dot(other_row) / inertia;
        }
    }
    let rhs = [-free_speed[0], -free_speed[1]];
    let determinant =
        effective_mass[0][0] * effective_mass[1][1] - effective_mass[0][1] * effective_mass[1][0];
    assert!(determinant > 0.0, "independent Delassus matrix must be SPD");
    let expected_impulses = [
        (rhs[0] * effective_mass[1][1] - effective_mass[0][1] * rhs[1]) / determinant,
        (effective_mass[0][0] * rhs[1] - effective_mass[1][0] * rhs[0]) / determinant,
    ];

    // The production world uses sequential impulses across solver_iterations.
    // Repeat the exact resolver kernel to isolate its manifold convergence.
    let mut callback = NoOpCallback;
    let default_iterations = world.solver_iterations;
    let mut default_iteration_impulses = None;
    let mut default_iteration_speeds = None;
    for iteration in 0..12 {
        contact = world.resolve_contact(contact, 1.0 / 60.0, &mut callback);
        if iteration + 1 == default_iterations {
            let body = &world.bodies[1];
            default_iteration_impulses = Some([contact.points[0].lambda, contact.points[1].lambda]);
            default_iteration_speeds = Some(offsets.map(|offset| {
                (body.linear_velocity
                    + integrator::angular_point_velocity(&body.angular_velocity, &offset))
                .dot(&normal)
            }));
        }
    }

    let default_iteration_impulses =
        default_iteration_impulses.expect("default solver iteration snapshot captured");
    let default_iteration_speeds =
        default_iteration_speeds.expect("default solver iteration velocities captured");
    let default_iteration_residual = default_iteration_speeds
        .iter()
        .map(|speed| speed.abs())
        .fold(0.0_f64, f64::max);
    let default_impulse_error = default_iteration_impulses
        .iter()
        .zip(expected_impulses)
        .map(|(observed, expected)| {
            (observed - expected).abs() / expected.abs().max(f64::MIN_POSITIVE)
        })
        .fold(0.0_f64, f64::max);
    assert!(
        default_iteration_residual < 2.0e-5 && default_impulse_error < 2.0e-5,
        "default {}-iteration PGS profile failed its declared residual envelope: velocity={default_iteration_residual:e}, impulse={default_impulse_error:e}",
        default_iterations
    );

    let after = &world.bodies[1];
    let observed_impulses = [contact.points[0].lambda, contact.points[1].lambda];
    let after_speeds = offsets.map(|offset| {
        (after.linear_velocity
            + integrator::angular_point_velocity(&after.angular_velocity, &offset))
        .dot(&normal)
    });
    for (i, ((observed, expected), speed)) in observed_impulses
        .iter()
        .zip(expected_impulses.iter())
        .zip(after_speeds.iter())
        .enumerate()
    {
        assert!(
            (observed - expected).abs() / expected.abs().max(f64::MIN_POSITIVE) < 2.0e-12,
            "contact {i} impulse disagrees with the independent coupled Delassus solution: observed={observed:e}, expected={expected:e}"
        );
        assert!(
            speed.abs() < 2.0e-12,
            "contact {i} retains normal velocity {speed:e}"
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

    let expected_angular_impulse =
        integrator::angular_impulse_from_point(&offsets[0], &(normal * observed_impulses[0])).add(
            &integrator::angular_impulse_from_point(&offsets[1], &(normal * observed_impulses[1])),
        );
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
    let energy_scale = before_energy
        .abs()
        .max(after_energy.abs())
        .max(impulse_work.abs());
    assert!(
        (after_energy - before_energy - impulse_work).abs() / energy_scale.max(f64::MIN_POSITIVE)
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

fn qualify_two_dynamic_body_case(frame_generator: Bivector<3>) {
    let frame = Rotor::from_bivector(&frame_generator);
    let mass_a = 2.0;
    let mass_b = 3.0;
    let inertia_a = mass_a / 6.0;
    let inertia_b = mass_b / 6.0;
    let local_normal = SVector::from([0.0, 1.0, 0.0]);
    let normal = frame.rotate_vector(&local_normal);
    let com_a = frame.rotate_vector(&SVector::zeros());
    let com_b = frame.rotate_vector(&SVector::from([0.0, 1.0, 0.0]));
    let point = frame.rotate_vector(&SVector::from([0.25, 0.5, 0.15]));
    let offset_a = point - com_a;
    let offset_b = point - com_b;

    let mut world = PhysicsWorld::<3>::new(SVector::zeros());
    world.baumgarte = 0.0;
    world.slop = 0.0;

    let mut body_a = RigidBody::<3>::new(
        BodyHandle(0),
        BodyType::Dynamic,
        Transform {
            translation: Point(com_a),
            rotation: frame.clone(),
        },
        Box::new(HyperBox::<3>::cube(0.5)),
        mass_a,
        SVector::from_element(inertia_a),
    );
    body_a.linear_velocity = normal * 0.5;
    body_a.friction = 0.0;

    let mut body_b = RigidBody::<3>::new(
        BodyHandle(1),
        BodyType::Dynamic,
        Transform {
            translation: Point(com_b),
            rotation: frame,
        },
        Box::new(HyperBox::<3>::cube(0.5)),
        mass_b,
        SVector::from_element(inertia_b),
    );
    body_b.linear_velocity = normal * -0.5;
    body_b.friction = 0.0;

    let handle_a = world.add_body(body_a);
    let handle_b = world.add_body(body_b);
    let contact = ContactManifold::single(handle_a, handle_b, normal, point, 0.0);

    let before_a = &world.bodies[0];
    let before_b = &world.bodies[1];
    let before_linear_momentum =
        before_a.linear_velocity * mass_a + before_b.linear_velocity * mass_b;
    let before_point_velocity_a = before_a.linear_velocity
        + integrator::angular_point_velocity(&before_a.angular_velocity, &offset_a);
    let before_point_velocity_b = before_b.linear_velocity
        + integrator::angular_point_velocity(&before_b.angular_velocity, &offset_b);
    let before_relative_velocity = before_point_velocity_b - before_point_velocity_a;
    let before_energy = before_a.kinetic_energy() + before_b.kinetic_energy();

    let lever_a_squared = (offset_a.norm_squared() - offset_a.dot(&normal).powi(2)).max(0.0);
    let lever_b_squared = (offset_b.norm_squared() - offset_b.dot(&normal).powi(2)).max(0.0);
    let inverse_effective_mass =
        1.0 / mass_a + 1.0 / mass_b + lever_a_squared / inertia_a + lever_b_squared / inertia_b;
    let expected_impulse = -before_relative_velocity.dot(&normal) / inverse_effective_mass;

    let mut callback = NoOpCallback;
    let solved = world.resolve_contact(contact, 1.0 / 60.0, &mut callback);
    let after_a = &world.bodies[0];
    let after_b = &world.bodies[1];
    let observed_impulse = solved.points[0].lambda;
    let impulse = normal * observed_impulse;
    let after_point_velocity_a = after_a.linear_velocity
        + integrator::angular_point_velocity(&after_a.angular_velocity, &offset_a);
    let after_point_velocity_b = after_b.linear_velocity
        + integrator::angular_point_velocity(&after_b.angular_velocity, &offset_b);
    let after_relative_velocity = after_point_velocity_b - after_point_velocity_a;

    assert!(
        (observed_impulse - expected_impulse).abs() / expected_impulse.abs().max(f64::MIN_POSITIVE)
            < 2.0e-12,
        "two-body impulse disagrees with the summed effective-mass oracle: observed={observed_impulse:e}, expected={expected_impulse:e}"
    );
    assert!(
        after_relative_velocity.dot(&normal).abs()
            / before_relative_velocity
                .dot(&normal)
                .abs()
                .max(f64::MIN_POSITIVE)
            < 2.0e-12,
        "two-body contact relative normal speed remains nonzero"
    );

    let after_linear_momentum = after_a.linear_velocity * mass_a + after_b.linear_velocity * mass_b;
    let momentum_scale = before_linear_momentum.norm().max(impulse.norm());
    assert!(
        (after_linear_momentum - before_linear_momentum).norm()
            / momentum_scale.max(f64::MIN_POSITIVE)
            < 2.0e-12,
        "two-body contact did not conserve total linear momentum"
    );

    let expected_torque_a = integrator::angular_impulse_from_point(&offset_a, &(-impulse));
    let expected_torque_b = integrator::angular_impulse_from_point(&offset_b, &impulse);
    let angular_change_a = after_a.angular_velocity.scale(inertia_a);
    let angular_change_b = after_b.angular_velocity.scale(inertia_b);
    assert!(
        angular_change_a.add(&expected_torque_a.scale(-1.0)).norm()
            / expected_torque_a.norm().max(f64::MIN_POSITIVE)
            < 2.0e-12,
        "body A angular momentum does not match its opposite contact impulse"
    );
    assert!(
        angular_change_b.add(&expected_torque_b.scale(-1.0)).norm()
            / expected_torque_b.norm().max(f64::MIN_POSITIVE)
            < 2.0e-12,
        "body B angular momentum does not match its contact impulse"
    );

    let impulse_work = impulse.dot(&((before_relative_velocity + after_relative_velocity) * 0.5));
    let after_energy = after_a.kinetic_energy() + after_b.kinetic_energy();
    let energy_scale = before_energy
        .abs()
        .max(after_energy.abs())
        .max(impulse_work.abs());
    assert!(
        (after_energy - before_energy - impulse_work).abs() / energy_scale.max(f64::MIN_POSITIVE)
            < 2.0e-12,
        "two-body impulse work and total kinetic-energy change disagree"
    );
}

#[test]
fn production_two_body_contact_response_matches_shared_impulse_oracle_across_frames() {
    for frame in [
        generator([0.0, 0.0, 0.0]),
        generator([0.31, -0.17, 0.23]),
        generator([-0.41, 0.28, 0.11]),
    ] {
        qualify_two_dynamic_body_case(frame);
    }
}
