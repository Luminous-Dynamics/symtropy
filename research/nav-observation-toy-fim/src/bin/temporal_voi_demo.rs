use nav_observation_toy_fim::dependency_graph::{
    exact_domain_horizon_two, DomainAction, DomainOutcome, FailureDomainModel,
};
use nav_observation_toy_fim::decision_value::{ActionCost, DecisionLoss, ResourceBudget};
use nav_observation_toy_fim::temporal_voi::{
    exact_temporal_horizon_two, TemporalTransitionModel,
};

fn action(id: &str, accuracy: f64, latency: u32) -> DomainAction {
    DomainAction {
        id: id.to_owned(),
        cost: ActionCost { energy_units: 1, latency_ticks: latency, bandwidth_units: 1 },
        failure_domains: vec!["clock".to_owned()],
        acquisition_success: vec![1.0; 4],
        outcomes: vec![
            DomainOutcome { id: "safe-reading".to_owned(), likelihood: vec![1.0-accuracy,1.0-accuracy,accuracy,accuracy] },
            DomainOutcome { id: "unsafe-reading".to_owned(), likelihood: vec![accuracy,accuracy,1.0-accuracy,1.0-accuracy] },
        ],
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = FailureDomainModel::new(vec!["clock".to_owned()], vec![0.25; 4])?;
    let dynamics = TemporalTransitionModel {
        probability_zero_to_one_per_tick: vec![0.02; 2],
        probability_one_to_zero_per_tick: vec![0.15; 2],
    };
    let actions = vec![
        action("fast-lower-accuracy",0.8,1),
        action("slow-higher-accuracy",0.95,4),
    ];
    let budget = ResourceBudget { energy_units:1, latency_ticks:4, bandwidth_units:1 };
    let static_plan = exact_domain_horizon_two(&actions,&model,&model.prior,budget,DecisionLoss::zero_one())?.ok_or("no static plan")?;
    let temporal_plan = exact_temporal_horizon_two(&actions,&model,&dynamics,&model.prior,budget,DecisionLoss::zero_one())?.ok_or("no temporal plan")?;
    println!("static_first_action={}",static_plan.first_action);
    println!("static_expected_terminal_risk={:.6}",static_plan.expected_terminal_risk);
    println!("temporal_first_action={}",temporal_plan.first_action);
    println!("temporal_expected_terminal_risk={:.6}",temporal_plan.expected_terminal_bayes_risk);
    for b in temporal_plan.branches {
        println!("branch={} elapsed_ticks={} posterior_service_one_at_decision={:.6}",b.branch_id,b.elapsed_ticks,b.posterior_service_one_at_decision);
    }
    println!("scope=synthetic_markov_state_drift_not_physical_hazard_calibration");
    Ok(())
}
