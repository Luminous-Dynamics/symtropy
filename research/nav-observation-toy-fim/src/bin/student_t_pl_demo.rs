use nav_observation_toy_fim::student_t_pl::{
    student_t_k_tir_2d, student_t_protection_levels_for_tir, Covariance2,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Explicit anisotropic 2D covariance in m^2. This is a numeric formula
    // example only, not an UrbanNav covariance sample.
    let covariance = Covariance2 { xx: 9.0, xy: 0.0, yy: 1.0 };
    let tir = 1.0e-3;
    println!("covariance=[9,0;0,1] m2 target_2d_radial_tir={tir}");
    for dof in [4.0_f64, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 100.0] {
        let k = student_t_k_tir_2d(dof, tir)?;
        let pl = student_t_protection_levels_for_tir(covariance, dof, tir)?;
        println!(
            "dof={dof:.0} K_TIR_2D={k:.9} PL_overall_m={:.9} PL_longitudinal_m={:.9} PL_lateral_m={:.9}",
            pl.overall_pl_m,
            pl.longitudinal_pl_m,
            pl.lateral_pl_m
        );
    }
    println!("scope=equation_level_student_t_covariance_baseline_not_estimator_or_urban_nav_result");
    Ok(())
}
