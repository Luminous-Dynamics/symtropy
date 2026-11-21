use nav_observation_toy_fim::correlated::CorrelatedRangeAction;
use nav_observation_toy_fim::correlation_envelope::{
    correlation_scenario_fisher, evaluate_correlation_envelope,
    robust_select_correlation_envelope, CorrelationScenario, PairwiseCorrelation,
};

fn pair(rho: f64) -> Vec<PairwiseCorrelation> {
    vec![PairwiseCorrelation {
        action_a: "receiver-a".to_owned(),
        action_b: "receiver-b".to_owned(),
        rho,
    }]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let same_bearing = vec![
        CorrelatedRangeAction::independent("receiver-a", 0.0, 1.0, 1),
        CorrelatedRangeAction::independent("receiver-b", 0.0, 1.0, 1),
    ];

    println!("SAME_BEARING_INFORMATION_VS_CORRELATION");
    for rho in [0.0, 0.5, 0.9] {
        let scenario = CorrelationScenario {
            scenario_id: format!("rho-{rho:.1}"),
            pairs: pair(rho),
        };
        let fisher = correlation_scenario_fisher(&same_bearing, &scenario)?;
        println!("  rho={rho:.1}, Jxx={:.9}, source_count=2", fisher.xx);
    }

    let orthogonal = vec![
        CorrelatedRangeAction::independent("receiver-a", 0.0, 1.0, 1),
        CorrelatedRangeAction::independent(
            "receiver-b",
            std::f64::consts::FRAC_PI_2,
            1.0,
            1,
        ),
    ];
    let scenarios = vec![
        CorrelationScenario { scenario_id: "independent".into(), pairs: pair(0.0) },
        CorrelationScenario { scenario_id: "moderate".into(), pairs: pair(0.5) },
        CorrelationScenario { scenario_id: "high".into(), pairs: pair(0.9) },
    ];
    let result = evaluate_correlation_envelope(&orthogonal, &scenarios)?;
    println!("WORST_CASE_CORRELATION_ENVELOPE");
    println!(
        "  scenario={}, lambda_min={:.9}, det={:.9}, rank={}",
        result.worst_case_scenario_id,
        result.worst_case_minimum_eigenvalue,
        result.worst_case_determinant,
        result.worst_case_rank
    );

    let candidate_actions = vec![
        CorrelatedRangeAction::independent("a-east", 0.0, 1.0, 1),
        CorrelatedRangeAction::independent("b-east-copy", 0.0, 1.0, 1),
        CorrelatedRangeAction::independent("c-north", std::f64::consts::FRAC_PI_2, 1.0, 1),
        CorrelatedRangeAction::independent("d-diagonal", std::f64::consts::FRAC_PI_4, 1.0, 1),
    ];
    let ids = ["a-east", "b-east-copy", "c-north", "d-diagonal"];
    let make_complete_scenario = |scenario_id: &str, correlated_pair: Option<(&str, &str, f64)>| {
        let mut pairs = Vec::new();
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let rho = correlated_pair
                    .filter(|(a, b, _)| {
                        (*a == ids[i] && *b == ids[j]) || (*a == ids[j] && *b == ids[i])
                    })
                    .map(|(_, _, value)| value)
                    .unwrap_or(0.0);
                pairs.push(PairwiseCorrelation {
                    action_a: ids[i].to_owned(),
                    action_b: ids[j].to_owned(),
                    rho,
                });
            }
        }
        CorrelationScenario {
            scenario_id: scenario_id.to_owned(),
            pairs,
        }
    };
    let candidate_scenarios = vec![
        make_complete_scenario("all-independent", None),
        make_complete_scenario("shared-clock", Some(("a-east", "b-east-copy", 0.9))),
    ];
    let selected =
        robust_select_correlation_envelope(&candidate_actions, &candidate_scenarios, 2)?;
    println!("ROBUST_SENSOR_SUBSET_UNDER_CORRELATION_UNCERTAINTY");
    println!(
        "  selected={:?}, cost={}, rank={}, worst_lambda_min={:.9}",
        selected.action_ids,
        selected.total_cost_units,
        selected.envelope.worst_case_rank,
        selected.envelope.worst_case_minimum_eigenvalue
    );
    Ok(())
}
