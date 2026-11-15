use symtropy_artificial_gravity::{
    angular_velocity_rad_s, coriolis_apparent_acceleration, radial_gravity_profile,
    radius_for_gravity_m, vector_magnitude, STANDARD_GRAVITY_M_S2,
};

fn main() -> Result<(), String> {
    let gravity_levels = [0.16_f64, 0.38, 1.0];
    let rotation_rates_rpm = [1.0_f64, 1.6, 2.0, 3.0, 3.5, 4.0];
    let movement_speeds_m_s = [0.0_f64, 0.5, 1.0, 1.5];
    let body_length_m = 2.0_f64;

    println!(
        "target_g,rpm,reference_radius_m,body_length_m,head_feet_gradient_fraction,speed_m_s,direction,coriolis_x_m_s2,coriolis_y_m_s2,coriolis_z_m_s2,coriolis_magnitude_m_s2,coriolis_fraction_of_target_g"
    );

    for target_g in gravity_levels {
        for rpm in rotation_rates_rpm {
            let radius = radius_for_gravity_m(target_g, rpm)
                .map_err(|error| format!("radius calculation failed: {error}"))?;
            let omega = angular_velocity_rad_s(rpm)
                .map_err(|error| format!("angular velocity failed: {error}"))?;
            let body_profile = radial_gravity_profile(radius, body_length_m, rpm)
                .map_err(|error| format!("body profile failed: {error}"))?;

            let directions = [
                ("radial_outward", [1.0_f64, 0.0, 0.0]),
                ("prograde_tangential", [0.0_f64, 1.0, 0.0]),
                ("retrograde_tangential", [0.0_f64, -1.0, 0.0]),
                ("axial", [0.0_f64, 0.0, 1.0]),
            ];

            for speed in movement_speeds_m_s {
                for (direction, unit_velocity) in directions {
                    let velocity = [
                        unit_velocity[0] * speed,
                        unit_velocity[1] * speed,
                        unit_velocity[2] * speed,
                    ];
                    let coriolis = coriolis_apparent_acceleration(
                        [0.0, 0.0, omega],
                        velocity,
                    )
                    .map_err(|error| format!("Coriolis calculation failed: {error}"))?;
                    let magnitude = vector_magnitude(coriolis);
                    let fraction_of_target_g =
                        magnitude / (target_g * STANDARD_GRAVITY_M_S2);

                    println!(
                        "{target_g:.2},{rpm:.1},{radius:.6},{body_length_m:.3},{:.9},{speed:.2},{direction},{:.9},{:.9},{:.9},{:.9},{:.9}",
                        body_profile.head_to_feet_gradient_fraction,
                        coriolis[0],
                        coriolis[1],
                        coriolis[2],
                        magnitude,
                        fraction_of_target_g
                    );
                }
            }
        }
    }

    Ok(())
}
