use nav_observation_toy_fim::decision_value::{
    select_action, ActionCost, BinaryBelief, DiagnosticAction, ObservationOutcome, ResourceBudget,
    SelectionCriterion,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let belief = BinaryBelief::new(0.9)?;
    let budget = ResourceBudget {
        energy_units: 2,
        latency_ticks: 2,
        bandwidth_units: 1,
    };
    let actions = vec![
        DiagnosticAction {
            id: "balanced_entropy".to_owned(),
            cost: ActionCost {
                energy_units: 1,
                latency_ticks: 1,
                bandwidth_units: 1,
            },
            acquisition_success_probability: 1.0,
            outcomes: vec![
                ObservationOutcome {
                    id: "supports_state_one".to_owned(),
                    likelihood_state_zero: 0.11,
                    likelihood_state_one: 0.89,
                },
                ObservationOutcome {
                    id: "supports_state_zero".to_owned(),
                    likelihood_state_zero: 0.89,
                    likelihood_state_one: 0.11,
                },
            ],
        },
        DiagnosticAction {
            id: "decision_relevant_alert".to_owned(),
            cost: ActionCost {
                energy_units: 2,
                latency_ticks: 2,
                bandwidth_units: 1,
            },
            acquisition_success_probability: 0.8,
            outcomes: vec![
                ObservationOutcome {
                    id: "adverse_alert".to_owned(),
                    likelihood_state_zero: 0.70,
                    likelihood_state_one: 0.04,
                },
                ObservationOutcome {
                    id: "no_alert".to_owned(),
                    likelihood_state_zero: 0.30,
                    likelihood_state_one: 0.96,
                },
            ],
        },
    ];

    println!("prior_state_one_probability={:.3}", belief.probability_state_one);
    println!("baseline_bayes_risk={:.6}", belief.bayes_risk());
    for criterion in [
        SelectionCriterion::ExpectedMutualInformation,
        SelectionCriterion::ExpectedDecisionValue,
    ] {
        let selected = select_action(&actions, belief, budget, criterion)?;
        match selected {
            Some(result) => println!(
                "criterion={criterion:?} selected={} expected_decision_value={:.6} expected_information_bits={:.6}",
                result.action_id,
                result.expected_decision_value,
                result.expected_mutual_information_bits
            ),
            None => println!("criterion={criterion:?} selected=none"),
        }
    }
    println!("scope=finite_one_step_synthetic_oracle_not_navigation_qualification");
    Ok(())
}
