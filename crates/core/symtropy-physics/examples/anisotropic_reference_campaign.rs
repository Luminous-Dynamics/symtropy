// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Deterministic CSV qualification campaign for the checked anisotropic references.
//!
//! Run with:
//! `cargo run --release --example anisotropic_reference_campaign -p symtropy-physics`
//!
//! This is a reference-quality campaign, not a production PhysicsWorld benchmark.
//! Its CSV output is intended to seed independent MuJoCo/Chrono differential runs.
//! Cross-engine comparison still requires a semantic-equivalence manifest for
//! frames, inertia reference points, timestep, integrator and contact law.

use nalgebra::{SMatrix, SVector};
use symtropy_math::{Bivector, Rotor};
use symtropy_physics::{
    BivectorInertia, PrincipalInertia3, angular_vector_to_bivector,
    bivector_to_angular_vector, rotational_kinetic_energy, step_principal_inertia,
    world_angular_momentum,
};

fn main() {
    println!(
        "case,parameter,observed,reference,absolute_error,relative_error,pass"
    );
    let mut failures = 0usize;

    failures += symmetric_top_precession_campaign();
    failures += asymmetric_top_convergence_campaign();
    failures += asymmetric_top_rk4_reference_campaign();
    failures += intermediate_axis_campaign();
    failures += off_center_impulse_energy_campaign();
    failures += bivector_roundtrip_campaign::<2>("D2");
    failures += bivector_roundtrip_campaign::<3>("D3");
    failures += bivector_roundtrip_campaign::<4>("D4");
    failures += bivector_roundtrip_campaign::<9>("D9");
    failures += mass_scaling_campaign();
    failures += body_frame_contact_campaign();
    failures += scale_safe_mass_properties_campaign();

    if failures != 0 {
        eprintln!("{failures} anisotropic reference campaign gate(s) failed");
        std::process::exit(1);
    }
}

fn report(case: &str, parameter: &str, observed: f64, reference: f64, tolerance: f64) -> usize {
    let error = (observed - reference).abs();
    let relative = error / reference.abs().max(1.0);
    let pass = error <= tolerance;
    println!(
        "{case},{parameter},{observed:.17},{reference:.17},{error:.17},{relative:.17},{pass}"
    );
    usize::from(!pass)
}

fn report_minimum(case: &str, parameter: &str, observed: f64, minimum: f64) -> usize {
    let margin = observed - minimum;
    let pass = observed >= minimum;
    println!(
        "{case},{parameter},{observed:.17},{minimum:.17},{:.17},{:.17},{pass}",
        margin.abs(),
        margin.abs() / minimum.abs().max(1.0)
    );
    usize::from(!pass)
}

fn symmetric_top_precession_error(dt: f64) -> f64 {
    let inertia = PrincipalInertia3::new([1.0, 1.0, 2.0]).expect("positive moments");
    let mut rotation = Rotor::<3>::identity();
    let mut angular_velocity = angular_vector_to_bivector(&SVector::from([0.8, 0.0, 0.7]));
    let duration = 1.2;
    let steps = (duration / dt).round() as usize;

    for _ in 0..steps {
        let step = step_principal_inertia(
            &rotation,
            &angular_velocity,
            inertia,
            &SVector::zeros(),
            dt,
        )
        .expect("finite symmetric-top step");
        rotation = step.rotation;
        angular_velocity = step.angular_velocity;
    }

    let omega_body = rotation
        .reverse()
        .rotate_vector(&bivector_to_angular_vector(&angular_velocity));
    let precession_rate = (2.0_f64 - 1.0) * 0.7;
    let phase = precession_rate * duration;
    let reference = SVector::from([0.8 * phase.cos(), 0.8 * phase.sin(), 0.7]);
    (omega_body - reference).norm()
}

fn symmetric_top_precession_campaign() -> usize {
    let coarse = symmetric_top_precession_error(1.0e-3);
    let fine = symmetric_top_precession_error(2.5e-4);
    let mut failures = 0;
    failures += report("symmetric_top_precession_error", "dt_1e-3", coarse, 0.0, 1.0e-2);
    failures += report("symmetric_top_precession_error", "dt_2p5e-4", fine, 0.0, 2.0e-4);
    failures += report_minimum(
        "symmetric_top_refinement_ratio",
        "coarse_over_fine",
        coarse / fine.max(f64::MIN_POSITIVE),
        3.0,
    );
    failures
}

