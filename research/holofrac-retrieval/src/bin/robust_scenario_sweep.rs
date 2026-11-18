use std::env;

use holofrac_spatial_retrieval::robust_scenarios::{
    synthetic_robust_scenario_problem, ScenarioWeightBasis,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_sha = env::var("HOLOFRAC_SOURCE_SHA").unwrap_or_else(|_| "UNSET".to_owned());
    println!("# HoloFrac cross-scenario robustness benchmark v0");
    println!("# source_sha={source_sha}");
    println!("# target_arch={}", env::consts::ARCH);
    println!("# target_os={}", env::consts::OS);
    println!("row_type,scenario_weight_basis,id,metric,value,detail");

    let problem = synthetic_robust_scenario_problem(ScenarioWeightBasis::Analytical);
    let report = problem.evaluate()?;

    println!(
        "summary,{basis},,scenario_count,{},",
        problem.scenarios.len(),
        basis = report.scenario_weight_basis.as_str()
    );
    println!(
        "summary,{basis},,candidate_count,{},",
        problem.candidates.len(),
        basis = report.scenario_weight_basis.as_str()
    );
    println!(
        "summary,{basis},,universal_pareto_candidates,{},",
        report.universal_pareto_candidate_ids.join(";"),
        basis = report.scenario_weight_basis.as_str()
    );
    println!(
        "summary,{basis},,robust_worst_case_frontier,{},",
        report.robust_worst_case_frontier_ids.join(";"),
        basis = report.scenario_weight_basis.as_str()
    );
    println!(
        "summary,{basis},,weighted_mean_frontier,{},",
        report.weighted_mean_frontier_ids.join(";"),
        basis = report.scenario_weight_basis.as_str()
    );

    for frontier in &report.scenario_frontiers {
        println!(
            "scenario_frontier,{basis},{scenario},candidate_ids,{},",
            frontier.candidate_ids.join(";"),
            basis = report.scenario_weight_basis.as_str(),
            scenario = frontier.scenario_id
        );
    }

    for candidate in &report.candidates {
        println!(
            "candidate,{basis},{id},weighted_mean_utility,{mean:.8},",
            basis = report.scenario_weight_basis.as_str(),
            id = candidate.candidate_id,
            mean = candidate.weighted_mean_utility
        );
        println!(
            "candidate,{basis},{id},worst_case_utility,{worst:.8},",
            basis = report.scenario_weight_basis.as_str(),
            id = candidate.candidate_id,
            worst = candidate.worst_case_utility
        );
        println!(
            "candidate,{basis},{id},feasible_scenario_count,{},",
            candidate.feasible_scenario_count,
            basis = report.scenario_weight_basis.as_str(),
            id = candidate.candidate_id
        );
        println!(
            "candidate,{basis},{id},weighted_feasible_share,{share:.8},",
            basis = report.scenario_weight_basis.as_str(),
            id = candidate.candidate_id,
            share = candidate.weighted_feasible_share
        );
        println!(
            "candidate,{basis},{id},guaranteed_feasible,{},",
            candidate.guaranteed_feasible,
            basis = report.scenario_weight_basis.as_str(),
            id = candidate.candidate_id
        );
        println!(
            "candidate,{basis},{id},scenario_frontier_membership,{},",
            candidate.scenario_frontier_membership_count,
            basis = report.scenario_weight_basis.as_str(),
            id = candidate.candidate_id
        );

        if let Some(probability) = candidate.probability_of_feasibility {
            println!(
                "candidate,{basis},{id},probability_of_feasibility,{probability:.8},",
                basis = report.scenario_weight_basis.as_str(),
                id = candidate.candidate_id
            );
        }

        if let Some(regret) = candidate.weighted_regret {
            println!(
                "candidate,{basis},{id},weighted_regret,{regret:.8},",
                basis = report.scenario_weight_basis.as_str(),
                id = candidate.candidate_id
            );
        }
        if let Some(regret) = candidate.maximum_regret {
            println!(
                "candidate,{basis},{id},maximum_regret,{regret:.8},",
                basis = report.scenario_weight_basis.as_str(),
                id = candidate.candidate_id
            );
        }

        for scenario_regret in &candidate.scenario_regrets {
            let value = scenario_regret
                .regret
                .map(|regret| format!("{regret:.8}"))
                .unwrap_or_else(|| "undefined".to_owned());
            let detail = if scenario_regret.candidate_feasible {
                "feasible"
            } else {
                "infeasible"
            };
            println!(
                "scenario_regret,{basis},{id},{scenario},{value},{detail}",
                basis = report.scenario_weight_basis.as_str(),
                id = candidate.candidate_id,
                scenario = scenario_regret.scenario_id
            );
        }
    }

    println!(
        "ranking,{basis},,maximin_guaranteed_feasible,{},",
        report.rank_guaranteed_by_maximin().join(";"),
        basis = report.scenario_weight_basis.as_str()
    );
    println!(
        "ranking,{basis},,minimax_regret_guaranteed_feasible,{},",
        report.rank_guaranteed_by_minimax_regret().join(";"),
        basis = report.scenario_weight_basis.as_str()
    );
    println!(
        "ranking,{basis},,weighted_mean_guaranteed_feasible,{},",
        report.rank_guaranteed_by_weighted_mean().join(";"),
        basis = report.scenario_weight_basis.as_str()
    );

    Ok(())
}
