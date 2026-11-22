use nav_observation_toy_fim::decision_value::{
    AcquisitionSuccessModel, ActionCost, BinaryBelief, DecisionLoss, DiagnosticAction,
    ObservationOutcome, ResourceBudget,
};
use nav_observation_toy_fim::sequential_value::{
    exact_horizon_two_plan, greedy_first_horizon_two_plan, SequentialCandidate,
};

fn candidate(
    id: &str,
    cost: ActionCost,
    p_positive_state_zero: f64,
    p_positive_state_one: f64,
    success: f64,
) -> SequentialCandidate {
    SequentialCandidate {
        action: DiagnosticAction {
            id: id.to_owned(),
            cost,
            acquisition_success_probability: success,
            outcomes: vec![
                ObservationOutcome {
                    id: "positive".to_owned(),
                    likelihood_state_zero: p_positive_state_zero,
                    likelihood_state_one: p_positive_state_one,
                },
                ObservationOutcome {
                    id: "negative".to_owned(),
                    likelihood_state_zero: 1.0 - p_positive_state_zero,
                    likelihood_state_one: 1.0 - p_positive_state_one,
                },
            ],
        },
        acquisition: AcquisitionSuccessModel::state_independent(success),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let one = ActionCost { energy_units: 1, latency_ticks: 1, bandwidth_units: 1 };
    let two = ActionCost { energy_units: 2, latency_ticks: 2, bandwidth_units: 1 };
    let candidates = vec![
        candidate("a-balanced-first", one, 0.11, 0.89, 1.0),
        candidate("b-local-follow-up", one, 0.20, 0.80, 1.0),
        candidate("c-high-one-step-value", two, 0.70, 0.04, 0.8),
    ];
    let belief = BinaryBelief::new(0.9)?;
    let budget = ResourceBudget { energy_units: 2, latency_ticks: 2, bandwidth_units: 1 };
    let loss = DecisionLoss::zero_one();

    let exact = exact_horizon_two_plan(&candidates, belief, budget, loss)?
        .ok_or("no positive-value contingent plan")?;
    let greedy = greedy_first_horizon_two_plan(&candidates, belief, budget, loss)?
        .ok_or("no positive-value greedy plan")?;

    println!("exact_first_action={}", exact.first_action_id);
    println!("exact_expected_terminal_risk={:.6}", exact.expected_terminal_bayes_risk);
    println!("exact_expected_decision_value={:.6}", exact.expected_decision_value);
    for branch in &exact.branches {
        println!(
            "branch={} probability={:.6} posterior_state_one={:.6} second_action={}",
            branch.branch_id,
            branch.branch_probability,
            branch.posterior_state_one_probability,
            branch.second_action_id.as_deref().unwrap_or("stop")
        );
    }
    println!("greedy_first_action={}", greedy.first_action_id);
    println!("greedy_expected_terminal_risk={:.6}", greedy.expected_terminal_bayes_risk);
    println!("greedy_expected_decision_value={:.6}", greedy.expected_decision_value);
    println!("scope=finite_binary_horizon_two_synthetic_oracle_not_navigation_qualification");
    Ok(())
}