fn asymmetric_top_observables(dt: f64) -> (f64, f64) {
    let inertia = PrincipalInertia3::new([1.0, 1.7, 2.8]).expect("positive moments");
    let mut rotation = Rotor::<3>::identity();
    let mut angular_velocity = angular_vector_to_bivector(&SVector::from([0.9, -0.6, 0.35]));
    let initial_energy =
        rotational_kinetic_energy(&rotation, &angular_velocity, inertia).expect("finite energy");
    let initial_momentum =
        world_angular_momentum(&rotation, &angular_velocity, inertia).expect("finite momentum");
    let duration = 1.0;
    let steps = (duration / dt).round() as usize;

    for _ in 0..steps {
        let step = step_principal_inertia(
            &rotation,
            &angular_velocity,
            inertia,
            &SVector::zeros(),
            dt,
        )
        .expect("finite asymmetric-top step");
        rotation = step.rotation;
        angular_velocity = step.angular_velocity;
    }

    let final_energy =
        rotational_kinetic_energy(&rotation, &angular_velocity, inertia).expect("finite energy");
    let final_momentum =
        world_angular_momentum(&rotation, &angular_velocity, inertia).expect("finite momentum");
    (
        (final_energy - initial_energy).abs() / initial_energy.abs().max(1.0e-15),
        (final_momentum - initial_momentum).norm(),
    )
}

fn asymmetric_top_convergence_campaign() -> usize {
    let (coarse_energy, coarse_momentum) = asymmetric_top_observables(1.0e-2);
    let (fine_energy, fine_momentum) = asymmetric_top_observables(2.5e-3);
    let mut failures = 0;
    failures += report(
        "asymmetric_top_relative_energy_drift",
        "dt_1e-2",
        coarse_energy,
        0.0,
        5.0e-2,
    );
    failures += report(
        "asymmetric_top_relative_energy_drift",
        "dt_2p5e-3",
        fine_energy,
        0.0,
        2.0e-2,
    );
    failures += report(
        "asymmetric_top_world_momentum_drift",
        "dt_1e-2",
        coarse_momentum,
        0.0,
        2.0e-9,
    );
    failures += report(
        "asymmetric_top_world_momentum_drift",
        "dt_2p5e-3",
        fine_momentum,
        0.0,
        2.0e-9,
    );
    failures += report_minimum(
        "asymmetric_top_energy_refinement_ratio",
        "coarse_over_fine",
        coarse_energy / fine_energy.max(f64::MIN_POSITIVE),
        1.0,
    );
    failures
}

fn euler_top_body_derivative(
    omega_body: &SVector<f64, 3>,
    principal_moments: &[f64; 3],
) -> SVector<f64, 3> {
    let angular_momentum = SVector::from([
        principal_moments[0] * omega_body[0],
        principal_moments[1] * omega_body[1],
        principal_moments[2] * omega_body[2],
    ]);
    let torque_free_rhs = angular_momentum.cross(omega_body);
    SVector::from([
        torque_free_rhs[0] / principal_moments[0],
        torque_free_rhs[1] / principal_moments[1],
        torque_free_rhs[2] / principal_moments[2],
    ])
}

fn rk4_euler_top_body(
    initial_omega: SVector<f64, 3>,
    principal_moments: [f64; 3],
    duration: f64,
    requested_dt: f64,
) -> SVector<f64, 3> {
    let steps = (duration / requested_dt).round() as usize;
    let dt = duration / steps as f64;
    let mut omega = initial_omega;
    for _ in 0..steps {
        let k1 = euler_top_body_derivative(&omega, &principal_moments);
        let k2 = euler_top_body_derivative(
            &(omega + k1 * (0.5 * dt)),
            &principal_moments,
        );
        let k3 = euler_top_body_derivative(
            &(omega + k2 * (0.5 * dt)),
            &principal_moments,
        );
        let k4 = euler_top_body_derivative(
            &(omega + k3 * dt),
            &principal_moments,
        );
        omega += (k1 + k2 * 2.0 + k3 * 2.0 + k4) * (dt / 6.0);
    }
    omega
}

