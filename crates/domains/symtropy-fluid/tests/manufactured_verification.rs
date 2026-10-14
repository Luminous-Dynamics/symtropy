// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_fluid::manufactured::{
    ManufacturedTaylorGreenProfile, manufactured_acceleration_mps2,
};
use symtropy_fluid::reference::{PeriodicMac2d, PeriodicMacConfig};
use symtropy_fluid::validation::MAX_DIAGNOSTIC_PROFILE_BYTES;

fn config() -> PeriodicMacConfig {
    PeriodicMacConfig {
        nx: 24,
        ny: 24,
        length_x_m: std::f64::consts::TAU,
        length_y_m: std::f64::consts::TAU,
        slab_depth_m: 1.0,
        density_kg_m3: 1.0,
        kinematic_viscosity_m2_s: 0.01,
        pressure_iterations: 800,
        max_advective_cfl: 0.5,
        max_diffusion_number: 0.24,
    }
}

fn profile() -> ManufacturedTaylorGreenProfile {
    ManufacturedTaylorGreenProfile {
        base_amplitude_mps: 0.15,
        modulation_fraction: 0.3,
        angular_frequency_rad_s: 1.7,
    }
}

#[test]
fn manufactured_forcing_is_discretely_solenoidal_on_the_mac_grid() {
    let config = config();
    let dx = config.dx();
    let dy = config.dy();
    let time_s = 0.37;
    let mut u_faces = vec![0.0; config.nx * config.ny];
    let mut v_faces = vec![0.0; config.nx * config.ny];

    for j in 0..config.ny {
        for i in 0..config.nx {
            let index = j * config.nx + i;
            let u_position = [i as f64 * dx, (j as f64 + 0.5) * dy];
            let v_position = [(i as f64 + 0.5) * dx, j as f64 * dy];
            u_faces[index] =
                manufactured_acceleration_mps2(&config, profile(), u_position, time_s).unwrap()[0];
            v_faces[index] =
                manufactured_acceleration_mps2(&config, profile(), v_position, time_s).unwrap()[1];
        }
    }

    let sampled_forcing = PeriodicMac2d::from_faces(config, u_faces, v_faces).unwrap();
    assert!(sampled_forcing.divergence_rms_per_s() < 1.0e-13);
}

#[test]
fn manufactured_profile_identity_is_bounded_and_bit_sensitive() {
    let a = profile();
    assert!(a.profile_identity().len() <= MAX_DIAGNOSTIC_PROFILE_BYTES);

    let mut b = a;
    b.base_amplitude_mps = f64::from_bits(a.base_amplitude_mps.to_bits() + 1);
    assert_ne!(a.profile_identity(), b.profile_identity());
}

#[test]
fn manufactured_forcing_rejects_non_finite_phase_evaluation() {
    let config = config();
    let mut profile = profile();
    profile.angular_frequency_rad_s = f64::MAX;

    let result = manufactured_acceleration_mps2(&config, profile, [1.0, 1.0], f64::MAX);
    assert!(result.is_err());
}
