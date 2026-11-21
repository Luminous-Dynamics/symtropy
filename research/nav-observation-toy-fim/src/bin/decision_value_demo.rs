use nav_observation_toy_fim::decision_value::{
    evaluate_action_with_acquisition_model, select_action, select_action_with_loss,
    AcquisitionSuccessModel, ActionCost, BinaryBelief, DecisionLoss, DiagnosticAction,
    ObservationOutcome, ResourceBudget, SelectionCriterion,
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
    // Illustrate that a false-safe claim can carry a larger cost than
    // unnecessary degradation. This changes the decision-value ranking while
    // leaving the mutual-information metric unchanged.
    let asymmetric_loss = DecisionLoss::new(10.0, 1.0)?;
    let asymmetric = select_action_with_loss(
        &actions,
        belief,
        budget,
        SelectionCriterion::ExpectedDecisionValue,
        asymmetric_loss,
    )?;
    if let Some(result) = asymmetric {
        println!(
            "criterion=AsymmetricDecisionValue(false_safe=10,false_degraded=1) selected={} expected_decision_value={:.6} expected_information_bits={:.6}",
            result.action_id,
            result.expected_decision_value,
            result.expected_mutual_information_bits
        );
    }
    // A deliberately uninformative measurement can still be preceded by
    // an informative acquisition failure when dropout is state-dependent.
    let missingness_action = DiagnosticAction {
        id: "state_dependent_dropout".to_owned(),
        cost: ActionCost::default(),
        acquisition_success_probability: 0.5,
        outcomes: vec![
            ObservationOutcome {
                id: "measurement_zero".to_owned(),
                likelihood_state_zero: 0.5,
                likelihood_state_one: 0.5,
            },
            ObservationOutcome {
                id: "measurement_one".to_owned(),
                likelihood_state_zero: 0.5,
                likelihood_state_one: 0.5,
            },
        ],
    };
    let missingness_belief = BinaryBelief::new(0.6)?;
    let missingness = evaluate_action_with_acquisition_model(
        &missingness_action,
        missingness_belief,
        ResourceBudget::default(),
        AcquisitionSuccessModel::new(0.2, 0.8)?,
    )?;
    println!(
        "model=state_dependent_missingness q_state_zero=0.2 q_state_one=0.8 success_probability={:.3} information_on_success={:.6} total_information_bits={:.6} decision_value={:.6}",
        missingness.acquisition_success_probability_under_prior,
        missingness.mutual_information_bits_on_success,
        missingness.expected_mutual_information_bits,
        missingness.expected_decision_value
    );
    println!("scope=finite_one_step_synthetic_oracle_not_navigation_qualification");
    Ok(())
}
