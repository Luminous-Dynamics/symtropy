use nav_observation_toy_fim::decision_value::{
    AcquisitionSuccessModel, ActionCost, BinaryBelief, DecisionLoss, DiagnosticAction,
    ObservationOutcome, ResourceBudget,
};
use nav_observation_toy_fim::robust_decision_value::{
    assess_action_under_model_uncertainty, ActionModelScenario,
};

fn model(
    scenario_id: &str,
    p_positive_state_zero: f64,
    p_positive_state_one: f64,
) -> ActionModelScenario {
    ActionModelScenario {
        scenario_id: scenario_id.to_owned(),
        action: DiagnosticAction {
            id: "camera_reobserve".to_owned(),
            cost: ActionCost {
                energy_units: 2,
                latency_ticks: 3,
                bandwidth_units: 1,
            },
            acquisition_success_probability: 1.0,
            outcomes: vec![
                ObservationOutcome {
                    id: "target_visible".to_owned(),
                    likelihood_state_zero: p_positive_state_zero,
                    likelihood_state_one: p_positive_state_one,
                },
                ObservationOutcome {
                    id: "target_not_visible".to_owned(),
                    likelihood_state_zero: 1.0 - p_positive_state_zero,
                    likelihood_state_one: 1.0 - p_positive_state_one,
                },
            ],
        },
        acquisition: AcquisitionSuccessModel::state_independent(0.9),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let belief = BinaryBelief::new(0.7)?;
    let scenarios = vec![
        model("nominal-clear-atmosphere", 0.85, 0.15),
        model("degraded-contrast", 0.65, 0.35),
        model("adverse-occlusion", 0.52, 0.48),
    ];
    let assessment = assess_action_under_model_uncertainty(
        &scenarios,
        belief,
        ResourceBudget {
            energy_units: 3,
            latency_ticks: 5,
            bandwidth_units: 2,
        },
        DecisionLoss::new(8.0, 1.0)?,
        1.5,
        0.6,
        1.0,
    )?;

    println!("action={}", assessment.action_id);
    println!("scenario_mean_value={:.6}", assessment.mean_decision_value);
    println!("minimum_scenario_value={:.6}", assessment.minimum_model_value);
    println!("downside_deviation={:.6}", assessment.downside_deviation);
    println!("risk_premium={:.6}", assessment.risk_premium);
    println!("ignorance_penalty={:.6}", assessment.ignorance_penalty);
    println!("risk_adjusted_value={:.6}", assessment.risk_adjusted_decision_value);
    println!("model_confidence={:.3}", assessment.model_confidence);
    println!("scope=finite_frozen_scenario_set_not_continuous_uncertainty_certificate");
    Ok(())
}