fn production_body_omega_after(
    dt: f64,
    initial_body_omega: SVector<f64, 3>,
    principal_moments: [f64; 3],
    duration: f64,
) -> SVector<f64, 3> {
    let inertia = PrincipalInertia3::new(principal_moments).expect("positive moments");
    let mut rotation = Rotor::<3>::identity();
    let mut angular_velocity = angular_vector_to_bivector(&initial_body_omega);
    let steps = (duration / dt).round() as usize;
    for _ in 0..steps {
        let step = step_principal_inertia(
            &rotation,
            &angular_velocity,
            inertia,
            &SVector::zeros(),
            dt,
        )
        .expect("finite torque-free top step");
        rotation = step.rotation;
        angular_velocity = step.angular_velocity;
    }
    rotation
        .reverse()
        .rotate_vector(&bivector_to_angular_vector(&angular_velocity))
}

fn asymmetric_top_rk4_reference_campaign() -> usize {
    let principal_moments = [1.0, 1.7, 2.8];
    let initial_omega = SVector::from([0.9, -0.6, 0.35]);
    let duration = 1.0;
    // Independent body-frame Euler equations integrated with RK4. Verify
    // the reference itself by halving its already-small timestep.
    let reference = rk4_euler_top_body(
        initial_omega,
        principal_moments,
        duration,
        1.0e-5,
    );
    let reference_half_step = rk4_euler_top_body(
        initial_omega,
        principal_moments,
        duration,
        5.0e-6,
    );
    let mut failures = 0;
    failures += report(
        "asymmetric_top_rk4_reference_convergence",
        "dt_1e-5_vs_5e-6",
        (reference - reference_half_step).norm(),
        0.0,
        1.0e-11,
    );

    let coarse = production_body_omega_after(
        1.0e-2,
        initial_omega,
        principal_moments,
        duration,
    );
    let fine = production_body_omega_after(
        2.5e-3,
        initial_omega,
        principal_moments,
        duration,
    );
    let coarse_error = (coarse - reference).norm();
    let fine_error = (fine - reference).norm();
    failures += report(
        "asymmetric_top_rk4_trajectory_error",
        "production_dt_1e-2",
        coarse_error,
        0.0,
        5.0e-3,
    );
    failures += report(
        "asymmetric_top_rk4_trajectory_error",
        "production_dt_2p5e-3",
        fine_error,
        0.0,
        1.0e-3,
    );
    failures += report_minimum(
        "asymmetric_top_rk4_refinement_ratio",
        "coarse_over_fine",
        coarse_error / fine_error.max(f64::MIN_POSITIVE),
        3.0,
    );
    failures
}

fn intermediate_axis_campaign() -> usize {
    let inertia = PrincipalInertia3::new([1.0, 2.0, 3.0]).expect("positive moments");
    let mut rotation = Rotor::<3>::identity();
    let initial_body_omega = SVector::from([0.01, 1.0, 0.02]);
    let mut angular_velocity = angular_vector_to_bivector(&initial_body_omega);
    let initial_transverse = (initial_body_omega[0].powi(2) + initial_body_omega[2].powi(2)).sqrt();
    let mut maximum_transverse = initial_transverse;

    for _ in 0..20_000 {
        let step = step_principal_inertia(
            &rotation,
            &angular_velocity,
            inertia,
            &SVector::zeros(),
            1.0e-3,
        )
        .expect("finite intermediate-axis step");
        rotation = step.rotation;
        angular_velocity = step.angular_velocity;
        let omega_body = rotation
            .reverse()
            .rotate_vector(&bivector_to_angular_vector(&angular_velocity));
        let transverse = (omega_body[0].powi(2) + omega_body[2].powi(2)).sqrt();
        maximum_transverse = maximum_transverse.max(transverse);
    }

    let final_momentum =
        world_angular_momentum(&rotation, &angular_velocity, inertia).expect("finite momentum");
    let initial_momentum = world_angular_momentum(
        &Rotor::<3>::identity(),
        &angular_vector_to_bivector(&initial_body_omega),
        inertia,
    )
    .expect("finite initial momentum");
    let growth = maximum_transverse / initial_transverse;
    let mut failures = 0;
    failures += report_minimum("intermediate_axis_growth", "dt_1e-3_T_20", growth, 20.0);
    failures += report(
        "intermediate_axis_world_momentum_drift",
        "dt_1e-3_T_20",
        (final_momentum - initial_momentum).norm(),
        0.0,
        2.0e-9,
    );
    failures
}

