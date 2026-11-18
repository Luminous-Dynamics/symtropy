use std::env;

use holofrac_spatial_retrieval::climate_robustness::{
    evaluate_climate_robustness, ClimateCandidate, ClimateScenario, ClimateScenarioOutcome,
    ClimateScenarioSet, ScenarioWeightSemantics, ClimateCriterion,
};

fn candidate(
    id: &str,
    present: [f64; 3],
    future: [f64; 3],
    compound: [f64; 3],
    feasibility: [bool; 3],
) -> ClimateCandidate {
    ClimateCandidate {
        candidate_id: id.to_owned(),
        scenario_outcomes: vec![
            ClimateScenarioOutcome {
                scenario_id: "present_tmy".to_owned(),
                outcomes: present,
                feasible: feasibility[0],
            },
            ClimateScenarioOutcome {
                scenario_id: "future_hot".to_owned(),
                outcomes: future,
                feasible: feasibility[1],
            },
            ClimateScenarioOutcome {
                scenario_id: "compound_heat_grid".to_owned(),
                outcomes: compound,
                feasible: feasibility[2],
            },
        ],
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_sha = env::var("HOLOFRAC_SOURCE_SHA").unwrap_or_else(|_| "UNSET".to_owned());
    println!("# HoloFrac cross-scenario climate robustness benchmark v0");
    println!("# source_sha={source_sha}");
    println!("# target_arch={}", env::consts::ARCH);
    println!("# target_os={}", env::consts::OS);
    println!("# scenario_weight_semantics=analytical_decision_weights");
    println!(
        "record_type,scenario,candidate,metric,value,detail"
    );

    let scenarios = ClimateScenarioSet {
        semantics: ScenarioWeightSemantics::AnalyticalDecisionWeights,
        scenarios: vec![
            ClimateScenario {
                scenario_id: "present_tmy".to_owned(),
                decision_weight: 0.5,
            },
            ClimateScenario {
                scenario_id: "future_hot".to_owned(),
                decision_weight: 0.3,
            },
            ClimateScenario {
                scenario_id: "compound_heat_grid".to_owned(),
                decision_weight: 0.2,
            },
        ],
    };

    let candidates = vec![
        candidate(
            "present_specialist",
            [0.96, 0.91, 0.72],
            [0.58, 0.52, 0.68],
            [0.42, 0.36, 0.61],
            [true, false, false],
        ),
        candidate(
            "balanced",
            [0.80, 0.78, 0.80],
            [0.76, 0.73, 0.78],
            [0.70, 0.66, 0.73],
            [true, true, true],
        ),
        candidate(
            "future_specialist",
            [0.69, 0.72, 0.66],
            [0.94, 0.88, 0.72],
            [0.88, 0.82, 0.66],
            [true, true, true],
        ),
        candidate(
            "low_cost_brittle",
            [0.76, 0.62, 0.98],
            [0.34, 0.30, 0.94],
            [0.24, 0.20, 0.90],
            [true, true, true],
        ),
        candidate(
            "stable_moderate",
            [0.76, 0.75, 0.74],
            [0.74, 0.72, 0.72],
            [0.72, 0.70, 0.71],
            [true, true, true],
        ),
    ];

    let report = evaluate_climate_robustness(&scenarios, &candidates)?;

    for frontier in &report.scenario_frontiers {
        println!(
            "scenario,{},{},feasible_candidate_count,{},",
            frontier.scenario_id,
            "",
            frontier.feasible_candidate_count
        );
        for candidate_id in &frontier.frontier_candidate_ids {
            println!(
                "scenario_frontier,{},{},member,true,",
                frontier.scenario_id,
                candidate_id
            );
        }
        for (index, criterion) in ClimateCriterion::ALL.iter().enumerate() {
            println!(
                "scenario_ideal,{},{},ideal_{},{:.8},",
                frontier.scenario_id,
                "",
                criterion.as_str(),
                frontier.ideal_outcomes[index]
            );
        }
    }

    for candidate_id in &report.union_frontier_candidate_ids {
        println!("union_frontier,,{},member,true,", candidate_id);
    }
    for candidate_id in &report.universal_pareto_candidate_ids {
        println!("universal_pareto,,{},member,true,", candidate_id);
    }
    for candidate_id in &report.robust_frontier_candidate_ids {
        println!("robust_frontier,,{},member,true,", candidate_id);
    }
    println!(
        "summary,,mean_pairwise_frontier_jaccard,{:.8},",
        report.mean_pairwise_frontier_jaccard
    );

    for row in &report.candidate_reports {
        println!(
            "candidate,,{},weighted_feasibility_coverage,{:.8},",
            row.candidate_id,
            row.weighted_feasibility_coverage
        );
        println!(
            "candidate,,{},guaranteed_feasible,{},",
            row.candidate_id,
            row.guaranteed_feasible
        );
        println!(
            "candidate,,{},scenario_frontier_membership_count,{},",
            row.candidate_id,
            row.scenario_frontier_membership_count
        );
        println!(
            "candidate,,{},universal_pareto_member,{},",
            row.candidate_id,
            row.universal_pareto_member
        );
        println!(
            "candidate,,{},maximum_regret,{:.8},",
            row.candidate_id,
            row.maximum_regret
        );

        for (index, criterion) in ClimateCriterion::ALL.iter().enumerate() {
            println!(
                "candidate,,{},weighted_mean_{},{:.8},",
                row.candidate_id,
                criterion.as_str(),
                row.weighted_mean_outcomes[index]
            );
            println!(
                "candidate,,{},worst_case_{},{:.8},",
                row.candidate_id,
                criterion.as_str(),
                row.worst_case_outcomes[index]
            );
            println!(
                "candidate,,{},weighted_regret_{},{:.8},",
                row.candidate_id,
                criterion.as_str(),
                row.weighted_regret_by_criterion[index]
            );
        }
    }

    Ok(())
}
