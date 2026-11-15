use symtropy_artificial_gravity::{
    angular_velocity_rad_s, radial_gravity_profile, radius_for_gravity_m,
    rotating_frame_apparent_acceleration, STANDARD_GRAVITY_M_S2,
};

fn main() -> Result<(), String> {
    let gravity_levels = [0.16_f64, 0.38, 1.0];
    let rotation_rates_rpm = [1.0_f64, 1.6, 2.0, 3.0, 3.5, 4.0];
    let angular_accelerations_rad_s2 = [0.0_f64, 1.0e-5, 5.0e-5];
    let movement_speeds_m_s = [0.0_f64, 0.5, 1.0, 1.5];
    let body_length_m = 2.0_f64;

    println!(
        "target_g,rpm,reference_radius_m,body_length_m,head_feet_gradient_fraction,alpha_z_rad_s2,speed_m_s,direction,centrifugal_x_m_s2,centrifugal_y_m_s2,centrifugal_z_m_s2,coriolis_x_m_s2,coriolis_y_m_s2,coriolis_z_m_s2,euler_x_m_s2,euler_y_m_s2,euler_z_m_s2,total_x_m_s2,total_y_m_s2,total_z_m_s2,motion_disturbance_magnitude_m_s2,motion_disturbance_fraction_of_target_g,total_apparent_magnitude_m_s2"
    );

    // Headings are defined in the local right-handed rotor frame.
    let directions = [
        ("radial_outward", [1.0_f64, 0.0, 0.0]),
        ("radial_inward", [-1.0_f64, 0.0, 0.0]),
        ("prograde_tangential", [0.0_f64, 1.0, 0.0]),
        ("retrograde_tangential", [0.0_f64, -1.0, 0.0]),
        ("axial", [0.0_f64, 0.0, 1.0]),
    ];

    for target_g in gravity_levels {
        for rpm in rotation_rates_rpm {
            let radius = radius_for_gravity_m(target_g, rpm)
                .map_err(|error| format!("radius calculation failed: {error}"))?;
            let omega = angular_velocity_rad_s(rpm)
                .map_err(|error| format!("angular velocity failed: {error}"))?;
            let body_profile = radial_gravity_profile(radius, body_length_m, rpm)
                .map_err(|error| format!("body profile failed: {error}"))?;
            let position_m = [radius, 0.0, 0.0];

            for alpha_z in angular_accelerations_rad_s2 {
                for speed in movement_speeds_m_s {
                    for (direction, unit_velocity) in directions {
                        let velocity = [
                            unit_velocity[0] * speed,
                            unit_velocity[1] * speed,
                            unit_velocity[2] * speed,
                        ];
                        let frame = rotating_frame_apparent_acceleration(
                            [0.0, 0.0, omega],
                            [0.0, 0.0, alpha_z],
                            position_m,
                            velocity,
                        )
                        .map_err(|error| format!("rotating-frame calculation failed: {error}"))?;

                        let motion_fraction = frame.motion_disturbance_magnitude_m_s2
                            / (target_g * STANDARD_GRAVITY_M_S2);

                        println!(
                            "{target_g:.2},{rpm:.1},{radius:.6},{body_length_m:.3},{:.9},{alpha_z:.9},{speed:.2},{direction},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{motion_fraction:.9},{:.9}",
                            body_profile.head_to_feet_gradient_fraction,
                            frame.centrifugal_m_s2[0],
                            frame.centrifugal_m_s2[1],
                            frame.centrifugal_m_s2[2],
                            frame.coriolis_m_s2[0],
                            frame.coriolis_m_s2[1],
                            frame.coriolis_m_s2[2],
                            frame.euler_m_s2[0],
                            frame.euler_m_s2[1],
                            frame.euler_m_s2[2],
                            frame.apparent_total_m_s2[0],
                            frame.apparent_total_m_s2[1],
                            frame.apparent_total_m_s2[2],
                            frame.motion_disturbance_magnitude_m_s2,
                            frame.apparent_total_magnitude_m_s2
                        );
                    }
                }
            }
        }
    }

    Ok(())
}
