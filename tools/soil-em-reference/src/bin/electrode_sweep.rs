use std::error::Error;

use soil_em_reference::{
    Medium,
    electrode::{hemispherical_electrode_impedance, two_hemispherical_electrode_impedance_approx},
};

fn main() -> Result<(), Box<dyn Error>> {
    let conductivities = [0.001_f64, 0.01, 0.1, 1.0];
    let frequencies = [0.0_f64, 60.0, 1_000.0, 10_000.0];
    let radii = [0.05_f64, 0.1, 0.25];
    let separations = [2.0_f64, 5.0, 10.0, 25.0];

    println!("frequency_hz,conductivity_s_per_m,relative_permittivity,radius_1_m,radius_2_m,separation_m,single_electrode_re_ohms,single_electrode_im_ohms,pair_approx_re_ohms,pair_approx_im_ohms,geometry_ratio");

    for frequency in frequencies {
        for conductivity in conductivities {
            for radius_1 in radii {
                for radius_2 in radii {
                    for separation in separations {
                        let medium = Medium::new(10.0, 1.0, conductivity)?;
                        let single = hemispherical_electrode_impedance(frequency, radius_1, medium)?;
                        let pair = two_hemispherical_electrode_impedance_approx(
                            frequency, radius_1, radius_2, separation, medium,
                        );
                        let geometry_ratio = radius_1.max(radius_2) / separation;
                        match pair {
                            Ok(pair) => println!(
                                "{:.3},{:.6},10.0,{:.4},{:.4},{:.4},{:.12e},{:.12e},{:.12e},{:.12e},{:.6}",
                                frequency, conductivity, radius_1, radius_2, separation,
                                single.re_ohms, single.im_ohms, pair.re_ohms, pair.im_ohms, geometry_ratio,
                            ),
                            Err(_) => println!(
                                "{:.3},{:.6},10.0,{:.4},{:.4},{:.4},{:.12e},{:.12e},NaN,NaN,{:.6}",
                                frequency, conductivity, radius_1, radius_2, separation,
                                single.re_ohms, single.im_ohms, geometry_ratio,
                            ),
                        }
                    }
                }
            }
        }
    }

    Ok(())
}