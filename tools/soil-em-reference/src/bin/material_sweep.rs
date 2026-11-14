use std::error::Error;

use soil_em_reference::{Medium, good_conductor_skin_depth, propagation};

fn main() -> Result<(), Box<dyn Error>> {
    let frequencies_hz = [100.0_f64, 1_000.0, 10_000.0, 100_000.0];
    let conductivities_s_per_m = [0.001_f64, 0.01, 0.1, 1.0];
    let relative_permittivities = [5.0_f64, 10.0, 30.0];

    println!("frequency_hz,conductivity_s_per_m,relative_permittivity,relative_permeability,conduction_ratio,attenuation_np_per_m,attenuation_db_per_m,phase_constant_rad_per_m,wavelength_m,skin_depth_m,good_conductor_skin_depth_m,field_ratio_at_100m,power_ratio_at_100m");

    for frequency in frequencies_hz {
        for conductivity in conductivities_s_per_m {
            for epsilon_r in relative_permittivities {
                let medium = Medium::new(epsilon_r, 1.0, conductivity)?;
                let result = propagation(frequency, medium)?;
                let approximate = good_conductor_skin_depth(frequency, medium).ok();
                let exact_depth = result.skin_depth_m.unwrap_or(f64::INFINITY);
                println!(
                    "{:.3},{:.6},{:.3},1.0,{:.8},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e}",
                    frequency, conductivity, epsilon_r, result.conduction_ratio,
                    result.attenuation_np_per_m, result.attenuation_db_per_m,
                    result.phase_constant_rad_per_m, result.wavelength_m, exact_depth,
                    approximate.unwrap_or(f64::NAN),
                    result.amplitude_ratio(100.0)?, result.power_ratio(100.0)?,
                );
            }
        }
    }

    Ok(())
}