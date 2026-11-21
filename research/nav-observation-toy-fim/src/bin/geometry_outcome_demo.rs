use nav_observation_toy_fim::correlated::CorrelatedRangeAction;
use nav_observation_toy_fim::geometry_outcome::expected_two_observation_error;
use nav_observation_toy_fim::robust::{ActionPerturbation, PerturbationScenario};

fn p(id: &str, delta: f64) -> ActionPerturbation {
    ActionPerturbation {
        action_id: id.to_owned(),
        bearing_delta_rad: delta,
        private_sigma_scale: 1.0,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fragile = vec![
        CorrelatedRangeAction::independent("fragile-east", 0.0, 0.2, 1),
        CorrelatedRangeAction::independent("fragile-north", std::f64::consts::FRAC_PI_2, 0.2, 1),
    ];
    let stable = vec![
        CorrelatedRangeAction::independent("stable-diagonal", std::f64::consts::FRAC_PI_4, 0.205, 1),
        CorrelatedRangeAction::independent("stable-antidiagonal", 3.0 * std::f64::consts::FRAC_PI_4, 0.205, 1),
    ];
    let scenario = PerturbationScenario {
        scenario_id: "adverse-bearing-error".to_owned(),
        actions: vec![
            p("fragile-east", 0.35),
            p("fragile-north", -0.35),
            p("stable-diagonal", 0.01),
            p("stable-antidiagonal", -0.01),
        ],
    };
    let truth = [10.0, -5.0];
    let fragile_error = expected_two_observation_error(&fragile, &scenario, truth)?;
    let stable_error = expected_two_observation_error(&stable, &scenario, truth)?;
    println!("EXPECTED_POSITION_ERROR_UNDER_BEARING_MISMATCH");
    println!(
        "  fragile: bias_squared={:.6}, noise_variance={:.6}, expected_squared_error={:.6}",
        fragile_error.bias_squared_m2,
        fragile_error.noise_variance_m2,
        fragile_error.expected_squared_error_m2
    );
    println!(
        "  stable: bias_squared={:.6}, noise_variance={:.6}, expected_squared_error={:.6}",
        stable_error.bias_squared_m2,
        stable_error.noise_variance_m2,
        stable_error.expected_squared_error_m2
    );
    Ok(())
}