fn off_center_impulse_energy_campaign() -> usize {
    let inertia = BivectorInertia::<3>::uniform_solid_ellipsoid(
        2.0,
        SVector::new(1.4, 0.9, 0.5),
    )
    .expect("valid ellipsoid");
    let offset = SVector::new(0.7, -0.25, 0.4);
    let impulse = SVector::new(-0.2, 0.9, 0.35).normalize();
    let angular_impulse = Bivector::from_wedge(&offset, &impulse);
    let angular_response = inertia
        .angular_velocity_from_momentum(&angular_impulse, 1.0e-12)
        .expect("finite angular impulse response");
    let point_velocity_change = -angular_response.apply_to_vector(&offset);
    let inverse_effective_mass = inertia
        .rotational_inverse_effective_mass(&offset, &impulse, 1.0e-12)
        .expect("finite contact effective mass");
    let rotational_energy = inertia
        .kinetic_energy(&angular_response)
        .expect("finite rotational energy");
    let half_impulse_work = 0.5 * impulse.dot(&point_velocity_change);
    let mut failures = 0;
    failures += report(
        "contact_effective_mass_response_error",
        "off_center_ellipsoid",
        (impulse.dot(&point_velocity_change) - inverse_effective_mass).abs(),
        0.0,
        2.0e-11,
    );
    failures += report(
        "contact_impulse_energy_identity_error",
        "off_center_ellipsoid",
        (rotational_energy - half_impulse_work).abs(),
        0.0,
        2.0e-11,
    );
    failures
}

fn bivector_roundtrip_campaign<const D: usize>(name: &str) -> usize {
    let semi_axes = SVector::<f64, D>::from_fn(|axis, _| 0.5 + axis as f64 * 0.2);
    let inertia =
        BivectorInertia::<D>::uniform_solid_ellipsoid(2.5, semi_axes).expect("valid ellipsoid");
    let mut angular_state = Bivector::<D>::zero();
    for i in 0..D {
        for j in (i + 1)..D {
            angular_state.set(i, j, ((i + 1) as f64 * (j + 2) as f64).sin() * 0.3);
        }
    }
    let momentum = inertia
        .angular_momentum(&angular_state)
        .expect("finite angular momentum");
    let recovered = inertia
        .angular_velocity_from_momentum(&momentum, 1.0e-12)
        .expect("finite inverse");
    report(
        "bivector_inertia_inverse_error",
        name,
        recovered.add(&angular_state.scale(-1.0)).norm(),
        0.0,
        2.0e-11,
    )
}

fn body_frame_contact_campaign() -> usize {
    let body_inertia = BivectorInertia::<3>::uniform_solid_ellipsoid(
        2.0,
        SVector::new(1.4, 0.9, 0.5),
    )
    .expect("valid body-frame ellipsoid");
    let mut generator = Bivector::<3>::zero();
    generator.set(0, 1, 0.31);
    generator.set(0, 2, -0.22);
    generator.set(1, 2, 0.43);
    let body_to_world = Rotor::from_bivector(&generator);
    let offset_body = SVector::new(0.7, -0.25, 0.4);
    let direction_body = SVector::new(-0.2, 0.9, 0.35).normalize();
    let offset_world = body_to_world.rotate_vector(&offset_body);
    let direction_world = body_to_world.rotate_vector(&direction_body);
    let body_result = body_inertia
        .rotational_inverse_effective_mass(&offset_body, &direction_body, 1.0e-12)
        .expect("body-frame contact response");
    let world_result = body_inertia
        .rotational_inverse_effective_mass_world(
            &body_to_world,
            &offset_world,
            &direction_world,
            1.0e-12,
        )
        .expect("world-frame contact response through body inertia");
    let explicit_world = body_inertia
        .rotated_by(&body_to_world)
        .expect("validated world inertia")
        .rotational_inverse_effective_mass(
            &offset_world,
            &direction_world,
            1.0e-12,
        )
        .expect("explicit world-frame contact response");
    let mut failures = 0;
    failures += report(
        "body_frame_contact_query_error",
        "rotated_anisotropic_ellipsoid",
        (world_result - body_result).abs(),
        0.0,
        2.0e-11,
    );
    failures += report(
        "body_frame_vs_explicit_world_contact_error",
        "rotated_anisotropic_ellipsoid",
        (world_result - explicit_world).abs(),
        0.0,
        2.0e-11,
    );
    failures
}

