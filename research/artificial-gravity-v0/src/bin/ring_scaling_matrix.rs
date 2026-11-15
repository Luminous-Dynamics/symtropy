use symtropy_artificial_gravity::{
    ring_scaling_for_fixed_line_density, ring_scaling_for_fixed_mass, RingScalingMetrics,
};

fn emit_row(
    model: &str,
    input_parameter: f64,
    target_g: f64,
    metrics: RingScalingMetrics,
) {
    println!(
        "{model},{input_parameter:.6},{target_g:.4},{:.6},{:.9},{:.9},{:.9},{:.9},{:.9}",
        metrics.radius_m,
        metrics.moving_mass_kg,
        metrics.angular_velocity_rad_s,
        metrics.tangential_speed_m_s,
        metrics.angular_momentum_kg_m2_s,
        metrics.rotational_energy_j,
        metrics.ideal_hoop_tension_n
    );
}

fn main() -> Result<(), String> {
    let radii_m = [50.0_f64, 100.0, 200.0, 350.0];
    let target_gravity_levels = [0.16_f64, 0.38, 0.5, 1.0];
    let fixed_masses_kg = [10_000.0_f64, 50_000.0];
    let fixed_line_densities_kg_m = [100.0_f64, 500.0];

    println!(
        "mass_basis,input_mass_or_line_density,target_g,radius_m,moving_mass_kg,omega_rad_s,tangential_speed_m_s,angular_momentum_kg_m2_s,rotational_energy_j,ideal_hoop_tension_n"
    );

    for target_g in target_gravity_levels {
        for radius in radii_m {
            for mass in fixed_masses_kg {
                let metrics = ring_scaling_for_fixed_mass(mass, radius, target_g)
                    .map_err(|error| format!("fixed-mass ring failed: {error}"))?;
                emit_row("fixed_total_mass", mass, target_g, metrics);
            }
            for density in fixed_line_densities_kg_m {
                let metrics = ring_scaling_for_fixed_line_density(density, radius, target_g)
                    .map_err(|error| format!("fixed-line-density ring failed: {error}"))?;
                emit_row("fixed_line_density", density, target_g, metrics);
            }
        }
    }

    Ok(())
}
