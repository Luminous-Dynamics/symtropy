use symtropy_artificial_gravity::{
    minimum_radius_for_design_envelope, ActiveRadiusConstraint,
};

fn constraint_name(value: ActiveRadiusConstraint) -> &'static str {
    match value {
        ActiveRadiusConstraint::BodyGradient => "body_gradient",
        ActiveRadiusConstraint::MaximumRotationRate => "maximum_rpm",
        ActiveRadiusConstraint::CoriolisMotionEnvelope => "coriolis_motion",
    }
}

fn main() -> Result<(), String> {
    let gravity_levels = [0.16_f64, 0.38, 0.5, 1.0];
    let body_length_m = 1.75_f64;
    let gradient_limits = [0.01_f64, 0.02, 0.03];
    let maximum_rpm_values = [1.6_f64, 2.0, 3.5];
    let coriolis_design_fractions = [0.05_f64, 0.10];
    let transverse_speeds_m_s = [0.0_f64, 0.5, 1.0, 1.5];

    println!(
        "target_g,body_length_m,max_gradient_fraction,max_rpm,max_coriolis_fraction,transverse_speed_m_s,gradient_min_radius_m,rpm_min_radius_m,coriolis_min_radius_m,minimum_radius_m,active_constraint,resulting_rpm,resulting_gradient_fraction,resulting_coriolis_fraction"
    );

    for target_g in gravity_levels {
        for gradient_limit in gradient_limits {
            for max_rpm in maximum_rpm_values {
                for coriolis_limit in coriolis_design_fractions {
                    for transverse_speed in transverse_speeds_m_s {
                        let result = minimum_radius_for_design_envelope(
                            target_g,
                            body_length_m,
                            gradient_limit,
                            max_rpm,
                            coriolis_limit,
                            transverse_speed,
                        )
                        .map_err(|error| format!("design envelope failed: {error}"))?;

                        println!(
                            "{target_g:.3},{body_length_m:.3},{gradient_limit:.4},{max_rpm:.3},{coriolis_limit:.4},{transverse_speed:.3},{:.9},{:.9},{:.9},{:.9},{},{:.9},{:.9},{:.9}",
                            result.gradient_min_radius_m,
                            result.rpm_min_radius_m,
                            result.coriolis_min_radius_m,
                            result.minimum_radius_m,
                            constraint_name(result.active_constraint),
                            result.resulting_rpm,
                            result.resulting_body_gradient_fraction,
                            result.resulting_coriolis_fraction
                        );
                    }
                }
            }
        }
    }

    Ok(())
}