fn scale_safe_mass_properties_campaign() -> usize {
    let tiny_mass = 1.0e-310;
    let samples = [
        (SVector::<f64, 3>::new(-1.0e155, 0.0, 0.0), tiny_mass),
        (SVector::<f64, 3>::new(1.0e155, 0.0, 0.0), tiny_mass),
    ];
    let forward = BivectorInertia::<3>::from_point_masses(samples)
        .expect("large-offset tiny-mass point cloud");
    let reversed = BivectorInertia::<3>::from_point_masses(samples.into_iter().rev())
        .expect("reversed point cloud");
    let mut failures = 0;
    failures += report(
        "point_cloud_large_offset_tiny_mass_sxx",
        "mass_1e-310_offset_1e155",
        forward.second_moment()[(0, 0)],
        2.0,
        1.0e-10,
    );
    failures += report(
        "point_cloud_order_invariance",
        "forward_vs_reverse",
        (forward.second_moment() - reversed.second_moment()).norm(),
        0.0,
        0.0,
    );

    let underflow_samples = [
        (SVector::<f64, 3>::new(-1.0e-100, 0.0, 0.0), 1.0e-300),
        (SVector::<f64, 3>::new(1.0e-100, 0.0, 0.0), 1.0e-300),
    ];
    let rejected = BivectorInertia::<3>::from_point_masses(underflow_samples).is_err();
    failures += report(
        "point_cloud_underflow_rejected",
        "nonzero_mass_moment_underflows_f64",
        if rejected { 0.0 } else { 1.0 },
        0.0,
        0.0,
    );

    let component = BivectorInertia::<3>::uniform_solid_ball(tiny_mass, 1.0)
        .expect("tiny but positive component inertia");
    let compound = BivectorInertia::<3>::combine_at_common_center(
        tiny_mass,
        &component,
        &SVector::new(1.0e155, 0.0, 0.0),
        tiny_mass,
        &component,
        &SVector::new(-1.0e155, 0.0, 0.0),
    )
    .expect("scale-safe parallel-axis composition");
    failures += report(
        "parallel_axis_large_offset_tiny_mass_sxx",
        "mass_1e-310_offset_1e155",
        compound.second_moment()[(0, 0)],
        2.0,
        1.0e-10,
    );
    failures
}

fn mass_scaling_campaign() -> usize {
    let second_moment = SMatrix::<f64, 3, 3>::new(
        1.4, 0.12, -0.08,
        0.12, 2.2, 0.17,
        -0.08, 0.17, 3.1,
    );
    let base = BivectorInertia::<3>::from_second_moment(second_moment).expect("valid base tensor");
    let mut angular_state = Bivector::<3>::zero();
    angular_state.set(0, 1, 0.3);
    angular_state.set(0, 2, -0.7);
    angular_state.set(1, 2, 0.2);

    let mut failures = 0;
    for scale in [1.0e-12, 1.0e12] {
        let scaled =
            BivectorInertia::<3>::from_second_moment(second_moment * scale).expect("scaled tensor");
        let scaled_momentum = scaled
            .angular_momentum(&angular_state)
            .expect("finite scaled momentum");
        let recovered = scaled
            .angular_velocity_from_momentum(&scaled_momentum, 1.0e-12)
            .expect("finite scaled inverse");
        failures += report(
            "mass_scale_angular_state_error",
            if scale < 1.0 { "1e-12" } else { "1e12" },
            recovered.add(&angular_state.scale(-1.0)).norm(),
            0.0,
            2.0e-11,
        );
        let base_energy = base.kinetic_energy(&angular_state).expect("finite base energy");
        let scaled_energy = scaled.kinetic_energy(&angular_state).expect("finite scaled energy");
        failures += report(
            "mass_scale_relative_energy_error",
            if scale < 1.0 { "1e-12" } else { "1e12" },
            (scaled_energy / (base_energy * scale) - 1.0).abs(),
            0.0,
            2.0e-11,
        );
    }
    failures
}
