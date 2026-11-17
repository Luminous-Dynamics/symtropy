use human_multirate_cosim::{
    AdaptiveConfig, Method, Scenario, simulate, simulate_adaptive_conservative_flux,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scenarios = [
        Scenario {
            name: "balanced",
            capacity_a: 1.0,
            capacity_b: 1.0,
            conductance: 0.5,
            initial_a: 1.0,
            initial_b: 0.0,
            horizon: 20.0,
        },
        Scenario {
            name: "asymmetric",
            capacity_a: 1.0,
            capacity_b: 4.0,
            conductance: 0.5,
            initial_a: 1.0,
            initial_b: 0.0,
            horizon: 40.0,
        },
        Scenario {
            name: "stiff",
            capacity_a: 0.2,
            capacity_b: 5.0,
            conductance: 0.8,
            initial_a: 1.0,
            initial_b: 0.0,
            horizon: 10.0,
        },
    ];

    let step_sets: [&[f64]; 3] = [
        &[0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 4.0, 10.0, 20.0],
        &[0.125, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 10.0, 20.0, 40.0],
        &[0.01, 0.025, 0.05, 0.1, 0.2, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0],
    ];

    println!(
        "scenario,method,step,steps,coupling_rate_step,max_abs_error,rms_error,balance_residual,min_state,max_state,positive,final_a,final_b"
    );

    for (scenario, steps) in scenarios.into_iter().zip(step_sets) {
        for step in steps {
            for method in Method::ALL {
                let score = simulate(scenario, method, *step)?;
                println!(
                    "{},{},{:.10},{},{:.10},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{},{:.12e},{:.12e}",
                    scenario.name,
                    method.as_str(),
                    step,
                    score.steps,
                    score.explicit_euler_stability_number,
                    score.max_abs_state_error,
                    score.rms_state_error,
                    score.max_abs_balance_residual,
                    score.min_state,
                    score.max_state,
                    score.positive_states,
                    score.final_a,
                    score.final_b,
                );
            }
        }

        let adaptive_config = AdaptiveConfig {
            initial_step: scenario.horizon,
            min_step: 1.0e-8,
            max_step: scenario.horizon,
            absolute_tolerance: 1.0e-7,
            relative_tolerance: 1.0e-6,
            max_attempts: 100_000,
        };
        let adaptive = simulate_adaptive_conservative_flux(scenario, adaptive_config)?;
        println!(
            "{},adaptive_conservative_shared_flux,{:.10},{},{:.10},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{},{:.12e},{:.12e}",
            scenario.name,
            adaptive_config.initial_step,
            adaptive.accepted_steps,
            scenario.coupling_rate() * adaptive_config.initial_step,
            adaptive.max_abs_state_error,
            adaptive.rms_state_error,
            adaptive.max_abs_balance_residual,
            adaptive.min_state,
            adaptive.max_state,
            adaptive.positive_states,
            adaptive.min_state,
            adaptive.max_state,
        );
    }

    Ok(())
}
