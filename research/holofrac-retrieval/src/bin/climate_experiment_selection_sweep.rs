use std::env;

use holofrac_spatial_retrieval::climate_experiment_selection::synthetic_climate_voi_request;
use holofrac_spatial_retrieval::experiment_selection::ExperimentSelectionPolicy;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_sha = env::var("HOLOFRAC_SOURCE_SHA").unwrap_or_else(|_| "UNSET".to_owned());
    println!("# HoloFrac climate experiment selection and value-of-information v0");
    println!("# source_sha={source_sha}");
    println!("# target_arch={}", env::consts::ARCH);
    println!("# target_os={}", env::consts::OS);

    let request = synthetic_climate_voi_request();
    let probability_basis_id = request
        .scenarios
        .semantics
        .probability_basis()
        .unwrap_or("UNAVAILABLE");
    println!("# probability_basis_id={probability_basis_id}");
    println!("# decision_objective_id={}", request.decision_objective_id);
    println!("# decision_authority_id={}", request.decision_authority_id);
    println!("# experiment_budget={:.8}", request.experiment_budget);
    println!(
        "row_type,policy,experiment,metric,value,detail"
    );

    for policy in ExperimentSelectionPolicy::ALL {
        let report = request.evaluate_policy(policy, 17)?;
        println!(
            "policy,{},{},selected_experiment,{},decision_without_measurement={}",
            policy.as_str(),
            report
                .policy_outcome
                .selected_experiment_id
                .as_deref()
                .unwrap_or("none"),
            report
                .policy_outcome
                .selected_experiment_id
                .as_deref()
                .unwrap_or("none"),
            report.policy_outcome.decision_without_measurement
        );
        println!(
            "policy,{},{},mutual_information_bits,{:.8},",
            policy.as_str(),
            report
                .policy_outcome
                .selected_experiment_id
                .as_deref()
                .unwrap_or("none"),
            report.policy_outcome.mutual_information_bits
        );
        println!(
            "policy,{},{},net_expected_decision_value,{:.8},",
            policy.as_str(),
            report
                .policy_outcome
                .selected_experiment_id
                .as_deref()
                .unwrap_or("none"),
            report.policy_outcome.net_expected_decision_value
        );
        println!(
            "policy,{},{},expected_decision_regret,{:.8},",
            policy.as_str(),
            report
                .policy_outcome
                .selected_experiment_id
                .as_deref()
                .unwrap_or("none"),
            report.policy_outcome.expected_decision_regret
        );

        for metrics in &report.experiment_metrics {
            println!(
                "experiment,{},{},mutual_information_bits,{:.8},",
                policy.as_str(),
                metrics.experiment_id,
                metrics.mutual_information_bits
            );
            println!(
                "experiment,{},{},expected_value_improvement,{:.8},",
                policy.as_str(),
                metrics.experiment_id,
                metrics.expected_value_improvement
            );
            println!(
                "experiment,{},{},net_expected_decision_value,{:.8},",
                policy.as_str(),
                metrics.experiment_id,
                metrics.net_expected_decision_value
            );
            println!(
                "experiment,{},{},expected_decision_regret,{:.8},",
                policy.as_str(),
                metrics.experiment_id,
                metrics.expected_decision_regret
            );
        }
    }

    Ok(())
}
