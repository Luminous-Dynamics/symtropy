use symtropy_artificial_gravity::{
    angular_velocity_rad_s, radial_gravity_profile, radius_for_gravity_m,
    STANDARD_GRAVITY_M_S2,
};

fn band_name(target_g: f64) -> &'static str {
    if (target_g - 0.16).abs() < 1e-12 {
        "lunar"
    } else if (target_g - 0.38).abs() < 1e-12 {
        "mars"
    } else {
        "earth"
    }
}

fn main() -> Result<(), String> {
    let common_rpm = 1.6_f64;
    let gravity_bands = [0.16_f64, 0.38, 1.0];
    let movement_speeds_m_s = [0.0_f64, 0.5, 1.0, 1.5];
    let body_length_m = 2.0_f64;
    let omega = angular_velocity_rad_s(common_rpm)
        .map_err(|error| format!("angular velocity failed: {error}"))?;

    println!(
        "band,target_g,common_rpm,omega_rad_s,reference_radius_m,body_length_m,head_feet_gradient_fraction,transverse_speed_m_s,coriolis_magnitude_m_s2,coriolis_fraction_of_local_g"
    );

    for target_g in gravity_bands {
        let radius = radius_for_gravity_m(target_g, common_rpm)
            .map_err(|error| format!("band radius failed: {error}"))?;
        let profile = radial_gravity_profile(radius, body_length_m, common_rpm)
            .map_err(|error| format!("body profile failed: {error}"))?;

        for speed in movement_speeds_m_s {
            let coriolis_magnitude = 2.0 * omega * speed;
            let local_acceleration = target_g * STANDARD_GRAVITY_M_S2;
            let coriolis_fraction = coriolis_magnitude / local_acceleration;

            println!(
                "{},{target_g:.3},{common_rpm:.3},{omega:.12},{radius:.9},{body_length_m:.3},{:.9},{speed:.3},{coriolis_magnitude:.9},{coriolis_fraction:.9}",
                band_name(target_g),
                profile.head_to_feet_gradient_fraction
            );
        }
    }

    Ok(())
}
