use nav_observation_toy_fim::correlated::CorrelatedRangeAction;
use nav_observation_toy_fim::correlation_envelope::{
    correlation_scenario_fisher, evaluate_correlation_envelope, CorrelationScenario,
    PairwiseCorrelation,
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
    Ok(())
}
