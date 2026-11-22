use nav_observation_toy_fim::adaptive_pl::{
    AdaptivePlConfig, AdaptivePlInput, AdaptivePlState, GnssBoundEvidence,
    geometric_sensitivity_per_m, project_directional_drift,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AdaptivePlConfig::paper_2026(3.0);
    let mut longitudinal = AdaptivePlState::new(config, 1.0, 1.0)?;
    let mut lateral = AdaptivePlState::new(config, 1.0, 1.0)?;

    let drift = project_directional_drift([0.9, 0.3], [0.7, 0.1], 0.0)?;
    let sensitivity = geometric_sensitivity_per_m(0.02, 1.0e-6)?;
    let sample = |directional_drift_m| AdaptivePlInput {
        delta_time_s: 0.1,
        directional_drift_m,
        geometric_residual_m: 0.08,
        geometric_sensitivity_per_m: sensitivity,
        normalized_environment_entropy: 0.7,
        gnss_bound: Some(GnssBoundEvidence {
            projected_innovation_m: 0.15,
            projected_sigma_m: 0.06,
        }),
    };

    for tick in 0..20 {
        let long = longitudinal.step(config, sample(drift.longitudinal_m))?;
        let lat = lateral.step(config, sample(drift.lateral_m))?;
        println!(
            "tick={tick} PL_long_m={:.6} PL_lat_m={:.6} entropy={:.3} gamma_long_mps={:.6} gamma_lat_mps={:.6}",
            long.protection_level_m,
            lat.protection_level_m,
            long.normalized_environment_entropy,
            long.instantaneous_error_rate_mps,
            lat.instantaneous_error_rate_mps
        );
    }
    println!("scope=equation_level_controller_harness_not_full_paper_reproduction_or_integrity_certificate");
    Ok(())
}
