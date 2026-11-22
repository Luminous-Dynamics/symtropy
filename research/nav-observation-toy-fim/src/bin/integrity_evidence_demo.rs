use nav_observation_toy_fim::integrity_evidence::{
    minimum_zero_failure_trials, one_sided_binomial_upper_bound, qualify_zero_failure_evidence,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let target_risk = 1.0e-3;
    let confidence = 0.95;
    let required = minimum_zero_failure_trials(target_risk, confidence)?;
    let at_gate = qualify_zero_failure_evidence(0, required, target_risk, confidence)?;
    let below_gate = qualify_zero_failure_evidence(0, required - 1, target_risk, confidence)?;

    println!("target_risk={target_risk}");
    println!("one_sided_confidence={confidence}");
    println!("minimum_independent_zero_failure_missions={required}");
    println!("upper_bound_at_gate={:.12}", at_gate.one_sided_upper_bound);
    println!("target_met_at_gate={}", at_gate.target_met);
    println!("upper_bound_one_trial_below={:.12}", below_gate.one_sided_upper_bound);
    println!("target_met_one_trial_below={}", below_gate.target_met);
    let observed_failures = 1;
    let trials = 10;
    let nonzero_upper = one_sided_binomial_upper_bound(confidence, observed_failures, trials)?;
    println!("nonzero_example_failures={observed_failures}");
    println!("nonzero_example_independent_trials={trials}");
    println!("nonzero_example_one_sided_upper_bound={nonzero_upper:.12}");
    println!("scope=independent_exchangeable_predeclared_mission_trials_not_high_rate_samples_or_exposure_hours");
    Ok(())
}
